# Desenho técnico

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/desenho-tecnico.md](../benchmark/desenho-tecnico.md).

## ✅ Pronto

- **Folhas, formatos e carimbo** (Folhas, carimbo) — Formatos ISO/ANSI, carimbos ISO 7200/ABNT, zonas ISO 5457.
- **Cortes e seções** (Corte) — Teste vistas_corte_detalhe_cotas_tabelas_e_exportacao.
- **Cotas associativas** (Cotas) — Cotas com estilo, setas e posição do texto.
- **Tabelas** (Tabelas) — TableKind (BOM, CutList, …).
- **Exportação** (PDF, SVG, DXF) — export.rs; teste de exportação.

## 🟡 Parcial

- **Vistas base e projetadas** (Vistas) — Só 1º diedro; sem 3º diedro (ASME). `BM-DES-01`
- **Detalhe, auxiliar, quebrada, corte parcial** (Detalhe, auxiliar) — Detalhe e auxiliar; sem vista interrompida nem corte parcial. `BM-DES-02`
- **Hachuras** (Hachura) — Passo fixo 2,5 sem propriedades de padrão/ângulo (layout.rs). `BM-DES-03`
- **Balões e lista de peças** (Balões, BOM) — Balões ligados à BOM; não funciona para estrutural (DR-33). `DR-33`
- **Propriedades por norma e herança** (Estilos) — Estilos por cota/nota; sem cadeia norma→folha→vista completa (DR-35..43). `DR-35`

## ❌ Ainda não existe

- **Atualização e estado da vista** (Recarrega silencioso) — Models::refresh() recarrega por data do arquivo; sem estado visível. `DR-34`
- **Desenho de estrutura soldada** (—) — Sem vistas por corpo ligadas à lista (DR-44, DR-45). `DR-45`
