# ADR 0103: peças repetidas de uma montagem dividem a geometria (B393)

## Contexto

O desenho de montagem do kart (84 componentes) levava 126 a 135 s para as quatro vistas. Depois dos ADR 0080,
0085 e 0089 a prévia saía em 6 a 12 s no kart, mas um conjunto sintético com peças de superfície livre
(engrenagens helicoidais, 13 tipos em 84 instâncias, `tests/conjunto_sintetico_b393.rs`) ainda levava 104 s até
a prévia. O tempo estava em três lugares que não dependem do número de peças distintas, e sim do de instâncias:

1. `posicionado` levava cada instância à sua posição com `BRepBuilderAPI_Transform(copy = true)`: uma cópia
   profunda da geometria por instância. Nada dividia malha, memória ou arquivo.
2. A malha de leitura (`malha_da_forma`) lê normal e curvatura de cada nó na superfície exata e calcula a
   assinatura de cada face (área por integração, projeção do centroide): 52 s das 54 s da malha do conjunto,
   feitos de novo para cada instância.
3. `mass_props` integrava o conjunto inteiro: 47 s para o volume, pedido pela legenda antes de a prévia sair.

## Decisão

1. `Shape::moved` (`move_shape` no kernel) leva a forma por translação ou rotação somando uma `TopLoc_Location`:
   a TShape é a mesma. A montagem usa isso em `posicionado`. Espelho e escala continuam copiando.
2. `malha_da_forma` guarda, por (TShape, orientação) da face, o cálculo dos nós e a assinatura feitos na origem,
   e as outras instâncias só levam o resultado pelo movimento rígido da face (posição, normal e ponto da
   assinatura; o eixo de cilindro, cone e toro refaz a regra do sinal). Movimento com espelho ou escala segue o
   caminho antigo.
3. `mass_props` integra cada sólido distinto uma vez e soma as instâncias (volume e área não mudam com o
   movimento; o centro é o ponderado pelo volume). Só quando todas as faces estão em sólidos e há mais de um.
4. A prévia da vista de montagem não espera pelo volume: ele vem na resposta da exata. A prévia sem volume não
   apaga o que já foi medido.
5. `hlr_rapida`: a silhueta de uma face analítica casava os vértices só pelo índice. A costura de um cilindro
   tem os nós duplicados, então a silhueta que cai sobre ela sumia (um contorno de cada eixo visto de frente, 28%
   das linhas visíveis do conjunto de eixos). Agora os vértices se casam pela posição dentro da própria face.

## Medição (conjunto sintético de 84 instâncias, release, máquina livre)

| Etapa | Antes | Depois |
|---|---|---|
| Malha do conjunto | 54 s | 9 s |
| Volume do conjunto | 47 s | 7,8 s (e fora do caminho da prévia) |
| Prévia das quatro vistas | 104 s | 14 s (carga 4 s, malha 9 s, projeção 0,2 s) |
| HLR exata, quatro vistas em sequência | 174 s | (processos em paralelo) 51 a 54 s |

A HLR exata do OpenCASCADE segue lenta com superfícies livres: o algoritmo trata cada face de cada instância e
não divide trabalho entre cópias. Ela continua em processos (ADR 0080), em segundo plano, e fica no disco
(ADR 0085) para as próximas aberturas.

## Conferência

- Volume, área e centro do conjunto batem com a soma das peças medidas uma a uma (erro relativo < 1e-6).
- Prévia contra a exata (conjunto de planos e cilindros, 84 instâncias): todo ponto das linhas visíveis da exata
  tem linha da prévia a até 0,5 mm (100%), e 97 a 100% da prévia tem linha da exata.

## Consequências

- Instâncias de uma peça repetida dividem a TShape: quem mudar uma forma no lugar (`BRepTools::Clean`, correção
  de tolerância) muda todas as cópias. As operações de modelagem produzem formas novas, então isso não ocorre
  hoje; a montagem só lê.
- A legenda mostra o volume da montagem depois da exata, não na prévia.
