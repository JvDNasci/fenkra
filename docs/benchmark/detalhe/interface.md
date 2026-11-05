# Benchmark de detalhe — Interface (padrão de experiência de todos os comandos)

> Texto próprio. Compara o Fenkra com softwares de modelagem CAD avançados e com o Blender e o FreeCAD, no
> nível do detalhe do gesto. Complementa [navegacao.md](../navegacao.md), [geral.md](../geral.md) e
> [interface-e-arvore.md](../../pesquisa/interface-e-arvore.md). Fontes públicas ficam na entrega da pesquisa e
> em `procedencia.md`; este arquivo não traz endereços.

## 0. Resumo executivo

O usuário testou e apontou sete dores: comandos pouco visuais, cota 3D difícil de puxar, roda do mouse
inconsistente, ícones que não explicam, tooltip pobre, campos sem dica e identidade pouco autêntica. A leitura
do código mostra que **as dores têm uma raiz comum: não existe um contrato de comando**. Cada comando decide por
conta própria se tem prévia, alça, rótulo, dica e ícone. Por isso o resultado é desigual. A solução é um contrato único, medido pelo checklist do fim do arquivo, e um teste automático falhar quando o contrato não é cumprido que falha quando o contrato não é cumprido (ideia já em B60, `backlog.md:53`).

As três maiores lacunas:

1. **Sem contrato de comando.** Prévia, alças e rótulo editável existem só para um punhado de comandos
   (`handles.rs:1-4`, `gizmo.rs:1-5`); o resto abre um formulário (`editor_panel`, `app.rs:6706-6732`).
2. **Campo numérico não é um componente.** A roda depende de cada chamador lembrar de chamar `wheel`
   (`ui_ext.rs:46-66`), exige foco (`ui_ext.rs:24`) e a dica é a mesma frase para todos os campos
   (`editor.rs:146`). A roda divergente entre painéis nasce daí (seção 2).
3. **Ajuda só em texto e ícones sem semântica.** `hover_help` mostra texto (`icons.rs:403-430`); todos os 356
   arquivos de `assets/icons` compartilham uma só matiz azul e variantes quase idênticas (seção 5).

## 1. Padrão de comando

### 1.1 Como cada referência conduz

- **Blender:** o operador roda na hora e o painel "ajustar última operação" refaz o resultado a cada campo; o gizmo da ferramenta dá a alça. Queremos essa ideia (resultado desde o primeiro quadro), mas com cancelamento sem rastro.
- **CADs avançados comerciais:** painel lateral no lugar da árvore; caixas de seleção com o campo ativo
  destacado; prévia translúcida; alça no 3D com a cota ao lado; botões de confirmar e cancelar também perto
  do cursor; seção "mais" recolhida com as opções raras. Em vários, a cota 3D é um rótulo que se arrasta, se
  clica para digitar e aceita expressão.
- **FreeCAD:** painel de tarefas com OK, Aplicar e Fechar; vários comandos sem prévia, e Aplicar vira prévia improvisada que deixa objetos no documento. Exemplo do que evitar.

### 1.2 O que o Fenkra tem hoje

- `Session` abre o comando como ambiente; o painel de edição fica à esquerda ou à direita
  (`settings.rs:382`, `app.rs:6733-6757`); `conclude_bar` (`app.rs:4605`) e `done_bar` (`app.rs:6248`) trazem
  Concluir e Cancelar; Enter e Esc funcionam (`interface-e-arvore.md:489`).
- Alças: `handles.rs` desenha seta linear, haste, ponta, círculo de pega e rótulo (`handles.rs:340-380`);
  `gizmo.rs` serve a pontos e transformações. Os dois cobrem poucos comandos (extrusão, recorte, plano
  deslocado, altura de primitivas, reforço).
- A ordem suporte → modo → parâmetros → avançado é meta do B20 (`backlog.md:111`) e ainda não vale em todos.

### 1.3 Defeitos concretos achados no código

