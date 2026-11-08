// SPDX-License-Identifier: LGPL-2.1-or-later
//! Análise linear de pórtico espacial 1D (B114): método da rigidez direta com barras de
//! Euler-Bernoulli, 6 graus de liberdade por nó (3 deslocamentos e 3 giros).
//!
//! Como funciona: cada barra tem uma matriz de rigidez 12×12 no referencial dela (x ao longo da
//! barra, y e z nos eixos principais da seção). A matriz é girada para o global e somada na matriz da
//! estrutura. Os apoios tiram os graus de liberdade presos; o sistema que sobra é resolvido por
//! eliminação de Gauss com pivô. Carga distribuída vira forças e momentos de engaste perfeito nas
//! pontas. Depois, os esforços nas pontas de cada barra saem de `k·u − f_engaste` e os diagramas de
//! N, V, T e M ao longo da barra saem do equilíbrio do trecho entre a ponta inicial e o corte.
//!
//! Convenção dos esforços internos: é o que a parte da barra depois do corte faz na parte de antes,
//! no referencial local. N > 0 é tração; com carga para baixo numa viga biapoiada, Mz > 0 no meio
//! (tração embaixo). A flecha é a deflexão da linha elástica no referencial local (cúbica de Hermite
//! mais a parcela da carga distribuída).
//!
//! Limites: linear, pequenos deslocamentos, sem cisalhamento de Timoshenko, sem flambagem e sem
//! não linearidade de apoio. Unidades coerentes à escolha de quem chama (o Fenkra usa N, mm e MPa).

use crate::geo3d::{self, V3};

/// Nó do pórtico.
#[derive(Clone, Debug, PartialEq)]
pub struct No {
    pub p: V3,
    /// graus presos: ux, uy, uz, rx, ry, rz (verdadeiro = preso)
    pub apoio: [bool; 6],
    /// carga no nó: Fx, Fy, Fz, Mx, My, Mz (global)
    pub carga: [f64; 6],
}

impl No {
    pub fn livre(p: V3) -> Self {
        No { p, apoio: [false; 6], carga: [0.; 6] }
    }
    pub fn engaste(p: V3) -> Self {
        No { p, apoio: [true; 6], carga: [0.; 6] }
    }
    /// Apoio de segundo gênero: deslocamentos presos, giros livres.
    pub fn rotula(p: V3) -> Self {
        No { p, apoio: [true, true, true, false, false, false], carga: [0.; 6] }
    }
}

/// Propriedades da seção e do material de uma barra.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Secao {
    /// módulo de elasticidade
    pub e: f64,
    /// módulo de cisalhamento
    pub g: f64,
    pub area: f64,
    /// momento de inércia em torno do eixo local y (flexão no plano x-z)
    pub iy: f64,
    /// momento de inércia em torno do eixo local z (flexão no plano x-y)
    pub iz: f64,
    /// constante de torção
    pub j: f64,
}

/// Barra entre dois nós.
#[derive(Clone, Debug, PartialEq)]
pub struct Barra {
    pub i: usize,
    pub j: usize,
    pub secao: Secao,
    /// vetor que define o eixo local y (projetado no plano normal à barra); `None` = global Z, ou
    /// global Y quando a barra é vertical
    pub ref_y: Option<V3>,
    /// carga distribuída uniforme no referencial global (força por comprimento)
    pub q: V3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Portico {
    pub nos: Vec<No>,
    pub barras: Vec<Barra>,
}

/// Esforços internos num ponto da barra, no referencial local.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Esforcos {
    /// posição a partir da ponta inicial
    pub x: f64,
    pub n: f64,
    pub vy: f64,
    pub vz: f64,
    pub t: f64,
    pub my: f64,
    pub mz: f64,
    /// deflexão local (v ao longo de y, w ao longo de z)
    pub v: f64,
    pub w: f64,
}

/// Resultado de uma barra: esforços de ponta (forças do nó na barra, local) e diagramas.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoBarra {
    pub comprimento: f64,
    /// eixos locais x, y, z no global
    pub eixos: [V3; 3],
    /// forças que os nós fazem na barra, local: [Fx, Fy, Fz, Mx, My, Mz] em i e depois em j
    pub pontas: [f64; 12],
    pub diagrama: Vec<Esforcos>,
}

