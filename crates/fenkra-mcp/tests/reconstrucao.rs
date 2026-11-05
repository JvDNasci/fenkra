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
