// SPDX-License-Identifier: LGPL-2.1-or-later
//! B511: a forma livre convertida sai como superfície comum (casco aberto) ou sólido comum e é usada
//! pelas outras bancadas: deslocar, espessar, extrudar até a superfície e furar. Editar um nó da gaiola
//! refaz tudo o que vem depois. Documento montado em código; status, área ou volume e caixa conferidos.

mod common;

use common::*;
use fenkra_core::subd::Gaiola;
use fenkra_core::*;
use fenkra_solidos::{recompute, regioes::area_da_malha, RecomputeResult, Status};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok | Status::Warning(_))), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

/// Folha ondulada de 100 × 100 mm em z = 30: a gaiola aberta vira superfície.
fn folha() -> Gaiola {
    let mut g = Gaiola::plano([0., 0., 30.], [100., 100.], [3, 3]);
    for n in &mut g.nos {
        n.p[2] += (n.p[0] * 0.03).sin() * 6.;
    }
    g
}

fn forma(g: Gaiola) -> FeatureKind {
    FeatureKind::Forma(Forma { gaiola: g, niveis: 3, congelada: false, conversao: Some(ConversaoForma { continuidade: 1, tol: s("0.01"), pausada: false }), vinculos: Default::default() })
}

fn malha_da_sup(r: &RecomputeResult, id: Id) -> &fenkra_kernel::Mesh {
    &r.surfaces.iter().find(|x| x.id == id).unwrap_or_else(|| panic!("superfície {id:?} no resultado")).mesh
}

fn caixa(m: &fenkra_kernel::Mesh) -> ([f64; 3], [f64; 3]) {
    let (mut a, mut b) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for k in 0..3 {
            a[k] = a[k].min(p[k] as f64);
            b[k] = b[k].max(p[k] as f64);
        }
    }
    (a, b)
}

/// Ponto da malha mais perto de (x, y) na vista de cima.
fn ponto_em(m: &fenkra_kernel::Mesh, x: f64, y: f64) -> [f64; 3] {
    m.positions
        .chunks_exact(3)
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .min_by(|a, b| ((a[0] - x).hypot(a[1] - y)).total_cmp(&(b[0] - x).hypot(b[1] - y)))
        .unwrap()
}

fn mover_no(d: &mut Document, id: Id, dz: f64) {
    if let Some(FeatureKind::Forma(f)) = d.feature_mut(id).map(|f| &mut f.kind) {
        // nó de dentro da gaiola, perto do centro
        let n = f.gaiola.nos.iter().min_by(|a, b| (a.p[0].hypot(a.p[1] - 16.7)).total_cmp(&b.p[0].hypot(b.p[1] - 16.7))).unwrap().id;
        f.gaiola.no_mut(n).unwrap().p[2] += dz;
    }
}

#[test]
fn forma_aberta_e_superficie_comum_com_faces_de_chave() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b511");
    let fm = d.add_feature(forma(folha()));
    let r = recompute(&d);
    ok(&r, &d, fm);
    assert!(r.bodies.is_empty(), "o casco aberto não vira corpo");
    let sv = r.surfaces.iter().find(|x| x.id == fm).expect("a forma está entre as superfícies");
    let area = area_da_malha(&sv.mesh);
    assert!(area > 100. * 100. && area < 100. * 100. * 1.1, "área {area}");
    let (lo, hi) = caixa(&sv.mesh);
    assert!((hi[0] - lo[0] - 100.).abs() < 25. && (hi[1] - lo[1] - 100.).abs() < 25., "caixa {lo:?} {hi:?}");
    // faces e arestas da superfície têm chave com a forma como dona (B515)
    let fk = sv.face_key(0).expect("chave da face");
    assert_eq!(fk.owner, fm);
    assert_eq!(fk.role, PAPEL_SUPERFICIE);
    let borda = (0..sv.mesh.edge_faces.len()).find(|e| sv.mesh.edge_faces[*e].contains(&fenkra_kernel::NO_FACE)).expect("borda");
    assert!(sv.edge_key(borda).is_some(), "a borda tem chave");
    assert!(Forma::e_superficie(match &d.feature(fm).unwrap().kind {
        FeatureKind::Forma(f) => f,
        _ => unreachable!(),
    }));
}

