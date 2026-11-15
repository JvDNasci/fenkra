// SPDX-License-Identifier: LGPL-2.1-or-later
//! B466: varredura cônica por 3 guias. Ordem de preenchimento, campo ativo no painel (contexto egui
//! sem janela), largura da coluna e lista ▾ com as arestas e curvas do modelo.

use super::*;
use crate::geo_ui::Seed;
use crate::slots::{curve, fill, Cands};
use eframe::egui;

fn s(x: &str) -> String {
    x.to_string()
}

/// Bloco 40 × 30 × 20 e uma varredura cônica por 3 guias recém-aberta (campos a escolher).
fn doc_varredura() -> (Document, Id) {
    let mut d = Document::new("varredura");
    let mut data = SketchData::default();
    let mut n = 9_466_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut &mut al, [-20., -15.], [20., 15.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    let kind = SweepKind::ConicGuides { guide1: curve(), guide2: curve(), apex: curve(), rho: s("0.5"), radius: String::new() };
    let id = d.add_geometry(RefKind::SurfSweepX { kind, spine: None, solution: 0, smooth: false, sections: s("24") });
    (d, id)
}

fn guias(d: &Document, id: Id) -> [CurveRef; 3] {
    match &d.reference(id).unwrap().kind {
        RefKind::SurfSweepX { kind: SweepKind::ConicGuides { guide1, guide2, apex, .. }, .. } => [guide1.clone(), guide2.clone(), apex.clone()],
        k => panic!("{k:?}"),
    }
}

fn aresta(i: f64) -> CurveRef {
    CurveRef::Edge(vec![[i, 0., 0.], [i, 10., 0.]])
}

fn clica(d: &mut Document, id: Id, c: CurveRef) -> bool {
    let r = d.references.iter_mut().find(|r| r.id == id).unwrap();
    let seed = Seed { curves: vec![c], ..Default::default() };
    fill(&mut r.kind, &mut Cands::from_seed(&seed, None))
}

/// Clique a clique: Guia 1, depois Guia 2, depois a do ápice. Antes o `Value` do serde percorria os
/// campos em ordem alfabética e o primeiro clique caía em `apex`.
#[test]
fn cliques_preenchem_as_guias_na_ordem_do_painel() {
    let (mut d, id) = doc_varredura();
    for i in 0..3 {
        assert!(clica(&mut d, id, aresta(i as f64)), "clique {i} preenche");
    }
    assert_eq!(guias(&d, id), [aresta(0.), aresta(1.), aresta(2.)]);
    assert!(!clica(&mut d, id, aresta(9.)), "com tudo preenchido o clique não troca nada");
    // esvaziar a Guia 2 (✖ do campo) faz o próximo clique voltar para ela
    if let Some(RefKind::SurfSweepX { kind: SweepKind::ConicGuides { guide2, .. }, .. }) = d.references.iter_mut().find(|r| r.id == id).map(|r| &mut r.kind) {
        *guide2 = curve();
    }
    assert!(clica(&mut d, id, aresta(7.)));
    assert_eq!(guias(&d, id), [aresta(0.), aresta(7.), aresta(2.)]);
    // perfil explícito: o perfil (declarado e desenhado primeiro) antes da guia, não em ordem de nome
    let mut k = RefKind::SurfSweepX { kind: SweepKind::Explicit { profile: curve(), guide1: curve(), guide2: None }, spine: None, solution: 0, smooth: false, sections: s("8") };
    assert_eq!(crate::slots::vazios_em_ordem(&k).len(), 2);
    assert!(fill(&mut k, &mut Cands::from_seed(&Seed { curves: vec![aresta(3.)], ..Default::default() }, None)));
    assert!(matches!(&k, RefKind::SurfSweepX { kind: SweepKind::Explicit { profile, .. }, .. } if *profile == aresta(3.)), "{k:?}");
}

/// Textos desenhados num quadro.
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

/// Desenha o painel da varredura numa coluna da largura padrão; devolve os textos e a largura usada.
fn desenha(d: &Document, id: Id, seed: &Seed) -> (Vec<String>, f32, f32) {
    let ctx = egui::Context::default();
    let mut r = d.reference(id).unwrap().clone();
    let largura = crate::paineis::LARGURA_COLUNA - 16.;
    let (mut usada, mut txt) = (0., vec![]);
    for _ in 0..3 {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 900.))), ..Default::default() };
        let mut saida = ctx.run_ui(raw, |root| {
            let caixa = egui::Rect::from_min_size(egui::pos2(8., 8.), egui::vec2(largura, 880.));
            let mut ui = root.new_child(egui::UiBuilder::new().max_rect(caixa));
            crate::geo_ui::panel(&mut ui, d, &mut r, &Default::default(), seed, None, None);
            usada = ui.min_rect().width();
        });
        saida.textures_delta.clear();
        txt = textos(&saida);
    }
    (txt, usada, largura)
}

/// O painel marca o campo que o próximo clique preenche, a marca avança com os cliques e os campos
/// cabem na coluna esquerda (a lista ▾ não alarga a linha com o texto do campo vazio).
#[test]
fn painel_marca_o_campo_ativo_e_cabe_na_coluna() {
    let (mut d, id) = doc_varredura();
    let seed = Seed::default();
    let (t, usada, largura) = desenha(&d, id, &seed);
    assert!(t.iter().any(|x| x == "▶ Guia 1"), "{t:?}");
    assert!(!t.iter().any(|x| x.starts_with("▶ Guia 2") || x.starts_with("▶ Guia do")), "um só campo ativo: {t:?}");
    assert!(usada <= largura + 0.5, "o painel usou {usada} de {largura}");
    clica(&mut d, id, aresta(0.));
    let (t, _, _) = desenha(&d, id, &seed);
    assert!(t.iter().any(|x| x == "▶ Guia 2"), "a marca avança: {t:?}");
    assert!(!t.iter().any(|x| x == "▶ Guia 1"), "{t:?}");
    clica(&mut d, id, aresta(1.));
    let (t, usada, _) = desenha(&d, id, &seed);
    assert!(t.iter().any(|x| x.starts_with("▶ Guia do")), "{t:?}");
    assert!(usada <= largura + 0.5, "com campos preenchidos (botão ✖) também cabe: {usada} de {largura}");
}

/// A lista ▾ oferece as curvas dos sketches e as arestas do corpo, com chave quando a aresta tem.
#[test]
fn lista_do_campo_tem_curvas_e_arestas_do_modelo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, _) = doc_varredura();
    let r = fenkra_solidos::recompute(&d);
    assert_eq!(r.bodies.len(), 1);
    let l = lista_de(&d, Some(&r));
    let curvas = l.iter().filter(|(_, c)| matches!(c, CurveRef::SketchCurve { .. })).count();
    let arestas: Vec<_> = l.iter().filter(|(_, c)| matches!(c, CurveRef::BodyEdge { .. })).collect();
    assert_eq!(curvas, 4, "as quatro linhas do retângulo");
    assert_eq!(arestas.len(), 12, "as doze arestas do bloco, todas com chave: {l:?}");
    assert!(arestas.iter().any(|(n, _)| n.ends_with("(20 mm)")), "a aresta vertical tem 20 mm: {arestas:?}");
    // escolhida na lista, a aresta ocupa o campo como o clique no 3D
    let (mut d2, id) = doc_varredura();
    assert!(clica(&mut d2, id, arestas[0].1.clone()));
    assert_eq!(guias(&d2, id)[0], arestas[0].1);
}
