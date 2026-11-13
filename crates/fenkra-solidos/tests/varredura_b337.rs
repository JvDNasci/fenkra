// SPDX-License-Identifier: LGPL-2.1-or-later
//! B337: varredura com guia suave. O sorteio tem semente fixa e passa pelo caminho do produto; cada
//! resultado termina em sólido bom, em erro nomeado ou, na guia com cantos, em aviso.

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_kernel::{Bool, Frame, Seg2, Shape};
use fenkra_solidos::{geo3d, recompute, Status};
use std::f64::consts::PI;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);

fn guia_sorteada(g: &mut Sorteio, amplitude: f64) -> Vec<[f64; 3]> {
    let n = 8 + (g.proximo() % 18) as usize;
    let p: Vec<[f64; 3]> = (0..n).map(|k| [10. * k as f64, g.cota(-amplitude, amplitude), g.cota(-amplitude, amplitude)]).collect();
    geo3d::spline(&p, false).unwrap().pts
}

fn comprimento(p: &[[f64; 3]]) -> f64 {
    p.windows(2).map(|q| geo3d::dist(q[0], q[1])).sum()
}

/// Mesmo critério do kernel para mandar a guia ao ramo de trechos retos.
fn tem_cantos(p: &[[f64; 3]]) -> bool {
    p.windows(3).any(|q| {
        let (a, b) = (geo3d::unit(geo3d::sub(q[1], q[0])).unwrap(), geo3d::unit(geo3d::sub(q[2], q[1])).unwrap());
        geo3d::dot(a, b).clamp(-1., 1.).acos() > 0.35
    })
}

/// Círculo de raio `r` normal à guia, no início dela.
fn perfil(p: &[[f64; 3]], r: f64) -> Shape {
    let t = geo3d::unit(geo3d::sub(p[1], p[0])).unwrap();
    let f = Frame { origin: p[0], x: geo3d::any_perp(t), normal: t };
    Shape::face(&f, &[vec![Seg2::Circle([0., 0.], r)]]).unwrap()
}

/// Volume assinado da malha: mede a mesma superfície por outro caminho que o da integral do kernel,
/// que erra em superfícies muito onduladas.
fn volume_da_malha(s: &Shape) -> f64 {
    let m = s.mesh(0.02).unwrap();
    let v = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
    let soma: f64 = m
        .indices
        .chunks_exact(3)
        .map(|t| {
            let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.
        })
        .sum();
    soma.abs()
}

/// Menor raio do círculo que passa por três pontos vizinhos da polilinha.
fn raio_discreto(p: &[[f64; 3]]) -> f64 {
    p.windows(3)
        .map(|q| {
            let (a, b, c) = (geo3d::dist(q[0], q[1]), geo3d::dist(q[1], q[2]), geo3d::dist(q[0], q[2]));
            let area2 = geo3d::norm(fenkra_kernel::cross(geo3d::sub(q[1], q[0]), geo3d::sub(q[2], q[0])));
            if area2 < 1e-15 { f64::INFINITY } else { a * b * c / (2. * area2) }
        })
        .fold(f64::INFINITY, f64::min)
}

/// Tubo de seção constante sem dobra: volume = área × comprimento da guia (teorema de Pappus).
#[test]
fn sorteio_de_guias_termina_em_tubo_certo_ou_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let mut g = Sorteio(7);
    let (mut bons, mut cantos, mut dobra, mut desistiu, mut tombou) = (0, 0, 0, 0, 0);
    for i in 0..120 {
        let a = [1., 3., 3., 6., 12.][i % 5];
        let p = guia_sorteada(&mut g, a);
        match perfil(&p, 1.).sweep(&p, 0) {
            Ok(s) => {
                assert!(s.is_valid(), "caso {i}: forma inválida passou calada");
                // com cantos, a seção gira com junta de esquadria (B350): o volume é o mesmo do tubo
                if tem_cantos(&p) {
                    cantos += 1;
                }
                let (vm, esperado) = (volume_da_malha(&s), PI * comprimento(&p));
                // a malha é inscrita: com deflexão 0,02 num raio 1 ela perde perto de 3% por corda
                assert!((vm / esperado - 1.).abs() < 0.05, "caso {i}: volume da malha {vm:.1}, esperado {esperado:.1}");
                bons += 1;
            }
            Err(e) if e.0.contains("dobraria") => {
                // a recusa por dobra tem de ser da guia, não da aproximação do kernel: o menor raio do
                // círculo por três amostras vizinhas da própria polilinha fica abaixo do braço do perfil
                // (com folga para a amostra, que corta a curva mais apertada entre dois pontos)
                let r = raio_discreto(&p);
                assert!(r < 1.5, "caso {i}: recusada por dobra, mas a guia tem raio {r:.2} mm para um perfil de 1 mm");
                dobra += 1
            }
            Err(e) if e.0.contains("guia suave") => desistiu += 1,
            Err(e) if e.0.contains("girar nos cantos") => tombou += 1,
            Err(e) => panic!("caso {i}: erro sem nome: {}", e.0),
        }
    }
    println!("bons {bons}, cantos {cantos}, dobra {dobra}, desistiu {desistiu}, tombou {tombou}");
    assert_eq!(desistiu, 0, "guia sem dobra precisa varrer");
    assert_eq!(tombou, 0, "guia com cantos sorteada precisa girar a seção (B350)");
    // antes da cúbica eram 77 de 120: a aproximação de grau alto ondulava e a checagem recusava por
    // dobra guias com raio de sobra (o portão por raio discreto acima pega isso)
    assert!(bons >= 100, "a maioria das guias sorteadas é suave e sem dobra: {bons}");
}

