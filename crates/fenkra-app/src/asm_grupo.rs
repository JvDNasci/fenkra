// SPDX-License-Identifier: LGPL-2.1-or-later
//! B485: o Grupo da montagem e o desfazer.
//!
//! O comando Grupo cria o elemento e já abre o campo do nome na árvore. Dois detalhes faziam o Ctrl+Z
//! parecer não funcionar:
//!
//! - com o campo do nome em foco, o atalho ia para o campo de texto, que não tinha nada a desfazer, e
//!   o desfazer da montagem não rodava (ele espera o teclado livre);
//! - ao sair do campo, o nome era gravado como um passo de desfazer próprio, mesmo sem mudar. O
//!   primeiro Ctrl+Z desfazia esse "Grupo" → "Grupo" e nada mudava na tela.
//!
//! Agora o nome igual não vira passo, o nome dado logo depois de criar faz parte da criação, e o
//! atalho de desfazer com o campo aberto e nada digitado desfaz o grupo.

use crate::asm_ui::AsmState;
use eframe::egui;
use crate::keymap::Binding;
use fenkra_core::Id;

/// Chave do texto em edição no campo do nome do elemento `id`, na árvore.
pub fn chave_do_nome(id: Id) -> egui::Id {
    egui::Id::unique(("asm_ren", id.0))
}

/// Grava o nome digitado no campo da árvore. Nome igual ao de antes não vira passo de desfazer; o nome
/// dado no campo que o próprio comando Grupo abriu entra no mesmo passo da criação, e um Ctrl+Z desfaz
/// os dois juntos.
pub fn confirmar_nome(st: &mut AsmState, id: Id, nome: &str) {
    st.renaming = None;
    let recem_criado = st.nome_na_criacao.take() == Some(id) && st.undo.last().is_some_and(|antes| antes.node(id).is_none());
    let novo = nome.trim();
    let Some(atual) = st.asm.node(id).map(|n| n.name.clone()) else { return };
    if atual == novo {
        return;
    }
    if recem_criado {
        st.redo.clear();
        st.modified = true;
    } else {
        st.push_undo();
    }
    if let Some(n) = st.asm.node_mut(id) {
        n.name = novo.to_string();
    }
    st.dirty = true;
}

