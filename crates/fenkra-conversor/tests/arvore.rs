// SPDX-License-Identifier: LGPL-2.1-or-later
//! B266: ida e volta da árvore paramétrica. O aceite é sobre as peças do corpus (B53): depois de gravar
//! e ler, volume, área, centroide e graus de liberdade de cada esboço continuam os mesmos.

#[path = "../../fenkra-solidos/tests/common/mod.rs"]
mod common;

use common::*;
use fenkra_conversor::arvore::*;
use fenkra_conversor::{ErroConv, OpcoesImportar, Registro};
use fenkra_core::sketch::{Constraint, Curve, Kind};
use fenkra_core::*;
use fenkra_montagem::{Assembly, NodeKind, Placement};
use fenkra_solidos::eixos::{chaveta_documento, eixo_documento, ParamEixo, TrechoEixo};
use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem};
use fenkra_solidos::fixadores::fixador_documento;
use fenkra_solidos::{example_document, recompute, Status};
use serde_json::{json, Value};
use std::path::PathBuf;

const TOL: f64 = 1e-6;

macro_rules! precisa_kernel {
    () => {
        if !kernel_disponivel() {
            return;
        }
    };
}

// ------------------------------------------------------------------ as peças do corpus

fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(s, BoolOp::Add, "20"));
    d
}

fn com_primitiva(nome: &str, k: PrimKind, v: [&str; 4]) -> Document {
    let mut d = Document::new(nome);
    d.add_feature(primitiva(k, v[0], v[1], v[2], v[3]));
    d
}

fn placa_com_grade() -> Document {
    let mut d = Document::new("placa com grade");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-50., -30.], [50., 30.]);
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let mut pts = vec![];
    for i in -2..=2 {
        for j in -1..=1 {
            pts.push([i as f64 * 20., j as f64 * 20.]);
        }
    }
    let sk = sk_pontos(&mut d, PlaneRef::Datum(topo), &pts);
    d.add_feature(furo_passante(sk, "6"));
    d
}

fn caixa_com_bolso() -> Document {
    let mut d = caixa();
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let sk = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
    d.add_feature(ext(sk, BoolOp::Cut, "5"));
    d
}

fn caixa_arredondada() -> Document {
    let mut d = caixa();
    d.add_feature(FeatureKind::Fillet(EdgeOp { extra: Default::default(), value: "2".into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
    d
}

fn caixa_casca() -> Document {
    let mut d = caixa();
    d.add_feature(FeatureKind::Shell(Shell { espessuras: vec![], faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![], closed: false }));
    d
}

fn flange() -> Document {
    let mut d = Document::new("flange");
    let base = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 30.)]);
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let centro = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    d.add_feature(furo_passante(centro, "20"));
    let furos = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[20., 0.], [0., 20.], [-20., 0.], [0., -20.]]);
    d.add_feature(furo_passante(furos, "6"));
    d
}

fn bloco_l() -> Document {
    let mut d = Document::new("bloco L");
    let sk = sk_poligono(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[0., 0.], [40., 0.], [40., 15.], [20., 15.], [20., 30.], [0., 30.]]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d
}

fn corpus() -> Vec<Document> {
    let eixo = ParamEixo {
        trechos: vec![TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None }, TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None }],
        furo_centro: false,
    };
    vec![
        caixa(),
        com_primitiva("cilindro", PrimKind::Cilindro, ["10", "30", "0", "0"]),
        com_primitiva("esfera", PrimKind::Esfera, ["10", "0", "0", "0"]),
        com_primitiva("cone", PrimKind::Cone, ["12", "6", "20", "0"]),
        com_primitiva("toro", PrimKind::Toro, ["20", "5", "0", "0"]),
        placa_com_grade(),
        caixa_com_bolso(),
        caixa_arredondada(),
        caixa_casca(),
        flange(),
        eixo_documento(&eixo).expect("eixo"),
        engrenagem_documento(&ParamEngrenagem { modulo: 2., dentes: 40, largura: 10., furo: 10., ..Default::default() }).expect("engrenagem"),
        fixador_documento("iso4762", "M6", Some(20.)).unwrap(),
        fixador_documento("iso4032", "M8", None).unwrap(),
        chaveta_documento(25., 40.).unwrap(),
        bloco_l(),
    ]
}