impl ResultadoBarra {
    /// Maior |valor| de um esforço ao longo do diagrama.
    pub fn maximo(&self, f: impl Fn(&Esforcos) -> f64) -> f64 {
        self.diagrama.iter().map(|e| f(e).abs()).fold(0., f64::max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resultado {
    /// deslocamentos de cada nó (global): ux, uy, uz, rx, ry, rz
    pub deslocamentos: Vec<[f64; 6]>,
    /// reações nos graus presos (global); zero nos livres
    pub reacoes: Vec<[f64; 6]>,
    pub barras: Vec<ResultadoBarra>,
}

/// Eixos locais da barra: x ao longo dela, y pelo vetor de referência, z = x × y.
pub fn eixos_locais(a: V3, b: V3, ref_y: Option<V3>) -> Result<[V3; 3], String> {
    let x = geo3d::unit(geo3d::sub(b, a)).ok_or("barra de comprimento zero")?;
    let r = ref_y.unwrap_or(if x[2].abs() > 0.999 { [0., 1., 0.] } else { [0., 0., 1.] });
    let y = geo3d::unit(geo3d::sub(r, geo3d::mul(x, geo3d::dot(r, x)))).ok_or("vetor de referência paralelo à barra")?;
    Ok([x, y, cross(x, y)])
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Rigidez 12×12 no referencial local.
fn rigidez_local(l: f64, s: &Secao) -> [[f64; 12]; 12] {
    let mut k = [[0.; 12]; 12];
    let mut put = |dofs: &[usize], m: &[&[f64]]| {
        for (a, ra) in dofs.iter().zip(m) {
            for (b, v) in dofs.iter().zip(ra.iter()) {
                k[*a][*b] += v;
            }
        }
    };
    let ea = s.e * s.area / l;
    put(&[0, 6], &[&[ea, -ea], &[-ea, ea]]);
    let gj = s.g * s.j / l;
    put(&[3, 9], &[&[gj, -gj], &[-gj, gj]]);
    let (l2, l3) = (l * l, l * l * l);
    let c = s.e * s.iz / l3;
    put(&[1, 5, 7, 11], &[&[12. * c, 6. * l * c, -12. * c, 6. * l * c], &[6. * l * c, 4. * l2 * c, -6. * l * c, 2. * l2 * c], &[-12. * c, -6. * l * c, 12. * c, -6. * l * c], &[6. * l * c, 2. * l2 * c, -6. * l * c, 4. * l2 * c]]);
    let c = s.e * s.iy / l3;
    put(&[2, 4, 8, 10], &[&[12. * c, -6. * l * c, -12. * c, -6. * l * c], &[-6. * l * c, 4. * l2 * c, 6. * l * c, 2. * l2 * c], &[-12. * c, 6. * l * c, 12. * c, 6. * l * c], &[-6. * l * c, 2. * l2 * c, 6. * l * c, 4. * l2 * c]]);
    k
}

/// Forças de engaste perfeito (as que os nós fazem na barra) para carga uniforme local `q`.
fn engaste_local(l: f64, q: V3) -> [f64; 12] {
    let mut f = [0.; 12];
    f[0] = q[0] * l / 2.;
    f[6] = q[0] * l / 2.;
    f[1] = q[1] * l / 2.;
    f[7] = q[1] * l / 2.;
    f[5] = q[1] * l * l / 12.;
    f[11] = -q[1] * l * l / 12.;
    f[2] = q[2] * l / 2.;
    f[8] = q[2] * l / 2.;
    f[4] = -q[2] * l * l / 12.;
    f[10] = q[2] * l * l / 12.;
    // são as cargas equivalentes nos nós; o nó faz na barra o oposto
    f.map(|v| -v)
}

fn para_local(e: &[V3; 3], v: V3) -> V3 {
    [geo3d::dot(e[0], v), geo3d::dot(e[1], v), geo3d::dot(e[2], v)]
}

fn para_global(e: &[V3; 3], v: V3) -> V3 {
    [0, 1, 2].map(|k| e[0][k] * v[0] + e[1][k] * v[1] + e[2][k] * v[2])
}

/// Resolve `a·x = b` por eliminação de Gauss com pivô parcial. `None` = sistema singular.
fn resolver(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    let escala = a.iter().flat_map(|r| r.iter()).fold(0f64, |m, v| m.max(v.abs())).max(1e-300);
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))?;
        if a[p][c].abs() < escala * 1e-12 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..n {
            let f = a[r][c] / a[c][c];
            if f == 0. {
                continue;
            }
            for k in c..n {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|k| a[r][k] * x[k]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    Some(x)
}

/// Amostras dos diagramas por barra (inclui as duas pontas).
pub const AMOSTRAS: usize = 21;

/// Analisa o pórtico.
pub fn analisar(p: &Portico) -> Result<Resultado, String> {
    let n = p.nos.len();
    if n == 0 || p.barras.is_empty() {
        return Err("pórtico sem nós ou sem barras".into());
    }
    let ndof = 6 * n;
    let mut k = vec![vec![0.; ndof]; ndof];
    let mut f = vec![0.; ndof];
    for (i, no) in p.nos.iter().enumerate() {
        for d in 0..6 {
            f[6 * i + d] += no.carga[d];
        }
    }
    struct Pre {
        l: f64,
        e: [V3; 3],
        kl: [[f64; 12]; 12],
        fe: [f64; 12],
        ql: V3,
    }
    let mut pre = vec![];
    for (bi, b) in p.barras.iter().enumerate() {
        if b.i >= n || b.j >= n || b.i == b.j {
            return Err(format!("barra {}: nós inválidos", bi + 1));
        }
        let (a, c) = (p.nos[b.i].p, p.nos[b.j].p);
        let l = geo3d::dist(a, c);
        let e = eixos_locais(a, c, b.ref_y).map_err(|m| format!("barra {}: {m}", bi + 1))?;
        let s = &b.secao;
        if !(s.e > 0. && s.area > 0. && s.iy > 0. && s.iz > 0. && s.j > 0. && s.g > 0.) {
            return Err(format!("barra {}: seção ou material sem rigidez", bi + 1));
        }
        let kl = rigidez_local(l, s);
        let ql = para_local(&e, b.q);
        let fe = engaste_local(l, ql);
        // matriz de rotação 12×12 (blocos 3×3 com as linhas = eixos locais): K = Tᵀ·k·T
        let t = |r: usize, c: usize| -> f64 {
            if r / 3 != c / 3 {
                0.
            } else {
                e[r % 3][c % 3]
            }
        };
        let dofs: Vec<usize> = (0..6).map(|d| 6 * b.i + d).chain((0..6).map(|d| 6 * b.j + d)).collect();
        // kt = k·T
        let mut kt = [[0.; 12]; 12];
        for r in 0..12 {
            for c in 0..12 {
                kt[r][c] = (0..12).filter(|m| m / 3 == c / 3).map(|m| kl[r][m] * t(m, c)).sum();
            }
        }
        for r in 0..12 {
            for c in 0..12 {
                let v: f64 = (0..12).filter(|m| m / 3 == r / 3).map(|m| t(m, r) * kt[m][c]).sum();
                k[dofs[r]][dofs[c]] += v;
            }
        }
        // a carga equivalente nos nós é o oposto do que o nó faz na barra no engaste
        for blk in 0..4 {
            let v = para_global(&e, [fe[3 * blk], fe[3 * blk + 1], fe[3 * blk + 2]]);
            for d in 0..3 {
                f[dofs[3 * blk + d]] -= v[d];
            }
        }
        pre.push(Pre { l, e, kl, fe, ql });
    }
    let livres: Vec<usize> = (0..ndof).filter(|d| !p.nos[d / 6].apoio[d % 6]).collect();
    let a: Vec<Vec<f64>> = livres.iter().map(|r| livres.iter().map(|c| k[*r][*c]).collect()).collect();
    let bb: Vec<f64> = livres.iter().map(|r| f[*r]).collect();
    let x = if livres.is_empty() { vec![] } else { resolver(a, bb).ok_or("estrutura instável: há um mecanismo (apoios ou ligações insuficientes)")? };
    let mut u = vec![0.; ndof];
    for (v, d) in x.iter().zip(&livres) {
        u[*d] = *v;
    }
    let mut reacoes = vec![[0.; 6]; n];
    for d in 0..ndof {
        if p.nos[d / 6].apoio[d % 6] {
            reacoes[d / 6][d % 6] = (0..ndof).map(|c| k[d][c] * u[c]).sum::<f64>() - f[d];
        }
    }
    let mut barras = vec![];
    for (b, pr) in p.barras.iter().zip(&pre) {
        let ug = |no: usize, o: usize| [u[6 * no + o], u[6 * no + o + 1], u[6 * no + o + 2]];
        let mut ul = [0.; 12];
        for (blk, (no, o)) in [(b.i, 0), (b.i, 3), (b.j, 0), (b.j, 3)].into_iter().enumerate() {
            let v = para_local(&pr.e, ug(no, o));
            ul[3 * blk..3 * blk + 3].copy_from_slice(&v);
        }
        let mut pontas = [0.; 12];
        for r in 0..12 {
            pontas[r] = (0..12).map(|c| pr.kl[r][c] * ul[c]).sum::<f64>() + pr.fe[r];
        }
        let (l, q) = (pr.l, pr.ql);
        let fi: V3 = [pontas[0], pontas[1], pontas[2]];
        let mi: V3 = [pontas[3], pontas[4], pontas[5]];
        let s = &b.secao;
        let mut diagrama = vec![];
        for k in 0..AMOSTRAS {
            let xs = l * k as f64 / (AMOSTRAS - 1) as f64;
            // equilíbrio do trecho [0, xs]: forças da ponta i, carga distribuída e o corte
            let rf = geo3d::mul(geo3d::add(fi, geo3d::mul(q, xs)), -1.);
            let braco_i: V3 = [-xs, 0., 0.];
            let braco_q: V3 = [-xs / 2., 0., 0.];
            let rm = geo3d::mul(geo3d::add(geo3d::add(mi, cross(braco_i, fi)), cross(braco_q, geo3d::mul(q, xs))), -1.);
            let xi = xs / l;
            let (n1, n2, n3, n4) = (1. - 3. * xi * xi + 2. * xi.powi(3), l * (xi - 2. * xi * xi + xi.powi(3)), 3. * xi * xi - 2. * xi.powi(3), l * (xi.powi(3) - xi * xi));
            let bolha = xs * xs * (l - xs) * (l - xs) / 24.;
            let v = n1 * ul[1] + n2 * ul[5] + n3 * ul[7] + n4 * ul[11] + q[1] * bolha / (s.e * s.iz);
            let w = n1 * ul[2] - n2 * ul[4] + n3 * ul[8] - n4 * ul[10] + q[2] * bolha / (s.e * s.iy);
            // `rf` e `rm` agem na face +x do trecho: N > 0 é tração; o sinal de My acompanha o de Mz
            // (positivo quando a carga no sentido −local traciona o lado +local)
            diagrama.push(Esforcos { x: xs, n: rf[0], vy: rf[1], vz: rf[2], t: rm[0], my: -rm[1], mz: rm[2], v, w });
        }
        barras.push(ResultadoBarra { comprimento: l, eixos: pr.e, pontas, diagrama });
    }
    let deslocamentos = (0..n).map(|i| [0, 1, 2, 3, 4, 5].map(|d| u[6 * i + d])).collect();
    Ok(Resultado { deslocamentos, reacoes, barras })
}

/// Propriedades de seções comuns, a partir das cotas em mm (cantos vivos, sem raios de
/// concordância). Devolve (área, Iy, Iz, J): `iz` é a inércia da flexão que usa a altura `h` (o eixo
/// local y da barra segue a altura) e `iy` a que usa a largura `b`.
pub mod secoes {
    use std::f64::consts::PI;

    /// Tubo redondo de diâmetro `d` e parede `t` (t = 0: barra maciça).
    pub fn tubo_redondo(d: f64, t: f64) -> (f64, f64, f64, f64) {
        let di = if t <= 0. || 2. * t >= d { 0. } else { d - 2. * t };
        let a = PI / 4. * (d * d - di * di);
        let i = PI / 64. * (d.powi(4) - di.powi(4));
        (a, i, i, 2. * i)
    }

    /// Tubo retangular `b` × `h` (largura × altura), parede `t` (t = 0: barra maciça). J de parede
    /// fina (Bredt) para tubo e a aproximação de Saint-Venant para a barra.
    pub fn tubo_retangular(b: f64, h: f64, t: f64) -> (f64, f64, f64, f64) {
        if t <= 0. || 2. * t >= b.min(h) {
            let (a, c) = (b.max(h), b.min(h));
            let j = a * c.powi(3) * (1. / 3. - 0.21 * c / a * (1. - c.powi(4) / (12. * a.powi(4))));
            return (b * h, h * b.powi(3) / 12., b * h.powi(3) / 12., j);
        }
        let (bi, hi) = (b - 2. * t, h - 2. * t);
        let a = b * h - bi * hi;
        let iy = (h * b.powi(3) - hi * bi.powi(3)) / 12.;
        let iz = (b * h.powi(3) - bi * hi.powi(3)) / 12.;
        let (bm, hm) = (b - t, h - t);
        let j = 2. * t * bm * bm * hm * hm / (bm + hm);
        (a, iy, iz, j)
    }

    /// Perfil I duplamente simétrico: altura `h`, mesa `b`, alma `tw`, mesa `tf`.
    pub fn perfil_i(h: f64, b: f64, tw: f64, tf: f64) -> (f64, f64, f64, f64) {
        let hw = h - 2. * tf;
        let a = 2. * b * tf + hw * tw;
        let iz = (b * h.powi(3) - (b - tw) * hw.powi(3)) / 12.;
        let iy = (2. * tf * b.powi(3) + hw * tw.powi(3)) / 12.;
        let j = (2. * b * tf.powi(3) + (h - tf) * tw.powi(3)) / 3.;
        (a, iy, iz, j)
    }
}

/// Aço dos membros quando o modelo nasce do documento (MPa).
pub const E_ACO: f64 = 200_000.;
pub const G_ACO: f64 = 77_000.;

/// Seção de pórtico de um membro estrutural, pelas cotas dele (mm, MPa). Perfis abertos fora do I
/// (U, T, cantoneira) ainda não entram: a torção e o eixo principal deles pedem outro cálculo.
pub fn secao_do_membro(kind: fenkra_core::SectionKind, d: &[f64]) -> Result<Secao, String> {
    use fenkra_core::SectionKind::*;
    let g = |i: usize| d.get(i).copied().ok_or_else(|| "cota da seção faltando".to_string());
    let (area, iy, iz, j) = match kind {
        TuboRedondo => secoes::tubo_redondo(g(0)?, g(1)?),
        BarraRedonda => secoes::tubo_redondo(g(0)?, 0.),
        TuboQuadrado => secoes::tubo_retangular(g(0)?, g(0)?, g(1)?),
        TuboRetangular => secoes::tubo_retangular(g(0)?, g(1)?, g(2)?),
        BarraQuadrada => secoes::tubo_retangular(g(0)?, g(0)?, 0.),
        BarraChata => secoes::tubo_retangular(g(0)?, g(1)?, 0.),
        PerfilI => secoes::perfil_i(g(0)?, g(1)?, g(2)?, g(3)?),
        _ => return Err(format!("seção {} ainda não entra na análise de pórtico", crate::structural::kind_label(kind))),
    };
    Ok(Secao { e: E_ACO, g: G_ACO, area, iy, iz, j })
}

/// Pórtico montado a partir dos membros retos do documento: as pontas viram nós (pontas a menos de
/// 0,01 mm são o mesmo nó) e a linha que tem um nó no meio (junta em T) é dividida nele. Sem apoios
/// nem cargas: quem chama escolhe. Devolve também, por barra, o (comando, linha) de origem.
pub fn modelo_da_estrutura(doc: &fenkra_core::Document) -> Result<(Portico, Vec<(fenkra_core::Id, usize)>), String> {
    use fenkra_core::FeatureKind;
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let mut linhas = vec![];
    for (_, f) in doc.active_features() {
        let FeatureKind::StructMember(m) = &f.kind else { continue };
        let dims: Vec<f64> = m.dims.iter().map(|x| fenkra_core::expr::eval_str(x.trim(), &vars).map_err(|e| format!("{}: {e}", f.name))).collect::<Result<_, _>>()?;
        let sec = secao_do_membro(m.kind, &dims).map_err(|e| format!("{}: {e}", f.name))?;
        for fr in crate::st_frames::member_frames(doc, f.id) {
            if !fr.straight {
                return Err(format!("{}: linha {} não é reta (curva ainda não entra no pórtico)", f.name, fr.line + 1));
            }
            linhas.push((f.id, fr.line, fr.start, fr.end, fr.y, sec));
        }
    }
    if linhas.is_empty() {
        return Err("não há membros estruturais".into());
    }
    let mut nos: Vec<No> = vec![];
    let no_de = |p: V3, nos: &mut Vec<No>| -> usize {
        match nos.iter().position(|n| geo3d::dist(n.p, p) < 0.01) {
            Some(k) => k,
            None => {
                nos.push(No::livre(p));
                nos.len() - 1
            }
        }
    };
    for l in &linhas {
        no_de(l.2, &mut nos);
        no_de(l.3, &mut nos);
    }
    let mut barras = vec![];
    let mut origem = vec![];
    for (id, linha, a, b, y, sec) in &linhas {
        let comp = geo3d::dist(*a, *b);
        let Some(u) = geo3d::unit(geo3d::sub(*b, *a)) else { continue };
        // nós no meio da linha, em ordem
        let mut meio: Vec<(f64, usize)> = nos
            .iter()
            .enumerate()
            .filter_map(|(k, n)| {
                let t = geo3d::dot(geo3d::sub(n.p, *a), u);
                let fora = geo3d::norm(geo3d::sub(geo3d::sub(n.p, *a), geo3d::mul(u, t)));
                (fora < 0.05 && t > 0.01 && t < comp - 0.01).then_some((t, k))
            })
            .collect();
        meio.sort_by(|x, y| x.0.total_cmp(&y.0));
        let seq: Vec<usize> = std::iter::once(no_de(*a, &mut nos)).chain(meio.into_iter().map(|m| m.1)).chain(std::iter::once(no_de(*b, &mut nos))).collect();
        for w in seq.windows(2) {
            barras.push(Barra { i: w[0], j: w[1], secao: *sec, ref_y: Some(*y), q: [0.; 3] });
            origem.push((*id, *linha));
        }
    }
    Ok((Portico { nos, barras }, origem))
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: f64 = 200_000.;
    const G: f64 = 77_000.;

    fn secao() -> Secao {
        let (area, iy, iz, j) = secoes::tubo_retangular(50., 100., 4.);
        Secao { e: E, g: G, area, iy, iz, j }
    }

    fn perto(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-9)
    }

    /// Balanço de 2 m com carga P na ponta: flecha PL³/3EI, giro PL²/2EI, momento no engaste PL.
    #[test]
    fn balanco_com_carga_na_ponta() {
        let s = secao();
        let (l, p) = (2000., -1000.);
        let mut ponta = No::livre([l, 0., 0.]);
        ponta.carga[2] = p;
        let pt = Portico { nos: vec![No::engaste([0., 0., 0.]), ponta], barras: vec![Barra { i: 0, j: 1, secao: s, ref_y: None, q: [0.; 3] }] };
        let r = analisar(&pt).unwrap();
        // barra em X: y local = Z global, então a flexão em Z usa Iz
        let flecha = p * l.powi(3) / (3. * E * s.iz);
        assert!(perto(r.deslocamentos[1][2], flecha, 1e-9), "{} {}", r.deslocamentos[1][2], flecha);
        assert!(perto(r.reacoes[0][2], -p, 1e-9));
        let b = &r.barras[0];
        assert!(perto(b.maximo(|e| e.mz), (p * l).abs(), 1e-9));
        // momento cai linear até zero na ponta, e a flecha do diagrama bate com a do nó
        assert!(b.diagrama.last().unwrap().mz.abs() < 1e-6);
        assert!(perto(b.diagrama.last().unwrap().v, flecha, 1e-9));
        // flecha no meio: P x²(3L − x)/6EI
        let m = &b.diagrama[AMOSTRAS / 2];
        assert!(perto(m.v, p * (l / 2.).powi(2) * (3. * l - l / 2.) / (6. * E * s.iz), 1e-9));
    }

    /// Viga biapoiada com carga uniforme: Mmáx = qL²/8 no meio, flecha 5qL⁴/384EI, reações qL/2.
    #[test]
    fn biapoiada_com_carga_uniforme() {
        let s = secao();
        let (l, q) = (6000., -2.);
        let mut a = No::rotula([0., 0., 0.]);
        // trava o giro em torno do eixo da barra e o deslocamento ao longo dela só num lado
        a.apoio[3] = true;
        let mut b = No::rotula([l, 0., 0.]);
        b.apoio[0] = false;
        let pt = Portico { nos: vec![a, b], barras: vec![Barra { i: 0, j: 1, secao: s, ref_y: None, q: [0., 0., q] }] };
        let r = analisar(&pt).unwrap();
        let br = &r.barras[0];
        let meio = &br.diagrama[AMOSTRAS / 2];
        assert!(perto(meio.mz, -q * l * l / 8., 1e-9), "Mz {} esperado {}", meio.mz, -q * l * l / 8.);
        assert!(perto(meio.v, 5. * q * l.powi(4) / (384. * E * s.iz), 1e-9));
        assert!(meio.vy.abs() < 1e-6, "cortante zero no meio: {}", meio.vy);
        assert!(perto(r.reacoes[0][2], -q * l / 2., 1e-9) && perto(r.reacoes[1][2], -q * l / 2., 1e-9));
        assert!(perto(br.maximo(|e| e.vy), (q * l / 2.).abs(), 1e-9));
    }

    /// Biengastada com carga uniforme: momento de ponta qL²/12, no meio qL²/24, flecha qL⁴/384EI.
    #[test]
    fn biengastada_com_carga_uniforme() {
        let s = secao();
        let (l, q) = (4000., -3.);
        let pt = Portico { nos: vec![No::engaste([0., 0., 0.]), No::engaste([l, 0., 0.])], barras: vec![Barra { i: 0, j: 1, secao: s, ref_y: None, q: [0., 0., q] }] };
        let r = analisar(&pt).unwrap();
        let d = &r.barras[0].diagrama;
        assert!(perto(d[0].mz, q * l * l / 12., 1e-9), "{}", d[0].mz);
        assert!(perto(d[AMOSTRAS / 2].mz, -q * l * l / 24., 1e-9));
        assert!(perto(d[AMOSTRAS / 2].v, q * l.powi(4) / (384. * E * s.iz), 1e-9));
    }

    /// Barra vertical em compressão: N = −P em toda a barra, encurtamento PL/EA.
    #[test]
    fn pilar_comprimido() {
        let s = secao();
        let mut topo = No::livre([0., 0., 3000.]);
        topo.carga[2] = -50_000.;
        let pt = Portico { nos: vec![No::engaste([0., 0., 0.]), topo], barras: vec![Barra { i: 0, j: 1, secao: s, ref_y: None, q: [0.; 3] }] };
        let r = analisar(&pt).unwrap();
        assert!(r.barras[0].diagrama.iter().all(|e| perto(e.n, -50_000., 1e-9)));
        assert!(perto(r.deslocamentos[1][2], -50_000. * 3000. / (E * s.area), 1e-9));
    }

    /// Pórtico plano engastado na base com força horizontal no topo: equilíbrio global e simetria
    /// dos momentos de base dos dois pilares iguais.
    #[test]
    fn portico_plano_em_equilibrio() {
        let s = secao();
        let (h, l, hx) = (3000., 5000., 10_000.);
        let mut c = No::livre([0., 0., h]);
        c.carga[0] = hx;
        let nos = vec![No::engaste([0., 0., 0.]), c, No::livre([l, 0., h]), No::engaste([l, 0., 0.])];
        let bar = |i, j| Barra { i, j, secao: s, ref_y: Some([0., 1., 0.]), q: [0.; 3] };
        let pt = Portico { nos, barras: vec![bar(0, 1), bar(1, 2), bar(3, 2)] };
        let r = analisar(&pt).unwrap();
        let rx: f64 = r.reacoes.iter().map(|x| x[0]).sum();
        assert!(perto(rx, -hx, 1e-9), "soma das reações horizontais {rx}");
        // momento global em torno de Y na origem: H·h + soma dos momentos de reação + Rz·x = 0
        let my: f64 = r.reacoes.iter().zip(&pt.nos).map(|(re, no)| re[4] + no.p[2] * re[0] - no.p[0] * re[2]).sum::<f64>() + h * hx;
        assert!(my.abs() < 1e-3 * hx * h, "momento global {my}");
        // os dois topos andam o mesmo (viga axialmente quase rígida)
        assert!(perto(r.deslocamentos[1][0], r.deslocamentos[2][0], 1e-3));
    }

    #[test]
    fn mecanismo_e_recusado() {
        let s = secao();
        let pt = Portico { nos: vec![No::rotula([0., 0., 0.]), No::livre([1000., 0., 0.])], barras: vec![Barra { i: 0, j: 1, secao: s, ref_y: None, q: [0., 0., -1.] }] };
        assert!(analisar(&pt).unwrap_err().contains("instável"));
    }

    #[test]
    fn propriedades_das_secoes() {
        let (a, iy, iz, j) = secoes::tubo_redondo(60.3, 3.6);
        assert!(perto(a, 641.2, 1e-3) && perto(iy, iz, 1e-12) && perto(j, 2. * iy, 1e-12));
        // IPE 200 com cantos vivos: Iz ≈ 1840 cm⁴ (a tabela, com raios, dá 1943)
        let (_, _, iz, _) = secoes::perfil_i(200., 100., 5.6, 8.5);
        assert!(iz > 1.75e7 && iz < 1.95e7, "{iz}");
        let (a, _, _, _) = secoes::tubo_retangular(50., 100., 4.);
        assert!(perto(a, 50. * 100. - 42. * 92., 1e-12));
    }

    /// Duas colunas e uma viga de tubo quadrado no documento; o montante do meio chega na viga em T
    /// e a divide. Com as bases engastadas e carga na viga, a soma das reações equilibra a carga.
    #[test]
    fn modelo_do_documento_divide_a_junta_em_t() {
        use fenkra_core::*;
        let s = |x: &str| x.to_string();
        let mut d = Document::new("p");
        let l = |d: &mut Document, a: V3, b: V3| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
        let paths = vec![l(&mut d, [0., 0., 0.], [0., 0., 2000.]), l(&mut d, [0., 0., 2000.], [3000., 0., 2000.]), l(&mut d, [3000., 0., 0.], [3000., 0., 2000.]), l(&mut d, [1500., 0., 0.], [1500., 0., 2000.])];
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths,
            family: s("Tubo quadrado"),
            designation: s("80x4"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("80"), s("4")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::Nenhum,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
        }));
        let (mut p, origem) = modelo_da_estrutura(&d).unwrap();
        assert_eq!(p.nos.len(), 6, "{:?}", p.nos.iter().map(|n| n.p).collect::<Vec<_>>());
        assert_eq!(p.barras.len(), 5, "a viga se divide no montante");
        assert_eq!(origem.len(), 5);
        for n in p.nos.iter_mut().filter(|n| n.p[2] < 1.) {
            n.apoio = [true; 6];
        }
        for b in p.barras.iter_mut() {
            if (p.nos[b.i].p[2] - 2000.).abs() < 1. && (p.nos[b.j].p[2] - 2000.).abs() < 1. {
                b.q = [0., 0., -5.];
            }
        }
        let r = analisar(&p).unwrap();
        let rz: f64 = r.reacoes.iter().map(|x| x[2]).sum();
        assert!(perto(rz, 5. * 3000., 1e-9), "{rz}");
        // o montante do meio leva mais carga que cada coluna da ponta
        let base = |x: f64| r.reacoes[p.nos.iter().position(|n| (n.p[0] - x).abs() < 1. && n.p[2] < 1.).unwrap()][2];
        assert!(base(1500.) > base(0.) && perto(base(0.), base(3000.), 1e-6));
    }
}
