# ADR 0012 — Painéis livres sobre a viewport, com código próprio

## Contexto

O usuário acha a tela cansativa e não quer painel fixo cobrindo o fundo (B312, B317, B318). A viewport
passa a ocupar a janela toda sob a barra superior. A árvore fica fixa e transparente no canto, e os demais
painéis flutuam translúcidos por cima, com encaixe nas bordas e posição salva por bancada.

## Decisão

Painéis e árvore usam código próprio (`paineis.rs` e `tree_ui.rs`), sem dependência nova.

As bibliotecas abertas de docking para egui dividem a janela em áreas com abas, que empurram umas às outras
e são donas do espaço central. Aqui o encaixe só alinha o painel à borda e nunca reserva espaço, então a
biblioteca resolveria o que não precisamos e atrapalharia o que precisamos: a viewport inteira atrás de
painéis translúcidos. Cada painel é um `egui::Window`, que já traz arrastar, redimensionar e ordem entre
janelas; o código próprio cuida só de posição, guia de encaixe e gravação.

A árvore é desenhada na mesma camada da viewport, depois dela, e não numa `Area`. Uma `Area` bloquearia
o mouse no retângulo inteiro. Na mesma camada, só as linhas capturam o mouse, e o vazio continua sendo o
modelo. O zoom da árvore escala fontes e espaçamentos da `Ui`, em vez da camada, para o texto seguir nítido.

## Consequências

- Encaixado, o painel continua por cima do modelo: nada redimensiona a viewport.
- O egui não marca a janela como arrastada quando ela é movida pelo título. O arraste é detectado pela
  janela sair da posição dada com o botão apertado sobre ela (`paineis::mostrar`).
- Painel em outro monitor (janela nativa separada) não está coberto por esta decisão e fica para depois.
- Janelas de diálogo (configurações, lista de peças, assistentes) continuam janelas comuns do egui.
