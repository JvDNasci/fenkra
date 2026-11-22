// SPDX-License-Identifier: LGPL-2.1-or-later
//! Encaixe de tubo com corte normal à parede, a boca de lobo (B574).
//!
//! # O problema
//!
//! Quando um tubo encosta noutro em ângulo, cortar a ponta por booleana (subtrair o outro tubo)
//! deixa a face de corte colada na superfície do vizinho. Onde o ângulo é agudo, essa face corta a
//! parede em diagonal e a borda vira uma ponta de faca: a solda derrete a quina e o encaixe fica
//! cheio de vão no outro lado.
//!
//! # Como o corte real funciona
//!
//! O plasma e o traçado à mão cortam **normal à parede**: em cada ponto do contorno do tubo a face
//! de corte contém a normal da parede, então a borda mantém a espessura inteira e é suave. A face
//! não é um plano: é varrida pelas normais da parede ao longo de uma curva `λ(s)` (a distância, na
//! direção do eixo, de cada ponto do contorno até o outro tubo). Essa curva vem de `st_encaixe`: a
//! pele externa e a pele interna do tubo cortado intersectam o tubo que recebe e dão duas curvas;
//! a que vale (a mais alta, a externa, a interna ou a média) é a referência escolhida.
//!
//! # Como o kernel monta
//!
//! * Tubo redondo: uma superfície B-spline passa pela grade (raio × ângulo) com `λ(θ)` constante ao
//!   longo do raio, e divide o tubo (`clip_until`); fica o pedaço do lado do corpo do membro. A
//!   face de corte é uma só, lisa.
//! * Tubo retangular: cada face plana tem o seu perfil `λ(c)` (c é a coordenada ao longo da face);
//!   o perfil é extrudado pela normal da face e limitado à cunha que vai do canto externo à
//!   diagonal, a mesma meia-esquadria que o tubo tem nos cantos. As quatro peças se encontram
//!   na diagonal com o mesmo `λ`, então a borda é contínua.
//!
//! Barras maciças ficam de fora: no centro não há parede e a normal não existe.

use crate::geo3d;
use crate::st_encaixe::{CorteEncaixe, Membro, Opcoes, Parede, Secao};
use fenkra_core::{FitCut, FitFace, FitOptions, FitWall};
use fenkra_kernel::{cross, Bool, Frame, Loop2, Seg2, Shape};

type V3 = [f64; 3];

/// Folga mínima admissível de espessura na borda de corte, em mm: abaixo dela a parede é fina demais
/// para o plasma segurar a borda.
pub const ESPESSURA_MINIMA: f64 = 1.0;
/// Interferência (invasão do tubo que recebe) tolerada, em mm.
pub const TOLERANCIA_INTERFERENCIA: f64 = 0.05;

/// Traduz as opções gravadas no documento para as do cálculo da curva. `base` traz o orçamento de
/// vão e a folga de solda do canto.
pub fn opcoes_do_fit(fit: &FitOptions, base: Opcoes) -> Opcoes {
    let modo = match fit.wall {
        FitWall::Auto => crate::st_encaixe::ModoParede::Automatico,
        FitWall::Outer => crate::st_encaixe::ModoParede::Externa,
        FitWall::Inner => crate::st_encaixe::ModoParede::Interna,
        FitWall::Middle => crate::st_encaixe::ModoParede::Media,
    };
    Opcoes { modo, perpendicular: true, face: fit.face, normal: fit.cut == FitCut::Normal, ..base }
}

/// Resultado da conferência de um encaixe: vão, interferência e espessura da borda.
#[derive(Clone, Debug, PartialEq)]
pub struct Validacao {
    /// maior vão de raiz entre o tubo cortado e o que recebe, na normal do que recebe
    pub vao_max: f64,
    /// maior invasão do tubo cortado na parede do que recebe
    pub interferencia_max: f64,
    /// espessura da parede na borda de corte (a parede inteira no corte normal)
    pub espessura_borda: f64,
    /// menor ângulo da ponta da parede, em graus (90 = borda reta, perto de 0 = ponta de faca)
    pub angulo_ponta_min: f64,
    /// pontos da curva com vão acima da tolerância
    pub pontos_acima: usize,
    pub avisos: Vec<String>,
}

impl Validacao {
    pub fn ok(&self) -> bool {
        self.avisos.is_empty()
    }
}

