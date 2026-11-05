# ADR 0018 — Tolerâncias normativas em crate próprio, sem dependências

## Contexto

Cotas com classe de ajuste (H7/g6), tolerância geral na legenda, limites do AP242 e a biblioteca de
fixadores precisam dos mesmos números da ISO 286 e da ISO 2768. Sem um lugar único, cada um digitaria os seus.

## Decisão

`fenkra-tolerancias` guarda só dados e funções puras (µm de saída, `VERSAO_TABELAS` para saber com que
revisão uma cota foi calculada). Não depende de `fenkra-core`, para desenho, biblioteca e montagem usarem
sem ciclo. O eixo é a tabela de referência; o furo é derivado pela regra geral (espelho) e pela regra do Δ.
Valores só entram quando conferidos em duas fontes: os valores publicados e as fórmulas da norma (estas
ficam como teste, com a folga do arredondamento que a norma faz à mão).

## Consequências

Cobre até 500 mm. Combinação que a norma não define devolve `None`, em vez de aproximar. Mudar um número
exige subir `VERSAO_TABELAS`.
