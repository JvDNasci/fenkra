// SPDX-License-Identifier: LGPL-2.1-or-later
//! B60: casos das mídias de tooltip que faltavam na barra de sólidos. Cada caso monta a peça de
//! partida em código e devolve o comando que o roteiro abre como novo. Quando o comando não tem
//! número para arrastar (booleana, espelho, multi-seções), o roteiro anima um comando ou uma
//! referência da peça (`Animacao::feature` e `Animacao::referencia`); os índices citados nos
//! roteiros estão anotados em cada caso.

use crate::demo_plastico;
use crate::previa::DemoPrevia;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

/// Nomes dos casos deste módulo (um por roteiro).
pub const CASOS: [&str; 41] = [
    "ext_simetrico", "ext_ate_plano", "ext_saida_filete", "ext_cego", "ranhura", "nervura", "rasgo_varrido", "multisecoes", "multisecoes_remover", "prim_toro", "prim_cunha",
    "prim_prisma", "bool_unir", "bool_subtrair", "bool_intersecao", "furo_rebaixado", "furo_escareado", "furo_roscado", "filete_variavel", "filete_face_face", "arestas_todas",
    "chanfro_dist_dist", "chanfro_dist_angulo", "chanfro_dist", "edit_mover_face", "edit_deslocar_face", "edit_raio_filete", "padrao_circular", "padrao_pontos", "espelho", "escala",
    "xf_mover", "xf_girar", "xf_simetria", "xf_afinidade", "pl_boss", "pl_snap", "pl_grelha", "pl_lip", "pl_ranhura", "edit_apagar_face",
];

fn s(x: &str) -> String {
    x.to_string()
}

fn sketch(d: &mut Document, plane: PlaneRef, monta: &dyn Fn(&mut SketchData, &mut dyn FnMut() -> Id)) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_900_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    monta(&mut data, &mut al);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

fn retangulo(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
    sketch(d, plane, &|data, al| {
        data.add_rect(&mut &mut *al, a, b, false);
    })
}

fn circulo(d: &mut Document, plane: PlaneRef, c: [f64; 2], r: f64) -> Id {
    sketch(d, plane, &|data, al| {
        let p = data.add_point(&mut &mut *al, c[0], c[1]);
        data.add_curve(&mut &mut *al, Curve::Circle { c: p, r }, false);
    })
}

fn ponto(d: &mut Document, plane: PlaneRef, c: [f64; 2]) -> Id {
    sketch(d, plane, &|data, al| {
        let p = data.add_point(&mut &mut *al, c[0], c[1]);
        data.add_curve(&mut &mut *al, Curve::Point { p }, false);
    })
}

fn extrusao(sk: Id, op: BoolOp, extent: Extent) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch: sk, op, extent, reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() })
}

fn xy() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::XY)
}

/// Caixa 60 x 40 x 30 (features 0 e 1).
fn caixa(d: &mut Document) {
    let sk = retangulo(d, xy(), [-30., -20.], [30., 20.]);
    d.add_feature(extrusao(sk, BoolOp::Add, Extent::Blind(s("30"))));
}

/// Plano do topo da caixa, escondido: a mídia mostra só a peça.
fn topo(d: &mut Document, z: &str) -> PlaneRef {
    let p = d.add_offset_plane(xy(), z);
    if let Some(r) = d.references.iter_mut().find(|r| r.id == p) {
        r.visible = false;
    }
    PlaneRef::Datum(p)
}

fn arestas_do_topo() -> Vec<[f64; 3]> {
    vec![[0., 20., 30.], [0., -20., 30.], [30., 0., 30.], [-30., 0., 30.]]
}

fn aresta(value: &str, edges: EdgeSel, mode: EdgeMode) -> EdgeOp {
    EdgeOp { value: s(value), edges, mode, prop: Default::default(), keys: vec![], extra: Default::default() }
}

fn furo(sk: Id, kind: HoleKind, diameter: &str, depth: &str, thread: Option<String>) -> FeatureKind {
    FeatureKind::Hole(Hole {
        sketch: sk,
        kind,
        diameter: s(diameter),
        depth: Extent::Blind(s(depth)),
        thread,
        thread_modeled: false,
        standard: None,
        points: vec![],
        dir: None,
        // o sketch está no topo: sem inverter, o furo desce para dentro da caixa
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    })
}

