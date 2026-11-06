// SPDX-License-Identifier: LGPL-2.1-or-later
//! A geometria externa muda de estrutura junto com a origem (B97).
//!
//! `projecao` diz qual é a forma; aqui ela entra no sketch. Três coisas podem mudar: o tipo da curva
//! (linha que vira arco ao editar o 3D, círculo que vira elipse ao inclinar o plano), o número de
//! travessias de uma curva pelo plano e o número de cantos de uma polilinha.
//!
//! O cálculo trabalha numa cópia do sketch e não pode alocar ids no documento. Por isso os ids novos
//! são derivados do vínculo, numa faixa só deles: o mesmo cálculo sempre dá os mesmos ids, e o pedido
//! ([`OpVinculo`]) pode ser reaplicado no documento pelo editor, sem que a geometria mude de nome.

use crate::projecao::{elipse_em, Exata};
use crate::sketch::{fit_circle2, resample, smooth_runs, Curve, Kind, LinkShape, SkLink};
use crate::{Id, SketchData};
use std::collections::HashSet;
use std::f64::consts::TAU;

type P = [f64; 2];

/// Primeiro id da faixa do cálculo. O documento aloca de baixo para cima e nunca chega aqui.
const FAIXA_DO_VINCULO: u64 = 1 << 62;
/// Bits da ordem dentro do vínculo.
const BITS_DA_ORDEM: u32 = 12;
/// Polilinha com mais vértices que isto é uma curva amostrada, não uma sequência de cantos.
const MAX_CANTOS: usize = 64;

fn dist(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Ids novos da geometria externa: o vínculo e a ordem, pulando o que já existe no sketch.
struct IdsDoVinculo {
    base: u64,
    ordem: u64,
    usados: HashSet<u64>,
}

impl IdsDoVinculo {
    fn proximo(&mut self) -> Id {
        loop {
            let id = self.base | (self.ordem & ((1 << BITS_DA_ORDEM) - 1));
            self.ordem += 1;
            if self.usados.insert(id) {
                return Id(id);
            }
        }
    }
}

/// O que passa da forma antiga para a nova. Quem se apoiava na ponta continua apoiado nela, mesmo que
/// a linha tenha virado arco.
#[derive(Clone, Copy, Default)]
pub(crate) struct Papeis {
    centro: Option<Id>,
    s: Option<Id>,
    e: Option<Id>,
    curva: Option<Id>,
}

/// O que o cálculo pede para a geometria externa. Vale igual na cópia do cálculo e no documento.
#[derive(Clone, PartialEq, Debug)]
pub enum OpVinculo {
    Exata(Id, Exata),
    Polilinha(Id, Vec<P>),
    /// travessias de uma curva pelo plano
    Pontos(Id, Vec<P>),
}

/// O que o pedido fez. Repetir um pedido já aplicado não refaz nada.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Efeito {
    /// pontos ou curvas nasceram ou saíram
    pub refeito: bool,
    /// restrições que não servem à forma nova e saíram
    pub perdidas: usize,
    /// pontos sem travessia que ficaram no lugar porque algo depende deles
    pub sobras: usize,
}

enum Nova<'a> {
    Exata(&'a Exata),
    Poli(&'a [P]),
}

fn dist_a_reta(p: P, a: P, b: P) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l = d[0].hypot(d[1]);
    if l < 1e-12 {
        return dist(p, a);
    }
    ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]).abs() / l
}

/// Tira os vértices do meio de um trecho reto: sobram os cantos de verdade. A malha vem em precisão
/// simples e deixa um resíduo de micrômetros, daí a folga.
fn so_os_cantos(uv: &[P], fechada: bool) -> Vec<P> {
    const FOLGA: f64 = 1e-3;
    let n = uv.len();
    let mut out: Vec<P> = vec![];
    for i in 0..n {
        let antes = out.last().copied().or(fechada.then(|| uv[n - 1]));
        let depois = if i + 1 < n { Some(uv[i + 1]) } else { fechada.then(|| out.first().copied().unwrap_or(uv[0])) };
        match (antes, depois) {
            (Some(a), Some(b)) if dist_a_reta(uv[i], a, b) < FOLGA => {}
            _ => out.push(uv[i]),
        }
    }
    out
}

