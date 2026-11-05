# Benchmark de detalhe — Sketch

> Compara cada comando da bancada com softwares de modelagem CAD avançados no nível de campo e de
> interação. Aprofunda [benchmark/sketch.md](../sketch.md) e [pesquisa/sketch.md](../../pesquisa/sketch.md):
> aquelas fichas tratam do que o comando faz; esta trata de como a pessoa o usa. Texto próprio, a
> partir de documentação pública. Fenkra auditado em alpha.26, lendo o código.

## Resumo: as 5 maiores lacunas de experiência

1. **Não dá para digitar a medida enquanto se desenha.** Nenhuma ferramenta de desenho tem campo de
   valor. A pessoa clica no olho e cota depois. Os CADs avançados e o FreeCAD abrem campos flutuantes
   ao lado do cursor: comprimento e ângulo, largura e altura, raio. É a maior diferença de ritmo.
2. **A prévia mente ou não existe.** Polígono, arcos e rasgo mostram círculos. Mover e Copiar mostram
   só as linhas. Espelhar, Offset, Filete, Chanfro, Aparar e Estender não mostram nada: o resultado
   aparece depois do clique.
3. **O comando não tem painel próprio.** O painel lateral mostra o estado do solver e um parágrafo.
   O único parâmetro de ferramenta do sketch inteiro é o número de lados do polígono. Não há modos,
   opções nem teclas que os troquem, então cada variante vira um botão ou simplesmente não existe.
4. **A ajuda chega tarde e sem imagem.** A explicação do tooltip leva 5 s para aparecer e é só texto.
   As dicas junto ao cursor faltam em Linha, Ponto, Selecionar e nas 11 restrições, e não passam
   pela tradução, embora ela exista.
5. **O sketch não mostra se vai funcionar.** Perfil fechado não ganha sombreamento, o sólido não é
   cortado no plano, os snaps não dizem o que encontraram e nada pode ser ocultado para limpar a
   tela. A pessoa só descobre que o perfil está aberto ao usar Checar ou ao falhar a extrusão.

## Como ler o checklist

- **Prévia 3D:** o resultado aparece na viewport, sobre o plano, antes de confirmar.
- **Manipulação 3D:** dá para ajustar na viewport por arraste, pega ou campo, durante ou depois.
- **Ícone claro:** avaliado pela definição vetorial em `scripts/gerar_icones.py`.
- **Tooltip:** avalia o texto (entradas, passos, resultado). Nenhum comando tem imagem ou GIF, e a
  parte longa sempre espera 5 s (`icons.rs:411`). Ao ativar a ferramenta, o mesmo texto aparece no
  painel (`sketch_ui.rs:2526`), o que compensa em parte.
- **Dica por campo:** cada entrada, clique ou valor, é explicada onde acontece.
- **Fluxo:** aceita seleção prévia, repete e termina de forma previsível.
- `—` significa que o critério não se aplica ao comando.

Linhas sem arquivo citado estão em `crates/fenkra-app/src/sketch_ui.rs`.