| Achado | Onde | Efeito |
|---|---|---|
| Passo da arrastada fixo em 1 mm; 0,1 mm com Shift | `handles.rs:350-352` | Em peça pequena o valor pula; em peça grande é lento. O passo ignora o zoom. |
| Atalho de precisão **contraditório**: no 3D, Shift é fino (÷10); no painel, Shift é ×10 | `handles.rs:350` x `ui_ext.rs:5-18` | O usuário aprende uma regra e erra na outra. |
| Rótulo 3D só reage a clique (`Sense::click`); não se arrasta, não aceita roda | `handles.rs:387` | A cota "puxa mal": só a seta arrasta, e a pega é um círculo de 22 px. |
| Valor ligado a parâmetro vira cadeado e some a interação | `handles.rs:1-4` | Sem caminho para "desvincular" ou "editar o parâmetro" a partir do rótulo. |
| Alça só existe para comandos numéricos "lineares" | `handles.rs:347` | Sem anel de ângulo, sem plano arrastável, sem alça de raio nos demais. |

### 1.4 Contrato de comando proposto (vale para todos)

1. **Prévia viva desde o primeiro suporte válido.** Assim que o campo obrigatório é preenchido, o resultado
   aparece translúcido com valores padrão; cada mudança de campo atualiza sem confirmar. Cor por semântica:
   acrescentar em azul-água, remover em vermelho translúcido, só referência em cinza tracejado. Prévia
   inválida mostra a causa em texto e destaca a entrada culpada. Opção "prévia desligada" por comando para
   modelos pesados (`interface-e-arvore.md:495`).
2. **Manipulador por tipo de grandeza**, sempre os mesmos símbolos:

   | Grandeza | Manipulador | Rótulo |
   |---|---|---|
   | Distância | seta linear com ponta cônica, pega de 28 px | "12,5 mm" |
   | Ângulo | anel com marcador de arco | "35°" |
   | Posição em plano | quadrado arrastável entre as setas | "x, y" |
   | Posição livre | esfera central | "x, y, z" |
   | Raio | alça na borda da aresta ou face | "R 3" |
   | Quantidade | losango com contador que se arrasta sobre um passo | "×6" |
   | Plano de referência | retângulo translúcido com borda, seta normal e anel de giro | distância ao plano de origem |
   | Direção | seta curva de inversão, também por tecla | "↔" |

3. **Rótulo de valor no 3D é um mini-campo completo:** clique entra em edição (hoje já entra, mas a dica promete duplo clique,
   `handles.rs:389-391`); arrastar o próprio rótulo altera o valor; roda altera; aceita expressão e unidade;
   mostra a unidade; com parâmetro vinculado mostra o nome (`L/2`) em vez de cadeado e permite desvincular.
4. **Passo adaptativo.** Passo = potência 1-2-5 que corresponde a cerca de 4 a 8 pixels de tela no zoom atual
   (nunca menor que a resolução do documento). Ctrl liga imantação a valores redondos, Shift reduz ÷10, Alt
   imanta a geometria (face, aresta, plano "até"). A **mesma regra** vale no 3D e nos campos do painel.
5. **Estados visuais** de toda alça: repouso (traço fino, cor do eixo), pairando (espessura ×1,5, cursor de
   mão aberta, rótulo "puxar para alterar"), ativa (cor quente, linha-guia pontilhada até a origem, medida
   grande), travada por parâmetro (cadeado), erro (vermelho com causa).
6. **Fluxo do painel:** Suporte → Modo → Parâmetros → Avançado (recolhido). A barra de status lista o que falta
   ("falta: escolher a face de apoio"). Clique no 3D preenche o campo ativo e passa ao próximo quando válido.
7. **Confirmação:** Enter e Esc; botão ✔/✘ no painel e junto ao rótulo 3D; "Aplicar e continuar" repete o
   comando com os mesmos parâmetros (útil em furos e filetes). Esc nunca deixa passo de desfazer.

### 1.5 Contrato aplicado a sete comandos (referência de aceite)

| Comando | Prévia | Manipulador principal | Extra obrigatório |
|---|---|---|---|
| Plano deslocado (offset de plano) | retângulo translúcido na nova posição, com borda e etiqueta "Plano" | seta normal ao plano + rótulo de distância | linha-guia até o plano de origem; botão de inverter; imantar a faces |
| Extrusão / recorte | sólido translúcido até o fim | seta no topo do perfil | modo "até face" mostra a face alvo realçada |
| Filete / chanfro | aresta arredondada | alça no ponto médio da aresta | varrer arestas tangentes em cadeia |
| Furo | cilindro de remoção | esfera na posição + seta de profundidade | diâmetro por anel na boca |
| Padrão linear | cópias fantasma | seta do passo e losango da quantidade | direção por aresta |
| Mover / girar | peça na nova pose | gizmo de eixos, planos e anéis (`gizmo.rs`) | digitar o delta |
| Cota de sketch | cota na viewport | rótulo arrastável | entra em edição ao criar |

