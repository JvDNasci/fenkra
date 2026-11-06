// SPDX-License-Identifier: LGPL-2.1-or-later
//! B284: extensão lateral da aba e laço de arestas. Cada modo gera a largura esperada (volume e
//! centro de massa contra a conta e contra a aba comum com recuos), e o laço retangular gera
//! quatro abas com mitra.

use super::*;
use fenkra_core::{ExtensaoAba, SmFlangeLoop};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.)
}

fn placa() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 4_000_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

fn laco(edges: Vec<[f64; 3]>, extent: ExtensaoAba, loop_all: bool, miter: bool) -> FeatureKind {
    FeatureKind::SmFlangeLoop(SmFlangeLoop { edges, edge_keys: vec![], loop_all, extent, invert: false, length: s("20"), angle: s("90"), radius: None, flip: false, side: 0, miter, relief: None, relief_kind: 0 })
}

fn aba(e: [f64; 3], off1: &str, off2: &str, miter: bool) -> FeatureKind {
    FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s(off1), off2: s(off2), flip: false, relief: None, relief_kind: 0, side: 0, miter, min_gap: true, edge_key: None })
}

/// (volume, centro de massa, caixa) do corpo, com todos os comandos conferidos.
fn mede(d: &Document) -> (f64, [f64; 3], [f64; 3], [f64; 3]) {
    let r = recompute(d);
    for f in &d.features {
        assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
    }
    let b = &r.bodies[0];
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in b.mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    (b.mass.volume, b.mass.center, lo, hi)
}

/// Volume de uma aba reta de 90° (raio 2, espessura 2, comprimento 20) com `w` de largura.
fn volume_aba(w: f64) -> f64 {
    PI / 4. * (16. - 4.) * w + 20. * 2. * w
}

#[test]
fn recuos_dos_quatro_modos() {
    let v = Vars::new();
    let ends = [[0., 0., 2.], [50., 0., 2.]];
    let o = |e: &ExtensaoAba, ends, inv| recuos_extensao(e, 50., ends, inv, &v).unwrap();
    assert_eq!(o(&ExtensaoAba::Inteira, ends, false), (0., 0.));
    assert_eq!(o(&ExtensaoAba::Centrada { largura: s("20") }, ends, false), (15., 15.));
    let desl = ExtensaoAba::Deslocada { inicio: s("10"), largura: s("20") };
    assert_eq!(o(&desl, ends, false), (10., 20.));
    // o kernel entregou a aresta ao contrário: a aba continua em x de 10 a 30, mas os recuos são
    // contados a partir da ponta 0 do kernel (x = 50), então saem trocados
    assert_eq!(o(&desl, [ends[1], ends[0]], false), (20., 10.));
    // inverter mede a partir da outra ponta
    assert_eq!(o(&desl, ends, true), (20., 10.));
    // entre duas referências (projetadas sobre a aresta)
    let entre = ExtensaoAba::Entre { a: [35., 7., 2.], b: [5., -3., 2.] };
    assert_eq!(o(&entre, ends, false), (5., 15.));
    // erros claros
    assert!(recuos_extensao(&ExtensaoAba::Centrada { largura: s("60") }, 50., ends, false, &v).unwrap_err().contains("maior que a aresta"));
    assert!(recuos_extensao(&ExtensaoAba::Deslocada { inicio: s("40"), largura: s("20") }, 50., ends, false, &v).unwrap_err().contains("passa do fim"));
    assert!(recuos_extensao(&ExtensaoAba::Entre { a: [10., 0., 2.], b: [10., 5., 2.] }, 50., ends, false, &v).unwrap_err().contains("mesmo ponto"));
}

#[test]
fn aba_centrada_tem_a_largura_pedida() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    d.add_feature(laco(vec![[100., 25., 2.]], ExtensaoAba::Centrada { largura: s("20") }, false, false));
    let (v, c, lo, hi) = mede(&d);
    assert!(perto(v, 10000. + volume_aba(20.), 1e-3), "volume {v}");
    // centrada: o centro de massa fica no meio da aresta (y = 25) e a caixa não cresce em y
    assert!((c[1] - 25.).abs() < 1e-3, "centro y {}", c[1]);
    assert!(lo[1] > -1e-4 && (hi[1] - 50.).abs() < 1e-3, "caixa y {} {}", lo[1], hi[1]);
    assert!((hi[2] - 24.).abs() < 0.05, "topo da aba {}", hi[2]);
}

