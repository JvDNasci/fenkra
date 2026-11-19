// SPDX-License-Identifier: LGPL-2.1-or-later
//! B514: aceite dos campos do painel. A junção abre com a lista "Elementos" ativa, três superfícies
//! clicadas no 3D entram na lista dentro do campo (clicar de novo tira), Tab passa à lista seguinte
//! e o OK recalcula a junção numa superfície só. O laço de abas recebe as arestas pelo clique.

use crate::geo_ui::{GeoKind, Seed};
use eframe::egui;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.into()
}

/// Folha vertical pela reta de `a` a `b`, 30 mm para cima.
fn folha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> SurfRef {
    let c = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
    SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: s("30"), length2: s("0"), until: None, until2: None }))
}

fn textos(saida: &egui::FullOutput) -> Vec<String> {
    saida
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect()
}

fn tecla(key: egui::Key) -> egui::Event {
    egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }
}

/// Um quadro do painel do comando aberto, como o `editor::show` desenha: `inicio`, painel e `fim`.
/// Devolve os textos e se o painel usou o clique (o `App` então limpa a seleção).
fn quadro(ctx: &egui::Context, d: &Document, r: &mut RefElement, seed: &Seed, eventos: Vec<egui::Event>) -> (Vec<String>, bool) {
    // o egui lê os modificadores do quadro (não os do evento): Shift+Tab precisa avisar o Shift antes
    let shift = eventos.iter().any(|e| matches!(e, egui::Event::Key { modifiers, .. } if modifiers.shift));
    let mut eventos = eventos;
    eventos.insert(0, egui::Event::ModifiersChanged(if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE }));
    let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 900.))), events: eventos, ..Default::default() };
    let sel: Vec<String> = seed.surface.iter().map(|x| format!("{x:?}")).collect();
    let mut usou = false;
    let mut saida = ctx.run_ui(raw, |root| {
        let caixa = egui::Rect::from_min_size(egui::pos2(8., 8.), egui::vec2(crate::paineis::LARGURA_COLUNA - 16., 880.));
        let mut ui = root.new_child(egui::UiBuilder::new().max_rect(caixa));
        crate::campos::inicio(ui.ctx(), r.id.0, crate::campos::assinatura(&sel));
        crate::geo_ui::panel(&mut ui, d, r, &Default::default(), seed, None, None);
        usou = crate::campos::fim(ui.ctx());
    });
    saida.textures_delta.clear();
    (textos(&saida), usou)
}

fn clicar(ctx: &egui::Context, d: &Document, r: &mut RefElement, x: SurfRef) {
    let (_, usou) = quadro(ctx, d, r, &Seed { surface: Some(x), ..Default::default() }, vec![]);
    assert!(usou, "o campo ativo usa o clique");
    // o App limpa a seleção depois que o painel a usa
    quadro(ctx, d, r, &Seed::default(), vec![]);
}

fn itens(r: &RefElement) -> Vec<SurfRef> {
    match &r.kind {
        RefKind::SurfJoin { items, .. } => items.clone(),
        k => panic!("não é junção: {k:?}"),
    }
}

