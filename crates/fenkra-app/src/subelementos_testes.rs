// SPDX-License-Identifier: LGPL-2.1-or-later
//! B577: o que o painel mostra para face, aresta e vértice. Documento montado em código e recalculado; a
//! tabela de nomes do cálculo alimenta `ref_nome` e `Pick::label`, e nenhum rótulo do fluxo normal pode
//! trazer coordenada, índice interno nem "perto de".

use crate::picking::{Pick, SURF_BASE};
use fenkra_core::sketch::Curve;
use fenkra_core::{AxisRef, BoolOp, CurveRef, Document, Extent, Extrude, FeatureKind, Id, OriginAxis, OriginPlane, PlaneRef, PointRef, RefKind, Sketch, SketchData};
use fenkra_solidos::recompute;

fn rect(d: &mut Document, a: [f64; 2], b: [f64; 2]) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_700_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let ids: Vec<Id> = [[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]].iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
    for i in 0..4 {
        data.add_curve(&mut al, Curve::Line { a: ids[i], b: ids[(i + 1) % 4] }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
}

fn sem_coordenada(txt: &str) {
    let txt = &txt.replace(" (espera o cálculo)", "");
    for ruim in ['(', ')', ',', '#', ';'] {
        assert!(!txt.contains(ruim), "coordenada ou índice no rótulo: {txt}");
    }
    assert!(!txt.contains("perto de"), "{txt}");
    // nenhum número com casa decimal (as cotas e os pontos têm; o número de face e aresta não)
    let b = txt.as_bytes();
    for i in 1..b.len().saturating_sub(1) {
        assert!(!(b[i] == b'.' && b[i - 1].is_ascii_digit() && b[i + 1].is_ascii_digit()), "número decimal no rótulo: {txt}");
    }
}

#[test]
fn nenhum_rotulo_do_fluxo_normal_tem_coordenada_nem_indice() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b577 app");
    let sk = rect(&mut d, [0., 0.], [40., 30.]);
    let bloco = d.add_feature(FeatureKind::Extrude(Extrude { thin_side: 0, sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, ends: Default::default() }));
    let linha = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([100., 0., 0.]), b: PointRef::Fixed([100., 30., 0.]), start: "0".into(), end: "0".into() });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(linha), dir: AxisRef::Origin(OriginAxis::X), length: "20".into(), length2: "0".into(), until: None, until2: None, mais: vec![] });
    let r = recompute(&d);
    crate::ref_nome::atualizar(&r);
    let t = crate::ref_nome::tabela().expect("tabela");
    assert!(!t.vazia());

    let mut vistos = std::collections::HashSet::new();
    let mut total = 0;
    let mut confere = |p: Pick, esperado_dono: Id| {
        let unico = !matches!(p, Pick::Vertex { .. });
        let nome = p.label(&d);
        sem_coordenada(&nome);
        assert!(nome.starts_with(&d.name_of(esperado_dono)), "{nome}");
        assert!(nome.contains("Face "), "{nome}");
        // um vértice é de várias arestas: ele tem um nome só, o da menor face e aresta que o tocam
        assert!(!unico || vistos.insert(nome.clone()), "nome repetido: {nome}");
        total += 1;
    };
    for f in &t.faces {
        let dono = f.dono;
        confere(crate::caixa_sub::pick_da_face(f), dono);
    }
    for a in &t.arestas {
        let dono = t.faces[a.face].dono;
        confere(crate::caixa_sub::pick_da_aresta(a), dono);
        for k in [1, 2] {
            confere(crate::caixa_sub::pick_do_vertice(a, k), dono);
        }
    }
    assert!(total > 30, "{total}");
    // o corpo e a superfície entram na tabela, cada um com o dono dele
    assert!(t.faces.iter().any(|f| f.dono == bloco && f.corpo < SURF_BASE));
    assert!(t.faces.iter().any(|f| f.dono == sup && f.corpo >= SURF_BASE));

    // campos de painel: com chave, o nome da hierarquia; sem chave, "sem nome" e nenhuma coordenada
    let b = &r.bodies[0];
    let a = t.arestas.iter().find(|a| a.corpo == 0 && b.edge_key(a.aresta).is_some()).unwrap();
    let k = b.edge_key(a.aresta).unwrap();
    let nome = crate::ref_nome::campo_aresta(&d, Some(&k), [1.5, 2.5, 3.5]);
    assert_eq!(nome, Pick::Edge { body: 0, edge: a.aresta, mid: a.meio }.label(&d), "o campo e o clique dizem o mesmo nome");
    sem_coordenada(&nome);
    sem_coordenada(&crate::ref_nome::campo_aresta(&d, None, [1.5, 2.5, 3.5]));
    sem_coordenada(&crate::ref_nome::campo_face(&d, None, [1.5, 2.5, 3.5]));
    let fk = b.face_key(0).unwrap();
    sem_coordenada(&crate::ref_nome::campo_face(&d, Some(&fk), [1.5, 2.5, 3.5]));
    // vértice e face guardados como ponto (canto de chapa, face aberta): o nome do que está ali
    let v = a.pontas[0];
    let canto = crate::ref_nome::campo_vertice(&d, [v[0] as f64, v[1] as f64, v[2] as f64]);
    sem_coordenada(&canto);
    assert!(canto.contains("Vértice"), "{canto}");
    let alto = crate::ref_nome::campo_face_no_ponto(&d, [20., 15., 20.]);
    sem_coordenada(&alto);
    assert!(alto.contains("Face"), "{alto}");
    // curva: trechos e pontas já eram por número
    sem_coordenada(&crate::geo_ui::curve_name(&d, &CurveRef::Segment { curve: linha, index: 0 }));
    sem_coordenada(&crate::geo_ui::point_name(&d, &fenkra_core::PointRef::CurveVertex { curve: linha, index: 0 }));
}

