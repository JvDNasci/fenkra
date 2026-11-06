// SPDX-License-Identifier: LGPL-2.1-or-later
//! Recursos de seção do eixo gráfico (B281): rasgos de chaveta com rotação (mais de um por trecho),
//! ranhura de anel elástico, ranhura simples, rosca e furo transversal.
//!
//! Cada recurso mora num trecho e mede sua posição a partir da **borda esquerda** (z menor) ou da
//! **direita** (z maior) do trecho até a face do recurso mais próxima dessa borda. Os recursos são
//! cortes acrescentados depois do corpo do eixo ([`eixo_documento`]); o corpo e os rasgos antigos do
//! trecho (`TrechoEixo::rasgo`) ficam como estão.
//!
//! - **Rasgo**: forma A da tabela da chaveta pelo Ø do trecho; plano girado de `rotacao` graus em
//!   torno do eixo (0° = +X, como o rasgo do trecho).
//! - **Ranhura** e **anel elástico**: anel de largura `largura` e profundidade `profundidade` cortado
//!   por revolução. O anel não embute a tabela do fabricante: largura e profundidade são dadas.
//! - **Rosca**: simplificação de volume equivalente, reduz o trecho ao diâmetro primitivo
//!   `d − 0,6495·P` (perfil básico métrico) ao longo do comprimento; sem filete.
//! - **Furo transversal**: cilindro que atravessa o eixo, perpendicular a ele, na direção `rotacao`.

use crate::biblioteca::chaveta_para_eixo;
use crate::eixos::{extrusao_cega, plano_yz, ParamEixo};
use crate::fixadores::Montador;
use fenkra_core::*;

/// Fator do diâmetro primitivo da rosca métrica: d2 = d − 0,6495·P (perfil básico ISO).
const FATOR_D2: f64 = 0.649_519;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Borda {
    Esquerda,
    Direita,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TipoRecurso {
    Rasgo { comprimento: f64, rotacao: f64 },
    Anel { largura: f64, profundidade: f64 },
    Ranhura { largura: f64, profundidade: f64 },
    Rosca { comprimento: f64, passo: f64 },
    FuroTransversal { diametro: f64, rotacao: f64 },
}

impl TipoRecurso {
    pub const NOMES: [&'static str; 5] = ["Rasgo", "Anel elástico", "Ranhura", "Rosca", "Furo transversal"];

    pub fn nome(&self) -> &'static str {
        match self {
            TipoRecurso::Rasgo { .. } => Self::NOMES[0],
            TipoRecurso::Anel { .. } => Self::NOMES[1],
            TipoRecurso::Ranhura { .. } => Self::NOMES[2],
            TipoRecurso::Rosca { .. } => Self::NOMES[3],
            TipoRecurso::FuroTransversal { .. } => Self::NOMES[4],
        }
    }

    /// Dimensão do recurso ao longo do eixo (mm).
    pub fn extensao(&self) -> f64 {
        match *self {
            TipoRecurso::Rasgo { comprimento, .. } | TipoRecurso::Rosca { comprimento, .. } => comprimento,
            TipoRecurso::Anel { largura, .. } | TipoRecurso::Ranhura { largura, .. } => largura,
            TipoRecurso::FuroTransversal { diametro, .. } => diametro,
        }
    }

    /// Recurso novo com valores razoáveis para o diâmetro `d` do trecho.
    pub fn padrao(nome: &str, d: f64) -> TipoRecurso {
        match nome {
            "Anel elástico" => TipoRecurso::Anel { largura: 1.1, profundidade: 0.6 },
            "Ranhura" => TipoRecurso::Ranhura { largura: 2., profundidade: 1. },
            "Rosca" => TipoRecurso::Rosca { comprimento: (d * 1.5).max(6.), passo: passo_grosso(d).unwrap_or(1.) },
            "Furo transversal" => TipoRecurso::FuroTransversal { diametro: (d / 4.).max(2.), rotacao: 0. },
            _ => TipoRecurso::Rasgo { comprimento: (d * 1.5).max(10.), rotacao: 0. },
        }
    }
}

/// Um recurso de seção: onde está (trecho, borda de referência, distância) e o que é.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecursoEixo {
    /// índice do trecho (0 = o primeiro, em z = 0)
    pub trecho: usize,
    pub borda: Borda,
    /// distância da borda escolhida do trecho até a face mais próxima do recurso
    pub posicao: f64,
    pub tipo: TipoRecurso,
}

