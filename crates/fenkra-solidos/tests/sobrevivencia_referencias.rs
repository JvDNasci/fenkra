// SPDX-License-Identifier: LGPL-2.1-or-later
//! B306: teste de sobrevivência das referências (ADR 0010).
//!
//! Para cada cenário do corpus, a referência (aresta de filete/chanfro ou face de casca) é escolhida
//! como o picking faz, num documento-base. Depois as cotas mudam ao acaso (semente fixa) e o documento
//! recalcula. Cada referência cai numa de três classes:
//!
//! - **mantida**: o comando continua ok e o resultado é o esperado pela fórmula fechada, com a face
//!   gerada exatamente onde a aresta/face lógica foi parar;
//! - **em erro com sugestão**: o comando fica em erro "referência perdida: ... escolha de novo";
//! - **trocada em silêncio**: o comando fica ok mas atuou em outra entidade (ou em nenhuma). Precisa
//!   ser sempre zero. É o único contador que reprova o teste.
//!
//! Há ainda "erro sem sugestão" (o kernel falhou por outro motivo) e "pulada" (cota fora do domínio em
//! que a fórmula vale), contadas à parte no relatório. Cada cenário roda com chave v2 e sem chave
//! (arquivo antigo, só o ponto clicado).
//!
//! Variáveis: `FENKRA_B306_N` (amostras por cenário e variante, padrão 1000), `FENKRA_B306_SEED`
//! (semente, padrão fixa) e `FENKRA_B306_SO` (reexecuta só a amostra de índice dado, com impressão).
//! O relatório vai para a tela e para `target/tmp/b306-relatorio-<variante>.txt`.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::{recompute, RecomputeResult, Status};
use std::f64::consts::PI;
use std::fmt::Write as _;

const SEMENTE_PADRAO: u64 = 0xB306_2026;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Esperanca {
    /// a entidade ainda existe: o comando tem que atuar nela
    Mantida,
    /// a entidade deixou de existir: o comando tem que parar com sugestão
    Perdida,
}

#[derive(Clone, PartialEq, Debug)]
enum Classe {
    Mantida,
    ErroComSugestao,
    ErroSemSugestao(String),
    Trocada(String),
}

struct Amostra {
    /// `None` = cota fora do domínio da fórmula (não conta)
    esperada: Option<Esperanca>,
    classe: Classe,
    descricao: String,
}

#[derive(Default)]
struct Contagem {
    total: usize,
    puladas: usize,
    mantidas: usize,
    erro_com_sugestao: usize,
    erro_sem_sugestao: usize,
    trocadas: usize,
    /// esperava mantida e veio erro com sugestão (seguro, mas a referência não sobreviveu)
    erro_indevido: usize,
    /// esperava perdida e veio erro com sugestão (o comportamento certo)
    perda_detectada: usize,
    exemplos: Vec<String>,
}

fn classificar(r: &RecomputeResult, d: &Document, comando: Id, esperada: Esperanca, confere_resultado: impl Fn(&RecomputeResult) -> Result<(), String>) -> Option<Classe> {
    // qualquer outro comando em erro = entrada inválida, a amostra não vale
    for f in &d.features {
        if f.id != comando && matches!(r.status.get(&f.id), Some(Status::Error(_)) | None) {
            return None;
        }
    }
    Some(match &r.status[&comando] {
        // erro com sugestão: diz o que se perdeu e manda escolher de novo (as duas redações existentes)
        Status::Error(m) if m.starts_with("referência perdida") || m.contains("reedite") || m.contains("escolha de novo") => Classe::ErroComSugestao,
        Status::Error(m) => Classe::ErroSemSugestao(m.clone()),
        Status::Ok | Status::Warning(_) => match esperada {
            Esperanca::Perdida => Classe::Trocada("a entidade não existe mais e o comando seguiu ok".into()),
            Esperanca::Mantida => match confere_resultado(r) {
                Ok(()) => Classe::Mantida,
                Err(e) => Classe::Trocada(e),
            },
        },
        Status::Inactive => return None,
    })
}

fn perto(a: f64, b: f64, tol_rel: f64, tol_abs: f64) -> bool {
    (a - b).abs() <= tol_rel * b.abs() + tol_abs
}

// ------------------------------------------------------------------ cenários

