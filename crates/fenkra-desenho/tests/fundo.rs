// SPDX-License-Identifier: LGPL-2.1-or-later
//! B427: a projeção das vistas de montagem roda numa thread própria, com progresso e cancelamento.
//! A interface recebe uma projeção vazia marcada `calculando` e a troca pela pronta quando chega;
//! o resultado é o mesmo da projeção feita na própria thread.

use fenkra_desenho::source::Models;
use fenkra_core::Id;
use fenkra_desenho::{Dir, Drawing, Format, Source, ViewKind};
use fenkra_montagem::{Assembly, NodeKind, Placement};
use std::time::{Duration, Instant};

fn montar(nome: &str, pecas: usize) -> (std::path::PathBuf, Source) {
    let dir = std::env::temp_dir().join(format!("{nome}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("placa.fkp"), fenkra_solidos::example_document().to_json()).unwrap();
    let mut a = Assembly::new("raiz");
    for k in 0..pecas {
        a.add_component(&format!("P{k}"), NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [200. * k as f64, 0., 0.], ..Default::default() }, None);
    }
    std::fs::write(dir.join("raiz.fka"), a.to_json()).unwrap();
    (dir, Source { path: "raiz.fka".into(), display_state: String::new(), hidden: vec![] })
}

/// Espera a projeção exata; devolve também se a prévia por malha apareceu antes.
fn esperar_exata(m: &mut Models, d: &Drawing, v: Id) -> (fenkra_desenho::source::Proj, bool) {
    let t = Instant::now();
    let mut viu_previa = false;
    loop {
        m.sondar();
        let p = m.proj(d, v);
        if p.previa {
            assert!(!p.lines.is_empty() && p.error.is_none(), "a prévia já traz as linhas: {:?}", p.error);
            viu_previa = true;
        }
        if !p.calculando && !p.previa {
            return (p, viu_previa);
        }
        assert!(t.elapsed() < Duration::from_secs(120), "a projeção em segundo plano não chegou");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn vista_de_montagem_sai_da_thread_da_interface_e_chega_igual_a_sincrona() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_desenho_fundo", 3);
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let v = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Topo), [100., 100.], 1.);
    // referência: a mesma vista calculada na própria thread
    let mut sinc = Models::new(Some(dir.clone()));
    let esperada = sinc.proj(&d, v);
    assert!(esperada.error.is_none() && !esperada.lines.is_empty());

    let mut m = Models::new(Some(dir.clone()));
    m.ativar_fundo(|| {});
    let t = Instant::now();
    let primeira = m.proj(&d, v);
    assert!(primeira.calculando, "volta na hora com o lugar da vista");
    assert!(primeira.lines.is_empty() && primeira.error.is_none());
    assert!(t.elapsed() < Duration::from_secs(2), "pedir não pode calcular aqui: {:?}", t.elapsed());
    assert!(m.andamento_do_fundo().is_some(), "há trabalho em andamento");
    m.sondar();
    let (pronta, viu_previa) = esperar_exata(&mut m, &d, v);
    assert!(viu_previa, "B393: a prévia por malha chega antes da exata");
    assert!(pronta.error.is_none(), "{:?}", pronta.error);
    assert_eq!(pronta.lines.len(), esperada.lines.len());
    assert!((pronta.hi[0] - esperada.hi[0]).abs() < 1e-6 && (pronta.lo[0] - esperada.lo[0]).abs() < 1e-6);
    assert!(m.andamento_do_fundo().is_none(), "terminou: sem barra");
    // pronta de novo é do cache, sem pedir outra vez
    assert!(!m.proj(&d, v).calculando);
    // sem carga pesada aqui: o modelo leve serve a árvore e as tabelas
    assert!(m.volume(&src) > 0., "volume medido pela thread, para a legenda");
    let _ = std::fs::remove_dir_all(dir);
}

