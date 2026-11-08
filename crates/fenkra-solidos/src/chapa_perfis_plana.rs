// SPDX-License-Identifier: LGPL-2.1-or-later
//! Planificação da aba entre perfis (B35) pelo método da triangulação, o mesmo das tabelas de
//! caldeiraria para funis e adaptadores.
//!
//! Os dois perfis são amostrados em pontos que se correspondem (cada trecho de um perfil com o
//! trecho do outro, retas com uma divisão só e curvas com várias). Entre os pontos vizinhos a
//! transição é uma faixa de quadriláteros, e cada um é partido pela diagonal em dois triângulos. Os
//! triângulos são abertos no plano um depois do outro, só pelas distâncias entre os vértices: num
//! quadrilátero plano (funil facetado) isso é exato, e numa transição curva o erro cai com o número
//! de divisões. A faixa é aberta na geratriz do primeiro ponto, que vira a costura.
//!
//! As distâncias são medidas na linha neutra: cada vértice sai do perfil para dentro do material,
//! na distância da fibra neutra, pelo cruzamento dos planos das duas faces vizinhas deslocados com o
//! plano do próprio perfil (é onde a casca de espessura termina). As geratrizes nos cantos dos
//! perfis, onde duas faces planas se encontram com ângulo, viram linhas de dobra de raio zero.

use fenkra_kernel::{cross, Loop2, Seg2};