## Checklist por comando

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Novo sketch | parcial | parcial | falta | ok | parcial | parcial | parcial | Pede plano ou face por aviso (`app.rs:1456`); não antecipa origem nem H/V |
| Concluir | ok | — | — | ok | parcial | — | parcial | Esc passa por até 4 níveis (247), mas o texto diz "Esc de novo sai" (2459) |
| Selecionar | parcial | — | parcial | ok | ok | falta | parcial | Arco e corpo da spline não arrastam (1450); sem janela |
| Linha | parcial | parcial | parcial | ok | ok | falta | parcial | Mostra só comprimento (1545); sem ângulo nem valor digitado |
| Retângulo | parcial | ok | parcial | ok | parcial | parcial | parcial | Forma fiel (1658), sem largura e altura |
| Retângulo pelo centro | parcial | ok | parcial | ok | parcial | parcial | parcial | Idem (1659) |
| Círculo | parcial | ok | parcial | ok | parcial | parcial | parcial | Sem leitura de raio (1663) |
| Círculo 3P | parcial | parcial | parcial | ok | parcial | parcial | parcial | Prévia só depois do 2º clique (1664) |
| Arco 3P | parcial | parcial | parcial | ok | parcial | parcial | parcial | Prévia é o círculo inteiro (1664); ordem início, fim, meio |
| Arco pelo centro | falta | parcial | parcial | parcial | parcial | parcial | parcial | Círculo inteiro na prévia (1669); sentido anti-horário não aparece |
| Rasgo | parcial | parcial | parcial | ok | parcial | parcial | parcial | Dois círculos e uma linha (1672); a dica diz "largura", o valor é raio (1775) |
| Polígono | parcial | parcial | parcial | ok | parcial | parcial | parcial | Prévia é círculo (1663); "lados" sem dica nem tradução (2528) |
| Ponto | ok | parcial | ok | ok | parcial | falta | ok | Sem dica no cursor; sem coordenada digitada |
| Spline | parcial | ok | parcial | ok | ok | parcial | parcial | Curva real na prévia (1599); alças só pelo menu de contexto |
| Mover | parcial | parcial | falta | ok | parcial | parcial | parcial | Prévia só de linhas (1873); sem distância digitada |
| Copiar | parcial | parcial | falta | ok | parcial | parcial | parcial | Dica diz "mover" (2134); uma cópia por vez; restrições não vêm junto |
| Espelhar | parcial | falta | falta | ok | parcial | parcial | parcial | O espelho só aparece depois do clique no eixo; eixo só linha |
| Offset | parcial | falta | parcial | ok | parcial | parcial | falta | Cria antes de mostrar (1972); perde a seleção prévia (242) |
| Filete | falta | falta | parcial | ok | parcial | parcial | falta | Aceita só o ponto do canto (2066); clicar na linha não dá retorno |
| Chanfro | falta | falta | parcial | ok | parcial | parcial | falta | Mesma entrada do filete; distâncias sempre iguais |
| Aparar | parcial | falta | falta | ok | parcial | parcial | parcial | Só linhas; nada destaca o trecho antes do clique (1839) |
| Estender | parcial | falta | falta | ok | parcial | parcial | parcial | Só linhas; nada mostra até onde vai |
| Converter | parcial | parcial | falta | parcial | ok | parcial | parcial | Destaca e nomeia a origem (1822); o resultado só surge depois |
| Interseção | parcial | parcial | falta | ok | parcial | parcial | parcial | Destaca a origem, não a curva que vai nascer |
| Projetar (painel) | parcial | falta | falta | falta | falta | parcial | parcial | Lista de botões por nome, sem ver o que é cada um (903) |
| Construção (seleção) | parcial | — | — | falta | parcial | falta | parcial | Usa `sk_converter` (`toolbar.rs:527`), e Converter usa outro ícone |
| Cota inteligente | ok | ok | parcial | ok | ok | parcial | ok | Melhor comando da bancada; a cota segue o mouse (2156) |
| Cota horizontal | parcial | ok | parcial | ok | parcial | parcial | parcial | Herda a prévia da cota inteligente |
| Cota vertical | parcial | ok | parcial | ok | parcial | parcial | parcial | Idem |
| Cota de ângulo | parcial | ok | parcial | ok | parcial | parcial | parcial | Quadrante não escolhido pelo mouse |
| Cota de raio | parcial | ok | parcial | ok | parcial | parcial | parcial | Troca para diâmetro só pelo menu |
| Cota de diâmetro | parcial | ok | parcial | ok | parcial | parcial | parcial | Idem |
| Coincidente | parcial | falta | — | ok | parcial | falta | parcial | Sem dica no cursor (2226) nem contagem de seleção |
| Ponto médio | parcial | falta | — | ok | parcial | falta | parcial | Idem |
| Fixa | ok | falta | — | ok | parcial | falta | parcial | Idem |
| Horizontal | ok | falta | — | ok | parcial | falta | parcial | Selo com a letra H depende do idioma |
| Vertical | ok | falta | — | ok | parcial | falta | parcial | Idem |
| Paralela | parcial | falta | — | parcial | parcial | falta | parcial | Ícone são só duas linhas, sem marca de restrição |
| Perpendicular | parcial | falta | — | ok | parcial | falta | parcial | Sem dica no cursor |
| Tangente | parcial | falta | — | ok | parcial | falta | parcial | Tooltip não diz qual lado da tangência resulta |
| Concêntrica | parcial | falta | — | ok | parcial | falta | parcial | Sem dica no cursor |
| Igual | parcial | falta | — | ok | parcial | falta | parcial | Só aos pares; sem seleção múltipla |
| Simétrica | falta | falta | — | ok | parcial | falta | parcial | Pede 3 entidades (73) e nada diz a ordem |
| Checar | parcial | parcial | — | parcial | ok | — | parcial | Anéis e uma linha de texto (1262); não sombreia o perfil |
| Modo construção | ok | — | — | parcial | ok | — | ok | Ícone quase igual ao de Construção (seleção) |
| Grade | ok | — | — | parcial | ok | — | ok | Mesma base de desenho do ícone de Snaps |
| Snaps | parcial | — | — | parcial | parcial | — | ok | Só pontos, origem e grade (1327) |
| Auto restrições | parcial | — | — | parcial | ok | — | ok | Só H/V a 2,5° e coincidência |
| Auto cotas | parcial | — | — | falta | parcial | — | ok | Ícone é o da cota com um ponto no canto; desligado por padrão |

