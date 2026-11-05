// SPDX-License-Identifier: LGPL-2.1-or-later
//! Modelo neutro: a árvore de produtos que todo provedor lê e grava.

use crate::Formato;
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};

pub type ProdutoNeutro = ProdutoXde;
pub type InstanciaNeutra = InstanciaXde;

/// Árvore de produtos com a origem dela. Um grafo: uma peça usada N vezes é um produto.
#[derive(Clone)]
pub struct ModeloNeutro {
    pub formato: Formato,
    pub xde: ModeloXde,
}

/// Uma peça no lugar em que aparece, depois de percorrer as montagens.
#[derive(Clone, Debug, PartialEq)]
pub struct Ocorrencia {
    pub produto: usize,
    /// nomes da raiz até aqui
    pub caminho: Vec<String>,
    pub local: [f64; 12],
    pub cor: Option<[f32; 4]>,
}

/// `a` depois de `b`, com as matrizes 3x4 por linhas.
fn compor(a: &[f64; 12], b: &[f64; 12]) -> [f64; 12] {
    let mut r = [0.; 12];
    for i in 0..3 {
        for j in 0..4 {
            let soma: f64 = (0..3).map(|k| a[i * 4 + k] * b[k * 4 + j]).sum();
            r[i * 4 + j] = soma + if j == 3 { a[i * 4 + 3] } else { 0. };
        }
    }
    r
}

/// Limite de profundidade: um arquivo com ciclo de montagens não pode travar o conversor.
const PROFUNDIDADE: usize = 64;

impl ModeloNeutro {
    /// Modelo de uma forma só, para formatos sem estrutura.
    pub fn de_forma(formato: Formato, nome: &str, forma: Shape) -> ModeloNeutro {
        let produto = ProdutoXde { nome: nome.into(), forma: Some(forma), ..Default::default() };
        ModeloNeutro { formato, xde: ModeloXde { produtos: vec![produto], raizes: vec![0], ..Default::default() } }
    }

    /// Nome de uma ocorrência: a instância quando o arquivo nomeou, senão o produto.
    fn nome_da_instancia(&self, i: &InstanciaXde) -> String {
        let automatico = i.nome.is_empty() || i.nome.starts_with("=>") || i.nome.starts_with("NAUO");
        if automatico {
            self.xde.produtos.get(i.produto).map(|p| p.nome.clone()).unwrap_or_default()
        } else {
            i.nome.clone()
        }
    }

    fn descer(&self, produto: usize, caminho: Vec<String>, local: [f64; 12], herdada: Option<[f32; 4]>, saida: &mut Vec<Ocorrencia>) {
        let Some(p) = self.xde.produtos.get(produto) else { return };
        if caminho.len() > PROFUNDIDADE {
            return;
        }
        if p.forma.is_some() {
            saida.push(Ocorrencia { produto, caminho, local, cor: herdada.or(p.cor) });
            return;
        }
        for f in &p.filhos {
            let mut c = caminho.clone();
            c.push(self.nome_da_instancia(f));
            self.descer(f.produto, c, compor(&local, &f.local), herdada.or(f.cor).or(p.cor), saida);
        }
    }

    /// Todas as peças nos seus lugares, em ordem de árvore.
    pub fn ocorrencias(&self) -> Vec<Ocorrencia> {
        let mut saida = Vec::new();
        for &r in &self.xde.raizes {
            let nome = self.xde.produtos.get(r).map(|p| p.nome.clone()).unwrap_or_default();
            self.descer(r, vec![nome], IDENTIDADE, None, &mut saida);
        }
        saida
    }

    pub fn instancias(&self) -> usize {
        self.xde.produtos.iter().map(|p| p.filhos.len()).sum()
    }

    pub fn tem_montagem(&self) -> bool {
        self.xde.produtos.iter().any(|p| p.forma.is_none())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn deslocado(x: f64) -> [f64; 12] {
        [1., 0., 0., x, 0., 1., 0., 0., 0., 0., 1., 0.]
    }

    fn instancia(produto: usize, nome: &str, local: [f64; 12]) -> InstanciaXde {
        InstanciaXde { produto, nome: nome.into(), local, cor: None }
    }

    #[test]
    fn matrizes_compoem_na_ordem_da_arvore() {
        // gira 90 graus em Z e desloca: o filho a 10 em X vai para 10 em Y, mais o deslocamento do pai
        let pai = [0., -1., 0., 5., 1., 0., 0., 0., 0., 0., 1., 0.];
        let r = compor(&pai, &deslocado(10.));
        assert_eq!([r[3], r[7], r[11]], [5., 10., 0.]);
    }

    fn com_ciclo() -> ModeloNeutro {
        let a = ProdutoXde { nome: "A".into(), filhos: vec![instancia(1, "", IDENTIDADE)], ..Default::default() };
        let b = ProdutoXde { nome: "B".into(), filhos: vec![instancia(0, "", IDENTIDADE)], ..Default::default() };
        ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![a, b], raizes: vec![0], ..Default::default() } }
    }

    #[test]
    fn ciclo_de_montagens_nao_trava() {
        assert!(com_ciclo().ocorrencias().is_empty());
    }

    #[test]
    fn nome_automatico_cede_ao_do_produto() {
        let m = com_ciclo();
        assert_eq!(m.nome_da_instancia(&instancia(1, "NAUO3", IDENTIDADE)), "B");
        assert_eq!(m.nome_da_instancia(&instancia(1, "Pino 2", IDENTIDADE)), "Pino 2");
    }
}
