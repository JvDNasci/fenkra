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
