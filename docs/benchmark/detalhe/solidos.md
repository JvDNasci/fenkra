# Benchmark de detalhe — bancada Sólidos

> Texto próprio, a partir de documentação pública de softwares de modelagem CAD avançados e de software livre
> (FreeCAD). Este arquivo não repete o benchmark por módulo ([solidos.md](../solidos.md)): aqui se compara o
> nível de detalhe de cada comando, campo a campo e gesto a gesto. Código lido em `crates/fenkra-app/src`
> (toolbar.rs, feat_ui.rs, editor.rs, handles.rs, gizmo.rs, icons.rs, app.rs).

## Como o Fenkra funciona hoje (base da comparação)

- A barra (`toolbar.rs:125-215`) agrupa os comandos de Sólidos em famílias; cada botão tem ícone SVG, nome, frase
  curta e frase longa. O tooltip (`icons.rs:403`) mostra título e frase curta e só revela a frase longa depois de
  5 s de mouse parado. É texto puro: não há imagem, GIF nem passo a passo.
- O painel do comando (`editor.rs:1194` extrusão, `:1215` revolução, `:1334` furo; `feat_ui.rs:188` os demais) é
  um formulário. Campos numéricos usam `expr` (`editor.rs:146`): rótulo com ícone de grandeza e uma dica genérica,
  igual para todos ("aceita número, parâmetro ou expressão..."). Não existe dica própria do campo.
- Não há prévia fantasma. A edição recalcula o documento a cada mudança (sessão com snapshot, `app.rs:234`), então
  o corpo "vira" o resultado; se a geometria falha, a viewport volta ao estado anterior e a pessoa não vê o que
  tentou fazer. Não há cor distinta para o material que entra ou sai.
- Manipuladores 3D (`handles.rs:49`): uma seta com cota editável para extrusão cega, plano médio e dois lados, altura
  de primitivas, raio/distância de filete e chanfro (a partir da primeira aresta), espessura da casca,
  profundidade do reforço, plano deslocado e algumas superfícies; arco de ângulo na revolução (`handles.rs:273`).
  Faces e arestas escolhidas aparecem só como bolinhas (`app.rs:5788`, `:5801`), sem realce da face ou da aresta
  inteira.
- `gizmo.rs` (setas por eixo, arco, escala) serve a pontos e transformações de geometria, não ao Mover corpo,
  Padrão, Espelho ou Escala dos sólidos.

## Resumo: as 5 maiores lacunas de experiência

1. **Sem prévia fantasma.** Os outros mostram o volume que vai entrar (ou sair) em cor própria, translúcido, antes
   de confirmar, e mantêm a última prévia válida (ou a superfície tentada) quando o valor fica inválido. No Fenkra o
   corpo é recalculado e, se falha, o retorno visual some. Atinge todos os comandos.
2. **Seleção sem realce de face/aresta.** Casca, saída, espessura, filete face-face e completo, rosca e padrão por
   faces usam bolinhas. Faltam realce da face inteira ao passar o mouse, cor por papel (face aberta, face neutra,
   face lateral), contador e lista com remover item.
3. **Manipulação 3D curta.** A seta cobre um valor e poucos términos. Faltam seta de saída e de ângulo, setas duplas
   para dois lados, arrastar até uma face (o término "até" encaixa na face sob o cursor), pontos de controle do
   filete variável, espessura por lado na nervura e rótulo com grandeza e unidade no 3D.
4. **Campos e tooltips sem ensino.** A dica é igual em todo campo numérico, as caixas de seleção não têm hover e o
   tooltip não tem imagem. Os outros mostram miniatura animada de poucos segundos e um texto por campo.
5. **Fluxo sem "campo ativo".** O padrão dos outros é: escolher o comando, o campo esperado fica destacado e o clique
   no 3D o preenche (perfil, face, aresta, corpo); ao completar, o painel avança sozinho. No Fenkra vários comandos
   pedem perfil, eixo, plano e corpo ferramenta em combos; só faces e arestas aceitam clique, sem indicar qual
   campo espera.

## Checklist por comando

