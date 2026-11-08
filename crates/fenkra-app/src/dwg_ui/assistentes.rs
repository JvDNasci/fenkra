// SPDX-License-Identifier: LGPL-2.1-or-later
//! Assistentes do desenho (B252): o diálogo de Novo desenho (formato, orientação, norma, método de
//! projeção, escala geral e origem, com miniatura da folha) e o de exportação em PDF, SVG e DXF
//! (folhas, cor ou preto e branco, mapeamento de espessura e prévia). A miniatura é a folha de
//! verdade desenhada pequena: o que se vê no diálogo é o que será criado ou gravado. Janelas livres
//! (movem-se e fecham), abertas só enquanto o comando está em uso.

use super::*;
use crate::theme;
use fenkra_desenho::novo::{NovoDesenho, Projecao, ESCALAS_AUTO};
use fenkra_desenho::opcoes::{self, Cores, Opcoes};

/// Estado do diálogo de Novo desenho.
pub struct NovoUi {
    cfg: NovoDesenho,
    models: Models,
    /// miniatura em cache: a configuração de que veio, as primitivas e o tamanho da folha
    miniatura: Option<(NovoDesenho, Vec<Item>, V2)>,
    /// quadros desde a abertura: nos primeiros a janela é recentrada (a tela pode ainda não ter
    /// tamanho e o tamanho da janela só se conhece depois do primeiro desenho); depois fica livre
    quadros: u8,
}

impl NovoUi {
    pub fn new(origem: Option<Source>) -> Self {
        NovoUi { cfg: NovoDesenho::padrao(&tr("Desenho"), origem), models: Models::new(None), miniatura: None, quadros: 0 }
    }
}

/// Estado do diálogo de exportação.
pub struct ExportUi {
    ext: &'static str,
    cfg: Opcoes,
    /// folhas marcadas (na ordem do desenho)
    marcadas: Vec<bool>,
    /// folha mostrada na prévia
    previa: usize,
    cache: Option<(Opcoes, usize, Vec<Item>, V2)>,
    /// quadros desde a abertura (a janela é recentrada nos primeiros; ver `NovoUi`)
    quadros: u8,
}

impl ExportUi {
    pub fn new(ext: &'static str, d: &Drawing, atual: Id) -> Self {
        let marcadas = d.sheets.iter().map(|s| s.id == atual).collect();
        let previa = d.sheets.iter().position(|s| s.id == atual).unwrap_or(0);
        ExportUi { ext, cfg: Opcoes::default(), marcadas, previa, cache: None, quadros: 0 }
    }

    fn opcoes(&self, d: &Drawing) -> Opcoes {
        let folhas: Vec<Id> = d.sheets.iter().zip(&self.marcadas).filter(|(_, m)| **m).map(|(s, _)| s.id).collect();
        Opcoes { folhas, ..self.cfg.clone() }
    }
}

/// Resultado de uma janela que pede algo ao resto do app.
pub enum Pedido {
    /// desenho novo e a origem das vistas
    Criar(Drawing, Option<Source>),
}

/// Pinta a folha pequena num quadro: papel, linhas e (se couber) os textos maiores.
fn miniatura(ui: &mut egui::Ui, items: &[Item], tam: V2, largura: f32) {
    let z = largura / tam[0] as f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(largura, tam[1] as f32 * z), egui::Sense::hover());
    let pintor = ui.painter_at(rect.expand(2.));
    pintor.rect_filled(rect.translate(Vec2::new(3., 3.)), 0., Color32::from_black_alpha(60));
    pintor.rect_filled(rect, 0., Color32::WHITE);
    let to = |p: V2| Pos2::new(rect.min.x + p[0] as f32 * z, rect.max.y - p[1] as f32 * z);
    for it in items {
        let cor = it.color.map(|c| Color32::from_rgb(c[0], c[1], c[2])).unwrap_or(Color32::BLACK);
        match &it.prim {
            Prim::Line { pts, style } => {
                let w = (it.width.unwrap_or(style.width()) as f32 * z * 2.).clamp(0.5, 3.);
                // segmento a segmento: nas curvas finas a junção em ângulo agudo do traçado contínuo
                // vira espinho; a prévia é pequena e a falta de junção não aparece
                let pts: Vec<Pos2> = pts.iter().map(|p| to(*p)).collect();
                for par in pts.windows(2) {
                    pintor.line_segment([par[0], par[1]], Stroke::new(w, cor));
                }
            }
            Prim::Fill { pts } => super::simbolos::preencher(&pintor, pts.iter().map(|p| to(*p)).collect(), cor),
            Prim::Text { pos, text, size, align, .. } => {
                let px = *size as f32 * z / 0.72;
                if px < 5. {
                    continue;
                }
                let g = pintor.layout_no_wrap(text.clone(), egui::FontId::proportional(px), cor);
                let fx = match align {
                    Align::Left => 0.,
                    Align::Center => -g.size().x / 2.,
                    Align::Right => -g.size().x,
                };
                pintor.galley(to(*pos) + Vec2::new(fx, -px * 0.95), g, cor);
            }
        }
    }
    pintor.rect_stroke(rect, 0., Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Outside);
}

