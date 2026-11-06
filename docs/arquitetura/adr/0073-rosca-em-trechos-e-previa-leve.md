# ADR 0073: rosca modelada em trechos de duas voltas, prévia só em linhas e cosmética em textura (B351)

## Contexto

Na alpha.30, fazer rosca num furo não funcionava: no furo escareado a rosca modelada não removia nada,
no furo cego o sulco passava do fundo, e a prévia refazia a hélice inteira a cada alteração (cerca de 2 s
num furo passante de 30 mm).

A causa foi medida. A ferramenta era uma varredura de um perfil em V ao longo de uma hélice de todas as
voltas: uma face só, de B-spline com dezenas de vãos. A interseção dela com o cilindro do furo falha, e a
booleana do OpenCASCADE devolve o corpo intacto, ou um sólido inválido, sem acusar erro. Numa bancada de
prova (bloco de 30 mm, furo de 6,8 mm, rosca M8x1,25) a rosca cortou ou não conforme o ponto de partida e o
comprimento: com a varredura inteira, 8 de 9 combinações falharam (7 sem cortar nada e 1 com erro).
Com trechos de uma ou duas voltas as 9 saíram corretas e válidas; com três ou seis voltas vários
resultados saíram inválidos.

## Decisão

1. **A ferramenta é feita em trechos de duas voltas** (`rosca_furo::ferramenta`). Cada trecho começa onde o
   anterior termina, na mesma fase da hélice, porque o comprimento dele é múltiplo do passo.
2. **O sulco cabe na parte cilíndrica.** Começa uma meia largura de filete abaixo do escareado ou rebaixo,
   termina uma meia largura antes do fundo do furo cego e, no passante, termina fora da peça (só corta ar).
3. **A booleana é conferida.** Se o volume do corpo não diminuiu, o comando avisa em vez de calar. Aviso
   também para o diâmetro diferente da broca de rosca e para a rosca que não cabe.
4. **Edição é leve** (`Cache::leve`, o comando aberto): a rosca modelada só registra a posição
   (`RoscaViz`); nada de hélice nem booleana. O sulco é construído ao concluir, e a assinatura do comando
   muda quando o modo muda, então o resultado leve nunca vira o final. A prévia caiu de cerca de 2 s para
   cerca de 10 ms.
5. **A prévia da rosca é só linhas** (hélice com o passo e o comprimento certos, na cor do tema), e a
   **rosca sem sulco real é textura**: a parede cilíndrica é achada pela malha e repetida na camada de
   fantasma, com o eixo, a origem e o passo nos atributos do vértice. O `fs_ghost` pinta o filete por pixel.
   Não há malha de rosca nem buffer novo na placa de vídeo.

## Consequências

- Rosca modelada continua paramétrica: o sulco é recalculado a cada recompute final a partir de
  profundidade, folga, diâmetro e escareado.
- O custo do sulco final não mudou de ordem (mais ferramentas pequenas, cada uma mais barata).
- A textura procura triângulos da malha de trabalho (O(triângulos) por rosca); com centenas de roscas na
  mesma peça isso vira o próximo gargalo, e o caminho é guardar os triângulos por geração do resultado.
