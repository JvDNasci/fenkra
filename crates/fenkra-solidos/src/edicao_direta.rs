// SPDX-License-Identifier: LGPL-2.1-or-later
//! B33: edição direta de faces. O corpo é alterado pelas faces, sem histórico de sketch: é o que
//! torna útil um corpo vindo de STEP.
//!
//! Como funciona no kernel (sempre sobre o corpo atual do coletor):
//! - Apagar: o kernel remove as faces e estende as vizinhas até se encontrarem (cura do vizinho).
//! - Deslocar: o algoritmo de offset do OpenCASCADE recebe uma distância só para as faces escolhidas
//!   e zero para o resto; as vizinhas acompanham. Em cilindro ou esfera a distância muda o raio.
//! - Mover: apaga as faces com cura; a diferença de volume entre o corpo curado e o original é o
//!   relevo (saliência ou reentrância). O relevo é transladado e devolvido por união ou corte.
//! - Raio do filete: apaga a face do filete (a aresta viva reaparece), acha a aresta e arredonda de
//!   novo com o raio novo.
//!
//! Toda operação termina na verificação de sólido válido: volume positivo, sólido e topologia
//! aprovada pelo kernel; senão o comando falha com mensagem e o corpo anterior fica intacto.

use super::*;
use fenkra_core::{DirectEdit, OpDireta};

/// Diferença de volume (fração do corpo) abaixo da qual duas formas são tratadas como iguais.
const EPS_REL: f64 = 1e-7;

impl Ctx<'_> {
    pub(crate) fn edicao_direta(&mut self, f: &Feature, d: &DirectEdit) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let (b, owners, roles) = self.body_owned(f.target).ok_or("não há corpo para editar")?;
        let tags = tags_for(owners);
        let pts = self.resolve_faces("edição direta", f, b, owners, roles, &d.faces, &d.face_keys)?;
        let vol0 = b.mass().volume;
        let (s, out) = match d.op {
            OpDireta::Apagar => b.defeature_tagged(&pts, &tags).map_err(k)?,
            OpDireta::Deslocar => {
                let v = ev(&d.value, &self.vars, "distância")?;
                if v.abs() < 1e-9 {
                    return Ok(Some("distância zero: nada a fazer".into()));
                }
                b.face_offset_tagged(&pts, v, &tags).map_err(k)?
            }
            OpDireta::Mover => {
                let v = [ev(&d.delta[0], &self.vars, "deslocamento X")?, ev(&d.delta[1], &self.vars, "deslocamento Y")?, ev(&d.delta[2], &self.vars, "deslocamento Z")?];
                if v.iter().all(|x| x.abs() < 1e-9) {
                    return Ok(Some("deslocamento zero: nada a fazer".into()));
                }
                mover_relevo(b, &pts, v, &tags)?
            }
            OpDireta::RaioFilete => {
                let r = ev(&d.value, &self.vars, "raio")?;
                if r < 0. {
                    return Err("o raio do filete não pode ser negativo".into());
                }
                trocar_raio(b, &pts, r, &tags)?
            }
        };
        valida(&s, vol0)?;
        let body = remap_body(s, owners, roles, &out, f.id);
        self.bodies.insert(f.target, body);
        Ok(None)
    }
}

/// Verificação de sólido válido do resultado.
fn valida(s: &Shape, vol0: f64) -> Result<(), String> {
    if s.is_null() {
        return Err("a edição não produziu corpo".into());
    }
    let m = s.mass().volume;
    if !(m > EPS_REL * vol0.abs().max(1.)) {
        return Err("a edição esvaziou o corpo: volume nulo".into());
    }
    if !s.is_solid() {
        return Err("a edição não deixou um sólido fechado".into());
    }
    if !s.is_valid() {
        return Err("a edição deixou o corpo com topologia inválida: use uma distância menor".into());
    }
    Ok(())
}

/// Leva o relevo formado pelas faces `pts` por `v`.
fn mover_relevo(b: &Shape, pts: &[[f64; 3]], v: [f64; 3], tags: &[i32]) -> Result<(Shape, Vec<i32>), String> {
    let k = |e: KernelError| e.0;
    let (curado, ctags) = b.defeature_tagged(pts, tags).map_err(k)?;
    let (v0, v1) = (b.mass().volume, curado.mass().volume);
    let tol = EPS_REL * v0.abs().max(1.);
    if (v1 - v0).abs() <= tol {
        return Err("estas faces não formam um relevo que possa ser movido (apagá-las não muda o volume)".into());
    }
    if v1 > v0 {
        // reentrância (furo, rasgo): o material tirado volta para o lugar e é tirado de novo mais adiante
        let vazio = curado.boolean(b, Bool::Cut).map_err(k)?;
        let novo = vazio.transformed(Xform::Translate(v)).map_err(k)?;
        curado.boolean_tagged(&novo, Bool::Cut, &ctags).map_err(k)
    } else {
        // saliência (boss, nervura): o material a mais é levado junto
        let sobra = b.boolean(&curado, Bool::Cut).map_err(k)?;
        let novo = sobra.transformed(Xform::Translate(v)).map_err(k)?;
        curado.boolean_tagged(&novo, Bool::Fuse, &ctags).map_err(k)
    }
}

/// Troca o raio de um filete cilíndrico: cura a face e arredonda a aresta viva que reaparece.
fn trocar_raio(b: &Shape, pts: &[[f64; 3]], r: f64, tags: &[i32]) -> Result<(Shape, Vec<i32>), String> {
    let k = |e: KernelError| e.0;
    if pts.len() != 1 {
        return Err("escolha uma única face de filete".into());
    }
    let p = pts[0];
    let topo = b.topology().map_err(k)?;
    let i = b.face_index(p, 1.0).map_err(k)?.ok_or("a face escolhida não existe mais")?;
    if topo.faces[i].kind != 1 {
        return Err("esta face não é um filete de raio constante (cilíndrico): escolha a face curva do filete".into());
    }
    let cil = b.cylinder_at(p).map_err(k)?;
    let (curado, ctags) = b.defeature_tagged(pts, tags).map_err(k)?;
    if r < 1e-9 {
        return Ok((curado, ctags));
    }
    // a aresta viva é reta, paralela ao eixo e fica perto dele, no meio do trecho do filete
    let comprimento = (cil.h1 - cil.h0).abs();
    let meio = (cil.h0 + cil.h1) / 2.;
    let topo2 = curado.topology().map_err(k)?;
    let mut melhor: Option<([f64; 3], f64)> = None;
    for m in &topo2.edge_mids {
        if m.iter().all(|x| x.abs() < 1e-12) {
            continue;
        }
        let rel = sub(*m, cil.origin);
        let ax = dot(rel, cil.dir);
        let perp = [rel[0] - cil.dir[0] * ax, rel[1] - cil.dir[1] * ax, rel[2] - cil.dir[2] * ax];
        let dist = dot(perp, perp).sqrt();
        if (ax - meio).abs() > 0.25 * comprimento + 0.5 || dist > 4. * cil.radius + 1. {
            continue;
        }
        if melhor.map_or(true, |(_, d)| dist < d) {
            melhor = Some((*m, dist));
        }
    }
    let (mid, _) = melhor.ok_or("não achei a aresta que o filete arredondava: o corpo curado não tem canto ali")?;
    let tol = cil.radius.max(r).max(0.5) * 0.5;
    curado.edges_tagged(r, &[mid], tol, false, &ctags).map_err(k)
}
