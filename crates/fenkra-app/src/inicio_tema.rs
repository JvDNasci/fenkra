// SPDX-License-Identifier: LGPL-2.1-or-later
//! Tela inicial e carregamento (B335): paleta derivada só do tema em uso e fundo animado.
//!
//! A paleta sai de `ThemeDef` (painel, janela, texto, destaque, peça), nunca de cor fixa, para a
//! tela inicial ser a mesma do restante do programa em qualquer tema, inclusive os do usuário.
//! O movimento é barato: poucas dezenas de segmentos e círculos pequenos, redesenhados a 30
//! quadros por segundo no máximo, e só com `movimento_permitido`.

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::theme::ThemeDef;

/// Taxa do movimento contínuo: 30 quadros por segundo bastam para deriva lenta e poupam a GPU.
pub const QUADRO_MS: u64 = 33;

/// Cores da tela inicial, todas derivadas do tema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paleta {
    pub escuro: bool,
    pub topo: Color32,
    pub base: Color32,
    pub texto: Color32,
    pub destaque: Color32,
    pub cartao: Color32,
    /// degradê atrás da miniatura do exemplo
    pub mini_topo: Color32,
    pub mini_base: Color32,
    pub peca: [u8; 3],
    pub aresta: [u8; 3],
}

fn rgb(c: [u8; 3]) -> Color32 {
    crate::theme::cor_rgb(c)
}

fn mistura(a: [u8; 3], b: [u8; 3], k: f32) -> [u8; 3] {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8;
    [m(0), m(1), m(2)]
}

impl Paleta {
    pub fn de(def: &ThemeDef) -> Paleta {
        // fundo: o painel do tema puxado de leve para o destaque (cima) e para o texto (baixo),
        // assim o texto do tema sempre tem contraste sobre ele
        let topo = mistura(def.panel, def.accent, 0.08);
        let base = mistura(def.panel, def.text, if def.dark { 0.07 } else { 0.10 });
        let peca = def.peca.unwrap_or(crate::theme::PECA_PADRAO);
        let aresta = def.aresta.unwrap_or_else(|| mistura(peca, [0; 3], 0.8));
        // miniatura: parte da janela; se a peça não se destacar, o fundo se afasta dela
        let (mut a, mut b) = (mistura(def.window, def.text, 0.07), mistura(def.window, def.text, 0.18));
        // confere as duas pontas do degradê, não só o meio
        let pior = crate::acessibilidade::contraste(peca, a).min(crate::acessibilidade::contraste(peca, b));
        if pior < 1.6 {
            let polo = if crate::acessibilidade::contraste(peca, [0; 3]) > crate::acessibilidade::contraste(peca, [255; 3]) { [0; 3] } else { [255; 3] };
            // afasta o fundo da peça, em passos, até o contraste mínimo ser atingido nas duas pontas
            let mut k = 0.30;
            loop {
                a = mistura(def.window, polo, k);
                b = mistura(def.window, polo, (k + 0.18).min(1.));
                if k >= 0.95 || (crate::acessibilidade::contraste(peca, a).min(crate::acessibilidade::contraste(peca, b)) >= 1.6) {
                    break;
                }
                k += 0.05;
            }
        }
        Paleta { escuro: def.dark, topo: rgb(topo), base: rgb(base), texto: rgb(def.text), destaque: rgb(def.accent), cartao: rgb(def.window), mini_topo: rgb(a), mini_base: rgb(b), peca, aresta }
    }

    /// Paleta do tema em vigor (o mesmo que as preferências aplicaram).
    pub fn atual() -> Paleta {
        Paleta::de(&crate::theme::current().unwrap_or_else(crate::theme::default_def))
    }
}

/// Movimento contínuo só quando a pessoa não pediu menos: animações ligadas, "reduzir movimento"
/// desligado e modo leve desligado.
pub fn movimento_permitido(animacoes: bool, reduzir_movimento: bool, modo_leve: bool) -> bool {
    animacoes && !reduzir_movimento && !modo_leve
}

