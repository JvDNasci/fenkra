---
name: video-fontes
description: Biblioteca de fontes livres do estúdio (OFL e Apache) por estilo (impacto, limpo, divertido, charme, manuscrito, tech, neon), com combinações de fontes, variação de legenda e regras de leitura no celular. Use ao escolher tipografia de legenda, título, thumbnail ou vinheta.
---

# Fontes

```bash
python ferramentas/baixar_fontes.py      # baixa 26 fontes livres + licenças para biblioteca/fontes
```

O catálogo, com o nome interno de cada família (o que vai no `.ass` e no Remotion), fica em
`biblioteca/fontes/catalogo.md`. Todas permitem uso comercial em vídeo.

## Combinações que funcionam (base + ênfase)

| Perfil | Base (legenda) | Ênfase (palavra-chave) | Título e thumbnail |
|---|---|---|---|
| Charmoso | Poppins SemiBold | DM Serif Display itálico | Shrikhand ou Abril Fatface |
| Direto | Anton (maiúsculas) | Anton na cor de destaque | Anton ou Bebas Neue |
| Divertido | Lilita One | Bangers | Bangers |
| Educativo | Poppins ExtraBold | Poppins Black na cor de destaque | Archivo Black |
| Tech e CAD | Space Mono | Space Mono na cor de destaque | Rubik Mono One ou Space Grotesk |
| Luxo | Poppins SemiBold | Playfair Display itálico | Playfair Display |
| Pessoal e bastidores | Poppins ExtraBold | Permanent Marker ou Caveat | Pacifico |

Use `legendas.py --estilo <estilo>`, que já traz essas combinações, ou `--fonte "Nome"` para trocar a
base.

## Variar sem bagunçar

- No máximo duas famílias por vídeo (base + ênfase), e uma terceira só no título ou thumbnail.
- Varie por bloco do roteiro: mesma base, mas a ênfase muda de cor ou de fonte quando a emoção muda.
- Em série de vídeos, alterne o estilo de ênfase entre episódios e mantenha a base da marca.
- Manuscritas e neon (Great Vibes, Monoton) só em 1-3 palavras grandes, nunca em legenda corrida.

## Leitura no celular

Altura mínima de 48 px em 1080×1920 (o padrão do `legendas.py` é 78 px); contorno de 5-8 px ou caixa
sólida; até 4 palavras por vez; maiúsculas só em fontes condensadas (Anton, Bebas Neue).

Fontes variáveis (Inter, Montserrat, Oswald, Space Grotesk, Syne, Caveat e Playfair) ficam no peso
padrão no `.ass`; para escolher o peso, use-as no Remotion ou no Pillow.
