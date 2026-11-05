---
name: fenkra-biblioteca
description: Diretrizes da bancada Biblioteca de normas do Fenkra. Use ao criar, corrigir ou pesquisar geração paramétrica de fixadores, engrenagens e dados normativos, ou ao tocar em crates/fenkra-solidos/src/biblioteca.rs.
---

# Biblioteca de normas

## Domínio

crates/fenkra-solidos/src/biblioteca.rs, hardware.rs, fixadores.rs, engrenagens.rs, eixos.rs; biblioteca/

## Antes de agir

1. Leia a seção da área em `docs/modulos/biblioteca-de-normas.md` e, se precisar de contexto, `docs/pesquisa/prendedores-industriais.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Biblioteca de normas\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de geração paramétrica de fixadores, engrenagens e dados normativos, busque primeiro documentação oficial do OpenCASCADE, normas,
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

Consulte montagem quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.
