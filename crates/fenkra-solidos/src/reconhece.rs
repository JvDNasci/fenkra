// SPDX-License-Identifier: LGPL-2.1-or-later
//! B143: reconhecimento de features em corpo importado (STEP).
//!
//! Fase 1 (`analisar`): lê o corpo sem alterá-lo. Monta o grafo de faces (cada aresta carrega a
//! convexidade: convexa, côncava ou tangente) e procura padrões analíticos:
//! - cilindro interno que fecha a volta e termina em dois lados abertos: furo passante; se um lado
//!   fecha num disco plano ou numa ponta cônica: furo cego;
//! - cone coaxial que alarga a boca: escareado; cilindro maior ligado a um menor por um anel
//!   plano: rebaixo;
//! - cilindro que cobre menos de meia volta e liga dois planos por arestas tangentes: filete de
//!   raio constante (convexo ou côncavo);
//! - face plana estreita entre duas faces planas por arestas convexas, com ângulo entre 10° e 80°
//!   em relação a cada vizinha: chanfro.
//!
//! Só superfícies analíticas. Toros e B-splines não viram feature: ficam no corpo e entram no
//! relatório. O texto da interface diz "estrutura inferida", nunca "histórico restaurado".
//!
//! Fase 2 (`converter`): para cada feature reconhecida, apaga as faces dela com a edição direta
//! (ADR 0058), recria como comando do Fenkra (furo, filete, chanfro) e só mantém a troca se volume,
//! área e centroide do corpo recriado coincidirem com o original em 1e-6 relativo. Senão o corpo
//! permanece como estava e o motivo vai para o relatório.

use fenkra_core::{
    AxisRef, DirectEdit, Document, EdgeMode, EdgeOp, EdgeSel, Extent, Feature, FeatureKind, Hole, HoleKind, OpDireta,
    PointRef,
};
use fenkra_kernel::{EdgeFormula, Mesh, Shape, NO_FACE};
use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

/// Acima disto a análise não roda por conta própria (peça de milhares de faces).
pub const MAX_FACES: usize = 4000;
/// No máximo tantas features por conversão; o resto fica como corpo.
pub const MAX_CONVERSOES: usize = 200;
/// Tolerância relativa da verificação de similaridade da fase 2.
pub const TOL_REL: f64 = 1e-6;

