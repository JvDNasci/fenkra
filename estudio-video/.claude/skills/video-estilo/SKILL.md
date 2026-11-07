---
name: video-estilo
description: Perfis de estilo de vídeo (charmoso, direto, cinematográfico, divertido, educativo, corporativo, luxo), níveis de efeitos (pouco, médio, muito), como casar efeito visual com efeito sonoro, e o senso comum dos vídeos atuais de redes sociais e do YouTube sobre o que é obrigatório, o que é diferencial e o que evitar. Use ao definir o estilo de um projeto, ao planejar efeitos e ao revisar se o vídeo está no padrão atual.
---

# Estilo, intensidade e senso comum

## 1. Perfis de estilo

Escolha um na entrevista. A pessoa pode misturar dois ("charmoso + educativo").

| Perfil | Abertura | Legenda (estilo em `legendas.py`) | Cor (look) | Trilha (`trilha.py`) | Efeitos |
|---|---|---|---|---|---|
| Charmoso | vinheta curta de 0,5-1 s com logo animado e brilho | `charme` ou `manuscrito` | `quente`, `pastel` | `charmoso` | pop, brilho, ding; transições suaves |
| Direto | sem vinheta: o gancho é a abertura | `impacto` | `natural-vivo` | `energetico` | hit, whoosh; cortes secos |
| Cinematográfico | plano aberto com título em fade | `limpo`, poucas palavras | `teal-laranja` | `epico` ou `emotivo` | boom, riser; speed ramp |
| Divertido | reação ou meme visual de 0,5 s | `divertido` | `natural-vivo` | `charmoso` ou `energetico` | pop, glitch, zoom exagerado |
| Educativo | promessa + "em 3 passos" | `limpo` ou `caixa` | `tech-limpo` | `calmo` ou `tecnologico` | clique, ding em cada item, setas |
| Corporativo | logo discreto, sem exageros | `caixa` | `tech-limpo` | `calmo` | swipe, transição-tom |
| Luxo | silêncio de 0,3 s, depois revelação lenta | `charme` | `vintage` leve ou `pb-contraste` | `emotivo` | brilho, boom suave; movimento lento |

**Introduções e chamadas para redes**: no vertical, nada de intro longa; o @ e o "siga" entram no
final (últimos 1,5 s) ou como selo discreto aos 60-70% do vídeo. No YouTube, a abertura de marca vem
**depois** do gancho (entre 10 e 30 s), com até 5 s, e a tela final dura de 5 a 20 s com o próximo
vídeo. As chamadas são animadas no motion com o ícone de cada rede **desenhado por nós** (forma
simples), nunca o logo oficial distorcido.

## 2. Nível de efeitos

Combine na entrevista e anote no briefing.

| Nível | Efeitos visuais a cada 10 s (vertical) | Efeitos sonoros | Intensidade da biblioteca | Quando |
|---|---|---|---|---|
| Pouco | 1-2 (zoom suave, texto) | só nas transições | `suave` | marca séria, depoimento, luxo |
| Médio | 3-4 | transições + textos-chave | `medio` | padrão da maioria dos canais |
| Muito | 6-8 (zoom, glitch, emoji, shake) | quase todo evento | `forte` + `medio` | humor, games, reação, público jovem |

No YouTube horizontal, divida por 3 (o público tolera planos mais longos).

## 3. Casar efeito visual com efeito sonoro

| Visual | Som da biblioteca (`biblioteca/sfx`) | Sincronia |
|---|---|---|
| Corte com movimento, whip pan | `transicao/whoosh` (duração = da transição) | começa 2 quadros antes |
| Zoom de impacto | `impacto/hit` | no quadro do zoom |
| Revelação, título, logo | `impacto/boom` ou `tensao/riser` → `boom` | riser termina no quadro da revelação |
| Texto ou ícone surgindo | `interface/pop` | no primeiro quadro do texto |
| Item de lista, acerto | `acento/ding` | no check |
| Antes e depois, brilho | `acento/brilho` | na troca |
| Erro, virada técnica | `acento/glitch` | junto do glitch visual (3-6 quadros) |
| Cursor, clique, interface | `interface/clique` | no clique |
| Troca de cena | `transicao/swipe` ou `transicao-tom` | no corte |

Regras: corte na batida (`audio/trilha.batidas.txt`) e impacto no COMPASSO; nunca dois impactos
fortes em menos de 0,5 s; abaixe a trilha 3-4 dB por 0,3 s sob um boom; silêncio antes da revelação
vale mais que mais um efeito.

## 4. Senso comum dos vídeos atuais (nov/2026)

**Obrigatório (todo mundo faz; sem isso o vídeo parece amador)**

- Gancho visual e textual antes de 1,5 s; nada de "oi, gente" ou logo parado no começo.
- Legenda queimada e legível (a maioria assiste sem som no vertical).
- Ritmo sem respirações: cortes de silêncio e jump cuts com troca de escala.
- Áudio limpo e no volume da plataforma (-14 LUFS); voz sempre acima da trilha.
- Texto e rosto dentro da área segura.
- Final que fecha: chamada clara ou loop.
- Exportação nativa por plataforma (sem marca d'água de outra rede).

**Diferencial (pouca gente faz bem; é aqui que o estúdio se destaca)**

- Design sonoro casado com a imagem (a tabela acima), não só "trilha de fundo".
- Paleta e look próprios por canal: o vídeo é reconhecido sem o logo.
- Variação tipográfica com intenção: palavra de ênfase em outra fonte (`charme`, `manuscrito`).
- Arco de emoção no roteiro, com a voz mudando de tom.
- B-roll e gráficos explicativos no lugar de "cabeça falando" o tempo todo.
- Laço aberto e recompensa, e um detalhe escondido que faz rever (aumenta a retenção).
- Trilha original: nenhuma reclamação de direitos e identidade sonora própria.

**Evitar (envelheceu ou irrita)**

- Intro longa, vinheta de mais de 1 s no vertical, "não esqueça de se inscrever" no começo.
- Efeito em tudo sem motivo; transições prontas chamativas a cada corte.
- Legenda com 8 ou mais palavras por vez, fonte fina, cor sem contraste.
- Música famosa sem licença (bloqueio, silenciamento ou perda de monetização).
- Voz robótica genérica sem direção; trilha mais alta que a fala.
- Clickbait que o vídeo não entrega (derruba retenção e confiança).
- Zoom digital que estoura a imagem e reenquadramento que corta cabeças.
