// SPDX-License-Identifier: LGPL-2.1-or-later
//! B579: nervura de asa e caverna de fuselagem. Aba em perfil curvo fechado.
//!
//! A web é uma chapa com o contorno de uma spline fechada (aerofólio NACA 2412 de corda 2000 e 1500 mm; oval de
//! fuselagem de 1200 × 900 mm). A aba contorna o perímetro inteiro, por aresta (aba curva, ângulo constante) e
//! por superfície (suporte = superfície que contém o contorno: extrusão da spline, loft entre aerofólios ou
//! entre ovais), para os dois lados da chapa, dobrada e planificada.
//!
//! Cada resultado é conferido contra a conta feita à parte, sem o motor (curva fechada convexa, giro 2π):
//! - web: área do contorno (fórmula do polígono fino) × espessura;
//! - aba dobrada: seção (setor da dobra mais parede, área `A`, momento `Q` na direção para fora) varrida pelo
//!   contorno; pelo teorema de Pappus o volume é `A·L + Q·Φ` (`L` o perímetro, `Φ = 2π`);
//! - aba planificada: faixa de largura `D = ba + comprimento` para fora do contorno: `t·(D·L + D²·Φ/2)`.
//!
//! A integração de volume do kernel (`mass`) erra até 13 % numa face plana limitada por uma spline longa
//! (medido: web da nervura de 1500 mm dava 239 602 mm³ contra 275 962 da malha); por isso os volumes são
//! medidos na malha (teorema da divergência), que não depende dela.

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{final_shape, recompute, Status};
use std::time::Instant;

const K: f64 = 0.44;

fn f(x: f64) -> String {
    format!("{x}")
}

/// Contorno NACA 2412 de corda `c`, bordo de fuga agudo: do bordo de fuga pelo extradorso até o bordo
/// de ataque e de volta pelo intradorso, `n` pontos por lado, espaçamento em cosseno.
pub fn naca2412(c: f64, n: usize) -> Vec<[f64; 2]> {
    let (m, p, tt) = (0.02, 0.4, 0.12);
    let yt = |x: f64| 5. * tt * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1036 * x.powi(4));
    let yc = |x: f64| if x < p { m / (p * p) * (2. * p * x - x * x) } else { m / ((1. - p) * (1. - p)) * ((1. - 2. * p) + 2. * p * x - x * x) };
    let dyc = |x: f64| if x < p { 2. * m / (p * p) * (p - x) } else { 2. * m / ((1. - p) * (1. - p)) * (p - x) };
    let xs: Vec<f64> = (0..=n).map(|i| (1. - (std::f64::consts::PI * i as f64 / n as f64).cos()) / 2.).collect();
    let pt = |x: f64, up: f64| {
        let th = dyc(x).atan();
        [(x - up * yt(x) * th.sin()) * c, (yc(x) + up * yt(x) * th.cos()) * c]
    };
    let mut v: Vec<[f64; 2]> = xs.iter().rev().map(|x| pt(*x, 1.)).collect();
    v.extend(xs.iter().skip(1).take(n - 1).map(|x| pt(*x, -1.)));
    v
}

