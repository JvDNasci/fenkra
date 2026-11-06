// SPDX-License-Identifier: LGPL-2.1-or-later
//! Montagem neutra (B105), nos dois sentidos.
//!
//! Importar: cada produto com forma vira um documento só, e cada ocorrência vira um componente que
//! aponta para ele. Quatro parafusos iguais são uma peça e quatro posições, então o B-rep entra uma vez
//! e mudar a peça muda as quatro. Subconjunto vira grupo com as posições já somadas, como no arquivo de
//! árvore, porque a montagem do Fenkra não precisa de um arquivo por nível para ter a mesma árvore.
//!
//! Exportar: os arquivos da montagem voltam a ser produtos e instâncias, e a mesma peça usada N vezes
//! vai uma vez para o arquivo. Achatada, tudo vira um produto só, para quem lê o arquivo sem árvore.

use crate::arvore::{nome_de_arquivo, Conjunto};
use crate::modelo::{compor, cor_de, superficies, PROFUNDIDADE};
use crate::{montar_documento, ErroConv, Formato, ModeloNeutro};
use fenkra_core::{Document, Id, SeloOrigem};
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, IDENTIDADE};
use fenkra_montagem::{Assembly, NodeKind, Placement};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn lugar_de(m: &[f64; 12]) -> Placement {
    Placement { origin: [m[3], m[7], m[11]], rot: [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]] }
}

fn matriz_de(p: &Placement) -> [f64; 12] {
    let (r, o) = (&p.rot, &p.origin);
    [r[0][0], r[0][1], r[0][2], o[0], r[1][0], r[1][1], r[1][2], o[1], r[2][0], r[2][1], r[2][2], o[2]]
}

fn rgb(c: [f32; 4]) -> [u8; 3] {
    [c[0], c[1], c[2]].map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
}

/// Nome de arquivo livre na lista, sem diferença de maiúscula: o Windows não vê diferença.
fn arquivo_livre(nome: &str, pecas: &[(String, Document)]) -> String {
    let base = nome_de_arquivo(nome);
    (1..).map(|i| if i == 1 { format!("{base}.fkp") } else { format!("{base} {i}.fkp") }).find(|n| !pecas.iter().any(|(q, _)| q.eq_ignore_ascii_case(n))).unwrap_or_default()
}

struct Montador<'a> {
    modelo: &'a ModeloNeutro,
    asm: Assembly,
    /// arquivo da peça de cada produto
    arquivos: HashMap<usize, String>,
}

impl Montador<'_> {
    fn descer(&mut self, produto: usize, nome: &str, pai: Option<Id>, local: [f64; 12], cor: Option<[f32; 4]>, nivel: usize) {
        let Some(p) = self.modelo.xde.produtos.get(produto) else { return };
        if nivel > PROFUNDIDADE {
            return;
        }
        if let Some(arquivo) = self.arquivos.get(&produto) {
            let id = self.asm.add_component(nome, NodeKind::Part { path: arquivo.clone() }, lugar_de(&local), pai);
            if let (Some(n), Some(c)) = (self.asm.nodes.iter_mut().find(|n| n.id == id), cor) {
                n.color = Some(rgb(c));
            }
            return;
        }
        // a raiz é a própria montagem: só o que está abaixo dela vira grupo
        let grupo = if nivel == 0 { pai } else { Some(self.asm.add_group(nome, pai)) };
        for f in &p.filhos {
            let filho = self.modelo.nome_da_instancia(f);
            self.descer(f.produto, &filho, grupo, compor(&local, &f.local), f.cor.or(cor), nivel + 1);
        }
    }
}

impl ModeloNeutro {
    /// O arquivo traz peças em lugares diferentes, e não uma peça só embrulhada em montagem?
    pub(crate) fn pede_conjunto(&self) -> bool {
        self.formato == Formato::Step && self.ocorrencias().len() > 1 && (self.tem_montagem() || self.xde.raizes.len() > 1)
    }

    /// Um documento por produto e a montagem que aponta para eles. Devolve quantos corpos entraram.
    pub(crate) fn para_conjunto(&self, nome: &str, caminho: &str, selo: &SeloOrigem) -> Result<(Conjunto, usize), ErroConv> {
        let mut pecas: Vec<(String, Document)> = Vec::new();
        let mut arquivos = HashMap::new();
        let mut corpos = 0;
        for c in self.ocorrencias() {
            if arquivos.contains_key(&c.produto) {
                continue;
            }
            let p = &self.xde.produtos[c.produto];
            let Some(forma) = p.forma.clone() else { continue };
            let nome_peca = if p.nome.trim().is_empty() { format!("Peça {}", pecas.len() + 1) } else { p.nome.clone() };
            let mut sozinha = ModeloNeutro::de_forma(self.formato, &nome_peca, forma);
            sozinha.xde.produtos[0].cor = p.cor;
            let (doc, n) = montar_documento(&sozinha, &nome_peca, caminho, selo.clone())?;
            corpos += n;
            let arquivo = arquivo_livre(&nome_peca, &pecas);
            arquivos.insert(c.produto, arquivo.clone());
            pecas.push((arquivo, doc));
        }
        // uma raiz só é a própria montagem: vale o nome que o arquivo deu a ela
        let raiz = if let [r] = self.xde.raizes[..] { self.xde.produtos.get(r).map(|p| p.nome.as_str()).filter(|n| !n.trim().is_empty()) } else { None };
        let mut m = Montador { modelo: self, asm: Assembly::new(raiz.unwrap_or(nome)), arquivos };
        for &r in &self.xde.raizes {
            let nome_raiz = self.xde.produtos.get(r).map(|p| p.nome.clone()).unwrap_or_default();
            m.descer(r, &nome_raiz, None, IDENTIDADE, None, 0);
        }
        // sem restrição nenhuma, o primeiro componente é a referência de quem vier depois
        if let Some(n) = m.asm.nodes.iter_mut().find(|n| matches!(n.kind, NodeKind::Part { .. })) {
            n.fixed = true;
        }
        Ok((Conjunto { pecas, montagem: Some(m.asm) }, corpos))
    }

