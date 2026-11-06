# ADR 0072: referências da montagem guardam como se reencontrar (B99, B263, B55)

## Contexto

Restrição, junta e repetição guardam a geometria já lida no sistema da peça, para o solucionador não
depender do modelo. Só a face clicada se relia, pelo ponto do clique. Aresta, vértice, ponto de encaixe e
geometria da montagem ficavam congelados, e a junta nem tinha de onde reler.

Reler a aresta pelo mesmo caminho da face dava errado sem aviso: no ponto da aresta, `geom_at` pergunta
pela face vizinha e pode devolver o eixo do cilindro ao lado, que tem o mesmo tipo de `Geom` da aresta.

## Decisão

1. **`MateRef` ganha `viva`**, um enum pequeno e copiável que diz como se reencontrar: aresta perto de um
   ponto, vértice, ponto de encaixe de um tipo, ou identidade na geometria da montagem. Sem ele, vale
   o `pick` da face, como antes, e arquivos antigos abrem sem mudança.
2. **A leitura fica em `releitura.rs`**, e o solucionador continua vendo só `Geom`.
3. **A junta guarda as `fontes`** de cada lado. Ela não usa a geometria direto: o sistema dela leva giro e
   deslocamento. Por isso `transportar` move o sistema pelo que o elemento andou, em vez de refazê-lo.
4. **Sem achar, fica como estava.** A referência mantém a última geometria boa. Reencontrar por
   proximidade é uma aposta, e uma aposta errada não deve derrubar a montagem.
5. **Origem feita de dois elementos não se relê.** Entre duas faces e cruzamento de arestas não têm um
   elemento só para seguir, então `fontes` fica vazio nesse lado.

## Consequências

- Mudar a peça acompanha sem refazer restrição, inclusive pela geometria da montagem.
- Reencontrar é pela distância: uma mudança grande pode achar o vizinho. Nomear aresta e vértice pela
  topologia resolveria isso, e fica para quando a nomeação topológica chegar à montagem.
- A releitura roda a cada `AsmState::solve`. Os candidatos de encaixe são contados uma vez por peça em
  cada passada; se pesar em peça muito grande, o próximo passo é só reler quando o arquivo mudar.
