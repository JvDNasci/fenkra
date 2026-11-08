// SPDX-License-Identifier: LGPL-2.1-or-later
//! B163: janela "Caminho de corte do tubo". Para um membro com encaixe, mostra o caminho da máquina
//! de tubo rotativo (`fenkra_solidos::st_caminho_tubo`): ajustes de kerf, entrada, saída, microjuntas
//! e tempo; o relatório de comprimento e tempo; a vista desenvolvida (o tubo aberto, X ao longo da
//! barra e o perímetro na vertical) com o caminho de cada operação; a tabela por ponto e o CSV. No 3D,
//! o caminho aparece como linha sobre a pele do tubo com setas da tocha, e a animação leva o cabeçote
//! pelos pontos na ordem da máquina.

use crate::app::App;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, RichText, Stroke};
use fenkra_core::Id;
use fenkra_solidos::st_caminho_tubo::{caminho_do_membro, CaminhoTubo, OpcoesCaminho};
use fenkra_solidos::st_encaixe_doc::encaixes_da_estrutura;
use glam::Vec3;
use std::sync::Arc;

/// Estado da janela (no contexto da interface).
#[derive(Clone, Default)]
struct Estado {
    aberta: bool,
    membro: Option<(Id, usize)>,
    opc: OpcoesCaminho,
    /// posição da animação, de 0 a 1 do caminho todo
    t: f64,
    tocando: bool,
    caminho: Option<Arc<CaminhoTubo>>,
    /// (geração do recálculo, membro) do caminho guardado
    chave: Option<(u64, (Id, usize))>,
}

fn id() -> egui::Id {
    egui::Id::unique("st_corte_tubo")
}

/// Abre a janela (botão da lista de corte).
pub fn abrir(ctx: &egui::Context) {
    let mut e: Estado = ctx.data(|d| d.get_temp(id())).unwrap_or_default();
    e.aberta = true;
    ctx.data_mut(|d| d.insert_temp(id(), e));
}

/// Pontos do caminho na ordem da máquina, com o índice da operação.
fn sequencia(c: &CaminhoTubo) -> Vec<(usize, fenkra_solidos::st_caminho_tubo::PontoTocha)> {
    c.operacoes.iter().enumerate().flat_map(|(k, o)| o.pontos.iter().map(move |p| (k, *p))).collect()
}

fn campo(ui: &mut egui::Ui, nome: &str, v: &mut f64, faixa: std::ops::RangeInclusive<f64>, sufixo: &str, dica: &str) {
    ui.label(tr(nome));
    let r = ui.add(crate::ui_ext::dv(v).speed(0.1).range(faixa).suffix(sufixo).max_decimals(2).dica(tr(dica)));
    crate::ui_ext::wheel(ui, &r, v);
    ui.end_row();
}

