// SPDX-License-Identifier: LGPL-2.1-or-later
//! Forma livre: volante, banco, bico, painel, pontão e proteção traseira; tubos das proteções.
use crate::apoio::*;
use crate::mestre::liga;
use crate::pecas::Pub;
use crate::quadro::membro;
use fenkra_core::subd::{self, Gaiola};
use fenkra_core::*;
use std::f64::consts::PI;

fn forma(d: &mut Document, g: Gaiola) -> Id {
    d.add_feature(FeatureKind::Forma(Forma { gaiola: g, niveis: 2, congelada: false, conversao: Some(ConversaoForma { continuidade: 1, tol: "0.05".into(), pausada: false }), vinculos: Default::default() }))
}

/// Gaiola de anel: n nós em volta, m na seção. Não há primitiva de anel: é feita por nós e faces.
fn anel(raio: f64, r_sec: f64, n: u32, m: u32) -> Gaiola {
    let mut g = Gaiola::caixa([0.; 3], [1.; 3], [1, 1, 1]);
    g.limpar_tudo();
    let mut ids = vec![];
    for i in 0..n {
        let a = 2. * PI * i as f64 / n as f64;
        for j in 0..m {
            let b = 2. * PI * j as f64 / m as f64;
            let r = raio + r_sec * b.cos();
            ids.push(g.novo_para_teste([r * a.cos(), r * a.sin(), r_sec * b.sin()]));
        }
    }
    for i in 0..n {
        for j in 0..m {
            let (i2, j2) = ((i + 1) % n, (j + 1) % m);
            let q = |a: u32, b: u32| ids[(a * m + b) as usize];
            g.face_para_teste(vec![q(i, j), q(i2, j), q(i2, j2), q(i, j2)]);
        }
    }
    g
}

trait Limpa {
    fn limpar_tudo(&mut self);
}
impl Limpa for Gaiola {
    fn limpar_tudo(&mut self) {
        self.nos.clear();
        self.faces.clear();
    }
}

pub fn di01(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["D_VOLANTE"], &[])?;
    let dv = match m.iter().find(|(p, _)| p == "D_VOLANTE") {
        Some((_, LinkedGeo::Value(v))) => *v,
        _ => 320.,
    };
    let f = forma(&mut d, anel(dv / 2. - 14., 14., 12, 4));
    if let Some(x) = d.feature_mut(f) {
        x.name = "Aro".into();
    }
    // raios: chapa extrudada em 3 braços
    let sk = sk_rect(&mut d, oxy(), [-dv / 2. + 16., -14.], [dv / 2. - 16., 14.]);
    d.add_feature(ext(sk, BoolOp::Add, sim("6")));
    let c = sk_circ(&mut d, oxy(), &[([0., 0.], 35.)]);
    d.add_feature(ext(c, BoolOp::Add, sim("8")));
    Ok(d)
}

/// Move nós por uma função da posição.
fn deforma(g: &mut Gaiola, f: impl Fn([f64; 3]) -> [f64; 3]) {
    for n in g.nos.iter_mut() {
        n.p = f(n.p);
    }
}

pub fn po01(_: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    // concha: caixa, faces de cima afundadas por extrusão negativa
    let mut g = Gaiola::caixa([0., 0., 200.], [420., 400., 400.], [4, 4, 4]);
    let topo: Vec<u32> = g.faces.iter().filter(|f| g.normal_face(f.id).is_some_and(|n| n[2] > 0.9)).filter(|f| g.centro_face(f.id).is_some_and(|c| c[0].abs() < 160. && c[1].abs() < 150.)).map(|f| f.id).collect();
    g.extrudar(&topo, -300.)?;
    deforma(&mut g, |p| [p[0] + p[2] * 0.35, p[1] * (1. - p[2] / 1500.), p[2]]);
    forma(&mut d, g);
    Ok(d)
}

fn casco(tam: [f64; 3], c: [f64; 3], div: [u32; 3], f: impl Fn([f64; 3]) -> [f64; 3]) -> Gaiola {
    let mut g = Gaiola::caixa(c, tam, div);
    deforma(&mut g, f);
    g
}

/// PR-06: bico, com simetria na gaiola.
pub fn pr06(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["L_NARIZ", "BAL_D", "EE", "DR_D"], &[])?;
    let v = |n: &str| match m.iter().find(|(p, _)| p == n) {
        Some((_, LinkedGeo::Value(x))) => *x,
        _ => 0.,
    };
    let (l, bal) = (v("L_NARIZ"), v("BAL_D"));
    let mut g = casco([bal - 200., l, 190.], [(bal - 200.) / 2., 0., 145.], [3, 4, 2], |p| {
        let t = (p[0] / 440.).clamp(0., 1.);
        [p[0], p[1] * (1. - 0.25 * t * t), 60. + (p[2] - 60.) * (1. - 0.45 * t)]
    });
    g.ligar_simetria(subd::Simetria { eixo: 1, pos: 0. })?;
    forma(&mut d, g);
    Ok(d)
}

pub fn pr07(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["L_PAINEL"], &[])?;
    let g = casco([60., 280., 330.], [0., 0., 280.], [1, 3, 3], |p| [p[0] + (p[2] - 115.) * 0.5, p[1] * (1. - (p[2] - 115.) / 800.), p[2]]);
    forma(&mut d, g);
    Ok(d)
}

