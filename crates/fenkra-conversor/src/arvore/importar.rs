// SPDX-License-Identifier: LGPL-2.1-or-later
//! Importar um arquivo de árvore pelo mesmo caminho do STEP: documento, relatório e miniatura. A
//! diferença é que aqui a geometria nasce do recálculo, e não do arquivo.

use super::*;
use crate::relatorio::*;
use crate::{ErroConv, Importado, Previa, Relatorio};
use fenkra_core::{Document, SeloOrigem};
use fenkra_solidos::{recompute, RecomputeResult, Status};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn avisos_dos_comandos(doc: &Document, r: &RecomputeResult, relatorio: &mut Relatorio) {
    for f in &doc.features {
        if let Some(Status::Error(m)) = r.status.get(&f.id) {
            relatorio.avisos.push(Nota::nova(AV_COMANDO, &[f.name.clone(), m.clone()]));
        }
    }
}

fn medir(r: &RecomputeResult, relatorio: &mut Relatorio) {
    for b in &r.bodies {
        relatorio.volume += b.mass.volume;
        relatorio.area += b.mass.area;
        relatorio.faces += b.faces;
        relatorio.solidos += 1;
        for p in b.mesh.positions.chunks_exact(3) {
            let p = [p[0] as f64, p[1] as f64, p[2] as f64];
            relatorio.caixa = Some(match relatorio.caixa {
                Some((a, c)) => ([0, 1, 2].map(|i| a[i].min(p[i])), [0, 1, 2].map(|i| c[i].max(p[i]))),
                None => (p, p),
            });
        }
    }
    relatorio.corpos = r.bodies.len();
}

/// Miniatura da peça principal, com a malha que o recálculo já fez.
fn previa(r: &RecomputeResult, caixa: Option<([f64; 3], [f64; 3])>) -> Previa {
    let mut v = Previa::default();
    let Some((a, b)) = caixa else { return v };
    let centro = [0, 1, 2].map(|i| (a[i] + b[i]) / 2.);
    for corpo in &r.bodies {
        let m = &corpo.mesh;
        if v.tris.len() + m.indices.len() / 3 > crate::importar::TRIANGULOS_DA_PREVIA {
            v.incompleta = true;
            continue;
        }
        let ponto = |q: &[f32]| [0, 1, 2].map(|i| (q[i] as f64 - centro[i]) as f32);
        let base = v.pos.len() as u32;
        for (q, n) in m.positions.chunks_exact(3).zip(m.normals.chunks_exact(3)) {
            v.pos.push(ponto(q));
            v.nrm.push([n[0], n[1], n[2]]);
            v.cor.push(Some(corpo.color));
        }
        v.tris.extend(m.indices.chunks_exact(3).map(|t| [base + t[0], base + t[1], base + t[2]]));
        for e in 0..m.edge_ranges.len() {
            if m.edge_smooth.get(e).is_some_and(|s| *s != 0) {
                continue;
            }
            let inicio = m.edge_ranges[e] as usize;
            let fim = m.edge_ranges.get(e + 1).map_or(m.edges.len() / 6, |x| *x as usize);
            v.arestas.extend(m.edges[inicio * 6..fim * 6].chunks_exact(6).map(|q| [ponto(&q[..3]), ponto(&q[3..])]));
        }
    }
    v
}

pub(crate) fn importar(caminho: &Path, bytes: &[u8], extensao_diverge: bool) -> Result<Importado, ErroConv> {
    let texto = std::str::from_utf8(bytes).map_err(|_| ErroConv::Leitura("o arquivo de árvore tem que ser UTF-8".into()))?;
    let arquivo = ler_texto(texto.trim_start_matches('\u{feff}'))?;
    let (mut conjunto, reancoragens) = remontar(&arquivo)?;

    let nome_arquivo = caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let hash = blake3::hash(bytes).to_hex().to_string();
    let esquema = format!("{NOME_FORMATO} v{}", arquivo.format_version);
    let mut relatorio = Relatorio {
        arquivo: nome_arquivo.clone(),
        hash: hash.clone(),
        tamanho: bytes.len() as u64,
        formato: crate::Formato::Arvore.nome().into(),
        provedor: arquivo.gerador.clone(),
        versao: format!("Fenkra {}", env!("CARGO_PKG_VERSION")),
        esquema: esquema.clone(),
        unidades: arquivo.unidades.clone(),
        escala: 1.,
        reparo: "Nenhum".into(),
        produtos: arquivo.pecas.len(),
        instancias: arquivo.montagem.as_ref().map_or(arquivo.pecas.len(), |m| m.componentes.iter().filter(|c| c.peca.is_some()).count()),
        ..Default::default()
    };
    if extensao_diverge {
        relatorio.avisos.push(Nota::nova(AV_EXTENSAO, &[crate::Formato::Arvore.nome().to_string()]));
    }
    for f in &arquivo.fora {
        relatorio.nao_carrega.push(Nota::nova(FORA_ARVORE, &[f.clone()]));
    }
    let reancoradas: usize = reancoragens.iter().map(|r| r.reancoradas).sum();
    if reancoradas > 0 {
        relatorio.avisos.push(Nota::nova(AV_REANCORADAS, &[reancoradas.to_string()]));
    }

    let selo = SeloOrigem {
        nome: nome_arquivo,
        hash,
        data: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        esquema,
        unidades: arquivo.unidades.clone(),
        derivado_de: None,
    };
    for (_, doc) in &mut conjunto.pecas {
        doc.origem = Some(selo.clone());
    }
    let (_, doc) = conjunto.pecas.first().cloned().ok_or_else(|| ErroConv::Leitura("arquivo sem peças".into()))?;
    let r = recompute(&doc);
    medir(&r, &mut relatorio);
    avisos_dos_comandos(&doc, &r, &mut relatorio);
    let previa = previa(&r, relatorio.caixa);
    let varias = conjunto.pecas.len() > 1 || conjunto.montagem.is_some();
    Ok(Importado { doc, relatorio, previa, conjunto: varias.then_some(conjunto) })
}
