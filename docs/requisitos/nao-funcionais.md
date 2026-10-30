# Requisitos não-funcionais

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RNF-001 | A interface nunca bloqueia durante recompute. Toda operação geométrica roda fora da thread da UI. | M | 1 | rascunho |
| RNF-002 | Recompute incremental: alterar uma feature recalcula apenas ela e as dependentes, inclusive entre documentos ligados. | M | 1 | rascunho |
| RNF-003 | Rotação/zoom/pan a ≥ 60 fps em modelo de 1 milhão de triângulos em GPU integrada recente. | M | 1 | rascunho |
| RNF-004 | Inicialização em menos de 3 s em SSD. | S | 1 | rascunho |
| RNF-005 | Núcleo sem dependência de UI, permitindo uso headless (scripts, testes, CLI) e futura compilação para WASM. | M | 1 | rascunho |
| RNF-006 | Formato de arquivo nativo aberto, documentado e versionado, com migração entre versões. | M | 1 | rascunho |
| RNF-007 | Salvamento automático e recuperação após falha. | M | 1 | rascunho |
| RNF-008 | Referências topológicas persistentes. Ver [ADR 0004](../arquitetura/adr/0004-naming-topologico.md). | M | 1 | rascunho |
| RNF-009 | Windows 10+ e Linux (x86_64) no MVP. macOS posterior. | M | 1 | rascunho |
| RNF-010 | Interface em português e inglês desde o início (i18n). | M | 1 | rascunho |
| RNF-011 | API Python para scripts e plugins, fora do caminho crítico de desempenho. | S | 2 | rascunho |
| RNF-012 | Testes de regressão geométrica automáticos (modelos de referência recalculados a cada commit). | M | 1 | rascunho |
| RNF-013 | Desfazer/refazer ilimitado dentro da sessão. | M | 1 | rascunho |
| RNF-014 | Unidades: mm, cm, m, pol; o documento guarda a unidade e converte na exibição. | M | 1 | rascunho |
| RNF-015 | Arquitetura modular: cada módulo é um plugin sobre o núcleo e pode ser desativado. Ver [ADR 0005](../arquitetura/adr/0005-arquitetura-modular.md). | M | 1 | rascunho |
| RNF-016 | Montagem de 1.000 componentes abre em modo visualização em menos de 10 s, sem carregar o histórico das peças. | M | 2 | rascunho |
| RNF-017 | Desenho com 20 vistas travadas abre em menos de 2 s (vistas travadas não recalculam). | M | 2 | rascunho |