Critérios: **Intuitivo** (dá para usar sem ler nada); **Prévia 3D** (fantasma do resultado antes de confirmar;
"parcial" = o corpo recalcula ao vivo, mas sem fantasma); **Manipulação 3D** (seta, arco ou alça arrastável com
cota); **Ícone claro** (mostra o resultado e se distingue dos vizinhos); **Tooltip** (nome, ideia e exemplo visual;
"parcial" = só texto); **Dica por campo** (cada campo explica a si mesmo); **Fluxo** (clique no 3D preenche a
etapa certa e o painel avança). Valores: ok, parcial, falta.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Extrusão (4 variações) | parcial | parcial | parcial | ok | parcial | parcial | parcial | Seta só em cego, plano médio e dois lados; sem seta de saída; sem arrastar até face. |
| Recorte (3 variações) | parcial | parcial | parcial | parcial | parcial | parcial | parcial | Recorte até plano reutiliza o ícone `ext_ate_plano` (`toolbar.rs:131` e `:142`); sentido do recorte confunde. |
| Revolução / Ranhura | parcial | parcial | ok | ok | parcial | parcial | parcial | Arco arrastável existe; eixo só por combo, sem clique em linha/aresta no 3D. |
| Nervura (varredura) | parcial | falta | falta | parcial | parcial | parcial | falta | Perfil e guia por combo; sem mostrar a trajetória nem o perfil posicionado. |
| Rasgo (varredura -) | parcial | falta | falta | parcial | parcial | parcial | falta | Mesmo painel da nervura; o nome "Rasgo" é ambíguo (chapa tem rasgo também). |
| Multi-seções (loft) | parcial | parcial | falta | ok | parcial | falta | falta | Sem guias, tensão nem conexões; lista de perfis sem realce no 3D. |
| Primitivas (7) | ok | parcial | parcial | ok | parcial | parcial | parcial | Seta só de altura; sem arrastar a base nem as outras medidas. |
| Unir / Subtrair / Interseção | parcial | falta | falta | ok | parcial | parcial | parcial | Corpo ferramenta em combo; não se escolhe clicando no corpo; `keep_tool` existe. |
| Reforço (stiffener) | parcial | falta | parcial | parcial | parcial | parcial | parcial | Seta de profundidade; sem lado/ambos os lados. |
| Furo (4 tipos) | parcial | falta | falta | parcial | parcial | parcial | parcial | Sem fantasma do furo, sem arrastar diâmetro/profundidade, sem clique na face para posicionar. |
| Filete (5 variações) | parcial | parcial | parcial | ok | parcial | parcial | parcial | Seta a partir da 1ª aresta; variável sem pontos; sem realce da aresta. |
| Chanfro (4 variações) | parcial | parcial | parcial | ok | parcial | parcial | parcial | Duas distâncias e distância + ângulo sem seta por lado; sem inverter por aresta. |
| Casca | parcial | parcial | parcial | ok | parcial | parcial | parcial | Seta de espessura; sem espessura por face; faces abertas só com bolinha. |
| Saída (draft) | falta | falta | falta | parcial | parcial | parcial | falta | Plano neutro em combo; sem seta de direção de extração nem leitura de ângulo no 3D. |
| Espessura | parcial | parcial | falta | parcial | parcial | parcial | parcial | Faces por bolinha; o sinal +/- decide acrescentar ou remover (pouco óbvio). |
| Rosca | falta | falta | falta | parcial | parcial | parcial | falta | Sem fantasma da rosca nem início/fim visíveis; face cilíndrica indicada por coordenadas em texto. |
| Padrão linear | parcial | parcial | falta | ok | parcial | parcial | parcial | Sem pontos de instância, sem pular instâncias, sem seta de espaçamento. |
| Padrão circular / por pontos | parcial | parcial | falta | ok | parcial | parcial | parcial | Sem arco de espaçamento nem instância fantasma. |
| Espelho | parcial | parcial | falta | ok | parcial | falta | parcial | Plano em combo; o plano de simetria não aparece no 3D. |
| Escala | parcial | parcial | falta | parcial | parcial | parcial | parcial | Sem alça de escala no 3D; fator só digitado. |
| Mover corpo (4 modos) | parcial | parcial | falta | ok | parcial | parcial | parcial | Sem setas de translação nem arco de rotação no corpo (o gizmo é só de geometria). |
| Plano deslocado (referência) | ok | parcial | ok | parcial | parcial | parcial | parcial | Seta com cota (`handles.rs:139`); falta o retângulo translúcido com rótulo de distância. |

Contagem: 22 linhas x 7 critérios = 154 células: **ok = 16, parcial = 109, falta = 29**.

## Por comando: o que os outros têm e o Fenkra não

### Extrusão e recorte

