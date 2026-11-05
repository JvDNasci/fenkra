//! B53: corpus de peças de referência e regressão.
//!
//! Dezesseis peças montadas só com a API pública (documento em código, recompute, status, volume e
//! caixa envolvente conferidos contra a fórmula fechada de cada uma), mais o orçamento de desempenho
//! do corpus: tesselação por etapa (malha, suavidade de arestas e polilinhas), recompute sem mudança,
//! recompute com edição local e memória do processo depois de 300 edições. Os limites são folgados
//! de propósito (rodam em perfil de depuração e em máquinas diferentes): pegam regressão de ordem de
//! grandeza, não ruído. Os números medidos saem na tela com `-- --nocapture`.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem};
use fenkra_solidos::eixos::{chaveta_documento, eixo_documento, ParamEixo, TrechoEixo};
use fenkra_solidos::fixadores::fixador_documento;
use fenkra_solidos::{final_shape, recompute, recompute_cached, tesselar, Cache};
use std::f64::consts::PI;
use std::time::Instant;

macro_rules! precisa_kernel {
    () => {
        if !kernel_disponivel() {
            return;
        }
    };
}

// ------------------------------------------------------------------ as 16 peças

fn caixa() -> Document {
    let mut d = Document::new("caixa");
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(s, BoolOp::Add, "20"));
    d
}

fn cilindro() -> Document {
    let mut d = Document::new("cilindro");
    d.add_feature(primitiva(PrimKind::Cilindro, "10", "30", "0", "0"));
    d
}

fn esfera() -> Document {
    let mut d = Document::new("esfera");
    d.add_feature(primitiva(PrimKind::Esfera, "10", "0", "0", "0"));
    d
}

fn cone() -> Document {
    let mut d = Document::new("cone");
    d.add_feature(primitiva(PrimKind::Cone, "12", "6", "20", "0"));
    d
}

fn toro() -> Document {
    let mut d = Document::new("toro");
    d.add_feature(primitiva(PrimKind::Toro, "20", "5", "0", "0"));
    d
}

/// Placa 100 × 60 × 8 com grade 5 × 3 de furos passantes Ø6 a cada 20 mm.
pub fn placa_com_grade() -> Document {
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

/// Caixa 40 × 30 × 20 com bolso 10 × 10 × 5 aberto em cima.
fn caixa_com_bolso() -> Document {
    let mut d = Document::new("caixa com bolso");
    let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(base, BoolOp::Add, "20"));
    let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
    let sk = sk_rect(&mut d, PlaneRef::Datum(topo), [-5., -5.], [5., 5.]);
    d.add_feature(ext(sk, BoolOp::Cut, "5"));
    d
}

fn caixa_arredondada() -> Document {
    let mut d = caixa();
    d.add_feature(FeatureKind::Fillet(EdgeOp { value: "2".into(), edges: EdgeSel::AllOfBody, mode: EdgeMode::Constant, prop: Default::default(), keys: vec![] }));
    d
}

/// Caixa 40 × 30 × 20 esvaziada com parede de 2, aberta em cima.
fn caixa_casca() -> Document {
    let mut d = caixa();
    d.add_feature(FeatureKind::Shell(Shell { faces: vec![[0., 0., 20.]], thickness: "2".into(), outward: false, face_keys: vec![] }));
    d
}

/// Flange Ø60 × 8 com furo central Ø20 e quatro furos Ø6 no círculo de Ø40.
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

fn eixo_escalonado() -> Document {
    let p = ParamEixo {
        trechos: vec![TrechoEixo { diametro: 20., comprimento: 50., chanfro_fim: 0., rasgo: None }, TrechoEixo { diametro: 25., comprimento: 70., chanfro_fim: 0., rasgo: None }],
        furo_centro: false,
    };
    eixo_documento(&p).expect("eixo")
}

fn engrenagem_40_dentes() -> Document {
    engrenagem_documento(&ParamEngrenagem { modulo: 2., dentes: 40, largura: 10., furo: 10., ..Default::default() }).expect("engrenagem")
}

