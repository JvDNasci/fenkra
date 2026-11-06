// SPDX-License-Identifier: LGPL-2.1-or-later
//! Diálogos de confirmação do PMI 3D (B299): balões automáticos com prévia e contagem, e o canto onde
//! nasce a legenda 3D. Painéis livres e encaixáveis, abertos só enquanto o comando pede resposta.
//!
//! Nada muda no documento até confirmar; a confirmação guarda um passo de desfazer e aplica tudo
//! de uma vez, então Ctrl+Z devolve o estado de antes em uma ação.

use crate::app::App;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::Camera;
use eframe::egui::{self, Pos2, Rect, RichText, Stroke};
use fenkra_core::pmi::{PmiKind, TITLE_TEMPLATE};
use fenkra_core::pmi_baloes::{self as pb, Candidato};
use glam::Vec3;
use std::sync::Mutex;

struct Baloes {
    cands: Vec<Candidato>,
    preservar: bool,
}

struct Legenda {
    lo: [f64; 3],
    hi: [f64; 3],
    /// 0..8 = cantos da caixa; 8 = vértice escolhido
    canto: usize,
    ponto: Option<[f64; 3]>,
}

static BALOES: Mutex<Option<Baloes>> = Mutex::new(None);
static LEGENDA: Mutex<Option<Legenda>> = Mutex::new(None);

fn travar<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn abrir_baloes(cands: Vec<Candidato>) {
    *travar(&BALOES) = Some(Baloes { cands, preservar: true });
}

pub fn abrir_legenda(lo: [f64; 3], hi: [f64; 3], ponto: Option<[f64; 3]>) {
    *travar(&LEGENDA) = Some(Legenda { lo, hi, canto: if ponto.is_some() { 8 } else { 0 }, ponto });
}

/// Há diálogo esperando resposta (a viewport não deve criar anotação por baixo).
pub fn aberto() -> bool {
    travar(&BALOES).is_some() || travar(&LEGENDA).is_some()
}

/// Texto "N criados, M trocados" do diálogo (e do aviso depois de aplicar).
pub fn resumo(criados: usize, trocados: usize) -> String {
    format!("{criados} {}, {trocados} {}", tr("criados"), tr("trocados"))
}

/// Ponto onde a legenda nasce e o deslocamento do texto.
fn nascimento(l: &Legenda) -> ([f64; 3], [f64; 3]) {
    match (l.canto, l.ponto) {
        (8, Some(p)) => {
            let meio = [0, 1, 2].map(|k| (l.lo[k] + l.hi[k]) / 2.);
            let sinal = |k: usize| if p[k] >= meio[k] { 1. } else { -1. };
            (p, [sinal(0) * 20., sinal(1) * 20., sinal(2) * 20.])
        }
        (i, _) => (pb::canto_da_caixa(l.lo, l.hi, i.min(7)), pb::deslocamento_do_canto(i.min(7))),
    }
}

/// Janelas dos diálogos; chamada a cada quadro com a área da viewport.
pub fn janela(app: &mut App, ctx: &egui::Context, area: Rect) {
    janela_baloes(app, ctx, area);
    janela_legenda(app, ctx, area);
}

enum Resposta {
    Nada,
    Aplicar,
    Cancelar,
}

fn botoes(ui: &mut egui::Ui, rotulo: &str, dica: &str, pode: bool) -> Resposta {
    let mut r = Resposta::Nada;
    ui.horizontal(|ui| {
        if ui.add_enabled(pode, egui::Button::new(RichText::new(tr(rotulo)).strong())).on_hover_text(tr(dica)).clicked() {
            r = Resposta::Aplicar;
        }
        if ui.button(tr("Cancelar")).on_hover_text(tr("Fecha sem mudar nada no modelo.")).clicked() {
            r = Resposta::Cancelar;
        }
    });
    r
}