/// Passo grosso da rosca métrica para o diâmetro nominal `d` (da tabela de fixadores).
pub fn passo_grosso(d: f64) -> Option<f64> {
    crate::hardware::metric(&format!("M{}", d.round())).map(|m| m.pitch)
}

/// Posição medida da outra borda para o mesmo lugar: serve para trocar de referência sem mexer no
/// recurso. `comprimento_trecho` é o L do trecho e `extensao` a dimensão do recurso ao longo do eixo.
pub fn posicao_na_outra_borda(comprimento_trecho: f64, extensao: f64, posicao: f64) -> f64 {
    comprimento_trecho - extensao - posicao
}

/// Intervalo `[z0, z1]` do recurso no eixo, ou `None` com o trecho inexistente.
pub fn intervalo_z(p: &ParamEixo, r: &RecursoEixo) -> Option<(f64, f64)> {
    let t = p.trechos.get(r.trecho)?;
    let inicio: f64 = p.trechos[..r.trecho].iter().map(|t| t.comprimento).sum();
    let e = r.tipo.extensao();
    let z0 = match r.borda {
        Borda::Esquerda => inicio + r.posicao,
        Borda::Direita => inicio + t.comprimento - e - r.posicao,
    };
    Some((z0, z0 + e))
}

/// Confere cada recurso contra o eixo.
pub fn validar_recursos(p: &ParamEixo, recursos: &[RecursoEixo]) -> Result<(), String> {
    for (i, r) in recursos.iter().enumerate() {
        let rot = format!("recurso {} ({})", i + 1, r.tipo.nome());
        let t = p.trechos.get(r.trecho).ok_or_else(|| format!("{rot}: o trecho {} não existe", r.trecho + 1))?;
        let valores = [r.posicao, r.tipo.extensao()];
        if valores.iter().any(|v| !v.is_finite()) {
            return Err(format!("{rot}: valor inválido"));
        }
        let e = r.tipo.extensao();
        if e <= 0. {
            return Err(format!("{rot}: a dimensão ao longo do eixo precisa ser positiva"));
        }
        if r.posicao < 0. || r.posicao + e > t.comprimento + 1e-9 {
            return Err(format!("{rot}: não cabe no trecho {} de {} mm (posição {} + {})", r.trecho + 1, t.comprimento, r.posicao, e));
        }
        let raio = t.diametro / 2.;
        match r.tipo {
            TipoRecurso::Rasgo { rotacao, .. } => {
                let c = chaveta_para_eixo(t.diametro).ok_or_else(|| format!("{rot}: Ø{} fora da tabela DIN 6885 (Ø6 a Ø85)", t.diametro))?;
                if e <= c.b {
                    return Err(format!("{rot}: o rasgo precisa ser mais longo que a largura da chaveta ({} mm)", c.b));
                }
                if !rotacao.is_finite() {
                    return Err(format!("{rot}: rotação inválida"));
                }
            }
            TipoRecurso::Anel { profundidade, .. } | TipoRecurso::Ranhura { profundidade, .. } => {
                if !(profundidade > 0. && profundidade < raio * 0.5) {
                    return Err(format!("{rot}: a profundidade precisa estar entre 0 e um quarto do diâmetro ({} mm)", t.diametro / 4.));
                }
            }
            TipoRecurso::Rosca { passo, .. } => {
                if !(passo > 0. && passo * FATOR_D2 < raio) {
                    return Err(format!("{rot}: passo {passo} inválido para o Ø{}", t.diametro));
                }
            }
            TipoRecurso::FuroTransversal { diametro, rotacao } => {
                if !rotacao.is_finite() || diametro >= t.diametro * 0.8 {
                    return Err(format!("{rot}: o furo precisa ser menor que 80% do Ø{}", t.diametro));
                }
            }
        }
    }
    Ok(())
}

/// Plano que passa pelo eixo Z, girado de `graus` a partir do YZ (a normal gira de +X). Sem rotação
/// devolve o próprio plano YZ da Origem.
fn plano_girado(m: &mut Montador, nome: &str, graus: f64) -> PlaneRef {
    if graus.abs() < 1e-9 {
        return PlaneRef::Origin(OriginPlane::YZ);
    }
    let id = m.doc.add_geometry(RefKind::AnglePlane { base: PlaneRef::Origin(OriginPlane::YZ), axis: AxisRef::Origin(OriginAxis::Z), angle: graus.to_string() });
    if let Some(r) = m.doc.references.iter_mut().find(|r| r.id == id) {
        r.name = nome.into();
        r.visible = false;
    }
    PlaneRef::Datum(id)
}

