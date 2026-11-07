---
name: video-marca
description: Como montar e aplicar o guia de marca de um cliente em vídeo (paleta, fontes, logo, tom de voz, estilo de legenda, vinhetas e assinatura sonora). Use ao começar com um cliente novo e ao conferir se um vídeo está na marca.
---

# Marca em vídeo

## Coleta

Peça ao cliente ou levante: manual de marca, logo em vetor (SVG ou PDF), fontes, site, perfis e 3
vídeos dos quais ele gosta e 3 dos quais não gosta. Extraia as cores com `python ferramentas/paleta.py --imagem logo.png --saida graficos/marca`
(ou `--cor` com o hex que o cliente já usa).

## O que o guia precisa decidir

Preencha `modelos/guia-marca.md`:

- **Paleta**: primária, secundária, destaque e neutros, em hex. Teste o contraste do texto sobre o
  fundo: mínimo de 4,5:1 (WCAG AA) em legenda.
- **Tipografia**: título e corpo, com licença que permita uso em vídeo. Sem licença, use fontes
  livres (Google Fonts com OFL). Tamanho mínimo de 48 px de altura em 1080×1920.
- **Legenda**: fonte, peso, cor, cor de destaque da palavra falada, contorno ou caixa, posição (a
  partir de y = 1150 no vertical) e de 1 a 4 palavras por vez.
- **Logo**: posição, tamanho, quando aparece (marca d'água discreta ou só no final).
- **Vinheta**: abertura de até 1 s no vertical (ou nenhuma) e de até 5 s no YouTube; tela final.
- **Tom de voz**: 3 adjetivos, palavras que usa e palavras proibidas.
- **Assinatura sonora**: estilo de trilha, BPM típico, SFX recorrente.
- **Não fazer**: o que fere a marca (ex.: piadas com concorrentes, cores neon, gírias).

## Kit técnico

Gere `graficos/marca/` com: `paleta.json` (lido pelos efeitos do motion), logo em PNG com alfa
de 1080 px e 512 px, um ASS de estilo de legenda (`ferramentas/legendas.py --estilo`) e a vinheta
renderizada.

## Revisão de marca

Pergunta única: "se eu tirar o logo, ainda dá para saber de quem é?". Se não, faltam cor, fonte ou
tom consistentes.
