// SPDX-License-Identifier: LGPL-2.1-or-later
//! Furo de montagem (B57): um eixo atravessa os componentes e cada peça do escopo recebe um furo
//! real, cilíndrico e dela, com link para a montagem.
//!
//! A montagem guarda o eixo e as regras ([`SerieFuros`]); a peça guarda o resultado (um comando
//! `Furo` comum, mais um `ExternalRef` cuja publicação começa com [`PREFIXO`]). Como o resultado é
//! um furo de verdade, `hole_table`, Fixadores, desenhos e restrições o enxergam sem caso especial.
//!
//! O eixo é a referência, não o mundo: ele nasce preso a um componente (ou a um ponto publicado,
//! ou a cotas da montagem). Mover outra peça do escopo não leva o eixo junto; [`sincronizar`]
//! regrava o furo dela no sistema local novo, e ele continua no mesmo eixo.
//!
//! [`sincronizar`] compara o que as séries pedem com o que as peças têm, e só isso. Desfazer,
//! suprimir, apagar a série e tirar uma peça do escopo não têm código próprio: o estado desejado
//! muda e a passada seguinte acerta os arquivos.
//!
//! Estados do link na peça:
//! - ativo: segue a série; a peça não edita esse furo;
//! - travado: o furo fica onde está até destravar;
//! - isolado (`Document::isolate_link`): o link some e o furo vira comum. A peça sai do escopo.
//!   Isso se reconhece por ainda existir o comando gravado sem o link, o que distingue "isolou"
//!   de "a série foi refeita pelo refazer", caso em que comando e link sumiram juntos.

use crate::links;
use crate::load::{full_path, PartCache};
use crate::view::shown;
use crate::*;
use fenkra_core::{AxisRef, Document, Extent, ExternalRef, FeatureKind, Hole, HoleKind, LinkState, PointRef};
use fenkra_kernel::{Bool, HoleShape, Shape};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Início da publicação de um link que não é cópia de geometria: é um recurso da montagem gravado
/// nesta peça. Links assim não têm o que reler na origem, e a origem é uma montagem.
pub const PREFIXO: &str = "montagem:furo:";

/// Um eixo no mundo: ponto onde o furo começa e direção para dentro do material.
pub type Eixo = (V3, V3);

/// De onde vem o eixo da série.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum EixoFuro {
    /// ponto e direção no sistema de um componente: o eixo acompanha esse componente
    Componente { comp: Id, p: V3, d: V3 },
    /// ponto publicado de uma peça; `p` é a última posição conhecida, usada para congelar o eixo
    /// quando o componente sai da montagem, nunca para resolver em silêncio
    Publicado { comp: Id, nome: String, d: V3, #[serde(default)] p: V3 },
    /// cotas da montagem (expressões em mm) e direção no sistema da montagem
    Cotas { x: String, y: String, z: String, d: V3 },
    /// pontos de um sketch da montagem; a direção é a normal do plano dele, para dentro
    Sketch { sketch: Id, #[serde(default)] inverter: bool },
    /// ponto publicado pela geometria da montagem
    PontoDaMontagem { nome: String, d: V3 },
}

impl EixoFuro {
    /// Componente de que o eixo depende.
    pub fn ancora(&self) -> Option<Id> {
        match self {
            EixoFuro::Componente { comp, .. } | EixoFuro::Publicado { comp, .. } => Some(*comp),
            _ => None,
        }
    }

    /// Troca a âncora por cotas com a posição de agora.
    pub fn congelar(&mut self, pl: &Placement) {
        let (p, d) = match self {
            EixoFuro::Componente { p, d, .. } | EixoFuro::Publicado { p, d, .. } => (pl.point(*p), pl.dir(*d)),
            _ => return,
        };
        let [x, y, z] = p.map(valor_cota);
        *self = EixoFuro::Cotas { x, y, z, d };
    }
}

/// O que muda de uma peça para outra na mesma série. Campo vazio segue a série.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FuroPeca {
    pub comp: Id,
    /// diâmetro nesta peça (folga na de cima, broca de rosca na de baixo)
    #[serde(default)]
    pub dia: String,
    /// rosca nesta peça (M8x1.25)
    #[serde(default)]
    pub rosca: String,
    /// profundidade a partir da boca nesta peça (furo cego)
    #[serde(default)]
    pub prof: String,
}

/// O que está gravado numa peça: guardado para reconhecer o furo que a pessoa isolou.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Gravado {
    pub comp: Id,
    pub ext: Id,
    pub feature: Id,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SerieFuros {
    pub eixo: EixoFuro,
    pub dia: String,
    /// profundidade da série, medida do início do eixo; vazio = atravessa todo o escopo
    #[serde(default)]
    pub depth: String,
    /// grava nas peças; `false` = só na montagem, com participantes (`targets`)
    #[serde(default = "sim")]
    pub gravar: bool,
    /// repetição do eixo, no sistema do eixo (o da âncora, ou o da montagem)
    #[serde(default)]
    pub padrao: Option<PatternKind>,
    #[serde(default)]
    pub pecas: Vec<FuroPeca>,
    #[serde(default)]
    pub gravados: Vec<Gravado>,
}

fn sim() -> bool {
    true
}

impl SerieFuros {
    /// Série gravada nas peças, passante, com o diâmetro dado.
    pub fn nova(eixo: EixoFuro, dia: &str) -> Self {
        SerieFuros { eixo, dia: dia.into(), depth: String::new(), gravar: true, padrao: None, pecas: vec![], gravados: vec![] }
    }

    pub fn peca(&self, comp: Id) -> Option<&FuroPeca> {
        self.pecas.iter().find(|p| p.comp == comp)
    }

    /// Ajuste da peça, criando a linha se ela ainda segue a série.
    pub fn peca_mut(&mut self, comp: Id) -> &mut FuroPeca {
        if let Some(i) = self.pecas.iter().position(|p| p.comp == comp) {
            return &mut self.pecas[i];
        }
        self.pecas.push(FuroPeca { comp, dia: String::new(), rosca: String::new(), prof: String::new() });
        self.pecas.last_mut().unwrap()
    }
}

/// O link é um recurso da montagem gravado na peça?
pub fn e_recurso(e: &ExternalRef) -> bool {
    e.publication.starts_with(PREFIXO)
}

