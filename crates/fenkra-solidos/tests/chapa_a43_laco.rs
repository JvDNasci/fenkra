// SPDX-License-Identifier: LGPL-2.1-or-later
//! B579, defeitos do teste com mouse da alpha.43: a aba por laço em perfil curvo fechado.
//!
//! O caminho que a pessoa escolhe para a nervura é a aba por laço ("laço da face inteira"). O contorno curvo
//! fechado chega do kernel em vários trechos (a spline do corte da asa quebra na costura e no bordo de fuga) e o
//! laço fazia uma aba por trecho tangente; a união das faixas separadas saía inválida na planificação. Agora o
//! contorno fechado de curvas vai numa aba só (o caminho de perfil fechado do B579), no contorno externo e no
//! furo, dobrada e planificada, com o mapa de deformação.
//!
//! Cada resultado é conferido contra a conta feita à parte (Pappus numa curva fechada de giro 2π):
//! - aba dobrada: seção (setor da dobra mais parede, área `A`, momento `Q`) varrida pelo contorno: `A·L + Q·Φ`;
//! - aba planificada: faixa de largura `D = ba + comprimento`: `t·(D·L + D²·Φ/2)`;
//! para o furo o giro é −2π (a aba vai para dentro).

mod common;

use common::*;
use fenkra_core::sketch::Curve;
use fenkra_core::*;
use fenkra_solidos::{chapa_fechada, final_shape, recompute, Status};
use std::time::Instant;

const K: f64 = 0.44;

fn s(x: &str) -> String {
    x.to_string()
}

