# Part Design / Features de sólido

> **Nota de método e confiabilidade.**
> - **[V]** marca o que li em fonte pública. **[C]** marca conhecimento geral que não confirmei nesta sessão.
> - **CATIA V5.** Li o guia oficial de Part Design copiado no site da City University (Londres). O CSS é da era V5R5, então os rótulos podem diferir do V5-6R2024. A estrutura funcional é estável.
> - **Inventor.** Li as páginas da ajuda Autodesk 2024–2026 diretamente.
> - **SolidWorks.** O WebFetch só devolveu a navegação de `help.solidworks.com`. Os trechos de SW vêm de resumos de busca dessas páginas, de Hawk Ridge, Javelin, Engineers Rule e CATI. Onde a fonte não cobria um detalhe, marquei [C].
> - **Outros CADs (Creo, NX, Solid Edge, Fusion, Onshape).** Cobertura rasa: Onshape e Fusion têm snippets [V], Creo e NX são quase só [C].
> - **Código do Fenkra.** Todas as referências `arquivo:linha` foram lidas nesta sessão.

---

## 1. Resumo e principais lacunas

O Fenkra já tem um **núcleo de Part Design amplo**. `FeatureKind` (doc.rs:1323) cobre extrusão/recorte, revolução/ranhura, furo (simples, rebaixado, escareado, contrafuro, rosca cosmética ou modelada), varredura, loft, nervura, filete e chanfro (constante, variável, face-face, completo), casca, saída, espessura, padrões, espelho, escala, mover corpo, combinar e dividir. Há também 12 tipos de plano de referência, pontos, linhas, curvas e superfícies.

A **filosofia robusta** está em construção:
- O ADR 0010 (referências por dono + papel + assinatura) já cobre filete, chanfro, casca, draft e espessura.
- `recompute_cached` (lib.rs:4198) reaproveita o que não mudou.
- Existem marcador de rollback, marcador por coletor e `suppressed`.

O que falta para chegar à filosofia do CATIA com a inteligência de SW e Inventor:

1. **Limites de término incompletos.**
   - Não há deslocamento (offset) nos limites "até plano" e "até superfície".
   - Não há "até vértice" nem "até corpo".
   - Não há limite diferente por lado (LIM1 até plano e LIM2 em cota).
   - Não há ponto de partida deslocado ("From").
   - O "até o último" usa a caixa envolvente, não a última parede.
   - A saída (taper) é **ignorada em silêncio** quando o término é "até…".
2. **Seleção de contornos e multi-pad.** `profile_face` usa todos os contornos do sketch. Não há comprimento por contorno nem contornos selecionados.
3. **Furo sem "Hole Wizard".**
   - `Hole.standard` é só informativo, e o diâmetro não deriva do tamanho.
   - Não há furo oblongo, escareado dos dois lados, nem tipo de rosca (NPT/BSP).
   - Não há profundidade até a ponta ou o ombro.
   - A posição por vértice (`PointRef::Fixed`) não é associativa.
4. **Filetes sem a camada "inteligente".** Faltam:
   - setback e raios múltiplos num comando;
   - pontos de controle no filete variável;
   - perfil cônico ou G2;
   - limites e "arestas a manter";
   - seleção por face, laço ou feature.

   O `EdgeSel::CreatedBy` **aplica a todas as arestas** com só um aviso (lib.rs:4184), e o atalho `ExtrudeDraftFillet` (app.rs:1720) depende dele.
5. **Draft, casca e nervura rasos.**
   - Draft sem linha de partição, reflect line, ângulo variável, degrau ou análise.
   - Casca com uma espessura só, sem "ignorar faces com problema" e sem aproximação.
   - Nervura sem o modo "From Side" (a nervura clássica).
6. **Padrões sem re-solução.** `replicate` (lib.rs:2644) copia a *ferramenta* já calculada. Não há "manter especificações" (CATIA), "Adjust" (Inventor) nem geometria vs. re-solução (SW). Também faltam:
   - pular instâncias;
   - espaçamento desigual;
   - padrão por curva e user pattern completo;
   - padrão de filetes e outras features de acabamento.
7. **Multicorpo.** Cada coletor tem um corpo, e `Combine` só une, subtrai ou intersecta. Faltam:
   - Union Trim e Remove Lump;
   - "dividir mantendo os dois lados" gerando coletores;
   - escopo de features (participantes);
   - "mesclar resultado" por comando.
8. **Contexto e diagnóstico.** Não existe Define In Work Object por feature (só `active_collector` + inserção no rollback). Não há modo de atualização manual nem estado "desatualizado". Também faltam Isolar/Extrair em falha, grafo pai/filho navegável e o diálogo de diagnóstico (B59).
9. **Conhecimento paramétrico.** Há parâmetros com expressões, ciclos detectados e funções básicas (expr.rs:58-101). Faltam:
   - tabelas de projeto e configurações;
   - parâmetros de texto e booleanos, lista de valores;
   - regras, verificações e reações;
   - supressão dirigida por expressão.
10. **Materiais e massa.** `Properties.material` é só texto. `MassProps` (kernel/lib.rs:203) tem apenas volume, área e centro: não há densidade por material, massa, matriz de inércia nem eixos principais. A densidade vive só no carimbo do desenho (default 7,85 g/cm³, desenho/lib.rs:506).

---

## 2. Fichas por comando/função

### Extrusão / Pad — adicionar material (CATIA: Pad / SW: Extruded Boss/Base / Inventor: Extrude (Join/New Solid) / outros: Fusion Extrude, Onshape Extrude / Fenkra: `Extrude` com `BoolOp::Add`)

**O que faz.** Dá volume a um perfil ao longo de uma direção.

**CATIA V5 — no detalhe**
- **Entradas.** Um perfil de sketch ou geometria plana do Generative Shape Design (nunca uma linha). Pode ser sub-elemento do sketch, e superfícies ou faces não planas também valem. [V]
- **Perfis.** Vários perfis não intersectantes no mesmo sketch são aceitos (um quadrado e um círculo geram cavidade). O perfil não pode ser tangente a si mesmo. [V]
- **Perfil aberto.** É aceito se material existente puder aparar a extrusão. Um limite pode ser "Up to Next" e o outro não. [V]
- **Direção.** O padrão é normal ao plano do sketch. Desmarcar "Normal to profile" permite escolher uma linha. Elementos do GSD não têm direção padrão e exigem uma. [V]
- **Comprimento.** Máximo 1.000.000 mm. LIM1 e LIM2 são arrastáveis. [V]
- **Reverse side** só vale para perfil aberto. [V]
- **Mirrored extent** estende no sentido oposto com o mesmo comprimento. Para comprimento diferente usa-se "More" e define-se o segundo limite. [V]
- **Thick** abre o "Thin Pad" (ver ficha própria). [V]
- **Saída na árvore.** Um Pad, com o sketch como filho. Há também Multi-Pad, e o "Drafted Filleted Pad" gera um Pad + um Draft + três Fillets. [V]
- **Update.** Editar o sketch recalcula. No Multi-Pad, um domínio novo entra com 0 mm e um removido exige atualização manual. [V]
- **Referência perdida.** Aparece erro de definição e o update oferece Editar, Desativar ou Excluir (ver ficha de Update). [V]

**SolidWorks — no detalhe**
- **"From".** Plano do sketch, superfície/face/plano, vértice ou deslocamento. [V]
- **Término da direção 1.** Blind, Through All, Up to Next, Up to Vertex, Up to Surface, Offset from Surface, Up to Body, Mid Plane. [V]
- **Direção 2.** Aceita as mesmas condições, exceto Mid Plane. [V]
- **Opções.**
  - Draft e "draft outward".
  - Contornos selecionados (um sketch alimenta vários cortes).
  - Direção por aresta/referência, mantida associada.
  - "Merge result" desmarcado gera outro corpo.
  - Feature scope limita os corpos afetados. [V]
- **Inteligente.** Condições dinâmicas ("Up to Surface") re-resolvem quando a geometria muda. [V]

**Inventor — no detalhe** [V]
- **Perfil.** Fechado, regiões ou laços. Perfil aberto só em superfície, ou sólido com "Match Shape".
- **Saída.** Sólido ou superfície. Booleana: Join, Cut, Intersect ou New Solid. Em multicorpo há um seletor de corpos participantes e "Predict Boolean".
- **Início ("From").** Plano de trabalho, plano de sketch ou face. "Extend face to end feature" liga sozinho se o From não intersecta o perfil.
- **Comportamento.** Direction 1, Flip, Symmetric (metade em cada lado), Asymmetric (A e B).
- **Extents.** Distance A/B, Through All, To (ponto, vértice, face ou plano), To Next com seletor de terminador, e Between (From + To).
- **Alternate Solution** escolhe a face mais próxima quando o término é ambíguo.
- **Taper A.** De 0 a 180°. Positivo aumenta a seção.

**Outros**
- **Onshape.** Tipos de término Blind, Up to next/face/part/vertex e Through all, com Symmetric só em Blind e Through all. O resultado é New/Add/Remove/Intersect, com "merge scope". [V]
- **Fusion.** Operações New Body/Join/Cut/Intersect/New Component. Ao editar um corte, escolhe-se entre "Auto-select" dos corpos e "# Bodies" fixos. [V]

**Fenkra hoje**
- **Estrutura.** `Extrude {sketch, op, extent, reverse, taper, thin, thin_side, dir}` (doc.rs:2503). O cálculo está em lib.rs:2720-2777. `Extent` (doc.rs:2486) tem Blind, Symmetric, TwoSides, ThroughAll, UpTo(plano), UpToNext, UpToLast e UpToSurface.
- **O que existe.**
  - Direção por `AxisRef`.
  - Papéis de face (`extrude_roles`) só nos casos Blind, Symmetric, TwoSides, ThroughAll e UpToLast. Nos demais os papéis ficam `None`, então as faces perdem a referência v2 (ADR 0010, Fase 2).
- **O que falta.**
  - **Bug provável 1.** Com `taper != 0` e término "até…", o ângulo é ignorado em silêncio. O ramo `UpTo`/`UpToLast`/`end_cutter` vem antes do teste de `taper` (lib.rs:2738-2768).
  - **Bug provável 2.** `UpToLast` usa a caixa envolvente (`last_dist`, lib.rs:2537), não a última parede. Num Pad isso estende material até o canto da caixa. O comentário em lib.rs:2548 admite a aproximação.
  - O sentido padrão de Add (+normal) e de Cut (−normal) difere. Com `dir` explícito, ambos vão pelo vetor. Isso é intencional (lib.rs:2724), mas é uma regra a documentar.
  - Não há "From" deslocado, Up to Vertex, Up to Body, offset em "até plano/superfície", término diferente por lado, contornos selecionados nem Multi-Pad.
  - "Passante" sem corpo no coletor dá o erro "precisa de um corpo existente" (lib.rs:1691).
  - Não há operação "novo corpo" na extrusão. O destino é sempre o coletor alvo.

**Melhor prática para o Fenkra**
- **Fluxo do painel.**
  1. Suporte: perfil/contornos, com seletor de contornos.
  2. Modo: Cego / Simétrico / Assimétrico (A e B, cada um com seu término).
  3. Parâmetros: A, B, offset do término e saída.
  4. Avançado: início deslocado, direção, parede fina e participantes.
- **Regras.**
  - Um término por lado: `Extent` por lado (A e B), não um enum único.
  - Saída com término "até…" deve virar erro explícito ou aviso, nunca ser ignorada.
  - Não existindo a face-alvo, o estado é "falhou", nunca "escolher outra".
- **Casos de teste.**
  - Cego, simétrico e assimétrico: volume = área × soma.
  - Perfil com furo.
  - Até próximo em degrau: termina na face certa. Depois muda a cota e continua certo.
  - Até último com corpo em L.
  - Saída + até plano: erro explícito.
  - Corte "até superfície" curva.
  - Mudar o sketch: papéis de face mantidos e filete posterior sobrevive.

---

### Recorte / Pocket (CATIA: Pocket / SW: Extruded Cut / Inventor: Extrude Cut / Fenkra: `Extrude` com `BoolOp::Cut`)

**O que faz.** Remove material extrudando um perfil.

**CATIA** [V]
- As opções do Pad se repetem (limites, perfil de superfície, Multi-Pocket).
- O Pocket de superfície remove material nos dois lados da superfície quando os limites são "plano" e "cota".
- "Up to Next" no topo do furo/bolso sempre para na próxima face encontrada.
- O Multi-Pocket cria uma feature com um bolso por domínio.
- O Drafted Filleted Pocket tem saída e filetes, com o segundo limite obrigatório.

**SolidWorks** [V]
- Cut-Extrude tem as mesmas condições do boss.
- "Flip side to cut" corta fora do perfil [C].
- Feature scope escolhe os corpos cortados.
- Cut-Extrude é permitido em corpos múltiplos.

**Inventor** [V]
- Cut usa o mesmo painel.
- Through All tem Flip e não oferece Join.
- Em multicorpo há seletor de participantes.

**Fenkra hoje**
- O mesmo código da extrusão, com `into = (op == Cut) != reverse` (lib.rs:2725).
- Faltam "cortar fora do perfil" e participantes. Um corte atinge o corpo do coletor alvo, inteiro.
- `ThroughAll` usa `diag*2 + 10` como comprimento (lib.rs:1697). Funciona, mas é um número mágico dependente da caixa.

