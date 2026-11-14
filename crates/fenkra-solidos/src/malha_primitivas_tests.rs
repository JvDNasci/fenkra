// SPDX-License-Identifier: LGPL-2.1-or-later
//! B169: malha → STEP sólido. Cada teste monta um sólido conhecido em código, tira a malha dele
//! (como um STL exportado), converte de volta pelo comando da árvore e confere status, volume,
//! caixa, os tipos de face e o desvio máximo entre a malha e o sólido.

use crate::corpo_malha::{embutir, malha_da_forma};
use crate::{final_shape, recompute, Status};
use fenkra_core::malha::{MeshOp, MeshOpKind, ModoBrep};
use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_malha::desvio::desvio;
use fenkra_malha::{Bvh, Malha};
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.into()
}

fn xy() -> PlaneRef {
    PlaneRef::Origin(OriginPlane::XY)
}

fn prim(d: &mut Document, kind: PrimKind, a: &str, b: &str, c: &str, dd: &str, op: BoolOp) {
    d.add_feature(FeatureKind::Primitive(Primitive { kind, plane: xy(), a: s(a), b: s(b), c: s(c), d: s(dd), op }));
}

/// Malha do sólido do documento, grossa como um STL comum (desvio de 0,05 mm, 20°).
fn malha_de(d: &Document) -> (Shape, Malha) {
    let f = final_shape(d).expect("forma");
    let m = malha_da_forma(&f, 0.05, 20f64.to_radians()).expect("malha");
    assert!(m.fechada(), "a malha de prova fecha");
    (f, m)
}

struct Conversao {
    forma: Shape,
    nota: String,
    tipos: [u32; 6],
}

fn converter(m: &Malha, modo: ModoBrep) -> Conversao {
    let mut d = Document::new("convertida");
    let dados = embutir(&mut d, m);
    d.add_feature(FeatureKind::MeshImport(MeshImport { path: "peça.stl".into(), solid: false, corpo: true, dados }));
    let op = d.add_feature(FeatureKind::MeshOp(MeshOp { op: MeshOpKind::ParaBrep { solid: true, modo, tol: String::new() } }));
    let r = recompute(&d);
    let nota = match &r.status[&op] {
        Status::Ok => String::new(),
        Status::Warning(w) => w.clone(),
        x => panic!("conversão: {x:?}"),
    };
    let nota = r.notes.get(&op).map(|n| n.text.clone()).filter(|t| !t.is_empty()).unwrap_or(nota);
    let forma = final_shape(&d).expect("sólido convertido");
    assert!(forma.is_solid(), "sólido");
    assert!(forma.is_valid(), "B-rep válido");
    let t = forma.face_types().expect("tipos");
    Conversao { forma, nota, tipos: [t[0], t[1], t[2], t[3], t[4], t[5]] }
}

/// Maior distância da malha até o sólido e do sólido (tesselado fino) até a malha.
fn desvio_maximo(m: &Malha, s: &Shape) -> f64 {
    let fina = malha_da_forma(s, 0.002, 5f64.to_radians()).expect("malha fina");
    desvio(m, &Bvh::nova(&fina)).maximo.max(desvio(&fina, &Bvh::nova(m)).maximo)
}

fn caixa(s: &Shape) -> ([f64; 3], [f64; 3]) {
    s.bbox().expect("caixa")
}

fn perto(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < tol)
}

