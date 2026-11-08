// SPDX-License-Identifier: LGPL-2.1-or-later
//! B39: a lista de comandos, a consulta de topologia, o checador e o contrato JSON da linha de comando.

#[path = "../../fenkra-solidos/tests/common/mod.rs"]
mod common;

use common::*;
use fenkra_cli::checador::{comparar, Tolerancias};
use fenkra_cli::pasta::Pasta;
use fenkra_cli::topologia::analisar;
use fenkra_cli::{filho, Isolamento, Sessao, API_VERSAO, COMANDOS, COMANDOS_DE_MODELO};
use fenkra_core::*;
use fenkra_kernel::Xform;
use fenkra_solidos::final_shape;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const CLI: &str = env!("CARGO_BIN_EXE_fenkra-cli");

macro_rules! precisa_kernel {
    () => {
        if !kernel_disponivel() {
            return;
        }
    };
}

fn pasta_limpa(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra-cli-peças-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sessao(d: &Path) -> Sessao {
    Sessao::nova(Pasta::nova(d).unwrap(), Isolamento::Local)
}

fn retangulo(l: &str) -> Value {
    json!({
        "plano": "XY",
        "pontos": [{ "id": "a", "x": 0, "y": 0 }, { "id": "b", "x": 40, "y": 0 }, { "id": "c", "x": 40, "y": 30 }, { "id": "d", "x": 0, "y": 30 }],
        "curvas": [
            { "id": "l1", "geometria": { "tipo": "reta", "a": "a", "b": "b" } }, { "id": "l2", "geometria": { "tipo": "reta", "a": "b", "b": "c" } },
            { "id": "l3", "geometria": { "tipo": "reta", "a": "c", "b": "d" } }, { "id": "l4", "geometria": { "tipo": "reta", "a": "d", "b": "a" } },
        ],
        "restricoes": [
            { "tipo": "fixa", "entidades": ["a"] }, { "tipo": "horizontal", "entidades": ["l1"] }, { "tipo": "vertical", "entidades": ["l2"] },
            { "tipo": "horizontal", "entidades": ["l3"] }, { "tipo": "vertical", "entidades": ["l4"] },
            { "tipo": "comprimento", "entidades": ["l1"], "valor": l }, { "tipo": "comprimento", "entidades": ["l2"], "valor": "30" },
        ],
    })
}

fn bloco(s: &mut Sessao) {
    s.executar("nova_peca", &json!({ "nome": "Bloco" }))
        .unwrap();
    s.executar(
        "definir_parametro",
        &json!({ "nome": "L", "expressao": "40" }),
    )
    .unwrap();
    let e = s.executar("criar_esboco", &retangulo("L")).unwrap();
    assert_eq!(e["status"], "ok", "{e}");
    assert_eq!(e["graus_de_liberdade"], 0, "{e}");
    let r = s.executar("extrudar", &json!({ "esboco": e["id"], "operacao": "unir", "termino": { "tipo": "cego", "distancia": "20" } })).unwrap();
    assert_eq!(r["status"], "ok", "{r}");
}

fn volume(s: &mut Sessao) -> f64 {
    s.executar("medir", &json!({})).unwrap()["massa"]["volume"]
        .as_f64()
        .unwrap()
}

fn placa(furo_x: f64) -> Document {
    let mut d = Document::new("placa");
    let base = sk_rect(
        &mut d,
        PlaneRef::Origin(OriginPlane::XY),
        [-50., -30.],
        [50., 30.],
    );
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let sk = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[furo_x, 0.], [-20., 10.]]);
    d.add_feature(furo_passante(sk, "6"));
    d
}

#[test]
fn peca_por_comandos_confere_status_volume_e_caixa() {
    precisa_kernel!();
    let d = pasta_limpa("bloco");
    let mut s = sessao(&d);
    bloco(&mut s);
    let r = s.executar("recalcular", &json!({})).unwrap();
    assert_eq!(r["status"], "ok", "{r}");
    assert_eq!(r["faces"], 6);
    assert!((r["volume"].as_f64().unwrap() - 24000.).abs() < 1e-6, "{r}");
    let m = s.executar("medir", &json!({})).unwrap();
    for i in 0..3 {
        assert!(
            (m["caixa"]["tamanho"][i].as_f64().unwrap() - [40., 30., 20.][i]).abs() < 1e-5,
            "{m}"
        );
    }
    // a cota segue o parâmetro: é peça paramétrica, não forma morta
    let p = s
        .executar(
            "definir_parametro",
            &json!({ "nome": "L", "expressao": "60" }),
        )
        .unwrap();
    assert_eq!(p["valor"], 60.0);
    assert!((volume(&mut s) - 36000.).abs() < 1e-6);
    let l = s.executar("parametros", &json!({})).unwrap();
    assert_eq!(l["parametros"][0]["nome"], "L", "{l}");
    assert_eq!(l["parametros"][0]["expressao"], "60", "{l}");
}