#[test]
fn juncao_tres_superficies_pelo_clique_tab_e_ok() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [0., 0., 0.]);
    let b = folha(&mut d, [0., 0., 0.], [0., 40., 0.]);
    let c = folha(&mut d, [0., 40., 0.], [-40., 40., 0.]);
    let id = d.add_geometry(GeoKind::OpJoin.make(&Seed::default()));
    let mut r = d.reference(id).unwrap().clone();
    let ctx = egui::Context::default();
    // ao abrir, a lista já é o campo ativo (sem botão de "+" nem de "usar a seleção")
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    let (t, _) = quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(t.iter().any(|x| x == "▶ Elementos (0)"), "{t:?}");
    assert!(crate::campos::painel_consome(&ctx), "com a lista ativa o painel usa o clique, não o preenchimento dos vazios");
    for x in [a, b, c] {
        clicar(&ctx, &d, &mut r, x);
    }
    assert_eq!(itens(&r), vec![a, b, c], "cada clique soma um elemento, o primeiro ocupa o item vazio");
    let (t, _) = quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(t.iter().any(|x| x == "▶ Elementos (3)"), "{t:?}");
    for x in [a, b, c] {
        let SurfRef::Ref(sid) = x else { unreachable!() };
        let nome = d.name_of(sid);
        assert!(t.iter().any(|y| *y == nome), "a lista dentro do campo mostra {nome}: {t:?}");
    }
    // clicar de novo num escolhido o tira; clicar outra vez devolve
    clicar(&ctx, &d, &mut r, b);
    assert_eq!(itens(&r), vec![a, c]);
    clicar(&ctx, &d, &mut r, b);
    assert_eq!(itens(&r), vec![a, c, b]);
    // Tab passa ao campo seguinte (a lista de curvas)
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![tecla(egui::Key::Tab)]);
    let (t, _) = quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(t.iter().any(|x| x == "▶ Curvas (0)"), "Tab cai no próximo campo: {t:?}");
    assert!(!t.iter().any(|x| x.starts_with("▶ Elementos")), "um só campo ativo: {t:?}");
    // mais um Tab: o parâmetro numérico (tolerância) recebe o foco do teclado
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![tecla(egui::Key::Tab)]);
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(ctx.text_edit_focused(), "o parâmetro recebe o foco do teclado");
    // Shift+Tab volta para as curvas e solta o teclado
    let volta = egui::Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::SHIFT };
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![volta]);
    let (t, _) = quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(t.iter().any(|x| x == "▶ Curvas (0)"), "Shift+Tab volta: {t:?}");
    assert!(!ctx.text_edit_focused());
    // OK: o documento recebe o que o painel editou e recalcula
    *d.references.iter_mut().find(|x| x.id == id).unwrap() = r;
    assert_eq!(crate::slots::count_unset(&d.reference(id).unwrap().kind), 0, "nenhum item vazio sobrou");
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let res = fenkra_solidos::recompute(&d);
    assert!(matches!(res.status[&id], fenkra_solidos::Status::Ok), "{:?}", res.status[&id]);
    let sh = fenkra_kernel::Shape::from_brep(&fenkra_solidos::surface_brep(&d, id).expect("superfície")).unwrap();
    let area = sh.mass().area;
    assert!((area - 3. * 40. * 30.).abs() < 1e-3, "área {area}");
    let (lo, hi) = sh.bbox().expect("caixa");
    for (v, e) in lo.iter().zip([-40., 0., 0.]).chain(hi.iter().zip([0., 40., 30.])) {
        assert!((v - e).abs() < 1e-3, "caixa {lo:?} {hi:?}");
    }
}

/// Junção que já tem tudo e nada vazio: o Enter só passa ao campo seguinte enquanto há um campo de
/// seleção mais adiante; na última lista ele volta a concluir o comando.
#[test]
fn enter_avanca_ate_a_ultima_lista_e_depois_conclui() {
    let mut d = Document::new("t");
    let a = folha(&mut d, [-40., 0., 0.], [0., 0., 0.]);
    let id = d.add_geometry(RefKind::SurfJoin { items: vec![a], tol: s("0.01"), curves: vec![] });
    let mut r = d.reference(id).unwrap().clone();
    let ctx = egui::Context::default();
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(crate::campos::enter_avanca(&ctx), "Elementos ativo e Curvas depois: Enter avança");
    quadro(&ctx, &d, &mut r, &Seed::default(), vec![tecla(egui::Key::Enter)]);
    let (t, _) = quadro(&ctx, &d, &mut r, &Seed::default(), vec![]);
    assert!(t.iter().any(|x| x == "▶ Curvas (0)"), "{t:?}");
    assert!(!crate::campos::enter_avanca(&ctx), "na última lista o Enter conclui o comando");
}

fn placa() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = fenkra_core::sketch::SketchData::default();
    let mut n = 4_000_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

