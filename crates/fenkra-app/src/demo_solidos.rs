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

/// B350: guia em U com cantos vivos e um tubo de raio 4: a seção gira em cada canto (esquadria).
fn varredura_cantos(doc: &mut Document) -> DemoPrevia {
    *doc = Document::new("Varredura em cantos");
    let pontos = [[0., 0., 0.], [60., 0., 0.], [60., 50., 0.], [0., 50., 30.]].into_iter().map(PointRef::Fixed).collect();
    let guia = doc.add_geometry(RefKind::Polyline { points: pontos, closed: false });
    let sk = sketch(doc, PlaneRef::Origin(OriginPlane::YZ), &|data, al| {
        let c = data.add_point(&mut &mut *al, 0., 0.);
        data.add_curve(&mut &mut *al, Curve::Circle { c, r: 6. }, false);
        let c2 = data.add_point(&mut &mut *al, 0., 0.);
        data.add_curve(&mut &mut *al, Curve::Circle { c: c2, r: 4. }, false);
    });
    DemoPrevia::Comando(FeatureKind::Sweep(Sweep { sketch: sk, spine: CurveRef::Ref(guia), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }))
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

/// Caixa 50 x 40 x 20 centrada em x e y.
fn caixa_simples(d: &mut Document) -> Id {
    *d = Document::new("Caixa");
    let sk = sketch(d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
        data.add_rect(&mut &mut *al, [-25., -20.], [25., 20.], false);
    });
    d.add_feature(extrude(sk, BoolOp::Add, "20"))
}

/// Pino 10 x 10 x 10 de onde saem as cópias dos padrões e do espelho, sobre uma base fina que ocupa o
/// espaço das cópias: o primeiro enquadramento da câmera já cobre todas.
fn pino(d: &mut Document, x0: f64, base: [[f64; 2]; 2]) -> Id {
    *d = Document::new("Pino");
    let sb = sketch(d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
        data.add_rect(&mut &mut *al, base[0], base[1], false);
    });
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(s("2")), reverse: true, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    let sk = sketch(d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
        data.add_rect(&mut &mut *al, [x0, 0.], [x0 + 10., 10.], false);
    });
    d.add_feature(extrude(sk, BoolOp::Add, "10"))
}

fn filete(valor: &str, aresta: [f64; 3], prop: EdgeProp) -> FeatureKind {
    FeatureKind::Fillet(EdgeOp { value: s(valor), edges: EdgeSel::Edges(vec![aresta]), mode: EdgeMode::Constant, prop, keys: vec![], extra: Default::default() })
}

fn furo_escareado(d: &mut Document, depth: Extent) {
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let sp = sketch(d, PlaneRef::Datum(topo), &|data, al| {
        let p = data.add_point(&mut &mut *al, 0., 0.);
        data.add_curve(&mut &mut *al, Curve::Point { p }, false);
    });
    d.add_feature(FeatureKind::Hole(Hole {
        sketch: sp,
        kind: HoleKind::Escareado { diameter: s("14"), angle: s("90") },
        diameter: s("6.6"),
        depth,
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    }));
}

