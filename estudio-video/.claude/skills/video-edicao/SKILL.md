---
name: video-edicao
description: Edição e montagem com ffmpeg e ferramentas livres, cobrindo organização de mídia, proxy, corte de silêncios, ritmo, reenquadramento 16:9 para 9:16, legendas queimadas, cortes de live e exportação por plataforma. Use ao cortar, montar, legendar, reenquadrar ou exportar qualquer vídeo.
---

# Edição

Ferramenta principal: `ferramentas/video.sh` (ffmpeg). Rode `ferramentas/video.sh ajuda` para ver
os comandos. Para uma linha do tempo visual, o projeto pode ser aberto no Kdenlive ou no Shotcut
(livres), mas o padrão do estúdio é tudo reproduzível por script.

## Ordem de trabalho

1. **Inventário**: `video.sh info brutos/*.mp4` (resolução, fps, duração e áudio). Anote no plano.
2. **Proxy** para revisar rápido: `video.sh proxy brutos/A001.mp4`.
3. **Transcrição**: `python ferramentas/legendas.py brutos/A001.mp4 --saida render/A001`
   gera `.json` com palavras e tempos, `.srt` e `.ass`. Use o texto para achar os melhores trechos
   (cortes de live: procure frases que funcionem sozinhas, com começo, conflito e conclusão).
4. **Corte bruto**: `python ferramentas/cortar_silencios.py entrada.mp4 saida.mp4 --limiar -35 --min 0,35`
   tira pausas. Revise: às vezes uma pausa é dramática e precisa ficar.
5. **Seleção**: `video.sh trecho entrada.mp4 00:01:12.4 00:01:41.0 saida.mp4` (recorte preciso, com
   reencode).
6. **Reenquadrar** para vertical: `video.sh vertical entrada.mp4 saida.mp4 desfoque|corte`.
   `desfoque` mantém o quadro inteiro sobre um fundo desfocado (bom para tela e slides);
   `corte` recorta o centro (bom para rosto centralizado). Rosto fora do centro: ajuste o `x` do
   recorte à mão ou faça o reenquadramento no Remotion com keyframes.
7. **Montar**: concatene os trechos com `video.sh juntar lista.txt saida.mp4` (arquivo no formato
   do concat demuxer: `file 'a.mp4'`, uma linha por trecho, todos com o mesmo codec e fps).
7b. **Cor**: `video.sh cor entrada.mp4 saida.mp4 <look> 0.7` (skill `video-cor`).
8. **Gráficos e B-roll**: sobreponha o que veio de `graficos/` e `ia/`
   (`video.sh sobrepor base.mp4 grafico.mov saida.mp4 <início_s>`).
9. **Legenda**: `video.sh legendar entrada.mp4 legenda.ass saida.mp4`.
10. **Áudio**: entregue ao sonoplasta; depois `video.sh loudness entrada.mp4 saida.mp4 -14`.
11. **Exportar**: `video.sh exportar entrada.mp4 vertical|youtube|feed|web saida.mp4`.

## Ritmo

- Vertical: plano médio de 1-3 s; nada parado por mais de 3 s sem movimento, texto ou zoom.
- YouTube: planos mais longos são aceitáveis, mas mude algo (B-roll, zoom, gráfico) a cada 5-10 s
  em fala contínua.
- Corte no movimento (o gesto começa num plano e termina no outro) esconde o corte.
- Corte J: o som do próximo plano entra antes da imagem. Corte L: o som do plano anterior continua.
  Use em conversas e transições de cena.
- Jump cut na mesma fala: alterne a escala (100% e 115%) para não parecer um tranco.

## Corte de live ou podcast em vários verticais

Transcreva, liste de 10 a 15 candidatos com tempo e motivo, escolha os 5 melhores com o produtor,
e para cada um: trecho, depois vertical, legenda animada, gancho de texto no primeiro segundo,
loudness e exportação. Nomeie `entregas/<projeto>-corte-01-<tema>.mp4`.

## Regras

- Nunca sobrescreva arquivos de `brutos/`.
- Intermediários em `render/`, com qualidade alta (CRF 16 ou ProRes) para não perder geração.
- Mantenha fps constante e igual ao da captação; não misture 25 e 30 sem conversão.
