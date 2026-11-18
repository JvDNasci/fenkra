// SPDX-License-Identifier: LGPL-2.1-or-later
//! B509: cálculo das medidas (medir item e medir entre), sem interface.
//!
//! Cada referência ([`RefMedida`]) é achada de novo no resultado do último cálculo pela chave v2
//! (ADR 0010): dono, papel e assinatura da face; para a aresta, as duas faces vizinhas. O ponto
//! clicado só desempata. Daí sai a geometria ([`Geo`]): ponto, reta, círculo, curva, plano,
//! cilindro ou superfície, com a malha da face para as distâncias.
//!
//! Precisão: área vem do kernel (exata). Raio do cilindro sai de um ajuste de círculo nos vértices
//! da malha, que ficam sobre a superfície exata. Distância mínima entre faces curvas é medida entre
//! os triângulos da malha (erro da ordem da flecha da tesselação); plano a plano, eixo a eixo e
//! ângulos usam a fórmula exata da face.

use fenkra_core::{ElemMedida, Medida, ModoMedida, RefMedida};
use fenkra_kernel::{EdgeFormula, Mesh, Topology};
use fenkra_solidos::{refs, RecomputeResult};

pub type V = [f64; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V) -> f64 {
    dot(a, a).sqrt()
}
fn dist(a: V, b: V) -> f64 {
    norm(sub(a, b))
}
fn unit(a: V) -> V {
    let n = norm(a);
    if n < 1e-15 {
        [0., 0., 1.]
    } else {
        mul(a, 1. / n)
    }
}
fn mid(a: V, b: V) -> V {
    mul(add(a, b), 0.5)
}

/// Unidade de um valor medido.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Unidade {
    Mm,
    Mm2,
    Grau,
}

impl Unidade {
    pub fn texto(self) -> &'static str {
        match self {
            Unidade::Mm => "mm",
            Unidade::Mm2 => "mm²",
            Unidade::Grau => "°",
        }
    }
}

/// Valor de uma linha: número com unidade ou ponto (centro).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Valor {
    Num(f64, Unidade),
    Ponto(V),
}

/// Uma linha do resultado. `rotulo` é o texto em português (a interface traduz com `tr`).
#[derive(Clone, PartialEq, Debug)]
pub struct Linha {
    pub rotulo: &'static str,
    pub valor: Valor,
}

/// Resultado de uma medida.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Leitura {
    pub linhas: Vec<Linha>,
    /// os dois pontos de apoio da distância (para a linha de medida no 3D)
    pub apoio: Option<(V, V)>,
    /// onde o rótulo se ancora no 3D
    pub ancora: Option<V>,
    /// referências que não se acharam mais (medida órfã: sem valor)
    pub perdidas: usize,
    /// o que falta para medir (seleção incompleta ou de tipo que não combina)
    pub falta: Option<&'static str>,
}

impl Leitura {
    pub fn orfa(&self) -> bool {
        self.perdidas > 0
    }
    /// Valor numérico da linha de rótulo `r`.
    pub fn valor(&self, r: &str) -> Option<f64> {
        self.linhas.iter().find(|l| l.rotulo == r).and_then(|l| match l.valor {
            Valor::Num(v, _) => Some(v),
            Valor::Ponto(_) => None,
        })
    }
    /// Valor principal (o primeiro número), para o rótulo curto e para virar parâmetro.
    pub fn principal(&self) -> Option<&Linha> {
        self.linhas.iter().find(|l| matches!(l.valor, Valor::Num(..)))
    }
}

/// Geometria resolvida de uma referência.
#[derive(Clone, Debug)]
pub enum Geo {
    Ponto(V),
    Reta { a: V, b: V },
    Circulo { c: V, n: V, r: f64, arco: f64, pts: Vec<V> },
    Curva { pts: Vec<V> },
    Face(Face),
}

#[derive(Clone, Debug)]
pub struct Face {
    pub tipo: TipoFace,
    pub area: f64,
    pub perimetro: f64,
    pub centro: V,
    pub tris: Vec<[V; 3]>,
}

