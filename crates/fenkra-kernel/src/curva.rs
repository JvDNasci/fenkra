// SPDX-License-Identifier: LGPL-2.1-or-later
//! B559: curva exata que vai ao kernel.
//!
//! Uma curva é uma lista de trechos, cada um com a geometria de verdade (reta, arco, círculo, elipse,
//! B-spline, spline pelos pontos). O kernel faz uma aresta exata por trecho e junta num fio: cantos
//! entre trechos ficam G0, sem virar "tudo reto". A polilinha amostrada (`Pontos`) só existe para
//! curvas que ainda não têm forma exata; o kernel a refaz em arestas suaves entre os cantos.
//!
//! Codificação para a ponte (`codifica`): `[NaN, n, (tipo, tamanho, dados...)×n]`. O `NaN` inicial
//! separa da lista crua de pontos que as funções antigas recebiam.

use crate::Xform;

#[derive(Clone, Debug, PartialEq)]
pub enum Trecho3 {
    /// reta de `a` a `b`
    Reta([f64; 3], [f64; 3]),
    /// arco do primeiro ao terceiro ponto, passando pelo segundo
    Arco([f64; 3], [f64; 3], [f64; 3]),
    /// círculo inteiro; começa em `centro + x·raio` e gira pela normal (regra da mão direita)
    Circulo { centro: [f64; 3], normal: [f64; 3], x: [f64; 3], raio: f64 },
    /// elipse (ou arco dela) no plano de `normal`: eixo maior `maior` (vetor com o comprimento),
    /// semieixo menor `menor`, parâmetros `t0` a `t1` em radianos
    Elipse { centro: [f64; 3], normal: [f64; 3], maior: [f64; 3], menor: f64, t0: f64, t1: f64 },
    /// B-spline com nós achatados; `pesos` vazio = não racional
    BSpline { grau: u32, polos: Vec<[f64; 3]>, pesos: Vec<f64>, nos: Vec<f64> },
    /// spline que passa pelos pontos, com a interpolação do kernel (a definição única, B559)
    Interpolada { pts: Vec<[f64; 3]>, fechada: bool },
    /// polilinha amostrada, sem forma exata conhecida
    Pontos(Vec<[f64; 3]>),
}

/// Curva exata: trechos em sequência, cada um começando onde o anterior termina.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curva3(pub Vec<Trecho3>);

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let n = norm(a);
    if n > 1e-300 { mul(a, 1. / n) } else { a }
}

/// Aplica a transformação a um ponto.
pub fn xform_ponto(x: &Xform, p: [f64; 3]) -> [f64; 3] {
    match *x {
        Xform::Translate(d) => add(p, d),
        Xform::Rotate(o, d, graus) => {
            let k = unit(d);
            let v = sub(p, o);
            let (c, s) = (graus.to_radians().cos(), graus.to_radians().sin());
            let kv = crate::cross(k, v);
            let kd = dot(k, v);
            add(o, [0, 1, 2].map(|i| v[i] * c + kv[i] * s + k[i] * kd * (1. - c)))
        }
        Xform::Mirror(o, n) => {
            let n = unit(n);
            sub(p, mul(n, 2. * dot(sub(p, o), n)))
        }
        Xform::Scale(o, k) => add(o, mul(sub(p, o), k)),
    }
}

