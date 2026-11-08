// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: painel do cordão de solda por conjuntos de faces (B285). Duas listas de faces (o
//! clique na viewport entra no conjunto ativo, Shift+clique tira), propagação por faces tangentes,
//! lista dos caminhos com o comprimento de cada um (passar o mouse na linha realça o caminho no 3D),
//! perna do filete, formato, cosmético ou sólido, contínuo ou intermitente (comprimento e vão, ou
//! comprimento e passo), alternado e término em plano. Cosmético é o padrão: os caminhos aparecem no
//! 3D como traço e alimentam o símbolo de solda do desenho; o sólido gera o metal depositado.

use crate::editor::{check, expr, plane_combo, section};
use crate::geo_ui::Seed;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, RichText, Stroke};
use fenkra_core::*;
use fenkra_solidos::{weld_beads, CordaoCalculado};
use glam::Vec3;
use std::collections::HashMap;
use std::sync::Arc;

type Vars = HashMap<String, f64>;

/// Cordões calculados na geração `geracao` do recálculo (a interface só refaz quando ela muda).
struct Cache {
    geracao: u64,
    comandos: usize,
    cordoes: Vec<CordaoCalculado>,
}

fn chave_cache() -> egui::Id {
    egui::Id::new("st_solda_cache")
}

fn chave_realce() -> egui::Id {
    egui::Id::new("st_solda_realce")
}

fn cache(ctx: &egui::Context) -> Option<Arc<Cache>> {
    ctx.data(|d| d.get_temp::<Arc<Cache>>(chave_cache()))
}

/// Tira a face do comando se ela já está num dos conjuntos (Shift+clique). Devolve se tirou.
pub fn tirar_face(k: &mut FeatureKind, p: [f64; 3], same: impl Fn([f64; 3], [f64; 3]) -> bool) -> bool {
    let FeatureKind::StructWeld(w) = k else { return false };
    match w.faces.iter().position(|q| same(*q, p)) {
        Some(i) => {
            w.remover(i);
            true
        }
        None => false,
    }
}

/// Clique simples: a face já escolhida sai, a nova entra no conjunto ativo.
pub fn alternar_face(w: &mut StructWeld, p: [f64; 3], key: Option<FaceKey>, same: impl Fn([f64; 3], [f64; 3]) -> bool) {
    match w.faces.iter().position(|q| same(*q, p)) {
        Some(i) => w.remover(i),
        None => w.adicionar(p, key),
    }
}

fn medida(v: f64) -> String {
    crate::app::fmt(v)
}

fn lista_de_faces(ui: &mut egui::Ui, w: &mut StructWeld, segundo: bool) {
    let (n1, total) = (w.split.min(w.faces.len()), w.faces.len());
    let faixa = if segundo { n1..total } else { 0..n1 };
    let mut tirar = None;
    for i in faixa {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} {} · {}", tr("Face"), i + 1, crate::app::p3(&w.faces[i], 1))).small());
            if ui.small_button("✖").on_hover_text(tr("Tira esta face do conjunto")).clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        w.remover(i);
    }
}

