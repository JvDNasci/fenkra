// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pontos de encaixe de uma face (B99): os lugares em que faz sentido prender uma peça em outra.
//!
//! Como funciona: a malha já traz, para cada aresta, as duas faces vizinhas. Para a face sob o cursor
//! olhamos só as arestas dela:
//! - aresta aberta dá os dois cantos e o meio;
//! - aresta fechada é um círculo: o centro dela é **centro de furo** numa face plana e ponto do
//!   **eixo do cilindro** numa face curva;
//! - face plana dá ainda o **centro de face**, pelo centroide dos triângulos.
//!
//! Cada candidato traz um sistema de eixos: Z é a normal da face (ou o eixo do cilindro) e X segue a
//! primeira aresta reta da face. É esse sistema que aparece na vista e que vira o elemento do comando.

use crate::*;
use fenkra_kernel::Mesh;

/// Aresta com comprimento abaixo disto não dá candidato (mm).
const MIN_ARESTA: f64 = 1e-3;
/// Dois candidatos mais perto que isto são o mesmo (mm).
const MESMO_PONTO: f64 = 1e-3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TipoEncaixe {
    CentroFace,
    MeioAresta,
    Canto,
    EixoCilindro,
    CentroFuro,
}

impl TipoEncaixe {
    pub fn label(self) -> &'static str {
        match self {
            TipoEncaixe::CentroFace => "Centro de face",
            TipoEncaixe::MeioAresta => "Meio de aresta",
            TipoEncaixe::Canto => "Canto",
            TipoEncaixe::EixoCilindro => "Eixo de cilindro",
            TipoEncaixe::CentroFuro => "Centro de furo",
        }
    }

    /// O elemento que o candidato vira no comando é um eixo? Nos demais, um plano no ponto.
    pub fn e_eixo(self) -> bool {
        matches!(self, TipoEncaixe::EixoCilindro | TipoEncaixe::CentroFuro)
    }
}

/// Ponto de encaixe, com o sistema de eixos dele.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Candidato {
    pub tipo: TipoEncaixe,
    pub p: V3,
    pub z: V3,
    pub x: V3,
}

