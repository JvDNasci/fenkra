// SPDX-License-Identifier: LGPL-2.1-or-later
//! B192: painel do esquema de cotagem automático (PMI 3D). Livre e encaixável, aberto só enquanto o
//! assistente está em uso: modo (Mais/Menos ou Geométrico), tolerâncias e uma linha por tipo de
//! feature. A prévia no 3D mostra as referências escolhidas (A, B, C) e onde cada anotação nasce;
//! as geradas que vão sair aparecem riscadas. Nada muda no modelo até Aplicar, que guarda um passo
//! de desfazer. A regra mora em `fenkra_solidos::esquema_pmi`.

use crate::app::App;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::Camera;
use crate::ui_ext::dv;
use eframe::egui::{self, Rect, RichText, Stroke};
use fenkra_core::pmi::Pmi;
use fenkra_solidos::esquema_pmi::{self as ep, ModoEsquema, PlanoRef, RegrasEsquema};
use glam::Vec3;
use std::cell::RefCell;

struct Estado {
    regras: RegrasEsquema,
    /// falso: "Refazer tudo" (a regra manda de novo nas geradas)
    preservar: bool,
    /// proposta para as regras de `calculada`
    proposta: Result<(Vec<PlanoRef>, Vec<Pmi>), String>,
    calculada: Option<(RegrasEsquema, String)>,
}

thread_local! {
    static ESTADO: RefCell<Option<Estado>> = const { RefCell::new(None) };
    static DEMO_FEITA: RefCell<bool> = const { RefCell::new(false) };
}

/// Abre o assistente (só na peça).
pub fn abrir(app: &mut App) {
    if app.in_asm() {
        app.toast(tr("Esquema automático: abra uma peça"), true);
        return;
    }
    ESTADO.with(|e| *e.borrow_mut() = Some(Estado { regras: RegrasEsquema::default(), preservar: true, proposta: Err(String::new()), calculada: None }));
}

/// O assistente está aberto (a viewport não deve criar anotação por baixo).
pub fn aberto() -> bool {
    ESTADO.with(|e| e.borrow().is_some())
}

/// Recalcula a proposta quando as regras mudaram.
fn atualizar(app: &App, e: &mut Estado) {
    // a peça também conta: trocar de documento ou mudar a árvore refaz a proposta
    let d = &app.open.doc;
    let chave = format!("{}|{}|{}", d.name, d.features.len(), d.peek_id());
    if e.calculada.as_ref().is_some_and(|(r, k)| *r == e.regras && *k == chave) {
        return;
    }
    e.proposta = ep::propor(d, &e.regras);
    e.calculada = Some((e.regras.clone(), chave));
}

/// Contagem do que Aplicar faria: novas, mantidas e geradas que saem.
fn contagem(atual: &[Pmi], proposta: &[Pmi]) -> (usize, usize, usize) {
    let existe = |n: &str| atual.iter().any(|p| ep::gerada(p) && p.name == n);
    let novas = proposta.iter().filter(|p| !existe(&p.name)).count();
    let sai = atual.iter().filter(|p| ep::gerada(p) && !proposta.iter().any(|q| q.name == p.name)).count();
    (novas, proposta.len() - novas, sai)
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Prévia no 3D: letra de cada referência, ponto e rótulo curto de cada anotação nova e as geradas
/// que vão sair, riscadas.
pub fn previa(app: &App, ui: &egui::Ui, rect: Rect, cam: &Camera) {
    ESTADO.with(|e| {
        let e = e.borrow();
        let Some(e) = e.as_ref() else { return };
        let Ok((drf, prop)) = &e.proposta else { return };
        let painter = ui.painter_at(rect);
        let acento = crate::theme::accent(ui.visuals());
        let fundo = ui.visuals().extreme_bg_color;
        let texto = ui.visuals().text_color();
        let proj = |p: [f64; 3]| cam.project(rect, v3(p));
        let fonte = egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.));
        for p in prop {
            let Some(&a0) = p.anchors.first() else { continue };
            let Some(a) = proj(a0) else { continue };
            let Some(t) = proj([a0[0] + p.offset[0], a0[1] + p.offset[1], a0[2] + p.offset[2]]) else { continue };
            painter.line_segment([a, t], Stroke::new(1.2, acento));
            painter.circle_filled(a, 3., acento);
            if let Some(b) = p.anchors.get(1).and_then(|b| proj(*b)) {
                painter.line_segment([a, b], Stroke::new(1.6, acento));
            }
            let rot = painter.layout_no_wrap(tr(&p.name).to_string(), fonte.clone(), texto);
            let caixa = Rect::from_center_size(t, rot.size() + egui::vec2(8., 4.));
            painter.rect(caixa, 3., fundo, Stroke::new(1., acento), egui::StrokeKind::Outside);
            painter.galley(caixa.center() - rot.size() / 2., rot, texto);
        }
        // referências: quadro com a letra sobre a face
        for r in drf {
            let Some(c) = proj(r.ponto) else { continue };
            let lado = 22.;
            let q = Rect::from_center_size(c, egui::vec2(lado, lado));
            painter.rect(q, 0., acento, Stroke::new(1.5, texto), egui::StrokeKind::Outside);
            painter.text(c, egui::Align2::CENTER_CENTER, r.letra.to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(15.)), fundo);
        }
        // geradas que saem
        let aviso = crate::theme::st_warn();
        for p in app.open.doc.pmi.iter().filter(|p| ep::gerada(p) && !prop.iter().any(|q| q.name == p.name)) {
            let Some(a) = p.anchors.first().and_then(|a| proj(*a)) else { continue };
            let r = 7.;
            painter.circle_stroke(a, r, Stroke::new(2., aviso));
            painter.line_segment([a + egui::vec2(-r, -r), a + egui::vec2(r, r)], Stroke::new(2., aviso));
        }
    });
}

