// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cadeia conectada e seleção por janela no sketch (B19).
//!
//! Cadeia é o caminho de curvas ligadas ponta a ponta. Ela para onde o caminho se divide, porque
//! Offset e Filete precisam de um percurso único, e não mistura construção com geometria normal.
//! Pontas ligadas por restrição de coincidência contam como o mesmo ponto.

use crate::sketch_relimita::{cruzamentos, traco, Traco};
use crate::sketch_ui::{curve_uv, Ent};
use eframe::egui::{Pos2, Rect};
use fenkra_core::sketch::{Curve, Kind};
use fenkra_core::{Id, SketchData};
use std::collections::{HashMap, HashSet};

type P = [f64; 2];

fn dist(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// As duas pontas da curva. Círculo e ponto não ligam com nada.
fn pontas(sk: &SketchData, c: Id) -> Option<(Id, Id)> {
    match &sk.curve(c)?.geom {
        Curve::Line { a, b } => Some((*a, *b)),
        Curve::Arc { s, e, .. } => Some((*s, *e)),
        Curve::Spline { pts } => Some((*pts.first()?, *pts.last()?)),
        Curve::Ellipse { c, s, e, .. } if s != c => Some((*s, *e)),
        _ => None,
    }
}

/// Representante de cada ponto ligado por coincidência.
fn representantes(sk: &SketchData) -> HashMap<Id, Id> {
    fn raiz(m: &mut HashMap<Id, Id>, x: Id) -> Id {
        let p = *m.entry(x).or_insert(x);
        if p == x {
            return x;
        }
        let r = raiz(m, p);
        m.insert(x, r);
        r
    }
    let mut m = HashMap::new();
    for k in &sk.constraints {
        if let Kind::Coincident(a, b) = k.kind {
            let (ra, rb) = (raiz(&mut m, a), raiz(&mut m, b));
            m.insert(ra, rb);
        }
    }
    let chaves: Vec<Id> = m.keys().copied().collect();
    for k in chaves {
        raiz(&mut m, k);
    }
    m
}

/// Percurso por curvas ligadas. `invertida`: a curva é percorrida do fim para o começo.
#[derive(Clone, Debug, PartialEq)]
pub struct Caminho {
    pub passos: Vec<(Id, bool)>,
    pub fechado: bool,
}

impl Caminho {
    pub fn curvas(&self) -> Vec<Id> {
        self.passos.iter().map(|x| x.0).collect()
    }
}

/// Caminho que passa por `curva`, só pelas curvas que `vale` aceita.
fn percorre(sk: &SketchData, curva: Id, vale: &dyn Fn(Id) -> bool) -> Caminho {
    let reps = representantes(sk);
    let rep = |p: Id| reps.get(&p).copied().unwrap_or(p);
    let tipo = sk.curve(curva).map(|c| c.construction);
    let mut chegam: HashMap<Id, Vec<Id>> = HashMap::new();
    for c in sk.curves.iter().filter(|c| Some(c.construction) == tipo && vale(c.id)) {
        if let Some((a, b)) = pontas(sk, c.id) {
            chegam.entry(rep(a)).or_default().push(c.id);
            chegam.entry(rep(b)).or_default().push(c.id);
        }
    }
    let Some((a0, b0)) = pontas(sk, curva) else { return Caminho { passos: vec![(curva, false)], fechado: false } };
    let (a0, b0) = (rep(a0), rep(b0));
    // anda a partir de `ponto`, saindo de `de`; devolve os passos e se voltou à curva de partida
    let anda = |mut ponto: Id, mut de: Id, vistas: &mut HashSet<Id>| {
        let mut passos = vec![];
        loop {
            let outras: Vec<Id> = chegam.get(&ponto).map(|v| v.iter().copied().filter(|c| *c != de).collect()).unwrap_or_default();
            let [prox] = outras[..] else { return (passos, false) };
            if prox == curva {
                return (passos, true);
            }
            if !vistas.insert(prox) {
                return (passos, false);
            }
            let Some((a, b)) = pontas(sk, prox) else { return (passos, false) };
            let (a, b) = (rep(a), rep(b));
            let inv = b == ponto && a != ponto;
            passos.push((prox, inv));
            ponto = if inv { a } else { b };
            de = prox;
        }
    };
    let mut vistas = HashSet::from([curva]);
    let (frente, fechado) = anda(b0, curva, &mut vistas);
    let mut passos = vec![];
    if !fechado {
        let (atras, _) = anda(a0, curva, &mut vistas);
        // andar para trás inverte o sentido de cada passo
        passos.extend(atras.into_iter().rev().map(|(c, inv)| (c, !inv)));
    }
    passos.push((curva, false));
    passos.extend(frente);
    Caminho { passos, fechado }
}

/// A cadeia conectada que passa pela curva.
pub fn cadeia(sk: &SketchData, curva: Id) -> Caminho {
    percorre(sk, curva, &|_| true)
}

/// O caminho formado exatamente por estas curvas, se elas formarem um.
pub fn caminho_das(sk: &SketchData, curvas: &[Id]) -> Option<Caminho> {
    let set: HashSet<Id> = curvas.iter().copied().collect();
    let c = percorre(sk, *curvas.first()?, &|x| set.contains(&x));
    (c.passos.len() == set.len()).then_some(c)
}

/// A curva da lista mais perto do ponto.
pub fn mais_perto(sk: &SketchData, curvas: &[Id], p: P) -> Option<Id> {
    curvas.iter().copied().filter_map(|c| traco(sk, c).map(|t| (dist(p, t.em(t.param(p).clamp(0., 1.))), c))).min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1)
}

