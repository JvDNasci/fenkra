// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ramos e mesclagem de documento, sem trava de peça (B130).
//!
//! Em vez de travar a peça, cada pessoa abre um **ramo**: uma linha de versões paralela à principal.
//! A principal (as versões do projeto) continua sendo a que todos veem; o ramo guarda o trabalho em
//! andamento sem vazar para ela.
//!
//! Como a mesclagem funciona (3 vias, por identidade):
//! - A **base** é a versão da principal em que o ramo nasceu. As outras duas pontas são a principal de
//!   hoje (nossa) e a ponta do ramo (deles). Cada feature tem um `Id` estável no documento, então o
//!   casamento é por `Id`, não por posição nem por bytes.
//! - Para cada feature, cada campo segue a regra clássica: se só um lado mudou, vale a mudança; se os
//!   dois mudaram para o mesmo valor, vale esse valor; se mudaram para valores diferentes, é conflito.
//!   Features diferentes nunca conflitam. O mesmo vale para parâmetros, coletores e o resto do
//!   documento. O comparador de campos é genérico (JSON), então um tipo novo de comando entra sozinho.
//! - Os `Id`s novos de cada lado não colidem: o ramo reserva uma faixa própria de ids ao nascer.
//! - A ordem das features novas é determinística (âncora = a feature anterior no lado que a criou,
//!   empate por `Id`), o que torna o resultado igual nas duas ordens de mesclagem.
//!
//! Linha principal protegida: o ramo só entra na principal depois de um **pedido de mesclagem**
//! revisado e aprovado por quem tem o papel `aprovador`. A aprovação passa pelo checador de qualidade
//! (o gancho `verificar`, que o app liga ao recálculo) e fica presa à ponta do ramo e à principal em
//! que foi dada: se qualquer um mudar depois, a aprovação deixa de valer. Depois de mesclar, o gancho
//! roda de novo no resultado e nada quebrado entra.
//!
//! Tudo fica em `ramos.json` (gravação atômica) e no diário só-acréscimo; o conteúdo das pontas vai
//! para `.versoes/` como as demais versões, então ramos reaproveitam a comparação (B214).

use crate::comparacao::{comparar_documentos, ComparacaoDocs, Mudanca};
use crate::diario::TipoEvento;
use crate::seguro::gravar_atomico;
use crate::{now, PResult, Project, ProjectError, Version, VERSIONS};
use fenkra_core::Document;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap, HashSet};

pub const ARQUIVO_RAMOS: &str = "ramos.json";
/// Nome reservado da linha principal.
pub const PRINCIPAL: &str = "principal";
/// Quantos ids cada ramo reserva para si (não colide com a principal nem com outro ramo).
const FAIXA_IDS: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoRamo {
    Aberto,
    Mesclado,
    Descartado,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoPedido {
    Aberto,
    Aprovado,
    Recusado,
}

/// Pedido de mesclagem do ramo na principal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pedido {
    pub autor: String,
    pub tempo: u64,
    pub mensagem: String,
    pub estado: EstadoPedido,
    #[serde(default)]
    pub revisor: String,
    #[serde(default)]
    pub motivo: String,
    /// ponta do ramo e versão da principal em que a aprovação foi dada
    #[serde(default)]
    pub hash_ponta: String,
    #[serde(default)]
    pub hash_principal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ramo {
    pub doc_id: u64,
    pub nome: String,
    pub autor: String,
    pub criado: u64,
    pub atualizado: u64,
    pub estado: EstadoRamo,
    /// versão da principal em que o ramo nasceu
    pub base_hash: String,
    /// conteúdo mais recente do ramo
    pub ponta_hash: String,
    /// número sequencial que define a faixa de ids do ramo; nunca é reaproveitado
    pub seq: u64,
    #[serde(default)]
    pub versoes: Vec<Version>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pedido: Option<Pedido>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Cofre {
    #[serde(default)]
    proximo_seq: u64,
    #[serde(default)]
    ramos: Vec<Ramo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lado {
    /// a principal de hoje
    Nossa,
    /// o ramo
    Deles,
}

/// Um ponto em que os dois lados mudaram a mesma coisa para valores diferentes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflito {
    /// identifica o ponto para a escolha (estável entre chamadas)
    pub chave: String,
    /// frase para a pessoa, ex.: `Furo 1: Hole.diameter`
    pub rotulo: String,
    /// feature envolvida, para destacar no 3D lado a lado
    pub feature: Option<u64>,
    pub base: String,
    pub nossa: String,
    pub deles: String,
}

/// Resultado de uma mesclagem calculada (ainda não gravada).
#[derive(Clone, Debug)]
pub struct Mescla {
    pub documento: Document,
    /// conflitos encontrados; o documento usa a escolha dada, ou `Nossa` quando nenhuma foi dada
    pub conflitos: Vec<Conflito>,
}

impl Mescla {
    pub fn limpa(&self) -> bool {
        self.conflitos.is_empty()
    }
}

/// Aviso antecipado: dois ramos (ou um ramo e a principal) mexeram na mesma feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvisoSobreposicao {
    pub ramo: String,
    /// outro ramo, ou [`PRINCIPAL`]
    pub contra: String,
    pub features: Vec<u64>,
}

// ---------------------------------------------------------------- mesclagem de 3 vias

struct Ctx<'a> {
    escolhas: &'a HashMap<String, Lado>,
    conflitos: Vec<Conflito>,
    /// nomes das features para o rótulo (de qualquer lado)
    nomes: HashMap<u64, String>,
}

