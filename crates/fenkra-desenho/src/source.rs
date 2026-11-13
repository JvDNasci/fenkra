// SPDX-License-Identifier: LGPL-2.1-or-later
//! Origem das vistas: lê peças/montagens do disco (com cache por data de modificação), monta a
//! forma e calcula as projeções (HLR), cortes, detalhes e os dados das tabelas.

use crate::*;
use fenkra_core::Document;
use fenkra_kernel::Shape;
use fenkra_montagem::load::{full_path, PartCache};
use fenkra_montagem::view::{compose, placed_shape, shown};
use fenkra_montagem::{Assembly, NodeKind};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub enum Model {
    Part(Box<Document>),
    Asm(Box<Assembly>),
}

/// Peça ou montagem carregada.
pub struct Loaded {
    pub model: Model,
    /// forma no mundo (montagem: peças posicionadas, com o estado de exibição)
    pub shape: Option<Shape>,
    /// montagem: cada peça posicionada (para hachurar cada uma com o seu ângulo no corte)
    pub pieces: Vec<Shape>,
    /// nome de cada peça de `pieces` (para marcar as que o corte não atravessa)
    pub piece_names: Vec<String>,
    /// linhas de explosão do estado de exibição, no mundo
    pub linhas: Vec<Vec<V3>>,
    /// arquivos de que depende (para saber quando recarregar)
    files: Vec<(PathBuf, Option<SystemTime>)>,
    pub error: Option<String>,
}

/// Eixos de uma vista: direção para o observador e eixos x/y da folha (no mundo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axes {
    pub toward: V3,
    pub x: V3,
    pub y: V3,
}

impl Axes {
    pub fn new(toward: V3, x: V3) -> Self {
        Axes { toward, x, y: cross(toward, x) }
    }
    pub fn of(d: Dir) -> Self {
        let (t, x) = d.axes();
        Axes::new(t, x)
    }
    /// Ponto do modelo nas coordenadas da vista.
    pub fn project(&self, p: V3) -> V2 {
        [dot(p, self.x), dot(p, self.y)]
    }
    /// Ponto da vista (profundidade 0) de volta ao mundo.
    pub fn unproject(&self, q: V2) -> V3 {
        [0, 1, 2].map(|k| self.x[k] * q[0] + self.y[k] * q[1])
    }
}

/// Projeção pronta de uma vista, em mm do modelo no plano da vista.
#[derive(Clone, Debug, Default)]
pub struct Proj {
    /// polilinhas e tipo: 0 visível, 1 tangente, 2 contorno, 3 oculta, 4 borda do detalhe, 7 linha de
    /// explosão
    pub lines: Vec<(Vec<V2>, u8)>,
    /// contornos cortados (hachura)
    pub hatch: Vec<Vec<V2>>,
    /// peça a que cada contorno de `hatch` pertence (0, 1, 2… só entre as peças cortadas): no corte
    /// de montagem cada peça recebe o seu ângulo
    pub hatch_group: Vec<usize>,
    /// x (na vista) das junções entre segmentos de um corte em degraus: o contorno da hachura não é
    /// traçado sobre elas (B106)
    pub seams: Vec<f64>,
    /// triângulos sombreados (do fundo para a frente) e o tom de cinza
    pub shade: Vec<([V2; 3], u8)>,
    /// textos presos à vista (ex.: "↑ 90° R2" nas linhas de dobra da planificada)
    pub labels: Vec<(V2, String)>,
    /// ponto da vista que fica em `View::pos` (centro do modelo — mantém vistas alinhadas)
    pub center: V2,
    pub lo: V2,
    pub hi: V2,
    pub axes: Option<Axes>,
    pub error: Option<String>,
    /// B427: a projeção está sendo calculada em segundo plano; isto é só o lugar dela
    pub calculando: bool,
    /// B393: linhas tiradas da malha (prévia rápida); a projeção exata ainda vai substituí-las
    pub previa: bool,
}

