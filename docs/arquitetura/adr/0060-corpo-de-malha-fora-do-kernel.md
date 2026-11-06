# ADR 0060: corpo de malha fora do kernel, com crate próprio e resultado guardado por conteúdo (B170)

## Contexto

A malha importada virava sólido facetado: cada triângulo uma face costurada pelo OpenCASCADE, com teto
de 200 mil triângulos. Escaneamento, modelo de impressão e saída de escultura passam disso fácil, e um
B-rep de triângulos é o pior dos dois mundos: pesado como B-rep e sem geometria exata. Os itens B119,
B120, B158, B164 e B169 precisam de uma base de malha de verdade.

## Decisão

1. **Crate `fenkra-malha`, em Rust puro.** Leitores e gravadores de STL, OBJ, PLY e 3MF saíram do
   conversor para lá, com a `Cena`, porque o `fenkra-solidos` precisa ler malha e o conversor depende
   dele. O crate não conhece documento nem kernel. Solda, reparo, decimação, BVH, desvio e booleanas
   recebem uma malha e devolvem outra.
2. **Corpo de malha ao lado do corpo B-rep.** `Ctx::malhas` guarda o corpo de cada coletor, sem `Shape`.
   No resultado ele é um `BodyResult` com uma face só: viewport, seleção e medidas funcionam sem saber
   que não é B-rep, e os comandos de sólido não o enxergam. A passagem é explícita, pelos comandos Malha
   para sólido (com o teto de sempre) e Sólido para malha.
3. **A árvore guarda o pedido, o documento guarda a malha.** `FeatureKind::MeshOp` tem só parâmetros.
   A malha importada fica em `Document::malhas`, por hash, comprimida e em base64 num `Arc<str>`. O
   comando leva a chave, então assinatura e desfazer não carregam megabytes, e o documento abre sem o
   arquivo de origem, como o corpo importado de STEP.
4. **Resultado guardado por conteúdo.** A chave de cada passo é o hash da chave anterior com os
   parâmetros, num cache global com teto de memória, fora do `Cache` do recálculo. Os comandos de malha
   não usam o reaproveitamento por comando, que só guarda `Shape`: eles sempre rodam, e rodar é achar a
   chave. `ParaBrep` é a exceção, porque o resultado dele é `Shape` e costurar custa caro; quando é
   reaproveitado, tira o corpo de malha do mesmo jeito.
5. **Booleana pela Manifold em Rust.** O crate `manifold-rust` (Apache-2.0, sem C++ a compilar) dá saída
   sempre fechada, em troca de exigir entrada fechada. O `boolmesh` foi descartado pela licença MPL-2.0,
   fora do `deny.toml`, e o `manifold-csg` por ligar a biblioteca em C++.
6. **Importar entra como corpo por padrão.** `OpcoesImportar::malha_em_solido` volta ao caminho antigo.
   Montagem que cabe no teto continua pelo B-rep, porque peça de montagem ainda é documento com sólido;
   acima do teto entra achatada, com aviso, em vez de ser recusada. `MeshImport` sem `corpo` segue
   recalculando como B-rep: documentos antigos não mudam.

## Consequências

- Não há mais teto de triângulos na entrada. O limite passa a ser a memória.
- Comando de sólido em coletor só com malha acusa falta de corpo, sem converter nada escondido.
- Gravar STEP de documento com corpo de malha é erro com a orientação de converter, e não perda calada.
- Decimar e booleana rodam na linha do cálculo e não obedecem ao Cancelar. Em malha enorme, isso segura
  a tela até acabar; fica para quando houver cálculo de malha em segundo plano.
- Malha embutida fica no documento mesmo depois que o comando sai da árvore, até alguém limpar.