/// Laço de abas aberto sem aresta (como o comando nasce sem nada selecionado): espera a aresta, cada
/// clique numa aresta soma uma, clicar de novo tira, e o recálculo dá as abas.
#[test]
fn laco_de_abas_recebe_as_arestas_pelo_clique() {
    let mut d = placa();
    let mut k = FeatureKind::SmFlangeLoop(SmFlangeLoop {
        edges: vec![[0.; 3]],
        edge_keys: vec![],
        loop_all: true,
        extent: ExtensaoAba::Inteira,
        invert: false,
        length: s("20"),
        angle: s("90"),
        radius: None,
        flip: false,
        side: 0,
        miter: true,
        relief: None,
        relief_kind: 0,
    });
    assert!(crate::slots::feature_incomplete(&k));
    assert_eq!(crate::entradas::campo_ativo(&k), Some(crate::entradas::Espera::Aresta));
    assert!(crate::slots::recebe_aresta_clicada(&k));
    let (e1, e2) = ([100., 25., 2.], [50., 50., 2.]);
    let sem_cadeia = |_: bool| -> Vec<[f64; 3]> { panic!("o laço não usa a cadeia tangente") };
    assert!(crate::slots::aresta_clicada(&mut k, e1, None, true, sem_cadeia));
    assert!(!crate::slots::feature_incomplete(&k), "a primeira aresta ocupa o lugar vazio");
    assert!(crate::slots::aresta_clicada(&mut k, e2, None, true, sem_cadeia));
    let FeatureKind::SmFlangeLoop(l) = &k else { unreachable!() };
    assert_eq!(l.edges, vec![e1, e2], "o segundo clique soma (sem botão)");
    assert!(!l.loop_all, "várias arestas valem como estão, como na criação");
    assert!(crate::slots::aresta_clicada(&mut k, e2, None, true, sem_cadeia));
    let FeatureKind::SmFlangeLoop(l) = &k else { unreachable!() };
    assert_eq!(l.edges, vec![e1], "clicar de novo tira");
    assert!(l.loop_all, "uma aresta só volta a valer pelo laço da face");
    assert!(crate::slots::aresta_clicada(&mut k, e2, None, true, sem_cadeia));
    let id = d.add_feature(k);
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let r = fenkra_solidos::recompute(&d);
    assert!(matches!(r.status[&id], fenkra_solidos::Status::Ok | fenkra_solidos::Status::Warning(_)), "{:?}", r.status[&id]);
    let b = &r.bodies[0];
    let placa = 100. * 50. * 2.;
    assert!(b.mass.volume > placa + 2000., "duas abas de 20 mm somam volume: {}", b.mass.volume);
    let zmax = b.mesh.positions.chunks_exact(3).map(|p| p[2]).fold(f32::MIN, f32::max);
    assert!(zmax > 15., "as abas sobem: z máx {zmax}");
}

fn laco_vazio() -> FeatureKind {
    FeatureKind::SmFlangeLoop(SmFlangeLoop { edges: vec![[0.; 3]], edge_keys: vec![], loop_all: true, extent: ExtensaoAba::Inteira, invert: false, length: s("20"), angle: s("90"), radius: None, flip: false, side: 0, miter: true, relief: None, relief_kind: 0 })
}

/// B530: duas arestas escolhidas no 3D (uma chave por aresta, pelo nome) viram duas abas; Shift não
/// é preciso, e clicar de novo numa delas a tira.
#[test]
fn laco_de_abas_duas_arestas_escolhidas_viram_duas_abas() {
    let mut d = placa();
    let mut k = laco_vazio();
    let sem_cadeia = |_: bool| -> Vec<[f64; 3]> { panic!("o laço não usa a cadeia tangente") };
    let (e1, e2) = ([100., 25., 2.], [50., 0., 2.]);
    assert!(crate::slots::aresta_clicada(&mut k, e1, None, true, sem_cadeia));
    assert!(crate::slots::aresta_clicada(&mut k, e2, None, true, sem_cadeia));
    let FeatureKind::SmFlangeLoop(l) = &k else { unreachable!() };
    assert_eq!(l.edges.len(), 2);
    assert!(!l.loop_all, "duas arestas valem como estão");
    let id = d.add_feature(k);
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let r = fenkra_solidos::recompute(&d);
    assert!(matches!(r.status[&id], fenkra_solidos::Status::Ok | fenkra_solidos::Status::Warning(_)), "{:?}", r.status[&id]);
    // duas abas de 20 mm: 100 mm + 50 mm de comprimento, cerca de 2 mm de espessura
    let acresc = r.bodies[0].mass.volume - 100. * 50. * 2.;
    assert!(acresc > 150. * 2. * 15. && acresc < 150. * 2. * 28., "volume das duas abas {acresc}");
}

