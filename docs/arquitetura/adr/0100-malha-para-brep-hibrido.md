# ADR 0100: malha para B-rep híbrido com primitivas reconhecidas

Data: 2026-11-14. Estado: aceita. Itens: B169.

## Contexto

Malha (STL, OBJ, 3MF) virava sólido facetado: uma face plana por triângulo. Funciona, mas o furo de um
STL vira centenas de faces, o volume perde a corda e nenhum comando de sólido reconhece o cilindro. O
item pede um segundo modo, com planos, cilindros, cones, esferas e toros reconhecidos, arestas pela
interseção das superfícies vizinhas e um sólido sempre válido.

## Decisão

1. **O reconhecimento fica no Rust** (`fenkra-solidos/src/malha_primitivas.rs`): componentes lisas
   separadas por cantos vivos (35°) tentam caber inteiras numa primitiva; depois, planos por
   crescimento de regiões (só vale a região de borda viva ou a compacta que pesa na componente: uma
   faixa de cilindro também é plana); por fim, sementes de dois anéis crescem com a primitiva curva que
   melhor se ajusta. O ajuste começa por conta direta (eixo pelas normais, círculo e esfera algébricos,
   raio menor do toro por busca) e termina em Levenberg-Marquardt na distância exata dos vértices.
2. **O kernel só monta** (`mesh_hybrid`, `hybrid.inc`). Recebe vértices já encaixados nas superfícies,
   os triângulos sem ajuste e, por região, a superfície e os laços de borda com a região vizinha de cada
   trecho. Trecho entre duas regiões analíticas vira a curva de `GeomAPI_IntSS` que passa por todos os
   vértices dele; perto de tangência (filete no plano) a interseção é instável e o trecho fica pelos
   vértices da malha. `ShapeFix_Face` cria as curvas paramétricas, escolhe o laço externo e põe a
   costura; tudo é costurado, consertado (`ShapeFix_Shape`), fechado em sólido e conferido
   (`BRepCheck_Analyzer`); as faces coplanares do mesmo domínio se unem.
3. **Sempre sai um B-rep válido**: se a montagem com as curvas falha, ou o volume foge mais de 2% do da
   malha, tenta só com os planos; se ainda falhar, facetado. O motivo vai para a nota do comando.
4. O modo é um campo do comando (`MeshOpKind::ParaBrep { modo, tol }`), com padrão facetado para os
   arquivos antigos; o comando novo nasce no modo de primitivas.

## Consequências

- O facetado também funde as coplanares: a caixa de 12 triângulos vira 6 faces.
- A nota do comando traz o que foi reconhecido e o desvio máximo malha × sólido.
- O ajuste NURBS livre por regiões fica para pesquisa separada, como o item manda.
