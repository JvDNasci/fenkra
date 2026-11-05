// SPDX-License-Identifier: LGPL-2.1-or-later
//! B84: exportação. A extensão decide o formato e as desconhecidas são recusadas; STEP e STL são
//! gravados e relidos num caminho com acento e espaço, conferindo volume e caixa envolvente.

mod common;

use common::*;
use fenkra_core::*;
use fenkra_kernel::Shape;
use fenkra_solidos::{export, export_com_preset, mesh_io, FormatoExport};
use std::fs;
use std::path::PathBuf;

/// Pasta nova, com acento e espaço, só deste teste.
fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra exportação {} {teste}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// Bloco 40 x 30 x 20 com canto em (-20, -15, 0).
fn bloco() -> Document {
    let mut d = Document::new("bloco");
    let s = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -15.], [20., 15.]);
    d.add_feature(ext(s, BoolOp::Add, "20"));
    d
}

fn cilindro() -> Document {
    let mut d = Document::new("cilindro");
    d.add_feature(primitiva(PrimKind::Cilindro, "10", "30", "0", "0"));
    d
}

fn caixa_dos_triangulos(t: &[f64]) -> ([f64; 3], [f64; 3]) {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for p in t.chunks_exact(3) {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

fn perto(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
}

#[test]
fn extensao_decide_o_formato_sem_olhar_a_caixa() {
    assert_eq!(FormatoExport::do_caminho("a.step"), Ok(FormatoExport::Step));
    assert_eq!(FormatoExport::do_caminho("a.STP"), Ok(FormatoExport::Step));
    assert_eq!(FormatoExport::do_caminho("pasta.stl/a b.StL"), Ok(FormatoExport::Stl));
}

#[test]
fn extensao_desconhecida_e_recusada_e_nada_e_gravado() {
    let dir = pasta("recusa");
    for nome in ["peça.obj", "peça.step.txt", "peça.stlx", "peça", "peça.", ".stl"] {
        let alvo = dir.join(nome);
        let e = export(&bloco(), None, alvo.to_str().unwrap()).expect_err(nome);
        assert!(e.contains(".step") && e.contains(".stl"), "{nome}: {e}");
        assert!(!alvo.exists(), "{nome} foi gravado mesmo recusado");
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn step_ida_e_volta_com_acento_e_espaco() {
    if !kernel_disponivel() {
        return;
    }
    let dir = pasta("step");
    for nome in ["peça usinada nº 1.step", "ação final.STP"] {
        let alvo = dir.join(nome);
        let caminho = alvo.to_str().unwrap();
        export(&bloco(), None, caminho).unwrap();
        assert!(alvo.is_file(), "o nome do arquivo saiu trocado: {:?}", fs::read_dir(&dir).unwrap().collect::<Vec<_>>());
        assert!(fs::read_to_string(&alvo).unwrap().starts_with("ISO-10303-21"));

        let lida = Shape::import_step(caminho).unwrap();
        assert!((lida.mass().volume - 24000.).abs() < 1e-3, "{}", lida.mass().volume);
        assert_eq!(lida.face_count(), 6);
        let (a, b) = lida.bbox().unwrap();
        assert!(perto(a, [-20., -15., 0.]) && perto(b, [20., 15., 20.]), "{a:?} {b:?}");
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stl_ida_e_volta_com_acento_e_espaco() {
    if !kernel_disponivel() {
        return;
    }
    let dir = pasta("stl");
    let alvo = dir.join("peça de impressão.stl");
    let caminho = alvo.to_str().unwrap();
    export(&bloco(), None, caminho).unwrap();

    let bytes = fs::read(&alvo).unwrap();
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 84 + 50 * n, "o tamanho é o de um STL binário");
    assert_eq!(n, 12, "seis faces planas dão dois triângulos cada");

    let t = mesh_io::read(caminho).unwrap();
    assert_eq!(t.len(), 12 * 9);
    let (a, b) = caixa_dos_triangulos(&t);
    assert!(perto(a, [-20., -15., 0.]) && perto(b, [20., 15., 20.]), "{a:?} {b:?}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stl_sai_igual_byte_a_byte_em_duas_exportacoes() {
    if !kernel_disponivel() {
        return;
    }
    let dir = pasta("repete");
    let (a, b) = (dir.join("a.stl"), dir.join("b.stl"));
    export(&cilindro(), None, a.to_str().unwrap()).unwrap();
    export(&cilindro(), None, b.to_str().unwrap()).unwrap();
    assert!(fs::read(a).unwrap() == fs::read(b).unwrap());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn presets_do_stl_mudam_a_malha_e_nao_a_medida() {
    if !kernel_disponivel() {
        return;
    }
    let dir = pasta("presets");
    let triangulos: Vec<usize> = PresetStl::TODOS
        .iter()
        .map(|&p| {
            let alvo = dir.join(format!("cilindro {}.stl", p.nome()));
            export_com_preset(&cilindro(), None, alvo.to_str().unwrap(), p).unwrap();
            let t = mesh_io::read(alvo.to_str().unwrap()).unwrap();
            let (a, b) = caixa_dos_triangulos(&t);
            assert!((b[2] - a[2] - 30.).abs() < 1e-3, "{p:?}: altura {}", b[2] - a[2]);
            t.len() / 9
        })
        .collect();
    assert!(triangulos[0] < triangulos[1] && triangulos[1] < triangulos[2], "{triangulos:?}");
    fs::remove_dir_all(dir).unwrap();
}
