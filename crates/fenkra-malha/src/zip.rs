// SPDX-License-Identifier: LGPL-2.1-or-later
//! ZIP mínimo: 3MF, AMF comprimido e USDZ são pacotes. Só o que eles usam: guardado ou deflate, sem
//! criptografia nem ZIP64. A leitura tem teto de tamanho, porque um pacote pequeno pode descompactar
//! em gigabytes.

use crate::Erro as ErroConv;

/// Maior arquivo descompactado que se aceita de um pacote.
const LIMITE: usize = 2 << 30;
const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const FIM: u32 = 0x0605_4b50;

fn crc32(dados: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in dados {
        c ^= u32::from(*b);
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

pub fn e_zip(b: &[u8]) -> bool {
    b.len() > 4 && b[..4] == LOCAL.to_le_bytes()
}

fn u16_em(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32_em(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// Uma entrada do diretório central.
struct Entrada {
    nome: String,
    metodo: usize,
    crc: usize,
    comprimido: usize,
    tamanho: usize,
    local: usize,
}

fn diretorio(b: &[u8]) -> Option<Vec<Entrada>> {
    let fim = (0..=b.len().checked_sub(22)?).rev().take(70_000).find(|i| b[*i..*i + 4] == FIM.to_le_bytes())?;
    let quantas = u16_em(b, fim + 10)?;
    let mut i = u32_em(b, fim + 16)?;
    let mut v = Vec::new();
    for _ in 0..quantas {
        if u32_em(b, i)? != CENTRAL as usize {
            return None;
        }
        let (n, extra, comentario) = (u16_em(b, i + 28)?, u16_em(b, i + 30)?, u16_em(b, i + 32)?);
        let nome = String::from_utf8_lossy(b.get(i + 46..i + 46 + n)?).to_string();
        v.push(Entrada { nome, metodo: u16_em(b, i + 10)?, crc: u32_em(b, i + 16)?, comprimido: u32_em(b, i + 20)?, tamanho: u32_em(b, i + 24)?, local: u32_em(b, i + 42)? });
        i += 46 + n + extra + comentario;
    }
    Some(v)
}

/// Nomes das entradas, para reconhecer o pacote sem abrir nada.
pub fn nomes(b: &[u8]) -> Vec<String> {
    diretorio(b).unwrap_or_default().into_iter().map(|e| e.nome).collect()
}

/// Conteúdo da primeira entrada que atende `filtro`.
pub fn abrir(b: &[u8], filtro: impl Fn(&str) -> bool) -> Result<Option<Vec<u8>>, ErroConv> {
    let ruim = |m: &str| ErroConv::Leitura(format!("pacote ZIP: {m}"));
    let lista = diretorio(b).ok_or_else(|| ruim("diretório ilegível"))?;
    let Some(e) = lista.iter().find(|e| filtro(&e.nome)) else { return Ok(None) };
    if e.tamanho > LIMITE {
        return Err(ruim("arquivo grande demais"));
    }
    let (Some(n), Some(extra)) = (u16_em(b, e.local + 26), u16_em(b, e.local + 28)) else { return Err(ruim("entrada cortada")) };
    let inicio = e.local + 30 + n + extra;
    let dados = b.get(inicio..inicio + e.comprimido).ok_or_else(|| ruim("entrada cortada"))?;
    let saida = match e.metodo {
        0 => dados.to_vec(),
        8 => miniz_oxide::inflate::decompress_to_vec_with_limit(dados, e.tamanho.max(1)).map_err(|_| ruim("deflate inválido"))?,
        m => return Err(ruim(&format!("compressão {m} não suportada"))),
    };
    if crc32(&saida) as usize != e.crc {
        return Err(ruim("soma de verificação não confere"));
    }
    Ok(Some(saida))
}

/// Grava entradas em ordem. `alinhar` põe o início de cada dado num múltiplo de 64 bytes e não
/// comprime: é a regra do USDZ, feito para ser lido direto da memória.
pub fn gravar(entradas: &[(&str, &[u8])], alinhar: bool) -> Result<Vec<u8>, ErroConv> {
    let mut saida = Vec::new();
    let mut central = Vec::new();
    for (nome, dados) in entradas {
        let crc = crc32(dados);
        let comprimido = if alinhar { None } else { Some(miniz_oxide::deflate::compress_to_vec(dados, 6)).filter(|c| c.len() < dados.len()) };
        let (metodo, corpo): (u16, &[u8]) = comprimido.as_ref().map(|c| (8, c.as_slice())).unwrap_or((0, dados));
        let (Ok(tamanho), Ok(reduzido)) = (u32::try_from(dados.len()), u32::try_from(corpo.len())) else {
            return Err(ErroConv::Gravacao("pacote maior que 4 GB".into()));
        };
        let local = saida.len() as u32;
        let falta = if alinhar { (64 - (saida.len() + 30 + nome.len()) % 64) % 64 } else { 0 };
        // o campo extra precisa de 4 bytes de cabeçalho: com menos, pula para o próximo múltiplo
        let extra = if falta != 0 && falta < 4 { falta + 64 } else { falta };
        let comum = |v: &mut Vec<u8>| {
            for x in [20u16, 0x0800, metodo, 0, 0x21] {
                v.extend_from_slice(&x.to_le_bytes());
            }
            for x in [crc, reduzido, tamanho] {
                v.extend_from_slice(&x.to_le_bytes());
            }
            v.extend_from_slice(&(nome.len() as u16).to_le_bytes());
        };
        saida.extend_from_slice(&LOCAL.to_le_bytes());
        comum(&mut saida);
        saida.extend_from_slice(&(extra as u16).to_le_bytes());
        saida.extend_from_slice(nome.as_bytes());
        if extra >= 4 {
            saida.extend_from_slice(&0x1986u16.to_le_bytes());
            saida.extend_from_slice(&((extra - 4) as u16).to_le_bytes());
            saida.resize(saida.len() + extra - 4, 0);
        }
        saida.extend_from_slice(corpo);

        central.extend_from_slice(&CENTRAL.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        comum(&mut central);
        // extra, comentário, disco, atributos internos e externos
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&local.to_le_bytes());
        central.extend_from_slice(nome.as_bytes());
    }
    let inicio = saida.len() as u32;
    let tamanho = central.len() as u32;
    saida.append(&mut central);
    saida.extend_from_slice(&FIM.to_le_bytes());
    saida.extend_from_slice(&[0; 4]);
    saida.extend_from_slice(&(entradas.len() as u16).to_le_bytes());
    saida.extend_from_slice(&(entradas.len() as u16).to_le_bytes());
    saida.extend_from_slice(&tamanho.to_le_bytes());
    saida.extend_from_slice(&inicio.to_le_bytes());
    saida.extend_from_slice(&[0; 2]);
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn crc_do_vetor_conhecido() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn ida_e_volta_com_compressao_e_acento() {
        let longo = vec![b'a'; 100_000];
        let z = gravar(&[("3D/peça.model", &longo), ("curto.txt", b"oi")], false).unwrap();
        assert!(e_zip(&z) && z.len() < 2000, "{}", z.len());
        assert_eq!(nomes(&z), vec!["3D/peça.model", "curto.txt"]);
        assert_eq!(abrir(&z, |n| n.ends_with(".model")).unwrap().unwrap(), longo);
        assert_eq!(abrir(&z, |n| n == "curto.txt").unwrap().unwrap(), b"oi");
        assert_eq!(abrir(&z, |n| n == "nada").unwrap(), None);
    }

    #[test]
    fn alinhado_comeca_em_multiplo_de_64() {
        let z = gravar(&[("a.usda", b"um"), ("nome-maior.png", b"dois")], true).unwrap();
        for dado in [b"um".as_slice(), b"dois".as_slice()] {
            let p = z.windows(dado.len()).position(|w| w == dado).unwrap();
            assert_eq!(p % 64, 0, "{dado:?}");
        }
        assert_eq!(abrir(&z, |n| n == "a.usda").unwrap().unwrap(), b"um");
    }

    #[test]
    fn dado_trocado_e_recusado_pela_soma() {
        let mut z = gravar(&[("a.txt", b"conteudo")], true).unwrap();
        let p = z.windows(8).position(|w| w == b"conteudo").unwrap();
        z[p] = b'X';
        assert!(abrir(&z, |_| true).is_err());
    }

    #[test]
    fn lixo_nao_derruba() {
        assert!(nomes(b"PK\x03\x04lixo").is_empty());
        assert!(abrir(b"PK\x03\x04lixo", |_| true).is_err());
    }
}
