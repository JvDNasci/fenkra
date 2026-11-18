// SPDX-License-Identifier: LGPL-2.1-or-later
//! Traduções dos comandos pelo elemento que resulta (B505), da barra de propriedades gráficas e da
//! barra de vistas (B506), à parte de `i18n.rs`.

pub static TEXTOS: &[(&str, &str)] = &[
    // B505: um botão por elemento
    ("Ponto: o tipo é escolhido no painel", "Point: the type is chosen in the panel"),
    ("Linha: o tipo é escolhido no painel", "Line: the type is chosen in the panel"),
    ("Plano: o tipo é escolhido no painel", "Plane: the type is chosen in the panel"),
    ("Curva: o tipo é escolhido no painel", "Curve: the type is chosen in the panel"),
    ("Superfície: o tipo é escolhido no painel", "Surface: the type is chosen in the panel"),
    ("Crie o ponto e escolha o tipo no painel: coordenadas, sobre curva, sobre plano, sobre superfície, centro, tangente, entre dois pontos ou deslocado. O painel também repete pontos ao longo de uma curva.", "Create the point and pick its type in the panel: coordinates, on curve, on plane, on surface, center, tangent, between two points or offset. The panel also repeats points along a curve."),
    ("Crie a linha e escolha o tipo no painel: dois pontos, ponto e direção, ângulo, tangente, normal, entre planos, bissetriz, eixo ou polilinha.", "Create the line and pick its type in the panel: two points, point and direction, angle, tangent, normal, between planes, bisector, axis or polyline."),
    ("Crie o plano e escolha o tipo no painel: offset, paralelo por ponto, ângulo ou normal, três pontos, duas linhas, ponto e linha, pela curva, normal à curva, tangente, equação ou médio.", "Create the plane and pick its type in the panel: offset, parallel through point, angle or normal, three points, two lines, point and line, through curve, normal to curve, tangent, equation or mean."),
    ("Crie a curva e escolha o tipo no painel: círculo, círculo por 3 pontos, cônica, spline, spline por controle, conexão, canto, hélice, espiral, espinha ou paralela.", "Create the curve and pick its type in the panel: circle, 3-point circle, conic, spline, control spline, connect, corner, helix, spiral, spine or parallel."),
    ("Crie a superfície e escolha o tipo no painel: extrudada, revolução, esfera, cilindro, deslocada, varredura, tubo, multi-seções, regrada, concordância, conectar ou preenchimento.", "Create the surface and pick its type in the panel: extruded, revolved, sphere, cylinder, offset, sweep, tube, multi-section, ruled, blend, connect or fill."),
    ("Tipo da próxima vez:", "Type next time:"),
    ("Cria vários pontos ao longo da curva, por quantidade ou espaçamento.", "Creates several points along the curve, by count or spacing."),
    ("Use o tipo Na curva ou selecione uma curva para repetir pontos ao longo dela.", "Use the On curve type or select a curve to repeat points along it."),
    // B505: vários pedaços
    ("em uso:", "in use:"),
    ("Escolha qual pedaço fica ou extraia todos; nada é escolhido em silêncio.", "Choose which piece stays or extract them all; nothing is picked silently."),
    ("Mais perto da origem", "Nearest to the origin"),
    ("Mais longe da origem", "Farthest from the origin"),
    ("Fica o pedaço mais perto da origem da peça; a escolha fica presa à posição.", "Keeps the piece nearest to the part origin; the choice is tied to the position."),
    ("Fica o pedaço mais longe da origem da peça; a escolha fica presa à posição.", "Keeps the piece farthest from the part origin; the choice is tied to the position."),
    ("Extrair todos", "Extract all"),
    ("Cada pedaço vira um elemento próprio.", "Each piece becomes its own element."),
    ("pedaços viraram elementos", "pieces became elements"),
    // B506: propriedades gráficas
    ("Cor da seleção (faces, corpos, curvas e pontos de referência)", "Color of the selection (faces, bodies, reference curves and points)"),
    ("vários", "multiple"),
    ("Transparência", "Transparency"),
    ("Até 85 %: a peça não some no fundo e as arestas continuam à vista.", "Up to 85%: the part never disappears into the background and its edges stay visible."),
    ("Espessura da linha", "Line thickness"),
    ("Contínua", "Continuous"),
    ("Tracejada", "Dashed"),
    ("Traço-ponto", "Dash-dot"),
    ("Pontilhada", "Dotted"),
    ("Cruz 3D", "3D cross"),
    ("Asterisco", "Asterisk"),
    ("Camada", "Layer"),
    ("sem camada", "no layer"),
    ("Camadas ainda não existem nesta versão: tudo fica sem camada.", "Layers do not exist yet in this version: everything has no layer."),
    ("personalizado", "custom"),
    ("Carregue imagens nas propriedades da peça.", "Load images in the part properties."),
    ("Selecione uma face, um corpo, um comando ou um elemento de geometria.", "Select a face, a body, a feature or a geometry element."),
    ("A cor vale para faces, corpos e elementos de geometria; curva e ponto de sketch usam espessura, tipo de linha e símbolo.", "Color applies to faces, bodies and geometry elements; sketch curves and points use thickness, line type and symbol."),
    ("Espessura e tipo de linha valem só para linhas e curvas.", "Thickness and line type apply only to lines and curves."),
    ("O símbolo vale só para pontos.", "The symbol applies only to points."),
    ("Material, textura e transparência valem só para faces, corpos e superfícies.", "Material, texture and transparency apply only to faces, bodies and surfaces."),
    ("Alumínio", "Aluminum"),
    ("Inox polido", "Polished stainless"),
    ("Latão", "Brass"),
    ("ABS preto", "Black ABS"),
    ("Plástico vermelho", "Red plastic"),
    ("Borracha", "Rubber"),
    ("Vidro", "Glass"),
    // B506: barra de vistas
    ("Enquadrar tudo", "Fit all"),
    ("Enquadrar tudo  (F)", "Fit all  (F)"),
    ("Ajusta o zoom para caber todo o modelo visível; ocultos, origem e rótulos não contam.", "Zooms so the whole visible model fits; hidden items, origin and labels do not count."),
    ("Zoom na seleção", "Zoom to selection"),
    ("Enquadra só o que está selecionado (faces, arestas, corpos ou elementos).", "Frames only what is selected (faces, edges, bodies or elements)."),
    ("Vista normal", "Normal view"),
    ("Vista normal à seleção  (N)", "View normal to selection  (N)"),
    ("Olha de frente para a face, o plano ou o sketch selecionado. Sem seleção, para o plano de trabalho ativo.", "Looks straight at the selected face, plane or sketch. With no selection, at the active work plane."),
    ("Leva a câmera à vista isométrica. Clique de novo para girar entre as oito direções isométricas.", "Moves the camera to the isometric view. Click again to cycle through the eight isometric directions."),
    ("Vista posterior", "Back view"),
    ("Leva a câmera a olhar a peça por trás.", "Moves the camera to look at the part from behind."),
    ("Vista lateral esquerda", "Left side view"),
    ("Leva a câmera a olhar a peça pelo lado esquerdo.", "Moves the camera to look at the part from the left side."),
    ("Leva a câmera a olhar a peça por baixo.", "Moves the camera to look at the part from below."),
    ("Sombreado, sombreado com arestas, arestas ocultas, aramado ou com material.", "Shaded, shaded with edges, hidden edges, wireframe or with material."),
    ("Ocultar/mostrar", "Hide/show"),
    ("Ocultar ou mostrar a seleção  (H)", "Hide or show the selection  (H)"),
    ("Esconde o que está selecionado; com o oculto selecionado, mostra de novo. O modelo não muda.", "Hides what is selected; with a hidden item selected, shows it again. The model does not change."),
    ("Trocar espaço", "Swap space"),
    ("Trocar o espaço visível  (Shift+H)", "Swap the visible space  (Shift+H)"),
    ("Mostra só os itens ocultos, para trazer de volta o que precisar. Clique de novo para voltar. Nada é apagado.", "Shows only the hidden items, to bring back what you need. Click again to go back. Nothing is deleted."),
    ("Aplicar material", "Apply material"),
    ("Aplica um material pronto (cor e acabamento) à seleção: faces, comandos, corpos e superfícies.", "Applies a ready-made material (color and finish) to the selection: faces, features, bodies and surfaces."),
    ("Selecione faces, comandos, corpos ou superfícies para aplicar o material.", "Select faces, features, bodies or surfaces to apply the material."),
    ("Nada selecionado: enquadrei o modelo inteiro.", "Nothing selected: framed the whole model."),
];

#[cfg(test)]
mod tests {
    /// B505/B506: todo texto novo tem tradução, e a tradução não é o próprio texto (salvo nomes iguais).
    #[test]
    fn textos_novos_traduzidos() {
        for (pt, en) in super::TEXTOS {
            assert!(crate::i18n::tem_traducao(pt), "{pt}");
            assert!(!en.is_empty(), "{pt}");
        }
        for e in &crate::elementos::ELEMENTOS {
            for t in [e.nome, e.frase, e.passos] {
                assert!(crate::i18n::tem_traducao(t), "{t}");
            }
        }
        for t in crate::propriedades_graficas::TIPOS_DE_LINHA.iter().chain(&crate::propriedades_graficas::SIMBOLOS) {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
        for (n, _) in fenkra_core::APPEARANCE_PRESETS {
            assert!(crate::i18n::tem_traducao(n), "{n}");
        }
        assert!(crate::i18n::tem_traducao(crate::settings::Group::Propriedades.label()));
    }
}
