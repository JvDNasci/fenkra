// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: janela "Cantos da estrutura" (B37). Lista os cantos onde linhas de membros se
//! encontram, realça cada um no 3D pela classe (dois membros, três ou mais, tratado, ignorado) e abre
//! um cartão com o tratamento, quem continua, a folga e o ângulo lido da geometria. O modo automático
//! agrupa cantos iguais; "Limpar tudo" devolve cada canto ao tratamento padrão do comando.

use crate::app::App;
use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, Color32, RichText};
use fenkra_core::CornerMode;
use fenkra_solidos::st_cantos::{self, Canto, ClasseCanto};

/// Cor do realce de cada classe, só pelo tema (legível em fundo claro e escuro).
pub fn cor_da_classe(c: ClasseCanto, v: &egui::Visuals) -> Color32 {
    match c {
        ClasseCanto::DoisMembros => v.text_color(),
        ClasseCanto::TresOuMais => theme::papel_ferramenta(),
        ClasseCanto::Tratado => theme::st_ok(),
        ClasseCanto::Ignorado => v.weak_text_color(),
    }
}

fn nome_da_classe(c: ClasseCanto) -> &'static str {
    match c {
        ClasseCanto::DoisMembros => "Dois membros",
        ClasseCanto::TresOuMais => "Três ou mais",
        ClasseCanto::Tratado => "Tratado",
        ClasseCanto::Ignorado => "Ignorado",
    }
}

fn nome_do_modo(m: CornerMode) -> &'static str {
    match m {
        CornerMode::MeiaEsquadria => "Meia-esquadria",
        CornerMode::Aparar => "Aparar",
        CornerMode::Encaixe => "Encaixe",
        CornerMode::Nenhum => "Nenhum",
        CornerMode::Dobra => "Dobra de tubo",
    }
}

/// Tratamentos oferecidos no cartão; `None` = volta ao padrão do comando.
const MODOS: [(Option<CornerMode>, &str); 5] = [(None, "padrão"), (Some(CornerMode::MeiaEsquadria), "Meia-esquadria"), (Some(CornerMode::Aparar), "Aparar"), (Some(CornerMode::Encaixe), "Encaixe"), (Some(CornerMode::Nenhum), "Nenhum")];

fn mesmo_ponto(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-4)
}

fn nome_da_linha(app: &App, l: st_cantos::Linha) -> String {
    format!("{} · {} {}", app.doc().name_of(l.0), tr("linha"), l.1 + 1)
}