#[derive(Clone, Copy, Debug)]
pub enum TipoFace {
    Plano { p: V, n: V },
    /// `p` sobre o eixo, `d` direção do eixo
    Cilindro { p: V, d: V, r: f64 },
    Outra,
}

// ------------------------------------------------------------------ resolver as referências

fn malha_pos(m: &Mesh, i: u32) -> V {
    let k = i as usize * 3;
    [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
}

fn triangulos(m: &Mesh, face: usize) -> Vec<[V; 3]> {
    let s = m.face_ranges.get(face).copied().unwrap_or(0) as usize;
    let e = m.face_ranges.get(face + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    m.indices.get(s..e).unwrap_or(&[]).chunks_exact(3).map(|t| [malha_pos(m, t[0]), malha_pos(m, t[1]), malha_pos(m, t[2])]).collect()
}

fn topologia(m: &Mesh) -> Topology {
    let edge_mids = m.edge_mids.chunks_exact(3).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]).collect();
    Topology { faces: m.face_info.clone(), edge_mids, edge_faces: m.edge_faces.clone() }
}

/// Malhas onde procurar: corpos (dono e papel de cada face) e superfícies de referência.
struct Fonte<'a> {
    mesh: &'a Mesh,
    owners: std::borrow::Cow<'a, [fenkra_core::Id]>,
    roles: std::borrow::Cow<'a, [i32]>,
    superficie: bool,
}

fn fontes(r: &RecomputeResult) -> Vec<Fonte<'_>> {
    let mut v: Vec<Fonte> = r.bodies.iter().map(|b| Fonte { mesh: &b.mesh, owners: (&b.face_owner[..]).into(), roles: (&b.face_role[..]).into(), superficie: false }).collect();
    for s in &r.surfaces {
        let n = s.mesh.face_info.len();
        v.push(Fonte { mesh: &s.mesh, owners: vec![s.id; n].into(), roles: refs::surface_roles(n).into(), superficie: true });
    }
    v
}

fn face_geo(m: &Mesh, j: usize) -> Option<Geo> {
    let info = m.face_info.get(j)?;
    let tris = triangulos(m, j);
    let mut a_tot = 0.;
    let mut c = [0.; 3];
    for t in &tris {
        let a = norm(cross(sub(t[1], t[0]), sub(t[2], t[0]))) / 2.;
        a_tot += a;
        c = add(c, mul(add(add(t[0], t[1]), t[2]), a / 3.));
    }
    let centro = if a_tot > 0. { mul(c, 1. / a_tot) } else { info.point };
    let perimetro: f64 = (0..m.edge_faces.len()).filter(|e| m.edge_faces[*e].contains(&(j as u32))).map(|e| comprimento(&crate::picking::edge_polyline_mesh(m, e))).sum();
    let tipo = match info.kind {
        0 => TipoFace::Plano { p: info.point, n: unit(info.dir) },
        1 => {
            let d = unit(info.dir);
            let pts: Vec<V> = tris.iter().flat_map(|t| t.iter().copied()).collect();
            match ajustar_circulo(&pts, d) {
                Some((p, r)) => TipoFace::Cilindro { p, d, r },
                None => TipoFace::Outra,
            }
        }
        _ => TipoFace::Outra,
    };
    Some(Geo::Face(Face { tipo, area: info.area.abs(), perimetro, centro, tris }))
}

fn comprimento(pts: &[V]) -> f64 {
    pts.windows(2).map(|w| dist(w[0], w[1])).sum()
}

fn aresta_geo(m: &Mesh, e: usize) -> Option<Geo> {
    let pts = crate::picking::edge_polyline_mesh(m, e);
    if pts.len() < 2 {
        return None;
    }
    match m.edge_formula(e) {
        Some(EdgeFormula::Line { a, b }) => Some(Geo::Reta { a, b }),
        Some(EdgeFormula::Conic { c, u, v, t0, t1 }) if (norm(u) - norm(v)).abs() < 1e-6 * norm(u).max(1.) => {
            let arco = (t1 - t0).abs().to_degrees();
            Some(Geo::Circulo { c, n: unit(cross(u, v)), r: norm(u), arco: if arco > 359.999 { 360. } else { arco }, pts })
        }
        _ => {
            // aresta reta sem fórmula (malha): reta se todos os pontos estão na corda
            let (a, b) = (pts[0], *pts.last().unwrap());
            let d = unit(sub(b, a));
            let reta = dist(a, b) > 1e-9 && pts.iter().all(|p| norm(cross(sub(*p, a), d)) < 1e-4);
            Some(if reta { Geo::Reta { a, b } } else { Geo::Curva { pts } })
        }
    }
}

