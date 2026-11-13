// SPDX-License-Identifier: LGPL-2.1-or-later
//! Corpo de malha na árvore (B170, ADR 0060).
//!
//! Um coletor pode ter um corpo de malha ao lado do corpo B-rep. Ele não tem `Shape`: mora em
//! `Ctx::malhas` e sai no resultado como um [`BodyResult`] de face única, e por isso a viewport, a
//! seleção e as medidas não precisam saber que não é B-rep. Nenhum comando de sólido enxerga esse
//! corpo; a ponte entre os dois mundos são as operações `ParaBrep` e `DeBrep`.
//!
//! Cada operação é pura: o resultado depende só da entrada e dos parâmetros. Por isso o resultado fica
//! guardado por uma chave de conteúdo (hash da chave anterior com os parâmetros), fora do `Cache`
//! do recálculo. Os comandos de malha sempre "rodam", mas rodar é achar a chave: desfazer, reabrir ou
//! exportar não refazem uma decimação de milhões de triângulos.

use super::*;
use fenkra_core::malha::{MeshOp, MeshOpKind};
use fenkra_kernel::NO_FACE;
use fenkra_malha::booleana::{booleana, OpMalha};
use fenkra_malha::decimar::OpcoesDecimar;
use fenkra_malha::desvio::{desvio, Desvio};
use fenkra_malha::{embutir as texto, juntar, Bvh, Diagnostico, Malha, OpcoesReparo};
use std::sync::Mutex;

/// Teto do caminho até o B-rep: cada triângulo vira uma face costurada.
pub const LIMITE_BREP: usize = 200_000;
/// Teto de memória das malhas guardadas entre cálculos.
const TETO_GUARDADAS: usize = 768 * 1024 * 1024;
/// Dobra entre faces acima disto fica viva: separa as normais e desenha a aresta.
const VINCO_GRAUS: f64 = 30.;
/// Vértice com faces demais fica com a normal da própria face: a média custa o quadrado do grau.
const GRAU_MAXIMO: usize = 64;
/// Acima disto só o contorno aberto é desenhado: linha viva em tudo vira um borrão.
const TETO_ARESTAS: usize = 400_000;

type Chave = [u8; 32];

/// Resultado de uma operação: a malha e o que dela se mostra.
pub struct Entrada {
    pub malha: Malha,
    pub exibicao: Mesh,
    pub massa: MassProps,
    pub diagnostico: Diagnostico,
    /// o que a operação fez, para o aviso do comando
    pub nota: Option<String>,
}

impl Entrada {
    fn bytes(&self) -> usize {
        self.malha.pos.len() * 24 + self.malha.tris.len() * 12 + self.exibicao.bytes()
    }
}

/// Corpo de malha de um coletor, durante o cálculo.
#[derive(Clone)]
pub struct CorpoMalha {
    pub entrada: Arc<Entrada>,
    chave: Chave,
    /// comando que deixou o corpo como está
    dono: Id,
}

#[derive(Default)]
struct Guardadas {
    itens: HashMap<Chave, (Arc<Entrada>, u64)>,
    relogio: u64,
    bytes: usize,
}

static GUARDADAS: Mutex<Option<Guardadas>> = Mutex::new(None);

fn guardada(chave: &Chave) -> Option<Arc<Entrada>> {
    let mut g = GUARDADAS.lock().ok()?;
    let g = g.get_or_insert_with(Default::default);
    g.relogio += 1;
    let agora = g.relogio;
    g.itens.get_mut(chave).map(|(e, uso)| {
        *uso = agora;
        e.clone()
    })
}

fn guardar(chave: Chave, e: Arc<Entrada>) {
    let Ok(mut g) = GUARDADAS.lock() else { return };
    let g = g.get_or_insert_with(Default::default);
    g.relogio += 1;
    g.bytes += e.bytes();
    let agora = g.relogio;
    if let Some((velha, _)) = g.itens.insert(chave, (e, agora)) {
        g.bytes -= velha.bytes();
    }
    while g.bytes > TETO_GUARDADAS && g.itens.len() > 1 {
        let Some(k) = g.itens.iter().min_by_key(|(_, (_, uso))| *uso).map(|(k, _)| *k) else { break };
        if let Some((e, _)) = g.itens.remove(&k) {
            g.bytes -= e.bytes();
        }
    }
}

fn chave_de(partes: &[&[u8]]) -> Chave {
    let mut h = blake3::Hasher::new();
    for p in partes {
        h.update(&(p.len() as u64).to_le_bytes());
        h.update(p);
    }
    *h.finalize().as_bytes()
}

