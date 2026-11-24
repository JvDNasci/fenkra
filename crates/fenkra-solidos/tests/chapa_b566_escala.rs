// SPDX-License-Identifier: LGPL-2.1-or-later
//! B566: a aba por superfície em qualquer escala.
//!
//! O mesmo caso (web de `c` × 0,6 `c` relimitada por um suporte, aba pela linha de dobra) é montado da
//! corda de 10 mm à de 10 000 mm, com espessura, raio e comprimento proporcionais à corda e também com
//! espessura fixa (1 e 3 mm) em peça grande; ângulos de 45°, 90° e 135°; material na linha de dobra, por
//! fora e por dentro; com e sem limites nos lados; dobrada e planificada; suporte plano, curvo (dorso de um
//! perfil de asa de quatro dígitos, spline por 61 pontos) e curvo deslocado.
//!
//! Cada resultado é conferido contra a conta feita à parte, sem o motor:
//! - a linha de dobra (OML) teórica é a curva exata do suporte no plano da web, amostrada fina (20 001
//!   pontos levados para cima da spline do kernel);
//! - web: a área sob a OML de x = 0 a `c`, menos a faixa do recuo (posição do material por fora ou por
//!   dentro: recuo de r·tg(θ/2) ou (r + t)·tg(θ/2));
//! - aba dobrada: a seção (setor da dobra mais parede, área `A`, momento `Q` em relação à linha de dobra,
//!   na direção para fora) varrida pela linha de dobra recuada. Pelo teorema de Pappus, numa curva plana o
//!   volume é `A·L + Q'·Φ`, com `L` o comprimento, `Φ` o quanto a tangente gira e `Q' = Q − recuo·A`;
//!   cortada por um plano x = constante (limite de lado) em vez do plano normal, ganha `Q'·(Oₓ/Tₓ)` na ponta;
//! - aba planificada: faixa de largura `ba + comprimento` desenvolvida para fora da linha recuada:
//!   `t·(dev·L' + dev²·Φ/2)`.
//!
//! O erro relativo do volume da aba fica abaixo de 0,1 % em todas as escalas (ver a tabela no ADR 0107,
//! seção B566 e B564), e a caixa envolvente bate com o envelope da seção ao longo da OML.

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_kernel::{Curva3, Shape, Trecho3};
use fenkra_solidos::{final_shape, recompute, Status};
use std::time::Instant;

fn f(x: f64) -> String {
    format!("{x}")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Sup {
    Plano,
    Curvo,
    Deslocado,
}

#[derive(Clone, Copy, Debug)]
struct Caso {
    c: f64,
    t: f64,
    r: f64,
    len: f64,
    ang: f64,
    side: u8,
    lados: bool,
    sup: Sup,
}

impl Caso {
    /// Espessura, raio e comprimento proporcionais à corda (2, 2 e 20 mm na corda de 200 mm).
    fn prop(c: f64, sup: Sup) -> Caso {
        let s = c / 200.;
        Caso { c, t: 2. * s, r: 2. * s, len: 20. * s, ang: 90., side: 0, lados: false, sup }
    }
    /// Espessura fixa (raio igual à espessura, aba de 20 mm).
    fn fixa(c: f64, t: f64, sup: Sup) -> Caso {
        Caso { c, t, r: t, len: 20., ang: 90., side: 0, lados: false, sup }
    }
    fn com(self, ang: f64, side: u8, lados: bool) -> Caso {
        Caso { ang, side, lados, ..self }
    }
    fn lim(&self) -> (f64, f64) {
        if self.lados {
            (0.25 * self.c, 0.75 * self.c)
        } else {
            (0., self.c)
        }
    }
    fn off(&self) -> f64 {
        self.c * 0.02
    }
}

fn naca(x: f64) -> f64 {
    5. * 0.12 * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4))
}

/// Ponto do dorso da asa de corda `c` no parâmetro `x` (0 a 1).
fn dorso(c: f64, x: f64) -> [f64; 2] {
    [-0.05 * c + 1.1 * c * x, 0.28 * c + 1.4 * c * naca(x) * 0.5 + 0.06 * c * (1. - x)]
}

