// SPDX-License-Identifier: LGPL-2.1-or-later
//! Eixo gráfico (B281): perfil lateral 2D ligado à grade, alças 3D nas arestas dos trechos e o menu
//! "Adicionar recurso" da seção (rasgo, anel elástico, ranhura, rosca, furo transversal).
//!
//! O eixo continua sendo o `ParamEixo` da janela de engrenagens; os recursos de seção ficam em
//! `GearsUi::recursos` e entram no documento por `eixo_documento_com_recursos`. Selecionar um
//! trecho (no desenho, nos botões ou numa alça) destaca o desenho e mostra os recursos dele.

use crate::app::{fmt, App};
use crate::asm_ui::{AsmState, GearsUi};
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use fenkra_kernel::Mesh;
use fenkra_solidos::eixos_recursos::{eixo_documento_com_recursos, intervalo_z, passo_grosso, posicao_na_outra_borda, Borda, RecursoEixo, TipoRecurso};
use fenkra_solidos::{recompute, ParamEixo};
use glam::Vec3;
use std::sync::Arc;

/// Opacidade do eixo fantasma.
const ALFA: f32 = 0.45;

/// Malha do eixo em memória, para a chave dos parâmetros que a geram.
pub struct MalhaEixo {
    pub chave: String,
    pub malha: Mesh,
}

fn chave(g: &GearsUi) -> String {
    format!("{:?}|{:?}", g.eixo, g.recursos)
}

/// Refaz a malha do eixo quando os parâmetros mudam (espera o mouse soltar durante o arrasto).
/// Devolve se a cena precisa ser refeita.
pub fn atualizar_malha(g: &mut GearsUi, ctx: &egui::Context) -> bool {
    if !g.eixo_grafico {
        let tinha = g.malha_eixo.is_some();
        g.malha_eixo = None;
        return tinha;
    }
    let k = chave(g);
    if g.malha_eixo.as_ref().is_some_and(|m| m.chave == k) {
        return false;
    }
    if ctx.input(|i| i.pointer.any_down()) && g.malha_eixo.is_some() {
        return false;
    }
    g.malha_eixo = eixo_documento_com_recursos(&g.eixo, &g.recursos)
        .ok()
        .and_then(|d| recompute(&d).bodies.into_iter().next())
        .map(|b| Arc::new(MalhaEixo { chave: k, malha: b.mesh }));
    true
}

/// O eixo como fantasma na cena, ao longo de Z a partir da origem.
pub fn na_cena(st: Option<&AsmState>, v: &egui::Visuals, cena: &mut crate::render::Scene) {
    let Some(g) = st.and_then(|s| s.gears_open.as_ref()).filter(|g| g.eixo_grafico) else { return };
    let Some(m) = g.malha_eixo.as_ref() else { return };
    let c = crate::theme::accent(v);
    crate::previa::malha_fantasma(&mut cena.ghost, &m.malha, [c.r(), c.g(), c.b()], ALFA);
}

// ---------------------------------------------------------------- perfil lateral

/// Retângulo de cada trecho no desenho lateral: eixo horizontal (z para a direita), centrado na
/// altura. O desenho cabe em `area` com a mesma escala nos dois sentidos.
pub fn retangulos_do_perfil(p: &ParamEixo, area: Rect) -> Vec<Rect> {
    let total = p.comprimento_total().max(1e-6);
    let dmax = p.trechos.iter().map(|t| t.diametro).fold(1e-6, f64::max);
    let escala = ((area.width() as f64 - 12.) / total).min((area.height() as f64 - 12.) / dmax).max(1e-6);
    let mut x = area.left() + 6. + ((area.width() as f64 - 12. - total * escala) / 2.) as f32;
    let cy = area.center().y;
    p.trechos
        .iter()
        .map(|t| {
            let (w, h) = ((t.comprimento * escala) as f32, (t.diametro * escala) as f32);
            let r = Rect::from_min_size(Pos2::new(x, cy - h / 2.), Vec2::new(w, h));
            x += w;
            r
        })
        .collect()
}

