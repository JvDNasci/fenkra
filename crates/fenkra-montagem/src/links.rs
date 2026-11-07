// SPDX-License-Identifier: LGPL-2.1-or-later
//! Links entre documentos vistos pela montagem (B70).
//!
//! Um link mora na peça que recebe a cópia (`ExternalRef` em `fenkra_core`): nome da origem, arquivo,
//! publicação, a cópia e, se foi feito na montagem, a transformação do sistema da origem para o
//! sistema da peça. Essa transformação não é um número guardado de uma vez por todas: ela é a
//! posição relativa dos dois componentes. Por isso [`atualizar`] a deriva das posições atuais a
//! cada passada, e a cópia acompanha a peça de origem quando ela se move.
//!
//! O estado que a pessoa vê ([`EstadoLink`]) sai do `LinkState` gravado e de onde o link está:
//! - `->`  em contexto: a origem está na montagem aberta (cópia e posição vivas);
//! - `->?` fora de contexto: a origem existe, mas não está nesta montagem (só a forma é viva);
//! - `->*` travado: a pessoa congelou a cópia; destravar volta a atualizar;
//! - `->x` quebrado: a pessoa cortou o link; a cópia fica e não há volta. É gravado sem caminho de
//!   origem, para `refresh_link` nunca reativá-lo sozinho;
//! - `->!` pendurado: o link quer atualizar e a origem ou a publicação sumiu. Volta sozinho se ela
//!   reaparecer.
//!
//! A política de cada link ([`Politica`]) mora na montagem, porque diz respeito a como este
//! conjunto se atualiza, e não à peça.
//!
//! Os documentos formam um grafo que precisa ser acíclico: uma atualização que entrasse num ciclo
//! nunca terminaria. [`depende_de`] é a verificação feita antes de criar um link ou um subconjunto.

use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_core::{Document, ExternalRef, LinkState, LinkXf, LinkedGeo};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Quando o link atualiza.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Politica {
    /// a cada mudança de posição ou da origem
    #[default]
    Automatica,
    /// ao abrir a montagem (e no botão Atualizar)
    AoAbrir,
    /// só no botão Atualizar
    Manual,
}

impl Politica {
    pub const TODAS: [Politica; 3] = [Politica::Automatica, Politica::AoAbrir, Politica::Manual];
    pub fn label(self) -> &'static str {
        match self {
            Politica::Automatica => "automática",
            Politica::AoAbrir => "ao abrir",
            Politica::Manual => "manual",
        }
    }
}

/// Política escolhida para um link. Link sem registro é automático.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PoliticaLink {
    /// caminho da peça que recebe o link, como está no nó
    pub alvo: String,
    pub ext: Id,
    pub politica: Politica,
}

/// O que disparou a atualização.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gatilho {
    /// a montagem foi resolvida: só links automáticos, e só se algo mudou
    Posicao,
    Abrir,
    /// botão Atualizar: todos, mesmo sem mudança aparente
    Manual,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EstadoLink {
    EmContexto,
    ForaDeContexto,
    Travado,
    Quebrado,
    Pendurado,
    Desativado,
}

impl EstadoLink {
    pub fn simbolo(self) -> &'static str {
        match self {
            EstadoLink::EmContexto => "->",
            EstadoLink::ForaDeContexto => "->?",
            EstadoLink::Travado => "->*",
            EstadoLink::Quebrado => "->x",
            EstadoLink::Pendurado => "->!",
            EstadoLink::Desativado => "->-",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            EstadoLink::EmContexto => "em contexto: a origem está nesta montagem",
            EstadoLink::ForaDeContexto => "fora de contexto: a origem não está nesta montagem",
            EstadoLink::Travado => "travado: a cópia não atualiza",
            EstadoLink::Quebrado => "quebrado: sem origem, a cópia fica como está",
            EstadoLink::Pendurado => "pendurado: a origem ou a publicação sumiu",
            EstadoLink::Desativado => "desativado provisoriamente",
        }
    }
}

/// Uma linha da lista de links de uma peça.
#[derive(Clone, Debug, PartialEq)]
pub struct LinhaLink {
    pub ext: Id,
    pub nome: String,
    pub origem: String,
    pub publicacao: String,
    pub estado: EstadoLink,
    pub politica: Politica,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AcaoLink {
    Travar,
    Destravar,
    Quebrar,
    Isolar,
}

/// O que uma atualização fez.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioLinks {
    pub atualizados: usize,
    /// links que a atualização encontrou sem origem
    pub pendurados: usize,
    /// links que ficaram para outro momento pela política
    pub adiados: usize,
    pub travados: usize,
    /// peças regravadas
    pub pecas: Vec<PathBuf>,
    /// subconjuntos regravados: peça que mudou lá dentro move as vizinhas presas a ela
    pub subconjuntos: Vec<PathBuf>,
    /// subconjuntos que a atualização não alcançou: sem isto, "0 peças" pareceria sucesso
    pub ilegiveis: Vec<PathBuf>,
    /// subconjuntos cujas restrições não fecharam com a forma nova; o arquivo fica como estava
    pub sem_solucao: Vec<PathBuf>,
}

impl Assembly {
    pub fn politica_link(&self, alvo: &str, ext: Id) -> Politica {
        self.politicas_link.iter().find(|p| p.alvo == alvo && p.ext == ext).map(|p| p.politica).unwrap_or_default()
    }

    pub fn definir_politica_link(&mut self, alvo: &str, ext: Id, politica: Politica) {
        self.politicas_link.retain(|p| !(p.alvo == alvo && p.ext == ext));
        if politica != Politica::default() {
            self.politicas_link.push(PoliticaLink { alvo: alvo.into(), ext, politica });
        }
    }
}