pub fn solda_panel(ui: &mut egui::Ui, doc: &Document, me: Id, w: &mut StructWeld, vars: &Vars, _seed: &Seed) {
    section(ui, "Conjuntos de faces");
    let (n1, n2) = (w.split.min(w.faces.len()), w.faces.len() - w.split.min(w.faces.len()));
    ui.horizontal(|ui| {
        if ui.selectable_label(!w.picking_b, format!("{} ({n1})", tr("Conjunto 1"))).on_hover_text(tr("Os próximos cliques em faces entram no conjunto 1, por exemplo as faces do perfil.")).clicked() {
            w.picking_b = false;
        }
        if ui.selectable_label(w.picking_b, format!("{} ({n2})", tr("Conjunto 2"))).on_hover_text(tr("Os próximos cliques em faces entram no conjunto 2, por exemplo a face da chapa onde o perfil encosta.")).clicked() {
            w.picking_b = true;
        }
    });
    ui.label(RichText::new(if w.picking_b { tr("Conjunto 2") } else { tr("Conjunto 1") }).strong().small());
    lista_de_faces(ui, w, w.picking_b);
    ui.horizontal(|ui| {
        if ui.button(tr("Limpar o conjunto")).on_hover_text(tr("Tira todas as faces do conjunto ativo.")).clicked() {
            let n = w.split.min(w.faces.len());
            let (de, ate) = if w.picking_b { (n, w.faces.len()) } else { (0, n) };
            for i in (de..ate).rev() {
                w.remover(i);
            }
        }
    });
    ui.label(RichText::new(tr("Clique nas faces na viewport para acrescentar ao conjunto ativo; Shift+clique tira a face.")).weak().small());
    check(ui, &mut w.tangent, "Propagar tangentes").on_hover_text(tr("Cada face escolhida leva junto a cadeia de faces tangentes a ela, como os arredondamentos de um tubo."));

    section(ui, "Caminhos");
    let calc = cache(ui.ctx()).and_then(|c| c.cordoes.iter().find(|x| x.feature == me).cloned());
    let mut realce: Option<usize> = None;
    match &calc {
        Some(c) => {
            ui.label(RichText::new(format!("{}: {} mm · {}: {} mm · {} {}", tr("Comprimento total"), medida(c.comprimento_total()), tr("soldado"), medida(c.comprimento_soldado()), c.trechos(), tr("trecho(s)"))).strong());
            for (i, p) in c.caminhos.iter().enumerate() {
                let r = ui.selectable_label(false, format!("{} {} · {} mm · {} {}{}", tr("Caminho"), i + 1, medida(p.comprimento), p.trechos.len(), tr("trecho(s)"), if p.fechado { format!(" · {}", tr("fechado")) } else { String::new() }));
                if r.hovered() {
                    realce = Some(i);
                }
                r.on_hover_text(tr("Passe o mouse para realçar este caminho no 3D."));
            }
            if c.modo == BeadMode::Solido {
                ui.label(RichText::new(format!("{}: {} mm³", tr("Metal depositado"), medida(c.volume))).small());
            }
        }
        None => {
            ui.label(RichText::new(tr("Escolha as faces dos dois conjuntos: os caminhos aparecem aqui e no 3D.")).weak().small());
        }
    }
    let agora = ui.ctx().input(|i| i.time);
    ui.ctx().data_mut(|d| d.insert_temp(chave_realce(), (agora, me, realce)));

    section(ui, "Cordão");
    expr(ui, "Perna do filete", &mut w.leg, vars, "mm");
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Formato")));
        ui.selectable_value(&mut w.shape, BeadShape::Plano, tr("Plano")).on_hover_text(tr("Seção triangular: o cordão de referência."));
        ui.selectable_value(&mut w.shape, BeadShape::Convexo, tr("Convexo")).on_hover_text(tr("Face abaulada para fora: deposita mais metal que o plano."));
        ui.selectable_value(&mut w.shape, BeadShape::Concavo, tr("Côncavo")).on_hover_text(tr("Face cavada para dentro: deposita menos metal e alivia a concentração de tensão."));
    });
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Modo")));
        ui.selectable_value(&mut w.mode, BeadMode::Cosmetico, tr("Cosmético")).on_hover_text(tr("Só traço e comprimentos, sem material. Alimenta o símbolo de solda do desenho."));
        ui.selectable_value(&mut w.mode, BeadMode::Solido, tr("Sólido")).on_hover_text(tr("Gera o metal depositado: entra na massa e na lista de itens da estrutura."));
    });
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Padrão")));
        ui.selectable_value(&mut w.pattern, BeadPattern::Continuo, tr("Contínuo")).on_hover_text(tr("Um trecho só por caminho, do começo ao fim."));
        ui.selectable_value(&mut w.pattern, BeadPattern::ComprimentoEVao, tr("Trecho e vão")).on_hover_text(tr("Trechos de um comprimento com um vão livre entre eles."));
        ui.selectable_value(&mut w.pattern, BeadPattern::ComprimentoEPasso, tr("Trecho e passo")).on_hover_text(tr("Trechos de um comprimento que se repetem a cada passo (início a início)."));
    });
    if w.pattern != BeadPattern::Continuo {
        expr(ui, "Comprimento do trecho", &mut w.seg_len, vars, "mm");
        expr(ui, if w.pattern == BeadPattern::ComprimentoEVao { "Vão entre trechos" } else { "Passo entre trechos" }, &mut w.spacing, vars, "mm");
        check(ui, &mut w.staggered, "Alternado").on_hover_text(tr("Nos caminhos de ordem par os trechos começam meio passo depois, como a solda dos dois lados de uma chapa."));
        ui.label(RichText::new(tr("Só entram trechos inteiros: o cordão nunca fica mais comprido que o pedido.")).weak().small());
    }
    let mut termina = w.end_plane.is_some();
    if check(ui, &mut termina, "Terminar em um plano").on_hover_text(tr("O cordão para onde cruza o plano e fica só de um dos lados.")).changed() {
        w.end_plane = termina.then(PlaneRef::default);
    }
    if let Some(p) = &mut w.end_plane {
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(tr("Plano")));
            plane_combo(ui, "sw_end", doc, p, None);
        });
        check(ui, &mut w.end_keep_front, "Manter o lado para onde a normal aponta");
    }
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Material")));
        ui.text_edit_singleline(&mut w.material).on_hover_text(tr("Metal de adição. Vazio usa o material da peça."));
    });
    ui.label(RichText::new(tr("O símbolo de solda do desenho é preenchido a partir deste cordão: medida z, contorno e n×l (e).")).weak().small());
    // B174: acessibilidade da tocha
    crate::st_tocha_ui::secao(ui, me);
}

