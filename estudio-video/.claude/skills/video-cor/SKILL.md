---
name: video-cor
description: Cor em vídeo, cobrindo paleta por marca ou canal (60-30-10, contraste WCAG), correção de cor dos brutos, igualar câmeras, looks de gradação por LUT, cor por emoção e clareza da peça sobre o fundo. Use ao criar paleta, ao corrigir ou dar look a um vídeo e ao revisar o contraste de textos.
---

# Cor

## Paleta por marca ou canal

```bash
python ferramentas/paleta.py --cor "#1E6FFF" --clima tecnologico --saida projetos/<cliente>/graficos/marca
python ferramentas/paleta.py --imagem logo.png --clima charmoso --saida ...
```

Gera `paleta.json` (lido por `legendas.py`, `previa.py` e pelos efeitos), `paleta.png` e o
relatório de contraste. O destaque é ajustado sozinho até ter 4,5:1 sobre o fundo.

- **60-30-10**: 60% fundo e neutros, 30% secundária, 10% destaque (só no que deve ser lido primeiro:
  a palavra falada, o preço, a seta).
- Uma cor de destaque por vídeo; duas no máximo.
- Canal próprio: escolha uma paleta "assinatura" e mantenha por meses; é ela que faz o canal ser
  reconhecido.

## Cor por emoção

| Emoção do bloco | Look (`biblioteca/luts`) | Ajuste |
|---|---|---|
| Alegria, acolhimento, comida | `quente` | saturação +, sombras levemente quentes |
| Tecnologia, precisão, clareza | `tech-limpo` ou `frio` | brancos limpos, azul nas sombras |
| Drama, cinema | `teal-laranja` | contraste +, pele quente sobre fundo frio |
| Nostalgia, história | `vintage` | pretos levantados, menos saturação |
| Leveza, beleza, infantil | `pastel` | contraste −, saturação − |
| Seriedade, depoimento forte | `pb-contraste` | |
| Noite, games, música eletrônica | `noturno-neon` | |
| Produto real, natureza | `natural-vivo` | |

## Fluxo

1. **Correção** (antes do look), com ffmpeg: exposição `eq=brightness=0.03:contrast=1.05`, balanço de
   branco `colortemperature=temperature=5600` ou `colorbalance`, e igualar câmeras ajustando uma pela
   outra num quadro com o mesmo objeto.
2. **Comparar looks**: `video.sh comparar-cor bruto.mp4 render/cor` monta um quadro com todos.
3. **Aplicar**: `video.sh cor entrada.mp4 saida.mp4 <look> 0.7` (força de 0 a 1; pele natural
   costuma pedir de 0,5 a 0,8).
4. **Conferir**: pele não pode ficar verde nem laranja; texto com contraste; a peça e o rosto
   destacados do fundo (se não, escureça e desfoque o fundo, ou use vinheta leve com `vignette`).

LUT de terceiros: só com licença de uso comercial. Os 9 looks do estúdio são nossos (`lut.py`).
