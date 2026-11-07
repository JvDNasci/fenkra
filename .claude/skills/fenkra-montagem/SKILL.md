---
name: fenkra-montagem
description: Diretrizes da bancada Montagem do Fenkra. Use ao criar, corrigir ou pesquisar solver de restrições de montagem, juntas, cinemática, links e contexto, ou ao tocar em crates/fenkra-montagem/src/ (solve.rs.
---

# Montagem

## Domínio

crates/fenkra-montagem/src/ (solve.rs, kin.rs, bom.rs, fixadores.rs); crates/fenkra-app/src/asm_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/montagem.md` e, se precisar de contexto, `docs/pesquisa/montagem.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Montagem\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de solver de restrições de montagem, juntas, cinemática, links e contexto, busque primeiro documentação oficial do OpenCASCADE, normas,
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

Consulte biblioteca, plm, estrutural quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Só conta como montada a peça restrita à vizinha: fixar com posição gravada dá 0 graus de liberdade e não prova nada.
- Peças ligadas por acoplamento ficam no mesmo .fka: junta é local ao arquivo.
- Compare a lista de peças com a árvore do produto: ok não vê a peça que ficou de fora.
- A origem de Caminho é a roda menor: posicione a corrente pelo pinhão.
- Roda dentada só vale se o corte abrir na borda: confira o dente na imagem, não o volume.
