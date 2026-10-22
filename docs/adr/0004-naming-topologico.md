# ADR 0004: Referências topológicas persistentes por histórico de geração

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
Features e cotas referenciam faces, arestas e vértices de features anteriores. Quando uma feature
anterior muda, o kernel renumera a topologia, e referências por índice ("Face7") apontam para a
entidade errada ou deixam de existir. Esse é o **problema do naming topológico**, que afetou o
FreeCAD por mais de uma década.

## Decisão
1. Toda entidade topológica recebe um **nome estável derivado da sua origem**, por exemplo:
   `Extrusao3 / lateral / gerada-por(Esboco2 / Linha5)`.
2. Usar `BRepTools_History` / `BRepAlgoAPI` history do OCCT para propagar nomes pelas operações
   (gerada, modificada, apagada).
3. Referências são **consultas** resolvidas a cada recompute, não índices.
4. Quando a consulta resolve para 0 ou mais de 1 entidade, usar desempate geométrico
   (posição, normal, área) e, se ainda ambíguo, **marcar a referência como quebrada** e avisar o usuário.

## Alternativas consideradas
- **Índices simples:** triviais, mas quebram com qualquer mudança.
- **Apenas heurística geométrica:** frágil quando a geometria muda muito.
- **Abordagem do FreeCAD 1.0 (TNP mitigation):** estudar como referência; licença compatível.

## Consequências
- ✅ Base para timeline estável (RNF-008) e cotas associativas no desenho (RF-DES-007).
- ⚠️ Precisa existir desde a primeira feature; acrescentar depois é muito mais caro.
- ⚠️ Custo de memória e CPU para guardar o histórico; mitigado com cache por feature.
