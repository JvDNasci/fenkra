// SPDX-License-Identifier: LGPL-2.1-or-later
//! B213: a matriz de formatos não é uma tabela escrita à mão. Cada célula é conferida gravando e lendo
//! um modelo de prova, e o documento publicado é gerado dela.

use fenkra_conversor::{Formato, Registro};
use std::path::{Path, PathBuf};

fn pasta(teste: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("Fenkra matriz açaí {} {teste}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn o_que_cada_formato_declara_e_o_que_ele_faz() {
    let r = Registro::default();
    let dir = pasta("sonda");
    for l in r.matriz() {
        let medido = r.sondar(l.formato, &dir).unwrap_or_else(|e| panic!("{}: {e}", l.formato.nome()));
        assert_eq!(medido, l.capacidades, "{} declara uma coisa e faz outra", l.formato.nome());
    }
}

#[test]
fn todo_formato_conhecido_tem_linha_e_quem_atende() {
    let m = Registro::default().matriz();
    assert_eq!(m.len(), Formato::TODOS.len());
    assert!(m.iter().all(|l| !l.provedor.is_empty()));
    assert!(m.iter().all(|l| l.capacidades.ler || l.capacidades.gravar));
    let occt = m.iter().find(|l| l.formato == Formato::Step).unwrap();
    assert_eq!(occt.provedor, "OpenCASCADE");
}

#[test]
fn registro_sem_provedor_some_da_matriz() {
    let vazio = Registro::vazio();
    let m = vazio.matriz();
    assert!(m.iter().all(|l| l.formato != Formato::Step));
    assert!(!vazio.capacidades(Formato::Step).gravar);
}

/// Sem esse teste a página de formatos poderia ficar velha. `FENKRA_ATUALIZAR_MATRIZ=1` regrava.
#[test]
fn documento_publicado_sai_da_matriz() {
    let caminho = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/modulos/formatos-suportados.md");
    let gerado = Registro::default().matriz_em_texto();
    if std::env::var("FENKRA_ATUALIZAR_MATRIZ").is_ok() {
        std::fs::write(&caminho, &gerado).unwrap();
    }
    let lido = std::fs::read_to_string(&caminho).unwrap_or_default().replace("\r\n", "\n");
    assert_eq!(lido, gerado, "rode com FENKRA_ATUALIZAR_MATRIZ=1 para regravar docs/modulos/formatos-suportados.md");
}
