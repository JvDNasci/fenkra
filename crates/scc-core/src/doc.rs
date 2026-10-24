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
    /// ponto de um sketch (associativo)
    SketchPoint { sketch: Id, point: Id },
    /// vértice de uma linha/polilinha/spline de referência (0 = início; nas polilinhas e
    /// splines, o índice do ponto de definição; nas demais, 1 = fim e 2 = meio)
    CurveVertex { curve: Id, index: usize },
}

impl Default for PointRef {
    fn default() -> Self {
        PointRef::Origin
    }
}

impl PointRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        match self {
            PointRef::Ref(x) => ids(x),
            PointRef::SketchPoint { sketch, .. } => ids(sketch),
            PointRef::CurveVertex { curve, .. } => ids(curve),
            PointRef::Origin | PointRef::Fixed(_) => {}
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
    /// qualquer curva de um sketch (linha, arco, círculo), associativa
    SketchCurve { sketch: Id, curve: Id },
    /// um trecho (segmento i → i+1) de uma polilinha de referência
    Segment { curve: Id, index: usize },
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
            CurveRef::SketchLine { sketch, .. } | CurveRef::SketchCurve { sketch, .. } => ids(sketch),
            CurveRef::Segment { curve, .. } => ids(curve),
            CurveRef::Axis(_) | CurveRef::Edge(_) => {}
        }
    }
}

/// Superfície usada como entrada: plano ou face clicada (ponto e normal, fixa nesta versão).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum SurfRef {
    Plane(PlaneRef),
    Face { point: [f64; 3], normal: [f64; 3], owner: Option<Id> },
    /// superfície de referência (pasta de geometria)
    Ref(Id),
}

impl Default for SurfRef {
    fn default() -> Self {
        SurfRef::Plane(PlaneRef::default())
    }
}

impl SurfRef {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id)) {
        match self {
            SurfRef::Plane(p) => p.walk(ids),
            SurfRef::Ref(x) => ids(x),
            SurfRef::Face { .. } => {}
        }
    }
}

/// Cópia da geometria publicada por outra peça (guardada aqui; "atualizar links" relê a origem).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum LinkedGeo {
    Plane { origin: [f64; 3], x: [f64; 3], normal: [f64; 3] },
    Point([f64; 3]),
    Curve(Vec<[f64; 3]>),
    /// parâmetro publicado (valor)
    Value(f64),
    /// superfície (texto B-rep)
    Surface(String),
}

impl LinkedGeo {
    pub fn class(&self) -> RefClass {
        match self {
            LinkedGeo::Plane { .. } => RefClass::Plane,
            LinkedGeo::Point(_) => RefClass::Point,
            LinkedGeo::Surface(_) => RefClass::Surface,
            _ => RefClass::Curve,
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
    /// variação da curvatura contínua (só em curvas; nas superfícies vale como G2)
    G3,
}

/// Perfil da varredura de superfície.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SweepProfile {
    /// curva desenhada (explícito)
    Curve(CurveRef),
    /// círculo de raio dado, normal à guia (tubo)
    Circle(String),
}

/// Transformação de um elemento de geometria.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum GeoXform {
    Translate { dir: AxisRef, dist: String },
    Rotate { axis: AxisRef, angle: String },
    Mirror { plane: PlaneRef },
    Scale { center: PointRef, factor: String },
}

impl GeoXform {
    fn walk(&mut self, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
        match self {
            GeoXform::Translate { dir, dist } => {
                dir.walk(ids);
                exprs(dist)
            }
            GeoXform::Rotate { axis, angle } => {
                axis.walk(ids);
                exprs(angle)
            }
            GeoXform::Mirror { plane } => plane.walk(ids),
            GeoXform::Scale { center, factor } => {
                center.walk(ids);
                exprs(factor)
            }
        }
    }
}

/// Classe do elemento de geometria de referência.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RefClass {
    Plane,
    Point,
    Curve,
    Surface,
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
    /// publicado para outras peças usarem por link (o nome é a publicação)
    #[serde(default)]
    pub published: bool,
    /// propriedades gráficas próprias (cor, acabamento)
    #[serde(default)]
    pub appearance: Option<Appearance>,
}

/// Estilo de exibição de linhas, curvas e pontos (como os tipos de linha do AutoCAD).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct LineStyle {
    /// espessura em pixels
    pub width: f32,
    /// 0 contínua, 1 tracejada, 2 traço-ponto, 3 pontilhada
    pub dash: u8,
    /// símbolo do ponto: 0 cruz 3D, 1 asterisco, 2 X, 3 círculo, 4 quadrado, 5 ponto
    pub point: u8,
    /// tamanho do símbolo (px)
    pub size: f32,
}

impl Document {
    pub fn line_style(&self, id: Id) -> Option<LineStyle> {
        self.line_styles.iter().find(|(x, _)| *x == id).map(|(_, s)| *s)
    }
    pub fn set_line_style(&mut self, id: Id, st: Option<LineStyle>) {
        self.line_styles.retain(|(x, _)| *x != id);
        if let Some(st) = st {
            self.line_styles.push((id, st));
        }
    }
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
    /// desligado provisoriamente: o elemento fica inativo (como suprimido) até reativar
    Desativado,
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
    /// arquivo da peça de origem (relativo à pasta do projeto, ou absoluto)
    #[serde(default)]
    pub path: String,
    /// geometria/valor copiado na última atualização
    #[serde(default)]
    pub snapshot: Option<LinkedGeo>,
    /// elemento ou parâmetro desta peça que recebe a cópia
    #[serde(default)]
    pub target: Option<Id>,
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

    // ---------------- modos adicionais
    /// Interseção de uma curva com um plano (solução 0, 1, …).
    PointCurvePlane { curve: CurveRef, plane: PlaneRef, which: usize },
    /// Interseção (ou ponto de menor distância) entre duas curvas.
    PointCurveCurve { a: CurveRef, b: CurveRef },
    /// Projeção de um ponto numa superfície ou plano.
    PointProject { point: PointRef, surface: SurfRef },
    /// Projeção de um ponto numa curva (ponto mais próximo).
    PointProjectCurve { point: PointRef, curve: CurveRef },
    /// Ponto deslocado de outro ao longo de uma direção.
    PointOffset { point: PointRef, dir: AxisRef, dist: String },
    /// Linha de interseção de dois planos.
    LinePlanes { a: PlaneRef, b: PlaneRef, length: String },
    /// Bissetriz de duas linhas.
    LineBisector { a: CurveRef, b: CurveRef, length: String },
    /// Eixo de uma face cilíndrica ou cônica (ponto clicado na face).
    LineAxis { face: [f64; 3] },
    /// Plano por um ponto, normal a uma linha.
    PlanePointNormal { point: PointRef, line: CurveRef },
    /// Plano médio entre dois planos.
    PlaneMid { a: PlaneRef, b: PlaneRef },
    /// Curva projetada num plano.
    CurveProject { curve: CurveRef, plane: PlaneRef },
    /// Spline pelo polígono de controle (B-spline uniforme).
    SplineControl { points: Vec<PointRef>, closed: bool },
    /// Cópia de uma curva (aresta extraída do modelo).
    CurveCopy { curve: CurveRef },
    /// Junta curvas que se tocam nas pontas numa curva só (contorno fechado vira perfil de sólido).
    CurveJoin { curves: Vec<CurveRef> },

    // ---------------- superfícies
    /// Face extraída do corpo (ponto clicado).
    SurfExtract { face: [f64; 3], owner: Option<Id> },
    /// Superfície deslocada.
    SurfOffset { surface: SurfRef, dist: String },
    /// Preenchimento de um contorno de curvas.
    SurfFill { curves: Vec<CurveRef> },
    /// Curva extrudada ao longo de uma direção.
    SurfExtrude {
        curve: CurveRef,
        dir: AxisRef,
        length: String,
        /// limite 2: comprimento para o outro lado
        #[serde(default = "zero")]
        length2: String,
    },
    /// Curva girada em torno de um eixo.
    SurfRevolve { curve: CurveRef, axis: AxisRef, angle: String },