impl Proj {
    fn bounds(&mut self) {
        let mut lo = [f64::MAX; 2];
        let mut hi = [f64::MIN; 2];
        for (pl, _) in &self.lines {
            for p in pl {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        if lo[0] > hi[0] {
            lo = self.center;
            hi = self.center;
        }
        self.lo = lo;
        self.hi = hi;
    }
}

/// Linha genérica de tabela.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Cache de peças, montagens e projeções de um desenho.
#[derive(Default)]
pub struct Models {
    /// pasta do desenho (caminhos relativos)
    pub dir: Option<PathBuf>,
    parts: PartCache,
    loaded: HashMap<(PathBuf, String), Loaded>,
    proj: HashMap<Id, (u64, Proj)>,
    /// projeção das vistas de montagem fora da thread da interface (B427); `None` = tudo aqui
    fundo: Option<crate::fundo::Fundo>,
    /// só na thread de projeção: progresso e cancelamento do pedido em andamento
    controle: Option<fenkra_core::trabalho::Controle>,
    /// só na thread de projeção: volume de cada fonte já medida (medir o kart leva segundos)
    volumes: HashMap<crate::fundo::Fonte, f64>,
}

/// Vista de montagem: a que mais demora e a que vai para a thread de projeção.
fn e_montagem(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("fka"))
}

fn stamp(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl Models {
    pub fn new(dir: Option<PathBuf>) -> Self {
        let mut m = Models { dir, ..Default::default() };
        // o desenho projeta o sólido: não precisa da malha das peças (B393)
        m.parts.so_forma = true;
        m
    }

    pub fn full(&self, path: &str) -> PathBuf {
        full_path(self.dir.as_deref(), path)
    }

    /// Esquece o que mudou no disco (chamar de tempos em tempos). Devolve se algo mudou.
    pub fn refresh(&mut self) -> bool {
        let stale: Vec<(PathBuf, String)> =
            self.loaded.iter().filter(|(_, l)| l.files.iter().any(|(p, t)| stamp(p) != *t)).map(|(k, _)| k.clone()).collect();
        for k in &stale {
            if let Some(l) = self.loaded.remove(k) {
                for (p, _) in l.files {
                    self.parts.forget(&p);
                }
            }
        }
        // montagens calculadas em segundo plano: arquivo mudado esquece a projeção; o próximo pedido refaz
        let mut velhas = vec![];
        if let Some(f) = self.fundo.as_mut() {
            let ks: Vec<_> = f.arquivos.iter().filter(|(_, a)| a.iter().any(|(p, t)| stamp(p) != *t)).map(|(k, _)| k.clone()).collect();
            for k in ks {
                if let Some(a) = f.arquivos.remove(&k) {
                    velhas.extend(a.into_iter().map(|(p, _)| p));
                }
                f.extras.remove(&k);
                self.loaded.remove(&k);
            }
        }
        for p in &velhas {
            self.parts.forget(p);
        }
        !stale.is_empty() || !velhas.is_empty()
    }

    /// Estado da vista: erro de projeção, origem (ou vista mãe) alterada no disco desde a última
    /// leitura, ou atualizada. Lê só as datas dos arquivos; não recalcula nada que já esteja em cache.
    pub fn view_state(&mut self, d: &Drawing, id: Id) -> ViewState {
        let Some(v) = d.view(id) else { return ViewState::Erro };
        let parent = match &v.kind {
            ViewKind::Projected { parent, .. } | ViewKind::Section { parent, .. } | ViewKind::Detail { parent, .. } => Some(*parent),
            _ => None,
        };
        let pstate = parent.map(|p| self.view_state(d, p));
        if pstate == Some(ViewState::Erro) || self.proj(d, id).error.is_some() {
            return ViewState::Erro;
        }
        let full = self.full(&v.source.path);
        let key = (full, format!("{}|{:?}", v.source.display_state, v.source.hidden));
        let stale = self.loaded.get(&key).is_some_and(|l| l.files.iter().any(|(p, t)| stamp(p) != *t));
        if stale || pstate == Some(ViewState::Desatualizada) {
            ViewState::Desatualizada
        } else {
            ViewState::Atualizada
        }
    }

    /// Esquece tudo (recarregar à força).
    pub fn clear(&mut self) {
        if let Some(f) = self.fundo.as_mut() {
            f.cancelar_tudo();
            f.cancelados.clear();
            f.arquivos.clear();
        }
        self.loaded.clear();
        self.proj.clear();
        self.parts = PartCache::default();
    }

    /// Chamada do furo (`4× Ø6,5 ▽12 · M8×1,25-6H`) cujo diâmetro é `diametro`, gerada pelas
    /// features da peça com o mesmo gerador do 3D (`pmi_feature`), para a folha e o 3D nunca
    /// divergirem. Serve ao arco clicado na vista: a ligação é pelo diâmetro, e vários furos de
    /// mesmo diâmetro e formas diferentes pegam o primeiro. `None` se a origem não é uma peça ou
    /// nenhum furo tem esse diâmetro.
    pub fn chamada_furo(&mut self, src: &Source, diametro: f64) -> Option<fenkra_core::pmi_feature::Callout> {
        let Model::Part(doc) = &self.load(src).model else { return None };
        doc.features.iter().filter(|f| matches!(f.kind, fenkra_core::FeatureKind::Hole(_))).find_map(|f| {
            let c = fenkra_core::pmi_feature::callout(doc, f.id, true)?;
            // o círculo do furo ou o do rebaixo/escareado (a borda de fora)
            let bate = |d: f64| (d - diametro).abs() < 0.01;
            let fora = c.parts.iter().any(|p| matches!(p.label, "rebaixo" | "escareado") && p.value.is_some_and(bate));
            (c.hole_diameter.is_some_and(bate) || fora).then_some(c)
        })
    }

    pub fn load(&mut self, src: &Source) -> &Loaded {
        let full = self.full(&src.path);
        let key = (full.clone(), format!("{}|{:?}", src.display_state, src.hidden));
        if !self.loaded.contains_key(&key) {
            // com a thread de projeção ligada, a montagem inteira (formas) só é montada lá: aqui
            // entra só o que a interface precisa a cada quadro (a árvore, as tabelas, os nomes)
            let l = if self.fundo.is_some() && e_montagem(&src.path) { self.read_leve(&full, &key) } else { self.read(&full, &src.display_state, &src.hidden) };
            self.loaded.insert(key.clone(), l);
        }
        &self.loaded[&key]
    }

    /// Leitura leve de uma montagem: o modelo, sem montar nenhuma forma (B427).
    fn read_leve(&self, full: &Path, fonte: &crate::fundo::Fonte) -> Loaded {
        let mut files = vec![(full.to_path_buf(), stamp(full))];
        if let Some(a) = self.fundo.as_ref().and_then(|f| f.arquivos.get(fonte)) {
            files = a.clone();
        }
        let nomes = self.fundo.as_ref().and_then(|f| f.extras.get(fonte)).map(|e| e.1.clone()).unwrap_or_default();
        match std::fs::read_to_string(full).map_err(|e| e.to_string()).and_then(|t| Assembly::from_json(&t)) {
            Ok(a) => Loaded { model: Model::Asm(Box::new(a)), shape: None, pieces: vec![], piece_names: nomes, linhas: vec![], files, error: None },
            Err(e) => Loaded { model: Model::Part(Box::new(Document::new("?"))), shape: None, pieces: vec![], piece_names: vec![], linhas: vec![], files, error: Some(format!("{}: {e}", full.display())) },
        }
    }

    /// Volume da forma da origem, em mm³ (0 se ainda não há forma). Montagem calculada em segundo
    /// plano devolve o que a thread mediu.
    pub fn volume(&mut self, src: &Source) -> f64 {
        if self.fundo.is_some() && e_montagem(&src.path) {
            let fonte = self.fonte_de(src);
            return self.fundo.as_ref().and_then(|f| f.extras.get(&fonte)).map_or(0., |e| e.0);
        }
        self.load(src).shape.as_ref().map_or(0., |s| s.mass().volume)
    }

    fn read(&mut self, full: &Path, state: &str, hidden: &[u64]) -> Loaded {
        let mut files = vec![(full.to_path_buf(), stamp(full))];
        let fail = |files, e: String| Loaded { model: Model::Part(Box::new(Document::new("?"))), shape: None, pieces: vec![], piece_names: vec![], linhas: vec![], files, error: Some(e) };
        let txt = match std::fs::read_to_string(full) {
            Ok(t) => t,
            Err(e) => return fail(files, format!("{}: {e}", full.display())),
        };
        let is_asm = full.extension().is_some_and(|e| e.eq_ignore_ascii_case("fka"));
        if !is_asm {
            return match Document::from_json(&txt) {
                Ok(doc) => {
                    let shape = fenkra_solidos::final_shape(&doc);
                    let error = shape.is_none().then(|| "a peça não tem corpo visível".to_string());
                    Loaded { model: Model::Part(Box::new(doc)), shape, pieces: vec![], piece_names: vec![], linhas: vec![], files, error }
                }
                Err(e) => fail(files, e.to_string()),
            };
        }
        let a = match Assembly::from_json(&txt) {
            Ok(a) => a,
            Err(e) => return fail(files, e),
        };
        let dir = full.parent().map(|p| p.to_path_buf());
        let estado = a.display_states.iter().find(|s| s.name == state);
        let vista = |c: fenkra_core::Id| !hidden.contains(&c.0);
        // a peça pode ter mudado depois que a montagem foi salva: as linhas saem das malhas de agora
        let mut refeito = None;
        if let Some(st) = estado.filter(|s| s.explosao.is_some()) {
            let mut copia = a.clone();
            let (_, r) = compose(&copia.with_state(st), dir.as_deref(), &mut self.parts, &mut vec![]);
            let corpos = r.bodies.iter().map(|b| (b.collector, &b.mesh.positions[..]));
            let centros = fenkra_montagem::display::centros_dos_corpos(corpos, |id| st.offset(id));
            copia.refazer_estados(&centros);
            refeito = copia.display_states.into_iter().find(|s| s.name == state);
        }
        let estado = refeito.as_ref().or(estado);
        let mut shown_asm = estado.map_or_else(|| a.clone(), |st| a.with_state(st));
        let linhas: Vec<Vec<V3>> = estado.map_or(vec![], |st| st.linhas.iter().filter(|l| vista(l.comp) && !st.hidden.contains(&l.comp)).map(|l| l.pontos.clone()).collect());
        // componentes escondidos só nesta vista
        for n in &mut shown_asm.nodes {
            if hidden.contains(&n.id.0) {
                n.visible = false;
            }
        }
        let mut all = Shape::empty();
        let mut pieces: Vec<Shape> = vec![];
        let mut piece_names: Vec<String> = vec![];
        // B393: as formas de todas as peças de uma vez (disco e várias threads), antes do laço
        self.parts.preparar_formas(&fenkra_montagem::load::arquivos_das_pecas(&shown_asm, dir.as_deref()));
        // progresso: um passo por componente e mais um para a projeção
        if let Some(c) = &self.controle {
            c.definir_total(shown_asm.nodes.len() as u32 + 1);
        }
        for n in &shown_asm.nodes {
            if let Some(c) = &self.controle {
                c.avancar();
                if c.cancelado() {
                    return fail(files, "cancelado".into());
                }
            }
            if let NodeKind::SubAssembly { path } = &n.kind {
                // B381: subconjunto entra com as peças de dentro, no sistema de quem o contém
                let fp = full_path(dir.as_deref(), path);
                files.push((fp.clone(), stamp(&fp)));
                if !shown(&shown_asm, n.id) {
                    continue;
                }
                for (nome, s) in pecas_do_subconjunto(path, dir.as_deref(), &n.placement, &mut self.parts, &mut files, 0) {
                    all = if all.is_null() { s.clone() } else { all.compound(&s) };
                    pieces.push(s);
                    piece_names.push(format!("{} / {nome}", n.name));
                }
                continue;
            }
            let NodeKind::Part { path } = &n.kind else { continue };
            let fp = full_path(dir.as_deref(), path);
            files.push((fp.clone(), stamp(&fp)));
            if !shown(&shown_asm, n.id) {
                continue;
            }
            if let Some(s) = placed_shape(&shown_asm, n.id, dir.as_deref(), &mut self.parts) {
                all = if all.is_null() { s.clone() } else { all.compound(&s) };
                pieces.push(s);
                piece_names.push(n.name.clone());
            }
        }
        let shape = (!all.is_null()).then_some(all);
        let error = shape.is_none().then(|| "a montagem não tem peças visíveis".to_string());
        Loaded { model: Model::Asm(Box::new(a)), shape, pieces, piece_names, linhas, files, error }
    }

    /// Projeção da vista (calcula só quando a vista, a mãe ou o arquivo mudam).
    pub fn proj(&mut self, d: &Drawing, id: Id) -> Proj {
        let Some(v) = d.view(id) else { return Proj::default() };
        // B393: a projetada só usa a direção, não a projeção da mãe: não espera por ela
        let parent = match &v.kind {
            ViewKind::Section { parent, .. } | ViewKind::Detail { parent, .. } => Some(*parent),
            _ => None,
        };
        let parent_proj = parent.map(|p| self.proj(d, p));
        // a mãe ainda está sendo calculada: a filha espera por ela
        if parent_proj.as_ref().is_some_and(|p| p.calculando) {
            return Proj { calculando: true, ..Default::default() };
        }
        let no_fundo = self.fundo.is_some() && e_montagem(&v.source.path);
        let mut h = DefaultHasher::new();
        crate::fundo::chave_da_vista(v, parent_proj.as_ref(), &mut h);
        if no_fundo {
            // sem carregar a montagem aqui: vale o que a thread leu e as datas de quando leu
            let fonte = self.fonte_de(&v.source);
            if let Some(a) = self.fundo.as_ref().and_then(|f| f.arquivos.get(&fonte)) {
                for (p, t) in a {
                    p.hash(&mut h);
                    t.hash(&mut h);
                }
            }
        } else {
            for (p, t) in &self.load(&v.source).files {
                p.hash(&mut h);
                t.hash(&mut h);
            }
        }
        let key = h.finish();
        if let Some((k, p)) = self.proj.get(&id) {
            if *k == key {
                let p = p.clone();
                // prévia guardada sem a exata a caminho (o fundo foi cancelado e retomado): pede a exata.
                // A chave guardada conta os arquivos lidos e difere da do pedido: compara só a vista.
                let livre = self.fundo.as_ref().is_some_and(|f| !f.tem(v.id) && !f.cancelados.contains_key(&v.id));
                if p.previa && no_fundo && livre {
                    let _ = self.pedir_em_fundo(v, parent_proj, key, true);
                }
                return p;
            }
        }
        if no_fundo {
            return self.pedir_em_fundo(v, parent_proj, key, false);
        }
        let p = self.compute(v, parent_proj.as_ref());
        self.proj.insert(id, (key, p.clone()));
        p
    }

    fn fonte_de(&self, src: &Source) -> crate::fundo::Fonte {
        (self.full(&src.path), format!("{}|{:?}", src.display_state, src.hidden))
    }

    /// Liga a projeção das vistas de montagem numa thread própria. `acordar` chama a interface
    /// quando uma vista fica pronta.
    pub fn ativar_fundo(&mut self, acordar: impl Fn() + Send + 'static) {
        if self.fundo.is_none() {
            self.fundo = Some(crate::fundo::Fundo::novo(acordar));
        }
    }

    /// Liga ou desliga a prévia por malha da projeção em segundo plano (ligada por padrão).
    pub fn previa_do_fundo(&mut self, ligada: bool) {
        if let Some(f) = self.fundo.as_mut() {
            f.com_previa = ligada;
        }
    }

    pub fn fundo_ativo(&self) -> bool {
        self.fundo.is_some()
    }

    /// Pede a projeção em segundo plano e devolve o lugar dela: vazia, marcada `calculando`, ou com
    /// o aviso de que foi cancelada.
    fn pedir_em_fundo(&mut self, v: &View, mae: Option<Proj>, chave: u64, sem_previa: bool) -> Proj {
        let dir = self.dir.clone();
        let Some(f) = self.fundo.as_mut() else { return Proj::default() };
        if f.cancelados.get(&v.id) == Some(&chave) {
            return Proj { error: Some("cálculo cancelado".into()), ..Default::default() };
        }
        f.cancelados.remove(&v.id);
        if !f.pedida(v.id, chave) {
            f.pedir(crate::fundo::Pedido { id: v.id, vista: v.clone(), mae, dir, sem_previa }, chave);
        }
        Proj { calculando: true, ..Default::default() }
    }

    /// Recolhe as vistas que a thread terminou. `true` se alguma chegou (redesenhar).
    pub fn sondar(&mut self) -> bool {
        let Some(mut f) = self.fundo.take() else { return false };
        let mut chegou = false;
        for (r, ped, _, _) in f.receber() {
            let fonte = r.fonte.clone();
            f.arquivos.insert(r.fonte, r.arquivos);
            f.extras.insert(fonte.clone(), (r.volume, r.nomes));
            // a leitura leve guardada não tinha os nomes das peças nem os arquivos
            self.loaded.remove(&fonte);
            // a chave final conta os arquivos que a thread acabou de ler
            let mut h = DefaultHasher::new();
            crate::fundo::chave_da_vista(&ped.vista, ped.mae.as_ref(), &mut h);
            for (p, t) in f.arquivos.get(&fonte).into_iter().flatten() {
                p.hash(&mut h);
                t.hash(&mut h);
            }
            self.proj.insert(ped.id, (h.finish(), r.proj));
            chegou = true;
        }
        // as vistas pedidas desde o último quadro vão juntas à thread
        f.seguir();
        self.fundo = Some(f);
        chegou
    }

    /// Progresso da projeção em segundo plano; `None` sem fundo ou sem nada a fazer.
    pub fn andamento_do_fundo(&self) -> Option<crate::fundo::Andamento> {
        self.fundo.as_ref().filter(|f| f.ocupado()).map(|f| f.andamento())
    }

    /// Cancela a projeção em segundo plano: as vistas pedidas ficam marcadas como canceladas.
    pub fn cancelar_fundo(&mut self) {
        if let Some(f) = self.fundo.as_mut() {
            f.cancelar_tudo();
        }
    }

    /// Há vistas paradas porque a pessoa cancelou o cálculo?
    pub fn vistas_canceladas(&self) -> bool {
        self.fundo.as_ref().is_some_and(|f| !f.cancelados.is_empty())
    }

    /// Libera as vistas canceladas para serem calculadas de novo (o próximo quadro pede).
    pub fn retomar_fundo(&mut self) {
        if let Some(f) = self.fundo.as_mut() {
            f.cancelados.clear();
        }
    }

    fn compute(&mut self, v: &View, parent: Option<&Proj>) -> Proj {
        match self.preparar(v, parent) {
            Etapa::Pronta(p) => p,
            Etapa::Hlr(h) => {
                let r = h.alvo.hlr(h.axes.toward, h.axes.x, h.hidden);
                concluir(*h, r)
            }
        }
    }

    /// Tudo da projeção antes das linhas ocultas: carga, eixos, corte, sombreado. A vista que não
    /// precisa de linhas ocultas (detalhe, esquema, planificada de tubo) já sai pronta.
    fn preparar(&mut self, v: &View, parent: Option<&Proj>) -> Etapa {
        let tangent = v.tangent_edges;
        let hidden = v.hidden_lines;
        let l = self.load(&v.source);
        let linhas_explosao = l.linhas.clone();
        let mut out = Proj { error: l.error.clone(), ..Default::default() };
        // detalhe: recorte circular da vista mãe
        if let ViewKind::Detail { center, radius, .. } = &v.kind {
            let Some(pp) = parent else { return Etapa::Pronta(out) };
            out.axes = pp.axes;
            out.center = *center;
            // contorno livre (perfil) quando existe; senão o círculo
            let perfil = &v.detalhe.contorno;
            let recorte = |pl: &[V2]| if perfil.len() >= 3 { crate::corte::recortar_poligono(pl, perfil) } else { clip_circle(pl, *center, *radius) };
            for (pl, k) in &pp.lines {
                for piece in recorte(pl) {
                    out.lines.push((piece, *k));
                }
            }
            for lp in &pp.hatch {
                for piece in recorte(lp) {
                    out.lines.push((piece, 0));
                }
            }
            if perfil.len() >= 3 {
                let mut fecha = perfil.clone();
                fecha.push(perfil[0]);
                out.lines.push((fecha, 4));
                out.center = perfil.iter().fold([0., 0.], |a, p| [a[0] + p[0] / perfil.len() as f64, a[1] + p[1] / perfil.len() as f64]);
                out.bounds();
                return Etapa::Pronta(out);
            }
            out.lines.push((circle(*center, *radius), 4));
            out.bounds();
            return Etapa::Pronta(out);
        }
        if let ViewKind::Schematic(d) = &v.kind {
            // o esquema só precisa das juntas e posições (as peças podem nem estar no disco)
            out.error = None;
            match &l.model {
                Model::Asm(a) => schematic(a, Axes::of(*d), &mut out),
                Model::Part(_) => out.error = Some("esquema cinemático: a origem precisa ser uma montagem com juntas".into()),
            }
            out.bounds();
            out.center = [(out.lo[0] + out.hi[0]) / 2., (out.lo[1] + out.hi[1]) / 2.];
            return Etapa::Pronta(out);
        }
        let mut bends: Vec<fenkra_solidos::BendLine> = vec![];
        let shape = match (&v.kind, &l.model) {
            (ViewKind::Flat, Model::Part(doc)) => {
                let mut flat = (**doc).clone();
                match &mut flat.sheet {
                    Some(s) => {
                        s.flat = true;
                        // linhas de dobra (linha de centro e limites do arco) para a planificada
                        bends = fenkra_solidos::recompute(&flat).bend_lines;
                        fenkra_solidos::final_shape(&flat)
                    }
                    None => {
                        // B331: tubo dobrado na estrutura: a "planificada" é o tubo reto com as marcas
                        let tubos = fenkra_solidos::tubos_dobrados(doc);
                        let Some(tubo) = tubos.iter().find(|t| !t.dobras.is_empty()) else {
                            out.error = Some("a peça não é de chapa nem tem tubo dobrado: não há planificação".into());
                            return Etapa::Pronta(out);
                        };
                        match fenkra_solidos::st_dobra_vista::vista_reta(tubo) {
                            Ok(dv) => {
                                use fenkra_solidos::st_dobra_corte::Camada;
                                for t in &dv.tracos {
                                    let estilo = match t.camada {
                                        Camada::Dobra | Camada::Marcacao => 3,
                                        _ => 0,
                                    };
                                    let mut pts: Vec<V2> = t.pts.clone();
                                    if t.fechado {
                                        if let Some(p0) = pts.first().copied() {
                                            pts.push(p0);
                                        }
                                    }
                                    out.lines.push((pts, estilo));
                                }
                                for r in &dv.rotulos {
                                    out.labels.push((r.pos, r.texto.clone()));
                                }
                                out.bounds();
                                out.center = [(out.lo[0] + out.hi[0]) / 2., (out.lo[1] + out.hi[1]) / 2.];
                            }
                            Err(e) => out.error = Some(e),
                        }
                        return Etapa::Pronta(out);
                    }
                }
            }
            _ => l.shape.clone(),
        };
        let pieces: Vec<Shape> = if matches!(v.kind, ViewKind::Section { .. }) { l.pieces.clone() } else { vec![] };
        let Some(shape) = shape else { return Etapa::Pronta(out) };
        let Some((lo, hi)) = shape.bbox() else { return Etapa::Pronta(out) };
        let c3 = [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.);
        let big = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max) * 2. + 10.;
        let axes = match &v.kind {
            ViewKind::Standard(d) | ViewKind::Projected { dir: d, .. } => Axes::of(*d),
            ViewKind::Camera { toward, x } => {
                let (Some(t), Some(x)) = (unit(*toward), unit(*x)) else {
                    out.error = Some("direção da câmera inválida".into());
                    return Etapa::Pronta(out);
                };
                // x perpendicular à direção
                let x = unit(sub(x, scale3(t, dot(x, t)))).unwrap_or([1., 0., 0.]);
                Axes::new(t, x)
            }
            ViewKind::Flat => {
                // olhando pela menor dimensão (a espessura da chapa)
                let e = [0, 1, 2].map(|k| hi[k] - lo[k]);
                if e[2] <= e[0] && e[2] <= e[1] {
                    Axes::of(Dir::Topo)
                } else if e[1] <= e[0] {
                    Axes::of(Dir::Frente)
                } else {
                    Axes::of(Dir::Direita)
                }
            }
            ViewKind::Section { a, b, .. } => {
                let Some(pa) = parent.and_then(|p| p.axes) else { return Etapa::Pronta(out) };
                let (a3, b3) = (pa.unproject(*a), pa.unproject(*b));
                let Some(u) = unit(sub(b3, a3)) else {
                    out.error = Some("linha de corte sem comprimento".into());
                    return Etapa::Pronta(out);
                };
                let n = cross(pa.toward, u);
                Axes::new(n, u)
            }
            ViewKind::Detail { .. } | ViewKind::Schematic(_) => unreachable!(),
        };
        out.axes = Some(axes);
        out.center = axes.project(c3);
        let target = if let ViewKind::Section { a, b, .. } = &v.kind {
            // remove o lado do observador (prisma em degraus para linhas com vários segmentos) e
            // hachura a face cortada de cada segmento (módulo `corte`)
            let pa = parent.and_then(|p| p.axes).unwrap();
            let pts = crate::corte::caminho(*a, *b, &v.corte.pontos);
            match crate::corte::cortar(&shape, &pieces, &pa, &axes, &pts, &v.corte, c3, big) {
                Ok(c) => {
                    out.hatch = c.hachura;
                    out.hatch_group = c.grupo;
                    out.seams = c.costuras;
                    if v.corte.fatia {
                        // fatia: só a face cortada, sem o que está atrás
                        out.lines = out.hatch.iter().map(|lp| (lp.iter().copied().chain(lp.first().copied()).collect(), 0u8)).collect();
                        out.bounds();
                        return Etapa::Pronta(out);
                    }
                    c.alvo
                }
                Err(e) => {
                    out.error = Some(format!("corte: {e}"));
                    shape
                }
            }
        } else {
            shape
        };
        // sombreado: triângulos da malha projetados, do fundo para a frente (pintor)
        if v.render != Render::Arestas {
            if let Ok(m) = target.mesh(fenkra_core::qualidade::desvio_com_fator(big, 0.002)) {
                let pt = |i: u32| -> V3 { [0, 1, 2].map(|k| m.positions[i as usize * 3 + k] as f64) };
                let mut tris: Vec<(f64, [V2; 3], u8)> = vec![];
                for t in m.indices.chunks_exact(3) {
                    let (a, b, c) = (pt(t[0]), pt(t[1]), pt(t[2]));
                    let Some(n) = unit(cross(sub(b, a), sub(c, a))) else { continue };
                    let f = dot(n, axes.toward);
                    // faces de costas ficam escondidas pelas da frente; de lado não aparecem
                    if f < 0.02 {
                        continue;
                    }
                    let q = [axes.project(a), axes.project(b), axes.project(c)];
                    let area = ((q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1])).abs();
                    if area < 1e-6 {
                        continue;
                    }
                    let depth = (dot(a, axes.toward) + dot(b, axes.toward) + dot(c, axes.toward)) / 3.;
                    let g = (150. + 95. * f.clamp(0., 1.)) as u8;
                    tris.push((depth, q, g));
                }
                tris.sort_by(|x, y| x.0.total_cmp(&y.0));
                out.shade = tris.into_iter().map(|(_, t, g)| (t, g)).collect();
            }
        }
        Etapa::Hlr(Box::new(Hlr { out, alvo: target, axes, hidden, tangent, bends, explosao: linhas_explosao, tamanho: big }))
    }

