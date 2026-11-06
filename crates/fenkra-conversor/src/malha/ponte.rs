// SPDX-License-Identifier: LGPL-2.1-or-later
//! Da cena ao modelo neutro e de volta.
//!
//! Para o modelo: cada conjunto de malhas vira um produto, e a peça repetida continua um produto só. A
//! malha fechada vira sólido facetado; a aberta, superfície. Partes soltas sobem separadas, porque o
//! kernel monta um sólido por casca e perderia as outras.
//!
//! Para a cena: cada produto vira malha pelo desvio pedido, uma vez por definição. Cor por face separa
//! a malha em uma por cor, porque quase nenhum formato tem cor por face.

use super::{erro, Cena, Malha, No};
use crate::modelo::PROFUNDIDADE;
use crate::{ErroConv, Formato, ModeloNeutro, OpcoesExportar, OpcoesImportar};
use fenkra_kernel::{InstanciaXde, ModeloXde, ProdutoXde, Shape, IDENTIDADE};
use std::collections::{BTreeMap, HashMap};

/// Teto do caminho até o B-rep: cada triângulo vira uma face costurada. É o mesmo do comando de
/// importar malha.
pub(crate) const LIMITE_BREP: usize = 200_000;

/// Forma de uma lista de malhas. Devolve também a cor da maior e quantas cavidades ficaram separadas.
fn forma_de(cena: &Cena, malhas: &[usize]) -> Result<(Option<Shape>, Option<[f32; 4]>, usize), ErroConv> {
    let mut junta = Malha::default();
    let (mut maior, mut cor) = (-1., None);
    for &i in malhas {
        let m = &cena.malhas[i];
        let area = m.area();
        if area > maior {
            (maior, cor) = (area, cena.cor_de(m));
        }
        let base = junta.pos.len() as u32;
        junta.pos.extend(&m.pos);
        junta.tris.extend(m.tris.iter().map(|t| t.map(|v| v + base)));
    }
    let partes = junta.soldada().componentes();
    let volumes: Vec<f64> = partes.iter().map(|p| if p.fechada() { p.volume() } else { 0. }).collect();
    let externo = volumes.iter().fold(0f64, |a, v| if v.abs() > a.abs() { *v } else { a });
    // casca do lado contrário da maior é uma cavidade: o kernel não subtrai, e ela vira sólido à parte
    let cavidades = volumes.iter().filter(|v| **v != 0. && v.signum() != externo.signum()).count();
    let mut forma: Option<Shape> = None;
    for p in partes {
        let tris: Vec<f64> = p.tris.iter().flat_map(|t| t.iter().flat_map(|i| p.pos[*i as usize])).collect();
        let Ok(s) = Shape::from_mesh(&tris, p.fechada()) else { continue };
        forma = Some(match forma {
            Some(f) => f.compound(&s),
            None => s,
        });
    }
    Ok((forma, cor, cavidades))
}

struct Subida<'a> {
    cena: &'a Cena,
    produtos: Vec<ProdutoXde>,
    por_no: HashMap<usize, usize>,
    por_malhas: HashMap<Vec<usize>, usize>,
    cavidades: usize,
    varias_cores: usize,
}

impl Subida<'_> {
    fn peca(&mut self, nome: &str, malhas: &[usize]) -> Result<usize, ErroConv> {
        if let Some(p) = self.por_malhas.get(malhas) {
            return Ok(*p);
        }
        let (forma, cor, cavidades) = forma_de(self.cena, malhas)?;
        self.cavidades += cavidades;
        if malhas.len() > 1 {
            let cores: Vec<_> = malhas.iter().map(|i| self.cena.cor_de(&self.cena.malhas[*i]).map(|c| c.map(f32::to_bits))).collect();
            self.varias_cores += usize::from(cores.windows(2).any(|w| w[0] != w[1]));
        }
        let proprio = malhas.first().map(|i| self.cena.malhas[*i].nome.as_str()).unwrap_or_default();
        let nome = if proprio.is_empty() { nome } else { proprio };
        self.produtos.push(ProdutoXde { nome: nome.into(), forma, cor, ..Default::default() });
        self.por_malhas.insert(malhas.to_vec(), self.produtos.len() - 1);
        Ok(self.produtos.len() - 1)
    }

    fn produto(&mut self, i: usize, nivel: usize) -> Result<usize, ErroConv> {
        if let Some(p) = self.por_no.get(&i) {
            return Ok(*p);
        }
        if nivel > PROFUNDIDADE {
            return Err(erro("malha", "árvore funda demais"));
        }
        let no = self.cena.nos[i].clone();
        if no.filhos.is_empty() {
            let p = self.peca(&no.nome, &no.malhas)?;
            self.por_no.insert(i, p);
            return Ok(p);
        }
        let mut filhos = Vec::new();
        if !no.malhas.is_empty() {
            let produto = self.peca(&no.nome, &no.malhas)?;
            filhos.push(InstanciaXde { produto, nome: no.nome.clone(), local: IDENTIDADE, cor: None });
        }
        for &f in &no.filhos {
            let produto = self.produto(f, nivel + 1)?;
            let n = &self.cena.nos[f];
            filhos.push(InstanciaXde { produto, nome: n.nome.clone(), local: n.local, cor: None });
        }
        self.produtos.push(ProdutoXde { nome: no.nome, filhos, ..Default::default() });
        self.por_no.insert(i, self.produtos.len() - 1);
        Ok(self.produtos.len() - 1)
    }
}