/// Os 61 pontos da spline do suporte.
fn pontos_do_dorso(c: f64) -> Vec<[f64; 3]> {
    (0..=60)
        .map(|i| {
            let u = i as f64 / 60.;
            let p = dorso(c, (1. - (std::f64::consts::PI * u).cos()) / 2.);
            [p[0], p[1], 0.]
        })
        .collect()
}

fn monta(k: &Caso, flat: bool) -> (Document, Id) {
    let c = k.c;
    let mut d = Document::new("escala");
    d.sheet = Some(SheetParams { thickness: f(k.t), radius: f(k.r), k: "0.44".into(), flat, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [c, c * 0.6]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let sup = match k.sup {
        Sup::Plano => {
            // o plano XZ tem normal −y: deslocado de −0,4 c, é o plano y = 0,4 c
            let p = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::XZ), offset: f(-0.4 * c) });
            SurfRef::Plane(PlaneRef::Datum(p))
        }
        Sup::Curvo | Sup::Deslocado => {
            let pts = pontos_do_dorso(c).into_iter().map(PointRef::Fixed).collect();
            let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
            let mut s = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: f(2. * c), length2: "0".into(), until: None, until2: None, mais: vec![] });
            if k.sup == Sup::Deslocado {
                s = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(s), dist: f(k.off()), both: false, join: OffsetJoin::Automatico });
            }
            SurfRef::Ref(s)
        }
    };
    let mut limites = LimitesAba::default();
    if k.lados {
        let (x1, x2) = k.lim();
        for (i, x) in [x1, x2].into_iter().enumerate() {
            let p = d.add_geometry(RefKind::OffsetPlane { base: PlaneRef::Origin(OriginPlane::YZ), offset: f(x) });
            let l = Some(LimiteAba::Superficie(SurfRef::Plane(PlaneRef::Datum(p))));
            if i == 0 {
                limites.lado1 = l;
            } else {
                limites.lado2 = l;
            }
        }
    }
    let mode = if k.ang == 90. { 0 } else { 2 };
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: sup, face: [c / 2., c * 0.1, k.t], length: f(k.len), radius: None, flip: false, keep_base: false, mode, angle: f(k.ang), limites, side: k.side, inv_lado: false }));
    (d, aba)
}

/// OML teórica: pontos (x, y) com x crescente, sobre a curva exata do suporte no plano da web.
fn oml_teorica(k: &Caso) -> Vec<[f64; 2]> {
    let c = k.c;
    let n = 20000;
    if k.sup == Sup::Plano {
        return (0..=n).map(|i| [-0.1 * c + 1.2 * c * i as f64 / n as f64, 0.4 * c]).collect();
    }
    let spline = Curva3(vec![Trecho3::Interpolada { pts: pontos_do_dorso(c), fechada: false }]);
    let amostra: Vec<[f64; 3]> = (0..=n)
        .map(|i| {
            let p = dorso(c, i as f64 / n as f64);
            [p[0], p[1], 0.]
        })
        .collect();
    let (curva, amostra) = if k.sup == Sup::Deslocado {
        // o suporte deslocado corta o plano na spline deslocada para fora (+y)
        let o = k.off();
        let cands: Vec<Curva3> = [o, -o].iter().filter_map(|d| Shape::curva_deslocada(&spline, [0., 0., 1.], *d).ok()).collect();
        let meio = |cv: &Curva3| Shape::curva_projeta(cv, &[[0.5 * c, 0.4 * c, 0.]]).map(|p| p[0][1]).unwrap_or(f64::MIN);
        let cv = cands.into_iter().max_by(|a, b| meio(a).total_cmp(&meio(b))).expect("deslocamento da spline");
        let a2: Vec<[f64; 3]> = amostra.iter().map(|p| [p[0], p[1] + o, 0.]).collect();
        (cv, a2)
    } else {
        (spline, amostra)
    };
    let mut q: Vec<[f64; 2]> = Shape::curva_projeta(&curva, &amostra).expect("projeção na spline").into_iter().map(|p| [p[0], p[1]]).collect();
    q.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-12);
    q
}