/// Caixa L × W × H com bolso `2mx × 2hy × dp` na face de cima; o filete/chanfro vai na aresta entre a
/// tampa de cima e a parede do bolso em x = +mx.
struct Bolso {
    base: Document,
    comando: Id,
    sk_base: Id,
    sk_bolso: Id,
}

fn montar_bolso(chanfrar: bool, com_chave: bool) -> Bolso {
    let mut d = Document::new("bolso");
    for (n, v) in [("H", "20"), ("dp", "5"), ("r", "1")] {
        d.add_param(n, v, Unit::Comprimento).unwrap();
    }
    let sk_base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(sk_base, BoolOp::Add, "H"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "H");
    let sk_bolso = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
    d.add_feature(ext(sk_bolso, BoolOp::Cut, "dp"));
    let r0 = recompute(&d);
    assert_eq!(problema_de_status(&d, &r0), None, "base do bolso");
    let b = &r0.bodies[0];
    let e = aresta_perto(b, [5., 0., 20.]);
    let mid = ponto_medio(b, e);
    let key = b.edge_key(e).expect("a aresta do bolso tem duas faces com papel conhecido");
    let keys = if com_chave { vec![Some(key)] } else { vec![] };
    let kind = if chanfrar { chanfro("r", vec![mid], keys) } else { filete("r", vec![mid], keys) };
    let comando = d.add_feature(kind);
    let r1 = recompute(&d);
    assert_eq!(r1.status[&comando], Status::Ok, "base do bolso com o comando de aresta: {:?}", r1.status[&comando]);
    Bolso { base: d, comando, sk_base, sk_bolso }
}