/// Ajuste de círculo (mínimos quadrados algébricos) dos pontos projetados no plano normal a `d`:
/// devolve um ponto do eixo e o raio.
fn ajustar_circulo(pts: &[V], d: V) -> Option<(V, f64)> {
    if pts.len() < 3 {
        return None;
    }
    let e1 = unit(if d[0].abs() < 0.9 { cross(d, [1., 0., 0.]) } else { cross(d, [0., 1., 0.]) });
    let e2 = cross(d, e1);
    let o = pts[0];
    // x² + y² + D x + E y + F = 0  →  [x y 1]·[D E F] = -(x² + y²)
    let mut ata = [[0f64; 3]; 3];
    let mut atb = [0f64; 3];
    for p in pts {
        let q = sub(*p, o);
        let (x, y) = (dot(q, e1), dot(q, e2));
        let row = [x, y, 1.];
        let rhs = -(x * x + y * y);
        for i in 0..3 {
            for k in 0..3 {
                ata[i][k] += row[i] * row[k];
            }
            atb[i] += row[i] * rhs;
        }
    }
    let s = resolver3(ata, atb)?;
    let (cx, cy) = (-s[0] / 2., -s[1] / 2.);
    let r2 = cx * cx + cy * cy - s[2];
    (r2 > 0.).then(|| (add(o, add(mul(e1, cx), mul(e2, cy))), r2.sqrt()))
}