/// Distância com sinal do ponto ao caminho: positiva à esquerda de quem percorre.
pub fn lado_e_distancia(sk: &SketchData, cam: &Caminho, p: P) -> Option<f64> {
    let c = mais_perto(sk, &cam.curvas(), p)?;
    let inv = cam.passos.iter().find(|x| x.0 == c)?.1;
    let d = match traco(sk, c)? {
        Traco::Reta { a, b } => ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])) / dist(a, b).max(1e-12),
        // no sentido anti-horário a esquerda aponta para o centro
        Traco::Arco { c, r, .. } => r - dist(p, c),
        _ => return None,
    };
    Some(if inv { -d } else { d })
}

/// Curva paralela de um passo do caminho, no sentido do percurso.
#[derive(Clone, Copy)]
enum Par {
    Reta { a: P, b: P },
    Arco { c: P, r: f64, ini: P, fim: P },
}

impl Par {
    fn ini(&self) -> P {
        match self {
            Par::Reta { a, .. } => *a,
            Par::Arco { ini, .. } => *ini,
        }
    }
    fn fim(&self) -> P {
        match self {
            Par::Reta { b, .. } => *b,
            Par::Arco { fim, .. } => *fim,
        }
    }
    /// Versão sem limites, para achar onde as paralelas vizinhas se encontram.
    fn sem_limite(&self, alcance: f64) -> Traco {
        match *self {
            Par::Reta { a, b } => {
                let l = dist(a, b).max(1e-12);
                let k = alcance / l;
                let d = [(b[0] - a[0]) * k, (b[1] - a[1]) * k];
                Traco::Reta { a: [a[0] - d[0], a[1] - d[1]], b: [b[0] + d[0], b[1] + d[1]] }
            }
            Par::Arco { c, r, .. } => Traco::Arco { c, r, a0: 0., varre: std::f64::consts::TAU },
        }
    }
}

