// SPDX-License-Identifier: LGPL-2.1-or-later
//! Nervura e rasgo varridos (B301): com o comando aberto, a trajetória fica realçada, o perfil
//! aparece no ponto inicial e a orientação escolhida (acompanha, fixa, Frenet) é demonstrada por
//! pares de setas ao longo do caminho. As setas usam a mesma regra do kernel: "acompanha" transporta
//! o perfil sem torcer, "fixa" nunca gira o perfil e "Frenet" segue a curvatura da guia.

use crate::app::App;
use crate::render::Camera;
use eframe::egui::{self, Pos2, Stroke};
use fenkra_core::{FeatureKind, Id, SweepMode};
use glam::Vec3;

/// Quantos quadros de orientação aparecem ao longo do caminho.
pub const QUADROS: usize = 7;

/// Orientação do perfil num ponto do caminho: `a` é o eixo X do perfil e `b` o eixo Y.
#[derive(Clone, Copy, Debug)]
pub struct Quadro {
    pub p: Vec3,
    pub t: Vec3,
    pub a: Vec3,
    pub b: Vec3,
}

fn perpendicular(v: Vec3, t: Vec3) -> Option<Vec3> {
    (v - t * v.dot(t)).try_normalize()
}

/// Ponto e tangente a `u` (0 a 1) do comprimento da polilinha.
fn amostra(pts: &[Vec3], u: f32) -> Option<(Vec3, Vec3)> {
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    if pts.len() < 2 || total < 1e-6 {
        return None;
    }
    let mut falta = u.clamp(0., 1.) * total;
    for w in pts.windows(2) {
        let l = (w[1] - w[0]).length();
        if l < 1e-9 {
            continue;
        }
        if falta <= l {
            return Some((w[0] + (w[1] - w[0]) * (falta / l), (w[1] - w[0]) / l));
        }
        falta -= l;
    }
    let n = pts.len();
    Some((pts[n - 1], (pts[n - 1] - pts[n - 2]).try_normalize()?))
}

