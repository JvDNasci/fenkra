# Fenkra CAD

> *Projete. Dobre. Monte. Renasça.*

CAD 3D paramétrico open source, leve e modular: modelagem sólida com sketches paramétricos,
projeto relacional com geometria mestre, montagens e mecanismos, desenho técnico sincronizado,
chapas metálicas, estruturas soldadas e gestão de projeto.

> **Status:** protótipo v0.1 do módulo Modelagem Sólida (Rust + OpenCASCADE). Ver [protótipo](docs/prototipo.md).

## Rodar

```bash
cargo run -p fenkra-app --release
```

Pré-requisitos (Rust, MSVC e OpenCASCADE via vcpkg) em [docs/prototipo.md](docs/prototipo.md).

## Código

| Crate | Papel |
|---|---|
| [fenkra-core](crates/fenkra-core) | Documento, árvore, linha de tempo, parâmetros, referências, edição não destrutiva |
| [fenkra-kernel](crates/fenkra-kernel) | Fachada C++ do OpenCASCADE ligada ao Rust via `cxx` |
| [fenkra-solidos](crates/fenkra-solidos) | Módulo Modelagem Sólida (recompute dos comandos) |
| [fenkra-project](crates/fenkra-project) | Gestão de projeto: versões, revisões congeladas, PN |
| [fenkra-app](crates/fenkra-app) | Interface egui + viewport wgpu |

## Documentação

| Documento | Conteúdo |
|---|---|
| [Protótipo](docs/prototipo.md) | Como compilar, o que já funciona e limitações |
| [Visão](docs/00-visao.md) | O que o projeto é, módulos, diferenciais e não-objetivos |
| [Requisitos](docs/requisitos/README.md) | Requisitos funcionais e não-funcionais com IDs |
| [Roadmap](docs/roadmap.md) | Fases e escopo de cada uma |
| [Reuso open source](docs/reuso-opensource.md) | O que dá pra aproveitar de outros projetos e as licenças |
| [ADRs](docs/adr/) | Decisões de arquitetura registradas |
| [Especificações](docs/specs/) | Ficha detalhada de cada feature |
| [Glossário](docs/glossario.md) | Termos próprios do projeto |
| [Procedência](docs/procedencia.md) | Fontes consultadas em cada especificação |

## Regras de especificação (resumo)

- Funcionalidades de outros CADs podem ser estudadas pela documentação pública, mas
  **tudo é reescrito com nossas palavras**. Não copiar texto, imagens, ícones ou screenshots.
- Não usar nomes que são marcas registradas de terceiros. Usar os termos do [glossário](docs/glossario.md).
- Toda fonte consultada vai para o [registro de procedência](docs/procedencia.md).
- Código de terceiros só entra respeitando a licença. Ver [reuso open source](docs/reuso-opensource.md).
