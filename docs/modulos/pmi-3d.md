# PMI 3D

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/pmi-3d.md](../benchmark/pmi-3d.md).

## ✅ Pronto

- **Cotas e notas 3D** (Cota, nota) — Pmi(0..10).
- **GD&T, referências e rugosidade** (GD&T, datum, rugosidade, solda) — NoteKind Gdt/Datum; símbolo de solda ISO 2553.
- **Vistas e capturas** (Capturas) — Capturas na árvore.

- **STEP AP242 semântico** (Exportar, "Anotações semânticas (PMI)") — cotas com tolerância (26 dos 31 tipos do
  OCCT), 15 tipos de tolerância geométrica, datums com precedência, alvos de datum, ligados às faces; a releitura
  confere tipo, valor, faces e ordem. Mapa do que não atravessa em [ADR 0061](../arquitetura/adr/0061-pmi-semantico-no-step-ap242.md).

## 🟡 Parcial

- **PMI gráfico no STEP** (B191) — rugosidade, solda, notas, balões e vistas salvas não vão no arquivo; o gravador do OCCT só leva apresentação com geometria própria por anotação. O diálogo tem "nenhum / semântico"; falta o modo "gráfico".

- **Realce e chamadas inteligentes** (Seta) — Realce falha na montagem; uma seta só; seta atravessa a anotação (DR-31, DR-47, DR-48). `DR-48`

## ❌ Ainda não existe

- **Consultor de tolerância** (—) —  `BM-PMI-01`