impl Trecho3 {
    pub fn inicio(&self) -> [f64; 3] {
        match self {
            Trecho3::Reta(a, _) | Trecho3::Arco(a, _, _) => *a,
            Trecho3::Circulo { centro, x, raio, .. } => add(*centro, mul(unit(*x), *raio)),
            Trecho3::Elipse { centro, normal, maior, menor, t0, .. } => elipse_ponto(*centro, *normal, *maior, *menor, *t0),
            Trecho3::BSpline { polos, .. } => polos[0],
            Trecho3::Interpolada { pts, .. } | Trecho3::Pontos(pts) => pts[0],
        }
    }
    pub fn fim(&self) -> [f64; 3] {
        match self {
            Trecho3::Reta(_, b) | Trecho3::Arco(_, _, b) => *b,
            Trecho3::Circulo { .. } => self.inicio(),
            Trecho3::Elipse { centro, normal, maior, menor, t1, .. } => elipse_ponto(*centro, *normal, *maior, *menor, *t1),
            Trecho3::BSpline { polos, .. } => *polos.last().unwrap(),
            Trecho3::Interpolada { pts, fechada } => if *fechada { pts[0] } else { *pts.last().unwrap() },
            Trecho3::Pontos(pts) => *pts.last().unwrap(),
        }
    }
    /// O mesmo trecho percorrido ao contrário.
    pub fn invertido(&self) -> Trecho3 {
        match self {
            Trecho3::Reta(a, b) => Trecho3::Reta(*b, *a),
            Trecho3::Arco(a, m, b) => Trecho3::Arco(*b, *m, *a),
            Trecho3::Circulo { centro, normal, x, raio } => Trecho3::Circulo { centro: *centro, normal: mul(*normal, -1.), x: *x, raio: *raio },
            Trecho3::Elipse { centro, normal, maior, menor, t0, t1 } => {
                // inverter a normal espelha o parâmetro: t vira −t
                Trecho3::Elipse { centro: *centro, normal: mul(*normal, -1.), maior: *maior, menor: *menor, t0: -*t1, t1: -*t0 }
            }
            Trecho3::BSpline { grau, polos, pesos, nos } => {
                let (a, b) = (nos[0], *nos.last().unwrap());
                Trecho3::BSpline {
                    grau: *grau,
                    polos: polos.iter().rev().copied().collect(),
                    pesos: pesos.iter().rev().copied().collect(),
                    nos: nos.iter().rev().map(|t| a + b - t).collect(),
                }
            }
            Trecho3::Interpolada { pts, fechada } => {
                let mut v = pts.clone();
                if *fechada {
                    // mesma curva fechada ao contrário, começando no mesmo ponto
                    v[1..].reverse();
                } else {
                    v.reverse();
                }
                Trecho3::Interpolada { pts: v, fechada: *fechada }
            }
            Trecho3::Pontos(p) => Trecho3::Pontos(p.iter().rev().copied().collect()),
        }
    }
    /// O trecho transformado (rígida, espelho ou escala uniforme: todas preservam o tipo).
    pub fn transformado(&self, x: &Xform) -> Trecho3 {
        let f = |p: [f64; 3]| xform_ponto(x, p);
        match self {
            Trecho3::Reta(a, b) => Trecho3::Reta(f(*a), f(*b)),
            Trecho3::Arco(a, m, b) => Trecho3::Arco(f(*a), f(*m), f(*b)),
            Trecho3::Circulo { centro, normal, x: xd, raio } => {
                let xd = unit(*xd);
                let yd = crate::cross(unit(*normal), xd);
                let (c, px, py) = (f(*centro), f(add(*centro, mul(xd, *raio))), f(add(*centro, mul(yd, *raio))));
                let (vx, vy) = (sub(px, c), sub(py, c));
                Trecho3::Circulo { centro: c, normal: unit(crate::cross(vx, vy)), x: unit(vx), raio: norm(vx) }
            }
            Trecho3::Elipse { centro, normal, maior, menor, t0, t1 } => {
                let yd = mul(crate::cross(unit(*normal), unit(*maior)), *menor);
                let (c, pa, pb) = (f(*centro), f(add(*centro, *maior)), f(add(*centro, yd)));
                let (va, vb) = (sub(pa, c), sub(pb, c));
                Trecho3::Elipse { centro: c, normal: unit(crate::cross(va, vb)), maior: va, menor: norm(vb), t0: *t0, t1: *t1 }
            }
            Trecho3::BSpline { grau, polos, pesos, nos } => Trecho3::BSpline { grau: *grau, polos: polos.iter().map(|p| f(*p)).collect(), pesos: pesos.clone(), nos: nos.clone() },
            Trecho3::Interpolada { pts, fechada } => Trecho3::Interpolada { pts: pts.iter().map(|p| f(*p)).collect(), fechada: *fechada },
            Trecho3::Pontos(p) => Trecho3::Pontos(p.iter().map(|q| f(*q)).collect()),
        }
    }
    /// Trecho com geometria exata (tudo menos a polilinha amostrada).
    pub fn exato(&self) -> bool {
        !matches!(self, Trecho3::Pontos(_))
    }
    fn codifica(&self, out: &mut Vec<f64>) {
        let (tipo, dados): (f64, Vec<f64>) = match self {
            Trecho3::Reta(a, b) => (0., [*a, *b].concat()),
            Trecho3::Arco(a, m, b) => (1., [*a, *m, *b].concat()),
            Trecho3::Circulo { centro, normal, x, raio } => (2., [centro.as_slice(), normal, x, &[*raio]].concat()),
            Trecho3::Elipse { centro, normal, maior, menor, t0, t1 } => (3., [centro.as_slice(), normal, maior, &[*menor, *t0, *t1]].concat()),
            Trecho3::BSpline { grau, polos, pesos, nos } => {
                let mut v = vec![*grau as f64, polos.len() as f64, nos.len() as f64, pesos.len() as f64];
                v.extend(polos.iter().flatten());
                v.extend(nos);
                v.extend(pesos);
                (4., v)
            }
            Trecho3::Interpolada { pts, fechada } => {
                let mut v = vec![if *fechada { 1. } else { 0. }];
                v.extend(pts.iter().flatten());
                (5., v)
            }
            Trecho3::Pontos(p) => (6., p.iter().flatten().copied().collect()),
        };
        out.push(tipo);
        out.push(dados.len() as f64);
        out.extend(dados);
    }
}

