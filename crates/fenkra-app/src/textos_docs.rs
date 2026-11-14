// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos documentos abertos em abas (B363), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Documentos abertos", "Open documents"),
    ("Ctrl+Tab passa para o próximo; Ctrl+Shift+Tab volta.", "Ctrl+Tab goes to the next one; Ctrl+Shift+Tab goes back."),
    ("Próximo documento", "Next document"),
    ("Documento anterior", "Previous document"),
    ("Fechar documento", "Close document"),
    ("Fecha o documento da frente; pergunta antes se ele tem alterações por salvar.", "Closes the front document; asks first if it has unsaved changes."),
    ("ainda não salvo", "not saved yet"),
    ("com alterações por salvar", "with unsaved changes"),
    ("sem alterações por salvar", "no unsaved changes"),
    ("Abrir recente", "Open recent"),
];
