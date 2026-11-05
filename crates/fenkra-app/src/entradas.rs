// SPDX-License-Identifier: LGPL-2.1-or-later
//! B247: o que o comando aberto espera e o que já recebeu. O painel destaca o campo ativo (perfil,
//! eixo, face, aresta, plano, corpo), o clique no 3D preenche esse campo e o destaque passa ao
//! próximo. No 3D, arestas, faces, plano neutro e corpo ferramenta aparecem inteiros, cada um na cor
//! do seu papel, nunca por bolinha. As cores vêm só do tema.

use crate::previa::Fantasma;
use crate::render::{LineVertex, Scene};
use crate::slots::{UNSET_AXIS, UNSET_CURVE, UNSET_PLANE, UNSET_POINT, UNSET_SKETCH, UNSET_SURF};
use eframe::egui::{self, Color32, RichText};
use fenkra_core::*;
use fenkra_kernel::Mesh;
use fenkra_solidos::RecomputeResult;
use serde_json::Value;

/// O tipo de coisa que o campo ativo espera.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Espera {
    Perfil,
    Eixo,
    Face,
    Aresta,
    Corpo,
    Plano,
    Ponto,
    Curva,
    Superficie,
}

impl Espera {
    pub fn rotulo(self) -> &'static str {
        match self {
            Espera::Perfil => "Perfil",
            Espera::Eixo => "Eixo",
            Espera::Face => "Face",
            Espera::Aresta => "Aresta",
            Espera::Corpo => "Corpo ferramenta",
            Espera::Plano => "Plano",
            Espera::Ponto => "Ponto",
            Espera::Curva => "Curva",
            Espera::Superficie => "Superfície",
        }
    }

    /// Frase do que clicar, para quem olha o painel.
    pub fn dica(self) -> &'static str {
        match self {
            Espera::Perfil => "clique num sketch ou curva fechada no 3D ou na árvore",
            Espera::Eixo => "clique numa aresta reta, eixo, face plana ou cilíndrica",
            Espera::Face => "clique numa face da peça",
            Espera::Aresta => "clique numa aresta da peça",
            Espera::Corpo => "clique no corpo que serve de ferramenta",
            Espera::Plano => "clique num plano ou face plana",
            Espera::Ponto => "clique num vértice, ponto ou centro de círculo",
            Espera::Curva => "clique numa aresta ou curva",
            Espera::Superficie => "clique numa face ou superfície",
        }
    }

    /// Cor do papel no tema: a mesma no painel e no 3D.
    pub fn cor(self) -> Color32 {
        match self {
            Espera::Corpo => crate::theme::papel_ferramenta(),
            Espera::Plano => crate::theme::papel_neutro(),
            Espera::Face | Espera::Aresta => crate::theme::st_warn(),
            _ => crate::theme::st_ok(),
        }
    }

    /// Ordem em que os campos de um comando são pedidos.
    fn prioridade(self) -> u8 {
        match self {
            Espera::Perfil => 0,
            Espera::Face => 1,
            Espera::Aresta => 2,
            Espera::Corpo => 3,
            Espera::Eixo => 4,
            Espera::Plano => 5,
            Espera::Curva => 6,
            Espera::Superficie => 7,
            Espera::Ponto => 8,
        }
    }
}

fn espera_do_sentinela(u: u64) -> Option<Espera> {
    Some(match u {
        x if x == UNSET_SKETCH.0 => Espera::Perfil,
        x if x == UNSET_AXIS.0 => Espera::Eixo,
        x if x == UNSET_PLANE.0 => Espera::Plano,
        x if x == UNSET_POINT.0 => Espera::Ponto,
        x if x == UNSET_CURVE.0 => Espera::Curva,
        x if x == UNSET_SURF.0 => Espera::Superficie,
        _ => return None,
    })
}

fn sentinelas(v: &Value, out: &mut Vec<Espera>) {
    match v {
        Value::Number(n) => out.extend(n.as_u64().and_then(espera_do_sentinela)),
        Value::Object(m) => m.values().for_each(|x| sentinelas(x, out)),
        Value::Array(a) => a.iter().for_each(|x| sentinelas(x, out)),
        _ => {}
    }
}

