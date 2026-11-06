// SPDX-License-Identifier: LGPL-2.1-or-later
//! B35: aba entre perfis (lofted), desdobrar/redobrar parcial e converter sólido em chapa.

use super::*;
use fenkra_core::{SmConvert, SmLoftFlange, SmUnfoldPartial};
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
    let mut n = 4_200_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [0., 0.], [100., 50.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

fn aba(d: &mut Document, e: [f64; 3]) -> Id {
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: e, length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }))
}

struct Medida {
    volume: f64,
    lo: [f64; 3],
    hi: [f64; 3],
}

fn mede(d: &Document) -> Medida {
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
    Medida { volume: b.mass.volume, lo, hi }
}

fn erro(d: &Document, id: Id) -> String {
    match &recompute(d).status[&id] {
        Status::Error(m) => m.clone(),
        o => panic!("esperava erro, veio {o:?}"),
    }
}

// ---------- desdobrar / redobrar parcial ----------

/// Volume da aba dobrada de 90° (raio 2, espessura 2, reta de 20) com largura `w`.
fn volume_dobrada(w: f64) -> f64 {
    PI / 4. * (16. - 4.) * w + 20. * 2. * w
}

/// Volume da aba planificada: tira de espessura 2 com o comprimento desenvolvido.
fn volume_plana(w: f64) -> f64 {
    let ba = chapa::compensacao(2., 2., 90., 0.44).ba;
    (ba + 20.) * 2. * w
}

#[test]
fn desdobrar_parcial_planifica_so_a_aba_escolhida() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    let a1 = aba(&mut d, [100., 25., 2.]);
    let a2 = aba(&mut d, [0., 25., 2.]);
    let dobrada = mede(&d);
    assert!(perto(dobrada.volume, 10000. + 2. * volume_dobrada(50.), 1e-3));
    assert!((dobrada.hi[2] - 24.).abs() < 0.05);
    d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial { features: vec![a2], refold: false }));
    let m = mede(&d);
    // a segunda aba deitou: o comprimento desenvolvido sai da aresta x = 0 para o lado de fora
    let ba = chapa::compensacao(2., 2., 90., 0.44).ba;
    assert!((m.lo[0] + (ba + 20.)).abs() < 0.05, "x mínimo {} esperado {}", m.lo[0], -(ba + 20.));
    // a primeira continua dobrada: a altura da peça é a da aba em pé
    assert!((m.hi[2] - 24.).abs() < 0.05, "z máximo {}", m.hi[2]);
    assert!((m.hi[0] - 104.).abs() < 0.05, "a aba dobrada de x = 100 continua em pé: x máximo {}", m.hi[0]);
    // o volume troca a aba dobrada pela tira plana
    assert!(perto(m.volume, 10000. + volume_dobrada(50.) + volume_plana(50.), 2e-3), "volume {} esperado {}", m.volume, 10000. + volume_dobrada(50.) + volume_plana(50.));
    let _ = a1;
}

#[test]
fn redobrar_parcial_volta_ao_estado_dobrado() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    let _a1 = aba(&mut d, [100., 25., 2.]);
    let a2 = aba(&mut d, [0., 25., 2.]);
    let dobrada = mede(&d);
    d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial { features: vec![a2], refold: false }));
    d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial { features: vec![a2], refold: true }));
    let m = mede(&d);
    assert!(perto(m.volume, dobrada.volume, 1e-9), "{} x {}", m.volume, dobrada.volume);
    for i in 0..3 {
        assert!((m.lo[i] - dobrada.lo[i]).abs() < 1e-4 && (m.hi[i] - dobrada.hi[i]).abs() < 1e-4);
    }
}

#[test]
fn desdobrar_todas_as_abas_e_a_planificada_inteira() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    let a1 = aba(&mut d, [100., 25., 2.]);
    let a2 = aba(&mut d, [0., 25., 2.]);
    let mut global = d.clone();
    global.sheet.as_mut().unwrap().flat = true;
    let g = mede(&global);
    d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial { features: vec![a1, a2], refold: false }));
    let m = mede(&d);
    assert!(perto(m.volume, g.volume, 1e-6), "parcial {} x global {}", m.volume, g.volume);
    for i in 0..3 {
        assert!((m.lo[i] - g.lo[i]).abs() < 1e-3 && (m.hi[i] - g.hi[i]).abs() < 1e-3, "eixo {i}: {:?} {:?} x {:?} {:?}", m.lo, m.hi, g.lo, g.hi);
    }
}

