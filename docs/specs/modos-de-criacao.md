# Modos de criação de cada elemento

Levantamento dos modos usuais em CADs paramétricos, descritos com nossas palavras, para planejar
as ferramentas do Fenkra. Coluna **Fase** = quando pretendemos implementar (ver
[backlog](../backlog-alpha.md)).

## Ponto (2D no sketch e 3D na peça)

| Modo | Descrição | Fase |
|---|---|---|
| Coordenadas | X, Y (sketch) ou X, Y, Z (3D), aceitando expressões | a3 / a5 |
| Em vértice | coincide com um vértice existente | a4 |
| Sobre curva | em uma curva, por distância ao longo dela ou por razão (0–1) | a5 |
| Sobre face/superfície | por coordenadas UV ou projetando outro ponto | a5 |
| Ponto médio | entre dois pontos ou no meio de uma aresta | a3 / a5 |
| Centro | centro de círculo, arco ou esfera | a3 / a5 |
| Interseção | curva × curva, curva × plano/superfície | a5 |
| Projeção | ponto projetado em plano, face ou curva | a5 |
| Deslocamento | a partir de outro ponto por um vetor ou ao longo de uma direção | a5 |

## Linha / eixo 3D

| Modo | Descrição | Fase |
|---|---|---|
| Dois pontos | entre dois pontos quaisquer | a5 |
| Ponto + direção | por um ponto, na direção de um eixo, aresta ou normal de face | a5 |
| Normal a superfície | por um ponto, perpendicular a uma face | a5 |
| Tangente a curva | em um ponto da curva | a5 |
| Interseção de planos | linha comum a dois planos | a5 |
| Eixo de face cilíndrica/cônica | eixo de revolução da face | a4 |
| Bissetriz | entre duas linhas | a5 |

## Plano

| Modo | Descrição | Fase |
|---|---|---|
| Deslocado | paralelo a outro plano/face plana a uma distância | ✔ a1 |
| Três pontos | passando por três pontos | a5 |
| Ponto + normal | por um ponto, perpendicular a uma linha/aresta/eixo | a5 |
| Linha + ponto | contendo uma linha e um ponto | a5 |
| Ângulo | girado em torno de uma linha a partir de outro plano | a5 |
| Normal a curva | perpendicular a uma curva em um ponto dela (ótimo para varreduras) | a5 |
| Tangente a superfície | tangente a uma face curva em um ponto/linha | a5 |
| Plano médio | entre dois planos/faces paralelas | a5 |
| Por face plana | coincidente com uma face | a4 |
| Por duas linhas | contendo duas linhas coplanares | a5 |

## Linha e polilinha (sketch)

| Modo | Descrição | Fase |
|---|---|---|
| Dois cliques | ponto inicial e final | a3 |
| Polilinha contínua | cada clique começa o próximo segmento; fecha ao clicar no primeiro ponto | a3 |
| Tangente | segmento tangente ao fim de um arco | a3 |
| Horizontal/vertical automáticas | inferidas durante o desenho (restrições automáticas) | a3 |
| Construção | linha auxiliar que não forma perfil | ✔ a1 |
| Linha de centro | eixo de simetria/revolução | a3 |

## Círculo e arco

| Modo | Descrição | Fase |
|---|---|---|
| Centro + raio | clique no centro e no raio | a3 |
| Três pontos | círculo/arco por três pontos | a3 |
| Dois pontos (diâmetro) | os pontos são extremos do diâmetro | a3 |
| Tangente a 3 entidades | círculo tangente a três linhas/curvas | a5 |
| Arco centro + extremos | centro, início e fim | a3 |
| Arco tangente | começa tangente ao fim de uma linha/arco | a3 |
| Círculo 3D | centro + eixo + raio, ou três pontos 3D | a5 |
| Filete de sketch | arco tangente entre duas linhas, recortando-as | a3 |

## Curvas (spline)

| Modo | Descrição | Fase |
|---|---|---|
| Por pontos de passagem | interpola pontos clicados | a3 / a5 (3D) |
| Por polígono de controle | pontos de controle influenciam a forma | a5 |
| Com tangências nas pontas | tangência/curvatura nas extremidades | a5 |
| Interseção de superfícies | curva comum a duas superfícies | a5 |
| Projeção | curva projetada em face/superfície | a5 |
| Hélice / espiral | por passo, altura, voltas | a5 |
| Conector entre linhas 3D | curva de transição G1/G2 entre extremidades de duas curvas | a5 |

## Formas de sketch

| Forma | Modos | Fase |
|---|---|---|
| Retângulo | 2 cantos · centro + canto · 3 pontos (inclinado) | a3 |
| Polígono | centro + vértice, inscrito/circunscrito | a3 |
| Rasgo | centro-centro · total · arco | a3 |
| Elipse | centro + eixos | a5 |
| Texto | fonte, altura, ângulo | a5 |

## Restrições do sketch

Geométricas: coincidente, horizontal, vertical, paralela, perpendicular, tangente, igual, concêntrica,
ponto médio, simétrica, fixa, sobre curva, colinear.
Dimensionais (pela ferramenta de cotagem): distância, distância horizontal/vertical, comprimento,
raio, diâmetro, ângulo. Toda cota aceita expressão e parâmetros (globais ou publicados).

## Comandos de sólido (já planejados nos requisitos)

| Comando | Modos de término / variações |
|---|---|
| Extrusão / recorte | cego, simétrico, dois sentidos, até o próximo, até face/vértice, passante, com ângulo de saída, parede fina |
| Revolução | ângulo, simétrica, dois sentidos, até face |
| Furo | simples, rebaixado, escareado, roscado; por pontos de sketch ou clique em face |
| Filete | raio constante, variável, face-face, completo |
| Chanfro | distância, distância-distância, distância-ângulo |
| Booleanas | unir, subtrair, interseção entre coletores |
