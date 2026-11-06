// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alças diretas na folha (B250). A vista selecionada ganha, sem abrir o painel:
//! - pega de mover no canto, com cadeado quando está presa à vista mãe pelo alinhamento;
//! - escala por passos (menos, valor, mais) na série preferida;
//! - corte: setas gordas nas pontas da linha de corte (um clique inverte o lado);
//! - detalhe: pega de raio no círculo (arrastar muda o raio ao vivo).
//! Arrastar uma vista mostra a linha de alinhamento e a cota da distância ao vizinho. As operações
//! vivem em `fenkra_desenho::alcas`; aqui só se desenha e se liga o mouse. Cores só do tema.

use super::*;
use crate::theme;
use fenkra_desenho::alcas as al;

/// Lado da área de clique das pegas (px).
const PEGA: f32 = 20.;

/// Começa um arraste que vira UM passo de desfazer: guarda o desenho de antes.
pub(super) fn begin_drag(st: &mut DwgState) {
    if st.drag_snap.is_none() {
        st.drag_snap = Some(st.d.clone());
    }
    st.dragging = true;
}

/// Termina o arraste: se algo mudou, o desenho de antes entra na pilha de desfazer (um passo só,
/// por maior que seja o caminho do mouse) e o refazer é limpo.
pub(super) fn end_drag(st: &mut DwgState) {
    st.dragging = false;
    st.drag_kind = 0;
    if let Some(antes) = st.drag_snap.take() {
        if antes != st.d {
            st.undo.push(antes);
            if st.undo.len() > 100 {
                st.undo.remove(0);
            }
            st.redo.clear();
            st.last = st.d.clone();
            st.modified = true;
            st.invalidate();
        }
    }
}

fn pega_cheia(painter: &egui::Painter, c: Pos2, cor: Color32) {
    painter.circle_filled(c, 6., cor);
    painter.circle_stroke(c, 6., Stroke::new(1.5, Color32::BLACK));
}

/// Cadeado fechado (corpo e argola) centrado em `c`.
fn cadeado(painter: &egui::Painter, c: Pos2, cor: Color32) {
    let corpo = Rect::from_center_size(c + Vec2::new(0., 2.), Vec2::new(10., 7.));
    painter.rect_filled(corpo, 1.5, cor);
    painter.rect_stroke(corpo, 1.5, Stroke::new(1.2, Color32::BLACK), egui::StrokeKind::Outside);
    let topo = corpo.top();
    let argola: Vec<Pos2> = [(-3., 0.), (-3., -3.5), (-1.5, -5.), (1.5, -5.), (3., -3.5), (3., 0.)].iter().map(|(x, y)| Pos2::new(c.x + x, topo + y)).collect();
    painter.add(egui::Shape::line(argola, Stroke::new(1.8, Color32::BLACK)));
}

/// Seta gorda de ponta `tip`, apontando para `dir` (vetor de tela unitário).
fn seta_gorda(painter: &egui::Painter, tip: Pos2, dir: Vec2, cor: Color32) {
    let perp = Vec2::new(-dir.y, dir.x);
    let base = tip - dir * 13.;
    let tri = vec![tip, base + perp * 8., base - perp * 8.];
    painter.add(egui::Shape::convex_polygon(tri, cor, Stroke::new(1.5, Color32::BLACK)));
}

