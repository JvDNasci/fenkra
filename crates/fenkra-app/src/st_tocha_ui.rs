// SPDX-License-Identifier: LGPL-2.1-or-later
//! B174: seção "Acessibilidade da tocha" do painel do cordão e o desenho do cone no 3D. Ligada, ela
//! testa a tocha em cada ponto do cordão em edição (ver `fenkra_solidos::st_tocha`) e mostra, no 3D,
//! as direções livres em verde, as bloqueadas em vermelho e os pontos sem direção nenhuma marcados.

use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, RichText, Stroke};
use fenkra_core::*;
use fenkra_solidos::st_tocha::{acessibilidade, sugestoes, Obstaculos, OpcoesTocha, PontoTocha};
use glam::Vec3;
use std::sync::Arc;

/// Estado da seção: ligada, as medidas da tocha e o último resultado.
#[derive(Clone, Default)]
struct Estado {
    ligada: bool,
    opcoes: OpcoesTocha,
    /// (geração do recálculo, comando, opções) do resultado guardado
    chave: Option<(u64, Id, [u64; 7])>,
    pontos: Arc<Vec<PontoTocha>>,
    textos: Arc<Vec<String>>,
}

fn id() -> egui::Id {
    egui::Id::unique("st_tocha_estado")
}

/// Estado guardado; na primeira vez, `FENKRA_DEMO_TOCHA` já liga o teste (captura sem mouse).
fn estado(ctx: &egui::Context) -> Estado {
    ctx.data(|d| d.get_temp(id())).unwrap_or_else(|| Estado { ligada: std::env::var("FENKRA_DEMO_TOCHA").is_ok(), ..Default::default() })
}

fn bits(o: &OpcoesTocha) -> [u64; 7] {
    [o.raio_bocal, o.afastamento, o.comprimento, o.faixa_trabalho, o.faixa_deslocamento, o.passo_angulo, o.passo].map(f64::to_bits)
}

fn campo(ui: &mut egui::Ui, nome: &str, v: &mut f64, faixa: std::ops::RangeInclusive<f64>, sufixo: &str, dica: &str) {
    ui.label(tr(nome));
    let r = ui.add(crate::ui_ext::dv(v).speed(1.).range(faixa).suffix(sufixo).max_decimals(1).dica(tr(dica)));
    crate::ui_ext::wheel(ui, &r, v);
    ui.end_row();
}

/// Seção do painel do cordão `me`.
pub fn secao(ui: &mut egui::Ui, me: Id) {
    crate::editor::section(ui, "Acessibilidade da tocha");
    let mut e = estado(ui.ctx());
    ui.checkbox(&mut e.ligada, tr("Testar a tocha em cada ponto")).on_hover_text(tr("Gira a tocha em torno da bissetriz das pernas e mostra no 3D as direções livres (verde) e as que batem na peça (vermelho)."));
    if e.ligada {
        egui::Grid::new("st_tocha_campos").num_columns(2).show(ui, |ui| {
            campo(ui, "Raio do bocal", &mut e.opcoes.raio_bocal, 2.0..=40.0, " mm", "Raio externo do bocal da tocha.");
            campo(ui, "Afastamento", &mut e.opcoes.afastamento, 0.0..=60.0, " mm", "Distância do ponto de solda à ponta do bocal (stick-out e recuo do bico).");
            campo(ui, "Comprimento livre", &mut e.opcoes.comprimento, 30.0..=1000.0, " mm", "Comprimento da tocha que precisa caber, medido do ponto de solda.");
            campo(ui, "Ângulo de trabalho", &mut e.opcoes.faixa_trabalho, 0.0..=80.0, "°", "Quanto a tocha pode girar em torno do cordão, para cada lado da bissetriz.");
            campo(ui, "Ângulo de deslocamento", &mut e.opcoes.faixa_deslocamento, 0.0..=80.0, "°", "Quanto a tocha pode inclinar ao longo do cordão, para a frente e para trás.");
            campo(ui, "Passo do ângulo", &mut e.opcoes.passo_angulo, 2.0..=45.0, "°", "Intervalo entre as direções testadas.");
            campo(ui, "Passo no cordão", &mut e.opcoes.passo, 2.0..=500.0, " mm", "Distância entre os pontos testados ao longo do cordão.");
        });
        if let Some((_, quem, _)) = e.chave.filter(|c| c.1 == me) {
            let _ = quem;
            let n = e.pontos.len();
            let ruins = e.pontos.iter().filter(|p| !p.viavel()).count();
            let livre = if n > 0 { e.pontos.iter().map(PontoTocha::fracao_livre).sum::<f64>() / n as f64 } else { 0. };
            ui.label(RichText::new(format!("{n} {} · {ruins} {} · {:.0}% {}", tr("pontos"), tr("inviáveis"), livre * 100., tr("das direções livres"))).strong());
            if ruins == 0 && n > 0 {
                ui.label(RichText::new(tr("✔ A tocha alcança todos os pontos")).color(crate::theme::st_ok()));
            }
            for s in e.textos.iter() {
                ui.label(RichText::new(format!("⚠ {s}")).color(crate::theme::st_warn()).small());
            }
        }
        ui.label(RichText::new(tr("Bocal tratado como cilindro; o cordão já depositado não conta como obstáculo.")).weak().small());
    }
    ui.ctx().data_mut(|d| d.insert_temp(id(), e));
}

