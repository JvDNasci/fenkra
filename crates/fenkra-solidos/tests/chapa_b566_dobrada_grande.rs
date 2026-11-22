// SPDX-License-Identifier: LGPL-2.1-or-later
//! B566 (pendência 1): a aba por superfície DOBRADA em asa grande.
//!
//! A web de `c` × 0,6 `c` é relimitada pelo dorso de um perfil de asa de quatro dígitos (extrudado na
//! envergadura, e também deslocado de 20 mm) e a aba de 20 mm sobe pela linha de dobra (OML). Antes, de
//! corda 400 para cima, a união da varredura da dobra com a web saía vazia ou só com uma das partes e o
//! comando dava erro; o planificado funcionava.
//!
//! Causa: a varredura seguia a polilinha das amostras da OML. As amostras vêm da seção do suporte pelo
//! plano da web, que o kernel entrega como polilinha, e caem nas cordas: até 0,08 mm fora da aresta
//! verdadeira da web na corda 400, e a flecha cresce com a peça. Além disso a polilinha era refeita em
//! retas e arcos aceitos com erro de até 0,2 % do raio (mais de 1 mm em raio grande), com quinas entre
//! eles: a ferramenta saía com 150 a 200 faces, transições arredondadas que se cruzam (o analisador da
//! booleana acusa autointerseção na corda 1000) e a face da dobra, tangente à face da web, nascendo
//! décimos de mm fora da borda dela. A booleana não classifica esse contato quase tangente e devolve
//! vazio. Agora a varredura segue a aresta exata da web (uma aresta, aparada nas amostras das pontas),
//! as amostras são levadas para cima dela e a faixa planificada tem as bordas em spline: a ferramenta
//! tem 10 faces e a fusão sai válida, sem tolerância difusa, em todas as escalas.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, Status};
use std::time::Instant;

fn s(x: &str) -> String {
    x.to_string()
}

fn naca(x: f64) -> f64 {
    5. * 0.12 * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4))
}

/// Ponto do dorso da asa de corda `c` no parâmetro `x` (0 a 1).
fn dorso(c: f64, x: f64) -> [f64; 2] {
    [-0.05 * c + 1.1 * c * x, 0.28 * c + 1.4 * c * naca(x) * 0.5 + 0.06 * c * (1. - x)]
}

/// Asa de corda `c` e envergadura `e` (mm), suporte opcionalmente deslocado de `off` mm.
fn asa(c: f64, e: f64, off: Option<&str>, flat: bool) -> (Document, Id) {
    let mut d = Document::new("asa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [c, c * 0.6]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let pts: Vec<PointRef> = (0..=60)
        .map(|i| {
            let u = i as f64 / 60.;
            let p = dorso(c, (1. - (std::f64::consts::PI * u).cos()) / 2.);
            PointRef::Fixed([p[0], p[1], 0.])
        })
        .collect();
    let curva = d.add_geometry(RefKind::Spline { points: pts, closed: false, tangents: vec![] });
    let mut sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: format!("{e}"), length2: s("0"), until: None, until2: None, mais: vec![] });
    if let Some(o) = off {
        sup = d.add_geometry(RefKind::SurfOffset { surface: SurfRef::Ref(sup), dist: s(o), both: false, join: OffsetJoin::Automatico });
    }
    let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [c / 2., c * 0.1, 2.], length: s("20"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90"), limites: Default::default(), side: 0, inv_lado: false }));
    (d, aba)
}