O primeiro item responde à reclamação do usuário: o offset deve mostrar o **plano**, com rótulo e seta de
pega. Hoje o plano deslocado é um dos poucos com seta (`handles.rs:1-3`), mas sem retângulo próprio, sem
linha-guia nem inversão.

## 2. Campo numérico padrão

### 2.1 Causa exata da roda do mouse

O código de roda é local e **opt-in**. `ui_ext::wheel` e `wheel_text` precisam ser chamados à mão logo após
cada campo; os campos são criados por `ui_ext::dv` (DragValue) ou `TextEdit`, nenhum dos quais chama a roda
sozinho (`ui_ext.rs:46-86`). Dentro da função, `notches` só devolve algo quando o campo está sob o mouse **e**
com foco do teclado (`ui_ext.rs:24`); sem foco, a roda rola o painel (comentário em `ui_ext.rs:20-22`). Um
`DragValue` só ganha foco depois de um clique que o põe em edição. Portanto: a roda "pega" apenas em campos
(a) em que alguém escreveu a chamada e (b) que já foram clicados.

Sobre esquerda x direita: o painel de edição é **um único closure** (`editor_panel`, `app.rs:6706-6732`)
chamado para os dois lados; só muda `Panel::left("editor_esq")` ou `Panel::right("editor")`
(`app.rs:6736`, `app.rs:6756`). Não há ramo por lado no código da roda, então a diferença **não vem do
lado**, e sim do **conteúdo** exibido: os campos que usam `expr` (`editor.rs:154-166`) têm a roda ligada;
campos que não têm a chamada continuam sem roda:

- lista de restrições do sketch e "Avançado" (`sketch_ui.rs:2620-2642`, `TextEdit` sem `wheel_text`);
- tabela de parâmetros (`editor.rs:1098-1102`, a coluna "Expressão" é um `TextEdit` puro);
- todos os `Slider` de `app.rs:7204-7496`, `asm_ui.rs:1855`, `asm_ui.rs:1868`, `asm_ui.rs:2543`;
- `DragValue` soltos em `asm_ui.rs:1924`, `dwg_ui.rs:1438`, `dwg_ui.rs:1780`, `geo_ui.rs:1241`.

Em resumo, ficou a cargo do desenvolvedor lembrar, em 44 pontos de uso. Não consegui reproduzir a diferença
em execução; a explicação acima é a única consistente com o código, e o teste é simples: abrir o mesmo comando
nos dois lados e conferir que o resultado é idêntico (será o aceite do item novo de campo padrão).

### 2.2 Como as referências tratam o campo

O Blender é a referência mais completa e documentada: arrastar horizontal muda o valor; Ctrl imanta a
passos; Shift refina; Ctrl + roda sobre o campo muda em passos; clique ou Enter digita; expressões
numéricas são avaliadas; unidades podem ser digitadas ("2 cm" num documento em milímetros); prefixo `#`
cria um vínculo. A roda exige Ctrl **de propósito**, para não alterar valores ao rolar o painel. Os CADs
comerciais combinam campo com unidade, aceitação de expressões e parâmetros nomeados, e mostram o valor
resolvido ao lado quando há expressão.

### 2.3 Componente `campo_num` (proposta)

Um único widget, `campo_num(ui, meta, &mut valor)`, que **todos** os painéis usam; chamar `DragValue` ou
`TextEdit` para números fica proibido por teste (busca por texto no CI). A `meta` é um registro:
`id` estável, rótulo, unidade (comprimento, ângulo, número, adimensional), faixa, passo base, dica, efeito
(figura opcional) e se aceita expressão.

Comportamento:

