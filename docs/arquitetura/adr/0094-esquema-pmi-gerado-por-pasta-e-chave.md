# ADR 0094: esquema de cotagem automático marcado por pasta e chave de nome

Data: 2026-11-14. Estado: aceita. Itens: B192 (com B117 e B144).

## Contexto

O assistente de esquema automático (B192) gera referências, cotas e tolerâncias no PMI 3D e precisa ser regenerável: quando a peça muda, o que ele criou se refaz, sem apagar o que a pessoa editou ou criou à mão. A anotação (`Pmi`) não tem campo para "gerada" nem para a origem, e acrescentar um campo ao `Pmi` mexe em dezenas de construções literais espalhadas pelo código e pelos testes.

## Decisão

1. **"Gerada" é estar na pasta `esquema_pmi::PASTA`** ("Esquema automático"). Tirar da pasta é o gesto de congelar: a anotação vira manual e o assistente nunca mais a toca.
2. **A chave da regra é o nome**: cada anotação proposta tem um nome estável ("Referência A", "Furo Ø6,6 (4×)", "Local … até B", "Básica … até B"). Medidas de natureza diferente em cada modo (localização com ± ou básica) têm nomes diferentes, então trocar o modo troca a anotação em vez de herdar a tolerância errada.
3. **Regenerar (`aplicar`)**: a gerada com a mesma chave fica, com texto, tolerância e estilo como a pessoa deixou, e só a geometria (âncoras e alvos) é refeita; a gerada sem chave na proposta sai; com `refazer` as geradas voltam ao que a regra manda. As manuais nunca são tocadas.
4. **As regras são dados** (`RegrasEsquema`, JSON), para reaproveitar entre peças.
5. **O desenho coordena pelo vínculo de importação** (B371): o desenho automático (B144) traz primeiro as anotações da pasta para a primeira vista que as mostra e depois só completa o que falta; o plano de inspeção (B117) dá à característica importada o número que ela tem no 3D.

## Consequências

- Renomear uma gerada dentro da pasta faz o assistente tratá-la como sumida (sai na próxima regeneração); a interface orienta tirar da pasta para guardar uma versão própria.
- O nome é dado do documento, gravado em português; a tradução fica para quando o `Pmi` ganhar campo de origem.
- Se um dia o `Pmi` ganhar um campo de origem, a pasta e o nome viram só apresentação, sem mudar a regra de regeneração.
