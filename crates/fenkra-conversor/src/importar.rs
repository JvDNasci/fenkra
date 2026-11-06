// SPDX-License-Identifier: LGPL-2.1-or-later
//! O caminho inteiro de uma importação: ler, ajustar a unidade, reparar, medir, montar o documento e
//! a miniatura. Roda dentro do processo filho; o resultado é só dado.

use crate::relatorio::*;
use crate::{detectar, montar_documento, texto_do_caminho, Contagem, Defeito, Destino, ErroConv, Formato, LinhaReparo, ModeloNeutro, OpcoesImportar, Registro, Relatorio, TipoDefeito};
use fenkra_core::{Document, SeloOrigem};
use fenkra_kernel::{version, InfoReparo, Xform};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Teto da miniatura: ela é reordenada e desenhada na CPU a cada quadro em que gira.
pub(crate) const TRIANGULOS_DA_PREVIA: usize = 60_000;
/// Limites do aviso de unidade: maior medida da peça, em mm.
const MAIOR_PLAUSIVEL: f64 = 10_000.;
const MENOR_PLAUSIVEL: f64 = 0.1;

/// Malha leve para a miniatura do diálogo, centrada na origem.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Previa {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    /// cor de cada vértice; `None` usa a cor do tema
    pub cor: Vec<Option<[u8; 3]>>,
    pub tris: Vec<[u32; 3]>,
    pub arestas: Vec<[[f32; 3]; 2]>,
    /// faltou espaço: algumas peças ficaram fora da miniatura
    pub incompleta: bool,
}

/// Resultado de ler um arquivo, pronto para a pessoa confirmar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Importado {
    pub doc: Document,
    pub relatorio: Relatorio,
    pub previa: Previa,
    /// arquivo de árvore com várias peças ou montagem: `doc` é a primeira, e o resto vem aqui
    #[serde(default)]
    pub conjunto: Option<crate::arvore::Conjunto>,
}

/// Por que a unidade do arquivo parece errada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suspeita {
    Grande,
    Pequena,
}

/// Peça com mais de 10 000 mm ou menos de 0,1 mm costuma ser unidade errada, não peça assim.
pub fn unidade_suspeita(medidas: [f64; 3]) -> Option<Suspeita> {
    let maior = medidas.into_iter().fold(0., f64::max);
    if maior > MAIOR_PLAUSIVEL {
        Some(Suspeita::Grande)
    } else if maior < MENOR_PLAUSIVEL {
        Some(Suspeita::Pequena)
    } else {
        None
    }
}

/// Milímetros por unidade declarada. O leitor já converteu para milímetros; o fator desfaz isso
/// quando a pessoa diz que a declaração está errada.
pub(crate) fn declarada_em_mm(unidades: &str) -> f64 {
    let u = unidades.to_uppercase();
    if u.contains("INCH") || u == "IN" {
        25.4
    } else if u.contains("FOOT") || u.contains("FEET") {
        304.8
    } else if u.contains("MICRO") {
        0.001
    } else if u.contains("CENTI") || u == "CM" {
        10.
    } else if u.contains("MILLI") || u == "MM" {
        1.
    } else if u.contains("METRE") || u.contains("METER") || u == "M" {
        1000.
    } else {
        1.
    }
}

fn escalar(modelo: &mut ModeloNeutro, k: f64) -> Result<(), ErroConv> {
    for p in &mut modelo.xde.produtos {
        if let Some(f) = &p.forma {
            p.forma = Some(f.transformed(Xform::Scale([0.; 3], k))?);
        }
        for filho in &mut p.filhos {
            for i in [3, 7, 11] {
                filho.local[i] *= k;
            }
        }
    }
    Ok(())
}

fn antes(i: &InfoReparo) -> Contagem {
    Contagem {
        faces: i.faces_antes,
        faces_ruins: i.faces_ruins_antes,
        solidos: i.solidos_antes,
        abertas: i.abertas_antes,
        soltas: i.soltas_antes,
        arestas_livres: i.arestas_livres_antes,
        tolerancia: i.tolerancia_antes,
    }
}

fn depois(i: &InfoReparo) -> Contagem {
    Contagem { faces: i.faces, faces_ruins: i.faces_ruins, solidos: i.solidos, abertas: i.abertas, soltas: i.soltas, arestas_livres: i.arestas_livres, tolerancia: i.tolerancia }
}