/// Retângulo de um recurso dentro do retângulo do trecho (faixa na borda de cima).
fn retangulo_do_recurso(p: &ParamEixo, rec: &RecursoEixo, trecho: Rect) -> Option<Rect> {
    let t = p.trechos.get(rec.trecho)?;
    let (z0, z1) = intervalo_z(p, rec)?;
    let inicio: f64 = p.trechos[..rec.trecho].iter().map(|t| t.comprimento).sum();
    let k = trecho.width() as f64 / t.comprimento.max(1e-9);
    let x0 = trecho.left() + ((z0 - inicio) * k) as f32;
    let x1 = trecho.left() + ((z1 - inicio) * k) as f32;
    let alto = match rec.tipo {
        TipoRecurso::FuroTransversal { .. } => trecho.height() * 0.5,
        _ => (trecho.height() * 0.22).max(4.),
    };
    let topo = if matches!(rec.tipo, TipoRecurso::FuroTransversal { .. }) { trecho.center().y - alto / 2. } else { trecho.top() };
    Some(Rect::from_min_max(Pos2::new(x0, topo), Pos2::new(x1.max(x0 + 2.), topo + alto)))
}

/// Desenha o perfil e devolve o trecho clicado.
pub fn perfil_lateral(ui: &mut egui::Ui, g: &mut GearsUi) {
    let w = ui.available_width().min(520.);
    let (resp, painter) = ui.allocate_painter(egui::vec2(w, 120.), egui::Sense::click());
    let resp = crate::teclado::anel(ui, resp);
    let v = ui.visuals().clone();
    painter.rect_filled(resp.rect, 4., v.extreme_bg_color);
    let rects = retangulos_do_perfil(&g.eixo, resp.rect);
    let (acento, texto, suave) = (crate::theme::accent(&v), v.text_color(), v.weak_text_color());
    // linha de centro
    painter.line_segment([Pos2::new(resp.rect.left() + 2., resp.rect.center().y), Pos2::new(resp.rect.right() - 2., resp.rect.center().y)], Stroke::new(1., suave));
    for (i, r) in rects.iter().enumerate() {
        let sel = g.trecho_sel == Some(i);
        painter.rect_filled(*r, 0., if sel { acento.gamma_multiply(0.35) } else { v.faint_bg_color });
        painter.rect_stroke(*r, 0., Stroke::new(if sel { 2. } else { 1.2 }, if sel { acento } else { texto }), egui::StrokeKind::Inside);
        painter.text(r.center_bottom() + Vec2::new(0., 12.), egui::Align2::CENTER_CENTER, format!("Ø{} × {}", fmt(g.eixo.trechos[i].diametro), fmt(g.eixo.trechos[i].comprimento)), egui::FontId::proportional(11.), if sel { acento } else { suave });
    }
    for rec in &g.recursos {
        let Some(tr_rect) = rects.get(rec.trecho) else { continue };
        if let Some(rr) = retangulo_do_recurso(&g.eixo, rec, *tr_rect) {
            let sel = g.trecho_sel == Some(rec.trecho);
            painter.rect_filled(rr, 1., if sel { acento } else { texto }.gamma_multiply(0.55));
            painter.rect_stroke(rr, 1., Stroke::new(1., texto), egui::StrokeKind::Inside);
        }
    }
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            if let Some(i) = rects.iter().position(|r| r.x_range().contains(p.x)) {
                g.trecho_sel = Some(i);
            }
        }
    }
    if let Some(p) = resp.hover_pos() {
        if let Some(i) = rects.iter().position(|r| r.x_range().contains(p.x)) {
            resp.on_hover_text(format!("{} {} · Ø{} · L {}", tr("Trecho"), i + 1, fmt(g.eixo.trechos[i].diametro), fmt(g.eixo.trechos[i].comprimento)));
        }
    }
}

// ---------------------------------------------------------------- recursos

/// Tira o trecho `i` e ajusta os recursos: os dele saem, os dos trechos seguintes recuam um índice.
pub fn remover_trecho(recursos: &mut Vec<RecursoEixo>, i: usize) {
    recursos.retain(|r| r.trecho != i);
    for r in recursos.iter_mut() {
        if r.trecho > i {
            r.trecho -= 1;
        }
    }
}