/// Bloco em L: 40 × 30 com o canto de 20 × 15 retirado, espessura 10.
fn bloco_l() -> Document {
    let mut d = Document::new("bloco L");
    let sk = sk_poligono(&mut d, PlaneRef::Origin(OriginPlane::XY), &[[0., 0.], [40., 0.], [40., 15.], [20., 15.], [20., 30.], [0., 30.]]);
    d.add_feature(ext(sk, BoolOp::Add, "10"));
    d
}

// ------------------------------------------------------------------ um teste por peça

#[test]
fn peca_01_caixa() {
    precisa_kernel!();
    confere("caixa", &caixa(), &Esperado::prisma(40. * 30. * 20., [40., 30., 20.], [-20., -15., 0.]));
}

#[test]
fn peca_02_cilindro() {
    precisa_kernel!();
    confere("cilindro", &cilindro(), &Esperado::curva(PI * 100. * 30., [20., 20., 30.]));
}

#[test]
fn peca_03_esfera() {
    precisa_kernel!();
    confere("esfera", &esfera(), &Esperado::curva(4. / 3. * PI * 1000., [20., 20., 20.]));
}

#[test]
fn peca_04_cone_truncado() {
    precisa_kernel!();
    confere("cone", &cone(), &Esperado::curva(PI * 20. / 3. * (144. + 72. + 36.), [24., 24., 20.]));
}

#[test]
fn peca_05_toro() {
    precisa_kernel!();
    confere("toro", &toro(), &Esperado::curva(2. * PI * PI * 20. * 25., [50., 50., 10.]));
}

#[test]
fn peca_06_placa_com_grade_de_furos() {
    precisa_kernel!();
    confere("placa com grade", &placa_com_grade(), &Esperado::prisma(100. * 60. * 8. - 15. * PI * 9. * 8., [100., 60., 8.], [-50., -30., 0.]));
}

#[test]
fn peca_07_caixa_com_bolso() {
    precisa_kernel!();
    confere("caixa com bolso", &caixa_com_bolso(), &Esperado::prisma(40. * 30. * 20. - 10. * 10. * 5., [40., 30., 20.], [-20., -15., 0.]));
}

#[test]
fn peca_08_caixa_com_todas_as_arestas_arredondadas() {
    precisa_kernel!();
    confere("caixa arredondada", &caixa_arredondada(), &Esperado { vol: vol_caixa_arredondada(40., 30., 20., 2.), tol_vol: 1e-3, tam: [40., 30., 20.], tol_tam: 0.01, minimo: Some([-20., -15., 0.]), via_malha: false });
}

#[test]
fn peca_09_caixa_casca_aberta_em_cima() {
    precisa_kernel!();
    confere("casca", &caixa_casca(), &Esperado::prisma(40. * 30. * 20. - 36. * 26. * 18., [40., 30., 20.], [-20., -15., 0.]));
}

#[test]
fn peca_10_flange_com_furos() {
    precisa_kernel!();
    let v = PI * 8. * (900. - 100. - 4. * 9.);
    confere("flange", &flange(), &Esperado::curva(v, [60., 60., 8.]));
}

#[test]
fn peca_11_eixo_escalonado() {
    precisa_kernel!();
    let v = PI / 4. * (20. * 20. * 50. + 25. * 25. * 70.);
    confere("eixo", &eixo_escalonado(), &Esperado::curva(v, [25., 25., 120.]));
}

#[test]
fn peca_12_engrenagem_de_40_dentes() {
    precisa_kernel!();
    let d = engrenagem_40_dentes();
    let r = recompute(&d);
    assert_eq!(problema_de_status(&d, &r), None);
    assert_eq!(r.bodies.len(), 1);
    // entre o cilindro de raiz (Ø75) e o de topo (Ø84), menos o furo Ø10, vezes a largura 10
    let (raiz, topo) = (37.5_f64, 42.0_f64);
    let v = volume_total(&r);
    let (lo, hi) = (PI * (raiz * raiz - 25.) * 10., PI * (topo * topo - 25.) * 10.);
    assert!(v > lo && v < hi, "volume {v} fora de [{lo}, {hi}]");
    let (a, b) = caixa_envolvente(&d);
    assert!((b[2] - a[2] - 10.).abs() < 1e-3, "largura {}", b[2] - a[2]);
    for i in 0..2 {
        let t = b[i] - a[i];
        assert!(t > 2. * raiz && t < 2. * topo + 0.1, "diâmetro externo do eixo {i}: {t}");
    }
    assert_eq!(r.bodies[0].faces as usize, r.bodies[0].mesh.face_ranges.len());
    // 40 dentes: pelo menos 2 faces de flanco por dente (topo, fundo e flancos somam mais)
    assert!(r.bodies[0].faces >= 80, "faces {}", r.bodies[0].faces);
}

