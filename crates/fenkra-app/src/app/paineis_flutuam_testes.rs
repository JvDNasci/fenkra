// SPDX-License-Identifier: LGPL-2.1-or-later
//! B507: o painel do comando e o assistente de reparo flutuam ou encaixam, à escolha, e lembram a
//! posição. O reparo aparece só quando há erro e fecha sozinho quando o problema é corrigido.

use super::arvore_arraste_testes::Cena;
use super::*;
use crate::paineis::Encaixe;

fn erros(c: &Cena) -> usize {
    c.app.result.as_ref().map_or(0, |r| r.status.values().filter(|s| matches!(s, Status::Error(_))).count())
}

#[test]
fn reparo_abre_com_o_erro_e_fecha_sozinho_quando_corrigido() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut c = Cena::nova();
    // documento válido: nenhum assistente
    assert_eq!(erros(&c), 0);
    assert!(!c.app.wizard_open, "sem erro, sem assistente");
    // quebra o sketch base: o plano dele deixa de existir
    let base = c.app.open.doc.features.iter().find(|f| f.name == "Sketch base").map(|f| f.id).unwrap();
    let plano_antes = match &c.app.open.doc.feature(base).unwrap().kind {
        FeatureKind::Sketch(s) => s.plane,
        _ => unreachable!(),
    };
    if let Some(FeatureKind::Sketch(s)) = c.app.open.doc.feature_mut(base).map(|f| &mut f.kind) {
        s.plane = PlaneRef::Datum(Id(987_654_321));
    }
    c.app.request_recompute();
    c.esperar_calculo();
    assert!(erros(&c) > 0, "o documento quebrou");
    assert!(c.app.wizard_open, "com erro, o assistente abre sozinho");
    // B532: abre recolhido, só o aviso; o painel grande só vem com o clique
    assert!(c.app.wizard_recolhido, "erro novo mostra só o aviso");
    c.passo(vec![]);
    c.passo(vec![]);
    assert!(c.app.wizard_open);
    assert!(!crate::paineis::abertos_ou_fechados(&c.ctx).iter().any(|(n, _, _)| n == "reparo"), "recolhido não desenha o painel");
    c.app.wizard_recolhido = false;
    c.passo(vec![]);
    c.passo(vec![]);
    assert!(crate::paineis::abertos_ou_fechados(&c.ctx).iter().any(|(n, _, _)| n == "reparo"), "assistente na tela");
    // corrige o erro: o assistente fecha sozinho, sem clique
    if let Some(FeatureKind::Sketch(s)) = c.app.open.doc.feature_mut(base).map(|f| &mut f.kind) {
        s.plane = plano_antes;
    }
    c.app.request_recompute();
    c.esperar_calculo();
    assert_eq!(erros(&c), 0, "corrigido");
    c.passo(vec![]);
    assert!(!c.app.wizard_open, "corrigido o erro, o assistente fechou");
    // e aberto à mão sem nada quebrado, também não fica preso
    c.app.wizard_open = true;
    c.passo(vec![]);
    assert!(!c.app.wizard_open, "sem erro, não fica preso na tela");
}