/// O comando da peça é um furo que só a montagem edita? Travado também: destravar devolve a edição
/// à montagem, e mexer aqui seria perdido.
pub fn editado_pela_montagem(doc: &Document, feature: Id) -> bool {
    doc.external.iter().any(|e| e_recurso(e) && e.target == Some(feature) && !e.path.is_empty())
}

fn chave(feat: Id, comp: Id, k: usize) -> String {
    format!("{PREFIXO}{}:{}:{k}", feat.0, comp.0)
}

/// Linha da verificação: a peça do escopo recebeu o furo?
#[derive(Clone, Debug, PartialEq)]
pub struct Verificacao {
    pub comp: Id,
    pub nome: String,
    /// furos achados nesta peça
    pub furos: usize,
    /// eixos esperados nesta peça
    pub esperados: usize,
    /// por que faltou
    pub motivo: Option<String>,
}

impl Verificacao {
    pub fn ok(&self) -> bool {
        self.motivo.is_none()
    }
}

/// O que uma passada fez.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioFuros {
    /// peças regravadas
    pub pecas: Vec<PathBuf>,
    pub gravados: usize,
    pub movidos: usize,
    pub removidos: usize,
    /// (série, componente) que saiu do escopo porque o furo foi isolado na peça
    pub isolados: Vec<(Id, Id)>,
    /// séries cujo eixo não resolveu: os furos ficam como estavam
    pub erros: Vec<String>,
    /// peças do escopo que não receberam o furo
    pub faltas: Vec<String>,
    pub avisos: Vec<String>,
}

fn num(e: &str, vars: &HashMap<String, f64>) -> Option<f64> {
    (!e.trim().is_empty()).then(|| expr::eval_str(e, vars).ok()).flatten()
}

fn ponto_publicado(a: &Assembly, comp: Id, nome: &str, dir: Option<&Path>, cache: &mut PartCache) -> Option<V3> {
    let NodeKind::Part { path } = &a.node(comp)?.kind else { return None };
    let doc = &cache.get(&full_path(dir, path), false).ok()?.doc;
    fenkra_solidos::publications(doc).into_iter().find_map(|(n, g)| match g {
        fenkra_core::LinkedGeo::Point(p) if n == nome => Some(p),
        _ => None,
    })
}

/// Eixos da série no mundo. Referência que não resolve é erro: o eixo nunca cai em outro lugar.
pub fn eixos(a: &Assembly, s: &SerieFuros, dir: Option<&Path>, cache: &mut PartCache) -> Result<Vec<Eixo>, String> {
    let vars = a.vars();
    let no = |c: &Id| a.node(*c).map(|n| n.placement).ok_or_else(|| format!("o componente #{} saiu da montagem", c.0));
    let (mut base, sistema): (Vec<Eixo>, Placement) = match &s.eixo {
        EixoFuro::Componente { comp, p, d } => {
            let pl = no(comp)?;
            (vec![(pl.point(*p), unit(pl.dir(*d)))], pl)
        }
        EixoFuro::Publicado { comp, nome, d, .. } => {
            let pl = no(comp)?;
            let p = ponto_publicado(a, *comp, nome, dir, cache).ok_or_else(|| format!("o ponto publicado {nome} sumiu de {}", a.name_of(*comp)))?;
            (vec![(pl.point(p), unit(pl.dir(*d)))], pl)
        }
        EixoFuro::Cotas { x, y, z, d } => {
            let v = |e: &String| expr::eval_str(e, &vars).map_err(|_| format!("cota inválida: {e}"));
            (vec![([v(x)?, v(y)?, v(z)?], unit(*d))], Placement::default())
        }
        EixoFuro::Sketch { sketch, inverter } => {
            let (pontos, n) = a.pontos_de_sketch(*sketch).ok_or_else(|| format!("o sketch da montagem #{} sumiu", sketch.0))?;
            if pontos.is_empty() {
                return Err("o sketch da montagem não tem pontos".into());
            }
            let d = if *inverter { n } else { n.map(|x| -x) };
            (pontos.into_iter().map(|p| (p, d)).collect(), Placement::default())
        }
        EixoFuro::PontoDaMontagem { nome, d } => match a.publicacao_da_montagem(nome) {
            Some(fenkra_core::LinkedGeo::Point(p)) => (vec![(p, unit(*d))], Placement::default()),
            _ => return Err(format!("o ponto publicado {nome} sumiu da montagem")),
        },
    };
    let Some(padrao) = &s.padrao else { return Ok(base) };
    let n = |e: &String| expr::eval_str(e, &vars).map_err(|_| format!("valor inválido no padrão: {e}"));
    let originais = base.clone();
    match padrao {
        PatternKind::Linear { dir: v, count, spacing } => {
            let (q, passo) = (n(count)?.round().max(1.) as usize, n(spacing)?);
            let v = unit(sistema.dir(*v));
            for i in 1..q {
                base.extend(originais.iter().map(|(p, d)| ([0, 1, 2].map(|k| p[k] + v[k] * passo * i as f64), *d)));
            }
        }
        PatternKind::Circular { origin, axis, count, angle } => {
            let (q, total) = (n(count)?.round().max(1.) as usize, n(angle)?);
            let passo = if (total - 360.).abs() < 1e-9 { total / q as f64 } else { total / (q.max(2) - 1) as f64 };
            let (o, w) = (sistema.point(*origin), unit(sistema.dir(*axis)));
            for i in 1..q {
                let g = w.map(|x| x * (passo * i as f64).to_radians());
                base.extend(originais.iter().map(|(p, d)| {
                    let r = rotvec(sub(*p, o), g);
                    ([0, 1, 2].map(|k| r[k] + o[k]), rotvec(*d, g))
                }));
            }
        }
    }
    Ok(base)
}

/// Ferramenta exata do furo: cilindro de verdade, para o escopo e para o modo só na montagem.
pub(crate) fn cilindro(p: V3, d: V3, dia: f64, len: f64) -> Option<Shape> {
    // um pouco antes da boca, para cortar limpo a face onde o eixo começa
    let o = [0, 1, 2].map(|i| p[i] - d[i] * 0.05);
    Shape::hole_tool_ext(o, d, dia, len + 0.05, HoleShape::Simple, 0., 0.).ok()
}

