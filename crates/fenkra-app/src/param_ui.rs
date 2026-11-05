// SPDX-License-Identifier: LGPL-2.1-or-later
//! Interface dos parâmetros com unidades, tipos, limites e supressão por expressão (B101).
//!
//! A tabela de parâmetros continua em `editor.rs`; aqui ficam as peças novas: o valor de cada tipo
//! na linha, o painel de detalhes (tipo, unidade de exibição, limites, checagem, opções), o texto dos
//! avisos da árvore e o campo "Suprimir quando" dos comandos.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::expr;
use fenkra_core::parametros::{preencher, AvisoParam, ParamTipo, UnidadeComp};
use fenkra_core::{Document, Id, Parameter, Unit};
use std::collections::HashMap;

fn fmt(v: f64) -> String {
    let t = format!("{v:.4}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Texto do valor na árvore e na tabela: na unidade de exibição, ou o texto do parâmetro de texto.
pub fn valor_texto(p: &Parameter, vars: &HashMap<String, f64>) -> Option<String> {
    match p.tipo {
        ParamTipo::Texto => Some(p.expr.clone()),
        ParamTipo::Booleano => vars.get(&p.name).map(|v| tr(if *v != 0.0 { "verdadeiro" } else { "falso" }).to_string()),
        _ => vars.get(&p.name).map(|v| {
            if p.unit == Unit::Comprimento {
                format!("{} {}", fmt(p.valor_exibido(*v)), p.exibir.simbolo())
            } else {
                format!("{} {}", fmt(*v), p.unit.symbol())
            }
        }),
    }
}

/// Mensagem do aviso, traduzida.
pub fn aviso_texto(a: &AvisoParam) -> String {
    preencher(tr(a.molde()), &a.args())
}

/// Todos os avisos de um parâmetro, um por linha (para o tooltip da árvore).
pub fn avisos_do_param_texto(avisos: &[AvisoParam], id: Id) -> Option<String> {
    let v: Vec<String> = avisos.iter().filter(|a| a.param == id).map(aviso_texto).collect();
    (!v.is_empty()).then(|| v.join("\n"))
}

/// Célula de edição da expressão, conforme o tipo do parâmetro.
pub fn editar_expr(ui: &mut egui::Ui, p: &mut Parameter) {
    match p.tipo {
        ParamTipo::Lista if !p.opcoes.is_empty() => {
            let atual = p.expr.clone();
            egui::ComboBox::from_id_salt(("lista", p.id.0)).width(100.).selected_text(atual).show_ui(ui, |ui| {
                for o in p.opcoes.clone() {
                    ui.selectable_value(&mut p.expr, o.clone(), o);
                }
            });
        }
        ParamTipo::Booleano if matches!(p.expr.trim(), "0" | "1" | "true" | "false") => {
            let mut b = matches!(p.expr.trim(), "1" | "true");
            if ui.checkbox(&mut b, "").changed() {
                p.expr = if b { "true".into() } else { "false".into() };
            }
        }
        _ => {
            let re = ui.add(egui::TextEdit::singleline(&mut p.expr).desired_width(100.));
            if p.tipo == ParamTipo::Numero {
                crate::ui_ext::wheel_text(ui, &re, &mut p.expr);
            }
        }
    }
}

/// Painel de detalhes de um parâmetro: tipo, unidade de exibição, limites, checagem e opções.
pub fn detalhes(ui: &mut egui::Ui, p: &mut Parameter) {
    ui.group(|ui| {
        ui.label(RichText::new(format!("{} · {}", tr("Detalhes de"), p.name)).strong());
        egui::Grid::new(("param_det", p.id.0)).num_columns(2).show(ui, |ui| {
            ui.label(tr("Tipo"));
            egui::ComboBox::from_id_salt(("param_tipo", p.id.0)).selected_text(tr(p.tipo.rotulo())).show_ui(ui, |ui| {
                for t in ParamTipo::TODOS {
                    if ui.selectable_value(&mut p.tipo, t, tr(t.rotulo())).changed() && t == ParamTipo::Booleano && p.unit != Unit::Numero {
                        p.unit = Unit::Numero;
                    }
                }
            })
            .response
            .on_hover_text(tr("Número, texto, booleano (verdadeiro ou falso) ou escolha numa lista. Texto não entra em contas; booleano vale 1 ou 0 nas expressões."));
            ui.end_row();
            if p.unit == Unit::Comprimento && p.tipo != ParamTipo::Texto {
                ui.label(tr("Mostrar em"));
                egui::ComboBox::from_id_salt(("param_exibir", p.id.0)).selected_text(p.exibir.simbolo()).show_ui(ui, |ui| {
                    for u in UnidadeComp::TODAS {
                        ui.selectable_value(&mut p.exibir, u, u.simbolo());
                    }
                })
                .response
                .on_hover_text(tr("Só muda como o valor aparece. A geometria continua a mesma: o valor interno é sempre em mm. Na expressão, escreva a unidade (2 pol, 10 cm) para converter."));
                ui.end_row();
            }
            if p.tipo != ParamTipo::Texto {
                for (rot, dica, campo) in [
                    ("Mínimo", "Expressão do menor valor aceito. Fora do limite o parâmetro avisa na árvore, mas o cálculo continua.", &mut p.min),
                    ("Máximo", "Expressão do maior valor aceito. Fora do limite o parâmetro avisa na árvore, mas o cálculo continua.", &mut p.max),
                    ("Checagem", "Condição que deve ser verdadeira, por exemplo Largura > Espessura*2. Se der falso, avisa na árvore.", &mut p.verificacao),
                ] {
                    ui.label(tr(rot));
                    ui.add(egui::TextEdit::singleline(campo).desired_width(160.)).on_hover_text(tr(dica));
                    ui.end_row();
                }
            }
        });
        if p.tipo == ParamTipo::Lista {
            ui.label(RichText::new(tr("Opções da lista")).small().weak());
            let mut tirar = None;
            for (i, o) in p.opcoes.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(o).desired_width(120.)).on_hover_text(tr("Cada opção é uma expressão, por exemplo 6, 8 ou 1/4 pol."));
                    if ui.small_button("✖").clicked() {
                        tirar = Some(i);
                    }
                });
            }
            if let Some(i) = tirar {
                p.opcoes.remove(i);
            }
            if ui.small_button(tr("➕ Opção")).clicked() {
                let n = p.opcoes.len() + 1;
                p.opcoes.push(if p.opcoes.is_empty() { p.expr.clone() } else { format!("{n}") });
            }
        }
    });
}