#[test]
fn desdobrar_parcial_confere_as_abas_escolhidas() {
    let mut d = placa();
    let a1 = aba(&mut d, [100., 25., 2.]);
    let vazio = d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial::default()));
    let parede = d.features[1].id;
    let nao_aba = d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial { features: vec![parede], refold: false }));
    let _ = a1;
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    assert!(erro(&d, vazio).contains("pelo menos uma aba"));
    assert!(erro(&d, nao_aba).contains("só abas"));
}

#[test]
fn desdobrar_parcial_sobrevive_a_salvar_e_abrir() {
    let mut d = placa();
    let a1 = aba(&mut d, [100., 25., 2.]);
    d.add_feature(FeatureKind::SmUnfoldPartial(SmUnfoldPartial { features: vec![a1], refold: true }));
    let volta: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
    assert_eq!(volta.features.last().unwrap().kind, d.features.last().unwrap().kind);
    assert_eq!(chapa_desdobra::desdobradas(&volta).len(), 0, "desdobrar e redobrar a mesma aba não deixa ninguém desdobrado");
}

// ---------- aba entre perfis ----------

fn perfil(d: &mut Document, plano: PlaneRef, meio: f64, n0: u64) -> Id {
    let mut data = SketchData::default();
    let mut n = n0;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, [-meio, -meio], [meio, meio], false);
    d.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }))
}

fn funil(ruled: bool, flip: bool) -> (Document, Id) {
    let mut d = Document::new("funil");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let a = perfil(&mut d, PlaneRef::Origin(OriginPlane::XY), 20., 4_300_000);
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
    let b = perfil(&mut d, PlaneRef::Datum(topo), 10., 4_310_000);
    let id = d.add_feature(FeatureKind::SmLoftFlange(SmLoftFlange { a, b, ruled, flip }));
    (d, id)
}

#[test]
fn aba_entre_perfis_facetada_e_uma_casca_da_espessura_da_chapa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, _) = funil(true, false);
    let m = {
        let r = recompute(&d);
        let b = &r.bodies[0];
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i] as f64);
                hi[i] = hi[i].max(p[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    };
    // o tronco maciço teria 28000 mm³; a casca de 2 mm tem a área lateral (~3795 mm²) vezes 2
    assert!(m.0 > 6500. && m.0 < 8500., "volume {}", m.0);
    // os perfis são a face de fora: a caixa é a deles (±20 na base, 30 de altura)
    assert!((m.1[0] + 20.).abs() < 0.05 && (m.2[0] - 20.).abs() < 0.05, "x {} {}", m.1[0], m.2[0]);
    assert!(m.1[2].abs() < 0.05 && (m.2[2] - 30.).abs() < 0.05, "z {} {}", m.1[2], m.2[2]);
}

#[test]
fn aba_entre_perfis_lisa_e_invertida() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (lisa, _) = funil(false, false);
    let r = recompute(&lisa);
    assert!(matches!(r.status[&lisa.features.last().unwrap().id], Status::Ok | Status::Warning(_)), "{:?}", r.status);
    assert!(r.bodies[0].mass.volume > 5000.);
    // invertida: a espessura cresce para fora dos perfis, então a caixa passa de ±20
    let (fora, id) = funil(true, true);
    let r = recompute(&fora);
    assert!(matches!(r.status[&id], Status::Ok | Status::Warning(_)), "{:?}", r.status[&id]);
    let b = &r.bodies[0];
    let xmax = b.mesh.positions.chunks_exact(3).map(|p| p[0] as f64).fold(f64::MIN, f64::max);
    assert!(xmax > 20.5, "x máximo {xmax}");
}

#[test]
fn aba_entre_perfis_erra_em_palavras() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // os dois perfis no mesmo lugar
    let mut d = Document::new("funil");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let a = perfil(&mut d, PlaneRef::Origin(OriginPlane::XY), 20., 4_320_000);
    let b = perfil(&mut d, PlaneRef::Origin(OriginPlane::XY), 10., 4_330_000);
    let id = d.add_feature(FeatureKind::SmLoftFlange(SmLoftFlange { a, b, ruled: true, flip: false }));
    assert!(erro(&d, id).contains("mesmo lugar"));
    // perfil aberto
    let mut d = Document::new("funil");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 4_340_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let p0 = data.add_point(&mut al, 0., 0.);
    let p1 = data.add_point(&mut al, 10., 0.);
    data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: p0, b: p1 }, false);
    let aberto = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
    let b = perfil(&mut d, PlaneRef::Datum(topo), 10., 4_350_000);
    let id = d.add_feature(FeatureKind::SmLoftFlange(SmLoftFlange { a: aberto, b, ruled: true, flip: false }));
    assert!(erro(&d, id).contains("contorno fechado"));
}

