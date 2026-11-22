// SPDX-License-Identifier: LGPL-2.1-or-later
//! B574: encaixe de tubo com corte normal à parede (boca de lobo). Documentos montados em código:
//! um tubo que recebe (corner Nenhum) e outro que chega em T ou Y com `fit.cut = Normal`. Confere
//! status, volume, caixa envolvente e a espessura da parede medida no sólido do kernel na borda de
//! corte; compara com o corte por booleana, que deixa a ponta fina.

use fenkra_core::*;
use fenkra_solidos::st_boca_lobo::{opcoes_do_fit, tolerancia_de_vao, validar};
use fenkra_solidos::st_encaixe::{calcular, CorteEncaixe, Membro, Opcoes, Secao};
use fenkra_solidos::{recompute, structural_members, MemberSolid, Status};

fn s(x: &str) -> String {
    x.to_string()
}

fn linha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
    CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }))
}

fn tubo(paths: Vec<CurveRef>, kind: SectionKind, dims: &[&str], corner: CornerMode, fit: FitOptions) -> StructMember {
    StructMember {
        paths,
        family: s("Tubo"),
        designation: s("teste"),
        kind,
        dims: dims.iter().map(|x| s(x)).collect(),
        anchor: 0,
        angle: s("0"),
        off_x: s("0"),
        off_y: s("0"),
        ext1: s("0"),
        ext2: s("0"),
        corner,
        gap: s("0"),
        joints: vec![],
        material: String::new(),
        mirror: false,
        tube_bend: None,
        fit,
    }
}

const REDONDO_88: (SectionKind, &[&str]) = (SectionKind::TuboRedondo, &["88.9", "4"]);
const REDONDO_60: (SectionKind, &[&str]) = (SectionKind::TuboRedondo, &["60.3", "3.6"]);
const QUADRADO_40: (SectionKind, &[&str]) = (SectionKind::TuboQuadrado, &["40", "3"]);
const RETANGULAR_60X30: (SectionKind, &[&str]) = (SectionKind::TuboRetangular, &["60", "30", "3"]);

/// O tubo que recebe passa em Z pelo nó; o que chega sai do nó a `graus` do eixo dele, no plano XZ.
/// Devolve o documento, o comando do que recebe e o do que chega.
fn t_doc(host: (SectionKind, &[&str]), br: (SectionKind, &[&str]), graus: f64, fit: FitOptions) -> (Document, Id, Id) {
    let mut d = Document::new("boca");
    let l1 = linha(&mut d, [0., 0., -400.], [0., 0., 600.]);
    let a = graus.to_radians();
    let l2 = linha(&mut d, [0., 0., 0.], [400. * a.sin(), 0., 400. * a.cos()]);
    let h = d.add_feature(FeatureKind::StructMember(tubo(vec![l1], host.0, host.1, CornerMode::Nenhum, FitOptions::default())));
    let b = d.add_feature(FeatureKind::StructMember(tubo(vec![l2], br.0, br.1, CornerMode::Encaixe, fit)));
    (d, h, b)
}

fn normal() -> FitOptions {
    FitOptions { cut: FitCut::Normal, ..Default::default() }
}

fn sol(d: &Document, f: Id) -> MemberSolid {
    structural_members(d).into_iter().find(|x| x.feature == f).expect("sólido do membro")
}

fn status_de(d: &Document, f: Id) -> Status {
    recompute(d).status.get(&f).cloned().expect("status")
}

/// Membro como o `st_encaixe` quer, no referencial do sólido.
fn membro_do_solido(m: &MemberSolid, secao: Secao, comprimento: f64, no: [f64; 3], para_dentro: [f64; 3]) -> Membro {
    Membro { secao, ponto: no, x: m.frame.x, y: m.frame.y(), para_dentro, centro: [0.; 2], comprimento }
}

/// Curva de corte calculada fora do recompute, com os mesmos dados e as opções do `fit`.
fn curva(branch: &MemberSolid, host: &MemberSolid, sb: Secao, sh: Secao, fit: &FitOptions) -> CorteEncaixe {
    let eu = membro_do_solido(branch, sb, 400., branch.frame.origin, branch.frame.normal);
    let h = membro_do_solido(host, sh, 1000., host.frame.origin, host.frame.normal);
    calcular(&eu, &[h], opcoes_do_fit(fit, Opcoes::default())).expect("curva")
}

