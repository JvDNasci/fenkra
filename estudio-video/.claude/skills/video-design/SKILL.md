---
name: video-design
description: Direção visual de vídeo com storyboard, composição, tipografia em movimento, hierarquia na tela do celular, thumbnails e capas. Use ao transformar roteiro em planos, ao criar thumbnail e ao revisar a leitura visual de um corte.
---

# Design de vídeo

## Storyboard

No `plano-edicao.md`, um quadro por plano:

| # | Tempo | Enquadramento | Imagem (origem) | Texto na tela | Movimento | Transição |
|---|---|---|---|---|---|---|
| 1 | 0,0-1,2 | close no rosto | bruto `A001.mp4` 00:12 | "VOCÊ ERRA ISSO" | zoom 100→115% | corte seco + whoosh |

Origem possível: bruto do cliente, IA (pedido para o `gerador-ia`), gráfico (pedido para o
`motion`) ou estoque licenciado.

## Composição

- No vertical, o assunto fica no terço superior-central (y ≈ 500-900); a legenda, abaixo dele.
- Regra dos terços no horizontal; olhar com espaço na direção do olhar.
- Um ponto de foco por vez: se há texto grande, o fundo simplifica (desfoque ou escurecimento).
- Contraste: texto claro sobre fundo escuro ou caixa sólida; nunca texto fino sobre imagem movimentada.
- Escala: alterne planos aberto, médio e fechado; repetir o mesmo enquadramento cansa.

## Tipografia na tela

Até 6 palavras por bloco de texto no vertical. Hierarquia por tamanho e peso, não por cor apenas.
Texto entra junto com a fala (ou 2-3 quadros antes) e sai no corte.

## Thumbnail (YouTube) e capa (vertical)

- Um assunto, de preferência um rosto com emoção clara ou o objeto do resultado.
- No máximo 4 palavras, que complementam o título (não repetem).
- Três camadas: fundo simples, assunto recortado (remoção de fundo no ArtCraft ou `rembg`) e texto.
- Contraste alto e cor de destaque da marca.
- Faça de 2 a 3 variações para teste A/B e confira a leitura em 160×90 px.

Para montar thumbnails em código, use Python com Pillow ou um quadro estático do Remotion
(`npx remotion still`).
