// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: linhas ocultas por mapa de profundidade, em segundos no conjunto do kart (ADR 0088).
//!
//! Como funciona: a malha da forma (a mesma do sombreado, com as arestas do B-Rep já discretizadas
//! pelo kernel) é projetada na direção da vista e rasterizada num mapa de profundidade: em cada
//! pixel fica a superfície mais perto do observador. Depois cada aresta é amostrada ao longo da
//! tela; uma amostra é visível quando está na frente (ou na mesma profundidade, com uma tolerância que
//! cresce com a inclinação da superfície no pixel) da superfície mais perto. Trechos seguidos com a mesma resposta viram uma
//! polilinha: visível no tipo da aresta (0 aresta, 1 tangente, 2 contorno) ou oculta (3).
//!
//! As linhas desenhadas são a geometria das arestas, não a do pixel; o pixel só decide onde a
//! linha passa de visível a oculta, com erro de até um pixel (na folha, frações de décimo de mm).
//! O contorno das faces curvas (silhueta) sai da malha: o lado comum a dois triângulos da mesma
//! face, um de frente e outro de costas para o observador.
//!
//! Sem estado global e sem OpenCASCADE: roda em várias threads, ao contrário da HLR exata.

use crate::source::Axes;
use fenkra_kernel::Mesh;
use std::collections::HashMap;

/// Lado maior do mapa de profundidade, em pixels.
pub const RESOLUCAO: usize = 3000;

type V2 = [f64; 2];

struct Mapa {
    w: usize,
    h: usize,
    u0: f64,
    v0: f64,
    /// tamanho do pixel em mm do modelo
    px: f64,
    z: Vec<f32>,
}

impl Mapa {
    fn pixel(&self, u: f64, v: f64) -> (isize, isize) {
        (((u - self.u0) / self.px).floor() as isize, ((v - self.v0) / self.px).floor() as isize)
    }

    fn z(&self, x: isize, y: isize) -> f32 {
        if x < 0 || y < 0 || x >= self.w as isize || y >= self.h as isize {
            return f32::NEG_INFINITY;
        }
        self.z[y as usize * self.w + x as usize]
    }

    /// A superfície mais perto do observador no pixel do ponto e quanto a profundidade dela muda por
    /// pixel ali. A variação de cada eixo é a menor dos dois vizinhos: num degrau (borda de uma peça
    /// na frente de outra ou do fundo) só um lado salta, e a borda não ganha tolerância; numa face
    /// rasante (a silhueta de um cilindro) os dois lados mudam muito, e a tolerância cresce junto.
    /// `None` é fundo: ali nada cobre a aresta.
    fn superficie(&self, i: isize, j: isize) -> Option<(f32, f32)> {
        let c = self.z(i, j);
        if c == f32::NEG_INFINITY {
            return None;
        }
        let d = |x: isize, y: isize| {
            let q = self.z(x, y);
            if q == f32::NEG_INFINITY {
                f32::INFINITY
            } else {
                (q - c).abs()
            }
        };
        let gx = d(i - 1, j).min(d(i + 1, j));
        let gy = d(i, j - 1).min(d(i, j + 1));
        let g = [gx, gy].into_iter().filter(|g| g.is_finite()).fold(0f32, f32::max);
        Some((c, g))
    }
}

/// Pontos da malha nas coordenadas da vista: (u, v, profundidade para o observador).
fn projetar_pontos(m: &Mesh, a: &Axes) -> Vec<[f64; 3]> {
    m.positions
        .chunks_exact(3)
        .map(|p| {
            let q = [p[0] as f64, p[1] as f64, p[2] as f64];
            let d = q[0] * a.toward[0] + q[1] * a.toward[1] + q[2] * a.toward[2];
            let uv = a.project(q);
            [uv[0], uv[1], d]
        })
        .collect()
}