/// Modelo neutro de uma cena lida.
pub(crate) fn para_modelo(mut cena: Cena, formato: Formato, o: &OpcoesImportar) -> Result<ModeloNeutro, ErroConv> {
    if !o.cores {
        cena.sem_cores();
    }
    cena.assentar();
    cena.enxugar();
    let (ocorrencias, inteiro) = cena.ocorrencias();
    if !inteiro {
        return Err(erro(formato.nome(), "árvore com ciclo, funda demais ou com ocorrências demais"));
    }
    let mut usadas: Vec<usize> = ocorrencias.iter().map(|(_, m, _)| *m).collect();
    usadas.sort_unstable();
    usadas.dedup();
    let triangulos: usize = usadas.iter().map(|i| cena.malhas[*i].tris.len()).sum();
    if triangulos > LIMITE_BREP {
        return Err(erro(formato.nome(), format!("malha grande demais para virar sólido ({triangulos} triângulos; máximo {LIMITE_BREP}): simplifique antes, ou converta direto para outro formato de malha")));
    }
    let mut s = Subida { cena: &cena, produtos: vec![], por_no: HashMap::new(), por_malhas: HashMap::new(), cavidades: 0, varias_cores: 0 };
    let mut raizes = Vec::new();
    for &r in &cena.raizes {
        raizes.push((s.produto(r, 0)?, r));
    }
    let (cavidades, varias_cores) = (s.cavidades, s.varias_cores);
    let mut produtos = s.produtos;
    let so_uma = raizes.len() == 1 && cena.nos[raizes[0].1].local == IDENTIDADE;
    let raizes = if so_uma {
        vec![raizes[0].0]
    } else {
        let filhos = raizes.iter().map(|(p, r)| InstanciaXde { produto: *p, nome: cena.nos[*r].nome.clone(), local: cena.nos[*r].local, cor: None }).collect();
        produtos.push(ProdutoXde { nome: cena.nome.clone(), filhos, ..Default::default() });
        vec![produtos.len() - 1]
    };
    if !o.nomes {
        for (i, p) in produtos.iter_mut().enumerate() {
            p.nome = format!("Peça {}", i + 1);
        }
    }
    let mut ignorados = cena.ignorados.clone();
    if cavidades > 0 {
        ignorados.insert("cavidade interna que ficou como sólido à parte".into(), cavidades);
    }
    if varias_cores > 0 {
        ignorados.insert("peça com vários materiais, que ficou com a cor do maior".into(), varias_cores);
    }
    let ignoradas = ignorados.iter().map(|(oque, n)| format!("{oque}\t{n}")).collect();
    Ok(ModeloNeutro { formato, xde: ModeloXde { produtos, raizes, esquema: formato.nome().into(), unidades: cena.unidade.clone(), ignoradas, ..Default::default() } })
}

/// Chave de cor: `f32` não entra em mapa.
fn bits(c: Option<[f32; 4]>) -> Option<[u32; 4]> {
    c.map(|c| c.map(f32::to_bits))
}

struct Descida<'a> {
    modelo: &'a ModeloNeutro,
    o: &'a OpcoesExportar,
    cena: Cena,
    por_produto: HashMap<(usize, Option<[u32; 4]>), usize>,
    malhas: HashMap<(usize, Option<[u32; 4]>), Vec<usize>>,
}

/// Malha de uma lista de faces, só com os vértices que elas usam.
fn recorte(nome: &str, pos: &[f32], nrm: &[f32], indices: &[u32], faixas: &[(usize, usize)]) -> Malha {
    let mut m = Malha { nome: nome.into(), ..Default::default() };
    let mut novo: HashMap<u32, u32> = HashMap::new();
    for &(a, b) in faixas {
        for t in indices[a..b].chunks_exact(3) {
            m.tris.push([t[0], t[1], t[2]].map(|v| {
                *novo.entry(v).or_insert_with(|| {
                    let i = v as usize * 3;
                    m.pos.push([pos[i], pos[i + 1], pos[i + 2]].map(f64::from));
                    if nrm.len() == pos.len() {
                        m.nrm.push([nrm[i], nrm[i + 1], nrm[i + 2]]);
                    }
                    (m.pos.len() - 1) as u32
                })
            }));
        }
    }
    m
}

