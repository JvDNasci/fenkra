// SPDX-License-Identifier: LGPL-2.1-or-later
//! Peça anotada do corpus para os testes de PMI no STEP (B191): a flange (disco com furo central e
//! quatro furos de fixação) com cotas, quadros, referências, um alvo e anotações gráficas.
//! Incluído por `#[path]` de mais de um teste; precisa do módulo `common` de `fenkra-solidos`.

#![allow(dead_code)]

use super::common::*;
use fenkra_core::gdt::Modif;
use fenkra_core::pmi::*;
use fenkra_core::pmi_alvos::{AlvoRef, FormaAlvo};
use fenkra_core::*;

pub fn flange() -> Document {
    let mut d = Document::new("flange anotada");
    let base = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 30.)]);
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let centro = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    d.add_feature(furo_passante(centro, "20"));
    let furos = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[20., 0.], [0., 20.], [-20., 0.], [0., -20.]]);
    d.add_feature(furo_passante(furos, "6"));
    d
}

fn pmi(id: u64, nome: &str, kind: PmiKind, targets: Vec<Option<PmiTarget>>) -> Pmi {
    Pmi { id: Id(id), name: nome.into(), kind, anchors: vec![[0.; 3]; targets.len().max(1)], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets }
}

fn plano(dir: [f64; 3], ponto: [f64; 3]) -> Option<PmiTarget> {
    Some(PmiTarget { class: TargetClass::FacePlana, point: ponto, dir, radius: 0., concave: false })
}

fn furo(centro: [f64; 3], raio: f64) -> Option<PmiTarget> {
    Some(PmiTarget { class: TargetClass::FaceCilindrica, point: centro, dir: [0., 0., 1.], radius: raio, concave: true })
}

/// A flange com o PMI de um desenho de fabricação típico.
pub fn flange_anotada() -> Document {
    let mut d = flange();
    let topo = [5., 5., 8.];
    // as anotações ligam às faces pela chave geométrica, como na interface
    let mut a = pmi(1, "A", PmiKind::Datum { letter: "A".into() }, vec![plano([0., 0., 1.], topo)]);
    a.anchors = vec![[5., 5., 8.]];
    let b = pmi(2, "B", PmiKind::Datum { letter: "B".into() }, vec![furo([0., 0., 4.], 10.)]);
    let mut espessura = pmi(3, "espessura", PmiKind::Linear, vec![plano([0., 0., -1.], [5., 5., 0.]), plano([0., 0., 1.], topo)]);
    espessura.anchors = vec![[5., 5., 0.], [5., 5., 8.]];
    espessura.tol_plus = "0,1".into();
    espessura.tol_minus = "0,05".into();
    let mut dia = pmi(4, "furo central", PmiKind::Diameter, vec![furo([0., 0., 4.], 10.)]);
    dia.anchors = vec![[0., 0., 4.], [10., 0., 4.]];
    dia.tol_plus = "0,02".into();
    dia.tol_minus = "0".into();
    let posicao = pmi(
        5,
        "posição dos furos",
        PmiKind::Gdt {
            ch: GdtChar::Posicao,
            tol: "0,2".into(),
            datums: "A B".into(),
            extra: GdtExtra { zona: Some(fenkra_core::gdt::Zona::Diametro), mods: vec![Modif::Mmc], ..Default::default() },
        },
        vec![furo([20., 0., 4.], 3.)],
    );
    let planeza = pmi(6, "planeza", PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: String::new(), extra: Default::default() }, vec![plano([0., 0., 1.], topo)]);
    let alvo = pmi(7, "A1", PmiKind::DatumTarget { alvo: AlvoRef { forma: FormaAlvo::Retangulo, letra: "A".into(), numero: 1, dim: [6., 4.], ..Default::default() } }, vec![plano([0., 0., 1.], topo)]);
    let rug = pmi(8, "Ra", PmiKind::Roughness { ra: "1,6".into(), process: String::new(), extra: Default::default() }, vec![plano([0., 0., 1.], topo)]);
    let nota = pmi(9, "nota", PmiKind::Note, vec![]);
    d.pmi = vec![a, b, espessura, dia, posicao, planeza, alvo, rug, nota];
    d
}