## Contagem do checklist

49 comandos, 343 células: **73 ok, 183 parcial, 53 falta** e 34 que não se aplicam.

| Critério | ok | parcial | falta | — |
|---|---|---|---|---|
| Intuitivo | 8 | 37 | 4 | 0 |
| Prévia 3D | 10 | 13 | 18 | 8 |
| Manipulação 3D | 1 | 20 | 9 | 19 |
| Ícone claro | 38 | 8 | 3 | 0 |
| Tooltip | 9 | 39 | 1 | 0 |
| Dica por campo | 0 | 27 | 15 | 7 |
| Fluxo | 7 | 39 | 3 | 0 |

## Por comando (os mais usados)

### Linha

**O que os outros têm.** Um CAD avançado abre dois campos no cursor depois do primeiro clique, com
comprimento e ângulo. Tab passa ao campo seguinte e o valor digitado trava o cursor, com um cadeado
visível. Enter fecha o segmento e cria as cotas. Outro CAD liga isso por opção, separa "digitar
define o comprimento" de "digitar também cria a cota" e aceita clicar e clicar ou clicar e arrastar.
O FreeCAD chama de parâmetros na vista e distingue campos de posição e de dimensão, com preferência
para mostrar nenhum, só dimensões ou ambos. O comando ainda oferece construção, linha infinita e
linha a partir do ponto médio.

**Como deveria ser no Fenkra.**
- Suporte: ponto inicial, com snap. Modo: Linha, Arco tangente a partir da ponta. Parâmetros:
  comprimento e ângulo. Avançado: construção, criar cota ao digitar.
- No 3D: segmento em prévia, rótulos de comprimento e de ângulo presos à linha, arco de ângulo a
  partir da horizontal, linha de inferência tracejada com o glifo H, V ou perpendicular antes do
  clique. Se o traço fechar o contorno, sombreia o perfil na hora.

**GIF (8 s).** Clique na origem; digite 40, Tab, 30, Enter; cadeado aparece; segmento nasce cotado;
mais dois cliques; o terceiro fecha no ponto inicial e o perfil fica azul.

### Retângulo e Retângulo pelo centro

**O que os outros têm.** No FreeCAD é um comando só, com quatro modos trocados pela tecla M: dois
cantos, centro e canto, três cantos e centro com dois cantos. Com a tecla U o retângulo nasce com
cantos arredondados, e com J ganha uma moldura deslocada. Largura e altura são digitáveis nos
campos da vista.

**Como deveria ser no Fenkra.**
- Unir `Rect` e `RectCenter` em um comando. Modo: 2 cantos, centro, 3 pontos (B32). Parâmetros:
  largura, altura. Avançado: raio de canto, moldura, construção.
- No 3D: retângulo em prévia com rótulo de largura e de altura nos lados e diagonais de construção
  quando for pelo centro. Cor de ponto preso quando o canto cair em um snap.

**GIF (7 s).** Clique no centro; mouse abre o retângulo com dois rótulos; digite 60, Tab, 30, Enter;
tecla U e arraste curto arredonda os cantos.

### Círculo e Arcos

**O que os outros têm.** O campo do círculo aceita raio ou diâmetro. Nos arcos, os campos variam por
ferramenta: distância, ângulo ou coordenada. O FreeCAD reúne modos e campos em uma seção no topo do
diálogo do sketcher.

