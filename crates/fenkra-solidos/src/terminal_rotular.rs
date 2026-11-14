// SPDX-License-Identifier: LGPL-2.1-or-later
//! Terminal rotular macho (B397): a ponta articulada dos braços da direção do kart.
//!
//! **Dados** (família `terminal_rotular_ilustrativo`): **medidas ilustrativas**, escolhidas pelo
//! Fenkra só para a peça ter forma coerente; nenhuma vem de norma ou catálogo. O diâmetro nominal
//! `d` é ao mesmo tempo o furo da esfera e a rosca da haste. Campos: diâmetro externo da carcaça
//! `dk`, espessura da carcaça `C1` (fictícia), diâmetro da esfera `dE`, largura da esfera `B`, do
//! centro da esfera ao fim da rosca `h`, diâmetro da haste `dh` (fictício) e comprimento da rosca `l1`.
//!
//! **Modelo**: centro da esfera na origem, furo da esfera no eixo **Y** e haste no eixo **Z**, para
//! baixo (como os fixadores: o corpo vai para −Z). Esfera `Ø dE` aparada em `|y| ≤ B/2`, carcaça
//! (disco `Ø dk` de espessura `C1`, `|y| ≤ C1/2`), haste `Ø dh` de `z = −h + l1` até o centro, rosca
//! `Ø d` de `z = −h` a `z = −h + l1` (sem filete desenhado) e o furo `Ø d` atravessando a esfera.
//!
//! **Coerência** (recusada fora dela): o furo sai inteiro dentro da esfera aparada
//! (`(d/2)² + (B/2)² ≤ (dE/2)²`), a esfera cabe na carcaça (`dE < dk`) e sobra dela dos dois lados
//! (`C1 < B`), a haste é mais grossa que a rosca e mais fina que a carcaça (`d < dh < C1`) e a rosca
//! começa abaixo da carcaça (`h − l1 > dk/2`).

use crate::biblioteca::{Familia, Tamanho};
use crate::fixadores::Montador;
use crate::pneumatica::plano_em;
use fenkra_core::*;

/// Plano paralelo ao XZ em `y` (expressão). A normal do XZ é −Y: a primitiva cresce de `y` para −Y.
fn plano_xz(m: &mut Montador, nome: &str, y: &str) -> Result<PlaneRef, String> {
    let p = m.doc.add_geometry(RefKind::PointCoords { x: "0".into(), y: y.into(), z: "0".into(), frame: None });
    let id = m.doc.add_geometry(RefKind::PlaneParallelPoint { base: PlaneRef::Origin(OriginPlane::XZ), point: PointRef::Ref(p) });
    for r in m.doc.references.iter_mut().filter(|r| r.id == p || r.id == id) {
        r.visible = false;
        r.name = if r.id == id { nome.into() } else { format!("{nome}_ponto") };
    }
    Ok(PlaneRef::Datum(id))
}

/// Confere as relações que fazem a peça fechar; `d` é o diâmetro nominal.
pub fn coerente(d: f64, v: &dyn Fn(&str) -> f64) -> Result<(), String> {
    let (dk, c1, de, b, h, dh, l1) = (v("dk"), v("C1"), v("dE"), v("B"), v("h"), v("dh"), v("l1"));
    let regras = [
        ((d / 2.).powi(2) + (b / 2.).powi(2) <= (de / 2.).powi(2), "o furo precisa sair dentro da esfera: (d/2)² + (B/2)² ≤ (dE/2)²"),
        (de < dk, "a esfera precisa caber na carcaça: dE < dk"),
        (c1 < b, "a esfera precisa sobrar da carcaça: C1 < B"),
        (d < dh && dh < c1, "a haste fica entre a rosca e a carcaça: d < dh < C1"),
        (h - l1 > dk / 2., "a rosca começa abaixo da carcaça: h − l1 > dk/2"),
    ];
    match regras.iter().find(|(ok, _)| !ok) {
        Some((_, msg)) => Err((*msg).to_string()),
        None => Ok(()),
    }
}