fn resolver3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let det = |m: [[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let d = det(a);
    if d.abs() < 1e-18 {
        return None;
    }
    let mut x = [0.; 3];
    for (c, xc) in x.iter_mut().enumerate() {
        let mut m = a;
        for r in 0..3 {
            m[r][c] = b[r];
        }
        *xc = det(m) / d;
    }
    Some(x)
}

/// Face mais perto do ponto (sem chave): só se o ponto estiver sobre ela.
fn face_pelo_ponto(fs: &[Fonte], p: V) -> Option<Geo> {
    let mut melhor: Option<(f64, usize, usize)> = None;
    for (k, f) in fs.iter().enumerate() {
        for j in 0..f.mesh.face_info.len() {
            let d = triangulos(f.mesh, j).iter().map(|t| ponto_tri(p, t).0).fold(f64::MAX, f64::min);
            if melhor.is_none_or(|m| d < m.0) {
                melhor = Some((d, k, j));
            }
        }
    }
    let (d, k, j) = melhor?;
    (d < 1e-2).then(|| face_geo(fs[k].mesh, j)).flatten()
}

fn aresta_pelo_ponto(fs: &[Fonte], p: V) -> Option<(usize, usize)> {
    let mut melhor: Option<(f64, usize, usize)> = None;
    for (k, f) in fs.iter().enumerate() {
        for e in 0..f.mesh.edge_ranges.len() {
            let pts = crate::picking::edge_polyline_mesh(f.mesh, e);
            let d = pts.windows(2).map(|w| ponto_seg(p, w[0], w[1]).0).fold(f64::MAX, f64::min);
            if melhor.is_none_or(|m| d < m.0) {
                melhor = Some((d, k, e));
            }
        }
    }
    let (d, k, e) = melhor?;
    (d < 1e-2).then_some((k, e))
}

fn achar_aresta(fs: &[Fonte], rf: &RefMedida) -> Result<(usize, usize), ()> {
    match &rf.aresta {
        Some(key) => {
            let mut ambigua = false;
            for (k, f) in fs.iter().enumerate() {
                if !f.owners.contains(&key.a.owner) && !f.owners.contains(&key.b.owner) {
                    continue;
                }
                let topo = topologia(f.mesh);
                let v = refs::View { topo: &topo, owners: &f.owners, roles: &f.roles };
                let achou = if f.superficie { refs::find_surface_edge(&v, key, Some(rf.ponto)) } else { refs::find_edge(&v, key, Some(rf.ponto)) };
                match achou {
                    Ok(e) => return Ok((k, e)),
                    Err(refs::Lost::Ambiguous(_)) => ambigua = true,
                    Err(refs::Lost::None) => {}
                }
            }
            // ambígua: o ponto clicado decide; sumiu: a medida fica órfã
            if ambigua {
                aresta_pelo_ponto(fs, rf.ponto).ok_or(())
            } else {
                Err(())
            }
        }
        None => aresta_pelo_ponto(fs, rf.ponto).ok_or(()),
    }
}

/// Geometria atual da referência; `None` se ela não existe mais (medida órfã).
pub fn resolver(r: &RecomputeResult, rf: &RefMedida) -> Option<Geo> {
    let fs = fontes(r);
    match rf.elem {
        ElemMedida::Face => match &rf.face {
            Some(key) => {
                let mut ambigua = false;
                for f in &fs {
                    if !f.owners.contains(&key.owner) {
                        continue;
                    }
                    let topo = topologia(f.mesh);
                    let v = refs::View { topo: &topo, owners: &f.owners, roles: &f.roles };
                    let achou = if f.superficie { refs::find_surface_face(&v, key, Some(rf.ponto)) } else { refs::find_face(&v, key, Some(rf.ponto)) };
                    match achou {
                        Ok(j) => return face_geo(f.mesh, j),
                        Err(refs::Lost::Ambiguous(_)) => ambigua = true,
                        Err(refs::Lost::None) => {}
                    }
                }
                if ambigua {
                    face_pelo_ponto(&fs, rf.ponto)
                } else {
                    None
                }
            }
            None => face_pelo_ponto(&fs, rf.ponto),
        },
        ElemMedida::Aresta => {
            let (k, e) = achar_aresta(&fs, rf).ok()?;
            aresta_geo(fs[k].mesh, e)
        }
        ElemMedida::Vertice => {
            if rf.aresta.is_some() {
                // o vértice é a ponta da aresta guardada mais perto do ponto clicado
                let (k, e) = achar_aresta(&fs, rf).ok()?;
                let pts = crate::picking::edge_polyline_mesh(fs[k].mesh, e);
                let (a, b) = (*pts.first()?, *pts.last()?);
                return Some(Geo::Ponto(if dist(a, rf.ponto) <= dist(b, rf.ponto) { a } else { b }));
            }
            // sem chave: vértice da malha no ponto
            fs.iter()
                .flat_map(|f| f.mesh.vertices.chunks_exact(3).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]))
                .map(|q| (dist(q, rf.ponto), q))
                .filter(|(d, _)| *d < 1e-2)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, q)| Geo::Ponto(q))
        }
    }
}

// ------------------------------------------------------------------ medir item

fn centro_de(g: &Geo) -> V {
    match g {
        Geo::Ponto(p) => *p,
        Geo::Reta { a, b } => mid(*a, *b),
        Geo::Circulo { c, .. } => *c,
        Geo::Curva { pts } => pts[pts.len() / 2],
        Geo::Face(f) => f.centro,
    }
}

fn num(l: &mut Vec<Linha>, rotulo: &'static str, v: f64, u: Unidade) {
    l.push(Linha { rotulo, valor: Valor::Num(v, u) });
}

