//! Reconhecimento geométrico do que foi clicado, para um campo aceitar o que faz sentido em vez
//! de só o tipo exato: aresta circular → centro e eixo; face plana → normal; face cilíndrica ou
//! cônica → eixo.

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> Option<V3> {
    let l = len(a);
    (l > 1e-12).then(|| [a[0] / l, a[1] / l, a[2] / l])
}

/// Círculo (ou arco) que passa pela polilinha: (centro, normal, raio). `None` se não for circular.
pub fn circle(p: &[V3]) -> Option<(V3, V3, f64)> {
    let n = p.len();
    if n < 4 {
        return None;
    }
    let (a, b, c) = (p[0], p[n / 3], p[2 * n / 3]);
    let (ab, ac) = (sub(b, a), sub(c, a));
    let nn = cross(ab, ac);
    let l2 = dot(nn, nn);
    if l2 < 1e-18 {
        return None;
    }
    // centro do círculo pelos três pontos
    let t1 = cross(nn, ab);
    let t2 = cross(ac, nn);
    let k = 0.5 / l2;
    let ctr = [0, 1, 2].map(|i| a[i] + k * (dot(ac, ac) * t1[i] + dot(ab, ab) * t2[i]));
    let r = len(sub(a, ctr));
    let nrm = unit(nn)?;
    let tol = 1e-3 * r + 1e-4;
    let ok = p.iter().all(|q| (len(sub(*q, ctr)) - r).abs() < tol && dot(sub(*q, ctr), nrm).abs() < tol);
    ok.then_some((ctr, nrm, r))
}

/// Forma reconhecida de uma face pela malha.
pub enum FaceKind {
    /// plana: normal
    Plane(V3),
    /// de revolução (cilindro, cone): ponto do eixo e direção
    Axis(V3, V3),
    Other,
}

/// Analisa os triângulos `idx` da malha (posições e normais por vértice).
pub fn face(pos: &[f32], nor: &[f32], idx: &[u32]) -> FaceKind {
    let mut vs: Vec<u32> = idx.to_vec();
    vs.sort_unstable();
    vs.dedup();
    if vs.len() < 3 || nor.len() < pos.len() {
        return FaceKind::Other;
    }
    let g = |a: &[f32], i: u32| [a[i as usize * 3] as f64, a[i as usize * 3 + 1] as f64, a[i as usize * 3 + 2] as f64];
    let pts: Vec<V3> = vs.iter().map(|i| g(pos, *i)).collect();
    let ns: Vec<V3> = vs.iter().filter_map(|i| unit(g(nor, *i))).collect();
    if ns.len() != pts.len() {
        return FaceKind::Other;
    }
    let mut m = [[0.; 3]; 3];
    for n in &ns {
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += n[i] * n[j];
            }
        }
    }
    let (vals, vecs) = jacobi(m);
    let mut o = [0, 1, 2];
    o.sort_by(|a, b| vals[*a].total_cmp(&vals[*b]));
    let col = |k: usize| [vecs[0][k], vecs[1][k], vecs[2][k]];
    let tot = vals.iter().sum::<f64>().max(1e-12);
    let mean = {
        let s = pts.iter().fold([0.; 3], |a, p| [a[0] + p[0], a[1] + p[1], a[2] + p[2]]);
        s.map(|x| x / pts.len() as f64)
    };
    // normais todas iguais: plana
    if vals[o[1]] / tot < 1e-4 {
        let mut n = col(o[2]);
        if dot(n, ns[0]) < 0. {
            n = n.map(|x| -x);
        }
        return FaceKind::Plane(n);
    }
    // normais num plano (perpendiculares a um eixo): cilindro; inclinadas por igual: cone
    let ax = col(o[0]);
    if vals[o[0]] / tot < 0.05 || {
        let c0 = dot(ns[0], ax);
        ns.iter().all(|n| (dot(*n, ax) - c0).abs() < 0.02)
    } {
        // ponto do eixo: mais próximo, em mínimos quadrados, das retas p + t·n (no plano ⊥ eixo)
        let mut a = [[0.; 3]; 3];
        let mut b = [0.; 3];
        for (p, n) in pts.iter().zip(&ns) {
            let n = unit(sub(*n, ax.map(|x| x * dot(*n, ax)))).unwrap_or(*n);
            for i in 0..3 {
                for j in 0..3 {
                    let q = if i == j { 1. } else { 0. } - n[i] * n[j] - ax[i] * ax[j];
                    a[i][j] += q;
                    b[i] += q * p[j];
                }
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += ax[i] * ax[j];
            }
            b[i] += ax[i] * dot(ax, mean);
        }
        if let Some(c) = solve3(a, b) {
            return FaceKind::Axis(c, ax);
        }
    }
    FaceKind::Other
}

fn solve3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<V3> {
    let det = |m: [[f64; 3]; 3]| dot(m[0], cross(m[1], m[2]));
    let d = det(a);
    if d.abs() < 1e-12 {
        return None;
    }
    let mut out = [0.; 3];
    for (k, o) in out.iter_mut().enumerate() {
        let mut m = a;
        for i in 0..3 {
            m[i][k] = b[i];
        }
        *o = det(m) / d;
    }
    Some(out)
}

/// Autovalores/autovetores de matriz simétrica 3×3 (Jacobi). Vetores nas colunas.
fn jacobi(mut a: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut v = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..50 {
        let (mut p, mut q, mut big) = (0, 1, 0.);
        for i in 0..3 {
            for j in i + 1..3 {
                if a[i][j].abs() > big {
                    big = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if big < 1e-15 {
            break;
        }
        let th = 0.5 * (2. * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (c, s) = (th.cos(), th.sin());
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
        }
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for k in 0..3 {
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    ([a[0][0], a[1][1], a[2][2]], v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circulo_e_cilindro_reconhecidos() {
        let ring: Vec<V3> = (0..24).map(|i| {
            let t = i as f64 / 24. * std::f64::consts::TAU;
            [5. + 10. * t.cos(), 2., 3. + 10. * t.sin()]
        }).collect();
        let (c, n, r) = circle(&ring).unwrap();
        assert!((r - 10.).abs() < 1e-6 && len(sub(c, [5., 2., 3.])) < 1e-6 && n[1].abs() > 0.999);
        assert!(circle(&[[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]]).is_none());
        // cilindro de raio 4 ao longo de Z, centrado em (1, 1)
        let (mut pos, mut nor, mut idx) = (vec![], vec![], vec![]);
        for i in 0..16 {
            let t = i as f64 / 16. * std::f64::consts::TAU;
            for z in [0., 10.] {
                pos.extend([(1. + 4. * t.cos()) as f32, (1. + 4. * t.sin()) as f32, z as f32]);
                nor.extend([t.cos() as f32, t.sin() as f32, 0.]);
            }
        }
        for i in 0..16u32 {
            let (a, b) = (i * 2, ((i + 1) % 16) * 2);
            idx.extend([a, b, a + 1, b, b + 1, a + 1]);
        }
        match face(&pos, &nor, &idx) {
            FaceKind::Axis(c, d) => {
                assert!(d[2].abs() > 0.999, "{d:?}");
                assert!((c[0] - 1.).abs() < 1e-3 && (c[1] - 1.).abs() < 1e-3, "{c:?}");
            }
            _ => panic!("cilindro não reconhecido"),
        }
    }
}
