---
name: fenkra-captura
description: Como ver o Fenkra rodando sem mouse: variáveis FENKRA_DEMO, captura de janela e critério de item visual. Use ao conferir interface, fechar um item com parte visual ou tirar uma captura.
---

# Conferir na tela

Item com parte visual só fecha depois de visto em captura. Só o que depende de mouse real fica como não conferido, e isso se escreve no relatório.

## Como capturar

Rode o binário com `FENKRA_DEMO_FOTO=<arquivo.png>`: ele grava a janela e fecha sozinho, sem abrir nada na tela do usuário. Leia o PNG e olhe de verdade.

- Use `APPDATA=<pasta temporária>` para rodar sem o estado salvo do usuário. A paleta flutuante e a rolagem da tela inicial nas capturas vêm desse estado, não do código.
- A lista de variáveis está em `docs/desenvolvimento.md`: `FENKRA_DEMO_JANELA`, `_PREVIA`, `_ASM`, `_EST`, `_SKETCH`, `_PMI`, `_DOBRA`, `_FORMA`, `_CURSO`, `_DWG_*`, `_TEMA`, `_SIMULAR`.
- Falta um caso? Crie uma variável nova e documente. Teste que só compila não vale como conferência.
- Em build de debug, "Unaligned" em laranja é aviso do egui, não defeito.

## O que olhar

- Peça distinta do fundo, cores do tema, texto sem corte, nada sobreposto.
- Janela fora da barra de menus e dentro da tela.
- O fluxo do comando na ordem: suporte, modo, parâmetros, avançado.
- Com tema: rode com `config.json` salvo, não só com a variável de demo.

## Relatório

Diga o que viu, o que corrigiu e o que falta. Captura estática não prova animação nem arraste.
