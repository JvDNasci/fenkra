// SPDX-License-Identifier: LGPL-2.1-or-later
//! B469: dobra por linha. A minibarra da linha não segura os cliques no 3D (contexto egui sem
//! janela, cliques simulados) e a linha padrão nasce no meio da face clicada, não na ponta da peça.

use super::*;

/// Um quadro: a "vista 3D" (fundo clicável na tela toda) e a minibarra por cima. Devolve a ação da
/// minibarra, se o fundo recebeu clique e os textos desenhados com o retângulo de cada um.
fn quadro(ctx: &egui::Context, eventos: Vec<egui::Event>, posicao: u8) -> (AcaoMinibarra, bool, Vec<(String, Rect)>) {
    let tela = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000., 800.));
    let raw = egui::RawInput { screen_rect: Some(tela), events: eventos, ..Default::default() };
    let (mut acao, mut fundo) = (AcaoMinibarra::default(), false);
    let mut saida = ctx.run_ui(raw, |root| {
        fundo = root.interact(tela, egui::Id::new("vista_3d"), egui::Sense::click()).clicked();
        acao = minibarra(root.ctx(), Id(5), egui::pos2(300., 300.), Some(posicao));
    });
    saida.textures_delta.clear();
    let textos = saida
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::epaint::Shape::Text(t) => Some((t.galley.text().to_string(), Rect::from_min_size(t.pos, t.galley.size()))),
            _ => None,
        })
        .collect();
    (acao, fundo, textos)
}

/// Clique completo (mover, apertar, soltar) em `p`, mais um quadro parado. Junta o que os quadros
/// devolveram: alguma ação de posição, se o fundo foi clicado, e os textos do último quadro.
fn clica(ctx: &egui::Context, p: Pos2, posicao: u8) -> (Option<u8>, bool, Vec<(String, Rect)>) {
    let botao = |pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    let (mut pos, mut fundo, mut txt) = (None, false, vec![]);
    for ev in [vec![egui::Event::PointerMoved(p)], vec![botao(true)], vec![botao(false)], vec![]] {
        let (a, f, t) = quadro(ctx, ev, posicao);
        pos = pos.or(a.posicao);
        fundo |= f;
        txt = t;
    }
    (pos, fundo, txt)
}

fn tem(txt: &[(String, Rect)], s: &str) -> bool {
    txt.iter().any(|(t, _)| t == s)
}

#[test]
fn menu_de_posicao_fecha_ao_escolher_e_ao_clicar_fora_sem_segurar_a_vista() {
    let ctx = egui::Context::default();
    let (mut a, mut txt) = (AcaoMinibarra::default(), vec![]);
    for _ in 0..2 {
        let (x, _, t) = quadro(&ctx, vec![], 0);
        (a, txt) = (x, t);
    }
    // fechado: só a linha de botões, com a posição atual no botão do menu
    assert!(tem(&txt, "Linha de centro ▾"), "{txt:?}");
    assert!(!tem(&txt, "Início da dobra"), "as posições não ficam abertas sobre a vista: {txt:?}");
    let botao = a.botao_posicao.expect("botão das posições");
    // um clique na vista, longe da minibarra, chega ao 3D
    let (_, fundo, _) = clica(&ctx, egui::pos2(800., 650.), 0);
    assert!(fundo, "a minibarra fechada não segura o clique no 3D");
    // o botão abre o menu
    let (_, _, txt) = clica(&ctx, botao.center(), 0);
    assert!(tem(&txt, "Início da dobra") && tem(&txt, "Fim da dobra"), "menu aberto: {txt:?}");
    // clicar fora fecha o menu e o clique também chega ao 3D
    let (pos, fundo, txt) = clica(&ctx, egui::pos2(800., 650.), 0);
    assert_eq!(pos, None);
    assert!(fundo, "com o menu aberto, o clique fora vai para o 3D");
    assert!(!tem(&txt, "Início da dobra"), "o clique fora fecha o menu: {txt:?}");
    // escolher uma posição devolve a escolha e fecha o menu
    let (_, _, txt) = clica(&ctx, botao.center(), 0);
    let fim = txt.iter().find(|(t, _)| t == "Fim da dobra").map(|(_, r)| r.center()).expect("item Fim da dobra");
    let (pos, fundo, txt) = clica(&ctx, fim, 0);
    assert_eq!(pos, Some(2));
    assert!(!fundo, "o clique no item é do menu, não do 3D");
    assert!(!tem(&txt, "Início da dobra"), "o menu fecha ao escolher: {txt:?}");
}

/// A linha padrão no corpo de verdade: chapa 100 × 50 × 2, clique perto da ponta da face de cima. A
/// linha vai ao meio (x = 50), de borda a borda, e o lado do clique fica parado.
#[test]
fn linha_padrao_no_meio_da_face_da_chapa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: "2".into(), radius: "2".into(), k: "0.44".into(), flat: false, ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 9_469_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut &mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let r = fenkra_solidos::recompute(&d);
    assert_eq!(r.bodies.len(), 1, "{:?}", r.status);
    let (lo, hi) = (r.bodies[0].mesh.positions.chunks_exact(3).fold(f32::INFINITY, |m, p| m.min(p[2])), r.bodies[0].mesh.positions.chunks_exact(3).fold(f32::NEG_INFINITY, |m, p| m.max(p[2])));
    let topo = hi as f64;
    assert!((topo - lo as f64 - 2.).abs() < 1e-3, "chapa de 2 mm: {lo} a {hi}");
    let clique = [5., 5., topo];
    let (a, b) = linha_padrao(&r, 0, clique, [0., 0., 1.], 1.).expect("linha no meio");
    assert!((a[0] - 50.).abs() < 1e-3 && (b[0] - 50.).abs() < 1e-3, "{a:?} {b:?}");
    assert!((geo3d::dist(a, b) - 50.).abs() < 1e-3 && (a[2] - topo).abs() < 1e-6, "de borda a borda, na face: {a:?} {b:?}");
    let flip = chapa_linha::flip_para_fixo(a, b, [0., 0., 1.], clique).unwrap();
    assert!(!chapa_linha::no_lado_movel(clique, a, b, [0., 0., 1.], flip), "o lado do clique fica parado");
    // aplicada, a dobra gira só a metade de lá: a caixa sobe da espessura para perto de 50 mm
    let mut d2 = d.clone();
    let id = d2.add_feature(FeatureKind::SmBend(SmBend { a, b, angle: "90".into(), radius: None, flip, down: false, sketch: None, position: 0, cega: None }));
    let r2 = fenkra_solidos::recompute(&d2);
    assert!(matches!(r2.status[&id], fenkra_solidos::Status::Ok | fenkra_solidos::Status::Warning(_)), "{:?}", r2.status[&id]);
    let caixa = fenkra_solidos::final_shape(&d2).expect("corpo").bbox().expect("caixa");
    assert!(caixa.0[0] > -1e-3 && caixa.1[0] < 56., "a metade do clique (x < 50) ficou no lugar: {caixa:?}");
    assert!(caixa.1[2] - caixa.0[2] > 45., "a outra metade subiu: {caixa:?}");
    let v = fenkra_solidos::final_shape(&d2).unwrap().mass().volume;
    assert!((v - 100. * 50. * 2.).abs() < 0.03 * 10_000., "volume da chapa quase igual depois da dobra: {v}");
}
