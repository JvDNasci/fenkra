// SPDX-License-Identifier: LGPL-2.1-or-later
//! ADR 0107, onda 0 (B562 base): um só "⇄ Inverter direção".
//!
//! O botão do painel e a seta clicável do 3D ligam ao **mesmo campo**: as duas coisas chamam
//! `alternar_ref` ou `alternar_feature`, a única função que sabe qual campo do comando troca o lado
//! (`flip`, `flip_a`, `more_flip`, a troca das duas espessuras...). Texto e ícone são os do widget
//! padrão de `chapa_unif_ui::inverter`, o mesmo de toda a Chapa.
//!
//! `controles` lista, por comando, os inversores que o painel mostra (um por seta do 3D). Quem
//! cria um comando novo com lado declara os controles aqui, e o teste de paridade confere que os
//! comandos irmãos têm o mesmo conjunto.

use crate::contrato::Alvo;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::*;

/// Um inversor de um comando: `campo` é o mesmo número da seta do 3D (`SetaDeLado::campo`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Controle {
    pub campo: u8,
    /// de que lado se trata, quando o comando tem mais de um inversor ("Fica da superfície 1")
    pub rotulo: &'static str,
}

/// Os inversores que o painel do comando mostra, na ordem das setas do 3D.
pub fn controles(alvo: Alvo) -> Vec<Controle> {
    match alvo {
        Alvo::Ref(k) => match k {
            RefKind::SurfSplit { more, .. } => {
                let mut v = vec![Controle { campo: 0, rotulo: "Fica deste lado" }];
                v.extend((0..more.len()).map(|i| Controle { campo: 2 + i as u8, rotulo: "Fica deste lado (corte seguinte)" }));
                v
            }
            RefKind::SurfTrim { .. } | RefKind::SurfFillet { .. } => vec![Controle { campo: 0, rotulo: "Fica da superfície 1" }, Controle { campo: 1, rotulo: "Fica da superfície 2" }],
            RefKind::CurveParallel { .. } => vec![Controle { campo: 0, rotulo: "Lado da paralela" }],
            RefKind::SurfRuled { .. } => vec![Controle { campo: 0, rotulo: "Sentido da faixa" }],
            // B562 (onda 2): irmã da extrusão sólida; o sentido é o sinal dos limites (`sinal_do_valor`)
            RefKind::SurfExtrude { .. } => vec![Controle { campo: 0, rotulo: "Sentido da extrusão" }],
            _ => vec![],
        },
        Alvo::Feature(k) => match k {
            FeatureKind::SplitBody(_) => vec![Controle { campo: 0, rotulo: "Fica deste lado" }],
            FeatureKind::SewSurface(x) if x.remove => vec![Controle { campo: 0, rotulo: "Fica deste lado" }],
            // simétrico: metade para cada lado, inverter não muda nada
            FeatureKind::ThickSurface(t) if !t.simetrico => vec![Controle { campo: 0, rotulo: "Lado da espessura" }],
            _ => vec![],
        },
    }
}

/// O inversor `campo` está ligado? Onde o inverter é uma troca (as duas espessuras), não há estado.
pub fn estado(alvo: Alvo, campo: u8) -> bool {
    match alvo {
        Alvo::Ref(k) => match k {
            RefKind::SurfSplit { flip, more_flip, .. } => match campo {
                0 => *flip,
                c => more_flip.get(c as usize - 2).copied().unwrap_or(*flip),
            },
            RefKind::SurfTrim { flip_a, flip_b, .. } | RefKind::SurfFillet { flip_a, flip_b, .. } => {
                if campo == 0 {
                    *flip_a
                } else {
                    *flip_b
                }
            }
            RefKind::CurveParallel { flip, .. } | RefKind::SurfRuled { flip, .. } => *flip,
            RefKind::SurfExtrude { length, .. } => extrusao_invertida(length),
            _ => false,
        },
        Alvo::Feature(k) => match k {
            FeatureKind::SplitBody(x) => x.flip,
            FeatureKind::SewSurface(x) => x.flip,
            // o engrossar inverte trocando as duas espessuras: está invertido quando toda a espessura
            // ficou do lado 2 (o botão aparece destacado, como nos outros comandos)
            FeatureKind::ThickSurface(t) if !t.simetrico => crate::sup3d::zerado(&t.t1) && !crate::sup3d::zerado(&t.t2),
            _ => false,
        },
    }
}

