// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutura de montagem do arquivo de árvore: quais peças, onde e sob qual grupo. Subconjunto vira
//! grupo com as posições já somadas, porque o arquivo é um só e não tem como apontar para outro.

use super::*;
use crate::ErroConv;
use fenkra_core::{Document, Id};
use fenkra_core::formato::Tipo;
use fenkra_montagem::{Assembly, NodeKind, Placement};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// O que um arquivo com várias peças vira: documentos prontos para gravar em pasta e a montagem que
/// aponta para eles pelo nome do arquivo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conjunto {
    /// nome do arquivo `.fkp` e o documento
    pub pecas: Vec<(String, Document)>,
    pub montagem: Option<Assembly>,
}

fn compor(pai: &Placement, filho: &Placement) -> Placement {
    let mut rot = [[0.; 3]; 3];
    for (i, linha) in rot.iter_mut().enumerate() {
        for (j, v) in linha.iter_mut().enumerate() {
            *v = (0..3).map(|k| pai.rot[i][k] * filho.rot[k][j]).sum();
        }
    }
    Placement { origin: pai.point(filho.origin), rot }
}

/// Acumula peças e componentes enquanto desce pelos subconjuntos.
struct Coleta {
    pecas: Vec<Peca>,
    por_caminho: HashMap<PathBuf, String>,
    componentes: Vec<Componente>,
    fora: Vec<String>,
    proximo: u64,
}

impl Coleta {
    fn peca(&mut self, caminho: &Path) -> Result<String, ErroConv> {
        if let Some(id) = self.por_caminho.get(caminho) {
            return Ok(id.clone());
        }
        let texto = std::fs::read_to_string(caminho).map_err(|e| ErroConv::Leitura(format!("'{}': {e}", caminho.display())))?;
        let mut doc = Document::from_json(&texto).map_err(|e| ErroConv::Leitura(format!("'{}': {e}", caminho.display())))?;
        atualizar_pontos(&mut doc);
        let mut e = exportar_peca(&doc);
        // a mesma peça copiada em dois arquivos tem o mesmo uid; o id aqui tem que ser único
        e.peca.id = format!("peca{}", self.pecas.len() + 1);
        self.fora.extend(e.fora.iter().map(|f| format!("{}: {}", doc.name, f.texto())));
        self.por_caminho.insert(caminho.to_path_buf(), e.peca.id.clone());
        let id = e.peca.id.clone();
        self.pecas.push(e.peca);
        Ok(id)
    }

    fn novo_id(&mut self) -> String {
        self.proximo += 1;
        format!("c{}", self.proximo)
    }

    fn montagem(&mut self, asm: &Assembly, pasta: &Path, base: &Placement, pai: Option<String>, pilha: &mut Vec<PathBuf>) -> Result<(), ErroConv> {
        if !asm.mates.is_empty() {
            self.fora.push(format!("{}: {} restrições entre peças não existem na versão 1; as posições atuais foram gravadas", asm.name, asm.mates.len()));
        }
        let mut ids: HashMap<Id, String> = HashMap::new();
        for n in &asm.nodes {
            ids.insert(n.id, self.novo_id());
        }
        for n in &asm.nodes {
            let id = ids[&n.id].clone();
            let dono = n.parent.and_then(|p| ids.get(&p).cloned()).or_else(|| pai.clone());
            let lugar = compor(base, &n.placement);
            let mut c = Componente { id: id.clone(), nome: n.name.clone(), peca: None, pai: dono, origem: lugar.origin, rotacao: lugar.rot, fixo: n.fixed, cor: n.color };
            match &n.kind {
                NodeKind::Group => self.componentes.push(c),
                NodeKind::Part { path } => {
                    c.peca = Some(self.peca(&pasta.join(path))?);
                    self.componentes.push(c);
                }
                NodeKind::SubAssembly { path } => {
                    let caminho = pasta.join(path);
                    if pilha.contains(&caminho) {
                        return Err(ErroConv::Leitura(format!("subconjunto '{}' contém a si mesmo", caminho.display())));
                    }
                    let texto = std::fs::read_to_string(&caminho).map_err(|e| ErroConv::Leitura(format!("'{}': {e}", caminho.display())))?;
                    let sub = Assembly::from_json(&texto).map_err(|e| ErroConv::Leitura(format!("'{}': {e}", caminho.display())))?;
                    self.componentes.push(c);
                    pilha.push(caminho.clone());
                    self.montagem(&sub, caminho.parent().unwrap_or(pasta), &lugar, Some(id), pilha)?;
                    pilha.pop();
                }
            }
        }
        Ok(())
    }
}

