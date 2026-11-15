// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fixações de cilindro pneumático ISO 15552 (B45): pé, flange, munhão e garfo traseiro.
//!
//! São peças soltas, modeladas só pelas cotas de interface do catálogo (o contorno de fundição é
//! simplificado). Eixo do cilindro no Z; a face que apoia no cilindro (ou, no pé, a face traseira da
//! chapa) fica em `z = 0`.
//!
//! - **Pé** (`fixacao_pe`, MS1): chapa de apoio `US × AT × AU` em `y = -AH` (AH é a distância do apoio
//!   ao eixo) e perna `US × AT` até `y = US/2`, com dois furos `Ø AB` afastados `TR` na chapa. O
//!   contorno superior da perna é aproximado pela metade da largura: o catálogo não dá a altura da perna.
//! - **Flange** (`fixacao_flange`, MF1/MF2): chapa quadrada `UF × UF × MF` com quatro furos `Ø FB` em
//!   quadrado de lado `TF`.
//! - **Munhão** (`fixacao_munhao`, MT): corpo `TM × US × TK` com dois munhões `Ø TD` de comprimento
//!   `TL`, um de cada lado, no meio da espessura.
//! - **Garfo traseiro** (`fixacao_garfo`, MP2): corpo `UB × UB × (FL + MR)`, rasgo de largura `CB` a
//!   partir de `z = L` (onde entra o olhal) e furo do pino `Ø CD` em `z = FL`, só nas abas. A ponta
//!   arredondada de raio `MR` é simplificada em canto vivo.

use crate::biblioteca::{Familia, Tamanho};
use crate::fixadores::Montador;
use fenkra_core::*;

fn medida(fam: &Familia, t: &Tamanho, c: &str) -> Result<f64, String> {
    t.valor(c).ok_or_else(|| format!("{} {}: falta '{c}'", fam.id, t.designacao))
}

/// Plano paralelo ao `base`, com a origem no ponto (x, y, z) dado em expressões; oculto na viewport.
fn plano_base(m: &mut Montador, nome: &str, base: OriginPlane, x: &str, y: &str, z: &str) -> PlaneRef {
    let p = m.doc.add_geometry(RefKind::PointCoords { x: x.into(), y: y.into(), z: z.into(), frame: None });
    let id = m.doc.add_geometry(RefKind::PlaneParallelPoint { base: PlaneRef::Origin(base), point: PointRef::Ref(p) });
    for r in m.doc.references.iter_mut().filter(|r| r.id == p || r.id == id) {
        r.visible = false;
        r.name = if r.id == id { nome.into() } else { format!("{nome}_ponto") };
    }
    PlaneRef::Datum(id)
}

fn xy(m: &mut Montador, nome: &str, x: &str, y: &str, z: &str) -> PlaneRef {
    plano_base(m, nome, OriginPlane::XY, x, y, z)
}

fn yz(m: &mut Montador, nome: &str, x: &str, y: &str, z: &str) -> PlaneRef {
    plano_base(m, nome, OriginPlane::YZ, x, y, z)
}

