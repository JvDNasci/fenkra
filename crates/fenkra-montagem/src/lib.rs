// SPDX-License-Identifier: LGPL-2.1-or-later
//! Montagem: árvore de componentes (grupos, peças, subconjuntos), posicionamento por restrições
//! e solucionador de graus de liberdade.
//!
//! - [`Assembly`]: documento `.fka` (serializado em JSON) com nós e restrições.
//! - [`solve`]: Levenberg–Marquardt sobre 6 GDL por corpo livre (componentes fixos não se movem;
//!   uma fixação conjunta faz várias peças se moverem como um corpo só).
//! - [`load`]: carrega e calcula as peças referenciadas (com cache por arquivo).

pub mod avulso;
pub mod bom;
pub mod conflito;
pub mod contexto;
pub mod display;
pub mod encaixe;
pub mod engrenagens;
pub mod estrutura;
pub mod fixadores;
pub mod furos;
pub mod geometria;
pub mod insercao;
pub mod interferencia;
pub mod kin;
pub mod links;
pub mod load;
pub mod lod;
pub mod manipulador;
pub mod pilhas;
pub mod proposta;
pub mod publicacoes;
pub mod repeticao;
pub mod simbolos;
pub mod substituicao;
pub mod view;
pub mod solve;
pub mod subconjunto;

pub use engrenagens::{calculo_par, inserir_par_engrenagens, CalculoPar, RelatorioEngrenagens};
pub use estrutura::{estrutura_para_montagem, plano_do_documento, Gerado, GrupoGerado, GrupoPlano, Plano};
pub use fixadores::{preencher_com, preencher_furos, EstadoFuro, MarcaFuro, OpcoesFixadores, RelatorioFixadores};
pub use furos::{EixoFuro, FuroPeca, SerieFuros};
pub use subconjunto::{Religacao, RelatorioExtracao};

use fenkra_core::{expr, Id, Properties};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type V3 = [f64; 3];

/// Id reservado da origem da montagem: uma restrição ou junta pode usá-la no lugar de um componente.
/// Ela não é um nó da árvore: não se move, não entra na lista de peças e não tem arquivo. Os ids dos
/// nós começam em 1.
pub const ORIGEM: Id = Id(0);

/// Posição de um componente: origem e rotação (colunas = eixos locais no mundo).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Placement {
    pub origin: V3,
    pub rot: [[f64; 3]; 3],
}

impl Default for Placement {
    fn default() -> Self {
        Placement { origin: [0.; 3], rot: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] }
    }
}

impl Placement {
    /// Ponto local → mundo.
    pub fn point(&self, p: V3) -> V3 {
        let r = &self.rot;
        [0, 1, 2].map(|i| self.origin[i] + r[0][i] * p[0] + r[1][i] * p[1] + r[2][i] * p[2])
    }
    /// Direção local → mundo.
    pub fn dir(&self, v: V3) -> V3 {
        let r = &self.rot;
        [0, 1, 2].map(|i| r[0][i] * v[0] + r[1][i] * v[1] + r[2][i] * v[2])
    }
    /// Ponto do mundo → local.
    pub fn to_local(&self, p: V3) -> V3 {
        let d = [p[0] - self.origin[0], p[1] - self.origin[1], p[2] - self.origin[2]];
        self.rot.map(|c| c[0] * d[0] + c[1] * d[1] + c[2] * d[2])
    }
    /// Direção do mundo → local.
    pub fn dir_to_local(&self, v: V3) -> V3 {
        self.rot.map(|c| c[0] * v[0] + c[1] * v[1] + c[2] * v[2])
    }
    /// Gira em torno do ponto `about` pelo vetor de rotação `w` (eixo × ângulo em radianos) e translada.
    pub fn moved(&self, t: V3, w: V3, about: V3) -> Placement {
        let rot_v = |v: V3| rotvec(v, w);
        let o = [0, 1, 2].map(|i| self.origin[i] - about[i]);
        let o = rot_v(o);
        Placement { origin: [0, 1, 2].map(|i| o[i] + about[i] + t[i]), rot: self.rot.map(rot_v) }
    }
    /// Composição: `self` aplicado depois de `inner` (inner em coordenadas deste).
    pub fn then(&self, inner: &Placement) -> Placement {
        Placement { origin: self.point(inner.origin), rot: inner.rot.map(|c| self.dir(c)) }
    }
    /// Inversa.
    pub fn inverse(&self) -> Placement {
        let r = &self.rot;
        let rt = [[r[0][0], r[1][0], r[2][0]], [r[0][1], r[1][1], r[2][1]], [r[0][2], r[1][2], r[2][2]]];
        let inv = Placement { origin: [0.; 3], rot: rt };
        let o = inv.dir(self.origin);
        Placement { origin: [-o[0], -o[1], -o[2]], rot: rt }
    }
}

