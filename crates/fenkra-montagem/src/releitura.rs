// SPDX-License-Identifier: LGPL-2.1-or-later
//! Referências que se reencontram quando a peça muda (B99, B263, B55).
//!
//! Uma restrição guarda a geometria já lida no sistema da peça. Isso é rápido para resolver, mas fica
//! velho quando a peça muda. Ler de novo depende do que foi clicado, e é isso que [`Viva`] guarda:
//!
//! - **face**: a nomeação topológica da peça (ADR 0010): o comando que criou a face e o papel dela
//!   para ele. Um filete ou uma face que andou não trocam a face; o ponto clicado (`MateRef::pick`)
//!   só desempata. Face sem nome usa o ponto, mas só enquanto ele continuar sobre a peça;
//! - **aresta nomeada**: a aresta entre duas faces nomeadas;
//! - **aresta**: procura na malha a aresta mais perto do ponto guardado. Perguntar pela face ali
//!   devolveria o eixo do cilindro vizinho, e não a aresta;
//! - **vértice**: a ponta de aresta mais perto;
//! - **ponto de encaixe**: o candidato do mesmo tipo mais perto, de preferência com a mesma normal,
//!   porque um canto pertence a três faces;
//! - **geometria da montagem**: a identidade do plano, ponto, linha ou sketch no documento embutido.
//!
//! Nada aqui adivinha: quando o que foi clicado sumiu, a referência fica como estava e a restrição
//! continua valendo com a última geometria boa. A face e a aresta ainda ficam marcadas como
//! perdidas, para a interface avisar e oferecer reancorar ([`reancorar`]).

use crate::encaixe::{self, TipoEncaixe};
use crate::kin::JFrame;
use crate::load::{full_path, geom_at, LoadedPart, PartCache};
use crate::*;
use fenkra_core::{EdgeKey, FaceKey};
use fenkra_kernel::{Mesh, Topology};
use fenkra_solidos::geo3d::{fit_circle, Curve3};
use fenkra_solidos::refs::{self, View};
use fenkra_solidos::{BodyResult, RecomputeResult};
use std::path::{Path, PathBuf};

/// Como a referência se reencontra. Sem isto, vale o ponto clicado na face.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Viva {
    /// aresta que passa perto de `ponto`, no sistema da peça
    Aresta { ponto: V3 },
    /// vértice mais perto da posição guardada na própria geometria
    Vertice,
    /// ponto de encaixe do tipo escolhido
    Encaixe { tipo: TipoEncaixe },
    /// plano, ponto, linha ou sketch da geometria da montagem
    Geometria { id: Id },
    /// face pelo nome que ela tem na peça (ADR 0010). `chave` vazia: a face não tem papel conhecido,
    /// e vale o ponto clicado. `perdida`: a peça mudou e não há como saber qual é a mesma face
    Face {
        chave: Option<FaceKey>,
        #[serde(default)]
        perdida: bool,
    },
    /// aresta entre duas faces nomeadas, com o ponto do meio para desempatar
    ArestaNomeada {
        chave: EdgeKey,
        ponto: V3,
        #[serde(default)]
        perdida: bool,
    },
}

/// Distância do ponto clicado até a face para ele ainda contar como "sobre a face" (mm). Cobre o
/// erro da malha em faces curvas; uma face que andou mais que isso deixa o ponto no ar.
const NA_FACE: f64 = 0.5;

/// A referência perdeu a face ou a aresta em que nasceu?
pub fn perdida(r: &MateRef) -> bool {
    matches!(r.viva, Some(Viva::Face { perdida: true, .. } | Viva::ArestaNomeada { perdida: true, .. }))
}

/// Troca o elemento A (`lado_a`) ou B da restrição por `nova`, escolhida de novo pelo usuário na
/// mesma peça. É a saída quando a referência se perdeu: nada é adivinhado, quem decide é o usuário.
pub fn reancorar(a: &mut Assembly, mate: Id, lado_a: bool, nova: MateRef) -> Result<(), String> {
    let m = a.mates.iter_mut().find(|m| m.id == mate).ok_or("restrição não encontrada")?;
    let (velha, outra) = if lado_a { (&m.a, &m.b) } else { (&m.b, &m.a) };
    if nova.comp != velha.comp {
        return Err("escolha o elemento novo na mesma peça".into());
    }
    let (ga, gb) = if lado_a { (nova.geom, outra.geom) } else { (outra.geom, nova.geom) };
    if !par_valido(&m.kind, &ga, &gb) {
        return Err("este elemento não serve para esta restrição".into());
    }
    if lado_a {
        m.a = nova;
    } else {
        m.b = nova;
    }
    Ok(())
}

/// Topologia de um corpo, lida da malha (mesma ordem de faces e arestas).
fn topologia(m: &Mesh) -> Topology {
    Topology { faces: m.face_info.clone(), edge_mids: m.edge_mids.chunks_exact(3).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]).collect(), edge_faces: m.edge_faces.clone() }
}

