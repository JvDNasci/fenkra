// SPDX-License-Identifier: LGPL-2.1-or-later
//! Multi-seções com controle (B34, B290): guias que a superfície toca, conectores que dizem qual ponto
//! de uma seção vai para qual ponto da outra e tangência nas pontas.
//!
//! O kernel escolhe sozinho onde cada seção começa, e é daí que vem a torção. Aqui a correspondência
//! é nossa: cada seção vira uma função de um parâmetro comum `u`, com os conectores e as guias nos
//! mesmos valores de `u` em todas elas. De seção em seção a superfície é uma spline em `v`.
//!
//! Com guias vale a soma de Gordon, `S = Sv + Su - Suv`: `Sv` interpola as seções, `Su` interpola as
//! guias e `Suv` tira o que as duas têm em comum (a interpolação dos cruzamentos). O resultado passa
//! pelas seções e pelas guias ao mesmo tempo, sem iteração.
//!
//! Canto vivo de seção não cabe numa B-spline cúbica sem ondular, por isso a grade é cortada ali e
//! cada trecho vira uma face.

use crate::geo3d::*;
use fenkra_kernel::Shape;

/// Pedido, já com as curvas calculadas.
#[derive(Clone, Debug, Default)]
pub struct Entrada {
    pub secoes: Vec<Curve3>,
    pub guias: Vec<Curve3>,
    /// frações por conector, uma por seção, no sentido próprio da curva
    pub conectores: Vec<Vec<f64>>,
    pub invertidas: Vec<bool>,
    pub regrada: bool,
    /// volta à primeira seção
    pub fechar: bool,
    /// tamanho da derivada em cada ponta (mm); zero deixa livre
    pub tangentes: [f64; 2],
}

/// Interpolação linear em relação aos valores: cada ponto da curva é uma soma ponderada dos nós e,
/// quando presas, das derivadas nas pontas.
struct Interp {
    nos: Vec<f64>,
    periodica: bool,
    /// segundas derivadas nos nós para cada valor unitário (colunas: nós, depois as duas derivadas)
    m: Vec<Vec<f64>>,
}

/// Eliminação de Gauss com pivô: os sistemas aqui têm o tamanho do número de seções.
fn resolver(mut a: Vec<Vec<f64>>, mut b: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, String> {
    let n = a.len();
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs())).unwrap_or(c);
        if a[p][c].abs() < 1e-14 {
            return Err("seções ou guias coincidentes".into());
        }
        a.swap(c, p);
        b.swap(c, p);
        for l in 0..n {
            if l == c {
                continue;
            }
            let k = a[l][c] / a[c][c];
            if k != 0. {
                for j in c..n {
                    a[l][j] -= k * a[c][j];
                }
                for j in 0..b[l].len() {
                    b[l][j] -= k * b[c][j];
                }
            }
        }
    }
    for l in 0..n {
        let k = a[l][l];
        b[l].iter_mut().for_each(|x| *x /= k);
    }
    Ok(b)
}

