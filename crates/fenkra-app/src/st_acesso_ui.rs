// SPDX-License-Identifier: LGPL-2.1-or-later
//! B136: comando "Escada e guarda-corpo". Painel livre aberto só durante o comando: escolhe escada ou
//! plataforma, mostra os itens da NR-12 que passam ou não, desenha a prévia das linhas no 3D e, ao
//! concluir, põe as linhas e os membros no documento (com desfazer).

use crate::app::App;
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, RichText, Stroke};
use fenkra_solidos::st_acesso::*;

/// O que o comando gera.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modo {
    Escada,
    Plataforma,
}

/// Estado do painel (guardado no contexto da interface enquanto o comando está aberto).
#[derive(Clone, Debug)]
pub struct Estado {
    pub modo: Modo,
    pub escada: Escada,
    pub guarda: GuardaCorpo,
    pub com_guarda: bool,
    /// plataforma: comprimento e largura útil; altura do piso
    pub comprimento: f64,
    pub largura: f64,
    pub piso: f64,
    /// lugar na peça: ponto e giro em torno de Z
    pub origem: [f64; 3],
    pub giro: f64,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { modo: Modo::Escada, escada: Escada::default(), guarda: GuardaCorpo::default(), com_guarda: true, comprimento: 2000., largura: 1000., piso: 1500., origem: [0.; 3], giro: 0. }
    }
}

fn id_estado() -> egui::Id {
    egui::Id::unique("st_acesso_estado")
}

/// Borda da plataforma com guarda-corpo: os três lados que não dão para a escada (o lado de y = 0 fica
/// aberto para a chegada).
pub fn borda_da_plataforma(e: &Estado) -> Vec<[f64; 3]> {
    let (a, b, z) = (e.largura / 2., e.comprimento, e.piso);
    vec![[-a, 0., z], [-a, b, z], [a, b, z], [a, 0., z]]
}

/// Peças e avisos do estado atual.
pub fn gerar(e: &Estado) -> Result<(Vec<Peca>, Vec<Aviso>, Option<Dimensoes>), String> {
    match e.modo {
        Modo::Escada => {
            let esc = Escada { guarda: e.com_guarda.then(|| e.guarda.clone()), ..e.escada.clone() };
            let (d, p) = pecas_escada(&esc)?;
            let v = verificar(&esc, &d);
            Ok((p, v, Some(d)))
        }
        Modo::Plataforma => {
            let mut v = verificar_guarda(&e.guarda);
            if e.largura < 600. - 1e-6 {
                v.push(Aviso { item: "NR-12 Anexo III 10 a".into(), texto: format!("largura útil {:.0} mm abaixo de 600 mm", e.largura), fora: true });
            }
            // a borda corre no sentido horário visto de cima: o lado de fora fica à esquerda
            Ok((guarda_corpo(&borda_da_plataforma(e), false, &e.guarda, "Plataforma: "), v, None))
        }
    }
}

/// Abre o comando.
pub fn abrir(app: &mut App) {
    app.st_acesso_open = true;
}

fn campo(ui: &mut egui::Ui, nome: &str, v: &mut f64, faixa: std::ops::RangeInclusive<f64>, dica: &str) {
    ui.label(tr(nome));
    let r = ui.add(crate::ui_ext::dv(v).speed(5.).range(faixa).suffix(" mm").max_decimals(1).dica(tr(dica)));
    crate::ui_ext::wheel(ui, &r, v);
    ui.end_row();
}