/// Rotação de `v` pelo vetor de rotação `w` (Rodrigues).
pub fn rotvec(v: V3, w: V3) -> V3 {
    let a = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
    if a < 1e-15 {
        return v;
    }
    let k = [w[0] / a, w[1] / a, w[2] / a];
    let (c, s) = (a.cos(), a.sin());
    let kv = cross(k, v);
    let kd = dot(k, v);
    [0, 1, 2].map(|i| v[i] * c + kv[i] * s + k[i] * kd * (1. - c))
}

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn unit(a: V3) -> V3 {
    let n = norm(a).max(1e-300);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Tipo de nó da árvore da montagem.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum NodeKind {
    /// elemento vazio só para organizar
    Group,
    /// peça (arquivo `.fkp`, relativo à pasta da montagem ou absoluto)
    Part { path: String },
    /// subconjunto (arquivo `.fka`)
    SubAssembly { path: String },
}

/// Como o componente é carregado: editável (geometria completa) ou só visualização (leve).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LoadMode {
    #[default]
    Editavel,
    Visualizacao,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub name: String,
    /// número sequencial de identificação (numeração automática; 0 = sem número)
    #[serde(default)]
    pub number: u32,
    pub kind: NodeKind,
    /// grupo/subconjunto pai na árvore (só organização: não muda a geometria)
    #[serde(default)]
    pub parent: Option<Id>,
    #[serde(default)]
    pub placement: Placement,
    /// fixo: referência global, sem graus de liberdade
    #[serde(default)]
    pub fixed: bool,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub mode: LoadMode,
    /// cor de destaque do componente (sobrepõe a da peça)
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    /// versão simplificada: o componente fica sempre na malha mais grossa, de qualquer distância.
    /// Só muda o desenho; volume, massa e seleção continuam os da peça
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub simplificada: bool,
}

fn yes() -> bool {
    true
}

/// Geometria de um componente usada numa restrição, no sistema local da peça.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Geom {
    /// plano / face plana: ponto e normal
    Plane { p: V3, n: V3 },
    /// eixo / face cilíndrica ou cônica / aresta reta: ponto e direção
    Axis { p: V3, d: V3 },
    Point(V3),
}

/// Elemento de um componente escolhido para a restrição. `pick` é o ponto clicado (local), usado
/// para reencontrar a face quando a peça muda.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct MateRef {
    pub comp: Id,
    pub geom: Geom,
    #[serde(default)]
    pub pick: Option<V3>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MateKind {
    /// eixos alinhados, pontos coincidentes, faces coplanares (mesmo sentido; `flip`: invertido)
    Coincident { flip: bool },
    /// faces em contato (coplanares e de frente uma para a outra)
    Contact,
    /// distância (aceita expressão; negativa troca o lado)
    Distance(String),
    /// ângulo entre planos/eixos (graus)
    Angle(String),
    Parallel,
    Perpendicular,
    /// engrenagem (ou polia, `belt`): giro de B = razão × giro de A, em torno dos eixos escolhidos.
    /// `refs` = (direção de referência de A local, a mesma no mundo ao criar, idem de B)
    Gear {
        ratio: String,
        #[serde(default)]
        belt: bool,
        #[serde(default)]
        refs: [V3; 4],
    },
    /// pinhão (A, eixo) e cremalheira (B, aresta/eixo): avanço de B = mm por volta × voltas de A.
    /// `refs` = (referência de A local, no mundo ao criar, ponto de B no mundo ao criar, direção de B local)
    RackPinion {
        mm_per_rev: String,
        #[serde(default)]
        refs: [V3; 4],
    },
}

impl MateKind {
    /// Restrição de movimento (relaciona giros/avanços, não posições).
    pub fn is_motion(&self) -> bool {
        matches!(self, MateKind::Gear { .. } | MateKind::RackPinion { .. })
    }
    pub fn label(&self) -> &'static str {
        match self {
            MateKind::Gear { belt: false, .. } => "Engrenagem",
            MateKind::Gear { belt: true, .. } => "Polia (correia)",
            MateKind::RackPinion { .. } => "Pinhão e cremalheira",
            MateKind::Coincident { .. } => "Coincidência",
            MateKind::Contact => "Contato",
            MateKind::Distance(_) => "Distância",
            MateKind::Angle(_) => "Ângulo",
            MateKind::Parallel => "Paralelismo",
            MateKind::Perpendicular => "Perpendicularidade",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            MateKind::Gear { belt: false, .. } => "⚙",
            MateKind::Gear { belt: true, .. } => "◎",
            MateKind::RackPinion { .. } => "⛭",
            MateKind::Coincident { .. } => "◎",
            MateKind::Contact => "⫴",
            MateKind::Distance(_) => "↔",
            MateKind::Angle(_) => "∠",
            MateKind::Parallel => "∥",
            MateKind::Perpendicular => "⟂",
        }
    }
}

