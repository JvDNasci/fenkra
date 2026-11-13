// SPDX-License-Identifier: LGPL-2.1-or-later
//! Janela "Engrenagens": abas Projeto e Verificar e a linha da correção mínima (B52).
//!
//! A aba **Projeto** é a que define a geometria (já existente em `asm_ui::gears_window`); a aba
//! **Verificar** só confere o par, com o método de resistência escolhido, e salva o relatório em
//! HTML. Nada da aba Verificar muda a peça.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_solidos::engrenagens_resistencia::{relatorio_html, verificar_par, EntradaVerificacao, Metodo, ResultadoVerificacao};
use fenkra_solidos::ParamEngrenagem;

/// Aba da janela de engrenagens.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Aba {
    #[default]
    Projeto,
    Verificar,
    /// eixo escalonado, chaveta e resistência do eixo (B52: cada aba cabe numa rolagem)
    Eixo,
}

/// Estado da aba Verificar (fica no `GearsUi`).
#[derive(Clone, Debug, Default)]
pub struct EstadoVerificar {
    pub aba: Aba,
    pub entrada: EntradaVerificacao,
    /// última mensagem do relatório salvo
    pub msg: Option<String>,
    /// entradas da resistência do eixo (aba Eixo)
    pub eixo: fenkra_solidos::eixos_resistencia::EntradaEixo,
}

/// Botões das abas no topo da janela.
pub fn abas(ui: &mut egui::Ui, v: &mut EstadoVerificar) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut v.aba, Aba::Projeto, RichText::new(tr("Projeto")).strong()).on_hover_text(tr("Define a geometria: tipo, módulo, dentes, ângulos, largura, furo e correção."));
        ui.selectable_value(&mut v.aba, Aba::Verificar, RichText::new(tr("Verificar")).strong()).on_hover_text(tr("Confere a resistência do par para a potência e a rotação dadas, sem mudar a peça."));
        ui.selectable_value(&mut v.aba, Aba::Eixo, RichText::new(tr("Eixo")).strong()).on_hover_text(tr("Eixo escalonado, recursos de seção, chaveta e resistência do eixo."));
    });
    ui.separator();
}

/// Texto da correção mínima, ex.: "mínimo sem rebaixo: x ≥ 0,415".
pub fn texto_correcao_minima(x_min: f64) -> String {
    format!("{} x ≥ {}", tr("mínimo sem rebaixo:"), crate::app::fmt((x_min * 1000.).round() / 1000.))
}

/// Linha abaixo do campo de correção: mostra o mínimo calculado e oferece usá-lo. Devolve se mudou x.
pub fn linha_correcao_minima(ui: &mut egui::Ui, p: &mut ParamEngrenagem) -> bool {
    let Ok(c) = fenkra_solidos::calculo_engrenagem(p) else { return false };
    let mut mudou = false;
    ui.horizontal_wrapped(|ui| {
        let cor = if c.rebaixado { ui.visuals().warn_fg_color } else { ui.visuals().weak_text_color() };
        ui.label(RichText::new(texto_correcao_minima(c.correcao_minima)).small().color(cor)).on_hover_text(tr(
            "Abaixo deste x a ponta da ferramenta corta o pé do dente (rebaixo) e enfraquece a raiz. O valor sai do ponto em que o arco da ponta encontra o flanco reto da ferramenta.",
        ));
        if c.correcao_minima > p.coef_correcao + 1e-9 {
            let alvo = (c.correcao_minima * 1000.).ceil() / 1000.;
            if alvo <= 1. && ui.small_button(tr("Usar o mínimo")).on_hover_text(tr("Troca a correção x pelo mínimo que evita o rebaixo.")).clicked() {
                p.coef_correcao = alvo;
                mudou = true;
            }
        }
    });
    mudou
}

fn material(ui: &mut egui::Ui, id: &str, m: &mut fenkra_solidos::calculo_projeto::Material) {
    egui::ComboBox::from_id_salt(id).selected_text(tr(m.nome)).show_ui(ui, |ui| {
        for op in fenkra_solidos::calculo_projeto::MATERIAIS {
            if ui.selectable_label(op.nome == m.nome, tr(op.nome)).clicked() {
                *m = *op;
            }
        }
    });
}

