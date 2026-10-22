# Protótipo v0.1 — Modelagem Sólida

Primeira versão executável do SolidCadCraft: interface, gestão de projeto simples e o módulo
Modelagem Sólida sobre o OpenCASCADE 8.0.1.

## Como compilar e rodar (Windows)

Pré-requisitos (uma vez):

1. Visual Studio com o workload **Desenvolvimento para desktop com C++** (MSVC).
2. Rust (rustup, toolchain `stable-x86_64-pc-windows-msvc`).
3. OpenCASCADE via vcpkg em `C:\dev\vcpkg`:

```bash
git clone https://github.com/microsoft/vcpkg.git C:/dev/vcpkg
```

```bash
C:/dev/vcpkg/bootstrap-vcpkg.bat -disableMetrics
```

```bash
C:/dev/vcpkg/vcpkg.exe install opencascade:x64-windows-release
```

Outro local: defina a variável `OCCT_ROOT` apontando para a pasta `installed/<triplet>`.

Rodar:

```bash
cargo run -p scc-app --release
```

Testes:

```bash
cargo test --workspace
```

## O que o protótipo cobre

| Área | Coberto | Requisitos |
|---|---|---|
| Árvore fixa: Origem (XY/XZ/YZ, eixos), Referências, Parâmetros, Coletores, pastas | ✅ | RF-ARV-001..046 (parcial) |
| Linha de tempo com marcador de rollback; inserir no marcador; mover comandos respeitando dependências | ✅ | RF-ARV-050..056 |
| Parâmetros com expressões, funções, detecção de ciclo, renomear reescrevendo expressões | ✅ | RF-PAR-001..006 |
| Propriedades: PN, descrição, material, tipo, personalizadas; massa por material | ✅ | RF-PAR-020..024 |
| Planos de referência como variáveis nomeadas; redirecionar o plano base | ✅ | RF-LNK-031, 035 (local) |
| Edição não destrutiva: tudo é editado no lugar; IDs estáveis | ✅ | RF-NUC-020 |
| Apagar com dependentes: substituir referência, apagar junto ou apagar só o item | ✅ | RF-NUC-021..024 |
| Comando com erro não é apagado e não invalida o resto | ✅ | RF-NUC-008, 024 |
| Sketch paramétrico: retângulo, círculo, polígono, rasgo, linhas encadeadas, pontos, construção | ✅ (por cotas, sem solver de restrições) | RF-ESB-002, 011, 013 |
| Extrusão / Recorte: cego, simétrico, dois sentidos, passante; adicionar/remover/interseção | ✅ | RF-SOL-001, 004 |
| Revolução em torno de eixo da Origem ou linha de sketch | ✅ | RF-SOL-003 |
| Furo e furo inteligente (M3–M20, passagem/roscado, simples/rebaixado/escareado), rosca cosmética | ✅ | RF-SOL-007, 008, 010 (cosmética) |
| Filete e chanfro (todas as arestas do coletor) | ✅ | RF-SOL-013, 014 (parcial) |
| Recompute em background; viewport só renderiza quando algo muda | ✅ | RNF-001 |
| Desfazer/refazer (Ctrl+Z / Ctrl+Y) | ✅ | RNF-013 |
| Exportar STEP e STL | ✅ | RF-NUC-010, 011 (exportação) |
| Projeto em pasta: PN automático, versões por hash, revisões A/B/C congeladas, estados, CSV | ✅ | RF-GPR-001, 002, 005, 006, 010..016 |
| Abrir revisão congelada somente leitura (edições são bloqueadas) | ✅ | RF-GPR-011, 014 |

## Limitações conhecidas (próximos passos)

- **Sem solver de restrições**: o sketch é dirigido por cotas/expressões. Próximo: avaliar PlaneGCS (LGPL).
- **Sem seleção na viewport** (faces/arestas): seleção é pela árvore e linha de tempo. Filete/chanfro
  aplicam a todas as arestas até existir seleção com referência persistente (ADR 0004).
- Ângulo de saída (draft), nervura por caminho e rosca modelada ainda não implementados.
- Recompute é completo (não incremental) — suficiente para peças pequenas; RNF-002 pendente.
- Tabela do furo inteligente é **preliminar** e deve migrar para a biblioteca pública (RF-BIB-014).
- Links entre documentos (geometria mestre) ainda não: só variáveis locais.
