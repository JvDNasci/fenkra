// SPDX-License-Identifier: LGPL-2.1-or-later
//! Modelo neutro: a árvore de produtos que todo provedor lê e grava.

use crate::{ErroConv, Formato};
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
pub(crate) fn compor(a: &[f64; 12], b: &[f64; 12]) -> [f64; 12] {
    let mut r = [0.; 12];
    for i in 0..3 {
        for j in 0..4 {
            let soma: f64 = (0..3).map(|k| a[i * 4 + k] * b[k * 4 + j]).sum();
            r[i * 4 + j] = soma + if j == 3 { a[i * 4 + 3] } else { 0. };
        }
    }
    r
}

pub(crate) fn cor_de(c: [u8; 3]) -> [f32; 4] {
    [c[0] as f32 / 255., c[1] as f32 / 255., c[2] as f32 / 255., 1.]
}

/// Limite de profundidade: um arquivo com ciclo de montagens não pode travar o conversor.
pub(crate) const PROFUNDIDADE: usize = 64;

impl ModeloNeutro {
    /// Modelo de uma forma só, para formatos sem estrutura.
    pub fn de_forma(formato: Formato, nome: &str, forma: Shape) -> ModeloNeutro {
        let produto = ProdutoXde { nome: nome.into(), forma: Some(forma), ..Default::default() };
        ModeloNeutro { formato, xde: ModeloXde { produtos: vec![produto], raizes: vec![0], ..Default::default() } }
    }

    /// Modelo de uma peça aberta: a forma final do documento, com a cor do primeiro coletor.
    pub fn de_documento(doc: &fenkra_core::Document) -> Result<ModeloNeutro, ErroConv> {
        let forma = fenkra_solidos::final_shape(doc).ok_or_else(|| ErroConv::Gravacao("o documento não tem forma para gravar".into()))?;
        let mut m = ModeloNeutro::de_forma(Formato::Step, &doc.name, forma);
        m.xde.produtos[0].cor = doc.collectors.first().map(|c| cor_de(c.color));
        Ok(m)
    }

    /// Modelo da peça aberta com o PMI semântico (cotas, tolerâncias, referências e alvos) ligado às
    /// faces. Devolve junto as notas do que o STEP não leva. Sem PMI no documento, sai como `de_documento`.
    pub fn de_documento_pmi(doc: &fenkra_core::Document) -> Result<(ModeloNeutro, Vec<crate::Nota>), ErroConv> {
        let mut m = Self::de_documento(doc)?;
        let Some(forma) = m.xde.produtos[0].forma.as_ref() else { return Ok((m, vec![])) };
        let saida = crate::pmi_step::anotacoes_do_documento(doc, forma);
        m.xde.anotacoes = saida.anotacoes;
        Ok((m, saida.fora))
    }

    /// Nome de uma ocorrência: a instância quando o arquivo nomeou, senão o produto.
    pub(crate) fn nome_da_instancia(&self, i: &InstanciaXde) -> String {
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

    /// Junta numa peça só os produtos sem sólido, em ordem. O IGES não tem árvore: cada superfície é uma
    /// raiz, e separadas elas nunca se costurariam. A cor de cada uma vira cor de face na peça junta.
    pub(crate) fn juntar_soltas(&mut self, nome: &str) -> Result<(), ErroConv> {
        if self.tem_montagem() {
            return Ok(());
        }
        let (mut soltas, mut outros) = (Vec::new(), Vec::new());
        for p in std::mem::take(&mut self.xde.produtos) {
            let tem_solido = p.forma.as_ref().map(|f| f.solidos().map(|s| !s.is_empty())).transpose()?.unwrap_or(false);
            if tem_solido {
                outros.push(p);
            } else {
                soltas.push(p);
            }
        }
        if soltas.len() > 1 {
            soltas = vec![superficies(soltas, nome)];
        }
        outros.extend(soltas);
        self.xde.raizes = (0..outros.len()).collect();
        self.xde.produtos = outros;
        Ok(())
    }
}

/// Uma peça com as superfícies dadas. O composto cresce em árvore equilibrada: em fila, mil superfícies
/// virariam mil níveis de composto para o explorador do kernel descer a cada consulta.
pub(crate) fn superficies(soltas: Vec<ProdutoXde>, nome: &str) -> ProdutoXde {
    let mut p = ProdutoXde { nome: nome.into(), ..Default::default() };
    let (mut formas, mut base) = (Vec::with_capacity(soltas.len()), 0);
    for s in soltas {
        let Some(forma) = s.forma else { continue };
        let faces = forma.face_count();
        match s.cor {
            Some(c) if s.cores_faces.is_empty() => p.cores_faces.extend((0..faces).map(|i| (base + i, c))),
            _ => p.cores_faces.extend(s.cores_faces.iter().map(|(i, c)| (base + i, *c))),
        }
        p.camadas.extend(s.camadas.into_iter().filter(|c| !c.is_empty()));
        base += faces;
        formas.push(forma);
    }
    p.camadas.sort_unstable();
    p.camadas.dedup();
    while formas.len() > 1 {
        formas = formas.chunks(2).map(|par| if let [a, b] = par { a.compound(b) } else { par[0].clone() }).collect();
    }
    p.forma = formas.pop();
    p
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