/// Vão de raiz que o encaixe admite sem chanfro: o orçamento da curva (1,5 mm por padrão) ou, se for
/// maior, a espessura da parede do tubo cortado, que é o que a solda de ângulo fecha sem preparo.
/// Vale como hipótese de trabalho, a conferir em peça real.
pub fn tolerancia_de_vao(secao: Secao, orcamento: f64) -> f64 {
    orcamento.max(secao.parede())
}

/// Confere a curva de corte: vão, interferência e espessura. `tol_vao` é o orçamento de vão em mm.
pub fn validar(c: &CorteEncaixe, tol_vao: f64) -> Validacao {
    let t = c.secao.parede();
    let normal = c.opc.normal && c.opc.perpendicular;
    let vao_max = c.folga_max();
    let interferencia_max = c.invasao_max();
    let acima = c.pontos.iter().filter(|p| p.folga_raiz > tol_vao + 1e-9).count();
    // sem corte normal a face segue a superfície do vizinho: a borda é uma cunha cujo ângulo vem da
    // diferença de alcance entre as duas peles
    let angulo_ponta_min = if normal {
        90.
    } else {
        c.pontos.iter().filter(|p| p.parede != Parede::Livre).map(|p| t.atan2((p.lambda_ext - p.lambda_int).abs()).to_degrees()).fold(90., f64::min)
    };
    // sem corte normal, a espessura que sobra na ponta é a da cunha: t · sen(ângulo da ponta)
    let espessura_borda = if normal { t } else { t * angulo_ponta_min.to_radians().sin() };
    let mut avisos = vec![];
    if t > 1e-9 && t < ESPESSURA_MINIMA {
        avisos.push(format!("parede de {t:.2} mm: a borda de corte fica fina demais (mínimo {ESPESSURA_MINIMA:.1} mm)"));
    }
    if !normal && t > 1e-9 && angulo_ponta_min < 30. {
        avisos.push(format!("ponta fina na borda do corte ({angulo_ponta_min:.0}°): use o corte normal à parede"));
    }
    if interferencia_max > TOLERANCIA_INTERFERENCIA {
        avisos.push(format!("interferência de {interferencia_max:.2} mm com o tubo que recebe: troque a referência para o modo automático"));
    }
    if acima > 0 {
        avisos.push(format!("vão de raiz de até {vao_max:.2} mm acima da tolerância de {tol_vao:.2} mm em {acima} pontos: a solda precisa de chanfro"));
    }
    Validacao { vao_max, interferencia_max, espessura_borda, angulo_ponta_min, pontos_acima: acima, avisos }
}

/// `Membro` do trecho `g` com a ponta no nó `no`, apontando para dentro dele em `para_dentro`
/// (dimensões nominais: o sólido do kernel também as usa).
pub(crate) fn membro_nominal(secao: Secao, x: V3, y: V3, shift: [f64; 2], comprimento: f64, no: V3, para_dentro: V3) -> Membro {
    Membro { secao, ponto: no, x, y, para_dentro, centro: [-shift[0], -shift[1]], comprimento }
}

/// Corta a ponta de `sh` (o sólido do membro `m`) pela curva normal à parede de `c`, ficando com o
/// lado que contém `manter`. `sobra` é quanto o sólido passa do nó para o lado de fora.
pub fn aparar(sh: &Shape, m: &Membro, c: &CorteEncaixe, manter: V3, sobra: f64) -> Result<Shape, String> {
    let u = geo3d::unit(m.para_dentro).ok_or("membro sem direção")?;
    let centro = geo3d::add(m.ponto, geo3d::add(geo3d::mul(m.x, m.centro[0]), geo3d::mul(m.y, m.centro[1])));
    match m.secao {
        Secao::Redonda { d, t } => {
            if t < 1e-9 {
                return Err("barra maciça: o corte normal à parede precisa de parede (use o corte booleano)".into());
            }
            aparar_redondo(sh, c, centro, m, u, d / 2., t, manter)
        }
        Secao::Retangular { b, h, t } => {
            if t < 1e-9 {
                return Err("barra maciça: o corte normal à parede precisa de parede (use o corte booleano)".into());
            }
            aparar_retangular(sh, c, centro, m, u, b / 2., h / 2., t, sobra)
        }
    }
}