/// B393: reabrir o desenho sem mudar nada traz a vista exata do disco, sem prévia e sem refazer as
/// linhas ocultas; mudar a peça muda a chave e a vista é refeita.
#[test]
fn vista_exata_reaberta_sai_do_disco_e_peca_mudada_refaz() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_desenho_fundo_disco", 2);
    std::env::set_var("FENKRA_CACHE_FORMAS", dir.join("cache"));
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let v = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [100., 100.], 1.);
    let mut m = Models::new(Some(dir.clone()));
    m.ativar_fundo(|| {});
    let (primeira, viu_previa) = esperar_exata(&mut m, &d, v);
    assert!(viu_previa && primeira.error.is_none());
    let guardadas = || std::fs::read_dir(dir.join("cache")).map_or(0, |r| r.filter(|e| e.as_ref().is_ok_and(|e| e.path().extension().is_some_and(|x| x == "fkl"))).count());
    // (outros testes do mesmo processo podem gravar na mesma pasta: a variável vale para todos)
    let antes = guardadas();
    assert!(antes >= 1, "a exata foi para o disco");

    // desenho reaberto: outro Models, mesmos arquivos
    let mut m2 = Models::new(Some(dir.clone()));
    m2.ativar_fundo(|| {});
    let (segunda, viu_previa) = esperar_exata(&mut m2, &d, v);
    assert!(!viu_previa, "do disco a vista já chega exata");
    assert_eq!(segunda.lines, primeira.lines);
    assert!((segunda.hi[0] - primeira.hi[0]).abs() < 1e-9 && (segunda.lo[1] - primeira.lo[1]).abs() < 1e-9);

    // a peça muda: outra chave, a vista é refeita (prévia e exata de novo)
    let mut doc = fenkra_solidos::example_document();
    doc.params[0].expr = "150".into();
    std::thread::sleep(Duration::from_millis(30));
    std::fs::write(dir.join("placa.fkp"), doc.to_json()).unwrap();
    let mut m3 = Models::new(Some(dir.clone()));
    m3.ativar_fundo(|| {});
    let (terceira, viu_previa) = esperar_exata(&mut m3, &d, v);
    assert!(viu_previa, "peça mudada não pode sair do disco");
    assert!((terceira.hi[0] - terceira.lo[0]) > (primeira.hi[0] - primeira.lo[0]) + 1., "a placa cresceu");
    assert!(guardadas() > antes, "a vista nova também foi para o disco");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn vistas_do_mesmo_quadro_vao_num_lote_e_cancelar_deixa_as_vistas_paradas_sem_recalcular_a_cada_quadro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_desenho_fundo_cancela", 2);
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let f = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [100., 100.], 1.);
    let t = d.add_view(s, src.clone(), ViewKind::Projected { parent: f, dir: Dir::Topo }, [100., 40.], 1.);
    let iso = d.add_view(s, src, ViewKind::Standard(Dir::Iso), [200., 100.], 1.);
    // B393: a exata vai em processos, como no aplicativo
    fenkra_desenho::hlr_processos::definir_executavel(env!("CARGO_BIN_EXE_fenkra-hlr").into());
    let mut m = Models::new(Some(dir.clone()));
    m.ativar_fundo(|| {});
    // B393: a projetada não espera a mãe; as três pedidas no mesmo quadro vão juntas
    for v in [f, t, iso] {
        assert!(m.proj(&d, v).calculando);
    }
    m.sondar();
    let a = m.andamento_do_fundo().expect("em andamento");
    assert!(a.calculando && a.esperando == 0, "um lote só com as três vistas: {a:?}");
    m.cancelar_fundo();
    assert!(m.vistas_canceladas());
    for v in [f, t, iso] {
        let parada = m.proj(&d, v);
        assert!(!parada.calculando && parada.error.as_deref() == Some("cálculo cancelado"), "{:?}", parada.error);
    }
    // pedir de novo no quadro seguinte não manda nada para a thread
    m.sondar();
    assert!(m.andamento_do_fundo().is_none());
    // a pessoa pede de novo: as vistas vêm, cada uma com a exata igual à calculada aqui
    m.retomar_fundo();
    assert!(!m.vistas_canceladas());
    for v in [f, t, iso] {
        let _ = m.proj(&d, v);
    }
    let mut sinc = Models::new(Some(dir.clone()));
    for v in [f, t, iso] {
        let (pronta, _) = esperar_exata(&mut m, &d, v);
        // a prévia guardada não pede a exata de novo: chegada a exata, nada mais na fila
        assert!(m.andamento_do_fundo().is_none(), "exata pedida duas vezes: {:?}", m.andamento_do_fundo());
        let esperada = sinc.proj(&d, v);
        assert!(pronta.error.is_none() && !pronta.lines.is_empty(), "{:?}", pronta.error);
        assert_eq!(pronta.lines.len(), esperada.lines.len(), "a exata em paralelo é a mesma da sequencial");
    }
    let _ = std::fs::remove_dir_all(dir);
}