**Melhor prática**
- O corte precisa de **corpo-alvo explícito** (participantes) quando houver mais de um coletor.
- Aviso se o corte não remove volume (hoje o resultado é silencioso).
- **Teste:** o corte que não toca o corpo deve marcar aviso "não removeu material".

---

### Limites de término: até próximo / último / plano / superfície / vértice / corpo, offset e espelhado (CATIA: Limit / SW: End Condition / Inventor: Extents / Fenkra: `Extent`)

**O que faz.** Define onde a extrusão, o furo ou a rosca termina.

**CATIA** [V]
- **Up to Next.** Para na primeira face detectada, que precisa interromper toda a extrusão, e o furo precisa atravessar material.
- **Up to Last.** Termina na última face.
- **Up to Plane.** O plano também define a direção.
- **Up to Surface.** Tem campo Offset: distância entre a superfície e a face de topo. Aceita faces do mesmo corpo ou do Part Body.
- **Mirrored extent / segundo limite.** Cada lado tem seu próprio tipo de limite.
- **Restrição.** Merge Ends não vale com Up to Plane/Surface.
- **Queixa de usuário.** Um relato diz que "Up to next" com offset escolheu a face errada em um degrau concêntrico. A saída sugerida foi Up to Surface/Plane. [V]

**SolidWorks** [V]
- Up To Vertex, Up To Surface, Offset From Surface, Up To Body, Up To Next.
- A condição é dinâmica: o comprimento acompanha a geometria.

**Inventor** [V]
- To (ponto, vértice, face ou plano), To Next com terminador, Between.
- "Extend face to end feature" estende a face-alvo além do plano quando necessário.
- Alternate Solution escolhe perto ou longe.

**Outros.** Onshape tem Up to face, Up to part e Up to vertex. Um relato de fórum diz que "Through all" ultrapassa o fim da peça. [V]

**Fenkra hoje**
- `end_cutter` (lib.rs:2554) devolve uma superfície de corte e se ela deve ser estendida. `UpTo(plano)`, `UpToSurface(Plane)` e `UpToNext` não estendem. `UpToSurface` de face/superfície estende.
- `UpToNext` usa o corpo inteiro como "próximo". Só o Add usa `extrude_to_next` (lib.rs:2756).
- **Falta** offset, vértice, corpo, término por lado e "estender face" como opção visível (hoje depende do tipo de `SurfRef`).
- **Risco.** `reach()` e `through()` dependem da caixa envolvente e de `1e4`/`1e5` fixos (lib.rs:2556,2577). Modelos muito grandes ou muito pequenos podem passar da tolerância.

**Melhor prática**
- **Tabela única de término** para extrusão, recorte, furo, rosca, revolução e nervura (hoje há `extent()` e `end_cutter()` separados). Cada término declara se admite offset, extensão e direção própria.
- **Regra de segurança.**
  - Quando o alvo não intersecta o perfil inteiro, mostrar o motivo ("a face não cobre o perfil") em vez de truncar.
  - Quando "Next" é ambíguo, mostrar as candidatas no 3D.
- **Casos de teste.**
  - Até próximo em corpo com 2 degraus.
  - Até superfície cilíndrica, com e sem "estender".
  - Offset positivo e negativo.
  - Alvo removido: referência perdida com reparo.
  - Dois lados com tipos diferentes.

---

### Parede fina (CATIA: Thick / SW: Thin Feature / Inventor: Extrude com perfil aberto + Match Shape / Fenkra: `thin`, `thin_side`)

**O que faz.** Gera uma parede a partir de um perfil aberto ou fechado.

**CATIA** [V]
- Thickness1 e Thickness2 (um para cada lado) e Neutral Fiber (distribui o total igualmente).
- Merge Ends apara contra o material existente e não está disponível com Up to Plane/Surface.
- A opção de parede fina não vale para superfície.

**SolidWorks** [V]
- Perfil aberto ativa Thin Feature sozinho.
- Os tipos (um lado, plano médio, dois lados), a tampa nas pontas e o auto-filete dos cantos são [C].

**Inventor** [C]
- O perfil aberto pode virar sólido com Match Shape.
- A espessura da nervura vale em um lado ou nos dois.

**Fenkra hoje**
- `profile_face(frame, sk, thin, side)` (lib.rs:2246) gera a parede. `thin_side` 0, 1 ou 2 (simétrica).
- Na `Sweep`, a parede só existe para contornos fechados: `if sk.loops.is_empty() → erro` (lib.rs:2938). Um perfil aberto com `thin` não funciona na varredura.
- Faltam Thickness1 ≠ Thickness2 e "Merge Ends".

**Melhor prática**
- Unificar `thin: Option<String>` + `thin_side` em `Thin { t1, t2, neutral, merge_ends, cap }`. A mesma estrutura serve a extrusão, revolução, varredura e nervura.
- **Casos de teste.**
  - Perfil aberto em U com 2 mm: volume = comprimento × 2 × altura.
  - Espessuras assimétricas.
  - Varredura de perfil aberto.

---

### Saída (taper) na extrusão (CATIA: Drafted Filleted Pad / SW: Draft on/off / Inventor: Taper A / Fenkra: `taper`)

**CATIA** [V]
- Pad drafted/filleted: ângulo, segundo limite como elemento neutro, três filetes opcionais.
- Cria na árvore um Pad, um Draft e três Fillets, cada um editável.

**SolidWorks** [V]
- O draft se aplica a todas as faces menos a de cima e a de baixo.
- "Draft outward" inverte.
- O draft por direção pode ser definido nas duas direções.
- Para saída seletiva, usar a feature Draft.

**Inventor** [V]
- Taper A mede-se normal ao plano do sketch, em ambas as direções.
- Positivo aumenta a seção.

**Fenkra hoje**
- `taper` só com Blind (erro explícito, lib.rs:2765) e ignorado nos demais (bug provável 1 acima).
- O atalho `ExtrudeDraftFillet` (app.rs:1720) faz saída 3° + filete 1 mm com `CreatedBy(ex)`, que hoje aplica a todas as arestas (lib.rs:4184).

**Melhor prática**
- Saída com plano neutro configurável (início, meio, fim) e "para fora".
- Implementar `CreatedBy` de verdade antes de manter o atalho. Hoje ele filetea o corpo inteiro.
- **Teste:** extrusão com saída sobre uma peça com outras features: só as arestas da extrusão recebem filete.

---

### Multi-pad / contornos selecionados / direção não normal (CATIA: Multi-Pad, Pad not normal / SW: Selected Contours, Direction of Extrusion / Inventor: seleção de regiões / Fenkra: parcial)

**CATIA** [V]
- Multi-Pad: todos os perfis fechados e não intersectantes.
  - A lista de domínios aceita comprimento por domínio ou comum (multisseleção).
  - O resultado é uma única feature com um pad por domínio.
  - Sketch alterado → aviso; editar a feature → erro de definição.
- Pad não normal: desmarca "Normal to profile" e escolhe uma linha como referência.

**SolidWorks** [V]
- Contornos selecionados reutilizam um sketch em extrusões de alturas diferentes.
- Direção por aresta/referência fica associada.

**Inventor** [V]
- Seleção de regiões por janela.
- Sketch compartilhado entre features.

**Fenkra hoje**
- Direção por `AxisRef` existe (`dir`).
- Contornos selecionados e comprimento por contorno não existem.

**Melhor prática**
- Contornos por **assinatura** (centroide + área), não por índice, para sobreviver a edição do sketch.
- Contorno novo: estado "novo contorno sem comprimento" (como o domínio com 0 mm do CATIA), nunca extrudar com valor adivinhado.
- **Teste:** sketch com 3 contornos, extrusões com alturas distintas; remover um contorno vira diagnóstico.

---

### Revolução / Ranhura (CATIA: Shaft / Groove / SW: Revolved Boss/Cut / Inventor: Revolve / Fenkra: `Revolve`)

**CATIA** [V]
- O perfil pode ser aberto ou fechado.
- Vários perfis não intersectantes ficam do mesmo lado do eixo.
- Se perfil e eixo estão no mesmo sketch, não é preciso selecionar o eixo.
- Primeiro e segundo ângulo (padrão 360°), arrastáveis (LIM1/LIM2).
- Reverse side vale para perfil aberto.
- Sem material para aparar → aviso para editar os parâmetros padrão.

**SolidWorks** [C]
- Ângulo, direção 2, perfil fino, contornos selecionados.

**Inventor** [V]
- Angle A e Angle B (assimétrico).
- Full revolve, To Next (indisponível em feature-base) e Between.
- A booleana usa Join, Cut ou Intersect.

**Fenkra hoje**
- `Revolve {axis, angle, symmetric, op, thin, angle2}` (doc.rs:2527; lib.rs:2778).
- Ângulo 2 valida soma ≤ 360. Ângulo único > 360 não é validado. Não há término "até plano/próximo". Perfil que cruza o eixo não é verificado antes de ir ao kernel (provável erro genérico do OCCT).

**Melhor prática**
- Validar antes do kernel: perfil coplanar com o eixo e não cruzando o eixo. A mensagem deve ser em português e apontar o contorno.
- **Teste:** perfil que cruza o eixo; ângulo 400; revolução simétrica; ranhura em eixo existente.

---

### Furo — todos os tipos, normas e posicionamento (CATIA: Hole / SW: Hole Wizard, Hole, Advanced Hole / Inventor: Hole / outros: Fusion Hole, Onshape Hole / Fenkra: `Hole`)

**O que faz.** Gera furos paramétricos (simples, rebaixados, escareados, roscados) em um ou mais pontos.

**CATIA** [V]
- **Tipos.** Simple, Tapered, Counterbored, Countersunk e Counterdrilled.
- **Restrições dos diâmetros.**
  - Rebaixo > furo, e furo mais fundo que o rebaixo.
  - Escareado: ângulo entre 0 e 180°.
- **Escareado.** Define-se por dois de três (profundidade, ângulo, diâmetro).
- **Extension.** Blind, Up to Next/Last/Plane/Surface, com offset no limite.
- **Fundo.** Plano ou em V (ex.: 110°).
- **Direção.** Padrão normal à face. Desmarcar "Normal to surface" permite escolher uma aresta ou linha.
- **Posição.** Seleciona-se uma referência (aresta circular + face), uma cota posiciona o furo concêntrico, ou edita-se o sketch de posicionamento (o ponto central).
- **Rosca.** A aba Thread Definition tem No Standard, Metric Thin Pitch e Metric Thick Pitch (valores AFNOR). Aceita tabela própria (Excel/Lotus/texto). Há rosca à esquerda, e o diâmetro do furo não pode passar do diâmetro da rosca. A rosca é só um ícone na árvore, sem geometria.
- **Subtleza.** O topo do furo para na próxima face. Com um filete acima da face escolhida, o topo sobe para o filete.

**SolidWorks** [V]
- **Hole Wizard.** Duas abas, Tipo e Posições. Os estilos incluem Counterbore, Countersink, Hole, Straight Tap, Tapered Tap e Legacy. Versões novas trazem os tipos em slot. [V]
- **Tamanho e ajuste.** Size + Fit (Close/Normal/Loose) vêm do Toolbox, editáveis. O "Custom Sizing" é usado com parcimônia.
- **Posições.** Face plana ou sketch 3D para superfícies não planas. Pontos de sketch com relações e cotas. Cada tamanho exige uma feature.
- **Smart Fasteners.** O estilo "Hole" é reconhecido pelos Smart Fasteners.
- **Valores calculados.** O contra-escareado do lado oposto tem valores calculados (broca + 0,05 mm ou 0,06 pol) que não podem ser alterados, mas há Favoritos. [V]
- **Profundidade.** Desde a versão 2020 pode-se definir até a ponta ou até o ombro. [V]
- **Advanced Hole.** Empilha elementos (rebaixo, escareado, reto, rosqueado) do lado próximo e do lado distante, e define profundidade a partir de uma superfície com offset. [V]

**Inventor** [V]
- **Posicionamento.** Pontos de sketch, face ou plano de trabalho, ponto + direção, Linear (cotas), Concentric (aresta ou face curva). "Allow Center Point Creation" restringe a pontos existentes.
- **Tipo e assento.** Simple, Clearance, Tapped e Taper Tapped. O assento é Counterbore, Spotface ou Countersink.
- **Ajuste.** Em Clearance: Normal/Close/Loose conforme o parafuso.
- **Término.** Distance, Through All e To. A direção 1 vai para dentro da face, a direção 2 inverte, e Simétrico só vale em furo simples passante.
- **Ponta.** Flat, Flat W, Angle, Angle V, Angle Y.
- **Rosca.** Tipo (ANSI Unified/Métrica/NPT/ISO cônica interna), tamanho, designação (passo), classe, Full Depth e mão esquerda/direita.
- **Avançado.** iMate automático e "Extend Start".

**Fenkra hoje**
- **Modelo.** `Hole {sketch, kind, diameter, depth: Extent, thread, thread_modeled, standard, points, dir, reverse, tip, taper, left_hand}` (doc.rs:2552). `HoleKind` tem Simples, Rebaixado, Escareado e Contrafurado (doc.rs:2543). O cálculo está em lib.rs:2799-2935.
- **O que faz bem.**
  - Furo inteligente: o centro fora da peça desce pela direção até a superfície e erra com mensagem se apontar para fora (lib.rs:2837-2853).
  - Pontos 3D projetados na superfície mais próxima, normal à superfície.
  - Rosca cosmética ou modelada (sulco helicoidal), à esquerda por espelho.
  - Tabela de furos (`hole_table`) e `MetricSize` com tap drill, folga e rebaixo para M3–M20 (hardware.rs:19-27).
