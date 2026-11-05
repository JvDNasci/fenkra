# Benchmark de detalhe — Estrutural e soldas

> Texto próprio, a partir de ajudas públicas de softwares de modelagem CAD avançados e da documentação do
> FreeCAD. O módulo já foi comparado por função em [estrutural-e-soldas.md](../estrutural-e-soldas.md) e em
> [pesquisa/estrutural.md](../../pesquisa/estrutural.md); aqui a pergunta é outra: **como cada comando se
> comporta na mão de quem o usa** (campos, prévia, manipuladores no 3D, dicas). Fontes consultadas ficam em
> [procedencia.md](../../arquitetura/procedencia.md).

## Como o Fenkra funciona hoje (levantamento no código)

Barra: `crates/fenkra-app/src/toolbar.rs:387-394` (Membro, família Placa/Reforço, Biblioteca, Lista de corte,
Gerar montagem). Cada botão tem nome, frase curta e um texto "avançado" que só aparece depois de 5 s com o
mouse parado (`icons.rs:403-428`, `hover_help`). Não há atalho nem imagem no tooltip.

Painel do Membro: `st_ui.rs:21-105`. Seções: Perfil (família e medida em listas, `st_ui.rs:25-52`), Cotas da
seção recolhidas (`:53-58`), Linhas guia (`:59`), Ancoragem e orientação com grade 3x3 de bolinhas e "centróide"
(`:61-79`), campos Rotação, Deslocamento X/Y, Prolongar início/fim (`:80-84`), Cantos (padrão) com quatro ícones
(`:85-94`), Folga de solda (`:95`) e a lista "Cantos com outros membros" (`:106-190`). Placa e Reforço têm
painel curto em `feat_ui.rs:429-451`: ponto, espessura, sobra (placa); aba, espessura, chanfro (reforço).

Campos: `editor.rs:146-175`. Todo campo numérico mostra o mesmo texto de ajuda (`EXPR_HINT`, `editor.rs:143`,
sobre expressões e roda do mouse) e um ícone de caractere escolhido pelo nome do rótulo (`param_icon`). Nenhum
campo explica o que ele significa na peça.

Criação: `app.rs:1545-1590`. O comando **cria o recurso na hora com valores padrão** (família 1, medida 4, meia
esquadria, folga 0). O resultado do recálculo é a única "prévia": não existe fantasma antes de confirmar, nem
manipulador no 3D. Uma busca por `StructMember`, `StructPlate` e `StructGusset` em `handles.rs`, `gizmo.rs`,
`render.rs`, `scene.rs` e `picking.rs` não encontra nada: **nenhum dos comandos da bancada tem alça no 3D**.
Se o Membro é acionado sem linhas selecionadas, aparece um aviso, mas o recurso vazio é criado mesmo assim
(`app.rs:1550-1554`). Placa e Reforço exigem um vértice previamente selecionado (`app.rs:1571-1579`).

Ícones: `assets/icons/st_*.svg`, 10 arquivos de cerca de 1 KB (um por comando e um por tipo de canto), em
degradê azul de 48 px.

## Resumo: as 5 maiores lacunas de experiência

1. **Sem prévia nem manipulador no 3D.** Os outros CADs mostram o perfil já sobre o esqueleto, trocam de lado
   ao arrastar uma seta, giram a seção por uma alça com ímã em 45 graus e deixam o prolongamento ser puxado no
   modelo. No Fenkra tudo é digitado num painel e o efeito só aparece no recálculo.
2. **Seleção do esqueleto fora do comando.** Nos outros, o comando já abre pedindo as linhas, agrupa linhas
   contíguas num grupo com tratamento de canto próprio e lista os segmentos. No Fenkra é preciso selecionar
   antes, e o erro vem depois de o recurso vazio existir.
3. **Cantos sem rosto.** Cada encontro deveria aparecer no 3D com cor por tipo de situação (dois membros, três
   ou mais, já tratado, ignorado), clique para trocar o tratamento e cartão com ângulo e folga. Hoje é uma
   tabela de texto "linha 1 ↔ linha 2" cuja posição só aparece no tooltip (`st_ui.rs:129`).
