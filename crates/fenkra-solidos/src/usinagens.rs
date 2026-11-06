// SPDX-License-Identifier: LGPL-2.1-or-later
//! Usinagens editáveis sobre o elemento de catálogo (B345): furo do eixo, rasgo de chaveta, cubo,
//! rebaixo, furos de alívio e parafuso de fixação.
//!
//! O elemento de transmissão (engrenagem, roda de corrente...) sai da biblioteca com a geometria
//! **padronizada** e uma lista de [`Usinagem`] que a pessoa adapta. Cada usinagem vira comandos
//! nomeados e paramétricos no documento (`Usinagem_<n>_...`), então dá para editar a medida na árvore
//! da peça ou trocar a lista e gerar de novo. O vínculo com o catálogo fica em `props.custom`:
//!
//! - `catalogo`: a chave da peça padronizada (o que a biblioteca gera sem usinagem);
//! - `usinagens`: a lista, em JSON, que reproduz a peça (ver [`vinculo_catalogo`]).
//!
//! Apagar a lista devolve a peça padronizada. Geometria: o eixo é Z, o disco vai de `z = 0` a
//! `z = largura`; o **cubo** é um cilindro acima do disco (`z = largura` a `largura + comprimento`);
//! o furo e o rasgo atravessam disco e cubo; o rebaixo entra pela face inferior (`z = 0`).

use crate::biblioteca::chaveta_para_eixo;
use crate::eixos::{extrusao_cega, plano_yz};
use crate::fixadores::Montador;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Usinagem {
    /// furo do eixo (mm), atravessando disco e cubo; vale o maior entre ele e o furo do catálogo
    FuroEixo { diametro: f64 },
    /// rasgo de chaveta DIN 6885 forma A pelo diâmetro do furo, em +X, ao longo de disco e cubo
    RasgoChaveta,
    /// cubo cilíndrico acima do disco
    Cubo { diametro: f64, comprimento: f64 },
    /// rebaixo cilíndrico na face inferior
    Rebaixo { diametro: f64, profundidade: f64 },
    /// furos de alívio no disco, em círculo de raio `raio_circulo`
    FurosAlivio { quantidade: u32, diametro: f64, raio_circulo: f64 },
    /// furo de parafuso de fixação (macho) radial no cubo, no giro dado a partir de +X; `rosca` é o
    /// diâmetro nominal métrico (o furo sai com o diâmetro de broca de macho `d − passo`)
    ParafusoFixacao { rosca: f64, rotacao: f64 },
}

impl Usinagem {
    pub const NOMES: [&'static str; 6] = ["Furo do eixo", "Rasgo de chaveta", "Cubo", "Rebaixo", "Furos de alívio", "Parafuso de fixação"];

    pub fn nome(&self) -> &'static str {
        match self {
            Usinagem::FuroEixo { .. } => Self::NOMES[0],
            Usinagem::RasgoChaveta => Self::NOMES[1],
            Usinagem::Cubo { .. } => Self::NOMES[2],
            Usinagem::Rebaixo { .. } => Self::NOMES[3],
            Usinagem::FurosAlivio { .. } => Self::NOMES[4],
            Usinagem::ParafusoFixacao { .. } => Self::NOMES[5],
        }
    }

    /// Usinagem nova com valores razoáveis para o disco `g`.
    pub fn padrao(nome: &str, g: &GeoBase) -> Usinagem {
        let furo = g.furo.max(8.);
        match nome {
            "Rasgo de chaveta" => Usinagem::RasgoChaveta,
            "Cubo" => Usinagem::Cubo { diametro: (furo * 1.8).max(furo + 6.), comprimento: (furo * 0.8).max(6.) },
            "Rebaixo" => Usinagem::Rebaixo { diametro: furo * 1.6, profundidade: (g.largura / 3.).max(1.) },
            "Furos de alívio" => {
                let meio = (g.raio_raiz + (furo * 1.8) / 2.) / 2.;
                Usinagem::FurosAlivio { quantidade: 4, diametro: ((g.raio_raiz - furo * 0.9) * 0.5).clamp(2., 20.), raio_circulo: meio }
            }
            "Parafuso de fixação" => Usinagem::ParafusoFixacao { rosca: (furo / 3.).round().max(3.), rotacao: 90. },
            _ => Usinagem::FuroEixo { diametro: furo },
        }
    }
}

