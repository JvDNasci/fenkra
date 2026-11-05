// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fluxo comum das ferramentas de sketch (B239).
//!
//! Uma tabela diz, para cada ferramenta: os passos (texto de cada clique), se ela aproveita a
//! seleção feita antes de escolhê-la, se repete até Esc e o que o Esc faz em cada situação. As
//! dicas junto ao cursor, a linha de próximo passo da barra de status e o teste que percorre todas
//! as ferramentas usam esta mesma tabela, então nenhuma ferramenta nova fica sem dica, tradução
//! ou tratamento de seleção prévia.

use crate::i18n::tr;
use crate::sketch_ui::{edit_hint, shape_hint, CKind, Tool};

/// Todas as ferramentas, incluindo cada restrição.
pub fn todas() -> Vec<Tool> {
    use CKind::*;
    let mut v = vec![
        Tool::Select,
        Tool::Line,
        Tool::Rect,
        Tool::RectCenter,
        Tool::Circle,
        Tool::Circle3,
        Tool::Arc3,
        Tool::ArcCenter,
        Tool::Slot,
        Tool::Polygon,
        Tool::Point,
        Tool::Dimension,
        Tool::DimH,
        Tool::DimV,
        Tool::DimAngle,
        Tool::DimRadius,
        Tool::DimDiameter,
        Tool::Move,
        Tool::Copy,
        Tool::Mirror,
        Tool::Offset,
        Tool::Fillet,
        Tool::Chamfer,
        Tool::Spline,
        Tool::Trim,
        Tool::Extend,
        Tool::Convert,
        Tool::Intersect,
    ];
    v.extend([Coincident, Horizontal, Vertical, Parallel, Perpendicular, Tangent, Equal, Concentric, Midpoint, Fixed, Symmetric].map(Tool::Constr));
    v
}

/// Ferramenta de cota (começa com a seleção prévia como o primeiro elemento).
pub fn cota(t: Tool) -> bool {
    matches!(t, Tool::Dimension | Tool::DimH | Tool::DimV | Tool::DimAngle | Tool::DimRadius | Tool::DimDiameter)
}

/// Texto de cada passo da ferramenta, na ordem (sem traduzir: passe por `tr`).
pub fn passos(t: Tool) -> Vec<&'static str> {
    match t {
        Tool::Select => vec!["clique ou arraste para selecionar; Shift soma"],
        Tool::Line => vec!["clique no ponto inicial", "clique no próximo ponto; no 1º ponto fecha o perfil"],
        Tool::Rect | Tool::RectCenter | Tool::Circle | Tool::Circle3 | Tool::Arc3 | Tool::ArcCenter | Tool::Slot | Tool::Polygon | Tool::Point => {
            (0..t.clicks_needed().max(1)).map(|n| shape_hint(t, n)).collect()
        }
        Tool::Spline => vec!["clique nos pontos da spline", "Enter / duplo clique / botão direito termina"],
        Tool::Move | Tool::Copy => vec![edit_hint(t, true, true, 0), edit_hint(t, false, false, 0), edit_hint(t, false, false, 1)],
        Tool::Mirror => vec![edit_hint(t, true, true, 0), edit_hint(t, false, false, 0)],
        Tool::Offset => vec![edit_hint(t, false, true, 0), edit_hint(t, false, false, 0)],
        Tool::Fillet | Tool::Chamfer => vec![edit_hint(t, false, true, 0)],
        Tool::Trim => vec!["clique no trecho a remover; Shift estende; clique no vazio encerra"],
        Tool::Extend => vec!["clique perto da ponta a estender; Shift apara; clique no vazio encerra"],
        Tool::Convert => vec!["clique numa aresta, vértice ou curva do modelo"],
        Tool::Intersect => vec!["clique numa face, superfície ou aresta que corta o plano"],
        Tool::Dimension => vec!["clique num elemento", "clique no vazio para posicionar a cota"],
        Tool::DimH | Tool::DimV => vec!["clique num ponto ou linha", "clique no vazio para posicionar a cota"],
        Tool::DimAngle => vec!["clique na 1ª linha", "clique na 2ª linha"],
        Tool::DimRadius | Tool::DimDiameter => vec!["clique num círculo ou arco", "clique no vazio para posicionar a cota"],
        Tool::Constr(k) => match k {
            CKind::Horizontal | CKind::Vertical | CKind::Fixed => vec!["clique no elemento"],
            CKind::Symmetric => vec!["clique no 1º ponto", "clique no 2º ponto", "clique na linha de simetria"],
            _ => vec!["clique no 1º elemento", "clique no 2º elemento"],
        },
    }
}

/// A ferramenta aproveita o que já estava selecionado ao ser escolhida (seleção prévia).
///
/// Offset parte da curva selecionada; Filete e Chanfro agem no canto das duas linhas selecionadas;
/// Aparar e Estender só agem nas curvas selecionadas; as cotas começam com a seleção; as restrições
/// somam a seleção aos cliques; Mover, Copiar e Espelhar usam as curvas já escolhidas.
pub fn usa_selecao(t: Tool) -> bool {
    !matches!(
        t,
        Tool::Line | Tool::Rect | Tool::RectCenter | Tool::Circle | Tool::Circle3 | Tool::Arc3 | Tool::ArcCenter | Tool::Slot | Tool::Polygon | Tool::Point | Tool::Spline | Tool::Convert | Tool::Intersect
    )
}

/// Depois de concluir um objeto a ferramenta continua ativa até Esc (só Selecionar é o repouso).
pub fn repete(t: Tool) -> bool {
    t != Tool::Select
}

