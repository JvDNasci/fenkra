//! Avaliação de sketches: resolve as restrições com os parâmetros e extrai perfis fechados,
//! pontos (centros de furo), linhas (eixos) e polilinhas para exibição.

use scc_core::sketch::{Curve, Seg, SolveReport};
use scc_core::{expr, Id, SketchData};
use scc_kernel::{Frame, Loop2, Seg2};
use std::collections::HashMap;
use std::f64::consts::PI;

pub type Vars = HashMap<String, f64>;

pub fn ev(src: &str, vars: &Vars, field: &str) -> Result<f64, String> {
    expr::eval_str(src, vars).map_err(|e| format!("campo '{field}' = '{src}': {e}"))
}

/// Sketch avaliado em coordenadas do plano.
#[derive(Default)]
pub struct EvalSketch {
    pub loops: Vec<Loop2>,
    pub points: Vec<[f64; 2]>,
    /// linhas por id da curva (para eixos de revolução)
    pub lines: HashMap<Id, ([f64; 2], [f64; 2])>,
    /// polilinhas para exibir: (é construção, pontos)
    pub viz: Vec<(bool, Vec<[f64; 2]>)>,
    pub warnings: Vec<String>,
    pub report: SolveReport,
}

fn seg(s: Seg) -> Seg2 {
    match s {
        Seg::Line(a, b) => Seg2::Line(a, b),
        Seg::Arc(a, m, b) => Seg2::Arc(a, m, b),
        Seg::Circle(c, r) => Seg2::Circle(c, r),
    }
}

pub fn eval_sketch(data: &SketchData, vars: &Vars) -> Result<EvalSketch, String> {
    let mut sk = data.clone();
    let report = sk.solve(vars, &[]);
    let mut out = EvalSketch::default();
    if !report.errors.is_empty() {
        return Err(report.errors.join("; "));
    }
    if !report.converged {
        out.warnings.push(format!("restrições em conflito ({} não satisfeitas)", report.failing.len()));
    }
    let (loops, open) = sk.loops();
    out.loops = loops.into_iter().map(|l| l.into_iter().map(seg).collect()).collect();
    if open > 0 {
        out.warnings.push(format!("{open} curva(s) não formam perfil fechado"));
    }
    out.points = sk.hole_points();
    for c in &sk.curves {
        let pts = match c.geom {
            Curve::Line { a, b } => {
                out.lines.insert(c.id, (sk.pos(a), sk.pos(b)));
                vec![sk.pos(a), sk.pos(b)]
            }
            Curve::Circle { c: ctr, r } => sample(&[Seg2::Circle(sk.pos(ctr), r)]),
            Curve::Arc { c: ctr, s, e } => {
                let (pc, ps, pe) = (sk.pos(ctr), sk.pos(s), sk.pos(e));
                sample(&[Seg2::Arc(ps, SketchData::arc_mid(pc, ps, pe), pe)])
            }
            Curve::Point { .. } => continue,
        };
        out.viz.push((c.construction, pts));
    }
    out.report = report;
    Ok(out)
}

/// Amostra o laço em pontos para desenhar.
pub fn sample(l: &[Seg2]) -> Vec<[f64; 2]> {
    let mut pts = vec![];
    for s in l {
        match *s {
            Seg2::Line(a, b) => {
                if pts.is_empty() {
                    pts.push(a)
                }
                pts.push(b)
            }
            Seg2::Circle(c, r) => {
                pts.extend((0..=64).map(|i| {
                    let a = 2. * PI * i as f64 / 64.;
                    [c[0] + r * a.cos(), c[1] + r * a.sin()]
                }));
            }
            Seg2::Arc(a, m, b) => {
                if let Some((c, r, a0, sweep)) = arc_params(a, m, b) {
                    if pts.is_empty() {
                        pts.push(a)
                    }
                    pts.extend((1..=24).map(|i| {
                        let t = a0 + sweep * i as f64 / 24.;
                        [c[0] + r * t.cos(), c[1] + r * t.sin()]
                    }));
                }
            }
        }
    }
    pts
}

/// Centro, raio, ângulo inicial e varredura (com sinal) de um arco por 3 pontos.
pub fn arc_params(a: [f64; 2], m: [f64; 2], b: [f64; 2]) -> Option<([f64; 2], f64, f64, f64)> {
    let d = 2. * (a[0] * (m[1] - b[1]) + m[0] * (b[1] - a[1]) + b[0] * (a[1] - m[1]));
    if d.abs() < 1e-12 {
        return None;
    }
    let sq = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let cx = (sq(a) * (m[1] - b[1]) + sq(m) * (b[1] - a[1]) + sq(b) * (a[1] - m[1])) / d;
    let cy = (sq(a) * (b[0] - m[0]) + sq(m) * (a[0] - b[0]) + sq(b) * (m[0] - a[0])) / d;
    let ang = |p: [f64; 2]| (p[1] - cy).atan2(p[0] - cx);
    let (t0, tm, t1) = (ang(a), ang(m), ang(b));
    let norm = |x: f64| (x % (2. * PI) + 2. * PI) % (2. * PI);
    let ccw = norm(t1 - t0);
    let sweep = if norm(tm - t0) < ccw { ccw } else { ccw - 2. * PI };
    Some(([cx, cy], ((a[0] - cx).powi(2) + (a[1] - cy).powi(2)).sqrt(), t0, sweep))
}

pub fn to3d(f: &Frame, p: [f64; 2]) -> [f32; 3] {
    f.point(p[0], p[1]).map(|x| x as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scc_core::sketch::Kind;

    #[test]
    fn retangulo_parametrico_gera_perfil() {
        let mut n = 0;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut s = SketchData::default();
        let (p, l) = s.add_rect(&mut al, [0., 0.], [10., 10.], false);
        s.add_constraint(&mut al, Kind::Fixed(p[0]));
        s.add_constraint(&mut al, Kind::Length(l[0], "L".into()));
        s.add_constraint(&mut al, Kind::Length(l[1], "L/2".into()));
        let mut v = Vars::new();
        v.insert("L".into(), 40.);
        let e = eval_sketch(&s, &v).unwrap();
        assert_eq!(e.loops.len(), 1);
        assert!(e.warnings.is_empty());
        assert_eq!(e.report.dof, 0);
    }

    #[test]
    fn arco_meia_volta() {
        let (c, r, _, sw) = arc_params([1., 0.], [0., 1.], [-1., 0.]).unwrap();
        assert!(c[0].abs() < 1e-12 && (r - 1.).abs() < 1e-12 && (sw - PI).abs() < 1e-12);
    }
}
