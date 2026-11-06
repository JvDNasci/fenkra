// SPDX-License-Identifier: LGPL-2.1-or-later
//! Marcas de centro associativas (B294): simples, circular e linear.
//!
//! Uma marca guarda os centros dos furos em 3D (como as cotas guardam as âncoras) e o raio do
//! furo. A cada desenho ela é projetada de novo na vista e reencontra os círculos que a
//! projeção traz; por isso acompanha a peça. O círculo de furos também sobrevive a uma mudança do
//! diâmetro do círculo: o centro do círculo (que não anda) e o raio do furo reencontram o mesmo
//! número de furos na nova posição. Arestas ocultas (tipo 3) contam como as visíveis: o centro
//! vem da aresta, não de um corte.

use crate::layout::arc_center;
use crate::source::Proj;
use crate::{Id, V2, V3};
use serde::{Deserialize, Serialize};

/// Como a marca é desenhada.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ModoCentro {
    /// uma cruz no centro de um furo
    #[default]
    Simples,
    /// os furos de um círculo de furos: cruz em cada um e o círculo que passa pelos centros
    Circular,
    /// eixo entre dois furos
    Linear,
}

impl ModoCentro {
    pub const ALL: [ModoCentro; 3] = [ModoCentro::Simples, ModoCentro::Circular, ModoCentro::Linear];
    pub fn label(self) -> &'static str {
        match self {
            ModoCentro::Simples => "Simples",
            ModoCentro::Circular => "Circular",
            ModoCentro::Linear => "Linear",
        }
    }
    /// Quantos furos a marca pede no mínimo.
    pub fn minimo(self) -> usize {
        match self {
            ModoCentro::Simples => 1,
            ModoCentro::Circular => 3,
            ModoCentro::Linear => 2,
        }
    }
}

fn estende() -> f64 {
    2.
}

fn sim() -> bool {
    true
}

/// Marca de centro de uma vista.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MarcaCentro {
    pub id: Id,
    pub view: Id,
    pub modo: ModoCentro,
    /// centros dos furos (modelo); a ordem é a da escolha
    pub anchors: Vec<V3>,
    /// raio dos furos escolhidos (mm do modelo); reencontra os círculos depois de uma mudança
    pub raio_furo: f64,
    /// circular: centro do círculo de furos (modelo)
    #[serde(default)]
    pub centro: Option<V3>,
    /// quanto a cruz passa da borda do furo (mm na folha)
    #[serde(default = "estende")]
    pub extra: f64,
    /// escondida só esta marca (a vista também pode esconder todas, `Drawing::centros_ocultos`)
    #[serde(default = "sim")]
    pub visivel: bool,
}

impl MarcaCentro {
    pub fn nova(id: Id, view: Id, modo: ModoCentro) -> Self {
        MarcaCentro { id, view, modo, anchors: vec![], raio_furo: 0., centro: None, extra: 2., visivel: true }
    }
}

/// Geometria de uma marca no plano da vista (mm do modelo), pronta para virar linhas.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tracos {
    pub segs: Vec<(V2, V2)>,
    /// círculos de furos: centro e raio
    pub circulos: Vec<(V2, f64)>,
    /// algum furo não foi encontrado de novo: a marca usa os pontos guardados e avisa
    pub quebrada: bool,
}

fn dist(a: V2, b: V2) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Círculos e arcos da projeção (visíveis, contornos e ocultos), sem repetir o mesmo.
pub fn arcos(p: &Proj) -> Vec<(V2, f64)> {
    let mut out: Vec<(V2, f64)> = vec![];
    for (pl, k) in &p.lines {
        if !matches!(k, 0 | 2 | 3) {
            continue;
        }
        if let Some((c, r)) = arc_center(pl) {
            if !out.iter().any(|(c2, r2)| dist(*c2, c) < 1e-3 && (r2 - r).abs() < 1e-3) {
                out.push((c, r));
            }
        }
    }
    out
}

