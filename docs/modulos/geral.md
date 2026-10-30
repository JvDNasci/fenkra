# Geral

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/geral.md](../benchmark/geral.md).

## 🟡 Parcial

- **Ocultar hierárquico** (Olho por item) — FolderShown sobrescreve o estado dos filhos; origem só pelo menu (DR-15, DR-30). `DR-30`
- **Pré-visualização ao vivo** (Recompute ao vivo) — Recompute ao vivo; rosca modelada sem preview (DR-02). `DR-02`
- **Direção normal a plano/superfície** (Direção) — Nem todos os comandos aceitam (DR-09). `DR-09`
- **Pastas e organização da árvore** (Pastas de geometria) — Sem "selecionados → nova pasta" com escolha do pai (DR-46). `DR-46`
- **Assistente de reparo** (Assistente de reparo) — Janela de reparo para comandos com erro; sem candidatos no 3D (ADR 0010). `DR-11`
- **Renderização de arestas** (Arestas vivas) — edge_smooth oculta tangências; linhas estranhas no render e em filetes de superfície (DR-08, DR-10). `DR-08`
- **API de automação** (Macros) — Macros próprias; sem API Python (RNF-011). `BM-GER-01`
