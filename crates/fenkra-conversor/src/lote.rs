// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conversão em lote (B213). O plano diz o que entra, em quais formatos sai, como se chama o arquivo e o
//! que fazer com o que já existe. [`simular`] responde tudo isso sem gravar nada, e [`executar`] roda
//! exatamente a lista simulada, um item por vez, no processo filho.
//!
//! Cada item grava numa pasta provisória e só depois vai para o destino: uma conversão que passou do
//! tempo não deixa arquivo pela metade, nem estraga o que existia. A pasta inteira se move porque o OBJ
//! grava também o MTL ao lado.

use crate::processo::{Resposta, Trabalho};
use crate::{ErroConv, Formato, OpcoesExportar, OpcoesImportar, Registro};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Regra de nome padrão: número da peça, revisão e extensão.
pub const REGRA_PADRAO: &str = "{PN}_{REV}.{ext}";
/// Extensões nativas, que só o lote aceita como entrada.
const PECA: &str = "fkp";
const MONTAGEM: &str = "fka";

/// Um arquivo a converter, com o que a regra de nome precisa saber dele.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntradaLote {
    pub caminho: PathBuf,
    pub nome: String,
    /// vazio em arquivo solto: a regra usa o nome
    pub pn: String,
    pub rev: String,
}

impl EntradaLote {
    pub fn de_arquivo(caminho: &Path) -> EntradaLote {
        let nome = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        EntradaLote { caminho: caminho.to_path_buf(), nome, pn: String::new(), rev: String::new() }
    }
}

/// O que fazer quando o destino já existe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeExistir {
    /// mantém o que está lá e não converte
    #[default]
    Pular,
    Substituir,
    /// grava ao lado, com um número no nome
    Numerar,
}

impl SeExistir {
    pub const TODOS: [SeExistir; 3] = [SeExistir::Pular, SeExistir::Substituir, SeExistir::Numerar];

    pub fn nome(self) -> &'static str {
        match self {
            SeExistir::Pular => "Pular",
            SeExistir::Substituir => "Substituir",
            SeExistir::Numerar => "Numerar",
        }
    }
}

/// Um formato de saída com as opções dele.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerfilSaida {
    pub formato: Formato,
    pub opcoes: OpcoesExportar,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanoLote {
    pub pasta: PathBuf,
    pub regra: String,
    pub perfis: Vec<PerfilSaida>,
    pub se_existir: SeExistir,
    pub importar: OpcoesImportar,
    /// limite de tempo de cada item, em segundos
    pub limite: u64,
}

impl Default for PlanoLote {
    fn default() -> Self {
        PlanoLote { pasta: PathBuf::new(), regra: REGRA_PADRAO.into(), perfis: vec![], se_existir: SeExistir::default(), importar: OpcoesImportar::default(), limite: 600 }
    }
}

/// O que a simulação decidiu para um item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Situacao {
    Converter,
    /// o destino existe e será trocado
    Substituir,
    /// o destino existia e o nome ganhou um número
    Numerado,
    /// não será convertido; o motivo é um modelo de frase
    Pular(String),
    /// não dá para converter; o motivo é um modelo de frase
    Recusado(String),
}

impl Situacao {
    pub fn converte(&self) -> bool {
        matches!(self, Situacao::Converter | Situacao::Substituir | Situacao::Numerado)
    }
}

pub const MOTIVO_EXISTE: &str = "O arquivo já existe.";
pub const MOTIVO_COLISAO: &str = "Outro item do lote grava neste mesmo arquivo.";
pub const MOTIVO_AUSENTE: &str = "O arquivo de entrada não existe.";
pub const MOTIVO_NAO_LE: &str = "O Fenkra não lê este arquivo.";
pub const MOTIVO_NAO_GRAVA: &str = "O Fenkra não grava este formato.";
pub const MOTIVO_SEM_RECEITA: &str = "Só uma peça do Fenkra tem a receita que este formato guarda.";
pub const MOTIVO_SEM_ESBOCO: &str = "Este formato é 2D: a entrada precisa ser uma peça com esboços ou um desenho.";
pub const MOTIVO_SEM_3D: &str = "A entrada é um desenho 2D: ela só vai para formatos 2D.";

