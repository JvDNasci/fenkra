// SPDX-License-Identifier: LGPL-2.1-or-later
//! Miniatura PNG desenhada em software, para o 3MF: gravar não pode depender da placa de vídeo nem da
//! janela aberta. Projeção isométrica com teste de profundidade e luz de frente; o fundo é transparente,
//! e a cor sai do material da malha.

use crate::{dot, normal_area, unit, zip, Cena};

const CINZA: [f32; 4] = [0.72, 0.74, 0.78, 1.];

fn pedaco(saida: &mut Vec<u8>, tipo: &[u8; 4], dados: &[u8]) {
    saida.extend_from_slice(&(dados.len() as u32).to_be_bytes());
    let inicio = saida.len();
    saida.extend_from_slice(tipo);
    saida.extend_from_slice(dados);
    let crc = zip::crc32(&saida[inicio..]);
    saida.extend_from_slice(&crc.to_be_bytes());
}

/// PNG RGBA de 8 bits, com a imagem já por linhas.
pub fn png(rgba: &[u8], lado: u32) -> Vec<u8> {
    let mut cru = Vec::with_capacity(rgba.len() + lado as usize);
    for linha in rgba.chunks_exact(lado as usize * 4) {
        cru.push(0);
        cru.extend_from_slice(linha);
    }
    let mut s = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut cabecalho = Vec::with_capacity(13);
    cabecalho.extend_from_slice(&lado.to_be_bytes());
    cabecalho.extend_from_slice(&lado.to_be_bytes());
    cabecalho.extend_from_slice(&[8, 6, 0, 0, 0]);
    pedaco(&mut s, b"IHDR", &cabecalho);
    pedaco(&mut s, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&cru, 6));
    pedaco(&mut s, b"IEND", &[]);
    s
}

/// Imagem RGBA da cena, vista de cima num canto. Vazia, a imagem fica transparente.
pub fn desenhar(cena: &Cena, lado: u32) -> Vec<u8> {
    let n = lado as usize;
    let mut imagem = vec![0u8; n * n * 4];
    let malhas = cena.no_lugar();
    // base da vista isométrica: direita, cima e para o observador
    let (dir, cima, olho) = (unit([1., -1., 0.]), unit([-1., -1., 2.]), unit([1., 1., 1.]));
    let proj = |p: [f64; 3]| [dot(p, dir), dot(p, cima), dot(p, olho)];
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in malhas.iter().flat_map(|m| &m.pos) {
        let q = proj(*p);
        (lo, hi) = ([lo[0].min(q[0]), lo[1].min(q[1])], [hi[0].max(q[0]), hi[1].max(q[1])]);
    }
    if !lo[0].is_finite() {
        return imagem;
    }
    let escala = f64::from(lado) * 0.92 / (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9);
    let (cx, cy) = ((lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.);
    let meio = f64::from(lado) / 2.;
    let tela = |q: [f64; 3]| [meio + (q[0] - cx) * escala, meio - (q[1] - cy) * escala, q[2]];
    let mut fundo = vec![f64::NEG_INFINITY; n * n];
    for m in &malhas {
        let cor = cena.cor_de(m).unwrap_or(CINZA);
        for t in &m.tris {
            let [a, b, c] = t.map(|i| m.pos[i as usize]);
            let normal = unit(normal_area(a, b, c));
            // luz vinda do olho: face de lado fica escura, sem nunca sumir
            let luz = (0.35 + 0.65 * dot(normal, olho).abs()) as f32;
            let [p, q, r] = [a, b, c].map(|v| tela(proj(v)));
            let area = (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
            if area.abs() < 1e-12 {
                continue;
            }
            let x0 = p[0].min(q[0]).min(r[0]).floor().max(0.) as usize;
            let x1 = (p[0].max(q[0]).max(r[0]).ceil() as usize).min(n.saturating_sub(1));
            let y0 = p[1].min(q[1]).min(r[1]).floor().max(0.) as usize;
            let y1 = (p[1].max(q[1]).max(r[1]).ceil() as usize).min(n.saturating_sub(1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
                    let w0 = ((q[0] - fx) * (r[1] - fy) - (q[1] - fy) * (r[0] - fx)) / area;
                    let w1 = ((r[0] - fx) * (p[1] - fy) - (r[1] - fy) * (p[0] - fx)) / area;
                    let w2 = 1. - w0 - w1;
                    if w0 < 0. || w1 < 0. || w2 < 0. {
                        continue;
                    }
                    let z = w0 * p[2] + w1 * q[2] + w2 * r[2];
                    let i = y * n + x;
                    if z > fundo[i] {
                        fundo[i] = z;
                        for k in 0..3 {
                            imagem[i * 4 + k] = (cor[k] * luz * 255.).clamp(0., 255.) as u8;
                        }
                        imagem[i * 4 + 3] = 255;
                    }
                }
            }
        }
    }
    imagem
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::teste::*;

    #[test]
    fn png_tem_assinatura_e_dimensoes() {
        let mut c = Cena::default();
        c.raiz_com(caixa([10., 10., 10.]));
        let p = png(&desenhar(&c, 64), 64);
        assert_eq!(&p[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(&p[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes([p[16], p[17], p[18], p[19]]), 64);
        assert_eq!(&p[p.len() - 8..p.len() - 4], b"IEND");
    }

    #[test]
    fn peca_aparece_no_meio_e_o_fundo_fica_transparente() {
        let mut c = Cena::default();
        c.raiz_com(caixa([10., 10., 10.]));
        let i = desenhar(&c, 64);
        let alfa = |x: usize, y: usize| i[(y * 64 + x) * 4 + 3];
        assert_eq!(alfa(32, 32), 255);
        assert_eq!(alfa(0, 0), 0);
        // o PNG volta inteiro pelo inflate: os dados não foram cortados
        let p = png(&i, 64);
        let inicio = 8 + 25 + 8;
        let tam = u32::from_be_bytes([p[33], p[34], p[35], p[36]]) as usize;
        let cru = miniz_oxide::inflate::decompress_to_vec_zlib(&p[inicio..inicio + tam]).unwrap();
        assert_eq!(cru.len(), 64 * (64 * 4 + 1));
    }

    #[test]
    fn cena_vazia_nao_quebra() {
        assert!(desenhar(&Cena::default(), 16).iter().all(|b| *b == 0));
    }
}