fn vista<'a>(t: &'a Topology, b: &'a BodyResult) -> View<'a> {
    View { topo: t, owners: &b.face_owner, roles: &b.face_role }
}

fn distancia_ao_triangulo(p: V3, a: V3, b: V3, c: V3) -> f64 {
    let (ab, ac) = (sub(b, a), sub(c, a));
    let n = cross(ab, ac);
    let nn = dot(n, n);
    if nn > 1e-24 {
        // projeção no plano do triângulo; se cair dentro, a distância é a altura
        let t = dot(sub(p, a), n) / nn;
        let q = [0, 1, 2].map(|i| p[i] - n[i] * t);
        let qa = sub(q, a);
        let (d00, d01, d11, d20, d21) = (dot(ab, ab), dot(ab, ac), dot(ac, ac), dot(qa, ab), dot(qa, ac));
        let den = d00 * d11 - d01 * d01;
        let v = (d11 * d20 - d01 * d21) / den;
        let u = (d00 * d21 - d01 * d20) / den;
        if v >= 0. && u >= 0. && v + u <= 1. {
            return norm(sub(p, q));
        }
    }
    distancia_ao_segmento(p, a, b).min(distancia_ao_segmento(p, b, c)).min(distancia_ao_segmento(p, c, a))
}

fn distancia_a_face(m: &Mesh, f: usize, q: V3) -> f64 {
    let (i0, i1) = (m.face_ranges[f] as usize, m.face_ranges.get(f + 1).map_or(m.indices.len(), |x| *x as usize));
    let v = |k: u32| {
        let k = k as usize * 3;
        [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
    };
    m.indices[i0..i1].chunks_exact(3).map(|t| distancia_ao_triangulo(q, v(t[0]), v(t[1]), v(t[2]))).fold(f64::MAX, f64::min)
}

/// Face da peça mais perto de `q`: (corpo, face, distância).
fn face_perto(lp: &LoadedPart, q: V3) -> Option<(usize, usize, f64)> {
    lp.result.bodies.iter().enumerate().flat_map(|(bi, b)| (0..b.mesh.face_ranges.len()).map(move |f| (bi, f, distancia_a_face(&b.mesh, f, q)))).min_by(|a, b| a.2.total_cmp(&b.2))
}

/// Aresta da peça mais perto de `q`: (corpo, aresta).
fn aresta_indice_perto(lp: &LoadedPart, q: V3) -> Option<(usize, usize)> {
    let medida = |p: &[V3]| p.windows(2).map(|w| distancia_ao_segmento(q, w[0], w[1])).fold(f64::MAX, f64::min);
    lp.result
        .bodies
        .iter()
        .enumerate()
        .flat_map(|(bi, b)| (0..b.mesh.edge_ranges.len()).map(move |e| (bi, e, medida(&encaixe::polilinha(&b.mesh, e)))))
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(bi, e, _)| (bi, e))
}

/// Como reencontrar a face clicada em `q` (sistema da peça): pelo nome dela, quando a face tem
/// papel conhecido e o nome sozinho já a separa das outras; senão pelo ponto.
pub fn viva_da_face(lp: &LoadedPart, q: V3) -> Option<Viva> {
    let (bi, j, d) = face_perto(lp, q)?;
    if d > NA_FACE {
        return None;
    }
    let b = &lp.result.bodies[bi];
    let t = topologia(&b.mesh);
    Some(Viva::Face { chave: refs::promote_face(&vista(&t, b), j), perdida: false })
}

/// Como reencontrar a aresta clicada perto de `ponto`: entre as duas faces nomeadas, quando dá; senão
/// pela aresta mais perto do ponto, como antes.
pub fn viva_da_aresta(lp: &LoadedPart, ponto: V3) -> Viva {
    let chave = aresta_indice_perto(lp, ponto).and_then(|(bi, e)| {
        let b = &lp.result.bodies[bi];
        let t = topologia(&b.mesh);
        refs::promote_edge(&vista(&t, b), e)
    });
    match chave {
        Some(chave) => Viva::ArestaNomeada { chave, ponto, perdida: false },
        None => Viva::Aresta { ponto },
    }
}

/// Elemento da face `j` do corpo `bi`. No plano, o ponto é o anterior levado até o plano novo: o
/// solucionador só olha o plano, e assim a restrição não muda à toa.
fn geom_da_face(lp: &LoadedPart, bi: usize, j: usize, antes: &Geom) -> Option<Geom> {
    let info = lp.result.bodies.get(bi)?.mesh.face_info.get(j)?;
    let g = geom_at(lp.shape.as_ref()?, info.point).ok()?;
    Some(match g {
        Geom::Plane { n, p } => {
            let p0 = base(antes).0;
            let s = dot(sub(p0, p), n);
            Geom::Plane { p: if s.abs() < 1e-9 { p0 } else { [0, 1, 2].map(|i| p0[i] - n[i] * s) }, n }
        }
        g => g,
    })
}

