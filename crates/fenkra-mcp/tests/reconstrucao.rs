// SPDX-License-Identifier: LGPL-2.1-or-later
//! B287: aceite do servidor. Um agente de roteiro fala com o binário real, por stdio, e só enxerga o
//! que as ferramentas devolvem. Ele não conhece a peça: lê o STEP, monta, compara e corrige pelo
//! relatório do checador, como faria um modelo. Serve de teste sem depender de modelo nenhum; o que
//! mede é se as ferramentas bastam para o trabalho, não a capacidade de um modelo em particular.

#[path = "../../fenkra-solidos/tests/common/mod.rs"]
mod common;

use common::*;
use fenkra_cli::pasta::Pasta;
use fenkra_cli::{Isolamento, Sessao};
use fenkra_core::*;
use fenkra_solidos::final_shape;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const ITERACOES: usize = 10;

macro_rules! precisa_kernel {
    () => {
        if !kernel_disponivel() {
            return;
        }
    };
}

// ------------------------------------------------------------------ as peças prismáticas do corpus (B53)

fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let s = sk_rect(
        &mut d,
        PlaneRef::Origin(OriginPlane::XY),
        [-20., -15.],
        [20., 15.],
    );
    d.add_feature(ext(s, BoolOp::Add, "20"));
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
    d.add_feature(FeatureKind::Fillet(EdgeOp {
        extra: Default::default(),
        value: "2".into(),
        edges: EdgeSel::AllOfBody,
        mode: EdgeMode::Constant,
        prop: Default::default(),
        keys: vec![],
    }));
    d
}