**O que existe nos outros.**
- Perfil escolhido por clique no 3D: cada região fechada do sketch é realçada ao passar o mouse e várias podem ser
  somadas; em geral clicar numa face plana também serve de perfil.
- Uma seta que sai do centro do perfil; arrastar muda a distância e o valor aparece numa etiqueta editável. A seta
  ganha duas pontas no modo simétrico e uma segunda seta no modo de dois lados.
- "Início" independente do plano do sketch: o próprio plano, um deslocamento, ou uma face/plano escolhido.
- Término por objeto com regra de extensão (só a face escolhida, as faces vizinhas, o corpo inteiro) e um
  deslocamento a partir do alvo.
- Operação explícita: unir, cortar, interseção, novo corpo; ao cortar, lista de corpos afetados.
- Ângulo de saída e parede fina com lado (um lado, outro, centrada) e espessura.
- Prévia translúcida em cor própria (uma cor para adição, outra para remoção), atualizada durante o arraste.
- FreeCAD: o painel mostra só os campos que valem para o tipo de término (comprimento só em dimensão; deslocamento só
  em passante, até o primeiro e até face), e o sentido é uma caixa "inverter".

**O que o Fenkra tem.** Término em 8 tipos com ícones (`editor.rs:419`), direção por linha, ângulo e parede fina, seta
no 3D só para cego, plano médio e dois lados (`handles.rs:55`).

**Como deveria ser no Fenkra.**
- *Suporte:* perfil clicando na região do sketch ou na face plana; o campo "Perfil" fica destacado esperando o clique.
- *Modo:* Cego, Plano médio, Dois lados, Passante, Até o próximo/último, Até face/plano/superfície/corpo; início:
  plano do sketch, deslocamento ou face.
- *Parâmetros:* distância (seta), ângulo de saída (seta curva na aresta do perfil), parede fina (lado e espessura),
  operação (unir, cortar, interseção, novo corpo).
- *Avançado:* segundo término com valor próprio, direção por eixo, contornos selecionados, corpos alvo.
- *No 3D:* fantasma translúcido (azul ao unir, vermelho ao cortar), seta com cota editável, setas nas duas pontas no
  simétrico, perfil realçado, rótulo "Distância 20 mm" que passa a "Até face" quando a seta encaixa numa face.
- *GIF do tooltip (7 s):* sketch de retângulo; o cursor passa sobre a região (realça); aparece o fantasma baixo; o
  cursor puxa a seta e o fantasma sobe com a etiqueta "20 mm"; clique no visto; o corpo fica sólido.

### Revolução / Ranhura

**Nos outros.** Eixo escolhido clicando numa linha do sketch, aresta ou eixo da origem (o eixo ganha realce e uma
linha tracejada); tipo de ângulo (360°, um lado, simétrico, dois lados) como botões; seta curva arrastável com
leitura em graus; prévia do sólido mesmo em giro parcial. Se o perfil cruza o eixo, o aviso aparece junto do campo.

**No Fenkra.** Arco arrastável (`handles.rs:273`) e ângulo 2 opcional, mas o eixo é um combo (`editor.rs:1223`) e não
há aviso de perfil que cruza o eixo antes de calcular (B75 prevê a validação).

**Como deveria ser.** Eixo por clique, botões de tipo de ângulo, eixo tracejado, aviso inline "o perfil cruza o eixo".
*GIF (6 s):* perfil em L ao lado de uma linha; clique na linha (vira eixo); o arco cresce até 360° e o corpo gira.

### Furo

**Nos outros.**
- Colocar clicando na face (o furo-guia segue o cursor) ou em pontos do sketch; vários furos antes de confirmar;
  posição editável por cotas.
- Tipo (simples, folga, roscado, cônico) e norma/tamanho que preenchem os diâmetros; editar à mão marca
  "personalizado". Profundidade de rosca separada da profundidade do furo, com folga no fundo medida em passos.
- Ponta da broca (118°, 135°, plana, valor livre) e deslocamento da ponta para o corte não invadir outra parede.
- Término: cego, até o próximo, até a entidade, passante; plano de início (face, plano do sketch, outro).
- Setas no 3D: inverter o sentido, e alças de diâmetro e profundidade.
- O nome da feature se atualiza com os valores ("Furo M8 roscado x 20").

**No Fenkra.** Boca, rosca e fixador no painel, norma/tamanho/folga (`editor.rs:1390`), pontos 3D projetados na
superfície e ponta; sem prévia nem seta de profundidade; sem clique na face para posicionar.

