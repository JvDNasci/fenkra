# ADR 0010: Referências v2 — dono + papel + assinatura, com ponto como reserva

- **Status:** proposta
- **Data:** 2026-10-29
- **Revê:** [ADR 0004](0004-naming-topologico.md) (parcialmente implementada)

## Contexto
A ADR 0004 previa nomes estáveis pelo histórico de geração do OCCT. O que foi implementado é mais
simples:
- faces e arestas escolhidas pelo usuário são guardadas como **pontos clicados**
  (`EdgeSel::Edges(Vec<[f64;3]>)` e os campos `face: [f64;3]` de Shell, Draft, Thickness, Thread e Sm*);
- a cada recompute, o ponto é casado com a entidade **mais próxima**;
- cada face do resultado sabe qual comando a criou (`BodyResult.face_owner`, teste
  `etiquetas_de_face_sobrevivem_a_booleana`), e `EdgeSel::CreatedBy(Id)` usa isso.

O design review (slide DR-11) mostrou o limite do ponto: alterar o perfil que corta um cubo fez o
filete "se perder". O mesmo vale para planos de face que ficam vivos depois que a face some (DR-21).

## Decisão
Cada referência a face, aresta ou vértice passa a ser uma **consulta em camadas**, resolvida a cada
recompute nesta ordem:
1. **Dono:** o comando que gerou a entidade (`face_owner`, levado às arestas e vértices pelas faces
   vizinhas).
2. **Papel:** o que a entidade é para o dono. Exemplos: tampa inicial/final, lateral gerada pelo
   segmento N do sketch, face do furo, aresta entre as faces A e B.
3. **Assinatura geométrica:** tipo de superfície/curva, normal ou eixo, área/comprimento.
4. **Ponto clicado:** fica como **reserva** para desempate e para arquivos antigos.

Se a consulta não resolver para exatamente uma entidade, o comando fica **em erro com sugestão**
(nunca escolhe em silêncio), e o assistente de reparo mostra os candidatos no 3D.

Arquivos atuais continuam abrindo: referência sem dono/papel resolve como hoje (pelo ponto) e é
promovida para v2 no primeiro recompute bem-sucedido.

## Alternativas consideradas
- **Só o ponto (atual):** simples, mas quebra quando a geometria anterior muda de lugar.
- **Histórico completo do OCCT (ADR 0004 pura):** o mais robusto, mas exige propagar
  `BRepTools_History` por todas as operações da fachada C++; fica como evolução do passo 2 (papel).

## Consequências
- ✅ Filetes, chanfros, cascas e planos de face sobrevivem a mudanças de perfil e de cota.
- ✅ Erros viram "referência perdida" com reparo guiado, em vez de resultado errado.
- ⚠️ O kernel precisa devolver papéis por face (tampas e laterais) nas extrusões, revoluções e varreduras.
- ⚠️ Migração gradual: comando por comando, com teste de regressão "muda o perfil, o filete continua".
