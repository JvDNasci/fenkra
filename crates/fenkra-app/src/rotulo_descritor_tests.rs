// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pendências das ondas 1 e 2 (ADR 0107): o rótulo que o painel mostra em cada campo declarado é o do
//! descritor (`Entrada.rotulo`), nos comandos de superfície e de chapa já declarados, e os widgets de
//! curva do contrato não trazem mais um rótulo próprio no código do painel.

use crate::contrato::{Alvo, FeatureSpec, TipoEntrada};
use crate::geo_ui::{GeoKind, Seed};
use crate::i18n::tr;
use eframe::egui;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.into()
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

/// Um quadro do painel do elemento, como o `editor::show` desenha.
fn quadro(ctx: &egui::Context, d: &Document, r: &mut RefElement) -> Vec<String> {
    let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 1600.))), ..Default::default() };
    let mut saida = ctx.run_ui(raw, |root| {
        let caixa = egui::Rect::from_min_size(egui::pos2(8., 8.), egui::vec2(crate::paineis::LARGURA_COLUNA - 16., 1580.));
        let mut ui = root.new_child(egui::UiBuilder::new().max_rect(caixa));
        crate::campos::inicio(ui.ctx(), r.id.0, crate::campos::assinatura::<String>(&[]));
        crate::campos::definir_contrato(ui.ctx(), crate::contrato::id_do_ativo(Alvo::Ref(&r.kind)));
        crate::geo_ui::panel(&mut ui, d, r, &Default::default(), &Seed::default(), None, None);
        crate::campos::fim(ui.ctx());
    });
    saida.textures_delta.clear();
    textos(&saida)
}

fn mostra(t: &[String], rotulo: &str) -> bool {
    t.iter().any(|x| x.trim_start_matches("▶ ").starts_with(rotulo))
}

#[test]
fn rotulo_do_painel_vem_do_descritor() {
    let mut d = Document::new("t");
    let reta = |d: &mut Document, a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
    let (l1, l2, l3) = (reta(&mut d, [0., 0., 0.], [10., 0., 0.]), reta(&mut d, [10., 0., 0.], [20., 5., 0.]), reta(&mut d, [0., 0., 0.], [0., 0., 40.]));
    let folha = |d: &mut Document, c: CurveRef| SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: s("30"), length2: s("0"), until: None, until2: None, mais: vec![] }));
    let (ca, cb) = (reta(&mut d, [-40., 0., 0.], [0., 0., 0.]), reta(&mut d, [0., 10., 0.], [40., 10., 0.]));
    let (sa, sb) = (folha(&mut d, ca), folha(&mut d, cb));
    let z = AxisRef::Origin(OriginAxis::Z);
    let kinds = vec![
        RefKind::SurfExtrude { curve: l1.clone(), dir: z, length: s("30"), length2: s("0"), until: None, until2: None, mais: vec![l2.clone()] },
        RefKind::SurfRevolve { curve: l1.clone(), axis: AxisRef::Origin(OriginAxis::X), angle: s("90"), angle2: s("0"), mais: vec![l2.clone()] },
        RefKind::SurfSweep { profile: SweepProfile::Curve(l1.clone()), spine: l3.clone(), mode: SweepMode::Corrigido, mais: vec![l2.clone()] },
        RefKind::SurfBlend { a: l1.clone(), sa: Some(sa), b: l2.clone(), sb: Some(sb), cont: Continuity::G1, tension: s("1"), tension2: None },
        RefKind::SurfSplit { surface: sa, tool: sb, flip: false, more: vec![], curve: Some(l3.clone()), more_flip: vec![] },
        RefKind::CurveParallel { curve: l1.clone(), support: SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY)), d1: s("5"), d2: s("5"), flip: false, more: vec![], both: false, d_other: String::new() },
        RefKind::SurfLoft { sections: vec![l1.clone(), l2.clone()], guide: Some(l3.clone()), ruled: false, closed: false, opts: Default::default() },
    ];
    let mut conferidos = 0;
    for k in kinds {
        let id = d.add_geometry(k.clone());
        let mut r = d.reference(id).unwrap().clone();
        let ctx = egui::Context::default();
        quadro(&ctx, &d, &mut r);
        let t = quadro(&ctx, &d, &mut r);
        for e in FeatureSpec::de(Alvo::Ref(&k)).entradas.iter().filter(|e| e.declarada) {
            let rot = tr(e.rotulo);
            assert!(mostra(&t, rot), "{:?}: o painel não mostra o rótulo do descritor \"{rot}\" do campo {}: {t:?}", GeoKind::of(&k), e.id);
            conferidos += 1;
        }
        if matches!(k, RefKind::SurfExtrude { .. } | RefKind::SurfSweep { .. }) {
            assert!(t.iter().any(|x| x.ends_with(&format!("{} (2)", tr("Perfil")))), "o perfil em lista mostra os dois itens: {t:?}");
        }
    }
    assert!(conferidos >= 15, "conferiu {conferidos} campos");
    // sem duplicação: o widget de curva do contrato recebe o `id` do campo e lê o rótulo do descritor
    let fonte = include_str!("geo_ui.rs");
    for w in ["curva_contrato(ui, \"", "perfil_lista(ui, \""] {
        let mut n = 0;
        for (i, _) in fonte.match_indices(w) {
            let resto = &fonte[i + w.len()..];
            let depois = &resto[resto.find('"').unwrap() + 1..];
            assert!(depois.starts_with(", doc"), "o widget ainda traz o rótulo no painel: {}", &fonte[i..i + 60]);
            n += 1;
        }
        assert!(n >= 2, "não achei as chamadas de {w}");
    }
}

/// Chapa (B560): a lista do modo Arestas mostra o rótulo do descritor do recurso, o mesmo nos três.
#[test]
fn rotulo_das_arestas_da_aba_vem_do_descritor() {
    let d = Document::new("t");
    let kinds = vec![
        FeatureKind::SmFlange(SmFlange { edge: [1., 0., 0.], length: s("10"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }),
        FeatureKind::SmCurveFlange(SmCurveFlange { edge: vec![[1., 0., 0.], [2., 0., 0.]], length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None, so_aresta: true, reta: None }),
        FeatureKind::SmFlangeLoop(SmFlangeLoop { edges: vec![[1., 0., 0.]], edge_keys: vec![], loop_all: false, extent: ExtensaoAba::Inteira, invert: false, length: s("10"), angle: s("90"), radius: None, flip: false, side: 0, miter: true, relief: None, relief_kind: 0, seguir: false }),
    ];
    for mut k in kinds {
        let rotulo = FeatureSpec::de(Alvo::Feature(&k)).entradas.iter().find(|e| e.declarada && e.tipo == TipoEntrada::Aresta).map(|e| e.rotulo).expect("campo de arestas");
        assert_eq!(rotulo, "Arestas", "os três recursos do modo Arestas têm o mesmo nome");
        let ctx = egui::Context::default();
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 900.))), ..Default::default() };
        let mut saida = ctx.run_ui(raw, |root| {
            let mut ui = root.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(8., 8.), egui::vec2(380., 880.))));
            crate::chapa_unif_ui::suporte(&mut ui, &d, &mut k);
        });
        saida.textures_delta.clear();
        let t = textos(&saida);
        assert!(mostra(&t, tr(rotulo)), "{t:?}");
    }
}
