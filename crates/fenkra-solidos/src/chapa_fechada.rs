// SPDX-License-Identifier: LGPL-2.1-or-later
//! B579: aba em perfil curvo fechado (nervura de asa, caverna de fuselagem).
//!
//! Como funciona no kernel: a aba de aresta e a aba por superfície varrem a seção da dobra ao longo de uma
//! cadeia de arestas da chapa. Num perfil fechado a cadeia volta ao começo, e três coisas deixam de valer:
//!
//! - **O que é "para fora"**. Em perfil aberto bastava afastar da caixa da peça; no contorno de um furo
//!   (caverna) o lado de fora do material aponta para o centro. A sonda `lado_fora` põe dois pontos no
//!   plano da face, um de cada lado da borda, e fica com o que cai fora do material (distância ao sólido
//!   maior que zero).
//! - **A seção do suporte vem em pedaços**. A interseção de um suporte fechado (superfície extrudada de
//!   uma spline fechada, face lateral de um loft) com o plano da web chega em uma ou várias polilinhas,
//!   quebradas na costura ou entre as faces do suporte. `junta` as une pelas pontas.
//! - **A faixa planificada é um anel**, não uma tira: duas curvas fechadas (a borda da chapa e a borda
//!   livre) formam dois laços de uma face. Com um laço só, a tira voltava para o começo por um corte de
//!   largura zero e a face nascia inválida.

use crate::geo3d::{self, V3};
use fenkra_kernel::{Bool, Frame, Seg2, Shape};

/// A polilinha volta ao ponto onde começou?
pub fn fechada(pts: &[V3]) -> bool {
    if pts.len() < 4 {
        return false;
    }
    let c = geo3d::dist(pts[0], pts[pts.len() - 1]);
    let passo = pts.windows(2).map(|w| geo3d::dist(w[0], w[1])).fold(0., f64::max);
    c < 1e-3 + 1e-3 * passo
}

/// Índice do vértice onde a linha fechada `pts` (última = primeira) é mais reta, medida com vizinhos a uma
/// `janela` de distância ao longo da linha. A varredura da dobra começa e termina ali: numa costura com canto (o
/// bordo de ataque de uma spline fechada, que se fecha nele) a seção do começo e a do fim não coincidem e a
/// união da aba falha.
pub fn inicio_mais_reto(pts: &[V3], janela: f64) -> usize {
    let n = pts.len() - 1;
    if n < 6 {
        return 0;
    }
    let mut s = vec![0.; n + 1];
    for i in 0..n {
        s[i + 1] = s[i] + geo3d::dist(pts[i], pts[(i + 1) % n]);
    }
    let total = s[n];
    let ao_longo = |i: usize, dir: i64| -> usize {
        let mut j = i as i64;
        let mut acc = 0.;
        for _ in 0..n {
            let nj = (j + dir).rem_euclid(n as i64);
            acc += geo3d::dist(pts[j as usize], pts[nj as usize]);
            j = nj;
            if acc >= janela.min(total / 6.) {
                break;
            }
        }
        j as usize
    };
    let mut melhor = (f64::MAX, 0);
    for i in 0..n {
        let (a, b) = (ao_longo(i, -1), ao_longo(i, 1));
        let (u, v) = (geo3d::sub(pts[i], pts[a]), geo3d::sub(pts[b], pts[i]));
        let giro = match (geo3d::unit(u), geo3d::unit(v)) {
            (Some(u), Some(v)) => geo3d::dot(u, v).clamp(-1., 1.).acos(),
            _ => f64::MAX,
        };
        if giro < melhor.0 {
            melhor = (giro, i);
        }
    }
    melhor.1
}

/// Une pedaços de polilinha pelas pontas coincidentes (a menos de `tol`). Cada saída é uma cadeia; se os
/// pedaços fecham um laço, a última ponta repete a primeira.
pub fn junta(pedacos: Vec<Vec<V3>>, tol: f64) -> Vec<Vec<V3>> {
    let mut resto: Vec<Vec<V3>> = pedacos.into_iter().filter(|p| p.len() >= 2).collect();
    let mut saida = vec![];
    while let Some(mut cad) = resto.pop() {
        loop {
            if fechada(&cad) {
                break;
            }
            let (a, z) = (cad[0], cad[cad.len() - 1]);
            // o pedaço que encosta numa das pontas (na ordem ou invertido): (índice, inverte, no fim)
            let achou = resto.iter().enumerate().find_map(|(i, p)| {
                let (pa, pz) = (p[0], p[p.len() - 1]);
                if geo3d::dist(z, pa) < tol {
                    Some((i, false, true))
                } else if geo3d::dist(z, pz) < tol {
                    Some((i, true, true))
                } else if geo3d::dist(a, pz) < tol {
                    Some((i, false, false))
                } else if geo3d::dist(a, pa) < tol {
                    Some((i, true, false))
                } else {
                    None
                }
            });
            let Some((i, inverte, no_fim)) = achou else { break };
            let mut p = resto.swap_remove(i);
            if inverte {
                p.reverse();
            }
            if no_fim {
                cad.extend(p.into_iter().skip(1));
            } else {
                p.pop();
                p.extend(cad);
                cad = p;
            }
        }
        saida.push(cad);
    }
    saida
}