/// Comprimento da polilinha.
pub fn comprimento(pts: &[Vec3]) -> f32 {
    pts.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

/// Quadros de orientação ao longo do caminho para o modo escolhido. `ex` e `ey` são os eixos do
/// plano do perfil no mundo.
pub fn quadros(pts: &[Vec3], modo: SweepMode, ex: Vec3, ey: Vec3, n: usize) -> Vec<Quadro> {
    let n = n.max(2);
    let mut out: Vec<Quadro> = vec![];
    let mut anterior: Option<Vec3> = None;
    for i in 0..n {
        let u = i as f32 / (n - 1) as f32;
        let Some((p, t)) = amostra(pts, u) else { return vec![] };
        let a = match modo {
            SweepMode::Fixo => ex,
            SweepMode::Corrigido => {
                // transporte sem torção: o eixo anterior perde só a parte que acompanha a nova tangente
                let base = anterior.unwrap_or(ex);
                perpendicular(base, t).or_else(|| perpendicular(ey, t)).unwrap_or(base)
            }
            SweepMode::Frenet => {
                // direção da curvatura: variação da tangente em volta do ponto
                let du = 0.5 / (n - 1) as f32;
                let curv = match (amostra(pts, (u - du).max(0.)), amostra(pts, (u + du).min(1.))) {
                    (Some((_, t0)), Some((_, t1))) => perpendicular(t1 - t0, t),
                    _ => None,
                };
                curv.or_else(|| anterior.and_then(|v| perpendicular(v, t))).or_else(|| perpendicular(ex, t)).unwrap_or(ex)
            }
        };
        anterior = Some(a);
        let b = if modo == SweepMode::Fixo { ey } else { t.cross(a).try_normalize().unwrap_or(ey) };
        out.push(Quadro { p, t, a, b });
    }
    out
}

/// Ângulo (graus) entre a normal do perfil e a tangente da guia no início: zero é o perfil normal à curva.
pub fn desvio_do_perfil(normal: Vec3, tangente: Vec3) -> f32 {
    let c = normal.normalize_or_zero().dot(tangente.normalize_or_zero()).abs().clamp(0., 1.);
    c.acos().to_degrees()
}

fn ponta(painter: &egui::Painter, de: Pos2, para: Pos2, cor: egui::Color32, largura: f32) {
    let v = para - de;
    if v.length() < 2. {
        return;
    }
    let d = v.normalized();
    let n = egui::vec2(-d.y, d.x);
    painter.line_segment([de, para], Stroke::new(largura, cor));
    painter.add(egui::Shape::convex_polygon(vec![para + d * 7., para + n * 4. - d * 2., para - n * 4. - d * 2.], cor, Stroke::NONE));
}

fn texto(ui: &egui::Ui, rect: egui::Rect, at: Pos2, t: String, cor: egui::Color32) {
    let painter = ui.painter_at(rect);
    let g = painter.layout_no_wrap(t, egui::FontId::proportional(12.), cor);
    let caixa = egui::Rect::from_min_size(at + egui::vec2(10., -10.), g.size() + egui::vec2(8., 4.));
    painter.rect_filled(caixa, 3., ui.visuals().extreme_bg_color.gamma_multiply(0.9));
    painter.galley(caixa.min + egui::vec2(4., 2.), g, cor);
}

/// Desenha a trajetória, o perfil no início e as setas de orientação do comando `id`.
pub fn desenhar(app: &App, ui: &egui::Ui, rect: egui::Rect, cam: &Camera, id: Id) {
    let (Some(f), Some(r)) = (app.open.doc.feature(id), app.result.as_ref()) else { return };
    let FeatureKind::Sweep(w) = &f.kind else { return };
    let Some(pts) = crate::picking::curve_pts(r, &w.spine) else { return };
    let pts: Vec<Vec3> = pts.iter().map(|q| Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32)).collect();
    if pts.len() < 2 {
        return;
    }
    let guia = crate::theme::st_warn();
    let destaque = crate::theme::accent(ui.visuals());
    let setas = crate::theme::papel_neutro();
    let painter = ui.painter_at(rect);

    // trajetória realçada, com a ponta de seta no fim
    let tela: Vec<Pos2> = pts.iter().filter_map(|p| cam.project(rect, *p)).collect();
    if tela.len() >= 2 {
        painter.add(egui::Shape::line(tela.clone(), Stroke::new(4., guia)));
        let (a, b) = (tela[tela.len() - 2], tela[tela.len() - 1]);
        ponta(&painter, a, b, guia, 4.);
    }
    // perfil no ponto inicial (onde a varredura começa) e o laço de volta, se estiver longe
    let inicio = pts[0];
    if let Some(sk) = r.sketches.iter().find(|s| s.feature == w.sketch) {
        let n = Vec3::new(sk.frame.normal[0] as f32, sk.frame.normal[1] as f32, sk.frame.normal[2] as f32);
        let ex = Vec3::new(sk.frame.x[0] as f32, sk.frame.x[1] as f32, sk.frame.x[2] as f32);
        let ey = n.cross(ex);
        let mut soma = Vec3::ZERO;
        let mut cont = 0;
        for par in sk.lines.chunks_exact(2) {
            let (a, b) = (Vec3::from(par[0]), Vec3::from(par[1]));
            soma += a + b;
            cont += 2;
            if let (Some(sa), Some(sb)) = (cam.project(rect, a), cam.project(rect, b)) {
                painter.line_segment([sa, sb], Stroke::new(3., destaque));
            }
        }
        if cont > 0 {
            let centro = soma / cont as f32;
            if (centro - inicio).length() > 0.5 {
                if let (Some(sa), Some(sb)) = (cam.project(rect, centro), cam.project(rect, inicio)) {
                    painter.extend(egui::Shape::dashed_line(&[sa, sb], Stroke::new(1.5, destaque), 6., 4.));
                }
            }
        }
        // orientação demonstrada ao longo do caminho
        let tam = (comprimento(&pts) * 0.07).clamp(2., 40.);
        for q in quadros(&pts, w.mode, ex, ey, QUADROS) {
            if let Some(sp) = cam.project(rect, q.p) {
                if let Some(sa) = cam.project(rect, q.p + q.a * tam) {
                    ponta(&painter, sp, sa, setas, 2.4);
                }
                if let Some(sb) = cam.project(rect, q.p + q.b * tam * 0.6) {
                    ponta(&painter, sp, sb, setas.gamma_multiply(0.65), 1.6);
                }
            }
        }
    }
    if let Some(s0) = cam.project(rect, inicio) {
        painter.circle_filled(s0, 5., guia);
        texto(ui, rect, s0, crate::i18n::tr("Início da guia").to_string(), guia);
    }
    if let Some(sf) = pts.last().and_then(|p| cam.project(rect, *p)) {
        texto(ui, rect, sf, crate::i18n::tr("Fim da guia").to_string(), guia);
    }
    let modo = match w.mode {
        SweepMode::Corrigido => "acompanha: o perfil segue a curva sem torcer",
        SweepMode::Fixo => "fixa: o perfil não gira",
        SweepMode::Frenet => "Frenet: o perfil segue a curvatura e pode torcer",
    };
    if let Some((p, _)) = amostra(&pts, 0.5) {
        if let Some(sm) = cam.project(rect, p) {
            texto(ui, rect, sm + egui::vec2(0., 28.), crate::i18n::tr(modo).to_string(), setas);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Caminho que sai pelo eixo X, sobe e depois vira para o lado: curva com torção.
    fn caminho() -> Vec<Vec3> {
        (0..=140)
            .map(|i| {
                let t = i as f32 * 0.5;
                Vec3::new(t, 10. * (1. - (t / 9.).cos()), 10. * (1. - (t / 14.).cos()))
            })
            .collect()
    }

    #[test]
    fn fixa_nunca_gira_e_acompanha_gira_com_a_curva() {
        let pts = caminho();
        let (ex, ey) = (Vec3::Y, Vec3::Z);
        let fixo = quadros(&pts, SweepMode::Fixo, ex, ey, QUADROS);
        assert_eq!(fixo.len(), QUADROS);
        assert!(fixo.iter().all(|q| (q.a - ex).length() < 1e-6 && (q.b - ey).length() < 1e-6), "fixa não gira");
        let seguido = quadros(&pts, SweepMode::Corrigido, ex, ey, QUADROS);
        // cada eixo fica perpendicular à tangente (o perfil continua normal ao caminho)
        assert!(seguido.iter().all(|q| q.a.dot(q.t).abs() < 1e-4 && q.b.dot(q.t).abs() < 1e-4 && q.a.dot(q.b).abs() < 1e-4));
        let ultimo = seguido.last().unwrap();
        assert!((ultimo.a - ex).length() > 0.05, "o eixo acompanha a curva e muda de direção");
    }

    #[test]
    fn frenet_difere_do_transporte_numa_curva_com_torcao() {
        let pts = caminho();
        let (ex, ey) = (Vec3::Y, Vec3::Z);
        let corr = quadros(&pts, SweepMode::Corrigido, ex, ey, QUADROS);
        let fren = quadros(&pts, SweepMode::Frenet, ex, ey, QUADROS);
        // ângulo entre os eixos dos dois modos no fim: com torção o Frenet gira à parte
        let c = corr.last().unwrap().a.dot(fren.last().unwrap().a).clamp(-1., 1.).acos().to_degrees();
        assert!(c > 5., "Frenet e transporte coincidiram ({c:.1}°): o exemplo não mostra torção");
        assert!(fren.iter().all(|q| q.a.dot(q.t).abs() < 1e-3));
    }

    #[test]
    fn reta_nao_tem_curvatura_e_o_frenet_nao_quebra() {
        let reta: Vec<Vec3> = (0..5).map(|i| Vec3::new(i as f32 * 10., 0., 0.)).collect();
        let q = quadros(&reta, SweepMode::Frenet, Vec3::Y, Vec3::Z, QUADROS);
        assert_eq!(q.len(), QUADROS);
        assert!(q.iter().all(|x| x.a.is_finite() && (x.a - Vec3::Y).length() < 1e-4));
    }

    #[test]
    fn desvio_do_perfil_mede_o_quanto_a_normal_foge_da_tangente() {
        assert!(desvio_do_perfil(Vec3::X, Vec3::X) < 1e-3);
        assert!(desvio_do_perfil(-Vec3::X, Vec3::X) < 1e-3, "o sentido não importa");
        assert!((desvio_do_perfil(Vec3::Z, Vec3::X) - 90.).abs() < 1e-3);
        assert!((desvio_do_perfil(Vec3::new(1., 1., 0.), Vec3::X) - 45.).abs() < 1e-3);
    }
}