#[test]
fn escolher_um_nivel_da_arvore_da_a_mesma_chave_do_clique_no_3d() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b577 arvore");
    let sk = rect(&mut d, [0., 0.], [40., 30.]);
    d.add_feature(FeatureKind::Extrude(Extrude { thin_side: 0, sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, ends: Default::default() }));
    let r = recompute(&d);
    crate::ref_nome::atualizar(&r);
    let t = crate::ref_nome::tabela().unwrap();
    let b = &r.bodies[0];
    // cada aresta da árvore, escolhida, é a aresta do corpo com a mesma chave que o 3D grava
    for a in &t.arestas {
        let Pick::Edge { body, edge, .. } = crate::caixa_sub::pick_da_aresta(a) else { panic!() };
        assert_eq!(body, 0);
        let k = b.edge_key(edge).expect("chave");
        assert_eq!(t.aresta_da_chave(&k).map(|x| x.aresta), Some(edge));
    }
    for f in &t.faces {
        let Pick::Face { face, .. } = crate::caixa_sub::pick_da_face(f) else { panic!() };
        assert_eq!(t.face_da_chave(&b.face_key(face).unwrap()).map(|x| x.face), Some(face));
    }
    // a árvore desenha sem entrar em pânico, com o nó do dono aberto
    let ctx = eframe::egui::Context::default();
    let raw = eframe::egui::RawInput { screen_rect: Some(eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(600., 600.))), ..Default::default() };
    let mut saida = ctx.run_ui(raw, |root| {
        let mut ui = root.new_child(eframe::egui::UiBuilder::new().max_rect(eframe::egui::Rect::from_min_size(eframe::egui::pos2(8., 8.), eframe::egui::vec2(380., 580.))));
        let ctx = ui.ctx().clone();
        crate::caixa_sub::navegador(&mut ui, &ctx, &d, "g_x");
    });
    saida.textures_delta.clear();
}