/// Contorno de nervura NACA 2412: perfil cortado em 97 % da corda (a nervura termina na longarina traseira ou
/// na faixa do bordo de fuga) e fechado por uma ponta elíptica lisa, sem canto vivo. Começa no bordo de ataque,
/// sobe pelo extradorso e volta pelo intradorso.
pub fn naca2412_rombo(c: f64, n: usize) -> Vec<[f64; 2]> {
    let (m, p, tt, xc) = (0.02, 0.4, 0.12, 0.97);
    let yt = |x: f64| 5. * tt * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1036 * x.powi(4));
    let yc = |x: f64| if x < p { m / (p * p) * (2. * p * x - x * x) } else { m / ((1. - p) * (1. - p)) * ((1. - 2. * p) + 2. * p * x - x * x) };
    let dyc = |x: f64| if x < p { 2. * m / (p * p) * (p - x) } else { 2. * m / ((1. - p) * (1. - p)) * (p - x) };
    let xs: Vec<f64> = (0..=n).map(|i| xc * (1. - (std::f64::consts::PI * i as f64 / n as f64).cos()) / 2.).collect();
    let pt = |x: f64, up: f64| {
        let th = dyc(x).atan();
        [(x - up * yt(x) * th.sin()) * c, (yc(x) + up * yt(x) * th.cos()) * c]
    };
    let mut v: Vec<[f64; 2]> = xs.iter().map(|x| pt(*x, 1.)).collect();
    // ponta elíptica entre os dois lados do perfil em xc: semieixo vertical = meia espessura, horizontal 1,2 vezes isso
    let (u, l) = (pt(xc, 1.), pt(xc, -1.));
    let (cx, cy, h) = ((u[0] + l[0]) / 2., (u[1] + l[1]) / 2., (u[1] - l[1]).abs() / 2.);
    for g in [60f64, 30., 0., -30., -60.] {
        v.push([cx + 1.2 * h * g.to_radians().cos(), cy + h * g.to_radians().sin()]);
    }
    v.extend(xs.iter().rev().take(n).map(|x| pt(*x, -1.)));
    v
}

/// Elipse de semieixos `a` (x) e `b` (y), `n` pontos.
pub fn oval(a: f64, b: f64, n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / n as f64;
            [a * t.cos(), b * t.sin()]
        })
        .collect()
}

/// Combinação linear de dois contornos de mesma contagem de pontos (`u` de 0 a 1), com deslocamento em x.
fn mistura(a: &[[f64; 2]], b: &[[f64; 2]], dx: f64, u: f64) -> Vec<[f64; 2]> {
    a.iter().zip(b).map(|(p, q)| [p[0] * (1. - u) + (q[0] + dx) * u, p[1] * (1. - u) + q[1] * u]).collect()
}

