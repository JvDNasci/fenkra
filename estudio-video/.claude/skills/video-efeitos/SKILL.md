---
name: video-efeitos
description: Receitas de efeitos e motion graphics para trends atuais e técnicas duradouras, como legenda animada palavra a palavra, zoom de impacto, speed ramp, transições, texto cinético, barra de progresso, glitch, chroma key, máscara e telas de abertura e encerramento, com Remotion, Motion Canvas, ffmpeg e Blender. Use ao criar qualquer efeito, animação ou gráfico em movimento.
---

# Efeitos e motion

## Escolha da ferramenta

| Necessidade | Ferramenta | Observação |
|---|---|---|
| Modelo reutilizável com dados (legendas, títulos, listas, gráficos) | **Remotion** (React) | Gratuito para pessoa física e empresa de até 3 pessoas; acima disso exige licença |
| Animação vetorial explicativa | **Motion Canvas** (TypeScript, MIT) | Ótimo para diagramas e tipografia |
| Filtro rápido sobre vídeo (zoom, cor, glitch, chroma, desfoque) | **ffmpeg** | Sem dependências |
| 3D, partículas, câmera | **Blender** (GPL) | Render em lote por linha de comando |
| Plano gerado por IA (cenário, imagem para vídeo) | **ArtCraft** | Veja `video-artcraft` |

Cores e fontes sempre vêm de `graficos/marca/paleta.json`.

## Projeto Remotion do estúdio

Na primeira vez, crie com `npx create-video@latest graficos/remotion` (modelo em branco, TypeScript).
Cada efeito vira uma composição com props (`texto`, `cor`, `duracao`). Render:

```bash
npx remotion render <Composicao> render/titulo.mov --codec=prores --prores-profile=4444 --props='{"texto":"..."}'
npx remotion still <Composicao> render/thumb.png --frame=30
```

## Receitas

**Legenda palavra a palavra com destaque** (a mais pedida no vertical): gere o `.ass` com
`ferramentas/legendas.py` (palavra falada em cor de destaque, de 1 a 4 palavras por vez). Para pop
de escala por palavra, faça no Remotion a partir do `.json` de palavras:
`scale = spring({frame: frame - inicioDaPalavra, fps, config: {damping: 12}})`, de 0,8 a 1,0.

**Zoom de impacto (punch-in)**: no Remotion, `interpolate(frame, [f0, f0+4], [1, 1.15])` na
`transform: scale()` do vídeo. No ffmpeg, para um trecho já recortado:
`-vf "scale=iw*1.15:-2,crop=1080:1920"` (corte seco com zoom). Alterne nos jump cuts.

**Zoom lento (Ken Burns)** em imagem parada: `ffmpeg -loop 1 -i foto.png -vf "zoompan=z='min(zoom+0.0008,1.12)':d=150:s=1080x1920:fps=30" -t 5 saida.mp4`.

**Speed ramp**: divida em três trechos (normal, rápido e normal), acelere o do meio com
`setpts=0.4*PTS` (e `atempo=2.5` no áudio ou silencie e coloque whoosh), e junte. Fica mais natural
com um riser no sonoplasta.

**Transições de tendência**: whip pan (desfoque direcional de 4-6 quadros + whoosh), zoom
através de objeto (casar a escala de dois planos), corte casado de forma ou movimento, máscara pelo
objeto que passa na frente da câmera. Em código: no Remotion, `@remotion/transitions` (slide, wipe,
fade) e transições próprias com `interpolate`.

**Texto cinético**: palavras entrando com `spring` escalonado (atraso de 2-3 quadros entre palavras),
peso alto, até 3 palavras por tela.

**Barra de progresso**: retângulo cuja largura é `frame / durationInFrames` do vídeo; no topo da
área segura.

**Glitch e RGB split**: `ffmpeg -vf "rgbashift=rh=8:bh=-8,noise=alls=20:allf=t"` por 3-6 quadros
na virada.

**Chroma key**: `-filter_complex "[1:v]chromakey=0x00ff00:0.15:0.05[fg];[0:v][fg]overlay"`. Fundo
real removido: use a remoção de fundo do ArtCraft ou `rembg` quadro a quadro.

**Desfoque de fundo atrás do texto**: `boxblur=20` no fundo e caixa semitransparente da cor da marca.

**Abertura e encerramento**: abertura no vertical com no máximo 1 s (ou nenhuma: o gancho é a
abertura). Tela final no YouTube com 5-20 s e espaço para os elementos finais da plataforma.

## Boas práticas

- Movimento com curva (ease ou spring), nunca linear, exceto em barra de progresso.
- Duração típica de entrada e saída: 6-10 quadros a 30 fps.
- Um efeito de impacto a cada 3-5 s no vertical; mais que isso vira ruído.
- Exporte com alfa (ProRes 4444 ou WebM VP9 com `yuva420p`) para o editor compor.
