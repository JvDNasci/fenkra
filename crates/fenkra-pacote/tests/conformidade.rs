// SPDX-License-Identifier: LGPL-2.1-or-later
//! Conformidade do pacote de visualização (B181): cinco pacotes de referência abertos em cada leitor.
//!
//! Leitores: o de referência em Rust (`Pacote::ler` e `validar`) e o da página (`visualizador.html`,
//! rodado pelo Node com `tests/leitor_web.mjs`). Os dois têm de ver a mesma árvore, as mesmas malhas
//! (soma de cada acessor depois de decodificar o meshopt), os mesmos passos e balões. Os pacotes de
//! referência ficam em `tests/referencia/` e são gravados de novo com `FENKRA_FKV_REFERENCIA=1`; abrir
//! os gravados garante que um leitor novo continua lendo o que já foi publicado.

use fenkra_montagem::display::{DisplayState, Explosao, ModoPasso, PassoExplosao};
use fenkra_montagem::{Assembly, NodeKind, Placement};
use fenkra_pacote::{de_arquivo, de_desenho, de_montagem, de_peca, meshopt, Opcoes, Pacote};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn pasta(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra_conf_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn referencia() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/referencia")
}

/// Montagem com grupo, subconjunto, estado de exibição e explosão de dois passos.
fn montagem(dir: &Path) -> PathBuf {
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut sub = Assembly::new("Subconjunto");
    sub.add_component("Placa de baixo", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
    std::fs::write(dir.join("sub.fka"), sub.to_json()).unwrap();
    let mut a = Assembly::new("Conjunto com passos");
    a.props.pn = "CJ-0100".into();
    let g = a.add_group("Placas de cima", None);
    let p1 = a.add_component("Placa 1", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [0., 0., 40.], ..Default::default() }, Some(g));
    a.add_component("Placa 2", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [150., 0., 40.], ..Default::default() }, Some(g));
    let s = a.add_component("Base", NodeKind::SubAssembly { path: "sub.fka".into() }, Placement::default(), None);
    a.display_states.push(DisplayState { explosao: Default::default(), name: "Só a base".into(), hidden: vec![g], colors: vec![], offsets: vec![], camera: None, linhas: vec![] });
    let mut e = Explosao::nova("Explosão 1");
    e.passos.push(PassoExplosao { nome: "Tirar as placas".into(), comps: vec![g], modo: ModoPasso::Linear { dir: [0., 0., 1.], dist: 80. }, suprimido: false });
    e.passos.push(PassoExplosao { nome: "Afastar a primeira".into(), comps: vec![p1], modo: ModoPasso::Linear { dir: [-1., 0., 0.], dist: 50. }, suprimido: false });
    e.passos.push(PassoExplosao { nome: "Baixar a base".into(), comps: vec![s], modo: ModoPasso::Linear { dir: [0., 0., -1.], dist: 30. }, suprimido: false });
    a.explosoes.push(e);
    let f = dir.join("conjunto.fka");
    std::fs::write(&f, a.to_json()).unwrap();
    f
}

fn desenho(dir: &Path) -> fenkra_desenho::Drawing {
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let src = fenkra_desenho::Source { path: "placa.fkp".into(), display_state: String::new(), hidden: vec![] };
    let mut d = fenkra_desenho::Drawing::new("Placa", fenkra_desenho::Format::A4);
    let sh = d.sheets[0].id;
    d.add_standard_set(sh, src, [60., 200.], 0.5, 70.);
    d
}

/// Os cinco pacotes de referência, pelo nome do arquivo.
fn os_cinco(dir: &Path) -> Vec<(&'static str, Pacote)> {
    let peca = fenkra_solidos::example_document();
    let f = montagem(dir);
    let d = desenho(dir);
    vec![
        ("1-peca.fkv", de_peca(&peca, &Opcoes::default()).unwrap()),
        ("2-peca-decimada.fkv", de_peca(&peca, &Opcoes { decimar: Some(0.5), capturas: false, ..Default::default() }).unwrap()),
        ("3-montagem-passos.fkv", de_arquivo(&f, &Opcoes { analise: false, ..Default::default() }).unwrap()),
        ("4-montagem-meshopt.fkv", de_arquivo(&f, &Opcoes { analise: false, compactar: true, ..Default::default() }).unwrap()),
        ("5-desenho.fkv", de_desenho(&d, dir, &Opcoes::default()).unwrap()),
    ]
}