/// Monta a fixação da família (pelo gerador) com as cotas do tamanho.
pub(crate) fn corpo(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    let campos: &[&str] = match fam.gerador.as_str() {
        "fixacao_pe" => &["AB", "AH", "AT", "AU", "TR", "US"],
        "fixacao_flange" => &["FB", "MF", "TF", "UF"],
        "fixacao_munhao" => &["TD", "TK", "TL", "TM", "US"],
        "fixacao_garfo" => &["CD", "CB", "FL", "L", "MR", "UB"],
        g => return Err(format!("{}: gerador '{g}' sem geometria de fixação", fam.id)),
    };
    for c in campos {
        m.param(c, &medida(fam, t, c)?.to_string())?;
    }
    let v = |c: &str| t.valor(c).unwrap_or(0.);
    match fam.gerador.as_str() {
        "fixacao_pe" => {
            if !(v("AT") < v("AH") && v("TR") + v("AB") < v("US") && v("AB") < v("AU")) {
                return Err(format!("{} {}: medidas incoerentes (AT < AH, TR + AB < US, AB < AU)", fam.id, t.designacao));
            }
            let placa = xy(m, "Plano_chapa", "0", "-AH+AT/2", "0");
            m.prim("Chapa de apoio", PrimKind::Caixa, placa, ["US", "AT", "AU", "0"], BoolOp::Add)?;
            let perna = xy(m, "Plano_perna", "0", "(-AH+AT+US/2)/2", "0");
            m.prim("Perna", PrimKind::Caixa, perna, ["US", "US/2+AH-AT", "AT", "0"], BoolOp::Add)?;
            // furos verticais (eixo Y) na chapa: o plano XZ tem a normal para -Y, então o furo desce de y = -AH+AT+1
            for (i, s) in [1, -1].into_iter().enumerate() {
                let p = plano_base(m, &format!("Plano_furo_{}", i + 1), OriginPlane::XZ, &format!("{s}*TR/2"), "-AH+AT+1", "AU/2");
                m.prim(&format!("Furo {}", i + 1), PrimKind::Cilindro, p, ["AB/2", "AT+2", "0", "0"], BoolOp::Cut)?;
            }
        }
        "fixacao_flange" => {
            if !(v("TF") + v("FB") < v("UF")) {
                return Err(format!("{} {}: medidas incoerentes (TF + FB < UF)", fam.id, t.designacao));
            }
            let base = xy(m, "Plano_apoio", "0", "0", "0");
            m.prim("Flange", PrimKind::Caixa, base, ["UF", "UF", "MF", "0"], BoolOp::Add)?;
            for (i, (sx, sy)) in [(1, 1), (-1, 1), (-1, -1), (1, -1)].into_iter().enumerate() {
                let p = xy(m, &format!("Plano_furo_{}", i + 1), &format!("{sx}*TF/2"), &format!("{sy}*TF/2"), "-1");
                m.prim(&format!("Furo {}", i + 1), PrimKind::Cilindro, p, ["FB/2", "MF+2", "0", "0"], BoolOp::Cut)?;
            }
        }
        "fixacao_munhao" => {
            if !(v("TD") < v("TK") * 1.5 && v("TD") < v("US")) {
                return Err(format!("{} {}: medidas incoerentes (TD < 1,5·TK e TD < US)", fam.id, t.designacao));
            }
            let base = xy(m, "Plano_apoio", "0", "0", "0");
            m.prim("Corpo", PrimKind::Caixa, base, ["TM", "US", "TK", "0"], BoolOp::Add)?;
            let dir = yz(m, "Plano_munhao_direito", "TM/2", "0", "TK/2");
            m.prim("Munhão direito", PrimKind::Cilindro, dir, ["TD/2", "TL", "0", "0"], BoolOp::Add)?;
            let esq = yz(m, "Plano_munhao_esquerdo", "-TM/2-TL", "0", "TK/2");
            m.prim("Munhão esquerdo", PrimKind::Cilindro, esq, ["TD/2", "TL", "0", "0"], BoolOp::Add)?;
        }
        _ => {
            // garfo
            if !(v("CB") < v("UB") && v("FL") - v("CD") / 2. >= v("L") && v("CD") < v("UB") && v("MR") > 0.) {
                return Err(format!("{} {}: medidas incoerentes (CB < UB, o pino acima do fundo do rasgo)", fam.id, t.designacao));
            }
            let base = xy(m, "Plano_apoio", "0", "0", "0");
            m.prim("Corpo", PrimKind::Caixa, base, ["UB", "UB", "FL+MR", "0"], BoolOp::Add)?;
            let rasgo = xy(m, "Plano_rasgo", "0", "0", "L");
            m.prim("Rasgo", PrimKind::Caixa, rasgo, ["CB", "UB+2", "FL+MR-L+1", "0"], BoolOp::Cut)?;
            let pino = yz(m, "Plano_pino", "-UB/2-1", "0", "FL");
            m.prim("Furo do pino", PrimKind::Cilindro, pino, ["CD/2", "UB+2", "0", "0"], BoolOp::Cut)?;
        }
    }
    Ok(())
}

/// Exemplo de camada de fabricante (B45): a série própria do cilindro DNC de um fabricante, com as
/// medidas do catálogo dele e a faixa de cursos de 10 a 2000 mm, como camada `Empresa` sobre a
/// pública. Não entra na biblioteca embutida: é o modelo de arquivo que uma empresa grava na sua pasta.
pub fn camada_fabricante_exemplo() -> Result<crate::biblioteca::Biblioteca, String> {
    crate::biblioteca::Biblioteca::de_jsons(crate::biblioteca::Camada::Empresa, &[("empresa_festo_dnc.json", include_str!("../../../biblioteca/camadas_exemplo/empresa_festo_dnc.json"))])
}

#[cfg(test)]
mod tests {
    use crate::biblioteca::{familia, TipoFixador};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    const FAMILIAS: [&str; 4] = ["iso15552_pe", "iso15552_flange", "iso15552_munhao", "iso15552_garfo"];