/// Ponto e tangente unitária da OML em x.
fn em_x(o: &[[f64; 2]], x: f64) -> ([f64; 2], [f64; 2]) {
    let i = o.windows(2).position(|w| w[0][0] <= x && w[1][0] >= x).expect("x fora da OML");
    let (a, b) = (o[i], o[i + 1]);
    let u = if b[0] > a[0] { (x - a[0]) / (b[0] - a[0]) } else { 0. };
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l = dx.hypot(dy);
    ([a[0] + u * dx, a[1] + u * dy], [dx / l, dy / l])
}

/// Integral de y em x de `a` a `b` na polilinha (x crescente).
fn area_sob(o: &[[f64; 2]], a: f64, b: f64) -> f64 {
    let mut s = 0.;
    for w in o.windows(2) {
        let (x0, x1) = (w[0][0].max(a), w[1][0].min(b));
        if x1 <= x0 || w[1][0] <= w[0][0] {
            continue;
        }
        let y = |x: f64| w[0][1] + (w[1][1] - w[0][1]) * (x - w[0][0]) / (w[1][0] - w[0][0]);
        s += (y(x0) + y(x1)) / 2. * (x1 - x0);
    }
    s
}

/// Comprimento da OML de x = `a` a `b`.
fn comprimento(o: &[[f64; 2]], a: f64, b: f64) -> f64 {
    let mut l = 0.;
    for w in o.windows(2) {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        if dx <= 0. {
            continue;
        }
        let (x0, x1) = (w[0][0].max(a), w[1][0].min(b));
        if x1 > x0 {
            l += dx.hypot(dy) * (x1 - x0) / dx;
        }
    }
    l
}

/// Normal para fora (+y, a web fica embaixo) da tangente `t` (x crescente).
fn fora(t: [f64; 2]) -> [f64; 2] {
    [-t[1], t[0]]
}

struct Esperado {
    vol: f64,
    aba: f64,
    lo: [f64; 3],
    hi: [f64; 3],
}

