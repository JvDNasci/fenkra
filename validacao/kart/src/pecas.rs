// SPDX-License-Identifier: LGPL-2.1-or-later
//! Etapas 4 a 9: chapas, peças de torno, eixo, rodas, direção, transmissão, freio.
//! Peças de giro têm o eixo em Y local, para entrarem na montagem só com translação.
use crate::apoio::*;
use crate::mestre::liga;
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::*;

pub type Pub = [(String, LinkedGeo)];

/// Círculos com a cota de diâmetro por expressão (centro fixo).
pub fn sk_cd(d: &mut Document, plane: PlaneRef, cs: &[(f64, f64, f64, &str)]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    for (x, y, dia, e) in cs {
        let p = data.add_point(&mut al, *x, *y);
        data.add_constraint(&mut al, Kind::Fixed(p));
        let c = data.add_curve(&mut al, Curve::Circle { c: p, r: dia / 2. }, false);
        data.add_constraint(&mut al, Kind::Diameter(c, (*e).into()));
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

pub fn nome(d: &mut Document, f: Id, n: &str) -> Id {
    if let Some(x) = d.feature_mut(f) {
        x.name = n.into();
    }
    f
}

/// Perfil de torno `[r, y]` no plano XY, girado em torno de Y.
pub fn torno(d: &mut Document, pts: &[[f64; 2]]) -> Id {
    let sk = sk_poli(d, oxy(), pts);
    d.add_feature(rev(sk, OriginAxis::Y))
}

pub fn circular(d: &mut Document, n: &str, src: Vec<Id>) -> Id {
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Y), count: n.into(), angle: "360".into() }, sources: src, skip: vec![] }))
}

pub fn cha(valor: &str, mids: Vec<[f64; 3]>) -> FeatureKind {
    let n = mids.len();
    FeatureKind::Chamfer(EdgeOp { value: valor.into(), edges: EdgeSel::Edges(mids), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![None; n], extra: Default::default() })
}

pub fn fil(valor: &str, mids: Vec<[f64; 3]>) -> FeatureKind {
    let n = mids.len();
    FeatureKind::Fillet(EdgeOp { value: valor.into(), edges: EdgeSel::Edges(mids), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![None; n], extra: Default::default() })
}

/// Sketch de pontos num plano XZ deslocado (pontos em x, z) e furo passante.
fn furos_xz(d: &mut Document, off: &str, pts: &[[f64; 2]], dia: &str) -> Id {
    let pl = d.add_offset_plane(oxz(), off);
    let sk = sk_pts(d, PlaneRef::Datum(pl), pts);
    d.add_feature(furo(sk, dia))
}

/// O que o clique faria: troca cada ponto pelo ponto médio da aresta mais perto dele.
pub fn pc(d: &mut Document, mut k: FeatureKind) -> Id {
    let r = fenkra_solidos::recompute(d);
    if let FeatureKind::Chamfer(e) | FeatureKind::Fillet(e) = &mut k {
        if let EdgeSel::Edges(mids) = &mut e.edges {
            if let Some(b) = r.bodies.first() {
                let m = &b.mesh;
                let n = m.edge_mids.len() / 3;
                for p in mids.iter_mut() {
                    let mut melhor = (f64::MAX, 0);
                    for i in 0..n {
                        let a = m.edge_ranges[i] as usize;
                        let z = m.edge_ranges.get(i + 1).map(|x| *x as usize).unwrap_or(m.edges.len() / 6);
                        for s in a * 2..z * 2 {
                            let q = &m.edges[s * 3..s * 3 + 3];
                            let dd = (0..3).map(|j| (q[j] as f64 - p[j]).powi(2)).sum::<f64>();
                            if dd < melhor.0 {
                                melhor = (dd, i);
                            }
                        }
                    }
                    *p = [0, 1, 2].map(|j| m.edge_mids[melhor.1 * 3 + j] as f64);
                }
            }
        }
    }
    d.add_feature(k)
}

// ------------------------------------------------------------------ chapa

pub fn chapa(d: &mut Document, t: &str) {
    d.sheet = Some(SheetParams { thickness: t.into(), radius: "2".into(), k: "0.44".into(), flat: false, ..Default::default() });
}

pub fn parede(d: &mut Document, sk: Id) -> Id {
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }))
}

pub fn aba(d: &mut Document, e: [f64; 3], len: &str, ang: &str) -> Id {
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: e, length: len.into(), angle: ang.into(), radius: None, off1: "0".into(), off2: "0".into(), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }))
}

