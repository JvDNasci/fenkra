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

/// Volume pela malha (teorema da divergência). A integração do kernel (`mass`) erra até 13 % numa face plana
/// limitada por uma spline longa (a web de uma nervura, medido), e o erro some na malha: a flecha da
/// tesselação é 2e-4 da diagonal (no mínimo 20 µm): o volume sai uns 2 % abaixo do verdadeiro nas paredes curvas,
/// o que basta para as conferências de "a fusão tem volume" (margem de 50 %) e custa uma fração do tempo.
pub fn volume(forma: &Shape) -> f64 {
    let diag = forma.bbox().map(|(a, b)| geo3d::dist(a, b)).unwrap_or(1.);
    let Ok(m) = forma.mesh((diag * 2e-4).max(0.02)) else { return forma.mass().volume };
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

    #[test]
    fn cadeia_aberta_nao_vira_laco() {
        let a: Vec<V3> = (0..10).map(|i| [i as f64, 0., 0.]).collect();
        assert!(!fechada(&a));
        assert_eq!(junta(vec![a], 1e-6).len(), 1);
    }
}
