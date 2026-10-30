# Requisitos — Projeto relacional, geometria mestre e links

Ver [ADR 0006](../arquitetura/adr/0006-links-e-projeto-relacional.md) para a arquitetura.

## Geometria mestre

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-LNK-001 | Documento de geometria mestre (esqueleto) com sketches, curvas, superfícies, planos, pontos e parâmetros. | M | 1 | rascunho |
| RF-LNK-002 | **Publicação explícita**: o mestre define quais elementos e parâmetros ficam disponíveis para outras peças (a "interface pública"). | M | 1 | rascunho |
| RF-LNK-003 | Elementos publicados têm nome estável; mudanças internas no mestre não quebram links enquanto a publicação existir. | M | 1 | rascunho |
| RF-LNK-004 | Hierarquia de mestres: um mestre pode derivar de outro mestre. | S | 2 | rascunho |

## Links entre documentos

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-LNK-010 | Tipos de link: **geometria** (cópia associativa de elemento publicado), **parâmetro**, **posição** (sistema de coordenadas) e **corpo** (peça derivada de outra peça). | M | 1 | rascunho |
| RF-LNK-011 | Estados do link: **ativo** (sincroniza), **congelado** (mantém o último valor), **quebrado** (alvo não encontrado). | M | 1 | rascunho |
| RF-LNK-012 | Política de atualização por link: automática, ao abrir o documento, ou manual. | M | 1 | rascunho |
| RF-LNK-013 | Link para uma revisão específica do alvo ou para "a revisão de trabalho". | M | 2 | rascunho |
| RF-LNK-014 | Links entre peças criados no contexto da montagem (ex.: furo de uma peça a partir da aresta de outra). | M | 2 | rascunho |
| RF-LNK-015 | O grafo de links entre documentos é acíclico; criar um ciclo é bloqueado com explicação. | M | 1 | rascunho |
| RF-LNK-016 | Ao abrir um documento com links desatualizados, mostrar o que mudou e permitir atualizar tudo, nada ou selecionar. | M | 1 | rascunho |
| RF-LNK-017 | Link quebrado mantém a última geometria conhecida (em cache) e não invalida o documento. | M | 1 | rascunho |

## Gerenciador de links

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-LNK-020 | Painel com todos os links de entrada e saída do documento: origem, destino, tipo, estado, revisão. | M | 1 | rascunho |
| RF-LNK-021 | Ações: atualizar, congelar, descongelar, redirecionar para outro alvo, substituir documento, remover. | M | 1 | rascunho |
| RF-LNK-022 | "Onde é usado": quais documentos dependem deste elemento publicado, antes de alterar ou apagar. | M | 2 | rascunho |
| RF-LNK-023 | Grafo visual de dependências do projeto. | S | 3 | rascunho |

## Variáveis geométricas referenciáveis

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-LNK-030 | Elementos que podem ser publicados e ligados: **superfícies**, faces, **planos**, eixos, **pontos**, **curvas** (2D e 3D), arestas, sistemas de coordenadas, corpos e parâmetros. | M | 1 | rascunho |
| RF-LNK-031 | Todo elemento publicado ou recebido por link é uma **variável referenciável** com nome (ex.: `Mestre.PlanoBase`, `Mestre.CurvaGuia`), listada na pasta Referências. | M | 1 | rascunho |
| RF-LNK-032 | Qualquer feature aceita uma variável como entrada, escolhida pelo nome ou clicando no elemento. | M | 1 | rascunho |
| RF-LNK-033 | Variáveis são **tipadas** (ponto, curva, plano, superfície, eixo, sistema de coordenadas, corpo, valor); a feature só aceita os tipos compatíveis. | M | 1 | rascunho |
| RF-LNK-034 | Expressões podem consultar medidas das variáveis: comprimento de curva, área de superfície, coordenadas de ponto, distância e ângulo entre elementos (ex.: `comprimento(Mestre.CurvaGuia)`). | M | 2 | rascunho |
| RF-LNK-035 | **Redirecionar uma variável** para outro elemento atualiza todos os comandos que a usam, sem reeditar cada um (aplica RF-NUC-021). | M | 1 | rascunho |
| RF-LNK-036 | O usuário pode nomear elementos locais como variáveis (ex.: uma face usada por várias features), mesmo sem publicá-los. | M | 1 | rascunho |
| RF-LNK-037 | Variável cujo alvo sumiu mantém o último valor (cache), é marcada como quebrada e pode ser redirecionada; os comandos que a usam não são apagados. | M | 1 | rascunho |