/// Mesmo tipo e mesma direção: plano com a normal para o mesmo lado, eixo paralelo.
fn mesma_direcao(a: &Geom, b: &Geom) -> bool {
    match (a, b) {
        (Geom::Plane { n: u, .. }, Geom::Plane { n: v, .. }) => dot(unit(*u), unit(*v)) > 0.999,
        (Geom::Axis { d: u, .. }, Geom::Axis { d: v, .. }) => dot(unit(*u), unit(*v)).abs() > 0.999,
        _ => false,
    }
}

/// O que uma leitura devolve: geometria, o que lembrar e o ponto clicado novo (quando o antigo ficou
/// fora da face).
type Lida = (Geom, Option<Viva>, Option<V3>);

/// Leitura da face, pelo nome ou, sem nome, pelo ponto clicado.
fn ler_face(r: &MateRef, lp: &LoadedPart, chave: Option<FaceKey>) -> Option<Lida> {
    let perdeu = |chave| Some((r.geom, Some(Viva::Face { chave, perdida: true }), None));
    let (bi, j, chave) = match chave {
        Some(k) => {
            let achadas: Vec<(usize, usize)> = lp
                .result
                .bodies
                .iter()
                .enumerate()
                .filter_map(|(bi, b)| {
                    let t = topologia(&b.mesh);
                    refs::find_face(&vista(&t, b), &k, r.pick).ok().map(|j| (bi, j))
                })
                .collect();
            let [(bi, j)] = achadas[..] else { return perdeu(Some(k)) };
            (bi, j, Some(k))
        }
        None => {
            // face sem nome: o ponto clicado. Fora da peça, a face mais perto só vale se estiver
            // virada para o mesmo lado; antes valia qualquer uma, e a restrição pulava de face
            let (bi, j, d) = face_perto(lp, r.pick?)?;
            if d > NA_FACE && !mesma_direcao(&geom_da_face(lp, bi, j, &r.geom)?, &r.geom) {
                return perdeu(None);
            }
            let b = &lp.result.bodies[bi];
            let t = topologia(&b.mesh);
            (bi, j, refs::promote_face(&vista(&t, b), j))
        }
    };
    let g = geom_da_face(lp, bi, j, &r.geom)?;
    // mantém o tipo escolhido: uma face plana que virou curva não é mais a mesma referência
    if std::mem::discriminant(&g) != std::mem::discriminant(&r.geom) {
        return perdeu(chave);
    }
    let m = &lp.result.bodies[bi].mesh;
    let pick = r.pick.filter(|q| distancia_a_face(m, j, *q) > NA_FACE).and_then(|_| m.face_info.get(j).map(|i| i.point));
    Some((g, Some(Viva::Face { chave, perdida: false }), pick))
}

/// Leitura da aresta entre duas faces nomeadas.
fn ler_aresta_nomeada(r: &MateRef, lp: &LoadedPart, chave: EdgeKey, ponto: V3) -> Option<Lida> {
    let achadas: Vec<Vec<V3>> = lp
        .result
        .bodies
        .iter()
        .filter_map(|b| {
            let t = topologia(&b.mesh);
            refs::find_edge(&vista(&t, b), &chave, Some(ponto)).ok().map(|e| encaixe::polilinha(&b.mesh, e))
        })
        .collect();
    let perdeu = Some((r.geom, Some(Viva::ArestaNomeada { chave, ponto, perdida: true }), None));
    let [pts] = &achadas[..] else { return perdeu };
    let Some(g) = eixo_da_aresta(pts, base(&r.geom).1) else { return perdeu };
    Some((g, Some(Viva::ArestaNomeada { chave, ponto: encaixe::meio(pts), perdida: false }), None))
}

fn malhas(lp: &LoadedPart) -> Vec<&Mesh> {
    lp.result.bodies.iter().map(|b| &b.mesh).collect()
}

fn distancia_ao_segmento(q: V3, a: V3, b: V3) -> f64 {
    let e = sub(b, a);
    let t = (dot(sub(q, a), e) / dot(e, e).max(1e-18)).clamp(0., 1.);
    norm(sub(q, [0, 1, 2].map(|i| a[i] + e[i] * t)))
}

/// Polilinha da aresta mais perto de `q`.
pub fn aresta_perto(ms: &[&Mesh], q: V3) -> Option<Vec<V3>> {
    let todas = ms.iter().flat_map(|m| (0..m.edge_ranges.len()).map(|e| encaixe::polilinha(m, e)));
    let medida = |p: &Vec<V3>| p.windows(2).map(|w| distancia_ao_segmento(q, w[0], w[1])).fold(f64::MAX, f64::min);
    todas.filter(|p| p.len() >= 2).min_by(|a, b| medida(a).total_cmp(&medida(b)))
}

