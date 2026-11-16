// SPDX-License-Identifier: LGPL-2.1-or-later
//! B477: todo comando que nasce sem entrada (nada selecionado no 3D) espera a seleção: o recálculo
//! não pode devolvê-lo com "referência perdida". O B465 cobriu só as referências por id; a Rosca,
//! a Casca e outras guardam a face por posição, e (0,0,0) é o "ainda não escolhi".

use super::*;
use crate::settings::Group;
use crate::toolbar::{entries, Act, Entry};

/// Comandos que abrem diálogo de arquivo do sistema (`rfd`: Abrir, Importar, Exportar, lista de corte em
/// CSV, malha) bloqueiam o teste esperando um clique que nunca vem; montagem, desenho e PMI não criam
/// comando na peça. Ficam de fora por lista explícita (pelo nome da variante).
fn abre_dialogo_ou_outro_documento(c: Cmd) -> bool {
    let nome = format!("{c:?}");
    const EXATOS: &[&str] = &["ImportarArquivo", "MeshImport", "StCutList", "StLibrary", "Impressao"];
    EXATOS.contains(&nome.as_str()) || ["Asm", "Dwg", "Pmi", "Malha"].iter().any(|p| nome.starts_with(p))
}

fn comandos_da_barra() -> Vec<(&'static str, Cmd)> {
    let mut out: Vec<(&'static str, Cmd)> = vec![];
    for g in Group::ALL {
        for e in entries(g) {
            let v = match e {
                Entry::One(i) => vec![i],
                Entry::Family(_, v) => v,
            };
            for i in v {
                if let Act::Cmd(c) = i.act {
                    if abre_dialogo_ou_outro_documento(c) {
                        continue;
                    }
                    if !out.iter().any(|(_, x)| *x == c) {
                        out.push((i.name, c));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn nenhum_comando_criado_vazio_sai_com_referencia_perdida() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FENKRA_DEMO_TESTE", "1");
    let todos = comandos_da_barra();
    assert!(todos.len() > 50, "a barra tem comandos: {}", todos.len());
    let mut ruins = vec![];
    for (nome, c) in todos {
        let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        // peça com sólido, para o recálculo ter onde procurar a face; nada selecionado
        let e = crate::examples::all().into_iter().find(|e| e.name == "Placa paramétrica").expect("exemplo");
        app.load_doc_pub(e.doc, None);
        app.started = true;
        let antes = app.open.doc.features.len();
        eprintln!("comando: {nome} ({c:?})");
        app.command(c);
        let r = fenkra_solidos::recompute(&app.open.doc);
        for f in app.open.doc.features.iter().skip(antes) {
            if let Some(fenkra_solidos::Status::Error(m)) = r.status.get(&f.id) {
                if m.contains("referência perdida") || m.contains("E-REF") {
                    ruins.push(format!("{nome} ({c:?}): {m}"));
                }
            }
        }
    }
    assert!(ruins.is_empty(), "comandos novos com referência perdida:\n{}", ruins.join("\n"));
}

// ------------------------------------------------------------ B480 e B481: abas

use fenkra_montagem::Assembly;
use std::path::PathBuf;

fn pasta(nome: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fenkra_r4_{nome}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn app_com_montagem_alterada() -> App {
    let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
    app.asm = Some(crate::asm_ui::AsmState::new(Assembly::new("conj4"), None));
    app.settings.bench = crate::settings::Bench::Montagem;
    app.started = false;
    app.inicio_pelo_menu = true; // voltou à tela inicial pelo menu
    app.open.modified = true;
    app
}

/// B480: "Novo desenho" e "Nova montagem" da tela inicial não substituem a aba com alterações.
#[test]
fn novo_desenho_da_tela_inicial_nao_substitui_aba_com_alteracoes() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FENKRA_DEMO_TESTE", "1");
    let mut app = app_com_montagem_alterada();
    app.novo_desenho_da_tela();
    assert_eq!(app.documentos.abas.len(), 2, "o desenho abriu numa aba nova");
    assert!(app.dwg.is_some() && app.asm.is_none(), "a aba da frente é só o desenho");
    assert!(app.started);
    let fundo = app.fundo_modificado().expect("a aba editada segue no fundo");
    assert!(app.documentos.abas.guardado(fundo).unwrap().modificado());
}

#[test]
fn nova_montagem_da_tela_inicial_nao_substitui_aba_com_alteracoes() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FENKRA_DEMO_TESTE", "1");
    let mut app = app_com_montagem_alterada();
    app.nova_montagem_da_tela();
    assert_eq!(app.documentos.abas.len(), 2);
    assert!(app.fundo_modificado().is_some(), "a montagem alterada continua no fundo");
    assert!(app.asm.is_some() && app.started);
}

#[test]
fn novo_desenho_ocupa_a_aba_que_nao_tem_nada() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FENKRA_DEMO_TESTE", "1");
    let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
    app.started = false;
    app.novo_desenho_da_tela();
    assert_eq!(app.documentos.abas.len(), 1);
    assert!(app.dwg.is_some());
}

/// B481: com a montagem na frente e a peça por baixo (mesma aba), abrir a peça a mostra; o menu Janela
/// diz que o documento da frente é a montagem.
#[test]
fn abrir_a_peca_com_a_montagem_na_frente_mostra_a_peca() {
    let _g = crate::pmi_nota_cmd::tests::SERIE.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FENKRA_DEMO_TESTE", "1");
    let dir = pasta("abrir_peca");
    let peca = dir.join("placa4.fkp");
    std::fs::write(&peca, crate::examples::all()[0].doc.to_json()).unwrap();
    let mut app = App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
    app.open_path(&peca);
    app.asm = Some(crate::asm_ui::AsmState::new(Assembly::new("conj4"), None));
    app.settings.bench = crate::settings::Bench::Montagem;
    assert!(app.in_asm());
    let (nome, arq) = app.documento_da_frente();
    assert_eq!(nome, "conj4", "o menu Janela fala da montagem que está na frente");
    assert_eq!(arq, None, "a montagem ainda não tem arquivo: nada de placa4.fkp");

    assert!(app.abrir_em_aba(&Acao::AbrirCaminho(peca.clone())));
    assert!(!app.in_asm(), "a peça foi para a frente");
    let (_, arq) = app.documento_da_frente();
    assert_eq!(arq.as_deref(), Some(peca.as_path()));
}