/// Repara cada definição uma vez, não cada ocorrência, e anota o que restou aberto.
fn reparar(modelo: &mut ModeloNeutro, o: &OpcoesImportar, r: &mut Relatorio) -> Result<(), ErroConv> {
    for p in &mut modelo.xde.produtos {
        let Some(f) = &p.forma else { continue };
        let (novo, info) = f.reparar(o.reparo.nivel(), o.tolerancia)?;
        r.faces_reparadas += info.faces_tocadas;
        r.faces_costuradas += info.faces_costuradas;
        r.faces_ruins += info.faces_ruins;
        let (a, d) = (antes(&info), depois(&info));
        r.antes.somar(&a);
        r.depois.somar(&d);
        if a.tem_defeito() || d.tem_defeito() {
            r.reparo_pecas.push(LinhaReparo { peca: p.nome.clone(), antes: a, depois: d });
        }
        if info.abertas > 0 || info.soltas > 0 {
            r.nao_fecharam.push(p.nome.clone());
        }
        if info.faces != info.faces_antes && !p.cores_faces.is_empty() {
            p.cores_faces.clear();
            r.avisos.push(Nota::nova(AV_CORES_REPARO, &[p.nome.clone()]));
        }
        p.forma = Some(novo);
    }
    Ok(())
}

/// Lista o que sobrou depois do reparo, cada defeito no lugar da primeira ocorrência da peça. Peça usada
/// N vezes tem o mesmo defeito nas N: repetir só encheria a lista.
fn listar_defeitos(modelo: &ModeloNeutro, r: &mut Relatorio) -> Result<(), ErroConv> {
    let mut vistos = std::collections::HashSet::new();
    let total: u32 = r.reparo_pecas.iter().map(|l| l.depois.faces_ruins + l.depois.soltas + l.depois.arestas_livres).sum();
    for c in modelo.ocorrencias() {
        let produto = &modelo.xde.produtos[c.produto];
        let Some(f) = produto.forma.as_ref() else { continue };
        let resta = LIMITE_DEFEITOS.saturating_sub(r.defeitos.len() as u32);
        if resta == 0 || !vistos.insert(c.produto) {
            continue;
        }
        for d in f.defeitos(resta)? {
            let tipo = match d.tipo {
                fenkra_kernel::TipoDefeito::FaceInvalida => TipoDefeito::FaceInvalida,
                fenkra_kernel::TipoDefeito::ArestaLivre => TipoDefeito::ArestaLivre,
                fenkra_kernel::TipoDefeito::FaceSolta => TipoDefeito::FaceSolta,
            };
            let segmentos = d.segmentos.iter().map(|s| s.map(|p| aplicar(&c.local, p, true).map(|v| v as f32))).collect();
            r.defeitos.push(Defeito { tipo, peca: produto.nome.clone(), indice: d.indice, segmentos });
        }
    }
    if total > r.defeitos.len() as u32 {
        r.avisos.push(Nota::nova(AV_DEFEITOS_FORA, &[r.defeitos.len().to_string(), total.to_string()]));
    }
    Ok(())
}

fn medir(modelo: &ModeloNeutro, r: &mut Relatorio) -> Result<(), ErroConv> {
    for c in modelo.ocorrencias() {
        let Some(f) = modelo.xde.produtos[c.produto].forma.as_ref() else { continue };
        let posta = f.com_matriz(&c.local)?;
        let m = f.mass();
        r.volume += m.volume;
        r.area += m.area;
        r.faces += f.face_count();
        r.solidos += posta.solidos()?.len() as u32;
        if let Some((a, b)) = posta.bbox() {
            r.caixa = Some(match r.caixa {
                Some((x, y)) => ([0, 1, 2].map(|i| x[i].min(a[i])), [0, 1, 2].map(|i| y[i].max(b[i]))),
                None => (a, b),
            });
        }
    }
    r.produtos = modelo.xde.produtos.len();
    r.instancias = modelo.instancias();
    Ok(())
}

