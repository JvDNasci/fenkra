// SPDX-License-Identifier: LGPL-2.1-or-later
//! B550: formas novas do sketch, cada uma com as restrições que a definem.
//!
//! - **Retângulo por 3 pontos** (inclinado): os dois primeiros cliques dão um lado em qualquer
//!   direção e o terceiro a altura, medida na perpendicular. Restrições: lados vizinhos
//!   perpendiculares e opostos paralelos. Sobram 5 graus de liberdade (posição, giro, largura e
//!   altura), os mesmos de um retângulo solto.
//! - **Paralelogramo**: três cantos seguidos; o quarto fecha a figura. Restrições: lados opostos
//!   paralelos. Sobram 6 graus de liberdade (posição, giro, dois lados e a abertura).
//! - **Círculo tangente a 3 elementos** (linhas, círculos ou arcos): a solução escolhida é a que fica
//!   do lado dos cliques. O círculo nasce com as três tangências e fica totalmente definido por elas.

use crate::sketch::{Curve, Kind, SketchData};
use crate::Id;

type P2 = [f64; 2];

fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn len(a: P2) -> f64 {
    a[0].hypot(a[1])
}

/// Cantos do retângulo inclinado: lado de `a` a `b` e altura até `c`, medida na perpendicular.
pub fn cantos_rect3(a: P2, b: P2, c: P2) -> Option<[P2; 4]> {
    let u = sub(b, a);
    let l = len(u);
    if l < 1e-9 {
        return None;
    }
    let n = [-u[1] / l, u[0] / l];
    let h = dot(sub(c, b), n);
    if h.abs() < 1e-9 {
        return None;
    }
    Some([a, b, [b[0] + n[0] * h, b[1] + n[1] * h], [a[0] + n[0] * h, a[1] + n[1] * h]])
}

/// Cantos do paralelogramo de cantos seguidos `a`, `b` e `c`; o quarto fecha a figura.
pub fn cantos_paralelogramo(a: P2, b: P2, c: P2) -> Option<[P2; 4]> {
    let (u, v) = (sub(b, a), sub(c, b));
    if cross(u, v).abs() < 1e-9 * len(u).max(len(v)).max(1.).powi(2) {
        return None;
    }
    Some([a, b, c, [a[0] + c[0] - b[0], a[1] + c[1] - b[1]]])
}

/// Elemento ao qual o círculo fica tangente.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Apoio {
    /// reta pelos dois pontos (a tangência vale na reta inteira, como a restrição do solver)
    Linha(P2, P2),
    /// centro e raio; o arco conta como o círculo inteiro
    Circulo(P2, f64),
}

impl Apoio {
    /// Ponto do elemento mais perto de `q`.
    fn pe(&self, q: P2) -> P2 {
        match *self {
            Apoio::Linha(a, b) => {
                let d = sub(b, a);
                let t = dot(sub(q, a), d) / dot(d, d).max(1e-18);
                [a[0] + d[0] * t, a[1] + d[1] * t]
            }
            Apoio::Circulo(c, r) => {
                let v = sub(q, c);
                let l = len(v).max(1e-12);
                [c[0] + v[0] * r / l, c[1] + v[1] * r / l]
            }
        }
    }
}

/// Círculo pelos três pontos.
pub fn circulo_3_pontos(a: P2, b: P2, c: P2) -> Option<(P2, f64)> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    let escala = len(sub(b, a)).max(len(sub(c, a))).max(1e-12);
    if d.abs() < 1e-9 * escala * escala {
        return None;
    }
    let q = |p: P2| p[0] * p[0] + p[1] * p[1];
    let ux = (q(a) * (b[1] - c[1]) + q(b) * (c[1] - a[1]) + q(c) * (a[1] - b[1])) / d;
    let uy = (q(a) * (c[0] - b[0]) + q(b) * (a[0] - c[0]) + q(c) * (b[0] - a[0])) / d;
    Some(([ux, uy], len(sub(a, [ux, uy]))))
}

