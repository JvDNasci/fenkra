// SPDX-License-Identifier: LGPL-2.1-or-later
//! 3DM pelo openNURBS (B211, ADR 0090). A ponte (`fenkra-3dm`) traz cada objeto com o nome, a camada, a
//! cor e a geometria; aqui a B-rep vira forma exata do kernel (`Shape::de_fluxo_nurbs`) e a malha vira
//! forma facetada. As camadas viram a árvore (veja [`modelo_do_arquivo`]); os blocos já chegam
//! expandidos, cada um no seu lugar.

use crate::modelo::cor_de;
use crate::{texto_do_caminho, Capacidades, ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar, Provedor};
use fenkra_3dm::{Arquivo, Camada, Geometria, Objeto};
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};
use std::path::Path;

/// Separador entre a camada mãe e a filha no nome guardado em `ProdutoXde::camadas`.
pub const SEPARADOR: &str = " > ";

pub struct Provedor3dm;

fn erro_leitura(e: fenkra_3dm::Erro) -> ErroConv {
    match e {
        fenkra_3dm::Erro::Ausente(t) => ErroConv::Ausente(t),
        fenkra_3dm::Erro::Leitura(t) | fenkra_3dm::Erro::Gravacao(t) => ErroConv::Leitura(t),
    }
}

fn erro_gravacao(e: fenkra_3dm::Erro) -> ErroConv {
    match e {
        fenkra_3dm::Erro::Ausente(t) => ErroConv::Ausente(t),
        fenkra_3dm::Erro::Leitura(t) | fenkra_3dm::Erro::Gravacao(t) => ErroConv::Gravacao(t),
    }
}

/// Malha do 3DM → forma facetada, fechada em sólido quando a malha fecha.
fn forma_da_malha(vertices: &[f64], triangulos: &[u32]) -> Result<Shape, ErroConv> {
    let ponto = |i: u32| -> Option<[f64; 3]> {
        let k = i as usize * 3;
        vertices.get(k..k + 3).map(|p| [p[0], p[1], p[2]])
    };
    let mut tris = Vec::with_capacity(triangulos.len() * 3);
    for t in triangulos.chunks_exact(3) {
        if let (Some(a), Some(b), Some(c)) = (ponto(t[0]), ponto(t[1]), ponto(t[2])) {
            tris.extend(a.into_iter().chain(b).chain(c));
        }
    }
    Ok(Shape::from_mesh(&tris, true)?)
}

/// Modelo neutro de um 3DM já lido: uma raiz com o nome do arquivo, uma submontagem por camada usada (na
/// hierarquia das camadas) e, dentro dela, os objetos. Assim a camada aparece na árvore (grupo na montagem,
/// prefixo no nome do corpo numa peça) e continua em `ProdutoXde::camadas` para a gravação de volta.
/// Objeto que o kernel não monta vai para `ignoradas`, com o motivo.
pub fn modelo_do_arquivo(a: &Arquivo, nome_do_arquivo: &str) -> ModeloNeutro {
    let mut xde = ModeloXde { unidades: "mm".into(), ignoradas: a.ignorados.clone(), ..Default::default() };
    xde.produtos.push(ProdutoXde { nome: nome_do_arquivo.into(), ..Default::default() });
    xde.raizes.push(0);
    // produto de cada camada, criado na primeira vez que um objeto cai nela (ou numa filha dela)
    let mut da_camada: Vec<Option<usize>> = vec![None; a.camadas.len()];
    fn produto_da_camada(a: &Arquivo, i: usize, da_camada: &mut [Option<usize>], xde: &mut ModeloXde, profundidade: usize) -> usize {
        if let Some(p) = da_camada[i] {
            return p;
        }
        let pai = match a.camadas[i].pai {
            Some(m) if m < a.camadas.len() && m != i && profundidade < crate::modelo::PROFUNDIDADE => produto_da_camada(a, m, da_camada, xde, profundidade + 1),
            _ => 0,
        };
        let p = xde.produtos.len();
        let nome = a.camadas[i].nome.clone();
        xde.produtos.push(ProdutoXde { nome: nome.clone(), ..Default::default() });
        xde.produtos[pai].filhos.push(InstanciaXde { produto: p, nome, local: IDENTIDADE, cor: None });
        da_camada[i] = Some(p);
        p
    }
    for (i, o) in a.objetos.iter().enumerate() {
        let forma = match &o.geometria {
            Geometria::Brep(f) => Shape::de_fluxo_nurbs(f, a.tolerancia).map_err(ErroConv::from),
            Geometria::Malha { vertices, triangulos } => forma_da_malha(vertices, triangulos),
        };
        let forma = match forma {
            Ok(f) => f,
            Err(e) => {
                xde.ignoradas.push(format!("objeto que o kernel não montou ({e})"));
                continue;
            }
        };
        let nome = if o.nome.is_empty() {
            let tipo = if matches!(o.geometria, Geometria::Malha { .. }) { "Malha" } else { "Corpo" };
            format!("{tipo} {}", i + 1)
        } else {
            o.nome.clone()
        };
        let camada = o.camada.filter(|&c| c < a.camadas.len());
        let pai = camada.map_or(0, |c| produto_da_camada(a, c, &mut da_camada, &mut xde, 0));
        let p = xde.produtos.len();
        let camadas = camada.map(|c| a.caminho_da_camada(c)).into_iter().collect();
        xde.produtos.push(ProdutoXde { nome: nome.clone(), forma: Some(forma), cor: a.cor_de(o).map(cor_de), camadas, ..Default::default() });
        xde.produtos[pai].filhos.push(InstanciaXde { produto: p, nome, local: IDENTIDADE, cor: None });
    }
    ModeloNeutro { formato: Formato::Tresdm, xde }
}