impl Interp {
    /// Spline cúbica pelos `nos`, crescentes. Na periódica o último nó é o primeiro de novo, um período
    /// depois, e o valor dele não conta como valor próprio.
    fn nova(nos: Vec<f64>, periodica: bool, presa: [bool; 2]) -> Result<Interp, String> {
        let n = nos.len();
        let k = if periodica { n - 1 } else { n };
        let h: Vec<f64> = nos.windows(2).map(|w| w[1] - w[0]).collect();
        if h.iter().any(|x| *x < 1e-12) {
            return Err("seções ou guias fora de ordem".into());
        }
        let colunas = k + 2;
        let (mut a, mut b) = (vec![vec![0.; k]; k], vec![vec![0.; colunas]; k]);
        if periodica {
            for i in 0..k {
                let (p, q) = (h[(i + k - 1) % k], h[i]);
                let (ant, seg) = ((i + k - 1) % k, (i + 1) % k);
                a[i][ant] += p;
                a[i][i] += 2. * (p + q);
                a[i][seg] += q;
                b[i][seg] += 6. / q;
                b[i][i] -= 6. / q + 6. / p;
                b[i][ant] += 6. / p;
            }
        } else {
            for i in 1..k - 1 {
                let (p, q) = (h[i - 1], h[i]);
                a[i][i - 1] = p;
                a[i][i] = 2. * (p + q);
                a[i][i + 1] = q;
                b[i][i + 1] = 6. / q;
                b[i][i] = -6. / q - 6. / p;
                b[i][i - 1] = 6. / p;
            }
            if presa[0] {
                a[0][0] = 2. * h[0];
                a[0][1] = h[0];
                b[0][1] = 6. / h[0];
                b[0][0] = -6. / h[0];
                b[0][k] = -6.;
            } else {
                a[0][0] = 1.;
            }
            let u = k - 1;
            if presa[1] {
                a[u][u] = 2. * h[u - 1];
                a[u][u - 1] = h[u - 1];
                b[u][u - 1] = 6. / h[u - 1];
                b[u][u] = -6. / h[u - 1];
                b[u][k + 1] = 6.;
            } else {
                a[u][u] = 1.;
            }
        }
        let m = resolver(a, b)?;
        Ok(Interp { nos, periodica, m })
    }

    /// Peso de cada nó em `x`, e das duas derivadas nos dois últimos lugares.
    fn pesos(&self, x: f64) -> Vec<f64> {
        let n = self.nos.len();
        let k = if self.periodica { n - 1 } else { n };
        let mut w = vec![0.; k + 2];
        let (t0, t1) = (self.nos[0], self.nos[n - 1]);
        let x = if self.periodica { t0 + (x - t0).rem_euclid(t1 - t0) } else { x };
        let i = self.nos.windows(2).position(|p| x <= p[1]).unwrap_or(n - 2);
        let h = self.nos[i + 1] - self.nos[i];
        let (a, b) = ((self.nos[i + 1] - x) / h, (x - self.nos[i]) / h);
        let (i1, seg) = (i, (i + 1) % k);
        w[i1] += a;
        w[seg] += b;
        let (ca, cb) = ((a * a * a - a) * h * h / 6., (b * b * b - b) * h * h / 6.);
        for c in 0..k + 2 {
            w[c] += ca * self.m[i1][c] + cb * self.m[seg][c];
        }
        w
    }
}

/// Interpolação cardinal linear, por trechos, para a superfície regrada.
fn pesos_lineares(nos: &[f64], x: f64) -> Vec<f64> {
    let n = nos.len();
    let mut w = vec![0.; n + 2];
    let i = nos.windows(2).position(|p| x <= p[1]).unwrap_or(n - 2);
    let t = ((x - nos[i]) / (nos[i + 1] - nos[i])).clamp(0., 1.);
    w[i] = 1. - t;
    w[i + 1] = t;
    w
}

/// Ponto da curva na fração `f` do comprimento; na fechada, `f` dá a volta.
fn ponto(c: &Curve3, f: f64, fechada: bool) -> V3 {
    let f = if fechada { f.rem_euclid(1.) } else { f.clamp(0., 1.) };
    c.at_dist(f * c.length()).0
}

/// Fração da curva mais perto de `p`, com a distância até ela.
fn fracao_de(c: &Curve3, p: V3) -> (f64, f64) {
    let (s, q) = c.nearest(p);
    (s / c.length().max(1e-12), dist(p, q))
}

/// Frações onde a seção tem canto vivo.
fn cantos(c: &Curve3, fechada: bool) -> Vec<f64> {
    let p = &c.pts;
    let n = if fechada { p.len() - 1 } else { p.len() };
    let total = c.length().max(1e-12);
    let mut acc = 0.;
    let mut out = vec![];
    for i in 0..n {
        if i > 0 {
            acc += dist(p[i - 1], p[i]);
        }
        let (ant, seg) = match (i, fechada) {
            (0, false) => continue,
            (0, true) => (p[n - 1], p[1]),
            _ if i + 1 >= p.len() => continue,
            _ => (p[i - 1], p[i + 1]),
        };
        if let (Some(u), Some(v)) = (unit(sub(p[i], ant)), unit(sub(seg, p[i]))) {
            if dot(u, v) < 0.87 {
                out.push(acc / total);
            }
        }
    }
    out
}