fn janela_baloes(app: &mut App, ctx: &egui::Context, area: Rect) {
    let Some(mut d) = travar(&BALOES).take() else { return };
    let plano = pb::planejar(crate::pmi_ui::list(app), &d.cands, d.preservar);
    let cfg = Painel { nome: "pmi_baloes".into(), titulo: tr("Balões automáticos").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 300., redimensiona: false };
    let posicao = app.settings.painel("pmi_baloes", Encaixe::Livre);
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        let (n, m) = plano.contagem();
        ui.label(RichText::new(resumo(n, m)).strong().size(15.));
        if plano.preservados > 0 {
            ui.label(RichText::new(format!("{} {}", plano.preservados, tr("feitos à mão preservados"))).small().color(crate::theme::st_ok()));
        }
        if plano.cobertos > 0 {
            ui.label(RichText::new(format!("{} {}", plano.cobertos, tr("números já cobertos por balão à mão"))).weak().small());
        }
        ui.checkbox(&mut d.preservar, tr("Preservar os balões feitos à mão")).on_hover_text(tr("Ligado: os balões que você criou ou marcou como feitos à mão ficam, e o número deles não ganha outro balão. Desligado: todos os balões atuais são trocados."));
        ui.label(RichText::new(tr("Prévia no modelo: balão cheio = novo, riscado = antigo que sai, verde = preservado.")).weak().small());
        botoes(ui, "Aplicar", "Cria os balões novos e troca os antigos de uma vez; Ctrl+Z desfaz tudo em um passo.", !d.cands.is_empty())
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel("pmi_baloes", p);
    }
    match saida.conteudo.unwrap_or(Resposta::Nada) {
        Resposta::Nada => *travar(&BALOES) = Some(d),
        Resposta::Cancelar => {}
        Resposta::Aplicar => aplicar_baloes(app, &d, &plano),
    }
}

/// Confirma: um passo de desfazer, remove os trocados e inclui os novos.
fn aplicar_baloes(app: &mut App, d: &Baloes, plano: &pb::Plano) {
    let centro = pb::centro(&d.cands);
    let Some(st) = app.asm.as_mut() else { return };
    st.push_undo();
    pb::remover_trocados(&mut st.asm.pmi, plano);
    let novos: Vec<(usize, Candidato)> = plano.criar.iter().map(|&i| (i, d.cands[i].clone())).collect();
    let (pasta, nome) = (tr("Balões").to_string(), tr("Balão").to_string());
    for (_, c) in novos {
        let id = fenkra_core::Id(app.next_pmi_id());
        if let Some(st) = app.asm.as_mut() {
            st.asm.pmi.push(pb::montar(&c, centro, id, &pasta, &nome));
        }
    }
    let (n, m) = plano.contagem();
    app.toast(resumo(n, m), false);
}

