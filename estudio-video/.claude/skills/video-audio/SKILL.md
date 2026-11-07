---
name: video-audio
description: Som para vídeo, abrangendo limpeza de voz, trilha, efeitos sonoros (whoosh, hit, riser, pop), ducking, mixagem e loudness por plataforma com ffmpeg e ferramentas livres. Use ao tratar áudio, escolher trilha, colocar SFX ou normalizar a saída.
---

# Áudio

## Ordem

1. **Voz**: limpeza, depois equalização, depois compressão.
2. **Trilha** no BPM do corte.
3. **SFX** em cada transição e impacto.
4. **Ducking** da trilha sob a fala.
5. **Loudness** para a plataforma.

## Voz com ffmpeg

```bash
ffmpeg -i voz.wav -af "highpass=f=80,afftdn=nf=-25,acompressor=threshold=-18dB:ratio=3:attack=5:release=80,equalizer=f=3000:t=q:w=1:g=2" voz_limpa.wav
```

Ruído forte: reduza com `arnndn` (modelo RNNoise) ou trate no Audacity (livre). Não exagere: voz
metálica é pior que um chiado leve.

## Ducking

```bash
ffmpeg -i voz.wav -i trilha.wav -filter_complex "[1:a][0:a]sidechaincompress=threshold=0.03:ratio=8:attack=20:release=300[duck];[duck][0:a]amix=inputs=2:duration=first:normalize=0" mix.wav
```

Trilha sob fala: de -18 a -24 dB abaixo da voz. Sem fala: pode subir.

## SFX

Whoosh nas transições, hit ou boom no zoom de impacto, pop em texto que aparece, riser antes da
virada e silêncio de 0,3 s antes da revelação (o silêncio também é efeito). Posicione 1-2 quadros
antes da imagem.

## Loudness

`ferramentas/video.sh loudness entrada.mp4 saida.mp4 -14` (duas passagens de `loudnorm`, pico real
de -1 dBTP). Valores por destino na skill `video-formatos`.

## Licenças

Use somente: biblioteca de áudio da própria plataforma (dentro dela), bibliotecas livres de
royalties com licença comercial (guarde o comprovante), Creative Commons compatível (CC0 ou CC BY
com crédito), música gerada com termos comerciais claros ou trilha encomendada. Registre cada uma
em `plano-edicao.md`. Trend de áudio da plataforma não pode ir para o arquivo exportado do cliente:
o cliente aplica o som dentro do app na hora de postar.
