// SPDX-License-Identifier: LGPL-2.1-or-later
//! Editor do quadro de tolerância geométrica no próprio 3D (B108/B188): cada casa do quadro é um
//! campo com lista (característica, zona, valor, modificadores, referências), a validação aparece
//! na hora em mensagem curta e o quadro nasce válido sem abrir o painel lateral.
//!
//! As regras vêm de `fenkra_core::gdt`; aqui só a interface. Cores só pelo tema.

use crate::i18n::tr;
use eframe::egui::{self, Color32, Rect, RichText, Stroke};
use fenkra_core::gdt::*;
use fenkra_core::pmi::{GdtChar, GdtExtra, Pmi};
use std::sync::Mutex;

/// Letras de referência que o modelo define, atualizadas a cada quadro de interface por quem
/// abre o editor (a lista vem da árvore de PMI do nível aberto).
static EXISTENTES: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn atualizar_existentes(pmi: &[Pmi]) {
    *EXISTENTES.lock().unwrap_or_else(|e| e.into_inner()) = datums_do_modelo(pmi);
}

pub fn existentes() -> Vec<String> {
    EXISTENTES.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Próxima letra livre de referência, dado o que o nível aberto já define.
pub fn proxima(pmi: &[Pmi]) -> String {
    proxima_letra(&datums_do_modelo(pmi))
}

/// Pinta traços vetoriais num quadrado de `lado` px no retângulo `r`.
pub fn pintar(ui: &egui::Ui, tracos: &[Traco], r: Rect, col: Color32) {
    let lado = r.width().min(r.height());
    let st = Stroke::new((lado * 0.09).max(1.), col);
    let p = ui.painter();
    for t in tracos {
        let pts: Vec<egui::Pos2> = t.pts.iter().map(|q| egui::pos2(r.center().x + q[0] as f32 * lado, r.center().y - q[1] as f32 * lado)).collect();
        if t.cheio {
            p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
        } else if t.fechado {
            p.add(egui::Shape::closed_line(pts, st));
        } else {
            p.add(egui::Shape::line(pts, st));
        }
    }
}

/// Ícone pequeno de uma característica.
fn icone_char(ui: &mut egui::Ui, c: GdtChar) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(16., 16.), egui::Sense::hover());
    pintar(ui, &c.tracado(), r.shrink(1.), ui.visuals().text_color());
}

/// Mensagens do consultor, curtas e coloridas pelo tema; no máximo `max`.
pub fn mensagens(ui: &mut egui::Ui, avisos: &[Aviso], max: usize) {
    let mut ord: Vec<&Aviso> = avisos.iter().collect();
    ord.sort_by(|a, b| b.gravidade.cmp(&a.gravidade));
    for a in ord.iter().take(max) {
        let (cor, marca) = match a.gravidade {
            Gravidade::Erro => (crate::theme::st_err(), "✖"),
            Gravidade::Aviso => (crate::theme::st_warn(), "⚠"),
            Gravidade::Info => (ui.visuals().weak_text_color(), "ℹ"),
        };
        ui.label(RichText::new(format!("{marca} {}", tr(&a.msg))).color(cor).small());
    }
    if ord.len() > max {
        ui.label(RichText::new(format!("+{}", ord.len() - max)).weak().small());
    }
}

/// Compat de um modificador no quadro atual, para habilitar ou não a opção.
fn compat_aqui(m: Modif, ch: GdtChar, x: &GdtExtra, pos: PosMod) -> Compat {
    compat(m, ch, x.elemento, pos, x.norma)
}

