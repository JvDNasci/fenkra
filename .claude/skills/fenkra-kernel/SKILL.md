---
name: fenkra-kernel
description: Diretrizes da bancada Kernel do Fenkra. Use ao criar, corrigir ou pesquisar B-Rep do OpenCASCADE, tolerâncias, topologia, ponte cxx e diagnóstico de falhas, ou ao tocar em crates/fenkra-kernel/ (lib.rs.
---

# Kernel

## Domínio

crates/fenkra-kernel/ (lib.rs, cpp/kernel.cpp, kernel.h)

## Antes de agir

1. Leia a seção da área em `docs/modulos/geral.md` e, se precisar de contexto, `docs/pesquisa/sintese.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Kernel\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de B-Rep do OpenCASCADE, tolerâncias, topologia, ponte cxx e diagnóstico de falhas, busque primeiro documentação oficial do OpenCASCADE, normas,
livros e artigos técnicos, e depois documentação pública de outros CADs. Guarde uma nota por assunto
na pasta de conhecimento, com: como funciona, fórmulas, casos de borda, sintomas de falha e como
depurar. Escreva com suas palavras, sem copiar texto ou imagem, e registre as fontes em
`docs/arquitetura/procedencia.md`. Pesquise só o que a tarefa pede: pesquisa tem custo.

## Diretrizes

Preencha com o que for aprendido. Cada regra leva o motivo.

## Como verificar

`cargo test -p <crate> <filtro>`, com documento montado em código conferindo status, volume e caixa
envolvente. Mudança visual: capture com as variáveis `FENKRA_DEMO*`.

## Vizinhos

Consulte todas as bancadas quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

- Booleana e malha em threads só sobre cópia profunda (`Shape::comuns`, `Shape::em_threads`, ADR 0081). A
  booleana não tem estado global, mas a mesma forma entra em vários pares e o comum pode devolver as faces da
  entrada: tesselar grava a malha nelas, e duas threads gravariam na mesma face. `Shape::clone` divide a topologia
  e não serve de cópia (B394).
- `posicionado` (montagem) copia a forma sem a malha: quem tessela a peça posicionada refaz a malha inteira a cada
  vez. Use a malha da vista quando ela basta, ou tessele em threads (B394).

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.
