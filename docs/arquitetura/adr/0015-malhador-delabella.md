# ADR 0015: Malhador Delabella no OpenCASCADE

- **Status:** aceita (rejeitar a adoção total; reavaliar só para faces planas com muitos contornos)
- **Data:** 2026-11-05
- **Item:** B235

## Contexto
O malhador padrão do OpenCASCADE é lento em face plana com muitos contornos, e uma face só não se divide
entre núcleos. O algoritmo Delabella (`IMeshTools_MeshAlgoType_Delabella`) tinha sido medido a 237 ms
contra 2.032 ms na placa 20x20, mas gerou 1.916 contra 4.224 triângulos na engrenagem, diferença sem
explicação. Faltava medir em todo o corpus antes de adotar.

## Medição
Teste `malhador_padrao_contra_delabella_no_corpus` (em `crates/fenkra-solidos/tests/corpus_pecas.rs`,
marcado `ignore` porque o interruptor do malhador é global; rode com `-- --ignored --nocapture`). Para
cada peça das dezesseis do corpus e para os ajustes Rascunho, Normal e Alta, mede tempo, triângulos, o
erro do volume que a malha fecha contra o volume exato e as faces que ficaram sem triângulo.

Resultado, em perfil de depuração com otimização, nesta máquina:

- **Quinze das dezesseis peças:** mesma contagem de triângulos (ou diferença de até 1% na esfera) e
  mesmo erro de volume. O Delabella foi de 1,1 a 3,3 vezes mais rápido nas peças curvas (toro Alta
  57 contra 19 ms, esfera Alta 30 contra 9 ms) e empatou nas planas pequenas.
- **Engrenagem reta m2 z40:** o Delabella deixou **80 faces sem nenhum triângulo** e o volume fechado
  pela malha ficou 32% errado (0,318 contra 0,001 do padrão). Os 1.856 triângulos contra 4.010 vêm daí:
  não é uma malha mais econômica, é uma malha com buracos. Nenhum erro foi sinalizado pelo malhador.

## Decisão
1. **Não adotar o Delabella como padrão.** Ele falha em silêncio numa peça do corpus, e uma peça com
   faces sumidas é pior do que uma peça lenta.
2. **Não adotar só para faces planas por enquanto.** O ganho grande (8 vezes) foi medido na placa 20x20,
   com muitos furos, e esse caso não está no corpus de dezesseis peças na escala em que aparece. Adotar
   exige antes (a) incluir a placa 20x20 no corpus, (b) detectar a face sem triângulo depois da malha e
   refazer essa face com o padrão, e (c) repetir a medição no perfil de lançamento.
3. **Manter o interruptor de medição.** `definir_malhador_delabella` existe só para comparar; o programa
   nunca o liga. O teste de aceite "nenhuma peça com desvio acima do pedido" fica como critério da
   futura adoção parcial.

## Consequências
- Nada muda para quem usa o programa.
- A conferência "face sem triângulo" é útil por si só: vale como diagnóstico para qualquer malhador.
- Medido só em depuração; o perfil de lançamento pode mudar as razões de tempo, não a falha da engrenagem.