/// CH-08: cadeira do mancal, base com duas abas e furo central.
pub fn ch08(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_EIXO"], &[])?;
    chapa(&mut d, "4");
    let sk = sk_rect(&mut d, oxy(), [-70., -45.], [70., 45.]);
    parede(&mut d, sk);
    aba(&mut d, [70., 0., 4.], "40", "90");
    aba(&mut d, [-70., 0., 4.], "40", "90");
    let p = d.add_offset_plane(oxy(), "4");
    let c = sk_pts(&mut d, PlaneRef::Datum(p), &[[0., 0.]]);
    d.add_feature(furo(c, "mestre_D_EIXO+30"));
    let f = sk_pts(&mut d, PlaneRef::Datum(p), &[[-50., -30.], [50., -30.], [-50., 30.], [50., 30.]]);
    d.add_feature(furo(f, "8.5"));
    Ok(d)
}

/// CH-09: suporte em C do pino-mestre, sólido.
pub fn ch09(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["CASTER", "D_TUBO"], &[])?;
    // perfil em C no plano XZ, extrudado em Y
    let sk = sk_poli(&mut d, oxz(), &[[0., -35.], [40., -35.], [40., -23.], [12., -23.], [12., 23.], [40., 23.], [40., 35.], [0., 35.]]);
    d.add_feature(ext(sk, BoolOp::Add, sim("30")));
    // furos do pino nos dois braços: plano XY, furo Ø10 passante
    let c = sk_cd(&mut d, oxy(), &[(28., 0., 10., "10")]);
    nome(&mut d, c, "Pino");
    let cut = d.add_feature(ext(c, BoolOp::Cut, Extent::ThroughAll));
    nome(&mut d, cut, "Furo do pino");
    pc(&mut d, fil("6", vec![[0., 0., 35.], [0., 0., -35.]]));
    pc(&mut d, cha("1", vec![[40., 0., 35.], [40., 0., -35.]]));
    Ok(d)
}

/// CH-10: soquete de para-choque, estrutural.
pub fn ch10(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_PC_INF", "EE"], &[])?;
    let a = ponto(&mut d, "A", "mestre_EE+60", "0", "60");
    let b = ponto(&mut d, "B", "mestre_EE+60+50", "0", "60");
    let c = polilinha(&mut d, "Soquete", &[a, b]);
    d.add_feature(FeatureKind::StructMember(tubo(vec![CurveRef::Ref(c)], "mestre_D_PC_INF+4", "2", "Ø24x2")));
    Ok(d)
}

/// CH-11: apoio do motor sobre os tubos, em chapa, com dobras e rasgos oblongos.
pub fn ch11(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    chapa(&mut d, "5");
    let sk = sk_rect(&mut d, oxy(), [-90., -60.], [90., 60.]);
    parede(&mut d, sk);
    aba(&mut d, [0., 60., 5.], "25", "90");
    aba(&mut d, [0., -60., 5.], "25", "90");
    let p = d.add_offset_plane(oxy(), "5");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        for (x, y) in [(-60., -35.), (-60., 35.), (60., -35.), (60., 35.)] {
            data.add_slot(&mut al, [x - 12., y], [x + 12., y], 4.25);
        }
    }
    let s2 = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(p), data }));
    d.add_feature(FeatureKind::SmCut(SmCut { sketch: s2, mode: 0, offset: "0".into(), draft: "0".into(), height: "0".into(), flip: false, normal_face: true }));
    Ok(d)
}

/// CH-12: suporte da coluna: orelha com aba e furo.
pub fn ch12(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_COLUNA"], &[])?;
    chapa(&mut d, "3");
    let sk = sk_rect(&mut d, oxy(), [-30., 0.], [30., 70.]);
    parede(&mut d, sk);
    aba(&mut d, [0., 0., 3.], "30", "90");
    let p = d.add_offset_plane(oxy(), "3");
    let c = sk_pts(&mut d, PlaneRef::Datum(p), &[[0., 45.]]);
    d.add_feature(furo(c, "mestre_D_COLUNA+6"));
    pc(&mut d, fil("10", vec![[30., 70., 1.5], [-30., 70., 1.5]]));
    Ok(d)
}

/// CH-13: orelha de pedal, placa com furo.
pub fn ch13(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    chapa(&mut d, "3");
    let sk = sk_rect(&mut d, oxy(), [-20., 0.], [20., 45.]);
    parede(&mut d, sk);
    aba(&mut d, [0., 0., 3.], "15", "90");
    let p = d.add_offset_plane(oxy(), "3");
    let c = sk_pts(&mut d, PlaneRef::Datum(p), &[[0., 30.]]);
    d.add_feature(furo(c, "8.5"));
    Ok(d)
}

