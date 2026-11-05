// SPDX-License-Identifier: LGPL-2.1-or-later
//! Empacotar: reúne um documento e tudo de que ele depende em uma pasta ou em um `.zip` (B93).
//!
//! Para que serve: mandar um conjunto para outra pessoa sem que ela precise reconstruir a árvore de
//! pastas do autor. O pacote é **autossuficiente**: todo caminho gravado nos documentos copiados é
//! reescrito para o lugar novo, e o `manifesto.json` diz de onde cada arquivo veio.
//!
//! Escolhas e por quê:
//! - `Hierarquica` preserva a posição relativa dos arquivos (as referências continuam válidas sem
//!   reescrita); `Plana` põe tudo numa pasta só, o que exige renomear colisões. Cada colisão é
//!   renomeada (`nome_2.fkp`) e **listada** no manifesto: nada é sobrescrito em silêncio.
//! - "Linkados" são os documentos citados por links entre peças (par uid + caminho); "derivados" são
//!   os documentos que carregam o selo de origem (nascidos de outro arquivo); "bibliotecas" são
//!   arquivos sob as pastas de biblioteca informadas. Cada categoria pode ficar de fora; o que ficou
//!   de fora é listado em `excluidas`, e a referência a ele vira caminho absoluto (continua achando o
//!   original na máquina de origem) e entra em `externas`.
//! - O `.zip` é gravado sem compressão (método "armazenado"): os documentos já são texto pequeno,
//!   e assim não se depende de biblioteca externa. O leitor aceita só o que o próprio escritor gera.
//! - Ao final o pacote é **reaberto** de verdade ([`verificar`]): relê o manifesto, confere tamanho e
//!   hash de cada arquivo, o uid de cada documento e se cada referência cai dentro do pacote.

use crate::relacoes::{onde_usado, referencias_do_json, trocar_caminhos, TipoRef};
use crate::seguro::gravar_atomico;
use crate::{PResult, ProjectError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

pub const NOME_MANIFESTO: &str = "manifesto.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estrutura {
    Plana,
    Hierarquica,
}

#[derive(Clone, Debug)]
pub struct OpcoesPacote {
    pub estrutura: Estrutura,
    /// inclui os desenhos que usam algum arquivo do pacote
    pub incluir_desenhos: bool,
    pub incluir_linkados: bool,
    pub incluir_derivados: bool,
    pub pular_bibliotecas: bool,
    pub pastas_biblioteca: Vec<PathBuf>,
    /// onde procurar desenhos que usam o conjunto
    pub pastas_busca: Vec<PathBuf>,
}

