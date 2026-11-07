---
name: video-musica
description: Música e efeitos sonoros sem direitos de terceiros, cobrindo composição original por clima, domínio público (regra brasileira dos 70 anos, obra e gravação separadas), música clássica e brasileira antiga, arranjos e remixes de obras em domínio público, bibliotecas livres confiáveis, biblioteca de SFX do estúdio e registro de licenças. Use ao escolher, compor ou conferir qualquer música ou efeito sonoro.
---

# Música e efeitos sem direitos de terceiros

## Atenção à regra (não são 50 anos)

No Brasil (Lei 9.610/98), a **obra** (composição e letra) entra em domínio público **70 anos
contados de 1º de janeiro do ano seguinte à morte do autor** (art. 41). Em parceria, conta a partir
da morte do **último** coautor vivo (art. 42). A **gravação** (fonograma) tem proteção própria de
70 anos a partir de 1º de janeiro do ano seguinte à fixação (art. 96), e intérpretes e gravadoras
têm direitos conexos.

Na prática:

- Em 2026 estão em domínio público no Brasil as obras de autores mortos até 1955.
- **Música em domínio público não quer dizer gravação livre.** Uma gravação moderna de Bach tem
  dono. Use partitura em domínio público com interpretação nossa, ou gravação marcada como domínio
  público ou CC0.
- Arranjo, edição moderna da partitura e letra traduzida podem ter direitos próprios.
- Outros países têm prazos diferentes (o vídeo é visto no mundo todo). Os sistemas de detecção das
  plataformas podem reclamar mesmo de obra livre: guarde a prova (fonte, licença e data) para
  contestar.
- Na dúvida, não use. Para cliente, prefira trilha original.

## Ordem de preferência

1. **Trilha original** com `ferramentas/trilha.py --clima <clima> --duracao <s>`. É inédita, sem risco
   e gera `.batidas.txt`. Para algo mais rico, componha no LMMS ou no Ardour (livres) ou gere em
   modelo de música com termos comerciais claros (veja `video-artcraft`).
2. **Arranjo ou remix próprio de obra em domínio público**: pegue a partitura ou o MIDI em domínio
   público (IMSLP, Mutopia Project; confira a licença de cada arquivo), mude andamento, instrumentos
   e ritmo, e renderize com FluidSynth e um soundfont livre (por exemplo FluidR3_GM, licença MIT) ou
   no LMMS. O resultado é uma gravação nossa.
3. **Gravação livre**: Musopen (filtre domínio público), Free Music Archive (só CC0 ou CC BY; evite NC
   e ND em trabalho de cliente), Incompetech (CC BY 4.0, com crédito obrigatório) e Pixabay Music
   (licença própria; algumas faixas estão em sistemas de detecção, então guarde o comprovante).
   Biblioteca de áudio da própria plataforma: só dentro dela.

## Catálogo inicial de obras em domínio público no Brasil

Confira sempre a data de morte de **todos** os autores (música e letra) antes de usar.

| Autor | Morte | Gêneros e obras para pesquisar |
|---|---|---|
| Chiquinha Gonzaga | 1935 | maxixe, polca, choro ("Ó Abre Alas", "Atraente") |
| Ernesto Nazareth | 1934 | tango brasileiro e choro ("Brejeiro", "Odeon") |
| Zequinha de Abreu | 1935 | choro ("Tico-Tico no Fubá", só a melodia; letras posteriores podem ter outro autor) |
| Noel Rosa | 1937 | samba (confira os parceiros de cada samba) |
| Anacleto de Medeiros | 1907 | choro e banda ("Iara") |
| Joaquim Callado | 1880 | choro ("Flor Amorosa") |
| Carlos Gomes | 1896 | ópera ("O Guarani", abertura) |
| Alberto Nepomuceno | 1920 | erudito brasileiro |
| Clássicos europeus | ... | Bach, Vivaldi, Mozart, Beethoven, Chopin, Tchaikovsky, Debussy (1918), Satie (1925), Ravel (1937) |

**Ainda protegidos no Brasil** (exemplos de armadilhas): Villa-Lobos (morte em 1959, livre só em
2030), Pixinguinha, Ary Barroso, Dorival Caymmi.

## Efeitos sonoros

- **Biblioteca do estúdio**: `python ferramentas/gerar_sfx.py`, com 15 efeitos em 3 intensidades e
  variações (whoosh, whoosh reverso, swipe, riser, hit, boom, queda grave, pop, clique, obturador,
  digitação, ding, brilho, glitch, transição de tom). É original, de uso livre, e o catálogo fica em
  `biblioteca/sfx/catalogo.csv`.
- Para ampliar: adicione uma função em `gerar_sfx.py`, ou grave e sintetize você mesmo.
- Fontes externas: Freesound **filtrado por CC0**; pacotes livres de royalties com uso comercial
  explícito (guarde a licença). Evite bibliotecas "só uso não comercial" em trabalho de cliente.
- Processar o efeito (altura, reverso, reverb, mistura de camadas) dá amplitude: um whoosh com
  `asetrate` vira três.

## Registro

Em `plano-edicao.md`, para cada faixa: título, autores, data de morte, fonte da partitura ou da
gravação, licença, link e data da consulta.