/// Monta, recalcula dobrada e planificada e confere status, volume e caixa envolvente.
fn confere(c: f64, e: f64, off: Option<&str>) {
    let t = Instant::now();
    let (d, aba) = asa(c, e, off, false);
    let dobrada = recompute(&d);
    let t_dobrada = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let (dp, abap) = asa(c, e, off, true);
    let plana = recompute(&dp);
    let t_plana = t.elapsed().as_secs_f64();
    let (vd, vp) = (volume_total(&dobrada), volume_total(&plana));
    let ((lo, hi), (lop, hip)) = (caixa_da_malha(&dobrada), caixa_da_malha(&plana));
    println!("B566 corda {c} ({off:?}): dobrada {t_dobrada:.2} s vol {vd:.1} caixa {lo:?} {hi:?}; planificada {t_plana:.2} s vol {vp:.1} caixa {lop:?} {hip:?}");
    assert_eq!(dobrada.status[&aba], Status::Ok, "corda {c} dobrada");
    assert_eq!(plana.status[&abap], Status::Ok, "corda {c} planificada");

    // volume: a web relimitada (a mesma nos dois estados) mais a aba. A aba dobrada tem
    // θ·(r + t/2)·t + comprimento·t por mm de linha de dobra (49,4 mm²) e a faixa planificada usa
    // θ·(r + k·t) com k = 0,44: a diferença do fator K é θ·(0,5 − k)·t² ≈ 0,38 mm³ por mm de linha de
    // dobra; a faixa plana, desenvolvida para fora de uma OML curva, ganha ou perde um pouco pela
    // curvatura. A OML mede cerca de uma corda: a diferença fica abaixo de 0,6 mm³ por mm de corda.
    assert!((vd - vp).abs() < 0.6 * c, "corda {c}: dobrada {vd:.1}, planificada {vp:.1}");
    // a web relimitada fica com ~57 % (dorso) ou ~60 % (deslocado) do retângulo de c × 0,6 c × 2
    let web = c * 0.6 * c * 2.;
    let aba_dobrada = vd - web * if off.is_some() { 0.6013 } else { 0.5678 };
    assert!(aba_dobrada > 46. * c && aba_dobrada < 53. * c, "corda {c}: a aba dobrada tem {aba_dobrada:.0} mm³ (esperado ~49,4 por mm de OML)");

    // caixa: a web vai de 0 a c em x e a aba sobe 2 + r + comprimento = 24 mm em z
    assert!(lo[0] > -0.5 && hi[0] < c + 0.5, "corda {c}: caixa dobrada em x {lo:?} {hi:?}");
    assert!(lo[2] > -1e-3 && (hi[2] - 24.).abs() < 0.05, "corda {c}: caixa dobrada em z {lo:?} {hi:?}");
    assert!(lop[2] > -1e-3 && hip[2] < 2. + 1e-3, "corda {c}: a planificada fica plana {lop:?} {hip:?}");
    // em y: o topo da linha de dobra (o dorso, mais o deslocamento) mais r + t na dobrada; na
    // planificada, a faixa desenvolvida (ba + comprimento = 24,5 mm) para fora dele
    let topo = (0..=2000).map(|i| dorso(c, i as f64 / 2000.)).filter(|p| p[0] >= 0. && p[0] <= c).map(|p| p[1]).fold(f64::MIN, f64::max) + off.map_or(0., |o| o.parse::<f64>().unwrap());
    assert!((hi[1] - (topo + 4.)).abs() < 0.5, "corda {c}: topo da dobrada {} contra {}", hi[1], topo + 4.);
    assert!((hip[1] - (topo + 24.52)).abs() < 0.5, "corda {c}: topo da planificada {} contra {}", hip[1], topo + 24.52);
}

#[test]
fn aba_dobrada_em_asa_de_corda_200() {
    if !kernel_disponivel() {
        return;
    }
    confere(200., 400., None);
}

#[test]
fn aba_dobrada_em_asa_de_corda_400() {
    if !kernel_disponivel() {
        return;
    }
    confere(400., 800., None);
}

#[test]
fn aba_dobrada_em_asa_de_corda_1000() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., 2000., None);
}

/// O caso do relato: corda 1000, envergadura 2000 e o suporte deslocado de 20 mm.
#[test]
fn aba_dobrada_em_asa_de_corda_1000_suporte_deslocado() {
    if !kernel_disponivel() {
        return;
    }
    confere(1000., 2000., Some("20"));
}
