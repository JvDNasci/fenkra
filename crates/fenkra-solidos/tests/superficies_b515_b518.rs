// SPDX-License-Identifier: LGPL-2.1-or-later
//! B515 (subelementos de superfície e curva nos comandos), B517 (filete entre superfícies e junção) e
//! B518 (deslocamento e engrossar de superfícies conectadas). Documento montado em código, recalculado
//! e conferido por status, área ou volume e caixa envolvente.

use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{final_shape, recompute, surface_brep, RecomputeResult, Status};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status[&id], Status::Ok), "{}: {:?}", d.name_of(id), r.status[&id]);
}

fn erro(r: &RecomputeResult, id: Id) -> String {
    match &r.status[&id] {
        Status::Error(m) => m.clone(),
        o => panic!("esperava erro, veio {o:?}"),
    }
}

fn forma(d: &Document, id: Id) -> Shape {
    Shape::from_brep(&surface_brep(d, id).expect("superfície")).unwrap()
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.)
}

fn pt(d: &mut Document, p: [f64; 3]) -> PointRef {
    let _ = d;
    PointRef::Fixed(p)
}

fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    let (a, b) = (pt(d, a), pt(d, b));
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a, b, start: s("0"), end: s("0") }))
}

fn extrude(d: &mut Document, c: CurveRef, eixo: OriginAxis, l: &str) -> Id {
    d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(eixo), length: s(l), length2: s("0"), until: None, until2: None, mais: vec![] })
}

fn sup<'a>(r: &'a RecomputeResult, id: Id) -> &'a fenkra_solidos::SurfaceViz {
    r.surfaces.iter().find(|x| x.id == id).expect("superfície no resultado")
}

fn meio_aresta(m: &fenkra_kernel::Mesh, e: usize) -> [f64; 3] {
    [0, 1, 2].map(|k| m.edge_mids[3 * e + k] as f64)
}

/// Caixa pela malha: a caixa do kernel inclui os polos das B-splines e a tolerância, e sobra até 2 mm
/// numa face deslocada; a malha fica sobre a superfície.
fn caixa_malha(m: &fenkra_kernel::Mesh) -> ([f64; 3], [f64; 3]) {
    let mut a = [f64::MAX; 3];
    let mut b = [f64::MIN; 3];
    for p in m.positions.chunks_exact(3) {
        for k in 0..3 {
            a[k] = a[k].min(p[k] as f64);
            b[k] = b[k].max(p[k] as f64);
        }
    }
    (a, b)
}

fn caixa_sup(r: &RecomputeResult, id: Id) -> ([f64; 3], [f64; 3]) {
    caixa_malha(&sup(r, id).mesh)
}

