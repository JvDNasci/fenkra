// SPDX-License-Identifier: LGPL-2.1-or-later
//! Do modelo neutro ao documento. Cada peça, em cada lugar em que aparece, vira um corpo com a
//! geometria embutida: o documento abre sem o arquivo de origem.
//!
//! O que não é sólido fica como superfície de referência. O comando de corpo ligado tamparia as bordas
//! abertas e inventaria um volume que o arquivo não tem, justo o que o relatório diz que não fechou.

use crate::{ErroConv, ModeloNeutro, Ocorrencia};
use fenkra_core::{Appearance, Document, FeatureKind, Finish, LinkedBody, LinkedGeo, SeloOrigem};
use fenkra_kernel::Shape;

/// Corpos de uma ocorrência. Peça com vários sólidos e nada solto se divide em um corpo por sólido;
/// com sobras abertas, fica inteira para o relatório e a geometria baterem.
fn corpos_da(forma: &Shape, c: &Ocorrencia) -> Result<Vec<Shape>, ErroConv> {
    let posta = forma.com_matriz(&c.local)?;
    let solidos = posta.solidos()?;
    let (_, info) = posta.reparar(0, 0.)?;
    Ok(if solidos.len() > 1 && info.abertas == 0 && info.soltas == 0 { solidos } else { vec![posta] })
}

fn rgb(c: [f32; 4]) -> [u8; 3] {
    [c[0], c[1], c[2]].map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
}

/// Nome do corpo: o caminho sem a raiz, que já é o nome do documento.
fn nome_do_corpo(c: &Ocorrencia, parte: usize, partes: usize) -> String {
    let caminho = if c.caminho.len() > 1 { &c.caminho[1..] } else { &c.caminho[..] };
    let nome = caminho.join(" / ");
    if partes > 1 {
        format!("{nome} {}", parte + 1)
    } else {
        nome
    }
}

/// Monta o documento com o selo de origem. Devolve também quantos corpos entraram.
pub fn montar_documento(modelo: &ModeloNeutro, nome: &str, caminho: &str, selo: SeloOrigem) -> Result<(Document, usize), ErroConv> {
    let mut doc = Document::new(nome);
    let primeiro = doc.active_collector;
    let (mut corpos, mut coletores) = (0, 0);
    for c in modelo.ocorrencias() {
        let Some(forma) = modelo.xde.produtos[c.produto].forma.as_ref() else { continue };
        let partes = corpos_da(forma, &c)?;
        for (i, parte) in partes.iter().enumerate() {
            let nome_corpo = Some(nome_do_corpo(&c, i, partes.len())).filter(|n| !n.trim_matches([' ', '/']).is_empty()).unwrap_or_else(|| format!("Corpo {}", corpos + 1));
            let geo = doc.add_link("importação", caminho, &nome_corpo, LinkedGeo::Surface(parte.to_brep()?));
            // a cópia vira geometria própria: o arquivo de fora não é mais lido a cada cálculo
            if let Some(ext) = doc.external.last().map(|e| e.id) {
                doc.isolate_link(ext);
            }
            corpos += 1;
            if !parte.is_solid() {
                if let (Some(r), Some(cor)) = (doc.references.iter_mut().find(|r| r.id == geo), c.cor) {
                    r.appearance = Some(Appearance { color: rgb(cor), finish: Finish::default(), texture: None });
                }
                continue;
            }
            let coletor = if coletores == 0 { primeiro } else { doc.add_collector(&nome_corpo) };
            coletores += 1;
            doc.active_collector = coletor;
            if let Some(k) = doc.collectors.iter_mut().find(|k| k.id == coletor) {
                k.name = nome_corpo.clone();
                if let Some(cor) = c.cor {
                    k.color = rgb(cor);
                }
            }
            doc.add_feature(FeatureKind::LinkedBody(LinkedBody { geo }));
        }
    }
    doc.active_collector = primeiro;
    doc.origem = Some(selo);
    Ok((doc, corpos))
}