/// Desenha e trata as alças da vista em foco. `to_mm` converte tela → folha.
pub(super) fn show(ui: &egui::Ui, painter: &egui::Painter, st: &mut DwgState, to_scr: &dyn Fn(V2) -> Pos2, to_mm: &dyn Fn(Pos2) -> V2, z: f32) {
    let Some(id) = st.cur() else { return };
    let Some(v) = st.d.view(id).cloned() else { return };
    if v.sheet != st.sheet {
        return;
    }
    let cor = theme::accent(ui.visuals());
    let texto = ui.visuals().text_color();
    let fundo = ui.visuals().window_fill;
    let p = st.models.proj(&st.d, id);
    let (lo, hi) = layout::view_rect(&v, &p);
    let mut movendo = st.dragging && st.drag_kind == 0 && st.cur() == Some(id);
    let sid = ui.scope_id();

    // pega de mover (canto superior esquerdo) e cadeado de alinhamento
    let tl = to_scr([lo[0], hi[1]]);
    let c = tl + Vec2::new(-15., -15.);
    painter.rect_filled(Rect::from_center_size(c, Vec2::splat(16.)), 3., cor);
    painter.rect_stroke(Rect::from_center_size(c, Vec2::splat(16.)), 3., Stroke::new(1.5, Color32::BLACK), egui::StrokeKind::Outside);
    for d in [Vec2::X, Vec2::Y] {
        painter.line_segment([c - d * 6., c + d * 6.], Stroke::new(1.6, Color32::WHITE));
    }
    let r = ui.interact(Rect::from_center_size(c, Vec2::splat(PEGA)), sid.with(("alca_mover", id.0)), egui::Sense::click_and_drag()).on_hover_text(tr("Arraste para mover a vista"));
    if r.drag_started() {
        begin_drag(st);
        st.drag_kind = 0;
    }
    if r.dragged() && st.dragging {
        let dv = r.drag_delta();
        move_item(st, id, [(dv.x / z) as f64, (-dv.y / z) as f64]);
        movendo = true;
    }
    if r.drag_stopped() {
        end_drag(st);
    }
    if let Some((mae, no_x)) = al::travada_na_mae(&st.d, id) {
        let lc = c + Vec2::new(22., 0.);
        painter.rect_filled(Rect::from_center_size(lc, Vec2::new(20., 18.)), 3., fundo);
        cadeado(painter, lc, cor);
        let nome = st.d.view(mae).map(|m| m.name.clone()).unwrap_or_default();
        let dica = if no_x { tr("Alinhada na vertical com a vista mãe") } else { tr("Alinhada na horizontal com a vista mãe") };
        ui.interact(Rect::from_center_size(lc, Vec2::new(20., 18.)), sid.with(("alca_cadeado", id.0)), egui::Sense::hover()).on_hover_text(format!("{dica}: {nome}"));
    }

    // escala por passos
    // ao lado do canto inferior direito, fora do caminho do rótulo da vista (que fica centrado embaixo)
    let base = to_scr([hi[0], lo[1]]) + Vec2::new(6., -20.);
    let celulas = [(-1, "−", tr("Reduz um passo na série de escalas")), (0, "", tr("Escala da vista")), (1, "+", tr("Amplia um passo na série de escalas"))];
    let largura = [20., 52., 20.];
    let mut x = base.x;
    for ((passo, rot, dica), w) in celulas.iter().zip(largura) {
        let rc = Rect::from_min_size(Pos2::new(x, base.y), Vec2::new(w, 20.));
        x += w;
        painter.rect_filled(rc, 3., fundo);
        painter.rect_stroke(rc, 3., Stroke::new(1.5, cor), egui::StrokeKind::Inside);
        let txt = if *passo == 0 { scale_text(v.scale) } else { rot.to_string() };
        painter.text(rc.center(), egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(12.), texto);
        let rr = ui.interact(rc, sid.with(("alca_escala", id.0, *passo)), egui::Sense::click()).on_hover_text(if al::vista_da_escala(&st.d, id) != Some(id) && *passo != 0 { format!("{dica} ({})", tr("vale para a vista de origem")) } else { dica.to_string() });
        if *passo != 0 && rr.clicked() {
            al::passo_de_escala(&mut st.d, id, *passo);
        }
    }

    // corte: setas gordas nas pontas da linha de corte (cada ponta olha para o seu segmento)
    if let ViewKind::Section { parent, .. } = &v.kind {
        if let Some(pv) = st.d.view(*parent).cloned() {
            let pp = st.models.proj(&st.d, pv.id);
            let path: Vec<V2> = fenkra_desenho::corte::caminho_da_vista(&st.d, id).into_iter().map(|q| layout::to_sheet(&pv, &pp, q)).collect();
            let n = path.len();
            if n >= 2 {
                let dir = |a: V2, b: V2| {
                    let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                    (l > 1e-9).then(|| [(b[0] - a[0]) / l, (b[1] - a[1]) / l])
                };
                let pontas = [(path[0], dir(path[0], path[1]), -1.), (path[n - 1], dir(path[n - 2], path[n - 1]), 1.)];
                for (e, u, s) in pontas {
                    let Some(u) = u else { continue };
                    let nn = [u[1], -u[0]];
                    let ns = Vec2::new(nn[0] as f32, -(nn[1] as f32));
                    let e2 = [e[0] + u[0] * s * 4., e[1] + u[1] * s * 4.];
                    let tip = to_scr(e2) + ns * 14.;
                    seta_gorda(painter, tip, ns, cor);
                    let rr = ui.interact(Rect::from_center_size(tip - ns * 7., Vec2::splat(PEGA + 6.)), sid.with(("alca_corte", id.0, s as i32)), egui::Sense::click()).on_hover_text(tr("Clique para inverter o lado do corte"));
                    if rr.clicked() {
                        al::inverter_corte(&mut st.d, id);
                    }
                }
            }
        }
        // vértices e pontos do meio da linha (B106)
        corte_ui::vertices(ui, painter, st, id, to_scr, to_mm, z);
    }

    // detalhe em perfil livre: vértices do contorno arrastáveis (B106)
    if matches!(v.kind, ViewKind::Detail { .. }) && !v.detalhe.contorno.is_empty() {
        corte_ui::vertices_detalhe(ui, painter, st, id, to_scr, to_mm);
    }
    // detalhe: pega de raio no círculo (o perfil livre não tem raio)
    if let (ViewKind::Detail { parent, center, radius, .. }, true) = (&v.kind, v.detalhe.contorno.is_empty()) {
        if let Some(pv) = st.d.view(*parent).cloned() {
            let pp = st.models.proj(&st.d, pv.id);
            let s0 = layout::to_sheet(&pv, &pp, *center);
            let hs = [s0[0] + radius * pv.scale, s0[1]];
            let hc = to_scr(hs);
            pega_cheia(painter, hc, cor);
            let rr = ui.interact(Rect::from_center_size(hc, Vec2::splat(PEGA)), sid.with(("alca_raio", id.0)), egui::Sense::click_and_drag()).on_hover_text(tr("Arraste para mudar o raio do detalhe"));
            if rr.drag_started() {
                begin_drag(st);
            }
            if rr.dragged() && st.dragging {
                if let Some(m) = rr.interact_pointer_pos().map(to_mm) {
                    let nr = ((m[0] - s0[0]).powi(2) + (m[1] - s0[1]).powi(2)).sqrt() / pv.scale.max(1e-9);
                    al::definir_raio_detalhe(&mut st.d, id, nr);
                }
            }
            if rr.dragged() || rr.hovered() {
                painter.text(hc + Vec2::new(10., -10.), egui::Align2::LEFT_BOTTOM, format!("R{}", st.d.num(*radius)), egui::FontId::proportional(12.), texto);
            }
            if rr.drag_stopped() {
                end_drag(st);
            }
        }
    }

    // arrastando: linha de alinhamento e cota da distância ao vizinho
    if movendo {
        let fraca = ui.visuals().weak_text_color();
        let tol = 0.3;
        for (a, b) in al::alinhamentos(&st.d, id, tol) {
            painter.extend(egui::Shape::dashed_line(&[to_scr(a), to_scr(b)], Stroke::new(1.4, cor), 7., 4.));
            for q in [a, b] {
                painter.circle_filled(to_scr(q), 3., cor);
            }
        }
        let outras: Vec<(Id, V2, V2)> = st.boxes.iter().filter(|(i, _, _)| st.d.view(*i).is_some_and(|o| o.sheet == st.sheet)).cloned().collect();
        let (mlo, mhi) = outras.iter().find(|o| o.0 == id).map(|o| (o.1, o.2)).unwrap_or((lo, hi));
        if let Some(n) = al::vizinho_mais_perto(id, mlo, mhi, &outras) {
            let (a, b) = (to_scr(n.de), to_scr(n.ate));
            painter.extend(egui::Shape::dashed_line(&[a, b], Stroke::new(1.2, fraca), 4., 3.));
            for q in [a, b] {
                painter.circle_filled(q, 2.5, fraca);
            }
            let meio = a + (b - a) / 2.;
            let g = painter.layout_no_wrap(format!("{} mm", st.d.num(n.dist)), egui::FontId::proportional(12.), texto);
            let rc = Rect::from_center_size(meio, g.size() + Vec2::new(10., 4.));
            painter.rect_filled(rc, 3., fundo);
            painter.rect_stroke(rc, 3., Stroke::new(1., cor), egui::StrokeKind::Inside);
            painter.galley(rc.min + Vec2::new(5., 2.), g, texto);
        }
    }
    let _ = to_mm;
}