- **Roda:** com o mouse sobre o campo, altera o valor em passos (Shift ÷10, Ctrl ×10, **igual ao 3D**). Para
  não perder a rolagem do painel, vale a regra: a roda age no campo quando um comando está aberto ou o campo
  tem foco; em painel de configuração, exige Ctrl. É uma opção em Configurações com os três modos (sempre
  sobre o campo, só com Ctrl, só com foco). O campo pairado ganha borda de acento e a dica diz "roda altera".
- **Arrasto:** arrastar no rótulo ou no campo ("scrub"); cursor de setas horizontais; velocidade proporcional
  à ordem de grandeza do valor.
- **Texto:** clique entra em edição com tudo selecionado; Enter confirma, Esc volta ao valor anterior, Tab
  segue ao campo seguinte.
- **Expressão e unidade:** `L/2 + 5`, `2*pi*r`, `2 in`, `15 cm`; o valor resolvido aparece em cinza ao lado
  (já existe em `editor.rs:168-170`); erro em vermelho com a causa (`editor.rs:175+`). Parâmetros mostram uma
  etiqueta com o nome e botão para desvincular.
- **Atalhos de campo:** botão do meio ou Backspace com o mouse em cima restaura o padrão; Ctrl+C/V copia
  valor; duplo clique na seta do rótulo inverte o sinal.
- **Dica por campo:** passar o mouse **no rótulo** mostra três linhas: o que faz, faixa típica e efeito
  ("aumenta a espessura para dentro da peça"). A dica genérica de expressão (`editor.rs:146`) passa a ser a
  última linha, não a única.
- **Estados:** válido, expressão avaliada, erro, travado por parâmetro, calculando.
- **Separador decimal** do idioma (`ui_ext.rs:69-71`), mantido.

Teste automático: percorrer todos os campos registrados e exigir dica não vazia e traduzida (regra do
projeto: texto novo com tradução em `i18n.rs`).

## 3. Tooltip em camadas com GIF

### 3.1 Estado atual

`hover_help` (`icons.rs:403-430`) dá o texto breve ao pairar e o avançado após **5 s** parado, sem figura.
`tool_button` (`icons.rs:372`) desenha ícone e nome. Cada `Item` tem `icon`, `name`, `brief`, `advanced`
(`toolbar.rs:50-56`), sem campo de mídia nem de atalho. O B60 prevê 500 ms e ~1,2 s (`backlog.md:53`).

### 3.2 Desenho em três camadas

| Camada | Quando | Conteúdo |
|---|---|---|
| 1 | 400 a 500 ms | nome, atalho **atual do mapa** (`keymap.rs:10`), frase de uma linha |
| 2 | ~1,2 s ou Shift | GIF de laço 4 a 8 s mostrando entrada → operação → resultado, mais três passos em texto |
| 3 | tecla F1 sobre o botão | abre a página de ajuda do comando (mesmo texto de `Comandos/`), com campos e armadilhas |

O atraso é configurável (0 desliga a camada 2). Passar o mouse também escreve a frase na barra de status.

### 3.3 Gerar os GIFs automaticamente com o próprio app

O app já tem o gancho: variáveis `FENKRA_DEMO*` abrem exemplos, fixam câmera e tema e **não gravam**
preferências (`docs/desenvolvimento.md:50-61`, `app.rs:355-362`). Falta (a) um roteiro e (b) captura.

1. **Roteiro por comando** (`docs/media/roteiros/<id>.json`): lista de passos com tempo — abrir o exemplo,
   fixar a câmera, escolher o suporte, abrir o comando, animar um campo de A para B durante N segundos,
   concluir. É um formato pequeno, versionado e revisável. O mesmo formato alimentará os tutoriais
   interativos (seção 7).
2. **Execução determinística:** `FENKRA_DEMO_GRAVAR=<id>` roda o roteiro com passo de tempo fixo (1/12 s),
   tamanho de janela fixo e tema fixo; a cada quadro pede captura ao eframe
   (`ViewportCommand::Screenshot` e o evento de retorno) e grava PNG em pasta temporária. Sem relógio real,
   o resultado é reproduzível byte a byte.
3. **Montagem fora do binário:** um script em Python (`scripts/media/`) junta os quadros em GIF com paleta
   reduzida (64 cores, tramado leve) ou em WebP animado, corta a área útil e escreve um `manifesto.json`.
   Esse script **não** vai no executável.