    /// Lote de vistas da thread de projeção (B393): carrega a montagem uma vez e roda as linhas
    /// ocultas de todas ao mesmo tempo, uma thread por vista. `previa` projeta a malha.
    pub(crate) fn calcular_lote(&mut self, l: crate::fundo::Lote, ctl: &fenkra_core::trabalho::Controle) -> Option<Vec<crate::fundo::Resposta>> {
        let Some(p0) = l.pedidos.first() else { return Some(vec![]) };
        self.dir = p0.dir.clone();
        // arquivo salvo desde a última vez: esquece a peça velha antes de calcular
        self.refresh();
        self.controle = Some(ctl.clone());
        self.parts.cancelar = Some(ctl.bandeira());
        // a carga feita antes ainda vale? (`refresh` esquece o que mudou no disco)
        self.volumes.retain(|k, _| self.loaded.contains_key(k));
        let medir = std::env::var_os("FENKRA_MEDIR_DESENHO").is_some();
        let t0 = std::time::Instant::now();
        let mut etapas = vec![];
        for p in &l.pedidos {
            etapas.push(self.preparar(&p.vista, p.mae.as_ref()));
            if ctl.cancelado() {
                break;
            }
        }
        self.controle = None;
        self.parts.cancelar = None;
        if ctl.cancelado() {
            // nada pela metade: a carga cancelada não fica no cache
            self.loaded.clear();
            self.proj.clear();
            return None;
        }
        // B393: linhas exatas já guardadas no disco (mesma vista, mesmos arquivos, mesmo programa)
        let chaves: Vec<Option<String>> = l
            .pedidos
            .iter()
            .zip(&etapas)
            .map(|(p, e)| match e {
                Etapa::Hlr(_) => {
                    let arquivos = self.load(&p.vista.source).files.clone();
                    crate::hlr_disco::chave(&p.vista, p.mae.as_ref(), &arquivos)
                }
                Etapa::Pronta(_) => None,
            })
            .collect();
        let mut do_disco: Vec<Option<crate::hlr_disco::Linhas>> = chaves.iter().map(|k| k.as_deref().and_then(crate::hlr_disco::ler)).collect();
        // as linhas ocultas contam como um passo a mais na barra
        ctl.somar_total(1);
        let t_hlr = std::time::Instant::now();
        let hlrs: Vec<&Hlr> = etapas.iter().zip(&do_disco).filter_map(|(e, d)| if let (Etapa::Hlr(h), None) = (e, d) { Some(&**h) } else { None }).collect();
        let vistas: Vec<_> = hlrs.iter().map(|h| fenkra_kernel::VistaHlr { forma: &h.alvo, para: h.axes.toward, x: h.axes.x, ocultas: h.hidden, prioridade_baixa: true }).collect();
        // desvio da malha da prévia: fino o bastante para os furos da folha saírem redondos
        let desvio = l.previa.then(|| hlrs.iter().map(|h| h.tamanho).fold(0., f64::max) * DESVIO_DA_PREVIA);
        // a exata vai um processo por vista; sem o executável, uma depois da outra aqui
        let cancelar = ctl.bandeira();
        let processos = if desvio.is_none() { crate::hlr_processos::exatas(&vistas, Some(&*cancelar)) } else { None };
        // B393: a prévia sai do mapa de profundidade (ADR 0089): a malha da forma é feita uma vez
        // (o malhador já usa várias threads) e cada vista é projetada numa thread, sem OpenCASCADE
        let rapida = desvio.map(|d| linhas_rapidas(&hlrs, d / DESVIO_DA_PREVIA * DESVIO_DA_RAPIDA));
        let mut linhas = processos.or(rapida).unwrap_or_else(|| fenkra_kernel::hlr_varias(&vistas, desvio)).into_iter();
        drop(vistas);
        if medir {
            eprintln!("[desenho] lote de {} ({}): preparar+carga {:?}, linhas {:?}", l.pedidos.len(), if l.previa { "prévia" } else { "exata" }, t_hlr - t0, t_hlr.elapsed());
        }
        ctl.avancar();
        let mut out = vec![];
        for (i, (p, e)) in l.pedidos.iter().zip(etapas).enumerate() {
            let guardada = do_disco[i].take();
            let veio_do_disco = guardada.is_some();
            let mut proj = match (e, guardada) {
                (Etapa::Pronta(p), _) => p,
                (Etapa::Hlr(h), Some(g)) => concluir(*h, Ok(g)),
                (Etapa::Hlr(h), None) => {
                    let r = linhas.next().unwrap_or_else(|| Err(fenkra_kernel::KernelError("sem resultado".into())));
                    if let (false, Ok(v), Some(k)) = (l.previa, &r, &chaves[i]) {
                        crate::hlr_disco::gravar(k, v);
                    }
                    concluir(*h, r)
                }
            };
            // a exata do disco já é a definitiva: não há prévia a refinar
            proj.previa = l.previa && proj.error.is_none() && !veio_do_disco;
            let fonte = self.fonte_de(&p.vista.source);
            let guardado = self.volumes.get(&fonte).copied();
            let (arquivos, volume, nomes) = {
                let ld = self.load(&p.vista.source);
                (ld.files.clone(), guardado.unwrap_or_else(|| ld.shape.as_ref().map_or(0., |s| s.mass().volume)), ld.piece_names.clone())
            };
            self.volumes.insert(fonte.clone(), volume);
            out.push(crate::fundo::Resposta { id: p.id, proj, fonte, arquivos, volume, nomes });
        }
        Some(out)
    }

