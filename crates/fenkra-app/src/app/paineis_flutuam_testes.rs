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
    // ele é desenhado e fica de pé enquanto o erro existe
    c.passo(vec![]);
    c.passo(vec![]);
    assert!(c.app.wizard_open);
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
