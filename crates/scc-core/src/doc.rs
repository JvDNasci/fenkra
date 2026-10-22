//! Documento de peça: árvore fixa + linha de tempo de comandos.
//!
//! ```text
//! Peça
//! ├── Origem (ponto, eixos X/Y/Z, planos XY/XZ/YZ)      RF-ARV-001
//! ├── Referências (planos de referência, variáveis)      RF-ARV-010
//! ├── Parâmetros                                         RF-ARV-020
//! ├── Coletores de corpos                                RF-ARV-040
//! ├── pastas personalizadas                              RF-ARV-030
//! └── comandos (ordem = linha de tempo)                  RF-ARV-050
//! ```

use crate::expr::{self, ExprError};
use crate::sketch::SketchData;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const FORMAT_VERSION: u32 = 1;

/// Identidade estável de qualquer objeto do documento (RF-NUC-020).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Id(pub u64);

// ---------------------------------------------------------------- Origem e referências

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OriginPlane {
    XY,
    XZ,
    YZ,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OriginAxis {
    X,
    Y,
    Z,
}

/// Referência a um plano: plano da Origem ou elemento da pasta Referências.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum PlaneRef {
    Origin(OriginPlane),
    Datum(Id),
}

impl Default for PlaneRef {
    fn default() -> Self {
        PlaneRef::Origin(OriginPlane::XY)
    }
}

/// Referência a um eixo: eixo da Origem ou linha de um sketch.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum AxisRef {
    Origin(OriginAxis),
    SketchLine { sketch: Id, entity: Id },
    /// linha/curva de referência (direção do início ao fim)
    Ref(Id),
    /// eixo fixo tirado de uma aresta clicada (fixo nesta versão)
    Fixed { origin: [f64; 3], dir: [f64; 3] },
}

impl AxisRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        match self {
            AxisRef::SketchLine { sketch, .. } => ids(sketch),
            AxisRef::Ref(x) => ids(x),
            AxisRef::Origin(_) | AxisRef::Fixed { .. } => {}
        }
    }
}

impl PlaneRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        if let PlaneRef::Datum(x) = self {
            ids(x)
        }
    }
}

/// Ponto usado como entrada de um elemento de geometria.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum PointRef {
    Origin,
    /// ponto de referência (pasta de geometria)
    Ref(Id),
    /// vértice ou ponto clicado no modelo (fixo nesta versão)
    Fixed([f64; 3]),
}

impl Default for PointRef {
    fn default() -> Self {
        PointRef::Origin
    }
}

impl PointRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        if let PointRef::Ref(x) = self {
            ids(x)
        }
    }
}

/// Curva usada como entrada.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CurveRef {
    Axis(OriginAxis),
    /// linha ou curva de referência
    Ref(Id),
    /// linha de um sketch
    SketchLine { sketch: Id, entity: Id },
    /// aresta clicada no modelo, como polilinha (fixa nesta versão)
    Edge(Vec<[f64; 3]>),
}

impl Default for CurveRef {
    fn default() -> Self {
        CurveRef::Axis(OriginAxis::X)
    }
}

impl CurveRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        match self {
            CurveRef::Ref(x) => ids(x),
            CurveRef::SketchLine { sketch, .. } => ids(sketch),
            CurveRef::Axis(_) | CurveRef::Edge(_) => {}
        }
    }
}

/// Superfície usada como entrada: plano ou face clicada (ponto e normal, fixa nesta versão).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum SurfRef {
    Plane(PlaneRef),
    Face { point: [f64; 3], normal: [f64; 3], owner: Option<Id> },
}

impl Default for SurfRef {
    fn default() -> Self {
        SurfRef::Plane(PlaneRef::default())
    }
}

impl SurfRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        if let SurfRef::Plane(p) = self {
            p.walk(ids)
        }
    }
}

/// Posição sobre uma curva: distância a partir do início ou proporção do comprimento (0..1).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum OnCurve {
    Distance(String),
    Ratio(String),
}

impl Default for OnCurve {
    fn default() -> Self {
        OnCurve::Ratio("0.5".into())
    }
}

impl OnCurve {
    fn expr_mut(&mut self) -> &mut String {
        match self {
            OnCurve::Distance(e) | OnCurve::Ratio(e) => e,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ConicKind {
    #[default]
    Elipse,
    Parabola,
    Hiperbole,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Continuity {
    G0,
    #[default]
    G1,
    G2,
}

/// Classe do elemento de geometria de referência.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefClass {
    Plane,
    Point,
    Curve,
}

/// Elemento da pasta Referências. O nome é a variável referenciável (RF-LNK-031);
/// os comandos apontam para o `id`, então renomear ou redirecionar não quebra nada.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RefElement {
    pub id: Id,
    pub name: String,
    pub kind: RefKind,
    pub visible: bool,
    /// pasta de geometria onde o elemento fica (sem linha do tempo)
    #[serde(default)]
    pub folder: Option<Id>,
}

/// Vista salva (câmera) — base para vistas 2D e detalhamento 3D.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NamedView {
    pub name: String,
    pub target: [f64; 3],
    pub yaw: f64,
    pub pitch: f64,
    pub dist: f64,
    pub ortho: bool,
}

/// Estado de um link com outra peça (ADR 0006).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LinkState {
    #[default]
    Ativo,
    Congelado,
    Quebrado,
}