/// Sinal (±1) com que `w` (tangente × normal da chapa, unitário, no plano da face) aponta para fora do
/// material no ponto `p` da borda, pela sonda de dois pontos. `None` quando a sonda não decide (os dois
/// lados dentro ou os dois fora do material: borda de uma face estreita).
pub fn lado_fora(corpo: &Shape, p: V3, w: V3, t: f64) -> Option<f64> {
    let e = (0.3 * t).max(1e-3);
    let fora = |s: f64| {
        let q = geo3d::add(p, geo3d::mul(w, s * e));
        corpo.surface_near(q).map(|(c, _)| geo3d::dist(c, q) > 0.5 * e).unwrap_or(false)
    };
    match (fora(1.), fora(-1.)) {
        (true, false) => Some(1.),
        (false, true) => Some(-1.),
        _ => None,
    }
}

/// Laço de segmentos retos pelos pontos (uma aresta por par): para a borda livre deslocada de uma curva
/// fechada, que no nariz de um aerofólio abre em leque (pontos a dezenas de mm um do outro ao lado de pontos
/// a 1 mm) e faz uma spline interpolada oscilar.
pub fn poligono(v: &[[f64; 2]]) -> Vec<Seg2> {
    v.windows(2).filter(|w| (w[0][0] - w[1][0]).hypot(w[0][1] - w[1][1]) > 1e-9).map(|w| Seg2::Line(w[0], w[1])).collect()
}

/// Anel plano entre duas curvas fechadas (no plano de `fr`): o prisma da maior menos o prisma da menor. A face
/// com dois laços do kernel decide quem é o furo pela área calculada por integração, que erra em laço de
/// spline longa (a faixa saía cheia, medido); aqui a maior é a de maior área pela fórmula do polígono. O prisma
/// do furo passa 1 mm dos dois lados para o corte não deixar faces coincidentes. `dir`, `frente` e `tras` são
/// os de `Shape::extrude`. `sa` e `sb` montam os segmentos de cada laço (spline pela borda da chapa, polígono
/// pela borda deslocada).
#[allow(clippy::too_many_arguments)]
pub fn anel(fr: &Frame, a: &[[f64; 2]], sa: &dyn Fn(&[[f64; 2]]) -> Vec<Seg2>, b: &[[f64; 2]], sb: &dyn Fn(&[[f64; 2]]) -> Vec<Seg2>, dir: V3, frente: f64, tras: f64) -> Result<Shape, String> {
    let area = |p: &[[f64; 2]]| p.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>().abs() / 2.;
    let ((grande, sg), (pequeno, sp)) = if area(a) >= area(b) { ((a, sa), (b, sb)) } else { ((b, sb), (a, sa)) };
    let k = |e: fenkra_kernel::KernelError| e.0;
    // o laço em duas metades: uma aresta que fecha sobre si mesma (spline fechada) deixa a costura frágil nas
    // booleanas (a união da faixa com a web saía vazia, medido)
    let laco = |v: &[[f64; 2]], s: &dyn Fn(&[[f64; 2]]) -> Vec<Seg2>| {
        let m = v.len() / 2;
        [s(&v[..=m]), s(&v[m..])].concat()
    };
    let fora = Shape::face(fr, &[laco(grande, sg)]).map_err(k)?.extrude(dir, frente, tras).map_err(k)?;
    let furo = Shape::face(fr, &[laco(pequeno, sp)]).map_err(k)?.extrude(dir, frente + 1., tras + 1.).map_err(k)?;
    fora.boolean(&furo, Bool::Cut).map_err(k)
}

