// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pacote de visualização `.fkv` (B181): o formato que o visualizador web (B338) abre sem kernel.
//!
//! O navegador não roda o OpenCASCADE, então o arquivo nativo do Fenkra não abre sozinho na web. O
//! Fenkra gera, a partir da peça, da montagem ou do desenho, um pacote só de leitura com tudo o que
//! se precisa para ver: malha glTF, árvore com IDs estáveis, atributos, PMI com alvos, capturas,
//! estados de exibição e folhas 2D em vetor. O pacote é um ZIP com `pacote.json`, `cena.glb` e
//! `folhas/*.json`; a especificação está em `docs/formatos/pacote-visualizacao.md`.
//!
//! A geração é função pura do documento: o mesmo documento dá o mesmo pacote, byte a byte.

mod analise;
mod folhas;
mod glb;
mod pagina;
mod pontos;
mod tipos;

pub use pagina::{pagina_html, pagina_html_completa};

use fenkra_core::pmi::Pmi;
use fenkra_core::Document;
use fenkra_desenho::source::Models;
use fenkra_desenho::Drawing;
use fenkra_malha::decimar::OpcoesDecimar;
use fenkra_malha::Malha;
use fenkra_montagem::{Assembly, NodeKind, Placement};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Versão do formato. Campo novo não muda o número; mudar ou tirar campo muda.
pub const VERSAO: u32 = 1;
pub const EXTENSAO: &str = "fkv";
pub const NOME_FORMATO: &str = "fenkra-visualizacao";

#[derive(Clone, Debug)]
pub struct Opcoes {
    /// fração dos triângulos que fica (0 a 1); `None` mantém a malha inteira. Decimar protege a
    /// propriedade intelectual e reduz o tamanho; os pontos de captura exatos saem junto, porque
    /// entregariam a geometria exata
    pub decimar: Option<f64>,
    /// grava os pontos de captura exatos (centros, raios, pontas e pontos médios)
    pub capturas: bool,
    /// calcula interferências e folgas da montagem e as grava no manifesto (B339); a página só lista
    pub analise: bool,
    /// progresso e cancelamento (B427): quando a geração roda numa thread, a interface lê os passos
    /// feitos e pode pedir para parar; cancelado, devolve `Err("cancelado")` sem pacote pela metade
    pub controle: Option<fenkra_core::trabalho::Controle>,
}

impl Opcoes {
    fn cancelado(&self) -> bool {
        self.controle.as_ref().is_some_and(|c| c.cancelado())
    }
    fn avancar(&self) {
        if let Some(c) = &self.controle {
            c.avancar();
        }
    }
}

/// Texto do erro de quem foi cancelado: a interface o reconhece e não mostra como falha.
pub const CANCELADO: &str = "cancelado";

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes { decimar: None, capturas: true, analise: true, controle: None }
    }
}

/// O pacote em memória: o manifesto e os arquivos que vão no ZIP, fora o próprio manifesto.
#[derive(Clone, Debug)]
pub struct Pacote {
    pub manifesto: Value,
    pub arquivos: Vec<(String, Vec<u8>)>,
}

fn r6(x: f64) -> f64 {
    let v = (x * 1e6).round() / 1e6;
    if v == 0. {
        0.
    } else {
        v
    }
}

fn matriz(p: &Placement) -> [f64; 12] {
    let (x, y, z) = (p.dir([1., 0., 0.]), p.dir([0., 1., 0.]), p.dir([0., 0., 1.]));
    let o = p.origin;
    [x[0], y[0], z[0], o[0], x[1], y[1], z[1], o[1], x[2], y[2], z[2], o[2]]
}

/// Uma peça calculada: o que o pacote precisa saber dela, guardado uma vez por arquivo.
struct Peca {
    doc: Document,
    corpos: Vec<Corpo>,
    massa_kg: f64,
    volume: f64,
    area: f64,
    caixa: Option<([f64; 3], [f64; 3])>,
    centro: Option<[f64; 3]>,
}

struct Corpo {
    id: u64,
    nome: String,
    cor: [u8; 3],
    malha_glb: usize,
    malha_id: String,
}

