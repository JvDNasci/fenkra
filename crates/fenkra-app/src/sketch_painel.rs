// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel do comando de sketch (B271, aplica o B20): estado do passo, modo, parâmetros e avançado.
//!
//! Variantes de uma mesma forma são um comando com modos. As ferramentas continuam sendo valores de
//! `Tool`, porque cada modo cria geometria diferente e os botões antigos precisam levar a ele; o que
//! muda é que trocar de modo não recomeça o passo. Os campos do painel escrevem em `ValueEntry`, o
//! mesmo estado dos campos junto ao cursor, então os dois lados mostram sempre o mesmo valor.

use crate::app::{fmt, App};
use crate::i18n::tr;
use crate::sketch_ui::Tool;
use eframe::egui::{self, RichText};

/// Modos do comando a que a ferramenta pertence; ferramenta sem variantes não tem modos.
pub fn modos(t: Tool) -> &'static [Tool] {
    const FAMILIAS: [&[Tool]; 6] = [
        &[Tool::Rect, Tool::RectCenter],
        &[Tool::Circle, Tool::Circle3],
        &[Tool::Arc3, Tool::ArcCenter],
        &[Tool::Fillet, Tool::Chamfer],
        &[Tool::Move, Tool::Copy],
        &[Tool::Trim, Tool::Extend],
    ];
    FAMILIAS.into_iter().find(|f| f.contains(&t)).unwrap_or(&[])
}

/// Próximo modo do comando, dando a volta; a própria ferramenta se ela não tiver modos.
pub fn proximo_modo(t: Tool) -> Tool {
    let m = modos(t);
    m.iter().position(|x| *x == t).map_or(t, |i| m[(i + 1) % m.len()])
}

/// Nome do comando, igual em todos os modos.
pub fn comando_de(t: Tool) -> &'static str {
    match t {
        Tool::Rect | Tool::RectCenter => "Retângulo",
        Tool::Circle | Tool::Circle3 => "Círculo",
        Tool::Arc3 | Tool::ArcCenter => "Arco",
        Tool::Fillet | Tool::Chamfer => "Canto",
        Tool::Move | Tool::Copy => "Mover ou copiar",
        Tool::Trim | Tool::Extend => "Relimitar",
        _ => t.info().1,
    }
}

/// Nome do modo no botão do painel.
pub fn nome_modo(t: Tool) -> &'static str {
    match t {
        Tool::Rect => "2 cantos",
        Tool::RectCenter | Tool::ArcCenter => "Pelo centro",
        Tool::Circle => "Centro e raio",
        Tool::Circle3 | Tool::Arc3 => "3 pontos",
        _ => t.info().1,
    }
}

/// Valores que o comando pede, na ordem dos passos. Os do passo atual ficam abertos; os outros esperam.
pub fn campos_do_comando(t: Tool) -> &'static [&'static str] {
    match t {
        Tool::Line => &["comprimento", "ângulo"],
        Tool::Arc3 => &["comprimento", "ângulo", "raio"],
        Tool::Slot => &["comprimento", "ângulo", "largura"],
        Tool::Rect | Tool::RectCenter => &["largura", "altura"],
        Tool::Circle | Tool::Circle3 => &["raio"],
        Tool::ArcCenter | Tool::Polygon => &["raio", "ângulo"],
        Tool::Move | Tool::Copy => &["distância", "ângulo"],
        Tool::Offset => &["distância"],
        _ => &[],
    }
}

/// Dica de cada campo do painel, ao passar o mouse.
pub fn dica_campo(rotulo: &str) -> &'static str {
    match rotulo {
        "comprimento" => "Comprimento do trecho a partir do último ponto. Digite para travar; aceita expressão.",
        "ângulo" => "Ângulo em graus a partir da horizontal do sketch; no arco pelo centro, o ângulo varrido. Digite para travar.",
        "largura" => "Largura total. Digite para travar; o outro valor continua seguindo o mouse.",
        "altura" => "Altura total. Digite para travar; o outro valor continua seguindo o mouse.",
        "raio" => "Raio da curva. Digite para travar; aceita expressão.",
        "distância" => "Distância até o ponto ou curva de origem. Digite para travar; o lado segue o mouse.",
        _ => "Valor do comando. Digite para travar; aceita expressão.",
    }
}