    /// Dados de uma tabela.
    pub fn table(&mut self, t: &Table, d: &Drawing) -> TableData {
        // B117: o plano de inspeção vem do próprio desenho, não da peça
        if t.kind == TableKind::Caracteristicas {
            return crate::inspecao::tabela(d, self);
        }
        let full = self.full(&t.source.path);
        let dir = full.parent().map(|p| p.to_path_buf());
        let num = |v: f64| d.num(v);
        let l = self.load(&t.source);
        if let Some(e) = &l.error {
            if l.shape.is_none() && matches!(l.model, Model::Part(_)) && l.files.len() == 1 && e.contains(':') {
                return TableData { headers: vec!["Erro".into()], rows: vec![vec![e.clone()]] };
            }
        }
        match (&t.kind, &l.model) {
            (TableKind::Bom, Model::Asm(a)) => {
                let rows = fenkra_montagem::bom::bom(a, dir.as_deref(), t.level);
                let mut out = vec![];
                for r in rows {
                    let (pn, desc, mat) = if r.sub {
                        match std::fs::read_to_string(&r.path).ok().and_then(|s| Assembly::from_json(&s).ok()) {
                            Some(x) => (x.props.pn, if x.props.description.is_empty() { x.name } else { x.props.description }, "—".to_string()),
                            None => (String::new(), r.name.clone(), String::new()),
                        }
                    } else {
                        // B102: o código e o material são os da configuração do componente; configuração
                        // que a peça não tem mostra o motivo no lugar do código
                        // B393: só o documento; calcular as peças aqui travava a interface (20 s no kart)
                        let doc = self.parts.documento(&r.path).cloned().and_then(|d| if r.config.is_empty() { Ok(d) } else { d.com_configuracao(&r.config).map_err(|e| e.to_string()) });
                        match doc {
                            Ok(doc) => {
                                let pr = &doc.props;
                                (pr.pn.clone(), if pr.description.is_empty() { doc.name.clone() } else { pr.description.clone() }, pr.material.clone())
                            }
                            Err(e) if !r.config.is_empty() => (format!("⚠ {e}"), r.name.clone(), String::new()),
                            Err(_) => (String::new(), r.name.clone(), String::new()),
                        }
                    };
                    // estruturada: recuo pela profundidade
                    let desc = format!("{}{}", "  ".repeat(r.depth), desc);
                    out.push(vec![r.item, pn, desc, mat, r.qty.to_string()]);
                }
                TableData { headers: ["Item", "Código", "Descrição", "Material", "Qtd."].map(String::from).to_vec(), rows: out }
            }
            // lista de corte da montagem: barras de todas as peças (vezes a quantidade de cada uma)
            (TableKind::CutList, Model::Asm(a)) => {
                let mut acc: Vec<(String, f64, usize, usize)> = vec![];
                for r in fenkra_montagem::bom::bom(a, dir.as_deref(), fenkra_montagem::bom::BomLevel::Pecas) {
                    let Ok(doc) = self.parts.documento(&r.path) else { continue };
                    for c in fenkra_solidos::cut_list(doc) {
                        let n = c.count * r.qty as usize;
                        match acc.iter_mut().find(|x| x.0 == c.designation && (x.1 - c.length).abs() < 0.05) {
                            Some(x) => {
                                x.2 += n;
                                x.3 += n * c.bars;
                            }
                            None => acc.push((c.designation.clone(), c.length, n, n * c.bars)),
                        }
                    }
                }
                TableData {
                    headers: ["Item", "Perfil", "Comprimento", "Qtd.", "Barras"].map(String::from).to_vec(),
                    rows: acc.into_iter().enumerate().map(|(i, (d, l, n, b))| vec![(i + 1).to_string(), d, num(l), n.to_string(), b.to_string()]).collect(),
                }
            }
            (TableKind::Bom, Model::Part(doc)) => TableData {
                headers: ["Item", "Código", "Descrição", "Material", "Qtd."].map(String::from).to_vec(),
                rows: vec![vec!["1".into(), doc.props.pn.clone(), if doc.props.description.is_empty() { doc.name.clone() } else { doc.props.description.clone() }, doc.props.material.clone(), "1".into()]],
            },
            (TableKind::Holes, Model::Part(doc)) => TableData {
                headers: ["Furo", "X", "Y", "Z", "Ø", "Prof.", "Rosca"].map(String::from).to_vec(),
                rows: fenkra_solidos::hole_table(doc)
                    .iter()
                    .enumerate()
                    .map(|(i, h)| vec![format!("A{}", i + 1), num(h.pos[0]), num(h.pos[1]), num(h.pos[2]), num(h.diameter), if h.through { "passante".into() } else { num(h.depth) }, h.thread.clone()])
                    .collect(),
            },
            (TableKind::Bends, Model::Part(doc)) => {
                // tabela do mapa: mesma ordem e etiqueta das linhas de dobra da planificada
                let linhas = fenkra_solidos::recompute(doc).bend_lines;
                // tubo dobrado na estrutura: a tabela de dobras do tubo (B331)
                let tubos = fenkra_solidos::tubos_dobrados(doc);
                if linhas.is_empty() && !tubos.is_empty() {
                    let mut rows = vec![];
                    let mut n = 0;
                    for (t, tubo) in tubos.iter().enumerate() {
                        for d in &tubo.dobras {
                            n += 1;
                            let r = &d.resultado;
                            rows.push(vec![n.to_string(), format!("T{}", t + 1), fenkra_solidos::st_dobra::nome_modo(r.modo).to_string(), format!("{}°", num(r.theta)), num(r.raio), num(d.inicio), num(d.fim), num(tubo.desenvolvido)]);
                        }
                    }
                    return TableData { headers: ["Nº", "Tubo", "Modo", "Ângulo", "Raio", "Início", "Fim", "Desenvolvido"].map(String::from).to_vec(), rows };
                }
                TableData {
                    headers: ["Nº", "Etiqueta", "Dobra", "Ângulo", "Raio", "Desenvolvido", "Dedução"].map(String::from).to_vec(),
                    rows: fenkra_solidos::tabela_dobras(doc, &linhas)
                        .iter()
                        .map(|b| vec![b.ordem.to_string(), b.rotulo.clone(), b.name.clone(), format!("{}°", num(b.angle)), num(b.radius), num(b.allowance), num(b.deduction)])
                        .collect(),
                }
            }
            (TableKind::CutList, Model::Part(doc)) => TableData {
                headers: ["Item", "Perfil", "Comprimento", "Qtd.", "Barras"].map(String::from).to_vec(),
                rows: fenkra_solidos::cut_list(doc)
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        // o número é o do item da estrutura (o mesmo da montagem gerada e dos balões); sem sólido cai no índice
                        let item = if r.item > 0 { r.item } else { i + 1 };
                        vec![item.to_string(), r.designation.clone(), num(r.length), r.count.to_string(), (r.bars * r.count).to_string()]
                    })
                    .collect(),
            },
            // tratado no começo: vem do desenho
            (TableKind::Caracteristicas, Model::Part(_)) => TableData { headers: vec![], rows: vec![] },
            (k, Model::Asm(_)) => TableData { headers: vec![k.label().into()], rows: vec![vec!["escolha uma peça (esta tabela não vale para montagem)".into()]] },
        }
    }

    /// Furos da peça de origem (vazio para montagem).
    pub fn furos(&mut self, src: &Source) -> Vec<fenkra_solidos::HoleRow> {
        match &self.load(src).model {
            Model::Part(doc) => fenkra_solidos::hole_table(doc),
            Model::Asm(_) => vec![],
        }
    }

    /// Propriedades da origem (para preencher a legenda).
    pub fn props(&mut self, src: &Source) -> Option<(String, String, String)> {
        match &self.load(src).model {
            Model::Part(d) => Some((d.props.pn.clone(), if d.props.description.is_empty() { d.name.clone() } else { d.props.description.clone() }, d.props.material.clone())),
            Model::Asm(a) => Some((a.props.pn.clone(), if a.props.description.is_empty() { a.name.clone() } else { a.props.description.clone() }, String::new())),
        }
    }
}