/// Por que uma peça não vira malha. Na montagem, nenhum dos dois derruba o pacote.
enum Falta {
    /// sem corpo e sem erro: peça de referência, como o mestre. Fica de fora sem aviso
    Esqueleto(String),
    /// a peça devia ter corpo: comando em erro, arquivo sumido ou ilegível
    Erro(String),
}

impl Falta {
    fn texto(self) -> String {
        match self {
            Falta::Esqueleto(nome) => format!("'{nome}' não tem corpo visível para mostrar"),
            Falta::Erro(e) => e,
        }
    }
}

struct Gerador<'a> {
    opc: &'a Opcoes,
    glb: glb::Construtor,
    malhas: Vec<Value>,
    arvore: Vec<Value>,
    pecas: HashMap<PathBuf, Rc<Peca>>,
    /// peças pelo conteúdo: dois arquivos iguais dividem a malha
    por_conteudo: HashMap<u64, Rc<Peca>>,
    avisos: Vec<String>,
    instancias: usize,
    triangulos: usize,
}

impl<'a> Gerador<'a> {
    fn novo(opc: &'a Opcoes) -> Self {
        Gerador { opc, glb: Default::default(), malhas: vec![], arvore: vec![], pecas: HashMap::new(), por_conteudo: HashMap::new(), avisos: vec![], instancias: 0, triangulos: 0 }
    }