fn nome_da_origem(o: &Option<Source>) -> String {
    match o {
        Some(s) => std::path::Path::new(&s.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| s.path.clone()),
        None => tr("folha em branco").to_string(),
    }
}

/// Diálogo de Novo desenho. Devolve o pedido de criação quando a pessoa confirma.
fn janela_novo(ctx: &egui::Context, novo: &mut Option<NovoUi>, aberta: Option<Source>) -> Option<Pedido> {
    let n = novo.as_mut()?;
    let (mut open, mut criar, mut cancelar) = (true, false, false);
    let centro = ctx.content_rect().center();
    let mut janela = egui::Window::new(tr("Novo desenho")).pivot(egui::Align2::CENTER_CENTER).default_pos(centro);
    if n.quadros < 4 {
        janela = janela.anchor(egui::Align2::CENTER_CENTER, [0., 0.]);
        n.quadros += 1;
        ctx.request_repaint();
    }
    janela.open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(340.);
                egui::Grid::new("novo_campos").num_columns(2).spacing([10., 8.]).show(ui, |ui| {
                    ui.label(tr("Nome"));
                    ui.add(egui::TextEdit::singleline(&mut n.cfg.nome).desired_width(190.)).on_hover_text(tr("Nome do desenho; também vira o nome do arquivo ao salvar"));
                    ui.end_row();
                    ui.label(tr("Norma"));
                    egui::ComboBox::from_id_salt("novo_norma").selected_text(n.cfg.norma.label()).width(190.).show_ui(ui, |ui| {
                        for s in fenkra_desenho::TitleStyle::ALL {
                            if ui.selectable_label(n.cfg.norma == s, s.label()).on_hover_text(tr("Modelo da legenda e do desenho: traz o formato e o método de projeção da família")).clicked() {
                                n.cfg.aplicar_norma(s);
                            }
                        }
                    }).response.on_hover_text(tr("Modelo da legenda e do desenho: traz o formato e o método de projeção da família"));
                    ui.end_row();
                    ui.label(tr("Formato"));
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("novo_formato").selected_text(n.cfg.formato.label()).show_ui(ui, |ui| {
                            for f in Format::ALL {
                                ui.selectable_value(&mut n.cfg.formato, f, f.label());
                            }
                        }).response.on_hover_text(tr("Tamanho da folha"));
                        ui.selectable_value(&mut n.cfg.retrato, false, tr("Paisagem"));
                        ui.selectable_value(&mut n.cfg.retrato, true, tr("Retrato"));
                    });
                    ui.end_row();
                    ui.label(tr("Projeção"));
                    egui::ComboBox::from_id_salt("novo_projecao").selected_text(tr(n.cfg.projecao.label())).width(190.).show_ui(ui, |ui| {
                        for p in Projecao::ALL {
                            ui.selectable_value(&mut n.cfg.projecao, p, tr(p.label()));
                        }
                    }).response.on_hover_text(tr("Onde ficam a vista superior e a lateral em relação à frente"));
                    ui.end_row();
                    ui.label(tr("Escala geral"));
                    egui::ComboBox::from_id_salt("novo_escala").selected_text(n.cfg.escala.map_or_else(|| tr("Automática").to_string(), scale_text)).show_ui(ui, |ui| {
                        ui.selectable_value(&mut n.cfg.escala, None, tr("Automática"));
                        for s in ESCALAS_AUTO {
                            ui.selectable_value(&mut n.cfg.escala, Some(s), scale_text(s));
                        }
                    }).response.on_hover_text(tr("Automática: a maior escala em que as três vistas cabem na folha"));
                    ui.end_row();
                    ui.label(tr("Origem"));
                    ui.vertical(|ui| {
                        ui.label(nome_da_origem(&n.cfg.origem));
                        if let Some(a) = aberta.clone().filter(|a| n.cfg.origem.as_ref() != Some(a)) {
                            if ui.small_button(tr("usar a peça aberta")).on_hover_text(tr("As três vistas saem da peça ou montagem aberta no 3D")).clicked() {
                                n.cfg.origem = Some(a);
                            }
                        }
                        if ui.small_button(tr("outro arquivo…")).clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter(tr("Peça ou montagem"), &["fkp", "fka"]).pick_file() {
                                n.cfg.origem = Some(Source { path: p.to_string_lossy().to_string(), display_state: String::new(), hidden: vec![] });
                            }
                        }
                        if n.cfg.origem.is_some() && ui.small_button(tr("folha em branco")).on_hover_text(tr("Cria só a folha, sem vistas")).clicked() {
                            n.cfg.origem = None;
                        }
                    });
                    ui.end_row();
                });
                if n.cfg.origem.is_none() {
                    ui.label(RichText::new(tr("Sem origem, o desenho nasce só com a folha e a legenda. Abra uma peça salva para sugerir as três vistas.")).weak().small());
                }
            });
            ui.vertical(|ui| {
                ui.label(RichText::new(tr("Miniatura da folha")).weak().small());
                if n.miniatura.as_ref().is_none_or(|m| m.0 != n.cfg) {
                    let d = n.cfg.criar(&mut n.models);
                    let items = layout::sheet(&d, d.sheets[0].id, &mut n.models);
                    n.miniatura = Some((n.cfg.clone(), items, n.cfg.tamanho()));
                }
                if let Some((_, items, tam)) = &n.miniatura {
                    miniatura(ui, items, *tam, 280.);
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(RichText::new(tr("Criar desenho")).color(theme::accent(ui.visuals()))).on_hover_text(tr("Cria o desenho com estas escolhas (Enter)")).clicked() {
                criar = true;
            }
            if ui.button(tr("Cancelar")).clicked() {
                cancelar = true;
            }
        });
        if !ui.ctx().egui_wants_keyboard_input() && ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
            criar = true;
        }
    });
    if criar {
        let n = novo.take()?;
        let mut models = n.models;
        let d = n.cfg.criar(&mut models);
        return Some(Pedido::Criar(d, n.cfg.origem));
    }
    if !open || cancelar {
        *novo = None;
    }
    None
}