/// Geometria importada de outra peça por link (sempre separada da geometria própria).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ExternalRef {
    pub id: Id,
    /// nome da variável referenciável (ex.: "Mestre.PlanoBase")
    pub name: String,
    /// documento de origem (PN ou nome) e publicação
    pub source: String,
    pub publication: String,
    pub state: LinkState,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum RefKind {
    /// Plano paralelo a outro plano, deslocado por uma expressão.
    OffsetPlane { base: PlaneRef, offset: String },
    /// Plano que passa por um eixo, girado de um ângulo a partir do plano base.
    AnglePlane { base: PlaneRef, axis: AxisRef, angle: String },
    /// Plano de uma face plana clicada na viewport (origem, direção X e normal).
    FacePlane { origin: [f64; 3], x: [f64; 3], normal: [f64; 3], owner: Option<Id> },
    /// Paralelo a um plano, passando por um ponto.
    PlaneParallelPoint { base: PlaneRef, point: PointRef },
    /// Por três pontos não colineares.
    PlaneThreePoints { a: PointRef, b: PointRef, c: PointRef },
    /// Por duas linhas coplanares (ou paralelas).
    PlaneTwoLines { a: CurveRef, b: CurveRef },
    /// Por um ponto e uma linha.
    PlanePointLine { point: PointRef, line: CurveRef },
    /// Plano que contém uma curva planar.
    PlaneCurve { curve: CurveRef },
    /// Normal a uma curva num ponto dela (distância ou proporção).
    PlaneNormalCurve { curve: CurveRef, at: OnCurve, reverse: bool },
    /// Tangente a uma superfície no ponto (projetado na superfície).
    PlaneTangentSurface { surface: SurfRef, point: PointRef },
    /// Ax + By + Cz = D
    PlaneEquation { a: String, b: String, c: String, d: String },
    /// Plano médio (ajuste) por três ou mais pontos.
    PlaneMeanPoints { points: Vec<PointRef> },

    // ---------------- pontos
    /// Coordenadas X, Y, Z (no sistema da Origem ou de um plano: u, v, normal).
    PointCoords { x: String, y: String, z: String, frame: Option<PlaneRef> },
    /// Sobre uma curva: distância/proporção a partir do início (ou do fim).
    PointOnCurve { curve: CurveRef, at: OnCurve, reverse: bool },
    /// Sobre um plano, nas coordenadas (u, v) do plano.
    PointOnPlane { plane: PlaneRef, u: String, v: String },
    /// Sobre uma superfície: deslocado (u, v) no plano tangente a partir do ponto da superfície.
    PointOnSurface { surface: SurfRef, u: String, v: String },
    /// Centro de círculo ou arco.
    PointCenter { curve: CurveRef },
    /// Ponto de uma curva planar onde a tangente é paralela a uma direção (extremo 0 ou 1).
    PointTangent { curve: CurveRef, dir: AxisRef, which: usize },
    /// Entre dois pontos, por proporção (0.5 = ponto médio).
    PointBetween { a: PointRef, b: PointRef, ratio: String },

    // ---------------- curvas
    /// Linha ponto a ponto, com prolongamento opcional nas pontas.
    LinePoints { a: PointRef, b: PointRef, start: String, end: String },
    /// Linha por ponto e direção (do `start` ao `end` ao longo da direção).
    LineDir { point: PointRef, dir: AxisRef, start: String, end: String },
    /// Linha em ângulo com uma curva (0° = tangente, 90° = normal), no plano de apoio.
    LineAngle { curve: CurveRef, point: PointRef, support: PlaneRef, angle: String, start: String, end: String },
    /// Linha normal a uma superfície no ponto.
    LineNormalSurface { surface: SurfRef, point: PointRef, start: String, end: String },
    /// Polilinha pelos pontos.
    Polyline { points: Vec<PointRef>, closed: bool },
    /// Círculo/arco por centro e raio num plano (ângulos de início e fim em graus).
    Circle { center: PointRef, plane: PlaneRef, radius: String, start: String, end: String },
    /// Círculo (ou arco) por três pontos.
    Circle3 { a: PointRef, b: PointRef, c: PointRef, full: bool },
    /// Spline interpolada pelos pontos.
    Spline { points: Vec<PointRef>, closed: bool },
    /// Hélice cilíndrica ou cônica.
    Helix { origin: PointRef, axis: AxisRef, radius: String, pitch: String, height: String, taper: String, left: bool },
    /// Espiral plana do raio inicial ao final.
    Spiral { center: PointRef, plane: PlaneRef, r0: String, r1: String, turns: String },
    /// Cônica (elipse, parábola, hipérbole) no plano.
    Conic { center: PointRef, plane: PlaneRef, kind: ConicKind, a: String, b: String, extent: String },
    /// Curva de conexão entre as pontas de duas curvas, com continuidade G0/G1/G2.
    Connect { a: CurveRef, a_end: bool, b: CurveRef, b_end: bool, cont: Continuity, tension: String },
    /// Canto (concordância) entre duas linhas.
    Corner { a: CurveRef, b: CurveRef, radius: String },
    /// Spine: curva guia normal a uma sequência de planos.
    Spine { planes: Vec<PlaneRef> },
}

impl RefKind {
    pub fn base(&self) -> Option<&PlaneRef> {
        match self {
            RefKind::OffsetPlane { base, .. } | RefKind::AnglePlane { base, .. } | RefKind::PlaneParallelPoint { base, .. } => Some(base),
            _ => None,
        }
    }
    pub fn base_mut(&mut self) -> Option<&mut PlaneRef> {
        match self {
            RefKind::OffsetPlane { base, .. } | RefKind::AnglePlane { base, .. } | RefKind::PlaneParallelPoint { base, .. } => Some(base),
            _ => None,
        }
    }

    pub fn class(&self) -> RefClass {
        use RefKind::*;
        match self {
            OffsetPlane { .. } | AnglePlane { .. } | FacePlane { .. } | PlaneParallelPoint { .. } | PlaneThreePoints { .. } | PlaneTwoLines { .. }
            | PlanePointLine { .. } | PlaneCurve { .. } | PlaneNormalCurve { .. } | PlaneTangentSurface { .. } | PlaneEquation { .. }
            | PlaneMeanPoints { .. } => RefClass::Plane,
            PointCoords { .. } | PointOnCurve { .. } | PointOnPlane { .. } | PointOnSurface { .. } | PointCenter { .. } | PointTangent { .. }
            | PointBetween { .. } => RefClass::Point,
            _ => RefClass::Curve,
        }
    }

