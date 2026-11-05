// SPDX-License-Identifier: LGPL-2.1-or-later
//! Apoio do filete e do chanfro (B289): trecho parcial de uma aresta e o diagnóstico de quando o
//! valor pedido não cabe na peça.
//!
//! O kernel só diz "raios grandes demais". Aqui o motivo ganha nome: mede a face vizinha mais
//! estreita ao lado das arestas escolhidas e procura, por bisseção, o maior valor que o kernel
//! aceita para a mesma seleção. A mensagem sai com os dois números, para a pessoa corrigir sem
//! tentativa e erro.

use super::*;

/// Maior valor (até `v`, que já falhou) que o kernel aceita: bisseção em 8 passos.
/// `None` quando nem um valor bem pequeno funciona (seleção ruim, não medida).
pub(crate) fn max_that_fits(v: f64, ok: impl Fn(f64) -> bool) -> Option<f64> {
    if !(v > 0.) {
        return None;
    }
    let mut hi = v;
    if !ok(v / 256.) {
        return None;
    }
    let mut lo = v / 256.;
    for _ in 0..8 {
        let mid = (lo + hi) / 2.;
        if ok(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo)
}

/// Pedaço de uma polilinha entre as distâncias `s0` e `s1` (mm contados do primeiro ponto).
/// Distâncias fora da polilinha são presas aos extremos; devolve vazio se sobrar menos que um ponto.
pub(crate) fn sub_polyline(pts: &[V3], s0: f64, s1: f64) -> Vec<V3> {
    let (a, c) = if s0 <= s1 { (s0, s1) } else { (s1, s0) };
    let mut out: Vec<V3> = vec![];
    let mut acc = 0.;
    let lerp = |p: V3, q: V3, t: f64| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, p[2] + (q[2] - p[2]) * t];
    for w in pts.windows(2) {
        let len = geo3d::dist(w[0], w[1]);
        let (lo, hi) = (acc, acc + len);
        acc = hi;
        if len < 1e-12 || hi <= a || lo >= c {
            continue;
        }
        let p = if lo < a { lerp(w[0], w[1], (a - lo) / len) } else { w[0] };
        let q = if hi > c { lerp(w[0], w[1], (c - lo) / len) } else { w[1] };
        if out.is_empty() {
            out.push(p);
        }
        out.push(q);
    }
    if out.len() < 2 {
        vec![]
    } else {
        out
    }
}

/// Texto curto de um número: sem zeros à direita.
fn n2(x: f64) -> String {
    format!("{}", (x * 100.).round() / 100.)
}

impl Ctx<'_> {
    /// Mensagem de erro do filete/chanfro que não coube. `v` é a maior medida pedida e `fits` o maior
    /// valor que o kernel aceitou para a mesma seleção.
    pub(crate) fn explain_edge_failure(&self, b: &Shape, picked: Option<&[[f64; 3]]>, chamfer: bool, v: f64, fits: Option<f64>, kernel_msg: &str) -> String {
        let (nome, med) = if chamfer { ("chanfro", "distância") } else { ("filete", "raio") };
        let mut out = format!("{nome} não cabe: {med} {} é grande demais para esta peça", n2(v));
        if let Some(w) = picked.and_then(|p| self.narrowest_neighbor(b, p)) {
            if v >= w * 0.98 {
                out += &format!(" — a face vizinha tem só ≈{} de largura, menos que o {med}", n2(w));
            } else {
                out += " — a superfície do " ;
                out += nome;
                out += " invadiria outra aresta ou a face oposta";
            }
        }
        match fits {
            Some(x) => out += &format!("; o maior {med} que cabe nesta seleção é ≈{}", n2(x)),
            None => out += "; nem um valor pequeno funciona, confira a seleção das arestas",
        }
        if !kernel_msg.is_empty() {
            out += &format!(" ({kernel_msg})");
        }
        out
    }

    /// Menor largura (área ÷ comprimento da aresta escolhida) entre as faces vizinhas das arestas.
    fn narrowest_neighbor(&self, b: &Shape, picked: &[[f64; 3]]) -> Option<f64> {
        let topo = b.topology().ok()?;
        let m = tesselar(b, &self.qualidade).ok()?;
        let nseg = m.edges.len() / 6;
        let mut best: Option<f64> = None;
        for q in picked {
            let nearest = |mids: &mut dyn Iterator<Item = [f64; 3]>| mids.enumerate().map(|(i, p)| (i, geo3d::dist(p, *q))).min_by(|a, c| a.1.total_cmp(&c.1)).map(|x| x.0);
            let te = nearest(&mut topo.edge_mids.iter().copied())?;
            let me = nearest(&mut (0..m.edge_mids.len() / 3).map(|e| [m.edge_mids[e * 3] as f64, m.edge_mids[e * 3 + 1] as f64, m.edge_mids[e * 3 + 2] as f64]))?;
            let s0 = *m.edge_ranges.get(me)? as usize;
            let s1 = m.edge_ranges.get(me + 1).map(|x| *x as usize).unwrap_or(nseg);
            let len: f64 = m
                .edges
                .chunks_exact(6)
                .take(s1)
                .skip(s0)
                .map(|g| geo3d::dist([g[0] as f64, g[1] as f64, g[2] as f64], [g[3] as f64, g[4] as f64, g[5] as f64]))
                .sum();
            if len < 1e-9 {
                continue;
            }
            for fc in topo.edge_faces.get(te)? {
                if let Some(fi) = topo.faces.get(*fc as usize) {
                    let w = fi.area / len;
                    best = Some(best.map_or(w, |x: f64| x.min(w)));
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trecho_de_polilinha() {
        let l = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]];
        let t = sub_polyline(&l, 4., 14.);
        assert_eq!(t.len(), 3);
        assert!((t[0][0] - 4.).abs() < 1e-9);
        assert!((t[1][0] - 10.).abs() < 1e-9 && t[1][1].abs() < 1e-9);
        assert!((t[2][1] - 4.).abs() < 1e-9);
        // fora da aresta: preso ao fim
        let t = sub_polyline(&l, 15., 99.);
        assert_eq!(t.len(), 2);
        assert!((t[0][1] - 5.).abs() < 1e-9 && (t[1][1] - 10.).abs() < 1e-9);
        assert!(sub_polyline(&l, 30., 40.).is_empty());
    }

    #[test]
    fn bisseccao_acha_o_limite() {
        let x = max_that_fits(10., |v| v <= 6.3).unwrap();
        assert!((x - 6.3).abs() < 0.1, "{x}");
        assert!(max_that_fits(10., |_| false).is_none());
    }
}
