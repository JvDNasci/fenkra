// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da tela inicial (B455), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("CAD paramétrico open source", "Open source parametric CAD"),
    ("Animações da tela inicial", "Start screen animations"),
    (
        "Realce suave ao passar o mouse e grade do fundo deslizando devagar. Fica desligada no modo leve, no adaptador gráfico por software e com \"reduzir movimento\" (do Fenkra ou do sistema).",
        "Soft highlight on hover and a slowly drifting background grid. Stays off in light mode, on a software graphics adapter and with \"reduce motion\" (Fenkra's or the system's).",
    ),
    ("desligada agora: modo leve, adaptador por software ou reduzir movimento", "off right now: light mode, software adapter or reduce motion"),
];