/// Malha de exibição: vértices repetidos só onde a normal quebra, e uma face única para o corpo todo.
pub fn exibicao(m: &Malha) -> Mesh {
    let normais: Vec<[f64; 3]> = m.tris.iter().map(|t| {
        let [a, b, c] = t.map(|i| m.pos[i as usize]);
        geo3d::cross(geo3d::sub(b, a), geo3d::sub(c, a))
    }).collect();
    // faces de cada vértice, em lista corrida
    let mut inicio = vec![0u32; m.pos.len() + 1];
    m.tris.iter().flatten().for_each(|v| inicio[*v as usize + 1] += 1);
    for i in 1..inicio.len() {
        inicio[i] += inicio[i - 1];
    }
    let mut cursor = inicio.clone();
    let mut faces = vec![0u32; m.tris.len() * 3];
    for (f, t) in m.tris.iter().enumerate() {
        for v in t {
            faces[cursor[*v as usize] as usize] = f as u32;
            cursor[*v as usize] += 1;
        }
    }
    let vinco = VINCO_GRAUS.to_radians().cos();
    let mut saida = Mesh { face_ranges: vec![0], ..Default::default() };
    let mut novos: HashMap<(u32, [i16; 3]), u32> = HashMap::with_capacity(m.pos.len());
    for (f, t) in m.tris.iter().enumerate() {
        let propria = unit(normais[f]);
        for &v in t {
            let lista = &faces[inicio[v as usize] as usize..inicio[v as usize + 1] as usize];
            let n = if lista.len() > GRAU_MAXIMO {
                propria
            } else {
                let soma = lista.iter().fold([0.; 3], |s, g| {
                    let o = normais[*g as usize];
                    if dot(unit(o), propria) >= vinco { geo3d::add(s, o) } else { s }
                });
                unit(soma)
            };
            let q = n.map(|x| (x * 1000.) as i16);
            let i = *novos.entry((v, q)).or_insert_with(|| {
                saida.positions.extend(m.pos[v as usize].map(|x| x as f32));
                saida.normals.extend(n.map(|x| x as f32));
                (saida.positions.len() / 3 - 1) as u32
            });
            saida.indices.push(i);
        }
    }
    let mut vivas = fenkra_malha::diagnostico::arestas_vivas(m, VINCO_GRAUS, false);
    if vivas.len() > TETO_ARESTAS {
        vivas = fenkra_malha::diagnostico::arestas_vivas(m, VINCO_GRAUS, true);
    }
    if !vivas.is_empty() {
        saida.edges = vivas.iter().flat_map(|(a, b)| [*a, *b]).flat_map(|v| m.pos[v as usize].map(|x| x as f32)).collect();
        saida.edge_ranges = vec![0];
        let p = m.pos[vivas[0].0 as usize];
        saida.edge_mids = p.iter().map(|x| *x as f32).collect();
        saida.edge_geom = vec![0.; fenkra_kernel::EDGE_GEOM];
        saida.edge_smooth = vec![0];
        saida.edge_faces = vec![[0, NO_FACE]];
    }
    let centro = m.centro();
    saida.face_info = vec![fenkra_kernel::FaceInfo { kind: 0, dir: [0., 0., 1.], area: m.area(), point: m.pos.first().copied().unwrap_or(centro) }];
    saida
}

fn entrada(malha: Malha, nota: Option<String>) -> Entrada {
    let diagnostico = malha.diagnostico();
    let massa = MassProps { volume: if diagnostico.arestas_livres == 0 { malha.volume() } else { 0. }, area: diagnostico.area, center: malha.centro() };
    Entrada { exibicao: exibicao(&malha), malha, massa, diagnostico, nota }
}

/// Acha pela chave ou calcula e guarda.
fn obter(chave: Chave, calcular: impl FnOnce() -> Result<(Malha, Option<String>), String>) -> Result<Arc<Entrada>, String> {
    if let Some(e) = guardada(&chave) {
        return Ok(e);
    }
    let (malha, nota) = calcular()?;
    if malha.tris.is_empty() {
        return Err("a operação deixou a malha sem triângulos".into());
    }
    let e = Arc::new(entrada(malha, nota));
    guardar(chave, e.clone());
    Ok(e)
}

/// Guarda a malha no documento e devolve a chave para o comando.
pub fn embutir(doc: &mut Document, m: &Malha) -> String {
    let t = texto::para_texto(m);
    let chave = blake3::hash(t.as_bytes()).to_hex().to_string();
    doc.malhas.entry(chave.clone()).or_insert_with(|| fenkra_core::malha::MalhaEmbutida(t.into()));
    chave
}

/// Malha do kernel soldada: a malha de uma forma repete o vértice em cada face.
pub fn malha_da_forma(s: &Shape, desvio: f64, angulo: f64) -> Result<Malha, String> {
    let k = s.mesh_angle(desvio, angulo).map_err(|e| e.0)?;
    let pos = k.positions.chunks_exact(3).map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]).collect();
    let tris = k.indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
    Ok(Malha { pos, tris, ..Default::default() }.soldada())
}

/// Identidade de uma forma para a chave: o cálculo fora do recompute não tem a assinatura do corpo.
fn impressao_da_forma(s: &Shape) -> String {
    format!("{:?}{:?}{}", s.mass(), s.bbox(), s.face_count())
}

/// Desvio padrão de quem pede malha de uma forma sem dizer: um milésimo da diagonal.
fn desvio_automatico(s: &Shape) -> f64 {
    s.bbox().map(|(a, b)| geo3d::dist(a, b) / 1000.).unwrap_or(0.05).max(1e-3)
}

