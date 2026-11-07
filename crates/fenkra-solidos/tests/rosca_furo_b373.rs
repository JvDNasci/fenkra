// SPDX-License-Identifier: LGPL-2.1-or-later
//! B373: rosca modelada em furo M16x2. Sulco contínuo (sem degrau na entrada nem faces soltas),
//! começando na boca como uma rosca feita com macho, sem falhar com ponta, profundidade de rosca e folga;
//! chanfro de entrada, folga em cima e embaixo, e ângulo de início com referência.
//!
//! Cada teste monta o documento em código, recalcula e confere status, volume e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_solidos::rosca_furo::gira;
use fenkra_solidos::{final_shape, recompute, Status};
use std::collections::HashMap;
use std::f64::consts::PI;

const XY: PlaneRef = PlaneRef::Origin(OriginPlane::XY);
/// Bloco de 60 x 60 x 30 com o furo de cima (z = 30), M16x2, broca de 14 mm.
const TOPO: f64 = 30.;

fn s(x: &str) -> String {
    x.into()
}

fn bloco(ajusta: impl FnOnce(&mut Hole)) -> (Document, Id) {
    let mut d = Document::new("M16");
    let sk = sk_rect(&mut d, XY, [-30., -30.], [30., 30.]);
    d.add_feature(ext(sk, BoolOp::Add, "30"));
    let topo = d.add_offset_plane(XY, "30");
    let sp = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[0., 0.]]);
    let mut h = Hole {
        sketch: sp,
        kind: HoleKind::Simples,
        diameter: s("14"),
        depth: Extent::Blind(s("20")),
        thread: Some(s("M16x2")),
        thread_modeled: true,
        standard: None,
        points: vec![],
        dir: None,
        reverse: false,
        tip: None,
        taper: s("0"),
        left_hand: false,
        extra: Default::default(),
    };
    ajusta(&mut h);
    let id = d.add_feature(FeatureKind::Hole(h));
    (d, id)
}

fn cosmetico(ajusta: impl Fn(&mut Hole)) -> f64 {
    let (d, _) = bloco(|h| {
        ajusta(h);
        h.thread_modeled = false;
    });
    volume_total(&recompute(&d))
}

fn passante(h: &mut Hole) {
    h.depth = Extent::ThroughAll;
}

fn escareado(h: &mut Hole) {
    h.kind = HoleKind::Escareado { diameter: s("26"), angle: s("90") };
}

fn rebaixado(h: &mut Hole) {
    h.kind = HoleKind::Rebaixado { diameter: s("20"), depth: s("6") };
}

/// Recalcula, exige status Ok (sem aviso de rosca que não coube ou não cortou) e sólido válido.
fn calcula(nome: &str, d: &Document, id: Id) -> fenkra_solidos::RecomputeResult {
    let r = recompute(d);
    assert!(matches!(r.status[&id], Status::Ok), "{nome}: {:?}", r.status[&id]);
    let f = final_shape(d).expect("forma final");
    assert!(f.is_valid(), "{nome}: o sólido saiu inválido");
    r
}

fn sulco_de(nome: &str, ajusta: impl Fn(&mut Hole)) -> f64 {
    let sem = cosmetico(&ajusta);
    let (d, id) = bloco(|h| ajusta(h));
    let r = calcula(nome, &d, id);
    // a caixa do kernel conta os polos das B-splines do sulco (passa de 30 em alguns milímetros); a da malha é a real
    let (lo, hi) = caixa_da_malha(&r);
    assert!((hi[0] - lo[0] - 60.).abs() < 0.05 && (hi[1] - lo[1] - 60.).abs() < 0.05 && (hi[2] - lo[2] - 30.).abs() < 0.05, "{nome}: a caixa mudou: {lo:?} {hi:?}");
    sem - volume_total(&r)
}

