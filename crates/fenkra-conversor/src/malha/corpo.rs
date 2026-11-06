// SPDX-License-Identifier: LGPL-2.1-or-later
//! Malha que entra e sai como corpo de malha (B170): sem passar pelo kernel e sem teto de triângulos.
//!
//! Cada ocorrência da cena vira um coletor com o comando de importar e a malha embutida. O B-rep
//! continua disponível por `malha_em_solido` e nas montagens que cabem nele, porque peça de montagem
//! ainda é documento com sólido.

use super::{ler_cena, Cena, Malha};
use crate::importar::o_que_fica_fora;
use crate::relatorio::{Nota, AV_EXTENSAO, AV_MALHA_SEM_UNIDADE, AV_ESCALA, AV_GRANDE, AV_PEQUENA, FORA_MONTAGEM};
use crate::{ErroConv, Formato, Importado, ModeloNeutro, OpcoesExportar, OpcoesImportar, Previa, Registro, Relatorio, Suspeita};
use fenkra_core::{Document, FeatureKind, MeshImport, SeloOrigem};
use fenkra_kernel::{ModeloXde, version};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Células da grade da miniatura no maior eixo.
const CELULAS_MINIATURA: usize = 96;
/// Até aqui a miniatura usa a malha como está.
const TRIS_MINIATURA: usize = 60_000;

/// Esta importação fica corpo de malha? Montagem que cabe no B-rep continua pelo caminho do conjunto,
/// e quem pede sólido também; montagem acima do teto entra achatada, em vez de ser recusada.
pub(crate) fn vai_como_corpo(formato: Formato, caminho: &Path, o: &OpcoesImportar) -> Result<bool, ErroConv> {
    if !formato.e_malha() || o.malha_em_solido || !Registro::default().capacidades(formato).ler {
        return Ok(false);
    }
    if o.destino != crate::Destino::Montagem || !Registro::default().capacidades(formato).montagem {
        return Ok(true);
    }
    let cena = ler_cena(formato, caminho)?;
    let (ocorrencias, _) = cena.ocorrencias();
    let mut usadas: Vec<usize> = ocorrencias.iter().map(|(_, m, _)| *m).collect();
    usadas.sort_unstable();
    usadas.dedup();
    let triangulos: usize = usadas.iter().map(|i| cena.malhas[*i].tris.len()).sum();
    Ok(ocorrencias.len() < 2 || triangulos > super::LIMITE_BREP)
}

/// Malhas de cada lugar da árvore. Um nó com vários materiais é um corpo só, com a cor da maior malha.
fn corpos_da_cena(cena: &Cena) -> (Vec<(String, Malha, Option<[f32; 4]>)>, usize) {
    let (ocorrencias, _) = cena.ocorrencias();
    // nós sem nome têm o mesmo caminho: o que junta duas malhas num corpo é serem do mesmo nó
    let dono: std::collections::HashMap<usize, usize> = cena.nos.iter().enumerate().rev().flat_map(|(i, n)| n.malhas.iter().map(move |m| (*m, i))).collect();
    let mut grupos: Vec<(Vec<String>, [f64; 12], Vec<usize>)> = Vec::new();
    for (caminho, malha, local) in ocorrencias {
        match grupos.last_mut() {
            Some((c, l, m)) if *c == caminho && *l == local && dono.get(&m[0]) == dono.get(&malha) => m.push(malha),
            _ => grupos.push((caminho, local, vec![malha])),
        }
    }
    let mut corpos = Vec::new();
    let mut varias_cores = 0;
    for (caminho, local, malhas) in grupos {
        let (mut maior, mut cor) = (-1., None);
        let mut postas = Vec::with_capacity(malhas.len());
        for &i in &malhas {
            let m = &cena.malhas[i];
            let area = m.area();
            if area > maior {
                (maior, cor) = (area, cena.cor_de(m));
            }
            let mut p = Malha { pos: m.pos.clone(), tris: m.tris.clone(), ..Default::default() };
            p.pos.iter_mut().for_each(|q| *q = fenkra_malha::cena::aplicar(&local, *q));
            if fenkra_malha::cena::determinante(&local) < 0. {
                p.tris.iter_mut().for_each(|t| t.swap(1, 2));
            }
            postas.push(p);
        }
        if malhas.len() > 1 {
            let cores: Vec<_> = malhas.iter().map(|i| cena.cor_de(&cena.malhas[*i]).map(|c| c.map(f32::to_bits))).collect();
            varias_cores += usize::from(cores.windows(2).any(|w| w[0] != w[1]));
        }
        let trecho = if caminho.len() > 1 { &caminho[1..] } else { &caminho[..] };
        let nome = trecho.join(" / ");
        let junta = fenkra_malha::juntar(&postas).soldada();
        // várias cascas fechadas viram um corpo cada, como os sólidos; cavidade fica com a peça dela
        let partes = junta.componentes();
        let externo = partes.iter().map(|p| p.volume()).fold(0f64, |a, v| if v.abs() > a.abs() { v } else { a });
        let separaveis = partes.len() > 1 && partes.iter().all(|p| p.fechada() && p.volume().signum() == externo.signum());
        if separaveis {
            corpos.extend(partes.into_iter().enumerate().map(|(i, p)| (format!("{nome} {}", i + 1).trim().to_string(), p, cor)));
        } else {
            corpos.push((nome, junta, cor));
        }
    }
    (corpos, varias_cores)
}

