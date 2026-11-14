# Teste com mouse e teclado reais, rodada 2 (B451), 14/11/2026

Rodada interrompida por pausa pedida pelo usuário às 21h, antes do primeiro caso. A trava do mouse
(`scratchpad\mouse.lock`) estava com outra sessão durante todo o tempo, e o release do app no
target próprio (`C:/cargo-target/o4_m2`) não chegou ao fim. Nenhum caso foi executado. Nenhum
`fenkra.exe` foi aberto por esta sessão.

Equipamento previsto: `scripts/teste_interface/fk.py` com `FK_ESTADO`, `APPDATA` e `LOCALAPPDATA`
próprios em pasta temporária. Relatório da rodada 1: [teste-mouse-2026-11-13.md](teste-mouse-2026-11-13.md).

## Resumo

| # | Caso | Resultado |
|---|---|---|
| 1 | Desenho: esboço na folha (B352) | não feito |
| 2 | Desenho: balões automáticos (B149) | não feito |
| 3 | Desenho: Cancelar na barra de progresso das vistas de montagem (B427) | não feito |
| 4 | Montagem: Reancorar restrição perdida | não feito |
| 5 | Biblioteca: inserir em furo | não feito |
| 6 | Biblioteca: inserir em ponto | não feito |
| 7 | B446: arrastar na árvore em todas as pastas | não feito |
| 8 | B448: nota com várias setas, duplo clique na seta, menu Mais | não feito |
| 9 | B449: setas na árvore depois do clique e redesenho imediato | não feito |
| 10 | B450: inserir da biblioteca sem arquivo pede salvar; aviso legível; voltar da peça sem rearmar | não feito |
| 11 | Layout B433: coluna esquerda e seta da árvore | não feito |
| 12 | Layout B436: barras por tipo e seta de variantes | não feito |

Nenhuma falha registrada, então nenhuma captura em `capturas-mouse/` e nenhum item de backlog novo.

## Preparação para a retomada

- Os roteiros `arvore_soltura_b446.json`, `arvore_setas_b449.json`, `menu_mais_fecha.json`,
  `pmi_nota_varias_setas.json` e `pmi_nota_enter_e_seta.json` foram escritos antes do layout novo
  (B433 a B436). As coordenadas da árvore e do menu "Mais" (1629, 65) devem ter mudado: a árvore agora
  fica na coluna da esquerda (`app/coluna.rs`) e as barras de recursos foram para a direita. Refazer as
  coordenadas por captura antes de rodar.
- Biblioteca em furo: o exemplo "Placa paramétrica" tem "Furos M6". Salvar a peça, criar montagem,
  inserir a peça, salvar a montagem (B450 exige arquivo) e então Biblioteca › Furo.
- Reancorar: criar restrição sobre uma face da peça, abrir a peça, apagar ou mudar o recurso dessa face,
  voltar; o painel da restrição mostra "Reancorar na seleção atual" (`asm_ui.rs`).
- Cancelar na barra: a barra só aparece enquanto a projeção das vistas de montagem roda em segundo
  plano (`dwg_ui/fundo_ui.rs`). Com montagem pequena ela some rápido; usar várias instâncias.
