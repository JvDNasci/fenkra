# ADR 0002: C++20 no núcleo, Qt 6 na interface, Python só para scripts

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
Desempenho é diferencial do projeto (RNF-001 a RNF-003). O FreeCAD tem muita lógica em Python no
caminho crítico, o que contribui para a lentidão. O kernel (OCCT) é C++.

## Decisão
- **Núcleo em C++20**, sem dependência de UI (RNF-005).
- **Interface em Qt 6**, com viewport de renderização própria (OpenGL 4.5, com caminho para Vulkan).
- **Python (pybind11)** apenas como API de scripts e plugins.
- Build com **CMake** e dependências via **vcpkg**.

## Alternativas consideradas
- **Rust:** segurança de memória e boa ergonomia, mas integração com OCCT exige bindings extensos.
- **Web (TypeScript + OCCT/WASM):** distribuição fácil, mas desempenho e memória limitados para modelos grandes.
  Mantemos a porta aberta compilando o núcleo para WASM no futuro.

## Consequências
- ✅ Acesso direto e sem overhead ao OCCT.
- ✅ Núcleo reutilizável em CLI, testes headless e WASM.
- ⚠️ C++ exige disciplina: sanitizers, testes e revisão de código desde o início.