**Como deveria ser no Fenkra.**
- Dois comandos com modo: Círculo (centro, 3 pontos, tangente a 3) e Arco (3 pontos, centro,
  tangente). Parâmetros: raio ou diâmetro, ângulo varrido.
- No 3D: desenhar só o arco que será criado, nunca o círculo inteiro. Hoje a prévia usa `circle()`
  nos três casos (1664, 1669, 1670), e no arco pelo centro a pessoa só descobre o lado depois do
  último clique. O sentido deve seguir o mouse, como no arco 3P.

**GIF (6 s).** Clique no centro e no início; o arco cresce com o mouse e mostra o ângulo; passar para
o outro lado inverte o sentido; clique termina.

### Polígono e Rasgo

**O que os outros têm.** Um CAD avançado aceita de 3 a 40 lados e escolhe círculo inscrito ou
circunscrito, deixado como construção. O Fenkra limita a 24 e só inscreve.

**Como deveria ser no Fenkra.**
- Polígono. Parâmetros: lados, inscrito ou circunscrito, valor do círculo, ângulo. Atalho de mais e
  menos lados com a prévia já aberta.
- Rasgo. Parâmetros: comprimento entre centros e largura total. A dica diz largura, mas o terceiro
  clique mede o raio a partir do segundo centro (1775), então o rasgo sai com o dobro do valor
  lido. Medir na perpendicular ao eixo e mostrar o contorno final, não dois círculos.

**GIF (6 s).** Polígono: clique no centro; hexágono real segue o mouse e gira; tecla de mais vira
octógono; clique fixa.

### Selecionar e arrastar

**O que os outros têm.** Arrastar geometria sub-restrita é a forma de ver o que está livre, e um CAD
avançado desenha glifos de graus de liberdade que mostram de que modo a geometria ainda pode se
mover, distintos dos glifos de restrição. Outro mantém uma barra de contexto com as relações
possíveis logo depois de selecionar, e ela continua aberta depois de aplicar uma relação para
permitir outra.

**Como deveria ser no Fenkra.** Arrastar arco por qualquer ponto e spline pela curva. Hoje o clique
nesses corpos não inicia arrasto nenhum (1450), o que parece travamento. Ao passar o mouse, mostrar
setas de grau de liberdade. Ao selecionar, abrir sem botão direito uma barra pequena com o que
`applicable()` já calcula (768).

**GIF (7 s).** Arrastar lado de retângulo sem cota: setas surgem, a forma muda; cotar; a seta some e
a cor muda para totalmente restrito.

### Cota inteligente

É o comando mais próximo do nível dos outros. As lacunas agora são finas. Veja a ficha 2.15 da
pesquisa para cota de referência, min/max e nomes.

**O que os outros têm a mais.** Arrastar para dentro ou para fora de um círculo troca entre raio e
diâmetro, e arrastar para um quadrante escolhe qual ângulo cotar. Passar o mouse pela linha de
construção alterna entre distância e cota de eixo. Valor negativo inverte o lado em comprimentos,
distâncias e ângulos. Cota que sobre-restringe vira referência sozinha, em cinza, e o menu de contexto
alterna entre comandar e referência.

**Como deveria ser no Fenkra.** Adotar esses gestos em `choose_dim` (586). O campo do valor deve
listar parâmetros ao digitar. "Novo valor (aceita expressão)" (2420) pode ganhar exemplo e
autocompletar.

**GIF (8 s).** Clicar no círculo; mouse para fora vira diâmetro, para dentro vira raio; clique
posiciona; digitar L/2 e Enter; círculo muda.

### Restrições

**O que os outros têm.** O caminho comum é selecionar antes e aplicar depois, pela barra de contexto.
Os glifos podem ser ocultados.

**Como deveria ser no Fenkra.**
- O comando sem seleção precisa de dica no cursor e contagem: "Simétrica: 1 de 3, escolha o 2º
  ponto". Hoje não há dica nenhuma (2226).
- Ao passar sobre o segundo elemento, mostrar a geometria já resolvida em fantasma. O Fenkra tem
  meios para isso: clonar `SketchData`, aplicar `apply_constraint`, resolver e desenhar sem gravar.
  Se conflitar, pintar de vermelho antes do clique.

**GIF (6 s).** Duas linhas soltas; Paralela; clicar na primeira; passar na segunda mostra o fantasma
já paralelo; clicar confirma e o glifo aparece.