/// CH-14: suporte da pinça: em L com dois furos.
pub fn ch14(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    chapa(&mut d, "5");
    let sk = sk_rect(&mut d, oxy(), [-45., 0.], [45., 60.]);
    parede(&mut d, sk);
    aba(&mut d, [0., 60., 5.], "40", "90");
    let p = d.add_offset_plane(oxy(), "5");
    let c = sk_pts(&mut d, PlaneRef::Datum(p), &[[-30., 25.], [30., 25.]]);
    d.add_feature(furo(c, "8.5"));
    Ok(d)
}

// ------------------------------------------------------------------ eixo

pub fn ex01(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_EIXO", "E_EIXO", "L_EIXO"], &[])?;
    let sk = sk_cd(&mut d, oxz(), &[(0., 0., 50., "mestre_D_EIXO"), (0., 0., 45., "mestre_D_EIXO-2*mestre_E_EIXO")]);
    let e = d.add_feature(ext(sk, BoolOp::Add, sim("mestre_L_EIXO")));
    nome(&mut d, e, "Tubo do eixo");
    let pl = d.add_offset_plane(oyz(), "mestre_D_EIXO/2");
    // YZ: u = Y, v = Z. Rasgo do cubo perto da ponta
    let r = sk_rect(&mut d, PlaneRef::Datum(pl), [460., -4.], [510., 4.]);
    let k = ext(r, BoolOp::Cut, blind("4"));
    let c1 = d.add_feature(k.clone());
    nome(&mut d, c1, "Rasgo do cubo");
    d.add_feature(FeatureKind::Mirror(Mirror { plane: oxz(), sources: vec![c1] }));
    let r2 = sk_rect(&mut d, PlaneRef::Datum(pl), [-120., -4.], [-80., 4.]);
    let mut k2 = k.clone();
    if let FeatureKind::Extrude(x) = &mut k2 {
        x.sketch = r2;
    }
    let c2 = d.add_feature(k2);
    nome(&mut d, c2, "Rasgo da coroa");
    let r3 = sk_rect(&mut d, PlaneRef::Datum(pl), [160., -4.], [200., 4.]);
    let mut k3 = k.clone();
    if let FeatureKind::Extrude(x) = &mut k3 {
        x.sketch = r3;
    }
    let c3 = d.add_feature(k3);
    nome(&mut d, c3, "Rasgo do disco");
    pc(&mut d, cha("1.5", vec![[25., 520., 0.], [-25., 520., 0.], [25., -520., 0.], [-25., -520., 0.]]));
    Ok(d)
}

pub fn ex02(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_EIXO"], &[])?;
    torno(&mut d, &[[40., -14.], [62., -14.], [62., 14.], [40., 14.]]);
    let sk = sk_circ(&mut d, oxz(), &[([0., 0.], 80.), ([0., 0.], 41.)]);
    d.add_feature(ext(sk, BoolOp::Add, sim("10")));
    let f = furos_xz(&mut d, "0", &[[68., 0.]], "8.5");
    circular(&mut d, "3", vec![f]);
    Ok(d)
}

/// EX-05: cubo traseiro: furo, flange, furos em padrão, rasgo e chanfros.
pub fn ex05(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_EIXO"], &[])?;
    torno(&mut d, &[[25., 0.], [42., 0.], [42., 60.], [70., 60.], [70., 72.], [25., 72.]]);
    let f = furos_xz(&mut d, "-72", &[[56., 0.]], "8.5");
    circular(&mut d, "4", vec![f]);
    let sk = sk_rect(&mut d, oxz(), [-4., 20.], [4., 29.]);
    let e = d.add_feature(ext(sk, BoolOp::Cut, Extent::ThroughAll));
    nome(&mut d, e, "Rasgo");
    pc(&mut d, cha("1", vec![[42., 0., 0.]]));
    pc(&mut d, fil("3", vec![[42., 60., 0.]]));
    Ok(d)
}

/// EX-06 e EX-07: porta-coroa e porta-disco.
pub fn porta(m: &Pub, r_flange: f64, furos: u32) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_EIXO"], &[])?;
    torno(&mut d, &[[25., 0.], [38., 0.], [38., 30.], [r_flange, 30.], [r_flange, 38.], [25., 38.]]);
    let f = furos_xz(&mut d, "-38", &[[r_flange - 10., 0.]], "6.5");
    circular(&mut d, &furos.to_string(), vec![f]);
    let sk = sk_rect(&mut d, oxz(), [-4., 20.], [4., 29.]);
    d.add_feature(ext(sk, BoolOp::Cut, Extent::ThroughAll));
    Ok(d)
}

