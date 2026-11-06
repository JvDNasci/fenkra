// SPDX-License-Identifier: LGPL-2.1-or-later
//! B113 e B197: a geometria por trás das análises de fabricação (DFM). Tudo aqui trabalha sobre a
//! malha de exibição do kernel (`Mesh`), sem regra de processo: espessura local, saliência,
//! canto vivo, ângulo de saída e undercut. As regras e os limites ficam em `dfm.rs`.
//!
//! Como cada medida funciona:
//! - **Espessura** pela bola máxima inscrita. A partir de um ponto da superfície, a bola encosta ali
//!   e cresce para dentro da peça até tocar outra parte da superfície; o diâmetro dela é a
//!   espessura local. Um raio solto contra a normal erraria nas paredes laterais de uma chapa fina
//!   (mediria o comprimento da chapa); a bola mede a distância entre as duas faces da chapa. O
//!   cálculo começa com o raio do raio solto e encolhe a bola pelo ponto mais próximo que a
//!   invade, até ela só tocar a superfície.
//! - **Saliência** pela normal contra a direção de construção: a face que olha para baixo mais
//!   deitada que o limite precisa de suporte. Faces planas apoiadas na mesa não contam.
//! - **Canto vivo**: aresta côncava sem filete (raio zero) entre duas faces; o raio de ferramenta
//!   mínimo do processo decide se ela é problema.
//! - **Saída e undercut**: para uma direção de abertura `d`, a inclinação de cada triângulo é
//!   `asin(|n·d|)` (0 é parede paralela à abertura, sem saída). O triângulo que olha para `+d` é de
//!   cavidade, o que olha para `-d` é de núcleo; se o caminho até a abertura está bloqueado por
//!   outra parte da peça, é undercut.

use fenkra_kernel::{Mesh, NO_FACE};