    /// Calcula a peça e põe as malhas dos corpos no GLB, uma vez por chave.
    fn peca(&mut self, chave: &Path, doc: Document) -> Result<Rc<Peca>, Falta> {
        if let Some(p) = self.pecas.get(chave) {
            return Ok(p.clone());
        }
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            doc.to_json().hash(&mut h);
            h.finish()
        };
        if let Some(p) = self.por_conteudo.get(&hash) {
            self.pecas.insert(chave.to_path_buf(), p.clone());
            return Ok(p.clone());
        }
        // um cálculo só: a massa sai dos corpos do cache deste mesmo recompute (antes eram dois)
        let mut cache = fenkra_solidos::Cache::default();
        let r = fenkra_solidos::recompute_cached(&doc, &mut cache);
        let massa = doc
            .efetivo()
            .ok()
            .and_then(|ef| fenkra_solidos::forma_cache::corpos_do_cache(&ef, &cache).map(|c| fenkra_solidos::massa::relatorio_massa_de_corpos(&ef, &c)))
            .unwrap_or_else(|| fenkra_solidos::massa::relatorio_massa(&doc));
        let (mut corpos, mut lo, mut hi) = (vec![], [f64::MAX; 3], [f64::MIN; 3]);
        let (mut volume, mut area) = (0., 0.);
        let base = self.malhas.len();
        for b in &r.bodies {
            let m = &b.mesh;
            if m.indices.is_empty() {
                continue;
            }
            let (pos, nrm, idx, exato) = match self.opc.decimar {
                Some(f) if f < 1. => {
                    let (p, i) = decimar(&m.positions, &m.indices, f);
                    (p, vec![], i, false)
                }
                _ => (m.positions.clone(), m.normals.clone(), m.indices.clone(), true),
            };
            for p in pos.chunks_exact(3) {
                for i in 0..3 {
                    lo[i] = lo[i].min(f64::from(p[i]));
                    hi[i] = hi[i].max(f64::from(p[i]));
                }
            }
            volume += b.mass.volume;
            area += b.mass.area;
            let nome = doc.collectors.iter().find(|c| c.id == b.collector).map_or_else(|| format!("Corpo {}", corpos.len() + 1), |c| c.name.clone());
            let id_malha = format!("m{}", base + corpos.len());
            let mut meta = json!({"id": id_malha, "nome": nome, "triangulos": idx.len() / 3, "vertices": pos.len() / 3});
            if exato && !self.opc.decimar.is_some() {
                meta["faces"] = json!(m.face_ranges);
                meta["tipos_de_face"] = json!(m.face_info.iter().map(|f| [f64::from(f.kind), r6(f.area)]).collect::<Vec<_>>());
                if self.opc.capturas {
                    meta["capturas"] = pontos::capturas(m);
                }
            }
            self.triangulos += idx.len() / 3;
            let g = self.glb.malha(&nome, &pos, &nrm, &idx, b.color, json!({"fkv": id_malha}));
            self.malhas.push(meta);
            corpos.push(Corpo { id: b.collector.0, nome, cor: b.color, malha_glb: g, malha_id: id_malha });
        }
        if corpos.is_empty() {
            let erro = r.status.values().find_map(|s| if let fenkra_solidos::Status::Error(m) = s { Some(m.clone()) } else { None });
            return Err(erro.map_or_else(|| Falta::Esqueleto(doc.name.clone()), |m| Falta::Erro(format!("comando em erro: {m}"))));
        }
        let p = Rc::new(Peca {
            centro: massa.total.as_ref().map(|t| t.centro),
            massa_kg: massa.massa_total_kg(),
            volume,
            area,
            caixa: Some((lo, hi)),
            doc,
            corpos,
        });
        self.pecas.insert(chave.to_path_buf(), p.clone());
        self.por_conteudo.insert(hash, p.clone());
        Ok(p)
    }

    fn atributos(p: &Peca, nome: &str) -> Value {
        let pr = &p.doc.props;
        let (lo, hi) = p.caixa.unwrap_or_default();
        let dims = [0, 1, 2].map(|i| r6(hi[i] - lo[i]));
        let mut v = json!({
            "nome": nome, "pn": pr.pn, "descricao": pr.description, "material": pr.material,
            "tipo_de_peca": format!("{:?}", pr.part_type),
            "massa_kg": r6(p.massa_kg), "volume_mm3": r6(p.volume), "area_mm2": r6(p.area),
            "caixa_mm": [lo.map(r6), hi.map(r6)],
            "dimensoes_mm": dims,
            "centro_de_massa_mm": p.centro.map(|c| c.map(r6)),
        });
        if !pr.custom.is_empty() {
            v["personalizados"] = pr.custom.iter().map(|(k, x)| (k.clone(), json!(x))).collect::<serde_json::Map<_, _>>().into();
        }
        v
    }

    /// Nó de peça e nós dos corpos, com as entradas na árvore. Devolve o nó GLB da peça.
    fn instanciar(&mut self, id: &str, nome: &str, pai: Option<&str>, p: &Peca, m: Option<[f64; 12]>, cor: Option<[u8; 3]>, visivel: bool) -> usize {
        self.instancias += 1;
        let no = self.glb.no(nome, id, m, &[]);
        let mut e = json!({"id": id, "pai": pai, "nome": nome, "tipo": "peca", "no": no, "malha": null, "atributos": Self::atributos(p, nome), "visivel": visivel});
        if let Some(c) = cor {
            e["cor"] = json!(c);
        }
        self.arvore.push(e);
        for c in &p.corpos {
            let cid = format!("{id}/c{}", c.id);
            let nc = self.glb.no(&c.nome, &cid, None, &[c.malha_glb]);
            self.glb.filho(no, nc);
            self.arvore.push(json!({"id": cid, "pai": id, "nome": c.nome, "tipo": "corpo", "no": nc, "malha": c.malha_id, "cor": c.cor, "visivel": true}));
        }
        no
    }

    #[allow(clippy::too_many_arguments)]
    fn descer(&mut self, asm: &Assembly, pasta: &Path, pai: Option<fenkra_core::Id>, id_pai: Option<&str>, no_pai: Option<usize>, mundo: &Placement, prefixo: &str, pilha: &mut Vec<PathBuf>, topo: &mut Vec<usize>) -> Result<f64, String> {
        let mut massa = 0.;
        let filhos: Vec<_> = asm.nodes.iter().filter(|n| n.parent == pai).cloned().collect();
        for n in filhos {
            if self.opc.cancelado() {
                return Err(CANCELADO.into());
            }
            self.opc.avancar();
            let id = format!("{prefixo}n{}", n.id.0);
            let lugar = mundo.then(&n.placement);
            let (no, m) = match &n.kind {
                NodeKind::Part { path } => {
                    let cheio = pasta.join(path);
                    let lida = std::fs::read_to_string(&cheio).map_err(|e| e.to_string()).and_then(|t| Document::from_json(&t).map_err(|e| e.to_string()));
                    let peca = match lida.map_err(|e| Falta::Erro(format!("{}: {e}", cheio.display()))).and_then(|doc| self.peca(&cheio, doc)) {
                        Ok(p) => p,
                        Err(Falta::Esqueleto(_)) => continue,
                        Err(Falta::Erro(motivo)) => {
                            self.avisos.push(format!("'{}' ficou de fora: {motivo}", n.name));
                            continue;
                        }
                    };
                    massa += peca.massa_kg;
                    let no = self.instanciar(&id, &n.name, id_pai, &peca, Some(matriz(&lugar)), n.color, n.visible);
                    (no, peca.massa_kg)
                }
                NodeKind::Group => {
                    let no = self.glb.no(&n.name, &id, None, &[]);
                    let pos = self.arvore.len();
                    self.arvore.push(json!({"id": id, "pai": id_pai, "nome": n.name, "tipo": "grupo", "no": no, "malha": null, "visivel": n.visible}));
                    let m = self.descer(asm, pasta, Some(n.id), Some(&id), Some(no), mundo, prefixo, pilha, &mut vec![])?;
                    self.arvore[pos]["atributos"] = json!({"nome": n.name, "massa_kg": r6(m)});
                    (no, m)
                }
                NodeKind::SubAssembly { path } => {
                    let cheio = pasta.join(path);
                    if pilha.contains(&cheio) {
                        return Err(format!("subconjunto '{}' contém a si mesmo", cheio.display()));
                    }
                    let lido = std::fs::read_to_string(&cheio).map_err(|e| e.to_string()).and_then(|t| Assembly::from_json(&t));
                    let sub = match lido {
                        Ok(mut s) => {
                            fenkra_montagem::flexivel::aplicar(&mut s, &asm.flex_de(n.id));
                            s
                        }
                        Err(e) => {
                            self.avisos.push(format!("'{}' ficou de fora: {}: {e}", n.name, cheio.display()));
                            continue;
                        }
                    };
                    if let Some(c) = &self.opc.controle {
                        c.somar_total(sub.nodes.len() as u32);
                    }
                    let no = self.glb.no(&n.name, &id, None, &[]);
                    let pos = self.arvore.len();
                    self.arvore.push(json!({"id": id, "pai": id_pai, "nome": n.name, "tipo": "montagem", "no": no, "malha": null, "visivel": n.visible}));
                    pilha.push(cheio.clone());
                    let prefixo_sub = format!("{id}/");
                    let m = self.descer(&sub, cheio.parent().unwrap_or(pasta), None, Some(&id), Some(no), &lugar, &prefixo_sub, pilha, &mut vec![])?;
                    pilha.pop();
                    self.arvore[pos]["atributos"] = json!({"nome": n.name, "pn": sub.props.pn, "descricao": sub.props.description, "massa_kg": r6(m)});
                    (no, m)
                }
            };
            // a massa da peça já entrou; a do grupo e a do subconjunto entram aqui
            if !matches!(n.kind, NodeKind::Part { .. }) {
                massa += m;
            }
            // os nós dos filhos já foram ligados à raiz; aqui só a hierarquia da cena
            match no_pai {
                Some(p) => self.glb.filho(p, no),
                None => topo.push(no),
            }
        }
        Ok(massa)
    }
}