pub fn ex08(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_DISCO", "E_DISCO"], &[])?;
    let sk = sk_cd(&mut d, oxz(), &[(0., 0., 200., "mestre_D_DISCO"), (0., 0., 90., "90")]);
    d.add_feature(ext(sk, BoolOp::Add, sim("mestre_E_DISCO")));
    let f = furos_xz(&mut d, "0", &[[55., 0.]], "6.5");
    circular(&mut d, "6", vec![f]);
    let v = furos_xz(&mut d, "0", &[[80., 0.]], "7");
    circular(&mut d, "24", vec![v]);
    pc(&mut d, cha("0.5", vec![[100., 3., 0.], [100., -3., 0.]]));
    Ok(d)
}

// ------------------------------------------------------------------ rodas

/// Aro de largura `l`: assento Ø126,2, borda Ø136,2, alma com furos, três pinos de talão.
pub fn aro(l: f64) -> Result<Document, String> {
    let mut d = Document::new("c");
    let h = l / 2.;
    torno(&mut d, &[[58., -h], [68.1, -h], [68.1, -h + 5.], [63.1, -h + 8.], [63.1, h - 8.], [68.1, h - 5.], [68.1, h], [58., h]]);
    let sk = sk_circ(&mut d, oxz(), &[([0., 0.], 59.), ([0., 0.], 27.)]);
    d.add_feature(ext(sk, BoolOp::Add, sim("8")));
    let f = furos_xz(&mut d, "0", &[[42., 0.]], "8.5");
    circular(&mut d, "4", vec![f]);
    // pinos de retenção do talão, do lado de fora do aro, radiais (direção X). O sketch fica num
    // plano normal a X, na superfície do aro: no plano XY com direção X (como era) o perfil varria a
    // si mesmo e o aro saía inválido, com interferência falsa na montagem (K-10, B388)
    let pl = d.add_offset_plane(oyz(), "63.1");
    let p = sk_circ(&mut d, PlaneRef::Datum(pl), &[([-h + 12., 0.], 1.5)]);
    let pino = d.add_feature(ext(p, BoolOp::Add, sim("6")));
    nome(&mut d, pino, "Pino do talão");
    circular(&mut d, "3", vec![pino]);
    Ok(d)
}

/// Pneu: perfil em spline girado (superfície) e engrossado.
pub fn pneu(m: &Pub, diam: &str, larg: &str) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &[diam, larg], &[])?;
    let (r, h) = (format!("mestre_{diam}/2"), format!("mestre_{larg}/2"));
    // X = raio, Y = axial
    let pts: Vec<Id> = [("66", "0.92"), ("(66+R)/2", "1.0"), ("R-8", "0.96"), ("R", "0.7"), ("R", "0"), ("R", "-0.7"), ("R-8", "-0.96"), ("(66+R)/2", "-1.0"), ("66", "-0.92")]
        .iter()
        .enumerate()
        .map(|(i, (x, y))| ponto(&mut d, &format!("P{i}"), &x.replace('R', &format!("({r})")), &format!("{y}*({h})"), "0"))
        .collect();
    let c = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Ref(*p)).collect(), closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfRevolve { curve: CurveRef::Ref(c), axis: AxisRef::Origin(OriginAxis::Y), angle: "360".into(), angle2: "0".into() });
    d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(sup), t1: "9".into(), t2: "0".into(), op: BoolOp::Add }));
    Ok(d)
}

// ------------------------------------------------------------------ direção

pub fn di02(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_polygon(&mut al, [0., 0.], 30., 6, 0.);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: oxy(), data }));
    d.add_feature(ext(sk, BoolOp::Add, blind("14")));
    let t = d.add_offset_plane(oxy(), "14");
    let c = sk_pts(&mut d, PlaneRef::Datum(t), &[[0., 0.]]);
    d.add_feature(furo(c, "20.2"));
    let f = sk_pts(&mut d, PlaneRef::Datum(t), &[[20., 0.]]);
    let h = d.add_feature(furo(f, "6.5"));
    d.add_feature(FeatureKind::Pattern(Pattern { kind: PatternKind::Circular { axis: AxisRef::Origin(OriginAxis::Z), count: "3".into(), angle: "360".into() }, sources: vec![h], skip: vec![] }));
    Ok(d)
}