/// Diálogo de exportação. Devolve a mensagem do resultado (texto, erro?) quando exporta.
fn janela_exportar(ctx: &egui::Context, st: &mut DwgState) -> Option<(String, bool)> {
    let mut ex = st.export.take()?;
    let (mut open, mut exportar, mut cancelar) = (true, false, false);
    let nomes: Vec<String> = st.d.sheets.iter().map(|s| s.name.clone()).collect();
    let centro = ctx.content_rect().center();
    let mut janela = egui::Window::new(format!("{} {}", tr("Exportar"), ex.ext.to_uppercase())).pivot(egui::Align2::CENTER_CENTER).default_pos(centro);
    if ex.quadros < 4 {
        janela = janela.anchor(egui::Align2::CENTER_CENTER, [0., 0.]);
        ex.quadros += 1;
        ctx.request_repaint();
    }
    janela.open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(250.);
                ui.label(RichText::new(tr("Folhas")).strong());
                for (i, nome) in nomes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut ex.marcadas[i], nome).on_hover_text(tr("Marque as folhas que entram no arquivo"));
                        if ui.selectable_label(ex.previa == i, "👁").on_hover_text(tr("Mostrar na prévia")).clicked() {
                            ex.previa = i;
                        }
                    });
                }
                if ex.ext != "pdf" && ex.marcadas.iter().filter(|m| **m).count() > 1 {
                    ui.label(RichText::new(tr("Neste formato cada folha vira um arquivo, com o nome da folha no nome.")).weak().small());
                }
                ui.add_space(6.);
                ui.label(RichText::new(tr("Cor")).strong());
                ui.horizontal_wrapped(|ui| {
                    for c in Cores::ALL {
                        ui.selectable_value(&mut ex.cfg.cores, c, tr(c.label())).on_hover_text(tr("Preto e branco: linhas e textos pretos; o sombreado claro some"));
                    }
                });
                ui.add_space(6.);
                ui.label(RichText::new(tr("Espessura das linhas")).strong());
                ui.horizontal(|ui| {
                    for (nome, f) in [("Como está", 1.), ("Fina", 0.6), ("Grossa", 1.6)] {
                        if ui.selectable_label((ex.cfg.fator_espessura - f).abs() < 1e-9, tr(nome)).on_hover_text(tr("Atalho para o multiplicador de espessura")).clicked() {
                            ex.cfg.fator_espessura = f;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Multiplicador"));
                    ui.add(crate::ui_ext::slider(&mut ex.cfg.fator_espessura, 0.25..=4.).logarithmic(true)).on_hover_text(tr("Multiplica a espessura de todas as linhas"));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Mínima (mm)"));
                    ui.add(crate::ui_ext::slider(&mut ex.cfg.espessura_minima, 0.0..=1.0)).on_hover_text(tr("Nenhuma linha sai mais fina que isto (0 = sem mínimo)"));
                });
            });
            ui.vertical(|ui| {
                ui.label(RichText::new(tr("Prévia")).weak().small());
                let o = ex.opcoes(&st.d);
                let chave = Opcoes { folhas: vec![], ..o.clone() };
                if ex.cache.as_ref().is_none_or(|c| c.0 != chave || c.1 != ex.previa) {
                    if let Some(sh) = st.d.sheets.get(ex.previa) {
                        let (sid, tam) = (sh.id, sh.size());
                        let items = o.aplicar(&layout::sheet(&st.d, sid, &mut st.models));
                        ex.cache = Some((chave, ex.previa, items, tam));
                    }
                }
                if let Some((_, _, items, tam)) = &ex.cache {
                    miniatura(ui, items, *tam, 320.);
                }
            });
        });
        ui.separator();
        let alguma = ex.marcadas.iter().any(|m| *m);
        ui.horizontal(|ui| {
            if ui.add_enabled(alguma, egui::Button::new(RichText::new(tr("Exportar…")).color(theme::accent(ui.visuals())))).on_hover_text(tr("Escolhe o arquivo e grava")).clicked() {
                exportar = true;
            }
            if ui.button(tr("Cancelar")).clicked() {
                cancelar = true;
            }
        });
    });
    let mut msg = None;
    if exportar {
        let nome = format!("{}.{}", st.d.name, ex.ext);
        if let Some(p) = rfd::FileDialog::new().add_filter(ex.ext.to_uppercase(), &[ex.ext]).set_file_name(nome).save_file() {
            let o = ex.opcoes(&st.d);
            msg = Some(match opcoes::salvar_com(&st.d, &mut st.models, &p.to_string_lossy(), &o) {
                Ok(f) => (format!("{} {}", tr("Exportado:"), f.join(", ")), false),
                Err(e) => (e, true),
            });
            return msg;
        }
    }
    if open && !cancelar {
        st.export = Some(ex);
    }
    msg
}

/// Abre o diálogo de Novo desenho.
pub(super) fn abrir_novo(st: &mut DwgState, aberta: Option<Source>) {
    st.novo = Some(NovoUi::new(aberta));
}

/// Abre o diálogo de exportação do formato (`pdf`, `svg` ou `dxf`).
pub(super) fn abrir_exportar(st: &mut DwgState, ext: &'static str) {
    st.export = Some(ExportUi::new(ext, &st.d, st.sheet));
}

/// Desenha os diálogos abertos. Devolve o pedido de criação, se houve, e a mensagem de exportação.
pub(super) fn janelas(ctx: &egui::Context, st: &mut DwgState, aberta: Option<Source>) -> (Option<Pedido>, Option<(String, bool)>) {
    let pedido = janela_novo(ctx, &mut st.novo, aberta);
    let msg = janela_exportar(ctx, st);
    (pedido, msg)
}