#[test]
fn aba_deslocada_e_entre_referencias_batem_com_a_aba_comum() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // referência: aba comum com recuos de 10 e 20 (largura 20 a partir de y = 10)
    let mut r = placa();
    r.add_feature(aba([100., 25., 2.], "10", "20", false));
    let (vr, cr, _, _) = mede(&r);
    assert!(perto(vr, 10000. + volume_aba(20.), 1e-3));
    // o centro de massa da aba está em y = 20, então o conjunto sobe acima de 25 - 1/2 da conta
    let esperado_y = (10000. * 25. + volume_aba(20.) * 20.) / vr;
    assert!((cr[1] - esperado_y).abs() < 0.05, "centro y {} esperado {esperado_y}", cr[1]);
    // deslocada: largura 20 a partir de 10 da ponta de menor y
    let mut d = placa();
    d.add_feature(laco(vec![[100., 25., 2.]], ExtensaoAba::Deslocada { inicio: s("10"), largura: s("20") }, false, false));
    let (v, c, _, _) = mede(&d);
    assert!(perto(v, vr, 1e-6), "volume {v} x {vr}");
    assert!((c[1] - cr[1]).abs() < 1e-6 && (c[0] - cr[0]).abs() < 1e-6, "centro {c:?} x {cr:?}");
    // entre duas referências: de y = 10 a y = 30 (pontos fora da linha são projetados)
    let mut d = placa();
    d.add_feature(laco(vec![[100., 25., 2.]], ExtensaoAba::Entre { a: [100., 30., 2.], b: [90., 10., 2.] }, false, false));
    let (v, c, _, _) = mede(&d);
    assert!(perto(v, vr, 1e-6), "volume {v} x {vr}");
    assert!((c[1] - cr[1]).abs() < 1e-6, "centro y {} x {}", c[1], cr[1]);
    // inverter: a mesma aba medida a partir da ponta de maior y fica em y = 20 a 40
    let mut d = placa();
    let mut k = laco(vec![[100., 25., 2.]], ExtensaoAba::Deslocada { inicio: s("10"), largura: s("20") }, false, false);
    if let FeatureKind::SmFlangeLoop(l) = &mut k {
        l.invert = true;
    }
    d.add_feature(k);
    let (_, ci, _, _) = mede(&d);
    let esperado_y = (10000. * 25. + volume_aba(20.) * 30.) / (10000. + volume_aba(20.));
    assert!((ci[1] - esperado_y).abs() < 0.05, "invertida: centro y {} esperado {esperado_y}", ci[1]);
}

#[test]
fn extensao_que_nao_cabe_vira_erro_claro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    let id = d.add_feature(laco(vec![[100., 25., 2.]], ExtensaoAba::Centrada { largura: s("80") }, false, false));
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Error(m) => assert!(m.contains("maior que a aresta"), "{m}"),
        o => panic!("esperava erro, veio {o:?}"),
    }
}

#[test]
fn laco_retangular_gera_quatro_abas_com_mitra() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // quatro abas num comando só, a partir de uma única aresta (laço da face de topo)
    let mut d = placa();
    d.add_feature(laco(vec![[100., 25., 2.]], ExtensaoAba::Inteira, true, true));
    let r = recompute(&d);
    for f in &d.features {
        assert!(matches!(r.status[&f.id], Status::Ok | Status::Warning(_)), "{}: {:?}", f.name, r.status[&f.id]);
    }
    assert_eq!(r.bend_lines.len(), 4, "uma linha de dobra por aba");
    let (v, c, lo, hi) = mede(&d);
    // as mesmas quatro abas feitas uma a uma, com mitra, dão exatamente o mesmo sólido
    let mut ref_doc = placa();
    for e in [[100., 25., 2.], [50., 50., 2.], [0., 25., 2.], [50., 0., 2.]] {
        ref_doc.add_feature(aba(e, "0", "0", true));
    }
    let (vr, cr, lor, hir) = mede(&ref_doc);
    assert!(perto(v, vr, 1e-6), "volume {v} x quatro abas {vr}");
    for i in 0..3 {
        assert!((c[i] - cr[i]).abs() < 1e-4 && (lo[i] - lor[i]).abs() < 1e-3 && (hi[i] - hir[i]).abs() < 1e-3, "eixo {i}: {c:?} {lo:?} {hi:?} x {cr:?} {lor:?} {hir:?}");
    }
    // a caixa: a placa de 100x50 mais a dobra (R + t = 4) de cada lado, e 20 mm de aba por cima
    assert!((hi[2] - 24.).abs() < 0.05, "topo {}", hi[2]);
    assert!((hi[0] - 104.).abs() < 0.05 && (lo[0] + 4.).abs() < 0.05, "caixa x {} {}", lo[0], hi[0]);
    assert!((hi[1] - 54.).abs() < 0.05 && (lo[1] + 4.).abs() < 0.05, "caixa y {} {}", lo[1], hi[1]);
    // a mitra tira material: menos que quatro abas inteiras sem mitra
    let mut sem = placa();
    for e in [[100., 25., 2.], [50., 50., 2.], [0., 25., 2.], [50., 0., 2.]] {
        sem.add_feature(aba(e, "0", "0", false));
    }
    let (vs, _, _, _) = mede(&sem);
    assert!(v < vs - 1., "com mitra {v} x sem mitra {vs}");
}

