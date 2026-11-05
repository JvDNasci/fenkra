// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: desenho da seção do perfil (B254). Miniatura em escala, seção com o nome de cada cota
//! sobre o desenho e os nove pontos mais o centróide da ancoragem para clicar. Só cores do tema.

use crate::i18n::tr;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use fenkra_core::SectionKind;
use fenkra_kernel::{Loop2, Seg2};
use fenkra_solidos::structural::{self, anchor_point};

/// Cor de preenchimento e de contorno da peça no desenho: o acento do tema sobre o fundo do painel.
fn cores(ui: &egui::Ui) -> (Color32, Color32) {
    let a = crate::theme::accent(ui.visuals());
    (a.gamma_multiply(0.45), a)
}

/// Contorno do laço em pontos (círculos viram polígonos de 48 lados; arcos e splines, pelos pontos
/// que os definem).
pub fn poligono(l: &Loop2) -> Vec<[f64; 2]> {
    let mut v = vec![];
    for s in l {
        match s {
            Seg2::Line(a, _) => v.push(*a),
            Seg2::Ellipse(c, m, r, t0, t1) => {
                let g = m[0].hypot(m[1]).max(1e-12);
                let (ux, uy) = (m[0] / g, m[1] / g);
                v.extend((0..48).map(|i| {
                    let t = t0 + (t1 - t0) * i as f64 / 48.;
                    [c[0] + g * t.cos() * ux - r * t.sin() * uy, c[1] + g * t.cos() * uy + r * t.sin() * ux]
                }));
            }
            Seg2::Arc(a, b, _) => {
                v.push(*a);
                v.push(*b)
            }
            Seg2::Circle(c, r) => {
                v.extend((0..48).map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / 48.;
                    [c[0] + r * t.cos(), c[1] + r * t.sin()]
                }));
            }
            Seg2::Spline(p) => v.extend(p.iter().take(p.len().saturating_sub(1)).copied()),
        }
    }
    v
}

fn area2(p: &[[f64; 2]]) -> f64 {
    (0..p.len()).map(|i| p[i][0] * p[(i + 1) % p.len()][1] - p[(i + 1) % p.len()][0] * p[i][1]).sum()
}

/// Triangulação por corte de orelhas de um polígono simples (côncavo serve: perfis I, U, T e L).
pub fn triangular(poly: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return vec![];
    }
    let mut idx: Vec<usize> = (0..n).collect();
    if area2(poly) < 0. {
        idx.reverse();
    }
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let dentro = |p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| cross(a, b, p) >= -1e-12 && cross(b, c, p) >= -1e-12 && cross(c, a, p) >= -1e-12;
    let mut out = vec![];
    let mut guarda = 0;
    while idx.len() > 3 && guarda < n * n {
        guarda += 1;
        let m = idx.len();
        let mut cortou = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if cross(a, b, c) <= 1e-12 {
                continue;
            }
            if idx.iter().any(|&k| k != ia && k != ib && k != ic && dentro(poly[k], a, b, c)) {
                continue;
            }
            out.push([ia, ib, ic]);
            idx.remove(i);
            cortou = true;
            break;
        }
        if !cortou {
            break;
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// Área do preenchimento (laços de fora menos furos), para conferir o desenho contra a seção.
pub fn area_desenhada(loops: &[Loop2]) -> f64 {
    loops
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let p = poligono(l);
            let a: f64 = triangular(&p).iter().map(|t| area2(&[p[t[0]], p[t[1]], p[t[2]]]).abs() / 2.).sum();
            if i == 0 {
                a
            } else {
                -a
            }
        })
        .sum()
}

/// Mapeia coordenadas da seção (mm, y para cima) para a tela dentro de `area`, com `px_por_mm`.
struct Mapa {
    centro: Pos2,
    px_por_mm: f32,
}

impl Mapa {
    fn tela(&self, p: [f64; 2]) -> Pos2 {
        Pos2::new(self.centro.x + p[0] as f32 * self.px_por_mm, self.centro.y - p[1] as f32 * self.px_por_mm)
    }
}

fn pintar(ui: &egui::Ui, mapa: &Mapa, loops: &[Loop2]) {
    let (cheio, contorno) = cores(ui);
    let painter = ui.painter();
    for (i, l) in loops.iter().enumerate() {
        let p = poligono(l);
        let cor = if i == 0 { cheio } else { ui.visuals().extreme_bg_color };
        let pts: Vec<Pos2> = p.iter().map(|q| mapa.tela(*q)).collect();
        let mut mesh = egui::Mesh::default();
        for q in &pts {
            mesh.colored_vertex(*q, cor);
        }
        for t in triangular(&p) {
            mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32);
        }
        painter.add(egui::Shape::mesh(mesh));
        painter.add(egui::Shape::closed_line(pts, Stroke::new(1.3, contorno)));
    }
}