fn rgb(c: [f32; 4]) -> [u8; 3] {
    [c[0], c[1], c[2]].map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
}

fn miniatura(corpos: &[(String, Malha, Option<[f32; 4]>)], r: &Relatorio) -> Previa {
    let centro = r.caixa.map(|(a, b)| [0, 1, 2].map(|i| (a[i] + b[i]) / 2.)).unwrap_or([0.; 3]);
    let total: usize = corpos.iter().map(|c| c.1.tris.len()).sum();
    let mut p = Previa::default();
    for (_, m, cor) in corpos {
        let leve;
        let m = if total > TRIS_MINIATURA {
            leve = m.grosseira(CELULAS_MINIATURA);
            p.incompleta = true;
            &leve
        } else {
            m
        };
        let cor = cor.map(rgb);
        // triângulos soltos: a miniatura é desenhada com a normal da face
        for t in &m.tris {
            let [a, b, c] = t.map(|i| m.pos[i as usize]);
            let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-300);
            let base = p.pos.len() as u32;
            for q in [a, b, c] {
                p.pos.push([0, 1, 2].map(|i| (q[i] - centro[i]) as f32));
                p.nrm.push(n.map(|x| (x / l) as f32));
                p.cor.push(cor);
            }
            p.tris.push([base, base + 1, base + 2]);
        }
    }
    p
}