fn carrega(nome: &str) -> Document {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/pesquisa/asa").join(nome);
    Document::from_json(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// Aceita Ok ou o aviso de conformação (curva forte): os dois dizem que a aba saiu.
fn sem_erro(st: &Status, quem: &str) {
    match st {
        Status::Ok => {}
        Status::Warning(m) => assert!(m.contains("conformação") || m.contains("canto vivo"), "{quem}: aviso inesperado: {m}"),
        outro => panic!("{quem}: {outro:?}"),
    }
}

/// Volume da malha de flecha 0,05 da peça de chapa (o corpo de menor caixa em z que tem a espessura da chapa).
fn volume_e_caixa(forma: &fenkra_kernel::Shape) -> (f64, [f64; 3], [f64; 3]) {
    let m = forma.mesh(0.05).expect("malha");
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    (chapa_fechada::volume_com(forma, 0.05), lo, hi)
}

/// Teoria (aba de ângulo `a` graus, giro `phi`): volume da aba dobrada e da planificada.
fn teoria(l: f64, t: f64, r: f64, len: f64, a_graus: f64, phi: f64) -> (f64, f64) {
    let a = a_graus.to_radians();
    let dev = a * (r + K * t) + len;
    let sec = a * t * (r + t / 2.) + len * t;
    let q = ((r + t).powi(3) - r.powi(3)) / 3. * (1. - a.cos()) + len * t * ((r + t / 2.) * a.sin() + len / 2. * a.cos());
    (sec * l + q * phi, t * (dev * l + dev * dev * phi / 2.))
}

// ------------------------------------------------------------------ a nervura do teste com mouse (arquivo salvo)

/// A nervura salva pelo teste da alpha.43 (asa enflechada 2000→1500 por loft, estação 1500, interseção "Corpo
/// inteiro", parede de 2 mm com 2 furos, aba por laço de 20 mm no contorno externo), dobrada e planificada.
/// A planificada falhava com "aba 2 de 3: a união da faixa planificada da aba com a chapa saiu inválida".
#[test]
fn nervura_a43_aba_por_laco_dobrada_e_planificada() {
    if !kernel_disponivel() {
        return;
    }
    let aba = Id(152);
    // a chapa sem a aba: volume e perímetro do contorno externo
    let mut sem = carrega("nervura-a43.fkp");
    sem.feature_mut(aba).unwrap().suppressed = true;
    let r0 = recompute(&sem);
    let web = final_shape(&sem).expect("chapa sem aba");
    let (vol_web, _, _) = volume_e_caixa(&web);
    let b0 = r0.bodies.iter().find(|b| b.collector == Id(145)).expect("corpo da chapa");
    let p = [238.61561584472656, -1502.0, 9.032608032226562];
    let d0 = fenkra_solidos::descreve_laco(&b0.mesh, p).expect("laço da aresta escolhida");
    assert!(d0.externo && d0.furos == 2, "o laço escolhido é o contorno externo de uma face com 2 furos: {d0:?}");
    let l = d0.comprimento;
    let (t, r, len) = (2., 2., 20.);
    let (dobrada, plana) = teoria(l, t, r, len, 90., std::f64::consts::TAU);
    for flat in [true, false] {
        let mut d = carrega("nervura-a43.fkp");
        d.sheet.as_mut().unwrap().flat = flat;
        let t0 = Instant::now();
        let res = recompute(&d);
        let seg = t0.elapsed().as_secs_f64();
        let estado = if flat { "planificada" } else { "dobrada" };
        sem_erro(&res.status[&aba], estado);
        let forma = final_shape(&d).expect("forma final");
        assert!(forma.is_valid(), "{estado}: forma inválida");
        let (v, lo, hi) = volume_e_caixa(&forma);
        let teorico = if flat { plana } else { dobrada };
        let erro = (v - vol_web - teorico) / teorico;
        println!("nervura a43 {estado}: volume {v:.0} (chapa {vol_web:.0}), aba {:.0} contra {teorico:.0} ({:+.2} % da aba), {seg:.1} s, {:?}", v - vol_web, erro * 100., res.status[&aba]);
        assert!(erro.abs() < 0.06, "{estado}: a aba mede {:.0} e a teoria dá {teorico:.0} ({:+.2} %)", v - vol_web, erro * 100.);
        // caixa: a web está no plano XZ (normal Y). Planificada, a faixa se abre no plano: X e Z crescem `dev` de cada lado
        // (o leque do bordo de fuga e do nariz cortado leva `dev` além das pontas); dobrada, X cresce r + t de cada lado
        // e Y o quanto a aba sobe (t + r + comprimento)
        let (_, wlo, whi) = volume_e_caixa(&web);
        let dev = std::f64::consts::FRAC_PI_2 * (r + K * t) + len;
        println!("  caixa da aba {lo:?} {hi:?}; da chapa {wlo:?} {whi:?}; dev {dev:.2}");
        if flat {
            // o nariz cortado leva a faixa inteira para o lado; no bordo de fuga agudo (18°) o leque do canto (raio `dev`)
            // não entra: as faixas dos dois lados terminam nele, e a falta é o setor do canto (cerca de 1 % do volume da aba)
            let (largura, cheia) = (hi[0] - lo[0], whi[0] - wlo[0] + 2. * dev);
            assert!(largura <= cheia + 3. && largura >= cheia - dev - 3., "planificada: X {largura:.1} contra {cheia:.1}");
            assert!((hi[2] - lo[2] - (whi[2] - wlo[2] + 2. * dev)).abs() < 3., "planificada: Z {:.1} contra {:.1}", hi[2] - lo[2], whi[2] - wlo[2] + 2. * dev);
            assert!((hi[1] - lo[1] - (whi[1] - wlo[1])).abs() < 0.3, "planificada: a espessura não muda");
        } else {
            assert!((hi[0] - lo[0] - (whi[0] - wlo[0] + 2. * (r + t))).abs() < 8., "dobrada: X {:.1} contra {:.1}", hi[0] - lo[0], whi[0] - wlo[0] + 2. * (r + t));
            assert!((hi[1] - lo[1] - (whi[1] - wlo[1] + r + len + t)).abs() < 4., "dobrada: Y {:.1} contra {:.1}", hi[1] - lo[1], whi[1] - wlo[1] + r + len + t);
        }
        // o mapa de deformação existe para a aba
        let analise = res.conformacoes.iter().find(|(id, _)| *id == aba);
        assert!(analise.is_some_and(|(_, a)| !a.pontos.is_empty()), "{estado}: a aba por laço entrega a análise de conformação");
    }
}

// ------------------------------------------------------------------ a caverna oval com anel

/// Anel oval (externo 1200 × 900, furo 1100 × 800, espessura 2) por spline fechada de `n` pontos.
fn caverna(plana: bool, n: usize) -> (Document, Id) {
    let mut d = Document::new("caverna");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: plana, ..Default::default() });
    let mut data = SketchData::default();
    let mut n_id = 9_300_000u64;
    let mut al = move || {
        n_id += 1;
        Id(n_id)
    };
    for (a, b) in [(600., 450.), (550., 400.)] {
        let mut ids: Vec<Id> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64 * std::f64::consts::TAU;
                data.add_point(&mut al, a * t.cos(), b * t.sin())
            })
            .collect();
        ids.push(ids[0]);
        data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let w = d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    (d, w)
}