4. **Cache:** o manifesto guarda hash do roteiro + versão do kernel; o CI regera só o que mudou, e um teste falha se um comando da barra não tiver mídia.

Orçamento: 360 × 220 px, 12 quadros por segundo, 4 a 8 s, 64 cores → alvo de **80 a 150 KB** por mídia.
Com ~200 comandos, 15 a 30 MB em disco.

### 3.4 Carga no egui sem pesar o binário

- **Não embutir.** Os ícones já entram no executável por `include_image!` (`icons.rs:365`; a pasta pesa
  1,6 MB). As mídias vão num pacote `ajuda.pack` (zip sem compressão extra) ao lado do executável, aberto sob
  demanda; o instalador oferece versão com e sem pacote. O binário não cresce.
- **Decodificar só ao pairar.** `egui_extras` já está com `svg` e `image` (`crates/fenkra-app/Cargo.toml:24`);
  o carregador de GIF e o de WebP são recursos opcionais do mesmo crate, a ligar quando se medir o custo.
  Lê-se os bytes do pacote e entrega-se com `Image::from_bytes` sob um endereço `bytes://ajuda/<id>`.
- **Plano B leve:** tira de quadros em um único PNG indexado + `fps` no manifesto; o app escolhe o quadro
  pelo tempo e recorta por UV. Um só `Texture`, sem decodificador de animação, e permite pausar e andar quadro
  a quadro. Recomendo medir os dois e ficar com o mais barato.
- **Cache LRU** de 16 mídias; textura liberada ao sair do botão.

## 4. Menu radial de acesso rápido

### 4.1 Referências

- **Menu de pizza do Blender:** aberto por uma tecla; os itens ficam em círculo ao redor do ponteiro; a
  seleção rápida é "tecla, mover um pouco, soltar"; soltar sem mover mantém o menu aberto para clicar; um
  disco central mostra a direção atual e só aceita seleção quando o mouse toca ou passa dele; itens aceitam
  letra sublinhada e número; submenus são indicados por um sinal de mais. O conteúdo é editável por quem
  escreve o menu, e o usuário pode trocar a tecla no mapa.
- **Menu de marcação dos CADs avançados:** acionado pelo botão direito, com oito setores; em modo gesto,
  basta traçar e soltar, sem o menu aparecer (memória muscular). Os comandos mudam por ambiente (sketch, peça,
  montagem) e o usuário edita setor por setor, com restaurar padrão, exportar e importar, e escolha entre
  texto, ícone ou ambos.

### 4.2 Hoje no Fenkra

Não há radial. O clique direito abre `viewport_menu` por tipo de elemento (`app.rs:4422`,
`interface-e-arvore.md:504-512`); o B41 do usuário cita "menu rápido" como teste pendente (`backlog.md:88`). O mapa de atalhos tem 38
ações (`keymap.rs:10-59`) e sua tecla de menu de vista é Espaço (`interface-e-arvore.md:547`).

### 4.3 Desenho proposto: "Roda de comandos"

- **Acionamento:** tecla configurável (padrão **Shift+W**, como pediu o usuário), registrada como nova
  `KeyAction::RodaComandos` no mapa; detecção de conflito já existe (`keymap.rs:301`). Segurar abre; soltar
  executa o setor sob a direção; toque curto fixa o menu aberto. Suprimida quando há campo de texto com foco
  (`egui_wants_keyboard_input`, `app.rs:6495`).
- **Geometria:** anel de 8 setores (cada setor de 45°, com 8° de folga), raio interno 44 px (zona morta com
  o disco central), raio externo ~150 px; o setor sob a direção cresce 12 %. Seleção por **ângulo** e não por
  alvo, assim o gesto de 60 px basta (modo gesto: sem desenhar o menu se soltar em menos de 150 ms).
- **Carrossel:** quando há mais de oito candidatos, a roda do mouse gira um segundo anel externo (páginas de
  oito); um ponto indica a página. Teclas 1 a 8 escolhem direto; Esc ou botão direito cancelam.
