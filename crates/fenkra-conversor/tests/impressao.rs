// SPDX-License-Identifier: LGPL-2.1-or-later
//! B119: documento montado em código até o arquivo de impressão, lido de volta.

use fenkra_conversor::impressao::{abrir_no_fatiador, arquivo_para_fatiador, gravar_3mf, gravar_stl, planejar, OpcoesImpressao};
use fenkra_conversor::malha::Malha;
use fenkra_core::{Appearance, BoolOp, Document, FeatureKind, Finish, Id, MeshImport, OriginPlane, PlaneRef, PrimKind, Primitive};
use fenkra_malha::impressao::Achado;
use fenkra_solidos::corpo_malha::embutir;
use std::collections::HashMap;
use std::path::PathBuf;

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra impressão {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn corpo_malha(d: &mut Document, m: &Malha) -> Id {
    let dados = embutir(d, m);
    d.add_feature(FeatureKind::MeshImport(MeshImport { path: "x.stl".into(), solid: false, corpo: true, dados }))
}

fn caixa_brep(d: &mut Document, l: &str, cor: Option<[u8; 3]>) -> Id {
    let id = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: l.into(), b: l.into(), c: l.into(), d: "0".into(), op: BoolOp::Add }));
    if let Some(color) = cor {
        d.features.iter_mut().find(|f| f.id == id).unwrap().appearance = Some(Appearance { color, finish: Finish::default(), texture: None });
    }
    id
}

fn dois_corpos() -> Document {
    let mut d = Document::new("Peças");
    caixa_brep(&mut d, "20", None);
    let c = d.add_collector("Bloco");
    d.active_collector = c;
    corpo_malha(&mut d, &fenkra_malha::cena::cubo("Bloco", 30.));
    d
}

fn texto_do_modelo(a: &std::path::Path) -> String {
    let b = std::fs::read(a).unwrap();
    String::from_utf8(fenkra_malha::zip::abrir(&b, |n| n == "3D/3dmodel.model").unwrap().unwrap()).unwrap()
}