/// Raio mínimo citado no aviso ("... desse lado (R mm) ...").
fn raio_do_aviso(m: &str) -> f64 {
    let i = m.find("desse lado (").expect("aviso com o raio") + "desse lado (".len();
    m[i..].split(' ').next().unwrap().parse().expect("número")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

// ------------------------------------------------------------------ B515

/// Superfície em L (duas faces planas, canto vivo) e uma curva solta. Face, aresta e vértice da
/// superfície e a curva com o vértice dela entram em comandos seguintes; mudar a superfície de
/// entrada leva todos junto, pela chave (dono + papel), e não pelo ponto clicado.
#[test]
fn b515_subelementos_da_superficie_e_da_curva_nos_comandos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b515");
    let poli = d.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([0., 0., 0.]), PointRef::Fixed([40., 0., 0.]), PointRef::Fixed([40., 30., 0.])], closed: false });
    let l = extrude(&mut d, CurveRef::Ref(poli), OriginAxis::Z, "20");
    let curva = reta(&mut d, [0., 60., 0.], [25., 60., 0.]);
    let CurveRef::Ref(curva_id) = curva else { unreachable!() };
    let r = recompute(&d);
    ok(&r, &d, l);
    let sv = sup(&r, l);
    let m = &sv.mesh;
    assert_eq!(m.face_info.len(), 2, "a polilinha com canto dá duas faces");
    // a face em y = 0 (normal ±Y), a aresta de cima dela (borda), a aresta do canto e um vértice
    let f0 = (0..2).find(|i| m.face_info[*i].dir[1].abs() > 0.99).expect("face em y = 0");
    let fk = sv.face_key(f0).expect("chave da face");
    assert_eq!((fk.owner, fk.role), (l, PAPEL_SUPERFICIE + f0 as i32));
    let topo_e = |alvo: [f64; 3]| (0..m.edge_faces.len()).min_by(|a, b| dist(meio_aresta(m, *a), alvo).total_cmp(&dist(meio_aresta(m, *b), alvo))).unwrap();
    let e_topo = topo_e([20., 0., 20.]);
    let e_canto = topo_e([40., 0., 10.]);
    let (ek_topo, ek_canto) = (sv.edge_key(e_topo).expect("borda tem chave"), sv.edge_key(e_canto).expect("canto tem chave"));
    assert_eq!(ek_topo.a, ek_topo.b, "borda: uma face só");
    assert_ne!(ek_canto.a, ek_canto.b, "canto: entre as duas faces");

    // 1) deslocamento só da face (sólidos e superfícies aceitam a face de uma superfície)
    let face = SurfRef::Face { point: m.face_info[f0].point, normal: m.face_info[f0].dir, owner: Some(l), key: Some(fk) };
    let off = d.add_geometry(RefKind::SurfOffset { surface: face, dist: s("5"), both: false, join: OffsetJoin::Automatico });
    // 2) extrusão da aresta de cima na direção da aresta do canto
    let borda = CurveRef::BodyEdge { pts: vec![[0., 0., 20.], [40., 0., 20.]], key: ek_topo };
    let dir = AxisRef::Edge { mid: [40., 0., 10.], key: Some(ek_canto) };
    let ext = d.add_geometry(RefKind::SurfExtrude { curve: borda, dir, length: s("10"), length2: s("0"), until: None, until2: None, mais: vec![] });
    // 3) plano pela aresta do canto e pelo vértice da ponta da borda
    let vert = PointRef::Vertex { point: [0., 0., 20.], edge: ek_topo };
    let canto = CurveRef::BodyEdge { pts: vec![[40., 0., 0.], [40., 0., 20.]], key: ek_canto };
    let plano = d.add_geometry(RefKind::PlanePointLine { point: vert, line: canto });
    // 4) a curva solta: a aresta dela extrudada, o vértice dela no começo de uma linha
    let ext_c = extrude(&mut d, curva.clone(), OriginAxis::Z, "8");
    let lin = d.add_geometry(RefKind::LinePoints { a: PointRef::CurveVertex { curve: curva_id, index: 0 }, b: vert, start: s("0"), end: s("0") });
    // 5) engrossar a face da superfície (bancada de sólidos)
    let grosso = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: face, t1: s("2"), t2: s("0"), op: BoolOp::Add, simetrico: false, dir: None }));

    let confere = |d: &Document, comp: f64| {
        let r = recompute(d);
        for id in [off, ext, plano, ext_c, lin, grosso] {
            ok(&r, d, id);
        }
        assert!(perto(forma(d, off).mass().area, comp * 20., 1e-3), "face deslocada {}", forma(d, off).mass().area);
        assert!(perto(forma(d, ext).mass().area, comp * 10., 1e-3), "borda extrudada {}", forma(d, ext).mass().area);
        let (a, b) = forma(d, ext).bbox().unwrap();
        // o sentido segue a aresta do canto (para cima ou para baixo, conforme a orientação dela)
        let vertical = (perto(a[2], 20., 1e-3) && perto(b[2], 30., 1e-3)) || (perto(a[2], 10., 1e-3) && perto(b[2], 20., 1e-3));
        assert!(vertical && perto(a[0], 40. - comp, 1e-3) && perto(b[0], 40., 1e-3), "caixa da borda extrudada {a:?} {b:?}");
        assert!(perto(forma(d, ext_c).mass().area, 25. * 8., 1e-3));
        let pl = r.planes.iter().find(|p| p.id == Some(plano)).expect("plano");
        assert!(pl.frame.normal[1].abs() > 0.999, "plano pela aresta do canto e pelo vértice: y = 0, {:?}", pl.frame.normal);
        let c = r.curves.iter().find(|c| c.id == lin).expect("linha");
        assert!(dist(c.pts[0], [0., 60., 0.]) < 1e-6 && dist(*c.pts.last().unwrap(), [40. - comp, 0., 20.]) < 1e-6, "linha do vértice da curva ao vértice da borda {:?}", (c.pts[0], c.pts.last()));
        let v = final_shape(d).expect("sólido").mass().volume;
        assert!(perto(v, comp * 20. * 2., 1e-3), "face engrossada {v}");
    };
    confere(&d, 40.);
    // a primeira perna cresce para trás: a face, a borda e o vértice seguem pela chave
    if let Some(RefKind::Polyline { points, .. }) = d.references.iter_mut().find(|x| x.id == poli).map(|x| &mut x.kind) {
        points[0] = PointRef::Fixed([-20., 0., 0.]);
    }
    confere(&d, 60.);
}