- **Conteúdo por contexto:** conjunto escolhido por (bancada, modo: sketch/peça/montagem/desenho, tipo de
  seleção). Cada setor tem ícone, nome e atalho. Exemplo no sketch com uma linha selecionada: Cota, Restrição
  horizontal, Offset, Aparar, Espelhar, Concluir sketch, Construção, Mais.
- **Próximos prováveis:** os dois primeiros setores são preenchidos por uma tabela local de transições
  "comando A → comando B" aprendida do próprio uso (contagem por par, com decaimento semanal), com padrão
  inicial vindo de um arquivo do projeto. Nada sai da máquina e o cálculo é uma busca em tabela, sem custo.
- **Edição pelo usuário:** em Configurações → Roda de comandos, um círculo desenhado com os oito setores; arrasta-se
  um comando do catálogo (a busca da seção 7 serve de lista) para um setor; cada contexto tem "restaurar",
  "exportar" e "importar"; escolher texto, ícone ou ambos. Os ids dos comandos serão os estáveis do B60
  (`toolbar.rs:66` hoje usa `ícone|nome`, chave frágil).
- **Custo:** egui desenha tudo com `Painter` num `Area` de ordem `Foreground` (como as janelas de edição já
  fazem em `sketch_ui.rs:2415`); a seleção por ângulo é um `atan2`. Meta: abrir em menos de 80 ms.

## 5. Ícones

### 5.1 Diagnóstico do conjunto

Existem 356 arquivos em `assets/icons`, cada um um SVG de 48 × 48 e 600 a 800 bytes, carregados por um `match`
gigante (`icons.rs:7-365`) com o ícone de configurações como silêncio de erro quando o nome não existe
(`icons.rs:365`). Os problemas de linguagem visual, vistos nos arquivos:

- **Uma única matiz.** Todos usam o mesmo azul de traço e três degradês azuis; 167 arquivos usam o degradê
  claro de preenchimento. Resultado: tudo parece igual na barra.
- **Mesmo cubo isométrico em muitos ícones** (estilo, exemplo, espessura): a única diferença é a altura de
  uma face.
- **Variantes quase idênticas** em famílias de seis ou mais (furo_*, ext_ate_*, chanfro_*, filete_*) sem
  um sinal de diferença legível em 24 px.
- **Metáforas de interface sem objeto** (exibicao, coletor, conexao, parametro) que não mostram o resultado.
- **Duplicidades:** pares como prop_minima e prop_tangente têm o mesmo traço principal.

### 5.2 Regras para um ícone que diz o que faz (e é original)

1. **Mostre o resultado, não a ferramenta.** Entrada em cinza, mudança em acento, resultado em azul pleno. Um
   filete mostra a quina já arredondada, um furo mostra a boca e a profundidade.
2. **Base + selo.** Famílias compartilham o desenho-base e diferem por um selo de 10 px no canto (mais, menos,
   seta, cadeado, ordem). O selo deve ser legível sozinho em 16 px.
3. **Paleta semântica**, com poucas cores: azul = geometria existente, laranja = o que muda ou é criado,
   cinza = referência/construção, vermelho = remoção, verde apenas para confirmação. Nada de degradê em
   ícone de 16 px.
4. **Grade de 48 com traço de 2,5 px**, cantos arredondados de 2 px, sem texto; duas versões por ícone: 16 px
   (formas cheias, 3 elementos no máximo) e 32 px (detalhe).
5. **Teste de 1 segundo:** mostrar o ícone sem nome a cinco pessoas; a meta é 4 acertos de 5 no grupo
   (verbo ou resultado). Quem falha sai da lista de aprovados.
6. **Originalidade:** desenhar do zero a partir do glossário (`docs/produto/glossario.md`), sem copiar
   forma de ícone de outro produto; manter um registro do rascunho de cada ícone novo.
7. **Um nome, um ícone.** Teste que falha se algum `Item.icon` cair no fallback ou se dois comandos usarem o
   mesmo arquivo sem serem a mesma família (`interface-e-arvore.md:556`).

### 5.3 Os 30 piores ícones hoje

Critério: ambiguidade do desenho, repetição de forma e distância entre o ícone e o que o comando faz.
Inspecionei o desenho de cerca de metade; o restante foi julgado pelo nome e pela família e deve ser
confirmado na revisão visual.