/// Maior lado (mm) da caixa da seção: base da escala comum de uma família.
pub fn lado_maior(kind: SectionKind, dims: &[f64]) -> Option<f64> {
    structural::section(kind, dims).ok().map(|(_, w, h, _)| w.max(h))
}

/// Miniatura da seção numa célula quadrada de `lado_px`. `escala_mm` é o maior lado da família: com a
/// mesma escala, os tamanhos aparecem na proporção real uns dos outros. Devolve a resposta (clique).
pub fn miniatura(ui: &mut egui::Ui, kind: SectionKind, dims: &[f64], escala_mm: f64, lado_px: f32, marcada: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(lado_px, lado_px), egui::Sense::click());
    let fundo = if marcada { ui.visuals().selection.bg_fill } else { ui.visuals().faint_bg_color };
    ui.painter().rect_filled(rect, 3., fundo);
    ui.painter().rect_stroke(rect, 3., Stroke::new(if marcada { 2. } else { 1. }, if marcada { crate::theme::accent(ui.visuals()) } else { ui.visuals().widgets.noninteractive.bg_stroke.color }), egui::StrokeKind::Inside);
    match structural::section(kind, dims) {
        Ok((loops, _, _, _)) => {
            let ppm = ((lado_px - 8.) as f64 / escala_mm.max(1e-6)) as f32;
            pintar(ui, &Mapa { centro: rect.center(), px_por_mm: ppm }, &loops);
        }
        Err(e) => {
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "⚠", egui::FontId::proportional(lado_px * 0.5), crate::theme::st_err());
            return resp.on_hover_text(e);
        }
    }
    resp
}