/// O campo que o próximo clique no 3D preenche, ou `None` quando o comando tem tudo o que precisa.
/// Quando falta mais de um, vale a ordem do comando (perfil, faces, arestas, eixo, plano…).
pub fn campo_ativo(k: &FeatureKind) -> Option<Espera> {
    let z = [0.; 3];
    let proprio = match k {
        FeatureKind::Shell(x) if x.faces.is_empty() => Some(Espera::Face),
        FeatureKind::Draft(x) if x.faces.is_empty() => Some(Espera::Face),
        FeatureKind::Thickness(x) if x.faces.is_empty() => Some(Espera::Face),
        FeatureKind::Thread(t) if t.face == z => Some(Espera::Face),
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => match (&o.mode, &o.edges) {
            (EdgeMode::FaceFace(a, b), _) if *a == z || *b == z => Some(Espera::Face),
            (EdgeMode::FullRound { mid, a, b }, _) if [mid, a, b].iter().any(|p| **p == z) => Some(Espera::Face),
            (EdgeMode::FaceFace(..) | EdgeMode::FullRound { .. }, _) => None,
            (_, EdgeSel::Edges(m)) if m.is_empty() => Some(Espera::Aresta),
            _ => None,
        },
        _ => None,
    };
    if proprio.is_some() {
        return proprio;
    }
    let mut falta = vec![];
    if let Ok(v) = serde_json::to_value(k) {
        sentinelas(&v, &mut falta);
    }
    falta.into_iter().min_by_key(|e| e.prioridade())
}

/// Quantos itens do campo de lista o comando já tem (faces da casca, arestas do filete…).
pub fn contagem(k: &FeatureKind) -> Option<(Espera, usize)> {
    let z = [0.; 3];
    match k {
        FeatureKind::Shell(x) => Some((Espera::Face, x.faces.len())),
        FeatureKind::Draft(x) => Some((Espera::Face, x.faces.len())),
        FeatureKind::Thickness(x) => Some((Espera::Face, x.faces.len())),
        FeatureKind::Thread(t) => Some((Espera::Face, usize::from(t.face != z))),
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => match &o.mode {
            EdgeMode::FaceFace(a, b) => Some((Espera::Face, [a, b].iter().filter(|p| ***p != z).count())),
            EdgeMode::FullRound { mid, a, b } => Some((Espera::Face, [mid, a, b].iter().filter(|p| ***p != z).count())),
            _ => match &o.edges {
                EdgeSel::Edges(m) => Some((Espera::Aresta, m.len())),
                _ => None,
            },
        },
        _ => None,
    }
}

/// Booleana: clicar num corpo troca o corpo ferramenta. Não aceita o corpo onde o resultado fica.
pub fn corpo_clicado(k: &mut FeatureKind, alvo: Id, collector: Id) -> bool {
    match k {
        FeatureKind::Combine(c) if collector != alvo && c.tool != collector => {
            c.tool = collector;
            true
        }
        _ => false,
    }
}

/// A saída espera o plano neutro: depois das faces, o próximo clique numa face plana o escolhe.
pub fn espera_plano_neutro(k: &FeatureKind) -> bool {
    matches!(k, FeatureKind::Draft(d) if !d.faces.is_empty() && matches!(d.neutral, PlaneRef::Datum(p) if crate::slots::is_unset(p)))
}

fn chave_plano(id: Id) -> egui::Id {
    egui::Id::new(("entradas_plano_neutro", id.0))
}

/// O clique no 3D deve escolher o plano neutro (e não mais uma face da lista)?
pub fn clique_vai_para_plano(ctx: &egui::Context, id: Id, k: &FeatureKind) -> bool {
    espera_plano_neutro(k) && ctx.data(|d| d.get_temp::<bool>(chave_plano(id))).unwrap_or(false)
}

