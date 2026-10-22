//! Cubo de vistas: gira junto com a peça; clicar numa face ou num canto leva àquela vista.
//! A tecla Espaço abre o seletor grande com as vistas padrão e as vistas salvas (câmeras).

use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::Vec3;

/// Faces: (normal, nome)
const FACES: [([f32; 3], &str); 6] = [
    ([0., -1., 0.], "Frente"),
    ([0., 1., 0.], "Trás"),
    ([1., 0., 0.], "Direita"),
    ([-1., 0., 0.], "Esquerda"),
    ([0., 0., 1.], "Topo"),
    ([0., 0., -1.], "Base"),
];

/// Direção (do alvo para o olho) e nome das vistas padrão.
pub const STANDARD: [([f32; 3], &str); 7] = [
    ([0., -1., 0.], "Frente"),
    ([0., 1., 0.], "Trás"),
    ([0., 0., 1.], "Topo"),
    ([0., 0., -1.], "Base"),
    ([-1., 0., 0.], "Esquerda"),
    ([1., 0., 0.], "Direita"),
    ([0.577, -0.577, 0.577], "Isométrica"),
];

fn rot(cam: &Camera, v: Vec3) -> Vec3 {
    // só a rotação da câmera: vetor no espaço da vista
    let view = glam::camera::rh::view::look_at_mat4(cam.eye(), cam.target, cam_up(cam));
    view.transform_vector3(v)
}

fn cam_up(cam: &Camera) -> Vec3 {
    if cam.pitch.abs() > 1.55 {
        Vec3::new(-cam.yaw.cos(), -cam.yaw.sin(), 0.) * cam.pitch.signum()
    } else {
        Vec3::Z
    }
}

/// Desenha o cubo dentro de `area` e devolve a direção escolhida (se houve clique).
pub fn cube(ui: &mut egui::Ui, area: Rect, cam: &Camera, id: &str) -> Option<Vec3> {
    let resp = ui.interact(area, egui::Id::unique(("viewcube", id)), egui::Sense::click());
    let painter = ui.painter_at(area);
    let c = area.center();
    let s = area.width().min(area.height()) * 0.30;
    let to2 = |v: Vec3| -> Pos2 {
        let r = rot(cam, v);
        c + egui::vec2(r.x, -r.y) * s
    };
    let hover = resp.hover_pos();
    let mut chosen = None;
    let dark = ui.visuals().dark_mode;
    // faces de trás para frente
    let mut faces: Vec<(f32, [f32; 3], &str)> = FACES.iter().map(|(n, l)| (rot(cam, Vec3::from(*n)).z, *n, *l)).collect();
    faces.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (z, n, label) in faces {
        if z <= 0.02 {
            continue;
        }
        let n = Vec3::from(n);
        let (u, w) = if n.z.abs() > 0.5 { (Vec3::X, Vec3::Y) } else if n.x.abs() > 0.5 { (Vec3::Y, Vec3::Z) } else { (Vec3::X, Vec3::Z) };
        let corners: Vec<Pos2> = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)].iter().map(|(a, b)| to2(n + u * *a + w * *b)).collect();
        let shape_rect = Rect::from_points(&corners);
        let inside = hover.is_some_and(|h| shape_rect.contains(h) && point_in_quad(h, &corners));
        let base = if dark { Color32::from_rgb(62, 92, 140) } else { Color32::from_rgb(196, 222, 250) };
        let fill = if inside { Color32::from_rgb(255, 190, 60) } else { base.gamma_multiply(0.55 + 0.45 * z.clamp(0., 1.)) };
        painter.add(egui::Shape::convex_polygon(corners.clone(), fill, Stroke::new(1.2, Color32::from_rgb(40, 110, 210))));
        let center = to2(n);
        let fs = (s * 0.32).clamp(8., 16.);
        painter.text(center, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(fs), if dark { Color32::WHITE } else { Color32::from_rgb(20, 40, 80) });
        if inside && resp.clicked() {
            chosen = Some(n);
        }
    }
    // cantos: vistas isométricas
    for x in [-1f32, 1.] {
        for y in [-1f32, 1.] {
            for z in [-1f32, 1.] {
                let d = Vec3::new(x, y, z);
                if rot(cam, d).z <= 0.3 {
                    continue;
                }
                let p = to2(d);
                let near = hover.is_some_and(|h| (h - p).length() < 7.);
                painter.circle_filled(p, if near { 6. } else { 3.5 }, if near { Color32::from_rgb(255, 190, 60) } else { Color32::from_rgb(40, 110, 210) });
                if near && resp.clicked() {
                    chosen = Some(d.normalize());
                }
            }
        }
    }
    chosen
}

fn point_in_quad(p: Pos2, q: &[Pos2]) -> bool {
    let mut sign = 0.;
    for i in 0..q.len() {
        let (a, b) = (q[i], q[(i + 1) % q.len()]);
        let c = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0. {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}
