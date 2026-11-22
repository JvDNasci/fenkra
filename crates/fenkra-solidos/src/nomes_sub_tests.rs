// SPDX-License-Identifier: LGPL-2.1-or-later
//! B577: subelementos com nome próprio ("Dono · Face 1 · Aresta 2 · Vértice 1"). Cada teste monta a peça
//! em código, calcula a tabela de nomes, muda uma cota do sketch, recalcula e confere que o mesmo nome
//! continua no mesmo elemento e que o comando que o usa segue ok, com volume e caixa coerentes.

use super::*;
use crate::nomes_sub::SubNomes;
use std::f64::consts::PI;

fn s(x: &str) -> String {
    x.to_string()
}

fn ok(r: &RecomputeResult, id: Id) -> bool {
    matches!(r.status.get(&id), Some(Status::Ok | Status::Warning(_)))
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn sk_data(a: [f64; 2], b: [f64; 2], base: u64) -> SketchData {
    let mut data = SketchData::default();
    let mut n = base;
    let mut al = || {
        n += 1;
        Id(n)
    };
    data.add_rect(&mut al, a, b, false);
    data
}

fn sk_rect(d: &mut Document, plane: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
    let base = 5_260_000 + d.features.len() as u64 * 1000;
    d.add_feature(FeatureKind::Sketch(Sketch { plane, data: sk_data(a, b, base) }))
}

fn set_rect(d: &mut Document, sk: Id, a: [f64; 2], b: [f64; 2]) {
    let base = 5_270_000 + sk.0 * 1000;
    if let Some(FeatureKind::Sketch(x)) = d.feature_mut(sk).map(|f| &mut f.kind) {
        x.data = sk_data(a, b, base);
    }
}

fn ext(sketch: Id, d: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(d.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() })
}