// ------------------------------------------------------------------ medidas

#[derive(Debug)]
struct Medidas {
    volume: f64,
    area: f64,
    centroide: [f64; 3],
    graus: Vec<usize>,
}

fn medir(d: &Document) -> Medidas {
    let r = recompute(d);
    for f in &d.features {
        assert!(!matches!(r.status.get(&f.id), Some(Status::Error(_))), "{}: comando {} com erro: {:?}", d.name, f.name, r.status.get(&f.id));
    }
    let volume: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    let area = r.bodies.iter().map(|b| b.mass.area).sum();
    let centroide = [0, 1, 2].map(|i| r.bodies.iter().map(|b| b.mass.center[i] * b.mass.volume).sum::<f64>() / volume);
    let vars = d.eval_params().unwrap();
    let graus = d.features.iter().filter_map(|f| if let FeatureKind::Sketch(s) = &f.kind { Some(s.data.clone().solve(&vars, &[]).dof) } else { None }).collect();
    Medidas { volume, area, centroide, graus }
}

fn igual(a: &Medidas, b: &Medidas, nome: &str) {
    assert!(a.volume > 0., "{nome}: peça sem volume");
    assert!((a.volume - b.volume).abs() < TOL, "{nome}: volume {} virou {}", a.volume, b.volume);
    assert!((a.area - b.area).abs() < TOL, "{nome}: área {} virou {}", a.area, b.area);
    for i in 0..3 {
        assert!((a.centroide[i] - b.centroide[i]).abs() < TOL, "{nome}: centroide {:?} virou {:?}", a.centroide, b.centroide);
    }
    assert_eq!(a.graus, b.graus, "{nome}: graus de liberdade dos esboços");
}

fn gravar(d: &Document) -> String {
    let e = exportar_peca(d);
    assert_eq!(e.fora, vec![], "{}: não deveria ficar nada de fora", d.name);
    serde_json::to_string_pretty(&e.arquivo()).unwrap()
}

fn ida_e_volta(d: &Document) -> Document {
    let texto = gravar(d);
    validar(&serde_json::from_str(&texto).unwrap()).unwrap_or_else(|e| panic!("{}: o que gravamos foge do esquema: {e}", d.name));
    let a = ler_texto(&texto).unwrap();
    ler_peca(&a.pecas[0]).unwrap().doc
}

// ------------------------------------------------------------------ aceite

#[test]
fn corpus_inteiro_volta_com_volume_area_centroide_e_graus() {
    precisa_kernel!();
    let pecas = corpus();
    assert_eq!(pecas.len(), 16);
    for d in pecas {
        let lida = ida_e_volta(&d);
        igual(&medir(&d), &medir(&lida), &d.name);
        assert_eq!(lida.features.len(), d.features.len(), "{}", d.name);
        assert_eq!(lida.features.iter().map(|f| &f.name).collect::<Vec<_>>(), d.features.iter().map(|f| &f.name).collect::<Vec<_>>());
        assert_eq!(lida.params.iter().map(|p| (&p.name, &p.expr)).collect::<Vec<_>>(), d.params.iter().map(|p| (&p.name, &p.expr)).collect::<Vec<_>>());
    }
}

#[test]
fn segunda_ida_grava_a_mesma_receita() {
    precisa_kernel!();
    // ids mudam a cada leitura; a árvore sem ids tem que ser a mesma
    fn sem_ids(v: &mut Value, mapa: &mut std::collections::HashMap<String, String>) {
        match v {
            Value::String(s) if s.len() > 1 && s.starts_with('e') && s[1..].chars().all(|c| c.is_ascii_digit()) => {
                let n = mapa.len();
                *s = mapa.entry(s.clone()).or_insert(format!("#{n}")).clone();
            }
            Value::Array(l) => l.iter_mut().for_each(|x| sem_ids(x, mapa)),
            Value::Object(o) => o.values_mut().for_each(|x| sem_ids(x, mapa)),
            _ => {}
        }
    }
    let d = placa_com_grade();
    let mut a = serde_json::to_value(exportar_peca(&d).peca).unwrap();
    let mut b = serde_json::to_value(exportar_peca(&ida_e_volta(&d)).peca).unwrap();
    a["id"] = json!("");
    b["id"] = json!("");
    sem_ids(&mut a, &mut Default::default());
    sem_ids(&mut b, &mut Default::default());
    assert_eq!(a, b);
}