/// Campo "Suprimir quando" de um comando. Devolve `true` se o texto mudou.
pub fn suprimir_se(ui: &mut egui::Ui, texto: &mut String, doc: &Document) -> bool {
    let mut mudou = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Suprimir quando")).small());
        let r = ui
            .add(egui::TextEdit::singleline(texto).desired_width(140.).hint_text(tr("ex.: Furos < 2")))
            .on_hover_text(tr("Expressão com os parâmetros: o comando fica suprimido enquanto o resultado for diferente de zero (ex.: 1 - ComChanfro). Vazio: só vale a caixa Suprimido."));
        mudou = r.changed();
    });
    if !texto.trim().is_empty() {
        if let Ok(vars) = doc.eval_params() {
            match expr::eval_str(texto, &vars) {
                Ok(v) => {
                    ui.label(RichText::new(tr(if v != 0.0 { "Suprimido agora" } else { "Ativo agora" })).weak().small());
                }
                Err(e) => {
                    ui.label(RichText::new(format!("⚠ {e}")).color(crate::theme::st_warn()).small());
                }
            }
        }
    }
    mudou
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valor_na_unidade_de_exibicao_e_tipos() {
        let mut d = Document::new("t");
        let a = d.add_param("L", "2 in", Unit::Comprimento).unwrap();
        let t = d.add_param("Cod", "AB-1", Unit::Numero).unwrap();
        let b = d.add_param("Liga", "1", Unit::Numero).unwrap();
        d.params.iter_mut().find(|p| p.id == a).unwrap().exibir = UnidadeComp::Pol;
        d.params.iter_mut().find(|p| p.id == t).unwrap().tipo = ParamTipo::Texto;
        d.params.iter_mut().find(|p| p.id == b).unwrap().tipo = ParamTipo::Booleano;
        let v = d.eval_params().unwrap();
        let por = |id| d.params.iter().find(|p| p.id == id).unwrap();
        assert_eq!(valor_texto(por(a), &v).unwrap(), "2 pol");
        assert_eq!(valor_texto(por(t), &v).unwrap(), "AB-1");
        assert_eq!(valor_texto(por(b), &v).unwrap(), "verdadeiro");
    }

    #[test]
    fn avisos_da_arvore_tem_texto_e_traducao() {
        let mut d = Document::new("t");
        let w = d.add_param("W", "3", Unit::Comprimento).unwrap();
        d.params[0].min = "5".into();
        let av = d.avisos_param();
        assert_eq!(avisos_do_param_texto(&av, w).unwrap(), "W está abaixo do mínimo (3 < 5)");
        for a in &av {
            assert!(crate::i18n::tem_traducao(a.molde()), "{}", a.molde());
        }
        for m in [
            "{} está acima do máximo ({} > {})",
            "A checagem de {} falhou",
            "Expressão inválida em {}: {}",
            "{} não é uma das opções da lista",
            "A lista {} não tem opções",
            "{} usa uma unidade que não combina com o tipo dele",
        ] {
            assert!(crate::i18n::tem_traducao(m), "{m}");
        }
    }
}
