// SPDX-License-Identifier: LGPL-2.1-or-later
//! Consulta de topologia: o que um corpo tem, em dados que um agente ou script consiga usar.
//!
//! Tudo sai da malha, que já traz faces, arestas, vizinhança e fórmula de cada aresta na mesma ordem,
//! mais o cilindro exato de cada face. Assim a consulta não precisa de nada novo na ponte C++.

use crate::{r6, r6v, ErroApi};
use fenkra_kernel::{EdgeFormula, Mesh, Shape, NO_FACE};
use serde_json::{json, Value};
use std::f64::consts::TAU;

/// Quantas faces e arestas saem por padrão: uma peça enorme não pode afogar o contexto do agente.
pub const LIMITE_PADRAO: usize = 2000;
/// O que `topologia` devolve quando ninguém filtra.
pub const SECOES: [&str; 6] = ["caixa", "massa", "faces", "arestas", "planos", "cilindros"];

const ANGULO_MALHA: f64 = 0.25;
/// Dois eixos são o mesmo furo se a distância entre eles for menor que isto (mm).
const TOL_EIXO: f64 = 1e-4;
/// Fração da volta a partir da qual um grupo de faces cilíndricas é um furo, e não um canto arredondado.
const VOLTA_COMPLETA: f64 = 0.98;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub tipo: u8,
    pub area: f64,
    pub ponto: [f64; 3],
    pub direcao: [f64; 3],
    pub raio: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CilindroFace {
    pub face: usize,
    pub origem: [f64; 3],
    pub eixo: [f64; 3],
    pub raio: f64,
    pub h0: f64,
    pub h1: f64,
    /// a superfície olha para o eixo: parede de furo
    pub interno: bool,
    pub area: f64,
}

/// Furo: faces cilíndricas internas de mesmo eixo e raio que fecham a volta.
#[derive(Clone, Debug, PartialEq)]
pub struct Furo {
    pub diametro: f64,
    pub eixo: [f64; 3],
    pub inicio: [f64; 3],
    pub fim: [f64; 3],
    pub faces: Vec<usize>,
}

impl Furo {
    pub fn comprimento(&self) -> f64 {
        dist(self.inicio, self.fim)
    }
}

/// O corpo inteiro analisado. A malha fica para o checador amostrar a superfície.
pub struct Resumo {
    pub caixa: ([f64; 3], [f64; 3]),
    pub volume: f64,
    pub area: f64,
    pub centroide: [f64; 3],
    /// momentos principais, em ordem crescente
    pub inercia: [f64; 3],
    pub faces: Vec<Face>,
    pub cilindros: Vec<CilindroFace>,
    pub furos: Vec<Furo>,
    pub malha: Mesh,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

fn soma(a: [f64; 3], b: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] + k * b[0], a[1] + k * b[1], a[2] + k * b[2]]
}

/// Distância do ponto `p` à reta (`o`, `d`), com `d` unitário.
pub(crate) fn dist_reta(p: [f64; 3], o: [f64; 3], d: [f64; 3]) -> f64 {
    let v = sub(p, o);
    dist(v, [d[0] * dot(v, d), d[1] * dot(v, d), d[2] * dot(v, d)])
}

/// Sentido canônico: a primeira componente que não é zero fica positiva.
fn canonico(n: [f64; 3]) -> [f64; 3] {
    let s = n.iter().find(|c| c.abs() > 1e-9).copied().unwrap_or(1.);
    n.map(|c| if s < 0. { -c } else { c })
}

fn nome_do_tipo(t: u8) -> &'static str {
    match t {
        0 => "plano",
        1 => "cilindro",
        2 => "cone",
        3 => "esfera",
        4 => "toro",
        _ => "livre",
    }
}

/// A normal da malha diz para onde a face olha de verdade. O sinal da orientação no B-Rep não basta:
/// um cilindro espelhado tem sistema de eixos invertido, e ali `REVERSED` quer dizer o contrário.
fn olha_para_o_eixo(m: &Mesh, f: usize, origem: [f64; 3], eixo: [f64; 3]) -> Option<bool> {
    let inicio = *m.face_ranges.get(f)? as usize;
    let fim = m
        .face_ranges
        .get(f + 1)
        .map_or(m.indices.len(), |v| *v as usize);
    let i = *m.indices.get(inicio..fim)?.first()? as usize * 3;
    let p = [
        m.positions[i] as f64,
        m.positions[i + 1] as f64,
        m.positions[i + 2] as f64,
    ];
    let n = [
        m.normals[i] as f64,
        m.normals[i + 1] as f64,
        m.normals[i + 2] as f64,
    ];
    let v = sub(p, origem);
    Some(dot(n, soma(v, eixo, -dot(v, eixo))) < 0.)
}