/// Menu "Adicionar recurso" e a lista de recursos do trecho selecionado.
pub fn recursos_do_trecho(ui: &mut egui::Ui, g: &mut GearsUi) {
    let Some(i) = g.trecho_sel.filter(|i| *i < g.eixo.trechos.len()) else {
        ui.label(RichText::new(tr("Clique num trecho do desenho para adicionar recursos nele.")).weak().small());
        return;
    };
    let t = g.eixo.trechos[i];
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} {} · Ø{}", tr("Trecho"), i + 1, fmt(t.diametro))).strong());
        ui.menu_button(tr("Adicionar recurso"), |ui| {
            for nome in TipoRecurso::NOMES {
                if ui.button(tr(nome)).clicked() {
                    g.recursos.push(RecursoEixo { trecho: i, borda: Borda::Esquerda, posicao: 0., tipo: TipoRecurso::padrao(nome, t.diametro) });
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr("Rasgo de chaveta, anel elástico, ranhura, rosca ou furo transversal neste trecho, com a posição medida da borda esquerda ou direita."));
    });
    let mut apagar = None;
    for (k, r) in g.recursos.iter_mut().enumerate().filter(|(_, r)| r.trecho == i) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr(r.tipo.nome())).strong());
            let antes = r.borda;
            ui.selectable_value(&mut r.borda, Borda::Esquerda, tr("da esquerda"));
            ui.selectable_value(&mut r.borda, Borda::Direita, tr("da direita"));
            if antes != r.borda {
                // troca de referência sem mover o recurso
                r.posicao = posicao_na_outra_borda(t.comprimento, r.tipo.extensao(), r.posicao).max(0.);
            }
            ui.add(crate::ui_ext::dv(&mut r.posicao).range(0.0..=10000.).speed(0.5).max_decimals(3).suffix(" mm").dica(tr("Distância da borda escolhida do trecho até a face do recurso mais próxima dela.")));
            match &mut r.tipo {
                TipoRecurso::Rasgo { comprimento, rotacao } => {
                    ui.add(crate::ui_ext::dv(comprimento).range(0.1..=10000.).speed(0.5).prefix("L ").suffix(" mm"));
                    ui.add(crate::ui_ext::dv(rotacao).range(-360.0..=360.).speed(1.).prefix("↻ ").suffix("°").dica(tr("Giro do rasgo em torno do eixo; 0° é o lado +X.")));
                }
                TipoRecurso::Anel { largura, profundidade } | TipoRecurso::Ranhura { largura, profundidade } => {
                    ui.add(crate::ui_ext::dv(largura).range(0.1..=1000.).speed(0.1).prefix("m ").suffix(" mm"));
                    ui.add(crate::ui_ext::dv(profundidade).range(0.05..=100.).speed(0.05).prefix("p ").suffix(" mm"));
                }
                TipoRecurso::Rosca { comprimento, passo } => {
                    ui.add(crate::ui_ext::dv(comprimento).range(0.1..=10000.).speed(0.5).prefix("L ").suffix(" mm"));
                    ui.add(crate::ui_ext::dv(passo).range(0.1..=20.).speed(0.05).prefix("P ").suffix(" mm"));
                    if let Some(p) = passo_grosso(t.diametro) {
                        ui.label(RichText::new(format!("{} {}", tr("passo grosso"), fmt(p))).weak().small());
                    }
                }
                TipoRecurso::FuroTransversal { diametro, rotacao } => {
                    ui.add(crate::ui_ext::dv(diametro).range(0.1..=500.).speed(0.1).prefix("Ø ").suffix(" mm"));
                    ui.add(crate::ui_ext::dv(rotacao).range(-360.0..=360.).speed(1.).prefix("↻ ").suffix("°"));
                }
            }
            if ui.small_button("✖").on_hover_text(tr("Remover recurso")).clicked() {
                apagar = Some(k);
            }
        });
    }
    if let Some(k) = apagar {
        g.recursos.remove(k);
    }
    if let Err(e) = fenkra_solidos::eixos_recursos::validar_recursos(&g.eixo, &g.recursos) {
        ui.label(RichText::new(format!("⚠ {e}")).small().color(crate::theme::st_warn()));
    }
}

// ---------------------------------------------------------------- alças 3D

/// Novo comprimento do trecho a partir do alvo do arrasto: múltiplos de 0,5 mm, nunca abaixo de 1.
pub fn comprimento_arrastado(alvo: f64) -> f64 {
    ((alvo * 2.).round() / 2.).clamp(1., 10_000.)
}