fn v(m: &Mesh, i: u32) -> V3 {
    let k = 3 * i as usize;
    [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
}

fn dist(a: V3, b: V3) -> f64 {
    norm(sub(a, b))
}

/// Polilinha da aresta `e`.
fn polilinha(m: &Mesh, e: usize) -> Vec<V3> {
    let nseg = m.edges.len() / 6;
    let (s0, s1) = (m.edge_ranges[e] as usize, m.edge_ranges.get(e + 1).map_or(nseg, |x| *x as usize));
    let mut pts = vec![];
    for s in m.edges[s0 * 6..s1 * 6].chunks_exact(6) {
        let (a, b) = ([s[0] as f64, s[1] as f64, s[2] as f64], [s[3] as f64, s[4] as f64, s[5] as f64]);
        if pts.is_empty() {
            pts.push(a);
        }
        pts.push(b);
    }
    pts
}

fn comprimento(p: &[V3]) -> f64 {
    p.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Ponto a meio caminho da polilinha.
fn meio(p: &[V3]) -> V3 {
    let alvo = comprimento(p) / 2.;
    let mut andou = 0.;
    for w in p.windows(2) {
        let d = dist(w[0], w[1]);
        if andou + d >= alvo && d > 0. {
            let t = (alvo - andou) / d;
            return [0, 1, 2].map(|i| w[0][i] + (w[1][i] - w[0][i]) * t);
        }
        andou += d;
    }
    p[0]
}

/// Normal do laço fechado (Newell).
fn normal_do_laco(p: &[V3]) -> V3 {
    let mut n = [0.; 3];
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    unit(n)
}

/// Centroide e normal da face, se ela é plana. Curva devolve `None` no centro.
fn plano_da_face(m: &Mesh, face: usize) -> (Option<V3>, V3) {
    let (i0, i1) = (m.face_ranges[face] as usize, m.face_ranges.get(face + 1).map_or(m.indices.len(), |x| *x as usize));
    let (mut area, mut c, mut n, mut plana) = (0., [0.; 3], None::<V3>, true);
    for t in m.indices[i0..i1].chunks_exact(3) {
        let (a, b, d) = (v(m, t[0]), v(m, t[1]), v(m, t[2]));
        let cr = cross(sub(b, a), sub(d, a));
        let dois = norm(cr);
        if dois < 1e-12 {
            continue;
        }
        let u = cr.map(|x| x / dois);
        match n {
            None => n = Some(u),
            Some(q) if dot(q, u) < 1. - 1e-4 => plana = false,
            _ => {}
        }
        area += dois;
        for k in 0..3 {
            c[k] += dois * (a[k] + b[k] + d[k]) / 3.;
        }
    }
    let n = n.unwrap_or([0., 0., 1.]);
    (plana.then(|| c.map(|x| x / area.max(1e-12))).filter(|_| area > 0.), n)
}

/// Candidatos da face `face` da malha, no sistema dela.
pub fn candidatos(m: &Mesh, face: usize) -> Vec<Candidato> {
    if face >= m.face_ranges.len() {
        return vec![];
    }
    let (centro, n) = plano_da_face(m, face);
    let plana = centro.is_some();
    let arestas: Vec<Vec<V3>> = (0..m.edge_ranges.len()).filter(|e| m.edge_faces.get(*e).is_some_and(|f| f.contains(&(face as u32)))).map(|e| polilinha(m, e)).filter(|p| p.len() >= 2 && comprimento(p) > MIN_ARESTA).collect();
    // X segue a primeira aresta reta; sem nenhuma, qualquer perpendicular
    let reta = arestas.iter().find(|p| (comprimento(p) - dist(p[0], p[p.len() - 1])).abs() < 1e-6 * comprimento(p).max(1.)).map(|p| unit(sub(p[p.len() - 1], p[0])));
    let eixos = |z: V3| {
        let x = reta.filter(|x| dot(*x, z).abs() < 0.99).map(|x| unit(sub(x, z.map(|c| c * dot(x, z))))).unwrap_or_else(|| any_perp(z));
        (z, x)
    };
    let mut v = vec![];
    let mut por = |tipo: TipoEncaixe, p: V3, z: V3, x: V3| {
        match v.iter_mut().find(|c: &&mut Candidato| dist(c.p, p) < MESMO_PONTO) {
            // furo no meio exato da face: o furo diz mais que o centro
            Some(c) if c.tipo == TipoEncaixe::CentroFace && tipo.e_eixo() => *c = Candidato { tipo, p, z, x },
            Some(_) => {}
            None => v.push(Candidato { tipo, p, z, x }),
        }
    };
    if let Some(c) = centro {
        let (z, x) = eixos(n);
        por(TipoEncaixe::CentroFace, c, z, x);
    }
    for a in &arestas {
        let (p0, p1) = (a[0], a[a.len() - 1]);
        if dist(p0, p1) < 1e-6 * comprimento(a).max(1.) {
            // laço fechado: círculo
            let aberto = &a[..a.len() - 1];
            let c = aberto.iter().fold([0.; 3], |s, q| [s[0] + q[0], s[1] + q[1], s[2] + q[2]]).map(|x| x / aberto.len() as f64);
            let mut z = normal_do_laco(aberto);
            if plana && dot(z, n) < 0. {
                z = z.map(|x| -x);
            }
            let (z, x) = eixos(z);
            por(if plana { TipoEncaixe::CentroFuro } else { TipoEncaixe::EixoCilindro }, c, z, x);
            continue;
        }
        let (z, x) = eixos(n);
        por(TipoEncaixe::Canto, p0, z, x);
        por(TipoEncaixe::Canto, p1, z, x);
        por(TipoEncaixe::MeioAresta, meio(a), z, x);
    }
    v
}

/// Desloca a origem do candidato nos eixos dele, com Y = Z × X.
pub fn deslocado(c: &Candidato, d: V3) -> V3 {
    let y = cross(c.z, c.x);
    [0, 1, 2].map(|i| c.p[i] + c.x[i] * d[0] + y[i] * d[1] + c.z[i] * d[2])
}

/// Geometria que o candidato vira no comando, no sistema de `pl`.
pub fn geom_local(c: &Candidato, pl: &Placement) -> Geom {
    let (p, z) = (pl.to_local(c.p), unit(pl.inverse().dir(c.z)));
    if c.tipo.e_eixo() {
        Geom::Axis { p, d: z }
    } else {
        Geom::Plane { p, n: z }
    }
}

/// Valor das três fórmulas de deslocamento; texto que não avalia conta como zero e devolve o índice.
pub fn avaliar_desloc(f: &[String; 3], vars: &HashMap<String, f64>) -> (V3, Option<usize>) {
    let mut d = [0.; 3];
    let mut erro = None;
    for k in 0..3 {
        if f[k].trim().is_empty() {
            continue;
        }
        match expr::eval_str(&f[k], vars) {
            Ok(x) => d[k] = x,
            Err(_) => erro = erro.or(Some(k)),
        }
    }
    (d, erro)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{sketch::Curve, BoolOp, Document, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};

    /// Bloco de 40 × 20 × 10 com um furo Ø6 fora do centro.
    fn bloco() -> Mesh {
        let mut d = Document::new("Bloco");
        let mut n = 7_500_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [0., 0.], [40., 20.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
        let mut c = SketchData::default();
        let p = c.add_point(&mut al, 12., 10.);
        c.add_curve(&mut al, Curve::Point { p }, false);
        let sc = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        d.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: "6".into(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        fenkra_solidos::recompute(&d).bodies.remove(0).mesh
    }

    fn face_com(m: &Mesh, teste: impl Fn(&[Candidato]) -> bool) -> Vec<Candidato> {
        (0..m.face_ranges.len()).map(|f| candidatos(m, f)).find(|c| teste(c)).expect("face")
    }

    #[test]
    fn face_de_cima_da_centro_cantos_meios_e_centro_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let m = bloco();
        let c = face_com(&m, |c| c.iter().any(|x| x.tipo == TipoEncaixe::CentroFuro && (x.p[2] - 10.).abs() < 1e-3));
        let conta = |t| c.iter().filter(|x| x.tipo == t).count();
        assert_eq!(conta(TipoEncaixe::Canto), 4);
        assert_eq!(conta(TipoEncaixe::MeioAresta), 4);
        assert_eq!(conta(TipoEncaixe::CentroFuro), 1);
        assert_eq!(conta(TipoEncaixe::CentroFace), 1);
        let furo = c.iter().find(|x| x.tipo == TipoEncaixe::CentroFuro).unwrap();
        assert!(dist(furo.p, [12., 10., 10.]) < 0.01, "{:?}", furo.p);
        assert!(furo.z[2].abs() > 0.999);
        assert!(c.iter().any(|x| x.tipo == TipoEncaixe::Canto && dist(x.p, [40., 20., 10.]) < 1e-3));
        assert!(c.iter().any(|x| x.tipo == TipoEncaixe::MeioAresta && dist(x.p, [20., 0., 10.]) < 1e-3));
    }

    #[test]
    fn eixos_do_candidato_sao_ortonormais_e_x_segue_a_aresta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let m = bloco();
        for f in 0..m.face_ranges.len() {
            for c in candidatos(&m, f) {
                assert!((norm(c.z) - 1.).abs() < 1e-6 && (norm(c.x) - 1.).abs() < 1e-6);
                assert!(dot(c.z, c.x).abs() < 1e-6, "{c:?}");
            }
        }
        let c = face_com(&m, |c| c.iter().any(|x| x.tipo == TipoEncaixe::CentroFuro));
        assert!(c[0].x.iter().filter(|x| x.abs() > 0.999).count() == 1, "X na direção de uma aresta: {:?}", c[0].x);
    }

    #[test]
    fn parede_do_furo_da_o_eixo_do_cilindro() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let m = bloco();
        let c = face_com(&m, |c| c.iter().any(|x| x.tipo == TipoEncaixe::EixoCilindro));
        let e = c.iter().find(|x| x.tipo == TipoEncaixe::EixoCilindro).unwrap();
        assert!(dist([e.p[0], e.p[1], 0.], [12., 10., 0.]) < 0.01, "{:?}", e.p);
        assert!(e.z[2].abs() > 0.999, "eixo em Z: {:?}", e.z);
        assert!(e.tipo.e_eixo());
    }

    #[test]
    fn deslocamento_vai_nos_eixos_do_candidato() {
        let c = Candidato { tipo: TipoEncaixe::Canto, p: [1., 2., 3.], z: [0., 0., 1.], x: [1., 0., 0.] };
        assert_eq!(deslocado(&c, [10., 20., 30.]), [11., 22., 33.]);
    }

    #[test]
    fn geometria_local_desfaz_a_posicao_da_peca() {
        let c = Candidato { tipo: TipoEncaixe::CentroFuro, p: [11., 2., 3.], z: [0., 0., 1.], x: [1., 0., 0.] };
        let pl = Placement { origin: [10., 0., 0.], ..Default::default() };
        assert_eq!(geom_local(&c, &pl), Geom::Axis { p: [1., 2., 3.], d: [0., 0., 1.] });
        let c = Candidato { tipo: TipoEncaixe::Canto, ..c };
        assert!(matches!(geom_local(&c, &pl), Geom::Plane { .. }));
    }

    #[test]
    fn junta_rigida_leva_um_ponto_ao_outro_com_giro_inverter_e_deslocamento() {
        use crate::kin::JointKind;
        use crate::proposta::{Classe, Comando, Elemento, Tipo};
        let mut a = Assembly::new("t");
        let base = a.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        let peca = a.add_component("B", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, None);
        a.node_mut(base).unwrap().fixed = true;
        let ca = Candidato { tipo: TipoEncaixe::CentroFuro, p: [20., 10., 10.], z: [0., 0., 1.], x: [1., 0., 0.] };
        let cb = Candidato { tipo: TipoEncaixe::CentroFuro, p: [105., 5., 0.], z: [0., 0., 1.], x: [1., 0., 0.] };
        let el = |comp: Id, c: &Candidato| Elemento::novo(MateRef { comp, geom: geom_local(c, &a.node(comp).unwrap().placement), pick: None }, Classe::Face);
        let mut c = Comando::novo(Tipo::Junta(JointKind::Rigid));
        c.receber(el(base, &ca));
        c.receber(el(peca, &cb));
        let ponto = |c: &Comando| {
            let p = c.resolver(&a).expect("resolve");
            assert!(!p.falhou());
            p.copia.node(peca).unwrap().placement.point([5., 5., 0.])
        };
        assert!(dist(ponto(&c), ca.p) < 1e-5, "os dois pontos se encontram: {:?}", ponto(&c));
        c.alinhamento.desloc = [0., 0., 7.];
        assert!((dist(ponto(&c), ca.p) - 7.).abs() < 1e-5);
        c.alinhamento.desloc = [0.; 3];
        c.alinhamento.angulo = 90.;
        let r = c.resolver(&a).unwrap();
        let x = r.copia.node(peca).unwrap().placement.dir([1., 0., 0.]);
        assert!(x[0].abs() < 1e-5, "girou 90°: {x:?}");
        c.alinhamento.angulo = 0.;
        c.alinhamento.inverter = true;
        let r = c.resolver(&a).unwrap();
        let z = r.copia.node(peca).unwrap().placement.dir([0., 0., 1.]);
        assert!(z[2] < -0.999, "inverteu o eixo principal: {z:?}");
    }

    #[test]
    fn formulas_usam_os_parametros_e_avisam_o_campo_ruim() {
        let vars: HashMap<String, f64> = [("folga".to_string(), 2.)].into_iter().collect();
        let (d, e) = avaliar_desloc(&["folga*3".into(), "".into(), "1+".into()], &vars);
        assert_eq!(d, [6., 0., 0.]);
        assert_eq!(e, Some(2));
    }
}