impl Ctx<'_> {
    fn decidir(&mut self, chave: String, rotulo: String, feature: Option<u64>, b: Option<&Value>, a: Option<&Value>, t: Option<&Value>) -> Option<Value> {
        let txt = |v: Option<&Value>| match v {
            None | Some(Value::Null) => "(nenhum)".to_string(),
            Some(Value::String(s)) if s.is_empty() => "(vazio)".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(o) => o.to_string(),
        };
        let lado = self.escolhas.get(&chave).copied().unwrap_or(Lado::Nossa);
        self.conflitos.push(Conflito { chave, rotulo, feature, base: txt(b), nossa: txt(a), deles: txt(t) });
        match lado {
            Lado::Nossa => a.cloned(),
            Lado::Deles => t.cloned(),
        }
    }
}

fn id_de(v: &Value) -> Option<u64> {
    v.as_object()?.get("id")?.as_u64()
}

/// Array de objetos com `id` numérico em todos os itens: casa por identidade.
fn e_chaveado(v: &Value) -> bool {
    matches!(v, Value::Array(a) if !a.is_empty() && a.iter().all(|x| id_de(x).is_some()))
}

fn mescla_valor(caminho: &str, rotulo: &str, feature: Option<u64>, b: Option<&Value>, a: Option<&Value>, t: Option<&Value>, ctx: &mut Ctx) -> Option<Value> {
    if a == t {
        return a.cloned();
    }
    if a == b {
        return t.cloned();
    }
    if t == b {
        return a.cloned();
    }
    match (a, t) {
        (Some(Value::Object(oa)), Some(Value::Object(ot))) => {
            let vazio = Map::new();
            let ob = b.and_then(Value::as_object).unwrap_or(&vazio);
            Some(Value::Object(mescla_objeto(caminho, rotulo, feature, ob, oa, ot, ctx)))
        }
        (Some(va @ Value::Array(xa)), Some(vt @ Value::Array(xt))) => {
            let vb = b.cloned().unwrap_or(Value::Array(vec![]));
            let xb = vb.as_array().cloned().unwrap_or_default();
            if (e_chaveado(va) || e_chaveado(vt)) && [&xb, xa, xt].iter().all(|l| l.iter().all(|x| id_de(x).is_some())) {
                return Some(Value::Array(mescla_chaveada(caminho, rotulo, &xb, xa, xt, ctx)));
            }
            if xa.len() == xt.len() && xb.len() == xa.len() {
                let itens = (0..xa.len()).filter_map(|i| mescla_valor(&format!("{caminho}[{i}]"), rotulo, feature, xb.get(i), xa.get(i), xt.get(i), ctx)).collect();
                return Some(Value::Array(itens));
            }
            ctx.decidir(caminho.into(), format!("{rotulo}"), feature, b, a, t)
        }
        _ => ctx.decidir(caminho.into(), rotulo.into(), feature, b, a, t),
    }
}

fn mescla_objeto(caminho: &str, rotulo: &str, feature: Option<u64>, b: &Map<String, Value>, a: &Map<String, Value>, t: &Map<String, Value>, ctx: &mut Ctx) -> Map<String, Value> {
    let chaves: BTreeSet<&String> = a.keys().chain(t.keys()).chain(b.keys()).collect();
    let mut saida = Map::new();
    for k in chaves {
        let sub = if caminho.is_empty() { k.clone() } else { format!("{caminho}/{k}") };
        let r = if rotulo.is_empty() { k.clone() } else { format!("{rotulo}.{k}") };
        if let Some(v) = mescla_valor(&sub, &r, feature, b.get(k), a.get(k), t.get(k), ctx) {
            saida.insert(k.clone(), v);
        }
    }
    saida
}