/// Comando que precisa passar pelo `run` mesmo com a assinatura igual: quem produz corpo de malha.
/// O cache do recálculo só guarda `Shape`; o de malha é o de conteúdo, e passar por ele é quase de graça.
pub(crate) fn sempre_roda(k: &FeatureKind) -> bool {
    match k {
        FeatureKind::MeshImport(m) => m.corpo,
        FeatureKind::MeshOp(m) => !matches!(m.op, MeshOpKind::ParaBrep { .. }),
        _ => false,
    }
}

/// O cache do recálculo devolveu o resultado do comando sem rodar. `ParaBrep` é o único de malha que
/// passa por ali, e o corpo de malha que ele consumiu tem de sumir do mesmo jeito.
pub(crate) fn ao_reaproveitar(malhas: &mut HashMap<Id, CorpoMalha>, f: &Feature) {
    if matches!(&f.kind, FeatureKind::MeshOp(m) if matches!(m.op, MeshOpKind::ParaBrep { .. })) {
        malhas.remove(&f.target);
    }
}

impl Ctx<'_> {
    pub(crate) fn importar_corpo_malha(&mut self, f: &Feature, m: &MeshImport) -> Result<Option<String>, String> {
        let origem: Vec<u8> = if m.dados.is_empty() {
            let meta = std::fs::metadata(&m.path).map_err(|e| format!("não foi possível ler '{}': {e}", m.path))?;
            format!("{}|{}|{:?}", m.path, meta.len(), meta.modified().ok()).into_bytes()
        } else {
            m.dados.clone().into_bytes()
        };
        let chave = chave_de(&[b"importar", &origem]);
        let e = obter(chave, || {
            let malha = if m.dados.is_empty() {
                let cena = fenkra_malha::ler_arquivo(std::path::Path::new(&m.path)).map_err(|e| e.to_string())?;
                let partes = cena.no_lugar();
                juntar(&partes).soldada()
            } else {
                let t = self.doc.malhas.get(&m.dados).ok_or("a malha embutida não está no documento")?;
                texto::de_texto(&t.0).map_err(|e| e.to_string())?
            };
            Ok((malha, None))
        })?;
        // segunda importação no mesmo coletor: as malhas ficam num corpo só, como os sólidos unidos
        let corpo = match self.malhas.get(&f.target) {
            Some(atual) => {
                let chave = chave_de(&[b"juntar", &atual.chave, &chave]);
                let (a, b) = (atual.entrada.clone(), e);
                CorpoMalha { entrada: obter(chave, || Ok((juntar([&a.malha, &b.malha]), None)))?, chave, dono: f.id }
            }
            None => CorpoMalha { entrada: e, chave, dono: f.id },
        };
        let aviso = aviso_de(&corpo.entrada.diagnostico);
        self.malhas.insert(f.target, corpo);
        Ok(aviso)
    }

    fn corpo_de(&self, alvo: Id) -> Result<CorpoMalha, String> {
        self.malhas.get(&alvo).cloned().ok_or_else(|| format!("o coletor '{}' não tem corpo de malha", self.doc.name_of(alvo)))
    }

    pub(crate) fn operar_malha(&mut self, f: &Feature, m: &MeshOp) -> Result<Option<String>, String> {
        let parametros = serde_json::to_vec(&m.op).unwrap_or_default();
        match &m.op {
            MeshOpKind::Reparar(r) => {
                let c = self.corpo_de(f.target)?;
                let o = OpcoesReparo {
                    solda: ev(&r.solda, &self.vars, "solda")?.max(0.),
                    nao_manifold: r.nao_manifold,
                    normais: r.normais,
                    furos: r.furos,
                    furo_maximo: r.furo_maximo as usize,
                    soltas: r.soltas,
                    solta_minima: if r.solta_minima.is_empty() { 0.001 } else { ev(&r.solta_minima, &self.vars, "parte mínima")?.clamp(0., 1.) },
                };
                let chave = chave_de(&[b"reparar", &c.chave, &parametros, format!("{o:?}").as_bytes()]);
                let e = obter(chave, || {
                    let (malha, rel) = c.entrada.malha.reparada(&o);
                    let nota = format!(
                        "{} vértices soldados, {} triângulos tirados, {} vértices separados, {} triângulos virados, {} furos fechados, {} partes tiradas",
                        rel.vertices_soldados, rel.triangulos_removidos, rel.vertices_separados, rel.triangulos_virados, rel.furos_fechados, rel.partes_removidas
                    );
                    Ok((malha, Some(nota)))
                })?;
                self.por_corpo(f, chave, e)
            }
            MeshOpKind::Decimar(d) => {
                let c = self.corpo_de(f.target)?;
                let o = OpcoesDecimar { proporcao: ev(&d.proporcao, &self.vars, "proporção")?.clamp(0.001, 1.), desvio: ev(&d.desvio, &self.vars, "desvio")?.max(0.) };
                let chave = chave_de(&[b"decimar", &c.chave, format!("{o:?}").as_bytes()]);
                let n = c.entrada.malha.tris.len();
                let e = obter(chave, || {
                    let malha = c.entrada.malha.decimada(&o);
                    let nota = format!("{n} → {} triângulos", malha.tris.len());
                    Ok((malha, Some(nota)))
                })?;
                self.por_corpo(f, chave, e)
            }
            MeshOpKind::Booleana(b) => {
                if b.tool == f.target {
                    return Err("o corpo ferramenta precisa ser de outro coletor".into());
                }
                let c = self.corpo_de(f.target)?;
                let (malha_tool, id_tool): (Arc<Entrada>, Vec<u8>) = match (self.malhas.get(&b.tool), self.body(b.tool)) {
                    (Some(t), _) => (t.entrada.clone(), t.chave.to_vec()),
                    (None, Some(s)) => {
                        let id = impressao_da_forma(s).into_bytes();
                        let k = chave_de(&[b"forma", &id]);
                        let s = s.clone();
                        (obter(k, || Ok((malha_da_forma(&s, desvio_automatico(&s), 0.26)?, None)))?, k.to_vec())
                    }
                    (None, None) => return Err(format!("o coletor '{}' não tem corpo (ou vem depois)", self.doc.name_of(b.tool))),
                };
                let op = match b.op {
                    BoolOp::Add => OpMalha::Unir,
                    BoolOp::Cut => OpMalha::Subtrair,
                    BoolOp::Intersect => OpMalha::Intersectar,
                };
                let chave = chave_de(&[b"booleana", &c.chave, &id_tool, format!("{op:?}").as_bytes()]);
                let e = obter(chave, || booleana(&c.entrada.malha, &malha_tool.malha, op).map(|m| (m, None)).map_err(|e| e.to_string()))?;
                if !b.keep_tool {
                    self.malhas.remove(&b.tool);
                    self.bodies.remove(&b.tool);
                }
                self.por_corpo(f, chave, e)
            }
            MeshOpKind::ParaBrep { solid } => {
                let c = self.corpo_de(f.target)?;
                let n = c.entrada.malha.tris.len();
                if n > LIMITE_BREP {
                    return Err(format!("malha grande demais para virar sólido ({n} triângulos; máximo {LIMITE_BREP}): decime antes"));
                }
                let mut forma: Option<Shape> = None;
                for p in c.entrada.malha.componentes() {
                    let tris: Vec<f64> = p.tris.iter().flat_map(|t| t.iter().flat_map(|i| p.pos[*i as usize])).collect();
                    let s = Shape::from_mesh(&tris, *solid && p.fechada()).map_err(|e| e.0)?;
                    forma = Some(match forma {
                        Some(f) => f.compound(&s),
                        None => s,
                    });
                }
                let s = forma.ok_or("a malha não tem triângulos")?;
                self.malhas.remove(&f.target);
                if s.is_solid() {
                    self.apply(f.target, s, BoolOp::Add, f.id)?;
                    Ok(Some(format!("{n} triângulos → sólido")))
                } else {
                    let r = match self.body(f.target) {
                        Some(b) => b.compound(&s),
                        None => s,
                    };
                    self.bodies.insert(f.target, fresh_body(r, f.id));
                    Ok(Some(format!("{n} triângulos → superfície aberta (a malha não fecha um volume)")))
                }
            }
            MeshOpKind::DeBrep { desvio, angulo } => {
                let s = self.body(f.target).ok_or("não há corpo B-rep para converter")?.clone();
                let (d, a) = (ev(desvio, &self.vars, "desvio")?, ev(angulo, &self.vars, "ângulo")?);
                let d = if d > 0. { d } else { desvio_automatico(&s) };
                let chave = chave_de(&[b"debrep", impressao_da_forma(&s).as_bytes(), format!("{d}|{a}").as_bytes()]);
                let e = obter(chave, || Ok((malha_da_forma(&s, d, a.clamp(1., 60.).to_radians())?, None)))?;
                self.bodies.remove(&f.target);
                self.por_corpo(f, chave, e)
            }
        }
    }

    fn por_corpo(&mut self, f: &Feature, chave: Chave, e: Arc<Entrada>) -> Result<Option<String>, String> {
        let nota = e.nota.clone();
        let aviso = aviso_de(&e.diagnostico);
        self.malhas.insert(f.target, CorpoMalha { entrada: e, chave, dono: f.id });
        Ok(match (nota, aviso) {
            (Some(n), Some(a)) => Some(format!("{n}; {a}")),
            (n, a) => n.or(a),
        })
    }
}