- **O que falta.**
  - `standard` é só texto informativo. O diâmetro não deriva do tamanho/ajuste, então o usuário pode ter "M8 roscado" com diâmetro 3 mm e o erro só aparece como nota.
  - O padrão do comando só nasce em "M6" (app.rs:1781). Faltam tipos em slot, spotface, roscas cônicas (NPT/BSP) e profundidade até a ponta/ombro.
  - Lado oposto e empilhamento (Advanced Hole) não existem.
  - Furo por vértice usa `PointRef::Fixed` (não associativo).
- **Bug provável.** Com término `Symmetric`/`TwoSides`, o furo usa só o primeiro valor (`self.extent(x)?.0`, lib.rs:2882). Um furo simétrico de 20 mm resultaria em 10 mm. Deveria ser erro ou ser suportado.

**Melhor prática**
- **Fluxo.**
  1. Posição: pontos de sketch/vértice/concêntrico/linear. Ponto 3D deve ser associativo à geometria (referência v2).
  2. Modo: tipo (simples, folga, roscado, cônico) × assento (nenhum, rebaixo, escareado, spotface).
  3. Tamanho: norma, tamanho e ajuste definem todos os diâmetros. Só "Custom" permite editar (e marca o furo como "personalizado").
  4. Término: cego, passante, até… e fundo (plano/ângulo).
  5. Avançado: direção, rosca modelada/cosmética, mão.
- **Regras para evitar bugs simples.**
  - O diâmetro do roscado vem da tabela. Se for editado, o rótulo muda para "personalizado".
  - Furo passante num corpo sem material no caminho: aviso, não erro.
  - A rosca nunca começa "no vazio" (já tratado em lib.rs:2916).
- **Casos de teste.**
  - Tabela M3–M20: volume removido bate com a fórmula.
  - Furo escareado e depois casca (DR-02/B48).
  - Furo em face inclinada.
  - Furo simétrico.
  - Mudar a espessura da placa: furo passante continua passante.
  - Padrão de furos roscados.
  - Furo por vértice depois que o vértice se move.

---

### Rosca (CATIA: Thread/Tap / SW: Cosmetic Thread, Thread / Inventor: Thread, Coil / Fenkra: `Thread`, `Hole.thread`)

**CATIA** [V]
- Cria rosca em furo ou eixo a partir de uma face cilíndrica.
- Existe análise de rosca.
- O furo roscado não mostra a rosca na geometria, só um ícone.

**SolidWorks** [V]
- Cosmetic Thread (rosca cosmética).
- A "Thread" modelada usa perfil próprio [C].

**Inventor** [V]
- **Faces.** Uma ou mais faces do mesmo diâmetro e contíguas. Na rosca parcial, escolhe-se a face perto da borda onde começa.
- **Especificação.** Type, Size, Designation (passo) e Class.
- **Profundidade.** O padrão "Depth" cobre a face inteira; desligado, usa-se um Offset.
- **Direção.** Serve para a nota de rosca, não altera a geometria.
- **Exibição.** Aparência (textura), não geometria, por desempenho. "Display thread in model" pode ser desligado. Para furo, usa-se o comando Hole.
- **Coil.** Move um perfil ao longo de uma hélice, para molas ou roscas modeladas.

**Fenkra hoje**
- `Thread {face, spec, length, modeled, end}` (doc.rs:2457; lib.rs:3984).
- A rosca cosmética devolve só um aviso de texto. Não existe objeto "rosca cosmética" para desenho ou lista de furos além de `hole_table`.
- A face é um ponto clicado (`[f64;3]`), sem chave v2 (ADR 0010, Fase 2).
- Faltam deslocamento inicial, chanfro de entrada, perfil e passo personalizados, preview (B05), rosca em várias faces e classe de ajuste.

**Melhor prática**
- Rosca cosmética como **atributo da face** (designação, comprimento, classe), visível no desenho. Rosca modelada só sob demanda (custa booleana helicoidal).
- **Teste:** mudar o diâmetro do eixo e a rosca acompanhar ou falhar com diagnóstico "Ø ≠ nominal" (hoje existe a nota em lib.rs:3998).

---

### Varredura — nervura e ranhura por curva (CATIA: Rib / Slot / SW: Swept Boss/Cut / Inventor: Sweep / Fenkra: `Sweep`)

**CATIA** [V]
- **Perfil e curva.** O perfil pode ser fechado ou aberto (aberto só se material existente o limitar). Vários contornos não intersectantes são aceitos.
- **Curva-guia.** Aberta, fechada plana ou 3D fechada (3D precisa ser contínua em tangência).
- **Controle do perfil.**
  - Keep angle: mantém o ângulo entre o plano do perfil e a tangente da curva.
  - Pulling direction: bom para hélices, escolhe-se o eixo.
  - Reference surface.
- **Opções.** Merge Ends, Thick Profile (Thickness1/2) e Neutral Fiber.

**SolidWorks** [C]
- Perfil + caminho + curvas-guia, orientação, torção, "Twist along path", tangência.

**Inventor** [V]
- Orientação **Path** (fixa em relação ao caminho) ou **Parallel**.
- **Taper.** Normal ao plano do sketch. Não existe em caminho fechado.
- **Twist.** Só em sweep perpendicular.
- **Path & Guide Rail.** Controla escala e torção.
- **Path & Guide Surface.** A normal da superfície controla o giro.

**Fenkra hoje**
- `Sweep {sketch, spine, mode, op, thin}` (doc.rs:2295; lib.rs:2936). Modos: Corrigido (Frenet corrigido), Fixo e Frenet.
- A guia vira polilinha de pontos (`spine.pts`).
- Sem roles de face (`apply` sem papéis), então filetes posteriores nas faces da varredura têm só o ponto como referência.
- Sem torção, escala/taper, guia secundária nem controle de perfil.
- Perfil aberto não funciona (ver "Parede fina").
- Sem verificação de que o perfil está no início da curva e normal a ela.

**Melhor prática**
- Aviso se o plano do perfil não é normal à tangente no ponto de início, com oferta de "alinhar".
- Estender `Sweep` com guia secundária (`SweepKind::Explicit` já existe nas superfícies, lib.rs `sweep_x` em 1973) e reaproveitar para sólidos.
- **Casos de teste.**
  - Tubo em curva 3D.
  - Perfil quadrado em hélice (Frenet vs. Corrigido).
  - Curva fechada.
  - Remoção do segmento da guia: diagnóstico.

---

### Loft / Multi-seções (CATIA: Multi-Sections Solid / SW: Lofted Boss / Inventor: Loft / Fenkra: `Loft`)

**CATIA** [V]
- **Entradas.** Seções (não podem se intersectar), guias (precisam estar unidas) e espinha (calculada ou do usuário).
- **Acoplamento (coupling).** Ratio, Tangência, Tangência-curvatura e Vértices.
- **Relimitação.** Início, fim, ambos ou nenhum. Os limites dependem da espinha: do usuário → extremidades da espinha; calculada sem guias → seções; calculada com guias → extremidades das guias.
- **Remove Lofted Material** é o equivalente de corte.

**SolidWorks** [C]
- Seções, curvas-guia, linha central, condições de início/fim e tensão.

**Inventor** [C]
- Trilhos, linha central, área, condições de contorno por seção (tangente/suave G2).

**Fenkra hoje**
- `Loft {sections, ruled, closed, op}` (doc.rs:2305; lib.rs:2955).
- Só usa `sk.loops[..1]` de cada seção. Contornos internos (furos) são descartados em silêncio.
- Sem guias, espinha, acoplamento, pontos de fechamento, tangência ou tensão (B12).
- Ordem das seções = ordem da lista. A orientação e o ponto de partida das seções não são controlados, então o resultado pode torcer.

**Melhor prática**
- Mostrar no 3D os pontos de acoplamento (como o CATIA) e permitir mover o ponto de partida.
- Avisar quando uma seção tem mais de um contorno e o loft descarta o resto.
- **Casos de teste.**
  - Círculo → quadrado.
  - Três seções.
  - Fechado.
  - Seções invertidas: a torção é detectada e avisada.

---

### Nervura / Stiffener (CATIA: Stiffener / SW: Rib / Inventor: Rib e Web / Fusion: Rib e Web / Fenkra: `Stiffener`)

**CATIA** [V]
- **From Side** (padrão): extrusão no plano do perfil, espessura normal a ele.
- **From Top:** extrusão normal ao plano, espessura no plano. O resultado é sempre aparado pelo material existente e atende redes de linhas.
- Perfil aberto precisa de material que o limite totalmente.
- **Neutral Fiber** distribui a espessura nos dois lados. Desligada, aceita Thickness 1 e 2 diferentes.

**SolidWorks** [C]
- Rib com espessura num lado, nos dois ou simétrica; direção paralela ou normal ao sketch; "Next reference"; draft.

**Inventor** [V]
- Rib (paralela ao plano) e Web (normal ao plano).
- Espessura num lado ou dividida; extensão To Next ou Finite.
- "Extend Profile" quando as pontas não tocam a peça.
- Taper e as abas Draft (Hold Thickness At Top/Root) e Boss (bosses nos pontos do perfil).

**Fenkra hoje**
- `Stiffener {sketch, thickness, depth, reverse, to_body}` (doc.rs:2434; lib.rs:3963).
- Usa só `sk.lines` (lib.rs:3966). Arcos e splines do sketch são ignorados em silêncio.
- A espessura é simétrica em torno da linha, sem lado (B11/DR-05).
- Só o modo "From Top" (extrusão normal ao plano). O modo "From Side" — a nervura clássica — **não existe**.
- Sem extensão do perfil, saída ou bosses.

**Melhor prática**
- Dois modos explícitos, "Paralelo ao sketch" e "Normal ao sketch" (nomes do Inventor), com espessura por lado e saída.
- Todo tipo de curva do sketch entra na nervura (ou avisa o que foi ignorado).
- **Casos de teste.**
  - Nervura diagonal em canto, de ponta a ponta, até o corpo.
  - Nervura com arco.
  - Mudança do corpo: a nervura recalcula até a nova parede.

---

### Filete de aresta — constante (CATIA: Edge Fillet / SW: Fillet Constant Size / Inventor: Fillet Constant / Fenkra: `Fillet` com `EdgeMode::Constant`)

**CATIA** [V]
- **Propagação.**
  - *Minimal*: ignora tangências, filetea só parte da aresta.
  - *Tangency*: segue as arestas tangentes.
- **Raio.** Constante, aplicado a todas as arestas.
- **Trim ribbons.** Só na propagação por tangência; apara os filetes sobrepostos.
- **Limiting element.** Plano, face ou superfície que limita o filete.
- **Edges to keep.** Em filetes de canto redondo, as arestas a manter são marcadas explicitamente.
- **Blend corner(s)** está em More (detalhes na página de canto).
- **Seleção por face.** Escolher uma face filetea suas arestas.
- Preview e limiting element só no Part Design P2.

**SolidWorks** [V]
- **Tipos.** Constant Size, Variable Size, Face e Full Round.
- **Opções.** Multiple radius, round corner e setback.
- **FilletXpert.** Organiza e reordena filetes simétricos de raio constante.
- **Perfil e overflow** (keep edge/keep surface) [C].

**Inventor** [V]
- **Tipos.** Edge, Face e Full Round, com os comportamentos Constant, Variable e Setback.
- **Seleção.** Aresta, laço, feature ou sólido, e All Fillets/All Rounds.
- **Continuidade.** Tangente (G1), Suave (G2) ou Invertida.
- **Avançado.**
  - Roll Along Sharp Edges: o raio varia para preservar as arestas vizinhas.
  - Rolling Ball Where Possible.
  - Automatic Edge Chain.
  - Preserve All Features.
- Trocar edge por face/full round exige apagar e refazer. Apagar a única aresta de um filete de uma aresta dá erro; num conjunto multi-aresta o filete atualiza enquanto sobrar uma.

**Fenkra hoje**
- `EdgeOp {value, edges: EdgeSel, mode, prop: EdgeProp, keys}` (doc.rs:2629). `EdgeProp::Tangente` e `Minima` espelham a propagação do CATIA. `Minima` pós-processa em `minimal_edges` (lib.rs:1923).
- Arestas por ponto médio + chave v2 (`EdgeKey` = aresta entre duas faces). Aresta perdida ou ambígua dá erro "referência perdida" (ADR 0010).
- `AllOfBody` (todas as arestas) e `Edges(vec)`. Com vetor vazio, avisa que nada foi aplicado.
- **Bug conhecido.** `EdgeSel::CreatedBy(_)` aplica a **todas** as arestas e só emite aviso (lib.rs:4183-4185).
- **Falta.**
  - Raios múltiplos num comando e setback.
  - Perfil cônico, G2 e invertido.
  - Limites, "arestas a manter", "trim ribbons" e overflow.
  - Seleção por face/laço/feature.
  - Um arquivo antigo com filete por ponto "perde" a aresta quando o perfil muda, até a promoção v2.
- A tolerância de casamento é 5% da diagonal da caixa, mínimo 1 mm (lib.rs:4153): em peças pequenas pode casar a aresta errada.