#[test]
fn laco_so_com_as_arestas_escolhidas() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // duas arestas escolhidas à mão (sem laço): duas abas, cada uma com a largura centrada
    let mut d = placa();
    d.add_feature(laco(vec![[100., 25., 2.], [0., 25., 2.]], ExtensaoAba::Centrada { largura: s("30") }, false, false));
    let (v, _, _, _) = mede(&d);
    assert!(perto(v, 10000. + 2. * volume_aba(30.), 1e-3), "volume {v}");
}

#[test]
fn laco_de_malha_acha_o_contorno_da_face_de_topo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let d = placa();
    let r = recompute(&d);
    let m = &r.bodies[0].mesh;
    // a aresta de topo em x = 100: o ponto médio dela é a semente
    let semente = (0..m.edge_faces.len())
        .min_by(|a, b| {
            let dist = |e: usize| geo3d::dist([m.edge_mids[e * 3] as f64, m.edge_mids[e * 3 + 1] as f64, m.edge_mids[e * 3 + 2] as f64], [100., 25., 2.]);
            dist(*a).total_cmp(&dist(*b))
        })
        .unwrap();
    let laco = laco_de_malha(m, semente);
    assert_eq!(laco.len(), 4, "{laco:?}");
    // todas na altura da face de topo (z = 2), e as quatro bordas do retângulo
    assert!(laco.iter().all(|p| (p[2] - 2.).abs() < 1e-4), "{laco:?}");
    for esperado in [[100., 25.], [50., 50.], [0., 25.], [50., 0.]] {
        assert!(laco.iter().any(|p| (p[0] - esperado[0]).abs() < 1e-3 && (p[1] - esperado[1]).abs() < 1e-3), "falta {esperado:?} em {laco:?}");
    }
}

/// B31: valor fora da faixa que o painel pinta de vermelho não pode deixar o selo em OK. O
/// recálculo falha com o limite escrito, na aba de aresta e no laço.
#[test]
fn aba_com_valor_fora_da_faixa_vira_erro_com_o_limite() {
    for (campo, valor, trecho) in [("angle", "0", "ângulo"), ("angle", "200", "180"), ("length", "-3", "comprimento"), ("radius", "0", "raio")] {
        let mut d = placa();
        let mut a = aba([100., 25., 2.], "0", "0", false);
        if let FeatureKind::SmFlange(f) = &mut a {
            match campo {
                "angle" => f.angle = s(valor),
                "length" => f.length = s(valor),
                _ => f.radius = Some(s(valor)),
            }
        }
        let id = d.add_feature(a);
        match &recompute(&d).status[&id] {
            Status::Error(m) => assert!(m.contains(trecho) && m.contains("faixa"), "{campo}={valor}: {m}"),
            o => panic!("{campo}={valor}: esperava erro, veio {o:?}"),
        }
    }
    let mut d = placa();
    let FeatureKind::SmFlangeLoop(mut l) = laco(vec![[100., 25., 2.]], ExtensaoAba::Inteira, false, false) else { panic!() };
    l.angle = s("181");
    let id = d.add_feature(FeatureKind::SmFlangeLoop(l));
    assert!(matches!(&recompute(&d).status[&id], Status::Error(m) if m.contains("ângulo")));
    // dentro da faixa segue OK
    let mut d = placa();
    let id = d.add_feature(aba([100., 25., 2.], "0", "0", false));
    assert!(matches!(recompute(&d).status[&id], Status::Ok | Status::Warning(_)));
}

#[test]
fn aba_laco_sobrevive_a_salvar_e_abrir() {
    let mut d = placa();
    d.add_feature(laco(vec![[100., 25., 2.]], ExtensaoAba::Deslocada { inicio: s("10"), largura: s("20") }, true, true));
    let json = serde_json::to_string(&d).unwrap();
    let volta: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(volta.features.last().unwrap().kind, d.features.last().unwrap().kind);
}