fn aparar_redondo(sh: &Shape, c: &CorteEncaixe, centro: V3, m: &Membro, u: V3, ro: f64, t: f64, manter: V3) -> Result<Shape, String> {
    let ri = (ro - t).max(0.);
    // a superfície só precisa atravessar a parede, um pouco além das duas peles
    let (r0, r1) = ((ri - 1.).max(ri * 0.5).max(0.05), ro + 1.);
    let n_r = 5;
    let ponto = |r: f64, th: f64, lam: f64| geo3d::add(geo3d::add(centro, geo3d::add(geo3d::mul(m.x, r * th.cos()), geo3d::mul(m.y, r * th.sin()))), geo3d::mul(u, lam));
    let mut linhas: Vec<Vec<V3>> = vec![];
    for k in 0..n_r {
        let r = r0 + (r1 - r0) * k as f64 / (n_r - 1) as f64;
        let mut l: Vec<V3> = c.pontos.iter().map(|p| ponto(r, p.s / ro, p.lambda)).collect();
        // fecha o círculo: o último ponto é o primeiro
        l.push(l[0]);
        linhas.push(l);
    }
    let sup = Shape::grid_surface(&linhas).map_err(|e| e.0)?;
    sh.clip_until(&sup, manter, false).map_err(|e| e.0)
}

/// Os quatro lados do tubo retangular: normal para fora, tangente (sentido anti-horário) e o
/// afastamento da face em relação ao centro.
fn lados(hb: f64, hh: f64) -> [([f64; 2], [f64; 2], f64, f64); 4] {
    // (n_out, e_c, afastamento da face, meio comprimento da face)
    [([1., 0.], [0., 1.], hb, hh), ([0., 1.], [-1., 0.], hh, hb), ([-1., 0.], [0., -1.], hb, hh), ([0., -1.], [1., 0.], hh, hb)]
}