/// Malha decimada: posições e índices.
fn decimar(pos: &[f32], idx: &[u32], fracao: f64) -> (Vec<f32>, Vec<u32>) {
    let m = Malha {
        pos: pos.chunks_exact(3).map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]).collect(),
        tris: idx.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect(),
        ..Default::default()
    };
    let d = m.decimada(&OpcoesDecimar { proporcao: fracao.clamp(0.01, 1.), desvio: 0. });
    (d.pos.iter().flat_map(|p| p.map(|v| v as f32)).collect(), d.tris.iter().flatten().copied().collect())
}

fn json_pmi(lista: &[Pmi], no: Option<&str>, malha: Option<&fenkra_solidos::RecomputeResult>, campos: &std::collections::HashMap<String, String>) -> Vec<Value> {
    lista
        .iter()
        .map(|p| {
            let valor = p.measure(&p.anchors);
            json!({
                "id": format!("p{}", p.id.0), "nome": p.name, "tipo": p.kind.label(), "dimensional": p.kind.is_dimension(),
                "texto": texto_pmi(p, valor, campos), "valor": valor.map(r6),
                "ancoras": p.anchors.iter().map(|a| a.map(r6)).collect::<Vec<_>>(),
                "deslocamento": p.offset.map(r6),
                "alvos": p.targets.iter().map(|t| t.as_ref().map(tipos::alvo)).collect::<Vec<_>>(),
                "no": no, "visivel": p.visible, "pasta": p.folder,
                "setas": setas(p),
                "cordao": cordao(p, malha),
            })
        })
        .collect()
}