pub fn window(app: &mut App, ctx: &egui::Context) {
    let mut e: Estado = ctx.data(|d| d.get_temp(id())).unwrap_or_default();
    // desenvolvimento: FENKRA_DEMO_CORTE_TUBO=<0..1> abre a janela uma vez, com o cabeçote nessa posição
    static DEMO: std::sync::Once = std::sync::Once::new();
    DEMO.call_once(|| {
        if let Ok(v) = std::env::var("FENKRA_DEMO_CORTE_TUBO") {
            e.aberta = true;
            e.t = v.parse().unwrap_or(0.4);
        }
    });
    if !e.aberta {
        return;
    }
    let membros = encaixes_da_estrutura(app.doc(), &Default::default());
    if e.membro.is_none_or(|m| !membros.iter().any(|x| (x.feature, x.linha) == m)) {
        e.membro = membros.first().map(|m| (m.feature, m.linha));
    }
    let chave = e.membro.map(|m| (app.geracao(), m));
    let mut aberta = true;
    let mut csv = false;
    egui::Window::new(tr("Caminho de corte do tubo")).id(egui::Id::unique("janela_corte_tubo")).open(&mut aberta).default_size([560., 620.]).resizable(true).show(ctx, |ui| {
        if membros.is_empty() {
            ui.label(RichText::new(tr("Nenhum membro com ponta de encaixe: o caminho de corte sai da curva do encaixe.")).weak());
            return;
        }
        let nome = |m: (Id, usize)| format!("{} · {} {}", app.doc().name_of(m.0), tr("linha"), m.1 + 1);
        egui::ComboBox::from_label(tr("Membro")).selected_text(e.membro.map(nome).unwrap_or_default()).show_ui(ui, |ui| {
            for m in &membros {
                ui.selectable_value(&mut e.membro, Some((m.feature, m.linha)), nome((m.feature, m.linha)));
            }
        });
        let antes = e.opc;
        egui::CollapsingHeader::new(tr("Ajustes do corte")).id_salt("corte_tubo_ajustes").default_open(false).show(ui, |ui| {
            egui::Grid::new("corte_tubo_campos").num_columns(2).show(ui, |ui| {
                campo(ui, "Kerf", &mut e.opc.kerf, 0.0..=5.0, " mm", "Largura do sulco de corte; o caminho anda metade dela para o lado da sobra.");
                campo(ui, "Entrada (lead-in)", &mut e.opc.entrada, 0.0..=50.0, " mm", "Comprimento da entrada (lead-in), pelo lado da sobra.");
                campo(ui, "Saída (lead-out)", &mut e.opc.saida, 0.0..=50.0, " mm", "Quanto a saída (lead-out) passa do ponto de início para fechar o corte.");
                let mut mj = e.opc.microjuntas as f64;
                campo(ui, "Microjuntas", &mut mj, 0.0..=12.0, "", "Pontes que seguram a peça na barra, espalhadas por igual no perímetro.");
                e.opc.microjuntas = mj.round() as usize;
                campo(ui, "Microjunta", &mut e.opc.microjunta, 0.0..=10.0, " mm", "Comprimento de cada microjunta.");
                campo(ui, "Velocidade", &mut e.opc.velocidade, 10.0..=50000.0, " mm/min", "Velocidade de corte da máquina para este material e espessura.");
                campo(ui, "Perfuração", &mut e.opc.perfuracao, 0.0..=30.0, " s", "Tempo de cada perfuração.");
            });
            ui.checkbox(&mut e.opc.inverter, tr("Sentido inverso")).on_hover_text(tr("Percorre a curva no outro sentido."));
        });
        if e.chave != chave || e.opc != antes || e.caminho.is_none() {
            e.caminho = e.membro.and_then(|m| membros.iter().find(|x| (x.feature, x.linha) == m)).and_then(|m| caminho_do_membro(m, &e.opc)).map(Arc::new);
            e.chave = chave;
        }
        let Some(c) = e.caminho.clone() else {
            ui.label(RichText::new(tr("Este membro não tem caminho de corte (seção sem encaixe calculado).")).weak());
            return;
        };
        ui.label(RichText::new(c.relatorio()).small().monospace());
        // animação do cabeçote
        ui.horizontal(|ui| {
            if ui.button(if e.tocando { tr("⏸ Pausar") } else { tr("▶ Animar") }).on_hover_text(tr("Leva o cabeçote pelos pontos na ordem da máquina, no 3D e na vista desenvolvida.")).clicked() {
                e.tocando = !e.tocando;
            }
            ui.add(egui::Slider::new(&mut e.t, 0.0..=1.0).show_value(false));
            if ui.button(tr("CSV…")).on_hover_text(tr("Tabela por ponto: operação, etapa, X, giro A, posição no perímetro, bisel, vetor da tocha e se corta.")).clicked() {
                csv = true;
            }
        });
        if e.tocando {
            e.t += ui.input(|i| i.stable_dt as f64) / 8.;
            if e.t > 1. {
                e.t = 0.;
            }
            ui.ctx().request_repaint();
        }
        let seq = sequencia(&c);
        let atual = ((seq.len().saturating_sub(1)) as f64 * e.t).round() as usize;
        // vista desenvolvida
        ui.label(RichText::new(tr("Vista desenvolvida (X ao longo da barra, perímetro na vertical)")).strong());
        let (resp, p) = ui.allocate_painter(egui::vec2(ui.available_width(), 180.), egui::Sense::hover());
        let r = resp.rect;
        p.rect_stroke(r, 2., Stroke::new(1., ui.visuals().weak_text_color()), egui::StrokeKind::Inside);
        let (xmin, xmax) = seq.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, q)| (a.min(q.x), b.max(q.x)));
        let (smin, smax) = seq.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, q)| (a.min(q.s), b.max(q.s)));
        let esc_x = (r.width() as f64 - 20.) / (xmax - xmin).max(1.);
        let esc_s = (r.height() as f64 - 20.) / (smax - smin).max(1.);
        let tela = |x: f64, s: f64| egui::pos2(r.left() + 10. + ((x - xmin) * esc_x) as f32, r.bottom() - 10. - ((s - smin) * esc_s) as f32);
        let cores = [crate::theme::accent(ui.visuals()), crate::theme::st_ok()];
        for w in seq.windows(2) {
            let ((k0, a), (k1, b)) = (w[0], w[1]);
            if k0 != k1 || (b.s - a.s).abs() > (smax - smin) * 0.5 {
                continue;
            }
            let cor = cores[k0 % 2];
            let st = if a.corta && b.corta { Stroke::new(2., cor) } else { Stroke::new(1., cor.gamma_multiply(0.35)) };
            p.line_segment([tela(a.x, a.s), tela(b.x, b.s)], st);
        }
        if let Some((_, q)) = seq.get(atual) {
            p.circle_filled(tela(q.x, q.s), 5., crate::theme::st_warn());
        }
        // tabela por ponto (as primeiras linhas; o CSV leva todas)
        egui::CollapsingHeader::new(format!("{} ({} {})", tr("Tabela por ponto"), seq.len(), tr("pontos"))).id_salt("corte_tubo_tabela").show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(160.).show(ui, |ui| {
                egui::Grid::new("corte_tubo_grade").striped(true).show(ui, |ui| {
                    for h in ["#", "X", "A", "S", "bisel", "corta"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for (i, (_, q)) in seq.iter().enumerate().take(400) {
                        let txt = |v: String| if i == atual { RichText::new(v).color(crate::theme::st_warn()) } else { RichText::new(v) };
                        ui.label(txt(format!("{}", i + 1)));
                        ui.label(txt(format!("{:.2}", q.x)));
                        ui.label(txt(format!("{:.2}°", q.a)));
                        ui.label(txt(format!("{:.2}", q.s)));
                        ui.label(txt(format!("{:.1}°", q.bisel)));
                        ui.label(txt(if q.corta { "1".into() } else { "0".into() }));
                        ui.end_row();
                    }
                });
            });
        });
        ui.label(RichText::new(tr("Colisão da tocha com o perfil e corte em linha comum ficam para uma fase posterior; G-code de máquina não é gerado.")).weak().small());
    });
    if csv {
        if let Some(c) = &e.caminho {
            if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("caminho_de_corte.csv").save_file() {
                match std::fs::write(&p, c.csv()) {
                    Ok(()) => app.toast(tr("Caminho de corte exportado"), false),
                    Err(err) => app.toast(format!("{}: {err}", tr("Erro")), true),
                }
            }
        }
    }
    e.aberta = aberta;
    ctx.data_mut(|d| d.insert_temp(id(), e));
}