/// Fundo: degradê do tema, brilho que respira, grade de chão andando, partículas e, quando há
/// movimento, um cubo de arame girando atrás da marca e uma varredura de luz.
pub fn fundo(p: &egui::Painter, rect: Rect, pal: &Paleta, t: f32, animar: bool) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(rect.left_top(), pal.topo);
    m.colored_vertex(rect.right_top(), pal.topo);
    m.colored_vertex(rect.right_bottom(), pal.base);
    m.colored_vertex(rect.left_bottom(), pal.base);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(m));
        let glow = rect.center_top() + egui::vec2((t * 0.3).sin() * 30., rect.height() * 0.3);
    let breathe = 1. + (if animar { 0.08 } else { 0.04 }) * (t * 0.8).sin();
    for k in 0..24 {
        let r = (460. - k as f32 * 17.) * breathe;
        p.circle_filled(glow, r, crate::theme::com_alfa(pal.destaque, if pal.escuro { 3 } else { 4 }));
    }
    // grade de chão (cor do destaque do tema), andando para a frente
    let linha = crate::theme::com_alfa(pal.destaque, if animar { 34 } else { 24 });
    let horizon = rect.top() + rect.height() * 0.62;
    let vp = egui::pos2(rect.center().x, horizon - rect.height() * 0.25);
    for i in -14..=14 {
        let x = rect.center().x + i as f32 * rect.width() / 14.;
        let b = egui::pos2(x, rect.bottom());
        let tt = (horizon - vp.y) / (b.y - vp.y);
        p.line_segment([vp + (b - vp) * tt, b], egui::Stroke::new(1., linha));
    }
    let shift = (t * if animar { 0.25 } else { 0.12 }).fract();
    for k in 0..10 {
        let f = ((k as f32 + shift) / 10.).powf(1.8);
        let y = horizon + (rect.bottom() - horizon) * f;
        p.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(1., linha));
    }
    // partículas subindo
    let n = if animar { 64 } else { 36 };
    for i in 0..n {
        let h = (i as f32 * 12.9898).sin() * 43758.547;
        let fx = h.fract().abs();
        let speed = (0.015 + 0.03 * ((i * 7 % 11) as f32 / 11.)) * if animar { 1.8 } else { 1. };
        let fy = 1. - ((t * speed + fx * 3.7).fract());
        let x = rect.left() + rect.width() * fx + (t * 0.5 + i as f32).sin() * 12.;
        let y = rect.top() + rect.height() * fy;
        let a = (fy * (1. - fy) * 4.).clamp(0., 1.);
        let size = 1. + (i % 3) as f32 * 0.8;
        p.circle_filled(egui::pos2(x, y), size, crate::theme::com_alfa(pal.destaque, (a * if pal.escuro { 90. } else { 110. }) as u8));
    }
    if animar {
        // varredura: faixa de luz cruzando a tela de tempos em tempos
        let ciclo = (t / 9.).fract();
        if ciclo < 0.35 {
            let k = ciclo / 0.35;
            let x = rect.left() + rect.width() * k;
            let fade = (k * (1. - k) * 4.).clamp(0., 1.);
            for j in 0..6 {
                let w = 6. + j as f32 * 10.;
                let a = (10. * fade * (1. - j as f32 / 6.)) as u8;
                p.rect_filled(Rect::from_center_size(egui::pos2(x, rect.center().y), egui::vec2(w, rect.height())), 0., crate::theme::com_alfa(pal.destaque, a));
            }
        }
        // cubo de arame girando, atrás da marca
        let c = glow + egui::vec2(0., 20.);
        let (seg, _) = cubo_de_arame(t * 0.35, c, rect.height().min(rect.width()) * 0.22);
        let fio = crate::theme::com_alfa(pal.destaque, if pal.escuro { 46 } else { 60 });
        for [a, b] in seg {
            p.line_segment([a, b], egui::Stroke::new(1.2, fio));
        }
    }
}