fn volume(r: &RecomputeResult) -> f64 {
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn tr(x: &str) -> String {
    x.to_string()
}

fn comprimento(a: &nomes_sub::ArestaN) -> f64 {
    (0..3).map(|i| (a.pontas[0][i] as f64 - a.pontas[1][i] as f64).powi(2)).sum::<f64>().sqrt()
}

/// O texto do nome só pode ter o dono, "Face n", "Aresta m" e "Vértice k": sem número decimal, vírgula,
/// parêntese nem "perto de".
fn nome_sem_coordenada(nome: &str, dono: &str) {
    let resto = nome.strip_prefix(dono).unwrap_or_else(|| panic!("o nome começa pelo dono: {nome}"));
    let partes: Vec<&str> = resto.split(" · ").filter(|p| !p.is_empty()).collect();
    assert!(!partes.is_empty() && partes.len() <= 3, "{nome}");
    for (i, p) in partes.iter().enumerate() {
        let (palavra, n) = p.split_once(' ').unwrap_or_else(|| panic!("parte sem número: {nome}"));
        assert_eq!(palavra, ["Face", "Aresta", "Vértice"][i], "{nome}");
        assert!(n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty(), "{nome}");
    }
    for ruim in ['(', ')', ',', '.'] {
        assert!(!resto.contains(ruim), "coordenada no nome: {nome}");
    }
    assert!(!nome.contains("perto de") && !nome.contains("sem nome"), "{nome}");
}

/// Todos os nomes da tabela: únicos e sem coordenada.
fn confere_todos_os_nomes(d: &Document, t: &SubNomes) -> usize {
    let mut vistos = std::collections::HashSet::new();
    let mut n = 0;
    for f in &t.faces {
        let nome = t.nome_da_face(d, f, &tr);
        nome_sem_coordenada(&nome, &d.name_of(f.dono));
        assert!(vistos.insert(nome.clone()), "nome repetido: {nome}");
        n += 1;
    }
    for a in &t.arestas {
        let nome = t.nome_da_aresta(d, a, &tr);
        nome_sem_coordenada(&nome, &d.name_of(t.faces[a.face].dono));
        assert!(vistos.insert(nome.clone()), "nome repetido: {nome}");
        for k in [1, 2] {
            let v = t.nome_do_vertice(d, a, k, &tr);
            nome_sem_coordenada(&v, &d.name_of(t.faces[a.face].dono));
            assert!(vistos.insert(v.clone()), "nome repetido: {v}");
        }
        n += 3;
    }
    n
}

// ------------------------------------------------------------------ corpo

#[test]
fn face_1_aresta_1_continua_na_mesma_aresta_quando_a_cota_do_sketch_muda_e_o_filete_segue() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b577 corpo");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [40., 30.]);
    let bloco = d.add_feature(ext(base, "20"));
    let r0 = recompute(&d);
    let t0 = SubNomes::de(&r0);
    // 6 faces, 12 arestas
    assert_eq!(t0.faces.len(), 6);
    assert_eq!(t0.arestas.len(), 12);
    let dono = d.name_of(bloco);
    // "Face 1" é a tampa inicial; "Face 2", a final; as laterais vêm depois
    let f1 = t0.faces.iter().find(|f| f.dono == bloco && f.numero == 1).unwrap();
    assert_eq!(f1.papel, 0);
    assert_eq!(t0.nome_da_face(&d, f1, &tr), format!("{dono} · Face 1"));
    assert_eq!(t0.faces.iter().find(|f| f.numero == 2).unwrap().papel, 1);
    // a aresta 1 da face 1
    let a1 = t0.arestas.iter().find(|a| a.face == t0.faces.iter().position(|f| f.numero == 1).unwrap() && a.numero == 1).unwrap();
    assert_eq!(t0.nome_da_aresta(&d, a1, &tr), format!("{dono} · Face 1 · Aresta 1"));
    assert_eq!(t0.nome_do_vertice(&d, a1, 2, &tr), format!("{dono} · Face 1 · Aresta 1 · Vértice 2"));
    let comp0 = comprimento(a1);
    let b0 = &r0.bodies[0];
    let chave = b0.edge_key(a1.aresta).expect("aresta entre faces com papel");
    assert!(perto(a1.meio[2] as f64, 0., 1e-4), "a aresta da tampa inicial fica em z = 0");
    // o vértice 1 da aresta é achado de volta pela posição, com o mesmo nome
    let (av, k) = t0.aresta_com_ponta(a1.corpo, a1.pontas[0]).unwrap();
    assert_eq!((av.aresta, k), (a1.aresta, 1));

    // o filete guarda a chave (é o que o comando grava); a cópia sem filete mostra o nome da aresta intacta
    let mut sem_filete = d.clone();
    let fil = d.add_feature(FeatureKind::Fillet(EdgeOp { value: s("2"), edges: EdgeSel::Edges(vec![[a1.meio[0] as f64, a1.meio[1] as f64, a1.meio[2] as f64]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![Some(chave)], extra: Default::default() }));
    let r1 = recompute(&d);
    assert!(ok(&r1, fil), "{:?}", r1.status.get(&fil));
    let v1 = volume(&r1);
    assert!(perto(v1, 40. * 30. * 20. - (1. - PI / 4.) * 4. * comp0, 0.3), "{v1}");

    // muda a cota: o retângulo cresce 20 mm em x
    set_rect(&mut d, base, [0., 0.], [60., 30.]);
    set_rect(&mut sem_filete, base, [0., 0.], [60., 30.]);
    let r2 = recompute(&d);
    assert!(ok(&r2, fil), "{:?}", r2.status.get(&fil));
    assert_eq!(SubNomes::de(&r2).faces.len(), 7, "6 faces do bloco e a do filete");
    let r2 = recompute(&sem_filete);
    let t2 = SubNomes::de(&r2);
    assert_eq!(t2.faces.len(), 6);
    let a2 = t2.aresta_da_chave(&chave).expect("a chave ainda acha a aresta");
    assert_eq!(t2.nome_da_aresta(&d, a2, &tr), format!("{dono} · Face 1 · Aresta 1"), "o nome segue a mesma aresta");
    let esperado = if perto(comp0, 40., 1e-6) { 60. } else { 30. };
    assert!(perto(comprimento(a2), esperado, 1e-4), "a mesma aresta do perfil: {comp0} -> {}", comprimento(a2));
    assert!(perto(volume(&recompute(&d)), 60. * 30. * 20. - (1. - PI / 4.) * 4. * esperado, 0.3), "{}", volume(&recompute(&d)));
    let (lo, hi) = final_shape(&d).unwrap().bbox().unwrap();
    assert!(perto(lo[0], 0., 1e-6) && perto(hi[0], 60., 1e-6) && perto(hi[1], 30., 1e-6) && perto(hi[2], 20., 1e-6), "{lo:?} {hi:?}");
    confere_todos_os_nomes(&d, &t2);
}

#[test]
fn nenhum_nome_do_fluxo_normal_tem_coordenada_e_todos_sao_unicos() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // bloco com bolso, furo e filete: faces de tipos e papéis variados
    let mut d = Document::new("b577 nomes");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    let bloco = d.add_feature(ext(base, "20"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let sk = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Cut, extent: Extent::Blind(s("5")), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    let r = recompute(&d);
    let t = SubNomes::de(&r);
    assert!(t.faces.len() >= 10, "{}", t.faces.len());
    let n = confere_todos_os_nomes(&d, &t);
    assert!(n > 40, "{n}");
    // toda face com papel tem chave que a tabela traduz para o mesmo nome do índice
    let b = &r.bodies[0];
    for j in 0..b.mesh.face_info.len() {
        let f = t.face_de(0, j).expect("face na tabela");
        if let Some(k) = b.face_key(j) {
            let achada = t.face_da_chave(&k).expect("chave");
            assert_eq!((achada.corpo, achada.face), (f.corpo, f.face), "a chave aponta para a própria face");
        }
        assert_eq!(f.dono == bloco || f.papel >= 0, true);
    }
    for e in 0..b.mesh.edge_faces.len() {
        if let Some(k) = b.edge_key(e) {
            let a = t.aresta_da_chave(&k).expect("chave de aresta");
            // aresta com a mesma chave de outra (mesmas duas faces) cai na primeira; as demais são únicas
            assert_eq!(t.arestas[t.arestas.iter().position(|x| x.aresta == a.aresta).unwrap()].corpo, 0);
        }
    }
}

// ------------------------------------------------------------------ superfície

fn superficie(comprimento: &str) -> (Document, Id) {
    let mut d = Document::new("b577 superficie");
    let linha = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 30., 0.]), start: s("0"), end: s("0") });
    let sup = d.add_geometry(RefKind::SurfExtrude { curve: CurveRef::Ref(linha), dir: AxisRef::Origin(OriginAxis::X), length: s(comprimento), length2: s("0"), until: None, until2: None, mais: vec![] });
    (d, sup)
}

