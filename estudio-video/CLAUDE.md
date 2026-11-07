# Estúdio de vídeo

Kit de subagentes e skills para produzir vídeos para clientes de freelance e para canal próprio:
verticais (Reels, TikTok, Shorts) e horizontais (YouTube, web). Responda em português do Brasil.

Abra o Claude Code **dentro desta pasta**, ou copie-a para um repositório próprio. Só assim os
agentes de `.claude/agents` e as skills de `.claude/skills` são carregados.

## Como pedir

- "Novo projeto: vídeo vertical de 30 s para a padaria X, lançamento do pão de fermentação natural."
- "Transforme `projetos/cliente-y/brutos/live.mp4` em 5 cortes verticais com legenda animada."
- "Pesquise as trends de edição desta semana para Reels de moda."

Em qualquer pedido de produção, siga a skill `video-fluxo`. Ela define as etapas, quem faz cada
uma e quando parar para a aprovação do cliente.

## Onde roda cada coisa

| Tarefa | Onde | Por quê |
|---|---|---|
| Pesquisa, roteiro, marca, storyboard, prompts | Nuvem ou local | Só texto e web |
| Corte, legenda, mixagem e exportação (ffmpeg) | Local, pela pasta conectada | Os brutos são grandes e ficam no seu disco |
| Motion graphics com código (Remotion / Motion Canvas) | Local; nuvem para rascunho | Renderizar exige CPU/GPU |
| Geração com IA no ArtCraft | Local, no app ou pelo uso do computador | O ArtCraft é um app de desktop sem API nem MCP |
| Transcrição (faster-whisper) | Local, com GPU se houver | Fica rápido e de graça |

Resumo: a nuvem planeja e o seu computador produz. Veja `README.md`.

## Estrutura

```
projetos/<cliente>/<projeto>/
  briefing.md  roteiro.md  marca.md  plano-edicao.md  qa.md
  brutos/  ia/  audio/  graficos/  render/  entregas/
modelos/       modelos dos documentos acima
ferramentas/   video.sh (ffmpeg) e legendas.py (whisper → ASS)
referencias/   fichas de referência e trends, com fonte e data
```

## Regras

- Nada sai para o cliente sem passar pelo agente `revisor` (skill `video-qa`).
- Música, fontes, imagens e clipes só com licença clara: do cliente, livres de royalties, com
  licença paga ou geradas por nós. Anote a licença em `plano-edicao.md`.
- Referências e trends servem de estudo: reescreva a ideia, não copie o vídeo, o texto nem a arte
  de ninguém. Toda fonte consultada vai para `referencias/` com link e data.
- Não use a voz nem a imagem de pessoas reais sem autorização por escrito. Conteúdo de IA que pareça
  real leva o rótulo exigido pela plataforma.
- Specs de plataforma mudam: confira na documentação oficial a cada trimestre e atualize
  `video-formatos`.
- Mídia pesada não vai para o git (veja `.gitignore`).
