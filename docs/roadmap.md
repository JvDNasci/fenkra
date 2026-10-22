# Roadmap

> As fases são sequenciais em prioridade, não em calendário. Estimativas grosseiras para uma pessoa em tempo parcial.

## Fase 0 — Especificação (agora)
- Visão, requisitos, ADRs principais
- Fichas das features do MVP
- Levantamento do que reaproveitar de projetos abertos

## Fase 1 — Fundação + Esboço + Sólidos básicos (MVP parte 1)
- Core: documento, árvore de features, recompute incremental em background, desfazer/refazer
- Naming topológico persistente
- Viewport com seleção, navegação e renderização própria
- Esboço 2D com solver de restrições
- Extrusão, revolução, furo, filete, chanfro
- Timeline com rollback
- STEP / STL import e export

## Fase 2 — Desenho técnico + Malha → sólido (MVP parte 2)
- Folhas, vistas, cortes, cotas associativas, exportação PDF/DXF
- Malha → sólido níveis 1 e 2
- Casca, padrões, espelhar, booleanas entre corpos
- API Python

**Fim do MVP:** peça mecânica completa, do esboço ao desenho cotado em PDF.

## Fase 3 — Estruturas soldadas + Montagens
- Esboço 3D, membros estruturais, biblioteca de perfis, cantos, lista de corte
- Montagens com juntas
- BOM e balões
- Sweep, loft

## Fase 4 — Refinamento
- Malha → sólido nível 3 (primitivas)
- Simbologia de solda, GD&T
- Biblioteca de elementos normalizados, vista explodida

## Fase 5 — BIM (módulo separado)
- Objetos arquitetônicos paramétricos, IFC (via IfcOpenShell)
- Representação 2D simbólica (NBR 6492) reaproveitando o motor de desenho
