// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenho da roda de comandos e o editor em Configurações (B256). Só cores do tema.

use super::{angulo_do_setor, item, Config, CONTEXTOS, RAIO_EXTERNO, RAIO_MORTO, SETORES};
use crate::i18n::tr;
use eframe::egui::{self, Color32, Pos2, RichText, Shape, Stroke};

/// Pontos de um setor anelar entre dois raios, com folga de 4 graus de cada lado.
fn setor_anel(c: Pos2, i: usize, r0: f32, r1: f32) -> Vec<Pos2> {
    let meio = angulo_do_setor(i);
    let (a0, a1) = (meio - 22.5f32.to_radians() + 4f32.to_radians(), meio + 22.5f32.to_radians() - 4f32.to_radians());
    let ponto = |a: f32, r: f32| c + egui::vec2(a.sin() * r, -a.cos() * r);
    let n = 8;
    let mut v: Vec<Pos2> = (0..=n).map(|k| ponto(a0 + (a1 - a0) * k as f32 / n as f32, r1)).collect();
    v.extend((0..=n).rev().map(|k| ponto(a0 + (a1 - a0) * k as f32 / n as f32, r0)));
    v
}

/// Desenha a roda sobre a tela inteira. `alvo` é o setor sob o ponteiro.
pub fn desenhar(ctx: &egui::Context, centro: Pos2, icones: &[String], pagina: usize, paginas: usize, alvo: Option<usize>) {
    egui::Area::new(egui::Id::new("roda_de_comandos")).order(egui::Order::Foreground).fixed_pos(Pos2::ZERO).interactable(false).show(ctx, |ui| {
        let v = ui.visuals().clone();
        let acento = crate::theme::accent(&v);
        let tela = ctx.content_rect();
        // a roda não sai da janela: o centro é empurrado para dentro
        let m = RAIO_EXTERNO * 1.12 + 8.;
        let c = Pos2::new(centro.x.clamp(tela.left() + m, (tela.right() - m).max(tela.left() + m)), centro.y.clamp(tela.top() + m, (tela.bottom() - m).max(tela.top() + m)));
        let p = ui.painter();
        p.circle_filled(c, RAIO_MORTO, v.window_fill.gamma_multiply(0.92));
        p.circle_stroke(c, RAIO_MORTO, Stroke::new(1., v.widgets.noninteractive.bg_stroke.color));
        for i in 0..SETORES {
            let ativo = alvo == Some(i);
            let ocupado = icones.get(i).is_some();
            let r1 = if ativo { RAIO_EXTERNO * 1.12 } else { RAIO_EXTERNO };
            let fundo = if ativo && ocupado { acento.gamma_multiply(0.85) } else { v.window_fill.gamma_multiply(if ocupado { 0.92 } else { 0.5 }) };
            p.add(Shape::convex_polygon(setor_anel(c, i, RAIO_MORTO + 4., r1), fundo, Stroke::new(1., v.widgets.noninteractive.bg_stroke.color)));
            let Some(ic) = icones.get(i) else { continue };
            let meio = angulo_do_setor(i);
            let r = (RAIO_MORTO + RAIO_EXTERNO) * 0.5;
            let pos = c + egui::vec2(meio.sin() * r, -meio.cos() * r);
            let texto_cor = if ativo { crate::theme::readable(acento, &v) } else { v.text_color() };
            egui::Image::new(crate::icons::get(ic)).paint_at(ui, egui::Rect::from_center_size(pos - egui::vec2(0., 8.), egui::vec2(34., 34.)));
            if let Some(it) = item(ic) {
                p.text(pos + egui::vec2(0., 20.), egui::Align2::CENTER_TOP, tr(it.name), egui::FontId::proportional(11.), texto_cor);
            }
            p.text(pos + egui::vec2(-26., -34.), egui::Align2::CENTER_CENTER, (i + 1).to_string(), egui::FontId::proportional(10.), texto_cor.gamma_multiply(0.7));
        }
        // centro: nome do comando apontado e a página
        let nome = alvo.and_then(|i| icones.get(i)).and_then(|ic| item(ic)).map(|it| tr(it.name).to_owned()).unwrap_or_default();
        p.text(c, egui::Align2::CENTER_CENTER, nome, egui::FontId::proportional(11.), v.text_color());
        if paginas > 1 {
            let pontos: Vec<Pos2> = (0..paginas).map(|k| c + egui::vec2((k as f32 - (paginas as f32 - 1.) / 2.) * 9., RAIO_MORTO - 9.)).collect();
            for (k, q) in pontos.iter().enumerate() {
                p.circle_filled(*q, 2.5, if k == pagina { acento } else { v.widgets.noninteractive.bg_stroke.color });
            }
        }
    });
}