/// Menu de modificadores do valor: marca os ativos e desabilita (com o motivo) os que a norma não
/// admite para esta característica.
fn menu_modificadores(ui: &mut egui::Ui, ch: GdtChar, x: &mut GdtExtra, mudou: &mut bool) {
    let n = x.resolvido().mods.len();
    let rotulo = if n == 0 { tr("Modif.").to_string() } else { format!("{} ({n})", tr("Modif.")) };
    ui.menu_button(rotulo, |ui| {
        let ativos = x.resolvido().mods;
        for m in Modif::ALL {
            let c = compat_aqui(m, ch, x, m.posicao());
            let mut on = ativos.contains(&m);
            let texto = format!("{} {}", m.texto(), tr(m.label()));
            let r = ui.add_enabled(c.admite() || on, egui::Checkbox::new(&mut on, texto));
            let r = match c {
                Compat::Nao(msg) | Compat::Aviso(msg) => r.on_hover_text(tr(msg)),
                Compat::Ok => r,
            };
            if r.changed() {
                x.migrar();
                if on {
                    x.mods.push(m);
                } else {
                    x.mods.retain(|y| *y != m);
                }
                *mudou = true;
            }
        }
    });
}

/// Editor compacto: uma linha de casas clicáveis mais as mensagens. Devolve `true` se algo mudou.
/// `avancado` mostra também norma, elemento, quadro composto e os extras (por unidade, máximo).
pub fn editor(ui: &mut egui::Ui, salt: &str, ch: &mut GdtChar, tol: &mut String, datums: &mut String, x: &mut GdtExtra, avancado: bool) -> bool {
    let mut mudou = false;
    // o desenho (folha) não tem as referências do modelo 3D
    let ex = if salt.starts_with("dwg") { vec![] } else { existentes() };
    ui.horizontal_wrapped(|ui| {
        // característica
        let antes = *ch;
        ui.horizontal(|ui| {
            icone_char(ui, *ch);
            egui::ComboBox::from_id_salt((salt, "ch")).width(120.).selected_text(tr(ch.label())).show_ui(ui, |ui| {
                for c in GdtChar::ALL {
                    ui.horizontal(|ui| {
                        icone_char(ui, c);
                        ui.selectable_value(ch, c, tr(c.label()));
                    });
                }
            });
        })
        .response
        .on_hover_text(tr("Característica geométrica controlada"));
        if *ch != antes {
            ao_trocar_caracteristica(*ch, datums, &ex);
            mudou = true;
        }
        // zona
        let z = x.resolvido().zona;
        let mut nz = z;
        egui::ComboBox::from_id_salt((salt, "zona")).width(86.).selected_text(if z.prefixo().is_empty() { tr(z.label()).to_string() } else { z.prefixo().to_string() }).show_ui(ui, |ui| {
            for o in Zona::ALL {
                ui.selectable_value(&mut nz, o, tr(o.label()));
            }
        })
        .response
        .on_hover_text(tr("Forma da zona de tolerância: largura, cilindro Ø, esfera SØ ou não uniforme"));
        if nz != z {
            x.migrar();
            x.zona = Some(nz);
            mudou = true;
        }
        // valor
        mudou |= ui.add(egui::TextEdit::singleline(tol).desired_width(54.).hint_text("0,05")).on_hover_text(tr("Valor da tolerância, no número do modelo")).changed();
        menu_modificadores(ui, *ch, x, &mut mudou);
        // campos extras dos modificadores ativos
        let r = x.resolvido();
        if r.mods.contains(&Modif::Projetada) {
            mudou |= ui.add(egui::TextEdit::singleline(&mut x.proj).desired_width(40.).hint_text(tr("altura"))).on_hover_text(tr("Altura da zona projetada")).changed();
        }
        if r.zona == Zona::NaoUniforme {
            mudou |= ui.add(egui::TextEdit::singleline(&mut x.uz).desired_width(40.).hint_text("UZ")).on_hover_text(tr("Deslocamento da zona não uniforme")).changed();
        }
        if r.mods.contains(&Modif::Entre) {
            mudou |= ui.add(egui::TextEdit::singleline(&mut x.entre.0).desired_width(24.)).changed();
            ui.label("↔");
            mudou |= ui.add(egui::TextEdit::singleline(&mut x.entre.1).desired_width(24.)).on_hover_text(tr("Letras dos dois pontos da região")).changed();
        }
        // referências: cada uma é uma casa com lista das que o modelo define
        let mut ds = separar_datums(datums);
        let mut remover = None;
        for (i, d) in ds.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let e = ui.add(egui::TextEdit::singleline(d).desired_width(22.));
                if e.changed() {
                    *d = d.trim().to_uppercase();
                    mudou = true;
                }
                ui.menu_button("▾", |ui| {
                    for l in &ex {
                        if ui.button(l).clicked() {
                            *d = l.clone();
                            mudou = true;
                            ui.close();
                        }
                    }
                    if ex.is_empty() {
                        ui.label(RichText::new(tr("Nenhuma referência no modelo")).weak().small());
                    }
                })
                .response
                .on_hover_text(tr("Referências definidas no modelo"));
                let ms: Vec<Modif> = x.datum_mods.get(i).cloned().unwrap_or_default();
                for m in [Modif::Mmc, Modif::Lmc] {
                    let on = ms.contains(&m);
                    let c = compat_aqui(m, *ch, x, PosMod::Datum);
                    let b = ui.add_enabled(c.admite() || on, egui::Button::selectable(on, m.texto()));
                    let b = match c {
                        Compat::Nao(msg) | Compat::Aviso(msg) => b.on_hover_text(tr(msg)),
                        Compat::Ok => b.on_hover_text(tr(m.label())),
                    };
                    if b.clicked() {
                        while x.datum_mods.len() <= i {
                            x.datum_mods.push(vec![]);
                        }
                        if on {
                            x.datum_mods[i].retain(|y| *y != m);
                        } else {
                            x.datum_mods[i].push(m);
                        }
                        mudou = true;
                    }
                }
                if ui.small_button("✕").on_hover_text(tr("Remover a referência")).clicked() {
                    remover = Some(i);
                }
            });
        }
        if let Some(i) = remover {
            ds.remove(i);
            if i < x.datum_mods.len() {
                x.datum_mods.remove(i);
            }
            mudou = true;
        }
        if ds.len() < 3 && ui.small_button("+ ref").on_hover_text(tr("Acrescenta uma casa de referência (a primeira ainda não usada neste quadro)")).clicked() {
            let nova = ex.iter().find(|l| !ds.contains(l)).cloned().unwrap_or_else(|| proxima_letra(&ds));
            ds.push(nova);
            mudou = true;
        }
        if mudou {
            *datums = ds.join(" ");
        }
    });
    if avancado {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Norma"));
            egui::ComboBox::from_id_salt((salt, "norma")).width(110.).selected_text(x.norma.label()).show_ui(ui, |ui| {
                for n in Norma::ALL {
                    mudou |= ui.selectable_value(&mut x.norma, n, n.label()).changed();
                }
            });
            ui.label(tr("Aplica-se a"));
            egui::ComboBox::from_id_salt((salt, "elem")).width(150.).selected_text(tr(x.elemento.label())).show_ui(ui, |ui| {
                for e in [Elemento::Superficie, Elemento::Mediano] {
                    mudou |= ui.selectable_value(&mut x.elemento, e, tr(e.label())).changed();
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Por unidade"));
            mudou |= ui.add(egui::TextEdit::singleline(&mut x.por_unidade).desired_width(46.).hint_text("100")).on_hover_text(tr("Comprimento ou lado da região em que o valor se aplica (valor/comprimento)")).changed();
            ui.label(tr("Máximo"));
            mudou |= ui.add(egui::TextEdit::singleline(&mut x.max_valor).desired_width(46.)).on_hover_text(tr("Valor máximo permitido para a tolerância por unidade")).changed();
        });
        let mut comp = x.second.is_some();
        ui.horizontal_wrapped(|ui| {
            if crate::editor::check(ui, &mut comp, "linha de baixo (quadro composto ou de segmento único)").changed() {
                x.second = comp.then(|| (String::new(), String::new()));
                x.second_ch = None;
                mudou = true;
            }
            if comp {
                let mut unico = x.second_ch.is_some();
                if crate::editor::check(ui, &mut unico, "segmento único (símbolo próprio)").changed() {
                    x.second_ch = unico.then_some(*ch);
                    mudou = true;
                }
                if let Some(c2) = &mut x.second_ch {
                    egui::ComboBox::from_id_salt((salt, "ch2")).width(120.).selected_text(tr(c2.label())).show_ui(ui, |ui| {
                        for c in GdtChar::ALL {
                            mudou |= ui.selectable_value(c2, c, tr(c.label())).changed();
                        }
                    });
                }
            }
        });
        if let Some((t2, d2)) = &mut x.second {
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância 2"));
                mudou |= ui.add(egui::TextEdit::singleline(t2).desired_width(60.)).changed();
                ui.label(tr("Referências 2"));
                mudou |= ui.add(egui::TextEdit::singleline(d2).desired_width(60.)).changed();
            });
        }
    }
    let av = x.validar(*ch, tol, datums, if ex.is_empty() { None } else { Some(&ex) });
    mensagens(ui, &av, if avancado { 6 } else { 2 });
    mudou
}