#[test]
fn peca_de_exemplo_volta_viva_e_o_parametro_ainda_comanda() {
    precisa_kernel!();
    let d = example_document();
    let mut lida = ida_e_volta(&d);
    igual(&medir(&d), &medir(&lida), "exemplo");
    let antes = medir(&lida).volume;
    definir_param(&mut lida, "esp", "20");
    let depois = medir(&lida).volume;
    assert!(depois > 1.5 * antes, "o histórico não comanda a peça: {antes} -> {depois}");
}

/// Retângulo cotado com uma folga: largura presa, altura livre.
fn esboco_cotado(cotas: bool) -> Document {
    let mut d = Document::new("cotado");
    d.add_param("L", "40", Unit::Comprimento).unwrap();
    let mut data = SketchData::default();
    let mut al = || d.alloc();
    data.add_rect(&mut al, [0., 0.], [40., 20.], false);
    let linhas: Vec<Id> = data.curves.iter().filter(|c| matches!(c.geom, Curve::Line { .. })).map(|c| c.id).collect();
    if cotas {
        let id = al();
        data.constraints.push(Constraint { id, kind: Kind::Length(linhas[0], "L".into()), label: None, reference: false, flipped: false });
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(ext(sk, BoolOp::Add, "5"));
    d
}

#[test]
fn graus_de_liberdade_sobrevivem_com_e_sem_cota() {
    precisa_kernel!();
    let (livre, cotado) = (esboco_cotado(false), esboco_cotado(true));
    let (a, b) = (medir(&livre), medir(&cotado));
    assert!(a.graus[0] > b.graus[0], "a cota deveria tirar um grau: {:?} {:?}", a.graus, b.graus);
    igual(&a, &medir(&ida_e_volta(&livre)), "livre");
    let lido = ida_e_volta(&cotado);
    igual(&b, &medir(&lido), "cotado");
    let FeatureKind::Sketch(s) = &lido.features[0].kind else { panic!() };
    assert!(s.data.constraints.iter().any(|c| matches!(&c.kind, Kind::Length(_, v) if v == "L")));
}

// ------------------------------------------------------------------ reancoragem (ADR 0010)

fn caixa_com_filete(ponto: [f64; 3]) -> Document {
    let mut d = caixa();
    d.add_feature(filete("3", vec![ponto], vec![]));
    d
}

fn arquivo_com_filete(ponto: [f64; 3]) -> Arquivo {
    let mut a = exportar_peca(&caixa()).arquivo();
    let corpo = a.pecas[0].corpos[0].id.clone();
    a.pecas[0].arvore.push(Item {
        id: "filete".into(),
        nome: "Filete de fora".into(),
        corpo: Some(corpo),
        suprimido: false,
        suprimir_se: String::new(),
        oculto: false,
        cor: None,
        comando: Comando::Filete { raio: "3".into(), arestas: Arestas::Pontos(vec![ponto]) },
    });
    a
}

#[test]
fn ponto_qualquer_sobre_a_aresta_reancora_no_meio_e_ganha_chave() {
    precisa_kernel!();
    // a aresta superior frontal vai de x = -20 a 20: o meio é x = 0, e o programa de fora mandou x = 13
    let certo = medir(&caixa_com_filete([0., -15., 20.]));
    let r = ler_peca(&arquivo_com_filete([13., -15., 20.]).pecas[0]).unwrap();
    assert_eq!(r.reancoragem.reancoradas, 1);
    assert_eq!(r.reancoragem.perdidas, vec![]);
    let FeatureKind::Fillet(o) = &r.doc.features.last().unwrap().kind else { panic!() };
    let EdgeSel::Edges(p) = &o.edges else { panic!() };
    assert!(p[0][0].abs() < 1e-3, "{:?}", p);
    assert!(o.keys[0].is_some(), "a referência deveria sair com dono e papel");
    igual(&certo, &medir(&r.doc), "filete reancorado");
}

#[test]
fn ponto_longe_de_tudo_fica_perdido_e_nao_escolhe_em_silencio() {
    precisa_kernel!();
    let r = ler_peca(&arquivo_com_filete([13., -14., 19.]).pecas[0]).unwrap();
    assert_eq!(r.reancoragem.reancoradas, 0);
    assert_eq!(r.reancoragem.perdidas.len(), 1, "{:?}", r.reancoragem);
    assert!(r.reancoragem.perdidas[0].1.starts_with("referência perdida"), "{:?}", r.reancoragem);
}

#[test]
fn ponto_no_vertice_e_ambiguo_e_nao_reancora() {
    precisa_kernel!();
    let r = ler_peca(&arquivo_com_filete([20., -15., 20.]).pecas[0]).unwrap();
    assert_eq!(r.reancoragem.reancoradas, 0);
    assert_eq!(r.reancoragem.perdidas.len(), 1, "{:?}", r.reancoragem);
}

#[test]
fn documento_editado_exporta_a_aresta_onde_ela_esta_agora() {
    precisa_kernel!();
    let mut d = Document::new("editada");
    d.add_param("H", "20", Unit::Comprimento).unwrap();
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(s, BoolOp::Add, "H"));
    let f = d.add_feature(filete("3", vec![[0., -15., 20.]], vec![]));
    // o recálculo dá a chave, como a tela faz; depois a cota muda e o ponto do clique fica para trás
    reancorar(&mut d);
    definir_param(&mut d, "H", "35");
    assert!(matches!(recompute(&d).status.get(&f), Some(Status::Ok)), "a chave segue a aresta");
    let esperado = medir(&d);
    atualizar_pontos(&mut d);
    let lida = ida_e_volta(&d);
    igual(&esperado, &medir(&lida), "editada");
}

