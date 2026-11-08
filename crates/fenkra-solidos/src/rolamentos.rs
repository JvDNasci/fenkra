// SPDX-License-Identifier: LGPL-2.1-or-later
//! Rolamentos rígidos de esferas, série 60 (B396): dados de contorno (ISO 15) e o modelo
//! simplificado em três anéis.
//!
//! **Dados**: só as medidas de contorno `d` (furo), `D` (externo) e `B` (largura), que são as mesmas
//! em todos os fabricantes. Uma linha só entra com duas fontes gratuitas que concordam; capacidade
//! de carga é dado de cada fabricante e não entra. Fontes consultadas em 2026-11-07 (ver
//! `docs/arquitetura/procedencia.md`): folha do rolamento 6005 do fabricante SKF, série 6005N do
//! fabricante NSK, listagens de distribuidor (Rubix, RS) para 6005 e 6010.
//!
//! **Modelo**: eixo no Z, centrado em `z = 0`, largura `B`. Pista externa (de `D` até o ombro `D2`),
//! pista interna (do furo até o ombro `d1`) e, entre elas, uma faixa de meia largura que representa o
//! conjunto esferas e gaiola e deixa a peça num corpo só. Os ombros ficam a 1/3 da seção radial
//! `(D - d) / 2` de cada lado (proporção típica da série 60; é simplificação, não dado de norma).
//! Sem canal de pista, esferas, vedação nem raio de canto.

use crate::fixadores::Montador;
use fenkra_core::*;

/// Linha de dados de um rolamento.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rolamento {
    pub designacao: &'static str,
    /// furo (mm)
    pub d: f64,
    /// diâmetro externo (mm)
    pub dext: f64,
    /// largura (mm)
    pub b: f64,
    /// fontes que concordam com as três medidas
    pub fontes: &'static str,
}

/// Rolamentos conferidos em duas fontes.
pub const ROLAMENTOS: &[Rolamento] = &[
    Rolamento { designacao: "6005", d: 25., dext: 47., b: 12., fontes: "SKF (folha do 6005), NSK (6005N), Rubix" },
    Rolamento { designacao: "6010", d: 50., dext: 80., b: 16., fontes: "NSK (6010), Rubix (SKF 6010), RS" },
];

pub fn rolamento(designacao: &str) -> Option<&'static Rolamento> {
    ROLAMENTOS.iter().find(|r| r.designacao.eq_ignore_ascii_case(designacao.trim()))
}

/// Documento do rolamento: parâmetros `d`, `D`, `B` editáveis; mudar um recompõe a peça.
pub fn rolamento_documento(designacao: &str) -> Result<Document, String> {
    let r = rolamento(designacao).ok_or_else(|| format!("rolamento '{designacao}' fora da biblioteca (disponíveis: {})", ROLAMENTOS.iter().map(|x| x.designacao).collect::<Vec<_>>().join(", ")))?;
    let desc = format!("Rolamento {}", r.designacao);
    let mut m = Montador::novo(&desc);
    m.doc.props.part_type = PartType::Hardware;
    m.doc.props.material = "Aço 1020".into();
    m.param("d", &r.d.to_string())?;
    m.param("D", &r.dext.to_string())?;
    m.param("B", &r.b.to_string())?;
    corpo(&mut m)?;
    m.doc.props.custom.push(("Norma".into(), "ISO 15 (medidas de contorno)".into()));
    m.publicar("Eixo", RefKind::LineDir { point: PointRef::Origin, dir: AxisRef::Origin(OriginAxis::Z), start: "-B".into(), end: "0".into() });
    Ok(m.doc)
}

