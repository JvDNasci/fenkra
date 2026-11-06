# ADR 0046: multi-seções com controle sai de uma grade calculada em Rust

Data: 2026-11-05. Item: B34. O cálculo já aceita conectores, sentido e tangência; a interface deles é do B290.

## Contexto

O multi-seções do kernel recebe só as seções e escolhe onde cada uma começa. Guias que a superfície toca
e conectores que dizem qual ponto vai com qual não têm entrada ali. O caminho com guia do kernel é uma
varredura: as seções percorrem a guia, mas nada garante várias guias ao mesmo tempo.

A interpolação de grade pronta do kernel também não serve: ela ondula a borda perto dos cantos em até
0,2% do tamanho, mesmo com todos os pontos da linha no mesmo plano, e refinar a grade não melhora.

## Decisão

- Sem guia, conector, inversão ou tangência, nada muda: o cálculo segue pelo kernel.
- Com qualquer um deles, `fenkra-solidos/src/loft.rs` calcula uma grade de pontos e a soma de Gordon cuida
  das guias. O kernel só recebe a grade, em `grid_surface`.
- `grid_surface` interpola linha a linha e depois coluna a coluna de polos, com os mesmos parâmetros, e
  monta a superfície por produto tensorial.
- A grade é cortada nos cantos vivos das seções, e cada trecho vira uma face.
- Análises são a referência `SurfCheck`, de classe ponto: o resultado é o ponto do pior caso, e os números
  vão na nota. Isso dá recálculo, árvore e painel sem um tipo novo de geometria.

## Consequências

Com controle, a superfície é uma aproximação por B-spline da grade, com 97 pontos por trecho inteiro de `u`
e 32 ou mais em `v`. Guias não valem em superfície que volta à primeira seção, e com guias a tangência das
pontas é ignorada: a soma de Gordon pediria o mesmo operador nas duas parcelas.