/// Conteúdo da aba Verificar. `par` é o par calculado (pinhão e coroa) ou o erro do Projeto.
pub fn aba_verificar(ui: &mut egui::Ui, v: &mut EstadoVerificar, par: Result<(&ParamEngrenagem, &ParamEngrenagem), &str>) {
    ui.label(RichText::new(tr(fenkra_solidos::calculo_projeto::AVISO_PRELIMINAR)).weak().small());
    crate::editor::section(ui, "Método");
    egui::ComboBox::from_id_salt("metodo_resistencia").selected_text(tr(v.entrada.metodo.rotulo())).show_ui(ui, |ui| {
        for m in Metodo::TODOS {
            ui.selectable_value(&mut v.entrada.metodo, m, tr(m.rotulo()));
        }
    });
    ui.label(RichText::new(tr("Flexão no pé pela parábola de Lewis no dente gerado (com o filete de raiz) e pressão de contato de Hertz no ponto primitivo, com o fator dinâmico de Barth.")).weak().small());
    crate::editor::section(ui, "Carga");
    let e = &mut v.entrada;
    let campo = |ui: &mut egui::Ui, rotulo: &str, dica: &str, x: &mut f64, r: std::ops::RangeInclusive<f64>, suf: &str| {
        ui.horizontal(|ui| {
            ui.label(tr(rotulo)).on_hover_text(tr(dica));
            let resp = ui.add(crate::ui_ext::dv(x).range(r).speed(0.1).max_decimals(3).suffix(suf)).on_hover_text(tr(dica));
            crate::ui_ext::wheel(ui, &resp, x);
        });
    };
    campo(ui, "Potência", "Potência que o par transmite, no eixo do pinhão.", &mut e.potencia_kw, 0.001..=100_000., " kW");
    campo(ui, "Rotação do pinhão", "Rotação do pinhão; a da coroa sai da razão.", &mut e.rpm_pinhao, 0.1..=100_000., " rpm");
    campo(ui, "Segurança exigida", "Coeficiente de segurança mínimo; abaixo dele aparece o aviso.", &mut e.fs_exigido, 1.0..=10., "");
    crate::editor::section(ui, "Materiais");
    ui.horizontal(|ui| {
        ui.label(tr("Pinhão")).on_hover_text(tr("Material do pinhão: E, ν e escoamento típicos, usados na flexão e no contato."));
        material(ui, "mat_pinhao", &mut e.material_pinhao);
    });
    ui.horizontal(|ui| {
        ui.label(tr("Coroa")).on_hover_text(tr("Material da coroa: E, ν e escoamento típicos, usados na flexão e no contato."));
        material(ui, "mat_coroa", &mut e.material_coroa);
    });
    campo(ui, "Pressão de contato admissível", "Pressão de Hertz que o material mais fraco aguenta (do fornecedor ou da norma do material); 0 = só mostra a pressão, sem comparar.", &mut e.contato_admissivel, 0.0..=10_000., " MPa");

    crate::editor::section(ui, "Resultados");
    let (p, c) = match par {
        Ok(x) => x,
        Err(erro) => {
            ui.label(RichText::new(format!("⚠ {} {erro}", tr("Corrija o projeto:"))).color(ui.visuals().error_fg_color));
            return;
        }
    };
    let r = match verificar_par(p, c, e) {
        Ok(r) => r,
        Err(erro) => {
            ui.label(RichText::new(format!("⚠ {erro}")).color(ui.visuals().error_fg_color));
            return;
        }
    };
    grade_resultados(ui, &r, e.fs_exigido);
    for a in &r.avisos {
        ui.label(RichText::new(format!("⚠ {a}")).small().color(ui.visuals().warn_fg_color));
    }
    ui.add_space(4.);
    if ui.button(tr("Exportar relatório...")).on_hover_text(tr("Salva a verificação (entradas, método, resultados e avisos) numa página HTML para anexar ao projeto.")).clicked() {
        let html = relatorio_html(p, c, e, &r, &|s| tr(s).to_string());
        if let Some(caminho) = rfd::FileDialog::new().add_filter(tr("Página web"), &["html"]).set_file_name("verificacao_engrenagens.html").save_file() {
            v.msg = Some(match std::fs::write(&caminho, html) {
                Ok(()) => format!("{} {}", tr("Relatório salvo em"), caminho.display()),
                Err(err) => format!("{} {err}", tr("Não consegui salvar o relatório:")),
            });
        }
    }
    if let Some(m) = &v.msg {
        ui.label(RichText::new(m).small());
    }
}

