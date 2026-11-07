// SPDX-License-Identifier: LGPL-2.1-or-later
//! Etapa 3: quadro, CH-01 a CH-07, tubos sobre o esqueleto.
use crate::apoio::*;
use crate::mestre::liga;
use fenkra_core::*;
use fenkra_solidos::{cut_list, member_failures, tubos_dobrados};

const P: &[&str] = &["D_TUBO", "E_TUBO", "R_DOBRA"];
const D: &str = "mestre_D_TUBO";
const E: &str = "mestre_E_TUBO";
const R: &str = "mestre_R_DOBRA";

pub fn membro(d: &mut Document, nome: &str, m: StructMember) -> Id {
    let f = d.add_feature(FeatureKind::StructMember(m));
    if let Some(x) = d.feature_mut(f) {
        x.name = nome.into();
    }
    f
}

pub fn longarina(m: &[(String, LinkedGeo)], lado: &str) -> Result<Document, String> {
    let mut d = Document::new("longarina");
    let n: Vec<String> = (0..4).map(|i| format!("LONG_{lado}_{i}")).collect();
    let c = liga(&mut d, m, P, &n.iter().map(|x| x.as_str()).collect::<Vec<_>>())?;
    let f = membro(&mut d, "Longarina", tubo(c.iter().map(|i| CurveRef::Ref(*i)).collect(), D, E, "Ø30x2,4"));
    dobra_nos(&mut d, f, 4, R);
    Ok(d)
}

pub fn reto(m: &[(String, LinkedGeo)], curvas: &[&str]) -> Result<Document, String> {
    let mut d = Document::new("tubo");
    let c = liga(&mut d, m, P, curvas)?;
    for (i, id) in c.iter().enumerate() {
        membro(&mut d, curvas[i], tubo(vec![CurveRef::Ref(*id)], D, E, "Ø30x2,4"));
    }
    Ok(d)
}

pub fn arco(m: &[(String, LinkedGeo)]) -> Result<Document, String> {
    let mut d = Document::new("arco");
    let c = liga(&mut d, m, P, &["ARCO_0", "ARCO_1", "ARCO_2"])?;
    let f = membro(&mut d, "Arco", tubo(c.iter().map(|i| CurveRef::Ref(*i)).collect(), D, E, "Ø30x2,4"));
    dobra_nos(&mut d, f, 3, R);
    Ok(d)
}

/// O quadro inteiro num documento: membros, encaixes e lista de corte.
pub fn quadro(m: &[(String, LinkedGeo)]) -> Result<Document, String> {
    let mut d = Document::new("quadro");
    d.props.stock_length = "6000".into();
    let c = liga(&mut d, m, P, &["LONG_E_0", "LONG_E_1", "LONG_E_2", "LONG_E_3", "LONG_D_0", "LONG_D_1", "LONG_D_2", "LONG_D_3", "TR_DIANT", "TR_MONT", "TR_TRAS", "ARCO_0", "ARCO_1", "ARCO_2", "BANCO_AE", "BANCO_AD", "BANCO_FE", "BANCO_FD"])?;
    let refs = |a: usize, b: usize| -> Vec<CurveRef> { c[a..b].iter().map(|i| CurveRef::Ref(*i)).collect() };
    let le = membro(&mut d, "CH-01 Longarina esquerda", tubo(refs(0, 4), D, E, "Ø30x2,4"));
    dobra_nos(&mut d, le, 4, R);
    let ld = membro(&mut d, "CH-02 Longarina direita", tubo(refs(4, 8), D, E, "Ø30x2,4"));
    dobra_nos(&mut d, ld, 4, R);
    for (i, n) in [(8, "CH-03 Travessa dianteira"), (9, "CH-04 Montante central"), (10, "CH-05 Travessa traseira")] {
        let mut t = tubo(vec![CurveRef::Ref(c[i])], D, E, "Ø30x2,4");
        t.corner = CornerMode::Encaixe;
        membro(&mut d, n, t);
    }
    let a = membro(&mut d, "CH-06 Arco", tubo(refs(11, 14), D, E, "Ø30x2,4"));
    dobra_nos(&mut d, a, 3, R);
    for (i, n) in [(14, "CH-07 Apoio banco AE"), (15, "CH-07 Apoio banco AD"), (16, "CH-07 Apoio banco FE"), (17, "CH-07 Apoio banco FD")] {
        let mut t = tubo(vec![CurveRef::Ref(c[i])], D, E, "Ø30x2,4");
        t.corner = CornerMode::Encaixe;
        membro(&mut d, n, t);
    }
    Ok(d)
}

pub fn relata_quadro(d: &Document) {
    println!("--- lista de corte");
    for r in cut_list(d) {
        println!("item {} {} L={:.1} x{} a1={:?} a2={:?} tubo={}", r.item, r.designation, r.length, r.count, r.angle1, r.angle2, r.tubo.is_some());
    }
    println!("--- tubos dobrados: {}", tubos_dobrados(d).len());
    for f in member_failures(d) {
        println!("falha membro {f:?}");
    }
}