/// No 3D: a linha do caminho sobre a pele do tubo, setas da tocha a cada tanto e o cabeçote da
/// animação.
pub fn overlay(ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera) {
    let Some(e) = ui.ctx().data(|d| d.get_temp::<Estado>(id())) else { return };
    let (true, Some(c)) = (e.aberta, e.caminho.as_ref()) else { return };
    let p = ui.painter_at(rect);
    let proj = |q: [f64; 3]| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
    let cor = crate::theme::accent(ui.visuals());
    let seq = sequencia(c);
    for w in seq.windows(2) {
        if w[0].0 != w[1].0 {
            continue;
        }
        if let (Some(a), Some(b)) = (proj(w[0].1.p3), proj(w[1].1.p3)) {
            let st = if w[0].1.corta && w[1].1.corta { Stroke::new(2.5, cor) } else { Stroke::new(1., cor.gamma_multiply(0.4)) };
            p.line_segment([a, b], st);
        }
    }
    // setas da tocha: umas 24 por operação
    let seta = |q: &fenkra_solidos::st_caminho_tubo::PontoTocha, l: f64, st: Stroke| {
        let base = [q.p3[0] - q.tocha[0] * l, q.p3[1] - q.tocha[1] * l, q.p3[2] - q.tocha[2] * l];
        if let (Some(a), Some(b)) = (proj(base), proj(q.p3)) {
            p.arrow(a, b - a, st);
        }
    };
    for o in &c.operacoes {
        let passo = (o.pontos.len() / 24).max(1);
        for q in o.pontos.iter().step_by(passo) {
            seta(q, 12., Stroke::new(1., crate::theme::st_ok()));
        }
    }
    let atual = ((seq.len().saturating_sub(1)) as f64 * e.t).round() as usize;
    if let Some((_, q)) = seq.get(atual) {
        seta(q, 30., Stroke::new(3., crate::theme::st_warn()));
    }
}
