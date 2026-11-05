// SPDX-License-Identifier: LGPL-2.1-or-later
//! Seleção por intenção (B95): a partir de uma face ou aresta clicada, junta as vizinhas que a
//! pessoa quer dizer, em vez de pedir um clique em cada uma.
//!
//! - **Tangentes**: faces ligadas por arestas suaves (o filete e o plano que ele concorda) e
//!   arestas que seguem na mesma direção na ponta (reta que vira arco sem quina).
//! - **Cadeia**: arestas ligadas ponta a ponta, mesmo com quina, até o contorno fechar ou aparecer
//!   uma bifurcação (três ou mais arestas no mesmo vértice).
//! - **Concêntricas**: círculos e superfícies curvas que giram em torno do mesmo eixo (furos e
//!   rebaixos coaxiais).
//!
//! Tudo é puro sobre a malha do kernel (`edge_smooth`, `edge_faces`, `edge_geom`, polilinhas), logo
//! testável sem tela. A regra escolhida é lembrada em `Regra` para a interface mostrar o que foi
//! feito e repetir o gesto; a gravação dela na referência v2 fica para a etapa do recompute.

use fenkra_kernel::{EdgeFormula, Mesh, NO_FACE};

/// Como a seleção cresce a partir do que foi clicado.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Regra {
    #[default]
    Nenhuma,
    Tangentes,
    Cadeia,
    Concentricas,
}

impl Regra {
    pub const TODAS: [Regra; 3] = [Regra::Tangentes, Regra::Cadeia, Regra::Concentricas];

    /// Texto do menu (traduzido em `i18n.rs`).
    pub fn rotulo(self) -> &'static str {
        match self {
            Regra::Nenhuma => "Só este",
            Regra::Tangentes => "Selecionar tangentes",
            Regra::Cadeia => "Selecionar cadeia",
            Regra::Concentricas => "Selecionar concêntricas",
        }
    }
}