#[test]
fn comando_recusado_nao_muda_a_arvore() {
    precisa_kernel!();
    let d = pasta_limpa("recusa");
    let mut s = sessao(&d);
    bloco(&mut s);
    let antes = s.executar("arvore", &json!({})).unwrap();
    let cego = json!({ "tipo": "cego", "distancia": "5" });
    let casos = [
        (
            "extrudar",
            json!({ "esboco": "nao-existe", "operacao": "unir", "termino": cego }),
            "arvore",
        ),
        (
            "extrudar",
            json!({ "esboco": "esboco1", "operacao": "explodir", "termino": cego }),
            "arvore",
        ),
        (
            "extrudar",
            json!({ "esboco": "esboco1", "operacao": "unir", "termino": cego, "campo_inventado": 1 }),
            "arvore",
        ),
        (
            "definir_parametro",
            json!({ "nome": "L", "expressao": "2 +" }),
            "parametro",
        ),
        (
            "definir_parametro",
            json!({ "nome": "nome com espaço", "expressao": "2" }),
            "arvore",
        ),
        (
            "macro",
            json!({ "texto": "qualquer" }),
            "comando_desconhecido",
        ),
    ];
    for (c, a, codigo) in casos {
        let e = s.executar(c, &a).unwrap_err();
        assert_eq!(e.codigo, codigo, "{c} {a}: {e}");
    }
    assert_eq!(s.executar("arvore", &json!({})).unwrap(), antes);
    assert!((volume(&mut s) - 24000.).abs() < 1e-6);
}

#[test]
fn erro_de_comando_leva_sugestao_no_envelope() {
    let e = fenkra_cli::ErroApi::argumento("falta o campo 'raio'");
    let v = fenkra_cli::envelope(Err(e));
    assert_eq!(v["erro"]["codigo"], "argumento");
    assert!(v["erro"]["sugestao"].as_str().is_some_and(|t| t.contains("campos")), "{v}");
    for c in ["argumento", "parametro", "arvore", "kernel", "formato", "leitura", "gravacao", "caminho_fora", "queda", "tempo", "step_bloqueado", "sem_peca", "sem_solido"] {
        assert!(!fenkra_cli::sugestao_do_codigo(c).starts_with("Releia"), "{c} sem sugestão própria");
    }
}

#[test]
fn comando_que_falha_no_calculo_avisa_e_desfazer_volta() {
    precisa_kernel!();
    let d = pasta_limpa("desfazer");
    let mut s = sessao(&d);
    bloco(&mut s);
    let r = s
        .executar("filetar", &json!({ "raio": "500", "arestas": "todas" }))
        .unwrap();
    assert_eq!(r["status"], "erro", "{r}");
    assert!(r["mensagem"].is_string(), "{r}");
    // B419: o item que falhou traz código, sugestão e a ação certa, na API e no relatório
    assert!(r["codigo"].as_str().is_some_and(|c| c.starts_with("E-")), "{r}");
    assert!(r["sugestao"].as_str().is_some_and(|t| !t.is_empty()), "{r}");
    assert!(r["acao"].is_string(), "{r}");
    let rel = s.executar("recalcular", &json!({})).unwrap();
    assert_eq!(rel["status"], "erro");
    let falho = rel["itens"].as_array().unwrap().iter().find(|i| i["status"] == "erro").unwrap();
    assert_eq!(falho["codigo"], r["codigo"], "{rel}");
    assert!(falho["item"].is_string() && falho["sugestao"].is_string(), "{falho}");
    s.executar("desfazer", &json!({})).unwrap();
    let r = s.executar("recalcular", &json!({})).unwrap();
    assert_eq!(r["status"], "ok", "{r}");
    assert_eq!(r["itens"].as_array().unwrap().len(), 2);
    assert!((r["volume"].as_f64().unwrap() - 24000.).abs() < 1e-6);
}

/// Decodifica o PNG devolvido e conta pixels laranja (a cor do corte) e pixels diferentes do fundo.
fn contar_pixels(png_bytes: &[u8]) -> (usize, usize, (u32, u32)) {
    let dec = png::Decoder::new(png_bytes);
    let mut leitor = dec.read_info().unwrap();
    let mut buf = vec![0; leitor.output_buffer_size()];
    let info = leitor.next_frame(&mut buf).unwrap();
    let px = &buf[..info.buffer_size()];
    let laranja = px.chunks_exact(3).filter(|c| i32::from(c[0]) > i32::from(c[2]) + 60).count();
    let fundo = [242u8, 245, 247];
    let peca = px.chunks_exact(3).filter(|c| c.iter().zip(fundo).any(|(a, b)| (i32::from(*a) - i32::from(b)).abs() > 8)).count();
    (laranja, peca, (info.width, info.height))
}