fn placa_com_grade() -> Document {
    let mut d = Document::new("placa com grade");
    let base = sk_rect(
        &mut d,
        PlaneRef::Origin(OriginPlane::XY),
        [-50., -30.],
        [50., 30.],
    );
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

fn flange() -> Document {
    let mut d = Document::new("flange");
    let base = sk_circulos(
        &mut d,
        PlaneRef::Origin(OriginPlane::XY),
        &[([0., 0.], 30.)],
    );
    d.add_feature(ext(base, BoolOp::Add, "8"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
    let centro = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    d.add_feature(furo_passante(centro, "20"));
    let furos = sk_pontos(
        &mut d,
        PlaneRef::Datum(topo),
        &[[20., 0.], [0., 20.], [-20., 0.], [0., -20.]],
    );
    d.add_feature(furo_passante(furos, "6"));
    d
}

fn bloco_l() -> Document {
    let mut d = Document::new("bloco L");
    let sk = sk_poligono(
        &mut d,
        PlaneRef::Origin(OriginPlane::XY),
        &[
            [0., 0.],
            [40., 0.],
            [40., 15.],
            [20., 15.],
            [20., 30.],
            [0., 30.],
        ],
    );
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d
}

// ------------------------------------------------------------------ cliente MCP

struct Cliente {
    filho: Child,
    entrada: ChildStdin,
    saida: BufReader<ChildStdout>,
    proximo: u64,
}

impl Drop for Cliente {
    fn drop(&mut self) {
        let _ = self.filho.kill();
        let _ = self.filho.wait();
    }
}

impl Cliente {
    fn iniciar(pasta: &Path, liberar: bool) -> Cliente {
        let mut c = Command::new(env!("CARGO_BIN_EXE_fenkra-mcp"));
        c.arg("--pasta")
            .arg(pasta)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if liberar {
            c.arg("--liberar-step");
        }
        let mut filho = c.spawn().expect("servidor não iniciou");
        let (entrada, saida) = (
            filho.stdin.take().unwrap(),
            BufReader::new(filho.stdout.take().unwrap()),
        );
        let mut k = Cliente {
            filho,
            entrada,
            saida,
            proximo: 1,
        };
        let r = k.pedido("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "roteiro", "version": "1" } }));
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18", "{r}");
        k.enviar(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        k
    }

    fn enviar(&mut self, v: &Value) {
        writeln!(self.entrada, "{v}").unwrap();
        self.entrada.flush().unwrap();
    }

    /// Cada linha da saída precisa ser JSON: uma estatística do kernel no meio quebraria aqui.
    fn linha(&mut self) -> Value {
        let mut l = String::new();
        self.saida.read_line(&mut l).unwrap();
        serde_json::from_str(&l).unwrap_or_else(|e| panic!("saída que não é JSON ({e}): {l:?}"))
    }

    fn pedido(&mut self, metodo: &str, params: Value) -> Value {
        let id = self.proximo;
        self.proximo += 1;
        self.enviar(&json!({ "jsonrpc": "2.0", "id": id, "method": metodo, "params": params }));
        let r = self.linha();
        assert_eq!(r["id"], id, "{r}");
        r
    }

    /// Chamada que pode falhar: devolve (dados, isError).
    fn tentar(&mut self, nome: &str, args: Value) -> (Value, bool) {
        let r = self.pedido("tools/call", json!({ "name": nome, "arguments": args }));
        let c = &r["result"];
        assert!(c.is_object(), "{nome}: {r}");
        let texto: Value = serde_json::from_str(c["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            texto, c["structuredContent"],
            "{nome}: texto e dados divergem"
        );
        (texto, c["isError"] == true)
    }

    fn chamar(&mut self, nome: &str, args: Value) -> Value {
        let (v, erro) = self.tentar(nome, args.clone());
        assert!(!erro, "{nome}({args}): {v}");
        v
    }

    fn recurso(&mut self, uri: &str) -> Value {
        let r = self.pedido("resources/read", json!({ "uri": uri }));
        serde_json::from_str(
            r["result"]["contents"][0]["text"]
                .as_str()
                .unwrap_or_else(|| panic!("{r}")),
        )
        .unwrap()
    }
}

fn pasta_limpa(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra-mcp-peças-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn gravar_alvo(d: &Document, pasta: &Path, nome: &str) {
    let forma = final_shape(d).expect("o alvo não gerou sólido");
    forma
        .export_step(pasta.join(nome).to_str().unwrap())
        .unwrap();
}

// ------------------------------------------------------------------ o agente de roteiro

fn xyz(v: &Value) -> [f64; 3] {
    [
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    ]
}

fn perto(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5)
}

fn pontas(a: &Value) -> ([f64; 3], [f64; 3]) {
    match a["tipo"].as_str().unwrap() {
        "reta" => (xyz(&a["a"]), xyz(&a["b"])),
        _ => (xyz(&a["inicio"]), xyz(&a["fim"])),
    }
}

/// Encadeia as arestas de uma face em laços fechados.
fn lacos<'a>(arestas: &[&'a Value]) -> Vec<Vec<&'a Value>> {
    let mut soltas: Vec<&Value> = arestas.to_vec();
    let mut saida = vec![];
    while let Some(primeira) = soltas.pop() {
        let mut laco = vec![primeira];
        if primeira["tipo"] != "circulo" {
            let (inicio, mut fim) = pontas(primeira);
            while !perto(inicio, fim) {
                let i = soltas
                    .iter()
                    .position(|a| {
                        let (p, q) = pontas(a);
                        perto(p, fim) || perto(q, fim)
                    })
                    .expect("contorno aberto");
                let a = soltas.remove(i);
                let (p, q) = pontas(a);
                fim = if perto(p, fim) { q } else { p };
                laco.push(a);
            }
        }
        saida.push(laco);
    }
    saida
}

fn extensao(laco: &[&Value]) -> f64 {
    let mut pontos = vec![];
    for a in laco {
        if a["tipo"] == "circulo" {
            return 2. * a["raio"].as_f64().unwrap();
        }
        let (p, q) = pontas(a);
        pontos.extend([p, q]);
    }
    let lado = |i: usize| {
        pontos.iter().map(|p| p[i]).fold(f64::MIN, f64::max)
            - pontos.iter().map(|p| p[i]).fold(f64::MAX, f64::min)
    };
    lado(0).max(lado(1))
}

/// Esboço de um laço, com uma cota em cada reta e em cada círculo.
struct Montador {
    pontos: Vec<Value>,
    curvas: Vec<Value>,
    cotas: Vec<Value>,
}

impl Montador {
    fn ponto(&mut self, p: [f64; 3]) -> String {
        if let Some(j) = self.pontos.iter().find(|j| {
            (j["x"].as_f64().unwrap() - p[0]).abs() < 1e-5
                && (j["y"].as_f64().unwrap() - p[1]).abs() < 1e-5
        }) {
            return j["id"].as_str().unwrap().to_string();
        }
        let id = format!("p{}", self.pontos.len());
        self.pontos.push(json!({ "id": id, "x": p[0], "y": p[1] }));
        id
    }

    fn laco(laco: &[&Value]) -> Montador {
        let mut m = Montador {
            pontos: vec![],
            curvas: vec![],
            cotas: vec![],
        };
        for (i, a) in laco.iter().enumerate() {
            let id = format!("c{i}");
            match a["tipo"].as_str().unwrap() {
                "reta" => {
                    let (p, q) = pontas(a);
                    let (x, y) = (m.ponto(p), m.ponto(q));
                    m.curvas
                        .push(json!({ "id": id, "geometria": { "tipo": "reta", "a": x, "b": y } }));
                    let comprimento = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
                    m.cotas.push(json!({ "tipo": "comprimento", "entidades": [id], "valor": comprimento.to_string() }));
                }
                "circulo" => {
                    let c = m.ponto(xyz(&a["centro"]));
                    let raio = a["raio"].as_f64().unwrap();
                    m.curvas.push(json!({ "id": id, "geometria": { "tipo": "circulo", "centro": c, "raio": raio } }));
                    m.cotas.push(json!({ "tipo": "diametro", "entidades": [id], "valor": (2. * raio).to_string() }));
                }
                "arco" => {
                    let (mut p, mut q) = pontas(a);
                    if a["normal"][2].as_f64().unwrap() < 0. {
                        std::mem::swap(&mut p, &mut q);
                    }
                    let (c, x, y) = (m.ponto(xyz(&a["centro"])), m.ponto(p), m.ponto(q));
                    m.curvas.push(json!({ "id": id, "geometria": { "tipo": "arco", "centro": c, "inicio": x, "fim": y } }));
                }
                t => panic!("aresta {t} fora do que o roteiro sabe esboçar"),
            }
        }
        m
    }

    fn enviar(self, c: &mut Cliente, plano: Value) -> String {
        let r = c.chamar("criar_esboco", json!({ "plano": plano, "pontos": self.pontos, "curvas": self.curvas, "restricoes": self.cotas }));
        assert_eq!(r["status"], "ok", "{r}");
        r["id"].as_str().unwrap().to_string()
    }
}

fn plano_em(c: &mut Cliente, z: f64) -> Value {
    if z.abs() < 1e-9 {
        return json!("XY");
    }
    let r = c.chamar("criar_plano", json!({ "base": "XY", "deslocamento": z }));
    json!({ "referencia": r["id"] })
}

fn aresta_da_face<'a>(t: &'a Value, face: &Value) -> Vec<&'a Value> {
    face["arestas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &t["arestas"][e.as_u64().unwrap() as usize])
        .collect()
}

fn face_plana(f: &Value, sinal: f64) -> bool {
    f["tipo"] == "plano" && (f["direcao"][2].as_f64().unwrap() - sinal).abs() < 1e-6
}

fn z_da(f: &Value) -> f64 {
    f["ponto"][2].as_f64().unwrap()
}

fn medida<'a>(c: &'a Value, nome: &str) -> &'a Value {
    c["medidas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["nome"] == nome)
        .unwrap()
}

/// Corta o contorno da face de `z` a partir do topo. O sentido do corte não vem de nenhuma tabela: se
/// o volume não mudou, o agente desfaz e tenta para o outro lado, como faria olhando o resultado.
fn cortar_bolso(c: &mut Cliente, t: &Value, face: &Value, topo: f64) {
    let arestas = aresta_da_face(t, face);
    let todos = lacos(&arestas);
    let externo = todos
        .iter()
        .max_by(|a, b| extensao(a).total_cmp(&extensao(b)))
        .unwrap();
    let plano = plano_em(c, topo);
    let sk = Montador::laco(externo).enviar(c, plano);
    let antes = c.chamar("recalcular", json!({}))["volume"]
        .as_f64()
        .unwrap();
    let profundidade = (topo - z_da(face)).to_string();
    for inverter in [false, true] {
        let r = c.chamar("extrudar", json!({ "esboco": sk, "operacao": "cortar", "termino": { "tipo": "cego", "distancia": profundidade }, "inverter": inverter }));
        if r["status"] == "ok" && r["volume"].as_f64().unwrap() < antes - 1e-6 {
            return;
        }
        c.chamar("desfazer", json!({}));
    }
    panic!("o corte não tirou material");
}

/// Reconstrói o alvo e devolve quantas comparações foram precisas.
fn reconstruir(c: &mut Cliente, alvo: &str, nome: &str) -> usize {
    let t = c.chamar("consultar_step", json!({ "arquivo": alvo }));
    let faces = t["faces"].as_array().unwrap();
    let (zmin, zmax) = (
        t["caixa"]["min"][2].as_f64().unwrap(),
        t["caixa"]["max"][2].as_f64().unwrap(),
    );
    let (lo, hi) = (xyz(&t["caixa"]["min"]), xyz(&t["caixa"]["max"]));
    c.chamar("nova_peca", json!({ "nome": nome }));
    c.chamar(
        "definir_parametro",
        json!({ "nome": "altura", "expressao": (zmax - zmin).to_string() }),
    );

    // canto arredondado por toda a volta: a base é a caixa, e o raio entra por filete
    let raio_do_filete = faces
        .iter()
        .find(|f| f["tipo"] == "esfera")
        .map(|f| f["raio"].as_f64().unwrap());
    let plano = plano_em(c, zmin);
    let sk = if raio_do_filete.is_some() {
        let quina = |x: f64, y: f64| [x, y, zmin];
        let lados = [
            (quina(lo[0], lo[1]), quina(hi[0], lo[1])),
            (quina(hi[0], lo[1]), quina(hi[0], hi[1])),
            (quina(hi[0], hi[1]), quina(lo[0], hi[1])),
            (quina(lo[0], hi[1]), quina(lo[0], lo[1])),
        ];
        let retas: Vec<Value> = lados
            .iter()
            .map(|(a, b)| json!({ "tipo": "reta", "a": a, "b": b }))
            .collect();
        Montador::laco(&retas.iter().collect::<Vec<_>>()).enviar(c, plano)
    } else {
        let base = faces
            .iter()
            .filter(|f| face_plana(f, -1.) && (z_da(f) - zmin).abs() < 1e-6)
            .max_by(|a, b| {
                a["area"]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&b["area"].as_f64().unwrap())
            })
            .expect("sem face de base");
        let arestas = aresta_da_face(&t, base);
        let todos = lacos(&arestas);
        Montador::laco(
            todos
                .iter()
                .max_by(|a, b| extensao(a).total_cmp(&extensao(b)))
                .unwrap(),
        )
        .enviar(c, plano)
    };
    let r = c.chamar("extrudar", json!({ "esboco": sk, "operacao": "unir", "termino": { "tipo": "cego", "distancia": "altura" } }));
    assert_eq!(r["status"], "ok", "{r}");

    let mut bolsos: Vec<&Value> = faces
        .iter()
        .filter(|f| face_plana(f, 1.) && z_da(f) < zmax - 1e-6 && raio_do_filete.is_none())
        .collect();
    for iteracao in 1..=ITERACOES {
        let cmp = c.chamar("comparar_alvo", json!({ "alvo": alvo }));
        if cmp["aprovado"] == true {
            return iteracao;
        }
        let faltam = cmp["furos"]["faltam"].as_array().unwrap();
        if !faltam.is_empty() {
            // um esboço por diâmetro, com um ponto em cada centro
            let mut diametros: Vec<f64> = faltam
                .iter()
                .map(|f| f["diametro"].as_f64().unwrap())
                .collect();
            diametros.dedup();
            for d in diametros {
                let centros: Vec<&Value> = faltam
                    .iter()
                    .filter(|f| f["diametro"].as_f64() == Some(d))
                    .collect();
                let pontos: Vec<Value> = centros.iter().enumerate().map(|(i, f)| json!({ "id": format!("p{i}"), "x": f["inicio"][0], "y": f["inicio"][1] })).collect();
                let curvas: Vec<Value> = (0..pontos.len()).map(|i| json!({ "id": format!("c{i}"), "geometria": { "tipo": "ponto", "ponto": format!("p{i}") } })).collect();
                let plano = plano_em(c, zmax);
                let r = c.chamar(
                    "criar_esboco",
                    json!({ "plano": plano, "pontos": pontos, "curvas": curvas }),
                );
                let r = c.chamar("furar", json!({ "esboco": r["id"], "diametro": d, "termino": { "tipo": "atravessar" } }));
                assert_eq!(r["status"], "ok", "{r}");
            }
            continue;
        }
        if let Some(raio) = raio_do_filete {
            let r = c.chamar("filetar", json!({ "raio": raio, "arestas": "todas" }));
            assert_eq!(r["status"], "ok", "{r}");
            continue;
        }
        if let Some(face) = bolsos.pop() {
            cortar_bolso(c, &t, face, zmax);
            continue;
        }
        panic!("{nome}: o roteiro não sabe corrigir: {cmp}");
    }
    panic!("{nome}: passou de {ITERACOES} iterações");
}

