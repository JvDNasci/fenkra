# ADR 0010: Referências v2 — dono + papel + assinatura, com ponto como reserva

- **Status:** aceita — fase 1 em 30/10/2026 (filete, chanfro, casca, draft, espessura); fase 2 em todas as bancadas em 18/11/2026 (B516)
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
- **Subelementos de superfície (B515, 18/11/2026):** a face, a aresta e o vértice de uma superfície de referência
  ganham chave como os de corpo. O dono é a própria superfície e o papel da face é o índice dela
  (`PAPEL_SUPERFICIE + i`); a aresta guarda as faces vizinhas e o próprio índice (`EdgeKey.edge`), e a borda (aresta
  livre, com uma face só) repete a face nos dois lados da chave. A resolução (`refs::find_surface_face`,
  `refs::find_surface_edge`) fica com o mesmo índice enquanto ele for do mesmo tipo (face) ou ficar entre as mesmas
  faces (aresta); se a superfície ganhou ou perdeu faces, desempatam a assinatura e o ponto, e o que não separa vira
  referência perdida. Com isso `SurfRef::Face`, `CurveRef::BodyEdge`, `AxisRef::Edge/Face`, `PointRef::Vertex` e o
  plano de face aceitam subelementos de superfície em qualquer comando. Limite conhecido: superfície que reordena faces
  do mesmo tipo sem mudar a contagem segue pelo índice.
- **Fase 2 em todas as bancadas (B516, 18/11/2026):** o painel mostrava "face em X Y Z"; agora toda face, aresta ou
  ponta escolhida tem identidade pelo modo como foi construída, e o painel mostra o nome dela
  (`refs::nome_face`, `refs::nome_aresta`, `refs::nome_ancora`; no app, `ref_nome.rs`, com tradução em
  `textos_referencias.rs`): "Extrusão 2 · tampa final", "Parede 1 · aresta 3 · tampa final", "Furo 1 · furo 2, boca",
  "Aba 1 · ponta", "Membro 1 · trecho 1 · início". A aresta entre a tampa e uma lateral do mesmo comando leva o número
  da aresta do perfil que gerou a lateral. As mensagens de referência perdida citam o nome e as candidatas pelo nome
  (`refs::sugerir_faces`, `refs::sugerir_arestas`).
  - **Âncoras de campo (`Feature::ancoras`):** os campos que só guardavam o ponto (linha da dobra, do joggle e da
    veneziana, com a âncora no ponto médio; face da aba por superfície e da conversão em chapa; centro do rasgo em
    arco; centros do furo flangeado, `points.i`; ponto da placa de extremidade e do reforço) ganham uma chave no próprio
    comando, `Ancora { campo, ponto, chave }`. A chave vale só enquanto o campo tiver o mesmo ponto: clicar outra face
    troca o ponto e a âncora velha deixa de valer sozinha, sem o painel saber dela (o mesmo cuidado de `carry_keys`).
    Foi escolhida a lista no comando, e não um campo `*_key` em cada struct, porque são dezenas de structs com uma
    centena de literais no código e nos testes; a regra "a chave só vale para o ponto em que nasceu" fica num lugar só
    (`Ancora::de`). Resolução (`ancoras.rs`): com âncora, a face sai de `find_face`; o ponto fica onde está se ainda
    estiver na face, e vai até ela pela normal se a face plana andou (a chapa engrossou); face sumida, ambígua ou que
    deixou o ponto fora dela é referência perdida com candidatas. Sem âncora, vale o ponto como antes e a face dele vira
    âncora (promoção, `Promoted::Ancoras`). Na chapa, a âncora só vale no estado (dobrado ou planificado) em que o ponto
    nasceu. A ponta de membro (`ChaveAncora::PontaMembro { membro, trecho, fim }`) identifica a ponta pelo comando e pelo
    trecho: a placa vai com a ponta mesmo quando outra fica mais perto do ponto antigo.
  - **Papéis da aba de chapa (`papeis::aba_topo`, `PAPEL_ABA = 900 000`):** a aba não tinha papel, e a aresta da ponta
    da aba (onde vai a bainha) e as da dobra ficavam só com o ponto (era a "exceção até as chaves de chapa saírem"). O
    papel sai da geometria da ferramenta, como nas varreduras: cilindros a `r` e `r + t` do eixo da dobra (dobra
    interna e externa), planos quase normais à aresta (lados 1 e 2, da dobra ou da parte reta), a raiz (a tira na
    parede, do lado da face de referência), a ponta (normal na direção em que a parte reta sai) e as faces interna e
    externa da parte reta (a `r` e `r + t` do eixo). Dobra, faces da parte reta e ponta não podem repetir; os lados
    podem, porque a meia-esquadria os corta. A aba por laço numera as abas do mesmo comando (`16 × aba + parte`). Não
    foi preciso levar o `BRepTools_History` à fachada C++: a booleana que soma a aba já devolve a origem de cada face
    (`boolean_tagged_src`), e o papel da ferramenta sai da geometria.
  - **Referências da pasta e términos:** plano de face, face extraída e eixo de face ou de aresta de arquivo antigo
    ganham a chave no fim da linha do tempo (`Promoted::Referencia`); o término "até a face" da extrusão e do furo
    ganha a chave (`Promoted::Ate`); a linha guia de membro que era o polígono de uma aresta (`CurveRef::Edge`) vira
    `CurveRef::BodyEdge` com chave (`Promoted::Caminhos`). O plano de face com chave passa a usar a consulta em camadas
    (`find_face` em todos os corpos); antes ficava com a primeira face do mesmo dono e papel que aparecesse.
  - **Montagem:** a aresta de arquivo antigo (`Viva::Aresta`, só o ponto) ganha o nome das duas faces na releitura,
    quando dá (`Viva::ArestaNomeada`).
  - **Sem escolha silenciosa:** a aba e a bainha sem chave entregavam o ponto ao kernel, que pega a aresta mais perto
    sem limite; agora só quando alguma aresta passa de fato pelo ponto (0,01 mm). Ponto no ar é referência perdida.
  - **Limites conhecidos:** o furo que promove centros (`Promoted::Furo`) e término no mesmo cálculo grava o término no
    cálculo seguinte (uma promoção por comando e por cálculo); a rosca não promove o término. Alívio e fechamento de
    canto (`SmCornerRelief.at`, `SmCornerClose.at`) apontam para um encontro de abas, e não uma face, e seguem pelo
    ponto. A tolerância antiga de aresta sem chave em documentos de chapa continua para os pontos ainda não
    promovidos. O assistente de reparo ainda não mostra no 3D as candidatas das âncoras (a mensagem já as cita).
