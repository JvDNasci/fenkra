// SPDX-License-Identifier: LGPL-2.1-or-later
//! `ver` (B417): a imagem da peça sem aplicativo aberto, para a pessoa ou o agente conferir o que
//! montou. Um rasterizador de software sobre a malha de trabalho: projeção ortogonal, z-buffer,
//! sombreamento por luz fixa, arestas por cima, corte por plano e suavização por supersampling.
//! Nada de janela, placa de vídeo ou display, então roda em servidor e em teste.
//!
//! O corte tira a metade da peça de um lado do plano `eixo = posição`. O que se vê do outro lado é a
//! face interna do casco, pintada com uma cor lisa de corte: o resultado lê como a seção maciça de
//! um sólido fechado, sem precisar fechar o contorno do corte.

use serde_json::{json, Value};

/// Vistas padrão, com a mesma convenção da folha de desenho: (direção do olho, direita da imagem).
pub const VISTAS: [&str; 7] = ["frente", "tras", "topo", "base", "esquerda", "direita", "iso"];

/// A vista "todas" junta frente, superior, direita e isométrica numa imagem só.
pub const VISTA_TODAS: &str = "todas";

fn eixo_da_vista(nome: &str) -> Option<([f64; 3], [f64; 3])> {
    let k = 1.0 / 3f64.sqrt();
    let s = 1.0 / 2f64.sqrt();
    Some(match nome {
        "frente" => ([0., -1., 0.], [1., 0., 0.]),
        "tras" => ([0., 1., 0.], [-1., 0., 0.]),
        "topo" => ([0., 0., 1.], [1., 0., 0.]),
        "base" => ([0., 0., -1.], [1., 0., 0.]),
        "esquerda" => ([-1., 0., 0.], [0., -1., 0.]),
        "direita" => ([1., 0., 0.], [0., 1., 0.]),
        "iso" => ([k, -k, k], [s, s, 0.]),
        _ => return None,
    })
}

/// Corte da peça por um plano perpendicular a um eixo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corte {
    /// 0 = x, 1 = y, 2 = z
    pub eixo: usize,
    /// posição do plano, em mm
    pub posicao: f64,
    /// fica a metade com coordenada maior que a posição (o padrão fica com a menor)
    pub manter_maior: bool,
}

#[derive(Clone, Debug)]
pub struct Pedido {
    pub vista: String,
    pub largura: u32,
    pub altura: u32,
    /// 1 enquadra a peça; maior aproxima
    pub zoom: f64,
    pub corte: Option<Corte>,
    pub arestas: bool,
    pub fundo_escuro: bool,
}

impl Default for Pedido {
    fn default() -> Self {
        Pedido { vista: "iso".into(), largura: 800, altura: 600, zoom: 1., corte: None, arestas: true, fundo_escuro: false }
    }
}

/// Malha de entrada: o que o kernel entrega, sem depender dele.
pub struct Malha<'a> {
    pub posicoes: &'a [f32],
    pub normais: &'a [f32],
    pub indices: &'a [u32],
    /// segmentos de aresta, seis números cada
    pub arestas: &'a [f32],
}

pub struct Imagem {
    pub largura: u32,
    pub altura: u32,
    pub rgb: Vec<u8>,
}

pub const LIMITE_LADO: u32 = 2400;

/// Lê o `corte` do pedido JSON: `{"eixo": "z", "posicao": 10, "lado": "menor"|"maior"}`.
pub fn corte_do_json(v: &Value) -> Result<Option<Corte>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let eixo = match v["eixo"].as_str().map(str::to_ascii_lowercase).as_deref() {
        Some("x") => 0,
        Some("y") => 1,
        Some("z") => 2,
        _ => return Err("o corte pede 'eixo' x, y ou z".into()),
    };
    let posicao = v["posicao"].as_f64().ok_or("o corte pede 'posicao' em mm")?;
    let manter_maior = match v["lado"].as_str() {
        None | Some("menor") => false,
        Some("maior") => true,
        Some(o) => return Err(format!("'lado' do corte é 'menor' ou 'maior', não '{o}'")),
    };
    Ok(Some(Corte { eixo, posicao, manter_maior }))
}

