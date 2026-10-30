# Desenho técnico

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/desenho-tecnico.md](../benchmark/desenho-tecnico.md).

## ✅ Pronto

- **Folhas, formatos e carimbo** (Folhas, carimbo) — Formatos ISO/ANSI, carimbos ISO 7200/ABNT, zonas ISO 5457.
- **Cortes e seções** (Corte) — Teste vistas_corte_detalhe_cotas_tabelas_e_exportacao.
- **Cotas associativas** (Cotas) — Cotas com estilo, setas e posição do texto.
- **Balões e lista de peças** (Balões, BOM) — Balões automáticos ligados à BOM e à lista de corte por item, inclusive em vistas estruturais (B26). `DR-33`
- **Tabelas** (Tabelas) — TableKind (BOM, CutList, …).
- **Exportação** (PDF, SVG, DXF) — export.rs; teste de exportação.

## 🟡 Parcial

- **Vistas base e projetadas** (Vistas) — Só 1º diedro; sem 3º diedro (ASME). `BM-DES-01`
- **Detalhe, auxiliar, quebrada, corte parcial** (Detalhe, auxiliar) — Detalhe e auxiliar; sem vista interrompida nem corte parcial. `BM-DES-02`
- **Hachuras** (Hachura) — Passo fixo 2,5 sem propriedades de padrão/ângulo (layout.rs). `BM-DES-03`
- **Propriedades por norma e herança** (Estilos) — Estilos por cota/nota; sem cadeia norma→folha→vista completa (DR-35..43). `DR-35`
- **Desenho de estrutura soldada** (—) — Lista de corte e balões por item (B26); falta vista própria por corpo/item e detalhe de ponta. `B50`

## ❌ Ainda não existe

- **Atualização e estado da vista** (Recarrega silencioso) — Models::refresh() recarrega por data do arquivo; sem estado visível. `DR-34`