/// Janela dos cantos. Mantém `app.st_cantos` atualizado enquanto aberta (o 3D desenha a partir dele).
pub fn window(app: &mut App, ctx: &egui::Context) {
    // a prévia do encaixe só vale no quadro em que o cartão a pediu
    crate::st_encaixe_ui::limpar(ctx);
    if !app.st_cantos_open {
        crate::st_limite_ui::limpar(ctx);
        if !app.st_cantos.is_empty() {
            app.st_cantos.clear();
        }
        return;
    }
    app.st_cantos = fenkra_solidos::member_corners(app.doc());
    let cantos = app.st_cantos.clone();
    let auto_id = egui::Id::new("st_cantos_auto");
    let mut auto: bool = ctx.data(|d| d.get_temp(auto_id)).unwrap_or(false);
    let mut open = true;
    let fechar = egui::Id::unique(("concluir_janela", "st3"));
    if ctx.data_mut(|m| m.remove_temp::<bool>(fechar)).unwrap_or(false) {
        open = false;
    }
    // mudança pedida pela interface: aplicada depois de desenhar, com desfazer
    let mut novo: Option<Box<dyn FnOnce(&mut fenkra_core::Document)>> = None;
    let mut aviso: Option<String> = None;
    let mut escolher: Option<[f64; 3]> = None;

    egui::Window::new(tr("⌐ Cantos da estrutura")).id(egui::Id::unique("janela_cantos")).open(&mut open).default_size([420., 460.]).resizable(true).show(ctx, |ui| {
        if crate::app::done_bar(ui) {
            ui.ctx().data_mut(|m| m.insert_temp(fechar, true));
        }
        crate::st_limite_ui::secao(ui, app, &mut novo, &mut aviso);
        ui.separator();
        if cantos.is_empty() {
            ui.label(RichText::new(tr("Nenhum canto: as linhas dos membros não se encontram em pontas.")).weak());
            return;
        }
        // legenda: classe, cor do realce no 3D e quantidade
        let (dois, tres, trat, ign) = st_cantos::contagem(&cantos);
        ui.horizontal_wrapped(|ui| {
            for (c, n) in [(ClasseCanto::DoisMembros, dois), (ClasseCanto::TresOuMais, tres), (ClasseCanto::Tratado, trat), (ClasseCanto::Ignorado, ign)] {
                // ponto e nome juntos: sem o grupo, a quebra de linha separava a cor do rótulo
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12., 12.), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 5.5, cor_da_classe(c, ui.visuals()));
                    ui.label(format!("{} ({n})", tr(nome_da_classe(c))));
                });
                ui.add_space(6.);
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut auto, tr("Automático")).on_hover_text(tr("Agrupa os cantos iguais (mesmos perfis, ângulo e tratamento) e aplica o tratamento ao grupo inteiro."));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let n: usize = app_regras(&cantos);
                if ui.add_enabled(n > 0, egui::Button::new(tr("Limpar tudo"))).on_hover_text(tr("Apaga todas as regras próprias de canto: cada canto volta ao tratamento padrão do comando.")).clicked() {
                    novo = Some(Box::new(|d| {
                        st_cantos::limpar_tudo(d);
                    }));
                    aviso = Some(format!("{} {}", n, tr("regras de canto removidas")));
                }
            });
        });
        ui.separator();
        if auto {
            let grupos = st_cantos::agrupar_iguais(&cantos);
            egui::ScrollArea::vertical().max_height(300.).show(ui, |ui| {
                for (gi, g) in grupos.iter().enumerate() {
                    let c = &cantos[g[0]];
                    let perfil = c.pares.first().map(|p| if p.perfil_a == p.perfil_b { p.perfil_a.clone() } else { format!("{} + {}", p.perfil_a, p.perfil_b) }).unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} × {}", g.len(), perfil)).strong());
                        if let Some(a) = c.angulo {
                            ui.label(format!("{a:.1}°"));
                        }
                        let mut pick: Option<Option<CornerMode>> = None;
                        egui::ComboBox::from_id_salt(("st_cantos_g", gi)).selected_text(tr(nome_do_modo(c.modo))).width(110.).show_ui(ui, |ui| {
                            for (x, n) in MODOS {
                                if ui.selectable_label(x == Some(c.modo) && c.explicito, tr(n)).clicked() {
                                    pick = Some(x);
                                }
                            }
                        });
                        if let Some(modo) = pick {
                            let (cs, g2) = (cantos.clone(), g.clone());
                            novo = Some(Box::new(move |d| {
                                st_cantos::aplicar_ao_grupo(d, &cs, &g2, modo);
                            }));
                            aviso = Some(format!("{} {}", g.len(), tr("cantos atualizados")));
                        }
                        if ui.small_button("◎").on_hover_text(tr("Mostrar no 3D")).clicked() {
                            escolher = Some(c.ponto);
                        }
                    });
                }
            });
        } else {
            egui::ScrollArea::vertical().max_height(180.).show(ui, |ui| {
                for c in &cantos {
                    let sel = app.st_cantos_sel.is_some_and(|p| mesmo_ponto(p, c.ponto));
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.), egui::Sense::click());
                    // B321: anel de foco quando o Tab chega aqui
                    let resp = crate::teclado::anel(ui, resp);
                    if sel || resp.hovered() {
                        ui.painter().rect_filled(r, 3., ui.visuals().widgets.hovered.weak_bg_fill);
                    }
                    ui.painter().circle_filled(r.left_center() + egui::vec2(9., 0.), 5., cor_da_classe(c.classe, ui.visuals()));
                    let ang = c.angulo.map(|a| format!(" · {a:.1}°")).unwrap_or_default();
                    let txt = format!("({:.0}, {:.0}, {:.0}) · {} · {}{}", c.ponto[0], c.ponto[1], c.ponto[2], tr(nome_do_modo(c.modo)), c.linhas.len(), ang);
                    ui.painter().text(r.left_center() + egui::vec2(22., 0.), egui::Align2::LEFT_CENTER, txt, egui::FontId::proportional(12.), ui.visuals().text_color());
                    if resp.clicked() {
                        escolher = Some(c.ponto);
                    }
                }
            });
            ui.separator();
            // cartão do canto escolhido
            let atual = app.st_cantos_sel.and_then(|p| cantos.iter().find(|c| mesmo_ponto(p, c.ponto)));
            match atual {
                None => {
                    ui.label(RichText::new(tr("Clique num canto na lista ou no 3D.")).weak());
                }
                Some(c) => cartao(ui, app, c, &mut novo, &mut aviso),
            }
        }
    });
    ctx.data_mut(|d| d.insert_temp(auto_id, auto));
    if let Some(p) = escolher {
        app.st_cantos_sel = Some(p);
    }
    if let Some(f) = novo {
        let antes = app.open.doc.clone();
        f(&mut app.open.doc);
        app.push_undo_pub(antes);
        if let Some(a) = aviso {
            app.toast(a, false);
        }
    }
    if !open {
        app.st_cantos_open = false;
        app.st_cantos_sel = None;
        app.st_cantos.clear();
    }
}