/// Lê a cena e monta o documento com um corpo de malha por ocorrência.
pub(crate) fn importar(formato: Formato, caminho: &Path, bytes: &[u8], texto: &str, o: &OpcoesImportar, extensao_diverge: bool) -> Result<Importado, ErroConv> {
    let mut cena = ler_cena(formato, caminho)?;
    let unidades = cena.unidade.clone();
    let nome_arquivo = caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let nome_doc = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Importado".into());
    let hash = blake3::hash(bytes).to_hex().to_string();
    let mut r = Relatorio {
        arquivo: nome_arquivo.clone(),
        hash: hash.clone(),
        tamanho: bytes.len() as u64,
        formato: formato.nome().into(),
        provedor: "Fenkra".into(),
        versao: format!("Fenkra {} / {}", env!("CARGO_PKG_VERSION"), version()),
        esquema: formato.nome().into(),
        unidades: unidades.clone(),
        escala: 1.,
        reparo: o.reparo.nome().into(),
        ..Default::default()
    };
    if extensao_diverge {
        r.avisos.push(Nota::nova(AV_EXTENSAO, &[formato.nome().to_string()]));
    }
    if unidades.is_empty() && o.unidade.em_mm().is_none() {
        r.avisos.push(Nota::nova(AV_MALHA_SEM_UNIDADE, &[formato.nome().to_string()]));
    }
    if let Some(mm) = o.unidade.em_mm() {
        r.escala = mm / crate::importar::declarada_em_mm(&unidades);
        if (r.escala - 1.).abs() > 1e-12 {
            cena.escalar(r.escala);
            r.avisos.push(Nota::nova(AV_ESCALA, &[format!("{}", r.escala)]));
        }
    }
    if !o.cores {
        cena.sem_cores();
    }
    cena.assentar();
    cena.enxugar();
    let (ocorrencias, inteiro) = cena.ocorrencias();
    if !inteiro {
        return Err(super::erro(formato.nome(), "árvore com ciclo, funda demais ou com ocorrências demais"));
    }
    let achatada = ocorrencias.len() > 1 && Registro::default().capacidades(formato).montagem;
    let (corpos, varias_cores) = corpos_da_cena(&cena);
    if corpos.is_empty() {
        return Err(ErroConv::Leitura(format!("o arquivo {} não tem triângulos", formato.nome())));
    }
    let selo = SeloOrigem { nome: nome_arquivo, hash, data: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0), esquema: formato.nome().into(), unidades: unidades.clone(), derivado_de: None, relatorio: None };

    let mut doc = Document::new(&nome_doc);
    let primeiro = doc.active_collector;
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for (i, (nome, malha, cor)) in corpos.iter().enumerate() {
        let nome = if !o.nomes || nome.trim_matches([' ', '/']).is_empty() { format!("Corpo {}", i + 1) } else { nome.clone() };
        let coletor = if i == 0 { primeiro } else { doc.add_collector(&nome) };
        doc.active_collector = coletor;
        if let Some(k) = doc.collectors.iter_mut().find(|k| k.id == coletor) {
            k.name = nome.clone();
            if let Some(c) = cor {
                k.color = rgb(*c);
            }
        }
        let dados = fenkra_solidos::corpo_malha::embutir(&mut doc, malha);
        doc.add_feature(FeatureKind::MeshImport(MeshImport { path: texto.into(), solid: false, corpo: true, dados }));
        let d = malha.diagnostico();
        r.faces += d.triangulos as u32;
        r.area += d.area;
        if d.arestas_livres == 0 && d.arestas_nao_manifold == 0 {
            r.solidos += 1;
            r.volume += d.volume.abs();
        } else {
            r.nao_fecharam.push(nome);
        }
        if let Some((a, b)) = malha.caixa() {
            lo = [0, 1, 2].map(|k| lo[k].min(a[k]));
            hi = [0, 1, 2].map(|k| hi[k].max(b[k]));
        }
    }
    doc.active_collector = primeiro;
    doc.origem = Some(selo);
    r.caixa = Some((lo, hi));
    r.corpos = corpos.len();
    r.produtos = corpos.len();
    r.instancias = corpos.len();
    if let Some(m) = r.medidas() {
        let maior = m.into_iter().fold(0., f64::max);
        match crate::unidade_suspeita(m) {
            Some(Suspeita::Grande) => r.avisos.push(Nota::nova(AV_GRANDE, &[format!("{maior:.1}")])),
            Some(Suspeita::Pequena) => r.avisos.push(Nota::nova(AV_PEQUENA, &[format!("{maior:.4}")])),
            None => {}
        }
    }
    let mut ignorados = cena.ignorados.clone();
    if varias_cores > 0 {
        ignorados.insert("peça com vários materiais, que ficou com a cor do maior".into(), varias_cores);
    }
    let ignoradas = ignorados.iter().map(|(oque, n)| format!("{oque}\t{n}")).collect();
    // só o que o relatório lê do modelo: formato e o que o leitor deixou de fora
    let casca = ModeloNeutro { formato, xde: ModeloXde { ignoradas, ..Default::default() } };
    o_que_fica_fora(&casca, false, &mut r);
    if achatada {
        r.nao_carrega.push(Nota::nova(FORA_MONTAGEM, &[]));
    }
    let embutido = serde_json::to_string(&r).ok();
    if let Some(s) = &mut doc.origem {
        s.relatorio = embutido;
    }
    let previa = miniatura(&corpos, &r);
    Ok(Importado { doc, relatorio: r, previa, conjunto: None })
}

/// Documento com corpo de malha para um formato de malha: os corpos B-rep descem pela tesselação das
/// opções, e os de malha saem como estão.
pub(crate) fn gravar_documento(doc: &Document, formato: Formato, destino: &Path, saida: &OpcoesExportar) -> Result<(), ErroConv> {
    let corpos = fenkra_solidos::corpo_malha::malhas_do_documento(doc, saida.malha.desvio.max(1e-4), saida.malha.angulo.max(1.).to_radians()).map_err(ErroConv::Gravacao)?;
    let mut cena = Cena { nome: doc.name.clone(), ..Default::default() };
    for (_, nome, cor, malha) in corpos {
        let mut m = malha;
        m.nome = nome;
        if saida.cores {
            let c = crate::modelo::cor_de(cor);
            let nome = format!("Cor {}", cena.materiais.len() + 1);
            m.material = Some(cena.material(&nome, c));
        }
        cena.raiz_com(m);
    }
    if cena.malhas.iter().all(|m| m.tris.is_empty()) {
        return Err(ErroConv::Gravacao("o documento não tem malha para gravar".into()));
    }
    super::gravar_cena(formato, &cena, destino, saida)
}
