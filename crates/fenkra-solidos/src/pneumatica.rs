// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cilindros pneumáticos (B45): modelo simplificado a partir das medidas de interface da família.
//!
//! **Dados** (família `iso15552`): haste `MM`, rosca da ponta `KK` (passo `P`), distância entre os
//! tirantes `TG`, rosca dos tirantes `RT`, ressalto de guia `B`, comprimento do corpo `L8`, distância
//! da face dianteira ao ombro da haste `WH`, comprimento da rosca da ponta `AM` e perfil `E`.
//!
//! **Modelo**: eixo no Z, face traseira em `z = 0`. Corpo quadrado maciço `E × E × L8`, quatro furos
//! `RT` nos cantos do quadrado `TG` em cada tampa (profundidade `2·RT`, simplificação), ressalto `Ø B`
//! na tampa dianteira, haste `Ø MM` até `L8 + WH + curso·s` e a rosca da ponta `Ø KK` por `AM`. O
//! parâmetro `s` (0 a 1) é a posição da haste: 0 recolhida, 1 no fim do curso. O comprimento do
//! ressalto não é medida conferida em duas fontes: fica em `min(4, WH/4)` mm.

use crate::biblioteca::{Familia, Tamanho};
use crate::fixadores::Montador;
use fenkra_core::*;

fn medida(fam: &Familia, t: &Tamanho, c: &str) -> Result<f64, String> {
    t.valor(c).ok_or_else(|| format!("{} {}: falta '{c}'", fam.id, t.designacao))
}

/// Plano paralelo ao XY pela posição (x, y, z), em expressões dos parâmetros; ponto e plano ficam
/// ocultos. Serve às primitivas que nascem na origem do plano (furos fora do eixo).
pub(crate) fn plano_em(m: &mut Montador, nome: &str, x: &str, y: &str, z: &str) -> Result<PlaneRef, String> {
    let p = m.doc.add_geometry(RefKind::PointCoords { x: x.into(), y: y.into(), z: z.into(), frame: None });
    let id = m.doc.add_geometry(RefKind::PlaneParallelPoint { base: PlaneRef::Origin(OriginPlane::XY), point: PointRef::Ref(p) });
    for r in m.doc.references.iter_mut().filter(|r| r.id == p || r.id == id) {
        r.visible = false;
        r.name = if r.id == id { nome.into() } else { format!("{nome}_ponto") };
    }
    Ok(PlaneRef::Datum(id))
}

/// Monta o cilindro com o curso dado; o parâmetro `d` (furo do cilindro) já existe.
pub(crate) fn corpo(m: &mut Montador, fam: &Familia, t: &Tamanho, curso: f64) -> Result<(), String> {
    for c in ["MM", "KK", "TG", "WH", "B", "AM", "L8", "RT", "E"] {
        m.param(c, &medida(fam, t, c)?.to_string())?;
    }
    m.param("curso", &curso.to_string())?;
    m.param("s", "0")?;
    m.param("VD", "min(4; WH/4)")?;
    let base = m.plano("Plano_traseiro", "0")?;
    let frente = m.plano("Plano_dianteiro", "L8")?;
    let ponta = m.plano("Plano_ombro", "L8+WH+curso*s")?;
    m.prim("Corpo", PrimKind::Caixa, base, ["E", "E", "L8", "0"], BoolOp::Add)?;
    // a primitiva cilindro nasce na origem do plano: cada furo tem o seu plano, por um ponto
    for (i, (sx, sy)) in [(1, 1), (-1, 1), (-1, -1), (1, -1)].into_iter().enumerate() {
        let (x, y) = (format!("{sx}*TG/2"), format!("{sy}*TG/2"));
        let tras = plano_em(m, &format!("Plano_furo_traseiro_{}", i + 1), &x, &y, "-1")?;
        let dian = plano_em(m, &format!("Plano_furo_dianteiro_{}", i + 1), &x, &y, "L8-2*RT")?;
        m.prim(&format!("Furo traseiro {}", i + 1), PrimKind::Cilindro, tras, ["RT/2", "2*RT+1", "0", "0"], BoolOp::Cut)?;
        m.prim(&format!("Furo dianteiro {}", i + 1), PrimKind::Cilindro, dian, ["RT/2", "2*RT+1", "0", "0"], BoolOp::Cut)?;
    }
    m.prim("Ressalto", PrimKind::Cilindro, frente.clone(), ["B/2", "VD", "0", "0"], BoolOp::Add)?;
    m.prim("Haste", PrimKind::Cilindro, frente, ["MM/2", "WH+curso*s", "0", "0"], BoolOp::Add)?;
    m.prim("Rosca da ponta", PrimKind::Cilindro, ponta, ["KK/2", "AM", "0", "0"], BoolOp::Add)
}