fn mundo(m: &MemberSolid, p: [f64; 3]) -> [f64; 3] {
    let (x, y, z, o) = (m.frame.x, m.frame.y(), m.frame.normal, m.frame.origin);
    [o[0] + x[0] * p[0] + y[0] * p[1] + z[0] * p[2], o[1] + x[1] * p[0] + y[1] * p[1] + z[1] * p[2], o[2] + x[2] * p[0] + y[2] * p[1] + z[2] * p[2]]
}

/// Medida na borda de corte do sólido, num plano que contém o eixo e a direção `dir` (no plano da
/// seção, referencial local), afastado `desloc` para o lado. Devolve `(z da borda, espessura da
/// parede a `eps` da borda, pontos da borda)`. A espessura é a soma dos trechos de material que a
/// reta `z = borda + eps` atravessa no lado de `dir` positivo.
fn borda(m: &MemberSolid, dir: [f64; 2], desloc: f64, eps: f64) -> (f64, f64, Vec<[f64; 3]>) {
    let n = [-dir[1], dir[0], 0.];
    let o = [n[0] * desloc, n[1] * desloc, 0.];
    let loops = m.shape.section_loops(o, n, 2000.).expect("seção pelo plano do eixo");
    let rho = |p: &[f64; 3]| p[0] * dir[0] + p[1] * dir[1];
    let mut melhor = None;
    for l in &loops {
        let media = l.iter().map(rho).sum::<f64>() / l.len() as f64;
        if media <= 0. {
            continue;
        }
        let zmin = l.iter().map(|p| p[2]).fold(f64::MAX, f64::min);
        let z0 = zmin + eps;
        let mut xs = vec![];
        for k in 0..l.len() {
            let (a, b) = (l[k], l[(k + 1) % l.len()]);
            if (a[2] - z0) * (b[2] - z0) < 0. {
                let f = (z0 - a[2]) / (b[2] - a[2]);
                xs.push(rho(&a) + (rho(&b) - rho(&a)) * f);
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        let esp: f64 = xs.chunks_exact(2).map(|p| p[1] - p[0]).sum();
        let pts: Vec<[f64; 3]> = l.iter().filter(|p| p[2] <= zmin + 0.02).copied().collect();
        melhor = Some((zmin, esp, pts));
    }
    melhor.expect("laço da parede no plano")
}

/// Distância (na normal) de um ponto do mundo à superfície de um tubo redondo que passa por Z em
/// (0, 0): positiva fora (vão), negativa dentro (interferência).
fn dist_cilindro_z(p: [f64; 3], raio: f64) -> f64 {
    p[0].hypot(p[1]) - raio
}

/// Maior vão e maior interferência medidos nos pontos da borda do sólido contra o tubo redondo que
/// passa por Z de raio `raio`, em todos os ângulos de `n` planos pelo eixo.
fn vao_e_interferencia(m: &MemberSolid, raio: f64, n: usize) -> (f64, f64, f64, f64) {
    let (mut vao, mut interf, mut esp_min, mut zmin_geral) = (0f64, 0f64, f64::MAX, f64::MAX);
    for k in 0..n {
        let th = std::f64::consts::TAU * k as f64 / n as f64;
        let (z, e, pts) = borda(m, [th.cos(), th.sin()], 0., 0.05);
        esp_min = esp_min.min(e);
        zmin_geral = zmin_geral.min(z);
        // do par de pontos da borda (pele externa e interna), a pele que fica mais longe do tubo é o vão
        let ds: Vec<f64> = pts.iter().map(|p| dist_cilindro_z(mundo(m, *p), raio)).collect();
        vao = vao.max(ds.iter().cloned().fold(f64::MIN, f64::max).max(0.));
        interf = interf.max((-ds.iter().cloned().fold(f64::MAX, f64::min)).max(0.));
    }
    (vao, interf, esp_min, zmin_geral)
}

#[test]
fn t_redondo_a_45_e_60_mantem_a_espessura_da_parede_na_borda() {
    for graus in [45., 60.] {
        let (d, h, b) = t_doc(REDONDO_88, REDONDO_60, graus, normal());
        let st = status_de(&d, b);
        assert!(!matches!(st, Status::Error(_)), "{graus}°: {st:?}");
        if let Status::Warning(w) = &st {
            assert!(!w.contains("indisponível"), "o corte normal tem de valer, não cair na booleana: {w}");
        }
        let (mb, mh) = (sol(&d, b), sol(&d, h));
        let c = curva(&mb, &mh, Secao::Redonda { d: 60.3, t: 3.6 }, Secao::Redonda { d: 88.9, t: 4. }, &normal());
        // volume: (R² − r²)/2 · ∫(L − λ) dθ, com os mesmos λ da curva
        let (ro, ri) = (60.3 / 2., 60.3 / 2. - 3.6);
        let n = c.pontos.len();
        let dth = std::f64::consts::TAU / n as f64;
        let esperado: f64 = (ro * ro - ri * ri) / 2. * c.pontos.iter().map(|p| 400. - p.lambda).sum::<f64>() * dth;
        let vol = mb.signature.volume as f64;
        assert!((vol - esperado).abs() / esperado < 0.004, "{graus}°: volume {vol} esperado {esperado}");
        // caixa: a ponta de fora para no fim da linha (400 mm); a de dentro, no menor λ da curva
        assert_eq!(mb.signature.bbox[5], 4000, "{graus}°: caixa {:?}", mb.signature.bbox);
        let zmin = mb.signature.bbox[2] as f64 / 10.;
        assert!((zmin - c.lambda_min()).abs() < 0.6, "{graus}°: caixa z {zmin} curva {}", c.lambda_min());
        // espessura da parede na borda, medida no sólido do kernel, em todos os ângulos
        let (vao, interf, esp_min, _) = vao_e_interferencia(&mb, 88.9 / 2., 48);
        assert!(esp_min > 3.6 * 0.97, "{graus}°: espessura mínima na borda {esp_min:.3} (parede 3,6)");
        // vão e interferência do kernel contra o validado pela curva
        let v = validar(&c, tolerancia_de_vao(Secao::Redonda { d: 60.3, t: 3.6 }, 1.5));
        assert!(interf < 0.05 && v.interferencia_max < 0.05, "{graus}°: interferência kernel {interf:.3} curva {:.3}", v.interferencia_max);
        assert!((vao - v.vao_max).abs() < 0.4, "{graus}°: vão kernel {vao:.2} curva {:.2}", v.vao_max);
        assert!(v.vao_max <= tolerancia_de_vao(Secao::Redonda { d: 60.3, t: 3.6 }, 1.5), "{graus}°: vão {:.2}", v.vao_max);
        assert!(v.espessura_borda >= 3.6 - 1e-9 && v.angulo_ponta_min > 89.9);
        println!("{graus}°: espessura borda {esp_min:.3} mm, vão {vao:.2} (curva {:.2}), interferência {interf:.3}, volume {vol:.0}/{esperado:.0}, zmin {zmin:.2}", v.vao_max);
    }
}

#[test]
fn a_booleana_deixa_ponta_fina_e_o_corte_normal_nao() {
    let (d, _, b) = t_doc(REDONDO_88, REDONDO_60, 45., FitOptions::default());
    let mb = sol(&d, b);
    let (_, _, esp_bool, _) = vao_e_interferencia(&mb, 88.9 / 2., 48);
    let (d, _, b) = t_doc(REDONDO_88, REDONDO_60, 45., normal());
    let mn = sol(&d, b);
    let (_, _, esp_norm, _) = vao_e_interferencia(&mn, 88.9 / 2., 48);
    println!("espessura mínima da borda a 0,05 mm: booleana {esp_bool:.3}, normal {esp_norm:.3}");
    assert!(esp_bool < 0.5 * 3.6, "a booleana a 45° termina em ponta fina: {esp_bool}");
    assert!(esp_norm > 0.97 * 3.6, "o corte normal mantém a parede: {esp_norm}");
}

#[test]
fn referencias_de_parede_e_de_face_mudam_a_curva_e_a_validacao_confere_com_o_kernel() {
    let sb = Secao::Redonda { d: 60.3, t: 3.6 };
    let sh = Secao::Redonda { d: 88.9, t: 4. };
    let mut zs = vec![];
    for (nome, wall, face) in [
        ("auto/externa", FitWall::Auto, FitFace::Outer),
        ("parede externa", FitWall::Outer, FitFace::Outer),
        ("parede interna", FitWall::Inner, FitFace::Outer),
        ("parede média", FitWall::Middle, FitFace::Outer),
        ("auto, face média do tubo que recebe", FitWall::Auto, FitFace::Middle),
        ("auto, face interna do tubo que recebe", FitWall::Auto, FitFace::Inner),
    ] {
        let fit = FitOptions { cut: FitCut::Normal, wall, face };
        let (d, h, b) = t_doc(REDONDO_88, REDONDO_60, 45., fit);
        let st = status_de(&d, b);
        assert!(!matches!(st, Status::Error(_)), "{nome}: {st:?}");
        let (mb, mh) = (sol(&d, b), sol(&d, h));
        let c = curva(&mb, &mh, sb, sh, &fit);
        let v = validar(&c, tolerancia_de_vao(sb, 1.5));
        // o raio de referência do tubo que recebe: a face escolhida
        let r_ref = match face {
            FitFace::Outer => 88.9 / 2.,
            FitFace::Middle => 88.9 / 2. - 2.,
            FitFace::Inner => 88.9 / 2. - 4.,
        };
        let (vao, interf, esp, zmin) = vao_e_interferencia(&mb, r_ref, 48);
        println!("{nome}: vão kernel {vao:.2} curva {:.2} · interferência kernel {interf:.2} curva {:.2} · espessura {esp:.2} · z {zmin:.2}", v.vao_max, v.interferencia_max);
        assert!(esp > 0.97 * 3.6, "{nome}: espessura {esp}");
        assert!((vao - v.vao_max).abs() < 0.4, "{nome}: vão kernel {vao:.2} curva {:.2}", v.vao_max);
        assert!((interf - v.interferencia_max).abs() < 0.4, "{nome}: interferência kernel {interf:.2} curva {:.2}", v.interferencia_max);
        if wall == FitWall::Auto {
            assert!(interf < 0.05, "{nome}: o automático nunca invade ({interf})");
        }
        zs.push((nome, zmin, v.interferencia_max));
    }
    // uma pele só invade o tubo que recebe: a validação avisa
    assert!(zs[1].2 > 0.05 || zs[2].2 > 0.05, "alguma parede sozinha invade: {zs:?}");
    // a face interna deixa o tubo entrar mais fundo que a externa
    assert!(zs[5].1 < zs[0].1 - 1., "face interna entra mais: {zs:?}");
    assert!(zs[4].1 < zs[0].1 && zs[5].1 < zs[4].1, "a ordem externa > meio > interna: {zs:?}");
}

/// Volume esperado do tubo retangular cortado: integra (L − λ) sobre a área da parede, com λ lido
/// das amostras da pele externa da curva (cada ponto da parede usa o λ da face de que está mais perto).
fn volume_retangular(c: &CorteEncaixe, m: &MemberSolid, b: f64, h: f64, t: f64, comprimento: f64) -> f64 {
    let (hb, hh) = (b / 2., h / 2.);
    // amostras da pele externa em coordenadas locais: (x, y, λ)
    let am: Vec<(f64, f64, f64)> = c
        .pontos
        .iter()
        .map(|p| {
            let q = [p.p3[0] - m.frame.normal[0] * p.lambda - m.frame.origin[0], p.p3[1] - m.frame.normal[1] * p.lambda - m.frame.origin[1], p.p3[2] - m.frame.normal[2] * p.lambda - m.frame.origin[2]];
            let (x, y) = (m.frame.x, m.frame.y());
            (q[0] * x[0] + q[1] * x[1] + q[2] * x[2], q[0] * y[0] + q[1] * y[1] + q[2] * y[2], p.lambda)
        })
        .collect();
    // λ numa face: interpola entre as amostras dela, pela coordenada ao longo da face
    let lam_na_face = |lado: usize, coord: f64| -> f64 {
        let mut v: Vec<(f64, f64)> = am
            .iter()
            .filter(|(x, y, _)| match lado {
                0 => (x - hb).abs() < 1e-6,
                1 => (y - hh).abs() < 1e-6,
                2 => (x + hb).abs() < 1e-6,
                _ => (y + hh).abs() < 1e-6,
            })
            .map(|(x, y, l)| (if lado % 2 == 0 { *y } else { *x }, *l))
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
        let i = v.partition_point(|p| p.0 < coord).clamp(1, v.len() - 1);
        let (a, b) = (v[i - 1], v[i]);
        a.1 + (b.1 - a.1) * ((coord - a.0) / (b.0 - a.0)).clamp(0., 1.)
    };
    let passo = 0.2;
    let mut vol = 0.;
    let (nx, ny) = ((b / passo) as usize, (h / passo) as usize);
    for i in 0..nx {
        for j in 0..ny {
            let (x, y) = (-hb + (i as f64 + 0.5) * passo, -hh + (j as f64 + 0.5) * passo);
            let na_parede = x.abs() > hb - t || y.abs() > hh - t;
            if !na_parede {
                continue;
            }
            // face mais próxima da pele externa
            let d = [hb - x, hh - y, hb + x, hh + y];
            let lado = (0..4).min_by(|a, b| d[*a].total_cmp(&d[*b])).unwrap();
            let coord = if lado % 2 == 0 { y } else { x };
            vol += (comprimento - lam_na_face(lado, coord)) * passo * passo;
        }
    }
    vol
}

/// Nó de tubos: o 1.º recebe em Z, o 2.º (opcional) em Y e o que chega a 45° no plano XZ é cortado
/// por todos.
fn no_de_tres(fit: FitOptions, com_segundo: bool) -> (Document, Id) {
    let mut d = Document::new("no3");
    let l1 = linha(&mut d, [0., 0., -400.], [0., 0., 600.]);
    d.add_feature(FeatureKind::StructMember(tubo(vec![l1], REDONDO_88.0, REDONDO_88.1, CornerMode::Nenhum, FitOptions::default())));
    if com_segundo {
        let l3 = linha(&mut d, [0., -500., 0.], [0., 500., 0.]);
        d.add_feature(FeatureKind::StructMember(tubo(vec![l3], SectionKind::TuboRedondo, &["110", "4"], CornerMode::Nenhum, FitOptions::default())));
    }
    let a = 45f64.to_radians();
    let l2 = linha(&mut d, [0., 0., 0.], [400. * a.sin(), 0., 400. * a.cos()]);
    let br = d.add_feature(FeatureKind::StructMember(tubo(vec![l2], REDONDO_60.0, REDONDO_60.1, CornerMode::Encaixe, fit)));
    (d, br)
}

#[test]
fn no_com_dois_tubos_recebendo_corta_pelos_dois_sem_interferir_em_nenhum() {
    let (d1, b1) = no_de_tres(normal(), false);
    let (d2, b2) = no_de_tres(normal(), true);
    for (d, b) in [(&d1, b1), (&d2, b2)] {
        let st = status_de(d, b);
        assert!(!matches!(st, Status::Error(_)), "{st:?}");
    }
    let (m1, m2) = (sol(&d1, b1), sol(&d2, b2));
    // o segundo tubo tira material a mais: o corte de dois vizinhos vale o de maior alcance em cada ponto
    assert!(m2.signature.volume + 100 < m1.signature.volume, "um vizinho: {} dois vizinhos: {}", m1.signature.volume, m2.signature.volume);
    // nenhum ponto da borda entra em nenhum dos dois tubos: o 1.º segue Z (raio 44,45), o 2.º segue Y (raio 55)
    let mut interf = 0f64;
    let mut perto_do_2 = false;
    for k in 0..48 {
        let th = std::f64::consts::TAU * k as f64 / 48.;
        let (_, _, pts) = borda(&m2, [th.cos(), th.sin()], 0., 0.05);
        for p in pts {
            let w = mundo(&m2, p);
            let d_z = w[0].hypot(w[1]) - 88.9 / 2.;
            let d_y = w[0].hypot(w[2]) - 110. / 2.;
            interf = interf.max(-d_z).max(-d_y);
            if d_y < d_z + 0.3 {
                perto_do_2 = true;
            }
        }
    }
    assert!(interf < 0.05, "interferência com algum dos dois tubos: {interf:.3}");
    assert!(perto_do_2, "o segundo tubo segura a ponta em algum trecho");
    println!("nó de três tubos: volume {} -> {}, interferência {interf:.3}", m1.signature.volume, m2.signature.volume);
}

#[test]
fn barra_macica_cai_na_booleana_com_aviso_e_arquivo_antigo_nao_traz_o_campo() {
    let (d, _, b) = t_doc(REDONDO_88, (SectionKind::BarraRedonda, &["30"]), 45., normal());
    let st = status_de(&d, b);
    match &st {
        Status::Warning(w) => assert!(w.contains("indisponível") && w.contains("booleana"), "{w}"),
        o => panic!("esperava aviso, veio {o:?}"),
    }
    assert!(sol(&d, b).signature.volume > 0, "a ponta existe, cortada por booleana");
    // sem corte normal, nada novo vai ao arquivo
    let (d0, _, _) = t_doc(REDONDO_88, REDONDO_60, 45., FitOptions::default());
    let j0 = serde_json::to_string(&d0).unwrap();
    assert!(!j0.contains("\"fit\""), "arquivo sem encaixe normal não traz fit");
    let (d1, _, _) = t_doc(REDONDO_88, REDONDO_60, 45., normal());
    let j1 = serde_json::to_string(&d1).unwrap();
    assert!(j1.contains("\"fit\"") && j1.contains("Normal"));
    let volta: Document = serde_json::from_str(&j1).unwrap();
    assert_eq!(serde_json::to_string(&volta).unwrap(), j1);
}

#[test]
fn gabarito_planificado_da_boca_de_lobo_sai_na_lista_de_corte_e_bate_com_o_solido() {
    let (d, _, b) = t_doc(REDONDO_88, REDONDO_60, 45., normal());
    let m = sol(&d, b);
    // lista de corte: a ponta 1 aparece como encaixe
    let linhas = fenkra_solidos::cut_list(&d);
    let r = linhas.iter().find(|r| r.members.contains(&(b, 0))).expect("linha do membro na lista");
    assert!(r.encaixe1 && !r.encaixe2, "ponta 1 é encaixe, ponta 2 é reta");
    // o gabarito: a curva de corte planificada, com o mesmo λ do sólido
    let todos = fenkra_solidos::st_encaixe_doc::encaixes_da_estrutura(&d, &Opcoes::default());
    assert_eq!(todos.len(), 1);
    let membro = &todos[0];
    let ponta = membro.ponta1.as_ref().expect("ponta 1");
    assert_eq!(ponta.fit.cut, FitCut::Normal);
    let k = &ponta.corte;
    let zmin = m.signature.bbox[2] as f64 / 10.;
    assert!((k.lambda_min() - zmin).abs() < 0.6, "gabarito {} sólido {zmin}", k.lambda_min());
    // a pele externa desenrolada tem o comprimento da curva em 3D
    assert!((k.comprimento_corte() - k.comprimento_3d()).abs() / k.comprimento_3d() < 0.005, "plano {} 3D {}", k.comprimento_corte(), k.comprimento_3d());
    assert!(ponta.validacao.espessura_borda >= 3.6 - 1e-9 && ponta.validacao.interferencia_max < 0.05);
    let desenho = membro.desenho();
    assert!(desenho.titulo.contains("Boca de lobo"));
    let (x, v) = (fenkra_solidos::st_dobra_saida::dxf(&desenho), fenkra_solidos::st_dobra_saida::svg(&desenho));
    assert!(x.contains("CORTE") && v.contains("CORTE") && v.contains("polyline"));
    println!("gabarito: corte {:.1} mm, λ {:.1}..{:.1}, vão {:.2}", k.comprimento_corte(), k.lambda_min(), k.lambda_max(), ponta.validacao.vao_max);
}

#[test]
fn canto_em_l_recebe_o_corte_normal_pelo_painel_de_cantos() {
    // L de dois trechos no mesmo comando, canto de encaixe a 45°
    let mut d = Document::new("l");
    let l1 = linha(&mut d, [0., 0., 0.], [0., 0., 500.]);
    let a = 45f64.to_radians();
    let l2 = linha(&mut d, [0., 0., 0.], [400. * a.sin(), 0., 400. * a.cos()]);
    let f = d.add_feature(FeatureKind::StructMember(tubo(vec![l1, l2], REDONDO_60.0, REDONDO_60.1, CornerMode::Encaixe, FitOptions::default())));
    let antes = structural_members(&d).into_iter().find(|x| x.feature == f && x.index == 1).unwrap();
    let cantos = fenkra_solidos::member_corners(&d);
    assert_eq!(cantos.len(), 1);
    assert_eq!(cantos[0].fit, FitOptions::default());
    assert!(fenkra_solidos::st_cantos::definir_fit(&mut d, &cantos[0], Some(normal())));
    let depois_cantos = fenkra_solidos::member_corners(&d);
    assert_eq!(depois_cantos[0].fit.cut, FitCut::Normal, "a opção ficou no canto");
    assert_eq!(depois_cantos[0].modo, CornerMode::Encaixe);
    let st = status_de(&d, f);
    assert!(!matches!(st, Status::Error(_)), "{st:?}");
    let depois = structural_members(&d).into_iter().find(|x| x.feature == f && x.index == 1).unwrap();
    // o corte normal tira mais material que a booleana (a curva segue a pele que toca primeiro)
    assert!(depois.signature.volume < antes.signature.volume, "{} -> {}", antes.signature.volume, depois.signature.volume);
    let (_, esp_antes, _) = borda(&antes, [1., 0.], 0., 0.05);
    let (_, esp_depois, _) = borda(&depois, [1., 0.], 0., 0.05);
    assert!(esp_depois > 0.97 * 3.6 && esp_antes < esp_depois, "espessura {esp_antes:.2} -> {esp_depois:.2}");
    // mudar a folga do canto não perde a opção
    assert!(fenkra_solidos::st_cantos::definir_regra(&mut d, (f, 0), (f, 1), Some(CornerMode::Encaixe), Some((f, 0)), Some("1".into())));
    assert_eq!(fenkra_solidos::member_corners(&d)[0].fit.cut, FitCut::Normal);
    // voltar ao padrão do membro
    let c = fenkra_solidos::member_corners(&d).remove(0);
    assert!(fenkra_solidos::st_cantos::definir_fit(&mut d, &c, None));
    assert_eq!(fenkra_solidos::member_corners(&d)[0].fit, FitOptions::default());
}

#[test]
fn t_quadrado_e_retangular_mantem_a_espessura_e_fecha_com_o_volume() {
    for (nome, br, b, h, t) in [("quadrado 40x3", QUADRADO_40, 40., 40., 3.), ("retangular 60x30x3", RETANGULAR_60X30, 60., 30., 3.)] {
        for graus in [45., 60.] {
            let (d, hs, bs) = t_doc(REDONDO_88, br, graus, normal());
            let st = status_de(&d, bs);
            assert!(!matches!(st, Status::Error(_)), "{nome} {graus}°: {st:?}");
            if let Status::Warning(w) = &st {
                assert!(!w.contains("indisponível"), "{nome} {graus}°: {w}");
            }
            let (mb, mh) = (sol(&d, bs), sol(&d, hs));
            assert!(mb.shape.is_valid(), "{nome} {graus}°: sólido inválido");
            let sec = Secao::Retangular { b, h, t };
            let fit = normal();
            let c = curva(&mb, &mh, sec, Secao::Redonda { d: 88.9, t: 4. }, &fit);
            // espessura da parede no meio e perto dos cantos das quatro faces
            let (ex, ey) = (mb.frame.x, mb.frame.y());
            let _ = (ex, ey);
            let mut esp_min = f64::MAX;
            for dir in [[1., 0.], [-1., 0.], [0., 1.], [0., -1.]] {
                let meia_ao_longo = if dir[0] != 0. { h / 2. } else { b / 2. };
                for f in [-0.6, 0., 0.6] {
                    let (_, e, _) = borda(&mb, dir, f * (meia_ao_longo - t), 0.05);
                    esp_min = esp_min.min(e);
                }
            }
            assert!(esp_min > 0.97 * t, "{nome} {graus}°: espessura mínima na borda {esp_min:.3}");
            let esperado = volume_retangular(&c, &mb, b, h, t, 400.);
            let vol = mb.signature.volume as f64;
            assert!((vol - esperado).abs() / esperado < 0.01, "{nome} {graus}°: volume {vol} esperado {esperado:.0}");
            assert_eq!(mb.signature.bbox[5], 4000);
            let v = validar(&c, tolerancia_de_vao(sec, 1.5));
            assert!(v.interferencia_max < 0.05, "{nome} {graus}°: interferência {:.3}", v.interferencia_max);
            // a mesma face booleana (ponta fina) como comparação
            let (d2, _, b2) = t_doc(REDONDO_88, br, graus, FitOptions::default());
            let m2 = sol(&d2, b2);
            let mut esp_bool = f64::MAX;
            for dir in [[1., 0.], [-1., 0.], [0., 1.], [0., -1.]] {
                let (_, e, _) = borda(&m2, dir, 0., 0.05);
                esp_bool = esp_bool.min(e);
            }
            println!("{nome} {graus}°: espessura normal {esp_min:.3} booleana {esp_bool:.3} · vão {:.2} · volume {vol:.0}/{esperado:.0} · zmin {:.1}", v.vao_max, mb.signature.bbox[2] as f64 / 10.);
        }
    }
}
