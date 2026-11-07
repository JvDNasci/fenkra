# Estúdio de vídeo: subagentes e skills

Equipe de agentes do Claude Code para produzir vídeos de freelance e de canal próprio: verticais
(Reels, TikTok, Shorts) e horizontais (YouTube e web).

## A equipe

| Agente | Papel | Skills |
|---|---|---|
| `produtor` | briefing, orçamento, orquestração, aprovações, entrega | video-fluxo, video-formatos |
| `pesquisador` | trends, referências do nicho, aulas e tutoriais | video-pesquisa |
| `roteirista` | ganchos, estrutura, falas, texto na tela, chamada para ação | video-roteiro |
| `marca` | guia de marca do cliente e revisão de identidade | video-marca |
| `designer` | storyboard, composição, tipografia, thumbnails | video-design |
| `editor` | corte, ritmo, reenquadramento, legenda, exportação | video-edicao |
| `motion` | efeitos, legendas animadas, transições, títulos | video-efeitos |
| `gerador-ia` | B-roll, cenários e personagens no ArtCraft | video-artcraft |
| `sonoplasta` | voz, trilha, SFX, ducking, loudness | video-audio |
| `revisor` | QA técnico, de marca, de direitos e de retenção | video-qa |
| `marketing` | posicionamento, título, capa, post, hashtags, testes, métricas e venda do serviço | video-divulgacao, video-vendas |
| `colorista` | paleta por marca ou canal, correção e looks de cor, contraste | video-cor |
| `musico` | trilha original, domínio público, arranjos, biblioteca de SFX e licenças | video-musica |

Skills transversais: `video-estilo` (perfis, nível de efeitos, casamento entre som e imagem, e o que
todo vídeo atual tem, o que é diferencial e o que evitar) e `video-fontes` (biblioteca de fontes).

O fluxo completo, com os pontos de aprovação, está em `.claude/skills/video-fluxo/SKILL.md`. Ele começa
por uma **mini entrevista** (`modelos/entrevista.md`) e passa por uma **prévia** (animatic + quadro de
estilo + teste de trilha) antes do render final.

## Onde cada coisa roda (decisão)

- **ArtCraft** é um app de desktop (Rust, MIT ou Apache-2.0) **sem API, CLI nem MCP**. Por isso
  ele roda no seu computador: você gera ou o agente opera o app pelo uso do computador. Os agentes
  escrevem os prompts e registram o que foi gerado.
- **Corte, legenda, áudio e render** rodam melhor no seu computador, onde estão os brutos, a GPU e
  o ArtCraft. Conecte a pasta do estúdio a uma sessão (Remote Control no app Claude para desktop)
  ou rode o Claude Code direto nela.
- **Pesquisa, roteiro, marca e storyboard** podem rodar em qualquer lugar, inclusive na nuvem.

## Instalação no seu computador

1. ffmpeg com libass (Windows: `winget install Gyan.FFmpeg`; macOS: `brew install ffmpeg`;
   Linux: `apt install ffmpeg`).
2. Python 3.10+ e `pip install faster-whisper` para transcrição e legendas.
3. Node 18+ para Remotion e Motion Canvas (criados por projeto com `npx`).
4. ArtCraft: getartcraft.com ou Releases de github.com/storytold/artcraft.
5. Opcionais livres: Blender, Kdenlive, Audacity.
6. Confira: `ferramentas/video.sh verificar`.

No Windows, rode os scripts pelo Git Bash ou pelo WSL.

## Primeiro projeto

```bash
cd estudio-video
claude
> Novo projeto: cliente "padaria-sol", 3 Reels de 30 s sobre o pão de fermentação natural.
```

O `produtor` faz o briefing e pede sua aprovação, e daí a equipe segue o fluxo.

## Bibliotecas (geradas por script, fora do git)

```bash
python ferramentas/baixar_fontes.py   # 26 fontes livres (OFL e Apache) por estilo, com licença
python ferramentas/gerar_sfx.py       # 90 efeitos sonoros originais: 15 tipos × 3 intensidades × 2 variações
python ferramentas/lut.py             # 9 looks de cor (.cube)
```

## Ferramentas

- `ferramentas/video.sh`: novo projeto, info, proxy, trecho, vertical (desfoque ou corte), juntar,
  sobrepor, legendar, medir, loudness, exportar por plataforma, folha de contato e grade de área
  segura.
- `ferramentas/legendas.py`: transcrição (faster-whisper), depois `.srt` e `.ass` com destaque
  palavra a palavra, nas cores de `paleta.json`.
- `ferramentas/cortar_silencios.py`: corte bruto que remove pausas.
- `ferramentas/trilha.py`: trilha original por clima (tecnológico, energético, charmoso, calmo, emotivo,
  épico), mais o mapa de batidas para cortar no ritmo.
- `ferramentas/paleta.py`: paleta da marca ou do canal (por cor ou extraída do logo), com contraste.
- `ferramentas/previa.py`: prévia (animatic) a partir de `planos.csv`.
- `video.sh tratar-voz`: limpa um áudio enviado (leve, padrão ou forte); `video.sh cor` e
  `comparar-cor`: looks de cor.
- `ferramentas/legendas.py --estilo`: limpo, impacto, podcast, caixa, charme, manuscrito, divertido e
  tech, com a palavra de ênfase em outra fonte.

## Exemplo pronto

`projetos/fenkra/cad-10s/`: vídeo vertical de 10 s sobre CAD paramétrico, feito de ponta a ponta com
o kit (roteiro, prévia, render 3D por código, trilha e SFX originais, cor, QA e divulgação). Para
refazer: `cd projetos/fenkra/cad-10s && ./produzir.sh`.

## Sugestão

Este kit está dentro do repositório do Fenkra só para nascer. Para usá-lo de verdade, copie a pasta
`estudio-video/` para um repositório próprio (por exemplo, `estudio-video`) e abra o Claude Code
nela.