/// Comprimento que certamente passa pela forma, a partir de `p` na direção `d`.
pub fn alcance(sh: &Shape, p: V3, d: V3) -> Option<f64> {
    let (lo, hi) = sh.bbox()?;
    let longe = (0..8)
        .map(|c| dot(sub([if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }], p), d))
        .fold(f64::MIN, f64::max);
    (longe > 1e-9).then_some(longe + 1.)
}

/// Peças que os eixos cruzam, na ordem da árvore. Vale volume em comum, e não o toque na linha de
/// centro: a peça que o cilindro pega de raspão entra, e a que já tem furo igual ou maior no
/// mesmo eixo não entra (não se fura o vazio).
pub fn escopo_pelo_eixo(a: &Assembly, s: &SerieFuros, dir: Option<&Path>, cache: &mut PartCache) -> Result<Vec<Id>, String> {
    let vars = a.vars();
    let dia = num(&s.dia, &vars).filter(|v| *v > 0.).ok_or_else(|| format!("diâmetro inválido: {}", s.dia))?;
    let prof = num(&s.depth, &vars).filter(|v| *v > 0.);
    let eixos = eixos(a, s, dir, cache)?;
    let mut out = vec![];
    let ids: Vec<Id> = a.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. }) && n.mode == LoadMode::Editavel && shown(a, n.id)).map(|n| n.id).collect();
    for id in ids {
        let Some(sh) = crate::view::raw_shape(a, id, dir, cache) else { continue };
        let cruza = eixos.iter().any(|(p, d)| {
            let Some(len) = alcance(&sh, *p, *d) else { return false };
            let Some(t) = cilindro(*p, *d, dia, prof.map_or(len, |v| v.min(len))) else { return false };
            sh.boolean(&t, Bool::Common).is_ok_and(|c| c.mass().volume > 1e-6)
        });
        if cruza {
            out.push(id);
        }
    }
    Ok(out)
}

/// Furo pedido para um arquivo de peça.
struct Pedido {
    serie: Id,
    comp: Id,
    chave: String,
    nome: String,
    furo: Hole,
}

fn furo(p: V3, d: V3, dia: f64, rosca: &str, depth: Extent) -> Hole {
    Hole {
        sketch: Id(0),
        kind: HoleKind::Simples,
        diameter: valor_cota(dia),
        depth,
        thread: (!rosca.trim().is_empty()).then(|| rosca.trim().to_string()),
        thread_modeled: false,
        standard: None,
        points: vec![PointRef::Fixed(p)],
        dir: Some(AxisRef::Fixed { origin: p, dir: d }),
        reverse: false,
        tip: None,
        taper: "0".into(),
        left_hand: false,
    }
}

/// Parâmetro da reta onde ela entra na forma, de `p` para a frente. `None`: não cruza.
fn entrada(sh: &Shape, p: V3, d: V3) -> Option<f64> {
    if let Ok((q, n)) = sh.surface_near(p) {
        // ponto dentro do material, ou sobre a face: a boca é ali mesmo
        if dot(sub(p, q), n) < 1e-6 {
            return Some(0.);
        }
    }
    match sh.ray_hit(p, d) {
        Ok(Some((_, t))) if t > -1e-9 => Some(t),
        _ => None,
    }
}

/// Forma da peça sem os furos de série: a referência para saber o que o eixo cruza. Com eles, o
/// eixo sempre cairia no vazio do próprio furo.
fn forma_base(doc: &Document) -> Option<Shape> {
    let mut d = doc.clone();
    let meus: HashSet<Id> = d.external.iter().filter(|e| e_recurso(e)).filter_map(|e| e.target).collect();
    d.features.retain(|f| !meus.contains(&f.id));
    d.rollback = d.rollback.min(d.features.len());
    d.external.retain(|e| !e_recurso(e));
    fenkra_solidos::final_shape(&d)
}

/// Peças da montagem por arquivo.
fn arquivos(a: &Assembly, dir: Option<&Path>) -> BTreeMap<PathBuf, Vec<Id>> {
    let mut m: BTreeMap<PathBuf, Vec<Id>> = BTreeMap::new();
    for n in &a.nodes {
        if let NodeKind::Part { path } = &n.kind {
            m.entry(full_path(dir, path)).or_default().push(n.id);
        }
    }
    m
}

/// Tira do escopo as peças que isolaram o furo: o comando continua lá, sem link.
fn recolher_isolados(a: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache, rel: &mut RelatorioFuros) {
    let mut fora: Vec<(Id, Id)> = vec![];
    for f in &a.features {
        let AsmFeatKind::Serie(s) = &f.kind else { continue };
        for g in &s.gravados {
            let Some(NodeKind::Part { path }) = a.node(g.comp).map(|n| &n.kind) else { continue };
            let Ok(p) = cache.get(&full_path(dir, path), false) else { continue };
            if p.doc.feature(g.feature).is_some() && !p.doc.external.iter().any(|e| e.id == g.ext) {
                fora.push((f.id, g.comp));
            }
        }
    }
    for (fid, comp) in &fora {
        if let Some(f) = a.features.iter_mut().find(|f| f.id == *fid) {
            f.targets.retain(|t| t != comp);
            if let AsmFeatKind::Serie(s) = &mut f.kind {
                s.pecas.retain(|p| p.comp != *comp);
                s.gravados.retain(|g| g.comp != *comp);
            }
        }
        if !rel.isolados.contains(&(*fid, *comp)) {
            rel.isolados.push((*fid, *comp));
        }
    }
}

/// Guarda onde cada ponto publicado está agora: é o que sobra para congelar o eixo se o componente
/// sair da montagem.
fn lembrar_publicados(a: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache) {
    let achados: Vec<(usize, V3)> = a
        .features
        .iter()
        .enumerate()
        .filter_map(|(i, f)| match &f.kind {
            AsmFeatKind::Serie(SerieFuros { eixo: EixoFuro::Publicado { comp, nome, .. }, .. }) => ponto_publicado(a, *comp, nome, dir, cache).map(|p| (i, p)),
            _ => None,
        })
        .collect();
    for (i, q) in achados {
        if let AsmFeatKind::Serie(SerieFuros { eixo: EixoFuro::Publicado { p, .. }, .. }) = &mut a.features[i].kind {
            *p = q;
        }
    }
}