/// Eixo da aresta: a reta dela, ou o eixo do círculo. Mantém o sentido de `antes`, para o que já foi
/// resolvido não virar de lado.
pub fn eixo_da_aresta(pts: &[V3], antes: Option<V3>) -> Option<Geom> {
    let c = Curve3::poly(pts.to_vec(), false).ok()?;
    let (p, d) = if c.is_straight() {
        (pts[0], unit(sub(pts[pts.len() - 1], pts[0])))
    } else {
        let (ctr, n, _) = fit_circle(&c).ok()?;
        (ctr, unit(n))
    };
    let vira = antes.is_some_and(|a| dot(a, d) < 0.);
    Some(Geom::Axis { p, d: if vira { d.map(|x| -x) } else { d } })
}

fn vertice_perto(ms: &[&Mesh], q: V3) -> Option<V3> {
    let pontas = ms.iter().flat_map(|m| (0..m.edge_ranges.len()).map(|e| encaixe::polilinha(m, e))).filter(|p| p.len() >= 2).flat_map(|p| [p[0], p[p.len() - 1]]);
    pontas.min_by(|a, b| norm(sub(*a, q)).total_cmp(&norm(sub(*b, q))))
}

/// Todos os candidatos de encaixe da peça.
fn encaixes_da_peca(lp: &LoadedPart) -> Vec<encaixe::Candidato> {
    malhas(lp).iter().flat_map(|m| (0..m.face_ranges.len()).flat_map(|f| encaixe::candidatos(m, f))).collect()
}

/// Candidato do `tipo` mais perto de `p`. A normal só desempata: o mesmo canto aparece em três faces.
fn encaixe_perto(todos: &[encaixe::Candidato], tipo: TipoEncaixe, p: V3, z: V3) -> Option<encaixe::Candidato> {
    let nota = |c: &encaixe::Candidato| norm(sub(c.p, p)) + (1. - dot(c.z, z));
    todos.iter().filter(|c| c.tipo == tipo).min_by(|a, b| nota(a).total_cmp(&nota(b))).copied()
}

/// Base e direção de um elemento.
fn base(g: &Geom) -> (V3, Option<V3>) {
    match *g {
        Geom::Plane { p, n } => (p, Some(n)),
        Geom::Axis { p, d } => (p, Some(d)),
        Geom::Point(p) => (p, None),
    }
}

/// Leitura nova da referência na peça dela. Devolve a geometria e o que precisa ser lembrado.
fn ler_na_peca(r: &MateRef, lp: &LoadedPart) -> Option<Lida> {
    let ms = malhas(lp);
    let (p, d) = base(&r.geom);
    match r.viva {
        Some(Viva::Aresta { ponto }) => {
            let pts = aresta_perto(&ms, ponto)?;
            let g = eixo_da_aresta(&pts, d)?;
            Some((g, Some(Viva::Aresta { ponto: encaixe::meio(&pts) }), None))
        }
        Some(Viva::ArestaNomeada { chave, ponto, .. }) => ler_aresta_nomeada(r, lp, chave, ponto),
        Some(Viva::Vertice) => Some((Geom::Point(vertice_perto(&ms, p)?), r.viva, None)),
        Some(Viva::Encaixe { .. } | Viva::Geometria { .. }) => None,
        Some(Viva::Face { chave, .. }) => ler_face(r, lp, chave),
        // arquivo antigo, só com o ponto clicado: a primeira leitura já guarda o nome da face
        None => ler_face(r, lp, None),
    }
}

/// Elemento da geometria da montagem pela identidade dele. No sketch, `antes` diz qual ponto ou linha
/// dentro dele: os elementos de um sketch não têm nome no resultado, então vale o mais perto.
pub fn ler_geometria(r: &RecomputeResult, id: Id, antes: Option<&Geom>) -> Option<Geom> {
    if let Some(f) = r.planes.iter().find(|x| x.id == Some(id)) {
        return Some(Geom::Plane { p: f.frame.origin, n: f.frame.normal });
    }
    if let Some(p) = r.points.iter().find(|x| x.id == id) {
        return Some(Geom::Point(p.p));
    }
    let d = antes.and_then(|g| base(g).1);
    if let Some(c) = r.curves.iter().find(|x| x.id == id) {
        return eixo_da_aresta(&c.pts, d);
    }
    let s = r.sketches.iter().find(|x| x.feature == id)?;
    let v = |p: &[f32; 3]| p.map(|x| x as f64);
    match antes? {
        // ponto solto ou ponta de linha: os dois se clicam do mesmo jeito
        Geom::Point(q) => s.points.iter().chain(&s.lines).map(v).min_by(|a, b| norm(sub(*a, *q)).total_cmp(&norm(sub(*b, *q)))).map(Geom::Point),
        Geom::Axis { p: q, .. } => {
            let w = s.lines.chunks_exact(2).map(|w| (v(&w[0]), v(&w[1])));
            let (a, b) = w.min_by(|x, y| distancia_ao_segmento(*q, x.0, x.1).total_cmp(&distancia_ao_segmento(*q, y.0, y.1)))?;
            eixo_da_aresta(&[a, b], d)
        }
        Geom::Plane { .. } => Some(Geom::Plane { p: s.frame.origin, n: s.frame.normal }),
    }
}