### Aparar e Estender

**O que os outros têm.** Em dois CADs avançados, parar o cursor sobre o trecho mostra o resultado
antes do clique. Pressionar e arrastar apara tudo que o traço tocar. Shift segurado vira Estender
até soltar, e o menu de contexto troca para Dividir ou Estender. No FreeCAD o trecho acende, círculos
temporários marcam os pontos de corte, as pontas ganham ponto sobre objeto e as restrições passam
para o que sobra. A ferramenta segue ativa até clique no vazio, botão direito ou Esc.

**Como deveria ser no Fenkra.** B18 cobre a geometria. Falta nomear o que a pessoa vê: trecho a sair
em vermelho tracejado, pontos de corte marcados, ponta a estender em prévia até o alvo. Clicar em
curva não aceita hoje dá erro depois; deve mostrar cursor recusado antes.

**GIF (6 s).** Mouse sobre o trecho: fica vermelho; clique apaga; arraste atravessa três trechos;
Shift e clique estendem uma ponta.

### Offset

**O que os outros têm.** No FreeCAD os modos Arco e Interseção trocam com M, U apaga a original, J
liga a restrição de distância e a distância pode ser digitada. Um CAD avançado oferece corrente
contígua, os dois lados, inverter e tampa nas pontas. Outro tem botão de inverter, para evitar
distância negativa, e seleciona a cadeia com duplo clique.

**Como deveria ser no Fenkra.**
- Suporte: curva ou cadeia, aceitando seleção prévia. Modo: um lado, dois lados. Parâmetros:
  distância, com inverter. Avançado: tampa, construção na base, apagar original.
- No 3D: curva paralela em fantasma seguindo o mouse, seta de lado, rótulo de distância e pega para
  arrastar. Hoje o primeiro clique no vazio já cria a curva (1972) e só depois abre a cota.

**GIF (7 s).** Duplo clique no contorno; fantasma aparece por fora; mouse para dentro inverte;
digitar 5 e Enter.

### Filete e Chanfro

**O que os outros têm.** O FreeCAD aceita o ponto do canto ou duas arestas, troca entre filete e
chanfro com M e tem "preservar canto" (tecla U), que deixa um ponto com as restrições do canto
antigo. Um CAD avançado mantém o ponto virtual quando o canto tem cotas e faz filetes seguidos de
mesmo raio ficarem iguais ao primeiro, com uma só cota.

**Como deveria ser no Fenkra.**
- Unir num comando com modo Filete, Chanfro. Suporte: canto, duas curvas, ou janela com vários cantos.
  Parâmetros: raio; no chanfro, duas distâncias ou distância e ângulo. Avançado: manter canto.
- No 3D: ao passar sobre o canto, arco em prévia com o raio atual. Hoje clicar na linha não faz nada
  e não avisa, porque só `Ent::Pt` é tratado (2066), e o alvo do ponto tem 9 px.
- O raio inicial sai de uma conta sobre as linhas; deve repetir o último usado.

**GIF (6 s).** Mouse no canto: arco fantasma de 5 mm; clique; próximo canto já traz 5 mm; quatro
cliques, uma cota só.

### Mover, Copiar e Espelhar

**O que os outros têm.** Entrada de distância e opção de manter restrições (ficha 2.27).

**Como deveria ser no Fenkra.** Prévia de todas as curvas, não só linhas. O mesmo laço que desenha a
prévia da forma serve para círculo, arco e spline. Em Copiar, repetir até Esc e levar restrições
internas. Em Espelhar, passar sobre a linha mostra o resultado em fantasma. Corrigir a dica "curvas
a mover" em Copiar. B32 traz a entrada numérica.

**GIF (6 s).** Selecionar contorno; Espelhar; passar sobre o eixo mostra o fantasma; clique confirma
e surgem glifos de simetria.

### Converter e Interseção

Destaque e nome da origem no hover já estão bons. Falta mostrar a curva que vai nascer, e na
Interseção isso é o principal, porque a origem é uma face e o resultado é outra coisa. O nome
também precisa de decisão: o botão se chama Converter, o ícone é o de projetar e o ícone chamado
`sk_converter` está em outro comando. Um CAD avançado exige projetar antes de usar arestas de corpos
em comandos como Offset; no Fenkra o offset ainda não aceita cadeia, então vale o mesmo cuidado.