4. **Campos mudos e perfil sem desenho.** Nenhum campo diz o que faz; o seletor de perfil é uma lista de
   texto, sem miniatura da seção, sem as cotas anotadas e sem busca. A ancoragem usa "•" e "◉" sem figura.
5. **Placa, reforço e solda pobres em opções.** Os outros oferecem lado da espessura, posição ao longo do
   membro, perfil poligonal, placa interna com recuo e cordão por conjuntos de faces (contínuo ou
   intermitente, com propagação pelas tangentes). O Fenkra tem três números por comando e nenhum cordão 3D.

## Checklist por comando

Legenda: ok, parcial, falta. "Manipulação 3D" avalia arrastar algo no modelo. Os ícones foram avaliados pelo que
o código mostra (arquivos, nomes, texto vizinho); confirmar em captura (`FENKRA_DEMO*`) antes de redesenhar.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Membro (painel geral) | parcial | parcial | falta | parcial | parcial | falta | parcial | Cria na hora com padrões; sem fantasma; texto só |
| Linhas guia (esqueleto) | parcial | parcial | falta | parcial | parcial | falta | falta | Exige seleção prévia; cria recurso vazio com aviso |
| Ancoragem e orientação | parcial | parcial | falta | falta | parcial | parcial | parcial | Grade de bolinhas sem desenho da seção; uma dica |
| Rotação, deslocamento, prolongar, folga | parcial | parcial | falta | falta | falta | falta | parcial | Só números; sem setas nem rótulo de cota |
| Cantos padrão (4 modos) | parcial | parcial | falta | ok | ok | parcial | parcial | Ícones por modo bons; sem prévia do corte |
| Cantos por par | falta | falta | falta | falta | parcial | parcial | parcial | Tabela textual; posição só no tooltip |
| Placa de extremidade | parcial | parcial | falta | parcial | parcial | falta | parcial | Escolhe a ponta pelo vértice; só externa |
| Reforço (gusset) | parcial | parcial | falta | parcial | parcial | parcial | parcial | Triangular ou chanfrado; sem lado, posição, perfil |
| Biblioteca de perfis | parcial | falta | falta | parcial | ok | parcial | parcial | Tabela de medidas; sem desenho da seção |
| Lista de corte | ok | falta | falta | parcial | ok | parcial | parcial | Sem realce no 3D, sem ângulos de ponta |
| Gerar montagem | parcial | falta | falta | parcial | ok | falta | parcial | Pede arquivo salvo; sem resumo antes de gerar |

Contagem das 77 células: 6 ok, 45 parcial, 26 falta.

## Por comando

### Membro (o mais usado)

**O que os outros têm em detalhe e o Fenkra não.**
- O comando abre já pedindo o esqueleto: o usuário clica linhas no modelo e cada clique vira um **grupo**
  numerado. Linhas encostadas formam um grupo contíguo, e só nele existe a opção "aplicar tratamento de canto".
  Grupos separados convivem no mesmo comando, cada um com perfil, ângulo e ancoragem próprios.
- **Perfil em três listas encadeadas** (norma, tipo, tamanho) com miniatura da seção, e o perfil já aparece
  sobre a linha antes de aceitar. Um recurso adicional chamado de ponto de perfuração (pierce point) deixa o
  usuário escolher qual vértice ou ponto da seção fica sobre a linha, clicando diretamente no desenho da seção,
  com o perfil deslizando para mostrar o resultado.
- **Espelhar** a seção em torno do eixo horizontal ou vertical e **alinhar** a um vetor (aresta, linha de
  construção, eixo), além do ângulo numérico.
- **Alças no modelo**: setas de deslocamento lineares e uma alça angular que gruda de 45 em 45 graus ao clique
  duplo; a opção de girar em torno do ponto escolhido ou do esqueleto; botões que ajustam a vista para olhar de
  frente para as alças. Em alguns CADs a origem e a orientação do perfil mudam enquanto o cursor percorre o
  esqueleto, antes do clique.
- **Folga entre segmentos** em dois campos separados: dentro do grupo e contra outros grupos. A folga encurta o
  comprimento na lista de corte mas mantém a extensão nominal.
