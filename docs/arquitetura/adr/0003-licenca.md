# ADR 0003: Licença LGPL-2.1-or-later

- **Status:** aceita em 13/11/2026 pelo mantenedor
- **Data:** 2026-10-21

## Contexto
A licença define que código de terceiros podemos incorporar e como outros podem usar o projeto.
Queremos poder aproveitar código do FreeCAD (LGPL-2.0+) e usar OCCT (LGPL-2.1) e Qt (LGPL-3).

## Decisão
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

## Titular e contribuições (decisão de 13/11/2026)
- O titular dos direitos do Fenkra é **João Vitor Dias Nascimento, pessoa física**. Quando a empresa dele
  for aberta, os direitos podem ser cedidos a ela por documento próprio; esta ADR será atualizada.
- O contato público é feito pelas issues do repositório (https://github.com/JvDNasci/fenkra). Por enquanto,
  nenhum e-mail pessoal fica no repositório.
- As contribuições passam a exigir o [CLA](../../../CLA.md) em lugar do DCO. Assim o titular pode oferecer
  no futuro uma solução integrada sob outra licença sem fechar o núcleo, que segue LGPL-2.1-or-later.
- A busca de patentes não será contratada enquanto o projeto for livre e gratuito. A decisão será revista
  antes de qualquer venda.
