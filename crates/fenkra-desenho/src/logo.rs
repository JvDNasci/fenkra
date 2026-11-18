// SPDX-License-Identifier: LGPL-2.1-or-later
//! B512: logotipo do bloco de título. A imagem (PNG ou JPEG) vira retângulos cheios coloridos, um por
//! trecho de mesma cor de cada linha de pixels, numa grade reduzida. Assim o logotipo é só mais uma
//! lista de primitivas `Fill`: aparece na tela e sai igual no PDF, no SVG e no DXF (como hachura
//! sólida), sem um tipo novo de primitiva em cada formato. O fundo branco ou transparente não vira
//! nada, para o logotipo não tampar as linhas da célula.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

/// Pixels no lado maior da grade reduzida: nítido na folha e leve no DXF.
pub const LADO: u32 = 96;

/// Trecho de uma linha de pixels: linha (0 = de cima), coluna inicial, coluna final (exclusiva) e cor.
pub type Trecho = (u32, u32, u32, [u8; 3]);

/// Imagem reduzida, pronta para virar retângulos.
#[derive(Clone, Debug, PartialEq)]
pub struct Retalhos {
    pub largura: u32,
    pub altura: u32,
    pub trechos: Vec<Trecho>,
}

fn cache() -> &'static Mutex<HashMap<PathBuf, (Option<SystemTime>, Arc<Retalhos>)>> {
    static C: OnceLock<Mutex<HashMap<PathBuf, (Option<SystemTime>, Arc<Retalhos>)>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Retalhos da imagem do arquivo (guardados enquanto o arquivo não muda). `None` se não abriu.
pub fn carregar(caminho: &Path) -> Option<Arc<Retalhos>> {
    let data = std::fs::metadata(caminho).ok()?.modified().ok();
    if let Some((d, r)) = cache().lock().ok()?.get(caminho) {
        if *d == data {
            return Some(r.clone());
        }
    }
    let bytes = std::fs::read(caminho).ok()?;
    let r = Arc::new(de_bytes(&bytes)?);
    if let Ok(mut c) = cache().lock() {
        c.insert(caminho.to_path_buf(), (data, r.clone()));
    }
    Some(r)
}

/// Retalhos de uma imagem em memória (PNG ou JPEG).
pub fn de_bytes(bytes: &[u8]) -> Option<Retalhos> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let k = (w.max(h) as f64 / LADO as f64).max(1.);
    let (nw, nh) = (((w as f64 / k).round() as u32).max(1), ((h as f64 / k).round() as u32).max(1));
    let mut trechos = vec![];
    for y in 0..nh {
        let mut atual: Option<(u32, [u8; 3])> = None;
        for x in 0..=nw {
            let cor = (x < nw).then(|| media(&img, x, y, nw, nh)).flatten();
            if atual.map(|a| a.1) != cor {
                if let Some((x0, c)) = atual.take() {
                    trechos.push((y, x0, x, c));
                }
                atual = cor.map(|c| (x, c));
            }
        }
    }
    Some(Retalhos { largura: nw, altura: nh, trechos })
}

/// Cor média do bloco de pixels da célula (x, y) da grade, em 16 níveis por canal; `None` para
/// transparente ou quase branco.
fn media(img: &image::RgbaImage, x: u32, y: u32, nw: u32, nh: u32) -> Option<[u8; 3]> {
    let (w, h) = img.dimensions();
    let (x0, x1) = (x * w / nw, ((x + 1) * w / nw).max(x * w / nw + 1).min(w));
    let (y0, y1) = (y * h / nh, ((y + 1) * h / nh).max(y * h / nh + 1).min(h));
    let (mut s, mut a, mut n) = ([0u64; 3], 0u64, 0u64);
    for yy in y0..y1 {
        for xx in x0..x1 {
            let p = img.get_pixel(xx, yy).0;
            let al = p[3] as u64;
            for i in 0..3 {
                s[i] += p[i] as u64 * al;
            }
            a += al;
            n += 1;
        }
    }
    if n == 0 || a < n * 128 {
        return None;
    }
    let c = [s[0] / a, s[1] / a, s[2] / a].map(|v| ((v as f64 / 17.).round() * 17.).min(255.) as u8);
    if c.iter().all(|v| *v >= 238) {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imagem_vira_trechos_sem_o_fundo() {
        // 4×2: linha de cima vermelha nas duas primeiras colunas, o resto branco; embaixo tudo azul
        let mut img = image::RgbaImage::from_pixel(4, 2, image::Rgba([255, 255, 255, 255]));
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([255, 0, 0, 255]));
        for x in 0..4 {
            img.put_pixel(x, 1, image::Rgba([0, 0, 255, 255]));
        }
        let mut png = vec![];
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let r = de_bytes(&png).unwrap();
        assert_eq!((r.largura, r.altura), (4, 2));
        assert_eq!(r.trechos, vec![(0, 0, 2, [255, 0, 0]), (1, 0, 4, [0, 0, 255])]);
    }
}