fn aparar_retangular(sh: &Shape, c: &CorteEncaixe, centro: V3, m: &Membro, u: V3, hb: f64, hh: f64, t: f64, sobra: f64) -> Result<Shape, String> {
    let em3 = |p: [f64; 2], lam: f64| geo3d::add(geo3d::add(centro, geo3d::add(geo3d::mul(m.x, p[0]), geo3d::mul(m.y, p[1]))), geo3d::mul(u, lam));
    let lam_max = c.pontos.iter().map(|p| p.lambda).fold(0., f64::max);
    let fundo = -(sobra + 5.);
    let folga = 1.;
    let mut out = sh.clone();
    for (n_out, e_c, f, meio) in lados(hb, hh) {
        // amostras da face: pontos da pele externa sobre ela, em ordem de c
        let mut perfil: Vec<[f64; 2]> = vec![];
        for p in &c.pontos {
            // a posição 2D da pele externa vem do 3D guardado na curva
            let q = geo3d::sub(p.p3, geo3d::mul(u, p.lambda));
            let r = geo3d::sub(q, centro);
            let (px, py) = (geo3d::dot(r, m.x), geo3d::dot(r, m.y));
            if (px * n_out[0] + py * n_out[1] - f).abs() < 1e-6 {
                perfil.push([px * e_c[0] + py * e_c[1], p.lambda]);
            }
        }
        // os dois cantos da face: o canto final é o primeiro ponto da face seguinte
        for sinal in [-1., 1.] {
            let cc = [n_out[0] * f + e_c[0] * meio * sinal, n_out[1] * f + e_c[1] * meio * sinal];
            if !perfil.iter().any(|q| (q[0] - meio * sinal).abs() < 1e-6) {
                let p = c.pontos.iter().min_by(|a, b| {
                    let d = |x: &crate::st_encaixe::PontoCorte| {
                        let r = geo3d::sub(geo3d::sub(x.p3, geo3d::mul(u, x.lambda)), centro);
                        ((geo3d::dot(r, m.x) - cc[0]).powi(2) + (geo3d::dot(r, m.y) - cc[1]).powi(2)).sqrt()
                    };
                    d(a).total_cmp(&d(b))
                });
                if let Some(p) = p {
                    perfil.push([meio * sinal, p.lambda]);
                }
            }
        }
        perfil.sort_by(|a, b| a[0].total_cmp(&b[0]));
        perfil.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-7);
        if perfil.len() < 2 {
            return Err("encaixe retangular sem amostras na face".into());
        }
        let perfil = simplificar(&perfil, 0.004);
        // quadro do perfil: x = ao longo da face, y = ±eixo do tubo, normal = para dentro da parede
        let n_in3 = geo3d::add(geo3d::mul(m.x, -n_out[0]), geo3d::mul(m.y, -n_out[1]));
        let e3 = geo3d::add(geo3d::mul(m.x, e_c[0]), geo3d::mul(m.y, e_c[1]));
        let origem = em3([n_out[0] * (f + folga), n_out[1] * (f + folga)], 0.);
        let quadro = Frame { origin: origem, x: e3, normal: n_in3 };
        let sg = if geo3d::dot(cross(n_in3, e3), u) >= 0. { 1. } else { -1. };
        let ponta = |v: [f64; 2]| [v[0], sg * v[1]];
        let a = perfil[0];
        let z = perfil[perfil.len() - 1];
        let mut laco: Loop2 = vec![];
        let (c0, c1) = (a[0] - 2. * folga, z[0] + 2. * folga);
        let baixo = sg * fundo;
        laco.push(Seg2::Line([c0, baixo], [c1, baixo]));
        laco.push(Seg2::Line([c1, baixo], ponta([c1, z[1]])));
        laco.push(Seg2::Line(ponta([c1, z[1]]), ponta(z)));
        // o perfil de cima, da direita para a esquerda
        let pts: Vec<[f64; 2]> = perfil.iter().rev().map(|q| ponta(*q)).collect();
        if pts.len() >= 4 {
            laco.push(Seg2::Spline(pts));
        } else {
            for w in pts.windows(2) {
                laco.push(Seg2::Line(w[0], w[1]));
            }
        }
        laco.push(Seg2::Line(ponta(a), ponta([c0, a[1]])));
        laco.push(Seg2::Line(ponta([c0, a[1]]), [c0, baixo]));
        let face = Shape::face(&quadro, &[laco]).map_err(|e| e.0)?;
        let prisma = face.extrude(n_in3, t + 2. * folga + 1., 0.).map_err(|e| e.0)?;
        // cunha da face: do canto externo até a diagonal, a meia-esquadria do tubo nos cantos
        let prof = t + folga;
        let l_in = (meio - prof).max(0.05);
        let cunha_2d = [[-(meio + folga), -folga], [meio + folga, -folga], [l_in, prof], [-l_in, prof]];
        let pos = |cw: [f64; 2]| em3([n_out[0] * f + e_c[0] * cw[0] - n_out[0] * cw[1], n_out[1] * f + e_c[1] * cw[0] - n_out[1] * cw[1]], fundo);
        let pts: Vec<V3> = cunha_2d.iter().map(|q| pos(*q)).collect();
        let cunha = Shape::poly_prism(&pts, geo3d::mul(u, lam_max - fundo + 2. * folga)).map_err(|e| e.0)?;
        let peca = prisma.boolean(&cunha, Bool::Common).map_err(|e| e.0)?;
        out = out.boolean(&peca, Bool::Cut).map_err(|e| e.0)?;
    }
    Ok(out)
}

/// Douglas–Peucker: tira os pontos que ficam a menos de `tol` da reta entre os vizinhos mantidos.
fn simplificar(p: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let (a, b) = (p[0], p[p.len() - 1]);
    let (mut melhor, mut d_max) = (0, 0.);
    for (i, q) in p.iter().enumerate().skip(1).take(p.len() - 2) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = dx.hypot(dy).max(1e-12);
        let d = ((q[0] - a[0]) * dy - (q[1] - a[1]) * dx).abs() / l;
        if d > d_max {
            d_max = d;
            melhor = i;
        }
    }
    if d_max <= tol {
        return vec![a, b];
    }
    let mut esq = simplificar(&p[..=melhor], tol);
    let dir = simplificar(&p[melhor..], tol);
    esq.pop();
    esq.extend(dir);
    esq
}

/// O encaixe pede o corte normal à parede?
pub fn e_normal(fit: &FitOptions) -> bool {
    fit.cut == FitCut::Normal
}

/// Texto curto da face de referência, para avisos.
pub fn nome_da_face(f: FitFace) -> &'static str {
    match f {
        FitFace::Outer => "face externa",
        FitFace::Middle => "meio da parede",
        FitFace::Inner => "face interna",
    }
}