    // ---------------- bancada de superfícies
    /// Esfera pelo centro e raio.
    SurfSphere { center: PointRef, radius: String },
    /// Cilindro (só a face lateral) por ponto e direção; comprimento para cada lado.
    SurfCylinder { point: PointRef, dir: AxisRef, radius: String, start: String, end: String },
    /// Varredura: perfil explícito ou circular ao longo da guia.
    SurfSweep { profile: SweepProfile, spine: CurveRef, mode: SweepMode },
    /// Multi-seções por curvas, com guia opcional.
    SurfLoft { sections: Vec<CurveRef>, guide: Option<CurveRef>, ruled: bool, closed: bool },
    /// Concordância entre duas bordas, com continuidade G0/G1/G2 às superfícies de apoio.
    SurfBlend { a: CurveRef, sa: Option<SurfRef>, b: CurveRef, sb: Option<SurfRef>, cont: Continuity, tension: String },
    /// Juntar superfícies (costura) numa só.
    SurfJoin { items: Vec<SurfRef>, tol: String },
    /// Curar: fecha frestas até a tolerância e une faces do mesmo suporte.
    SurfHeal { surface: SurfRef, tol: String },
    /// Dividir uma superfície por outra (ou plano) e ficar com um lado.
    SurfSplit { surface: SurfRef, tool: SurfRef, flip: bool },
    /// Aparar duas superfícies uma pela outra e juntar os lados escolhidos.
    SurfTrim { a: SurfRef, b: SurfRef, flip_a: bool, flip_b: bool },
    /// Extrair várias faces de uma vez (multi-extração).
    SurfMulti { faces: Vec<[f64; 3]> },
    /// Uma face de uma superfície composta (desmontar).
    SurfPart { surface: SurfRef, index: usize },
    /// Desaparar: devolve a face aos limites naturais do suporte.
    SurfUntrim { surface: SurfRef },
    /// Extrapolar: prolongar a borda mais próxima do ponto.
    SurfExtrapolate {
        surface: SurfRef,
        near: PointRef,
        length: String,
        /// mantém a curvatura (mesma superfície) em vez de prolongar tangente
        #[serde(default)]
        curvature: bool,
    },
    /// Translação, rotação, simetria ou escala de qualquer elemento (ponto, curva, plano, superfície).
    GeoTransform { source: Id, class: RefClass, xf: GeoXform },
    /// Curva de interseção entre duas superfícies (ou planos).
    CurveIntersect { a: SurfRef, b: SurfRef, which: usize },
    /// Curva projetada (na normal) sobre uma superfície.
    CurveProjectSurf {
        curve: CurveRef,
        surface: SurfRef,
        /// projeção ao longo de uma direção; `None` = normal à superfície
        #[serde(default)]
        dir: Option<AxisRef>,
    },
    /// Curva paralela sobre um plano/superfície de apoio, com distância constante ou variando
    /// linearmente do início (d1) ao fim (d2).
    CurveParallel { curve: CurveRef, support: SurfRef, d1: String, d2: String, flip: bool },
    /// Contorno (borda livre) de uma superfície.
    CurveBoundary { surface: SurfRef, which: usize },

    /// Geometria vinda de outra peça por link (ver `Document.external`).
    Linked { ext: Id, class: RefClass },
    /// Cópia isolada (link quebrado de propósito): a geometria fica fixa nesta peça.
    Isolated { geo: LinkedGeo },
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
            PlanePointNormal { .. } | PlaneMid { .. } => RefClass::Plane,
            PointCoords { .. } | PointOnCurve { .. } | PointOnPlane { .. } | PointOnSurface { .. } | PointCenter { .. } | PointTangent { .. }
            | PointBetween { .. } | PointCurvePlane { .. } | PointCurveCurve { .. } | PointProject { .. } | PointProjectCurve { .. }
            | PointOffset { .. } => RefClass::Point,
            SurfExtract { .. } | SurfOffset { .. } | SurfFill { .. } | SurfExtrude { .. } | SurfRevolve { .. } => RefClass::Surface,
            SurfSphere { .. } | SurfCylinder { .. } | SurfSweep { .. } | SurfLoft { .. } | SurfBlend { .. } | SurfJoin { .. } | SurfHeal { .. }
            | SurfSplit { .. } | SurfTrim { .. } | SurfMulti { .. } | SurfPart { .. } | SurfUntrim { .. } | SurfExtrapolate { .. } => RefClass::Surface,
            GeoTransform { class, .. } => *class,
            Linked { class, .. } => *class,
            Isolated { geo } => geo.class(),
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
            PointCurvePlane { curve, plane, .. } | CurveProject { curve, plane } => {
                curve.walk(ids);
                plane.walk(ids)
            }
            PointCurveCurve { a, b } => {
                a.walk(ids);
                b.walk(ids)
            }
            PointProject { point, surface } => {
                point.walk(ids);
                surface.walk(ids)
            }
            PointProjectCurve { point, curve } => {
                point.walk(ids);
                curve.walk(ids)
            }
            PointOffset { point, dir, dist } => {
                point.walk(ids);
                dir.walk(ids);
                exprs(dist)
            }
            LinePlanes { a, b, length } => {
                a.walk(ids);
                b.walk(ids);
                exprs(length)
            }
            LineBisector { a, b, length } => {
                a.walk(ids);
                b.walk(ids);
                exprs(length)
            }
            LineAxis { .. } | SurfExtract { .. } | Linked { .. } | Isolated { .. } => {}
            PlanePointNormal { point, line } => {
                point.walk(ids);
                line.walk(ids)
            }
            PlaneMid { a, b } => {
                a.walk(ids);
                b.walk(ids)
            }
            SplineControl { points, .. } => {
                for q in points {
                    q.walk(ids)
                }
            }
            CurveCopy { curve } => curve.walk(ids),
            SurfOffset { surface, dist } => {
                surface.walk(ids);
                exprs(dist)
            }
            SurfFill { curves } | CurveJoin { curves } => {
                for c in curves {
                    c.walk(ids)
                }
            }
            SurfExtrude { curve, dir, length, length2 } => {
                curve.walk(ids);
                dir.walk(ids);
                exprs(length);
                exprs(length2)
            }
            SurfRevolve { curve, axis, angle } => {
                curve.walk(ids);
                axis.walk(ids);
                exprs(angle)
            }
            SurfSphere { center, radius } => {
                center.walk(ids);
                exprs(radius)
            }
            SurfCylinder { point, dir, radius, start, end } => {
                point.walk(ids);
                dir.walk(ids);
                exprs(radius);
                exprs(start);
                exprs(end)
            }
            SurfSweep { profile, spine, .. } => {
                match profile {
                    SweepProfile::Curve(c) => c.walk(ids),
                    SweepProfile::Circle(r) => exprs(r),
                }
                spine.walk(ids)
            }
            SurfLoft { sections, guide, .. } => {
                for c in sections {
                    c.walk(ids)
                }
                if let Some(g) = guide {
                    g.walk(ids)
                }
            }
            SurfBlend { a, sa, b, sb, tension, .. } => {
                a.walk(ids);
                b.walk(ids);
                for s in [sa, sb].into_iter().flatten() {
                    s.walk(ids)
                }
                exprs(tension)
            }
            SurfJoin { items, tol } => {
                for s in items {
                    s.walk(ids)
                }
                exprs(tol)
            }
            SurfHeal { surface, tol } => {
                surface.walk(ids);
                exprs(tol)
            }
            SurfSplit { surface, tool, .. } => {
                surface.walk(ids);
                tool.walk(ids)
            }
            SurfTrim { a, b, .. } | CurveIntersect { a, b, .. } => {
                a.walk(ids);
                b.walk(ids)
            }
            SurfMulti { .. } => {}
            SurfPart { surface, .. } | SurfUntrim { surface } | CurveBoundary { surface, .. } => surface.walk(ids),
            SurfExtrapolate { surface, near, length, .. } => {
                surface.walk(ids);
                near.walk(ids);
                exprs(length)
            }
            GeoTransform { source, xf, .. } => {
                ids(source);
                xf.walk(ids, exprs)
            }
            CurveProjectSurf { curve, surface, dir } => {
                curve.walk(ids);
                surface.walk(ids);
                if let Some(a) = dir {
                    a.walk(ids)
                }
            }
            CurveParallel { curve, support, d1, d2, .. } => {
                curve.walk(ids);
                support.walk(ids);
                exprs(d1);
                exprs(d2)
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
    /// publicado para outras peças usarem por link
    #[serde(default)]
    pub published: bool,
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
    /// textura de imagem (índice em `Document.textures`), multiplicada pela cor
    #[serde(default)]
    pub texture: Option<u8>,
}

/// Imagem usada como textura (mapeamento triplanar: não precisa de coordenadas UV).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TextureRef {
    pub name: String,
    /// arquivo PNG/JPG (absoluto ou relativo à pasta do projeto)
    pub path: String,
    /// tamanho de uma repetição da imagem, em mm
    pub scale: f64,
}