/// O que a peça padronizada oferece às usinagens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoBase {
    /// espessura do disco (z de 0 a largura)
    pub largura: f64,
    /// raio da raiz dos dentes: o alívio precisa ficar abaixo dele
    pub raio_raiz: f64,
    /// furo do catálogo (0 = sem furo)
    pub furo: f64,
    /// a peça já tem cubo (roda de corrente), com (comprimento, diâmetro): `Cubo` é recusado
    pub cubo_proprio: Option<(f64, f64)>,
}

fn rosca_passo(d: f64) -> Option<f64> {
    crate::eixos_recursos::passo_grosso(d)
}

/// Diâmetro da broca de macho de uma rosca métrica de passo grosso: `d − P`.
pub fn diametro_furo_de_macho(rosca: f64) -> Option<f64> {
    rosca_passo(rosca).map(|p| rosca - p)
}

/// Dimensões efetivas depois das usinagens: (furo final, comprimento do cubo, diâmetro do cubo).
fn efetivas(g: &GeoBase, lista: &[Usinagem]) -> (f64, f64, f64) {
    let mut furo = g.furo;
    let (mut cubo_l, mut cubo_d) = g.cubo_proprio.unwrap_or((0., 0.));
    for u in lista {
        match *u {
            Usinagem::FuroEixo { diametro } => furo = furo.max(diametro),
            Usinagem::Cubo { diametro, comprimento } => {
                cubo_l = comprimento;
                cubo_d = diametro;
            }
            _ => {}
        }
    }
    (furo, cubo_l, cubo_d)
}

/// Confere a lista contra a peça: erro em português com o que corrigir.
pub fn validar(g: &GeoBase, lista: &[Usinagem]) -> Result<(), String> {
    let (furo, cubo_l, cubo_d) = efetivas(g, lista);
    let tem_cubo = lista.iter().any(|u| matches!(u, Usinagem::Cubo { .. }));
    if tem_cubo && g.cubo_proprio.is_some() {
        return Err("esta peça já sai com cubo: edite o cubo dela em vez de acrescentar outro".into());
    }
    if lista.iter().filter(|u| matches!(u, Usinagem::Cubo { .. })).count() > 1 {
        return Err("só um cubo por peça".into());
    }
    for u in lista {
        match *u {
            Usinagem::FuroEixo { diametro } => {
                if !(diametro.is_finite() && diametro > 0.) {
                    return Err("o furo do eixo precisa ser positivo".into());
                }
                if diametro >= 2. * g.raio_raiz - 4. {
                    return Err(format!("o furo do eixo precisa ser menor que {:.1} mm (deixa 2 mm de aro sob a raiz)", 2. * g.raio_raiz - 4.));
                }
            }
            Usinagem::RasgoChaveta => {
                let k = chaveta_para_eixo(furo).ok_or_else(|| format!("furo Ø{furo} fora da tabela DIN 6885 (Ø6 a Ø85): sem rasgo de chaveta"))?;
                let parede = if cubo_d > 0. { (cubo_d - furo) / 2. } else { g.raio_raiz - furo / 2. };
                if parede <= k.t2 + 1. {
                    return Err(format!("o rasgo (t2 {:.1} mm) não cabe: sobra só {:.1} mm de parede no cubo", k.t2, parede));
                }
            }
            Usinagem::Cubo { diametro, comprimento } => {
                if !(comprimento > 0. && comprimento.is_finite()) || !(diametro.is_finite()) {
                    return Err("o cubo precisa de diâmetro e comprimento positivos".into());
                }
                if diametro < furo + 2. {
                    return Err(format!("o diâmetro do cubo precisa passar o furo em 2 mm ({:.1} mm no mínimo)", furo + 2.));
                }
                if diametro / 2. > g.raio_raiz {
                    return Err("o cubo passa da raiz dos dentes".into());
                }
            }
            Usinagem::Rebaixo { diametro, profundidade } => {
                if !(diametro > furo && profundidade > 0. && profundidade < g.largura) {
                    return Err(format!("o rebaixo precisa de diâmetro maior que o furo ({furo:.1}) e profundidade menor que a largura ({:.1})", g.largura));
                }
                if diametro / 2. >= g.raio_raiz - 1. {
                    return Err("o rebaixo passa da raiz dos dentes".into());
                }
            }
            Usinagem::FurosAlivio { quantidade, diametro, raio_circulo } => {
                if !(2..=16).contains(&quantidade) || !(diametro > 0.) {
                    return Err("os furos de alívio pedem de 2 a 16 furos de diâmetro positivo".into());
                }
                let rc = raio_circulo;
                let r_cubo = if cubo_d > 0. { cubo_d / 2. } else { furo / 2. };
                if rc - diametro / 2. <= r_cubo + 1. || rc + diametro / 2. >= g.raio_raiz - 1. {
                    return Err(format!("os furos de alívio precisam ficar entre o cubo ({:.1} mm) e a raiz ({:.1} mm), com 1 mm de sobra", r_cubo + 1., g.raio_raiz - 1.));
                }
                // os furos vizinhos não podem se tocar: corda entre centros maior que o diâmetro + 1
                if 2. * rc * (PI / quantidade as f64).sin() <= diametro + 1. {
                    return Err("os furos de alívio ficam encostados: reduza a quantidade ou o diâmetro".into());
                }
            }
            Usinagem::ParafusoFixacao { rosca, rotacao } => {
                let broca = diametro_furo_de_macho(rosca).ok_or_else(|| format!("rosca M{rosca} fora da tabela métrica"))?;
                if !rotacao.is_finite() {
                    return Err("giro do parafuso inválido".into());
                }
                let parede = if cubo_d > 0. { (cubo_d - furo) / 2. } else { g.raio_raiz - furo / 2. };
                if cubo_l <= 0. && g.largura < 2. * broca {
                    return Err("sem cubo, a largura da peça precisa passar de 2 diâmetros do furo do parafuso".into());
                }
                if parede < broca + 1. {
                    return Err(format!("o furo do parafuso (Ø{broca:.1}) não cabe na parede do cubo ({parede:.1} mm)"));
                }
                if cubo_l > 0. && cubo_l < broca + 2. {
                    return Err("o cubo é curto demais para o parafuso de fixação".into());
                }
            }
        }
    }
    Ok(())
}