#[test]
fn ver_devolve_a_imagem_da_peca_por_vista_com_corte_e_grava_arquivo() {
    use base64::Engine;
    precisa_kernel!();
    let d = pasta_limpa("ver");
    let mut s = sessao(&d);
    bloco(&mut s);
    let pedir = |s: &mut Sessao, a: Value| {
        let r = s.executar("ver", &a).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(r["imagem_base64"].as_str().unwrap()).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{r}");
        (r, bytes)
    };
    // inteira: a peça aparece e nada tem a cor de corte
    let (r, b) = pedir(&mut s, json!({ "vista": "iso", "largura": 320, "altura": 240 }));
    let (laranja, peca, tam) = contar_pixels(&b);
    assert_eq!(tam, (320, 240));
    assert_eq!(laranja, 0, "{r}");
    assert!(peca > 5_000 && peca < 60_000, "{peca}");
    assert_eq!(r["tipo"], "image/png");
    // cortada pela metade de baixo vista de baixo: o miolo é a face de corte
    let (c, b) = pedir(&mut s, json!({ "vista": "base", "largura": 320, "altura": 240, "corte": { "eixo": "z", "posicao": 10, "lado": "maior" } }));
    assert!(contar_pixels(&b).0 > 3_000, "{c}");
    assert_eq!(c["corte"]["eixo"], "z");
    assert!(c.get("aviso").is_none(), "{c}");
    // plano fora da peça: avisa, e a resposta diz onde a peça está
    let (f, _) = pedir(&mut s, json!({ "corte": { "eixo": "z", "posicao": 500 } }));
    assert!(f["aviso"].as_str().unwrap().contains("fora da peça"), "{f}");
    // quatro vistas em uma imagem
    let (_, b) = pedir(&mut s, json!({ "vista": "todas", "largura": 400, "altura": 300 }));
    assert_eq!(contar_pixels(&b).2, (400, 300));
    // grava o arquivo e não troca outro sem pedir
    let g = s.executar("ver", &json!({ "saida": "peca.png", "largura": 200, "altura": 150 })).unwrap();
    assert_eq!(g["arquivo"], "peca.png");
    assert!(g.get("imagem_base64").is_none());
    assert_eq!(&std::fs::read(d.join("peca.png")).unwrap()[..4], b"\x89PNG");
    let e = s.executar("ver", &json!({ "saida": "peca.png" })).unwrap_err();
    assert_eq!(e.codigo, "ja_existe", "{e}");
    // pedidos errados
    assert_eq!(s.executar("ver", &json!({ "vista": "diagonal" })).unwrap_err().codigo, "argumento");
    assert_eq!(s.executar("ver", &json!({ "largura": 10 })).unwrap_err().codigo, "argumento");
    assert_eq!(s.executar("ver", &json!({ "corte": { "eixo": "w", "posicao": 1 } })).unwrap_err().codigo, "argumento");
}

#[test]
fn salvar_abrir_e_exportar_mantem_a_peca() {
    precisa_kernel!();
    let d = pasta_limpa("ida");
    let mut s = sessao(&d);
    bloco(&mut s);
    s.executar("salvar", &json!({ "arquivo": "b.fkx" }))
        .unwrap();
    s.executar("salvar", &json!({ "arquivo": "b.fkp" }))
        .unwrap();
    s.executar("exportar", &json!({ "arquivo": "b.step" }))
        .unwrap();
    for a in ["b.fkx", "b.fkp"] {
        let mut o = sessao(&d);
        let r = o.executar("abrir", &json!({ "arquivo": a })).unwrap();
        assert_eq!(r["editavel"], true, "{a}: {r}");
        assert!((volume(&mut o) - 24000.).abs() < 1e-6, "{a}");
    }
    assert_eq!(
        s.executar("exportar", &json!({ "arquivo": "b.step" }))
            .unwrap_err()
            .codigo,
        "ja_existe"
    );
    let t = s
        .executar(
            "topologia",
            &json!({ "arquivo": "b.step", "secoes": ["massa"] }),
        )
        .unwrap();
    assert!(
        (t["massa"]["volume"].as_f64().unwrap() - 24000.).abs() < 1e-3,
        "{t}"
    );
}

