---
name: fenkra-desenho
description: Diretrizes da bancada Desenho técnico do Fenkra. Use ao criar, corrigir ou pesquisar projeção, remoção de linhas ocultas, cortes, cotas e normas de desenho, ou ao tocar em crates/fenkra-desenho/src/.
---

# Desenho técnico

## Domínio

crates/fenkra-desenho/src/; crates/fenkra-kernel/cpp/drawing.inc; crates/fenkra-app/src/dwg_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/desenho-tecnico.md` e, se precisar de contexto, `docs/pesquisa/desenho.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Desenho técnico\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de projeção, remoção de linhas ocultas, cortes, cotas e normas de desenho, busque primeiro documentação oficial do OpenCASCADE, normas,
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

Consulte pmi, montagem quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

- A HLR exata do OpenCASCADE (`HLRBRep_Algo`) guarda estado global: duas no mesmo processo, mesmo com cópia
  profunda da forma, devolvem vista vazia ou com linhas a menos, sem erro. Paralelize só em processos
  (`hlr_processos.rs`); o kernel serializa as chamadas no processo (`HLR_EXATA`). A projeção por malha
  (`HLRBRep_PolyAlgo`) roda bem em threads (B393, ADR 0080).

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Tabela e anotação ficam na área útil, longe do carimbo, com aviso quando não couber.
- Escala automática considera altura e largura contra a folha e cai em escala padronizada.
- Desenho de montagem precisa percorrer subconjuntos; vista vazia sempre gera aviso.
- Ok de desenho exige conferir cotas, furos na vista e tabela dentro da folha.
