# Requisitos — Montagem e Mecanismos

## Estrutura

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-MON-001 | Documento de montagem com componentes (peças e submontagens) referenciados por arquivo e revisão. | M | 2 | rascunho |
| RF-MON-002 | Componente **fixo** ou **flutuante**; indicação dos graus de liberdade restantes de cada componente. | M | 2 | rascunho |
| RF-MON-003 | Inserir componentes da biblioteca (fixadores, perfis) escolhendo tamanho na tabela. | M | 2 | rascunho |
| RF-MON-004 | Padrões de componentes (linear, circular, seguindo padrão de furos de uma peça). | M | 2 | rascunho |
| RF-MON-005 | Modos visualizar / contexto / editar por componente (RF-UI-010 a 013). | M | 2 | rascunho |
| RF-MON-006 | Parâmetros e propriedades de montagem (RF-PAR); parâmetros da montagem podem dirigir peças via link. | M | 2 | rascunho |

## Restrições e mecanismos

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-MON-010 | Restrições geométricas: coincidente, concêntrico, distância, ângulo, paralelo, perpendicular, tangente, simétrico, travado. | M | 2 | rascunho |
| RF-MON-011 | Juntas: rígida, rotacional, prismática (deslizante), cilíndrica, planar, esférica, parafuso (rotação + translação acopladas). | M | 2 | rascunho |
| RF-MON-012 | Limites de movimento (mínimo/máximo) em juntas e restrições de distância/ângulo. | M | 2 | rascunho |
| RF-MON-013 | Acoplamentos: engrenagem (relação de rotação), pinhão-cremalheira, came-seguidor simples. | S | 3 | rascunho |
| RF-MON-014 | **Drives**: dirigir uma junta por valor, intervalo ou função do tempo; animar e gravar vídeo. | M | 2 | rascunho |
| RF-MON-015 | Arrastar componentes respeitando restrições (movimento cinemático interativo). | M | 2 | rascunho |
| RF-MON-016 | Detecção de colisão durante o movimento (parar ou destacar). | S | 3 | rascunho |
| RF-MON-017 | Diagnóstico de restrições conflitantes ou redundantes, indicando quais. | M | 2 | rascunho |

## Recursos inteligentes

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-MON-020 | **Rosca entre peças**: ao montar parafuso em furo roscado ligado, a especificação é compartilhada e a junta tipo parafuso pode ser criada automaticamente. | M | 2 | rascunho |
| RF-MON-021 | Inserir fixadores automaticamente em padrões de furos inteligentes (escolhe comprimento pela espessura empilhada). | S | 3 | rascunho |
| RF-MON-022 | Criar peça nova no contexto da montagem, com links para as vizinhas (RF-LNK-014). | M | 2 | rascunho |
| RF-MON-023 | Detecção de interferência estática. | M | 2 | rascunho |
| RF-MON-024 | Vista explodida com passos. | S | 3 | rascunho |
| RF-MON-025 | Substituir componente mantendo restrições quando a topologia equivalente existir. | S | 3 | rascunho |