/// Borda livre da faixa planificada de uma linha fechada: cada ponto de `pts` deslocado `d` para fora (`outs`),
/// com juntas redondas nos cantos convexos e de quina (mitra) nos côncavos. Deslocar ponto a ponto pela direção
/// do ponto deixa, num canto vivo (o bordo de fuga de uma nervura, o nariz cortado), um único ponto sobre a
/// bissetriz: a faixa perde o setor que a conformação precisa naquele canto (o leque de raio `d`) ou, num canto
/// muito agudo, a borda se cruza e o prisma da faixa sai inválido (a união com a chapa devolvia volume zero). O
/// canto é o ponto onde a normal para fora gira mais de 8° de um trecho ao seguinte; ali entram pontos do arco
/// de raio `d` de 6° em 6° (convexo) ou o ponto de encontro das duas bordas (côncavo). `n` é a normal do plano da
/// chapa e `s` o sinal que `outs` tem em relação a tangente × `n`. A última amostra repete a primeira.
pub fn desloca_com_juntas(pts: &[V3], outs: &[V3], n: V3, s: f64, d: f64) -> Vec<V3> {
    let m = pts.len().saturating_sub(1);
    if m < 3 || outs.len() != pts.len() {
        return pts.iter().zip(outs).map(|(p, o)| geo3d::add(*p, geo3d::mul(*o, d))).collect();
    }
    let normal_do_trecho = |i: usize, j: usize| geo3d::unit(geo3d::sub(pts[j], pts[i])).and_then(|t| geo3d::unit(fenkra_kernel::cross(t, n)).map(|o| (t, geo3d::mul(o, s))));
    let mut v: Vec<V3> = Vec::with_capacity(pts.len() + 32);
    for i in 0..m {
        let (ant, prox) = (normal_do_trecho((i + m - 1) % m, i), normal_do_trecho(i, (i + 1) % m));
        let (Some((t_in, o_in)), Some((_, o_out))) = (ant, prox) else {
            v.push(geo3d::add(pts[i], geo3d::mul(outs[i], d)));
            continue;
        };
        let c = geo3d::dot(o_in, o_out).clamp(-1., 1.);
        let giro = c.acos();
        if giro < 8f64.to_radians() {
            v.push(geo3d::add(pts[i], geo3d::mul(outs[i], d)));
            continue;
        }
        let a = geo3d::add(pts[i], geo3d::mul(o_in, d));
        let b = geo3d::add(pts[i], geo3d::mul(o_out, d));
        if geo3d::dot(geo3d::sub(b, a), t_in) > 0. {
            // convexo: o arco de raio d de uma normal à outra
            let sinal = if geo3d::dot(fenkra_kernel::cross(o_in, o_out), n) >= 0. { 1. } else { -1. };
            let passos = (giro / 6f64.to_radians()).ceil().max(1.) as usize;
            for k in 0..=passos {
                let ang = sinal * giro * k as f64 / passos as f64;
                let (sn, cs) = ang.sin_cos();
                let perp = fenkra_kernel::cross(n, o_in);
                let o = geo3d::add(geo3d::mul(o_in, cs), geo3d::mul(perp, sn));
                v.push(geo3d::add(pts[i], geo3d::mul(o, d)));
            }
        } else {
            // côncavo: as duas bordas se cruzam no ponto de encontro (mitra)
            let soma = geo3d::add(o_in, o_out);
            let f = d / (1. + c).max(1e-3);
            v.push(geo3d::add(pts[i], geo3d::mul(soma, f)));
        }
    }
    v.push(v[0]);
    v
}

