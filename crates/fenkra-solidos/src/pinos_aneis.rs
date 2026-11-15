// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pinos, anéis de retenção e pino com colar (B63).
//!
//! **Convenção** (a dos fixadores): eixo em Z, face de apoio em `z = 0`, corpo para `-Z`.
//! - Pino cilíndrico: cilindro de Ø `d` e comprimento `L`. O chanfro `c` da norma não é modelado.
//! - Pino elástico (ranhurado): tubo de Ø `d` (o pino já montado no furo), parede `s` e comprimento
//!   `L`. A ranhura longitudinal não é modelada.
//! - Cupilha: as duas hastes juntas viram um cilindro de Ø `dmax` e comprimento `L`; o olho vira um
//!   disco de Ø `cmax` e altura `dmax` sobre a face de apoio. Forma simplificada: o fio dobrado de
//!   verdade tem seção em meia-lua e olho vazado.
//! - Anel externo (eixo): coroa de espessura `s` com furo de Ø `d3` (livre) e largura radial `b`,
//!   logo Ø externo `d3 + 2b`. Anel interno (furo): Ø externo `d3` e furo de `d3 - 2b`. A abertura e
//!   os olhais não são modelados.
//! - Pino com colar: cabeça cilíndrica de Ø `dk` × `k`, haste de Ø `d` e comprimento `L`. Quando o
//!   tamanho traz `Dc` e `hc`, o colar de fechamento (cilindro de Ø `Dc` e altura `hc`) fica logo
//!   abaixo da pega máxima `L - sobra`. A rosca e o recesso sextavado da ponta não são modelados.

use crate::biblioteca::{Familia, Tamanho};
use crate::fixadores::Montador;
use fenkra_core::*;

fn medida(fam: &Familia, t: &Tamanho, c: &str) -> Result<f64, String> {
    t.valor(c).ok_or_else(|| format!("{} {}: falta '{c}'", fam.id, t.designacao))
}

/// Pino cilíndrico, elástico ou cupilha.
pub(crate) fn pino(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    m.param("L", &l.to_string())?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    match fam.gerador.as_str() {
        "pino_cilindrico" => {
            m.param("c", &medida(fam, t, "c")?.to_string())?;
            m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)
        }
        "pino_elastico" => {
            let s = medida(fam, t, "s")?;
            let d = t.diametro_nominal().ok_or("pino sem diâmetro")?;
            if 2. * s >= d {
                return Err(format!("{} {}: a parede pede menos que o raio", fam.id, t.designacao));
            }
            m.param("s", &s.to_string())?;
            m.prim("Corpo", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)?;
            let p_furo = m.plano("Plano_furo", "-L-1")?;
            m.prim("Furo", PrimKind::Cilindro, p_furo, ["(d-2*s)/2", "L+2", "0", "0"], BoolOp::Cut)
        }
        _ => {
            let (dmax, cmax) = (medida(fam, t, "dmax")?, medida(fam, t, "cmax")?);
            if cmax <= dmax {
                return Err(format!("{} {}: o olho precisa ser maior que as hastes", fam.id, t.designacao));
            }
            m.param("dmax", &dmax.to_string())?;
            m.param("cmax", &cmax.to_string())?;
            m.prim("Hastes", PrimKind::Cilindro, p_corpo, ["dmax/2", "L", "0", "0"], BoolOp::Add)?;
            let topo = PlaneRef::Origin(OriginPlane::XY);
            m.prim("Olho", PrimKind::Cilindro, topo, ["cmax/2", "dmax", "0", "0"], BoolOp::Add)
        }
    }
}

/// Anel de retenção externo (DIN 471) ou interno (DIN 472).
pub(crate) fn anel(m: &mut Montador, fam: &Familia, t: &Tamanho) -> Result<(), String> {
    let (s, d3, b) = (medida(fam, t, "s")?, medida(fam, t, "d3")?, medida(fam, t, "b")?);
    let externo = fam.gerador == "anel_externo";
    let (de, di) = if externo { (d3 + 2. * b, d3) } else { (d3, d3 - 2. * b) };
    if !(di > 0. && de > di) {
        return Err(format!("{} {}: coroa sem largura (Ø externo {de}, furo {di})", fam.id, t.designacao));
    }
    m.param("s", &s.to_string())?;
    m.param("d3", &d3.to_string())?;
    m.param("b", &b.to_string())?;
    let p_corpo = m.plano("Plano_anel", "-s")?;
    let (re, ri) = if externo { ("(d3+2*b)/2", "d3/2") } else { ("d3/2", "(d3-2*b)/2") };
    m.prim("Anel", PrimKind::Cilindro, p_corpo, [re, "s", "0", "0"], BoolOp::Add)?;
    let p_furo = m.plano("Plano_furo", "-s-1")?;
    m.prim("Furo", PrimKind::Cilindro, p_furo, [ri, "s+2", "0", "0"], BoolOp::Cut)
}

/// Pino de cabeça com colar de fechamento.
pub(crate) fn pino_colar(m: &mut Montador, fam: &Familia, t: &Tamanho, l: f64) -> Result<(), String> {
    let (dk, k, sobra) = (medida(fam, t, "dk")?, medida(fam, t, "k")?, medida(fam, t, "sobra")?);
    let d = t.diametro_nominal().ok_or("pino sem diâmetro")?;
    if !(dk > d && l > sobra) {
        return Err(format!("{} {}: cabeça maior que a haste e L maior que a sobra do colar", fam.id, t.designacao));
    }
    m.param("L", &l.to_string())?;
    m.param("dk", &dk.to_string())?;
    m.param("k", &k.to_string())?;
    m.param("sobra", &sobra.to_string())?;
    // a maior pilha que este L fecha
    m.param("pega_max", "L-sobra")?;
    let topo = PlaneRef::Origin(OriginPlane::XY);
    m.prim("Cabeca", PrimKind::Cilindro, topo, ["dk/2", "k", "0", "0"], BoolOp::Add)?;
    let p_corpo = m.plano("Plano_corpo", "-L")?;
    m.prim("Haste", PrimKind::Cilindro, p_corpo, ["d/2", "L", "0", "0"], BoolOp::Add)?;
    if let (Some(dc), Some(hc)) = (t.valor("Dc"), t.valor("hc")) {
        if !(dc > d && hc <= sobra) {
            return Err(format!("{} {}: colar maior que a haste e mais baixo que a sobra", fam.id, t.designacao));
        }
        m.param("Dc", &dc.to_string())?;
        m.param("hc", &hc.to_string())?;
        let p_colar = m.plano("Plano_colar", "-pega_max-hc")?;
        m.prim("Colar", PrimKind::Cilindro, p_colar, ["Dc/2", "hc", "0", "0"], BoolOp::Add)?;
    }
    Ok(())
}