/// Sentido imposto entre as normais (ou eixos) dos dois elementos de uma restrição.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Sentido {
    /// não impõe: vale o lado em que a peça estiver (restrições gravadas antes do B71)
    #[default]
    Livre,
    Mesmo,
    Oposto,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Mate {
    pub id: Id,
    pub name: String,
    pub kind: MateKind,
    pub a: MateRef,
    pub b: MateRef,
    #[serde(default)]
    pub suppressed: bool,
    /// alinhamento: sentido das normais em Distância, Ângulo e Paralelismo (a Coincidência guarda
    /// o dela em `flip`, e o Contato é sempre oposto)
    #[serde(default)]
    pub sentido: Sentido,
    /// lado: sentido da cota. A Distância é medida do lado oposto ao da normal de A
    #[serde(default)]
    pub lado_invertido: bool,
}

impl Mate {
    /// "Inverter alinhamento" muda alguma coisa neste par?
    pub fn aceita_alinhamento(&self) -> bool {
        let iguais = matches!((self.a.geom, self.b.geom), (Geom::Plane { .. }, Geom::Plane { .. }) | (Geom::Axis { .. }, Geom::Axis { .. }));
        match self.kind {
            MateKind::Coincident { .. } | MateKind::Distance(_) | MateKind::Angle(_) | MateKind::Parallel => iguais,
            MateKind::Contact => matches!((self.a.geom, self.b.geom), (Geom::Plane { .. }, Geom::Plane { .. })),
            _ => false,
        }
    }

    /// "Inverter lado" muda alguma coisa neste par? Só onde a cota tem sinal: medida ao longo da
    /// normal de um plano. Entre pontos e eixos a distância é um módulo, e o ângulo entre duas
    /// direções também não tem sinal.
    pub fn aceita_lado(&self) -> bool {
        matches!(self.kind, MateKind::Distance(_)) && (matches!(self.a.geom, Geom::Plane { .. }) || matches!(self.b.geom, Geom::Plane { .. }))
    }
}

/// O par de elementos faz sentido para a restrição? Par sem sentido não gera resíduo: a restrição
/// nasceria sem efeito nenhum.
pub fn par_valido(kind: &MateKind, a: &Geom, b: &Geom) -> bool {
    let ponto = |g: &Geom| matches!(g, Geom::Point(_));
    match kind {
        MateKind::Angle(_) | MateKind::Parallel | MateKind::Perpendicular => !ponto(a) && !ponto(b),
        MateKind::Contact => !matches!((a, b), (Geom::Point(_), Geom::Axis { .. }) | (Geom::Axis { .. }, Geom::Point(_))),
        _ => true,
    }
}

/// Valor de cota com 4 casas, sem zeros à direita: 12.5, 0, -3.1416.
pub fn valor_cota(x: f64) -> String {
    let v = (x * 1e4).round() / 1e4;
    if v == 0. {
        return "0".into();
    }
    format!("{v:.4}").trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Qualquer direção perpendicular a `d` (unitária).
pub fn any_perp(d: V3) -> V3 {
    let a = if d[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    unit(cross(d, a))
}

/// Recurso feito na montagem que muda a forma de peças (associativo: segue as posições).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AsmFeature {
    pub id: Id,
    pub name: String,
    pub kind: AsmFeatKind,
    /// componentes afetados
    pub targets: Vec<Id>,
    #[serde(default)]
    pub suppressed: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum AsmFeatKind {
    /// recorte pela forma de outro componente (furo do pino, ranhura da aba, encaixe)
    Cut { tool: Id, #[serde(default)] clearance: String },
    /// furo atravessando as peças: ponto e direção no mundo, diâmetro e profundidade (vazio = passante).
    /// Forma antiga: só muda a vista e não segue as peças. O comando novo cria [`AsmFeatKind::Serie`]
    Hole { p: V3, d: V3, dia: String, depth: String },
    /// furo de montagem pelo eixo, gravado nas peças (ou só na montagem): ver [`furos`]
    Serie(SerieFuros),
}

impl AsmFeatKind {
    pub fn label(&self) -> &'static str {
        match self {
            AsmFeatKind::Cut { .. } => "Recorte pela peça",
            AsmFeatKind::Hole { .. } | AsmFeatKind::Serie(_) => "Furo entre peças",
        }
    }
}

/// Fixação conjunta: as peças se comportam como um corpo rígido único.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RigidGroup {
    pub id: Id,
    pub name: String,
    pub comps: Vec<Id>,
}

/// Repetição inteligente: cópias de um componente em padrão linear ou circular. As cópias
/// acompanham o original e são refeitas quando a quantidade muda.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CompPattern {
    pub id: Id,
    pub name: String,
    pub source: Id,
    pub kind: PatternKind,
    /// cópias geradas (nós da árvore, na ordem): por posição, uma de cada fonte
    #[serde(default)]
    pub instances: Vec<Id>,
    /// outros componentes repetidos junto com `source`
    #[serde(default)]
    pub extras: Vec<Id>,
    /// posições (i, j) sem cópia; ver [`repeticao`]
    #[serde(default)]
    pub pular: Vec<(u32, u32)>,
    /// aresta, eixo ou face que dá a direção (ou o eixo, no circular): quando há, ela manda
    #[serde(default)]
    pub ref1: Option<MateRef>,
    /// o mesmo para a segunda direção
    #[serde(default)]
    pub ref2: Option<MateRef>,
    #[serde(default)]
    pub inverter: bool,
    #[serde(default)]
    pub inverter2: bool,
}