/// Offset do caminho inteiro a `d` (positivo à esquerda de quem percorre). Paralelas vizinhas ficam
/// unidas no ponto em que se cruzam; sem cruzamento, uma linha fecha o vão. O percurso paralelo é o
/// mesmo do original: cada paralela guarda relação com a sua curva (paralela e distância, ou centro
/// comum e raio). Devolve as curvas novas.
pub fn offset_caminho(sk: &mut SketchData, al: &mut impl FnMut() -> Id, cam: &Caminho, d: f64, construction: bool) -> Result<Vec<Id>, &'static str> {
    if d.abs() < 1e-9 {
        return Err("o cursor está sobre a cadeia: afaste para dar a distância");
    }
    let mut pares = vec![];
    for (id, inv) in &cam.passos {
        pares.push(match (sk.curve(*id).map(|c| c.geom.clone()), traco(sk, *id)) {
            (Some(Curve::Line { .. }), Some(Traco::Reta { a, b })) => {
                let (a, b) = if *inv { (b, a) } else { (a, b) };
                let l = dist(a, b).max(1e-12);
                let n = [-(b[1] - a[1]) / l, (b[0] - a[0]) / l];
                Par::Reta { a: [a[0] + n[0] * d, a[1] + n[1] * d], b: [b[0] + n[0] * d, b[1] + n[1] * d] }
            }
            (Some(Curve::Arc { .. }), Some(t @ Traco::Arco { c, r, .. })) => {
                let r2 = if *inv { r + d } else { r - d };
                if r2 <= 1e-6 {
                    return Err("o offset passa do raio de um arco da cadeia");
                }
                let (s, e) = (t.em(0.), t.em(1.));
                let leva = |q: P| [c[0] + (q[0] - c[0]) * r2 / r, c[1] + (q[1] - c[1]) * r2 / r];
                let (ini, fim) = if *inv { (leva(e), leva(s)) } else { (leva(s), leva(e)) };
                Par::Arco { c, r: r2, ini, fim }
            }
            _ => return Err("offset de cadeia funciona em linhas e arcos"),
        });
    }
    let n = pares.len();
    let alcance: f64 = pares.iter().map(|p| dist(p.ini(), p.fim())).sum::<f64>() + d.abs() * 4. + 1.;
    // emendas: onde a paralela i encontra a i+1
    let par_de_emenda: Vec<usize> = if cam.fechado { (0..n).collect() } else { (0..n.saturating_sub(1)).collect() };
    let mut ini: Vec<P> = pares.iter().map(|p| p.ini()).collect();
    let mut fim: Vec<P> = pares.iter().map(|p| p.fim()).collect();
    let mut emendada = vec![false; n];
    for &i in &par_de_emenda {
        let j = (i + 1) % n;
        let (f, s) = (pares[i].fim(), pares[j].ini());
        let junta = [(f[0] + s[0]) / 2., (f[1] + s[1]) / 2.];
        if dist(f, s) < 1e-9 {
            (fim[i], ini[j], emendada[i]) = (junta, junta, true);
            continue;
        }
        let perto = cruzamentos(&pares[i].sem_limite(alcance), &pares[j].sem_limite(alcance)).into_iter().min_by(|a, b| dist(*a, junta).total_cmp(&dist(*b, junta)));
        if let Some(q) = perto {
            (fim[i], ini[j], emendada[i]) = (q, q, true);
        }
    }
    // uma reta que encurtou demais inverte de sentido: a paralela sumiria
    for (i, p) in pares.iter().enumerate() {
        if let Par::Reta { a, b } = p {
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [fim[i][0] - ini[i][0], fim[i][1] - ini[i][1]]);
            if u[0] * w[0] + u[1] * w[1] <= 1e-9 {
                return Err("o offset é grande demais para esta cadeia");
            }
        }
    }
    // pontos: uma emenda é um ponto só
    let mut p_ini: Vec<Option<Id>> = vec![None; n];
    let mut p_fim: Vec<Option<Id>> = vec![None; n];
    for &i in &par_de_emenda {
        if emendada[i] {
            let id = sk.add_point(al, fim[i][0], fim[i][1]);
            p_fim[i] = Some(id);
            p_ini[(i + 1) % n] = Some(id);
        }
    }
    let mut novas = vec![];
    let val = crate::app::fmt(d.abs());
    for (i, (id, inv)) in cam.passos.iter().enumerate() {
        let s = p_ini[i].unwrap_or_else(|| sk.add_point(al, ini[i][0], ini[i][1]));
        let e = p_fim[i].unwrap_or_else(|| sk.add_point(al, fim[i][0], fim[i][1]));
        (p_ini[i], p_fim[i]) = (Some(s), Some(e));
        match pares[i] {
            Par::Reta { .. } => {
                let c = sk.add_curve(al, Curve::Line { a: s, b: e }, construction);
                sk.add_constraint(al, Kind::Parallel(*id, c));
                sk.add_constraint(al, Kind::PointLineDistance(s, *id, val.clone()));
                novas.push(c);
            }
            Par::Arco { r, .. } => {
                let Some(Curve::Arc { c: ctr, .. }) = sk.curve(*id).map(|c| c.geom.clone()) else { continue };
                let c = sk.add_curve(al, if *inv { Curve::Arc { c: ctr, s: e, e: s } } else { Curve::Arc { c: ctr, s, e } }, construction);
                sk.add_constraint(al, Kind::Radius(c, crate::app::fmt(r)));
                novas.push(c);
            }
        }
    }
    // vão sem cruzamento: uma linha liga as pontas
    for &i in &par_de_emenda {
        if !emendada[i] {
            if let (Some(a), Some(b)) = (p_fim[i], p_ini[(i + 1) % n]) {
                novas.push(sk.add_curve(al, Curve::Line { a, b }, construction));
            }
        }
    }
    Ok(novas)
}