/// O que a releitura precisa, calculado uma vez por passada.
pub struct Leitor<'a> {
    pecas: HashMap<Id, PathBuf>,
    cache: &'a mut PartCache,
    geometria: Option<RecomputeResult>,
    /// candidatos de cada peça: a conta passa por todas as faces, e uma peça costuma ter várias referências
    encaixes: HashMap<PathBuf, Vec<encaixe::Candidato>>,
}

impl<'a> Leitor<'a> {
    pub fn novo(a: &Assembly, cache: &'a mut PartCache, dir: Option<&Path>) -> Self {
        let pecas = a
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::Part { path } => Some((n.id, full_path(dir, path))),
                _ => None,
            })
            .collect();
        let usa_geometria = a.mates.iter().flat_map(|m| [&m.a, &m.b]).chain(a.joints.iter().flat_map(|j| j.fontes.iter().flatten())).any(|r| matches!(r.viva, Some(Viva::Geometria { .. })));
        let geometria = a.geometria.as_ref().filter(|_| usa_geometria).map(fenkra_solidos::recompute);
        Leitor { pecas, cache, geometria, encaixes: HashMap::new() }
    }

    /// Atualiza a referência. Devolve se a geometria mudou.
    pub fn reler(&mut self, r: &mut MateRef) -> bool {
        let lida = if let Some(Viva::Geometria { id }) = r.viva {
            self.geometria.as_ref().and_then(|g| ler_geometria(g, id, Some(&r.geom))).map(|g| (g, r.viva, None))
        } else if r.viva.is_none() && r.pick.is_none() {
            None
        } else {
            let Some(arq) = self.pecas.get(&r.comp) else { return false };
            let Ok(lp) = self.cache.get(arq, true) else { return false };
            if let Some(Viva::Encaixe { tipo }) = r.viva {
                let todos = self.encaixes.entry(arq.clone()).or_insert_with(|| encaixes_da_peca(lp));
                let (p, d) = base(&r.geom);
                d.and_then(|z| encaixe_perto(todos, tipo, p, z)).map(|c| (encaixe::geom_local(&c, &Placement::default()), r.viva, None))
            } else {
                ler_na_peca(r, lp)
            }
        };
        let Some((g, viva, pick)) = lida else { return false };
        let mudou = g != r.geom;
        r.geom = g;
        if viva.is_some() {
            r.viva = viva;
        }
        if pick.is_some() {
            r.pick = pick;
        }
        mudou
    }
}

/// Leva o sistema da junta junto com o elemento em que ele nasceu: o que estava deslocado do elemento
/// continua deslocado igual, e o eixo gira o que o elemento girou.
pub fn transportar(f: &mut JFrame, antes: &Geom, depois: &Geom) {
    let ((p0, d0), (p1, d1)) = (base(antes), base(depois));
    let eixo = d0.zip(d1).map_or([0.; 3], |(u, v)| cross(unit(u), unit(v)));
    let s = norm(eixo);
    let angulo = d0.zip(d1).map_or(0., |(u, v)| s.atan2(dot(unit(u), unit(v))));
    let gira = |v: V3| if s < 1e-12 { v } else { rotvec(v, unit(eixo).map(|e| e * angulo)) };
    let o = gira(sub(f.o, p0));
    f.o = [0, 1, 2].map(|i| p1[i] + o[i]);
    f.z = unit(gira(f.z));
    f.x = unit(gira(f.x));
}