fn confere_reconstrucao(nome: &str, d: Document) {
    let pasta = pasta_limpa(nome);
    gravar_alvo(&d, &pasta, "alvo.step");
    let forma = final_shape(&d).unwrap();
    let mut c = Cliente::iniciar(&pasta, true);
    let iteracoes = reconstruir(&mut c, "alvo.step", nome);
    assert!(iteracoes <= ITERACOES);

    // o aceite do item, medido por fora do checador
    let m = c.chamar("medir", json!({}));
    let volume = m["massa"]["volume"].as_f64().unwrap();
    let alvo = forma.mass().volume;
    assert!(
        (volume - alvo).abs() / alvo < 0.005,
        "{nome}: volume {volume} contra {alvo}"
    );
    let cmp = c.chamar("comparar_alvo", json!({ "alvo": "alvo.step" }));
    assert_eq!(cmp["furos"]["ok"], true, "{nome}: {}", cmp["furos"]);
    assert!(medida(&cmp, "volume")["desvio"].as_f64().unwrap() < 0.005);
    let r = c.chamar("recalcular", json!({}));
    assert_eq!(r["status"], "ok", "{nome}: {r}");
    assert_eq!(r["corpos"], 1, "{nome}");
    println!(
        "{nome}: {iteracoes} iterações, volume {volume} (alvo {alvo}), {} itens",
        r["itens"].as_array().unwrap().len()
    );

    // o resultado é árvore de verdade, com cotas e parâmetro
    let arvore = c.recurso("fenkra://arvore");
    assert_eq!(
        arvore["pecas"][0]["parametros"][0]["nome"], "altura",
        "{nome}"
    );
    assert!(
        arvore["pecas"][0]["arvore"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["comando"]["tipo"] == "esboco"
                && !i["comando"]["restricoes"].as_array().unwrap().is_empty()),
        "{nome}: esboço sem cota"
    );
    c.chamar("salvar_peca", json!({ "arquivo": "reconstruida.fkx" }));

    // a sequência é o artefato: refeita sem servidor nem agente, dá a mesma peça
    let registro = c.recurso("fenkra://comandos");
    let arquivo = registro["arquivo"].as_str().unwrap().to_string();
    assert!(pasta.join(&arquivo).is_file(), "{nome}: {arquivo}");
    let mut s = Sessao::nova(Pasta::nova(&pasta).unwrap(), Isolamento::Local);
    s.executar("reproduzir", &json!({ "arquivo": arquivo }))
        .unwrap();
    let refeito = s.executar("medir", &json!({})).unwrap();
    assert_eq!(
        refeito["massa"], m["massa"],
        "{nome}: a sequência refeita mudou a peça"
    );
}

#[test]
fn reconstroi_caixa() {
    precisa_kernel!();
    confere_reconstrucao("caixa", caixa());
}

#[test]
fn reconstroi_caixa_com_bolso() {
    precisa_kernel!();
    confere_reconstrucao("bolso", caixa_com_bolso());
}

#[test]
fn reconstroi_placa_com_grade() {
    precisa_kernel!();
    confere_reconstrucao("grade", placa_com_grade());
}

#[test]
fn reconstroi_flange() {
    precisa_kernel!();
    confere_reconstrucao("flange", flange());
}

#[test]
fn reconstroi_bloco_l() {
    precisa_kernel!();
    confere_reconstrucao("bloco", bloco_l());
}

/// Fora do corpus: cantos verticais arredondados, para o contorno trazer arcos.
fn placa_de_cantos_redondos() -> Document {
    let mut d = caixa();
    d.add_feature(filete(
        "4",
        vec![
            [20., 15., 10.],
            [-20., 15., 10.],
            [-20., -15., 10.],
            [20., -15., 10.],
        ],
        vec![None; 4],
    ));
    d
}

#[test]
fn reconstroi_contorno_com_arcos() {
    precisa_kernel!();
    confere_reconstrucao("arcos", placa_de_cantos_redondos());
}

#[test]
fn reconstroi_caixa_arredondada_com_filete() {
    precisa_kernel!();
    confere_reconstrucao("arredondada", caixa_arredondada());
}

// ------------------------------------------------------------------ limites do servidor