/// Novo diâmetro a partir do raio arrastado: o diâmetro anda de 1 em 1 mm (raio em 0,5), mínimo 2.
pub fn diametro_arrastado(raio_alvo: f64) -> f64 {
    ((raio_alvo * 2.).round()).clamp(2., 1000.)
}

/// Alças nas arestas dos trechos: no fim de cada trecho (arrastar ao longo do eixo muda L) e na
/// lateral do meio dele (arrastar para fora muda Ø). Arrastar seleciona o trecho.
pub fn alcas(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(g) = app.asm.as_mut().and_then(|s| s.gears_open.as_mut()).filter(|g| g.eixo_grafico) else { return };
    let v = ui.visuals().clone();
    let (cor, fundo) = (crate::theme::accent(&v), v.window_fill().gamma_multiply(0.85));
    let p = ui.painter_at(rect);
    let proj = |q: [f64; 3]| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
    let ctx = ui.ctx().clone();
    let mut z = 0.;
    for i in 0..g.eixo.trechos.len() {
        let t = g.eixo.trechos[i];
        let (z0, z1) = (z, z + t.comprimento);
        z = z1;
        let sel = g.trecho_sel == Some(i);
        // fim do trecho: muda L
        if let (Some(a), Some(b)) = (proj([0., 0., z1 - 1.]), proj([0., 0., z1])) {
            let n = proj([0., 0., z1]).zip(proj([0., 0., z1 - 10.])).map(|(x, y)| x - y).unwrap_or(b - a);
            let px_mm = n.length() / 10.;
            let id = egui::Id::new(("alca_eixo_l", i));
            let r = ui.interact(Rect::from_center_size(b, egui::vec2(18., 18.)), id, egui::Sense::click_and_drag());
            let c = if r.hovered() || r.dragged() || sel { crate::theme::st_warn() } else { cor };
            p.circle_filled(b, 6., c);
            p.circle_stroke(b, 6., Stroke::new(1.5, v.window_stroke().color));
            if r.hovered() || r.dragged() {
                let g_ = p.layout_no_wrap(format!("L = {}", fmt(t.comprimento)), egui::FontId::proportional(12.), c);
                p.rect_filled(Rect::from_min_size(b + Vec2::new(10., -22.), g_.size()).expand(2.), 3., fundo);
                p.galley(b + Vec2::new(10., -22.), g_, c);
            }
            if r.drag_started() {
                ctx.data_mut(|d| d.insert_temp(id, t.comprimento));
                g.trecho_sel = Some(i);
            }
            if r.dragged() {
                let alvo = crate::alca_cota::alvo_do_arrasto(ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(t.comprimento), r.drag_delta(), n, px_mm);
                ctx.data_mut(|d| d.insert_temp(id, alvo));
                g.eixo.trechos[i].comprimento = comprimento_arrastado(alvo);
            }
        }
        // lateral do meio: muda Ø
        let zm = (z0 + z1) / 2.;
        let raio = t.diametro / 2.;
        if let (Some(c0), Some(c1)) = (proj([raio, 0., zm]), proj([raio + 10., 0., zm])) {
            let n = c1 - c0;
            let px_mm = n.length() / 10.;
            let id = egui::Id::new(("alca_eixo_d", i));
            let r = ui.interact(Rect::from_center_size(c0, egui::vec2(18., 18.)), id, egui::Sense::click_and_drag());
            let c = if r.hovered() || r.dragged() { crate::theme::st_warn() } else { cor };
            p.rect_filled(Rect::from_center_size(c0, egui::vec2(11., 11.)), 2., c);
            if r.hovered() || r.dragged() {
                let g_ = p.layout_no_wrap(format!("Ø = {}", fmt(t.diametro)), egui::FontId::proportional(12.), c);
                p.rect_filled(Rect::from_min_size(c0 + Vec2::new(10., -22.), g_.size()).expand(2.), 3., fundo);
                p.galley(c0 + Vec2::new(10., -22.), g_, c);
            }
            if r.drag_started() {
                ctx.data_mut(|d| d.insert_temp(id, raio));
                g.trecho_sel = Some(i);
            }
            if r.dragged() {
                let alvo = crate::alca_cota::alvo_do_arrasto(ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(raio), r.drag_delta(), n, px_mm);
                ctx.data_mut(|d| d.insert_temp(id, alvo));
                g.eixo.trechos[i].diametro = diametro_arrastado(alvo);
            }
        }
    }
    let _ = Color32::TRANSPARENT;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_solidos::TrechoEixo;

    fn eixo() -> ParamEixo {
        ParamEixo { trechos: vec![TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None }, TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None }], furo_centro: false }
    }

    #[test]
    fn o_perfil_lateral_segue_a_grade_em_escala_e_ordem() {
        let area = Rect::from_min_size(Pos2::new(0., 0.), Vec2::new(512., 120.));
        let r = retangulos_do_perfil(&eixo(), area);
        assert_eq!(r.len(), 2);
        // comprimentos proporcionais a 50:70 e diâmetros a 20:25, lado a lado
        assert!(((r[0].width() / r[1].width()) - 50. / 70.).abs() < 1e-4);
        assert!(((r[0].height() / r[1].height()) - 20. / 25.).abs() < 1e-4);
        assert!((r[0].right() - r[1].left()).abs() < 1e-3);
        assert!(r.iter().all(|x| area.contains_rect(*x)));
        // mudar L na grade muda o desenho
        let mut e = eixo();
        e.trechos[0].comprimento = 100.;
        let r2 = retangulos_do_perfil(&e, area);
        assert!(((r2[0].width() / r2[1].width()) - 100. / 70.).abs() < 1e-4);
    }

    #[test]
    fn o_recurso_aparece_dentro_do_trecho_na_posicao_certa() {
        let e = eixo();
        let area = Rect::from_min_size(Pos2::new(0., 0.), Vec2::new(512., 120.));
        let r = retangulos_do_perfil(&e, area);
        let rec = RecursoEixo { trecho: 1, borda: Borda::Esquerda, posicao: 7., tipo: TipoRecurso::Ranhura { largura: 7., profundidade: 1. } };
        let rr = retangulo_do_recurso(&e, &rec, r[1]).unwrap();
        // 7 de 70 mm = 10% do trecho
        assert!(((rr.left() - r[1].left()) / r[1].width() - 0.1).abs() < 1e-4 && (rr.width() / r[1].width() - 0.1).abs() < 1e-4);
        // medido pela direita no mesmo lugar: 70 - 7 - 7 = 56
        let dir = RecursoEixo { borda: Borda::Direita, posicao: 56., ..rec };
        assert_eq!(retangulo_do_recurso(&e, &dir, r[1]), Some(rr));
    }

    #[test]
    fn remover_trecho_leva_os_recursos_dele_e_reindexa_os_outros() {
        let mk = |trecho| RecursoEixo { trecho, borda: Borda::Esquerda, posicao: 0., tipo: TipoRecurso::Ranhura { largura: 1., profundidade: 0.5 } };
        let mut r = vec![mk(0), mk(1), mk(2), mk(1)];
        remover_trecho(&mut r, 1);
        assert_eq!(r.iter().map(|x| x.trecho).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn arrastar_a_alca_muda_l_ou_diametro_em_passos_e_a_grade_acompanha() {
        let mut e = eixo();
        let eixo_na_tela = Vec2::new(30., 0.);
        // 3 px por mm; 30 px ao longo do eixo = 10 mm
        let alvo = crate::alca_cota::alvo_do_arrasto(e.trechos[0].comprimento, Vec2::new(30., 4.), eixo_na_tela, 3.);
        e.trechos[0].comprimento = comprimento_arrastado(alvo);
        assert_eq!(e.trechos[0].comprimento, 60.);
        // para fora no raio: 20 → raio 10 + 2,3 mm → Ø 24,6 → 25 (múltiplos de 1)
        let alvo = crate::alca_cota::alvo_do_arrasto(10., Vec2::new(7., 0.), Vec2::new(30., 0.), 3.);
        assert_eq!(diametro_arrastado(alvo), 25.);
        assert_eq!(comprimento_arrastado(-50.), 1.);
        assert_eq!(diametro_arrastado(0.), 2.);
        assert!(e.validar().is_ok());
    }
}
