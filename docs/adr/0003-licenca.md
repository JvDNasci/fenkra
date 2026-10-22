# ADR 0003: Licença LGPL-2.1-or-later

- **Status:** proposta (decisão do mantenedor)
- **Data:** 2026-10-21

## Contexto
A licença define que código de terceiros podemos incorporar e como outros podem usar o projeto.
Queremos poder aproveitar código do FreeCAD (LGPL-2.0+) e usar OCCT (LGPL-2.1) e Qt (LGPL-3).

## Decisão (proposta)
**LGPL-2.1-or-later** para o projeto.

## Alternativas consideradas
- **GPL-3.0:** permitiria incorporar SolveSpace, CGAL, LibreCAD e LibreDWG, mas impede que empresas
  usem o núcleo como biblioteca em produtos próprios, o que reduz adoção e contribuição corporativa.
- **MIT/Apache-2.0:** máxima liberdade, mas **não** permite incorporar código LGPL do FreeCAD,
  e o binário distribuído continua com obrigações LGPL do OCCT e do Qt.
- **LGPL-2.1-or-later (escolhida):** compatível com FreeCAD, OCCT e Qt; permite uso como biblioteca;
  modificações no próprio projeto continuam abertas.

## Consequências
- ✅ Podemos incorporar código do FreeCAD (ex.: PlaneGCS).
- ❌ Não podemos incorporar código GPL; ver [reuso-opensource.md](../reuso-opensource.md).
- Recomendado consultar um advogado de propriedade intelectual antes da primeira versão pública.
