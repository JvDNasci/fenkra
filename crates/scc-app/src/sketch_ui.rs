//! Editor de sketch interativo na viewport (alpha.3).
//!
//! Desenho livre com o mouse sobre o plano do sketch (a vista é alinhada ao entrar), snaps,
//! restrições e cotas automáticas, ferramenta de cotagem com expressões, aplicação de restrições,
//! arraste resolvido pelo solver e cores de estado (livre / totalmente restrito / conflito).

use crate::app::{fmt, App, Sel};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use glam::Vec3;
use scc_core::sketch::{Curve, Kind, SolveReport};
use scc_core::{FeatureKind, Id, PlaneRef, SketchData};
use scc_kernel::Frame;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Line,
    Rect,
    RectCenter,
    Circle,
    Circle3,
    Arc3,
    ArcCenter,
    Slot,
    Polygon,
    Point,
    Dimension,
    DimH,
    DimV,
    DimAngle,
    DimRadius,
    DimDiameter,
    Move,
    Copy,
    Mirror,
    Offset,
    Fillet,
    Constr(CKind),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CKind {
    Coincident,
    Horizontal,
    Vertical,
    Parallel,
    Perpendicular,
    Tangent,
    Equal,
    Concentric,
    Midpoint,
    Fixed,
    Symmetric,
}

impl CKind {
    pub fn needs(self) -> usize {
        match self {
            CKind::Horizontal | CKind::Vertical | CKind::Fixed => 1,
            CKind::Symmetric => 3,
            _ => 2,
        }
    }
}