#[test]
fn peca_com_comando_fora_do_formato_mede_mas_nao_recebe_item() {
    precisa_kernel!();
    let d = pasta_limpa("fora");
    let mut doc = Document::new("com fora");
    doc.add_param("L", "5", Unit::Comprimento).unwrap();
    let sk = sk_rect(
        &mut doc,
        PlaneRef::Origin(OriginPlane::XY),
        [0., 0.],
        [10., 10.],
    );
    doc.add_feature(ext(sk, BoolOp::Add, "L"));
    let topo = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "L");
    let centro = sk_pontos(&mut doc, PlaneRef::Datum(topo), &[[5., 5.]]);
    // furo cônico não existe no `.fkx` versão 1: é o caso de peça que não cabe na árvore
    let FeatureKind::Hole(mut furo) = furo_passante(centro, "3") else {
        unreachable!()
    };
    furo.taper = "2".into();
    doc.add_feature(FeatureKind::Hole(furo));
    std::fs::write(d.join("x.fkp"), doc.to_json()).unwrap();
    let mut s = sessao(&d);
    let r = s.executar("abrir", &json!({ "arquivo": "x.fkp" })).unwrap();
    assert_eq!(r["editavel"], false, "{r}");
    assert!(!r["fora"].as_array().unwrap().is_empty(), "{r}");
    let antes = volume(&mut s);
    s.executar(
        "definir_parametro",
        &json!({ "nome": "L", "expressao": "10" }),
    )
    .unwrap();
    assert!(volume(&mut s) > antes);
    assert_eq!(
        s.executar("criar_esboco", &retangulo("10"))
            .unwrap_err()
            .codigo,
        "arvore_incompleta"
    );
    s.executar("exportar", &json!({ "arquivo": "x.step" }))
        .unwrap();
    assert!(d.join("x.step").is_file());
}

#[test]
fn features_infere_os_furos_da_placa_depois_do_step() {
    precisa_kernel!();
    let d = pasta_limpa("features");
    final_shape(&placa(20.))
        .unwrap()
        .export_step(d.join("p.step").to_str().unwrap())
        .unwrap();
    let mut s = sessao(&d);
    let f = s
        .executar("features", &json!({ "arquivo": "p.step" }))
        .unwrap();
    assert!(f["aviso"].as_str().unwrap().contains("não é o histórico original"));
    assert_eq!(f["contagem"]["furos_passantes"].as_u64().unwrap() + f["contagem"]["furos_cegos"].as_u64().unwrap(), 2, "{f}");
    for x in f["features"].as_array().unwrap() {
        assert!((x["diametro"].as_f64().unwrap() - 6.).abs() < 1e-6, "{x}");
    }
}