/// Os três anéis do modelo, de `z = -B` a `z = 0` (a face de apoio fica em `z = 0`, como nos
/// fixadores da biblioteca), a partir dos parâmetros `d`, `D` e `B` já criados.
pub(crate) fn corpo(m: &mut Montador) -> Result<(), String> {
    m.param("D2", "D-(D-d)/3")?;
    m.param("d1", "d+(D-d)/3")?;
    let base = m.plano("Plano_base", "-B")?;
    let meio = m.plano("Plano_faixa", "-3*B/4")?;
    let furo = m.plano("Plano_furo", "-B-1")?;
    m.prim("Pista externa", PrimKind::Cilindro, base, ["D/2", "B", "0", "0"], BoolOp::Add)?;
    m.prim("Vão interno", PrimKind::Cilindro, furo, ["D2/2", "B+2", "0", "0"], BoolOp::Cut)?;
    m.prim("Pista interna", PrimKind::Cilindro, base, ["d1/2", "B", "0", "0"], BoolOp::Add)?;
    m.prim("Esferas e gaiola", PrimKind::Cilindro, meio, ["D2/2", "B/2", "0", "0"], BoolOp::Add)?;
    m.prim("Furo", PrimKind::Cilindro, furo, ["d/2", "B+2", "0", "0"], BoolOp::Cut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    #[test]
    fn so_rolamentos_conferidos() {
        assert!(ROLAMENTOS.iter().all(|r| r.fontes.contains(',') && r.d < r.dext && r.b > 0.), "duas fontes por linha");
        assert_eq!(rolamento("6005").map(|r| (r.d, r.dext, r.b)), Some((25., 47., 12.)));
        assert_eq!(rolamento(" 6010 ").map(|r| (r.d, r.dext, r.b)), Some((50., 80., 16.)));
        assert!(rolamento_documento("9999").unwrap_err().contains("6005"));
    }

    /// B396: a mesma peça sai da biblioteca de normas (família `iso15_60`), apoiada em z = 0.
    #[test]
    fn rolamento_da_biblioteca_tem_as_medidas_e_apoio_em_zero() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for r in ROLAMENTOS {
            let doc = crate::fixadores::fixador_documento("iso15_60", r.designacao, None).unwrap();
            assert_eq!(doc.props.description, format!("ISO 15 {}", r.designacao));
            let rc = recompute(&doc);
            assert!(doc.features.iter().all(|f| rc.status[&f.id] == Status::Ok), "{}", r.designacao);
            assert_eq!(rc.bodies.len(), 1);
            let z: Vec<f64> = rc.bodies[0].mesh.positions.chunks_exact(3).map(|p| p[2] as f64).collect();
            let (lo, hi) = (z.iter().cloned().fold(f64::MAX, f64::min), z.iter().cloned().fold(f64::MIN, f64::max));
            assert!((lo + r.b).abs() < 1e-3 && hi.abs() < 1e-3, "{}: z de {lo} a {hi}", r.designacao);
            let (d2, d1) = (r.dext - (r.dext - r.d) / 3., r.d + (r.dext - r.d) / 3.);
            let esperado = PI / 4. * ((r.dext * r.dext - d2 * d2) * r.b + (d1 * d1 - r.d * r.d) * r.b + (d2 * d2 - d1 * d1) * r.b / 2.);
            let v = rc.bodies[0].mass.volume;
            assert!((v - esperado).abs() / esperado < 0.01, "{}: volume {v} contra {esperado}", r.designacao);
        }
    }

    #[test]
    fn rolamento_gera_um_corpo_com_as_medidas_de_contorno() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for r in ROLAMENTOS {
            let doc = rolamento_documento(r.designacao).unwrap();
            let rc = recompute(&doc);
            assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
            for f in &doc.features {
                assert_eq!(rc.status[&f.id], Status::Ok, "{} / {}", r.designacao, f.name);
            }
            assert_eq!(rc.bodies.len(), 1, "{}: um corpo só", r.designacao);
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i] as f64);
                    hi[i] = hi[i].max(p[i] as f64);
                }
            }
            assert!((hi[2] - lo[2] - r.b).abs() < 1e-3, "{}: largura {lo:?} {hi:?}", r.designacao);
            assert!((hi[0] - lo[0] - r.dext).abs() < 0.2 && (hi[1] - lo[1] - r.dext).abs() < 0.2, "{}: externo", r.designacao);
            let (d2, d1) = (r.dext - (r.dext - r.d) / 3., r.d + (r.dext - r.d) / 3.);
            let esperado = PI / 4. * ((r.dext * r.dext - d2 * d2) * r.b + (d1 * d1 - r.d * r.d) * r.b + (d2 * d2 - d1 * d1) * r.b / 2.);
            let v = rc.bodies[0].mass.volume;
            assert!((v - esperado).abs() / esperado < 0.01, "{}: volume {v} contra {esperado}", r.designacao);
        }
    }
}