/// Quantos cliques tem a ferramenta quando a contagem "1 de 3" faz sentido (passos finitos).
pub fn total(t: Tool) -> Option<usize> {
    let n = passos(t).len();
    (n > 1 && !matches!(t, Tool::Line | Tool::Spline)).then_some(n)
}

/// O que o Esc faz agora, igual ao que `SketchEdit::cancel` executa: cancela o passo em curso;
/// sem passo, volta à seleção; em Selecionar, limpa a seleção e, sem seleção, sai do sketch.
pub fn esc_texto(t: Tool, em_passo: bool, tem_selecao: bool) -> &'static str {
    if em_passo {
        "Esc cancela o passo"
    } else if t != Tool::Select {
        "Esc volta à seleção"
    } else if tem_selecao {
        "Esc limpa a seleção"
    } else {
        "Esc sai do sketch"
    }
}

/// Dica junto ao cursor: "Simétrica: 1 de 3 · clique no 1º ponto · Esc cancela o passo".
/// `passo` é o índice (0 a n-1) do passo atual; `texto` substitui o texto padrão do passo quando a
/// ferramenta decide o passo pelo estado (Mover, Offset...).
pub fn dica(t: Tool, passo: usize, texto: Option<&'static str>, em_passo: bool, tem_selecao: bool) -> String {
    let ps = passos(t);
    let corpo = texto.unwrap_or_else(|| ps[passo.min(ps.len() - 1)]);
    let esc = esc_texto(t, em_passo, tem_selecao);
    match total(t) {
        Some(n) => format!("{}: {} {} {n} · {} · {}", tr(t.info().1), passo.min(n - 1) + 1, tr("de"), tr(corpo), tr(esc)),
        None => format!("{} · {}", tr(corpo), tr(esc)),
    }
}

/// Recusa de um clique que não acertou nada.
pub fn recusa_vazio(t: Tool) -> &'static str {
    if matches!(t, Tool::Constr(_)) {
        "nada sob o cursor: clique num ponto ou curva do sketch"
    } else {
        "nada sob o cursor"
    }
}

/// Recusa quando a seleção prévia limita a ferramenta e o clique caiu fora dela.
pub const FORA_DA_SELECAO: &str = "fora da seleção prévia: Esc limpa a seleção para agir em qualquer curva";

/// Todos os textos novos do fluxo, para a tabela de traduções e o teste.
pub fn textos() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = todas().into_iter().flat_map(passos).collect();
    v.extend(["Esc cancela o passo", "Esc volta à seleção", "Esc limpa a seleção", "Esc sai do sketch", "de", FORA_DA_SELECAO]);
    v.extend([recusa_vazio(Tool::Select), recusa_vazio(Tool::Constr(CKind::Parallel))]);
    v
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn toda_ferramenta_tem_passos_e_traducao() {
        for t in todas() {
            assert!(!passos(t).is_empty(), "{t:?} sem passos");
            assert!(passos(t).iter().all(|p| !p.is_empty()), "{t:?} com passo vazio");
        }
        let faltam: Vec<&str> = textos().into_iter().filter(|x| !crate::i18n::tem_traducao(x)).collect();
        assert!(faltam.is_empty(), "sem tradução: {faltam:?}");
    }

    #[test]
    fn todas_as_ferramentas_estao_na_lista() {
        // `info` não tem braço padrão: ferramenta nova sem entrada aqui quebra a compilação lá;
        // aqui conferimos que cada nome da lista é único.
        let nomes: Vec<String> = todas().iter().map(|t| format!("{t:?}")).collect();
        let mut ordenados = nomes.clone();
        ordenados.sort();
        ordenados.dedup();
        assert_eq!(nomes.len(), ordenados.len());
        assert_eq!(nomes.len(), 28 + 11);
    }

    #[test]
    fn selecao_previa_esta_decidida_para_cada_ferramenta() {
        for t in todas() {
            let usa = usa_selecao(t);
            let desenha = matches!(t, Tool::Line | Tool::Rect | Tool::RectCenter | Tool::Circle | Tool::Circle3 | Tool::Arc3 | Tool::ArcCenter | Tool::Slot | Tool::Polygon | Tool::Point | Tool::Spline | Tool::Convert | Tool::Intersect);
            assert_eq!(usa, !desenha, "{t:?}");
        }
        // as que o pedido nomeia
        for t in [Tool::Offset, Tool::Fillet, Tool::Chamfer, Tool::Trim, Tool::Extend, Tool::Dimension, Tool::DimH, Tool::DimV, Tool::DimAngle, Tool::DimRadius, Tool::DimDiameter] {
            assert!(usa_selecao(t), "{t:?} deveria aceitar seleção prévia");
        }
    }

    #[test]
    fn restricao_conta_os_cliques() {
        let t = Tool::Constr(CKind::Symmetric);
        assert_eq!(total(t), Some(3));
        let d = dica(t, 0, None, false, false);
        assert!(d.starts_with("Simétrica: 1 de 3"), "{d}");
        let d = dica(t, 2, None, true, true);
        assert!(d.contains("3 de 3") && d.contains("Esc cancela o passo"), "{d}");
        // uma restrição de um clique não mostra contagem
        assert_eq!(total(Tool::Constr(CKind::Horizontal)), None);
    }

    #[test]
    fn esc_diz_o_que_cancel_faz() {
        assert_eq!(esc_texto(Tool::Line, true, false), "Esc cancela o passo");
        assert_eq!(esc_texto(Tool::Line, false, false), "Esc volta à seleção");
        assert_eq!(esc_texto(Tool::Select, false, true), "Esc limpa a seleção");
        assert_eq!(esc_texto(Tool::Select, false, false), "Esc sai do sketch");
    }
}