/// Vetor de área: dá o sentido em que a seção fechada gira.
fn giro(p: &[V3]) -> V3 {
    let c = p.iter().fold([0.; 3], |a, q| add(a, *q));
    let c = mul(c, 1. / p.len() as f64);
    p.windows(2).fold([0.; 3], |a, w| add(a, cross(sub(w[0], c), sub(w[1], c))))
}

fn centro(c: &Curve3) -> V3 {
    let n = 32;
    let s = (0..n).fold([0.; 3], |a, i| add(a, ponto(c, i as f64 / n as f64, false)));
    mul(s, 1. / n as f64)
}

/// Seção com o sentido e o mapa de `u` para fração da curva.
struct Secao {
    curva: Curve3,
    /// `u` e fração nos nós do mapa, crescentes; na fechada a última é a primeira mais um
    mapa: Vec<(f64, f64)>,
}

impl Secao {
    fn fracao(&self, u: f64) -> f64 {
        let i = self.mapa.windows(2).position(|p| u <= p[1].0).unwrap_or(self.mapa.len() - 2);
        let ((u0, f0), (u1, f1)) = (self.mapa[i], self.mapa[i + 1]);
        f0 + (f1 - f0) * ((u - u0) / (u1 - u0).max(1e-12)).clamp(0., 1.)
    }
    /// `u` da fração `f`: o caminho inverso do mapa.
    fn u_de(&self, f: f64, fechada: bool) -> f64 {
        let f = if fechada { self.mapa[0].1 + (f - self.mapa[0].1).rem_euclid(1.) } else { f };
        let i = self.mapa.windows(2).position(|p| f <= p[1].1).unwrap_or(self.mapa.len() - 2);
        let ((u0, a), (u1, b)) = (self.mapa[i], self.mapa[i + 1]);
        u0 + (u1 - u0) * ((f - a) / (b - a).max(1e-12)).clamp(0., 1.)
    }
    fn em(&self, u: f64, fechada: bool) -> V3 {
        ponto(&self.curva, self.fracao(u), fechada)
    }
}

/// Curva que a grade precisa tocar, já na parametrização comum em `v`.
struct Guia {
    curva: Curve3,
    /// `v` da seção e fração da guia no cruzamento
    mapa: Vec<(f64, f64)>,
    /// `u` do cruzamento em todas as seções
    u: f64,
}

impl Guia {
    fn em(&self, v: f64) -> V3 {
        let i = self.mapa.windows(2).position(|p| v <= p[1].0).unwrap_or(self.mapa.len() - 2);
        let ((v0, f0), (v1, f1)) = (self.mapa[i], self.mapa[i + 1]);
        ponto(&self.curva, f0 + (f1 - f0) * ((v - v0) / (v1 - v0).max(1e-12)).clamp(0., 1.), false)
    }
}

/// Afastamento aceito entre guia e seção: 0,5% da seção, com um piso para peças pequenas.
fn folga_da_guia(c: &Curve3) -> f64 {
    (c.length() * 0.005).max(0.01)
}

/// Ponto da guia mais perto da seção e a distância. A guia é percorrida em passos curtos: uma reta tem
/// só as duas pontas na lista de pontos.
fn cruzamento(g: &Curve3, c: &Curve3) -> Result<(V3, f64), String> {
    resample(g, 400).into_iter().map(|p| (p, fracao_de(c, p).1)).min_by(|a, b| a.1.total_cmp(&b.1)).ok_or_else(|| "guia vazia".to_string())
}

