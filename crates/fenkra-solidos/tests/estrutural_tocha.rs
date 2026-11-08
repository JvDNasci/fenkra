// SPDX-License-Identifier: LGPL-2.1-or-later
//! B174: acessibilidade da tocha. Tubo 40x40 em pé com placa no alto e cordão no encontro; um tubo
//! retangular largo a 30 mm da face +X fecha o acesso daquele lado. Os pontos dessa face ficam sem direção livre,
//! com o tubo vizinho como bloqueio, e os das outras três faces têm direção livre.

use fenkra_core::*;
use fenkra_solidos::st_tocha::{acessibilidade, sugestoes, Obstaculos, OpcoesTocha};
use fenkra_solidos::{recompute, weld_beads, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn tubo(d: &mut Document, x: f64, z1: f64, kind: SectionKind, dims: &[&str]) -> Id {
    let l = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([x, 0., 0.]), b: PointRef::Fixed([x, 0., z1]), start: s("0"), end: s("0") }));
    d.add_feature(FeatureKind::StructMember(StructMember {
        paths: vec![l],
        family: s("Tubo"),
        designation: dims.join("x"),
        kind,
        dims: dims.iter().map(|x| s(x)).collect(),
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::Nenhum,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }))
}

#[test]
fn vizinho_fecha_o_acesso_de_um_lado_do_cordao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("tocha");
    tubo(&mut d, 0., 500., SectionKind::TuboQuadrado, &["40", "3"]);
    d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 500.], "8", "5")));
    // tubo retangular de 200 de largura (em Y) e 40 de altura (em X): de x = 50 a 90, y de -100 a 100
    let vizinho = tubo(&mut d, 70., 490., SectionKind::TuboRetangular, &["200", "40", "3"]);
    if let Some(f) = d.feature_mut(vizinho) {
        f.name = "Tubo vizinho".into();
    }
    let mut w = StructWeld::novo("5");
    for p in [[20., 0., 250.], [-20., 0., 250.], [0., 20., 250.], [0., -20., 250.]] {
        w.adicionar(p, None);
    }
    w.picking_b = true;
    w.adicionar([22., 0., 500.], None);
    let wid = d.add_feature(FeatureKind::StructWeld(w));
    let r = recompute(&d);
    assert_eq!(r.status[&wid], Status::Ok, "{:?}", r.status[&wid]);
    let cordoes = weld_beads(&d);
    assert_eq!(cordoes.len(), 1);
    assert!(cordoes[0].caminhos.iter().all(|c| c.pernas.len() + 1 == c.pts.len()));

    let obs = Obstaculos::da_peca(&r);
    let pts = acessibilidade(&cordoes[0], &obs, &OpcoesTocha::default());
    // 160 mm de contorno a cada 20 mm
    assert_eq!(pts.len(), 8, "{}", pts.len());
    for p in &pts {
        let lado_fechado = p.p[0] > 19.;
        assert_eq!(p.viavel(), !lado_fechado, "ponto {:?}: livres {:.2}", p.p, p.fracao_livre());
        if lado_fechado {
            assert!(p.bloqueios.contains(&vizinho), "{:?}", p.bloqueios);
        }
        // a referência aponta para fora do tubo e para baixo da placa
        assert!(p.referencia[2] < -0.5, "{:?}", p.referencia);
    }
    let txt = sugestoes(&d, &pts);
    assert!(txt[0].contains("2 de 8") && txt[0].contains("Tubo vizinho"), "{txt:?}");

    // sem o vizinho, todo ponto tem direção livre
    d.features.retain(|f| f.id != vizinho);
    let r = recompute(&d);
    let pts = acessibilidade(&weld_beads(&d)[0], &Obstaculos::da_peca(&r), &OpcoesTocha::default());
    assert!(pts.iter().all(|p| p.viavel()));
    assert!(sugestoes(&d, &pts).iter().all(|t| !t.contains("sem direção livre")));
}