**Melhor prática**
- **Fluxo.**
  1. Suporte: aresta, laço, face, feature ou "criadas por".
  2. Modo: constante, variável, setback.
  3. Parâmetros: raio, e raios por conjunto.
  4. Avançado: propagação, perfil, overflow, limites.
- **Regra.** Nunca escolher outra aresta em silêncio. Já adotado na Fase 1.
- Implementar `CreatedBy` por `face_owner` + papel (Fase 2 do ADR).
- **Casos de teste.**
  - Filete + mudar o perfil do recorte (já testado).
  - Filete em aresta cuja face some → erro.
  - Raio maior que a parede → diagnóstico claro.
  - `CreatedBy` com 2 extrusões: só as arestas do dono.
  - Propagação mínima vs. tangente em borda com concordância.
  - Todos os filetes de uma feature.

---

### Filete variável (CATIA: Variable Radius Fillet / SW: Variable Size / Inventor: Variable / Fenkra: `EdgeMode::Variable`)

**CATIA** [V]
- Cada aresta tem dois vértices com raio. Pode-se adicionar pontos com raios próprios, vindos de cliques, planos (interseção com a aresta) ou pontos 3D.
- **Variação.** Cúbica (padrão) ou linear. Raio zero é permitido.
- **Spine.** Permite filetear arestas consecutivas sem tangência como uma só lógica (exige licença GSD).
- **Restrições.** Em arestas fechadas, remove-se o vértice padrão. O modo linear não vale para aresta fechada nem contínua em tangência.

**SolidWorks / Inventor** [V]
- **Inventor.** Raios inicial e final e pontos intermediários (posição de 0 a 1), com transição suave ou linear. Avisos de sobreposição.
- **SolidWorks.** Pontos de controle definem a variação [V].

**Fenkra hoje**
- `Variable(String)`: raio no início = valor, no fim = o campo (doc.rs:2658).
- O "início" e "fim" dependem da orientação da aresta no OCCT, que o usuário não vê. Isso é um risco de resultado trocado.
- Sem pontos intermediários (B49).

**Melhor prática**
- Pontos de controle com posição proporcional (0..1) e setas de orientação visíveis no 3D.
- **Teste:** aresta reta com 3 pontos (3, 6, 3): a seção no meio mede 6.

---

### Filete face-face, tritangente e completo (CATIA: Face-Face / Tritangent Fillet / SW: Face Fillet / Full Round / Inventor: Face Fillet / Full Round / Fenkra: `FaceFace`, `FullRound`)

**CATIA** [V]
- **Tritangente.** Precisa de três faces: duas de apoio e a face a ser removida (o filete é tangente a ela). Aceita elemento limitante (plano, face ou superfície).
- **Face-Face.** Filete entre duas faces não adjacentes.

**SolidWorks** [V]
- Face Fillet junta faces não adjacentes e não contínuas.
- Full Round dispensa o raio: o software o calcula pelas seleções.

**Inventor** [V]
- Face Fillet entre dois conjuntos de faces.
- Full Round: variável, tangente a três conjuntos de faces e substitui o conjunto central.

**Fenkra hoje**
- `FaceFace(a, c)` e `FullRound {mid, a, b}` guardam pontos 3D crus (doc.rs:2665-2667), sem `FaceKey` (ADR 0010 Fase 2 prevê).
- `FullRound` exige duas faces laterais planas e calcula a largura pela distância entre elas (lib.rs:4132-4136). Faces laterais não paralelas ou não planas dão erro.
- `AllOfBody` serve de marcador, sem sentido real nesses modos.

**Melhor prática**
- Chaves v2 de face nos dois modos.
- **Casos de teste.**
  - Full round em nervura de 4 mm.
  - Face-face entre faces separadas por um furo.
  - Mudar a largura da nervura: o filete acompanha.
  - Faces paralelas é pré-condição: mensagem específica se não forem.

---

### Setback e cantos de filete (CATIA: Blend corner / SW: Setback, Round corner / Inventor: Corner setback / Fenkra: ausente)

**CATIA** [V]
- "Blend corner(s)" fica em More. Os detalhes estão em página própria (Reshaping Corners) e a página não menciona distância de setback.

**SolidWorks** [V]
- Setback e round corner na lista de tipos.

**Inventor** [V]
- Onde três ou mais arestas de raio constante se encontram num vértice, define-se uma distância de setback por aresta.
- "Minimal" calcula o setback mínimo, que costuma dar a transição mais suave e maior taxa de sucesso.

**Fenkra hoje.** Não há comando. Se o OCCT falha no canto, o erro é do kernel.

**Melhor prática.** Setback por vértice, com opção "mínimo automático". **Teste:** cubo com três arestas filetadas no mesmo vértice; setback 2 mm.

---

### Chanfro (CATIA: Chamfer / SW: Chamfer / Inventor: Chamfer / Fenkra: `Chamfer`)

**CATIA** [V]
- Padrão Length1/Angle; alternativa Length1/Length2.
- Seleciona-se arestas ou uma face (chanfra suas arestas).
- Pode adicionar ou remover material.
- A página não detalha propagação, mas o modelo é o mesmo do filete.

**SolidWorks e Inventor** [C]
- Distância × ângulo, duas distâncias, distância igual, vértice.
- Inventor tem Distance, Distance and Angle, e Two Distances.

**Fenkra hoje**
- `Chamfer(EdgeOp)` com `Constant`, `TwoDist` e `DistAngle` (doc.rs:2660-2663).
- O comprimento "1" e "2" dependem da orientação da aresta, sem indicação visual.
- Sem chanfro de vértice, sem chanfro variável e sem limite.

**Melhor prática.** Mostrar qual face recebe d1 (seta no 3D) e permitir "inverter". **Teste:** distância × ângulo 45° equivale a duas distâncias iguais.

---

### Draft — neutro, partição, reflect line, variável (CATIA: Draft Angle / Draft with Parting Element / Reflect Line / Variable Angle / SW: Draft (Neutral Plane, Parting Line, Step Draft), DraftXpert / Inventor: Draft / Fenkra: `Draft`)

**CATIA** [V]
- **Faces a inclinar.** Por face neutra (CATIA descobre as faces, em vermelho) ou seleção explícita (em rosa).
- **Elemento neutro.** Define a curva neutra. A primeira face define a direção de extração, que por padrão é a normal da face neutra e fica "controlada por referência".
- **Ângulo.** Pode valer por face. O padrão é 5°.
- **Partição (parting element).** Plano, face ou superfície que divide a peça em duas. "Parting = Neutral" reaproveita o neutro. "Draft both sides" inclina nos dois sentidos. Ângulos negativos são permitidos.
- **Propagação.** Nenhuma ou Suave (inclui faces tangentes).
- **Limiting elements.** Planos ou faces que cortam totalmente a face inclinada.
- **Forma "Square".** Resolve ângulo maior que o das faces vizinhas, sem garantir desmoldagem.
- **Reflect line.** As linhas de reflexão da face (ex.: cilindro) servem de linha neutra. Funciona depois de filetes. Aceita partição e elementos limitantes.
- **Ângulo variável.** Pontos na aresta (cliques, planos ou pontos 3D), cada um com seu ângulo. Mudar faces ou neutro apaga os pontos. Faces não contínuas em tangência não podem ser multisselecionadas.
- **Falha.** Faces torcidas: o CATIA oferece Deactivate + Extract Geometry. O novo draft depois da primeira usa por padrão a mesma direção de extração.

**SolidWorks** [V]
- **Neutral Plane.** A superfície gira em torno do plano neutro, mesmo sem cruzá-lo. Propagação: nenhuma, ao longo da tangente, todas as faces, faces internas ou externas.
- **Parting Line.** Gira em torno de uma aresta (geralmente de uma Split Line). Cada lado pode ter ângulo próprio e não vale para faces cilíndricas. Desde 2022 permite draft através de linhas de partição numa só feature. "Allow reduced angle" é exclusivo desse tipo.
- **Step Draft.** Cria um degrau na linha de partição, com opção de degraus afunilados.
- **DraftXpert.** Só plano neutro; usa análise de draft dentro do comando para selecionar faces.
- **Draft Analysis.** Verde = positivo, amarelo = precisa de draft, vermelho = negativo.

**Inventor** [V]
- **Tipos.** Fixed Edge, Fixed Plane e Parting Line.
- **Direção e seleção.** A direção de extração vem de face, plano, aresta ou eixo, com Flip. Automatic Face Chain liga faces tangentes.
- **Ângulos.** One Way, Symmetric ou Asymmetric (Draft Angle 2).
- **Automatic Blending** mantém filetes vizinhos no draft.
- **Parting Line.** A ferramenta é um sketch, plano ou superfície. "Move Parting Line" usa Static Edges e Angle for Top/Both/Bottom, com ângulo absoluto ou relativo.

**Fenkra hoje**
- `Draft {faces, neutral: PlaneRef, angle, reverse, face_keys}` (doc.rs:2374; lib.rs:2993).
- A direção de extração é a normal do plano neutro, sem poder separar as duas coisas (neutro × direção).
- Sem partição, reflect line, ângulo variável, degrau, propagação nem análise de draft.
- Sem fallback quando o OCCT falha, só o erro do kernel.
- `faces` vazio → aviso e nada aplicado.

**Melhor prática**
- **Fluxo.**
  1. Suporte: faces (ou "por plano neutro", que as descobre).
  2. Modo: neutro / partição / reflect line.
  3. Parâmetros: ângulo e direção de extração.
  4. Avançado: propagação, limites, ângulo variável.
- **Análise de draft** (cor por face contra a direção) como ferramenta de diagnóstico.
- **Casos de teste.**
  - Bloco com 4 faces a 3°: volume e cotas no plano neutro fixas.
  - Partição no plano médio: duas metades com sinais opostos.
  - Draft em face cilíndrica (reflect line).
  - Draft depois de editar a extrusão: a referência v2 sobrevive.

---

### Casca (CATIA: Shell / SW: Shell / Inventor: Shell / Fenkra: `Shell`)

**CATIA** [V]
- Faces a remover (roxas), "Default inside thickness" e "Other thickness faces" com valor por face.
- Cascas consecutivas pedem o segundo valor menor (metade).
- Corpos multidomínio: uma só casca com uma face por domínio.
- **Falha.** O CATIA oferece **ignorar as faces com problema**, com "Reset ignored faces" ao editar. Espessura externa pode exigir Extract.

**SolidWorks** [C]
- Múltiplas espessuras por face, casca para fora, casca sem face removida (peça oca fechada).

**Inventor** [V]
- Remove Faces com Automatic Face Chain.
- Direção Inside, Outside ou Both Sides.
- Espessura uniforme + Unique Face Thickness.
- **Allow Approximation:** Mean, Never Too Thin, Never Too Thick, Optimized, Specify Tolerance.
- Em multicorpo, escolhe-se os sólidos participantes.

**Fenkra hoje**
- `Shell {faces, thickness, outward, face_keys}` (doc.rs:2363; lib.rs:2984).
- Espessura única, sentido interno/externo, faces por chave v2 (ADR 0010).
- Casca falha com spline e furo escareado (DR-02/DR-07, B06). Sem "ignorar faces", sem aproximação e sem espessura por face.
- Sem faces → aviso e nenhuma ação (não cria peça oca fechada).

**Melhor prática**
- **Estratégia de falha em três degraus:**
  1. Tentar exato.
  2. Oferecer "ignorar faces" listando quais (como o CATIA).
  3. Oferecer aproximação com tolerância (como o Inventor).

  Sempre mostrando o que mudou.
- **Casos de teste.**
  - Caixa com 1 face aberta e 2 mm (volume = V − (a−4)(b−4)(c−2)).
  - Casca com furo escareado.
  - Casca para fora com raio vs. reto.
  - Espessura por face.
  - Mudar a altura: a face aberta continua a mesma (já há teste, lib.rs:7044).

---

### Espessura / Thickness (CATIA: Thickness / SW: Thicken, Offset Face / Inventor: Offset, Thicken / Fenkra: `Thickness`)

**CATIA** [V]
- Seleciona faces e dá um valor. Positivo é o padrão documentado.
- "Other thickness faces" permite valor diferente por face.
- Se uma face não pode ser deslocada, o CATIA oferece ignorá-la.

**SolidWorks e Inventor** [C]
- Move Face / Offset Face e Thicken de superfície.

**Fenkra hoje**
- `Thickness {faces, value, face_keys}`: positivo adiciona, negativo remove, zero → aviso (lib.rs:3004). Implementada por `faces_offset_tool` + booleana.
- Falha em face de sólido (DR-12/B07). Sem valor por face.

**Melhor prática.** Mesmo esquema de "ignorar faces" da casca. **Teste:** +1 mm em face plana = volume + área × 1.

---

### Dividir / Split (CATIA: Split / SW: Split, Cut com superfície / Inventor: Split / Fenkra: `SplitBody`)

**CATIA** [V]
- Seleciona o corpo e o elemento divisor, e fica com um lado.

**SolidWorks e Inventor** [V]
- **Inventor.**
  - *Split Face*: divide faces com sketch 2D/3D, plano de trabalho ou superfície (o 3D precisa cruzar totalmente).
  - *Remove one side*: escolhe Keep Side Default ou Flip.
  - *Split Solid*: parte um sólido em dois, e "Make Part/Components" exporta cada um mantendo associatividade.
- **SolidWorks.** Parte em vários corpos [C].

