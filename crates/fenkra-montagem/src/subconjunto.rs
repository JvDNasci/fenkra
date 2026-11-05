//! Extrair componentes para um subconjunto novo e inserir de volta, sem perda (B73).
//!
//! O nó do subconjunto nasce na origem, então o sistema do subconjunto é o mesmo mundo da montagem
//! de cima: as peças mantêm as posições, e o que estava no mundo (padrões, furos) continua valendo.
//!
//! - Tudo que liga só peças de dentro **vai junto**, com os mesmos ids. Manter os ids é o que torna
//!   a volta exata e dispensa traduzir as referências cruzadas (juntas × acoplamentos, padrões,
//!   estados de exibição).
//! - Restrição, junta ou fixação com o exterior é **religada** ao nó do subconjunto: o lado de dentro
//!   passa a apontar para o nó, com a geometria levada do sistema da peça para o do subconjunto. A
//!   peça não se move, e o conjunto continua preso ao resto.
//! - O original de cada item religado ou dividido fica em [`Assembly::religacoes`]. O nó só sabe que
//!   "algo de dentro" estava ligado; sem esse registro, a volta não saberia a qual peça devolver.
//!
//! O que não tem como atravessar a fronteira aparece em `perdidas`, sem silêncio: acoplamento entre
//! uma junta de dentro e outra de fora, e recorte em que a ferramenta e o alvo ficam em lados
//! opostos. Eles voltam ao inserir de volta.

use crate::kin::{Coupling, JFrame, JRef, Joint, JointKind};
use crate::*;
use std::collections::{HashMap, HashSet};

/// Item da montagem como era antes de extrair.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum ItemOriginal {
    Mate(Mate),
    Joint(Joint),
    Rigid(RigidGroup),
    Coupling(Coupling),
    Feature(AsmFeature),
}

/// O que é preciso para desfazer a ligação de um item com o nó do subconjunto `sub`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Religacao {
    pub sub: Id,
    pub original: ItemOriginal,
}

/// O que uma extração (ou a volta) fez.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelatorioExtracao {
    pub componentes: usize,
    pub restricoes: usize,
    pub juntas: usize,
    pub acoplamentos: usize,
    pub padroes: usize,
    pub fixacoes: usize,
    pub recursos: usize,
    pub parametros: usize,
    pub estados: usize,
    /// itens ligados ao nó do subconjunto
    pub religadas: Vec<String>,
    /// o que ficou de fora, uma frase por item
    pub perdidas: Vec<String>,
}

fn geom_em(g: &Geom, p: &Placement) -> Geom {
    match *g {
        Geom::Plane { p: o, n } => Geom::Plane { p: p.point(o), n: p.dir(n) },
        Geom::Axis { p: o, d } => Geom::Axis { p: p.point(o), d: p.dir(d) },
        Geom::Point(o) => Geom::Point(p.point(o)),
    }
}

/// Lado de uma restrição agora no nó: as referências de giro são locais à peça e mudam de sistema.
fn religar_mate(m: &mut Mate, lado_a: bool, no: Id, pl: &Placement) {
    let r = if lado_a { &mut m.a } else { &mut m.b };
    *r = MateRef { comp: no, geom: geom_em(&r.geom, pl), pick: None };
    match &mut m.kind {
        MateKind::Gear { refs, .. } => {
            let i = if lado_a { 0 } else { 2 };
            refs[i] = pl.dir(refs[i]);
        }
        MateKind::RackPinion { refs, .. } => {
            let i = if lado_a { 0 } else { 3 };
            refs[i] = pl.dir(refs[i]);
        }
        _ => {}
    }
}

fn frame_em(f: &JFrame, p: &Placement) -> JFrame {
    JFrame { o: p.point(f.o), z: p.dir(f.z), x: p.dir(f.x) }
}

/// Lado de uma junta agora no nó. Curva e came ficam no sistema de A.
fn religar_junta(j: &mut Joint, lado_a: bool, no: Id, pl: &Placement) {
    let r: &mut JRef = if lado_a { &mut j.a } else { &mut j.b };
    *r = JRef { comp: no, frame: frame_em(&r.frame, pl) };
    if lado_a {
        if let JointKind::PointCurve { curve: c } | JointKind::Cam { profile: c, .. } = &mut j.kind {
            for q in c.iter_mut() {
                *q = pl.point(*q);
            }
        }
    }
}

/// Nomes usados numa expressão.
fn nomes(e: &str, out: &mut Vec<String>) {
    if let Ok(ast) = expr::parse(e) {
        expr::names(&ast, out);
    }
}

/// Parâmetros de que as expressões do subconjunto precisam, com as dependências entre eles.
fn parametros_usados(a: &Assembly, sub: &Assembly) -> Vec<AsmParam> {
    let mut quer: Vec<String> = vec![];
    for m in &sub.mates {
        match &m.kind {
            MateKind::Distance(e) | MateKind::Angle(e) | MateKind::Gear { ratio: e, .. } | MateKind::RackPinion { mm_per_rev: e, .. } => nomes(e, &mut quer),
            _ => {}
        }
    }
    for j in &sub.joints {
        for e in [&j.drive_angle, &j.drive_len].into_iter().flatten() {
            nomes(e, &mut quer);
        }
        match &j.kind {
            JointKind::Screw { pitch: e } | JointKind::Cam { radius: e, .. } => nomes(e, &mut quer),
            _ => {}
        }
    }
    for c in &sub.couplings {
        nomes(&c.ratio, &mut quer);
    }
    for p in &sub.patterns {
        match &p.kind {
            PatternKind::Linear { count, spacing, .. } => [count, spacing].iter().for_each(|e| nomes(e, &mut quer)),
            PatternKind::Circular { count, angle, .. } => [count, angle].iter().for_each(|e| nomes(e, &mut quer)),
        }
    }
    for f in &sub.features {
        match &f.kind {
            AsmFeatKind::Cut { clearance, .. } => nomes(clearance, &mut quer),
            AsmFeatKind::Hole { dia, depth, .. } => [dia, depth].iter().for_each(|e| nomes(e, &mut quer)),
            AsmFeatKind::Serie(s) => {
                nomes(&s.dia, &mut quer);
                nomes(&s.depth, &mut quer);
            }
        }
    }
    let mut vistos: HashSet<String> = HashSet::new();
    while let Some(n) = quer.pop() {
        if let Some(p) = a.params.iter().find(|p| p.name == n) {
            if vistos.insert(n) {
                nomes(&p.expr, &mut quer);
            }
        }
    }
    // a ordem da montagem importa: um parâmetro só enxerga os anteriores
    a.params.iter().filter(|p| vistos.contains(&p.name)).cloned().collect()
}