/// Peças de um subconjunto (e dos subconjuntos dentro dele), já levadas ao sistema de quem o contém.
/// Os arquivos lidos entram em `files` para a vista saber quando refazer.
fn pecas_do_subconjunto(path: &str, dir: Option<&Path>, pl: &fenkra_montagem::Placement, cache: &mut PartCache, files: &mut Vec<(PathBuf, Option<SystemTime>)>, fundo: u32) -> Vec<(String, Shape)> {
    if fundo > 16 {
        return vec![];
    }
    let full = full_path(dir, path);
    let Some(sub) = std::fs::read_to_string(&full).ok().and_then(|t| Assembly::from_json(&t).ok()) else { return vec![] };
    let sub_dir = full.parent().map(|x| x.to_path_buf());
    let mut out = vec![];
    for n in &sub.nodes {
        if !shown(&sub, n.id) {
            continue;
        }
        match &n.kind {
            NodeKind::Part { path } => {
                let fp = full_path(sub_dir.as_deref(), path);
                files.push((fp.clone(), stamp(&fp)));
                if let Some(s) = placed_shape(&sub, n.id, sub_dir.as_deref(), cache).and_then(|s| fenkra_montagem::view::posicionado(s, pl)) {
                    out.push((n.name.clone(), s));
                }
            }
            NodeKind::SubAssembly { path } => {
                let fp = full_path(sub_dir.as_deref(), path);
                files.push((fp.clone(), stamp(&fp)));
                for (nome, s) in pecas_do_subconjunto(path, sub_dir.as_deref(), &n.placement, cache, files, fundo + 1) {
                    if let Some(s) = fenkra_montagem::view::posicionado(s, pl) {
                        out.push((format!("{} / {nome}", n.name), s));
                    }
                }
            }
            NodeKind::Group => {}
        }
    }
    out
}