#[test]
fn caixa_com_furo_vira_planos_e_cilindro_com_volume_exato() {
    let mut d = Document::new("caixa");
    prim(&mut d, PrimKind::Caixa, "60", "40", "20", "0", BoolOp::Add);
    let (orig, m) = malha_de(&d);
    let (lo0, hi0) = caixa(&orig);
    // furo passante de raio 8 no meio
    let mut d = Document::new("furo");
    prim(&mut d, PrimKind::Caixa, "60", "40", "20", "0", BoolOp::Add);
    let c = [(lo0[0] + hi0[0]) / 2., (lo0[1] + hi0[1]) / 2.];
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: xy(), a: s("8"), b: s("40"), c: s("0"), d: s("0"), op: BoolOp::Cut }));
    let (orig, m2) = malha_de(&d);
    let _ = (m, c);
    let v0 = orig.mass().volume;
    let conv = converter(&m2, ModoBrep::Primitivas);
    let [planos, cil, ..] = conv.tipos;
    assert_eq!(cil, 1, "o furo vira um cilindro: {:?} ({})", conv.tipos, conv.nota);
    assert_eq!(planos, 6, "seis faces planas, fundidas: {:?}", conv.tipos);
    assert_eq!(conv.tipos[5], 0, "sem face triangular sobrando");
    let v = conv.forma.mass().volume;
    assert!((v - v0).abs() < 1e-4 * v0, "volume {v} contra {v0}: o cilindro exato não perde a corda");
    let (lo, hi) = caixa(&conv.forma);
    let (lo0, hi0) = caixa(&orig);
    // a caixa do kernel soma a tolerância das arestas, que guarda a corda da malha
    assert!(perto(lo, lo0, 0.02) && perto(hi, hi0, 0.02), "{lo:?} {hi:?}");
    let dmax = desvio_maximo(&m2, &conv.forma);
    assert!(dmax < 0.06, "desvio máximo {dmax}: a corda da malha");
    assert!(conv.nota.contains("cilindro"), "{}", conv.nota);
    assert!(conv.nota.contains("desvio"), "{}", conv.nota);
}

#[test]
fn facetado_costura_e_funde_as_faces_coplanares() {
    let mut d = Document::new("caixa");
    prim(&mut d, PrimKind::Caixa, "30", "20", "10", "0", BoolOp::Add);
    let (orig, m) = malha_de(&d);
    assert!(m.tris.len() >= 12);
    let conv = converter(&m, ModoBrep::Facetado);
    assert_eq!(conv.forma.face_count(), 6, "doze triângulos viram seis faces: {:?}", conv.tipos);
    assert!((conv.forma.mass().volume - 6000.).abs() < 1e-6);
    let (lo, hi) = caixa(&conv.forma);
    let (lo0, hi0) = caixa(&orig);
    assert!(perto(lo, lo0, 1e-6) && perto(hi, hi0, 1e-6));
    assert!(desvio_maximo(&m, &conv.forma) < 1e-6);
}

#[test]
fn cilindro_esfera_cone_e_toro_sao_reconhecidos() {
    let casos: [(PrimKind, [&str; 4], usize, f64); 4] = [
        (PrimKind::Cilindro, ["10", "30", "0", "0"], 1, PI * 100. * 30.),
        (PrimKind::Esfera, ["12", "0", "0", "0"], 3, 4. / 3. * PI * 12f64.powi(3)),
        (PrimKind::Cone, ["10", "4", "20", "0"], 2, PI * 20. / 3. * (100. + 40. + 16.)),
        (PrimKind::Toro, ["20", "5", "0", "0"], 4, 2. * PI * PI * 20. * 25.),
    ];
    for (k, [a, b, c, dd], tipo, vol) in casos {
        let mut d = Document::new("prim");
        prim(&mut d, k, a, b, c, dd, BoolOp::Add);
        let (orig, m) = malha_de(&d);
        let v0 = orig.mass().volume;
        assert!((v0 - vol).abs() < 1e-3 * vol, "{k:?}: volume de partida {v0} contra {vol}");
        let conv = converter(&m, ModoBrep::Primitivas);
        assert!(conv.tipos[tipo] >= 1, "{k:?}: {:?} ({})", conv.tipos, conv.nota);
        assert_eq!(conv.tipos[5], 0, "{k:?}: sobrou face sem ajuste {:?} ({})", conv.tipos, conv.nota);
        let v = conv.forma.mass().volume;
        assert!((v - vol).abs() < 2e-3 * vol, "{k:?}: volume {v} contra {vol} (malha {})", m.volume());
        let (lo, hi) = caixa(&conv.forma);
        let (lo0, hi0) = caixa(&orig);
        assert!(perto(lo, lo0, 0.1) && perto(hi, hi0, 0.1), "{k:?}: a caixa de partida vem folgada pela malha do kernel: {lo:?} {hi:?} contra {lo0:?} {hi0:?}");
        let dmax = desvio_maximo(&m, &conv.forma);
        assert!(dmax < 0.06, "{k:?}: desvio máximo {dmax}");
    }
}

