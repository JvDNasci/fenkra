---
name: fenkra-sketch
description: Diretrizes da bancada Sketch do Fenkra. Use ao criar, corrigir ou pesquisar solver de restrições, graus de liberdade, curvas e splines, ou ao tocar em crates/fenkra-core/src/sketch.rs.
---

# Sketch

Antes de criar ou mudar um comando, siga a skill `fenkra-comando` (contrato do ADR 0107).

## Domínio

crates/fenkra-core/src/sketch.rs, crates/fenkra-app/src/sketch_ui.rs, sketch_ext.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/sketch.md` e, se precisar de contexto, `docs/pesquisa/sketch.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Sketch\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de solver de restrições, graus de liberdade, curvas e splines, busque primeiro documentação oficial do OpenCASCADE, normas,
livros e artigos técnicos, e depois documentação pública de outros CADs. Guarde uma nota por assunto
na pasta de conhecimento, com: como funciona, fórmulas, casos de borda, sintomas de falha e como
depurar. Escreva com suas palavras, sem copiar texto ou imagem, e registre as fontes em
`docs/arquitetura/procedencia.md`. Pesquise só o que a tarefa pede: pesquisa tem custo.

## Diretrizes

Preencha com o que for aprendido. Cada regra leva o motivo.

- Origem e eixos H/V são entidades fixas do sketch (`SketchData.eixos`, ADR 0092). Ferramenta nova que
  apaga, move, apara ou lista "as linhas do sketch" filtra `e_eixo` / `e_da_origem`: senão ela age nos
  eixos ou pega a linha errada (eles entram no fim de `curves`).
- Posição do sketch por referência usa `LinkSrc` e é resolvida no recálculo (`sketch_posicao.rs`). Referência
  perdida volta para a automática com aviso, nunca erro: o sketch derrubaria a peça inteira.

## Como verificar

`cargo test -p <crate> <filtro>`, com documento montado em código conferindo status, volume e caixa
envolvente. Mudança visual: capture com as variáveis `FENKRA_DEMO*`.

## Vizinhos

Consulte solidos, superficies quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

- Pontas a 10 m (eixos) levadas à tela por `cam.project` somem na perspectiva (caem atrás da câmera). Projete
  pontos perto do meio (`perto_do_meio`) e estenda na tela.
- "O quadro criou algo" não é "o elemento terminou" (B572). O 1º clique da Linha já cria o ponto e, sobre
  uma curva, o vínculo dele; contar restrição nova como elemento fazia a linha solta voltar à seleção
  antes do 2º clique. Ferramenta de forma decide por `preso::criou_elemento` (só curva que não é ponto
  solto). Interação nova se testa pelo `overlay` de verdade com eventos crus, como em
  `sketch_primeiro_clique_testes.rs`, e não só pela função pura.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.