#[test]
fn step_fica_preso_sem_o_aceite_da_pessoa() {
    precisa_kernel!();
    let pasta = pasta_limpa("aceite");
    gravar_alvo(&caixa(), &pasta, "alvo.step");
    let mut c = Cliente::iniciar(&pasta, false);
    for (nome, args) in [
        ("consultar_step", json!({ "arquivo": "alvo.step" })),
        ("comparar_alvo", json!({ "alvo": "alvo.step" })),
    ] {
        let (v, erro) = c.tentar(nome, args);
        assert!(erro, "{nome}");
        assert_eq!(v["codigo"], "step_bloqueado", "{v}");
        assert!(!v.to_string().contains("volume"), "{v}");
    }
    // o aceite não tem porta no protocolo, e nenhuma outra ferramenta lê o STEP
    let (v, erro) = c.tentar(
        "consultar_step",
        json!({ "arquivo": "alvo.step", "aceite": true, "liberar_step": true }),
    );
    assert!(erro && v["codigo"] == "step_bloqueado", "{v}");
    let (v, erro) = c.tentar("abrir_peca", json!({ "arquivo": "alvo.step" }));
    assert!(erro && v["codigo"] == "formato", "{v}");
    c.chamar("nova_peca", json!({}));
    let (v, erro) = c.tentar("topologia_modelo", json!({ "arquivo": "alvo.step" }));
    assert!(erro && v["codigo"] == "argumento", "{v}");
}

#[test]
fn caminho_fora_da_pasta_e_recusado_em_toda_ferramenta() {
    precisa_kernel!();
    let pasta = pasta_limpa("fora");
    let externa = pasta_limpa("fora-externa");
    gravar_alvo(&caixa(), &externa, "segredo.step");
    let mut c = Cliente::iniciar(&pasta, true);
    c.chamar("nova_peca", json!({}));
    let de_fora = externa.join("segredo.step").to_string_lossy().to_string();
    for caminho in [de_fora.as_str(), "../segredo.step", "..\\segredo.step"] {
        for (nome, args) in [
            ("consultar_step", json!({ "arquivo": caminho })),
            ("comparar_alvo", json!({ "alvo": caminho })),
        ] {
            let (v, erro) = c.tentar(nome, args);
            assert!(
                erro && v["codigo"] == "caminho_fora",
                "{nome} {caminho}: {v}"
            );
        }
    }
    let (v, erro) = c.tentar("salvar_peca", json!({ "arquivo": "../fuga.fkx" }));
    assert!(erro && v["codigo"] == "caminho_fora", "{v}");
    let (v, erro) = c.tentar(
        "salvar_peca",
        json!({ "arquivo": externa.join("fuga.fkx").to_string_lossy(), "sobrescrever": true }),
    );
    assert!(erro && v["codigo"] == "caminho_fora", "{v}");
    assert!(!externa.join("fuga.fkx").exists());
}

#[test]
fn nunca_sobrescreve_sem_pedir_e_a_saida_segue_limpa() {
    precisa_kernel!();
    let pasta = pasta_limpa("sobrescrever");
    let mut c = Cliente::iniciar(&pasta, true);
    c.chamar("nova_peca", json!({}));
    let k = c.chamar("criar_esboco", json!({
        "plano": "XY",
        "pontos": [{ "id": "a", "x": 0, "y": 0 }, { "id": "b", "x": 10, "y": 0 }, { "id": "c", "x": 10, "y": 10 }, { "id": "d", "x": 0, "y": 10 }],
        "curvas": [
            { "id": "1", "geometria": { "tipo": "reta", "a": "a", "b": "b" } }, { "id": "2", "geometria": { "tipo": "reta", "a": "b", "b": "c" } },
            { "id": "3", "geometria": { "tipo": "reta", "a": "c", "b": "d" } }, { "id": "4", "geometria": { "tipo": "reta", "a": "d", "b": "a" } },
        ],
    }));
    c.chamar("extrudar", json!({ "esboco": k["id"], "operacao": "unir", "termino": { "tipo": "cego", "distancia": "5" } }));
    // a gravação do kernel escreve estatística em stdout: `linha` falharia se ela chegasse ao cliente
    c.chamar("exportar_step", json!({ "arquivo": "a.step" }));
    let conteudo = std::fs::read(pasta.join("a.step")).unwrap();
    let (v, erro) = c.tentar("exportar_step", json!({ "arquivo": "a.step" }));
    assert!(erro && v["codigo"] == "ja_existe", "{v}");
    assert_eq!(std::fs::read(pasta.join("a.step")).unwrap(), conteudo);
    c.chamar(
        "exportar_step",
        json!({ "arquivo": "a.step", "sobrescrever": true }),
    );
    assert!(
        c.chamar("consultar_step", json!({ "arquivo": "a.step" }))["massa"]["volume"]
            .as_f64()
            .unwrap()
            > 499.
    );

    c.chamar("salvar_peca", json!({ "arquivo": "a.fkx" }));
    let (v, erro) = c.tentar("salvar_peca", json!({ "arquivo": "a.fkx" }));
    assert!(erro && v["codigo"] == "ja_existe", "{v}");
    // duas sessões seguidas não dividem o registro
    let primeiro = c.recurso("fenkra://comandos")["arquivo"].clone();
    drop(c);
    let mut c = Cliente::iniciar(&pasta, true);
    c.chamar("nova_peca", json!({}));
    assert_ne!(c.recurso("fenkra://comandos")["arquivo"], primeiro);
}

#[test]
fn step_que_nao_abre_nao_mata_o_servidor() {
    let pasta = pasta_limpa("lixo");
    std::fs::write(pasta.join("lixo.step"), "isto não é STEP").unwrap();
    let mut c = Cliente::iniciar(&pasta, true);
    let (v, erro) = c.tentar("consultar_step", json!({ "arquivo": "lixo.step" }));
    assert!(erro, "{v}");
    assert_eq!(c.pedido("ping", json!({}))["result"], json!({}));
}

#[test]
fn nenhuma_ferramenta_executa_codigo_nem_escapa_da_lista() {
    let pasta = pasta_limpa("lista");
    let mut c = Cliente::iniciar(&pasta, false);
    let r = c.pedido("tools/list", json!({}));
    let nomes: Vec<&str> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for exigida in [
        "consultar_step",
        "criar_esboco",
        "extrudar",
        "furar",
        "filetar",
        "recalcular",
        "medir",
        "comparar_alvo",
    ] {
        assert!(nomes.contains(&exigida), "{exigida}");
    }
    assert!(
        !nomes
            .iter()
            .any(|n| ["executar", "shell", "macro", "script", "reproduzir"]
                .iter()
                .any(|p| n.contains(p))),
        "{nomes:?}"
    );
    for t in r["result"]["tools"].as_array().unwrap() {
        assert_eq!(t["inputSchema"]["type"], "object", "{t}");
        assert_eq!(t["annotations"]["openWorldHint"], false, "{t}");
    }
    let r = c.pedido(
        "tools/call",
        json!({ "name": "reproduzir", "arguments": {} }),
    );
    assert_eq!(r["error"]["code"], -32602, "{r}");
}

// ------------------------------------------------------------------ protocolo

