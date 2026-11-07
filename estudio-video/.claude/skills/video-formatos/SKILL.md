---
name: video-formatos
description: Especificações técnicas de cada plataforma (resolução, proporção, fps, codec, bitrate, loudness, duração, áreas seguras e thumbnail) para Reels, TikTok, Shorts, YouTube e web. Use ao planejar, compor, exportar ou revisar qualquer vídeo.
---

# Formatos por plataforma

Valores de trabalho do estúdio, revisados em nov/2026. As plataformas mudam limites com
frequência: antes de uma entrega importante, confira a central de criadores oficial e atualize esta
tabela com a data.

## Saídas padrão

| Saída | Resolução | Proporção | fps | Vídeo | Áudio |
|---|---|---|---|---|---|
| Vertical (Reels, TikTok, Shorts) | 1080×1920 | 9:16 | 30 (60 para jogo e esporte) | H.264 High, CRF 18, ~12-16 Mbps | AAC 320 kbps, 48 kHz |
| Feed Instagram | 1080×1350 | 4:5 | 30 | H.264 | AAC |
| YouTube 1080p | 1920×1080 | 16:9 | igual à captação (24, 30 ou 60) | H.264 High, ~12 Mbps (30 fps) ou ~18 Mbps (60 fps) | AAC 384 kbps, 48 kHz |
| YouTube 4K | 3840×2160 | 16:9 | igual à captação | H.264 ~45-68 Mbps ou HEVC | AAC 384 kbps |
| Web e site | 1920×1080 ou 1280×720 | 16:9 | 30 | H.264 + `+faststart`; versão WebM opcional | AAC 128-192 kbps |

Comum a todas: `yuv420p`, SDR Rec.709, `-movflags +faststart` e fps constante.

## Loudness

| Destino | Integrado | Pico real |
|---|---|---|
| YouTube, Reels, TikTok, Shorts | -14 LUFS | -1 dBTP |
| Podcast e web com voz | -16 LUFS | -1 dBTP |

## Áreas seguras no vertical (1080×1920)

A interface das plataformas cobre partes da tela. Deixe texto e rosto fora destas faixas:

- topo: 220 px (nome do perfil, abas);
- base: 420 px (legenda do post, música, barra de comentário);
- direita: 140 px (botões de curtir, comentar e compartilhar);
- esquerda: 60 px.

Área segura resultante: x de 60 a 940 e y de 220 a 1500. Para servir a todas as plataformas com um
arquivo só, use essa área. `ferramentas/video.sh grade` gera uma máscara para conferir.

## Duração (ponto de partida)

| Formato | Faixa que costuma render | Limite técnico (conferir) |
|---|---|---|
| Reels e TikTok | 7-15 s (loop), 30-60 s (conteúdo), até 90 s | vídeos longos permitidos; o algoritmo favorece retenção |
| Shorts | 15-60 s | até 3 min |
| YouTube | 8-15 min para conteúdo; capítulos a partir de 3 blocos | — |

## Thumbnail do YouTube

1280×720, JPG ou PNG, abaixo de 2 MB. Teste a leitura em 160×90 px: se não entender nesse tamanho,
refaça.

## Capa do vertical

Quadro de 1080×1920 com o título dentro da área central de 1080×1350, porque o grid do perfil
recorta em 4:5 ou 3:4.