fn grade_resultados(ui: &mut egui::Ui, r: &ResultadoVerificacao, fs: f64) {
    let f = |x: f64| if x == 0. { "—".to_string() } else { crate::app::fmt((x * 1000.).round() / 1000.) };
    egui::Grid::new("grade_verificar").num_columns(3).spacing([14., 3.]).show(ui, |ui| {
        ui.label("");
        ui.label(RichText::new(tr("Pinhão")).strong());
        ui.label(RichText::new(tr("Coroa")).strong());
        ui.end_row();
        for (rotulo, a, b) in [
            ("Torque no pinhão (N·m)", r.torque_pinhao, 0.),
            ("Força tangencial Ft (N)", r.forca_tangencial, 0.),
            ("Força radial Fr (N)", r.forca_radial, 0.),
            ("Velocidade no primitivo (m/s)", r.velocidade, 0.),
            ("Fator dinâmico Kv", r.kv, 0.),
            ("Fator de forma de Lewis Y", r.y_pinhao, r.y_coroa),
            ("Tensão de flexão no pé (MPa)", r.flexao_pinhao, r.flexao_coroa),
            ("Pressão de contato de Hertz (MPa)", r.contato, 0.),
        ] {
            ui.label(tr(rotulo));
            ui.label(f(a));
            ui.label(f(b));
            ui.end_row();
        }
        for (rotulo, a, b) in [("Segurança à flexão", r.fs_flexao_pinhao, r.fs_flexao_coroa), ("Segurança ao contato", r.fs_contato, 0.)] {
            ui.label(RichText::new(tr(rotulo)).strong());
            for x in [a, b] {
                let cor = if x > 0. && x < fs { ui.visuals().warn_fg_color } else { ui.visuals().text_color() };
                ui.label(RichText::new(f(x)).strong().color(cor));
            }
            ui.end_row();
        }
    });
}

