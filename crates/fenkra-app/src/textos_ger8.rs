// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções da leva Geral e PDM/PLM (B130 ramos e cópia derivada, B305 perfil por regra), à parte
//! de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    ("principal andou:", "main line moved:"),
    ("Trazer a principal", "Bring in main line"),
    ("Traz para o ramo o que a principal mudou. Onde os dois mexeram no mesmo valor, fica o do ramo; a principal não muda.", "Brings into the branch what the main line changed. Where both changed the same value, the branch keeps its own; the main line does not change."),
    ("Cópia derivada", "Derived copy"),
    ("Cria um documento novo a partir da principal, com vida própria; depois ele pode voltar ao original como ramo", "Creates a new document from the main line, with a life of its own; later it can go back to the original as a branch"),
    ("Cópia derivada de", "Derived copy of"),
    ("Levar ao original como ramo", "Take to the original as a branch"),
    ("A principal desta cópia vira um ramo do original, com o nome acima; daí segue o pedido de mesclagem", "The main line of this copy becomes a branch of the original, with the name above; then the merge request follows"),
    ("Principal trazida para o ramo", "Main line brought into the branch"),
    ("conflito(s) ficaram com o ramo", "conflict(s) kept the branch value"),
    ("Cópia derivada criada", "Derived copy created"),
    ("Levada ao original como ramo", "Taken to the original as a branch"),
    ("Regras do perfil", "Profile rules"),
    ("Quando roda", "When it runs"),
    ("Perfil travado: cada mudança só pode deixar a regra mais rígida (limite menor ou gravidade maior).", "Locked profile: each change can only make the rule stricter (lower limit or higher severity)."),
    ("Perfil livre: quem tem acesso ao projeto ajusta os limites.", "Unlocked profile: anyone with project access adjusts the limits."),
];