/// Frases de motivo, para a tradução da interface ser conferida.
pub const MOTIVOS: &[&str] = &[MOTIVO_EXISTE, MOTIVO_COLISAO, MOTIVO_AUSENTE, MOTIVO_NAO_LE, MOTIVO_NAO_GRAVA, MOTIVO_SEM_RECEITA, MOTIVO_SEM_ESBOCO, MOTIVO_SEM_3D];

/// Uma conversão da lista: uma entrada num formato de saída.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemLote {
    pub origem: PathBuf,
    pub destino: PathBuf,
    pub perfil: usize,
    pub situacao: Situacao,
}

/// Como um item terminou.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultadoItem {
    pub origem: PathBuf,
    pub destino: PathBuf,
    pub saida: Result<(), ErroConv>,
}

fn extensao(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Expande pastas, em qualquer profundidade, e fica só com o que o conversor consegue ler.
pub fn listar_entradas(caminhos: &[PathBuf], registro: &Registro) -> Vec<EntradaLote> {
    let mut aceitas: HashSet<&str> = registro.extensoes_de_leitura().into_iter().collect();
    aceitas.extend([PECA, MONTAGEM]);
    let mut achados = Vec::new();
    let mut pilha: Vec<PathBuf> = caminhos.to_vec();
    pilha.reverse();
    while let Some(p) = pilha.pop() {
        if p.is_dir() {
            let mut filhos: Vec<PathBuf> = std::fs::read_dir(&p).into_iter().flatten().flatten().map(|e| e.path()).collect();
            filhos.sort();
            filhos.reverse();
            pilha.extend(filhos);
        } else if aceitas.contains(extensao(&p).as_str()) {
            achados.push(EntradaLote::de_arquivo(&p));
        }
    }
    achados
}

/// Caractere que o Windows recusa em nome de arquivo. Um PN pode trazer barra.
fn limpar_nome(nome: &str) -> String {
    let limpo: String = nome.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    let limpo = limpo.trim_end_matches(['.', ' ']).trim_start().to_string();
    let base = limpo.split('.').next().unwrap_or("").to_uppercase();
    let reservado = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL") || (base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT")) && base.as_bytes()[3].is_ascii_digit());
    if reservado {
        format!("_{limpo}")
    } else {
        limpo
    }
}

/// Aplica a regra. `{PN}` cai no nome quando a entrada não tem número, e separador que sobrou de uma
/// lacuna vazia some: `{PN}_{REV}.{ext}` sem revisão dá `peca.step`, e não `peca_.step`.
pub fn nome_pela_regra(regra: &str, e: &EntradaLote, f: Formato) -> String {
    let pn = if e.pn.is_empty() { e.nome.as_str() } else { e.pn.as_str() };
    let ext = f.extensoes()[0];
    let mut t = regra.replace("{PN}", pn).replace("{REV}", &e.rev).replace("{NOME}", &e.nome).replace("{FORMATO}", f.nome());
    let tem_ext = t.contains("{ext}");
    t = t.replace("{ext}", ext);
    for sep in ["_", "-", " "] {
        t = t.replace(&format!("{sep}.{ext}"), &format!(".{ext}"));
        while t.contains(&format!("{sep}{sep}")) {
            t = t.replace(&format!("{sep}{sep}"), sep);
        }
    }
    let t = t.trim_start_matches(['_', '-', ' ']).to_string();
    let t = if tem_ext { t } else { format!("{t}.{ext}") };
    limpar_nome(&t)
}