/// Sketch com uma spline fechada pelos pontos (a ponta repete o primeiro ponto).
pub fn sk_spline_fechada(d: &mut Document, plane: PlaneRef, pts: &[[f64; 2]]) -> Id {
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let mut ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
    ids.push(ids[0]);
    data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

#[derive(Clone, Debug)]
pub enum Suporte {
    /// extrusão da própria spline do contorno (o suporte passa exatamente pela borda da web)
    Extrusao,
    /// loft entre dois contornos fechados em z = 0 e z = `z1` (a web fica numa estação do meio)
    Loft { a: Vec<[f64; 2]>, b: Vec<[f64; 2]>, z1: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Via {
    Superficie,
    Aresta,
}

#[derive(Clone, Debug)]
pub struct Caso {
    pub nome: String,
    pub pts: Vec<[f64; 2]>,
    /// altura do plano da web
    pub z0: f64,
    pub t: f64,
    pub r: f64,
    pub len: f64,
    pub via: Via,
    pub flip: bool,
    pub suporte: Suporte,
    pub flat: bool,
}

impl Caso {
    pub fn dev(&self) -> f64 {
        std::f64::consts::FRAC_PI_2 * (self.r + K * self.t) + self.len
    }
}

fn pts3(p: &[[f64; 2]], z: f64) -> Vec<PointRef> {
    p.iter().map(|q| PointRef::Fixed([q[0], q[1], z])).collect()
}

/// Os pontos da aresta de cima da web, como o app os guarda: a polilinha da aresta do B-rep, com o ponto
/// médio que o kernel dá à aresta no meio da lista (o comando acha a aresta pelo elemento do meio e leva a
/// cadeia inteira pelo quanto ele andou).
pub fn aresta_da_web(d: &Document, z: f64) -> Vec<[f64; 3]> {
    let forma = final_shape(d).expect("forma da web");
    let topo = forma.topology().expect("topologia");
    let mut melhor: (Vec<[f64; 3]>, [f64; 3]) = (vec![], [0.; 3]);
    for m in &topo.edge_mids {
        if let Ok(e) = forma.edge_at(*m) {
            if e.len() > melhor.0.len() && e.iter().all(|q| (q[2] - z).abs() < 1e-6) {
                melhor = (e, *m);
            }
        }
    }
    let (mut e, m) = melhor;
    assert!(e.len() > 8, "a aresta de cima da web não foi lida");
    // aresta fechada: gira a lista para o ponto médio do kernel ficar no meio
    let fechada = e[0] == e[e.len() - 1] || (e[0][0] - e[e.len() - 1][0]).hypot(e[0][1] - e[e.len() - 1][1]) < 1e-6;
    if fechada {
        e.pop();
        if e.len() % 2 == 1 {
            e.remove(e.len() / 4);
        }
        let n = e.len();
        let j = (0..n).min_by(|a, b| dist3(e[*a], m).total_cmp(&dist3(e[*b], m))).unwrap();
        e.rotate_left((j + n / 2) % n);
        e.push(e[0]);
        let meio = e.len() / 2;
        e[meio] = m;
    }
    e
}

fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

pub fn monta(k: &Caso) -> (Document, Id) {
    let mut d = Document::new("nervura");
    d.sheet = Some(SheetParams { thickness: f(k.t), radius: f(k.r), k: f(K), flat: k.flat, ..Default::default() });
    let plano = if k.z0 == 0. { PlaneRef::Origin(OriginPlane::XY) } else { PlaneRef::Datum(d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XY), offset: f(k.z0) })) };
    let sk = sk_spline_fechada(&mut d, plano, &k.pts);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let ztopo = k.z0 + k.t;
    match k.via {
        Via::Aresta => {
            let edge = aresta_da_web(&d, ztopo);
            let aba = d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: f(k.len), flip: k.flip, angle: "90".into(), radius: None, side: 0, edge_key: None, so_aresta: false, reta: None }));
            (d, aba)
        }
        Via::Superficie => {
            let s = match &k.suporte {
                Suporte::Extrusao => {
                    let curva = d.add_geometry(RefKind::Spline { points: pts3(&k.pts, k.z0), closed: true, tangents: vec![] });
                    let ext = (k.pts.iter().map(|p| p[0].abs().max(p[1].abs())).fold(0., f64::max) * 2.).max(100.);
                    d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: f(ext), length2: f(ext), until: None, until2: None, mais: vec![] })
                }
                Suporte::Loft { a, b, z1 } => {
                    let ca = d.add_geometry(RefKind::Spline { points: pts3(a, 0.), closed: true, tangents: vec![] });
                    let cb = d.add_geometry(RefKind::Spline { points: pts3(b, *z1), closed: true, tangents: vec![] });
                    d.add_geometry(RefKind::SurfLoft { sections: vec![CurveRef::Ref(ca), CurveRef::Ref(cb)], guide: None, ruled: true, closed: false, opts: Default::default() })
                }
            };
            // o ponto da face base: o centro do contorno (o primeiro ponto cai na borda)
            let c = k.pts.iter().fold([0., 0.], |a, p| [a[0] + p[0] / k.pts.len() as f64, a[1] + p[1] / k.pts.len() as f64]);
            let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(s), face: [c[0], c[1], ztopo], length: f(k.len), radius: None, flip: k.flip, keep_base: false, mode: 0, angle: "90".into(), limites: Default::default(), side: 0, inv_lado: false }));
            (d, aba)
        }
    }
}

/// Volume pela malha (teorema da divergência nos triângulos).
pub fn volume_da_malha(forma: &Shape, deflexao: f64) -> f64 {
    let m = forma.mesh(deflexao).expect("malha");
    let p = |i: u32| {
        let k = i as usize * 3;
        [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
    };
    let mut v = 0.;
    for t in m.indices.chunks_exact(3) {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        v += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]);
    }
    v / 6.
}

/// Caixa dos vértices de uma malha fina.
pub fn caixa_fina(forma: &Shape, deflexao: f64) -> ([f64; 3], [f64; 3]) {
    let m = forma.mesh(deflexao).expect("malha");
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    (lo, hi)
}