/// Ligar `peca` à origem no caminho `path` fecharia um ciclo?
pub fn fecha_ciclo(peca: &Path, path: &str, raiz: Option<&Path>) -> bool {
    depende_de(&resolver(peca, path, raiz), peca, raiz)
}

/// Caminho do arquivo de origem de um link. Relativo: primeiro a pasta da peça, depois `raiz`.
pub fn resolver(peca: &Path, path: &str, raiz: Option<&Path>) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let perto = peca.parent().map(|d| d.join(p)).unwrap_or_else(|| p.to_path_buf());
    match raiz {
        Some(r) if !perto.exists() => r.join(p),
        _ => perto,
    }
}

fn canonico(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Documentos que este usa diretamente: as origens dos links de uma peça, os componentes de uma
/// montagem.
fn usados(arquivo: &Path, raiz: Option<&Path>) -> Vec<PathBuf> {
    let Ok(txt) = std::fs::read_to_string(arquivo) else { return vec![] };
    if arquivo.extension().is_some_and(|e| e == "fka") {
        let pasta = arquivo.parent();
        let Ok(a) = Assembly::from_json(&txt) else { return vec![] };
        return a.nodes.iter().filter_map(|n| match &n.kind {
            NodeKind::Part { path } | NodeKind::SubAssembly { path } => Some(full_path(pasta, path)),
            NodeKind::Group => None,
        }).collect();
    }
    let Ok(d) = Document::from_json(&txt) else { return vec![] };
    // o furo de série aponta para a montagem que o manda, mas não depende dela para existir: se
    // contasse, toda peça com furo de montagem fecharia ciclo com a própria montagem
    d.external.iter().filter(|e| !e.path.is_empty() && !crate::furos::e_recurso(e)).map(|e| resolver(arquivo, &e.path, raiz)).collect()
}

/// `origem` usa `alvo`, direta ou indiretamente? Ligar `alvo` a `origem` nesse caso fecha um ciclo.
/// Caminha pelo grafo inteiro: um limite de profundidade deixaria passar ciclos longos.
pub fn depende_de(origem: &Path, alvo: &Path, raiz: Option<&Path>) -> bool {
    let alvo = canonico(alvo);
    let mut vistos: HashSet<PathBuf> = HashSet::new();
    let mut pilha = vec![canonico(origem)];
    while let Some(f) = pilha.pop() {
        if f == alvo {
            return true;
        }
        if vistos.insert(f.clone()) {
            pilha.extend(usados(&f, raiz).iter().map(|p| canonico(p)));
        }
    }
    false
}

/// Peças da montagem sem repetir arquivo: (primeiro componente, caminho do nó, caminho completo).
fn pecas(a: &Assembly, dir: Option<&Path>) -> Vec<(Id, String, PathBuf)> {
    let mut vistos: HashSet<PathBuf> = HashSet::new();
    a.nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some((n.id, path.clone(), full_path(dir, path))),
            _ => None,
        })
        .filter(|(_, _, f)| vistos.insert(f.clone()))
        .collect()
}

/// Componente da origem na montagem: o de mesmo nome entre as instâncias do arquivo, senão a primeira.
fn no_da_origem<'a>(a: &'a Assembly, dir: Option<&Path>, arquivo: &Path, nome: &str) -> Option<&'a Node> {
    let alvo = canonico(arquivo);
    let mesmas: Vec<&Node> = a.nodes.iter().filter(|n| matches!(&n.kind, NodeKind::Part { path } if canonico(&full_path(dir, path)) == alvo)).collect();
    mesmas.iter().find(|n| n.name == nome).or(mesmas.first()).copied()
}

fn estado(a: &Assembly, dir: Option<&Path>, peca: &Path, e: &ExternalRef) -> EstadoLink {
    match e.state {
        LinkState::Congelado => return EstadoLink::Travado,
        LinkState::Desativado => return EstadoLink::Desativado,
        LinkState::Quebrado if e.path.is_empty() => return EstadoLink::Quebrado,
        LinkState::Quebrado => return EstadoLink::Pendurado,
        LinkState::Ativo => {}
    }
    if e.path.is_empty() {
        return EstadoLink::Quebrado;
    }
    let src = resolver(peca, &e.path, dir);
    if crate::furos::e_recurso(e) && e.source == a.name {
        // a origem é esta própria montagem, não um componente dela
        EstadoLink::EmContexto
    } else if !src.exists() {
        EstadoLink::Pendurado
    } else if no_da_origem(a, dir, &src, &e.source).is_some() {
        EstadoLink::EmContexto
    } else {
        EstadoLink::ForaDeContexto
    }
}

/// O link `e`, da peça `peca`, aponta para o arquivo `origem`?
pub(crate) fn usa_origem(peca: &Path, e: &ExternalRef, origem: &Path, dir: Option<&Path>) -> bool {
    !e.path.is_empty() && !crate::furos::e_recurso(e) && canonico(&resolver(peca, &e.path, dir)) == canonico(origem)
}

/// Publicação como a pessoa lê: o furo de série tem nome interno.
pub fn rotulo_publicacao(p: &str) -> &str {
    if p.starts_with(crate::furos::PREFIXO) {
        "furo da montagem"
    } else {
        p
    }
}

/// Lista de links da peça de um componente.
pub fn links_da_peca(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, comp: Id) -> Vec<LinhaLink> {
    let Some(NodeKind::Part { path }) = a.node(comp).map(|n| &n.kind) else { return vec![] };
    let full = full_path(dir, path);
    let Ok(p) = cache.get(&full, false) else { return vec![] };
    p.doc
        .external
        .iter()
        .map(|e| LinhaLink { ext: e.id, nome: e.name.clone(), origem: e.source.clone(), publicacao: e.publication.clone(), estado: estado(a, dir, &full, e), politica: a.politica_link(path, e.id) })
        .collect()
}