| # | Arquivo | Problema |
|---|---|---|
| 1 | exibicao.svg | retângulos dentro de retângulo, sem objeto reconhecível |
| 2 | coletor.svg | blocos empilhados que não dizem "coletor" |
| 3 | conexao.svg | curva entre duas retas, confunde com spline |
| 4 | parametro.svg | duas barras, parece menu ou ajuste |
| 5 | estilo.svg | cubo genérico |
| 6 | exemplo.svg | o mesmo cubo do estilo, com 2 px de diferença |
| 7 | espessura.svg | o mesmo cubo, sem indicar a medida |
| 8 | sk_offset.svg | dois quadrados concêntricos tracejados, parece escala ou moldura |
| 9 | escala.svg | quadrado dentro de quadrado com seta, difícil separar de xf_escala |
| 10 | xf_escala.svg | provável duplicata do anterior (confirmar) |
| 11 | prop_minima.svg | quase idêntico ao prop_tangente |
| 12 | prop_tangente.svg | idem; a tangente não aparece |
| 13 | saida.svg | trapézio com arco, sem sentido claro |
| 14 | canto.svg | arco entre duas retas, não diz o tipo de canto |
| 15 | sk_converter.svg | duas linhas com seta curta, sem a ideia de "virar construção" |
| 16 | sk_snap.svg | grade fina e círculo, parece "grade" e não "imã" |
| 17 | sk_rasgo.svg | rasgo e ranhura.svg são o mesmo desenho |
| 18 | ranhura.svg | idem (confirmar) |
| 19 | wf_repetir.svg | curva de pontos, confunde com spline_controle e ponto_curva |
| 20 | spine.svg | três faces inclinadas; ninguém sabe que é "espinha" |
| 21 | spline.svg | par próximo de spline_controle e spine |
| 22 | ortho.svg | quadrado + bloco inclinado, não lembra "ortogonal" |
| 23 | furo_simples.svg | mesma silhueta de furo_cego e furo_passante |
| 24 | furo_cego.svg | sem profundidade visível |
| 25 | furo_rebaixado.svg | difícil separar de furo_escareado |
| 26 | furo_escareado.svg | idem; sem o cone |
| 27 | chanfro_dist.svg | indistinguível de chanfro_dist_dist em 24 px |
| 28 | chanfro_dist_angulo.svg | idem; o ângulo não aparece |
| 29 | cont_g0.svg / cont_g1.svg / cont_g2.svg | três níveis de continuidade sem a diferença que importa (curvatura) |
| 30 | ext_ate_face.svg / ext_ate_ultimo.svg | família de cinco sem a face-alvo destacada |

Sem cobertura de ícone para a roda de comandos, o menu radial e a busca: eles dependem de um conjunto limpo
de 16 px, então o redesenho precede aquele trabalho.

## 6. Identidade visual

### 6.1 Layout familiar

O desenho que os profissionais reconhecem: faixa de comandos no topo por bancada, árvore à esquerda,
painel de propriedades junto da árvore (ou à direita), viewport com cubo de vistas no canto superior direito,
barra de status embaixo com dicas e coordenadas. O Fenkra já tem quase tudo: faixa superior com rolagem
horizontal (`app.rs:2670-2690`), barras laterais (`app.rs:2724`, `app.rs:2737`), árvore e painel
(`app.rs:6692-6757`), tela inicial, personalização de grupos (`toolbar.rs:66-91`). Falta uma barra de acesso rápido fixa (salvar, desfazer, refazer) e a barra de status como lugar de "próximo passo" e dica do campo.

### 6.2 O que torna o Fenkra original

- **Linguagem de forma própria:** cantos de 6 px, filetes de 1 px, acento único por tema, sem sombra
  pesada. O acento passa a ser usado de modo funcional (comando ativo, valor editável, seleção) e nunca
  decorativo.
- **Semântica de cor constante** em ícones, alças e prévias (seção 5.2): o usuário aprende a cor uma vez.
- **Alças e rótulos com desenho próprio**, não a imitação do gizmo de outro produto: pega grande, linha-guia
  pontilhada, rótulo em cápsula com a unidade em cinza.