/// Arco da projeção que corresponde a um centro: o que cai em cima dele ou, depois de uma mudança
/// do modelo, o mais perto com o mesmo raio (até 4 raios de distância).
fn reencontra(arcos: &[(V2, f64)], q: V2, raio: f64) -> Option<(V2, f64)> {
    let mesmo = |r: f64| raio <= 0. || (r - raio).abs() <= raio * 0.02 + 1e-3;
    if let Some(a) = arcos.iter().find(|(c, _)| dist(*c, q) <= crate::layout::ANCHOR_TOL) {
        return Some(*a);
    }
    arcos
        .iter()
        .filter(|(c, r)| mesmo(*r) && dist(*c, q) <= 4. * r.max(raio))
        .min_by(|a, b| dist(a.0, q).total_cmp(&dist(b.0, q)))
        .copied()
}

/// Furos de um círculo de furos: `n` arcos do mesmo raio, todos à mesma distância de `c`, a mais
/// próxima de `r0` quando houver mais de um grupo assim.
fn grupo_circular(arcos: &[(V2, f64)], c: V2, raio: f64, n: usize, r0: f64) -> Option<Vec<(V2, f64)>> {
    let cand: Vec<(V2, f64, f64)> = arcos.iter().filter(|(_, r)| raio <= 0. || (r - raio).abs() <= raio * 0.02 + 1e-3).map(|(p, r)| (*p, *r, dist(*p, c))).collect();
    let mut melhor: Option<(f64, Vec<(V2, f64)>)> = None;
    for (_, _, d0) in &cand {
        let tol = (d0 * 0.005).max(1e-3);
        let g: Vec<(V2, f64)> = cand.iter().filter(|(_, _, d)| (d - d0).abs() <= tol).map(|(p, r, _)| (*p, *r)).collect();
        if g.len() == n {
            let gap = (d0 - r0).abs();
            if melhor.as_ref().is_none_or(|(b, _)| gap < *b) {
                melhor = Some((gap, g));
            }
        }
    }
    melhor.map(|(_, g)| g)
}

/// Cruz de um furo: dois eixos de linha de centro que passam `ext` além da borda.
fn cruz(t: &mut Tracos, c: V2, r: f64, ext: f64) {
    let h = r + ext;
    t.segs.push(([c[0] - h, c[1]], [c[0] + h, c[1]]));
    t.segs.push(([c[0], c[1] - h], [c[0], c[1] + h]));
}

/// Geometria da marca na vista `p`. `escala` converte o `extra` (folha) para mm do modelo.
pub fn tracos(m: &MarcaCentro, p: &Proj, escala: f64) -> Tracos {
    let mut t = Tracos::default();
    let Some(ax) = p.axes else {
        t.quebrada = true;
        return t;
    };
    let ext = m.extra / escala.max(1e-9);
    let pontos: Vec<V2> = m.anchors.iter().map(|a| ax.project(*a)).collect();
    let arcs = arcos(p);
    let achar = |q: V2| reencontra(&arcs, q, m.raio_furo);
    match m.modo {
        ModoCentro::Simples | ModoCentro::Linear => {
            let mut furos = vec![];
            for q in &pontos {
                match achar(*q) {
                    Some(a) => furos.push(a),
                    None => {
                        t.quebrada = true;
                        furos.push((*q, m.raio_furo));
                    }
                }
            }
            for (c, r) in &furos {
                cruz(&mut t, *c, *r, ext);
            }
            if m.modo == ModoCentro::Linear && furos.len() >= 2 {
                let (a, b) = (furos[0].0, furos[1].0);
                let d = dist(a, b).max(1e-9);
                let u = [(b[0] - a[0]) / d, (b[1] - a[1]) / d];
                // o eixo vai de borda a borda, passando `ext` do furo de cada ponta
                let (ea, eb) = (furos[0].1 + ext, furos[1].1 + ext);
                t.segs.push(([a[0] - u[0] * ea, a[1] - u[1] * ea], [b[0] + u[0] * eb, b[1] + u[1] * eb]));
            }
        }
        ModoCentro::Circular => {
            let n = pontos.len();
            let Some(c3) = m.centro else {
                t.quebrada = true;
                return t;
            };
            let c = ax.project(c3);
            let r0 = if n > 0 { pontos.iter().map(|q| dist(*q, c)).sum::<f64>() / n as f64 } else { 0. };
            // os furos guardados ainda estão em cima de círculos? senão procura o grupo pelo centro
            let ainda: Vec<(V2, f64)> = pontos.iter().filter_map(|q| arcs.iter().find(|(a, _)| dist(*a, *q) <= crate::layout::ANCHOR_TOL).copied()).collect();
            let furos = if ainda.len() == n {
                ainda
            } else {
                match grupo_circular(&arcs, c, m.raio_furo, n, r0) {
                    Some(g) => g,
                    None => {
                        t.quebrada = true;
                        pontos.iter().map(|q| (*q, m.raio_furo)).collect()
                    }
                }
            };
            if furos.is_empty() {
                return t;
            }
            let rc = furos.iter().map(|(q, _)| dist(*q, c)).sum::<f64>() / furos.len() as f64;
            t.circulos.push((c, rc));
            for (q, r) in &furos {
                cruz(&mut t, *q, *r, ext);
            }
            // cruz no centro do círculo de furos
            let h = ext + 2. / escala.max(1e-9);
            t.segs.push(([c[0] - h, c[1]], [c[0] + h, c[1]]));
            t.segs.push(([c[0], c[1] - h], [c[0], c[1] + h]));
        }
    }
    t
}

