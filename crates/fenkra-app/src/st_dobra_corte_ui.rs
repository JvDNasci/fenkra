// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: tubo aberto e gabarito (B328, B329, B333, B334). Na janela do nó de dobra, mostra o
//! tubo fechado (o sólido do 3D) ou aberto (planificado com os contornos de corte) e exporta o
//! contorno em DXF e SVG para laser e o gabarito de marcação em PDF e SVG, em escala 1:1.
//!
//! A geometria vem de derivação sem protótipo (`docs/pesquisa/dobra-de-tubos.md`): o painel diz,
//! sempre, que está "a validar em peça real".

use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, Color32, RichText, Stroke};
use fenkra_solidos::st_dobra::{DobraDoTubo, TuboDobrado, AVISO_DERIVADA};
use fenkra_solidos::st_dobra_corte::{self, Camada, Desenho};
use fenkra_solidos::st_dobra_saida;

/// A dobra tem corte plano para mostrar e exportar?
pub fn tem_corte(d: &DobraDoTubo) -> bool {
    st_dobra_corte::tem_corte(&d.resultado)
}

fn cor(ui: &egui::Ui, c: Camada) -> Color32 {
    match c {
        Camada::Corte => theme::st_err(),
        Camada::Marcacao => theme::accent(ui.visuals()),
        Camada::Dobra => theme::st_ok(),
        Camada::Contorno => ui.visuals().weak_text_color(),
        Camada::Cota => ui.visuals().text_color(),
    }
}