pub const APPEARANCE_PRESETS: [(&str, Appearance); 8] = [
    ("Aço", Appearance { color: [168, 172, 178], finish: Finish::MetalEscovado, texture: None }),
    ("Alumínio", Appearance { color: [205, 208, 212], finish: Finish::MetalEscovado, texture: None }),
    ("Inox polido", Appearance { color: [215, 218, 222], finish: Finish::MetalPolido, texture: None }),
    ("Latão", Appearance { color: [201, 168, 92], finish: Finish::MetalPolido, texture: None }),
    ("ABS preto", Appearance { color: [38, 40, 44], finish: Finish::Acetinado, texture: None }),
    ("Plástico vermelho", Appearance { color: [200, 40, 40], finish: Finish::Brilhante, texture: None }),
    ("Borracha", Appearance { color: [30, 30, 32], finish: Finish::Borracha, texture: None }),
    ("Vidro", Appearance { color: [170, 210, 230], finish: Finish::Vidro, texture: None }),
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
    #[serde(default)]
    pub texture: Option<u8>,
    /// marcador da linha do tempo deste coletor: quantos comandos dele estão ativos (None = todos)
    #[serde(default)]
    pub marker: Option<usize>,
    /// corpo publicado para outras peças (importado como forma B-rep)
    #[serde(default)]
    pub published: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: Id,
    pub name: String,
    /// geometria livre (planos, pontos, curvas, sketches — sem linha do tempo) ou comandos de um coletor
    #[serde(default)]
    pub kind: FolderKind,
    /// pasta-mãe (subpastas de geometria)
    #[serde(default)]
    pub parent: Option<Id>,
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
    /// Nervura (adiciona) ou rasgo (remove): perfil varrido ao longo de uma curva guia.
    Sweep(Sweep),
    /// Multi-seções (loft) que adiciona ou remove.
    Loft(Loft),
    /// Sólido primitivo (caixa, cilindro, esfera, cone, toro, cunha, prisma).
    Primitive(Primitive),
    Shell(Shell),
    Draft(Draft),
    Thickness(Thickness),
    Pattern(Pattern),
    Mirror(Mirror),
    Scale(Scale),
    /// Reforço: parede fina a partir das linhas de um sketch.
    Stiffener(Stiffener),
    /// Rosca numa face cilíndrica (eixo ou furo).
    Thread(Thread),
    /// Booleana entre coletores: o corpo de `tool` une, subtrai ou intersecta o deste coletor.
    Combine(Combine),
    /// Dividir o corpo por uma superfície ou plano e ficar com um lado (Split).
    SplitBody(SplitBody),
    /// Engrossar superfície em sólido (Thick Surface / Thicken).
    ThickSurface(ThickSurface),
    /// Fechar superfície em sólido (Close Surface / Stitch).
    CloseSurface(CloseSurface),
    /// Costurar superfície no corpo (Sew Surface).
    SewSurface(SewSurface),
    /// Preencher fronteiras: volumes fechados por superfícies e planos (Boundary Fill).
    BoundaryFill(BoundaryFill),
    /// Malha STL/OBJ convertida em B-rep.
    MeshImport(MeshImport),
    /// Mover o corpo inteiro: translação, rotação, simetria ou afinidade.
    MoveBody(MoveBody),
    /// Chapa: parede base a partir de um perfil fechado (espessura global).
    SmWall(SmWall),
    /// Chapa: aba de aresta (dobra + parede reta).
    SmFlange(SmFlange),
    /// Chapa: bainha (dobra de 180° na borda).
    SmHem(SmHem),
    /// Chapa: joggle (desvio em Z) a partir de uma linha na face.
    SmJoggle(SmJoggle),
    /// dobra por linha (chapa já plana dobrada numa linha da face)
    SmBend(SmBend),
    /// veneziana (lanceta levantada)
    SmLouver(SmLouver),
    /// Chapa: estampo (rebaixo/reforço) com o perfil de um sketch na face.
    SmStamp(SmStamp),
    /// Chapa: furo flangeado (colar dobrado) nos pontos de um sketch.
    SmFlangedHole(SmFlangedHole),
    /// Estrutural: membros (perfis) ao longo de linhas/curvas do esqueleto.
    StructMember(StructMember),
    /// Estrutural: placa de extremidade na ponta de um membro.
    StructPlate(StructPlate),
    /// Estrutural: reforço (cantoneira/gusset) entre dois membros.
    StructGusset(StructGusset),
}

/// Tipo de seção de perfil estrutural.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SectionKind {
    TuboQuadrado,
    TuboRetangular,
    TuboRedondo,
    PerfilI,
    PerfilU,
    Cantoneira,
    BarraChata,
    BarraRedonda,
    BarraQuadrada,
    PerfilT,
}