// ------------------------------------------------------------------------- editor

fn chave_ctx() -> egui::Id {
    egui::Id::new("roda_editor_contexto")
}

/// Pedido para abrir o editor (o botão em Atalhos só levanta a bandeira).
pub fn pedir_abrir(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("roda_editor_aberto"), true));
}

fn lista(cfg: &Config, chave: &str) -> Vec<String> {
    cfg.slots.iter().find(|(c, _)| c == chave).map(|(_, v)| v.clone()).unwrap_or_else(|| super::padrao(chave).iter().map(|s| (*s).to_owned()).collect())
}

/// Mover o item `de` para antes da posição `para` (índices da lista antes da remoção).
pub fn mover(v: &mut Vec<String>, de: usize, para: usize) {
    if de >= v.len() {
        return;
    }
    let x = v.remove(de);
    let alvo = if para > de { para - 1 } else { para };
    v.insert(alvo.min(v.len()), x);
}

/// Põe um comando do catálogo antes de `pos` (move se já estiver na lista).
pub fn inserir(v: &mut Vec<String>, icone: &str, pos: usize) {
    if let Some(i) = v.iter().position(|x| x == icone) {
        mover(v, i, pos);
    } else {
        v.insert(pos.min(v.len()), icone.to_owned());
    }
}