**GIF (6 s).** Mouse em aresta do sólido: aresta acende e surge fantasma no plano; clique cria;
editar o sólido e o sketch acompanha.

## Detalhes finos que os outros cuidam e o Fenkra ainda não

- **Paleta de sketch sempre aberta.** Um CAD avançado reúne opções do comando ativo, tipo de linha
  (construção, eixo), olhar de frente, grade, snap, fatiar, mostrar perfil, pontos, cotas, restrições,
  construção e projetadas, tudo em caixas de marcar.
- **Fatiar no plano.** Corta os corpos temporariamente onde encontram o plano do sketch. No Fenkra o
  sketch é pintado por cima de tudo, em overlay egui, sem profundidade: desenhar dentro de uma peça
  exige adivinhar.
- **Perfil sombreado.** Avisa em tempo real que o contorno fechou. Um tutorial oficial indica isso
  para achar folgas pequenas. O Fenkra já calcula `sk.loops()`.
- **Snap com nome.** Hoje um anel ou um ponto (1376). Precisa de glifo por tipo. B96 cobre os novos
  snaps; o glifo deve dizer o tipo.
- **Barra de atalhos.** Um CAD avançado abre com a tecla S uma barra personalizável no cursor.
- **Atalhos.** Só 9 ações de sketch têm tecla (`keymap.rs:225`). Aparar, Offset, Filete e Espelhar não
  têm. Os tooltips trazem a tecla escrita à mão, como `(X)`, e um dos perfis usa Q (`keymap.rs:261`).
- **Tradução.** `hint` vai direto ao `painter.text` sem `tr` (1814, 1851, 2144, 2222), assim como
  o texto do painel (2526), o status (2457, 2459) e "lados" (2528). Várias frases já estão em `i18n.rs`,
  mas o texto em inglês nunca aparece.
- **Ícones sem uso.** `sk_linha` e `sk_girar` existem em `icons.rs:291-294`, sem comando. `sk_girar`
  anuncia um comando que não existe.

## Modelo de painel proposto para todo comando de sketch

1. **Linha de estado:** comando, passo atual e o que falta ("2 de 3: escolha o fim do arco").
2. **Modo:** botões com miniatura; tecla M percorre, como no FreeCAD.
3. **Parâmetros:** os mesmos valores dos campos da viewport, ligados entre si. Tab passa adiante e
   Enter confirma.
4. **Avançado:** recolhido; construção, criar cota ao digitar, manter canto.
5. **Estado do sketch:** graus de liberdade e restrições do elemento, como hoje, abaixo do comando.

## O que não dá para ser 3D, e por quê

- **Lista de restrições e conflitos.** É uma relação entre muitos elementos, com busca e remoção. No
  3D ficam só os glifos e o destaque; a lista fica no painel.
- **Graus de liberdade e estado do solver.** São números de um sistema inteiro, não de uma curva. As
  setas locais ajudam, mas o total é texto.
- **Expressões e parâmetros.** Precisam de teclado, nomes e mensagens de erro. O campo pode flutuar
  na viewport; a edição longa fica no painel.
- **Relatório de Checar e reparo.** Marcas ficam no 3D; contagem e tolerância são tabela.
- **Modos e opções de política.** Associativo ou isolado, criar cota ao digitar: são decisões, não
  posições no espaço.
- **Prévia do sólido que o sketch vai gerar.** O sketch é plano e ainda não sabe a feature seguinte.
  Prévia real fica no comando de sólido.
- **Prévia de restrição em sketch grande.** Dá para fazer, mas depende de resolver a cada movimento.
  Com Jacobiano numérico (ficha 2.17) precisa de orçamento de tempo e queda para só destacar.

## Limites desta análise

- Auditoria por leitura de código; a aplicação não foi executada. B41 continua sendo o teste com mouse.
- Ícones avaliados pelas formas vetoriais, sem ver em tamanho real. Vale conferir na barra.
- Parte do que se diz dos outros vem de ajuda de versões antigas e de fóruns; os detalhes podem ter
  mudado. O comportamento do campo com vírgula para X e Y veio de fórum e ficou de fora.
- Não achei fonte que confirme prévia ao vivo no filete dos outros. A proposta é nossa.
- A wiki do FreeCAD recusou o acesso automático; li as páginas em espelho público.