/// Tolerância (mm) para dizer que duas pontas são o mesmo ponto.
const PONTA: f64 = 1e-3;
/// Cosseno mínimo entre as direções de duas arestas para a ponta contar como tangente (~14 graus).
const COS_TANGENTE: f64 = 0.97;
/// Distância máxima (mm) de um centro ao eixo para dois círculos serem coaxiais.
const TOL_EIXO: f64 = 1e-3;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norma(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let n = norma(a).max(1e-12);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Polilinha de uma aresta (na ordem dos segmentos da malha).
fn polilinha(m: &Mesh, e: usize) -> Vec<[f64; 3]> {
    let Some(&s0) = m.edge_ranges.get(e) else { return vec![] };
    let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
    let mut out: Vec<[f64; 3]> = vec![];
    for sg in m.edges.chunks_exact(6).take(s1).skip(s0 as usize) {
        if out.is_empty() {
            out.push([sg[0] as f64, sg[1] as f64, sg[2] as f64]);
        }
        out.push([sg[3] as f64, sg[4] as f64, sg[5] as f64]);
    }
    out
}

/// Pontas de cada aresta e a direção de saída em cada uma (apontando para fora da aresta).
struct Ponta {
    pos: [f64; 3],
    saindo: [f64; 3],
}

fn pontas(m: &Mesh, e: usize) -> Option<[Ponta; 2]> {
    let p = polilinha(m, e);
    if p.len() < 2 {
        return None;
    }
    let n = p.len();
    // a direção de saída olha o primeiro segmento que tem comprimento
    let ini = (1..n).find(|i| norma(sub(p[*i], p[0])) > 1e-9).map(|i| unit(sub(p[i], p[0])))?;
    let fim = (0..n - 1).rev().find(|i| norma(sub(p[*i], p[n - 1])) > 1e-9).map(|i| unit(sub(p[i], p[n - 1])))?;
    Some([Ponta { pos: p[0], saindo: ini }, Ponta { pos: p[n - 1], saindo: fim }])
}

fn mesmo_ponto(a: [f64; 3], b: [f64; 3]) -> bool {
    norma(sub(a, b)) < PONTA
}

/// Faces alcançadas a partir de `face` atravessando só arestas suaves (tangentes entre as faces).
/// Inclui a própria face; a ordem é a da busca.
pub fn faces_tangentes(m: &Mesh, face: usize) -> Vec<usize> {
    let nf = m.face_ranges.len();
    if face >= nf {
        return vec![];
    }
    let mut visto = vec![false; nf];
    let mut fila = vec![face];
    visto[face] = true;
    let mut i = 0;
    while i < fila.len() {
        let f = fila[i];
        i += 1;
        for (e, fs) in m.edge_faces.iter().enumerate() {
            if m.edge_smooth.get(e).copied().unwrap_or(0) == 0 {
                continue;
            }
            let outra = if fs[0] as usize == f && fs[1] != NO_FACE {
                fs[1]
            } else if fs[1] as usize == f && fs[0] != NO_FACE {
                fs[0]
            } else {
                continue;
            };
            let o = outra as usize;
            if o < nf && !visto[o] {
                visto[o] = true;
                fila.push(o);
            }
        }
    }
    fila
}

/// Arestas ligadas ponta a ponta a `edge`. Com `so_tangentes`, só segue quando a aresta seguinte
/// sai na mesma direção (sem quina); sem ele, segue qualquer aresta que encosta, parando nas
/// bifurcações (um vértice com mais de duas arestas não é seguido por nenhum dos ramos).
pub fn arestas_em_cadeia(m: &Mesh, edge: usize, so_tangentes: bool) -> Vec<usize> {
    let ne = m.edge_ranges.len();
    if edge >= ne {
        return vec![];
    }
    let todas: Vec<Option<[Ponta; 2]>> = (0..ne).map(|e| pontas(m, e)).collect();
    if todas[edge].is_none() {
        return vec![edge];
    }
    // quantas pontas de aresta caem em cada ponto (para reconhecer bifurcação)
    let grau = |p: [f64; 3]| todas.iter().flatten().flat_map(|a| a.iter()).filter(|q| mesmo_ponto(q.pos, p)).count();
    let mut cadeia = vec![edge];
    // cada ponta aberta da aresta inicial é seguida para um lado
    for lado in 0..2 {
        let mut atual = edge;
        let mut ponta_atual = lado;
        loop {
            let Some(pa) = todas[atual].as_ref().map(|a| (a[ponta_atual].pos, a[ponta_atual].saindo)) else { break };
            if !so_tangentes && grau(pa.0) != 2 {
                break;
            }
            let mut achada = None;
            for (e, ps) in todas.iter().enumerate() {
                if e == atual || cadeia.contains(&e) {
                    continue;
                }
                let Some(ps) = ps else { continue };
                for (k, q) in ps.iter().enumerate() {
                    if !mesmo_ponto(q.pos, pa.0) {
                        continue;
                    }
                    // na ponta em comum as duas arestas saem em sentidos opostos quando são tangentes
                    if so_tangentes && dot(pa.1, q.saindo) > -COS_TANGENTE {
                        continue;
                    }
                    achada = Some((e, 1 - k));
                    break;
                }
                if achada.is_some() {
                    break;
                }
            }
            let Some((e, outra)) = achada else { break };
            cadeia.push(e);
            atual = e;
            ponta_atual = outra;
        }
    }
    cadeia
}

/// Eixo (ponto e direção unitária) de um círculo da malha.
fn eixo_do_circulo(m: &Mesh, e: usize) -> Option<([f64; 3], [f64; 3])> {
    match m.edge_formula(e)? {
        EdgeFormula::Conic { c, u, v, .. } => {
            let n = cross(u, v);
            (norma(n) > 1e-12).then(|| (c, unit(n)))
        }
        EdgeFormula::Line { .. } => None,
    }
}

fn sobre_o_eixo(eixo: ([f64; 3], [f64; 3]), p: [f64; 3]) -> bool {
    let d = sub(p, eixo.0);
    norma(cross(d, eixo.1)) < TOL_EIXO
}

/// Arestas circulares coaxiais com a de `edge` (inclui ela). Aresta que não é círculo: só ela.
pub fn arestas_concentricas(m: &Mesh, edge: usize) -> Vec<usize> {
    let Some(eixo) = eixo_do_circulo(m, edge) else { return vec![edge] };
    (0..m.edge_ranges.len())
        .filter(|e| *e == edge || eixo_do_circulo(m, *e).is_some_and(|(c, n)| dot(n, eixo.1).abs() > 0.999 && sobre_o_eixo(eixo, c)))
        .collect()
}

/// Faces curvas que giram em torno do mesmo eixo da face `face` (inclui ela). O eixo vem dos
/// círculos de borda da face; face sem círculo de borda (plano, por exemplo): só ela.
pub fn faces_concentricas(m: &Mesh, face: usize) -> Vec<usize> {
    let nf = m.face_ranges.len();
    if face >= nf {
        return vec![];
    }
    let bordas_de = |f: usize| (0..m.edge_faces.len()).filter(move |e| m.edge_faces[*e].contains(&(f as u32)));
    let Some(eixo) = bordas_de(face).find_map(|e| eixo_do_circulo(m, e)) else { return vec![face] };
    let mut out = vec![face];
    for e in 0..m.edge_faces.len() {
        let Some((c, n)) = eixo_do_circulo(m, e) else { continue };
        if dot(n, eixo.1).abs() < 0.999 || !sobre_o_eixo(eixo, c) {
            continue;
        }
        for f in m.edge_faces[e] {
            let f = f as usize;
            // só superfícies curvas: o plano que fecha um furo não gira em torno do eixo
            if f < nf && !out.contains(&f) && m.face_info.get(f).is_some_and(|i| i.kind != 0) {
                out.push(f);
            }
        }
    }
    out
}

/// Aplica a regra à face ou aresta clicada e devolve tudo que entra na seleção (o clicado vem
/// primeiro). Outros tipos de escolha, ou regra sem sentido para o tipo (cadeia numa face), devolvem
/// só o que foi clicado.
pub fn por_regra(r: &fenkra_solidos::RecomputeResult, p: &crate::picking::Pick, regra: Regra) -> Vec<crate::picking::Pick> {
    use crate::picking::{mesh_of, Pick};
    let so_ele = || vec![*p];
    let primeiro = |p: &Pick, mut v: Vec<Pick>| {
        let mesma = |x: &Pick| match (x, p) {
            (Pick::Face { body: b1, face: f1, .. }, Pick::Face { body: b2, face: f2, .. }) => b1 == b2 && f1 == f2,
            (Pick::Edge { body: b1, edge: e1, .. }, Pick::Edge { body: b2, edge: e2, .. }) => b1 == b2 && e1 == e2,
            _ => x == p,
        };
        if let Some(i) = v.iter().position(mesma) {
            v.swap(0, i);
        } else {
            v.insert(0, *p);
        }
        v
    };
    match *p {
        Pick::Face { body, face, .. } => {
            let Some(m) = mesh_of(r, body) else { return so_ele() };
            let faces = match regra {
                Regra::Tangentes => faces_tangentes(m, face),
                Regra::Concentricas => faces_concentricas(m, face),
                _ => return so_ele(),
            };
            let donos = r.bodies.get(body).map(|b| b.face_owner.as_slice()).unwrap_or(&[]);
            let v = faces
                .into_iter()
                .filter_map(|f| {
                    let info = m.face_info.get(f)?;
                    let dir = [info.dir[0] as f32, info.dir[1] as f32, info.dir[2] as f32];
                    Some(Pick::Face { body, face: f, owner: donos.get(f).copied().unwrap_or(fenkra_core::Id(0)), point: info.point.map(|v| v as f32), normal: dir })
                })
                .collect();
            primeiro(p, v)
        }
        Pick::Edge { body, edge, .. } => {
            let Some(m) = mesh_of(r, body) else { return so_ele() };
            let arestas = match regra {
                Regra::Tangentes => arestas_em_cadeia(m, edge, true),
                Regra::Cadeia => arestas_em_cadeia(m, edge, false),
                Regra::Concentricas => arestas_concentricas(m, edge),
                Regra::Nenhuma => return so_ele(),
            };
            let v = arestas
                .into_iter()
                .map(|e| {
                    let mid = m.edge_mids.get(e * 3..e * 3 + 3).map(|x| [x[0], x[1], x[2]]).unwrap_or([0.; 3]);
                    Pick::Edge { body, edge: e, mid }
                })
                .collect();
            primeiro(p, v)
        }
        _ => so_ele(),
    }
}

/// Textos novos do menu e das mensagens (inglês), somados à tabela de `i18n.rs`.
pub static TEXTOS: &[(&str, &str)] = &[
    ("Selecionar outro   Alt + roda", "Select another   Alt + wheel"),
    ("Selecionar tangentes", "Select tangent"),
    ("Selecionar cadeia", "Select chain"),
    ("Selecionar concêntricas", "Select concentric"),
    ("Seleção por laço", "Lasso selection"),
    ("Laço: arraste no 3D para circundar o que quer selecionar", "Lasso: drag in the 3D view around what you want to select"),
    ("Laço desligado", "Lasso off"),
    ("elemento(s) na seleção", "item(s) selected"),
    ("Filtro temporário", "Temporary filter"),
    ("faces", "faces"),
    ("arestas", "edges"),
    ("vértices", "vertices"),
    ("faces, arestas e vértices", "faces, edges and vertices"),
    ("Mostrar origem", "Show origin"),
    ("Ocultar origem", "Hide origin"),
    ("a origem está oculta: os planos voltam com ela", "the origin is hidden: the planes come back with it"),
    ("a pasta está oculta: este item mantém o próprio estado", "the folder is hidden: this item keeps its own state"),
];

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::{BoolOp, Document, EdgeOp, EdgeSel, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData};

    /// Bloco 40 x 40 x 20 com furo passante de 10 em um canto e as quatro arestas verticais
    /// arredondadas: dá faces tangentes (cantos), círculos coaxiais (furo) e arestas em cadeia.
    fn bloco_com_furo() -> Option<(Document, fenkra_solidos::RecomputeResult)> {
        if !fenkra_kernel::AVAILABLE {
            return None;
        }
        let mut d = Document::new("bloco");
        let mut n = 91_000_000u64;
        let mut al = move || {
            n += 1;
            fenkra_core::Id(n)
        };
        let mut data = SketchData::default();
        data.add_rect(&mut al, [0., 0.], [40., 40.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        let ex = Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() };
        d.add_feature(FeatureKind::Extrude(ex));
        let mut furo = SketchData::default();
        let c = furo.add_point(&mut al, 20., 20.);
        furo.add_curve(&mut al, fenkra_core::sketch::Curve::Circle { c, r: 5. }, false);
        let sk2 = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: furo }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk2, op: BoolOp::Cut, extent: Extent::ThroughAll, reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        // canto (40, 40) arredondado: a aresta vertical em z = 10
        d.add_feature(FeatureKind::Fillet(EdgeOp { value: "5".into(), edges: EdgeSel::Edges(vec![[40., 40., 10.]]), mode: Default::default(), prop: Default::default(), keys: vec![] }));
        let r = fenkra_solidos::recompute(&d);
        Some((d, r))
    }

    fn malha(r: &fenkra_solidos::RecomputeResult) -> &Mesh {
        &r.bodies.first().expect("um corpo").mesh
    }

    #[test]
    fn concentricas_acham_os_dois_circulos_do_furo() {
        let Some((_, r)) = bloco_com_furo() else { return };
        let m = malha(&r);
        let circ: Vec<usize> = (0..m.edge_ranges.len()).filter(|e| eixo_do_circulo(m, *e).is_some()).collect();
        assert_eq!(circ.len(), 2, "furo passante tem borda no topo e no fundo");
        let c = arestas_concentricas(m, circ[0]);
        assert_eq!(c.len(), 2, "{c:?}");
        // uma aresta reta não é círculo: só ela
        let reta = (0..m.edge_ranges.len()).find(|e| eixo_do_circulo(m, *e).is_none()).unwrap();
        assert_eq!(arestas_concentricas(m, reta), vec![reta]);
    }

    #[test]
    fn faces_concentricas_ignoram_os_planos() {
        let Some((_, r)) = bloco_com_furo() else { return };
        let m = malha(&r);
        let cil = (0..m.face_info.len()).find(|f| m.face_info[*f].kind != 0).expect("parede do furo");
        let c = faces_concentricas(m, cil);
        assert_eq!(c, vec![cil], "só uma parede curva em torno desse eixo");
        let plano = (0..m.face_info.len()).find(|f| m.face_info[*f].kind == 0).unwrap();
        assert_eq!(faces_concentricas(m, plano).len(), 1, "o plano do topo tem o círculo mas não gira em torno dele");
    }

    #[test]
    fn cadeia_segue_o_contorno_do_topo_mesmo_com_quina() {
        let Some((_, r)) = bloco_com_furo() else { return };
        let m = malha(&r);
        // uma aresta do topo do bloco: z = 20, reta
        let topo = (0..m.edge_ranges.len())
            .find(|e| matches!(m.edge_formula(*e), Some(EdgeFormula::Line { a, b }) if (a[2] - 20.).abs() < 1e-6 && (b[2] - 20.).abs() < 1e-6 && a[1].abs() < 1e-6 && b[1].abs() < 1e-6))
            .expect("aresta do topo");
        let com_quina = arestas_em_cadeia(m, topo, false);
        // nos cantos do bloco chegam 3 arestas (duas do topo e a vertical): bifurcação, a cadeia
        // não atravessa; a seleção fica só nela
        assert_eq!(com_quina, vec![topo], "{com_quina:?}");
        // tangente: a quina de 90 graus também não passa
        assert_eq!(arestas_em_cadeia(m, topo, true), vec![topo]);
    }

    #[test]
    fn faces_tangentes_juntam_as_paredes_e_o_filete_do_canto() {
        let Some((_, r)) = bloco_com_furo() else { return };
        let m = malha(&r);
        let curva = (0..m.face_info.len())
            .find(|f| m.face_info[*f].kind != 0 && (m.face_info[*f].point[0] - 40.).abs() < 6. && (m.face_info[*f].point[1] - 40.).abs() < 6.)
            .expect("face do filete");
        let g = faces_tangentes(m, curva);
        assert_eq!(g.len(), 3, "parede x + filete + parede y: {g:?}");
        assert!(g.contains(&curva));
        // o topo é plano e fica de fora (a aresta dele com as paredes é viva)
        assert!(g.iter().all(|f| m.face_info[*f].kind != 0 || m.face_info[*f].dir[2].abs() < 1e-6), "nenhuma tampa plana (topo/fundo) entra: {g:?}");
        assert!(faces_tangentes(m, 999).is_empty());
    }

    #[test]
    fn cadeia_tangente_passa_pelo_arco_do_canto() {
        let Some((_, r)) = bloco_com_furo() else { return };
        let m = malha(&r);
        let topo = (0..m.edge_ranges.len())
            .find(|e| matches!(m.edge_formula(*e), Some(EdgeFormula::Line { a, b }) if (a[2] - 20.).abs() < 1e-6 && (b[2] - 20.).abs() < 1e-6 && (a[0] - 40.).abs() < 1e-6 && (b[0] - 40.).abs() < 1e-6))
            .expect("aresta de cima da parede x");
        let c = arestas_em_cadeia(m, topo, true);
        assert_eq!(c.len(), 3, "reta + arco + reta sem quina: {c:?}");
    }

    #[test]
    fn tangentes_atravessam_arestas_suaves() {
        // malha sintética: três faces em fila, 0-1 ligadas por aresta suave, 1-2 por aresta viva
        let m = Mesh {
            face_ranges: vec![0, 3, 6],
            edge_faces: vec![[0, 1], [1, 2]],
            edge_smooth: vec![1, 0],
            ..Default::default()
        };
        assert_eq!(faces_tangentes(&m, 0), vec![0, 1]);
        assert_eq!(faces_tangentes(&m, 2), vec![2]);
        assert_eq!(faces_tangentes(&m, 1), vec![1, 0]);
    }

    /// Três arestas retas em fila (a, b, c); b faz quina com c. A cadeia tangente para na quina;
    /// a cadeia simples atravessa.
    fn tres_arestas() -> Mesh {
        let seg = |a: [f32; 3], b: [f32; 3]| [a[0], a[1], a[2], b[0], b[1], b[2]];
        let mut edges = vec![];
        edges.extend(seg([0., 0., 0.], [10., 0., 0.]));
        edges.extend(seg([10., 0., 0.], [20., 0., 0.]));
        edges.extend(seg([20., 0., 0.], [20., 10., 0.]));
        Mesh { edges, edge_ranges: vec![0, 1, 2], ..Default::default() }
    }

    #[test]
    fn cadeia_tangente_para_na_quina_e_a_simples_atravessa() {
        let m = tres_arestas();
        let mut t = arestas_em_cadeia(&m, 0, true);
        t.sort();
        assert_eq!(t, vec![0, 1], "reta + reta alinhadas, a quina de 90 graus não passa");
        let mut c = arestas_em_cadeia(&m, 0, false);
        c.sort();
        assert_eq!(c, vec![0, 1, 2], "cadeia simples segue até o fim");
    }

    #[test]
    fn bifurcacao_interrompe_a_cadeia_simples() {
        let mut m = tres_arestas();
        // uma quarta aresta sai do mesmo ponto (10,0,0): o vértice passa a ter três arestas
        m.edges.extend([10., 0., 0., 10., 0., 5.]);
        m.edge_ranges.push(3);
        assert_eq!(arestas_em_cadeia(&m, 0, false), vec![0], "em (10,0,0) há bifurcação");
    }

    #[test]
    fn regra_do_menu_devolve_o_clicado_primeiro_e_so_ele_quando_nao_se_aplica() {
        let Some((_, r)) = bloco_com_furo() else { return };
        let m = malha(&r);
        let curva = (0..m.face_info.len())
            .find(|f| m.face_info[*f].kind != 0 && (m.face_info[*f].point[0] - 40.).abs() < 6. && (m.face_info[*f].point[1] - 40.).abs() < 6.)
            .unwrap();
        let clicada = crate::picking::Pick::Face { body: 0, face: curva, owner: r.bodies[0].face_owner[curva], point: [0.; 3], normal: [0.; 3] };
        let v = por_regra(&r, &clicada, Regra::Tangentes);
        assert_eq!(v.len(), 3);
        assert!(matches!(v[0], crate::picking::Pick::Face { face, .. } if face == curva), "o clicado fica na frente");
        assert_eq!(por_regra(&r, &clicada, Regra::Cadeia), vec![clicada], "cadeia não tem sentido para face");
        let aresta = crate::picking::Pick::Edge { body: 0, edge: 0, mid: [0.; 3] };
        assert_eq!(por_regra(&r, &aresta, Regra::Nenhuma), vec![aresta]);
        assert_eq!(por_regra(&r, &crate::picking::Pick::Geo(fenkra_core::Id(5)), Regra::Tangentes).len(), 1);
    }
}
