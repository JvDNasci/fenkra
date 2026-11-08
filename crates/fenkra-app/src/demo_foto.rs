// SPDX-License-Identifier: LGPL-2.1-or-later
//! `FENKRA_DEMO_FOTO=<arquivo.png>`: grava a janela depois de alguns segundos e fecha o programa. Serve
//! para conferir mudança visual sem depender do que está na tela.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};

/// Segundos até a foto: a abertura, a composição e a primeira cena precisam ter passado.
const ESPERA: f64 = 9.;

pub fn quadro(ctx: &egui::Context) {
    let Ok(arquivo) = std::env::var("FENKRA_DEMO_FOTO") else { return };
    static FEITA: AtomicBool = AtomicBool::new(false);
    ctx.request_repaint();
    // montagem grande pede mais tempo: `FENKRA_DEMO_FOTO_ESPERA=<segundos>`
    let espera = std::env::var("FENKRA_DEMO_FOTO_ESPERA").ok().and_then(|v| v.parse().ok()).unwrap_or(ESPERA);
    if ctx.input(|i| i.time) > espera && !FEITA.swap(true, Ordering::Relaxed) {
        ctx.request_screenshot(move |img| {
            let [w, h] = img.size;
            let mut bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
            // FENKRA_DEMO_SIMULAR=protanopia|deuteranopia|tritanopia: grava a captura como essa visão a veria (B321)
            let def = match std::env::var("FENKRA_DEMO_SIMULAR").as_deref() {
                Ok("protanopia") => Some(crate::acessibilidade::Deficiencia::Protanopia),
                Ok("deuteranopia") => Some(crate::acessibilidade::Deficiencia::Deuteranopia),
                Ok("tritanopia") => Some(crate::acessibilidade::Deficiencia::Tritanopia),
                _ => None,
            };
            if let Some(d) = def {
                crate::acessibilidade::simular_quadro(&mut bytes, d);
            }
            if let Some(b) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                let _ = b.save(&arquivo);
            }
            std::process::exit(0);
        });
    }
}

/// `FENKRA_DEMO_APARENCIA=preto|branco|<RRGGBB>` pinta a peça aberta com essa cor, para conferir o
/// contraste dela com o fundo do tema (B425). `FENKRA_DEMO_TEMA` escolhe o tema.
pub fn aparencia(doc: &mut fenkra_core::Document) {
    let Ok(v) = std::env::var("FENKRA_DEMO_APARENCIA") else { return };
    let cor = match v.as_str() {
        "preto" => [12, 12, 14],
        "branco" => [250, 250, 250],
        hex => {
            let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0x808080);
            [(n >> 16) as u8, (n >> 8) as u8, n as u8]
        }
    };
    doc.appearance = Some(fenkra_core::Appearance { color: cor, finish: fenkra_core::Finish::Acetinado, texture: None });
}
