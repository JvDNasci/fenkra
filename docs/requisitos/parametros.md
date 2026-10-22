# Requisitos — Parâmetros, cálculos e propriedades

## Parâmetros e cálculos

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-PAR-001 | Tabela de parâmetros por documento, com nome, valor, unidade, expressão e comentário. | M | 1 | rascunho |
| RF-PAR-002 | Qualquer cota de sketch ou parâmetro de feature aceita expressão (ex.: `L/2 + folga`). | M | 1 | rascunho |
| RF-PAR-003 | Funções matemáticas (sin, cos, sqrt, min, max, round, ceil, floor, abs) e condicionais (`if(cond, a, b)`). | M | 1 | rascunho |
| RF-PAR-004 | Tipos: comprimento, ângulo, número, inteiro, booleano, texto e lista (seleção entre opções). | M | 1 | rascunho |
| RF-PAR-005 | Escopos: sketch, peça, montagem e geometria mestre; referência entre escopos por nome qualificado (ex.: `Mestre.diametro_eixo`). | M | 1 | rascunho |
| RF-PAR-006 | Detecção de dependência circular com mensagem clara indicando o ciclo. | M | 1 | rascunho |
| RF-PAR-007 | Tabelas de configuração: várias linhas de valores para gerar variantes da mesma peça (ex.: tamanhos de um flange). | S | 2 | rascunho |
| RF-PAR-008 | Importar/exportar parâmetros em CSV/XLSX. | S | 2 | rascunho |
| RF-PAR-009 | Regras simples de validação: limites mínimo/máximo e aviso quando violados. | S | 2 | rascunho |
| RF-PAR-010 | Parâmetros calculados a partir da geometria (ex.: comprimento de aresta, área de face, massa), somente leitura. | S | 2 | rascunho |

## Propriedades de peça e montagem

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-PAR-020 | Propriedades padrão em toda peça e montagem: **PN** (número da peça), **descrição**, **material**, **tipo de peça**, revisão, autor. | M | 1 | rascunho |
| RF-PAR-021 | Tipos de peça: fabricada, comprada, hardware (biblioteca), chapa, membro estrutural, montagem, fantasma. | M | 1 | rascunho |
| RF-PAR-022 | Propriedades personalizadas definidas pelo usuário (texto, número, data, lista), inclusive com expressão. | M | 1 | rascunho |
| RF-PAR-023 | Modelos de propriedades por empresa/projeto (quais campos existem e quais são obrigatórios). | S | 2 | rascunho |
| RF-PAR-024 | Biblioteca de materiais com densidade, cor/aparência e hachura de desenho. | M | 1 | rascunho |
| RF-PAR-025 | Toda propriedade pode ser usada no carimbo do desenho, em notas e nas colunas da BOM. | M | 2 | rascunho |
| RF-PAR-026 | Propriedades por corpo (em peças multi-corpo, soldas e chapas). | M | 2 | rascunho |
