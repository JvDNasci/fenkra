// SPDX-License-Identifier: LGPL-2.1-or-later
//! Medição manual (B393): tempo de carga e de projeção das vistas do conjunto do kart.
//! `cargo test -p fenkra-desenho --test medir_kart -- --ignored --nocapture`

use fenkra_desenho::source::Models;
use fenkra_desenho::*;
use std::time::Instant;

#[test]
#[ignore]
fn tempo_das_vistas_do_conjunto_do_kart() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let src = Source { path: "KART.fka".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("kart", Format::A1);
    let s = d.sheets[0].id;
    let mut m = Models::new(Some(dir));
    let t = Instant::now();
    let l = m.load(&src);
    println!("carga: {:?} pecas={} erro={:?}", t.elapsed(), l.pieces.len(), l.error);
    for dir in [Dir::Frente, Dir::Topo, Dir::Esquerda, Dir::Iso] {
        let v = d.add_view(s, src.clone(), ViewKind::Standard(dir), [100., 100.], 0.05);
        let t = Instant::now();
        let p = m.proj(&d, v);
        println!("{dir:?}: {:?} linhas={} erro={:?}", t.elapsed(), p.lines.len(), p.error);
    }
}

/// B393: linhas ocultas das quatro vistas, uma depois da outra, em paralelo e por malha.
/// `FENKRA_MEDIR_SEQ=1` mede também a sequencial (demora minutos).
#[test]
#[ignore]
fn hlr_paralelo_e_por_malha_no_kart() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let src = Source { path: "KART.fka".into(), display_state: String::new(), hidden: vec![] };
    let mut m = Models::new(Some(dir));
    let t = Instant::now();
    let forma = m.load(&src).shape.clone().expect("forma do kart");
    println!("carga: {:?}", t.elapsed());
    let eixos: Vec<_> = [Dir::Frente, Dir::Topo, Dir::Esquerda, Dir::Iso].iter().map(|d| source::Axes::of(*d)).collect();
    let vistas: Vec<_> = eixos.iter().map(|a| fenkra_kernel::VistaHlr { forma: &forma, para: a.toward, x: a.x, ocultas: false, prioridade_baixa: false }).collect();
    if std::env::var("FENKRA_MEDIR_SEQ").is_ok() {
        for a in &eixos {
            let t = Instant::now();
            let l = forma.hlr(a.toward, a.x, false).unwrap();
            println!("sequencial: {:?} linhas={}", t.elapsed(), l.len());
        }
    }
    fenkra_desenho::hlr_processos::definir_executavel(env!("CARGO_BIN_EXE_fenkra-hlr").into());
    let t = Instant::now();
    let r = fenkra_desenho::hlr_processos::exatas(&vistas, None).expect("processos");
    println!("exata em processos: {:?} linhas={:?}", t.elapsed(), r.iter().map(|x| x.as_ref().map_or(0, |l| l.len())).collect::<Vec<_>>());
    let (lo, hi) = forma.bbox().unwrap();
    let big = (0..3).map(|k| hi[k] - lo[k]).fold(0., f64::max);
    let t = Instant::now();
    let r = fenkra_kernel::hlr_varias(&vistas, Some(big * 0.0005));
    println!("por malha em paralelo: {:?} linhas={:?}", t.elapsed(), r.iter().map(|x| x.as_ref().map_or(0, |l| l.len())).collect::<Vec<_>>());
    assert!(r.iter().all(|x| x.as_ref().is_ok_and(|l| !l.is_empty())));
}