const DICA_ESPERA: &str = "Este valor abre quando o passo dele chegar.";
const DICA_LADOS: &str = "Número de lados do polígono regular.";
const DICA_RAIO_CANTO: &str = "Raio do próximo filete. Vazio: o maior que cabe, limitado pelas linhas do canto.";
const DICA_DIST_CANTO: &str = "Distância do próximo chanfro. Vazio: a maior que cabe, limitada pelas linhas do canto.";
const DICA_CONSTRUCAO: &str = "A geometria nova nasce como construção: ajuda a desenhar e não entra no perfil.";
const DICA_COTA_DIGITAR: &str = "O valor digitado no campo do cursor vira cota da curva, guardando a expressão. Desligado, ele só posiciona a geometria.";
const DICA_AUTO_REST: &str = "Linhas quase horizontais ou verticais e os apoios dos snaps viram restrições ao criar.";
const DICA_AUTO_COTA: &str = "A forma nasce com as cotas principais.";
const DICA_SNAPS: &str = "O cursor procura pontos, curvas e alinhamentos antes da grade.";

/// A ferramenta desenha geometria nova.
fn desenha(t: Tool) -> bool {
    matches!(t, Tool::Line | Tool::Rect | Tool::RectCenter | Tool::Circle | Tool::Circle3 | Tool::Arc3 | Tool::ArcCenter | Tool::Slot | Tool::Polygon | Tool::Point | Tool::Spline)
}

