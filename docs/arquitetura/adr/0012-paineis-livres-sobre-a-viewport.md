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

## Atualização (B358): painéis como janelas

- Cada painel ganhou cabeçalho próprio (`paineis::cabecalho`): título, minimizar/expandir e fechar. A barra de
  título do `egui::Window` fica desligada porque ela não aceita botões nossos nem dica traduzida.
- `Posicao` guarda também `minimizado`, `fechado`, `largura` e `altura`, todos com padrão no serde, então
  preferências antigas continuam válidas. `Saida::movido` passa a avisar qualquer mudança dessas, não só o
  fim de um arraste.
- Redimensionar vale nos quatro lados e cantos. O egui já oferece as oito alças, mas só aumenta a altura de uma
  janela cujo conteúdo preenche o espaço: o corpo é um `ScrollArea` com a altura que a pessoa escolheu
  (calculada pelo deslocamento do ponteiro, igual à conta do egui). Antes da primeira alteração a altura
  continua sendo a do conteúdo. A posição não é imposta enquanto a alça está apertada, senão os lados de cima
  e da esquerda brigavam com o `current_pos`.
- Ao arrastar perto de uma borda aparece a faixa da borda e, dentro dela, o retângulo exato em que o painel vai
  ficar (`previa_de` lê esse retângulo nos testes). Painéis encaixados na mesma borda e no mesmo trecho dela se
  empilham para dentro em vez de se cobrirem (`colocar_empilhado`), o que resolve a barra direita junto do
  painel de comando.
- Todo painel se registra a cada quadro. O menu Exibir > Painéis lista os vivos com uma caixa de marcação, de
  modo que fechar nunca perde o painel. Um diálogo que some e volta reaparece aberto; barras e grupos
  (`Opcoes::persiste`) continuam fechados até a pessoa reabrir.
- Mudar de borda vertical para horizontal esquece o tamanho guardado, e as barras se deitam em fila quando
  encaixadas em cima ou embaixo.

### Segunda tela (B366), ainda não feita

O egui/eframe 0.37 tem viewports nativos (`show_viewport_immediate`), compatíveis com o wgpu usado aqui, e o
conteúdo de cada painel já é uma função de `&mut Ui`, sem depender da janela principal. Ficou de fora desta
entrega porque exige: (1) saber onde o ponteiro está na tela inteira enquanto o botão está apertado fora da
janela principal, o que o winit só entrega com captura do mouse e muda a detecção de arraste de
`paineis::mostrar_com`; (2) rodar o conteúdo dentro de outro contexto de entrada, com o estado do app
emprestado ao viewport imediato; (3) um estado novo `Encaixe::Janela` com posição em coordenadas da tela. Nada
disso conflita com o formato atual: `Posicao` é só dado serializável, e acrescentar `Encaixe::Janela` e uma
posição de tela é compatível com as preferências existentes. O desenho do B358 não depende da janela
principal além de `area`, que passaria a ser a área do viewport onde o painel está.