fn nomear(m: &mut Montador, id: Id, nome: &str) {
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = nome.into();
    }
}

/// Acrescenta as usinagens ao documento da peça padronizada (já com disco e furo do catálogo).
pub(crate) fn aplicar(m: &mut Montador, g: &GeoBase, lista: &[Usinagem]) -> Result<(), String> {
    validar(g, lista)?;
    let (furo, cubo_l, _) = efetivas(g, lista);
    let b = g.largura;
    let topo = b + cubo_l;
    // 1. o cubo acrescenta material antes dos cortes
    for (i, u) in lista.iter().enumerate() {
        if let Usinagem::Cubo { diametro, comprimento } = *u {
            let k = i + 1;
            m.param(&format!("u{k}_dcubo"), &diametro.to_string())?;
            m.param(&format!("u{k}_lcubo"), &comprimento.to_string())?;
            let plano = m.plano(&format!("Plano_usinagem_{k}_cubo"), "b-0.5")?;
            m.prim(&format!("Usinagem_{k}_Cubo"), PrimKind::Cilindro, plano, [&format!("u{k}_dcubo/2"), &format!("u{k}_lcubo+0.5"), "0", "0"], BoolOp::Add)?;
        }
    }
    // 2. furo do eixo (atravessa o cubo também)
    for (i, u) in lista.iter().enumerate() {
        if let Usinagem::FuroEixo { diametro } = *u {
            let k = i + 1;
            m.param(&format!("u{k}_furo"), &diametro.to_string())?;
            let plano = m.plano(&format!("Plano_usinagem_{k}_furo"), "-1")?;
            m.prim(&format!("Usinagem_{k}_Furo_eixo"), PrimKind::Cilindro, plano, [&format!("u{k}_furo/2"), &(topo + 2.).to_string(), "0", "0"], BoolOp::Cut)?;
        } else if matches!(u, Usinagem::Cubo { .. }) && g.furo > 0. {
            // o cubo novo fechou o furo do catálogo: reabre pelo furo do catálogo
            let k = i + 1;
            let plano = m.plano(&format!("Plano_usinagem_{k}_furo_cubo"), "-1")?;
            m.prim(&format!("Usinagem_{k}_Furo_no_cubo"), PrimKind::Cilindro, plano, ["furo/2", &(topo + 2.).to_string(), "0", "0"], BoolOp::Cut)?;
        }
    }
    // 3. rasgo de chaveta, do furo à parede, em todo o comprimento
    for (i, u) in lista.iter().enumerate() {
        match *u {
            Usinagem::RasgoChaveta => {
                let k = i + 1;
                let c = chaveta_para_eixo(furo).ok_or("furo fora da tabela DIN 6885")?;
                m.param(&format!("u{k}_kb"), &c.b.to_string())?;
                m.param(&format!("u{k}_kt2"), &c.t2.to_string())?;
                let plano = plano_yz(m, &format!("Plano_usinagem_{k}_rasgo"), &format!("{}-1", furo / 2.))?;
                let mut data = SketchData::default();
                let mut al = || m.doc.alloc();
                data.add_rect(&mut al, [-c.b / 2., -0.5], [c.b / 2., topo + 0.5], false);
                let sk = m.sketch(&format!("Usinagem_{k}_Rasgo"), plano, data);
                extrusao_cega(m, &format!("Usinagem_{k}_Rasgo_chaveta"), sk, &format!("u{k}_kt2+1"), BoolOp::Cut, true);
            }
            Usinagem::Rebaixo { diametro, profundidade } => {
                let k = i + 1;
                m.param(&format!("u{k}_dreb"), &diametro.to_string())?;
                m.param(&format!("u{k}_preb"), &profundidade.to_string())?;
                let plano = m.plano(&format!("Plano_usinagem_{k}_rebaixo"), "-1")?;
                m.prim(&format!("Usinagem_{k}_Rebaixo"), PrimKind::Cilindro, plano, [&format!("u{k}_dreb/2"), &format!("u{k}_preb+1"), "0", "0"], BoolOp::Cut)?;
            }
            Usinagem::FurosAlivio { quantidade, diametro, raio_circulo } => {
                let k = i + 1;
                m.param(&format!("u{k}_dali"), &diametro.to_string())?;
                let plano = PlaneRef::Origin(OriginPlane::XY);
                let mut data = SketchData::default();
                {
                    let mut al = || m.doc.alloc();
                    for j in 0..quantidade {
                        let a = 2. * PI * (j as f64 + 0.5) / quantidade as f64;
                        let c = data.add_point(&mut al, raio_circulo * a.cos(), raio_circulo * a.sin());
                        data.add_curve(&mut al, Curve::Circle { c, r: diametro / 2. }, false);
                    }
                }
                let sk = m.sketch(&format!("Usinagem_{k}_Alivio"), plano, data);
                let id = m.doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::TwoSides((topo + 2.).to_string(), (topo + 2.).to_string()), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
                nomear(m, id, &format!("Usinagem_{k}_Furos_alivio"));
            }
            Usinagem::ParafusoFixacao { rosca, rotacao } => {
                let k = i + 1;
                let broca = diametro_furo_de_macho(rosca).ok_or("rosca fora da tabela")?;
                m.param(&format!("u{k}_dmacho"), &broca.to_string())?;
                let base = crate::eixos_recursos::plano_girado(m, &format!("Plano_usinagem_{k}_giro"), rotacao);
                let mut data = SketchData::default();
                let z_meio = if cubo_l > 0. { b + cubo_l / 2. } else { b / 2. };
                {
                    let mut al = || m.doc.alloc();
                    let c = data.add_point(&mut al, 0., z_meio);
                    data.add_curve(&mut al, Curve::Circle { c, r: broca / 2. }, false);
                }
                let sk = m.sketch(&format!("Usinagem_{k}_Parafuso"), base, data);
                // do centro para fora (um lado só): o outro lado do cubo fica inteiro
                let meio = (g.raio_raiz * 2. + 4.).to_string();
                let id = m.doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::Blind(meio), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
                nomear(m, id, &format!("Usinagem_{k}_Furo_parafuso_fixacao"));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Grava o vínculo com o catálogo na peça: a chave padronizada e a lista que reproduz as usinagens.
pub fn registrar_vinculo(doc: &mut Document, chave_catalogo: &str, lista: &[Usinagem]) {
    doc.props.custom.retain(|(k, _)| k != "catalogo" && k != "usinagens");
    doc.props.custom.push(("catalogo".into(), chave_catalogo.into()));
    doc.props.custom.push(("usinagens".into(), serde_json::to_string(lista).unwrap_or_else(|_| "[]".into())));
}

/// Lê o vínculo: (chave do catálogo, usinagens). `None` se a peça não veio do catálogo de transmissão.
pub fn vinculo_catalogo(doc: &Document) -> Option<(String, Vec<Usinagem>)> {
    let chave = doc.props.custom.iter().find(|(k, _)| k == "catalogo")?.1.clone();
    let lista = doc.props.custom.iter().find(|(k, _)| k == "usinagens").and_then(|(_, v)| serde_json::from_str(v).ok()).unwrap_or_default();
    Some((chave, lista))
}

/// Sufixo de chave que distingue peças com usinagens diferentes (vazio sem usinagem).
pub fn sufixo_de_chave(lista: &[Usinagem]) -> String {
    if lista.is_empty() {
        return String::new();
    }
    let h = blake3::hash(serde_json::to_string(lista).unwrap_or_default().as_bytes());
    format!("-u{}", &h.to_hex()[..8])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engrenagens::{engrenagem_documento, ParamEngrenagem};
    use crate::{recompute, Status};

    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let r = recompute(doc);
        assert!(r.param_error.is_none(), "{:?}", r.param_error);
        for f in &doc.features {
            assert_eq!(r.status[&f.id], Status::Ok, "{}", f.name);
        }
        assert_eq!(r.bodies.len(), 1);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    fn base() -> ParamEngrenagem {
        ParamEngrenagem { modulo: 2., dentes: 40, largura: 12., furo: 10., ..Default::default() }
    }

    #[test]
    fn validacao_diz_o_que_corrigir() {
        let g = GeoBase { largura: 12., raio_raiz: 37.5, furo: 10., cubo_proprio: None };
        assert!(validar(&g, &[]).is_ok());
        assert!(validar(&g, &[Usinagem::Cubo { diametro: 11., comprimento: 10. }]).unwrap_err().contains("2 mm"));
        assert!(validar(&g, &[Usinagem::Cubo { diametro: 30., comprimento: 10. }, Usinagem::Cubo { diametro: 30., comprimento: 5. }]).is_err());
        assert!(validar(&g, &[Usinagem::Rebaixo { diametro: 20., profundidade: 12. }]).is_err());
        assert!(validar(&g, &[Usinagem::Rebaixo { diametro: 20., profundidade: 5. }]).is_ok());
        // alívio: fora do espaço entre cubo e raiz, ou encostado
        assert!(validar(&g, &[Usinagem::FurosAlivio { quantidade: 4, diametro: 10., raio_circulo: 35. }]).is_err());
        assert!(validar(&g, &[Usinagem::FurosAlivio { quantidade: 16, diametro: 10., raio_circulo: 25. }]).is_err());
        assert!(validar(&g, &[Usinagem::FurosAlivio { quantidade: 4, diametro: 10., raio_circulo: 25. }]).is_ok());
        // parafuso sem cubo e furo grande demais para a parede
        assert!(validar(&g, &[Usinagem::ParafusoFixacao { rosca: 6., rotacao: 90. }]).is_ok());
        assert!(validar(&g, &[Usinagem::ParafusoFixacao { rosca: 7.3, rotacao: 90. }]).unwrap_err().contains("tabela"));
        // rasgo com o furo fora da DIN 6885 (Ø5)
        let pequeno = GeoBase { furo: 5., ..g };
        assert!(validar(&pequeno, &[Usinagem::RasgoChaveta]).is_err());
        // peça com cubo próprio recusa um segundo
        let roda = GeoBase { cubo_proprio: Some((8., 22.)), ..g };
        assert!(validar(&roda, &[Usinagem::Cubo { diametro: 30., comprimento: 10. }]).unwrap_err().contains("cubo"));
        assert_eq!(diametro_furo_de_macho(6.), Some(5.));
    }

    #[test]
    fn cada_usinagem_tira_ou_poe_o_volume_esperado_e_mantem_o_vinculo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = base();
        let (v0, lo0, hi0) = medir(&engrenagem_documento(&p).unwrap());
        let com = |u: Vec<Usinagem>| engrenagem_documento(&ParamEngrenagem { usinagens: u, ..base() }).unwrap();
        // cubo Ø20 × 10 acima do disco, com o furo Ø10 atravessando: acrescenta π (10² − 5²) · 10
        let (vc, _, hic) = medir(&com(vec![Usinagem::Cubo { diametro: 20., comprimento: 10. }]));
        let esperado = PI * (100. - 25.) * 10.;
        assert!(((vc - v0) - esperado).abs() / esperado < 0.01, "cubo {} contra {esperado}", vc - v0);
        assert!((hic[2] - 22.).abs() < 1e-3 && (hi0[2] - 12.).abs() < 1e-3 && lo0[2].abs() < 1e-3);
        // rebaixo Ø20 × 4: tira π (10² − 5²) · 4
        let (vr, ..) = medir(&com(vec![Usinagem::Rebaixo { diametro: 20., profundidade: 4. }]));
        let esperado = PI * (100. - 25.) * 4.;
        assert!(((v0 - vr) - esperado).abs() / esperado < 0.01, "rebaixo {}", v0 - vr);
        // 4 furos de alívio Ø8 pelo disco de 12: tira 4 π 4² · 12
        let (va, ..) = medir(&com(vec![Usinagem::FurosAlivio { quantidade: 4, diametro: 8., raio_circulo: 25. }]));
        let esperado = 4. * PI * 16. * 12.;
        assert!(((v0 - va) - esperado).abs() / esperado < 0.01, "alívio {}", v0 - va);
        // aumentar o furo do eixo para Ø14 tira a coroa entre Ø10 e Ø14
        let (vf, ..) = medir(&com(vec![Usinagem::FuroEixo { diametro: 14. }]));
        let esperado = PI * (49. - 25.) * 12.;
        assert!(((v0 - vf) - esperado).abs() / esperado < 0.01, "furo {}", v0 - vf);
        // rasgo de chaveta (Ø10: b3 × t2 1,4... pela tabela) tira volume sem mexer na caixa
        let (vk, lok, hik) = medir(&com(vec![Usinagem::RasgoChaveta]));
        assert!(vk < v0 && (v0 - vk) > 5. && lok[2].abs() < 1e-3 && (hik[2] - 12.).abs() < 1e-3);
        // parafuso de fixação M4 sobre cubo: tira o cilindro de Ø3,3 de um lado da parede
        let so_cubo = vec![Usinagem::Cubo { diametro: 22., comprimento: 12. }];
        let (vcubo, ..) = medir(&com(so_cubo.clone()));
        let mut com_parafuso = so_cubo;
        com_parafuso.push(Usinagem::ParafusoFixacao { rosca: 4., rotacao: 90. });
        let (vp, ..) = medir(&com(com_parafuso));
        let broca = diametro_furo_de_macho(4.).unwrap();
        let parede = (22. - 10.) / 2.;
        let aprox = PI * (broca / 2.).powi(2) * parede;
        assert!(vp < vcubo && ((vcubo - vp) - aprox).abs() / aprox < 0.15, "parafuso {} contra ~{aprox}", vcubo - vp);

        // vínculo com o catálogo: a chave padronizada não muda com as usinagens
        let doc = com(vec![Usinagem::Cubo { diametro: 20., comprimento: 10. }]);
        let (chave, lista) = vinculo_catalogo(&doc).expect("vínculo");
        assert_eq!(chave, base().chave());
        assert_eq!(lista, vec![Usinagem::Cubo { diametro: 20., comprimento: 10. }]);
        assert!(vinculo_catalogo(&engrenagem_documento(&base()).unwrap()).is_some_and(|(_, l)| l.is_empty()));
        // a peça com usinagem tem arquivo próprio
        let comu = ParamEngrenagem { usinagens: vec![Usinagem::RasgoChaveta], ..base() };
        assert_ne!(comu.chave_arquivo(), base().chave_arquivo());
        assert_eq!(comu.chave(), base().chave());
    }

    #[test]
    fn usinagem_invalida_recusa_o_documento() {
        let ruim = ParamEngrenagem { usinagens: vec![Usinagem::Rebaixo { diametro: 100., profundidade: 4. }], ..base() };
        assert!(engrenagem_documento(&ruim).is_err());
        assert!(crate::engrenagens::calculo(&ruim).is_err(), "o cálculo já avisa");
    }
}
