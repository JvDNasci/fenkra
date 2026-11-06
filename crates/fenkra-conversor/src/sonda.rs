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

/// Uma caixa usada duas vezes, com cor e camada.
fn modelo_de_prova() -> Result<ModeloNeutro, ErroConv> {
    let forma = Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [LADO; 3])?;
    let peca = ProdutoXde { nome: "Peca".into(), forma: Some(forma), cor: Some([0.8, 0.2, 0.2, 1.]), camadas: vec![CAMADA.into()], ..Default::default() };
    let longe = [1., 0., 0., 100., 0., 1., 0., 0., 0., 0., 1., 0.];
    let filhos = vec![InstanciaXde { produto: 0, nome: "A".into(), local: IDENTIDADE, cor: None }, InstanciaXde { produto: 0, nome: "B".into(), local: longe, cor: None }];
    let topo = ProdutoXde { nome: "Conjunto".into(), filhos, ..Default::default() };
    Ok(ModeloNeutro { formato: Formato::Step, xde: ModeloXde { produtos: vec![peca, topo], raizes: vec![1], ..Default::default() } })
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
    let montagem = lido.tem_montagem() && lido.instancias() == 2 && formas.len() == 1;
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
        montagem: t.matches("def Mesh").count() == 1 && t.matches("references = ").count() == 2,
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

/// DXF só se lê por aqui: a prova é um círculo numa camada, com a unidade declarada.
fn sondar_dxf(registro: &Registro, pasta: &Path) -> Result<Capacidades, ErroConv> {
    use fenkra_core::sketch::Curve;
    use fenkra_core::FeatureKind;
    use fenkra_dxf::{Camada, Desenho, Entidade, Item, Unidade};
    let d = Desenho { unidade: Unidade::Milimetro, camadas: vec![Camada::nova(CAMADA, 7, "Continuous")], itens: vec![Item::novo(CAMADA, Entidade::Circulo { centro: [0., 0.], raio: LADO })], ..Default::default() };
    let caminho = pasta.join("sonda.dxf");
    std::fs::write(&caminho, fenkra_dxf::gravar(&d)).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
    let lido = registro.importar(&caminho, &OpcoesImportar::default())?;
    let esboco = lido.doc.features.iter().find_map(|f| match &f.kind {
        FeatureKind::Sketch(s) => Some((f.name.as_str(), &s.data)),
        _ => None,
    });
    let exata = esboco.is_some_and(|(_, s)| s.curves.iter().any(|c| matches!(c.geom, Curve::Circle { .. })));
    Ok(Capacidades { ler: true, exata, camadas: esboco.is_some_and(|(n, _)| n == CAMADA), unidade: lido.relatorio.avisos.is_empty(), ..Default::default() })
}

impl Registro {
    /// Mede o que o formato faz de verdade, gravando e lendo em `pasta`.
    pub fn sondar(&self, formato: Formato, pasta: &Path) -> Result<Capacidades, ErroConv> {
        if formato == Formato::Arvore {
            return sondar_arvore(self, pasta);
        }
        if formato == Formato::Dxf {
            return sondar_dxf(self, pasta);
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