/// A parte de um estado de exibição que diz respeito às peças de dentro.
fn estado_de_dentro(s: &display::DisplayState, dentro: &HashSet<Id>) -> display::DisplayState {
    display::DisplayState {
        name: s.name.clone(),
        hidden: s.hidden.iter().copied().filter(|c| dentro.contains(c)).collect(),
        colors: s.colors.iter().copied().filter(|(c, _)| dentro.contains(c)).collect(),
        offsets: s.offsets.iter().copied().filter(|(c, _)| dentro.contains(c)).collect(),
        camera: s.camera,
    }
}

impl Assembly {
    /// Grupo fechado da extração: um padrão não se divide, então pedir o original ou uma cópia traz
    /// o padrão inteiro.
    fn fechar_selecao(&self, ids: &[Id]) -> HashSet<Id> {
        let mut dentro: HashSet<Id> = ids.iter().copied().filter(|c| self.node(*c).is_some_and(|n| !matches!(n.kind, NodeKind::Group))).collect();
        for p in &self.patterns {
            if dentro.contains(&p.source) || p.instances.iter().any(|i| dentro.contains(i)) {
                dentro.insert(p.source);
                dentro.extend(p.instances.iter().copied());
            }
        }
        dentro
    }

    /// Tira os componentes `ids` desta montagem para um subconjunto novo, gravado em `path`. Devolve
    /// o subconjunto, o nó criado aqui no lugar deles e o relatório.
    pub fn extract_subassembly(&mut self, ids: &[Id], name: &str, path: &str) -> (Assembly, Id, RelatorioExtracao) {
        let dentro = self.fechar_selecao(ids);
        let mut rel = RelatorioExtracao { componentes: dentro.len(), ..Default::default() };
        let pl_de: HashMap<Id, Placement> = self.nodes.iter().filter(|n| dentro.contains(&n.id)).map(|n| (n.id, n.placement)).collect();
        let fora = |c: Id| !dentro.contains(&c);

        // nó do subconjunto: entra onde estava a primeira peça, e fica fixo se alguma era fixa
        let pos = self.nodes.iter().position(|n| dentro.contains(&n.id)).unwrap_or(self.nodes.len());
        let mut pais = self.nodes.iter().filter(|n| dentro.contains(&n.id)).map(|n| n.parent);
        let pai = pais.next().flatten().filter(|p| pais.all(|q| q == Some(*p)));
        let no = self.alloc();
        let no_nome = self.unique(name);

        let mut sub = Assembly::new(name);
        sub.next_id = self.next_id;
        sub.relational = self.relational;

        // nós, com os grupos que os organizam
        let mut grupos: Vec<Id> = vec![];
        let mut g = self.nodes.iter().filter(|n| dentro.contains(&n.id)).filter_map(|n| n.parent).collect::<Vec<_>>();
        while let Some(id) = g.pop() {
            if !grupos.contains(&id) {
                grupos.push(id);
                g.extend(self.node(id).and_then(|n| n.parent));
            }
        }
        sub.nodes = self.nodes.iter().filter(|n| dentro.contains(&n.id) || grupos.contains(&n.id)).cloned().collect();

        // restrições
        for m in std::mem::take(&mut self.mates) {
            let (ia, ib) = (dentro.contains(&m.a.comp), dentro.contains(&m.b.comp));
            match (ia, ib) {
                (true, true) => {
                    rel.restricoes += 1;
                    sub.mates.push(m);
                }
                (false, false) => self.mates.push(m),
                _ => {
                    let mut n = m.clone();
                    religar_mate(&mut n, ia, no, &pl_de[if ia { &m.a.comp } else { &m.b.comp }]);
                    rel.religadas.push(m.name.clone());
                    self.religacoes.push(Religacao { sub: no, original: ItemOriginal::Mate(m) });
                    self.mates.push(n);
                }
            }
        }

        // juntas
        for j in std::mem::take(&mut self.joints) {
            let (ia, ib) = (dentro.contains(&j.a.comp), dentro.contains(&j.b.comp));
            match (ia, ib) {
                (true, true) => {
                    rel.juntas += 1;
                    sub.joints.push(j);
                }
                (false, false) => self.joints.push(j),
                _ => {
                    let mut n = j.clone();
                    religar_junta(&mut n, ia, no, &pl_de[if ia { &j.a.comp } else { &j.b.comp }]);
                    rel.religadas.push(j.name.clone());
                    self.religacoes.push(Religacao { sub: no, original: ItemOriginal::Joint(j) });
                    self.joints.push(n);
                }
            }
        }

        // acoplamentos: seguem as juntas; dividido entre os dois arquivos não tem como funcionar
        let junta_foi = |id: Id| sub.joints.iter().any(|j| j.id == id);
        for c in std::mem::take(&mut self.couplings) {
            match (junta_foi(c.a), junta_foi(c.b)) {
                (true, true) => {
                    rel.acoplamentos += 1;
                    sub.couplings.push(c);
                }
                (false, false) => self.couplings.push(c),
                _ => {
                    rel.perdidas.push(format!("{}: acoplamento entre uma junta de dentro e outra de fora", c.name));
                    self.religacoes.push(Religacao { sub: no, original: ItemOriginal::Coupling(c) });
                }
            }
        }

        // fixações conjuntas: o nó entra no lugar das peças que saíram
        for g in std::mem::take(&mut self.rigid) {
            let de_dentro: Vec<Id> = g.comps.iter().copied().filter(|c| dentro.contains(c)).collect();
            if de_dentro.is_empty() {
                self.rigid.push(g);
            } else if de_dentro.len() == g.comps.len() {
                rel.fixacoes += 1;
                sub.rigid.push(g);
            } else {
                if de_dentro.len() >= 2 {
                    sub.rigid.push(RigidGroup { id: g.id, name: g.name.clone(), comps: de_dentro });
                }
                let mut comps: Vec<Id> = g.comps.iter().copied().filter(|c| fora(*c)).collect();
                comps.push(no);
                rel.religadas.push(g.name.clone());
                self.rigid.push(RigidGroup { id: g.id, name: g.name.clone(), comps });
                self.religacoes.push(Religacao { sub: no, original: ItemOriginal::Rigid(g) });
            }
        }

        // padrões: já vieram inteiros
        for p in std::mem::take(&mut self.patterns) {
            if dentro.contains(&p.source) {
                rel.padroes += 1;
                sub.patterns.push(p);
            } else {
                self.patterns.push(p);
            }
        }

        // recursos entre peças
        for f in std::mem::take(&mut self.features) {
            let (de_dentro, de_fora): (Vec<Id>, Vec<Id>) = f.targets.iter().partition(|t| dentro.contains(t));
            if de_dentro.is_empty() && !matches!(f.kind, AsmFeatKind::Cut { tool, .. } if dentro.contains(&tool)) {
                self.features.push(f);
                continue;
            }
            // recorte: a ferramenta precisa estar no mesmo arquivo que o alvo
            let (vai, fica) = match f.kind {
                AsmFeatKind::Cut { tool, .. } if dentro.contains(&tool) => (de_dentro.clone(), vec![]),
                AsmFeatKind::Cut { .. } => (vec![], de_fora.clone()),
                AsmFeatKind::Hole { .. } | AsmFeatKind::Serie(_) => (de_dentro.clone(), de_fora.clone()),
            };
            // a série se divide e cada metade mora num arquivo: a âncora pode ter ficado do outro
            // lado, então as duas passam a cotas. O eixo fica onde estava
            let mut f = f;
            if let AsmFeatKind::Serie(s) = &mut f.kind {
                if let Some(pl) = s.eixo.ancora().and_then(|c| self.node(c).or(sub.node(c))).map(|n| n.placement) {
                    s.eixo.congelar(&pl);
                }
                s.gravados.clear();
            }
            if !vai.is_empty() {
                rel.recursos += 1;
                sub.features.push(AsmFeature { targets: vai.clone(), ..f.clone() });
            }
            if !fica.is_empty() {
                self.features.push(AsmFeature { targets: fica.clone(), ..f.clone() });
            }
            if vai.len() + fica.len() < f.targets.len() || vai.is_empty() && fica.is_empty() {
                rel.perdidas.push(format!("{}: recorte com a ferramenta e o alvo em lados opostos", f.name));
            }
            if vai.len() != f.targets.len() {
                self.religacoes.push(Religacao { sub: no, original: ItemOriginal::Feature(f) });
            }
        }

        // parâmetros, números de item e políticas de link das peças que foram
        sub.params = parametros_usados(self, &sub);
        rel.parametros = sub.params.len();
        let caminhos: HashSet<String> = sub.nodes.iter().filter_map(|n| match &n.kind {
            NodeKind::Part { path } | NodeKind::SubAssembly { path } => Some(path.clone()),
            NodeKind::Group => None,
        }).collect();
        sub.items = self.items.iter().filter(|i| caminhos.contains(&i.key)).cloned().collect();
        sub.politicas_link = self.politicas_link.iter().filter(|p| caminhos.contains(&p.alvo)).cloned().collect();

        // estados de exibição e animações: o subconjunto leva a parte dele; aqui em cima as entradas
        // antigas não atrapalham e fazem a volta ser exata. O nó oculta se tudo dentro estava oculto.
        for s in &mut self.display_states {
            let d = estado_de_dentro(s, &dentro);
            if dentro.iter().all(|c| s.hidden.contains(c)) {
                s.hidden.push(no);
            }
            sub.display_states.push(d);
        }
        rel.estados = sub.display_states.len();
        sub.animations = self.animations.iter().map(|a| display::Animation { name: a.name.clone(), keys: a.keys.iter().map(|k| display::Key { t: k.t, state: estado_de_dentro(&k.state, &dentro) }).collect() }).collect();

        let fixo = sub.nodes.iter().any(|n| n.fixed);
        let numero = sub.nodes.iter().map(|n| n.number).filter(|n| *n > 0).min().unwrap_or(0);
        self.nodes.retain(|n| !dentro.contains(&n.id));
        // os grupos seguem aqui para as peças que ficaram; sumindo a última, ficam vazios e inofensivos
        let pos = pos.min(self.nodes.len());
        self.nodes.insert(pos, Node { id: no, name: no_nome, number: numero, kind: NodeKind::SubAssembly { path: path.into() }, parent: pai, placement: Placement::default(), fixed: fixo, visible: true, mode: LoadMode::Editavel, color: None });
        (sub, no, rel)
    }