/// Texto da anotação como aparece no aplicativo: a legenda 3D (carimbo) tem os campos `{nome}`,
/// `{codigo}`, `{massa}`… trocados pelos dados da peça; sem isso o visualizador mostrava o modelo cru.
fn texto_pmi(p: &Pmi, valor: Option<f64>, campos: &std::collections::HashMap<String, String>) -> String {
    match &p.kind {
        fenkra_core::pmi::PmiKind::TitleBlock { template } => fenkra_desenho::resolve(template, campos),
        _ => p.display(valor),
    }
}

/// Campos da legenda 3D de uma peça (os mesmos do aplicativo).
fn campos_da_legenda(doc: &Document, massa_kg: f64, volume_mm3: f64) -> std::collections::HashMap<String, String> {
    let mut f = std::collections::HashMap::new();
    f.insert("data".to_string(), fenkra_desenho::today());
    f.insert("nome".into(), doc.name.clone());
    f.insert("codigo".into(), doc.props.pn.clone());
    f.insert("descricao".into(), if doc.props.description.is_empty() { doc.name.clone() } else { doc.props.description.clone() });
    f.insert("material".into(), doc.props.material.clone());
    f.insert("massa".into(), format!("{massa_kg:.3} kg").replace('.', ","));
    f.insert("volume".into(), format!("{:.1} cm³", volume_mm3 / 1000.).replace('.', ","));
    for (k, v) in &doc.props.custom {
        f.insert(k.to_lowercase(), v.clone());
    }
    f
}

/// Cordão de solda para exibir (B369): a perna em mm e os segmentos da aresta marcada. É só
/// exibição; não entra em massa, volume nem lista de corte. `visivel` segue a escolha do painel.
fn cordao(p: &Pmi, r: Option<&fenkra_solidos::RecomputeResult>) -> Value {
    use fenkra_core::pmi::PmiKind;
    let (PmiKind::Weld { size, extra, .. }, Some(r)) = (&p.kind, r) else { return Value::Null };
    let (Some(perna), Some(q)) = (fenkra_core::pmi::weld_leg_mm(size), p.anchors.first()) else { return Value::Null };
    let perto = |v: &[f32]| (0..3).all(|i| (f64::from(v[i]) - q[i]).abs() < 1e-3);
    for b in &r.bodies {
        let m = &b.mesh;
        if let Some(e) = m.edge_mids.chunks_exact(3).position(perto) {
            let Some(&s0) = m.edge_ranges.get(e) else { continue };
            let s1 = m.edge_ranges.get(e + 1).map_or(m.edges.len() / 6, |x| *x as usize);
            let segs: Vec<Value> = m.edges.chunks_exact(6).take(s1).skip(s0 as usize).map(|s| json!([[r6(f64::from(s[0])), r6(f64::from(s[1])), r6(f64::from(s[2]))], [r6(f64::from(s[3])), r6(f64::from(s[4])), r6(f64::from(s[5]))]])).collect();
            return json!({"perna_mm": r6(perna), "visivel": !extra.hide_bead, "segmentos": segs});
        }
    }
    Value::Null
}

/// Uma seta por âncora, com o lado da nota de onde sai (0 automático, 1 esquerda, 2 direita, 3 topo,
/// 4 base), a ponta e os vértices intermediários. É o que o visualizador desenha na borda do texto.
fn setas(p: &Pmi) -> Vec<Value> {
    use fenkra_core::pmi::PmiArrow;
    use fenkra_core::pmi_leader as pl;
    let tem = matches!(p.kind, fenkra_core::pmi::PmiKind::Note | fenkra_core::pmi::PmiKind::FeatureNote { .. });
    if !tem {
        return vec![];
    }
    p.anchors
        .iter()
        .enumerate()
        .map(|(k, a)| {
            let ponta = match pl::arrow(p, k) {
                PmiArrow::Padrao | PmiArrow::Cheia => "cheia",
                PmiArrow::Aberta => "aberta",
                PmiArrow::Ponto => "ponto",
                PmiArrow::Barra => "barra",
                PmiArrow::Nenhuma => "nenhuma",
            };
            let vias: Vec<Value> = p.style.leader_via.get(k).map(|v| v.iter().map(|q| json!(q.map(r6))).collect()).unwrap_or_default();
            json!({"ancora": a.map(r6), "lado": pl::side(p, k), "ponta": ponta, "vias": vias})
        })
        .collect()
}