/// O atalho de desfazer (`atalho`) com o campo do nome aberto e nada digitado nele: fecha o campo e
/// desfaz o último passo da montagem (o Grupo recém-criado). Com texto digitado, o atalho fica com o
/// campo, para desfazer a digitação. Devolve se desfez.
pub fn desfazer_no_nome(ctx: &egui::Context, st: &mut AsmState, atalho: Option<&Binding>) -> bool {
    let (Some(id), Some(b)) = (st.renaming, atalho) else { return false };
    let Some(tecla) = egui::Key::from_name(&b.key) else { return false };
    let chave = chave_do_nome(id);
    let texto: Option<String> = ctx.data(|m| m.get_temp(chave));
    let nome = st.asm.node(id).map(|n| n.name.clone());
    if texto.is_some_and(|t| Some(t) != nome) {
        return false;
    }
    let mut mods = egui::Modifiers::NONE;
    if b.ctrl {
        mods = mods | egui::Modifiers::COMMAND;
    }
    if b.shift {
        mods = mods | egui::Modifiers::SHIFT;
    }
    if b.alt {
        mods = mods | egui::Modifiers::ALT;
    }
    if !ctx.input_mut(|i| i.consume_key(mods, tecla)) {
        return false;
    }
    st.renaming = None;
    st.nome_na_criacao = None;
    ctx.data_mut(|m| m.remove::<String>(chave));
    ctx.memory_mut(|m| m.stop_text_input());
    let Some(antes) = st.undo.pop() else { return false };
    st.redo.push(std::mem::replace(&mut st.asm, antes));
    if st.sel.is_some_and(|s| st.asm.node(s).is_none()) {
        st.sel = None;
    }
    st.dirty = true;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::KeyAction;
    use fenkra_montagem::{Assembly, NodeKind, Placement};

    fn montagem() -> (crate::app::App, Id) {
        let mut app = crate::app::App::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        app.settings.bench = crate::settings::Bench::Montagem;
        let mut a = Assembly::new("t");
        let p = a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        app.asm = Some(AsmState::new(a, None));
        (app, p)
    }

    fn grupos(st: &AsmState) -> Vec<String> {
        st.asm.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Group)).map(|n| n.name.clone()).collect()
    }

    /// Ctrl+Z da montagem, como o `App` faz com o teclado livre.
    fn desfazer(st: &mut AsmState) {
        let antes = st.undo.pop().expect("passo de desfazer");
        st.redo.push(std::mem::replace(&mut st.asm, antes));
    }

    #[test]
    fn nome_igual_nao_vira_passo_e_um_ctrl_z_desfaz_o_grupo() {
        let (mut app, _) = montagem();
        crate::asm_ui::command(&mut app, crate::app::Cmd::AsmGroup);
        let st = app.asm.as_mut().unwrap();
        let g = st.sel.unwrap();
        assert_eq!(grupos(st), ["Grupo"]);
        assert_eq!(st.renaming, Some(g), "o campo do nome abre junto");
        assert_eq!(st.undo.len(), 1);
        // sair do campo sem digitar
        confirmar_nome(st, g, "Grupo");
        assert_eq!(st.undo.len(), 1, "o nome igual não é um passo");
        desfazer(st);
        assert!(grupos(st).is_empty(), "um Ctrl+Z tira o grupo");
        assert_eq!(st.asm.nodes.len(), 1, "a placa continua");
    }

    #[test]
    fn nome_dado_logo_depois_entra_no_passo_da_criacao() {
        let (mut app, _) = montagem();
        crate::asm_ui::command(&mut app, crate::app::Cmd::AsmGroup);
        let st = app.asm.as_mut().unwrap();
        let g = st.sel.unwrap();
        confirmar_nome(st, g, "  Ferragens ");
        assert_eq!(grupos(st), ["Ferragens"]);
        assert_eq!(st.undo.len(), 1);
        desfazer(st);
        assert!(grupos(st).is_empty());
        // refazer volta o grupo já com o nome
        let depois = st.redo.pop().unwrap();
        st.undo.push(std::mem::replace(&mut st.asm, depois));
        assert_eq!(grupos(st), ["Ferragens"]);
        // renomear depois, em outro momento, é um passo próprio
        confirmar_nome(st, g, "Parafusos");
        assert_eq!(st.undo.len(), 2);
        desfazer(st);
        assert_eq!(grupos(st), ["Ferragens"]);
    }

    /// Um quadro com o atalho de desfazer apertado.
    fn quadro_com_ctrl_z(ctx: &egui::Context, st: &mut AsmState, atalho: Option<&Binding>) -> bool {
        let ev = egui::Event::Key { key: egui::Key::Z, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
        let raw = egui::RawInput { events: vec![ev], ..Default::default() };
        let mut feito = false;
        let mut saida = ctx.run_ui(raw, |ui| feito = desfazer_no_nome(ui.ctx(), st, atalho));
        saida.textures_delta.clear();
        feito
    }

    #[test]
    fn ctrl_z_com_o_campo_do_nome_aberto_desfaz_o_grupo() {
        let (mut app, placa) = montagem();
        let atalho = app.settings.keys.binding(KeyAction::Undo);
        assert!(atalho.as_ref().is_some_and(|b| b.ctrl && b.key == "Z"), "{atalho:?}");
        crate::asm_ui::command(&mut app, crate::app::Cmd::AsmGroup);
        let st = app.asm.as_mut().unwrap();
        let g = st.sel.unwrap();
        let ctx = egui::Context::default();
        // digitou um nome: o Ctrl+Z é do campo de texto
        ctx.data_mut(|m| m.insert_temp(chave_do_nome(g), "Ferr".to_string()));
        assert!(!quadro_com_ctrl_z(&ctx, st, atalho.as_ref()));
        assert_eq!(grupos(st), ["Grupo"]);
        // nada digitado (o texto é o nome de agora): desfaz o grupo e fecha o campo
        ctx.data_mut(|m| m.insert_temp(chave_do_nome(g), "Grupo".to_string()));
        assert!(quadro_com_ctrl_z(&ctx, st, atalho.as_ref()));
        assert!(grupos(st).is_empty(), "o grupo saiu");
        assert_eq!(st.renaming, None, "o campo do nome fechou");
        assert_eq!(st.sel, None, "a seleção não aponta para o grupo que sumiu");
        assert_eq!(st.redo.len(), 1, "Ctrl+Y traz de volta");
        assert!(st.asm.node(placa).is_some());
        // sem campo aberto, o atalho fica com o App (desfazer de sempre)
        assert!(!quadro_com_ctrl_z(&ctx, st, atalho.as_ref()));
    }
}