/// Mescla listas de objetos com `id`: o que está em um só lado entra, o que nenhum lado mexeu fica,
/// o que um lado apagou e o outro editou é conflito.
fn mescla_chaveada(caminho: &str, rotulo: &str, b: &[Value], a: &[Value], t: &[Value], ctx: &mut Ctx) -> Vec<Value> {
    let mapa = |l: &[Value]| -> HashMap<u64, Value> { l.iter().filter_map(|x| id_de(x).map(|i| (i, x.clone()))).collect() };
    let (mb, ma, mt) = (mapa(b), mapa(a), mapa(t));
    let ids: BTreeSet<u64> = mb.keys().chain(ma.keys()).chain(mt.keys()).copied().collect();
    let nome_de = |ctx: &Ctx, id: u64, v: Option<&Value>| -> String {
        v.and_then(|x| x.get("name")).and_then(Value::as_str).map(str::to_string).or_else(|| ctx.nomes.get(&id).cloned()).unwrap_or_else(|| format!("#{id}"))
    };

    let mut mantidos: HashMap<u64, Value> = HashMap::new();
    for id in ids {
        let (vb, va, vt) = (mb.get(&id), ma.get(&id), mt.get(&id));
        let nome = nome_de(ctx, id, va.or(vt).or(vb));
        let chave = format!("{caminho}#{id}");
        let r = if rotulo.is_empty() { nome.clone() } else { format!("{nome}") };
        let item = match (vb, va, vt) {
            (Some(vb), Some(va), Some(vt)) => mescla_valor(&chave, &r, Some(id), Some(vb), Some(va), Some(vt), ctx),
            (Some(vb), Some(va), None) => {
                if va == vb {
                    None
                } else {
                    ctx.decidir(format!("{chave}/removida"), format!("{nome}: alterada na principal e removida no ramo"), Some(id), Some(vb), Some(va), None)
                }
            }
            (Some(vb), None, Some(vt)) => {
                if vt == vb {
                    None
                } else {
                    ctx.decidir(format!("{chave}/removida"), format!("{nome}: removida na principal e alterada no ramo"), Some(id), Some(vb), None, Some(vt))
                }
            }
            (Some(_), None, None) => None,
            (None, Some(va), None) => Some(va.clone()),
            (None, None, Some(vt)) => Some(vt.clone()),
            (None, Some(va), Some(vt)) => {
                if va == vt {
                    Some(va.clone())
                } else {
                    ctx.decidir(format!("{chave}/colisao"), format!("{nome}: mesmo id criado nos dois lados com conteúdos diferentes"), Some(id), None, Some(va), Some(vt))
                }
            }
            (None, None, None) => None,
        };
        if let Some(v) = item {
            mantidos.insert(id, v);
        }
    }

    let ids_de = |l: &[Value]| -> Vec<u64> { l.iter().filter_map(id_de).collect() };
    let (ob, oa, ot) = (ids_de(b), ids_de(a), ids_de(t));
    let base_ids: HashSet<u64> = ob.iter().copied().collect();
    let filtrar = |o: &[u64], so_base: bool| -> Vec<u64> { o.iter().copied().filter(|i| mantidos.contains_key(i) && (base_ids.contains(i) == so_base)).collect() };
    let (sb, sa, st) = (filtrar(&ob, true), filtrar(&oa, true), filtrar(&ot, true));
    // esqueleto: ids que já existiam na base e sobreviveram, na ordem do lado que a mudou
    let mut esqueleto = if sa == sb {
        st.clone()
    } else if st == sb || sa == st {
        sa.clone()
    } else {
        let escolha = ctx.decidir(
            format!("{caminho}/ordem"),
            format!("ordem de {rotulo} mudou nos dois lados"),
            None,
            Some(&Value::String(format!("{sb:?}"))),
            Some(&Value::String(format!("{sa:?}"))),
            Some(&Value::String(format!("{st:?}"))),
        );
        if escolha.as_ref().and_then(Value::as_str) == Some(&format!("{st:?}")) { st.clone() } else { sa.clone() }
    };
    // sobreviventes da base ausentes da ordem escolhida (resolução de conflito): entram depois do
    // antecessor que tinham na base
    for (i, id) in ob.iter().enumerate() {
        if mantidos.contains_key(id) && !esqueleto.contains(id) {
            let pos = ob[..i].iter().rev().find_map(|p| esqueleto.iter().position(|e| e == p).map(|k| k + 1)).unwrap_or(0);
            esqueleto.insert(pos, *id);
        }
    }

    // novos: âncora = a feature anterior (já mantida) no lado que a criou
    let mut filhos: HashMap<Option<u64>, BTreeSet<u64>> = HashMap::new();
    for lado in [&oa, &ot] {
        for (i, id) in lado.iter().enumerate() {
            if base_ids.contains(id) || !mantidos.contains_key(id) {
                continue;
            }
            let ancora = lado[..i].iter().rev().find(|p| mantidos.contains_key(p)).copied();
            filhos.entry(ancora).or_default().insert(*id);
        }
    }
    fn emitir(ancora: Option<u64>, filhos: &HashMap<Option<u64>, BTreeSet<u64>>, visto: &mut HashSet<u64>, saida: &mut Vec<u64>) {
        if let Some(fs) = filhos.get(&ancora) {
            for &f in fs {
                if visto.insert(f) {
                    saida.push(f);
                    emitir(Some(f), filhos, visto, saida);
                }
            }
        }
    }
    let (mut visto, mut ordem): (HashSet<u64>, Vec<u64>) = (HashSet::new(), vec![]);
    emitir(None, &filhos, &mut visto, &mut ordem);
    for id in &esqueleto {
        ordem.push(*id);
        emitir(Some(*id), &filhos, &mut visto, &mut ordem);
    }
    // qualquer novo cuja âncora não foi alcançada (não deveria acontecer) vai ao fim, por id
    let mut sobra: Vec<u64> = mantidos.keys().copied().filter(|i| !ordem.contains(i)).collect();
    sobra.sort_unstable();
    ordem.extend(sobra);
    ordem.into_iter().filter_map(|i| mantidos.remove(&i)).collect()
}

/// Mescla de 3 vias de documentos: `base` é o ancestral, `nossa` a principal de hoje, `deles` o ramo.
/// `escolhas` resolve conflitos pela chave; sem escolha vale `Nossa`.
pub fn mesclar_documentos(base: &Document, nossa: &Document, deles: &Document, escolhas: &HashMap<String, Lado>) -> PResult<Mescla> {
    let v = |d: &Document| serde_json::to_value(d);
    let (vb, va, vt) = (v(base)?, v(nossa)?, v(deles)?);
    let mut nomes = HashMap::new();
    for d in [base, nossa, deles] {
        for f in &d.features {
            nomes.insert(f.id.0, f.name.clone());
        }
    }
    let mut ctx = Ctx { escolhas, conflitos: vec![], nomes };
    let (ob, oa, ot) = (vb.as_object().cloned().unwrap_or_default(), va.as_object().cloned().unwrap_or_default(), vt.as_object().cloned().unwrap_or_default());
    // campos que não são conteúdo: o contador de ids vale o maior; versão do programa e uid, os nossos
    let mut ob2 = ob.clone();
    let (mut oa2, mut ot2) = (oa.clone(), ot.clone());
    for k in ["next_id", "app_version", "uid"] {
        ob2.remove(k);
        oa2.remove(k);
        ot2.remove(k);
    }
    let mut obj = mescla_objeto("", "", None, &ob2, &oa2, &ot2, &mut ctx);
    for k in ["app_version", "uid"] {
        if let Some(x) = oa.get(k) {
            obj.insert(k.into(), x.clone());
        }
    }
    let proximo = [&ob, &oa, &ot].iter().filter_map(|o| o.get("next_id").and_then(Value::as_u64)).max().unwrap_or(1);
    obj.insert("next_id".into(), Value::from(proximo));
    let documento: Document = serde_json::from_value(Value::Object(obj))?;
    Ok(Mescla { documento, conflitos: ctx.conflitos })
}