/// Seções do comando, na ordem: estado do passo, modo, parâmetros e avançado.
pub fn comando(app: &mut App, ui: &mut egui::Ui) {
    let vars = app.vars();
    let tecla = app.settings.keys.binding(crate::keymap::KeyAction::SkModo).map(|b| b.text());
    let Some(edit) = app.sketch.as_mut() else { return };
    let tool = edit.tool;
    let destaque = crate::theme::accent(ui.visuals());

    ui.label(RichText::new(tr(comando_de(tool))).strong());
    ui.label(RichText::new(edit.dica_passo()).color(destaque));
    ui.label(RichText::new(tr(tool.info().3)).weak().small());

    // ---- modo
    if !modos(tool).is_empty() {
        ui.add_space(2.);
        ui.horizontal_wrapped(|ui| {
            for m in modos(tool).iter().copied() {
                let img = egui::Image::new(crate::icons::get(m.info().0)).fit_to_exact_size(egui::vec2(18., 18.));
                let mut dica = tr(m.info().2).to_string();
                if let Some(k) = &tecla {
                    dica = format!("{dica}\n{k}: {}", tr("percorre os modos"));
                }
                if ui.add(egui::Button::image_and_text(img, tr(nome_modo(m))).selected(m == tool)).on_hover_text(dica).clicked() {
                    edit.escolher(m);
                }
            }
        });
    }

    // ---- parâmetros: os mesmos valores dos campos da viewport
    let rotulos = campos_do_comando(tool);
    if !rotulos.is_empty() || matches!(tool, Tool::Polygon | Tool::Fillet | Tool::Chamfer) {
        ui.add_space(2.);
        ui.label(RichText::new(tr("Parâmetros")).weak().small());
    }
    let mut foco_saiu = false;
    egui::Grid::new("sk_parametros").num_columns(2).show(ui, |ui| {
        for rotulo in rotulos {
            ui.label(tr(rotulo)).on_hover_text(tr(dica_campo(rotulo)));
            match edit.campos.iter().position(|c| c.label == *rotulo) {
                Some(i) => {
                    let c = &edit.campos[i];
                    let medida = format!("{}{}", fmt(c.value), if c.angle { "°" } else { "" });
                    let mut texto = edit.entry.texto_do_campo(i);
                    let r = ui.add(egui::TextEdit::singleline(&mut texto).hint_text(medida).desired_width(110.)).on_hover_text(tr(dica_campo(rotulo)));
                    if r.changed() {
                        edit.entry.escrever(i, &texto, &vars);
                    }
                    if r.lost_focus() {
                        foco_saiu = true;
                        edit.confirmar_campo = ui.input(|x| x.key_pressed(egui::Key::Enter));
                    }
                }
                None => {
                    let mut vazio = String::new();
                    ui.add_enabled(false, egui::TextEdit::singleline(&mut vazio).hint_text("—").desired_width(110.)).on_disabled_hover_text(format!("{} {}", tr(dica_campo(rotulo)), tr(DICA_ESPERA)));
                }
            }
            ui.end_row();
        }
        if tool == Tool::Polygon {
            ui.label(tr("lados")).on_hover_text(tr(DICA_LADOS));
            ui.add(crate::ui_ext::slider(&mut edit.polygon_sides, 3..=24)).on_hover_text(tr(DICA_LADOS));
            ui.end_row();
        }
        if matches!(tool, Tool::Fillet | Tool::Chamfer) {
            let chanfro = tool == Tool::Chamfer;
            let (nome, dica) = if chanfro { ("distância", DICA_DIST_CANTO) } else { ("raio", DICA_RAIO_CANTO) };
            ui.label(tr(nome)).on_hover_text(tr(dica));
            let id = ui.make_persistent_id(("sk_canto", chanfro));
            if !ui.memory(|m| m.has_focus(id)) {
                edit.texto_canto = edit.valor_canto(chanfro).map(fmt).unwrap_or_default();
            }
            let r = ui.add(egui::TextEdit::singleline(&mut edit.texto_canto).id(id).hint_text(tr("automático")).desired_width(110.)).on_hover_text(tr(dica));
            if r.changed() {
                let t = edit.texto_canto.trim().to_string();
                if t.is_empty() {
                    edit.definir_canto(chanfro, None);
                } else if let Ok(v) = fenkra_core::expr::eval_str(&t, &vars) {
                    edit.definir_canto(chanfro, Some(v).filter(|v| *v > 0.));
                }
            }
            ui.end_row();
        }
    });
    if foco_saiu {
        edit.entry.encerrar_digitacao(&vars);
    }

    // ---- avançado
    if desenha(tool) || tool == Tool::Offset {
        egui::CollapsingHeader::new(tr("Avançado")).id_salt("sk_avancado").default_open(false).show(ui, |ui| {
            ui.checkbox(&mut edit.construction, tr("construção")).on_hover_text(tr(DICA_CONSTRUCAO));
            if desenha(tool) {
                let s = &mut app.settings;
                ui.checkbox(&mut s.dim_on_type, tr("Criar cota ao digitar")).on_hover_text(tr(DICA_COTA_DIGITAR));
                ui.checkbox(&mut s.auto_constraints, tr("Restrições automáticas")).on_hover_text(tr(DICA_AUTO_REST));
                ui.checkbox(&mut s.auto_dims, tr("Cotas automáticas")).on_hover_text(tr(DICA_AUTO_COTA));
                ui.checkbox(&mut s.snaps, tr("Snaps")).on_hover_text(tr(DICA_SNAPS));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::tem_traducao;
    use crate::sketch_ui::{measures_labels, SketchEdit, ValueEntry};
    use crate::toolbar::{all_entries, Act, Entry};
    use fenkra_core::Id;

    #[test]
    fn os_quatro_comandos_do_pedido_tem_modos_e_a_volta_fecha() {
        for par in [[Tool::Rect, Tool::RectCenter], [Tool::Circle, Tool::Circle3], [Tool::Arc3, Tool::ArcCenter], [Tool::Fillet, Tool::Chamfer]] {
            assert_eq!(modos(par[0]), modos(par[1]));
            assert_eq!(proximo_modo(par[0]), par[1]);
            assert_eq!(proximo_modo(par[1]), par[0]);
            assert_eq!(comando_de(par[0]), comando_de(par[1]));
        }
        assert_eq!(proximo_modo(Tool::Line), Tool::Line);
    }

    #[test]
    fn trocar_de_modo_continua_o_passo() {
        let mut e = SketchEdit::new(Id(1), false);
        e.set_tool(Tool::Circle);
        e.clique_de_teste([1., 2.]);
        e.percorrer_modo();
        assert_eq!(e.tool, Tool::Circle3);
        assert_eq!(e.indice_passo(), 1, "o primeiro ponto do círculo vale no modo de 3 pontos");
        // o modo seguinte ficaria completo com os cliques dados: recomeça
        e.clique_de_teste([3., 2.]);
        e.percorrer_modo();
        assert_eq!(e.tool, Tool::Circle);
        assert_eq!(e.indice_passo(), 0);
    }

    /// Os botões antigos da barra levam ao modo certo, estando em qualquer modo do comando.
    #[test]
    fn botoes_antigos_levam_ao_modo_certo() {
        let mut vistos = 0;
        for e in all_entries() {
            let itens = match e {
                Entry::One(i) => vec![i],
                Entry::Family(_, v) => v,
            };
            for i in itens {
                let Act::SkTool(alvo) = i.act else { continue };
                for origem in std::iter::once(Tool::Select).chain(modos(alvo).iter().copied()) {
                    let mut ed = SketchEdit::new(Id(1), false);
                    ed.set_tool(origem);
                    ed.escolher(alvo);
                    assert_eq!(ed.tool, alvo, "{origem:?} -> {alvo:?}");
                }
                vistos += 1;
            }
        }
        assert!(vistos > 30, "a barra do sketch sumiu da lista: {vistos}");
    }

    /// Todo valor que a viewport pede aparece no painel, com dica própria e traduzida.
    #[test]
    fn todo_campo_tem_dica_e_esta_no_painel() {
        for t in crate::sketch_fluxo::todas() {
            for n in 1..=2 {
                for rotulo in measures_labels(t, n) {
                    assert!(campos_do_comando(t).contains(&rotulo), "{t:?} passo {n}: {rotulo} fora do painel");
                }
            }
            for rotulo in campos_do_comando(t) {
                let d = dica_campo(rotulo);
                assert_ne!(d, dica_campo("?"), "{rotulo} sem dica própria");
                assert!(tem_traducao(d) && tem_traducao(rotulo), "{rotulo} sem tradução");
            }
        }
        for d in [DICA_ESPERA, DICA_LADOS, DICA_RAIO_CANTO, DICA_DIST_CANTO, DICA_CONSTRUCAO, DICA_COTA_DIGITAR, DICA_AUTO_REST, DICA_AUTO_COTA, DICA_SNAPS] {
            assert!(tem_traducao(d), "{d}");
        }
        for m in [Tool::Rect, Tool::RectCenter, Tool::Circle, Tool::Circle3, Tool::Arc3, Tool::ArcCenter, Tool::Fillet, Tool::Chamfer] {
            assert!(tem_traducao(nome_modo(m)) && tem_traducao(comando_de(m)), "{m:?}");
        }
    }

    /// Painel e viewport são o mesmo estado: o que o painel escreve trava o campo do cursor.
    #[test]
    fn campo_do_painel_trava_o_campo_da_viewport() {
        let vars = Default::default();
        let mut e = ValueEntry::default();
        e.escrever(1, "2*", &vars);
        assert_eq!(e.valor_de_teste(2, &vars), [None, None], "expressão pela metade não trava");
        e.escrever(1, "2*15", &vars);
        assert_eq!(e.valor_de_teste(2, &vars), [None, Some(30.)]);
        e.encerrar_digitacao(&vars);
        assert_eq!(e.texto_do_campo(1), "2*15");
        e.escrever(0, "40", &vars);
        assert_eq!(e.valor_de_teste(2, &vars), [Some(40.), Some(30.)], "trocar de campo mantém o anterior travado");
        e.escrever(1, "", &vars);
        assert_eq!(e.valor_de_teste(2, &vars), [Some(40.), None], "apagar destrava");
    }
}