type V3 = [f64; 3];
type V2 = [f64; 2];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norma(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> Option<V3> {
    let n = norma(a);
    (n > 1e-12).then(|| mul(a, 1. / n))
}
fn dist(a: V3, b: V3) -> f64 {
    norma(sub(a, b))
}

/// Um perfil já amostrado: os pontos em ordem (sem repetir o primeiro no fim) e, para cada ponto,
/// se ele é o começo de um trecho do sketch (um canto, onde pode haver dobra).
#[derive(Clone, Debug, PartialEq)]
pub struct PerfilAmostrado {
    pub pontos: Vec<V3>,
    pub canto: Vec<bool>,
}

/// Linha de dobra na chapa planificada, em coordenadas do plano (x, y).
#[derive(Clone, Debug, PartialEq)]
pub struct DobraPlana {
    pub a: V2,
    pub b: V2,
    /// ângulo da dobra (graus) e se ela fecha para o lado de fora dos perfis
    pub angulo: f64,
    pub para_fora: bool,
}

/// A chapa planificada: contorno fechado (sem repetir o primeiro ponto) e linhas de dobra.
#[derive(Clone, Debug, PartialEq)]
pub struct Desenvolvimento {
    pub contorno: Vec<V2>,
    pub dobras: Vec<DobraPlana>,
    /// comprimento das duas bordas desenvolvidas (do perfil de baixo e do de cima)
    pub borda_a: f64,
    pub borda_b: f64,
}

impl Desenvolvimento {
    /// Área do contorno (fórmula do laço).
    pub fn area(&self) -> f64 {
        let n = self.contorno.len();
        (0..n).map(|i| {
            let (p, q) = (self.contorno[i], self.contorno[(i + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f64>()
        .abs()
            / 2.
    }
}

/// Polilinha densa de um trecho do sketch, do início ao fim.
fn polilinha(s: &Seg2) -> Vec<V2> {
    match s {
        Seg2::Line(a, b) => vec![*a, *b],
        Seg2::Arc(a, m, b) => match circulo_por_tres(*a, *m, *b) {
            Some((c, r, t0, varre)) => (0..=96).map(|k| {
                let t = t0 + varre * k as f64 / 96.;
                [c[0] + r * t.cos(), c[1] + r * t.sin()]
            })
            .collect(),
            None => vec![*a, *b],
        },
        Seg2::Circle(c, r) => (0..=192).map(|k| {
            let t = k as f64 / 192. * std::f64::consts::TAU;
            [c[0] + r * t.cos(), c[1] + r * t.sin()]
        })
        .collect(),
        Seg2::Spline(p) => p.clone(),
        Seg2::Ellipse(c, m, r, t0, t1) => fenkra_core::projecao::elipse_amostras(*c, *m, *r, *t0, *t1),
    }
}

/// Centro, raio, ângulo inicial e varredura (com sinal) do arco que passa por `a`, `m` e `b`.
fn circulo_por_tres(a: V2, m: V2, b: V2) -> Option<(V2, f64, f64, f64)> {
    let d = 2. * (a[0] * (m[1] - b[1]) + m[0] * (b[1] - a[1]) + b[0] * (a[1] - m[1]));
    if d.abs() < 1e-12 {
        return None;
    }
    let q = |p: V2| p[0] * p[0] + p[1] * p[1];
    let cx = (q(a) * (m[1] - b[1]) + q(m) * (b[1] - a[1]) + q(b) * (a[1] - m[1])) / d;
    let cy = (q(a) * (b[0] - m[0]) + q(m) * (a[0] - b[0]) + q(b) * (m[0] - a[0])) / d;
    let ang = |p: V2| (p[1] - cy).atan2(p[0] - cx);
    let (ta, tm, tb) = (ang(a), ang(m), ang(b));
    let tau = std::f64::consts::TAU;
    let ccw = |x: f64| (x - ta).rem_euclid(tau);
    // anti-horário se o meio vem antes do fim andando no sentido anti-horário
    let varre = if ccw(tm) < ccw(tb) { ccw(tb) } else { ccw(tb) - tau };
    Some(([cx, cy], ((a[0] - cx).powi(2) + (a[1] - cy).powi(2)).sqrt(), ta, varre))
}

/// Amostra os dois laços com pontos que se correspondem. Os dois precisam do mesmo número de
/// trechos; o segundo é girado (e invertido, se preciso) para o par de cantos mais perto, como o
/// kernel faz no loft. `para3d_a`/`para3d_b` levam do plano de cada sketch ao espaço.
pub fn amostrar(a: &Loop2, b: &Loop2, para3d_a: &dyn Fn(V2) -> V3, para3d_b: &dyn Fn(V2) -> V3) -> Result<(PerfilAmostrado, PerfilAmostrado), String> {
    if a.len() != b.len() || a.is_empty() {
        return Err(format!("a planificação liga trecho com trecho e os perfis têm {} e {} trechos: desenhe os dois com o mesmo número de lados", a.len(), b.len()));
    }
    let pa: Vec<Vec<V3>> = a.iter().map(|s| polilinha(s).into_iter().map(para3d_a).collect()).collect();
    let pb: Vec<Vec<V3>> = b.iter().map(|s| polilinha(s).into_iter().map(para3d_b).collect()).collect();
    let centro = |p: &Vec<Vec<V3>>| {
        let n = p.len() as f64;
        p.iter().fold([0.; 3], |c, s| add(c, mul(s[0], 1. / n)))
    };
    let (ca, cb) = (centro(&pa), centro(&pb));
    let n = a.len();
    // variantes do segundo laço: n giros, nos dois sentidos
    let inverte = |p: &Vec<Vec<V3>>| -> Vec<Vec<V3>> { p.iter().rev().map(|s| s.iter().rev().copied().collect()).collect() };
    let mut melhor: Option<(f64, Vec<Vec<V3>>)> = None;
    for sentido in [pb.clone(), inverte(&pb)] {
        for k in 0..n {
            let v: Vec<Vec<V3>> = (0..n).map(|i| sentido[(i + k) % n].clone()).collect();
            let custo: f64 = (0..n).map(|i| dist(sub(pa[i][0], ca), sub(v[i][0], cb)).powi(2)).sum();
            if melhor.as_ref().is_none_or(|m| custo < m.0 - 1e-9) {
                melhor = Some((custo, v));
            }
        }
    }
    let pb = melhor.map(|m| m.1).unwrap_or(pb);
    let (mut sa, mut sb) = (PerfilAmostrado { pontos: vec![], canto: vec![] }, PerfilAmostrado { pontos: vec![], canto: vec![] });
    for i in 0..n {
        let reta = |s: &Seg2| matches!(s, Seg2::Line(..));
        let m = if reta(&a[i]) && pb[i].len() == 2 { 1 } else if matches!(a[i], Seg2::Circle(..)) { 72 } else { 24 };
        let (xa, xb) = (reamostra2(&pa[i], m), reamostra2(&pb[i], m));
        for (j, (p, q)) in xa.into_iter().zip(xb).enumerate() {
            sa.pontos.push(p);
            sb.pontos.push(q);
            sa.canto.push(j == 0);
            sb.canto.push(j == 0);
        }
    }
    Ok((sa, sb))
}

/// `m` pontos da polilinha igualmente espaçados pelo comprimento, do início (incluído) ao fim
/// (excluído).
fn reamostra2(p: &[V3], m: usize) -> Vec<V3> {
    let comp: Vec<f64> = std::iter::once(0.)
        .chain(p.windows(2).scan(0., |acc, w| {
            *acc += dist(w[1], w[0]);
            Some(*acc)
        }))
        .collect();
    let total = *comp.last().unwrap_or(&0.);
    (0..m)
        .map(|k| {
            let alvo = total * k as f64 / m as f64;
            let i = comp.partition_point(|c| *c <= alvo).clamp(1, p.len().max(2) - 1);
            let (c0, c1) = (comp[i - 1], comp[i]);
            let t = if c1 - c0 > 1e-12 { (alvo - c0) / (c1 - c0) } else { 0. };
            add(p[i - 1], mul(sub(p[i], p[i - 1]), t))
        })
        .collect()
}

/// Normal (para fora dos perfis) do triângulo `p`, `q`, `r`, já com o sentido do laço: `giro` é +1
/// quando o laço de baixo anda no sentido anti-horário visto de cima.
fn normal(p: V3, q: V3, r: V3, giro: f64) -> Option<V3> {
    unit(mul(cross(sub(q, p), sub(r, p)), giro))
}

/// Ponto deslocado `delta` (para fora: positivo) das duas faces de normais `nl` e `nr`, ficando no
/// plano de normal `np` que passa por `p`.
fn desloca(p: V3, nl: V3, nr: V3, np: V3, delta: f64) -> V3 {
    // m com m·nl = 1, m·nr = 1 e m·np = 0 (cruzamento dos três planos); com as faces quase
    // paralelas, a normal média
    let det = dot(nl, cross(nr, np));
    let m = if det.abs() > 1e-6 && norma(cross(nl, nr)) > 1e-4 {
        // regra de Cramer em forma vetorial, com os lados direitos 1, 1 e 0
        mul(add(cross(nr, np), cross(np, nl)), 1. / det)
    } else {
        let nm = unit(add(nl, nr)).unwrap_or(nl);
        let c = dot(nm, np);
        let den = 1. - c * c;
        if den < 1e-9 {
            nm
        } else {
            mul(sub(nm, mul(np, c)), 1. / den)
        }
    };
    add(p, mul(m, delta))
}

/// Terceiro vértice de um triângulo aberto no plano: conhecidos `a2`, `b2` (imagens de `a`, `b`) e
/// as distâncias de `x` a eles, fica à direita do segmento orientado de `a2` para `b2`.
fn abre(a2: V2, b2: V2, da: f64, db: f64) -> V2 {
    let e = [b2[0] - a2[0], b2[1] - a2[1]];
    let l = (e[0] * e[0] + e[1] * e[1]).sqrt().max(1e-12);
    let u = [e[0] / l, e[1] / l];
    let ao_longo = (da * da - db * db + l * l) / (2. * l);
    let h = (da * da - ao_longo * ao_longo).max(0.).sqrt();
    // direita de u é (u.y, -u.x)
    [a2[0] + u[0] * ao_longo + u[1] * h, a2[1] + u[1] * ao_longo - u[0] * h]
}

/// Desenvolve a transição entre `a` (de baixo) e `b` (de cima), com a fibra neutra a `delta` dos
/// perfis (positivo para fora deles). `na` e `nb` são as normais dos planos dos perfis.
pub fn desenvolver(a: &PerfilAmostrado, b: &PerfilAmostrado, na: V3, nb: V3, delta: f64) -> Result<Desenvolvimento, String> {
    let n = a.pontos.len();
    if n < 3 || b.pontos.len() != n {
        return Err("os perfis precisam de pelo menos três pontos correspondentes".into());
    }
    let (p, q) = (&a.pontos, &b.pontos);
    // sentido do laço de baixo em volta do eixo que vai de um perfil ao outro
    let c = |v: &Vec<V3>| v.iter().fold([0.; 3], |s, x| add(s, mul(*x, 1. / n as f64)));
    let (ca, cb) = (c(p), c(q));
    let eixo = unit(sub(cb, ca)).ok_or("os dois perfis estão no mesmo lugar")?;
    let area: V3 = (0..n).fold([0.; 3], |s, i| add(s, cross(sub(p[i], ca), sub(p[(i + 1) % n], ca))));
    let giro = if dot(area, eixo) >= 0. { 1. } else { -1. };
    // cada quadrilátero i (p_i, p_i+1, q_i+1, q_i) em dois triângulos pela diagonal p_i–q_i+1:
    // T1 = (p_i, q_i+1, q_i) com a borda de cima e T2 = (p_i, p_i+1, q_i+1) com a de baixo
    let nt1: Vec<V3> = (0..n).map(|i| normal(p[i], q[(i + 1) % n], q[i], giro).or_else(|| normal(p[i], p[(i + 1) % n], q[(i + 1) % n], giro)).unwrap_or(eixo)).collect();
    let nt2: Vec<V3> = (0..n).map(|i| normal(p[i], p[(i + 1) % n], q[(i + 1) % n], giro).unwrap_or(nt1[i])).collect();
    // vértices na fibra neutra: cada ponto entre a face da esquerda (i-1) e a da direita (i)
    let pn: Vec<V3> = (0..n).map(|i| desloca(p[i], nt2[(i + n - 1) % n], nt2[i], na, delta)).collect();
    let qn: Vec<V3> = (0..n).map(|i| desloca(q[i], nt1[(i + n - 1) % n], nt1[i], nb, delta)).collect();
    // abre a faixa: a geratriz 0 em pé no eixo y, andando para +x
    let mut p2 = vec![[0., 0.]];
    let mut q2 = vec![[0., dist(pn[0], qn[0])]];
    let mut dobras = vec![];
    for i in 0..n {
        let j = (i + 1) % n;
        let (pi, qi) = (p2[i], q2[i]);
        // T1: q_j a partir da geratriz p_i–q_i
        let qj = abre(pi, qi, dist(pn[i], qn[j]), dist(qn[i], qn[j]));
        // T2: p_j a partir da diagonal p_i–q_j
        let pj = abre(pi, qj, dist(pn[i], pn[j]), dist(qn[j], pn[j]));
        p2.push(pj);
        q2.push(qj);
        // linha de dobra na geratriz j, se é um canto dos dois perfis e as faces fazem ângulo
        if j != 0 && a.canto[j] && b.canto[j] {
            let (nl, nr) = (nt2[i], nt1[j]);
            let ang = dot(nl, nr).clamp(-1., 1.).acos().to_degrees();
            if ang > 0.5 {
                // fecha para fora se a face seguinte cai para o lado de dentro da normal da anterior
                let centro_dir = mul(add(add(p[j], q[j]), add(p[(j + 1) % n], q[(j + 1) % n])), 0.25);
                let para_fora = dot(nl, sub(centro_dir, p[j])) > 0.;
                dobras.push(DobraPlana { a: pj, b: qj, angulo: ang, para_fora });
            }
        }
    }
    let borda = |v: &Vec<V2>| v.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt()).sum::<f64>();
    let (borda_a, borda_b) = (borda(&p2), borda(&q2));
    let mut contorno = p2;
    contorno.extend(q2.into_iter().rev());
    // o contorno anda à direita: o de baixo para +x e o de cima de volta
    Ok(Desenvolvimento { contorno, dobras, borda_a, borda_b })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadrado(meio: f64) -> Loop2 {
        let c = [[-meio, -meio], [meio, -meio], [meio, meio], [-meio, meio]];
        (0..4).map(|i| Seg2::Line(c[i], c[(i + 1) % 4])).collect()
    }

    /// Funil quadrado 40 para 20 a 30 de altura, na face de fora (delta 0): cada face é um trapézio
    /// plano, então a planificada tem a área das quatro faces e as bordas o perímetro de cada perfil.
    #[test]
    fn funil_quadrado_abre_com_a_area_das_faces() {
        let (a, b) = amostrar(&quadrado(20.), &quadrado(10.), &|p| [p[0], p[1], 0.], &|p| [p[0], p[1], 30.]).unwrap();
        let d = desenvolver(&a, &b, [0., 0., 1.], [0., 0., 1.], 0.).unwrap();
        let altura = (30f64.powi(2) + 10f64.powi(2)).sqrt();
        let area = 4. * (40. + 20.) / 2. * altura;
        assert!((d.area() - area).abs() < 1e-6 * area, "área {} esperada {area}", d.area());
        assert!((d.borda_a - 160.).abs() < 1e-9 && (d.borda_b - 80.).abs() < 1e-9);
        // três dobras nos cantos internos (o quarto canto é a costura), todas iguais
        assert_eq!(d.dobras.len(), 3);
        let a0 = d.dobras[0].angulo;
        assert!(d.dobras.iter().all(|x| (x.angulo - a0).abs() < 1e-9 && !x.para_fora));
        // ângulo entre duas faces vizinhas: normais (0, -cos, sin) e (cos, 0, sin) com tan = 10/30
        let s = 10f64.atan2(30.).sin();
        let esperado = (s * s).clamp(-1., 1.).acos().to_degrees();
        assert!((a0 - esperado).abs() < 1e-6, "dobra {a0} esperada {esperado}");
    }

    /// Tronco de cone r 30 → 15, altura 40: a planificada é um setor de coroa, com as bordas do
    /// tamanho das duas circunferências e a área lateral do tronco.
    #[test]
    fn tronco_de_cone_abre_num_setor_de_coroa() {
        let ca: Loop2 = vec![Seg2::Circle([0., 0.], 30.)];
        let cb: Loop2 = vec![Seg2::Circle([0., 0.], 15.)];
        let (a, b) = amostrar(&ca, &cb, &|p| [p[0], p[1], 0.], &|p| [p[0], p[1], 40.]).unwrap();
        let d = desenvolver(&a, &b, [0., 0., 1.], [0., 0., 1.], 0.).unwrap();
        let tau = std::f64::consts::TAU;
        let g = (40f64.powi(2) + 15f64.powi(2)).sqrt();
        let lateral = std::f64::consts::PI * (30. + 15.) * g;
        assert!((d.area() - lateral).abs() < 0.005 * lateral, "área {} esperada {lateral}", d.area());
        assert!((d.borda_a - tau * 30.).abs() < 0.005 * tau * 30., "borda de baixo {}", d.borda_a);
        assert!((d.borda_b - tau * 15.).abs() < 0.005 * tau * 15., "borda de cima {}", d.borda_b);
        assert!(d.dobras.is_empty(), "cone é calandrado, sem linha de dobra");
        // setor de coroa: o ângulo do setor é 2π·r/geratriz-do-ápice = 2π·30/(2g)
        let raio_ext = 2. * g;
        let angulo = tau * 30. / raio_ext;
        let corda = 2. * raio_ext * (angulo / 2.).sin();
        let (p0, pn) = (d.contorno[0], d.contorno[a.pontos.len()]);
        let corda_obtida = ((pn[0] - p0[0]).powi(2) + (pn[1] - p0[1]).powi(2)).sqrt();
        assert!((corda_obtida - corda).abs() < 0.01 * corda, "corda {corda_obtida} esperada {corda}");
    }

    /// A fibra neutra para dentro encurta as bordas: no plano do perfil, a face inclinada deslocada
    /// 1 mm na normal recua 1/cos(inclinação), dos dois lados de cada canto.
    #[test]
    fn fibra_neutra_para_dentro_encurta_as_bordas() {
        let (a, b) = amostrar(&quadrado(20.), &quadrado(10.), &|p| [p[0], p[1], 0.], &|p| [p[0], p[1], 30.]).unwrap();
        let fora = desenvolver(&a, &b, [0., 0., 1.], [0., 0., 1.], 0.).unwrap();
        let dentro = desenvolver(&a, &b, [0., 0., 1.], [0., 0., 1.], -1.).unwrap();
        // a face inclinada recua 1 mm na normal, que no plano do perfil é 1/cos(inclinação)
        let incl = 10f64.atan2(30.);
        let recuo = 1. / incl.cos();
        assert!((dentro.borda_a - (160. - 8. * recuo)).abs() < 1e-6, "borda {}", dentro.borda_a);
        assert!(dentro.area() < fora.area());
    }

    #[test]
    fn perfis_com_trechos_diferentes_erram_em_palavras() {
        let tri: Loop2 = vec![Seg2::Line([0., 0.], [10., 0.]), Seg2::Line([10., 0.], [0., 10.]), Seg2::Line([0., 10.], [0., 0.])];
        let e = amostrar(&quadrado(20.), &tri, &|p| [p[0], p[1], 0.], &|p| [p[0], p[1], 30.]).unwrap_err();
        assert!(e.contains("mesmo número de lados"), "{e}");
    }

    #[test]
    fn segundo_perfil_girado_e_alinhado_ao_primeiro() {
        // o mesmo quadrado começando de outro canto e no outro sentido
        let c = [[10., 10.], [10., -10.], [-10., -10.], [-10., 10.]];
        let b: Loop2 = (0..4).map(|i| Seg2::Line(c[i], c[(i + 1) % 4])).collect();
        let (pa, pb) = amostrar(&quadrado(20.), &b, &|p| [p[0], p[1], 0.], &|p| [p[0], p[1], 30.]).unwrap();
        for (x, y) in pa.pontos.iter().zip(&pb.pontos) {
            assert!((x[0] / 2. - y[0]).abs() < 1e-9 && (x[1] / 2. - y[1]).abs() < 1e-9, "{x:?} {y:?}");
        }
    }
}