/// Cantos entre duas linhas das curvas dadas: os pontos em que Filete e Chanfro agem.
pub fn cantos(sk: &SketchData, curvas: &[Id]) -> Vec<Id> {
    let mut chegam: HashMap<Id, Vec<Id>> = HashMap::new();
    for c in &sk.curves {
        if let Curve::Line { a, b } = c.geom {
            chegam.entry(a).or_default().push(c.id);
            chegam.entry(b).or_default().push(c.id);
        }
    }
    let mut v: Vec<Id> = vec![];
    for c in curvas {
        if let Some(Curve::Line { a, b }) = sk.curve(*c).map(|x| &x.geom) {
            for p in [*a, *b] {
                if !v.contains(&p) && chegam.get(&p).is_some_and(|l| l.len() == 2 && l.iter().all(|x| curvas.contains(x))) {
                    v.push(p);
                }
            }
        }
    }
    v
}

fn dentro(r: Rect, p: Pos2) -> bool {
    r.contains(p)
}

/// O trecho toca o retângulo? Recorta o trecho pelos quatro lados (Liang-Barsky).
fn toca(r: Rect, a: Pos2, b: Pos2) -> bool {
    let (mut t0, mut t1) = (0f32, 1f32);
    let d = b - a;
    for (p, q) in [(-d.x, a.x - r.left()), (d.x, r.right() - a.x), (-d.y, a.y - r.top()), (d.y, r.bottom() - a.y)] {
        if p == 0. {
            if q < 0. {
                return false;
            }
        } else if p < 0. {
            t0 = t0.max(q / p);
        } else {
            t1 = t1.min(q / p);
        }
        if t0 > t1 {
            return false;
        }
    }
    true
}

/// O que a janela pega. Janela: só o que está todo dentro. Cruzada: também o que ela toca.
/// Pontos das pontas das curvas ficam de fora: a curva já os representa.
pub fn na_janela(sk: &SketchData, tela: &dyn Fn(P) -> Pos2, r: Rect, cruzada: bool) -> Vec<Ent> {
    let mut v = vec![];
    for c in &sk.curves {
        if let Curve::Point { p } = c.geom {
            if dentro(r, tela(sk.pos(p))) {
                v.push(Ent::Pt(p));
            }
            continue;
        }
        let q: Vec<Pos2> = curve_uv(sk, c).into_iter().map(tela).collect();
        let pega = if cruzada { q.windows(2).any(|w| toca(r, w[0], w[1])) } else { !q.is_empty() && q.iter().all(|p| dentro(r, *p)) };
        if pega {
            v.push(Ent::Cv(c.id));
        }
    }
    v
}

/// O arrasto para a esquerda é a cruzada; para a direita, a janela.
pub fn e_cruzada(inicio: Pos2, fim: Pos2) -> bool {
    fim.x < inicio.x
}

#[cfg(test)]
mod testes {
    use super::*;
    use eframe::egui::pos2;
    use std::collections::HashMap as Mapa;

    fn ids() -> impl FnMut() -> Id {
        let mut n = 3000u64;
        move || {
            n += 1;
            Id(n)
        }
    }