pub fn demo(nome: &str, doc: &mut Document) -> Option<DemoPrevia> {
    // B17: `passo_<lado em mm>`, caixa desse tamanho com a extrusão aberta (com `FENKRA_DEMO_PASSO`)
    if let Some(lado) = nome.strip_prefix("passo_").and_then(|t| t.parse::<f64>().ok()) {
        *doc = Document::new("Passo");
        let sk = sketch(doc, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
            data.add_rect(&mut &mut *al, [-lado / 2., -lado / 2.], [lado / 2., lado / 2.], false);
        });
        // baixa, para a seta caber na vista de qualquer tamanho
        return Some(DemoPrevia::Comando(extrude(sk, BoolOp::Add, &(lado / 5.).to_string())));
    }
    // B228: `plano_<lado em mm>`, cubo desse lado e o plano deslocado de 0,4 lado aberto (retângulo
    // translúcido, seta, rótulo e botão de inverter); com `FENKRA_DEMO_INVERTER=1` o botão é apertado
    if let Some(lado) = nome.strip_prefix("plano_").and_then(|t| t.parse::<f64>().ok()) {
        *doc = Document::new("Plano");
        let sk = sketch(doc, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
            data.add_rect(&mut &mut *al, [-lado / 2., -lado / 2.], [lado / 2., lado / 2.], false);
        });
        doc.add_feature(extrude(sk, BoolOp::Add, &(lado / 5.).to_string()));
        return Some(DemoPrevia::Ref(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: (lado * 0.4).to_string() }));
    }
    // B228: ponto por coordenadas aberto, para ver a esfera de posição entre as setas e os planos
    if nome == "ponto_xyz" {
        *doc = Document::new("Ponto");
        let sk = sketch(doc, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
            data.add_rect(&mut &mut *al, [-20., -20.], [20., 20.], false);
        });
        doc.add_feature(extrude(sk, BoolOp::Add, "8"));
        return Some(DemoPrevia::Ref(RefKind::PointCoords { x: "10".into(), y: "6".into(), z: "20".into(), frame: None }));
    }
    Some(match nome {
        // B91: padrão linear com a terceira cópia pulada (ponto com X) e as setas de espaçamento
        "padrao_pontos" => {
            let b = pino(doc, 0., [[-10., -8.], [90., 18.]]);
            DemoPrevia::Comando(FeatureKind::Pattern(Pattern {
                kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: s("4"), spacing: s("20"), dir2: None, count2: s("1"), spacing2: s("0"), total: false },
                sources: vec![b],
                skip: vec![(2, 0)],
            }))
        }
        // B91: padrão circular com o arco do ângulo total
        "padrao_circular" => {
            let b = pino(doc, 15., [[-30., -30.], [30., 30.]]);
            DemoPrevia::Comando(FeatureKind::Pattern(Pattern {
                kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: s("5"), angle: s("240") },
                sources: vec![b],
                skip: vec![(3, 0)],
            }))
        }
        // B91 e B275: o plano do espelho translúcido, com rótulo
        "espelho" => {
            let b = pino(doc, 8., [[-30., -8.], [30., 18.]]);
            DemoPrevia::Comando(FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::YZ), sources: vec![b] }))
        }
        // B90: o furo-guia pousa no centro da face de cima (sem mouse, só para a captura)
        "furo_guia" => {
            caixa_simples(doc);
            DemoPrevia::Comando(FeatureKind::Hole(Hole {
                sketch: crate::slots::UNSET_SKETCH,
                kind: HoleKind::Escareado { diameter: s("13") , angle: s("90") },
                diameter: s("6.6"),
                depth: Extent::Blind(s("14")),
                thread: None,
                thread_modeled: false,
                standard: None,
                points: vec![],
                dir: None,
                reverse: false,
                tip: None,
                taper: s("0"),
                left_hand: false,
                extra: Default::default(),
            }))
        }
        // B289: filete com propagação tangente: o canto vertical arredondado faz a cadeia passar por três arestas
        "filete_tangente" => {
            caixa_simples(doc);
            doc.add_feature(filete("6", [25., -20., 10.], EdgeProp::Tangente));
            // depois do filete do canto, a aresta de cima da frente vai de x = -25 a 19: o ponto médio é -3
            DemoPrevia::Comando(filete("3", [-3., -20., 20.], EdgeProp::Tangente))
        }
        // B289: filete assimétrico com a seta que aponta para a face da primeira medida
        "filete_assimetrico" => {
            caixa_simples(doc);
            DemoPrevia::Comando(FeatureKind::Fillet(EdgeOp { value: s("4"), edges: EdgeSel::Edges(vec![[0., -20., 20.]]), mode: EdgeMode::Asym(s("10")), prop: EdgeProp::Tangente, keys: vec![], extra: EdgeExtra { flips: vec![false], ..Default::default() } }))
        }
        // B289: filete parcial, com as setas de início e de fim do trecho
        "filete_parcial" => {
            caixa_simples(doc);
            DemoPrevia::Comando(FeatureKind::Fillet(EdgeOp { value: s("4"), edges: EdgeSel::Edges(vec![[0., -20., 20.]]), mode: EdgeMode::Constant, prop: EdgeProp::Tangente, keys: vec![], extra: EdgeExtra { partial: Some([s("10"), s("35")]), ..Default::default() } }))
        }
        // B246 e B289: raio grande demais; a última prévia boa fica, a causa vai junto da seta e a superfície tentada aparece
        "filete_grande" => {
            caixa_simples(doc);
            DemoPrevia::Comando(filete("30", [0., -20., 20.], EdgeProp::Tangente))
        }
        // B48: casca abrindo a face do escareado de um furo cego
        "escareado_casca" => {
            caixa_simples(doc);
            furo_escareado(doc, Extent::Blind(s("12")));
            DemoPrevia::Comando(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[20., 15., 20.]], thickness: s("3"), outward: false, face_keys: vec![], closed: false }))
        }
        // B288 e B89: um término por lado
        "dois_lados" => {
            *doc = Document::new("Dois lados");
            let sk = sketch(doc, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
                data.add_rect(&mut &mut *al, [-20., -15.], [20., 15.], false);
            });
            DemoPrevia::Comando(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::TwoSides(s("14"), s("6")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }))
        }
        // B288: ângulo de saída da extrusão, com o arco na aresta do perfil
        "saida" => {
            *doc = Document::new("Saída");
            let sk = sketch(doc, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
                data.add_rect(&mut &mut *al, [-12., -12.], [12., 12.], false);
            });
            DemoPrevia::Comando(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("6"), thin: None, thin_side: 0, dir: None, ends: Default::default() }))
        }
        // B288: base da primitiva (largura e profundidade) e altura
        "primitiva_base" => {
            *doc = Document::new("Primitiva");
            DemoPrevia::Comando(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("30"), b: s("20"), c: s("12"), d: s("0"), op: BoolOp::Add }))
        }
        // B288: mover corpo, com a seta de translação; "escala_corpo" mostra uma seta por eixo
        "mover_corpo" => {
            caixa_simples(doc);
            DemoPrevia::Comando(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("30") } }))
        }
        "escala_corpo" => {
            caixa_simples(doc);
            DemoPrevia::Comando(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Affinity { plane: PlaneRef::Origin(OriginPlane::XY), fx: s("1.4"), fy: s("1"), fz: s("0.6") } }))
        }
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
        "varredura_cantos" => varredura_cantos(doc),
        _ => return None,
    })
}
