# ADR 0102: compressão meshopt, passos e balões no pacote de visualização

- Estado: aceita
- Data: 2026-11-14
- Itens: B181 (pacote versionado), B146 (visualizador HTML), base de B124, B156 e B162

## Contexto

O pacote `.fkv` (ADR 0062) já levava malha glTF, árvore com IDs estáveis, PMI, capturas, vistas e
estados. Faltavam três coisas para alimentar as instruções de montagem, o visualizador e a VR: malha
compacta, os passos da explosão e os balões ligados à lista de peças. Também faltava um teste que
abrisse pacotes de referência em mais de um leitor.

## Decisão

1. **Compressão pela extensão aberta `EXT_meshopt_compression`**, com codificador e decodificador
   próprios (Rust em `fenkra-pacote/src/meshopt.rs`, JavaScript na página), escritos pela
   especificação pública. Só os modos `ATTRIBUTES` (versão 0) e `INDICES` (versão 1), sem filtros. O
   modo `TRIANGLES` dá arquivo um pouco menor, mas é bem mais difícil de conferir; fica para depois.
   Sem dependência nova: o decodificador da página não pode baixar nada.
2. **A compressão é opção (`Opcoes::compactar`), não padrão.** Com ela, a extensão entra em
   `extensionsRequired` e leitores glTF genéricos sem suporte recusam a cena. O padrão continua sendo
   o glTF que qualquer leitor abre; o visualizador do Fenkra lê os dois. O orçamento de tamanho
   (B146) liga a compressão sozinho quando o pacote passa do limite.
3. **Passos calculados no gerador.** O navegador não tem kernel nem o modelo de explosão; cada passo
   vai com quem se moveu, o deslocamento acumulado de cada componente, a seta, a lista de peças do
   passo e os balões. A lista do passo sai das peças que saíram do lugar, nunca digitada.
4. **Balão aponta para o número do item da lista** (`lista[].item`) e para o ID da árvore. Cada peça
   da árvore leva `arquivo`, a mesma chave da lista, para casar sem ambiguidade.
5. **Campos aditivos: o formato continua na versão 1.** Leitor antigo ignora `passos`, `baloes` e
   `arquivo`; só a cena compactada exige leitor novo, e ela é escolha de quem gera.
6. **Conformidade em dois leitores.** `tests/conformidade.rs` abre cinco pacotes de referência
   gravados em `tests/referencia/` no leitor Rust e no leitor da página (o mesmo trecho de
   `visualizador.html`, rodado pelo Node) e exige o mesmo resumo, inclusive a soma de cada acessor
   depois de decodificar.

## Consequências

- Pacote compactado da montagem de teste: 34 kB contra 91 kB sem compressão, sem perda (os bytes
  decodificados são iguais aos originais).
- Mudou a regra da cena: todo nó da página guarda a matriz base, para que peça na origem também
  se mova na explosão.
- Os pacotes de referência são regravados com `FENKRA_FKV_REFERENCIA=1` só quando o formato muda de
  propósito.