- **Permitir saliência** (a barra passa do esqueleto) e **mesclar corpos de arcos** tangentes.
- Mudar só o tamanho **preserva** os tratamentos de ponta já feitos.
- Presets para reaplicar combinações de opções.

**Como deveria ser no Fenkra.**
- *Suporte:* arestas, linhas de referência e curvas, escolhidas dentro do comando, com realce e lista de
  segmentos (agrupados por contiguidade).
- *Modo:* Simples (família, medida, lado) e Avançado (ancoragem, espelho, alinhar, folgas, saliência).
- *Parâmetros:* família, medida, rotação, deslocamento X/Y, prolongar, folgas (interna e contra outros grupos).
- *Avançado:* presets nomeados, preservar tratamentos ao trocar a medida, mesclar arcos.
- *No 3D:* fantasma translúcido do perfil sobre cada linha, seção desenhada no ponto mais próximo do cursor com
  as cotas da seção anotadas, grade de 9 pontos de ancoragem como pontos clicáveis sobre a própria seção, seta
  dupla de deslocamento X/Y, arco com ímã de 45 graus para a rotação, alças de prolongar nas duas pontas com
  leitura em mm, nome do segmento ("linha 3") junto à linha.
- *Roteiro do GIF do tooltip (8 s):* (0-2 s) clicar duas linhas de um esqueleto em L e ver o contorno amarelo;
  (2-4 s) o perfil aparece em fantasma, a seção em destaque no canto da tela; (4-6 s) clicar um ponto da seção
  e ver o perfil deslizar; (6-8 s) arrastar a alça angular até o ímã de 45 graus e confirmar.

### Cantos: padrão e por par

**Os outros têm.**
- Em modelos de estrutura inteira, uma janela de **gerenciamento de cantos** que lista cada canto, agrupa os
  parecidos, pinta os cantos no modelo por categoria (simples de dois membros, complexo com três ou mais,
  tratado, ignorado) e tem modo automático e manual; também limpa tudo com um botão.
- Em canto simples: tipos de corte (meia-esquadria, topo 1, topo 2, aparar), com **troca de lado por seta** que
  inverte quem passa. Em meia-esquadria, escolher entre bissetriz e rente. Em aparar, escolher corte reto (rente
  às faces planas de contato) ou corte encaixado (contorno do corpo vizinho).
- Em canto complexo: corte por corpo, por plano (primeiro contato ou contato total), folga, ignorar tratamento
  e permitir prolongar. Quem corta quem é definido por ordem de ferramenta.
- Outro CAD avançado trata a ponta como operação separada com **prévia**: o primeiro membro fica amarelo, o
  segundo azul, e o azul é o que sofre o corte; um botão troca os papéis; dois campos de deslocamento; aviso de
  que mais de dois membros dão resultado imprevisível. Para entalhe: seleciona quem recebe e quem dá o formato,
  com perfil básico (um deslocamento), moldes em I, C ou T (vários) ou perfil de biblioteca; opções "estender o
  perfil do entalhe" e "corte perpendicular", pensada para corte a laser em tubo.
- Aparar/estender até uma **face**: escolher a face (inclusive curva), os membros, inverter o lado que recebe o
  corte e dar um afastamento. Os ângulos de corte vão para propriedades do item.
- No software livre FreeCAD (módulo de arquitetura), a estrutura aceita apenas extrusão do perfil; cantos
  ficam por conta de booleanas manuais, o que mostra o piso do que o usuário tolera.
- Em um CAD de nuvem, o padrão do quadro é um tipo (meia-esquadria, topo, topo encaixado, nenhum) e há
  **substituição por canto**, com seta para inverter o topo.

**Como deveria ser no Fenkra.**
- *Suporte:* cada encontro detectado, mostrado como esfera pequena colorida no 3D (cor por situação).
- *Modo:* automático (usa o padrão do membro) e manual (clicar o canto abre um cartão flutuante).
- *Parâmetros:* tratamento, quem continua (seta), folga do canto, ângulo lido do sólido.
- *Avançado:* ignorar canto, permitir prolongar, ordem de corte entre três ou mais membros, agrupar cantos
  iguais e aplicar a todos.
