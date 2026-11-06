// SPDX-License-Identifier: LGPL-2.1-or-later
//! Sonda da matriz de formatos (B213): em vez de acreditar no que o provedor declara, grava um modelo de
//! prova e olha o que voltou. O modelo tem de tudo: duas instâncias da mesma peça, cor, camada e uma
//! medida gravada em polegadas. Um teste compara a sonda com a declaração de cada formato.
//!
//! PMI fica de fora da prova: o documento ainda não recebe PMI do STEP, então nenhum formato leva. Para
//! só gravar, a prova abre o arquivo e procura as marcas do formato.

use crate::arvore::exportar_montagem;
use crate::{Capacidades, ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Registro, UnidadeSaida};
use fenkra_core::Document;
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};
use std::path::Path;

/// Medida da peça de prova.
const LADO: f64 = 20.;
const CAMADA: &str = "Sonda";

/// Uma caixa usada três vezes, com cor e camada: duas num subconjunto e uma ao lado dele. Sem o
/// subconjunto, quem só repete a peça passaria por quem guarda a árvore.
fn modelo_de_prova() -> Result<ModeloNeutro, ErroConv> {
    let forma = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [LADO; 3])?;
    let peca = ProdutoXde { nome: "Peca".into(), forma: Some(forma), cor: Some([0.8, 0.2, 0.2, 1.]), camadas: vec![CAMADA.into()], ..Default::default() };
    let longe = [1., 0., 0., 100., 0., 1., 0., 0., 0., 0., 1., 0.];
    let mais_longe = [1., 0., 0., 300., 0., 1., 0., 0., 0., 0., 1., 0.];
    let filhos = vec![InstanciaXde { produto: 0, nome: "A".into(), local: IDENTIDADE, cor: None }, InstanciaXde { produto: 0, nome: "B".into(), local: longe, cor: None }];
    let sub = ProdutoXde { nome: "Sub".into(), filhos, ..Default::default() };
    let filhos = vec![InstanciaXde { produto: 1, nome: "S".into(), local: IDENTIDADE, cor: None }, InstanciaXde { produto: 0, nome: "C".into(), local: mais_longe, cor: None }];
    let topo = ProdutoXde { nome: "Conjunto".into(), filhos, ..Default::default() };
    Ok(ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![peca, sub, topo], raizes: vec![2], ..Default::default() } })
}

fn com_forma(m: &ModeloNeutro) -> Vec<&Shape> {
    m.xde.produtos.iter().filter_map(|p| p.forma.as_ref()).collect()
}

/// O que voltou de um formato que se lê.
fn medir(lido: &ModeloNeutro) -> Result<Capacidades, ErroConv> {
    let formas = com_forma(lido);
    // uma caixa exata tem 6 faces; facetada, 12 triângulos. Conta por sólido: quem não tem árvore junta as duas
    let forma = formas.first();
    let solidos = forma.map(|f| f.solidos().map(|s| s.len())).transpose()?.unwrap_or(0).max(1) as u32;
    let faces = forma.map(|f| f.face_count()).unwrap_or(0) / solidos;
    let cor = lido.xde.produtos.iter().any(|p| p.cor.is_some() || !p.cores_faces.is_empty() || p.filhos.iter().any(|f| f.cor.is_some()));
    let camadas = lido.xde.produtos.iter().any(|p| p.camadas.iter().any(|c| c == CAMADA));
    let aninhada = lido.xde.produtos.iter().any(|p| p.filhos.iter().any(|f| lido.xde.produtos.get(f.produto).is_some_and(|q| !q.filhos.is_empty())));
    let montagem = aninhada && lido.ocorrencias().len() == 3 && formas.len() == 1;
    // a peça foi gravada em polegadas: quem declara a unidade traz a medida de volta
    let medida = formas.first().and_then(|f| f.bbox()).map(|(a, b)| b[1] - a[1]).unwrap_or(0.);
    let unidade = !lido.xde.unidades.is_empty() && (medida - LADO).abs() < 1e-3;
    Ok(Capacidades { exata: faces == 6, malha: faces > 6, cor, camadas, montagem, unidade, ..Default::default() })
}

/// O que foi gravado num formato que não se lê: procura as marcas no texto.
fn inspecionar(caminho: &Path, formato: Formato) -> Result<Capacidades, ErroConv> {
    let bytes = std::fs::read(caminho).map_err(|e| ErroConv::Leitura(e.to_string()))?;
    let aberto = if formato == Formato::Usdz { crate::malha::zip::abrir(&bytes, |n| n.ends_with(".usda"))?.unwrap_or_default() } else { bytes };
    let t = String::from_utf8_lossy(&aberto);
    Ok(Capacidades {
        malha: t.contains("point3f[] points"),
        cor: t.contains("primvars:displayColor") && t.contains("UsdPreviewSurface"),
        montagem: t.matches("def Mesh").count() == 1 && t.matches("references = ").count() == 3 && t.contains("def Xform \"S\""),
        unidade: t.contains("metersPerUnit = 0.0254"),
        ..Default::default()
    })
}

/// Formatos que guardam receita: passa por documento e montagem, e não pelo modelo neutro.
fn sondar_arvore(registro: &Registro, pasta: &Path) -> Result<Capacidades, ErroConv> {
    use fenkra_core::Unit;
    use fenkra_montagem::{Assembly, NodeKind};
    let mut d = Document::new("Sonda");
    d.add_param("largura", "20", Unit::Comprimento).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
    d.collectors[0].color = [200, 40, 40];
    let caminho = pasta.join("sonda.fkx");
    registro.gravar_documento(&d, &caminho, &OpcoesExportar::default())?;
    let lido = registro.importar(&caminho, &OpcoesImportar::default())?;
    let peca = pasta.join("sonda.fkp");
    std::fs::write(&peca, d.to_json()).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
    let mut a = Assembly::new("Conjunto");
    a.add_component("Peça", NodeKind::Part { path: "sonda.fkp".into() }, Default::default(), None);
    let arquivo = exportar_montagem(&a, pasta)?;
    Ok(Capacidades {
        ler: true,
        gravar: true,
        cor: lido.doc.collectors.first().is_some_and(|k| k.color == [200, 40, 40]),
        montagem: arquivo.montagem.is_some(),
        parametros: lido.doc.params.iter().any(|p| p.name == "largura"),
        unidade: arquivo.unidades == "mm",
        ..Default::default()
    })
}

impl Registro {
    /// Mede o que o formato faz de verdade, gravando e lendo em `pasta`.
    pub fn sondar(&self, formato: Formato, pasta: &Path) -> Result<Capacidades, ErroConv> {
        if formato == Formato::Arvore {
            return sondar_arvore(self, pasta);
        }
        if formato.e_2d() {
            return crate::plano::sondar(self, formato, pasta);
        }
        let caminho = pasta.join(format!("sonda.{}", formato.extensoes()[0]));
        let saida = OpcoesExportar { unidade: UnidadeSaida::Polegada, ..Default::default() };
        // quem não tem árvore recebe a mesma prova: montagem é uma das coisas medidas
        let gravou = self.gravar(&modelo_de_prova()?, &caminho, &saida).is_ok() && caminho.is_file();
        let lido = self.provedor(formato)?.ler(formato, &caminho, &OpcoesImportar::default());
        Ok(match lido {
            Ok(m) => Capacidades { ler: true, gravar: gravou, ..medir(&m)? },
            Err(_) if gravou => Capacidades { gravar: true, ..inspecionar(&caminho, formato)? },
            Err(_) => Capacidades::default(),
        })
    }
}
