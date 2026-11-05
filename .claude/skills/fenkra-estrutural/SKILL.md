---
name: fenkra-estrutural
description: Diretrizes da bancada Estrutural e soldas do Fenkra. Use ao criar, corrigir ou pesquisar perfis, cortes entre membros, lista de corte e simbologia de solda, ou ao tocar em crates/fenkra-solidos/src/structural.rs.
---

# Estrutural e soldas

## Domínio

crates/fenkra-solidos/src/structural.rs; crates/fenkra-montagem/src/estrutura.rs; crates/fenkra-app/src/st_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/estrutural-e-soldas.md` e, se precisar de contexto, `docs/pesquisa/estrutural.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Estrutural e soldas\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de perfis, cortes entre membros, lista de corte e simbologia de solda, busque primeiro documentação oficial do OpenCASCADE, normas,
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

Consulte montagem, desenho quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.
