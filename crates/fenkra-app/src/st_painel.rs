// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: seletor visual de perfil e ancoragem clicável (B254). Cascata norma, tipo e tamanho,
//! com busca e miniatura da seção em escala; a ancoragem se escolhe clicando os nove pontos e o
//! centróide sobre a própria seção, que mostra o nome de cada cota.

use crate::app::fmt;
use crate::i18n::tr;
use crate::st_secao::{lado_maior, miniatura, secao_com_cotas};
use eframe::egui::{self, RichText};
use fenkra_core::StructMember;
use fenkra_solidos::structural::{self, Family};

/// Cotas da medida do membro, avaliadas (cotas com expressão que não resolve viram 0 e a seção
/// inválida aparece como tal).
fn dims_do_membro(m: &StructMember, vars: &std::collections::HashMap<String, f64>) -> Vec<f64> {
    m.dims.iter().map(|e| fenkra_core::expr::eval_str(e, vars).unwrap_or(0.)).collect()
}

/// Normas (grupos) presentes na lista, na ordem em que aparecem.
pub fn normas(fams: &[Family]) -> Vec<String> {
    let mut v: Vec<String> = vec![];
    for f in fams {
        if !v.contains(&f.norm) {
            v.push(f.norm.clone());
        }
    }
    v
}

/// Texto da busca casa com a família, a norma ou a medida (sem diferenciar maiúsculas).
pub fn casa(busca: &str, fam: &Family, medida: &str) -> bool {
    let b = busca.trim().to_lowercase();
    b.is_empty() || b.split_whitespace().all(|w| format!("{} {} {}", fam.norm, fam.name, medida).to_lowercase().contains(w))
}

/// Resultado da busca: (família, índice da medida), até `limite`.
pub fn buscar<'a>(fams: &'a [Family], busca: &str, limite: usize) -> Vec<(&'a Family, usize)> {
    fams.iter().flat_map(|f| f.sizes.iter().enumerate().filter(|(_, (n, _))| casa(busca, f, n)).map(move |(i, _)| (f, i))).take(limite).collect()
}

fn escolher(m: &mut StructMember, f: &Family, i: usize) {
    m.family = f.name.clone();
    m.kind = f.kind;
    if let Some((n, d)) = f.sizes.get(i) {
        m.designation = n.clone();
        m.dims = d.iter().map(|v| fmt(*v)).collect();
    }
}

fn escala_da_familia(f: &Family) -> f64 {
    f.sizes.iter().filter_map(|(_, d)| lado_maior(f.kind, d)).fold(0., f64::max).max(1.)
}