/// O que o comando deve dizer sobre uma malha que não fecha.
fn aviso_de(d: &Diagnostico) -> Option<String> {
    (!d.estanque()).then(|| format!("malha não estanque: {} arestas livres, {} não manifold, {} invertidas", d.arestas_livres, d.arestas_nao_manifold + d.vertices_nao_manifold, d.arestas_invertidas))
}

/// Resultado do corpo de malha, para a lista de corpos do recálculo.
pub(crate) fn resultado(c: &BodyCollector, m: &CorpoMalha) -> BodyResult {
    BodyResult { collector: c.id, color: c.color, mesh: m.entrada.exibicao.clone(), mass: m.entrada.massa, faces: 1, face_owner: vec![m.dono], face_role: vec![-1] }
}

/// Corpo de malha pronto para sair do documento.
pub struct CorpoDeMalha {
    pub coletor: Id,
    pub nome: String,
    pub cor: [u8; 3],
    pub entrada: Arc<Entrada>,
}

fn com_contexto<R>(doc: &Document, f: impl FnOnce(&Ctx) -> R) -> Option<R> {
    let efetivo = doc.efetivo().ok()?;
    let doc: &Document = &efetivo;
    let vars = doc.eval_params().ok()?;
    let mut cx = Ctx {
        doc,
        vars,
        qualidade: Default::default(),
        bodies: HashMap::new(),
        sketches: HashMap::new(),
        status: HashMap::new(),
        tools: HashMap::new(),
        memo: RefCell::new(HashMap::new()),
        notes: Default::default(),
        body_sig: HashMap::new(),
        last_sheet: None,
        sheet_maps: vec![],
        cut_outlines: vec![],
        malhas: HashMap::new(),
        roscas: vec![],
        previas: vec![],
        leve: None,
        relatorios_forma: vec![],
        formas_resolvidas: vec![],
        steps: HashMap::new(),
        miter_next: false,
        promoted: RefCell::new(None),
        aviso_cantos: RefCell::new(None),
    };
    for g in doc.active_features().map(|(_, g)| g) {
        let _ = cx.run(g);
    }
    Some(f(&cx))
}