/// Sentidos, frações dos conectores (das guias também) e cruzamentos das guias.
/// Seções no sentido em que a superfície as percorre, e quais foram viradas: cada uma acompanha a
/// anterior, e `invertidas` troca esse sentido por pedido da pessoa.
fn orientar(secoes: &[Curve3], invertidas: &[bool]) -> (Vec<Curve3>, Vec<bool>) {
    let fechada = |c: &Curve3| c.closed || dist(c.start(), c.end()) < 1e-6;
    let mut cs: Vec<Curve3> = vec![];
    let mut virou = vec![];
    for (i, c) in secoes.iter().enumerate() {
        let mut c = c.clone();
        let mut inv = false;
        if let Some(a) = cs.last() {
            inv = if fechada(a) {
                dot(giro(&a.pts), giro(&c.pts)) < 0.
            } else {
                dist(a.start(), c.start()) + dist(a.end(), c.end()) > dist(a.start(), c.end()) + dist(a.end(), c.start())
            };
            // o sentido das anteriores já pode estar trocado, e o vetor `a` já o carrega
        }
        inv ^= invertidas.get(i).copied().unwrap_or(false);
        if inv {
            c.pts.reverse();
        }
        virou.push(inv);
        cs.push(c);
    }
    (cs, virou)
}

/// Quais seções a superfície percorre ao contrário, para a seta do 3D dizer a verdade.
pub fn sentidos(secoes: &[Curve3], invertidas: &[bool]) -> Vec<bool> {
    orientar(secoes, invertidas).1
}

fn preparar(e: &Entrada) -> Result<(Vec<Curve3>, Vec<bool>, Vec<Vec<f64>>), String> {
    if e.secoes.len() < 2 {
        return Err("são precisas pelo menos duas seções".into());
    }
    let fechada = |c: &Curve3| c.closed || dist(c.start(), c.end()) < 1e-6;
    let f0 = fechada(&e.secoes[0]);
    if e.secoes.iter().any(|c| fechada(c) != f0) {
        return Err("misture só seções abertas ou só fechadas".into());
    }
    let (cs, virou) = orientar(&e.secoes, &e.invertidas);
    let mut cadeias: Vec<Vec<f64>> = vec![];
    for k in &e.conectores {
        if k.len() != cs.len() {
            return Err("conector sem ponto em todas as seções: reabra e confira".into());
        }
        cadeias.push(k.iter().zip(&virou).map(|(f, v)| if *v { 1. - *f } else { *f }).collect());
    }
    for (j, g) in e.guias.iter().enumerate() {
        let mut k = vec![];
        for (i, c) in cs.iter().enumerate() {
            let (p, d) = cruzamento(g, c)?;
            if d > folga_da_guia(c).max(g.length() / 400.) {
                return Err(format!("a guia {} não toca a seção {}", j + 1, i + 1));
            }
            k.push(fracao_de(c, p).0);
        }
        cadeias.push(k);
    }
    Ok((cs, virou, cadeias))
}

/// Direção que a superfície segue ao sair da seção `c`, rumo a `alvo`.
fn saida(c: &Curve3, alvo: V3) -> Result<V3, String> {
    let meio = centro(c);
    let rumo = unit(sub(alvo, meio)).ok_or("seções no mesmo lugar")?;
    if c.is_straight() {
        let d = unit(sub(c.end(), c.start())).ok_or("seção sem comprimento")?;
        return unit(sub(rumo, mul(d, dot(rumo, d)))).ok_or_else(|| "seção alinhada com a seguinte".to_string());
    }
    let (f, _) = fit_plane(&c.pts)?;
    Ok(if dot(f.normal, rumo) < 0. { mul(f.normal, -1.) } else { f.normal })
}

/// Direção e ponto de partida das setas de tangência: centro da seção e direção que entra na superfície.
pub fn setas_de_tangencia(secoes: &[Curve3]) -> Option<[(V3, V3); 2]> {
    let (a, b) = (secoes.first()?, secoes.last()?);
    let (i, j) = (centro(secoes.get(1)?), centro(secoes.get(secoes.len().checked_sub(2)?)?));
    Some([(centro(a), saida(a, i).ok()?), (centro(b), saida(b, j).ok()?)])
}

