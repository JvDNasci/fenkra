// SPDX-License-Identifier: LGPL-2.1-or-later
//! B454: malha e textura com caminho relativo abrem a partir da pasta do documento.
//!
//! Cenário: uma peça guarda a malha em `recursos/bloco.stl` e a textura em `recursos/grao.png`, ambos
//! relativos à peça. Ela é empacotada em `.zip`, a pasta original é apagada e o pacote é aberto numa
//! outra pasta temporária, como se fosse outra máquina. A pasta de trabalho do teste (a do crate) não
//! tem nenhum desses arquivos: se o comando ainda lesse a partir dela, o recálculo falharia.

use crate::pacote::{empacotar, ler_zip, OpcoesPacote};
use fenkra_core::{Document, FeatureKind, MeshImport, TextureRef};
use std::path::{Path, PathBuf};

fn pasta(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fenkra-b454-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// STL de texto de uma caixa fechada `l` (x, y, z), com a normal para fora.
fn stl_caixa(l: [f64; 3]) -> String {
    let v = |i: usize| [if i & 1 != 0 { l[0] } else { 0. }, if i & 2 != 0 { l[1] } else { 0. }, if i & 4 != 0 { l[2] } else { 0. }];
    // cada face em dois triângulos, ordem anti-horária vista de fora
    let faces = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
    let mut s = String::from("solid bloco\n");
    for f in faces {
        for t in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]] {
            s += "facet normal 0 0 0\nouter loop\n";
            for i in t {
                let p = v(i);
                s += &format!("vertex {} {} {}\n", p[0], p[1], p[2]);
            }
            s += "endloop\nendfacet\n";
        }
    }
    s + "endsolid bloco\n"
}

/// Empacota a peça em `.zip`, apaga a origem e extrai o pacote noutra pasta. Devolve o arquivo da peça.
fn levar_para_outra_pasta(nome: &str, corpo: bool) -> (PathBuf, PathBuf) {
    let origem = pasta(&format!("{nome}-origem"));
    std::fs::create_dir_all(origem.join("peca/recursos")).unwrap();
    std::fs::write(origem.join("peca/recursos/bloco.stl"), stl_caixa([10., 20., 30.])).unwrap();
    std::fs::write(origem.join("peca/recursos/grao.png"), [0x89u8, b'P', b'N', b'G', 1, 2, 3]).unwrap();
    let mut d = Document::new("Bloco");
    d.add_feature(FeatureKind::MeshImport(MeshImport { path: "recursos/bloco.stl".into(), solid: true, corpo, dados: String::new() }));
    d.textures.push(TextureRef { name: "Grão".into(), path: "recursos/grao.png".into(), scale: 20. });
    let arquivo = origem.join("peca/bloco.fkp");
    std::fs::write(&arquivo, d.to_json()).unwrap();

    let zip = pasta(&format!("{nome}-zip")).join("bloco.zip");
    let r = empacotar(&arquivo, &zip, &OpcoesPacote::default()).unwrap();
    assert!(r.verificacao.ok(), "{:?}", r.verificacao);
    assert!(r.manifesto.externas.is_empty() && r.manifesto.ausentes.is_empty(), "{:?}", r.manifesto);
    std::fs::remove_dir_all(&origem).unwrap();

    let destino = pasta(&format!("{nome}-outra-maquina"));
    for (n, dados) in ler_zip(&std::fs::read(&zip).unwrap()).unwrap() {
        let p = destino.join(&n);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, dados).unwrap();
    }
    let _ = std::fs::remove_dir_all(zip.parent().unwrap());
    (destino.join(&r.manifesto.raiz), destino)
}

/// Recalcula e confere status, volume (6000) e caixa envolvente (10 x 20 x 30).
fn conferir(doc: &Document) {
    let r = fenkra_solidos::recompute(doc);
    let f = &doc.features[0];
    assert!(matches!(r.status[&f.id], fenkra_solidos::Status::Ok | fenkra_solidos::Status::Warning(_)), "{:?}", r.status);
    assert_eq!(r.bodies.len(), 1);
    assert!((r.bodies[0].mass.volume - 6000.).abs() < 1e-3, "volume {}", r.bodies[0].mass.volume);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in r.bodies[0].mesh.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    for (k, esperado) in [10f32, 20., 30.].into_iter().enumerate() {
        assert!((hi[k] - lo[k] - esperado).abs() < 1e-3, "caixa {lo:?} {hi:?}");
    }
}

fn abrir_pacote_noutra_pasta(nome: &str, corpo: bool) {
    let (arquivo, destino) = levar_para_outra_pasta(nome, corpo);
    assert!(!Path::new("recursos/bloco.stl").exists(), "a pasta de trabalho não pode ter a malha");
    let lido = Document::from_json(&std::fs::read_to_string(&arquivo).unwrap()).unwrap();
    let FeatureKind::MeshImport(m) = &lido.features[0].kind else { panic!() };
    assert_eq!(m.path, "recursos/bloco.stl", "o pacote mantém o caminho relativo");

    // a causa do B454: sem saber a pasta do documento, o caminho relativo cai na pasta de trabalho
    let sem_pasta = fenkra_solidos::recompute(&lido);
    assert!(matches!(sem_pasta.status[&lido.features[0].id], fenkra_solidos::Status::Error(_)), "{:?}", sem_pasta.status);

    let doc = lido.na_pasta(&arquivo);
    conferir(&doc);
    let tex = doc.arquivo_externo(&doc.textures[0].path);
    assert_eq!(std::fs::read(&tex).unwrap(), [0x89u8, b'P', b'N', b'G', 1, 2, 3], "a textura abre da pasta do pacote");
    let _ = std::fs::remove_dir_all(&destino);
}

#[test]
fn malha_brep_e_textura_relativas_abrem_do_pacote_em_outra_pasta() {
    abrir_pacote_noutra_pasta("brep", false);
}

#[test]
fn corpo_de_malha_relativo_abre_do_pacote_em_outra_pasta() {
    abrir_pacote_noutra_pasta("corpo", true);
}

#[test]
fn caminho_absoluto_antigo_continua_abrindo() {
    let d = pasta("absoluto");
    let stl = d.join("bloco.stl");
    std::fs::write(&stl, stl_caixa([10., 20., 30.])).unwrap();
    let mut doc = Document::new("Bloco");
    doc.add_feature(FeatureKind::MeshImport(MeshImport { path: stl.to_string_lossy().into(), solid: true, ..Default::default() }));
    // documento gravado noutro lugar: o absoluto vale como está, com ou sem pasta conhecida
    conferir(&doc);
    conferir(&doc.clone().na_pasta(&pasta("absoluto-outro").join("bloco.fkp")));
    let _ = std::fs::remove_dir_all(&d);
}
