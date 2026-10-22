# Requisitos — Gestão de Projeto

Ver [ADR 0009](../adr/0009-projeto-e-revisoes.md). Escopo: uso individual ou equipe pequena com pasta
compartilhada, sem servidor.

## Projeto e estrutura de produto

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-GPR-001 | **Projeto** = pasta com índice próprio, que contém todos os documentos (peças, montagens, mestres, desenhos). | M | 1 | rascunho |
| RF-GPR-002 | **Estrutura de produto** (árvore) gerada das montagens, com PN, descrição, quantidade, tipo e revisão de cada item. | M | 2 | rascunho |
| RF-GPR-003 | Itens da estrutura sem modelo 3D (ex.: tinta, etiqueta, item comprado só com PN). | S | 2 | rascunho |
| RF-GPR-004 | Busca por PN, descrição, propriedade e tipo. | M | 2 | rascunho |
| RF-GPR-005 | Numeração automática de PN configurável (prefixo, sequência, por tipo). | S | 2 | rascunho |
| RF-GPR-006 | Exportar estrutura de produto para CSV/XLSX. | M | 2 | rascunho |

## Versões e revisões

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-GPR-010 | **Versões**: cada salvamento relevante gera uma versão (histórico automático, recuperável). | M | 1 | rascunho |
| RF-GPR-011 | **Revisões**: o usuário libera uma revisão (A, B, C... ou 01, 02...) com comentário; a revisão liberada é **congelada** (somente leitura). | M | 2 | rascunho |
| RF-GPR-012 | Criar nova revisão a partir da liberada (a de trabalho), sem alterar a congelada. | M | 2 | rascunho |
| RF-GPR-013 | Liberar revisão de montagem com todos os componentes de uma vez (liberação em conjunto). | M | 2 | rascunho |
| RF-GPR-014 | Abrir, comparar e exportar qualquer revisão anterior. | M | 2 | rascunho |
| RF-GPR-015 | Comparação visual entre revisões (geometria acrescentada/removida em cores). | S | 3 | rascunho |
| RF-GPR-016 | Estados simples: em trabalho, liberado, obsoleto. | M | 2 | rascunho |
| RF-GPR-017 | Tabela de revisões do desenho preenchida automaticamente (RF-DES-044). | M | 2 | rascunho |

## Gestão de links do projeto

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-GPR-020 | Visão geral de todos os links do projeto, com filtros por estado (ativo, congelado, quebrado, desatualizado). | M | 2 | rascunho |
| RF-GPR-021 | Renomear, mover ou copiar documentos **atualizando os links** de quem os usa. | M | 1 | rascunho |
| RF-GPR-022 | **Salvar como / copiar projeto**: copiar uma montagem com todas as dependências, escolhendo quais copiar e quais reutilizar. | M | 2 | rascunho |
| RF-GPR-023 | Reparar links quebrados em lote (procurar em pasta, por PN ou por nome). | M | 2 | rascunho |
| RF-GPR-024 | Pacote do projeto (arquivo único com todas as dependências) para enviar a terceiros. | M | 2 | rascunho |
| RF-GPR-025 | Bloqueio simples de edição para equipe em pasta compartilhada (quem está editando o quê). | S | 3 | rascunho |