/// Direção média (unitária) dos vértices da malha sobre o plano da boca, entre o raio da broca e o do
/// sulco: é o centro do arco que o sulco abre na boca. `None` se o sulco não chega à boca.
fn centro_do_sulco_na_boca(r: &fenkra_solidos::RecomputeResult) -> Option<([f64; 2], f64)> {
    let (mut sx, mut sy, mut n) = (0., 0., 0);
    let mut angulos = vec![];
    for b in &r.bodies {
        for p in b.mesh.positions.chunks_exact(3) {
            let (x, y, z) = (p[0] as f64, p[1] as f64, p[2] as f64);
            let rad = (x * x + y * y).sqrt();
            if (z - TOPO).abs() < 1e-3 && rad > 7.05 && rad < 8.1 {
                sx += x / rad;
                sy += y / rad;
                n += 1;
                angulos.push(y.atan2(x).to_degrees());
            }
        }
    }
    if n == 0 {
        return None;
    }
    // largura angular do arco em volta da média
    let centro = sy.atan2(sx).to_degrees();
    let aberto = angulos.iter().map(|a| ((a - centro + 540.).rem_euclid(360.) - 180.).abs()).fold(0., f64::max);
    Some(([sx / n as f64, sy / n as f64], aberto * 2.))
}

fn angulo_entre(a: [f64; 2], b: [f64; 2]) -> f64 {
    let (na, nb) = ((a[0] * a[0] + a[1] * a[1]).sqrt(), (b[0] * b[0] + b[1] * b[1]).sqrt());
    ((a[0] * b[0] + a[1] * b[1]) / (na * nb)).clamp(-1., 1.).acos().to_degrees()
}

// ------------------------------------------------------------------ defeitos 1 e 2: sulco contínuo e entrada na boca

#[test]
fn furo_cego_m16x2_gera_sulco_valido_sem_aviso() {
    if !kernel_disponivel() {
        return;
    }
    // 9 voltas de um V de 1 mm de fundo sobre a parede de Ø14: cerca de 255 mm³
    let v = sulco_de("cego", |_| {});
    assert!(v > 200. && v < 320., "o sulco do furo cego tirou {v} mm³");
}

#[test]
fn sulco_atravessa_a_boca_e_nao_comeca_dentro_do_furo() {
    if !kernel_disponivel() {
        return;
    }
    // com a ponta da hélice cortada na boca, o arco aberto na face de cima ficava só de um lado do ponto
    // de início (meia volta de arco); com a hélice vindo de fora o arco é simétrico em volta do ângulo
    let (d, id) = bloco(|_| {});
    let r = calcula("boca", &d, id);
    let (centro, largura) = centro_do_sulco_na_boca(&r).expect("o sulco precisa chegar à boca do furo");
    // referência automática = +X, ângulo 0 em torno de -Z (o furo entra no material)
    assert!(angulo_entre(centro, [1., 0.]) < 12., "o sulco devia cruzar a boca em 0°: centro {centro:?}");
    assert!(largura > 150., "o arco do sulco na boca tem só {largura:.0}°: a entrada está cortada em degrau");
}

#[test]
fn passante_m16x2_atravessa_a_peca_inteira() {
    if !kernel_disponivel() {
        return;
    }
    let v = sulco_de("passante", passante);
    assert!(v > 380. && v < 560., "o sulco do furo passante tirou {v} mm³");
}

#[test]
fn sulco_nao_deixa_faces_soltas() {
    if !kernel_disponivel() {
        return;
    }
    // antes, cada emenda entre trechos de duas voltas deixava duas membranas planas sem espessura
    // (de 0,58 mm²) dentro do sulco. Faces planas que sobram: as seis do bloco, o fundo do furo cego e a
    // tampa onde o sulco termina (no passante a hélice sai pelo fundo da peça e não há tampa)
    for (nome, ajusta, planas) in [("cego", (|_: &mut Hole| {}) as fn(&mut Hole), 8), ("passante", passante, 6)] {
        let (d, id) = bloco(ajusta);
        let _ = calcula(nome, &d, id);
        let f = final_shape(&d).unwrap();
        let t = f.topology().unwrap();
        let n_planas = t.faces.iter().filter(|x| x.kind == 0).count();
        assert_eq!(n_planas, planas, "{nome}: faces planas soltas no sulco ({} faces no total)", t.faces.len());
        let pequenas = t.faces.iter().filter(|x| x.kind == 0 && x.area < 5.).count();
        assert!(pequenas <= 1, "{nome}: {pequenas} faces planas pequenas");
    }
}

// ------------------------------------------------------------------ defeito 3: ponta, rosca e folga