/// O perfil só dobra no plano da curva: a altura de uma parede erguida sobre a guia não conta.
#[test]
fn parede_alta_sobre_guia_plana_nao_e_recusada_por_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let p: Vec<[f64; 3]> = (0..=40).map(|i| { let a = 0.04 * i as f64; [20. * a.sin(), 20. * (1. - a.cos()), 0.] }).collect();
    let (w, h) = (2., 100.);
    let f = Frame { origin: p[0], x: [0., 0., 1.], normal: [1., 0., 0.] };
    let c = [[-h / 2., -w / 2.], [h / 2., -w / 2.], [h / 2., w / 2.], [-h / 2., w / 2.], [-h / 2., -w / 2.]];
    let laco: Vec<Seg2> = c.windows(2).map(|q| Seg2::Line(q[0], q[1])).collect();
    let s = Shape::face(&f, &[laco]).unwrap().sweep(&p, 0).expect("a parede cabe na curva de raio 20");
    let esperado = w * h * comprimento(&p);
    assert!(s.is_valid());
    assert!((s.mass().volume / esperado - 1.).abs() < 0.01, "volume {} esperado {esperado}", s.mass().volume);
    let (a, b) = s.bbox().unwrap();
    assert!((b[2] - a[2] - h).abs() < 0.1, "altura da parede {a:?} {b:?}");
}

#[test]
fn curva_mais_fechada_que_o_perfil_da_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let p: Vec<[f64; 3]> = (0..=40).map(|i| { let a = 0.04 * i as f64; [3. * a.sin(), 3. * (1. - a.cos()), 0.] }).collect();
    let e = perfil(&p, 5.).sweep(&p, 0).err().expect("perfil de raio 5 numa curva de raio 3 dobra");
    assert!(e.0.contains("dobraria"), "{}", e.0);
    assert!(perfil(&p, 1.).sweep(&p, 0).is_ok(), "o perfil de raio 1 cabe nessa curva");
}

/// A seção gira nos cantos (B350): nem a guia com cantos nem a suave levam o aviso de cantos.
#[test]
fn guia_com_cantos_e_guia_suave_saem_sem_aviso_de_cantos() {
    if !kernel_disponivel() {
        return;
    }
    let status = |p: Vec<[f64; 3]>, kind: fn(Vec<PointRef>) -> RefKind| {
        let mut d = Document::new("guia");
        let g = d.add_geometry(kind(p.into_iter().map(PointRef::Fixed).collect()));
        let perfil = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::YZ), &[([0., 0.], 2.)]);
        let id = d.add_feature(FeatureKind::Sweep(Sweep { sketch: perfil, spine: CurveRef::Ref(g), mode: SweepMode::Corrigido, op: BoolOp::Add, thin: None }));
        recompute(&d).status[&id].clone()
    };
    let l = status(vec![[0., 0., 0.], [50., 0., 0.], [50., 50., 0.]], |points| RefKind::Polyline { points, closed: false });
    assert_eq!(l, Status::Ok, "{l:?}");
    let suave = status(vec![[0., 0., 0.], [20., 5., 0.], [40., -5., 0.], [60., 0., 0.]], |points| RefKind::Spline { points, closed: false, tangents: vec![] });
    assert!(!matches!(&suave, Status::Warning(m) if m.contains("cantos")) && !matches!(suave, Status::Error(_)), "{suave:?}");
}