/// Faz as peças ficarem como as séries pedem. `arquivo` é o da montagem: sem ele o link da peça
/// não teria para onde apontar. `aberta` é a peça em edição: o documento dela está na memória de
/// quem a abriu, e gravar por baixo perderia a edição.
pub fn sincronizar(a: &mut Assembly, dir: Option<&Path>, arquivo: Option<&Path>, cache: &mut PartCache, aberta: Option<&Path>) -> RelatorioFuros {
    let mut rel = RelatorioFuros::default();
    recolher_isolados(a, dir, cache, &mut rel);
    lembrar_publicados(a, dir, cache);
    let vars = a.vars();

    // 1) o que cada série pede
    let mut intocadas: HashSet<Id> = HashSet::new();
    let mut por_serie: Vec<(Id, String, SerieFuros, Vec<Id>, Vec<Eixo>)> = vec![];
    for f in &a.features {
        let AsmFeatKind::Serie(s) = &f.kind else { continue };
        if f.suppressed || !s.gravar {
            continue;
        }
        match eixos(a, s, dir, cache) {
            Ok(e) => por_serie.push((f.id, f.name.clone(), s.clone(), f.targets.clone(), e)),
            Err(e) => {
                intocadas.insert(f.id);
                rel.erros.push(format!("{}: {e}", f.name));
            }
        }
    }
    if arquivo.is_none() {
        if !por_serie.is_empty() {
            rel.erros.push("Salve a montagem para gravar os furos nas peças".into());
        }
        return rel;
    }

    // 2) arquivo por arquivo
    let mut gravados: Vec<(Id, Gravado)> = vec![];
    for (full, comps) in arquivos(a, dir) {
        if aberta.is_some_and(|x| x == full) {
            continue;
        }
        let Ok(doc0) = cache.get(&full, false).map(|p| p.doc.clone()) else { continue };
        let tem_recurso = doc0.external.iter().any(|e| e_recurso(e) && e.source == a.name);
        let pede = por_serie.iter().any(|x| x.3.iter().any(|t| comps.contains(t)));
        if !tem_recurso && !pede {
            continue;
        }
        let mut base: Option<Option<Shape>> = None;
        let mut pedidos: Vec<Pedido> = vec![];
        for (fid, nome, s, alvos, eixos) in &por_serie {
            for comp in alvos.iter().filter(|t| comps.contains(t)) {
                let Some(no) = a.node(*comp) else { continue };
                let pl = no.placement;
                let ajuste = s.peca(*comp);
                let Some(dia) = ajuste.and_then(|p| num(&p.dia, &vars)).or_else(|| num(&s.dia, &vars)).filter(|v| *v > 0.) else {
                    rel.faltas.push(format!("{}: diâmetro inválido em {nome}", no.name));
                    continue;
                };
                let rosca = ajuste.map(|p| p.rosca.as_str()).unwrap_or("");
                let cego = ajuste.and_then(|p| num(&p.prof, &vars)).filter(|v| *v > 0.);
                let serie_prof = num(&s.depth, &vars).filter(|v| *v > 0.);
                let forma = base.get_or_insert_with(|| forma_base(&doc0));
                let mut cruzou = 0;
                for (k, (p, d)) in eixos.iter().enumerate() {
                    let (pl_p, pl_d) = (pl.to_local(*p), unit(pl.dir_to_local(*d)));
                    let Some(t) = forma.as_ref().and_then(|sh| entrada(sh, pl_p, pl_d)) else { continue };
                    let depth = match (cego, serie_prof) {
                        (Some(v), _) => Extent::Blind(valor_cota(v)),
                        (None, Some(v)) if v <= t => continue,
                        (None, Some(v)) => Extent::Blind(valor_cota(v - t)),
                        (None, None) => Extent::ThroughAll,
                    };
                    cruzou += 1;
                    // duas instâncias do arquivo no mesmo eixo local pedem o mesmo furo
                    if pedidos.iter().any(|x| matches!(x.furo.points[0], PointRef::Fixed(q) if norm(sub(q, pl_p)) < 1e-6)) {
                        continue;
                    }
                    pedidos.push(Pedido { serie: *fid, comp: *comp, chave: chave(*fid, *comp, k), nome: nome.clone(), furo: furo(pl_p, pl_d, dia, rosca, depth) });
                }
                if cruzou == 0 {
                    rel.faltas.push(format!("{}: o eixo de {nome} não cruza a peça", no.name));
                }
            }
        }
        if !pedidos.is_empty() && comps.len() > 1 {
            let dentro = pedidos.iter().map(|p| p.comp).collect::<HashSet<_>>().len();
            if dentro < comps.len() {
                let n = a.node(comps[0]).map(|n| n.name.clone()).unwrap_or_default();
                rel.avisos.push(format!("{n}: o furo aparece nas {} instâncias do arquivo", comps.len()));
            }
        }

        let mut doc = doc0.clone();
        let mut mudou = false;
        for ped in &pedidos {
            let achado = doc.external.iter().find(|e| e.publication == ped.chave && e.source == a.name).cloned();
            match achado {
                Some(e) if e.state != LinkState::Ativo => {
                    if let Some(t) = e.target {
                        gravados.push((ped.serie, Gravado { comp: ped.comp, ext: e.id, feature: t }));
                    }
                }
                Some(e) if e.target.is_some_and(|t| doc.feature(t).is_some()) => {
                    let t = e.target.unwrap();
                    gravados.push((ped.serie, Gravado { comp: ped.comp, ext: e.id, feature: t }));
                    if let Some(f) = doc.features.iter_mut().find(|f| f.id == t) {
                        if !matches!(&f.kind, FeatureKind::Hole(h) if *h == ped.furo) {
                            f.kind = FeatureKind::Hole(ped.furo.clone());
                            rel.movidos += 1;
                            mudou = true;
                        }
                    }
                }
                outro => {
                    // apagado na peça: o link manda, o furo volta
                    let feature = doc.add_feature(FeatureKind::Hole(ped.furo.clone()));
                    let nome = doc.unique_name(&ped.nome);
                    if let Some(f) = doc.features.iter_mut().find(|f| f.id == feature) {
                        f.name = nome;
                    }
                    let ext = match outro {
                        Some(e) => {
                            if let Some(x) = doc.external.iter_mut().find(|x| x.id == e.id) {
                                x.target = Some(feature);
                            }
                            e.id
                        }
                        None => {
                            let ext = doc.alloc();
                            let alvo = caminho_de(&full, arquivo.unwrap());
                            doc.external.push(ExternalRef { id: ext, name: format!("{}.{}", a.name, ped.nome), source: a.name.clone(), publication: ped.chave.clone(), state: LinkState::Ativo, path: alvo, uid: String::new(), snapshot: None, target: Some(feature), xform: None });
                            ext
                        }
                    };
                    gravados.push((ped.serie, Gravado { comp: ped.comp, ext, feature }));
                    rel.gravados += 1;
                    mudou = true;
                }
            }
        }

        // 3) o que está na peça e ninguém mais pede. Série sem eixo resolvido fica como está.
        let quer: HashSet<&str> = pedidos.iter().map(|p| p.chave.as_str()).collect();
        let sobra: Vec<ExternalRef> = doc
            .external
            .iter()
            .filter(|e| e_recurso(e) && e.source == a.name && e.state == LinkState::Ativo && !quer.contains(e.publication.as_str()))
            .filter(|e| !intocadas.iter().any(|i| e.publication.starts_with(&format!("{PREFIXO}{}:", i.0))))
            .cloned()
            .collect();
        for e in sobra {
            doc.external.retain(|x| x.id != e.id);
            if let Some(t) = e.target {
                doc.features.retain(|f| f.id != t);
                doc.rollback = doc.rollback.min(doc.features.len());
            }
            rel.removidos += 1;
            mudou = true;
        }

        if mudou {
            match links::gravar(&full, &doc, cache) {
                Ok(()) => rel.pecas.push(full.clone()),
                Err(e) => {
                    let nome = a.node(comps[0]).map(|n| n.name.clone()).unwrap_or_default();
                    rel.faltas.push(format!("{nome}: {e}"));
                }
            }
        }
    }

    // 4) memória do que foi gravado, para reconhecer um isolamento depois
    for f in &mut a.features {
        if let AsmFeatKind::Serie(s) = &mut f.kind {
            if s.gravar && !intocadas.contains(&f.id) {
                s.gravados = gravados.iter().filter(|(i, _)| *i == f.id).map(|(_, g)| *g).collect();
            }
        }
    }
    rel
}

