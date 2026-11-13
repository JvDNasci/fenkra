// SPDX-License-Identifier: LGPL-2.1-or-later
//! B356: prévia leve nos modos especiais do filete, na chapa e no estrutural. Com o comando aberto
//! (`Cache::leve`) o corpo passa intacto e a prévia diz onde e quanto; ao concluir, o cálculo real
//! roda e o resultado cabe na prévia.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute_cached, Cache, RecomputeResult, Status};

fn s(x: &str) -> String {
    x.into()
}

fn leve_e_final(d: &Document, id: Id) -> (RecomputeResult, RecomputeResult) {
    let mut cache = Cache::default();
    cache.leve = Some(id);
    let leve = recompute_cached(d, &mut cache);
    cache.leve = None;
    let final_ = recompute_cached(d, &mut cache);
    (leve, final_)
}

fn caixa_dos_cantos(c: &[[f64; 3]; 8]) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in c {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

/// A caixa `a` contém a caixa `b` (com folga de `tol`).
fn contem(a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3]), tol: f64) -> bool {
    (0..3).all(|i| a.0[i] <= b.0[i] + tol && a.1[i] >= b.1[i] - tol)
}

fn ok(r: &RecomputeResult, id: Id) -> bool {
    matches!(r.status.get(&id), Some(Status::Ok))
}

fn bloco() -> Document {
    let mut d = Document::new("bloco");
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-30., -20.], [30., 20.]);
    d.add_feature(ext(sk, BoolOp::Add, "20"));
    d
}

#[test]
fn filete_variavel_leve_usa_o_maior_raio_e_nao_mexe_no_corpo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = bloco();
    let sem = volume_total(&fenkra_solidos::recompute(&d));
    let mut k = filete("2", vec![[0., -20., 20.]], vec![]);
    if let FeatureKind::Fillet(o) = &mut k {
        o.mode = EdgeMode::Variable(s("5"));
    }
    let id = d.add_feature(k);
    let (leve, final_) = leve_e_final(&d, id);
    assert!(ok(&leve, id), "{:?}", leve.status.get(&id));
    assert!((volume_total(&leve) - sem).abs() < 1e-6, "a prévia não mexe no corpo");
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia do filete variável");
    assert!((v.valor - 5.).abs() < 1e-9 && v.arestas.len() == 1, "{v:?}");
    // ao concluir: o filete real tira material, entre o de raio 2 e o de raio 5 constantes
    assert!(ok(&final_, id), "{:?}", final_.status.get(&id));
    let tirado = sem - volume_total(&final_);
    let canto = |r: f64| (1. - std::f64::consts::FRAC_PI_4) * r * r * 60.;
    assert!(tirado > canto(2.) && tirado < canto(5.), "tirado {tirado}");
    assert!(final_.previas.is_empty());
}

#[test]
fn chanfro_distancia_e_angulo_leve_usa_a_maior_distancia() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = bloco();
    let mut k = chanfro("2", vec![[0., -20., 20.]], vec![]);
    if let FeatureKind::Chamfer(o) = &mut k {
        o.mode = EdgeMode::DistAngle(s("60"));
    }
    let id = d.add_feature(k);
    let (leve, _) = leve_e_final(&d, id);
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    assert!(v.chanfro && (v.valor - 2. * 60f64.to_radians().tan()).abs() < 1e-9, "{v:?}");
}

#[test]
fn filete_face_a_face_leve_guarda_as_faces_clicadas() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = bloco();
    let sem = volume_total(&fenkra_solidos::recompute(&d));
    let (topo, frente) = ([0., 0., 20.], [0., -20., 10.]);
    let mut k = filete("3", vec![], vec![]);
    if let FeatureKind::Fillet(o) = &mut k {
        o.mode = EdgeMode::FaceFace(topo, frente);
        o.edges = EdgeSel::AllOfBody;
    }
    let id = d.add_feature(k);
    let (leve, final_) = leve_e_final(&d, id);
    assert!(ok(&leve, id), "{:?}", leve.status.get(&id));
    assert!((volume_total(&leve) - sem).abs() < 1e-6);
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia");
    assert_eq!(v.pares, vec![[topo, frente]]);
    assert!(ok(&final_, id), "{:?}", final_.status.get(&id));
    assert!(volume_total(&final_) < sem - 1., "o real arredonda a aresta comum");
}

