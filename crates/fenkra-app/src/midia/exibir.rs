// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desenho da mídia no tooltip (B257, camada 2 do B278). Só uma mídia fica decodificada e com
//! textura de cada vez: ao pairar outro botão, a anterior é solta. Os bytes do GIF ficam no cache
//! LRU do pacote; aqui só entram a decodificação e o quadro do instante.

use super::gif::{self, Gif};
use super::pacote::{caminho_padrao, Midias};
use eframe::egui;
use std::cell::RefCell;

struct Ativa {
    id: String,
    gif: Gif,
    textura: Option<egui::TextureHandle>,
    quadro: usize,
}

struct Estado {
    midias: Midias,
    ativa: Option<Ativa>,
}

thread_local! {
    // aberto na primeira dica que pede mídia; sem pacote, `Midias` fica vazio e nada acontece
    static ESTADO: RefCell<Option<Estado>> = const { RefCell::new(None) };
}

/// Há mídia para este comando? Abre o pacote na primeira pergunta.
pub fn tem(id: &str) -> bool {
    ESTADO.with(|e| {
        let mut e = e.borrow_mut();
        let st = e.get_or_insert_with(|| Estado { midias: Midias::abrir(caminho_padrao().as_deref()), ativa: None });
        st.midias.tem(id)
    })
}

/// O botão deixou de ser pairado: libera a textura dele.
pub fn soltar(id: &str) {
    ESTADO.with(|e| {
        if let Some(st) = e.borrow_mut().as_mut() {
            if st.ativa.as_ref().is_some_and(|a| a.id == id) {
                st.ativa = None;
            }
        }
    });
}

/// Mídia ativa neste momento (para teste).
pub fn ativa() -> Option<String> {
    ESTADO.with(|e| e.borrow().as_ref().and_then(|s| s.ativa.as_ref().map(|a| a.id.clone())))
}

/// Desenha o quadro do instante `t` (segundos desde que a camada abriu); devolve falso se não
/// há mídia ou ela não abre.
pub fn desenhar(ui: &mut egui::Ui, id: &str, t: f64, largura: f32) -> bool {
    let ok = ESTADO.with(|e| {
        let mut e = e.borrow_mut();
        let Some(st) = e.as_mut() else { return false };
        if st.ativa.as_ref().is_none_or(|a| a.id != id) {
            st.ativa = None;
            let Some(bytes) = st.midias.bytes(id) else { return false };
            match gif::ler(&bytes) {
                Ok(g) => st.ativa = Some(Ativa { id: id.to_owned(), gif: g, textura: None, quadro: usize::MAX }),
                Err(_) => return false,
            }
        }
        let a = st.ativa.as_mut().expect("ativa");
        let n = a.gif.quadro_em(t);
        if a.quadro != n || a.textura.is_none() {
            let q = &a.gif.quadros[n];
            let img = egui::ColorImage::from_rgba_unmultiplied([a.gif.largura, a.gif.altura], &q.rgba);
            match a.textura.as_mut() {
                Some(tex) => tex.set(img, egui::TextureOptions::LINEAR),
                None => a.textura = Some(ui.ctx().load_texture(format!("ajuda_midia_{id}"), img, egui::TextureOptions::LINEAR)),
            }
            a.quadro = n;
        }
        let tex = a.textura.as_ref().expect("textura");
        let alt = largura * a.gif.altura as f32 / a.gif.largura as f32;
        ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), egui::vec2(largura, alt))));
        true
    });
    if ok {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(1000 / super::FPS as u64));
    }
    ok
}