/// Monta a marca a partir dos pontos clicados (plano da vista): cada ponto é trocado pelo círculo
/// mais próximo da projeção. Devolve `None` se faltar furo ou algum clique não cair perto de um
/// círculo.
pub fn montar(id: Id, view: Id, modo: ModoCentro, cliques: &[V2], p: &Proj) -> Option<MarcaCentro> {
    let ax = p.axes?;
    let arcs = arcos(p);
    let mut m = MarcaCentro::nova(id, view, modo);
    let mut raios = vec![];
    for q in cliques {
        let (c, r) = arcs.iter().min_by(|a, b| (dist(a.0, *q) - a.1).abs().min(dist(a.0, *q)).total_cmp(&(dist(b.0, *q) - b.1).abs().min(dist(b.0, *q)))).copied()?;
        // o clique tem de estar na borda ou dentro do círculo
        if dist(c, *q) > r * 1.5 + 1e-3 {
            return None;
        }
        if m.anchors.iter().any(|a| dist(ax.project(*a), c) < 1e-6) {
            continue;
        }
        m.anchors.push(ax.unproject(c));
        raios.push(r);
    }
    if m.anchors.len() < modo.minimo() {
        return None;
    }
    if modo == ModoCentro::Linear {
        m.anchors.truncate(2);
        raios.truncate(2);
    }
    m.raio_furo = raios.iter().sum::<f64>() / raios.len() as f64;
    if modo == ModoCentro::Circular {
        let pts: Vec<V2> = m.anchors.iter().map(|a| ax.project(*a)).collect();
        let c = circulo_por_pontos(&pts)?;
        m.centro = Some(ax.unproject(c));
    }
    Some(m)
}

