# ADR 0104 — Barras soltas e encaixáveis, painéis que mudam de tamanho e árvore transparente

Estado: aceito (18/11/2026). Revisa o ADR 0094 nos pontos das barras de comando, do tamanho dos
painéis de borda e da árvore. Itens: B502 e B503.

## Contexto

O ADR 0094 (B433, B434) fixou as barras de comando: recursos só à direita e navegação só embaixo, com
painéis de tamanho fixo (`resizable(false)`, `exact_size`), e pôs a árvore numa coluna opaca que reserva
espaço. Com isso sumiu o que existia desde 24/10 (commit 686cc68): alça em todo grupo, arraste com
fantasma até a doca, barra flutuante onde fosse solta e barras laterais redimensionáveis.

No teste da alpha.37 (18/11) o usuário apontou essa perda como a maior da versão. Ele usa softwares de
modelagem CAD avançados de superfícies, em que cada barra se arrasta pela alça, flutua, encaixa em
qualquer borda, fecha no "x" e volta pela lista de barras, com o layout guardado por bancada. A
referência pública está em `docs/benchmark/detalhe/interface-referencia-2026-11.md` (regras R1.x, R2.x e
R7.x).

## Decisão

1. **Toda barra tem lugar próprio, por bancada** (`barras_layout.rs`). Uma barra é um grupo de comandos
   do sistema ou uma caixa montada pela pessoa. O lugar é uma das quatro bordas (topo, esquerda,
   direita, embaixo) ou um ponto onde ela flutua, e fica em `Settings::barras`, uma lista por bancada
   que só guarda o que difere do padrão. O padrão continua o do ADR 0094: recursos à direita e navegação
   embaixo. A doca antiga (`docks`, igual para todas as bancadas) é só lida pela migração.
2. **Arrastar pela alça** (`toolbar::alca`, `app/barras_soltas.rs`). Toda barra encaixada tem alça;
   a flutuante tem a alça no cabeçalho. O egui só começa o arraste depois de alguns pixels, então um
   clique não solta a barra. Enquanto arrasta, as faixas das quatro bordas aparecem fracas, a da vez
   acende como prévia de onde a barra vai encaixar e um fantasma da barra segue o cursor e desliza até
   a faixa. Soltar numa borda encaixa; soltar longe delas deixa a barra flutuando ali.
3. **Fechar não apaga.** A barra flutuante tem "x"; qualquer barra fecha pelo clique direito. A barra
   fechada guarda a borda e o ponto, continua listada no menu Janela (e em Exibir) > Barras de
   ferramentas, com caixa de marcação, e reabre onde estava.
4. **Restaurar**: "Restaurar o layout desta bancada" e "Restaurar o layout de todas as bancadas". As
   caixas de comandos da pessoa nunca são apagadas por restaurar; só a posição delas volta. O layout
   restaurado é igual, byte a byte, ao padrão serializado.
5. **Caixa de comandos própria** (`caixa_comandos.rs`). "Nova barra de comandos" cria uma caixa; em
   Personalizar comandos, os ícones da lista se arrastam para dentro dela, mudam de ordem dentro dela e
   saem quando arrastados para fora. A caixa guarda só a chave de cada comando, então um comando de outra
   bancada entra sem duplicar código. Chave que não existe mais aparece como indisponível, sem derrubar
   a caixa.
6. **Todos os painéis de borda mudam de tamanho** (`barras_layout::painel_na_borda`). As barras do topo,
   da esquerda, da direita e de baixo mudam de tamanho pela beira; a medida é gravada por bancada quando
   o botão solta (nunca a cada quadro do arraste). Sem medida gravada, o painel usa o tamanho automático
   (a largura que cabe os ícones). A coluna da esquerda e os painéis encaixados do ADR 0094 já mudavam de
   tamanho e continuam assim.
7. **Árvore transparente sobre a vista** (`app/coluna.rs`, `arvore_sobre_modelo`). Sem comando aberto,
   não há coluna: a árvore fica no canto da área 3D, sem placa, com o contorno da regra de contraste do
   tema (`tree_ui::Contorno`). Move pela alça do cabeçalho, muda de largura pela beira da direita e amplia
   com Ctrl + roda, sem mexer na câmera; posição e largura ficam nas preferências. Ela é desenhada depois
   da vista e na mesma camada: só as linhas capturam o mouse, o vazio continua sendo a peça.
8. **O painel do comando continua na coluna**, que reserva espaço quando ele está aberto (ADR 0094,
   item 1). As propriedades de um item escolhido também ocupam a coluna, e a árvore fica ao lado, sobre a
   vista.

## Regra permanente

Barras flutuantes e encaixáveis, painéis que mudam de tamanho e a árvore transparente fazem parte do
produto. Nenhuma mudança de layout pode remover nada disso sem o ok explícito do usuário. Quem precisar
mudar o layout escreve outra ADR e pergunta antes.

## Migração das preferências

`LAYOUT_VERSAO` passa a 3. A barra que estava oculta na doca antiga fica fechada em todas as bancadas; o
resto nasce no lugar padrão. As chaves novas (`barras`, `arvore_desloc`, `arvore_largura`) têm padrão,
então arquivo antigo é lido sem erro.

## Consequências

- O que o ADR 0094 dizia sobre "as docas antigas valem como 'na barra'" deixa de valer: o lugar escolhido
  é respeitado em qualquer borda.
- "Árvore transparente" volta ao checklist de interface (B319); "painel do comando na coluna, aberto só
  durante o comando" continua.
- Barra flutuante fora da tela (troca de monitor ou de resolução) volta para dentro ao ser desenhada.
- Renomear uma caixa própria esquece a posição dela (o lugar é guardado pelo nome); ela volta à direita.
- Com a árvore sobre a vista, a peça pode ficar atrás do texto da árvore. O contorno mantém o texto
  legível, e a árvore se move ou estreita para liberar a peça.