pub fn pedido_do_json(a: &Value) -> Result<Pedido, String> {
    let mut p = Pedido::default();
    if let Some(v) = a["vista"].as_str() {
        let v = v.to_ascii_lowercase();
        if eixo_da_vista(&v).is_none() && v != VISTA_TODAS {
            return Err(format!("vista '{v}' não existe; use {} ou {VISTA_TODAS}", VISTAS.join(", ")));
        }
        p.vista = v;
    }
    let lado = |nome: &str, padrao: u32| -> Result<u32, String> {
        match a[nome].as_u64() {
            None if a[nome].is_null() => Ok(padrao),
            Some(n) if (64..=u64::from(LIMITE_LADO)).contains(&n) => Ok(n as u32),
            _ => Err(format!("'{nome}' vai de 64 a {LIMITE_LADO} pixels")),
        }
    };
    p.largura = lado("largura", p.largura)?;
    p.altura = lado("altura", p.altura)?;
    if let Some(z) = a["zoom"].as_f64() {
        if !(0.1..=50.).contains(&z) {
            return Err("'zoom' vai de 0,1 a 50".into());
        }
        p.zoom = z;
    }
    p.corte = corte_do_json(&a["corte"])?;
    if let Some(b) = a["arestas"].as_bool() {
        p.arestas = b;
    }
    p.fundo_escuro = a["fundo"].as_str() == Some("escuro");
    Ok(p)
}