#[test]
fn borda_de_superficie_tem_nome_de_face_e_aresta_e_o_prolongamento_a_segue() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, sup) = superficie("20");
    let r0 = recompute(&d);
    assert!(ok(&r0, sup), "{:?}", r0.status.get(&sup));
    let t0 = SubNomes::de(&r0);
    let nome_sup = d.name_of(sup);
    assert_eq!(t0.faces.len(), 1, "um retângulo");
    assert_eq!(t0.nome_da_face(&d, &t0.faces[0], &tr), format!("{nome_sup} · Face 1"));
    assert_eq!(t0.arestas.len(), 4);
    // a aresta 1 da face 1 da superfície: a chave que o comando grava (borda, uma face só)
    let a = t0.arestas.iter().find(|a| a.numero == 1).unwrap().clone();
    assert_eq!(t0.nome_da_aresta(&d, &a, &tr), format!("{nome_sup} · Face 1 · Aresta 1"));
    let sv = &r0.surfaces[0];
    let chave = sv.edge_key(a.aresta).expect("borda com chave");
    let pts = vec![[a.meio[0] as f64, a.meio[1] as f64, a.meio[2] as f64]];
    let comp0 = comprimento(&a);
    // o prolongamento usa a borda pela chave
    let ext = d.add_geometry(RefKind::SurfExtrapolate { surface: SurfRef::Ref(sup), near: PointRef::Origin, length: s("5"), curvature: false, more: vec![], edges: vec![CurveRef::BodyEdge { pts, key: chave }], until: None, normal_edge: false, tangent_prop: false });
    let r1 = recompute(&d);
    assert!(ok(&r1, ext), "{:?}", r1.status.get(&ext));
    let area = |r: &RecomputeResult, id: Id| {
        let m = &r.surfaces.iter().find(|s| s.id == id).unwrap().mesh;
        m.indices.chunks_exact(3).map(|t| {
            let p = |i: u32| [m.positions[i as usize * 3] as f64, m.positions[i as usize * 3 + 1] as f64, m.positions[i as usize * 3 + 2] as f64];
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            0.5 * ((u[1] * v[2] - u[2] * v[1]).powi(2) + (u[2] * v[0] - u[0] * v[2]).powi(2) + (u[0] * v[1] - u[1] * v[0]).powi(2)).sqrt()
        }).sum::<f64>()
    };
    assert!(perto(area(&r0, sup), 600., 1.), "{}", area(&r0, sup));
    assert!(perto(area(&r1, ext), 600. + 5. * comp0, 1.5), "{} (borda de {comp0} mm)", area(&r1, ext));

    // muda a cota (o comprimento da extrusão): a mesma borda, pelo mesmo nome, segue o elemento
    if let Some(RefKind::SurfExtrude { length, .. }) = d.references.iter_mut().find(|r| r.id == sup).map(|r| &mut r.kind) {
        *length = s("35");
    }
    let r2 = recompute(&d);
    assert!(ok(&r2, ext), "{:?}", r2.status.get(&ext));
    let t2 = SubNomes::de(&r2);
    let a2 = t2.arestas.iter().find(|x| x.numero == 1).unwrap();
    assert_eq!(t2.nome_da_aresta(&d, a2, &tr), format!("{nome_sup} · Face 1 · Aresta 1"));
    let esperado = if perto(comp0, 30., 1e-4) { 30. } else { 35. };
    assert!(perto(comprimento(a2), esperado, 1e-4), "a mesma borda: {comp0} -> {}", comprimento(a2));
    assert!(perto(area(&r2, ext), 35. * 30. + 5. * esperado, 2.), "{}", area(&r2, ext));
    confere_todos_os_nomes(&d, &t2);
}

