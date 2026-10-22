# ADR 0002: Rust no sistema, C++ só na fachada do kernel, egui + wgpu na interface

- **Status:** aceita (substitui a proposta inicial "C++20 + Qt 6")
- **Data:** 2026-10-21

## Contexto
Desempenho e leveza são diferenciais do projeto (RNF-001 a RNF-003). O FreeCAD tem muita lógica em
Python no caminho crítico, o que contribui para a lentidão. O kernel (OCCT) é C++.
Um protótipo em Python foi descartado pelo mantenedor: o objetivo é um aplicativo leve desde o início.

## Decisão
- **Rust** para todo o sistema: núcleo (documento, árvore, parâmetros, links), módulos, gestão de
  projeto, renderização e interface.
- **C++17** apenas na **fachada fina do OpenCASCADE** (`crates/scc-kernel/cpp`), ligada ao Rust pelo
  crate **`cxx`**. Nenhuma exceção do OCCT atravessa a ponte; tudo vira `Result`.
- **Interface: egui (eframe)**, imediata e leve, com **viewport própria em wgpu** (textura offscreen
  com MSAA e profundidade).
- **OCCT via vcpkg** (`opencascade:x64-windows-release`); o `build.rs` detecta a instalação e, sem ela,
  compila em modo "sem kernel" para a interface continuar testável.
- Python fica como possibilidade futura **apenas** para scripts de usuário (RNF-011), fora do caminho crítico.

## Estrutura

| Crate | Linguagem | Papel |
|---|---|---|
| `scc-core` | Rust | Documento, árvore, linha de tempo, parâmetros/expressões, referências, edição não destrutiva |
| `scc-kernel` | Rust + C++ | Fachada segura do OpenCASCADE |
| `scc-solidos` | Rust | Módulo Modelagem Sólida: recompute dos comandos |
| `scc-project` | Rust | Gestão de projeto: versões, revisões congeladas, PN |
| `scc-app` | Rust | Interface egui + viewport wgpu |

## Alternativas consideradas
- **C++20 + Qt 6:** acesso direto ao OCCT, mas build mais pesado e mais risco de erros de memória.
- **Python + PySide6 + OCP:** rápido para prototipar, mas descartado por peso e desempenho.
- **Qt via cxx-qt / Slint:** visual mais nativo; egui escolhido por leveza e velocidade de iteração.
  A interface fica isolada em `scc-app`, então trocar de toolkit no futuro não afeta o núcleo.
- **Kernel em Rust puro (Truck, Fornjot):** ainda imaturos em filetes, offset e STEP.

## Consequências
- ✅ Binário leve, sem runtime, com segurança de memória fora da fachada C++.
- ✅ Núcleo reutilizável em CLI, testes headless e, no futuro, WASM.
- ⚠️ Cada função nova do OCCT exige uma entrada na fachada C++ e na ponte `cxx`.
- ⚠️ egui é de modo imediato: a interface precisa evitar trabalho pesado por quadro
  (por isso o recompute roda em outra thread e a viewport só renderiza quando algo muda).