// ------------------------------------------------------------------ B517

/// Base plana e parede vinda de uma curva (spline) que atravessa a base: filete, troca de raio e
/// troca de lado pela seta.
#[test]
fn b517_filete_entre_superficie_de_curva_e_plano() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b517");
    let c_base = reta(&mut d, [0., -50., 0.], [100., -50., 0.]);
    let base = extrude(&mut d, c_base, OriginAxis::Y, "100");
    // parede reta em x = 50, de z = -20 a 40, com a largura da base: o caso com área conhecida (com
    // larguras diferentes o filete corre até a borda da mais larga, e a conta das pontas muda)
    let c_wall = reta(&mut d, [50., -50., -20.], [50., 50., -20.]);
    let wall = extrude(&mut d, c_wall, OriginAxis::Z, "60");
    let fil = d.add_geometry(RefKind::SurfFillet { a: SurfRef::Ref(base), b: SurfRef::Ref(wall), flip_a: false, flip_b: false, radius: s("5"), radius2: None });
    let r = recompute(&d);
    // a seta escolhe o lado: tenta as quatro combinações e fica com a de cima e x < 50
    let mut certo = None;
    for (fa, fb) in [(false, false), (true, false), (false, true), (true, true)] {
        if let Some(RefKind::SurfFillet { flip_a, flip_b, .. }) = d.references.iter_mut().find(|x| x.id == fil).map(|x| &mut x.kind) {
            (*flip_a, *flip_b) = (fa, fb);
        }
        let r = recompute(&d);
        ok(&r, &d, fil);
        let (a, b) = caixa_sup(&r, fil);
        if b[0] < 50. + 1e-3 && a[2] > -1e-3 {
            certo = Some((fa, fb));
        }
    }
    let _ = r;
    let (fa, fb) = certo.expect("uma das combinações fica com x < 50 e z > 0");
    let ajusta = |d: &mut Document, fa: bool, fb: bool, raio: &str| {
        if let Some(RefKind::SurfFillet { flip_a, flip_b, radius, .. }) = d.references.iter_mut().find(|x| x.id == fil).map(|x| &mut x.kind) {
            (*flip_a, *flip_b, *radius) = (fa, fb, s(raio));
        }
    };
    // área: base 50 × 100 + parede 40 × 100, menos 2r × 100 aparados, mais o quarto de cilindro
    let esperado = |r: f64| 9000. - 2. * r * 100. + PI / 2. * r * 100.;
    for raio in [5., 8.] {
        ajusta(&mut d, fa, fb, &raio.to_string());
        let r = recompute(&d);
        ok(&r, &d, fil);
        let f = forma(&d, fil);
        let faces: Vec<f64> = f.topology().unwrap().faces.iter().map(|x| x.area).collect();
        assert!(perto(f.mass().area, esperado(raio), 2e-3), "filete r={raio}: {} != {} (faces {faces:?})", f.mass().area, esperado(raio));
        assert_eq!(f.sew_report().unwrap().domains, 1, "uma superfície só");
        let (a, b) = caixa_sup(&r, fil);
        assert!(perto(a[0], 0., 1e-6) && perto(b[0], 50., 1e-3) && perto(a[2], 0., 1e-3) && perto(b[2], 40., 1e-3), "caixa {a:?} {b:?}");
    }
    // inverter a seta da base: fica o lado x > 50
    ajusta(&mut d, !fa, fb, "5");
    let r = recompute(&d);
    ok(&r, &d, fil);
    let (a, b) = caixa_sup(&r, fil);
    assert!(perto(a[0], 50., 1e-3) && perto(b[0], 100., 1e-6), "lado invertido {a:?} {b:?}");
    assert!(perto(forma(&d, fil).mass().area, esperado(5.), 2e-3));

    // parede de spline que só encosta na base (sem atravessar): o filete sai e a parede fica inteira
    let sp = d.add_geometry(RefKind::Spline { points: vec![PointRef::Fixed([20., -40., 0.]), PointRef::Fixed([35., 0., 0.]), PointRef::Fixed([25., 40., 0.])], closed: false, tangents: vec![] });
    let curva = extrude(&mut d, CurveRef::Ref(sp), OriginAxis::Z, "30");
    let fil2 = d.add_geometry(RefKind::SurfFillet { a: SurfRef::Ref(base), b: SurfRef::Ref(curva), flip_a: false, flip_b: false, radius: s("4"), radius2: None });
    let mut algum = false;
    for (fa, fb) in [(false, false), (true, false), (false, true), (true, true)] {
        if let Some(RefKind::SurfFillet { flip_a, flip_b, .. }) = d.references.iter_mut().find(|x| x.id == fil2).map(|x| &mut x.kind) {
            (*flip_a, *flip_b) = (fa, fb);
        }
        let r = recompute(&d);
        match &r.status[&fil2] {
            Status::Ok => {
                algum = true;
                let f = forma(&d, fil2);
                assert_eq!(f.sew_report().unwrap().domains, 1);
                let (a, b) = caixa_sup(&r, fil2);
                assert!(perto(a[2], 0., 1e-3) && perto(b[2], 30., 1e-3), "caixa do filete com a spline {a:?} {b:?}");
            }
            Status::Error(m) => assert!(m.contains("inverta") || m.contains("raio"), "erro sem orientação: {m}"),
            o => panic!("{o:?}"),
        }
    }
    assert!(algum, "o filete entre a superfície da curva e o plano tem de sair para algum lado");
}

