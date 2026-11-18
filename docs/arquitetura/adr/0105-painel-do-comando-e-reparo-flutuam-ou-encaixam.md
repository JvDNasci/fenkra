# ADR 0105 — Painel do comando e assistente de reparo flutuam ou encaixam

Estado: aceito (18/11/2026). Revisa o item 8 do ADR 0104 e o ADR 0094 no ponto "nenhum painel fica
livre". Item: B507.

## Contexto

Pelo ADR 0094 todo painel ficava encaixado (direita, base ou coluna) e o único jeito de soltar um era
a janela nativa do sistema (B366). O ADR 0104 devolveu as barras soltas, mas manteve o painel do
comando preso à coluna. No teste da alpha.37 (18/11) o usuário pediu que o painel do comando e o
assistente de reparo flutuem ou encaixem, à escolha, como nos softwares de modelagem CAD avançados, e
que o reparo não fique preso na tela depois de corrigido. Referência pública:
`docs/benchmark/detalhe/interface-referencia-2026-11.md`, tópico 7 (R7.1, R7.2, R7.4).

## Decisão

1. **Flutuar é opção do painel** (`paineis::Opcoes::flutua`). Num painel com a opção, `Encaixe::Livre`
   vale de verdade: ele fica sobre a vista, onde foi solto, e não reserva espaço. Os demais painéis
   seguem o ADR 0094 (livre vira direita ou base).
2. **Escolha pelo gesto ou pelo botão.** Arrastar até uma borda encaixa; soltar longe das bordas
   deixa flutuando. O cabeçalho ganha um botão que alterna entre flutuar e encaixar.
3. **Posição lembrada e validada.** A posição flutuante fica nas preferências, por bancada, e é
   conferida contra a área a cada quadro (`paineis::posicao_livre`): guardada noutro monitor ou numa
   tela maior, o painel abre inteiro à vista. Encaixar não esquece a posição flutuante.
4. **Painel do comando** (`app/coluna.rs`): na coluna por padrão; o botão ao lado do "x" e o clique
   direito no título o soltam. Flutuando, a árvore volta a ficar transparente sobre a vista, para
   escolher entradas. Encaixar, pelo botão ou arrastando até qualquer borda, devolve à coluna
   (`Opcoes::encaixe_ao_fixar`). A janela nativa (B366) continua como terceira opção. A primeira vez
   que flutua, nasce perto da borda direita, abaixo do cubo, longe da seleção.
5. **Assistente de reparo** (`App::wizard_window`): flutua ou encaixa como qualquer painel com a
   opção. Só existe com erro: abre sozinho quando o recálculo traz erro novo e fecha sozinho quando
   não sobra nenhum item quebrado (`reparo_ui::assistente_aberto`), em vez de mostrar "nada quebrado"
   até alguém fechar.

## Consequências

- Painel flutuante cobre parte da peça; é escolha da pessoa, e o botão de encaixar está sempre no
  cabeçalho.
- A posição guardada de "comando" serve ao modo flutuante e ao nativo; cada modo confere a posição
  contra a sua área antes de usar.
- Guardar a posição por comando (R7.2) fica para depois: hoje é uma por bancada, para todos os
  comandos.