/// Normal (para fora do material) da face de chapa a que a linha fechada `pts` pertence, em `mid` (o ponto do meio).
/// O plano da linha dá a direção (método de Newell); o sinal e o lado vêm de raios: o ponto a meia espessura sob a
/// face, do lado do material, está dentro da chapa (a reta na normal cruza a forma um número ímpar de vezes), e do
/// lado oposto, no furo, não está. A normal que a sonda de `surface_near` dá a um passo para o centro da peça erra
/// para a aresta de um furo no meio da chapa: o passo cai no vazio do furo e a superfície mais perto é a parede dele.
pub fn normal_da_face(corpo: &Shape, pts: &[V3], mid: V3, t: f64) -> Option<V3> {
    let mut nn = [0.; 3];
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        nn[0] += (a[1] - b[1]) * (a[2] + b[2]);
        nn[1] += (a[2] - b[2]) * (a[0] + b[0]);
        nn[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    let n = geo3d::unit(nn)?;
    let im = pts.len() / 2;
    let tan = geo3d::unit(geo3d::sub(pts[(im + 1).min(pts.len() - 1)], pts[im.saturating_sub(1)]))?;
    let h = geo3d::unit(fenkra_kernel::cross(tan, n))?;
    let dentro = |p: V3| corpo.ray_hits(p, n).map(|v| v.iter().filter(|x| **x > 1e-6).count() % 2 == 1).unwrap_or(false);
    for sn in [-1., 1.] {
        let lados: Vec<f64> = [1., -1.]
            .into_iter()
            .filter(|sh| dentro(geo3d::add(geo3d::add(mid, geo3d::mul(h, sh * 0.3 * t)), geo3d::mul(n, sn * 0.5 * t))))
            .collect();
        if lados.len() == 1 {
            // `sn·n` aponta para dentro da chapa a partir da face da aresta; para fora é o contrário
            return Some(geo3d::mul(n, -sn));
        }
    }
    None
}

/// Largura máxima de uma faixa que se afasta da linha fechada `pts` na direção `outs` (unitários, no plano da
/// chapa) sem se cruzar, ou `None` se a faixa de `largura` cabe. Numa aba para dentro de um furo, ou para fora de
/// um contorno côncavo, a borda deslocada passa a menos de `largura` de outro trecho da linha (a faixa se cruza e
/// a varredura da dobra ou a união da faixa planificada falham no kernel sem dizer por quê). A conferência é pelas
/// amostras: o ponto deslocado de uma amostra tem de ficar a `largura` da linha, e a de qualquer outra amostra
/// mais perto que 97 % disso é cruzamento. Devolve, por bisseção, a maior largura que passa.
pub fn largura_que_cabe(pts: &[V3], outs: &[V3], largura: f64) -> Option<f64> {
    let n = pts.len().min(outs.len());
    if n < 4 || largura <= 1e-6 {
        return None;
    }
    // até 400 pontos deslocados (a faixa varia devagar ao longo da linha); todas as amostras como alvo
    let passo = (n / 400).max(1);
    let cruza = |w: f64| {
        (0..n).step_by(passo).any(|i| {
            let q = geo3d::add(pts[i], geo3d::mul(outs[i], w));
            pts[..n].iter().any(|p| geo3d::dist(q, *p) < 0.97 * w - 1e-9)
        })
    };
    if !cruza(largura) {
        return None;
    }
    let (mut lo, mut hi) = (0., largura);
    for _ in 0..14 {
        let m = (lo + hi) / 2.;
        if cruza(m) {
            hi = m;
        } else {
            lo = m;
        }
    }
    Some(lo)
}

/// Volume pela malha (teorema da divergência). A integração do kernel (`mass`) erra até 13 % numa face plana
/// limitada por uma spline longa (a web de uma nervura, medido), e o erro some na malha: a flecha da
/// tesselação é 2e-4 da diagonal (no mínimo 20 µm): o volume sai uns 2 % abaixo do verdadeiro nas paredes curvas,
/// o que basta para as conferências de "a fusão tem volume" (margem de 50 %) e custa uma fração do tempo.
pub fn volume(forma: &Shape) -> f64 {
    let diag = forma.bbox().map(|(a, b)| geo3d::dist(a, b)).unwrap_or(1.);
    volume_com(forma, (diag * 2e-4).max(0.02))
}

/// `volume` com a flecha da malha dada.
pub fn volume_com(forma: &Shape, flecha: f64) -> f64 {
    let Ok(m) = forma.mesh(flecha) else { return forma.mass().volume };
    let p = |i: u32| {
        let k = i as usize * 3;
        [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
    };
    let mut v = 0.;
    for t in m.indices.chunks_exact(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        v += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]);
    }
    v / 6.
}

