# Benchmark — Geral

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#geral).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Ocultar é hierárquico e local ao mesmo tempo; toda feature tem preview e diagnóstico de atualização.
- **SOLIDWORKS:** Pastas na árvore, "What's wrong" com reparo de referências, cores de estado.
- **Inventor:** Design Doctor para reparar; pastas e visibilidade por objeto.

## O que adotar no Fenkra

- Ocultar hierárquico que preserva o estado do filho (DR-30).
- Reparo com candidatos no 3D (ADR 0010).

## O que evitar

- Ações de pasta que sobrescrevem o estado de cada item.

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| GER-01 | Ocultar hierárquico | 🟡 | FolderShown sobrescreve o estado dos filhos; origem só pelo menu (DR-15, DR-30). | DR-30 |
| GER-02 | Pré-visualização ao vivo | 🟡 | Recompute ao vivo; rosca modelada sem preview (DR-02). | DR-02 |
| GER-03 | Direção normal a plano/superfície | 🟡 | Nem todos os comandos aceitam (DR-09). | DR-09 |
| GER-04 | Pastas e organização da árvore | 🟡 | Sem "selecionados → nova pasta" com escolha do pai (DR-46). | DR-46 |
| GER-05 | Assistente de reparo | 🟡 | Janela de reparo para comandos com erro; sem candidatos no 3D (ADR 0010). | DR-11 |
| GER-06 | Renderização de arestas | 🟡 | edge_smooth oculta tangências; linhas estranhas no render e em filetes de superfície (DR-08, DR-10). | DR-08 |
| GER-07 | API de automação | 🟡 | Macros próprias; sem API Python (RNF-011). | BM-GER-01 |