#[test]
fn forma_aberta_desloca_espessa_e_fura_e_a_edicao_segue() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b511");
    let fm = d.add_feature(forma(folha()));
    let off = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(fm), dist: s("5"), both: false, join: OffsetJoin::Automatico });
    let gr = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(fm), t1: s("3"), t2: s("0"), op: BoolOp::Add }));
    let r = recompute(&d);
    ok(&r, &d, fm);
    ok(&r, &d, off);
    ok(&r, &d, gr);
    let area = area_da_malha(malha_da_sup(&r, fm));
    // deslocar: área parecida (a folha é pouco curva), afastada 5 mm
    let m_off = malha_da_sup(&r, off);
    let a_off = area_da_malha(m_off);
    assert!((a_off - area).abs() / area < 0.06, "área deslocada {a_off} contra {area}");
    let (lo0, hi0) = caixa(malha_da_sup(&r, fm));
    let (lo1, hi1) = caixa(m_off);
    assert!(((hi1[2] - lo1[2]) - (hi0[2] - lo0[2])).abs() < 4. && (lo1[2] - lo0[2]).abs() > 2.5, "caixa {lo0:?}..{hi0:?} → {lo1:?}..{hi1:?}");
    // espessar: um sólido com volume perto de área × espessura
    assert_eq!(r.bodies.len(), 1);
    let v = r.bodies[0].mass.volume;
    assert!((v - area * 3.).abs() / (area * 3.) < 0.03, "volume {v} contra {}", area * 3.);
    let (cl, ch) = caixa_da_malha(&r);
    assert!(ch[2] - cl[2] < hi0[2] - lo0[2] + 3.5 && ch[2] - cl[2] > hi0[2] - lo0[2], "caixa do sólido {cl:?}..{ch:?}");

    // furar o sólido espessado clicando na face da forma: some perto de π r² t (a folha inclina pouco)
    let centro = ponto_em(malha_da_sup(&r, fm), 0., 0.);
    let mut h = match furo_passante(Id(0), "10") {
        FeatureKind::Hole(h) => h,
        _ => unreachable!(),
    };
    h.points = vec![PointRef::Fixed(centro)];
    let furo = d.add_feature(FeatureKind::Hole(h));
    let r2 = recompute(&d);
    ok(&r2, &d, furo);
    let tirado = v - r2.bodies[0].mass.volume;
    let esperado = PI * 25. * 3.;
    assert!(tirado > esperado * 0.95 && tirado < esperado * 1.25, "o furo tirou {tirado}, esperado ≈ {esperado}");

    // editar um nó da gaiola: tudo depois recalcula, sem perder referência
    mover_no(&mut d, fm, 12.);
    let r3 = recompute(&d);
    for id in [fm, off, gr, furo] {
        ok(&r3, &d, id);
    }
    let (_, hi2) = caixa(malha_da_sup(&r3, fm));
    assert!(hi2[2] > hi0[2] + 2., "o nó subiu: {} contra {}", hi2[2], hi0[2]);
    let (_, hi3) = caixa(malha_da_sup(&r3, off));
    assert!(hi3[2] > hi1[2] + 2., "o deslocamento seguiu a forma: {} contra {}", hi3[2], hi1[2]);
    let area3 = area_da_malha(malha_da_sup(&r3, fm));
    let v3 = r3.bodies[0].mass.volume;
    assert!((v3 - (area3 * 3. - esperado)).abs() / (area3 * 3.) < 0.04, "volume depois da edição {v3} contra {}", area3 * 3. - esperado);
    let (_, ch3) = caixa_da_malha(&r3);
    assert!(ch3[2] > ch[2] + 2., "o sólido seguiu a forma: {} contra {}", ch3[2], ch[2]);
}