fn o_que_fica_fora(modelo: &ModeloNeutro, conjunto: bool, r: &mut Relatorio) {
    let xde = &modelo.xde;
    if modelo.formato == Formato::Brep {
        r.nao_carrega.push(Nota::nova(FORA_BREP, &[]));
        r.nao_carrega.push(Nota::nova(FORA_BREP_UNIDADE, &[]));
    }
    if xde.esquema.to_uppercase().contains("CONFIG_CONTROL") {
        r.nao_carrega.push(Nota::nova(FORA_AP203, &[]));
    }
    if xde.pmi != [0, 0, 0] {
        r.nao_carrega.push(Nota::nova(FORA_PMI, &xde.pmi.map(|n| n.to_string())));
    }
    let faces: usize = xde.produtos.iter().map(|p| p.cores_faces.len()).sum();
    if faces > 0 {
        r.nao_carrega.push(Nota::nova(FORA_COR_FACE, &[faces.to_string()]));
    }
    let mut camadas: Vec<&str> = xde.produtos.iter().flat_map(|p| p.camadas.iter().map(String::as_str)).collect();
    camadas.sort_unstable();
    camadas.dedup();
    if !camadas.is_empty() {
        r.nao_carrega.push(Nota::nova(FORA_CAMADAS, &[camadas.join(", ")]));
    }
    if modelo.tem_montagem() && !conjunto {
        r.nao_carrega.push(Nota::nova(FORA_MONTAGEM, &[]));
    }
    if modelo.formato.e_malha() {
        for item in &xde.ignoradas {
            let (oque, n) = item.split_once('\t').unwrap_or((item, "1"));
            r.nao_carrega.push(Nota::nova(FORA_MALHA_ITENS, &[oque.to_string(), n.to_string()]));
        }
    } else if !xde.ignoradas.is_empty() {
        let mut contagem = std::collections::BTreeMap::new();
        for tipo in &xde.ignoradas {
            *contagem.entry(tipo.as_str()).or_insert(0usize) += 1;
        }
        let lista: Vec<String> = contagem.iter().map(|(tipo, n)| format!("{tipo} ({n})")).collect();
        r.nao_carrega.push(Nota::nova(FORA_IGES, &[lista.join(", ")]));
    }
    if modelo.formato.e_malha() {
        r.nao_carrega.push(Nota::nova(FORA_MALHA, &[modelo.formato.nome().to_string()]));
    }
    if modelo.formato == Formato::Iges || modelo.formato.e_malha() {
        for p in r.nao_fecharam.clone() {
            r.avisos.push(Nota::nova(AV_SUPERFICIES, &[p]));
        }
    }
}

fn aplicar(m: &[f64; 12], p: [f64; 3], deslocar: bool) -> [f64; 3] {
    let t = if deslocar { 1. } else { 0. };
    [0, 1, 2].map(|i| m[i * 4] * p[0] + m[i * 4 + 1] * p[1] + m[i * 4 + 2] * p[2] + t * m[i * 4 + 3])
}

/// Miniatura: malha grosseira de cada definição, repetida em cada lugar em que ela aparece.
fn previa(modelo: &ModeloNeutro, r: &Relatorio) -> Result<Previa, ErroConv> {
    let mut v = Previa::default();
    let Some((a, b)) = r.caixa else { return Ok(v) };
    let centro = [0, 1, 2].map(|i| (a[i] + b[i]) / 2.);
    let diagonal = (0..3).map(|i| (b[i] - a[i]).powi(2)).sum::<f64>().sqrt();
    let desvio = (diagonal / 120.).max(1e-4);
    let mut malhas = std::collections::HashMap::new();
    for c in modelo.ocorrencias() {
        let Some(f) = modelo.xde.produtos[c.produto].forma.as_ref() else { continue };
        if !malhas.contains_key(&c.produto) {
            malhas.insert(c.produto, f.mesh(desvio)?);
        }
        let m = &malhas[&c.produto];
        if v.tris.len() + m.indices.len() / 3 > TRIANGULOS_DA_PREVIA {
            v.incompleta = true;
            continue;
        }
        let cor = c.cor.map(|k| [k[0], k[1], k[2]].map(|x| (x.clamp(0., 1.) * 255.) as u8));
        let base = v.pos.len() as u32;
        let ponto = |q: &[f32]| {
            let p = aplicar(&c.local, [q[0] as f64, q[1] as f64, q[2] as f64], true);
            [0, 1, 2].map(|i| (p[i] - centro[i]) as f32)
        };
        for (q, n) in m.positions.chunks_exact(3).zip(m.normals.chunks_exact(3)) {
            v.pos.push(ponto(q));
            v.nrm.push(aplicar(&c.local, [n[0] as f64, n[1] as f64, n[2] as f64], false).map(|x| x as f32));
            v.cor.push(cor);
        }
        v.tris.extend(m.indices.chunks_exact(3).map(|t| [base + t[0], base + t[1], base + t[2]]));
        let segmentos = m.edges.len() / 6;
        for e in 0..m.edge_ranges.len() {
            if m.edge_smooth.get(e).is_some_and(|s| *s != 0) {
                continue;
            }
            let inicio = m.edge_ranges[e] as usize;
            let fim = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(segmentos);
            v.arestas.extend(m.edges[inicio * 6..fim * 6].chunks_exact(6).map(|q| [ponto(&q[..3]), ponto(&q[3..])]));
        }
    }
    Ok(v)
}

