//! B223: cada comando de chapa devolve uma malha de prévia com os valores padrão, sem gravar nada
//! no documento, e o que seria inválido volta com a causa em texto.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{previa_comando, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn parametros() -> Option<SheetParams> {
    Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() })
}

fn parede(sk: Id) -> FeatureKind {
    FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None })
}

fn chapa() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = parametros();
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(parede(sk));
    d
}

fn sk_linhas(d: &mut Document, plane: PlaneRef, segs: &[([f64; 2], [f64; 2])]) -> Id {
    let mut data = SketchData::default();
    let mut n = 9_000_000u64 + d.features.len() as u64 * 1000;
    let mut al = || {
        n += 1;
        Id(n)
    };
    for (a, b) in segs {
        let pa = data.add_point(&mut al, a[0], a[1]);
        let pb = data.add_point(&mut al, b[0], b[1]);
        data.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: pa, b: pb }, false);
    }
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
}

/// Os comandos de chapa da barra (menos a aba por superfície, que pede um suporte): a prévia com os
/// valores padrão devolve malha, o documento real não muda e a peça muda de volume.
#[test]
fn todo_comando_de_chapa_devolve_malha_de_previa() {
    if !kernel_disponivel() {
        return;
    }
    // parede base: a prévia é a primeira peça
    let mut v = Document::new("vazio");
    v.sheet = parametros();
    let sk = sk_rect(&mut v, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [60., 40.]);
    let p = previa_comando(&v, parede(sk));
    assert!(p.malha.as_ref().is_some_and(|m| !m.positions.is_empty()) && (p.volume - 60. * 40. * 2.).abs() < 1., "parede base: {:?}", p.status);

    let mut d = chapa();
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
    let quadrado = sk_rect(&mut d, PlaneRef::Datum(topo), [40., 15.], [60., 35.]);
    let pontos = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[50., 25.]]);
    let perfil = sk_linhas(&mut d, PlaneRef::Origin(OriginPlane::XZ), &[([100., 2.], [100., 20.]), ([100., 20.], [110., 20.])]);
    let n_antes = d.features.len();
    let comandos: Vec<(&str, FeatureKind)> = vec![
        ("aba", FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None })),
        ("bainha", FeatureKind::SmHem(SmHem { edge: [100., 25., 2.], length: s("8"), gap: s("0"), flip: false, kind: 0, edge_key: None })),
        ("dobra por linha", FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("90"), radius: None, flip: false, down: false, sketch: None, position: 0 })),
        ("joggle", FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("6"), flip: true, angle: None, plane: None, keep_length: false })),
        ("veneziana", FeatureKind::SmLouver(SmLouver { a: [30., 10., 2.], b: [30., 40., 2.], width: s("12"), height: s("5"), flip: false, down: false })),
        ("estampo", FeatureKind::SmStamp(SmStamp { sketch: quadrado, depth: s("3"), flip: false, kind: 0, width: String::new(), punch: None })),
        ("furo flangeado", FeatureKind::SmFlangedHole(SmFlangedHole { sketch: pontos, diameter: s("8"), height: s("4"), flip: false, points: vec![], standard: String::new() })),
        ("recorte", FeatureKind::SmCut(SmCut { sketch: quadrado, mode: 0, offset: String::new(), draft: s("5"), height: s("4"), flip: false, normal_face: false })),
        ("aba por perfil", FeatureKind::SmProfileFlange(SmProfileFlange { sketch: perfil, l1: s("0"), l2: s("40"), radius: None, flip: false })),
        ("aba curva", FeatureKind::SmCurveFlange(SmCurveFlange { edge: vec![[100., 0., 2.], [100., 25., 2.], [100., 50., 2.]], length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None })),
        ("alívio de canto", FeatureKind::SmCornerRelief(SmCornerRelief { at: [100., 50., 2.], size: s("4"), kind: 0 })),
        ("rasgo circular", FeatureKind::SmArcSlot(SmArcSlot { center: [50., 25., 2.], radius: s("15"), width: s("4"), start: s("0"), sweep: s("90") })),
    ];
    let v_base = fenkra_solidos::recompute(&d).bodies[0].mass.volume;
    for (nome, kind) in comandos {
        let p = previa_comando(&d, kind);
        assert!(matches!(p.status, Status::Ok | Status::Warning(_)), "{nome}: {:?}", p.status);
        let m = p.malha.unwrap_or_else(|| panic!("{nome}: sem malha de prévia"));
        assert!(!m.positions.is_empty() && p.volume > 0., "{nome}: malha vazia");
        // a veneziana conserva o volume (rampa por paralelogramo); os demais mudam a peça
        assert!((p.volume - v_base).abs() > 1e-6 || nome == "veneziana", "{nome}: a prévia não mudou a peça ({} = {v_base})", p.volume);
        assert_eq!(d.features.len(), n_antes, "a prévia não grava no documento");
    }
}

/// Resultado inválido: a prévia vem sem malha e com a causa em texto.
#[test]
fn previa_invalida_traz_a_causa_em_texto() {
    if !kernel_disponivel() {
        return;
    }
    let d = chapa();
    let p = previa_comando(&d, FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("200"), radius: None, flip: false, down: false, sketch: None, position: 0 }));
    assert!(p.malha.is_none());
    assert!(matches!(&p.status, Status::Error(m) if m.contains("ângulo")), "{:?}", p.status);
}