pub fn window(app: &mut App, ctx: &egui::Context) {
    if !app.st_acesso_open {
        ctx.data_mut(|d| d.remove::<Estado>(id_estado()));
        return;
    }
    let mut e: Estado = ctx.data(|d| d.get_temp(id_estado())).unwrap_or_default();
    let mut open = true;
    let mut concluir = false;
    egui::Window::new(tr("Escada e guarda-corpo")).id(egui::Id::unique("janela_st_acesso")).open(&mut open).default_size([380., 520.]).resizable(true).show(ctx, |ui| {
        concluir = crate::app::done_bar(ui);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut e.modo, Modo::Escada, tr("Escada")).on_hover_text(tr("Escada de degraus com longarinas, descansos a cada 3 m de altura e guarda-corpo dos dois lados."));
            ui.selectable_value(&mut e.modo, Modo::Plataforma, tr("Plataforma")).on_hover_text(tr("Guarda-corpo em três lados de uma plataforma; o lado da chegada fica aberto."));
        });
        egui::Grid::new("st_acesso_campos").num_columns(2).show(ui, |ui| match e.modo {
            Modo::Escada => {
                ui.label(tr("Tipo"));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut e.escada.tipo, TipoEscada::ComEspelho, tr("com espelho")).on_hover_text(tr("Degrau fechado na frente (NR-12, Anexo III, item 12): altura de 200 a 250 mm e piso de no mínimo 200 mm."));
                    ui.selectable_value(&mut e.escada.tipo, TipoEscada::SemEspelho, tr("sem espelho")).on_hover_text(tr("Degrau vazado (item 11): altura até 250 mm, piso de no mínimo 150 mm e 600 ≤ g + 2h ≤ 660 mm."));
                });
                ui.end_row();
                campo(ui, "Desnível", &mut e.escada.altura, 100.0..=30000.0, "Altura do piso de baixo ao piso de chegada. O gerador divide em degraus iguais.");
                campo(ui, "Largura útil", &mut e.escada.largura, 300.0..=3000.0, "Largura livre entre as longarinas. A NR-12 pede no mínimo 600 mm (500 mm em lance único abaixo de 1,50 m).");
                campo(ui, "Espelho máximo", &mut e.escada.espelho_max, 100.0..=300.0, "Altura entre degraus que não deve ser passada; o número de degraus sai do desnível.");
                let mut auto = e.escada.piso.is_none();
                ui.label(tr("Piso"));
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut auto, tr("pela fórmula")).on_hover_text(tr("Profundidade livre pelo meio da faixa 600 ≤ g + 2h ≤ 660 mm.")).changed() {
                        e.escada.piso = if auto { None } else { Some(250.) };
                    }
                    if let Some(g) = e.escada.piso.as_mut() {
                        let r = ui.add(crate::ui_ext::dv(g).speed(5.).range(100.0..=500.0).suffix(" mm").dica(tr("Profundidade livre do degrau (g).")));
                        crate::ui_ext::wheel(ui, &r, g);
                    }
                });
                ui.end_row();
                campo(ui, "Descanso", &mut e.escada.descanso, 300.0..=5000.0, "Comprimento da plataforma de descanso entre lances. A NR-12 pede no mínimo 600 mm, a cada 3 m de altura no máximo.");
            }
            Modo::Plataforma => {
                campo(ui, "Comprimento", &mut e.comprimento, 300.0..=30000.0, "Comprimento da plataforma, medido a partir do lado aberto.");
                campo(ui, "Largura útil", &mut e.largura, 300.0..=10000.0, "Largura livre da plataforma. A NR-12 pede no mínimo 600 mm.");
                campo(ui, "Altura do piso", &mut e.piso, 0.0..=30000.0, "Altura do piso da plataforma na peça.");
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if e.modo == Modo::Escada {
                ui.checkbox(&mut e.com_guarda, tr("Guarda-corpo")).on_hover_text(tr("Rodapé, montantes, travessão intermediário e superior dos dois lados."));
            } else {
                ui.label(RichText::new(tr("Guarda-corpo")).strong());
            }
            ui.selectable_value(&mut e.guarda.norma, NormaGuarda::Nr12, "NR-12").on_hover_text(tr("Meio de acesso a máquina: superior de 1100 a 1200 mm, intermediário a 700 mm, rodapé de 200 mm."));
            ui.selectable_value(&mut e.guarda.norma, NormaGuarda::Nbr14718, "NBR 14718").on_hover_text(tr("Guarda-corpo de edificação: só a altura mínima de 1100 mm é conferida; vãos, parte escalável e cargas ficam por sua conta."));
        });
        if e.com_guarda || e.modo == Modo::Plataforma {
            egui::Grid::new("st_acesso_guarda").num_columns(2).show(ui, |ui| {
                campo(ui, "Altura", &mut e.guarda.altura, 500.0..=2000.0, "Topo do travessão superior, medido do piso (na escada, da linha dos bocéis).");
                campo(ui, "Travessão intermediário", &mut e.guarda.intermediario, 100.0..=1500.0, "Centro do travessão intermediário, medido do piso.");
                campo(ui, "Rodapé", &mut e.guarda.rodape, 50.0..=500.0, "Altura da barra do rodapé.");
                campo(ui, "Vão dos montantes", &mut e.guarda.vao_montantes, 300.0..=3000.0, "Distância máxima entre montantes. A NR-12 não fixa: é escolha do projeto.");
            });
        }
        ui.separator();
        egui::Grid::new("st_acesso_lugar").num_columns(2).show(ui, |ui| {
            for (k, n) in ["X", "Y", "Z"].iter().enumerate() {
                ui.label(format!("{} {n}", tr("Origem")));
                let r = ui.add(crate::ui_ext::dv(&mut e.origem[k]).speed(5.).suffix(" mm").dica(tr("Ponto da peça onde fica a frente do primeiro degrau (ou o meio do lado aberto da plataforma).")));
                crate::ui_ext::wheel(ui, &r, &mut e.origem[k]);
                ui.end_row();
            }
            ui.label(tr("Giro"));
            let r = ui.add(crate::ui_ext::dv(&mut e.giro).speed(1.).range(-360.0..=360.0).suffix("°").dica(tr("Giro em torno do eixo Z: 0° sobe no sentido +Y.")));
            crate::ui_ext::wheel(ui, &r, &mut e.giro);
            ui.end_row();
        });
        ui.separator();
        match gerar(&e) {
            Ok((p, v, d)) => {
                if let Some(d) = d {
                    let lances = d.lances.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(" + ");
                    ui.label(format!("{} {} {:.1} mm · {} {:.0} mm · {:.1}° · {} {lances}", d.espelhos, tr("espelhos de"), d.h, tr("piso"), d.g, d.angulo, tr("espelhos por lance:")));
                    ui.label(RichText::new(format!("{} {:.0} mm", tr("comprimento em planta"), d.comprimento)).weak());
                }
                ui.label(RichText::new(format!("{} {}", p.len(), tr("membros a criar"))).weak());
                if v.iter().all(|a| !a.fora) {
                    ui.label(RichText::new(tr("✔ Atende os itens conferidos do Anexo III da NR-12")).color(crate::theme::st_ok()));
                }
                for a in &v {
                    let cor = if a.fora { crate::theme::st_err() } else { crate::theme::st_warn() };
                    ui.label(RichText::new(format!("{} {}: {}", if a.fora { "✖" } else { "ⓘ" }, a.item, tr(&a.texto))).color(cor).small());
                }
            }
            Err(err) => {
                ui.label(RichText::new(format!("⚠ {}", tr(&err))).color(crate::theme::st_err()));
            }
        }
        ui.label(RichText::new(tr("Os valores conferidos são os do Anexo III da NR-12 (meios de acesso a máquinas). Cálculo de resistência e fixação ficam com o projetista.")).weak().small());
    });
    if concluir {
        if let Ok((p, _, _)) = gerar(&e) {
            let antes = app.open.doc.clone();
            let ids = inserir(&mut app.open.doc, &p, e.origem, e.giro);
            app.push_undo_pub(antes);
            app.toast(format!("{} {}", ids.len(), tr("membros criados")), false);
        }
        open = false;
    }
    if open {
        ctx.data_mut(|d| d.insert_temp(id_estado(), e));
    } else {
        app.st_acesso_open = false;
        ctx.data_mut(|d| d.remove::<Estado>(id_estado()));
    }
}

