// SPDX-License-Identifier: LGPL-2.1-or-later
//! Projeção e interseção exatas de curvas 3D no plano do sketch (B97).
//!
//! Reta e cônica são descritas pela fórmula, não por pontos amostrados. Uma cônica é
//! `P(t) = c + u cos t + v sin t`: círculo com `u` e `v` iguais e perpendiculares, elipse no caso
//! geral. Projetar no plano é uma transformação afim, então a imagem continua dessa forma. Basta
//! achar os eixos principais do par `u`, `v` projetado. O plano corta a cônica onde
//! `C + A cos t + B sin t = 0`, que tem solução direta, com todos os pontos.

use crate::sketch::{Curve, LinkShape, LinkSrc, SkLink, SketchData};
use crate::Id;
use std::f64::consts::{PI, TAU};

pub type V3 = [f64; 3];
type P = [f64; 2];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norma(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn escala(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn soma(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn dist2(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Plano do sketch: origem, eixos H e V e normal, todos unitários.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plano {
    pub origem: V3,
    pub h: V3,
    pub v: V3,
    pub n: V3,
}

impl Plano {
    pub fn uv(&self, p: V3) -> P {
        let d = sub(p, self.origem);
        [dot(d, self.h), dot(d, self.v)]
    }
    fn vetor(&self, d: V3) -> P {
        [dot(d, self.h), dot(d, self.v)]
    }
}

/// Curva 3D conhecida pela fórmula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curva3 {
    Reta { a: V3, b: V3 },
    /// `P(t) = c + u cos t + v sin t`, com `t` de `t0` a `t1`
    Conica { c: V3, u: V3, v: V3, t0: f64, t1: f64 },
    /// sem fórmula: fica a polilinha
    Outra,
}

/// Forma exata no plano do sketch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Exata {
    Ponto(P),
    Reta(P, P),
    Circulo { c: P, r: f64 },
    /// anti-horário de `s` até `e`
    Arco { c: P, s: P, e: P },
    /// `maior`: vetor do centro à ponta do semieixo maior
    Elipse { c: P, maior: P, menor: f64, t0: f64, t1: f64 },
}

impl Exata {
    pub fn nome(&self) -> &'static str {
        match self {
            Exata::Ponto(_) => "ponto",
            Exata::Reta(..) => "linha",
            Exata::Circulo { .. } => "círculo",
            Exata::Arco { .. } => "arco",
            Exata::Elipse { .. } => "elipse",
        }
    }
}

/// Ponto da elipse no parâmetro `t`, no sentido anti-horário a partir do semieixo maior.
pub fn elipse_em(c: P, maior: P, menor: f64, t: f64) -> P {
    let l = maior[0].hypot(maior[1]).max(1e-300);
    let n = [-maior[1] / l * menor, maior[0] / l * menor];
    [c[0] + maior[0] * t.cos() + n[0] * t.sin(), c[1] + maior[1] * t.cos() + n[1] * t.sin()]
}

/// Polilinha da elipse, para desenhar e medir.
pub fn elipse_amostras(c: P, maior: P, menor: f64, t0: f64, t1: f64) -> Vec<P> {
    let n = (((t1 - t0).abs() / TAU) * 96.).ceil().max(8.) as usize;
    (0..=n).map(|i| elipse_em(c, maior, menor, t0 + (t1 - t0) * i as f64 / n as f64)).collect()
}

/// A curva projetada no plano. `None` se não houver fórmula.
pub fn projeta(curva: &Curva3, pl: &Plano) -> Option<Exata> {
    match *curva {
        Curva3::Outra => None,
        Curva3::Reta { a, b } => {
            let (p, q) = (pl.uv(a), pl.uv(b));
            // reta na direção da normal: vira um ponto
            Some(if dist2(p, q) < 1e-9 { Exata::Ponto(p) } else { Exata::Reta(p, q) })
        }
        Curva3::Conica { c, u, v, t0, t1 } => {
            let (c2, u2, v2) = (pl.uv(c), pl.vetor(u), pl.vetor(v));
            let det = u2[0] * v2[1] - u2[1] * v2[0];
            let (lu, lv) = (norma(u), norma(v));
            if det.abs() < 1e-9 * (lu * lv).max(1e-300) {
                return Some(conica_de_perfil(c2, u2, v2, t0, t1));
            }
            // eixos principais: em t* a derivada é perpendicular ao raio
            let ts = 0.5 * (2. * (u2[0] * v2[0] + u2[1] * v2[1])).atan2(u2[0] * u2[0] + u2[1] * u2[1] - v2[0] * v2[0] - v2[1] * v2[1]);
            let (cs, sn) = (ts.cos(), ts.sin());
            let a = [u2[0] * cs + v2[0] * sn, u2[1] * cs + v2[1] * sn];
            let b = [-u2[0] * sn + v2[0] * cs, -u2[1] * sn + v2[1] * cs];
            let (la, lb) = (a[0].hypot(a[1]), b[0].hypot(b[1]));
            // o plano pode ver a curva pelo outro lado: o sentido vira, e o intervalo com ele
            let (q0, q1) = if det > 0. { (t0 - ts, t1 - ts) } else { (ts - t1, ts - t0) };
            let cheia = t1 - t0 >= TAU - 1e-9;
            if (la - lb).abs() <= 1e-9 * la {
                if cheia {
                    return Some(Exata::Circulo { c: c2, r: la });
                }
                return Some(Exata::Arco { c: c2, s: elipse_em(c2, a, la, q0), e: elipse_em(c2, a, la, q1) });
            }
            Some(Exata::Elipse { c: c2, maior: a, menor: lb, t0: q0, t1: q1 })
        }
    }
}

/// Cônica vista de perfil: o plano dela contém a normal do sketch, e a imagem é um segmento.
fn conica_de_perfil(c: P, u: P, v: P, t0: f64, t1: f64) -> Exata {
    let d = if u[0].hypot(u[1]) >= v[0].hypot(v[1]) { u } else { v };
    let l = d[0].hypot(d[1]).max(1e-300);
    let d = [d[0] / l, d[1] / l];
    let (a, b) = (u[0] * d[0] + u[1] * d[1], v[0] * d[0] + v[1] * d[1]);
    let s = |t: f64| a * t.cos() + b * t.sin();
    // extremos: as pontas do intervalo e os pontos em que a derivada zera dentro dele
    let t_ext = b.atan2(a);
    let (mut lo, mut hi) = (s(t0).min(s(t1)), s(t0).max(s(t1)));
    for k in -4..=4 {
        let t = t_ext + k as f64 * PI;
        if t >= t0 && t <= t1 {
            lo = lo.min(s(t));
            hi = hi.max(s(t));
        }
    }
    Exata::Reta([c[0] + d[0] * lo, c[1] + d[1] * lo], [c[0] + d[0] * hi, c[1] + d[1] * hi])
}

/// Onde a curva atravessa o plano: todos os pontos. `None` se não houver fórmula.
pub fn corta(curva: &Curva3, pl: &Plano) -> Option<Vec<V3>> {
    let alt = |p: V3| dot(sub(p, pl.origem), pl.n);
    match *curva {
        Curva3::Outra => None,
        Curva3::Reta { a, b } => {
            let (da, db) = (alt(a), alt(b));
            if (da - db).abs() < 1e-12 || da * db > 0. {
                return Some(vec![]);
            }
            Some(vec![soma(a, escala(sub(b, a), da / (da - db)))])
        }
        Curva3::Conica { c, u, v, t0, t1 } => {
            let (k, a, b) = (alt(c), dot(u, pl.n), dot(v, pl.n));
            let r = a.hypot(b);
            // plano paralelo ao da cônica, ou longe demais dela
            if r < 1e-12 || k.abs() > r * (1. + 1e-12) {
                return Some(vec![]);
            }
            let (fase, abre) = (b.atan2(a), (-k / r).clamp(-1., 1.).acos());
            let mut ts: Vec<f64> = vec![];
            for t in [fase - abre, fase + abre] {
                let t = t0 + (t - t0).rem_euclid(TAU);
                if t <= t1 + 1e-12 && !ts.iter().any(|x: &f64| ((x - t).rem_euclid(TAU)).min((t - x).rem_euclid(TAU)) < 1e-9) {
                    ts.push(t);
                }
            }
            ts.sort_by(f64::total_cmp);
            Some(ts.into_iter().map(|t| soma(c, soma(escala(u, t.cos()), escala(v, t.sin())))).collect())
        }
    }
}

/// Reconhece reta ou círculo numa polilinha cujos pontos estão exatamente sobre a curva, como a que
/// vem de outro sketch ou de uma referência. Aresta de corpo não passa por aqui: o kernel diz o tipo.
pub fn reconhece(pts: &[V3]) -> Curva3 {
    let n = pts.len();
    if n < 2 {
        return Curva3::Outra;
    }
    let (p0, pn) = (pts[0], pts[n - 1]);
    let tamanho = pts.iter().map(|p| norma(sub(*p, p0))).fold(0., f64::max);
    if tamanho < 1e-12 {
        return Curva3::Outra;
    }
    let tol = 1e-9 * (1. + tamanho);
    let d = sub(pn, p0);
    let ld = norma(d);
    if ld > tol && pts.iter().all(|p| norma(cross(sub(*p, p0), d)) / ld < tol) {
        return Curva3::Reta { a: p0, b: pn };
    }
    if n < 4 {
        return Curva3::Outra;
    }
    let (a, b, c) = (p0, pts[n / 3], pts[2 * n / 3]);
    let (ab, ac) = (sub(b, a), sub(c, a));
    let nn = cross(ab, ac);
    let n2 = dot(nn, nn);
    if n2 < 1e-24 {
        return Curva3::Outra;
    }
    // centro do círculo pelos três pontos
    let k = soma(escala(cross(nn, ab), dot(ac, ac)), escala(cross(ac, nn), dot(ab, ab)));
    let ctr = soma(a, escala(k, 0.5 / n2));
    let r = norma(sub(a, ctr));
    let nu = escala(nn, 1. / n2.sqrt());
    if !pts.iter().all(|p| (norma(sub(*p, ctr)) - r).abs() < tol * 10. && dot(sub(*p, ctr), nu).abs() < tol * 10.) {
        return Curva3::Outra;
    }
    let u = sub(p0, ctr);
    // o sentido é o do percurso: `v` aponta para onde a polilinha vai
    let giro = cross(u, sub(pts[1], ctr));
    let nu = if dot(giro, nu) >= 0. { nu } else { escala(nu, -1.) };
    let v = cross(nu, u);
    let t1 = if norma(sub(pn, p0)) < tol * 10. { TAU } else { dot(sub(pn, ctr), v).atan2(dot(sub(pn, ctr), u)).rem_euclid(TAU) };
    Curva3::Conica { c: ctr, u, v, t0: 0., t1 }
}

impl SketchData {
    fn ponto_fixo(&mut self, al: &mut impl FnMut() -> Id, q: P) -> Id {
        let id = self.add_point(al, q[0], q[1]);
        if let Some(p) = self.point_mut(id) {
            p.fixed = true;
        }
        id
    }

    /// Põe a geometria externa no sketch com a forma exata, ligada à origem.
    pub fn add_link_exata(&mut self, al: &mut impl FnMut() -> Id, source: LinkSrc, intersect: bool, forma: &Exata, construction: bool) -> Id {
        let shape = match *forma {
            Exata::Ponto(q) => {
                let p = self.ponto_fixo(al, q);
                self.add_curve(al, Curve::Point { p }, construction);
                LinkShape::Poly { points: vec![p], closed: false, isolated: false }
            }
            Exata::Reta(a, b) => {
                let (a, b) = (self.ponto_fixo(al, a), self.ponto_fixo(al, b));
                let curve = self.add_curve(al, Curve::Line { a, b }, construction);
                LinkShape::Line { a, b, curve }
            }
            Exata::Circulo { c, r } => {
                let center = self.ponto_fixo(al, c);
                let curve = self.add_curve(al, Curve::Circle { c: center, r }, construction);
                LinkShape::Circle { center, curve }
            }
            Exata::Arco { c, s, e } => {
                let (center, s, e) = (self.ponto_fixo(al, c), self.ponto_fixo(al, s), self.ponto_fixo(al, e));
                let curve = self.add_curve(al, Curve::Arc { c: center, s, e }, construction);
                LinkShape::Arc { center, s, e, curve }
            }
            Exata::Elipse { c, maior, menor, t0, t1 } => {
                let center = self.ponto_fixo(al, c);
                // inteira: as pontas não existem e o centro faz o papel delas
                let (s, e) = if t1 - t0 >= TAU - 1e-9 {
                    (center, center)
                } else {
                    (self.ponto_fixo(al, elipse_em(c, maior, menor, t0)), self.ponto_fixo(al, elipse_em(c, maior, menor, t1)))
                };
                let curve = self.add_curve(al, Curve::Ellipse { c: center, s, e, major: maior, minor: menor, t0, t1 }, construction);
                LinkShape::Ellipse { center, s, e, curve }
            }
        };
        let id = al();
        self.links.push(SkLink { id, source, intersect, shape, policy: self.link_policy });
        id
    }

    /// Reposiciona a geometria externa com a forma nova. Devolve `false` se a origem virou outro tipo
    /// de curva, porque os pontos guardados não servem para a forma nova.
    pub fn update_link_exata(&mut self, link: Id, forma: &Exata) -> bool {
        let Some(l) = self.links.iter().find(|l| l.id == link).cloned() else { return false };
        let mut poe = |id: Id, q: P| {
            if let Some(p) = self.point_mut(id) {
                (p.x, p.y) = (q[0], q[1]);
            }
        };
        match (&l.shape, *forma) {
            (LinkShape::Poly { points, isolated: false, .. }, Exata::Ponto(q)) if points.len() == 1 => poe(points[0], q),
            (LinkShape::Line { a, b, .. }, Exata::Reta(p, q)) => {
                poe(*a, p);
                poe(*b, q);
            }
            (LinkShape::Circle { center, curve }, Exata::Circulo { c, r }) => {
                poe(*center, c);
                if let Some(Curve::Circle { r: rr, .. }) = self.curves.iter_mut().find(|x| x.id == *curve).map(|x| &mut x.geom) {
                    *rr = r;
                }
            }
            (LinkShape::Arc { center, s, e, .. }, Exata::Arco { c, s: ps, e: pe }) => {
                poe(*center, c);
                poe(*s, ps);
                poe(*e, pe);
            }
            (LinkShape::Ellipse { center, s, e, curve }, Exata::Elipse { c, maior, menor, t0, t1 }) => {
                poe(*center, c);
                if s != center {
                    poe(*s, elipse_em(c, maior, menor, t0));
                    poe(*e, elipse_em(c, maior, menor, t1));
                }
                if let Some(Curve::Ellipse { major, minor, t0: a, t1: b, .. }) = self.curves.iter_mut().find(|x| x.id == *curve).map(|x| &mut x.geom) {
                    (*major, *minor, *a, *b) = (maior, menor, t0, t1);
                }
            }
            _ => return false,
        }
        true
    }

    /// Solta a geometria externa: ela fica como geometria comum, com os pontos livres.
    pub fn release_link(&mut self, link: Id) {
        let Some(i) = self.links.iter().position(|l| l.id == link) else { return };
        let l = self.links.remove(i);
        let ids: Vec<Id> = match l.shape {
            LinkShape::Poly { points, .. } => points,
            LinkShape::Line { a, b, .. } => vec![a, b],
            LinkShape::Circle { center, .. } => vec![center],
            LinkShape::Arc { center, s, e, .. } | LinkShape::Ellipse { center, s, e, .. } => vec![center, s, e],
        };
        for id in ids {
            if let Some(p) = self.point_mut(id) {
                p.fixed = false;
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const XY: Plano = Plano { origem: [0., 0., 0.], h: [1., 0., 0.], v: [0., 1., 0.], n: [0., 0., 1.] };

    fn ids() -> impl FnMut() -> Id {
        let mut n = 100u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    fn circulo(c: V3, u: V3, v: V3) -> Curva3 {
        Curva3::Conica { c, u, v, t0: 0., t1: TAU }
    }

    fn perto(a: P, b: P) -> bool {
        dist2(a, b) < 1e-9
    }

    #[test]
    fn circulo_paralelo_ao_plano_fica_circulo() {
        let e = projeta(&circulo([3., 4., 9.], [5., 0., 0.], [0., 5., 0.]), &XY).unwrap();
        assert_eq!(e, Exata::Circulo { c: [3., 4.], r: 5. });
    }

    #[test]
    fn circulo_inclinado_vira_elipse_com_os_eixos_certos() {
        // círculo de raio 10 inclinado 60° em torno de x: a altura encolhe por cos 60° = 0,5
        let (s, c) = (60f64.to_radians().sin(), 60f64.to_radians().cos());
        let e = projeta(&circulo([0., 0., 0.], [10., 0., 0.], [0., 10. * c, 10. * s]), &XY).unwrap();
        let Exata::Elipse { c: ctr, maior, menor, t0, t1 } = e else { panic!("{e:?}") };
        assert!(perto(ctr, [0., 0.]));
        assert!((maior[0].hypot(maior[1]) - 10.).abs() < 1e-9);
        assert!((menor - 5.).abs() < 1e-9);
        assert!(maior[1].abs() < 1e-9, "eixo maior em x: {maior:?}");
        assert!((t1 - t0 - TAU).abs() < 1e-12);
    }

    #[test]
    fn arco_da_elipse_passa_pelos_mesmos_pontos() {
        // a fórmula projetada tem de cair onde a curva 3D cai, nos dois sentidos de vista
        let (u, v) = ([6., 2., 1.], [-1., 3., 4.]);
        let (t0, t1) = (0.4, 2.2);
        let curva = Curva3::Conica { c: [1., 2., 3.], u, v, t0, t1 };
        for pl in [XY, Plano { origem: [0., 0., 0.], h: [0., 1., 0.], v: [1., 0., 0.], n: [0., 0., -1.] }] {
            let Some(Exata::Elipse { c, maior, menor, t0: a, t1: b }) = projeta(&curva, &pl) else { panic!() };
            assert!(b > a);
            let ponto = |t: f64| pl.uv(soma([1., 2., 3.], soma(escala(u, t.cos()), escala(v, t.sin()))));
            let (s, e) = (elipse_em(c, maior, menor, a), elipse_em(c, maior, menor, b));
            // a ordem das pontas depende do lado de que se olha, a curva não
            assert!((perto(s, ponto(t0)) && perto(e, ponto(t1))) || (perto(s, ponto(t1)) && perto(e, ponto(t0))));
            let m = elipse_em(c, maior, menor, (a + b) / 2.);
            let amostra: Vec<P> = (0..=2000).map(|i| ponto(t0 + (t1 - t0) * i as f64 / 2000.)).collect();
            assert!(amostra.iter().any(|q| dist2(*q, m) < 0.02), "meio fora da curva: {m:?}");
        }
    }

    #[test]
    fn arco_de_circulo_paralelo_e_anti_horario() {
        let c = Curva3::Conica { c: [0., 0., 2.], u: [10., 0., 0.], v: [0., 10., 0.], t0: 0., t1: PI / 2. };
        let Some(Exata::Arco { c: k, s, e }) = projeta(&c, &XY) else { panic!() };
        assert!(perto(k, [0., 0.]) && perto(s, [10., 0.]) && perto(e, [0., 10.]));
        // visto por trás, o sentido se inverte: as pontas trocam para seguir anti-horário
        let tras = Plano { origem: [0., 0., 0.], h: [0., 1., 0.], v: [1., 0., 0.], n: [0., 0., -1.] };
        let Some(Exata::Arco { s, e, .. }) = projeta(&c, &tras) else { panic!() };
        assert!(perto(s, [10., 0.]) && perto(e, [0., 10.]), "{s:?} {e:?}");
    }

    #[test]
    fn circulo_de_perfil_vira_segmento() {
        let e = projeta(&circulo([0., 0., 0.], [10., 0., 0.], [0., 0., 10.]), &XY).unwrap();
        assert_eq!(e, Exata::Reta([-10., 0.], [10., 0.]));
        // meia volta de 0 a 90°: só de 0 a 10
        let q = Curva3::Conica { c: [0., 0., 0.], u: [10., 0., 0.], v: [0., 0., 10.], t0: 0., t1: PI / 2. };
        let Some(Exata::Reta(a, b)) = projeta(&q, &XY) else { panic!() };
        assert!(perto(a, [0., 0.]) && perto(b, [10., 0.]));
    }

    #[test]
    fn reta_na_direcao_da_normal_vira_ponto() {
        assert_eq!(projeta(&Curva3::Reta { a: [1., 2., 0.], b: [1., 2., 9.] }, &XY), Some(Exata::Ponto([1., 2.])));
    }

    #[test]
    fn plano_corta_o_circulo_nos_dois_pontos_exatos() {
        // círculo de raio 10 no plano XZ, centro na origem: y = 0... corta z = 6 em x = ±8
        let pl = Plano { origem: [0., 0., 6.], ..XY };
        let v = corta(&circulo([0., 0., 0.], [10., 0., 0.], [0., 0., 10.]), &pl).unwrap();
        assert_eq!(v.len(), 2);
        for p in &v {
            assert!((p[0].abs() - 8.).abs() < 1e-12 && (p[2] - 6.).abs() < 1e-12, "{p:?}");
        }
        assert!(v[0][0] * v[1][0] < 0.);
    }

    #[test]
    fn corte_respeita_o_trecho_e_a_tangencia() {
        let arco = Curva3::Conica { c: [0., 0., 0.], u: [10., 0., 0.], v: [0., 0., 10.], t0: 0., t1: PI / 2. };
        assert_eq!(corta(&arco, &Plano { origem: [0., 0., 6.], ..XY }).unwrap().len(), 1);
        // tangente em z = 10: um ponto só; acima dele, nenhum
        let c = circulo([0., 0., 0.], [10., 0., 0.], [0., 0., 10.]);
        assert_eq!(corta(&c, &Plano { origem: [0., 0., 10.], ..XY }).unwrap().len(), 1);
        assert!(corta(&c, &Plano { origem: [0., 0., 11.], ..XY }).unwrap().is_empty());
        // plano paralelo ao do círculo
        assert!(corta(&circulo([0., 0., 1.], [1., 0., 0.], [0., 1., 0.]), &XY).unwrap().is_empty());
    }

    #[test]
    fn corte_de_reta() {
        let v = corta(&Curva3::Reta { a: [0., 0., -1.], b: [4., 0., 3.] }, &XY).unwrap();
        assert_eq!(v, vec![[1., 0., 0.]]);
        assert!(corta(&Curva3::Reta { a: [0., 0., 1.], b: [4., 0., 3.] }, &XY).unwrap().is_empty());
    }

    #[test]
    fn polilinha_exata_e_reconhecida() {
        assert_eq!(reconhece(&[[0., 0., 0.], [1., 1., 1.], [2., 2., 2.]]), Curva3::Reta { a: [0., 0., 0.], b: [2., 2., 2.] });
        let arco: Vec<V3> = (0..=20).map(|i| {
            let t = 0.3 + 1.1 * i as f64 / 20.;
            [5. + 7. * t.cos(), 7. * t.sin(), 2.]
        })
        .collect();
        let Curva3::Conica { c, u, v, t0, t1 } = reconhece(&arco) else { panic!() };
        assert!(norma(sub(c, [5., 0., 2.])) < 1e-9);
        assert!((norma(u) - 7.).abs() < 1e-9 && (norma(v) - 7.).abs() < 1e-9);
        assert!((t1 - t0 - 1.1).abs() < 1e-9);
        // o fim da fórmula cai no fim da polilinha
        let fim = soma(c, soma(escala(u, t1.cos()), escala(v, t1.sin())));
        assert!(norma(sub(fim, arco[20])) < 1e-9);
        let volta: Vec<V3> = (0..=48).map(|i| {
            let t = TAU * i as f64 / 48.;
            [3. * t.cos(), 3. * t.sin(), 0.]
        })
        .collect();
        assert!(matches!(reconhece(&volta), Curva3::Conica { t1, .. } if (t1 - TAU).abs() < 1e-12));
        // curva qualquer não finge ser círculo
        let torta: Vec<V3> = (0..20).map(|i| [i as f64, (i as f64 * 0.7).sin() * 3., 0.]).collect();
        assert_eq!(reconhece(&torta), Curva3::Outra);
    }

    #[test]
    fn vinculo_exato_acompanha_e_recusa_outro_tipo() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let f = Exata::Elipse { c: [0., 0.], maior: [10., 0.], menor: 5., t0: 0., t1: TAU };
        let l = sk.add_link_exata(&mut al, LinkSrc::Edge { mid: [0.; 3] }, false, &f, false);
        let n = Exata::Elipse { c: [1., 1.], maior: [0., 20.], menor: 3., t0: 0., t1: TAU };
        assert!(sk.update_link_exata(l, &n));
        let Curve::Ellipse { c, major, minor, .. } = sk.curves[0].geom else { panic!() };
        assert_eq!((sk.pos(c), major, minor), ([1., 1.], [0., 20.], 3.));
        assert!(!sk.update_link_exata(l, &Exata::Circulo { c: [0., 0.], r: 1. }));
    }

    #[test]
    fn arco_externo_fecha_perfil_com_uma_linha() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let e = Exata::Elipse { c: [0., 0.], maior: [10., 0.], menor: 5., t0: 0., t1: PI };
        sk.add_link_exata(&mut al, LinkSrc::Edge { mid: [0.; 3] }, false, &e, false);
        // as pontas da meia elipse são pontos do sketch: uma linha entre elas fecha o contorno
        let [_, s, f] = [sk.points[0].id, sk.points[1].id, sk.points[2].id];
        sk.add_curve(&mut al, Curve::Line { a: f, b: s }, false);
        let (loops, abertos) = sk.chains();
        assert_eq!((loops.len(), abertos.len()), (1, 0));
    }

    #[test]
    fn soltar_deixa_a_geometria_livre() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = sk.add_link_exata(&mut al, LinkSrc::Edge { mid: [0.; 3] }, false, &Exata::Reta([0., 0.], [9., 0.]), false);
        assert!(sk.points.iter().all(|p| p.fixed));
        sk.release_link(l);
        assert!(sk.links.is_empty() && sk.points.iter().all(|p| !p.fixed));
        assert_eq!(sk.curves.len(), 1);
    }
}
