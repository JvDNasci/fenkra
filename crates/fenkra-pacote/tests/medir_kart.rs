// SPDX-License-Identifier: LGPL-2.1-or-later
//! Medição manual (B400): quanto leva o `.fkv` da montagem do kart e onde vai o tempo.
//! `cargo test -p fenkra-pacote --test medir_kart -- --ignored --nocapture`

use fenkra_core::trabalho::Controle;
use fenkra_montagem::Assembly;
use fenkra_pacote::{de_montagem, Opcoes};
use std::time::Instant;

fn kart() -> (Assembly, std::path::PathBuf) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let a = Assembly::from_json(&std::fs::read_to_string(dir.join("KART.fka")).unwrap()).unwrap();
    (a, dir)
}

#[test]
#[ignore]
fn tempo_do_fkv_do_kart_por_etapa() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (a, dir) = kart();
    for (nome, o) in [
        ("sem analise, malha inteira", Opcoes { analise: false, capturas: false, ..Default::default() }),
        ("sem analise, decimado 0,3", Opcoes { analise: false, capturas: false, decimar: Some(0.3), ..Default::default() }),
        ("com analise, decimado 0,3 (a simplificada do painel)", Opcoes { capturas: false, decimar: Some(0.3), ..Default::default() }),
    ] {
        let c = Controle::novo();
        let o = Opcoes { controle: Some(c.clone()), ..o };
        let t = Instant::now();
        let p = de_montagem(&a, &dir, &o).unwrap();
        println!("{nome}: {:?} triangulos={} passos={:?}", t.elapsed(), p.manifesto["estatisticas"]["triangulos"], c.progresso());
    }
}

#[test]
fn cancelar_para_a_geracao_sem_pacote_pela_metade() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let (a, dir) = kart();
    let c = Controle::novo();
    c.cancelar();
    let o = Opcoes { controle: Some(c), ..Default::default() };
    let t = Instant::now();
    let r = de_montagem(&a, &dir, &o);
    assert_eq!(r.err().as_deref(), Some(fenkra_pacote::CANCELADO), "cancelado antes de começar não gera nada");
    assert!(t.elapsed().as_secs() < 10, "parou no primeiro componente: {:?}", t.elapsed());
}