impl Registro {
    /// Lê um arquivo e devolve o documento com selo, relatório e miniatura.
    pub fn importar(&self, caminho: &Path, o: &OpcoesImportar) -> Result<Importado, ErroConv> {
        let texto = texto_do_caminho(caminho)?;
        let bytes = std::fs::read(caminho).map_err(|e| ErroConv::Leitura(format!("não foi possível ler '{texto}': {e}")))?;
        let (formato, extensao_diverge) = detectar(caminho, &bytes)?;
        if formato == Formato::Arvore {
            return crate::arvore::importar(caminho, &bytes, extensao_diverge);
        }
        if formato == Formato::Dxf {
            return crate::dxf::importar(caminho, &bytes, o, extensao_diverge);
        }
        let provedor = self.provedor(formato)?;
        let mut modelo = provedor.ler(formato, caminho, o)?;

        let nome_arquivo = caminho.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let nome_doc = caminho.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Importado".into());
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let esquema = if formato == Formato::Brep { formato.nome().to_string() } else { modelo.xde.esquema.clone() };
        let unidades = if formato == Formato::Brep { "mm".to_string() } else { modelo.xde.unidades.clone() };
        let mut r = Relatorio {
            arquivo: nome_arquivo.clone(),
            hash: hash.clone(),
            tamanho: bytes.len() as u64,
            formato: formato.nome().into(),
            provedor: provedor.id().into(),
            versao: format!("Fenkra {} / {}", env!("CARGO_PKG_VERSION"), version()),
            esquema: esquema.clone(),
            unidades: unidades.clone(),
            escala: 1.,
            reparo: o.reparo.nome().into(),
            ..Default::default()
        };
        if extensao_diverge {
            r.avisos.push(Nota::nova(AV_EXTENSAO, &[formato.nome().to_string()]));
        }
        if formato.e_malha() && unidades.is_empty() && o.unidade.em_mm().is_none() {
            r.avisos.push(Nota::nova(AV_MALHA_SEM_UNIDADE, &[formato.nome().to_string()]));
        }
        if let Some(mm) = o.unidade.em_mm() {
            r.escala = mm / declarada_em_mm(&unidades);
            if (r.escala - 1.).abs() > 1e-12 {
                escalar(&mut modelo, r.escala)?;
                r.avisos.push(Nota::nova(AV_ESCALA, &[format!("{}", r.escala)]));
            }
        }
        reparar(&mut modelo, o, &mut r)?;
        listar_defeitos(&modelo, &mut r)?;
        medir(&modelo, &mut r)?;
        if let Some(m) = r.medidas() {
            let maior = m.into_iter().fold(0., f64::max);
            match unidade_suspeita(m) {
                Some(Suspeita::Grande) => r.avisos.push(Nota::nova(AV_GRANDE, &[format!("{maior:.1}")])),
                Some(Suspeita::Pequena) => r.avisos.push(Nota::nova(AV_PEQUENA, &[format!("{maior:.4}")])),
                None => {}
            }
        }
        let conjunto = o.destino == Destino::Montagem && modelo.pede_conjunto();
        o_que_fica_fora(&modelo, conjunto, &mut r);

        let selo = SeloOrigem {
            nome: nome_arquivo,
            hash,
            data: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            esquema,
            unidades,
            derivado_de: None,
            relatorio: None,
        };
        // o documento antes da miniatura: a malha fica na forma e iria junto para o B-rep
        let (mut doc, mut pecas) = if conjunto {
            let (c, corpos) = modelo.para_conjunto(&nome_doc, texto, &selo)?;
            r.corpos = corpos;
            let primeira = c.pecas.first().map(|(_, d)| d.clone()).ok_or_else(|| ErroConv::Leitura("arquivo sem peças".into()))?;
            (primeira, Some(c))
        } else {
            let (d, corpos) = montar_documento(&modelo, &nome_doc, texto, selo)?;
            r.corpos = corpos;
            (d, None)
        };
        // só na primeira peça: em todas, mil peças levariam mil cópias do relatório
        let embutido = serde_json::to_string(&r).ok();
        if let Some(s) = &mut doc.origem {
            s.relatorio.clone_from(&embutido);
        }
        if let Some((_, primeira)) = pecas.as_mut().and_then(|c| c.pecas.first_mut()) {
            if let Some(s) = &mut primeira.origem {
                s.relatorio = embutido;
            }
        }
        let previa = previa(&modelo, &r)?;
        Ok(Importado { doc, relatorio: r, previa, conjunto: pecas })
    }