enum Resposta {
    Nada,
    Aplicar,
    Cancelar,
}

fn linha_feature(ui: &mut egui::Ui, nome: &str, dica: &str, r: &mut ep::RegraFeature, tol_rotulo: Option<(&str, &str)>) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut r.gerar, tr(nome)).on_hover_text(tr(dica));
        if let Some((rot, d)) = tol_rotulo {
            ui.add_enabled_ui(r.gerar, |ui| {
                ui.label(tr(rot));
                ui.add(egui::TextEdit::singleline(&mut r.tol).desired_width(52.)).on_hover_text(tr(d));
            });
        }
    });
}

/// Janela do assistente; chamada a cada quadro com a área da viewport.
pub fn janela(app: &mut App, ctx: &egui::Context, area: Rect) {
    demo(app);
    let Some(mut e) = ESTADO.with(|e| e.borrow_mut().take()) else { return };
    if app.in_asm() {
        return;
    }
    atualizar(app, &mut e);
    let cfg = Painel { nome: "pmi_esquema".into(), titulo: tr("Esquema de cotagem automático").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 320., redimensiona: false };
    let posicao = app.settings.painel("pmi_esquema", Encaixe::Livre);
    let atual = app.open.doc.pmi.clone();
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        // 1. modo
        ui.label(RichText::new(tr("Modo")).strong());
        ui.horizontal(|ui| {
            for m in [ModoEsquema::MaisMenos, ModoEsquema::Geometrico] {
                let dica = match m {
                    ModoEsquema::MaisMenos => "Medidas com ± (tamanho total, localização dos furos, largura dos rasgos) e o furo pela classe de ajuste.",
                    ModoEsquema::Geometrico => "Planeza na referência A, perpendicularidade de B e C, posição dos furos em A, B e C; a localização vira cota básica.",
                };
                ui.selectable_value(&mut e.regras.modo, m, tr(m.label())).on_hover_text(tr(dica));
            }
        });
        // 2. tolerâncias
        ui.separator();
        ui.label(RichText::new(tr("Tolerâncias")).strong());
        egui::Grid::new("pmi_esquema_tol").num_columns(2).show(ui, |ui| {
            if e.regras.modo == ModoEsquema::MaisMenos {
                ui.add(dv(&mut e.regras.tol_linear).rotulo(tr("Linear ±")).suffix(" mm").speed(0.01).range(0.001..=10.).padrao(0.1).dica(tr("Tolerância das medidas de tamanho e de localização.")));
                ui.end_row();
            } else {
                ui.add(dv(&mut e.regras.planeza).rotulo(tr("Planeza A")).suffix(" mm").speed(0.005).range(0.001..=5.).padrao(0.05).dica(tr("Planeza da referência primária (a face maior).")));
                ui.end_row();
                ui.add(dv(&mut e.regras.perpendicularidade).rotulo(tr("Perpendicularidade")).suffix(" mm").speed(0.005).range(0.001..=5.).padrao(0.05).dica(tr("Perpendicularidade da secundária a A e da terciária a A e B.")));
                ui.end_row();
                ui.add(dv(&mut e.regras.posicao).rotulo(tr("Posição Ø")).suffix(" mm").speed(0.01).range(0.001..=5.).padrao(0.1).dica(tr("Zona cilíndrica de posição dos furos, em relação a A, B e C.")));
                ui.end_row();
            }
        });
        // 3. regras por feature
        ui.separator();
        ui.label(RichText::new(tr("Features")).strong());
        linha_feature(ui, "Furos", "Diâmetro por grupo de furos iguais (n×) e a localização do primeiro a partir de B e C.", &mut e.regras.furo, Some(("Classe", "Classe ISO 286 do furo, como H7 ou H8; vazio deixa sem classe.")));
        linha_feature(ui, "Rasgos e degraus", "Largura entre duas faces paralelas que se encaram.", &mut e.regras.rasgo, Some(("±", "Tolerância da largura, em mm.")));
        linha_feature(ui, "Chanfros", "Nota C1 (ou 1 × 2) na face do chanfro.", &mut e.regras.chanfro, None);
        // 4. resultado
        ui.separator();
        let pode = match &e.proposta {
            Ok((drf, prop)) => {
                let letras: Vec<String> = drf.iter().map(|r| r.letra.to_string()).collect();
                ui.label(format!("{}: {}", tr("Referências"), letras.join(", "))).on_hover_text(tr("A: a maior face plana; B: a maior perpendicular a A; C: a maior perpendicular às duas. Empate fica com a face de apoio."));
                let (n, m, s) = contagem(&atual, prop);
                ui.label(RichText::new(format!("{n} {}, {m} {}, {s} {}", tr("novas"), tr("mantidas"), tr("saem"))).strong());
                true
            }
            Err(msg) => {
                ui.label(RichText::new(if msg.is_empty() { tr("Calculando…").to_string() } else { tr(msg).to_string() }).color(crate::theme::st_warn()));
                false
            }
        };
        ui.checkbox(&mut e.preservar, tr("Preservar as edições das anotações geradas")).on_hover_text(tr("Ligado: a anotação gerada que você editou fica com o seu texto e a sua tolerância; só a posição acompanha a peça. Desligado: as geradas voltam ao que a regra manda. O que está fora da pasta Esquema automático nunca é tocado."));
        ui.label(RichText::new(tr("Prévia no modelo: quadro com a letra = referência, rótulo = anotação nova, riscado = gerada que sai.")).weak().small());
        let mut r = Resposta::Nada;
        ui.horizontal(|ui| {
            if ui.add_enabled(pode, egui::Button::new(RichText::new(tr("Aplicar")).strong())).on_hover_text(tr("Cria e atualiza as anotações da pasta Esquema automático; Ctrl+Z desfaz em um passo.")).clicked() {
                r = Resposta::Aplicar;
            }
            if ui.button(tr("Cancelar")).on_hover_text(tr("Fecha sem mudar nada no modelo.")).clicked() {
                r = Resposta::Cancelar;
            }
        });
        r
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel("pmi_esquema", p);
    }
    match saida.conteudo.unwrap_or(Resposta::Nada) {
        Resposta::Nada => ESTADO.with(|s| *s.borrow_mut() = Some(e)),
        Resposta::Cancelar => {}
        Resposta::Aplicar => {
            if let Ok((_, prop)) = e.proposta {
                let antes = app.open.doc.clone();
                let rel = ep::aplicar(&mut app.open.doc, prop, !e.preservar);
                app.push_undo_pub(antes);
                app.toast(format!("{}: {} {}, {} {}, {} {}", tr("Esquema automático"), rel.novas, tr("novas"), rel.mantidas, tr("mantidas"), rel.removidas, tr("saem")), false);
            }
        }
    }
}