#[test]
fn protocolo_responde_o_que_a_especificacao_pede() {
    let pasta = pasta_limpa("protocolo");
    let mut c = Cliente::iniciar(&pasta, false);
    assert_eq!(c.pedido("ping", json!({}))["result"], json!({}));
    assert_eq!(
        c.pedido("metodo/inventado", json!({}))["error"]["code"],
        -32601
    );
    // versão desconhecida no aperto de mão cai na mais nova que temos
    let r = c.pedido("initialize", json!({ "protocolVersion": "1900-01-01" }));
    assert_eq!(r["result"]["protocolVersion"], "2025-11-25", "{r}");

    // pedido com versão própria: o erro lista o que existe, e o resultado leva `resultType`
    let meta = |v: &str| json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": v } });
    let r = c.pedido("tools/list", meta("1900-01-01"));
    assert_eq!(r["error"]["code"], -32022, "{r}");
    assert_eq!(r["error"]["data"]["requested"], "1900-01-01");
    assert!(
        r["error"]["data"]["supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "2026-07-28"),
        "{r}"
    );
    let r = c.pedido("tools/list", meta("2026-07-28"));
    assert_eq!(r["result"]["resultType"], "complete", "{r}");
    let r = c.pedido("server/discover", meta("2026-07-28"));
    assert!(
        r["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "2025-06-18"),
        "{r}"
    );
    assert_eq!(
        r["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], "fenkra",
        "{r}"
    );

    // marca de ordem de bytes no começo da linha não atrapalha
    c.entrada
        .write_all(b"\xEF\xBB\xBF{\"jsonrpc\":\"2.0\",\"id\":900,\"method\":\"ping\"}\n")
        .unwrap();
    c.entrada.flush().unwrap();
    assert_eq!(c.linha()["id"], 900);

    // JSON quebrado responde e o servidor continua
    writeln!(c.entrada, "{{quebrado").unwrap();
    c.entrada.flush().unwrap();
    assert_eq!(c.linha()["error"]["code"], -32700);
    writeln!(c.entrada, "[1, 2]").unwrap();
    c.entrada.flush().unwrap();
    assert_eq!(c.linha()["error"]["code"], -32600);

    // notificação não tem resposta: a próxima linha é a do ping
    c.enviar(&json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} }));
    assert_eq!(c.pedido("ping", json!({}))["result"], json!({}));

    let r = c.pedido("resources/list", json!({}));
    let uris: Vec<&str> = r["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["uri"].as_str().unwrap())
        .collect();
    assert_eq!(
        uris,
        ["fenkra://arvore", "fenkra://relatorio", "fenkra://comandos"]
    );
    assert_eq!(
        c.pedido("resources/read", json!({ "uri": "file:///etc/passwd" }))["error"]["code"],
        -32002
    );
    assert_eq!(c.recurso("fenkra://arvore")["codigo"], "sem_peca");
}

#[test]
fn entrada_fechada_encerra_o_servidor() {
    let pasta = pasta_limpa("fim");
    let mut p = Command::new(env!("CARGO_BIN_EXE_fenkra-mcp"))
        .arg("--pasta")
        .arg(&pasta)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    drop(p.stdin.take());
    let inicio = std::time::Instant::now();
    loop {
        if let Some(s) = p.try_wait().unwrap() {
            assert!(s.success(), "{s:?}");
            return;
        }
        assert!(inicio.elapsed().as_secs() < 20, "o servidor não encerrou");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

// ------------------------------------------------------------------ ondas do B420

fn retangulo(x0: f64, y0: f64, x1: f64, y1: f64) -> (Value, Value) {
    let pontos = json!([{ "id": "p0", "x": x0, "y": y0 }, { "id": "p1", "x": x1, "y": y0 }, { "id": "p2", "x": x1, "y": y1 }, { "id": "p3", "x": x0, "y": y1 }]);
    let curvas: Vec<Value> = (0..4).map(|i| json!({ "id": format!("c{i}"), "geometria": { "tipo": "reta", "a": format!("p{i}"), "b": format!("p{}", (i + 1) % 4) } })).collect();
    (pontos, json!(curvas))
}

/// B420: revolução, seleção semântica, chanfro, furo em padrão circular e espelho só com o que o
/// servidor oferece, conferindo volume e caixa; a sequência refeita sem servidor dá a mesma peça.
#[test]
fn ondas_revolucao_padrao_espelho_e_selecao_pelo_servidor() {
    precisa_kernel!();
    let pasta = pasta_limpa("ondas");
    let mut c = Cliente::iniciar(&pasta, false);
    let r = c.pedido("tools/list", json!({}));
    let nomes: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for n in ["revolucionar", "criar_padrao", "espelhar", "criar_casca", "criar_primitiva", "selecionar", "consultar_biblioteca", "listar_comandos"] {
        assert!(nomes.contains(&n), "{n}: {nomes:?}");
    }
    let d = c.chamar("listar_comandos", json!({ "busca": "padrão" }));
    assert!(d["ferramentas"].as_array().unwrap().iter().any(|t| t["nome"] == "criar_padrao"), "{d}");

    c.chamar("nova_peca", json!({ "nome": "anel" }));
    // anel: perfil de x 10 a 20 e z 0 a 30, girado em Z
    let (pontos, curvas) = retangulo(10., 0., 20., 30.);
    let e = c.chamar("criar_esboco", json!({ "id": "perfil", "plano": "XZ", "pontos": pontos, "curvas": curvas }));
    assert_eq!(e["status"], "ok", "{e}");
    let r = c.chamar("revolucionar", json!({ "id": "anel", "esboco": "perfil", "eixo": "Z", "angulo": 360, "operacao": "unir" }));
    assert_eq!(r["status"], "ok", "{r}");
    let pi = std::f64::consts::PI;
    let v_anel = pi * (400. - 100.) * 30.;
    assert!((r["volume"].as_f64().unwrap() - v_anel).abs() < 1., "{r}");

    // as duas bordas de cima, sem ler a topologia
    let s = c.chamar("selecionar", json!({ "lado": "topo", "tipo": "circulo" }));
    assert_eq!(s["quantidade"], 2, "{s}");
    let r = c.chamar("chanfrar", json!({ "distancia": 1, "arestas": s["pontos"] }));
    assert_eq!(r["status"], "ok", "{r}");
    let v_chanfro = r["volume"].as_f64().unwrap();
    // dois chanfros 1 × 1: cada um tira um triângulo de área 0,5 girado no raio do seu centroide
    let tirado = 0.5 * 2. * pi * (20. - 1. / 3.) + 0.5 * 2. * pi * (10. + 1. / 3.);
    assert!((v_anel - v_chanfro - tirado).abs() < 0.5, "{v_chanfro}");

    // furo Ø3 passante em r = 15, repetido 6 vezes em volta de Z
    let ponto = json!({ "id": "furos", "plano": "XY", "pontos": [{ "id": "p0", "x": 15., "y": 0. }], "curvas": [{ "id": "c0", "geometria": { "tipo": "ponto", "ponto": "p0" } }] });
    c.chamar("criar_esboco", ponto);
    let r = c.chamar("furar", json!({ "id": "furo1", "esboco": "furos", "diametro": 3, "termino": { "tipo": "atravessar" }, "inverter": true }));
    assert_eq!(r["status"], "ok", "{r}");
    let v_um_furo = r["volume"].as_f64().unwrap();
    assert!((v_chanfro - v_um_furo - pi * 1.5 * 1.5 * 30.).abs() < 1., "{r}");
    let r = c.chamar("criar_padrao", json!({ "forma": { "tipo": "circular", "eixo": "Z", "quantidade": 6, "angulo": 360 }, "fontes": ["furo1"] }));
    assert_eq!(r["status"], "ok", "{r}");
    let v_furos = r["volume"].as_f64().unwrap();
    assert!((v_chanfro - v_furos - 6. * pi * 1.5 * 1.5 * 30.).abs() < 1., "{v_furos}");
    let s = c.chamar("selecionar", json!({ "alvo": "faces", "tipo": "cilindro", "raio": 1.5 }));
    assert_eq!(s["quantidade"], 6, "seis paredes de furo: {s}");

    // espelho pelo XY: o anel dobra para baixo
    let r = c.chamar("espelhar", json!({ "plano": "XY" }));
    assert_eq!(r["status"], "ok", "{r}");
    let r = c.chamar("recalcular", json!({}));
    assert_eq!(r["status"], "ok", "{r}");
    assert!((r["caixa"]["min"][2].as_f64().unwrap() + 30.).abs() < 1e-3, "{r}");
    assert!((r["caixa"]["max"][2].as_f64().unwrap() - 30.).abs() < 1e-3, "{r}");
    assert!((r["volume"].as_f64().unwrap() - 2. * v_furos).abs() < 1., "{r}");
    let m = c.chamar("medir", json!({}));

    // biblioteca: há famílias, e a busca vazia não quebra
    let b = c.chamar("consultar_biblioteca", json!({ "busca": "" }));
    assert!(!b["familias"].as_array().unwrap().is_empty(), "{b}");

    // a sequência refeita sem servidor dá a mesma peça
    let registro = c.recurso("fenkra://comandos");
    let arquivo = registro["arquivo"].as_str().unwrap().to_string();
    drop(c);
    let mut s = Sessao::nova(Pasta::nova(&pasta).unwrap(), Isolamento::Local);
    s.executar("reproduzir", &json!({ "arquivo": arquivo })).unwrap();
    assert_eq!(s.executar("medir", &json!({})).unwrap()["massa"], m["massa"]);
}

/// B420: casca pela face de cima escolhida por sentido.
#[test]
fn casca_abre_a_face_escolhida_por_lado() {
    precisa_kernel!();
    let pasta = pasta_limpa("casca_ondas");
    let mut c = Cliente::iniciar(&pasta, false);
    c.chamar("nova_peca", json!({}));
    let (pontos, curvas) = retangulo(0., 0., 40., 20.);
    c.chamar("criar_esboco", json!({ "id": "base", "plano": "XY", "pontos": pontos, "curvas": curvas }));
    c.chamar("extrudar", json!({ "esboco": "base", "operacao": "unir", "termino": { "tipo": "cego", "distancia": "10" } }));
    let s = c.chamar("selecionar", json!({ "alvo": "faces", "lado": "topo" }));
    assert_eq!(s["quantidade"], 1, "{s}");
    let r = c.chamar("criar_casca", json!({ "faces": s["pontos"], "espessura": 2 }));
    assert_eq!(r["status"], "ok", "{r}");
    let v = r["volume"].as_f64().unwrap();
    assert!((v - (8000. - 36. * 16. * 8.)).abs() < 1., "{r}");
    let (e, erro) = c.tentar("criar_casca", json!({ "tipo": "padrao", "faces": [], "espessura": 2 }));
    assert!(erro, "tipo trocado passou: {e}");
}

// ------------------------------------------------------------------ chapa, estrutural e montagem (B420)

fn caixa_de(v: &Value) -> ([f64; 3], [f64; 3]) {
    (xyz(&v["caixa"]["min"]), xyz(&v["caixa"]["max"]))
}

fn caixa_do_documento(d: &Document) -> ([f64; 3], [f64; 3]) {
    final_shape(d).and_then(|s| s.bbox()).expect("a referência tem sólido")
}

fn volume_do_documento(d: &Document) -> f64 {
    let r = fenkra_solidos::recompute(d);
    for f in &d.features {
        assert_eq!(r.status[&f.id], fenkra_solidos::Status::Ok, "referência: {}", f.name);
    }
    r.bodies.iter().map(|b| b.mass.volume).sum()
}

fn confere_com_referencia(r: &Value, d: &Document, tol: f64, rotulo: &str) {
    let v = r["volume"].as_f64().unwrap();
    let alvo = volume_do_documento(d);
    assert!((v - alvo).abs() < tol, "{rotulo}: volume {v} contra {alvo}");
    let ((lo, hi), (a, b)) = (caixa_de(r), caixa_do_documento(d));
    for i in 0..3 {
        assert!((lo[i] - a[i]).abs() < 1e-3 && (hi[i] - b[i]).abs() < 1e-3, "{rotulo}: caixa {lo:?} {hi:?} contra {a:?} {b:?}");
    }
}

/// Refaz o registro da sessão sem servidor e mede de novo com o comando dado.
fn refeito(pasta: &Path, c: &mut Cliente, comando: &str) -> Value {
    let arquivo = c.recurso("fenkra://comandos")["arquivo"].as_str().unwrap().to_string();
    let mut s = Sessao::nova(Pasta::nova(pasta).unwrap(), Isolamento::Local);
    s.executar("reproduzir", &json!({ "arquivo": arquivo })).unwrap();
    s.executar(comando, &json!({})).unwrap()
}

/// B420: chapa pelo servidor real. Parede, aba e bainha geradas do esquema, conferidas contra o mesmo
/// documento montado em código, dobrada e planificada; a sequência refeita dá a mesma peça.
#[test]
fn chapa_pelo_servidor_dobrada_e_planificada() {
    precisa_kernel!();
    let pasta = pasta_limpa("chapa");
    let mut c = Cliente::iniciar(&pasta, false);
    c.chamar("nova_peca", json!({ "nome": "suporte" }));
    let (pontos, curvas) = retangulo(0., 0., 100., 50.);
    c.chamar("criar_esboco", json!({ "id": "base", "plano": "XY", "pontos": pontos, "curvas": curvas }));
    let ch = c.chamar("definir_chapa", json!({ "espessura": 2, "raio": 2, "fator_k": 0.44 }));
    assert_eq!(ch["chapa"]["espessura"], "2", "{ch}");
    let r = c.chamar("criar_parede_chapa", json!({ "esboco": "base" }));
    assert_eq!(r["status"], "ok", "{r}");
    assert!((r["volume"].as_f64().unwrap() - 10000.).abs() < 1e-3, "{r}");
    let s = c.chamar("selecionar", json!({ "lado": "direita" }));
    let meio = s["pontos"].as_array().unwrap().iter().find(|p| (p[2].as_f64().unwrap() - 2.).abs() < 1e-6).cloned().expect("aresta de cima da borda direita");
    let r = c.chamar("criar_aba", json!({ "aresta": meio, "comprimento": 10, "angulo": 90 }));
    assert_eq!(r["status"], "ok", "{r}");
    // chapa + dobra de 90° (anel de raios 2 e 4) + perna reta de 10, na largura de 50
    let dobra = std::f64::consts::FRAC_PI_4 * (16. - 4.) * 50.;
    assert!((r["volume"].as_f64().unwrap() - (10000. + dobra + 1000.)).abs() < 2., "{r}");
    let r = c.chamar("criar_bainha", json!({ "aresta": [0, 25, 2], "comprimento": 6, "folga": 0 }));
    assert_eq!(r["status"], "ok", "{r}");
    let dobrada = c.chamar("recalcular", json!({}));
    assert_eq!(dobrada["status"], "ok", "{dobrada}");

    // a mesma peça em código
    let mut d = Document::new("suporte");
    d.sheet = Some(SheetParams { thickness: "2".into(), radius: "2".into(), k: "0.44".into(), ..Default::default() });
    let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let ponto = |p: &Value| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap(), p[2].as_f64().unwrap()];
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: ponto(&meio), length: "10".into(), angle: "90".into(), radius: None, off1: "0".into(), off2: "0".into(), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None }));
    d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 25., 2.], length: "6".into(), gap: "0".into(), flip: false, kind: 0, edge_key: None }));
    confere_com_referencia(&dobrada, &d, 1e-3, "dobrada");

    // planificada: o mesmo material no plano
    let p = c.chamar("definir_chapa", json!({ "planificada": true }));
    assert_eq!(p["status"], "ok", "{p}");
    assert_eq!(p["chapa"]["espessura"], "2", "só o campo pedido muda: {p}");
    let (_, hi) = caixa_de(&p);
    assert!((hi[2] - 2.).abs() < 1e-3, "planificada: {p}");
    let (vd, vp) = (dobrada["volume"].as_f64().unwrap(), p["volume"].as_f64().unwrap());
    assert!((vd - vp).abs() / vd < 0.01, "dobrada {vd}, planificada {vp}");
    d.sheet.as_mut().unwrap().flat = true;
    confere_com_referencia(&p, &d, 1e-3, "planificada");
    let arvore = c.recurso("fenkra://arvore");
    assert_eq!(arvore["pecas"][0]["chapa"]["planificada"], true, "{arvore}");
    // o registro refaz a peça igual, planificada inclusive
    let m = c.chamar("medir", json!({}));
    assert_eq!(refeito(&pasta, &mut c, "medir")["massa"], m["massa"]);
}

