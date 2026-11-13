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

// as funções sobre a malha moraram aqui; foram para o `fenkra-solidos` para o recompute refazer a
// regra gravada no filete e no chanfro (B95)
pub use fenkra_solidos::selecao_regras::{arestas_concentricas, arestas_em_cadeia, eixo_do_circulo, faces_concentricas, faces_tangentes};

impl Regra {
    /// A regra como fica gravada no filete ou chanfro (B95).
    pub fn gravada(self) -> Option<fenkra_core::RegraSelecao> {
        match self {
            Regra::Tangentes => Some(fenkra_core::RegraSelecao::Tangentes),
            Regra::Cadeia => Some(fenkra_core::RegraSelecao::Cadeia),
            Regra::Concentricas => Some(fenkra_core::RegraSelecao::Concentricas),
            Regra::Nenhuma => None,
        }
    }
}

/// Faces da seleção atual como o comando de face as grava (B95: casca, saída, edição direta). Se a
/// seleção é a que uma regra de faces (tangentes, concêntricas) acabou de produzir, fica só a face
/// semente e a regra, refeita a cada recálculo; senão, a lista explícita sem regra.
pub fn faces_com_regra(picks: &[crate::picking::Pick], faces: &[[f64; 3]], keys: &[Option<fenkra_core::FaceKey>]) -> (Vec<[f64; 3]>, Vec<Option<fenkra_core::FaceKey>>, Vec<fenkra_core::RegraAresta>) {
    let de_face = matches!(picks.first(), Some(crate::picking::Pick::Face { .. }));
    let regra = regra_da_selecao(picks).filter(|r| matches!(r, Regra::Tangentes | Regra::Concentricas)).and_then(|r| r.gravada());
    match regra {
        Some(g) if de_face && !faces.is_empty() => (faces[..1].to_vec(), keys.iter().take(1).cloned().collect(), vec![fenkra_core::RegraAresta { semente: 0, regra: g }]),
        _ => (faces.to_vec(), keys.to_vec(), vec![]),
    }
}

/// Nome da regra gravada, para o painel do comando.
pub fn nome_gravada(r: fenkra_core::RegraSelecao) -> &'static str {
    match r {
        fenkra_core::RegraSelecao::Tangentes => "tangentes",
        fenkra_core::RegraSelecao::Cadeia => "cadeia",
        fenkra_core::RegraSelecao::Concentricas => "concêntricas",
    }
}

/// Linha do painel dos comandos de face com a regra gravada (B95) e o botão de soltá-la.
pub fn regra_faces_ui(ui: &mut eframe::egui::Ui, regras: &mut Vec<fenkra_core::RegraAresta>, faces: &[[f64; 3]]) {
    use crate::i18n::tr;
    let Some(r) = regras.first().copied() else { return };
    let mut soltar = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(eframe::egui::RichText::new(format!("{}: {} {} {}", tr("Regra gravada"), tr(nome_gravada(r.regra)), tr("a partir da face"), faces.get(r.semente).map(|p| crate::app::p3(p, 1)).unwrap_or_default())).small())
            .on_hover_text(tr("A cada recálculo a regra é refeita a partir da face semente: faces que a mudança do modelo criar entram sozinhas."));
        soltar = ui.small_button(tr("Soltar a regra")).on_hover_text(tr("Deixa só as faces da lista, sem refazer a regra.")).clicked();
    });
    if soltar {
        regras.clear();
    }
}

/// Mantém as regras ligadas à mesma face semente quando a lista muda no painel (B95): a semente
/// que saiu da lista leva a regra junto; as outras seguem o novo índice.
pub fn acompanhar_sementes(regras: &mut Vec<fenkra_core::RegraAresta>, antes: &[[f64; 3]], depois: &[[f64; 3]]) {
    regras.retain_mut(|r| {
        let Some(p) = antes.get(r.semente) else { return false };
        match depois.iter().position(|q| q == p) {
            Some(i) => {
                r.semente = i;
                true
            }
            None => false,
        }
    });
}

thread_local! {
    /// A última seleção feita por regra e a regra usada: se a seleção ainda é essa quando o filete
    /// nasce, ele grava só a semente e a regra (B95).
    static ULTIMA: std::cell::RefCell<Option<(Vec<crate::picking::Pick>, Regra)>> = const { std::cell::RefCell::new(None) };
}

/// Guarda a seleção que a regra acabou de produzir (a semente vem primeiro).
pub fn lembrar(picks: &[crate::picking::Pick], regra: Regra) {
    ULTIMA.with(|u| *u.borrow_mut() = Some((picks.to_vec(), regra)));
}

/// A seleção atual é a que a regra produziu? Então devolve a regra; qualquer clique depois disso
/// (somar ou tirar uma aresta) desfaz a ligação e a lista fica explícita.
pub fn regra_da_selecao(picks: &[crate::picking::Pick]) -> Option<Regra> {
    ULTIMA.with(|u| u.borrow().as_ref().filter(|(p, _)| p.as_slice() == picks && !picks.is_empty()).map(|x| x.1))
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
    ("tangentes", "tangent"),
    ("cadeia", "chain"),
    ("concêntricas", "concentric"),
    ("Regra gravada", "Saved rule"),
    ("a partir da aresta", "from edge"),
    ("A cada recálculo a regra é refeita a partir da aresta semente: arestas que a mudança do modelo criar ou mover entram sozinhas.", "On every recompute the rule is redone from the seed edge: edges created or moved by a model change come in by themselves."),
    ("Soltar a regra", "Drop the rule"),
    ("Deixa só as arestas da lista, sem refazer a regra.", "Keeps only the listed edges, without redoing the rule."),
    ("a partir da face", "from face"),
    ("A cada recálculo a regra é refeita a partir da face semente: faces que a mudança do modelo criar entram sozinhas.", "On every recompute the rule is redone from the seed face: faces created by a model change come in by themselves."),
    ("Deixa só as faces da lista, sem refazer a regra.", "Keeps only the listed faces, without redoing the rule."),
];

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_kernel::{EdgeFormula, Mesh};
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
        d.add_feature(FeatureKind::Fillet(EdgeOp { value: "5".into(), edges: EdgeSel::Edges(vec![[40., 40., 10.]]), mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
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