/// Dá para ir da entrada a este formato?
fn recusa(registro: &Registro, origem: &Path, f: Formato) -> Option<&'static str> {
    if !origem.is_file() {
        return Some(MOTIVO_AUSENTE);
    }
    let e = extensao(origem);
    let nativa = e == PECA || e == MONTAGEM;
    let entrada = Formato::TODOS.into_iter().find(|q| q.extensoes().contains(&e.as_str()));
    if !nativa && !entrada.is_some_and(|q| registro.capacidades(q).ler) {
        return Some(MOTIVO_NAO_LE);
    }
    if !registro.capacidades(f).gravar {
        return Some(MOTIVO_NAO_GRAVA);
    }
    let plana = |q: Formato| q.e_2d();
    let tem_receita = e == PECA || e == MONTAGEM || entrada == Some(Formato::Arvore);
    if f == Formato::Arvore && !tem_receita {
        return Some(MOTIVO_SEM_RECEITA);
    }
    if plana(f) && !(e == PECA || entrada.is_some_and(|q| plana(q) || q == Formato::Arvore)) {
        return Some(MOTIVO_SEM_ESBOCO);
    }
    if !plana(f) && f != Formato::Arvore && entrada.is_some_and(plana) {
        return Some(MOTIVO_SEM_3D);
    }
    None
}

/// `peca.step`, `peca (2).step`... o primeiro nome livre em disco e na lista.
fn numerar(destino: &Path, ocupados: &HashSet<String>) -> PathBuf {
    let base = destino.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = extensao(destino);
    (2..).map(|i| destino.with_file_name(format!("{base} ({i}).{ext}"))).find(|p| !p.exists() && !ocupados.contains(&chave(p))).unwrap_or_else(|| destino.to_path_buf())
}

/// O Windows não diferencia maiúscula em nome de arquivo.
fn chave(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}

/// Decide cada item sem gravar nada. O que sai daqui é o que [`executar`] vai rodar.
pub fn simular(entradas: &[EntradaLote], plano: &PlanoLote, registro: &Registro) -> Vec<ItemLote> {
    let mut ocupados: HashSet<String> = HashSet::new();
    let mut itens = Vec::with_capacity(entradas.len() * plano.perfis.len());
    for e in entradas {
        for (perfil, p) in plano.perfis.iter().enumerate() {
            let mut destino = plano.pasta.join(nome_pela_regra(&plano.regra, e, p.formato));
            let situacao = if let Some(motivo) = recusa(registro, &e.caminho, p.formato) {
                Situacao::Recusado(motivo.into())
            } else {
                let colide = ocupados.contains(&chave(&destino));
                match (plano.se_existir, destino.exists(), colide) {
                    (SeExistir::Numerar, existe, colide) if existe || colide => {
                        destino = numerar(&destino, &ocupados);
                        Situacao::Numerado
                    }
                    (_, _, true) => Situacao::Pular(MOTIVO_COLISAO.into()),
                    (SeExistir::Pular, true, _) => Situacao::Pular(MOTIVO_EXISTE.into()),
                    (SeExistir::Substituir, true, _) => Situacao::Substituir,
                    _ => Situacao::Converter,
                }
            };
            if situacao.converte() {
                ocupados.insert(chave(&destino));
            }
            itens.push(ItemLote { origem: e.caminho.clone(), destino, perfil, situacao });
        }
    }
    itens
}

/// Move o que o filho gravou para a pasta de saída. Se tiver um MTL, o OBJ já aponta para ele pelo nome.
fn publicar(provisoria: &Path, pasta: &Path) -> Result<(), ErroConv> {
    let falha = |e: std::io::Error| ErroConv::Gravacao(e.to_string());
    for f in std::fs::read_dir(provisoria).map_err(falha)?.flatten() {
        let alvo = pasta.join(f.file_name());
        // a troca é atômica no mesmo disco; a pasta provisória fica dentro da de saída para isso valer
        std::fs::rename(f.path(), &alvo).map_err(falha)?;
    }
    Ok(())
}