/// Volume da malha em duas flechas (a de `volume` e uma quarta parte dela), ou `None` se elas discordam em mais de 2 %. Uma forma
/// costurada com a tolerância das arestas acima da flecha pode tesselar certo numa flecha e dar um volume de dezenas de vezes
/// o da peça noutra (medido na fuselagem por loft); só serve o que mede igual nas duas. A flecha grossa (1e-3 da diagonal) fica de
/// fora: ela mesma erra uns 8 % numa nervura de 2 m, costurada ou não.
pub fn volume_estavel(forma: &Shape) -> Option<f64> {
    let diag = forma.bbox().map(|(a, b)| geo3d::dist(a, b)).unwrap_or(1.);
    let vs = [(diag * 2e-4).max(0.02), (diag * 5e-5).max(0.05)].map(|d| volume_com(forma, d));
    let (lo, hi) = vs.iter().fold((f64::MAX, f64::MIN), |a, v| (a.0.min(*v), a.1.max(*v)));
    (lo > 0. && hi - lo < 0.02 * hi).then_some(vs[0])
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn junta_pedacos_de_um_circulo_em_um_laco() {
        let n = 40;
        let pt = |i: usize| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            [10. * a.cos(), 10. * a.sin(), 0.]
        };
        let a: Vec<V3> = (0..=20).map(pt).collect();
        let mut b: Vec<V3> = (20..=40).map(pt).collect();
        b.reverse();
        let r = junta(vec![b, a], 1e-6);
        assert_eq!(r.len(), 1);
        assert!(fechada(&r[0]), "laço aberto: {} pontos", r[0].len());
        assert_eq!(r[0].len(), 41);
    }

    /// Losango fino (pontas agudas) em sentido anti-horário, cada lado em passos de 5 mm.
    fn losango() -> Vec<V3> {
        let cantos = [[0., 0.], [50., -8.], [100., 0.], [50., 8.], [0., 0.]];
        let mut v: Vec<V3> = vec![];
        for w in cantos.windows(2) {
            let n = (geo3d::dist([w[0][0], w[0][1], 0.], [w[1][0], w[1][1], 0.]) / 5.).ceil() as usize;
            for i in 0..n {
                let u = i as f64 / n as f64;
                v.push([w[0][0] + (w[1][0] - w[0][0]) * u, w[0][1] + (w[1][1] - w[0][1]) * u, 0.]);
            }
        }
        v.push(v[0]);
        v
    }

    #[test]
    fn borda_deslocada_leva_o_leque_na_ponta_aguda() {
        let pts = losango();
        let outs: Vec<V3> = (0..pts.len())
            .map(|i| {
                let t = geo3d::unit(geo3d::sub(pts[(i + 1).min(pts.len() - 1)], pts[i.saturating_sub(1)])).unwrap_or([1., 0., 0.]);
                geo3d::unit(crate::geo3d::cross(t, [0., 0., 1.])).unwrap_or([0., -1., 0.])
            })
            .collect();
        let q = desloca_com_juntas(&pts, &outs, [0., 0., 1.], 1., 20.);
        let (xmin, xmax) = q.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
        assert!((xmax - 120.).abs() < 0.5, "a ponta direita leva o leque de 20 mm: x máximo {xmax:.2}");
        assert!((xmin + 20.).abs() < 0.5, "a ponta esquerda também: x mínimo {xmin:.2}");
        assert!(fechada(&q));
        // todos os pontos da borda a pelo menos 19,9 mm da linha (nenhum cruzamento para dentro)
        for p in &q {
            let d = pts.iter().map(|x| geo3d::dist(*p, *x)).fold(f64::MAX, f64::min);
            assert!(d > 19.0, "ponto {p:?} a {d:.2} mm da linha");
        }
    }

    #[test]
    fn faixa_que_nao_cabe_no_furo_acha_a_largura_que_cabe() {
        // círculo de raio 40 (furo): a faixa para dentro de 30 cabe, a de 50 não; cabe até uns 40
        let n = 120;
        let pts: Vec<V3> = (0..=n).map(|i| {
            let a = std::f64::consts::TAU * (i % n) as f64 / n as f64;
            [40. * a.cos(), 40. * a.sin(), 0.]
        }).collect();
        let dentro: Vec<V3> = pts.iter().map(|p| geo3d::mul(*p, -1. / 40.)).collect();
        assert_eq!(largura_que_cabe(&pts, &dentro, 30.), None);
        let cabe = largura_que_cabe(&pts, &dentro, 50.).expect("50 mm não cabe num furo de raio 40");
        assert!(cabe > 30. && cabe < 41., "cabe {cabe:.1}");
        // para fora nunca se cruza
        let fora: Vec<V3> = pts.iter().map(|p| geo3d::mul(*p, 1. / 40.)).collect();
        assert_eq!(largura_que_cabe(&pts, &fora, 200.), None);
    }

    #[test]
    fn cadeia_aberta_nao_vira_laco() {
        let a: Vec<V3> = (0..10).map(|i| [i as f64, 0., 0.]).collect();
        assert!(!fechada(&a));
        assert_eq!(junta(vec![a], 1e-6).len(), 1);
    }
}
