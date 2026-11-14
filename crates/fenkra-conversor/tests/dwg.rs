// SPDX-License-Identifier: LGPL-2.1-or-later
//! B211: DWG pelo LibreDWG externo (ADR 0090). O teste com o programa de verdade só roda onde ele está
//! instalado e, sem ele, diz que foi ignorado. O encanamento (procura pela variável, argumentos, pasta
//! temporária, leitura do DXF gerado, gravação) roda sempre, com um programa falso que guarda o DXF.
//!
//! A variável `FENKRA_LIBREDWG` é do processo inteiro: tudo o que mexe nela fica num teste só, em ordem.

use fenkra_conversor::{detectar, dwg, Formato, OpcoesExportar, OpcoesImportar, Registro};
use fenkra_core::sketch::{Curve, SketchData};
use fenkra_core::{Document, FeatureKind, OriginPlane, PlaneRef, Sketch};
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    // sem acento: o programa falso é um .cmd, e o cmd.exe lê o caminho na página de código do console
    let p = std::env::temp_dir().join(format!("fenkra_dwg_{}_{teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Esboço "Contorno" com um retângulo 80 × 40 e um furo de raio 6.
fn documento() -> Document {
    let mut d = Document::new("Placa");
    let mut sk = SketchData::default();
    let mut al = || d.alloc();
    sk.add_rect(&mut al, [0., 0.], [80., 40.], false);
    let c = sk.add_point(&mut al, 20., 20.);
    sk.add_curve(&mut al, Curve::Circle { c, r: 6. }, false);
    let id = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: sk }));
    if let Some(f) = d.features.iter_mut().find(|f| f.id == id) {
        f.name = "Contorno".into();
    }
    d
}

/// Grava em DWG, lê de volta e confere camada, entidades e caixa.
fn ida_e_volta(dir: &Path) -> PathBuf {
    let r = Registro::default();
    let arquivo = dir.join("placa.dwg");
    r.gravar_documento(&documento(), &arquivo, &OpcoesExportar::default()).unwrap();
    assert!(dwg::e_dwg(&std::fs::read(&arquivo).unwrap()), "o arquivo gravado tem cabeçalho de DWG");
    let lido = r.importar(&arquivo, &OpcoesImportar::default()).unwrap();
    assert_eq!(lido.relatorio.formato, "DWG");
    let (nome, sk) = lido.doc.features.iter().find_map(|f| if let FeatureKind::Sketch(s) = &f.kind { Some((f.name.as_str(), &s.data)) } else { None }).unwrap();
    assert_eq!(nome, "Contorno");
    let linhas = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Line { .. })).count();
    let furos = sk.curves.iter().filter(|c| matches!(c.geom, Curve::Circle { r, .. } if (r - 6.).abs() < 1e-9)).count();
    assert_eq!((linhas, furos), (4, 1), "{:?}", sk.curves);
    let (lo, hi) = lido.relatorio.caixa.unwrap();
    assert!(lo[0].abs() < 1e-9 && lo[1].abs() < 1e-9 && (hi[0] - 80.).abs() < 1e-9 && (hi[1] - 40.).abs() < 1e-9, "{lo:?} {hi:?}");
    arquivo
}

#[test]
fn dwg_e_reconhecido_pelo_cabecalho_e_esta_na_matriz() {
    assert_eq!(detectar(Path::new("a.dwg"), b"AC1032\0\0\0\0\0\0").unwrap(), (Formato::Dwg, false));
    assert_eq!(detectar(Path::new("a.bin"), b"AC1015\0\0\0\0\0\0").unwrap(), (Formato::Dwg, true));
    let linha = Registro::default().matriz().into_iter().find(|l| l.formato == Formato::Dwg).unwrap();
    assert_eq!(linha.provedor, "LibreDWG (externo)");
    assert!(linha.capacidades.ler && linha.capacidades.gravar && linha.capacidades.exata);
    assert!(Formato::Dwg.e_2d());
}

#[test]
fn dwg_ida_e_volta_com_o_programa_real_falso_e_ausente() {
    // 1. o LibreDWG de verdade, se a máquina tiver
    if dwg::disponivel() {
        ida_e_volta(&pasta("real"));
    } else {
        eprintln!("DWG REAL IGNORADO: LibreDWG (dwg2dxf e dxf2dwg) não está instalado nesta máquina");
    }

    // 2. programa falso: o dxf2dwg guarda o DXF ao lado e grava um "DWG" só com o cabeçalho; o dwg2dxf devolve
    //    o DXF guardado. Confere a procura pela variável, os argumentos e o caminho dos arquivos.
    let falso = pasta("falso");
    std::fs::write(falso.join("dxf2dwg.cmd"), "@echo off\r\ncopy /y \"%~4\" \"%~dp0ultimo.dxf\" >nul\r\n>\"%~3\" echo AC1015 falso\r\n").unwrap();
    std::fs::write(falso.join("dwg2dxf.cmd"), "@echo off\r\ncopy /y \"%~dp0ultimo.dxf\" \"%~3\" >nul\r\n").unwrap();
    std::env::set_var(dwg::VARIAVEL, &falso);
    assert_eq!(dwg::programa("dwg2dxf"), Some(falso.join("dwg2dxf.cmd")));
    let arquivo = ida_e_volta(&pasta("pelo_falso"));
    // a folha do desenho técnico também abre o DWG
    let (folha, r) = fenkra_conversor::folha::abrir_como_desenho(&arquivo).unwrap();
    assert_eq!((r.esbocos, folha.esbocos[0].nome.as_str()), (1, "Contorno"));

    // 3. apontado para uma pasta sem o programa: a mensagem diz o que instalar e onde apontar
    let vazia = pasta("vazia");
    std::env::set_var(dwg::VARIAVEL, &vazia);
    assert!(dwg::programa("dwg2dxf").is_none(), "apontado e não achado não cai no PATH");
    let arquivo = vazia.join("x.dwg");
    std::fs::write(&arquivo, b"AC1032\0\0\0\0\0\0").unwrap();
    let e = Registro::default().importar(&arquivo, &OpcoesImportar::default()).unwrap_err().to_string();
    assert!(e.contains("LibreDWG") && e.contains("ftp.gnu.org") && e.contains("Programas externos"), "{e}");
    let e = Registro::default().gravar_documento(&documento(), &vazia.join("y.dwg"), &OpcoesExportar::default()).unwrap_err().to_string();
    assert!(e.contains("dxf2dwg") && e.contains("LibreDWG"), "{e}");
    std::env::remove_var(dwg::VARIAVEL);
}
