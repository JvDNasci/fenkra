// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pacote externo de mídia de ajuda (`ajuda.pack`) e cache LRU (B257).
//!
//! O pacote fica ao lado do executável e não entra nele: o binário não cresce. Só o índice é lido ao
//! abrir; cada GIF é lido do disco na primeira vez que o botão é pairado e fica no cache (16
//! mídias, as mais antigas saem primeiro).
//!
//! Formato (escrito por `scripts/media/montar.py`):
//! `FKAJ` + u32 (little endian) com o tamanho do índice + índice em JSON + dados. O índice é
//! `{"versao":1,"entradas":[{"id","offset","len","hash"}]}`, com `offset` contado desde o início dos
//! dados e `hash` igual ao `hash_gif` do manifesto.

use serde::Deserialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Mídias guardadas ao mesmo tempo.
pub const CAPACIDADE: usize = 16;

#[derive(Clone, Debug, Deserialize)]
pub struct EntradaPacote {
    pub id: String,
    pub offset: u64,
    pub len: u64,
    pub hash: String,
}

#[derive(Deserialize)]
struct Indice {
    versao: u32,
    entradas: Vec<EntradaPacote>,
}

pub struct Pacote {
    caminho: PathBuf,
    entradas: Vec<EntradaPacote>,
    base: u64,
}

impl Pacote {
    pub fn abrir(caminho: &Path) -> Result<Pacote, String> {
        let mut f = std::fs::File::open(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        let mut cab = [0u8; 8];
        f.read_exact(&mut cab).map_err(|e| format!("pacote curto: {e}"))?;
        if &cab[0..4] != b"FKAJ" {
            return Err("não é um pacote de ajuda".into());
        }
        let n = u32::from_le_bytes([cab[4], cab[5], cab[6], cab[7]]) as usize;
        if n > 1 << 20 {
            return Err("índice do pacote grande demais".into());
        }
        let mut json = vec![0u8; n];
        f.read_exact(&mut json).map_err(|e| format!("índice cortado: {e}"))?;
        let idx: Indice = serde_json::from_slice(&json).map_err(|e| format!("índice inválido: {e}"))?;
        if idx.versao != 1 {
            return Err(format!("pacote de versão {} não suportado", idx.versao));
        }
        Ok(Pacote { caminho: caminho.to_owned(), entradas: idx.entradas, base: 8 + n as u64 })
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entradas.iter().map(|e| e.id.as_str())
    }

    pub fn tem(&self, id: &str) -> bool {
        self.entradas.iter().any(|e| e.id == id)
    }

    /// Lê os bytes do GIF e confere o hash gravado no índice.
    pub fn ler(&self, id: &str) -> Result<Vec<u8>, String> {
        let e = self.entradas.iter().find(|e| e.id == id).ok_or_else(|| format!("{id} não está no pacote"))?;
        let mut f = std::fs::File::open(&self.caminho).map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(self.base + e.offset)).map_err(|e| e.to_string())?;
        let mut b = vec![0u8; e.len as usize];
        f.read_exact(&mut b).map_err(|e| format!("mídia {id} cortada: {e}"))?;
        if super::hash(&b) != e.hash {
            return Err(format!("mídia {id} não confere com o índice"));
        }
        Ok(b)
    }
}

/// Cache por uso recente: o último acessado fica no fim; ao passar da capacidade sai o do começo.
pub struct Lru<V> {
    cap: usize,
    itens: Vec<(String, Arc<V>)>,
}

impl<V> Lru<V> {
    pub fn novo(cap: usize) -> Self {
        Lru { cap: cap.max(1), itens: vec![] }
    }

    pub fn obter(&mut self, id: &str) -> Option<Arc<V>> {
        let i = self.itens.iter().position(|(k, _)| k == id)?;
        let par = self.itens.remove(i);
        let v = par.1.clone();
        self.itens.push(par);
        Some(v)
    }

    pub fn por(&mut self, id: &str, v: V) -> Arc<V> {
        let v = Arc::new(v);
        self.itens.retain(|(k, _)| k != id);
        self.itens.push((id.to_owned(), v.clone()));
        while self.itens.len() > self.cap {
            self.itens.remove(0);
        }
        v
    }

    pub fn len(&self) -> usize {
        self.itens.len()
    }

    pub fn ids(&self) -> Vec<&str> {
        self.itens.iter().map(|(k, _)| k.as_str()).collect()
    }
}

/// Onde procurar o pacote: variável `FENKRA_AJUDA_PACK`, ao lado do executável e, só em
/// desenvolvimento, `docs/media/ajuda.pack` do repositório.
pub fn caminho_padrao() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = vec![];
    if let Ok(p) = std::env::var("FENKRA_AJUDA_PACK") {
        cands.push(p.into());
    }
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_owned)) {
        cands.push(d.join("ajuda.pack"));
    }
    #[cfg(debug_assertions)]
    cands.push(super::pasta_docs().join("ajuda.pack"));
    cands.into_iter().find(|p| p.is_file())
}

