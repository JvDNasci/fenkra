// SPDX-License-Identifier: LGPL-2.1-or-later
//! Rebites sólidos aeronáuticos (B43, B63): cabeça universal (AN470) e escareada de 100° (AN426).
//!
//! **Convenção** (a dos fixadores): eixo em Z, face de apoio em `z = 0`, corpo para `-Z`.
//! - Universal: a cabeça é uma calota esférica de Ø `dk` e altura `k` acima da face; o L conta
//!   só o corpo, de `z = -L` a `0`.
//! - Escareado: a cabeça é um cone de 100° rente à face, de Ø `d` em `z = -k` a Ø `dk` em `z = 0`;
//!   o L conta a cabeça, como no parafuso escareado. `k` não vem da tabela: sai de `dk` e `d`.
//!
//! **Comprimento pela pega**: a pessoa informa a espessura da pilha; o L é o menor da série com
//! `L >= pega + 1,5·d` ([`crate::biblioteca::rebite_pela_pega`]). A sobra de 1,5·d além da pilha é o
//! material que vira a cabeça de fechamento. O documento guarda `pega_max = L - 1,5·d`, a maior pilha
//! que esse L fecha.

use crate::biblioteca::{Familia, Tamanho, SOBRA_REBITE};
use crate::fixadores::Montador;
use fenkra_core::*;

/// Meio ângulo do escareado de 100°, em graus.
pub const MEIO_ANGULO_ESCAREADO: f64 = 50.;

fn medida(fam: &Familia, t: &Tamanho, c: &str) -> Result<f64, String> {
    t.valor(c).ok_or_else(|| format!("{} {}: falta '{c}'", fam.id, t.designacao))
}

/// Altura da cabeça escareada de 100° pelo diâmetro da cabeça e o do corpo.
pub fn altura_escareada(dk: f64, d: f64) -> f64 {
    (dk - d) / 2. / MEIO_ANGULO_ESCAREADO.to_radians().tan()
}

/// Raio da esfera da calota de Ø `dk` e altura `k`.
pub fn raio_da_calota(dk: f64, k: f64) -> f64 {
    ((dk / 2.).powi(2) + k * k) / (2. * k)
}