/// Resumo do leitor Rust, no mesmo formato do `leitor_web.mjs`.
fn resumo(p: &Pacote) -> Value {
    let m = &p.manifesto;
    let mut acessores = vec![];
    let mut compactado = false;
    if let Some(c) = m["cena"].as_str() {
        let glb = &p.arquivos.iter().find(|a| a.0 == c).unwrap().1;
        let cru = meshopt::descompactar(glb).unwrap();
        compactado = cru != *glb;
        let tj = u32::from_le_bytes([cru[12], cru[13], cru[14], cru[15]]) as usize;
        let j: Value = serde_json::from_slice(&cru[20..20 + tj]).unwrap();
        let bin = &cru[20 + tj + 8..];
        for a in j["accessors"].as_array().unwrap() {
            let v = &j["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
            let (tam, f): (usize, fn(&[u8]) -> f64) = match a["componentType"].as_u64().unwrap() {
                5126 => (4, |b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
                5120 => (1, |b| f64::from(b[0] as i8)),
                5123 => (2, |b| f64::from(u16::from_le_bytes([b[0], b[1]]))),
                _ => (4, |b| f64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
            };
            let nc = if a["type"] == "VEC3" { 3 } else { 1 };
            let passo = v["byteStride"].as_u64().map_or(tam * nc, |s| s as usize);
            let ini = v["byteOffset"].as_u64().unwrap_or(0) as usize;
            let mut soma = 0.;
            for i in 0..a["count"].as_u64().unwrap() as usize {
                for k in 0..nc {
                    soma += f(&bin[ini + i * passo + k * tam..]);
                }
            }
            acessores.push(soma);
        }
    }
    let ids = |l: &Value, k: &str| l.as_array().into_iter().flatten().map(|e| e[k].clone()).collect::<Vec<_>>();
    serde_json::json!({
        "formato": m["formato"], "versao": m["versao"], "tipo": m["tipo"], "nome": m["nome"],
        "arvore": ids(&m["arvore"], "id"), "malhas": m["malhas"].as_array().map_or(0, Vec::len),
        "passos": m["passos"].as_array().into_iter().flatten().map(|e| e["passos"].as_array().map_or(0, Vec::len)).collect::<Vec<_>>(),
        "baloes": ids(&m["baloes"], "item"), "folhas": m["folhas"].as_array().map_or(0, Vec::len),
        "compactado": compactado, "acessores": acessores,
    })
}

/// Abre os arquivos no leitor da página pelo Node. `None` quando o Node não está instalado.
fn leitor_web(arquivos: &[PathBuf]) -> Option<Vec<Value>> {
    let html = Path::new(env!("CARGO_MANIFEST_DIR")).join("web/visualizador.html");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/leitor_web.mjs");
    let saida = std::process::Command::new("node").arg(script).arg(html).args(arquivos).output().ok()?;
    assert!(saida.status.success(), "o leitor web falhou: {}", String::from_utf8_lossy(&saida.stderr));
    Some(serde_json::from_slice(&saida.stdout).expect("JSON do leitor web"))
}

/// Cada pacote nos dois leitores: o resumo tem de ser igual, campo a campo.
fn conferir_nos_dois(arquivos: &[PathBuf]) -> Option<Vec<Value>> {
    let web = leitor_web(arquivos);
    if web.is_none() {
        eprintln!("Node ausente: só o leitor Rust foi conferido");
    }
    for (k, a) in arquivos.iter().enumerate() {
        let p = Pacote::ler(&std::fs::read(a).unwrap()).unwrap();
        p.validar().unwrap_or_else(|e| panic!("{}: {e}", a.display()));
        let r = resumo(&p);
        if let Some(w) = &web {
            for campo in ["formato", "versao", "tipo", "nome", "arvore", "malhas", "passos", "baloes", "folhas", "compactado"] {
                assert_eq!(w[k][campo], r[campo], "{}: o campo '{campo}' difere entre os leitores", a.display());
            }
            // somas iguais em precisão dupla (o JSON do Node escreve inteiro sem ponto)
            let num = |v: &Value| v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect::<Vec<_>>();
            assert_eq!(num(&w[k]["acessores"]), num(&r["acessores"]), "{}: as malhas decodificadas diferem entre os leitores", a.display());
            assert!(!num(&r["acessores"]).is_empty() || r["folhas"].as_u64() > Some(0), "{}: pacote vazio", a.display());
        }
    }
    web
}

#[test]
fn os_cinco_pacotes_novos_abrem_igual_nos_dois_leitores_e_voltam_iguais() {
    let dir = pasta("cinco");
    let mut arquivos = vec![];
    for (nome, p) in os_cinco(&dir) {
        p.validar().unwrap();
        // ida e volta: gravar, ler e ver o mesmo manifesto e os mesmos arquivos
        let b = p.bytes().unwrap();
        let q = Pacote::ler(&b).unwrap();
        assert_eq!(q.manifesto, p.manifesto, "{nome}");
        assert_eq!(q.arquivos, p.arquivos, "{nome}");
        assert_eq!(q.bytes().unwrap(), b, "{nome}: regravar dá os mesmos bytes");
        let f = dir.join(nome);
        std::fs::write(&f, &b).unwrap();
        arquivos.push(f);
    }
    let web = conferir_nos_dois(&arquivos);
    // medição sem kernel: entre centros de furo vizinhos da placa (±48, ±24) dá 96 e 48, em 1e-6
    let medidas = |v: &Value| v.as_array().unwrap().iter().filter_map(Value::as_f64).collect::<Vec<_>>();
    if let Some(w) = web {
        let d = medidas(&w[0]["medidas"][0]);
        for alvo in [96., 48., (96f64 * 96. + 48. * 48.).sqrt()] {
            assert!(d.iter().any(|x| (x - alvo).abs() < 1e-6), "medida {alvo} não saiu dos pontos de captura: {d:?}");
        }
    }
    if std::env::var("FENKRA_FKV_REFERENCIA").is_ok() {
        std::fs::create_dir_all(referencia()).unwrap();
        for f in &arquivos {
            std::fs::copy(f, referencia().join(f.file_name().unwrap())).unwrap();
        }
    }
}

#[test]
fn pacotes_de_referencia_gravados_abrem_nos_dois_leitores() {
    let mut arquivos: Vec<PathBuf> = std::fs::read_dir(referencia()).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "fkv")).collect();
    arquivos.sort();
    assert_eq!(arquivos.len(), 5, "faltam pacotes de referência em {}", referencia().display());
    conferir_nos_dois(&arquivos);
}

#[test]
fn passos_trazem_deslocamento_lista_e_baloes_calculados() {
    let dir = pasta("passos");
    let f = montagem(&dir);
    let p = de_arquivo(&f, &Opcoes { analise: false, ..Default::default() }).unwrap();
    p.validar().unwrap();
    let m = &p.manifesto;
    let lista = m["lista"].as_array().unwrap();
    // uma linha só na lista: a placa, três vezes (duas no grupo, uma no subconjunto)
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0]["quantidade"], 3);
    let item = lista[0]["item"].clone();
    let baloes = m["baloes"].as_array().unwrap();
    assert_eq!(baloes.len(), 1, "um balão por item da lista");
    assert_eq!(baloes[0]["item"], item);
    let ex = &m["passos"][0];
    assert_eq!(ex["nome"], "Explosão 1");
    let passos = ex["passos"].as_array().unwrap();
    assert_eq!(passos.len(), 3);
    let arv = m["arvore"].as_array().unwrap();
    let id_de = |nome: &str| arv.iter().find(|e| e["nome"] == nome).unwrap()["id"].as_str().unwrap().to_string();
    let (g, p1, base) = (id_de("Placas de cima"), id_de("Placa 1"), id_de("Base"));
    // passo 1: o grupo sobe 80; a lista do passo tem as duas placas de cima
    assert_eq!(passos[0]["movidas"], serde_json::json!([g]));
    assert_eq!(passos[0]["deslocamentos"][0]["d"], serde_json::json!([0., 0., 80.]));
    assert_eq!(passos[0]["lista"][0]["quantidade"], 2);
    assert_eq!(passos[0]["lista"][0]["item"], item);
    // o balão do passo fica no centro da placa 1 (centro da placa em (0, 0, z) + 40 de montagem + 80)
    let b = &passos[0]["baloes"][0];
    assert_eq!(b["item"], item);
    let bz = b["ponto"][2].as_f64().unwrap();
    let centro_local = {
        let r = fenkra_solidos::recompute(&fenkra_solidos::example_document());
        let m = &r.bodies[0].mesh;
        m.positions.chunks_exact(3).map(|q| f64::from(q[2])).sum::<f64>() / (m.positions.len() / 3) as f64
    };
    assert!((bz - (centro_local + 40. + 80.)).abs() < 1e-3, "balão em z={bz}, esperado {}", centro_local + 120.);
    // passo 2: a placa 1 acumula os dois deslocamentos; o grupo continua com o do passo 1
    let d2: Vec<_> = passos[1]["deslocamentos"].as_array().unwrap().iter().map(|d| (d["id"].as_str().unwrap().to_string(), d["d"].clone())).collect();
    assert!(d2.contains(&(g.clone(), serde_json::json!([0., 0., 80.]))));
    assert!(d2.contains(&(p1.clone(), serde_json::json!([-50., 0., 0.]))));
    assert_eq!(passos[1]["lista"][0]["quantidade"], 1);
    // passo 3: o subconjunto desce; a lista do passo é a placa de dentro dele
    assert_eq!(passos[2]["movidas"], serde_json::json!([base]));
    assert_eq!(passos[2]["lista"][0]["quantidade"], 1);
    assert!(passos[2]["seta"]["direcao"][2].as_f64().unwrap() < -0.99);
    // as linhas de explosão saem da posição montada
    assert!(!ex["linhas"].as_array().unwrap().is_empty());
}

