---
name: fenkra-superficies
description: Diretrizes da bancada Superfícies e wireframe do Fenkra. Use ao criar, corrigir ou pesquisar NURBS, continuidade G0/G1/G2, costura, offset e preenchimento, ou ao tocar em crates/fenkra-solidos/src/geo3d.rs.
---

# Superfícies e wireframe

## Domínio

crates/fenkra-solidos/src/geo3d.rs; crates/fenkra-kernel/cpp/surfaces.inc, hybrid.inc; crates/fenkra-app/src/geo_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/superficies-e-wireframe.md` e, se precisar de contexto, `docs/pesquisa/superficies.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Superfícies e wireframe\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de NURBS, continuidade G0/G1/G2, costura, offset e preenchimento, busque primeiro documentação oficial do OpenCASCADE, normas,
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

Consulte solidos, kernel quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Superfície de varredura com perfil circular em spline 3D falha: use varredura sólida como contorno até corrigir (K-08).