/// B393: o caminho da interface: quatro vistas (frente, duas projetadas e isométrica) pedidas no
/// mesmo quadro à thread de projeção. Mede quando chega a prévia e quando chega a exata.
#[test]
#[ignore]
fn quatro_vistas_do_kart_pela_thread_de_projecao() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let src = Source { path: "KART.fka".into(), display_state: String::new(), hidden: vec![] };
    let mut d = Drawing::new("kart", Format::A1);
    let s = d.sheets[0].id;
    let f = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [200., 300.], 0.05);
    let t = d.add_view(s, src.clone(), ViewKind::Projected { parent: f, dir: Dir::Topo }, [200., 150.], 0.05);
    let e = d.add_view(s, src.clone(), ViewKind::Projected { parent: f, dir: Dir::Esquerda }, [400., 300.], 0.05);
    let i = d.add_view(s, src, ViewKind::Standard(Dir::Iso), [400., 150.], 0.05);
    let vistas = [f, t, e, i];
    // a exata vai em processos, como no aplicativo
    fenkra_desenho::hlr_processos::definir_executavel(env!("CARGO_BIN_EXE_fenkra-hlr").into());
    let mut m = Models::new(Some(dir));
    m.ativar_fundo(|| {});
    let t0 = Instant::now();
    let (mut previa, mut exata) = (None, None);
    while exata.is_none() {
        m.sondar();
        let ps: Vec<_> = vistas.iter().map(|v| m.proj(&d, *v)).collect();
        if previa.is_none() && ps.iter().all(|p| p.previa || (!p.calculando && !p.lines.is_empty())) {
            previa = Some(t0.elapsed());
            println!("prévia das quatro: {:?} linhas={:?}", t0.elapsed(), ps.iter().map(|p| p.lines.len()).collect::<Vec<_>>());
        }
        if ps.iter().all(|p| !p.calculando && !p.previa) {
            exata = Some(t0.elapsed());
            println!("exata das quatro: {:?} linhas={:?} erros={:?}", t0.elapsed(), ps.iter().map(|p| p.lines.len()).collect::<Vec<_>>(), ps.iter().filter_map(|p| p.error.clone()).collect::<Vec<_>>());
            // duas threads na mesma topologia devolviam vistas vazias, sem erro
            assert!(ps.iter().all(|p| p.error.is_none() && !p.lines.is_empty()), "toda vista exata tem linhas");
        }
        assert!(t0.elapsed().as_secs() < 900, "não terminou");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Sonda: quatro cópias da mesma vista em paralelo devem dar o mesmo resultado da sequencial.
#[test]
#[ignore]
fn sonda_hlr_paralelo_deterministico() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let src = Source { path: "KART.fka".into(), display_state: String::new(), hidden: vec![] };
    let mut m = Models::new(Some(dir));
    let forma = m.load(&src).shape.clone().expect("forma do kart");
    let a = source::Axes::of(Dir::Iso);
    let vistas: Vec<_> = (0..4).map(|_| fenkra_kernel::VistaHlr { forma: &forma, para: a.toward, x: a.x, ocultas: false, prioridade_baixa: false }).collect();
    if std::env::var("FENKRA_SONDA_POLY").is_ok() {
        let um = fenkra_kernel::hlr_varias(&vistas[..1], Some(3.)).remove(0).unwrap().len();
        for _ in 0..3 {
            let r: Vec<_> = fenkra_kernel::hlr_varias(&vistas, Some(3.)).into_iter().map(|x| x.map_or(0, |l| l.len())).collect();
            println!("malha: uma={um} paralelo={r:?}");
        }
        return;
    }
    let seq = forma.hlr(a.toward, a.x, false).unwrap().len();
    for _ in 0..2 {
        let r: Vec<_> = fenkra_kernel::hlr_varias(&vistas, None).into_iter().map(|x| x.map_or(0, |l| l.len())).collect();
        println!("sequencial={seq} paralelo={r:?}");
    }
    let seq2 = forma.hlr(a.toward, a.x, false).unwrap().len();
    println!("sequencial de novo={seq2}");
}

/// Onde vai o tempo da carga da montagem (B393): calcular cada peça, posicionar ou juntar.
#[test]
#[ignore]
fn fases_da_carga_do_conjunto_do_kart() {
    use fenkra_montagem::load::{full_path, PartCache};
    use fenkra_montagem::view::{placed_shape, shown};
    use fenkra_montagem::{Assembly, NodeKind};
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let a = Assembly::from_json(&std::fs::read_to_string(dir.join("KART.fka")).unwrap()).unwrap();
    let mut cache = PartCache::default();
    cache.so_forma = true;
    let (mut t_peca, mut t_junta) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let mut lista = vec![];
    let mut n = 0;
    for nd in &a.nodes {
        let NodeKind::SubAssembly { path } = &nd.kind else { continue };
        let fp = full_path(Some(&dir), path);
        let sub = Assembly::from_json(&std::fs::read_to_string(&fp).unwrap()).unwrap();
        let sd = fp.parent().unwrap().to_path_buf();
        for pn in &sub.nodes {
            if !matches!(pn.kind, NodeKind::Part { .. }) || !shown(&sub, pn.id) {
                continue;
            }
            let t = Instant::now();
            let s = placed_shape(&sub, pn.id, Some(&sd), &mut cache);
            t_peca += t.elapsed();
            if let Some(s) = s {
                lista.push(s);
                n += 1;
            }
        }
    }
    let mut all = fenkra_kernel::Shape::empty();
    for s in &lista {
        let t = Instant::now();
        all = if all.is_null() { s.clone() } else { all.compound(s) };
        t_junta += t.elapsed();
    }
    let t = Instant::now();
    let _ = all.bbox();
    println!("bbox do aninhado: {:?}", t.elapsed());
    let _ = full_path(Some(&dir), "");
    println!("pecas diretas={n} calcular+posicionar={t_peca:?} juntar={t_junta:?}");
}