/// Valores crescentes sem repetição até `tol`.
fn unicos(mut v: Vec<f64>, tol: f64) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < tol);
    v
}

/// Grades da superfície: uma por trecho entre cantos, com as linhas em `u`.
pub fn grades(e: &Entrada) -> Result<Vec<Vec<Vec<V3>>>, String> {
    let (cs, _, cadeias) = preparar(e)?;
    let fechada = cs[0].closed || dist(cs[0].start(), cs[0].end()) < 1e-6;
    let m = cs.len();
    // conectores em ordem no primeiro corte; em cada seção a mesma ordem precisa valer
    let mut ordem: Vec<usize> = (0..cadeias.len()).collect();
    ordem.sort_by(|a, b| cadeias[*a][0].total_cmp(&cadeias[*b][0]));
    let k = ordem.len();
    // `u` de cada conector: a fração média dele nas seções. Assim uma guia na ponta da seção aberta fica
    // em `u` 0 ou 1 e não abre um trecho vazio; na fechada, conta-se a partir do primeiro conector
    let u_no = |n: usize| {
        let soma: f64 = (0..m)
            .map(|i| {
                let f = cadeias[ordem[n]][i];
                if fechada { (f - cadeias[ordem[0]][i]).rem_euclid(1.) } else { f }
            })
            .sum();
        soma / m as f64
    };
    let mut secoes = vec![];
    for (i, c) in cs.iter().enumerate() {
        let mut mapa = vec![];
        if fechada && k > 0 {
            let f0 = cadeias[ordem[0]][i];
            let mut f = f0;
            for (n, o) in ordem.iter().enumerate() {
                let mut x = cadeias[*o][i];
                while x < f - 1e-9 {
                    x += 1.;
                }
                if n > 0 && x > f0 + 1. + 1e-9 {
                    return Err("conectores cruzados: eles precisam seguir a mesma ordem em todas as seções".into());
                }
                mapa.push((u_no(n), x));
                f = x;
            }
            mapa.push((1., f0 + 1.));
        } else if fechada {
            mapa = vec![(0., 0.), (1., 1.)];
        } else {
            mapa.push((0., 0.));
            let mut f = 0.;
            for (n, o) in ordem.iter().enumerate() {
                let x = cadeias[*o][i];
                if x < f {
                    return Err("conectores cruzados: eles precisam seguir a mesma ordem em todas as seções".into());
                }
                mapa.push((u_no(n), x));
                f = x;
            }
            mapa.push((1., 1.));
            // conector na ponta: o nó repetido não acrescenta nada
            mapa.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6);
            if let Some(ultimo) = mapa.last_mut() {
                *ultimo = (1., 1.);
            }
        }
        secoes.push(Secao { curva: c.clone(), mapa });
    }
    // `v` de cada seção: distância média entre as seções, de 0 a 1
    let mut nos = vec![0.];
    let qtd = if e.fechar { m + 1 } else { m };
    for i in 1..qtd {
        let (a, b) = (&cs[i - 1], &cs[i % m]);
        let d = (0..8).map(|q| dist(ponto(a, q as f64 / 8., false), ponto(b, q as f64 / 8., false))).sum::<f64>() / 8.;
        nos.push(nos[i - 1] + d.max(1e-9));
    }
    let total = *nos.last().unwrap();
    nos.iter_mut().for_each(|x| *x /= total);
    let presa = [e.tangentes[0].abs() > 1e-9 && !e.fechar, e.tangentes[1].abs() > 1e-9 && !e.fechar];
    let interp = if e.regrada { None } else { Some(Interp::nova(nos.clone(), e.fechar, presa)?) };
    let pv = |v: f64| interp.as_ref().map_or_else(|| pesos_lineares(&nos, v), |i| i.pesos(v));
    let d0 = if presa[0] { mul(saida(&cs[0], centro(&cs[1]))?, e.tangentes[0]) } else { [0.; 3] };
    let d1 = if presa[1] { mul(saida(&cs[m - 1], centro(&cs[m - 2]))?, -e.tangentes[1]) } else { [0.; 3] };

    // guias: `u` comum e mapa em `v`
    let mut guias: Vec<Guia> = vec![];
    for g in &e.guias {
        let mut mapa = vec![];
        let mut u = 0.;
        for (i, s) in secoes.iter().enumerate() {
            let p = cruzamento(g, &s.curva)?.0;
            let f = fracao_de(g, p).0;
            if i == 0 {
                u = s.u_de(fracao_de(&s.curva, p).0, fechada);
            }
            mapa.push((nos[i], f));
        }
        if e.fechar {
            return Err("guias não valem em superfície que volta à primeira seção".into());
        }
        // a guia pode ter sido desenhada do fim para o começo
        if mapa.windows(2).all(|w| w[1].1 < w[0].1) {
            mapa.iter_mut().for_each(|x| x.1 = 1. - x.1);
            let mut c = g.clone();
            c.pts.reverse();
            guias.push(Guia { curva: c, mapa, u });
        } else if mapa.windows(2).all(|w| w[1].1 > w[0].1) {
            guias.push(Guia { curva: g.clone(), mapa, u });
        } else {
            return Err("a guia precisa cruzar as seções na ordem".into());
        }
        // as pontas da guia passam das seções: a superfície fica entre a primeira e a última
    }
    guias.sort_by(|a, b| a.u.total_cmp(&b.u));
    let interp_u = match guias.len() {
        0 | 1 => None,
        _ => {
            let mut us: Vec<f64> = guias.iter().map(|g| g.u).collect();
            if fechada {
                us.push(us[0] + 1.);
            }
            Some(Interp::nova(us, fechada, [false, false])?)
        }
    };
    let pu = |u: f64| -> Vec<f64> {
        match (&interp_u, guias.len()) {
            (Some(i), n) => i.pesos(u)[..n].to_vec(),
            (None, n) => vec![1.; n],
        }
    };
    // o cruzamento de cada guia com cada seção, para a parte comum
    let cruz: Vec<Vec<V3>> = guias.iter().map(|g| nos[..m].iter().map(|v| g.em(*v)).collect()).collect();

    // cortes em `u`: nós do mapa e cantos das seções
    let mut cortes = vec![0., 1.];
    for s in &secoes {
        cortes.extend(cantos(&s.curva, fechada).into_iter().map(|f| s.u_de(f, fechada)));
    }
    let cortes = unicos(cortes, 1e-3);
    // cortes em `v`: a regrada dobra em cada seção
    let faixas: Vec<(f64, f64)> = if e.regrada { nos.windows(2).map(|w| (w[0], w[1])).collect() } else { vec![(0., 1.)] };
    let linhas_v = |a: f64, b: f64| -> Vec<f64> {
        let n = if e.regrada { 4 } else { (16 * (qtd - 1)).max(32) };
        (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
    };

    let mut out = vec![];
    for w in cortes.windows(2) {
        let (ua, ub) = (w[0], w[1]);
        let nu = (((ub - ua) * 96.).ceil() as usize).max(6);
        let us: Vec<f64> = (0..=nu).map(|i| ua + (ub - ua) * i as f64 / nu as f64).collect();
        // os pontos das seções não mudam com `v`: uma vez por `u`
        let linhas: Vec<Vec<V3>> = us.iter().map(|u| secoes.iter().map(|s| s.em(*u, fechada)).collect()).collect();
        let pesos_u: Vec<Vec<f64>> = us.iter().map(|u| pu(*u)).collect();
        for (va, vb) in &faixas {
            let vs = linhas_v(*va, *vb);
            let pesos_v: Vec<Vec<f64>> = vs.iter().map(|v| pv(*v)).collect();
            let guias_v: Vec<Vec<V3>> = guias.iter().map(|g| vs.iter().map(|v| g.em(*v)).collect()).collect();
            let grade: Vec<Vec<V3>> = linhas
                .iter()
                .enumerate()
                .map(|(iu, sec)| {
                    vs.iter()
                        .enumerate()
                        .map(|(iv, _)| {
                            let w = &pesos_v[iv];
                            let mut p = [0.; 3];
                            for (i, q) in sec.iter().enumerate() {
                                p = add(p, mul(*q, w[i]));
                            }
                            let extra = w.len() - 2;
                            let mut soma = add(p, add(mul(d0, w[extra]), mul(d1, w[extra + 1])));
                            for (j, lu) in pesos_u[iu].iter().enumerate() {
                                // `Su - Suv`: o que a guia tem a mais que a interpolação dos cruzamentos
                                let mut comum = [0.; 3];
                                for (i, q) in cruz[j].iter().enumerate() {
                                    comum = add(comum, mul(*q, w[i]));
                                }
                                soma = add(soma, mul(sub(guias_v[j][iv], comum), *lu));
                            }
                            soma
                        })
                        .collect()
                })
                .collect();
            out.push(grade);
        }
    }
    Ok(out)
}

