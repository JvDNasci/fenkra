// SPDX-License-Identifier: LGPL-2.1-or-later
//! Origem e eixos H e V do sketch (B437).
//!
//! Todo sketch tem três entidades fixas: o ponto da origem e as linhas de construção dos eixos H e V.
//! São pontos e linhas comuns para o solver, então entram em qualquer restrição sem caso especial:
//! coincidente vira ponto na curva, distância de um ponto ao eixo é distância ponto-linha, paralela
//! e perpendicular usam a direção da linha. As linhas vão de `-ALCANCE` a `+ALCANCE`: como o solver
//! trata linha como reta infinita em ponto-na-curva e distância, o comprimento só importa para o
//! clique e o snap acharem o eixo em qualquer ponto da tela. As pontas ficam escondidas.
//!
//! Onde a origem e os eixos ficam no espaço é a posição do sketch (`SketchPos`): automática pela
//! referência do plano, ou escolhida pelo usuário. Os pontos do sketch não mudam quando a posição
//! muda: muda o sistema de coordenadas em que eles estão.

use crate::sketch::{Curve, LinkSrc, SketchData};
use crate::{Id, OriginAxis, PlaneRef};
use serde::{Deserialize, Serialize};

/// Meio comprimento das linhas dos eixos, em mm.
pub const ALCANCE: f64 = 1.0e4;

/// As entidades da origem e dos eixos dentro do sketch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EixosSk {
    /// ponto fixo da origem
    pub origem: Id,
    /// linha de construção do eixo H (de -ALCANCE a +ALCANCE em H)
    pub h: Id,
    /// linha de construção do eixo V
    pub v: Id,
}

/// De onde vem a origem do sketch.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum OrigemSk {
    /// a origem do plano-base (a referência do plano decide)
    #[default]
    Auto,
    /// a origem da peça projetada no plano
    OrigemPeca,
    /// um ponto do modelo (vértice, ponto de referência, ponto de outro sketch) projetado no plano
    Ponto(LinkSrc),
    /// a interseção de duas linhas, projetadas no plano
    Intersecao(LinkSrc, LinkSrc),
    /// o ponto médio de uma aresta ou linha
    PontoMedio(LinkSrc),
    /// o centro (centroide) de uma face
    CentroFace(LinkSrc),
}

/// O que o eixo H segue.
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum OrientaSk {
    /// o H do plano-base
    #[default]
    Auto,
    /// um eixo global projetado no plano
    Eixo(OriginAxis),
    /// a direção de uma aresta ou linha projetada no plano
    Linha(LinkSrc),
    /// da origem para um ponto
    Ponto(LinkSrc),
    /// a reta em que outro plano corta o plano do sketch
    Plano(PlaneRef),
}

impl OrigemSk {
    pub fn nome(&self) -> &'static str {
        match self {
            OrigemSk::Auto => "Automática",
            OrigemSk::OrigemPeca => "Origem da peça",
            OrigemSk::Ponto(_) => "Ponto projetado",
            OrigemSk::Intersecao(..) => "Interseção de duas linhas",
            OrigemSk::PontoMedio(_) => "Ponto médio",
            OrigemSk::CentroFace(_) => "Centro da face",
        }
    }
    /// Quantas escolhas no 3D o modo pede.
    pub fn escolhas(&self) -> usize {
        match self {
            OrigemSk::Auto | OrigemSk::OrigemPeca => 0,
            OrigemSk::Intersecao(..) => 2,
            _ => 1,
        }
    }
    fn fontes(&mut self) -> Vec<&mut LinkSrc> {
        match self {
            OrigemSk::Auto | OrigemSk::OrigemPeca => vec![],
            OrigemSk::Ponto(a) | OrigemSk::PontoMedio(a) | OrigemSk::CentroFace(a) => vec![a],
            OrigemSk::Intersecao(a, b) => vec![a, b],
        }
    }
}

impl OrientaSk {
    pub fn nome(&self) -> &'static str {
        match self {
            OrientaSk::Auto => "Automática",
            OrientaSk::Eixo(OriginAxis::X) => "Eixo X",
            OrientaSk::Eixo(OriginAxis::Y) => "Eixo Y",
            OrientaSk::Eixo(OriginAxis::Z) => "Eixo Z",
            OrientaSk::Linha(_) => "Linha",
            OrientaSk::Ponto(_) => "Ponto",
            OrientaSk::Plano(_) => "Plano",
        }
    }
}