#[derive(Clone, Copy)]
struct V {
    p: [f64; 3],
    n: [f64; 3],
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn lerp(a: V, b: V, t: f64) -> V {
    let f = |i: usize| a.p[i] + (b.p[i] - a.p[i]) * t;
    let g = |i: usize| a.n[i] + (b.n[i] - a.n[i]) * t;
    V { p: [f(0), f(1), f(2)], n: [g(0), g(1), g(2)] }
}

/// Recorta o polígono contra o semiespaço mantido; devolve o polígono que sobra.
fn recortar(poli: &[V], c: Corte) -> Vec<V> {
    // distância com sinal: positiva do lado que fica
    let d = |v: &V| if c.manter_maior { v.p[c.eixo] - c.posicao } else { c.posicao - v.p[c.eixo] };
    let mut saida = Vec::with_capacity(poli.len() + 2);
    for i in 0..poli.len() {
        let (a, b) = (poli[i], poli[(i + 1) % poli.len()]);
        let (da, db) = (d(&a), d(&b));
        if da >= 0. {
            saida.push(a);
        }
        if (da >= 0.) != (db >= 0.) {
            saida.push(lerp(a, b, da / (da - db)));
        }
    }
    saida
}

struct Camera {
    centro: [f64; 3],
    olho: [f64; 3],
    direita: [f64; 3],
    cima: [f64; 3],
    escala: f64,
    cx: f64,
    cy: f64,
}

impl Camera {
    /// Projeta: (x, y) em pixels, com y para baixo, e a profundidade (maior = mais perto).
    fn proj(&self, p: [f64; 3]) -> (f64, f64, f64) {
        let q = sub(p, self.centro);
        (self.cx + dot(q, self.direita) * self.escala, self.cy - dot(q, self.cima) * self.escala, dot(q, self.olho))
    }
}

const COR_PECA: [f64; 3] = [0.56, 0.64, 0.76];
const COR_CORTE: [f64; 3] = [0.93, 0.60, 0.24];

/// Uma vista só, no tamanho pedido.
fn uma_vista(m: &Malha, p: &Pedido, vista: &str, largura: u32, altura: u32) -> Imagem {
    const SS: usize = 2; // supersampling por lado
    let (w, h) = (largura as usize * SS, altura as usize * SS);
    let fundo = if p.fundo_escuro { [0.10, 0.11, 0.13] } else { [0.95, 0.96, 0.97] };
    let mut cor = vec![[fundo[0], fundo[1], fundo[2]]; w * h];
    let mut prof = vec![f64::NEG_INFINITY; w * h];
    let (olho, direita) = eixo_da_vista(vista).unwrap_or(eixo_da_vista("iso").expect("iso existe"));
    let olho = norm(olho);
    let direita = norm(direita);
    let cima = norm(cross(olho, direita));

    // triângulos já recortados
    let vert = |i: usize| V {
        p: [f64::from(m.posicoes[3 * i]), f64::from(m.posicoes[3 * i + 1]), f64::from(m.posicoes[3 * i + 2])],
        n: if m.normais.len() >= 3 * i + 3 { [f64::from(m.normais[3 * i]), f64::from(m.normais[3 * i + 1]), f64::from(m.normais[3 * i + 2])] } else { [0.; 3] },
    };
    let mut tris: Vec<[V; 3]> = Vec::with_capacity(m.indices.len() / 3);
    for t in m.indices.chunks_exact(3) {
        let (a, b, c) = (vert(t[0] as usize), vert(t[1] as usize), vert(t[2] as usize));
        // sem normal de vértice, a do triângulo
        let ng = norm(cross(sub(b.p, a.p), sub(c.p, a.p)));
        let (a, b, c) = if dot(a.n, a.n) < 1e-9 { (V { n: ng, ..a }, V { n: ng, ..b }, V { n: ng, ..c }) } else { (a, b, c) };
        match p.corte {
            None => tris.push([a, b, c]),
            Some(cc) => {
                let poli = recortar(&[a, b, c], cc);
                for i in 1..poli.len().saturating_sub(1) {
                    tris.push([poli[0], poli[i], poli[i + 1]]);
                }
            }
        }
    }
    let mut segs: Vec<([f64; 3], [f64; 3])> = Vec::new();
    if p.arestas {
        for s in m.arestas.chunks_exact(6) {
            let a = [f64::from(s[0]), f64::from(s[1]), f64::from(s[2])];
            let b = [f64::from(s[3]), f64::from(s[4]), f64::from(s[5])];
            match p.corte {
                None => segs.push((a, b)),
                Some(cc) => {
                    let va = V { p: a, n: [0.; 3] };
                    let vb = V { p: b, n: [0.; 3] };
                    let r = recortar(&[va, vb], cc);
                    // dois vértices: o segmento recortado (o polígono de 2 pontos volta com ida e volta)
                    if r.len() >= 2 {
                        segs.push((r[0].p, r[1].p));
                    }
                }
            }
        }
    }

    // enquadramento: caixa de tudo o que sobrou
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for t in &tris {
        for v in t {
            for i in 0..3 {
                lo[i] = lo[i].min(v.p[i]);
                hi[i] = hi[i].max(v.p[i]);
            }
        }
    }
    if lo[0] > hi[0] {
        let f = [(fundo[0] * 255.) as u8, (fundo[1] * 255.) as u8, (fundo[2] * 255.) as u8];
        return Imagem { largura, altura, rgb: std::iter::repeat(f).take(largura as usize * altura as usize).flatten().collect() };
    }
    let centro = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2., (lo[2] + hi[2]) / 2.];
    let mut meio = 1e-9f64;
    for &x in &[lo[0], hi[0]] {
        for &y in &[lo[1], hi[1]] {
            for &z in &[lo[2], hi[2]] {
                let q = sub([x, y, z], centro);
                meio = meio.max(dot(q, direita).abs()).max(dot(q, cima).abs());
            }
        }
    }
    // a caixa inteira cabe em 90% do lado menor; o zoom aproxima a partir daí
    let escala = 0.45 * (w.min(h) as f64) / meio * p.zoom;
    let cam = Camera { centro, olho, direita, cima, escala, cx: w as f64 / 2., cy: h as f64 / 2. };