/// O resultado teórico do caso: volume total, volume da aba e caixa envolvente.
fn esperado(k: &Caso, o: &[[f64; 2]], flat: bool) -> Esperado {
    let (t, r, len) = (k.t, k.r, k.len);
    let a = k.ang.to_radians();
    let half = (a / 2.).min(80f64.to_radians());
    let off = match k.side {
        1 => r * half.tan(),
        2 => (r + t) * half.tan(),
        _ => 0.,
    };
    let (xa, xb) = k.lim();
    let ((_, ta), (_, tb)) = (em_x(o, xa), em_x(o, xb));
    let l = comprimento(o, xa, xb);
    let phi = ta[1].atan2(ta[0]) - tb[1].atan2(tb[0]);
    // web: área sob a OML menos a faixa do recuo
    let web = area_sob(o, 0., k.c);
    let faixa = if off <= 0. {
        0.
    } else if k.lados && !flat {
        // a dobrada corta o recorte nos planos x = limite
        let rec: Vec<[f64; 2]> = o
            .windows(2)
            .map(|w| {
                let tg = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
                let n = tg[0].hypot(tg[1]);
                let fo = fora([tg[0] / n, tg[1] / n]);
                [w[0][0] - off * fo[0], w[0][1] - off * fo[1]]
            })
            .collect();
        area_sob(o, xa, xb) - area_sob(&rec, xa, xb)
    } else {
        off * l - off * off * phi / 2.
    };
    let dev = a * (r + 0.44 * t) + len;
    let l_rec = l - off * phi;
    let (aba, wmin, wmax, ztop) = if flat {
        (t * (dev * l_rec + dev * dev * phi / 2.), 0., dev, t)
    } else {
        let area = a * t * (r + t / 2.) + len * t;
        let q = ((r + t).powi(3) - r.powi(3)) / 3. * (1. - a.cos()) + len * t * ((r + t / 2.) * a.sin() + len / 2. * a.cos());
        let q1 = q - off * area;
        let g = |tg: [f64; 2]| fora(tg)[0] / tg[0];
        let corte = if k.lados { q1 * (g(ta) - g(tb)) } else { 0. };
        // envelope da seção: w (para fora) e altura acima da face interna
        let (ie, oe) = ([r * a.sin(), r - r * a.cos()], [(r + t) * a.sin(), r - (r + t) * a.cos()]);
        let (it, ot) = ([ie[0] + len * a.cos(), ie[1] + len * a.sin()], [oe[0] + len * a.cos(), oe[1] + len * a.sin()]);
        let arco = if a >= std::f64::consts::FRAC_PI_2 { r + t } else { (r + t) * a.sin() };
        let wmax = [0., arco, it[0], ot[0]].into_iter().fold(f64::MIN, f64::max);
        let wmin = [0., it[0], ot[0]].into_iter().fold(f64::MAX, f64::min);
        let ytop = [0., it[1], ot[1], oe[1]].into_iter().fold(f64::MIN, f64::max);
        (area * l + q1 * phi + corte, wmin, wmax, t + ytop)
    };
    // caixa: web e envelope da aba
    let (mut lo, mut hi): ([f64; 3], [f64; 3]) = ([0., 0., 0.], [k.c, 0., ztop]);
    let mut add = |p: [f64; 2]| {
        lo[0] = lo[0].min(p[0]);
        lo[1] = lo[1].min(p[1]);
        hi[0] = hi[0].max(p[0]);
        hi[1] = hi[1].max(p[1]);
    };
    add(em_x(o, 0.).0);
    add(em_x(o, k.c).0);
    for p in o.iter().filter(|p| p[0] >= 0. && p[0] <= k.c && (off <= 0. || p[0] <= xa || p[0] >= xb)) {
        add(*p);
    }
    let corte_x = k.lados && !flat;
    let ws: Vec<f64> = (0..=40).map(|i| wmin + (wmax - wmin) * i as f64 / 40.).collect();
    let mut pts: Vec<([f64; 2], [f64; 2])> = o.windows(2).map(|w| {
        let tg = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
        let n = tg[0].hypot(tg[1]);
        (w[0], [tg[0] / n, tg[1] / n])
    }).filter(|(p, _)| corte_x || (p[0] >= xa && p[0] <= xb)).collect();
    pts.push(em_x(o, xa));
    pts.push(em_x(o, xb));
    for (p, tg) in pts {
        let fo = fora(tg);
        for w in &ws {
            let q = [p[0] + (w - off) * fo[0], p[1] + (w - off) * fo[1]];
            if !corte_x || (q[0] >= xa - 1e-9 && q[0] <= xb + 1e-9) {
                add(q);
            }
        }
    }
    Esperado { vol: t * (web - faixa) + aba, aba, lo, hi }
}

/// Monta, recalcula e confere status, volume e caixa. Devolve a linha da tabela.
fn confere(k: &Caso, flat: bool) -> String {
    let o = oml_teorica(k);
    let e = esperado(k, &o, flat);
    let t0 = Instant::now();
    let (d, aba) = monta(k, flat);
    let res = recompute(&d);
    let dt = t0.elapsed().as_secs_f64();
    let estado = if flat { "planificada" } else { "dobrada" };
    assert!(ok_ou_conformacao(&res.status[&aba]), "{k:?} {estado}");
    assert_eq!(res.bodies.len(), 1, "{k:?} {estado}: um corpo só");
    // A medida do volume. Na dobrada, a integração padrão do kernel (`mass`, pontos de Gauss fixos) erra
    // ~6e-6 do total nas faces varridas da dobra, mais que 1e-3 do volume da aba quando a espessura é fixa e a
    // peça cresce; a adaptativa (`volume_preciso`) acerta. Na planificada (só faces planas e extrusões) é o
    // contrário: a adaptativa não converge (informa erro 1e-16 e o volume muda 3e-5 com o eps pedido) e a
    // padrão bate com a conta a 2e-7.
    let forma = final_shape(&d).expect("forma final");
    assert!(forma.is_valid(), "{k:?} {estado}: forma inválida");
    let v = if flat { volume_total(&res) } else { forma.volume_preciso() };
    let erro = (v - e.vol) / e.aba;
    let (lo, hi) = caixa_da_malha(&res);
    let linha = format!("B566 escala {k:?} {estado}: {dt:.2} s, volume {v:.6e} (teórico {:.6e}, padrão {:.6e}), erro na aba {erro:+.3e}, caixa {lo:.4?} {hi:.4?} (teórica {:.4?} {:.4?})", e.vol, volume_total(&res), e.lo, e.hi);
    println!("{linha}");
    assert!(erro.abs() < 1e-3, "{k:?} {estado}: volume {v:.4} contra {:.4} (erro {erro:+.2e} do volume da aba {:.4})", e.vol, e.aba);
    // a malha de exibição erra o extremo das faces curvas pelo desvio dela (relativo à diagonal)
    let diag = ((e.hi[0] - e.lo[0]).powi(2) + (e.hi[1] - e.lo[1]).powi(2) + (e.hi[2] - e.lo[2]).powi(2)).sqrt();
    let tol = (6e-4 * diag.max(k.c)).max(1e-3);
    for i in 0..3 {
        assert!((lo[i] - e.lo[i]).abs() < tol && (hi[i] - e.hi[i]).abs() < tol, "{k:?} {estado}: caixa {lo:?} {hi:?} contra {:?} {:?} (tolerância {tol})", e.lo, e.hi);
    }
    linha
}