    /// Percorre todos os ids referenciados e todas as expressões.
    pub fn walk(&mut self, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
        use RefKind::*;
        match self {
            OffsetPlane { base, offset } => {
                base.walk(ids);
                exprs(offset)
            }
            AnglePlane { base, axis, angle } => {
                base.walk(ids);
                axis.walk(ids);
                exprs(angle)
            }
            FacePlane { .. } => {}
            PlaneParallelPoint { base, point } => {
                base.walk(ids);
                point.walk(ids)
            }
            PlaneThreePoints { a, b, c } => {
                for p in [a, b, c] {
                    p.walk(ids)
                }
            }
            PlaneTwoLines { a, b } => {
                a.walk(ids);
                b.walk(ids)
            }
            PlanePointLine { point, line } => {
                point.walk(ids);
                line.walk(ids)
            }
            PlaneCurve { curve } | PointCenter { curve } => curve.walk(ids),
            PlaneNormalCurve { curve, at, .. } | PointOnCurve { curve, at, .. } => {
                curve.walk(ids);
                exprs(at.expr_mut())
            }
            PlaneTangentSurface { surface, point } => {
                surface.walk(ids);
                point.walk(ids)
            }
            PlaneEquation { a, b, c, d } => {
                for e in [a, b, c, d] {
                    exprs(e)
                }
            }
            PlaneMeanPoints { points } | Polyline { points, .. } | Spline { points, .. } => {
                for p in points {
                    p.walk(ids)
                }
            }
            PointCoords { x, y, z, frame } => {
                if let Some(f) = frame {
                    f.walk(ids)
                }
                for e in [x, y, z] {
                    exprs(e)
                }
            }
            PointOnPlane { plane, u, v } => {
                plane.walk(ids);
                exprs(u);
                exprs(v)
            }
            PointOnSurface { surface, u, v } => {
                surface.walk(ids);
                exprs(u);
                exprs(v)
            }
            PointTangent { curve, dir, .. } => {
                curve.walk(ids);
                dir.walk(ids)
            }
            PointBetween { a, b, ratio } => {
                a.walk(ids);
                b.walk(ids);
                exprs(ratio)
            }
            LinePoints { a, b, start, end } => {
                a.walk(ids);
                b.walk(ids);
                exprs(start);
                exprs(end)
            }
            LineDir { point, dir, start, end } => {
                point.walk(ids);
                dir.walk(ids);
                exprs(start);
                exprs(end)
            }
            LineAngle { curve, point, support, angle, start, end } => {
                curve.walk(ids);
                point.walk(ids);
                support.walk(ids);
                for e in [angle, start, end] {
                    exprs(e)
                }
            }
            LineNormalSurface { surface, point, start, end } => {
                surface.walk(ids);
                point.walk(ids);
                exprs(start);
                exprs(end)
            }
            Circle { center, plane, radius, start, end } => {
                center.walk(ids);
                plane.walk(ids);
                for e in [radius, start, end] {
                    exprs(e)
                }
            }
            Circle3 { a, b, c, .. } => {
                for p in [a, b, c] {
                    p.walk(ids)
                }
            }
            Helix { origin, axis, radius, pitch, height, taper, .. } => {
                origin.walk(ids);
                axis.walk(ids);
                for e in [radius, pitch, height, taper] {
                    exprs(e)
                }
            }
            Spiral { center, plane, r0, r1, turns } => {
                center.walk(ids);
                plane.walk(ids);
                for e in [r0, r1, turns] {
                    exprs(e)
                }
            }
            Conic { center, plane, a, b, extent, .. } => {
                center.walk(ids);
                plane.walk(ids);
                for e in [a, b, extent] {
                    exprs(e)
                }
            }
            Connect { a, b, tension, .. } => {
                a.walk(ids);
                b.walk(ids);
                exprs(tension)
            }
            Corner { a, b, radius } => {
                a.walk(ids);
                b.walk(ids);
                exprs(radius)
            }
            Spine { planes } => {
                for p in planes {
                    p.walk(ids)
                }
            }
        }
    }