/// Área e perímetro de um contorno fechado (polígono).
pub fn area_e_perimetro(p: &[[f64; 2]]) -> (f64, f64) {
    let (mut a, mut l) = (0., 0.);
    for i in 0..p.len() {
        let (u, v) = (p[i], p[(i + 1) % p.len()]);
        a += u[0] * v[1] - v[0] * u[1];
        l += (v[0] - u[0]).hypot(v[1] - u[1]);
    }
    (a.abs() / 2., l)
}

/// Volumes teóricos de aba de 90° num contorno fechado convexo (giro 2π): web, total dobrada, total planificada.
pub fn teoria(cont: &[[f64; 2]], t: f64, r: f64, len: f64) -> (f64, f64, f64) {
    let (area, l) = area_e_perimetro(cont);
    let a = std::f64::consts::FRAC_PI_2;
    let dev = a * (r + K * t) + len;
    let sec = a * t * (r + t / 2.) + len * t;
    let q = ((r + t).powi(3) - r.powi(3)) / 3. * (1. - a.cos()) + len * t * ((r + t / 2.) * a.sin() + len / 2. * a.cos());
    let phi = std::f64::consts::TAU;
    let web = area * t;
    (web, web + sec * l + q * phi, web + t * (dev * l + dev * dev * phi / 2.))
}

pub struct Medida {
    pub status: Status,
    pub volume: f64,
    pub lo: [f64; 3],
    pub hi: [f64; 3],
    pub segundos: f64,
}

/// Monta, recalcula e confere o que vale para todo caso: status sem erro (aviso só de conformação), um corpo
/// só, forma válida e não vazia. Devolve a medição.
pub fn roda(k: &Caso) -> Medida {
    let (d, aba) = monta(k);
    let t0 = Instant::now();
    let r = recompute(&d);
    let segundos = t0.elapsed().as_secs_f64();
    let estado = if k.flat { "planificada" } else { "dobrada" };
    let status = r.status[&aba].clone();
    match &status {
        Status::Ok => {}
        Status::Warning(m) => assert!(m.contains("conformação") || m.contains("canto vivo"), "{}: {estado}: aviso inesperado: {m}", k.nome),
        outro => panic!("{}: {estado}: {outro:?}", k.nome),
    }
    assert_eq!(r.bodies.len(), 1, "{}: {estado}: um corpo só", k.nome);
    let forma = final_shape(&d).expect("forma final");
    assert!(forma.is_valid(), "{}: {estado}: forma inválida", k.nome);
    let volume = volume_da_malha(&forma, 0.05);
    let (lo, hi) = caixa_fina(&forma, 0.05);
    Medida { status, volume, lo, hi, segundos }
}

/// Confere a medição contra a teoria e escreve a linha da tabela.
fn confere(k: &Caso, cont: &[[f64; 2]], tol: f64) -> Medida {
    let m = roda(k);
    let (web, dobrada, plana) = teoria(cont, k.t, k.r, k.len);
    let teorico = if k.flat { plana } else { dobrada };
    let erro = (m.volume - teorico) / (teorico - web);
    let aviso = matches!(m.status, Status::Warning(_));
    println!(
        "| {} | {} | {} | {} | {:.0} | {:.0} | {:+.2} % | {:.1} s | {} |",
        k.nome,
        if k.via == Via::Aresta { "aresta" } else { "superfície" },
        if k.flip { "baixo" } else { "cima" },
        if k.flat { "planificada" } else { "dobrada" },
        m.volume,
        teorico,
        erro * 100.,
        m.segundos,
        if aviso { "aviso" } else { "ok" }
    );
    assert!(erro.abs() < tol, "{} ({}): volume {:.0} contra {:.0} (erro de {:.2} % do volume da aba)", k.nome, if k.flat { "planificada" } else { "dobrada" }, m.volume, teorico, erro * 100.);
    // caixa em z: a web de t e a aba que sobe (ou desce) t + r + comprimento da face de referência
    let extensao_z = if k.flat { k.t } else { k.t + k.r + k.len };
    assert!((m.hi[2] - m.lo[2] - extensao_z).abs() < 0.3, "{}: caixa em z {:.2} contra {:.2}", k.nome, m.hi[2] - m.lo[2], extensao_z);
    // caixa em x e y: a do contorno mais o quanto a aba avança para fora (r + t dobrada; D planificada)
    let fora = if k.flat { k.dev() } else { k.r + k.t };
    let (cx0, _) = cont.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
    let (cy0, cy1) = cont.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
    assert!((m.lo[0] - (cx0 - fora)).abs() < 1.5, "{}: x mínimo {:.2} contra {:.2}", k.nome, m.lo[0], cx0 - fora);
    assert!((m.lo[1] - (cy0 - fora)).abs() < 1.5 && (m.hi[1] - (cy1 + fora)).abs() < 1.5, "{}: y {:.2}..{:.2} contra {:.2}..{:.2}", k.nome, m.lo[1], m.hi[1], cy0 - fora, cy1 + fora);
    m
}