/// Círculo tangente aos três elementos, do lado dos cliques `dicas` (um em cada elemento).
///
/// Há até oito soluções. A escolhida parte do círculo pelos pés dos cliques: cada reta exige o centro
/// do lado em que esse círculo começa, e cada círculo exige tangência por fora, ou por dentro se o
/// centro de partida já está dentro dele. Com o lado de cada um fixado as equações deixam de ter
/// módulo, e o método de Newton (amortecido) acha a raiz; com três retas o sistema é linear e a
/// resposta sai num passo.
pub fn circulo_tangente_3(e: &[Apoio; 3], dicas: &[P2; 3]) -> Option<(P2, f64)> {
    let pes = [0, 1, 2].map(|k| e[k].pe(dicas[k]));
    let (c0, r0) = circulo_3_pontos(pes[0], pes[1], pes[2])?;
    let escala = r0.max(1.);
    // sinal de cada equação, decidido no círculo de partida
    let lado: [f64; 3] = [0, 1, 2].map(|k| match e[k] {
        Apoio::Linha(a, b) => {
            if cross(sub(b, a), sub(c0, a)) >= 0. {
                1.
            } else {
                -1.
            }
        }
        Apoio::Circulo(c, r) => {
            if len(sub(c0, c)) < r {
                -1.
            } else {
                1.
            }
        }
    });
    let res = |x: [f64; 3]| -> [f64; 3] {
        let (c, r) = ([x[0], x[1]], x[2]);
        [0, 1, 2].map(|k| match e[k] {
            Apoio::Linha(a, b) => {
                let d = sub(b, a);
                lado[k] * cross(d, sub(c, a)) / len(d).max(1e-12) - r
            }
            // por fora: |c − ck| = rk + r; por dentro: |c − ck| = rk − r
            Apoio::Circulo(ck, rk) => {
                if lado[k] > 0. {
                    len(sub(c, ck)) - rk - r
                } else {
                    len(sub(c, ck)) - rk + r
                }
            }
        })
    };
    let norma = |v: [f64; 3]| v.iter().map(|a| a * a).sum::<f64>().sqrt();
    let mut x = [c0[0], c0[1], r0];
    let mut f = res(x);
    for _ in 0..80 {
        if norma(f) < 1e-11 * escala {
            break;
        }
        // Jacobiano numérico 3 × 3 e passo de Newton por Cramer
        let h = 1e-7 * escala;
        let mut j = [[0.; 3]; 3];
        for col in 0..3 {
            let mut y = x;
            y[col] += h;
            let fy = res(y);
            for lin in 0..3 {
                j[lin][col] = (fy[lin] - f[lin]) / h;
            }
        }
        let det3 = |m: [[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        let det = det3(j);
        if det.abs() < 1e-14 {
            return None;
        }
        let passo: [f64; 3] = [0, 1, 2].map(|col| {
            let mut m = j;
            for lin in 0..3 {
                m[lin][col] = -f[lin];
            }
            det3(m) / det
        });
        // amortece até o resíduo cair
        let mut t = 1.;
        loop {
            let y = [x[0] + t * passo[0], x[1] + t * passo[1], x[2] + t * passo[2]];
            let fy = res(y);
            if norma(fy) < norma(f) || t < 1e-4 {
                x = y;
                f = fy;
                break;
            }
            t *= 0.5;
        }
    }
    (norma(f) < 1e-8 * escala && x[2] > 1e-9).then_some(([x[0], x[1]], x[2]))
}

impl SketchData {
    /// Quatro linhas fechadas pelos cantos, sem restrição. Devolve (pontos, linhas).
    fn quadrilatero(&mut self, al: &mut impl FnMut() -> Id, c: [P2; 4], construction: bool) -> ([Id; 4], [Id; 4]) {
        let p = c.map(|q| self.add_point(al, q[0], q[1]));
        let l = [0, 1, 2, 3].map(|i| self.add_curve(al, Curve::Line { a: p[i], b: p[(i + 1) % 4] }, construction));
        (p, l)
    }

    /// Retângulo inclinado por 3 pontos (lado de `a` a `b`, altura até `c`): lados vizinhos
    /// perpendiculares e opostos paralelos. Devolve (pontos, linhas), na ordem a, b e os dois do
    /// lado oposto; `None` se os pontos não formam um retângulo.
    pub fn add_rect3(&mut self, al: &mut impl FnMut() -> Id, a: P2, b: P2, c: P2, construction: bool) -> Option<([Id; 4], [Id; 4])> {
        let cantos = cantos_rect3(a, b, c)?;
        let (p, l) = self.quadrilatero(al, cantos, construction);
        self.add_constraint(al, Kind::Perpendicular(l[0], l[1]));
        self.add_constraint(al, Kind::Parallel(l[0], l[2]));
        self.add_constraint(al, Kind::Parallel(l[1], l[3]));
        Some((p, l))
    }

    /// Paralelogramo pelos cantos seguidos `a`, `b` e `c`: lados opostos paralelos.
    pub fn add_paralelogramo(&mut self, al: &mut impl FnMut() -> Id, a: P2, b: P2, c: P2, construction: bool) -> Option<([Id; 4], [Id; 4])> {
        let cantos = cantos_paralelogramo(a, b, c)?;
        let (p, l) = self.quadrilatero(al, cantos, construction);
        self.add_constraint(al, Kind::Parallel(l[0], l[2]));
        self.add_constraint(al, Kind::Parallel(l[1], l[3]));
        Some((p, l))
    }

    /// O elemento como apoio de tangência: linha, círculo ou arco.
    pub fn apoio_de(&self, curva: Id) -> Option<Apoio> {
        match self.curve(curva)?.geom {
            Curve::Line { a, b } => Some(Apoio::Linha(self.pos(a), self.pos(b))),
            Curve::Circle { c, r } => Some(Apoio::Circulo(self.pos(c), r)),
            Curve::Arc { c, s, .. } => Some(Apoio::Circulo(self.pos(c), len(sub(self.pos(s), self.pos(c))))),
            _ => None,
        }
    }

    /// Linha, círculo ou arco mais perto de `q`, fora de `fora`: é o elemento que o clique escolheu.
    pub fn apoio_perto(&self, q: P2, fora: &[Id]) -> Option<Id> {
        let dist = |c: &crate::sketch::SkCurve| -> Option<f64> {
            match c.geom {
                Curve::Line { a, b } => {
                    let (pa, pb) = (self.pos(a), self.pos(b));
                    let d = sub(pb, pa);
                    let t = (dot(sub(q, pa), d) / dot(d, d).max(1e-18)).clamp(0., 1.);
                    Some(len(sub(q, [pa[0] + d[0] * t, pa[1] + d[1] * t])))
                }
                Curve::Circle { .. } | Curve::Arc { .. } => match self.apoio_de(c.id)? {
                    Apoio::Circulo(ctr, r) => Some((len(sub(q, ctr)) - r).abs()),
                    _ => None,
                },
                _ => None,
            }
        };
        self.curves.iter().filter(|c| !fora.contains(&c.id)).filter_map(|c| Some((c.id, dist(c)?))).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
    }

    /// Círculo tangente às três curvas, do lado dos cliques, com as três tangências. Devolve o
    /// círculo; `None` com curva repetida, curva que não serve de apoio ou sem solução.
    pub fn add_circulo_tangente(&mut self, al: &mut impl FnMut() -> Id, curvas: [Id; 3], dicas: [P2; 3], construction: bool) -> Option<Id> {
        if curvas[0] == curvas[1] || curvas[1] == curvas[2] || curvas[0] == curvas[2] {
            return None;
        }
        let e = [self.apoio_de(curvas[0])?, self.apoio_de(curvas[1])?, self.apoio_de(curvas[2])?];
        let (c, r) = circulo_tangente_3(&e, &dicas)?;
        let pc = self.add_point(al, c[0], c[1]);
        let circ = self.add_curve(al, Curve::Circle { c: pc, r }, construction);
        for k in curvas {
            self.add_constraint(al, Kind::Tangent(circ, k));
        }
        Some(circ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn alocador() -> impl FnMut() -> Id {
        let mut n = 0u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    fn perto(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn retangulo_3_pontos_inclinado_tem_5_graus_de_liberdade() {
        let mut sk = SketchData::default();
        let mut al = alocador();
        let (a, b) = ([0., 0.], [40. * 30f64.to_radians().cos(), 40. * 30f64.to_radians().sin()]);
        let (p, l) = sk.add_rect3(&mut al, a, b, [5., 30.], false).unwrap();
        assert_eq!(sk.constraints.len(), 3);
        let rep = sk.solve(&HashMap::new(), &[]);
        assert!(rep.converged);
        assert_eq!(rep.dof, 5, "posição, giro, largura e altura");
        // lado de 40 e altura pela perpendicular do 3º clique
        let pos = p.map(|q| sk.pos(q));
        assert!(perto(len(sub(pos[1], pos[0])), 40.));
        let h = len(sub(pos[2], pos[1]));
        assert!(perto(h, cross(sub(b, a), sub([5., 30.], a)) / 40.), "altura {h}");
        assert!(perto(dot(sub(pos[1], pos[0]), sub(pos[2], pos[1])), 0.));
        assert_eq!(l.len(), 4);
        // pontos alinhados não fazem retângulo
        assert!(sk.add_rect3(&mut al, a, b, [b[0] * 2., b[1] * 2.], false).is_none());
    }

    #[test]
    fn paralelogramo_fecha_no_quarto_canto_com_lados_paralelos() {
        let mut sk = SketchData::default();
        let mut al = alocador();
        let (p, _) = sk.add_paralelogramo(&mut al, [0., 0.], [40., 0.], [50., 20.], false).unwrap();
        assert_eq!(sk.pos(p[3]), [10., 20.]);
        let rep = sk.solve(&HashMap::new(), &[]);
        assert!(rep.converged);
        assert_eq!(rep.dof, 6);
        assert!(sk.add_paralelogramo(&mut al, [0., 0.], [40., 0.], [80., 0.], false).is_none());
    }

    #[test]
    fn tangente_a_tres_linhas_e_o_circulo_inscrito() {
        // triângulo 3-4-5: o inscrito tem raio 1 e centro em (1, 1)
        let e = [Apoio::Linha([0., 0.], [4., 0.]), Apoio::Linha([4., 0.], [0., 3.]), Apoio::Linha([0., 3.], [0., 0.])];
        let (c, r) = circulo_tangente_3(&e, &[[2., 0.], [2., 1.5], [0., 1.5]]).unwrap();
        assert!(perto(r, 1.) && perto(c[0], 1.) && perto(c[1], 1.), "{c:?} {r}");
        // cliques do lado de fora de um lado dão o exinscrito correspondente (raio = área / (s − a))
        let (_, r_ex) = circulo_tangente_3(&e, &[[6., 0.], [5., -0.75], [0., -2.]]).unwrap();
        assert!(r_ex > 1.5, "exinscrito {r_ex}");
    }

    #[test]
    fn tangente_a_dois_circulos_e_uma_linha() {
        // dois círculos de raio 10 em x = ±20 sobre o chão y = 0, tocando o chão
        let e = [Apoio::Circulo([-20., 10.], 10.), Apoio::Circulo([20., 10.], 10.), Apoio::Linha([-50., 0.], [50., 0.])];
        let (c, r) = circulo_tangente_3(&e, &[[-12., 6.], [12., 6.], [0., 0.]]).unwrap();
        // o pequeno entre os dois, no chão: (20)² + (r − 10)² = (r + 10)² → r = 10
        assert!(perto(c[0], 0.) && perto(r, 10.) && perto(c[1], 10.), "{c:?} {r}");
    }

    #[test]
    fn circulo_tangente_no_sketch_ganha_tres_tangencias_e_fica_definido() {
        let mut sk = SketchData::default();
        let mut al = alocador();
        let p = [[0., 0.], [40., 0.], [0., 30.]].map(|q| sk.add_point(&mut al, q[0], q[1]));
        let l = [0, 1, 2].map(|i| sk.add_curve(&mut al, Curve::Line { a: p[i], b: p[(i + 1) % 3] }, false));
        for q in p {
            sk.add_constraint(&mut al, Kind::Fixed(q));
        }
        let cliques = [[20., 0.], [20., 15.], [0., 15.]];
        let escolhidas = cliques.map(|q| sk.apoio_perto(q, &[]).unwrap());
        assert_eq!(escolhidas, l);
        let c = sk.add_circulo_tangente(&mut al, escolhidas, cliques, false).unwrap();
        let antes = sk.clone();
        let rep = sk.solve(&HashMap::new(), &[]);
        assert!(rep.converged);
        assert_eq!(rep.dof, 0, "três tangências definem o círculo");
        assert!(perto(sk.radius_of(c).unwrap(), 10.), "inscrito do 30-40-50");
        assert_eq!(sk.points, antes.points, "o solver não precisou mexer em nada");
        assert_eq!(sk.constraints.iter().filter(|k| matches!(k.kind, Kind::Tangent(..))).count(), 3);
        assert!(sk.add_circulo_tangente(&mut al, [l[0], l[0], l[1]], cliques, false).is_none());
    }
}