fn plano_deslocado(m: &mut Montador, nome: &str, base: PlaneRef, desvio: &str) -> Result<PlaneRef, String> {
    if base == PlaneRef::Origin(OriginPlane::YZ) {
        return plano_yz(m, nome, desvio);
    }
    let id = m.doc.add_offset_plane(base, desvio);
    let r = m.doc.references.iter_mut().find(|r| r.id == id).ok_or("plano não criado")?;
    r.name = nome.into();
    r.visible = false;
    Ok(PlaneRef::Datum(id))
}

fn nomear(m: &mut Montador, id: Id, nome: &str) {
    if let Some(f) = m.doc.features.iter_mut().find(|f| f.id == id) {
        f.name = nome.into();
    }
}

/// Corte por revolução de um anel entre os raios `r_in` e `r_out`, de `z0` a `z1`.
fn corte_anel(m: &mut Montador, nome: &str, r_in: f64, r_out: f64, z0: f64, z1: f64) {
    let mut data = SketchData::default();
    let mut al = || m.doc.alloc();
    data.add_rect(&mut al, [r_in, z0], [r_out, z1], false);
    let sk = m.sketch(&format!("Perfil_{nome}"), PlaneRef::Origin(OriginPlane::XZ), data);
    let id = m.doc.add_feature(FeatureKind::Revolve(Revolve { sketch: sk, axis: AxisRef::Origin(OriginAxis::Z), angle: "360".into(), symmetric: false, op: BoolOp::Cut, thin: None, thin_side: 0, angle2: None }));
    nomear(m, id, nome);
}