- **Nomes de temas e de estilos de navegação sem marcas.** Há um tema embutido (`theme.rs:34-42`) cujo nome pode
  lembrar produto de terceiros, e variantes de `NavStyle` e de `Preset` que levam nome de produtos
  proprietários (`settings.rs:188-200`, `keymap.rs`, ver `interface-e-arvore.md:543`). A regra do projeto
  proíbe marca na interface; trocar por nomes funcionais ("estilo tipo-A", "orbitar com botão do meio") é item
  de revisão obrigatória.

### 6.3 Densidade e temas

Três densidades (compacta 24 px sem rótulo, padrão 32, confortável 40) ligadas ao `icon_size` (`app.rs:7321`); sete temas embutidos (`theme.rs:34-42`) pedem teste automático de contraste (4,5:1); rótulos 3D usam fonte fixa de 13 px (`handles.rs:380`) e devem seguir a escala de interface.

## 7. Onboarding

### 7.1 Estado atual

Tela inicial com exemplos testados (`app.rs:1007`, `docs/desenvolvimento.md:44`); sem tutorial interno e sem busca de comandos (B60, `backlog.md:53`).

### 7.2 Proposta

1. **Busca de comandos (Ctrl+K):** um campo no topo que filtra por nome, sinônimo em português e inglês e
   bancada; cada resultado mostra ícone, atalho, a camada 2 do tooltip (GIF) e, ao executar, troca de
   bancada se preciso. É o catálogo que alimenta o editor da roda de comandos. Aceite: "fur" acha Furo,
   "fillet" acha Filete.
2. **Lições interativas** reaproveitando o roteiro da seção 3.3: cada lição é um roteiro com *verificações*
   ("o documento tem uma extrusão de 20 mm"), destaque em halo sobre o botão ou a alça esperada, e botão
   "mostre-me" que executa o passo. O progresso fica nas preferências. Lições iniciais: câmera e seleção,
   primeiro sketch cotado, extrusão, furo e padrão, primeira montagem.
3. **Primeira execução:** escolher estilo de navegação e densidade com prévia ao vivo.
4. **Ajuda por tecla (F1):** abre a página do comando sob o mouse; o conteúdo vem dos mesmos textos de
   `Comandos/`, copiados para `docs/ajuda/` para que viajem com o produto.
5. **Cartões de próximo passo** na barra de status (já previstos no B20): "falta: escolher o perfil".

## CHECKLIST PADRÃO (qualquer comando)

Critério objetivo; marque cada linha com ok, parcial ou falta.

| Item | ok | parcial | falta |
|---|---|---|---|
| **Intuitivo** | alguém que nunca viu o comando completa a tarefa de exemplo em até 60 s sem ajuda | completa, mas precisa abrir a ajuda ou erra um campo | não completa ou precisa de explicação externa |
| **Prévia 3D** | resultado translúcido aparece sozinho após o primeiro suporte válido e atualiza a cada campo em < 300 ms; erro mostra causa | prévia só após confirmar um campo ou só em alguns modos | sem prévia; resultado só após Concluir |
| **Manipulação 3D** | cada grandeza numérica tem a alça do tipo certo (seta, anel, plano), rótulo editável que arrasta, aceita roda e expressão, passo adaptativo | só seta ou só rótulo, passo fixo ou sem expressão | nenhuma alça; só o painel |
| **Ícone claro** | passa no teste de 1 segundo (4 de 5), tem versão de 16 e 32 px, distingue-se da família por selo | reconhecido por 2 a 3 de 5 ou sem versão de 16 px | menos de 2 de 5, repete outro ícone ou cai no fallback |
| **Tooltip com GIF** | três camadas; GIF de 4 a 8 s gerado pelo roteiro, ≤ 150 KB; atalho atual; F1 abre a ajuda | só texto em duas camadas, ou GIF desatualizado | só o nome |
| **Dica por campo** | cada campo tem dica de 3 linhas traduzida (o que faz, faixa, efeito) e roda igual em ambos os painéis | dica genérica de expressão ou só em parte dos campos | campos sem dica |
| **Fluxo** | Suporte → Modo → Parâmetros → Avançado; campo ativo destacado; Enter/Esc; barra de status diz o que falta; Esc não deixa passo de desfazer | ordem parcial ou sem "o que falta" | formulário sem ordem, sem Enter/Esc ou com passo de desfazer residual |