**Fenkra hoje**
- `SplitBody {surface, flip}` mantém **um lado** (`split_side`, lib.rs:4034). Mantém `fresh_body`, sem papéis de face.
- Não existe "manter os dois lados em dois coletores" nem divisão de faces.

**Melhor prática.** Opção "manter os dois": o lado extra vai para um coletor novo e nomeado. **Teste:** dividir ao meio: V₁ + V₂ = V.

---

### Booleanas e múltiplos corpos (CATIA: Insert Body, Add/Remove/Intersect/Assemble/Union Trim/Remove Lump / SW: Combine, Intersect, Move/Copy, Merge result, Feature scope / Inventor: Combine, Derive, Make Part / Fenkra: `Combine`, coletores)

**CATIA** [V]
- **Insert Body.** O novo corpo fica sublinhado como corpo ativo. Part Body e Body.N são autônomos até serem combinados por Assemble, Add, Trim, Intersect, Remove ou Remove Lump.
- **Define In Work Object** vem antes de uma booleana: a operação depende do objeto em trabalho no momento da criação.
- **Union Trim.**
  - Regra 1: só os corpos escolhidos são removidos.
  - Regra 2: o corpo mantido remove os demais.
  - Regra 3: REMOVE é desnecessário se KEEP existir.
  - Faces a remover (rosa), a manter (azul).
- **Remove Lump.** Remove regiões e cavidades involuntárias (ex.: de uma casca).
- **Limitação.** Assemble/Add/Trim/Intersect/Remove/Remove Lump não podem ser reaplicados a corpos já associados. Copiar e colar o resultado contorna.
- **Alterar tipo.** "Change a Boolean Operation into Another One" é um comando contextual.
- **Atenção.** Evitar elementos tangentes (instabilidade).
- **Licenças e Define In Work Object.** Relatos IBM indicam que se um conjunto geométrico é o objeto em trabalho, a booleana volumétrica deixa de estar disponível.

**SolidWorks** [V]
- **Combine.** Add, Subtract (a ordem importa) e Common. Só dentro de um arquivo multicorpo.
- **Merge result** é por feature. O Feature scope limita os corpos afetados.
- **Intersect** e **Move/Copy Body** (este desliga o Merge result sozinho).
- Intersect é usado para obter o volume interno (cavidade) e rodar Mass Properties sobre ele.

**Inventor** [V]
- **Combine.** Basebody, Toolbodies, Keep Toolbody, e Join/Cut/Intersect. Apagar o Combine restaura o corpo consumido.
- **Derive e Move Bodies.** Derive traz componentes como corpos ou toolbodies.
- **Make Part/Make Components.** Exportam corpos como peças associativas.

**Fenkra hoje**
- Um coletor = um corpo (`BodyCollector`, doc.rs:1260). `Feature.target` escolhe o coletor.
- `Combine {tool, op, keep_tool}` (lib.rs:4023): une, subtrai ou intersecta, e remove o corpo da ferramenta salvo `keep_tool`. É o equivalente ao Combine do Inventor.
- `apply_roles` mantém papéis de face no corpo e `None` na ferramenta do Combine, que por isso tem papéis -1.
- Faltam Union Trim, Remove Lump, Change Boolean e participantes por feature.
- O `active_collector` é o "objeto em trabalho" implícito.

**Melhor prática**
- **Regras.**
  - Toda feature mostra no painel o **corpo-alvo** (coletor).
  - O Combine deve poder ser editado depois: trocar união por subtração sem refazer.
  - Para peças do tipo Fabricada, o resultado final deve ser um corpo sólido único. Aviso se há mais de um lump ("N regiões desconexas") e oferta de Remove Lump.
- **Casos de teste.**
  - Dois blocos sobrepostos: V = V₁ + V₂ − V∩.
  - Intersect de esfera com caixa.
  - Subtração com `keep_tool`.
  - Combine com a ferramenta definida depois na linha do tempo: erro "vem depois".
  - Remover o coletor-ferramenta: diagnóstico.

---

### Padrão retangular, circular, por sketch/pontos, por curva e user pattern (CATIA: Rectangular/Circular/User Pattern / SW: Linear/Circular/Curve/Sketch/Table Driven, Fill / Inventor: Rectangular/Circular/Sketch Driven / Fenkra: `Pattern`)

**CATIA** [V]
- **Retangular.** Quatro modos de parâmetro:
  - Instâncias e comprimento;
  - Instâncias e espaçamento;
  - Espaçamento e comprimento.

  Espaçamento desigual por fórmulas em i e j (Knowledge Advisor). A direção de referência é uma aresta ou face plana, e a segunda direção é opcional.
  - *Objeto a repetir.* Uma feature, uma lista (do mesmo corpo) ou o sólido atual.
  - *Keep specifications* preserva o limite (Up to Next etc.) em cada instância, então elas podem ter comprimentos diferentes.
  - *Row in direction*, ângulo de rotação, representação simplificada.
  - *Instâncias.* Remover e restaurar clicando nos pontos.
  - *Restrições.* A primeira feature da lista não pode ser de acabamento (dress-up). A lista não aceita transformações, cascas, divisões ou corpos. Não se acrescenta features a um padrão de feature única. Não se converte lista em sólido atual, nem o inverso. O comprimento não admite fórmula nos modos "…e comprimento".
- **Circular.** Instâncias e ângulo total; instâncias e espaçamento angular; espaçamento angular e ângulo total; coroa completa. A definição de coroa tem círculos e espaçamento. Alinhamento radial opcional. O ângulo total não aceita fórmula. **Apagar uma instância apaga o padrão inteiro;** para manter a geometria usa-se Explode.
- **User Pattern.**
  - Posições são pontos de um sketch, onde se pode desmarcar pontos.
  - A âncora é por padrão o centro de gravidade do elemento, ou um vértice escolhido.
  - Aceita feature, lista ou corpo; "Keep specifications" não vale para lista.
  - A lista aceita acréscimos só se o padrão já for de lista.

**SolidWorks** [V]
- **Seed only** (somente o original na direção 2).
- Pular instâncias (não vale para corpos).
- **Vary sketch** vs. **Geometry pattern.** A geometry pattern copia exato, ignora condições de término e é mais rápida.
- Curve driven, sketch driven e table driven [C].

**Inventor** [V]
- **Escopo.** Retangular em duas direções ou ao longo de um caminho, circular, e sketch driven. Padrão de features individuais **ou do sólido inteiro**, incluindo features que não podem ser repetidas isoladamente.
- **Retangular.** Midplane, Flip, Start e Orientation (Identical ou ao longo da tangente).
- **Circular.** Posicionamento Incremental (espaçamento) ou Fitted (ângulo total, melhor para projetos que mudam). Orientação Rotacional ou Fixa.
- **Sketch driven.** Aceita pontos 2D ou 3D.
- **Join** ou Create New Bodies.
- **Compute.**
  - *Optimized:* copia faces, não aceita sobreposição.
  - *Identical:* replica o resultado original.
  - *Adjust:* recalcula o término de cada instância (preserva a intenção, mas é lento, e uma face plana pode parar o padrão).
- **Supressão.** Pode-se suprimir todas as ocorrências ou uma só.
- **Furo passante.** Num padrão, um furo Through All só fura a parte sólida que atravessa. Se não era o esperado, mude o término para distância.

**Fenkra hoje**
- `Pattern {kind: Linear | Circular | Points, sources}` (doc.rs:2395). Linear tem 2 direções, `total` (comprimento total). Circular tem contagem + ângulo total. Points desloca do ponto-base.
- `replicate` (lib.rs:2644) transforma e funde a ferramenta de cada feature-fonte já calculada (de `self.steps/tools`). Nenhuma re-solução de término. Fonte vazia = corpo inteiro.
- **Falta.**
  - "Manter especificações", "Adjust", supressão de instâncias, espaçamento desigual, padrão por curva, orientação ao longo do caminho, coroas e "seed only".
  - Padrão de features de acabamento, casca ou draft (não geram "ferramenta").
  - Limite de contagem (n enorme trava).
  - Circular parcial: `total/(n−1)` é o espaçamento, de modo que a primeira e a última instâncias ficam nas pontas.
  - `n ≤ 1` vira aviso, mas contagem 0 ou negativa é arredondada para 1 em silêncio.

**Melhor prática**
- Duas políticas explícitas de término: **"Cópia exata"** (rápida, como hoje) e **"Reavaliar término por instância"** (recalcula a ferramenta contra o corpo atual). A segunda é a única que mantém intenção de projeto em padrões de "até próximo". Padrão recomendado para furos: reavaliar.
- Instâncias suprimíveis por índice estável. Furo passante em padrão: aviso quando a instância não toca material.
- **Casos de teste.**
  - 4×3 furos: 12 furos, volume exato.
  - Circular com 360° e com 180° (extremos coincidem com as pontas).
  - Padrão de "até próximo" em degraus: a cópia exata erra e a reavaliada acerta (este é o teste que justifica a opção).
  - Apagar a feature-fonte: padrão em diagnóstico.
  - Padrão por pontos de sketch: mover um ponto move a instância.

---

### Espelho (CATIA: Mirror / SW: Mirror / Inventor: Mirror / Fenkra: `Mirror`)

**CATIA** [V]
- Plano ou face de espelho; objeto = corpo ou lista de features.
- Pode gerar duas porções independentes de material no mesmo corpo.

**Inventor** [V]
- Features, sólido ou novo corpo; "Remove Original" (peças esquerda/direita).
- **Não espelháveis:** features aplicadas ao corpo inteiro (todos os filetes/arredondamentos, casca) e resultados de intersect.

**SolidWorks** [C]
- Espelha features, faces ou corpos, com opção de fundir.

**Fenkra hoje**
- `Mirror {plane, sources}` (lib.rs:3063) usa `replicate` com `Xform::Mirror`. Mesma limitação dos padrões (só features que geram ferramenta). Sem "remover original" e sem "novo corpo".
- Espelho do corpo inteiro funde o espelhado ao original (soma). Se o corpo cruza o plano, o resultado depende da booleana.

**Melhor prática.** "Corpo novo" e "remover original". **Teste:** meia peça espelhada = peça inteira (V × 2).

---

### Transformações — translação, rotação, simetria, afinidade (CATIA: Translation/Rotation/Symmetry/Axis to Axis / SW: Move/Copy Body / Inventor: Move Bodies / Fenkra: `MoveBody`)

**CATIA** [V]
- **Translação.** Direção por linha, normal de plano ou componentes X/Y/Z; distância arrastável. Aplica-se aos corpos correntes e cria "Translat.xxx" na árvore.
- A página não cobre rotação, simetria nem axis-to-axis em detalhe.

**SolidWorks** [V]
- Move/Copy Body desliga o Merge result.

**Inventor** [V]
- Move Bodies posiciona um corpo dentro do documento da peça antes de combinar.

**Fenkra hoje**
- `MoveBody {xf: BodyXf}` com Translate, Rotate, Symmetry e Affinity (doc.rs:1800; lib.rs:3921). Afinidade perde os papéis das faces (`fresh_body`), então referências v2 a faces anteriores caem para o ponto.
- Move o corpo; não copia.

**Melhor prática.** Opção "copiar". Preservar papéis na afinidade quando as fatores forem positivos. **Teste:** transladar e rotacionar 360° volta ao mesmo volume e à mesma caixa.

---

### Escala (CATIA: Scaling / SW: Scale / Inventor: Scale / Fenkra: `Scale`)

**CATIA** [V]
- Resolve o corpo selecionado com razão, em torno de um ponto, plano ou face. Em torno de plano/face com razão ≠ 1, o resultado é uma **afinidade**.

**Fenkra hoje**
- `Scale {center: PointRef, factor}` uniforme em torno do ponto (lib.rs:3954); a afinidade está no `MoveBody`.
- A escala muda as cotas das features anteriores sem alterar suas expressões. Isso pode confundir: as cotas do sketch deixam de valer na leitura.

**Melhor prática.** Aviso no comando: "escala é uma transformação final, não muda as cotas". **Teste:** fator 2 → volume × 8 e área × 4.

---

### Elementos de referência — planos, eixos, pontos (CATIA: Plane/Line/Point, Axis System, Datum / SW: Reference Geometry / Inventor: Work Features / Fenkra: `RefElement`)

**CATIA** [V]
- Guia de Part Design aponta para as páginas genéricas de ponto, linha e plano.
- **Publicar elemento.** Há comando Publication.
- **Datum.** Cria datums (com nota para desligar o modo Histórico). Há também Axis System e Apply Material.
- **Planos.** Offset, Angle/Normal, Through 3 points, 2 lines, Point+Line, Planar curve, Normal to curve, Tangent to surface, Equation, Mean through points [C].

**SolidWorks e Inventor** [C]
- Planos de trabalho, eixos de trabalho e pontos de trabalho. Plano médio entre dois planos, tangente a cilindro, entre outros.

**Fenkra hoje** (doc.rs:472-706)
- **Planos.** 12 tipos: Offset, Ângulo, Face, Paralelo por ponto, 3 pontos, 2 linhas, Ponto+linha, Curva, Normal à curva, Tangente à superfície, Equação, Médio.
- **Pontos.** Coordenadas, na curva, no plano, na superfície, centro, tangente, entre pontos, interseções, projeções e deslocamento.
- **Linhas e curvas.** Linha por pontos, por direção, por ângulo, normal à superfície, entre planos, bissetriz, eixo de face; polilinha, círculo, spline, hélice, cônica, projeção, cópia e junção.
- **Superfícies.** Extração, offset, fill, extrusão, revolução, varredura, loft, blend, junção, divisão, aparar, filete e demais.
- **Publicação e vínculos.** Publicação (`published`), `Linked` e `Isolated`, com estado `LinkState`.
- **Eixo.** Não é elemento de primeira classe: `AxisRef` aponta a linha de sketch, uma linha de referência ou um eixo fixo derivado de aresta.
- **`FacePlane`** fixa origem, X e normal, com `owner` opcional (DR-21, B13). Se a face some, o plano continua vivo.
- `AxisRef::Fixed` e `PointRef::Fixed` são "fixos nesta versão", ou seja, não acompanham a geometria.