/// Junção de três faixas que se tocam: uma superfície só.
#[test]
fn b517_juncao_de_tres_superficies_resulta_em_uma() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("junção");
    let mut ids = vec![];
    for (a, b) in [([0., 0., 0.], [30., 0., 0.]), ([30., 0., 0.], [30., 20., 0.]), ([30., 20., 0.], [0., 20., 10.])] {
        let c = reta(&mut d, a, b);
        ids.push(extrude(&mut d, c, OriginAxis::Z, "15"));
    }
    let j = d.add_geometry(RefKind::SurfJoin { items: ids.iter().map(|x| SurfRef::Ref(*x)).collect(), tol: s("0.01"), curves: vec![] });
    let r = recompute(&d);
    ok(&r, &d, j);
    let f = forma(&d, j);
    let rep = f.sew_report().unwrap();
    assert_eq!(rep.domains, 1, "três faixas viram uma");
    // a terceira reta sobe 10 mm, mas a faixa é extrudada em Z: conta só o comprimento em planta (30)
    assert!(perto(f.mass().area, (30. + 20. + 30.) * 15., 1e-6), "área {}", f.mass().area);
    let (a, b) = f.bbox().unwrap();
    assert!(perto(b[0], 30., 1e-6) && perto(b[1], 20., 1e-6) && perto(b[2], 25., 1e-6) && perto(a[2], 0., 1e-6));
}