    fn medir(doc: &fenkra_core::Document) -> (f64, [f64; 3], [f64; 3]) {
        let rc = recompute(doc);
        assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
        for f in &doc.features {
            assert_eq!(rc.status[&f.id], Status::Ok, "{} / {}", doc.name, f.name);
        }
        assert_eq!(rc.bodies.len(), 1, "{}", doc.name);
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in rc.bodies[0].mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (rc.bodies[0].mass.volume, lo, hi)
    }

    /// As quatro famílias cobrem Ø32 a Ø125, são do tipo certo e dizem de onde vem cada cota.
    #[test]
    fn familias_de_fixacao_do_catalogo() {
        for id in FAMILIAS {
            let f = familia(id).unwrap_or_else(|| panic!("falta {id}"));
            assert_eq!(f.tipo, TipoFixador::FixacaoCilindro, "{id}");
            assert_eq!(f.norma, "ISO 15552");
            assert_eq!(f.tamanhos.iter().map(|t| t.diametro_nominal().unwrap()).collect::<Vec<_>>(), [32., 40., 50., 63., 80., 100., 125.], "{id}");
            // pé e garfo trazem as duas fontes; flange e munhão uma só (nível B), sem se passar por conferidos
            let duas = id.ends_with("_pe") || id.ends_with("_garfo");
            assert_eq!(f.nivel, if duas { "C" } else { "B" }, "{id}");
            assert!(f.tamanhos.iter().all(|t| t.confirmado_por == if duas { 2 } else { 1 }), "{id}");
            assert!(f.fontes.len() >= if duas { 2 } else { 1 });
        }
    }

    /// Cotas de pé e garfo contra a segunda tabela (alumínio, outro fabricante): AH e TR do pé, e CD,
    /// CB, FL e MR do garfo coincidem nas sete bitolas; o que só uma fonte traz fica dito na nota.
    #[test]
    fn pe_e_garfo_batem_com_a_segunda_tabela() {
        // pé alto (colunas C e F da tabela): par {AH, TR}
        let pe = [(32., 32., 32.), (40., 36., 36.), (50., 45., 45.), (63., 50., 50.), (80., 63., 63.), (100., 75., 71.), (125., 90., 91.)];
        let f = familia("iso15552_pe").unwrap();
        for (d, c, fcol) in pe {
            let t = f.tamanho(&format!("Ø{d}")).unwrap();
            let (ah, tr) = (t.valor("AH").unwrap(), t.valor("TR").unwrap());
            // o par aparece nos dois fabricantes; o pé de 125 da segunda tabela tem 91 contra 90 (tolerância de 1 mm)
            assert!(((ah - c).abs() <= 1. && (tr - fcol).abs() <= 1.) || ((tr - c).abs() <= 1. && (ah - fcol).abs() <= 1.), "{d}: AH {ah}, TR {tr} contra {c}, {fcol}");
        }
        // garfo MP2: (CD, CB, FL, MR) da segunda tabela e do olhal do primeiro catálogo
        let garfo = [(32., [10., 26., 22., 10.]), (40., [12., 28., 25., 12.]), (50., [12., 32., 27., 12.]), (63., [16., 40., 32., 16.]), (80., [16., 50., 36., 16.]), (100., [20., 60., 41., 20.]), (125., [25., 70., 50., 25.])];
        let f = familia("iso15552_garfo").unwrap();
        for (d, esp) in garfo {
            let t = f.tamanho(&format!("Ø{d}")).unwrap();
            let got = ["CD", "CB", "FL", "MR"].map(|c| t.valor(c).unwrap());
            assert_eq!(got, esp, "Ø{d}");
        }
    }