    /// Grava um documento. Formatos de receita saem da árvore, e os outros da forma recalculada.
    pub fn gravar_documento(&self, doc: &Document, destino: &Path, saida: &crate::OpcoesExportar) -> Result<(), ErroConv> {
        match Formato::por_extensao(destino) {
            Some(Formato::Arvore) => {
                let mut d = doc.clone();
                crate::arvore::atualizar_pontos(&mut d);
                let arquivo = crate::arvore::exportar_peca(&d).arquivo();
                let texto = serde_json::to_string_pretty(&arquivo).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
                std::fs::write(destino, texto).map_err(|e| ErroConv::Gravacao(format!("não foi possível gravar '{}': {e}", destino.display())))
            }
            _ => self.gravar(&ModeloNeutro::de_documento(doc)?, destino, saida),
        }
    }

    /// Lê e grava noutro formato. Só passa por documento quem precisa dele: a peça do Fenkra, a árvore e
    /// o desenho. Malha para malha nem sobe ao B-rep.
    pub fn converter(&self, origem: &Path, destino: &Path, o: &OpcoesImportar, saida: &crate::OpcoesExportar) -> Result<(), ErroConv> {
        let bytes = std::fs::read(origem).map_err(|e| ErroConv::Leitura(format!("não foi possível ler '{}': {e}", origem.display())))?;
        let extensao = origem.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let texto = || String::from_utf8_lossy(&bytes).to_string();
        match extensao.as_str() {
            "fkp" => {
                let doc = Document::from_json(&texto()).map_err(|e| ErroConv::Leitura(format!("peça ilegível: {e}")))?;
                return self.gravar_documento(&doc, destino, saida);
            }
            "fka" => {
                let asm = fenkra_montagem::Assembly::from_json(&texto()).map_err(|e| ErroConv::Leitura(format!("montagem ilegível: {e}")))?;
                let pasta = origem.parent().unwrap_or(Path::new("."));
                if Formato::por_extensao(destino) == Some(Formato::Arvore) {
                    let arquivo = crate::arvore::exportar_montagem(&asm, pasta)?;
                    let json = serde_json::to_string_pretty(&arquivo).map_err(|e| ErroConv::Gravacao(e.to_string()))?;
                    return std::fs::write(destino, json).map_err(|e| ErroConv::Gravacao(e.to_string()));
                }
                return self.gravar(&ModeloNeutro::da_montagem(&asm, pasta)?, destino, saida);
            }
            _ => {}
        }
        let (formato, _) = detectar(origem, &bytes)?;
        if formato == Formato::Arvore || formato.e_2d() {
            let i = self.importar(origem, o)?;
            return self.gravar_documento(&i.doc, destino, saida);
        }
        let saida_formato = self.formato_de_saida(destino)?;
        if formato.e_malha() && saida_formato.e_malha() {
            return crate::malha::converter(formato, origem, saida_formato, destino, o, saida);
        }
        let modelo = self.provedor(formato)?.ler(formato, origem, o)?;
        self.gravar(&modelo, destino, saida)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn limites_do_aviso_de_unidade() {
        assert_eq!(unidade_suspeita([10_000.1, 1., 1.]), Some(Suspeita::Grande));
        assert_eq!(unidade_suspeita([10_000., 1., 1.]), None);
        assert_eq!(unidade_suspeita([0.09, 0.05, 0.01]), Some(Suspeita::Pequena));
        assert_eq!(unidade_suspeita([0.1, 0.05, 0.01]), None);
    }

    #[test]
    fn unidade_declarada_pelo_nome_do_step() {
        assert_eq!(declarada_em_mm("INCH"), 25.4);
        assert_eq!(declarada_em_mm("MILLIMETRE"), 1.);
        assert_eq!(declarada_em_mm("METRE"), 1000.);
        assert_eq!(declarada_em_mm("IN"), 25.4, "o IGES abrevia");
        assert_eq!(declarada_em_mm("CM"), 10.);
        assert_eq!(declarada_em_mm(""), 1.);
    }
}
