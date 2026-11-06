// SPDX-License-Identifier: LGPL-2.1-or-later
//! Corpus do pacote de visualização: peça, montagem e desenho montados em código, recalculados, e o
//! pacote conferido contra os números do próprio documento.
//!
//! `FENKRA_FKV_SAIDA=<pasta>` grava os `.fkv` e as páginas autocontidas para ver no navegador.

use fenkra_core::doc::NamedView;
use fenkra_core::pmi::{Pmi, PmiCapture, PmiKind, PmiTarget, TargetClass};
use fenkra_core::{Document, Id};
use fenkra_desenho::{Dim, DimKind, Drawing, Format, Source, Table, TableKind, ViewKind};
use fenkra_montagem::display::DisplayState;
use fenkra_montagem::{Assembly, NodeKind, Placement};
use fenkra_pacote::{de_arquivo, de_desenho, de_montagem, de_peca, pagina_html, Opcoes, Pacote};
use std::path::{Path, PathBuf};

fn pasta(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra_pacote_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn saida(nome: &str, p: &Pacote) {
    let Ok(dir) = std::env::var("FENKRA_FKV_SAIDA") else { return };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let b = p.bytes().unwrap();
    std::fs::write(dir.join(format!("{nome}.fkv")), &b).unwrap();
    std::fs::write(dir.join(format!("{nome}.html")), pagina_html(Some(&b))).unwrap();
}

/// A placa de exemplo com anotações 3D: cota de diâmetro presa ao furo, cota linear, nota, captura.
fn placa() -> Document {
    let mut d = fenkra_solidos::example_document();
    let id = d.alloc();
    d.pmi.push(Pmi {
        id,
        name: "Diâmetro do furo".into(),
        kind: PmiKind::Diameter,
        anchors: vec![[48., 24., 10.], [53., 24., 10.]],
        offset: [10., 10., 12.],
        text: String::new(),
        tol_plus: "0,1".into(),
        tol_minus: "0,1".into(),
        visible: true,
        plane: None,
        style: Default::default(),
        folder: String::new(),
        targets: vec![
            Some(PmiTarget { class: TargetClass::ArestaCircular, point: [48., 24., 10.], dir: [0., 0., 1.], radius: 5., concave: true }),
            None,
        ],
    });
    let id2 = d.alloc();
    d.pmi.push(Pmi {
        id: id2,
        name: "Comprimento".into(),
        kind: PmiKind::Linear,
        anchors: vec![[-60., -36., 10.], [60., -36., 10.]],
        offset: [0., -12., 0.],
        text: String::new(),
        tol_plus: String::new(),
        tol_minus: String::new(),
        visible: true,
        plane: None,
        style: Default::default(),
        folder: String::new(),
        targets: vec![],
    });
    let cap = d.alloc();
    d.pmi_captures.push(PmiCapture {
        id: cap,
        name: "Furo em detalhe".into(),
        view: NamedView { name: "Furo".into(), target: [48., 24., 10.], yaw: -0.5, pitch: 0.9, dist: 90., ortho: false },
        shown: vec![id],
        hidden: vec![],
        with_model: false,
    });
    d.views.push(NamedView { name: "Topo".into(), target: [0., 0., 5.], yaw: 0., pitch: 1.5, dist: 300., ortho: true });
    d
}

fn corpus(dir: &Path, quantas: usize, distintas: bool) -> PathBuf {
    std::fs::write(dir.join("placa.fkp"), placa().to_json()).unwrap();
    let mut a = Assembly::new("Conjunto de exemplo");
    a.props.pn = "CJ-0001".into();
    let g = a.add_group("Placas", None);
    let mut ids = vec![];
    for k in 0..quantas {
        let arq = if distintas { format!("placa_{k}.fkp") } else { "placa.fkp".to_string() };
        if distintas && k > 0 {
            // cada peça com a espessura diferente: nenhuma divide malha com outra
            let mut d = placa();
            d.params.iter_mut().find(|p| p.name == "esp").unwrap().expr = format!("{}", 8. + k as f64 * 0.1);
            std::fs::write(dir.join(&arq), d.to_json()).unwrap();
        }
        let p = Placement { origin: [(k % 10) as f64 * 150., (k / 10) as f64 * 100., (k % 3) as f64 * 30.], ..Default::default() };
        ids.push(a.add_component(&format!("Placa {}", k + 1), NodeKind::Part { path: if distintas && k == 0 { "placa.fkp".into() } else { arq } }, p, Some(g)));
    }
    if quantas >= 2 {
        a.display_states.push(DisplayState {
            name: "Explodido".into(),
            hidden: vec![],
            colors: vec![(ids[0], [200, 80, 60])],
            offsets: vec![(ids[1], [0., 0., 60.])],
            camera: None,
            linhas: vec![],
        });
        a.display_states.push(DisplayState { name: "Só a primeira".into(), hidden: ids[1..].to_vec(), colors: vec![], offsets: vec![], camera: None, linhas: vec![] });
    }
    let f = dir.join(format!("conjunto_{quantas}{}.fka", if distintas { "d" } else { "" }));
    std::fs::write(&f, a.to_json()).unwrap();
    f
}

fn desenho(dir: &Path) -> PathBuf {
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let src = Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("Placa de exemplo", Format::A3);
    let sh = d.sheets[0].id;
    let ids = d.add_standard_set(sh, src.clone(), [95., 235.], 1., 100.);
    let mut m = fenkra_desenho::source::Models::new(Some(dir.to_path_buf()));
    let f = m.proj(&d, ids[0]);
    if let Some(ax) = f.axes {
        let id = d.alloc();
        d.dims.push(Dim { id, view: ids[0], kind: DimKind::Horizontal, anchors: vec![ax.unproject([f.lo[0], f.hi[1]]), ax.unproject([f.hi[0], f.hi[1]])], offset: 8., text: String::new(), tol: String::new(), ..Default::default() });
    }
    let id = d.alloc();
    d.tables.push(Table { id, sheet: sh, kind: TableKind::Holes, source: src, pos: [270., 150.], level: Default::default(), cfg: Default::default() });
    let f = dir.join("placa.fkd");
    std::fs::write(&f, d.to_json()).unwrap();
    f
}

#[test]
fn peca_vira_pacote_e_volta_conferido() {
    let doc = placa();
    let r = fenkra_solidos::recompute(&doc);
    let vol: f64 = r.bodies.iter().map(|b| b.mass.volume).sum();
    let p = de_peca(&doc, &Opcoes::default()).unwrap();
    p.validar().unwrap();
    let m = &p.manifesto;
    assert_eq!(m["tipo"], "peca");
    assert_eq!(m["atributos"]["pn"], "FK-0001");
    assert!((m["atributos"]["volume_mm3"].as_f64().unwrap() - vol).abs() < 1e-3);
    // caixa 120 x 72 x altura do corpo: o comprimento da placa
    let dims = m["atributos"]["dimensoes_mm"].as_array().unwrap();
    assert!((dims[0].as_f64().unwrap() - 120.).abs() < 1e-3 && (dims[1].as_f64().unwrap() - 72.).abs() < 1e-3);
    // PMI: valor medido do comprimento e texto da cota de diâmetro com a tolerância
    let pmi = m["pmi"].as_array().unwrap();
    assert_eq!(pmi.len(), 2);
    assert!(pmi[0]["texto"].as_str().unwrap().starts_with('Ø'));
    assert!((pmi[1]["valor"].as_f64().unwrap() - 120.).abs() < 1e-9);
    assert_eq!(pmi[0]["alvos"][0]["classe"], "aresta_circular");
    assert_eq!(m["capturas"][0]["pmi"].as_array().unwrap().len(), 1);
    // pontos de captura exatos: centro do furo em (48, 24) e raio dos furos de 5 mm, em precisão dupla
    let cap = &m["malhas"][0]["capturas"];
    let achou = cap["pontos"].as_array().unwrap().iter().any(|q| q[3] == 2.0 && (q[0].as_f64().unwrap() - 48.).abs() < 1e-9 && (q[1].as_f64().unwrap() - 24.).abs() < 1e-9);
    assert!(achou, "centro do furo ausente dos pontos de captura");
    assert!(cap["circulos"].as_array().unwrap().iter().any(|c| (c[3].as_f64().unwrap() - 5.).abs() < 1e-6 || (c[3].as_f64().unwrap() - 6.).abs() < 1e-6));
    // ida e volta pelo ZIP e determinismo byte a byte
    let b = p.bytes().unwrap();
    let q = Pacote::ler(&b).unwrap();
    q.validar().unwrap();
    assert_eq!(q.manifesto, p.manifesto);
    assert_eq!(b, de_peca(&doc, &Opcoes::default()).unwrap().bytes().unwrap());
    saida("peca", &p);
}

#[test]
fn montagem_tem_ids_estaveis_estados_e_massa_somada() {
    let dir = pasta("montagem");
    let f = corpus(&dir, 3, false);
    let p = de_arquivo(&f, &Opcoes::default()).unwrap();
    p.validar().unwrap();
    let m = &p.manifesto;
    assert_eq!(m["tipo"], "montagem");
    // uma peça única em três lugares: a malha entra uma vez
    assert_eq!(m["estatisticas"]["pecas_unicas"], 1);
    assert_eq!(m["estatisticas"]["instancias"], 3);
    assert_eq!(m["malhas"].as_array().unwrap().len(), 1);
    let arv = m["arvore"].as_array().unwrap();
    let pecas: Vec<_> = arv.iter().filter(|e| e["tipo"] == "peca").collect();
    assert_eq!(pecas.len(), 3);
    let una = pecas[0]["atributos"]["massa_kg"].as_f64().unwrap();
    assert!(una > 0.);
    assert!((m["atributos"]["massa_kg"].as_f64().unwrap() - 3. * una).abs() < 1e-5);
    assert_eq!(m["estados"].as_array().unwrap().len(), 2);
    assert_eq!(m["estados"][1]["ocultos"].as_array().unwrap().len(), 2);
    assert!(m["lista"].as_array().unwrap().iter().any(|r| r["quantidade"] == 3));
    // os IDs vêm dos IDs do documento: gerar de novo dá os mesmos
    let ids = |p: &Pacote| p.manifesto["arvore"].as_array().unwrap().iter().map(|e| e["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(ids(&p), ids(&de_arquivo(&f, &Opcoes::default()).unwrap()));
    saida("montagem", &p);
}

#[test]
fn desenho_vira_folhas_vetoriais() {
    let dir = pasta("desenho");
    let f = desenho(&dir);
    let p = de_arquivo(&f, &Opcoes::default()).unwrap();
    p.validar().unwrap();
    let m = &p.manifesto;
    assert_eq!(m["tipo"], "desenho");
    assert!(m["cena"].is_null());
    let folha: serde_json::Value = serde_json::from_slice(&p.arquivos.iter().find(|a| a.0 == "folhas/f0.json").unwrap().1).unwrap();
    assert_eq!(folha["largura"], 420.);
    let itens = folha["itens"].as_array().unwrap();
    assert!(itens.iter().filter(|i| i["t"] == "l").count() > 50, "poucas linhas na folha");
    assert!(itens.iter().any(|i| i["t"] == "x"), "sem textos (cotas e carimbo)");
    assert!(!folha["vistas"].as_array().unwrap().is_empty());
    saida("desenho", &p);
}

#[test]
fn tamanho_do_pacote_de_cem_pecas() {
    let dir = pasta("cem");
    let compartilhada = de_arquivo(&corpus(&dir, 100, false), &Opcoes::default()).unwrap();
    let distintas = de_arquivo(&corpus(&dir, 100, true), &Opcoes::default()).unwrap();
    let (a, b) = (compartilhada.bytes().unwrap().len(), distintas.bytes().unwrap().len());
    let dec = de_arquivo(&dir.join("conjunto_100d.fka"), &Opcoes { decimar: Some(0.3), capturas: false }).unwrap().bytes().unwrap().len();
    eprintln!("100 instâncias de 1 peça: {a} bytes; 100 peças distintas: {b} bytes; distintas decimadas a 30%: {dec} bytes");
    assert_eq!(distintas.manifesto["estatisticas"]["pecas_unicas"], 100);
    assert!(a < b);
    assert!(b < 10 * 1024 * 1024, "meta de 10 MB estourada: {b}");
    saida("grade100", &distintas);
}

#[test]
fn decimar_reduz_a_malha_e_tira_os_pontos_exatos() {
    let doc = placa();
    let cheio = de_peca(&doc, &Opcoes::default()).unwrap();
    let leve = de_peca(&doc, &Opcoes { decimar: Some(0.4), capturas: false }).unwrap();
    leve.validar().unwrap();
    let t = |p: &Pacote| p.manifesto["malhas"][0]["triangulos"].as_u64().unwrap();
    assert!(t(&leve) < t(&cheio), "{} !< {}", t(&leve), t(&cheio));
    assert!(leve.manifesto["malhas"][0].get("capturas").is_none());
    assert_eq!(leve.manifesto["decimado"], 0.4);
}

#[test]
fn pagina_e_autocontida_e_sem_rede() {
    let p = de_peca(&placa(), &Opcoes::default()).unwrap();
    let h = pagina_html(Some(&p.bytes().unwrap()));
    assert!(h.contains("THREE") && h.contains("fkv-dados"));
    assert!(!h.contains("/*@THREE@*/") && !h.contains("/*@PACOTE@*/"));
    // nenhum recurso externo carregado: sem src/href/@import para http
    for marca in ["src=\"http", "href=\"http", "@import", "<link", "src='http"] {
        assert!(!h.contains(marca), "a página referencia a rede: {marca}");
    }
    let vazia = pagina_html(None);
    assert!(vazia.len() < h.len());
}

#[test]
fn documento_sem_corpo_e_erro_claro() {
    let d = Document::new("Vazia");
    let e = de_peca(&d, &Opcoes::default()).unwrap_err();
    assert!(e.contains("corpo"), "{e}");
    let _ = (Id(0), de_montagem, de_desenho);
}