/// Problemas que impedem a entrada de um documento mesclado, sem precisar do kernel: ids repetidos,
/// nomes de parâmetro repetidos.
pub fn problemas_estruturais(d: &Document) -> Vec<String> {
    let mut p = vec![];
    let mut ids = HashSet::new();
    for f in &d.features {
        if !ids.insert(f.id.0) {
            p.push(format!("id repetido na árvore: {}", f.name));
        }
    }
    let mut nomes = HashSet::new();
    for q in &d.params {
        if !nomes.insert(q.name.to_lowercase()) {
            p.push(format!("parâmetro repetido: {}", q.name));
        }
    }
    p
}

// ---------------------------------------------------------------- projeto

/// Conferência extra do documento mesclado (o app liga ao recálculo: status, volume, referências).
/// Devolve as frases dos problemas; vazio = pode entrar.
pub type Verificador<'a> = &'a dyn Fn(&Document) -> Vec<String>;

impl Project {
    fn cofre(&self) -> Cofre {
        std::fs::read_to_string(self.root.join(ARQUIVO_RAMOS)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    fn gravar_cofre(&self, c: &Cofre) -> PResult<()> {
        self.pode_gravar()?;
        gravar_atomico(&self.root.join(ARQUIVO_RAMOS), serde_json::to_string_pretty(c)?.as_bytes())?;
        Ok(())
    }

    fn evento_ramo(&self, tipo: TipoEvento, id: u64, de: &str, para: &str, motivo: &str) -> PResult<()> {
        self.registrar(tipo, id, de, para, motivo)
    }

    pub fn ramos(&self, id: u64) -> Vec<Ramo> {
        self.cofre().ramos.into_iter().filter(|r| r.doc_id == id).collect()
    }

    pub fn ramo(&self, id: u64, nome: &str) -> PResult<Ramo> {
        self.ramos(id).into_iter().find(|r| r.nome == nome).ok_or(ProjectError::NotFound)
    }

    fn hash_principal(&self, id: u64) -> PResult<String> {
        self.entry(id)?.versions.last().map(|v| v.hash.clone()).ok_or_else(|| ProjectError::Recusado("o documento ainda não tem versão gravada".into()))
    }

    /// Grava o conteúdo como versão imutável (mesma identidade de conteúdo das versões da principal).
    fn gravar_conteudo(&self, doc: &Document) -> PResult<String> {
        let json = doc.to_json();
        let sem_app: String = json.lines().filter(|l| !l.trim_start().starts_with("\"app_version\"")).collect::<Vec<_>>().join("\n");
        let hash = blake3::hash(sem_app.as_bytes()).to_hex()[..16].to_string();
        let caminho = self.root.join(VERSIONS).join(format!("{hash}.fkp"));
        if !caminho.exists() {
            gravar_atomico(&caminho, json.as_bytes())?;
        }
        Ok(hash)
    }

    /// Abre um ramo a partir da versão atual da principal. O ramo ganha uma faixa própria de ids.
    pub fn criar_ramo(&mut self, id: u64, nome: &str) -> PResult<Ramo> {
        self.pode_gravar()?;
        let nome = nome.trim();
        if nome.is_empty() || nome.eq_ignore_ascii_case(PRINCIPAL) || nome.contains(['/', '\\']) {
            return Err(ProjectError::Recusado(format!("nome de ramo inválido: '{nome}'")));
        }
        let mut cofre = self.cofre();
        if cofre.ramos.iter().any(|r| r.doc_id == id && r.nome == nome && r.estado == EstadoRamo::Aberto) {
            return Err(ProjectError::Recusado(format!("já existe um ramo aberto chamado '{nome}'")));
        }
        let base_hash = self.hash_principal(id)?;
        let mut doc = self.load_version(&base_hash)?;
        cofre.proximo_seq += 1;
        let seq = cofre.proximo_seq;
        // faixa própria: ids novos do ramo ficam longe dos da principal e de outros ramos
        let alvo = seq * FAIXA_IDS;
        if doc.peek_id() < alvo {
            doc.reserve_ids(alvo - doc.peek_id());
        }
        let ponta_hash = self.gravar_conteudo(&doc)?;
        let t = now();
        let ramo = Ramo {
            doc_id: id,
            nome: nome.into(),
            autor: self.autor.clone(),
            criado: t,
            atualizado: t,
            estado: EstadoRamo::Aberto,
            base_hash,
            ponta_hash: ponta_hash.clone(),
            seq,
            versoes: vec![Version { hash: ponta_hash.clone(), time: t, note: "ramo criado".into(), author: self.autor.clone(), comment: String::new(), restored_from: None }],
            pedido: None,
        };
        cofre.ramos.push(ramo.clone());
        self.gravar_cofre(&cofre)?;
        self.evento_ramo(TipoEvento::Ramo, id, PRINCIPAL, nome, "ramo criado")?;
        Ok(ramo)
    }

    /// Documento na ponta do ramo, para abrir e editar sem tocar na principal.
    pub fn carregar_ramo(&self, id: u64, nome: &str) -> PResult<Document> {
        self.load_version(&self.ramo(id, nome)?.ponta_hash)
    }

    /// Grava o trabalho no ramo. A principal não muda e outra pessoa só vê isto se pedir o ramo.
    pub fn salvar_no_ramo(&mut self, id: u64, nome: &str, doc: &Document, nota: &str) -> PResult<()> {
        self.pode_gravar()?;
        let mut cofre = self.cofre();
        let hash = self.gravar_conteudo(doc)?;
        let autor = self.autor.clone();
        let r = cofre.ramos.iter_mut().find(|r| r.doc_id == id && r.nome == nome).ok_or(ProjectError::NotFound)?;
        if r.estado != EstadoRamo::Aberto {
            return Err(ProjectError::Recusado(format!("o ramo '{nome}' não está aberto")));
        }
        if r.ponta_hash == hash {
            return Ok(());
        }
        r.ponta_hash = hash.clone();
        r.atualizado = now();
        r.versoes.push(Version { hash: hash.clone(), time: now(), note: nota.into(), author: autor, comment: String::new(), restored_from: None });
        // mudou depois de pedido/aprovação: o pedido volta a ser aberto para nova revisão
        if let Some(p) = &mut r.pedido {
            if p.estado == EstadoPedido::Aprovado && p.hash_ponta != hash {
                p.estado = EstadoPedido::Aberto;
                p.motivo = "o ramo mudou depois da aprovação".into();
            }
        }
        self.gravar_cofre(&cofre)?;
        self.evento_ramo(TipoEvento::Ramo, id, nome, &hash, nota)
    }

    /// O que o ramo mudou desde que nasceu (reaproveita o comparador de B214).
    pub fn comparar_ramo(&self, id: u64, nome: &str) -> PResult<ComparacaoDocs> {
        let r = self.ramo(id, nome)?;
        Ok(comparar_documentos(&self.load_version(&r.base_hash)?, &self.load_version(&r.ponta_hash)?))
    }

    /// Calcula a mesclagem do ramo na principal sem gravar nada (prévia para o lado a lado no 3D).
    pub fn previa_mesclagem(&self, id: u64, nome: &str, escolhas: &HashMap<String, Lado>) -> PResult<Mescla> {
        let r = self.ramo(id, nome)?;
        let base = self.load_version(&r.base_hash)?;
        let nossa = self.load_version(&self.hash_principal(id)?)?;
        let deles = self.load_version(&r.ponta_hash)?;
        mesclar_documentos(&base, &nossa, &deles, escolhas)
    }

    /// Features que cada ramo aberto mexeu e que outro ramo, ou a própria principal, também mexeu:
    /// o aviso aparece antes do conflito.
    pub fn avisos_de_sobreposicao(&self, id: u64) -> PResult<Vec<AvisoSobreposicao>> {
        let abertos: Vec<Ramo> = self.ramos(id).into_iter().filter(|r| r.estado == EstadoRamo::Aberto).collect();
        let principal = self.load_version(&self.hash_principal(id)?)?;
        let tocadas = |c: &ComparacaoDocs| -> BTreeSet<u64> { c.features.iter().filter(|m| m.mudanca != Mudanca::Adicionada).map(|m| m.id).collect() };
        let mut por_ramo = vec![];
        for r in &abertos {
            let base = self.load_version(&r.base_hash)?;
            let ponta = self.load_version(&r.ponta_hash)?;
            por_ramo.push((r, tocadas(&comparar_documentos(&base, &ponta)), tocadas(&comparar_documentos(&base, &principal))));
        }
        let mut avisos = vec![];
        for (i, (r, meu, na_principal)) in por_ramo.iter().enumerate() {
            let comum: Vec<u64> = meu.intersection(na_principal).copied().collect();
            if !comum.is_empty() {
                avisos.push(AvisoSobreposicao { ramo: r.nome.clone(), contra: PRINCIPAL.into(), features: comum });
            }
            for (o, outro, _) in por_ramo.iter().skip(i + 1) {
                let comum: Vec<u64> = meu.intersection(outro).copied().collect();
                if !comum.is_empty() {
                    avisos.push(AvisoSobreposicao { ramo: r.nome.clone(), contra: o.nome.clone(), features: comum });
                }
            }
        }
        Ok(avisos)
    }

    fn exige_papel(&self, papel: &str) -> PResult<()> {
        if self.papeis.iter().any(|p| p == papel) {
            Ok(())
        } else {
            Err(ProjectError::Recusado(format!("a linha principal é protegida: falta o papel '{papel}'")))
        }
    }

    /// Pede a entrada do ramo na principal. Qualquer pessoa com acesso pode pedir.
    pub fn pedir_mesclagem(&mut self, id: u64, nome: &str, mensagem: &str) -> PResult<()> {
        self.pode_gravar()?;
        let mut cofre = self.cofre();
        let autor = self.autor.clone();
        let r = cofre.ramos.iter_mut().find(|r| r.doc_id == id && r.nome == nome).ok_or(ProjectError::NotFound)?;
        if r.estado != EstadoRamo::Aberto {
            return Err(ProjectError::Recusado(format!("o ramo '{nome}' não está aberto")));
        }
        r.pedido = Some(Pedido { autor, tempo: now(), mensagem: mensagem.into(), estado: EstadoPedido::Aberto, revisor: String::new(), motivo: String::new(), hash_ponta: String::new(), hash_principal: String::new() });
        self.gravar_cofre(&cofre)?;
        self.evento_ramo(TipoEvento::Mesclagem, id, nome, PRINCIPAL, &format!("pedido de mesclagem: {mensagem}"))
    }

    /// Revisão do pedido. Aprovar exige o papel `aprovador` e passa pelo checador de qualidade
    /// (`verificar` sobre a mesclagem prevista); a aprovação fica presa à ponta do ramo e à principal.
    pub fn revisar_pedido(&mut self, id: u64, nome: &str, aprovar: bool, motivo: &str, verificar: Verificador) -> PResult<()> {
        self.pode_gravar()?;
        self.exige_papel("aprovador")?;
        let ramo = self.ramo(id, nome)?;
        let pedido = ramo.pedido.clone().ok_or_else(|| ProjectError::Recusado("o ramo não tem pedido de mesclagem".into()))?;
        if pedido.estado != EstadoPedido::Aberto {
            return Err(ProjectError::Recusado("o pedido já foi revisado".into()));
        }
        let hash_principal = self.hash_principal(id)?;
        if aprovar {
            let previa = self.previa_mesclagem(id, nome, &HashMap::new())?;
            let mut problemas = problemas_estruturais(&previa.documento);
            problemas.extend(verificar(&previa.documento));
            if !problemas.is_empty() {
                return Err(ProjectError::Conferencia(problemas.join("; ")));
            }
        }
        let mut cofre = self.cofre();
        let revisor = self.autor.clone();
        let r = cofre.ramos.iter_mut().find(|r| r.doc_id == id && r.nome == nome).ok_or(ProjectError::NotFound)?;
        if let Some(p) = &mut r.pedido {
            p.estado = if aprovar { EstadoPedido::Aprovado } else { EstadoPedido::Recusado };
            p.revisor = revisor;
            p.motivo = motivo.into();
            p.hash_ponta = ramo.ponta_hash.clone();
            p.hash_principal = hash_principal;
        }
        self.gravar_cofre(&cofre)?;
        self.evento_ramo(TipoEvento::Mesclagem, id, nome, if aprovar { "aprovado" } else { "recusado" }, motivo)
    }

    /// Mescla o ramo na principal: cria uma versão nova da principal, nunca reescreve as anteriores.
    /// Exige papel `aprovador` e pedido aprovado para esta ponta; conflitos sem escolha recusam; o
    /// resultado passa pela conferência antes de entrar.
    pub fn mesclar_ramo(&mut self, id: u64, nome: &str, escolhas: &HashMap<String, Lado>, verificar: Verificador) -> PResult<Mescla> {
        self.pode_gravar()?;
        self.exige_papel("aprovador")?;
        let ramo = self.ramo(id, nome)?;
        if ramo.estado != EstadoRamo::Aberto {
            return Err(ProjectError::Recusado(format!("o ramo '{nome}' não está aberto")));
        }
        let hash_principal = self.hash_principal(id)?;
        match &ramo.pedido {
            Some(p) if p.estado == EstadoPedido::Aprovado && p.hash_ponta == ramo.ponta_hash => {}
            Some(p) if p.estado == EstadoPedido::Aprovado => return Err(ProjectError::Recusado("o ramo mudou depois da aprovação: peça nova revisão".into())),
            _ => return Err(ProjectError::Recusado("a linha principal só recebe ramos com pedido de mesclagem aprovado".into())),
        }
        let mescla = self.previa_mesclagem(id, nome, escolhas)?;
        // conflito só passa se a pessoa escolheu um lado para cada um
        let sem_escolha: Vec<&Conflito> = mescla.conflitos.iter().filter(|c| !escolhas.contains_key(&c.chave)).collect();
        if !sem_escolha.is_empty() {
            let lista: Vec<String> = sem_escolha.iter().map(|c| format!("{} (principal: {}, ramo: {})", c.rotulo, c.nossa, c.deles)).collect();
            return Err(ProjectError::Recusado(format!("conflitos sem resolver: {}", lista.join("; "))));
        }
        let mut problemas = problemas_estruturais(&mescla.documento);
        problemas.extend(verificar(&mescla.documento));
        if !problemas.is_empty() {
            return Err(ProjectError::Conferencia(problemas.join("; ")));
        }
        let _ = hash_principal;
        self.save_comentado(id, &mescla.documento, &format!("mesclagem do ramo {nome}"), &format!("ramo '{nome}' de {}", ramo.autor))?;
        let mut cofre = self.cofre();
        if let Some(r) = cofre.ramos.iter_mut().find(|r| r.doc_id == id && r.nome == nome) {
            r.estado = EstadoRamo::Mesclado;
            r.atualizado = now();
        }
        self.gravar_cofre(&cofre)?;
        self.evento_ramo(TipoEvento::Mesclagem, id, nome, PRINCIPAL, &format!("mesclado ({} conflito(s) resolvido(s))", mescla.conflitos.len()))?;
        Ok(mescla)
    }

    /// Descarta o ramo (o conteúdo das versões continua em `.versoes`).
    pub fn descartar_ramo(&mut self, id: u64, nome: &str) -> PResult<()> {
        self.pode_gravar()?;
        let mut cofre = self.cofre();
        let r = cofre.ramos.iter_mut().find(|r| r.doc_id == id && r.nome == nome).ok_or(ProjectError::NotFound)?;
        if r.estado != EstadoRamo::Aberto {
            return Err(ProjectError::Recusado(format!("o ramo '{nome}' não está aberto")));
        }
        r.estado = EstadoRamo::Descartado;
        self.gravar_cofre(&cofre)?;
        self.evento_ramo(TipoEvento::Ramo, id, nome, "", "ramo descartado")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diario;
    use fenkra_core::{BoolOp, FeatureKind, Id, OriginPlane, PlaneRef, PrimKind, Primitive};
    use std::path::PathBuf;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-ramos-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn caixa(d: &mut Document, a: &str) -> Id {
        let s = |x: &str| x.to_string();
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(a), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }))
    }