fn perimetro(a: f64, b: f64, n: usize) -> f64 {
    let p = |i: usize| {
        let t = (i % n) as f64 / n as f64 * std::f64::consts::TAU;
        [a * t.cos(), b * t.sin()]
    };
    (0..n).map(|i| (p(i + 1)[0] - p(i)[0]).hypot(p(i + 1)[1] - p(i)[1])).sum()
}

/// Ponto médio da aresta de cima da chapa no laço de `semi` (semieixo x): pelo kernel, na forma sem a aba.
fn meio_do_laco(d: &Document, semi: f64, ztopo: f64) -> [f64; 3] {
    let forma = final_shape(d).expect("forma");
    let topo = forma.topology().expect("topologia");
    topo.edge_mids
        .iter()
        .copied()
        .filter(|m| (m[2] - ztopo).abs() < 1e-3)
        .min_by(|a, b| ((a[0].hypot(a[1] * semi / 450.) - semi).abs()).total_cmp(&(b[0].hypot(b[1] * semi / 450.) - semi).abs()))
        .expect("aresta do laço")
}

#[derive(Clone, Copy, Debug)]
struct CasoLaco {
    furo: bool,
    baixo: bool,
    plana: bool,
    ang: f64,
}

fn caso_laco(c: CasoLaco) -> (Document, Id) {
    let (mut d, _) = caverna(c.plana, 96);
    let semi = if c.furo { 550. } else { 600. };
    let mid = meio_do_laco(&d, semi, 2.);
    let aba = d.add_feature(FeatureKind::SmFlangeLoop(SmFlangeLoop {
        edges: vec![mid],
        edge_keys: vec![],
        loop_all: true,
        extent: ExtensaoAba::Inteira,
        invert: false,
        length: s("20"),
        angle: format!("{}", c.ang),
        radius: None,
        flip: c.baixo,
        side: 0,
        miter: true,
        relief: None,
        relief_kind: 0,
        seguir: false,
    }));
    (d, aba)
}

fn confere_laco(c: CasoLaco, tol: f64) {
    let (d, aba) = caso_laco(c);
    let t0 = Instant::now();
    let r = recompute(&d);
    let seg = t0.elapsed().as_secs_f64();
    let quem = format!("{c:?}");
    sem_erro(&r.status[&aba], &quem);
    assert_eq!(r.bodies.len(), 1, "{quem}: um corpo só");
    let forma = final_shape(&d).expect("forma final");
    assert!(forma.is_valid(), "{quem}: forma inválida");
    let (v, lo, hi) = volume_e_caixa(&forma);
    // a chapa sem a aba (o anel de 2 mm), medida igual
    let mut sem = d.clone();
    sem.feature_mut(aba).unwrap().suppressed = true;
    let (vw, _, _) = volume_e_caixa(&final_shape(&sem).expect("forma da chapa"));
    let (a, b) = if c.furo { (550., 400.) } else { (600., 450.) };
    let l = perimetro(a, b, 96);
    let phi = if c.furo { -std::f64::consts::TAU } else { std::f64::consts::TAU };
    let (dob, pla) = teoria(l, 2., 2., 20., c.ang, phi);
    let teorico = if c.plana { pla } else { dob };
    let erro = (v - vw - teorico) / teorico;
    println!("| caverna laço {} {} {} {}° | {:.0} | {:.0} | {:+.2} % | {:.1} s | {:?} |", if c.furo { "furo" } else { "externo" }, if c.baixo { "baixo" } else { "cima" }, if c.plana { "planificada" } else { "dobrada" }, c.ang, v - vw, teorico, erro * 100., seg, r.status[&aba]);
    assert!(erro.abs() < tol, "{quem}: a aba mede {:.0} e a teoria dá {teorico:.0} ({:+.2} %)", v - vw, erro * 100.);
    // caixa em x e y: o anel; fora do furo a aba não passa dele, e para fora ela avança (r + t dobrada, D planificada)
    let ra = c.ang.to_radians();
    let dev = ra * (2. + K * 2.) + 20.;
    // dobrada: a face externa da dobra avança (r + t)·sen(a) e a parede reta mais comprimento·cos(a)
    let fora = if c.plana { dev } else { 4. * ra.sin() + 20. * ra.cos() };
    if !c.furo {
        assert!((hi[0] - lo[0] - (1200. + 2. * fora)).abs() < 2.5, "{quem}: largura {:.1} contra {:.1}", hi[0] - lo[0], 1200. + 2. * fora);
    } else {
        assert!((hi[0] - lo[0] - 1200.).abs() < 2.5, "{quem}: a aba do furo não sai do anel: largura {:.1}", hi[0] - lo[0]);
    }
    // o mapa de deformação
    let analise = r.conformacoes.iter().find(|(id, _)| *id == aba);
    assert!(analise.is_some_and(|(_, a)| !a.pontos.is_empty()), "{quem}: análise de conformação");
}