/// Confere todos os casos nos dois estados e só então falha, com a lista do que não passou.
fn varre(casos: &[Caso]) {
    let mut falhas = vec![];
    for k in casos {
        for flat in [false, true] {
            if let Err(e) = std::panic::catch_unwind(|| confere(k, flat)) {
                let m = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                println!("FALHA {k:?} planificada {flat}: {m}");
                falhas.push(format!("{k:?} planificada {flat}: {m}"));
            }
        }
    }
    assert!(falhas.is_empty(), "{} falhas:\n{}", falhas.len(), falhas.join("\n"));
}

const CORDAS: [f64; 10] = [10., 20., 50., 100., 200., 400., 1000., 2000., 5000., 10000.];

#[test]
fn escala_proporcional_dorso() {
    if !kernel_disponivel() {
        return;
    }
    varre(&CORDAS.map(|c| Caso::prop(c, Sup::Curvo)));
}

#[test]
fn escala_proporcional_dorso_deslocado() {
    if !kernel_disponivel() {
        return;
    }
    varre(&CORDAS.map(|c| Caso::prop(c, Sup::Deslocado)));
}

#[test]
fn escala_proporcional_plano() {
    if !kernel_disponivel() {
        return;
    }
    varre(&CORDAS.map(|c| Caso::prop(c, Sup::Plano)));
}

/// Espessura fixa de 1 e 3 mm em peça grande: a aba fica fina perto da peça.
#[test]
fn escala_espessura_fixa_em_peca_grande() {
    if !kernel_disponivel() {
        return;
    }
    let mut v = vec![];
    for t in [1., 3.] {
        for c in [1000., 2000., 5000., 10000.] {
            v.push(Caso::fixa(c, t, Sup::Curvo));
        }
    }
    v.push(Caso::fixa(10000., 1., Sup::Deslocado));
    v.push(Caso::fixa(10000., 3., Sup::Plano));
    varre(&v);
}

/// Ângulos, posição do material e limites dos lados nas pontas da escala.
#[test]
fn escala_angulos_posicao_e_lados() {
    if !kernel_disponivel() {
        return;
    }
    let mut v = vec![];
    for c in [10., 200., 10000.] {
        let p = |s| Caso::prop(c, s);
        v.push(p(Sup::Curvo).com(45., 1, true));
        v.push(p(Sup::Deslocado).com(135., 2, false));
        v.push(p(Sup::Curvo).com(90., 2, true));
        v.push(p(Sup::Plano).com(45., 0, false));
        v.push(p(Sup::Plano).com(135., 1, true));
    }
    v.push(Caso::fixa(10000., 1., Sup::Curvo).com(135., 2, true));
    v.push(Caso::fixa(5000., 3., Sup::Deslocado).com(45., 1, false));
    // aba maior que a peça: raio de 3 mm e 20 mm de aba numa web de 10 × 6
    v.push(Caso::fixa(10., 3., Sup::Curvo));
    v.push(Caso::fixa(10., 3., Sup::Plano).com(135., 0, false));
    varre(&v);
}

// ------------------------------------------------------------------ aba curva por arestas, cadeia de três

