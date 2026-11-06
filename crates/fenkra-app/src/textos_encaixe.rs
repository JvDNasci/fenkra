// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções do corte de encaixe normal entre membros (B155). Tabela própria para não disputar
//! `i18n.rs` com outras bancadas.

pub static TEXTOS: &[(&str, &str)] = &[
    ("Corte de encaixe (laser ou plasma em tubo)", "Cope cut (tube laser or plasma)"),
    ("Parede", "Wall"),
    ("Parede interna", "Inner wall"),
    ("Parede externa", "Outer wall"),
    ("Média das paredes", "Mean of the walls"),
    ("parede externa", "outer wall"),
    ("parede interna", "inner wall"),
    ("média das paredes", "mean of the walls"),
    ("sem contato", "no contact"),
    (
        "Automático: ponto a ponto, a parede que toca primeiro; nunca invade o outro tubo e dá a menor abertura. Externa, interna e média escolhem uma só, e a invasão, se houver, é avisada.",
        "Automatic: point by point, the wall that touches first; it never enters the other tube and gives the smallest opening. Outer, inner and mean pick a single one, and any overlap is reported.",
    ),
    ("Orçamento de folga", "Gap budget"),
    (
        "Abertura de raiz máxima admissível, medida na normal do outro tubo. Os trechos acima dela ficam em vermelho: só um chanfro resolve.",
        "Largest acceptable root opening, measured on the other tube's normal. Stretches above it turn red: only a bevel fixes them.",
    ),
    ("Corte perpendicular à parede", "Cut square to the wall"),
    (
        "Ligado: laser de 3 eixos, uma curva só para a espessura inteira (sobra abertura de raiz). Desligado: com chanfro, a face de corte acompanha o outro tubo e a abertura é zero.",
        "On: 3-axis laser, one curve for the whole thickness (a root opening is left). Off: with bevel, the cut face follows the other tube and the opening is zero.",
    ),
    ("Sem curva neste canto: a seção ainda não é tubo ou barra, ou os eixos são paralelos.", "No curve at this corner: the section is not a tube or bar yet, or the axes are parallel."),
    ("Comprimento de corte:", "Cut length:"),
    ("abertura máxima:", "largest opening:"),
    ("Toda a curva cabe no orçamento de folga.", "The whole curve fits the gap budget."),
    ("trecho(s) acima do orçamento", "stretch(es) over budget"),
    ("use chanfro ou aumente o orçamento", "use a bevel or raise the budget"),
    ("A parede escolhida invade o outro tubo em até", "The chosen wall enters the other tube by up to"),
    ("Chanfro necessário, no máximo:", "Bevel needed, at most:"),
    ("Tubo aberto com a curva de encaixe das pontas, para laser de tubo. Camada CORTE, em milímetros.", "Open tube with the cope curve of the ends, for tube laser. CORTE layer, in millimetres."),
    ("O mesmo contorno em SVG, em milímetros reais.", "The same outline as SVG, in real millimetres."),
    ("Sem protótipo: a validar em peça real.", "No prototype yet: to be validated on a real part."),
    ("abertura", "opening"),
    ("chanfro ideal", "ideal bevel"),
    ("chanfro", "bevel"),
];
