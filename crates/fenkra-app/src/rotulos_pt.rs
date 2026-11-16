// SPDX-License-Identifier: LGPL-2.1-or-later
//! B475: rótulo em português não leva o termo inglês copiado entre parênteses ("Mover o corpo
//! (Translation)"). O termo vem do glossário em português (`docs/produto/glossario.md`); a versão em
//! inglês fica na tradução de `i18n.rs`, não no rótulo.

/// Palavras inglesas que já apareceram copiadas em rótulos. Um parêntese só com elas é reprovado.
const INGLES: &[&str] = &[
    "translation", "rotation", "symmetry", "affinity", "shaft", "groove", "rib", "stiffener", "draft", "shell", "thickness", "mirror", "scale", "split", "sew", "surface", "boundary", "fill", "user", "pattern", "loft",
    "lofted", "gusset", "extrude", "sweep", "revolve", "fillet", "chamfer", "offset", "trim", "mate", "joint", "sketch", "hole", "flange", "bend", "pad", "pocket", "move", "rotate", "copy", "array", "thread", "tap", "joggle",
];

/// Primeiro parêntese do rótulo formado só por palavras inglesas, se houver.
pub fn termo_ingles_entre_parenteses(rotulo: &str) -> Option<String> {
    let mut resto = rotulo;
    while let Some(i) = resto.find('(') {
        let depois = &resto[i + 1..];
        let Some(f) = depois.find(')') else { break };
        let dentro = &depois[..f];
        let palavras: Vec<String> = dentro.split(|c: char| c == ' ' || c == '-' || c == '/').filter(|p| !p.is_empty()).map(|p| p.to_lowercase()).collect();
        if !palavras.is_empty() && palavras.iter().all(|p| INGLES.contains(&p.as_str())) {
            return Some(dentro.to_owned());
        }
        resto = &depois[f + 1..];
    }
    None
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::toolbar::{all_entries, entry_title, Entry};

    #[test]
    fn o_detector_acha_so_o_termo_ingles() {
        assert_eq!(termo_ingles_entre_parenteses("Mover o corpo (Translation)").as_deref(), Some("Translation"));
        assert_eq!(termo_ingles_entre_parenteses("Costurar superfície (Sew Surface)").as_deref(), Some("Sew Surface"));
        assert_eq!(termo_ingles_entre_parenteses("Padrão por pontos (user pattern)").as_deref(), Some("user pattern"));
        assert_eq!(termo_ingles_entre_parenteses("Mover o corpo (translação)"), None);
        assert_eq!(termo_ingles_entre_parenteses("Concluir sketch  (Esc)"), None);
        assert_eq!(termo_ingles_entre_parenteses("Lista de materiais (BOM)"), None);
    }

    /// Nenhum nome nem frase da barra leva termo inglês entre parênteses.
    #[test]
    fn rotulos_da_barra_nao_copiam_termo_ingles() {
        let mut ruins = vec![];
        for e in all_entries() {
            let itens = match &e {
                Entry::One(i) => vec![*i],
                Entry::Family(_, v) => v.clone(),
            };
            for i in itens {
                for texto in [i.name, i.brief] {
                    if let Some(t) = termo_ingles_entre_parenteses(texto) {
                        ruins.push(format!("{texto:?} ({t})"));
                    }
                }
            }
        }
        assert!(ruins.is_empty(), "rótulos com termo inglês entre parênteses: {ruins:#?}");
    }

    /// B483: termos ingleses soltos (fora de parênteses) que já escaparam para tooltip ou item de menu.
    const SOLTOS: &[&str] = &["joggle", "thread", "tap"];

    fn palavra_solta(texto: &str) -> Option<&'static str> {
        let baixo = texto.to_lowercase();
        let palavras: Vec<&str> = baixo.split(|c: char| !c.is_alphanumeric()).collect();
        SOLTOS.iter().copied().find(|t| palavras.contains(t))
    }

    #[test]
    fn o_detector_acha_termo_solto_e_parentese_com_barra() {
        assert_eq!(termo_ingles_entre_parenteses("Rosca em eixo ou furo (Thread/Tap)").as_deref(), Some("Thread/Tap"));
        assert_eq!(palavra_solta("Joggle (desvio em Z)"), Some("joggle"));
        assert_eq!(palavra_solta("Desvio em Z (degrau na chapa)"), None);
    }

    /// B483: tooltip (frase curta e passo a passo), nome e item de menu de todo comando da barra, e o
    /// título de cada família (o menu da barra), estão em português: sem termo inglês copiado.
    #[test]
    fn tooltips_e_itens_de_menu_nao_levam_termo_ingles() {
        let mut ruins = vec![];
        for e in all_entries() {
            let mut textos: Vec<&str> = vec![entry_title(&e).1];
            let itens = match &e {
                Entry::One(i) => vec![*i],
                Entry::Family(_, v) => v.clone(),
            };
            for i in &itens {
                textos.extend([i.name, i.brief, i.advanced]);
            }
            for t in textos {
                if let Some(x) = termo_ingles_entre_parenteses(t) {
                    ruins.push(format!("{t:?} (parêntese: {x})"));
                } else if let Some(x) = palavra_solta(t) {
                    ruins.push(format!("{t:?} (termo: {x})"));
                }
            }
        }
        ruins.sort();
        ruins.dedup();
        assert!(ruins.is_empty(), "textos com termo inglês: {ruins:#?}");
    }

    /// As variantes de mover do B475 usam o termo em português e têm tradução.
    #[test]
    fn variantes_de_mover_em_portugues_com_traducao() {
        let achadas: Vec<&str> = all_entries()
            .iter()
            .filter_map(|e| if let Entry::Family(k, v) = e { (*k == "mover_corpo").then(|| v.iter().map(|i| i.brief).collect::<Vec<_>>()) } else { None })
            .flatten()
            .collect();
        assert_eq!(achadas, ["Mover o corpo (translação)", "Girar o corpo (rotação)", "Simetria do corpo (reflexão)", "Afinidade (escala por eixo)"]);
        for r in achadas {
            assert!(crate::i18n::tem_traducao(r), "{r} sem tradução");
        }
    }
}
