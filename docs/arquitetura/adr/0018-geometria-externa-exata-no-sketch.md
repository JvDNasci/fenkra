# ADR 0018: geometria externa exata no sketch

Estado: aceita. Item B97.

## Contexto

Converter e Interseção trabalhavam com polilinhas: a aresta do corpo vinha da malha, em precisão
simples, e o sketch adivinhava círculos pela forma dos pontos. Arco virava spline, elipse virava
spline, e a interseção só achava onde a polilinha cruzava o plano. O vínculo não dizia se a origem
tinha sumido, e não havia como parar de acompanhar sem apagar a geometria.

`BodyResult` não guarda a forma do kernel, então a interface não pode perguntar nada a ele depois do
cálculo.

## Decisão

1. **A malha carrega a fórmula de cada aresta.** `Mesh::edge_geom` traz, em precisão dupla, reta ou
   cônica `P(t) = c + u cos t + v sin t`. Doze números por aresta custam pouco perto da polilinha e
   servem à interface e ao recálculo sem guardar a forma.
2. **Projeção e corte são contas puras em `fenkra-core::projecao`.** A projeção é afim e mantém a
   forma da cônica, então basta achar os eixos principais. O corte tem solução direta, com todos os
   pontos. Aresta de outro tipo no corte vai ao kernel (`Shape::edge_plane`, interseção da curva com
   o plano). Sem fórmula na projeção, fica a polilinha de antes.
3. **O sketch ganha `Curve::Ellipse`, só como geometria externa.** A forma vem da origem e não entra
   no solver: sem variáveis novas, com centro e pontas fixos. O perfil vai ao kernel como elipse de
   verdade (`Seg2::Ellipse`).
4. **Política por sketch e por vínculo:** associativa, isolada ou instantânea. Isolar guarda a origem
   e para de acompanhar. Instantânea nasce solta. O estado quebrado não é gravado: o cálculo diz a
   cada vez, em `link_quebrados`, e o vínculo volta sozinho quando a origem volta.
5. **Sketch com posição própria:** `SketchData::pos` guarda origem, troca e inversão de H e V sobre o
   plano-base. A normal é sempre H × V, para o plano seguir com orientação direta. Inverter um eixo
   só vira a normal, e a extrusão passa para o outro lado.

## Consequências

- Arquivos antigos abrem: os campos novos têm padrão, e vínculos antigos continuam polilinhas.
- Se a origem mudar de tipo (círculo que vira elipse ao inclinar o plano), o vínculo fica quebrado
  com o motivo, porque os pontos guardados não servem à forma nova. Recriar o vínculo resolve.
- A aresta ainda é reencontrada pelo ponto médio, com folga de 5 mm. Nomeação topológica no vínculo
  fica para depois.
- Quantidade de pontos de interseção é fixada ao criar: se o 3D passar a cortar em mais pontos, os
  novos não nascem sozinhos.
- Elipse não apara, não estende e não espelha.