/// O documento tem comando que faz corpo de malha? Não calcula nada.
pub fn tem_corpo_de_malha(doc: &Document) -> bool {
    doc.features.iter().any(|f| matches!(&f.kind, FeatureKind::MeshImport(m) if m.corpo) || matches!(&f.kind, FeatureKind::MeshOp(_)))
}

/// Corpos de malha do documento, na ordem dos coletores.
pub fn corpos_de_malha(doc: &Document) -> Vec<CorpoDeMalha> {
    com_contexto(doc, |cx| doc_corpos(cx)).unwrap_or_default()
}

fn doc_corpos(cx: &Ctx) -> Vec<CorpoDeMalha> {
    cx.doc.collectors.iter().filter_map(|c| cx.malhas.get(&c.id).map(|m| CorpoDeMalha { coletor: c.id, nome: c.name.clone(), cor: c.color, entrada: m.entrada.clone() })).collect()
}

/// Malha de cada corpo do documento, B-rep incluso, para quem grava e confere triângulos. O desvio é
/// em mm e o ângulo em radianos; só valem para o corpo B-rep.
pub fn malhas_do_documento(doc: &Document, desvio: f64, angulo: f64) -> Result<Vec<(Id, String, [u8; 3], Malha)>, String> {
    com_contexto(doc, |cx| -> Result<Vec<(Id, String, [u8; 3], Malha)>, String> {
        let mut v = Vec::new();
        for c in &cx.doc.collectors {
            if let Some(s) = cx.body(c.id) {
                v.push((c.id, c.name.clone(), c.color, malha_da_forma(s, desvio, angulo)?));
            }
            if let Some(m) = cx.malhas.get(&c.id) {
                v.push((c.id, c.name.clone(), c.color, m.entrada.malha.clone()));
            }
        }
        Ok(v)
    })
    .ok_or("o documento não pôde ser calculado")?
}

/// Corpo com a malha separada por cor.
pub struct CorpoColorido {
    pub coletor: Id,
    pub nome: String,
    pub partes: Vec<(Malha, [u8; 3])>,
}

/// Malha de um corpo B-rep, partida pela cor de cada face. A cor é a do comando que criou a face, ou a
/// do coletor. A partição é por face, e a solda das partes devolve a casca inteira.
fn partes_por_cor(s: &Shape, donos: &[Id], doc: &Document, cor: [u8; 3], desvio: f64, angulo: f64) -> Result<Vec<(Malha, [u8; 3])>, String> {
    let k = s.mesh_angle(desvio, angulo).map_err(|e| e.0)?;
    let pos: Vec<[f64; 3]> = k.positions.chunks_exact(3).map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]).collect();
    let mut grupos: Vec<([u8; 3], Vec<[u32; 3]>)> = Vec::new();
    for (i, inicio) in k.face_ranges.iter().enumerate() {
        let fim = k.face_ranges.get(i + 1).copied().unwrap_or(k.indices.len() as u32);
        let propria = donos.get(i).and_then(|d| doc.features.iter().find(|f| f.id == *d)).and_then(|f| f.appearance.as_ref()).map(|a| a.color).unwrap_or(cor);
        let g = match grupos.iter().position(|(c, _)| *c == propria) {
            Some(g) => g,
            None => {
                grupos.push((propria, Vec::new()));
                grupos.len() - 1
            }
        };
        grupos[g].1.extend(k.indices[*inicio as usize..fim as usize].chunks_exact(3).map(|t| [t[0], t[1], t[2]]));
    }
    Ok(grupos.into_iter().map(|(c, tris)| (Malha { pos: pos.clone(), tris, ..Default::default() }.soldada(), c)).collect())
}