// ------------------------------------------------------------------ o que não cabe

#[test]
fn comando_sem_equivalente_e_quem_depende_dele_ficam_escritos_no_arquivo() {
    let mut d = Document::new("com fora");
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [10., 10.]);
    let e = d.add_feature(ext(s, BoolOp::Add, "5"));
    let m = d.add_feature(FeatureKind::Scale(Scale { center: PointRef::Origin, factor: "2".into() }));
    let p = d.add_feature(FeatureKind::Mirror(Mirror { plane: PlaneRef::Origin(OriginPlane::XY), sources: vec![m] }));
    let x = exportar_peca(&d);
    assert_eq!(x.peca.arvore.iter().map(|i| i.nome.clone()).collect::<Vec<_>>(), vec![d.name_of(s), d.name_of(e)]);
    let nomes: Vec<&str> = x.fora.iter().map(|f| f.nome.as_str()).collect();
    assert_eq!(nomes, vec![d.name_of(m).as_str(), d.name_of(p).as_str()]);
    let a = x.arquivo();
    assert_eq!(a.fora.len(), 2);
    validar(&serde_json::to_value(&a).unwrap()).unwrap();
}

#[test]
fn referencia_e_esboco_que_se_apontam_em_qualquer_ordem_entram() {
    let mut d = Document::new("cruzada");
    let s = sk_poligono(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[0., 0.], [10., 0.], [10., 10.]]);
    let (id_ponto, id_linha) = {
        let FeatureKind::Sketch(k) = &d.feature(s).unwrap().kind else { panic!() };
        (k.data.points[0].id, k.data.curves[0].id)
    };
    d.add_geometry(RefKind::LineDir { point: PointRef::SketchPoint { sketch: s, point: id_ponto }, dir: AxisRef::SketchLine { sketch: s, entity: id_linha }, start: "0".into(), end: "10".into() });
    let e = exportar_peca(&d);
    assert_eq!(e.fora, vec![]);
    let lido = ler_peca(&e.arquivo().pecas[0]).unwrap().doc;
    assert_eq!(lido.references.len(), 1);
    assert!(matches!(lido.references[0].kind, RefKind::LineDir { .. }));
}

// ------------------------------------------------------------------ arquivos ruins

fn texto(v: Value) -> String {
    serde_json::to_string(&v).unwrap()
}