    fn lado_a(d: &mut Document, id: Id, v: &str) {
        if let FeatureKind::Primitive(p) = &mut d.features.iter_mut().find(|f| f.id == id).unwrap().kind {
            p.a = v.into();
        }
    }

    fn valor_a(d: &Document, id: Id) -> String {
        match &d.features.iter().find(|f| f.id == id).unwrap().kind {
            FeatureKind::Primitive(p) => p.a.clone(),
            _ => unreachable!(),
        }
    }

    fn base_com_duas() -> (Document, Id, Id) {
        let mut d = Document::new("Placa");
        let x = caixa(&mut d, "40");
        let y = caixa(&mut d, "50");
        (d, x, y)
    }

    #[test]
    fn edicoes_em_features_diferentes_mesclam_sem_conflito() {
        let (base, x, y) = base_com_duas();
        let (mut a, mut t) = (base.clone(), base.clone());
        lado_a(&mut a, x, "41");
        lado_a(&mut t, y, "52");
        let m = mesclar_documentos(&base, &a, &t, &HashMap::new()).unwrap();
        assert!(m.limpa(), "{:?}", m.conflitos);
        assert_eq!((valor_a(&m.documento, x).as_str(), valor_a(&m.documento, y).as_str()), ("41", "52"));
    }

    #[test]
    fn mesmo_parametro_nos_dois_lados_e_conflito_e_a_escolha_decide() {
        let (base, x, _) = base_com_duas();
        let (mut a, mut t) = (base.clone(), base.clone());
        lado_a(&mut a, x, "41");
        lado_a(&mut t, x, "99");
        let m = mesclar_documentos(&base, &a, &t, &HashMap::new()).unwrap();
        assert_eq!(m.conflitos.len(), 1);
        assert_eq!((m.conflitos[0].base.as_str(), m.conflitos[0].nossa.as_str(), m.conflitos[0].deles.as_str()), ("40", "41", "99"));
        assert_eq!(m.conflitos[0].feature, Some(x.0));
        assert_eq!(valor_a(&m.documento, x), "41", "sem escolha vale a principal");
        let esc = HashMap::from([(m.conflitos[0].chave.clone(), Lado::Deles)]);
        let m2 = mesclar_documentos(&base, &a, &t, &esc).unwrap();
        assert_eq!(valor_a(&m2.documento, x), "99");
    }

