// SPDX-License-Identifier: LGPL-2.1-or-later
//! B86: na planificada nenhum comando some. O estampo por punção não amassa a chapa plana: a
//! pegada dele (a seção pelo plano da chapa) fica marcada como ferramenta.

use super::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn placa() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 4_500_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [-50., -25.], [50., 25.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

/// Chapa com um punção cilíndrico (raio 5) no centro e o estampo por punção dele.
fn com_puncao() -> (Document, Id, Id) {
    let mut d = placa();
    let chapa = d.collectors[0].id;
    let pun = d.add_collector("Punção");
    // o punção apoia a base na face de cima da chapa (z = 2)
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
    d.active_collector = pun;
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Datum(topo), a: s("5"), b: s("10"), c: s("0"), d: s("0"), op: BoolOp::Add }));
    d.active_collector = chapa;
    let st = d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: Id(0), depth: s("3"), flip: false, kind: 4, width: String::new(), punch: Some(pun) }));
    (d, st, chapa)
}

fn volume_da_chapa(r: &RecomputeResult, chapa: Id) -> f64 {
    r.bodies.iter().find(|b| b.collector == chapa).map(|b| b.mass.volume).expect("corpo da chapa")
}

#[test]
fn estampo_por_puncao_na_planificada_marca_a_pegada_e_nao_amassa_a_chapa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, st, chapa) = com_puncao();
    // dobrada: o punção forma o copo e muda o volume da chapa
    let rd = recompute(&d);
    assert!(matches!(rd.status[&st], Status::Ok | Status::Warning(_)), "{:?}", rd.status[&st]);
    let v_dobrada = volume_da_chapa(&rd, chapa);
    assert!((v_dobrada - 10000.).abs() > 1., "o copo deveria mudar o volume ({v_dobrada})");
    // planificada: a chapa continua plana (100 x 50 x 2) e a pegada do punção vira marca de ferramenta
    d.sheet.as_mut().unwrap().flat = true;
    let rp = recompute(&d);
    assert!(matches!(rp.status[&st], Status::Ok | Status::Warning(_)), "{:?}", rp.status[&st]);
    assert!((volume_da_chapa(&rp, chapa) - 10000.).abs() < 1e-6, "a chapa plana não leva o copo");
    let marca = rp.cut_outlines.iter().find(|o| o.feature == st).expect("o estampo some da planificada");
    assert_eq!(marca.flat.len(), 1, "uma pegada");
    // a pegada é o círculo de raio 5 do punção, na face de cima da chapa (z = 2)
    for p in &marca.flat[0] {
        assert!((p[2] - 2.).abs() < 1e-3, "fora da face: {p:?}");
        assert!((p[0].hypot(p[1]) - 5.).abs() < 0.05, "fora do círculo de raio 5: {p:?}");
    }
    // e sai na camada de ferramenta da planificada
    let plana = planificada::planificar(&d).expect("planificada");
    assert!(plana.ferramenta.iter().any(|e| matches!(e, planificada::Entidade::Circulo { raio, .. } if (raio - 5.).abs() < 0.05) || matches!(e, planificada::Entidade::Polilinha(..) | planificada::Entidade::Arco { .. })), "{:?}", plana.ferramenta);
    assert!(plana.ausentes.iter().all(|a| a.feature != st), "o estampo não pode estar ausente: {:?}", plana.ausentes);
}
