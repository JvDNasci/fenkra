# ADR 0095 — Compasso: o cubo de vistas que se prende na peça

Estado: aceito (14/11/2026). Item: B438. Complementa o ADR 0094 (o cubo pequeno no canto é o único
elemento sobre a área 3D).

## Contexto

O manipulador da montagem (B261) nasce no centro da caixa da peça, com eixos do mundo, da peça ou de
uma face escolhida por um clique extra numa barrinha flutuante. Para mover ao longo de uma aresta ou girar
em volta de um furo, a pessoa precisava escolher a referência, depois o pivô, e só então puxar a seta.
O usuário pediu o fluxo dos softwares de modelagem CAD avançados que ele usa: o próprio cubo de vistas
vira um compasso, que se arrasta até a peça e gruda numa face, aresta ou eixo.

## Decisão

- O cubo do canto continua sendo o cubo de vistas (clique numa face, arrasto dentro dele gira a vista).
  Arrastar para fora dele, com o botão apertado, leva o compasso; a vista volta ao ângulo do aperto.
- Soltar sobre a peça usa a mesma leitura de geometria das restrições de montagem: face plana dá Z na
  normal; aresta reta dá Z ao longo dela; face cilíndrica ou aresta circular dá Z no eixo. A origem é o
  ponto solto levado para o plano ou para a reta. X é o eixo do mundo menos alinhado com Z.
- Preso num componente, o quadro fica guardado no sistema dele e acompanha a peça. Mover outra peça
  passa o compasso para ela, para seguir a mão.
- Preso, o compasso é a referência e o pivô do manipulador da montagem: as setas, os anéis e os rótulos
  digitáveis nascem na origem dele, nos eixos dele. O eixo que uma restrição impõe continua mandando no
  giro. Sem compasso preso, o manipulador antigo fica como estava, para não perder função.
- O painel do compasso ocupa a coluna da esquerda (ADR 0094) quando nenhum comando de verdade está
  aberto: coordenada incremental ou absoluta, referência local (eixos do compasso; na absoluta, o lugar
  onde foi preso) ou global, posição e ângulo por eixo e botões de incremento.
- Volta ao canto: soltar no lugar do canto, clicar nele, o botão do painel, fechar o painel ou o atalho
  "Devolver o compasso ao canto" (Alt+Home).

## Conta

O painel monta uma transformação rígida T. Incremental: translação nos eixos da referência e giros (X,
depois Y, depois Z, eixos fixos) pela origem do compasso já deslocada. Absoluta: T = G ∘ Q⁻¹, onde Q é o
quadro atual do compasso e G o quadro digitado. Cada peça escolhida vai para T ∘ P e entra pelo mesmo
caminho do rótulo digitado do manipulador (respeita restrições e colisão, um desfazer por peça).

## Consequências

- Na peça aberta (sem montagem) o compasso prende e orienta a vista, mas não move elementos: mover e
  girar valem para componentes da montagem.
- Os ângulos absolutos usam giros em eixos fixos X, Y, Z; fora de ±90° em Y a leitura pode trocar a
  combinação de ângulos equivalente, o que não muda a posição final.
