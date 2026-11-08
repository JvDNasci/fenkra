// SPDX-License-Identifier: LGPL-2.1-or-later
//! Animação curta do comando na dica, feita pelo próprio programa (B377).
//!
//! Sem vídeo e sem pacote de mídia: cada família de comando tem uma cena de poucas formas (um
//! retângulo que cresce, um furo que abre, uma dobra que gira) calculada por um tempo de 0 a 1 que
//! se repete. A cena é só geometria em coordenadas de 0 a 1; quem desenha escolhe as cores do tema.
//! Uma opção das configurações troca a dica inteira pela versão simples, sem passo a passo nem
//! animação.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use std::sync::atomic::{AtomicBool, Ordering};

static SIMPLES: AtomicBool = AtomicBool::new(false);
static REDUZIDO: AtomicBool = AtomicBool::new(false);

/// Publica as escolhas das configurações a cada quadro: dica simples e movimento reduzido.
pub fn definir(simples: bool, reduzir_movimento: bool) {
    SIMPLES.store(simples, Ordering::Relaxed);
    REDUZIDO.store(reduzir_movimento, Ordering::Relaxed);
}

/// A dica simples está ligada?
pub fn simples() -> bool {
    SIMPLES.load(Ordering::Relaxed)
}

/// Papel de cada forma, para o desenho escolher a cor no tema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Papel {
    /// o que já existe
    Base,
    /// o que o comando produz
    Resultado,
    /// linha de apoio (eixo, plano de espelho, seta)
    Guia,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Forma {
    Poligono(Vec<[f32; 2]>, Papel, bool),
    Circulo([f32; 2], f32, Papel, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cena {
    Extrusao,
    Revolucao,
    Furo,
    Arredondar,
    Chanfro,
    Casca,
    Padrao,
    Espelho,
    Dobra,
    /// cota entre dois alvos: os dois elementos acendem e a linha de cota abre entre eles (B185)
    Cota,
    /// raio ou diâmetro: a linha sai do centro até a borda
    CotaRaio,
    /// ângulo: o arco abre entre as duas arestas
    CotaAngulo,
    Generica,
}

/// A família das cotas (PMI 3D e desenho): todas mostram a mesma ideia, escolher os alvos e ver a
/// medida aparecer, cada uma com o seu traço.
fn cena_de_cota(n: &str) -> Option<Cena> {
    let cota = n.starts_with("cota_") || n.starts_with("des_cota") || matches!(n, "pmi_linear" | "pmi_raio" | "pmi_diametro" | "pmi_angulo" | "pmi_ordenada" | "pmi_cadeia" | "pmi_base" | "pmi_arco" | "pmi_feature" | "sk_cota" | "sk_auto_cota");
    if !cota {
        return None;
    }
    Some(if n.contains("raio") || n.contains("diametro") || n.ends_with("_r") || n.ends_with("_d") || n == "pmi_feature" {
        Cena::CotaRaio
    } else if n.contains("ang") || n == "pmi_arco" {
        Cena::CotaAngulo
    } else {
        Cena::Cota
    })
}

/// Cena de um comando, pelo nome do ícone.
pub fn cena_do_icone(icone: &str) -> Cena {
    let n = icone.to_lowercase();
    let tem = |t: &[&str]| t.iter().any(|x| n.contains(x));
    if let Some(c) = cena_de_cota(&n) {
        c
    } else if tem(&["revol", "giro"]) {
        Cena::Revolucao
    } else if tem(&["extru", "aba_", "nervura", "protrus"]) || n == "aba" {
        Cena::Extrusao
    } else if tem(&["furo", "hole", "rebaixo", "rasgo"]) {
        Cena::Furo
    } else if tem(&["chanfro"]) {
        Cena::Chanfro
    } else if tem(&["filete", "arestas_selecionar", "raio"]) {
        Cena::Arredondar
    } else if tem(&["casca", "shell", "espessura"]) {
        Cena::Casca
    } else if tem(&["padrao", "pattern", "matriz"]) {
        Cena::Padrao
    } else if tem(&["espelho", "mirror", "simetria"]) {
        Cena::Espelho
    } else if tem(&["dobra", "desdobrar", "bend", "sm_"]) {
        Cena::Dobra
    } else {
        Cena::Generica
    }
}

/// Ritmo da repetição: o quadro vai de 0 a 1 em 2,2 s, pára um instante no fim e recomeça.
pub const CICLO_S: f64 = 3.0;
const SUBIDA_S: f64 = 2.2;

/// Posição 0..1 da cena no tempo `t` (segundos), com folga parada no fim e suavização.
pub fn progresso(t: f64) -> f32 {
    let x = (t.rem_euclid(CICLO_S) / SUBIDA_S).clamp(0., 1.) as f32;
    x * x * (3. - 2. * x)
}

fn retangulo(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<[f32; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

/// Formas da cena no instante `p` (0..1), todas dentro do quadrado unitário.
pub fn quadro(c: Cena, p: f32) -> Vec<Forma> {
    use Forma::{Circulo, Poligono};
    use Papel::{Base, Guia, Resultado};
    let p = p.clamp(0., 1.);
    match c {
        Cena::Extrusao => {
            // perfil no chão e o prisma subindo, com a tampa de cima acompanhando
            let (x0, x1, base) = (0.22, 0.62, 0.80);
            let alt = 0.50 * p;
            let dx = 0.18;
            let dy = 0.12;
            let topo = base - alt;
            vec![
                Poligono(vec![[x0, base], [x1, base], [x1 + dx, base - dy], [x0 + dx, base - dy]], Base, false),
                Poligono(vec![[x0, topo], [x1, topo], [x1, base], [x0, base]], Resultado, true),
                Poligono(vec![[x1, topo], [x1 + dx, topo - dy], [x1 + dx, base - dy], [x1, base]], Resultado, true),
                Poligono(vec![[x0, topo], [x1, topo], [x1 + dx, topo - dy], [x0 + dx, topo - dy]], Resultado, true),
                Poligono(vec![[0.12, base], [0.12, 0.30]], Guia, false),
            ]
        }
        Cena::Revolucao => {
            // meio perfil à direita do eixo e o corpo abrindo para a esquerda
            let eixo = 0.5;
            let largura = 0.28;
            vec![
                Poligono(vec![[eixo, 0.1], [eixo, 0.9]], Guia, false),
                Poligono(retangulo(eixo, 0.25, eixo + largura, 0.75), Base, false),
                Poligono(retangulo(eixo - largura * p, 0.25, eixo, 0.75), Resultado, true),
                Circulo([eixo, 0.25], 0.03 + largura * p * 0.5, Resultado, false),
            ]
        }
        Cena::Furo => {
            // a placa e o furo abrindo, com a seta da broca descendo
            let r = 0.16 * p;
            vec![
                Poligono(retangulo(0.15, 0.45, 0.85, 0.80), Base, false),
                Circulo([0.5, 0.62], r, Resultado, true),
                Poligono(vec![[0.5, 0.12 + 0.2 * p], [0.5, 0.40]], Guia, false),
            ]
        }
        Cena::Arredondar | Cena::Chanfro => {
            // o canto vivo é trocado por um arco (filete) ou por um corte reto (chanfro)
            let (x0, y0, x1, y1) = (0.2, 0.2, 0.8, 0.8);
            let r = 0.22 * p;
            let mut pts = vec![[x0, y0], [x1, y0], [x1, y1 - r]];
            if c == Cena::Arredondar {
                for i in 0..=8 {
                    let a = i as f32 / 8. * std::f32::consts::FRAC_PI_2;
                    pts.push([x1 - r + r * a.cos(), y1 - r + r * a.sin()]);
                }
            } else {
                pts.push([x1 - r, y1]);
            }
            pts.push([x0, y1]);
            vec![Poligono(retangulo(x0, y0, x1, y1), Guia, false), Poligono(pts, Resultado, true)]
        }
        Cena::Casca => {
            let ins = 0.16 * p;
            vec![Poligono(retangulo(0.2, 0.2, 0.8, 0.8), Base, false), Poligono(retangulo(0.2 + ins, 0.2 + ins, 0.8 - ins, 0.8 - ins), Resultado, true)]
        }
        Cena::Padrao => {
            // a semente e as cópias que vão aparecendo em fila
            let n = 1 + (p * 4.0).floor() as usize;
            let mut v = vec![Circulo([0.2, 0.5], 0.08, Base, true)];
            for i in 1..n.min(5) {
                v.push(Circulo([0.2 + 0.15 * i as f32, 0.5], 0.08, Resultado, true));
            }
            v
        }
        Cena::Espelho => {
            // o triângulo da esquerda e a cópia que se abre do outro lado do plano
            let eixo = 0.5;
            let d = 0.08 + 0.22 * p;
            vec![
                Poligono(vec![[eixo, 0.1], [eixo, 0.9]], Guia, false),
                Poligono(vec![[eixo - 0.08, 0.3], [eixo - 0.30, 0.7], [eixo - 0.08, 0.7]], Base, true),
                Poligono(vec![[eixo + 0.08, 0.3], [eixo + d + 0.0, 0.7], [eixo + 0.08, 0.7]], Resultado, true),
            ]
        }
        Cena::Dobra => {
            // faixa plana e a aba da direita girando em torno da linha de dobra
            let a = p * std::f32::consts::FRAC_PI_2;
            let (hx, hy) = (0.55, 0.70);
            let comp = 0.30;
            vec![
                Poligono(retangulo(0.12, hy - 0.04, hx, hy), Base, true),
                Poligono(vec![[hx, hy], [hx + comp * a.cos(), hy - comp * a.sin()], [hx + comp * a.cos(), hy - comp * a.sin() - 0.04], [hx, hy - 0.04]], Resultado, true),
                Poligono(vec![[hx, 0.15], [hx, 0.85]], Guia, false),
            ]
        }
        Cena::Cota => {
            // a peça, os dois alvos acesos e a linha de cota abrindo entre eles, com as setas
            let (a, b, y) = (0.22, 0.78, 0.30);
            let m = 0.5;
            let meia = (b - a) / 2. * p;
            let mut v = vec![
                Poligono(retangulo(a, 0.50, b, 0.85), Base, false),
                Poligono(vec![[a, 0.50], [a, y - 0.06]], Guia, false),
                Poligono(vec![[b, 0.50], [b, y - 0.06]], Guia, false),
                Circulo([a, 0.50], 0.035, Resultado, true),
                Circulo([b, 0.50], 0.035, Resultado, true),
            ];
            if p > 0.02 {
                v.push(Poligono(vec![[m - meia, y], [m + meia, y]], Resultado, false));
                v.push(Poligono(vec![[m - meia, y], [m - meia + 0.05, y - 0.03], [m - meia + 0.05, y + 0.03]], Resultado, true));
                v.push(Poligono(vec![[m + meia, y], [m + meia - 0.05, y - 0.03], [m + meia - 0.05, y + 0.03]], Resultado, true));
            }
            v
        }
        Cena::CotaRaio => {
            // o furo aceso e a linha de cota saindo do centro até a borda
            let (c, r) = ([0.42, 0.55], 0.25);
            let ang: f32 = -0.6;
            let fim = [c[0] + r * ang.cos() * p, c[1] + r * ang.sin() * p];
            let mut v = vec![Circulo(c, r, Base, false), Circulo(c, 0.02, Guia, true)];
            if p > 0.02 {
                v.push(Poligono(vec![c, fim], Resultado, false));
                v.push(Poligono(vec![fim, [0.86, 0.12 + 0.18 * (1. - p)]], Guia, false));
            }
            v
        }
        Cena::CotaAngulo => {
            // duas arestas que se encontram e o arco da cota abrindo de uma até a outra
            let (o, r) = ([0.18, 0.82], 0.45);
            let fim: f32 = -0.85;
            let mut v = vec![Poligono(vec![o, [0.92, 0.82]], Base, false), Poligono(vec![o, [o[0] + 0.62 * fim.cos(), o[1] + 0.62 * fim.sin()]], Base, false)];
            let n = 12;
            let arco: Vec<[f32; 2]> = (0..=n).map(|i| fim * p * i as f32 / n as f32).map(|a| [o[0] + r * a.cos(), o[1] + r * a.sin()]).collect();
            if p > 0.02 {
                v.push(Poligono(arco, Resultado, false));
            }
            v
        }
        Cena::Generica => {
            // clique no item e o resultado aparecendo
            vec![
                Poligono(retangulo(0.2, 0.3, 0.8, 0.7), Base, false),
                Poligono(retangulo(0.2, 0.3, 0.2 + 0.6 * p, 0.7), Resultado, true),
                Circulo([0.2 + 0.6 * p, 0.5], 0.04, Guia, true),
            ]
        }
    }
}

fn cor(papel: Papel, v: &egui::Visuals) -> Color32 {
    match papel {
        Papel::Base => v.widgets.noninteractive.fg_stroke.color,
        Papel::Resultado => crate::theme::accent(v),
        Papel::Guia => v.weak_text_color(),
    }
}

/// Desenha a cena do comando `icone` no tempo `t`, num quadro de `lado` pixels.
pub fn desenhar(ui: &mut egui::Ui, icone: &str, t: f64, lado: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(lado, lado * 0.6), egui::Sense::hover());
    let v = ui.visuals().clone();
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 6., v.extreme_bg_color);
    let pos = |q: [f32; 2]| -> Pos2 { Rect::from_min_size(rect.min, rect.size()).min + egui::vec2(q[0] * rect.width(), q[1] * rect.height()) };
    let escala = rect.height();
    // movimento reduzido: a cena pára no resultado, sem repetir
    let reduzido = REDUZIDO.load(Ordering::Relaxed);
    let prog = if reduzido { 1. } else { progresso(t) };
    for f in quadro(cena_do_icone(icone), prog) {
        match f {
            Forma::Poligono(pts, papel, cheio) => {
                let c = cor(papel, &v);
                let pts: Vec<Pos2> = pts.into_iter().map(pos).collect();
                if pts.len() == 2 {
                    p.line_segment([pts[0], pts[1]], Stroke::new(1.5, c));
                } else if cheio {
                    p.add(egui::Shape::convex_polygon(pts, c.gamma_multiply(0.35), Stroke::new(1.5, c)));
                } else {
                    p.add(egui::Shape::closed_line(pts, Stroke::new(1.5, c)));
                }
            }
            Forma::Circulo(c0, r, papel, cheio) => {
                let c = cor(papel, &v);
                let r = r * escala;
                if cheio {
                    p.circle(pos(c0), r, c.gamma_multiply(0.35), Stroke::new(1.5, c));
                } else {
                    p.circle_stroke(pos(c0), r, Stroke::new(1.5, c));
                }
            }
        }
    }
    if !reduzido {
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const TODAS: [Cena; 13] = [Cena::Extrusao, Cena::Revolucao, Cena::Furo, Cena::Arredondar, Cena::Chanfro, Cena::Casca, Cena::Padrao, Cena::Espelho, Cena::Dobra, Cena::Cota, Cena::CotaRaio, Cena::CotaAngulo, Cena::Generica];

    fn pontos(f: &Forma) -> Vec<[f32; 2]> {
        match f {
            Forma::Poligono(p, ..) => p.clone(),
            Forma::Circulo(c, r, ..) => vec![[c[0] - r, c[1] - r], [c[0] + r, c[1] + r]],
        }
    }

    #[test]
    fn toda_cena_anima_de_fato_e_cabe_no_quadro() {
        for c in TODAS {
            let (a, b) = (quadro(c, 0.0), quadro(c, 1.0));
            assert_ne!(a, b, "{c:?} não muda entre o começo e o fim");
            for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
                for f in quadro(c, p) {
                    for q in pontos(&f) {
                        assert!((-0.001..=1.001).contains(&q[0]) && (-0.001..=1.001).contains(&q[1]), "{c:?} p={p} sai do quadro: {q:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn icone_leva_a_cena_da_sua_familia() {
        assert_eq!(cena_do_icone("extrusao"), Cena::Extrusao);
        assert_eq!(cena_do_icone("revolucao"), Cena::Revolucao);
        assert_eq!(cena_do_icone("furo_simples"), Cena::Furo);
        assert_eq!(cena_do_icone("chanfro"), Cena::Chanfro);
        assert_eq!(cena_do_icone("arestas_selecionar"), Cena::Arredondar);
        assert_eq!(cena_do_icone("sm_desdobrar"), Cena::Dobra);
        assert_eq!(cena_do_icone("qualquer_coisa"), Cena::Generica);
    }

    /// B185: toda cota do catálogo (PMI 3D, desenho e sketch) tem a animação de cota na dica, e o
    /// raio não cai mais na cena do filete por causa do nome.
    #[test]
    fn toda_cota_do_catalogo_tem_animacao_de_cota() {
        for n in ["pmi_linear", "pmi_ordenada", "pmi_cadeia", "pmi_base", "des_cota_h", "des_cota_v", "des_cota_al", "des_cota_inteligente", "cota_horizontal", "sk_cota"] {
            assert_eq!(cena_do_icone(n), Cena::Cota, "{n}");
        }
        for n in ["pmi_raio", "pmi_diametro", "pmi_feature", "des_cota_r", "des_cota_d", "cota_raio", "cota_diametro"] {
            assert_eq!(cena_do_icone(n), Cena::CotaRaio, "{n}");
        }
        for n in ["pmi_angulo", "pmi_arco", "des_cota_ang", "cota_angulo"] {
            assert_eq!(cena_do_icone(n), Cena::CotaAngulo, "{n}");
        }
        assert_eq!(cena_do_icone("filete"), Cena::Arredondar);
    }

    #[test]
    fn progresso_sobe_para_no_fim_e_recomeca() {
        assert_eq!(progresso(0.0), 0.0);
        assert!((progresso(1.1) - 0.5).abs() < 1e-3);
        assert_eq!(progresso(2.6), 1.0, "parado no fim do ciclo");
        assert!(progresso(CICLO_S + 0.01) < 0.01, "recomeça");
    }

    #[test]
    fn dica_simples_liga_e_desliga() {
        definir(true, false);
        assert!(simples());
        definir(false, false);
        assert!(!simples());
    }
}