pub fn di03(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_COLUNA", "E_COLUNA"], &[])?;
    let a = ponto(&mut d, "A", "0", "0", "0");
    let b = ponto(&mut d, "B", "0", "0", "560");
    let c = polilinha(&mut d, "Coluna", &[a, b]);
    d.add_feature(FeatureKind::StructMember(tubo(vec![CurveRef::Ref(c)], "mestre_D_COLUNA", "mestre_E_COLUNA", "Ø20x2")));
    Ok(d)
}

pub fn di04(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    torno(&mut d, &[[10.2, 0.], [20., 0.], [20., 6.], [30., 6.], [30., 14.], [10.2, 14.]]);
    let f = furos_xz(&mut d, "-14", &[[25., 0.]], "6.5");
    circular(&mut d, "2", vec![f]);
    Ok(d)
}

pub fn di05(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    chapa(&mut d, "5");
    let sk = sk_poli(&mut d, oxy(), &[[-60., -60.], [60., -60.], [60., -30.], [25., 30.], [-25., 30.], [-60., -30.]]);
    parede(&mut d, sk);
    let p = d.add_offset_plane(oxy(), "5");
    let c = sk_pts(&mut d, PlaneRef::Datum(p), &[[0., 10.]]);
    d.add_feature(furo(c, "20.2"));
    let f = sk_pts(&mut d, PlaneRef::Datum(p), &[[-45., -45.], [45., -45.]]);
    d.add_feature(furo(f, "8.2"));
    Ok(d)
}

pub fn di06(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["PM"], &[])?;
    let a = ponto(&mut d, "A", "0", "0", "0");
    let b = ponto(&mut d, "B", "0", "mestre_PM/2-60", "0");
    let c = polilinha(&mut d, "Tirante", &[a, b]);
    let mut t = tubo(vec![CurveRef::Ref(c)], "10", "0", "Ø10");
    t.kind = SectionKind::BarraRedonda;
    t.dims = vec!["10".into()];
    d.add_feature(FeatureKind::StructMember(t));
    Ok(d)
}

/// DI-08: manga de eixo: ponta Ø25, olhal do pino, braço, com booleana entre coletores e espelho.
pub fn di08(m: &Pub, espelhar: bool) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["CASTER", "PM", "BD"], &[])?;
    // ponta de eixo em Y, do olhal para fora
    torno(&mut d, &[[0., 0.], [12.5, 0.], [12.5, 70.], [10., 72.], [0., 72.]]);
    // olhal do pino-mestre: vertical em Z
    let o = sk_circ(&mut d, oxy(), &[([0., 0.], 16.)]);
    d.add_feature(ext(o, BoolOp::Add, sim("44")));
    // braço de direção num segundo coletor, unido depois
    let col = d.add_collector("Braco");
    let principal = d.active_collector;
    d.active_collector = col;
    let b = sk_poli(&mut d, oxy(), &[[-12., -12.], [-85., -40.], [-95., -30.], [-16., 8.]]);
    d.add_feature(ext(b, BoolOp::Add, sim("10")));
    d.active_collector = principal;
    d.add_feature(FeatureKind::Combine(Combine { tool: col, op: BoolOp::Add, keep_tool: false }));
    let f = sk_cd(&mut d, oxy(), &[(0., 0., 10., "10"), (-90., -35., 8.2, "8.2")]);
    d.add_feature(ext(f, BoolOp::Cut, Extent::ThroughAll));
    if espelhar {
        d.add_feature(FeatureKind::MoveBody(MoveBody { xf: BodyXf::Symmetry { plane: oxz() } }));
    }
    Ok(d)
}

pub fn di09(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    torno(&mut d, &[[0., -44.], [8., -44.], [8., -40.], [5., -40.], [5., 40.], [4., 44.], [0., 44.]]);
    Ok(d)
}

pub fn di10(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    torno(&mut d, &[[12.5, 0.], [28., 0.], [28., 40.], [52., 40.], [52., 48.], [12.5, 48.]]);
    let f = furos_xz(&mut d, "-48", &[[42., 0.]], "8.5");
    circular(&mut d, "4", vec![f]);
    Ok(d)
}

// ------------------------------------------------------------------ transmissão (roda e corrente à mão: 219 fora da biblioteca)