/// Letras das outras referências do nível (todas menos a anotação `id`).
pub fn letras_de_outras(pmi: &[Pmi], id: fenkra_core::Id) -> Vec<String> {
    pmi.iter().filter(|p| p.id != id).filter_map(|p| if let fenkra_core::pmi::PmiKind::Datum { letter } = &p.kind { Some(letter.trim().to_string()) } else { None }).collect()
}

/// Campo da letra de uma referência: recusa na hora e explica; devolve `true` se mudou.
pub fn campo_letra(ui: &mut egui::Ui, letra: &mut String, outras: &[String]) -> bool {
    let r = ui.add(egui::TextEdit::singleline(letra).desired_width(48.));
    let mudou = r.changed();
    if mudou {
        *letra = letra.trim().to_uppercase();
    }
    let mut av = vec![];
    if let Err(m) = letra_valida(letra) {
        av.push(Aviso { gravidade: Gravidade::Erro, msg: m.to_string() });
    } else if outras.iter().any(|u| u == letra.trim()) {
        av.push(Aviso { gravidade: Gravidade::Erro, msg: "essa letra já é de outra referência".to_string() });
    }
    mensagens(ui, &av, 2);
    mudou
}

/// Janelinha-resumo do consultor para o painel: achados do modelo todo.
pub fn painel_consultor(ui: &mut egui::Ui, pmi: &[Pmi]) {
    let chave = egui::Id::new("pmi_consultor_norma_estado");
    let mut norma: Norma = ui.data_mut(|d| *d.get_temp_mut_or_default::<Norma>(chave));
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Consultor de normas")).strong());
        egui::ComboBox::from_id_salt("pmi_consultor_norma").width(110.).selected_text(norma.label()).show_ui(ui, |ui| {
            for n in Norma::ALL {
                ui.selectable_value(&mut norma, n, n.label());
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(chave, norma));
    let achados = consultar(pmi, norma);
    if achados.is_empty() {
        ui.label(RichText::new(format!("✔ {}", tr("Nenhum problema nas referências e quadros"))).color(crate::theme::st_ok()).small());
        return;
    }
    for a in achados.iter().take(12) {
        let nome = pmi.iter().find(|p| p.id == a.id).map(|p| p.name.clone()).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(nome).small().strong());
            mensagens(ui, std::slice::from_ref(&a.aviso), 1);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxima_letra_do_modelo_pula_i_o_q() {
        let mk = |l: &str| Pmi {
            id: fenkra_core::Id(1),
            name: String::new(),
            kind: fenkra_core::pmi::PmiKind::Datum { letter: l.into() },
            anchors: vec![],
            offset: [0.; 3],
            text: String::new(),
            tol_plus: String::new(),
            tol_minus: String::new(),
            visible: true,
            plane: None,
            style: Default::default(),
            folder: String::new(),
            targets: vec![],
        };
        assert_eq!(proxima(&[]), "A");
        let ate_h: Vec<Pmi> = "ABCDEFGH".chars().map(|c| mk(&c.to_string())).collect();
        assert_eq!(proxima(&ate_h), "J", "depois de H vem J, nunca I");
    }
}
