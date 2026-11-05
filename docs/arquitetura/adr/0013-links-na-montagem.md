# ADR 0013: Estado derivado e política de links na montagem

- **Status:** aceita
- **Data:** 2026-11-05

## Contexto
A ADR 0006 pede estado e política por link. O link mora na peça (`ExternalRef`), mas uma peça pode
entrar em várias montagens, e a transformação de um link feito em contexto é a posição relativa de
dois componentes, que só existe numa montagem.

## Decisão
1. **O estado que a pessoa vê é derivado.** Em contexto, fora de contexto e pendurado saem do
   `LinkState` gravado somado ao lugar onde o link está. Nada disso vai para a peça, porque a mesma
   peça pode estar em contexto numa montagem e fora em outra.
2. **Quebrado é gravado sem caminho de origem.** `refresh_link` reativa um link `Quebrado` quando a
   publicação reaparece, o que é certo para o pendurado. O corte pedido pela pessoa não pode voltar
   sozinho, e o caminho vazio marca isso sem criar um estado novo no formato da peça.
3. **A política mora na montagem** (`Assembly::politicas_link`), porque diz como aquele conjunto se
   atualiza. Link sem registro é automático.
4. **A transformação é recalculada a cada atualização** pelas posições atuais. O valor gravado na
   peça é só o último conhecido, usado quando a origem não está na montagem aberta.
5. **A atualização roda depois de resolver**, e não durante o arraste: ela grava arquivos e recalcula
   peças. Sem mudança de posição nem de arquivo, nenhum link é lido.
6. **O grafo é verificado por inteiro** ao criar link e ao inserir subconjunto, sem limite de
   profundidade, atravessando peças e montagens.

## Consequências
- Montagens e peças antigas abrem sem migração: todos os links ficam automáticos.
- Links dentro de um subconjunto só atualizam com o subconjunto aberto.
- Com várias instâncias do mesmo arquivo, vale a origem de mesmo nome e a primeira instância da peça
  que recebe. Links por instância ficam para quando a peça tiver cópias independentes.
- Desfazer na montagem não desfaz o arquivo da peça na hora; a cópia volta na resolução seguinte.