/// Desenha o planificado no espaço do painel, enquadrando a região dos cortes.
fn desenhar(ui: &mut egui::Ui, d: &Desenho) {
    let cortes: Vec<[f64; 2]> = d.tracos.iter().filter(|t| t.camada == Camada::Corte).flat_map(|t| t.pts.iter().copied()).collect();
    if cortes.is_empty() {
        return;
    }
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in &cortes {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    // enquadra os cortes, com folga dos lados e a faixa da placa embaixo
    lo[0] -= 12.;
    hi[0] += 12.;
    lo[1] = (lo[1] - 3.).min(0.);
    hi[1] += 3.;
    let (bw, bh) = ((hi[0] - lo[0]).max(1.), (hi[1] - lo[1]).max(1.));
    let largura = ui.available_width().clamp(160., 520.);
    let escala = (largura as f64 / bw).min(190. / bh);
    let tam = egui::vec2((bw * escala) as f32, (bh * escala) as f32);
    let (resp, pintor) = ui.allocate_painter(tam, egui::Sense::hover());
    pintor.rect_filled(resp.rect, 2.0, ui.visuals().extreme_bg_color);
    let mapa = |p: [f64; 2]| egui::pos2(resp.rect.left() + ((p[0] - lo[0]) * escala) as f32, resp.rect.bottom() - ((p[1] - lo[1]) * escala) as f32);
    let pintor = pintor.with_clip_rect(resp.rect);
    for camada in [Camada::Contorno, Camada::Dobra, Camada::Marcacao, Camada::Corte] {
        let largura_traco = if camada == Camada::Corte { 1.6 } else { 1.0 };
        for t in d.tracos.iter().filter(|t| t.camada == camada) {
            let mut pts: Vec<egui::Pos2> = t.pts.iter().map(|p| mapa(*p)).collect();
            if t.fechado {
                if let Some(p0) = pts.first().copied() {
                    pts.push(p0);
                }
            }
            if t.tracejado {
                for par in pts.windows(2) {
                    pintor.extend(egui::Shape::dashed_line(&[par[0], par[1]], Stroke::new(largura_traco, cor(ui, camada)), 4.0, 3.0));
                }
            } else {
                pintor.add(egui::Shape::line(pts, Stroke::new(largura_traco, cor(ui, camada))));
            }
        }
    }
    resp.on_hover_text(tr("Tubo desenrolado em escala, com a região dos cortes ampliada. Vermelho: corte; azul: marcação; verde: zona de dobra; cinza: contorno do tubo."));
}

fn salvar(ui: &egui::Ui, nome: &str, filtro: (&str, &str), bytes: Vec<u8>) {
    let id = egui::Id::new("st_dobra_saida_msg");
    let Some(caminho) = rfd::FileDialog::new().add_filter(filtro.0, &[filtro.1]).set_file_name(nome).save_file() else { return };
    let msg = match std::fs::write(&caminho, bytes) {
        Ok(()) => format!("{} {}", tr("Salvo em"), caminho.display()),
        Err(e) => format!("{} {e}", tr("Falha ao salvar:")),
    };
    ui.ctx().data_mut(|d| d.insert_temp(id, msg));
}

/// Painel do tubo aberto e dos arquivos de corte e gabarito da dobra `dobra` do tubo `t`.
pub fn painel(ui: &mut egui::Ui, t: &TuboDobrado, dobra: &DobraDoTubo, joint: usize) {
    if !tem_corte(dobra) {
        return;
    }
    ui.add_space(4.);
    ui.label(RichText::new(format!("⚠ {}", tr(AVISO_DERIVADA))).color(theme::st_warn()))
        .on_hover_text(tr("As fórmulas do corte vêm de derivação própria, conferida só por conta numérica. Faça um protótipo em peça real antes de cortar a série."));
    egui::CollapsingHeader::new(tr("Tubo aberto e gabarito")).id_salt(("st_dobra_aberto_cab", joint)).default_open(false).show(ui, |ui| {
        let chave = egui::Id::new(("st_dobra_aberto", joint));
        let mut aberto: bool = ui.ctx().data(|d| d.get_temp(chave)).unwrap_or(false);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut aberto, false, tr("Fechado (3D)")).on_hover_text(tr("O tubo dobrado: é o sólido que aparece no 3D."));
            ui.selectable_value(&mut aberto, true, tr("Aberto (planificado)")).on_hover_text(tr("O tubo reto antes de dobrar, com os cortes nas paredes, em escala real."));
        });
        ui.ctx().data_mut(|d| d.insert_temp(chave, aberto));
        let plano = st_dobra_corte::planificado(t);
        if !aberto {
            ui.label(RichText::new(tr("O tubo fechado é o sólido que aparece no 3D.")).weak().small());
        } else {
            match &plano {
                Ok(d) => {
                    desenhar(ui, d);
                    ui.label(RichText::new(format!("{} {:.1} mm", tr("Comprimento de corte no desenho:"), d.comprimento_de_corte())).weak().small())
                        .on_hover_text(tr("Soma dos traços de corte (camada CORTE) dos contornos exibidos."));
                }
                Err(e) => {
                    ui.label(RichText::new(format!("⛔ {e}")).color(theme::st_err()));
                }
            }
        }
        ui.add_space(2.);
        let nome_base = t.designacao.replace(' ', "_");
        ui.horizontal_wrapped(|ui| {
            if let Ok(d) = &plano {
                if ui.button(tr("Contorno (DXF)")).on_hover_text(tr("Contorno plano dos cortes para laser de tubo, com camadas CORTE, MARCACAO e DOBRA, em milímetros.")).clicked() {
                    salvar(ui, &format!("{nome_base}-contorno.dxf"), ("DXF", "dxf"), st_dobra_saida::dxf(d).into_bytes());
                }
                if ui.button(tr("Contorno (SVG)")).on_hover_text(tr("O mesmo contorno em SVG, em milímetros reais.")).clicked() {
                    salvar(ui, &format!("{nome_base}-contorno.svg"), ("SVG", "svg"), st_dobra_saida::svg(d).into_bytes());
                }
            }
            if let Ok(g) = st_dobra_corte::gabarito(t) {
                let folhas = st_dobra_saida::folhas(&g);
                if ui.button(tr("Gabarito 1:1 (PDF)")).on_hover_text(tr("Régua e contorno em escala real para riscar o tubo na serralheria. Imprima a 100%, sem ajustar à página, e confira a barra de 100 mm.")).clicked() {
                    salvar(ui, &format!("{nome_base}-gabarito.pdf"), ("PDF", "pdf"), st_dobra_saida::pdf(&g));
                }
                if ui.button(tr("Gabarito (SVG)")).on_hover_text(tr("O mesmo gabarito em SVG, em milímetros reais.")).clicked() {
                    salvar(ui, &format!("{nome_base}-gabarito.svg"), ("SVG", "svg"), st_dobra_saida::svg(&g).into_bytes());
                }
                if folhas.0 * folhas.1 > 1 {
                    ui.label(RichText::new(format!("{} {}", folhas.0 * folhas.1, tr("folhas A4"))).weak().small()).on_hover_text(tr("O gabarito não cabe numa folha: o PDF vem dividido, com marcas de canto para colar."));
                }
            }
        });
        if let Some(m) = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::new("st_dobra_saida_msg"))) {
            ui.label(RichText::new(m).weak().small());
        }
    });
}