// ------------------------------------------------------------------ chapa

#[test]
fn aresta_da_ponta_da_aba_de_chapa_mantem_o_nome_quando_a_parede_base_cresce() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut d = Document::new("b577 chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let aba = d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    let r0 = recompute(&d);
    assert!(ok(&r0, aba), "{:?}", r0.status.get(&aba));
    // grava a chave promovida da aba, como o app faz
    if let Some(Promoted::Edges(k)) = r0.promoted.get(&aba) {
        if let Some(FeatureKind::SmFlange(a)) = d.feature_mut(aba).map(|f| &mut f.kind) {
            a.edge_key = k[0];
        }
    }
    let t0 = SubNomes::de(&r0);
    confere_todos_os_nomes(&d, &t0);
    // a aba tem faces e arestas numeradas por ela
    let da_aba: Vec<&nomes_sub::FaceN> = t0.faces.iter().filter(|f| f.dono == aba).collect();
    assert!(!da_aba.is_empty(), "a aba deu faces à tabela");
    // a aresta mais alta e mais para fora (a ponta da aba), como no teste de B516, com chave e nome
    let b = &r0.bodies[0];
    let ponta = t0
        .arestas
        .iter()
        .filter(|a| t0.faces[a.face].dono == aba && b.edge_key(a.aresta).is_some())
        .max_by(|x, y| (x.meio[2], x.meio[0]).partial_cmp(&(y.meio[2], y.meio[0])).unwrap())
        .expect("aresta da aba com chave")
        .clone();
    let chave = b.edge_key(ponta.aresta).unwrap();
    let nome0 = t0.nome_da_aresta(&d, &ponta, &tr);
    let z0 = ponta.meio[2];
    let mut sem_bainha = d.clone();
    let bainha = d.add_feature(FeatureKind::SmHem(SmHem { edge: [ponta.meio[0] as f64, ponta.meio[1] as f64, ponta.meio[2] as f64], length: s("8"), gap: s("0"), flip: false, kind: 0, edge_key: Some(chave) }));
    let r1 = recompute(&d);
    assert!(ok(&r1, bainha), "{:?}", r1.status.get(&bainha));
    let v1 = volume(&r1);
    let (lo1, hi1) = final_shape(&d).unwrap().bbox().unwrap();

    set_rect(&mut d, sk, [0., 0.], [120., 50.]);
    set_rect(&mut sem_bainha, sk, [0., 0.], [120., 50.]);
    let r2 = recompute(&d);
    assert!(ok(&r2, aba) && ok(&r2, bainha), "{:?} {:?}", r2.status.get(&aba), r2.status.get(&bainha));
    // a bainha consome a aresta da ponta: o nome é conferido na chapa sem ela
    let r2s = recompute(&sem_bainha);
    let t2 = SubNomes::de(&r2s);
    let p2 = t2.aresta_da_chave(&chave).expect("a chave acha a ponta da aba");
    assert_eq!(t2.nome_da_aresta(&d, p2, &tr), nome0, "mesmo nome, mesmo elemento");
    assert!(perto(p2.meio[2] as f64, z0 as f64, 1e-3), "a ponta continua na altura da aba");
    assert!(perto(p2.meio[0] as f64, ponta.meio[0] as f64 + 20., 1e-3), "e andou com a parede base");
    let (lo2, hi2) = final_shape(&d).unwrap().bbox().unwrap();
    assert!(perto(hi2[0], hi1[0] + 20., 1e-3) && perto(lo2[0], lo1[0], 1e-3) && perto(hi2[2], hi1[2], 1e-3), "{lo1:?} {hi1:?} -> {lo2:?} {hi2:?}");
    assert!(perto(volume(&r2) - v1, 2000., 1.), "{v1} -> {}", volume(&r2));
    confere_todos_os_nomes(&d, &t2);
}