/// Fonte que aponta para outro objeto do documento (referência ou sketch).
fn walk_fonte(s: &mut LinkSrc, ids: &mut dyn FnMut(&mut Id)) {
    match s {
        LinkSrc::Ref(x) => ids(x),
        LinkSrc::SketchCurve { sketch, .. } => ids(sketch),
        _ => {}
    }
}

impl crate::sketch::SketchPos {
    /// Percorre os ids que a posição referencia (para renomear, apagar e ordenar dependências).
    pub fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        for f in self.modo.fontes() {
            walk_fonte(f, ids);
        }
        match &mut self.orienta {
            OrientaSk::Linha(s) | OrientaSk::Ponto(s) => walk_fonte(s, ids),
            OrientaSk::Plano(p) => p.walk(ids),
            _ => {}
        }
    }
    /// Posição que um arquivo antigo tinha: origem do plano mais o deslocamento, sem referência.
    pub fn e_automatica(&self) -> bool {
        self.modo == OrigemSk::Auto && self.orienta == OrientaSk::Auto
    }
}

impl SketchData {
    /// Origem e eixos do sketch, criando o que faltar. Um ponto fixo em (0, 0) que já marque a
    /// origem (sketches antigos o criam ao clicar na origem) vira a origem, sem duplicar.
    pub fn garantir_eixos(&mut self, al: &mut impl FnMut() -> Id) -> EixosSk {
        if let Some(e) = self.eixos.filter(|e| self.point(e.origem).is_some() && self.curve(e.h).is_some() && self.curve(e.v).is_some()) {
            return e;
        }
        let ja = self.eixos.map(|e| e.origem).filter(|o| self.point(*o).is_some()).or_else(|| {
            self.points.iter().find(|p| p.fixed && p.x.abs() < 1e-12 && p.y.abs() < 1e-12 && self.curves.iter().any(|c| matches!(c.geom, Curve::Point { p: q } if q == p.id))).map(|p| p.id)
        });
        let origem = match ja {
            Some(o) => o,
            None => {
                let o = self.add_point(al, 0., 0.);
                self.point_mut(o).unwrap().fixed = true;
                self.add_curve(al, Curve::Point { p: o }, true);
                o
            }
        };
        let mut linha = |sk: &mut SketchData, a: [f64; 2], b: [f64; 2]| {
            let (pa, pb) = (sk.add_point(al, a[0], a[1]), sk.add_point(al, b[0], b[1]));
            sk.point_mut(pa).unwrap().fixed = true;
            sk.point_mut(pb).unwrap().fixed = true;
            sk.add_curve(al, Curve::Line { a: pa, b: pb }, true)
        };
        let h = match self.eixos.map(|e| e.h).filter(|c| self.curve(*c).is_some()) {
            Some(c) => c,
            None => linha(self, [-ALCANCE, 0.], [ALCANCE, 0.]),
        };
        let v = match self.eixos.map(|e| e.v).filter(|c| self.curve(*c).is_some()) {
            Some(c) => c,
            None => linha(self, [0., -ALCANCE], [0., ALCANCE]),
        };
        let e = EixosSk { origem, h, v };
        self.eixos = Some(e);
        e
    }

    /// A curva é um dos eixos.
    pub fn e_eixo(&self, curva: Id) -> bool {
        self.eixos.is_some_and(|e| e.h == curva || e.v == curva)
    }

    /// Ponta escondida de um eixo: não se desenha, não se clica e não vira snap.
    pub fn e_ponta_de_eixo(&self, ponto: Id) -> bool {
        let Some(e) = self.eixos else { return false };
        [e.h, e.v].iter().any(|c| matches!(self.curve(*c).map(|x| &x.geom), Some(Curve::Line { a, b }) if *a == ponto || *b == ponto))
    }