/// Grava a peça pelo cache: o arquivo, ou a memória se ela ainda não tem arquivo.
pub fn gravar_peca(full: &Path, doc: &Document, cache: &mut PartCache) -> Result<(), String> {
    gravar(full, doc, cache)
}

pub(crate) fn gravar(full: &Path, doc: &Document, cache: &mut PartCache) -> Result<(), String> {
    // peça nova ainda sem arquivo: o disco só a conhece no Salvar da montagem
    if cache.e_pendente(full) {
        cache.definir_pendente(full, doc.clone());
        return Ok(());
    }
    std::fs::write(full, doc.to_json()).map_err(|e| format!("{}: {e}", full.display()))?;
    cache.forget(full);
    Ok(())
}

/// Travar, Destravar, Quebrar ou Isolar um link da peça do componente `comp`.
pub fn aplicar(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, comp: Id, ext: Id, acao: AcaoLink) -> Result<(), String> {
    let Some(NodeKind::Part { path }) = a.node(comp).map(|n| &n.kind) else { return Err("o componente não é uma peça".into()) };
    let full = full_path(dir, path);
    let mut doc = cache.get(&full, false)?.doc.clone();
    if acao == AcaoLink::Isolar {
        doc.isolate_link(ext);
        return gravar(&full, &doc, cache);
    }
    let e = doc.external.iter_mut().find(|e| e.id == ext).ok_or("link não encontrado")?;
    if e.path.is_empty() {
        return Err("link quebrado não volta: use Isolar para ficar só com a cópia".into());
    }
    match acao {
        AcaoLink::Travar => e.state = LinkState::Congelado,
        AcaoLink::Destravar => e.state = LinkState::Ativo,
        AcaoLink::Quebrar => {
            e.state = LinkState::Quebrado;
            e.path.clear();
        }
        AcaoLink::Isolar => {}
    }
    gravar(&full, &doc, cache)
}

fn modificado(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Sistema da origem → sistema da peça, pelas posições atuais.
fn xf_atual(origem: &Placement, alvo: &Placement) -> LinkXf {
    let r = alvo.inverse().then(origem);
    [r.origin, r.rot[0], r.rot[1], r.rot[2]]
}

fn xf_igual(a: &LinkXf, b: &LinkXf) -> bool {
    a.iter().zip(b).all(|(u, v)| norm(sub(*u, *v)) < 1e-9)
}

/// Montagem em que uma peça é atualizada. `dir` acha os componentes dela e `raiz`, a pasta da
/// montagem aberta, é o segundo lugar onde procurar a origem de um link relativo.
#[derive(Clone, Copy)]
struct Contexto<'a> {
    a: &'a Assembly,
    dir: Option<&'a Path>,
    raiz: Option<&'a Path>,
}

/// Subconjunto lido do disco.
struct Sub {
    arquivo: PathBuf,
    a: Assembly,
    dir: Option<PathBuf>,
}

impl Sub {
    fn contexto<'a>(&'a self, raiz: Option<&'a Path>) -> Contexto<'a> {
        Contexto { a: &self.a, dir: self.dir.as_deref(), raiz }
    }
}

/// Subconjuntos em qualquer nível, cada um antes dos que ele contém. Arquivo já visto não é lido de
/// novo: é o que segura um ciclo e o subconjunto usado duas vezes.
fn subconjuntos(a: &Assembly, dir: Option<&Path>, vistos: &mut HashSet<PathBuf>, rel: &mut RelatorioLinks) -> Vec<Sub> {
    let mut lista = vec![];
    for n in &a.nodes {
        let NodeKind::SubAssembly { path } = &n.kind else { continue };
        let arquivo = full_path(dir, path);
        if !vistos.insert(canonico(&arquivo)) {
            continue;
        }
        let lido = std::fs::read_to_string(&arquivo).map_err(|e| e.to_string()).and_then(|t| Assembly::from_json(&t));
        let Ok(sub) = lido else {
            rel.ilegiveis.push(arquivo);
            continue;
        };
        let pasta = arquivo.parent().map(Path::to_path_buf);
        let dentro = subconjuntos(&sub, pasta.as_deref(), vistos, rel);
        lista.push(Sub { arquivo, a: sub, dir: pasta });
        lista.extend(dentro);
    }
    lista
}

/// Atualiza os links da montagem. Abrir e o botão Atualizar descem pelos subconjuntos: a peça de um
/// subconjunto recebe a mudança do mestre sem ninguém abrir o `.fka` dela. O gatilho `Posicao` fica
/// neste nível, porque mover um componente aqui não muda nada dentro de um subconjunto, e ele roda
/// a cada solução.
pub fn atualizar(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, gatilho: Gatilho) -> RelatorioLinks {
    let mut rel = RelatorioLinks::default();
    let topo = Contexto { a, dir, raiz: dir };
    if gatilho == Gatilho::Posicao {
        passadas(&[topo], cache, gatilho, None, &mut rel);
        return rel;
    }
    let mut vistos = HashSet::new();
    let mut subs = subconjuntos(a, dir, &mut vistos, &mut rel);
    let mut todos = vec![topo];
    todos.extend(subs.iter().map(|s| s.contexto(dir)));
    passadas(&todos, cache, gatilho, None, &mut rel);
    // de dentro para fora: a lista traz cada subconjunto antes dos que ele contém
    for s in subs.iter_mut().rev() {
        resolver_subconjunto(s, dir, cache, &mut rel);
    }
    rel
}