    // luz: de cima, da esquerda e de trás do olho
    let luz = norm([olho[0] + cima[0] * 0.7 - direita[0] * 0.5, olho[1] + cima[1] * 0.7 - direita[1] * 0.5, olho[2] + cima[2] * 0.7 - direita[2] * 0.5]);

    for t in &tris {
        let pr: Vec<(f64, f64, f64)> = t.iter().map(|v| cam.proj(v.p)).collect();
        let area = (pr[1].0 - pr[0].0) * (pr[2].1 - pr[0].1) - (pr[2].0 - pr[0].0) * (pr[1].1 - pr[0].1);
        if area.abs() < 1e-9 {
            continue;
        }
        let x0 = pr.iter().map(|q| q.0).fold(f64::MAX, f64::min).floor().max(0.) as usize;
        let x1 = (pr.iter().map(|q| q.0).fold(f64::MIN, f64::max).ceil().min(w as f64 - 1.)).max(0.) as usize;
        let y0 = pr.iter().map(|q| q.1).fold(f64::MAX, f64::min).floor().max(0.) as usize;
        let y1 = (pr.iter().map(|q| q.1).fold(f64::MIN, f64::max).ceil().min(h as f64 - 1.)).max(0.) as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let w0 = ((pr[1].0 - px) * (pr[2].1 - py) - (pr[2].0 - px) * (pr[1].1 - py)) / area;
                let w1 = ((pr[2].0 - px) * (pr[0].1 - py) - (pr[0].0 - px) * (pr[2].1 - py)) / area;
                let w2 = 1. - w0 - w1;
                if w0 < -1e-9 || w1 < -1e-9 || w2 < -1e-9 {
                    continue;
                }
                let z = w0 * pr[0].2 + w1 * pr[1].2 + w2 * pr[2].2;
                let i = y * w + x;
                if z <= prof[i] {
                    continue;
                }
                prof[i] = z;
                let n = norm([
                    w0 * t[0].n[0] + w1 * t[1].n[0] + w2 * t[2].n[0],
                    w0 * t[0].n[1] + w1 * t[1].n[1] + w2 * t[2].n[1],
                    w0 * t[0].n[2] + w1 * t[1].n[2] + w2 * t[2].n[2],
                ]);
                let de_costas = dot(n, olho) < 0.;
                cor[i] = if de_costas && p.corte.is_some() {
                    // casco visto por dentro: é a face do corte
                    let l = 0.82 + 0.18 * dot(norm([-n[0], -n[1], -n[2]]), luz).max(0.);
                    [COR_CORTE[0] * l, COR_CORTE[1] * l, COR_CORTE[2] * l]
                } else {
                    let nn = if de_costas { [-n[0], -n[1], -n[2]] } else { n };
                    let l = 0.30 + 0.70 * dot(nn, luz).max(0.);
                    [COR_PECA[0] * l, COR_PECA[1] * l, COR_PECA[2] * l]
                };
            }
        }
    }

    // arestas por cima, com um pequeno viés de profundidade para não sumirem na própria face
    let tinta = if p.fundo_escuro { [0.92, 0.94, 0.97] } else { [0.08, 0.09, 0.11] };
    let bias = 1e-3 * meio.max(1.);
    for (a, b) in &segs {
        let (pa, pb) = (cam.proj(*a), cam.proj(*b));
        let passos = ((pb.0 - pa.0).hypot(pb.1 - pa.1).ceil() as usize).max(1);
        for k in 0..=passos {
            let u = k as f64 / passos as f64;
            let (x, y, z) = (pa.0 + (pb.0 - pa.0) * u, pa.1 + (pb.1 - pa.1) * u, pa.2 + (pb.2 - pa.2) * u);
            if x < 0. || y < 0. || x >= w as f64 || y >= h as f64 {
                continue;
            }
            let (xi, yi) = (x as usize, y as usize);
            if z + bias >= prof[yi * w + xi] {
                cor[yi * w + xi] = tinta;
            }
        }
    }

    // reduz o supersampling
    let mut rgb = Vec::with_capacity(largura as usize * altura as usize * 3);
    for y in 0..altura as usize {
        for x in 0..largura as usize {
            let mut s = [0.; 3];
            for dy in 0..SS {
                for dx in 0..SS {
                    let c = cor[(y * SS + dy) * w + x * SS + dx];
                    for k in 0..3 {
                        s[k] += c[k];
                    }
                }
            }
            for v in s {
                rgb.push((v / (SS * SS) as f64 * 255.).round().clamp(0., 255.) as u8);
            }
        }
    }
    Imagem { largura, altura, rgb }
}