#[test]
fn meshopt_compacta_sem_perder_nada() {
    let dir = pasta("meshopt");
    let f = montagem(&dir);
    let a = Assembly::from_json(&std::fs::read_to_string(&f).unwrap()).unwrap();
    let cru = de_montagem(&a, &dir, &Opcoes { analise: false, ..Default::default() }).unwrap();
    let comp = de_montagem(&a, &dir, &Opcoes { analise: false, compactar: true, ..Default::default() }).unwrap();
    comp.validar().unwrap();
    let glb = |p: &Pacote| p.arquivos.iter().find(|x| x.0 == "cena.glb").unwrap().1.clone();
    let (g0, g1) = (glb(&cru), glb(&comp));
    let (j1, _) = (serde_json::from_slice::<Value>(&g1[20..20 + u32::from_le_bytes([g1[12], g1[13], g1[14], g1[15]]) as usize]).unwrap(), 0);
    assert!(j1["extensionsRequired"].as_array().unwrap().iter().any(|e| e == meshopt::EXTENSAO));
    assert_eq!(j1["buffers"][1]["extensions"][meshopt::EXTENSAO]["fallback"], true);
    assert!(g1.len() < g0.len(), "compactado {} contra {}", g1.len(), g0.len());
    // decodificado, cada vista tem os mesmos bytes da versão sem compressão
    assert_eq!(resumo(&cru)["acessores"], resumo(&comp)["acessores"]);
    let d = meshopt::descompactar(&g1).unwrap();
    let tj = |g: &[u8]| u32::from_le_bytes([g[12], g[13], g[14], g[15]]) as usize;
    assert_eq!(&d[20 + tj(&d) + 8..], &g0[20 + tj(&g0) + 8..], "binário decodificado igual ao original");
    // o manifesto não muda com a compressão
    assert_eq!(cru.manifesto, comp.manifesto);
    // e a miniatura do PLM lê a cena compactada
    let mini = fenkra_pacote::miniatura::de_pacote(&comp.bytes().unwrap(), 64).unwrap();
    assert!(mini.cobertura() > 0.05);
}