#[test]
fn painel_do_comando_flutua_e_volta_para_a_coluna() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut c = Cena::nova();
    let placa = c.app.open.doc.features.iter().find(|f| f.name == "Placa").map(|f| f.id).unwrap();
    c.app.start_session(placa, false, None);
    for _ in 0..4 {
        c.passo(vec![]);
    }
    assert_eq!(c.app.settings.painel("comando", Encaixe::Esquerda).encaixe, Encaixe::Esquerda, "nasce na coluna");
    // soltar: o painel sai da coluna e flutua, já na tela inteira
    c.app.soltar_comando();
    for _ in 0..4 {
        c.passo(vec![]);
    }
    let p = c.app.settings.painel("comando", Encaixe::Esquerda);
    assert_eq!(p.encaixe, Encaixe::Livre);
    let vivo = crate::paineis::abertos_ou_fechados(&c.ctx).into_iter().find(|(n, _, _)| n == "comando").expect("painel do comando flutuando");
    assert_eq!(vivo.2.encaixe, Encaixe::Livre);
    assert!(c.app.session.is_some(), "o comando continua aberto");
    // a posição guardada fora da tela (outro monitor) não esconde o painel
    let mut p = c.app.settings.painel("comando", Encaixe::Esquerda);
    p.pos = Some([9000., 9000.]);
    c.app.settings.guardar_painel("comando", p);
    for _ in 0..4 {
        c.passo(vec![]);
    }
    let r = crate::paineis::retangulo_de(&c.ctx, "comando").expect("painel desenhado");
    let tela = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.));
    assert!(tela.contains_rect(r), "painel inteiro na tela: {r:?}");
    // encaixar devolve à coluna (o mesmo que o botão do cabeçalho faz)
    let mut p = c.app.settings.painel("comando", Encaixe::Esquerda);
    p.encaixe = Encaixe::Esquerda;
    c.app.settings.guardar_painel("comando", p);
    c.passo(vec![]);
    c.passo(vec![]);
    assert!(c.app.session.is_some());
    assert_eq!(c.app.settings.painel("comando", Encaixe::Esquerda).encaixe, Encaixe::Esquerda);
    assert!(c.app.settings.painel("comando", Encaixe::Esquerda).pos.is_some(), "a posição continua guardada para a próxima vez");
}

/// B528 e B541: sketch usado por vários recursos; apagar o sketch abre o menu, o Enter o confirma, o
/// assistente apaga todos os itens com erro de uma vez e o desfazer devolve tudo.
#[test]
fn apagar_todos_os_itens_com_erro_e_desfazer_devolve() {
    if !fenkra_kernel::AVAILABLE {
        return;
    }
    let mut c = Cena::nova();
    let total = c.app.open.doc.features.len();
    let base = c.app.open.doc.features.iter().find(|f| f.name == "Sketch base").map(|f| f.id).unwrap();
    let dependentes = c.app.open.doc.dependents(base).len();
    assert!(dependentes > 0, "o sketch base tem dependentes");
    // Delete no sketch selecionado abre o menu das peças afetadas
    c.app.sel = Sel::Feature(base);
    c.app.delete_selected();
    assert!(c.app.delete_dialog.is_some(), "o menu abriu");
    let enter = egui::Event::Key { key: egui::Key::Enter, physical_key: Some(egui::Key::Enter), pressed: true, repeat: false, modifiers: Default::default() };
    // B541: Enter confirma o menu (apaga só o sketch) e não abre a peça
    c.passo(vec![enter.clone()]);
    c.passo(vec![]);
    assert!(c.app.delete_dialog.is_none(), "o Enter confirmou o menu");
    assert!(c.app.open_pending.is_none(), "o Enter não foi para a peça");
    assert_eq!(c.app.open.doc.features.len(), total - 1, "só o sketch saiu");
    // os dependentes ficaram com erro
    c.esperar_calculo();
    assert!(erros(&c) > 0, "os dependentes ficaram com erro");
    let com_erro: Vec<Id> = c.app.result.as_ref().unwrap().status.iter().filter(|(_, s)| matches!(s, Status::Error(_))).map(|(i, _)| *i).collect();
    // B528: apagar todos, com confirmação (Enter) e um desfazer só
    c.app.apagar_erros = Some(com_erro.clone());
    c.passo(vec![]);
    assert!(c.app.apagar_erros.is_some(), "a confirmação espera");
    c.passo(vec![enter]);
    c.passo(vec![]);
    assert!(c.app.apagar_erros.is_none(), "confirmado");
    for i in &com_erro {
        assert!(c.app.open.doc.feature(*i).is_none() && c.app.open.doc.reference(*i).is_none(), "o item com erro saiu");
    }
    let sobraram = c.app.open.doc.features.len();
    assert!(sobraram < total - 1);
    c.app.undo();
    assert_eq!(c.app.open.doc.features.len(), total - 1, "um desfazer devolve todos os itens com erro");
}