/// Seção maior com o nome e o valor de cada cota sobre o desenho e, se `ancora` for dado, os nove
/// pontos e o centróide para clicar. Devolve a ancoragem clicada, se houve clique.
pub fn secao_com_cotas(ui: &mut egui::Ui, kind: SectionKind, dims: &[f64], ancora: Option<u8>, clicavel: bool) -> Option<u8> {
    let tamanho = egui::vec2(ui.available_width().clamp(200., 330.), 190.);
    let (rect, _) = ui.allocate_exact_size(tamanho, egui::Sense::hover());
    ui.painter().rect_filled(rect, 4., ui.visuals().faint_bg_color);
    let Ok((loops, w, h, cen)) = structural::section(kind, dims) else {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, tr("medida inválida"), egui::FontId::proportional(13.), crate::theme::st_err());
        return None;
    };
    let (_, contorno) = cores(ui);
    let miolo = rect.shrink2(egui::vec2(46., 34.));
    let ppm = (miolo.width() as f64 / w.max(1e-6)).min(miolo.height() as f64 / h.max(1e-6)) as f32;
    let mapa = Mapa { centro: miolo.center(), px_por_mm: ppm };
    pintar(ui, &mapa, &loops);
    let texto = ui.visuals().text_color();
    let painter = ui.painter();
    // cotas: linha com marcas nas pontas e o nome com o valor ao lado
    let nomes = structural::dim_names(kind);
    let marcas = structural::dimension_marks(kind, dims);
    let mut fila_direita = 0;
    for c in &marcas {
        let (a, b) = (mapa.tela(c.a), mapa.tela(c.b));
        let txt = format!("{} {}", tr(c.name), crate::app::fmt(c.value));
        let g = painter.layout_no_wrap(txt, egui::FontId::proportional(11.), texto);
        if c.overall {
            // fora da caixa: horizontal acima, vertical à esquerda
            let horizontal = (c.a[1] - c.b[1]).abs() < 1e-9;
            let (a, b, pos) = if horizontal {
                let off = egui::vec2(0., -12.);
                (a + off, b + off, Pos2::new((a.x + b.x) / 2. - g.size().x / 2., a.y - 14. - g.size().y))
            } else {
                let off = egui::vec2(-12., 0.);
                (a + off, b + off, Pos2::new(a.x - 14. - g.size().x, (a.y + b.y) / 2. - g.size().y / 2.))
            };
            painter.line_segment([a, b], Stroke::new(1., contorno));
            for q in [a, b] {
                let t = if horizontal { egui::vec2(0., 4.) } else { egui::vec2(4., 0.) };
                painter.line_segment([q - t, q + t], Stroke::new(1., contorno));
            }
            painter.galley(pos, g, texto);
        } else {
            // espessura: segmento sobre a peça e o nome numa fila à direita, com linha de chamada
            painter.line_segment([a, b], Stroke::new(2.5, crate::theme::st_warn()));
            let destino = Pos2::new(rect.right() - 6. - g.size().x, rect.top() + 8. + fila_direita as f32 * 15.);
            fila_direita += 1;
            painter.line_segment([(a + b.to_vec2()) / 2., Pos2::new(destino.x - 3., destino.y + g.size().y / 2.)], Stroke::new(0.8, crate::theme::st_warn()));
            painter.galley(destino, g, texto);
        }
    }
    debug_assert_eq!(nomes.len(), dims.len().min(nomes.len()));
    let ancora = ancora?;
    // os nove pontos da caixa e o centróide
    let mut clicado = None;
    for a in 0u8..=9 {
        let q = mapa.tela(anchor_point(a, w, h, cen));
        let r = Rect::from_center_size(q, egui::vec2(16., 16.));
        let resp = ui.interact(r, egui::Id::unique(("st_ancora", a)), if clicavel { egui::Sense::click() } else { egui::Sense::hover() });
        let ativo = a == ancora;
        let cor = if ativo { crate::theme::st_warn() } else if resp.hovered() { crate::theme::accent(ui.visuals()) } else { texto };
        let p = ui.painter();
        if a == 9 {
            p.line_segment([q - egui::vec2(5., 0.), q + egui::vec2(5., 0.)], Stroke::new(2., cor));
            p.line_segment([q - egui::vec2(0., 5.), q + egui::vec2(0., 5.)], Stroke::new(2., cor));
        } else if ativo {
            p.circle_filled(q, 5., cor);
        } else {
            p.circle_stroke(q, 4., Stroke::new(1.5, cor));
        }
        let dica = if a == 9 { tr("centróide").to_string() } else if a == 0 { tr("centro da caixa").to_string() } else { tr("ponto da seção que fica sobre a linha guia").to_string() };
        resp.clone().on_hover_text(dica);
        if resp.clicked() {
            clicado = Some(a);
        }
    }
    clicado
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangulacao_cobre_a_area_da_secao_de_toda_familia_de_fabrica() {
        for fam in structural::builtin() {
            for (n, d) in &fam.sizes {
                let (loops, ..) = structural::section(fam.kind, d).unwrap();
                let desenhada = area_desenhada(&loops);
                // a área exata: tubos = caixa − miolo; perfis conhecidos pelas fórmulas do perfil
                let esperada = match fam.kind {
                    SectionKind::TuboQuadrado => d[0] * d[0] - (d[0] - 2. * d[1]).powi(2),
                    SectionKind::TuboRetangular => d[0] * d[1] - (d[0] - 2. * d[2]) * (d[1] - 2. * d[2]),
                    SectionKind::BarraChata => d[0] * d[1],
                    SectionKind::BarraQuadrada => d[0] * d[0],
                    SectionKind::PerfilI => 2. * d[1] * d[3] + (d[0] - 2. * d[3]) * d[2],
                    SectionKind::PerfilU => d[2] * d[0] + 2. * (d[1] - d[2]) * d[3],
                    SectionKind::PerfilT => d[1] * d[3] + (d[0] - d[3]) * d[2],
                    SectionKind::Cantoneira => d[1] * d[2] + (d[0] - d[2]) * d[2],
                    // círculos viram polígonos de 48 lados: perde menos de 0,2 % da área
                    SectionKind::TuboRedondo => std::f64::consts::PI * ((d[0] / 2.).powi(2) - (d[0] / 2. - d[1]).powi(2)),
                    SectionKind::BarraRedonda => std::f64::consts::PI * (d[0] / 2.).powi(2),
                };
                assert!((desenhada - esperada).abs() <= 0.005 * esperada, "{} {n}: desenhada {desenhada} contra {esperada}", fam.name);
            }
        }
    }

    #[test]
    fn orelhas_em_poligono_concavo() {
        // L: 6 vértices, área 3 (2x2 menos um canto de 1x1)
        let l = [[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]];
        let t = triangular(&l);
        assert_eq!(t.len(), 4);
        let soma: f64 = t.iter().map(|i| area2(&[l[i[0]], l[i[1]], l[i[2]]]).abs() / 2.).sum();
        assert!((soma - 3.).abs() < 1e-9);
        // sentido horário dá o mesmo resultado
        let mut r = l;
        r.reverse();
        let soma: f64 = triangular(&r).iter().map(|i| area2(&[r[i[0]], r[i[1]], r[i[2]]]).abs() / 2.).sum();
        assert!((soma - 3.).abs() < 1e-9);
        assert!(triangular(&[[0., 0.], [1., 0.]]).is_empty());
    }

    #[test]
    fn toda_familia_de_fabrica_tem_miniatura() {
        for fam in structural::builtin() {
            assert!(!fam.sizes.is_empty(), "{}", fam.name);
            let maior = fam.sizes.iter().filter_map(|(_, d)| lado_maior(fam.kind, d)).fold(0., f64::max);
            assert!(maior > 0., "{}", fam.name);
            for (n, d) in &fam.sizes {
                let (loops, w, h, _) = structural::section(fam.kind, d).unwrap();
                assert!(area_desenhada(&loops) > 0. && w <= maior && h <= maior, "{} {n}", fam.name);
            }
        }
    }
}
