// SPDX-License-Identifier: LGPL-2.1-or-later
//! B537: forma livre aberta (2 × 2) espessada e deslocada; a gaiola volta à edição com o espessar
//! depois dela. Durante a edição quem usa a forma espera (inativo, sem erro) e, ao concluir, o espessar
//! e o deslocamento recalculam com a forma nova. Mesmo cache entre os cálculos, como o app faz.

mod common;

use common::*;
use fenkra_core::subd::Gaiola;
use fenkra_core::*;
use fenkra_solidos::{recompute_cached, Cache, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn ok(r: &RecomputeResult, d: &Document, id: Id) {
    assert!(matches!(r.status.get(&id), Some(Status::Ok | Status::Warning(_))), "{}: {:?}", d.name_of(id), r.status.get(&id));
}

fn forma_de(d: &mut Document, id: Id) -> &mut Forma {
    match d.feature_mut(id).map(|f| &mut f.kind) {
        Some(FeatureKind::Forma(f)) => f,
        _ => panic!("forma"),
    }
}

fn caixa_da_sup(r: &RecomputeResult, id: Id) -> ([f64; 3], [f64; 3]) {
    let m = &r.surfaces.iter().find(|x| x.id == id).unwrap_or_else(|| panic!("superfície {id:?}")).mesh;
    let (mut a, mut b) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for k in 0..3 {
            a[k] = a[k].min(p[k] as f64);
            b[k] = b[k].max(p[k] as f64);
        }
    }
    (a, b)
}

#[test]
fn editar_a_gaiola_com_espessar_e_deslocar_depois_recalcula_os_dois() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("b537");
    // folha 2 × 2 de 60 mm, com o nó do meio um pouco acima: casco aberto, superfície comum
    let mut g = Gaiola::plano([0.; 3], [60., 60.], [2, 2]);
    let meio = g.nos.iter().find(|n| n.p[0].abs() < 1e-9 && n.p[1].abs() < 1e-9).expect("nó do meio").id;
    g.no_mut(meio).unwrap().p[2] = 4.;
    let fm = d.add_feature(FeatureKind::Forma(Forma { gaiola: g, niveis: 3, congelada: false, conversao: Some(ConversaoForma { continuidade: 1, tol: s("0.01"), pausada: false }), vinculos: Default::default() }));
    let esp = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(fm), t1: s("2"), t2: s("0"), op: BoolOp::Add, simetrico: false, dir: None }));
    let off = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(fm), dist: s("5"), both: false, join: OffsetJoin::Automatico });
    // como o app ao concluir: o espessar e o deslocamento consomem (ocultam) a forma
    d.hide_consumed(esp);
    d.hide_consumed(off);
    let mut cache = Cache::default();
    let r0 = recompute_cached(&d, &mut cache);
    for id in [fm, esp, off] {
        ok(&r0, &d, id);
    }
    assert_eq!(r0.bodies.len(), 1, "o espessar faz um sólido");
    let v0 = r0.bodies[0].mass.volume;
    assert!(v0 > 60. * 60. * 2. * 0.97 && v0 < 60. * 60. * 2. * 1.1, "volume {v0}");
    let (_, ch0) = caixa_da_malha(&r0);
    let (_, oh0) = caixa_da_sup(&r0, off);

    // volta à gaiola: a forma pausa, os usuários esperam sem erro e a forma lisa continua na tela
    forma_de(&mut d, fm).conversao.as_mut().unwrap().pausada = true;
    let r1 = recompute_cached(&d, &mut cache);
    ok(&r1, &d, fm);
    assert!(matches!(r1.status.get(&esp), Some(Status::Inactive)), "espessar em espera: {:?}", r1.status.get(&esp));
    assert!(matches!(r1.status.get(&off), Some(Status::Inactive)), "deslocar em espera: {:?}", r1.status.get(&off));
    assert!(r1.formas.iter().any(|x| x.id == fm), "a gaiola aberta mostra a forma mesmo consumida");

    // muda um ponto da gaiola e conclui
    forma_de(&mut d, fm).gaiola.no_mut(meio).unwrap().p[2] = 20.;
    let r2 = recompute_cached(&d, &mut cache);
    assert!(matches!(r2.status.get(&esp), Some(Status::Inactive)), "{:?}", r2.status.get(&esp));
    forma_de(&mut d, fm).conversao.as_mut().unwrap().pausada = false;
    let r3 = recompute_cached(&d, &mut cache);
    for id in [fm, esp, off] {
        ok(&r3, &d, id);
    }
    assert_eq!(r3.bodies.len(), 1);
    let v3 = r3.bodies[0].mass.volume;
    assert!(v3 > v0 * 1.02, "a cúpula mais alta tem mais área, e o sólido mais volume: {v3} contra {v0}");
    let (_, ch3) = caixa_da_malha(&r3);
    assert!(ch3[2] > ch0[2] + 5., "o sólido subiu com o nó: {} contra {}", ch3[2], ch0[2]);
    let (_, oh3) = caixa_da_sup(&r3, off);
    assert!(oh3[2] > oh0[2] + 5., "o deslocamento seguiu a forma: {} contra {}", oh3[2], oh0[2]);
}