/// Peça que mudou dentro de um subconjunto carrega as restrições presas às faces dela. Sem resolver
/// aqui, a forma nova chegaria e as vizinhas ficariam onde estavam, até alguém abrir o `.fka`.
fn resolver_subconjunto(s: &mut Sub, raiz: Option<&Path>, cache: &mut PartCache, rel: &mut RelatorioLinks) {
    let dir = s.dir.clone();
    if !pecas(&s.a, dir.as_deref()).iter().any(|(_, _, f)| rel.pecas.contains(f)) {
        return;
    }
    let antes = s.a.to_json();
    crate::load::refresh_refs(&mut s.a, cache, dir.as_deref());
    if !crate::solve::solve(&mut s.a).converged {
        rel.sem_solucao.push(s.arquivo.clone());
        return;
    }
    // a solução pode ter movido a origem de um link em contexto
    passadas(&[s.contexto(raiz)], cache, Gatilho::Posicao, None, rel);
    let agora = s.a.to_json();
    if agora != antes && gravar_montagem(&s.arquivo, &agora).is_ok() {
        rel.subconjuntos.push(s.arquivo.clone());
    }
}

/// Grava num arquivo ao lado e troca: uma falha no meio não pode estragar o subconjunto.
fn gravar_montagem(arquivo: &Path, json: &str) -> std::io::Result<()> {
    let temp = arquivo.with_extension("fka.tmp");
    std::fs::write(&temp, json)?;
    std::fs::rename(&temp, arquivo)
}

/// Atualiza só um link (botão da lista).
pub fn atualizar_um(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, comp: Id, ext: Id) -> RelatorioLinks {
    let mut rel = RelatorioLinks::default();
    passadas(&[Contexto { a, dir, raiz: dir }], cache, Gatilho::Manual, Some((comp, ext)), &mut rel);
    rel
}

/// Repete enquanto alguma peça mudar: A → B → C pede duas passadas se C vier antes de B na árvore.
/// Com o grafo acíclico, o número de peças é teto de sobra. Peça usada em mais de uma montagem é
/// atualizada só na primeira: duas posições relativas fariam a cópia alternar sem parar.
fn passadas(contextos: &[Contexto], cache: &mut PartCache, gatilho: Gatilho, so: Option<(Id, Ext)>, rel: &mut RelatorioLinks) {
    let mut vistos: HashSet<PathBuf> = HashSet::new();
    let alvos: Vec<(Contexto, Id, String, PathBuf)> = contextos
        .iter()
        .flat_map(|c| pecas(c.a, c.dir).into_iter().map(move |(id, rotulo, full)| (*c, id, rotulo, full)))
        .filter(|(_, _, _, f)| vistos.insert(f.clone()))
        .collect();
    for k in 0..=alvos.len() {
        let mut mudou = false;
        for (ctx, comp, rotulo, full) in &alvos {
            if so.is_some_and(|(c, _)| c != *comp) {
                continue;
            }
            // peça sem link nem é calculada: abrir uma montagem grande não pode passar por aqui
            let Some(mut doc) = cache.documento(full).ok().filter(|d| !d.external.is_empty()).cloned() else { continue };
            let antes = (doc.external.clone(), doc.params.clone());
            for e in antes.0.iter().filter(|e| so.is_none_or(|(_, x)| x == e.id)) {
                atualizar_link(*ctx, cache, gatilho, *comp, rotulo, full, &mut doc, e, rel, k == 0);
            }
            if doc.external != antes.0 || doc.params != antes.1 {
                if gravar(full, &doc, cache).is_ok() && !rel.pecas.contains(full) {
                    rel.pecas.push(full.clone());
                }
                mudou = true;
            }
        }
        if !mudou {
            break;
        }
    }
}

type Ext = Id;