#[test]
fn peca_13_parafuso_allen_m6x20() {
    precisa_kernel!();
    let d = fixador_documento("iso4762", "M6", Some(20.)).unwrap();
    // cabeça Ø10 × 6, haste Ø6 × 20 (abaixo de z = 0) e sextavado interno de abertura 5 e profundidade 3
    let v = PI / 4. * 100. * 6. + PI / 4. * 36. * 20. - 3_f64.sqrt() / 2. * 25. * 3.;
    confere("parafuso", &d, &Esperado { vol: v, tol_vol: 0.01, tam: [10., 10., 26.], tol_tam: 0.2, minimo: None, via_malha: true });
}

#[test]
fn peca_14_porca_sextavada_m8() {
    precisa_kernel!();
    let d = fixador_documento("iso4032", "M8", None).unwrap();
    let v = (3_f64.sqrt() / 2. * 169. - PI / 4. * 64.) * 6.8;
    confere("porca", &d, &Esperado { vol: v, tol_vol: 0.01, tam: [13. / 30_f64.to_radians().cos(), 13., 6.8], tol_tam: 1e-3, minimo: None, via_malha: true });
}

#[test]
fn peca_15_chaveta_din6885_40() {
    precisa_kernel!();
    // eixo Ø25 pede 8 × 7; comprimento 40 com pontas arredondadas (estádio)
    let d = chaveta_documento(25., 40.).unwrap();
    let (b, h, l) = (8.0_f64, 7.0_f64, 40.0_f64);
    let v = h * (b * (l - b) + PI * b * b / 4.);
    confere("chaveta", &d, &Esperado { vol: v, tol_vol: 2e-3, tam: [l, b, h], tol_tam: 0.1, minimo: None, via_malha: true });
}

#[test]
fn peca_16_bloco_em_l() {
    precisa_kernel!();
    confere("bloco L", &bloco_l(), &Esperado::prisma(900. * 10., [40., 30., 10.], [0., 0., 0.]));
}

#[test]
fn corpus_tem_as_dezesseis_pecas_e_todas_serializam() {
    let pecas = [
        caixa(),
        cilindro(),
        esfera(),
        cone(),
        toro(),
        placa_com_grade(),
        caixa_com_bolso(),
        caixa_arredondada(),
        caixa_casca(),
        flange(),
        eixo_escalonado(),
        engrenagem_40_dentes(),
        fixador_documento("iso4762", "M6", Some(20.)).unwrap(),
        fixador_documento("iso4032", "M8", None).unwrap(),
        chaveta_documento(25., 40.).unwrap(),
        bloco_l(),
    ];
    assert_eq!(pecas.len(), 16);
    if !kernel_disponivel() {
        return;
    }
    for d in &pecas {
        // salvar e abrir devolve uma peça que recalcula com o mesmo volume e as mesmas faces
        let volta = Document::from_json(&d.to_json()).unwrap_or_else(|e| panic!("{}: {e}", d.name));
        let (a, b) = (recompute(d), recompute(&volta));
        assert_eq!(problema_de_status(&volta, &b), None, "{}", d.name);
        assert_eq!(a.bodies[0].faces, b.bodies[0].faces, "{}", d.name);
        assert!((volume_total(&a) - volume_total(&b)).abs() < 1e-6 * volume_total(&a).max(1.), "{}: volume depois de salvar e abrir", d.name);
    }
}