#[test]
fn ponta_profundidade_de_rosca_e_folga_nao_falham() {
    if !kernel_disponivel() {
        return;
    }
    let v = sulco_de("ponta+rosca+folga", |h| {
        h.tip = Some(s("118"));
        h.extra.thread_depth = s("14");
        h.extra.thread_relief = s("1");
    });
    assert!(v > 100., "o sulco com ponta tirou {v} mm³");
    // a profundidade de rosca igual à do furo, com ponta e deslocamento negativo da ponta
    let v = sulco_de("ponta+deslocamento", |h| {
        h.tip = Some(s("118"));
        h.extra.tip_offset = s("-4.2");
        h.extra.thread_depth = s("18");
    });
    assert!(v > 150., "o sulco tirou {v} mm³");
}

#[test]
fn folga_embaixo_em_passos_mm_e_porcentagem_dao_o_mesmo_fim() {
    if !kernel_disponivel() {
        return;
    }
    let base = sulco_de("sem folga", |_| {});
    // 2 passos = 4 mm = 20% de 20 mm
    let a = sulco_de("2 passos", |h| h.extra.thread_relief = s("2"));
    let b = sulco_de("4 mm", |h| {
        h.extra.thread_relief = s("4");
        h.extra.thread_relief_unit = Some(UnidadeMedida::Mm);
    });
    let c = sulco_de("20%", |h| {
        h.extra.thread_relief = s("20");
        h.extra.thread_relief_unit = Some(UnidadeMedida::Porcento);
    });
    assert!(a < base - 40., "a folga embaixo devia encurtar o sulco: {base} -> {a}");
    assert!((a - b).abs() < 3. && (a - c).abs() < 3., "mesma folga em unidades diferentes: {a} {b} {c}");
}

// ------------------------------------------------------------------ recurso 4: entrada

fn volume_final(nome: &str, ajusta: impl Fn(&mut Hole)) -> f64 {
    let (d, id) = bloco(ajusta);
    volume_total(&calcula(nome, &d, id))
}

#[test]
fn chanfro_de_entrada_em_mm_e_em_porcentagem() {
    if !kernel_disponivel() {
        return;
    }
    let sem = volume_final("sem chanfro", |_| {});
    // cone de 45° de c mm: pi * c * (r * c + c^2 / 3), com r = 7. Parte do sulco já estava dentro do cone
    // (uns 15 mm³ a 1,5 mm), então o volume a mais é o cone menos essa parte
    let cone = |c: f64| PI * c * (7. * c + c * c / 3.);
    let mm = volume_final("chanfro 1,5 mm", |h| h.extra.thread_chamfer = s("1.5"));
    let tirou = sem - mm;
    assert!(tirou > cone(1.5) - 25. && tirou < cone(1.5) + 0.5, "chanfro de 1,5 mm tirou {tirou} mm³ (cone: {})", cone(1.5));
    // 100% da profundidade do filete (0,6134 * 2 = 1,2268 mm): chega ao diâmetro externo da rosca
    let pc = volume_final("chanfro 100%", |h| {
        h.extra.thread_chamfer = s("100");
        h.extra.thread_chamfer_unit = UnidadeMedida::Porcento;
    });
    let tirou = sem - pc;
    assert!(tirou > cone(1.2268) - 20. && tirou < cone(1.2268) + 0.5, "chanfro de 100% tirou {tirou} mm³ (cone: {})", cone(1.2268));
    // o mesmo chanfro em mm: 1,2268
    let igual = volume_final("chanfro 1,2268 mm", |h| h.extra.thread_chamfer = s("1.2268"));
    assert!((igual - pc).abs() < 0.5, "100% do filete e 1,2268 mm devem dar o mesmo: {igual} {pc}");
}

#[test]
fn folga_em_cima_deixa_a_boca_lisa() {
    if !kernel_disponivel() {
        return;
    }
    let sem = sulco_de("sem folga", |_| {});
    let (d, id) = bloco(|h| h.extra.thread_top_gap = s("4"));
    let r = calcula("folga em cima", &d, id);
    assert!(centro_do_sulco_na_boca(&r).is_none(), "com 4 mm de folga em cima a boca devia ficar lisa");
    let com = sulco_de("folga em cima", |h| h.extra.thread_top_gap = s("4"));
    // cerca de 2 voltas a menos (255 mm³ em 9 voltas)
    assert!(com < sem - 40. && com > 60., "folga em cima: {sem} -> {com}");
    // 20% da profundidade útil (20 mm) = 4 mm
    let pc = sulco_de("20%", |h| {
        h.extra.thread_top_gap = s("20");
        h.extra.thread_top_gap_unit = UnidadeMedida::Porcento;
    });
    assert!((pc - com).abs() < 3., "4 mm e 20% de 20 mm: {com} {pc}");
}