#[cfg(test)]
mod tests {
    use crate::biblioteca::{familia, tamanho, TipoFixador};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    #[test]
    fn familia_iso15552_conferida_e_com_cursos() {
        let f = familia("iso15552").unwrap();
        assert_eq!((f.tipo, f.nivel.as_str(), f.fontes.len()), (TipoFixador::CilindroPneumatico, "C", 2));
        assert_eq!(f.tamanhos.iter().map(|t| t.diametro_nominal().unwrap()).collect::<Vec<_>>(), [32., 40., 50., 63., 80., 100.]);
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2 && t.nota.as_deref().unwrap().contains("Metalwork e Airwork concordam")));
        let t = tamanho("iso15552", "Ø50").unwrap();
        let v = |c: &str| t.valor(c).unwrap();
        assert_eq!((v("MM"), v("KK"), v("P"), v("TG"), v("WH"), v("B"), v("AM"), v("L8"), v("RT")), (20., 16., 1.5, 46.5, 37., 40., 32., 106., 8.));
        // ZJ = L8 + WH nas seis linhas (120, 135, 143, 158, 174, 189 nos dois catálogos)
        let zj: Vec<f64> = f.tamanhos.iter().map(|t| t.valor("L8").unwrap() + t.valor("WH").unwrap()).collect();
        assert_eq!(zj, [120., 135., 143., 158., 174., 189.]);
        // haste maior que a rosca e furos dentro do perfil
        assert!(f.tamanhos.iter().all(|t| t.valor("MM") > t.valor("KK") && t.valor("TG").unwrap() + t.valor("RT").unwrap() < t.valor("E").unwrap()));
        assert_eq!(f.limites_de_comprimento(t), Some((25., 1000.)));
    }

    fn caixa(doc: &fenkra_core::Document) -> (f64, [f64; 3], [f64; 3]) {
        let rc = recompute(doc);
        assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
        for f in &doc.features {
            assert_eq!(rc.status[&f.id], Status::Ok, "{}", f.name);
        }
        assert_eq!(rc.bodies.len(), 1);
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (rc.bodies[0].mass.volume, lo, hi)
    }

    #[test]
    fn cilindro_gera_corpo_haste_e_segue_o_curso() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("iso15552").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let doc = fixador_documento("iso15552", &t.designacao, Some(160.)).unwrap();
            assert_eq!(doc.props.description, format!("ISO 15552 {}x160", t.designacao));
            let (vol, lo, hi) = caixa(&doc);
            let total = v("L8") + v("WH") + v("AM");
            assert!(lo[2].abs() < 1e-3 && (hi[2] - total).abs() < 1e-3, "{}: z de {} a {}", t.designacao, lo[2], hi[2]);
            assert!((hi[0] - lo[0] - v("E")).abs() < 1e-3 && (hi[1] - lo[1] - v("E")).abs() < 1e-3, "{}", t.designacao);
            let furos = 8. * PI / 4. * v("RT").powi(2) * 2. * v("RT");
            let vd = (v("WH") / 4.).min(4.);
            let esperado = v("E").powi(2) * v("L8") - furos + PI / 4. * (v("B").powi(2) * vd + v("MM").powi(2) * (v("WH") - vd) + v("KK").powi(2) * v("AM"));
            // tolerância apertada: os oito furos precisam estar em lugares distintos (sobrepostos, sobra
            // material); a haste dentro do ressalto não conta duas vezes
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
            // haste no fim do curso: a ponta anda o curso inteiro e o volume ganha a haste
            let mut ext = doc.clone();
            ext.params.iter_mut().find(|p| p.name == "s").unwrap().expr = "1".into();
            let (vol2, _, hi2) = caixa(&ext);
            assert!((hi2[2] - total - 160.).abs() < 1e-3, "{}", t.designacao);
            let haste = PI / 4. * v("MM").powi(2) * 160.;
            assert!((vol2 - vol - haste).abs() / haste < 0.01, "{}", t.designacao);
        }
    }

    #[test]
    fn curso_fora_da_serie_e_recusado_e_sem_pedido_vale_100() {
        assert!(fixador_documento("iso15552", "Ø32", Some(10.)).unwrap_err().contains("curso 10 fora de 25 a 1000"));
        assert!(fixador_documento("iso15552", "Ø32", Some(1200.)).is_err());
        let doc = fixador_documento("iso15552", "Ø32", None).unwrap();
        assert_eq!(doc.props.description, "ISO 15552 Ø32x100");
        // curso livre dentro da faixa, fora dos padrões
        assert!(fixador_documento("iso15552", "Ø40", Some(137.)).is_ok());
    }
}