fn membro(paths: Vec<CurveRef>) -> StructMember {
    StructMember {
        paths,
        family: s("Tubo"),
        designation: s("teste"),
        kind: SectionKind::TuboQuadrado,
        dims: vec![s("40"), s("3")],
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner: CornerMode::MeiaEsquadria,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

#[test]
fn membro_leve_e_a_caixa_da_secao_ao_longo_da_guia() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("quadro");
    let l1 = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let l2 = linha(&mut d, [300., 0., 0.], [300., 200., 0.]);
    let id = d.add_feature(FeatureKind::StructMember(membro(vec![l1, l2])));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(ok(&leve, id), "{:?}", leve.status.get(&id));
    assert!(volume_total(&leve) < 1e-6, "nenhum tubo construído na prévia");
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia do membro");
    assert_eq!(v.caixas.len(), 2, "uma caixa por trecho reto");
    // cada caixa tem o comprimento do trecho e a seção de 40 x 40
    let (lo, hi) = caixa_dos_cantos(&v.caixas[0]);
    let lados = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    assert!((lados[0] - 300.).abs() < 1e-6 && (lados[1] - 40.).abs() < 1e-6 && (lados[2] - 40.).abs() < 1e-6, "{lados:?}");
    // o resultado real cabe nas caixas da prévia
    assert!(ok(&final_, id), "{:?}", final_.status.get(&id));
    let juntas = v.caixas.iter().map(caixa_dos_cantos).fold(([f64::MAX; 3], [f64::MIN; 3]), |a, b| ([0, 1, 2].map(|i| a.0[i].min(b.0[i])), [0, 1, 2].map(|i| a.1[i].max(b.1[i]))));
    assert!(contem(juntas, caixa_da_malha(&final_), 0.5), "{juntas:?} contra {:?}", caixa_da_malha(&final_));
    assert!(volume_total(&final_) > 1000.);
}

#[test]
fn membro_no_meio_da_arvore_calcula_completo() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = Document::new("quadro");
    let l1 = linha(&mut d, [0., 0., 0.], [300., 0., 0.]);
    let id = d.add_feature(FeatureKind::StructMember(membro(vec![l1])));
    let l2 = linha(&mut d, [0., 100., 0.], [300., 100., 0.]);
    d.add_feature(FeatureKind::StructMember(membro(vec![l2])));
    let (leve, _) = leve_e_final(&d, id);
    assert!(leve.previas.iter().all(|p| p.feature != id), "comando com outros depois não fica leve");
    assert!(volume_total(&leve) > 1000.);
}

fn chapa_base() -> Document {
    let mut d = Document::new("chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d
}

#[test]
fn aba_de_chapa_leve_mostra_a_caixa_da_aba_e_guarda_a_linha_de_dobra() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_base();
    let sem = volume_total(&fenkra_solidos::recompute(&d));
    let id = d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [50., 0., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(ok(&leve, id), "{:?}", leve.status.get(&id));
    assert!((volume_total(&leve) - sem).abs() < 1e-6, "a aba não é construída na prévia");
    let v = leve.previas.iter().find(|p| p.feature == id).expect("prévia da aba");
    // as alças de ângulo, raio e recuos leem a linha de dobra do comando: ela continua na prévia
    assert!(leve.bend_lines.iter().any(|b| b.feature == id), "a linha de dobra da aba fica na prévia");
    let zona = caixa_dos_cantos(&v.caixas[0]);
    assert!(zona.1[0] - zona.0[0] > 99.9, "a caixa da aba vai de ponta a ponta da aresta: {zona:?}");
    assert!(ok(&final_, id), "{:?}", final_.status.get(&id));
    assert!(volume_total(&final_) > sem + 1000.);
    // o material novo (tudo o que sai da parede de 2 mm) cabe na zona
    let fora: Vec<[f64; 3]> = final_.bodies.iter().flat_map(|b| b.mesh.positions.chunks(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])).filter(|p| p[2] > 2.01 || p[2] < -0.01).collect();
    assert!(!fora.is_empty());
    let novo = fora.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |a, p| ([0, 1, 2].map(|i| a.0[i].min(p[i])), [0, 1, 2].map(|i| a.1[i].max(p[i]))));
    assert!(contem(zona, novo, 0.5), "{zona:?} contra {novo:?}");
}

#[test]
fn dobra_por_linha_leve_nao_gira_a_chapa_e_guarda_a_linha() {
    if !kernel_disponivel() {
        return;
    }
    let mut d = chapa_base();
    let sem = volume_total(&fenkra_solidos::recompute(&d));
    let caixa0 = caixa_da_malha(&fenkra_solidos::recompute(&d));
    let id = d.add_feature(FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("45"), radius: None, flip: true, down: false, sketch: None, position: 0 }));
    let (leve, final_) = leve_e_final(&d, id);
    assert!(ok(&leve, id), "{:?}", leve.status.get(&id));
    assert!((volume_total(&leve) - sem).abs() < 1e-6);
    assert_eq!(caixa_da_malha(&leve), caixa0, "a chapa não gira na prévia");
    assert!(leve.bend_lines.iter().any(|b| b.feature == id), "a linha da dobra fica para as alças");
    assert!(leve.previas.iter().any(|p| p.feature == id && !p.caixas.is_empty()));
    // ao concluir, a chapa dobra: a caixa sai do plano
    assert!(ok(&final_, id), "{:?}", final_.status.get(&id));
    assert!(caixa_da_malha(&final_).1[2] > caixa0.1[2] + 5.);
    assert!((volume_total(&final_) - sem).abs() < 0.02 * sem, "dobrar conserva o volume");
}

#[test]
fn recorte_de_chapa_continua_completo() {
    let k = FeatureKind::SmCut(SmCut { sketch: Id(1), mode: 0, offset: String::new(), draft: String::new(), height: String::new(), flip: false, normal_face: false });
    assert!(!fenkra_solidos::previa_leve::cobre(&k), "o recorte mostra o contorno nos dois estados, que só o cálculo real dá");
}