impl Tool {
    /// (ícone, nome curto, descrição breve, descrição avançada)
    pub fn info(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Tool::Select => ("sk_selecionar", "Selecionar", "Selecionar e arrastar", "Clique para selecionar (Shift soma). Arraste pontos, linhas ou a borda de círculos: o solver mantém as restrições. Delete apaga a seleção."),
            Tool::Line => ("sk_polilinha", "Linha", "Linha / polilinha  (L)", "Cada clique cria um segmento ligado ao anterior. Clique no primeiro ponto para fechar o perfil; Esc ou botão direito termina. Linhas quase horizontais/verticais recebem a restrição automaticamente."),
            Tool::Rect => ("sk_retangulo", "Retângulo", "Retângulo por 2 cantos  (R)", "Clique em dois cantos opostos. Cria 4 linhas com restrições horizontal/vertical; com cotas automáticas, recebe largura e altura."),
            Tool::RectCenter => ("sk_retangulo_centro", "Ret. centro", "Retângulo pelo centro", "Clique no centro e depois num canto. O centro fica preso à diagonal (ponto médio), ótimo para peças simétricas na origem."),
            Tool::Circle => ("sk_circulo", "Círculo", "Círculo por centro e raio  (C)", "Clique no centro e depois em um ponto da borda. Com cotas automáticas recebe o diâmetro."),
            Tool::Circle3 => ("sk_circulo3", "Círculo 3P", "Círculo por 3 pontos", "Clique em três pontos da circunferência."),
            Tool::Arc3 => ("sk_arco3", "Arco 3P", "Arco por 3 pontos  (A)", "Clique no início, no fim e em um ponto intermediário do arco."),
            Tool::ArcCenter => ("sk_arco_centro", "Arco centro", "Arco por centro", "Clique no centro, no início e no fim (sentido anti-horário)."),
            Tool::Slot => ("sk_rasgo", "Rasgo", "Rasgo centro a centro", "Clique nos dois centros e depois na largura. Cria dois arcos tangentes às linhas, com raios iguais."),
            Tool::Polygon => ("sk_poligono", "Polígono", "Polígono regular", "Clique no centro e depois num vértice. Lados iguais inscritos num círculo de construção; o número de lados fica no painel."),
            Tool::Point => ("sk_ponto", "Ponto", "Ponto  (P)", "Ponto isolado: serve de centro de furo ou referência para cotas."),
            Tool::DimH => ("cota_horizontal", "Horizontal", "Cota horizontal", "Dois pontos ou uma linha: distância na horizontal do sketch."),
            Tool::DimV => ("cota_vertical", "Vertical", "Cota vertical", "Dois pontos ou uma linha: distância na vertical do sketch."),
            Tool::DimAngle => ("cota_angulo", "Ângulo", "Cota de ângulo", "Clique em duas linhas."),
            Tool::DimRadius => ("cota_raio", "Raio", "Cota de raio", "Clique num círculo ou arco."),
            Tool::DimDiameter => ("cota_diametro", "Diâmetro", "Cota de diâmetro", "Clique num círculo ou arco."),
            Tool::Move => ("sk_mover", "Mover", "Mover seleção", "Selecione curvas (ou clique nelas com a ferramenta), depois clique no ponto base e no destino. As restrições continuam valendo."),
            Tool::Copy => ("sk_copiar", "Copiar", "Copiar seleção", "Selecione curvas, clique no ponto base e no destino: cria uma cópia."),
            Tool::Mirror => ("sk_espelhar", "Espelhar", "Espelhar seleção", "Selecione curvas e clique na linha de simetria: cria a cópia espelhada com restrições de simetria."),
            Tool::Offset => ("sk_offset", "Offset", "Offset (curva paralela)", "Clique numa linha, círculo ou arco e depois onde a nova curva deve passar. A distância vira cota."),
            Tool::Fillet => ("sk_filete", "Filete", "Filete de canto", "Clique no canto entre duas linhas: insere um arco tangente e abre a cota do raio."),
            Tool::Dimension => ("sk_cota", "Cota", "Cotagem inteligente  (D)", "Linha → comprimento; círculo → diâmetro; arco → raio; dois pontos → distância (Ctrl: horizontal, Alt: vertical); duas linhas → ângulo; ponto e linha → distância. O valor aceita expressões e parâmetros (ex.: L/2)."),
            Tool::Constr(k) => match k {
                CKind::Coincident => ("r_coincidente", "Coincidente", "Coincidente", "Dois pontos viram um só; ponto + curva prende o ponto sobre a curva."),
                CKind::Horizontal => ("r_horizontal", "Horizontal", "Horizontal", "Linha horizontal, ou dois pontos na mesma altura."),
                CKind::Vertical => ("r_vertical", "Vertical", "Vertical", "Linha vertical, ou dois pontos alinhados na vertical."),
                CKind::Parallel => ("r_paralela", "Paralela", "Paralela", "Duas linhas paralelas."),
                CKind::Perpendicular => ("r_perpendicular", "Perpend.", "Perpendicular", "Duas linhas a 90°."),
                CKind::Tangent => ("r_tangente", "Tangente", "Tangente", "Linha e círculo/arco, ou dois círculos tangentes."),
                CKind::Equal => ("r_igual", "Igual", "Igual", "Duas linhas de mesmo comprimento, ou dois círculos/arcos de mesmo raio."),
                CKind::Concentric => ("r_concentrica", "Concêntrica", "Concêntrica", "Círculos/arcos com o mesmo centro."),
                CKind::Midpoint => ("r_ponto_medio", "Ponto médio", "Ponto médio", "Um ponto no meio de uma linha."),
                CKind::Fixed => ("r_fixa", "Fixa", "Fixar ponto", "O ponto não se move mais."),
                CKind::Symmetric => ("r_simetrica", "Simétrica", "Simétrica", "Dois pontos simétricos em relação a uma linha."),
            },
        }
    }
    fn clicks_needed(self) -> usize {
        match self {
            Tool::Rect | Tool::RectCenter | Tool::Circle => 2,
            Tool::Circle3 | Tool::Arc3 | Tool::ArcCenter | Tool::Slot => 3,
            Tool::Polygon => 2,
            Tool::Point => 1,
            Tool::Move | Tool::Copy => 2,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Ent {
    Pt(Id),
    Cv(Id),
    Cs(Id),
}

#[derive(Clone, Copy, Debug)]
struct Snap {
    uv: [f64; 2],
    point: Option<Id>,
    origin: bool,
}

struct Drag {
    pts: Vec<(Id, [f64; 2])>,
    start: [f64; 2],
    circle: Option<Id>,
}

pub struct SketchEdit {
    pub feature: Id,
    pub tool: Tool,
    clicks: Vec<Snap>,
    chain_last: Option<Id>,
    chain_first: Option<Id>,
    pub sel: Vec<Ent>,
    drag: Option<Drag>,
    pub dim_edit: Option<(Id, String, bool)>,
    pub report: SolveReport,
    pub construction: bool,
    pub polygon_sides: usize,
    pub prev_ortho: bool,
    hits: Vec<(Rect, Ent)>,
    /// mostra a checagem de contornos
    pub check: bool,
}

impl SketchEdit {
    pub fn new(feature: Id, prev_ortho: bool) -> Self {
        SketchEdit {
            feature,
            tool: Tool::Select,
            clicks: vec![],
            chain_last: None,
            chain_first: None,
            sel: vec![],
            drag: None,
            dim_edit: None,
            report: SolveReport::default(),
            construction: false,
            polygon_sides: 6,
            prev_ortho,
            hits: vec![],
            check: false,
        }
    }
    pub fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        self.clicks.clear();
        self.chain_last = None;
        self.chain_first = None;
        if !matches!(t, Tool::Constr(_) | Tool::Select | Tool::Move | Tool::Copy | Tool::Mirror) {
            self.sel.clear();
        }
    }
    /// Cancela a operação em andamento; devolve false se não havia nada a cancelar.
    pub fn cancel(&mut self) -> bool {
        if !self.clicks.is_empty() || self.chain_last.is_some() || self.dim_edit.is_some() {
            self.clicks.clear();
            self.chain_last = None;
            self.chain_first = None;
            self.dim_edit = None;
            true
        } else if self.tool != Tool::Select {
            self.tool = Tool::Select;
            true
        } else if !self.sel.is_empty() {
            self.sel.clear();
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------- acesso ao sketch e ao plano

pub fn data(app: &App) -> Option<&SketchData> {
    let id = app.sketch.as_ref()?.feature;
    match &app.open.doc.feature(id)?.kind {
        FeatureKind::Sketch(s) => Some(&s.data),
        _ => None,
    }
}

fn plane_of(app: &App) -> Option<PlaneRef> {
    let id = app.sketch.as_ref()?.feature;
    match &app.open.doc.feature(id)?.kind {
        FeatureKind::Sketch(s) => Some(s.plane),
        _ => None,
    }
}

pub fn frame(app: &App) -> Option<Frame> {
    let id = app.sketch.as_ref()?.feature;
    let r = app.result.as_ref();
    if let Some(f) = r.and_then(|r| r.sketches.iter().find(|s| s.feature == id)).map(|s| s.frame) {
        return Some(f);
    }
    match plane_of(app)? {
        PlaneRef::Origin(o) => Some(scc_solidos::origin_frame(o)),
        PlaneRef::Datum(d) => r?.planes.iter().find(|p| p.id == Some(d)).map(|p| p.frame),
    }
}

struct View {
    cam: Camera,
    rect: Rect,
    f: Frame,
}

impl View {
    fn to_screen(&self, uv: [f64; 2]) -> Pos2 {
        let p = self.f.point(uv[0], uv[1]);
        self.cam.project(self.rect, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)).unwrap_or(Pos2::new(-1e4, -1e4))
    }
    fn to_uv(&self, pos: Pos2) -> Option<[f64; 2]> {
        let (o, d) = self.cam.ray(self.rect, pos);
        let n = Vec3::from(self.f.normal.map(|x| x as f32));
        let po = Vec3::from(self.f.origin.map(|x| x as f32));
        let den = d.dot(n);
        if den.abs() < 1e-6 {
            return None;
        }
        let t = (po - o).dot(n) / den;
        let hit = o + d * t - po;
        let (x, y) = (Vec3::from(self.f.x.map(|v| v as f32)), Vec3::from(self.f.y().map(|v| v as f32)));
        Some([hit.dot(x) as f64, hit.dot(y) as f64])
    }
    /// pixels por milímetro (aprox.)
    fn ppm(&self) -> f32 {
        (self.to_screen([1., 0.]) - self.to_screen([0., 0.])).length().max(1e-4)
    }
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0., 1.);
    (a + ab * t - p).length()
}

fn d2(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn circum(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<([f64; 2], f64)> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let sq = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let ux = (sq(a) * (b[1] - c[1]) + sq(b) * (c[1] - a[1]) + sq(c) * (a[1] - b[1])) / d;
    let uy = (sq(a) * (c[0] - b[0]) + sq(b) * (a[0] - c[0]) + sq(c) * (b[0] - a[0])) / d;
    Some(([ux, uy], d2([ux, uy], a)))
}

// ---------------------------------------------------------------- operações de modelo

/// Ponto para um snap: existente, origem (cria/acha o ponto fixo da origem) ou novo.
fn point_for(sk: &mut SketchData, al: &mut impl FnMut() -> Id, s: Snap) -> Id {
    if let Some(p) = s.point {
        return p;
    }
    if s.origin {
        if let Some(p) = sk.points.iter().find(|p| p.fixed && p.x.abs() < 1e-9 && p.y.abs() < 1e-9) {
            return p.id;
        }
        return scc_solidos::origin_point(sk, al);
    }
    sk.add_point(al, s.uv[0], s.uv[1])
}

fn is_line(sk: &SketchData, id: Id) -> bool {
    matches!(sk.curve(id).map(|c| &c.geom), Some(Curve::Line { .. }))
}
fn is_round(sk: &SketchData, id: Id) -> bool {
    matches!(sk.curve(id).map(|c| &c.geom), Some(Curve::Circle { .. } | Curve::Arc { .. }))
}

/// Aplica uma restrição à seleção. Devolve Err com a explicação quando a seleção não serve.
pub fn apply_constraint(sk: &mut SketchData, al: &mut impl FnMut() -> Id, k: CKind, sel: &[Ent]) -> Result<(), &'static str> {
    let pts: Vec<Id> = sel.iter().filter_map(|e| if let Ent::Pt(p) = e { Some(*p) } else { None }).collect();
    let cvs: Vec<Id> = sel.iter().filter_map(|e| if let Ent::Cv(c) = e { Some(*c) } else { None }).collect();
    let lines: Vec<Id> = cvs.iter().copied().filter(|c| is_line(sk, *c)).collect();
    let rounds: Vec<Id> = cvs.iter().copied().filter(|c| is_round(sk, *c)).collect();
    let mut add = |sk: &mut SketchData, kind| {
        sk.add_constraint(al, kind);
    };
    match k {
        CKind::Horizontal | CKind::Vertical => {
            if !lines.is_empty() {
                for l in lines {
                    add(sk, if k == CKind::Horizontal { Kind::Horizontal(l) } else { Kind::Vertical(l) });
                }
            } else if pts.len() == 2 {
                add(sk, if k == CKind::Horizontal { Kind::HorizontalPts(pts[0], pts[1]) } else { Kind::VerticalPts(pts[0], pts[1]) });
            } else {
                return Err("selecione uma linha ou dois pontos");
            }
        }
        CKind::Parallel | CKind::Perpendicular => {
            if lines.len() != 2 {
                return Err("selecione duas linhas");
            }
            add(sk, if k == CKind::Parallel { Kind::Parallel(lines[0], lines[1]) } else { Kind::Perpendicular(lines[0], lines[1]) });
        }
        CKind::Tangent => match (lines.len(), rounds.len()) {
            (1, 1) => add(sk, Kind::Tangent(lines[0], rounds[0])),
            (0, 2) => add(sk, Kind::Tangent(rounds[0], rounds[1])),
            _ => return Err("selecione linha + círculo/arco, ou dois círculos"),
        },
        CKind::Equal => {
            if lines.len() == 2 {
                add(sk, Kind::Equal(lines[0], lines[1]))
            } else if rounds.len() == 2 {
                add(sk, Kind::Equal(rounds[0], rounds[1]))
            } else {
                return Err("selecione duas linhas ou dois círculos/arcos");
            }
        }
        CKind::Concentric => {
            if rounds.len() != 2 {
                return Err("selecione dois círculos/arcos");
            }
            add(sk, Kind::Concentric(rounds[0], rounds[1]));
        }
        CKind::Coincident => {
            if pts.len() == 2 {
                let (keep, gone) = if sk.point(pts[1]).is_some_and(|p| p.fixed) { (pts[1], pts[0]) } else { (pts[0], pts[1]) };
                sk.merge_points(gone, keep);
            } else if pts.len() == 1 && cvs.len() == 1 {
                add(sk, Kind::PointOnCurve(pts[0], cvs[0]));
            } else {
                return Err("selecione dois pontos, ou um ponto e uma curva");
            }
        }
        CKind::Midpoint => {
            if pts.len() != 1 || lines.len() != 1 {
                return Err("selecione um ponto e uma linha");
            }
            add(sk, Kind::Midpoint(pts[0], lines[0]));
        }
        CKind::Fixed => {
            if pts.is_empty() {
                return Err("selecione pontos");
            }
            for p in pts {
                add(sk, Kind::Fixed(p));
            }
        }
        CKind::Symmetric => {
            if pts.len() != 2 || lines.len() != 1 {
                return Err("selecione dois pontos e uma linha");
            }
            add(sk, Kind::Symmetric(pts[0], pts[1], lines[0]));
        }
    }
    Ok(())
}

/// Cota inteligente a partir da seleção. `mods`: (ctrl, alt).
fn dimension_for(sk: &SketchData, sel: &[Ent], mods: (bool, bool)) -> Option<Kind> {
    let e = String::new();
    match sel {
        [Ent::Cv(c)] => match sk.curve(*c)?.geom {
            Curve::Line { .. } => Some(Kind::Length(*c, e)),
            Curve::Circle { .. } => Some(Kind::Diameter(*c, e)),
            Curve::Arc { .. } => Some(Kind::Radius(*c, e)),
            Curve::Point { .. } => None,
        },
        [Ent::Pt(a), Ent::Pt(b)] => Some(match mods {
            (true, _) => Kind::DistanceX(*a, *b, e),
            (_, true) => Kind::DistanceY(*a, *b, e),
            _ => Kind::Distance(*a, *b, e),
        }),
        [Ent::Cv(a), Ent::Cv(b)] if is_line(sk, *a) && is_line(sk, *b) => Some(Kind::Angle(*a, *b, e)),
        [Ent::Pt(p), Ent::Cv(l)] | [Ent::Cv(l), Ent::Pt(p)] if is_line(sk, *l) => Some(Kind::PointLineDistance(*p, *l, e)),
        _ => None,
    }
}

/// Cota para as ferramentas específicas (horizontal, vertical, ângulo, raio, diâmetro).
fn dimension_for_tool(sk: &SketchData, sel: &[Ent], tool: Tool, mods: (bool, bool)) -> Option<Kind> {
    let e = String::new();
    let line_pts = |c: Id| match sk.curve(c).map(|c| &c.geom) {
        Some(Curve::Line { a, b }) => Some((*a, *b)),
        _ => None,
    };
    match tool {
        Tool::Dimension => dimension_for(sk, sel, mods),
        Tool::DimH | Tool::DimV => {
            let (a, b) = match sel {
                [Ent::Pt(a), Ent::Pt(b)] => (*a, *b),
                [Ent::Cv(c)] => line_pts(*c)?,
                _ => return None,
            };
            Some(if tool == Tool::DimH { Kind::DistanceX(a, b, e) } else { Kind::DistanceY(a, b, e) })
        }
        Tool::DimAngle => match sel {
            [Ent::Cv(a), Ent::Cv(b)] if is_line(sk, *a) && is_line(sk, *b) => Some(Kind::Angle(*a, *b, e)),
            _ => None,
        },
        Tool::DimRadius | Tool::DimDiameter => match sel {
            [Ent::Cv(c)] if is_round(sk, *c) => Some(if tool == Tool::DimRadius { Kind::Radius(*c, e) } else { Kind::Diameter(*c, e) }),
            _ => None,
        },
        _ => None,
    }
}

/// Seleção ainda incompleta para a ferramenta de cota?
fn dimension_waits(sk: &SketchData, sel: &[Ent], tool: Tool) -> bool {
    match tool {
        Tool::Dimension => dimension_needs_more(sk, sel),
        Tool::DimH | Tool::DimV => matches!(sel, [Ent::Pt(_)]),
        Tool::DimAngle => matches!(sel, [Ent::Cv(c)] if is_line(sk, *c)),
        _ => false,
    }
}

fn dimension_needs_more(sk: &SketchData, sel: &[Ent]) -> bool {
    matches!(sel, [Ent::Pt(_)]) || matches!(sel, [Ent::Cv(c)] if matches!(sk.curve(*c).map(|c| &c.geom), Some(Curve::Point { .. })))
}

/// Remove a seleção do sketch.
pub fn delete_selection(sk: &mut SketchData, sel: &[Ent]) {
    for e in sel {
        match *e {
            Ent::Cs(c) => sk.constraints.retain(|x| x.id != c),
            Ent::Cv(c) => sk.remove_curve(c),
            Ent::Pt(p) => {
                let users: Vec<Id> = sk.curves.iter().filter(|c| SketchData::curve_points(&c.geom).contains(&p)).map(|c| c.id).collect();
                for u in users {
                    sk.remove_curve(u);
                }
                if !sk.point(p).is_some_and(|x| x.fixed) {
                    sk.points.retain(|x| x.id != p);
                }
                sk.prune_constraints();
            }
        }
    }
}

// ---------------------------------------------------------------- desenho e interação

const FREE: Color32 = Color32::from_rgb(80, 170, 255);
const SEL: Color32 = Color32::from_rgb(255, 205, 60);
const BAD: Color32 = Color32::from_rgb(255, 90, 80);
const CONSTR: Color32 = Color32::from_rgb(230, 160, 70);

fn glyph(k: &Kind) -> &'static str {
    match k {
        Kind::Coincident(..) => "●",
        Kind::Horizontal(_) | Kind::HorizontalPts(..) => "H",
        Kind::Vertical(_) | Kind::VerticalPts(..) => "V",
        Kind::Parallel(..) => "∥",
        Kind::Perpendicular(..) => "⊥",
        Kind::Equal(..) => "=",
        Kind::Tangent(..) => "T",
        Kind::Concentric(..) => "◎",
        Kind::Midpoint(..) => "M",
        Kind::PointOnCurve(..) => "∈",
        Kind::Fixed(_) => "⚓",
        Kind::Symmetric(..) => "⇋",
        _ => "",
    }
}

/// Desenha o sketch em edição e trata a interação. Chamado pela viewport.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, resp: &egui::Response, cam: Camera) {
    let Some(f) = frame(app) else { return };
    let Some(mut sk) = data(app).cloned() else {
        app.sketch = None;
        return;
    };
    let v = View { cam, rect, f };
    let vars = app.vars();
    let dark = !app.light_background();
    let defined = if dark { Color32::from_rgb(235, 238, 242) } else { Color32::from_rgb(20, 24, 30) };
    let painter = ui.painter_at(rect);
    let ppm = v.ppm();
    let original = sk.clone();
    let settings = app.settings.clone();
    let mut toast: Option<(String, bool)> = None;

    // ---------- grade e eixos
    if settings.grid {
        let mut step = settings.grid_step.max(0.01) as f64;
        while (step as f32) * ppm < 14. {
            step *= if (step / settings.grid_step as f64).log10().fract().abs() < 0.01 { 5. } else { 2. };
        }
        let corners: Vec<[f64; 2]> = [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()].iter().filter_map(|p| v.to_uv(*p)).collect();
        if corners.len() == 4 {
            let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
            for c in &corners {
                for i in 0..2 {
                    lo[i] = lo[i].min(c[i]);
                    hi[i] = hi[i].max(c[i]);
                }
            }
            let gc = if dark { Color32::from_white_alpha(14) } else { Color32::from_black_alpha(18) };
            let gc5 = if dark { Color32::from_white_alpha(30) } else { Color32::from_black_alpha(38) };
            let n0 = (lo[0] / step).floor() as i64;
            let n1 = (hi[0] / step).ceil() as i64;
            if n1 - n0 < 400 {
                for i in n0..=n1 {
                    let x = i as f64 * step;
                    let c = if i % 5 == 0 { gc5 } else { gc };
                    painter.line_segment([v.to_screen([x, lo[1]]), v.to_screen([x, hi[1]])], Stroke::new(1., c));
                }
            }
            let m0 = (lo[1] / step).floor() as i64;
            let m1 = (hi[1] / step).ceil() as i64;
            if m1 - m0 < 400 {
                for i in m0..=m1 {
                    let y = i as f64 * step;
                    let c = if i % 5 == 0 { gc5 } else { gc };
                    painter.line_segment([v.to_screen([lo[0], y]), v.to_screen([hi[0], y])], Stroke::new(1., c));
                }
            }
            painter.line_segment([v.to_screen([lo[0], 0.]), v.to_screen([hi[0], 0.])], Stroke::new(1.2, Color32::from_rgba_unmultiplied(230, 80, 80, 110)));
            painter.line_segment([v.to_screen([0., lo[1]]), v.to_screen([0., hi[1]])], Stroke::new(1.2, Color32::from_rgba_unmultiplied(80, 200, 90, 110)));
        }
    }

    let Some(edit) = app.sketch.as_mut() else { return };
    let report = edit.report.clone();
    let failing: Vec<Id> = report.failing.clone();
    let selected = |e: Ent, ed: &SketchEdit| ed.sel.contains(&e);

    // ---------- curvas
    let pt_color = |id: Id| if report.fixed_points.contains(&id) { defined } else { FREE };
    for c in &sk.curves {
        let bad = failing.iter().any(|f| sk.constraints.iter().any(|k| k.id == *f && k.kind.refs().contains(&c.id)));
        let all_fixed = SketchData::curve_points(&c.geom).iter().all(|p| report.fixed_points.contains(p));
        let mut col = if c.construction { CONSTR } else if all_fixed { defined } else { FREE };
        if bad {
            col = BAD;
        }
        if selected(Ent::Cv(c.id), edit) {
            col = SEL;
        }
        let w = if c.construction { 1.2 } else { 2.0 };
        let pts: Vec<Pos2> = match c.geom {
            Curve::Line { a, b } => vec![v.to_screen(sk.pos(a)), v.to_screen(sk.pos(b))],
            Curve::Circle { c: ctr, r } => {
                let cc = sk.pos(ctr);
                (0..=72).map(|i| {
                    let t = i as f64 / 72. * std::f64::consts::TAU;
                    v.to_screen([cc[0] + r * t.cos(), cc[1] + r * t.sin()])
                })
                .collect()
            }
            Curve::Arc { c: ctr, s, e } => {
                let (pc, ps, pe) = (sk.pos(ctr), sk.pos(s), sk.pos(e));
                let r = d2(ps, pc);
                let a0 = (ps[1] - pc[1]).atan2(ps[0] - pc[0]);
                let mut a1 = (pe[1] - pc[1]).atan2(pe[0] - pc[0]);
                while a1 <= a0 {
                    a1 += std::f64::consts::TAU;
                }
                (0..=48).map(|i| {
                    let t = a0 + (a1 - a0) * i as f64 / 48.;
                    v.to_screen([pc[0] + r * t.cos(), pc[1] + r * t.sin()])
                })
                .collect()
            }
            Curve::Point { .. } => vec![],
        };
        if c.construction {
            for w2 in pts.windows(2) {
                let shapes = egui::Shape::dashed_line(&[w2[0], w2[1]], Stroke::new(w, col), 6., 4.);
                painter.extend(shapes);
            }
        } else if pts.len() >= 2 {
            painter.add(egui::Shape::line(pts, Stroke::new(w, col)));
        }
    }
    for p in &sk.points {
        let s = v.to_screen([p.x, p.y]);
        let mut col = pt_color(p.id);
        if selected(Ent::Pt(p.id), edit) {
            col = SEL;
        }
        let is_hole = sk.curves.iter().any(|c| matches!(c.geom, Curve::Point { p: q } if q == p.id) && !c.construction);
        let r = if is_hole { 4.5 } else { 3.0 };
        painter.rect_filled(Rect::from_center_size(s, egui::vec2(r * 2., r * 2.)), 1., col);
        if p.fixed {
            painter.circle_stroke(s, 6., Stroke::new(1.2, col));
        }
    }

    // ---------- restrições (glifos) e cotas
    edit.hits.clear();
    let mut stack: HashMap<(i32, i32), i32> = HashMap::new();
    let font = egui::FontId::proportional(12.);
    let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
    let anchor = |id: Id| -> [f64; 2] {
        if let Some(p) = sk.point(id) {
            return [p.x, p.y];
        }
        match sk.curve(id).map(|c| &c.geom) {
            Some(Curve::Line { a, b }) => mid(sk.pos(*a), sk.pos(*b)),
            Some(Curve::Circle { c, r }) => {
                let cc = sk.pos(*c);
                [cc[0] + r * 0.7, cc[1] + r * 0.7]
            }
            Some(Curve::Arc { c, s, e }) => SketchData::arc_mid(sk.pos(*c), sk.pos(*s), sk.pos(*e)),
            Some(Curve::Point { p }) => sk.pos(*p),
            None => [0., 0.],
        }
    };
    for c in &sk.constraints {
        let bad = failing.contains(&c.id);
        let sel = edit.sel.contains(&Ent::Cs(c.id));
        if !c.kind.is_dimension() {
            let refs = c.kind.refs();
            let at = match &c.kind {
                Kind::HorizontalPts(a, b) | Kind::VerticalPts(a, b) => mid(anchor(*a), anchor(*b)),
                Kind::Symmetric(_, _, l) => anchor(*l),
                _ => anchor(refs[0]),
            };
            let s = v.to_screen(at);
            let key = ((s.x / 12.) as i32, (s.y / 12.) as i32);
            let n = stack.entry(key).or_default();
            let pos = s + egui::vec2(10. + *n as f32 * 15., 10.);
            *n += 1;
            let r = Rect::from_center_size(pos, egui::vec2(13., 13.));
            // símbolo discreto; destaca quando o elemento ou a restrição está selecionado
            let related = refs.iter().any(|x| edit.sel.contains(&Ent::Cv(*x)) || edit.sel.contains(&Ent::Pt(*x)));
            let col = if bad { BAD } else if sel || related { SEL } else if dark { Color32::from_white_alpha(120) } else { Color32::from_black_alpha(120) };
            if sel || related || bad {
                painter.circle_stroke(pos, 7., Stroke::new(1., col));
            }
            painter.text(pos, egui::Align2::CENTER_CENTER, glyph(&c.kind), egui::FontId::proportional(10.), col);
            edit.hits.push((r, Ent::Cs(c.id)));
            continue;
        }
        // ---- cotas
        let col = if bad { BAD } else if sel { SEL } else if dark { Color32::from_rgb(255, 215, 120) } else { Color32::from_rgb(150, 70, 0) };
        let val = sk.measure(&c.kind).unwrap_or(0.);
        let mut e = c.kind.clone();
        let expr = e.value_mut().cloned().unwrap_or_default();
        let num = expr.trim().parse::<f64>().is_ok();
        let txt = {
            let base = match &c.kind {
                Kind::Radius(..) => format!("R{}", fmt(val)),
                Kind::Diameter(..) => format!("Ø{}", fmt(val)),
                Kind::Angle(..) => format!("{}°", fmt(val)),
                _ => fmt(val),
            };
            if num { base } else { format!("{expr} = {base}") }
        };
        let st = Stroke::new(1., col);
        let arrow = |a: Pos2, b: Pos2| {
            let d = (b - a).normalized();
            let n = egui::vec2(-d.y, d.x);
            let tip = b;
            painter.add(egui::Shape::convex_polygon(vec![tip, tip - d * 8. + n * 3., tip - d * 8. - n * 3.], col, Stroke::NONE));
        };
        let linear = |a: [f64; 2], b: [f64; 2], dir: Option<[f32; 2]>| -> Pos2 {
            let (sa, sb) = (v.to_screen(a), v.to_screen(b));
            let (pa, pb, n) = match dir {
                // horizontal/vertical na tela do plano
                Some(d) => {
                    let d = egui::vec2(d[0], d[1]);
                    let n = egui::vec2(-d.y, d.x);
                    let base = if (sa - Pos2::ZERO).dot(n) > (sb - Pos2::ZERO).dot(n) { sa } else { sb };
                    let pa = sa + n * (base - sa).dot(n);
                    let pb = sb + n * (base - sb).dot(n);
                    (pa, pb, n)
                }
                None => {
                    let d = (sb - sa).normalized();
                    (sa, sb, egui::vec2(-d.y, d.x))
                }
            };
            let off = n * 26.;
            let (qa, qb) = (pa + off, pb + off);
            painter.line_segment([sa, qa + n * 4.], Stroke::new(0.8, col));
            painter.line_segment([sb, qb + n * 4.], Stroke::new(0.8, col));
            painter.line_segment([qa, qb], st);
            arrow(qb, qa);
            arrow(qa, qb);
            qa + (qb - qa) / 2. + n * 2.
        };
        let ux = v.to_screen([1., 0.]) - v.to_screen([0., 0.]);
        let uy = v.to_screen([0., 1.]) - v.to_screen([0., 0.]);
        let tpos = match &c.kind {
            Kind::Length(l, _) => match sk.curve(*l).map(|c| &c.geom) {
                Some(Curve::Line { a, b }) => linear(sk.pos(*a), sk.pos(*b), None),
                _ => continue,
            },
            Kind::Distance(a, b, _) => linear(sk.pos(*a), sk.pos(*b), None),
            Kind::DistanceX(a, b, _) => {
                let d = ux.normalized();
                linear(sk.pos(*a), sk.pos(*b), Some([d.x, d.y]))
            }
            Kind::DistanceY(a, b, _) => {
                let d = uy.normalized();
                linear(sk.pos(*a), sk.pos(*b), Some([d.x, d.y]))
            }
            Kind::PointLineDistance(p, l, _) => match sk.curve(*l).map(|c| &c.geom) {
                Some(Curve::Line { a, b }) => {
                    let (pa, pb, pp) = (sk.pos(*a), sk.pos(*b), sk.pos(*p));
                    let d = [pb[0] - pa[0], pb[1] - pa[1]];
                    let t = ((pp[0] - pa[0]) * d[0] + (pp[1] - pa[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]).max(1e-12);
                    linear(pp, [pa[0] + d[0] * t, pa[1] + d[1] * t], None)
                }
                _ => continue,
            },
            Kind::Radius(cv, _) | Kind::Diameter(cv, _) => {
                let (ctr, r) = match sk.curve(*cv).map(|c| &c.geom) {
                    Some(Curve::Circle { c, r }) => (sk.pos(*c), *r),
                    Some(Curve::Arc { c, s, .. }) => (sk.pos(*c), d2(sk.pos(*s), sk.pos(*c))),
                    _ => continue,
                };
                let edge = [ctr[0] + r * 0.7071, ctr[1] + r * 0.7071];
                let (sc, se) = (v.to_screen(ctr), v.to_screen(edge));
                let d = (se - sc).normalized();
                let out = se + d * 30.;
                if matches!(c.kind, Kind::Diameter(..)) {
                    let opp = v.to_screen([ctr[0] - r * 0.7071, ctr[1] - r * 0.7071]);
                    painter.line_segment([opp, out], st);
                    arrow(sc, opp);
                } else {
                    painter.line_segment([sc, out], st);
                }
                arrow(sc, se);
                out + d * 4.
            }
            Kind::Angle(l1, l2, _) => match (sk.curve(*l1).map(|c| c.geom.clone()), sk.curve(*l2).map(|c| c.geom.clone())) {
                (Some(Curve::Line { a, b }), Some(Curve::Line { a: c2, b: d })) => {
                    // vértice: extremo compartilhado ou ponto médio entre as linhas
                    let shared = [a, b].into_iter().find(|p| *p == c2 || *p == d);
                    let vtx = shared.map(|p| sk.pos(p)).unwrap_or(mid(sk.pos(a), sk.pos(c2)));
                    let sv = v.to_screen(vtx);
                    let other = |x: Id, y: Id| if Some(x) == shared { y } else { x };
                    let (p1, p2) = (v.to_screen(sk.pos(other(a, b))), v.to_screen(sk.pos(other(c2, d))));
                    let (a1, a2) = ((p1 - sv).angle(), (p2 - sv).angle());
                    let mut da = a2 - a1;
                    while da > std::f32::consts::PI {
                        da -= std::f32::consts::TAU;
                    }
                    while da < -std::f32::consts::PI {
                        da += std::f32::consts::TAU;
                    }
                    let pts: Vec<Pos2> = (0..=20).map(|i| sv + egui::Vec2::angled(a1 + da * i as f32 / 20.) * 34.).collect();
                    painter.add(egui::Shape::line(pts, st));
                    sv + egui::Vec2::angled(a1 + da / 2.) * 46.
                }
                _ => continue,
            },
            _ => continue,
        };
        let galley = painter.layout_no_wrap(txt, font.clone(), col);
        let r = Rect::from_center_size(tpos, galley.size() + egui::vec2(8., 4.));
        painter.rect_filled(r, 3., if dark { Color32::from_black_alpha(170) } else { Color32::from_white_alpha(220) });
        painter.galley(r.min + egui::vec2(4., 2.), galley, col);
        edit.hits.push((r, Ent::Cs(c.id)));
    }

    // ---------- checagem de contornos
    if edit.check {
        let mut degree: HashMap<Id, usize> = HashMap::new();
        for c in sk.curves.iter().filter(|c| !c.construction) {
            let ends = match c.geom {
                Curve::Line { a, b } => vec![a, b],
                Curve::Arc { s, e, .. } => vec![s, e],
                _ => vec![],
            };
            for p in ends {
                *degree.entry(p).or_default() += 1;
            }
        }
        let open: Vec<Id> = degree.iter().filter(|(_, d)| **d == 1).map(|(p, _)| *p).collect();
        for p in &open {
            painter.circle_stroke(v.to_screen(sk.pos(*p)), 9., Stroke::new(2.2, BAD));
        }
        let mut overlaps = 0;
        for (i, a) in sk.points.iter().enumerate() {
            for b in &sk.points[i + 1..] {
                if (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6 {
                    overlaps += 1;
                    painter.circle_stroke(v.to_screen([a.x, a.y]), 12., Stroke::new(1.6, Color32::from_rgb(255, 160, 40)));
                }
            }
        }
        let (loops, _) = sk.loops();
        let msg = format!("Contornos: {} fechado(s) · {} ponta(s) aberta(s) · {} ponto(s) sobreposto(s)", loops.len(), open.len(), overlaps);
        let col = if open.is_empty() && overlaps == 0 { Color32::from_rgb(120, 210, 130) } else { Color32::from_rgb(255, 170, 60) };
        painter.text(rect.left_top() + egui::vec2(10., 62.), egui::Align2::LEFT_TOP, msg, egui::FontId::proportional(12.), col);
    }

    // ---------- entrada do mouse
    let (shift, ctrl, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.command, i.modifiers.alt));
    let hover = resp.hover_pos();
    let cursor_uv = hover.and_then(|p| v.to_uv(p));

    // pick: pontos > rótulos > curvas
    let pick = |pos: Pos2, sk: &SketchData, hits: &[(Rect, Ent)]| -> Option<Ent> {
        let mut best: Option<(f32, Ent)> = None;
        for p in &sk.points {
            let d = (v.to_screen([p.x, p.y]) - pos).length();
            if d < 9. && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, Ent::Pt(p.id)));
            }
        }
        if best.is_some() {
            return best.map(|b| b.1);
        }
        if let Some((_, e)) = hits.iter().rev().find(|(r, _)| r.contains(pos)) {
            return Some(*e);
        }
        for c in &sk.curves {
            let d = match c.geom {
                Curve::Line { a, b } => seg_dist(pos, v.to_screen(sk.pos(a)), v.to_screen(sk.pos(b))),
                Curve::Circle { c: ctr, r } => ((pos - v.to_screen(sk.pos(ctr))).length() - r as f32 * ppm).abs(),
                Curve::Arc { c: ctr, s, .. } => ((pos - v.to_screen(sk.pos(ctr))).length() - d2(sk.pos(s), sk.pos(ctr)) as f32 * ppm).abs(),
                Curve::Point { .. } => f32::MAX,
            };
            if d < 7. && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, Ent::Cv(c.id)));
            }
        }
        best.map(|b| b.1)
    };

    let snap_at = |pos: Pos2, sk: &SketchData, exclude: &[Id]| -> Option<Snap> {
        let uv = v.to_uv(pos)?;
        for p in &sk.points {
            if exclude.contains(&p.id) {
                continue;
            }
            if (v.to_screen([p.x, p.y]) - pos).length() < 10. {
                return Some(Snap { uv: [p.x, p.y], point: Some(p.id), origin: false });
            }
        }
        if settings.snaps && (v.to_screen([0., 0.]) - pos).length() < 10. {
            return Some(Snap { uv: [0., 0.], point: None, origin: true });
        }
        if settings.snaps && settings.grid {
            let step = {
                let mut s = settings.grid_step.max(0.01) as f64;
                while (s as f32) * ppm < 14. {
                    s *= if (s / settings.grid_step as f64).log10().fract().abs() < 0.01 { 5. } else { 2. };
                }
                s
            };
            let g = [(uv[0] / step).round() * step, (uv[1] / step).round() * step];
            if (v.to_screen(g) - pos).length() < 7. {
                return Some(Snap { uv: g, point: None, origin: false });
            }
        }
        Some(Snap { uv, point: None, origin: false })
    };

    let mut al_ids: Vec<Id> = vec![];
    // ids alocados aqui vêm do documento depois (para não emprestar o documento duas vezes)
    let mut next_tmp = 0u64;
    let base_id = app.open.doc.peek_id();
    let mut al = || {
        let id = Id(base_id + next_tmp);
        next_tmp += 1;
        id
    };

    let tool = edit.tool;
    let cur_snap = hover.and_then(|p| {
        let excl: Vec<Id> = edit.drag.as_ref().map(|d| d.pts.iter().map(|x| x.0).collect()).unwrap_or_default();
        snap_at(p, &sk, &excl)
    });

    // destaque do snap
    if let (Some(s), true) = (cur_snap, tool != Tool::Select) {
        let sp = v.to_screen(s.uv);
        if s.point.is_some() || s.origin {
            painter.circle_stroke(sp, 7., Stroke::new(1.6, SEL));
        } else {
            painter.circle_filled(sp, 2.5, SEL);
        }
    }

    let primary_click = resp.clicked_by(egui::PointerButton::Primary);
    let secondary_click = resp.clicked_by(egui::PointerButton::Secondary);
    let double = resp.double_clicked();

    match tool {
        Tool::Select => {
            // arrastar
            if resp.drag_started_by(egui::PointerButton::Primary) {
                let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
                let start = v.to_uv(origin).unwrap_or([0., 0.]);
                match pick(origin, &sk, &edit.hits) {
                    Some(Ent::Pt(p)) if !sk.point(p).is_some_and(|x| x.fixed) => edit.drag = Some(Drag { pts: vec![(p, sk.pos(p))], start, circle: None }),
                    Some(Ent::Cv(c)) => match sk.curve(c).map(|c| c.geom.clone()) {
                        Some(Curve::Line { a, b }) => edit.drag = Some(Drag { pts: vec![(a, sk.pos(a)), (b, sk.pos(b))], start, circle: None }),
                        Some(Curve::Circle { .. }) => edit.drag = Some(Drag { pts: vec![], start, circle: Some(c) }),
                        _ => {}
                    },
                    _ => {}
                }
            }
            if let (Some(d), true, Some(uv)) = (&edit.drag, resp.dragged_by(egui::PointerButton::Primary), cursor_uv) {
                let delta = [uv[0] - d.start[0], uv[1] - d.start[1]];
                let mut pins = vec![];
                if d.pts.len() == 1 {
                    let target = cur_snap.filter(|s| s.point.is_none()).map(|s| s.uv).unwrap_or(uv);
                    if let Some(p) = sk.point_mut(d.pts[0].0) {
                        p.x = target[0];
                        p.y = target[1];
                    }
                    pins.push(d.pts[0].0);
                } else {
                    for (id, o) in &d.pts {
                        if let Some(p) = sk.point_mut(*id) {
                            if !p.fixed {
                                p.x = o[0] + delta[0];
                                p.y = o[1] + delta[1];
                            }
                        }
                        pins.push(*id);
                    }
                }
                if let Some(cid) = d.circle {
                    let ctr = match sk.curve(cid).map(|c| &c.geom) {
                        Some(Curve::Circle { c, .. }) => sk.pos(*c),
                        _ => [0., 0.],
                    };
                    if let Some(scc_core::sketch::SkCurve { geom: Curve::Circle { r, .. }, .. }) = sk.curves.iter_mut().find(|c| c.id == cid) {
                        *r = d2(uv, ctr).max(1e-3);
                    }
                }
                edit.report = sk.solve(&vars, &pins);
            }
            if resp.drag_stopped() && edit.drag.take().is_some() {
                edit.report = sk.solve(&vars, &[]);
            }
            if primary_click {
                match hover.and_then(|p| pick(p, &sk, &edit.hits)) {
                    Some(e) => {
                        if shift {
                            if let Some(i) = edit.sel.iter().position(|x| *x == e) {
                                edit.sel.remove(i);
                            } else {
                                edit.sel.push(e);
                            }
                        } else {
                            edit.sel = vec![e];
                        }
                        if double {
                            if let Ent::Cs(c) = e {
                                if let Some(k) = sk.constraints.iter_mut().find(|x| x.id == c).and_then(|x| x.kind.value_mut()) {
                                    edit.dim_edit = Some((c, k.clone(), false));
                                }
                            }
                        }
                    }
                    None => {
                        if !shift {
                            edit.sel.clear()
                        }
                    }
                }
            }
        }

        Tool::Line => {
            // prévia
            if let (Some(last), Some(s)) = (edit.chain_last, cur_snap) {
                let a = v.to_screen(sk.pos(last));
                painter.line_segment([a, v.to_screen(s.uv)], Stroke::new(1.5, FREE.gamma_multiply(0.7)));
                let len = d2(sk.pos(last), s.uv);
                painter.text(v.to_screen(s.uv) + egui::vec2(12., -12.), egui::Align2::LEFT_BOTTOM, fmt(len), font.clone(), FREE);
            }
            if primary_click {
                if let Some(s) = cur_snap {
                    let p = point_for(&mut sk, &mut al, s);
                    match edit.chain_last {
                        None => {
                            edit.chain_first = Some(p);
                            edit.chain_last = Some(p);
                        }
                        Some(last) if last != p => {
                            let l = sk.add_curve(&mut al, Curve::Line { a: last, b: p }, edit.construction);
                            let (a, b) = (sk.pos(last), sk.pos(p));
                            let ang = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees().abs();
                            if settings.auto_constraints {
                                if ang < 2.5 || (180. - ang) < 2.5 {
                                    sk.add_constraint(&mut al, Kind::Horizontal(l));
                                } else if (ang - 90.).abs() < 2.5 {
                                    sk.add_constraint(&mut al, Kind::Vertical(l));
                                }
                            }
                            if settings.auto_dims {
                                sk.add_constraint(&mut al, Kind::Length(l, fmt(d2(a, b))));
                            }
                            if Some(p) == edit.chain_first {
                                edit.chain_last = None;
                                edit.chain_first = None;
                            } else {
                                edit.chain_last = Some(p);
                            }
                            edit.report = sk.solve(&vars, &[]);
                        }
                        _ => {}
                    }
                }
            }
            if secondary_click || double {
                edit.chain_last = None;
                edit.chain_first = None;
            }
        }

        Tool::Rect | Tool::RectCenter | Tool::Circle | Tool::Circle3 | Tool::Arc3 | Tool::ArcCenter | Tool::Slot | Tool::Polygon | Tool::Point => {
            if primary_click {
                if let Some(s) = cur_snap {
                    edit.clicks.push(s);
                }
            }
            if secondary_click {
                edit.clicks.clear();
            }
            let cl = edit.clicks.clone();
            // prévia da forma
            if let Some(s) = cur_snap {
                let pv = Stroke::new(1.5, FREE.gamma_multiply(0.7));
                let c = cl.iter().map(|x| x.uv).collect::<Vec<_>>();
                let m = s.uv;
                let poly = |pts: &[[f64; 2]], close: bool| {
                    let mut p: Vec<Pos2> = pts.iter().map(|x| v.to_screen(*x)).collect();
                    if close {
                        p.push(p[0]);
                    }
                    painter.add(egui::Shape::line(p, pv));
                };
                let circle = |ctr: [f64; 2], r: f64| {
                    let pts: Vec<[f64; 2]> = (0..=64).map(|i| {
                        let t = i as f64 / 64. * std::f64::consts::TAU;
                        [ctr[0] + r * t.cos(), ctr[1] + r * t.sin()]
                    })
                    .collect();
                    poly(&pts, false);
                };
                match (tool, c.len()) {
                    (Tool::Rect, 1) => poly(&[c[0], [m[0], c[0][1]], m, [c[0][0], m[1]]], true),
                    (Tool::RectCenter, 1) => {
                        let a = [2. * c[0][0] - m[0], 2. * c[0][1] - m[1]];
                        poly(&[a, [m[0], a[1]], m, [a[0], m[1]]], true)
                    }
                    (Tool::Circle | Tool::Polygon, 1) => circle(c[0], d2(c[0], m)),
                    (Tool::Circle3 | Tool::Arc3, 2) => {
                        if let Some((ctr, r)) = circum(c[0], c[1], m) {
                            circle(ctr, r)
                        }
                    }
                    (Tool::ArcCenter, 1) => circle(c[0], d2(c[0], m)),
                    (Tool::ArcCenter, 2) => circle(c[0], d2(c[0], c[1])),
                    (Tool::Slot, 1) => poly(&[c[0], m], false),
                    (Tool::Slot, 2) => {
                        poly(&[c[0], c[1]], false);
                        circle(c[0], d2(c[1], m).max(0.1));
                        circle(c[1], d2(c[1], m).max(0.1));
                    }
                    _ => {}
                }
            }
            if !cl.is_empty() && cl.len() >= tool.clicks_needed() {
                edit.clicks.clear();
                let c = cl.iter().map(|x| x.uv).collect::<Vec<_>>();
                let cons = edit.construction;
                match tool {
                    Tool::Point => {
                        let p = point_for(&mut sk, &mut al, cl[0]);
                        sk.add_curve(&mut al, Curve::Point { p }, cons);
                    }
                    Tool::Rect | Tool::RectCenter => {
                        let (a, b) = if tool == Tool::Rect { (c[0], c[1]) } else { ([2. * c[0][0] - c[1][0], 2. * c[0][1] - c[1][1]], c[1]) };
                        if (a[0] - b[0]).abs() > 1e-6 && (a[1] - b[1]).abs() > 1e-6 {
                            let (p, l) = sk.add_rect(&mut al, a, b, cons);
                            if tool == Tool::RectCenter {
                                let ctr = point_for(&mut sk, &mut al, cl[0]);
                                if cl[0].point.is_none() && !cl[0].origin {
                                    sk.add_curve(&mut al, Curve::Point { p: ctr }, true);
                                }
                                let diag = sk.add_curve(&mut al, Curve::Line { a: p[0], b: p[2] }, true);
                                sk.add_constraint(&mut al, Kind::Midpoint(ctr, diag));
                                if let Some(q) = cl[1].point {
                                    sk.merge_points(p[2], q);
                                }
                            } else {
                                if let Some(q) = cl[0].point {
                                    sk.merge_points(p[0], q);
                                } else if cl[0].origin {
                                    let o = point_for(&mut sk, &mut al, cl[0]);
                                    sk.merge_points(p[0], o);
                                }
                                if let Some(q) = cl[1].point {
                                    sk.merge_points(p[2], q);
                                }
                            }
                            if settings.auto_dims {
                                sk.add_constraint(&mut al, Kind::Length(l[0], fmt((a[0] - b[0]).abs())));
                                sk.add_constraint(&mut al, Kind::Length(l[1], fmt((a[1] - b[1]).abs())));
                            }
                        }
                    }
                    Tool::Circle => {
                        let ctr = point_for(&mut sk, &mut al, cl[0]);
                        let r = d2(c[0], c[1]).max(1e-3);
                        let cv = sk.add_curve(&mut al, Curve::Circle { c: ctr, r }, cons);
                        if settings.auto_dims {
                            sk.add_constraint(&mut al, Kind::Diameter(cv, fmt(2. * r)));
                        }
                    }
                    Tool::Circle3 => {
                        if let Some((ctr, r)) = circum(c[0], c[1], c[2]) {
                            let pc = sk.add_point(&mut al, ctr[0], ctr[1]);
                            sk.add_curve(&mut al, Curve::Circle { c: pc, r }, cons);
                        }
                    }
                    Tool::Arc3 | Tool::ArcCenter => {
                        let geom = if tool == Tool::Arc3 {
                            circum(c[0], c[2], c[1]).map(|(ctr, _)| {
                                // garante sentido anti-horário de início a fim passando pelo meio
                                let ang = |p: [f64; 2]| (p[1] - ctr[1]).atan2(p[0] - ctr[0]);
                                let norm = |x: f64| (x % std::f64::consts::TAU + std::f64::consts::TAU) % std::f64::consts::TAU;
                                let ccw = norm(ang(c[2]) - ang(c[0])) > norm(ang(c[1]) - ang(c[0]));
                                (ctr, if ccw { (0, 1) } else { (1, 0) })
                            })
                        } else {
                            let r = d2(c[0], c[1]);
                            let e = c[2];
                            let de = d2(e, c[0]).max(1e-9);
                            let _ = r;
                            Some((c[0], if ((c[1][0] - c[0][0]) * (e[1] - c[0][1]) - (c[1][1] - c[0][1]) * (e[0] - c[0][0])) >= 0. { (0, 1) } else { (1, 0) })).map(|x| {
                                let _ = de;
                                x
                            })
                        };
                        if let Some((ctr, order)) = geom {
                            let pc = if tool == Tool::ArcCenter { point_for(&mut sk, &mut al, cl[0]) } else { sk.add_point(&mut al, ctr[0], ctr[1]) };
                            let (i0, i1) = if tool == Tool::Arc3 { (0, 1) } else { (1, 2) };
                            let mut ends = [cl[i0], cl[i1]];
                            if tool == Tool::ArcCenter {
                                // projeta o fim no raio do início
                                let r = d2(c[0], c[1]);
                                let e = c[2];
                                let de = d2(e, c[0]).max(1e-9);
                                ends[1] = Snap { uv: [c[0][0] + (e[0] - c[0][0]) * r / de, c[0][1] + (e[1] - c[0][1]) * r / de], point: None, origin: false };
                            }
                            let ps = point_for(&mut sk, &mut al, ends[0]);
                            let pe = point_for(&mut sk, &mut al, ends[1]);
                            let (s, e) = if order == (0, 1) { (ps, pe) } else { (pe, ps) };
                            let arc = sk.add_curve(&mut al, Curve::Arc { c: pc, s, e }, cons);
                            if settings.auto_dims {
                                let r = d2(sk.pos(s), sk.pos(pc));
                                sk.add_constraint(&mut al, Kind::Radius(arc, fmt(r)));
                            }
                        }
                    }
                    Tool::Slot => {
                        let r = d2(c[1], c[2]).max(0.1);
                        let (c1, c2, arc) = sk.add_slot(&mut al, c[0], c[1], r);
                        for (i, q) in [(0usize, c1), (1, c2)] {
                            if let Some(p) = cl[i].point {
                                sk.merge_points(q, p);
                            }
                        }
                        if settings.auto_dims {
                            sk.add_constraint(&mut al, Kind::Distance(c1, c2, fmt(d2(c[0], c[1]))));
                            sk.add_constraint(&mut al, Kind::Radius(arc, fmt(r)));
                        }
                    }
                    Tool::Polygon => {
                        let r = d2(c[0], c[1]).max(0.1);
                        let circ = sk.add_polygon(&mut al, c[0], r, edit.polygon_sides);
                        if let (Some(p), Some(Curve::Circle { c: pc, .. })) = (cl[0].point, sk.curve(circ).map(|c| c.geom.clone())) {
                            sk.merge_points(pc, p);
                        }
                    }
                    _ => {}
                }
                edit.report = sk.solve(&vars, &[]);
            }
            // dica dos cliques
            let hint = match (tool, cl.len()) {
                (Tool::Rect, 0) => "1º canto",
                (Tool::Rect, _) => "canto oposto",
                (Tool::RectCenter, 0) => "centro",
                (Tool::RectCenter, _) => "canto",
                (Tool::Circle, 0) | (Tool::Polygon, 0) | (Tool::ArcCenter, 0) => "centro",
                (Tool::Circle, _) | (Tool::Polygon, _) => "raio",
                (Tool::ArcCenter, 1) => "início do arco",
                (Tool::ArcCenter, _) => "fim do arco",
                (Tool::Circle3, n) => ["1º ponto", "2º ponto", "3º ponto"][n.min(2)],
                (Tool::Arc3, n) => ["início", "fim", "ponto do arco"][n.min(2)],
                (Tool::Slot, n) => ["1º centro", "2º centro", "largura"][n.min(2)],
                _ => "",
            };
            if let (Some(h), false) = (hover, hint.is_empty()) {
                painter.text(h + egui::vec2(14., 14.), egui::Align2::LEFT_TOP, hint, egui::FontId::proportional(11.), FREE);
            }
        }

        Tool::Move | Tool::Copy | Tool::Mirror | Tool::Offset | Tool::Fillet => {
            let picked = if primary_click { hover.and_then(|p| pick(p, &sk, &edit.hits)) } else { None };
            let sel_curves: Vec<Id> = edit.sel.iter().filter_map(|e| if let Ent::Cv(c) = e { Some(*c) } else { None }).collect();
            match tool {
                Tool::Move | Tool::Copy => {
                    if sel_curves.is_empty() {
                        if let Some(Ent::Cv(c)) = picked {
                            edit.sel.push(Ent::Cv(c));
                        }
                    } else if primary_click {
                        if let Some(sn) = cur_snap {
                            edit.clicks.push(sn);
                        }
                    }
                    let cl = edit.clicks.clone();
                    if let (1, Some(sn)) = (cl.len(), cur_snap) {
                        // prévia deslocada
                        let d = [sn.uv[0] - cl[0].uv[0], sn.uv[1] - cl[0].uv[1]];
                        for c in &sel_curves {
                            if let Some(Curve::Line { a, b }) = sk.curve(*c).map(|x| x.geom.clone()) {
                                let (pa, pb) = (sk.pos(a), sk.pos(b));
                                painter.line_segment([v.to_screen([pa[0] + d[0], pa[1] + d[1]]), v.to_screen([pb[0] + d[0], pb[1] + d[1]])], Stroke::new(1.5, FREE.gamma_multiply(0.6)));
                            }
                        }
                    }
                    if cl.len() >= 2 {
                        edit.clicks.clear();
                        let d = [cl[1].uv[0] - cl[0].uv[0], cl[1].uv[1] - cl[0].uv[1]];
                        let pts: Vec<Id> = {
                            let mut v: Vec<Id> = sel_curves.iter().filter_map(|c| sk.curve(*c)).flat_map(|c| SketchData::curve_points(&c.geom)).collect();
                            v.sort();
                            v.dedup();
                            v
                        };
                        if tool == Tool::Move {
                            for p in &pts {
                                if let Some(pp) = sk.point_mut(*p) {
                                    if !pp.fixed {
                                        pp.x += d[0];
                                        pp.y += d[1];
                                    }
                                }
                            }
                            edit.report = sk.solve(&vars, &pts);
                        } else {
                            let map: HashMap<Id, Id> = pts.iter().map(|p| {
                                let q = sk.pos(*p);
                                (*p, sk.add_point(&mut al, q[0] + d[0], q[1] + d[1]))
                            }).collect();
                            for c in &sel_curves {
                                if let Some(cv) = sk.curve(*c).cloned() {
                                    let m = |x: Id| map[&x];
                                    let g = match cv.geom {
                                        Curve::Line { a, b } => Curve::Line { a: m(a), b: m(b) },
                                        Curve::Circle { c, r } => Curve::Circle { c: m(c), r },
                                        Curve::Arc { c, s, e } => Curve::Arc { c: m(c), s: m(s), e: m(e) },
                                        Curve::Point { p } => Curve::Point { p: m(p) },
                                    };
                                    sk.add_curve(&mut al, g, cv.construction);
                                }
                            }
                            edit.report = sk.solve(&vars, &[]);
                        }
                    }
                }
                Tool::Mirror => {
                    if let Some(Ent::Cv(c)) = picked {
                        if !sel_curves.is_empty() && is_line(&sk, c) && !sel_curves.contains(&c) {
                            if let Some(Curve::Line { a: la, b: lb }) = sk.curve(c).map(|x| x.geom.clone()) {
                                let (pa, pb) = (sk.pos(la), sk.pos(lb));
                                let dd = [pb[0] - pa[0], pb[1] - pa[1]];
                                let l2 = dd[0] * dd[0] + dd[1] * dd[1];
                                let refl = |q: [f64; 2]| {
                                    let t = ((q[0] - pa[0]) * dd[0] + (q[1] - pa[1]) * dd[1]) / l2.max(1e-12);
                                    let f = [pa[0] + dd[0] * t, pa[1] + dd[1] * t];
                                    [2. * f[0] - q[0], 2. * f[1] - q[1]]
                                };
                                let mut pts: Vec<Id> = sel_curves.iter().filter_map(|x| sk.curve(*x)).flat_map(|x| SketchData::curve_points(&x.geom)).collect();
                                pts.sort();
                                pts.dedup();
                                let map: HashMap<Id, Id> = pts.iter().map(|p| {
                                    let q = refl(sk.pos(*p));
                                    (*p, sk.add_point(&mut al, q[0], q[1]))
                                }).collect();
                                for x in &sel_curves {
                                    if let Some(cv) = sk.curve(*x).cloned() {
                                        let m = |x: Id| map[&x];
                                        let g = match cv.geom {
                                            Curve::Line { a, b } => Curve::Line { a: m(a), b: m(b) },
                                            Curve::Circle { c, r } => Curve::Circle { c: m(c), r },
                                            // espelhar inverte o sentido do arco
                                            Curve::Arc { c, s, e } => Curve::Arc { c: m(c), s: m(e), e: m(s) },
                                            Curve::Point { p } => Curve::Point { p: m(p) },
                                        };
                                        sk.add_curve(&mut al, g, cv.construction);
                                    }
                                }
                                for (p, q) in &map {
                                    sk.add_constraint(&mut al, Kind::Symmetric(*p, *q, c));
                                }
                                edit.sel.clear();
                                edit.report = sk.solve(&vars, &[]);
                            }
                        } else if let Some(e) = picked {
                            if !edit.sel.contains(&e) {
                                edit.sel.push(e);
                            }
                        }
                    }
                }
                Tool::Offset => {
                    if edit.sel.is_empty() {
                        if let Some(Ent::Cv(c)) = picked {
                            edit.sel = vec![Ent::Cv(c)];
                        }
                    } else if let (true, Some(uv), [Ent::Cv(c)]) = (primary_click, cursor_uv, edit.sel.clone().as_slice()) {
                        let c = *c;
                        match sk.curve(c).map(|x| x.geom.clone()) {
                            Some(Curve::Line { a, b }) => {
                                let (pa, pb) = (sk.pos(a), sk.pos(b));
                                let dd = [pb[0] - pa[0], pb[1] - pa[1]];
                                let len = (dd[0] * dd[0] + dd[1] * dd[1]).sqrt().max(1e-12);
                                let n = [-dd[1] / len, dd[0] / len];
                                let dist = (uv[0] - pa[0]) * n[0] + (uv[1] - pa[1]) * n[1];
                                let na = sk.add_point(&mut al, pa[0] + n[0] * dist, pa[1] + n[1] * dist);
                                let nb = sk.add_point(&mut al, pb[0] + n[0] * dist, pb[1] + n[1] * dist);
                                let l2 = sk.add_curve(&mut al, Curve::Line { a: na, b: nb }, edit.construction);
                                sk.add_constraint(&mut al, Kind::Parallel(c, l2));
                                let id = sk.add_constraint(&mut al, Kind::PointLineDistance(na, c, fmt(dist.abs())));
                                edit.dim_edit = Some((id, fmt(dist.abs()), true));
                            }
                            Some(Curve::Circle { c: ctr, .. }) => {
                                let r2 = d2(uv, sk.pos(ctr)).max(1e-3);
                                let cv = sk.add_curve(&mut al, Curve::Circle { c: ctr, r: r2 }, edit.construction);
                                let id = sk.add_constraint(&mut al, Kind::Diameter(cv, fmt(2. * r2)));
                                edit.dim_edit = Some((id, fmt(2. * r2), true));
                            }
                            Some(Curve::Arc { c: ctr, s: ps, e: pe }) => {
                                let pc = sk.pos(ctr);
                                let r2 = d2(uv, pc).max(1e-3);
                                let at = |q: [f64; 2]| {
                                    let r = d2(q, pc).max(1e-12);
                                    [pc[0] + (q[0] - pc[0]) * r2 / r, pc[1] + (q[1] - pc[1]) * r2 / r]
                                };
                                let (qs, qe) = (at(sk.pos(ps)), at(sk.pos(pe)));
                                let ns = sk.add_point(&mut al, qs[0], qs[1]);
                                let ne = sk.add_point(&mut al, qe[0], qe[1]);
                                let cv = sk.add_curve(&mut al, Curve::Arc { c: ctr, s: ns, e: ne }, edit.construction);
                                let id = sk.add_constraint(&mut al, Kind::Radius(cv, fmt(r2)));
                                edit.dim_edit = Some((id, fmt(r2), true));
                            }
                            _ => {}
                        }
                        edit.sel.clear();
                        edit.report = sk.solve(&vars, &[]);
                    }
                }
                Tool::Fillet => {
                    if let Some(Ent::Pt(p)) = picked {
                        let lines: Vec<(Id, Id)> = sk.curves.iter().filter_map(|c| match c.geom {
                            Curve::Line { a, b } if a == p => Some((c.id, b)),
                            Curve::Line { a, b } if b == p => Some((c.id, a)),
                            _ => None,
                        }).collect();
                        if lines.len() != 2 {
                            toast = Some(("clique num canto onde duas linhas se encontram".into(), true));
                        } else {
                            let pc = sk.pos(p);
                            let (q1, q2) = (sk.pos(lines[0].1), sk.pos(lines[1].1));
                            let unit = |q: [f64; 2]| {
                                let l = d2(q, pc).max(1e-12);
                                ([(q[0] - pc[0]) / l, (q[1] - pc[1]) / l], l)
                            };
                            let ((u1, l1), (u2, l2)) = (unit(q1), unit(q2));
                            let cosang = (u1[0] * u2[0] + u1[1] * u2[1]).clamp(-1., 1.);
                            let th = cosang.acos();
                            if th < 1e-3 || (std::f64::consts::PI - th) < 1e-3 {
                                toast = Some(("as linhas são colineares".into(), true));
                            } else {
                                let t = 0.3 * l1.min(l2);
                                let r = t * (th / 2.).tan();
                                let bis = {
                                    let b = [u1[0] + u2[0], u1[1] + u2[1]];
                                    let l = (b[0] * b[0] + b[1] * b[1]).sqrt().max(1e-12);
                                    [b[0] / l, b[1] / l]
                                };
                                let dc = t / (th / 2.).cos();
                                let ctr = [pc[0] + bis[0] * dc, pc[1] + bis[1] * dc];
                                let t1 = sk.add_point(&mut al, pc[0] + u1[0] * t, pc[1] + u1[1] * t);
                                let t2 = sk.add_point(&mut al, pc[0] + u2[0] * t, pc[1] + u2[1] * t);
                                let cc = sk.add_point(&mut al, ctr[0], ctr[1]);
                                // as linhas passam a terminar nos pontos de tangência
                                for (lid, tp) in [(lines[0].0, t1), (lines[1].0, t2)] {
                                    if let Some(scc_core::sketch::SkCurve { geom: Curve::Line { a, b }, .. }) = sk.curves.iter_mut().find(|c| c.id == lid) {
                                        if *a == p {
                                            *a = tp
                                        } else {
                                            *b = tp
                                        }
                                    }
                                }
                                let (s1, s2) = (sk.pos(t1), sk.pos(t2));
                                let ccw = (s1[0] - ctr[0]) * (s2[1] - ctr[1]) - (s1[1] - ctr[1]) * (s2[0] - ctr[0]) > 0.;
                                let arc = sk.add_curve(&mut al, if ccw { Curve::Arc { c: cc, s: t1, e: t2 } } else { Curve::Arc { c: cc, s: t2, e: t1 } }, false);
                                sk.add_constraint(&mut al, Kind::Tangent(lines[0].0, arc));
                                sk.add_constraint(&mut al, Kind::Tangent(lines[1].0, arc));
                                let id = sk.add_constraint(&mut al, Kind::Radius(arc, fmt(r)));
                                edit.dim_edit = Some((id, fmt(r), true));
                                // o canto antigo some se ninguém mais o usa
                                if !sk.curves.iter().any(|c| SketchData::curve_points(&c.geom).contains(&p)) {
                                    sk.points.retain(|x| x.id != p || x.fixed);
                                    sk.prune_constraints();
                                }
                                edit.report = sk.solve(&vars, &[]);
                            }
                        }
                    }
                }
                _ => {}
            }
            if secondary_click {
                edit.clicks.clear();
                edit.sel.clear();
            }
            if let Some(h) = hover {
                let hint = match (tool, sel_curves.is_empty(), edit.clicks.len()) {
                    (Tool::Move | Tool::Copy, true, _) => "clique nas curvas a mover",
                    (Tool::Move | Tool::Copy, false, 0) => "ponto base",
                    (Tool::Move | Tool::Copy, false, _) => "destino",
                    (Tool::Mirror, true, _) => "selecione as curvas a espelhar",
                    (Tool::Mirror, false, _) => "clique na linha de simetria",
                    (Tool::Offset, _, _) if edit.sel.is_empty() => "clique na curva",
                    (Tool::Offset, _, _) => "clique onde a paralela deve passar",
                    (Tool::Fillet, _, _) => "clique no canto entre duas linhas",
                    _ => "",
                };
                painter.text(h + egui::vec2(14., 14.), egui::Align2::LEFT_TOP, hint, egui::FontId::proportional(11.), FREE);
            }
        }

        Tool::Dimension | Tool::DimH | Tool::DimV | Tool::DimAngle | Tool::DimRadius | Tool::DimDiameter | Tool::Constr(_) => {
            if primary_click {
                if let Some(e) = hover.and_then(|p| pick(p, &sk, &edit.hits)) {
                    if !matches!(e, Ent::Cs(_)) {
                        // ponto isolado (curva Point) vale como ponto
                        let e = match e {
                            Ent::Cv(c) => match sk.curve(c).map(|c| &c.geom) {
                                Some(Curve::Point { p }) => Ent::Pt(*p),
                                _ => e,
                            },
                            _ => e,
                        };
                        if !edit.sel.contains(&e) {
                            edit.sel.push(e);
                        }
                    }
                }
                match tool {
                    Tool::Dimension | Tool::DimH | Tool::DimV | Tool::DimAngle | Tool::DimRadius | Tool::DimDiameter => {
                        if !dimension_waits(&sk, &edit.sel, tool) {
                            if let Some(mut k) = dimension_for_tool(&sk, &edit.sel, tool, (ctrl, alt)) {
                                let m = sk.measure(&k).unwrap_or(0.);
                                *k.value_mut().unwrap() = fmt(m);
                                let id = sk.add_constraint(&mut al, k);
                                edit.dim_edit = Some((id, fmt(m), true));
                                edit.report = sk.solve(&vars, &[]);
                            }
                            edit.sel.clear();
                        }
                    }
                    Tool::Constr(k) => {
                        if edit.sel.len() >= k.needs() || (k == CKind::Fixed && !edit.sel.is_empty()) {
                            match apply_constraint(&mut sk, &mut al, k, &edit.sel) {
                                Ok(()) => edit.report = sk.solve(&vars, &[]),
                                Err(m) => toast = Some((m.to_string(), true)),
                            }
                            edit.sel.clear();
                        }
                    }
                    _ => {}
                }
            }
            if secondary_click {
                edit.sel.clear();
            }
            if let (Some(h), Tool::Dimension) = (hover, tool) {
                painter.text(h + egui::vec2(14., 14.), egui::Align2::LEFT_TOP, "linha, círculo, 2 pontos (Ctrl: horiz., Alt: vert.) ou 2 linhas", egui::FontId::proportional(11.), FREE);
            }
        }
    }

    // ---------- edição do valor da cota
    let mut close_dim = false;
    if let Some((cid, text, _new)) = &mut edit.dim_edit {
        let pos = edit.hits.iter().find(|(_, e)| *e == Ent::Cs(*cid)).map(|(r, _)| r.center_bottom()).or(hover).unwrap_or(rect.center());
        egui::Area::new(egui::Id::unique("cota_edit")).fixed_pos(pos + egui::vec2(-60., 4.)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(RichText::new("Valor (aceita expressão)").small());
                let r = ui.add(egui::TextEdit::singleline(text).desired_width(120.));
                r.request_focus();
                let ok = scc_core::expr::eval_str(text, &vars).is_ok();
                if !ok {
                    ui.label(RichText::new("expressão inválida").small().color(BAD));
                }
                let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
                if enter && ok {
                    if let Some(v) = sk.constraints.iter_mut().find(|x| x.id == *cid).and_then(|x| x.kind.value_mut()) {
                        *v = text.trim().to_string();
                    }
                    close_dim = true;
                }
                if esc {
                    close_dim = true;
                }
            });
        });
    }
    if close_dim {
        edit.dim_edit = None;
        edit.report = sk.solve(&vars, &[]);
    }

    // ---------- status
    let st = &edit.report;
    let (txt, col) = if !st.converged {
        (format!("⚠ Conflito: {} restrição(ões) não satisfeita(s)", st.failing.len()), BAD)
    } else if st.dof == 0 {
        ("✔ Totalmente restrito".to_string(), Color32::from_rgb(120, 210, 130))
    } else {
        (format!("Sub-restrito · {} grau(s) de liberdade", st.dof), FREE)
    };
    painter.text(rect.left_top() + egui::vec2(10., 26.), egui::Align2::LEFT_TOP, format!("✏ Editando sketch · {txt}"), egui::FontId::proportional(13.), col);
    let tool_name = tool.info().2;
    painter.text(rect.left_top() + egui::vec2(10., 44.), egui::Align2::LEFT_TOP, format!("Ferramenta: {tool_name} · Esc cancela · Esc de novo sai do sketch"), egui::FontId::proportional(11.), col.gamma_multiply(0.8));

    // ---------- grava no documento (ids alocados aqui são reservados no documento)
    let _ = &mut al_ids;
    app.open.doc.reserve_ids(next_tmp);
    if sk != original {
        if let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(app.sketch.as_ref().unwrap().feature).map(|f| &mut f.kind) {
            s.data = sk;
        }
    }
    if let Some((m, e)) = toast {
        app.toast(m, e);
    }
}