/// Roda a lista simulada. `rodar` executa um trabalho, em geral num [`crate::processo::Filho`]. Erro num
/// item não para o lote; cancelar para, e o que faltava sai como cancelado. `aviso` recebe cada resultado.
pub fn executar(itens: &[ItemLote], plano: &PlanoLote, mut rodar: impl FnMut(Trabalho) -> Result<Resposta, ErroConv>, mut aviso: impl FnMut(usize, &ResultadoItem)) -> Vec<ResultadoItem> {
    let mut saida = Vec::new();
    let mut parar = false;
    for (i, item) in itens.iter().enumerate().filter(|(_, i)| i.situacao.converte()) {
        let resultado = if parar { Err(ErroConv::Cancelado) } else { um_item(item, plano, &mut rodar) };
        parar |= matches!(resultado, Err(ErroConv::Cancelado));
        let r = ResultadoItem { origem: item.origem.clone(), destino: item.destino.clone(), saida: resultado };
        aviso(i, &r);
        saida.push(r);
    }
    saida
}

fn um_item(item: &ItemLote, plano: &PlanoLote, rodar: &mut impl FnMut(Trabalho) -> Result<Resposta, ErroConv>) -> Result<(), ErroConv> {
    let perfil = plano.perfis.get(item.perfil).ok_or_else(|| ErroConv::Gravacao("perfil inexistente".into()))?;
    let pasta = item.destino.parent().unwrap_or(Path::new("."));
    let falha = |e: std::io::Error| ErroConv::Gravacao(format!("{}: {e}", pasta.display()));
    std::fs::create_dir_all(pasta).map_err(falha)?;
    let provisoria = pasta.join(format!(".fenkra-lote-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&provisoria);
    std::fs::create_dir_all(&provisoria).map_err(falha)?;
    let nome = item.destino.file_name().ok_or_else(|| ErroConv::Gravacao("destino sem nome".into()))?;
    let trabalho = Trabalho::Converter { origem: item.origem.clone(), destino: provisoria.join(nome), importar: plano.importar, exportar: perfil.opcoes };
    let resultado = rodar(trabalho).and_then(|_| publicar(&provisoria, pasta));
    let _ = std::fs::remove_dir_all(&provisoria);
    resultado
}

#[cfg(test)]
mod testes {
    use super::*;

    fn entrada(nome: &str, pn: &str, rev: &str) -> EntradaLote {
        EntradaLote { caminho: format!("{nome}.step").into(), nome: nome.into(), pn: pn.into(), rev: rev.into() }
    }

    #[test]
    fn regra_preenche_e_lacuna_vazia_nao_deixa_sobra() {
        assert_eq!(nome_pela_regra(REGRA_PADRAO, &entrada("Placa", "FK-0002", "B"), Formato::Step), "FK-0002_B.step");
        assert_eq!(nome_pela_regra(REGRA_PADRAO, &entrada("Placa", "", ""), Formato::Stl), "Placa.stl");
        assert_eq!(nome_pela_regra("{REV}-{NOME}.{ext}", &entrada("Placa", "", ""), Formato::Glb), "Placa.glb");
        assert_eq!(nome_pela_regra("{NOME} {FORMATO}", &entrada("Placa", "", "A"), Formato::Tmf), "Placa 3MF.3mf");
    }

    #[test]
    fn nome_com_barra_ou_reservado_nao_quebra_o_caminho() {
        assert_eq!(nome_pela_regra(REGRA_PADRAO, &entrada("x", "FK/12:3", "A"), Formato::Step), "FK_12_3_A.step");
        assert_eq!(nome_pela_regra("{NOME}.{ext}", &entrada("CON", "", ""), Formato::Step), "_CON.step");
        assert_eq!(nome_pela_regra("{NOME}.{ext}", &entrada("COM1", "", ""), Formato::Step), "_COM1.step");
        assert_eq!(nome_pela_regra("{NOME}.{ext}", &entrada("COMPOSTO", "", ""), Formato::Step), "COMPOSTO.step");
    }

    #[test]
    fn motivos_sao_listados_para_traduzir() {
        assert!(MOTIVOS.contains(&MOTIVO_COLISAO) && MOTIVOS.contains(&MOTIVO_SEM_3D));
    }
}