/// Ponto da elipse no parâmetro `t`.
pub fn elipse_ponto(centro: [f64; 3], normal: [f64; 3], maior: [f64; 3], menor: f64, t: f64) -> [f64; 3] {
    let y = mul(crate::cross(unit(normal), unit(maior)), menor);
    add(centro, add(mul(maior, t.cos()), mul(y, t.sin())))
}

impl Curva3 {
    /// Polilinha crua (curva sem forma exata).
    pub fn pontos(p: &[[f64; 3]]) -> Curva3 {
        Curva3(vec![Trecho3::Pontos(p.to_vec())])
    }
    /// Polilinha de retas de verdade (cantos vivos em cada vértice).
    pub fn retas(p: &[[f64; 3]]) -> Curva3 {
        Curva3(p.windows(2).filter(|w| norm(sub(w[1], w[0])) > 1e-9).map(|w| Trecho3::Reta(w[0], w[1])).collect())
    }
    pub fn vazia(&self) -> bool {
        self.0.is_empty()
    }
    /// Todos os trechos têm geometria exata.
    pub fn exata(&self) -> bool {
        !self.0.is_empty() && self.0.iter().all(Trecho3::exato)
    }
    pub fn inicio(&self) -> Option<[f64; 3]> {
        self.0.first().map(Trecho3::inicio)
    }
    pub fn fim(&self) -> Option<[f64; 3]> {
        self.0.last().map(Trecho3::fim)
    }
    pub fn invertida(&self) -> Curva3 {
        Curva3(self.0.iter().rev().map(Trecho3::invertido).collect())
    }
    pub fn transformada(&self, x: &Xform) -> Curva3 {
        Curva3(self.0.iter().map(|t| t.transformado(x)).collect())
    }
    /// Acrescenta os trechos de `outra` no fim.
    pub fn junta(&mut self, outra: Curva3) {
        self.0.extend(outra.0);
    }
    /// Codificação da ponte (ver o módulo).
    pub fn codifica(&self) -> Vec<f64> {
        let mut out = vec![f64::NAN, self.0.len() as f64];
        for t in &self.0 {
            t.codifica(&mut out);
        }
        out
    }
    /// Lê a codificação da ponte (o que o kernel devolve de uma aresta).
    pub fn decodifica(v: &[f64]) -> Option<Curva3> {
        if v.len() < 2 || !v[0].is_nan() {
            return None;
        }
        let n = v[1] as usize;
        let mut i = 2;
        let mut out = vec![];
        let p3 = |d: &[f64], k: usize| -> [f64; 3] { [d[k], d[k + 1], d[k + 2]] };
        for _ in 0..n {
            let tipo = *v.get(i)? as u32;
            let tam = *v.get(i + 1)? as usize;
            let d = v.get(i + 2..i + 2 + tam)?;
            i += 2 + tam;
            out.push(match tipo {
                0 if tam == 6 => Trecho3::Reta(p3(d, 0), p3(d, 3)),
                1 if tam == 9 => Trecho3::Arco(p3(d, 0), p3(d, 3), p3(d, 6)),
                2 if tam == 10 => Trecho3::Circulo { centro: p3(d, 0), normal: p3(d, 3), x: p3(d, 6), raio: d[9] },
                3 if tam == 12 => Trecho3::Elipse { centro: p3(d, 0), normal: p3(d, 3), maior: p3(d, 6), menor: d[9], t0: d[10], t1: d[11] },
                4 if tam >= 4 => {
                    let (grau, np, nk, nw) = (d[0] as u32, d[1] as usize, d[2] as usize, d[3] as usize);
                    if tam != 4 + 3 * np + nk + nw {
                        return None;
                    }
                    let polos = (0..np).map(|k| p3(d, 4 + 3 * k)).collect();
                    let nos = d[4 + 3 * np..4 + 3 * np + nk].to_vec();
                    let pesos = d[4 + 3 * np + nk..].to_vec();
                    Trecho3::BSpline { grau, polos, pesos, nos }
                }
                5 if tam >= 1 => Trecho3::Interpolada { fechada: d[0] > 0.5, pts: d[1..].chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect() },
                6 => Trecho3::Pontos(d.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()),
                _ => return None,
            });
        }
        Some(Curva3(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverter_duas_vezes_volta_ao_mesmo() {
        let c = Curva3(vec![
            Trecho3::Reta([0., 0., 0.], [10., 0., 0.]),
            Trecho3::Arco([10., 0., 0.], [15., 5., 0.], [10., 10., 0.]),
            Trecho3::BSpline { grau: 3, polos: vec![[10., 10., 0.], [5., 12., 0.], [2., 9., 0.], [0., 10., 0.]], pesos: vec![], nos: vec![0., 0., 0., 0., 1., 1., 1., 1.] },
        ]);
        assert_eq!(c.invertida().invertida(), c);
        assert_eq!(c.invertida().inicio(), c.fim());
    }

    #[test]
    fn espelho_mantem_o_circulo() {
        let t = Trecho3::Circulo { centro: [5., 0., 0.], normal: [0., 0., 1.], x: [1., 0., 0.], raio: 3. };
        let Trecho3::Circulo { centro, raio, normal, .. } = t.transformado(&Xform::Mirror([0.; 3], [1., 0., 0.])) else { panic!() };
        assert!((centro[0] + 5.).abs() < 1e-12 && (raio - 3.).abs() < 1e-12);
        assert!((normal[2] + 1.).abs() < 1e-12, "o espelho inverte o sentido");
    }

    #[test]
    fn codificacao_comeca_pelo_marcador() {
        let c = Curva3::retas(&[[0.; 3], [1., 0., 0.], [1., 1., 0.]]);
        let v = c.codifica();
        assert!(v[0].is_nan());
        assert_eq!(v[1], 2.);
    }
}