impl PatternKind {
    /// Linear de uma direção só.
    pub fn linear(dir: V3, count: &str, spacing: &str) -> Self {
        PatternKind::Linear { dir, count: count.into(), spacing: spacing.into(), dir2: None, count2: "1".into(), spacing2: "0".into() }
    }
}

impl CompPattern {
    /// Repetição ainda fora da montagem, com o que o comando vai preenchendo.
    pub fn rascunho(source: Id, kind: PatternKind) -> Self {
        CompPattern { id: Id(0), name: String::new(), source, kind, instances: vec![], extras: vec![], pular: vec![], ref1: None, ref2: None, inverter: false, inverter2: false }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PatternKind {
    /// direção no mundo, quantidade total (com o original) e espaçamento; segunda direção opcional
    Linear {
        dir: V3,
        count: String,
        spacing: String,
        #[serde(default)]
        dir2: Option<V3>,
        #[serde(default)]
        count2: String,
        #[serde(default)]
        spacing2: String,
    },
    /// eixo no mundo, quantidade total e ângulo total (360 = volta completa)
    Circular { origin: V3, axis: V3, count: String, angle: String },
    /// pelos furos: bocas do padrão `feature` da peça `comp`, no sistema dela
    Furos { comp: Id, feature: Id, pontos: Vec<V3> },
}

/// Parâmetro da montagem (usável nas distâncias e ângulos).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AsmParam {
    pub name: String,
    pub expr: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Assembly {
    pub format_version: u32,
    pub name: String,
    #[serde(default)]
    pub props: Properties,
    next_id: u64,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub mates: Vec<Mate>,
    #[serde(default)]
    pub rigid: Vec<RigidGroup>,
    #[serde(default)]
    pub params: Vec<AsmParam>,
    #[serde(default)]
    pub patterns: Vec<CompPattern>,
    /// juntas da cinemática e acoplamentos entre elas
    #[serde(default)]
    pub joints: Vec<kin::Joint>,
    #[serde(default)]
    pub couplings: Vec<kin::Coupling>,
    /// estados de exibição salvos e animações
    #[serde(default)]
    pub display_states: Vec<display::DisplayState>,
    #[serde(default)]
    pub animations: Vec<display::Animation>,
    /// explosões por passos: a entrada das instruções de montagem e da simulação de sequência
    #[serde(default)]
    pub explosoes: Vec<display::Explosao>,
    /// anotações 3D (PMI) deste nível de montagem
    #[serde(default)]
    pub pmi: Vec<fenkra_core::pmi::Pmi>,
    /// números de item fixos da lista de peças (por arquivo do componente)
    #[serde(default)]
    pub items: Vec<bom::ItemNo>,
    /// capturas de vista das anotações deste nível
    #[serde(default)]
    pub pmi_captures: Vec<fenkra_core::pmi::PmiCapture>,
    /// recursos entre peças (recortes, furos), refeitos com as posições
    #[serde(default)]
    pub features: Vec<AsmFeature>,
    /// pilhas de fixadores: recalculáveis, ligadas aos furos (ver [`pilhas`])
    #[serde(default)]
    pub pilhas: Vec<pilhas::Pilha>,
    /// interferências ignoradas e o limite "ignorar menores que"
    #[serde(default)]
    pub interf: interferencia::RegrasInterf,
    /// projeto relacional (peças novas fixas, posicionadas por links entre peças) × por restrições
    #[serde(default)]
    pub relational: bool,
    /// quando cada link das peças atualiza (só os que fogem do automático)
    #[serde(default)]
    pub politicas_link: Vec<links::PoliticaLink>,
    /// originais dos itens ligados ao nó de um subconjunto, para inserir de volta
    #[serde(default)]
    pub religacoes: Vec<Religacao>,
    /// sketches, pontos, eixos e planos da própria montagem: ver [`geometria`]
    #[serde(default)]
    pub geometria: Option<fenkra_core::Document>,
    /// componente que é a peça esqueleto (fica fora da lista de materiais)
    #[serde(default)]
    pub esqueleto: Option<Id>,
    /// a seleção em contexto aceita geometria não publicada (o padrão é só publicada)
    #[serde(default)]
    pub contexto_livre: bool,
    /// planos e eixos da origem fora da vista
    #[serde(default)]
    pub origem_oculta: bool,
    /// configuração (da tabela de projeto da peça) escolhida por componente, pelo nome (B102)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<ConfigComp>,
}

/// Qual configuração da peça um componente usa. O nome precisa existir na tabela da peça: se não
/// existir, o componente aparece com erro, nunca com "a primeira" configuração.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ConfigComp {
    pub comp: Id,
    pub nome: String,
}

impl Assembly {
    /// Existe um componente, restrição, junta, acoplamento, padrão ou fixação com este id?
    pub fn has_item(&self, id: Id) -> bool {
        self.nodes.iter().any(|n| n.id == id)
            || self.mates.iter().any(|m| m.id == id)
            || self.joints.iter().any(|j| j.id == id)
            || self.couplings.iter().any(|c| c.id == id)
            || self.patterns.iter().any(|p| p.id == id)
            || self.rigid.iter().any(|g| g.id == id)
            || self.pmi.iter().any(|p| p.id == id)
            || self.features.iter().any(|f| f.id == id)
    }

