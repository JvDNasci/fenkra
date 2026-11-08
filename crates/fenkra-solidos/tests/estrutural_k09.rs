// SPDX-License-Identifier: LGPL-2.1-or-later
//! B387 (K-09 do kart): membros retos com canto Encaixe chegando numa longarina dobrada. Toda ponta
//! tem de sair recortada no tubo e na lista de corte, tanto no meio de um trecho (junta em T) quanto
//! num nó de dobra. Documento montado em código com a geometria do quadro do kart.

use fenkra_core::*;
use fenkra_solidos::st_encaixe::Opcoes;
use fenkra_solidos::st_encaixe_doc::encaixes_da_estrutura;
use fenkra_solidos::{cut_list, recompute, structural_members, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn tubo(paths: Vec<CurveRef>, corner: CornerMode) -> StructMember {
    StructMember {
        paths,
        family: s("Tubo redondo"),
        designation: s("Ø30x2,4"),
        kind: SectionKind::TuboRedondo,
        dims: vec![s("30"), s("2.4")],
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

const Z: f64 = 40.;

/// Longarina do kart: 4 trechos ligados por dobradeira de raio 90 no centro.
fn longarina(d: &mut Document, y: f64, yd: f64) -> Id {
    let p = [[-100., y, Z + 35.], [120., y, Z], [624., y, Z], [870., yd, Z], [1100., yd, Z]];
    let c: Vec<CurveRef> = p.windows(2).map(|w| linha(d, w[0], w[1])).collect();
    let f = d.add_feature(FeatureKind::StructMember(tubo(c, CornerMode::Nenhum)));
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        for i in 1..4 {
            m.joints.push(StructJoint { mine: i, other: f, other_path: i - 1, mode: CornerMode::Dobra, first: false, gap: None, bend: Some(TubeBend { mode: BendMode::Dobradeira, radius: s("90"), ..Default::default() }) });
        }
    }
    f
}

/// O quadro: duas longarinas, três travessas (uma num nó de dobra, duas no meio de trechos) e dois
/// apoios inclinados. Devolve o documento e os comandos das peças cortadas.
fn quadro() -> (Document, Vec<(Id, &'static str)>) {
    let mut d = Document::new("quadro");
    longarina(&mut d, 350., 280.);
    longarina(&mut d, -350., -280.);
    let mut cortadas = vec![];
    for (nome, a, b) in [
        // nó de dobra (LG1 em x = 120)
        ("TR_TRAS", [120., -350., Z], [120., 350., Z]),
        // meio do trecho LG1-LG2
        ("TR_MONT", [436.8, -350., Z], [436.8, 350., Z]),
        // meio do trecho LG3-LG4
        ("TR_DIANT", [1000., -280., Z], [1000., 280., Z]),
        // apoio do banco: sai da longarina e sobe para dentro, ponta de cima solta
        ("BANCO_AE", [300., 350., Z], [300., 190., Z + 170.]),
        ("BANCO_AD", [300., -350., Z], [300., -190., Z + 170.]),
    ] {
        let l = linha(&mut d, a, b);
        let f = d.add_feature(FeatureKind::StructMember(tubo(vec![l], CornerMode::Encaixe)));
        if let Some(x) = d.feature_mut(f) {
            x.name = nome.into();
        }
        cortadas.push((f, nome));
    }
    (d, cortadas)
}

#[test]
fn toda_ponta_que_chega_na_longarina_sai_recortada() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (d, cortadas) = quadro();
    let r = recompute(&d);
    for (id, st) in &r.status {
        assert!(matches!(st, Status::Ok), "{:?}: {st:?}", d.feature(*id).map(|f| &f.name));
    }
    let sol = structural_members(&d);
    let rows = cut_list(&d);
    for (f, nome) in &cortadas {
        assert!(sol.iter().any(|x| x.feature == *f), "{nome} sem sólido");
        let row = rows.iter().find(|r| r.members.contains(&(*f, 0))).unwrap_or_else(|| panic!("{nome} fora da lista"));
        // pontas que tocam a longarina: as duas nas travessas, só a de baixo nos apoios
        let ambas = nome.starts_with("TR_");
        assert!(row.encaixe1, "{nome}: ponta 1 devia sair como encaixe na lista (a1={:?})", row.angle1);
        assert_eq!(row.encaixe2, ambas, "{nome}: ponta 2 (a2={:?})", row.angle2);
        if !ambas {
            assert_eq!(row.angle2, Some(0.), "{nome}: a ponta solta é corte reto");
        }
    }
    // o relatório de encaixe (curva do laser) cobre as mesmas pontas
    let enc = encaixes_da_estrutura(&d, &Opcoes::default());
    for (f, nome) in &cortadas {
        let e = enc.iter().find(|e| e.feature == *f).unwrap_or_else(|| panic!("{nome} sem curva de encaixe"));
        assert!(e.ponta1.is_some(), "{nome}: sem curva na ponta 1");
        assert_eq!(e.ponta2.is_some(), nome.starts_with("TR_"), "{nome}: ponta 2");
    }
}

/// A longarina não muda porque uma travessa chega no nó de dobra: antes, a sobra da junta apagava o
/// recuo da dobradeira e a perna passava 15 mm do nó, fazendo um bico para fora da curva.
#[test]
fn travessa_no_no_de_dobra_nao_deixa_bico_na_longarina() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let longa = |com_travessa: bool| {
        let mut d = Document::new("k09");
        let f = longarina(&mut d, 350., 280.);
        if com_travessa {
            let l = linha(&mut d, [120., -350., Z], [120., 350., Z]);
            d.add_feature(FeatureKind::StructMember(tubo(vec![l], CornerMode::Encaixe)));
        }
        let sol = structural_members(&d);
        let pecas: Vec<_> = sol.iter().filter(|x| x.feature == f).collect();
        let vol: f64 = pecas.iter().map(|x| x.shape.mass().volume).sum();
        // caixa de cada peça no referencial dela (décimos de mm)
        let caixas: Vec<[i64; 6]> = pecas.iter().map(|x| x.signature.bbox).collect();
        (vol, caixas)
    };
    let (v0, c0) = longa(false);
    let (v1, c1) = longa(true);
    assert!(v0 > 0.);
    assert!((v1 - v0).abs() < 1e-6 * v0, "a longarina mudou com a travessa: {v0} -> {v1}");
    assert_eq!(c0, c1, "caixa das peças mudou");
}

/// Ponta que entra no meio de outro membro sem Aparar nem Encaixe não passa calada: o comando avisa.
#[test]
fn ponta_em_t_sem_tratamento_avisa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("k09");
    longarina(&mut d, 350., 280.);
    let l = linha(&mut d, [436.8, 350., Z], [436.8, 0., Z]);
    let f = d.add_feature(FeatureKind::StructMember(tubo(vec![l], CornerMode::Nenhum)));
    let r = recompute(&d);
    match r.status.get(&f) {
        Some(Status::Warning(w)) => assert!(w.contains("sem corte"), "{w}"),
        o => panic!("devia avisar: {o:?}"),
    }
    // com Encaixe o aviso some
    if let Some(FeatureKind::StructMember(m)) = d.feature_mut(f).map(|f| &mut f.kind) {
        m.corner = CornerMode::Encaixe;
    }
    assert!(matches!(recompute(&d).status.get(&f), Some(Status::Ok)));
}