fn base() -> Value {
    json!({"formato": "fenkra-arvore", "format_version": 1, "unidades": "mm", "pecas": [{"id": "p", "nome": "P", "arvore": [
        {"id": "s", "nome": "Esboço", "comando": {"tipo": "esboco", "plano": "XY", "pontos": [{"id": "a", "x": 0, "y": 0}, {"id": "b", "x": 10, "y": 0}], "curvas": [{"id": "l", "geometria": {"tipo": "reta", "a": "a", "b": "b"}}]}}
    ]}]})
}

fn mensagem(r: Result<Remontada, ErroConv>) -> String {
    r.err().map(|e| e.to_string()).unwrap_or_default()
}

#[test]
fn arquivo_escrito_a_mao_passa() {
    let a = ler_texto(&texto(base())).unwrap();
    let r = ler_peca(&a.pecas[0]).unwrap();
    let FeatureKind::Sketch(s) = &r.doc.features[0].kind else { panic!() };
    assert_eq!((s.data.points.len(), s.data.curves.len()), (2, 1));
}

#[test]
fn exemplo_da_especificacao_le_e_calcula() {
    precisa_kernel!();
    let doc = include_str!("../../../docs/formatos/arvore.md");
    let inicio = doc.find("```json").unwrap() + 7;
    let fim = inicio + doc[inicio..].find("```").unwrap();
    let a = ler_texto(&doc[inicio..fim]).unwrap();
    let r = ler_peca(&a.pecas[0]).unwrap();
    assert_eq!(r.reancoragem.perdidas, vec![]);
    let m = medir(&r.doc);
    let esperado = 40. * 20. * 5. - (4. - std::f64::consts::PI) * 5.;
    assert!((m.volume - esperado).abs() < 1e-3, "{} contra {esperado}", m.volume);
    assert_eq!(m.graus, vec![7], "oito coordenadas menos a cota de comprimento");
}

#[test]
fn versao_futura_e_recusada_e_explica() {
    let mut v = base();
    v["format_version"] = json!(99);
    let e = ler_texto(&texto(v)).unwrap_err().to_string();
    assert!(e.contains("v99") && e.contains("mais novo"), "{e}");
}

#[test]
fn id_solto_e_id_repetido_sao_recusados_com_o_nome_da_peca() {
    let mut v = base();
    v["pecas"][0]["arvore"].as_array_mut().unwrap().push(json!({"id": "e", "nome": "E", "comando": {"tipo": "extrusao", "esboco": "nao_existe", "operacao": "unir", "termino": {"tipo": "cego", "distancia": "5"}}}));
    let a = ler_texto(&texto(v.clone())).unwrap();
    let m = mensagem(ler_peca(&a.pecas[0]));
    assert!(m.contains("'nao_existe'") && m.contains("peça 'P'"), "{m}");

    v["pecas"][0]["arvore"][1]["id"] = json!("s");
    let a = ler_texto(&texto(v)).unwrap();
    assert!(mensagem(ler_peca(&a.pecas[0])).contains("repetido"));
}

#[test]
fn restricao_com_entidades_de_menos_ou_cota_sem_valor_nao_passam() {
    let mut v = base();
    v["pecas"][0]["arvore"][0]["comando"]["restricoes"] = json!([{"tipo": "coincidente", "entidades": ["a"]}]);
    let a = ler_texto(&texto(v.clone())).unwrap();
    assert!(mensagem(ler_peca(&a.pecas[0])).contains("pede 2"));
    v["pecas"][0]["arvore"][0]["comando"]["restricoes"] = json!([{"tipo": "comprimento", "entidades": ["l"]}]);
    let a = ler_texto(&texto(v)).unwrap();
    assert!(mensagem(ler_peca(&a.pecas[0])).contains("sem valor"));
}

// ------------------------------------------------------------------ importar e montagem