/// Passa para `sub` os furos que a montagem `de` gravou nas peças dele. Depois de extrair um
/// subconjunto as peças saem da árvore de cima, que deixa de ver esses arquivos: sem a troca, o link
/// ficaria apontando para quem não cuida mais dele. As séries e os nós mantêm os ids ao extrair,
/// então as chaves continuam valendo.
pub fn adotar(sub: &Assembly, de: &str, dir: Option<&Path>, arquivo: &Path, cache: &mut PartCache) {
    let chaves: Vec<String> = sub.features.iter().filter(|f| matches!(f.kind, AsmFeatKind::Serie(_))).flat_map(|f| f.targets.iter().map(|c| format!("{PREFIXO}{}:{}:", f.id.0, c.0))).collect();
    for full in arquivos(sub, dir).into_keys() {
        let Ok(mut doc) = cache.get(&full, false).map(|p| p.doc.clone()) else { continue };
        let mut mudou = false;
        for e in doc.external.iter_mut().filter(|e| e_recurso(e) && e.source == de && chaves.iter().any(|c| e.publication.starts_with(c.as_str()))) {
            e.source = sub.name.clone();
            e.path = caminho_de(&full, arquivo);
            mudou = true;
        }
        if mudou {
            let _ = links::gravar(&full, &doc, cache);
        }
    }
}

/// Caminho de `alvo` como um link da peça o guarda: relativo à pasta dela, quando dá.
pub(crate) fn caminho_de(peca: &Path, alvo: &Path) -> String {
    match peca.parent().and_then(|d| alvo.strip_prefix(d).ok()) {
        Some(r) => r.to_string_lossy().replace('\\', "/"),
        None => alvo.to_string_lossy().to_string(),
    }
}

/// Toda peça do escopo recebeu o furo? Olha o resultado na peça: a tabela de furos e o estado do
/// comando depois de recalculada, e não o pedido.
pub fn verificar(a: &Assembly, feat: Id, dir: Option<&Path>, cache: &mut PartCache) -> Vec<Verificacao> {
    let Some(f) = a.features.iter().find(|f| f.id == feat) else { return vec![] };
    let AsmFeatKind::Serie(s) = &f.kind else { return vec![] };
    let eixos = match eixos(a, s, dir, cache) {
        Ok(e) => e,
        Err(e) => return f.targets.iter().map(|c| Verificacao { comp: *c, nome: a.name_of(*c), furos: 0, esperados: 0, motivo: Some(e.clone()) }).collect(),
    };
    let mut out = vec![];
    for c in &f.targets {
        let nome = a.name_of(*c);
        let Some(n) = a.node(*c) else { continue };
        let NodeKind::Part { path } = &n.kind else {
            out.push(Verificacao { comp: *c, nome, furos: 0, esperados: 0, motivo: Some("subconjunto: abra a montagem dele".into()) });
            continue;
        };
        let lp = match cache.get(&full_path(dir, path), false) {
            Ok(l) => l,
            Err(e) => {
                out.push(Verificacao { comp: *c, nome, furos: 0, esperados: 0, motivo: Some(e) });
                continue;
            }
        };
        let meus: Vec<&Gravado> = s.gravados.iter().filter(|g| g.comp == *c).collect();
        let erro = meus.iter().find_map(|g| match lp.result.status.get(&g.feature) {
            Some(fenkra_solidos::Status::Error(m)) => Some(m.clone()),
            _ => None,
        });
        let linhas = fenkra_solidos::hole_table(&lp.doc);
        let furos = eixos
            .iter()
            .filter(|(p, d)| {
                let (lp_, ld) = (n.placement.to_local(*p), n.placement.dir_to_local(*d));
                linhas.iter().any(|r| {
                    let v = sub(r.pos, lp_);
                    // os pontos de sketch vêm da vista, em precisão simples
                    norm(cross(v, unit(ld))) < 1e-3 && dot(unit(r.dir), unit(ld)).abs() > 1. - 1e-6
                })
            })
            .count();
        let motivo = erro.or_else(|| (furos == 0).then(|| "a peça não tem furo neste eixo".to_string()));
        out.push(Verificacao { comp: *c, nome, furos, esperados: meus.len().max(1), motivo });
    }
    out
}

