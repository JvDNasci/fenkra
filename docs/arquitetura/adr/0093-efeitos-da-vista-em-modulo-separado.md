# ADR 0093: efeitos da vista em módulo de shader separado, com queda para o nível Rápido

Data: 2026-11-14. Item: B139.

## Contexto

Sombra suave e oclusão de ambiente precisam ler texturas de profundidade e, a sombra, um amostrador
de comparação. O programa precisa abrir em OpenGL ES 3, em adaptador de software e em placa antiga, e
nenhum efeito pode virar requisito de abertura (docs/benchmark/detalhe/kernel.md, modo leve).

## Decisão

- O shader comum ganha duas funções, `sombra_em` e `oclusao_em`, que no **módulo base** devolvem 1.
  Um **segundo módulo**, montado da mesma fonte com essas funções reais e um grupo de recursos 1
  (mapa de sombra, amostrador de comparação, profundidade da câmera em meia resolução), só é criado no
  primeiro quadro que pede sombra ou oclusão.
- Os pipelines comuns usam só o grupo 0. Só as faces opacas dos modos sombreados trocam para o
  pipeline do módulo dos efeitos.
- O módulo dos efeitos, as passadas de profundidade e as texturas são criados dentro de escopos de
  erro de validação e interno. Erro ou falta de recurso (`COMPARISON_SAMPLERS`, textura menor que
  1024) derruba a capacidade para "sem profundidade": o nível cai para Rápido, o motivo vai para o
  registro de partida e a interface mostra um aviso uma vez.
- O PBR não usa textura nem passada, então fica no módulo base e vale em qualquer placa.
- Os efeitos nascem desligados no modo leve (fazem parte da combinação desfeita ao sair dele) e no
  adaptador de software (até a pessoa escolher). O nível Rápido não tem sombra nem oclusão; "Simplificar
  ao girar" usa o Rápido enquanto a câmera se move.

## Consequências

- Placa sem o recurso ou com driver que falha no shader dos efeitos continua com a vista de antes.
- A oclusão usa pontos numa espiral fixa, sem ruído e sem passada de borrão: em cantos grandes pode
  aparecer degrau suave em vez de granulado. Uma passada de borrão fica para quando houver render final.
- A cena de apresentação (B182) reaproveita a luz principal e o uniforme dos efeitos.