**Melhor prática**
- Eixo como elemento próprio (cilindro, aresta, interseção de planos, dois pontos), publicável.
- **Regra de ouro** (já no ADR 0011): oferecer planos/eixos/pontos de referência antes de faces ao criar sketch.
- **Teste:** plano de face depois de a face sumir → "falhou" com diagnóstico (B13).

---

### Define In Work Object, inserir no meio e rollback (CATIA: Define In Work Object, Scan / SW: Rollback Bar, Insert Feature / Inventor: End of Part marker / Fenkra: `rollback`, `marker`, `active_collector`)

**CATIA** [V]
- **Scan / Define in Work Object.** O Scan percorre as features para trás (sketches são ignorados) e "Exit" isola o objeto atual, sublinhado na árvore. Pode-se definir uma feature como local direto pelo menu contextual.
- O guia não detalha as consequências no rollback. Pelo uso, as features novas entram depois do objeto em trabalho [C].

**SolidWorks** [V]
- A barra de rollback não reduz o tempo de reconstrução (só esconde features do gráfico).
- A **freeze bar** reduz o tempo, mas congela as features acima dela.
- Se o rollback é puxado a uma posição, o modelo reconstrói até lá, com ou sem erros.

**Fenkra hoje**
- `Document.rollback` (marcador geral, doc.rs:2710) e `BodyCollector.marker` (por coletor).
- `add_feature` insere na posição do marcador (doc.rs:3224).
- `is_active` cruza `suppressed`, `rollback` e o marcador do coletor (doc.rs:2832).
- O objeto em trabalho é o coletor ativo mais a posição do marcador. Não existe DIWO por feature nem um indicador sublinhado.

**Melhor prática** (B59)
- Mostrar sempre "onde entra o próximo comando" (linha sublinhada e barra de fim de peça).
- Aviso quando se edita uma feature "antiga" que tem filhos posteriores.
- **Teste:** inserir um furo no meio: as features seguintes recalculam e o furo aparece na posição, não no fim.

---

### Stop / Deactivate / Isolate / Update e diagnóstico (CATIA: Update, Deactivate, Stop Update, Extract / SW: Rebuild, What's Wrong, Suppress / Inventor: Design Doctor, Suppress / Fenkra: `Status`, `suppressed`)

**CATIA** [V]
- **Update.** Automático (Tools → Options → Mechanical Design) ou manual (ícone, Edit → Update, contextual com barra de progresso). A peça que precisa de update mostra um símbolo e a geometria em vermelho.
- **Falha.** A feature que causa a falha leva um símbolo amarelo. A face removida aparece em amarelo com o texto "Removed Face". A falha abre um diálogo com diagnóstico, e a feature falha oferece **Editar** (janela "Feature Definition Error" com a face removida destacada), **Desativar** ou **Excluir**.
- **Stop update.** "Associate stop update" interrompe o update com uma nota; "Deactivate stop update" o desliga (parênteses vermelhos).
- **Exclusão.** Opções "Delete exclusive parents" e "Children". Apagar uma feature usada por um acabamento recalcula o acabamento. Apagar uma instância de padrão apaga o padrão. Desfazer disponível.
- **Reordenar.** Edit → Reorder escolhe "After:". As regras de pais e filhos não são detalhadas.
- **Extract Geometry** (a página não usa o termo "Isolate"): numa falha de draft, o CATIA desativa o draft e cria um nó "Extracted Geometry" com as superfícies, para reconstruir com Thick Surface, Split e Add.
- **Replace.** Substituição de superfície, corpo e suporte de sketch ("Change Sketch Support") e mover sketches de um corpo.

**SolidWorks** [V]
- Rollback × freeze bar (acima). Em "What's Wrong" e dangling references faltou fonte confirmada [C].

**Inventor** [V]
- Avisos no painel quando há conflito. Apagar a única aresta de um filete de uma aresta causa erro.

**Fenkra hoje**
- `Status {Ok, Warning, Error, Inactive}` por feature (lib.rs:32). O cache por assinatura reaproveita o que não mudou (lib.rs:4308).
- `suppressed` equivale a Deactivate. A atualização é sempre automática, fora da thread de interface.
- **Falta.**
  - Modo manual e estado "desatualizado".
  - Diálogo de diagnóstico com Editar, Substituir, Isolar, Desativar, Reordenar (B59).
  - Isolar/Extrair em falha.
  - Grafo de pais e filhos navegável.
  - Stop update.
  - Substituir referência.

**Melhor prática**
- Erro = estado visível na árvore com diagnóstico acionável ("a aresta X sumiu; candidatas: …"). **Nunca apagar em silêncio.**
- Isolar: transformar o resultado de uma feature falha em geometria de referência (`Isolated` já existe) para reconstruir por cima.
- **Casos de teste.**
  - Apagar a aresta que um filete usa → erro com candidatas e a feature continua na árvore (já há `referencia_perdida_nao_apaga_comando`).
  - Reordenar um espelho depois do pad.
  - Suprimir o pai: filhos viram "inativos".

---

### Parâmetros, fórmulas, regras e tabelas de projeto (CATIA: Knowledgeware / SW: Equations, Global Variables, Design Tables / Inventor: Parameters, iLogic / Fenkra: `Parameter`, `expr`)

**CATIA** [V]
- Knowledgeware incorpora conhecimento na modelagem por **fórmulas** (relacionam parâmetros), **regras** (lógica condicional), **verificações** (checks) e **reações** (disparam atualização por evento). As tabelas de projeto aplicam valores de uma planilha (criada dos parâmetros do documento ou de arquivo externo).
- O espaçamento desigual de padrões usa fórmulas em i e j.
- Um tutorial indica que isso está além do Part Design básico: é uma camada sobre ele.

**SolidWorks** [V]
- Variáveis globais no diálogo Equations dirigem cotas e podem variar por configuração. Tabelas de projeto entram no Equations. Valores vinculados (linked values) estão sendo substituídos por variáveis globais.

**Inventor** [V]
- **Tipos.** Parâmetros de modelo (criados ao cotar) e de usuário. Podem ser de texto ou verdadeiro/falso (sem expressões) e ter lista de valores.
- **Integração.** Vínculo com planilha, XML, exportação para propriedades do componente, e uso em regras iLogic.
- **Entre arquivos.** Qualquer arquivo de modelagem pode vincular parâmetros de outro.

**Fenkra hoje**
- `Parameter {name, expr, unit, comment, published}` (doc.rs:1114). `eval_params` resolve em ordem de dependência e detecta ciclos com o caminho "A → B → A" (doc.rs:3180-3220).
- Funções em `expr.rs`: `sin`, `cos`, `atan`, `sqrt`, `abs`, `round`, `floor`, `ceil`, `min`, `max`, `if(c, a, b)`, constantes `pi` e `e`. Trigonometria em graus.
- Campos de feature são strings de expressão. A assinatura do cache inclui só os parâmetros que a feature cita (`vars_sig`).
- Publicação de parâmetros para outras peças (`published`).
- **Falta.**
  - Tabela de projeto e configurações.
  - Parâmetros de texto, booleano, lista, tabela de consulta.
  - Regras, verificações e reações.
  - Supressão por expressão ("suprimir se Furos=0").
  - Padrão com contagem por expressão, que existe, mas sem espaçamento desigual.
  - Vínculo com planilha.

**Melhor prática**
- Camada de **famílias**: uma tabela (CSV/planilha) onde cada linha é uma variante e cada coluna é um parâmetro (inclusive o booleano "suprimir feature X"). Uma coluna de configuração escolhe a linha ativa.
- **Regras e verificações** como lista de condições avaliadas depois do recompute, com mensagem na árvore ("espessura < 2 mm"). Reações ficam para depois.
- Avaliar a supressão por expressão antes do `is_active`.
- **Casos de teste.**
  - Ciclo A→B→A: mensagem com o caminho.
  - Unidade: ângulo × comprimento não se misturam sem aviso.
  - Mudar uma variante: o volume bate com o valor esperado de cada linha.

---

### Materiais e propriedades de massa (CATIA: Apply Material, Measure Inertia / SW: Mass Properties, Override / Inventor: iProperties físicas / Fenkra: `Properties.material`, `MassProps`)

**CATIA** [V]
- Aplica-se material ao PartBody e roda Measure Inertia; a massa aparece também na aba Mass das propriedades da peça. A densidade vem do material aplicado.
- A medição lista volume, área, massa e densidade; o objeto de inércia calcula massa, densidade, centro de gravidade, **matriz de inércia, eixos principais e momentos principais** (CAA).
- Problemas conhecidos de versões antigas: densidade -1 em caso não uniforme, e massa errada em produtos com features desativadas (IBM).

**SolidWorks** [V]
- O "Mass Properties" aplicado a uma cavidade criada por Intersect dá o volume interno. Não obtive fonte sobre "Override Mass Properties" (marco [C]).

**Inventor** [V parcial]
- A pesquisa não retornou as propriedades físicas; as regras iLogic e propriedades de componente existem, mas a parte de massa é [C].

**Fenkra hoje**
- `Properties.material: String` (padrão "Aço 1020", doc.rs:1175), sem tabela de densidade.
- `Shape::mass()` devolve só volume, área e centro (kernel/lib.rs:203).
- A densidade existe só no carimbo do desenho (7,85 g/cm³ por padrão, desenho/lib.rs:506), editada à mão. O campo "massa" do desenho calcula `vol/1000 × densidade/1000` (layout.rs:440).
- Não há inércia, eixos principais, massa por corpo nem material por corpo.

**Melhor prática**
- Tabela de materiais com densidade, módulo e cor, ligada a `Properties.material` e ao carimbo do desenho. Massa por corpo (coletor) e total.
- Estender `MassProps` com matriz de inércia (OCCT `GProp_GProps` entrega isso) e eixos principais.
- Avisar quando o material é indefinido (densidade 0) em vez de mostrar massa 0.
- **Casos de teste.**
  - Cubo 10×10×10 mm de aço 7,85: massa 7,85 g e centro (5,5,5).
  - Inércia de um cilindro contra a fórmula.
  - Peça com corpo suprimido: a massa não conta a feature suprimida.

---

## 3. Requisitos levantados

Prioridade: **P0** quebra o uso · **P1** importante · **P2** desejável · **P3** futuro.

### Funcionais