- *No 3D:* fantasma do corte (meia-esquadria com a bissetriz como plano translúcido), membro que continua em
  amarelo e o que para em azul, seta de troca de lado clicável, rótulo "45,0 graus" e "folga 2 mm" sobre o canto.
- *Roteiro do GIF (6 s):* quadro de quatro tubos; passar o mouse num canto e ver o cartão; clicar "Aparar" e ver
  um tubo recuar; clicar na seta de lado e ver a troca; clicar "Aplicar aos 4 cantos iguais".

### Placa de extremidade (tampa)

**Os outros têm.** Escolher a **face da ponta** diretamente (não um vértice); espessura; sentido da espessura
(para fora, que aumenta o comprimento total, ou **interna**, com um recuo dentro do tubo); afastamento da
borda do membro até a borda da tampa, dado como valor ou como **razão da parede** (0 a 1); botão de inverter o
sentido do afastamento. A tampa vira corpo próprio e entra na lista de corte.

**Como deveria ser no Fenkra.** *Suporte:* face da ponta. *Parâmetros:* espessura, sobra, sentido
(externa/interna), recuo. *Avançado:* razão da parede, furos de fixação (padrão 4 furos com diâmetro e
distância da borda), chanfro nos cantos. *No 3D:* fantasma da placa, cota de espessura com seta, hachura no
trecho interno quando for tampa interna. *GIF (5 s):* clicar a face da ponta de um tubo, a placa aparece,
alternar "interna" e ver a placa entrar no tubo com o recuo cotado.

### Reforço (gusset)

**Os outros têm.** Perfil **triangular** (duas distâncias, uma por aba, com botão de inverter) ou **poligonal**
(com chanfro nos cantos); lado da espessura (**interno, dos dois lados, externo**) e valor; **posição ao longo
da aresta de encontro** (início, meio, fim) mais um afastamento invertível; escolha do par de faces do encontro
por clique. Um CAD avançado também permite criar vários reforços de uma vez ao longo da aresta (repetição).

**No Fenkra** o painel pede um ponto e três números (`feat_ui.rs:440-451`). **Deveria:** escolher as duas
faces ou a aresta de encontro; abas D1 e D2; espessura com lado; posição e afastamento; tipo triangular,
chanfrado ou poligonal; repetição (quantidade ou espaçamento). *No 3D:* fantasma do reforço com as abas
cotadas por seta, alça na ponta de cada aba, seta de afastamento. *GIF (6 s):* clicar a aresta do encontro
de um tubo com outro; arrastar a aba 1 e ver o número mudar; trocar o lado da espessura; repetir três vezes.

### Aparar/estender (hoje ausente, mas é o comando que mais ajuda a esquerda dos cantos)

Entram: limite (plano, face, corpo ou outro membro), membros a mexer, tipo de canto (meia-esquadria, topo,
aparar), **permitir prolongar** (alonga se o membro não alcança o limite), inversão do lado e folga. Dica de
uso encontrada nas ajudas: preferir uma face plana como limite e só usar corpos para limites curvos (tubo
redondo, superfície escalonada). *No 3D:* limite destacado, seta indicando o lado que sobra, trecho removido em
vermelho translúcido e trecho acrescentado em verde. Já tratado no item B37; aqui só se detalha a interação.

### Cordão de solda (hoje só símbolo no desenho)

**Os outros têm.**
- Dois níveis: **cosmético** (leve, só gráfico, o símbolo anotado vai para o desenho) e **sólido** (gera
  material, entra na massa e na lista). O cosmético é a recomendação padrão; o sólido serve quando o desenho
  precisa atualizar símbolo e anotação do cordão juntos.
- Seleção por **conjuntos de faces** (primeiro conjunto, depois "continuar" e o segundo conjunto), tecla Shift
  para tirar uma face, opção de **encadear tangentes** e **cordão de comprimento total**.
- Lista de caminhos únicos, limitada a dois corpos por caminho; ao escolher, o caminho é realçado.
- Tamanho (perna do filete; um valor deixa as duas iguais), formato (plano, convexo, côncavo), **intermitente**
  (comprimento e vão, ou passo = comprimento + vão), **alternado**, e término em plano de trabalho ou face.
