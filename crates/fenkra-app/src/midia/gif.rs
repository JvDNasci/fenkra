// SPDX-License-Identifier: LGPL-2.1-or-later
//! Leitor de GIF próprio (B257). A mídia de ajuda é GIF de 64 cores; ler com poucas linhas aqui
//! evita ligar um decodificador de animação ao executável e deixa os quadros prontos em RGBA para
//! subir como textura um a um.

/// Um quadro já composto sobre a tela inteira.
pub struct Quadro {
    pub rgba: Vec<u8>,
    /// atraso em centésimos de segundo
    pub atraso_cs: u16,
}

pub struct Gif {
    pub largura: usize,
    pub altura: usize,
    pub quadros: Vec<Quadro>,
}

impl Gif {
    /// Duração do laço em segundos.
    pub fn duracao(&self) -> f64 {
        self.quadros.iter().map(|q| q.atraso_cs.max(1) as f64 / 100.).sum()
    }

    /// Índice do quadro no instante `t` (o laço recomeça sozinho).
    pub fn quadro_em(&self, t: f64) -> usize {
        let total = self.duracao();
        if self.quadros.is_empty() || total <= 0. {
            return 0;
        }
        let mut resto = t.rem_euclid(total);
        for (i, q) in self.quadros.iter().enumerate() {
            let d = q.atraso_cs.max(1) as f64 / 100.;
            if resto < d {
                return i;
            }
            resto -= d;
        }
        self.quadros.len() - 1
    }
}

fn u16le(b: &[u8], i: usize) -> Result<usize, String> {
    match b.get(i..i + 2) {
        Some(x) => Ok(x[0] as usize | (x[1] as usize) << 8),
        None => Err("GIF truncado".into()),
    }
}

/// Junta os sub-blocos de dados a partir de `i`; devolve os bytes e a posição depois do terminador.
fn sub_blocos(b: &[u8], mut i: usize) -> Result<(Vec<u8>, usize), String> {
    let mut out = vec![];
    loop {
        let n = *b.get(i).ok_or("GIF truncado")? as usize;
        i += 1;
        if n == 0 {
            return Ok((out, i));
        }
        out.extend_from_slice(b.get(i..i + n).ok_or("GIF truncado")?);
        i += n;
    }
}

/// LZW do GIF: códigos de largura variável, do bit menos significativo.
fn lzw(dados: &[u8], min: u8, esperado: usize) -> Result<Vec<u8>, String> {
    if !(2..=11).contains(&min) {
        return Err("tamanho mínimo de código inválido".into());
    }
    let (limpar, fim) = (1usize << min, (1usize << min) + 1);
    // tabela: cada código guarda o anterior e o último byte; remonta-se ao contrário
    let mut prefixo: Vec<u16> = vec![0; 4096];
    let mut sufixo: Vec<u8> = vec![0; 4096];
    for i in 0..limpar {
        sufixo[i] = i as u8;
    }
    let mut prox = fim + 1;
    let mut largura = min as usize + 1;
    let mut out: Vec<u8> = Vec::with_capacity(esperado);
    let (mut acc, mut nbits, mut pos) = (0u32, 0usize, 0usize);
    let mut anterior: Option<usize> = None;
    let mut pilha: Vec<u8> = vec![];
    let expandir = |mut c: usize, pilha: &mut Vec<u8>, prefixo: &[u16], sufixo: &[u8]| {
        pilha.clear();
        while c >= limpar + 2 {
            pilha.push(sufixo[c]);
            c = prefixo[c] as usize;
        }
        pilha.push(sufixo[c]);
        pilha.reverse();
    };
    loop {
        while nbits < largura {
            let Some(&byte) = dados.get(pos) else { return Ok(out) };
            acc |= (byte as u32) << nbits;
            nbits += 8;
            pos += 1;
        }
        let cod = (acc & ((1 << largura) - 1)) as usize;
        acc >>= largura;
        nbits -= largura;
        if cod == limpar {
            prox = fim + 1;
            largura = min as usize + 1;
            anterior = None;
            continue;
        }
        if cod == fim {
            return Ok(out);
        }
        match anterior {
            None => {
                if cod >= limpar {
                    return Err("código inicial inválido".into());
                }
                out.push(cod as u8);
            }
            Some(ant) => {
                if cod < prox {
                    expandir(cod, &mut pilha, &prefixo, &sufixo);
                    out.extend_from_slice(&pilha);
                    if prox < 4096 {
                        prefixo[prox] = ant as u16;
                        sufixo[prox] = pilha[0];
                        prox += 1;
                    }
                } else if cod == prox && prox < 4096 {
                    expandir(ant, &mut pilha, &prefixo, &sufixo);
                    let primeiro = pilha[0];
                    out.extend_from_slice(&pilha);
                    out.push(primeiro);
                    prefixo[prox] = ant as u16;
                    sufixo[prox] = primeiro;
                    prox += 1;
                } else {
                    return Err("código fora da tabela".into());
                }
            }
        }
        anterior = Some(cod);
        if prox == (1 << largura) && largura < 12 {
            largura += 1;
        }
    }
}