/// Medidas de um elemento: comprimento, raio, diâmetro, área, ângulo, centro, conforme o tipo.
pub fn medir_item(g: &Geo) -> Leitura {
    let mut l = vec![];
    match g {
        Geo::Ponto(p) => l.push(Linha { rotulo: "Posição", valor: Valor::Ponto(*p) }),
        Geo::Reta { a, b } => num(&mut l, "Comprimento", dist(*a, *b), Unidade::Mm),
        Geo::Circulo { r, arco, pts, c, .. } => {
            num(&mut l, "Comprimento", comprimento(pts), Unidade::Mm);
            num(&mut l, "Raio", *r, Unidade::Mm);
            num(&mut l, "Diâmetro", 2. * r, Unidade::Mm);
            if *arco < 360. {
                num(&mut l, "Ângulo do arco", *arco, Unidade::Grau);
            }
            l.push(Linha { rotulo: "Centro", valor: Valor::Ponto(*c) });
        }
        Geo::Curva { pts } => num(&mut l, "Comprimento", comprimento(pts), Unidade::Mm),
        Geo::Face(f) => {
            num(&mut l, "Área", f.area, Unidade::Mm2);
            if f.perimetro > 0. {
                num(&mut l, "Perímetro", f.perimetro, Unidade::Mm);
            }
            if let TipoFace::Cilindro { r, p, d } = f.tipo {
                num(&mut l, "Raio", r, Unidade::Mm);
                num(&mut l, "Diâmetro", 2. * r, Unidade::Mm);
                // centro: o ponto do eixo na altura do centro da face
                let c = add(p, mul(d, dot(sub(f.centro, p), d)));
                l.push(Linha { rotulo: "Centro", valor: Valor::Ponto(c) });
            } else {
                l.push(Linha { rotulo: "Centro", valor: Valor::Ponto(f.centro) });
            }
        }
    }
    Leitura { linhas: l, ancora: Some(centro_de(g)), ..Default::default() }
}

// ------------------------------------------------------------------ medir entre

/// Direção do elemento para o ângulo: (direção, é normal de plano?).
fn direcao(g: &Geo) -> Option<(V, bool)> {
    match g {
        Geo::Reta { a, b } => Some((unit(sub(*b, *a)), false)),
        Geo::Circulo { n, .. } => Some((*n, true)),
        Geo::Face(f) => match f.tipo {
            TipoFace::Plano { n, .. } => Some((n, true)),
            TipoFace::Cilindro { d, .. } => Some((d, false)),
            TipoFace::Outra => None,
        },
        _ => None,
    }
}

/// Eixo (ponto e direção) de cilindro, círculo ou reta.
fn eixo(g: &Geo) -> Option<(V, V)> {
    match g {
        Geo::Circulo { c, n, .. } => Some((*c, *n)),
        Geo::Face(Face { tipo: TipoFace::Cilindro { p, d, .. }, .. }) => Some((*p, *d)),
        _ => None,
    }
}

fn ponto_seg(p: V, a: V, b: V) -> (f64, V) {
    let ab = sub(b, a);
    let t = (dot(sub(p, a), ab) / dot(ab, ab).max(1e-30)).clamp(0., 1.);
    let q = add(a, mul(ab, t));
    (dist(p, q), q)
}

/// Ponto do triângulo mais perto de `p` (Ericson, "Real-Time Collision Detection", 5.1.5).
fn ponto_tri(p: V, t: &[V; 3]) -> (f64, V) {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    let q = if d1 <= 0. && d2 <= 0. {
        a
    } else {
        let bp = sub(p, b);
        let (d3, d4) = (dot(ab, bp), dot(ac, bp));
        let cp = sub(p, c);
        let (d5, d6) = (dot(ab, cp), dot(ac, cp));
        let vc = d1 * d4 - d3 * d2;
        let vb = d5 * d2 - d1 * d6;
        let va = d3 * d6 - d5 * d4;
        if d3 >= 0. && d4 <= d3 {
            b
        } else if vc <= 0. && d1 >= 0. && d3 <= 0. {
            add(a, mul(ab, d1 / (d1 - d3)))
        } else if d6 >= 0. && d5 <= d6 {
            c
        } else if vb <= 0. && d2 >= 0. && d6 <= 0. {
            add(a, mul(ac, d2 / (d2 - d6)))
        } else if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
            add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))))
        } else {
            let den = 1. / (va + vb + vc);
            add(a, add(mul(ab, vb * den), mul(ac, vc * den)))
        }
    };
    (dist(p, q), q)
}