/// Tratamento dos cantos onde dois membros se encontram.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum CornerMode {
    Nenhum,
    #[default]
    MeiaEsquadria,
    /// o membro posterior para na face do anterior
    Aparar,
    /// recorte de encaixe: a ponta acompanha o contorno do outro membro
    Encaixe,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct StructMember {
    /// linhas guia (cada uma vira um membro)
    pub paths: Vec<CurveRef>,
    pub family: String,
    pub designation: String,
    pub kind: SectionKind,
    /// cotas da seção (aceitam expressões)
    pub dims: Vec<String>,
    /// ponto de ancoragem da seção na linha guia (0 centro, 1–8 caixa, 9 centróide)
    pub anchor: u8,
    /// rotação da seção em torno da linha guia (graus)
    pub angle: String,
    pub off_x: String,
    pub off_y: String,
    /// prolongamento no início e no fim
    pub ext1: String,
    pub ext2: String,
    pub corner: CornerMode,
    /// folga de solda nos cantos
    pub gap: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct StructPlate {
    /// ponto perto da ponta do membro
    pub at: [f64; 3],
    pub thickness: String,
    /// sobra além da seção, em cada lado
    pub margin: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct StructGusset {
    /// ponto perto do encontro dos dois membros
    pub at: [f64; 3],
    pub leg: String,
    pub thickness: String,
    /// chanfro de alívio no canto (0 = triangular)
    pub chamfer: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmJoggle {
    /// dois pontos na face da chapa: a linha do joggle
    pub a: [f64; 3],
    pub b: [f64; 3],
    /// desvio (normal à chapa) e comprimento da rampa
    pub depth: String,
    pub run: String,
    /// lado da linha que desce (o outro fica)
    pub flip: bool,
}

/// Dobra por linha: a chapa é dobrada ao longo da linha a–b da face (linha de centro da dobra).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmBend {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub angle: String,
    /// raio interno próprio (senão o da chapa)
    #[serde(default)]
    pub radius: Option<String>,
    /// lado da linha que se move (o outro fica)
    pub flip: bool,
    /// dobrar para baixo (para dentro do material) em vez de para cima
    #[serde(default)]
    pub down: bool,
}

/// Veneziana: corte na linha a–b e a chapa atrás dela levantada em rampa (ventilação).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmLouver {
    pub a: [f64; 3],
    pub b: [f64; 3],
    /// largura da rampa (perpendicular à linha) e altura da abertura
    pub width: String,
    pub height: String,
    /// lado da linha onde fica a rampa
    pub flip: bool,
    /// levantar para o outro lado da chapa
    #[serde(default)]
    pub down: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmStamp {
    /// perfil fechado num sketch no plano da face da chapa
    pub sketch: Id,
    pub depth: String,
    /// estampar para o outro lado
    pub flip: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmFlangedHole {
    /// pontos do sketch = centros
    pub sketch: Id,
    pub diameter: String,
    /// altura do colar
    pub height: String,
    pub flip: bool,
}

/// Parâmetros globais de chapa metálica da peça.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SheetParams {
    pub thickness: String,
    /// raio interno de dobra padrão
    pub radius: String,
    /// fator K (posição da linha neutra: 0 = face interna, 0,5 = meio)
    pub k: String,
    /// peça mostrada planificada (desdobrada)
    #[serde(default)]
    pub flat: bool,
}

impl Default for SheetParams {
    fn default() -> Self {
        SheetParams { thickness: "2".into(), radius: "2".into(), k: "0.44".into(), flat: false }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmWall {
    pub sketch: Id,
    /// espessura para o outro lado do plano do sketch
    pub reverse: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmFlange {
    /// ponto clicado na aresta reta da chapa
    pub edge: [f64; 3],
    /// comprimento reto depois da dobra
    pub length: String,
    pub angle: String,
    /// raio interno (`None` = o da peça)
    pub radius: Option<String>,
    /// recuo lateral a partir de cada ponta da aresta
    pub off1: String,
    pub off2: String,
    /// dobrar para o outro lado
    pub flip: bool,
    /// alívio de canto nas pontas recuadas (largura); `None` = sem alívio
    #[serde(default)]
    pub relief: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SmHem {
    pub edge: [f64; 3],
    pub length: String,
    /// folga interna (0 = esmagada; > 0 = aberta/circular)
    pub gap: String,
    pub flip: bool,
}

/// Transformação do corpo (o original deixa de existir na posição antiga).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum BodyXf {
    Translate { dir: AxisRef, dist: String },
    Rotate { axis: AxisRef, angle: String },
    Symmetry { plane: PlaneRef },
    /// escala independente em X, Y, Z do sistema do plano (x do plano, y, normal)
    Affinity { plane: PlaneRef, fx: String, fy: String, fz: String },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MoveBody {
    pub xf: BodyXf,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SplitBody {
    pub surface: SurfRef,
    /// fica com o lado oposto à normal
    pub flip: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ThickSurface {
    pub surface: SurfRef,
    /// espessura para o lado da normal
    pub t1: String,
    /// espessura para o outro lado
    pub t2: String,
    pub op: BoolOp,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CloseSurface {
    pub surface: SurfRef,
    /// tolerância de costura (Stitch)
    pub tol: String,
    pub op: BoolOp,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SewSurface {
    pub surface: SurfRef,
    /// remove o material do lado escolhido em vez de acrescentar o volume entre a superfície e o corpo
    pub remove: bool,
    pub flip: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BoundaryFill {
    pub tools: Vec<SurfRef>,
    /// células que não entram (um ponto dentro de cada)
    pub exclude: Vec<[f64; 3]>,
    pub op: BoolOp,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MeshImport {
    /// arquivo .stl ou .obj (relativo à pasta do projeto ou absoluto)
    pub path: String,
    /// tenta fechar em sólido (senão fica superfície/casca)
    pub solid: bool,
}

impl FeatureKind {
    pub fn type_name(&self) -> &'static str {
        use FeatureKind::*;
        match self {
            Sketch(_) => "Sketch",
            Extrude(e) if e.op == BoolOp::Cut => "Recorte",
            Extrude(_) => "Extrusão",
            Revolve(r) if r.op == BoolOp::Cut => "Ranhura",
            Revolve(_) => "Revolução",
            Hole(_) => "Furo",
            Fillet(_) => "Filete",
            Chamfer(_) => "Chanfro",
            Sweep(w) if w.op == BoolOp::Cut => "Rasgo",
            Sweep(_) => "Nervura",
            Loft(l) if l.op == BoolOp::Cut => "MultiSeçõesRemovida",
            Loft(_) => "MultiSeções",
            Primitive(p) => p.kind.label(),
            Shell(_) => "Casca",
            Draft(_) => "Saída",
            Thickness(_) => "Espessura",
            Pattern(p) => match p.kind {
                PatternKind::Linear { .. } => "PadrãoLinear",
                PatternKind::Circular { .. } => "PadrãoCircular",
                PatternKind::Points { .. } => "PadrãoPontos",
            },
            Mirror(_) => "Espelho",
            Scale(_) => "Escala",
            Stiffener(_) => "Reforço",
            Thread(_) => "Rosca",
            Combine(c) => match c.op {
                BoolOp::Add => "Unir",
                BoolOp::Cut => "Subtrair",
                BoolOp::Intersect => "Intersecao",
            },
            SplitBody(_) => "Dividir",
            ThickSurface(_) => "Engrossar",
            CloseSurface(_) => "Fechar",
            SewSurface(_) => "Costurar",
            BoundaryFill(_) => "PreencherFronteiras",
            MeshImport(_) => "Malha",
            SmWall(_) => "ParedeBase",
            SmFlange(_) => "Aba",
            SmHem(_) => "Bainha",
            SmJoggle(_) => "Joggle",
            SmBend(_) => "Dobra por linha",
            SmLouver(_) => "Veneziana",
            SmStamp(_) => "Estampo",
            SmFlangedHole(_) => "FuroFlangeado",
            StructMember(_) => "Membro",
            StructPlate(_) => "Placa",
            StructGusset(_) => "Reforço",
            MoveBody(m) => match m.xf {
                BodyXf::Translate { .. } => "Translação",
                BodyXf::Rotate { .. } => "Rotação",
                BodyXf::Symmetry { .. } => "Simetria",
                BodyXf::Affinity { .. } => "Afinidade",
            },
        }
    }
    pub fn icon(&self) -> &'static str {
        use FeatureKind::*;
        match self {
            Sketch(_) => "✏",
            Extrude(e) if e.op == BoolOp::Cut => "⬇",
            Extrude(_) => "⬆",
            Revolve(_) => "⟳",
            Hole(_) => "◎",
            Fillet(_) => "◜",
            Chamfer(_) => "◸",
            Sweep(_) => "〰",
            Loft(_) => "⧖",
            Primitive(_) => "▣",
            Shell(_) => "▢",
            Draft(_) => "⏢",
            Thickness(_) => "▤",
            Pattern(_) => "⁂",
            Mirror(_) => "⇋",
            Scale(_) => "⤢",
            Stiffener(_) => "┃",
            Thread(_) => "⌇",
            Combine(_) => "⊕",
            SplitBody(_) => "⫽",
            ThickSurface(_) => "▥",
            CloseSurface(_) => "⬢",
            SewSurface(_) => "⧉",
            BoundaryFill(_) => "▦",
            MeshImport(_) => "△",
            MoveBody(_) => "✥",
            SmWall(_) => "▭",
            SmFlange(_) => "⌐",
            SmHem(_) => "⊃",
            SmJoggle(_) => "⌇",
            SmBend(_) => "⌐",
            SmLouver(_) => "≋",
            SmStamp(_) => "◡",
            SmFlangedHole(_) => "◉",
            StructMember(_) => "⌶",
            StructPlate(_) => "▬",
            StructGusset(_) => "◺",
        }
    }

    /// Percorre os ids referenciados e as expressões do comando.
    pub fn walk(&mut self, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
        use FeatureKind::*;
        match self {
            Sketch(s) => {
                s.plane.walk(ids);
                for v in s.data.exprs_mut() {
                    exprs(v)
                }
                // geometria externa: referências e sketches de origem
                for l in &mut s.data.links {
                    match &mut l.source {
                        crate::sketch::LinkSrc::Ref(x) => ids(x),
                        crate::sketch::LinkSrc::SketchCurve { sketch, .. } => ids(sketch),
                        _ => {}
                    }
                }
            }
            Extrude(e) => {
                ids(&mut e.sketch);
                if let Some(a) = &mut e.dir {
                    a.walk(ids)
                }
                extent_walk(&mut e.extent, ids, exprs);
                exprs(&mut e.taper);
                if let Some(t) = &mut e.thin {
                    exprs(t)
                }
            }
            Revolve(r) => {
                ids(&mut r.sketch);
                r.axis.walk(ids);
                exprs(&mut r.angle);
                if let Some(a) = &mut r.angle2 {
                    exprs(a)
                }
                if let Some(t) = &mut r.thin {
                    exprs(t)
                }
            }
            Hole(h) => {
                ids(&mut h.sketch);
                exprs(&mut h.diameter);
                extent_walk(&mut h.depth, ids, exprs);
                match &mut h.kind {
                    HoleKind::Simples => {}
                    HoleKind::Rebaixado { diameter, depth } => {
                        exprs(diameter);
                        exprs(depth)
                    }
                    HoleKind::Escareado { diameter, angle } => {
                        exprs(diameter);
                        exprs(angle)
                    }
                    HoleKind::Contrafurado { diameter, depth, angle } => {
                        exprs(diameter);
                        exprs(depth);
                        exprs(angle)
                    }
                }
                for p in &mut h.points {
                    p.walk(ids)
                }
                if let Some(a) = &mut h.dir {
                    a.walk(ids)
                }
                if let Some(t) = &mut h.tip {
                    exprs(t)
                }
                exprs(&mut h.taper);
            }
            Fillet(o) | Chamfer(o) => {
                exprs(&mut o.value);
                if let EdgeSel::CreatedBy(x) = &mut o.edges {
                    ids(x)
                }
                match &mut o.mode {
                    EdgeMode::Constant | EdgeMode::FaceFace(..) | EdgeMode::FullRound { .. } => {}
                    EdgeMode::Variable(v) | EdgeMode::TwoDist(v) | EdgeMode::DistAngle(v) => exprs(v),
                }
            }
            Sweep(w) => {
                ids(&mut w.sketch);
                w.spine.walk(ids);
                if let Some(t) = &mut w.thin {
                    exprs(t)
                }
            }
            Loft(l) => {
                for x in &mut l.sections {
                    ids(x)
                }
            }
            Primitive(p) => {
                p.plane.walk(ids);
                for e in [&mut p.a, &mut p.b, &mut p.c, &mut p.d] {
                    exprs(e)
                }
            }
            Shell(sh) => exprs(&mut sh.thickness),
            Draft(d) => {
                d.neutral.walk(ids);
                exprs(&mut d.angle)
            }
            Thickness(t) => exprs(&mut t.value),
            Pattern(p) => {
                for x in &mut p.sources {
                    ids(x)
                }
                match &mut p.kind {
                    PatternKind::Linear { dir, count, spacing, dir2, count2, spacing2, .. } => {
                        dir.walk(ids);
                        if let Some(d) = dir2 {
                            d.walk(ids)
                        }
                        for e in [count, spacing, count2, spacing2] {
                            exprs(e)
                        }
                    }
                    PatternKind::Circular { axis, count, angle } => {
                        axis.walk(ids);
                        exprs(count);
                        exprs(angle)
                    }
                    PatternKind::Points { points, base } => {
                        base.walk(ids);
                        for q in points {
                            q.walk(ids)
                        }
                    }
                }
            }
            Mirror(m) => {
                m.plane.walk(ids);
                for x in &mut m.sources {
                    ids(x)
                }
            }
            Scale(sc) => {
                sc.center.walk(ids);
                exprs(&mut sc.factor)
            }
            Stiffener(st) => {
                ids(&mut st.sketch);
                exprs(&mut st.thickness);
                exprs(&mut st.depth)
            }
            Thread(t) => {
                exprs(&mut t.length);
                if let Some(e) = &mut t.end {
                    extent_walk(e, ids, exprs);
                }
            }
            Combine(c) => ids(&mut c.tool),
            SplitBody(s) => s.surface.walk(ids),
            ThickSurface(t) => {
                t.surface.walk(ids);
                exprs(&mut t.t1);
                exprs(&mut t.t2)
            }
            CloseSurface(c) => {
                c.surface.walk(ids);
                exprs(&mut c.tol)
            }
            SewSurface(s) => s.surface.walk(ids),
            BoundaryFill(b) => {
                for t in &mut b.tools {
                    t.walk(ids)
                }
            }
            MeshImport(_) => {}
            SmWall(w) => ids(&mut w.sketch),
            SmFlange(f) => {
                for e in [&mut f.length, &mut f.angle, &mut f.off1, &mut f.off2] {
                    exprs(e)
                }
                if let Some(r) = &mut f.radius {
                    exprs(r)
                }
                if let Some(r) = &mut f.relief {
                    exprs(r)
                }
            }
            SmJoggle(j) => {
                exprs(&mut j.depth);
                exprs(&mut j.run)
            }
            SmBend(b) => {
                exprs(&mut b.angle);
                if let Some(r) = &mut b.radius {
                    exprs(r)
                }
            }
            SmLouver(l) => {
                exprs(&mut l.width);
                exprs(&mut l.height)
            }
            SmStamp(s) => {
                ids(&mut s.sketch);
                exprs(&mut s.depth)
            }
            SmFlangedHole(h) => {
                ids(&mut h.sketch);
                exprs(&mut h.diameter);
                exprs(&mut h.height)
            }
            StructMember(m) => {
                for p in &mut m.paths {
                    p.walk(ids)
                }
                for e in m.dims.iter_mut().chain([&mut m.angle, &mut m.off_x, &mut m.off_y, &mut m.ext1, &mut m.ext2, &mut m.gap]) {
                    exprs(e)
                }
            }
            StructPlate(p) => {
                exprs(&mut p.thickness);
                exprs(&mut p.margin)
            }
            StructGusset(g) => {
                exprs(&mut g.leg);
                exprs(&mut g.thickness);
                exprs(&mut g.chamfer)
            }
            SmHem(h) => {
                exprs(&mut h.length);
                exprs(&mut h.gap)
            }
            MoveBody(m) => match &mut m.xf {
                BodyXf::Translate { dir, dist } => {
                    dir.walk(ids);
                    exprs(dist)
                }
                BodyXf::Rotate { axis, angle } => {
                    axis.walk(ids);
                    exprs(angle)
                }
                BodyXf::Symmetry { plane } => plane.walk(ids),
                BodyXf::Affinity { plane, fx, fy, fz } => {
                    plane.walk(ids);
                    exprs(fx);
                    exprs(fy);
                    exprs(fz)
                }
            },
        }
    }
}

fn extent_walk(e: &mut Extent, ids: &mut dyn FnMut(&mut Id), exprs: &mut dyn FnMut(&mut String)) {
    match e {
        Extent::Blind(d) | Extent::Symmetric(d) => exprs(d),
        Extent::TwoSides(a, b) => {
            exprs(a);
            exprs(b)
        }
        Extent::ThroughAll | Extent::UpToNext | Extent::UpToLast => {}
        Extent::UpTo(p) => p.walk(ids),
        Extent::UpToSurface(s) => s.walk(ids),
    }
}

/// Como o perfil é orientado ao longo da curva guia.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum SweepMode {
    /// Frenet corrigido: acompanha a curva sem torcer (padrão)
    #[default]
    Corrigido,
    /// orientação fixa (o perfil não gira)
    Fixo,
    /// Frenet puro (segue a curvatura, pode torcer)
    Frenet,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Sweep {
    pub sketch: Id,
    pub spine: CurveRef,
    pub mode: SweepMode,
    pub op: BoolOp,
    #[serde(default)]
    pub thin: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Loft {
    pub sections: Vec<Id>,
    pub ruled: bool,
    pub closed: bool,
    pub op: BoolOp,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PrimKind {
    Caixa,
    Cilindro,
    Esfera,
    Cone,
    Toro,
    Cunha,
    Prisma,
}

impl PrimKind {
    pub const ALL: [PrimKind; 7] = [PrimKind::Caixa, PrimKind::Cilindro, PrimKind::Esfera, PrimKind::Cone, PrimKind::Toro, PrimKind::Cunha, PrimKind::Prisma];
    pub fn label(self) -> &'static str {
        match self {
            PrimKind::Caixa => "Caixa",
            PrimKind::Cilindro => "Cilindro",
            PrimKind::Esfera => "Esfera",
            PrimKind::Cone => "Cone",
            PrimKind::Toro => "Toro",
            PrimKind::Cunha => "Cunha",
            PrimKind::Prisma => "Prisma",
        }
    }
    /// Nomes das medidas a, b, c, d ("" = não usada).
    pub fn params(self) -> [&'static str; 4] {
        match self {
            PrimKind::Caixa => ["Largura", "Profundidade", "Altura", ""],
            PrimKind::Cilindro => ["Raio", "Altura", "", ""],
            PrimKind::Esfera => ["Raio", "", "", ""],
            PrimKind::Cone => ["Raio da base", "Raio do topo", "Altura", ""],
            PrimKind::Toro => ["Raio maior", "Raio menor", "", ""],
            PrimKind::Cunha => ["X", "Y", "Z", "X do topo"],
            PrimKind::Prisma => ["Lados", "Raio", "Altura", ""],
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Primitive {
    pub kind: PrimKind,
    pub plane: PlaneRef,
    pub a: String,
    pub b: String,
    pub c: String,
    pub d: String,
    pub op: BoolOp,
}

/// Casca: esvazia o sólido abrindo as faces escolhidas (pontos clicados).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Shell {
    pub faces: Vec<[f64; 3]>,
    pub thickness: String,
    pub outward: bool,
}

/// Ângulo de saída nas faces escolhidas, a partir de um plano neutro (puxa pela normal dele).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub faces: Vec<[f64; 3]>,
    pub neutral: PlaneRef,
    pub angle: String,
    pub reverse: bool,
}

/// Espessura: acrescenta (positivo) ou tira (negativo) material das faces escolhidas.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Thickness {
    pub faces: Vec<[f64; 3]>,
    pub value: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PatternKind {
    /// Uma ou duas direções: quantidade e espaçamento.
    Linear {
        dir: AxisRef,
        count: String,
        spacing: String,
        dir2: Option<AxisRef>,
        count2: String,
        spacing2: String,
        /// o espaçamento é o comprimento total (instâncias e comprimento total)
        #[serde(default)]
        total: bool,
    },
    /// Em volta de um eixo: quantidade e ângulo total (360 = volta completa, distribuída por igual).
    Circular { axis: AxisRef, count: String, angle: String },
    /// Cópias nos pontos escolhidos (deslocadas do ponto base).
    Points { points: Vec<PointRef>, base: PointRef },
}

/// Padrão: repete os comandos escolhidos (ou o corpo inteiro, se a lista estiver vazia).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub kind: PatternKind,
    pub sources: Vec<Id>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Mirror {
    pub plane: PlaneRef,
    pub sources: Vec<Id>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Scale {
    pub center: PointRef,
    pub factor: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stiffener {
    pub sketch: Id,
    pub thickness: String,
    pub depth: String,
    /// para o outro lado do plano do sketch
    pub reverse: bool,
    /// vai até encontrar o corpo (ignora a profundidade)
    #[serde(default)]
    pub to_body: bool,
}

/// Booleana entre coletores de corpos.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Combine {
    /// coletor cujo corpo é a ferramenta
    pub tool: Id,
    pub op: BoolOp,
    /// mantém o corpo ferramenta visível depois da operação
    pub keep_tool: bool,
}

/// Rosca numa face cilíndrica: externa num eixo, interna num furo.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Thread {
    /// ponto clicado na face cilíndrica
    pub face: [f64; 3],
    /// designação da tabela de roscas (ex.: "M8x1.25")
    pub spec: String,
    pub length: String,
    /// sulco helicoidal real (booleana) em vez de cosmética
    pub modeled: bool,
    /// término: `None` = comprimento; passante = face inteira; até plano/superfície
    #[serde(default)]
    pub end: Option<Extent>,
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
    /// Até a próxima parede do corpo na direção da extrusão.
    UpToNext,
    /// Até a última parede do corpo (o ponto mais distante dele nessa direção).
    UpToLast,
    /// Até uma superfície ou face (estendida para cortar a ferramenta inteira).
    UpToSurface(SurfRef),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Extrude {
    pub sketch: Id,
    pub op: BoolOp,
    pub extent: Extent,
    pub reverse: bool,
    /// ângulo de saída (graus; 0 = reto)
    #[serde(default = "zero")]
    pub taper: String,
    /// parede fina com esta espessura
    #[serde(default)]
    pub thin: Option<String>,
    /// direção da extrusão (linha, aresta, eixo); `None` = normal ao plano do sketch
    #[serde(default)]
    pub dir: Option<AxisRef>,
}

fn zero() -> String {
    "0".into()
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Revolve {
    pub sketch: Id,
    pub axis: AxisRef,
    pub angle: String,
    pub symmetric: bool,
    pub op: BoolOp,
    #[serde(default)]
    pub thin: Option<String>,
    /// segundo ângulo (para o outro lado do plano do sketch)
    #[serde(default)]
    pub angle2: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum HoleKind {
    Simples,
    Rebaixado { diameter: String, depth: String },
    Escareado { diameter: String, angle: String },
    /// rebaixo com fundo cônico (counterdrilled)
    Contrafurado { diameter: String, depth: String, angle: String },
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
    /// Centros por pontos 3D (vários furos de uma vez); vazio = pontos do sketch.
    #[serde(default)]
    pub points: Vec<PointRef>,
    /// Direção do furo (linha/aresta/eixo); sem direção: normal à superfície no ponto
    /// (pontos 3D) ou normal ao plano do sketch.
    #[serde(default)]
    pub dir: Option<AxisRef>,
    /// Inverter o sentido do furo.
    #[serde(default)]
    pub reverse: bool,
    /// Fundo em V: ângulo da ponta da broca (ex.: 118); `None` = fundo plano.
    #[serde(default)]
    pub tip: Option<String>,
    /// Furo cônico: meio-ângulo de conicidade (graus); "0" = cilíndrico.
    #[serde(default = "zero")]
    pub taper: String,
    /// Rosca à esquerda.
    #[serde(default)]
    pub left_hand: bool,
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
    #[serde(default)]
    pub mode: EdgeMode,
}

/// Variações de filete/chanfro.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum EdgeMode {
    /// raio ou distância constante
    #[default]
    Constant,
    /// filete variável: raio no início = valor, no fim = este
    Variable(String),
    /// chanfro com duas distâncias (a segunda)
    TwoDist(String),
    /// chanfro por distância e ângulo (graus)
    DistAngle(String),
    /// filete entre duas faces (pontos clicados): arredonda a aresta comum
    FaceFace([f64; 3], [f64; 3]),
    /// filete tritangente/completo: consome a face do meio entre duas faces laterais
    FullRound { mid: [f64; 3], a: [f64; 3], b: [f64; 3] },
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
    /// imagens de textura usadas nas aparências (até 8)
    #[serde(default)]
    pub textures: Vec<TextureRef>,
    /// parâmetros de chapa metálica (espessura, raio, fator K, estado planificado)
    #[serde(default)]
    pub sheet: Option<SheetParams>,
    /// anotações 3D (PMI) desta peça
    #[serde(default)]
    pub pmi: Vec<crate::pmi::Pmi>,
    /// capturas de vista do PMI (câmera + anotações visíveis)
    #[serde(default)]
    pub pmi_captures: Vec<crate::pmi::PmiCapture>,
    /// estilo próprio de linha/ponto de elementos de geometria (espessura, tipo de linha, símbolo)
    #[serde(default)]
    pub line_styles: Vec<(Id, LineStyle)>,
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
            textures: vec![],
            sheet: None,
            pmi: vec![],
            pmi_captures: vec![],
            line_styles: vec![],
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

    pub fn name_taken(&self, name: &str) -> bool {
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
        self.collectors.push(BodyCollector { id, name: name.into(), color, visible: true, finish: Finish::default(), texture: None, marker: None, published: false });
        id
    }

    /// Pasta de geometria livre (sem linha do tempo).
    pub fn add_geometry_folder(&mut self, name: &str) -> Id {
        let id = self.alloc();
        self.folders.push(Folder { id, name: name.into(), kind: FolderKind::Geometria, parent: None });
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
        self.folders.push(Folder { id, name: name.into(), kind: FolderKind::Comandos, parent: None });
        id
    }

    /// Remove a pasta; os comandos dentro dela voltam para a raiz (não são apagados).
    pub fn remove_folder(&mut self, id: Id) {
        let Some(fo) = self.folder(id).cloned() else { return };
        if fo.kind == FolderKind::Geometria {
            // o conteúdo sobe para a pasta-mãe (ou outra pasta de geometria): sketches livres continuam livres
            let to = fo.parent.or_else(|| self.folders.iter().find(|f| f.kind == FolderKind::Geometria && f.id != id).map(|f| f.id));
            self.folders.retain(|f| f.id != id);
            let to = to.unwrap_or_else(|| self.add_geometry_folder("Geometria"));
            for f in &mut self.features {
                if f.folder == Some(id) {
                    f.folder = Some(to);
                }
            }
            for r in &mut self.references {
                if r.folder == Some(id) {
                    r.folder = Some(to);
                }
            }
            for f in &mut self.folders {
                if f.parent == Some(id) {
                    f.parent = Some(to);
                }
            }
            return;
        }
        self.folders.retain(|f| f.id != id);
        for f in &mut self.features {
            if f.folder == Some(id) {
                f.folder = None;
            }
        }
    }

    /// Subpasta de geometria.
    pub fn add_subfolder(&mut self, name: &str, parent: Id) -> Id {
        let id = self.add_geometry_folder(name);
        if let Some(f) = self.folders.iter_mut().find(|f| f.id == id) {
            f.parent = Some(parent);
        }
        id
    }

    /// `a` está dentro de `b` (ou é `b`)?
    pub fn folder_within(&self, a: Id, b: Id) -> bool {
        let mut cur = Some(a);
        let mut guard = 0;
        while let Some(c) = cur {
            if c == b {
                return true;
            }
            cur = self.folder(c).and_then(|f| f.parent);
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        false
    }

    /// Move um item da árvore de geometria (elemento, sketch livre ou pasta) para a pasta `to`.
    pub fn move_to_folder(&mut self, item: Id, to: Id) -> Result<(), DocError> {
        if self.folder(to).is_none_or(|f| f.kind != FolderKind::Geometria) {
            return Err(DocError::Incompatible);
        }
        if let Some(r) = self.references.iter_mut().find(|r| r.id == item) {
            r.folder = Some(to);
            return Ok(());
        }
        if self.folder(item).is_some() {
            if self.folder_within(to, item) {
                return Err(DocError::Incompatible);
            }
            if let Some(f) = self.folders.iter_mut().find(|f| f.id == item) {
                f.parent = Some(to);
            }
            return Ok(());
        }
        let free = self.feature(item).is_some_and(|f| matches!(f.kind, FeatureKind::Sketch(_)));
        match self.features.iter_mut().find(|f| f.id == item) {
            Some(f) if free => {
                f.folder = Some(to);
                Ok(())
            }
            _ => Err(DocError::Incompatible),
        }
    }

    /// Nome livre derivado de `name` (tira o número do fim e numera de novo).
    fn copy_name(&self, name: &str) -> String {
        let base = name.trim_end_matches(|c: char| c.is_ascii_digit());
        self.unique_name(if base.is_empty() { "Copia" } else { base })
    }

    /// Duplica um item da geometria na pasta `to`; pastas são copiadas com o conteúdo.
    /// As entradas da cópia continuam apontando para os mesmos elementos de origem.
    pub fn duplicate_into(&mut self, item: Id, to: Id) -> Option<Id> {
        if let Some(r) = self.reference(item).cloned() {
            let id = self.alloc();
            let name = self.copy_name(&r.name);
            self.references.push(RefElement { id, name, folder: Some(to), ..r });
            return Some(id);
        }
        if let Some(fo) = self.folder(item).cloned() {
            if self.folder_within(to, item) {
                return None;
            }
            let nf = self.add_subfolder(&format!("{} (cópia)", fo.name), to);
            let refs: Vec<Id> = self.references.iter().filter(|r| r.folder == Some(item)).map(|r| r.id).collect();
            let sks: Vec<Id> = self.features.iter().filter(|f| f.folder == Some(item) && self.is_free(f)).map(|f| f.id).collect();
            let subs: Vec<Id> = self.folders.iter().filter(|f| f.parent == Some(item)).map(|f| f.id).collect();
            for x in refs.into_iter().chain(sks).chain(subs) {
                self.duplicate_into(x, nf);
            }
            return Some(nf);
        }
        let f = self.feature(item).cloned().filter(|f| matches!(f.kind, FeatureKind::Sketch(_)))?;
        let id = self.alloc();
        let name = self.copy_name(&f.name);
        self.features.push(Feature { id, name, folder: Some(to), ..f });
        if self.rollback == self.features.len() - 1 {
            self.rollback += 1;
        }
        Some(id)
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
        self.references.push(RefElement { id, name, kind: RefKind::OffsetPlane { base, offset: offset.into() }, visible: true, folder: None, published: false, appearance: None });
        self.file_last_reference();
        id
    }

    /// Plano de uma face plana (criado ao clicar numa face para fazer um sketch).
    pub fn add_face_plane(&mut self, origin: [f64; 3], x: [f64; 3], normal: [f64; 3], owner: Option<Id>) -> Id {
        let id = self.alloc();
        let name = self.unique_name("PlanoFace");
        self.references.push(RefElement { id, name, kind: RefKind::FacePlane { origin, x, normal, owner }, visible: true, folder: None, published: false, appearance: None });
        self.file_last_reference();
        id
    }

    /// Plano que passa por um eixo, girado `angle` graus a partir de `base`.
    pub fn add_angle_plane(&mut self, base: PlaneRef, axis: AxisRef, angle: &str) -> Id {
        let id = self.alloc();
        let name = self.unique_name("Plano");
        self.references.push(RefElement { id, name, kind: RefKind::AnglePlane { base, axis, angle: angle.into() }, visible: true, folder: None, published: false, appearance: None });
        self.file_last_reference();
        id
    }

    /// Cria um elemento de geometria de referência (plano, ponto ou curva) na pasta de geometria padrão.
    pub fn add_geometry(&mut self, kind: RefKind) -> Id {
        let id = self.alloc();
        let base = match kind.class() {
            RefClass::Surface => "Superficie",
            RefClass::Plane => "Plano",
            RefClass::Point => "Ponto",
            RefClass::Curve => match kind {
                RefKind::LinePoints { .. }
                | RefKind::LineDir { .. }
                | RefKind::LineAngle { .. }
                | RefKind::LineNormalSurface { .. }
                | RefKind::LinePlanes { .. }
                | RefKind::LineBisector { .. }
                | RefKind::LineAxis { .. } => "Linha",
                _ => "Curva",
            },
        };
        let name = self.unique_name(base);
        self.references.push(RefElement { id, name, kind, visible: true, folder: None, published: false, appearance: None });
        self.file_last_reference();
        id
    }

    /// Importa uma publicação de outra peça: cria o link e o elemento (ou parâmetro) que recebe a cópia.
    pub fn add_link(&mut self, source: &str, path: &str, publication: &str, geo: LinkedGeo) -> Id {
        let clean = |t: &str| t.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect::<String>();
        let base = format!("{}_{}", clean(source), clean(publication));
        let ext = self.alloc();
        let target = match &geo {
            LinkedGeo::Value(v) => {
                let name = self.unique_name(&base);
                let id = self.alloc();
                self.params.push(Parameter { id, name, expr: format!("{v}"), unit: Unit::Comprimento, comment: format!("link: {source}.{publication}"), published: false });
                id
            }
            g => {
                let class = g.class();
                let id = self.alloc();
                let name = if self.name_taken(&base) { self.unique_name(&base) } else { base.clone() };
                self.references.push(RefElement { id, name, kind: RefKind::Linked { ext, class }, visible: true, folder: None, published: false, appearance: None });
                id
            }
        };
        self.external.push(ExternalRef {
            id: ext,
            name: base,
            source: source.into(),
            publication: publication.into(),
            state: LinkState::Ativo,
            path: path.into(),
            snapshot: Some(geo),
            target: Some(target),
        });
        target
    }

    /// Atualiza a cópia de um link (None = a publicação sumiu: link quebrado, a cópia antiga fica).
    pub fn refresh_link(&mut self, ext: Id, geo: Option<LinkedGeo>) {
        let Some(e) = self.external.iter_mut().find(|e| e.id == ext) else { return };
        if matches!(e.state, LinkState::Congelado | LinkState::Desativado) {
            return;
        }
        let Some(g) = geo else {
            e.state = LinkState::Quebrado;
            return;
        };
        e.state = LinkState::Ativo;
        let target = e.target;
        if let (LinkedGeo::Value(v), Some(t)) = (&g, target) {
            if let Some(p) = self.params.iter_mut().find(|p| p.id == t) {
                p.expr = format!("{v}");
            }
        }
        if let Some(e) = self.external.iter_mut().find(|e| e.id == ext) {
            e.snapshot = Some(g);
        }
    }

    /// Isola o link: a cópia atual vira geometria fixa desta peça e o link deixa de existir.
    pub fn isolate_link(&mut self, ext: Id) {
        let Some(e) = self.external.iter().find(|e| e.id == ext).cloned() else { return };
        if let (Some(t), Some(g)) = (e.target, e.snapshot.clone()) {
            if let Some(r) = self.references.iter_mut().find(|r| r.id == t) {
                r.kind = RefKind::Isolated { geo: g };
            }
        }
        self.external.retain(|x| x.id != ext);
    }

    /// Religa o link a outra peça/publicação (assistente de relink).
    pub fn relink(&mut self, ext: Id, source: &str, path: &str, publication: &str, geo: LinkedGeo) {
        if let Some(e) = self.external.iter_mut().find(|e| e.id == ext) {
            e.source = source.into();
            e.path = path.into();
            e.publication = publication.into();
            e.state = LinkState::Ativo;
        }
        self.refresh_link(ext, Some(geo));
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
        self.params.push(Parameter { id, name: name.into(), expr: expr.into(), unit, comment: String::new(), published: false });
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
            feat.kind.walk(&mut |_| {}, &mut |e| f(e));
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
        let mut v = vec![];
        kind.clone().walk(&mut |x| {
            if !v.contains(x) {
                v.push(*x)
            }
        }, &mut |_| {});
        v
    }

    /// Entradas que a operação "consome": ficam ocultas enquanto ela existe (superfícies juntadas,
    /// divididas, engrossadas…; sketches usados pelos comandos de forma) e voltam a aparecer
    /// quando a operação é apagada.
    pub fn consumed(&self, id: Id) -> Vec<Id> {
        let sref = |s: &SurfRef| match s {
            SurfRef::Ref(x) => Some(*x),
            _ => None,
        };
        if let Some(r) = self.reference(id) {
            use RefKind::*;
            return match &r.kind {
                SurfJoin { items, .. } => items.iter().filter_map(sref).collect(),
                SurfHeal { surface, .. } | SurfSplit { surface, .. } | SurfUntrim { surface } | SurfExtrapolate { surface, .. } => sref(surface).into_iter().collect(),
                SurfTrim { a, b, .. } => [sref(a), sref(b)].into_iter().flatten().collect(),
                GeoTransform { source, .. } => vec![*source],
                _ => vec![],
            };
        }
        let Some(f) = self.feature(id) else { return vec![] };
        use FeatureKind::*;
        match &f.kind {
            Extrude(e) => vec![e.sketch],
            SmWall(w) => vec![w.sketch],
            SmStamp(s) => vec![s.sketch],
            SmFlangedHole(h) => vec![h.sketch],
            Revolve(r) => vec![r.sketch],
            Sweep(w) => vec![w.sketch],
            Loft(l) => l.sections.clone(),
            Stiffener(s) => vec![s.sketch],
            Hole(h) => vec![h.sketch],
            SplitBody(s) => sref(&s.surface).into_iter().collect(),
            ThickSurface(t) => sref(&t.surface).into_iter().collect(),
            CloseSurface(c) => sref(&c.surface).into_iter().collect(),
            SewSurface(s) => sref(&s.surface).into_iter().collect(),
            BoundaryFill(b) => b.tools.iter().filter_map(sref).collect(),
            _ => vec![],
        }
    }

    fn set_shown(&mut self, id: Id, shown: bool) {
        if let Some(r) = self.references.iter_mut().find(|r| r.id == id) {
            r.visible = shown;
        } else if let Some(f) = self.features.iter_mut().find(|f| f.id == id) {
            if matches!(f.kind, FeatureKind::Sketch(_)) {
                f.hidden = !shown;
            }
        }
    }

    /// Oculta as entradas consumidas pela operação (chamado ao criá-la ou concluí-la).
    pub fn hide_consumed(&mut self, id: Id) {
        for x in self.consumed(id) {
            self.set_shown(x, false);
        }
    }

    /// Renomeia comando, elemento, pasta ou coletor (nomes de comandos e elementos são únicos).
    pub fn rename_item(&mut self, id: Id, name: &str) -> Result<(), DocError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DocError::InvalidName(name.into()));
        }
        if let Some(f) = self.folders.iter_mut().find(|f| f.id == id) {
            f.name = name.into();
            return Ok(());
        }
        if let Some(c) = self.collectors.iter_mut().find(|c| c.id == id) {
            c.name = name.into();
            return Ok(());
        }
        if self.name_of(id) == name {
            return Ok(());
        }
        if self.name_taken(name) {
            return Err(DocError::DuplicateName(name.into()));
        }
        if let Some(f) = self.features.iter_mut().find(|f| f.id == id) {
            f.name = name.into();
        } else if let Some(r) = self.references.iter_mut().find(|r| r.id == id) {
            r.name = name.into();
        } else {
            return Err(DocError::NotFound);
        }
        Ok(())
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
                // comandos de um coletor dependem dele (apagar o corpo leva os comandos)
                if (Self::refs_of(&f.kind).contains(&cur) || f.target == cur) && !out.contains(&f.id) {
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
        for r in &mut self.references {
            r.kind.walk(&mut |x| swap(x, &mut n), &mut |_| {});
        }
        for f in &mut self.features {
            f.kind.walk(&mut |x| swap(x, &mut n), &mut |_| {});
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
        // entradas que as operações apagadas consumiam voltam a aparecer (se ninguém mais as consome)
        let freed: Vec<Id> = kill.iter().flat_map(|k| self.consumed(*k)).filter(|x| !kill.contains(x)).collect();
        self.features.retain(|f| !kill.contains(&f.id));
        self.references.retain(|r| !kill.contains(&r.id));
        let ids: Vec<Id> = self.features.iter().map(|f| f.id).chain(self.references.iter().map(|r| r.id)).collect();
        let still: HashSet<Id> = ids.iter().flat_map(|i| self.consumed(*i)).collect();
        for x in freed {
            if !still.contains(&x) {
                self.set_shown(x, true);
            }
        }
        // coletores de corpos: sempre fica pelo menos um; comandos órfãos vão para o primeiro
        if self.collectors.iter().any(|c| kill.contains(&c.id)) {
            self.collectors.retain(|c| !kill.contains(&c.id));
            if self.collectors.is_empty() {
                let n = self.unique_name("Corpo");
                self.add_collector(&n);
            }
            let first = self.collectors[0].id;
            if kill.contains(&self.active_collector) {
                self.active_collector = first;
            }
            for f in &mut self.features {
                if kill.contains(&f.target) {
                    f.target = first;
                }
            }
        }
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

    #[test]
    fn pastas_de_geometria_subpastas_mover_copiar() {
        let mut d = Document::new("t");
        let a = d.add_geometry_folder("A");
        let b = d.add_subfolder("B", a);
        let p = d.add_geometry(RefKind::PointCoords { x: "1".into(), y: "0".into(), z: "0".into(), frame: None });
        d.move_to_folder(p, b).unwrap();
        assert_eq!(d.reference(p).unwrap().folder, Some(b));
        // pasta não pode ir para dentro dela mesma
        assert!(d.move_to_folder(a, b).is_err());
        let sk = d.add_free_sketch(PlaneRef::default(), b);
        let c = d.duplicate_into(b, a).unwrap();
        assert_eq!(d.folder(c).unwrap().parent, Some(a));
        assert_eq!(d.references.iter().filter(|r| r.folder == Some(c)).count(), 1);
        assert_eq!(d.features.iter().filter(|f| f.folder == Some(c) && d.is_free(f)).count(), 1);
        // apagar a subpasta sobe o conteúdo e o sketch continua livre
        d.remove_folder(b);
        assert_eq!(d.reference(p).unwrap().folder, Some(a));
        assert!(d.is_free(d.feature(sk).unwrap()));
    }

    #[test]
    fn apagar_corpo_leva_os_comandos_e_mantem_um_coletor() {
        let mut d = Document::new("t");
        let c0 = d.active_collector;
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::default(), data: Default::default() }));
        assert!(d.dependents(c0).contains(&sk));
        d.delete(c0, true);
        assert!(d.feature(sk).is_none());
        assert_eq!(d.collectors.len(), 1);
        assert_ne!(d.active_collector, c0);
        assert_eq!(d.active_collector, d.collectors[0].id);
    }

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
            reverse: false, taper: s("0"), thin: None, dir: None
        }));
        let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("1"), edges: EdgeSel::CreatedBy(ex), mode: EdgeMode::Constant }));
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
            reverse: false, taper: s("0"), thin: None, dir: None
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