    /// Polilinha de linhas ligadas; `fecha` liga a última ponta à primeira.
    fn poli(sk: &mut SketchData, al: &mut impl FnMut() -> Id, q: &[P], fecha: bool) -> Vec<Id> {
        let p: Vec<Id> = q.iter().map(|x| sk.add_point(al, x[0], x[1])).collect();
        let mut v: Vec<Id> = p.windows(2).map(|w| sk.add_curve(al, Curve::Line { a: w[0], b: w[1] }, false)).collect();
        if fecha {
            v.push(sk.add_curve(al, Curve::Line { a: p[p.len() - 1], b: p[0] }, false));
        }
        v
    }

    #[test]
    fn duplo_clique_pega_a_cadeia_ate_a_bifurcacao() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[0., 0.], [10., 0.], [20., 0.], [30., 0.]], false);
        // ramo em (20, 0): a cadeia para ali, e a linha solta fica de fora
        let Curve::Line { b: junta, .. } = sk.curve(l[1]).unwrap().geom else { panic!() };
        let f = sk.add_point(&mut al, 20., 10.);
        sk.add_curve(&mut al, Curve::Line { a: junta, b: f }, false);
        poli(&mut sk, &mut al, &[[0., 50.], [10., 50.]], false);
        let c = cadeia(&sk, l[0]);
        assert_eq!(c.curvas(), vec![l[0], l[1]]);
        assert!(!c.fechado);
    }

    #[test]
    fn cadeia_fechada_e_sem_ordem_da_o_percurso() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]], true);
        let c = cadeia(&sk, l[2]);
        assert!(c.fechado);
        assert_eq!(c.passos.len(), 4);
        let mexida = [l[3], l[1], l[0], l[2]];
        let c2 = caminho_das(&sk, &mexida).unwrap();
        assert!(c2.fechado && c2.passos.len() == 4);
        assert!(caminho_das(&sk, &[l[0], l[2]]).is_none());
    }

    #[test]
    fn cadeia_passa_por_arco_e_pela_coincidencia() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[-10., 0.], [0., 0.]], false);
        let Curve::Line { b: p, .. } = sk.curve(l[0]).unwrap().geom else { panic!() };
        // o arco termina no ponto da linha: percorrido ao contrário
        let (c, s) = (sk.add_point(&mut al, 0., 5.), sk.add_point(&mut al, 5., 5.));
        let a = sk.add_curve(&mut al, Curve::Arc { c, s, e: p }, false);
        // outra linha com ponto próprio, ligado por coincidência
        let q = poli(&mut sk, &mut al, &[[5., 5.], [5., 20.]], false);
        let Curve::Line { a: r, .. } = sk.curve(q[0]).unwrap().geom else { panic!() };
        sk.add_constraint(&mut al, Kind::Coincident(r, s));
        let k = cadeia(&sk, l[0]);
        assert_eq!(k.passos, vec![(l[0], false), (a, true), (q[0], false)]);
    }

    #[test]
    fn construcao_nao_entra_na_cadeia_da_geometria() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[0., 0.], [10., 0.], [20., 0.]], false);
        sk.curves.iter_mut().find(|c| c.id == l[1]).unwrap().construction = true;
        assert_eq!(cadeia(&sk, l[0]).curvas(), vec![l[0]]);
    }

    #[test]
    fn offset_de_quadrado_fecha_nos_cantos_e_fica_cotado() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]], true);
        let cam = cadeia(&sk, l[0]);
        // anti-horário: esquerda é para dentro
        let d = lado_e_distancia(&sk, &cam, [5., 2.]).unwrap();
        assert!((d - 2.).abs() < 1e-9);
        let novas = offset_caminho(&mut sk, &mut al, &cam, d, false).unwrap();
        assert_eq!(novas.len(), 4);
        // quatro pontos novos, um por canto: o contorno de dentro fecha
        assert_eq!(sk.points.len(), 8);
        let (loops, _) = sk.loops();
        assert_eq!(loops.len(), 2);
        let area: f64 = novas
            .iter()
            .map(|c| {
                let t = traco(&sk, *c).unwrap();
                t.comprimento()
            })
            .sum();
        assert!((area - 24.).abs() < 1e-9);
        let rel = sk.solve(&Mapa::new(), &[]);
        assert!(rel.failing.is_empty(), "{rel:?}");
    }

    #[test]
    fn offset_de_linha_arco_linha_mantem_a_tangencia() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        // L de canto arredondado: reta, arco de 90° com centro em (10, 10), reta
        let (a, b) = (sk.add_point(&mut al, 0., 0.), sk.add_point(&mut al, 10., 0.));
        let (c, e, f) = (sk.add_point(&mut al, 10., 10.), sk.add_point(&mut al, 20., 10.), sk.add_point(&mut al, 20., 20.));
        let l1 = sk.add_curve(&mut al, Curve::Line { a, b }, false);
        let arc = sk.add_curve(&mut al, Curve::Arc { c, s: b, e }, false);
        let l2 = sk.add_curve(&mut al, Curve::Line { a: e, b: f }, false);
        let cam = cadeia(&sk, arc);
        assert_eq!(cam.curvas(), vec![l1, arc, l2]);
        // para fora: 3 à direita do percurso
        let novas = offset_caminho(&mut sk, &mut al, &cam, -3., false).unwrap();
        assert_eq!(novas.len(), 3);
        let t = traco(&sk, novas[1]).unwrap();
        assert!((t.comprimento() - 13. * std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(dist(t.em(0.), [10., -3.]) < 1e-9 && dist(t.em(1.), [23., 10.]) < 1e-9);
        assert!(sk.solve(&Mapa::new(), &[]).failing.is_empty());
    }

    #[test]
    fn offset_recusa_o_que_nao_cabe() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]], true);
        let cam = cadeia(&sk, l[0]);
        assert!(offset_caminho(&mut sk.clone(), &mut al, &cam, 6., false).is_err());
        assert!(offset_caminho(&mut sk, &mut al, &cam, 0., false).is_err());
    }

    #[test]
    fn cantos_da_cadeia_ignoram_as_pontas() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[0., 0.], [10., 0.], [10., 10.], [20., 10.]], false);
        assert_eq!(cantos(&sk, &l).len(), 2);
        let q = poli(&mut sk, &mut al, &[[0., 30.], [10., 30.], [10., 40.], [0., 40.]], true);
        assert_eq!(cantos(&sk, &q).len(), 4);
    }

    #[test]
    fn janela_pega_o_que_esta_dentro_e_a_cruzada_o_que_toca() {
        let (mut sk, mut al) = (SketchData::default(), ids());
        let l = poli(&mut sk, &mut al, &[[1., 1.], [5., 1.]], false);
        let m = poli(&mut sk, &mut al, &[[5., 5.], [50., 5.]], false);
        let c = sk.add_point(&mut al, 100., 100.);
        let circ = sk.add_curve(&mut al, Curve::Circle { c, r: 3. }, false);
        let p = sk.add_point(&mut al, 2., 2.);
        sk.add_curve(&mut al, Curve::Point { p }, false);
        let tela = |q: P| pos2(q[0] as f32, q[1] as f32);
        let r = Rect::from_min_max(pos2(0., 0.), pos2(10., 10.));
        let janela = na_janela(&sk, &tela, r, false);
        assert!(janela.contains(&Ent::Cv(l[0])) && janela.contains(&Ent::Pt(p)) && !janela.contains(&Ent::Cv(m[0])));
        let cruzada = na_janela(&sk, &tela, r, true);
        assert!(cruzada.contains(&Ent::Cv(m[0])) && !cruzada.contains(&Ent::Cv(circ)));
        // a cruzada dentro do círculo não toca nele; encostando, pega
        let dentro_do_circulo = Rect::from_min_max(pos2(99., 99.), pos2(101., 101.));
        assert!(na_janela(&sk, &tela, dentro_do_circulo, true).is_empty());
        let encosta = Rect::from_min_max(pos2(99., 99.), pos2(104., 101.));
        assert!(na_janela(&sk, &tela, encosta, true).contains(&Ent::Cv(circ)));
        assert!(e_cruzada(pos2(10., 0.), pos2(2., 0.)) && !e_cruzada(pos2(0., 0.), pos2(2., 0.)));
    }
}
