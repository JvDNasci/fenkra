//! Avaliação de sketches: entidades → laços fechados (perfis), pontos e linhas para exibição.

use scc_core::{expr, Id, SketchGeom, SketchEntity};
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
    /// linhas por id da entidade (para eixos de revolução)
    pub lines: HashMap<Id, ([f64; 2], [f64; 2])>,
    /// polilinhas para exibir: (é construção, pontos)
    pub viz: Vec<(bool, Vec<[f64; 2]>)>,
    pub warnings: Vec<String>,
}

pub fn eval_sketch(entities: &[SketchEntity], vars: &Vars) -> Result<EvalSketch, String> {
    let mut out = EvalSketch::default();
    let mut open_lines: Vec<([f64; 2], [f64; 2])> = vec![];
    for e in entities {
        let f = e.geom.fields();
        let names: Vec<&str> = {
            let mut g = e.geom.clone();
            g.fields_mut().into_iter().map(|(n, _)| n).collect()
        };
        let v: Vec<f64> = f
            .iter()
            .zip(&names)
            .map(|(s, n)| ev(s, vars, &format!("{} · {n}", e.geom.label())))
            .collect::<Result<_, _>>()?;
        let lp: Option<Loop2> = match &e.geom {
            SketchGeom::Line { .. } => {
                let (a, b) = ([v[0], v[1]], [v[2], v[3]]);
                out.lines.insert(e.id, (a, b));
                out.viz.push((e.construction, vec![a, b]));
                if !e.construction {
                    open_lines.push((a, b));
                }
                None
            }
            SketchGeom::Point { .. } => {
                out.points.push([v[0], v[1]]);
                None
            }
            SketchGeom::Rect { .. } => {
                let (cx, cy, w, h) = (v[0], v[1], v[2].abs() / 2., v[3].abs() / 2.);
                if w <= 0. || h <= 0. {
                    return Err("retângulo com largura ou altura zero".into());
                }
                let p = [[cx - w, cy - h], [cx + w, cy - h], [cx + w, cy + h], [cx - w, cy + h]];
                Some((0..4).map(|i| Seg2::Line(p[i], p[(i + 1) % 4])).collect())
            }
            SketchGeom::Circle { .. } => {
                if v[2] <= 0. {
                    return Err("círculo com raio menor ou igual a zero".into());
                }
                Some(vec![Seg2::Circle([v[0], v[1]], v[2])])
            }
            SketchGeom::Polygon { .. } => {
                let n = v[3].round() as usize;
                if n < 3 || v[2] <= 0. {
                    return Err("polígono precisa de 3 ou mais lados e raio positivo".into());
                }
                let p: Vec<[f64; 2]> = (0..n)
                    .map(|i| {
                        let a = PI / 2. + 2. * PI * i as f64 / n as f64;
                        [v[0] + v[2] * a.cos(), v[1] + v[2] * a.sin()]
                    })
                    .collect();
                Some((0..n).map(|i| Seg2::Line(p[i], p[(i + 1) % n])).collect())
            }
            SketchGeom::Slot { .. } => {
                let (cx, cy, l, r, ang) = (v[0], v[1], v[2] / 2., v[3] / 2., v[4].to_radians());
                if l <= 0. || r <= 0. {
                    return Err("rasgo com comprimento ou largura inválidos".into());
                }
                let (c, s) = (ang.cos(), ang.sin());
                let at = |u: f64, w: f64| [cx + u * c - w * s, cy + u * s + w * c];
                Some(vec![
                    Seg2::Line(at(-l, -r), at(l, -r)),
                    Seg2::Arc(at(l, -r), at(l + r, 0.), at(l, r)),
                    Seg2::Line(at(l, r), at(-l, r)),
                    Seg2::Arc(at(-l, r), at(-l - r, 0.), at(-l, -r)),
                ])
            }
        };
        if let Some(l) = lp {
            if !e.construction {
                out.viz.push((false, sample(&l)));
                out.loops.push(l);
            } else {
                out.viz.push((true, sample(&l)));
            }
        }
    }
    // encadeia linhas soltas em laços fechados
    let (loops, open) = chain(open_lines);
    out.loops.extend(loops);
    if open > 0 {
        out.warnings.push(format!("{open} linha(s) não formam perfil fechado"));
    }
    Ok(out)
}

fn chain(mut lines: Vec<([f64; 2], [f64; 2])>) -> (Vec<Loop2>, usize) {
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6;
    let mut loops = vec![];
    let mut open = 0;
    while let Some((a, b)) = lines.pop() {
        let mut lp = vec![Seg2::Line(a, b)];
        let mut end = b;
        let mut closed = near(a, b);
        while !closed {
            let Some(i) = lines.iter().position(|(p, q)| near(*p, end) || near(*q, end)) else { break };
            let (p, q) = lines.swap_remove(i);
            let (p, q) = if near(p, end) { (p, q) } else { (q, p) };
            lp.push(Seg2::Line(p, q));
            end = q;
            closed = near(end, a);
        }
        if closed && lp.len() >= 3 {
            loops.push(lp);
        } else {
            open += lp.len();
        }
    }
    (loops, open)
}

/// Amostra o laço em pontos para desenhar.
pub fn sample(l: &Loop2) -> Vec<[f64; 2]> {
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
fn arc_params(a: [f64; 2], m: [f64; 2], b: [f64; 2]) -> Option<([f64; 2], f64, f64, f64)> {
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
    use scc_core::{Id, SketchEntity};

    fn line(id: u64, a: (f64, f64), b: (f64, f64)) -> SketchEntity {
        SketchEntity {
            id: Id(id),
            construction: false,
            geom: SketchGeom::Line { x1: a.0.to_string(), y1: a.1.to_string(), x2: b.0.to_string(), y2: b.1.to_string() },
        }
    }

    #[test]
    fn linhas_formam_laco_e_aberta_gera_aviso() {
        let e = vec![line(1, (0., 0.), (10., 0.)), line(2, (10., 10.), (10., 0.)), line(3, (10., 10.), (0., 0.)), line(4, (50., 0.), (60., 0.))];
        let s = eval_sketch(&e, &Vars::new()).unwrap();
        assert_eq!(s.loops.len(), 1);
        assert_eq!(s.loops[0].len(), 3);
        assert_eq!(s.warnings.len(), 1);
    }

    #[test]
    fn expressoes_do_sketch_usam_parametros() {
        let mut v = Vars::new();
        v.insert("L".into(), 40.);
        let e = vec![SketchEntity {
            id: Id(1),
            construction: false,
            geom: SketchGeom::Rect { cx: "0".into(), cy: "0".into(), w: "L".into(), h: "L/2".into() },
        }];
        let s = eval_sketch(&e, &v).unwrap();
        assert_eq!(s.loops[0][1], Seg2::Line([20., -10.], [20., 10.]));
    }

    #[test]
    fn arco_meia_volta() {
        let (c, r, _, sw) = arc_params([1., 0.], [0., 1.], [-1., 0.]).unwrap();
        assert!(c[0].abs() < 1e-12 && (r - 1.).abs() < 1e-12 && (sw - PI).abs() < 1e-12);
    }
}