/// A imagem pedida: uma vista, ou as quatro principais em grade.
pub fn renderizar(m: &Malha, p: &Pedido) -> Imagem {
    if p.vista != VISTA_TODAS {
        return uma_vista(m, p, &p.vista, p.largura, p.altura);
    }
    let (lw, lh) = (p.largura / 2, p.altura / 2);
    let partes: Vec<Imagem> = ["frente", "topo", "direita", "iso"].iter().map(|v| uma_vista(m, p, v, lw, lh)).collect();
    let (w, h) = (lw * 2, lh * 2);
    let mut rgb = vec![0u8; w as usize * h as usize * 3];
    for (k, im) in partes.iter().enumerate() {
        let (ox, oy) = ((k as u32 % 2) * lw, (k as u32 / 2) * lh);
        for y in 0..lh {
            let dst = (((oy + y) * w + ox) * 3) as usize;
            let src = (y * lw * 3) as usize;
            rgb[dst..dst + lw as usize * 3].copy_from_slice(&im.rgb[src..src + lw as usize * 3]);
        }
    }
    // filetes de separação entre as vistas
    let sep = if p.fundo_escuro { 70u8 } else { 190u8 };
    for x in 0..w {
        for y in [lh - 1, lh] {
            let i = ((y * w + x) * 3) as usize;
            rgb[i..i + 3].fill(sep);
        }
    }
    for y in 0..h {
        for x in [lw - 1, lw] {
            let i = ((y * w + x) * 3) as usize;
            rgb[i..i + 3].fill(sep);
        }
    }
    Imagem { largura: w, altura: h, rgb }
}

/// PNG da imagem.
pub fn png(im: &Imagem) -> Result<Vec<u8>, String> {
    let mut saida = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut saida, im.largura, im.altura);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&im.rgb).map_err(|e| e.to_string())?;
    }
    Ok(saida)
}