fn json_capturas(c: &[fenkra_core::pmi::PmiCapture]) -> Vec<Value> {
    c.iter()
        .map(|c| json!({"id": format!("k{}", c.id.0), "nome": c.name, "camera": tipos::camera(&c.view), "pmi": c.shown.iter().map(|i| format!("p{}", i.0)).collect::<Vec<_>>(), "ocultos": c.hidden.iter().map(|i| i.0).collect::<Vec<_>>(), "com_modelo": c.with_model}))
        .collect()
}

fn terminar(g: Gerador, mut m: Value, raizes: &[usize], nome: &str) -> Pacote {
    m["malhas"] = json!(g.malhas);
    m["arvore"] = json!(g.arvore);
    m["avisos"] = json!(g.avisos);
    m["estatisticas"] = json!({"pecas_unicas": g.por_conteudo.len(), "instancias": g.instancias, "triangulos": g.triangulos});
    m["decimado"] = json!(g.opc.decimar);
    let glb = g.glb.finalizar(raizes, nome);
    m["cena"] = json!("cena.glb");
    Pacote { manifesto: m, arquivos: vec![("cena.glb".into(), glb)] }
}

fn base(tipo: &str, nome: &str) -> Value {
    json!({
        "formato": NOME_FORMATO, "versao": VERSAO, "gerador": format!("Fenkra {}", env!("CARGO_PKG_VERSION")),
        "tipo": tipo, "nome": nome, "unidade": "mm", "eixo_para_cima": "Z",
        "malhas": [], "arvore": [], "pmi": [], "capturas": [], "vistas": [], "estados": [], "folhas": [],
    })
}

/// Pacote de uma peça.
pub fn de_peca(doc: &Document, opc: &Opcoes) -> Result<Pacote, String> {
    let mut g = Gerador::novo(opc);
    let peca = g.peca(Path::new("<peça>"), doc.clone()).map_err(Falta::texto)?;
    let raiz = g.instanciar("peca", &doc.name, None, &peca, None, None, true);
    let mut m = base("peca", &doc.name);
    m["atributos"] = Gerador::atributos(&peca, &doc.name);
    // o cordão de solda precisa da aresta marcada: só recalcula quando há solda com medida
    let r = doc.pmi.iter().any(|p| matches!(p.kind, fenkra_core::pmi::PmiKind::Weld { .. })).then(|| fenkra_solidos::recompute(doc));
    let campos = campos_da_legenda(doc, peca.massa_kg, peca.volume);
    m["pmi"] = json!(json_pmi(&doc.pmi, Some("peca"), r.as_ref(), &campos));
    m["capturas"] = json!(json_capturas(&doc.pmi_captures));
    m["vistas"] = json!(doc.views.iter().map(|v| json!({"nome": v.name, "camera": tipos::camera(v)})).collect::<Vec<_>>());
    Ok(terminar(g, m, &[raiz], &doc.name))
}

