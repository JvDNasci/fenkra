# Requisitos — Desenho Técnico

Ver [ADR 0007](../arquitetura/adr/0007-vistas-como-blocos.md).

## Folhas e normas

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-DES-001 | Folhas A0–A4 e carimbo editável (NBR 10068 / ISO 5457), com campos preenchidos pelas propriedades (RF-PAR-025). | M | 2 | rascunho |
| RF-DES-002 | Projeção em 1º diedro (padrão ABNT/ISO) e 3º diedro. | M | 2 | rascunho |
| RF-DES-003 | Escala por vista e da folha, com valores normalizados (NBR 8196). | M | 2 | rascunho |
| RF-DES-004 | Modelos de folha e estilos (cotas, textos, linhas) por empresa. | M | 2 | rascunho |

## Vistas inteligentes

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-DES-010 | **Cada vista é um bloco** (grupo) com sua geometria 2D, cotas e anotações. | M | 2 | rascunho |
| RF-DES-011 | Estado do link da vista com o 3D: **ligada** (atualiza), **travada** (congela a última geração) ou **desatualizada** (indicada visualmente, atualiza sob comando). | M | 2 | rascunho |
| RF-DES-012 | Vistas padrão (frontal, superior, lateral, isométrica) por arrastar ou um comando, com escolha automática de orientação e escala. | M | 2 | rascunho |
| RF-DES-013 | Vistas projetadas, auxiliares, de detalhe e de quebra. | M | 2 | rascunho |
| RF-DES-014 | **Cortes** (total, meio corte, em desvio, parcial) e **seções** (rebatida e removida). | M | 2 | rascunho |
| RF-DES-015 | Pré-visualização da vista em < 1 s; cálculo exato de linhas ocultas em background. | M | 2 | rascunho |
| RF-DES-016 | Linhas ocultas, de centro e eixos de simetria automáticos (configuráveis). | M | 2 | rascunho |
| RF-DES-017 | Vista planificada de chapa com linhas e tabela de dobras. | M | 2 | rascunho |
| RF-DES-018 | Vistas de montagem com estado de exibição e vista explodida. | S | 3 | rascunho |

## Cotagem e anotações

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-DES-020 | Cotas lineares, angulares, radiais, diametrais, ordenadas, de chanfro, em cadeia e por linha de base. | M | 2 | rascunho |
| RF-DES-021 | Cotas associativas presas à topologia persistente; sobrevivem a mudanças no modelo. | M | 2 | rascunho |
| RF-DES-022 | Cotas órfãs são destacadas, nunca apagadas silenciosamente. | M | 2 | rascunho |
| RF-DES-023 | Importar cotas do modelo (dimensões dos sketches e features). | S | 2 | rascunho |
| RF-DES-024 | Chamadas automáticas de furos e roscas a partir do furo inteligente. | M | 2 | rascunho |
| RF-DES-025 | Tolerâncias, ajustes ISO, GD&T, acabamento superficial, símbolos de solda. | S | 3 | rascunho |
| RF-DES-026 | Hachuras por material (NBR 12298). | M | 2 | rascunho |
| RF-DES-027 | Notas com campos de propriedades e parâmetros. | M | 2 | rascunho |

## Geometria 2D

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-DES-030 | Desenho 2D geométrico livre: linhas, arcos, círculos, polilinhas, splines, com snaps. | M | 2 | rascunho |
| RF-DES-031 | Transformações simples: mover, copiar, girar, espelhar, escalar, offset, aparar, estender. | M | 2 | rascunho |
| RF-DES-032 | Restrições 2D opcionais (o mesmo solver dos sketches). | S | 3 | rascunho |
| RF-DES-033 | Blocos 2D reutilizáveis (símbolos). | S | 3 | rascunho |
| RF-DES-034 | Camadas (layers) com cor, tipo e espessura de linha. | M | 2 | rascunho |

## Tabelas e saída

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-DES-040 | **Lista de peças / BOM** associativa, com colunas configuráveis por propriedade (PN, descrição, material, tipo, quantidade, personalizadas). | M | 2 | rascunho |
| RF-DES-041 | BOM por nível, planificada (só peças) e só itens comprados. | M | 2 | rascunho |
| RF-DES-042 | Balões associativos ligados à BOM. | M | 2 | rascunho |
| RF-DES-043 | Tabelas de lista de corte (soldas), dobras (chapas) e furos. | M | 2 | rascunho |
| RF-DES-044 | Tabela de revisões ligada à gestão de projeto. | M | 2 | rascunho |
| RF-DES-045 | Exportar PDF (vetorial), DXF, SVG e PNG; exportar várias folhas em lote. | M | 2 | rascunho |