// ------------------------------------------------------------------ recurso 5: ângulo de início

#[test]
fn angulo_de_inicio_gira_o_sulco_sem_mudar_o_volume() {
    if !kernel_disponivel() {
        return;
    }
    let mut vols = vec![];
    for ang in [0., 120., 240.] {
        let (d, id) = bloco(|h| h.extra.thread_start_angle = format!("{ang}"));
        let r = calcula(&format!("{ang}°"), &d, id);
        vols.push(volume_total(&r));
        let (centro, _) = centro_do_sulco_na_boca(&r).expect("sulco na boca");
        // o furo entra por -Z: a regra da mão direita em torno de -Z gira no sentido horário visto de cima
        let esperado = gira([1., 0., 0.], [0., 0., -1.], ang);
        assert!(angulo_entre(centro, [esperado[0], esperado[1]]) < 12., "ângulo {ang}: o sulco cruza a boca em {centro:?}, esperado {esperado:?}");
    }
    assert!((vols[0] - vols[1]).abs() < 1. && (vols[0] - vols[2]).abs() < 1., "o ângulo não muda o volume: {vols:?}");
}

#[test]
fn angulo_de_inicio_conta_a_partir_da_referencia_escolhida() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(|h| {
        h.extra.thread_start_ref = Some(AxisRef::Origin(OriginAxis::Y));
        h.extra.thread_start_angle = s("120");
    });
    let r = calcula("referência Y", &d, id);
    let (centro, _) = centro_do_sulco_na_boca(&r).expect("sulco na boca");
    let esperado = gira([0., 1., 0.], [0., 0., -1.], 120.);
    assert!(angulo_entre(centro, [esperado[0], esperado[1]]) < 12., "centro {centro:?} esperado {esperado:?}");
}

#[test]
fn rosca_a_esquerda_cruza_a_boca_no_mesmo_angulo() {
    if !kernel_disponivel() {
        return;
    }
    let (d, id) = bloco(|h| {
        h.left_hand = true;
        h.extra.thread_start_angle = s("120");
    });
    let r = calcula("esquerda", &d, id);
    let (centro, largura) = centro_do_sulco_na_boca(&r).expect("sulco na boca");
    let esperado = gira([1., 0., 0.], [0., 0., -1.], 120.);
    assert!(angulo_entre(centro, [esperado[0], esperado[1]]) < 12., "centro {centro:?} esperado {esperado:?}");
    assert!(largura > 150., "arco de {largura:.0}°");
    let v = sulco_de("esquerda", |h| h.left_hand = true);
    assert!(v > 200. && v < 320., "{v}");
}

// ------------------------------------------------------------------ a escada de tentativas nunca desiste

#[test]
fn todas_as_combinacoes_cortam_e_ficam_validas() {
    if !kernel_disponivel() {
        return;
    }
    let bocas: [(&str, fn(&mut Hole)); 3] = [("simples", |_| {}), ("escareado", escareado), ("rebaixado", rebaixado)];
    for (nb, boca) in bocas {
        for (fundo, e_passante) in [("cego", false), ("passante", true)] {
            for ang in [0., 120., 300.] {
                let nome = format!("{nb} {fundo} {ang}°");
                let (d, id) = bloco(|h| {
                    boca(h);
                    if e_passante {
                        passante(h);
                    }
                    h.extra.thread_start_angle = format!("{ang}");
                });
                let _ = calcula(&nome, &d, id);
                let sem = cosmetico(|h| {
                    boca(h);
                    if e_passante {
                        passante(h);
                    }
                });
                let v = sem - volume_total(&recompute(&d));
                assert!(v > 100., "{nome}: o sulco tirou só {v} mm³");
            }
        }
    }
}

// ------------------------------------------------------------------ exportação fechada