impl Descida<'_> {
    /// Malhas de um produto com a cor pedida. A malha do kernel sai uma vez por definição e cor.
    fn malhas_de(&mut self, produto: usize, cor: Option<[f32; 4]>) -> Result<Vec<usize>, ErroConv> {
        let chave = (produto, bits(cor));
        if let Some(v) = self.malhas.get(&chave) {
            return Ok(v.clone());
        }
        let p = &self.modelo.xde.produtos[produto];
        let Some(forma) = &p.forma else { return Ok(vec![]) };
        let falha = |e: fenkra_kernel::KernelError| ErroConv::Gravacao(e.0);
        let k = forma.mesh_angle(self.o.malha.desvio.max(1e-4), self.o.malha.angulo.max(1.).to_radians()).map_err(falha)?;
        let faces: Vec<(usize, usize)> = (0..k.face_ranges.len()).map(|i| (k.face_ranges[i] as usize, k.face_ranges.get(i + 1).map(|x| *x as usize).unwrap_or(k.indices.len()))).collect();
        let mut grupos: BTreeMap<Option<[u32; 4]>, Vec<(usize, usize)>> = BTreeMap::new();
        for (i, f) in faces.iter().enumerate() {
            let propria = p.cores_faces.iter().find(|(q, _)| *q as usize == i).map(|(_, c)| *c);
            grupos.entry(bits(propria.filter(|_| self.o.cores && cor.is_none() || self.o.cores && p.cor == cor).or(cor))).or_default().push(*f);
        }
        let mut v = Vec::new();
        for (c, faixas) in grupos {
            let mut m = recorte(&p.nome, &k.positions, &k.normals, &k.indices, &faixas);
            if let Some(c) = c.filter(|_| self.o.cores) {
                let nome = format!("Cor {}", self.cena.materiais.len() + 1);
                m.material = Some(self.cena.material(&nome, c.map(f32::from_bits)));
            }
            v.push(self.cena.malhas.len());
            self.cena.malhas.push(m);
        }
        self.malhas.insert(chave, v.clone());
        Ok(v)
    }

    fn no_de(&mut self, produto: usize, herdada: Option<[f32; 4]>, nivel: usize) -> Result<Option<usize>, ErroConv> {
        if nivel > PROFUNDIDADE {
            return Ok(None);
        }
        let Some(p) = self.modelo.xde.produtos.get(produto) else { return Ok(None) };
        let cor = herdada.or(p.cor);
        if let Some(i) = self.por_produto.get(&(produto, bits(cor))) {
            return Ok(Some(*i));
        }
        let mut no = No::novo(&p.nome);
        no.malhas = self.malhas_de(produto, cor)?;
        for f in &p.filhos {
            let Some(filho) = self.no_de(f.produto, herdada.or(f.cor).or(p.cor), nivel + 1)? else { continue };
            let mut instancia = No::novo(&self.modelo.nome_da_instancia(f));
            instancia.local = f.local;
            instancia.filhos = vec![filho];
            self.cena.nos.push(instancia);
            no.filhos.push(self.cena.nos.len() - 1);
        }
        self.cena.nos.push(no);
        let i = self.cena.nos.len() - 1;
        self.por_produto.insert((produto, bits(cor)), i);
        Ok(Some(i))
    }
}

/// Cena de um modelo, com a malha pelo desvio das opções.
pub(crate) fn de_modelo(modelo: &ModeloNeutro, o: &OpcoesExportar) -> Result<Cena, ErroConv> {
    let mut d = Descida { modelo, o, cena: Cena::default(), por_produto: HashMap::new(), malhas: HashMap::new() };
    for &r in &modelo.xde.raizes {
        if let Some(i) = d.no_de(r, None, 0)? {
            d.cena.raizes.push(i);
        }
    }
    let mut cena = d.cena;
    cena.nome = modelo.xde.raizes.first().and_then(|r| modelo.xde.produtos.get(*r)).map(|p| p.nome.clone()).unwrap_or_default();
    cena.enxugar();
    if !o.nomes {
        for (i, n) in cena.nos.iter_mut().enumerate() {
            n.nome = format!("No_{}", i + 1);
        }
        for (i, m) in cena.malhas.iter_mut().enumerate() {
            m.nome = format!("Malha_{}", i + 1);
        }
    }
    if cena.malhas.iter().all(|m| m.tris.is_empty()) {
        return Err(ErroConv::Gravacao("o modelo não tem forma para gravar".into()));
    }
    Ok(cena)
}