/// Centro do círculo que passa pelos pontos (os três primeiros decidem; os outros conferem).
pub fn circulo_por_pontos(pts: &[V2]) -> Option<V2> {
    if pts.len() < 3 {
        return None;
    }
    let (a, b, c) = (pts[0], pts[1], pts[2]);
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-9 {
        return None;
    }
    let sq = |p: V2| p[0] * p[0] + p[1] * p[1];
    let ux = (sq(a) * (b[1] - c[1]) + sq(b) * (c[1] - a[1]) + sq(c) * (a[1] - b[1])) / d;
    let uy = (sq(a) * (c[0] - b[0]) + sq(b) * (a[0] - c[0]) + sq(c) * (b[0] - a[0])) / d;
    let r = dist(a, [ux, uy]);
    pts.iter().all(|q| (dist(*q, [ux, uy]) - r).abs() <= r * 0.01 + 1e-3).then_some([ux, uy])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{circle, Axes};

    /// Vista de topo (x, y) com `n` furos de raio `rf` num círculo de raio `rc`.
    fn flange(n: usize, rc: f64, rf: f64) -> Proj {
        let mut p = Proj { axes: Some(Axes::new([0., 0., 1.], [1., 0., 0.])), ..Default::default() };
        p.lines.push((circle([0., 0.], 60.), 0));
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            p.lines.push((circle([rc * a.cos(), rc * a.sin()], rf), 0));
        }
        p
    }

    #[test]
    fn flange_de_seis_furos_gera_marca_circular_e_acompanha_o_diametro() {
        let p = flange(6, 40., 4.);
        // cliques em cima dos seis furos
        let cliques: Vec<V2> = (0..6).map(|i| [40. * (std::f64::consts::TAU * i as f64 / 6.).cos(), 40. * (std::f64::consts::TAU * i as f64 / 6.).sin()]).collect();
        let m = montar(Id(1), Id(2), ModoCentro::Circular, &cliques, &p).expect("marca circular");
        assert_eq!(m.anchors.len(), 6);
        assert!(m.centro.is_some());
        let t = tracos(&m, &p, 1.);
        assert!(!t.quebrada);
        assert_eq!(t.circulos.len(), 1, "um círculo de furos");
        assert!((t.circulos[0].1 - 40.).abs() < 0.05, "raio do círculo {}", t.circulos[0].1);
        // seis cruzes nos furos (2 eixos cada) mais a do centro do círculo
        assert_eq!(t.segs.len(), 6 * 2 + 2);
        // o diâmetro do círculo de furos muda de 80 para 100: a marca segue os furos
        let p2 = flange(6, 50., 4.);
        let t2 = tracos(&m, &p2, 1.);
        assert!(!t2.quebrada, "a marca reencontra os seis furos");
        assert!((t2.circulos[0].1 - 50.).abs() < 0.05, "raio novo {}", t2.circulos[0].1);
        assert_eq!(t2.segs.len(), 6 * 2 + 2);
        // se um furo some, a marca avisa em vez de inventar
        let mut p3 = flange(6, 50., 4.);
        p3.lines.pop();
        assert!(tracos(&m, &p3, 1.).quebrada);
    }

    #[test]
    fn simples_cruza_alem_da_borda_e_linear_liga_dois_furos() {
        let p = flange(6, 40., 4.);
        let m = montar(Id(1), Id(2), ModoCentro::Simples, &[[40., 0.5]], &p).unwrap();
        let t = tracos(&m, &p, 1.);
        assert_eq!(t.segs.len(), 2);
        let (a, b) = t.segs[0];
        assert!((dist(a, b) - 2. * (4. + 2.)).abs() < 1e-6, "cruz passa 2 mm da borda");
        // a escala da folha 2:1 reduz a extensão no modelo
        let t2 = tracos(&m, &p, 2.);
        assert!((dist(t2.segs[0].0, t2.segs[0].1) - 2. * (4. + 1.)).abs() < 1e-6);
        let l = montar(Id(3), Id(2), ModoCentro::Linear, &[[40., 0.], [-40., 0.]], &p).unwrap();
        let t = tracos(&l, &p, 1.);
        // duas cruzes e o eixo entre os furos, de borda a borda
        assert_eq!(t.segs.len(), 5);
        let eixo = t.segs[4];
        assert!((dist(eixo.0, eixo.1) - (80. + 2. * 6.)).abs() < 1e-6);
    }

    #[test]
    fn clique_longe_de_circulos_nao_cria_marca_e_oculta_conta() {
        let p = flange(6, 40., 4.);
        assert!(montar(Id(1), Id(2), ModoCentro::Simples, &[[0., 0.]], &Proj { axes: p.axes, ..Default::default() }).is_none());
        assert!(montar(Id(1), Id(2), ModoCentro::Circular, &[[40., 0.], [20., 34.64]], &p).is_none(), "circular pede ao menos três furos");
        // furo só em aresta oculta (tipo 3) também recebe a marca
        let mut o = Proj { axes: p.axes, ..Default::default() };
        o.lines.push((circle([10., 10.], 3.), 3));
        let m = montar(Id(1), Id(2), ModoCentro::Simples, &[[10., 10.]], &o).unwrap();
        assert!(!tracos(&m, &o, 1.).quebrada);
    }
}