    /// Prepara as referências de uma restrição de movimento (engrenagem, cremalheira) com as
    /// posições atuais: o giro/avanço zero é o de agora.
    pub fn init_motion(&mut self, id: Id) {
        let Some(i) = self.mates.iter().position(|m| m.id == id) else { return };
        let (ra, rb) = (self.mates[i].a, self.mates[i].b);
        let (Some(pa), Some(pb)) = (self.lugar(ra.comp), self.lugar(rb.comp)) else { return };
        let axis = |g: &Geom| match *g {
            Geom::Axis { p, d } => (p, d),
            Geom::Plane { p, n } => (p, n),
            Geom::Point(p) => (p, [0., 0., 1.]),
        };
        let (_, da) = axis(&ra.geom);
        let (ob, db) = axis(&rb.geom);
        let la = any_perp(unit(da));
        let lb = any_perp(unit(db));
        match &mut self.mates[i].kind {
            MateKind::Gear { refs, .. } => *refs = [la, pa.dir(la), lb, pb.dir(lb)],
            MateKind::RackPinion { refs, .. } => *refs = [la, pa.dir(la), pb.point(ob), unit(db)],
            _ => {}
        }
    }

    /// Distância e Ângulo assumem a medida de agora entre os elementos, para a peça não saltar ao
    /// nascer: o valor, o lado e o alinhamento saem da posição atual. Devolve `false` se o par não
    /// tem essa medida.
    pub fn iniciar_medida(&mut self, id: Id) -> bool {
        let Some(i) = self.mates.iter().position(|m| m.id == id) else { return false };
        let m = self.mates[i].clone();
        if !matches!(m.kind, MateKind::Distance(_) | MateKind::Angle(_)) {
            return true;
        }
        let (Some(pa), Some(pb)) = (self.lugar(m.a.comp), self.lugar(m.b.comp)) else { return false };
        let Some(v) = solve::medida(&m.kind, &m.a.geom, &pa, &m.b.geom, &pb) else { return false };
        let m = &mut self.mates[i];
        m.sentido = Sentido::Livre;
        m.lado_invertido = false;
        match &mut m.kind {
            MateKind::Distance(e) => {
                *e = valor_cota(v.abs());
                m.lado_invertido = v < 0.;
                if m.aceita_alinhamento() {
                    m.sentido = solve::sentido_atual(&m.a.geom, &pa, &m.b.geom, &pb);
                }
            }
            MateKind::Angle(e) => *e = valor_cota(v),
            _ => {}
        }
        true
    }