/// Lê um GIF89a ou GIF87a sem entrelaçamento (o que a esteira gera).
pub fn ler(b: &[u8]) -> Result<Gif, String> {
    if b.len() < 13 || &b[0..3] != b"GIF" {
        return Err("não é um GIF".into());
    }
    let (w, h) = (u16le(b, 6)?, u16le(b, 8)?);
    if w == 0 || h == 0 || w * h > 4_000_000 {
        return Err("tamanho de GIF fora do aceito".into());
    }
    let flags = b[10];
    let mut i = 13;
    let mut global: Vec<u8> = vec![];
    if flags & 0x80 != 0 {
        let n = 3 * (1usize << ((flags & 7) + 1));
        global = b.get(i..i + n).ok_or("GIF truncado")?.to_vec();
        i += n;
    }
    let mut tela = vec![0u8; w * h * 4];
    let mut quadros = vec![];
    // controle gráfico do próximo quadro: atraso, índice transparente, descarte
    let (mut atraso, mut transp, mut descarte) = (0u16, None::<u8>, 0u8);
    while i < b.len() {
        match b[i] {
            0x3B => break,
            0x21 => {
                let rotulo = *b.get(i + 1).ok_or("GIF truncado")?;
                if rotulo == 0xF9 {
                    let p = b.get(i + 3).ok_or("GIF truncado")?;
                    descarte = (p >> 2) & 7;
                    atraso = u16le(b, i + 4)? as u16;
                    transp = (p & 1 != 0).then(|| b[i + 6]);
                }
                let (_, fim) = sub_blocos(b, i + 2)?;
                i = fim;
            }
            0x2C => {
                let (x, y, fw, fh) = (u16le(b, i + 1)?, u16le(b, i + 3)?, u16le(b, i + 5)?, u16le(b, i + 7)?);
                let p = *b.get(i + 9).ok_or("GIF truncado")?;
                i += 10;
                if p & 0x40 != 0 {
                    return Err("GIF entrelaçado não suportado".into());
                }
                let mut tabela = global.clone();
                if p & 0x80 != 0 {
                    let n = 3 * (1usize << ((p & 7) + 1));
                    tabela = b.get(i..i + n).ok_or("GIF truncado")?.to_vec();
                    i += n;
                }
                let min = *b.get(i).ok_or("GIF truncado")?;
                let (dados, fim) = sub_blocos(b, i + 1)?;
                i = fim;
                let idx = lzw(&dados, min, fw * fh)?;
                if idx.len() < fw * fh || x + fw > w || y + fh > h {
                    return Err("quadro de GIF incompleto".into());
                }
                let antes = (descarte == 3).then(|| tela.clone());
                for (n, &c) in idx.iter().take(fw * fh).enumerate() {
                    if Some(c) == transp {
                        continue;
                    }
                    let c = c as usize * 3;
                    let cor = tabela.get(c..c + 3).ok_or("índice fora da paleta")?;
                    let d = ((y + n / fw) * w + x + n % fw) * 4;
                    tela[d..d + 3].copy_from_slice(cor);
                    tela[d + 3] = 255;
                }
                quadros.push(Quadro { rgba: tela.clone(), atraso_cs: atraso });
                match descarte {
                    2 => {
                        for yy in y..y + fh {
                            let a = (yy * w + x) * 4;
                            tela[a..a + fw * 4].fill(0);
                        }
                    }
                    3 => tela = antes.unwrap_or(tela),
                    _ => {}
                }
                (atraso, transp, descarte) = (0, None, 0);
            }
            _ => return Err("bloco de GIF desconhecido".into()),
        }
    }
    if quadros.is_empty() {
        return Err("GIF sem quadros".into());
    }
    Ok(Gif { largura: w, altura: h, quadros })
}

