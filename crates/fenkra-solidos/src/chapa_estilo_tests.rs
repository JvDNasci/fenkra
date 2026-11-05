// SPDX-License-Identifier: LGPL-2.1-or-later
//! B283: estilos de chapa nomeados. Herança, sobrescrita, troca de estilo e persistência
//! conferidas por volume, tabela de dobras e arquivo salvo.

use super::*;
use fenkra_core::EstiloRegras;
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn base(off: &str, relief: Option<&str>) -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 3_000_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s(off), off2: s(off), flip: false, relief: relief.map(s), relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    d.sheet_styles = vec![
        EstiloRegras::novo("Fino").com("espessura", "2").com("raio", "2").com("k", "0.44"),
        EstiloRegras::novo("Raio 2x").com("espessura", "2").com("raio", "2*espessura").com("k", "0.40").com("alivio", "3"),
    ];
    d
}

fn volume(d: &Document) -> f64 {
    let r = recompute(d);
    for f in &d.features {
        assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
    }
    r.bodies[0].mass.volume
}

/// Volume esperado: chapa 100x50x2 + dobra de 90° com raio `r` + aba reta de 20 mm.
fn esperado(r: f64) -> f64 {
    10000. + PI / 4. * ((r + 2.).powi(2) - r * r) * 50. + 20. * 2. * 50.
}

#[test]
fn trocar_o_estilo_recalcula_a_peca_e_a_tabela_de_dobras() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = base("0", None);
    d.sheet.as_mut().unwrap().escolhe_estilo("Fino");
    let v_fino = volume(&d);
    assert!((v_fino - esperado(2.)).abs() < 1e-3 * v_fino, "estilo Fino {v_fino}");
    assert!((bend_table(&d)[0].radius - 2.).abs() < 1e-9);

    // o raio é fórmula da espessura: no estilo "Raio 2x" vira 4 mm
    d.sheet.as_mut().unwrap().escolhe_estilo("Raio 2x");
    let v = volume(&d);
    assert!((v - esperado(4.)).abs() < 1e-3 * v, "estilo Raio 2x {v}");
    assert!((bend_table(&d)[0].radius - 4.).abs() < 1e-9);
    // o K do estilo vale: desenvolvido = 90° x (4 + 0,40 x 2)
    assert!((bend_table(&d)[0].allowance - PI / 2. * (4. + 0.4 * 2.)).abs() < 1e-9);

    // mudar a espessura dentro do estilo move o raio junto
    d.sheet_styles[1].campos.insert(s("espessura"), s("3"));
    let tab = bend_table(&d);
    assert!((tab[0].radius - 6.).abs() < 1e-9, "raio {}", tab[0].radius);
}

#[test]
fn sobrescrita_da_peca_vence_o_estilo_e_o_cadeado_devolve() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = base("0", None);
    d.sheet.as_mut().unwrap().escolhe_estilo("Raio 2x");
    // editar o raio na janela, com estilo ativo, vira sobrescrita
    let est = d.sheet_styles.clone();
    let antes = d.sheet.as_ref().unwrap().efetivo(&est);
    let mut depois = antes.clone();
    depois.radius = s("3");
    d.sheet.as_mut().unwrap().aplica_edicao(&antes, &depois, &est);
    assert_eq!(d.sheet.as_ref().unwrap().overrides, vec![s("raio")]);
    let v = volume(&d);
    assert!((v - esperado(3.)).abs() < 1e-3 * v, "sobrescrito {v}");
    // soltar o cadeado volta a herdar
    d.sheet.as_mut().unwrap().restaura("raio");
    let v = volume(&d);
    assert!((v - esperado(4.)).abs() < 1e-3 * v, "herdado {v}");
    // a aba também sobrescreve (raio próprio) sem mexer no estilo
    if let FeatureKind::SmFlange(fl) = &mut d.features.last_mut().unwrap().kind {
        fl.radius = Some(s("1.5*espessura"));
    }
    let v = volume(&d);
    assert!((v - esperado(3.)).abs() < 1e-3 * v, "aba sobrescreve {v}");
}

#[test]
fn aba_herda_o_alivio_do_estilo_e_a_propria_vence() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // o estilo define alívio de 3 mm: a aba herda
    let mut d = base("5", None);
    d.sheet.as_mut().unwrap().escolhe_estilo("Raio 2x");
    let v_estilo = volume(&d);
    // a mesma coisa escrita à mão na aba (alívio retangular de 3 mm) dá o mesmo volume
    let mut d2 = base("5", Some("3"));
    d2.sheet.as_mut().unwrap().escolhe_estilo("Raio 2x");
    let v_aba = volume(&d2);
    assert!((v_estilo - v_aba).abs() < 1e-6, "herdado {v_estilo} x próprio {v_aba}");
    // o estilo sem alívio deixa o material (a aba continua inteira atrás da dobra)
    let mut d3 = base("5", None);
    d3.sheet.as_mut().unwrap().escolhe_estilo("Fino");
    d3.sheet_styles[0].campos.insert(s("raio"), s("2*espessura"));
    d3.sheet_styles[0].campos.insert(s("k"), s("0.40"));
    let v_sem = volume(&d3);
    assert!(v_estilo < v_sem - 1., "o alívio herdado tirou material: {v_estilo} x {v_sem}");
}

#[test]
fn estilo_sobrevive_a_salvar_e_abrir() {
    let mut d = base("0", None);
    d.sheet.as_mut().unwrap().escolhe_estilo("Raio 2x");
    d.sheet.as_mut().unwrap().overrides.push(s("k"));
    d.sheet.as_mut().unwrap().k = s("0.5");
    let json = serde_json::to_string(&d).unwrap();
    let volta: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(volta.sheet_styles.len(), 2);
    let e = volta.sheet_efetivo().unwrap();
    assert_eq!((e.radius.as_str(), e.k.as_str(), e.relief.as_str()), ("2*espessura", "0.5", "3"));
    assert_eq!(volta.sheet.as_ref().unwrap().origem(&volta.sheet_styles, "k"), fenkra_core::Origem::Sobrescrita);
}