    /// Ids de que este elemento depende (planos, pontos, curvas, sketches).
    pub fn deps(&self) -> Vec<Id> {
        let mut out = vec![];
        self.clone().walk(&mut |x| out.push(*x), &mut |_| {});
        out
    }
    /// Sketch usado como eixo (se houver).
    pub fn axis_sketch(&self) -> Option<Id> {
        match self {
            RefKind::AnglePlane { axis: AxisRef::SketchLine { sketch, .. }, .. } => Some(*sketch),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- Parâmetros e propriedades

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Unit {
    Comprimento,
    Angulo,
    Numero,
}

impl Unit {
    pub fn symbol(self) -> &'static str {
        match self {
            Unit::Comprimento => "mm",
            Unit::Angulo => "°",
            Unit::Numero => "",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Parameter {
    pub id: Id,
    pub name: String,
    pub expr: String,
    pub unit: Unit,
    pub comment: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PartType {
    Fabricada,
    Comprada,
    Hardware,
    Chapa,
    MembroEstrutural,
    Fantasma,
}

impl PartType {
    pub const ALL: [PartType; 6] = [
        PartType::Fabricada,
        PartType::Comprada,
        PartType::Hardware,
        PartType::Chapa,
        PartType::MembroEstrutural,
        PartType::Fantasma,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PartType::Fabricada => "Fabricada",
            PartType::Comprada => "Comprada",
            PartType::Hardware => "Hardware",
            PartType::Chapa => "Chapa",
            PartType::MembroEstrutural => "Membro estrutural",
            PartType::Fantasma => "Fantasma",
        }
    }
}

/// Propriedades de peça/corpo (RF-PAR-020..022).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Properties {
    pub pn: String,
    pub description: String,
    pub material: String,
    pub part_type: PartType,
    pub custom: Vec<(String, String)>,
}

impl Default for Properties {
    fn default() -> Self {
        Self {
            pn: String::new(),
            description: String::new(),
            material: "Aço 1020".into(),
            part_type: PartType::Fabricada,
            custom: vec![],
        }
    }
}

// ---------------------------------------------------------------- Aparência

/// Acabamento visual (pré-render rápido). Não confundir com o material de engenharia da peça.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Finish {
    Fosco,
    #[default]
    Acetinado,
    Brilhante,
    MetalEscovado,
    MetalPolido,
    Borracha,
    Vidro,
}

impl Finish {
    pub const ALL: [Finish; 7] =
        [Finish::Fosco, Finish::Acetinado, Finish::Brilhante, Finish::MetalEscovado, Finish::MetalPolido, Finish::Borracha, Finish::Vidro];
    pub fn label(self) -> &'static str {
        match self {
            Finish::Fosco => "Fosco",
            Finish::Acetinado => "Acetinado",
            Finish::Brilhante => "Plástico brilhante",
            Finish::MetalEscovado => "Metal escovado",
            Finish::MetalPolido => "Metal polido",
            Finish::Borracha => "Borracha",
            Finish::Vidro => "Vidro",
        }
    }
    /// (rugosidade, metalicidade, opacidade)
    pub fn params(self) -> (f32, f32, f32) {
        match self {
            Finish::Fosco => (0.95, 0.0, 1.0),
            Finish::Acetinado => (0.6, 0.0, 1.0),
            Finish::Brilhante => (0.25, 0.0, 1.0),
            Finish::MetalEscovado => (0.45, 1.0, 1.0),
            Finish::MetalPolido => (0.15, 1.0, 1.0),
            Finish::Borracha => (1.0, 0.0, 1.0),
            Finish::Vidro => (0.08, 0.0, 0.35),
        }
    }
}

/// Aparência de peça, coletor ou comando. Precedência na exibição: comando > peça > coletor.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Appearance {
    pub color: [u8; 3],
    pub finish: Finish,
}

pub const APPEARANCE_PRESETS: [(&str, Appearance); 8] = [
    ("Aço", Appearance { color: [168, 172, 178], finish: Finish::MetalEscovado }),
    ("Alumínio", Appearance { color: [205, 208, 212], finish: Finish::MetalEscovado }),
    ("Inox polido", Appearance { color: [215, 218, 222], finish: Finish::MetalPolido }),
    ("Latão", Appearance { color: [201, 168, 92], finish: Finish::MetalPolido }),
    ("ABS preto", Appearance { color: [38, 40, 44], finish: Finish::Acetinado }),
    ("Plástico vermelho", Appearance { color: [200, 40, 40], finish: Finish::Brilhante }),
    ("Borracha", Appearance { color: [30, 30, 32], finish: Finish::Borracha }),
    ("Vidro", Appearance { color: [170, 210, 230], finish: Finish::Vidro }),
];

// ---------------------------------------------------------------- Coletores e pastas

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BodyCollector {
    pub id: Id,
    pub name: String,
    pub color: [u8; 3],
    pub visible: bool,
    #[serde(default)]
    pub finish: Finish,
    /// marcador da linha do tempo deste coletor: quantos comandos dele estão ativos (None = todos)
    #[serde(default)]
    pub marker: Option<usize>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: Id,
    pub name: String,
    /// geometria livre (planos, pontos, curvas, sketches — sem linha do tempo) ou comandos de um coletor
    #[serde(default)]
    pub kind: FolderKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum FolderKind {
    /// pasta de organização de comandos dentro de um coletor
    #[default]
    Comandos,
    /// pasta de geometria livre criada pelo projetista (sem linha do tempo)
    Geometria,
}

// ---------------------------------------------------------------- Comandos

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Feature {
    pub id: Id,
    pub name: String,
    pub kind: FeatureKind,
    pub suppressed: bool,
    /// Pasta personalizada (apenas organização; não altera a ordem, RF-ARV-032).
    pub folder: Option<Id>,
    /// Coletor de corpos de destino (RF-ARV-042).
    pub target: Id,
    /// Oculto na viewport (sketches; nos sólidos, oculta as faces geradas por ele).
    #[serde(default)]
    pub hidden: bool,
    /// Aparência das faces geradas por este comando.
    #[serde(default)]
    pub appearance: Option<Appearance>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum FeatureKind {
    Sketch(Sketch),
    Extrude(Extrude),
    Revolve(Revolve),
    Hole(Hole),
    Fillet(EdgeOp),
    Chamfer(EdgeOp),
}

impl FeatureKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            FeatureKind::Sketch(_) => "Sketch",
            FeatureKind::Extrude(e) if e.op == BoolOp::Cut => "Recorte",
            FeatureKind::Extrude(_) => "Extrusão",
            FeatureKind::Revolve(_) => "Revolução",
            FeatureKind::Hole(_) => "Furo",
            FeatureKind::Fillet(_) => "Filete",
            FeatureKind::Chamfer(_) => "Chanfro",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            FeatureKind::Sketch(_) => "✏",
            FeatureKind::Extrude(e) if e.op == BoolOp::Cut => "⬇",
            FeatureKind::Extrude(_) => "⬆",
            FeatureKind::Revolve(_) => "⟳",
            FeatureKind::Hole(_) => "◎",
            FeatureKind::Fillet(_) => "◜",
            FeatureKind::Chamfer(_) => "◸",
        }
    }
}

