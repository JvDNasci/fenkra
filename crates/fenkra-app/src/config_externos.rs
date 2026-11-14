// SPDX-License-Identifier: LGPL-2.1-or-later
//! Página "Programas externos" das configurações (B211): onde está o LibreDWG, que lê e grava DWG.
//!
//! O LibreDWG é GPL e fica fora do executável (ADR 0090). A pasta escolhida aqui vai para a variável
//! `FENKRA_LIBREDWG`, que o conversor lê no próprio processo e o processo filho herda.

use crate::i18n::tr;
use crate::settings::Settings;
use eframe::egui::{self, RichText};
use fenkra_conversor::dwg;

/// Passa a pasta escolhida ao conversor. Vazia: o conversor procura no PATH e nas pastas de costume.
pub fn aplicar(s: &Settings) {
    let atual = std::env::var(dwg::VARIAVEL).unwrap_or_default();
    if atual == s.libredwg.trim() {
        return;
    }
    if s.libredwg.trim().is_empty() {
        std::env::remove_var(dwg::VARIAVEL);
    } else {
        std::env::set_var(dwg::VARIAVEL, s.libredwg.trim());
    }
}

pub fn pagina(ui: &mut egui::Ui, s: &mut Settings) {
    ui.label(RichText::new(tr("LibreDWG (DWG)")).strong());
    ui.label(RichText::new(tr("Programa livre, instalado à parte, que o Fenkra chama para ler e gravar DWG. Ele roda em outro processo e não faz parte do Fenkra.")).weak().small());
    ui.horizontal(|ui| {
        ui.label(tr("Pasta"));
        ui.add(egui::TextEdit::singleline(&mut s.libredwg).desired_width(320.).hint_text(tr("vazio: procura no PATH"))).on_hover_text(tr("Pasta com dwg2dxf e dxf2dwg. Vazia, o Fenkra procura no PATH e nas pastas de instalação de costume."));
        if ui.button(tr("Escolher…")).on_hover_text(tr("Escolhe a pasta do LibreDWG.")).clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                s.libredwg = p.to_string_lossy().into();
            }
        }
    });
    aplicar(s);
    let (ler, gravar) = (dwg::programa("dwg2dxf"), dwg::programa("dxf2dwg"));
    let linha = |ui: &mut egui::Ui, nome: &str, achado: &Option<std::path::PathBuf>| {
        let texto = match achado {
            Some(p) => format!("✔ {nome}: {}", p.display()),
            None => format!("✖ {nome}: {}", tr("não encontrado")),
        };
        ui.label(RichText::new(texto).small());
    };
    linha(ui, "dwg2dxf", &ler);
    linha(ui, "dxf2dwg", &gravar);
    if ler.is_none() || gravar.is_none() {
        ui.label(RichText::new(tr("Como instalar: baixe o pacote para Windows em ftp.gnu.org/gnu/libredwg (ou rode vcpkg install libredwg[tools]), descompacte e escolha aqui a pasta com dwg2dxf e dxf2dwg.")).small());
    }
}
