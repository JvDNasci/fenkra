// SPDX-License-Identifier: LGPL-2.1-or-later
//! Prendedores aeronáuticos de forma própria (B43): parafuso de 12 pontos flangeado (MS21250) e pino
//! reto de cabeça com furo de cupilha (MS20392).
//!
//! **Convenção** (a dos fixadores): eixo em Z, face de apoio em `z = 0`, corpo para `-Z`.
//! - 12 pontos: o flange de Ø `A` e espessura `Q` fica logo acima da face; sobre ele sobe o elemento
//!   de chave, um prisma reto de 12 lados de Ø `D` (circunscrito), até a altura total `k`. As ondas
//!   do elemento de chave da folha não são modeladas.
//! - Pino: cabeça cilíndrica Ø `dk` × `k`, haste de Ø `A` e comprimento `L` sob a cabeça, e um furo
//!   transversal de Ø `E` para a cupilha com o centro a `G + E/2` da face de apoio, onde
//!   `G = L − OL` é o comprimento efetivo da folha.

use crate::biblioteca::{Familia, Tamanho};
use crate::fixadores::Montador;
use fenkra_core::*;

fn medida(fam: &Familia, t: &Tamanho, c: &str) -> Result<f64, String> {
    t.valor(c).ok_or_else(|| format!("{} {}: falta '{c}'", fam.id, t.designacao))
}

