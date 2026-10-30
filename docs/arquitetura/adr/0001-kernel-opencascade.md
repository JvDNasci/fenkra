# ADR 0001: Usar OpenCASCADE como kernel geométrico

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
Um kernel B-rep (sólidos com faces exatas, booleanas, filetes, offset, STEP) é a parte mais cara de um CAD.
Os kernels comerciais (Parasolid, ACIS, CGM) são pagos e fechados. Escrever um do zero leva décadas.

## Decisão
Usar **OpenCASCADE Technology (OCCT)** como kernel, isolado atrás de uma camada própria (`geom/`)
para que o resto do sistema não dependa diretamente da API do OCCT.

## Alternativas consideradas
- **Truck (Rust):** moderno e Apache-2.0, mas imaturo em filetes, offset e STEP.
- **Fornjot (Rust):** experimental.
- **Kernel próprio:** inviável no horizonte do projeto.
- **Parasolid/ACIS:** licença incompatível com open source.

## Consequências
- ✅ Booleanas, filetes, casca, STEP/IGES e HLR disponíveis desde o início.
- ✅ Mesma base do FreeCAD e CadQuery: muito material e código de referência.
- ⚠️ Filetes complexos e booleanas em casos-limite falham às vezes; precisaremos de tratamento de erro e heurísticas.
- ⚠️ HLR exato é lento; ver requisito RF-DES-005.
- A camada `geom/` permite trocar ou complementar o kernel no futuro.