// ---------- converter sólido em chapa ----------

fn bloco() -> Document {
    let mut d = Document::new("bloco");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let sk = perfil(&mut d, PlaneRef::Origin(OriginPlane::XY), 15., 4_360_000);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    d
}

#[test]
fn converte_reconhece_espessura_e_dobras_de_uma_chapa_dobrada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    aba(&mut d, [100., 25., 2.]);
    aba(&mut d, [0., 25., 2.]);
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: String::new() }));
    let a = chapa_converte::analisar_corpo(&d, id).unwrap();
    assert!((a.espessura - 2.).abs() < 1e-3, "espessura {}", a.espessura);
    assert_eq!(a.dobras.len(), 2, "{:?}", a.dobras);
    for b in &a.dobras {
        assert!((b.raio - 2.).abs() < 0.02, "raio {}", b.raio);
        assert!((b.angulo - 90.).abs() < 0.5, "ângulo {}", b.angulo);
        assert!((b.largura - 50.).abs() < 0.1, "largura {}", b.largura);
    }
    assert!((a.raio_minimo().unwrap() - 2.).abs() < 0.02);
    // o comando avisa o que reconheceu, sem mudar o corpo
    let r = recompute(&d);
    match &r.status[&id] {
        Status::Warning(m) => assert!(m.contains("2 dobra(s)") && m.contains("espessura 2.00"), "{m}"),
        o => panic!("esperava aviso, veio {o:?}"),
    }
    let sem = {
        let mut x = d.clone();
        x.features.pop();
        recompute(&x).bodies[0].mass.volume
    };
    assert!(perto(r.bodies[0].mass.volume, sem, 1e-9));
}

#[test]
fn converte_chapa_plana_sem_dobras() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [50., 25., 2.], thickness: s("2") }));
    let a = chapa_converte::analisar_corpo(&d, id).unwrap();
    assert_eq!(a.dobras.len(), 0);
    assert!(a.resumo().contains("chapa plana"));
    assert!(matches!(recompute(&d).status[&id], Status::Warning(_) | Status::Ok));
}

#[test]
fn converte_recusa_bloco_maciço_e_espessura_errada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = bloco();
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: String::new() }));
    assert!(erro(&d, id).contains("espessura não é constante"), "{}", erro(&d, id));
    // espessura pedida diferente da medida
    let mut d = placa();
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: s("3") }));
    assert!(erro(&d, id).contains("pedidos"));
    // face fixa que não é da chapa
    let mut d = placa();
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [100., 25., 1.], thickness: String::new() }));
    assert!(erro(&d, id).contains("face fixa"));
}

#[test]
fn converte_libera_os_comandos_de_chapa_sobre_o_corpo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // um corpo convertido recebe uma aba depois, como qualquer chapa
    let mut d = placa();
    d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: String::new() }));
    aba(&mut d, [100., 25., 2.]);
    let m = mede(&d);
    assert!(perto(m.volume, 10000. + volume_dobrada(50.), 1e-3));
}

// ---------- converter: mapa de dobras e planificada ----------

/// Canal em U sem nenhum comando de chapa (um corpo "de fora"): seção fechada com arcos exatos,
/// espessura 2, raio interno 2, 40 mm de vão interno e 18 mm de altura de aba, extrudada 50 mm.
fn canal_u() -> Document {
    use fenkra_core::sketch::Curve;
    let mut d = Document::new("canal");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut n = 4_400_000u64;
    let mut al = || {
        n += 1;
        Id(n)
    };
    let pts = [(0., 20.), (0., 4.), (4., 0.), (40., 0.), (44., 4.), (44., 20.), (42., 20.), (42., 4.), (40., 2.), (4., 2.), (2., 4.), (2., 20.), (4., 4.), (40., 4.)];
    let p: Vec<Id> = pts.iter().map(|(x, y)| data.add_point(&mut al, *x, *y)).collect();
    // contorno: lado de fora, topo da aba direita, lado de dentro e topo da aba esquerda
    let linhas = [(0, 1), (2, 3), (4, 5), (5, 6), (6, 7), (8, 9), (10, 11), (11, 0)];
    for (a, b) in linhas {
        data.add_curve(&mut al, Curve::Line { a: p[a], b: p[b] }, false);
    }
    // cantos: centro, começo e fim (anti-horário)
    for (c, a, b) in [(12, 1, 2), (13, 3, 4), (13, 8, 7), (12, 10, 9)] {
        data.add_curve(&mut al, Curve::Arc { c: p[c], s: p[a], e: p[b] }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("50")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None, ends: Default::default() }));
    d
}