/// Rasteriza os triângulos no mapa, em faixas de linhas, uma thread por faixa.
fn rasterizar(pts: &[[f64; 3]], idx: &[u32], resolucao: usize) -> Option<Mapa> {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in pts {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    if lo[0] > hi[0] {
        return None;
    }
    let lado = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-6);
    let px = lado / resolucao as f64;
    // margem de dois pixels em volta: os vizinhos da borda da peça são fundo
    let (u0, v0) = (lo[0] - 2. * px, lo[1] - 2. * px);
    let w = ((hi[0] - u0) / px).ceil() as usize + 3;
    let h = ((hi[1] - v0) / px).ceil() as usize + 3;
    let mut z = vec![f32::NEG_INFINITY; w * h];
    let n = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8);
    let faixa = h.div_ceil(n);
    std::thread::scope(|s| {
        for (k, pedaco) in z.chunks_mut(faixa * w).enumerate() {
            let y0 = k * faixa;
            let y1 = y0 + pedaco.len() / w;
            s.spawn(move || {
                for t in idx.chunks_exact(3) {
                    let [a, b, c] = [pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]];
                    let q = |p: [f64; 3]| [(p[0] - u0) / px, (p[1] - v0) / px, p[2]];
                    let (a, b, c) = (q(a), q(b), q(c));
                    let ymin = a[1].min(b[1]).min(c[1]).floor().max(y0 as f64) as isize;
                    let ymax = a[1].max(b[1]).max(c[1]).ceil().min(y1 as f64 - 1.) as isize;
                    if ymin > ymax {
                        continue;
                    }
                    let xmin = a[0].min(b[0]).min(c[0]).floor().max(0.) as isize;
                    let xmax = a[0].max(b[0]).max(c[0]).ceil().min(w as f64 - 1.) as isize;
                    let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
                    if area.abs() < 1e-12 {
                        continue;
                    }
                    for y in ymin..=ymax {
                        let py = y as f64 + 0.5;
                        for x in xmin..=xmax {
                            let pxc = x as f64 + 0.5;
                            let w0 = ((b[0] - pxc) * (c[1] - py) - (c[0] - pxc) * (b[1] - py)) / area;
                            let w1 = ((c[0] - pxc) * (a[1] - py) - (a[0] - pxc) * (c[1] - py)) / area;
                            let w2 = 1. - w0 - w1;
                            if w0 < -1e-9 || w1 < -1e-9 || w2 < -1e-9 {
                                continue;
                            }
                            let d = (w0 * a[2] + w1 * b[2] + w2 * c[2]) as f32;
                            let cel = &mut pedaco[(y as usize - y0) * w + x as usize];
                            if d > *cel {
                                *cel = d;
                            }
                        }
                    }
                }
            });
        }
    });
    Some(Mapa { w, h, u0, v0, px, z })
}

/// Lados de silhueta das faces curvas: lado comum a dois triângulos, um de frente e outro de costas
/// para o observador. Nas superfícies livres os vértices se casam pela posição, não pelo índice: cada
/// retalho tem os seus, e a silhueta de uma superfície feita de muitos retalhos (um pneu) atravessa as
/// costuras entre eles.
/// Devolve pares de índices de ponto (o primeiro vértice achado em cada posição).
fn silhuetas(m: &Mesh, toward: [f64; 3]) -> Vec<[usize; 2]> {
    let ntri = m.indices.len() / 3;
    let mut out = vec![];
    let nf = m.face_ranges.len();
    let mut canon: HashMap<[i64; 3], usize> = HashMap::new();
    let mut lados: HashMap<(usize, usize), bool> = HashMap::new();
    for f in 0..nf {
        // face plana não tem silhueta
        if m.face_info.get(f).is_some_and(|i| i.kind == 0) {
            continue;
        }
        let t0 = m.face_ranges[f] as usize / 3;
        let t1 = m.face_ranges.get(f + 1).map_or(ntri, |x| *x as usize / 3);
        for t in t0..t1 {
            let i = &m.indices[t * 3..t * 3 + 3];
            let p = |k: usize| {
                let j = i[k] as usize * 3;
                [m.positions[j] as f64, m.positions[j + 1] as f64, m.positions[j + 2] as f64]
            };
            let (a, b, c) = (p(0), p(1), p(2));
            let (e1, e2) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            // triângulo de lado (a parede de um furo vista de cima, a faixa do filete bem na tangência):
            // o sinal seria ruído e espalharia silhueta pela parede inteira; conta como de frente, e a
            // silhueta fica na divisa com o primeiro triângulo de costas
            let ln = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let d = (n[0] * toward[0] + n[1] * toward[1] + n[2] * toward[2]) / ln.max(1e-300);
            let frente = d > -1e-3;
            // superfície livre: posição em milésimos de mm, o mesmo ponto em dois retalhos cai na mesma
            // chave; face analítica (furo, filete) fica com os próprios vértices, a silhueta é dela
            let livre = m.face_info.get(f).is_some_and(|x| x.kind == 5);
            let mut id = |k: usize| {
                if !livre {
                    return i[k] as usize;
                }
                let q = p(k).map(|x| (x * 1000.).round() as i64);
                *canon.entry(q).or_insert(i[k] as usize)
            };
            let v = [id(0), id(1), id(2)];
            for (x, y) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                if x == y {
                    continue;
                }
                let chave = (x.min(y), x.max(y));
                match lados.remove(&chave) {
                    Some(f0) if f0 != frente => out.push([chave.0, chave.1]),
                    Some(_) => {}
                    None => {
                        lados.insert(chave, frente);
                    }
                }
            }
        }
    }
    out
}

