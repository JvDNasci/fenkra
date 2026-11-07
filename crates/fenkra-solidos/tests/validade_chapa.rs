// SPDX-License-Identifier: LGPL-2.1-or-later
//! B432: a peça de chapa entra na conferência de validade do kernel (B413).
//!
//! Antes, qualquer documento com `sheet` ficava de fora porque as abas saíam com costura nas dobras
//! que o `BRepCheck` reprovava mesmo com volume e planificação certos. Aqui cada comando de chapa é
//! montado em código, calculado dobrado e planificado, e o corpo final tem de ser válido para o
//! kernel e o comando não pode ganhar o aviso de sólido inválido.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{final_shape, recompute, validade, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn base() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

#[allow(clippy::too_many_arguments)]
fn aba(e: [f64; 3], len: &str, off: &str, relief: Option<&str>, kind: u8, side: u8, miter: bool) -> FeatureKind {
    FeatureKind::SmFlange(SmFlange { edge: e, length: s(len), angle: s("90"), radius: None, off1: s(off), off2: s(off), flip: false, relief: relief.map(s), relief_kind: kind, side, miter, min_gap: true, edge_key: None })
}

/// Web de 100 × 50 e um suporte curvo que corta a web perto de x = 80: a aba sobe seguindo o contorno
/// (varredura por um caminho em spline ou em polilinha, que saía com arestas gêmeas, B432).
fn nervura(spline: bool) -> Document {
    let mut d = base();
    let pts: Vec<PointRef> = (0..=12)
        .map(|i| {
            let y = -10. + 70. * i as f64 / 12.;
            PointRef::Fixed([75. + 5. * (y / 60. * std::f64::consts::PI).sin(), y, 0.])
        })
        .collect();
    let curva = d.add_geometry(if spline { RefKind::Spline { points: pts, closed: false, tangents: vec![] } } else { RefKind::Polyline { points: pts, closed: false } });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(curva), dir: AxisRef::Origin(OriginAxis::Z), length: s("60"), length2: s("0"), until: None, until2: None });
    d.add_feature(FeatureKind::SmSurfFlange(SmSurfFlange { support: SurfRef::Ref(sup), face: [40., 25., 2.], length: s("15"), radius: None, flip: false, keep_base: false, mode: 0, angle: s("90") }));
    d
}

/// Confere o documento nos dois estados: nenhum comando com aviso ou erro e o sólido válido.
fn confere_valido(nome: &str, d: &Document, falhas: &mut Vec<String>) {
    confere(nome, d, falhas, false)
}

/// `avisos_de_estado`: aceita os avisos de "não existe neste estado" (joggle desfeito na planificada).
fn confere(nome: &str, d: &Document, falhas: &mut Vec<String>, avisos_de_estado: bool) {
    for plana in [false, true] {
        let mut d = d.clone();
        d.sheet.as_mut().unwrap().flat = plana;
        let r = recompute(&d);
        let estado = if plana { "planificada" } else { "dobrada" };
        for f in &d.features {
            let so_aviso_de_estado = avisos_de_estado && plana && matches!(&r.status[&f.id], Status::Warning(w) if w.starts_with("planificada:"));
            if r.status[&f.id] != Status::Ok && !so_aviso_de_estado {
                falhas.push(format!("{nome} ({estado}): {}: {:?}", f.name, r.status[&f.id]));
            }
        }
        match final_shape(&d) {
            Some(sh) if !sh.is_valid() => falhas.push(format!("{nome} ({estado}): sólido inválido: {}", validade::diagnostico(&sh))),
            None => falhas.push(format!("{nome} ({estado}): sem forma")),
            _ => {}
        }
    }
}

#[test]
fn pecas_de_chapa_passam_na_validade_do_kernel() {
    if !kernel_disponivel() {
        return;
    }
    let mut falhas = vec![];
    let mut d = base();
    confere_valido("parede", &d, &mut falhas);
    d.add_feature(aba([100., 25., 2.], "20", "0", None, 0, 0, false));
    confere_valido("aba simples", &d, &mut falhas);
    let mut d = base();
    d.add_feature(aba([100., 25., 2.], "20", "5", None, 0, 0, false));
    confere_valido("aba com recuos", &d, &mut falhas);
    let mut d = base();
    d.add_feature(aba([100., 25., 2.], "20", "5", None, 0, 2, false));
    confere_valido("aba com material por dentro", &d, &mut falhas);
    let mut d = base();
    d.add_feature(aba([100., 25., 2.], "20", "5", None, 0, 0, true));
    confere_valido("aba em meia-esquadria", &d, &mut falhas);
    for (nome, kind) in [("quadrado", 0), ("redondo", 1), ("rasgo", 2)] {
        let mut d = base();
        d.add_feature(aba([100., 25., 2.], "20", "5", Some("3"), kind, 0, false));
        confere_valido(&format!("aba com alívio {nome}"), &d, &mut falhas);
    }
    let mut d = base();
    d.add_feature(aba([100., 25., 2.], "20", "5", Some("3"), 1, 2, true));
    confere_valido("aba com tudo (do teste de recortes)", &d, &mut falhas);
    let mut d = base();
    d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: s("6"), gap: s("2"), flip: false, kind: 1, edge_key: None }));
    confere_valido("bainha em gota", &d, &mut falhas);
    let mut d = base();
    d.add_feature(FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("5"), flip: true, angle: Some(s("45")), plane: None, keep_length: true }));
    confere("joggle", &d, &mut falhas, true);
    let mut d = base();
    d.add_feature(FeatureKind::SmCornerRelief(SmCornerRelief { at: [100., 50., 2.], size: s("6"), kind: 0 }));
    confere_valido("alívio de canto", &d, &mut falhas);
    let mut d = base();
    d.add_feature(aba([100., 25., 2.], "0", "0", None, 0, 0, false));
    d.add_feature(aba([102., 25., 4.], "10", "0", None, 0, 0, false));
    confere_valido("aba sobre aba", &d, &mut falhas);
    for spline in [true, false] {
        confere_valido(&format!("aba por superfície curva (spline={spline})"), &nervura(spline), &mut falhas);
    }
    assert!(falhas.is_empty(), "peças de chapa reprovadas pela validade do kernel:\n{}", falhas.join("\n"));
}