/// Superfície do pedido: uma face por trecho, costuradas.
pub fn superficie(e: &Entrada) -> Result<Shape, String> {
    let mut acc: Option<Shape> = None;
    for g in grades(e)? {
        let f = Shape::grid_surface(&g).map_err(|x| x.0)?;
        acc = Some(match acc {
            Some(a) => a.compound(&f),
            None => f,
        });
    }
    let s = acc.ok_or("superfície vazia")?;
    s.sewn(1e-3).map_err(|x| x.0)
}

/// Linhas que ligam os pontos de um conector, para o desenho no 3D: um ponto por seção.
pub fn pontos_do_conector(secoes: &[Curve3], fracoes: &[f64]) -> Vec<V3> {
    secoes.iter().zip(fracoes).map(|(c, f)| ponto(c, *f, c.closed)).collect()
}

/// Conector novo: parte da fração `f` da primeira seção e cai no ponto mais perto de cada seção seguinte.
pub fn conector_novo(secoes: &[Curve3], f: f64) -> Vec<f64> {
    let mut out = vec![f];
    let Some(c0) = secoes.first() else { return out };
    let p = ponto(c0, f, c0.closed);
    let meio = centro(c0);
    for c in &secoes[1..] {
        // relativo ao centro de cada seção: uma seção maior ou deslocada não puxa o conector
        let alvo = add(centro(c), sub(p, meio));
        out.push(fracao_de(c, alvo).0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aprox(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} ≠ {b}");
    }

    #[test]
    fn spline_passa_pelos_nos_e_respeita_derivada_presa() {
        let i = Interp::nova(vec![0., 0.4, 1.], false, [true, true]).unwrap();
        let w = i.pesos(0.4);
        aprox(w[1], 1., 1e-9);
        aprox(w[0], 0., 1e-9);
        // derivada numérica na ponta: valores nulos e derivada 2
        let v = |x: f64| i.pesos(x)[3] * 2.;
        aprox((v(1e-6) - v(0.)) / 1e-6, 2., 1e-3);
    }

    #[test]
    fn duas_secoes_livres_ficam_retas() {
        let i = Interp::nova(vec![0., 1.], false, [false, false]).unwrap();
        aprox(i.pesos(0.25)[0], 0.75, 1e-12);
    }

    #[test]
    fn periodica_volta_ao_comeco() {
        let i = Interp::nova(vec![0., 0.3, 1.], true, [false, false]).unwrap();
        aprox(i.pesos(1.)[0], 1., 1e-9);
        aprox(i.pesos(0.3)[1], 1., 1e-9);
        aprox(i.pesos(0.)[0], 1., 1e-9);
    }
}