/// Corpos do documento para impressão, B-rep pela tesselação pedida e malha como está.
pub fn corpos_coloridos(doc: &Document, desvio: f64, angulo: f64) -> Result<Vec<CorpoColorido>, String> {
    com_contexto(doc, |cx| -> Result<Vec<CorpoColorido>, String> {
        let mut v = Vec::new();
        for c in cx.doc.collectors.iter().filter(|c| c.visible) {
            if let Some((s, donos, _)) = cx.body_owned(c.id) {
                v.push(CorpoColorido { coletor: c.id, nome: c.name.clone(), partes: partes_por_cor(s, donos, cx.doc, c.color, desvio, angulo)? });
            }
            if let Some(m) = cx.malhas.get(&c.id) {
                v.push(CorpoColorido { coletor: c.id, nome: c.name.clone(), partes: vec![(m.entrada.malha.clone(), c.color)] });
            }
        }
        Ok(v)
    })
    .ok_or("o documento não pôde ser calculado")?
}

/// STL com corpo de malha: corpo B-rep sai pela malha do preset, corpo de malha sai como está.
pub(crate) fn exportar_stl(cx: &Ctx, coletor: Option<Id>, forma: &Shape, path: &str, preset: PresetStl) -> Result<(), String> {
    let mut partes: Vec<Malha> = cx.doc.collectors.iter().filter(|c| coletor.is_none_or(|x| x == c.id)).filter_map(|c| cx.malhas.get(&c.id)).map(|m| m.entrada.malha.clone()).collect();
    if !forma.is_null() {
        let q = preset.qualidade();
        let diagonal = forma.bbox().map(|(a, b)| geo3d::dist(a, b)).unwrap_or(100.);
        partes.push(malha_da_forma(forma, q.desvio(diagonal), q.angulo_rad())?);
    }
    if partes.is_empty() {
        return Err("nada para exportar".into());
    }
    let mut cena = fenkra_malha::Cena::default();
    cena.raiz_com(juntar(&partes));
    std::fs::write(path, fenkra_malha::stl::gravar(&cena, true, 1.)).map_err(|e| e.to_string())
}

/// Corpo de malha contra corpo B-rep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesvioBrep {
    /// da malha até a superfície do B-rep
    pub ida: Desvio,
    /// do B-rep até a malha
    pub volta: Desvio,
    /// desvio da tesselação usada como referência: o erro da própria medida
    pub incerteza: f64,
}

