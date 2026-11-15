// SPDX-License-Identifier: LGPL-2.1-or-later
//! B471: abas com o mesmo arquivo e arquivo que muda no disco, vistos pelo estado do `App` (sem janela).
//!
//! - abrir um exemplo pela tela inicial nunca substitui a aba da frente que tem alterações;
//! - a aba de fundo recarrega do disco quando outra aba salvou o mesmo arquivo, ou avisa quando ela
//!   mesma tem alterações por salvar;
//! - "Editar a peça" pela montagem leva para a aba que já tem o arquivo, em vez de abrir uma cópia.

use super::*;
use fenkra_montagem::{Assembly, NodeKind, Placement};
use std::path::PathBuf;

fn pasta(nome: &str) -> PathBuf {
    std::env::set_var("FENKRA_DEMO_TESTE", "1");
    let dir = std::env::temp_dir().join(format!("fenkra_abas_r3_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn app_vazio() -> App {
    App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()))
}

fn nome_do_exemplo(i: usize) -> String {
    crate::examples::all()[i].doc.name.clone()
}

#[test]
fn exemplo_da_tela_inicial_nao_substitui_aba_com_alteracoes() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = app_vazio();
    app.load_doc_pub(crate::examples::all()[0].doc.clone(), None);
    app.started = false;
    app.inicio_pelo_menu = true; // voltou à tela inicial pelo menu, com a peça aberta
    app.open.modified = true;
    assert_eq!(app.documentos.abas.len(), 1);
    app.abrir_exemplo_da_tela(1);
    assert_eq!(app.documentos.abas.len(), 2, "o exemplo abriu numa aba nova");
    assert_eq!(app.open.doc.name, nome_do_exemplo(1));
    assert!(app.started);
    let fundo = app.fundo_modificado().expect("a aba editada continua no fundo, com as alterações");
    let guardada = app.documentos.abas.guardado(fundo).unwrap();
    assert_eq!(guardada.open.doc.name, nome_do_exemplo(0));
    assert!(guardada.modificado());
}

#[test]
fn exemplo_da_tela_inicial_ocupa_a_aba_que_nao_tem_nada() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = app_vazio();
    app.started = false;
    app.abrir_exemplo_da_tela(1);
    assert_eq!(app.documentos.abas.len(), 1, "sem nada a guardar, o exemplo ocupa o lugar da aba");
    assert_eq!(app.open.doc.name, nome_do_exemplo(1));
}

/// Placa aberta de um arquivo numa aba, outra aba na frente; outra aba salva o arquivo; ao voltar à
/// placa o que mudou no disco aparece.
fn duas_abas_com_arquivo_mudado(nome: &str) -> (App, PathBuf, u64) {
    let dir = pasta(nome);
    let arq = dir.join("a.fkp");
    std::fs::write(&arq, crate::examples::all()[0].doc.to_json()).unwrap();
    let mut app = app_vazio();
    app.open_path(&arq);
    assert_eq!(app.open.doc.name, nome_do_exemplo(0));
    let id_peca = app.documentos.abas.ativa();
    assert!(app.abrir_em_aba(&Acao::NovaPeca));
    assert_ne!(app.documentos.abas.ativa(), id_peca);
    // outra aba (ou outro programa) grava outro conteúdo no mesmo arquivo
    std::fs::write(&arq, crate::examples::all()[1].doc.to_json()).unwrap();
    (app, arq, id_peca)
}

#[test]
fn aba_de_fundo_recarrega_quando_o_arquivo_muda_no_disco() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let (mut app, _arq, id) = duas_abas_com_arquivo_mudado("recarrega");
    assert!(app.trocar_documento(id));
    assert_eq!(app.open.doc.name, nome_do_exemplo(1), "a aba mostra o que está no disco");
    assert!(!app.open.modified);
    assert!(app.toasts.iter().any(|t| t.0.contains("Recarregada") && !t.2), "avisa que recarregou: {:?}", app.toasts);
}

#[test]
fn aba_de_fundo_com_alteracoes_avisa_e_nao_perde_o_trabalho() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let dir = pasta("avisa");
    let arq = dir.join("a.fkp");
    std::fs::write(&arq, crate::examples::all()[0].doc.to_json()).unwrap();
    let mut app = app_vazio();
    app.open_path(&arq);
    let id = app.documentos.abas.ativa();
    app.open.modified = true; // alteração por salvar nesta aba
    assert!(app.abrir_em_aba(&Acao::NovaPeca));
    std::fs::write(&arq, crate::examples::all()[1].doc.to_json()).unwrap();
    assert!(app.trocar_documento(id));
    assert_eq!(app.open.doc.name, nome_do_exemplo(0), "o trabalho da aba continua como estava");
    assert!(app.open.modified);
    assert!(app.toasts.iter().any(|t| t.0.contains("mudou no disco") && t.2), "avisa a mudança: {:?}", app.toasts);
}

#[test]
fn aba_de_fundo_sem_mudanca_no_disco_nao_avisa() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let dir = pasta("igual");
    let arq = dir.join("a.fkp");
    std::fs::write(&arq, crate::examples::all()[0].doc.to_json()).unwrap();
    let mut app = app_vazio();
    app.open_path(&arq);
    let id = app.documentos.abas.ativa();
    assert!(app.abrir_em_aba(&Acao::NovaPeca));
    app.toasts.clear();
    assert!(app.trocar_documento(id));
    assert!(app.toasts.is_empty(), "{:?}", app.toasts);
}

#[test]
fn editar_a_peca_pela_montagem_leva_para_a_aba_que_ja_tem_o_arquivo() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    let dir = pasta("editar");
    let peca = dir.join("a.fkp");
    std::fs::write(&peca, crate::examples::all()[0].doc.to_json()).unwrap();
    let mut m = Assembly::new("Conjunto");
    let comp = m.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
    let fka = dir.join("conjunto.fka");
    std::fs::write(&fka, m.to_json()).unwrap();

    let mut app = app_vazio();
    app.open_path(&peca);
    let id_peca = app.documentos.abas.ativa();
    assert!(app.abrir_em_aba(&Acao::AbrirCaminho(fka.clone())));
    assert!(app.asm.is_some(), "a montagem está na frente");
    assert_eq!(app.documentos.abas.len(), 2);

    crate::asm_ui::ativar_peca(&mut app, comp);

    assert_eq!(app.documentos.abas.len(), 2, "nenhuma aba nova com a mesma peça");
    assert_eq!(app.documentos.abas.ativa(), id_peca, "a aba que já tinha o arquivo veio para a frente");
    assert!(app.asm.is_none());
    assert_eq!(app.open.file.as_deref(), Some(peca.as_path()));
}