fn app_regras(cantos: &[Canto]) -> usize {
    cantos.iter().filter(|c| c.explicito).count()
}

/// Cartão do canto: tratamento, quem continua (com a seta para trocar), folga e ângulo.
fn cartao(ui: &mut egui::Ui, app: &App, c: &Canto, novo: &mut Option<Box<dyn FnOnce(&mut fenkra_core::Document)>>, aviso: &mut Option<String>) {
    ui.label(RichText::new(tr(nome_da_classe(c.classe))).strong().color(cor_da_classe(c.classe, ui.visuals())));
    if c.linhas.len() != 2 {
        ui.label(RichText::new(tr("Três ou mais linhas chegam a este ponto: cada par é tratado pelo comando posterior (veja o painel do membro).")).weak());
        for l in &c.linhas {
            ui.label(format!("• {}", nome_da_linha(app, *l)));
        }
        return;
    }
    let (a, b) = (c.linhas[0], c.linhas[1]);
    ui.label(format!("{}  ↔  {}", nome_da_linha(app, a), nome_da_linha(app, b)));
    if let Some(ang) = c.angulo {
        ui.label(format!("{} {ang:.1}°", tr("Ângulo entre os membros:"))).on_hover_text(tr("Lido da direção das duas linhas guia no canto."));
    }
    ui.horizontal(|ui| {
        ui.label(tr("Tratamento"));
        let cur = if c.explicito { Some(c.modo) } else { None };
        let rotulo = if c.explicito { tr(nome_do_modo(c.modo)).to_string() } else { format!("{} ({})", tr("padrão"), tr(nome_do_modo(c.modo))) };
        egui::ComboBox::from_id_salt("st_cantos_modo").selected_text(rotulo).width(150.).show_ui(ui, |ui| {
            for (x, n) in MODOS {
                if ui.selectable_label(x == cur, tr(n)).clicked() {
                    let (cont, folga) = (c.continua.or(Some(a)), (c.folga != 0.).then(|| format!("{}", c.folga)));
                    *novo = Some(Box::new(move |d| {
                        st_cantos::definir_regra(d, a, b, x, cont, folga);
                    }));
                }
            }
        });
    });
    if matches!(c.modo, CornerMode::Aparar | CornerMode::Encaixe) {
        let (cont, para) = match c.continua {
            Some(x) if x == a => (a, b),
            _ => (b, a),
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} {}", tr("continua:"), nome_da_linha(app, cont))).color(theme::st_warn()));
            if ui.button("⇄").on_hover_text(tr("Trocar quem continua: o outro membro passa a parar nele.")).clicked() {
                let c2 = c.clone();
                *novo = Some(Box::new(move |d| {
                    st_cantos::trocar_continua(d, &c2);
                }));
                *aviso = Some(tr("Quem continua foi trocado").to_string());
            }
        });
        ui.label(RichText::new(format!("{} {}", tr("para:"), nome_da_linha(app, para))).color(theme::accent(ui.visuals())));
        let mut g = if c.explicito && c.folga != 0. { format!("{}", c.folga) } else { String::new() };
        ui.horizontal(|ui| {
            ui.label(tr("Folga de solda"));
            let campo = ui.add(egui::TextEdit::singleline(&mut g).desired_width(60.).hint_text(tr("folga"))).on_hover_text(tr("Distância entre o membro que para e a face de quem continua, em mm."));
            // B155: durante a edição o 3D mostra o valor num rótulo flutuante
            crate::st_encaixe_ui::marcar_foco_folga(ui.ctx(), campo.has_focus());
            if campo.changed() {
                let modo = c.modo;
                let cont = c.continua;
                *novo = Some(Box::new(move |d| {
                    st_cantos::definir_regra(d, a, b, Some(modo), cont, Some(g));
                }));
            }
        });
        if c.modo == CornerMode::Encaixe {
            crate::st_encaixe_ui::cartao(ui, app, c);
        }
    }
}