/// Documento do eixo com os recursos de seção.
pub fn eixo_documento_com_recursos(p: &ParamEixo, recursos: &[RecursoEixo]) -> Result<Document, String> {
    validar_recursos(p, recursos)?;
    let doc = crate::eixos::eixo_documento(p)?;
    let mut m = Montador { doc };
    for (i, rec) in recursos.iter().enumerate() {
        let k = i + 1;
        let t = p.trechos[rec.trecho];
        let (z0, z1) = intervalo_z(p, rec).ok_or("trecho inexistente")?;
        let raio = t.diametro / 2.;
        match rec.tipo {
            TipoRecurso::Rasgo { rotacao, .. } => {
                let c = chaveta_para_eixo(t.diametro).ok_or("Ø fora da tabela")?;
                m.param(&format!("rkb{k}"), &c.b.to_string())?;
                m.param(&format!("rkt{k}"), &c.t1.to_string())?;
                let base = plano_girado(&mut m, &format!("Plano_giro_rasgo_r{k}"), rotacao);
                let plano = plano_deslocado(&mut m, &format!("Plano_rasgo_r{k}"), base, &format!("{}-rkt{k}", raio))?;
                let sk = crate::eixos::estadio(&mut m, &format!("Rasgo_r{k}"), plano, [0., z0 + c.b / 2.], [0., z1 - c.b / 2.], c.b / 2.);
                extrusao_cega(&mut m, &format!("Rasgo_chaveta_r{k}"), sk, &format!("rkt{k}+1"), BoolOp::Cut, true);
            }
            TipoRecurso::Anel { profundidade, .. } | TipoRecurso::Ranhura { profundidade, .. } => {
                let nome = if matches!(rec.tipo, TipoRecurso::Anel { .. }) { "Anel_elastico" } else { "Ranhura" };
                corte_anel(&mut m, &format!("{nome}_{k}"), raio - profundidade, raio + 1., z0, z1);
            }
            TipoRecurso::Rosca { passo, .. } => {
                corte_anel(&mut m, &format!("Rosca_{k}"), raio - FATOR_D2 * passo / 2., raio + 1., z0, z1);
            }
            TipoRecurso::FuroTransversal { diametro, rotacao } => {
                let base = plano_girado(&mut m, &format!("Plano_giro_furo_{k}"), rotacao);
                let mut data = SketchData::default();
                let mut al = || m.doc.alloc();
                let c = data.add_point(&mut al, 0., (z0 + z1) / 2.);
                data.add_curve(&mut al, fenkra_core::sketch::Curve::Circle { c, r: diametro / 2. }, false);
                let sk = m.sketch(&format!("Furo_transversal_{k}"), base, data);
                let meio = (t.diametro + 2.).to_string();
                let id = m.doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::TwoSides(meio.clone(), meio), reverse: false, taper: "0".into(), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
                nomear(&mut m, id, &format!("Furo_transversal_{k}"));
            }
        }
    }
    Ok(m.doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eixos::{RasgoChaveta, TrechoEixo};
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    fn base() -> ParamEixo {
        ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None },
                TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None },
            ],
            furo_centro: false,
        }
    }

    /// (volume, centro de massa) do eixo com os recursos.
    fn medir(p: &ParamEixo, r: &[RecursoEixo]) -> (f64, [f64; 3]) {
        let doc = eixo_documento_com_recursos(p, r).unwrap();
        let res = recompute(&doc);
        assert!(res.param_error.is_none(), "{:?}", res.param_error);
        for f in &doc.features {
            assert!(matches!(res.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, res.status[&f.id]);
        }
        assert_eq!(res.bodies.len(), 1);
        (res.bodies[0].mass.volume, res.bodies[0].mass.center)
    }

    fn rec(trecho: usize, borda: Borda, posicao: f64, tipo: TipoRecurso) -> RecursoEixo {
        RecursoEixo { trecho, borda, posicao, tipo }
    }

    fn perto(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1.)
    }

    #[test]
    fn ranhura_e_anel_tiram_o_volume_do_anel_e_a_borda_direita_vale_a_mesma_coisa() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = base();
        let (v0, c0) = medir(&p, &[]);
        let esperado = PI * (12.5_f64.powi(2) - 11.5_f64.powi(2)) * 2.;
        let esq = rec(1, Borda::Esquerda, 10., TipoRecurso::Ranhura { largura: 2., profundidade: 1. });
        let (v, c) = medir(&p, &[esq]);
        assert!(perto(v0 - v, esperado, 0.01), "ranhura {} contra {esperado}", v0 - v);
        // o mesmo lugar medido pela borda direita: 70 - 2 - 10 = 58
        let dir = rec(1, Borda::Direita, posicao_na_outra_borda(70., 2., 10.), TipoRecurso::Ranhura { largura: 2., profundidade: 1. });
        let (vd, cd) = medir(&p, &[dir]);
        assert!(perto(vd, v, 1e-6) && perto(cd[2], c[2], 1e-6), "{vd} {v} / {} {}", cd[2], c[2]);
        // a ranhura cortou de verdade: o centro de massa se mexe
        assert!((c[2] - c0[2]).abs() > 1e-4);
        assert_eq!(intervalo_z(&p, &esq), Some((60., 62.)));
        assert_eq!(intervalo_z(&p, &dir), Some((60., 62.)));
        // anel elástico: mesma geometria, outro nome
        let anel = rec(1, Borda::Esquerda, 10., TipoRecurso::Anel { largura: 2., profundidade: 1. });
        assert!(perto(medir(&p, &[anel]).0, v, 1e-9));
    }

    #[test]
    fn rosca_reduz_ao_diametro_primitivo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = base();
        let (v0, _) = medir(&p, &[]);
        // M20 passo 2,5 em 15 mm: d2 = 20 − 0,6495·2,5
        let r = rec(0, Borda::Esquerda, 0., TipoRecurso::Rosca { comprimento: 15., passo: 2.5 });
        let r_in = 10. - FATOR_D2 * 2.5 / 2.;
        let esperado = PI * (100. - r_in * r_in) * 15.;
        let (v, _) = medir(&p, &[r]);
        assert!(perto(v0 - v, esperado, 0.01), "{} contra {esperado}", v0 - v);
        assert_eq!(passo_grosso(20.), Some(2.5));
    }

    #[test]
    fn furo_transversal_tira_o_cilindro_menos_a_curvatura_do_eixo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = base();
        let (v0, c0) = medir(&p, &[]);
        // Ø6 no meio do trecho de Ø25: integra a corda do eixo ao longo do furo sobre o disco do furo
        let (rh, rr) = (3., 12.5_f64);
        let n = 600;
        let mut esperado = 0.;
        for i in 0..n {
            for j in 0..n {
                let (y, z) = (-rh + (i as f64 + 0.5) * 2. * rh / n as f64, -rh + (j as f64 + 0.5) * 2. * rh / n as f64);
                if y * y + z * z <= rh * rh {
                    esperado += 2. * (rr * rr - y * y).sqrt() * (2. * rh / n as f64).powi(2);
                }
            }
        }
        let furo = |rot: f64| rec(1, Borda::Esquerda, 32., TipoRecurso::FuroTransversal { diametro: 6., rotacao: rot });
        for rot in [0., 90., 45.] {
            let (v, c) = medir(&p, &[furo(rot)]);
            assert!(perto(v0 - v, esperado, 0.01), "rot {rot}: {} contra {esperado}", v0 - v);
            // o furo está em z = 85: o centro de massa sai de onde estava só pelo volume que perdeu
            let c_esperado = (v0 * c0[2] - (v0 - v) * 85.) / v;
            assert!((c[2] - c_esperado).abs() < 0.01, "rot {rot}: {} contra {c_esperado}", c[2]);
        }
    }

    #[test]
    fn rasgos_com_rotacao_cortam_o_mesmo_volume_em_lados_diferentes() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let p = base();
        let (v0, _) = medir(&p, &[]);
        let r0 = rec(1, Borda::Esquerda, 10., TipoRecurso::Rasgo { comprimento: 40., rotacao: 0. });
        let (v_0, c_0) = medir(&p, &[r0]);
        let dv = v0 - v_0;
        assert!(dv > 100., "o rasgo corta algo: {dv}");
        // igual ao rasgo antigo do trecho
        let mut antigo = base();
        antigo.trechos[1].rasgo = Some(RasgoChaveta { comprimento: 40., desloc_inicio: 10. });
        assert!(perto(medir(&antigo, &[]).0, v_0, 1e-6));
        // rotações: mesmo volume, lado diferente (o centro de massa foge do lado cortado)
        let girado = |g: f64| medir(&p, &[RecursoEixo { tipo: TipoRecurso::Rasgo { comprimento: 40., rotacao: g }, ..r0 }]);
        let (v90, c90) = girado(90.);
        let (v180, c180) = girado(180.);
        assert!(perto(v90, v_0, 0.002) && perto(v180, v_0, 0.002), "{v_0} {v90} {v180}");
        assert!(c_0[0] < -1e-4 && c180[0] > 1e-4, "0° e 180° em lados opostos: {} {}", c_0[0], c180[0]);
        assert!(c90[0].abs() < 1e-3 && c90[1].abs() > 1e-4, "90° corta no eixo Y: {c90:?}");
        // mais de um rasgo no mesmo trecho: dois (0° e 180°) tiram o dobro
        let r180 = RecursoEixo { tipo: TipoRecurso::Rasgo { comprimento: 40., rotacao: 180. }, ..r0 };
        let (v2, c2) = medir(&p, &[r0, r180]);
        assert!(perto(v0 - v2, 2. * dv, 0.005), "{} contra {}", v0 - v2, 2. * dv);
        assert!(c2[0].abs() < 1e-3, "simétricos: {c2:?}");
    }

    #[test]
    fn recursos_invalidos_sao_recusados_com_motivo() {
        let p = base();
        let ok = TipoRecurso::Ranhura { largura: 2., profundidade: 1. };
        assert!(validar_recursos(&p, &[rec(0, Borda::Esquerda, 5., ok)]).is_ok());
        let msg = |r: RecursoEixo| validar_recursos(&p, &[r]).unwrap_err();
        assert!(msg(rec(5, Borda::Esquerda, 5., ok)).contains("não existe"));
        assert!(msg(rec(0, Borda::Esquerda, 49., ok)).contains("não cabe"));
        assert!(msg(rec(0, Borda::Direita, 49., ok)).contains("não cabe"));
        assert!(msg(rec(0, Borda::Esquerda, -1., ok)).contains("não cabe"));
        assert!(msg(rec(0, Borda::Esquerda, 5., TipoRecurso::Ranhura { largura: 2., profundidade: 6. })).contains("profundidade"));
        assert!(msg(rec(0, Borda::Esquerda, 5., TipoRecurso::Rasgo { comprimento: 4., rotacao: 0. })).contains("mais longo"));
        assert!(msg(rec(0, Borda::Esquerda, 5., TipoRecurso::FuroTransversal { diametro: 18., rotacao: 0. })).contains("80%"));
        assert!(msg(rec(0, Borda::Esquerda, 5., TipoRecurso::Rosca { comprimento: 10., passo: 0. })).contains("passo"));
        assert!(eixo_documento_com_recursos(&p, &[rec(5, Borda::Esquerda, 0., ok)]).is_err());
    }
}