#[test]
fn varios_corpos_ficam_na_mesa_sem_encostar_e_o_arquivo_volta_com_eles() {
    let d = dois_corpos();
    let o = OpcoesImpressao::default();
    let p = planejar(&d, &o, &HashMap::new()).unwrap();
    assert_eq!(p.corpos.len(), 2);
    assert!(p.pronto(), "{:?}", p.corpos.iter().map(|c| &c.conferencia.achados).collect::<Vec<_>>());
    let dir = pasta("mesa");
    let a = dir.join("plano.3mf");
    gravar_3mf(&p, &o, "Peças", &a).unwrap();
    let t = texto_do_modelo(&a);
    assert!(t.contains(r#"unit="millimeter""#));
    assert_eq!(t.matches("<item ").count(), 2);
    let c = fenkra_conversor::malha::ler_para_teste(fenkra_conversor::Formato::Tmf, &a).unwrap();
    let postas = c.no_lugar();
    assert_eq!(postas.len(), 2);
    let caixas: Vec<_> = postas.iter().map(|m| m.caixa().unwrap()).collect();
    for (lo, hi) in &caixas {
        assert!(lo[2].abs() < 1e-6, "toda peça encosta na mesa: {lo:?}");
        assert!((0..2).all(|i| lo[i] >= -1e-6 && hi[i] <= o.mesa[i] + 1e-6), "{lo:?} {hi:?}");
    }
    let (a0, a1) = (caixas[0], caixas[1]);
    assert!(a0.1[0] + o.espaco <= a1.0[0] + 1e-6 || a1.1[0] + o.espaco <= a0.0[0] + 1e-6 || a0.1[1] + o.espaco <= a1.0[1] + 1e-6 || a1.1[1] + o.espaco <= a0.0[1] + 1e-6, "{a0:?} {a1:?}");
    let v: f64 = postas.iter().map(|m| m.soldada().volume()).sum();
    assert!((v - (8000. + 27000.)).abs() < 1., "{v}");
}

#[test]
fn metadados_miniatura_e_tesselacao_chegam_no_pacote() {
    let d = dois_corpos();
    let o = OpcoesImpressao { titulo: "Suporte & base".into(), autor: "João".into(), descricao: "Peça de teste".into(), ..Default::default() };
    let p = planejar(&d, &o, &HashMap::new()).unwrap();
    let dir = pasta("meta");
    let a = dir.join("m.3mf");
    gravar_3mf(&p, &o, "x", &a).unwrap();
    let t = texto_do_modelo(&a);
    assert!(t.contains(r#"<metadata name="Title">Suporte &amp; base</metadata>"#), "{t}");
    assert!(t.contains(r#"<metadata name="Designer">João</metadata>"#));
    assert!(t.contains(r#"<metadata name="Description">Peça de teste</metadata>"#));
    let b = std::fs::read(&a).unwrap();
    let png = fenkra_malha::zip::abrir(&b, |n| n == "Metadata/thumbnail.png").unwrap().unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let rel = String::from_utf8(fenkra_malha::zip::abrir(&b, |n| n == "_rels/.rels").unwrap().unwrap()).unwrap();
    assert!(rel.contains("metadata/thumbnail"), "{rel}");
    let tipos = String::from_utf8(fenkra_malha::zip::abrir(&b, |n| n == "[Content_Types].xml").unwrap().unwrap()).unwrap();
    assert!(tipos.contains(r#"Extension="png""#), "{tipos}");

    // sem miniatura o pacote não leva a relação nem o arquivo
    let sem = OpcoesImpressao { miniatura: false, ..o };
    gravar_3mf(&p, &sem, "x", &a).unwrap();
    let b = std::fs::read(&a).unwrap();
    assert!(fenkra_malha::zip::abrir(&b, |n| n == "Metadata/thumbnail.png").unwrap().is_none());
}

#[test]
fn tesselacao_configuravel_muda_a_quantidade_de_triangulos() {
    let mut d = Document::new("Cilindro");
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: "20".into(), b: "40".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
    let grossa = planejar(&d, &OpcoesImpressao { desvio: 0.5, angulo: 40., ..Default::default() }, &HashMap::new()).unwrap();
    let fina = planejar(&d, &OpcoesImpressao { desvio: 0.005, angulo: 5., ..Default::default() }, &HashMap::new()).unwrap();
    let t = |p: &fenkra_conversor::impressao::PlanoImpressao| p.corpos[0].unida.tris.len();
    assert!(t(&fina) > 4 * t(&grossa), "{} e {}", t(&fina), t(&grossa));
    assert!(fina.corpos[0].conferencia.pronta());
}

#[test]
fn cor_por_face_vai_por_material_num_objeto_so_e_continua_fechado() {
    let mut d = Document::new("Duas cores");
    caixa_brep(&mut d, "20", Some([200, 30, 30]));
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: "5".into(), b: "40".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
    let n = d.features.len() - 1;
    d.features[n].appearance = Some(Appearance { color: [30, 30, 200], finish: Finish::default(), texture: None });
    let o = OpcoesImpressao::default();
    let p = planejar(&d, &o, &HashMap::new()).unwrap();
    assert_eq!(p.corpos.len(), 1);
    assert!(p.corpos[0].partes.len() >= 2, "uma malha por cor");
    assert!(p.corpos[0].unida.diagnostico().estanque(), "{:?}", p.corpos[0].unida.diagnostico());
    let dir = pasta("cor");
    let a = dir.join("c.3mf");
    gravar_3mf(&p, &o, "x", &a).unwrap();
    let t = texto_do_modelo(&a);
    assert_eq!(t.matches("<object ").count(), 1, "um objeto: partir por cor abriria a malha");
    assert!(t.contains(" p1=\""), "cor no triângulo");
    assert!(t.matches("<base ").count() >= 2);
    let volta = fenkra_conversor::malha::ler_para_teste(fenkra_conversor::Formato::Tmf, &a).unwrap();
    assert!(volta.materiais.len() >= 2);
    let j = fenkra_malha::juntar(volta.no_lugar().iter()).soldada();
    assert!(j.diagnostico().estanque(), "{:?}", j.diagnostico());
}

#[test]
fn assentar_gira_a_peca_no_arquivo_sem_mudar_o_documento() {
    let mut d = Document::new("L");
    let m = {
        let mut c = fenkra_malha::cena::cubo("Barra", 10.);
        c.pos.iter_mut().for_each(|p| p[2] *= 6.);
        c
    };
    corpo_malha(&mut d, &m);
    let coletor = d.collectors[0].id;
    let o = OpcoesImpressao::default();
    let antes = planejar(&d, &o, &HashMap::new()).unwrap();
    let z = |p: &fenkra_conversor::impressao::PlanoImpressao| fenkra_malha::impressao::medida_depois(&p.corpos[0].posicao, &p.corpos[0].unida)[2];
    assert!((z(&antes) - 60.).abs() < 1e-6);
    let deitada = planejar(&d, &o, &HashMap::from([(coletor, [1., 0., 0.])])).unwrap();
    assert!((z(&deitada) - 10.).abs() < 1e-6);
    let posta = deitada.corpos[0].unida.transformada(&deitada.corpos[0].posicao);
    assert!(posta.caixa().unwrap().0[2].abs() < 1e-9);
    assert_eq!(d.features.len(), 1);
}

#[test]
fn verificacao_aponta_o_que_impede_e_o_mapa_de_balanco_acha_a_marquise() {
    let mut aberta = fenkra_malha::cena::cubo("Aberta", 10.);
    aberta.tris.pop();
    let mut d = Document::new("Ruim");
    corpo_malha(&mut d, &aberta);
    let p = planejar(&d, &OpcoesImpressao::default(), &HashMap::new()).unwrap();
    assert!(!p.pronto());
    assert!(p.corpos[0].conferencia.achados.contains(&Achado::NaoEstanque));

    let mut d = Document::new("Grande");
    corpo_malha(&mut d, &fenkra_malha::cena::cubo("Grande", 300.));
    let p = planejar(&d, &OpcoesImpressao::default(), &HashMap::new()).unwrap();
    assert!(!p.corpos[0].coube && !p.pronto());
    assert!(gravar_3mf(&p, &OpcoesImpressao::default(), "x", &pasta("grande").join("g.3mf")).is_err());

    // laje sobre um pilar, por booleana de malha: a sobra da laje fica sem apoio
    let mut pilar = fenkra_malha::cena::cubo("", 10.);
    pilar.pos.iter_mut().for_each(|q| *q = [q[0] + 10., q[1], q[2] * 2.]);
    let mut laje = fenkra_malha::cena::cubo("", 10.);
    laje.pos.iter_mut().for_each(|q| *q = [q[0] * 3., q[1], q[2] * 0.5 + 20.]);
    let t = fenkra_malha::booleana::booleana(&pilar, &laje, fenkra_malha::booleana::OpMalha::Unir).unwrap();
    let mut d = Document::new("T");
    corpo_malha(&mut d, &t);
    let p = planejar(&d, &OpcoesImpressao::default(), &HashMap::new()).unwrap();
    assert!((p.corpos[0].balanco.area - 200.).abs() < 1e-6, "{}", p.corpos[0].balanco.area);
}

#[test]
fn stl_binario_sai_na_mesa() {
    let d = dois_corpos();
    let o = OpcoesImpressao::default();
    let p = planejar(&d, &o, &HashMap::new()).unwrap();
    let a = pasta("stl").join("m.stl");
    gravar_stl(&p, "Peças", &a).unwrap();
    let b = std::fs::read(&a).unwrap();
    let n = u32::from_le_bytes([b[80], b[81], b[82], b[83]]) as usize;
    assert_eq!(b.len(), 84 + 50 * n, "binário");
    let c = fenkra_conversor::malha::ler_para_teste(fenkra_conversor::Formato::Stl, &a).unwrap();
    assert!(c.no_lugar()[0].caixa().unwrap().0[2].abs() < 1e-3);
}

#[test]
fn fatiador_e_outro_processo_e_o_caminho_vai_inteiro() {
    assert!(abrir_no_fatiador("", std::path::Path::new("x.3mf")).unwrap_err().to_string().contains("fatiador"));
    assert!(abrir_no_fatiador("programa_que_nao_existe_do_fenkra", std::path::Path::new("x.3mf")).is_err());
    let p = arquivo_para_fatiador("Peça com espaço/e barra");
    assert!(p.to_string_lossy().ends_with(".3mf") && !p.file_name().unwrap().to_string_lossy().contains(' '));
    // um programa de verdade recebe o arquivo como um argumento e o Fenkra não espera por ele
    let dir = pasta("fatiador");
    let alvo = dir.join("com espaço.3mf");
    std::fs::write(&alvo, b"x").unwrap();
    let programa = if cfg!(windows) { "where.exe" } else { "ls" };
    let mut filho = abrir_no_fatiador(programa, &alvo).unwrap();
    let _ = filho.wait();
}

/// Amostras para o validador externo: `FENKRA_3MF_SAIDA=<pasta> cargo test -p fenkra-conversor --test
/// impressao amostras` e depois `python scripts/validar_3mf.py <pasta>`.
#[test]
fn amostras_para_a_lib3mf() {
    let Some(dir) = std::env::var_os("FENKRA_3MF_SAIDA").map(PathBuf::from) else { return };
    std::fs::create_dir_all(&dir).unwrap();
    let o = OpcoesImpressao { titulo: "Amostra Fenkra".into(), autor: "Fenkra".into(), descricao: "Dois corpos".into(), ..Default::default() };
    gravar_3mf(&planejar(&dois_corpos(), &o, &HashMap::new()).unwrap(), &o, "dois corpos", &dir.join("dois_corpos.3mf")).unwrap();
    let mut d = Document::new("Duas cores");
    caixa_brep(&mut d, "20", Some([200, 30, 30]));
    d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: "5".into(), b: "40".into(), c: "0".into(), d: "0".into(), op: BoolOp::Add }));
    let n = d.features.len() - 1;
    d.features[n].appearance = Some(Appearance { color: [30, 30, 200], finish: Finish::default(), texture: None });
    gravar_3mf(&planejar(&d, &o, &HashMap::new()).unwrap(), &o, "cores", &dir.join("cor_por_face.3mf")).unwrap();
}