/// Pacote de uma montagem. `pasta` é a pasta do `.fka`: os caminhos dos componentes são relativos a ela.
pub fn de_montagem(asm: &Assembly, pasta: &Path, opc: &Opcoes) -> Result<Pacote, String> {
    let mut g = Gerador::novo(opc);
    if let Some(c) = &opc.controle {
        // um passo por componente (os subconjuntos somam os seus ao abrir) e dois para análise e fecho
        c.definir_total(asm.nodes.len() as u32 + 2);
    }
    let mut topo = vec![];
    let massa = g.descer(asm, pasta, None, None, None, &Placement::default(), "", &mut vec![], &mut topo)?;
    if g.instancias == 0 {
        return Err(format!("a montagem '{}' não tem peças com corpo visível", asm.name));
    }
    let mut m = base("montagem", &asm.name);
    m["atributos"] = json!({"nome": asm.name, "pn": asm.props.pn, "descricao": asm.props.description, "massa_kg": r6(massa)});
    let mut campos = std::collections::HashMap::new();
    campos.insert("data".to_string(), fenkra_desenho::today());
    campos.insert("nome".into(), asm.name.clone());
    campos.insert("codigo".into(), asm.props.pn.clone());
    campos.insert("descricao".into(), if asm.props.description.is_empty() { asm.name.clone() } else { asm.props.description.clone() });
    campos.insert("massa".into(), format!("{massa:.3} kg").replace('.', ","));
    m["pmi"] = json!(json_pmi(&asm.pmi, None, None, &campos));
    m["capturas"] = json!(json_capturas(&asm.pmi_captures));
    m["estados"] = json!(asm.display_states.iter().map(tipos::estado).collect::<Vec<_>>());
    if opc.cancelado() {
        return Err(CANCELADO.into());
    }
    if opc.analise {
        m["analise"] = analise::analisar_montagem(asm, pasta);
    }
    opc.avancar();
    if opc.cancelado() {
        return Err(CANCELADO.into());
    }
    m["lista"] = json!(fenkra_montagem::bom::bom(asm, Some(pasta), fenkra_montagem::bom::BomLevel::Pecas)
        .iter()
        .map(|r| json!({"item": r.item, "quantidade": r.qty, "nome": r.name, "arquivo": r.key}))
        .collect::<Vec<_>>());
    Ok(terminar(g, m, &topo, &asm.name))
}

/// Pacote de um desenho: só as folhas, em vetor.
pub fn de_desenho(d: &Drawing, pasta: &Path, _opc: &Opcoes) -> Result<Pacote, String> {
    let mut modelos = Models::new(Some(pasta.to_path_buf()));
    let mut arquivos = vec![];
    let mut lista = vec![];
    for (i, s) in d.sheets.iter().enumerate() {
        let f = folhas::folha(d, s.id, &mut modelos).ok_or("folha não encontrada")?;
        let nome = format!("folhas/f{i}.json");
        lista.push(json!({"id": s.id.0, "nome": s.name, "arquivo": nome, "largura": f["largura"], "altura": f["altura"], "formato": f["formato"]}));
        arquivos.push((nome, serde_json::to_vec(&f).map_err(|e| e.to_string())?));
    }
    if lista.is_empty() {
        return Err("o desenho não tem folhas".into());
    }
    let mut m = base("desenho", &d.name);
    m["folhas"] = json!(lista);
    m["tabelas"] = json!(folhas::tabelas(d, &mut modelos));
    let campos = d.sheets.first().map(|s| fenkra_desenho::layout::sheet_fields(d, s, &mut modelos)).unwrap_or_default();
    m["atributos"] = json!({"nome": d.name, "titulo": fenkra_desenho::resolve(&d.title.title, &campos), "pn": fenkra_desenho::resolve(&d.title.pn, &campos), "material": fenkra_desenho::resolve(&d.title.material, &campos), "revisao": d.title.revision, "projecao": format!("{:?}", d.projection)});
    m["estatisticas"] = json!({"folhas": d.sheets.len(), "vistas": d.views.len(), "cotas": d.dims.len()});
    Ok(Pacote { manifesto: m, arquivos })
}

/// Abre o arquivo pelo sufixo: `.fkp` (peça), `.fka` (montagem) ou `.fkd` (desenho).
pub fn de_arquivo(caminho: &Path, opc: &Opcoes) -> Result<Pacote, String> {
    let txt = std::fs::read_to_string(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    let pasta = caminho.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    match caminho.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
        Some("fkp") => de_peca(&Document::from_json(&txt).map_err(|e| e.to_string())?, opc),
        Some("fka") => de_montagem(&Assembly::from_json(&txt)?, pasta, opc),
        Some("fkd") => de_desenho(&Drawing::from_json(&txt)?, pasta, opc),
        _ => Err("use um arquivo .fkp (peça), .fka (montagem) ou .fkd (desenho)".into()),
    }
}

