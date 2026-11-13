// SPDX-License-Identifier: LGPL-2.1-or-later
//! B53: as dezesseis peças do corpus, montadas só com a API pública. Ficam aqui para outros testes
//! (o checador de similaridade do B237, no `fenkra-cli` e no `fenkra-app`) usarem as mesmas peças.

use super::*;
use fenkra_core::*;
use fenkra_solidos::eixos::{chaveta_documento, eixo_documento, ParamEixo, TrechoEixo};
use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem};
use fenkra_solidos::fixadores::fixador_documento;

// ------------------------------------------------------------------ as 16 peças

pub fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(s, BoolOp::Add, "20"));
    d
}

pub fn cilindro() -> Document {
    let mut d = Document::new("cilindro");
    d.add_feature(primitiva(PrimKind::Cilindro, "10", "30", "0", "0"));
    d
}

pub fn esfera() -> Document {
    let mut d = Document::new("esfera");
    d.add_feature(primitiva(PrimKind::Esfera, "10", "0", "0", "0"));
    d
}

pub fn cone() -> Document {
    let mut d = Document::new("cone");
    d.add_feature(primitiva(PrimKind::Cone, "12", "6", "20", "0"));
    d
}

pub fn toro() -> Document {
    let mut d = Document::new("toro");
    d.add_feature(primitiva(PrimKind::Toro, "20", "5", "0", "0"));
    d
}

/// Placa 100 × 60 × 8 com grade 5 × 3 de furos passantes Ø6 a cada 20 mm.
pub fn placa_com_grade() -> Document {
    let mut d = Document::new("placa com grade");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-50., -30.], [50., 30.]);
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let mut pts = vec![];
    for i in -2..=2 {
        for j in -1..=1 {
            pts.push([i as f64 * 20., j as f64 * 20.]);
        }
    }
    let sk = sk_pontos(&mut d, PlaneRef::Datum(topo), &pts);
    d.add_feature(furo_passante(sk, "6"));
    d
}

/// Caixa 40 × 30 × 20 com bolso 10 × 10 × 5 aberto em cima.
pub fn caixa_com_bolso() -> Document {
    let mut d = Document::new("caixa com bolso");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(base, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let sk = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
    d.add_feature(ext(sk, BoolOp::Cut, "5"));
    d
}

pub fn caixa_arredondada() -> Document {
    let mut d = caixa();
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: "2".into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
    d
}

/// Caixa 40 × 30 × 20 esvaziada com parede de 2, aberta em cima.
pub fn caixa_casca() -> Document {
    let mut d = caixa();
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![], closed: false, regras: vec![] }));
    d
}

/// Flange Ø60 × 8 com furo central Ø20 e quatro furos Ø6 no círculo de Ø40.
pub fn flange() -> Document {
    let mut d = Document::new("flange");
    let base = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 30.)]);
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let centro = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    d.add_feature(furo_passante(centro, "20"));
    let furos = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[20., 0.], [0., 20.], [-20., 0.], [0., -20.]]);
    d.add_feature(furo_passante(furos, "6"));
    d
}

pub fn eixo_escalonado() -> Document {
    let p = ParamEixo {
        trechos: vec![TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None }, TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None }],
        furo_centro: false,
    };
    eixo_documento(&p).expect("eixo")
}

pub fn engrenagem_40_dentes() -> Document {
    engrenagem_documento(&ParamEngrenagem { modulo: 2., dentes: 40, largura: 10., furo: 10., ..Default::default() }).expect("engrenagem")
}

/// Bloco em L: 40 × 30 com o canto de 20 × 15 retirado, espessura 10.
pub fn bloco_l() -> Document {
    let mut d = Document::new("bloco L");
    let sk = sk_poligono(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[0., 0.], [40., 0.], [40., 15.], [20., 15.], [20., 30.], [0., 30.]]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d
}

pub fn pecas() -> [Document; 16] {
    [
        caixa(),
        cilindro(),
        esfera(),
        cone(),
        toro(),
        placa_com_grade(),
        caixa_com_bolso(),
        caixa_arredondada(),
        caixa_casca(),
        flange(),
        eixo_escalonado(),
        engrenagem_40_dentes(),
        fixador_documento("iso4762", "M6", Some(20.)).unwrap(),
        fixador_documento("iso4032", "M8", None).unwrap(),
        chaveta_documento(25., 40.).unwrap(),
        bloco_l(),
    ]
}
