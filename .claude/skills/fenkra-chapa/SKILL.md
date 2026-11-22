---
name: fenkra-chapa
description: Diretrizes da bancada Chapa metálica do Fenkra. Use ao criar, corrigir ou pesquisar fator K, tolerância de dobra, planificação, alívios e mapa de dobras, ou ao tocar em crates/fenkra-kernel/cpp/sheet.inc.
---

# Chapa metálica

Antes de criar ou mudar um comando, siga a skill `fenkra-comando` (contrato do ADR 0107).

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

- Dobra por linha nasce sem linha (`a == b`): o recálculo trata isso como "esperando a face fixa" e o clique na face põe a linha no meio dela (`chapa_linha::linha_no_meio`). Valor padrão com `a != b` faz a dobra sair na ponta da peça (B469).
- Comando de chapa na lista `previa_leve::so_no_fim` mostra só a caixa da zona durante a edição. Só entra nela o que é caro de construir; a dobra cônica saiu (B470) e a parede base também (sem corpo com o painel aberto, a pessoa achava que o comando não fez nada).
- Aba, bainha, laço e aba curva nascem com a aresta em (0,0,0) quando nada estava selecionado. Esse ponto é "a escolher" (`aresta_a_escolher`): nunca resolva a aresta mais perto da origem, que dava erro ou aba na aresta errada.
- Teste de chapa que só monta o documento pronto não pega bug de interface. Use `chapa_fluxo_tests.rs`: `Cache::leve` com o comando aberto, aresta chegando depois, ponto em `f32` e chave promovida gravada.
- No reconhecimento de espessura, faces paralelas só contam se estão de costas (normais opostas, material entre elas): a fenda de um rasgo também tem faces que se enfrentam, com ar no meio.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Para mostrar planificação, o 3D deve estar na chapa plana: tabela de dobras certa no desenho não vale como captura.
- Depois de abrir o painel, confira que ele aparece na captura antes de dizer que foi visto.