/// A única função que inverte o lado de um elemento de `references`. O clique na seta do 3D
/// (`sup3d::inverter_lado`) e o botão do painel (`botoes_ref`) chamam esta.
pub fn alternar_ref(k: &mut RefKind, campo: u8) {
    match k {
        RefKind::SurfSplit { flip, more, more_flip, .. } => {
            more_flip.resize(more.len(), *flip);
            match campo {
                0 => *flip = !*flip,
                c => {
                    if let Some(f) = more_flip.get_mut(c as usize - 2) {
                        *f = !*f;
                    }
                }
            }
        }
        RefKind::SurfTrim { flip_a, flip_b, .. } | RefKind::SurfFillet { flip_a, flip_b, .. } => {
            let f = if campo == 0 { flip_a } else { flip_b };
            *f = !*f;
        }
        RefKind::CurveParallel { flip, .. } | RefKind::SurfRuled { flip, .. } => *flip = !*flip,
        // o sinal dos dois limites troca junto: a faixa de -limite 2 a limite 1 vira o espelho dela
        RefKind::SurfExtrude { length, length2, .. } => alternar_extrusao(length, length2),
        _ => {}
    }
}

/// B562 (onda 2): a extrusão de superfície está invertida? O sentido é o sinal do limite 1
/// (`sinal_do_valor`): o botão do painel e a seta do 3D leem daqui.
pub fn extrusao_invertida(length: &str) -> bool {
    fenkra_core::expr::sinal_negativo(length)
}

/// B562 (onda 2): inverte a extrusão de superfície trocando o sinal dos dois limites; a faixa de
/// `-limite 2` a `limite 1` vira o espelho dela pela curva (a mesma coisa que a extrusão sólida faz com
/// `reverse`). O botão do painel e a seta do 3D chamam esta.
pub fn alternar_extrusao(length: &mut String, length2: &mut String) {
    *length = fenkra_core::expr::negar(length);
    *length2 = fenkra_core::expr::negar(length2);
}

/// A única função que inverte o lado de um comando de `features`. No engrossar, as duas espessuras
/// trocam de lado.
pub fn alternar_feature(k: &mut FeatureKind, campo: u8) {
    let _ = campo;
    match k {
        FeatureKind::SplitBody(x) => x.flip = !x.flip,
        FeatureKind::SewSurface(x) => x.flip = !x.flip,
        FeatureKind::ThickSurface(t) if !t.simetrico => std::mem::swap(&mut t.t1, &mut t.t2),
        _ => {}
    }
}

/// O widget padrão, para campo que é uma caixa de verdade (`reverse`, `flip`): o mesmo texto e ícone
/// do da Chapa. Devolve se foi clicado.
pub fn botao(ui: &mut egui::Ui, flip: &mut bool) -> bool {
    crate::chapa_unif_ui::inverter(ui, flip).clicked()
}

/// O botão de um inversor que troca por `alternar_*` (não é uma caixa só): destacado se estiver ligado.
fn botao_de(ui: &mut egui::Ui, ligado: bool, rotulo: Option<&str>) -> bool {
    let mut copia = ligado;
    let mut clicou = false;
    ui.horizontal(|ui| {
        if let Some(r) = rotulo {
            ui.label(RichText::new(tr(r)).weak().small());
        }
        clicou = crate::chapa_unif_ui::inverter(ui, &mut copia).clicked();
    });
    clicou
}

/// Os botões de inverter de um elemento de `references`, ligados ao mesmo campo da seta do 3D.
/// Devolve se algum foi clicado.
pub fn botoes_ref(ui: &mut egui::Ui, k: &mut RefKind) -> bool {
    let cs = controles(Alvo::Ref(k));
    let mut mudou = false;
    for c in &cs {
        let ligado = estado(Alvo::Ref(k), c.campo);
        if botao_de(ui, ligado, (cs.len() > 1).then_some(c.rotulo)) {
            alternar_ref(k, c.campo);
            mudou = true;
        }
    }
    mudou
}

/// Os botões de inverter de um comando de `features`.
pub fn botoes_feature(ui: &mut egui::Ui, k: &mut FeatureKind) -> bool {
    let cs = controles(Alvo::Feature(k));
    let mut mudou = false;
    for c in &cs {
        let ligado = estado(Alvo::Feature(k), c.campo);
        if botao_de(ui, ligado, (cs.len() > 1).then_some(c.rotulo)) {
            alternar_feature(k, c.campo);
            mudou = true;
        }
    }
    mudou
}