/// Monta o terminal; o parâmetro `d` (furo e rosca) já existe.
pub(crate) fn corpo(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    let d = t.diametro_nominal().ok_or_else(|| format!("{}: designação inválida {}", fam.id, t.designacao))?;
    let campos = ["dk", "C1", "dE", "B", "h", "dh", "l1"];
    if let Some(c) = campos.iter().find(|c| t.valor(c).is_none()) {
        return Err(format!("{} {}: falta '{c}'", fam.id, t.designacao));
    }
    coerente(d, &|c| t.valor(c).unwrap_or(0.)).map_err(|e| format!("{} {}: {e}", fam.id, t.designacao))?;
    for c in campos {
        m.param(c, &t.valor(c).unwrap_or(0.).to_string())?;
    }
    // esfera aparada: a interseção pega só o que já existe (a esfera, primeiro comando)
    m.prim("Esfera", PrimKind::Esfera, PlaneRef::Origin(OriginPlane::XY), ["dE/2", "0", "0", "0"], BoolOp::Add)?;
    let faces = plano_xz(m, "Plano_faces_da_esfera", "B/2")?;
    m.prim("Faces da esfera", PrimKind::Caixa, faces, ["2*dk", "2*dk", "B", "0"], BoolOp::Intersect)?;
    let carcaca = plano_xz(m, "Plano_carcaca", "C1/2")?;
    m.prim("Carcaça", PrimKind::Cilindro, carcaca, ["dk/2", "C1", "0", "0"], BoolOp::Add)?;
    let haste = plano_em(m, "Plano_haste", "0", "0", "-h+l1")?;
    m.prim("Haste", PrimKind::Cilindro, haste, ["dh/2", "h-l1", "0", "0"], BoolOp::Add)?;
    let rosca = m.plano("Plano_rosca", "-h")?;
    m.prim("Rosca", PrimKind::Cilindro, rosca, ["d/2", "l1", "0", "0"], BoolOp::Add)?;
    let furo = plano_xz(m, "Plano_furo", "B/2+1")?;
    m.prim("Furo da esfera", PrimKind::Cilindro, furo, ["d/2", "B+2", "0", "0"], BoolOp::Cut)?;
    // o pino da junta esférica passa pelo furo: a montagem se prende a este eixo
    m.publicar("Eixo_esfera", RefKind::LineDir { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Y), start: "-B/2".into(), end: "B/2".into() });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::coerente;
    use crate::biblioteca::{familia, TipoFixador};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    fn caixa(doc: &fenkra_core::Document) -> (f64, [f64; 3], [f64; 3]) {
        let rc = recompute(doc);
        assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
        for f in &doc.features {
            assert_eq!(rc.status[&f.id], Status::Ok, "{}", f.name);
        }
        assert_eq!(rc.bodies.len(), 1);
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (rc.bodies[0].mass.volume, lo, hi)
    }

    /// Volume da esfera de raio `r` entre os planos `|y| ≤ a`.
    fn faixa_de_esfera(r: f64, a: f64) -> f64 {
        PI * (2. * a * r * r - 2. * a.powi(3) / 3.)
    }

    /// Volume da haste (raio `rh`, eixo Z) dentro do disco da carcaça (raio `rk`, eixo Y): para cada
    /// (x, y) da seção da haste, a altura dentro do disco é √(rk² − x²). Regra do ponto médio.
    fn haste_na_carcaca(rh: f64, rk: f64) -> f64 {
        let n = 20000;
        let dx = 2. * rh / n as f64;
        (0..n).map(|i| -rh + (i as f64 + 0.5) * dx).map(|x| 2. * (rh * rh - x * x).sqrt() * (rk * rk - x * x).sqrt() * dx).sum()
    }

    #[test]
    fn familia_ilustrativa_marcada_e_coerente() {
        let f = familia("terminal_rotular_ilustrativo").unwrap();
        assert_eq!((f.tipo, f.gerador.as_str(), f.nivel.as_str()), (TipoFixador::TerminalRotular, "terminal_rotular", "A"));
        // B397: medidas fictícias, nunca passam por conferidas
        assert!(f.ilustrativo && f.nome.contains("ilustrativas"));
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por == 0 && t.nota.as_deref().unwrap().starts_with("ILUSTRATIVO")));
        assert_eq!(f.tamanhos.iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>(), ["M6", "M8", "M10"]);
        let m8 = f.tamanho("M8").unwrap();
        assert_eq!((m8.valor("C1"), m8.valor("dh")), (Some(10.), Some(9.5)));
        for t in &f.tamanhos {
            coerente(t.diametro_nominal().unwrap(), &|c| t.valor(c).unwrap()).unwrap();
        }
        // a haste mais fina que a rosca não fecha
        let v = |c: &str| if c == "dh" { 7. } else { m8.valor(c).unwrap() };
        assert!(coerente(8., &v).unwrap_err().contains("d < dh"));
    }

    #[test]
    fn terminal_gera_esfera_carcaca_e_haste_com_volume_e_caixa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("terminal_rotular_ilustrativo").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let d = t.diametro_nominal().unwrap();
            let doc = fixador_documento("terminal_rotular_ilustrativo", &t.designacao, None).unwrap();
            assert!(doc.params.iter().any(|p| p.name == "C1") && doc.params.iter().any(|p| p.name == "dh"));
            assert!(doc.references.iter().any(|r| r.published && r.name == "Eixo_esfera"));
            let (vol, lo, hi) = caixa(&doc);
            // caixa: carcaça em X, esfera sobrando em Y, da ponta da rosca ao topo da carcaça em Z
            let (rk, a) = (v("dk") / 2., v("B") / 2.);
            assert!((lo[0] + rk).abs() < 0.05 && (hi[0] - rk).abs() < 0.05, "{}: x {lo:?} {hi:?}", t.designacao);
            assert!((lo[1] + a).abs() < 1e-3 && (hi[1] - a).abs() < 1e-3, "{}: y {lo:?} {hi:?}", t.designacao);
            assert!((lo[2] + v("h")).abs() < 1e-3 && (hi[2] - rk).abs() < 0.05, "{}: z {lo:?} {hi:?}", t.designacao);
            // volume: disco + esfera fora do disco + haste fora do disco + rosca − furo
            let (r, c) = (v("dE") / 2., v("C1") / 2.);
            let disco = PI * rk * rk * v("C1");
            let esfera = faixa_de_esfera(r, a) - faixa_de_esfera(r, c);
            let rh = v("dh") / 2.;
            let haste = PI * rh * rh * (v("h") - v("l1")) - haste_na_carcaca(rh, rk);
            let rosca = PI * d * d / 4. * v("l1");
            let furo = PI * d * d / 4. * v("B");
            let esperado = disco + esfera + haste + rosca - furo;
            assert!((vol - esperado).abs() / esperado < 2e-3, "{}: volume {vol} contra {esperado}", t.designacao);
        }
    }
}