/// Legenda de cores do que o comando mostra no 3D: (cor, texto).
pub fn legenda(k: &FeatureKind, destaque: Color32) -> Vec<(Color32, &'static str)> {
    let aberta = (crate::theme::st_warn(), "Face aberta");
    match k {
        FeatureKind::Shell(_) => vec![aberta, (destaque, "Parede nova")],
        FeatureKind::Draft(_) => vec![(crate::theme::st_warn(), "Face"), (crate::theme::papel_neutro(), "Plano neutro")],
        FeatureKind::Thickness(_) | FeatureKind::Thread(_) => vec![(crate::theme::st_warn(), "Face")],
        FeatureKind::Fillet(_) | FeatureKind::Chamfer(_) => vec![(crate::theme::st_warn(), "Aresta ou face")],
        FeatureKind::Combine(_) => vec![(crate::theme::papel_ferramenta(), "Corpo ferramenta")],
        FeatureKind::SplitBody(_) | FeatureKind::SewSurface(_) => vec![(crate::theme::papel_ferramenta(), "Ferramenta")],
        FeatureKind::ThickSurface(_) | FeatureKind::CloseSurface(_) => vec![(crate::theme::st_warn(), "Superfície")],
        _ => vec![],
    }
}

/// Faixa no topo do painel do comando aberto: campo ativo destacado, contador e legenda de cores.
pub fn faixa(ui: &mut egui::Ui, id: Id, k: &FeatureKind) {
    let ativo = campo_ativo(k);
    let acc = crate::theme::accent(ui.visuals());
    let cor = ativo.map(|e| e.cor()).unwrap_or_else(crate::theme::st_ok);
    egui::Frame::NONE.fill(cor.gamma_multiply(0.14)).stroke(egui::Stroke::new(1.5, cor)).corner_radius(6.).inner_margin(8.).show(ui, |ui| {
        ui.set_width(ui.available_width());
        match ativo {
            Some(e) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("⊙ {}: {}", crate::i18n::tr("Campo ativo"), crate::i18n::tr(e.rotulo()))).strong().color(cor));
                });
                ui.label(RichText::new(crate::i18n::tr(e.dica())).small());
            }
            None => {
                ui.label(RichText::new(format!("✔ {}", crate::i18n::tr("Entradas completas"))).strong().color(cor));
                if matches!(k, FeatureKind::Combine(_)) {
                    ui.label(RichText::new(crate::i18n::tr(Espera::Corpo.dica())).small());
                }
            }
        }
        if let Some((e, n)) = contagem(k) {
            ui.label(RichText::new(format!("{}: {n}", crate::i18n::tr(if e == Espera::Aresta { "Arestas escolhidas" } else { "Faces escolhidas" }))).small());
        }
        let leg = legenda(k, acc);
        if !leg.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (c, t) in leg {
                    ui.label(RichText::new("■").color(c));
                    ui.label(RichText::new(crate::i18n::tr(t)).small());
                }
            });
        }
        // a saída pede faces e depois o plano neutro: o botão muda o destino do clique
        if espera_plano_neutro(k) {
            let ligado = ui.ctx().data(|d| d.get_temp::<bool>(chave_plano(id))).unwrap_or(false);
            let txt = if ligado { "Clique numa face plana ou plano para ser o neutro" } else { "Escolher o plano neutro no 3D" };
            if ui.add(egui::Button::new(RichText::new(crate::i18n::tr(txt)).color(acc)).selected(ligado)).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(chave_plano(id), !ligado));
            }
        } else {
            ui.ctx().data_mut(|d| d.remove_temp::<bool>(chave_plano(id)));
        }
    });
    ui.add_space(4.);
}

/// Aresta da malha mais perto do ponto (a assinatura do comando é o ponto médio dela) e a distância.
pub fn aresta_perto(m: &Mesh, p: [f64; 3]) -> Option<(usize, f32)> {
    let alvo = glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let nseg = m.edges.len() / 6;
    let mut melhor: Option<(usize, f32)> = None;
    for e in 0..m.edge_ranges.len() {
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
        let d = (s0..s1)
            .map(|s| {
                let q = &m.edges[s * 6..s * 6 + 6];
                let (a, b) = (glam::Vec3::new(q[0], q[1], q[2]), glam::Vec3::new(q[3], q[4], q[5]));
                let ab = b - a;
                let t = if ab.length_squared() > 1e-12 { ((alvo - a).dot(ab) / ab.length_squared()).clamp(0., 1.) } else { 0. };
                (a + ab * t - alvo).length()
            })
            .fold(f32::MAX, f32::min);
        if melhor.is_none_or(|(_, x)| d < x) {
            melhor = Some((e, d));
        }
    }
    melhor
}

