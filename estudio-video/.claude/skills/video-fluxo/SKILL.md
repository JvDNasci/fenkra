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

## 1. Briefing (produtor), aprovado pela pessoa

Preencha `briefing.md`: objetivo, público, plataforma e formato, duração, chamada para ação, prazo,
material disponível e referências que o cliente gosta e não gosta. Sem objetivo e público, não
avance.

## 2. Pesquisa (pesquisador)

Em paralelo com a marca. Saída: de 3 a 5 fichas em `referencias/`, com o que adaptar.

## 3. Marca (marca)

Saída: `marca.md`. Cliente recorrente: só confira se mudou algo.

## 4. Roteiro (roteirista), aprovado pela pessoa

Saída: `roteiro.md` com três ganchos e blocos com tempo.

## 5. Storyboard e direção de arte (designer)

Saída: storyboard em `plano-edicao.md` e pedidos de IA e de gráficos.

## 6. Produção em paralelo

| Quem | O quê | Pasta |
|---|---|---|
| gerador-ia | B-roll, cenários e personagens no ArtCraft | `ia/` |
| motion | aberturas, títulos, legendas animadas e transições | `graficos/` |
| sonoplasta | trilha, efeitos e limpeza de voz | `audio/` |

## 7. Montagem (editor)

Rascunho em baixa (`video.sh proxy`), depois o corte final e as exportações por plataforma.

## 8. Mix final (sonoplasta) e QA (revisor)

O revisor escreve `qa.md`. Item reprovado volta ao especialista citado. Repita até aprovar.

## 9. Entrega (produtor), aprovada pela pessoa

`entregas/` com os arquivos por plataforma, a thumbnail, as legendas `.srt`, a sugestão de texto do
post e as hashtags. Inclua um resumo das licenças.

## Paralelismo

Rode juntos o 2 com o 3 e os três do 6. Não paralelize roteiro e storyboard: o storyboard depende
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
