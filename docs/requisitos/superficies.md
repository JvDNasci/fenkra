# Requisitos — Superfícies Complexas

## Curvas

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-SUP-001 | Curvas 3D: por pontos, por polígono de controle, hélice, espiral, projeção de curva em superfície, interseção de superfícies. | M | 2 | rascunho |
| RF-SUP-002 | Continuidade de curvas: G0, G1 (tangente), G2 (curvatura) nas junções. | M | 2 | rascunho |
| RF-SUP-003 | Gráfico de curvatura (pente) em curvas. | M | 2 | rascunho |

## Superfícies paramétricas

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-SUP-010 | **Loft** de superfície e de sólido entre perfis, com curvas-guia e controle de continuidade nas extremidades. | M | 2 | rascunho |
| RF-SUP-011 | Extrusão, revolução e varredura de superfície. | M | 2 | rascunho |
| RF-SUP-012 | Superfície de preenchimento (fill) de contorno fechado com continuidade G0/G1/G2 nas bordas. | M | 2 | rascunho |
| RF-SUP-013 | Superfície de transição suave (blend) entre duas superfícies. | S | 3 | rascunho |
| RF-SUP-014 | Operações: aparar, estender, costurar, offset, espelhar, unir. | M | 2 | rascunho |
| RF-SUP-015 | Converter superfícies costuradas fechadas em sólido; espessar superfície em sólido. | M | 2 | rascunho |

## Análise

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-SUP-020 | **Análise de zebra** (listras refletidas) em tempo real. | M | 2 | rascunho |
| RF-SUP-021 | **Análise de desvio** entre superfícies/curvas (distância e desvio angular) com mapa de cores. | M | 2 | rascunho |
| RF-SUP-022 | Mapa de curvatura (gaussiana e média) com mapa de cores. | S | 2 | rascunho |
| RF-SUP-023 | Análise de ângulo de saída (draft) por cor. | S | 2 | rascunho |
| RF-SUP-024 | Verificação de continuidade nas arestas (G0/G1/G2) com indicação visual. | M | 2 | rascunho |

## Futuro: superfícies por pontos de controle

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-SUP-030 | Modelagem por gaiola de controle (estilo subdivisão do Blender): mover vértices, arestas e faces, extrudar, cortar loops. | C | 5 | rascunho |
| RF-SUP-031 | Converter a superfície de controle em B-rep (NURBS) utilizável pelas features sólidas. | C | 5 | rascunho |
| RF-SUP-032 | Importar/exportar malhas de controle de ferramentas abertas (ex.: OBJ do Blender). | C | 5 | rascunho |
| RF-SUP-033 | Verificar patentes antes de implementar (modelagem por subdivisão convertida em B-rep). | M | 5 | rascunho |