/// Coroa ou pinhão por dente recortado em padrão circular: o contorno da corrente 219.
pub fn roda_manual(m: &Pub, dp: &str, n: &str, furo_d: f64) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &[dp, n, "PASSO"], &[])?;
    let dpv = match m.iter().find(|(p, _)| p == dp) {
        Some((_, LinkedGeo::Value(v))) => *v,
        _ => return Err("sem primitivo".into()),
    };
    let sk = sk_cd(&mut d, oxz(), &[(0., 0., dpv + 6., &format!("mestre_{dp}+6")), (0., 0., furo_d, &format!("{furo_d}"))]);
    d.add_feature(ext(sk, BoolOp::Add, sim("4.5")));
    let g = sk_cd(&mut d, oxz(), &[(dpv / 2., 0., 4.9, "4.9")]);
    let c = d.add_feature(ext(g, BoolOp::Cut, Extent::ThroughAll));
    nome(&mut d, c, "Assento do rolo");
    let p = circular(&mut d, &format!("mestre_{n}"), vec![c]);
    nome(&mut d, p, "Dentes");
    Ok(d)
}

/// Corrente 106 elos à mão: rolos e placas por pontos do caminho.
pub fn corrente_manual(m: &Pub) -> Result<Document, String> {
    use fenkra_solidos::transmissao_corrente::Caminho;
    let v = |n: &str| match m.iter().find(|(p, _)| p == n) {
        Some((_, LinkedGeo::Value(x))) => *x,
        _ => 0.,
    };
    let (r1, r2, c) = (v("DP_PINHAO") / 2., v("DP_COROA") / 2., v("DC"));
    let cam = Caminho::novo(r1, r2, c)?;
    let elos = v("ELOS") as usize;
    println!("corrente: caminho {:.2} mm, {:.2} passos", cam.comprimento(), cam.comprimento() / v("PASSO"));
    let mut d = Document::new("c");
    let pts = cam.elos(elos, 0.);
    let cs: Vec<([f64; 2], f64)> = pts.iter().map(|p| ([p.x, p.y], 2.45)).collect();
    let sk = sk_circ(&mut d, oxy(), &cs);
    d.add_feature(ext(sk, BoolOp::Add, sim("4.8")));
    Ok(d)
}

pub fn tr04(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    chapa(&mut d, "1.5");
    let sk = sk_rect(&mut d, oxy(), [-140., 0.], [150., 40.]);
    parede(&mut d, sk);
    aba(&mut d, [0., 0., 1.5], "118", "90");
    aba(&mut d, [0., 40., 1.5], "118", "90");
    Ok(d)
}

// ------------------------------------------------------------------ motor e freio

pub fn mo01(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["DC"], &[])?;
    let b = sk_rect(&mut d, oxy(), [-130., -90.], [130., 90.]);
    d.add_feature(ext(b, BoolOp::Add, blind("150")));
    let top = d.add_offset_plane(oxy(), "150");
    let c = sk_circ(&mut d, PlaneRef::Datum(top), &[([0., 0.], 55.)]);
    d.add_feature(ext(c, BoolOp::Add, blind("130")));
    pc(&mut d, fil("8", vec![[130., 90., 75.], [-130., 90., 75.], [130., -90., 75.], [-130., -90., 75.]]));
    // ponta de eixo, para o lado do pinhão (Y)
    let pl = d.add_offset_plane(oxz(), "90");
    let e = sk_circ(&mut d, PlaneRef::Datum(pl), &[([0., 75.], 11.)]);
    d.add_feature(ext(e, BoolOp::Add, blind("45")));
    Ok(d)
}

pub fn mo02(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    let sk = sk_poli(&mut d, oxz(), &[[-110., 0.], [110., 0.], [110., 14.], [70., 14.], [50., 40.], [-50., 40.], [-70., 14.], [-110., 14.]]);
    d.add_feature(ext(sk, BoolOp::Add, sim("120")));
    let f = sk_pts(&mut d, oxy(), &[[-90., -40.], [-90., 40.], [90., -40.], [90., 40.]]);
    d.add_feature(furo(f, "8.5"));
    d.add_feature(FeatureKind::Draft(Draft { faces: vec![[60., 60., 27.]], neutral: oxy(), angle: "5".into(), reverse: false, face_keys: vec![] }));
    Ok(d)
}

pub fn ex09(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    let b = sk_rect(&mut d, oxy(), [-45., -32.], [45., 32.]);
    d.add_feature(ext(b, BoolOp::Add, blind("50")));
    // garganta do disco
    let g = sk_rect(&mut d, oxz(), [-50., 18.], [50., 32.]);
    d.add_feature(ext(g, BoolOp::Cut, Extent::ThroughAll));
    // cilindros
    let t = d.add_offset_plane(oxy(), "50");
    let c = sk_circ(&mut d, PlaneRef::Datum(t), &[([-22., 0.], 14.), ([22., 0.], 14.)]);
    d.add_feature(ext(c, BoolOp::Cut, blind("24")));
    let f = sk_pts(&mut d, oxy(), &[[-38., 24.], [38., 24.]]);
    d.add_feature(furo(f, "8.5"));
    pc(&mut d, fil("4", vec![[45., 32., 8.], [-45., 32., 8.], [45., -32., 8.], [-45., -32., 8.]]));
    Ok(d)
}

