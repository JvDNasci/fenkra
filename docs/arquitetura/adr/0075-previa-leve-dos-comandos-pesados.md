# ADR 0075: prévia leve dos comandos pesados, só linhas até concluir (B356)

## Contexto

A ADR 0073 (B351) mostrou que a prévia da rosca caiu de cerca de 2 s para 10 ms quando, com o comando
aberto (`Cache::leve`), o cálculo deixou de construir a geometria e devolveu só a posição. O usuário pediu
o mesmo em todos os comandos: o programa responde na hora e a geometria final só nasce ao confirmar.

O custo foi medido (release, cache aquecido, um campo mudando). Cada operação do OpenCASCADE refaz faces e
booleanas sobre o corpo inteiro:

| comando | caso | completo |
|---|---|---|
| padrão linear | 1000 instâncias | 5800 ms |
| casca | placa com 240 furos | 6400 ms |
| filete / chanfro | mesma placa, todas as arestas | 1900 ms |
| padrão linear | 90 instâncias | 190 ms |
| padrão circular | 40 instâncias | 75 ms |
| varredura em linha, loft de 6 seções | | 33 ms (não precisa) |

## Decisão

1. **Um só mecanismo, o da B351.** `Cache::leve` (o comando aberto) vale para qualquer comando que
   `previa_leve::cobre`. Em `run_feature`, antes do cálculo normal, `Ctx::previa_leve` roda a versão leve:
   o corpo passa intacto e o comando devolve uma `PreviaViz` (`RecomputeResult::previas`), sem tocar no
   OpenCASCADE.
2. **A prévia é geometria barata no lugar e no tamanho finais:**
   - filete e chanfro: as arestas resolvidas pelas mesmas chaves do cálculo real e o valor; a interface
     desenha em cada face plana vizinha a linha de tangência (chanfro: a distância; filete:
     `r / tan(a / 2)`, com `a` o ângulo entre as faces);
   - casca: a caixa da cavidade (caixa do corpo recuada pela espessura, aberta onde a face escolhida é
     uma lateral da caixa);
   - padrão e espelho: a caixa da ferramenta de cada comando de origem, levada por cada transformação
     (`previa_leve::leva`, a mesma que o kernel aplicaria). `Ctx::instancias` é compartilhada com o
     cálculo real, então a posição não pode divergir.
3. **O resultado leve nunca vira o final.** A assinatura do comando ganha a marca "leve" (como na rosca), e
   a assinatura do corpo vira uma função só do corpo de entrada: a malha em cache vale enquanto a pessoa
   edita (nada é re-tesselado), e os comandos seguintes enxergam um corpo distinto do final. Ao concluir,
   o app zera `leve` e o cálculo real roda com status, volume e caixa envolvente como antes.
4. **Desenho na camada de linhas** (`fenkra-app/src/previa_leve_ui.rs`), na cor do tema. Sem malha nova nem
   buffer novo na placa de vídeo.

## Consequências

- Em edição, filete, chanfro, casca e padrão não mostram o material removido ou criado, nem erros que só o
  cálculo real acusa (raio grande demais, por exemplo); o erro aparece ao concluir. O erro de espessura da
  casca maior que o corpo continua imediato.
- O filete só tem fórmula de tangência em faces planas; em face curva a linha usa o valor do raio.
- A casca de corpo não prismático tem a cavidade aproximada pela caixa envolvente.
- Ficam sem prévia leve (continuam completos): filete em modos especiais (variável, face a face, completo,
  assimétrico), varredura, loft, chapa e estrutural. Varredura e loft medidos custam dezenas de ms; chapa e
  estrutural não foram medidos nesta entrega.