fn extensao(d: &Document) -> ([f64; 3], [f64; 3], f64) {
    let r = recompute(d);
    let b = &r.bodies[0];
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in b.mesh.positions.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] as f64);
            hi[i] = hi[i].max(p[i] as f64);
        }
    }
    (lo, hi, b.mass.volume)
}

/// O corpo de fora ganha o mapa de dobras: a tabela lista as duas dobras do canal, com o
/// comprimento desenvolvido da conta, e a chapa planificada tem o comprimento desenvolvido total,
/// a largura de 50 e a espessura de 2.
#[test]
fn converter_cria_o_mapa_de_dobras_e_planifica_o_corpo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = canal_u();
    let id = d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: s("2") }));
    let r = recompute(&d);
    let aviso = match &r.status[&id] {
        Status::Warning(m) => m.clone(),
        o => panic!("esperava aviso, veio {o:?}"),
    };
    assert!(aviso.contains("2 dobra(s) no mapa de dobras"), "{aviso}");
    let ba = chapa::compensacao(2., 2., 90., 0.44).ba;
    assert_eq!(r.bend_lines.len(), 2, "uma linha de dobra por dobra reconhecida");
    let tab = tabela_dobras(&d, &r.bend_lines);
    assert_eq!(tab.len(), 2);
    for linha in &tab {
        assert!((linha.angle - 90.).abs() < 0.5 && (linha.radius - 2.).abs() < 0.02, "{linha:?}");
        assert!((linha.allowance - ba).abs() < 0.02, "{linha:?}");
        assert_eq!(linha.feature, id);
    }
    // as duas dobras fecham para o mesmo lado da face de referência
    assert!(r.bend_lines.iter().all(|b| b.up == r.bend_lines[0].up));
    // dobrado: o corpo não mudou
    let (_, _, v_dobrado) = extensao(&d);
    let mut sem = d.clone();
    sem.features.pop();
    assert!(perto(v_dobrado, extensao(&sem).2, 1e-9));
    // planificada: trechos retos (36 do fundo + 2 x 16 das abas) e os dois arcos desenvolvidos
    let mut plana = d.clone();
    plana.sheet.as_mut().unwrap().flat = true;
    let (lo, hi, v) = extensao(&plana);
    let total = 36. + 2. * 16. + 2. * ba;
    let ext = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let mut o = ext;
    o.sort_by(|a, b| b.total_cmp(a));
    assert!((o[0] - total).abs() < 0.2, "comprimento desenvolvido {ext:?} contra {total}");
    assert!((o[1] - 50.).abs() < 0.1 && (o[2] - 2.).abs() < 0.1, "largura e espessura {ext:?}");
    assert!(perto(v, total * 2. * 50., 5e-3), "volume plano {v} contra {}", total * 100.);
    // a tabela do mapa é a mesma planificada, e as linhas de dobra caem dentro do blank
    let rp = recompute(&plana);
    assert_eq!(rp.bend_lines.len(), 2);
    for b in &rp.bend_lines {
        for p in &b.center {
            assert!((0..3).all(|i| p[i] >= lo[i] - 0.01 && p[i] <= hi[i] + 0.01), "linha de dobra fora do blank: {p:?}");
        }
    }
}

/// A face fixa escolhe a base: com a face externa de uma aba como base, a planificada tem o mesmo
/// comprimento desenvolvido e o mesmo volume.
#[test]
fn converter_com_face_fixa_numa_aba_planifica_igual() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = canal_u();
    // face externa da aba esquerda (x = 0, no meio da altura reta)
    d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0., 12., 25.], thickness: String::new() }));
    let mut plana = d.clone();
    plana.sheet.as_mut().unwrap().flat = true;
    let ba = chapa::compensacao(2., 2., 90., 0.44).ba;
    let total = 36. + 2. * 16. + 2. * ba;
    let (lo, hi, v) = extensao(&plana);
    let comp = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(hi[2] - lo[2]);
    assert!((comp - total).abs() < 0.2, "{comp} contra {total}");
    assert!(perto(v, total * 100., 5e-3), "{v}");
}

/// Chapa plana convertida não tem dobra: nada a mapear, e a planificada é o próprio corpo.
#[test]
fn converter_chapa_plana_planifica_sem_mapa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = placa();
    d.add_feature(FeatureKind::SmConvert(SmConvert { face: [0.; 3], thickness: String::new() }));
    d.sheet.as_mut().unwrap().flat = true;
    let (_, _, v) = extensao(&d);
    assert!(perto(v, 10000., 1e-6));
    assert!(recompute(&d).bend_lines.is_empty());
}
