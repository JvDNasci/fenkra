# ADR 0010: Referências v2 — dono + papel + assinatura, com ponto como reserva

- **Status:** aceita — fase 1 implementada em 30/10/2026 (filete, chanfro, casca, draft, espessura)
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

## Estado (30/10/2026)
- **Fase 1 (feita):** papel de face na extrusão e revolução, propagado pelas booleanas; `FaceKey`/`EdgeKey`
  com o ponto como reserva em filete, chanfro, casca, draft e espessura; "referência perdida" em vez de
  escolher em silêncio (faces a mais de 1 mm e arestas não achadas viram erro); promoção de arquivos antigos.
- **Fase 2:** chaves para rosca, filete face-face, arredondamento completo e chapa; papéis para extrusão até
  plano, varredura, loft, furos e padrões; candidatos no 3D no assistente de reparo; `EdgeSel::CreatedBy`.
- **Correção (B15, 05/11/2026):** a referência de aresta sem chave (arquivo antigo) casava o ponto com a
  aresta mais próxima dentro de 5% da diagonal, não de 1 mm, e trocava de aresta em silêncio (1,07% das
  amostras do B306). Agora o ponto só vale a até 0,1 mm (o ponto guardado é o médio exato da aresta;
  geometria que não mudou casa a ~1e-9), o ponto exato da aresta achada é o que vai ao kernel, e fora
  disso o comando fica em "referência perdida" com sugestão. O mesmo vale para a face sem chave (1 mm).
  Exceção até as chaves de chapa saírem: documentos com parede de chapa mantêm a tolerância antiga.
- **Superfícies (B76, 05/11/2026):** `SurfRef::Face`, `SurfExtract`, `SurfMulti`, `FacePlane`, `AxisRef::Face` e `AxisRef::Edge`
  guardam a chave ao lado do ponto; `CurveRef::BodyEdge` e `PointRef::Vertex` são a aresta e o vértice por dono e papel. A
  polilinha gravada em `BodyEdge` é só a última forma: a curva vem do corpo a cada recompute. Arquivos antigos seguem pelo ponto,
  sem promoção. `SurfEdgeFillet` fica fora da chave porque superfície de referência não tem dono por face; ali vale o ponto a até
  0,1 mm e fora disso é referência perdida. `which` de Interseção e Contorno vira âncora espacial no primeiro cálculo, e número
  fora da lista é erro, nunca "a última".
- **Fase 2, rosca e filetes por face (B15, 13/11/2026):** `Thread.face_key` e `EdgeExtra.face_keys` (face-face: a, b;
  completo: meio, a, b) guardam a chave ao lado do ponto. O recompute resolve pela chave com `resolve_faces` (o ponto sozinho vale
  a até 1 mm) e promove o ponto antigo no primeiro cálculo; o app grava a promoção e, ao trocar a face clicada, a chave anda junto
  com o ponto (a face nova fica sem chave até o próximo cálculo). Falta da fase 2: papéis no kernel para varredura, loft, extrusão
  até plano e furos; `EdgeSel::CreatedBy` por papéis; candidatos no 3D no reparo; troca de `PointRef/AxisRef::Fixed` por
  geometria de origem.
- **Fase 2, papéis que faltavam (B15, 13/11/2026):** a varredura (tubo por trecho com junta de esquadria), o loft, a extrusão
  até plano, até superfície, até o próximo e com saída, e os furos passam a dar papel às faces (`papeis.rs`). O papel sai da
  geometria do resultado, com a convenção da extrusão: tampa inicial é a face no plano do perfil que encosta nas arestas dele,
  lateral `2 + i` é a outra face da aresta `i`, tampa final é a que encosta na última seção (loft), a plana restante com a área
  da tampa inicial (varredura) ou o que sobra sem encostar na tampa inicial (extrusões); o trecho seguinte de uma lateral numa
  guia com cantos herda o papel pela junta. Na dúvida (aresta do perfil que some, face com dois papéis) a ferramenta fica sem
  papéis e a referência segue pelo ponto. Furos têm papéis próprios a partir de 1000 (`1000 + 16 × furo + parte`: parede,
  parede da cabeça, escareado, ponta, boca, fundo, assento). O lado 2 da extrusão soma os papéis dos dois lados.
  `EdgeSel::CreatedBy` vale pelas arestas entre faces do comando e, quando o comando tem papéis, só entre faces de papel
  conhecido (`refs::arestas_criadas`). O assistente de reparo mostra as candidatas da face ou aresta perdida numeradas no 3D
  (mesmo dono e papel quando é ambígua; as mais perto do último ponto quando sumiu); o clique troca o ponto e a chave é refeita
  no cálculo seguinte (`reparo.rs`, `reparo_ui.rs`). O centro fixo do furo sobre uma face com papel vira `PointRef::OnFace`, e a
  direção fixa que coincide com uma aresta reta (mesmo sentido) ou com a normal de uma face plana vira `AxisRef::Edge` ou
  `AxisRef::Face` (`Promoted::Furo`, `fixos.rs`); coordenada fora da peça continua fixa, porque pode ter sido digitada de
  propósito. Com isso a fase 2 fecha; o histórico completo do OCCT (alternativa 2) segue como evolução.