/// Costura entre dois retalhos de uma superfície livre (pneu, para-choque, banco): aresta lisa entre
/// duas faces que não são plano, cilindro, cone, esfera nem toro. A HLR do kernel separa as arestas lisas
/// em tangentes de grau 1 (a borda de um filete, desenhada como tangente) e costuras de regularidade
/// maior, que não desenha nunca. A malha só diz "lisa"; a costura é reconhecida pelo tipo das duas faces:
/// a borda de um filete sempre encosta numa face analítica.
fn costura(m: &Mesh, e: usize) -> bool {
    let Some(&[fa, fb]) = m.edge_faces.get(e) else { return false };
    let livre = |f: u32| m.face_info.get(f as usize).is_some_and(|i| i.kind == 5);
    m.edge_smooth.get(e).is_some_and(|x| *x != 0) && livre(fa) && livre(fb)
}

/// Lados soltos (pares de índices de ponto) em cadeias que seguem pelos pontos em comum.
fn encadear(lados: &[[usize; 2]]) -> Vec<Vec<usize>> {
    let mut viz: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, [a, b]) in lados.iter().enumerate() {
        viz.entry(*a).or_default().push(k);
        viz.entry(*b).or_default().push(k);
    }
    let mut usado = vec![false; lados.len()];
    let mut out = vec![];
    // começa pelas pontas (ponto com um lado só) e depois pelo que sobrou (laços fechados)
    let mut ordem: Vec<usize> = (0..lados.len()).collect();
    ordem.sort_by_key(|k| !lados[*k].iter().any(|p| viz[p].len() == 1));
    for k0 in ordem {
        if usado[k0] {
            continue;
        }
        usado[k0] = true;
        let [a, b] = lados[k0];
        // a ponta solta vai no começo
        let (ini, mut fim) = if viz[&b].len() == 1 && viz[&a].len() != 1 { (b, a) } else { (a, b) };
        let mut c = vec![ini, fim];
        while let Some(&k) = viz[&fim].iter().find(|k| !usado[**k]) {
            usado[k] = true;
            fim = if lados[k][0] == fim { lados[k][1] } else { lados[k][0] };
            c.push(fim);
        }
        out.push(c);
    }
    out
}

/// Junta segmentos (a, b) em polilinhas, cortando onde muda a visibilidade.
struct Juntar<'a> {
    mapa: &'a Mapa,
    tol: f64,
    ocultas: bool,
    out: Vec<(Vec<V2>, u8)>,
    atual: Option<(Vec<V2>, bool)>,
}

impl Juntar<'_> {
    /// A amostra está na frente da superfície do pixel, ou nela (com a tolerância da inclinação)?
    ///
    /// Amostra que cai num pixel de fundo, rente à borda de uma peça, olha os quatro vizinhos: a
    /// aresta de trás que se projeta sobre o contorno da da frente (a de cima de trás de uma placa vista
    /// de frente) fica oculta pelo vizinho, e não duplica a linha do contorno.
    fn visivel(&self, p: [f64; 3]) -> bool {
        let (i, j) = self.mapa.pixel(p[0], p[1]);
        let na_frente = |x: isize, y: isize| self.mapa.superficie(x, y).map(|(z, g)| p[2] as f32 + self.tol as f32 + 2. * g >= z);
        match na_frente(i, j) {
            Some(v) => v,
            None => [(i - 1, j), (i + 1, j), (i, j - 1), (i, j + 1)].into_iter().all(|(x, y)| na_frente(x, y).unwrap_or(true)),
        }
    }

    fn fechar(&mut self, kind: u8) {
        if let Some((l, vis)) = self.atual.take() {
            // como na HLR do kernel, a tangente escondida não vira linha oculta
            if l.len() > 1 && (vis || (self.ocultas && kind != 1)) {
                self.out.push((l, if vis { kind } else { 3 }));
            }
        }
    }

    fn ponto(&mut self, p: [f64; 3], vis: bool, kind: u8) {
        match &mut self.atual {
            Some((l, v)) if *v == vis => l.push([p[0], p[1]]),
            Some((l, _)) => {
                // o ponto da troca fica nas duas, para a linha não ter falha
                let ultimo = *l.last().unwrap();
                self.fechar(kind);
                self.atual = Some((vec![ultimo, [p[0], p[1]]], vis));
            }
            None => self.atual = Some((vec![[p[0], p[1]]], vis)),
        }
    }

    /// Um segmento da aresta, amostrado a cada pixel.
    fn segmento(&mut self, a: [f64; 3], b: [f64; 3], kind: u8) {
        let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = ((l / self.mapa.px).ceil() as usize).clamp(1, 4000);
        if self.atual.is_none() {
            let v = self.visivel(a);
            self.ponto(a, v, kind);
        }
        for k in 1..=n {
            let t = k as f64 / n as f64;
            let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
            let v = self.visivel(p);
            self.ponto(p, v, kind);
        }
    }
}