/// Teclas do modo sketch. Devolve true se o sketch deve ser fechado.
pub fn keys(app: &mut App, ctx: &egui::Context) -> bool {
    if ctx.egui_wants_keyboard_input() {
        return false;
    }
    let Some(edit) = app.sketch.as_mut() else { return false };
    let k = |key| ctx.input(|i| i.key_pressed(key) && !i.modifiers.command);
    if k(egui::Key::Escape) && !edit.cancel() {
        return true;
    }
    for (key, t) in [
        (egui::Key::L, Tool::Line),
        (egui::Key::R, Tool::Rect),
        (egui::Key::C, Tool::Circle),
        (egui::Key::A, Tool::Arc3),
        (egui::Key::P, Tool::Point),
        (egui::Key::D, Tool::Dimension),
        (egui::Key::S, Tool::Select),
    ] {
        if k(key) {
            edit.set_tool(t);
        }
    }
    if k(egui::Key::X) {
        edit.construction = !edit.construction;
    }
    if k(egui::Key::G) {
        app.settings.grid = !app.settings.grid;
    }
    if k(egui::Key::Delete) {
        let sel = std::mem::take(&mut app.sketch.as_mut().unwrap().sel);
        let id = app.sketch.as_ref().unwrap().feature;
        let vars = app.vars();
        if let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
            delete_selection(&mut s.data, &sel);
            let rep = s.data.solve(&vars, &[]);
            app.sketch.as_mut().unwrap().report = rep;
        }
    }
    false
}