/// Placa de `c` × 0,6 `c` com o canto (c, 0,6 c) arredondado em raio 0,2 `c`, e a aba curva pela cadeia
/// tangente lado direito + arco + lado de cima (três arestas). Com o material por fora ou por dentro a linha de
/// dobra recua, e a curva recuada é a cadeia deslocada trecho a trecho pelo kernel (antes, a polilinha
/// refeita em retas e arcos).
fn placa_arredondada(c: f64, t: f64, r: f64, len: f64, ang: f64, side: u8, flat: bool) -> (Document, Id) {
    let (w, h, rr) = (c, 0.6 * c, 0.2 * c);
    let mut d = Document::new("cadeia");
    d.sheet = Some(SheetParams { thickness: f(t), radius: f(r), k: "0.44".into(), flat, ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 9_200_000u64;
    let mut al = move || {
        n += 1;
        Id(n)
    };
    let p: Vec<Id> = [[0., 0.], [w, 0.], [w, h - rr], [w - rr, h - rr], [w - rr, h], [0., h]].iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    data.add_curve(&mut al, Curve::Line { a: p[0], b: p[1] }, false);
    data.add_curve(&mut al, Curve::Line { a: p[1], b: p[2] }, false);
    data.add_curve(&mut al, Curve::Arc { c: p[3], s: p[2], e: p[4] }, false);
    data.add_curve(&mut al, Curve::Line { a: p[4], b: p[5] }, false);
    data.add_curve(&mut al, Curve::Line { a: p[5], b: p[0] }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    // a cadeia como o clique a guarda: pontos sobre as três arestas, na face de cima
    let mut edge = vec![];
    for i in 0..=4 {
        edge.push([w, (h - rr) * i as f64 / 4., t]);
    }
    for i in 1..=60 {
        let a = std::f64::consts::FRAC_PI_2 * i as f64 / 60.;
        edge.push([w - rr + rr * a.cos(), h - rr + rr * a.sin(), t]);
    }
    for i in 1..=4 {
        edge.push([(w - rr) * (1. - i as f64 / 4.), h, t]);
    }
    let aba = d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: f(len), flip: false, angle: f(ang), radius: None, side, edge_key: None, so_aresta: false, reta: None }));
    (d, aba)
}

#[test]
fn aba_curva_em_cadeia_de_tres_arestas_com_recuo_em_qualquer_escala() {
    if !kernel_disponivel() {
        return;
    }
    let mut falhas = vec![];
    for c in [10., 1000., 10000.] {
        let s = c / 200.;
        let (t, r, len) = (2. * s, 2. * s, 20. * s);
        for (ang, side) in [(90., 0u8), (90., 1), (90., 2), (135., 2), (45., 1)] {
            for flat in [false, true] {
                let (w, h, rr) = (c, 0.6 * c, 0.2 * c);
                let a = (ang as f64).to_radians();
                let half = (a / 2.).min(80f64.to_radians());
                let off = match side {
                    1 => r * half.tan(),
                    2 => (r + t) * half.tan(),
                    _ => 0.,
                };
                let phi = std::f64::consts::FRAC_PI_2;
                let l = (h - rr) + rr * phi + (w - rr);
                let web = w * h - (1. - std::f64::consts::FRAC_PI_4) * rr * rr - (off * l - off * off * phi / 2.);
                let l_rec = l - off * phi;
                let aba_v = if flat {
                    let dev = a * (r + 0.44 * t) + len;
                    t * (dev * l_rec + dev * dev * phi / 2.)
                } else {
                    let area = a * t * (r + t / 2.) + len * t;
                    let q = ((r + t).powi(3) - r.powi(3)) / 3. * (1. - a.cos()) + len * t * ((r + t / 2.) * a.sin() + len / 2. * a.cos());
                    area * l + (q - off * area) * phi
                };
                let esperado = t * web + aba_v;
                let t0 = Instant::now();
                let (d, aba) = placa_arredondada(c, t, r, len, ang, side, flat);
                let res = recompute(&d);
                let dt = t0.elapsed().as_secs_f64();
                let forma = final_shape(&d);
                let v = match &forma {
                    Some(fm) if !flat => fm.volume_preciso(),
                    _ => volume_total(&res),
                };
                let erro = (v - esperado) / aba_v;
                let linha = format!("B566 cadeia corda {c} ângulo {ang} posição {side} planificada {flat}: {:?}, {dt:.2} s, volume {v:.6e} (teórico {esperado:.6e}), erro na aba {erro:+.3e}", res.status[&aba]);
                println!("{linha}");
                if !ok_ou_conformacao(&res.status[&aba]) || !forma.is_some_and(|fm| fm.is_valid()) || res.bodies.len() != 1 || erro.abs() > 1e-3 {
                    falhas.push(linha);
                }
            }
        }
    }
    assert!(falhas.is_empty(), "{} falhas:\n{}", falhas.len(), falhas.join("\n"));
}

