# ADR 0088: prévia das vistas por mapa de profundidade, com as arestas exatas (B393)

## Contexto

Depois dos ADR 0080 e 0085, a primeira abertura do desenho do kart (84 componentes, quatro vistas) mostrava a
prévia em cerca de 16 s e as vistas exatas de 67 s a 4 min. A prévia vinha da projeção por malha do
OpenCASCADE (`HLRBRep_PolyAlgo`): para cada vista, uma cópia profunda da forma inteira, a malha refeita nela e
o algoritmo de linhas ocultas sobre todos os triângulos. As curvas saíam como polígonos da malha.

A meta do usuário é ver as vistas em segundos.

## Decisão

1. **A prévia sai de um mapa de profundidade** (`fenkra-desenho/src/hlr_rapida.rs`). A malha da forma é feita
   uma vez por forma distinta (o malhador já usa várias threads) e cada vista roda numa thread:
   - os triângulos são projetados na direção da vista e rasterizados num mapa de até 3000 pixels no lado
     maior, guardando em cada pixel a superfície mais perto do observador (faixas de linhas em threads);
   - cada aresta da malha (as arestas do B-Rep, já discretizadas pelo kernel) e cada lado de silhueta (o lado
     comum a dois triângulos da mesma face curva, um de frente e outro de costas) é amostrada a cada pixel;
   - a amostra é visível se está na frente da superfície do pixel, com tolerância de dois pixels, um milésimo
     do tamanho e o dobro da variação de profundidade por pixel ali (a menor dos dois vizinhos de cada eixo:
     no degrau de uma borda só um lado salta; na face rasante, a silhueta de um cilindro, os dois);
   - amostra em pixel de fundo, rente à borda de uma peça, olha os quatro vizinhos: a aresta de trás que cai
     sobre o contorno da da frente fica oculta e não duplica a linha;
   - a aresta lisa entre duas superfícies livres (costura entre retalhos) não sai, como na HLR do kernel;
   - trechos seguidos com a mesma resposta viram uma polilinha nos tipos da HLR do kernel (0 aresta, 1
     tangente, 2 contorno, 3 oculta); tangente escondida não vira oculta, como no kernel.
2. **As linhas desenhadas são a geometria das arestas.** O pixel só decide onde a linha passa de visível a
   oculta, com erro de até um pixel: numa vista de 300 mm na folha, uns 0,1 mm.
3. **A exata continua depois**, nos processos do ADR 0080, e substitui a prévia; a exportação espera por ela.
   A vista achada no disco (ADR 0085) chega exata, sem prévia.
4. Sem estado global e sem OpenCASCADE na projeção: roda em threads, ao contrário da HLR exata.

## Conferência

`tests/hlr_rapida_b393.rs` compara com a HLR exata na placa de exemplo, nas vistas de frente, topo, direita e
isométrica: a caixa das linhas visíveis bate em 0,5 mm e o comprimento de linha visível em 8% (medido: 520
contra 525 mm, 1698 contra 1790, 328 contra 333, 1604 contra 1585). As linhas ocultas que coincidem com uma
visível (a aresta de baixo sob a de cima, na vista de topo) a exata desenha e esta não; na folha não se vê.

## Medição (kart, 84 componentes, quatro vistas, release, máquina livre, `medir_kart`)

| Caso | Antes (prévia pela malha do kernel) | Agora |
|---|---|---|
| Primeira abertura, nada no disco | prévia em ~16 s | prévia em 11,9 s (carga 5,8 s, malha 3,6 s, quatro vistas 0,5 s) |
| Formas das peças já no disco | não medido | prévia em 6,4 s (carga 0,5 s, malha 3,5 s) |
| Exata das quatro, nos processos | 67 a 84 s | 49 a 54 s (não mudou nesta decisão) |

Projetar as quatro vistas no mapa leva meio segundo; o que sobra da prévia é a carga das formas e a malha da
forma do conjunto (226 mil triângulos). As costuras entre os retalhos de uma superfície lisa (pneus,
para-choques, banco) não saem: a HLR do kernel só desenha tangentes de grau 1 (a borda de um filete).  A
malha só diz "lisa", então a aresta lisa entre duas superfícies livres (nenhuma analítica) conta como
costura e não sai; a borda de um filete encosta numa face analítica e continua como tangente. Sem isso a
prévia pintava a grade dos retalhos.

## Consequências

- A prévia agora mostra furos redondos (as arestas são as do B-Rep), não polígonos.
- Peça muito pequena perto de uma muito grande na mesma vista recebe poucos pixels: a troca entre visível e
  oculta fica mais grosseira nela. A exata corrige quando chega.
- O mapa ocupa até 36 MB por vista durante a projeção.
- Próximo passo para chegar a segundos na primeira abertura: guardar a malha da prévia junto das formas no
  disco, ou malhar as peças em paralelo antes de montar o conjunto.