/// Desenvolvimento: `FENKRA_DEMO_PMI_ESQUEMA=mm|geo|aplicado` abre o assistente na peça aberta no
/// modo pedido (`aplicado`: já aplica o Mais/Menos e reabre no Geométrico, mostrando o que sai).
fn demo(app: &mut App) {
    let Ok(modo) = std::env::var("FENKRA_DEMO_PMI_ESQUEMA") else { return };
    // espera a peça de exemplo chegar ao documento
    if app.open.doc.features.is_empty() {
        return;
    }
    if DEMO_FEITA.with(|d| d.replace(true)) || app.in_asm() {
        return;
    }
    if modo == "aplicado" {
        let _ = ep::gerar(&mut app.open.doc, &RegrasEsquema::default(), false);
    }
    abrir(app);
    if modo != "mm" {
        ESTADO.with(|e| {
            if let Some(e) = e.borrow_mut().as_mut() {
                e.regras.modo = ModoEsquema::Geometrico;
            }
        });
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn contagem_separa_novas_mantidas_e_as_que_saem() {
        let mk = |n: &str, pasta: &str| Pmi { id: fenkra_core::Id(1), name: n.into(), kind: fenkra_core::pmi::PmiKind::Linear, anchors: vec![[0.; 3]; 2], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: pasta.into(), targets: vec![] };
        let atual = vec![mk("Total A", ep::PASTA), mk("Total B", ep::PASTA), mk("Total A", "")];
        let prop = vec![mk("Total A", ep::PASTA), mk("Planeza A", ep::PASTA)];
        assert_eq!(contagem(&atual, &prop), (1, 1, 1));
    }
}