pub fn ex10(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        let c = data.add_point(&mut al, 0., 0.);
        let a0 = data.add_point(&mut al, 70., -22.);
        let a1 = data.add_point(&mut al, 70., 22.);
        let b1 = data.add_point(&mut al, 96., 28.);
        let b0 = data.add_point(&mut al, 96., -28.);
        data.add_curve(&mut al, Curve::Line { a: a0, b: b0 }, false);
        data.add_curve(&mut al, Curve::Line { a: a1, b: b1 }, false);
        let (r0, r1) = ((70f64.powi(2) + 22f64.powi(2)).sqrt(), (96f64.powi(2) + 28f64.powi(2)).sqrt());
        let _ = (r0, r1);
        data.add_curve(&mut al, Curve::Line { a: b0, b: b1 }, false);
        data.add_curve(&mut al, Curve::Arc { c, s: a0, e: a1 }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: oxy(), data }));
    d.add_feature(ext(sk, BoolOp::Add, blind("9")));
    Ok(d)
}

pub fn ex11(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    torno(&mut d, &[[0., 0.], [12., 0.], [12., 70.], [16., 70.], [16., 78.], [0., 78.]]);
    let c = sk_circ(&mut d, oxz(), &[([0., 0.], 7.)]);
    d.add_feature(ext(c, BoolOp::Cut, blind("60")));
    let o = sk_rect(&mut d, oxy(), [-30., 60.], [30., 72.]);
    d.add_feature(ext(o, BoolOp::Add, sim("10")));
    d.add_feature(FeatureKind::Thread(Thread { face: [12., 30., 0.], spec: "M24x3".into(), length: "40".into(), modeled: false, end: None, start: String::new() }));
    Ok(d)
}

/// EX-12: varredura de tubo por spline 3D.
pub fn ex12(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["EE"], &[])?;
    let pts: Vec<Id> = [("600", "-120", "60"), ("420", "-300", "60"), ("260", "-330", "90"), ("120", "-280", "120"), ("60", "-160", "110")].iter().enumerate().map(|(i, (x, y, z))| ponto(&mut d, &format!("L{i}"), &format!("{x}*mestre_EE/1040"), y, z)).collect();
    let c = d.add_geometry(RefKind::Spline { points: pts.iter().map(|p| PointRef::Ref(*p)).collect(), closed: false, tangents: vec![] });
    let sup = d.add_geometry(RefKind::SurfSweep { profile: SweepProfile::Circle("2.5".into()), spine: CurveRef::Ref(c), mode: SweepMode::Corrigido });
    d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(sup), t1: "1".into(), t2: "0".into(), op: BoolOp::Add }));
    Ok(d)
}

// ------------------------------------------------------------------ posto

pub fn po02(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["E_REFORCO"], &[])?;
    d.sheet = Some(SheetParams { thickness: "mestre_E_REFORCO".into(), radius: "2".into(), k: "0.44".into(), ..Default::default() });
    let sk = sk_circ(&mut d, oxy(), &[([0., 0.], 25.)]);
    parede(&mut d, sk);
    let t = d.add_offset_plane(oxy(), "mestre_E_REFORCO");
    let c = sk_pts(&mut d, PlaneRef::Datum(t), &[[0., 0.]]);
    d.add_feature(furo(c, "8.5"));
    Ok(d)
}

pub fn po04(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["E_ASSOALHO", "D_FURO_ASS", "LARG_Q"], &[])?;
    d.sheet = Some(SheetParams { thickness: "mestre_E_ASSOALHO".into(), radius: "2".into(), k: "0.44".into(), ..Default::default() });
    let sk = sk_poli(&mut d, oxy(), &[[0., -300.], [420., -300.], [640., -240.], [640., 240.], [420., 300.], [0., 300.]]);
    parede(&mut d, sk);
    aba(&mut d, [320., -300., 1.5], "15", "90");
    aba(&mut d, [320., 300., 1.5], "15", "90");
    let t = d.add_offset_plane(oxy(), "mestre_E_ASSOALHO");
    let c = sk_pts(&mut d, PlaneRef::Datum(t), &[[60., -240.]]);
    let h = d.add_feature(furo(c, "mestre_D_FURO_ASS"));
    d.add_feature(FeatureKind::Pattern(Pattern {
        kind: PatternKind::Linear { dir: AxisRef::Origin(OriginAxis::X), count: "6".into(), spacing: "80".into(), dir2: Some(AxisRef::Origin(OriginAxis::Y)), count2: "7".into(), spacing2: "80".into(), total: false },
        sources: vec![h],
        skip: vec![],
    }));
    Ok(d)
}

