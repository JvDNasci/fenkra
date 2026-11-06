# ADR 0069: malha de tela separada da malha de trabalho

Data: 2026-11-06. Item: B346, fecha B10. Liga com B61 (presets de malha), B125 e ADR 0053 (cópias da
montagem), B236 (modo leve), B34 e B180 (análises de superfície).

## Contexto

Em malha grossa a peça mostrava facetas: silhueta de polígono, listras da zebra quebradas nas arestas
dos triângulos, isofotas em zigue-zague e, no mapa de curvatura, a forma da malha na cor.

As hipóteses foram conferidas uma a uma:

| Hipótese | Resultado |
|---|---|
| Normal interpolada sem renormalizar no fragmento | Falsa: `fs_mesh` já renormaliza |
| Iluminação por vértice | Falsa: tudo é por pixel |
| Zebra e isofotas por vértice | Falsa: já eram por pixel |
| Normal da superfície só nos vértices | Verdadeira, mas não é a causa principal (ver abaixo) |
| Normal errada nos polos | Verdadeira: no polo a derivada some e o nó ficava com a média de um triângulo |
| Curvatura indefinida nos polos | Verdadeira: virava zero, uma mancha de "plano" no topo da esfera |
| Cor da curvatura por vértice | Verdadeira: a escala é logarítmica e tem paradas, e interpolar a cor desenhava os triângulos |
| Triângulo longo em face curva | Verdadeira para a curvatura: o malhador só olha a flecha |
| Quebra de normal nas costuras | Falsa: cada lado lê a normal exata |
| Reflexo sem filtragem | Falsa: a listra já é suavizada por `fwidth` |
| Silhueta poligonal | Verdadeira: nenhum sombreamento corrige o contorno da geometria |

### A causa da zebra quebrada

O ponto central é contraintuitivo. Numa esfera ou num cilindro, a normal dos vértices interpolada e
renormalizada é exatamente a normal do ponto radial correspondente: trocar por uma normal mais exata não
muda nada. O que entorta a listra é o lugar do pixel. Ele cai na corda do triângulo, não na superfície, e
o raio de visão chega à corda num ponto diferente de onde chegaria à superfície. Esse deslocamento é
contínuo por triângulo e muda de direção na aresta: a listra dobra ali.

Medido num corte de cilindro, com raios paralelos até 70° de incidência, comparando o reflexo da malha
com o do círculo exato. O período da listra é de cerca de 6,4° de reflexo:

| Ângulo entre vértices | Erro da normal | Erro do reflexo | Fração do período |
|---|---|---|---|
| 45° | 4,8° | 9,7° | 151% |
| 30° | 3,5° | 7,0° | 110% |
| 20° (qualidade Normal) | 1,5° | 3,1° | 48% |
| 12° (qualidade Alta) | 0,85° | 1,7° | 27% |
| 10° | 0,60° | 1,2° | 19% |
| 8° | 0,32° | 0,64° | 10% |
| 6° | 0,18° | 0,35° | 5,5% |
| 5° | 0,15° | 0,30° | 4,7% |
| 4° | 0,09° | 0,17° | 2,7% |

O erro cresce com o quadrado do ângulo. Mesmo a qualidade Alta deixa a listra errada em um quarto do
período, então nenhum ajuste de sombreamento resolve: a geometria desenhada precisa chegar perto da
superfície.

## Decisão

1. **Duas malhas.** A malha de trabalho continua seguindo a qualidade e continua sendo a da seleção, das
   medidas, do desenho e das chaves de face. A malha de tela segue a câmera e só entra na cena. Assim a
   seleção não fica mais lenta com malha fina e nada do que se mede muda com o zoom.
2. **Erro em pixels.** A malha de tela pede flecha de meio pixel, em degraus de potência de 2: zoom
   pequeno não refaz nada e órbita nunca refaz. O ângulo é 10° no sombreado e 5° nas análises, o primeiro
   valor da tabela abaixo de 5% do período.
3. **Cópia da forma.** `tessellate_view` malha uma cópia sem triângulos, com a mesma ordem de faces e
   arestas. A malha guardada na forma, reaproveitada entre edições, fica como estava.
4. **Fora do caminho da edição.** O pedido parte da vista quando a câmera para e o cálculo termina, e
   anda pela mesma fila do cálculo, com a mesma geração: um pedido novo cancela o anterior. Editar não
   tessela. O corpo que mudou volta à malha de trabalho até a vista pedir de novo, e o resto segue polido.
5. **Mapas por pixel.** A curvatura vai como atributo do vértice, no lugar da cor, e a escala é aplicada
   no fragmento. `MeshVertex` não muda de tamanho. A inclinação sai da normal por pixel, com a troca de
   faixa suavizada.