fn atualizar(app: &crate::app::App, ctx: &egui::Context) {
    let geracao = app.geracao();
    let comandos = app.doc().features.len();
    let atual = cache(ctx);
    let velho = atual.as_ref().is_none_or(|c| c.geracao != geracao || c.comandos != comandos);
    // arrastar um valor não refaz o cálculo a cada quadro: espera soltar o mouse
    if !velho || app.calculando() || ctx.input(|i| i.pointer.any_down()) {
        return;
    }
    let cordoes = weld_beads(app.doc());
    ctx.data_mut(|d| d.insert_temp(chave_cache(), Arc::new(Cache { geracao, comandos, cordoes })));
}

/// Traço dos cordões no 3D: cada trecho em cor de destaque, o caminho inteiro mais fino por baixo
/// (os vãos do intermitente ficam visíveis) e o caminho da linha sob o mouse em evidência.
pub fn overlay(app: &mut crate::app::App, ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera) {
    if !app.doc().active_features().any(|(_, f)| matches!(f.kind, FeatureKind::StructWeld(_))) {
        return;
    }
    atualizar(app, ui.ctx());
    let Some(c) = cache(ui.ctx()) else { return };
    crate::st_tocha_ui::overlay(app, ui, rect, cam, &c.cordoes);
    let em_edicao = app.session.as_ref().map(|s| s.id);
    let (agora, realce) = (ui.ctx().input(|i| i.time), ui.ctx().data(|d| d.get_temp::<(f64, Id, Option<usize>)>(chave_realce())));
    let v = ui.visuals().clone();
    let p = ui.painter_at(rect);
    let proj = |q: &[f64; 3]| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
    let linha = |pts: &[[f64; 3]], tr: Stroke| {
        let tela: Vec<_> = pts.iter().filter_map(proj).collect();
        for s in tela.windows(2) {
            p.line_segment([s[0], s[1]], tr);
        }
    };
    for calc in c.cordoes.iter() {
        let cor = if Some(calc.feature) == em_edicao { crate::theme::accent(&v) } else { crate::theme::st_ok() };
        for (i, caminho) in calc.caminhos.iter().enumerate() {
            let destaque = realce.is_some_and(|(t, id, k)| agora - t < 0.25 && id == calc.feature && k == Some(i));
            if destaque {
                linha(&caminho.pts, Stroke::new(6., crate::theme::st_warn()));
            } else if calc.intermitente() {
                linha(&caminho.pts, Stroke::new(1., cor.gamma_multiply(0.45)));
            }
            let largura = if calc.modo == BeadMode::Cosmetico { 3. } else { 1.5 };
            for t in &caminho.trechos {
                linha(&t.pts, Stroke::new(largura, cor));
            }
        }
    }
}