/// Aba por laço no contorno externo da caverna, para cima e para baixo, dobrada e planificada.
#[test]
fn caverna_oval_laco_externo_cima_e_baixo() {
    if !kernel_disponivel() {
        return;
    }
    for (baixo, plana) in [(false, false), (false, true), (true, false), (true, true)] {
        confere_laco(CasoLaco { furo: false, baixo, plana, ang: 90. }, 0.06);
    }
}

/// Aba por laço na aresta do furo (para dentro do furo), para cima e para baixo, dobrada e planificada.
#[test]
fn caverna_oval_laco_do_furo_cima_e_baixo() {
    if !kernel_disponivel() {
        return;
    }
    for (baixo, plana) in [(false, false), (false, true), (true, false), (true, true)] {
        confere_laco(CasoLaco { furo: true, baixo, plana, ang: 90. }, 0.06);
    }
}

/// Aba de 60° (ângulo diferente de 90°) por laço, dobrada e planificada.
#[test]
fn caverna_oval_laco_a_60_graus() {
    if !kernel_disponivel() {
        return;
    }
    for plana in [false, true] {
        confere_laco(CasoLaco { furo: false, baixo: false, plana, ang: 60. }, 0.06);
    }
}

/// O laço diz qual contorno pegou: externo ou furo.
#[test]
fn descreve_o_laco_externo_e_o_furo() {
    if !kernel_disponivel() {
        return;
    }
    let (d, _) = caverna(false, 96);
    let r = recompute(&d);
    let m = &r.bodies[0].mesh;
    let ext = fenkra_solidos::descreve_laco(m, meio_do_laco(&d, 600., 2.)).expect("externo");
    assert!(ext.externo && ext.furo == 0 && ext.furos == 1, "{ext:?}");
    assert!((ext.comprimento - perimetro(600., 450., 96)).abs() < 0.01 * ext.comprimento, "perímetro {:.1}", ext.comprimento);
    let furo = fenkra_solidos::descreve_laco(m, meio_do_laco(&d, 550., 2.)).expect("furo");
    assert!(!furo.externo && furo.furo == 1 && furo.furos == 1, "{furo:?}");
    assert!(furo.comprimento < ext.comprimento);
}

// ------------------------------------------------------------------ aba para dentro de furo