/// B420: estrutural pelo servidor real. Membros em meia-esquadria sobre as retas de um esboço, placa
/// na ponta e reforço no canto, conferidos contra o mesmo quadro montado em código.
#[test]
fn estrutural_pelo_servidor_membro_placa_e_reforco() {
    precisa_kernel!();
    let pasta = pasta_limpa("estrutural");
    let mut c = Cliente::iniciar(&pasta, false);
    c.chamar("nova_peca", json!({ "nome": "quadro" }));
    // esqueleto em L no plano XZ: x do esboço é X, y é Z
    let e = c.chamar("criar_esboco", json!({
        "id": "esqueleto", "plano": "XZ",
        "pontos": [{ "id": "o", "x": 0, "y": 0 }, { "id": "t", "x": 0, "y": 500 }, { "id": "d", "x": 400, "y": 0 }],
        "curvas": [{ "id": "pilar", "geometria": { "tipo": "reta", "a": "o", "b": "t" } }, { "id": "viga", "geometria": { "tipo": "reta", "a": "o", "b": "d" } }],
    }));
    assert_eq!(e["status"], "ok", "{e}");
    let d = c.chamar("listar_comandos", json!({ "busca": "membro" }));
    assert!(d["ferramentas"].as_array().unwrap().iter().any(|t| t["nome"] == "criar_membro"), "{d}");
    let r = c.chamar("criar_membro", json!({
        "caminhos": [{ "esboco": "esqueleto", "curva": "pilar" }, { "esboco": "esqueleto", "curva": "viga" }],
        "secao": "tubo_quadrado", "cotas": [40, 3], "canto": "meia_esquadria", "designacao": "40x3",
    }));
    assert_eq!(r["status"], "ok", "{r}");
    let v_tubos = r["volume"].as_f64().unwrap();
    // dois tubos 40 x 3 (900 mm de linha), com a meia-esquadria trocando material de lugar no canto
    let secao = 40. * 40. - 34. * 34.;
    assert!((v_tubos - secao * 900.).abs() / (secao * 900.) < 0.05, "{r}");
    let r = c.chamar("criar_placa_extremidade", json!({ "em": [0, 0, 500], "espessura": 8, "sobra": 5 }));
    assert_eq!(r["status"], "ok", "{r}");
    assert!((r["volume"].as_f64().unwrap() - v_tubos - 50. * 50. * 8.).abs() < 1., "placa 50 x 50 x 8: {r}");
    let r = c.chamar("criar_reforco", json!({ "em": [0, 0, 0], "aba": 60, "espessura": 6, "chanfro": 10 }));
    assert_eq!(r["status"], "ok", "{r}");
    let quadro = c.chamar("recalcular", json!({}));
    assert_eq!(quadro["status"], "ok", "{quadro}");
    let (lo, hi) = caixa_de(&quadro);
    assert!(hi[2] > 507. && hi[0] > 399. && lo[2] < -19., "{quadro}");

    // o mesmo quadro em código, sobre retas de referência
    let mut d = Document::new("quadro");
    let reta = |d: &mut Document, a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: "0".into(), end: "0".into() }));
    let (p, v) = (reta(&mut d, [0., 0., 0.], [0., 0., 500.]), reta(&mut d, [0., 0., 0.], [400., 0., 0.]));
    d.add_feature(FeatureKind::StructMember(StructMember {
        paths: vec![p, v],
        family: String::new(),
        designation: "40x3".into(),
        kind: SectionKind::TuboQuadrado,
        dims: vec!["40".into(), "3".into()],
        anchor: 0,
        angle: "0".into(),
        off_x: "0".into(),
        off_y: "0".into(),
        ext1: "0".into(),
        ext2: "0".into(),
        corner: CornerMode::MeiaEsquadria,
        gap: "0".into(),
        joints: vec![],
        material: String::new(),
        mirror: false,
    }));
    d.add_feature(FeatureKind::StructPlate(StructPlate::nova([0., 0., 500.], "8", "5")));
    d.add_feature(FeatureKind::StructGusset(StructGusset::novo([0., 0., 0.], "60", "6", "10")));
    confere_com_referencia(&quadro, &d, 1., "quadro");

    let arvore = c.recurso("fenkra://arvore");
    let tipos: Vec<&str> = arvore["pecas"][0]["arvore"].as_array().unwrap().iter().map(|i| i["comando"]["tipo"].as_str().unwrap()).collect();
    assert_eq!(tipos, ["esboco", "membro", "placa_extremidade", "reforco"]);
    let m = c.chamar("medir", json!({}));
    assert_eq!(refeito(&pasta, &mut c, "medir")["massa"], m["massa"]);
}