6. **Curvatura onde ela muda.** No mapa de curvatura, cada aresta interna é conferida no meio, na
   superfície exata, e dividida se a interpolação errar mais de 3%. A borda da face não é tocada: é o que
   garante que nenhuma fresta se abre. A divisão é por aresta, então os dois triângulos recebem o mesmo nó.
7. **Limites.** Teto de 1,5 milhão de triângulos por peça, 300 mil no modo leve, repartidos pela parte
   curva de cada corpo; passando disso o pedido alarga, nunca além da malha de trabalho. No modo leve só as
   análises pedem malha, com flecha de 1,5 pixel.
8. **Montagem não muda.** As cópias continuam com o nível pelo erro na tela (ADR 0053). Nenhum byte a
   mais por vértice nem por cópia. Os mapas por pixel valem na montagem pelo caminho sem cópias, que já era
   o do mapa de curvatura.
9. **Forma livre.** A gaiola não tem B-rep: afinar é dar mais passos da subdivisão, que converge para a
   mesma superfície limite. A curvatura é estimada do operador de forma de cada triângulo, pela variação
   das normais nas arestas.

Fica de fora da malha de tela o que pinta sobre a malha de trabalho: a análise de fabricação, o mapa de
desvio e a prévia dos comandos. A malha fina ficaria por cima dessas pinturas. Nesses casos a cena volta à
malha de trabalho.

## Medidas

GeForce GTX 1060, janela de 1920 x 1080, 4 amostras de anti-serrilhamento, câmera girando sem parar. "Placa" é
a mediana do tempo até a placa terminar o quadro. "Pedido" é o maior tempo do cálculo com a malha de tela,
fora da thread da interface. "Cena" é o maior tempo de montar a cena na CPU. A malha de trabalho é a Normal.

| Peça, modo | Triângulos de trabalho | Triângulos de tela | Placa sem | Placa com | Pedido | Cena |
|---|---|---|---|---|---|---|
| Toro, zebra | 3.312 | 41.472 | 0,45 ms | 0,51 ms | 357 ms | 2,8 ms |
| Painel de dupla curvatura, zebra | não contado | 28.422 | 0,45 ms | 0,51 ms | 886 ms | 0,5 ms |
| Bloco com filetes, zebra | 610 | 6.266 | 0,52 ms | 0,55 ms | 32 ms | 0,6 ms |
| Placa paramétrica, zebra | 14.120 | 204.064 | 0,53 ms | 0,89 ms | 229 ms | 12,7 ms |

O custo na placa fica abaixo de meio milissegundo mesmo com 14 vezes mais triângulos. O tempo de CPU do
quadro não mudou. Máquina com placa integrada não foi medida.

A cor do mapa por pixel confere com a antiga, calculada na CPU: nos pontos comparados em esfera, cilindro e
toro a diferença máxima foi de 1 em 255.

Os números da curvatura conferem com o valor exato nos testes do kernel: esfera de raio 40 dá 1/40 nas
duas direções em todos os nós, polos incluídos, com erro abaixo de 1e-5; cilindro de raio 30 dá 1/30 e 0,
com erro abaixo de 1e-6; no toro, o nó novo do refino confere com a fórmula exata.

## Alternativas descartadas

- **Normal e curvatura em textura por face, em grade UV.** Foi a candidata inicial. Não resolve a zebra:
  a tabela acima mostra que a normal interpolada já é a certa, e o erro é do lugar do pixel. Também não
  corrige a silhueta, gasta um atributo a mais por vértice nas cópias e, nas peças comuns, cheias de faces
  pequenas, ocupa mais memória que a malha que substituiria.
- **Raio contra a superfície exata no fragmento.** Acerta o lugar do pixel, mas precisa da geometria da
  face na placa, com passos de Newton por pixel, e ainda deixa a silhueta poligonal. Custo alto para PC
  fraco e duas implementações da mesma superfície.
- **Refinar por triângulos curvos na própria cena.** Barato, sem kernel, mas aproxima a superfície em vez
  de lê-la e abre frestas nas arestas vivas, onde as normais dos dois lados são diferentes.
- **Trocar a qualidade da malha de trabalho conforme o zoom.** É uma linha de código, mas a seleção anda
  triângulo a triângulo a cada movimento do mouse e tudo o que lê a malha passaria a depender do zoom.

## Consequências

- A qualidade da malha nas configurações passa a decidir a malha de trabalho e o que aparece enquanto a
  malha de tela não chega; a tela fica polida em qualquer ajuste.
- Zoom muito grande numa peça grande para no teto de triângulos: a malha é da peça inteira, não só do que
  aparece. Refinar por face visível exigiria malhar faces soltas e reabriria o problema das frestas.
- A troca de malha ao parar a câmera aparece como uma pequena melhora, não como um salto, porque a flecha
  da malha de trabalho já é pequena na vista enquadrada.