/// Mede o corpo de malha de um coletor contra o corpo B-rep de outro. O B-rep é tesselado bem mais
/// fino que a diagonal pede para a exibição, e `incerteza` diz o quanto.
pub fn desvio_malha_brep(doc: &Document, coletor_malha: Id, coletor_brep: Id) -> Result<DesvioBrep, String> {
    com_contexto(doc, |cx| {
        let m = cx.malhas.get(&coletor_malha).ok_or("o coletor não tem corpo de malha")?;
        let s = cx.body(coletor_brep).ok_or("o coletor não tem corpo B-rep")?;
        let incerteza = desvio_automatico(s) / 5.;
        let referencia = malha_da_forma(s, incerteza, 0.1)?;
        let ida = desvio(&m.entrada.malha, &Bvh::nova(&referencia));
        let volta = desvio(&referencia, &Bvh::nova(&m.entrada.malha));
        Ok(DesvioBrep { ida, volta, incerteza })
    })
    .ok_or("o documento não pôde ser calculado")?
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::malha::{MeshBoolean, MeshDecimate, MeshRepair};

    fn caixa(l: [f64; 3], d: [f64; 3]) -> Malha {
        let pos: Vec<[f64; 3]> = (0..8).map(|i| [if i & 1 > 0 { l[0] } else { 0. } + d[0], if i & 2 > 0 { l[1] } else { 0. } + d[1], if i & 4 > 0 { l[2] } else { 0. } + d[2]]).collect();
        let quads: [[u32; 4]; 6] = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        Malha { pos, tris: quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect(), ..Default::default() }
    }

    /// Grade fechada: uma caixa com o topo subdividido, para ter triângulos que decimar.
    fn placa(n: u32) -> Malha {
        let mut m = Malha::default();
        for j in 0..=n {
            for i in 0..=n {
                let (x, y) = (f64::from(i), f64::from(j));
                m.pos.push([x, y, 10. + ((x * 0.3).sin() + (y * 0.2).cos()) * 2.]);
            }
        }
        let k = |i: u32, j: u32| j * (n + 1) + i;
        for j in 0..n {
            for i in 0..n {
                m.tris.push([k(i, j), k(i + 1, j), k(i + 1, j + 1)]);
                m.tris.push([k(i, j), k(i + 1, j + 1), k(i, j + 1)]);
            }
        }
        let base = m.pos.len() as u32;
        let aro: Vec<u32> = (0..n).map(|i| k(i, 0)).chain((0..n).map(|j| k(n, j))).chain((0..n).rev().map(|i| k(i + 1, n))).chain((0..n).rev().map(|j| k(0, j + 1))).collect();
        for v in &aro {
            let p = m.pos[*v as usize];
            m.pos.push([p[0], p[1], 0.]);
        }
        let l = aro.len() as u32;
        for i in 0..l {
            let (a, b, c, d) = (aro[i as usize], aro[((i + 1) % l) as usize], base + i, base + (i + 1) % l);
            m.tris.push([a, c, d]);
            m.tris.push([a, d, b]);
        }
        let centro = m.pos.len() as u32;
        m.pos.push([f64::from(n) / 2., f64::from(n) / 2., 0.]);
        for i in 0..l {
            m.tris.push([centro, base + (i + 1) % l, base + i]);
        }
        m
    }

    fn corpo(d: &mut Document, m: &Malha) -> Id {
        let dados = embutir(d, m);
        d.add_feature(FeatureKind::MeshImport(MeshImport { path: "peça.stl".into(), solid: false, corpo: true, dados }))
    }

    fn sem_erro(d: &Document, r: &RecomputeResult) {
        for f in &d.features {
            assert!(!matches!(r.status[&f.id], Status::Error(_)), "{:?}", r.status[&f.id]);
        }
    }

    fn caixa_do_corpo(b: &BodyResult) -> ([f32; 3], [f32; 3]) {
        b.mesh.positions.chunks_exact(3).fold(([f32::MAX; 3], [f32::MIN; 3]), |(lo, hi), p| ([0, 1, 2].map(|i| lo[i].min(p[i])), [0, 1, 2].map(|i| hi[i].max(p[i]))))
    }

    #[test]
    fn corpo_de_malha_entra_na_arvore_com_volume_e_caixa() {
        let mut d = Document::new("m");
        corpo(&mut d, &caixa([10., 20., 30.], [1., 2., 3.]));
        let r = recompute(&d);
        sem_erro(&d, &r);
        assert_eq!(r.bodies.len(), 1);
        assert!((r.bodies[0].mass.volume - 6000.).abs() < 1e-6);
        assert!((r.bodies[0].mass.area - 2200.).abs() < 1e-6);
        assert_eq!(caixa_do_corpo(&r.bodies[0]), ([1., 2., 3.], [11., 22., 33.]));
        assert_eq!(r.bodies[0].mesh.face_ranges, vec![0]);
        assert_eq!(r.bodies[0].face_owner.len(), r.bodies[0].mesh.face_ranges.len());
        assert_eq!(r.bodies[0].mesh.indices.len(), 36);
        // as arestas vivas da caixa são desenhadas, e a normal quebra em cada canto
        assert_eq!(r.bodies[0].mesh.edges.len() / 6, 12 + 0, "doze arestas, sem a diagonal das faces");
        assert_eq!(r.bodies[0].mesh.positions.len() / 3, 24, "cada canto com as três normais");
    }

    #[test]
    fn mais_de_duzentos_mil_triangulos_entram_como_corpo() {
        let mut d = Document::new("m");
        let m = placa(330);
        assert!(m.tris.len() > 200_000, "{}", m.tris.len());
        corpo(&mut d, &m);
        let r = recompute(&d);
        sem_erro(&d, &r);
        assert_eq!(r.bodies[0].mesh.indices.len() / 3, m.tris.len());
        assert!(r.bodies[0].mass.volume > 0.);
        // e o documento salvo devolve o mesmo corpo sem o arquivo
        let j = serde_json::to_string(&d).unwrap();
        let v: Document = serde_json::from_str(&j).unwrap();
        assert_eq!(recompute(&v).bodies[0].mesh.indices.len() / 3, m.tris.len());
    }

    #[test]
    fn reparo_fecha_o_furo_pela_arvore() {
        let mut m = caixa([10., 10., 10.], [0.; 3]);
        m.tris.remove(0);
        let mut d = Document::new("m");
        let i = corpo(&mut d, &m);
        let r = recompute(&d);
        assert!(matches!(&r.status[&i], Status::Warning(a) if a.contains("não estanque")), "{:?}", r.status[&i]);
        assert_eq!(r.bodies[0].mass.volume, 0., "casca aberta não tem volume");
        let o = d.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::Reparar(MeshRepair::default()) }));
        let r = recompute(&d);
        sem_erro(&d, &r);
        assert!(matches!(&r.status[&o], Status::Warning(a) if a.contains("1 furos fechados")), "{:?}", r.status[&o]);
        assert!((r.bodies[0].mass.volume - 1000.).abs() < 1e-6);
    }

    #[test]
    fn decimar_e_parametrico_e_o_cache_de_conteudo_nao_mistura() {
        let mut d = Document::new("m");
        corpo(&mut d, &placa(40));
        d.add_param("fracao", "0.25", Unit::Numero).unwrap();
        let n = placa(40).tris.len();
        d.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::Decimar(MeshDecimate { proporcao: "fracao".into(), desvio: "0".into() }) }));
        let r = recompute(&d);
        sem_erro(&d, &r);
        let t = r.bodies[0].mesh.indices.len() / 3;
        assert!(t <= n / 4 + 8 && t > n / 6, "{t} de {n}");
        d.params.iter_mut().for_each(|p| p.expr = "0.5".into());
        let t2 = recompute(&d).bodies[0].mesh.indices.len() / 3;
        assert!(t2 > t, "{t2} e {t}");
    }

    #[test]
    fn booleana_de_malha_com_malha_e_com_solido() {
        let mut d = Document::new("m");
        corpo(&mut d, &caixa([10., 10., 10.], [0.; 3]));
        let c2 = d.add_collector("Ferramenta");
        d.active_collector = c2;
        corpo(&mut d, &caixa([10., 10., 10.], [5., 5., 5.]));
        d.active_collector = d.collectors[0].id;
        d.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::Booleana(MeshBoolean { tool: c2, op: BoolOp::Cut, keep_tool: false }) }));
        let r = recompute(&d);
        sem_erro(&d, &r);
        assert_eq!(r.bodies.len(), 1, "a ferramenta é consumida");
        assert!((r.bodies[0].mass.volume - 875.).abs() < 1e-6, "{}", r.bodies[0].mass.volume);
    }

    #[test]
    fn para_brep_tem_teto_e_de_brep_volta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("m");
        corpo(&mut d, &caixa([10., 10., 10.], [0.; 3]));
        let id = d.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::ParaBrep { solid: true } }));
        let mut cache = Cache::default();
        for _ in 0..2 {
            // a segunda volta reaproveita o sólido e não pode deixar a malha junto
            let r = recompute_cached(&d, &mut cache);
            assert_eq!(r.bodies.len(), 1);
            assert!((r.bodies[0].mass.volume - 1000.).abs() < 1e-6);
            assert_eq!(r.bodies[0].faces, 12);
        }
        assert!(final_shape(&d).is_some_and(|s| s.is_solid()));
        d.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::DeBrep { desvio: "0.01".into(), angulo: "15".into() } }));
        let r = recompute(&d);
        sem_erro(&d, &r);
        assert_eq!(r.bodies.len(), 1);
        assert_eq!(r.bodies[0].faces, 1, "voltou a ser corpo de malha");
        assert!((r.bodies[0].mass.volume - 1000.).abs() < 1e-3);
        assert!(matches!(r.status[&id], Status::Warning(_)));

        let mut g = Document::new("g");
        corpo(&mut g, &placa(330));
        g.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::ParaBrep { solid: true } }));
        let r = recompute(&g);
        assert!(matches!(&r.status[&g.features[1].id], Status::Error(e) if e.contains("decime")), "{:?}", r.status[&g.features[1].id]);
    }

    #[test]
    fn desvio_contra_o_brep_acha_a_folga_conhecida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // corpo B-rep de caixa e, ao lado, a mesma caixa em malha com a tampa 0,5 mm mais alta
        let mut d = Document::new("m");
        let brep = d.active_collector;
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: "10".into(), b: "10".into(), c: "10".into(), d: "0".into(), op: BoolOp::Add }));
        let c2 = d.add_collector("Malha");
        d.active_collector = c2;
        let mut m = caixa([10., 10., 10.], [-5., -5., 0.]);
        m.pos.iter_mut().filter(|p| p[2] > 5.).for_each(|p| p[2] += 0.5);
        corpo(&mut d, &m);
        let r = desvio_malha_brep(&d, c2, brep).unwrap();
        assert!((r.ida.maximo - 0.5).abs() < 1e-3, "{r:?}");
        assert!(r.incerteza < 0.01);
    }

    #[test]
    fn stl_do_documento_leva_o_corpo_de_malha() {
        let dir = std::env::temp_dir().join(format!("fenkra corpo malha {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let alvo = dir.join("saída.stl");
        let mut d = Document::new("m");
        corpo(&mut d, &caixa([10., 10., 10.], [0.; 3]));
        export_com_preset(&d, None, alvo.to_str().unwrap(), PresetStl::Normal).unwrap();
        let cena = fenkra_malha::ler_arquivo(&alvo).unwrap();
        assert!((cena.no_lugar()[0].soldada().volume() - 1000.).abs() < 1e-3);
        let e = export_com_preset(&d, None, dir.join("x.step").to_str().unwrap(), PresetStl::Normal).unwrap_err();
        assert!(e.contains("sólido"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn arquivo_mudado_em_disco_entra_de_novo() {
        let dir = std::env::temp_dir().join(format!("fenkra corpo disco {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("c.stl");
        let grava = |l: f64| {
            let mut c = fenkra_malha::Cena::default();
            c.raiz_com(caixa([l, l, l], [0.; 3]));
            std::fs::write(&p, fenkra_malha::stl::gravar(&c, true, 1.)).unwrap();
        };
        grava(10.);
        let mut d = Document::new("m");
        d.add_feature(FeatureKind::MeshImport(MeshImport { path: p.to_string_lossy().into(), solid: false, corpo: true, dados: String::new() }));
        let mut cache = Cache::default();
        assert!((recompute_cached(&d, &mut cache).bodies[0].mass.volume - 1000.).abs() < 1e-6);
        grava(20.);
        assert!((recompute_cached(&d, &mut cache).bodies[0].mass.volume - 8000.).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