    /// Inverte o sentido das normais. O Contato é a coincidência de faces opostas: invertido, vira
    /// coincidência de faces no mesmo sentido. Devolve `false` se o par não tem alinhamento.
    pub fn inverter_alinhamento(&mut self, id: Id) -> bool {
        let Some(i) = self.mates.iter().position(|m| m.id == id) else { return false };
        if !self.mates[i].aceita_alinhamento() {
            return false;
        }
        let (ra, rb) = (self.mates[i].a, self.mates[i].b);
        let atual = match (self.lugar(ra.comp), self.lugar(rb.comp)) {
            (Some(pa), Some(pb)) => solve::sentido_atual(&ra.geom, &pa, &rb.geom, &pb),
            _ => return false,
        };
        let m = &mut self.mates[i];
        let angulo = matches!(m.kind, MateKind::Angle(_));
        match &mut m.kind {
            MateKind::Coincident { flip } => *flip = !*flip,
            MateKind::Contact => m.kind = MateKind::Coincident { flip: false },
            _ => {
                m.sentido = match (m.sentido, atual) {
                    (Sentido::Mesmo, _) => Sentido::Oposto,
                    (Sentido::Oposto, _) => Sentido::Mesmo,
                    // o ângulo livre é medido entre as normais como estão, sem olhar a posição
                    (Sentido::Livre, _) if angulo => Sentido::Oposto,
                    (Sentido::Livre, Sentido::Oposto) => Sentido::Mesmo,
                    (Sentido::Livre, _) => Sentido::Oposto,
                };
            }
        }
        true
    }

    /// Passa a cota para o outro lado da referência. Devolve `false` se a cota do par não tem lado.
    pub fn inverter_lado(&mut self, id: Id) -> bool {
        match self.mates.iter_mut().find(|m| m.id == id) {
            Some(m) if m.aceita_lado() => {
                m.lado_invertido = !m.lado_invertido;
                true
            }
            _ => false,
        }
    }

    /// Recurso entre peças novo.
    pub fn add_feature(&mut self, kind: AsmFeatKind, targets: Vec<Id>) -> Id {
        let id = self.alloc();
        let n = self.features.len() + 1;
        self.features.push(AsmFeature { id, name: format!("{} {n}", kind.label()), kind, targets, suppressed: false });
        id
    }

    pub fn new(name: &str) -> Self {
        Assembly { format_version: 1, name: name.into(), props: Properties::default(), next_id: 1, nodes: vec![], mates: vec![], rigid: vec![], params: vec![], patterns: vec![], joints: vec![], couplings: vec![], display_states: vec![], animations: vec![], explosoes: vec![], pmi: vec![], items: vec![], pmi_captures: vec![], features: vec![], pilhas: vec![], interf: Default::default(), relational: false, politicas_link: vec![], religacoes: vec![], geometria: None, esqueleto: None, contexto_livre: false, origem_oculta: false, configs: vec![] }
    }

    /// Configuração escolhida para o componente (vazio = a ativa da própria peça).
    pub fn config_de(&self, comp: Id) -> &str {
        self.configs.iter().find(|c| c.comp == comp).map(|c| c.nome.as_str()).unwrap_or("")
    }

    /// Escolhe a configuração do componente pelo nome; nome vazio volta ao padrão da peça.
    pub fn definir_config(&mut self, comp: Id, nome: &str) {
        self.configs.retain(|c| c.comp != comp);
        if !nome.trim().is_empty() {
            self.configs.push(ConfigComp { comp, nome: nome.trim().to_string() });
        }
    }

