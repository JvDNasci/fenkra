# ADR 0024 — Escolha de região por âncora com o lado de cada corte

## Contexto

Aparar e dividir por região (B277) deixa a pessoa clicar nos pedaços. O resultado precisa continuar o mesmo
depois de mudar uma cota. O kernel devolve as faces em ordem própria, e essa ordem muda quando o modelo muda, então
o número do pedaço não serve. Só o ponto clicado também não: se a ferramenta anda por cima dele, o ponto passa
a cair no pedaço vizinho e a escolha troca de lado sem aviso.

## Decisão

- **Comando novo no documento:** `RefKind::SurfRegions`, com superfície, ferramentas, curva, modo, ação e marcas.
  `SurfSplit` e `SurfTrim` continuam calculando os arquivos antigos e saem da barra; o lado deles vira seta no 3D.
- **Marca:** `RegionMark` guarda de qual superfície o pedaço veio, um ponto dentro dele e o sinal do ponto em
  relação à normal de cada corte.
- **Resolver:** entre os pedaços da mesma origem com os mesmos lados, fica o mais perto do ponto. O lado decide
  primeiro; a distância só desempata, por exemplo numa folha em U cortada nos dois braços.
- **Pedaço que sumiu:** a marca não é aplicada em outro. Ela fica guardada e a referência sai com aviso. Trocar sozinho
  seria decidir pela pessoa, e o erro só apareceria depois.
- **Lado do pedaço, não só do ponto:** uma folha que cresce além da ferramenta fica com um rasgo e deixa de ter duas
  metades. O ponto dela ainda cairia de um lado, e a marca da metade apagaria a folha inteira. Os vértices do pedaço
  votam: com vértices dos dois lados, ele recebe um valor próprio, que não casa com a marca.
- **O que é um pedaço:** faces ligadas por aresta fora do corte. Uma folha dobrada tem duas faces antes do corte, e
  contar face em vez de pedaço daria quatro regiões para dois lados. A separação fica no kernel (`split_regions`).
- **Ferramentas uma a uma:** vão ao corte como lista. Num composto só, duas ferramentas que se cruzam contam como
  uma forma que corta a si mesma, e a operação exige o contrário.

## Consequências

- A inversão da normal de uma ferramenta muda todos os sinais: as marcas ficam sem pedaço e a tela avisa, em vez de
  inverter a escolha sem dizer nada. A pessoa clica de novo.
- Acrescentar uma ferramenta depois não invalida as marcas: o corte que a marca não conhece não entra na comparação.
- Tirar uma ferramenta pelo painel renumera as origens das marcas e descarta as que vinham dela.