/// Traduz a montagem e as peças que ela usa. `pasta` é a pasta do `.fka`.
pub fn exportar_montagem(asm: &Assembly, pasta: &Path) -> Result<Arquivo, ErroConv> {
    let mut c = Coleta { pecas: vec![], por_caminho: HashMap::new(), componentes: vec![], fora: vec![], proximo: 0 };
    c.montagem(asm, pasta, &Placement::default(), None, &mut vec![])?;
    Ok(Arquivo {
        formato: NOME_FORMATO.into(),
        format_version: Tipo::Arvore.versao_atual(),
        gerador: exportar::gerador(),
        unidades: "mm".into(),
        pecas: c.pecas,
        montagem: Some(Montagem { nome: asm.name.clone(), componentes: c.componentes }),
        fora: c.fora,
    })
}

/// Nome de arquivo tirado de texto de fora: nada de separador nem de `..`, ou um arquivo hostil
/// escreveria fora da pasta escolhida.
fn nome_de_arquivo(nome: &str) -> String {
    let limpo: String = nome.chars().map(|c| if c.is_alphanumeric() || " -_".contains(c) { c } else { '_' }).collect();
    let limpo = limpo.trim();
    if limpo.is_empty() {
        "peca".into()
    } else {
        limpo.chars().take(80).collect()
    }
}

/// Remonta todas as peças do arquivo e a montagem. Devolve também as reancoragens, na ordem das peças.
pub fn remontar(arquivo: &Arquivo) -> Result<(Conjunto, Vec<Reancoragem>), ErroConv> {
    let mut pecas: Vec<(String, Document)> = vec![];
    let mut por_id: HashMap<&str, String> = HashMap::new();
    let mut reancoragens = vec![];
    for p in &arquivo.pecas {
        let r = ler_peca(&p)?;
        let base = nome_de_arquivo(&p.nome);
        let nome = (1..).map(|i| if i == 1 { format!("{base}.fkp") } else { format!("{base} {i}.fkp") }).find(|n| !pecas.iter().any(|(q, _)| q.eq_ignore_ascii_case(n))).unwrap_or_default();
        if por_id.insert(&p.id, nome.clone()).is_some() {
            return Err(ErroConv::Leitura(format!("id de peça '{}' repetido", p.id)));
        }
        reancoragens.push(r.reancoragem);
        pecas.push((nome, r.doc));
    }
    let montagem = arquivo.montagem.as_ref().map(|m| remontar_estrutura(m, &por_id)).transpose()?;
    Ok((Conjunto { pecas, montagem }, reancoragens))
}

fn remontar_estrutura(m: &Montagem, por_id: &HashMap<&str, String>) -> Result<Assembly, ErroConv> {
    let mut a = Assembly::new(&m.nome);
    let mut ids: HashMap<&str, Id> = HashMap::new();
    for c in &m.componentes {
        let pai = c.pai.as_deref().map(|p| ids.get(p).copied().ok_or_else(|| ErroConv::Leitura(format!("componente '{}': o pai '{p}' tem que vir antes dele", c.nome)))).transpose()?;
        let id = match &c.peca {
            None => a.add_group(&c.nome, pai),
            Some(p) => {
                let path = por_id.get(p.as_str()).ok_or_else(|| ErroConv::Leitura(format!("componente '{}': a peça '{p}' não existe", c.nome)))?;
                a.add_component(&c.nome, NodeKind::Part { path: path.clone() }, Placement { origin: c.origem, rot: c.rotacao }, pai)
            }
        };
        if let Some(n) = a.nodes.iter_mut().find(|n| n.id == id) {
            n.placement = Placement { origin: c.origem, rot: c.rotacao };
            n.fixed = c.fixo;
            n.color = c.cor;
        }
        if ids.insert(&c.id, id).is_some() {
            return Err(ErroConv::Leitura(format!("id de componente '{}' repetido", c.id)));
        }
    }
    Ok(a)
}

/// Grava as peças e a montagem em `pasta` e devolve o caminho do que deve abrir.
pub fn gravar_em_pasta(c: &Conjunto, pasta: &Path) -> Result<PathBuf, ErroConv> {
    let gravar = |nome: &str, texto: String| std::fs::write(pasta.join(nome), texto).map_err(|e| ErroConv::Gravacao(format!("'{nome}': {e}")));
    for (nome, doc) in &c.pecas {
        gravar(nome, doc.to_json())?;
    }
    let Some(a) = &c.montagem else {
        return c.pecas.first().map(|(n, _)| pasta.join(n)).ok_or_else(|| ErroConv::Gravacao("arquivo sem peças".into()));
    };
    let nome = format!("{}.fka", nome_de_arquivo(&a.name));
    gravar(&nome, a.to_json())?;
    Ok(pasta.join(nome))
}