/// B530: clicar numa FACE com o laço aberto pega o contorno inteiro (as quatro arestas da placa).
#[test]
fn laco_de_abas_face_clicada_pega_o_contorno() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    let r0 = fenkra_solidos::recompute(&d);
    let b = &r0.bodies[0];
    // a face de cima (normal +Z, de maior área)
    let face = (0..b.mesh.face_info.len()).filter(|f| b.mesh.face_info[*f].dir[2] > 0.9).max_by(|a, c| b.mesh.face_info[*a].area.total_cmp(&b.mesh.face_info[*c].area)).unwrap();
    let e = crate::slots::aresta_externa_da_face(&b.mesh, face).expect("a face tem arestas");
    let mid = [b.mesh.edge_mids[e * 3] as f64, b.mesh.edge_mids[e * 3 + 1] as f64, b.mesh.edge_mids[e * 3 + 2] as f64];
    let mut k = laco_vazio();
    // havia duas arestas escolhidas: a face substitui tudo
    if let FeatureKind::SmFlangeLoop(l) = &mut k {
        l.edges = vec![[100., 25., 2.], [50., 0., 2.]];
        l.loop_all = false;
    }
    assert!(crate::slots::face_clicada_no_laco(&mut k, mid, b.edge_key(e)));
    let FeatureKind::SmFlangeLoop(l) = &k else { unreachable!() };
    assert!(l.loop_all && l.edges == vec![mid], "a face vale pelo contorno inteiro");
    let id = d.add_feature(k);
    let r = fenkra_solidos::recompute(&d);
    assert!(matches!(r.status[&id], fenkra_solidos::Status::Ok | fenkra_solidos::Status::Warning(_)), "{:?}", r.status[&id]);
    // quatro abas: perímetro 300 mm
    let acresc = r.bodies[0].mass.volume - 100. * 50. * 2.;
    assert!(acresc > 300. * 2. * 15., "o contorno inteiro ganhou aba: {acresc}");
    // não é laço: face de um comando que não é laço não muda nada
    let mut outro = FeatureKind::SmFlange(fenkra_core::SmFlange { edge: [0.; 3], length: s("5"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None });
    assert!(!crate::slots::face_clicada_no_laco(&mut outro, mid, None));
}

/// Aba curva aberta: o clique numa aresta põe a cadeia tangente e a chave da aresta.
#[test]
fn aba_curva_recebe_a_cadeia_pelo_clique() {
    let mut k = FeatureKind::SmCurveFlange(SmCurveFlange { edge: vec![], length: s("15"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None, so_aresta: false });
    assert!(crate::slots::feature_incomplete(&k));
    assert!(crate::slots::aresta_clicada(&mut k, [5., 0., 2.], None, true, |_| vec![[0., 0., 2.], [5., 0., 2.], [10., 0., 2.]]));
    let FeatureKind::SmCurveFlange(c) = &k else { unreachable!() };
    assert_eq!(c.edge.len(), 3);
    assert!(!crate::slots::feature_incomplete(&k));
}

/// Teste da alpha.38: com a aba de aresta aberta, clicar numa aresta curva troca pela aba curva só
/// naquela aresta, com os mesmos parâmetros; numa aresta reta continua a aba de aresta.
#[test]
fn aba_de_aresta_vira_curva_no_clique_em_curva() {
    let s = |x: &str| x.to_string();
    let mut k = FeatureKind::SmFlange(fenkra_core::SmFlange { edge: [0.; 3], length: s("25"), angle: s("60"), radius: None, off1: s("0"), off2: s("0"), flip: true, relief: None, relief_kind: 0, side: 1, miter: false, min_gap: true, edge_key: None });
    assert!(crate::slots::aresta_clicada(&mut k, [5., 0., 2.], None, true, |_| panic!("aresta reta não usa a cadeia")));
    assert!(matches!(k, FeatureKind::SmFlange(_)));
    let arco = vec![[10., 0., 2.], [7.07, 7.07, 2.], [0., 10., 2.]];
    assert!(!crate::slots::polilinha_reta(&arco));
    assert!(crate::slots::aresta_clicada(&mut k, arco[1], None, false, |so| {
        assert!(so, "só a aresta clicada");
        arco.clone()
    }));
    let FeatureKind::SmCurveFlange(c) = &k else { panic!("virou aba curva") };
    assert_eq!((c.length.as_str(), c.angle.as_str(), c.flip, c.side, c.so_aresta, c.edge.len()), ("25", "60", true, 1, true, 3));
}
