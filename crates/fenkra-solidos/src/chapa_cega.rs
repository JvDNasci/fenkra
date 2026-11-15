// SPDX-License-Identifier: LGPL-2.1-or-later
//! B40: dobra cega com furo de alívio aeronáutico.
//!
//! Como funciona no kernel: a dobra por linha comum trata a linha como infinita e gira toda a chapa
//! do lado móvel. Na dobra cega a linha a–b termina dentro da chapa e só a faixa entre as duas
//! pontas gira. O corpo é dividido por uma caixa que segue a linha (largura = comprimento a–b,
//! atravessando a espessura e o lado móvel): a faixa passa por `bend_line` (a mesma conta de
//! compensação, o mesmo mapa de giro) e o resto da chapa fica plano e é fundido de volta. Antes da
//! divisão, um furo de alívio é aberto em cada ponta da linha: sem ele a chapa rasga no canto da
//! dobra. O mínimo do furo é `max(3 mm, 2·t + r)`, regra do projeto para que o furo cubra a zona
//! dobrada nas duas pontas; um furo menor pedido é subido a esse mínimo, com aviso.
//!
//! Planificada: a chapa plana mantém só os furos de alívio (nada gira), e o volume bate com o da
//! dobrada, porque a faixa dobrada conserva o comprimento desenvolvido.

use super::*;

/// Diâmetro do furo de alívio e se o pedido foi subido ao mínimo. `pedido` vazio usa o mínimo.
pub fn diametro_alivio(t: f64, r: f64, pedido: Option<f64>) -> (f64, bool) {
    let minimo = (2. * t + r).max(3.);
    match pedido {
        Some(d) if d >= minimo => (d, false),
        Some(_) => (minimo, true),
        None => (minimo, false),
    }
}

impl Ctx<'_> {
    /// Dobra cega de uma linha. Devolve o corpo dobrado (`None` quando só se mapeia, como na
    /// planificada), o corpo já com os furos de alívio e o mapa de giro da faixa.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bend_line_cega(&self, cur: &Shape, p: V3, q: V3, ang: f64, r: f64, kf: f64, t: f64, flip: bool, down: bool, delta: f64, build: bool, dia: f64) -> Result<(Option<Shape>, Shape, SheetMap), String> {
        let k = |e: KernelError| e.0;
        let mid = geo3d::lerp(p, q, 0.5);
        let (on, n) = cur.surface_near(mid).map_err(k)?;
        let e = geo3d::unit(geo3d::sub(q, p)).ok_or("os dois pontos da linha coincidem")?;
        let pa = geo3d::sub(p, geo3d::mul(n, geo3d::dot(geo3d::sub(p, on), n)));
        let len = geo3d::dist(p, q);
        if len < dia {
            return Err(format!("a dobra cega tem {len:.2} mm, menos que o furo de alívio ({dia:.2} mm): alongue a linha ou reduza o furo"));
        }
        let (lo, hi) = cur.bbox().ok_or("corpo sem dimensões")?;
        let big = geo3d::dist(lo, hi) * 2. + 10.;
        // furos de alívio nas duas pontas da linha, atravessando a chapa
        let furo = |c: V3| Shape::hole_tool(geo3d::add(c, geo3d::mul(n, 1.)), geo3d::mul(n, -1.), dia, t * 2. + 2., HoleShape::Simple).map_err(k);
        let furos = furo(pa)?.compound(&furo(geo3d::add(pa, geo3d::mul(e, len)))?);
        let com_furos = cur.boolean(&furos, Bool::Cut).map_err(k)?;
        // a caixa da faixa: de uma ponta à outra, sem limite no sentido de y (lado móvel e fixo)
        let y = fenkra_kernel::cross(n, e);
        let o = geo3d::sub(geo3d::sub(pa, geo3d::mul(n, t * 2. + 1.)), geo3d::mul(y, big));
        let faixa = Shape::box_tool(o, e, n, [len, 2. * big, t * 2. + 2.]).map_err(k)?;
        let tira = com_furos.boolean(&faixa, Bool::Common).map_err(k)?;
        let resto = com_furos.boolean(&faixa, Bool::Cut).map_err(k)?;
        if tira.is_null() {
            return Err("a linha da dobra cega não encosta na chapa".into());
        }
        let (dobrada, map) = self.bend_line(&tira, p, q, ang, r, kf, t, flip, down, delta, build)?;
        match dobrada {
            Some(d) if !resto.is_null() => Ok((Some(d.boolean(&resto, Bool::Fuse).map_err(k)?), com_furos, map)),
            Some(d) => Ok((Some(d), com_furos, map)),
            None => Ok((None, com_furos, map)),
        }
    }
}