| ID | Requisito | Pri. | Critério de aceite |
|---|---|---|---|
| RF-SOL-01 | O término de cada lado da extrusão/recorte é independente (cego, até próximo/último/plano/superfície/vértice/corpo) | P0 | Extrusão com lado A "até plano" e lado B "cego 5": volume bate; o `Extent` por lado persiste e reabre |
| RF-SOL-02 | Offset nos términos "até plano" e "até superfície" | P1 | Offset +2 e −2 mudam o comprimento em 2 mm; teste de volume |
| RF-SOL-03 | Início deslocado ("From"): plano, face ou deslocamento | P1 | Extrusão começando em plano a 10 mm: faces inicial e final nas cotas esperadas |
| RF-SOL-04 | Saída (taper) com qualquer término, ou erro explícito (nunca ignorar) | P0 | `taper=3` com "até plano" ou aplica ou dá erro nomeado; nunca resultado sem saída calado |
| RF-SOL-05 | "Até o último" usa a última parede real (não a caixa) | P1 | Corpo em L: Pad "até último" termina na face mais distante do material, não no canto da caixa |
| RF-SOL-06 | Contornos selecionados e comprimento por contorno (multi-pad), com assinatura estável | P1 | Sketch com 3 contornos, alturas 5/10/15; apagar um contorno gera diagnóstico |
| RF-SOL-07 | Parede fina com Thickness1 ≠ Thickness2, fibra neutra e fusão de pontas, em extrusão, revolução, varredura e nervura | P1 | Perfil aberto em U: volume = comprimento × (t1+t2) × altura; varredura com perfil aberto + parede fina funciona |
| RF-SOL-08 | Furo guiado por norma/tamanho/ajuste: diâmetros derivam da tabela; edição manual marca "personalizado" | P0 | Escolher M8 roscado fixa broca 6,8; editar o diâmetro troca o rótulo |
| RF-SOL-09 | Tipos de furo: simples, folga, roscado, cônico; assentos: rebaixo, escareado (ângulo editável), spotface, contrafuro; oblongo | P1 | Um teste por combinação com volume removido conferido (B48) |
| RF-SOL-10 | Furo simétrico/dois lados suportado ou rejeitado com erro (nunca usar só a metade) | P0 | `Symmetric(20)` fura 20 mm centrados ou dá erro; nunca 10 mm |
| RF-SOL-11 | Profundidade do furo até a ponta ou até o ombro | P2 | Furo Ø10 com ponta 118° e profundidade 20 "ao ombro": cilindro de 20 + cone |
| RF-SOL-12 | Posição de furo por vértice/aresta circular associativa (referência v2) | P1 | Mover o vértice move o furo; vértice removido → "falhou" com diagnóstico |
| RF-SOL-13 | Rosca cosmética como atributo da face (designação, comprimento, classe) com aviso de Ø≠nominal; modelada sob demanda | P1 | Tabela M3–M20: aviso quando o Ø difere; aparece na tabela de furos e no desenho |
| RF-SOL-14 | Rosca com deslocamento inicial, chanfro de entrada, passo/perfil próprios e preview (B05) | P1 | Rosca parcial de 10 mm a partir de 5 mm; preview antes de confirmar |
| RF-SOL-15 | Varredura com guia secundária, torção e escala; controle do perfil (manter ângulo/direção) | P2 | Varredura com 2 guias muda a seção entre 10 e 20 mm; mede-se na seção |
| RF-SOL-16 | Loft com guias/espinha, acoplamento visível, tangência e tensão (B12) | P2 | Loft com guia segue a guia (distância máx. < tol.); ponto de acoplamento editável |
| RF-SOL-17 | Loft/varredura avisam quando descartam contornos internos | P1 | Seção com furo gera aviso (ou furo no loft) em vez de descarte silencioso |
| RF-SOL-18 | Nervura com modos "paralelo ao sketch" e "normal ao sketch", espessura por lado, arcos incluídos e extensão do perfil | P1 | Nervura diagonal com arco: volume conferido; arco nunca ignorado em silêncio (B11) |
| RF-SOL-19 | Filete: raios múltiplos num comando, setback, perfil cônico e G2, limites e arestas a manter | P2 | Cubo com 3 arestas num vértice e setback 2: geometria fecha; cada raio persiste |
| RF-SOL-20 | `EdgeSel::CreatedBy(feature)` seleciona só as arestas do dono (por papéis), nunca todas | P0 | Duas extrusões + filete `CreatedBy(1)`: só arestas da extrusão 1 recebem; `ExtrudeDraftFillet` passa nesse teste |
| RF-SOL-21 | Filete variável com pontos de controle (posição 0..1) e orientação visível | P2 | 3 pontos (3,6,3): seção no meio mede 6 (B49) |
| RF-SOL-22 | Chaves v2 para face-face, tritangente/completo, rosca e furo (ADR 0010 Fase 2) | P1 | Mudar a espessura da nervura e o full round acompanha; face perdida → erro |
| RF-SOL-23 | Chanfro com indicação visual de qual distância vai em qual face e chanfro de vértice | P2 | Teste de simetria d1×d2 contra d×ângulo |
| RF-SOL-24 | Draft com direção de extração independente do plano neutro, partição, reflect line, ângulo variável e análise | P2 | Partição no plano médio: duas metades com sinais opostos; análise colore as faces |
| RF-SOL-25 | Casca: espessura por face, degraus de fallback (exato → ignorar faces → aproximar) e casca sem face aberta | P0 | Casca com spline e furo escareado resolve (B06); a lista de faces ignoradas aparece; casca fechada tem volume V − V(interior) |
| RF-SOL-26 | Espessura em face de sólido com "ignorar faces" | P1 | +1 mm em face plana = volume + área (B07) |
| RF-SOL-27 | Dividir mantendo os dois lados (um vira coletor novo) | P1 | V₁ + V₂ = V e os dois coletores aparecem na árvore |
| RF-SOL-28 | Booleanas: Union Trim, Remove Lump, trocar o tipo de booleana, aviso de várias regiões | P1 | Peça com 2 lumps avisa; Remove Lump deixa 1; trocar união por subtração recalcula |
| RF-SOL-29 | Escopo de features/participantes: cada feature mostra e permite escolher os corpos-alvo | P1 | Dois coletores: um corte atinge só o escolhido |
| RF-SOL-30 | Padrão: política "cópia exata" × "reavaliar término por instância" | P0 | Padrão de recorte "até próximo" sobre degraus: com "reavaliar", cada instância termina na sua face |
| RF-SOL-31 | Padrão: suprimir instâncias, espaçamento desigual, contagem com limite, coroa, orientação ao longo de curva | P2 | Suprimir a instância 3 de 6 retira a geometria; contagem > limite dá erro |
| RF-SOL-32 | Padrão de features de acabamento (filete, chanfro, draft) e de casca | P2 | Filete padronizado em 4 furos com a mesma geometria dos furos |
| RF-SOL-33 | Espelho com "novo corpo" e "remover original" | P2 | Meia peça espelhada = peça inteira (V × 2) |
| RF-SOL-34 | Transformação com cópia; afinidade preserva papéis quando possível | P2 | Translação com cópia deixa dois corpos |
| RF-SOL-35 | Eixo como elemento de referência de primeira classe (cilindro, aresta, interseção de planos) | P1 | Eixo de cilindro é publicável e usado em revolução e padrão |
| RF-SOL-36 | `PointRef::Fixed`/`AxisRef::Fixed` substituídos por referência associativa quando vêm de geometria | P1 | Mover a aresta de origem move o eixo derivado |
| RF-SOL-37 | Plano de face vira "falhou" com diagnóstico quando a face some (B13/DR-21) | P1 | Remover a face: o plano marca erro, sem seguir outra face |
| RF-SOL-38 | Objeto em trabalho visível e inserir no meio; edição de feature com filhos posteriores avisa | P0 | Inserir furo no meio: as seguintes recalculam; o indicador mostra a posição (B59) |
| RF-SOL-39 | Diálogo de diagnóstico: Editar (realça a referência), Substituir, Isolar, Desativar, Reordenar | P0 | Apagar uma aresta usada: o diálogo oferece as cinco ações e cada uma funciona |
| RF-SOL-40 | Isolar/extrair o resultado de uma feature falha como geometria de referência | P2 | Draft que falha vira superfícies extraídas utilizáveis |
| RF-SOL-41 | Modo de atualização manual e estado "desatualizado" por feature | P2 | Em manual, editar uma cota marca as features dependentes e só "Atualizar" recalcula |
| RF-SOL-42 | Parâmetros: texto, booleano, lista de valores; supressão de feature por expressão | P1 | Parâmetro `Furos=0` suprime a feature de furos; o recompute respeita |
| RF-SOL-43 | Tabela de projeto/famílias: linhas de variantes e configuração ativa | P1 | CSV com 3 variantes: cada uma recalcula com volume esperado; salvar/abrir preserva |
| RF-SOL-44 | Regras e verificações pós-recompute com mensagem na árvore | P2 | Regra "espessura ≥ 2" viola e mostra aviso na árvore |
| RF-SOL-45 | Tabela de materiais com densidade e massa por corpo e total; ligação com o carimbo do desenho | P0 | Cubo de aço 10 mm: massa 7,85 g; o desenho mostra o mesmo valor |
| RF-SOL-46 | Propriedades de massa completas: centro, matriz de inércia, eixos e momentos principais | P1 | Cilindro: inércia confere com a fórmula analítica a 1e-6 |
| RF-SOL-47 | Revolução valida perfil coplanar e não cruzando o eixo, com mensagem em português | P1 | Perfil cruzando o eixo dá erro nomeado e aponta o contorno |
| RF-SOL-48 | Análise de draft e de curvatura como ferramentas de diagnóstico | P3 | Faces coloridas por sinal do draft na direção escolhida |

### Não funcionais

| ID | Requisito | Pri. | Critério de aceite |
|---|---|---|---|
| RNF-SOL-01 | Nenhuma referência é resolvida para outra entidade em silêncio; sem resolução vira estado "falhou" | P0 | Teste de regressão por comando: mudar o perfil/cota e a referência continua ou o comando falha, nunca muda de alvo |
| RNF-SOL-02 | Nenhum parâmetro de um comando é ignorado em silêncio; combinação inválida dá erro ou aviso | P0 | Auditoria: toda combinação (taper × término, simétrico × furo, perfil aberto × varredura…) tem teste de erro ou de efeito |
| RNF-SOL-03 | Todo comando de sólido tem uma peça no corpus B53 com valor esperado (volume/área/caixa/faces) | P0 | O script de cobertura feature → teste falha se faltar peça |
| RNF-SOL-04 | Mudar e voltar cada cota de uma peça do corpus retorna ao volume original (idempotência) | P0 | Teste automático por peça |
| RNF-SOL-05 | Recompute incremental: mudar uma cota refaz só o ramo afetado | P1 | `cache.reused` ≥ 90% das features numa peça de 50 features ao mudar uma cota final |
| RNF-SOL-06 | Salvar e abrir preserva as features com resultado idêntico, inclusive chaves v2 | P0 | Round-trip: mesmo volume e mesmas faces; arquivos antigos abrem e são promovidos |
| RNF-SOL-07 | Tolerâncias e constantes de "alcance" (`1e4`, `1e5`, `diag×2+10`, 5% da diagonal) derivadas do tamanho do modelo, sem números mágicos espalhados | P1 | Peça de 0,5 mm e de 5 m dão os mesmos resultados relativos |
| RNF-SOL-08 | Mensagens de erro em português, com a feature, o campo e uma ação sugerida | P1 | Revisão por amostragem: nenhum erro bruto do OCCT chega ao usuário sem contexto |
| RNF-SOL-09 | Recompute fora da thread de interface; padrões grandes têm limite e cancelamento | P1 | Padrão 100×100 cancelável, UI responsiva |
| RNF-SOL-10 | Malha fechada e sólido válido após cada feature (checagem opcional de validade) | P1 | `BRepCheck` ok no corpus; falha vira aviso na árvore |
| RNF-SOL-11 | Preview ao vivo dos comandos de sólido sem alterar o modelo | P2 | Arrastar o manipulador não grava histórico até confirmar |
| RNF-SOL-12 | Toda feature nova mantém papéis de face (v2) nas faces geradas | P1 | Teste: filete posterior nas faces da varredura/loft/furo sobrevive à edição |

---

## 4. Itens de backlog propostos

Tamanho: P ≈ até 100 k tokens · M ≈ 100–250 k · G > 250 k. Os IDs `BS-xx` são provisórios para não colidir com os B-existentes.

| ID | Título | O que muda | Pri. | Tam. | Dependências |
|---|---|---|---|---|---|
| BS-01 | Términos por lado + offset + início deslocado | `Extent` por lado (A/B), offset em plano/superfície, "From"; unifica `extent()` e `end_cutter()` | P0 | G | B15 |
| BS-02 | Saída sem perda silenciosa e "até último" real | Taper com qualquer término ou erro; última parede em vez da caixa; papéis de face em todos os términos | P0 | M | BS-01 |
| BS-03 | Auditoria "nada ignorado em silêncio" | Teste por combinação inválida (taper×término, furo simétrico, varredura aberta, nervura com arco…) + correções | P0 | M | B53 |
| BS-04 | `CreatedBy` real e atalho de extrusão com saída e filete | Seleção por dono+papel; `ExtrudeDraftFillet` filetea só as arestas dele | P0 | M | B15 |
| BS-05 | Furo guiado por norma (Hole Wizard do Fenkra) | Norma/tamanho/ajuste → diâmetros; tipos; spotface; oblongo; ponta/ombro; posição associativa | P0 | G | B48, B05 |
| BS-06 | Rosca: atributo da face, deslocamento, chanfro, preview | Rosca cosmética como atributo; parcial com offset; passo/perfil próprios | P1 | M | B05, BS-05 |
| BS-07 | Contornos selecionados e multi-pad | Seleção por assinatura; comprimento por contorno; estado "novo contorno" | P1 | M | BS-01 |
| BS-08 | Parede fina unificada | `Thin {t1, t2, neutral, merge_ends, cap}` em extrusão, revolução, varredura e nervura | P1 | M | — |
| BS-09 | Nervura com dois modos e todas as curvas | Paralelo/normal ao sketch; lados; arcos; extensão; saída; bosses opcionais | P1 | M | B11, BS-08 |
| BS-10 | Filetes avançados | Raios múltiplos, setback, perfil cônico/G2, limites, arestas a manter, variável com pontos (B49) | P2 | G | B49, BS-04 |
| BS-11 | Chaves v2 fase 2 | Face-face, tritangente/completo, rosca, furo; papéis em varredura/loft/até-plano | P1 | G | B15 |
| BS-12 | Draft completo | Direção ≠ neutro, partição, reflect line, ângulo variável, análise de draft | P2 | G | — |
| BS-13 | Casca e espessura robustas | Espessura por face; fallback em degraus (exato → ignorar → aproximar); casca fechada | P0 | G | B06, B07 |
| BS-14 | Multicorpo | Dividir mantendo os dois lados, Union Trim, Remove Lump, trocar a booleana, participantes | P1 | G | B59 |
| BS-15 | Padrões com re-solução | Política "cópia exata" × "reavaliar"; suprimir instâncias; espaçamento desigual; limite; coroa | P0 | G | — |
| BS-16 | Padrões/espelho de acabamento e corpo | Filete/chanfro/draft em padrões; espelho "novo corpo/remover original"; transformação com cópia | P2 | M | BS-15 |
| BS-17 | Padrão por curva e user pattern completo | Orientação ao longo do caminho; âncora; "manter especificações" | P2 | M | BS-15 |
| BS-18 | Loft e varredura com guias | Guias, espinha, acoplamento, tangência/tensão, torção/escala; aviso de contornos descartados | P2 | G | B12 |
| BS-19 | Eixo de primeira classe e referências associativas | Elemento Eixo; fim de `Fixed` quando há geometria de origem; plano de face "falhou" | P1 | M | B13 |
| BS-20 | Objeto em trabalho e inserir no meio | Indicador sublinhado, barra de fim de peça, aviso ao editar com filhos | P0 | M | B59 |
| BS-21 | Diagnóstico de atualização | Diálogo Editar/Substituir/Isolar/Desativar/Reordenar; pais e filhos; Stop update | P0 | G | B59 |
| BS-22 | Isolar/extrair e atualização manual | Extrair resultado falho como referência; modo manual; estado "desatualizado" | P2 | M | BS-21 |
| BS-23 | Parâmetros ampliados | Texto/booleano/lista; supressão por expressão; vínculo a planilha | P1 | M | — |
| BS-24 | Famílias (tabelas de projeto) | Linhas = variantes, colunas = parâmetros/supressão; configuração ativa | P1 | G | BS-23 |
| BS-25 | Regras e verificações | Condições pós-recompute com mensagem na árvore | P2 | M | BS-23 |
| BS-26 | Materiais e massa | Tabela de materiais com densidade; massa por corpo; carimbo do desenho ligado | P0 | M | — |
| BS-27 | Propriedades de massa completas | Inércia, eixos e momentos principais (`GProp`) | P1 | P | BS-26 |
| BS-28 | Validações de revolução e de varredura | Perfil coplanar/sem cruzar o eixo; perfil no início da guia; mensagens em português | P1 | P | — |
| BS-29 | Corpus de Part Design | Uma peça por comando, com volume/área/caixa; "mudar e voltar" cada cota | P0 | G | B53 |
| BS-30 | Ferramentas de análise | Análise de draft e de curvatura | P3 | M | BS-12 |

