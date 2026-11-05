# ADR 0043: conversão de subdivisão para B-rep por amostras e preenchimento (B179)

## Contexto

Longe dos nós irregulares a superfície de Catmull-Clark é bicúbica por quadrilátero. Em volta de um nó
de valência diferente de 4 ela não é polinomial, e remendos bicúbicos com aresta chegando no nó batem
num problema clássico: as condições de tangência das arestas em volta do nó formam um sistema cíclico
que, em valência par, só tem solução mexendo nos polos vizinhos.

## Decisão

1. **Remendo exato por ajuste, não por vizinhança.** A superfície é amostrada numa grade de 9 por 9 por
   quadrilátero, com as máscaras do limite, e os 16 polos saem por mínimos quadrados. Polinômio recuperado
   por ajuste volta exato, então não existe tabela de casos para borda, canto e vinco, e o resíduo prova
   o que é exato. Quem passa de 1e-9 do tamanho da forma é irregular.
2. **G1 e G2 tiram a aresta do nó.** Os quadriláteros não exatos, unidos pelas arestas lisas, formam uma
   região, e ela vira uma face só de preenchimento, com a condição nas faces vizinhas e passando por
   amostras do limite. O nó irregular fica no meio de uma face lisa e o problema cíclico não aparece.
3. **G0 usa remendos bicúbicos** com a curva de cada aresta compartilhada, então fecha sem folga.
4. **Região recusada cai para G0**, e o relatório diz qual e por quê. Falhar a conversão inteira por um nó
   tiraria da pessoa a chance de ver onde está o problema.
5. **O desvio é medido contra a malha fina da face**, com flecha de 3e-5 do tamanho. A projeção exata do
   kernel devolve a distância até a borda quando o ponto já está sobre a superfície. O número é um teto:
   inclui a flecha.
6. **Gaiola aberta pausa a conversão.** `ConversaoForma::pausada` faz o cálculo parar na forma, com os
   filhos em `desatualizados`; concluir refaz tudo. O campo vai para o arquivo porque o cache do cálculo
   usa o comando serializado.
7. **Nome de face = face da gaiola + pedaço**, ou o nó irregular numa região. Mover nó não troca nomes;
   inserir aresta troca, e a barra avisa quantos sumiram.

## Dois achados sobre o preenchimento do OpenCASCADE

- `GeomAbs_G2` é recusado: o enum vira número de derivadas e vale 3. Quem vale 2 é `GeomAbs_C1`, e a
  condição aplicada então é a de curvatura.
- Com arestas de apoio o fio pode sair ao contrário da superfície, porque o plano médio vem de autovetores
  e tem sinal arbitrário. A face fica sem interior e o malhador devolve nada. O teste é o ponto no infinito
  cair dentro da face; a correção inverte o fio. O conserto genérico de orientação errou nas duas direções.

## Consequências

- Uma caixa de 2 por 2 por 2 dá 368 faces. Regiões regulares vizinhas ainda não se fundem numa B-spline de
  vários vãos: seria menos faces, com o mesmo resultado geométrico.
- G2 custa perto de 1,2 s por nó irregular, contra 0,2 s em G1. A barra avisa antes quando passa de 10 s.
- Vinco que termina num nó deixa a região com fenda: ela não é disco e cai para G0.
- `subd_brep::decompor` só depende da gaiola, e o B158 pode reaproveitar remendos e regiões.