// ------------------------------------------------------------------ B518

/// L do relato: base plana, concordância de raio 5 e parede vertical, em três superfícies juntadas.
/// Devolve (documento, junção).
fn l_com_concordancia() -> (Document, Id) {
    let mut d = Document::new("L");
    let base = reta(&mut d, [0., 0., 0.], [35., 0., 0.]);
    let k = 5. * (PI / 4.).sin();
    let arco = CurveRef::Ref(d.add_geometry(RefKind::Circle3 { a: PointRef::Fixed([35., 0., 0.]), b: PointRef::Fixed([35. + k, 0., 5. - k]), c: PointRef::Fixed([40., 0., 5.]), full: false }));
    let parede = reta(&mut d, [40., 0., 5.], [40., 0., 40.]);
    let ids: Vec<Id> = [base, arco, parede].into_iter().map(|c| extrude(&mut d, c, OriginAxis::Y, "30")).collect();
    let j = d.add_geometry(RefKind::SurfJoin { items: ids.iter().map(|x| SurfRef::Ref(*x)).collect(), tol: s("0.01"), curves: vec![] });
    (d, j)
}

/// Seção do L engrossado de `t` para dentro (lado do centro da concordância) ou para fora.
fn secao_l(t: f64, dentro: bool) -> f64 {
    let r = 5.;
    let arco = if dentro { PI / 4. * (2. * r * t - t * t) } else { PI / 4. * (2. * r * t + t * t) };
    70. * t + arco
}

#[test]
fn b518_engrossar_o_l_fecha_num_solido_valido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d0, j) = l_com_concordancia();
    let r = recompute(&d0);
    ok(&r, &d0, j);
    assert_eq!(forma(&d0, j).sew_report().unwrap().domains, 1);
    // espessura pequena, para cada lado: um sólido válido com o volume da seção vezes a largura
    for (t1, t2) in [("1", "0"), ("0", "1")] {
        let mut d = d0.clone();
        let g = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(j), t1: s(t1), t2: s(t2), op: BoolOp::Add, simetrico: false, dir: None }));
        let r = recompute(&d);
        ok(&r, &d, g);
        let sh = final_shape(&d).expect("sólido");
        assert!(sh.is_valid(), "sólido válido");
        let v = sh.mass().volume;
        let (a, b) = caixa_malha(&r.bodies[0].mesh);
        assert_eq!(r.bodies.len(), 1, "um corpo só");
        let dentro = perto(v, 30. * secao_l(1., true), 5e-3);
        let fora = perto(v, 30. * secao_l(1., false), 5e-3);
        assert!(dentro || fora, "volume {v}: dentro {} fora {}", 30. * secao_l(1., true), 30. * secao_l(1., false));
        // caixa sem sobra: para dentro não passa do L; para fora cresce só a espessura
        if dentro {
            assert!(perto(a[0], 0., 1e-3) && perto(b[0], 40., 1e-3) && perto(a[2], 0., 1e-3) && perto(b[2], 40., 1e-3), "caixa {a:?} {b:?}");
        } else {
            assert!(perto(a[0], 0., 1e-3) && perto(b[0], 41., 1e-3) && perto(a[2], -1., 1e-3) && perto(b[2], 40., 1e-3), "caixa {a:?} {b:?}");
        }
        assert!(perto(a[1], 0., 1e-4) && perto(b[1], 30., 1e-4), "largura {a:?} {b:?}");
    }
    // espessura maior que o raio da concordância: para dentro avisa o raio mínimo; para fora sai válido
    let mut avisos = 0;
    let mut validos = 0;
    for (t1, t2) in [("8", "0"), ("0", "8")] {
        let mut d = d0.clone();
        let g = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(j), t1: s(t1), t2: s(t2), op: BoolOp::Add, simetrico: false, dir: None }));
        let r = recompute(&d);
        match &r.status[&g] {
            Status::Ok => {
                let sh = final_shape(&d).expect("sólido");
                assert!(sh.is_valid());
                assert!(perto(sh.mass().volume, 30. * secao_l(8., false), 5e-3), "para fora {}", sh.mass().volume);
                validos += 1;
            }
            Status::Error(m) => {
                assert!(m.contains("raio de curvatura") && (raio_do_aviso(m) - 5.).abs() < 0.05, "o aviso traz o raio mínimo: {m}");
                avisos += 1;
            }
            o => panic!("{o:?}"),
        }
    }
    assert_eq!((avisos, validos), (1, 1), "um lado avisa o raio, o outro engrossa");
}

