// SPDX-License-Identifier: LGPL-2.1-or-later
//! Leitura dos rasgos de chaveta de uma peça (B292).
//!
//! O comando Chaveta parte de duas peças prontas, o eixo e o cubo, e precisa saber onde está cada
//! rasgo sem pedir nada à pessoa. Como funciona: os geradores do Fenkra (`eixos`, `eixos_recursos`,
//! `engrenagens`) cortam o rasgo com uma extrusão cega chamada `Rasgo_chaveta*`, a partir de um plano
//! paralelo ao YZ (deslocado por `offset`, e girado em torno de Z quando o rasgo tem rotação), para o
//! lado +normal. Aqui a leitura é o caminho inverso: acha essas extrusões, avalia o deslocamento do
//! plano e a profundidade com os parâmetros da peça e lê a largura e a faixa em Z do contorno do
//! sketch (estádio com pontas arredondadas no eixo; retângulo no cubo).
//!
//! Medidas devolvidas são distâncias ao eixo Z da peça: `piso` é onde o corte começa (o fundo do
//! rasgo do eixo, a `d/2 - t1`) e `teto` onde termina (o fundo do rasgo do cubo, a `furo/2 + t2`).

use fenkra_core::sketch::Curve;
use fenkra_core::*;
use std::collections::HashMap;

/// Um rasgo de chaveta lido do documento de uma peça.
#[derive(Clone, Debug, PartialEq)]
pub struct RasgoLido {
    pub nome: String,
    /// largura do rasgo (b)
    pub b: f64,
    /// distância do eixo da peça até o início do corte (fundo do rasgo no eixo)
    pub piso: f64,
    /// distância do eixo da peça até o fim do corte (fundo do rasgo no cubo)
    pub teto: f64,
    /// giro do rasgo em torno de Z, em graus, a partir de +X
    pub rotacao: f64,
    /// faixa em Z (coordenada local da peça), com as pontas arredondadas incluídas
    pub z0: f64,
    pub z1: f64,
}

impl RasgoLido {
    pub fn comprimento(&self) -> f64 {
        self.z1 - self.z0
    }
    /// Direção radial do rasgo no plano XY local da peça.
    pub fn direcao(&self) -> [f64; 2] {
        let r = self.rotacao.to_radians();
        [r.cos(), r.sin()]
    }
}

fn avaliar(expr: &str, vars: &HashMap<String, f64>) -> Option<f64> {
    expr::eval_str(expr, vars).ok().filter(|v| v.is_finite())
}

/// Plano do sketch como (rotação em graus, deslocamento ao longo da normal); só os planos que os
/// geradores usam (YZ, deslocado e/ou girado em torno de Z).
fn plano_do_rasgo(doc: &Document, p: PlaneRef, vars: &HashMap<String, f64>) -> Option<(f64, f64)> {
    match p {
        PlaneRef::Origin(OriginPlane::YZ) => Some((0., 0.)),
        PlaneRef::Origin(_) => None,
        PlaneRef::Datum(id) => match &doc.references.iter().find(|r| r.id == id)?.kind {
            RefKind::OffsetPlane { base, offset } => {
                let (rot, desl) = plano_do_rasgo(doc, *base, vars)?;
                Some((rot, desl + avaliar(offset, vars)?))
            }
            RefKind::AnglePlane { base: PlaneRef::Origin(OriginPlane::YZ), axis: AxisRef::Origin(OriginAxis::Z), angle } => Some((avaliar(angle, vars)?, 0.)),
            _ => None,
        },
    }
}

/// Todos os rasgos de chaveta do documento (extrusões de corte cujo nome começa com `Rasgo_chaveta`),
/// em ordem de posição ao longo de Z. Rasgo que a leitura não entende (plano ou contorno fora do
/// padrão dos geradores) fica de fora.
pub fn rasgos_do_documento(doc: &Document) -> Vec<RasgoLido> {
    let Ok(vars) = doc.eval_params() else { return vec![] };
    let mut out = vec![];
    for f in doc.features.iter().filter(|f| !f.suppressed && f.name.starts_with("Rasgo_chaveta")) {
        let FeatureKind::Extrude(x) = &f.kind else { continue };
        if x.op != BoolOp::Cut || !x.reverse {
            continue;
        }
        let Extent::Blind(prof) = &x.extent else { continue };
        let Some(prof) = avaliar(prof, &vars) else { continue };
        let Some(FeatureKind::Sketch(sk)) = doc.features.iter().find(|g| g.id == x.sketch).map(|g| &g.kind) else { continue };
        let Some((rotacao, desloc)) = plano_do_rasgo(doc, sk.plane, &vars) else { continue };
        if sk.data.points.is_empty() {
            continue;
        }
        let (mut xa, mut xb, mut ya, mut yb) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in &sk.data.points {
            xa = xa.min(p.x);
            xb = xb.max(p.x);
            ya = ya.min(p.y);
            yb = yb.max(p.y);
        }
        let b = xb - xa;
        // estádio: os pontos são os centros das pontas e as extremidades dos arcos, todos a meia
        // largura das pontas; o rasgo vai um raio além deles
        let arredondado = sk.data.curves.iter().any(|c| matches!(c.geom, Curve::Arc { .. }));
        let (z0, z1) = if arredondado { (ya - b / 2., yb + b / 2.) } else { (ya, yb) };
        out.push(RasgoLido { nome: f.name.clone(), b, piso: desloc, teto: desloc + prof, rotacao, z0, z1 });
    }
    out.sort_by(|a, b| a.z0.total_cmp(&b.z0));
    out
}