/// O que não cabe dá erro com o motivo, nunca corpo vazio ou errado com status Ok. Com o lado invertido
/// a web fica acima do dorso e a linha de dobra é côncava: perto do canto x = 0 o raio dela é ~0,36 da corda
/// (71 mm na corda 200). A faixa planificada de 100 mm se cruza ali; a dobrada, de parede a prumo, cabe.
#[test]
fn o_que_nao_cabe_da_erro_claro() {
    if !kernel_disponivel() {
        return;
    }
    // recuo maior que a chapa: material por dentro a 135° em chapa de 3 mm, raio 3: recuo de
    // (3 + 3)·tg(67,5°) = 14,5 mm numa web de 10 × 4 (suporte plano em y = 4). Antes a web sumia no recorte
    // e sobrava só a aba, com status Ok.
    for flat in [false, true] {
        let k = Caso::fixa(10., 3., Sup::Plano).com(135., 2, false);
        let (d, aba) = monta(&k, flat);
        let res = recompute(&d);
        let st = &res.status[&aba];
        println!("B566 não cabe: recuo maior que a chapa, planificada {flat}: {st:?}");
        assert!(matches!(st, Status::Error(m) if m.contains("recuo")), "planificada {flat}: {st:?}");
    }
    let base = Caso::prop(200., Sup::Curvo);
    for (len, raio, flat) in [(100., None, true), (20., Some(100.), false), (100., None, false)] {
        let k = Caso { len, ..base };
        let (mut d, aba) = monta(&k, flat);
        if let Some(FeatureKind::SmSurfFlange(sf)) = d.features.iter_mut().find(|x| x.id == aba).map(|x| &mut x.kind) {
            sf.inv_lado = true;
            sf.face = [100., 110., 2.];
            sf.radius = raio.map(f);
        }
        let res = recompute(&d);
        let st = &res.status[&aba];
        println!("B566 não cabe: comprimento {len}, raio {raio:?}, planificada {flat}: {st:?}, volume {:.1}", volume_total(&res));
        match st {
            Status::Error(m) => assert!(!m.trim().is_empty(), "erro sem motivo"),
            _ => {
                // se calculou, o corpo é um só, válido e maior que a web (a aba está lá)
                let forma = final_shape(&d).expect("forma final");
                assert!(forma.is_valid() && res.bodies.len() == 1, "comprimento {len}, raio {raio:?}, planificada {flat}: {st:?} com corpo inválido");
                let web = 200. * 120. * 2. * (1. - 0.5678);
                assert!(forma.volume_preciso() > web, "comprimento {len}, raio {raio:?}, planificada {flat}: {st:?} sem a aba ({:.1} contra a web {web:.1})", forma.volume_preciso());
            }
        }
    }
}

/// A varredura completa (todas as combinações; lenta).
#[test]
#[ignore]
fn escala_completa() {
    if !kernel_disponivel() {
        return;
    }
    let mut base = vec![];
    for sup in [Sup::Plano, Sup::Curvo, Sup::Deslocado] {
        base.extend(CORDAS.map(|c| Caso::prop(c, sup)));
        for t in [1., 3.] {
            base.extend([1000., 2000., 5000., 10000.].map(|c| Caso::fixa(c, t, sup)));
        }
    }
    let mut casos = vec![];
    for b in base {
        for ang in [45., 90., 135.] {
            for side in [0, 1, 2] {
                for lados in [false, true] {
                    casos.push(b.com(ang, side, lados));
                }
            }
        }
    }
    varre(&casos);
}
