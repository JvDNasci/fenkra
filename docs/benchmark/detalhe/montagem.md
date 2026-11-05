# Benchmark de detalhe — bancada Montagem

> Texto próprio, a partir de ajudas públicas de softwares de modelagem CAD avançados e do FreeCAD. Complementa
> [montagem.md](../montagem.md) e [pesquisa/montagem.md](../../pesquisa/montagem.md), que tratam do que existe
> e do modelo de dados. Aqui o assunto é o gesto: o que a pessoa vê e faz em cada comando.
> Os caminhos abaixo são relativos a `crates/fenkra-app/src/`, salvo indicação. As avaliações vêm da leitura do
> código e dos ícones; o aplicativo não foi executado para este levantamento.

## Como a bancada funciona hoje

- **Duas barras.** Montagem (`toolbar.rs:220-264`) e Cinemática (`toolbar.rs:397-414`).
- **Restrição e junta são "dois cliques e pronto".** O comando guarda `pending` ou `pending_joint`
  (`asm_ui.rs:499`, `asm_ui.rs:554`). No segundo clique a restrição já nasce e o solver move a peça
  (`asm_ui.rs:847-868`, `asm_ui.rs:693-723`). Não existe etapa de prévia nem de confirmação.
- **Editar vem depois.** O painel só abre com o item criado (`asm_ui.rs:1943-2006`); Cancelar volta ao estado
  guardado (`asm_ui.rs:2059-2067`). Esse mecanismo de cópia e volta já serve de base para uma prévia real.
- **O 3D mostra pouco.** Na viewport, a montagem desenha a seleção, o realce sob o cursor e o manipulador
  (`app.rs:5812-5836`). Não há símbolo de restrição ou junta, seta de normal, rótulo de valor nem indicação de
  grau de liberdade. Os GDL aparecem na árvore e na barra de status (`asm_ui.rs:1139-1144`, `app.rs:5372-5377`).
- **Cinco comandos são janelas flutuantes:** Simular, Exibição, Lista de peças, Fixadores e Engrenagens
  (`app.rs:6769-6773`). Só Simular e Exibição mudam o que se vê na viewport enquanto a janela está aberta.

Defeitos pequenos que explicam a sensação de aspereza, achados nesta leitura:

| Onde | O que acontece |
|---|---|
| `asm_ui.rs:1636` | O aviso da restrição pendente imprime o nome interno do enum, em inglês ("Coincident"). |
| `asm_ui.rs:1634` | Só a restrição tem aviso de etapa; junta pendente não tem. A instrução é um aviso que some. |
| `app.rs:6553` | Esc limpa a seleção, mas o comando pendente continua vivo. |
| `asm_ui.rs:74-75` | Distância nasce com 0 e Ângulo com 90, ignorando a posição real: a peça salta. |
| `asm_ui.rs:1986` | O valor é um texto livre, sem unidade nem validação, ao contrário de `editor::expr`. |
| `asm_ui.rs:2340` | A restrição rápida abre em posição fixa no topo, longe do que foi clicado. |
| `asm_ui.rs:2365` | Componente fixo fica sem manipulador, sem explicação. |
| `asm_ui.rs:2045`, `app.rs:6691` | A lista de interferências mora no painel, e o painel à esquerda (padrão) só aparece em edição. O aviso diz "veja no painel", que pode nem estar na tela. |
| `fenkra-montagem/src/kin.rs` | Não há limite de junta no código. `docs/modulos/montagem.md` diz "limites e drives", o que está desatualizado. |

## Resumo: as 5 maiores lacunas de experiência

1. **Sem prévia e sem confirmar.** A restrição acontece no segundo clique. Nos CADs avançados a peça se move como
   prévia, a pessoa inverte o lado, ajusta o valor e só então aceita. No Fenkra o erro só é visto depois que ele
   já foi aplicado, e o conserto está em outro lugar.
2. **O 3D não fala.** Não há seta de normal, símbolo de junta nem cota no modelo. Para saber o que prende uma peça
   é preciso ler a árvore. Nos outros, o símbolo fica no modelo e abre a edição com dois cliques.