#[test]
fn caixa_arredondada_tem_filetes_cilindricos_e_cantos_esfericos() {
    let mut d = Document::new("arredondada");
    prim(&mut d, PrimKind::Caixa, "40", "40", "40", "0", BoolOp::Add);
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("5"), edges: EdgeSel::AllOfBody, mode: Default::default(), prop: Default::default(), keys: vec![], extra: Default::default() }));
    let (orig, m) = malha_de(&d);
    let conv = converter(&m, ModoBrep::Primitivas);
    let [planos, cil, _, esf, ..] = conv.tipos;
    assert_eq!(planos, 6, "{:?} ({})", conv.tipos, conv.nota);
    assert_eq!(cil, 12, "{:?} ({})", conv.tipos, conv.nota);
    assert_eq!(esf, 8, "{:?} ({})", conv.tipos, conv.nota);
    let (v, v0) = (conv.forma.mass().volume, orig.mass().volume);
    assert!((v - v0).abs() < 1e-3 * v0, "volume {v} contra {v0}");
    assert!(desvio_maximo(&m, &conv.forma) < 0.06);
}

#[test]
fn placa_de_exemplo_com_furo_arredondado_fecha_com_as_curvas_que_costuram() {
    let m = crate::malha_primitivas::malha_de_exemplo();
    let tol = crate::malha_primitivas::tolerancia_automatica(&m);
    let rec = crate::malha_primitivas::reconhecer(&m, tol);
    let ds = crate::malha_primitivas::descrever(&m, &rec, tol);
    let _ = ds;
    let conv = converter(&m, ModoBrep::Primitivas);
    // o furo e os filetes retos continuam cilindros mesmo quando um tipo mais difícil volta a triângulo
    assert!(conv.tipos[1] >= 1, "{:?} ({}) reconhecido: {}", conv.tipos, conv.nota, rec.resumo());
    let v0 = m.volume();
    assert!((conv.forma.mass().volume - v0).abs() < 0.02 * v0);
    assert!(crate::malha_primitivas::desvio_maximo(&m, &conv.forma) < 0.06, "{}", conv.nota);
}

#[test]
fn malha_sem_primitiva_fica_triangular_e_ainda_fecha() {
    // octaedro torto: nenhuma face cabe com outra
    let pos = vec![[0., 0., 10.], [7., 1., 0.], [-1., 6., 1.], [-8., -1., 0.], [2., -7., -1.], [0.5, 0.3, -11.]];
    let tris = vec![[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1], [5, 2, 1], [5, 3, 2], [5, 4, 3], [5, 1, 4]];
    let m = Malha { pos, tris, ..Default::default() };
    assert!(m.volume() > 0.);
    let conv = converter(&m, ModoBrep::Primitivas);
    assert!((conv.forma.mass().volume - m.volume()).abs() < 1e-6 * m.volume());
    assert_eq!(conv.forma.face_count(), 8);
}

#[test]
fn step_ap242_do_convertido_volta_igual() {
    let mut d = Document::new("cil");
    prim(&mut d, PrimKind::Cilindro, "10", "30", "0", "0", BoolOp::Add);
    let (_, m) = malha_de(&d);
    let conv = converter(&m, ModoBrep::Primitivas);
    let pasta = std::env::temp_dir().join(format!("fenkra_b169_{}", std::process::id()));
    std::fs::create_dir_all(&pasta).unwrap();
    let arq = pasta.join("convertido.step");
    let modelo = fenkra_kernel::ModeloXde { produtos: vec![fenkra_kernel::ProdutoXde { nome: "convertido".into(), forma: Some(conv.forma.clone()), ..Default::default() }], raizes: vec![0], ..Default::default() };
    modelo.gravar(arq.to_str().unwrap(), fenkra_kernel::OpcoesGravarXde { esquema: fenkra_kernel::EsquemaStep::Ap242, ..Default::default() }).expect("STEP AP242");
    let texto = std::fs::read_to_string(&arq).unwrap();
    assert!(texto.contains("CYLINDRICAL_SURFACE"), "o STEP leva a superfície analítica");
    let (lida, info) = Shape::import_step_info(arq.to_str().unwrap()).expect("ler STEP");
    assert!(info.esquema.to_ascii_uppercase().contains("AP242"), "esquema {}", info.esquema);
    assert!((lida.mass().volume - conv.forma.mass().volume).abs() < 1e-6 * conv.forma.mass().volume);
    let _ = std::fs::remove_dir_all(&pasta);
}
