// SPDX-License-Identifier: LGPL-2.1-or-later
//! Laço de abas num contorno com curva (B530): retângulo 100 × 60 com cantos R10, laço inteiro.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn chapa(d: &mut Document) {
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
}

fn confere_ok(d: &Document) -> fenkra_solidos::RecomputeResult {
    let r = recompute(d);
    for f in &d.features {
        assert!(ok_ou_conformacao(&r.status[&f.id]), "{}: {:?}", f.name, r.status[&f.id]);
    }
    r
}

fn laco(pontos: Vec<[f64; 3]>, todo: bool) -> FeatureKind {
    FeatureKind::SmFlangeLoop(SmFlangeLoop {
        edges: pontos,
        edge_keys: vec![],
        loop_all: todo,
        extent: ExtensaoAba::Inteira,
        invert: false,
        length: s("12"),
        angle: s("90"),
        radius: None,
        flip: false,
        side: 0,
        miter: true,
        relief: None,
        relief_kind: 0, seguir: false,
    })
}

#[test]
fn laco_inteiro_do_retangulo_arredondado() {
    if !kernel_disponivel() {
        return;
    }
    // retângulo 100 × 60 com cantos R10: a aba segue a borda de cima inteira (reta + arco + reta)
    let mut d = Document::new("laco_arredondado");
    chapa(&mut d);
    let mut data = SketchData::default();
    let mut n = 7_100_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    fn p(data: &mut SketchData, al: &mut impl FnMut() -> Id, x: f64, y: f64) -> Id {
        data.add_point(al, x, y)
    }
    let (r, w, h) = (10., 100., 60.);
    let a1 = p(&mut data, &mut al, r, 0.);
    let a2 = p(&mut data, &mut al, w - r, 0.);
    let c2 = p(&mut data, &mut al, w - r, r);
    let b2 = p(&mut data, &mut al, w, r);
    let b3 = p(&mut data, &mut al, w, h - r);
    let c3 = p(&mut data, &mut al, w - r, h - r);
    let a3 = p(&mut data, &mut al, w - r, h);
    let a4 = p(&mut data, &mut al, r, h);
    let c4 = p(&mut data, &mut al, r, h - r);
    let b4 = p(&mut data, &mut al, 0., h - r);
    let b1 = p(&mut data, &mut al, 0., r);
    let c1 = p(&mut data, &mut al, r, r);
    use fenkra_core::sketch::Curve;
    for c in [
        Curve::Line { a: a1, b: a2 },
        Curve::Arc { c: c2, s: a2, e: b2 },
        Curve::Line { a: b2, b: b3 },
        Curve::Arc { c: c3, s: b3, e: a3 },
        Curve::Line { a: a3, b: a4 },
        Curve::Arc { c: c4, s: a4, e: b4 },
        Curve::Line { a: b4, b: b1 },
        Curve::Arc { c: c1, s: b1, e: a1 },
    ] {
        data.add_curve(&mut al, c, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let parede = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let (r0, w0, h0) = (10., 100., 60.);
    let _ = (r0, w0, h0, parede);
    // base sem aba: volume da chapa
    let vbase = recompute(&d).bodies[0].mass.volume;
    // clicar numa aresta reta (borda de cima) com "laço da face inteira"
    d.add_feature(laco(vec![[w / 2., h, 2.]], true));
    let r1 = confere_ok(&d);
    let (_, max) = caixa_da_malha(&r1);
    assert!((max[2] - 16.).abs() < 0.2, "altura {max:?}");
    let v = r1.bodies[0].mass.volume;
    // contorno 2*(80+40) + 2*pi*10 = 302,8 mm; seção da aba = espessura 2 × (perna 12 + dobra ~4,5)
    // = ~33 mm², a ~1 mm da aresta para fora: volume ~ 302,8 * 33 = ~10 000 mm³
    let acresc = v - vbase;
    assert!(acresc > 9000. && acresc < 11500., "volume da aba {acresc}");
    // cobre o contorno todo: bordas e cantos têm parede (100 x 60 + r + t = 4 mm para fora de cada lado).
    // A varredura do kernel ainda deixa até ~1 mm de folga na zona da dobra (aproximação), por isso 1,5 mm.
    let (min, max) = caixa_da_malha(&r1);
    assert!((max[0] - min[0] - 108.).abs() < 1.5 && (max[1] - min[1] - 68.).abs() < 2., "caixa {min:?} {max:?}");
    // planifica
    d.sheet.as_mut().unwrap().flat = true;
    let rf = confere_ok(&d);
    let (_, maxf) = caixa_da_malha(&rf);
    assert!(maxf[2] < 2.01, "planificada é plana {maxf:?}");
}

/// B560: o modo único "Arestas": uma lista com reta, curva e reta (a borda de cima, o canto
/// arredondado e a borda da direita) vira uma aba só, com os trechos curvos emendados às retas; uma
/// reta sozinha e uma curva sozinha pelo mesmo modo também dão aba. Volume e caixa conferidos.
#[test]
fn b560_lista_mista_reta_curva_reta_e_uma_aba_continua() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("lista_mista");
    chapa(&mut d);
    let sk = {
        let mut data = SketchData::default();
        let mut n = 7_200_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let (r, w, h) = (10., 100., 60.);
        let mut pt = |x: f64, y: f64| data.add_point(&mut al, x, y);
        let (a1, a2, c2, b2, b3, c3, a3, a4, c4, b4, b1, c1) = (pt(r, 0.), pt(w - r, 0.), pt(w - r, r), pt(w, r), pt(w, h - r), pt(w - r, h - r), pt(w - r, h), pt(r, h), pt(r, h - r), pt(0., h - r), pt(0., r), pt(r, r));
        use fenkra_core::sketch::Curve;
        let cs = [
            Curve::Line { a: a1, b: a2 },
            Curve::Arc { c: c2, s: a2, e: b2 },
            Curve::Line { a: b2, b: b3 },
            Curve::Arc { c: c3, s: b3, e: a3 },
            Curve::Line { a: a3, b: a4 },
            Curve::Arc { c: c4, s: a4, e: b4 },
            Curve::Line { a: b4, b: b1 },
            Curve::Arc { c: c1, s: b1, e: a1 },
        ];
        for c in cs {
            data.add_curve(&mut al, c, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }))
    };
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let vbase = recompute(&d).bodies[0].mass.volume;
    let arco = [90. + 10. * 45f64.to_radians().cos(), 50. + 10. * 45f64.to_radians().sin(), 2.];
    // borda de cima + canto + borda da direita, escolhidas uma a uma (sem o laço da face)
    let id = d.add_feature(laco(vec![[50., 60., 2.], arco, [100., 30., 2.]], false));
    let r = confere_ok(&d);
    let (_, hi) = caixa_da_malha(&r);
    assert!((hi[2] - 16.).abs() < 0.3, "altura da aba {hi:?}");
    let acresc = r.bodies[0].mass.volume - vbase;
    // 80 (cima) + 15,7 (arco) + 40 (direita) = 135,7 mm de aresta, seção ~33 mm²
    assert!(acresc > 3500. && acresc < 5500., "volume das abas da lista mista {acresc}");
    assert!(ok_ou_conformacao(&r.status[&id]));
    // planificada correta
    d.sheet.as_mut().unwrap().flat = true;
    let rf = confere_ok(&d);
    assert!(caixa_da_malha(&rf).1[2] < 2.01, "planificada é plana");

    // uma aresta curva sozinha pelo mesmo modo (a aba curva) e uma reta sozinha dão aba
    for (nome, p) in [("curva", arco), ("reta", [50., 60., 2.])] {
        let mut d1 = d.clone();
        d1.sheet.as_mut().unwrap().flat = false;
        d1.features.pop();
        d1.add_feature(laco(vec![p], false));
        let r1 = confere_ok(&d1);
        let ganho = r1.bodies[0].mass.volume - vbase;
        assert!(ganho > 300., "{nome}: a aba somou volume ({ganho})");
        assert!(caixa_da_malha(&r1).1[2] > 15., "{nome}: a aba sobe");
    }
}