pub(crate) fn corpo(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let d = t.diametro_nominal().ok_or("rebite sem diâmetro")?;
    let dk = medida(fam, t, "dk")?;
    if dk <= d {
        return Err(format!("{} {}: a cabeça precisa ser maior que o corpo", fam.id, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("dk", &dk.to_string())?;
    m.param("pega_max", &format!("L-{SOBRA_REBITE}*d"))?;
    match fam.gerador.as_str() {
        "rebite_universal" => {
            m.param("k", &medida(fam, t, "k")?.to_string())?;
            m.param("R", "((dk/2)^2+k^2)/(2*k)")?;
            let centro = m.plano("Plano_calota", "k-R")?;
            m.prim("Cabeca", PrimKind::Esfera, centro, ["R", "0", "0", "0"], BoolOp::Add)?;
            // tira a parte da esfera abaixo da face de apoio, antes de pôr o corpo
            let baixo = m.plano("Plano_corte_calota", "-2*R-1")?;
            m.prim("Corte_calota", PrimKind::Caixa, baixo, ["3*R", "3*R", "2*R+1", "0"], BoolOp::Cut)?;
            let p_corpo = m.plano("Plano_corpo", "-L")?;
            m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)
        }
        "rebite_escareado" => {
            if l <= altura_escareada(dk, d) {
                return Err(format!("{} {}: comprimento menor que a cabeça", fam.id, t.designacao));
            }
            m.param("k", &format!("(dk-d)/2/tan({MEIO_ANGULO_ESCAREADO})"))?;
            let p_corpo = m.plano("Plano_corpo", "-L")?;
            m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L-k", "0", "0"], BoolOp::Add)?;
            let p_cabeca = m.plano("Plano_cabeca", "-k")?;
            m.prim("Cabeca", PrimKind::Cone, p_cabeca, ["d/2", "dk/2", "k", "0"], BoolOp::Add)
        }
        g => Err(format!("{}: gerador de rebite '{g}' desconhecido", fam.id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biblioteca::{familia, rebite_pela_pega, tamanho, TipoFixador};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    const POL: f64 = 25.4;

    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let rc = recompute(doc);
        assert!(rc.param_error.is_none(), "{:?}", rc.param_error);
        assert!(doc.features.iter().all(|f| rc.status[&f.id] == Status::Ok), "{}", doc.name);
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

    #[test]
    fn dados_dos_rebites_em_duas_fontes() {
        for id in ["an470", "an426"] {
            let f = familia(id).unwrap();
            assert_eq!((f.tipo, f.nivel.as_str()), (TipoFixador::Rebite, "C"));
            assert!(f.fontes.len() >= 2 && f.tamanhos.iter().all(|t| t.confirmado_por >= 2), "{id}");
            assert_eq!(f.tamanhos.iter().map(|t| t.designacao.as_str()).collect::<Vec<_>>().len(), 5, "{id}");
        }
        // AN470-4 (Ø 1/8"): cabeça de 0,238 a 0,262 e altura de 0,054 a 0,064 pol
        let t = tamanho("an470", "AN470-4").unwrap();
        assert!((t.valor("dk").unwrap() / POL - 0.25).abs() < 1e-3 && (t.valor("k").unwrap() / POL - 0.059).abs() < 1e-3);
        // o cone de 100° reproduz a altura de referência publicada para o MS20426
        for (des, ref_pol) in [("AN426-3", 0.036), ("AN426-4", 0.042), ("AN426-5", 0.055), ("AN426-6", 0.070), ("AN426-8", 0.095)] {
            let t = tamanho("an426", des).unwrap();
            let k = altura_escareada(t.valor("dk").unwrap(), t.diametro_nominal().unwrap()) / POL;
            assert!((k - ref_pol).abs() < 0.0015, "{des}: k {k} contra {ref_pol}");
        }
        // achado pelo diâmetro em polegada também
        assert_eq!(tamanho("an470", "5/32 pol").unwrap().designacao, "AN470-5");
    }

    #[test]
    fn comprimento_pela_pega() {
        let f = familia("an470").unwrap();
        let t = f.tamanho("AN470-4").unwrap();
        // pilha de 0,080": 0,080 + 1,5 × 0,125 = 0,2675" → o traço -5 (5/16")
        let l = rebite_pela_pega(f, t, 0.080 * POL).unwrap();
        assert!((l - 5. / 16. * POL).abs() < 1e-3, "{l}");
        // pilha exata de um traço não pula para o seguinte
        let l = rebite_pela_pega(f, t, 4. / 16. * POL - 1.5 * 0.125 * POL).unwrap();
        assert!((l - 4. / 16. * POL).abs() < 1e-3, "{l}");
        // pilha grossa demais para a série
        assert!(rebite_pela_pega(f, t, 2. * POL).is_none());
    }

    #[test]
    fn universal_tem_calota_acima_da_face_e_corpo_abaixo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("an470").unwrap().tamanhos {
            let (d, dk, k) = (t.diametro_nominal().unwrap(), t.valor("dk").unwrap(), t.valor("k").unwrap());
            let l = 6. / 16. * POL;
            let doc = fixador_documento("an470", &t.designacao, Some(l)).unwrap();
            let (vol, lo, hi) = medir(&doc);
            assert!((lo[2] + l).abs() < 1e-3 && (hi[2] - k).abs() < 1e-3, "{}: z {} a {}", t.designacao, lo[2], hi[2]);
            assert!((hi[0] - lo[0] - dk).abs() < 0.02 * dk, "{}: largura {}", t.designacao, hi[0] - lo[0]);
            let r = raio_da_calota(dk, k);
            let esperado = PI * k * k * (3. * r - k) / 3. + PI * d * d / 4. * l;
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
        }
    }

    #[test]
    fn escareado_fica_rente_com_o_cone_de_100_graus() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        for t in &familia("an426").unwrap().tamanhos {
            let (d, dk) = (t.diametro_nominal().unwrap(), t.valor("dk").unwrap());
            let k = altura_escareada(dk, d);
            let l = 8. / 16. * POL;
            let doc = fixador_documento("an426", &t.designacao, Some(l)).unwrap();
            let (vol, lo, hi) = medir(&doc);
            assert!((lo[2] + l).abs() < 1e-3 && hi[2].abs() < 1e-3, "{}: z {} a {}", t.designacao, lo[2], hi[2]);
            assert!((hi[0] - lo[0] - dk).abs() < 0.02 * dk, "{}", t.designacao);
            let (r1, r2) = (d / 2., dk / 2.);
            let esperado = PI * k / 3. * (r1 * r1 + r1 * r2 + r2 * r2) + PI * r1 * r1 * (l - k);
            assert!((vol - esperado).abs() / esperado < 1e-3, "{}: volume {vol} contra {esperado}", t.designacao);
        }
    }

    #[test]
    fn rebite_publica_eixo_ponta_e_pega() {
        let doc = fixador_documento("an470", "AN470-4", Some(5. / 16. * POL)).unwrap();
        assert!(doc.references.iter().any(|r| r.published && r.name == "Ponta"));
        assert!(doc.params.iter().any(|p| p.name == "pega_max"));
        assert!(doc.props.custom.iter().any(|(k, v)| k == "Norma" && v.starts_with("AN 470")));
    }
}
