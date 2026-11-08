// SPDX-License-Identifier: LGPL-2.1-or-later
//! B382 (K-04 do kart): mudar o entre-eixos no mestre e atualizar os links pela montagem de topo
//! chega às peças dos subconjuntos. Ignorado por padrão: copia `validacao/kart/modelo` para uma
//! pasta temporária e recalcula as peças. Rode com
//! `cargo test -p fenkra-montagem --release --test links_kart -- --ignored --nocapture`.

use fenkra_montagem::links::{atualizar, Gatilho};
use fenkra_montagem::load::PartCache;
use fenkra_montagem::{Assembly, NodeKind};
use std::path::{Path, PathBuf};

fn copiar(de: &Path, para: &Path) {
    std::fs::create_dir_all(para).unwrap();
    for e in std::fs::read_dir(de).unwrap().flatten() {
        let p = e.path();
        if p.is_file() {
            std::fs::copy(&p, para.join(e.file_name())).unwrap();
        }
    }
}

/// Arquivos de peça usados pela montagem, descendo pelos subconjuntos.
fn usadas(dir: &Path, fka: &str, out: &mut Vec<String>) {
    let Ok(a) = std::fs::read_to_string(dir.join(fka)).map(|t| Assembly::from_json(&t).unwrap()) else { return };
    for n in &a.nodes {
        match &n.kind {
            NodeKind::Part { path } => out.push(path.clone()),
            NodeKind::SubAssembly { path } => usadas(dir, path, out),
            NodeKind::Group => {}
        }
    }
}

/// Valor guardado do link `mestre_EE` da peça.
fn ee_da_peca(p: &Path) -> Option<f64> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    v["external"].as_array()?.iter().find(|l| l["publication"] == "EE").and_then(|l| l["snapshot"]["Value"].as_f64())
}

#[test]
#[ignore]
fn entre_eixos_do_mestre_desce_pelo_kart() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let modelo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../validacao/kart/modelo");
    let Ok(txt) = std::fs::read_to_string(modelo.join("KART.fka")) else { return };
    let dir = std::env::temp_dir().join(format!("fenkra_links_kart_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copiar(&modelo, &dir);
    // as peças do kart que leem o entre-eixos estão todas dentro de subconjuntos (quadro, freio,
    // carroceria); EX-12 fica na pasta, mas nenhuma montagem a usa
    let mut nomes = vec![];
    usadas(&dir, "KART.fka", &mut nomes);
    let ligadas: Vec<PathBuf> = nomes.iter().map(|n| dir.join(n)).filter(|p| ee_da_peca(p) == Some(1040.)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    assert!(ligadas.len() >= 5, "{ligadas:?}");
    assert!(ee_da_peca(&dir.join("EX-12.fkp")) == Some(1040.) && !nomes.iter().any(|n| n == "EX-12.fkp"));
    let mestre = dir.join("MESTRE.fkp");
    let m = std::fs::read_to_string(&mestre).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&m).unwrap();
    let ee = v["params"].as_array_mut().unwrap().iter_mut().find(|p| p["name"] == "EE").unwrap();
    ee["expr"] = "1070".into();
    std::fs::write(&mestre, v.to_string()).unwrap();

    let k = Assembly::from_json(&txt).unwrap();
    let t = std::time::Instant::now();
    let rel = atualizar(&k, Some(&dir), &mut PartCache::default(), Gatilho::Manual);
    println!("kart: {} peças e {} subconjuntos regravados em {:.1} s", rel.pecas.len(), rel.subconjuntos.len(), t.elapsed().as_secs_f64());
    assert!(rel.ilegiveis.is_empty(), "{:?}", rel.ilegiveis);
    for p in &ligadas {
        assert_eq!(ee_da_peca(p), Some(1070.), "{} não recebeu o entre-eixos novo", p.display());
        assert!(rel.pecas.iter().any(|x| x.file_name() == p.file_name()), "{} fora do relatório", p.display());
    }
    assert_eq!(ee_da_peca(&dir.join("EX-12.fkp")), Some(1040.), "peça fora da montagem não muda");
    let _ = std::fs::remove_dir_all(dir);
}