/// BUG registrado: salvar e abrir não devolve o mesmo documento. O JSON perde o último dígito de alguns
/// `f64` (o `serde_json` do workspace está sem `float_roundtrip`), então coordenadas de sketch voltam
/// com 1 ulp de diferença: `==` falha e a assinatura do cache muda depois de reabrir o arquivo.
#[test]
#[ignore = "bug: serde_json sem float_roundtrip; coordenadas do sketch da engrenagem voltam com 1 ulp de erro (corrigir em Cargo.toml do workspace)"]
fn salvar_e_abrir_devolve_documento_identico() {
    let d = engrenagem_40_dentes();
    let volta = Document::from_json(&d.to_json()).unwrap();
    assert_eq!(volta, d);
}

// ------------------------------------------------------------------ orçamento de desempenho

/// Limites em segundos (perfil de depuração, folgados em ordem de grandeza).
mod orcamento {
    pub const TESSELACAO_ENGRENAGEM: f64 = 20.;
    pub const TESSELACAO_PLACA: f64 = 10.;
    pub const RECOMPUTE_SEM_MUDANCA: f64 = 2.;
    pub const RECOMPUTE_EDICAO_LOCAL: f64 = 10.;
    /// crescimento de memória privada do processo entre a edição 50 e a 300, em MiB
    pub const CRESCIMENTO_MEMORIA_MIB: f64 = 150.;
}

fn segundos<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t = Instant::now();
    let r = f();
    (r, t.elapsed().as_secs_f64())
}

#[test]
fn desempenho_tesselacao_por_etapa() {
    precisa_kernel!();
    for (nome, d, limite) in [("engrenagem 40 dentes", engrenagem_40_dentes(), orcamento::TESSELACAO_ENGRENAGEM), ("placa com grade", placa_com_grade(), orcamento::TESSELACAO_PLACA)] {
        let (forma, t_forma) = segundos(|| final_shape(&d).expect("forma final"));
        let (malha, t_malha) = segundos(|| tesselar(&forma, &Default::default()).expect("tesselação"));
        // etapa 1: malha de triângulos
        assert!(!malha.positions.is_empty() && malha.indices.len() % 3 == 0, "{nome}: malha vazia ou quebrada");
        assert_eq!(malha.positions.len(), malha.normals.len(), "{nome}: uma normal por posição");
        assert!(malha.indices.iter().all(|i| (*i as usize) < malha.positions.len() / 3), "{nome}: índice fora da malha");
        // etapa 2: suavidade de arestas (uma marca por aresta)
        let n_arestas = malha.edge_ranges.len();
        assert!(n_arestas > 0, "{nome}: sem arestas");
        assert_eq!(malha.edge_smooth.len(), n_arestas, "{nome}: suavidade por aresta");
        assert_eq!(malha.edge_mids.len(), 3 * n_arestas, "{nome}: ponto médio por aresta");
        assert!(malha.edge_smooth.iter().any(|s| *s == 1) && malha.edge_smooth.iter().any(|s| *s == 0), "{nome}: espera arestas suaves e vivas");
        // etapa 3: polilinhas das arestas (segmentos de 6 floats)
        assert!(!malha.edges.is_empty() && malha.edges.len() % 6 == 0, "{nome}: polilinhas");
        println!("B53 tesselação {nome}: forma {t_forma:.3}s, malha {t_malha:.3}s, {} triângulos, {} arestas ({} suaves), {} segmentos", malha.indices.len() / 3, n_arestas, malha.edge_smooth.iter().filter(|s| **s == 1).count(), malha.edges.len() / 6);
        assert!(t_forma + t_malha < limite, "{nome}: {:.2}s passa de {limite}s", t_forma + t_malha);
    }
}

#[test]
fn desempenho_recompute_sem_mudanca() {
    precisa_kernel!();
    let d = engrenagem_40_dentes();
    let mut cache = Cache::default();
    let (primeiro, t1) = segundos(|| recompute_cached(&d, &mut cache));
    assert_eq!(problema_de_status(&d, &primeiro), None);
    let v = volume_total(&primeiro);
    let (segundo, t2) = segundos(|| recompute_cached(&d, &mut cache));
    println!("B53 recompute engrenagem: a frio {t1:.3}s, sem mudança {t2:.4}s, {} de {} comandos reaproveitados", cache.reused, d.features.len());
    assert!(cache.reused >= d.features.len() / 2, "sem mudança, a maior parte vem do cache: {} de {}", cache.reused, d.features.len());
    assert!((volume_total(&segundo) - v).abs() < 1e-9, "o cache não pode mudar o resultado");
    assert!(t2 < orcamento::RECOMPUTE_SEM_MUDANCA, "{t2:.3}s passa de {}s", orcamento::RECOMPUTE_SEM_MUDANCA);
    assert!(t2 <= t1, "recalcular sem mudança não pode custar mais que o cálculo a frio");
}