/// Linhas da vista pela malha `m`: 0 aresta visível, 1 tangente visível, 2 contorno, 3 oculta (com
/// `ocultas`). Mesmas coordenadas e tipos da HLR do kernel.
pub fn projetar(m: &Mesh, a: &Axes, ocultas: bool, resolucao: usize) -> Vec<(Vec<V2>, u8)> {
    let pts = projetar_pontos(m, a);
    let Some(mapa) = rasterizar(&pts, &m.indices, resolucao) else { return vec![] };
    let lado = mapa.px * resolucao as f64;
    // tolerância de profundidade: dois pixels mais um milésimo do tamanho (aresta sobre a própria face)
    let tol = 2. * mapa.px + 1e-3 * lado;
    let mut j = Juntar { mapa: &mapa, tol, ocultas, out: vec![], atual: None };
    let proj = |p: &[f32]| {
        let q = [p[0] as f64, p[1] as f64, p[2] as f64];
        let uv = a.project(q);
        [uv[0], uv[1], q[0] * a.toward[0] + q[1] * a.toward[1] + q[2] * a.toward[2]]
    };
    let nseg = m.edges.len() / 6;
    let ne = m.edge_ranges.len();
    for e in 0..ne {
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map_or(nseg, |x| *x as usize);
        if costura(m, e) {
            continue;
        }
        let kind = if m.edge_smooth.get(e).is_some_and(|x| *x != 0) { 1 } else { 0 };
        for sg in s0..s1 {
            let s = &m.edges[sg * 6..sg * 6 + 6];
            let (pa, pb) = (proj(&s[0..3]), proj(&s[3..6]));
            // segmento que não emenda no anterior começa uma polilinha nova
            if let Some((l, _)) = &j.atual {
                let u = l.last().unwrap();
                if (u[0] - pa[0]).abs() > 1e-9 || (u[1] - pa[1]).abs() > 1e-9 {
                    j.fechar(kind);
                }
            }
            j.segmento(pa, pb, kind);
        }
        j.fechar(kind);
    }
    // os lados de silhueta viram cadeias: uma polilinha por contorno, não um pedaço por lado
    for cadeia in encadear(&silhuetas(m, a.toward)) {
        for w in cadeia.windows(2) {
            j.segmento(pts[w[0]], pts[w[1]], 2);
        }
        j.fechar(2);
    }
    j.out
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Cubo de 10 mm com uma aresta por lado do quadrado de cima e de baixo e as quatro verticais.
    fn cubo() -> Mesh {
        let v = |x: f32, y: f32, z: f32| [x, y, z];
        let c = [v(0., 0., 0.), v(10., 0., 0.), v(10., 10., 0.), v(0., 10., 0.), v(0., 0., 10.), v(10., 0., 10.), v(10., 10., 10.), v(0., 10., 10.)];
        let positions: Vec<f32> = c.iter().flatten().copied().collect();
        // seis faces, dois triângulos cada, normais para fora
        let indices = vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7];
        let arestas = [(0, 1), (1, 2), (2, 3), (3, 0), (4, 5), (5, 6), (6, 7), (7, 4), (0, 4), (1, 5), (2, 6), (3, 7)];
        let edges: Vec<f32> = arestas.iter().flat_map(|(a, b)| c[*a].iter().chain(c[*b].iter()).copied().collect::<Vec<_>>()).collect();
        Mesh {
            positions,
            indices,
            face_ranges: (0..6).map(|k| k * 6).collect(),
            edges,
            edge_ranges: (0..12).collect(),
            edge_smooth: vec![0; 12],
            face_info: (0..6).map(|_| fenkra_kernel::FaceInfo { kind: 0, dir: [0.; 3], area: 0., point: [0.; 3] }).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn lados_soltos_viram_uma_cadeia() {
        let c = encadear(&[[3, 4], [1, 2], [2, 3]]);
        assert_eq!(c.len(), 1);
        assert!(c[0] == vec![1, 2, 3, 4] || c[0] == vec![4, 3, 2, 1], "{c:?}");
        // laço fechado: volta ao começo
        let c = encadear(&[[0, 1], [1, 2], [2, 0]]);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].len(), 4);
        assert_eq!(c[0].first(), c[0].last());
    }

    #[test]
    fn costura_entre_retalhos_livres_nao_vira_linha() {
        let mut m = cubo();
        m.edge_faces = vec![[0, 2]; 12];
        m.edge_smooth = vec![1; 12];
        // lisa entre faces analíticas (a borda de um filete): continua, como tangente
        assert!(!costura(&m, 0));
        // lisa entre duas superfícies livres: costura entre retalhos, não sai
        for f in &mut m.face_info {
            f.kind = 5;
        }
        assert!(costura(&m, 0));
        let a = Axes::new([0., -1., 0.], [1., 0., 0.]);
        let arestas = |m: &Mesh| projetar(m, &a, false, 400).into_iter().filter(|(_, k)| *k != 2).count();
        let mut viva = m.clone();
        viva.edge_smooth = vec![0; 12];
        assert!(arestas(&m) < arestas(&viva), "a costura some da vista");
        // viva entre superfícies livres: aresta de verdade
        assert!(!costura(&viva, 0));
    }

    fn comprimento(l: &[(Vec<V2>, u8)], kind: u8) -> f64 {
        l.iter().filter(|(_, k)| *k == kind).flat_map(|(p, _)| p.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())).sum()
    }

    #[test]
    fn cubo_de_frente_fica_no_quadrado() {
        // vista de frente: observador em -Y, x da folha = X
        let a = Axes::new([0., -1., 0.], [1., 0., 0.]);
        let l = projetar(&cubo(), &a, true, 400);
        // o quadrado da frente inteiro é visível (40 mm de linha); as de trás caem por baixo dele
        assert!(comprimento(&l, 0) >= 40. - 1e-6, "{l:?}");
        for (p, _) in &l {
            for q in p {
                assert!(q[0] > -1e-6 && q[0] < 10. + 1e-6 && q[1] > -1e-6 && q[1] < 10. + 1e-6);
            }
        }
    }

    #[test]
    fn cubo_em_isometrica_tem_nove_arestas_visiveis_e_tres_ocultas() {
        let t = [-1. / 3f64.sqrt(), -1. / 3f64.sqrt(), 1. / 3f64.sqrt()];
        let x = [1. / 2f64.sqrt(), -1. / 2f64.sqrt(), 0.];
        let a = Axes::new(t, x);
        let l = projetar(&cubo(), &a, true, 600);
        // aresta de 10 mm vista em isométrica: 10 * raiz(2/3)
        let e = 10. * (2f64 / 3.).sqrt();
        let (vis, ocu) = (comprimento(&l, 0), comprimento(&l, 3));
        assert!((vis - 9. * e).abs() < 0.3 && (ocu - 3. * e).abs() < 0.3, "visível {vis}, oculta {ocu}");
    }

    #[test]
    fn segunda_peca_atras_da_primeira_fica_oculta() {
        let mut m = cubo();
        let n = (m.positions.len() / 3) as u32;
        // cubo igual 30 mm atrás (y = 30..40), menor na tela: some inteiro atrás do da frente
        let extra: Vec<f32> = m.positions.chunks(3).flat_map(|p| [2. + p[0] * 0.5, 30. + p[1], 2. + p[2] * 0.5]).collect();
        m.positions.extend(extra);
        let idx: Vec<u32> = m.indices.iter().map(|i| i + n).collect();
        let base = m.indices.len() as u32;
        m.indices.extend(idx);
        m.face_ranges.extend((0..6).map(|k| base + k * 6));
        m.face_info.extend((0..6).map(|_| fenkra_kernel::FaceInfo { kind: 0, dir: [0.; 3], area: 0., point: [0.; 3] }));
        let extra_e: Vec<f32> = m.edges.chunks(3).flat_map(|p| [2. + p[0] * 0.5, 30. + p[1], 2. + p[2] * 0.5]).collect();
        let nseg = (m.edges.len() / 6) as u32;
        m.edges.extend(extra_e);
        m.edge_ranges.extend((0..12).map(|k| nseg + k));
        m.edge_smooth.extend([0u8; 12]);
        let a = Axes::new([0., -1., 0.], [1., 0., 0.]);
        let l = projetar(&m, &a, false, 400);
        // só as linhas do cubo da frente: nenhum ponto no meio do quadrado (onde o de trás estaria)
        for (p, k) in &l {
            assert_ne!(*k, 3);
            for q in p {
                let dentro = q[0] > 2.5 && q[0] < 6.5 && q[1] > 2.5 && q[1] < 6.5;
                assert!(!dentro, "linha do cubo de trás apareceu: {q:?}");
            }
        }
    }
}