/// Os grupos de comandos irmãos que o painel trata do mesmo jeito (teste de paridade): dentro de um
/// grupo, todos têm de ter inversor.
#[cfg(test)]
pub fn grupos_irmaos() -> Vec<(&'static str, Vec<Vec<Controle>>)> {
    use crate::slots;
    let surf = || slots::surf();
    let sf = |f: u64| SurfRef::Ref(Id(f));
    let curva = || slots::curve();
    vec![
        (
            "corte de superfície: o lado que fica",
            vec![
                controles(Alvo::Ref(&RefKind::SurfSplit { surface: sf(1), tool: surf(), flip: false, more: vec![], curve: None, more_flip: vec![] })),
                controles(Alvo::Ref(&RefKind::SurfTrim { a: sf(1), b: surf(), flip_a: false, flip_b: false })),
                controles(Alvo::Ref(&RefKind::SurfFillet { a: sf(1), b: surf(), flip_a: false, flip_b: false, radius: "5".into(), radius2: None })),
                controles(Alvo::Feature(&FeatureKind::SplitBody(SplitBody { surface: sf(1), flip: false }))),
                controles(Alvo::Feature(&FeatureKind::SewSurface(SewSurface { surface: sf(1), remove: true, flip: false }))),
            ],
        ),
        (
            "lado de uma faixa ou paralela",
            vec![
                controles(Alvo::Ref(&RefKind::SurfRuled { curves: vec![curva()], support: None, mode: RuledMode::Normal, dir: AxisRef::Origin(OriginAxis::Z), length: "20".into(), flip: false })),
                controles(Alvo::Ref(&RefKind::CurveParallel { curve: curva(), support: sf(1), d1: "5".into(), d2: "5".into(), flip: false, more: vec![], both: false, d_other: String::new() })),
            ],
        ),
        (
            "sentido da extrusão de superfície e do engrossar (irmãos da extrusão sólida)",
            vec![
                controles(Alvo::Ref(&RefKind::SurfExtrude { curve: curva(), dir: AxisRef::Origin(OriginAxis::Z), length: "30".into(), length2: "0".into(), until: None, until2: None })),
                controles(Alvo::Feature(&FeatureKind::ThickSurface(ThickSurface::normal(sf(1), "2", "0")))),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slots;

    #[test]
    fn todo_comando_irmao_tem_inversor() {
        for (grupo, cmds) in grupos_irmaos() {
            for (i, c) in cmds.iter().enumerate() {
                assert!(!c.is_empty(), "o comando {i} do grupo '{grupo}' não tem inversor no painel");
            }
        }
    }

    #[test]
    fn irmaos_de_dois_lados_tem_um_inversor_por_lado() {
        let trim = RefKind::SurfTrim { a: slots::surf(), b: slots::surf(), flip_a: false, flip_b: false };
        let fil = RefKind::SurfFillet { a: slots::surf(), b: slots::surf(), flip_a: false, flip_b: false, radius: "5".into(), radius2: None };
        // Aparar e Filete entre superfícies são irmãos: mesmo conjunto de inversores, mesmos campos
        assert_eq!(controles(Alvo::Ref(&trim)), controles(Alvo::Ref(&fil)));
        assert_eq!(controles(Alvo::Ref(&trim)).iter().map(|c| c.campo).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn o_filete_entre_superficies_deixa_de_descartar_os_dois_lados() {
        // o painel antigo escrevia `let _ = (flip_a, flip_b)`: agora o botão troca cada lado
        let mut k = RefKind::SurfFillet { a: slots::surf(), b: slots::surf(), flip_a: false, flip_b: false, radius: "5".into(), radius2: None };
        alternar_ref(&mut k, 1);
        match &k {
            RefKind::SurfFillet { flip_a, flip_b, .. } => assert_eq!((*flip_a, *flip_b), (false, true)),
            _ => unreachable!(),
        }
        assert!(estado(Alvo::Ref(&k), 1) && !estado(Alvo::Ref(&k), 0));
    }

    #[test]
    fn dividir_com_varios_cortes_inverte_so_o_corte_pedido() {
        let mut k = RefKind::SurfSplit { surface: slots::surf(), tool: slots::surf(), flip: false, more: vec![slots::surf(), slots::surf()], curve: None, more_flip: vec![] };
        assert_eq!(controles(Alvo::Ref(&k)).iter().map(|c| c.campo).collect::<Vec<_>>(), vec![0, 2, 3]);
        alternar_ref(&mut k, 3);
        match &k {
            RefKind::SurfSplit { flip, more_flip, .. } => assert_eq!((*flip, more_flip.clone()), (false, vec![false, true])),
            _ => unreachable!(),
        }
    }

    #[test]
    fn engrossar_inverte_trocando_as_duas_espessuras_e_o_simetrico_nao_tem_botao() {
        let mut k = FeatureKind::ThickSurface(ThickSurface::normal(slots::surf(), "2", "0"));
        assert_eq!(controles(Alvo::Feature(&k)).len(), 1);
        alternar_feature(&mut k, 0);
        match &k {
            FeatureKind::ThickSurface(t) => assert_eq!((t.t1.as_str(), t.t2.as_str()), ("0", "2")),
            _ => unreachable!(),
        }
        if let FeatureKind::ThickSurface(t) = &mut k {
            t.simetrico = true;
        }
        assert!(controles(Alvo::Feature(&k)).is_empty());
    }

    /// Trecho do código-fonte entre dois marcadores (o braço de um comando no painel).
    fn braco<'a>(fonte: &'a str, de: &str, ate: &str) -> &'a str {
        let i = fonte.find(de).unwrap_or_else(|| panic!("não achei {de}"));
        let j = fonte[i..].find(ate).map(|j| i + j).unwrap_or_else(|| panic!("não achei {ate} depois de {de}"));
        &fonte[i..j]
    }

    /// ADR 0107, decisão 4 (onda 2): extrusão sólida, extrusão de superfície e engrossar são irmãos com
    /// as mesmas opções de direção. Os três têm **um** "⇄ Inverter direção" (o widget padrão), com um
    /// estado que liga e volta ao documento de antes; as duas extrusões mostram as mesmas seções na
    /// mesma ordem (Perfil, Extensão, o inverter logo abaixo da extensão) e o campo Direção.
    #[test]
    fn paridade_extrusao_solida_superficie_e_engrossar() {
        // estados: liga e volta (a sólida é a caixa `reverse`, conferida no painel logo abaixo)
        let sup0 = RefKind::SurfExtrude { curve: slots::curve(), dir: AxisRef::Origin(OriginAxis::Z), length: "30".into(), length2: "5".into(), until: None, until2: None };
        let mut sup = sup0.clone();
        assert_eq!(controles(Alvo::Ref(&sup)).len(), 1, "a extrusão de superfície tem um inversor");
        assert!(!estado(Alvo::Ref(&sup), 0));
        alternar_ref(&mut sup, 0);
        assert!(estado(Alvo::Ref(&sup), 0), "invertida, o botão aparece destacado");
        alternar_ref(&mut sup, 0);
        assert_eq!(sup, sup0, "inverter duas vezes volta ao documento de antes");
        let eng0 = FeatureKind::ThickSurface(ThickSurface::normal(slots::surf(), "2", "0"));
        let mut eng = eng0.clone();
        assert_eq!(controles(Alvo::Feature(&eng)).len(), 1, "o engrossar tem um inversor");
        alternar_feature(&mut eng, 0);
        assert!(estado(Alvo::Feature(&eng), 0));
        alternar_feature(&mut eng, 0);
        assert_eq!(eng, eng0);

        // painel: o mesmo widget, as mesmas seções, na mesma ordem
        let editor = braco(include_str!("editor.rs"), "FeatureKind::Extrude(e) => {", "FeatureKind::Revolve(r) => {");
        let geo = braco(include_str!("geo_ui.rs"), "R::SurfExtrude { curve, dir, length", "R::SurfRevolve {");
        for (nome, fonte, botao) in [("extrusão sólida", editor, "inverter::botao(ui, &mut e.reverse)"), ("extrusão de superfície", geo, "inverter::botao(ui, &mut inv)")] {
            let pos = |x: &str| fonte.find(x).unwrap_or_else(|| panic!("{nome} sem {x}"));
            let (perfil, extensao, inv) = (pos("section(ui, \"Perfil\")"), pos("section(ui, \"Extensão\")"), pos(botao));
            assert!(perfil < extensao && extensao < inv, "{nome}: Perfil, Extensão e o inverter, nessa ordem");
            assert!(fonte.contains("\"Direção\""), "{nome} sem o campo Direção");
        }
    }

    /// Paridade dos comandos de troca direta (Extrusão, Furo, Ângulo de saída, Nervura e os recursos de
    /// plástico): nenhum tem caixa de marcar própria, todos usam o botão padrão no campo de sempre.
    #[test]
    fn comandos_de_troca_direta_usam_o_botao_padrao() {
        let fontes = [("editor.rs", include_str!("editor.rs")), ("feat_ui.rs", include_str!("feat_ui.rs")), ("plastico_rec_ui.rs", include_str!("plastico_rec_ui.rs"))];
        for (nome, fonte) in fontes {
            for proibido in ["\"inverter sentido\"", "\"inverter a direção de extração\"", "\"para o lado da normal do plano\"", "\"para o outro lado do plano\""] {
                assert!(!fonte.contains(proibido), "{nome} ainda tem a caixa {proibido}");
            }
        }
        let tudo: String = fontes.iter().map(|f| f.1).collect();
        for esperado in ["inverter::botao(ui, &mut e.reverse)", "inverter::botao(ui, &mut h.reverse)", "inverter::botao(ui, &mut d.reverse)", "inverter::botao(ui, &mut st.reverse)"] {
            assert!(tudo.contains(esperado), "falta {esperado}");
        }
    }
}