/// Distância máxima (mm) entre o ponto guardado e a aresta para contar como a mesma.
const SOBRE_A_ARESTA: f32 = 0.5;

fn pontos_da_aresta(m: &Mesh, e: usize) -> usize {
    let nseg = m.edges.len() / 6;
    let s0 = m.edge_ranges[e] as usize;
    let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
    (s1 - s0) * 2
}

/// Aresta inteira, do começo ao fim, na camada que fica sempre visível.
fn aresta_inteira(dst: &mut Vec<LineVertex>, m: &Mesh, e: usize, col: [f32; 4]) {
    let nseg = m.edges.len() / 6;
    let s0 = m.edge_ranges[e] as usize;
    let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
    for q in m.edges[s0 * 6..s1 * 6].chunks_exact(3) {
        dst.push(LineVertex { pos: [q[0], q[1], q[2]], col });
    }
}

/// O que o comando aberto mostra inteiro no 3D, além das faces que a prévia já realça: as arestas
/// do filete e do chanfro e o corpo ferramenta da booleana (o plano neutro é de `previa::plano_referenciado`).
pub fn destacar(doc: &Document, r: &RecomputeResult, f: &Fantasma, s: &mut Scene) {
    let Some(feat) = doc.feature(f.id) else { return };
    match &feat.kind {
        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
            let EdgeSel::Edges(lista) = &o.edges else { return };
            let Some(m) = crate::previa::malha_de_entrada(r, f, feat.target) else { return };
            let col = crate::previa::cor_linha(f.entrada);
            for p in lista.iter().filter(|p| **p != [0.; 3]) {
                if let Some((e, _)) = aresta_perto(m, *p).filter(|x| x.1 < SOBRE_A_ARESTA) {
                    aresta_inteira(&mut s.overlay, m, e, col);
                    // o traço grosso acompanha a aresta ponto a ponto
                    let pts: Vec<[f32; 3]> = s.overlay[s.overlay.len() - pontos_da_aresta(m, e)..].iter().map(|v| v.pos).collect();
                    crate::render::push_wide(&mut s.wide, &pts, col, 3.5, 0);
                }
            }
        }
        FeatureKind::Combine(c) => {
            // o corpo ferramenta de antes: depois da operação ele some do resultado
            let m = f.antes.iter().find(|b| b.collector == c.tool).map(|b| &b.mesh).or_else(|| r.bodies.iter().find(|b| b.collector == c.tool).map(|b| &b.mesh));
            if let Some(m) = m {
                for fi in 0..m.face_ranges.len() {
                    crate::previa::face_fantasma(&mut s.ghost, m, fi, f.ferramenta);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo_ui::Seed;
    use crate::slots::{fill, Cands};
    use fenkra_solidos::recompute;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn casca() -> FeatureKind {
        FeatureKind::Shell(Shell { faces: vec![], thickness: s("2"), outward: false, face_keys: vec![], closed: false })
    }

    /// Casca, espessura e rosca esperam a face; depois do clique o campo some e o contador sobe.
    #[test]
    fn casca_espessura_e_rosca_pedem_a_face_e_avancam() {
        let mut k = casca();
        assert_eq!(campo_ativo(&k), Some(Espera::Face));
        assert_eq!(contagem(&k), Some((Espera::Face, 0)));
        crate::feat_ui::add_face(&mut k, [0., 0., 20.]);
        assert_eq!(campo_ativo(&k), None);
        assert_eq!(contagem(&k), Some((Espera::Face, 1)));

        let mut k = FeatureKind::Thickness(Thickness { faces: vec![], value: s("2"), face_keys: vec![], remove: false });
        assert_eq!(campo_ativo(&k), Some(Espera::Face));
        crate::feat_ui::add_face(&mut k, [0., 0., 20.]);
        assert_eq!(campo_ativo(&k), None);

        let mut k = FeatureKind::Thread(Thread { face: [0.; 3], spec: s("M10x1.5"), length: s("20"), modeled: false, end: None, start: s("0") });
        assert_eq!(campo_ativo(&k), Some(Espera::Face));
        crate::feat_ui::add_face(&mut k, [6., 0., 10.]);
        assert_eq!(campo_ativo(&k), None);
    }

    /// Filete entre faces: o primeiro clique preenche uma face e o campo continua ativo até a segunda.
    #[test]
    fn filete_face_face_pede_duas_faces() {
        let op = EdgeOp { value: s("2"), edges: EdgeSel::AllOfBody, mode: EdgeMode::FaceFace([0.; 3], [0.; 3]), prop: Default::default(), keys: vec![] };
        let mut k = FeatureKind::Fillet(op);
        assert_eq!(campo_ativo(&k), Some(Espera::Face));
        crate::feat_ui::add_face(&mut k, [0., 0., 20.]);
        assert_eq!(campo_ativo(&k), Some(Espera::Face), "falta a segunda face");
        assert_eq!(contagem(&k), Some((Espera::Face, 1)));
        crate::feat_ui::add_face(&mut k, [20., 0., 10.]);
        assert_eq!(campo_ativo(&k), None);
        assert_eq!(contagem(&k), Some((Espera::Face, 2)));
        // arestas: vazio pede aresta; uma aresta basta
        let op = EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] };
        let mut k = FeatureKind::Chamfer(op);
        assert_eq!(campo_ativo(&k), Some(Espera::Aresta));
        if let FeatureKind::Chamfer(o) = &mut k {
            o.edges = EdgeSel::Edges(vec![[0., 0., 10.]]);
        }
        assert_eq!(campo_ativo(&k), None);
    }

    /// Saída: faces primeiro, depois o plano neutro, que o clique em face plana preenche pelos campos vazios.
    #[test]
    fn saida_pede_faces_e_depois_o_plano_neutro() {
        let mut k = FeatureKind::Draft(Draft { faces: vec![], neutral: crate::slots::plane(), angle: s("3"), reverse: false, face_keys: vec![] });
        assert_eq!(campo_ativo(&k), Some(Espera::Face));
        assert!(!espera_plano_neutro(&k), "sem faces o clique vai para a lista de faces");
        crate::feat_ui::add_face(&mut k, [20., 0., 10.]);
        assert_eq!(campo_ativo(&k), Some(Espera::Plano));
        assert!(espera_plano_neutro(&k));
        let seed = Seed { planes: vec![PlaneRef::Origin(OriginPlane::XY)], ..Default::default() };
        assert!(fill(&mut k, &mut Cands::from_seed(&seed, None)), "o clique preenche o plano");
        assert_eq!(campo_ativo(&k), None);
        assert!(!espera_plano_neutro(&k));
    }

    /// Revolução: perfil, depois eixo; cada seleção preenche um campo e o ativo avança.
    #[test]
    fn revolucao_pede_perfil_e_depois_eixo() {
        let mut k = FeatureKind::Revolve(Revolve { sketch: UNSET_SKETCH, axis: crate::slots::axis(), angle: s("360"), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None });
        assert_eq!(campo_ativo(&k), Some(Espera::Perfil));
        assert!(fill(&mut k, &mut Cands::from_seed(&Seed::default(), Some(Id(42)))));
        assert_eq!(campo_ativo(&k), Some(Espera::Eixo));
        let mut c = Cands::default();
        c.alt_axes.push_back(AxisRef::Origin(OriginAxis::Z));
        assert!(fill(&mut k, &mut c));
        assert_eq!(campo_ativo(&k), None);
    }

    /// Booleana: o clique num corpo troca a ferramenta, sem combo, e nunca o corpo do resultado.
    #[test]
    fn booleana_troca_a_ferramenta_pelo_clique() {
        let mut k = FeatureKind::Combine(Combine { tool: Id(5), op: BoolOp::Cut, keep_tool: false });
        assert!(!corpo_clicado(&mut k, Id(1), Id(1)), "o corpo do resultado não é ferramenta");
        assert!(!corpo_clicado(&mut k, Id(1), Id(5)), "já é a ferramenta");
        assert!(corpo_clicado(&mut k, Id(1), Id(7)));
        assert!(matches!(&k, FeatureKind::Combine(c) if c.tool == Id(7)));
        assert_eq!(campo_ativo(&k), None);
    }

    fn sk_quadrado(d: &mut Document) -> Id {
        let mut data = SketchData::default();
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        data.add_rect(&mut al, [-20., -20.], [20., 20.], false);
        d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
    }

    fn caixa() -> Document {
        let mut d = Document::new("caixa");
        let sk = sk_quadrado(&mut d);
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
        d
    }

    fn fantasma(id: Id, antes: &[crate::previa::CorpoBase]) -> Fantasma<'_> {
        Fantasma { id, cor: [10, 20, 30], entrada: [40, 50, 60], erro: [70, 80, 90], retirada: [100, 110, 120], ferramenta: [130, 140, 150], neutro: [160, 170, 180], antes }
    }

    fn tem(s: &Scene, c: [u8; 3]) -> bool {
        s.ghost.iter().any(|v| (0..3).all(|i| (v.col[i] - c[i] as f32 / 255.).abs() < 1e-6))
    }

    /// A aresta escolhida (guardada pelo ponto médio) aparece inteira, do começo ao fim, na cor do papel.
    #[test]
    fn aresta_escolhida_aparece_inteira() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = caixa();
        let antes = recompute(&d);
        let base = crate::previa::corpos_base(Some(&antes));
        let m = &antes.bodies[0].mesh;
        // aresta da tampa em y = 20: de (-20,20,20) a (20,20,20), ponto médio (0,20,20)
        let (e, dist) = aresta_perto(m, [0., 20., 20.]).expect("aresta");
        assert!(dist < 1e-3, "{dist}");
        let nseg = m.edges.len() / 6;
        let seg = |e: usize| m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg) - m.edge_ranges[e] as usize;
        let id = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![[0., 20., 20.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
        let r = recompute(&d);
        let mut cena = Scene::default();
        destacar(&d, &r, &fantasma(id, &base), &mut cena);
        assert_eq!(cena.overlay.len(), seg(e) * 2, "a aresta inteira, sem bolinha");
        assert!(cena.overlay.iter().all(|v| (v.col[0] - 40. / 255.).abs() < 1e-6));
        let xs: Vec<f32> = cena.overlay.iter().map(|v| v.pos[0]).collect();
        assert!(xs.iter().cloned().fold(f32::MAX, f32::min) < -19.9 && xs.iter().cloned().fold(f32::MIN, f32::max) > 19.9, "vai de ponta a ponta");
        assert!(!cena.wide.is_empty());
        // ponto longe de qualquer aresta não realça nada
        let id2 = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![[0., 0., 10.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
        let mut vazia = Scene::default();
        destacar(&d, &r, &fantasma(id2, &base), &mut vazia);
        assert!(vazia.overlay.is_empty());
    }

    /// Booleana: o corpo ferramenta de antes aparece inteiro na cor de ferramenta.
    #[test]
    fn corpo_ferramenta_aparece_inteiro_na_cor_do_papel() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = caixa();
        let antes = recompute(&d);
        let base = crate::previa::corpos_base(Some(&antes));
        let tool = antes.bodies[0].collector;
        let id = d.add_feature(FeatureKind::Combine(Combine { tool, op: BoolOp::Cut, keep_tool: false }));
        let mut cena = Scene::default();
        destacar(&d, &antes, &fantasma(id, &base), &mut cena);
        assert!(tem(&cena, [130, 140, 150]));
        assert_eq!(cena.ghost.len(), antes.bodies[0].mesh.indices.len(), "todas as faces do corpo");
    }
}