fn caso(nome: &str, pts: Vec<[f64; 2]>, t: f64, len: f64, via: Via, flip: bool, flat: bool) -> Caso {
    Caso { nome: nome.into(), pts, z0: 0., t, r: t, len, via, flip, suporte: Suporte::Extrusao, flat }
}

/// Nervura planificada: chapa de 1 mm com aba de 8 mm para cima; chapa de 2 mm com aba de 25 mm para baixo; por
/// aresta e por superfície, nas duas cordas. A faixa é o anel da conformação (borda livre deslocada de `D`).
#[test]
fn nervura_naca_planificada_aresta_e_superficie() {
    if !kernel_disponivel() {
        return;
    }
    let (c2000, c1500) = (naca2412_rombo(2000., 2000), naca2412_rombo(1500., 2000));
    for (via, flip) in [(Via::Aresta, false), (Via::Aresta, true), (Via::Superficie, false), (Via::Superficie, true)] {
        let (t, len) = if flip { (2., 25.) } else { (1., 8.) };
        confere(&caso(&format!("NACA 2412, corda 2000, t {t}, aba {len}"), naca2412_rombo(2000., 60), t, len, via.clone(), flip, true), &c2000, 0.06);
    }
    for via in [Via::Aresta, Via::Superficie] {
        confere(&caso("NACA 2412, corda 1500, t 1.5, aba 15", naca2412_rombo(1500., 60), 1.5, 15., via, false, true), &c1500, 0.06);
    }
}

/// Nervura dobrada (aba para cima, aresta): a varredura leva uns 8 s, mas a fusão com a web (spline de 139 pontos)
/// leva de 100 a 500 s e às vezes é recusada; fica de fora da bateria comum. Roda com `--ignored`.
#[test]
#[ignore = "lento: a fusão da aba dobrada com a web da nervura leva minutos"]
fn nervura_naca_dobrada_aresta() {
    if !kernel_disponivel() {
        return;
    }
    let cont = naca2412_rombo(2000., 2000);
    confere(&caso("NACA 2412, corda 2000, t 1, aba 8", naca2412_rombo(2000., 60), 1., 8., Via::Aresta, false, false), &cont, 0.06);
}

#[test]
fn caverna_oval_1200_por_900_aresta_e_superficie() {
    if !kernel_disponivel() {
        return;
    }
    let cont = oval(600., 450., 4000);
    for via in [Via::Aresta, Via::Superficie] {
        for flip in [false, true] {
            for flat in [false, true] {
                confere(&caso("oval 1200 x 900, t 1.2, aba 12", oval(600., 450., 96), 1.2, 12., via.clone(), flip, flat), &cont, 0.06);
            }
        }
    }
}