/// Pontos mais próximos entre dois segmentos (Ericson 5.1.9).
fn seg_seg(p1: V, q1: V, p2: V, q2: V) -> (f64, V, V) {
    let (d1, d2, r) = (sub(q1, p1), sub(q2, p2), sub(p1, p2));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (s, t);
    if a <= 1e-30 && e <= 1e-30 {
        return (dist(p1, p2), p1, p2);
    }
    if a <= 1e-30 {
        s = 0.;
        t = (f / e).clamp(0., 1.);
    } else {
        let c = dot(d1, r);
        if e <= 1e-30 {
            t = 0.;
            s = (-c / a).clamp(0., 1.);
        } else {
            let b = dot(d1, d2);
            let den = a * e - b * b;
            let mut s0 = if den > 1e-30 { ((b * f - c * e) / den).clamp(0., 1.) } else { 0. };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0. {
                t0 = 0.;
                s0 = (-c / a).clamp(0., 1.);
            } else if t0 > 1. {
                t0 = 1.;
                s0 = ((b - c) / a).clamp(0., 1.);
            }
            s = s0;
            t = t0;
        }
    }
    let (c1, c2) = (add(p1, mul(d1, s)), add(p2, mul(d2, t)));
    (dist(c1, c2), c1, c2)
}

/// Elemento como peças simples: pontos, segmentos e triângulos.
struct Pecas {
    pontos: Vec<V>,
    segs: Vec<(V, V)>,
    tris: Vec<[V; 3]>,
}

fn pecas(g: &Geo) -> Pecas {
    let poly = |pts: &[V]| pts.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>();
    match g {
        Geo::Ponto(p) => Pecas { pontos: vec![*p], segs: vec![], tris: vec![] },
        Geo::Reta { a, b } => Pecas { pontos: vec![*a, *b], segs: vec![(*a, *b)], tris: vec![] },
        Geo::Circulo { pts, .. } | Geo::Curva { pts } => Pecas { pontos: pts.clone(), segs: poly(pts), tris: vec![] },
        Geo::Face(f) => {
            let pontos = f.tris.iter().flat_map(|t| t.iter().copied()).collect();
            let segs = f.tris.iter().flat_map(|t| [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]).collect();
            Pecas { pontos, segs, tris: f.tris.clone() }
        }
    }
}

/// Distância mínima entre dois elementos (com os pontos de apoio).
fn dist_min(ga: &Geo, gb: &Geo) -> (f64, V, V) {
    let (a, b) = (pecas(ga), pecas(gb));
    let mut best = (f64::MAX, [0.; 3], [0.; 3]);
    let mut ver = |d: f64, p: V, q: V| {
        if d < best.0 {
            best = (d, p, q);
        }
    };
    // pontos de um contra pontos, segmentos e triângulos do outro (nos dois sentidos)
    for (x, y, troca) in [(&a, &b, false), (&b, &a, true)] {
        for p in &x.pontos {
            if y.segs.is_empty() && y.tris.is_empty() {
                for q in &y.pontos {
                    let d = dist(*p, *q);
                    if troca { ver(d, *q, *p) } else { ver(d, *p, *q) }
                }
            }
            for (s0, s1) in &y.segs {
                let (d, q) = ponto_seg(*p, *s0, *s1);
                if troca { ver(d, q, *p) } else { ver(d, *p, q) }
            }
            for t in &y.tris {
                let (d, q) = ponto_tri(*p, t);
                if troca { ver(d, q, *p) } else { ver(d, *p, q) }
            }
        }
    }
    // segmento contra segmento (arestas que se cruzam sem vértice perto), com limite de custo
    if a.segs.len() * b.segs.len() <= 400_000 {
        for (p1, q1) in &a.segs {
            for (p2, q2) in &b.segs {
                let (d, c1, c2) = seg_seg(*p1, *q1, *p2, *q2);
                ver(d, c1, c2);
            }
        }
    }
    best
}

/// Plano ou reta tratados como ilimitados (geometria infinita).
#[derive(Clone, Copy)]
enum Infinito {
    Plano(V, V),
    Reta(V, V),
}

fn infinito(g: &Geo) -> Option<Infinito> {
    match g {
        Geo::Reta { a, b } => Some(Infinito::Reta(*a, unit(sub(*b, *a)))),
        Geo::Face(Face { tipo: TipoFace::Plano { p, n }, .. }) => Some(Infinito::Plano(*p, *n)),
        Geo::Face(Face { tipo: TipoFace::Cilindro { p, d, .. }, .. }) => Some(Infinito::Reta(*p, *d)),
        _ => None,
    }
}