/// Janela do editor: contexto, setores em ordem (arrastar para reordenar), catálogo para arrastar,
/// restaurar, exportar e importar.
pub fn editor(ctx: &egui::Context, cfg: &mut Config) {
    let aberto_id = egui::Id::new("roda_editor_aberto");
    if !ctx.data(|d| d.get_temp::<bool>(aberto_id)).unwrap_or(false) {
        return;
    }
    let mut aberto = true;
    egui::Window::new(tr("Roda de comandos")).id(egui::Id::new("janela_roda_editor")).open(&mut aberto).default_size([620., 420.]).show(ctx, |ui| {
        let mut sel: usize = ui.ctx().data(|d| d.get_temp(chave_ctx())).unwrap_or(0);
        sel = sel.min(CONTEXTOS.len() - 1);
        ui.horizontal(|ui| {
            ui.label(tr("Contexto:"));
            egui::ComboBox::from_id_salt("roda_ctx").selected_text(tr(CONTEXTOS[sel].1)).show_ui(ui, |ui| {
                for (i, (_, r)) in CONTEXTOS.iter().enumerate() {
                    ui.selectable_value(&mut sel, i, tr(r));
                }
            });
        });
        ui.ctx().data_mut(|d| d.insert_temp(chave_ctx(), sel));
        let chave = CONTEXTOS[sel].0;
        let mut atual = lista(cfg, chave);
        let original = atual.clone();
        ui.label(RichText::new(tr("Arraste um comando do catálogo para a lista. Os setores 1 a 8 formam a primeira página; os outros viram páginas, que a roda do mouse vira.")).weak().small());
        ui.separator();
        ui.columns(2, |cols| {
            // lista da roda
            cols[0].label(RichText::new(tr("Na roda")).strong());
            let mut remover: Option<usize> = None;
            let mut soltou: Option<(usize, String)> = None;
            egui::ScrollArea::vertical().id_salt("roda_lista").max_height(300.).show(&mut cols[0], |ui| {
                for (i, ic) in atual.iter().enumerate() {
                    if i > 0 && i % SETORES == 0 {
                        ui.label(RichText::new(format!("— {} {} —", tr("página"), i / SETORES + 1)).weak().small());
                    }
                    let (resposta, carga) = ui.dnd_drop_zone::<(usize, String), _>(egui::Frame::group(ui.style()), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i % SETORES + 1));
                            ui.dnd_drag_source(egui::Id::new(("roda_item", i)), (i, ic.clone()), |ui| {
                                ui.add(egui::Image::new(crate::icons::get(ic)).fit_to_exact_size(egui::vec2(20., 20.)));
                                ui.label(item(ic).map(|it| tr(it.name)).unwrap_or(ic.as_str()));
                            });
                            if ui.small_button("✕").on_hover_text(tr("Tirar da roda")).clicked() {
                                remover = Some(i);
                            }
                        });
                    });
                    let _ = resposta;
                    if let Some(c) = carga {
                        soltou = Some((i, c.1.clone()));
                    }
                }
                // área final para acrescentar
                let (_, fim) = ui.dnd_drop_zone::<(usize, String), _>(egui::Frame::group(ui.style()), |ui| {
                    ui.label(RichText::new(tr("Solte aqui para acrescentar")).weak());
                });
                if let Some(c) = fim {
                    soltou = Some((usize::MAX, c.1.clone()));
                }
            });
            if let Some(i) = remover {
                atual.remove(i);
            }
            if let Some((pos, ic)) = soltou {
                inserir(&mut atual, &ic, pos.min(atual.len()));
            }
            // catálogo
            cols[1].label(RichText::new(tr("Catálogo de comandos")).strong());
            egui::ScrollArea::vertical().id_salt("roda_catalogo").max_height(300.).show(&mut cols[1], |ui| {
                for it in crate::ajuda_cmd::comandos() {
                    ui.dnd_drag_source(egui::Id::new(("roda_cat", it.icon)), (usize::MAX, it.icon.to_owned()), |ui| {
                        ui.horizontal(|ui| {
                            ui.add(egui::Image::new(crate::icons::get(it.icon)).fit_to_exact_size(egui::vec2(18., 18.)));
                            ui.label(tr(it.name));
                        });
                    });
                }
            });
        });
        if atual != original {
            if atual.is_empty() {
                cfg.restaurar(chave);
            } else {
                cfg.definir(chave, atual);
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(tr("Restaurar este contexto")).clicked() {
                cfg.restaurar(chave);
            }
            if ui.button(tr("Exportar…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().set_file_name("roda_de_comandos.json").add_filter("JSON", &["json"]).save_file() {
                    let _ = std::fs::write(p, cfg.exportar());
                }
            }
            if ui.button(tr("Importar…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    let validos: Vec<&str> = crate::ajuda_cmd::comandos().iter().map(|i| i.icon).collect();
                    let r = std::fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|t| cfg.importar(&t, &validos));
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("roda_import_msg"), r.map(|n| format!("{n}")).unwrap_or_else(|e| e)));
                }
            }
            if let Some(m) = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::new("roda_import_msg"))) {
                let ok = m.parse::<usize>().is_ok();
                ui.label(RichText::new(if ok { format!("{m} {}", tr("contextos importados")) } else { m }).color(if ok { crate::theme::st_ok() } else { crate::theme::st_err() }).small());
            }
        });
        let _ = Color32::TRANSPARENT;
    });
    if !aberto {
        ctx.data_mut(|d| d.insert_temp(aberto_id, false));
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn mover_e_inserir_reordenam_sem_duplicar() {
        let mut a = v(&["a", "b", "c", "d"]);
        mover(&mut a, 0, 3); // a vai para antes de d
        assert_eq!(a, v(&["b", "c", "a", "d"]));
        mover(&mut a, 3, 0);
        assert_eq!(a, v(&["d", "b", "c", "a"]));
        inserir(&mut a, "x", 1);
        assert_eq!(a, v(&["d", "x", "b", "c", "a"]));
        inserir(&mut a, "a", 0); // já existe: move
        assert_eq!(a, v(&["a", "d", "x", "b", "c"]));
        inserir(&mut a, "z", usize::MAX);
        assert_eq!(a.last().map(String::as_str), Some("z"));
        assert_eq!(a.len(), 6);
    }

    #[test]
    fn setor_anel_cabe_entre_os_raios_e_nao_toca_o_vizinho() {
        let c = Pos2::new(100., 100.);
        for i in 0..SETORES {
            for p in setor_anel(c, i, RAIO_MORTO + 1., RAIO_EXTERNO) {
                let d = (p - c).length();
                assert!(d >= RAIO_MORTO + 0.99 && d <= RAIO_EXTERNO + 0.01);
                // o ponto do setor i aponta para o setor i (a folga mantém longe do limite)
                assert_eq!(super::super::setor_do_deslocamento(p.x - c.x, p.y - c.y), Some(i));
            }
        }
    }
}