/// Resumo em JSON do que foi desenhado, sem a imagem.
pub fn descricao(p: &Pedido, im: &Imagem) -> Value {
    let mut v = json!({ "vista": p.vista, "largura": im.largura, "altura": im.altura, "zoom": p.zoom, "arestas": p.arestas });
    if let Some(c) = p.corte {
        let eixo = ["x", "y", "z"][c.eixo];
        v["corte"] = json!({ "eixo": eixo, "posicao": c.posicao, "lado": if c.manter_maior { "maior" } else { "menor" } });
    }
    v
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Caixa de 40 x 30 x 20 mm com cantos em (0,0,0), com normais por face.
    fn caixa() -> (Vec<f32>, Vec<f32>, Vec<u32>, Vec<f32>) {
        let (a, b, c) = (40f32, 30f32, 20f32);
        let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
            ([0., 0., -1.], [[0., 0., 0.], [0., b, 0.], [a, b, 0.], [a, 0., 0.]]),
            ([0., 0., 1.], [[0., 0., c], [a, 0., c], [a, b, c], [0., b, c]]),
            ([0., -1., 0.], [[0., 0., 0.], [a, 0., 0.], [a, 0., c], [0., 0., c]]),
            ([0., 1., 0.], [[0., b, 0.], [0., b, c], [a, b, c], [a, b, 0.]]),
            ([-1., 0., 0.], [[0., 0., 0.], [0., 0., c], [0., b, c], [0., b, 0.]]),
            ([1., 0., 0.], [[a, 0., 0.], [a, b, 0.], [a, b, c], [a, 0., c]]),
        ];
        let (mut pos, mut nrm, mut idx) = (vec![], vec![], vec![]);
        for (n, q) in faces {
            let base = (pos.len() / 3) as u32;
            for v in q {
                pos.extend(v);
                nrm.extend(n);
            }
            idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let mut arestas = vec![];
        for (x, y) in [(0., 0.), (a, 0.), (a, b), (0., b)] {
            arestas.extend([x, y, 0., x, y, c]);
        }
        (pos, nrm, idx, arestas)
    }

    fn fundo(p: &Pedido) -> [u8; 3] {
        if p.fundo_escuro { [26, 28, 33] } else { [242, 245, 247] }
    }

    fn pixel(im: &Imagem, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * im.largura + x) * 3) as usize;
        [im.rgb[i], im.rgb[i + 1], im.rgb[i + 2]]
    }

    fn desenhados(im: &Imagem, p: &Pedido) -> usize {
        let f = fundo(p);
        im.rgb.chunks_exact(3).filter(|c| (i32::from(c[0]) - i32::from(f[0])).abs() > 6 || (i32::from(c[1]) - i32::from(f[1])).abs() > 6).count()
    }

    #[test]
    fn caixa_aparece_no_meio_e_o_canto_fica_no_fundo() {
        let (pos, nrm, idx, ar) = caixa();
        let m = Malha { posicoes: &pos, normais: &nrm, indices: &idx, arestas: &ar };
        let p = Pedido { vista: "frente".into(), largura: 400, altura: 300, ..Default::default() };
        let im = renderizar(&m, &p);
        assert_eq!((im.largura, im.altura), (400, 300));
        assert_ne!(pixel(&im, 200, 150), fundo(&p), "o centro tem a peça");
        assert_eq!(pixel(&im, 2, 2), fundo(&p), "o canto é fundo");
        // a frente é 40 x 20: ocupa 0,9 do lado menor em altura ou largura
        let n = desenhados(&im, &p);
        assert!(n > 20_000 && n < 120_000, "{n}");
    }

    #[test]
    fn peca_nunca_se_confunde_com_o_fundo_claro_ou_escuro() {
        let (pos, nrm, idx, ar) = caixa();
        let m = Malha { posicoes: &pos, normais: &nrm, indices: &idx, arestas: &ar };
        for escuro in [false, true] {
            let p = Pedido { vista: "iso".into(), fundo_escuro: escuro, ..Default::default() };
            let im = renderizar(&m, &p);
            let l = |c: [u8; 3]| 0.2126 * f64::from(c[0]) + 0.7152 * f64::from(c[1]) + 0.0722 * f64::from(c[2]);
            let centro = pixel(&im, p.largura / 2, p.altura / 2);
            assert!((l(centro) - l(fundo(&p))).abs() > 40., "escuro={escuro}: {centro:?}");
        }
    }

    #[test]
    fn vistas_diferentes_dao_imagens_diferentes_e_a_desconhecida_e_recusada() {
        let (pos, nrm, idx, ar) = caixa();
        let m = Malha { posicoes: &pos, normais: &nrm, indices: &idx, arestas: &ar };
        let f = renderizar(&m, &Pedido { vista: "frente".into(), ..Default::default() });
        let t = renderizar(&m, &Pedido { vista: "topo".into(), ..Default::default() });
        assert_ne!(f.rgb, t.rgb);
        assert!(pedido_do_json(&json!({ "vista": "diagonal" })).unwrap_err().contains("não existe"));
    }

    #[test]
    fn corte_tira_metade_e_mostra_a_cor_de_corte() {
        let (pos, nrm, idx, ar) = caixa();
        let m = Malha { posicoes: &pos, normais: &nrm, indices: &idx, arestas: &ar };
        let base = Pedido { vista: "frente".into(), ..Default::default() };
        let inteira = desenhados(&renderizar(&m, &base), &base);
        // corta em y = 15 mantendo y < 15 e olhando de frente (-y): a frente da peça continua à vista
        let cortada_menor = Pedido { corte: Some(Corte { eixo: 1, posicao: 15., manter_maior: false }), ..base.clone() };
        let a = renderizar(&m, &cortada_menor);
        assert!((desenhados(&a, &cortada_menor) as i64 - inteira as i64).abs() < 400, "a silhueta é a mesma");
        // mantendo y > 15, quem olha de frente vê o casco por dentro: cor de corte
        let cortada_maior = Pedido { corte: Some(Corte { eixo: 1, posicao: 15., manter_maior: true }), ..base.clone() };
        let b = renderizar(&m, &cortada_maior);
        let c = pixel(&b, b.largura / 2, b.altura / 2);
        assert!(c[0] > c[2] + 60, "laranja de corte esperado, veio {c:?}");
        // corte em z = 10 visto de cima, mantendo o de baixo: a silhueta é a mesma; visto de baixo, o miolo é corte
        let topo = Pedido { vista: "base".into(), corte: Some(Corte { eixo: 2, posicao: 10., manter_maior: true }), ..Default::default() };
        let t = renderizar(&m, &topo);
        let c = pixel(&t, t.largura / 2, t.altura / 2);
        assert!(c[0] > c[2] + 60, "{c:?}");
        // corte além da peça: nada sobra
        let fora = Pedido { corte: Some(Corte { eixo: 0, posicao: -5., manter_maior: false }), ..Default::default() };
        assert_eq!(desenhados(&renderizar(&m, &fora), &fora), 0);
    }

    #[test]
    fn zoom_aproxima_e_todas_junta_quatro_vistas() {
        let (pos, nrm, idx, ar) = caixa();
        let m = Malha { posicoes: &pos, normais: &nrm, indices: &idx, arestas: &ar };
        let p1 = Pedido { vista: "frente".into(), ..Default::default() };
        let p2 = Pedido { zoom: 1.5, ..p1.clone() };
        assert!(desenhados(&renderizar(&m, &p2), &p2) > desenhados(&renderizar(&m, &p1), &p1));
        let todas = Pedido { vista: VISTA_TODAS.into(), largura: 400, altura: 300, ..Default::default() };
        let im = renderizar(&m, &todas);
        assert_eq!((im.largura, im.altura), (400, 300));
        for (x, y) in [(100, 75), (300, 75), (100, 225), (300, 225)] {
            assert_ne!(pixel(&im, x, y), fundo(&todas), "({x},{y}) sem peça");
        }
    }

    #[test]
    fn png_sai_valido_e_pedido_fora_do_limite_e_recusado() {
        let (pos, nrm, idx, ar) = caixa();
        let m = Malha { posicoes: &pos, normais: &nrm, indices: &idx, arestas: &ar };
        let im = renderizar(&m, &Pedido { largura: 200, altura: 150, ..Default::default() });
        let bytes = png(&im).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert!(pedido_do_json(&json!({ "largura": 99999 })).is_err());
        assert!(pedido_do_json(&json!({ "zoom": 0 })).is_err());
        assert!(pedido_do_json(&json!({ "corte": { "eixo": "w", "posicao": 1 } })).is_err());
        assert_eq!(pedido_do_json(&json!({ "corte": { "eixo": "Z", "posicao": 5, "lado": "maior" } })).unwrap().corte, Some(Corte { eixo: 2, posicao: 5., manter_maior: true }));
    }
}
