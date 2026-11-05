# ADR 0035: forma livre por subdivisão, com gaiola de números persistentes (B153)

## Contexto

Editar polos NURBS não resolve a forma orgânica: a grade é retangular, então um ponto novo obriga a uma
fileira inteira. A bancada precisa de uma malha de controle de topologia livre, dentro da árvore
paramétrica, e que depois vire B-rep (B179).

## Decisão

1. **Catmull-Clark próprio em Rust**, em `fenkra-core/src/subd.rs`, com vincos semi-agudos. OpenSubdiv
   (Apache-2.0) fica como opção posterior: não tem ponte para Rust e a malha de uma forma cabe na CPU.
2. **A gaiola é o dado; a semiaresta é derivada.** O arquivo guarda nós e faces com número persistente
   que nunca se repete. A topologia (`Topo`) é refeita quando preciso. Assim uma operação não precisa
   manter ponteiros válidos, e quem aponta para o número continua apontando depois de inserir aresta.
3. **`FeatureKind::Forma` na árvore.** Sem conversão, a superfície aparece por `RecomputeResult::formas`
   e não é corpo. A subdivisão roda a cada cálculo, fora do cache: leva menos que a busca.
4. **Simetria guarda só a metade.** `Gaiola::inteira()` gera o reflexo, com os números marcados por
   `ESPELHO`. Não há como as metades divergirem, e nó da costura fica preso ao plano.
5. **Peso do vinco de 0 a 1** vira nitidez de até 3 passos; 1 é vinco cheio. Mais do que isso não muda
   a forma de modo visível e custaria níveis de refinamento na conversão.
6. **Regras sem egui.** `forma_edit.rs` decide o que está sob o mouse e o que cada gesto faz, e
   `forma_ui.rs` só desenha. É o que permite testar o fluxo sem mouse, inclusive o aceite de 10 ações.
7. **Gaiola desenhada pelo pintor da interface**, não pela cena: as cores vêm do tema a cada quadro e o
   realce do mouse não refaz a cena. A oclusão é por face de frente, o que basta para mirar; numa forma
   muito côncava uma aresta atrás pode aparecer por cima.

## Consequências

- Tangência no nó é uma operação sobre o anel, refeita depois de cada edição; não é uma restrição
  resolvida em conjunto, então dois nós vizinhos com tangência disputam os mesmos pontos.
- Suavizar borda muda a topologia na hora, sem parâmetro vivo: para refazer é preciso desfazer.
- Vínculos do nó a referências do modelo ficam para depois: o número persistente já dá a âncora.
