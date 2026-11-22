// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, onda 1: paridade dos irmãos da chapa. A aba (os modos Arestas, Perfil e Superfície), a
//! bainha, a dobra, o joggle e a veneziana têm as mesmas opções de posição, inverter e raio onde elas
//! fazem sentido, com os mesmos nomes. O teste lê a ficha de cada um (descritor, controles de inverter
//! e dados de conversão) e o texto dos painéis, para uma opção nova num irmão sem o outro falhar aqui.

use crate::contrato::Alvo;
use crate::inverter::controles;
use fenkra_core::aba_unif::*;
use fenkra_core::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn aba_de_aresta() -> FeatureKind {
    FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: Some(s("3")), off1: s("0"), off2: s("0"), flip: true, relief: None, relief_kind: 0, side: 2, miter: false, min_gap: true, edge_key: None })
}

fn irmaos_com_inverter() -> Vec<(&'static str, FeatureKind)> {
    let aba = aba_de_aresta();
    let mut v = vec![("aba de aresta", aba.clone())];
    for (nome, m) in [("aba de laço", ModoAba::Laco), ("aba curva", ModoAba::Curva), ("aba de perfil", ModoAba::Perfil), ("aba por superfície", ModoAba::Superficie)] {
        v.push((nome, converter_aba(&aba, m).unwrap()));
    }
    v.push(("bainha", FeatureKind::SmHem(SmHem { edge: [1., 0., 0.], length: s("8"), gap: s("0"), flip: false, kind: 0, edge_key: None })));
    v.push(("dobra", FeatureKind::SmBend(SmBend { a: [1., 0., 0.], b: [2., 0., 0.], angle: s("45"), radius: None, flip: false, down: false, sketch: None, position: 0, cega: None })));
    v.push(("joggle", FeatureKind::SmJoggle(SmJoggle { a: [1., 0., 0.], b: [2., 0., 0.], depth: s("3"), run: s("6"), flip: false, angle: None, plane: None, keep_length: false })));
    v.push(("veneziana", FeatureKind::SmLouver(SmLouver { a: [1., 0., 0.], b: [2., 0., 0.], width: s("12"), height: s("5"), flip: false, down: false })));
    v
}

/// Todo irmão tem o inverter padrão, ligado ao campo que a seta do 3D troca.
#[test]
fn todos_os_irmaos_da_chapa_tem_o_inverter_padrao() {
    for (nome, mut k) in irmaos_com_inverter() {
        let cs = controles(Alvo::Feature(&k));
        assert!(!cs.is_empty(), "{nome}: sem inverter");
        for c in cs {
            assert!(crate::inverter::bool_de_feature(&mut k, c.campo).is_some(), "{nome}: o controle {} não liga a nenhum campo", c.campo);
        }
    }
}

/// A posição do material (lado) existe nos quatro modos que têm linha de dobra apoiada numa aresta ou
/// numa superfície e viaja entre eles: face interna, face externa e começo do raio.
#[test]
fn a_posicao_do_material_e_a_mesma_nos_modos_da_aba() {
    let aba = aba_de_aresta();
    for m in [ModoAba::Aresta, ModoAba::Laco, ModoAba::Curva, ModoAba::Superficie] {
        let k = converter_aba(&aba, m).unwrap();
        let lado = match &k {
            FeatureKind::SmFlange(x) => x.side,
            FeatureKind::SmFlangeLoop(x) => x.side,
            FeatureKind::SmCurveFlange(x) => x.side,
            FeatureKind::SmSurfFlange(x) => x.side,
            _ => unreachable!(),
        };
        assert_eq!(lado, 2, "{m:?} perdeu a posição do material");
        // e volta ao modo de aresta sem perder
        let FeatureKind::SmFlange(volta) = converter_aba(&k, ModoAba::Aresta).unwrap() else { panic!() };
        assert_eq!(volta.side, 2, "{m:?}: a ida e a volta perdem a posição");
    }
}

/// O raio próprio e a direção são os mesmos em todos os modos da aba.
#[test]
fn o_raio_e_a_direcao_viajam_por_todos_os_modos_da_aba() {
    let aba = aba_de_aresta();
    for m in MODOS_ABA {
        let k = converter_aba(&aba, m).unwrap();
        let (raio, flip) = match &k {
            FeatureKind::SmFlange(x) => (x.radius.clone(), x.flip),
            FeatureKind::SmFlangeLoop(x) => (x.radius.clone(), x.flip),
            FeatureKind::SmCurveFlange(x) => (x.radius.clone(), x.flip),
            FeatureKind::SmProfileFlange(x) => (x.radius.clone(), x.flip),
            FeatureKind::SmSurfFlange(x) => (x.radius.clone(), x.flip),
            _ => unreachable!(),
        };
        assert_eq!(raio.as_deref(), Some("3"), "{m:?}");
        assert!(flip, "{m:?}");
    }
}

/// Os painéis dos irmãos dizem as mesmas palavras: "Lado do material" com as três posições, o "raio
/// próprio (senão o da chapa)" e o "Raio interno", sem variantes por comando.
#[test]
fn os_paineis_dos_irmaos_usam_os_mesmos_nomes() {
    let feat = include_str!("feat_ui.rs");
    let laco = include_str!("chapa_laco_ui.rs");
    // quatro painéis com a posição: aba de aresta, aba curva, aba por superfície (feat_ui) e o laço
    assert_eq!(feat.matches("ui.label(tr(\"Lado do material\"))").count(), 3, "aba, aba curva e aba por superfície");
    assert_eq!(laco.matches("ui.label(tr(\"Lado do material\"))").count(), 1, "laço de abas");
    for n in ["dobra por fora", "material por fora", "material por dentro"] {
        assert!(feat.matches(&format!("\"{n}\"")).count() >= 3, "a posição '{n}' falta em algum painel de aba");
    }
    for proibido in ["raio de dobra próprio (senão o da chapa)", "\"Raio de dobra\"", "\"Comprimento (EOP)\""] {
        assert!(!feat.contains(proibido), "variante de nome fora do padrão: {proibido}");
    }
    assert!(feat.matches("raio próprio (senão o da chapa)").count() >= 4, "aba, dobra, aba curva e aba por superfície têm o raio próprio");
}

/// A aba de arestas é um modo só: Aresta, Laço e Curva não aparecem separados na barra de modos.
#[test]
fn a_barra_de_modos_da_aba_tem_arestas_perfil_e_superficie() {
    let ui = include_str!("chapa_unif_ui.rs");
    assert!(ui.contains("const MODOS: [(&str, &str, &str); 3]"));
    for antigo in ["(\"sm_aba_laco\"", "(\"sm_aba_curva\""] {
        assert!(!ui.contains(antigo), "o modo {antigo} não é mais um botão: virou parte de Arestas");
    }
    assert_eq!(MODOS_ABA_UI.len(), 3);
}