/// Placa 400 × 200 com um furo de raio `r_furo` e a aba por laço na aresta dele.
fn placa_com_furo(plana: bool, r_furo: f64, len: f64) -> (Document, Id) {
    let mut d = Document::new("placa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: plana, ..Default::default() });
    let mut data = SketchData::default();
    let mut n_id = 9_500_000u64;
    let mut al = move || {
        n_id += 1;
        Id(n_id)
    };
    data.add_rect(&mut al, [0., 0.], [400., 200.], false);
    let c = data.add_point(&mut al, 200., 100.);
    data.add_curve(&mut al, Curve::Circle { c, r: r_furo }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let forma = final_shape(&d).expect("chapa");
    let mid = forma
        .topology()
        .expect("topologia")
        .edge_mids
        .iter()
        .copied()
        .find(|m| (m[2] - 2.).abs() < 1e-3 && ((m[0] - 200.).hypot(m[1] - 100.) - r_furo).abs() < 0.5)
        .expect("aresta do furo");
    let aba = d.add_feature(FeatureKind::SmFlangeLoop(SmFlangeLoop {
        edges: vec![mid],
        edge_keys: vec![],
        loop_all: true,
        extent: ExtensaoAba::Inteira,
        invert: false,
        length: format!("{len}"),
        angle: s("90"),
        radius: None,
        flip: false,
        side: 0,
        miter: true,
        relief: None,
        relief_kind: 0,
        seguir: false,
    }));
    (d, aba)
}

/// Furo pequeno demais para a aba: erro claro (a faixa se cruza do outro lado), nunca "command not done".
#[test]
fn aba_maior_que_o_furo_da_erro_claro() {
    if !kernel_disponivel() {
        return;
    }
    for plana in [false, true] {
        let (d, aba) = placa_com_furo(plana, 15., 20.);
        let r = recompute(&d);
        match &r.status[&aba] {
            Status::Error(m) => {
                assert!(m.contains("não cabe") && m.contains("comprimento reto de até"), "plana={plana}: mensagem: {m}");
                assert!(!m.contains("command not done"), "{m}");
            }
            outro => panic!("plana={plana}: esperava erro claro, veio {outro:?}"),
        }
    }
}

/// Furo de raio 40 com aba de 20 mm para dentro: cabe, dobrada e planificada, com o volume de Pappus (giro −2π).
#[test]
fn aba_para_dentro_de_furo_que_cabe() {
    if !kernel_disponivel() {
        return;
    }
    for plana in [false, true] {
        let (d, aba) = placa_com_furo(plana, 40., 12.);
        let r = recompute(&d);
        sem_erro(&r.status[&aba], &format!("plana={plana}"));
        let forma = final_shape(&d).expect("forma");
        assert!(forma.is_valid(), "plana={plana}: forma inválida");
        let (v, _, _) = volume_e_caixa(&forma);
        let mut sem = d.clone();
        sem.feature_mut(aba).unwrap().suppressed = true;
        let (vw, _, _) = volume_e_caixa(&final_shape(&sem).expect("chapa"));
        let (dob, pla) = teoria(2. * std::f64::consts::PI * 40., 2., 2., 12., 90., -std::f64::consts::TAU);
        let teorico = if plana { pla } else { dob };
        let erro = (v - vw - teorico) / teorico;
        println!("furo r40 {} : aba {:.0} contra {teorico:.0} ({:+.2} %)", if plana { "planificada" } else { "dobrada" }, v - vw, erro * 100.);
        assert!(erro.abs() < 0.06, "plana={plana}: aba {:.0} contra {teorico:.0} ({:+.2} %)", v - vw, erro * 100.);
    }
}

// ------------------------------------------------------------------ ângulo diferente de 90° na aba por superfície

/// Aba por superfície (tipo Ângulo, 60°) no perfil fechado: o suporte é a extrusão da própria spline.
#[test]
fn aba_por_superficie_a_60_graus_no_perfil_fechado() {
    if !kernel_disponivel() {
        return;
    }
    let n = 96;
    let pts: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            [600. * t.cos(), 450. * t.sin()]
        })
        .collect();
    for plana in [true, false] {
        let mut d = Document::new("sup60");
        d.sheet = Some(SheetParams { thickness: s("1.2"), radius: s("1.2"), k: s("0.44"), flat: plana, ..Default::default() });
        let mut data = SketchData::default();
        let mut n_id = 9_600_000u64;
        let mut al = move || {
            n_id += 1;
            Id(n_id)
        };
        let mut ids: Vec<Id> = pts.iter().map(|p| data.add_point(&mut al, p[0], p[1])).collect();
        ids.push(ids[0]);
        data.add_curve(&mut al, Curve::Spline { pts: ids }, false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        let curva = d.add_geometry(RefKind::Spline { points: pts.iter().map(|q| PointRef::Fixed([q[0], q[1], 0.])).collect(), closed: true, tangents: vec![] });
        let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("1200"), length2: s("1200"), until: None, until2: None, mais: vec![] });
        let aba = d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange {
            support: SurfRef::Ref(sup),
            face: [0., 0., 1.2],
            length: s("10"),
            radius: None,
            flip: false,
            keep_base: false,
            mode: 2,
            angle: s("60"),
            limites: Default::default(),
            side: 0,
            inv_lado: false,
        }));
        let t0 = Instant::now();
        let r = recompute(&d);
        let seg = t0.elapsed().as_secs_f64();
        sem_erro(&r.status[&aba], &format!("plana={plana}"));
        let forma = final_shape(&d).expect("forma");
        assert!(forma.is_valid(), "plana={plana}: forma inválida");
        let (v, _, _) = volume_e_caixa(&forma);
        let mut sem = d.clone();
        sem.feature_mut(aba).unwrap().suppressed = true;
        let (vw, _, _) = volume_e_caixa(&final_shape(&sem).expect("chapa"));
        let l = perimetro(600., 450., n);
        let (dob, pla) = teoria(l, 1.2, 1.2, 10., 60., std::f64::consts::TAU);
        let teorico = if plana { pla } else { dob };
        let erro = (v - vw - teorico) / teorico;
        println!("superfície 60° {}: aba {:.0} contra {teorico:.0} ({:+.2} %), {seg:.1} s, {:?}", if plana { "planificada" } else { "dobrada" }, v - vw, erro * 100., r.status[&aba]);
        assert!(erro.abs() < 0.06, "plana={plana}: aba {:.0} contra {teorico:.0} ({:+.2} %)", v - vw, erro * 100.);
    }
}