/// Refaz o teste quando o recálculo, o comando ou as medidas mudam, e desenha o cone no 3D.
pub fn overlay(app: &crate::app::App, ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera, cordoes: &[fenkra_solidos::CordaoCalculado]) {
    let Some(me) = app.session.as_ref().map(|s| s.id) else { return };
    let mut e = estado(ui.ctx());
    if !e.ligada {
        return;
    }
    let chave = (app.geracao(), me, bits(&e.opcoes));
    if e.chave != Some(chave) && !app.calculando() && !ui.ctx().input(|i| i.pointer.any_down()) {
        let (Some(r), Some(c)) = (app.result.as_ref(), cordoes.iter().find(|c| c.feature == me)) else { return };
        let obs = Obstaculos::da_peca(r);
        let pontos = acessibilidade(c, &obs, &e.opcoes);
        e.textos = Arc::new(sugestoes(app.doc(), &pontos).iter().map(|s| tr(s).to_string()).collect());
        e.pontos = Arc::new(pontos);
        e.chave = Some(chave);
        ui.ctx().data_mut(|d| d.insert_temp(id(), e.clone()));
    }
    if e.chave.map(|c| c.1) != Some(me) {
        return;
    }
    let p = ui.painter_at(rect);
    let proj = |q: [f64; 3]| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
    // comprimento do traço de cada direção na tela: uma fração do comprimento livre
    let l = (e.opcoes.comprimento * 0.25).clamp(10., 80.);
    for pt in e.pontos.iter() {
        let Some(a) = proj(pt.p) else { continue };
        for d in &pt.direcoes {
            let Some(b) = proj(fenkra_solidos::geo3d::add(pt.p, fenkra_solidos::geo3d::mul(d.d, l))) else { continue };
            let cor = if d.livre { crate::theme::st_ok() } else { crate::theme::st_err().gamma_multiply(0.5) };
            p.line_segment([a, b], Stroke::new(if d.livre { 1.5 } else { 1. }, cor));
        }
        if let Some(k) = pt.melhor {
            if let Some(b) = proj(fenkra_solidos::geo3d::add(pt.p, fenkra_solidos::geo3d::mul(pt.direcoes[k].d, l * 1.3))) {
                p.line_segment([a, b], Stroke::new(3., crate::theme::accent(ui.visuals())));
            }
        } else {
            p.circle_stroke(a, 7., Stroke::new(2.5, crate::theme::st_err()));
        }
    }
}