3. **O que se escolhe é uma face inteira, e não um ponto de encaixe.** O clique vira plano, eixo ou ponto sem mostrar
   o que foi entendido (`asm_ui.rs:331`). Os melhores CADs mostram pontos candidatos ao passar o mouse, com o
   sistema de eixos visível, e a junta nasce com origem, alinhamento e deslocamento claros.
4. **Junta sem movimento na criação.** Não há botão de animar, limites nem rótulo de eixo. Quem cria uma dobradiça só
   descobre se ela gira indo a outra janela, depois de ligar um comando e calcular quadros (`asm_ui.rs:2528`).
5. **Comandos que nascem com valor fixo, sem olhar para a peça.** Repetição nasce em X, 3 cópias, 20 mm
   (`asm_ui.rs:543`) e pede o vetor em números (`asm_ui.rs:1930`). Furo nasce com Ø6 (`asm_ui.rs:652`). Inserir cai
   20 mm ao lado (`asm_ui.rs:437`). Falta a regra "aponte no 3D em vez de digitar".

## Checklist por comando

Critérios iguais aos das outras bancadas: **Intuitivo** (dá para usar sem ler nada); **Prévia 3D** (fantasma do
resultado antes de confirmar; "parcial" = o resultado já aplicado recalcula ao vivo); **Manipulação 3D** (seta,
arco ou alça arrastável com valor); **Ícone claro** (mostra o resultado e se distingue dos vizinhos, avaliado em
tamanho reduzido); **Tooltip** ("parcial" = só texto; nenhum comando tem imagem, `icons.rs:403`); **Dica por campo**;
**Fluxo** (clique no 3D preenche a etapa certa e o painel conduz). Valores: ok, parcial, falta. O traço (—) marca
critério que não se aplica ao comando.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Nova | ok | — | — | ok | parcial | — | ok | Aviso cita um emoji como botão (`asm_ui.rs:396`) |
| Abrir | ok | — | — | ok | parcial | — | ok | Sem miniatura do conjunto |
| Salvar | ok | — | — | ok | parcial | — | ok | Item B56 cobre o Salvar tudo |
| Inserir peça | parcial | falta | falta | ok | parcial | — | parcial | Diálogo do sistema; cai ao lado, sem seguir o cursor |
| Subconjunto | parcial | falta | falta | ok | parcial | — | parcial | Igual a Inserir peça |
| Peça nova | parcial | falta | falta | ok | parcial | — | parcial | Sai da montagem para a bancada Sólidos (B54) |
| Subconjunto da seleção | parcial | falta | — | parcial | parcial | — | parcial | Restrições externas somem, com aviso (B73) |
| Recorte pela peça | falta | parcial | falta | parcial | parcial | parcial | falta | Exige árvore e 3D antes de clicar no comando |
| Furo entre peças | parcial | parcial | falta | ok | parcial | parcial | falta | Ø6 fixo; posição só pelo clique (B57) |
| Copiar geometria | falta | falta | — | parcial | parcial | — | falta | Só face plana; seleção antes do comando |
| Fixadores | parcial | falta | falta | ok | parcial | parcial | parcial | Não mostra quais furos serão ocupados |
| Engrenagens | parcial | falta | falta | ok | parcial | parcial | parcial | Cálculo ótimo, mas sem imagem do par |
| Grupo | ok | — | — | ok | parcial | — | ok | |
| Substituir | parcial | falta | — | ok | parcial | — | parcial | Sem tabela do que casou (B55) |
| Repetição | falta | parcial | falta | ok | parcial | parcial | falta | X, 3, 20 fixos; vetor digitado |
| Numerar | ok | — | — | parcial | parcial | — | ok | Não mostra os números no 3D |
| Lista de peças | parcial | falta | — | ok | parcial | parcial | parcial | Linha não realça a peça; três colunas |
| Automática | parcial | falta | falta | parcial | parcial | — | parcial | Não diz qual tipo vai escolher |
| Coincidência | parcial | parcial | falta | ok | parcial | falta | parcial | "invertida" sem dica nem seta |
| Contato | parcial | parcial | falta | ok | parcial | — | parcial | Sem inverter |
| Distância | falta | parcial | falta | ok | parcial | parcial | falta | Nasce em 0; lado por valor negativo |
| Ângulo | falta | parcial | falta | ok | parcial | parcial | parcial | Nasce em 90; sem setor nem arco (B71) |
| Paralelo | parcial | parcial | falta | ok | parcial | — | parcial | |
| Perpendicular | parcial | parcial | falta | ok | parcial | — | parcial | |
| Engrenagem (restrição) | parcial | falta | falta | ok | parcial | ok | parcial | Razão −1 fixa; não lê os dentes |
| Polia | parcial | falta | falta | parcial | parcial | ok | parcial | Sem desenho da correia |
| Cremalheira | parcial | falta | falta | ok | parcial | ok | parcial | Valor inicial 31,4159 sem relação com a peça |
| Fixar | ok | — | — | ok | parcial | — | parcial | Manipulador some sem explicação |
| Fixação conjunta | parcial | falta | — | parcial | parcial | — | parcial | Sem marca no 3D de quem ficou preso |
| Alinhar | parcial | parcial | falta | parcial | parcial | — | parcial | Sem desfazer guiado se não convergir (B71) |
| Atualizar | ok | — | — | ok | parcial | — | ok | |
| Interferências | parcial | falta | — | ok | parcial | — | falta | Só texto; sem realce nem opções |
| Restrição rápida | parcial | falta | falta | — | parcial | — | parcial | Lista longa em posição fixa; ícones são caracteres |
| Mover e girar (manipulador) | parcial | ok | parcial | — | falta | parcial | parcial | Só eixos do mundo; sem valor digitado |
| Junta de revolução | parcial | falta | parcial | parcial | parcial | parcial | parcial | Sem animar, limites nem eixo no 3D |
| Junta prismática | parcial | falta | parcial | ok | parcial | parcial | parcial | Idem |
| Junta cilíndrica | parcial | falta | parcial | ok | parcial | parcial | parcial | Idem |
| Junta de parafuso | parcial | falta | parcial | ok | parcial | parcial | parcial | Passo inicial 2 mm, sem ler a rosca |
| Junta esférica | parcial | falta | falta | parcial | parcial | parcial | parcial | Comando só por lei |
| Junta planar | parcial | falta | falta | parcial | parcial | parcial | parcial | Idem |
| Junta rígida | parcial | falta | — | parcial | parcial | — | parcial | Parece repetir Fixação conjunta |
| Ponto em curva | falta | falta | falta | ok | parcial | falta | falta | A ordem importa e só o erro avisa |
| Came | falta | falta | falta | ok | parcial | parcial | falta | Tooltip explica a ordem; 3D não |
| Do conjunto | parcial | falta | — | falta | parcial | — | parcial | Sem lista do que será convertido |
| Exibição (estados) | parcial | ok | — | falta | parcial | parcial | parcial | Três assuntos em uma janela |
| Explosão | parcial | ok | parcial | falta | parcial | parcial | falta | Sem passos nem linhas |
| Animação | parcial | ok | — | falta | parcial | parcial | parcial | Quadro novo sempre vai para o fim +1 s |
| Simular | falta | parcial | falta | parcial | parcial | parcial | falta | Exige calcular quadros; gráfico sem eixos |

