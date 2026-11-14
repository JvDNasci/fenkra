// SPDX-License-Identifier: LGPL-2.1-or-later
//! Posição do sketch por referência (B437): de onde vem a origem e o que o eixo H segue.
//!
//! O plano-base vem da referência do plano (`Sketch.plane`). Sobre ele, a origem pode ser a da
//! peça, um ponto projetado, a interseção de duas linhas, o ponto médio de uma aresta ou o centro de
//! uma face; a direção H pode seguir um eixo global, uma linha, um ponto ou outro plano. Tudo é
//! reencontrado a cada cálculo pelo mesmo caminho da geometria externa (`link_world`), então a
//! posição acompanha o modelo. Se a referência some, o sketch não quebra a peça: volta para a posição
//! automática e avisa.

use crate::geo3d::{self, V3};
use crate::profile;
use crate::{origin_axis, Ctx, Geo};
use fenkra_core::sketch::{LinkPolicy, LinkShape, LinkSrc, SkLink};
use fenkra_core::sketch_origem::{OrigemSk, OrientaSk};
use fenkra_core::{Id, Sketch};
use fenkra_kernel::Frame;

/// Ponto médio pelo comprimento de uma polilinha.
pub fn meio_da_polilinha(p: &[V3]) -> Option<V3> {
    let total: f64 = p.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum();
    if p.len() < 2 || total <= 1e-12 {
        return p.first().copied();
    }
    let mut falta = total / 2.;
    for w in p.windows(2) {
        let d = geo3d::dist(w[0], w[1]);
        if d >= falta && d > 0. {
            return Some(geo3d::lerp(w[0], w[1], falta / d));
        }
        falta -= d;
    }
    p.last().copied()
}

/// Interseção, no plano `f`, das retas que passam pelas pontas de cada polilinha.
pub fn intersecao_no_plano(f: &Frame, a: &[V3], b: &[V3]) -> Result<V3, String> {
    let uv = |q: V3| {
        let d = geo3d::sub(q, f.origin);
        [geo3d::dot(d, f.x), geo3d::dot(d, f.y())]
    };
    let (Some(a0), Some(a1), Some(b0), Some(b1)) = (a.first(), a.last(), b.first(), b.last()) else { return Err("linha vazia".into()) };
    let (p, r) = (uv(*a0), uv(*a1));
    let (q, s) = (uv(*b0), uv(*b1));
    let d1 = [r[0] - p[0], r[1] - p[1]];
    let d2 = [s[0] - q[0], s[1] - q[1]];
    let den = d1[0] * d2[1] - d1[1] * d2[0];
    let escala = (d1[0].hypot(d1[1]) * d2[0].hypot(d2[1])).max(1e-12);
    if den.abs() / escala < 1e-9 {
        return Err("as duas linhas são paralelas no plano do sketch".into());
    }
    let t = ((q[0] - p[0]) * d2[1] - (q[1] - p[1]) * d2[0]) / den;
    Ok(f.point(p[0] + d1[0] * t, p[1] + d1[1] * t))
}

impl Ctx<'_> {
    /// Pontos 3D de uma referência, pelo mesmo caminho da geometria externa.
    fn pontos_da_fonte(&self, src: &LinkSrc, f: &Frame) -> Result<Vec<V3>, String> {
        let l = SkLink { id: Id(0), source: *src, intersect: false, shape: LinkShape::Poly { points: vec![], closed: false, isolated: false }, policy: LinkPolicy::Associativo };
        let p = self.link_world(&l, f)?;
        if p.is_empty() {
            return Err("a referência não tem pontos".into());
        }
        Ok(p)
    }

    /// Origem no espaço que o modo pede (`None`: a do plano-base).
    fn origem_3d(&self, modo: &OrigemSk, f: &Frame) -> Result<Option<V3>, String> {
        Ok(Some(match modo {
            OrigemSk::Auto => return Ok(None),
            OrigemSk::OrigemPeca => [0.; 3],
            OrigemSk::Ponto(s) => self.pontos_da_fonte(s, f)?[0],
            OrigemSk::Intersecao(a, b) => intersecao_no_plano(f, &self.pontos_da_fonte(a, f)?, &self.pontos_da_fonte(b, f)?)?,
            OrigemSk::PontoMedio(s) => meio_da_polilinha(&self.pontos_da_fonte(s, f)?).ok_or("linha vazia")?,
            OrigemSk::CentroFace(s) => {
                let face = match s {
                    LinkSrc::Face { point } => self.face_of_bodies(*point)?,
                    LinkSrc::Ref(id) => match self.geo(*id, 0)? {
                        Geo::Surface(sh) => sh,
                        _ => return Err("a referência não é uma superfície".into()),
                    },
                    _ => return Err("escolha uma face".into()),
                };
                face.mass().center
            }
        }))
    }

    /// Direção que H segue (`None`: a do plano-base).
    fn direcao_h(&self, o: &OrientaSk, f: &Frame, origem: V3) -> Result<Option<V3>, String> {
        Ok(Some(match o {
            OrientaSk::Auto => return Ok(None),
            OrientaSk::Eixo(a) => origin_axis(*a),
            OrientaSk::Linha(s) => {
                let p = self.pontos_da_fonte(s, f)?;
                geo3d::sub(*p.last().unwrap(), p[0])
            }
            OrientaSk::Ponto(s) => geo3d::sub(self.pontos_da_fonte(s, f)?[0], origem),
            OrientaSk::Plano(pr) => geo3d::cross(f.normal, self.plane(pr, 0)?.normal),
        }))
    }

    /// Plano do sketch com a posição própria. A referência perdida não derruba a peça: a parte que
    /// falhou volta a ser automática e o aviso diz qual foi.
    pub(crate) fn frame_do_sketch(&self, s: &Sketch) -> Result<(Frame, Vec<String>), String> {
        let base = self.plane(&s.plane, 0)?;
        let p = &s.data.pos;
        if p.e_automatica() {
            return Ok((profile::posiciona(base, p), vec![]));
        }
        let mut avisos = vec![];
        let o3 = self.origem_3d(&p.modo, &base).unwrap_or_else(|e| {
            avisos.push(format!("origem do sketch ({}): {e}; ficou a automática", p.modo.nome()));
            None
        });
        let h3 = self.direcao_h(&p.orienta, &base, o3.unwrap_or(base.origin)).unwrap_or_else(|e| {
            avisos.push(format!("orientação do sketch ({}): {e}; ficou a automática", p.orienta.nome()));
            None
        });
        let f = profile::posiciona_em(base, p, o3, h3).unwrap_or_else(|e| {
            avisos.push(format!("orientação do sketch: {e}; ficou a automática"));
            profile::posiciona_em(base, p, o3, None).unwrap_or(base)
        });
        Ok((f, avisos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meio_e_intersecao() {
        let m = meio_da_polilinha(&[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]]).unwrap();
        assert!(geo3d::dist(m, [10., 0., 0.]) < 1e-12);
        let f = Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] };
        let p = intersecao_no_plano(&f, &[[0., 5., 3.], [10., 5., 3.]], &[[2., 0., -1.], [2., 10., 7.]]).unwrap();
        assert!(geo3d::dist(p, [2., 5., 0.]) < 1e-9, "{p:?}");
        assert!(intersecao_no_plano(&f, &[[0., 0., 0.], [1., 0., 0.]], &[[0., 1., 0.], [5., 1., 0.]]).is_err());
    }
}