impl Default for OpcoesPacote {
    fn default() -> Self {
        OpcoesPacote {
            estrutura: Estrutura::Hierarquica,
            incluir_desenhos: true,
            incluir_linkados: true,
            incluir_derivados: true,
            pular_bibliotecas: false,
            pastas_biblioteca: vec![],
            pastas_busca: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrada {
    /// `raiz`, `referenciado`, `desenho`, `linkado`
    pub papel: String,
    pub original: String,
    pub pacote: String,
    pub uid: String,
    pub hash: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Colisao {
    pub original: String,
    pub pedido: String,
    pub gravado: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Excluida {
    pub original: String,
    pub motivo: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Externa {
    pub documento: String,
    pub caminho: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifesto {
    pub versao: u32,
    pub app_version: String,
    pub raiz: String,
    pub estrutura: String,
    pub entradas: Vec<Entrada>,
    pub colisoes: Vec<Colisao>,
    pub excluidas: Vec<Excluida>,
    pub externas: Vec<Externa>,
    /// referenciados que não existem no disco de origem
    pub ausentes: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verificacao {
    pub faltando: Vec<String>,
    pub hash_ou_tamanho_diferente: Vec<String>,
    pub uid_diferente: Vec<String>,
    pub ilegiveis: Vec<String>,
    /// (documento, caminho) de referências que não caem dentro do pacote nem estão declaradas externas
    pub refs_quebradas: Vec<(String, String)>,
    pub nao_listados: Vec<String>,
}

impl Verificacao {
    pub fn ok(&self) -> bool {
        self.faltando.is_empty()
            && self.hash_ou_tamanho_diferente.is_empty()
            && self.uid_diferente.is_empty()
            && self.ilegiveis.is_empty()
            && self.refs_quebradas.is_empty()
            && self.nao_listados.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct ResultadoPacote {
    pub manifesto: Manifesto,
    pub verificacao: Verificacao,
}

// ---------------------------------------------------------------------------------- caminhos

fn absoluto(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Caminho absoluto sem `.` e `..`, só lexical (não exige que exista).
fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in absoluto(p).components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c.as_os_str()),
        }
    }
    out
}

fn chave(p: &Path) -> String {
    let b = std::fs::canonicalize(p).unwrap_or_else(|_| lexical(p));
    let t = b.to_string_lossy().replace('\\', "/");
    let t = t.strip_prefix("//?/").unwrap_or(&t).to_string();
    if cfg!(windows) { t.to_lowercase() } else { t }
}

fn texto(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Caminho relativo entre dois caminhos "virtuais" do pacote (separados por `/`, sem unidade).
fn relativo_virtual(de_dir: &str, para: &str) -> String {
    let a: Vec<&str> = de_dir.split('/').filter(|s| !s.is_empty()).collect();
    let b: Vec<&str> = para.split('/').filter(|s| !s.is_empty()).collect();
    let mut i = 0;
    while i < a.len() && i < b.len().saturating_sub(1) && a[i] == b[i] {
        i += 1;
    }
    let mut partes: Vec<&str> = vec![".."; a.len() - i];
    partes.extend(&b[i..]);
    partes.join("/")
}

/// Junta `dir` e `rel` (ambos virtuais) resolvendo `..`; `None` se sair da raiz do pacote.
fn juntar_virtual(dir: &str, rel: &str) -> Option<String> {
    if rel.starts_with('/') || rel.contains(':') {
        return None;
    }
    let mut partes: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for s in rel.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                partes.pop()?;
            }
            x => partes.push(x),
        }
    }
    Some(partes.join("/"))
}

fn nome_com_contador(nome: &str, n: u32) -> String {
    let p = Path::new(nome);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match p.extension() {
        Some(e) => format!("{stem}_{n}.{}", e.to_string_lossy()),
        None => format!("{stem}_{n}"),
    }
}

// ---------------------------------------------------------------------------------- zip mínimo

fn crc32(dados: &[u8]) -> u32 {
    let mut tabela = [0u32; 256];
    for (i, t) in tabela.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    !dados.iter().fold(!0u32, |c, b| tabela[((c ^ *b as u32) & 0xFF) as usize] ^ (c >> 8))
}

fn u16le(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn u32le(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// Zip sem compressão. Nomes em UTF-8 (bit 11 dos flags). Sem zip64: até 65535 arquivos de até 4 GiB.
pub fn escrever_zip(arquivos: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    if arquivos.len() > 65_535 {
        return Err("arquivos demais para um .zip simples".into());
    }
    let mut saida: Vec<u8> = vec![];
    let mut central: Vec<u8> = vec![];
    for (nome, dados) in arquivos {
        if dados.len() as u64 >= u32::MAX as u64 || saida.len() as u64 >= u32::MAX as u64 {
            return Err("pacote grande demais para um .zip simples".into());
        }
        let crc = crc32(dados);
        let deslocamento = saida.len() as u32;
        u32le(&mut saida, 0x0403_4b50);
        u16le(&mut saida, 20);
        u16le(&mut saida, 0x0800);
        u16le(&mut saida, 0); // armazenado
        u16le(&mut saida, 0); // hora
        u16le(&mut saida, 0x0021); // 1980-01-01
        u32le(&mut saida, crc);
        u32le(&mut saida, dados.len() as u32);
        u32le(&mut saida, dados.len() as u32);
        u16le(&mut saida, nome.len() as u16);
        u16le(&mut saida, 0);
        saida.extend_from_slice(nome.as_bytes());
        saida.extend_from_slice(dados);

        u32le(&mut central, 0x0201_4b50);
        u16le(&mut central, 20);
        u16le(&mut central, 20);
        u16le(&mut central, 0x0800);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u16le(&mut central, 0x0021);
        u32le(&mut central, crc);
        u32le(&mut central, dados.len() as u32);
        u32le(&mut central, dados.len() as u32);
        u16le(&mut central, nome.len() as u16);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u32le(&mut central, 0);
        u32le(&mut central, deslocamento);
        central.extend_from_slice(nome.as_bytes());
    }
    let inicio_central = saida.len() as u32;
    let tam_central = central.len() as u32;
    saida.extend_from_slice(&central);
    u32le(&mut saida, 0x0605_4b50);
    u16le(&mut saida, 0);
    u16le(&mut saida, 0);
    u16le(&mut saida, arquivos.len() as u16);
    u16le(&mut saida, arquivos.len() as u16);
    u32le(&mut saida, tam_central);
    u32le(&mut saida, inicio_central);
    u16le(&mut saida, 0);
    Ok(saida)
}

fn le16(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}
fn le32(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// Lê um zip gerado por [`escrever_zip`] (só método armazenado) conferindo o CRC de cada arquivo.
/// Recusa nomes que escapam da pasta (`..`, caminho absoluto).
pub fn ler_zip(b: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let ruim = || "zip inválido ou truncado".to_string();
    let inicio_busca = b.len().saturating_sub(22 + 65_535);
    let eocd = (inicio_busca..=b.len().checked_sub(22).ok_or_else(ruim)?).rev().find(|&i| le32(b, i) == Some(0x0605_4b50)).ok_or_else(ruim)?;
    let n = le16(b, eocd + 10).ok_or_else(ruim)?;
    let mut p = le32(b, eocd + 16).ok_or_else(ruim)?;
    let mut saida = vec![];
    for _ in 0..n {
        if le32(b, p) != Some(0x0201_4b50) {
            return Err(ruim());
        }
        let metodo = le16(b, p + 10).ok_or_else(ruim)?;
        let crc = le32(b, p + 16).ok_or_else(ruim)?;
        let tam = le32(b, p + 20).ok_or_else(ruim)?;
        let nlen = le16(b, p + 28).ok_or_else(ruim)?;
        let elen = le16(b, p + 30).ok_or_else(ruim)?;
        let clen = le16(b, p + 32).ok_or_else(ruim)?;
        let off = le32(b, p + 42).ok_or_else(ruim)?;
        let nome = String::from_utf8(b.get(p + 46..p + 46 + nlen).ok_or_else(ruim)?.to_vec()).map_err(|_| ruim())?;
        p += 46 + nlen + elen + clen;
        if metodo != 0 {
            return Err("o pacote usa compressão, que este leitor não suporta".into());
        }
        if nome.starts_with('/') || nome.contains("..") || nome.contains(':') || nome.contains('\\') {
            return Err(format!("nome inseguro no pacote: {nome}"));
        }
        if le32(b, off) != Some(0x0403_4b50) {
            return Err(ruim());
        }
        let ln = le16(b, off + 26).ok_or_else(ruim)?;
        let le = le16(b, off + 28).ok_or_else(ruim)?;
        let ini = off + 30 + ln + le;
        let dados = b.get(ini..ini + tam).ok_or_else(ruim)?.to_vec();
        if crc32(&dados) as usize != crc {
            return Err(format!("CRC diferente em {nome}"));
        }
        if !nome.ends_with('/') {
            saida.push((nome, dados));
        }
    }
    Ok(saida)
}

// ---------------------------------------------------------------------------------- empacotar

struct Item {
    original: PathBuf,
    papel: &'static str,
}

fn tem_origem(texto: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(texto).map(|v| v.get("origem").map(|o| !o.is_null()).unwrap_or(false)).unwrap_or(false)
}

fn eh_desenho(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("fkd")
}

fn na_biblioteca(p: &Path, pastas: &[PathBuf]) -> bool {
    let k = chave(p);
    pastas.iter().any(|b| {
        let kb = chave(b);
        k.starts_with(&format!("{}/", kb.trim_end_matches('/')))
    })
}

/// Reúne os arquivos do pacote (fecho das referências + desenhos) e o que ficou de fora.
fn reunir(raiz: &Path, o: &OpcoesPacote, m: &mut Manifesto) -> PResult<Vec<Item>> {
    let mut itens: Vec<Item> = vec![];
    let mut vistos: HashSet<String> = HashSet::new();
    let mut fila: Vec<usize> = vec![];
    let r = absoluto(raiz);
    vistos.insert(chave(&r));
    itens.push(Item { original: r, papel: "raiz" });
    fila.push(0);
    let mut desenhos_buscados: HashSet<String> = HashSet::new();
    loop {
        while let Some(i) = fila.pop() {
            let arq = itens[i].original.clone();
            let txt = std::fs::read_to_string(&arq)?;
            let dir = arq.parent().map(Path::to_path_buf).unwrap_or_default();
            for r in referencias_do_json(&txt) {
                let completo = absoluto(&crate::referencias::Ref { uid: r.uid.clone(), caminho: r.caminho.clone() }.caminho_completo(&dir));
                let k = chave(&completo);
                if vistos.contains(&k) {
                    continue;
                }
                if !completo.is_file() {
                    vistos.insert(k);
                    m.ausentes.push(texto(&completo));
                    continue;
                }
                let excluir = if r.tipo == TipoRef::Link && !o.incluir_linkados {
                    Some("link entre peças (opção desmarcada)")
                } else if o.pular_bibliotecas && na_biblioteca(&completo, &o.pastas_biblioteca) {
                    Some("biblioteca (opção marcada)")
                } else if !o.incluir_derivados && std::fs::read_to_string(&completo).map(|t| tem_origem(&t)).unwrap_or(false) {
                    Some("documento derivado (opção desmarcada)")
                } else {
                    None
                };
                vistos.insert(k);
                if let Some(motivo) = excluir {
                    m.excluidas.push(Excluida { original: texto(&completo), motivo: motivo.into() });
                    continue;
                }
                itens.push(Item { original: completo, papel: if r.tipo == TipoRef::Link { "linkado" } else { "referenciado" } });
                fila.push(itens.len() - 1);
            }
        }
        if !o.incluir_desenhos {
            break;
        }
        // desenhos que usam algo já incluído; cada novo desenho pode trazer mais referências
        let mut novos = false;
        let atuais: Vec<PathBuf> = itens.iter().filter(|i| !eh_desenho(&i.original)).map(|i| i.original.clone()).collect();
        for alvo in atuais {
            if !desenhos_buscados.insert(chave(&alvo)) {
                continue;
            }
            let uid = crate::referencias::ler_uid(&alvo);
            for u in onde_usado(&o.pastas_busca, &alvo, &uid).into_iter().filter(|u| eh_desenho(&u.arquivo)) {
                let k = chave(&u.arquivo);
                if vistos.insert(k) {
                    itens.push(Item { original: absoluto(&u.arquivo), papel: "desenho" });
                    fila.push(itens.len() - 1);
                    novos = true;
                }
            }
        }
        if !novos && fila.is_empty() {
            break;
        }
    }
    Ok(itens)
}

/// Caminho de cada item dentro do pacote, registrando colisões da estrutura plana.
fn posicionar(itens: &[Item], estrutura: Estrutura, m: &mut Manifesto) -> Vec<String> {
    match estrutura {
        Estrutura::Plana => {
            let mut usados: HashSet<String> = HashSet::new();
            usados.insert(NOME_MANIFESTO.to_string());
            itens
                .iter()
                .map(|i| {
                    let nome = i.original.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let mut final_ = nome.clone();
                    let mut n = 2;
                    while !usados.insert(final_.to_lowercase()) {
                        final_ = nome_com_contador(&nome, n);
                        n += 1;
                    }
                    if final_ != nome {
                        m.colisoes.push(Colisao { original: texto(&i.original), pedido: nome, gravado: final_.clone() });
                    }
                    final_
                })
                .collect()
        }
        Estrutura::Hierarquica => {
            let comps: Vec<Vec<String>> = itens.iter().map(|i| partes(&lexical(&i.original))).collect();
            // base comum entre os arquivos da mesma unidade da raiz
            let unidade0 = comps[0].first().cloned().unwrap_or_default();
            let mesma: Vec<&Vec<String>> = comps.iter().filter(|c| c.first() == Some(&unidade0)).collect();
            let mut base: Vec<String> = mesma[0][..mesma[0].len() - 1].to_vec();
            for c in &mesma {
                let dir = &c[..c.len() - 1];
                let mut k = 0;
                while k < base.len() && k < dir.len() && base[k].eq_ignore_ascii_case(&dir[k]) {
                    k += 1;
                }
                base.truncate(k);
            }
            comps
                .iter()
                .map(|c| {
                    if c.first() == Some(&unidade0) {
                        c[base.len()..].join("/")
                    } else {
                        // outra unidade: não existe caminho relativo entre elas
                        let u: String = c[0].chars().filter(|x| x.is_alphanumeric()).collect();
                        format!("_unidade_{u}/{}", c[1..].join("/"))
                    }
                })
                .collect()
        }
    }
}

/// Componentes de um caminho absoluto: a unidade (ou `/`) vem primeiro.
fn partes(p: &Path) -> Vec<String> {
    p.components()
        .map(|c| match c {
            Component::Prefix(x) => x.as_os_str().to_string_lossy().into_owned(),
            Component::RootDir => "/".to_string(),
            c => c.as_os_str().to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>()
        .into_iter()
        .fold(vec![], |mut v: Vec<String>, c| {
            // junta "C:" e "/" numa só componente de unidade
            if c == "/" && v.len() == 1 {
                return v;
            }
            v.push(c);
            v
        })
}

/// Empacota `raiz` e suas dependências em `destino`: uma pasta nova ou um arquivo `.zip`.
/// A pasta de destino precisa não existir ou estar vazia; um `.zip` existente é substituído (atômico).
pub fn empacotar(raiz: &Path, destino: &Path, o: &OpcoesPacote) -> PResult<ResultadoPacote> {
    let em_zip = destino.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false);
    if !em_zip && destino.is_dir() && std::fs::read_dir(destino)?.next().is_some() {
        return Err(ProjectError::Recusado("a pasta de destino não está vazia".into()));
    }
    if !em_zip && destino.is_file() {
        return Err(ProjectError::Recusado("o destino é um arquivo".into()));
    }
    let mut m = Manifesto {
        versao: 1,
        app_version: fenkra_core::formato::APP_VERSION.into(),
        estrutura: if o.estrutura == Estrutura::Plana { "plana".into() } else { "hierarquica".into() },
        ..Default::default()
    };
    let itens = reunir(raiz, o, &mut m)?;
    let caminhos = posicionar(&itens, o.estrutura, &mut m);
    let mapa: HashMap<String, &String> = itens.iter().zip(&caminhos).map(|(i, c)| (chave(&i.original), c)).collect();
    m.raiz = caminhos[0].clone();

    let mut arquivos: Vec<(String, Vec<u8>)> = vec![];
    for (item, pac) in itens.iter().zip(&caminhos) {
        let txt = std::fs::read_to_string(&item.original)?;
        let dir_orig = item.original.parent().map(Path::to_path_buf).unwrap_or_default();
        let dir_pac = pac.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let mut trocas: Vec<(String, String)> = vec![];
        let mut vistos: HashSet<String> = HashSet::new();
        for r in referencias_do_json(&txt) {
            if !vistos.insert(r.caminho.clone()) {
                continue;
            }
            let completo = absoluto(&crate::referencias::Ref { uid: r.uid.clone(), caminho: r.caminho.clone() }.caminho_completo(&dir_orig));
            match mapa.get(&chave(&completo)) {
                Some(alvo) => {
                    let novo = relativo_virtual(dir_pac, alvo);
                    if novo != r.caminho {
                        trocas.push((r.caminho, novo));
                    }
                }
                None => {
                    // fora do pacote (excluída, ausente ou de outra origem): aponta para o original
                    let abs = texto(&lexical(&completo));
                    m.externas.push(Externa { documento: pac.clone(), caminho: abs.clone() });
                    if abs != r.caminho {
                        trocas.push((r.caminho, abs));
                    }
                }
            }
        }
        let (saida, _) = trocar_caminhos(&txt, &trocas);
        m.entradas.push(Entrada {
            papel: item.papel.into(),
            original: texto(&item.original),
            pacote: pac.clone(),
            uid: fenkra_core::formato::uid_do_json(&saida),
            hash: blake3::hash(saida.as_bytes()).to_hex().to_string(),
            bytes: saida.len() as u64,
        });
        arquivos.push((pac.clone(), saida.into_bytes()));
    }
    let manifesto_json = serde_json::to_string_pretty(&m)?;
    arquivos.push((NOME_MANIFESTO.into(), manifesto_json.into_bytes()));

    if em_zip {
        let z = escrever_zip(&arquivos).map_err(ProjectError::Recusado)?;
        gravar_atomico(destino, &z)?;
    } else {
        for (nome, dados) in &arquivos {
            gravar_atomico(&destino.join(nome), dados)?;
        }
    }
    let verificacao = verificar(destino)?;
    Ok(ResultadoPacote { manifesto: m, verificacao })
}

// ---------------------------------------------------------------------------------- verificar

fn ler_pasta(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> std::io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            ler_pasta(base, &p, out)?;
        } else if p.extension().and_then(|x| x.to_str()) != Some("bak") {
            let rel = p.strip_prefix(base).map(texto).unwrap_or_default();
            out.insert(rel, std::fs::read(&p)?);
        }
    }
    Ok(())
}

/// Reabre o pacote (pasta ou `.zip`) e confere tudo que o manifesto promete.
pub fn verificar(pacote: &Path) -> PResult<Verificacao> {
    let mut arquivos: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    if pacote.is_dir() {
        ler_pasta(pacote, pacote, &mut arquivos)?;
    } else {
        for (n, d) in ler_zip(&std::fs::read(pacote)?).map_err(ProjectError::Recusado)? {
            arquivos.insert(n, d);
        }
    }
    let manifesto: Manifesto = match arquivos.get(NOME_MANIFESTO) {
        Some(b) => serde_json::from_slice(b)?,
        None => return Err(ProjectError::Recusado("o pacote não tem manifesto.json".into())),
    };
    let mut v = Verificacao::default();
    let listados: HashSet<&str> = manifesto.entradas.iter().map(|e| e.pacote.as_str()).collect();
    for nome in arquivos.keys() {
        if nome != NOME_MANIFESTO && !listados.contains(nome.as_str()) {
            v.nao_listados.push(nome.clone());
        }
    }
    let externas: HashSet<(&str, &str)> = manifesto.externas.iter().map(|e| (e.documento.as_str(), e.caminho.as_str())).collect();
    for e in &manifesto.entradas {
        let Some(dados) = arquivos.get(&e.pacote) else {
            v.faltando.push(e.pacote.clone());
            continue;
        };
        if dados.len() as u64 != e.bytes || blake3::hash(dados).to_hex().to_string() != e.hash {
            v.hash_ou_tamanho_diferente.push(e.pacote.clone());
        }
        let Ok(txt) = std::str::from_utf8(dados) else {
            v.ilegiveis.push(e.pacote.clone());
            continue;
        };
        if serde_json::from_str::<serde_json::Value>(txt).is_err() {
            v.ilegiveis.push(e.pacote.clone());
            continue;
        }
        if fenkra_core::formato::uid_do_json(txt) != e.uid {
            v.uid_diferente.push(e.pacote.clone());
        }
        let dir = e.pacote.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for r in referencias_do_json(txt) {
            let dentro = juntar_virtual(dir, &r.caminho).map(|p| arquivos.contains_key(&p)).unwrap_or(false);
            if !dentro && !externas.contains(&(e.pacote.as_str(), r.caminho.as_str())) {
                v.refs_quebradas.push((e.pacote.clone(), r.caminho));
            }
        }
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-pct-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn escrever(dir: &Path, rel: &str, json: &str) -> PathBuf {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, json).unwrap();
        p
    }

    fn peca(uid: &str, nome: &str) -> String {
        format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "{nome}"}}"#)
    }

    fn montagem(uid: &str, comps: &[(u64, &str)]) -> String {
        let nos: Vec<String> = comps.iter().map(|(id, p)| format!(r#"{{"id": {id}, "name": "c{id}", "kind": {{"Part": {{"path": "{p}"}}}}}}"#)).collect();
        format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "Conj", "nodes": [{}]}}"#, nos.join(","))
    }

    fn desenho(uid: &str, fonte: &str) -> String {
        format!(r#"{{"format_version": 1, "uid": "{uid}", "name": "Des", "views": [{{"id": 1, "name": "F", "source": {{"path": "{fonte}"}}}}]}}"#)
    }

    /// proj/montagens/conj.fka -> ../pecas/a.fkp e ../comum/b.fkp ; proj/desenhos/d.fkd -> conj
    fn cenario(nome: &str) -> (PathBuf, PathBuf) {
        let d = tmp(nome);
        escrever(&d, "pecas/a.fkp", &peca("ua", "A"));
        escrever(&d, "comum/b.fkp", &peca("ub", "B"));
        escrever(&d, "lib/parafuso.fkp", &peca("up", "Parafuso"));
        let c = escrever(&d, "montagens/conj.fka", &montagem("uc", &[(1, "../pecas/a.fkp"), (2, "../comum/b.fkp"), (3, "../lib/parafuso.fkp")]));
        escrever(&d, "desenhos/d.fkd", &desenho("ud", "../montagens/conj.fka"));
        (d, c)
    }

    fn opcoes(d: &Path) -> OpcoesPacote {
        OpcoesPacote { pastas_busca: vec![d.to_path_buf()], pastas_biblioteca: vec![d.join("lib")], ..Default::default() }
    }

    fn nomes(m: &Manifesto) -> Vec<String> {
        let mut v: Vec<String> = m.entradas.iter().map(|e| e.pacote.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn zip_ida_e_volta_e_detecta_adulteracao() {
        let arqs = vec![("a.txt".to_string(), b"ola".to_vec()), ("sub/b.json".to_string(), "ação".as_bytes().to_vec()), ("vazio".to_string(), vec![])];
        let z = escrever_zip(&arqs).unwrap();
        assert_eq!(ler_zip(&z).unwrap(), arqs);
        // CRC conhecido: crc32("123456789") = 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        let mut ruim = z.clone();
        ruim[36] ^= 0xFF; // dentro dos dados do primeiro arquivo
        assert!(ler_zip(&ruim).is_err());
        assert!(ler_zip(&z[..z.len() - 10]).is_err());
        // nome que escapa da pasta é recusado
        let z2 = escrever_zip(&[("../fora.txt".to_string(), b"x".to_vec())]).unwrap();
        assert!(ler_zip(&z2).unwrap_err().contains("inseguro"));
    }

    /// Conferência manual de interoperabilidade: grava um zip em `FENKRA_ZIP_OUT` para abrir com
    /// outro programa (`python -m zipfile -t arquivo.zip`). Rode com `--ignored`.
    #[test]
    #[ignore]
    fn grava_zip_para_conferir_em_outro_programa() {
        let Ok(saida) = std::env::var("FENKRA_ZIP_OUT") else { return };
        let z = escrever_zip(&[("a.txt".to_string(), b"ola".to_vec()), ("sub/b.json".to_string(), "ação".as_bytes().to_vec())]).unwrap();
        std::fs::write(saida, z).unwrap();
    }

    #[test]
    fn pasta_hierarquica_preserva_a_estrutura_e_reabre_sem_erro() {
        let (d, c) = cenario("hier");
        let dest = d.join("saida_pasta");
        let r = empacotar(&c, &dest, &opcoes(&d)).unwrap();
        assert_eq!(nomes(&r.manifesto), vec!["comum/b.fkp", "desenhos/d.fkd", "lib/parafuso.fkp", "montagens/conj.fka", "pecas/a.fkp"]);
        assert!(r.verificacao.ok(), "{:?}", r.verificacao);
        assert!(dest.join("manifesto.json").is_file());
        assert_eq!(r.manifesto.raiz, "montagens/conj.fka");
        // o pacote é independente dos originais: apagar a origem não quebra a verificação
        std::fs::remove_dir_all(d.join("pecas")).unwrap();
        assert!(verificar(&dest).unwrap().ok());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn zip_e_verificado_reabrindo_o_arquivo() {
        let (d, c) = cenario("zip");
        let dest = d.join("pacote.zip");
        let r = empacotar(&c, &dest, &opcoes(&d)).unwrap();
        assert!(r.verificacao.ok(), "{:?}", r.verificacao);
        let lidos: Vec<String> = ler_zip(&std::fs::read(&dest).unwrap()).unwrap().into_iter().map(|(n, _)| n).collect();
        assert!(lidos.contains(&"manifesto.json".to_string()) && lidos.contains(&"pecas/a.fkp".to_string()));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn plana_renomeia_colisoes_e_lista_no_manifesto() {
        let d = tmp("plana");
        escrever(&d, "x/base.fkp", &peca("u1", "X"));
        escrever(&d, "y/base.fkp", &peca("u2", "Y"));
        let c = escrever(&d, "conj.fka", &montagem("uc", &[(1, "x/base.fkp"), (2, "y/base.fkp")]));
        let dest = d.join("plano");
        let o = OpcoesPacote { estrutura: Estrutura::Plana, pastas_busca: vec![d.clone()], ..Default::default() };
        let r = empacotar(&c, &dest, &o).unwrap();
        assert_eq!(nomes(&r.manifesto), vec!["base.fkp", "base_2.fkp", "conj.fka"]);
        assert_eq!(r.manifesto.colisoes.len(), 1);
        let col = &r.manifesto.colisoes[0];
        assert_eq!((col.pedido.as_str(), col.gravado.as_str()), ("base.fkp", "base_2.fkp"));
        assert!(col.original.ends_with("y/base.fkp"));
        // as referências foram reescritas para os nomes novos e cada uma aponta para o arquivo certo
        let conj = std::fs::read_to_string(dest.join("conj.fka")).unwrap();
        assert!(conj.contains("\"base.fkp\"") && conj.contains("\"base_2.fkp\""));
        assert_eq!(crate::referencias::ler_uid(&dest.join("base_2.fkp")), "u2");
        assert!(r.verificacao.ok(), "{:?}", r.verificacao);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn opcoes_de_exclusao_listam_o_que_ficou_de_fora() {
        let (d, c) = cenario("excl");
        let dest = d.join("sem_lib");
        let mut o = opcoes(&d);
        o.pular_bibliotecas = true;
        o.incluir_desenhos = false;
        let r = empacotar(&c, &dest, &o).unwrap();
        assert_eq!(nomes(&r.manifesto), vec!["comum/b.fkp", "montagens/conj.fka", "pecas/a.fkp"]);
        assert_eq!(r.manifesto.excluidas.len(), 1);
        assert!(r.manifesto.excluidas[0].original.ends_with("lib/parafuso.fkp") && r.manifesto.excluidas[0].motivo.contains("biblioteca"));
        // a referência à biblioteca vira externa declarada, então o pacote ainda verifica
        assert_eq!(r.manifesto.externas.len(), 1);
        assert!(r.verificacao.ok(), "{:?}", r.verificacao);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn derivados_e_linkados_podem_ficar_de_fora() {
        let d = tmp("deriv");
        escrever(&d, "importada.fkp", r#"{"format_version":1,"uid":"ui","name":"Imp","origem":{"nome":"x.step","hash":"h"}}"#);
        escrever(&d, "ligada.fkp", &peca("ul", "Ligada"));
        let c = escrever(
            &d,
            "p.fkp",
            r#"{"format_version":1,"uid":"up","name":"P","links":[{"uid":"ul","caminho":"ligada.fkp"}],"nodes":[{"id":1,"name":"I","kind":{"Part":{"path":"importada.fkp"}}}]}"#,
        );
        let tudo = empacotar(&c, &d.join("t"), &OpcoesPacote { pastas_busca: vec![d.clone()], ..Default::default() }).unwrap();
        assert_eq!(nomes(&tudo.manifesto), vec!["importada.fkp", "ligada.fkp", "p.fkp"]);
        let o = OpcoesPacote { incluir_derivados: false, incluir_linkados: false, pastas_busca: vec![d.clone()], ..Default::default() };
        let r = empacotar(&c, &d.join("s"), &o).unwrap();
        assert_eq!(nomes(&r.manifesto), vec!["p.fkp"]);
        assert_eq!(r.manifesto.excluidas.len(), 2);
        assert!(r.verificacao.ok(), "{:?}", r.verificacao);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn referencia_ausente_na_origem_e_registrada() {
        let d = tmp("ausente");
        let c = escrever(&d, "conj.fka", &montagem("uc", &[(1, "nao-existe.fkp")]));
        let r = empacotar(&c, &d.join("p"), &OpcoesPacote::default()).unwrap();
        assert_eq!(r.manifesto.ausentes.len(), 1);
        assert!(r.manifesto.ausentes[0].ends_with("nao-existe.fkp"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn verificacao_pega_arquivo_trocado_faltando_e_referencia_quebrada() {
        let (d, c) = cenario("verif");
        let dest = d.join("pac");
        empacotar(&c, &dest, &opcoes(&d)).unwrap();
        // 1. conteúdo trocado
        let a = dest.join("pecas/a.fkp");
        let original = std::fs::read(&a).unwrap();
        std::fs::write(&a, b"{\"uid\": \"ua\", \"name\": \"ALTERADO\"}").unwrap();
        let v = verificar(&dest).unwrap();
        assert_eq!(v.hash_ou_tamanho_diferente, vec!["pecas/a.fkp".to_string()]);
        std::fs::write(&a, original).unwrap();
        // 2. arquivo sumiu
        std::fs::remove_file(dest.join("comum/b.fkp")).unwrap();
        let v = verificar(&dest).unwrap();
        assert_eq!(v.faltando, vec!["comum/b.fkp".to_string()]);
        assert!(v.refs_quebradas.iter().any(|(doc, c)| doc == "montagens/conj.fka" && c.contains("b.fkp")));
        assert!(!v.ok());
        // 3. arquivo estranho no pacote
        std::fs::write(dest.join("sobra.txt"), b"x").unwrap();
        assert_eq!(verificar(&dest).unwrap().nao_listados, vec!["sobra.txt".to_string()]);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn pasta_de_destino_com_conteudo_e_recusada() {
        let (d, c) = cenario("ocupada");
        let dest = d.join("ocupada");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("meu.txt"), b"x").unwrap();
        assert!(matches!(empacotar(&c, &dest, &opcoes(&d)), Err(ProjectError::Recusado(_))));
        assert!(dest.join("meu.txt").is_file());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn caminhos_virtuais() {
        assert_eq!(relativo_virtual("montagens", "pecas/a.fkp"), "../pecas/a.fkp");
        assert_eq!(relativo_virtual("", "a.fkp"), "a.fkp");
        assert_eq!(relativo_virtual("a/b", "a/c.fkp"), "../c.fkp");
        assert_eq!(relativo_virtual("a", "a/b.fkp"), "b.fkp");
        assert_eq!(juntar_virtual("montagens", "../pecas/a.fkp"), Some("pecas/a.fkp".into()));
        assert_eq!(juntar_virtual("", "../fora.fkp"), None);
        assert_eq!(juntar_virtual("a", "C:/x.fkp"), None);
    }
}