/// Pacote aberto e cache dos bytes lidos. Falha silenciosa: sem pacote, a dica fica só em texto.
pub struct Midias {
    pacote: Option<Pacote>,
    cache: Lru<Vec<u8>>,
}

impl Midias {
    pub fn abrir(caminho: Option<&Path>) -> Midias {
        let pacote = caminho.and_then(|c| Pacote::abrir(c).ok());
        Midias { pacote, cache: Lru::novo(CAPACIDADE) }
    }

    pub fn tem_pacote(&self) -> bool {
        self.pacote.is_some()
    }

    pub fn tem(&self, id: &str) -> bool {
        self.pacote.as_ref().is_some_and(|p| p.tem(id))
    }

    /// Bytes do GIF do comando, do cache ou do disco.
    pub fn bytes(&mut self, id: &str) -> Option<Arc<Vec<u8>>> {
        if let Some(b) = self.cache.obter(id) {
            return Some(b);
        }
        let b = self.pacote.as_ref()?.ler(id).ok()?;
        Some(self.cache.por(id, b))
    }

    pub fn em_cache(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Escreve um pacote pequeno no formato do script.
    pub fn escrever(caminho: &Path, midias: &[(&str, &[u8])]) {
        let mut off = 0u64;
        let mut ents = vec![];
        for (id, b) in midias {
            ents.push(format!(r#"{{"id":"{id}","offset":{off},"len":{},"hash":"{}"}}"#, b.len(), super::super::hash(b)));
            off += b.len() as u64;
        }
        let json = format!(r#"{{"versao":1,"entradas":[{}]}}"#, ents.join(","));
        let mut dados = b"FKAJ".to_vec();
        dados.extend((json.len() as u32).to_le_bytes());
        dados.extend(json.as_bytes());
        for (_, b) in midias {
            dados.extend(*b);
        }
        std::fs::write(caminho, dados).unwrap();
    }

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_midia_{nome}_{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn le_cada_midia_do_pacote_e_confere_o_hash() {
        let d = pasta("le");
        let arq = d.join("a.pack");
        escrever(&arq, &[("casca", b"GIF-casca"), ("rosca", b"GIF-rosca-maior")]);
        let p = Pacote::abrir(&arq).unwrap();
        assert_eq!(p.ids().collect::<Vec<_>>(), ["casca", "rosca"]);
        assert_eq!(p.ler("rosca").unwrap(), b"GIF-rosca-maior");
        assert!(p.ler("nada").is_err());
        // um byte estragado no disco é pego pelo hash
        let mut b = std::fs::read(&arq).unwrap();
        let n = b.len();
        b[n - 1] ^= 1;
        std::fs::write(&arq, b).unwrap();
        assert!(p.ler("rosca").unwrap_err().contains("não confere"));
        assert!(Pacote::abrir(&d.join("nao_existe.pack")).is_err());
        std::fs::write(d.join("lixo.pack"), b"XXXXXXXXXXXX").unwrap();
        assert!(Pacote::abrir(&d.join("lixo.pack")).is_err());
    }

    #[test]
    fn lru_tira_o_menos_recente_e_reordena_no_acesso() {
        let mut c: Lru<u32> = Lru::novo(3);
        for (k, v) in [("a", 1), ("b", 2), ("c", 3)] {
            c.por(k, v);
        }
        assert_eq!(c.obter("a").as_deref(), Some(&1)); // a fica como a mais recente
        c.por("d", 4); // estoura: sai o b, o mais antigo
        assert_eq!(c.ids(), ["c", "a", "d"]);
        assert!(c.obter("b").is_none());
        c.por("a", 10); // regravar não duplica
        assert_eq!(c.len(), 3);
        assert_eq!(c.obter("a").as_deref(), Some(&10));
    }

    #[test]
    fn midias_so_le_do_disco_uma_vez_e_respeita_a_capacidade() {
        let d = pasta("midias");
        let arq = d.join("m.pack");
        let nomes: Vec<String> = (0..CAPACIDADE + 4).map(|i| format!("c{i}")).collect();
        let corpos: Vec<Vec<u8>> = (0..nomes.len()).map(|i| vec![i as u8; 10]).collect();
        let pares: Vec<(&str, &[u8])> = nomes.iter().zip(&corpos).map(|(n, c)| (n.as_str(), c.as_slice())).collect();
        escrever(&arq, &pares);
        let mut m = Midias::abrir(Some(&arq));
        assert!(m.tem_pacote() && m.tem("c0") && !m.tem("zz"));
        for n in &nomes {
            assert!(m.bytes(n).is_some());
        }
        assert_eq!(m.em_cache(), CAPACIDADE);
        // apagado o arquivo, o que está no cache continua servindo e o que saiu não volta
        std::fs::remove_file(&arq).unwrap();
        assert!(m.bytes(nomes.last().unwrap()).is_some());
        assert!(m.bytes("c0").is_none());
        assert!(!Midias::abrir(None).tem_pacote());
    }
}