    /// Origem ou eixo: entidade que o usuário não apaga, não move e não apara.
    pub fn e_da_origem(&self, id: Id) -> bool {
        self.eixos.is_some_and(|e| e.origem == id) || self.e_eixo(id) || self.e_ponta_de_eixo(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::{Kind, SketchPos};

    fn aloc() -> impl FnMut() -> Id {
        let mut n = 0;
        move || {
            n += 1;
            Id(n)
        }
    }

    #[test]
    fn eixos_nascem_uma_vez_e_reaproveitam_a_origem_antiga() {
        let mut al = aloc();
        let mut sk = SketchData::default();
        // origem como os sketches antigos a criavam
        let o = sk.add_point(&mut al, 0., 0.);
        sk.point_mut(o).unwrap().fixed = true;
        sk.add_curve(&mut al, Curve::Point { p: o }, true);
        let e = sk.garantir_eixos(&mut al);
        assert_eq!(e.origem, o);
        let n = (sk.points.len(), sk.curves.len());
        assert_eq!(sk.garantir_eixos(&mut al), e);
        assert_eq!((sk.points.len(), sk.curves.len()), n, "a segunda chamada não cria nada");
        assert!(sk.e_eixo(e.h) && sk.e_eixo(e.v) && !sk.e_eixo(o));
        assert!(sk.e_da_origem(o));
        // eixos são construção: não fecham perfil
        assert!(sk.loops().0.is_empty());
    }

    #[test]
    fn restricoes_aos_eixos_seguram_a_geometria() {
        let mut al = aloc();
        let mut sk = SketchData::default();
        let e = sk.garantir_eixos(&mut al);
        let a = sk.add_point(&mut al, 3., 7.);
        let b = sk.add_point(&mut al, 25., 11.);
        let l = sk.add_curve(&mut al, Curve::Line { a, b }, false);
        // linha paralela ao eixo H, ponta a no eixo V, a 5 do eixo H; ponta b a 30 do eixo V
        sk.add_constraint(&mut al, Kind::Parallel(l, e.h));
        sk.add_constraint(&mut al, Kind::PointOnCurve(a, e.v));
        sk.add_constraint(&mut al, Kind::PointLineDistance(a, e.h, "5".into()));
        sk.add_constraint(&mut al, Kind::PointLineDistance(b, e.v, "30".into()));
        let r = sk.solve(&Default::default(), &[]);
        assert!(r.converged && r.errors.is_empty(), "{r:?}");
        let (pa, pb) = (sk.pos(a), sk.pos(b));
        assert!(pa[0].abs() < 1e-6 && (pa[1].abs() - 5.).abs() < 1e-6, "{pa:?}");
        assert!((pb[1] - pa[1]).abs() < 1e-6 && (pb[0].abs() - 30.).abs() < 1e-6, "{pb:?}");
        // os eixos não se mexem
        assert_eq!(sk.pos(e.origem), [0., 0.]);
        assert_eq!(r.dof, 0, "linha inteira presa aos eixos");
    }

    #[test]
    fn coincidente_com_a_origem_e_vertical_ao_eixo_v() {
        let mut al = aloc();
        let mut sk = SketchData::default();
        let e = sk.garantir_eixos(&mut al);
        let a = sk.add_point(&mut al, 2., -3.);
        let b = sk.add_point(&mut al, 6., 20.);
        let l = sk.add_curve(&mut al, Curve::Line { a, b }, false);
        sk.add_constraint(&mut al, Kind::Coincident(a, e.origem));
        sk.add_constraint(&mut al, Kind::Parallel(l, e.v));
        sk.add_constraint(&mut al, Kind::Length(l, "40".into()));
        let r = sk.solve(&Default::default(), &[]);
        assert!(r.converged, "{r:?}");
        assert!(sk.pos(a)[0].abs() < 1e-6 && sk.pos(a)[1].abs() < 1e-6);
        assert!(sk.pos(b)[0].abs() < 1e-6 && (sk.pos(b)[1].abs() - 40.).abs() < 1e-6);
    }

    #[test]
    fn arquivo_antigo_le_posicao_automatica_com_deslocamento() {
        let antigo = r#"{"origem":[12.5,-4.0],"trocar":false,"inverter_h":true,"inverter_v":false}"#;
        let p: SketchPos = serde_json::from_str(antigo).unwrap();
        assert!(p.e_automatica());
        assert_eq!(p.origem, [12.5, -4.0]);
        assert!(p.inverter_h);
        // e um sketch antigo, sem eixos, também abre
        let s: SketchData = serde_json::from_str(r#"{"points":[],"curves":[],"constraints":[]}"#).unwrap();
        assert!(s.eixos.is_none() && s.pos.e_automatica());
    }

    #[test]
    fn posicao_com_referencia_vai_e_volta_no_arquivo() {
        let p = SketchPos { modo: OrigemSk::Intersecao(LinkSrc::Edge { mid: [1., 2., 3.] }, LinkSrc::Ref(Id(7))), orienta: OrientaSk::Plano(PlaneRef::Datum(Id(9))), origem: [1., 2.], ..Default::default() };
        let t = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<SketchPos>(&t).unwrap(), p);
        let mut q = p;
        let mut vistos = vec![];
        q.walk(&mut |x| vistos.push(*x));
        assert_eq!(vistos, vec![Id(7), Id(9)]);
    }
}