    /// Pé: volume exato (chapa + perna − 2 furos) e caixa envolvente; 7 tamanhos.
    #[test]
    fn pe_gera_chapa_perna_e_furos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("iso15552_pe").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let doc = fixador_documento("iso15552_pe", &t.designacao, None).unwrap();
            let (vol, lo, hi) = medir(&doc);
            let esperado = v("US") * v("AT") * v("AU") + v("US") * (v("US") / 2. + v("AH") - v("AT")) * v("AT") - 2. * PI / 4. * v("AB").powi(2) * v("AT");
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
            assert!((hi[0] - lo[0] - v("US")).abs() < 1e-3 && (lo[1] + v("AH")).abs() < 1e-3 && (hi[1] - v("US") / 2.).abs() < 1e-3, "{}: {lo:?} {hi:?}", t.designacao);
            assert!(lo[2].abs() < 1e-3 && (hi[2] - v("AU")).abs() < 1e-3, "{}", t.designacao);
        }
    }

    #[test]
    fn flange_gera_chapa_com_quatro_furos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("iso15552_flange").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let doc = fixador_documento("iso15552_flange", &t.designacao, None).unwrap();
            let (vol, lo, hi) = medir(&doc);
            let esperado = (v("UF").powi(2) - 4. * PI / 4. * v("FB").powi(2)) * v("MF");
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
            assert!((hi[0] - lo[0] - v("UF")).abs() < 1e-3 && (hi[1] - lo[1] - v("UF")).abs() < 1e-3 && lo[2].abs() < 1e-3 && (hi[2] - v("MF")).abs() < 1e-3, "{}", t.designacao);
        }
    }

    #[test]
    fn munhao_gera_corpo_e_dois_munhoes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("iso15552_munhao").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let doc = fixador_documento("iso15552_munhao", &t.designacao, None).unwrap();
            let (vol, lo, hi) = medir(&doc);
            let esperado = v("TM") * v("US") * v("TK") + 2. * PI / 4. * v("TD").powi(2) * v("TL");
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
            let total = v("TM") + 2. * v("TL");
            assert!((hi[0] - lo[0] - total).abs() < 1e-3 && (hi[0] + lo[0]).abs() < 1e-3, "{}: x de {} a {}", t.designacao, lo[0], hi[0]);
            assert!((hi[1] - lo[1] - v("US")).abs() < 1e-3 && lo[2].abs() < 1e-3 && (hi[2] - v("TK")).abs() < 1e-3, "{}", t.designacao);
        }
    }

    #[test]
    fn garfo_gera_corpo_rasgo_e_furo_do_pino() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("iso15552_garfo").unwrap().tamanhos {
            let v = |c: &str| t.valor(c).unwrap();
            let doc = fixador_documento("iso15552_garfo", &t.designacao, None).unwrap();
            let (vol, lo, hi) = medir(&doc);
            let comp = v("FL") + v("MR");
            let esperado = v("UB") * v("UB") * comp - v("CB") * v("UB") * (comp - v("L")) - PI / 4. * v("CD").powi(2) * (v("UB") - v("CB"));
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
            assert!((hi[0] - lo[0] - v("UB")).abs() < 1e-3 && (hi[1] - lo[1] - v("UB")).abs() < 1e-3 && lo[2].abs() < 1e-3 && (hi[2] - comp).abs() < 1e-3, "{}", t.designacao);
        }
    }

    /// Camada de fabricante: a série do catálogo entra por cima da pública (mesclada), traz cursos de 10
    /// a 2000, gera o cilindro com as medidas dele e deixa a família ISO intacta.
    #[test]
    fn camada_de_fabricante_soma_a_serie_propria() {
        use crate::biblioteca::{Biblioteca, Camada};
        let empresa = super::camada_fabricante_exemplo().unwrap();
        let publica = Biblioteca::de_jsons(Camada::Publica, &[("iso15552.json", include_str!("../../../biblioteca/pneumatica/iso15552.json"))]).unwrap();
        let m = Biblioteca::mesclar(&[publica.clone(), empresa]);
        let (iso, dnc) = (m.familia("iso15552").unwrap(), m.familia("festo_dnc").unwrap());
        assert_eq!((iso.camada, dnc.camada), (Camada::Publica, Camada::Empresa));
        assert_eq!(&iso.tamanhos, &publica.familia("iso15552").unwrap().tamanhos);
        // ZJ = L8 + WH conferido no catálogo (120, 135, 143, 158, 174, 189, 225) e a faixa de cursos
        let zj: Vec<f64> = dnc.tamanhos.iter().map(|t| t.valor("L8").unwrap() + t.valor("WH").unwrap()).collect();
        assert_eq!(zj, [120., 135., 143., 158., 174., 189., 225.]);
        let t = dnc.tamanho("Ø63").unwrap();
        assert_eq!(dnc.limites_de_comprimento(t), Some((10., 2000.)));
        // o cilindro da camada sai com o perfil do fabricante (E = 75 no Ø63) e curso de 10 mm, que a norma recusa
        assert!(crate::fixadores::fixador_documento("iso15552", "Ø63", Some(10.)).is_err());
        if fenkra_kernel::AVAILABLE {
            let doc = crate::fixadores::gerar(dnc, "Ø63", Some(10.)).unwrap();
            let (_, lo, hi) = medir(&doc);
            assert!((hi[0] - lo[0] - 75.).abs() < 1e-3 && (hi[2] - (121. + 37. + 32.)).abs() < 1e-3, "{lo:?} {hi:?}");
        }
    }
}