/// Asa enflechada: loft entre o aerofólio de corda 2000 e o de corda 1500 (envergadura 4000, recuo 600); a
/// nervura fica na estação 1500, e o suporte da aba é a superfície do loft.
#[test]
fn nervura_na_asa_enflechada_suporte_loft() {
    if !kernel_disponivel() {
        return;
    }
    let (a, b) = (naca2412_rombo(2000., 60), naca2412_rombo(1500., 60));
    let u = 1500. / 4000.;
    let estacao = mistura(&a, &b, 600., u);
    // o mesmo contorno fino da estação, para a teoria
    let cont = mistura(&naca2412_rombo(2000., 2000), &naca2412_rombo(1500., 2000), 600., u);
    for flat in [true] {
        let mut k = caso("asa enflechada 2000 -> 1500, estação 1500", estacao.clone(), 1.5, 15., Via::Superficie, false, flat);
        k.z0 = 1500.;
        k.suporte = Suporte::Loft { a: a.clone(), b: b.iter().map(|p| [p[0] + 600., p[1]]).collect(), z1: 4000. };
        confere(&k, &cont, 0.08);
    }
}

/// Fuselagem de dupla curvatura: loft entre ovais de 1200 × 900 e 800 × 600 em 3000 mm; a caverna fica na estação 1500.
#[test]
fn caverna_na_fuselagem_loft_de_ovais() {
    if !kernel_disponivel() {
        return;
    }
    let (a, b) = (oval(600., 450., 96), oval(400., 300., 96));
    let estacao = mistura(&a, &b, 0., 0.5);
    let cont = mistura(&oval(600., 450., 4000), &oval(400., 300., 4000), 0., 0.5);
    for flip in [false] {
        for flat in [false, true] {
            let mut k = caso("fuselagem 1200x900 -> 800x600, estação 1500", estacao.clone(), 1.2, 12., Via::Superficie, flip, flat);
            k.z0 = 1500.;
            k.suporte = Suporte::Loft { a: a.clone(), b: b.clone(), z1: 3000. };
            confere(&k, &cont, 0.08);
        }
    }
}

/// O nariz do aerofólio (raio de uns 24 mm) pede conformação além do limite: aviso com o trecho, o valor e o
/// comprimento de aba que cabe, nunca erro; e a oval grande, com aba baixa, fica dentro do limite.
#[test]
fn aviso_de_conformacao_no_nariz_e_no_bordo_de_fuga() {
    if !kernel_disponivel() {
        return;
    }
    let nervura = roda(&caso("nariz", naca2412_rombo(1500., 60), 1.5, 15., Via::Aresta, false, true));
    let Status::Warning(m) = &nervura.status else { panic!("esperava aviso de conformação, veio {:?}", nervura.status) };
    assert!(m.contains("enruga") && m.contains("alívio") && m.contains("aba nesse trecho"), "{m}");
    let oval_grande = roda(&caso("oval", oval(600., 450., 96), 1.2, 8., Via::Aresta, false, true));
    assert_eq!(oval_grande.status, Status::Ok, "{:?}", oval_grande.status);
}

/// Bordo de fuga agudo de verdade (spline fechada que se fecha num canto de 14°, como no perfil de papel): a aba
/// ou sai válida, ou dá erro que diz onde o perfil fecha demais e o que fazer; nunca corpo vazio ou inválido
/// com status Ok.
#[test]
fn bordo_de_fuga_agudo_sai_valido_ou_explica_o_trecho() {
    if !kernel_disponivel() {
        return;
    }
    for flat in [false, true] {
        let k = caso("NACA 2412 agudo, corda 1500", naca2412(1500., 60), 1.5, 15., Via::Aresta, false, flat);
        let (d, aba) = monta(&k);
        let r = recompute(&d);
        match &r.status[&aba] {
            Status::Error(m) => {
                assert!(m.contains("canto vivo") || m.contains("raio de curvatura"), "erro sem dizer o trecho: {m}");
                println!("bordo agudo ({}): {m}", if flat { "planificada" } else { "dobrada" });
            }
            outro => {
                let forma = final_shape(&d).expect("forma");
                assert!(forma.is_valid(), "status {outro:?} com forma inválida");
                let (web, dobrada, plana) = teoria(&naca2412(1500., 2000), 1.5, 1.5, 15.);
                let v = volume_da_malha(&forma, 0.02);
                assert!(v > web + 0.5 * ((if flat { plana } else { dobrada }) - web), "status {outro:?} mas sem a aba: {v:.0} contra {web:.0}");
            }
        }
    }
}