/// Realce no 3D: um disco em cada canto (cor da classe), o canto escolhido com anel e a prévia do corte
/// (o membro que continua em amarelo, o que para em azul). Clicar num disco escolhe o canto.
pub fn draw(ui: &egui::Ui, rect: egui::Rect, vp: &crate::render::Viewport, cantos: &[Canto], sel: &mut Option<[f64; 3]>) {
    let proj = |p: [f64; 3]| vp.project(rect, glam::Vec3::from(p.map(|v| v as f32)));
    for (i, c) in cantos.iter().enumerate() {
        let Some(pos) = proj(c.ponto) else { continue };
        let escolhido = sel.is_some_and(|p| mesmo_ponto(p, c.ponto));
        let cor = cor_da_classe(c.classe, ui.visuals());
        if escolhido && c.linhas.len() == 2 {
            if let Some(p) = c.pares.first() {
                let (cont, para) = match c.continua {
                    Some(x) if x == p.b => ((p.fim_b, p.b), (p.fim_a, p.a)),
                    _ => ((p.fim_a, p.a), (p.fim_b, p.b)),
                };
                let tem_regra_de_corte = c.continua.is_some();
                let (cor_cont, cor_para) = if tem_regra_de_corte { (theme::st_warn(), theme::accent(ui.visuals())) } else { (cor, cor) };
                for ((fim, _), cor_l) in [(cont, cor_cont), (para, cor_para)] {
                    if let Some(q) = proj(fim) {
                        ui.painter().line_segment([pos, q], egui::Stroke::new(4., cor_l));
                    }
                }
            }
        }
        let r = if escolhido { 9. } else { 7. };
        ui.painter().circle_filled(pos, r, cor);
        ui.painter().circle_stroke(pos, r, egui::Stroke::new(if escolhido { 2.5 } else { 1.5 }, ui.visuals().extreme_bg_color));
        if escolhido {
            ui.painter().circle_stroke(pos, r + 4., egui::Stroke::new(2., theme::accent(ui.visuals())));
        }
        let resp = ui.interact(egui::Rect::from_center_size(pos, egui::vec2(18., 18.)), egui::Id::new(("st_canto_pick", i)), egui::Sense::click());
        if resp.clicked() {
            *sel = Some(c.ponto);
        }
        resp.on_hover_text(format!("{} · {}", tr(nome_da_classe(c.classe)), tr(nome_do_modo(c.modo))));
    }
    // B155: prévia fantasma do corte de encaixe do canto escolhido
    crate::st_encaixe_ui::desenhar(ui, rect, vp);
}