---

## 5. Fontes

**CATIA V5 (guia oficial de Part Design copiado em staff.city.ac.uk)**
- https://www.staff.city.ac.uk/~ra600/ME2105/Catia%20course/CATIA%20Tutorials/prtug_C2/prtugbt0500.htm (índice de features baseadas em sketch)
- …/prtugbt0501.htm (Pad), …/prtugbt0513.htm (Up to Surface), …/prtugbt0527.htm (Pad/Pocket de superfície), …/prtugbt0514.htm (direção não normal), …/prtugbt0524.htm (Multi-Pad), …/prtugbt0520.htm (Drafted Filleted Pad), …/prtugbt0526.htm (parede fina), …/prtugbt0504.htm (Shaft/Groove), …/prtugbt0503.htm (Hole), …/prtugbt0522.htm (furo roscado), …/prtugbt0507.htm (Rib), …/prtugbt0506.htm (Stiffener), …/prtugbt0518.htm (Loft)
- …/prtugbt0600.htm (índice de dress-up), …/prtugbt0606.htm (Edge Fillet), …/prtugbt0609.htm (raio variável), …/prtugbt0610.htm (Tritangent), …/prtugbt0602.htm (Chamfer), …/prtugbt0612.htm (Draft básico), …/prtugbt0603.htm (partição), …/prtugbt0613.htm (reflect lines), …/prtugbt0618.htm (ângulo variável), …/prtugbt0604.htm (Shell), …/prtugbt0605.htm (Thickness)
- …/prtugbt0700.htm (índice de transformações), …/prtugbt0701.htm, …/prtugbt0702.htm, …/prtugbt0703.htm (padrões), …/prtugbt0704.htm (Mirror), …/prtugbt0707.htm (Scaling), …/prtugbt0713.htm (Translation)
- …/prtugbt1000.htm, …/prtugbt1001.htm, …/prtugbt1002.htm, …/prtugbt1006.htm, …/prtugbt1008.htm (corpos e booleanas), …/prtugbt1200.htm (baseadas em superfície)
- …/prtugbt0900.htm, …/prtugbt0904.htm, …/prtugbt0907.htm, …/prtugbt0911.htm (modificação, reordenar, excluir, atualizar), …/prtugbt0906.htm (Scan/Define in Work Object), …/prtugbt0913.htm e …/prtugbt1500.htm (substituir), …/prtugbt0800.htm (propriedades), …/prtugbt1400.htm (ferramentas), …/prtugbt1600.htm (elementos de referência), …/prtugat0500.htm (extrair geometria)
- https://www.staff.city.ac.uk/~ra600/ME2105/Catia%20course/CATIA%20Tutorials/kwrug_C2/kwruggl.htm e https://public.dhe.ibm.com/software/plm/uk/catiav5_knowledgeware.pdf (Knowledgeware)
- https://www.goengineer.com/training/catia-v5-knowledge-based-engineering
- https://www.ibm.com/support/pages/node/5228157, https://www.ibm.com/support/pages/apar/HE04170 (Define In Work Object e booleanas)
- https://workbench.grabcad.com/tutorials/mass-property-option-not-showing-in-catia-v5-r19 e https://www.ibm.com/support/pages/apar/HD75121 (Measure Inertia)
- https://pycatia.readthedocs.io/en/0.93/api/pycatia/space_analyses_interfaces/inertia.html (inércia)
- https://pycatia.readthedocs.io/en/latest/api/pycatia/mec_mod_interfaces/ordered_geometrical_sets.html
- https://grabcad.com/questions/problem-with-up-to-next-extrusion-in-catia

**SolidWorks** (conteúdo por resumos de busca e blogs; `help.solidworks.com` não renderizou no WebFetch)
- https://help.solidworks.com/2022/English/SolidWorks/Sldworks/c_Hole_Wizard_Overview.htm
- https://help.solidworks.com/2019/English/SolidWorks/sldworks/HIDD_DVE_HW_HOLE_SPEC.htm
- https://help.solidworks.com/2020/English/WhatsNew/t_define_hole_depth.htm
- https://help.solidworks.com/2019/english/SolidWorks/sldworks/r_advanced_hole_type_pm.htm
- https://help.solidworks.com/2019/english/SolidWorks/sldworks/c_Fillets_Folder.htm
- https://help.solidworks.com/2019/English/SolidWorks/Sldworks/c_FilletXpert_Folder.htm
- https://help.solidworks.com/2012/English/SolidWorks/Sldworks/HIDD_DVE_DRAFT.htm
- https://help.solidworks.com/2012/English/solidworks/Sldworks/Neutral_Plane_Draft.htm
- https://help.solidworks.com/2017/english/SolidWorks/Sldworks/t_draftxpert.htm
- https://help.solidworks.com/2012/English/solidworks/Sldworks/HIDD_DVE_FEAT_COMBINE_BODIES.htm
- https://help.solidworks.com/2022/English/SolidWorks/sldworks/c_global_variables.htm
- https://help.solidworks.com/2014/English/SOLIDWORKS/Sldworks/c_equations_and_design_tables.htm
- https://help.solidworks.com/2023/english/SolidWorks/Sldworks/r_curve_driven_comp_pattern_pm.htm e https://help.solidworks.com/2012/english/solidworks/sldworks/hidd_pattern_by_curve.htm
- https://hawkridgesys.com/blog/deep-dive-solidworks-hole-wizard
- https://hawkridgesys.com/blog/solidworks-draft-features-you-need-to-know
- https://hawkridgesys.com/blog/combine-intersect-multibody-commands-in-solidworks
- https://hawkridgesys.com/blog/solidworks-up-to-surface-end-condition
- https://hawkridgesys.com/blog/solidworks-global-variables-and-equations
- https://www.engineersrule.com/advanced-breakdown-solidworks-boss-extrude-tool/
- https://www.javelin-tech.com/blog/2017/11/solidworks-rollback-bar/ e https://www.javelin-tech.com/blog/?p=30685 (freeze bar)
- https://www.javelin-tech.com/blog/2026/06/how-to-use-patterns-in-solidworks/ (citado em resumo de busca; o WebFetch devolveu 403)
- https://www.javelin-tech.com/blog/2023/05/how-to-use-fillet-tools-in-solidworks/ (idem)
- https://www.cati.com/?p=133460 (draft através de linhas de partição, 2022)
- https://www.cati.com/?p=118948
- https://solidprofessor.com/blog/how-to-use-the-advanced-hole-tool-in-solidworks/

**Inventor** (leitura direta)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-B0CFA8E7-D3F5-48A9-BE6F-C8DF668301BD.htm (Extrude)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-590A8760-FFBB-41C8-8169-6B7002207884.htm (Fillet)
- https://help.autodesk.com/cloudhelp/2025/ENU/Inventor-Help/files/GUID-94EB6947-0347-4888-870D-4BA22DB144D8.htm (Hole)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-C0848D58-12CB-4605-8165-0836232927F0.htm (Thread)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm (Sketched features)
- https://help.autodesk.com/cloudhelp/2025/ENU/Inventor-Help/files/GUID-D9BDDBFF-CCC3-46B9-921E-E30F3849D7DE.htm (Shell)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-38133835-2EDB-414E-ACB1-2487A96900F7.htm (Draft)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-BFE147D7-49FE-4DA8-9AC2-1A5ABC4F0C8D.htm (Rib)
- https://help.autodesk.com/cloudhelp/2024/ENU/Inventor-Help/files/GUID-9B3D1B72-340A-4F3E-9AD1-80E6DDD91ED2.htm (Patterns e Mirror)
- https://help.autodesk.com/cloudhelp/2022/ENU/Inventor-Help/files/GUID-349C84EE-2D24-4DFC-A239-AFD55E5441AD.htm (Combine/Split)
- https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-72CA8251-F892-4DA1-9A5E-88FE3CB1257E.htm
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-401BABD2-8D5B-4300-BA8D-70037445044F.htm (Revolve)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-FCC0126F-50ED-449B-A0EF-1DED0287170B.htm e https://help.autodesk.com/cloudhelp/2015/ENU/InventorLT-Help/files/GUID-EACB5058-46B3-4D5B-A76A-79663AFC73AD.htm (Sweep)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-595F9D8E-9EEA-4446-91AB-830E5BAE548E.htm e https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-8E5C63EA-0540-4EBE-9B17-B8159C7FE40C.htm (Parâmetros)

**Outros**
- https://cad.onshape.com/help/Content/PartStudio/extrude.htm e https://cad.onshape.com/help/Content/extrude.htm (Onshape Extrude)
- https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-REF-EXTRUDE.htm e https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-CREATE-SOLID-FROM-SKETCH.htm (Fusion)
- https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-REF-SHELL.htm (Fusion Shell)
- https://www.ptc.com/en/support/article/CS25675 e https://support.ptc.com/help/wnc/r12.0.0.0/en/Windchill_Help_Center/ProEWCIntegAHFamTableStruct.html (Creo: tabelas de padrão e família; pouca cobertura)

**Código e documentos do Fenkra lidos**
- `crates/fenkra-core/src/doc.rs`, `crates/fenkra-core/src/expr.rs`, `crates/fenkra-solidos/src/lib.rs`, `crates/fenkra-solidos/src/hardware.rs`, `crates/fenkra-app/src/app.rs`, `crates/fenkra-kernel/src/lib.rs`
- `docs/arquitetura/adr/0010-referencias-v2.md`, `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`, `docs/benchmark/catalogo.md`, `docs/backlog/backlog.md`

---

## Notas da entrega

- **Resultado.** O relatório inteiro está acima. Não criei nem alterei arquivos, e não executei build nem testes: foi só leitura e pesquisa.
- **Limitações das fontes.**
  - O WebFetch devolveu só a navegação de `help.solidworks.com`, e 403 em duas páginas do Javelin. SolidWorks, portanto, está menos aprofundado que CATIA e Inventor. Os itens SW sem URL específica estão marcados [C].
  - O guia de CATIA é de uma versão antiga, e Creo, NX, Solid Edge e SolidWorks têm cobertura rasa.
  - Não verifiquei o comportamento do kernel OCCT em casos de borda. Os "bugs prováveis" vêm da leitura do código, não de execução, e valem confirmar com testes antes de priorizar.
- **Bugs prováveis levantados** (todos a confirmar por teste):
  1. Saída (taper) ignorada com término "até…" (lib.rs:2738-2768).
  2. `EdgeSel::CreatedBy` aplica a todas as arestas (lib.rs:4183-4185). O atalho `ExtrudeDraftFillet` (app.rs:1720) depende disso.
  3. Furo com término simétrico ou de dois lados usa só o primeiro valor (lib.rs:2882).
  4. Nervura ignora arcos e splines do sketch (lib.rs:3966).
  5. Varredura com perfil aberto + parede fina dá erro (lib.rs:2938).
  6. "Até o último" usa a caixa envolvente (lib.rs:2537).
  7. Loft descarta contornos internos em silêncio (lib.rs:2962).
- **Riscos.** Os tamanhos P/M/G do backlog são estimativas grosseiras. As dependências com B15, B53 e B59 são as mais críticas: sem as chaves v2 completas e o corpus, as melhorias de sólido ficam sem rede de segurança.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Relatório completo de Part Design com 5 maiores lacunas — (1) términos e início da extrusão incompletos, com saída ignorada em silêncio e "até último" pela caixa; (2) furo sem assistente por norma e rosca sem atributo/preview; (3) filetes sem setback, raios múltiplos e `CreatedBy` que aplica a todas as arestas; (4) padrões que copiam a ferramenta pronta, sem re-solução do término, e multicorpo sem Union Trim, Remove Lump ou escopo; (5) sem diagnóstico de atualização, tabelas de projeto, regras nem materiais/massa/inércia; fontes de SolidWorks parciais.