#[test]
fn face_da_forma_por_chave_e_extrusao_ate_a_superficie() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b511");
    let fm = d.add_feature(forma(folha()));
    let r = recompute(&d);
    let sv = r.surfaces.iter().find(|x| x.id == fm).unwrap();
    // uma face só, escolhida pela chave (como o clique no 3D faz com um subelemento)
    let k = sv.face_key(3).unwrap();
    let p = ponto_em(&sv.mesh, 0., 0.);
    let uma = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Face { point: p, normal: [0., 0., 1.], owner: Some(fm), key: Some(k) }, dist: s("2"), both: false, join: OffsetJoin::Automatico });
    // extrusão de um quadrado no XY até a superfície da forma
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-10., -10.], [10., 10.]);
    let ex = d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::UpToSurface(SurfRef::Ref(fm)), reverse: false, taper: s("0"), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    ok(&r, &d, uma);
    ok(&r, &d, ex);
    let a1 = area_da_malha(malha_da_sup(&r, uma));
    let a_total = area_da_malha(malha_da_sup(&r, fm));
    assert!(a1 > 1. && a1 < a_total * 0.5, "uma face só: {a1} de {a_total}");
    // a folha passa por z = 30 ± 6 sobre o quadrado: o prisma tem 400 × altura média
    let v = r.bodies[0].mass.volume;
    assert!(v > 400. * 24. && v < 400. * 36., "volume {v}");
    let (lo, hi) = caixa_da_malha(&r);
    assert!(lo[2].abs() < 0.5 && hi[2] > 24. && hi[2] < 37., "caixa {lo:?}..{hi:?}");
}

#[test]
fn forma_em_edicao_deixa_os_usuarios_esperando_e_a_fechada_e_solido_furavel() {
    if !kernel_disponivel() {
        return;
    }
    // gaiola aberta em edição: o deslocamento espera (inativo), sem erro
    let mut d = Document::new("b511");
    let fm = d.add_feature(forma(folha()));
    let off = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(fm), dist: s("5"), both: false, join: OffsetJoin::Automatico });
    if let Some(FeatureKind::Forma(f)) = d.feature_mut(fm).map(|f| &mut f.kind) {
        f.conversao.as_mut().unwrap().pausada = true;
    }
    let r = recompute(&d);
    assert!(matches!(r.status.get(&off), Some(Status::Inactive)), "{:?}", r.status.get(&off));
    assert!(r.surfaces.iter().all(|x| x.id != fm), "em edição a forma mostra a gaiola, não o B-rep");

    // gaiola fechada: sólido comum, furado direto; como superfície ela avisa que é sólido
    let mut d = Document::new("b511");
    let fm = d.add_feature(forma(Gaiola::caixa([0., 0., 25.], [60., 60., 50.], [2, 2, 2])));
    let off = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(fm), dist: s("5"), both: false, join: OffsetJoin::Automatico });
    let r = recompute(&d);
    ok(&r, &d, fm);
    let v0 = r.bodies[0].mass.volume;
    assert!(matches!(r.status.get(&off), Some(Status::Error(e)) if e.contains("sólido")), "{:?}", r.status.get(&off));
    let mut h = match furo_passante(Id(0), "8") {
        FeatureKind::Hole(h) => h,
        _ => unreachable!(),
    };
    let topo = r.bodies[0].mesh.positions.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).max_by(|a, b| (a[2] - a[0].hypot(a[1])).total_cmp(&(b[2] - b[0].hypot(b[1])))).unwrap();
    h.points = vec![PointRef::Fixed(topo)];
    let furo = d.add_feature(FeatureKind::Hole(h));
    let r = recompute(&d);
    ok(&r, &d, furo);
    assert!(r.bodies[0].mass.volume < v0 - PI * 16. * 20., "o furo atravessou: {} de {v0}", r.bodies[0].mass.volume);
}
