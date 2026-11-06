// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do Desenho: cortes completos e vistas especiais (B106), cota inteligente e chamadas (B107).

pub static TEXTOS: &[(&str, &str)] = &[
    // B106: linha de corte, fantasma e opções
    ("Clique o primeiro ponto da linha de corte", "Click the first point of the section line"),
    ("Clique o segundo ponto da linha de corte", "Click the second point of the section line"),
    ("Clique mais pontos; Enter ou duplo clique conclui a linha", "Click more points; Enter or double-click finishes the line"),
    ("Clique onde a vista em corte vai ficar (Ctrl solta o alinhamento)", "Click where the section view goes (Ctrl releases the alignment)"),
    ("Clique o primeiro ponto do contorno do detalhe", "Click the first point of the detail outline"),
    ("Clique os pontos do contorno; Enter ou duplo clique conclui", "Click the outline points; Enter or double-click finishes"),
    ("Clique onde a vista de detalhe vai ficar", "Click where the detail view goes"),
    ("Profundidade", "Depth"),
    ("Distância", "Distance"),
    ("Total", "Full"),
    ("quanto do material atrás do plano de corte aparece", "how much material behind the cutting plane is shown"),
    ("Fatia", "Slice"),
    ("Borda", "Edge"),
    ("Lisa", "Smooth"),
    ("Serrilhada", "Jagged"),
    ("Corte", "Section"),
    ("Fatia mostra só a face cortada. Borda serrilhada desenha a face com traço de ruptura.", "Slice shows only the cut face. A jagged edge draws the face with a break line."),
    ("Linha reta: arraste as pontas na folha; clique no ponto do meio para criar um vértice (corte em degraus ou meio corte).", "Straight line: drag the ends on the sheet; click the midpoint to add a vertex (offset or half section)."),
    ("vértices na linha de corte; botão direito num vértice remove.", "vertices on the section line; right-click a vertex to remove it."),
    ("Peças que não são cortadas", "Parts that are not cut"),
    ("a peça aparece inteira, sem corte nem hachura", "the part is shown whole, with no cut or hatching"),
    ("Clique para criar um vértice; arraste para deslocar o segmento", "Click to add a vertex; drag to move the segment"),
    ("Arraste para mover o ponto da linha de corte; botão direito remove", "Drag to move the section line point; right-click removes it"),
    ("⇄ Inverter o lado visto", "⇄ Flip the viewed side"),
    ("troca o sentido da linha de corte: a vista passa a olhar para o lado oposto", "reverses the section line: the view now looks at the opposite side"),
    ("Arraste para mover o ponto do contorno; botão direito remove", "Drag to move the outline point; right-click removes it"),
    ("Contorno", "Outline"),
    ("Círculo", "Circle"),
    ("Perfil livre", "Free profile"),
    ("Selecione a vista, trace a linha de corte (clique em cada ponto; Enter conclui) e posicione: face cortada hachurada.", "Select the view, trace the section line (click each point; Enter finishes) and place it: the cut face is hatched."),
    // B107: cota inteligente
    ("Inteligente", "Smart"),
    ("Cota inteligente", "Smart dimension"),
    ("Uma ferramenta só: o arco, a aresta ou os dois vértices sob o mouse propõem o tipo; Tab troca; clique fixa o afastamento. Segue ativa até Esc ou clique direito.", "One tool: the arc, edge or two vertices under the mouse propose the type; Tab switches; a click sets the offset. It stays active until Esc or right-click."),
    ("Passe o mouse num arco, aresta ou vértice e clique", "Hover an arc, edge or vertex and click"),
    ("Clique onde a cota fica (Tab troca o tipo; clique direito cancela)", "Click where the dimension goes (Tab switches the type; right-click cancels)"),
    ("Clique o segundo vértice", "Click the second vertex"),
    ("Clique um arco, uma aresta ou dois vértices", "Click an arc, an edge or two vertices"),
    ("Tab troca o tipo da cota", "Tab switches the dimension type"),
    ("Chamada do furo", "Hole callout"),
    ("Passe o cursor sobre uma vista", "Move the cursor over a view"),
    ("Sem aresta ou vértice aqui", "No edge or vertex here"),
    ("A vista não tem projeção válida", "The view has no valid projection"),
    ("Use a mesma vista da primeira seleção", "Use the same view as the first selection"),
    ("Precisão própria da tolerância", "Own tolerance precision"),
    ("Como diâmetro (Ø)", "As diameter (Ø)"),
    ("Entre parênteses (referência)", "In parentheses (reference)"),
    ("Inspeção (oval)", "Inspection (oval)"),
];

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn toda_frase_nova_tem_traducao_nao_vazia() {
        for (pt, en) in TEXTOS {
            assert!(!pt.is_empty() && !en.is_empty(), "{pt}");
        }
        // os motivos da cota inteligente vêm de `Motivo::texto` e precisam estar na tabela
        use fenkra_desenho::cotas::Motivo;
        for m in [Motivo::ForaDeVista, Motivo::SemAresta, Motivo::VistaSemProjecao, Motivo::OutraVista] {
            assert!(TEXTOS.iter().any(|(pt, _)| *pt == m.texto()), "{}", m.texto());
        }
    }
}
