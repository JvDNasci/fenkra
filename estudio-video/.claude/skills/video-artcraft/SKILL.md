---
name: video-artcraft
description: Como usar o ArtCraft (IDE livre de imagem e vídeo com IA, github.com/storytold/artcraft) e os modelos de geração para B-roll, cenários, personagens consistentes, poses 3D, imagem para vídeo, remoção de fundo e música, além de escrever prompts e controlar custo. Use ao precisar gerar qualquer imagem, vídeo, som ou 3D com IA para um projeto.
---

# ArtCraft e geração com IA

## O que é

O ArtCraft (MIT ou Apache-2.0) é um app de desktop para Windows, macOS e Linux que junta canvas 2D,
editor de cena 3D (posar personagem e posicionar câmera) e um catálogo de modelos de imagem, vídeo,
música, malha 3D e mundos. Ele dá controle de composição antes de gerar, o que é exatamente o que
falta num prompt de texto puro.

Instalação: baixe em getartcraft.com ou em Releases do GitHub. Compilar do código está em
`_docs/dev_setup.md` do repositório.

## Como o estúdio opera

O ArtCraft **não tem API, CLI nem MCP** (conferido em nov/2026 na página do repositório). Então:

1. **Planejar na sessão** (nuvem ou local): o agente escreve o prompt, a proporção, a duração, a
   referência de pose e câmera e o modelo sugerido em `ia/pedidos.md`.
2. **Gerar no computador**: a pessoa executa no ArtCraft ou, com o uso do computador ligado
   na sessão local, o agente opera o app. Saída salva em `projetos/.../ia/`.
3. **Registrar** em `ia/registro.md`: arquivo, modelo, prompt, semente, créditos gastos e
   observação de licença do modelo.

Se o repositório ganhar API ou MCP, troque o passo 2 por chamada direta e atualize esta skill.

## Qual recurso para qual plano

| Plano | Recurso do ArtCraft |
|---|---|
| Cenário ou B-roll inexistente | imagem por texto, depois imagem para vídeo |
| Mesmo personagem em vários planos | personagem posado na cena 3D + imagem de referência fixa |
| Ângulo de câmera exato | cena 3D: posicione a câmera e gere a partir dela |
| Trocar fundo ou objeto | inpainting no canvas |
| Recorte para thumbnail ou composição | remoção de fundo |
| Trilha ou efeito de som | modelos de música e som (confira a licença comercial) |

## Prompt de imagem e vídeo

Estrutura: **sujeito + ação + cenário + câmera (lente, plano, movimento) + luz + estilo + proporção
+ duração**. Exemplo:

> Padeira de avental bege tirando pão de fermentação natural do forno a lenha, vapor subindo; close
> em 50 mm, câmera fazendo dolly in lento; luz quente lateral da janela, fim de tarde; foto
> realista, granulação leve; 9:16; 5 s.

Para vídeo: um movimento de câmera e uma ação por geração. Planos de 3-6 s são os mais estáveis;
monte sequências cortando vários planos curtos.

## Consistência

Fixe a imagem de referência do personagem e a paleta; repita as mesmas palavras de estilo e lente em
todos os prompts do projeto (guarde-as em `ia/pedidos.md` como "bloco de estilo"); gere em lote e
escolha.

## Custo e cuidado

- Estime créditos antes e anote no orçamento. Gere em resolução baixa até aprovar o enquadramento.
- Confira os termos de uso comercial de cada modelo antes de entregar ao cliente.
- Não gere rostos, vozes ou marcas de pessoas e empresas reais sem autorização; não peça "no estilo
  de <artista vivo>".
- Marque como conteúdo gerado por IA quando a plataforma exigir (cenas realistas de pessoas e
  eventos).