## Por comando: o que os outros têm e como deveria ser

Padrão do painel, igual ao das outras bancadas: **suporte** (o que foi escolhido), **modo** (o tipo), **parâmetros**
(valor), **avançado**. Alças, rótulos e campos seguem os itens da bancada Interface (contrato de manipulação 3D,
campo numérico único e esteira de GIF), e por isso não se repetem aqui.

### Restrições posicionais (Coincidência, Contato, Distância, Ângulo, Paralelo, Perpendicular)

**O que os outros têm e o Fenkra não:**

- Prévia ligada por padrão: ao escolher o segundo elemento a peça vai para a posição, e nada está gravado ainda.
- Valor inicial medido: uma opção preenche o deslocamento com a distância atual, para quem já está perto.
- Setas de normal na própria face. Dois modos claros, encostado ou nivelado, decididos pelas normais e trocados com um
  botão quando a prévia sai errada.
- Duas inversões diferentes: **inverter alinhamento** (o sentido das normais) e **inverter cota** (o lado da distância).
  O Fenkra junta tudo em uma caixa "invertida" e no truque do valor negativo.
- Barrinha flutuante perto do cursor, com tipo, valor, inverter e aceitar, para não ir ao painel.
- Inverter direto pela árvore, sem abrir a edição.
- Campos de seleção visíveis, com a chance de trocar uma das duas referências sem refazer o comando.