fn componente<'a>(estado: &'a Value, id: &str) -> &'a Value {
    estado["componentes"].as_array().unwrap().iter().find(|c| c["id"] == id).unwrap_or_else(|| panic!("{id}: {estado}"))
}

/// B420: montagem pelo servidor real. Duas peças salvas na pasta, um pino repetido, contato e eixo
/// coincidente, movimento que a restrição desfaz, lista de peças, gravação e repetição pelo registro.
#[test]
fn montagem_pelo_servidor_restricao_movimento_e_lista_de_pecas() {
    precisa_kernel!();
    let pasta = pasta_limpa("montagem");
    let mut c = Cliente::iniciar(&pasta, false);
    // as peças: bloco 40 x 30 x 20 e pino Ø10 x 30
    c.chamar("nova_peca", json!({ "nome": "bloco" }));
    let (pontos, curvas) = retangulo(-20., -15., 20., 15.);
    c.chamar("criar_esboco", json!({ "id": "base", "plano": "XY", "pontos": pontos, "curvas": curvas }));
    c.chamar("extrudar", json!({ "esboco": "base", "operacao": "unir", "termino": { "tipo": "cego", "distancia": "20" } }));
    c.chamar("salvar_peca", json!({ "arquivo": "bloco.fkp" }));
    c.chamar("nova_peca", json!({ "nome": "pino" }));
    let r = c.chamar("criar_primitiva", json!({ "forma": "cilindro", "plano": "XY", "a": 5, "b": 30, "c": 0, "d": 0, "operacao": "unir" }));
    assert_eq!(r["status"], "ok", "{r}");
    c.chamar("salvar_peca", json!({ "arquivo": "pino.fkp" }));

    let (e, erro) = c.tentar("inserir_componente", json!({ "arquivo": "bloco.fkp" }));
    assert!(erro && e["codigo"] == "sem_montagem", "{e}");
    c.chamar("nova_montagem", json!({ "nome": "conjunto" }));
    let (e, erro) = c.tentar("inserir_componente", json!({ "arquivo": "bloco.fkx" }));
    assert!(erro && e["codigo"] == "formato", "{e}");
    let b = c.chamar("inserir_componente", json!({ "arquivo": "bloco.fkp" }));
    assert_eq!((b["id"].as_str(), b["fixo"].as_bool()), (Some("c1"), Some(true)), "{b}");
    let p = c.chamar("inserir_componente", json!({ "id": "pino", "arquivo": "pino.fkp", "origem": [100, 0, 0] }));
    assert_eq!(p["fixo"], false, "{p}");
    c.chamar("inserir_componente", json!({ "id": "solto", "arquivo": "pino.fkp", "origem": [200, 50, 0] }));

    // os pontos das faces saem da caixa de cada componente, como faria quem só lê as respostas
    let estado = c.chamar("ver_montagem", json!({}));
    let (lo, hi) = caixa_de(componente(&estado, "pino"));
    let (cx, cy, raio) = ((lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2., (hi[0] - lo[0]) / 2.);
    assert!((raio - 5.).abs() < 1e-3, "{estado}");
    let (_, topo) = caixa_de(componente(&estado, "c1"));
    let r = c.chamar("restringir", json!({ "tipo": "contato", "a": { "componente": "c1", "face": [0, 0, topo[2]] }, "b": { "componente": "pino", "face": [cx, cy, lo[2]] } }));
    assert_eq!((r["status"].as_str(), r["convergiu"].as_bool()), (Some("ok"), Some(true)), "{r}");
    // o contato subiu o pino: a face lateral se lê na posição nova; na antiga, o ponto não toca a peça
    let (e, erro) = c.tentar("restringir", json!({ "tipo": "coincidente", "a": { "componente": "c1", "eixo": "Z" }, "b": { "componente": "pino", "face": [cx + raio, cy, (lo[2] + hi[2]) / 2.] } }));
    assert!(erro && e["codigo"] == "face", "{e}");
    let estado = c.chamar("ver_montagem", json!({}));
    let (lo, hi) = caixa_de(componente(&estado, "pino"));
    assert!((lo[2] - topo[2]).abs() < 1e-6, "{estado}");
    let (cx, cy) = ((lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.);
    let r = c.chamar("restringir", json!({ "tipo": "coincidente", "a": { "componente": "c1", "eixo": "Z" }, "b": { "componente": "pino", "face": [cx + raio, cy, (lo[2] + hi[2]) / 2.] } }));
    assert_eq!(r["status"], "ok", "{r}");
    let (e, erro) = c.tentar("restringir", json!({ "tipo": "paralela", "a": { "componente": "c1", "ponto": [0, 0, 0] }, "b": { "componente": "solto", "eixo": "Z" } }));
    assert!(erro && e["codigo"] == "restricao", "{e}");

    let estado = c.chamar("ver_montagem", json!({}));
    let pino = componente(&estado, "pino");
    let o = xyz(&pino["origem"]);
    assert!(o[0].abs() < 1e-6 && o[1].abs() < 1e-6 && (o[2] - 20.).abs() < 1e-6, "o pino sobe no topo do bloco, no eixo: {pino}");
    assert_eq!(pino["graus_de_liberdade"], 1, "sobra o giro em torno do eixo: {pino}");
    assert_eq!(componente(&estado, "solto")["graus_de_liberdade"], 6, "{estado}");

    // mover o pino preso: o solucionador traz de volta; mover o solto: ele fica onde foi posto
    let r = c.chamar("mover_componente", json!({ "componente": "pino", "deslocar": [10, 0, 0] }));
    let o = xyz(&r["origem"]);
    assert!(o[0].abs() < 1e-6 && (o[2] - 20.).abs() < 1e-6, "a restrição segura o pino: {r}");
    let r = c.chamar("mover_componente", json!({ "componente": "solto", "origem": [0, 100, -30], "girar": { "eixo": "X", "angulo": 90 } }));
    assert!((xyz(&r["origem"])[2] + 30.).abs() < 1e-6, "{r}");
    let estado = c.chamar("ver_montagem", json!({}));
    let pi = std::f64::consts::PI;
    let v_esperado = 40. * 30. * 20. + 2. * pi * 25. * 30.;
    assert!((estado["volume"].as_f64().unwrap() - v_esperado).abs() < 1., "{estado}");
    let (lo, hi) = caixa_de(&estado);
    // bloco de z 0 a 20, pino de 20 a 50; o solto deitado em Y, de y 70 a 100, centrado em z = -30
    let esperado = ([-20., -15., -35.], [20., 100., 50.]);
    for i in 0..3 {
        assert!((lo[i] - esperado.0[i]).abs() < 1e-3 && (hi[i] - esperado.1[i]).abs() < 1e-3, "caixa {lo:?} {hi:?}: {estado}");
    }

    // desfazer volta o último movimento
    let u = c.chamar("desfazer", json!({}));
    assert_eq!(u["componentes"], 3, "{u}");
    let estado = c.chamar("ver_montagem", json!({}));
    assert!((xyz(&componente(&estado, "solto")["origem"])[0] - 200.).abs() < 1e-6, "{estado}");
    c.chamar("mover_componente", json!({ "componente": "solto", "origem": [0, 100, -30], "girar": { "eixo": "X", "angulo": 90 } }));

    let bom = c.chamar("lista_de_pecas", json!({}));
    let linhas = bom["linhas"].as_array().unwrap();
    let qtd = |arq: &str| linhas.iter().find(|l| l["arquivo"] == arq).map(|l| l["quantidade"].as_u64().unwrap());
    assert_eq!((qtd("bloco.fkp"), qtd("pino.fkp"), bom["pecas"].as_u64()), (Some(1), Some(2), Some(3)), "{bom}");
    assert!((linhas.iter().find(|l| l["arquivo"] == "pino.fkp").unwrap()["volume_unitario"].as_f64().unwrap() - pi * 25. * 30.).abs() < 0.1, "{bom}");

    c.chamar("salvar_montagem", json!({ "arquivo": "conjunto.fka" }));
    let asm = fenkra_montagem::Assembly::from_json(&std::fs::read_to_string(pasta.join("conjunto.fka")).unwrap()).unwrap();
    assert_eq!((asm.nodes.len(), asm.mates.len()), (3, 2));
    let (e, erro) = c.tentar("salvar_montagem", json!({ "arquivo": "conjunto.fka" }));
    assert!(erro && e["codigo"] == "ja_existe", "{e}");
    let aberta = c.chamar("salvar_montagem", json!({ "arquivo": "conjunto.fkx" }));
    assert!(aberta["fora"].as_array().unwrap().iter().any(|f| f.as_str().unwrap().contains("restrições")), "{aberta}");

    // o registro refaz a montagem igual, sem servidor
    let refeita = refeito(&pasta, &mut c, "montagem");
    assert_eq!(refeita["volume"], estado_final(&mut c)["volume"]);
    assert_eq!(refeita["caixa"], estado_final(&mut c)["caixa"]);
}

fn estado_final(c: &mut Cliente) -> Value {
    c.chamar("ver_montagem", json!({}))
}