#[test]
fn topologia_acha_planos_cilindros_e_furos_depois_do_step() {
    precisa_kernel!();
    let d = pasta_limpa("topologia");
    final_shape(&placa(20.))
        .unwrap()
        .export_step(d.join("p.step").to_str().unwrap())
        .unwrap();
    let mut s = sessao(&d);
    let t = s
        .executar("topologia", &json!({ "arquivo": "p.step" }))
        .unwrap();
    assert_eq!(t["contagem"]["furos"], 2, "{}", t["contagem"]);
    assert_eq!(t["contagem"]["planos"], 6, "{}", t["contagem"]);
    for f in t["furos"].as_array().unwrap() {
        assert!((f["diametro"].as_f64().unwrap() - 6.).abs() < 1e-6, "{f}");
        assert!(
            (f["comprimento"].as_f64().unwrap() - 8.).abs() < 1e-6,
            "{f}"
        );
    }
    assert_eq!(t["planos"].as_array().unwrap().len(), 6);
    assert!(
        t["cilindros"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["interno"] == true),
        "{}",
        t["cilindros"]
    );
    for i in 0..3 {
        assert!(
            (t["caixa"]["tamanho"][i].as_f64().unwrap() - [100., 60., 8.][i]).abs() < 1e-5,
            "{}",
            t["caixa"]
        );
    }
    // a normal da face do topo aponta para fora, e a aresta traz a fórmula
    let topo = t["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["tipo"] == "plano" && f["ponto"][2].as_f64().unwrap() > 7.9)
        .unwrap();
    assert!(
        (topo["direcao"][2].as_f64().unwrap() - 1.).abs() < 1e-9,
        "{topo}"
    );
    let arestas = t["arestas"].as_array().unwrap();
    assert!(
        arestas
            .iter()
            .any(|a| a["tipo"] == "reta" && a["a"].is_array()),
        "sem reta"
    );
    assert!(
        arestas
            .iter()
            .any(|a| matches!(a["tipo"].as_str(), Some("circulo" | "arco"))
                && (a["raio"].as_f64().unwrap() - 3.).abs() < 1e-6),
        "sem círculo"
    );
    // filtro e corte
    let f = s
        .executar(
            "topologia",
            &json!({ "arquivo": "p.step", "secoes": ["faces"], "limite": 2 }),
        )
        .unwrap();
    assert_eq!(f["faces"].as_array().unwrap().len(), 2);
    assert_eq!(f["cortado"], true);
    assert!(f["caixa"].is_null());
    assert_eq!(
        s.executar(
            "topologia",
            &json!({ "arquivo": "p.step", "secoes": ["tudo"] })
        )
        .unwrap_err()
        .codigo,
        "argumento"
    );
}

#[test]
fn furo_espelhado_continua_furo_e_cilindro_externo_nao_e() {
    precisa_kernel!();
    // o espelho inverte o sistema de eixos do cilindro: é aqui que o sinal da orientação engana
    let forma = final_shape(&placa(20.)).unwrap();
    let espelhada = forma
        .transformed(Xform::Mirror([0.; 3], [1., 0., 0.]))
        .unwrap();
    assert_eq!(analisar(&espelhada).unwrap().furos.len(), 2);
    let mut d = Document::new("pino");
    d.add_feature(primitiva(PrimKind::Cilindro, "10", "30", "0", "0"));
    let pino = analisar(&final_shape(&d).unwrap()).unwrap();
    assert_eq!(pino.furos.len(), 0);
    assert_eq!(pino.cilindros.len(), 1);
    assert!(!pino.cilindros[0].interno);
    // canto arredondado por dentro é cilindro interno, mas não fecha a volta: não é furo
    let mut d = Document::new("rasgo");
    let s = sk_rect(
        &mut d,
        PlaneRef::Origin(OriginPlane::XY),
        [-20., -20.],
        [20., 20.],
    );
    d.add_feature(ext(s, BoolOp::Add, "10"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
    let b = sk_rect(&mut d, PlaneRef::Datum(topo), [-10., -10.], [10., 10.]);
    d.add_feature(ext(b, BoolOp::Cut, "20"));
    d.add_feature(filete(
        "2",
        vec![
            [10., 10., 5.],
            [-10., 10., 5.],
            [-10., -10., 5.],
            [10., -10., 5.],
        ],
        vec![None; 4],
    ));
    let furo = analisar(&final_shape(&d).unwrap()).unwrap();
    assert_eq!(furo.furos.len(), 0, "cantos viraram furo");
    assert!(
        furo.cilindros.iter().filter(|c| c.interno).count() >= 4,
        "{:?}",
        furo.cilindros
    );
}

#[test]
fn checador_aprova_igual_em_1e6_e_aponta_o_furo_deslocado() {
    precisa_kernel!();
    let a = analisar(&final_shape(&placa(20.)).unwrap()).unwrap();
    let b = analisar(&final_shape(&placa(20.)).unwrap()).unwrap();
    let justas = Tolerancias {
        volume: 1e-6,
        area: 1e-6,
        centroide: 1e-6,
        caixa: 1e-6,
        inercia: 1e-6,
        distancia: 1e-6,
        ..Default::default()
    };
    let r = comparar(&a, &b, &justas);
    assert_eq!(r["aprovado"], true, "{r}");

    let deslocada = analisar(&final_shape(&placa(20.5)).unwrap()).unwrap();
    let r = comparar(&a, &deslocada, &Tolerancias::default());
    assert_eq!(r["aprovado"], false, "{r}");
    let d = &r["furos"]["deslocados"][0];
    assert!(
        (d["desvio"].as_f64().unwrap() - 0.5).abs() < 1e-6,
        "{}",
        r["furos"]
    );
    assert!(
        (d["alvo"]["inicio"][0].as_f64().unwrap() - 20.).abs() < 1e-6,
        "{d}"
    );
    assert!(
        (d["modelo"]["inicio"][0].as_f64().unwrap() - 20.5).abs() < 1e-6,
        "{d}"
    );
    assert_eq!(r["furos"]["faltam"].as_array().unwrap().len(), 0);
}

#[test]
fn checador_conta_os_furos_por_diametro_nos_dois_corpos() {
    precisa_kernel!();
    let completa = analisar(&final_shape(&placa(20.)).unwrap()).unwrap();
    let mut lisa = Document::new("lisa");
    let base = sk_rect(&mut lisa, PlaneRef::Origin(OriginPlane::XY), [-50., -30.], [50., 30.]);
    lisa.add_feature(ext(base, BoolOp::Add, "8"));
    let lisa = analisar(&final_shape(&lisa).unwrap()).unwrap();
    let r = comparar(&completa, &lisa, &Tolerancias::default());
    let linhas = r["furos"]["por_diametro"].as_array().unwrap();
    assert!(!linhas.is_empty(), "{}", r["furos"]);
    let total_alvo: u64 = linhas.iter().map(|l| l["alvo"].as_u64().unwrap()).sum();
    assert_eq!(total_alvo as usize, completa.furos.len());
    assert!(linhas.iter().all(|l| l["modelo"] == 0 && l["ok"] == false), "{}", r["furos"]);
    let r = comparar(&completa, &completa, &Tolerancias::default());
    assert!(r["furos"]["por_diametro"].as_array().unwrap().iter().all(|l| l["ok"] == true));
}

#[test]
fn checador_acha_furo_que_falta_sobra_e_superficie_longe() {
    precisa_kernel!();
    let completa = analisar(&final_shape(&placa(20.)).unwrap()).unwrap();
    let mut lisa = Document::new("lisa");
    let base = sk_rect(
        &mut lisa,
        PlaneRef::Origin(OriginPlane::XY),
        [-50., -30.],
        [50., 30.],
    );
    lisa.add_feature(ext(base, BoolOp::Add, "8"));
    let lisa = analisar(&final_shape(&lisa).unwrap()).unwrap();
    let t = Tolerancias::default();
    let r = comparar(&completa, &lisa, &t);
    assert_eq!(
        r["furos"]["faltam"].as_array().unwrap().len(),
        2,
        "{}",
        r["furos"]
    );
    let r = comparar(&lisa, &completa, &t);
    assert_eq!(
        r["furos"]["sobram"].as_array().unwrap().len(),
        2,
        "{}",
        r["furos"]
    );

    // a medida bate em volume, mas a peça está em outro lugar
    let mut outra = Document::new("outra");
    let b = sk_rect(
        &mut outra,
        PlaneRef::Origin(OriginPlane::XY),
        [-40., -30.],
        [60., 30.],
    );
    outra.add_feature(ext(b, BoolOp::Add, "8"));
    let r = comparar(&lisa, &analisar(&final_shape(&outra).unwrap()).unwrap(), &t);
    assert_eq!(r["aprovado"], false);
    assert_eq!(r["distancia"]["ok"], false, "{}", r["distancia"]);
    assert!(
        (r["distancia"]["maxima"].as_f64().unwrap() - 10.).abs() < 0.5,
        "{}",
        r["distancia"]
    );
    assert!(
        r["medidas"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["nome"] == "volume")
            .unwrap()["ok"]
            == true
    );

    assert_eq!(
        Tolerancias::de_json(&json!({ "volume": -1 }))
            .unwrap_err()
            .codigo,
        "argumento"
    );
    assert_eq!(
        Tolerancias::de_json(&json!({ "inventada": 1 }))
            .unwrap_err()
            .codigo,
        "argumento"
    );
}

#[test]
fn registro_refaz_a_peca_e_recusa_comando_que_nao_e_de_modelo() {
    precisa_kernel!();
    let d = pasta_limpa("registro");
    let mut s = Sessao::nova(Pasta::nova(&d).unwrap(), Isolamento::Local).com_registro();
    bloco(&mut s);
    s.executar("filetar", &json!({ "raio": "500", "arestas": "todas" }))
        .unwrap();
    s.executar("desfazer", &json!({})).unwrap();
    s.executar("salvar", &json!({ "arquivo": "b.fkx" }))
        .unwrap();
    let r = s.executar("registro", &json!({})).unwrap();
    let arquivo = r["arquivo"].as_str().unwrap().to_string();
    assert_eq!(arquivo, "comandos/sessao-001.jsonl");
    assert!(
        !r["comandos"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["comando"] == "salvar"),
        "{r}"
    );

    let mut o = sessao(&d);
    o.executar("reproduzir", &json!({ "arquivo": arquivo }))
        .unwrap();
    assert_eq!(
        o.executar("arvore", &json!({})).unwrap()["pecas"],
        s.executar("arvore", &json!({})).unwrap()["pecas"]
    );

    let cabecalho = format!(
        "{}\n",
        json!({ "formato": "fenkra-comandos", "format_version": API_VERSAO })
    );
    std::fs::write(
        d.join("mau.jsonl"),
        format!(
            "{cabecalho}{}\n",
            json!({ "comando": "exportar", "args": { "arquivo": "x.step" } })
        ),
    )
    .unwrap();
    assert_eq!(
        o.executar("reproduzir", &json!({ "arquivo": "mau.jsonl" }))
            .unwrap_err()
            .codigo,
        "formato"
    );
    assert!(!d.join("x.step").exists());
    std::fs::write(
        d.join("velho.jsonl"),
        format!(
            "{}\n",
            json!({ "formato": "fenkra-comandos", "format_version": 99 })
        ),
    )
    .unwrap();
    assert_eq!(
        o.executar("reproduzir", &json!({ "arquivo": "velho.jsonl" }))
            .unwrap_err()
            .codigo,
        "formato"
    );
}

#[test]
fn queda_do_leitor_nao_derruba_a_sessao() {
    let d = pasta_limpa("queda");
    let pedido = filho::pedido_de_queda();
    let e = filho::rodar(Path::new(CLI), pedido, Duration::from_secs(30)).unwrap_err();
    assert_eq!(e.codigo, "queda", "{e}");
    std::fs::write(d.join("ruim.step"), "ISO-10303-21;\nquebrado").unwrap();
    let mut s = Sessao::nova(Pasta::nova(&d).unwrap(), Isolamento::Processo(CLI.into()));
    assert!(s
        .executar("topologia", &json!({ "arquivo": "ruim.step" }))
        .is_err());
    assert_eq!(s.executar("versao", &json!({})).unwrap()["api"], API_VERSAO);
}

// ------------------------------------------------------------------ linha de comando

fn cli(pasta: &Path, args: &[&str]) -> (i32, Value) {
    let o = Command::new(CLI)
        .arg("--pasta")
        .arg(pasta)
        .args(args)
        .output()
        .unwrap();
    let texto = String::from_utf8_lossy(&o.stdout);
    let v = serde_json::from_str(texto.trim())
        .unwrap_or_else(|e| panic!("{args:?}: saída não é um JSON só ({e}): {texto:?}"));
    (o.status.code().unwrap(), v)
}

#[test]
fn linha_de_comando_devolve_o_contrato_estavel() {
    precisa_kernel!();
    let d = pasta_limpa("linha");
    let mut s = sessao(&d);
    bloco(&mut s);
    s.executar("salvar", &json!({ "arquivo": "b.fkx" }))
        .unwrap();

    let (c, v) = cli(&d, &["versao"]);
    assert_eq!(
        (c, v["api"].as_u64(), v["ok"].as_bool()),
        (0, Some(API_VERSAO as u64), Some(true)),
        "{v}"
    );
    let (c, v) = cli(&d, &["recalcular", "b.fkx"]);
    assert_eq!(c, 0, "{v}");
    let r = &v["resultado"];
    for chave in [
        "status", "itens", "corpos", "volume", "area", "faces", "caixa",
    ] {
        assert!(!r[chave].is_null(), "falta {chave}: {r}");
    }
    assert_eq!(
        cli(&d, &["parametros", "b.fkx"]).1["resultado"]["parametros"][0]["valor"],
        40.0
    );

    // o script clássico: abre, muda parâmetro, exporta
    let (c, v) = cli(&d, &["definir", "b.fkx", "L", "80", "--saida", "c.fkx"]);
    assert_eq!(c, 0, "{v}");
    let (c, v) = cli(&d, &["exportar", "c.fkx", "c.step"]);
    assert_eq!(c, 0, "{v}");
    for (comando, chave) in [
        ("faces", "faces"),
        ("cilindros", "cilindros"),
        ("planos", "planos"),
        ("caixa", "caixa"),
    ] {
        let (c, v) = cli(&d, &[comando, "c.step"]);
        assert_eq!(c, 0, "{comando}: {v}");
        assert!(!v["resultado"][chave].is_null(), "{comando}: {v}");
    }
    assert!(
        (cli(&d, &["caixa", "c.step"]).1["resultado"]["caixa"]["tamanho"][0]
            .as_f64()
            .unwrap()
            - 80.)
            .abs()
            < 1e-5
    );

    let (c, v) = cli(&d, &["comparar", "c.step", "b.fkx"]);
    assert_eq!(c, 0, "{v}");
    assert_eq!(v["resultado"]["aprovado"], false, "{v}");

    // erro: código 1, JSON com `erro`, e nada sobrescrito
    let (c, v) = cli(&d, &["exportar", "c.fkx", "c.step"]);
    assert_eq!(
        (c, v["ok"].as_bool(), v["erro"]["codigo"].as_str()),
        (1, Some(false), Some("ja_existe")),
        "{v}"
    );
    assert_eq!(
        cli(&d, &["exportar", "c.fkx", "c.step", "--sobrescrever"]).0,
        0
    );
    let (c, v) = cli(&d, &["medir", "../fora.fkx"]);
    assert_eq!(
        (c, v["erro"]["codigo"].as_str()),
        (1, Some("caminho_fora")),
        "{v}"
    );
    let o = Command::new(CLI).args(["inventado"]).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn modo_servir_responde_uma_linha_por_pedido_com_o_id() {
    use std::io::{BufRead, BufReader, Write};
    let d = pasta_limpa("servir");
    let mut p = Command::new(CLI)
        .arg("--pasta")
        .arg(&d)
        .arg("servir")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut e = p.stdin.take().unwrap();
    let mut s = BufReader::new(p.stdout.take().unwrap());
    let mut linha = |texto: &str| {
        writeln!(e, "{texto}").unwrap();
        e.flush().unwrap();
        let mut l = String::new();
        s.read_line(&mut l).unwrap();
        serde_json::from_str::<Value>(&l).unwrap()
    };
    let r = linha(r#"{"id": 7, "comando": "versao"}"#);
    assert_eq!(
        (r["id"].as_u64(), r["ok"].as_bool()),
        (Some(7), Some(true)),
        "{r}"
    );
    let r = linha(r#"{"id": "x", "comando": "inventado"}"#);
    assert_eq!(
        (r["id"].as_str(), r["erro"]["codigo"].as_str()),
        (Some("x"), Some("comando_desconhecido")),
        "{r}"
    );
    let r = linha("isto não é json");
    assert_eq!(r["erro"]["codigo"], "argumento", "{r}");
    // o PowerShell manda marca de ordem de bytes na primeira linha de um tubo
    let r = linha("\u{feff}{\"id\": 8, \"comando\": \"versao\"}");
    assert_eq!(
        (r["id"].as_u64(), r["ok"].as_bool()),
        (Some(8), Some(true)),
        "{r}"
    );
    drop(e);
    assert!(p.wait().unwrap().success());
}

#[test]
fn listas_de_comandos_ficam_coerentes() {
    for c in COMANDOS_DE_MODELO {
        assert!(COMANDOS.contains(&c), "{c}");
    }
    assert!(!COMANDOS
        .iter()
        .any(|c| ["macro", "script", "executar", "shell", "eval"]
            .iter()
            .any(|p| c.contains(p))));
    let d = pasta_limpa("lista");
    let mut s = sessao(&d);
    for c in COMANDOS {
        // argumento vazio pode dar erro, mas nunca "não existe": a lista e o despacho são a mesma coisa
        if let Err(e) = s.executar(c, &json!({})) {
            assert_ne!(e.codigo, "comando_desconhecido", "{c}");
        }
    }
}

// ------------------------------------------------------------------ API Python

#[test]
fn api_python_abre_muda_parametro_e_exporta() {
    precisa_kernel!();
    let Some(python) = ["python", "python3"].into_iter().find(|p| {
        Command::new(p)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }) else {
        eprintln!("python ausente: teste da API Python pulado");
        return;
    };
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    let o = Command::new(python)
        .args(["-I", "-B"])
        .arg(raiz.join("tests/test_fenkra.py"))
        .env("FENKRA_CLI", CLI)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}

/// B237: cada peça do corpus, montada duas vezes, passa no checador com tudo em 1e-6 relativo; e a
/// mesma peça depois de ida e volta por STEP passa nas medidas de massa e nos furos em 1e-6.
#[test]
fn checador_aprova_as_pecas_do_corpus_iguais_em_1e6() {
    precisa_kernel!();
    let justas = Tolerancias {
        volume: 1e-6,
        area: 1e-6,
        centroide: 1e-6,
        caixa: 1e-6,
        inercia: 1e-6,
        distancia: 1e-6,
        ..Default::default()
    };
    // o gravador de STEP não aceita acento no caminho: pasta própria, só ASCII
    let dir = std::env::temp_dir().join(format!("fenkra-b237-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (i, (a, b)) in common::corpus::pecas().iter().zip(common::corpus::pecas().iter()).enumerate() {
        let sa = final_shape(a).unwrap_or_else(|| panic!("{}", a.name));
        let ra = analisar(&sa).unwrap();
        let r = comparar(&ra, &analisar(&final_shape(b).unwrap()).unwrap(), &justas);
        assert_eq!(r["aprovado"], true, "{}: {r}", a.name);
        // ida e volta por STEP: a geometria é a mesma, só a malha pode mudar
        let arq = dir.join(format!("peca{i:02}.step"));
        sa.export_step(&arq.to_string_lossy()).unwrap_or_else(|e| panic!("{}: {e:?} {}", a.name, arq.display()));
        let volta = fenkra_kernel::Shape::import_step(&arq.to_string_lossy()).unwrap();
        let r = comparar(&ra, &analisar(&volta).unwrap(), &justas);
        for m in r["medidas"].as_array().unwrap() {
            if m["nome"] != "caixa" {
                assert_eq!(m["ok"], true, "{} depois do STEP: {m}", a.name);
            }
        }
        assert_eq!(r["furos"]["ok"], true, "{} depois do STEP: {}", a.name, r["furos"]);
    }
}
