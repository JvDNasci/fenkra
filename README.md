# Fenkra CAD

> *Projete. Dobre. Monte. Renasça.*

CAD 3D paramétrico open source, leve e modular: modelagem sólida com sketches paramétricos,
projeto relacional com geometria mestre, montagens e mecanismos, desenho técnico sincronizado,
chapas metálicas, estruturas soldadas e gestão de projeto.

> **Status:** alpha (`0.1.0-alpha.26`): sólidos, superfícies, chapa, estrutural/soldas, montagem e mecanismos,
> desenho técnico, PMI 3D e gestão de projeto já funcionam. Ver o [checklist](docs/requisitos/checklist.md) e o [backlog](docs/backlog/backlog.md).

## Rodar

```bash
cargo run -p fenkra-app --release
```

Pré-requisitos (Rust, MSVC e OpenCASCADE via vcpkg) em [docs/prototipo.md](docs/desenvolvimento.md).

## Código

| Crate | Papel |
|---|---|
| [fenkra-core](crates/fenkra-core) | Documento, árvore, linha de tempo, parâmetros, sketch e solver, referências, edição não destrutiva |
| [fenkra-kernel](crates/fenkra-kernel) | Fachada C++ do OpenCASCADE ligada ao Rust via `cxx` |
| [fenkra-solidos](crates/fenkra-solidos) | Recompute dos comandos: sólidos, superfícies, chapa, estrutural, roscas e perfis |
| [fenkra-montagem](crates/fenkra-montagem) | Montagem: restrições, solver, juntas, mecanismos, explodida, recursos de montagem |
| [fenkra-desenho](crates/fenkra-desenho) | Desenho técnico: folhas, vistas, cortes, cotas, tabelas, exportação PDF/SVG/DXF |
| [fenkra-project](crates/fenkra-project) | Gestão de projeto: versões, revisões congeladas, PN, estados |
| [fenkra-app](crates/fenkra-app) | Interface egui + viewport wgpu, bancadas, temas e tela inicial |

## Documentação

| Documento | Conteúdo |
|---|---|
| [Índice da documentação](docs/README.md) | Por onde começar e onde fica cada coisa |
| [Desenvolvimento](docs/desenvolvimento.md) | Compilar, testar, variáveis de teste, release e git |
| [Visão](docs/produto/visao.md) · [Roadmap](docs/produto/roadmap.md) | O que o projeto é e as fases |
| [Requisitos](docs/requisitos/README.md) · [Checklist](docs/requisitos/checklist.md) | Requisitos com IDs e o estado de cada feature |
| [Módulos](docs/modulos/) | Estado e lacunas de cada módulo |
| [Benchmark](docs/benchmark/) | Catálogo de features e comparação de workflow com outros CADs |
| [Backlog](docs/backlog/backlog.md) | O que está aberto, com prioridade e onda |
| [ADRs](docs/arquitetura/adr/) | Decisões de arquitetura |
| [Contextos](docs/contextos/) | Material de referência do usuário e o design review |
| [Glossário](docs/produto/glossario.md) | Termos próprios do projeto |
| [Procedência](docs/arquitetura/procedencia.md) | Fontes consultadas em cada especificação |

## Regras de especificação (resumo)

- Funcionalidades de outros CADs podem ser estudadas pela documentação pública, mas
  **tudo é reescrito com nossas palavras**. Não copiar texto, imagens, ícones ou screenshots.
- Não usar nomes que são marcas registradas de terceiros. Usar os termos do [glossário](docs/produto/glossario.md).
- Toda fonte consultada vai para o [registro de procedência](docs/arquitetura/procedencia.md).
- Código de terceiros só entra respeitando a licença. Ver [reuso open source](docs/arquitetura/reuso-opensource.md).