    pub fn alloc(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Abre qualquer versão antiga pela cadeia de migração (B64).
    pub fn from_json(s: &str) -> Result<Self, String> {
        let p = fenkra_core::formato::preparar(fenkra_core::formato::Tipo::Montagem, s).map_err(|e| e.to_string())?;
        serde_json::from_value(p.valor).map_err(|e| e.to_string())
    }
    /// Gravada por um Fenkra mais novo: abrir somente para leitura.
    pub fn formato_futuro(&self) -> bool {
        self.format_version > fenkra_core::formato::Tipo::Montagem.versao_atual()
    }

    /// Posição de um componente. A origem da montagem é o sistema do mundo.
    pub fn lugar(&self, id: Id) -> Option<Placement> {
        if id == ORIGEM {
            return Some(Placement::default());
        }
        self.node(id).map(|n| n.placement)
    }

    pub fn node(&self, id: Id) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: Id) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn name_of(&self, id: Id) -> String {
        if id == ORIGEM {
            return "Origem".into();
        }
        self.node(id)
            .map(|n| n.name.clone())
            .or_else(|| self.mates.iter().find(|m| m.id == id).map(|m| m.name.clone()))
            .or_else(|| self.rigid.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .or_else(|| self.joints.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .or_else(|| self.couplings.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .or_else(|| self.patterns.iter().find(|r| r.id == id).map(|r| r.name.clone()))
            .unwrap_or_else(|| format!("#{}", id.0))
    }

    pub(crate) fn unique_pub(&self, base: &str) -> String {
        self.unique(base)
    }

    fn unique(&self, base: &str) -> String {
        let taken = |n: &str| {
            self.nodes.iter().any(|x| x.name == n)
                || self.mates.iter().any(|x| x.name == n)
                || self.rigid.iter().any(|x| x.name == n)
                || self.patterns.iter().any(|x| x.name == n)
                || self.joints.iter().any(|x| x.name == n)
                || self.couplings.iter().any(|x| x.name == n)
        };
        if !taken(base) {
            return base.to_string();
        }
        (2..).map(|i| format!("{base}.{i}")).find(|n| !taken(n)).unwrap()
    }

    /// Grupo vazio de organização.
    pub fn add_group(&mut self, name: &str, parent: Option<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique(name);
        self.nodes.push(Node { id, name, number: 0, kind: NodeKind::Group, parent, placement: Placement::default(), fixed: false, visible: true, mode: LoadMode::Editavel, color: None, simplificada: false });
        id
    }

    /// Insere uma peça (ou subconjunto) existente. O primeiro componente entra fixo.
    pub fn add_component(&mut self, name: &str, kind: NodeKind, placement: Placement, parent: Option<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique(name);
        let first = !self.nodes.iter().any(|n| !matches!(n.kind, NodeKind::Group));
        let number = self.nodes.iter().map(|n| n.number).max().unwrap_or(0) + 1;
        self.nodes.push(Node { id, name, number, kind, parent, placement, fixed: first, visible: true, mode: LoadMode::Editavel, color: None, simplificada: false });
        id
    }

    pub fn add_mate(&mut self, kind: MateKind, a: MateRef, b: MateRef) -> Id {
        let id = self.alloc();
        let name = self.unique(kind.label());
        self.mates.push(Mate { id, name, kind, a, b, suppressed: false, sentido: Sentido::Livre, lado_invertido: false });
        id
    }

    pub fn add_rigid(&mut self, comps: Vec<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique("Fixação conjunta");
        self.rigid.push(RigidGroup { id, name, comps });
        id
    }

    pub fn add_pattern(&mut self, source: Id, kind: PatternKind) -> Id {
        let id = self.alloc();
        let name = self.unique("Repetição");
        self.patterns.push(CompPattern { id, name, ..CompPattern::rascunho(source, kind) });
        self.update_patterns();
        id
    }

    /// Refaz as cópias de cada repetição: cria/apaga instâncias conforme a quantidade e posiciona
    /// cada uma a partir do original.
    pub fn update_patterns(&mut self) {
        let vars = self.vars();
        let pats = self.patterns.clone();
        for (k, p) in pats.iter().enumerate() {
            let fontes = self.fontes(p);
            if !fontes.contains(&p.source) {
                continue;
            }
            let passos: Vec<repeticao::Passo> = self.passos(p, &vars).into_iter().filter(|x| !x.pulada).collect();
            let precisa = passos.len() * fontes.len();
            let mut inst = p.instances.clone();
            inst.retain(|i| self.node(*i).is_some());
            while inst.len() < precisa {
                let Some(src) = self.node(fontes[inst.len() % fontes.len()]).cloned() else { break };
                let id = self.alloc();
                let name = self.unique(&src.name);
                self.nodes.push(Node { id, name, number: 0, fixed: true, ..src });
                inst.push(id);
            }
            while inst.len() > precisa {
                let id = inst.pop().unwrap();
                self.nodes.retain(|n| n.id != id);
                self.mates.retain(|m| m.a.comp != id && m.b.comp != id);
            }
            for (k, id) in inst.iter().enumerate() {
                let Some(src) = self.node(fontes[k % fontes.len()]).cloned() else { continue };
                let pl = passos[k / fontes.len()].aplicar(&src.placement);
                if let Some(n) = self.node_mut(*id) {
                    n.placement = pl;
                    n.kind = src.kind.clone();
                    // as cópias seguem o original: não têm graus de liberdade próprios
                    n.fixed = true;
                }
            }
            self.patterns[k].instances = inst;
        }
    }

    /// Numeração automática: índices sequenciais na ordem da árvore (só peças e subconjuntos).
    pub fn renumber(&mut self) {
        let mut k = 0;
        for n in self.nodes.iter_mut().filter(|n| !matches!(n.kind, NodeKind::Group)) {
            k += 1;
            n.number = k;
        }
    }

    /// Apaga um nó (filhos sobem um nível) ou uma restrição; restrições do componente vão junto.
    pub fn delete(&mut self, id: Id) {
        self.features.retain(|f| f.id != id && !matches!(f.kind, AsmFeatKind::Cut { tool, .. } if tool == id));
        let posicao = self.node(id).map(|n| n.placement);
        if self.esqueleto == Some(id) {
            self.esqueleto = None;
        }
        for f in &mut self.features {
            f.targets.retain(|t| *t != id);
            if let AsmFeatKind::Serie(s) = &mut f.kind {
                s.pecas.retain(|p| p.comp != id);
                // o eixo não some com a peça que o ancorava: fica onde está
                if let (Some(pl), true) = (&posicao, s.eixo.ancora() == Some(id)) {
                    s.eixo.congelar(pl);
                }
            }
        }
        let parent = self.node(id).and_then(|n| n.parent);
        for n in self.nodes.iter_mut().filter(|n| n.parent == Some(id)) {
            n.parent = parent;
        }
        self.nodes.retain(|n| n.id != id);
        self.mates.retain(|m| m.id != id && m.a.comp != id && m.b.comp != id);
        self.joints.retain(|j| j.id != id && j.a.comp != id && j.b.comp != id);
        let alive: Vec<Id> = self.joints.iter().map(|j| j.id).collect();
        self.couplings.retain(|c| c.id != id && alive.contains(&c.a) && alive.contains(&c.b));
        self.rigid.retain(|r| r.id != id);
        // apagar a repetição apaga as cópias; apagar o original apaga a repetição
        let gone: Vec<Id> = self.patterns.iter().filter(|p| p.id == id || p.source == id).flat_map(|p| p.instances.clone()).collect();
        self.patterns.retain(|p| p.id != id && p.source != id);
        self.nodes.retain(|n| !gone.contains(&n.id));
        for r in &mut self.rigid {
            r.comps.retain(|c| *c != id);
        }
    }

    /// Substitui a peça de um componente mantendo nome, posição e restrições (as faces são
    /// reencontradas pelo ponto clicado).
    pub fn replace(&mut self, id: Id, kind: NodeKind) {
        if let Some(n) = self.node_mut(id) {
            n.kind = kind;
        }
    }

    /// Reordena só a árvore (visualização): move `id` para antes de `before` (ou para o fim).
    pub fn reorder(&mut self, id: Id, before: Option<Id>) {
        let Some(i) = self.nodes.iter().position(|n| n.id == id) else { return };
        let n = self.nodes.remove(i);
        let j = before.and_then(|b| self.nodes.iter().position(|x| x.id == b)).unwrap_or(self.nodes.len());
        self.nodes.insert(j, n);
    }

    /// Valores dos parâmetros da montagem.
    pub fn vars(&self) -> HashMap<String, f64> {
        let mut v = HashMap::new();
        for p in &self.params {
            if let Ok(x) = expr::eval_str(&p.expr, &v) {
                v.insert(p.name.clone(), x);
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_ida_e_volta() {
        let p = Placement::default().moved([1., 2., 3.], [0., 0., std::f64::consts::FRAC_PI_2], [0.; 3]);
        let q = [1., 0., 0.];
        let w = p.point(q);
        assert!(norm(sub(w, [1., 3., 3.])) < 1e-12, "{w:?}");
        assert!(norm(sub(p.to_local(w), q)) < 1e-12);
        let inv = p.inverse();
        assert!(norm(sub(inv.point(w), q)) < 1e-12);
    }

    #[test]
    fn repeticao_acompanha_a_quantidade() {
        let mut a = Assembly::new("P");
        let _base = a.add_component("Base", NodeKind::Part { path: "b.fkp".into() }, Placement::default(), None);
        let bolt = a.add_component("Parafuso", NodeKind::Part { path: "p.fkp".into() }, Placement { origin: [10., 0., 0.], ..Default::default() }, None);
        let pat = a.add_pattern(bolt, PatternKind::linear([1., 0., 0.], "3", "20"));
        assert_eq!(a.patterns[0].instances.len(), 2);
        let last = *a.patterns[0].instances.last().unwrap();
        assert!(norm(sub(a.node(last).unwrap().placement.origin, [50., 0., 0.])) < 1e-9);
        if let PatternKind::Linear { count, .. } = &mut a.patterns[0].kind {
            *count = "5".into();
        }
        a.update_patterns();
        assert_eq!(a.nodes.len(), 6, "base + original + 4 cópias");
        a.delete(pat);
        assert_eq!(a.nodes.len(), 2);
    }

    #[test]
    fn json_e_numeracao() {
        let mut a = Assembly::new("A");
        let g = a.add_group("Fixadores", None);
        let p1 = a.add_component("Base", NodeKind::Part { path: "base.fkp".into() }, Placement::default(), None);
        let p2 = a.add_component("Tampa", NodeKind::Part { path: "tampa.fkp".into() }, Placement::default(), Some(g));
        assert!(a.node(p1).unwrap().fixed && !a.node(p2).unwrap().fixed, "o primeiro entra fixo");
        a.reorder(p2, Some(g));
        a.renumber();
        assert_eq!(a.node(p2).unwrap().number, 1);
        let b = Assembly::from_json(&a.to_json()).unwrap();
        assert_eq!(a, b);
        a.delete(g);
        assert_eq!(a.node(p2).unwrap().parent, None, "filhos sobem um nível");
    }
}