#[test]
fn desempenho_recompute_com_edicao_local() {
    precisa_kernel!();
    let mut d = placa_com_grade();
    let mut cache = Cache::default();
    let (_, t_frio) = segundos(|| recompute_cached(&d, &mut cache));
    // edita só o último comando (diâmetro dos furos): a extrusão da placa vem do cache
    let furo = d.features.last().unwrap().id;
    if let Some(FeatureKind::Hole(h)) = d.feature_mut(furo).map(|f| &mut f.kind) {
        h.diameter = "8".into();
    }
    let (r, t_local) = segundos(|| recompute_cached(&d, &mut cache));
    assert_eq!(problema_de_status(&d, &r), None);
    let esperado = 100. * 60. * 8. - 15. * PI * 16. * 8.;
    assert!((volume_total(&r) - esperado).abs() < 1e-3 * esperado, "volume {} vs {esperado}", volume_total(&r));
    println!("B53 edição local (placa): a frio {t_frio:.3}s, edição do último comando {t_local:.3}s, {} comandos reaproveitados", cache.reused);
    assert!(cache.reused >= 1, "a extrusão antes da edição tem que vir do cache");
    assert!(t_local < orcamento::RECOMPUTE_EDICAO_LOCAL, "{t_local:.3}s passa de {}s", orcamento::RECOMPUTE_EDICAO_LOCAL);
}

#[test]
fn desempenho_memoria_depois_de_300_edicoes() {
    precisa_kernel!();
    let Some(_) = memoria_privada() else {
        println!("B53 memória: medida indisponível nesta plataforma, teste ignorado");
        return;
    };
    let mut d = placa_com_grade();
    let furo = d.features.last().unwrap().id;
    let mut cache = Cache::default();
    let mut base = 0usize;
    let t = Instant::now();
    for i in 0..300 {
        // alterna o diâmetro entre 5,0 e 7,9 mm para nunca repetir o mesmo cálculo em seguida
        if let Some(FeatureKind::Hole(h)) = d.feature_mut(furo).map(|f| &mut f.kind) {
            h.diameter = format!("{}", 5. + (i % 30) as f64 / 10.);
        }
        let r = recompute_cached(&d, &mut cache);
        assert_eq!(problema_de_status(&d, &r), None, "edição {i}");
        if i == 49 {
            base = memoria_privada().unwrap();
        }
    }
    let fim = memoria_privada().unwrap();
    let crescimento = (fim as f64 - base as f64) / (1024. * 1024.);
    println!("B53 memória: {:.1} MiB na edição 50, {:.1} MiB na 300 (cresceu {crescimento:.1} MiB), 300 edições em {:.2}s", base as f64 / 1048576., fim as f64 / 1048576., t.elapsed().as_secs_f64());
    assert!(crescimento < orcamento::CRESCIMENTO_MEMORIA_MIB, "memória cresceu {crescimento:.1} MiB entre a edição 50 e a 300 (limite {} MiB)", orcamento::CRESCIMENTO_MEMORIA_MIB);
    // o cache não pode crescer sem limite: o último cálculo reaproveita tudo antes do furo
    assert!(cache.reused >= 1);
}

#[test]
fn recompute_do_corpus_e_deterministico() {
    precisa_kernel!();
    // duas rodadas independentes dão o mesmo volume e a mesma contagem de faces (base da regressão)
    for d in [engrenagem_40_dentes(), placa_com_grade(), caixa_arredondada()] {
        let (a, b) = (recompute(&d), recompute(&d));
        assert_eq!(a.bodies[0].faces, b.bodies[0].faces, "{}", d.name);
        assert!((volume_total(&a) - volume_total(&b)).abs() < 1e-9, "{}", d.name);
    }
}