pub static TEXTOS: &[(&str, &str)] = &[
    ("Define a geometria: tipo, módulo, dentes, ângulos, largura, furo e correção.", "Defines the geometry: type, module, teeth, angles, face width, bore and correction."),
    ("Confere a resistência do par para a potência e a rotação dadas, sem mudar a peça.", "Checks the strength of the pair for the given power and speed, without changing the part."),
    ("mínimo sem rebaixo:", "minimum without undercut:"),
    (
        "Abaixo deste x a ponta da ferramenta corta o pé do dente (rebaixo) e enfraquece a raiz. O valor sai do ponto em que o arco da ponta encontra o flanco reto da ferramenta.",
        "Below this x the cutter tip cuts into the tooth root (undercut) and weakens it. The value comes from the point where the tip arc meets the straight flank of the cutter.",
    ),
    ("Usar o mínimo", "Use the minimum"),
    ("Troca a correção x pelo mínimo que evita o rebaixo.", "Sets the correction x to the minimum that avoids undercut."),
    ("Método", "Method"),
    ("Lewis e Hertz (livro-texto)", "Lewis and Hertz (textbook)"),
    ("ISO 6336 (ainda não disponível)", "ISO 6336 (not available yet)"),
    (
        "Flexão no pé pela parábola de Lewis no dente gerado (com o filete de raiz) e pressão de contato de Hertz no ponto primitivo, com o fator dinâmico de Barth.",
        "Root bending by the Lewis parabola on the generated tooth (with the root fillet) and Hertz contact pressure at the pitch point, with the Barth dynamic factor.",
    ),
    ("Potência", "Power"),
    ("Potência que o par transmite, no eixo do pinhão.", "Power transmitted by the pair, at the pinion shaft."),
    ("Rotação do pinhão", "Pinion speed"),
    ("Rotação do pinhão; a da coroa sai da razão.", "Pinion speed; the gear speed follows from the ratio."),
    ("Segurança exigida", "Required safety factor"),
    ("Coeficiente de segurança mínimo; abaixo dele aparece o aviso.", "Minimum safety factor; below it a warning shows up."),
    ("Material do pinhão: E, ν e escoamento típicos, usados na flexão e no contato.", "Pinion material: typical E, ν and yield, used for bending and contact."),
    ("Material da coroa: E, ν e escoamento típicos, usados na flexão e no contato.", "Gear material: typical E, ν and yield, used for bending and contact."),
    ("Pressão de contato admissível", "Allowable contact pressure"),
    (
        "Pressão de Hertz que o material mais fraco aguenta (do fornecedor ou da norma do material); 0 = só mostra a pressão, sem comparar.",
        "Hertz pressure the weaker material withstands (from the supplier or the material standard); 0 = only shows the pressure, without comparing.",
    ),
    ("Resultados", "Results"),
    ("Corrija o projeto:", "Fix the design:"),
    ("Exportar relatório...", "Export report..."),
    ("Salva a verificação (entradas, método, resultados e avisos) numa página HTML para anexar ao projeto.", "Saves the check (inputs, method, results and warnings) as an HTML page to attach to the project."),
    ("Relatório salvo em", "Report saved to"),
    ("Não consegui salvar o relatório:", "Could not save the report:"),
    ("Torque no pinhão (N·m)", "Pinion torque (N·m)"),
    ("Força tangencial Ft (N)", "Tangential force Ft (N)"),
    ("Força radial Fr (N)", "Radial force Fr (N)"),
    ("Velocidade no primitivo (m/s)", "Pitch line velocity (m/s)"),
    ("Fator dinâmico Kv", "Dynamic factor Kv"),
    ("Fator de forma de Lewis Y", "Lewis form factor Y"),
    ("Tensão de flexão no pé (MPa)", "Root bending stress (MPa)"),
    ("Pressão de contato de Hertz (MPa)", "Hertz contact pressure (MPa)"),
    ("Segurança à flexão", "Bending safety factor"),
    ("Segurança ao contato", "Contact safety factor"),
    ("Verificação do par de engrenagens", "Gear pair check"),
    ("Escoamento Sy (MPa)", "Yield Sy (MPa)"),
    ("Largura (mm)", "Face width (mm)"),
    ("Potência e rotação do pinhão", "Pinion power and speed"),
    ("segurança exigida", "required safety factor"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textos_novos_tem_traducao_e_nao_repetem() {
        let mut vistos = std::collections::HashSet::new();
        for (pt, en) in TEXTOS {
            assert!(!en.is_empty() && vistos.insert(*pt), "{pt}");
        }
        for m in Metodo::TODOS {
            assert!(TEXTOS.iter().any(|(pt, _)| *pt == m.rotulo()), "{}", m.rotulo());
        }
    }

    #[test]
    fn correcao_minima_aparece_com_tres_casas() {
        let c = fenkra_solidos::calculo_engrenagem(&ParamEngrenagem { dentes: 10, coef_correcao: 0.5, ..Default::default() }).unwrap();
        let t = texto_correcao_minima(c.correcao_minima);
        assert!(t.contains("0,415") || t.contains("0.415"), "{t}");
    }

    #[test]
    fn usar_o_minimo_poe_x_logo_acima_do_limite_e_tira_o_rebaixo() {
        let mut p = ParamEngrenagem { dentes: 10, ..Default::default() };
        assert!(fenkra_solidos::calculo_engrenagem(&p).unwrap().rebaixado);
        let ctx = egui::Context::default();
        let mut mudou = false;
        // sem clique o x não muda
        let mut saida = ctx.run_ui(Default::default(), |ui| {
            mudou = linha_correcao_minima(ui, &mut p);
        });
        saida.textures_delta.clear();
        assert!(!mudou && p.coef_correcao == 0.);
        // o mesmo arredondamento do botão tira o rebaixo
        let x = (fenkra_solidos::calculo_engrenagem(&p).unwrap().correcao_minima * 1000.).ceil() / 1000.;
        assert!(!fenkra_solidos::calculo_engrenagem(&ParamEngrenagem { coef_correcao: x, ..p }).unwrap().rebaixado);
    }
}
