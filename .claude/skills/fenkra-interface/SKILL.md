---
name: fenkra-interface
description: Diretrizes da bancada Interface do Fenkra. Use ao criar, corrigir ou pesquisar ícones, tooltips, temas, navegação 3D, seleção e renderização wgpu, ou ao tocar em crates/fenkra-app/src/ (toolbar.rs.
---

# Interface

## Domínio

crates/fenkra-app/src/ (toolbar.rs, theme.rs, icons.rs, i18n.rs, viewcube.rs, gizmo.rs, keymap.rs, render.rs, picking.rs); assets/

## Antes de agir

1. Leia a seção da área em `docs/modulos/navegacao.md` e, se precisar de contexto, `docs/pesquisa/interface-e-arvore.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Interface\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de ícones, tooltips, temas, navegação 3D, seleção e renderização wgpu, busque primeiro documentação oficial do OpenCASCADE, normas,
livros e artigos técnicos, e depois documentação pública de outros CADs. Guarde uma nota por assunto
na pasta de conhecimento, com: como funciona, fórmulas, casos de borda, sintomas de falha e como
depurar. Escreva com suas palavras, sem copiar texto ou imagem, e registre as fontes em
`docs/arquitetura/procedencia.md`. Pesquise só o que a tarefa pede: pesquisa tem custo.

## Diretrizes

Preencha com o que for aprendido. Cada regra leva o motivo.

- **Idioma por inteiro.** Todo texto novo de interface entra em `i18n.rs` e é conferido nos dois
  idiomas, olhando a tela inteira e não só o campo novo. O nome de cada recurso nasce no idioma da
  interface ativo no momento da criação, e o documento guarda esse idioma: se a pessoa trocar de
  idioma depois, os nomes já criados ficam como estão. Motivo: na alpha.37 os recursos nasciam em
  português com o menu em inglês, e vice-versa (18/11/2026).
- **Personalização é do produto.** Barras se arrastam, flutuam, encaixam em qualquer borda, fecham no
  "x" e se restauram. Todos os painéis mudam de tamanho. Nenhuma mudança de layout pode remover isso
  sem o ok do usuário. Motivo: o B433 fixou as barras e o usuário sentiu como a maior perda da
  alpha.37. Referência: `docs/benchmark/detalhe/interface-referencia-2026-11.md`.
- **Seleção igual em toda parte.** A seleção vinda da árvore some com clique no vazio, com Esc e ao
  selecionar outra coisa, igual à do 3D. Motivo: bug relatado mais de 10 vezes.
- **Comando pelo elemento que resulta.** Há um só Ponto, Linha, Plano, Curva e Superfície, com os
  tipos dentro do painel. Operações (interseção, projeção, extrair) ficam à parte.

## Como verificar

`cargo test -p <crate> <filtro>`, com documento montado em código conferindo status, volume e caixa
envolvente. Mudança visual: capture com as variáveis `FENKRA_DEMO*`.

## Vizinhos

Consulte todas as bancadas quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Antes de capturar, enquadre a peça e esconda origem e rótulos de curva.
- Teste toda cor nova também com preto e branco contra o fundo do tema.
- Texto na árvore que não seja nome do usuário precisa de nome legível ou sair da tela.
- Acabamento da peça não chega na montagem (K-25): confira a aparência na peça aberta.
