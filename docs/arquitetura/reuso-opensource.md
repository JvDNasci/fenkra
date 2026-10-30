# Reuso de projetos open source

> ⚠️ Confirme sempre a licença no repositório oficial antes de copiar qualquer código. Esta tabela é
> um ponto de partida, não uma verificação jurídica.

## Regra geral

A licença do projeto define o que pode entrar. Com a licença proposta **LGPL-2.1-or-later**
(ver [ADR 0003](adr/0003-licenca.md)):

- ✅ **Pode incorporar código**: LGPL-2.1+, MIT, BSD, Apache-2.0, MPL-2.0, zlib, Boost.
- ⚠️ **Pode usar como biblioteca vinculada**: LGPL-3.0 (fica LGPL-3 no conjunto distribuído).
- ❌ **Não pode incorporar** sem tornar o projeto inteiro GPL: GPL-2.0, GPL-3.0.
  Código GPL pode ser **estudado** para entender a abordagem, mas a implementação deve ser nossa.

Ao copiar código permitido: manter os cabeçalhos de copyright originais, registrar a origem em
`THIRD_PARTY.md` (a criar) e marcar as modificações.

## Bibliotecas que vamos usar como dependência

| Projeto | Licença | Uso previsto |
|---|---|---|
| OpenCASCADE (OCCT) | LGPL-2.1 + exceção | Kernel B-rep, booleanas, filetes, STEP/IGES, HLR |
| Qt 6 | LGPL-3.0 / comercial | Interface |
| pybind11 | BSD-3 | API Python |
| Eigen | MPL-2.0 | Álgebra linear (solver) |
| IfcOpenShell | LGPL-3.0 | IFC (fase BIM) |
| Manifold | Apache-2.0 | Booleanas de malha rápidas (opcional) |
| Open3D | MIT | Processamento de malha / RANSAC (alternativa ao CGAL) |
| libigl | MPL-2.0 | Utilitários de malha |

## Código que pode ser aproveitado (estudar e possivelmente incorporar)

| Projeto | Licença | O que aproveitar | Incorporar? |
|---|---|---|---|
| **FreeCAD** | LGPL-2.0-or-later | **PlaneGCS** (solver de esboço), importadores, lógica de furo/rosca, tabelas de parafusos, partes do TechDraw como referência | ✅ |
| **CadQuery** | Apache-2.0 | Wrappers de OCCT, seleção de faces/arestas, exemplos de uso do kernel | ✅ |
| **build123d** | Apache-2.0 | Idem; boa referência de API de alto nível | ✅ |
| **SolveSpace** | GPL-3.0 | Solver de restrições excelente (como referência de algoritmo) | ❌ só estudar |
| **LibreCAD** | GPL-2.0 | Ferramentas de esboço 2D, aparar/estender, DXF | ❌ só estudar |
| **QCAD (community)** | GPL-3.0 | Ferramentas 2D, cotas | ❌ só estudar |
| **OpenSCAD** | GPL-2.0 | — | ❌ só estudar |
| **LibreDWG** | GPL-3.0 | Leitura DWG | ❌ (ou processo separado) |
| **CGAL** | GPL-3.0 na maioria dos pacotes (inclui Shape Detection); LGPL no kernel | RANSAC de primitivas para malha → sólido | ❌ preferir Open3D |
| **Bonsai (ex-BlenderBIM)** | GPL-3.0 | Referência de fluxo BIM | ❌ só estudar |

## Candidatos imediatos para o MVP

1. **PlaneGCS** (FreeCAD) — avaliar extrair como biblioteca independente para o solver de esboço.
2. **Seleção e wrappers OCCT** do CadQuery/build123d — referência para a camada de geometria.
3. **Tabelas de furos/roscas e fixadores ISO** — dados do FreeCAD (verificar licença dos dados).
4. **RANSAC do Open3D** — detecção de planos para malha → sólido nível 2/3.