/// Relê tudo o que veio das peças: as duas pontas de cada restrição e de cada junta.
pub fn reler_montagem(a: &mut Assembly, cache: &mut PartCache, dir: Option<&Path>) {
    let mut l = Leitor::novo(a, cache, dir);
    for m in &mut a.mates {
        l.reler(&mut m.a);
        l.reler(&mut m.b);
    }
    for j in &mut a.joints {
        let [fa, fb] = &mut j.fontes;
        for (fonte, lado) in [(fa, &mut j.a), (fb, &mut j.b)] {
            let Some(r) = fonte else { continue };
            let antes = r.geom;
            if l.reler(r) {
                transportar(&mut lado.frame, &antes, &r.geom);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kin::{JRef, JointKind};
    use fenkra_core::{sketch::Curve, BoolOp, Document, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_releitura_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Bloco de `l` × 20 × `h` com um furo Ø6 em (`x`, 10).
    fn bloco(l: f64, h: f64, x: f64) -> Document {
        let mut d = Document::new("Bloco");
        let mut n = 7_700_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [l, 20.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(h.to_string()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), &h.to_string());
        let mut c = SketchData::default();
        let p = c.add_point(&mut al, x, 10.);
        c.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: "6".into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        d
    }

    fn montagem(dir: &Path, doc: &Document) -> (Assembly, Id, PartCache) {
        std::fs::write(dir.join("b.fkp"), doc.to_json()).unwrap();
        let mut a = Assembly::new("A");
        let c = a.add_component("Bloco", NodeKind::Part { path: "b.fkp".into() }, Placement::default(), None);
        (a, c, PartCache::default())
    }

    fn trocar(dir: &Path, cache: &mut PartCache, doc: &Document) {
        let f = dir.join("b.fkp");
        std::fs::write(&f, doc.to_json()).unwrap();
        cache.forget(&f);
    }

    /// O bloco com filete de raio `r` na aresta de cima em X = `l`.
    fn bloco_com_filete(l: f64, h: f64, x: f64, r: f64) -> Document {
        use fenkra_core::{EdgeOp, EdgeSel};
        let mut d = bloco(l, h, x);
        d.add_feature(FeatureKind::Fillet(EdgeOp { value: r.to_string(), edges: EdgeSel::Edges(vec![[l, 10., h]]), mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
        d
    }

    /// Referência criada como a interface cria: clique na face, guardando o nome dela.
    fn clique_na_face(dir: &Path, cache: &mut PartCache, c: Id, q: V3) -> MateRef {
        let lp = cache.get(&dir.join("b.fkp"), true).unwrap();
        let geom = geom_at(lp.shape.as_ref().unwrap(), q).unwrap();
        MateRef { comp: c, geom, pick: Some(q), viva: viva_da_face(lp, q) }
    }

    #[test]
    fn filete_na_peca_mantem_a_restricao_na_mesma_face() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("filete");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        // face de cima, clicada perto da aresta que vai ganhar o filete
        let mut r = clique_na_face(&dir, &mut cache, c, [37., 10., 10.]);
        assert!(matches!(r.viva, Some(Viva::Face { chave: Some(_), perdida: false })), "{:?}", r.viva);
        // filete de 4 na aresta e a peça 5 mais alta: o ponto clicado fica dentro da peça, mais perto
        // da face da ponta do que da de cima
        trocar(&dir, &mut cache, &bloco_com_filete(40., 15., 12., 4.));
        Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r);
        let Geom::Plane { p, n } = r.geom else { panic!("{:?}", r.geom) };
        assert!(n[2] > 0.999, "continua na face de cima: {n:?}");
        assert!((p[2] - 15.).abs() < 1e-6, "{p:?}");
        assert!(!perdida(&r));
    }

    #[test]
    fn face_que_andou_continua_a_mesma() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("face_andou");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        let mut r = clique_na_face(&dir, &mut cache, c, [40., 10., 5.]);
        trocar(&dir, &mut cache, &bloco(60., 10., 12.));
        assert!(Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r));
        let Geom::Plane { p, n } = r.geom else { panic!("{:?}", r.geom) };
        assert!(n[0] > 0.999, "a face da ponta, não a de cima: {n:?}");
        assert!((p[0] - 60.).abs() < 1e-6, "{p:?}");
        // o ponto clicado vai junto, para a reserva continuar valendo
        assert!((r.pick.unwrap()[0] - 60.).abs() < 1e-6, "{:?}", r.pick);
    }

    #[test]
    fn referencia_antiga_sem_nome_avisa_em_vez_de_pular_para_outra_face() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("antiga");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        let antes = Geom::Plane { p: [40., 10., 5.], n: [1., 0., 0.] };
        // como gravavam as versões anteriores: só o ponto clicado
        let mut r = MateRef { comp: c, geom: antes, pick: Some([40., 10., 5.]), viva: None };
        trocar(&dir, &mut cache, &bloco(60., 10., 12.));
        Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r);
        assert_eq!(r.geom, antes, "a face mais perto do ponto é a de cima: não pode virar ela");
        assert!(perdida(&r));
        // sem mudança, a referência antiga ganha o nome da face na primeira leitura
        let mut r2 = MateRef { comp: c, geom: Geom::Plane { p: [60., 10., 5.], n: [1., 0., 0.] }, pick: Some([60., 10., 5.]), viva: None };
        Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r2);
        assert!(matches!(r2.viva, Some(Viva::Face { chave: Some(_), perdida: false })), "{:?}", r2.viva);
    }

    #[test]
    fn aresta_que_virou_filete_avisa_e_aceita_reancorar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("aresta_filete");
        let (mut a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        let ponto = [40., 10., 10.];
        let viva = viva_da_aresta(cache.get(&dir.join("b.fkp"), true).unwrap(), ponto);
        assert!(matches!(viva, Viva::ArestaNomeada { perdida: false, .. }), "{viva:?}");
        let antes = Geom::Axis { p: [40., 0., 10.], d: [0., 1., 0.] };
        let ra = MateRef { comp: c, geom: antes, pick: None, viva: Some(viva) };
        let outra = a.add_component("Outra", NodeKind::Part { path: "o.fkp".into() }, Placement::default(), None);
        let rb = MateRef { comp: outra, geom: Geom::Axis { p: [0.; 3], d: [0., 1., 0.] }, pick: None, viva: None };
        let m = a.add_mate(MateKind::Coincident { flip: false }, ra, rb);
        // a aresta virou filete: não há mais aresta entre as duas faces
        trocar(&dir, &mut cache, &bloco_com_filete(40., 10., 12., 3.));
        reler_montagem(&mut a, &mut cache, Some(&dir));
        let mm = a.mates.iter().find(|x| x.id == m).unwrap();
        assert!(perdida(&mm.a));
        assert_eq!(mm.a.geom, antes, "segue com a última geometria boa");
        // reancorar: só na mesma peça
        let errada = MateRef { comp: outra, ..mm.a };
        assert!(reancorar(&mut a, m, true, errada).is_err());
        let viva = viva_da_aresta(cache.get(&dir.join("b.fkp"), true).unwrap(), [37., 10., 10.]);
        let nova = MateRef { comp: c, geom: Geom::Axis { p: [37., 0., 10.], d: [0., 1., 0.] }, pick: None, viva: Some(viva) };
        reancorar(&mut a, m, true, nova).unwrap();
        reler_montagem(&mut a, &mut cache, Some(&dir));
        assert!(!perdida(&a.mates.iter().find(|x| x.id == m).unwrap().a));
    }

    #[test]
    fn ponto_de_encaixe_acompanha_o_furo_que_andou() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("furo");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        let mut r = MateRef { comp: c, geom: Geom::Axis { p: [12., 10., 10.], d: [0., 0., 1.] }, pick: None, viva: Some(Viva::Encaixe { tipo: TipoEncaixe::CentroFuro }) };
        trocar(&dir, &mut cache, &bloco(40., 15., 25.));
        assert!(Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r));
        let Geom::Axis { p, d } = r.geom else { panic!("{:?}", r.geom) };
        assert!(norm(sub(p, [25., 10., 15.])) < 0.02, "{p:?}");
        assert!(d[2].abs() > 0.999);
        assert_eq!(r.viva, Some(Viva::Encaixe { tipo: TipoEncaixe::CentroFuro }));
    }

    #[test]
    fn canto_de_encaixe_fica_na_face_de_mesma_normal() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("canto");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        let mut r = MateRef { comp: c, geom: Geom::Plane { p: [40., 20., 10.], n: [1., 0., 0.] }, pick: None, viva: Some(Viva::Encaixe { tipo: TipoEncaixe::Canto }) };
        trocar(&dir, &mut cache, &bloco(60., 10., 12.));
        Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r);
        let Geom::Plane { p, n } = r.geom else { panic!() };
        assert!(norm(sub(p, [60., 20., 10.])) < 1e-3, "{p:?}");
        assert!(n[0] > 0.999, "continua na face de normal X: {n:?}");
    }

    #[test]
    fn aresta_reta_le_a_aresta_e_nao_a_face_vizinha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("aresta");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        // aresta vertical do canto (40, 20): o eixo é Z, e a altura muda de 10 para 25
        let mut r = MateRef { comp: c, geom: Geom::Axis { p: [40., 20., 0.], d: [0., 0., -1.] }, pick: None, viva: Some(Viva::Aresta { ponto: [40., 20., 5.] }) };
        trocar(&dir, &mut cache, &bloco(55., 25., 12.));
        // o bloco cresceu em X: a aresta mais perto do ponto guardado é a do outro lado do rasgo novo
        let mut r2 = r;
        r2.viva = Some(Viva::Aresta { ponto: [55., 20., 5.] });
        Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r2);
        let Geom::Axis { p, d } = r2.geom else { panic!("{:?}", r2.geom) };
        assert!(d[2] < -0.999, "sentido preservado: {d:?}");
        assert!((p[0] - 55.).abs() < 1e-3, "{p:?}");
        let Some(Viva::Aresta { ponto }) = r2.viva else { panic!() };
        assert!((ponto[2] - 12.5).abs() < 1e-3, "o ponto guardado acompanha: {ponto:?}");
        // uma aresta do furo vira o eixo do círculo
        r.viva = Some(Viva::Aresta { ponto: [15., 10., 25.] });
        Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r);
        let Geom::Axis { p, .. } = r.geom else { panic!() };
        assert!(norm(sub([p[0], p[1], 0.], [12., 10., 0.])) < 0.02, "{p:?}");
    }

    #[test]
    fn vertice_acompanha_a_peca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("vertice");
        let (a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        let mut r = MateRef { comp: c, geom: Geom::Point([40., 0., 10.]), pick: None, viva: Some(Viva::Vertice) };
        trocar(&dir, &mut cache, &bloco(42., 11., 12.));
        assert!(Leitor::novo(&a, &mut cache, Some(&dir)).reler(&mut r));
        assert_eq!(r.geom, Geom::Point([42., 0., 11.]));
    }

    #[test]
    fn peca_que_muda_leva_junto_a_restricao_e_a_junta_presas_no_ponto() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("junta");
        let (mut a, c, mut cache) = montagem(&dir, &bloco(40., 10., 12.));
        a.node_mut(c).unwrap().fixed = true;
        let pino = a.add_component("Pino", NodeKind::Part { path: "p.fkp".into() }, Placement { origin: [12., 10., 10.], ..Default::default() }, None);
        let furo = Geom::Axis { p: [12., 10., 10.], d: [0., 0., 1.] };
        let fa = JFrame::from_axis([12., 10., 10.], [0., 0., 1.]);
        let fb = JFrame::from_axis([0.; 3], [0., 0., 1.]);
        let id = a.add_joint(JointKind::Revolute, JRef { comp: c, frame: fa }, JRef { comp: pino, frame: fb });
        a.joints[0].fontes[0] = Some(MateRef { comp: c, geom: furo, pick: None, viva: Some(Viva::Encaixe { tipo: TipoEncaixe::CentroFuro }) });
        trocar(&dir, &mut cache, &bloco(40., 10., 30.));
        crate::load::refresh_refs(&mut a, &mut cache, Some(&dir));
        assert!(crate::solve::solve(&mut a).converged);
        let o = a.node(pino).unwrap().placement.origin;
        assert!(norm(sub(o, [30., 10., 10.])) < 0.02, "o pino foi com o furo: {o:?}");
        assert_eq!(a.joints[0].id, id);
    }

    #[test]
    fn sistema_da_junta_guarda_o_deslocamento_e_gira_com_o_elemento() {
        let mut f = JFrame { o: [1., 2., 3.], z: [0., 0., 1.], x: [1., 0., 0.] };
        let (antes, depois) = (Geom::Axis { p: [1., 2., 0.], d: [0., 0., 1.] }, Geom::Axis { p: [5., 2., 0.], d: [1., 0., 0.] });
        transportar(&mut f, &antes, &depois);
        assert!(norm(sub(f.o, [8., 2., 0.])) < 1e-9, "{:?}", f.o);
        assert!(norm(sub(f.z, [1., 0., 0.])) < 1e-9 && norm(sub(f.x, [0., 0., -1.])) < 1e-9, "{f:?}");
    }

    #[test]
    fn geometria_da_montagem_mantem_a_restricao_viva() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut a = Assembly::new("A");
        let plano = a.geometria_mut().add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "15");
        let base = a.add_component("Base", NodeKind::Part { path: "b.fkp".into() }, Placement::default(), None);
        a.node_mut(base).unwrap().fixed = true;
        let c = a.add_component("Tampa", NodeKind::Part { path: "t.fkp".into() }, Placement::default(), None);
        let r = fenkra_solidos::recompute(a.geometria.as_ref().unwrap());
        let g = ler_geometria(&r, plano, None).unwrap();
        assert!(matches!(g, Geom::Plane { p, .. } if (p[2] - 15.).abs() < 1e-9), "{g:?}");
        let ra = MateRef { comp: ORIGEM, geom: g, pick: None, viva: Some(Viva::Geometria { id: plano }) };
        let rb = MateRef { comp: c, geom: Geom::Plane { p: [0.; 3], n: [0., 0., -1.] }, pick: None, viva: None };
        a.add_mate(MateKind::Contact, ra, rb);
        assert!(crate::solve::solve(&mut a).converged);
        assert!((a.node(c).unwrap().placement.origin[2] - 15.).abs() < 1e-5);
        // mudar o plano leva a peça junto, sem refazer a restrição
        let doc = a.geometria.as_mut().unwrap();
        doc.references.clear();
        let novo = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "40");
        doc.references.iter_mut().find(|x| x.id == novo).unwrap().id = plano;
        crate::load::refresh_refs(&mut a, &mut PartCache::default(), None);
        assert!(crate::solve::solve(&mut a).converged);
        assert!((a.node(c).unwrap().placement.origin[2] - 40.).abs() < 1e-5, "{:?}", a.node(c).unwrap().placement.origin);
    }

    #[test]
    fn linha_e_ponto_do_sketch_da_montagem_viram_elementos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut a = Assembly::new("A");
        let mut n = 7_800_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [30., 20.], false);
        let sk = a.geometria_mut().add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        let r = fenkra_solidos::recompute(a.geometria.as_ref().unwrap());
        let g = ler_geometria(&r, sk, Some(&Geom::Point([29., 19., 0.]))).unwrap();
        assert_eq!(g, Geom::Point([30., 20., 0.]));
        let Geom::Axis { d, .. } = ler_geometria(&r, sk, Some(&Geom::Axis { p: [15., 0.1, 0.], d: [1., 0., 0.] })).unwrap() else { panic!() };
        assert!(d[0] > 0.999, "{d:?}");
    }
}
