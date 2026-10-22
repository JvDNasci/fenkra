//! Vetoriza a arte do ícone: separa fundo azul, traço preto e preenchimento branco,
//! remove o selo do canto superior direito e traça os contornos como caminhos SVG.
//!
//!     cargo run --release --manifest-path tools/vetorizar/Cargo.toml -- assets/app/fenix-ref.png assets/app/fenix.svg
//!
//! O SVG resultante pode ser refinado no VectorCraft/Inkscape.

use image::{GrayImage, Luma, Rgb, RgbImage};
use imageproc::contours::{find_contours, BorderType};
use std::fmt::Write;

/// Classe de cada pixel: 0 = fundo azul, 1 = preto (traço), 2 = branco.
fn classify(p: &Rgb<u8>) -> u8 {
    let [r, g, b] = p.0.map(|v| v as i32);
    let blue = b > 150 && b - r > 70 && b - g > 40;
    if blue {
        0
    } else if r + g + b < 3 * 110 {
        1
    } else {
        2
    }
}

/// Douglas–Peucker em polígono fechado.
fn simplify(pts: &[(f32, f32)], eps: f32) -> Vec<(f32, f32)> {
    fn rec(p: &[(f32, f32)], eps: f32, out: &mut Vec<(f32, f32)>) {
        if p.len() < 3 {
            out.extend_from_slice(&p[..p.len().saturating_sub(1)]);
            return;
        }
        let (a, b) = (p[0], p[p.len() - 1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (mut idx, mut dmax) = (0, 0.0f32);
        for (i, q) in p.iter().enumerate().skip(1).take(p.len() - 2) {
            let d = ((q.0 - a.0) * dy - (q.1 - a.1) * dx).abs() / len;
            if d > dmax {
                dmax = d;
                idx = i;
            }
        }
        if dmax > eps {
            rec(&p[..=idx], eps, out);
            rec(&p[idx..], eps, out);
        } else {
            out.push(a);
        }
    }
    if pts.len() < 4 {
        return pts.to_vec();
    }
    // polígono fechado: simplifica as duas metades separadamente (extremos iguais degeneram)
    let mid = pts.len() / 2;
    let mut out = vec![];
    rec(&pts[..=mid], eps, &mut out);
    let mut second = pts[mid..].to_vec();
    second.push(pts[0]);
    rec(&second, eps, &mut out);
    out
}

/// Caminho SVG (regra par-ímpar) com todos os contornos da máscara.
fn trace(mask: &GrayImage, scale: f32, eps: f32, min_len: usize) -> String {
    let mut d = String::new();
    for c in find_contours::<u32>(mask) {
        if c.points.len() < min_len {
            continue;
        }
        let pts: Vec<(f32, f32)> = c.points.iter().map(|p| (p.x as f32 * scale, p.y as f32 * scale)).collect();
        let s = simplify(&pts, eps);
        if s.len() < 3 {
            continue;
        }
        let _ = c.border_type == BorderType::Hole;
        write!(d, "M{:.1} {:.1}", s[0].0, s[0].1).unwrap();
        for p in &s[1..] {
            write!(d, "L{:.1} {:.1}", p.0, p.1).unwrap();
        }
        d.push('Z');
    }
    d
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (inp, out) = (&args[1], &args[2]);
    let mut img: RgbImage = image::open(inp).expect("imagem de entrada").to_rgb8();
    let (w, h) = img.dimensions();
    // selo "Made with AI" no canto superior direito → fundo
    for y in 0..(h as f32 * 0.075) as u32 {
        for x in (w as f32 * 0.82) as u32..w {
            img.put_pixel(x, y, Rgb([20, 100, 230]));
        }
    }
    // máscaras: figura inteira (preto + branco) e só o branco
    let mut fig = GrayImage::new(w, h);
    let mut white = GrayImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels() {
        let c = classify(p);
        if c != 0 {
            fig.put_pixel(x, y, Luma([255]));
        }
        if c == 2 {
            white.put_pixel(x, y, Luma([255]));
        }
    }
    let count = |m: &GrayImage| m.pixels().filter(|p| p.0[0] > 0).count();
    eprintln!("figura: {} px, branco: {} px, contornos figura: {}", count(&fig), count(&white), find_contours::<u32>(&fig).len());
    let size = 256.0f32;
    let k = size / w as f32;
    let fig_d = trace(&fig, k, 0.35, 12);
    let white_d = trace(&white, k, 0.3, 8);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">
<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2f80ee"/><stop offset="1" stop-color="#1257d6"/></linearGradient></defs>
<rect x="0" y="0" width="256" height="256" rx="46" fill="url(#bg)"/>
<g transform="translate(14 14) scale(0.89)">
<path fill="#0b0d12" fill-rule="evenodd" d="{fig_d}"/>
<path fill="#f7f7f4" fill-rule="evenodd" d="{white_d}"/>
</g>
</svg>
"##
    );
    std::fs::write(out, svg).expect("gravar svg");
    println!("ok: {} bytes", std::fs::metadata(out).map(|m| m.len()).unwrap_or(0));
}
