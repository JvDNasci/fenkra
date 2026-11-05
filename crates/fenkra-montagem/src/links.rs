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

/// Atualiza os links da montagem.
pub fn atualizar(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, gatilho: Gatilho) -> RelatorioLinks {
    passadas(a, dir, cache, gatilho, None)
}

/// Atualiza só um link (botão da lista).
pub fn atualizar_um(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, comp: Id, ext: Id) -> RelatorioLinks {
    passadas(a, dir, cache, Gatilho::Manual, Some((comp, ext)))
}

/// Repete enquanto alguma peça mudar: A → B → C pede duas passadas se C vier antes de B na árvore.
/// Com o grafo acíclico, o número de peças é teto de sobra.
fn passadas(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, gatilho: Gatilho, so: Option<(Id, Ext)>) -> RelatorioLinks {
    let mut rel = RelatorioLinks::default();
    let alvos = pecas(a, dir);
    for k in 0..=alvos.len() {
        let mut mudou = false;
        for (comp, rotulo, full) in &alvos {
            if so.is_some_and(|(c, _)| c != *comp) {
                continue;
            }
            let Some(mut doc) = cache.get(full, false).ok().filter(|p| !p.doc.external.is_empty()).map(|p| p.doc.clone()) else { continue };
            let antes = (doc.external.clone(), doc.params.clone());
            for e in antes.0.iter().filter(|e| so.is_none_or(|(_, x)| x == e.id)) {
                atualizar_link(a, dir, cache, gatilho, *comp, rotulo, full, &mut doc, e, &mut rel, k == 0);
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
    rel
}

type Ext = Id;

#[allow(clippy::too_many_arguments)]
fn atualizar_link(a: &Assembly, dir: Option<&Path>, cache: &mut PartCache, gatilho: Gatilho, comp: Id, rotulo: &str, full: &Path, doc: &mut Document, e: &ExternalRef, rel: &mut RelatorioLinks, conta: bool) {
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
    let src = resolver(full, &e.path, dir);
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
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("4".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 }));
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