fn pasta(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("Fenkra árvore {} {nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn importar_pelo_registro_entrega_relatorio_miniatura_e_selo() {
    precisa_kernel!();
    let dir = pasta("registro");
    let arquivo = dir.join("placa.fkx");
    let d = placa_com_grade();
    std::fs::write(&arquivo, gravar(&d)).unwrap();
    let i = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap();
    igual(&medir(&d), &medir(&i.doc), "importada");
    assert!((i.relatorio.volume - medir(&d).volume).abs() < TOL);
    assert_eq!(i.relatorio.corpos, 1);
    assert!(i.relatorio.esquema.starts_with("fenkra-arvore v1"));
    assert!(!i.previa.tris.is_empty());
    assert_eq!(i.doc.origem.as_ref().unwrap().nome, "placa.fkx");
    assert!(i.conjunto.is_none());
}

#[test]
fn montagem_volta_com_estrutura_posicoes_e_peca_repetida_uma_vez() {
    precisa_kernel!();
    let dir = pasta("montagem");
    std::fs::write(dir.join("caixa.fkp"), caixa().to_json()).unwrap();
    std::fs::write(dir.join("bloco.fkp"), bloco_l().to_json()).unwrap();
    let mut sub = Assembly::new("sub");
    sub.add_component("Bloco", NodeKind::Part { path: "bloco.fkp".into() }, Placement { origin: [0., 0., 7.], ..Default::default() }, None);
    std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
    let mut a = Assembly::new("Conjunto");
    let g = a.add_group("Grupo", None);
    a.add_component("Caixa A", NodeKind::Part { path: "caixa.fkp".into() }, Placement::default(), Some(g));
    a.add_component("Caixa B", NodeKind::Part { path: "caixa.fkp".into() }, Placement { origin: [100., 0., 0.], ..Default::default() }, Some(g));
    a.add_component("Sub", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement { origin: [0., 50., 0.], ..Default::default() }, None);

    let arquivo = exportar_montagem(&a, &dir).unwrap();
    assert_eq!(arquivo.pecas.len(), 2, "caixa repetida entra uma vez");
    let v = serde_json::to_value(&arquivo).unwrap();
    validar(&v).unwrap();

    let (c, _) = remontar(&ler_texto(&v.to_string()).unwrap()).unwrap();
    let saida = pasta("montagem saida");
    let fka = gravar_em_pasta(&c, &saida).unwrap();
    let lida = Assembly::from_json(&std::fs::read_to_string(&fka).unwrap()).unwrap();
    let no = |nome: &str| lida.nodes.iter().find(|n| n.name == nome).unwrap_or_else(|| panic!("{nome}"));
    assert_eq!(no("Caixa B").placement.origin, [100., 0., 0.]);
    assert_eq!(no("Caixa B").parent, Some(no("Grupo").id));
    assert_eq!(no("Bloco").placement.origin, [0., 50., 7.], "posição do subconjunto somada");
    assert_eq!(no("Bloco").parent, Some(no("Sub").id));
    let NodeKind::Part { path } = &no("Caixa A").kind else { panic!() };
    let doc = Document::from_json(&std::fs::read_to_string(saida.join(path)).unwrap()).unwrap();
    igual(&medir(&caixa()), &medir(&doc), "peça da montagem");
}

#[test]
fn nome_hostil_nao_escreve_fora_da_pasta() {
    let mut v = base();
    v["pecas"][0]["nome"] = json!("..\\..\\fora/da pasta");
    v["montagem"] = json!({"nome": "../../fora", "componentes": [{"id": "c", "nome": "C", "peca": "p", "origem": [0, 0, 0], "rotacao": [[1, 0, 0], [0, 1, 0], [0, 0, 1]]}]});
    let (c, _) = remontar(&ler_texto(&texto(v)).unwrap()).unwrap();
    let dir = pasta("hostil");
    let fka = gravar_em_pasta(&c, &dir).unwrap();
    assert_eq!(fka.parent(), Some(dir.as_path()));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
}

#[test]
fn componente_que_aponta_para_peca_ou_pai_que_nao_existem_e_recusado() {
    let mut v = base();
    v["montagem"] = json!({"nome": "M", "componentes": [{"id": "c", "nome": "C", "peca": "x", "origem": [0, 0, 0], "rotacao": [[1, 0, 0], [0, 1, 0], [0, 0, 1]]}]});
    assert!(remontar(&ler_texto(&texto(v.clone())).unwrap()).err().unwrap().to_string().contains("'x'"));
    v["montagem"]["componentes"][0]["peca"] = json!("p");
    v["montagem"]["componentes"][0]["pai"] = json!("z");
    assert!(remontar(&ler_texto(&texto(v)).unwrap()).err().unwrap().to_string().contains("'z'"));
}