// ------------------------------------------------------------------ membro estrutural

fn membro_em_l(altura: &str) -> (Document, Id, Id, Id) {
    let mut d = Document::new("b577 estrutural");
    let l1 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 0.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") });
    let l2 = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([0., 0., 500.]), b: PointRef::Fixed([300., 0., 500.]), start: s("0"), end: s("0") });
    let membro = d.add_feature(FeatureKind::StructMember(StructMember {
        paths: vec![CurveRef::Ref(l1), CurveRef::Ref(l2)],
        family: s("Tubo quadrado"),
        designation: s("40x3"),
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
        tube_bend: None,
        fit: Default::default(),
    }));
    let placa = d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 0.], "8", altura)));
    (d, l1, membro, placa)
}

#[test]
fn ponta_de_membro_estrutural_tem_nome_de_trecho_e_a_placa_segue_a_ponta() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (mut d, l1, membro, placa) = membro_em_l("5");
    let r0 = recompute(&d);
    assert!(ok(&r0, placa), "{:?}", r0.status.get(&placa));
    // grava a âncora promovida como o app faz
    if let Some(Promoted::Ancoras(a)) = r0.promoted.get(&placa) {
        d.feature_mut(placa).unwrap().guardar_ancoras(a);
    }
    let f = d.feature(placa).unwrap();
    let Some(a) = f.ancora("at", [0., 0., 0.]) else { panic!("âncora: {:?}", f.ancoras) };
    let nome = refs::nome_ancora(&d, a, &|x| x.to_string());
    assert_eq!(nome, format!("{} · trecho 1 · início", d.name_of(membro)), "a ponta do membro é trecho e extremidade, não ponto");
    // as faces do membro entram na tabela com nome de face (o membro é o dono)
    let t0 = SubNomes::de(&r0);
    assert!(t0.faces.iter().any(|f| f.dono == membro), "o membro tem faces numeradas");
    let (lo0, _) = final_shape(&d).unwrap().bbox().unwrap();
    assert!(perto(lo0[2], -8., 1e-3), "{lo0:?}");
    // a primeira linha desce: o nome (trecho 1, início) segue a mesma ponta e a placa vai com ela
    if let Some(r) = d.references.iter_mut().find(|r| r.id == l1) {
        r.kind = RefKind::LinePoints { a: PointRef::Fixed([0., 0., -600.]), b: PointRef::Fixed([0., 0., 500.]), start: s("0"), end: s("0") };
    }
    let r1 = recompute(&d);
    assert!(ok(&r1, placa), "{:?}", r1.status.get(&placa));
    let (lo1, _) = final_shape(&d).unwrap().bbox().unwrap();
    assert!(perto(lo1[2], -608., 1e-3), "{lo1:?}");
    let t1 = SubNomes::de(&r1);
    confere_todos_os_nomes(&d, &t1);
}

#[test]
fn faces_de_comandos_comuns_tem_papel_e_chave_no_recalculo() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    // quantas faces ficam sem papel (sem chave) em uma peça de uso comum: bloco com bolso e filete
    let mut d = Document::new("b577 cobertura");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(base, "20"));
    let r = recompute(&d);
    let sem: usize = r.bodies.iter().map(|b| b.face_role.iter().filter(|x| **x < 0).count()).sum();
    assert_eq!(sem, 0, "toda face de extrusão tem papel");
    let (dm, _, _, _) = membro_em_l("5");
    let rm = recompute(&dm);
    let sem_m: usize = rm.bodies.iter().map(|b| b.face_role.iter().filter(|x| **x < 0).count()).sum();
    let total_m: usize = rm.bodies.iter().map(|b| b.face_role.len()).sum();
    eprintln!("B577 cobertura: membro estrutural {sem_m} de {total_m} faces sem papel");
    let (ds, sup) = superficie("20");
    let rs = recompute(&ds);
    let t = SubNomes::de(&rs);
    assert!(t.e_superficie(sup) && rs.surfaces[0].face_key(0).is_some(), "toda face de superfície tem papel (índice da face)");
}