**Como deveria ser no Fenkra:**

- *Suporte:* duas caixas, "Elemento 1" e "Elemento 2", cor A e cor B, com o tipo reconhecido escrito ("Eixo de
  Suporte.fkp"). Clique avança sozinho; clique na caixa volta a ela.
- *Modo:* botões com desenho: Contato, Coincidência, Distância, Ângulo, Paralelo, Perpendicular. Trocar o modo
  atualiza a prévia, sem repetir a seleção.
- *Parâmetros:* valor com unidade, começando na medida atual; **Inverter alinhamento** e **Inverter lado**.
- *Avançado:* nome, suprimir, usar parâmetro.
- *No 3D:* a peça móvel vira fantasma na posição final, com a original esmaecida. Setas nas normais, a de A em uma cor
  e a de B em outra. Para distância, cota arrastável entre as duas faces; para ângulo, arco arrastável.
  O status diz "faltam 2 GDL" antes de aceitar. Se o solver não converge, a prévia fica vermelha e aponta a restrição
  em conflito, e nada muda na montagem.
- *Teclas:* Enter aceita e continua no comando; Esc cancela de verdade; Tab inverte.
- *GIF (8 s):* duas peças. Clica na face de uma, seta aparece; clica na da outra, fantasma encosta; aperta Tab e o
  fantasma inverte; digita 5 e abre a folga; Enter.

### Restrição automática e restrição rápida

**Os outros:** arrastar uma peça sobre outra sugere o tipo pelo cursor; uma tecla inverte; uma tecla modificadora
liga e desliga esse modo. A escolha do tipo vem da geometria (cilindro com cilindro, face com face).

**Fenkra:** Automática decide em silêncio (`asm_ui.rs:375`). A rápida lista até nove botões iguais, com a mesma ordem
de importância, incluindo Engrenagem para quaisquer dois eixos (`asm_ui.rs:101`).

**Deveria ser:** barrinha ao lado do segundo clique, com a sugestão em destaque e já em prévia, e no máximo três
alternativas à vista; o resto em "mais". Passar o mouse sobre uma opção troca a prévia. *GIF (6 s):* clica em dois furos;
a barrinha surge junto do cursor com Coincidência destacada; passar em Distância muda a prévia; clique aceita.

### Mover e girar (manipulador)

**Os outros:** manipulador com escolha de referência (mundo, peça, face); colisão com escopo (todos ou selecionados) e
parada ao encostar; modo físico, em que uma peça empurra a outra; folga dinâmica medida durante o arraste; valor
digitado para deslocamento e giro.

**Fenkra:** bom começo. Setas, quadrados e anéis com valor durante o arraste e passo (`gizmo.rs:164-221`), arrasto
respeitando restrições (`asm_ui.rs:2424-2430`) e parada por colisão (`asm_ui.rs:2432`). Faltam eixos da peça, pivô
escolhido, valor digitado e giro por número: há só botões de 90° (`asm_ui.rs:1759-1767`). O centro é a média dos vértices
(`asm_ui.rs:2370-2376`), o que tira o manipulador do centro em peças com malha desigual. As caixas de opção ficam em um
painel que nem sempre está visível.

**Deveria ser:** barrinha junto do manipulador: mundo/peça/face, pivô, passo, colisão, respeitar restrições. Peça presa
mostra um cadeado com o motivo ("fixa" ou "2 restrições"); peça com GDL mostra só as alças que podem mover. *GIF (6 s):*
peça com junta de revolução; só o anel aparece; arrasta e ela gira; uma peça encosta, ela para com o aviso.

### Inserir peça e subconjunto

**Os outros:** lista com miniatura e pasta recente; a peça segue o cursor, clique posiciona e repete para mais uma cópia;
a primeira peça alinha a origem da montagem. Também há versão com restrição já na entrada e encaixe por interfaces.

**Fenkra:** diálogo de arquivos do sistema, sem miniatura; a peça vai para `x + 20` (`asm_ui.rs:437`). O texto do tooltip
é fiel: a primeira entra fixa (`fenkra-montagem/src/lib.rs:541`), mas o 3D não mostra isso.

**Deveria ser:** painel de recentes com miniatura (o `PreviewCache` já existe, `app.rs:141`); depois de escolher, a peça
fantasma acompanha o cursor; Esc encerra. Primeira peça com selo "fixa na origem". *GIF (6 s):* escolhe a miniatura,
fantasma segue o mouse, dois cliques colocam duas cópias, Esc encerra.

### Juntas (nove tipos)

**O que os outros têm e o Fenkra não:**

- Tipo automático pela origem: círculos dão rotação, pontos no cilindro dão cilíndrica, ponto em esfera dá esférica.
- Origem com modos: em um ponto; **entre duas faces**, centrada; na interseção de duas arestas.
- Pontos candidatos ao passar o mouse, com sistema de eixos visível, e tecla para travar um ponto.
- Alinhamento na criação: ângulo, deslocamento em X, Y e Z, inverter e girar o eixo secundário de 90 em 90 graus.
- **Animar:** um botão faz a peça andar no 3D, ainda na criação. Uma opção inclui limites e outros movimentos.
- Limites Mínimo, Máximo e Repouso, com caixa que liga cada um sem perder o valor; marca na árvore de quem tem limite.
- Símbolo da junta no modelo; dois cliques nele abrem a edição.

**Fenkra:** cria com dois cliques e sem deslocamento (`asm_ui.rs:807-821`). No painel há um cursor do comando, o melhor
detalhe da bancada, de ±180° e ±200 mm (`asm_ui.rs:1855`, `asm_ui.rs:1868`), mas ele só aparece depois de ligar a caixa,
com faixa fixa. O campo de texto e o cursor são repetidos sem unidade. Não há limites no modelo de dados.

**Deveria ser:**

- *Suporte:* origem na peça A e na peça B, com "modo de origem".
- *Modo:* tipo com desenho do movimento; "Automático" primeiro.
- *Parâmetros:* ângulo inicial e deslocamentos; Inverter; passo (parafuso); rolete (came).
- *Avançado:* limites com Mínimo, Atual e Máximo; comando; acoplar a outra junta.
- *No 3D:* sistema de eixos nas duas origens, com seta curva ou reta mostrando o que se move. Botão **Animar** faz a
  peça ir e voltar. Limites como arco ou trilho com marcas arrastáveis. Depois de pronta, fica um símbolo pequeno.
- *GIF revolução (7 s):* passa o mouse no furo, pontos aparecem; clica; clica no pino; a seta curva surge; Animar e a
  tampa abre e fecha; arrasta a marca para 110°.
- *GIF came (9 s):* clica no perfil e ele acende; clica no rolete; o seguidor encosta; Animar e o seguidor sobe e desce.

Ponto em curva e Came pedem ordem certa, mas o comando só avisa no erro (`asm_ui.rs:825`, `asm_ui.rs:831`). Duas caixas com
rótulos "Trajetória" e "Seguidor" resolvem isso sem texto extra, e aceitam clique em qualquer ordem pelo tipo do elemento.

### Engrenagem, polia e cremalheira (restrições) e janela Engrenagens

**Os outros:** a razão vem dos diâmetros ou dentes escolhidos, com sentido invertível; os geradores mostram o par no 3D.

**Fenkra:** a janela Engrenagens é ótima no cálculo (`asm_ui.rs:2860`), mas sem desenho do par nem dos dados. A restrição nasce
com −1, 1 ou 31,4159 (`asm_ui.rs:69-71`). A dica por campo é a melhor da bancada ("−z1/z2", `asm_ui.rs:1992`).

**Deveria ser:** ler o raio das faces cilíndricas para sugerir a razão; seta curva em cada eixo mostrando o sentido; cursor
que gira o par. Janela com desenho 2D do engrenamento e fantasma do par. *GIF (6 s):* clica nos dois eixos; razão aparece
sozinha; arrasta a seta e as duas giram.

### Repetição

**Os outros:** direção por aresta ou cota, com Inverter; segunda direção com opção de copiar só a semente; marcas em
cada posição para **pular** uma cópia; repetição guiada por um padrão de furos já existente, que respeita o que o padrão
pulou.

**Fenkra:** nasce pronta em X, com vetor numérico.

**Deveria ser:** *Suporte:* componentes e direção clicando em aresta, eixo ou face. *Modo:* linear, circular, **pelos furos**.
*Parâmetros:* quantidade, espaçamento e Inverter. *Avançado:* segunda direção, pular. *No 3D:* fantasmas das cópias,
seta arrastável para espaçamento, marca clicável em cada posição. *GIF (7 s):* escolhe o parafuso; clica na aresta; fantasmas
surgem; arrasta a seta; clica numa marca e a cópia some.

### Fixadores

**Os outros:** ordem guiada: primeiro a posição, depois o fixador. Modos por furo, concêntrico, por ponto e linear. O
plano inicial define o sentido. A pilha é montada com linhas "clique para adicionar". Na prévia, uma **alça 3D estica o
comprimento e só para nos comprimentos existentes**; dois cliques abrem a lista. Opção de seguir o padrão.

**Fenkra:** janela com regra global e aplicação direta. O resultado é correto, mas a pessoa não vê antes o que vai entrar.

**Deveria ser:** antes de preencher, marcar no 3D cada furo reconhecido: verde (entra), amarelo (já tem), cinza (sem tamanho).
Clicar em uma marca exclui o furo. Fantasma da pilha no furo sob o cursor. *GIF (8 s):* abre, marcas acendem; desmarca um
furo; passa o mouse e a pilha aparece; aplica. Depende de B58 e B51.

### Interferências

**Os outros:** escolha do que verificar, lista de resultados com volume, ordenar por tamanho; selecionar um item deixa as
peças transparentes e mostra o volume em vermelho; exibição das demais (arame, oculto, transparente); **ignorar** e "ignorar
menores que"; coincidência como interferência, ou não; subconjunto como peça única; ocultar peça durante a análise.

**Fenkra:** só texto, tolerância fixa de 0,01 (`asm_ui.rs:681`), sem clique e sem volume no 3D.

**Deveria ser:** painel próprio com lista ordenada, clique realça e enquadra, volume comum em vermelho no 3D, ignorar com
motivo salvo na montagem. *GIF (6 s):* roda; três itens; clica no primeiro, câmera vai até lá, o resto fica transparente
e o volume aparece.

### Explosão e animação (Exibição)

**Os outros:** lista de **passos** reordenável arrastando, com marca de passo editado; passo radial; espaçamento automático ao
arrastar um grupo; arrastar a seta do manipulador cria o passo; linhas de explosão automáticas, ligadas ao centro ou à origem
da peça; suprimir passo.

**Fenkra:** cursor de afastamento global e ajuste à mão (`asm_ui.rs:2378-2385`, `asm_ui.rs:3177`). Offset solto, sem passo e sem
linha. O ajuste à mão usa só eixos do mundo. Quadro-chave novo vai sempre para o fim (`asm_ui.rs:3251`), e a linha do tempo não
é arrastável.

**Deveria ser:** separar em Estados, Explosão e Animação. Passos com seta no 3D e linhas pontilhadas automáticas. *GIF (7 s):*
seleciona o parafuso, puxa a seta, passo 1 aparece e a linha pontilhada também; escolhe as arruelas e puxa com espaçamento.

### Simular

**Os outros:** posições salvas por valor da restrição, com tempo de transição e ordem; cursor para percorrer; exportar vídeo;
gráficos com eixos e unidades.

**Fenkra:** janela funcional e leis em `t`, com uma barreira: quadros precisam ser calculados, e o gráfico não traz eixos
(`asm_ui.rs:2559-2566`). Quando faltam comandos, o aviso é só uma frase (`asm_ui.rs:2489`).

**Deveria ser:** lista de juntas com cursor direto (sem calcular) e, em seguida, tempo. Trajetória de um ponto no 3D. Item
B121 cobre o estudo. *GIF (7 s):* arrasta o cursor da manivela e o mecanismo anda; clica em Tocar; trajetória é desenhada.

### Lista de peças

**Os outros:** cinco tipos de estrutura (normal, fantasma, referência, comprada, inseparável); colunas configuráveis com
propriedades da peça; vista só de peças que ignora a estrutura.

**Fenkra:** três visões e número fixo, o que é um diferencial (`asm_ui.rs:2644-2690`), porém com três colunas e sem ligação com o
3D.

**Deveria ser:** passar o mouse na linha realça a peça; clicar seleciona. Colunas PN, material e massa, quando existirem (B92, B25).

### Outros

- **Recorte pela peça / Copiar geometria:** hoje exigem selecionar antes de clicar no comando, e o erro explica a ordem
  (`asm_ui.rs:631-638`, `asm_ui.rs:659-669`). Devem abrir painel com caixas "Peça que corta" e "Peças a recortar", aceitando seleção
  depois do comando. Fantasma do material removido em vermelho.
- **Furo entre peças:** seta no ponto de início, com a direção e o diâmetro em alça (B57).
- **Substituir:** tabela de restrições que casaram ou não, com realce (B55).
- **Fixar e Fixação conjunta:** marca no 3D (cadeado, e elo ligando as peças do grupo).
- **Numerar:** balão temporário com o número sobre cada peça ao usar o comando.

## Ícones e tooltips

- Paralelo, Perpendicular, Contato, Distância e Coincidência comunicam bem. Revolução, Planar, Rígida, Do conjunto e
  Exibição são os mais fracos: não mostram a peça nem o movimento.
- Rígida e Fixação conjunta fazem o mesmo para quem lê; o tooltip precisa dizer a diferença, ou um dos dois sai.
- Dois comandos com nome "Engrenagens" e "Engrenagem" em barras próximas, um gerador e outro restrição.
- Restrição rápida usa caracteres (`asm_ui.rs:52-66`), e Coincidência e Polia têm o mesmo símbolo.
- Tooltip em dois níveis: o avançado leva 5 s (`icons.rs:402`). Falta imagem e, nas restrições, a ordem de seleção.
- Painel: cerca de 24 campos e nenhuma dica ao passar o mouse neles; as 4 que existem são dos botões da barra
  (`asm_ui.rs:1628-2110`). O que ajuda são as legendas fixas em letra pequena, presentes em parte dos campos.

## O que não dá para ser 3D, e por quê

- **Lista de peças, numeração e tabelas.** O conteúdo é texto e comparação entre linhas; o 3D só serve de realce ligado à linha.
- **Leis de movimento em `t`, parâmetros e fórmulas.** São expressões. O 3D mostra o efeito, não a fórmula.
- **Gráficos de velocidade e aceleração.** O eixo do tempo não existe no modelo; a trajetória, sim, pode ir ao 3D.
- **Cálculo de engrenagem.** Módulo, correção e ângulo de pressão pedem tabela; um desenho do engrenamento ajuda, mas não
  substitui os números.
- **Escolha de arquivo, norma e família de fixador.** É consulta a catálogo; a miniatura ajuda, mas a decisão é por dados.
- **Conflito do solver.** O conjunto mínimo em conflito é uma explicação lógica; o 3D realça as peças, e o texto diz o motivo.
- **Estados, revisão, nomes e salvar.** Não têm geometria.
- **Valor exato.** A alça serve para aproximar; a precisão é do campo. Por isso cada alça precisa de um rótulo editável.
