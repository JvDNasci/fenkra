// SPDX-License-Identifier: LGPL-2.1-or-later
//! Casos de `FENKRA_DEMO_PREVIA` da bancada Sólidos (B33, B300, B49, B301): cada um monta a peça de
//! partida em código e devolve o comando a abrir como novo, para a captura mostrar o painel, as
//! setas e a prévia sem mouse.

use crate::previa::DemoPrevia;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn sketch(d: &mut Document, plane: PlaneRef, monta: &dyn Fn(&mut SketchData, &mut dyn FnMut() -> Id)) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_700_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    monta(&mut data, &mut al);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

fn extrude(sk: Id, op: BoolOp, dist: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch: sk, op, extent: Extent::Blind(s(dist)), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() })
}

/// Caixa 40 x 30 x 20 com um boss de raio 5 em (0, 0) acima do topo e um filete r = 3 no canto (20, 15).
fn peca_com_relevos(d: &mut Document) {
    *d = Document::new("Edição direta");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let sk = sketch(d, xy, &|data, al| {
        data.add_rect(&mut &mut *al, [-20., -15.], [20., 15.], false);
    });
    d.add_feature(extrude(sk, BoolOp::Add, "20"));
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("3"), edges: EdgeSel::Edges(vec![[20., 15., 10.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    let sb = sketch(d, xy, &|data, al| {
        let c = data.add_point(&mut &mut *al, 0., 0.);
        data.add_curve(&mut &mut *al, Curve::Circle { c, r: 5. }, false);
    });
    d.add_feature(extrude(sb, BoolOp::Add, "30"));
}

fn edicao(op: OpDireta, faces: &[[f64; 3]], value: &str, delta: [&str; 3]) -> DemoPrevia {
    let mut e = DirectEdit::novo(op);
    e.faces = faces.to_vec();
    e.value = s(value);
    e.delta = delta.map(s);
    DemoPrevia::Comando(FeatureKind::DirectEdit(e))
}

/// Caminho que sai pelo eixo X, sobe e vira para o lado: uma curva com torção (pontos a cada 0,5 mm).
fn guia_com_torcao() -> Vec<[f64; 3]> {
    (0..=140)
        .map(|i| {
            let t = i as f64 * 0.5;
            [t, 8. * (1. - (t / 20.).cos()), 12. * (1. - (t / 28.).cos())]
        })
        .collect()
}

fn varredura(doc: &mut Document, modo: SweepMode) -> DemoPrevia {
    *doc = Document::new("Varredura");
    // a guia é uma polilinha de referência: o primeiro enquadramento já cobre o caminho todo
    let pontos = guia_com_torcao().into_iter().map(PointRef::Fixed).collect();
    let guia = doc.add_geometry(RefKind::Polyline { points: pontos, closed: false });
    // perfil no plano YZ (normal = X), no começo da guia
    let sk = sketch(doc, PlaneRef::Origin(OriginPlane::YZ), &|data, al| {
        data.add_rect(&mut &mut *al, [-20., -3.], [20., 3.], false);
    });
    DemoPrevia::Comando(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Ref(guia), mode: modo, op: BoolOp::Add, thin: None }))
}

/// Dois corpos em coletores separados: a caixa e uma ferramenta alta que a atravessa.
fn dois_corpos(d: &mut Document) -> (Id, Id) {
    *d = Document::new("Booleana");
    let xy = PlaneRef::Origin(OriginPlane::XY);
    let a = d.active_collector;
    let sk = sketch(d, xy, &|data, al| {
        data.add_rect(&mut &mut *al, [-25., -25.], [25., 25.], false);
    });
    d.add_feature(extrude(sk, BoolOp::Add, "20"));
    let b = d.add_collector("Ferramenta");
    d.active_collector = b;
    let sk2 = sketch(d, xy, &|data, al| {
        let c = data.add_point(&mut &mut *al, 0., 0.);
        data.add_curve(&mut &mut *al, Curve::Circle { c, r: 12. }, false);
    });
    d.add_feature(extrude(sk2, BoolOp::Add, "40"));
    d.active_collector = a;
    (a, b)
}

pub fn demo(nome: &str, doc: &mut Document) -> Option<DemoPrevia> {
    Some(match nome {
        "edicao_mover" => {
            peca_com_relevos(doc);
            edicao(OpDireta::Mover, &[[5., 0., 25.], [0., 0., 30.]], "0", ["10", "6", "0"])
        }
        "edicao_apagar" => {
            peca_com_relevos(doc);
            edicao(OpDireta::Apagar, &[[5., 0., 25.], [0., 0., 30.]], "0", ["0", "0", "0"])
        }
        "edicao_deslocar" => {
            peca_com_relevos(doc);
            edicao(OpDireta::Deslocar, &[[5., 0., 25.]], "3", ["0", "0", "0"])
        }
        "edicao_raio" => {
            peca_com_relevos(doc);
            let k = 3. / 2f64.sqrt();
            edicao(OpDireta::RaioFilete, &[[17. + k, 12. + k, 10.]], "7", ["0", "0", "0"])
        }
        "booleana_cliques" => {
            let (_, b) = dois_corpos(doc);
            DemoPrevia::Comando(FeatureKind::Combine(Combine { tool: b, op: BoolOp::Cut, keep_tool: false }))
        }
        "filete_controles" => {
            *doc = Document::new("Filete variável");
            let sk = sketch(doc, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
                data.add_rect(&mut &mut *al, [-20., -15.], [20., 15.], false);
            });
            doc.add_feature(extrude(sk, BoolOp::Add, "20"));
            let extra = EdgeExtra { controls: vec![ControlePonto { at: s("30"), radius: s("5") }, ControlePonto { at: s("70"), radius: s("3") }], smooth: true, ..Default::default() };
            DemoPrevia::Comando(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![[0., -15., 20.]]), mode: EdgeMode::Variable(s("6")), prop: Default::default(), keys: vec![], extra }))
        }
        "varredura_sem_torcao" => varredura(doc, SweepMode::Corrigido),
        "varredura_torcao" => varredura(doc, SweepMode::Frenet),
        "varredura_fixa" => varredura(doc, SweepMode::Fixo),
        _ => return None,
    })
}