/// Esquema cinemático: símbolo de cada junta, barras ligando as juntas de cada peça e terra nas
/// peças fixas (tamanho dos símbolos proporcional ao mecanismo).
fn schematic(a: &Assembly, ax: Axes, out: &mut Proj) {
    use fenkra_montagem::kin::JointKind as J;
    let joints: Vec<_> = a.joints.iter().filter(|j| !j.suppressed).collect();
    if joints.is_empty() {
        out.error = Some("a montagem não tem juntas (Cinemática › juntas ou Mecanismo a partir das restrições)".into());
        return;
    }
    out.axes = Some(ax);
    let place = |c: Id| a.node(c).map(|n| n.placement).unwrap_or_default();
    // posição e eixo (na vista) de cada junta, pelo lado A
    let info: Vec<(V2, V2, &J, Id, Id)> = joints
        .iter()
        .map(|j| {
            let (o, z, _) = j.a.frame.world(&place(j.a.comp));
            (ax.project(o), [dot(z, ax.x), dot(z, ax.y)], &j.kind, j.a.comp, j.b.comp)
        })
        .collect();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (p, ..) in &info {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let s = ((hi[0] - lo[0]).max(hi[1] - lo[1]) * 0.05).max(3.);
    let rect = |c: V2, u: V2, a: f64, b: f64| -> Vec<V2> {
        let n = [-u[1], u[0]];
        let q = |i: f64, j: f64| [c[0] + u[0] * a * i + n[0] * b * j, c[1] + u[1] * a * i + n[1] * b * j];
        vec![q(-1., -1.), q(1., -1.), q(1., 1.), q(-1., 1.), q(-1., -1.)]
    };
    for (p, z, kind, ca, _) in &info {
        let zl = (z[0] * z[0] + z[1] * z[1]).sqrt();
        let u = if zl > 1e-6 { [z[0] / zl, z[1] / zl] } else { [1., 0.] };
        let edge_on = zl > 0.3;
        match kind {
            J::Revolute | J::Cylindrical | J::Screw { .. } if !edge_on => out.lines.push((circle(*p, s * 0.5), 0)),
            J::Revolute | J::Cylindrical | J::Screw { .. } => {
                // mancal visto de lado: retângulo atravessado pelo eixo
                out.lines.push((rect(*p, u, s * 0.6, s * 0.35), 0));
                out.lines.push((vec![[p[0] - u[0] * s, p[1] - u[1] * s], [p[0] + u[0] * s, p[1] + u[1] * s]], 0));
            }
            J::Prismatic => {
                out.lines.push((rect(*p, u, s * 0.8, s * 0.4), 0));
                out.lines.push((vec![[p[0] - u[0] * s * 1.6, p[1] - u[1] * s * 1.6], [p[0] + u[0] * s * 1.6, p[1] + u[1] * s * 1.6]], 0));
            }
            J::Spherical => {
                out.lines.push((circle(*p, s * 0.55), 0));
                out.lines.push((circle(*p, s * 0.25), 0));
            }
            J::Cam { profile, .. } => {
                // o próprio perfil do came, posicionado
                let pl = place(*ca);
                let mut pts: Vec<V2> = profile.iter().map(|q| ax.project(pl.point(*q))).collect();
                if let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                out.lines.push((pts, 0));
            }
            _ => out.lines.push((rect(*p, [1., 0.], s * 0.4, s * 0.4), 0)),
        }
    }
    // barras: juntas de cada peça ligadas em sequência
    let mut comps: Vec<Id> = vec![];
    for (_, _, _, ca, cb) in &info {
        for c in [ca, cb] {
            if !comps.contains(c) {
                comps.push(*c);
            }
        }
    }
    for c in comps {
        let pts: Vec<V2> = info.iter().filter(|i| i.3 == c || i.4 == c).map(|i| i.0).collect();
        let fixed = a.node(c).is_some_and(|n| n.fixed);
        if pts.len() >= 2 && !fixed {
            out.lines.push((pts.clone(), 0));
        }
        if fixed {
            // terra: base com hachuras sob cada junta da peça fixa
            for p in pts {
                let y = p[1] - s * 0.9;
                out.lines.push((vec![[p[0], p[1] - s * 0.5], [p[0], y]], 0));
                out.lines.push((vec![[p[0] - s, y], [p[0] + s, y]], 0));
                for i in 0..5 {
                    let x = p[0] - s + i as f64 * s * 0.5;
                    out.lines.push((vec![[x, y], [x - s * 0.35, y - s * 0.4]], 0));
                }
            }
        }
    }
}

// ---------- geometria 2D/3D simples

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn scale3(a: V3, k: f64) -> V3 {
    a.map(|x| x * k)
}
pub fn unit(a: V3) -> Option<V3> {
    let l = dot(a, a).sqrt();
    (l > 1e-12).then(|| a.map(|x| x / l))
}

pub fn circle(c: V2, r: f64) -> Vec<V2> {
    (0..=72).map(|i| {
        let t = i as f64 / 72. * std::f64::consts::TAU;
        [c[0] + r * t.cos(), c[1] + r * t.sin()]
    })
    .collect()
}

/// Partes da polilinha dentro do círculo (cortando os segmentos na borda).
pub fn clip_circle(pl: &[V2], c: V2, r: f64) -> Vec<Vec<V2>> {
    let inside = |p: V2| (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r * r;
    // interseção do segmento p→q com o círculo (parâmetros t em [0,1])
    let hits = |p: V2, q: V2| -> Vec<f64> {
        let d = [q[0] - p[0], q[1] - p[1]];
        let f = [p[0] - c[0], p[1] - c[1]];
        let a = d[0] * d[0] + d[1] * d[1];
        if a < 1e-18 {
            return vec![];
        }
        let b = 2. * (f[0] * d[0] + f[1] * d[1]);
        let cc = f[0] * f[0] + f[1] * f[1] - r * r;
        let disc = b * b - 4. * a * cc;
        if disc < 0. {
            return vec![];
        }
        let s = disc.sqrt();
        let mut v: Vec<f64> = [(-b - s) / (2. * a), (-b + s) / (2. * a)].into_iter().filter(|t| (0. ..=1.).contains(t)).collect();
        v.sort_by(f64::total_cmp);
        v
    };
    let lerp = |p: V2, q: V2, t: f64| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
    let mut out = vec![];
    let mut cur: Vec<V2> = vec![];
    for w in pl.windows(2) {
        let (p, q) = (w[0], w[1]);
        let mut ts = vec![0.];
        ts.extend(hits(p, q));
        ts.push(1.);
        for s in ts.windows(2) {
            let (a, b) = (lerp(p, q, s[0]), lerp(p, q, s[1]));
            let m = lerp(p, q, (s[0] + s[1]) / 2.);
            if inside(m) {
                if cur.is_empty() {
                    cur.push(a);
                }
                cur.push(b);
            } else if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

/// Desvio da malha da prévia, como fração do tamanho da vista (o dobro da maior medida).
const DESVIO_DA_PREVIA: f64 = 0.0004;
/// Desvio da malha da prévia pelo mapa de profundidade, em fração do maior lado (B393). O ângulo da
/// malha (0,35 rad) já deixa cada furo com 18 lados ou mais; o desvio linear maior só tira triângulos
/// das superfícies grandes e curvas, que eram a maior parte do tempo da prévia no kart.
const DESVIO_DA_RAPIDA: f64 = 0.0015;

/// Uma vista preparada: pronta ou esperando as linhas ocultas.
enum Etapa {
    Pronta(Proj),
    Hlr(Box<Hlr>),
}

/// O que falta para terminar a vista depois das linhas ocultas.
struct Hlr {
    out: Proj,
    alvo: Shape,
    axes: Axes,
    hidden: bool,
    tangent: bool,
    bends: Vec<fenkra_solidos::BendLine>,
    explosao: Vec<Vec<V3>>,
    tamanho: f64,
}

/// Linhas da prévia pelo mapa de profundidade (B393, `hlr_rapida`): uma malha por forma distinta,
/// uma thread por vista.
fn linhas_rapidas(hlrs: &[&Hlr], desvio: f64) -> Vec<fenkra_kernel::KResult<Vec<(Vec<V2>, u8)>>> {
    let mut malhas: Vec<(usize, fenkra_kernel::KResult<fenkra_kernel::Mesh>)> = vec![];
    let mut de_quem: Vec<usize> = vec![];
    for (i, h) in hlrs.iter().enumerate() {
        match malhas.iter().position(|(j, _)| hlrs[*j].alvo.mesma(&h.alvo)) {
            Some(k) => de_quem.push(k),
            None => {
                de_quem.push(malhas.len());
                let t = std::time::Instant::now();
                malhas.push((i, h.alvo.mesh(desvio)));
                if std::env::var_os("FENKRA_MEDIR_DESENHO").is_some() {
                    eprintln!("[desenho] malha da prévia: {:?}, {} triângulos", t.elapsed(), malhas.last().and_then(|m| m.1.as_ref().ok()).map_or(0, |m| m.indices.len() / 3));
                }
            }
        }
    }
    std::thread::scope(|s| {
        let tarefas: Vec<_> = hlrs
            .iter()
            .zip(&de_quem)
            .map(|(h, k)| {
                let (m, axes, ocultas) = (&malhas[*k].1, &h.axes, h.hidden);
                s.spawn(move || match m {
                    Ok(m) => Ok(crate::hlr_rapida::projetar(m, axes, ocultas, crate::hlr_rapida::RESOLUCAO)),
                    Err(e) => Err(e.clone()),
                })
            })
            .collect();
        tarefas.into_iter().map(|t| t.join().unwrap_or_else(|_| Err(fenkra_kernel::KernelError("projeção da vista falhou".into())))).collect()
    })
}

/// Termina a vista com as linhas ocultas: tangentes, linhas de dobra e de explosão, limites.
fn concluir(h: Hlr, linhas: fenkra_kernel::KResult<Vec<(Vec<V2>, u8)>>) -> Proj {
    let Hlr { mut out, axes, tangent, bends, explosao, .. } = h;
    match linhas {
        Ok(lines) => out.lines = lines.into_iter().filter(|(_, k)| tangent || *k != 1).collect(),
        Err(e) => out.error = Some(e.0),
    }
    // planificada: linhas de dobra (5 = centro, 6 = limites do arco) e o rótulo de cada uma
    for b in &bends {
        let pr = |q: [V3; 2]| vec![axes.project(q[0]), axes.project(q[1])];
        if b.curva.len() > 2 {
            out.lines.push((b.curva.iter().map(|q| axes.project(*q)).collect(), 5));
        } else {
            out.lines.push((pr(b.center), 5));
        }
        out.lines.push((pr(b.start), 6));
        out.lines.push((pr(b.end), 6));
        let m = [(b.center[0][0] + b.center[1][0]) / 2., (b.center[0][1] + b.center[1][1]) / 2., (b.center[0][2] + b.center[1][2]) / 2.];
        out.labels.push((axes.project(m), b.label()));
    }
    out.lines.extend(explosao.iter().map(|l| (l.iter().map(|p| axes.project(*p)).collect(), 7u8)));
    out.bounds();
    out
}

#[cfg(test)]
mod testes_lista {
    use super::*;
    use crate::{Format, Table, TableKind};

    /// B393: a lista de peças de uma montagem só lê os documentos. Antes ela calculava cada peça
    /// (comandos e malha) na thread da interface, e o desenho do kart ficava parado uns 20 s.
    #[test]
    fn lista_de_pecas_da_montagem_nao_calcula_as_pecas() {
        let dir = std::env::temp_dir().join(format!("fenkra_lista_sem_calculo_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut peca = fenkra_solidos::example_document();
        peca.props.pn = "PN-77".into();
        std::fs::write(dir.join("placa.fkp"), peca.to_json()).unwrap();
        let mut a = Assembly::new("raiz");
        for k in 0..2 {
            a.add_component(&format!("P{k}"), fenkra_montagem::NodeKind::Part { path: "placa.fkp".into() }, Default::default(), None);
        }
        std::fs::write(dir.join("raiz.fka"), a.to_json()).unwrap();
        let src = Source { path: "raiz.fka".into(), display_state: String::new(), hidden: vec![] };
        let mut d = Drawing::new("t", Format::A3);
        let s = d.sheets[0].id;
        let t = Table { id: d.alloc(), sheet: s, kind: TableKind::Bom, source: src, pos: [0., 0.], level: Default::default(), cfg: Default::default() };
        let mut m = Models::new(Some(dir.clone()));
        let dados = m.table(&t, &d);
        assert!(dados.rows.iter().any(|r| r.iter().any(|c| c == "PN-77")), "{:?}", dados.rows);
        assert!(m.parts.tirar_recem().is_empty(), "a lista não pode calcular peça");
        let _ = std::fs::remove_dir_all(dir);
    }
}
