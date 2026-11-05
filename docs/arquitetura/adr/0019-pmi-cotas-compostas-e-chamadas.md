# ADR 0019 — Cadeia e base como cotas comuns; caminho da chamada no estilo

## Contexto

B185 pede cota em cadeia e a partir de uma base; B23 pede nota com várias pontas e vértices intermediários;
B186 pede chamada gerada da feature. Cada uma poderia virar um tipo novo com dados próprios (lista de
valores, lista de caminhos), o que obrigaria o diagnóstico, a tolerância e a exportação a tratar casos novos.

## Decisão

- Cadeia e base geram várias `Pmi` comuns (linear ou ordenada), numa pasta da árvore. Cada trecho tem alvo,
  estado e tolerância próprios, que é o que o AP242 espera de uma cota simples.
- Os vértices intermediários vão em `PmiStyle::leader_via` (um caminho por ponta, em coordenadas do modelo).
  A ponta continua sendo a âncora, então reancorar, diagnosticar e exportar não mudam. Arquivo antigo sem o
  campo abre com a chamada reta.
- A chamada de feature (`PmiKind::FeatureNote`) guarda só o id da feature; o texto é regerado dos parâmetros a
  cada quadro e o último texto fica em `Pmi::text` para quem lê o documento sem a interface.
- A variante de medida escolhida com Tab é estado da sessão (como o plano da anotação), não do documento: o
  documento guarda só o tipo final.

## Consequências

Nenhum tipo novo exige solver, exportador ou migração. O custo é que a cadeia e a base não se movem como um
objeto só: a pasta da árvore é o agrupamento. A contagem `n×` depende de `Curve::Point` no sketch do furo.