#[cfg(test)]
mod testes {
    use super::*;

    /// GIF mínimo de 2x2 com dois quadros, escrito à mão com o codificador "sem compressão" (um
    /// código limpar a cada pixel): cobre cabeçalho, controle gráfico, transparência e laço.
    fn gif_de_teste() -> Vec<u8> {
        let mut g = b"GIF89a".to_vec();
        g.extend([2, 0, 2, 0, 0x80 | 1, 0, 0]); // 2x2, tabela global de 4 cores
        g.extend([255, 0, 0, 0, 255, 0, 0, 0, 255, 9, 9, 9]);
        let quadro = |g: &mut Vec<u8>, idx: [u8; 4], transp: Option<u8>| {
            g.extend([0x21, 0xF9, 4, if transp.is_some() { 0b101 } else { 0b100 }, 50, 0, transp.unwrap_or(0), 0]);
            g.extend([0x2C, 0, 0, 0, 0, 2, 0, 2, 0, 0, 2]);
            // largura inicial 3 bits: limpar=4, fim=5; cada pixel entra como um código de 3 bits
            let mut bits: Vec<u8> = vec![];
            let mut push = |c: u8| (0..3).for_each(|k| bits.push((c >> k) & 1));
            push(4);
            for p in idx {
                push(p);
                push(4);
            }
            push(5);
            let bytes: Vec<u8> = bits.chunks(8).map(|c| c.iter().enumerate().fold(0u8, |a, (k, b)| a | (b << k))).collect();
            g.push(bytes.len() as u8);
            g.extend(bytes);
            g.push(0);
        };
        quadro(&mut g, [0, 1, 2, 3], None);
        quadro(&mut g, [1, 1, 1, 1], Some(1)); // tudo transparente: o quadro 2 repete o 1
        g.push(0x3B);
        g
    }

    #[test]
    fn le_quadros_cores_transparencia_e_laco() {
        let gif = ler(&gif_de_teste()).expect("gif");
        assert_eq!((gif.largura, gif.altura, gif.quadros.len()), (2, 2, 2));
        let q0 = &gif.quadros[0].rgba;
        assert_eq!(&q0[0..4], &[255, 0, 0, 255]);
        assert_eq!(&q0[4..8], &[0, 255, 0, 255]);
        assert_eq!(&q0[12..16], &[9, 9, 9, 255]);
        // o segundo quadro só tem pixels transparentes: herda o primeiro
        assert_eq!(gif.quadros[1].rgba, *q0);
        assert!((gif.duracao() - 1.0).abs() < 1e-9);
        assert_eq!((gif.quadro_em(0.0), gif.quadro_em(0.6), gif.quadro_em(1.2)), (0, 1, 0));
    }

    #[test]
    fn recusa_lixo_e_arquivo_cortado() {
        assert!(ler(b"nao e gif").is_err());
        let g = gif_de_teste();
        assert!(ler(&g[..g.len() / 2]).is_err());
    }
}