/// O prazo chega à booleana, que é onde a nervura lenta gasta o tempo, e some com a guarda.
#[test]
fn limite_de_tempo_interrompe_a_booleana_com_erro_nomeado() {
    if !kernel_disponivel() {
        return;
    }
    let f = Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] };
    let a = Shape::primitive(1, &f, &[10., 20.]).unwrap();
    let b = Shape::primitive(1, &Frame { origin: [5., 0., 0.], ..f }, &[10., 20.]).unwrap();
    {
        let _prazo = fenkra_kernel::limitar_tempo(0.);
        let e = a.boolean(&b, Bool::Fuse).err().expect("prazo vencido deve interromper");
        assert!(e.0.contains("limite de tempo"), "{}", e.0);
    }
    assert!(a.boolean(&b, Bool::Fuse).is_ok(), "fora da guarda não há prazo");
}

/// Com duas curvas no sketch, a primeira nervura sai e a segunda falha: a chapa volta inteira. É o mesmo
/// caminho do limite de tempo, que pode vencer depois de uma soma já aplicada.
#[test]
fn nervura_que_falha_no_meio_devolve_a_chapa_intacta() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("nervura");
    d.sheet = Some(SheetParams { thickness: "2".into(), radius: "2".into(), k: "0.44".into(), flat: false, ..Default::default() });
    let base = sk_rect(&mut d, XY, [-20., -30.], [300., 30.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: base, reverse: false, offset: String::new(), mid: false, surface: None }));
    let topo = d.add_offset_plane(XY, "2");
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    let fechada: Vec<Id> = [[0., 10.], [2., 14.], [4., 10.], [6., 14.], [8., 10.]].iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    data.add_curve(&mut al, Curve::Spline { pts: fechada }, false);
    let boa: Vec<Id> = [[0., -15.], [60., -14.], [120., -15.], [180., -14.]].iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
    data.add_curve(&mut al, Curve::Spline { pts: boa }, false);
    let curva = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
    let id = d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: curva, depth: "2".into(), flip: false, kind: 1, width: "6".into(), punch: None }));
    let r = recompute(&d);
    assert!(matches!(&r.status[&id], Status::Error(_)), "{:?}", r.status[&id]);
    assert!((volume_total(&r) - 320. * 60. * 2.).abs() < 1e-6, "a chapa devia ficar intacta: {}", volume_total(&r));
}

/// Nervura de chapa sorteada: toda resposta chega antes do limite de 60 s somado a uma folga, porque a
/// booleana só confere o prazo entre etapas.
#[test]
fn sorteio_de_nervuras_de_chapa_responde_no_prazo() {
    if !kernel_disponivel() {
        return;
    }
    let mut g = Sorteio(11);
    let (mut pior, mut boas) = (Duration::ZERO, 0);
    for i in 0..30 {
        let n = 8 + (g.proximo() % 18) as usize;
        let a = [1., 3., 6.][i % 3];
        let pts: Vec<[f64; 2]> = (0..n).map(|k| [10. * k as f64, g.cota(-a, a)]).collect();
        let (tx, rx) = mpsc::channel();
        let inicio = Instant::now();
        std::thread::spawn(move || {
            let mut d = Document::new("nervura");
            d.sheet = Some(SheetParams { thickness: "2".into(), radius: "2".into(), k: "0.44".into(), flat: false, ..Default::default() });
            let base = sk_rect(&mut d, XY, [-20., -30.], [300., 30.]);
            d.add_feature(FeatureKind::SmWall(SmWall { sketch: base, reverse: false, offset: String::new(), mid: false, surface: None }));
            let topo = d.add_offset_plane(XY, "2");
            let mut data = SketchData::default();
            let mut al = || d.alloc();
            let ids: Vec<Id> = pts.iter().map(|q| data.add_point(&mut al, q[0], q[1])).collect();
            data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
            let curva = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data }));
            let id = d.add_feature(FeatureKind::SmStamp(SmStamp { sketch: curva, depth: "2".into(), flip: false, kind: 1, width: "6".into(), punch: None }));
            let r = recompute(&d);
            let _ = tx.send(r.status[&id].clone());
        });
        let st = rx.recv_timeout(Duration::from_secs(90)).unwrap_or_else(|_| panic!("nervura {i} passou de 90 s sem resposta"));
        let t = inicio.elapsed();
        pior = pior.max(t);
        println!("nervura {i}: {:.1} s {}", t.as_secs_f64(), format!("{st:?}").chars().take(80).collect::<String>());
        match st {
            Status::Error(m) => assert!(["dobraria", "guia suave", "limite de tempo"].iter().any(|n| m.contains(n)), "nervura {i}: erro sem nome: {m}"),
            _ => boas += 1,
        }
    }
    println!("pior nervura: {:.1} s, boas {boas}", pior.as_secs_f64());
    assert!(boas >= 5, "a checagem de dobra não pode recusar todas as nervuras: {boas}");
}