**Como deveria ser.**
- *Suporte:* clicar na face (furo fantasma segue o cursor, alinhado à normal) ou pontos do sketch.
- *Modo:* simples, rebaixado, escareado, roscado, cônico.
- *Parâmetros:* norma e tamanho, diâmetro, profundidade, ponta.
- *Avançado:* início deslocado, profundidade de rosca, folga no fundo, ângulo do escareado.
- *No 3D:* cilindro fantasma com cota de diâmetro e seta de profundidade; escareado desenhado no ângulo certo.
- *GIF (8 s):* o cursor sobre uma face mostra o círculo-guia; clique; troca "M6" por "M8" e o fantasma engrossa; puxa a
  seta de profundidade.

### Filete e chanfro

**Nos outros.**
- Arestas escolhidas por clique, por face (todas as arestas dela) ou por propagação tangente (ligada por padrão);
  arestas realçadas e contador "3 arestas".
- Seta de raio sobre a própria aresta; outra seta para a posição inicial no filete parcial; setas para trocar o
  lado do filete assimétrico; "largura" no lugar de raio; controle de forma (circular, cônico, curvatura).
- Filete variável com pontos nos vértices e pontos extras sobre a aresta, raio por ponto e transição suave opcional.
- Quando o raio não cabe, mostram a superfície tentada (em vermelho) em vez de apagar o resultado.
- Chanfro: três tipos (igual, duas distâncias, distância e ângulo) com botão inverter e substituição por aresta;
  medida a partir da aresta ou do ponto tangente.
- Lembram o último raio usado e partem de 45° no chanfro por ângulo.

**No Fenkra.** Cinco variações de filete, quatro de chanfro, seta a partir da 1ª aresta, aviso quando a variação
exige arestas escolhidas (`feat_ui.rs:1055`). Faltam propagação tangente, aresta realçada, parcial, assimétrico,
visualização do erro e memória do último valor.

**Como deveria ser.** Realçar a aresta inteira (cor e espessura), propagação tangente ligada, seta de raio na aresta
mais próxima do cursor, pontos de controle no variável (B49), mostrar a superfície tentada quando falha. *GIF (6 s):*
o mouse passa na aresta (realça); clique; a propagação acende as arestas tangentes; a seta puxa o raio; o canto fica
redondo.

### Casca

**Nos outros.** Clique nas faces a remover (cor própria); espessura com seta na face; botão para o lado de fora;
espessura por face e opção "esvaziar sem abrir"; controle deslizante que compara antes e depois.

**No Fenkra.** Espessura única, "para fora", faces por bolinha, seta numa face (`handles.rs:96`).

**Como deveria ser.** Faces abertas em vermelho translúcido, espessura por face em lista, esvaziar sem abrir, corte de
seção da prévia (meio corpo) para ver a parede. *GIF (6 s):* clique na face de cima (fica vermelha); a parede aparece
em corte; a seta puxa 3 mm.

### Saída (draft)

**Nos outros.** Primeiro o plano neutro (ou a linha de partição) com uma seta da direção de extração, depois as faces;
ângulo com arco no 3D; análise de cor por face conforme a inclinação, para conferir se desmolda; inverter por botão.
Em alguns só há uma direção de saída por feature (para duas direções, divide-se o corpo antes).

**No Fenkra.** Ângulo, plano neutro em combo, inverter; sem seta de direção nem análise de cor.

**Como deveria ser.** Seta de extração no plano neutro, plano desenhado, faces em lista, análise verde/amarelo/vermelho
do ângulo mínimo. *GIF (5 s):* a seta de extração aponta para cima; as laterais ficam vermelhas e depois verdes
conforme o ângulo sobe.

### Padrões e espelho

**Nos outros.** Prévia com um ponto por instância; clicar no ponto "pula" a instância (entra numa lista com X para
restaurar); setas de distância e arco de ângulo; direção por aresta ou eixo no 3D; modos "espaçamento" e "comprimento
total"; padrão de faces e de corpos; instância contada junto com a original. O espelho mostra o plano e deixa unir
ou manter separado.

**No Fenkra.** Espaçamento ou comprimento total, duas direções, circular e por pontos (`feat_ui.rs:307`); sem pular
instâncias, sem pontos de instância, espelho sem mostrar o plano. O B91 cobre pular e reavaliar.

**Como deveria ser.** Fantasmas pontilhados e um ponto clicável por instância; seta de espaçamento no primeiro passo;
plano de espelho translúcido com rótulo "Plano XY". *GIF (6 s):* uma peça, 4 cópias em fantasma; clique numa (some);
puxa a seta e o espaçamento muda.

