// SPDX-License-Identifier: LGPL-2.1-or-later
//! B304: dobrar e desdobrar animado e ordem de dobra editável.
//!
//! O controle em 0% e 100% reproduz exatamente os dois estados; no meio a aba abre na proporção.
//! A ordem escolhida (com número repetido) aparece na tabela, na planificada e no DXF.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::chapa_animacao::{doc_parcial, fixos};
use fenkra_solidos::planificada::{dxf, planificar};
use fenkra_solidos::{recompute, tabela_dobras, Status};

fn s(x: &str) -> String {
    x.to_string()
}

/// Placa 100 x 50 x 2 com abas de 20 mm a 90° nas pontas x = 0 e x = 100, e uma bainha.
fn peca(com_bainha: bool) -> (Document, Id, Id) {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let aba = |e: [f64; 3]| FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None });
    let a1 = d.add_feature(aba([100., 25., 2.]));
    let a2 = d.add_feature(aba([0., 25., 2.]));
    if com_bainha {
        d.add_feature(FeatureKind::SmHem(SmHem { edge: [50., 0., 2.], length: s("6"), gap: s("0"), flip: false, kind: 0, edge_key: None }));
    }
    (d, a1, a2)
}

fn caixa(d: &Document) -> ([f64; 3], [f64; 3]) {
    caixa_da_malha(&recompute(d))
}

#[test]
fn zero_e_cem_por_cento_sao_os_estados_de_verdade() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _, _) = peca(false);
    // 0%: o documento dobrado, igual
    assert_eq!(doc_parcial(&d, 0.), d);
    // 100%: só liga a planificada
    let mut plana = d.clone();
    plana.sheet.as_mut().unwrap().flat = true;
    assert_eq!(doc_parcial(&d, 1.), plana);
    let (lo, hi) = caixa(&doc_parcial(&d, 1.));
    // a planificada é plana (só a espessura) e mais comprida que a placa (as abas desenvolvidas)
    assert!(hi[0] - lo[0] > 130., "{lo:?} {hi:?}");
    assert!((hi[2] - lo[2] - 2.).abs() < 1e-3, "planificada com altura {lo:?} {hi:?}");
}

#[test]
fn no_meio_as_abas_abrem_na_proporcao_e_o_volume_fica() {
    if !kernel_disponivel() {
        return;
    }
    let (d, a1, a2) = peca(false);
    let r0 = recompute(&d);
    let v0 = volume_total(&r0);
    let (lo0, hi0) = caixa_da_malha(&r0);
    let mut alturas = vec![hi0[2] - lo0[2]];
    for f in [0.25, 0.5, 0.75] {
        let p = doc_parcial(&d, f);
        // a expressão preserva o valor de antes, multiplicado
        let FeatureKind::SmFlange(x) = &p.feature(a1).unwrap().kind else { panic!() };
        assert!(x.angle.starts_with("(90)*"), "{}", x.angle);
        let r = recompute(&p);
        for id in [a1, a2] {
            assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{f}: {:?}", r.status[&id]);
        }
        let v = volume_total(&r);
        assert!(((v - v0) / v0).abs() < 0.003, "{f}: volume {v} contra {v0}");
        let (lo, hi) = caixa_da_malha(&r);
        alturas.push(hi[2] - lo[2]);
        // a aba de 20 mm (mais o que o arco perdeu, f * BA) a (90 * (1 - f))° sobe perto disso vezes
        // o seno, mais raio e espessura
        let ang = (90. * (1. - f)).to_radians();
        let perna = 20. + f * std::f64::consts::PI / 2. * (2. + 0.44 * 2.);
        assert!(hi[2] - lo[2] < perna * ang.sin() + 6. && hi[2] - lo[2] > 20. * ang.sin(), "{f}: altura {}", hi[2] - lo[2]);
    }
    alturas.push(2.);
    assert!(alturas.windows(2).all(|w| w[1] < w[0]), "a peça não abre aos poucos: {alturas:?}");
}

#[test]
fn bainha_fica_ate_o_fim_e_e_listada() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _, _) = peca(true);
    let f = fixos(&d);
    assert_eq!(f.len(), 1, "{f:?}");
    let r = recompute(&doc_parcial(&d, 0.5));
    assert!(r.status.values().all(|s| !matches!(s, Status::Error(_))), "{:?}", r.status);
}

#[test]
fn ordem_editada_com_repeticao_vai_para_tabela_planificada_e_dxf() {
    if !kernel_disponivel() {
        return;
    }
    let (mut d, a1, a2) = peca(false);
    let r = recompute(&d);
    let t = tabela_dobras(&d, &r.bend_lines);
    assert_eq!(t.iter().map(|x| x.ordem).collect::<Vec<_>>(), vec![1, 2], "sem escolha, a ordem do mapa");
    // a segunda aba primeiro; depois as duas no mesmo número (mesmo golpe)
    let sp = d.sheet.as_mut().unwrap();
    sp.define_ordem(a2, 0, 1);
    sp.define_ordem(a1, 0, 2);
    let t = tabela_dobras(&d, &r.bend_lines);
    let por = |id: Id| t.iter().find(|x| x.feature == id).unwrap().ordem;
    assert_eq!((por(a1), por(a2)), (2, 1));
    d.sheet.as_mut().unwrap().define_ordem(a1, 0, 1);
    let t = tabela_dobras(&d, &r.bend_lines);
    assert!(t.iter().all(|x| x.ordem == 1), "número repetido permitido");
    // planificada e DXF com a ordem escolhida
    d.sheet.as_mut().unwrap().define_ordem(a1, 0, 7);
    let p = planificar(&d).unwrap();
    let mut ordens: Vec<usize> = p.dobras.iter().map(|x| x.ordem).collect();
    ordens.sort();
    assert_eq!(ordens, vec![1, 7]);
    assert!(dxf(&p).contains("\n7 "), "o DXF não traz o número 7");
    // gravado e lido de volta
    let json = serde_json::to_string(&d).unwrap();
    let lido: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(lido.sheet.unwrap().ordem_de(a1, 0), Some(7));
}