fn reta_ponto(o: V, d: V, p: V) -> V {
    add(o, mul(d, dot(sub(p, o), d)))
}

/// Distância com plano/reta ilimitados contra o outro elemento.
fn dist_infinita(ia: Infinito, gb: &Geo) -> Option<(f64, V, V)> {
    let ib = infinito(gb);
    let pb = pecas(gb).pontos;
    match (ia, ib) {
        (Infinito::Plano(p, n), Some(Infinito::Plano(q, m))) => (dot(n, m).abs() > 1. - 1e-9).then(|| {
            let s = dot(sub(q, p), n);
            (s.abs(), sub(q, mul(n, s)), q)
        }),
        (Infinito::Plano(p, n), Some(Infinito::Reta(o, d))) | (Infinito::Reta(o, d), Some(Infinito::Plano(p, n))) => (dot(n, d).abs() < 1e-9).then(|| {
            let s = dot(sub(o, p), n);
            (s.abs(), sub(o, mul(n, s)), o)
        }),
        (Infinito::Reta(o1, d1), Some(Infinito::Reta(o2, d2))) => {
            let c = cross(d1, d2);
            if norm(c) < 1e-9 {
                let q = reta_ponto(o1, d1, o2);
                Some((dist(q, o2), q, o2))
            } else {
                // retas reversas: pontos da perpendicular comum
                let r = sub(o1, o2);
                let (b, cc, e) = (dot(d1, d2), dot(d1, r), dot(d2, r));
                let den = 1. - b * b;
                let s = (b * e - cc) / den;
                let t = (e - b * cc) / den;
                let (p1, p2) = (add(o1, mul(d1, s)), add(o2, mul(d2, t)));
                Some((dist(p1, p2), p1, p2))
            }
        }
        (Infinito::Plano(p, n), _) => {
            // plano ilimitado contra pontos: zero se ficam dos dois lados
            let s: Vec<f64> = pb.iter().map(|q| dot(sub(*q, p), n)).collect();
            let (lo, hi) = s.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            if lo <= 0. && hi >= 0. {
                let q = pb[s.iter().enumerate().min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))?.0];
                return Some((0., q, q));
            }
            let (i, v) = s.iter().enumerate().min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))?;
            Some((v.abs(), sub(pb[i], mul(n, *v)), pb[i]))
        }
        (Infinito::Reta(o, d), _) => pb.iter().map(|q| {
            let r = reta_ponto(o, d, *q);
            (dist(r, *q), r, *q)
        }).min_by(|a, b| a.0.total_cmp(&b.0)),
    }
}

fn angulo(a: (V, bool), b: (V, bool)) -> f64 {
    let c = dot(a.0, b.0).abs().clamp(0., 1.);
    let ang = c.acos().to_degrees();
    // reta com plano: o ângulo é com o plano, não com a normal dele
    if a.1 != b.1 {
        90. - ang
    } else {
        ang
    }
}