/// Polilinha sem o último ponto, quando ele repete o primeiro.
fn sem_fecho(uv: &[P]) -> (&[P], bool) {
    let fechada = uv.len() > 2 && dist(uv[0], uv[uv.len() - 1]) < 1e-4;
    (if fechada { &uv[..uv.len() - 1] } else { uv }, fechada)
}

/// Os cantos na ordem dos pontos guardados. Contorno fechado pode começar em outro canto ou ter virado
/// de sentido, e cada ponto deve seguir o canto mais perto do seu.
fn na_ordem_de(guardados: &[P], cantos: &[P], fechada: bool) -> Vec<P> {
    let n = cantos.len();
    let custo = |v: &[P]| guardados.iter().zip(v).map(|(a, b)| dist(*a, *b)).sum::<f64>();
    let invertida: Vec<P> = cantos.iter().rev().copied().collect();
    let mut melhor = cantos.to_vec();
    for base in [cantos, &invertida[..]] {
        for giro in 0..if fechada { n } else { 1 } {
            let v: Vec<P> = (0..n).map(|i| base[(i + giro) % n]).collect();
            if custo(&v) < custo(&melhor) - 1e-12 {
                melhor = v;
            }
        }
    }
    melhor
}

/// A restrição sobre a curva ainda faz sentido depois de ela mudar de tipo?
fn perde_sentido(k: &Kind, antes: &Curve, agora: &Curve) -> bool {
    if std::mem::discriminant(antes) == std::mem::discriminant(agora) {
        return false;
    }
    let redonda = |g: &Curve| matches!(g, Curve::Circle { .. } | Curve::Arc { .. });
    match k {
        Kind::PointOnCurve(..) => !matches!(agora, Curve::Line { .. } | Curve::Circle { .. } | Curve::Arc { .. }),
        Kind::Radius(..) | Kind::Diameter(..) | Kind::Concentric(..) | Kind::Equal(..) | Kind::Tangent(..) => !(redonda(antes) && redonda(agora)),
        _ => true,
    }
}

fn pontos_do_link(shape: &LinkShape) -> Vec<Id> {
    match shape {
        LinkShape::Poly { points, .. } => points.clone(),
        LinkShape::Line { a, b, .. } => vec![*a, *b],
        LinkShape::Circle { center, .. } => vec![*center],
        LinkShape::Arc { center, s, e, .. } | LinkShape::Ellipse { center, s, e, .. } => vec![*center, *s, *e],
    }
}

fn papeis_de(shape: &LinkShape) -> Papeis {
    match shape {
        LinkShape::Poly { points, .. } => Papeis { s: points.first().copied(), e: points.last().copied().filter(|_| points.len() > 1), ..Default::default() },
        LinkShape::Line { a, b, .. } => Papeis { s: Some(*a), e: Some(*b), ..Default::default() },
        LinkShape::Circle { center, .. } => Papeis { centro: Some(*center), ..Default::default() },
        // elipse inteira: as pontas são o próprio centro
        LinkShape::Ellipse { center, s, .. } if s == center => Papeis { centro: Some(*center), ..Default::default() },
        LinkShape::Arc { center, s, e, .. } | LinkShape::Ellipse { center, s, e, .. } => Papeis { centro: Some(*center), s: Some(*s), e: Some(*e), ..Default::default() },
    }
}

impl SketchData {
    /// Ponto fixo da geometria externa: o que já tinha o papel, ou um novo.
    fn ponto_externo(&mut self, al: &mut impl FnMut() -> Id, reuso: Option<Id>, q: P) -> Id {
        let id = reuso.filter(|p| self.point(*p).is_some()).unwrap_or_else(|| self.add_point(al, q[0], q[1]));
        if let Some(p) = self.point_mut(id) {
            (p.x, p.y, p.fixed) = (q[0], q[1], true);
        }
        id
    }

    fn curva_externa(&mut self, al: &mut impl FnMut() -> Id, reuso: Option<Id>, geom: Curve, construction: bool) -> Id {
        match reuso.and_then(|c| self.curves.iter_mut().find(|x| x.id == c)) {
            Some(c) => {
                c.geom = geom;
                c.id
            }
            None => self.add_curve(al, geom, construction),
        }
    }