/// Placa 45 x 40 x 8 abaixo do plano XY e a guia em polilinha (referência 0) saindo da origem ao
/// longo de X; o perfil é um círculo de raio 4 no plano YZ, no começo da guia.
fn placa_e_guia(d: &mut Document) -> (Id, CurveRef) {
    let sk = retangulo(d, xy(), [-5., -20.], [40., 20.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("8")), reverse: true, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    let pontos = [[0., 0., 0.], [12., 0., 0.], [24., 0., 0.], [36., 0., 0.]].into_iter().map(PointRef::Fixed).collect();
    let guia = d.add_geometry(RefKind::Polyline { points: pontos, closed: false });
    let perfil = circulo(d, PlaneRef::Origin(OriginPlane::YZ), [0., 0.], 4.);
    (perfil, CurveRef::Ref(guia))
}

/// Dois coletores: a caixa 50 x 50 x 20 (features 0 e 1) e o cilindro de raio 12 da ferramenta
/// (features 2 e 3, altura animada).
fn dois_corpos(d: &mut Document, altura: &str) -> Id {
    let a = d.active_collector;
    let sk = retangulo(d, xy(), [-25., -25.], [25., 25.]);
    d.add_feature(extrusao(sk, BoolOp::Add, Extent::Blind(s("20"))));
    let b = d.add_collector("Ferramenta");
    d.active_collector = b;
    let sk2 = circulo(d, xy(), [0., 0.], 12.);
    d.add_feature(extrusao(sk2, BoolOp::Add, Extent::Blind(s(altura))));
    d.active_collector = a;
    b
}

/// Base fina 60 x 26 x 2 (features 0 e 1) e o pino 10 x 10 em `x0` (features 2 e 3, altura 10).
fn pino(d: &mut Document, x0: f64, base: [[f64; 2]; 2]) -> Id {
    let sb = retangulo(d, xy(), base[0], base[1]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind(s("2")), reverse: true, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    let sk = retangulo(d, xy(), [x0, 0.], [x0 + 10., 10.]);
    d.add_feature(extrusao(sk, BoolOp::Add, Extent::Blind(s("10"))))
}

/// Caixa 40 x 30 x 20 com um boss de raio 5 no meio do topo e um filete r = 3 no canto vertical
/// (20, 15): o caso da edição direta.
fn peca_com_relevos(d: &mut Document) {
    let sk = retangulo(d, xy(), [-20., -15.], [20., 15.]);
    d.add_feature(extrusao(sk, BoolOp::Add, Extent::Blind(s("20"))));
    d.add_feature(FeatureKind::Fillet(aresta("3", EdgeSel::Edges(vec![[20., 15., 10.]]), EdgeMode::Constant)));
    let sb = circulo(d, xy(), [0., 0.], 5.);
    d.add_feature(extrusao(sb, BoolOp::Add, Extent::Blind(s("30"))));
}

fn edicao(op: OpDireta, faces: &[[f64; 3]], value: &str) -> FeatureKind {
    let mut e = DirectEdit::novo(op);
    e.faces = faces.to_vec();
    e.value = s(value);
    e.delta = [s("0"), s("0"), s("0")];
    FeatureKind::DirectEdit(e)
}

fn primitiva(kind: PrimKind, v: [&str; 4]) -> FeatureKind {
    FeatureKind::Primitive(Primitive { kind, plane: xy(), a: s(v[0]), b: s(v[1]), c: s(v[2]), d: s(v[3]), op: BoolOp::Add })
}

fn mover(xf: BodyXf) -> FeatureKind {
    FeatureKind::MoveBody(MoveBody { xf })
}

/// Documento de partida e comando novo do caso; `None` se o nome não é deste módulo.
pub fn montar(caso: &str) -> Option<(Document, FeatureKind)> {
    let mut d = Document::new("Mídia");
    let kind = match caso {
        "ext_simetrico" => {
            let sk = retangulo(&mut d, xy(), [-30., -20.], [30., 20.]);
            extrusao(sk, BoolOp::Add, Extent::Symmetric(s("8")))
        }
        // referência 0: o plano de término, deslocado do XY
        "ext_ate_plano" => {
            let sk = retangulo(&mut d, xy(), [-30., -20.], [30., 20.]);
            let p = d.add_offset_plane(xy(), "8");
            extrusao(sk, BoolOp::Add, Extent::UpTo(PlaneRef::Datum(p)))
        }
        // feature 1: a extrusão com saída; o comando aberto é o filete nas arestas criadas por ela,
        // o par que o atalho cria
        "ext_saida_filete" => {
            let sk = retangulo(&mut d, xy(), [-20., -15.], [20., 15.]);
            let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
            FeatureKind::Fillet(aresta("0.5", EdgeSel::CreatedBy(ex), EdgeMode::Constant))
        }
        "ext_cego" => {
            caixa(&mut d);
            let t = topo(&mut d, "30");
            let sk = retangulo(&mut d, t, [-12., -10.], [12., 10.]);
            extrusao(sk, BoolOp::Cut, Extent::Blind(s("2")))
        }
        // eixo Y: o cilindro de partida é uma revolução de 360°, a ranhura gira o perfil no flanco
        "ranhura" => {
            let base = retangulo(&mut d, xy(), [0., 0.], [16., 40.]);
            d.add_feature(FeatureKind::Revolve(Revolve { sketch: base, axis: AxisRef::Origin(OriginAxis::Y), angle: s("360"), symmetric: false, op: BoolOp::Add, thin: None, thin_side: 0, angle2: None }));
            let sk = retangulo(&mut d, xy(), [12., 16.], [17., 24.]);
            FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Y), angle: s("20"), symmetric: false, op: BoolOp::Cut, thin: None, thin_side: 0, angle2: None })
        }
        // referência 0: a guia; o último ponto se desloca em Y e a guia faz a curva
        "nervura" | "rasgo_varrido" => {
            let (perfil, spine) = placa_e_guia(&mut d);
            FeatureKind::Sweep(Sweep { sketch: perfil, spine, mode: SweepMode::Corrigido, op: if caso == "nervura" { BoolOp::Add } else { BoolOp::Cut }, thin: None })
        }
        // referência 0: o plano do segundo perfil
        "multisecoes" => {
            let a = retangulo(&mut d, xy(), [-20., -15.], [20., 15.]);
            let p = topo(&mut d, "15");
            let b = circulo(&mut d, p, [0., 0.], 8.);
            FeatureKind::Loft(Loft { sections: vec![a, b], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() })
        }
        // referência 1: o plano do perfil do fundo, que desce para dentro da caixa
        "multisecoes_remover" => {
            caixa(&mut d);
            let t = topo(&mut d, "30");
            let a = retangulo(&mut d, t, [-20., -12.], [20., 12.]);
            let p = topo(&mut d, "26");
            let b = circulo(&mut d, p, [0., 0.], 6.);
            FeatureKind::Loft(Loft { sections: vec![a, b], ruled: false, closed: false, op: BoolOp::Cut, torcao: vec![], inicio: Default::default(), fim: Default::default() })
        }
        "prim_toro" => primitiva(PrimKind::Toro, ["20", "3", "0", "0"]),
        "prim_cunha" => primitiva(PrimKind::Cunha, ["40", "30", "20", "40"]),
        "prim_prisma" => primitiva(PrimKind::Prisma, ["6", "15", "4", "0"]),
        // features 2 e 3: a ferramenta, cuja altura cresce
        "bool_unir" => {
            let b = dois_corpos(&mut d, "10");
            FeatureKind::Combine(Combine { tool: b, op: BoolOp::Add, keep_tool: false })
        }
        "bool_subtrair" => {
            let b = dois_corpos(&mut d, "4");
            FeatureKind::Combine(Combine { tool: b, op: BoolOp::Cut, keep_tool: false })
        }
        "bool_intersecao" => {
            let b = dois_corpos(&mut d, "4");
            FeatureKind::Combine(Combine { tool: b, op: BoolOp::Intersect, keep_tool: false })
        }
        "furo_rebaixado" | "furo_escareado" | "furo_roscado" => {
            caixa(&mut d);
            let t = topo(&mut d, "30");
            let c = ponto(&mut d, t, [0., 0.]);
            match caso {
                "furo_rebaixado" => furo(c, HoleKind::Rebaixado { diameter: s("11"), depth: s("5") }, "6.6", "20", None),
                "furo_escareado" => furo(c, HoleKind::Escareado { diameter: s("8"), angle: s("90") }, "6.6", "20", None),
                _ => furo(c, HoleKind::Simples, "5", "4", Some(s("M6x1"))),
            }
        }
        "filete_variavel" => {
            caixa(&mut d);
            FeatureKind::Fillet(aresta("1", EdgeSel::Edges(vec![[0., -20., 30.]]), EdgeMode::Variable(s("1"))))
        }
        // a face da frente e a de cima: o filete fica na aresta comum
        "filete_face_face" => {
            caixa(&mut d);
            FeatureKind::Fillet(aresta("1", EdgeSel::AllOfBody, EdgeMode::FaceFace([0., -20., 15.], [0., 0., 30.])))
        }
        "arestas_todas" => {
            caixa(&mut d);
            FeatureKind::Fillet(aresta("0.5", EdgeSel::AllOfBody, EdgeMode::Constant))
        }
        "chanfro_dist_dist" => {
            caixa(&mut d);
            FeatureKind::Chamfer(aresta("2", EdgeSel::Edges(arestas_do_topo()), EdgeMode::TwoDist(s("1"))))
        }
        "chanfro_dist_angulo" => {
            caixa(&mut d);
            FeatureKind::Chamfer(aresta("4", EdgeSel::Edges(arestas_do_topo()), EdgeMode::DistAngle(s("20"))))
        }
        "chanfro_dist" => {
            caixa(&mut d);
            FeatureKind::Chamfer(aresta("0.5", EdgeSel::AllOfBody, EdgeMode::Constant))
        }
        "edit_mover_face" => {
            peca_com_relevos(&mut d);
            edicao(OpDireta::Mover, &[[5., 0., 25.], [0., 0., 30.]], "0")
        }
        // a face lateral da caixa simples (a peça com boss e filete por histórico também desloca
        // desde o B464, coberta em fenkra-solidos/tests/edicao_direta_b464.rs)
        "edit_deslocar_face" => {
            caixa(&mut d);
            edicao(OpDireta::Deslocar, &[[-30., 0., 15.]], "0.5")
        }
        "edit_raio_filete" => {
            peca_com_relevos(&mut d);
            let k = 3. / 2f64.sqrt();
            edicao(OpDireta::RaioFilete, &[[17. + k, 12. + k, 10.]], "1")
        }
        // a cura: o boss some e o topo se fecha; a feature 1 (altura da caixa) cresce para a peça
        // não ficar parada enquanto o comando já mostra o resultado
        "edit_apagar_face" => {
            peca_com_relevos(&mut d);
            edicao(OpDireta::Apagar, &[[5., 0., 25.], [0., 0., 30.]], "0")
        }
        "padrao_circular" => {
            let b = pino(&mut d, 15., [[-30., -30.], [30., 30.]]);
            FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: s("6"), angle: s("60") }, sources: vec![b], skip: vec![] })
        }
        // a segunda cópia corre em X ao longo da base
        "padrao_pontos" => {
            let b = pino(&mut d, -30., [[-32., -8.], [32., 18.]]);
            FeatureKind::Pattern(Pattern {
                kind: PatternKind::Points { points: vec![PointRef::Fixed([10., 0., 0.]), PointRef::Fixed([20., 0., 0.])], base: PointRef::Fixed([0., 0., 0.]) },
                sources: vec![b],
                skip: vec![],
            })
        }
        // features 2 e 3: o pino de onde sai a cópia, que cresce
        "espelho" => {
            let b = pino(&mut d, 8., [[-30., -8.], [30., 18.]]);
            FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::YZ), sources: vec![b] })
        }
        "escala" => {
            caixa(&mut d);
            FeatureKind::Scale(Scale { center: PointRef::Origin, factor: s("0.5") })
        }
        "xf_mover" => {
            caixa(&mut d);
            mover(BodyXf::Translate { dir: AxisRef::Origin(OriginAxis::X), dist: s("0") })
        }
        // fora do eixo, para o giro em Z levar a peça para outro lugar
        "xf_girar" => {
            let sk = retangulo(&mut d, xy(), [10., -8.], [40., 8.]);
            d.add_feature(extrusao(sk, BoolOp::Add, Extent::Blind(s("12"))));
            mover(BodyXf::Rotate { axis: AxisRef::Origin(OriginAxis::Z), angle: s("0") })
        }
        // features 2 e 3: o pino, que cresce do lado espelhado
        "xf_simetria" => {
            pino(&mut d, 12., [[4., -8.], [30., 18.]]);
            mover(BodyXf::Symmetry { plane: PlaneRef::Origin(OriginPlane::YZ) })
        }
        "xf_afinidade" => {
            caixa(&mut d);
            mover(BodyXf::Affinity { plane: xy(), fx: s("1"), fy: s("1"), fz: s("1") })
        }
        "pl_boss" | "pl_snap" | "pl_grelha" | "pl_lip" | "pl_ranhura" => {
            // a caixa injetada dos casos de prévia do B148, com a regra plástica ABS
            match demo_plastico::demo(&caso[3..], &mut d)? {
                DemoPrevia::Comando(k) => k,
                _ => return None,
            }
        }
        _ => return None,
    };
    // a mídia mostra só a peça: sem a origem e os rótulos dos planos
    d.origin_visible = false;
    Some((d, kind))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todo_caso_b60_monta_e_recalcula_sem_erro() {
        // junta as falhas de todos os casos numa mensagem só
        let mut falhas = vec![];
        for c in CASOS {
            let (mut d, k) = montar(c).unwrap_or_else(|| panic!("{c} não monta"));
            let id = d.add_feature(k);
            let r = fenkra_solidos::recompute(&d);
            if let Some(e) = r.status.values().find(|s| matches!(s, fenkra_solidos::Status::Error(_))) {
                falhas.push(format!("{c}: {e:?}"));
            } else if !r.status.contains_key(&id) {
                falhas.push(format!("{c}: sem status do comando"));
            } else if r.bodies.iter().map(|b| b.mass.volume).sum::<f64>() <= 1. {
                falhas.push(format!("{c}: sem volume"));
            }
        }
        assert!(falhas.is_empty(), "{falhas:#?}");
        assert!(montar("nao_existe").is_none());
    }
}
