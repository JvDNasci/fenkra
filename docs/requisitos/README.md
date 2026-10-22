# Requisitos

## Convenções

- **ID**: `RF-<área>-<nº>` para funcionais, `RNF-<nº>` para não-funcionais. IDs nunca são reaproveitados.
- **Prioridade** (MoSCoW): **M** = obrigatório, **S** = deveria ter, **C** = poderia ter, **W** = não agora.
- **Fase**: conforme o [roadmap](../roadmap.md).
- **Status**: `rascunho` → `aprovado` → `especificado` (tem ficha em `specs/`) → `implementado`.

## Como usar

1. Cada feature está no [**checklist de features**](checklist.md), com os requisitos que ela atende.
2. Para montar uma feature: aprovar os requisitos → escrever a ficha em `specs/` → implementar → testar.
3. Marcar cada etapa no checklist e atualizar o status do requisito.

## Fundação (transversal)

| Arquivo | Prefixo | Área |
|---|---|---|
| [nao-funcionais.md](nao-funcionais.md) | RNF | Desempenho, usabilidade, portabilidade, qualidade |
| [nucleo.md](nucleo.md) | RF-NUC | Documento, tipos de documento, formatos |
| [arvore.md](arvore.md) | RF-ARV | Árvore da peça (Origem, Referências, Parâmetros, pastas, coletores) e linha de tempo |
| [parametros.md](parametros.md) | RF-PAR | Parâmetros, cálculos e propriedades (PN, material...) |
| [links-relacional.md](links-relacional.md) | RF-LNK | Projeto relacional, geometria mestre, links entre peças |
| [esboco.md](esboco.md) | RF-ESB | Sketch 2D/3D e restrições |
| [biblioteca.md](biblioteca.md) | RF-BIB | Biblioteca pública de hardware e perfis |
| [interface.md](interface.md) | RF-UI | Menu minimalista, modos de visualização |

## Módulos

| Arquivo | Prefixo | Módulo |
|---|---|---|
| [solidos.md](solidos.md) | RF-SOL | Modelagem Sólida |
| [superficies.md](superficies.md) | RF-SUP | Superfícies Complexas |
| [montagem.md](montagem.md) | RF-MON | Montagem e Mecanismos |
| [desenho.md](desenho.md) | RF-DES | Desenho Técnico |
| [chapas.md](chapas.md) | RF-CHP | Chapas Metálicas |
| [soldas.md](soldas.md) | RF-SDA | Estruturas Soldadas |
| [gestao-projeto.md](gestao-projeto.md) | RF-GPR | Gestão de Projeto |
| [malha.md](malha.md) | RF-MAL | Conversão malha → sólido |
