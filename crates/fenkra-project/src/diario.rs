// SPDX-License-Identifier: LGPL-2.1-or-later
//! Diário só-acréscimo do projeto (B25, VPM fase 1).
//!
//! Por que um arquivo de linhas (`.diario.jsonl`) e não uma lista dentro do índice:
//! - o índice é regravado inteiro a cada salvamento; o diário **nunca** é regravado. Cada evento é uma
//!   linha nova no fim do arquivo (modo `append`), então nenhum bug de edição consegue reescrever o
//!   histórico, e uma queda no meio de uma linha estraga no máximo a última linha;
//! - a leitura tolera linhas ilegíveis (as pula e conta), porque um diário parcialmente lido vale mais
//!   do que um diário que se recusa a abrir;
//! - cada linha carrega autor, data e motivo: é a trilha de auditoria que a tabela de estados (B265) e
//!   a restauração de versão exigem.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const ARQUIVO_DIARIO: &str = ".diario.jsonl";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoEvento {
    /// mudança de estado do ciclo de vida (inclui as automáticas, marcadas no motivo)
    Transicao,
    /// versão nova criada ao salvar
    Versao,
    /// versão restaurada: uma versão nova que aponta para o conteúdo antigo
    Restauracao,
    /// revisão liberada (letra criada)
    Revisao,
    /// referência reparada ou arquivo movido
    Reparo,
    /// violação de regra de qualidade aceita (B305): `autor` é quem aceitou, `de` a regra, `para` a versão
    Excecao,
    /// ramo criado, salvo ou descartado (B130): `de` é o ramo, `para` a versão
    Ramo,
    /// pedido, revisão e entrada de um ramo na linha principal (B130)
    Mesclagem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evento {
    /// segundos desde 1970
    pub tempo: u64,
    pub autor: String,
    pub tipo: TipoEvento,
    pub doc_id: u64,
    #[serde(default)]
    pub uid: String,
    /// estado ou versão de origem (texto livre curto)
    #[serde(default)]
    pub de: String,
    /// estado ou versão de destino
    #[serde(default)]
    pub para: String,
    #[serde(default)]
    pub motivo: String,
}

pub fn caminho(raiz: &Path) -> PathBuf {
    raiz.join(ARQUIVO_DIARIO)
}

/// Acrescenta um evento ao fim do diário e força o disco. Nunca lê nem reescreve o que já existe.
pub fn acrescentar(raiz: &Path, e: &Evento) -> std::io::Result<()> {
    let mut linha = serde_json::to_string(e).map_err(std::io::Error::other)?;
    linha.push('\n');
    // depois de uma queda no meio de uma linha, o arquivo termina sem quebra: o evento novo começa
    // numa linha própria em vez de colar na linha quebrada
    if std::fs::read(caminho(raiz)).map(|b| !b.is_empty() && b.last() != Some(&b'\n')).unwrap_or(false) {
        linha.insert(0, '\n');
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(caminho(raiz))?;
    // uma única escrita por evento: duas escritas poderiam intercalar com outro processo
    f.write_all(linha.as_bytes())?;
    f.sync_data()
}

/// Eventos legíveis do diário, na ordem em que foram gravados, e quantas linhas foram puladas.
pub fn ler(raiz: &Path) -> (Vec<Evento>, usize) {
    let Ok(s) = std::fs::read_to_string(caminho(raiz)) else { return (vec![], 0) };
    let mut v = vec![];
    let mut puladas = 0;
    for l in s.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<Evento>(l) {
            Ok(e) => v.push(e),
            Err(_) => puladas += 1,
        }
    }
    (v, puladas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-diario-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn ev(n: u64, motivo: &str) -> Evento {
        Evento { tempo: n, autor: "ana".into(), tipo: TipoEvento::Transicao, doc_id: 1, uid: "u".into(), de: "A".into(), para: "B".into(), motivo: motivo.into() }
    }

    #[test]
    fn acrescentar_nunca_reescreve_o_que_ja_existe() {
        let d = tmp("append");
        acrescentar(&d, &ev(1, "primeiro")).unwrap();
        let antes = std::fs::read_to_string(caminho(&d)).unwrap();
        acrescentar(&d, &ev(2, "segundo")).unwrap();
        let depois = std::fs::read_to_string(caminho(&d)).unwrap();
        assert!(depois.starts_with(&antes), "o conteúdo anterior fica intacto byte a byte");
        let (v, puladas) = ler(&d);
        assert_eq!((v.len(), puladas), (2, 0));
        assert_eq!(v[1].motivo, "segundo");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn linha_truncada_por_queda_nao_impede_ler_o_resto() {
        let d = tmp("truncada");
        acrescentar(&d, &ev(1, "ok")).unwrap();
        // simula a queda no meio da escrita de uma linha
        let mut f = std::fs::OpenOptions::new().append(true).open(caminho(&d)).unwrap();
        f.write_all(b"{\"tempo\":2,\"autor\":\"an").unwrap();
        drop(f);
        let (v, puladas) = ler(&d);
        assert_eq!(v.len(), 1);
        assert_eq!(puladas, 1);
        acrescentar(&d, &ev(3, "depois da queda")).unwrap();
        let (v, puladas) = ler(&d);
        assert_eq!((v.len(), puladas), (2, 1), "o evento novo não cola na linha quebrada");
        assert_eq!(v[1].motivo, "depois da queda");
        let _ = std::fs::remove_dir_all(d);
    }
}