#[test]
fn b518_deslocar_casco_com_concordancia_e_com_quina_viva() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // plano e cilindro concordantes
    let (mut d, j) = l_com_concordancia();
    let off = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(j), dist: s("2"), both: false, join: OffsetJoin::Automatico });
    let off2 = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(off), dist: s("1"), both: false, join: OffsetJoin::Automatico });
    let r = recompute(&d);
    ok(&r, &d, off);
    ok(&r, &d, off2);
    let f = forma(&d, off);
    let rep = f.sew_report().unwrap();
    assert_eq!(rep.domains, 1, "deslocada é uma superfície só");
    assert_eq!(rep.free_edges, 8, "arestas livres só na borda (2 pontas + 3 de cada lado)");
    let a = f.mass().area;
    let dentro = 2100. + 30. * PI / 2. * 3.;
    let fora = 2100. + 30. * PI / 2. * 7.;
    assert!(perto(a, dentro, 3e-3) || perto(a, fora, 3e-3), "área {a}: {dentro} ou {fora}");
    let f2 = forma(&d, off2);
    assert_eq!(f2.sew_report().unwrap().domains, 1, "o segundo deslocamento também sai inteiro");
    // deslocamento maior que o raio para dentro: avisa o raio mínimo (para fora, sai)
    let mut d2 = d.clone();
    let big = d2.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(j), dist: s("7"), both: false, join: OffsetJoin::Automatico });
    let big_neg = d2.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(j), dist: s("-7"), both: false, join: OffsetJoin::Automatico });
    let r2 = recompute(&d2);
    let estados = [&r2.status[&big], &r2.status[&big_neg]];
    assert_eq!(estados.iter().filter(|x| matches!(x, Status::Ok)).count(), 1, "{estados:?}");
    let m = if matches!(estados[0], Status::Ok) { erro(&r2, big_neg) } else { erro(&r2, big) };
    assert!(m.contains("raio de curvatura") && (raio_do_aviso(&m) - 5.).abs() < 0.05, "{m}");

    // quina viva: duas faces planas a 90°
    let mut q = Document::new("quina");
    let poli = q.add_geometry(RefKind::Polyline { points: vec![PointRef::Fixed([0., 0., 0.]), PointRef::Fixed([40., 0., 0.]), PointRef::Fixed([40., 0., 30.])], closed: false });
    let l = extrude(&mut q, CurveRef::Ref(poli), OriginAxis::Y, "30");
    let o1 = q.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(l), dist: s("3"), both: false, join: OffsetJoin::Automatico });
    let o2 = q.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(o1), dist: s("2"), both: false, join: OffsetJoin::Automatico });
    let r = recompute(&q);
    ok(&r, &q, o1);
    ok(&r, &q, o2);
    let f = forma(&q, o1);
    assert_eq!(f.sew_report().unwrap().domains, 1, "quina viva: uma superfície só");
    let a = f.mass().area;
    let casos = [30. * (70. - 6.), 30. * (70. + 6.), 30. * (70. + PI / 2. * 3.)];
    assert!(casos.iter().any(|c| perto(a, *c, 2e-3)), "área {a}: {casos:?}");
    assert_eq!(forma(&q, o2).sew_report().unwrap().domains, 1);
}