impl Pacote {
    /// O que ficou de fora do pacote, uma frase por item.
    pub fn avisos(&self) -> Vec<String> {
        self.manifesto["avisos"].as_array().map_or_else(Vec::new, |l| l.iter().filter_map(|a| a.as_str().map(String::from)).collect())
    }

    /// O ZIP do `.fkv`: o manifesto primeiro, depois cena e folhas, sempre na mesma ordem.
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        let mj = serde_json::to_vec(&self.manifesto).map_err(|e| e.to_string())?;
        let mut e: Vec<(&str, &[u8])> = vec![("pacote.json", &mj)];
        e.extend(self.arquivos.iter().map(|(n, b)| (n.as_str(), b.as_slice())));
        fenkra_malha::zip::gravar(&e, false).map_err(|e| e.to_string())
    }

    pub fn gravar(&self, caminho: &Path) -> Result<usize, String> {
        let b = self.bytes()?;
        std::fs::write(caminho, &b).map_err(|e| format!("não foi possível gravar '{}': {e}", caminho.display()))?;
        Ok(b.len())
    }

    /// Lê um `.fkv`: o leitor de referência em Rust, usado pelo teste de conformidade.
    pub fn ler(bytes: &[u8]) -> Result<Pacote, String> {
        let mj = fenkra_malha::zip::abrir(bytes, |n| n == "pacote.json").map_err(|e| e.to_string())?.ok_or("o pacote não tem pacote.json")?;
        let manifesto: Value = serde_json::from_slice(&mj).map_err(|e| format!("pacote.json: {e}"))?;
        let mut arquivos = vec![];
        for n in fenkra_malha::zip::nomes(bytes) {
            if n == "pacote.json" || n.ends_with('/') {
                continue;
            }
            let b = fenkra_malha::zip::abrir(bytes, |x| x == n).map_err(|e| e.to_string())?.unwrap_or_default();
            arquivos.push((n, b));
        }
        Ok(Pacote { manifesto, arquivos })
    }

    /// Confere o que todo leitor precisa: versão, referências da árvore, cena e folhas.
    pub fn validar(&self) -> Result<(), String> {
        let m = &self.manifesto;
        if m["formato"] != NOME_FORMATO {
            return Err("não é um pacote de visualização do Fenkra".into());
        }
        let v = m["versao"].as_u64().ok_or("sem versão")?;
        if v > u64::from(VERSAO) {
            return Err(format!("pacote da versão {v}; este leitor entende até a {VERSAO}"));
        }
        let arq = |n: &str| self.arquivos.iter().find(|a| a.0 == n).map(|a| &a.1);
        let mut nos = 0;
        if let Some(c) = m["cena"].as_str() {
            let (j, _) = glb::ler_json(arq(c).ok_or("a cena apontada não está no pacote")?)?;
            nos = j["nodes"].as_array().map_or(0, Vec::len);
            let malhas = j["meshes"].as_array().map_or(0, Vec::len);
            let declaradas = m["malhas"].as_array().map_or(0, Vec::len);
            if malhas != declaradas {
                return Err(format!("o GLB tem {malhas} malhas e o manifesto declara {declaradas}"));
            }
        }
        let mut ids = std::collections::HashSet::new();
        for e in m["arvore"].as_array().into_iter().flatten() {
            let id = e["id"].as_str().ok_or("entrada da árvore sem id")?;
            if !ids.insert(id) {
                return Err(format!("id repetido na árvore: {id}"));
            }
            if e["no"].as_u64().is_none_or(|n| n as usize >= nos) {
                return Err(format!("o nó de '{id}' não existe na cena"));
            }
        }
        for e in m["arvore"].as_array().into_iter().flatten() {
            if let Some(p) = e["pai"].as_str() {
                if !ids.contains(p) {
                    return Err(format!("pai '{p}' não existe"));
                }
            }
        }
        for f in m["folhas"].as_array().into_iter().flatten() {
            let n = f["arquivo"].as_str().ok_or("folha sem arquivo")?;
            let j: Value = serde_json::from_slice(arq(n).ok_or_else(|| format!("folha '{n}' ausente"))?).map_err(|e| e.to_string())?;
            if j["itens"].as_array().is_none() {
                return Err(format!("a folha '{n}' não tem itens"));
            }
        }
        Ok(())
    }
}