### Multi-seções (loft) e varredura

**Nos outros.** Perfis escolhidos clicando em ordem e numerados no 3D; linhas de conexão entre perfis com alças nos
vértices (evitam torção); guias contínuas que cruzam cada perfil; condição inicial e final por perfil (normal,
tangente, curvatura) com setas de intensidade; transição em tempo real. Varredura: perfil, trajetória, orientação
(acompanha, fixa, torção) e escala ou torção ao longo do caminho.

**No Fenkra.** Lista com ▲▼✖, regrada e fechada; sem guias nem condições (B12); varredura com 3 modos de orientação.

**Como deveria ser.** Clicar os perfis no 3D com numeração, linhas de conexão arrastáveis, condição inicial e final
com seta em cada ponta. *GIF (8 s):* clica 3 perfis (1, 2, 3 acendem); a transição aparece; arrasta um vértice de
conexão e a torção some.

### Rosca, espessura, escala, mover corpo, booleanas, primitivas, reforço

- **Rosca:** os outros mostram a rosca na face cilíndrica escolhida, com início e fim visíveis e deslocamento de
  início; passo e perfil vêm de tabela; avisam quando o diâmetro não coincide. No Fenkra a face aparece como
  coordenadas em texto (`feat_ui.rs:395`).
- **Espessura:** face com realce; acrescentar ou remover por botão, não por sinal digitado.
- **Escala:** alça no 3D e eixos independentes; no Fenkra só o campo "Fator".
- **Mover corpo:** setas por eixo e arco de rotação sobre o corpo, com encaixe em arestas e faces; no Fenkra, quatro
  modos digitados.
- **Booleanas:** alvo e ferramenta escolhidos clicando nos corpos, que ganham cores (alvo e ferramenta), com "manter
  ferramentas" e prévia; o Fenkra usa combo e tem `keep_tool`.
- **Primitivas:** o corpo aparece ao primeiro clique no plano, com arrasto da base e da altura; o Fenkra pede o plano
  num combo e depois só tem a seta de altura.
- **Reforço:** lado da parede (um lado, outro, dois) e "até o corpo" com prévia; B11 cobre.

## Ícones e tooltips (valem para todos)

- Ícone repetido: `ext_ate_plano` serve à extrusão até plano e ao recorte até plano (`toolbar.rs:131` e `:142`).
- O ícone deve mostrar o resultado (peça isométrica com a face afetada em cor de destaque) e cada variação deve se
  diferenciar por uma marca (seta de dois lados, plano tracejado, cor do corte).
- O tooltip deve dizer primeiro "o que clicar e o que acontece" e só depois a definição. Hoje as frases descrevem o
  efeito ("Empurra o perfil fechado...") mas não o passo.
- O B60 prevê tooltip em dois estágios e mini fluxo; falta a miniatura em loop de 5 a 10 s, silenciosa. Os roteiros
  estão em cada comando acima.
- Dica por campo: cada `expr` deve ter texto próprio ("Distância: quanto a extrusão avança a partir do plano do
  sketch") e, quando ajudar, uma figurinha de cota; hoje todas dizem o mesmo (`editor.rs:143`). Caixas de seleção
  também precisam de hover: "inverter sentido", "simétrico ao plano do sketch", "parede para fora".

## O que não dá para ser 3D, e por quê

- **Expressões e parâmetros** (L/2 + 5): são texto, e um valor ligado a parâmetro não pode ser arrastado sem quebrar o
  vínculo; o Fenkra já trava com cadeado. Manter.
- **Norma, tamanho e ajuste de furo e rosca:** são escolhas de tabela, não de geometria; o 3D só mostra o resultado.
- **Operação (unir, cortar, interseção):** é decisão lógica; o 3D reforça pela cor da prévia, mas a escolha é botão.
- **Quantidade de instâncias, número de faces, nomes e alvo de corpo:** inteiros ou nomes; podem virar rótulo no 3D,
  mas se digitam.
- **Parâmetros de qualidade** (tolerância de costura, continuidade, regrada ou suave): efeito sutil demais para um
  gesto; ficam no bloco avançado do painel.
- **Resultados de cálculo pesado** (rosca modelada, loft com guias): a prévia pode ser simplificada (hélice,
  perfis) e o sólido real vir só ao confirmar, com a etiqueta "prévia simplificada".