    /// Monta a forma exata, aproveitando os papéis que vieram da forma anterior.
    pub(crate) fn monta_exata(&mut self, al: &mut impl FnMut() -> Id, forma: &Exata, r: Papeis, construction: bool) -> LinkShape {
        match *forma {
            Exata::Ponto(q) => {
                let p = self.ponto_externo(al, r.s.or(r.centro), q);
                self.curva_externa(al, r.curva, Curve::Point { p }, construction);
                LinkShape::Poly { points: vec![p], closed: false, isolated: false }
            }
            Exata::Reta(a, b) => {
                let (a, b) = (self.ponto_externo(al, r.s, a), self.ponto_externo(al, r.e, b));
                let curve = self.curva_externa(al, r.curva, Curve::Line { a, b }, construction);
                LinkShape::Line { a, b, curve }
            }
            Exata::Circulo { c, r: raio } => {
                let center = self.ponto_externo(al, r.centro, c);
                let curve = self.curva_externa(al, r.curva, Curve::Circle { c: center, r: raio }, construction);
                LinkShape::Circle { center, curve }
            }
            Exata::Arco { c, s, e } => {
                let (center, s, e) = (self.ponto_externo(al, r.centro, c), self.ponto_externo(al, r.s, s), self.ponto_externo(al, r.e, e));
                let curve = self.curva_externa(al, r.curva, Curve::Arc { c: center, s, e }, construction);
                LinkShape::Arc { center, s, e, curve }
            }
            Exata::Elipse { c, maior, menor, t0, t1 } => {
                let center = self.ponto_externo(al, r.centro, c);
                // inteira: as pontas não existem e o centro faz o papel delas
                let (s, e) = if t1 - t0 >= TAU - 1e-9 {
                    (center, center)
                } else {
                    (self.ponto_externo(al, r.s, elipse_em(c, maior, menor, t0)), self.ponto_externo(al, r.e, elipse_em(c, maior, menor, t1)))
                };
                let curve = self.curva_externa(al, r.curva, Curve::Ellipse { c: center, s, e, major: maior, minor: menor, t0, t1 }, construction);
                LinkShape::Ellipse { center, s, e, curve }
            }
        }
    }

