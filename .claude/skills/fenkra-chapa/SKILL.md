---
name: fenkra-chapa
description: Diretrizes da bancada Chapa metálica do Fenkra. Use ao criar, corrigir ou pesquisar fator K, tolerância de dobra, planificação, alívios e mapa de dobras, ou ao tocar em crates/fenkra-kernel/cpp/sheet.inc.
---

# Chapa metálica

## Domínio

crates/fenkra-kernel/cpp/sheet.inc; trechos de chapa em crates/fenkra-solidos/src/lib.rs e crates/fenkra-app/src/feat_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/chapa-metalica.md` e, se precisar de contexto, `docs/pesquisa/chapa.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Chapa metálica\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de fator K, tolerância de dobra, planificação, alívios e mapa de dobras, busque primeiro documentação oficial do OpenCASCADE, normas,
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

Consulte solidos, desenho quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Para mostrar planificação, o 3D deve estar na chapa plana: tabela de dobras certa no desenho não vale como captura.
- Depois de abrir o painel, confira que ele aparece na captura antes de dizer que foi visto.