/// Medidas entre duas listas de elementos.
pub fn medir_entre(a: &[Geo], b: &[Geo], ilimitado: bool) -> Leitura {
    if a.is_empty() || b.is_empty() {
        return Leitura { falta: Some("Escolha os elementos das duas listas."), ..Default::default() };
    }
    let mut l = vec![];
    // distância mínima entre as listas (o menor par)
    let mut best: Option<(f64, V, V)> = None;
    for ga in a {
        for gb in b {
            let d = dist_min(ga, gb);
            if best.is_none_or(|x| d.0 < x.0) {
                best = Some(d);
            }
        }
    }
    let (d, pa, pb) = best.unwrap();
    num(&mut l, "Distância mínima", d, Unidade::Mm);
    let mut apoio = Some((pa, pb));
    if let ([Geo::Ponto(p)], [Geo::Ponto(q)]) = (a, b) {
        num(&mut l, "ΔX", (q[0] - p[0]).abs(), Unidade::Mm);
        num(&mut l, "ΔY", (q[1] - p[1]).abs(), Unidade::Mm);
        num(&mut l, "ΔZ", (q[2] - p[2]).abs(), Unidade::Mm);
    }
    if let ([ga], [gb]) = (a, b) {
        if let (Some(da), Some(db)) = (direcao(ga), direcao(gb)) {
            num(&mut l, "Ângulo", angulo(da, db), Unidade::Grau);
        }
        // plano a plano: planos paralelos, distância entre os planos ilimitados
        if let (Geo::Face(Face { tipo: TipoFace::Plano { p, n }, .. }), Geo::Face(Face { tipo: TipoFace::Plano { p: q, n: m }, .. })) = (ga, gb) {
            if dot(*n, *m).abs() > 1. - 1e-9 {
                num(&mut l, "Plano a plano", dot(sub(*q, *p), *n).abs(), Unidade::Mm);
            }
        }
        // eixo: cilindros e círculos medem pelo eixo; ponto e plano contra o eixo também
        match (eixo(ga), eixo(gb)) {
            (Some((o1, d1)), Some((o2, d2))) => {
                if let Some((x, _, _)) = dist_infinita(Infinito::Reta(o1, d1), &Geo::Reta { a: o2, b: add(o2, d2) }) {
                    num(&mut l, "Eixo a eixo", x, Unidade::Mm);
                }
                if let (Geo::Circulo { c: c1, .. }, Geo::Circulo { c: c2, .. }) = (ga, gb) {
                    num(&mut l, "Centro a centro", dist(*c1, *c2), Unidade::Mm);
                }
            }
            (Some((o, d)), None) | (None, Some((o, d))) => {
                let outro = if eixo(ga).is_some() { gb } else { ga };
                if let Some((x, _, _)) = dist_infinita(Infinito::Reta(o, d), outro) {
                    num(&mut l, "Distância ao eixo", x, Unidade::Mm);
                }
            }
            _ => {}
        }
        // geometria infinita: plano e reta ilimitados
        if ilimitado {
            let r = match (infinito(ga), infinito(gb)) {
                (Some(ia), _) => dist_infinita(ia, gb),
                (None, Some(ib)) => dist_infinita(ib, ga).map(|(x, p, q)| (x, q, p)),
                _ => None,
            };
            if let Some((x, p, q)) = r {
                num(&mut l, "Distância (ilimitada)", x, Unidade::Mm);
                apoio = Some((p, q));
            }
        }
    }
    let ancora = apoio.map(|(p, q)| mid(p, q));
    Leitura { linhas: l, apoio, ancora, ..Default::default() }
}

/// Valor atual de uma medida mantida no documento.
pub fn avaliar(r: &RecomputeResult, m: &Medida) -> Leitura {
    let mut perdidas = 0;
    let mut res = |lista: &[RefMedida]| -> Vec<Geo> {
        lista.iter().filter_map(|rf| {
            let g = resolver(r, rf);
            if g.is_none() {
                perdidas += 1;
            }
            g
        }).collect()
    };
    let (a, b) = (res(&m.a), res(&m.b));
    if perdidas > 0 {
        return Leitura { perdidas, ..Default::default() };
    }
    let mut l = match m.modo {
        ModoMedida::Item => match a.as_slice() {
            [g] => medir_item(g),
            _ => Leitura { falta: Some("Escolha um elemento."), ..Default::default() },
        },
        ModoMedida::Entre => medir_entre(&a, &b, m.infinito),
    };
    if !m.campos.is_empty() {
        l.linhas.retain(|x| m.campos.iter().any(|c| c == x.rotulo));
    }
    l
}

/// Linhas do resultado que o núcleo pode escrever (para a tradução e para a escolha de campos).
pub const ROTULOS: [&str; 20] = [
    "Posição", "Comprimento", "Raio", "Diâmetro", "Ângulo do arco", "Centro", "Área", "Perímetro", "Distância mínima", "ΔX", "ΔY", "ΔZ", "Ângulo", "Plano a plano", "Eixo a eixo",
    "Centro a centro", "Distância ao eixo", "Distância (ilimitada)", "Escolha os elementos das duas listas.", "Escolha um elemento.",
];
