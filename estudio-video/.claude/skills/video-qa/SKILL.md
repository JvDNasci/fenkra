---
name: video-qa
description: Checklist de revisão antes de entregar ou publicar, cobrindo specs técnicas, áreas seguras, legendas e ortografia, loudness, marca, licenças, gancho e retenção. Use sempre que um vídeo estiver pronto para o cliente ou para postar.
---

# QA de entrega

## Checagens automáticas

```bash
ferramentas/video.sh info final.mp4                 # resolução, fps, codec, duração
ferramentas/video.sh medir final.mp4                # LUFS integrado e pico real
ferramentas/video.sh quadros final.mp4 render/qa    # folha de contato, 1 quadro a cada 2 s
ferramentas/video.sh grade final.mp4 render/qa      # quadros com a área segura sobreposta
```

Abra as imagens geradas e confira-as visualmente.

## Checklist (escreva em `qa.md`)

**Técnico**
- [ ] Resolução, proporção e fps conforme `video-formatos`
- [ ] H.264 `yuv420p`, `+faststart`, sem quadro preto ou congelado no começo e no fim
- [ ] Loudness dentro de ±1 LU do alvo; pico real ≤ -1 dBTP; sem estalo
- [ ] Áudio em sincronia (confira uma consoante explosiva, como "p", com a boca)

**Leitura**
- [ ] Nenhum texto ou rosto fora da área segura
- [ ] Legendas sem erro de português, nomes próprios corretos e sincronizadas
- [ ] Texto legível em celular (≥ 48 px de altura no vertical) e com contraste

**Narrativa**
- [ ] Gancho visual e textual antes de 1,5 s
- [ ] Nenhum trecho parado acima do limite de ritmo
- [ ] Chamada para ação clara ou final em loop
- [ ] Funciona sem som (a informação essencial está na tela)

**Marca e direitos**
- [ ] Cores, fontes, logo e tom conforme `marca.md`
- [ ] Toda música, fonte, imagem e clipe com licença registrada
- [ ] Conteúdo de IA rotulado quando exigido; nenhuma pessoa real sem autorização

**Entrega**
- [ ] Nomes de arquivo padronizados; thumbnail, `.srt`, texto do post e hashtags juntos

Cada item reprovado leva o tempo, o problema e o responsável.