fn janela_legenda(app: &mut App, ctx: &egui::Context, area: Rect) {
    let Some(mut d) = travar(&LEGENDA).take() else { return };
    let cfg = Painel { nome: "pmi_legenda".into(), titulo: tr("Legenda 3D").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 300., redimensiona: false };
    let posicao = app.settings.painel("pmi_legenda", Encaixe::Livre);
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.label(tr("Onde a legenda nasce"));
        let nome = |i: usize| if i == 8 { tr("No vértice selecionado").to_string() } else { tr(pb::nome_do_canto(i)).to_string() };
        egui::ComboBox::from_id_salt("pmi_legenda_canto").selected_text(nome(d.canto)).width(240.).show_ui(ui, |ui| {
            for i in 0..8 {
                ui.selectable_value(&mut d.canto, i, nome(i));
            }
            if d.ponto.is_some() {
                ui.selectable_value(&mut d.canto, 8, nome(8));
            }
        })
        .response
        .on_hover_text(tr("Um dos oito cantos da caixa do modelo, ou o vértice que você selecionou antes de abrir o comando."));
        if d.ponto.is_none() {
            ui.label(RichText::new(tr("Para nascer num ponto, selecione um vértice antes de abrir o comando.")).weak().small());
        }
        ui.label(RichText::new(tr("A prévia mostra o quadro da legenda no modelo.")).weak().small());
        botoes(ui, "Criar legenda", "Cria a legenda no ponto escolhido, afastada do modelo.", true)
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel("pmi_legenda", p);
    }
    match saida.conteudo.unwrap_or(Resposta::Nada) {
        Resposta::Nada => *travar(&LEGENDA) = Some(d),
        Resposta::Cancelar => {}
        Resposta::Aplicar => {
            let (ponto, off) = nascimento(&d);
            crate::pmi_ui::criar_legenda(app, ponto, off);
        }
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Prévia no 3D, desenhada por baixo das anotações reais.
pub fn previa(app: &App, ui: &egui::Ui, rect: Rect, cam: &Camera) {
    let painter = ui.painter_at(rect);
    let acento = crate::theme::accent(ui.visuals());
    if let Some(d) = travar(&BALOES).as_ref() {
        let plano = pb::planejar(crate::pmi_ui::list(app), &d.cands, d.preservar);
        let centro = pb::centro(&d.cands);
        let raio = |item: &str| 11. + 3.5 * item.chars().count().saturating_sub(1) as f32;
        let proj = |p: [f64; 3]| cam.project(rect, v3(p));
        // antigos que saem: riscados; preservados: anel verde
        for p in crate::pmi_ui::list(app) {
            let PmiKind::Balloon { item, .. } = &p.kind else { continue };
            let (Some(a), Some(&a0)) = (proj(p.anchors.first().copied().unwrap_or([0.; 3])), p.anchors.first()) else { continue };
            let Some(t) = proj([a0[0] + p.offset[0], a0[1] + p.offset[1], a0[2] + p.offset[2]]) else { continue };
            let sai = plano.trocar.contains(&p.id);
            let cor = if sai { crate::theme::st_warn() } else { crate::theme::st_ok() };
            painter.line_segment([a, t], Stroke::new(1.2, cor));
            painter.circle_stroke(t, raio(item), Stroke::new(2., cor));
            if sai {
                let r = raio(item) * 0.8;
                painter.line_segment([t + egui::vec2(-r, -r), t + egui::vec2(r, r)], Stroke::new(2., cor));
                painter.line_segment([t + egui::vec2(-r, r), t + egui::vec2(r, -r)], Stroke::new(2., cor));
            }
        }
        for &i in &plano.criar {
            let c = &d.cands[i];
            let off = pb::afastamento(c, centro);
            let (Some(a), Some(t)) = (proj(c.ancora), proj([c.ancora[0] + off[0], c.ancora[1] + off[1], c.ancora[2] + off[2]])) else { continue };
            let r = raio(&c.item);
            let dir = (a - t).normalized();
            painter.line_segment([t + dir * r, a], Stroke::new(1.5, acento));
            painter.circle_filled(a, 3., acento);
            painter.circle_filled(t, r, ui.visuals().window_fill().gamma_multiply(0.9));
            painter.circle_stroke(t, r, Stroke::new(2., acento));
            painter.text(t, egui::Align2::CENTER_CENTER, &c.item, egui::FontId::proportional(13.), ui.visuals().strong_text_color());
        }
    }
    if let Some(l) = travar(&LEGENDA).as_ref() {
        let (p, off) = nascimento(l);
        let q = [p[0] + off[0], p[1] + off[1], p[2] + off[2]];
        if let (Some(a), Some(t)) = (cam.project(rect, v3(p)), cam.project(rect, v3(q))) {
            let quadro = Rect::from_min_size(t, egui::vec2(130., 54.));
            painter.line_segment([a, quadro.left_top()], Stroke::new(1.5, acento));
            painter.circle_filled(a, 4., acento);
            painter.rect_filled(quadro, 3., ui.visuals().window_fill().gamma_multiply(0.9));
            painter.rect_stroke(quadro, 3., Stroke::new(2., acento), egui::StrokeKind::Inside);
            for (k, linha) in TITLE_TEMPLATE.lines().take(4).enumerate() {
                let y = quadro.top() + 8. + k as f32 * 12.;
                let w = 30. + 20. * ((linha.len() % 5) as f32);
                painter.line_segment([Pos2::new(quadro.left() + 8., y), Pos2::new(quadro.left() + 8. + w.min(110.), y)], Stroke::new(2., ui.visuals().weak_text_color()));
            }
        }
    }
}