    /// Reescreve os caminhos de arquivo (peças, números de item e políticas de link): usado quando o
    /// documento muda de pasta.
    pub fn trocar_caminhos(&mut self, f: impl Fn(&str) -> String) {
        for n in &mut self.nodes {
            if let NodeKind::Part { path } | NodeKind::SubAssembly { path } = &mut n.kind {
                *path = f(path);
            }
        }
        for i in &mut self.items {
            i.key = f(&i.key);
        }
        for p in &mut self.politicas_link {
            p.alvo = f(&p.alvo);
        }
    }

    /// Troca os ids de `sub` que colidem com os desta montagem. O subconjunto pode ter ganhado itens
    /// depois de extraído, e os dois lados distribuem ids sozinhos.
    fn acomodar_ids(&mut self, sub: &mut Assembly) {
        self.next_id = self.next_id.max(sub.next_id);
        let mut mapa: HashMap<Id, Id> = HashMap::new();
        let ocupados: Vec<Id> = sub.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| n.id).chain(sub.mates.iter().map(|m| m.id)).chain(sub.joints.iter().map(|j| j.id)).chain(sub.couplings.iter().map(|c| c.id)).chain(sub.patterns.iter().map(|p| p.id)).collect();
        for id in ocupados {
            if self.has_item(id) {
                let novo = self.alloc();
                mapa.insert(id, novo);
            }
        }
        if mapa.is_empty() {
            return;
        }
        let t = |id: &mut Id| {
            if let Some(n) = mapa.get(id) {
                *id = *n;
            }
        };
        for n in &mut sub.nodes {
            t(&mut n.id);
            if let Some(p) = &mut n.parent {
                t(p);
            }
        }
        for m in &mut sub.mates {
            t(&mut m.id);
            t(&mut m.a.comp);
            t(&mut m.b.comp);
        }
        for j in &mut sub.joints {
            t(&mut j.id);
            t(&mut j.a.comp);
            t(&mut j.b.comp);
        }
        for c in &mut sub.couplings {
            t(&mut c.id);
            t(&mut c.a);
            t(&mut c.b);
        }
        for g in &mut sub.rigid {
            g.comps.iter_mut().for_each(t);
        }
        for p in &mut sub.patterns {
            t(&mut p.id);
            t(&mut p.source);
            p.instances.iter_mut().for_each(t);
        }
        for f in &mut sub.features {
            f.targets.iter_mut().for_each(t);
            if let AsmFeatKind::Cut { tool, .. } = &mut f.kind {
                t(tool);
            }
        }
        for s in sub.display_states.iter_mut().chain(sub.animations.iter_mut().flat_map(|a| a.keys.iter_mut().map(|k| &mut k.state))) {
            s.hidden.iter_mut().for_each(t);
            s.colors.iter_mut().for_each(|(c, _)| t(c));
            s.offsets.iter_mut().for_each(|(c, _)| t(c));
        }
    }

    /// Desfaz o subconjunto `no`: as peças de `sub` voltam para esta montagem, na mesma posição no
    /// mundo, e o que foi religado volta às peças originais. Os caminhos de `sub` já devem estar
    /// relativos a esta montagem ([`Assembly::trocar_caminhos`]).
    pub fn inserir_de_volta(&mut self, no: Id, sub: &Assembly) -> Result<RelatorioExtracao, String> {
        let nd = self.node(no).cloned().ok_or("componente não encontrado")?;
        if !matches!(nd.kind, NodeKind::SubAssembly { .. }) {
            return Err("o componente não é um subconjunto".into());
        }
        let mut sub = sub.clone();
        self.acomodar_ids(&mut sub);
        let p = nd.placement;
        let mut rel = RelatorioExtracao::default();

        // o que estava no sistema do subconjunto passa para o desta montagem
        for n in &mut sub.nodes {
            n.placement = p.then(&n.placement);
        }
        for pt in &mut sub.patterns {
            match &mut pt.kind {
                PatternKind::Linear { dir, .. } => *dir = p.dir(*dir),
                PatternKind::Circular { origin, axis, .. } => {
                    *origin = p.point(*origin);
                    *axis = p.dir(*axis);
                }
            }
        }
        for f in &mut sub.features {
            match &mut f.kind {
                AsmFeatKind::Hole { p: o, d, .. } => {
                    *o = p.point(*o);
                    *d = p.dir(*d);
                }
                AsmFeatKind::Serie(s) => {
                    if let EixoFuro::Cotas { x, y, z, d } = &mut s.eixo {
                        let num = |e: &str| expr::eval_str(e, &HashMap::new()).unwrap_or(0.);
                        let [nx, ny, nz] = p.point([num(x), num(y), num(z)]).map(valor_cota);
                        (*x, *y, *z, *d) = (nx, ny, nz, p.dir(*d));
                    }
                }
                AsmFeatKind::Cut { .. } => {}
            }
        }
        for m in &mut sub.mates {
            match &mut m.kind {
                MateKind::Gear { refs, .. } => {
                    refs[1] = p.dir(refs[1]);
                    refs[3] = p.dir(refs[3]);
                }
                MateKind::RackPinion { refs, .. } => {
                    refs[1] = p.dir(refs[1]);
                    refs[2] = p.point(refs[2]);
                }
                _ => {}
            }
        }

        // peças e grupos: na posição do nó
        let pos = self.nodes.iter().position(|n| n.id == no).unwrap_or(self.nodes.len());
        self.nodes.retain(|n| n.id != no);
        let novos: Vec<Node> = sub.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group) || self.node(n.id).is_none()).cloned().collect();
        rel.componentes = novos.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count();
        let vivo = |id: Id| sub.nodes.iter().any(|n| n.id == id);
        for (k, n) in novos.into_iter().enumerate() {
            self.nodes.insert((pos + k).min(self.nodes.len()), n);
        }

        // o que foi religado volta para a peça de origem, com o que mudou enquanto esteve no nó
        let (restauradas, resto): (Vec<Religacao>, Vec<Religacao>) = std::mem::take(&mut self.religacoes).into_iter().partition(|r| r.sub == no);
        self.religacoes = resto;
        let mut features_restauradas: Vec<Id> = vec![];
        for r in restauradas {
            match r.original {
                ItemOriginal::Mate(o) => {
                    let Some(i) = self.mates.iter().position(|m| m.id == o.id) else { continue };
                    let lado_perdido = (self.mates[i].a.comp == no && !vivo(o.a.comp)) || (self.mates[i].b.comp == no && !vivo(o.b.comp));
                    if lado_perdido {
                        rel.perdidas.push(format!("{}: a peça saiu do subconjunto", o.name));
                        self.mates.remove(i);
                        continue;
                    }
                    let m = &mut self.mates[i];
                    if m.a.comp == no {
                        m.a = o.a;
                    }
                    if m.b.comp == no {
                        m.b = o.b;
                    }
                    if m.kind.is_motion() {
                        m.kind = o.kind;
                    }
                    rel.religadas.push(o.name);
                }
                ItemOriginal::Joint(o) => {
                    let Some(i) = self.joints.iter().position(|j| j.id == o.id) else { continue };
                    let j = &mut self.joints[i];
                    if j.a.comp == no {
                        j.a = o.a;
                        j.kind = o.kind.clone();
                    }
                    if j.b.comp == no {
                        j.b = o.b;
                    }
                    rel.religadas.push(o.name);
                }
                ItemOriginal::Rigid(o) => {
                    sub.rigid.retain(|g| g.id != o.id);
                    self.rigid.retain(|g| g.id != o.id);
                    rel.religadas.push(o.name.clone());
                    self.rigid.push(o);
                }
                ItemOriginal::Coupling(o) => {
                    rel.religadas.push(o.name.clone());
                    self.couplings.push(o);
                }
                ItemOriginal::Feature(o) => {
                    features_restauradas.push(o.id);
                    self.features.retain(|f| f.id != o.id);
                    rel.religadas.push(o.name.clone());
                    self.features.push(o);
                }
            }
        }

        // ligação feita no nó depois de extrair: não há peça de origem, vai para a primeira
        let primeira = sub.nodes.iter().find(|n| !matches!(n.kind, NodeKind::Group)).map(|n| (n.id, n.placement));
        for m in &mut self.mates {
            for r in [&mut m.a, &mut m.b] {
                if r.comp == no {
                    match primeira {
                        Some((id, pl)) => {
                            *r = MateRef { comp: id, geom: geom_em(&r.geom, &pl.inverse().then(&p)), pick: None };
                            rel.perdidas.push(format!("{}: criada no subconjunto, foi para a primeira peça", m.name));
                        }
                        None => rel.perdidas.push(format!("{}: subconjunto vazio", m.name)),
                    }
                }
            }
        }
        self.mates.retain(|m| m.a.comp != no && m.b.comp != no);
        self.joints.retain(|j| j.a.comp != no && j.b.comp != no);
        for g in &mut self.rigid {
            g.comps.retain(|c| *c != no);
        }

        // o que estava dentro
        rel.restricoes = sub.mates.len();
        rel.juntas = sub.joints.len();
        rel.acoplamentos = sub.couplings.len();
        rel.padroes = sub.patterns.len();
        rel.fixacoes = sub.rigid.len();
        self.mates.append(&mut sub.mates);
        self.joints.append(&mut sub.joints);
        self.couplings.append(&mut sub.couplings);
        self.patterns.append(&mut sub.patterns);
        self.rigid.append(&mut sub.rigid);
        sub.features.retain(|f| !features_restauradas.contains(&f.id));
        rel.recursos = sub.features.len();
        self.features.append(&mut sub.features);

        // parâmetros: vale o desta montagem, e o conflito é avisado
        for q in sub.params.drain(..) {
            match self.params.iter().find(|x| x.name == q.name) {
                Some(x) if x.expr != q.expr => rel.perdidas.push(format!("{}: parâmetro ficou com o valor desta montagem ({})", q.name, x.expr)),
                Some(_) => {}
                None => {
                    rel.parametros += 1;
                    self.params.push(q);
                }
            }
        }
        for i in sub.items.drain(..) {
            if !self.items.iter().any(|x| x.key == i.key) {
                self.items.push(i);
            }
        }
        for q in sub.politicas_link.drain(..) {
            if !self.politicas_link.iter().any(|x| x.alvo == q.alvo && x.ext == q.ext) {
                self.politicas_link.push(q);
            }
        }

        // estados: soma ao de mesmo nome, sem repetir
        for s in &mut self.display_states {
            s.hidden.retain(|c| *c != no);
        }
        for s in sub.display_states.drain(..) {
            rel.estados += 1;
            match self.display_states.iter_mut().find(|x| x.name == s.name) {
                Some(x) => {
                    for c in s.hidden {
                        if !x.hidden.contains(&c) {
                            x.hidden.push(c);
                        }
                    }
                    for c in s.colors {
                        if !x.colors.iter().any(|y| y.0 == c.0) {
                            x.colors.push(c);
                        }
                    }
                    for c in s.offsets {
                        if !x.offsets.iter().any(|y| y.0 == c.0) {
                            x.offsets.push(c);
                        }
                    }
                }
                None => self.display_states.push(s),
            }
        }
        for a in sub.animations.drain(..) {
            if !self.animations.iter().any(|x| x.name == a.name) {
                self.animations.push(a);
            }
        }
        Ok(rel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kin::CouplingKind;

    fn z() -> Geom {
        Geom::Axis { p: [0.; 3], d: [0., 0., 1.] }
    }

    fn topo() -> Geom {
        Geom::Plane { p: [5., 5., 10.], n: [0., 0., 1.] }
    }

    fn base() -> Geom {
        Geom::Plane { p: [5., 5., 0.], n: [0., 0., -1.] }
    }

    fn comp(a: &mut Assembly, nome: &str, o: V3) -> Id {
        a.add_component(nome, NodeKind::Part { path: format!("{nome}.fkp") }, Placement { origin: o, ..Default::default() }, None)
    }

    fn r(comp: Id, geom: Geom) -> MateRef {
        MateRef { comp, geom, pick: None }
    }

    fn junta(a: &mut Assembly, nome: &str, ca: Id, cb: Id, f: JFrame) -> Id {
        let id = a.alloc();
        a.joints.push(Joint { id, name: nome.into(), kind: JointKind::Revolute, a: JRef { comp: ca, frame: f }, b: JRef { comp: cb, frame: f }, angle0: 0., len0: 0., drive_angle: None, drive_len: None, suppressed: false });
        id
    }

    /// Base fixa, depois A e B (que serão extraídos, A girado) e C (fica). Tudo o que a extração
    /// precisa levar, religar ou recusar.
    fn conjunto() -> (Assembly, Id, Id, Id, Id) {
        let mut a = Assembly::new("Geral");
        a.params.push(AsmParam { name: "base".into(), expr: "5".into() });
        a.params.push(AsmParam { name: "folga".into(), expr: "base - 1".into() });
        a.params.push(AsmParam { name: "alheio".into(), expr: "99".into() });
        let g = a.add_group("Grupo", None);
        let base_c = comp(&mut a, "Base", [0.; 3]);
        let ca = a.add_component("A", NodeKind::Part { path: "A.fkp".into() }, Placement::default().moved([0., 0., 10.], [0., 0., 0.4], [0.; 3]), Some(g));
        let cb = a.add_component("B", NodeKind::Part { path: "B.fkp".into() }, Placement { origin: [0., 0., 24.], ..Default::default() }, Some(g));
        let cc = comp(&mut a, "C", [40., 0., 0.]);

        // de dentro
        a.add_mate(MateKind::Distance("folga".into()), r(ca, topo()), r(cb, base()));
        a.add_mate(MateKind::Coincident { flip: false }, r(ca, z()), r(cb, z()));
        let jd = junta(&mut a, "Junta dentro", ca, cb, JFrame::from_axis([0.; 3], [0., 0., 1.]));
        let jd2 = junta(&mut a, "Junta dentro 2", cb, ca, JFrame::from_axis([1., 0., 0.], [0., 0., 1.]));
        let id = a.alloc();
        a.couplings.push(Coupling { id, name: "Acoplamento dentro".into(), kind: CouplingKind::Gear, a: jd, b: jd2, ratio: "folga".into() });
        a.add_feature(AsmFeatKind::Hole { p: [0., 0., 30.], d: [0., 0., -1.], dia: "folga".into(), depth: String::new() }, vec![ca, cb]);
        a.add_pattern(cb, PatternKind::Linear { dir: [1., 0., 0.], count: "3".into(), spacing: "20".into() });

        // com o exterior
        a.add_mate(MateKind::Contact, r(base_c, topo()), r(ca, base()));
        a.add_mate(MateKind::Coincident { flip: false }, r(base_c, z()), r(ca, z()));
        let jf = junta(&mut a, "Junta fora", base_c, ca, JFrame::from_axis([0.; 3], [0., 0., 1.]));
        let id = a.alloc();
        a.couplings.push(Coupling { id, name: "Acoplamento cruzado".into(), kind: CouplingKind::Gear, a: jd, b: jf, ratio: "1".into() });
        a.add_rigid(vec![ca, cc]);
        a.add_feature(AsmFeatKind::Cut { tool: cc, clearance: "0".into() }, vec![cb]);

        a.display_states.push(display::DisplayState { name: "Limpo".into(), hidden: vec![ca, cc], colors: vec![(cb, [9, 9, 9])], offsets: vec![(ca, [0., 0., 5.])], camera: None });
        a.animations.push(display::Animation { name: "Giro".into(), keys: vec![display::Key { t: 1., state: display::DisplayState { name: "k".into(), hidden: vec![cb], ..Default::default() } }] });
        a.items.push(bom::ItemNo { key: "A.fkp".into(), no: 7 });
        a.items.push(bom::ItemNo { key: "C.fkp".into(), no: 8 });
        a.politicas_link.push(links::PoliticaLink { alvo: "A.fkp".into(), ext: Id(99), politica: links::Politica::Manual });
        (a, base_c, ca, cb, cc)
    }

    fn nomes_de<T>(v: &[T], f: impl Fn(&T) -> String) -> Vec<String> {
        let mut n: Vec<String> = v.iter().map(f).collect();
        n.sort();
        n
    }

    fn origem(a: &Assembly, c: Id) -> V3 {
        a.node(c).unwrap().placement.origin
    }

    #[test]
    fn extrair_leva_tudo_que_e_de_dentro() {
        let (mut a, _, ca, cb, _) = conjunto();
        let (sub, no, rel) = a.extract_subassembly(&[ca, cb], "Sub", "sub.fka");

        // o padrão vem inteiro: B e duas cópias, além de A; e o grupo que organizava
        assert_eq!(rel.componentes, 4, "{rel:?}");
        assert_eq!(sub.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Group)).count(), 1);
        assert_eq!(sub.node(ca).unwrap().parent, sub.nodes.iter().find(|n| n.name == "Grupo").map(|n| n.id));
        assert_eq!((rel.restricoes, rel.juntas, rel.acoplamentos, rel.padroes, rel.recursos), (2, 2, 1, 1, 1), "{rel:?}");
        assert_eq!(sub.patterns[0].instances.len(), 2);
        assert_eq!(sub.features.len(), 1);
        assert_eq!(sub.features[0].targets, vec![ca, cb]);

        // só os parâmetros usados, com a dependência e na ordem
        assert_eq!(sub.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["base", "folga"]);
        assert_eq!(sub.vars()["folga"], 4.);

        // estados e números que dizem respeito às peças
        assert_eq!(sub.display_states[0].hidden, vec![ca]);
        assert_eq!(sub.display_states[0].colors, vec![(cb, [9, 9, 9])]);
        assert_eq!(sub.animations[0].keys[0].state.hidden, vec![cb]);
        assert_eq!(sub.items.len(), 1);
        assert_eq!(sub.politicas_link.len(), 1);

        // o que ficou em cima
        assert!(matches!(&a.node(no).unwrap().kind, NodeKind::SubAssembly { path } if path == "sub.fka"));
        assert!(a.node(ca).is_none() && a.node(cb).is_none());
        assert!(a.patterns.is_empty());
    }

    #[test]
    fn restricoes_com_o_exterior_ficam_no_no_e_nada_se_move() {
        let (mut a, base_c, ca, cb, cc) = conjunto();
        a.node_mut(base_c).unwrap().fixed = true;
        a.mates.retain(|m| !matches!(m.kind, MateKind::Distance(_)));
        let antes = a.node(ca).unwrap().placement;
        let (sub, no, rel) = a.extract_subassembly(&[ca, cb], "Sub", "sub.fka");
        assert_eq!(nomes_de(&rel.religadas, |s| s.clone()), ["Coincidência.2", "Contato", "Fixação conjunta", "Junta fora"]);

        // os lados de dentro apontam para o nó, com a geometria no mundo
        let m = a.mates.iter().find(|m| m.name == "Contato").unwrap();
        assert_eq!(m.b.comp, no);
        let Geom::Plane { p, n } = m.b.geom else { panic!() };
        assert!(norm(sub_v(p, antes.point([5., 5., 0.]))) < 1e-12);
        assert!(norm(sub_v(n, [0., 0., -1.])) < 1e-12);
        let j = a.joints.iter().find(|j| j.name == "Junta fora").unwrap();
        assert_eq!(j.b.comp, no);
        assert!(norm(sub_v(j.b.frame.o, antes.point([0.; 3]))) < 1e-12);

        // a fixação com C agora prende o nó
        let g = a.rigid.iter().find(|g| g.name == "Fixação conjunta").unwrap();
        assert!(g.comps.contains(&no) && g.comps.contains(&cc) && g.comps.len() == 2);

        // a peça de dentro não se move no subconjunto, e o nó fica onde está
        assert_eq!(sub.node(ca).unwrap().placement, antes);
        assert_eq!(a.node(no).unwrap().placement, Placement::default());
    }

    #[test]
    fn o_no_herda_os_graus_de_liberdade_da_peca() {
        let mut a = Assembly::new("Pilha");
        let f = comp(&mut a, "F", [0.; 3]);
        let c = comp(&mut a, "P", [0., 0., 10.]);
        a.add_mate(MateKind::Contact, r(f, topo()), r(c, base()));
        a.add_mate(MateKind::Coincident { flip: false }, r(f, z()), r(c, z()));
        assert!(solve::solve(&mut a).converged);
        assert_eq!(solve::dof_per_comp(&a)[&c], 1, "só gira");
        let antes = a.node(c).unwrap().placement;

        let (sub, no, rel) = a.extract_subassembly(&[c], "Sub", "sub.fka");
        assert_eq!(rel.religadas.len(), 2);
        let rep = solve::solve(&mut a);
        assert!(rep.converged, "{rep:?}");
        assert_eq!(solve::dof_per_comp(&a)[&no], 1, "continua só girando");
        // no mundo a peça segue onde estava: o nó na origem, a peça com a posição de antes
        let mundo = a.node(no).unwrap().placement.then(&sub.node(c).unwrap().placement);
        assert!(norm(crate::sub(mundo.origin, antes.origin)) < 1e-9, "{mundo:?}");
    }

    fn sub_v(a: V3, b: V3) -> V3 {
        crate::sub(a, b)
    }

    #[test]
    fn o_que_nao_atravessa_a_fronteira_e_relatado() {
        let (mut a, _, ca, cb, _) = conjunto();
        let (_, _, rel) = a.extract_subassembly(&[ca, cb], "Sub", "sub.fka");
        let txt = rel.perdidas.join(" | ");
        assert!(txt.contains("Acoplamento cruzado") && txt.contains("Recorte pela peça"), "{txt}");
        assert_eq!(rel.perdidas.len(), 2, "{txt}");
        assert!(a.couplings.is_empty());
        assert!(a.features.is_empty());
    }

    #[test]
    fn furo_entre_pecas_de_dentro_e_de_fora_se_divide() {
        let mut a = Assembly::new("Furo");
        let c1 = comp(&mut a, "P1", [0.; 3]);
        let c2 = comp(&mut a, "P2", [0., 0., 10.]);
        let f = a.add_feature(AsmFeatKind::Hole { p: [0., 0., 30.], d: [0., 0., -1.], dia: "6".into(), depth: String::new() }, vec![c1, c2]);
        let (sub, no, rel) = a.extract_subassembly(&[c2], "Sub", "sub.fka");
        assert!(rel.perdidas.is_empty(), "{rel:?}");
        assert_eq!(a.features[0].targets, vec![c1]);
        assert_eq!(sub.features[0].targets, vec![c2]);
        a.inserir_de_volta(no, &sub).unwrap();
        assert_eq!(a.features.len(), 1);
        assert_eq!(a.features[0].id, f);
        assert_eq!(a.features[0].targets, vec![c1, c2]);
    }

    #[test]
    fn extrair_e_inserir_de_volta_da_o_mesmo_resultado() {
        let (original, _, ca, cb, _) = conjunto();
        // o JSON arredonda o último dígito: compara documentos que passaram pelo arquivo
        let original = Assembly::from_json(&original.to_json()).unwrap();
        let mut a = original.clone();
        let (sub, no, _) = a.extract_subassembly(&[ca, cb], "Sub", "sub.fka");
        // passa pelo arquivo, como no aplicativo
        let sub = Assembly::from_json(&sub.to_json()).unwrap();
        let a2 = Assembly::from_json(&a.to_json()).unwrap();
        let mut a = a2;
        let rel = a.inserir_de_volta(no, &sub).unwrap();
        assert!(rel.perdidas.is_empty(), "{:?}", rel.perdidas);

        let ordena = |x: &Assembly| {
            let mut y = x.clone();
            y.next_id = 0;
            y.nodes.sort_by_key(|n| n.id.0);
            y.mates.sort_by_key(|n| n.id.0);
            y.joints.sort_by_key(|n| n.id.0);
            y.couplings.sort_by_key(|n| n.id.0);
            y.rigid.sort_by_key(|n| n.id.0);
            y.patterns.sort_by_key(|n| n.id.0);
            y.features.sort_by_key(|n| n.id.0);
            y.items.sort_by_key(|i| i.no);
            for s in &mut y.display_states {
                s.hidden.sort_by_key(|c| c.0);
                s.colors.sort_by_key(|c| c.0 .0);
                s.offsets.sort_by_key(|c| c.0 .0);
            }
            y
        };
        let (x, y) = (ordena(&original), ordena(&a));
        assert_eq!(x.nodes, y.nodes);
        assert_eq!(x.mates, y.mates);
        assert_eq!(x.joints, y.joints);
        assert_eq!(x.couplings, y.couplings);
        assert_eq!(x.rigid, y.rigid);
        assert_eq!(x.patterns, y.patterns);
        assert_eq!(x.features, y.features);
        assert_eq!(x.params, y.params);
        assert_eq!(x.display_states, y.display_states);
        assert_eq!(x.animations, y.animations);
        assert_eq!(x.items, y.items);
        assert_eq!(x.politicas_link, y.politicas_link);
        assert!(y.religacoes.is_empty());
        assert_eq!(x, y);
    }

    #[test]
    fn subconjunto_movido_leva_tudo_de_volta_para_a_posicao_nova() {
        let mut a = Assembly::new("Movido");
        let fixa = comp(&mut a, "F", [0.; 3]);
        let c = comp(&mut a, "P", [10., 0., 0.]);
        a.add_pattern(c, PatternKind::Circular { origin: [0.; 3], axis: [0., 0., 1.], count: "2".into(), angle: "360".into() });
        let (sub, no, _) = a.extract_subassembly(&[c], "Sub", "sub.fka");
        let p = Placement::default().moved([0., 0., 50.], [0., 0., std::f64::consts::FRAC_PI_2], [0.; 3]);
        a.node_mut(no).unwrap().placement = p;
        a.inserir_de_volta(no, &sub).unwrap();
        assert!(norm(crate::sub(origem(&a, c), [0., 10., 50.])) < 1e-9, "{:?}", origem(&a, c));
        // o padrão acompanha: refeito, a cópia fica do outro lado do eixo, na altura nova
        a.update_patterns();
        let copia = a.patterns[0].instances[0];
        assert!(norm(crate::sub(origem(&a, copia), [0., -10., 50.])) < 1e-9, "{:?}", origem(&a, copia));
        assert!(a.node(fixa).is_some() && a.node(no).is_none());
    }

    #[test]
    fn ids_novos_nos_dois_lados_nao_colidem_ao_voltar() {
        let mut a = Assembly::new("Colisão");
        let f = comp(&mut a, "F", [0.; 3]);
        let c = comp(&mut a, "P", [10., 0., 0.]);
        let (mut sub, no, _) = a.extract_subassembly(&[c], "Sub", "sub.fka");
        // cada lado ganhou uma peça depois de extrair: os dois distribuem o mesmo próximo id
        let aqui = comp(&mut a, "Nova aqui", [0., 20., 0.]);
        let la = comp(&mut sub, "Nova lá", [0., 30., 0.]);
        assert_eq!(aqui, la);
        let m = sub.add_mate(MateKind::Coincident { flip: false }, r(c, z()), r(la, z()));
        a.inserir_de_volta(no, &sub).unwrap();
        assert_eq!(a.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).count(), 4);
        let ids: HashSet<Id> = a.nodes.iter().map(|n| n.id).collect();
        assert_eq!(ids.len(), 4);
        assert_eq!(a.node(aqui).unwrap().name, "Nova aqui");
        let nova = a.nodes.iter().find(|n| n.name == "Nova lá").unwrap().id;
        assert_ne!(nova, aqui);
        let mm = a.mates.iter().find(|x| x.b.comp == nova).expect("a restrição segue a peça");
        assert_eq!(mm.a.comp, c);
        let _ = (f, m);
    }

    #[test]
    fn peca_fixa_deixa_o_no_fixo_e_parametro_em_conflito_e_avisado() {
        let mut a = Assembly::new("Fixa");
        let f = comp(&mut a, "F", [0.; 3]);
        let c = comp(&mut a, "P", [10., 0., 0.]);
        a.params.push(AsmParam { name: "d".into(), expr: "5".into() });
        a.add_mate(MateKind::Distance("d".into()), r(f, topo()), r(c, base()));
        let (sub, no, _) = a.extract_subassembly(&[f, c], "Sub", "sub.fka");
        assert!(a.node(no).unwrap().fixed);
        a.params[0].expr = "6".into();
        let rel = a.inserir_de_volta(no, &sub).unwrap();
        assert_eq!(rel.perdidas.len(), 1, "{rel:?}");
        assert_eq!(a.params[0].expr, "6");
        assert!(a.inserir_de_volta(f, &sub).is_err(), "só subconjunto volta");
    }

    #[test]
    fn caminhos_trocam_em_nos_itens_e_politicas() {
        let (mut a, _, ca, cb, _) = conjunto();
        let (mut sub, _, _) = a.extract_subassembly(&[ca, cb], "Sub", "sub.fka");
        sub.trocar_caminhos(|p| format!("x/{p}"));
        assert!(matches!(&sub.node(ca).unwrap().kind, NodeKind::Part { path } if path == "x/A.fkp"));
        assert_eq!(sub.items[0].key, "x/A.fkp");
        assert_eq!(sub.politicas_link[0].alvo, "x/A.fkp");
    }
}