/// Seletor em cascata: busca, norma, tipo e a grade de tamanhos com miniaturas na mesma escala.
pub fn seletor(ui: &mut egui::Ui, fams: &[Family], m: &mut StructMember) {
    let chave = egui::Id::unique("st_busca");
    let mut busca: String = ui.ctx().data(|d| d.get_temp(chave)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Buscar")));
        ui.add(egui::TextEdit::singleline(&mut busca).desired_width(180.).hint_text(tr("tubo, W200, 40x3…"))).on_hover_text(tr("Procura em norma, tipo e medida. Clique num resultado para escolher."));
        if !busca.is_empty() && ui.small_button("✖").clicked() {
            busca.clear();
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(chave, busca.clone()));
    if !busca.trim().is_empty() {
        let achados = buscar(fams, &busca, 60);
        if achados.is_empty() {
            ui.label(RichText::new(tr("Nada encontrado.")).weak());
        }
        egui::ScrollArea::vertical().id_salt("st_busca_lista").max_height(170.).show(ui, |ui| {
            for (f, i) in achados {
                let (n, d) = &f.sizes[i];
                ui.horizontal(|ui| {
                    let marcada = m.family == f.name && m.designation == *n;
                    if miniatura(ui, f.kind, d, escala_da_familia(f), 32., marcada).clicked() || ui.selectable_label(marcada, format!("{} · {}", f.name, n)).clicked() {
                        escolher(m, f, i);
                    }
                });
            }
        });
        return;
    }
    let lista = normas(fams);
    let atual = fams.iter().find(|f| f.name == m.family);
    let norma_chave = egui::Id::unique("st_norma");
    let mut norma: String = ui.ctx().data(|d| d.get_temp(norma_chave)).or_else(|| atual.map(|f| f.norm.clone())).or_else(|| lista.first().cloned()).unwrap_or_default();
    // se a família do membro mudou de norma por fora (exemplo, arquivo), a cascata acompanha
    if let Some(f) = atual {
        if f.norm != norma && ui.ctx().data(|d| d.get_temp::<String>(egui::Id::unique("st_norma_de"))).as_deref() != Some(f.name.as_str()) {
            norma = f.norm.clone();
        }
    }
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Norma")));
        egui::ComboBox::from_id_salt("st_norma_cb").selected_text(tr(&norma)).width(180.).show_ui(ui, |ui| {
            for n in &lista {
                if ui.selectable_label(*n == norma, tr(n)).clicked() {
                    norma = n.clone();
                }
            }
        });
    });
    ui.ctx().data_mut(|d| {
        d.insert_temp(norma_chave, norma.clone());
        d.insert_temp(egui::Id::unique("st_norma_de"), m.family.clone());
    });
    let da_norma: Vec<&Family> = fams.iter().filter(|f| f.norm == norma).collect();
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Tipo")));
        egui::ComboBox::from_id_salt("st_fam").selected_text(&m.family).width(180.).show_ui(ui, |ui| {
            for f in &da_norma {
                if ui.selectable_label(f.name == m.family, &f.name).clicked() && f.name != m.family {
                    escolher(m, f, 0);
                }
            }
        });
    });
    let Some(f) = fams.iter().find(|f| f.name == m.family) else { return };
    ui.label(RichText::new(tr("Tamanho")).small());
    let escala = escala_da_familia(f);
    egui::ScrollArea::vertical().id_salt("st_tamanhos").max_height(150.).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, (n, d)) in f.sizes.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(52.);
                    let r = miniatura(ui, f.kind, d, escala, 48., *n == m.designation);
                    ui.label(RichText::new(n).small());
                    if r.on_hover_text(format!("{} · {}", f.name, n)).clicked() {
                        escolher(m, f, i);
                    }
                });
            }
        });
    });
}

/// Seção com os nomes das cotas e a ancoragem clicável.
pub fn ancoragem(ui: &mut egui::Ui, m: &mut StructMember, vars: &std::collections::HashMap<String, f64>) {
    let dims = dims_do_membro(m, vars);
    if let Some(a) = secao_com_cotas(ui, m.kind, &dims, Some(m.anchor), true) {
        m.anchor = a;
    }
    ui.label(RichText::new(tr("Clique num ponto para ele ficar sobre a linha guia; a prévia no 3D desliza o perfil junto.")).weak().small());
}

/// Lista de famílias do seletor: o nome da norma por tipo de seção, para as famílias do usuário.
pub fn norma_do_usuario() -> &'static str {
    "Minhas famílias"
}

/// A seção da biblioteca do usuário: desenho da medida escolhida com as cotas anotadas.
pub fn desenho_da_medida(ui: &mut egui::Ui, kind: fenkra_core::SectionKind, dims: &[f64]) {
    secao_com_cotas(ui, kind, dims, None, false);
    if let Err(e) = structural::section(kind, dims) {
        ui.label(RichText::new(format!("⚠ {e}")).color(crate::theme::st_err()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_busca_acha_por_tipo_norma_e_medida() {
        let fams = structural::builtin();
        let todos: usize = fams.iter().map(|f| f.sizes.len()).sum();
        assert_eq!(buscar(&fams, "", 1000).len(), todos);
        let w = buscar(&fams, "w200", 10);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0.sizes[w[0].1].0, "W200x15");
        // várias palavras: todas precisam casar
        assert_eq!(buscar(&fams, "tubo quadrado 40x3", 10).len(), 1);
        assert!(buscar(&fams, "tubo xyz", 10).is_empty());
        // pela norma (grupo)
        assert!(buscar(&fams, "laminados", 100).len() >= 7);
    }

    #[test]
    fn toda_familia_de_fabrica_tem_norma_e_a_cascata_cobre_todas() {
        let fams = structural::builtin();
        assert!(fams.iter().all(|f| !f.norm.is_empty()));
        let n = normas(&fams);
        assert!(n.len() >= 3);
        // toda família aparece em exatamente uma norma
        let por_norma: usize = n.iter().map(|x| fams.iter().filter(|f| f.norm == *x).count()).sum();
        assert_eq!(por_norma, fams.len());
    }
}