/// PO-05 e PO-06: pedal em chapa: placa, aba de pé, dobra por linha e furos.
pub fn pedal(largura: f64) -> Result<Document, String> {
    let mut d = Document::new("c");
    chapa(&mut d, "4");
    let h = largura / 2.;
    let sk = sk_poli(&mut d, oxy(), &[[-15., 0.], [15., 0.], [15., 120.], [h, 150.], [h, 220.], [-h, 220.], [-h, 150.], [-15., 120.]]);
    parede(&mut d, sk);
    aba(&mut d, [0., 220., 4.], "20", "75");
    let t = d.add_offset_plane(oxy(), "4");
    let c = sk_pts(&mut d, PlaneRef::Datum(t), &[[0., 14.], [0., 60.]]);
    d.add_feature(furo(c, "8.5"));
    Ok(d)
}

pub fn barra(n: &str, a: [&str; 3], b: [&str; 3], dia: &str) -> Result<Document, String> {
    let mut d = Document::new("c");
    let p = ponto(&mut d, "A", a[0], a[1], a[2]);
    let q = ponto(&mut d, "B", b[0], b[1], b[2]);
    let c = polilinha(&mut d, n, &[p, q]);
    let mut t = tubo(vec![CurveRef::Ref(c)], dia, "0", "barra");
    t.kind = SectionKind::BarraRedonda;
    t.dims = vec![dia.into()];
    d.add_feature(FeatureKind::StructMember(t));
    Ok(d)
}

/// PO-03: escora do banco.
pub fn po03(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["LARG_Q"], &[])?;
    let a = ponto(&mut d, "A", "300", "mestre_LARG_Q/2", "230");
    let b = ponto(&mut d, "B", "120", "mestre_LARG_Q/2-120", "430");
    let c = polilinha(&mut d, "Escora", &[a, b]);
    d.add_feature(FeatureKind::StructMember(tubo(vec![CurveRef::Ref(c)], "18", "1.5", "Ø18x1,5")));
    Ok(d)
}

/// PO-08: tanque: multi-seções (loft) de retângulos, casca e bocal.
pub fn po08(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    let s0 = sk_rect(&mut d, oxy(), [-110., -130.], [110., 130.]);
    let p1 = d.add_offset_plane(oxy(), "100");
    let s1 = sk_rect(&mut d, PlaneRef::Datum(p1), [-125., -145.], [125., 145.]);
    let p2 = d.add_offset_plane(oxy(), "190");
    let s2 = sk_rect(&mut d, PlaneRef::Datum(p2), [-90., -110.], [90., 110.]);
    d.add_feature(FeatureKind::Loft(Loft { sections: vec![s0, s1, s2], ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![], thickness: "2.5".into(), outward: false, face_keys: vec![], closed: true }));
    let c = sk_circ(&mut d, PlaneRef::Datum(p2), &[([0., 0.], 22.)]);
    d.add_feature(ext(c, BoolOp::Add, blind("25")));
    Ok(d)
}

// ------------------------------------------------------------------ motor, escape

/// MO-03: escape: multi-seções circulares em planos que acompanham uma curva.
pub fn mo03(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    let mut secs = vec![];
    for (z, r) in [(0., 18.), (120., 22.), (260., 52.), (420., 58.), (560., 30.), (640., 12.)] {
        let p = d.add_offset_plane(oxy(), &format!("{z}"));
        secs.push(sk_circ(&mut d, PlaneRef::Datum(p), &[([z * 0.1, 0.], r)]));
    }
    d.add_feature(FeatureKind::Loft(Loft { sections: secs, ruled: false, closed: false, op: BoolOp::Add, torcao: vec![], inicio: Default::default(), fim: Default::default() }));
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 0.], [64., 0., 640.]], thickness: "1.5".into(), outward: false, face_keys: vec![], closed: false }));
    Ok(d)
}

/// Placa de biblioteca que falta: revolução de marcação.
pub fn rolamento(di: f64, de: f64, l: f64) -> Result<Document, String> {
    let mut d = Document::new("c");
    let (ri, re, h) = (di / 2., de / 2., l / 2.);
    torno(&mut d, &[[ri, -h], [re, -h], [re, h], [ri, h]]);
    Ok(d)
}