/// Prévia no 3D: as linhas guia de todas as peças, na cor de destaque do tema.
pub fn overlay(app: &App, ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera) {
    if !app.st_acesso_open {
        return;
    }
    let Some(e) = ui.ctx().data(|d| d.get_temp::<Estado>(id_estado())) else { return };
    let Ok((pecas, _, _)) = gerar(&e) else { return };
    let cor = crate::theme::accent(ui.visuals());
    let painter = ui.painter_at(rect);
    for p in &pecas {
        for l in &p.linhas {
            let tela: Vec<egui::Pos2> = l.iter().filter_map(|v| {
                let q = posicionar(*v, e.origem, e.giro);
                cam.project(rect, glam::Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32))
            }).collect();
            if tela.len() >= 2 {
                painter.add(egui::Shape::line(tela, Stroke::new(2., cor)));
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn plataforma_estreita_reprova_pelo_item_10() {
        let e = Estado { modo: Modo::Plataforma, largura: 500., ..Default::default() };
        let (p, v, _) = gerar(&e).unwrap();
        assert!(v.iter().any(|a| a.item == "NR-12 Anexo III 10 a"));
        assert_eq!(p.len(), 4, "rodapé, montantes e dois travessões");
        // o lado aberto (y = 0) não tem rodapé: a borda começa e termina nele
        let b = borda_da_plataforma(&e);
        assert_eq!(b.first().unwrap()[1], 0.);
        assert_eq!(b.last().unwrap()[1], 0.);
    }

    #[test]
    fn escada_padrao_atende_e_sem_guarda_reprova() {
        let mut e = Estado::default();
        let (_, v, d) = gerar(&e).unwrap();
        assert!(v.is_empty() && d.is_some(), "{v:?}");
        e.com_guarda = false;
        let (_, v, _) = gerar(&e).unwrap();
        assert!(v.iter().any(|a| a.item == "NR-12 Anexo III 7"));
    }
}