pub fn pr08(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["VAO_PONTAO"], &[])?;
    let g = casco([640., 190., 140.], [0., 0., 110.], [4, 2, 2], |p| {
        let t = p[0] / 320.;
        [p[0], p[1] + 30. * t, 40. + (p[2] - 40.) * (1. - 0.2 * t * t)]
    });
    forma(&mut d, g);
    Ok(d)
}

pub fn pr09(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["LT", "VAO_TRAS"], &[])?;
    let lt = match m.iter().find(|(p, _)| p == "LT") {
        Some((_, LinkedGeo::Value(v))) => *v,
        _ => 1380.,
    };
    let g = casco([120., lt, 200.], [0., 0., 140.], [2, 6, 2], |p| [p[0] + 0.00006 * p[1] * p[1], p[1], p[2]]);
    forma(&mut d, g);
    Ok(d)
}

// ------------------------------------------------------------------ proteções

/// PR-01 e PR-02: para-choque em U dobrado com trecho reto dirigido pelo mestre.
pub fn parachoque(m: &Pub, sup: bool) -> Result<Document, String> {
    let mut d = Document::new("c");
    let (dn, rn, zn, ap, x) = if sup { ("D_PC_SUP", "RETO_PC_SUP", "Z_PC_SUP", "275", "mestre_EE+mestre_BAL_D-20") } else { ("D_PC_INF", "RETO_PC_INF", "Z_PC_INF", "225", "mestre_EE+mestre_BAL_D-60") };
    liga(&mut d, m, &[dn, rn, zn, "EE", "BAL_D"], &[])?;
    let z = format!("mestre_{zn}-mestre_{dn}/2");
    let r = format!("mestre_{rn}/2");
    let a = ponto(&mut d, "A", "mestre_EE+190", &format!("-{ap}"), &z);
    let b = ponto(&mut d, "B", &format!("{x}-40"), &format!("-{r}-40"), &z);
    let c = ponto(&mut d, "C", x, &format!("-{r}"), &z);
    let e = ponto(&mut d, "D", x, &r, &z);
    let f = ponto(&mut d, "E", &format!("{x}-40"), &format!("{r}+40"), &z);
    let g = ponto(&mut d, "F", "mestre_EE+190", ap, &z);
    let p = polilinha(&mut d, "Parachoque", &[a, b, c, e, f, g]);
    let id = membro(&mut d, "Tubo", tubo(trechos(p, 5), &format!("mestre_{dn}"), "2", "para-choque"));
    dobra_nos(&mut d, id, 5, "35");
    Ok(d)
}

pub fn pr03(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["EE", "BAL_D", "Z_PC_SUP", "Z_PC_INF", "D_PC_SUP", "D_PC_INF"], &[])?;
    let a = ponto(&mut d, "A", "mestre_EE+mestre_BAL_D-60", "0", "mestre_Z_PC_INF-mestre_D_PC_INF/2");
    let b = ponto(&mut d, "B", "mestre_EE+mestre_BAL_D-20", "0", "mestre_Z_PC_SUP-mestre_D_PC_SUP/2");
    let c = polilinha(&mut d, "Ligacao", &[a, b]);
    membro(&mut d, "Tubo", tubo(vec![CurveRef::Ref(c)], "16", "2", "Ø16x2"));
    Ok(d)
}

pub fn lateral(m: &Pub, sup: bool) -> Result<Document, String> {
    let mut d = Document::new("c");
    let (yn, z) = if sup { ("Y_LAT_SUP", "mestre_Z_LAT_SUP-10") } else { ("Y_LAT_INF", "60") };
    liga(&mut d, m, &[yn, "LARG_Q", "Z_LAT_SUP", "EE"], &[])?;
    let y = format!("mestre_{yn}");
    let q = "mestre_LARG_Q/2";
    let a = ponto(&mut d, "A", "mestre_EE*0.25", q, "Z0".replace("Z0", "60").as_str());
    let b = ponto(&mut d, "B", "mestre_EE*0.25+80", &y, z);
    let c = ponto(&mut d, "C", "mestre_EE*0.25+480", &y, z);
    let e = ponto(&mut d, "D", "mestre_EE*0.25+560", q, "60");
    let p = polilinha(&mut d, "Barra", &[a, b, c, e]);
    let id = membro(&mut d, "Tubo", tubo(trechos(p, 3), "20", "2", "Ø20x2"));
    dobra_nos(&mut d, id, 3, "40");
    Ok(d)
}

pub fn pr10(m: &Pub) -> Result<Document, String> {
    let mut d = Document::new("c");
    liga(&mut d, m, &["BAL_T", "LARG_Q"], &[])?;
    let a = ponto(&mut d, "A", "-100", "mestre_LARG_Q/2", "80");
    let b = ponto(&mut d, "B", "-mestre_BAL_T+80", "mestre_LARG_Q/2+60", "140");
    let c = polilinha(&mut d, "Suporte", &[a, b]);
    membro(&mut d, "Tubo", tubo(vec![CurveRef::Ref(c)], "20", "2", "Ø20x2"));
    Ok(d)
}