/// As 12 arestas de um cubo girando em torno do eixo vertical, em projeção isométrica, centrado
/// em `c`, com meia-aresta `r`. Devolve também o retângulo que contém todos os pontos.
pub fn cubo_de_arame(giro: f32, c: Pos2, r: f32) -> (Vec<[Pos2; 2]>, Rect) {
    let (s, co) = giro.sin_cos();
    let (se, ce) = 0.62f32.sin_cos();
    let proj = |x: f32, y: f32, z: f32| {
        let (x1, y1) = (x * co - y * s, x * s + y * co);
        Pos2::new(c.x + x1 * r, c.y - (z * ce + y1 * se) * r)
    };
    let v = |i: usize| proj(if i & 1 == 0 { -1. } else { 1. }, if i & 2 == 0 { -1. } else { 1. }, if i & 4 == 0 { -1. } else { 1. });
    let mut seg = Vec::with_capacity(12);
    for i in 0..8usize {
        for bit in [1usize, 2, 4] {
            if i & bit == 0 {
                seg.push([v(i), v(i | bit)]);
            }
        }
    }
    let mut caixa = Rect::NOTHING;
    for [a, b] in &seg {
        caixa.extend_with(*a);
        caixa.extend_with(*b);
    }
    (seg, caixa)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn paleta_vem_do_tema_em_todos_os_temas() {
        for def in crate::theme::builtin() {
            let p = Paleta::de(&def);
            assert_eq!(p.escuro, def.dark, "{}", def.name);
            assert_eq!(p.texto, rgb(def.text), "{}", def.name);
            assert_eq!(p.destaque, rgb(def.accent), "{}", def.name);
            assert_eq!(p.cartao, rgb(def.window), "{}", def.name);
            // o texto do tema continua legível sobre o fundo da tela inicial
            for fundo in [p.topo, p.base] {
                let c = crate::acessibilidade::contraste([p.texto.r(), p.texto.g(), p.texto.b()], [fundo.r(), fundo.g(), fundo.b()]);
                assert!(c >= 4.5, "{}: contraste {c}", def.name);
            }
        }
    }

    #[test]
    fn peca_sempre_se_destaca_da_miniatura() {
        for def in crate::theme::builtin() {
            let p = Paleta::de(&def);
            for fundo in [p.mini_topo, p.mini_base] {
                let c = crate::acessibilidade::contraste(p.peca, [fundo.r(), fundo.g(), fundo.b()]);
                assert!(c >= 1.3, "{}: a peça some no fundo ({c})", def.name);
            }
        }
    }

    #[test]
    fn temas_diferentes_dao_fundos_diferentes() {
        let t = crate::theme::builtin();
        let escuro = t.iter().find(|d| d.dark).unwrap();
        let claro = t.iter().find(|d| !d.dark).unwrap();
        assert_ne!(Paleta::de(escuro).topo, Paleta::de(claro).topo);
        assert!(Paleta::de(escuro).topo.r() < Paleta::de(claro).topo.r());
    }

    #[test]
    fn movimento_respeita_reduzir_movimento_e_modo_leve() {
        assert!(movimento_permitido(true, false, false));
        assert!(!movimento_permitido(true, true, false));
        assert!(!movimento_permitido(true, false, true));
        assert!(!movimento_permitido(false, false, false));
    }

    #[test]
    fn cubo_tem_doze_arestas_e_cabe_no_raio() {
        let c = Pos2::new(100., 100.);
        for k in 0..20 {
            let (seg, caixa) = cubo_de_arame(k as f32 * 0.4, c, 50.);
            assert_eq!(seg.len(), 12);
            assert!(caixa.width() <= 50. * 2. * 1.8 && caixa.height() <= 50. * 2. * 1.8);
        }
    }
}