pub type V3 = [f64; 3];

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn esc(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn cruz(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norma(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    let n = norma(a);
    if n < 1e-300 {
        [0.; 3]
    } else {
        esc(a, 1. / n)
    }
}

/// Um triângulo da malha com a normal que aponta para fora do material.
#[derive(Clone, Copy, Debug)]
pub struct Tri {
    pub p: [V3; 3],
    pub n: V3,
    pub area: f64,
    pub face: usize,
}

impl Tri {
    pub fn centro(&self) -> V3 {
        esc(add(add(self.p[0], self.p[1]), self.p[2]), 1. / 3.)
    }
    /// Ponto por coordenadas baricêntricas.
    pub fn ponto(&self, b: [f64; 3]) -> V3 {
        add(add(esc(self.p[0], b[0]), esc(self.p[1], b[1])), esc(self.p[2], b[2]))
    }
}

#[derive(Clone, Copy, Debug)]
struct No {
    lo: V3,
    hi: V3,
    /// folha: primeiro triângulo; interno: filho da esquerda
    a: u32,
    /// interno: filho da direita
    b: u32,
    /// folha: quantidade de triângulos; interno: 0
    n: u32,
}

/// Malha de triângulos com árvore de caixas para raio e ponto mais próximo.
pub struct Malha {
    pub tris: Vec<Tri>,
    nos: Vec<No>,
    /// ordem dos triângulos dentro das folhas
    ordem: Vec<u32>,
    pub lo: V3,
    pub hi: V3,
    pub diag: f64,
    /// quantidade de faces do corpo
    pub faces: usize,
}

fn v3f(v: &[f32], i: usize) -> V3 {
    [v[i * 3] as f64, v[i * 3 + 1] as f64, v[i * 3 + 2] as f64]
}

impl Malha {
    pub fn de(m: &Mesh) -> Option<Malha> {
        let faces = m.face_info.len();
        let mut tris = Vec::with_capacity(m.indices.len() / 3);
        for f in 0..faces {
            let ini = m.face_ranges.get(f).copied()? as usize;
            let fim = m.face_ranges.get(f + 1).map_or(m.indices.len(), |v| *v as usize);
            for t in m.indices.get(ini..fim)?.chunks_exact(3) {
                let p = [v3f(&m.positions, t[0] as usize), v3f(&m.positions, t[1] as usize), v3f(&m.positions, t[2] as usize)];
                let c = cruz(sub(p[1], p[0]), sub(p[2], p[0]));
                let area = norma(c) / 2.;
                if area < 1e-14 {
                    continue;
                }
                let mut n = unit(c);
                // a normal de exibição do vértice diz para que lado é "fora"
                let nv = add(add(v3f(&m.normals, t[0] as usize), v3f(&m.normals, t[1] as usize)), v3f(&m.normals, t[2] as usize));
                if dot(n, nv) < 0. {
                    n = esc(n, -1.);
                }
                tris.push(Tri { p, n, area, face: f });
            }
        }
        if tris.is_empty() {
            return None;
        }
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for t in &tris {
            for q in &t.p {
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
        }
        let diag = norma(sub(hi, lo)).max(1e-9);
        let mut mal = Malha { tris, nos: vec![], ordem: vec![], lo, hi, diag, faces };
        mal.ordem = (0..mal.tris.len() as u32).collect();
        let n = mal.tris.len();
        mal.constroi(0, n);
        Some(mal)
    }

    fn caixa(&self, ini: usize, fim: usize) -> (V3, V3) {
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for &i in &self.ordem[ini..fim] {
            for q in &self.tris[i as usize].p {
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
        }
        (lo, hi)
    }

    fn constroi(&mut self, ini: usize, fim: usize) -> usize {
        let (lo, hi) = self.caixa(ini, fim);
        let idx = self.nos.len();
        self.nos.push(No { lo, hi, a: ini as u32, b: 0, n: (fim - ini) as u32 });
        if fim - ini <= 4 {
            return idx;
        }
        let ext = sub(hi, lo);
        let eixo = if ext[0] >= ext[1] && ext[0] >= ext[2] { 0 } else if ext[1] >= ext[2] { 1 } else { 2 };
        let tris = &self.tris;
        self.ordem[ini..fim].sort_by(|a, b| tris[*a as usize].centro()[eixo].total_cmp(&tris[*b as usize].centro()[eixo]));
        let meio = (ini + fim) / 2;
        let esq = self.constroi(ini, meio);
        let dir = self.constroi(meio, fim);
        self.nos[idx].a = esq as u32;
        self.nos[idx].b = dir as u32;
        self.nos[idx].n = 0;
        idx
    }

    fn caixa_raio(no: &No, o: V3, inv: V3, tmax: f64) -> bool {
        let (mut t0, mut t1) = (0f64, tmax);
        for k in 0..3 {
            let (a, b) = ((no.lo[k] - o[k]) * inv[k], (no.hi[k] - o[k]) * inv[k]);
            let (a, b) = if a < b { (a, b) } else { (b, a) };
            // eixo paralelo ao raio: inv infinito; NaN quando a origem está na face da caixa
            if a.is_nan() || b.is_nan() {
                continue;
            }
            t0 = t0.max(a);
            t1 = t1.min(b);
            if t0 > t1 {
                return false;
            }
        }
        true
    }

    /// Distância até o primeiro triângulo cruzado pelo raio `o + t d` (`d` unitário), em `0 < t <= tmax`.
    pub fn raio(&self, o: V3, d: V3, tmax: f64) -> Option<f64> {
        self.raio_com(o, d, tmax, false).map(|x| x.0)
    }

    /// Como `raio`, devolvendo também o triângulo atingido.
    pub fn raio_tri(&self, o: V3, d: V3, tmax: f64) -> Option<(f64, usize)> {
        self.raio_com(o, d, tmax, false)
    }

    /// Há algum triângulo no caminho?
    pub fn bloqueado(&self, o: V3, d: V3, tmax: f64) -> bool {
        self.raio_com(o, d, tmax, true).is_some()
    }

    fn raio_com(&self, o: V3, d: V3, tmax: f64, qualquer: bool) -> Option<(f64, usize)> {
        let inv = [1. / d[0], 1. / d[1], 1. / d[2]];
        let mut melhor = tmax;
        let mut achou = None;
        let mut pilha = vec![0usize];
        while let Some(i) = pilha.pop() {
            let no = &self.nos[i];
            if !Self::caixa_raio(no, o, inv, melhor) {
                continue;
            }
            if no.n == 0 {
                pilha.push(no.a as usize);
                pilha.push(no.b as usize);
                continue;
            }
            for &k in &self.ordem[no.a as usize..(no.a + no.n) as usize] {
                if let Some(t) = hit(&self.tris[k as usize], o, d) {
                    if t > 0. && t <= melhor {
                        if qualquer {
                            return Some((t, k as usize));
                        }
                        melhor = t;
                        achou = Some((t, k as usize));
                    }
                }
            }
        }
        achou
    }

    /// Ponto da superfície mais perto de `c` (distância, ponto). `ate`: ignora o que está além.
    pub fn mais_perto(&self, c: V3, ate: f64) -> Option<(f64, V3, usize)> {
        let mut melhor = ate;
        let mut res = None;
        let mut pilha = vec![0usize];
        while let Some(i) = pilha.pop() {
            let no = &self.nos[i];
            if dist_caixa(no, c) > melhor {
                continue;
            }
            if no.n == 0 {
                pilha.push(no.a as usize);
                pilha.push(no.b as usize);
                continue;
            }
            for &k in &self.ordem[no.a as usize..(no.a + no.n) as usize] {
                let q = ponto_no_tri(&self.tris[k as usize], c);
                let d = norma(sub(q, c));
                if d < melhor {
                    melhor = d;
                    res = Some((d, q, k as usize));
                }
            }
        }
        res
    }
}

fn dist_caixa(no: &No, c: V3) -> f64 {
    let mut s = 0.;
    for k in 0..3 {
        let d = (no.lo[k] - c[k]).max(c[k] - no.hi[k]).max(0.);
        s += d * d;
    }
    s.sqrt()
}

/// Möller–Trumbore nos dois lados do triângulo.
fn hit(t: &Tri, o: V3, d: V3) -> Option<f64> {
    let (e1, e2) = (sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0]));
    let p = cruz(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-18 {
        return None;
    }
    let inv = 1. / det;
    let s = sub(o, t.p[0]);
    let u = dot(s, p) * inv;
    if !(-1e-9..=1. + 1e-9).contains(&u) {
        return None;
    }
    let q = cruz(s, e1);
    let v = dot(d, q) * inv;
    if v < -1e-9 || u + v > 1. + 1e-9 {
        return None;
    }
    Some(dot(e2, q) * inv)
}

/// Ponto do triângulo mais perto de `p` (Ericson, "Real-Time Collision Detection").
fn ponto_no_tri(t: &Tri, p: V3) -> V3 {
    let (a, b, c) = (t.p[0], t.p[1], t.p[2]);
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return add(a, esc(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return add(a, esc(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add(b, esc(sub(c, b), w));
    }
    let den = 1. / (va + vb + vc);
    add(a, add(esc(ab, vb * den), esc(ac, vc * den)))
}

// ---------------------------------------------------------------------------------- espessura

/// Uma medida de espessura num ponto da superfície.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Amostra {
    pub ponto: V3,
    pub face: usize,
    /// diâmetro da bola máxima inscrita, em mm
    pub valor: f64,
    /// área da superfície que a amostra representa, em mm²
    pub peso: f64,
}

/// Espessura local por bola máxima inscrita. Cerca de `max_amostras` pontos, repartidos pela área:
/// cada triângulo recebe uma grade de centros de subtriângulos, e uma face grande de poucos
/// triângulos não fica com um ponto só. Pontos que não acham o lado de dentro (malha aberta) ficam
/// de fora.
pub fn espessura(m: &Malha, max_amostras: usize) -> Vec<Amostra> {
    let total: f64 = m.tris.iter().map(|t| t.area).sum::<f64>().max(1e-300);
    // triângulos demais para um ponto cada: anda em passos
    let passo = (m.tris.len() / max_amostras.max(1)).max(1);
    let eps = m.diag * 1e-5;
    let mut r = vec![];
    for (i, t) in m.tris.iter().enumerate() {
        if i % passo != 0 {
            continue;
        }
        let quer = (max_amostras as f64 * t.area * passo as f64 / total).round().max(1.);
        // m x m subtriângulos (m² pontos), no máximo 16 x 16
        let n = (quer.sqrt().ceil() as usize).clamp(1, 16);
        let peso = t.area * passo as f64 / (n * n) as f64;
        for b in centros(n) {
            let p = t.ponto(b);
            if let Some(v) = bola(m, p, t.n, eps) {
                r.push(Amostra { ponto: p, face: t.face, valor: v, peso });
            }
        }
    }
    r
}

/// Centros dos n² subtriângulos em que a divisão regular em n parte cada lado do triângulo.
fn centros(n: usize) -> Vec<[f64; 3]> {
    let k = n as f64;
    let mut v = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..(n - i) {
            // triângulo "para cima" do canto (i, j)
            let (a, b) = ((i as f64 + 1. / 3.) / k, (j as f64 + 1. / 3.) / k);
            v.push([1. - a - b, a, b]);
            if j + 1 < n - i {
                // triângulo "para baixo" vizinho
                let (a, b) = ((i as f64 + 2. / 3.) / k, (j as f64 + 2. / 3.) / k);
                v.push([1. - a - b, a, b]);
            }
        }
    }
    v
}

/// Bola que encosta em `p`: raio e o segundo ponto de contacto com a normal dele.
struct Tangente {
    r: f64,
    q: Option<(V3, V3)>,
}

fn tangente(m: &Malha, p: V3, n: V3, eps: f64) -> Option<Tangente> {
    let dentro = esc(n, -1.);
    let (t0, k0) = m.raio_tri(add(p, esc(dentro, eps)), dentro, m.diag)?;
    // o raio parte de eps para dentro: a distância até a outra parede é t0 + eps
    let mut r = (t0 + eps) / 2.;
    let mut q = Some((add(p, esc(dentro, t0 + eps)), m.tris[k0].n));
    for _ in 0..40 {
        let c = add(p, esc(dentro, r));
        // só interessa quem invade a bola, com folga contra o ruído do próprio ponto
        let (d, x, k) = match m.mais_perto(c, r * (1. - 1e-4) - eps) {
            Some(x) => x,
            None => break,
        };
        let pq = sub(p, x);
        let proj = dot(n, pq);
        if proj <= 1e-12 || d >= r {
            break;
        }
        // bola que passa por p e por x, com o centro na reta da normal
        let novo = dot(pq, pq) / (2. * proj);
        if novo >= r * (1. - 1e-9) {
            break;
        }
        r = novo;
        q = Some((x, m.tris[k].n));
    }
    Some(Tangente { r, q })
}

/// Distância de `c` até a superfície, se `c` está dentro do material (senão 0).
fn folga_dentro(m: &Malha, c: V3) -> f64 {
    match m.mais_perto(c, f64::MAX) {
        Some((d, q, k)) if dot(m.tris[k].n, sub(c, q)) < 0. => d,
        _ => 0.,
    }
}

/// Espessura local em `p` (normal `n` para fora do material): diâmetro da bola inscrita que, a
/// partir da que encosta em `p`, anda pelo eixo medial até o raio parar de crescer.
///
/// A bola que só encosta em `p` mede pouco perto da quina de uma chapa: na parede lateral de uma
/// chapa de 1 mm, a 0,3 mm do fundo, ela tem 0,67 mm. Essa bola toca também o fundo, ou seja, o
/// centro dela está num ramo do eixo medial que sai do canto; andando por esse ramo (afastando o
/// centro dos dois pontos de contacto) o raio cresce até a meia espessura, onde o ramo encontra o
/// plano do meio. O máximo do raio no caminho é a espessura. Onde a bola toca dois pontos opostos
/// (face grande de uma chapa) o centro já está no plano do meio e fica onde está.
pub fn bola(m: &Malha, p: V3, n: V3, eps: f64) -> Option<f64> {
    let t = tangente(m, p, n, eps)?;
    let r = t.r;
    let c = add(p, esc(n, -r));
    let Some((q, _)) = t.q else { return Some(2. * r) };
    let soma = add(unit(sub(c, p)), unit(sub(c, q)));
    let sn = norma(soma);
    if sn < 0.2 {
        return Some(2. * r);
    }
    let u = esc(soma, 1. / sn);
    let f = |x: f64| folga_dentro(m, add(c, esc(u, x)));
    let h = r / 6.;
    let (mut melhor, mut xm) = (r, 0.);
    let mut x = 0.;
    while x < m.diag {
        x += h;
        let d = f(x);
        if d > melhor {
            (melhor, xm) = (d, x);
        } else if d < melhor * 0.85 {
            break;
        }
    }
    // o pico é uma quina de duas retas: refina por seção áurea entre os vizinhos do melhor passo
    let (mut lo, mut hi) = ((xm - h).max(0.), xm + h);
    let g = 0.618_033_988_749_895;
    for _ in 0..24 {
        let (x1, x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
        let (f1, f2) = (f(x1), f(x2));
        if f1 > melhor {
            melhor = f1;
        }
        if f2 > melhor {
            melhor = f2;
        }
        if f1 < f2 {
            lo = x1;
        } else {
            hi = x2;
        }
    }
    Some(2. * melhor)
}

/// Faixa de espessura: valores dentro de uma tolerância relativa entre si.
#[derive(Clone, Debug, PartialEq)]
pub struct Faixa {
    pub min: f64,
    pub max: f64,
    /// média ponderada pela área
    pub media: f64,
    /// área da superfície nesta faixa, em mm²
    pub area: f64,
    pub amostras: usize,
}

/// Agrupa as amostras em faixas: uma faixa começa no menor valor ainda solto e leva tudo até
/// `valor * (1 + tol)`. A legenda do mapa usa isto.
pub fn faixas(a: &[Amostra], tol: f64) -> Vec<Faixa> {
    let mut v: Vec<&Amostra> = a.iter().collect();
    v.sort_by(|x, y| x.valor.total_cmp(&y.valor));
    let mut r: Vec<Faixa> = vec![];
    let mut soma = 0.;
    for s in v {
        let nova = r.last().map_or(true, |f| s.valor > f.min * (1. + tol));
        if nova {
            if let Some(f) = r.last_mut() {
                f.media = soma / f.area.max(1e-300);
            }
            soma = 0.;
            r.push(Faixa { min: s.valor, max: s.valor, media: s.valor, area: 0., amostras: 0 });
        }
        if let Some(f) = r.last_mut() {
            f.max = s.valor;
            f.area += s.peso;
            f.amostras += 1;
            soma += s.valor * s.peso;
        }
    }
    if let Some(f) = r.last_mut() {
        f.media = soma / f.area.max(1e-300);
    }
    r
}

/// Só as faixas que juntam ao menos `fracao` da área medida (a legenda do mapa). O resto são
/// amostras de quina e de degrau, onde a bola é limitada pela geometria local, não pela espessura.
pub fn faixas_principais(a: &[Amostra], tol: f64, fracao: f64) -> Vec<Faixa> {
    let total: f64 = a.iter().map(|x| x.peso).sum();
    faixas(a, tol).into_iter().filter(|f| f.area >= total * fracao).collect()
}

// ---------------------------------------------------------------------------------- saliência

/// Resultado por face da análise de saliência.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceSaliencia {
    pub face: usize,
    /// maior inclinação em relação à vertical, em graus: 0 parede, 90 teto plano
    pub pior: f64,
    /// área que passa do limite, em mm²
    pub area: f64,
    pub area_total: f64,
    pub ponto: V3,
}

/// Faces que olham para baixo mais deitadas que `limite` graus a partir da vertical. `subir` é a
/// direção de construção (para cima). Faces planas apoiadas na mesa ficam de fora.
pub fn saliencia(m: &Malha, subir: V3, limite: f64) -> Vec<FaceSaliencia> {
    let up = unit(subir);
    let mesa = m.tris.iter().flat_map(|t| t.p.iter()).map(|p| dot(*p, up)).fold(f64::MAX, f64::min);
    let tol_mesa = m.diag * 1e-4;
    let mut por_face: Vec<FaceSaliencia> = (0..m.faces).map(|f| FaceSaliencia { face: f, pior: 0., area: 0., area_total: 0., ponto: [0.; 3] }).collect();
    let mut pior_area = vec![0.0f64; m.faces];
    for t in &m.tris {
        let f = &mut por_face[t.face];
        f.area_total += t.area;
        let c = -dot(t.n, up);
        if c <= 0. {
            continue;
        }
        // o triângulo deitado na mesa é só apoio
        let na_mesa = t.p.iter().all(|p| (dot(*p, up) - mesa).abs() <= tol_mesa);
        if na_mesa {
            continue;
        }
        let ang = c.clamp(0., 1.).asin().to_degrees();
        if ang > limite {
            f.area += t.area;
            f.pior = f.pior.max(ang);
            if t.area > pior_area[t.face] {
                pior_area[t.face] = t.area;
                f.ponto = t.centro();
            }
        }
    }
    por_face.retain(|f| f.area > 0.);
    por_face
}

// ---------------------------------------------------------------------------------- canto vivo

/// Aresta côncava sem filete: dois lados que se encontram em raio zero.
#[derive(Clone, Debug, PartialEq)]
pub struct CantoVivo {
    pub aresta: usize,
    pub faces: [usize; 2],
    pub ponto: V3,
    pub comprimento: f64,
    /// ângulo interno entre as faces, em graus (90 num canto de bolso)
    pub abertura: f64,
}

/// Arestas côncavas vivas (não suaves) maiores que `min_comp`. O filete de raio pequeno é medido
/// pelo reconhecimento de features, não aqui.
pub fn cantos_vivos(mesh: &Mesh, m: &Malha, min_comp: f64) -> Vec<CantoVivo> {
    let mut r = vec![];
    for (e, par) in mesh.edge_faces.iter().enumerate() {
        if par[0] == NO_FACE || par[1] == NO_FACE || mesh.edge_smooth.get(e).copied().unwrap_or(0) != 0 {
            continue;
        }
        let (fa, fb) = (par[0] as usize, par[1] as usize);
        let (Some(&s0), Some(&s1)) = (mesh.edge_ranges.get(e), Some(mesh.edge_ranges.get(e + 1).unwrap_or(&((mesh.edges.len() / 6) as u32)))) else { continue };
        let segs: Vec<[V3; 2]> = (s0 as usize..s1 as usize).filter_map(|s| mesh.edges.get(s * 6..s * 6 + 6).map(|w| [[w[0] as f64, w[1] as f64, w[2] as f64], [w[3] as f64, w[4] as f64, w[5] as f64]])).collect();
        if segs.is_empty() {
            continue;
        }
        let comp: f64 = segs.iter().map(|s| norma(sub(s[1], s[0]))).sum();
        if comp < min_comp {
            continue;
        }
        let mid = match mesh.edge_mids.get(e * 3..e * 3 + 3) {
            Some(w) => [w[0] as f64, w[1] as f64, w[2] as f64],
            None => continue,
        };
        // trecho da aresta mais perto do ponto médio dá a tangente
        let tang = segs.iter().min_by(|a, b| dist_seg(mid, **a).total_cmp(&dist_seg(mid, **b))).map(|s| unit(sub(s[1], s[0]))).unwrap_or([0.; 3]);
        // triângulo de cada face mais perto do meio da aresta
        let perto = |f: usize| m.tris.iter().filter(|t| t.face == f).min_by(|a, b| norma(sub(a.centro(), mid)).total_cmp(&norma(sub(b.centro(), mid))));
        let (Some(ta), Some(tb)) = (perto(fa), perto(fb)) else { continue };
        let tb_in = sub(tb.centro(), mid);
        let tb_in = unit(sub(tb_in, esc(tang, dot(tb_in, tang))));
        let ta_in = sub(ta.centro(), mid);
        let ta_in = unit(sub(ta_in, esc(tang, dot(ta_in, tang))));
        // o lado de b sai da aresta para o ar (convexa) ou para cima do material de a (côncava)
        let conc = dot(ta.n, tb_in);
        if conc <= 1e-6 {
            continue;
        }
        let abertura = dot(ta_in, tb_in).clamp(-1., 1.).acos().to_degrees();
        r.push(CantoVivo { aresta: e, faces: [fa, fb], ponto: mid, comprimento: comp, abertura });
    }
    r
}

fn dist_seg(p: V3, s: [V3; 2]) -> f64 {
    let ab = sub(s[1], s[0]);
    let t = (dot(sub(p, s[0]), ab) / dot(ab, ab).max(1e-300)).clamp(0., 1.);
    norma(sub(p, add(s[0], esc(ab, t))))
}

// ---------------------------------------------------------------------------------- saída e undercut

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Classe {
    /// olha para a direção de abertura e vê a saída livre
    Cavidade,
    /// olha para o lado oposto e vê a saída livre
    Nucleo,
    /// paralela à abertura (saída abaixo do mínimo): pede ângulo ou ação lateral
    Lateral,
    /// parte da face é cavidade e parte núcleo, ou nenhuma classe domina
    Ambigua,
}

impl Classe {
    pub fn nome(self) -> &'static str {
        match self {
            Classe::Cavidade => "cavidade",
            Classe::Nucleo => "núcleo",
            Classe::Lateral => "lateral",
            Classe::Ambigua => "ambígua",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaceSaida {
    pub face: usize,
    pub classe: Classe,
    /// menor e maior inclinação entre os triângulos, em graus (0 = paralela à abertura)
    pub saida_min: f64,
    pub saida_max: f64,
    /// área bloqueada ao longo da abertura, em mm²
    pub area_undercut: f64,
    pub area_total: f64,
    pub ponto: V3,
}

impl FaceSaida {
    pub fn undercut(&self) -> bool {
        self.area_undercut > self.area_total * 1e-3
    }
}

/// Classifica cada face para a abertura `d` (unitária ou não). `saida_min`: ângulo mínimo, em graus,
/// abaixo do qual a parede conta como paralela à abertura. A fração dominante de uma classe é 90% da área.
pub fn saida(m: &Malha, d: V3, saida_min: f64) -> Vec<FaceSaida> {
    let d = unit(d);
    let smin = saida_min.to_radians().sin();
    let eps = m.diag * 1e-5;
    let mut r: Vec<FaceSaida> = (0..m.faces).map(|f| FaceSaida { face: f, classe: Classe::Lateral, saida_min: 90., saida_max: 0., area_undercut: 0., area_total: 0., ponto: [0.; 3] }).collect();
    let mut areas = vec![[0f64; 3]; m.faces];
    let mut maior = vec![0f64; m.faces];
    for t in &m.tris {
        let c = dot(t.n, d);
        let ang = c.abs().clamp(0., 1.).asin().to_degrees();
        let f = &mut r[t.face];
        f.area_total += t.area;
        f.saida_min = f.saida_min.min(ang);
        f.saida_max = f.saida_max.max(ang);
        if t.area > maior[t.face] {
            maior[t.face] = t.area;
            f.ponto = t.centro();
        }
        let k = if c > smin { 0 } else if c < -smin { 1 } else { 2 };
        areas[t.face][k] += t.area;
        if k != 2 {
            // a saída é pelo lado para onde a face olha; o que está no caminho é undercut
            let sentido = if k == 0 { d } else { esc(d, -1.) };
            let o = add(t.centro(), esc(t.n, eps));
            if m.bloqueado(o, sentido, m.diag * 2.) {
                f.area_undercut += t.area;
            }
        }
    }
    for f in &mut r {
        let a = areas[f.face];
        let tot = f.area_total.max(1e-300);
        f.classe = if a[0] >= 0.9 * tot {
            Classe::Cavidade
        } else if a[1] >= 0.9 * tot {
            Classe::Nucleo
        } else if a[2] >= 0.9 * tot {
            Classe::Lateral
        } else {
            Classe::Ambigua
        };
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Caixa `lo`..`hi` como malha, sem o kernel: 12 triângulos com as normais para fora.
    pub fn caixa(lo: V3, hi: V3) -> Malha {
        let c = |x: usize, y: usize, z: usize| [if x == 0 { lo[0] } else { hi[0] }, if y == 0 { lo[1] } else { hi[1] }, if z == 0 { lo[2] } else { hi[2] }];
        // cada face: 4 cantos em ordem anti-horária vista de fora
        let quadros: [([V3; 4], V3); 6] = [
            ([c(0, 0, 0), c(0, 1, 0), c(1, 1, 0), c(1, 0, 0)], [0., 0., -1.]),
            ([c(0, 0, 1), c(1, 0, 1), c(1, 1, 1), c(0, 1, 1)], [0., 0., 1.]),
            ([c(0, 0, 0), c(1, 0, 0), c(1, 0, 1), c(0, 0, 1)], [0., -1., 0.]),
            ([c(0, 1, 0), c(0, 1, 1), c(1, 1, 1), c(1, 1, 0)], [0., 1., 0.]),
            ([c(0, 0, 0), c(0, 0, 1), c(0, 1, 1), c(0, 1, 0)], [-1., 0., 0.]),
            ([c(1, 0, 0), c(1, 1, 0), c(1, 1, 1), c(1, 0, 1)], [1., 0., 0.]),
        ];
        let mut tris = vec![];
        for (f, (q, n)) in quadros.iter().enumerate() {
            for p in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
                let area = norma(cruz(sub(p[1], p[0]), sub(p[2], p[0]))) / 2.;
                tris.push(Tri { p, n: *n, area, face: f });
            }
        }
        let diag = norma(sub(hi, lo));
        let mut m = Malha { tris, nos: vec![], ordem: vec![], lo, hi, diag, faces: 6 };
        m.ordem = (0..m.tris.len() as u32).collect();
        let n = m.tris.len();
        m.constroi(0, n);
        m
    }

    #[test]
    fn raio_acha_a_parede_oposta() {
        let m = caixa([0.; 3], [40., 20., 3.]);
        let t = m.raio([10., 10., 0.001], [0., 0., 1.], 100.).unwrap();
        assert!((t - 2.999).abs() < 1e-9, "{t}");
        assert!(m.raio([10., 10., 5.], [0., 0., 1.], 100.).is_none());
    }

    #[test]
    fn bola_da_chapa_mede_a_espessura_e_nao_o_comprimento() {
        // chapa 40 x 20 x 1: o raio solto na lateral mediria 40; a bola mede 1
        let m = caixa([0.; 3], [40., 20., 1.]);
        for a in espessura(&m, 600) {
            assert!((a.valor - 1.).abs() < 0.05, "{a:?}");
        }
    }

    #[test]
    fn faixas_separam_um_e_tres_milimetros() {
        let mut a = vec![];
        for v in [1.0, 1.01, 0.99, 3.0, 3.02, 2.98] {
            a.push(Amostra { ponto: [0.; 3], face: 0, valor: v, peso: 1. });
        }
        let f = faixas(&a, 0.05);
        assert_eq!(f.len(), 2);
        assert!((f[0].media - 1.).abs() < 0.02 && (f[1].media - 3.).abs() < 0.05);
    }

    #[test]
    fn saliencia_ignora_a_mesa_e_a_parede() {
        let m = caixa([0.; 3], [10., 10., 10.]);
        // sem teto virado para baixo e com o fundo na mesa: nada a reportar
        assert!(saliencia(&m, [0., 0., 1.], 45.).is_empty());
        // construindo de lado (z para a frente), a face de -y vira "teto" e o fundo vira parede
        let r = saliencia(&m, [0., 1., 0.], 45.);
        // com y para cima, o fundo (-y) é a mesa e também é ignorado
        assert!(r.is_empty());
    }

    #[test]
    fn saida_da_caixa_e_cavidade_nucleo_e_lateral() {
        let m = caixa([0.; 3], [10., 10., 10.]);
        let r = saida(&m, [0., 0., 1.], 1.);
        assert_eq!(r[0].classe, Classe::Nucleo);
        assert_eq!(r[1].classe, Classe::Cavidade);
        for f in 2..6 {
            assert_eq!(r[f].classe, Classe::Lateral);
            assert!(r[f].saida_max < 1e-6);
        }
        assert!(r.iter().all(|f| !f.undercut()));
    }
}
