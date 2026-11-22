// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pendências das ondas 1 e 2 (ADR 0107): Espessura (faces de um sólido) e Engrossar superfície são
//! irmãos e mostram as mesmas opções com os mesmos nomes: a seção "Espessura" com o campo "Espessura" e
//! o "⇄ Inverter direção" padrão, e a Operação com os textos de toda a família (Adicionar, Remover,
//! Interseção, os mesmos da extrusão). Dois lados e Simétrico só existem no Engrossar: a camada de uma
//! face sólida tem um lado útil só (para fora acrescenta, para dentro remove).

use super::*;
use crate::contrato::Alvo;
use crate::inverter::{alternar_feature, controles, estado};

/// Trecho do código-fonte entre dois marcadores (o braço de um comando no painel).
fn braco<'a>(fonte: &'a str, de: &str, ate: &str) -> &'a str {
    let i = fonte.find(de).unwrap_or_else(|| panic!("não achei {de}"));
    let j = fonte[i..].find(ate).map(|j| i + j).unwrap_or_else(|| panic!("não achei {ate} depois de {de}"));
    &fonte[i..j]
}

fn espessura(remove: bool) -> FeatureKind {
    FeatureKind::Thickness(Thickness { faces: vec![[0., 0., 10.]], value: "3".into(), face_keys: vec![], remove, regras: vec![] })
}

#[test]
fn espessura_e_engrossar_tem_as_mesmas_secoes_e_nomes() {
    let fonte = include_str!("feat_ui.rs");
    let esp = braco(fonte, "FeatureKind::Thickness(t) => {", "FeatureKind::DirectEdit(e)");
    let eng = braco(fonte, "FeatureKind::ThickSurface(t) => {", "FeatureKind::CloseSurface(c)");
    let lados = braco(fonte, "fn engrossar_lados(", "fn engrossar_direcao(");
    // a entrada, depois a seção Espessura, depois a operação
    for (nome, braco, entrada, op) in [("Espessura", esp, "section(ui, \"Faces\")", "operacao_da_camada("), ("Engrossar", eng, "section(ui, \"Superfície\")", "hyb_op(")] {
        let pos = |x: &str| braco.find(x).unwrap_or_else(|| panic!("{nome} sem {x}"));
        assert!(pos(entrada) < pos("section(ui, \"Espessura\")") && pos("section(ui, \"Espessura\")") < pos(op), "{nome}: entrada, Espessura e Operação, nessa ordem");
    }
    // o campo chama-se Espessura nos dois, e o inverter é o widget padrão
    // (o texto procurado é montado aos pedaços: o leitor de rótulos de `textos_campos` não o toma por campo)
    let campo = format!("{}(ui, \"Espessura\"", "expr");
    assert!(esp.contains(&campo) && lados.contains(&campo));
    assert!(esp.contains("inverter::botao(ui, &mut t.remove)"), "Espessura: o botão padrão no campo da seta");
    assert!(lados.matches("inverter_engrossar(ui, t)").count() == 2, "Engrossar: o botão padrão em Um lado e em Dois lados");
    assert!(braco(fonte, "fn inverter_engrossar(", "\n}\n").contains("inverter::botao("));
    assert!(!esp.contains("radio_value"), "a Espessura não tem mais rádio próprio");
    assert!(!esp.contains("\"Distância\"") && !esp.contains("\"Acrescentar\""), "os nomes antigos saíram");
}

#[test]
fn operacao_usa_os_textos_da_familia() {
    let editor = braco(include_str!("editor.rs"), "fn op_combo(", "\n}\n");
    for (icone, nome, dica) in OPERACOES {
        assert!(editor.contains(&format!("(\"{icone}\", \"{nome}\", \"{dica}\"")), "a extrusão não usa {nome}: as operações divergiram");
    }
    let fonte = include_str!("feat_ui.rs");
    assert!(!fonte.contains("\"Une ao corpo (ou cria o corpo)\""), "o texto antigo do Engrossar saiu");
}

#[test]
fn inverter_dos_dois_liga_ao_campo_do_lado() {
    // Espessura: um inversor, no mesmo campo da operação (Remover = camada para dentro)
    let mut k = espessura(false);
    assert_eq!(controles(Alvo::Feature(&k)).len(), 1, "a Espessura tem um inversor");
    assert!(!estado(Alvo::Feature(&k), 0));
    alternar_feature(&mut k, 0);
    assert_eq!(k, espessura(true), "inverter leva a camada para dentro: Remover");
    assert!(estado(Alvo::Feature(&k), 0), "o botão aparece destacado");
    alternar_feature(&mut k, 0);
    assert_eq!(k, espessura(false));
    // Engrossar: um inversor, que troca as espessuras de lado; o simétrico não tem
    let e0 = FeatureKind::ThickSurface(ThickSurface::normal(SurfRef::Ref(Id(1)), "2", "0"));
    let mut e = e0.clone();
    assert_eq!(controles(Alvo::Feature(&e)).len(), 1);
    alternar_feature(&mut e, 0);
    let FeatureKind::ThickSurface(t) = &e else { unreachable!() };
    assert!(crate::inverter::engrossar_invertido(t), "o botão do painel lê o mesmo estado da seta");
    alternar_feature(&mut e, 0);
    assert_eq!(e, e0);
    let mut sim = ThickSurface::normal(SurfRef::Ref(Id(1)), "4", "0");
    sim.simetrico = true;
    let antes = sim.clone();
    crate::inverter::alternar_engrossar(&mut sim);
    assert_eq!(sim, antes, "no simétrico inverter não muda nada");
}

/// O painel não desenha um segundo "⇄ Inverter direção" depois do painel do comando (antes o editor
/// somava um botão a toda feature com inversor, e a chapa aparecia com dois).
#[test]
fn um_so_botao_de_inverter_por_painel() {
    let editor = include_str!("editor.rs");
    assert!(!editor.contains("botoes_feature("), "o editor ainda soma um botão genérico");
    let fonte = include_str!("feat_ui.rs");
    for (de, ate) in [("FeatureKind::SplitBody(sp) => {", "FeatureKind::ThickSurface(t)"), ("FeatureKind::SewSurface(sw) => {", "FeatureKind::BoundaryFill(bf)")] {
        assert!(braco(fonte, de, ate).contains("inverter::botao("), "{de} sem o botão padrão");
    }
}