/// Parafuso MS21250: haste de Ø d, flange, elemento de chave de 12 lados.
pub(crate) fn doze_pontos(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let (a, q, dd, k) = (medida(fam, t, "A")?, medida(fam, t, "Q")?, medida(fam, t, "D")?, medida(fam, t, "k")?);
    let d = t.diametro_nominal().ok_or("sem diâmetro")?;
    if !(a > dd && dd > d && k > q) {
        return Err(format!("{} {}: precisa de flange > elemento de chave > haste e cabeça mais alta que o flange", fam.id, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("P", &medida(fam, t, "P")?.to_string())?;
    m.param("s", &medida(fam, t, "s")?.to_string())?;
    m.param("k", &k.to_string())?;
    m.param("A", &a.to_string())?;
    m.param("Q", &q.to_string())?;
    m.param("D", &dd.to_string())?;
    m.param("b", &t.valor("b").unwrap_or(l).to_string())?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Flange", PrimKind::Cilindro, topo.clone(), ["A/2", "Q", "0", "0"], BoolOp::Add)?;
    m.prim("Elemento_12_pontos", PrimKind::Prisma, topo, ["12", "D/2", "k", "0"], BoolOp::Add)?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)
}

/// Pino MS20392: cabeça, haste e furo de cupilha.
pub(crate) fn pino_cabeca(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let (a, dk, k, e, off) = (medida(fam, t, "A")?, medida(fam, t, "dk")?, medida(fam, t, "k")?, medida(fam, t, "E")?, medida(fam, t, "OL")?);
    if !(dk > a && e < a && l > off) {
        return Err(format!("{} {}: cabeça maior que a haste, furo menor que a haste e comprimento maior que L − G", fam.id, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("A", &a.to_string())?;
    m.param("dk", &dk.to_string())?;
    m.param("k", &k.to_string())?;
    m.param("E", &e.to_string())?;
    // G = L − OL; o centro do furo fica a G + E/2 da face de apoio
    m.param("G", &format!("L-{off}"))?;
    m.param("F", "G+E/2")?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Cabeca", PrimKind::Cilindro, topo, ["dk/2", "k", "0", "0"], BoolOp::Add)?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Haste", PrimKind::Cilindro, p_corpo, ["A/2", "L", "0", "0"], BoolOp::Add)?;
    // furo transversal: cilindro com eixo em Y, de Ø E, a F sob a face de apoio
    let id = m.doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XZ), "-(A/2+1)");
    let r = m.doc.references.iter_mut().find(|r| r.id == id).ok_or("plano não criado")?;
    r.name = "Plano_cupilha".into();
    r.visible = false;
    m.prim("Furo_cupilha", PrimKind::Cilindro, PlaneRef::Datum(id), ["E/2", "A+2", "0", "-F"], BoolOp::Cut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biblioteca::{familia, tamanho};
    use crate::fixadores::fixador_documento;
    use crate::{recompute, Status};
    use std::f64::consts::PI;

    const POL: f64 = 25.4;

    /// Volume, mínimo e máximo do corpo único, com todos os comandos em Ok.
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
    fn ms21250_04_tem_flange_12_pontos_e_haste() {
        let t = tamanho("ms21250", "MS21250-04").unwrap();
        let l = t.comprimentos[3];
        let doc = fixador_documento("ms21250", "MS21250-04", Some(l)).unwrap();
        let (v, lo, hi) = medir(&doc);
        let (a, q, dd, k, d) = (0.433 * POL, 0.069 * POL, 0.3045 * POL, 0.300 * POL, 0.250 * POL);
        // caixa: do fim da haste (-L) ao topo da cabeça (k), flange Ø A
        assert!((lo[2] + l).abs() < 1e-2 && (hi[2] - k).abs() < 1e-2, "{lo:?} {hi:?}");
        assert!((hi[0] - lo[0] - a).abs() < 0.05, "largura = Ø do flange");
        // o volume fica entre os sólidos de referência: o cilindro do elemento (12 lados é menor que o círculo)
        let cil = PI * (a / 2.).powi(2) * q + PI * (dd / 2.).powi(2) * (k - q) + PI * (d / 2.).powi(2) * l;
        let dodeca = 3. * (dd / 2.).powi(2) * (k - q) + PI * (a / 2.).powi(2) * q + PI * (d / 2.).powi(2) * l;
        assert!((v - dodeca).abs() < 0.5 && v < cil, "v {v} dodecágono {dodeca} cilindro {cil}");
    }

    #[test]
    fn ms20392_3_c15_tem_furo_de_cupilha_na_haste() {
        let f = familia("ms20392").unwrap();
        let t = tamanho("ms20392", "MS20392-3").unwrap();
        // MS20392-3C15: G = 15/32 pol e L = G + 0,203 pol (folha: 0,672)
        let l = 0.672 * POL;
        assert!(t.comprimentos.iter().any(|c| (c - l).abs() < 2e-2), "L da série");
        let doc = fixador_documento("ms20392", "MS20392-3", Some(t.comprimentos.iter().copied().find(|c| (c - l).abs() < 2e-2).unwrap())).unwrap();
        let (v, lo, hi) = medir(&doc);
        let (a, dk, k, e) = (0.248 * POL, 0.375 * POL, 0.062 * POL, 0.076 * POL);
        let lt = t.comprimentos.iter().copied().find(|c| (c - l).abs() < 2e-2).unwrap();
        assert!((lo[2] + lt).abs() < 1e-2 && (hi[2] - k).abs() < 1e-2, "{lo:?} {hi:?}");
        assert!((hi[0] - lo[0] - dk).abs() < 0.05);
        // sem furo o volume seria cabeça + haste; o furo transversal tira π (E/2)² A
        let cheio = PI * (dk / 2.).powi(2) * k + PI * (a / 2.).powi(2) * lt;
        let furo = PI * (e / 2.).powi(2) * a;
        // a malha subestima os cilindros em cerca de 0,5%; o furo tira entre 60% e 100% de π (E/2)² A (a corda é menor que A fora do centro)
        let tirado = cheio - v;
        assert!(tirado > 0.6 * furo && tirado < furo + 0.012 * cheio, "furo: cheio {cheio} v {v} tirado {tirado} limite {furo}");
        assert_eq!(f.tamanhos.len(), 9);
        // o tamanho 1/4 pol tem duas fontes (folha + páginas do MS20392-3C15)
        assert_eq!(t.confirmado_por, 2);
    }
    /// Cada família confere as cotas com a folha MS e, onde há segunda fonte, com a ficha do distribuidor.
    #[test]
    fn cotas_contra_as_folhas_ms_e_as_fichas() {
        let v = |f: &str, n: &str, c: &str| tamanho(f, n).unwrap().valor(c).unwrap() / POL;
        // MS20004 (folha E: cabeça 0,428 a 0,438, altura 0,250, sextavado 0,188 a 0,190; ficha MS20004-4 igual)
        assert!((0.428..=0.438).contains(&v("ms20004", "MS20004", "dk")) && (v("ms20004", "MS20004", "k") - 0.250).abs() < 1e-3);
        assert!((0.188..=0.190).contains(&v("ms20004", "MS20004", "s")));
        // MS20005 e MS20006 (fichas MS20005-8 e MS20006-8): cabeça 0,521-0,531 / 0,639-0,649, altura 0,312 / 0,375
        assert!((0.521..=0.531).contains(&v("ms20004", "MS20005", "dk")) && (v("ms20004", "MS20005", "k") - 0.312).abs() < 1e-3);
        assert!((0.639..=0.649).contains(&v("ms20004", "MS20006", "dk")) && (v("ms20004", "MS20006", "k") - 0.375).abs() < 1e-3);
        // MS20008 (ficha MS20008-10): cabeça 0,818-0,828, sextavado 0,375-0,378, rosca 0,787 a 0,837
        assert!((0.818..=0.828).contains(&v("ms20004", "MS20008", "dk")) && (0.375..=0.378).contains(&v("ms20004", "MS20008", "s")));
        assert!((0.787..=0.837).contains(&v("ms20004", "MS20008", "b")));
        // MS20004-10 (ficha): L 1,110 a 1,140 pol com pega de 5/8 pol
        let t = tamanho("ms20004", "MS20004").unwrap();
        assert!(t.comprimentos.iter().any(|l| (1.110..=1.140).contains(&(l / POL))));
        assert!(tamanho("ms20004", "MS20004").unwrap().comprimentos.iter().any(|l| (l / POL - 0.75).abs() < 1e-3), "MS20004-4: L 3/4 pol");
        // MS20073-03 e -04 (fichas): entre faces 0,367-0,376 / 0,430-0,439 e altura 0,172-0,203 / 0,203-0,234
        assert!((0.367..=0.376).contains(&v("ms20073", "MS20073-03", "s")) && (0.172..=0.203).contains(&v("ms20073", "MS20073-03", "k")));
        assert!((0.430..=0.439).contains(&v("ms20073", "MS20073-04", "s")) && (0.203..=0.234).contains(&v("ms20073", "MS20073-04", "k")));
        // MS20073-04-10: L 1,031 pol com pega de 0,500 pol
        let t = tamanho("ms20073", "MS20073-04").unwrap();
        assert!(t.comprimentos.iter().any(|l| (l / POL - 1.031).abs() < 2e-3 && (l / POL - t.valor("b").unwrap() / POL - 0.5).abs() < 2e-3));
        // MS20002-4 e -5 (fichas): externo 0,521-0,541 / 0,583-0,603, furo 0,252-0,260 / 0,315-0,324, espessura 0,052-0,072
        for (n, a, b) in [("MS20002-4", (0.521, 0.541), (0.252, 0.260)), ("MS20002-5", (0.583, 0.603), (0.315, 0.324))] {
            assert!((a.0..=a.1).contains(&v("ms20002", n, "d2")) && (b.0..=b.1).contains(&v("ms20002", n, "d1")) && (0.052..=0.072).contains(&v("ms20002", n, "h")), "{n}");
        }
        // MS20392-3 (fichas do 3C15): haste 0,246 a 0,248, cabeça 3/8, furo de cupilha 0,066 a 0,086
        assert!((0.246..=0.248).contains(&v("ms20392", "MS20392-3", "A")) && (v("ms20392", "MS20392-3", "dk") - 0.375).abs() < 1e-3);
        assert!((0.066..=0.086).contains(&v("ms20392", "MS20392-3", "E")));
        // AN9 e AN10: entre faces também na folha MS20073 (0,865-0,876 e 0,928-0,939) e na ficha (0,865-0,877 e 0,928-0,940)
        assert!((0.865..=0.876).contains(&v("an3", "AN9", "s")) && (0.928..=0.939).contains(&v("an3", "AN10", "s")));
        assert!((0.296..=0.328).contains(&v("an3", "AN9", "k")) && (0.328..=0.360).contains(&v("an3", "AN10", "k")));
        // AN9-24A (L 2 21/32, pega 1 3/4) e AN10-40A (L 4,141, pega 3,188); o 40 fica fora da série curta, o 24 do AN9 não
        assert!(tamanho("an3", "AN9").unwrap().comprimentos.iter().any(|l| (l / POL - 2.656).abs() < 2e-3));
        assert!((v("an3", "AN10", "b") - 0.953).abs() < 1e-3 && (v("an3", "AN9", "b") - 0.906).abs() < 1e-3);
        // as famílias de uma fonte só ficam marcadas PENDENTE
        for id in ["ms20004", "ms20073", "ms20002", "ms20392", "ms21250"] {
            let f = familia(id).unwrap();
            for t in &f.tamanhos {
                assert!(t.confirmado_por >= 2 || t.nota.as_deref().is_some_and(|n| n.contains("PENDENTE")), "{id} {}", t.designacao);
            }
        }
        assert!(familia("ms21250").unwrap().nivel == "B" && familia("ms21250").unwrap().tamanhos.iter().all(|t| t.confirmado_por == 1));
        assert!(tamanho("an3", "AN9").unwrap().confirmado_por == 1 && tamanho("an3", "AN10").unwrap().confirmado_por == 1);
    }

    #[test]
    fn ms20004_ms20073_e_ms20002_geram_o_solido() {
        // MS20004-10 (1/4-28, pega 5/8): cabeça cilíndrica Ø0,433 × 0,250 com sextavado interno de 0,189 pol
        let t = tamanho("ms20004", "MS20004").unwrap();
        let l = *t.comprimentos.iter().find(|l| (*l / POL - 1.125).abs() < 1e-3).unwrap();
        let (v, lo, hi) = medir(&fixador_documento("ms20004", "MS20004", Some(l)).unwrap());
        let (dk, k, d, s) = (0.433 * POL, 0.250 * POL, 0.250 * POL, 0.189 * POL);
        assert!((lo[2] + l).abs() < 1e-2 && (hi[2] - k).abs() < 1e-2 && (hi[0] - lo[0] - dk).abs() < 0.05);
        let socket = 3f64.sqrt() / 2. * s * s * (k / 2.);
        let esperado = PI * (dk / 2.).powi(2) * k + PI * (d / 2.).powi(2) * l - socket;
        assert!((v - esperado).abs() < 0.01 * esperado, "v {v} esperado {esperado}");
        // MS20073-04-10: hexágono de 0,4345 pol entre faces, altura 0,2185 pol
        let t = tamanho("ms20073", "MS20073-04").unwrap();
        let l = *t.comprimentos.iter().find(|l| (*l / POL - 1.031).abs() < 2e-3).unwrap();
        let (v, lo, hi) = medir(&fixador_documento("ms20073", "MS20073-04", Some(l)).unwrap());
        let (s, k, d) = (0.4345 * POL, 0.2185 * POL, 0.250 * POL);
        assert!((lo[2] + l).abs() < 1e-2 && (hi[2] - k).abs() < 1e-2);
        let esperado = 3f64.sqrt() / 2. * s * s * k + PI * (d / 2.).powi(2) * l;
        assert!((v - esperado).abs() < 0.01 * esperado, "v {v} esperado {esperado}");
        // MS20002-4: anel Ø0,531 por Ø0,256, espessura 0,062 pol, de z = -h a 0
        let (v, lo, hi) = medir(&fixador_documento("ms20002", "MS20002-4", None).unwrap());
        let (d2, d1, h) = (0.531 * POL, 0.256 * POL, 0.062 * POL);
        assert!((v - PI / 4. * (d2 * d2 - d1 * d1) * h).abs() < 0.5 && (lo[2] + h).abs() < 1e-3 && hi[2].abs() < 1e-3 && (hi[0] - lo[0] - d2).abs() < 0.05);
        // AN10-12 (traço -12: 1 1/4 pol de L base): gera com a cabeça de 0,934 pol entre faces
        let t = tamanho("an3", "AN10").unwrap();
        let l = t.comprimentos[0];
        let (_, lo, hi) = medir(&fixador_documento("an3", "AN10", Some(l)).unwrap());
        assert!((lo[2] + l).abs() < 1e-2 && (hi[2] - 0.344 * POL).abs() < 1e-2);
    }
}
