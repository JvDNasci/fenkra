// SPDX-License-Identifier: LGPL-2.1-or-later
//! Chamadas geradas a partir das features (B186): furo, rosca, rebaixo, escareado, chanfro, filete,
//! espessura e dobra de chapa. O texto sai dos parâmetros da própria feature, sem digitar nada:
//! `4× Ø6,5 ▽12 · M8×1,25-6H`, `C2`, `R3`, `esp. 2`. É um gerador puro, também usado pelo desenho
//! (B107) para que a chamada do 3D e a da folha nunca divirjam.
//!
//! A contagem `n×` vem do padrão: quantos centros o furo tem vezes quantas instâncias os padrões
//! que o repetem produzem. Se a geometria deixa de ter essa quantidade (um furo saiu do padrão), a
//! chamada fica `Ambígua`, como as cotas cujo alvo ficou em dúvida.

use crate::doc::{Document, EdgeMode, EdgeSel, Extent, FeatureKind, HoleKind, PatternKind};
use crate::expr;
use crate::pmi::{num, PmiState};
use crate::sketch::Curve;
use crate::Id;
use std::collections::HashMap;

/// Uma parte da chamada composta, que também pode virar cota simples com tolerância própria.
#[derive(Clone, PartialEq, Debug)]
pub struct SimpleDim {
    /// "contagem", "diâmetro", "profundidade", "rosca", "rebaixo", "escareado", "raio", "distância",
    /// "ângulo", "espessura"
    pub label: &'static str,
    pub text: String,
    /// valor numérico em mm (ou graus) quando a parte é uma medida que aceita tolerância
    pub value: Option<f64>,
}

/// Resultado do gerador para uma feature.
#[derive(Clone, PartialEq, Debug)]
pub struct Callout {
    /// chamada composta
    pub text: String,
    /// as partes da mesma chamada, uma cota simples cada (forma que o AP242 exige)
    pub parts: Vec<SimpleDim>,
    /// quantas repetições a chamada declara (`n×`); `None` quando a feature não repete
    pub count: Option<usize>,
    /// diâmetro do furo, para conferir a contagem na geometria
    pub hole_diameter: Option<f64>,
}

/// Designação da rosca para exibição: "M8x1.25" → "M8×1,25-6H" (furo) ou "-6g" (eixo). Quando a
/// designação já traz a classe ("M8x1.25-6H") ela é mantida.
pub fn thread_text(spec: &str, internal: bool, left_hand: bool) -> String {
    let s = spec.trim().replace(['x', 'X'], "×").replace('.', ",");
    let s = if s.contains('-') { s } else { format!("{s}-{}", if internal { "6H" } else { "6g" }) };
    if left_hand { format!("{s} LH") } else { s }
}

/// Chanfro: distância igual a 45° vira "C2"; duas distâncias, "2×3"; distância e ângulo, "2×30°".
pub fn chamfer_text(d: f64, mode: &ChamferMode) -> String {
    match mode {
        ChamferMode::Equal => format!("C{}", num(d)),
        ChamferMode::TwoDist(d2) => format!("{}×{}", num(d), num(*d2)),
        ChamferMode::DistAngle(a) => format!("{}×{}°", num(d), num(*a)),
    }
}

