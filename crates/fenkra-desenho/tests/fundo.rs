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

fn esperar_pronta(m: &mut Models, d: &Drawing, v: Id) -> fenkra_desenho::source::Proj {
    let t = Instant::now();
    loop {
        m.sondar();
        let p = m.proj(d, v);
        if !p.calculando {
            return p;
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
    let pronta = esperar_pronta(&mut m, &d, v);
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

#[test]
fn vista_filha_espera_a_mae_e_cancelar_deixa_as_vistas_paradas_sem_recalcular_a_cada_quadro() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (dir, src) = montar("fenkra_desenho_fundo_cancela", 2);
    let mut d = Drawing::new("t", Format::A3);
    let s = d.sheets[0].id;
    let f = d.add_view(s, src.clone(), ViewKind::Standard(Dir::Frente), [100., 100.], 1.);
    let t = d.add_view(s, src, ViewKind::Projected { parent: f, dir: Dir::Topo }, [100., 40.], 1.);
    let mut m = Models::new(Some(dir.clone()));
    m.ativar_fundo(|| {});
    // a filha pedida antes da mãe vira o lugar dela e só segue quando a mãe chega
    assert!(m.proj(&d, t).calculando);
    m.cancelar_fundo();
    assert!(m.vistas_canceladas());
    let parada = m.proj(&d, f);
    assert!(!parada.calculando && parada.error.as_deref() == Some("cálculo cancelado"), "{:?}", parada.error);
    // pedir de novo no quadro seguinte não manda nada para a thread
    assert!(m.andamento_do_fundo().is_none());
    // a pessoa pede de novo: as vistas vêm
    m.retomar_fundo();
    assert!(!m.vistas_canceladas());
    let pronta = esperar_pronta(&mut m, &d, f);
    assert!(pronta.error.is_none() && !pronta.lines.is_empty(), "{:?}", pronta.error);
    let filha = esperar_pronta(&mut m, &d, t);
    assert!(filha.error.is_none() && !filha.lines.is_empty(), "{:?}", filha.error);
    let _ = std::fs::remove_dir_all(dir);
}