    #[test]
    fn mudar_para_o_mesmo_valor_nao_e_conflito() {
        let (base, x, _) = base_com_duas();
        let (mut a, mut t) = (base.clone(), base.clone());
        lado_a(&mut a, x, "44");
        lado_a(&mut t, x, "44");
        assert!(mesclar_documentos(&base, &a, &t, &HashMap::new()).unwrap().limpa());
    }

    #[test]
    fn resultado_e_igual_nas_duas_ordens_de_mesclagem() {
        let (base, x, y) = base_com_duas();
        let (mut a, mut t) = (base.clone(), base.clone());
        lado_a(&mut a, x, "41");
        // cada lado cria uma feature nova e um parâmetro; o ramo usa uma faixa de ids própria
        caixa(&mut a, "60");
        let alvo = t.peek_id() + FAIXA_IDS;
        t.reserve_ids(alvo - t.peek_id());
        let nova = caixa(&mut t, "70");
        lado_a(&mut t, y, "52");
        t.add_param("Largura", "10", fenkra_core::Unit::Comprimento).unwrap();
        let ab = mesclar_documentos(&base, &a, &t, &HashMap::new()).unwrap();
        let ba = mesclar_documentos(&base, &t, &a, &HashMap::new()).unwrap();
        assert!(ab.limpa() && ba.limpa());
        assert_eq!(ab.documento.to_json(), ba.documento.to_json(), "a ordem de mesclagem não pode mudar o resultado");
        assert_eq!(ab.documento.features.len(), 4);
        assert!(ab.documento.features.iter().any(|f| f.id == nova));
        assert_eq!(ab.documento.params.len(), t.params.len());
    }