/// Junta as faces internas de mesmo eixo e raio. Um furo sai do STEP partido em duas metades, e a
/// volta conta pela área: área / (raio × altura) dá o ângulo que o grupo cobre.
fn achar_furos(cilindros: &[CilindroFace]) -> Vec<Furo> {
    let mut grupos: Vec<Vec<&CilindroFace>> = vec![];
    for c in cilindros.iter().filter(|c| c.interno) {
        let par = |g: &&mut Vec<&CilindroFace>| {
            let b = g[0];
            (b.raio - c.raio).abs() < TOL_EIXO
                && dot(b.eixo, c.eixo).abs() > 1. - 1e-9
                && dist_reta(c.origem, b.origem, b.eixo) < TOL_EIXO
        };
        match grupos.iter_mut().find(|g| par(g)) {
            Some(g) => g.push(c),
            None => grupos.push(vec![c]),
        }
    }
    let mut furos: Vec<Furo> = grupos
        .into_iter()
        .filter_map(|g| {
            let b = g[0];
            // a altura de cada face vai para o sistema da primeira: eixos iguais podem ter sentidos e origens opostos
            let faixa = |c: &CilindroFace| {
                let s = dot(c.eixo, b.eixo).signum();
                let base = dot(sub(c.origem, b.origem), b.eixo);
                let (x, y) = (base + s * c.h0, base + s * c.h1);
                (x.min(y), x.max(y))
            };
            let mut faixas: Vec<(f64, f64)> = g.iter().map(|c| faixa(c)).collect();
            faixas.sort_by(|a, c| a.0.total_cmp(&c.0));
            let (lo, hi) = (
                faixas[0].0,
                faixas.iter().map(|f| f.1).fold(f64::MIN, f64::max),
            );
            let mut coberto = 0.;
            let mut ate = f64::MIN;
            for (a, c) in &faixas {
                coberto += (c - a.max(ate)).max(0.);
                ate = ate.max(*c);
            }
            let area: f64 = g.iter().map(|c| c.area).sum();
            let volta = area / (b.raio * coberto.max(1e-12)) / TAU;
            (volta >= VOLTA_COMPLETA).then(|| Furo {
                diametro: 2. * b.raio,
                eixo: canonico(b.eixo),
                inicio: soma(b.origem, b.eixo, lo),
                fim: soma(b.origem, b.eixo, hi),
                faces: g.iter().map(|c| c.face).collect(),
            })
        })
        .collect();
    furos.sort_by(|a, b| {
        a.diametro.total_cmp(&b.diametro).then(
            a.inicio
                .partial_cmp(&b.inicio)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    furos
}

/// Analisa um corpo. `None` em `caixa` vira erro: forma vazia não tem o que consultar.
pub fn analisar(s: &Shape) -> Result<Resumo, ErroApi> {
    let caixa = s
        .bbox()
        .ok_or_else(|| ErroApi::novo("sem_solido", "a forma está vazia"))?;
    let diagonal = dist(caixa.0, caixa.1).max(1e-6);
    let malha = s.mesh_angle(diagonal * 1e-3, ANGULO_MALHA)?;
    let massa = s.mass();
    let mut inercia = s.inertia().map_or([0.; 3], |i| i.principal);
    inercia.sort_by(f64::total_cmp);
    let (mut faces, mut cilindros) = (vec![], vec![]);
    for (i, f) in malha.face_info.iter().enumerate() {
        let mut face = Face {
            tipo: f.kind,
            area: f.area,
            ponto: f.point,
            direcao: f.dir,
            raio: None,
        };
        match f.kind {
            1 => {
                let c = s.cylinder_at(f.point)?;
                let interno = olha_para_o_eixo(&malha, i, c.origin, c.dir).unwrap_or(c.internal);
                face.raio = Some(c.radius);
                cilindros.push(CilindroFace {
                    face: i,
                    origem: c.origin,
                    eixo: c.dir,
                    raio: c.radius,
                    h0: c.h0,
                    h1: c.h1,
                    interno,
                    area: f.area,
                });
            }
            2..=4 => face.raio = s.face_props(f.point).ok().map(|p| p.radius),
            _ => {}
        }
        faces.push(face);
    }
    let furos = achar_furos(&cilindros);
    Ok(Resumo {
        caixa,
        volume: massa.volume,
        area: massa.area,
        centroide: massa.center,
        inercia,
        faces,
        cilindros,
        furos,
        malha,
    })
}

fn aresta_json(m: &Mesh, e: usize) -> Value {
    let vizinhas: Vec<u32> = m
        .edge_faces
        .get(e)
        .map(|p| p.iter().copied().filter(|f| *f != NO_FACE).collect())
        .unwrap_or_default();
    let mut v = match m.edge_formula(e) {
        Some(EdgeFormula::Line { a, b }) => json!({ "tipo": "reta", "a": r6v(a), "b": r6v(b) }),
        Some(EdgeFormula::Conic { c, u, v, t0, t1 })
            if (dot(u, u) - dot(v, v)).abs() < 1e-9 * dot(u, u).max(1.) =>
        {
            let ponto = |t: f64| soma(soma(c, u, t.cos()), v, t.sin());
            let n = fenkra_kernel::cross(u, v);
            let k = dot(n, n).sqrt().max(1e-12);
            let tipo = if (t1 - t0 - TAU).abs() < 1e-6 {
                "circulo"
            } else {
                "arco"
            };
            json!({ "tipo": tipo, "centro": r6v(c), "raio": r6(dot(u, u).sqrt()), "normal": r6v(n.map(|x| x / k)), "inicio": r6v(ponto(t0)), "fim": r6v(ponto(t1)), "angulo": r6((t1 - t0).to_degrees()) })
        }
        Some(EdgeFormula::Conic { c, .. }) => json!({ "tipo": "elipse", "centro": r6v(c) }),
        None => json!({ "tipo": "outra" }),
    };
    v["indice"] = json!(e);
    v["faces"] = json!(vizinhas);
    if let Some(p) = m.edge_mids.get(e * 3..e * 3 + 3) {
        v["meio"] = json!(r6v([p[0] as f64, p[1] as f64, p[2] as f64]));
    }
    v
}

/// Planos do corpo: faces de mesma normal (sem sentido) e mesma distância à origem.
fn planos_json(r: &Resumo) -> Vec<Value> {
    let mut planos: Vec<([f64; 3], f64, f64, Vec<usize>)> = vec![];
    for (i, f) in r.faces.iter().enumerate().filter(|(_, f)| f.tipo == 0) {
        let n = canonico(f.direcao);
        let d = dot(n, f.ponto);
        match planos
            .iter_mut()
            .find(|p| dot(p.0, n) > 1. - 1e-9 && (p.1 - d).abs() < 1e-6)
        {
            Some(p) => {
                p.2 += f.area;
                p.3.push(i);
            }
            None => planos.push((n, d, f.area, vec![i])),
        }
    }
    planos.iter().map(|(n, d, a, faces)| json!({ "normal": r6v(*n), "deslocamento": r6(*d), "area": r6(*a), "faces": faces })).collect()
}

impl Resumo {
    /// JSON das seções pedidas. `limite` corta faces e arestas e avisa em `cortado`.
    pub fn json(&self, secoes: &[&str], limite: usize) -> Value {
        let mut v = json!({});
        let mut contagem = [0usize; 6];
        self.faces
            .iter()
            .for_each(|f| contagem[f.tipo.min(5) as usize] += 1);
        v["contagem"] = json!({
            "faces": self.faces.len(), "arestas": self.malha.edge_ranges.len(), "furos": self.furos.len(),
            "planos": contagem[0], "cilindros": contagem[1], "cones": contagem[2], "esferas": contagem[3], "toros": contagem[4], "livres": contagem[5],
        });
        let (lo, hi) = self.caixa;
        let mut cortado = false;
        for s in secoes {
            match *s {
                "caixa" => {
                    v["caixa"] =
                        json!({ "min": r6v(lo), "max": r6v(hi), "tamanho": r6v(sub(hi, lo)) })
                }
                "massa" => {
                    v["massa"] = json!({ "volume": r6(self.volume), "area": r6(self.area), "centroide": r6v(self.centroide), "inercia": r6v(self.inercia) })
                }
                "faces" => {
                    cortado |= self.faces.len() > limite;
                    let mut arestas: Vec<Vec<usize>> = vec![vec![]; self.faces.len()];
                    for (e, par) in self.malha.edge_faces.iter().enumerate() {
                        for f in par.iter().filter(|f| **f != NO_FACE) {
                            if let Some(l) = arestas.get_mut(*f as usize) {
                                if !l.contains(&e) {
                                    l.push(e);
                                }
                            }
                        }
                    }
                    let lista: Vec<Value> = self.faces.iter().enumerate().take(limite).map(|(i, f)| {
                        let mut j = json!({ "indice": i, "tipo": nome_do_tipo(f.tipo), "area": r6(f.area), "ponto": r6v(f.ponto), "direcao": r6v(f.direcao), "arestas": arestas[i] });
                        if let Some(r) = f.raio {
                            j["raio"] = json!(r6(r));
                        }
                        j
                    }).collect();
                    v["faces"] = json!(lista);
                }
                "arestas" => {
                    let total = self.malha.edge_ranges.len();
                    cortado |= total > limite;
                    v["arestas"] = json!((0..total.min(limite))
                        .map(|e| aresta_json(&self.malha, e))
                        .collect::<Vec<_>>());
                }
                "planos" => v["planos"] = json!(planos_json(self)),
                "cilindros" => {
                    v["cilindros"] = json!(self.cilindros.iter().map(|c| json!({
                        "face": c.face, "origem": r6v(c.origem), "eixo": r6v(c.eixo), "raio": r6(c.raio), "diametro": r6(2. * c.raio),
                        "inicio": r6v(soma(c.origem, c.eixo, c.h0)), "fim": r6v(soma(c.origem, c.eixo, c.h1)), "interno": c.interno, "area": r6(c.area),
                    })).collect::<Vec<_>>());
                    v["furos"] = json!(self.furos.iter().map(|f| json!({ "diametro": r6(f.diametro), "eixo": r6v(f.eixo), "inicio": r6v(f.inicio), "fim": r6v(f.fim), "comprimento": r6(f.comprimento()), "faces": f.faces })).collect::<Vec<_>>());
                }
                _ => {}
            }
        }
        v["cortado"] = json!(cortado);
        v
    }
}