/// Soldando os vértices (1 µm), toda aresta do STL pertence a exatamente dois triângulos.
fn stl_fechado(caminho: &std::path::Path) -> Result<usize, String> {
    let bytes = std::fs::read(caminho).map_err(|e| e.to_string())?;
    let mut tris: Vec<[[f64; 3]; 3]> = vec![];
    if bytes.starts_with(b"solid") && bytes.windows(6).any(|w| w == b"facet ") {
        let texto = String::from_utf8_lossy(&bytes);
        let mut v: Vec<[f64; 3]> = vec![];
        for l in texto.lines() {
            let l = l.trim();
            if let Some(r) = l.strip_prefix("vertex") {
                let n: Vec<f64> = r.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                v.push([n[0], n[1], n[2]]);
                if v.len() == 3 {
                    tris.push([v[0], v[1], v[2]]);
                    v.clear();
                }
            }
        }
    } else {
        let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        for i in 0..n {
            let o = 84 + i * 50 + 12;
            let f = |k: usize| f32::from_le_bytes(bytes[o + k * 4..o + k * 4 + 4].try_into().unwrap()) as f64;
            tris.push([[f(0), f(1), f(2)], [f(3), f(4), f(5)], [f(6), f(7), f(8)]]);
        }
    }
    let chave = |p: [f64; 3]| [(p[0] * 1e3).round() as i64, (p[1] * 1e3).round() as i64, (p[2] * 1e3).round() as i64];
    let mut arestas: HashMap<([i64; 3], [i64; 3]), u32> = HashMap::new();
    for t in &tris {
        for i in 0..3 {
            let (a, b) = (chave(t[i]), chave(t[(i + 1) % 3]));
            if a == b {
                continue;
            }
            *arestas.entry(if a < b { (a, b) } else { (b, a) }).or_default() += 1;
        }
    }
    let abertas = arestas.values().filter(|&&n| n != 2).count();
    if abertas > 0 {
        return Err(format!("{abertas} arestas de {} sem par", arestas.len()));
    }
    Ok(tris.len())
}

#[test]
fn stl_exportado_e_fechado_com_chanfro_angulo_e_passante() {
    if !kernel_disponivel() {
        return;
    }
    let pasta = std::env::temp_dir().join("fenkra_b373");
    std::fs::create_dir_all(&pasta).unwrap();
    let casos: [(&str, fn(&mut Hole)); 2] = [
        ("cego_chanfro_120", |h| {
            h.extra.thread_chamfer = s("1");
            h.extra.thread_start_angle = s("120");
        }),
        ("passante_folga_topo", |h| {
            h.depth = Extent::ThroughAll;
            h.extra.thread_top_gap = s("2");
        }),
    ];
    for (nome, ajusta) in casos {
        let (d, id) = bloco(ajusta);
        let _ = calcula(nome, &d, id);
        let f = final_shape(&d).unwrap();
        let arq = pasta.join(format!("{nome}.stl"));
        f.export_stl(arq.to_str().unwrap(), 0.02, 0.2).expect("exportar STL");
        match stl_fechado(&arq) {
            Ok(n) => println!("{nome}: STL fechado com {n} triângulos em {}", arq.display()),
            Err(e) => panic!("{nome}: STL aberto: {e}"),
        }
    }
}

// ------------------------------------------------------------------ arquivos antigos

#[test]
fn arquivo_antigo_sem_os_campos_novos_abre_igual() {
    // o bloco `extra` de antes do B373 não tem unidade, folga em cima, chanfro nem ângulo
    let antigo = r#"{"thread_depth":"","thread_relief":"1.5","tip_offset":""}"#;
    let e: HoleExtra = serde_json::from_str(antigo).expect("o formato antigo continua abrindo");
    assert_eq!(e.thread_relief, "1.5");
    assert_eq!(e.thread_relief_unit, None, "sem unidade gravada, a folga continua em passos");
    assert!(e.thread_top_gap.is_empty() && e.thread_chamfer.is_empty() && e.thread_start_angle.is_empty() && e.thread_start_ref.is_none());
    // e o furo antigo, recalculado, tira o sulco de sempre
    let (d, id) = bloco(|h| h.extra = e.clone());
    let r = calcula("antigo", &d, id);
    assert!(volume_total(&r) < cosmetico(|h| h.extra = e.clone()) - 100.);
}
