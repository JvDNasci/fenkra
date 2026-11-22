// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pendências das ondas 1 e 2 (ADR 0107, B562): a seta clicável de inverter no 3D dos comandos de
//! `features` com `inverter::bool_de_feature` (extrusão, espessura, parede, abas, bainha, dobra, joggle,
//! veneziana). O clique na seta e o botão do painel ligam ao mesmo campo, e a seta aponta para onde o
//! material vai: invertido, ela vira junto. Documentos montados em código e recalculados.

use super::*;
use crate::contrato::Alvo;
use fenkra_core::sketch::Curve;
use fenkra_solidos::{recompute, Status};

fn s(x: &str) -> String {
    x.into()
}

fn rect(d: &mut Document, a: [f64; 2], b: [f64; 2]) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_500_000u64 + d.features.len() as u64 * 1000;
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

fn extrusao(sketch: Id, reverse: bool) -> FeatureKind {
    FeatureKind::Extrude(Extrude { thin_side: 0, sketch, op: BoolOp::Add, extent: Extent::Blind(s("10")), reverse, taper: s("0"), thin: None, dir: None, ends: Default::default() })
}

/// Chapa 100 × 50 × 2 com aba, bainha e dobra por linha (a mesma base dos testes das alças de chapa).
fn chapa() -> (Document, Vec<Id>) {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = rect(&mut d, [0., 0.], [100., 50.]);
    let w = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let a = d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("5"), off2: s("5"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    let h = d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: s("8"), gap: s("2"), flip: false, kind: 1, edge_key: None }));
    let b = d.add_feature(FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("45"), radius: None, flip: true, down: false, sketch: None, position: 0, cega: None }));
    (d, vec![w, a, h, b])
}

/// Bloco 100 × 50 × 10 com espessura de 3 mm na face de cima.
fn bloco() -> (Document, Vec<Id>) {
    let mut d = Document::new("bloco");
    let sk = rect(&mut d, [0., 0.], [100., 50.]);
    let e = d.add_feature(extrusao(sk, false));
    let t = d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[50., 25., 10.]], value: s("3"), face_keys: vec![], remove: false, regras: vec![] }));
    (d, vec![e, t])
}

fn kind(d: &Document, id: Id) -> FeatureKind {
    d.feature(id).unwrap().kind.clone()
}

/// O botão do painel: `inverter::botao(ui, &mut campo)` no campo de `bool_de_feature`.
fn pelo_botao(d: &Document, id: Id, campo: u8) -> Document {
    let mut d = d.clone();
    let k = &mut d.feature_mut(id).unwrap().kind;
    let b = crate::inverter::bool_de_feature(k, campo).expect("campo do botão");
    *b = !*b;
    d
}

/// Para cada comando: o 3D oferece as mesmas setas que o painel tem botões; a seta e o botão dão o
/// mesmo documento; e, recalculado, a seta do mesmo campo aponta para o outro lado.
#[test]
fn seta_do_3d_e_botao_do_painel_invertem_o_mesmo_campo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (chapa, ids_chapa) = chapa();
    let (bloco, ids_bloco) = bloco();
    let mut conferidos = 0;
    for (d, ids) in [(chapa, ids_chapa), (bloco, ids_bloco)] {
        let r = recompute(&d);
        for &id in &ids {
            let nome = d.name_of(id);
            assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{nome}: {:?}", r.status.get(&id));
            let botoes: Vec<u8> = crate::inverter::controles(Alvo::Feature(&kind(&d, id))).iter().map(|c| c.campo).collect();
            let setas = setas_de_lado(&d, &r, id);
            let campos: Vec<u8> = setas.iter().map(|s| s.campo).collect();
            assert_eq!(botoes, campos, "{nome}: painel e 3D oferecem os mesmos inversores");
            for seta in setas {
                let mut pela_seta = d.clone();
                inverter_lado(&mut pela_seta, id, seta.campo);
                assert_eq!(pela_seta.features, pelo_botao(&d, id, seta.campo).features, "{nome}: botão e seta divergem");
                let r2 = recompute(&pela_seta);
                assert!(matches!(r2.status.get(&id), Some(Status::Ok)), "{nome} invertido: {:?}", r2.status.get(&id));
                let depois = setas_de_lado(&pela_seta, &r2, id).into_iter().find(|x| x.campo == seta.campo).unwrap_or_else(|| panic!("{nome}: sem seta depois de inverter"));
                assert!(seta.dir.dot(depois.dir) < -0.3, "{nome}: a seta não virou ({:?} → {:?})", seta.dir, depois.dir);
                conferidos += 1;
            }
        }
    }
    assert!(conferidos >= 6, "o teste conferiu {conferidos} setas");
}

/// A seta fica coerente com a direção: na extrusão ela sai do centro do sketch e aponta para o corpo
/// (+Z), e invertida aponta para -Z, onde o corpo foi parar.
#[test]
fn seta_da_extrusao_solida_aponta_para_o_corpo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    for reverse in [false, true] {
        let mut d = Document::new("ext");
        let sk = rect(&mut d, [0., 0.], [100., 50.]);
        let e = d.add_feature(extrusao(sk, reverse));
        let r = recompute(&d);
        let seta = setas_de_lado(&d, &r, e).pop().expect("seta da extrusão");
        assert!(seta.base.z.abs() < 1e-3, "sai do plano do sketch: {:?}", seta.base);
        let b = r.bodies.iter().find(|b| b.collector == d.feature(e).unwrap().target).expect("corpo");
        let zs: Vec<f32> = b.mesh.positions.chunks_exact(3).map(|p| p[2]).collect();
        let meio = (zs.iter().copied().fold(f32::MAX, f32::min) + zs.iter().copied().fold(f32::MIN, f32::max)) / 2.;
        assert!((meio - if reverse { -5. } else { 5. }).abs() < 1e-3, "corpo no lado certo: {meio}");
        assert!(seta.dir.z * meio > 0. && seta.dir.z.abs() > 0.99, "a seta aponta para o corpo: {:?} (meio {meio})", seta.dir);
        assert_eq!(crate::inverter::estado(Alvo::Feature(&kind(&d, e)), 0), reverse, "o botão mostra o mesmo estado");
    }
}

/// A espessura: a seta sai das faces escolhidas para fora (acrescenta) e, invertida, para dentro
/// (remove), o mesmo campo da operação do painel.
#[test]
fn seta_da_espessura_segue_acrescentar_e_remover() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, ids) = bloco();
    let t = ids[1];
    let r = recompute(&d);
    let seta = setas_de_lado(&d, &r, t).pop().expect("seta da espessura");
    assert!(seta.dir.z > 0.9, "acrescentar: para fora da face de cima {:?}", seta.dir);
    inverter_lado(&mut d, t, 0);
    let FeatureKind::Thickness(x) = kind(&d, t) else { unreachable!() };
    assert!(x.remove, "a seta liga o mesmo campo da operação (Remover)");
    let r = recompute(&d);
    let seta = setas_de_lado(&d, &r, t).pop().expect("seta da espessura");
    assert!(seta.dir.z < -0.9, "remover: para dentro {:?}", seta.dir);
}
