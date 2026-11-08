# ADR 0081: booleanas e malhas da interferência em threads, sobre cópias profundas (B394)

## Contexto

A análise de interferências do kart (84 formas, 121 pares que passam pelos filtros de caixa e de malha) levava
48,5 s com as peças já calculadas. O perfil, numa thread só, mostrou de onde vinha o tempo:

- a booleana Comum dos 121 pares: 28 s, sendo 2 s cada um dos três pares mais caros;
- a unificação de faces (`ShapeUpgrade_UnifySameDomain`) que a `boolean` faz no fim de toda operação: cerca de
  11 s, sem mudar o volume nem a malha do comum;
- a malha de cada forma para o filtro antes da booleana: 6 a 7 s. `posicionado` leva a peça ao mundo com
  `BRepBuilderAPI_Transform` em modo cópia, que não traz a malha: toda análise tesselava as 84 formas de novo.

A HLR exata do OpenCASCADE guarda estado global e erra em threads (ADR 0080). Era preciso saber se a booleana
tinha o mesmo problema antes de paralelizar.

## O que vale para a booleana

- A booleana não tem estado global conhecido: rodar os mesmos pares em 8 e 16 threads deu os mesmos volumes,
  par a par, que a sequencial, no teste do kernel e nos 95 achados do kart, em várias rodadas.
- O risco é outro: dados divididos. A mesma peça entra em vários pares (o aro com o pneu, o cubo e o eixo), e o
  comum de uma peça inteira dentro de outra devolve as próprias faces da entrada. Tesselar esse resultado grava a
  malha nessas faces, e duas threads gravariam na mesma face ao mesmo tempo. No modo padrão a booleana também
  pode aumentar a tolerância de arestas e vértices das entradas.

## Decisão

1. `Shape::comum_medida`: Comum sem unificação e em modo não destrutivo (`SetNonDestructive`), para medir.
2. `Shape::comuns(pares, threads, f)`: as booleanas em threads (0 = uma por núcleo), cada uma sobre cópias
   profundas das duas formas (`BRepBuilderAPI_Copy` com a geometria, sem a malha). `f` roda na própria thread com o
   resultado, que não divide nada com ninguém: ali se mede o volume e se tessela. `f` precisa ser `Sync`, e
   `Shape` não é: o compilador impede que `f` leve outra forma para dentro da thread.
3. `Shape::em_threads(formas, threads, f)`: o mesmo para uma forma por vez (as malhas do filtro).
4. As threads pegam o próximo item livre; a interferência manda primeiro os pares com mais triângulos, para os
   caros não ficarem para o fim.
5. Com uma thread só, nada é copiado e tudo roda na thread de quem chamou.

## Medição (kart, release, máquina ocupada por outros agentes)

| Etapa | Antes | Agora |
|---|---|---|
| Análise com as peças no cache | 48,5 s | 9,6 a 10,0 s (16 threads) |
| Primeira análise, com as peças por calcular | 77,7 s | 22,4 s |
| A mesma análise numa thread só, sem unificação | 48,5 s | 37 a 39 s (65 s numa rodada com a máquina cheia) |

A primeira análise caiu também porque a interferência passou a pedir só a forma das peças (`PartCache::so_forma`,
o modo do B393): a malha de vista que o `get` fazia de cada peça não servia à análise, que tessela a forma já
posicionada. Uma peça que o `get` já calculou com a forma é usada como está.

## Consequências

- Os achados são os mesmos, na mesma ordem: o teste ignorado `interferencia_kart` compara a análise em threads
  com a de uma thread, par a par.
- A malha do comum vem sem a unificação: mais faces pequenas, a mesma superfície. Só a vista usa essa malha.
- Cada thread copia as duas formas do par. No kart isso é pouco perto da booleana; numa peça muito grande o pico de
  memória cresce com o número de threads.
- O cálculo frio das peças (a primeira análise na interface, porque o cache da interface não é o da thread de
  composição, que não guarda formas) continua numa thread só: são cerca de 13 s dos 22 s no kart. Paralelizá-lo
  exige saber se o recálculo inteiro dos comandos é seguro em threads, o que esta decisão não cobre.
- A análise ainda roda na thread da interface, que fica parada enquanto isso.
