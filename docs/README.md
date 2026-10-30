# Documentação do Fenkra

## Por onde começar

| Quero… | Leia |
|---|---|
| entender o produto | [Visão](produto/visao.md) → [Roadmap](produto/roadmap.md) → [Glossário](produto/glossario.md) |
| compilar, testar, gerar release | [Desenvolvimento](desenvolvimento.md) |
| saber o que já funciona | [Checklist](requisitos/checklist.md) e [Módulos](modulos/) |
| saber o que vem a seguir | [Backlog](backlog/backlog.md) |
| comparar com outros CADs | [Benchmark](benchmark/): catálogo de features e workflows |
| entender uma decisão técnica | [ADRs](arquitetura/adr/) |

## Estrutura

```
docs/
  README.md            este índice
  desenvolvimento.md   compilar, testar, variáveis FENKRA_DEMO_*, release, git
  produto/             visão, roadmap (com o estado atual), glossário
  arquitetura/         ADRs, reuso de open source, registro de procedência
  requisitos/          requisitos por área (IDs RF-/RNF-) e o checklist de features
  specs/               fichas de feature (modelo + modos de criação)
  modulos/             um arquivo por módulo: o que existe, workflow, lacunas
  benchmark/           catálogo de features e workflow comparado (só fontes públicas)
  backlog/             backlog.md (aberto) e historico/ (backlogs das alphas 1–26)
  contextos/           material de referência do usuário e o design review (DR-01…DR-58)
```

## Convenções

- **IDs de backlog:**
  - `DR-NN`: slide do [design review](contextos/design-review.md);
  - `CTX-<tema>`: um texto de [contexto](contextos/);
  - `BM-<módulo>-NN`: lacuna achada no [benchmark](benchmark/).
- **Requisitos:** `RF-<área>-NNN` / `RNF-NNN` (ver [requisitos](requisitos/README.md)).
- **Fontes externas:** só documentação pública, reescrita com palavras próprias, com cada URL
  registrada em [procedência](arquitetura/procedencia.md) e em [benchmark/fontes.md](benchmark/fontes.md).
