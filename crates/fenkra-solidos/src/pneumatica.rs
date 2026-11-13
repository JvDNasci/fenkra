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

/// Minicilindro redondo ISO 6432 com o curso dado; o parâmetro `d` (furo do cilindro) já existe.
///
/// Eixo no Z, com o centro do pino do olhal traseiro em `z = 0`. Olhal maciço `EW × EW` de `-CD` a
/// `CD` (sem o furo do pino), tubo `Ø D` de `CD` até a face dianteira em `XC - WF`, nariz roscado
/// `Ø BE` por `WF/2`, haste `Ø MM` até o ombro em `XC + curso·s` e a rosca da ponta `Ø KK` por `AM`.
/// O comprimento do nariz não é medida conferida em duas fontes: fica na metade de `WF`.
pub(crate) fn corpo_redondo(m: &mut Montador, fam: &Familia, t: &Tamanho, curso: f64) -> Result<(), String> {
    for c in ["MM", "KK", "AM", "BE", "WF", "XC", "EW", "CD", "D"] {
        m.param(c, &medida(fam, t, c)?.to_string())?;
    }
    let v = |c: &str| t.valor(c).unwrap_or(0.);
    if !(v("D") > v("BE") && v("BE") > v("MM") && v("MM") >= v("KK") && v("XC") > v("WF") + v("CD")) {
        return Err(format!("{} {}: medidas incoerentes (D > BE > MM ≥ KK e XC > WF + CD)", fam.id, t.designacao));
    }
    m.param("curso", &curso.to_string())?;
    m.param("s", "0")?;
    let olhal = m.plano("Plano_olhal", "-CD")?;
    let tubo = m.plano("Plano_tubo", "CD")?;
    let frente = m.plano("Plano_dianteiro", "XC-WF")?;
    let ponta = m.plano("Plano_ombro", "XC+curso*s")?;
    m.prim("Olhal", PrimKind::Caixa, olhal, ["EW", "EW", "2*CD", "0"], BoolOp::Add)?;
    m.prim("Tubo", PrimKind::Cilindro, tubo, ["D/2", "XC-WF-CD", "0", "0"], BoolOp::Add)?;
    m.prim("Nariz", PrimKind::Cilindro, frente.clone(), ["BE/2", "WF/2", "0", "0"], BoolOp::Add)?;
    m.prim("Haste", PrimKind::Cilindro, frente, ["MM/2", "WF+curso*s", "0", "0"], BoolOp::Add)?;
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
        assert_eq!((f.tipo, f.nivel.as_str(), f.fontes.len()), (TipoFixador::CilindroPneumatico, "C", 4));
        // B45: a série inteira da norma, Ø32 a Ø320
        assert_eq!(f.tamanhos.iter().map(|t| t.diametro_nominal().unwrap()).collect::<Vec<_>>(), [32., 40., 50., 63., 80., 100., 125., 160., 200., 250., 320.]);
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2 && t.nota.as_deref().unwrap().contains("concordam")));
        // Ø125 a Ø320: L8 + WH é o ZJ da Tameson (225, 260, 275, 305, 340); KK M27x2 a M48x2
        let grandes: Vec<(f64, f64)> = f.tamanhos[6..].iter().map(|t| (t.valor("L8").unwrap() + t.valor("WH").unwrap(), t.valor("KK").unwrap())).collect();
        assert_eq!(grandes, [(225., 27.), (260., 36.), (275., 36.), (305., 42.), (340., 48.)]);
        let t = tamanho("iso15552", "Ø200").unwrap();
        assert_eq!((t.valor("TG"), t.valor("B"), t.valor("AM"), t.valor("RT"), t.valor("E")), (Some(175.), Some(75.), Some(72.), Some(16.), Some(220.)));
        let t = tamanho("iso15552", "Ø50").unwrap();
        let v = |c: &str| t.valor(c).unwrap();
        assert_eq!((v("MM"), v("KK"), v("P"), v("TG"), v("WH"), v("B"), v("AM"), v("L8"), v("RT")), (20., 16., 1.5, 46.5, 37., 40., 32., 106., 8.));
        // ZJ = L8 + WH nas seis linhas (120, 135, 143, 158, 174, 189 nos dois catálogos)
        let zj: Vec<f64> = f.tamanhos[..6].iter().map(|t| t.valor("L8").unwrap() + t.valor("WH").unwrap()).collect();
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
    fn minicilindro_iso6432_conferido_e_com_cursos_por_tamanho() {
        let f = familia("iso6432").unwrap();
        assert_eq!((f.tipo, f.gerador.as_str(), f.nivel.as_str(), f.fontes.len()), (TipoFixador::CilindroPneumatico, "cilindro_redondo", "C", 3));
        assert_eq!(f.tamanhos.iter().map(|t| t.diametro_nominal().unwrap()).collect::<Vec<_>>(), [8., 10., 12., 16., 20., 25.]);
        assert!(f.tamanhos.iter().all(|t| t.confirmado_por >= 2));
        // tabelas 1, 3 e 4 da norma: AM, XC, WF e a rosca do nariz BE por tamanho
        let col = |c: &str| f.tamanhos.iter().map(|t| t.valor(c).unwrap()).collect::<Vec<_>>();
        assert_eq!(col("AM"), [12., 12., 16., 16., 20., 22.]);
        assert_eq!(col("XC"), [64., 64., 75., 82., 95., 104.]);
        assert_eq!(col("WF"), [16., 16., 22., 22., 24., 28.]);
        assert_eq!(col("BE"), [12., 12., 16., 16., 22., 22.]);
        assert_eq!(col("KK"), [4., 4., 6., 6., 8., 10.]);
        // cursos por tamanho, até o máximo recomendado (100, 200 e 500)
        let maximo = |d: &str| f.limites_de_comprimento(f.tamanho(d).unwrap()).unwrap().1;
        assert_eq!((maximo("Ø8"), maximo("Ø16"), maximo("Ø25")), (100., 200., 500.));
    }

    #[test]
    fn porca_do_nariz_casa_com_a_rosca_do_minicilindro() {
        // fixação MR3: cada rosca BE do cilindro tem a sua porca na biblioteca
        let p = familia("iso6432_mr3").unwrap();
        assert_eq!(p.tipo, TipoFixador::Porca);
        // B17-dados: a porca M22x1,5 fica com chave 32 e altura KW 11 (norma e Aventics), conferida
        let m22 = p.tamanho("M22x1,5").unwrap();
        assert_eq!((m22.valor("s"), m22.valor("m"), m22.confirmado_por), (Some(32.), Some(11.), 2));
        for t in &familia("iso6432").unwrap().tamanhos {
            let be = t.valor("BE").unwrap();
            let porca = p.tamanhos.iter().find(|q| q.diametro_nominal() == Some(be)).unwrap_or_else(|| panic!("{}: sem porca M{be}", t.designacao));
            assert!(porca.valor("s").unwrap() > be && porca.valor("m").unwrap() < t.valor("WF").unwrap() / 2. + 1e-9 || porca.confirmado_por < 2, "{}", t.designacao);
        }
    }

    #[test]
    fn minicilindro_gera_olhal_tubo_nariz_e_haste() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("iso6432").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let doc = fixador_documento("iso6432", &t.designacao, Some(50.)).unwrap();
            let (vol, lo, hi) = caixa(&doc);
            // do fundo do olhal à ponta da rosca, com a haste recolhida
            assert!((lo[2] + v("CD")).abs() < 1e-3 && (hi[2] - v("XC") - v("AM")).abs() < 1e-3, "{}: z de {} a {}", t.designacao, lo[2], hi[2]);
            assert!((hi[0] - lo[0] - v("D")).abs() < 1e-2, "{}", t.designacao);
            let c = |d: f64, h: f64| PI / 4. * d * d * h;
            let esperado = v("EW").powi(2) * 2. * v("CD") + c(v("D"), v("XC") - v("WF") - v("CD")) + c(v("BE"), v("WF") / 2.) + c(v("MM"), v("WF") / 2.) + c(v("KK"), v("AM"));
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
            let mut ext = doc.clone();
            ext.params.iter_mut().find(|p| p.name == "s").unwrap().expr = "1".into();
            let (_, _, hi2) = caixa(&ext);
            assert!((hi2[2] - hi[2] - 50.).abs() < 1e-3, "{}", t.designacao);
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