impl Assembly {
    /// Série nova: o escopo é o que o eixo cruza, mais `alvos`. Sem peça nenhuma, não cria.
    pub fn add_serie_furos(&mut self, s: SerieFuros, alvos: Vec<Id>, dir: Option<&Path>, cache: &mut PartCache) -> Result<Id, String> {
        let mut alvos = alvos;
        for c in escopo_pelo_eixo(self, &s, dir, cache)? {
            if !alvos.contains(&c) {
                alvos.push(c);
            }
        }
        if alvos.is_empty() {
            return Err("o eixo não cruza nenhuma peça".into());
        }
        Ok(self.add_feature(AsmFeatKind::Serie(s), alvos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::{self, AcaoLink};
    use fenkra_core::{BoolOp, Extrude, OriginPlane, PlaneRef, Sketch, SketchData};

    /// Placa de 60 × 60 × 10, de z = 0 a z = 10, centrada na origem.
    fn placa() -> Document {
        let mut d = Document::new("Placa");
        let mut n = 7_000_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut base = SketchData::default();
        base.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind("10".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 }));
        d
    }

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra_furos_{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    struct Caso {
        dir: PathBuf,
        arq: PathBuf,
        a: Assembly,
        cache: PartCache,
        cima: Id,
        baixo: Id,
    }

    impl Caso {
        /// Placa de baixo (z 0..10, fixa) e de cima (z 10..20), arquivos separados, e uma série Ø6
        /// presa à de cima, do topo para baixo, a 5 mm do centro.
        fn novo(nome: &str) -> (Caso, Id) {
            let dir = pasta(nome);
            std::fs::write(dir.join("baixo.fkp"), placa().to_json()).unwrap();
            std::fs::write(dir.join("cima.fkp"), placa().to_json()).unwrap();
            let mut a = Assembly::new("Conjunto");
            let baixo = a.add_component("Baixo", NodeKind::Part { path: "baixo.fkp".into() }, Placement::default(), None);
            let cima = a.add_component("Cima", NodeKind::Part { path: "cima.fkp".into() }, Placement { origin: [0., 0., 10.], ..Default::default() }, None);
            let mut cache = PartCache::default();
            let s = SerieFuros::nova(EixoFuro::Componente { comp: cima, p: [5., 0., 10.], d: [0., 0., -1.] }, "6");
            let f = a.add_serie_furos(s, vec![], Some(&dir), &mut cache).unwrap();
            let arq = dir.join("conjunto.fka");
            std::fs::write(&arq, a.to_json()).unwrap();
            (Caso { dir, arq, a, cache, cima, baixo }, f)
        }
        fn sync(&mut self) -> RelatorioFuros {
            sincronizar(&mut self.a, Some(&self.dir), Some(&self.arq), &mut self.cache, None)
        }
        fn doc(&mut self, comp: Id) -> Document {
            let NodeKind::Part { path } = self.a.node(comp).unwrap().kind.clone() else { panic!() };
            self.cache.get(&full_path(Some(&self.dir), &path), false).unwrap().doc.clone()
        }
        fn volume(&mut self, comp: Id) -> f64 {
            let d = self.doc(comp);
            let r = fenkra_solidos::recompute(&d);
            assert!(r.status.values().all(|x| !matches!(x, fenkra_solidos::Status::Error(_))), "{:?}", r.status);
            r.bodies.iter().map(|b| b.mass.volume).sum()
        }
        fn serie(&mut self, f: Id) -> &mut SerieFuros {
            match &mut self.a.features.iter_mut().find(|x| x.id == f).unwrap().kind {
                AsmFeatKind::Serie(s) => s,
                _ => panic!(),
            }
        }
    }

    const CHEIA: f64 = 60. * 60. * 10.;
    fn cilindro_vol(dia: f64, h: f64) -> f64 {
        std::f64::consts::PI * dia * dia / 4. * h
    }

    #[test]
    fn furo_real_e_cilindrico_em_cada_peca_do_escopo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("real");
        assert_eq!(c.a.features[0].targets.len(), 2, "o eixo cruza as duas placas");
        let r = c.sync();
        assert_eq!((r.gravados, r.pecas.len()), (2, 2), "{r:?}");
        assert!(r.faltas.is_empty() && r.erros.is_empty(), "{r:?}");
        for comp in [c.cima, c.baixo] {
            // volume do cilindro exato: um prisma de 64 lados ficaria 0,16% abaixo
            let v = c.volume(comp);
            assert!((CHEIA - v - cilindro_vol(6., 10.)).abs() < 1e-3, "{v}");
            let d = c.doc(comp);
            let t = fenkra_solidos::hole_table(&d);
            assert_eq!(t.len(), 1, "a tabela de furos enxerga o furo");
            assert!(t[0].through && (t[0].diameter - 6.).abs() < 1e-9);
            // a parede é cilíndrica de verdade: vira eixo numa restrição
            let sh = fenkra_solidos::final_shape(&d).unwrap();
            assert!(matches!(crate::load::geom_at(&sh, [8., 0., 5.]).unwrap(), Geom::Axis { .. }));
            assert!(editado_pela_montagem(&d, d.external[0].target.unwrap()));
        }
        let v = verificar(&c.a, f, Some(&c.dir), &mut c.cache);
        assert!(v.len() == 2 && v.iter().all(Verificacao::ok), "{v:?}");
        // de novo, sem mudança: não regrava nada
        let r = c.sync();
        assert!(r.pecas.is_empty(), "{r:?}");
    }

    #[test]
    fn mover_uma_peca_mantem_o_furo_no_eixo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("mover");
        c.sync();
        let baixo = c.baixo;
        c.a.node_mut(baixo).unwrap().placement.origin = [2., 0., 0.];
        let r = c.sync();
        assert_eq!((r.movidos, r.pecas.len()), (1, 1), "só a peça movida muda: {r:?}");
        let t = fenkra_solidos::hole_table(&c.doc(baixo));
        assert!(norm(sub(c.a.node(baixo).unwrap().placement.point(t[0].pos), [5., 0., 10.])) < 1e-6, "no eixo: {:?}", t[0].pos);
        // a âncora leva o eixo: o furo da outra peça a acompanha
        let cima = c.cima;
        c.a.node_mut(cima).unwrap().placement.origin = [0., 7., 10.];
        c.sync();
        let t = fenkra_solidos::hole_table(&c.doc(baixo));
        assert!(norm(sub(c.a.node(baixo).unwrap().placement.point(t[0].pos), [5., 7., 10.])) < 1e-6, "{:?}", t[0].pos);
        assert!(verificar(&c.a, f, Some(&c.dir), &mut c.cache).iter().all(Verificacao::ok));
    }