fn amostra_bolso(b: &Bolso, chanfrar: bool, s: &mut Sorteio) -> Amostra {
    let (l, w, h) = (s.cota(30., 80.), s.cota(20., 60.), s.cota(8., 40.));
    let dp = s.cota(2.5, h - 2.);
    let mx = s.cota(2., l / 2. + 8.);
    let hy = s.cota(2., w / 2. - 2.);
    let r = s.cota(0.3, 1.5);
    let descricao = format!("L={l} W={w} H={h} dp={dp} mx={mx} hy={hy} {}={r}", if chanfrar { "c" } else { "r" });
    let folga = l / 2. - mx;
    let pular = |classe| Amostra { esperada: None, classe, descricao: descricao.clone() };
    // domínio da fórmula: a parede existe com um ressalto largo o bastante, ou desapareceu de vez
    let esperada = if folga >= 1.5 * r + 0.5 {
        Esperanca::Mantida
    } else if folga <= -0.5 {
        Esperanca::Perdida
    } else {
        return pular(Classe::Mantida);
    };
    let mut d = b.base.clone();
    set_rect(&mut d, b.sk_base, [-l / 2., -w / 2.], [l / 2., w / 2.]);
    set_rect(&mut d, b.sk_bolso, [-mx, -hy], [mx, hy]);
    for (n, v) in [("H", h), ("dp", dp), ("r", r)] {
        definir_param(&mut d, n, &v.to_string());
    }
    let res = recompute(&d);
    let v_esp = l * w * h - 2. * mx * 2. * hy * dp - if chanfrar { r * r / 2. } else { r * r * (1. - PI / 4.) } * 2. * hy;
    let confere = |r_: &RecomputeResult| -> Result<(), String> {
        let v = volume_total(r_);
        if !perto(v, v_esp, 2e-4, 0.05) {
            return Err(format!("volume {v:.3} esperado {v_esp:.3}"));
        }
        let faces = &r_.bodies[0].mesh.face_info;
        let na_aresta_certa = |f: &fenkra_kernel::FaceInfo| f.point[0] >= mx - 0.02 && f.point[0] <= mx + r + 0.02 && f.point[2] >= h - r - 0.02 && f.point[2] <= h + 0.02;
        let achadas: Vec<_> = faces
            .iter()
            .filter(|f| if chanfrar { f.kind == 0 && (f.dir[0].abs() - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.02 && (f.dir[2].abs() - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.02 } else { f.kind == 1 && f.dir[1].abs() > 0.99 })
            .collect();
        match achadas.as_slice() {
            [f] if na_aresta_certa(f) => Ok(()),
            [f] => Err(format!("a face nova ficou em {:?}, longe da aresta x=+{mx}", f.point)),
            outras => Err(format!("{} faces de {} (esperava 1)", outras.len(), if chanfrar { "chanfro" } else { "filete" })),
        }
    };
    match classificar(&res, &d, b.comando, esperada, confere) {
        Some(classe) => Amostra { esperada: Some(esperada), classe, descricao },
        None => pular(Classe::Mantida),
    }
}

/// Caixa L × W × H esvaziada por casca de espessura `t`, aberta pela tampa de cima.
struct Casca {
    base: Document,
    comando: Id,
    sk_base: Id,
}

fn montar_casca(com_chave: bool) -> Casca {
    let mut d = Document::new("casca");
    for (n, v) in [("H", "20"), ("t", "2")] {
        d.add_param(n, v, Unit::Comprimento).unwrap();
    }
    let sk_base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    let caixa = d.add_feature(ext(sk_base, BoolOp::Add, "H"));
    let r0 = recompute(&d);
    let b = &r0.bodies[0];
    let topo = (0..b.face_role.len()).find(|i| b.face_owner[*i] == caixa && b.face_role[*i] == 1).expect("tampa final");
    let keys = if com_chave { vec![b.face_key(topo)] } else { vec![] };
    let comando = d.add_feature(FeatureKind::Shell(Shell { faces: vec![[0., 0., 20.]], thickness: "t".into(), outward: false, face_keys: keys, closed: false }));
    let r1 = recompute(&d);
    assert_eq!(r1.status[&comando], Status::Ok, "base da casca: {:?}", r1.status[&comando]);
    Casca { base: d, comando, sk_base }
}

fn amostra_casca(b: &Casca, s: &mut Sorteio) -> Amostra {
    let (l, w, h) = (s.cota(30., 80.), s.cota(20., 60.), s.cota(8., 40.));
    let t = s.cota(0.5, 3.);
    let descricao = format!("L={l} W={w} H={h} t={t}");
    let mut d = b.base.clone();
    set_rect(&mut d, b.sk_base, [-l / 2., -w / 2.], [l / 2., w / 2.]);
    definir_param(&mut d, "H", &h.to_string());
    definir_param(&mut d, "t", &t.to_string());
    let res = recompute(&d);
    let v_esp = l * w * h - (l - 2. * t) * (w - 2. * t) * (h - t);
    let anel = l * w - (l - 2. * t) * (w - 2. * t);
    let confere = |r_: &RecomputeResult| -> Result<(), String> {
        let v = volume_total(r_);
        if !perto(v, v_esp, 2e-4, 0.05) {
            return Err(format!("volume {v:.3} esperado {v_esp:.3}"));
        }
        let faces = &r_.bodies[0].mesh.face_info;
        // aberta em cima: o anel de topo existe em z = H e não há anel em z = 0
        let anel_em = |z: f64| faces.iter().any(|f| f.kind == 0 && f.dir[2].abs() > 0.99 && (f.point[2] - z).abs() < 0.02 && perto(f.area, anel, 0.01, 0.1));
        if !anel_em(h) {
            return Err(format!("sem anel de abertura no topo (z={h})"));
        }
        if anel_em(0.) {
            return Err("a casca abriu a face de baixo".into());
        }
        Ok(())
    };
    match classificar(&res, &d, b.comando, Esperanca::Mantida, confere) {
        Some(classe) => Amostra { esperada: Some(Esperanca::Mantida), classe, descricao },
        None => Amostra { esperada: None, classe: Classe::Mantida, descricao },
    }
}

/// Cilindro de raio R e altura H com filete de raio r na borda circular de cima.
struct Cilindro {
    base: Document,
    comando: Id,
    sk_base: Id,
}

fn montar_cilindro(com_chave: bool) -> Cilindro {
    let mut d = Document::new("cilindro");
    for (n, v) in [("H", "20"), ("r", "1")] {
        d.add_param(n, v, Unit::Comprimento).unwrap();
    }
    let sk_base = sk_circulos(&mut d, PlaneRef::Origin(OriginPlane::XY), &[([0., 0.], 10.)]);
    d.add_feature(ext(sk_base, BoolOp::Add, "H"));
    let r0 = recompute(&d);
    let b = &r0.bodies[0];
    // a aresta circular de cima: a que tem um ponto médio em z = 20
    let e = (0..b.mesh.edge_mids.len() / 3).find(|e| (ponto_medio(b, *e)[2] - 20.).abs() < 1e-3).expect("borda circular de cima");
    let mid = ponto_medio(b, e);
    let keys = if com_chave { vec![Some(b.edge_key(e).expect("chave da borda"))] } else { vec![] };
    let comando = d.add_feature(filete("r", vec![mid], keys));
    let r1 = recompute(&d);
    assert_eq!(r1.status[&comando], Status::Ok, "base do cilindro: {:?}", r1.status[&comando]);
    Cilindro { base: d, comando, sk_base }
}

fn amostra_cilindro(b: &Cilindro, s: &mut Sorteio) -> Amostra {
    let (raio, h) = (s.cota(4., 30.), s.cota(6., 40.));
    let r = s.cota(0.3, (raio / 2.).min(h / 2.).min(2.5));
    let descricao = format!("R={raio} H={h} r={r}");
    let mut d = b.base.clone();
    set_circulo(&mut d, b.sk_base, [0., 0.], raio);
    definir_param(&mut d, "H", &h.to_string());
    definir_param(&mut d, "r", &r.to_string());
    let res = recompute(&d);
    let v_esp = PI * raio * raio * h - vol_filete_borda_circular(raio, r);
    let confere = |r_: &RecomputeResult| -> Result<(), String> {
        let v = volume_total(r_);
        if !perto(v, v_esp, 2e-3, 0.1) {
            return Err(format!("volume {v:.3} esperado {v_esp:.3}"));
        }
        // a tampa de cima encolheu para o círculo de raio R − r; a de baixo continua inteira
        let faces = &r_.bodies[0].mesh.face_info;
        let tampa = |z: f64, area: f64| faces.iter().any(|f| f.kind == 0 && f.dir[2].abs() > 0.99 && (f.point[2] - z).abs() < 0.02 && perto(f.area, area, 0.01, 0.1));
        if !tampa(h, PI * (raio - r).powi(2)) || !tampa(0., PI * raio * raio) {
            return Err("o filete não ficou na borda de cima".into());
        }
        Ok(())
    };
    match classificar(&res, &d, b.comando, Esperanca::Mantida, confere) {
        Some(classe) => Amostra { esperada: Some(Esperanca::Mantida), classe, descricao },
        None => Amostra { esperada: None, classe: Classe::Mantida, descricao },
    }
}

// ------------------------------------------------------------------ execução e relatório

fn variavel(nome: &str) -> Option<u64> {
    std::env::var(nome).ok().and_then(|v| v.trim().parse().ok())
}

fn rodar(nome: &str, n: usize, semente: u64, so: Option<usize>, mut uma: impl FnMut(&mut Sorteio) -> Amostra) -> Contagem {
    let mut c = Contagem::default();
    for i in 0..n {
        if so.is_some_and(|k| k != i) {
            continue;
        }
        // cada amostra tem a própria sequência: dá para repetir a falha i sem rodar as anteriores
        let mut s = Sorteio(semente ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let a = uma(&mut s);
        c.total += 1;
        let Some(esperada) = a.esperada else {
            c.puladas += 1;
            continue;
        };
        if so.is_some() {
            println!("B306 {nome} #{i}: {} -> {:?} (esperada {esperada:?})", a.descricao, a.classe);
        }
        match &a.classe {
            Classe::Mantida => c.mantidas += 1,
            Classe::ErroComSugestao => {
                c.erro_com_sugestao += 1;
                match esperada {
                    Esperanca::Mantida => c.erro_indevido += 1,
                    Esperanca::Perdida => c.perda_detectada += 1,
                }
            }
            Classe::ErroSemSugestao(m) => {
                c.erro_sem_sugestao += 1;
                if c.exemplos.len() < 8 {
                    c.exemplos.push(format!("erro sem sugestão #{i} [{}]: {m}", a.descricao));
                }
            }
            Classe::Trocada(m) => {
                c.trocadas += 1;
                if c.exemplos.len() < 8 {
                    c.exemplos.push(format!("TROCADA EM SILÊNCIO #{i} [{}] (esperada {esperada:?}): {m}", a.descricao));
                }
            }
        }
    }
    c
}

fn linha(nome: &str, c: &Contagem) -> String {
    let validas = c.total - c.puladas;
    format!(
        "{nome:<22} {validas:>6} {:>8} {:>10} {:>10} {:>8}   (perda detectada {}, erro indevido {}, puladas {})",
        c.mantidas, c.erro_com_sugestao, c.erro_sem_sugestao, c.trocadas, c.perda_detectada, c.erro_indevido, c.puladas
    )
}

/// Roda os quatro cenários na variante dada e devolve (relatório, trocadas em silêncio).
fn campanha(variante: &str, com_chave: bool) -> (String, usize) {
    let n = variavel("FENKRA_B306_N").map(|v| v as usize).unwrap_or(1000);
    let semente = variavel("FENKRA_B306_SEED").unwrap_or(SEMENTE_PADRAO);
    let so = variavel("FENKRA_B306_SO").map(|v| v as usize);
    let bf = montar_bolso(false, com_chave);
    let bc = montar_bolso(true, com_chave);
    let ca = montar_casca(com_chave);
    let ci = montar_cilindro(com_chave);
    let linhas = [
        ("filete na borda do bolso", rodar("filete-bolso", n, semente, so, |s| amostra_bolso(&bf, false, s))),
        ("chanfro na borda do bolso", rodar("chanfro-bolso", n, semente.wrapping_add(1), so, |s| amostra_bolso(&bc, true, s))),
        ("casca na tampa de cima", rodar("casca", n, semente.wrapping_add(2), so, |s| amostra_casca(&ca, s))),
        ("filete na borda circular", rodar("filete-circular", n, semente.wrapping_add(3), so, |s| amostra_cilindro(&ci, s))),
    ];
    let mut rel = String::new();
    writeln!(rel, "B306 sobrevivência das referências, variante: {variante}").unwrap();
    writeln!(rel, "semente {semente:#x}, {n} amostras por comando").unwrap();
    writeln!(rel, "{:<22} {:>6} {:>8} {:>10} {:>10} {:>8}", "comando", "válidas", "mantida", "erro+sug.", "erro s/sug", "TROCADA").unwrap();
    let mut soma = Contagem::default();
    for (nome, c) in &linhas {
        writeln!(rel, "{}", linha(nome, c)).unwrap();
        soma.total += c.total;
        soma.puladas += c.puladas;
        soma.mantidas += c.mantidas;
        soma.erro_com_sugestao += c.erro_com_sugestao;
        soma.erro_sem_sugestao += c.erro_sem_sugestao;
        soma.trocadas += c.trocadas;
        soma.erro_indevido += c.erro_indevido;
        soma.perda_detectada += c.perda_detectada;
    }
    writeln!(rel, "{}", linha("TOTAL", &soma)).unwrap();
    for (_, c) in &linhas {
        for e in &c.exemplos {
            writeln!(rel, "  {e}").unwrap();
        }
    }
    let _ = writeln!(rel, "repetir uma falha: FENKRA_B306_SEED={semente} FENKRA_B306_SO=<índice> cargo test -p fenkra-solidos --test sobrevivencia_referencias -- --nocapture");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join(format!("b306-relatorio-{variante}.txt")), &rel);
    println!("{rel}");
    (rel, soma.trocadas)
}

#[test]
fn referencias_com_chave_v2_nunca_sao_trocadas_em_silencio() {
    if !kernel_disponivel() {
        return;
    }
    let (rel, trocadas) = campanha("com-chave", true);
    assert_eq!(trocadas, 0, "referência trocada em silêncio:\n{rel}");
}

/// BUG registrado (aresta só com o ponto clicado, arquivo antigo): o recompute casa o ponto com a aresta
/// mais próxima dentro de 5% da diagonal do corpo (`resolve_edges` em lib.rs e `refs::edge_near`, mínimo de
/// 1 mm), mas a ADR 0010 promete erro para aresta a mais de 1 mm. Com o perfil do bolso mudado, o
/// ponto antigo cai perto de outra aresta (ex.: o encontro piso-parede do bolso, a 4 mm) e o filete
/// segue ok na aresta errada. Rode com `--ignored` para ver os números.
#[test]
fn referencias_antigas_so_com_ponto_nunca_sao_trocadas_em_silencio() {
    if !kernel_disponivel() {
        return;
    }
    let (rel, trocadas) = campanha("so-ponto", false);
    assert_eq!(trocadas, 0, "referência só com ponto trocada em silêncio:\n{rel}");
}

#[test]
fn sorteio_com_semente_fixa_repete() {
    let (mut a, mut b) = (Sorteio(SEMENTE_PADRAO), Sorteio(SEMENTE_PADRAO));
    let xs: Vec<f64> = (0..5).map(|_| a.cota(2., 50.)).collect();
    let ys: Vec<f64> = (0..5).map(|_| b.cota(2., 50.)).collect();
    assert_eq!(xs, ys);
    assert!(xs.iter().all(|v| (2.0..=50.0).contains(v)));
}