    #[test]
    fn remover_de_um_lado_e_editar_do_outro_e_conflito() {
        let (base, x, y) = base_com_duas();
        let (mut a, mut t) = (base.clone(), base.clone());
        a.features.retain(|f| f.id != x);
        lado_a(&mut t, x, "45");
        let m = mesclar_documentos(&base, &a, &t, &HashMap::new()).unwrap();
        assert_eq!(m.conflitos.len(), 1);
        assert!(m.documento.features.iter().all(|f| f.id != x), "sem escolha vale a principal (removida)");
        let esc = HashMap::from([(m.conflitos[0].chave.clone(), Lado::Deles)]);
        assert!(mesclar_documentos(&base, &a, &t, &esc).unwrap().documento.features.iter().any(|f| f.id == x));
        // remoção sem edição do outro lado entra sozinha
        let mut limpo = base.clone();
        limpo.features.retain(|f| f.id != y);
        let m = mesclar_documentos(&base, &limpo, &base, &HashMap::new()).unwrap();
        assert!(m.limpa() && m.documento.features.len() == 1);
    }

    fn projeto(nome: &str) -> (Project, u64, PathBuf) {
        let raiz = tmp(nome);
        let mut p = Project::create(&raiz, "Teste").unwrap();
        p.autor = "ana".into();
        let (d, _, _) = base_com_duas();
        let id = p.add_document(d).unwrap();
        (p, id, raiz)
    }

    fn sem_problema(_: &Document) -> Vec<String> {
        vec![]
    }