    #[test]
    fn tipo_por_peca_folga_em_cima_e_roscado_cego_embaixo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("tipo");
        let (cima, baixo) = (c.cima, c.baixo);
        c.serie(f).peca_mut(cima).dia = "6.6".into();
        let p = c.serie(f).peca_mut(baixo);
        p.dia = "5".into();
        p.rosca = "M6".into();
        p.prof = "8".into();
        c.sync();
        let (t1, t2) = (fenkra_solidos::hole_table(&c.doc(cima)), fenkra_solidos::hole_table(&c.doc(baixo)));
        assert!(t1[0].through && (t1[0].diameter - 6.6).abs() < 1e-9, "{t1:?}");
        assert!(!t2[0].through && t2[0].thread == "M6" && (t2[0].depth - 8.).abs() < 1e-9, "{t2:?}");
        let v = c.volume(baixo);
        assert!((CHEIA - v - cilindro_vol(5., 8.)).abs() < 1e-3, "{v}");
    }

    #[test]
    fn fixadores_preenchem_a_pilha_gravada_pela_serie() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("fix");
        let baixo = c.baixo;
        c.serie(f).dia = "6.6".into();
        let p = c.serie(f).peca_mut(baixo);
        p.dia = "5".into();
        p.rosca = "M6".into();
        c.sync();
        let dir = c.dir.clone();
        let rel = crate::preencher_furos(&mut c.a, Some(&dir), &mut c.cache, &Default::default()).unwrap();
        assert_eq!(rel.fixadores, 1, "{rel:?}");
    }

    #[test]
    fn desfazer_e_refazer_acertam_as_pecas_pelo_estado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("desfazer");
        c.sync();
        let com = c.a.clone();
        let cima = c.cima;
        c.a.features.clear();
        let r = c.sync();
        assert_eq!(r.removidos, 2, "{r:?}");
        assert!((c.volume(cima) - CHEIA).abs() < 1e-6);
        assert!(c.doc(cima).external.is_empty());
        // refazer volta com `gravados` velhos: comando e link sumiram juntos, então não é isolamento
        c.a = com;
        let r = c.sync();
        assert_eq!(r.gravados, 2, "{r:?}");
        assert!(r.isolados.is_empty());
        assert!(c.volume(cima) < CHEIA - 1.);
    }

    #[test]
    fn suprimir_tira_e_devolve_o_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("suprimir");
        c.sync();
        c.a.features[0].suppressed = true;
        c.sync();
        let cima = c.cima;
        assert!((c.volume(cima) - CHEIA).abs() < 1e-6);
        c.a.features[0].suppressed = false;
        c.sync();
        assert!(c.volume(cima) < CHEIA - 1.);
    }

    #[test]
    fn isolar_deixa_o_furo_comum_e_tira_a_peca_do_escopo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("isolar");
        c.sync();
        let (cima, baixo) = (c.cima, c.baixo);
        let d = c.doc(baixo);
        let ext = d.external[0].id;
        let hole = d.external[0].target.unwrap();
        let dir = c.dir.clone();
        links::aplicar(&c.a, Some(&dir), &mut c.cache, baixo, ext, AcaoLink::Isolar).unwrap();
        let r = c.sync();
        assert_eq!(r.isolados, vec![(f, baixo)], "{r:?}");
        assert_eq!(c.a.features[0].targets, vec![cima]);
        let d = c.doc(baixo);
        assert!(d.external.is_empty() && d.feature(hole).is_some() && !editado_pela_montagem(&d, hole));
        // a série deixa de mandar: apagá-la não leva o furo isolado
        c.a.features.clear();
        c.sync();
        assert!(c.volume(baixo) < CHEIA - 1.);
        assert!((c.volume(cima) - CHEIA).abs() < 1e-6);
    }

    #[test]
    fn travado_nao_acompanha_ate_destravar() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("travar");
        c.sync();
        let baixo = c.baixo;
        let ext = c.doc(baixo).external[0].id;
        let dir = c.dir.clone();
        links::aplicar(&c.a, Some(&dir), &mut c.cache, baixo, ext, AcaoLink::Travar).unwrap();
        c.a.node_mut(baixo).unwrap().placement.origin = [2., 0., 0.];
        let r = c.sync();
        assert!(r.pecas.is_empty(), "{r:?}");
        links::aplicar(&c.a, Some(&dir), &mut c.cache, baixo, ext, AcaoLink::Destravar).unwrap();
        assert_eq!(c.sync().movidos, 1);
    }

    #[test]
    fn furo_apagado_na_peca_volta_pelo_link() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("apagado");
        c.sync();
        let baixo = c.baixo;
        let mut d = c.doc(baixo);
        let hole = d.external[0].target.unwrap();
        d.features.retain(|x| x.id != hole);
        d.rollback = d.rollback.min(d.features.len());
        std::fs::write(c.dir.join("baixo.fkp"), d.to_json()).unwrap();
        c.cache.forget(&c.dir.join("baixo.fkp"));
        assert_eq!(c.sync().gravados, 1);
        assert_eq!(c.doc(baixo).external.len(), 1, "o link é o mesmo");
        assert!(verificar(&c.a, f, Some(&c.dir), &mut c.cache).iter().all(Verificacao::ok));
    }

    #[test]
    fn tirar_do_escopo_remove_so_daquela_peca() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("escopo");
        c.sync();
        let (cima, baixo) = (c.cima, c.baixo);
        c.a.features[0].targets.retain(|t| *t != baixo);
        assert_eq!(c.sync().removidos, 1);
        assert!((c.volume(baixo) - CHEIA).abs() < 1e-6);
        assert!(c.volume(cima) < CHEIA - 1.);
    }

    #[test]
    fn padrao_e_peca_fora_do_eixo_ficam_na_verificacao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("padrao");
        c.serie(f).padrao = Some(PatternKind::Linear { dir: [1., 0., 0.], count: "3".into(), spacing: "10".into() });
        let r = c.sync();
        assert_eq!(r.gravados, 6, "{r:?}");
        let cima = c.cima;
        let v = c.volume(cima);
        assert!((CHEIA - v - 3. * cilindro_vol(6., 10.)).abs() < 1e-2, "{v}");
        // com a placa de baixo fora do caminho, a verificação acusa
        let baixo = c.baixo;
        c.a.node_mut(baixo).unwrap().placement.origin = [500., 0., 0.];
        let r = c.sync();
        assert_eq!(r.faltas.len(), 1, "{r:?}");
        let v = verificar(&c.a, f, Some(&c.dir), &mut c.cache);
        assert!(!v.iter().find(|x| x.comp == baixo).unwrap().ok(), "{v:?}");
    }

    #[test]
    fn referencia_perdida_e_erro_e_nao_mexe_nos_furos() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("perdida");
        c.sync();
        let cima = c.cima;
        c.serie(f).eixo = EixoFuro::Publicado { comp: cima, nome: "Sumiu".into(), d: [0., 0., -1.], p: [0.; 3] };
        let r = c.sync();
        assert_eq!(r.erros.len(), 1, "{r:?}");
        assert_eq!(r.removidos + r.movidos, 0, "{r:?}");
        assert!(c.volume(cima) < CHEIA - 1.);
    }

    #[test]
    fn so_na_montagem_nao_toca_as_pecas_e_corta_a_vista() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, f) = Caso::novo("so_montagem");
        c.serie(f).gravar = false;
        let r = c.sync();
        assert!(r.pecas.is_empty(), "{r:?}");
        let cima = c.cima;
        assert!(c.doc(cima).external.is_empty());
        let v = crate::view::placed_shape(&c.a, cima, Some(&c.dir), &mut c.cache).unwrap().mass().volume;
        assert!((CHEIA - v - cilindro_vol(6., 10.)).abs() < 1e-3, "{v}");
    }

    #[test]
    fn duas_instancias_do_arquivo_avisam() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("instancias");
        std::fs::write(dir.join("p.fkp"), placa().to_json()).unwrap();
        let mut a = Assembly::new("I");
        let n1 = a.add_component("P", NodeKind::Part { path: "p.fkp".into() }, Placement::default(), None);
        a.add_component("P", NodeKind::Part { path: "p.fkp".into() }, Placement { origin: [200., 0., 0.], ..Default::default() }, None);
        let mut cache = PartCache::default();
        let s = SerieFuros::nova(EixoFuro::Componente { comp: n1, p: [0., 0., 10.], d: [0., 0., -1.] }, "6");
        a.add_serie_furos(s, vec![], Some(&dir), &mut cache).unwrap();
        let arq = dir.join("i.fka");
        let r = sincronizar(&mut a, Some(&dir), Some(&arq), &mut cache, None);
        assert_eq!(r.avisos.len(), 1, "{r:?}");
    }

    #[test]
    fn link_de_furo_nao_fecha_ciclo_nem_vira_pendurado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("ciclo");
        c.sync();
        let dir = c.dir.clone();
        let rel = links::atualizar(&c.a, Some(&dir), &mut c.cache, links::Gatilho::Manual);
        assert_eq!((rel.pendurados, rel.atualizados), (0, 0), "{rel:?}");
        assert!(!links::depende_de(&dir.join("cima.fkp"), &c.arq, None), "a peça não depende da montagem");
    }

    #[test]
    fn subconjunto_leva_o_furo_da_peca_que_foi() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("sub");
        c.sync();
        let (cima, baixo) = (c.cima, c.baixo);
        let (mut sub_a, _, rel) = c.a.extract_subassembly(&[cima], "Sub", "sub.fka");
        assert!(rel.perdidas.is_empty(), "{rel:?}");
        let arq_sub = c.dir.join("sub.fka");
        adotar(&sub_a, "Conjunto", Some(&c.dir), &arq_sub, &mut c.cache);
        let r = sincronizar(&mut sub_a, Some(&c.dir), Some(&arq_sub), &mut c.cache, None);
        assert_eq!((r.gravados, r.removidos), (0, 0), "o furo é o mesmo, com outro dono: {r:?}");
        let r = c.sync();
        assert_eq!(r.removidos, 0, "a peça de baixo fica no mesmo lugar: {r:?}");
        let txt = std::fs::read_to_string(c.dir.join("cima.fkp")).unwrap();
        let d = Document::from_json(&txt).unwrap();
        assert_eq!((d.external.len(), d.external[0].source.as_str(), d.external[0].path.as_str()), (1, "Sub", "sub.fka"));
        let v = fenkra_solidos::recompute(&d);
        let vol: f64 = v.bodies.iter().map(|b| b.mass.volume).sum();
        assert!((CHEIA - vol - cilindro_vol(6., 10.)).abs() < 1e-3, "um furo só: {vol}");
        assert!((CHEIA - c.volume(baixo) - cilindro_vol(6., 10.)).abs() < 1e-3);
    }

    #[test]
    fn peca_em_edicao_nao_e_regravada() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut c, _) = Caso::novo("aberta");
        let aberta = c.dir.join("cima.fkp");
        let r = sincronizar(&mut c.a, Some(&c.dir), Some(&c.arq), &mut c.cache, Some(&aberta));
        assert_eq!(r.pecas, vec![c.dir.join("baixo.fkp")], "{r:?}");
    }

    #[test]
    fn apagar_a_ancora_congela_o_eixo_e_json_volta_igual() {
        let mut a = Assembly::new("A");
        let c1 = a.add_component("X", NodeKind::Part { path: "x.fkp".into() }, Placement { origin: [1., 2., 3.], ..Default::default() }, None);
        let c2 = a.add_component("Y", NodeKind::Part { path: "y.fkp".into() }, Placement::default(), None);
        a.add_feature(AsmFeatKind::Serie(SerieFuros::nova(EixoFuro::Componente { comp: c1, p: [0., 0., 1.], d: [0., 0., -1.] }, "6")), vec![c1, c2]);
        assert_eq!(Assembly::from_json(&a.to_json()).unwrap(), a);
        a.delete(c1);
        let AsmFeatKind::Serie(s) = &a.features[0].kind else { panic!() };
        assert_eq!(s.eixo, EixoFuro::Cotas { x: "1".into(), y: "2".into(), z: "4".into(), d: [0., 0., -1.] });
        assert_eq!(a.features[0].targets, vec![c2]);
    }
}