/// Sketch: plano + geometria livre com restrições (ver [`crate::sketch`]).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Sketch {
    pub plane: PlaneRef,
    #[serde(flatten)]
    pub data: SketchData,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BoolOp {
    Add,
    Cut,
    Intersect,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Extent {
    Blind(String),
    /// Plano médio: metade para cada lado do plano do sketch.
    Symmetric(String),
    TwoSides(String, String),
    ThroughAll,
    /// Até um plano (da Origem ou de referência, inclusive inclinado): termina exatamente nele.
    UpTo(PlaneRef),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Extrude {
    pub sketch: Id,
    pub op: BoolOp,
    pub extent: Extent,
    pub reverse: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Revolve {
    pub sketch: Id,
    pub axis: AxisRef,
    pub angle: String,
    pub symmetric: bool,
    pub op: BoolOp,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum HoleKind {
    Simples,
    Rebaixado { diameter: String, depth: String },
    Escareado { diameter: String, angle: String },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Hole {
    /// Os pontos deste sketch são os centros dos furos.
    pub sketch: Id,
    pub kind: HoleKind,
    pub diameter: String,
    pub depth: Extent,
    /// Rosca paramétrica cosmética (ex.: "M8x1.25"). `None` = sem rosca.
    pub thread: Option<String>,
    /// rosca modelada (sulco helicoidal real, por booleana) em vez de cosmética
    #[serde(default)]
    pub thread_modeled: bool,
    /// Especificação usada pelo furo inteligente (ex.: "M8 normal"), só informativa.
    pub standard: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum EdgeSel {
    /// Todas as arestas do coletor de destino.
    AllOfBody,
    /// Arestas geradas por um comando (referência persistente simplificada).
    CreatedBy(Id),
    /// Arestas escolhidas na viewport, guardadas por assinatura geométrica (ponto médio xyz);
    /// a cada recompute, cada uma é resolvida para a aresta mais próxima (ADR 0004).
    Edges(Vec<[f64; 3]>),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EdgeOp {
    pub value: String,
    pub edges: EdgeSel,
}

// ---------------------------------------------------------------- Documento

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Document {
    pub format_version: u32,
    pub name: String,
    pub props: Properties,
    /// Aparência da peça inteira (sobrepõe a cor dos coletores).
    #[serde(default)]
    pub appearance: Option<Appearance>,
    next_id: u64,
    pub origin_visible: bool,
    pub references: Vec<RefElement>,
    pub params: Vec<Parameter>,
    pub collectors: Vec<BodyCollector>,
    pub active_collector: Id,
    pub folders: Vec<Folder>,
    /// referências externas (geometria de outras peças por link)
    #[serde(default)]
    pub external: Vec<ExternalRef>,
    /// vistas salvas (câmeras)
    #[serde(default)]
    pub views: Vec<NamedView>,
    /// Ordem do vetor = ordem da linha de tempo.
    pub features: Vec<Feature>,
    /// Quantidade de comandos ativos (marcador de rollback, RF-ARV-051).
    pub rollback: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DocError {
    #[error("objeto não encontrado")]
    NotFound,
    #[error("nome já existe: {0}")]
    DuplicateName(String),
    #[error("nome inválido: {0}")]
    InvalidName(String),
    #[error("parâmetro {0}: {1}")]
    Param(String, ExprError),
    #[error("a ordem violaria a dependência de {0}")]
    Order(String),
    #[error("tipos incompatíveis para substituir referência")]
    Incompatible,
    #[error("o documento é somente leitura (revisão congelada)")]
    ReadOnly,
}

const PALETTE: [[u8; 3]; 6] = [[150, 170, 190], [205, 160, 110], [120, 175, 135], [180, 130, 170], [200, 190, 120], [120, 160, 200]];

impl Document {
    pub fn new(name: &str) -> Self {
        let mut d = Document {
            format_version: FORMAT_VERSION,
            name: name.into(),
            props: Properties::default(),
            appearance: None,
            next_id: 1,
            origin_visible: true,
            references: vec![],
            params: vec![],
            collectors: vec![],
            active_collector: Id(0),
            folders: vec![],
            external: vec![],
            views: vec![],
            features: vec![],
            rollback: 0,
        };
        let c = d.add_collector("Corpo principal");
        d.active_collector = c;
        d
    }

    /// Próximo id que será alocado (sem alocar).
    pub fn peek_id(&self) -> u64 {
        self.next_id
    }
    /// Reserva `n` ids a partir de [`Self::peek_id`] (usado por editores que alocam fora do documento).
    pub fn reserve_ids(&mut self, n: u64) {
        self.next_id += n;
    }

    pub fn alloc(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialização")
    }
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    // ------------------------------------------------ nomes

    fn name_taken(&self, name: &str) -> bool {
        self.params.iter().any(|p| p.name == name)
            || self.references.iter().any(|r| r.name == name)
            || self.features.iter().any(|f| f.name == name)
    }

    /// Próximo nome livre: "Extrusão1", "Extrusão2", ...
    pub fn unique_name(&self, base: &str) -> String {
        (1..).map(|i| format!("{base}{i}")).find(|n| !self.name_taken(n)).unwrap()
    }

    // ------------------------------------------------ coletores e pastas

    pub fn add_collector(&mut self, name: &str) -> Id {
        let id = self.alloc();
        let color = PALETTE[self.collectors.len() % PALETTE.len()];
        self.collectors.push(BodyCollector { id, name: name.into(), color, visible: true, finish: Finish::default(), marker: None });
        id
    }

    /// Pasta de geometria livre (sem linha do tempo).
    pub fn add_geometry_folder(&mut self, name: &str) -> Id {
        let id = self.alloc();
        self.folders.push(Folder { id, name: name.into(), kind: FolderKind::Geometria });
        id
    }

    /// Pasta de geometria padrão (cria "Geometria" se ainda não existir).
    pub fn default_geometry_folder(&mut self) -> Id {
        match self.folders.iter().find(|f| f.kind == FolderKind::Geometria) {
            Some(f) => f.id,
            None => self.add_geometry_folder("Geometria"),
        }
    }

    pub fn folder(&self, id: Id) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }

    /// Sketch livre: fica numa pasta de geometria, fora das linhas do tempo dos coletores.
    pub fn is_free(&self, f: &Feature) -> bool {
        matches!(f.kind, FeatureKind::Sketch(_)) && f.folder.and_then(|fo| self.folder(fo)).is_some_and(|fo| fo.kind == FolderKind::Geometria)
    }

    /// O comando está ativo? (não suprimido e antes do marcador do seu coletor e do marcador geral)
    pub fn is_active(&self, idx: usize) -> bool {
        let Some(f) = self.features.get(idx) else { return false };
        if f.suppressed {
            return false;
        }
        if self.is_free(f) {
            return true;
        }
        if idx >= self.rollback {
            return false;
        }
        let marker = self.collectors.iter().find(|c| c.id == f.target).and_then(|c| c.marker);
        match marker {
            None => true,
            Some(m) => self.features[..idx].iter().filter(|g| g.target == f.target && !self.is_free(g)).count() < m,
        }
    }

    pub fn add_folder(&mut self, name: &str) -> Id {
        let id = self.alloc();
        self.folders.push(Folder { id, name: name.into(), kind: FolderKind::Comandos });
        id
    }

    /// Remove a pasta; os comandos dentro dela voltam para a raiz (não são apagados).
    pub fn remove_folder(&mut self, id: Id) {
        self.folders.retain(|f| f.id != id);
        for f in &mut self.features {
            if f.folder == Some(id) {
                f.folder = None;
            }
        }
    }

    // ------------------------------------------------ referências

    /// Coloca o último elemento de referência criado na pasta de geometria padrão.
    fn file_last_reference(&mut self) {
        let fo = self.default_geometry_folder();
        if let Some(r) = self.references.last_mut() {
            if r.folder.is_none() {
                r.folder = Some(fo);
            }
        }
    }

    pub fn add_offset_plane(&mut self, base: PlaneRef, offset: &str) -> Id {
        let id = self.alloc();
        let name = self.unique_name("Plano");
        self.references.push(RefElement { id, name, kind: RefKind::OffsetPlane { base, offset: offset.into() }, visible: true, folder: None });
        self.file_last_reference();
        id
    }

    /// Plano de uma face plana (criado ao clicar numa face para fazer um sketch).
    pub fn add_face_plane(&mut self, origin: [f64; 3], x: [f64; 3], normal: [f64; 3], owner: Option<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique_name("PlanoFace");
        self.references.push(RefElement { id, name, kind: RefKind::FacePlane { origin, x, normal, owner }, visible: true, folder: None });
        self.file_last_reference();
        id
    }

    /// Plano que passa por um eixo, girado `angle` graus a partir de `base`.
    pub fn add_angle_plane(&mut self, base: PlaneRef, axis: AxisRef, angle: &str) -> Id {
        let id = self.alloc();
        let name = self.unique_name("Plano");
        self.references.push(RefElement { id, name, kind: RefKind::AnglePlane { base, axis, angle: angle.into() }, visible: true, folder: None });
        self.file_last_reference();
        id
    }

    /// Cria um elemento de geometria de referência (plano, ponto ou curva) na pasta de geometria padrão.
    pub fn add_geometry(&mut self, kind: RefKind) -> Id {
        let id = self.alloc();
        let base = match kind.class() {
            RefClass::Plane => "Plano",
            RefClass::Point => "Ponto",
            RefClass::Curve => match kind {
                RefKind::LinePoints { .. } | RefKind::LineDir { .. } | RefKind::LineAngle { .. } | RefKind::LineNormalSurface { .. } => "Linha",
                _ => "Curva",
            },
        };
        let name = self.unique_name(base);
        self.references.push(RefElement { id, name, kind, visible: true, folder: None });
        self.file_last_reference();
        id
    }

    pub fn reference(&self, id: Id) -> Option<&RefElement> {
        self.references.iter().find(|r| r.id == id)
    }

    // ------------------------------------------------ parâmetros

    pub fn add_param(&mut self, name: &str, expr: &str, unit: Unit) -> Result<Id, DocError> {
        validate_name(name)?;
        if self.name_taken(name) {
            return Err(DocError::DuplicateName(name.into()));
        }
        let id = self.alloc();
        self.params.push(Parameter { id, name: name.into(), expr: expr.into(), unit, comment: String::new() });
        Ok(id)
    }

    /// Renomeia o parâmetro e reescreve todas as expressões que o usam (edição não destrutiva).
    pub fn rename_param(&mut self, id: Id, new_name: &str) -> Result<(), DocError> {
        validate_name(new_name)?;
        let old = self.params.iter().find(|p| p.id == id).ok_or(DocError::NotFound)?.name.clone();
        if old == new_name {
            return Ok(());
        }
        if self.name_taken(new_name) {
            return Err(DocError::DuplicateName(new_name.into()));
        }
        self.for_each_expr(|e| *e = rename_ident(e, &old, new_name));
        self.params.iter_mut().find(|p| p.id == id).unwrap().name = new_name.into();
        Ok(())
    }

    /// Aplica `f` a todas as expressões do documento.
    pub fn for_each_expr(&mut self, mut f: impl FnMut(&mut String)) {
        for p in &mut self.params {
            f(&mut p.expr)
        }
        for r in &mut self.references {
            r.kind.walk(&mut |_| {}, &mut |e| f(e));
        }
        for feat in &mut self.features {
            match &mut feat.kind {
                FeatureKind::Sketch(s) => {
                    for v in s.data.exprs_mut() {
                        f(v)
                    }
                }
                FeatureKind::Extrude(e) => extent_exprs(&mut e.extent, &mut f),
                FeatureKind::Revolve(r) => f(&mut r.angle),
                FeatureKind::Hole(h) => {
                    f(&mut h.diameter);
                    extent_exprs(&mut h.depth, &mut f);
                    match &mut h.kind {
                        HoleKind::Simples => {}
                        HoleKind::Rebaixado { diameter, depth } => {
                            f(diameter);
                            f(depth)
                        }
                        HoleKind::Escareado { diameter, angle } => {
                            f(diameter);
                            f(angle)
                        }
                    }
                }
                FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => f(&mut o.value),
            }
        }
    }

    /// Avalia todos os parâmetros respeitando dependências; detecta ciclos (RF-PAR-006).
    pub fn eval_params(&self) -> Result<HashMap<String, f64>, DocError> {
        let mut asts = HashMap::new();
        for p in &self.params {
            let ast = expr::parse(&p.expr).map_err(|e| DocError::Param(p.name.clone(), e))?;
            asts.insert(p.name.as_str(), ast);
        }
        let mut vals = HashMap::new();
        let mut visiting = Vec::<&str>::new();
        fn visit<'a>(
            n: &'a str,
            asts: &'a HashMap<&'a str, expr::Ast>,
            vals: &mut HashMap<String, f64>,
            visiting: &mut Vec<&'a str>,
        ) -> Result<(), DocError> {
            if vals.contains_key(n) {
                return Ok(());
            }
            if let Some(pos) = visiting.iter().position(|v| *v == n) {
                let mut cyc: Vec<&str> = visiting[pos..].to_vec();
                cyc.push(n);
                return Err(DocError::Param(n.into(), ExprError::Cycle(cyc.join(" → "))));
            }
            let Some(ast) = asts.get(n) else { return Ok(()) };
            visiting.push(n);
            let mut deps = vec![];
            expr::names(ast, &mut deps);
            for d in &deps {
                if let Some((k, _)) = asts.get_key_value(d.as_str()) {
                    visit(k, asts, vals, visiting)?;
                }
            }
            visiting.pop();
            let v = expr::eval(ast, vals).map_err(|e| DocError::Param(n.into(), e))?;
            vals.insert(n.into(), v);
            Ok(())
        }
        for p in &self.params {
            visit(&p.name, &asts, &mut vals, &mut visiting)?;
        }
        Ok(vals)
    }

    // ------------------------------------------------ comandos

    /// Insere um comando na posição do marcador de rollback (RF-ARV-051).
    /// Insere um comando na linha do tempo do coletor ativo, na posição do marcador dele.
    pub fn add_feature(&mut self, kind: FeatureKind) -> Id {
        let id = self.alloc();
        let name = self.unique_name(kind.type_name());
        let target = self.active_collector;
        let len = self.features.len();
        let marker = self.collectors.iter().find(|c| c.id == target).and_then(|c| c.marker);
        let at = match marker {
            Some(m) => {
                let idxs: Vec<usize> = (0..len).filter(|i| self.features[*i].target == target && !self.is_free(&self.features[*i])).collect();
                if m < idxs.len() { idxs[m] } else { self.rollback.min(len) }
            }
            None => self.rollback.min(len),
        };
        self.features.insert(at, Feature { id, name, kind, suppressed: false, folder: None, target, hidden: false, appearance: None });
        if at < self.rollback {
            self.rollback += 1;
        } else {
            self.rollback = at + 1;
        }
        if let Some(c) = self.collectors.iter_mut().find(|c| c.id == target) {
            if let Some(m) = c.marker.as_mut() {
                *m += 1;
            }
        }
        id
    }

    /// Sketch livre numa pasta de geometria (fora das linhas do tempo).
    pub fn add_free_sketch(&mut self, plane: PlaneRef, folder: Id) -> Id {
        let id = self.alloc();
        let name = self.unique_name("Sketch");
        let target = self.active_collector;
        let kind = FeatureKind::Sketch(Sketch { plane, data: Default::default() });
        self.features.push(Feature { id, name, kind, suppressed: false, folder: Some(folder), target, hidden: false, appearance: None });
        if self.rollback == self.features.len() - 1 {
            self.rollback += 1;
        }
        id
    }

    pub fn feature(&self, id: Id) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }
    pub fn feature_mut(&mut self, id: Id) -> Option<&mut Feature> {
        self.features.iter_mut().find(|f| f.id == id)
    }
    pub fn feature_index(&self, id: Id) -> Option<usize> {
        self.features.iter().position(|f| f.id == id)
    }

    /// Ids que um comando referencia diretamente.
    pub fn refs_of(kind: &FeatureKind) -> Vec<Id> {
        let plane = |p: &PlaneRef| match p {
            PlaneRef::Datum(id) => Some(*id),
            _ => None,
        };
        match kind {
            FeatureKind::Sketch(s) => plane(&s.plane).into_iter().collect(),
            FeatureKind::Extrude(e) => {
                let mut v = vec![e.sketch];
                if let Extent::UpTo(PlaneRef::Datum(id)) = e.extent {
                    v.push(id)
                }
                v
            }
            FeatureKind::Revolve(r) => {
                let mut v = vec![r.sketch];
                let mut ax = r.axis;
                ax.walk(&mut |x| {
                    if *x != r.sketch {
                        v.push(*x)
                    }
                });
                v
            }
            FeatureKind::Hole(h) => vec![h.sketch],
            FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => match &o.edges {
                EdgeSel::CreatedBy(id) => vec![*id],
                EdgeSel::AllOfBody | EdgeSel::Edges(_) => vec![],
            },
        }
    }

    /// Todos os objetos que dependem (direta ou indiretamente) de `id`.
    pub fn dependents(&self, id: Id) -> Vec<Id> {
        let mut out: Vec<Id> = vec![];
        let mut frontier = vec![id];
        while let Some(cur) = frontier.pop() {
            for r in &self.references {
                let hit = r.kind.deps().contains(&cur);
                if hit && !out.contains(&r.id) {
                    out.push(r.id);
                    frontier.push(r.id);
                }
            }
            for f in &self.features {
                if Self::refs_of(&f.kind).contains(&cur) && !out.contains(&f.id) {
                    out.push(f.id);
                    frontier.push(f.id);
                }
            }
        }
        out
    }

    /// Nome de exibição de qualquer objeto.
    pub fn name_of(&self, id: Id) -> String {
        if let Some(f) = self.feature(id) {
            return f.name.clone();
        }
        if let Some(r) = self.reference(id) {
            return r.name.clone();
        }
        if let Some(c) = self.collectors.iter().find(|c| c.id == id) {
            return c.name.clone();
        }
        format!("#{}", id.0)
    }

    /// Substituir referência (RF-NUC-021): tudo que apontava para `old` passa a apontar para `new`.
    /// Os comandos são mantidos; apenas a entrada muda.
    pub fn replace_reference(&mut self, old: Id, new: Id) -> Result<usize, DocError> {
        let kind_of = |d: &Document, id: Id| -> Option<u8> {
            if let Some(f) = d.feature(id) {
                return Some(if matches!(f.kind, FeatureKind::Sketch(_)) { 0 } else { 1 });
            }
            d.reference(id).map(|_| 2)
        };
        match (kind_of(self, old), kind_of(self, new)) {
            (Some(a), Some(b)) if a == b && old != new => {}
            _ => return Err(DocError::Incompatible),
        }
        let mut n = 0;
        let swap = |x: &mut Id, n: &mut usize| {
            if *x == old {
                *x = new;
                *n += 1
            }
        };
        let swap_plane = |p: &mut PlaneRef, n: &mut usize| {
            if let PlaneRef::Datum(x) = p {
                swap(x, n)
            }
        };
        for r in &mut self.references {
            r.kind.walk(&mut |x| swap(x, &mut n), &mut |_| {});
        }
        for f in &mut self.features {
            match &mut f.kind {
                FeatureKind::Sketch(s) => swap_plane(&mut s.plane, &mut n),
                FeatureKind::Extrude(e) => {
                    swap(&mut e.sketch, &mut n);
                    if let Extent::UpTo(p) = &mut e.extent {
                        swap_plane(p, &mut n)
                    }
                }
                FeatureKind::Revolve(r) => {
                    swap(&mut r.sketch, &mut n);
                    r.axis.walk(&mut |x| swap(x, &mut n));
                }
                FeatureKind::Hole(h) => swap(&mut h.sketch, &mut n),
                FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
                    if let EdgeSel::CreatedBy(x) = &mut o.edges {
                        swap(x, &mut n)
                    }
                }
            }
        }
        Ok(n)
    }

    /// Apaga o objeto e, se `with_dependents`, também os dependentes.
    /// Sem `with_dependents`, os dependentes ficam com referência perdida (aviso), mas
    /// **não são apagados** (RF-NUC-024).
    pub fn delete(&mut self, id: Id, with_dependents: bool) {
        let mut kill: HashSet<Id> = HashSet::from([id]);
        if with_dependents {
            kill.extend(self.dependents(id));
        }
        let before_rb = self.features[..self.rollback.min(self.features.len())]
            .iter()
            .filter(|f| kill.contains(&f.id))
            .count();
        self.features.retain(|f| !kill.contains(&f.id));
        self.references.retain(|r| !kill.contains(&r.id));
        self.rollback = self.rollback.saturating_sub(before_rb).min(self.features.len());
    }

    /// Move um comando para outra posição da linha de tempo, se as dependências permitirem (RF-ARV-053).
    pub fn move_feature(&mut self, id: Id, to: usize) -> Result<(), DocError> {
        let from = self.feature_index(id).ok_or(DocError::NotFound)?;
        let to = to.min(self.features.len() - 1);
        let f = self.features.remove(from);
        // tudo que o comando usa precisa vir antes; tudo que o usa precisa vir depois
        let uses = Self::refs_of(&f.kind);
        let users = self.dependents(id);
        let ok_before = self.features[..to].iter().filter(|g| uses.contains(&g.id)).count()
            == self.features.iter().filter(|g| uses.contains(&g.id)).count();
        let ok_after = !self.features[..to].iter().any(|g| users.contains(&g.id));
        if !(ok_before && ok_after) {
            let culprit = if !ok_before { uses } else { users };
            let name = culprit.first().map(|i| self.name_of(*i)).unwrap_or_default();
            self.features.insert(from, f);
            return Err(DocError::Order(name));
        }
        self.features.insert(to, f);
        Ok(())
    }

    /// Comandos ativos (antes do marcador e não suprimidos).
    pub fn active_features(&self) -> impl Iterator<Item = (usize, &Feature)> {
        self.features.iter().enumerate().filter(|(i, _)| self.is_active(*i))
    }
}

fn extent_exprs(e: &mut Extent, f: &mut impl FnMut(&mut String)) {
    match e {
        Extent::Blind(d) | Extent::Symmetric(d) => f(d),
        Extent::TwoSides(a, b) => {
            f(a);
            f(b)
        }
        Extent::ThroughAll | Extent::UpTo(_) => {}
    }
}

fn validate_name(n: &str) -> Result<(), DocError> {
    let ok = n.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok { Ok(()) } else { Err(DocError::InvalidName(n.into())) }
}

/// Troca o identificador `old` por `new` respeitando limites de palavra.
pub fn rename_ident(src: &str, old: &str, new: &str) -> String {
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
    let mut out = String::with_capacity(src.len());
    let mut word = String::new();
    for c in src.chars().chain(std::iter::once('\0')) {
        if is_id(c) {
            word.push(c);
        } else {
            if !word.is_empty() {
                out.push_str(if word == old { new } else { &word });
                word.clear();
            }
            if c != '\0' {
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.into()
    }

    fn sketch_rect(d: &mut Document) -> Id {
        d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: Default::default() }))
    }

    #[test]
    fn documento_novo_tem_corpo_principal() {
        let d = Document::new("Peça");
        assert_eq!(d.collectors.len(), 1);
        assert_eq!(d.active_collector, d.collectors[0].id);
    }

    #[test]
    fn parametros_dependencias_e_ciclo() {
        let mut d = Document::new("p");
        d.add_param("L", "A*2", Unit::Comprimento).unwrap();
        d.add_param("A", "10", Unit::Comprimento).unwrap();
        assert_eq!(d.eval_params().unwrap()["L"], 20.0);
        d.params[1].expr = s("L/2");
        assert!(matches!(d.eval_params(), Err(DocError::Param(_, ExprError::Cycle(_)))));
    }

    #[test]
    fn renomear_parametro_reescreve_expressoes() {
        let mut d = Document::new("p");
        let a = d.add_param("A", "10", Unit::Comprimento).unwrap();
        d.add_param("AB", "A+1", Unit::Comprimento).unwrap();
        d.rename_param(a, "Largura").unwrap();
        assert_eq!(d.params[1].expr, "Largura+1");
        assert_eq!(rename_ident("A*AB+A", "A", "X"), "X*AB+X");
    }

    #[test]
    fn rollback_insere_no_marcador() {
        let mut d = Document::new("p");
        let a = sketch_rect(&mut d);
        let b = sketch_rect(&mut d);
        d.rollback = 1;
        let c = sketch_rect(&mut d);
        assert_eq!(d.features.iter().map(|f| f.id).collect::<Vec<_>>(), vec![a, c, b]);
        assert_eq!(d.rollback, 2);
    }

    #[test]
    fn dependentes_substituir_e_apagar() {
        let mut d = Document::new("p");
        let sk1 = sketch_rect(&mut d);
        let sk2 = sketch_rect(&mut d);
        let ex = d.add_feature(FeatureKind::Extrude(Extrude {
            sketch: sk1,
            op: BoolOp::Add,
            extent: Extent::Blind(s("10")),
            reverse: false,
        }));
        let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::CreatedBy(ex) }));
        assert_eq!(d.dependents(sk1), vec![ex, fil]);

        // substituir referência mantém o comando e muda a entrada
        assert_eq!(d.replace_reference(sk1, sk2).unwrap(), 1);
        assert!(d.dependents(sk1).is_empty());
        assert!(matches!(d.replace_reference(sk2, ex), Err(DocError::Incompatible)));

        // apagar sem dependentes não remove os comandos que usam
        d.delete(sk2, false);
        assert!(d.feature(ex).is_some());
        d.delete(ex, true);
        assert!(d.feature(fil).is_none());
        assert_eq!(d.rollback, d.features.len());
    }

    #[test]
    fn mover_respeita_dependencias() {
        let mut d = Document::new("p");
        let sk = sketch_rect(&mut d);
        let ex = d.add_feature(FeatureKind::Extrude(Extrude {
            sketch: sk,
            op: BoolOp::Add,
            extent: Extent::Blind(s("10")),
            reverse: false,
        }));
        assert!(d.move_feature(ex, 0).is_err());
        assert_eq!(d.features[1].id, ex);
        let sk2 = sketch_rect(&mut d);
        d.move_feature(sk2, 0).unwrap();
        assert_eq!(d.features[0].id, sk2);
    }

    #[test]
    fn marcador_por_coletor_e_sketch_livre() {
        let mut d = Document::new("p");
        let a = sketch_rect(&mut d);
        let b = sketch_rect(&mut d);
        let c2 = d.add_collector("Corpo 2");
        d.feature_mut(b).unwrap().target = c2;
        // marcador do coletor principal em 0: só o sketch de Corpo 2 fica ativo
        d.collectors[0].marker = Some(0);
        assert!(!d.is_active(0) && d.is_active(1));
        // sketch numa pasta de geometria é livre (fora da linha do tempo)
        let g = d.add_geometry_folder("Geometria");
        d.feature_mut(a).unwrap().folder = Some(g);
        assert!(d.is_active(0));
        // planos novos vão para a pasta de geometria
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        assert_eq!(d.reference(pl).unwrap().folder, Some(g));
    }

    #[test]
    fn json_ida_e_volta() {
        let mut d = Document::new("p");
        d.add_param("L", "100", Unit::Comprimento).unwrap();
        sketch_rect(&mut d);
        let d2 = Document::from_json(&d.to_json()).unwrap();
        assert_eq!(d2.features.len(), 1);
        assert_eq!(d2.params[0].name, "L");
    }
}