    /// Modelo da montagem aberta. `pasta` é a pasta do `.fka`: os caminhos dos nós são relativos a ela.
    pub fn da_montagem(asm: &Assembly, pasta: &Path) -> Result<ModeloNeutro, ErroConv> {
        let mut l = Leitor { produtos: vec![], por_caminho: HashMap::new() };
        let raiz = l.montagem(asm, pasta, &mut vec![])?;
        Ok(ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: l.produtos, raizes: vec![raiz], ..Default::default() } })
    }

    /// Um produto só com todas as peças nos seus lugares. A cor de cada ocorrência vira cor de face,
    /// porque um produto só tem uma cor própria.
    pub(crate) fn achatado(&self) -> Result<ModeloNeutro, ErroConv> {
        let mut soltas = Vec::new();
        for c in self.ocorrencias() {
            let Some(f) = self.xde.produtos[c.produto].forma.as_ref() else { continue };
            soltas.push(ProdutoXde { forma: Some(f.com_matriz(&c.local)?), cor: c.cor, ..Default::default() });
        }
        let nome = self.xde.raizes.first().and_then(|r| self.xde.produtos.get(*r)).map(|p| p.nome.clone()).unwrap_or_default();
        if soltas.is_empty() {
            return Err(ErroConv::Gravacao("o modelo não tem forma para gravar".into()));
        }
        Ok(ModeloNeutro { formato: self.formato, xde: ModeloXde { produtos: vec![superficies(soltas, &nome)], raizes: vec![0], ..Default::default() } })
    }
}

/// Junta os produtos enquanto percorre os arquivos da montagem.
struct Leitor {
    produtos: Vec<ProdutoXde>,
    por_caminho: HashMap<PathBuf, usize>,
}

impl Leitor {
    fn peca(&mut self, caminho: &Path) -> Result<Option<usize>, ErroConv> {
        if let Some(i) = self.por_caminho.get(caminho) {
            return Ok(Some(*i));
        }
        let texto = std::fs::read_to_string(caminho).map_err(|e| ErroConv::Gravacao(format!("'{}': {e}", caminho.display())))?;
        let doc = Document::from_json(&texto).map_err(|e| ErroConv::Gravacao(format!("'{}': {e}", caminho.display())))?;
        let Some(forma) = fenkra_solidos::final_shape(&doc) else { return Ok(None) };
        let cor = doc.collectors.first().map(|c| cor_de(c.color));
        self.produtos.push(ProdutoXde { nome: doc.name.clone(), forma: Some(forma), cor, ..Default::default() });
        let i = self.produtos.len() - 1;
        self.por_caminho.insert(caminho.to_path_buf(), i);
        Ok(Some(i))
    }

    /// Produto da montagem, depois dos filhos: o gravador aceita qualquer ordem, mas o filho precisa
    /// existir para a instância apontar para ele.
    fn montagem(&mut self, asm: &Assembly, pasta: &Path, pilha: &mut Vec<PathBuf>) -> Result<usize, ErroConv> {
        let filhos = self.nivel(asm, None, pasta, pilha)?;
        self.produtos.push(ProdutoXde { nome: asm.name.clone(), filhos, ..Default::default() });
        Ok(self.produtos.len() - 1)
    }

    fn nivel(&mut self, asm: &Assembly, pai: Option<Id>, pasta: &Path, pilha: &mut Vec<PathBuf>) -> Result<Vec<InstanciaXde>, ErroConv> {
        let mut v = Vec::new();
        for n in asm.nodes.iter().filter(|n| n.parent == pai) {
            let (produto, local) = match &n.kind {
                NodeKind::Part { path } => match self.peca(&pasta.join(path))? {
                    Some(i) => (i, matriz_de(&n.placement)),
                    None => continue,
                },
                // grupo só organiza: os filhos já têm a posição dentro da montagem
                NodeKind::Group => {
                    let filhos = self.nivel(asm, Some(n.id), pasta, pilha)?;
                    if filhos.is_empty() {
                        continue;
                    }
                    self.produtos.push(ProdutoXde { nome: n.name.clone(), filhos, ..Default::default() });
                    (self.produtos.len() - 1, IDENTIDADE)
                }
                NodeKind::SubAssembly { path } => {
                    let caminho = pasta.join(path);
                    if pilha.contains(&caminho) {
                        return Err(ErroConv::Gravacao(format!("subconjunto '{}' contém a si mesmo", caminho.display())));
                    }
                    let texto = std::fs::read_to_string(&caminho).map_err(|e| ErroConv::Gravacao(format!("'{}': {e}", caminho.display())))?;
                    let sub = Assembly::from_json(&texto).map_err(|e| ErroConv::Gravacao(format!("'{}': {e}", caminho.display())))?;
                    pilha.push(caminho.clone());
                    let i = self.montagem(&sub, caminho.parent().unwrap_or(pasta), pilha)?;
                    pilha.pop();
                    (i, matriz_de(&n.placement))
                }
            };
            v.push(InstanciaXde { produto, nome: n.name.clone(), local, cor: n.color.map(cor_de) });
        }
        Ok(v)
    }
}