    /// Círculo (se a polilinha fechada for um) ou polilinha de pontos fixos ligados por linhas.
    pub(crate) fn monta_poli(&mut self, al: &mut impl FnMut() -> Id, uv: &[P], r: Papeis, construction: bool) -> LinkShape {
        if let Some((c, raio)) = fit_circle2(uv) {
            return self.monta_exata(al, &Exata::Circulo { c, r: raio }, r, construction);
        }
        let (aberta, closed) = sem_fecho(uv);
        let n = aberta.len();
        let ids: Vec<Id> = aberta
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let reuso = if i == 0 { r.s } else if i + 1 == n { r.e } else { None };
                self.ponto_externo(al, reuso, *q)
            })
            .collect();
        if let [p] = ids[..] {
            self.curva_externa(al, r.curva, Curve::Point { p }, construction);
        }
        // trechos suaves (arcos, splines convertidas) viram spline; cantos e retas, linhas
        for run in smooth_runs(aberta, closed && ids.len() > 2) {
            if run.len() >= 4 {
                self.add_curve(al, Curve::Spline { pts: run.iter().map(|i| ids[*i]).collect() }, construction);
            } else {
                for w in run.windows(2) {
                    self.add_curve(al, Curve::Line { a: ids[w[0]], b: ids[w[1]] }, construction);
                }
            }
        }
        LinkShape::Poly { points: ids, closed, isolated: false }
    }

    /// Curvas da geometria externa. A polilinha não guarda as suas: são as que só usam os pontos dela.
    fn curvas_do_link(&self, shape: &LinkShape) -> Vec<Id> {
        match shape {
            LinkShape::Poly { points, .. } => self.curves.iter().filter(|c| Self::curve_points(&c.geom).iter().all(|p| points.contains(p))).map(|c| c.id).collect(),
            LinkShape::Line { curve, .. } | LinkShape::Circle { curve, .. } | LinkShape::Arc { curve, .. } | LinkShape::Ellipse { curve, .. } => vec![*curve],
        }
    }

    /// Alguma restrição se apoia nesta geometria externa?
    fn link_tem_dependentes(&self, shape: &LinkShape) -> bool {
        let meus: HashSet<Id> = pontos_do_link(shape).into_iter().chain(self.curvas_do_link(shape)).collect();
        self.constraints.iter().any(|k| k.kind.refs().iter().any(|r| meus.contains(r)))
    }

    /// A forma exata pode tomar o lugar do que o vínculo é hoje? Forma exata sempre pode, porque a
    /// origem manda. Polilinha só quando nada se apoia nos trechos dela, que deixariam de existir.
    pub fn link_aceita_forma(&self, link: Id) -> bool {
        match self.links.iter().find(|l| l.id == link).map(|l| &l.shape) {
            Some(s @ LinkShape::Poly { points, .. }) => points.len() == 1 || !self.link_tem_dependentes(s),
            Some(_) => true,
            None => false,
        }
    }

    fn ids_do_vinculo(&self, link: Id) -> IdsDoVinculo {
        let usados = self.points.iter().map(|p| p.id.0).chain(self.curves.iter().map(|c| c.id.0)).chain(self.constraints.iter().map(|c| c.id.0)).chain(self.links.iter().map(|l| l.id.0)).collect();
        IdsDoVinculo { base: FAIXA_DO_VINCULO | ((link.0 << BITS_DA_ORDEM) & (FAIXA_DO_VINCULO - 1)), ordem: 0, usados }
    }

    /// Troca a forma do vínculo. Pontas, centro e curva mantêm o id quando o papel existe na forma
    /// nova; o que sobra sai, com as restrições que perderam o sentido. Devolve quantas saíram.
    fn refaz_link(&mut self, i: usize, nova: Nova) -> usize {
        let SkLink { id: link, shape: antigo, .. } = self.links[i].clone();
        let curvas = self.curvas_do_link(&antigo);
        let pontos = pontos_do_link(&antigo);
        let construction = curvas.first().and_then(|c| self.curve(*c)).is_some_and(|c| c.construction);
        let mut r = papeis_de(&antigo);
        r.curva = if let [c] = curvas[..] { Some(c) } else { None };
        let geom_antes = r.curva.and_then(|c| self.curve(c)).map(|c| (c.id, c.geom.clone()));
        let mut ids = self.ids_do_vinculo(link);
        let mut al = || ids.proximo();
        // só a forma exata herda a curva: numa polilinha os trechos antigos saem e os novos nascem
        let reusada = if matches!(nova, Nova::Exata(_)) { r.curva } else { None };
        r.curva = reusada;
        let shape = match nova {
            Nova::Exata(f) => self.monta_exata(&mut al, f, r, construction),
            Nova::Poli(uv) => self.monta_poli(&mut al, uv, r, construction),
        };
        self.curves.retain(|c| !curvas.contains(&c.id) || Some(c.id) == reusada);
        self.spline_tan.retain(|t| !curvas.contains(&t.curve) || Some(t.curve) == reusada);
        let ficam = pontos_do_link(&shape);
        for p in pontos.into_iter().filter(|p| !ficam.contains(p)) {
            if self.curves.iter().any(|c| Self::curve_points(&c.geom).contains(&p)) {
                // outra curva do sketch se juntou a ele: fica, mas sem a origem para segurá-lo
                if let Some(x) = self.point_mut(p) {
                    x.fixed = false;
                }
            } else {
                self.points.retain(|x| x.id != p);
            }
        }
        let antes = self.constraints.len();
        let existe = |sk: &Self, x: Id| sk.point(x).is_some() || sk.curve(x).is_some();
        let fora: Vec<Id> = self
            .constraints
            .iter()
            .filter(|k| {
                let refs = k.kind.refs();
                let mudou = geom_antes.as_ref().is_some_and(|(c, g)| refs.contains(c) && self.curve(*c).is_some_and(|x| perde_sentido(&k.kind, g, &x.geom)));
                mudou || !refs.iter().all(|x| existe(self, *x))
            })
            .map(|k| k.id)
            .collect();
        self.constraints.retain(|k| !fora.contains(&k.id));
        self.links[i].shape = shape;
        antes - self.constraints.len()
    }

    /// Aplica o pedido do cálculo. Pode ser repetido: o que já está certo não muda.
    pub fn aplica_vinculo(&mut self, op: &OpVinculo) -> Efeito {
        match op {
            OpVinculo::Exata(l, forma) => {
                let Some(i) = self.links.iter().position(|x| x.id == *l) else { return Efeito::default() };
                if self.update_link_exata(*l, forma) {
                    return Efeito::default();
                }
                Efeito { refeito: true, perdidas: self.refaz_link(i, Nova::Exata(forma)), sobras: 0 }
            }
            OpVinculo::Polilinha(l, uv) => self.aplica_polilinha(*l, uv),
            OpVinculo::Pontos(l, uv) => self.aplica_pontos(*l, uv),
        }
    }

    fn aplica_polilinha(&mut self, link: Id, uv: &[P]) -> Efeito {
        let Some(i) = self.links.iter().position(|l| l.id == link) else { return Efeito::default() };
        let (aberta, fechada) = sem_fecho(uv);
        let cantos = so_os_cantos(aberta, fechada);
        match self.links[i].shape.clone() {
            LinkShape::Poly { points, closed, .. } if points.len() > 1 => {
                let guardados: Vec<P> = points.iter().map(|p| self.pos(*p)).collect();
                if cantos.len() == points.len() && fechada == closed {
                    // mesmos cantos: cada ponto vai para o seu, que reamostrar tiraria dos cantos
                    for (id, q) in points.iter().zip(na_ordem_de(&guardados, &cantos, fechada)) {
                        if let Some(p) = self.point_mut(*id) {
                            (p.x, p.y) = (q[0], q[1]);
                        }
                    }
                    return Efeito::default();
                }
                let mesma_conta = so_os_cantos(&guardados, closed).len() == cantos.len();
                if mesma_conta || cantos.len() > MAX_CANTOS || self.link_tem_dependentes(&self.links[i].shape) {
                    self.update_link(link, uv);
                    return Efeito::default();
                }
            }
            LinkShape::Circle { .. } if fit_circle2(uv).is_some() => {
                self.update_link(link, uv);
                return Efeito::default();
            }
            _ => {}
        }
        let mut q = if cantos.len() > MAX_CANTOS { resample(aberta, MAX_CANTOS) } else { cantos };
        if fechada {
            q.push(q[0]);
        }
        Efeito { refeito: true, perdidas: self.refaz_link(i, Nova::Poli(&q)), sobras: 0 }
    }

    /// Cada ponto segue a travessia mais perto da sua, porque a ordem delas muda junto com o 3D.
    /// Travessia nova nasce como ponto; ponto sem travessia sai, se nada depender dele.
    fn aplica_pontos(&mut self, link: Id, uv: &[P]) -> Efeito {
        let Some(i) = self.links.iter().position(|l| l.id == link) else { return Efeito::default() };
        let LinkShape::Poly { points, .. } = self.links[i].shape.clone() else { return Efeito::default() };
        let mut pares: Vec<(f64, usize, usize)> = vec![];
        for (a, p) in points.iter().enumerate() {
            let q = self.pos(*p);
            pares.extend(uv.iter().enumerate().map(|(b, t)| (dist(q, *t), a, b)));
        }
        pares.sort_by(|x, y| x.0.total_cmp(&y.0));
        let (mut travessia, mut dono) = (vec![None; points.len()], vec![false; uv.len()]);
        for (_, a, b) in pares {
            if travessia[a].is_none() && !dono[b] {
                travessia[a] = Some(b);
                dono[b] = true;
            }
        }
        let construction = self.curvas_do_link(&self.links[i].shape).first().and_then(|c| self.curve(*c)).is_some_and(|c| c.construction);
        let mut ids = self.ids_do_vinculo(link);
        let mut efeito = Efeito::default();
        let mut ficam: Vec<Id> = vec![];
        for (p, b) in points.iter().zip(&travessia) {
            if let Some(b) = b {
                if let Some(x) = self.point_mut(*p) {
                    (x.x, x.y) = (uv[*b][0], uv[*b][1]);
                }
                ficam.push(*p);
                continue;
            }
            let curva = self.curves.iter().find(|c| matches!(c.geom, Curve::Point { p: k } if k == *p)).map(|c| c.id);
            let preso = self.constraints.iter().any(|k| k.kind.refs().iter().any(|r| r == p || Some(*r) == curva))
                || self.curves.iter().any(|c| Some(c.id) != curva && Self::curve_points(&c.geom).contains(p));
            if preso {
                efeito.sobras += 1;
                ficam.push(*p);
            } else {
                self.curves.retain(|c| Some(c.id) != curva);
                self.points.retain(|x| x.id != *p);
                efeito.refeito = true;
            }
        }
        for (q, _) in uv.iter().zip(&dono).filter(|x| !*x.1) {
            let mut al = || ids.proximo();
            let p = self.ponto_externo(&mut al, None, *q);
            self.add_curve(&mut al, Curve::Point { p }, construction);
            ficam.push(p);
            efeito.refeito = true;
        }
        if let LinkShape::Poly { points, .. } = &mut self.links[i].shape {
            *points = ficam;
        }
        efeito
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::sketch::LinkSrc;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 100u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    const FONTE: LinkSrc = LinkSrc::Edge { mid: [0.; 3] };

    fn com_reta(al: &mut impl FnMut() -> Id) -> (SketchData, Id) {
        let mut sk = SketchData::default();
        let l = sk.add_link_exata(al, FONTE, false, &Exata::Reta([0., 0.], [10., 0.]), false);
        (sk, l)
    }

    fn pontas(sk: &SketchData) -> (Id, Id, Id) {
        let LinkShape::Line { a, b, curve } = sk.links[0].shape else { panic!("{:?}", sk.links[0].shape) };
        (a, b, curve)
    }

    #[test]
    fn linha_que_vira_arco_mantem_pontas_e_curva() {
        let mut al = ids();
        let (mut sk, l) = com_reta(&mut al);
        let (a, b, curva) = pontas(&sk);
        let solto = sk.add_point(&mut al, 5., 5.);
        // a ponta guarda quem se apoia nela; o comprimento não existe mais num arco
        let dist = sk.add_constraint(&mut al, Kind::Distance(a, solto, "7".into()));
        sk.add_constraint(&mut al, Kind::Length(curva, "10".into()));
        let op = OpVinculo::Exata(l, Exata::Arco { c: [5., 0.], s: [0., 0.], e: [10., 0.] });
        let ef = sk.aplica_vinculo(&op);
        assert_eq!((ef.refeito, ef.perdidas), (true, 1));
        let LinkShape::Arc { center, s, e, curve } = sk.links[0].shape else { panic!() };
        assert_eq!((s, e, curve), (a, b, curva));
        assert!(matches!(sk.curve(curva).unwrap().geom, Curve::Arc { .. }));
        assert!(sk.point(center).unwrap().fixed);
        assert_eq!(sk.pos(center), [5., 0.]);
        assert!(sk.constraints.iter().any(|k| k.id == dist));
        // repetir não muda nada: o editor reaplica o pedido com o resultado do cálculo anterior
        let copia = sk.clone();
        assert_eq!(sk.aplica_vinculo(&op), Efeito::default());
        assert_eq!(sk, copia);
    }

    #[test]
    fn ids_novos_nao_dependem_de_quem_calcula() {
        let mut al = ids();
        let (base, l) = com_reta(&mut al);
        let op = OpVinculo::Exata(l, Exata::Circulo { c: [5., 0.], r: 5. });
        let (mut x, mut y) = (base.clone(), base);
        x.aplica_vinculo(&op);
        y.aplica_vinculo(&op);
        assert_eq!(x, y);
        // e ficam fora do que o documento aloca
        let LinkShape::Circle { center, .. } = x.links[0].shape else { panic!() };
        assert!(center.0 >= FAIXA_DO_VINCULO);
    }

    #[test]
    fn circulo_que_vira_elipse_e_volta_guarda_o_centro() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let l = sk.add_link_exata(&mut al, FONTE, false, &Exata::Circulo { c: [1., 2.], r: 5. }, false);
        let LinkShape::Circle { center, curve } = sk.links[0].shape else { panic!() };
        let raio = sk.add_constraint(&mut al, Kind::Radius(curve, "5".into()));
        sk.aplica_vinculo(&OpVinculo::Exata(l, Exata::Elipse { c: [1., 2.], maior: [5., 0.], menor: 3., t0: 0., t1: TAU }));
        assert!(matches!(sk.links[0].shape, LinkShape::Ellipse { center: c, curve: k, .. } if c == center && k == curve));
        assert!(!sk.constraints.iter().any(|k| k.id == raio), "elipse não tem raio");
        sk.aplica_vinculo(&OpVinculo::Exata(l, Exata::Circulo { c: [1., 2.], r: 5. }));
        assert!(matches!(sk.curve(curve).unwrap().geom, Curve::Circle { .. }));
        assert_eq!(sk.points.len(), 1);
    }

    #[test]
    fn circulo_externo_nao_sobra_grau_de_liberdade() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let l = sk.add_link_exata(&mut al, FONTE, false, &Exata::Circulo { c: [1., 2.], r: 5. }, false);
        let rep = sk.solve(&Default::default(), &[]);
        assert_eq!(rep.dof, 0);
        assert_eq!(sk.radius_of(sk.curves[0].id), Some(5.));
        // solto, ele volta a ser um círculo comum
        sk.release_link(l);
        assert_eq!(sk.solve(&Default::default(), &[]).dof, 3);
    }

    #[test]
    fn raio_sobrevive_de_circulo_para_arco() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let l = sk.add_link_exata(&mut al, FONTE, false, &Exata::Circulo { c: [0., 0.], r: 5. }, false);
        let LinkShape::Circle { curve, .. } = sk.links[0].shape else { panic!() };
        sk.add_constraint(&mut al, Kind::Radius(curve, "5".into()));
        let ef = sk.aplica_vinculo(&OpVinculo::Exata(l, Exata::Arco { c: [0., 0.], s: [5., 0.], e: [0., 5.] }));
        assert_eq!(ef.perdidas, 0);
    }

    #[test]
    fn elipse_inteira_que_vira_trecho_ganha_pontas() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let inteira = Exata::Elipse { c: [0., 0.], maior: [5., 0.], menor: 3., t0: 0., t1: TAU };
        let l = sk.add_link_exata(&mut al, FONTE, false, &inteira, false);
        sk.aplica_vinculo(&OpVinculo::Exata(l, Exata::Elipse { c: [0., 0.], maior: [5., 0.], menor: 3., t0: 0., t1: 1. }));
        let LinkShape::Ellipse { center, s, e, .. } = sk.links[0].shape else { panic!() };
        assert!(s != center && e != center && s != e);
        assert_eq!(sk.pos(s), [5., 0.]);
    }

    #[test]
    fn ponta_presa_por_outra_curva_fica_livre_quando_o_vinculo_a_larga() {
        let mut al = ids();
        let (mut sk, l) = com_reta(&mut al);
        let (a, ..) = pontas(&sk);
        let q = sk.add_point(&mut al, 0., 9.);
        sk.add_curve(&mut al, Curve::Line { a, b: q }, false);
        sk.aplica_vinculo(&OpVinculo::Exata(l, Exata::Circulo { c: [5., 0.], r: 5. }));
        assert!(!sk.point(a).unwrap().fixed);
        assert_eq!(sk.curves.len(), 2);
    }

    /// Interseção com duas travessias.
    fn com_travessias(al: &mut impl FnMut() -> Id) -> (SketchData, Id, Vec<Id>) {
        let mut sk = SketchData::default();
        let l = sk.add_link_points(al, FONTE, &[[0., 0.], [10., 0.]], false).unwrap();
        let LinkShape::Poly { points, .. } = sk.links[0].shape.clone() else { panic!() };
        (sk, l, points)
    }

    fn pontos_dele(sk: &SketchData) -> Vec<Id> {
        let LinkShape::Poly { points, .. } = sk.links[0].shape.clone() else { panic!() };
        points
    }

    #[test]
    fn travessia_nova_nasce_e_cada_ponto_segue_a_sua() {
        let mut al = ids();
        let (mut sk, l, p) = com_travessias(&mut al);
        // a ordem vem trocada e há uma a mais no meio
        let op = OpVinculo::Pontos(l, vec![[10.5, 0.], [5., 1.], [0.5, 0.]]);
        let ef = sk.aplica_vinculo(&op);
        assert!(ef.refeito);
        assert_eq!((sk.pos(p[0]), sk.pos(p[1])), ([0.5, 0.], [10.5, 0.]));
        let agora = pontos_dele(&sk);
        assert_eq!(agora.len(), 3);
        assert!(sk.point(agora[2]).unwrap().fixed);
        assert_eq!(sk.pos(agora[2]), [5., 1.]);
        assert_eq!(sk.curves.iter().filter(|c| matches!(c.geom, Curve::Point { .. })).count(), 3);
        // o cálculo seguinte acha tudo em dia
        assert!(!sk.aplica_vinculo(&op).refeito);
    }

    #[test]
    fn travessia_que_some_leva_o_ponto_a_menos_que_algo_dependa_dele() {
        let mut al = ids();
        let (mut sk, l, p) = com_travessias(&mut al);
        let ef = sk.aplica_vinculo(&OpVinculo::Pontos(l, vec![[0., 0.]]));
        assert_eq!((ef.refeito, ef.sobras), (true, 0));
        assert_eq!(pontos_dele(&sk), vec![p[0]]);
        assert!(sk.point(p[1]).is_none());

        let (mut sk, l, p) = com_travessias(&mut al);
        let q = sk.add_point(&mut al, 10., 5.);
        sk.add_constraint(&mut al, Kind::Distance(p[1], q, "5".into()));
        let ef = sk.aplica_vinculo(&OpVinculo::Pontos(l, vec![[0., 0.]]));
        assert_eq!((ef.refeito, ef.sobras), (false, 1));
        assert_eq!(pontos_dele(&sk).len(), 2);
    }

    const QUADRO: [P; 5] = [[0., 0.], [40., 0.], [40., 10.], [0., 10.], [0., 0.]];

    #[test]
    fn contorno_com_os_mesmos_cantos_nao_escorrega() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let l = sk.add_link(&mut al, FONTE, true, &QUADRO, false).unwrap();
        let p = pontos_dele(&sk);
        assert_eq!(p.len(), 4);
        // a malha traz vértices no meio dos lados e começa em outro canto
        let uv = [[40., 12.], [20., 12.], [0., 12.], [0., 0.], [40., 0.], [40., 6.], [40., 12.]];
        assert!(!sk.aplica_vinculo(&OpVinculo::Polilinha(l, uv.to_vec())).refeito);
        assert_eq!(sk.pos(p[2]), [40., 12.]);
        assert_eq!(sk.pos(p[3]), [0., 12.]);
        assert_eq!(sk.pos(p[0]), [0., 0.]);
    }

    #[test]
    fn contorno_que_ganha_canto_ganha_ponto() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let l = sk.add_link(&mut al, FONTE, true, &QUADRO, false).unwrap();
        let uv = vec![[0., 0.], [40., 0.], [40., 10.], [20., 15.], [0., 10.], [0., 0.]];
        let op = OpVinculo::Polilinha(l, uv);
        assert!(sk.aplica_vinculo(&op).refeito);
        assert_eq!(pontos_dele(&sk).len(), 5);
        assert_eq!(sk.curves.len(), 5);
        assert!(!sk.aplica_vinculo(&op).refeito, "a próxima vez acha tudo em dia");
    }

    #[test]
    fn polilinha_com_restricao_nao_perde_os_trechos() {
        let mut al = ids();
        let mut sk = SketchData::default();
        let l = sk.add_link(&mut al, FONTE, true, &QUADRO, false).unwrap();
        let p = pontos_dele(&sk);
        let q = sk.add_point(&mut al, 90., 90.);
        sk.add_constraint(&mut al, Kind::Distance(p[1], q, "5".into()));
        assert!(!sk.link_aceita_forma(l));
        let uv = vec![[0., 0.], [40., 0.], [40., 10.], [20., 15.], [0., 10.], [0., 0.]];
        let ef = sk.aplica_vinculo(&OpVinculo::Polilinha(l, uv));
        assert_eq!((ef.refeito, pontos_dele(&sk).len()), (false, 4));
    }

    #[test]
    fn linha_sem_formula_vira_polilinha_e_guarda_as_pontas() {
        let mut al = ids();
        let (mut sk, l) = com_reta(&mut al);
        let (a, b, _) = pontas(&sk);
        let uv = vec![[0., 0.], [5., 3.], [10., 0.]];
        let ef = sk.aplica_vinculo(&OpVinculo::Polilinha(l, uv));
        assert!(ef.refeito);
        let p = pontos_dele(&sk);
        assert_eq!((p[0], p[2], p.len()), (a, b, 3));
        assert_eq!(sk.curves.len(), 2);
    }
}
