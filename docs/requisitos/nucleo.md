# Requisitos — Núcleo (documento, árvore, timeline, formatos)

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-NUC-001 | Tipos de documento: peça, montagem, desenho e geometria mestre (esqueleto). | M | 1 | rascunho |
| RF-NUC-002 | Árvore de features e linha de tempo de comandos, detalhadas em [arvore.md](arvore.md). | M | 1 | rascunho |
| RF-NUC-003 | Editar qualquer feature da árvore e recalcular as dependentes. | M | 1 | rascunho |
| RF-NUC-004 | Reordenar features quando as dependências permitirem, avisando quando não permitirem. | S | 2 | rascunho |
| RF-NUC-005 | Suprimir / reativar features. | M | 1 | rascunho |
| RF-NUC-006 | Multi-corpo: uma peça pode conter vários sólidos e superfícies, organizados em coletores de corpos (RF-ARV-040). | M | 1 | rascunho |
| RF-NUC-007 | Geometria de referência: planos, eixos, pontos e sistemas de coordenadas (pasta Referências, RF-ARV-010). | M | 1 | rascunho |
| RF-NUC-008 | Diagnóstico de erro claro na feature que falhou, sem invalidar o resto da árvore. | M | 1 | rascunho |
| RF-NUC-009 | Toda feature declara suas entradas de forma explícita, e qualquer entrada pode vir de um link externo (RF-LNK). | M | 1 | rascunho |
| RF-NUC-010 | Importar/exportar STEP (AP203/AP214/AP242), IGES e BREP. | M | 1 | rascunho |
| RF-NUC-011 | Importar/exportar STL, OBJ, 3MF; importar PLY. | M | 1 | rascunho |
| RF-NUC-012 | Importar/exportar DXF e SVG em sketches e desenhos. | M | 2 | rascunho |
| RF-NUC-013 | Propriedades de massa: volume, área, massa, centro de massa e inércia. | M | 1 | rascunho |
| RF-NUC-014 | Medição entre entidades (distância, ângulo, raio), inclusive entre peças de montagem. | M | 1 | rascunho |
| RF-NUC-015 | Seção dinâmica (plano de corte na viewport). | S | 2 | rascunho |
| RF-NUC-016 | Avaliar suporte a DWG (leitura/escrita) por processo externo, respeitando licenças. | C | 4 | rascunho |

## Edição não destrutiva

Princípio: **modificar é reeditar, nunca apagar e recriar.** Cada comando, sketch, entidade e link tem
uma identidade interna estável; as referências dos comandos seguintes apontam para essa identidade.

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-NUC-020 | Modificar uma feature, sketch ou link **reedita o objeto existente** mantendo seu ID interno; nunca apaga e recria. | M | 1 | rascunho |
| RF-NUC-021 | **Substituir referência**: trocar a entrada de um comando (sketch, face, curva, superfície, plano, link) por outra; os comandos dependentes passam a usar a nova entrada sem precisar ser refeitos. | M | 1 | rascunho |
| RF-NUC-022 | Ao editar um sketch, entidades existentes mantêm seus IDs; entidades novas ganham IDs novos; substituir uma entidade por outra transfere as referências. | M | 1 | rascunho |
| RF-NUC-023 | Ao apagar algo que tem dependentes: listar os dependentes e oferecer **substituir referência**, **apagar junto** ou **cancelar**. Nunca quebrar silenciosamente. | M | 1 | rascunho |
| RF-NUC-024 | Comando com referência perdida **não é apagado**: fica com aviso, mantém o último resultado em cache e pode ser reapontado por reedição. | M | 1 | rascunho |
| RF-NUC-025 | Reparar referências em lote: mostrar todos os comandos com referência perdida e permitir reapontar um a um ou por correspondência sugerida. | S | 2 | rascunho |