/// Painel lateral do sketch em edição: limpo, mostra só o necessário para o passo atual.
pub fn panel(app: &mut App, ui: &mut egui::Ui) {
    let vars = app.vars();
    let Some(edit) = app.sketch.as_mut() else { return };
    let id = edit.feature;
    let name = app.open.doc.feature(id).map(|f| f.name.clone()).unwrap_or_default();
    ui.heading(format!("✏ {name}"));
    let st = &edit.report;
    let (txt, col) = if !st.converged {
        ("Conflito entre restrições".to_string(), BAD)
    } else if st.dof == 0 {
        ("Totalmente restrito".to_string(), Color32::from_rgb(120, 210, 130))
    } else {
        (format!("Sub-restrito · {} grau(s) de liberdade", st.dof), FREE)
    };
    ui.label(RichText::new(txt).color(col).strong());
    let tool = edit.tool;
    ui.label(RichText::new(tool.info().3).weak().small());
    if tool == Tool::Polygon {
        ui.add(egui::Slider::new(&mut edit.polygon_sides, 3..=24).text("lados"));
    }
    ui.add_space(4.);
    let mut finish = false;
    if ui.add(egui::Button::new(RichText::new("✔ Concluir sketch").strong()).min_size(egui::vec2(180., 30.))).clicked() {
        finish = true;
    }
    ui.separator();
    let sel = edit.sel.clone();
    let mut del: Option<Id> = None;
    let mut new_sel: Option<Vec<Ent>> = None;
    let failing = edit.report.failing.clone();
    if let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
        let d = &mut s.data;
        if sel.is_empty() {
            ui.label(RichText::new("Clique num elemento para ver suas restrições e cotas.").weak());
        }
        // elementos selecionados e as restrições ligadas a eles
        let sel_ids: Vec<Id> = sel.iter().map(|e| match e {
            Ent::Pt(x) | Ent::Cv(x) | Ent::Cs(x) => *x,
        }).collect();
        for e in &sel {
            match e {
                Ent::Pt(p) => {
                    if let Some(pt) = d.point_mut(*p) {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Ponto").strong());
                            ui.add(egui::DragValue::new(&mut pt.x).speed(0.5).prefix("x "));
                            ui.add(egui::DragValue::new(&mut pt.y).speed(0.5).prefix("y "));
                            ui.checkbox(&mut pt.fixed, "fixo");
                        });
                    }
                }
                Ent::Cv(c) => {
                    if let Some(cv) = d.curves.iter_mut().find(|x| x.id == *c) {
                        let kind = match cv.geom {
                            Curve::Line { .. } => "Linha",
                            Curve::Circle { .. } => "Círculo",
                            Curve::Arc { .. } => "Arco",
                            Curve::Point { .. } => "Ponto",
                        };
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(kind).strong());
                            ui.checkbox(&mut cv.construction, "construção");
                        });
                    }
                }
                Ent::Cs(_) => {}
            }
        }
        let related: Vec<usize> = (0..d.constraints.len())
            .filter(|i| {
                let c = &d.constraints[*i];
                sel_ids.contains(&c.id) || c.kind.refs().iter().any(|r| sel_ids.contains(r))
            })
            .collect();
        if !related.is_empty() {
            ui.add_space(4.);
            ui.label(RichText::new(format!("Restrições deste elemento ({})", related.len())).weak());
        }
        for i in related {
            let c = &mut d.constraints[i];
            let bad = failing.contains(&c.id);
            ui.horizontal(|ui| {
                let g = glyph(&c.kind);
                let label = format!("{} {}", if g.is_empty() { "↔" } else { g }, c.kind.label());
                ui.label(if bad { RichText::new(label).color(BAD) } else { RichText::new(label) });
                if let Some(v) = c.kind.value_mut() {
                    let ok = scc_core::expr::eval_str(v, &vars).is_ok();
                    let mut te = egui::TextEdit::singleline(v).desired_width(80.);
                    if !ok {
                        te = te.text_color(BAD);
                    }
                    ui.add(te);
                }
                if ui.small_button("✖").on_hover_text("remover restrição").clicked() {
                    del = Some(c.id);
                }
            });
        }
        // avançado: tudo, recolhido
        ui.add_space(8.);
        egui::CollapsingHeader::new(format!("Avançado · todas as restrições ({})", d.constraints.len())).default_open(false).show(ui, |ui| {
            for c in &mut d.constraints {
                let bad = failing.contains(&c.id);
                ui.horizontal(|ui| {
                    let label = if bad { RichText::new(c.kind.label()).color(BAD) } else { RichText::new(c.kind.label()) };
                    if ui.selectable_label(sel.contains(&Ent::Cs(c.id)), label).clicked() {
                        new_sel = Some(vec![Ent::Cs(c.id)]);
                    }
                    if let Some(v) = c.kind.value_mut() {
                        ui.add(egui::TextEdit::singleline(v).desired_width(80.));
                    }
                    if ui.small_button("✖").clicked() {
                        del = Some(c.id);
                    }
                });
            }
        });
        if let Some(x) = del {
            d.constraints.retain(|c| c.id != x);
        }
        let rep = d.solve(&vars, &[]);
        edit_report_set(app, rep);
    }
    if let Some(ns) = new_sel {
        if let Some(e) = app.sketch.as_mut() {
            e.sel = ns;
        }
    }
    if finish {
        app.finish_sketch();
    }
}

fn edit_report_set(app: &mut App, rep: SolveReport) {
    if let Some(e) = app.sketch.as_mut() {
        e.report = rep;
    }
}

/// Entra no modo sketch.
pub fn begin(app: &mut App, id: Id) {
    let prev = app.settings.ortho;
    let mut e = SketchEdit::new(id, prev);
    let vars = app.vars();
    if let Some(FeatureKind::Sketch(s)) = app.open.doc.feature_mut(id).map(|f| &mut f.kind) {
        e.report = s.data.clone().solve(&vars, &[]);
    }
    app.sketch = Some(e);
    app.sel = Sel::Feature(id);
}