type V3 = [f64; 3];

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mais(a: V3, b: V3, k: f64) -> V3 {
    [a[0] + k * b[0], a[1] + k * b[1], a[2] + k * b[2]]
}
fn norma(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    let n = norma(a).max(1e-300);
    [a[0] / n, a[1] / n, a[2] / n]
}
fn cruz(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dist_reta(p: V3, o: V3, d: V3) -> f64 {
    let v = sub(p, o);
    norma(mais(v, d, -dot(v, d)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convexidade {
    Convexa,
    Concava,
    Tangente,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    FuroPassante,
    FuroCego,
    Filete,
    Chanfro,
}

impl Tipo {
    pub fn nome(self) -> &'static str {
        match self {
            Tipo::FuroPassante => "furo passante",
            Tipo::FuroCego => "furo cego",
            Tipo::Filete => "filete",
            Tipo::Chanfro => "chanfro",
        }
    }
}

/// Uma feature inferida do corpo. As medidas são as que couberam no corpo, sem a lógica de projeto.
#[derive(Clone, Debug, PartialEq)]
pub struct Encontrada {
    pub tipo: Tipo,
    /// furo: diâmetro; filete: raio; chanfro: distância na primeira face
    pub medida: f64,
    /// furo: profundidade desde a boca (cilindros somados); chanfro: distância na segunda face;
    /// filete: ângulo coberto, em graus
    pub medida2: f64,
    /// filete: convexo (arredonda uma quina); furo: falso
    pub convexo: bool,
    /// furo: ponto da boca, sobre o eixo
    pub boca: V3,
    /// furo: sentido do eixo entrando no material; filete e chanfro: direção da aresta
    pub direcao: V3,
    /// escareado: diâmetro na superfície e ângulo incluído (graus)
    pub escareado: Option<(f64, f64)>,
    /// rebaixo: diâmetro e profundidade
    pub rebaixo: Option<(f64, f64)>,
    /// ponta cônica no fundo do furo cego: ângulo incluído (graus)
    pub ponta: Option<f64>,
    /// índices das faces (ordem da malha) que formam a feature
    pub faces: Vec<usize>,
    /// um ponto sobre cada face de `faces`, para selecionar no 3D e para a edição direta
    pub pontos: Vec<V3>,
    /// ponto de referência da feature no 3D (meio do eixo do furo ou da aresta viva)
    pub centro: V3,
    /// a fase 2 sabe recriar esta feature como comando do Fenkra
    pub convertivel: bool,
    pub aviso: Option<String>,
}

impl Encontrada {
    pub fn descricao(&self) -> String {
        match self.tipo {
            Tipo::FuroPassante | Tipo::FuroCego => {
                let mut s = format!("{} Ø{:.3}", self.tipo.nome(), self.medida);
                if self.tipo == Tipo::FuroCego {
                    s += &format!(", profundidade {:.3}", self.medida2);
                }
                if let Some((d, a)) = self.escareado {
                    s += &format!(", escareado Ø{d:.3} a {a:.1}°");
                }
                if let Some((d, p)) = self.rebaixo {
                    s += &format!(", rebaixo Ø{d:.3} x {p:.3}");
                }
                if let Some(a) = self.ponta {
                    s += &format!(", ponta {a:.1}°");
                }
                s
            }
            Tipo::Filete => format!("filete {} R{:.3} ({:.0}°)", if self.convexo { "convexo" } else { "côncavo" }, self.medida, self.medida2),
            Tipo::Chanfro => format!("chanfro {:.3} x {:.3}", self.medida, self.medida2),
        }
    }
}

/// O que a análise viu no corpo.
#[derive(Clone, Debug, Default)]
pub struct Reconhecimento {
    pub faces: usize,
    pub arestas_convexas: usize,
    pub arestas_concavas: usize,
    pub arestas_tangentes: usize,
    /// faces fora do que o reconhecimento sabe ler: toros e superfícies livres (B-splines)
    pub toros: usize,
    pub livres: usize,
    pub encontradas: Vec<Encontrada>,
    /// grupos de faces analíticas que lembram feature mas não fecharam (relatados, não convertidos)
    pub duvidosas: Vec<String>,
    /// motivo, quando a análise nem rodou (peça grande demais, forma vazia)
    pub pulada: Option<String>,
}

impl Reconhecimento {
    pub fn contar(&self, t: Tipo) -> usize {
        self.encontradas.iter().filter(|e| e.tipo == t).count()
    }

    /// Texto curto para o relatório de importação. Sempre diz que a estrutura é inferida.
    pub fn resumo(&self) -> String {
        if let Some(p) = &self.pulada {
            return format!("Features inferidas: análise não feita ({p})\n");
        }
        let mut s = format!(
            "Features inferidas (estrutura deduzida da geometria, não é o histórico original): {} furos passantes, {} furos cegos, {} filetes, {} chanfros\nGrafo de faces: {} faces; arestas {} convexas, {} côncavas, {} tangentes\n",
            self.contar(Tipo::FuroPassante),
            self.contar(Tipo::FuroCego),
            self.contar(Tipo::Filete),
            self.contar(Tipo::Chanfro),
            self.faces,
            self.arestas_convexas,
            self.arestas_concavas,
            self.arestas_tangentes,
        );
        if self.toros + self.livres > 0 {
            s += &format!("Ficam como corpo: {} faces tóricas e {} superfícies livres (B-spline)\n", self.toros, self.livres);
        }
        for d in &self.duvidosas {
            s += &format!("Não reconhecido: {d}\n");
        }
        s
    }
}

// ---------------------------------------------------------------------------------------- grafo

/// Malha com os índices que a análise usa o tempo todo.
struct Grafo {
    m: Mesh,
    face_edges: Vec<Vec<usize>>,
    diag: f64,
}

fn v3f(v: &[f32], i: usize) -> V3 {
    [v[i * 3] as f64, v[i * 3 + 1] as f64, v[i * 3 + 2] as f64]
}

impl Grafo {
    fn novo(s: &Shape) -> Result<Grafo, String> {
        let caixa = s.bbox().ok_or("a forma está vazia")?;
        let diag = norma(sub(caixa.1, caixa.0)).max(1e-6);
        let m = s.mesh_angle(diag * 1e-3, 0.25).map_err(|e| e.0)?;
        let mut face_edges = vec![vec![]; m.face_info.len()];
        for (e, p) in m.edge_faces.iter().enumerate() {
            for f in p.iter().filter(|f| **f != NO_FACE) {
                if let Some(v) = face_edges.get_mut(*f as usize) {
                    if !v.contains(&e) {
                        v.push(e);
                    }
                }
            }
        }
        Ok(Grafo { m, face_edges, diag })
    }

    fn vizinhas(&self, e: usize) -> Option<(usize, usize)> {
        let p = self.m.edge_faces.get(e)?;
        (p[0] != NO_FACE && p[1] != NO_FACE).then_some((p[0] as usize, p[1] as usize))
    }

    fn outra(&self, e: usize, f: usize) -> Option<usize> {
        let (a, b) = self.vizinhas(e)?;
        if a == f { Some(b) } else if b == f { Some(a) } else { None }
    }

    fn triangulos(&self, f: usize) -> &[u32] {
        let ini = self.m.face_ranges.get(f).copied().unwrap_or(0) as usize;
        let fim = self.m.face_ranges.get(f + 1).map_or(self.m.indices.len(), |v| *v as usize);
        self.m.indices.get(ini..fim).unwrap_or(&[])
    }

    /// Normal de fora da face, pelo vértice da malha mais perto de `p`.
    fn normal_perto(&self, f: usize, p: V3) -> Option<V3> {
        let mut melhor: Option<(f64, V3)> = None;
        for &i in self.triangulos(f) {
            let q = v3f(&self.m.positions, i as usize);
            let d = norma(sub(q, p));
            if melhor.map_or(true, |(m, _)| d < m) {
                melhor = Some((d, v3f(&self.m.normals, i as usize)));
            }
        }
        melhor.map(|(_, n)| unit(n))
    }

    /// Vértice da face mais perto da aresta que ainda está fora dela: aponta para dentro da face.
    fn interno_perto(&self, f: usize, p: V3, t: V3) -> Option<V3> {
        let eps = self.diag * 1e-5;
        let mut melhor: Option<(f64, V3)> = None;
        for &i in self.triangulos(f) {
            let q = v3f(&self.m.positions, i as usize);
            let d = sub(q, p);
            let perp = mais(d, t, -dot(d, t));
            let dp = norma(perp);
            if dp > eps && melhor.map_or(true, |(m, _)| dp < m) {
                melhor = Some((dp, perp));
            }
        }
        melhor.map(|(_, v)| v)
    }

    fn tangente(&self, e: usize) -> Option<V3> {
        match self.m.edge_formula(e)? {
            EdgeFormula::Line { a, b } => Some(unit(sub(b, a))),
            EdgeFormula::Conic { u, v, t0, t1, .. } => {
                let t = (t0 + t1) / 2.;
                Some(unit(mais([-u[0] * t.sin(), -u[1] * t.sin(), -u[2] * t.sin()], v, t.cos())))
            }
        }
    }

    fn meio(&self, e: usize) -> V3 {
        v3f(&self.m.edge_mids, e)
    }

    /// Convexidade da aresta. Tangente vem do kernel; o resto, da normal de uma face contra o lado
    /// em que a outra face entra.
    fn convexidade(&self, e: usize) -> Option<Convexidade> {
        let (a, b) = self.vizinhas(e)?;
        if a == b {
            return None;
        }
        if self.m.edge_smooth.get(e).copied().unwrap_or(0) == 1 {
            return Some(Convexidade::Tangente);
        }
        let mid = self.meio(e);
        let t = self.tangente(e)?;
        let na = self.normal_perto(a, mid)?;
        let d = self.interno_perto(b, mid, t)?;
        Some(if dot(na, d) < 0. { Convexidade::Convexa } else { Convexidade::Concava })
    }

    /// Normal de uma face plana.
    fn normal_plano(&self, f: usize) -> Option<V3> {
        let p = self.m.face_info.get(f)?.point;
        self.normal_perto(f, p)
    }

    /// Arestas circulares da face: centro, raio e normal do plano do círculo.
    fn circulos(&self, f: usize) -> Vec<(V3, f64, V3, usize)> {
        let mut r = vec![];
        for &e in &self.face_edges[f] {
            if let Some(EdgeFormula::Conic { c, u, v, .. }) = self.m.edge_formula(e) {
                let (ru, rv) = (norma(u), norma(v));
                if ru > 1e-9 && (ru - rv).abs() < 1e-6 * (1. + ru) && dot(u, v).abs() < 1e-6 * ru * rv {
                    r.push((c, ru, unit(cruz(u, v)), e));
                }
            }
        }
        r
    }
}

// ---------------------------------------------------------------------------------------- furos

#[derive(Clone, Copy, Debug)]
struct Cil {
    face: usize,
    origem: V3,
    eixo: V3,
    raio: f64,
    h0: f64,
    h1: f64,
    interno: bool,
    area: f64,
}

#[derive(Clone, Copy, Debug)]
enum Fim {
    Aberto,
    Fundo { face: usize },
    Ponta { face: usize, angulo: f64 },
    Escareado { face: usize, diametro: f64, angulo: f64 },
    Degrau { face: usize },
    BaseRebaixo { face: usize },
    Desconhecido,
}

struct GrupoFuro {
    raio: f64,
    eixo: V3,
    origem: V3,
    lo: f64,
    hi: f64,
    faces: Vec<usize>,
    fim_lo: Fim,
    fim_hi: Fim,
}

impl GrupoFuro {
    fn ponto(&self, s: f64) -> V3 {
        mais(self.origem, self.eixo, s)
    }
}

/// O sinal da orientação no B-Rep não basta: um cilindro espelhado tem sistema de eixos invertido.
/// A normal da malha diz para onde a face olha de verdade.
fn olha_para_o_eixo(g: &Grafo, f: usize, origem: V3, eixo: V3) -> Option<bool> {
    let i = *g.triangulos(f).first()? as usize;
    let p = v3f(&g.m.positions, i);
    let n = v3f(&g.m.normals, i);
    let v = sub(p, origem);
    Some(dot(n, mais(v, eixo, -dot(v, eixo))) < 0.)
}

fn agrupa_furos(cils: &[Cil]) -> Vec<(Vec<Cil>, f64, f64)> {
    const TOL: f64 = 1e-4;
    let mut grupos: Vec<Vec<Cil>> = vec![];
    for c in cils.iter().filter(|c| c.interno) {
        let ok = |g: &Vec<Cil>| {
            let b = g[0];
            (b.raio - c.raio).abs() < TOL && dot(b.eixo, c.eixo).abs() > 1. - 1e-9 && dist_reta(c.origem, b.origem, b.eixo) < TOL
        };
        match grupos.iter_mut().find(|g| ok(g)) {
            Some(g) => g.push(*c),
            None => grupos.push(vec![*c]),
        }
    }
    let mut saida = vec![];
    for g in grupos {
        let b = g[0];
        let faixa = |c: &Cil| {
            let s = dot(c.eixo, b.eixo).signum();
            let base = dot(sub(c.origem, b.origem), b.eixo);
            let (x, y) = (base + s * c.h0, base + s * c.h1);
            (x.min(y), x.max(y))
        };
        let mut faixas: Vec<(f64, f64)> = g.iter().map(faixa).collect();
        faixas.sort_by(|a, c| a.0.total_cmp(&c.0));
        let lo = faixas[0].0;
        let hi = faixas.iter().map(|f| f.1).fold(f64::MIN, f64::max);
        let (mut coberto, mut ate) = (0., f64::MIN);
        for (a, c) in &faixas {
            coberto += (c - a.max(ate)).max(0.);
            ate = ate.max(*c);
        }
        let area: f64 = g.iter().map(|c| c.area).sum();
        let volta = area / (b.raio * coberto.max(1e-12)) / TAU;
        if volta >= 0.98 {
            saida.push((g, lo, hi));
        }
    }
    saida
}

fn classifica_fim(g: &Grafo, gr: &GrupoFuro, s: f64) -> Fim {
    let p = gr.ponto(s);
    let tol = 1e-3 * (1. + gr.raio);
    let dentro: HashSet<usize> = gr.faces.iter().copied().collect();
    let mut viz: Option<usize> = None;
    for &f in &gr.faces {
        for (c, r, n, e) in g.circulos(f) {
            if (r - gr.raio).abs() < tol && norma(sub(c, p)) < tol && dot(n, gr.eixo).abs() > 1. - 1e-6 {
                if let Some(o) = g.outra(e, f) {
                    if !dentro.contains(&o) {
                        viz = Some(o);
                    }
                }
            }
        }
    }
    let Some(v) = viz else { return Fim::Desconhecido };
    let info = g.m.face_info[v];
    // círculos coaxiais da vizinha, fora o do próprio furo
    let coaxiais: Vec<(V3, f64)> = g
        .circulos(v)
        .into_iter()
        .filter(|(c, _, n, _)| dist_reta(*c, gr.origem, gr.eixo) < tol && dot(*n, gr.eixo).abs() > 1. - 1e-6)
        .map(|(c, r, _, _)| (c, r))
        .collect();
    match info.kind {
        0 => {
            let Some(n) = g.normal_plano(v) else { return Fim::Desconhecido };
            if dot(n, gr.eixo).abs() < 1. - 1e-6 {
                return Fim::Desconhecido;
            }
            let outros: Vec<f64> = coaxiais.iter().map(|c| c.1).filter(|r| (r - gr.raio).abs() > tol).collect();
            if outros.iter().any(|r| *r < gr.raio) {
                return Fim::Degrau { face: v };
            }
            if outros.iter().any(|r| *r > gr.raio) {
                return Fim::BaseRebaixo { face: v };
            }
            if (info.area - PI * gr.raio * gr.raio).abs() < 0.01 * PI * gr.raio * gr.raio {
                Fim::Fundo { face: v }
            } else {
                Fim::Aberto
            }
        }
        2 => {
            // cone coaxial: a outra borda circular alarga (escareado), estreita até um ponto (ponta)
            // ou não existe. O vértice do cone sai do kernel como um círculo de raio quase zero.
            let outros: Vec<(V3, f64)> = coaxiais.into_iter().filter(|(_, r)| (r - gr.raio).abs() > tol).collect();
            match outros.first() {
                Some((c2, r2)) if *r2 > gr.raio => {
                    let h = norma(sub(*c2, p)).max(1e-12);
                    Fim::Escareado { face: v, diametro: 2. * r2, angulo: 2. * ((r2 - gr.raio) / h).atan().to_degrees() }
                }
                Some((c2, r2)) if *r2 < 1e-3 * (1. + gr.raio) => {
                    let h = norma(sub(*c2, p)).max(1e-12);
                    Fim::Ponta { face: v, angulo: 2. * (gr.raio / h).atan().to_degrees() }
                }
                _ => Fim::Desconhecido,
            }
        }
        _ => Fim::Desconhecido,
    }
}

fn fim_aberto(f: &Fim) -> bool {
    matches!(f, Fim::Aberto | Fim::Escareado { .. } | Fim::BaseRebaixo { .. })
}

fn fim_fechado(f: &Fim) -> bool {
    matches!(f, Fim::Fundo { .. } | Fim::Ponta { .. })
}

fn faces_do_fim(f: &Fim) -> Option<usize> {
    match f {
        Fim::Fundo { face } | Fim::Ponta { face, .. } | Fim::Escareado { face, .. } | Fim::Degrau { face } | Fim::BaseRebaixo { face } => Some(*face),
        _ => None,
    }
}

fn acha_furos(g: &Grafo, cils: &[Cil], duvidosas: &mut Vec<String>) -> Vec<Encontrada> {
    let mut grupos: Vec<GrupoFuro> = agrupa_furos(cils)
        .into_iter()
        .map(|(cs, lo, hi)| {
            let b = cs[0];
            let mut gr = GrupoFuro { raio: b.raio, eixo: b.eixo, origem: b.origem, lo, hi, faces: cs.iter().map(|c| c.face).collect(), fim_lo: Fim::Desconhecido, fim_hi: Fim::Desconhecido };
            gr.fim_lo = classifica_fim(g, &gr, lo);
            gr.fim_hi = classifica_fim(g, &gr, hi);
            gr
        })
        .collect();
    let mut usados = vec![false; grupos.len()];
    let mut saida = vec![];
    let ponto_de = |f: usize| g.m.face_info[f].point;

    // rebaixos: o grupo maior tem um Degrau cuja face é a BaseRebaixo do menor
    for i in 0..grupos.len() {
        for j in 0..grupos.len() {
            if i == j || usados[i] || usados[j] {
                continue;
            }
            let (grande, pequeno) = (&grupos[i], &grupos[j]);
            let degrau = [grande.fim_lo, grande.fim_hi].into_iter().find_map(|f| if let Fim::Degrau { face } = f { Some(face) } else { None });
            let base = [pequeno.fim_lo, pequeno.fim_hi].into_iter().find_map(|f| if let Fim::BaseRebaixo { face } = f { Some(face) } else { None });
            let (Some(d), Some(b)) = (degrau, base) else { continue };
            if d != b || grande.raio <= pequeno.raio {
                continue;
            }
            // lado do degrau no grande, lado da base no pequeno
            let grande_degrau_hi = matches!(grande.fim_hi, Fim::Degrau { .. });
            let (boca_g, outro_g) = if grande_degrau_hi { (grande.fim_lo, grande.hi) } else { (grande.fim_hi, grande.lo) };
            let (boca_p_s, longe_p, longe_fim) =
                if matches!(pequeno.fim_lo, Fim::BaseRebaixo { .. }) { (pequeno.lo, pequeno.hi, pequeno.fim_hi) } else { (pequeno.hi, pequeno.lo, pequeno.fim_lo) };
            let _ = boca_p_s;
            if !matches!(boca_g, Fim::Aberto) {
                continue;
            }
            let passante = fim_aberto(&longe_fim);
            if !passante && !fim_fechado(&longe_fim) {
                continue;
            }
            let boca = grande.ponto(if grande_degrau_hi { grande.lo } else { grande.hi });
            let prof_r = (outro_g - if grande_degrau_hi { grande.lo } else { grande.hi }).abs();
            let longe = pequeno.ponto(longe_p);
            let dir = unit(sub(longe, boca));
            let total = norma(sub(longe, boca));
            let mut faces: Vec<usize> = grande.faces.iter().chain(pequeno.faces.iter()).copied().collect();
            faces.push(d);
            let mut ponta = None;
            if let Some(f) = faces_do_fim(&longe_fim) {
                faces.push(f);
            }
            let mut profundidade = total;
            if let Fim::Ponta { angulo, .. } = longe_fim {
                ponta = Some(angulo);
                // profundidade do cilindro, sem a ponta
                profundidade = total;
            }
            let pontos = faces.iter().map(|f| ponto_de(*f)).collect();
            saida.push(Encontrada {
                tipo: if passante { Tipo::FuroPassante } else { Tipo::FuroCego },
                medida: 2. * pequeno.raio,
                medida2: profundidade,
                convexo: false,
                boca,
                direcao: dir,
                escareado: None,
                rebaixo: Some((2. * grande.raio, prof_r)),
                ponta,
                faces,
                pontos,
                centro: mais(boca, dir, total / 2.),
                convertivel: true,
                aviso: None,
            });
            usados[i] = true;
            usados[j] = true;
        }
    }

    for (i, gr) in grupos.iter_mut().enumerate() {
        if usados[i] {
            continue;
        }
        let (lo_aberto, hi_aberto) = (fim_aberto(&gr.fim_lo), fim_aberto(&gr.fim_hi));
        let (lo_fechado, hi_fechado) = (fim_fechado(&gr.fim_lo), fim_fechado(&gr.fim_hi));
        // boca = lado aberto (com preferência ao escareado); fundo = lado fechado ou o outro aberto
        let (boca_s, boca_fim, longe_s, longe_fim) = if lo_aberto && (hi_fechado || hi_aberto) && !(matches!(gr.fim_hi, Fim::Escareado { .. }) && !matches!(gr.fim_lo, Fim::Escareado { .. })) {
            (gr.lo, gr.fim_lo, gr.hi, gr.fim_hi)
        } else if hi_aberto && (lo_fechado || lo_aberto) {
            (gr.hi, gr.fim_hi, gr.lo, gr.fim_lo)
        } else {
            duvidosas.push(format!("cilindro interno Ø{:.3} em {:.2?} sem fechamento claro", 2. * gr.raio, gr.ponto((gr.lo + gr.hi) / 2.)));
            continue;
        };
        if matches!(boca_fim, Fim::BaseRebaixo { .. }) || matches!(longe_fim, Fim::BaseRebaixo { .. }) {
            duvidosas.push(format!("furo Ø{:.3} sobre um rebaixo sem par coaxial", 2. * gr.raio));
            continue;
        }
        let passante = fim_aberto(&longe_fim);
        let sinal = if boca_s < longe_s { 1. } else { -1. };
        let dir = mais([0.; 3], gr.eixo, sinal);
        let boca_cil = gr.ponto(boca_s);
        let comp = (longe_s - boca_s).abs();
        let mut faces = gr.faces.clone();
        let mut escareado = None;
        let mut boca = boca_cil;
        if let Fim::Escareado { face, diametro, angulo } = boca_fim {
            faces.push(face);
            escareado = Some((diametro, angulo));
            // a boca fica na superfície: do círculo menor até o maior ao longo do eixo
            let prof_cone = (diametro / 2. - gr.raio) / (angulo.to_radians() / 2.).tan();
            boca = mais(boca_cil, dir, -prof_cone);
        }
        let extra = norma(sub(boca_cil, boca));
        if let Fim::Escareado { face, .. } = longe_fim {
            faces.push(face);
            duvidosas.push(format!("furo Ø{:.3} escareado nos dois lados: só um escareado é convertido", 2. * gr.raio));
        }
        let mut ponta = None;
        if let Some(f) = faces_do_fim(&longe_fim) {
            if !matches!(longe_fim, Fim::Escareado { .. }) {
                faces.push(f);
            }
        }
        if let Fim::Ponta { angulo, .. } = longe_fim {
            ponta = Some(angulo);
        }
        let pontos = faces.iter().map(|f| ponto_de(*f)).collect();
        saida.push(Encontrada {
            tipo: if passante { Tipo::FuroPassante } else { Tipo::FuroCego },
            medida: 2. * gr.raio,
            medida2: comp + extra,
            convexo: false,
            boca,
            direcao: dir,
            escareado,
            rebaixo: None,
            ponta,
            faces,
            pontos,
            centro: mais(boca, dir, (comp + extra) / 2.),
            convertivel: !matches!(longe_fim, Fim::Escareado { .. }),
            aviso: None,
        });
    }
    saida
}

// ------------------------------------------------------------------------------ filetes e chanfros

fn acha_filetes(g: &Grafo, cils: &[Cil], em_furo: &HashSet<usize>) -> Vec<Encontrada> {
    let mut saida = vec![];
    for c in cils {
        if em_furo.contains(&c.face) {
            continue;
        }
        let comp = c.h1 - c.h0;
        if comp <= 1e-9 {
            continue;
        }
        let volta = c.area / (c.raio * comp);
        if !(0.05..PI * 0.95).contains(&volta) {
            continue;
        }
        // duas arestas retas, paralelas ao eixo e tangentes a planos
        let mut tang = 0;
        for &e in &g.face_edges[c.face] {
            let Some(EdgeFormula::Line { a, b }) = g.m.edge_formula(e) else { continue };
            if dot(unit(sub(b, a)), c.eixo).abs() < 1. - 1e-6 {
                continue;
            }
            if g.m.edge_smooth.get(e).copied().unwrap_or(0) != 1 {
                continue;
            }
            if let Some(o) = g.outra(e, c.face) {
                if g.m.face_info[o].kind == 0 {
                    tang += 1;
                }
            }
        }
        if tang != 2 {
            continue;
        }
        let eixo = if c.eixo[0] + c.eixo[1] + c.eixo[2] >= 0. { c.eixo } else { c.eixo.map(|x| -x) };
        saida.push(Encontrada {
            tipo: Tipo::Filete,
            medida: c.raio,
            medida2: volta.to_degrees(),
            convexo: !c.interno,
            boca: g.m.face_info[c.face].point,
            direcao: eixo,
            escareado: None,
            rebaixo: None,
            ponta: None,
            faces: vec![c.face],
            pontos: vec![g.m.face_info[c.face].point],
            centro: mais(c.origem, c.eixo, (c.h0 + c.h1) / 2.),
            convertivel: true,
            aviso: None,
        });
    }
    saida
}

fn acha_chanfros(g: &Grafo) -> Vec<Encontrada> {
    let mut saida = vec![];
    for (f, info) in g.m.face_info.iter().enumerate() {
        if info.kind != 0 || g.face_edges[f].len() != 4 {
            continue;
        }
        let Some(nf) = g.normal_plano(f) else { continue };
        // as quatro arestas precisam ser retas; as duas mais longas, paralelas e convexas
        let mut retas: Vec<(usize, f64, V3)> = vec![];
        for &e in &g.face_edges[f] {
            if let Some(EdgeFormula::Line { a, b }) = g.m.edge_formula(e) {
                retas.push((e, norma(sub(b, a)), unit(sub(b, a))));
            }
        }
        if retas.len() != 4 {
            continue;
        }
        retas.sort_by(|a, b| b.1.total_cmp(&a.1));
        let (l1, l2) = (retas[0], retas[1]);
        if dot(l1.2, l2.2).abs() < 1. - 1e-6 {
            continue;
        }
        let (Some(va), Some(vb)) = (g.outra(l1.0, f), g.outra(l2.0, f)) else { continue };
        if g.m.face_info[va].kind != 0 || g.m.face_info[vb].kind != 0 || va == vb {
            continue;
        }
        if g.convexidade(l1.0) != Some(Convexidade::Convexa) || g.convexidade(l2.0) != Some(Convexidade::Convexa) {
            continue;
        }
        let (Some(na), Some(nb)) = (g.normal_plano(va), g.normal_plano(vb)) else { continue };
        let ang_a = dot(nf, na).clamp(-1., 1.).acos();
        let ang_b = dot(nf, nb).clamp(-1., 1.).acos();
        let theta = dot(na, nb).clamp(-1., 1.).acos();
        let (lo, hi) = (10f64.to_radians(), 80f64.to_radians());
        if !(lo..=hi).contains(&ang_a) || !(lo..=hi).contains(&ang_b) || (ang_a + ang_b - theta).abs() > 1e-3 || theta > 150f64.to_radians() {
            continue;
        }
        // largura da face: área / comprimento médio das duas arestas longas
        let largura = info.area / ((l1.1 + l2.1) / 2.).max(1e-12);
        let (da, db) = (largura * ang_b.sin() / theta.sin(), largura * ang_a.sin() / theta.sin());
        saida.push(Encontrada {
            tipo: Tipo::Chanfro,
            medida: da,
            medida2: db,
            convexo: true,
            boca: info.point,
            direcao: l1.2,
            escareado: None,
            rebaixo: None,
            ponta: None,
            faces: vec![f],
            pontos: vec![info.point],
            centro: info.point,
            convertivel: (da - db).abs() < 1e-6 * (1. + da),
            aviso: ((da - db).abs() >= 1e-6 * (1. + da)).then(|| "chanfro assimétrico: fica como corpo".to_string()),
        });
    }
    saida
}

// ---------------------------------------------------------------------------------------- fase 1

/// Fase 1: analisa o corpo e marca o que reconhece, sem alterá-lo.
pub fn analisar(s: &Shape) -> Reconhecimento {
    let mut r = Reconhecimento::default();
    if s.face_count() as usize > MAX_FACES {
        r.pulada = Some(format!("{} faces, acima do limite de {MAX_FACES}", s.face_count()));
        return r;
    }
    let g = match Grafo::novo(s) {
        Ok(g) => g,
        Err(e) => {
            r.pulada = Some(e);
            return r;
        }
    };
    r.faces = g.m.face_info.len();
    for e in 0..g.m.edge_faces.len() {
        match g.convexidade(e) {
            Some(Convexidade::Convexa) => r.arestas_convexas += 1,
            Some(Convexidade::Concava) => r.arestas_concavas += 1,
            Some(Convexidade::Tangente) => r.arestas_tangentes += 1,
            None => {}
        }
    }
    let mut cils = vec![];
    for (i, f) in g.m.face_info.iter().enumerate() {
        match f.kind {
            1 => {
                if let Ok(c) = s.cylinder_at(f.point) {
                    let interno = olha_para_o_eixo(&g, i, c.origin, c.dir).unwrap_or(c.internal);
                    cils.push(Cil { face: i, origem: c.origin, eixo: c.dir, raio: c.radius, h0: c.h0, h1: c.h1, interno, area: f.area });
                }
            }
            4 => r.toros += 1,
            5 => r.livres += 1,
            _ => {}
        }
    }
    let mut duv = vec![];
    let furos = acha_furos(&g, &cils, &mut duv);
    let em_furo: HashSet<usize> = furos.iter().flat_map(|f| f.faces.iter().copied()).collect();
    // cilindros internos de furo que não fecharam também não são filete
    let grupos_furo: HashSet<usize> = agrupa_furos(&cils).into_iter().flat_map(|(g, _, _)| g.into_iter().map(|c| c.face)).collect();
    let mut todos_em_furo = em_furo;
    todos_em_furo.extend(grupos_furo);
    let filetes = acha_filetes(&g, &cils, &todos_em_furo);
    let chanfros = acha_chanfros(&g);
    r.encontradas.extend(furos);
    r.encontradas.extend(filetes);
    r.encontradas.extend(chanfros);
    r.duvidosas = duv;
    r
}

// ---------------------------------------------------------------------------------------- fase 2

/// Resultado da conversão: o documento novo (ou o original, se nada passou) e o que aconteceu com
/// cada feature.
#[derive(Clone, Debug)]
pub struct Conversao {
    pub doc: Document,
    pub convertidas: Vec<usize>,
    /// índice da feature em `Reconhecimento::encontradas` e o motivo de ter ficado como corpo
    pub mantidas: Vec<(usize, String)>,
    pub relatorio: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Medida {
    pub volume: f64,
    area: f64,
    centro: V3,
    diag: f64,
}

fn medida(s: &Shape) -> Option<Medida> {
    let m = s.mass();
    let (lo, hi) = s.bbox()?;
    Some(Medida { volume: m.volume, area: m.area, centro: m.center, diag: norma(sub(hi, lo)) })
}

/// Volume, área e centroide dentro de `TOL_REL` do original (centroide em fração da diagonal).
pub fn similar(a: &Medida, b: &Medida) -> Result<(), String> {
    let rel = |x: f64, y: f64| (x - y).abs() / x.abs().max(1e-12);
    if rel(a.volume, b.volume) > TOL_REL {
        return Err(format!("volume mudou {:.2e} (relativo)", rel(a.volume, b.volume)));
    }
    if rel(a.area, b.area) > TOL_REL {
        return Err(format!("área mudou {:.2e} (relativo)", rel(a.area, b.area)));
    }
    let dc = norma(sub(a.centro, b.centro)) / a.diag.max(1e-12);
    if dc > TOL_REL {
        return Err(format!("centroide andou {dc:.2e} da diagonal"));
    }
    Ok(())
}

fn texto(x: f64) -> String {
    format!("{x:.9}")
}

fn dir_edicao(faces: &[V3]) -> DirectEdit {
    let mut e = DirectEdit::novo(OpDireta::Apagar);
    e.faces = faces.to_vec();
    e.value = "0".into();
    e.delta = ["0".into(), "0".into(), "0".into()];
    e
}

/// Arestas retas do corpo: ponto médio e direção.
fn retas(s: &Shape) -> Vec<(V3, V3)> {
    let Ok(g) = Grafo::novo(s) else { return vec![] };
    (0..g.m.edge_faces.len())
        .filter_map(|e| match g.m.edge_formula(e) {
            Some(EdgeFormula::Line { a, b }) => Some((g.meio(e), unit(sub(b, a)))),
            _ => None,
        })
        .collect()
}

fn comando_de_furo(e: &Encontrada) -> FeatureKind {
    let kind = if let Some((d, p)) = e.rebaixo {
        HoleKind::Rebaixado { diameter: texto(d), depth: texto(p) }
    } else if let Some((d, a)) = e.escareado {
        HoleKind::Escareado { diameter: texto(d), angle: texto(a) }
    } else {
        HoleKind::Simples
    };
    let depth = if e.tipo == Tipo::FuroPassante { Extent::ThroughAll } else { Extent::Blind(texto(e.medida2)) };
    FeatureKind::Hole(Hole {
        sketch: fenkra_core::Id(0),
        kind,
        diameter: texto(e.medida),
        depth,
        thread: None,
        thread_modeled: false,
        standard: None,
        points: vec![PointRef::Fixed(e.boca)],
        dir: Some(AxisRef::Fixed { origin: e.boca, dir: e.direcao }),
        reverse: false,
        tip: e.ponta.map(texto),
        taper: "0".into(),
        left_hand: false,
        extra: Default::default(),
    })
}

fn comando_de_aresta(e: &Encontrada, meio: V3) -> FeatureKind {
    let op = EdgeOp {
        value: texto(e.medida),
        edges: EdgeSel::Edges(vec![meio]),
        mode: EdgeMode::Constant,
        prop: Default::default(),
        keys: vec![None],
        extra: Default::default(),
    };
    if e.tipo == Tipo::Filete { FeatureKind::Fillet(op) } else { FeatureKind::Chamfer(op) }
}

/// Fase 2: recria cada feature reconhecida como comando do Fenkra e só mantém a troca se o corpo
/// recriado for igual ao original em volume, área e centroide (1e-6 relativo). `quais` escolhe por
/// índice de `Reconhecimento::encontradas`; `None` tenta todas.
pub fn converter(doc: &Document, rec: &Reconhecimento, quais: Option<&[usize]>) -> Result<Conversao, String> {
    let original = crate::final_shape(doc).ok_or("o documento não tem corpo")?;
    let alvo = medida(&original).ok_or("corpo sem caixa")?;
    let mut atual = doc.clone();
    let mut convertidas = vec![];
    let mut mantidas = vec![];
    let indices: Vec<usize> = match quais {
        Some(q) => q.to_vec(),
        None => (0..rec.encontradas.len()).collect(),
    };
    for (n, i) in indices.into_iter().enumerate() {
        let Some(e) = rec.encontradas.get(i) else { continue };
        if !e.convertivel {
            mantidas.push((i, e.aviso.clone().unwrap_or_else(|| "tipo ainda não convertido".into())));
            continue;
        }
        if n >= MAX_CONVERSOES {
            mantidas.push((i, format!("passou do limite de {MAX_CONVERSOES} conversões por vez")));
            continue;
        }
        let mut tent = atual.clone();
        tent.add_feature(FeatureKind::DirectEdit(dir_edicao(&e.pontos)));
        let Some(curado) = crate::final_shape(&tent) else {
            mantidas.push((i, "a cura das faces falhou".into()));
            continue;
        };
        match e.tipo {
            Tipo::FuroPassante | Tipo::FuroCego => {
                tent.add_feature(comando_de_furo(e));
            }
            Tipo::Filete | Tipo::Chanfro => {
                // a aresta viva reaparece paralela à direção da feature, a mais perto dela
                let alcance = if e.tipo == Tipo::Filete { 4. * e.medida + 1e-3 } else { 4. * (e.medida + e.medida2) + 1e-3 };
                let achada = retas(&curado)
                    .into_iter()
                    .filter(|(_, d)| dot(*d, e.direcao).abs() > 1. - 1e-6)
                    .map(|(m, _)| (dist_reta(m, e.centro, e.direcao), m))
                    .filter(|(d, _)| *d < alcance)
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                let Some((_, meio)) = achada else {
                    mantidas.push((i, "a aresta viva não reapareceu depois da cura".into()));
                    continue;
                };
                tent.add_feature(comando_de_aresta(e, meio));
            }
        }
        let r = crate::recompute(&tent);
        if let Some((id, msg)) = r.status.iter().find_map(|(id, s)| if let crate::Status::Error(m) = s { Some((*id, m.clone())) } else { None }) {
            let _ = id;
            mantidas.push((i, format!("o comando recriado falhou: {msg}")));
            continue;
        }
        let Some(novo) = crate::final_shape(&tent).as_ref().and_then(medida) else {
            mantidas.push((i, "o corpo recriado ficou vazio".into()));
            continue;
        };
        match similar(&alvo, &novo) {
            Ok(()) => {
                atual = tent;
                convertidas.push(i);
            }
            Err(m) => mantidas.push((i, format!("o corpo recriado difere do original: {m}"))),
        }
    }
    let mut relatorio = format!(
        "Conversão de features inferidas: {} convertidas, {} mantidas como corpo (estrutura inferida da geometria, não é o histórico original)\n",
        convertidas.len(),
        mantidas.len()
    );
    for (i, m) in &mantidas {
        relatorio += &format!("Mantida: {} ({m})\n", rec.encontradas[*i].descricao());
    }
    Ok(Conversao { doc: atual, convertidas, mantidas, relatorio })
}

/// Atalho: analisa o corpo final do documento.
pub fn analisar_documento(doc: &Document) -> Option<Reconhecimento> {
    crate::final_shape(doc).map(|s| analisar(&s))
}

#[allow(dead_code)]
fn _usa(_: &Feature) {}