/// B393: carga das formas do kart em paralelo e, depois, do disco. Confere, peça a peça, que o
/// cálculo em threads dá o mesmo volume e a mesma caixa da sequencial (o recálculo dos comandos não
/// pode ter estado global escondido) e mede as cargas.
#[test]
#[ignore]
fn carga_do_kart_em_paralelo_e_do_disco() {
    use fenkra_montagem::formas_disco::{self, Pedido};
    use fenkra_montagem::load::arquivos_das_pecas;
    use fenkra_montagem::Assembly;
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let cache = std::env::temp_dir().join(format!("fenkra_medir_kart_formas_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    std::env::set_var("FENKRA_CACHE_FORMAS", &cache);
    let a = Assembly::from_json(&std::fs::read_to_string(dir.join("KART.fka")).unwrap()).unwrap();
    let mut arquivos = arquivos_das_pecas(&a, Some(&dir));
    arquivos.sort();
    arquivos.dedup();
    let pedidos = || -> Vec<Pedido> {
        arquivos
            .iter()
            .map(|f| {
                let texto = std::fs::read_to_string(f).unwrap();
                let doc = fenkra_core::Document::from_json(&texto).unwrap();
                Pedido { caminho: f.clone(), texto, doc }
            })
            .collect()
    };
    let medida = |s: &Option<fenkra_kernel::Shape>| s.as_ref().map(|s| (s.mass().volume, s.bbox().unwrap()));
    if std::env::var_os("FENKRA_MEDIR_SEQ").is_some() {
        std::env::set_var("FENKRA_SEM_CACHE_FORMAS", "1");
        let t = Instant::now();
        let seq = formas_disco::calcular(pedidos(), 1, None);
        let t_seq = t.elapsed();
        let t = Instant::now();
        let par = formas_disco::calcular(pedidos(), 0, None);
        let t_par = t.elapsed();
        std::env::remove_var("FENKRA_SEM_CACHE_FORMAS");
        assert_eq!(seq.len(), par.len());
        for ((f, a), (g, b)) in seq.iter().zip(&par) {
            assert_eq!(f, g);
            match (medida(a), medida(b)) {
                (Some((va, (la, ha))), Some((vb, (lb, hb)))) => {
                    assert!((va - vb).abs() <= 1e-6 * va.abs().max(1.), "{}: volume {va} x {vb}", f.display());
                    for k in 0..3 {
                        assert!((la[k] - lb[k]).abs() < 1e-6 && (ha[k] - hb[k]).abs() < 1e-6, "{}: caixa", f.display());
                    }
                }
                (None, None) => {}
                (x, y) => panic!("{}: {x:?} x {y:?}", f.display()),
            }
        }
        println!("pecas={} sequencial={t_seq:?} paralelo={t_par:?} (mesmos volumes e caixas)", arquivos.len());
    }

    let src = Source { path: "KART.fka".into(), display_state: String::new(), hidden: vec![] };
    let t = Instant::now();
    let mut m = Models::new(Some(dir.clone()));
    let l = m.load(&src);
    assert!(l.error.is_none(), "{:?}", l.error);
    let v1 = l.shape.as_ref().unwrap().mass().volume;
    println!("carga fria (peças em paralelo, gravadas no disco): {:?}", t.elapsed());
    let t = Instant::now();
    let mut m = Models::new(Some(dir));
    let l = m.load(&src);
    let v2 = l.shape.as_ref().unwrap().mass().volume;
    println!("carga com as formas no disco: {:?}", t.elapsed());
    assert!((v1 - v2).abs() <= 1e-6 * v1, "{v1} {v2}");
    let _ = std::fs::remove_dir_all(&cache);
}