fn rgb(c: [f32; 4]) -> [u8; 3] {
    [0, 1, 2].map(|i| (c[i].clamp(0., 1.) * 255.).round() as u8)
}

/// Índice da camada com o caminho dado ("Mãe > Filha"), criando as que faltam com a cor da peça.
fn camada(camadas: &mut Vec<Camada>, caminho: &str, cor: [u8; 3]) -> usize {
    let mut pai = None;
    let mut atual = 0;
    for nome in caminho.split(SEPARADOR) {
        atual = match camadas.iter().position(|c| c.nome == nome && c.pai == pai) {
            Some(i) => i,
            None => {
                camadas.push(Camada { nome: nome.into(), cor, pai, visivel: true });
                camadas.len() - 1
            }
        };
        pai = Some(atual);
    }
    atual
}

/// Arquivo 3DM do modelo: cada peça no seu lugar, com a camada e a cor. Devolve junto os avisos do
/// openNURBS sobre B-reps que ele não considera válidas.
pub fn arquivo_do_modelo(modelo: &ModeloNeutro) -> Result<Arquivo, ErroConv> {
    let falha = |e: fenkra_kernel::KernelError| ErroConv::Gravacao(e.0);
    let mut a = Arquivo { tolerancia: 0.001, ..Default::default() };
    for c in modelo.ocorrencias() {
        let p = &modelo.xde.produtos[c.produto];
        let Some(forma) = p.forma.as_ref() else { continue };
        let posta = forma.com_matriz(&c.local).map_err(falha)?;
        let cor = c.cor.or(p.cor).map(rgb);
        let camada = p.camadas.iter().find(|n| !n.is_empty()).map(|n| camada(&mut a.camadas, n, cor.unwrap_or([0, 0, 0])));
        let nome = c.caminho.last().cloned().unwrap_or_else(|| p.nome.clone());
        a.objetos.push(Objeto { nome, camada, cor, geometria: Geometria::Brep(posta.fluxo_nurbs().map_err(falha)?), aproximado: false });
    }
    if a.objetos.is_empty() {
        return Err(ErroConv::Gravacao("o modelo não tem forma para gravar".into()));
    }
    Ok(a)
}

/// Grava e devolve os avisos do openNURBS.
pub fn gravar_com_avisos(modelo: &ModeloNeutro, caminho: &Path) -> Result<Vec<String>, ErroConv> {
    let a = arquivo_do_modelo(modelo)?;
    fenkra_3dm::gravar(caminho, &a).map_err(erro_gravacao)
}

impl Provedor for Provedor3dm {
    fn id(&self) -> &'static str {
        "openNURBS"
    }

    fn formatos(&self) -> &'static [Formato] {
        &[Formato::Tresdm]
    }

    fn capacidades(&self, _: Formato) -> Capacidades {
        let ok = fenkra_3dm::disponivel();
        // a unidade é declarada no arquivo e a leitura converte para milímetros
        Capacidades { ler: ok, gravar: ok, exata: ok, cor: ok, camadas: ok, unidade: ok, ..Default::default() }
    }

    fn ler(&self, _: Formato, caminho: &Path, _: &OpcoesImportar) -> Result<ModeloNeutro, ErroConv> {
        texto_do_caminho(caminho)?;
        let a = fenkra_3dm::ler(caminho).map_err(erro_leitura)?;
        let nome = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        Ok(modelo_do_arquivo(&a, &nome))
    }

    fn gravar(&self, _: Formato, modelo: &ModeloNeutro, caminho: &Path, _: &OpcoesExportar) -> Result<(), ErroConv> {
        gravar_com_avisos(modelo, caminho).map(|_| ())
    }
}