/// Forma longa do chanfro simétrico: "2×45°".
pub fn chamfer_long(d: f64) -> String {
    format!("{}×45°", num(d))
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ChamferMode {
    Equal,
    TwoDist(f64),
    DistAngle(f64),
}

/// Raio de filete: "R3"; variável, "R3→5"; com várias arestas, "4× R3".
pub fn fillet_text(r: f64, r2: Option<f64>, n: usize) -> String {
    let base = match r2 {
        Some(r2) => format!("R{}→{}", num(r), num(r2)),
        None => format!("R{}", num(r)),
    };
    if n > 1 { format!("{n}× {base}") } else { base }
}

/// Espessura da chapa: "esp. 2".
pub fn thickness_text(t: f64) -> String {
    format!("esp. {}", num(t))
}

/// Dobra: "R2 · 90°" (raio interno e ângulo).
pub fn bend_text(r: f64, angle: f64) -> String {
    format!("R{} · {}°", num(r), num(angle))
}

/// Dados de um furo já avaliados (mm e graus).
#[derive(Clone, PartialEq, Debug)]
pub struct HoleSpec {
    pub count: usize,
    pub diameter: f64,
    /// profundidade cega; `None` = passante ou até outro limite
    pub depth: Option<f64>,
    pub kind: HoleShape,
    pub thread: Option<String>,
    pub left_hand: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub enum HoleShape {
    Simple,
    /// rebaixo: diâmetro e profundidade
    Counterbore(f64, f64),
    /// escareado: diâmetro da boca e ângulo
    Countersink(f64, f64),
}

/// Texto composto do furo: `4× Ø6,5 ▽12 · M8×1,25-6H`.
pub fn hole_text(h: &HoleSpec) -> String {
    hole_callout(h).text
}

/// Texto e partes do furo.
pub fn hole_callout(h: &HoleSpec) -> Callout {
    let mut parts = vec![];
    let mut s = String::new();
    if h.count > 1 {
        s += &format!("{}× ", h.count);
        parts.push(SimpleDim { label: "contagem", text: format!("{}×", h.count), value: Some(h.count as f64) });
    }
    let dia = format!("Ø{}", num(h.diameter));
    s += &dia;
    parts.push(SimpleDim { label: "diâmetro", text: dia, value: Some(h.diameter) });
    match &h.depth {
        Some(d) => {
            let t = format!("▽{}", num(*d));
            s += &format!(" {t}");
            parts.push(SimpleDim { label: "profundidade", text: t, value: Some(*d) });
        }
        None => {
            if h.thread.is_none() {
                s += " passante";
            }
        }
    }
    match h.kind {
        HoleShape::Simple => {}
        HoleShape::Counterbore(d, p) => {
            let t = format!("⌴ Ø{} ▽{}", num(d), num(p));
            s += &format!(" {t}");
            parts.push(SimpleDim { label: "rebaixo", text: t, value: Some(d) });
        }
        HoleShape::Countersink(d, a) => {
            let t = format!("⌵ Ø{}×{}°", num(d), num(a));
            s += &format!(" {t}");
            parts.push(SimpleDim { label: "escareado", text: t, value: Some(d) });
        }
    }
    if let Some(sp) = &h.thread {
        let t = thread_text(sp, true, h.left_hand);
        s += &format!(" · {t}");
        parts.push(SimpleDim { label: "rosca", text: t, value: None });
    }
    Callout { text: s, parts, count: (h.count > 1).then_some(h.count), hole_diameter: Some(h.diameter) }
}

/// Estado da contagem: o que a chamada declara contra o que a geometria tem.
pub fn count_state(declared: usize, found: usize) -> (PmiState, String) {
    if declared == found {
        (PmiState::Ok, String::new())
    } else {
        (PmiState::Ambigua, format!("a chamada declara {declared}× e a geometria tem {found}"))
    }
}

fn val(vars: &HashMap<String, f64>, src: &str) -> Option<f64> {
    expr::eval_str(src, vars).ok()
}

/// Quantas instâncias os padrões ativos que repetem `id` produzem (1 quando nenhum o repete).
pub fn pattern_multiplier(doc: &Document, vars: &HashMap<String, f64>, id: Id) -> usize {
    let mut m = 1usize;
    for (_, f) in doc.active_features() {
        let FeatureKind::Pattern(p) = &f.kind else { continue };
        if !p.sources.contains(&id) {
            continue;
        }
        let inst = match &p.kind {
            PatternKind::Linear { count, dir2, count2, .. } => {
                let a = val(vars, count).unwrap_or(1.).round().max(1.) as usize;
                let b = if dir2.is_some() { val(vars, count2).unwrap_or(1.).round().max(1.) as usize } else { 1 };
                a * b
            }
            PatternKind::Circular { count, .. } => val(vars, count).unwrap_or(1.).round().max(1.) as usize,
            PatternKind::Points { points, .. } => points.len().max(1),
        };
        m *= inst;
    }
    m
}

/// Centros que o furo tem sozinho: pontos 3D, ou os pontos isolados do sketch dele.
fn hole_centers(doc: &Document, h: &crate::doc::Hole) -> usize {
    if !h.points.is_empty() {
        return h.points.len();
    }
    for (_, f) in doc.active_features() {
        if f.id == h.sketch {
            if let FeatureKind::Sketch(s) = &f.kind {
                let n = s.data.curves.iter().filter(|c| !c.construction && matches!(c.geom, Curve::Point { .. })).count();
                return n.max(1);
            }
        }
    }
    1
}

/// Chamada da feature `id`; `None` para tipos sem chamada ou parâmetros que não avaliam.
/// `internal` diz se a rosca avulsa está num furo (classe 6H) ou num eixo (6g).
pub fn callout(doc: &Document, id: Id, internal: bool) -> Option<Callout> {
    let vars = doc.eval_params().ok()?;
    let f = doc.features.iter().find(|f| f.id == id)?;
    match &f.kind {
        FeatureKind::Hole(h) => {
            let kind = match &h.kind {
                HoleKind::Simples => HoleShape::Simple,
                HoleKind::Rebaixado { diameter, depth } => HoleShape::Counterbore(val(&vars, diameter)?, val(&vars, depth)?),
                HoleKind::Contrafurado { diameter, depth, .. } => HoleShape::Counterbore(val(&vars, diameter)?, val(&vars, depth)?),
                HoleKind::Escareado { diameter, angle } => HoleShape::Countersink(val(&vars, diameter)?, val(&vars, angle)?),
            };
            let depth = match &h.depth {
                Extent::Blind(d) => Some(val(&vars, d)?),
                _ => None,
            };
            let count = hole_centers(doc, h) * pattern_multiplier(doc, &vars, id);
            Some(hole_callout(&HoleSpec { count, diameter: val(&vars, &h.diameter)?, depth, kind, thread: h.thread.clone(), left_hand: h.left_hand }))
        }
        FeatureKind::Thread(t) => {
            let text = thread_text(&t.spec, internal, false);
            let len = val(&vars, &t.length);
            let mut parts = vec![SimpleDim { label: "rosca", text: text.clone(), value: None }];
            let mut s = text;
            if let (Some(l), false) = (len, t.end.is_some()) {
                let tl = format!("▽{}", num(l));
                s += &format!(" {tl}");
                parts.push(SimpleDim { label: "profundidade", text: tl, value: Some(l) });
            }
            Some(Callout { text: s, parts, count: None, hole_diameter: None })
        }
        FeatureKind::Fillet(o) => {
            let r = val(&vars, &o.value)?;
            let r2 = match &o.mode {
                EdgeMode::Variable(v) => Some(val(&vars, v)?),
                _ => None,
            };
            let n = match &o.edges {
                EdgeSel::Edges(e) => e.len(),
                _ => 1,
            };
            let text = fillet_text(r, r2, n);
            Some(Callout { parts: vec![SimpleDim { label: "raio", text: format!("R{}", num(r)), value: Some(r) }], text, count: (n > 1).then_some(n), hole_diameter: None })
        }
        FeatureKind::Chamfer(o) => {
            let d = val(&vars, &o.value)?;
            let mode = match &o.mode {
                EdgeMode::TwoDist(v) => ChamferMode::TwoDist(val(&vars, v)?),
                EdgeMode::DistAngle(v) => ChamferMode::DistAngle(val(&vars, v)?),
                _ => ChamferMode::Equal,
            };
            Some(Callout { text: chamfer_text(d, &mode), parts: vec![SimpleDim { label: "distância", text: num(d), value: Some(d) }], count: None, hole_diameter: None })
        }
        FeatureKind::SmBend(b) => {
            let sheet = doc.sheet.as_ref()?;
            let r = match &b.radius {
                Some(r) => val(&vars, r)?,
                None => val(&vars, &sheet.radius)?,
            };
            let a = val(&vars, &b.angle)?;
            let t = val(&vars, &sheet.thickness)?;
            Some(Callout {
                text: format!("{} · {}", bend_text(r, a), thickness_text(t)),
                parts: vec![
                    SimpleDim { label: "raio", text: format!("R{}", num(r)), value: Some(r) },
                    SimpleDim { label: "ângulo", text: format!("{}°", num(a)), value: Some(a) },
                    SimpleDim { label: "espessura", text: thickness_text(t), value: Some(t) },
                ],
                count: None,
                hole_diameter: None,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{EdgeOp, Hole, Pattern, Thread};
    use crate::sketch::{SkCurve, SkPoint};

    fn furo(count: usize, d: f64, depth: Option<f64>, kind: HoleShape, thread: Option<&str>) -> HoleSpec {
        HoleSpec { count, diameter: d, depth, kind, thread: thread.map(str::to_string), left_hand: false }
    }

    #[test]
    fn furo_roscado_com_contagem() {
        let t = hole_text(&furo(4, 6.5, Some(12.), HoleShape::Simple, Some("M8x1.25")));
        assert_eq!(t, "4× Ø6,5 ▽12 · M8×1,25-6H");
        assert_eq!(hole_text(&furo(1, 9., None, HoleShape::Simple, None)), "Ø9 passante");
        let mut lh = furo(1, 6.8, Some(10.), HoleShape::Simple, Some("M8x1.25"));
        lh.left_hand = true;
        assert_eq!(hole_text(&lh), "Ø6,8 ▽10 · M8×1,25-6H LH");
        // classe própria da designação é respeitada
        assert_eq!(thread_text("M10x1.5-7H", true, false), "M10×1,5-7H");
        assert_eq!(thread_text("M8x1.25", false, false), "M8×1,25-6g");
    }

    #[test]
    fn rebaixo_e_escareado() {
        assert_eq!(hole_text(&furo(2, 6.6, None, HoleShape::Counterbore(11., 6.5), None)), "2× Ø6,6 passante ⌴ Ø11 ▽6,5");
        assert_eq!(hole_text(&furo(1, 5.5, None, HoleShape::Countersink(10.4, 90.), None)), "Ø5,5 passante ⌵ Ø10,4×90°");
    }

    #[test]
    fn partes_sao_cotas_simples() {
        let c = hole_callout(&furo(4, 6.5, Some(12.), HoleShape::Simple, Some("M8x1.25")));
        let rot: Vec<&str> = c.parts.iter().map(|p| p.label).collect();
        assert_eq!(rot, ["contagem", "diâmetro", "profundidade", "rosca"]);
        assert_eq!(c.parts[1].value, Some(6.5));
        assert_eq!(c.parts[3].value, None, "a rosca não tem tolerância numérica");
        assert_eq!(c.count, Some(4));
        assert_eq!(hole_callout(&furo(1, 5., None, HoleShape::Simple, None)).count, None);
    }

    #[test]
    fn chanfro_filete_espessura_dobra() {
        assert_eq!(chamfer_text(2., &ChamferMode::Equal), "C2");
        assert_eq!(chamfer_long(2.), "2×45°");
        assert_eq!(chamfer_text(2., &ChamferMode::TwoDist(3.)), "2×3");
        assert_eq!(chamfer_text(1.5, &ChamferMode::DistAngle(30.)), "1,5×30°");
        assert_eq!(fillet_text(3., None, 1), "R3");
        assert_eq!(fillet_text(3., Some(5.), 1), "R3→5");
        assert_eq!(fillet_text(0.5, None, 4), "4× R0,5");
        assert_eq!(thickness_text(2.), "esp. 2");
        assert_eq!(bend_text(2., 90.), "R2 · 90°");
    }

    #[test]
    fn contagem_ambigua_quando_um_furo_sai_do_padrao() {
        assert_eq!(count_state(4, 4).0, PmiState::Ok);
        let (e, msg) = count_state(4, 3);
        assert_eq!(e, PmiState::Ambigua);
        assert_eq!(msg, "a chamada declara 4× e a geometria tem 3");
    }

    #[test]
    fn chamada_vem_do_documento() {
        let mut d = Document::new("t");
        // sketch com um ponto isolado = um centro de furo
        let mut sk = crate::doc::Sketch::default();
        sk.data.points.push(SkPoint { id: Id(1), x: 0., y: 0., fixed: false });
        sk.data.curves.push(SkCurve { id: Id(2), construction: false, geom: Curve::Point { p: Id(1) } });
        let sid = d.add_feature(FeatureKind::Sketch(sk));
        let h = d.add_feature(FeatureKind::Hole(Hole {
            sketch: sid,
            kind: HoleKind::Simples,
            diameter: "6.5".into(),
            depth: Extent::Blind("12".into()),
            thread: Some("M8x1.25".into()),
            extra: Default::default(),
            thread_modeled: false,
            standard: None,
            points: vec![],
            dir: None,
            reverse: false,
            tip: None,
            taper: "0".into(),
            left_hand: false,
        }));
        let c = callout(&d, h, true).unwrap();
        assert_eq!(c.text, "Ø6,5 ▽12 · M8×1,25-6H");
        assert_eq!(c.count, None);
        // um padrão linear de 4 repete o furo: a contagem vem dele
        d.add_feature(FeatureKind::Pattern(Pattern {
            kind: PatternKind::Linear { dir: crate::doc::AxisRef::Fixed { origin: [0.; 3], dir: [1., 0., 0.] }, count: "4".into(), spacing: "20".into(), dir2: None, count2: "1".into(), spacing2: "0".into(), total: false },
            sources: vec![h],
            skip: Default::default(),
        }));
        let c = callout(&d, h, true).unwrap();
        assert_eq!(c.text, "4× Ø6,5 ▽12 · M8×1,25-6H");
        assert_eq!(c.count, Some(4));
        assert_eq!(c.hole_diameter, Some(6.5));
        // parâmetro nomeado na expressão
        d.add_param("raio", "3", crate::doc::Unit::Comprimento).unwrap();
        let f = d.add_feature(FeatureKind::Fillet(EdgeOp { value: "raio".into(), edges: EdgeSel::Edges(vec![[0.; 3], [1.; 3]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
        assert_eq!(callout(&d, f, false).unwrap().text, "2× R3");
        let ch = d.add_feature(FeatureKind::Chamfer(EdgeOp { value: "2".into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
        assert_eq!(callout(&d, ch, false).unwrap().text, "C2");
        let th = d.add_feature(FeatureKind::Thread(Thread { face: [0.; 3], spec: "M8x1.25".into(), length: "16".into(), modeled: false, end: None, start: String::new() }));
        assert_eq!(callout(&d, th, false).unwrap().text, "M8×1,25-6g ▽16");
        assert_eq!(callout(&d, Id(9999), false), None);
    }
}
