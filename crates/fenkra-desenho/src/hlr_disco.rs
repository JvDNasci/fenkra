// SPDX-License-Identifier: LGPL-2.1-or-later
//! B393: linhas ocultas exatas das vistas de montagem guardadas em disco.
//!
//! A projeção exata do kart leva de 40 s a mais de um minuto, mesmo em processos paralelos (ADR 0080).
//! Reabrir o desenho sem mudar nada não precisa refazê-la: as linhas vão para um arquivo cuja chave é
//! a vista (tipo, direção, escala, corte, mãe), o conteúdo de cada arquivo lido para montá-la e a
//! identidade do programa. Qualquer mudança em peça, montagem, vista ou programa muda a chave.
//!
//! Fica ao lado das formas guardadas (`fenkra_montagem::formas_disco::pasta`), com a mesma chave de
//! desligar (`FENKRA_SEM_CACHE_FORMAS=1`).

use crate::fundo::Arquivos;
use crate::source::Proj;
use crate::{View, V2};
use fenkra_montagem::formas_disco::{desligado, fnv, identidade_do_programa, limpar_uma_vez, pasta};
use std::path::PathBuf;

pub type Linhas = Vec<(Vec<V2>, u8)>;

/// Chave da vista: programa, vista (com a mãe) e o conteúdo de cada arquivo de que ela depende.
/// `None` se algum arquivo não pôde ser lido.
pub fn chave(v: &View, mae: Option<&Proj>, arquivos: &Arquivos) -> Option<String> {
    let mut vista = std::collections::hash_map::DefaultHasher::new();
    crate::fundo::chave_da_vista(v, mae, &mut vista);
    let vista = std::hash::Hasher::finish(&vista);
    let mut h = 0xcbf29ce484222325u64;
    fnv(&mut h, identidade_do_programa().as_bytes());
    fnv(&mut h, &vista.to_le_bytes());
    let mut g = 0x84222325cbf29ce4u64;
    let mut lista: Vec<&PathBuf> = arquivos.iter().map(|(p, _)| p).collect();
    lista.sort();
    lista.dedup();
    for p in lista {
        let dados = std::fs::read(p).ok()?;
        fnv(&mut h, p.to_string_lossy().as_bytes());
        fnv(&mut h, &dados);
        fnv(&mut g, &dados);
    }
    fnv(&mut g, &vista.to_le_bytes());
    Some(format!("{h:016x}{g:016x}"))
}

fn arquivo(chave: &str) -> PathBuf {
    pasta().join(format!("{chave}.fkl"))
}

/// Codifica: quantidade de linhas, e por linha o tipo, a quantidade de pontos e os pontos (f64 LE).
pub fn codificar(l: &Linhas) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + l.iter().map(|(p, _)| 5 + p.len() * 16).sum::<usize>());
    out.extend_from_slice(b"FKL1");
    out.extend_from_slice(&(l.len() as u32).to_le_bytes());
    for (pts, k) in l {
        out.push(*k);
        out.extend_from_slice(&(pts.len() as u32).to_le_bytes());
        for p in pts {
            out.extend_from_slice(&p[0].to_le_bytes());
            out.extend_from_slice(&p[1].to_le_bytes());
        }
    }
    out
}

pub fn decodificar(d: &[u8]) -> Option<Linhas> {
    let mut i = 0usize;
    let mut pega = |n: usize| -> Option<&[u8]> {
        let s = d.get(i..i + n)?;
        i += n;
        Some(s)
    };
    if pega(4)? != b"FKL1" {
        return None;
    }
    let u32_ = |s: &[u8]| u32::from_le_bytes(s.try_into().unwrap()) as usize;
    let n = u32_(pega(4)?);
    let mut out = Vec::with_capacity(n.min(1 << 20));
    for _ in 0..n {
        let k = pega(1)?[0];
        let m = u32_(pega(4)?);
        let mut pts = Vec::with_capacity(m.min(1 << 20));
        for _ in 0..m {
            let x = f64::from_le_bytes(pega(8)?.try_into().ok()?);
            let y = f64::from_le_bytes(pega(8)?.try_into().ok()?);
            pts.push([x, y]);
        }
        out.push((pts, k));
    }
    (i == d.len()).then_some(out)
}

pub fn ler(chave: &str) -> Option<Linhas> {
    if desligado() {
        return None;
    }
    decodificar(&std::fs::read(arquivo(chave)).ok()?)
}

pub fn gravar(chave: &str, l: &Linhas) {
    if desligado() || l.is_empty() {
        return;
    }
    let dir = pasta();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    limpar_uma_vez(&dir);
    let tmp = dir.join(format!("{chave}.{}.tmp", std::process::id()));
    if std::fs::write(&tmp, codificar(l)).is_ok() && std::fs::rename(&tmp, arquivo(chave)).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codifica_e_decodifica_as_linhas() {
        let l: Linhas = vec![(vec![[0., 1.], [2.5, -3.]], 0), (vec![], 3), (vec![[1e9, -1e-9]], 7)];
        assert_eq!(decodificar(&codificar(&l)).unwrap(), l);
        let mut d = codificar(&l);
        d.pop();
        assert!(decodificar(&d).is_none(), "arquivo cortado não vale");
        assert!(decodificar(b"XXXX").is_none());
    }
}
