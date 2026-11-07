---
name: video-fluxo
description: Fluxo completo de produção de vídeo do estúdio, do briefing à entrega, com os especialistas de cada etapa, os pontos de aprovação e o orçamento. Use sempre que pedirem um vídeo novo, cortes de um bruto, uma série para um canal ou um pacote para um cliente, mesmo sem falar em etapas.
---

# Fluxo de produção

## 0. Abrir o projeto

```bash
ferramentas/video.sh novo <cliente> <projeto>
```

Isso cria `projetos/<cliente>/<projeto>/` com as pastas e copia os modelos.

## 1. Mini entrevista e briefing (produtor), aprovado pela pessoa

Comece pela mini entrevista de `modelos/entrevista.md`: pergunte só o que faltou no pedido, sempre
com opções prontas (perfil de estilo, nível de efeitos, emoção principal, paleta, fontes, música,
chamadas para redes, referências). "Tanto faz" = o padrão indicado na entrevista.

Preencha `briefing.md`: objetivo, público, plataforma e formato, duração, chamada para ação, prazo,
material disponível e referências que o cliente gosta e não gosta. Sem objetivo e público, não
avance.

## 2. Pesquisa (pesquisador) e posicionamento (marketing)

Em paralelo com a marca. Saídas: de 3 a 5 fichas em `referencias/` (trends + fontes do tema, se a
pessoa não mandou referências) e as 5 perguntas de posicionamento em `divulgacao.md`.

## 3. Marca (marca) e paleta (colorista)

Saída: `marca.md` + `graficos/marca/paleta.json`. Cliente recorrente: só confira se mudou algo.

## 4. Roteiro (roteirista), aprovado pela pessoa

Saída: `roteiro.md` com três ganchos e blocos com tempo.

## 5. Storyboard e direção de arte (designer)

Saída: storyboard em `plano-edicao.md`, `planos.csv` (tempo, texto, cor, mídia e sfx por plano) e
pedidos de IA e de gráficos.

## 5b. Prévia (editor + músico + motion), aprovada pela pessoa

Antes de produzir de verdade, mostre:

1. **Animatic**: `python ferramentas/previa.py planos.csv render/previa.mp4 --trilha audio/trilha.wav`,
   com o ritmo, os textos e os efeitos sonoros nos tempos certos.
2. **Quadro de estilo**: um quadro com a legenda no estilo escolhido, o look de cor
   (`video.sh comparar-cor`) e a paleta.
3. **Teste de trilha**: 10 s da trilha candidata (ou duas opções).

Ajuste com a pessoa e só então siga.

## 6. Produção em paralelo

| Quem | O quê | Pasta |
|---|---|---|
| gerador-ia | B-roll, cenários e personagens no ArtCraft | `ia/` |
| motion | aberturas, títulos, legendas animadas, transições e chamadas para redes | `graficos/` |
| musico | trilha original ou em domínio público, mais SFX da biblioteca | `audio/` |
| sonoplasta | tratamento de voz | `audio/` |
| colorista | correção e look dos brutos | `render/` |

## 7. Montagem (editor)

Rascunho em baixa (`video.sh proxy`), depois o corte final e as exportações por plataforma.

## 8. Mix final (sonoplasta) e QA (revisor)

O revisor escreve `qa.md`. Item reprovado volta ao especialista citado. Repita até aprovar.

## 9. Entrega (produtor + marketing), aprovada pela pessoa

`entregas/` com os arquivos por plataforma, a thumbnail, as legendas `.srt` e o `divulgacao.md`
(texto do post, hashtags, comentário fixado, plano de publicação e de testes). Inclua um resumo das
licenças.

## 10. Resultado (marketing)

Com 24 h e com 7 dias, leia as métricas (skill `video-divulgacao`) e anote as lições em
`referencias/` para o próximo vídeo.

## Paralelismo

Rode juntos o 2 com o 3 e os do 6. Não paralelize roteiro e storyboard: o storyboard depende
do roteiro aprovado.

## Orçamento de referência por peça

Ajuste ao seu mercado; os valores são pontos de partida para conversar, não tabela.

| Pacote | Conteúdo | Rodadas de ajuste |
|---|---|---|
| Corte simples | 1 vertical de até 60 s, com bruto do cliente, legenda e trilha | 1 |
| Vertical completo | roteiro, B-roll de IA, motion e 3 ganchos testáveis | 2 |
| Pacote de cortes | de 5 a 10 verticais extraídos de um vídeo longo ou de uma live | 1 |
| YouTube | de 8 a 15 min com roteiro, motion, thumbnail e capítulos | 2 |

Antes de começar, estime e diga: horas de trabalho, créditos de IA (ArtCraft e modelos) e tempo de
render. Avise quando passar de 80% do combinado.

## Atalho para pedido pequeno

"Só corta e legenda": editor, depois sonoplasta (loudness) e revisor. Pule as outras etapas.