#[allow(clippy::too_many_arguments)]
fn atualizar_link(ctx: Contexto, cache: &mut PartCache, gatilho: Gatilho, comp: Id, rotulo: &str, full: &Path, doc: &mut Document, e: &ExternalRef, rel: &mut RelatorioLinks, conta: bool) {
    let Contexto { a, dir, raiz } = ctx;
    if e.path.is_empty() || e.state == LinkState::Desativado || crate::furos::e_recurso(e) {
        return;
    }
    if e.state == LinkState::Congelado {
        rel.travados += usize::from(conta);
        return;
    }
    let vale = match a.politica_link(rotulo, e.id) {
        Politica::Automatica => true,
        Politica::AoAbrir => gatilho != Gatilho::Posicao,
        Politica::Manual => gatilho == Gatilho::Manual,
    };
    if !vale {
        rel.adiados += usize::from(conta);
        return;
    }
    let src = resolver(full, &e.path, raiz);
    // a transformação nasce das posições de agora, se os dois estão nesta montagem
    let novo_xf = e.xform.and_then(|_| {
        let (ns, nt) = (no_da_origem(a, dir, &src, &e.source)?, a.node(comp)?);
        Some(xf_atual(&ns.placement, &nt.placement))
    });
    let xf_mudou = match (&novo_xf, &e.xform) {
        (Some(n), Some(v)) => !xf_igual(n, v),
        _ => false,
    };
    let origem_nova = match (modificado(&src), modificado(full)) {
        (Some(s), Some(t)) => s > t,
        _ => true,
    };
    let forca = gatilho == Gatilho::Manual || (gatilho == Gatilho::Abrir && e.state == LinkState::Quebrado);
    if !forca && !xf_mudou && !origem_nova {
        return;
    }
    // pelo nome de agora ou por um apelido, e o link passa a guardar o nome de agora
    let achada = cache.get(&src, false).ok().and_then(|p| {
        let atual = crate::publicacoes::nome_atual(&p.doc, &e.publication).to_string();
        crate::publicacoes::com_apelidos(&p.doc).into_iter().find(|(n, _)| *n == e.publication).map(|(_, g)| (atual, g))
    });
    let geo = achada.as_ref().map(|(_, g)| g.clone());
    if let (Some((atual, _)), Some(d)) = (&achada, doc.external.iter_mut().find(|d| d.id == e.id)) {
        d.publication = atual.clone();
    }
    // publicação que trocou de tipo não serve: a peça usa a cópia como plano, curva...
    let geo: Option<LinkedGeo> = geo.filter(|g| e.snapshot.as_ref().is_none_or(|s| std::mem::discriminant(s) == std::mem::discriminant(g)));
    let sumiu = geo.is_none();
    if let (Some(x), Some(d)) = (novo_xf, doc.external.iter_mut().find(|d| d.id == e.id)) {
        d.xform = Some(x);
    }
    doc.refresh_link(e.id, geo);
    if sumiu {
        rel.pendurados += usize::from(e.state != LinkState::Quebrado);
    } else if doc.external.iter().find(|d| d.id == e.id).is_some_and(|d| d != e) {
        rel.atualizados += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, RefKind, Sketch, SketchData};

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_links_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Peça que só publica um plano `z` acima da própria origem.
    fn mestre(z: &str) -> Document {
        let mut d = Document::new("Mestre");
        let id = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), z);
        let r = d.references.iter_mut().find(|r| r.id == id).unwrap();
        r.name = "Base".into();
        r.published = true;
        d
    }

    fn bloco_sobre(d: &mut Document, plano: Id) {
        let mut n = 8_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [-5., -5.], [5., 5.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(plano), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("4".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    }

    /// Montagem com Mestre (fixo, em `z0`) e Filha (na origem). A filha tem um bloco de 10 × 10 × 4
    /// apoiado no plano publicado, copiado por link no contexto da montagem.
    fn conjunto(dir: &Path, z0: f64) -> (Assembly, Id, Id) {
        std::fs::write(dir.join("mestre.fkp"), mestre("10").to_json()).unwrap();
        let mut a = Assembly::new("Links");
        let m = a.add_component("Mestre", NodeKind::Part { path: "mestre.fkp".into() }, Placement { origin: [0., 0., z0], ..Default::default() }, None);
        let f = a.add_component("Filha", NodeKind::Part { path: "filha.fkp".into() }, Placement::default(), None);
        let xf = xf_atual(&a.node(m).unwrap().placement, &a.node(f).unwrap().placement);
        let geo = fenkra_solidos::publications(&mestre("10")).remove(0).1;
        let mut d = Document::new("Filha");
        let plano = d.add_link_xf("Mestre", dir.join("mestre.fkp").to_str().unwrap(), "Base", geo, xf);
        bloco_sobre(&mut d, plano);
        std::fs::write(dir.join("filha.fkp"), d.to_json()).unwrap();
        (a, m, f)
    }

    fn filha(dir: &Path) -> Document {
        Document::from_json(&std::fs::read_to_string(dir.join("filha.fkp")).unwrap()).unwrap()
    }

    /// Topo e base (z) e volume do bloco da filha.
    fn caixa(dir: &Path) -> (f64, f64, f64) {
        let d = filha(dir);
        let r = fenkra_solidos::recompute(&d);
        assert!(r.status.values().all(|s| matches!(s, fenkra_solidos::Status::Ok)), "{:?}", r.status);
        let s = fenkra_solidos::final_shape(&d).expect("forma");
        let (lo, hi) = s.bbox().unwrap();
        (lo[2], hi[2], s.mass().volume)
    }

    fn sem_kernel() -> bool {
        !fenkra_kernel::AVAILABLE
    }

    #[test]
    fn a_copia_acompanha_a_posicao_da_origem() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("posicao");
        let (mut a, m, f) = conjunto(&dir, 0.);
        let mut cache = PartCache::default();
        let (z0, z1, v) = caixa(&dir);
        assert!((z0 - 10.).abs() < 1e-6 && (z1 - 14.).abs() < 1e-6 && (v - 400.).abs() < 1e-6, "{z0} {z1} {v}");

        assert_eq!(links_da_peca(&a, Some(&dir), &mut cache, f)[0].estado, EstadoLink::EmContexto);

        // nada mudou: nem regrava
        let rel = atualizar(&a, Some(&dir), &mut cache, Gatilho::Posicao);
        assert!(rel.pecas.is_empty(), "{rel:?}");

        // a origem sobe 7: o plano vai junto, e a peça mantém o volume
        a.node_mut(m).unwrap().placement.origin[2] = 7.;
        let rel = atualizar(&a, Some(&dir), &mut cache, Gatilho::Posicao);
        assert_eq!(rel.atualizados, 1, "{rel:?}");
        let (z0, z1, v) = caixa(&dir);
        assert!((z0 - 17.).abs() < 1e-6 && (z1 - 21.).abs() < 1e-6 && (v - 400.).abs() < 1e-6, "{z0} {z1} {v}");

        // mover a peça que recebe faz o contrário: no sistema dela, a origem desceu
        a.node_mut(f).unwrap().placement.origin[2] = 5.;
        atualizar(&a, Some(&dir), &mut cache, Gatilho::Posicao);
        let (z0, _, _) = caixa(&dir);
        assert!((z0 - 12.).abs() < 1e-6, "{z0}");
    }

    #[test]
    fn a_copia_acompanha_a_forma_da_origem() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("forma");
        let (a, _, _) = conjunto(&dir, 0.);
        let mut cache = PartCache::default();
        std::fs::write(dir.join("mestre.fkp"), mestre("25").to_json()).unwrap();
        let rel = atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel.atualizados, 1, "{rel:?}");
        assert!((caixa(&dir).0 - 25.).abs() < 1e-6);
    }

    #[test]
    fn politica_decide_quando_atualiza() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("politica");
        let (mut a, m, f) = conjunto(&dir, 0.);
        let mut cache = PartCache::default();
        let ext = filha(&dir).external[0].id;

        a.definir_politica_link("filha.fkp", ext, Politica::Manual);
        a.node_mut(m).unwrap().placement.origin[2] = 7.;
        for g in [Gatilho::Posicao, Gatilho::Abrir] {
            let rel = atualizar(&a, Some(&dir), &mut cache, g);
            assert_eq!((rel.atualizados, rel.adiados), (0, 1), "{g:?}");
        }
        assert!((caixa(&dir).0 - 10.).abs() < 1e-6);

        a.definir_politica_link("filha.fkp", ext, Politica::AoAbrir);
        assert_eq!(links_da_peca(&a, Some(&dir), &mut cache, f)[0].politica, Politica::AoAbrir);
        assert_eq!(atualizar(&a, Some(&dir), &mut cache, Gatilho::Posicao).adiados, 1);
        assert_eq!(atualizar(&a, Some(&dir), &mut cache, Gatilho::Abrir).atualizados, 1);
        assert!((caixa(&dir).0 - 17.).abs() < 1e-6);

        // a política vai para o arquivo da montagem, e a padrão não ocupa espaço
        let b = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(b.politica_link("filha.fkp", ext), Politica::AoAbrir);
        a.definir_politica_link("filha.fkp", ext, Politica::Automatica);
        assert!(a.politicas_link.is_empty());
    }

    #[test]
    fn travar_destravar_quebrar_e_isolar() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("acoes");
        let (mut a, m, f) = conjunto(&dir, 0.);
        let mut cache = PartCache::default();
        let ext = filha(&dir).external[0].id;
        let estado = |a: &Assembly, cache: &mut PartCache| links_da_peca(a, Some(&dir), cache, f)[0].estado;

        // travado: a origem anda e a cópia fica
        aplicar(&a, Some(&dir), &mut cache, f, ext, AcaoLink::Travar).unwrap();
        assert_eq!(estado(&a, &mut cache), EstadoLink::Travado);
        a.node_mut(m).unwrap().placement.origin[2] = 7.;
        let rel = atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!((rel.atualizados, rel.travados), (0, 1), "{rel:?}");
        assert!((caixa(&dir).0 - 10.).abs() < 1e-6);

        // destravado: volta a seguir
        aplicar(&a, Some(&dir), &mut cache, f, ext, AcaoLink::Destravar).unwrap();
        assert_eq!(atualizar_um(&a, Some(&dir), &mut cache, f, ext).atualizados, 1);
        assert!((caixa(&dir).0 - 17.).abs() < 1e-6);

        // quebrado: a cópia fica, nada a reativa, e destravar é recusado
        aplicar(&a, Some(&dir), &mut cache, f, ext, AcaoLink::Quebrar).unwrap();
        assert_eq!(estado(&a, &mut cache), EstadoLink::Quebrado);
        a.node_mut(m).unwrap().placement.origin[2] = 0.;
        let rel = atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel, RelatorioLinks::default());
        assert_eq!(estado(&a, &mut cache), EstadoLink::Quebrado);
        assert!((caixa(&dir).0 - 17.).abs() < 1e-6);
        assert!(aplicar(&a, Some(&dir), &mut cache, f, ext, AcaoLink::Destravar).is_err());

        // isolado: a referência vira geometria própria e o link some
        aplicar(&a, Some(&dir), &mut cache, f, ext, AcaoLink::Isolar).unwrap();
        let d = filha(&dir);
        assert!(d.external.is_empty());
        assert!(d.references.iter().any(|r| matches!(r.kind, RefKind::Isolated { .. })));
        assert!((caixa(&dir).0 - 17.).abs() < 1e-6);
    }

    #[test]
    fn pendurado_quando_a_origem_some_e_volta_sozinho() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("pendurado");
        let (a, _, f) = conjunto(&dir, 0.);
        let mut cache = PartCache::default();

        // a publicação deixa de existir
        let mut d = mestre("10");
        d.references[0].published = false;
        std::fs::write(dir.join("mestre.fkp"), d.to_json()).unwrap();
        let rel = atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel.pendurados, 1, "{rel:?}");
        assert_eq!(links_da_peca(&a, Some(&dir), &mut cache, f)[0].estado, EstadoLink::Pendurado);
        assert!((caixa(&dir).0 - 10.).abs() < 1e-6, "a cópia antiga segura a peça");

        // reaparece: ao abrir, o link se recupera
        std::fs::write(dir.join("mestre.fkp"), mestre("10").to_json()).unwrap();
        atualizar(&a, Some(&dir), &mut cache, Gatilho::Abrir);
        assert_eq!(links_da_peca(&a, Some(&dir), &mut cache, f)[0].estado, EstadoLink::EmContexto);

        // o arquivo some
        std::fs::remove_file(dir.join("mestre.fkp")).unwrap();
        cache.forget(&dir.join("mestre.fkp"));
        assert_eq!(links_da_peca(&a, Some(&dir), &mut cache, f)[0].estado, EstadoLink::Pendurado);
    }

    #[test]
    fn fora_de_contexto_quando_a_origem_nao_esta_na_montagem() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("fora");
        let (mut a, m, f) = conjunto(&dir, 0.);
        let mut cache = PartCache::default();
        a.delete(m);
        assert_eq!(links_da_peca(&a, Some(&dir), &mut cache, f)[0].estado, EstadoLink::ForaDeContexto);
        // a forma continua viva; a posição fica a última conhecida
        std::fs::write(dir.join("mestre.fkp"), mestre("25").to_json()).unwrap();
        atualizar(&a, Some(&dir), &mut cache, Gatilho::Manual);
        assert!((caixa(&dir).0 - 25.).abs() < 1e-6);
    }

    #[test]
    fn corrente_de_links_atualiza_em_qualquer_ordem_da_arvore() {
        if sem_kernel() {
            return;
        }
        // C usa o plano de B, que usa o plano de A; C vem antes na árvore
        let dir = pasta("corrente");
        let a_doc = mestre("10");
        std::fs::write(dir.join("a.fkp"), a_doc.to_json()).unwrap();
        let geo = |d: &Document| fenkra_solidos::publications(d).remove(0).1;
        let mut b = Document::new("B");
        let pb = b.add_link("A", dir.join("a.fkp").to_str().unwrap(), "Base", geo(&a_doc));
        let r = b.references.iter_mut().find(|r| r.id == pb).unwrap();
        r.name = "Base".into();
        r.published = true;
        std::fs::write(dir.join("b.fkp"), b.to_json()).unwrap();
        let mut c = Document::new("C");
        let pc = c.add_link("B", dir.join("b.fkp").to_str().unwrap(), "Base", geo(&b));
        bloco_sobre(&mut c, pc);
        std::fs::write(dir.join("c.fkp"), c.to_json()).unwrap();

        let mut m = Assembly::new("Corrente");
        for n in ["c", "b", "a"] {
            m.add_component(n, NodeKind::Part { path: format!("{n}.fkp") }, Placement::default(), None);
        }
        std::fs::write(dir.join("a.fkp"), mestre("30").to_json()).unwrap();
        let mut cache = PartCache::default();
        let rel = atualizar(&m, Some(&dir), &mut cache, Gatilho::Manual);
        assert_eq!(rel.pecas.len(), 2, "{rel:?}");
        let d = Document::from_json(&std::fs::read_to_string(dir.join("c.fkp")).unwrap()).unwrap();
        assert!((fenkra_solidos::final_shape(&d).unwrap().bbox().unwrap().0[2] - 30.).abs() < 1e-6);
    }

    /// Peça com um bloco de 10 × 10 × `ee`, altura vinda do mestre por um valor publicado.
    fn peca_com_valor(dir: &Path, nome: &str, ee: f64) {
        let mut d = Document::new(nome);
        let alvo = d.add_link("Mestre", dir.join("mestre.fkp").to_str().unwrap(), "EE", LinkedGeo::Value(ee));
        let param = d.params.iter().find(|p| p.id == alvo).unwrap().name.clone();
        let mut n = 8_100_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        s.add_rect(&mut al, [-5., -5.], [5., 5.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: s }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(param), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        std::fs::write(dir.join(format!("{nome}.fkp")), d.to_json()).unwrap();
    }

    fn mestre_ee(dir: &Path, ee: f64) {
        let mut d = Document::new("Mestre");
        let id = d.alloc();
        d.params.push(fenkra_core::Parameter { published: true, ..fenkra_core::Parameter::novo(id, "EE", &ee.to_string(), fenkra_core::Unit::Comprimento) });
        std::fs::write(dir.join("mestre.fkp"), d.to_json()).unwrap();
    }

    /// Altura e volume da peça do arquivo.
    fn altura(arquivo: &Path) -> (f64, f64) {
        let d = Document::from_json(&std::fs::read_to_string(arquivo).unwrap()).unwrap();
        let r = fenkra_solidos::recompute(&d);
        assert!(r.status.values().all(|s| matches!(s, fenkra_solidos::Status::Ok)), "{:?}", r.status);
        let s = fenkra_solidos::final_shape(&d).expect("forma");
        let (lo, hi) = s.bbox().unwrap();
        (hi[2] - lo[2], s.mass().volume)
    }

    /// Topo com o mestre e um subconjunto, que tem uma peça e outro subconjunto com mais uma.
    fn topo_com_subconjuntos(dir: &Path) -> Assembly {
        mestre_ee(dir, 20.);
        peca_com_valor(dir, "funda", 20.);
        peca_com_valor(dir, "rasa", 20.);
        let mut fundo = Assembly::new("Fundo");
        fundo.add_component("Funda", NodeKind::Part { path: "funda.fkp".into() }, Placement::default(), None);
        std::fs::write(dir.join("fundo.fka"), fundo.to_json()).unwrap();
        let mut meio = Assembly::new("Meio");
        meio.add_component("Rasa", NodeKind::Part { path: "rasa.fkp".into() }, Placement::default(), None);
        meio.add_component("Fundo", NodeKind::SubAssembly { path: "fundo.fka".into() }, Placement::default(), None);
        std::fs::write(dir.join("meio.fka"), meio.to_json()).unwrap();
        let mut topo = Assembly::new("Topo");
        topo.add_component("Mestre", NodeKind::Part { path: "mestre.fkp".into() }, Placement::default(), None);
        topo.add_component("Meio", NodeKind::SubAssembly { path: "meio.fka".into() }, Placement::default(), None);
        topo
    }

    #[test]
    fn mudanca_do_mestre_desce_pelos_subconjuntos() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("descer");
        let topo = topo_com_subconjuntos(&dir);
        let (h, v) = altura(&dir.join("funda.fkp"));
        assert!((h - 20.).abs() < 1e-6 && (v - 2000.).abs() < 1e-6, "{h} {v}");

        mestre_ee(&dir, 35.);
        let rel = atualizar(&topo, Some(&dir), &mut PartCache::default(), Gatilho::Manual);
        assert_eq!((rel.pecas.len(), rel.atualizados), (2, 2), "{rel:?}");
        assert!(rel.ilegiveis.is_empty(), "{rel:?}");
        for f in ["funda.fkp", "rasa.fkp"] {
            let (h, v) = altura(&dir.join(f));
            assert!((h - 35.).abs() < 1e-6 && (v - 3500.).abs() < 1e-6, "{f}: {h} {v}");
        }
    }

    #[test]
    fn mover_um_componente_nao_le_os_subconjuntos() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("posicao_sub");
        let topo = topo_com_subconjuntos(&dir);
        // com o arquivo fora, qualquer leitura apareceria em `ilegiveis`
        std::fs::remove_file(dir.join("meio.fka")).unwrap();
        let rel = atualizar(&topo, Some(&dir), &mut PartCache::default(), Gatilho::Posicao);
        assert_eq!(rel, RelatorioLinks::default());
    }

    #[test]
    fn subconjunto_que_nao_abre_vira_aviso() {
        if sem_kernel() {
            return;
        }
        let dir = pasta("ilegivel");
        let topo = topo_com_subconjuntos(&dir);
        std::fs::remove_file(dir.join("meio.fka")).unwrap();
        let rel = atualizar(&topo, Some(&dir), &mut PartCache::default(), Gatilho::Manual);
        assert_eq!(rel.ilegiveis, vec![dir.join("meio.fka")], "{rel:?}");
    }

    #[test]
    fn subconjunto_em_ciclo_nao_prende_a_atualizacao() {
        let dir = pasta("ciclo_sub");
        let mut a = Assembly::new("A");
        a.add_component("B", NodeKind::SubAssembly { path: "b.fka".into() }, Placement::default(), None);
        std::fs::write(dir.join("a.fka"), a.to_json()).unwrap();
        let mut b = Assembly::new("B");
        b.add_component("A", NodeKind::SubAssembly { path: "a.fka".into() }, Placement::default(), None);
        std::fs::write(dir.join("b.fka"), b.to_json()).unwrap();
        let rel = atualizar(&a, Some(&dir), &mut PartCache::default(), Gatilho::Manual);
        assert_eq!(rel, RelatorioLinks::default());
    }

    #[test]
    fn vizinha_presa_a_peca_do_subconjunto_acompanha_a_forma_nova() {
        if sem_kernel() {
            return;
        }
        // tampa apoiada no topo da peça: a altura vem do mestre, e a tampa tem de subir com ela.
        // O ponto clicado segue mais perto do topo que das laterais, por isso 20 → 23
        let dir = pasta("vizinha");
        mestre_ee(&dir, 20.);
        peca_com_valor(&dir, "base", 20.);
        let mut tampa = Document::new("Tampa");
        let plano = tampa.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "0");
        bloco_sobre(&mut tampa, plano);
        std::fs::write(dir.join("tampa.fkp"), tampa.to_json()).unwrap();
        let mut sub = Assembly::new("Sub");
        let b = sub.add_component("Base", NodeKind::Part { path: "base.fkp".into() }, Placement::default(), None);
        sub.node_mut(b).unwrap().fixed = true;
        let t = sub.add_component("Tampa", NodeKind::Part { path: "tampa.fkp".into() }, Placement { origin: [0., 0., 20.], ..Default::default() }, None);
        let topo_base = MateRef { comp: b, geom: Geom::Plane { p: [0., 0., 20.], n: [0., 0., 1.] }, pick: Some([0., 0., 20.]), viva: None };
        let fundo_tampa = MateRef { comp: t, geom: Geom::Plane { p: [0., 0., 0.], n: [0., 0., -1.] }, pick: None, viva: None };
        sub.add_mate(MateKind::Contact, topo_base, fundo_tampa);
        assert!(crate::solve::solve(&mut sub).converged);
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        let mut topo = Assembly::new("Topo");
        topo.add_component("Mestre", NodeKind::Part { path: "mestre.fkp".into() }, Placement::default(), None);
        topo.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);

        mestre_ee(&dir, 23.);
        let rel = atualizar(&topo, Some(&dir), &mut PartCache::default(), Gatilho::Manual);
        assert_eq!(rel.subconjuntos, vec![dir.join("sub.fka")], "{rel:?}");
        let lido = Assembly::from_json(&std::fs::read_to_string(dir.join("sub.fka")).unwrap()).unwrap();
        let z = lido.node(t).unwrap().placement.origin[2];
        assert!((z - 23.).abs() < 1e-6, "a tampa ficou em {z}");
    }

    #[test]
    fn ciclo_e_detectado_em_qualquer_profundidade() {
        let dir = pasta("ciclo");
        let n = 12;
        // p0 usa p1, que usa p2, ... além do limite antigo de 8 níveis
        for i in 0..n {
            let mut d = Document::new(&format!("P{i}"));
            if i + 1 < n {
                d.add_link("x", &format!("p{}.fkp", i + 1), "Base", LinkedGeo::Point([0.; 3]));
            }
            std::fs::write(dir.join(format!("p{i}.fkp")), d.to_json()).unwrap();
        }
        let (p0, fim) = (dir.join("p0.fkp"), dir.join(format!("p{}.fkp", n - 1)));
        assert!(depende_de(&p0, &fim, None), "ligar a última à primeira fecharia um ciclo");
        assert!(!depende_de(&fim, &p0, None), "o contrário é seguro");

        // um ciclo que já existe no disco não prende a busca
        let mut d = Document::new("Ultima");
        d.add_link("x", "p0.fkp", "Base", LinkedGeo::Point([0.; 3]));
        std::fs::write(&fim, d.to_json()).unwrap();
        assert!(!depende_de(&p0, &dir.join("outra.fkp"), None));

        // montagem que inclui a si mesma por um subconjunto
        let mut sup = Assembly::new("Sup");
        sup.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);
        std::fs::write(dir.join("sup.fka"), sup.to_json()).unwrap();
        let mut sub = Assembly::new("Sub");
        sub.add_component("P", NodeKind::Part { path: "p1.fkp".into() }, Placement::default(), None);
        std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
        assert!(depende_de(&dir.join("sup.fka"), &dir.join("sub.fka"), None));
        assert!(depende_de(&dir.join("sup.fka"), &dir.join("p5.fkp"), None), "atravessa a peça");
    }
}