/// Diâmetro do trecho do eixo (`d1`, `l1`, `d2`, `l2`...) que contém a coordenada `z`; `None` se a
/// peça não é um eixo escalonado do Fenkra.
pub fn diametro_do_trecho(doc: &Document, z: f64) -> Option<f64> {
    let vars = doc.eval_params().ok()?;
    let mut inicio = 0.;
    for k in 1.. {
        let (d, l) = (vars.get(&format!("d{k}"))?, vars.get(&format!("l{k}"))?);
        if z >= inicio - 1e-9 && z <= inicio + l + 1e-9 {
            return Some(*d);
        }
        inicio += l;
    }
    None
}

/// Diâmetro do furo do cubo (parâmetro `furo` das engrenagens).
pub fn furo_do_cubo(doc: &Document) -> Option<f64> {
    doc.eval_params().ok()?.get("furo").copied().filter(|d| *d > 0.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eixos::{eixo_documento, ParamEixo, RasgoChaveta, TrechoEixo};
    use crate::eixos_recursos::{eixo_documento_com_recursos, Borda, RecursoEixo, TipoRecurso};
    use crate::engrenagens::{engrenagem_documento, ParamEngrenagem};

    fn eixo() -> ParamEixo {
        ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None },
                TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: Some(RasgoChaveta { comprimento: 40., desloc_inicio: 10. }) },
            ],
            furo_centro: false,
        }
    }

    #[test]
    fn le_o_rasgo_do_trecho_do_eixo() {
        let d = eixo_documento(&eixo()).unwrap();
        let r = rasgos_do_documento(&d);
        assert_eq!(r.len(), 1);
        let r = &r[0];
        // Ø25: b 8, t1 4,0; piso a 12,5 - 4 = 8,5; trecho 2 começa em z = 50, rasgo de 60 a 100
        assert!((r.b - 8.).abs() < 1e-9 && (r.piso - 8.5).abs() < 1e-9, "{r:?}");
        assert!((r.z0 - 60.).abs() < 1e-9 && (r.z1 - 100.).abs() < 1e-9, "{r:?}");
        assert!(r.rotacao.abs() < 1e-9 && (r.teto - 8.5 - 5.).abs() < 1e-9, "{r:?}");
        assert_eq!(diametro_do_trecho(&d, 80.), Some(25.));
        assert_eq!(diametro_do_trecho(&d, 10.), Some(20.));
        assert_eq!(diametro_do_trecho(&d, 500.), None);
    }

    #[test]
    fn le_o_rasgo_girado_de_um_recurso() {
        let p = ParamEixo { trechos: vec![TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None }], furo_centro: false };
        let rec = RecursoEixo { trecho: 0, borda: Borda::Esquerda, posicao: 15., tipo: TipoRecurso::Rasgo { comprimento: 30., rotacao: 90. } };
        let d = eixo_documento_com_recursos(&p, &[rec]).unwrap();
        let r = rasgos_do_documento(&d);
        assert_eq!(r.len(), 1);
        assert!((r[0].rotacao - 90.).abs() < 1e-9 && (r[0].z0 - 15.).abs() < 1e-9 && (r[0].z1 - 45.).abs() < 1e-9, "{:?}", r[0]);
        assert!((r[0].piso - 8.5).abs() < 1e-9);
    }

    #[test]
    fn le_o_rasgo_do_cubo_da_engrenagem() {
        let p = ParamEngrenagem { furo: 25., largura: 20., rasgo_cubo: true, dentes: 40, ..Default::default() };
        let d = engrenagem_documento(&p).unwrap();
        let r = rasgos_do_documento(&d);
        assert_eq!(r.len(), 1, "{r:?}");
        // furo Ø25: b 8, t2 3,3; o corte vai de furo/2 - 1 até furo/2 + t2
        assert!((r[0].b - 8.).abs() < 1e-9 && (r[0].teto - (12.5 + 3.3)).abs() < 1e-9, "{:?}", r[0]);
        assert!(r[0].z0 < 0. && r[0].z1 > 20., "o rasgo atravessa a largura: {:?}", r[0]);
        assert_eq!(furo_do_cubo(&d), Some(25.));
    }
}