- Símbolo criado junto ou depois; a preparação (chanfro, abertura) vai descrita no símbolo.

**Como deveria ser no Fenkra.** *Suporte:* par de faces ou aresta de encontro. *Modo:* cosmético (padrão) e
sólido. *Parâmetros:* perna, tipo (filete, ranhura), comprimento e vão, lado. *Avançado:* propagar tangentes,
início e fim em plano, cordão alternado, processo e ordem. *No 3D:* linha colorida ao longo do caminho com
seção do cordão pequena na ponta, trechos intermitentes tracejados, símbolo flutuante com a medida (a4) que
leva ao símbolo do desenho. *GIF (8 s):* clicar o par de faces de um T, a linha amarela aparece; mudar para
intermitente e ver os trechos; passar para o desenho e ver o símbolo já preenchido.

### Lista de corte

**Os outros têm.** Cada item guarda comprimento, **ângulo 1** (ponta mais perto do esqueleto) e **ângulo 2**
(ponta oposta), descrição, material, quantidade e comprimento total, e esses campos não são editáveis (vêm do
sólido). Iguais com materiais diferentes ficam em pastas separadas. Propriedades do esboço do perfil e do
recurso são herdadas pelos itens. Uma janela de agrupamento deixa o usuário reagrupar. No Fenkra, só perfil,
comprimento, quantidade e barras (`st_ui.rs:344-415`).

**Deveria.** Colunas de ângulos de ponta e comprimento da linha de centro vs. ponta longa; ordenar por coluna;
clicar a linha **realça os corpos no 3D** (e o contrário: clicar um corpo seleciona a linha); filtro por
material; exportação com os mesmos campos. *GIF (5 s):* clicar a linha "50x3 · 1.250 mm" e ver três tubos
acenderem no modelo.

### Biblioteca de perfis

**Os outros têm** seleção em cascata (norma, tipo, tamanho), miniatura da seção por tamanho, perfis
personalizados criados desenhando um esboço de seção e marcando o ponto de perfuração, e arquivos de tabela
que o usuário adiciona (o FreeCAD usa um CSV simples, tudo em mm). **Fenkra:** janela com tabela por família
(`st_ui.rs:238-343`), sem desenho. **Deveria:** painel lateral com a seção desenhada em escala e as cotas
nomeadas (altura, largura, espessuras), atualizada ao editar; marcador do ponto de ancoragem; aviso de medida
inválida junto à linha.

### Gerar montagem

Os outros mostram, antes de gerar, uma lista do que será criado (quantidade de arquivos, nomes) e permitem
renomear. O Fenkra exige arquivo salvo e avisa por mensagem curta (`st_ui.rs:207-236`). **Deveria** abrir
um resumo (peças, instâncias, pasta de destino) com Confirmar.

## O que não dá para ser 3D, e por quê

- **Biblioteca de perfis e tabela de medidas:** é cadastro de dados; o 3D não ajuda. O que ajuda é o desenho 2D
  da seção em escala, que se faz com a mesma rotina que gera a seção.
- **Lista de corte e exportação CSV:** é uma tabela. O 3D entra só como realce ao clicar uma linha.
- **Gerar montagem:** a operação é de arquivos e instâncias; só o resumo prévio e, depois, a árvore da montagem.
- **Folga de solda, espessura e tamanho do cordão:** valores pequenos (1 a 6 mm) em estruturas de metros; a
  seta não se enxerga. Mostrar rótulo flutuante com zoom automático ao editar o campo, não alça arrastável.
- **Ordem de corte e regras de dominância entre três ou mais membros:** é lógica; o 3D mostra o resultado e
  números sobre os membros, mas a decisão é por lista.
- **Níveis de exigência de solda (processo, norma, ensaios):** são texto do símbolo, não geometria.

## Contagem do checklist

11 linhas, 7 critérios por linha, 77 células: 6 ok, 45 parcial, 26 falta. A manipulação 3D está 100% em falta e a dica por campo quase também.