    #[test]
    fn ramo_isola_o_trabalho_e_a_principal_so_recebe_com_pedido_aprovado() {
        let (mut p, id, raiz) = projeto("fluxo");
        let principal_antes = p.load(id).unwrap();
        let x = principal_antes.features[0].id;
        p.criar_ramo(id, "furos").unwrap();
        let mut doc_ramo = p.carregar_ramo(id, "furos").unwrap();
        lado_a(&mut doc_ramo, x, "41");
        p.salvar_no_ramo(id, "furos", &doc_ramo, "ajuste").unwrap();
        // a principal não vê o trabalho do ramo
        assert_eq!(valor_a(&p.load(id).unwrap(), x), "40");
        assert_eq!(p.entry(id).unwrap().versions.len(), 1);
        // a principal segue andando em outra feature
        let mut dp = p.load(id).unwrap();
        let y = dp.features[1].id;
        lado_a(&mut dp, y, "55");
        p.save(id, &dp, "principal andou").unwrap();
        // sem pedido aprovado, não entra
        let e = p.mesclar_ramo(id, "furos", &HashMap::new(), &sem_problema).unwrap_err();
        assert!(e.to_string().contains("pedido de mesclagem aprovado"), "{e}");
        p.pedir_mesclagem(id, "furos", "pronto").unwrap();
        // sem o papel, não aprova nem mescla
        p.papeis = vec!["projetista".into()];
        assert!(p.revisar_pedido(id, "furos", true, "ok", &sem_problema).is_err());
        p.papeis = vec!["projetista".into(), "aprovador".into()];
        // o checador de qualidade barra a aprovação
        let barra = |_: &Document| vec!["volume zero".to_string()];
        assert!(p.revisar_pedido(id, "furos", true, "ok", &barra).is_err());
        p.revisar_pedido(id, "furos", true, "ok", &sem_problema).unwrap();
        let m = p.mesclar_ramo(id, "furos", &HashMap::new(), &sem_problema).unwrap();
        assert!(m.limpa());
        let depois = p.load(id).unwrap();
        assert_eq!((valor_a(&depois, x).as_str(), valor_a(&depois, y).as_str()), ("41", "55"));
        assert_eq!(p.entry(id).unwrap().versions.len(), 3, "a mesclagem é uma versão nova");
        assert_eq!(p.ramo(id, "furos").unwrap().estado, EstadoRamo::Mesclado);
        let tipos: Vec<_> = diario::ler(&raiz).0.into_iter().map(|e| e.tipo).collect();
        assert!(tipos.contains(&TipoEvento::Mesclagem) && tipos.contains(&TipoEvento::Ramo));
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn mudar_o_ramo_depois_da_aprovacao_invalida_a_aprovacao() {
        let (mut p, id, raiz) = projeto("invalida");
        p.criar_ramo(id, "r").unwrap();
        p.pedir_mesclagem(id, "r", "").unwrap();
        p.revisar_pedido(id, "r", true, "", &sem_problema).unwrap();
        let mut d = p.carregar_ramo(id, "r").unwrap();
        let x = d.features[0].id;
        lado_a(&mut d, x, "77");
        p.salvar_no_ramo(id, "r", &d, "depois da aprovação").unwrap();
        assert!(p.mesclar_ramo(id, "r", &HashMap::new(), &sem_problema).is_err());
        assert_eq!(p.ramo(id, "r").unwrap().pedido.unwrap().estado, EstadoPedido::Aberto);
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn conflito_exige_escolha_e_resultado_quebrado_nao_entra() {
        let (mut p, id, raiz) = projeto("conflito");
        let x = p.load(id).unwrap().features[0].id;
        p.criar_ramo(id, "r").unwrap();
        let mut d = p.carregar_ramo(id, "r").unwrap();
        lado_a(&mut d, x, "99");
        p.salvar_no_ramo(id, "r", &d, "").unwrap();
        let mut dp = p.load(id).unwrap();
        lado_a(&mut dp, x, "41");
        p.save(id, &dp, "").unwrap();
        p.pedir_mesclagem(id, "r", "").unwrap();
        p.revisar_pedido(id, "r", true, "", &sem_problema).unwrap();
        let e = p.mesclar_ramo(id, "r", &HashMap::new(), &sem_problema).unwrap_err();
        assert!(e.to_string().contains("conflitos sem resolver"), "{e}");
        let chave = p.previa_mesclagem(id, "r", &HashMap::new()).unwrap().conflitos[0].chave.clone();
        let esc = HashMap::from([(chave, Lado::Deles)]);
        // a conferência pós-mescla reprova: nada entra
        let versoes = p.entry(id).unwrap().versions.len();
        assert!(p.mesclar_ramo(id, "r", &esc, &|_| vec!["referência quebrada".into()]).is_err());
        assert_eq!(p.entry(id).unwrap().versions.len(), versoes);
        p.mesclar_ramo(id, "r", &esc, &sem_problema).unwrap();
        assert_eq!(valor_a(&p.load(id).unwrap(), x), "99");
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn dois_ramos_criando_features_nao_colidem_ids_e_avisam_sobreposicao() {
        let (mut p, id, raiz) = projeto("avisos");
        let x = p.load(id).unwrap().features[0].id;
        p.criar_ramo(id, "a").unwrap();
        p.criar_ramo(id, "b").unwrap();
        let (mut da, mut db) = (p.carregar_ramo(id, "a").unwrap(), p.carregar_ramo(id, "b").unwrap());
        let (na, nb) = (caixa(&mut da, "1"), caixa(&mut db, "2"));
        assert_ne!(na, nb, "faixas de ids próprias");
        lado_a(&mut da, x, "11");
        lado_a(&mut db, x, "22");
        p.salvar_no_ramo(id, "a", &da, "").unwrap();
        p.salvar_no_ramo(id, "b", &db, "").unwrap();
        let av = p.avisos_de_sobreposicao(id).unwrap();
        assert_eq!(av.len(), 1);
        assert_eq!((av[0].ramo.as_str(), av[0].contra.as_str(), av[0].features.clone()), ("a", "b", vec![x.0]));
        let c = p.comparar_ramo(id, "a").unwrap();
        assert_eq!(c.features.len(), 2);
        assert!(p.criar_ramo(id, "principal").is_err());
        assert!(p.criar_ramo(id, "a").is_err());
        let _ = std::fs::remove_dir_all(raiz);
    }
}
