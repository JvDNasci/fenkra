# Estado atual dos comandos: Sólidos, Montagem, Desenho e PMI (B565 / B569)

Levantamento de 21/11/2026, só de leitura do código (develop, após a alpha.41). Serve de base ao contrato
único de feature (B565) e à matriz de paridade (B569). Os campos de seleção (widgets, Tab, cadeia, junção
na hora, inverter) estão em [selecao-auditoria.md](../../benchmark/detalhe/selecao-auditoria.md) e **não
são repetidos aqui**; só são citados quando mudam a conclusão.

Siglas de arquivo (todos em `crates/`): `doc.rs` = `fenkra-core/src/doc.rs`; `S/lib.rs` =
`fenkra-solidos/src/lib.rs`; `feat_ui` = `fenkra-app/src/feat_ui.rs`; `editor` = `fenkra-app/src/editor.rs`;
`app.rs` = `fenkra-app/src/app.rs`; `K/features.inc` = `fenkra-kernel/cpp/features.inc` (idem
`kernel.cpp`, `hybrid.inc`, `drawing.inc`); `M/` = `fenkra-montagem/src/`; `D/` = `fenkra-desenho/src/`.
"Inferido" marca o que li do fluxo, mas não confirmei rodando.

---------------------------------------------------------------------------------------------------

## 1. Ciclo de vida comum a todo comando de Sólidos

| Etapa | Onde | Fato |
|---|---|---|
| Dado | `FeatureKind` (`doc.rs:1843`) | Uma variante por comando; `Feature` (`doc.rs:1814`) traz `target` (coletor), `suppressed`, `suppress_if` (expressão), `ancoras` (B516). |
| Painel | `editor.rs:1539 feature_ui` → `feat_ui.rs:270 panel` | `feature_ui` trata Extrusão, Revolução, Furo, Filete/Chanfro; o resto cai em `feat_ui::panel` (retorna `true` quando já desenhou; aí só acrescenta `target_combo`). Não há trait: é um `match` gigante nos dois arquivos. |
| Sessão | `app.rs:3226 start_session` | Guarda `snapshot` do documento, `is_new`, `corpos_antes`. Concluir = `app.rs:3289`; cancelar = `3322` (volta ao snapshot; comando novo some). |
| Cálculo | `app.rs:120-160 Worker` | `recompute_cached` em thread própria, pedido novo cancela o anterior (bandeira lida pelo kernel). |
| Motor | `S/lib.rs:4780 Ctx::run` → um braço por `FeatureKind` | Cada braço devolve `Ok(None)`, `Ok(Some(aviso))` ou `Err(msg)`; o status vira `Ok / Warning / Error / Inactive`. |
| Prévia | `previa.rs:83 da_sessao` + `Cache::leve` | Ver 1.2. |
| Manipuladores | `handles.rs:604 overlay` + `handles/{solidos,aresta,padrao,furo,limites,vocab}.rs` + `gizmo.rs`, `loft_3d.rs`, `varredura_ui.rs` | Vocabulário B228 (seta, anel, plano, esfera, raio, losango). |

### 1.1 Falha e validação (igual para todos)

- Não há função de "validar antes do OK". A validação é o próprio cálculo: o braço faz `ev(...)?` e
  `Err(String)` (ex.: `lib.rs:4832`, `5271`). O resultado aparece como status no bloco comum do painel
  (`editor.rs:1132 bloco_comum`, recebe `Status`) e, desde o B419, com causa estrutural
  (`lib.rs:839 tipos_de_erro` → `erro.rs:14 ErrorKind`: `ReferenciaPerdida`, `PaiFalhou`, `GeometriaPerdida`,
  `Propria`).
- **O painel não fecha e o Concluir não é bloqueado**: `editor.rs:1208 aplicar_acao` chama
  `conclude_session` (`app.rs:3289`) sem consultar o status. O comando com erro fica na árvore.
  Único filtro: `selecao.rs:157 descartar_ao_sair` cancela um comando **novo** com erro quando a pessoa
  troca de comando sem concluir. Contraste: Montagem bloqueia (`asm_cmd.rs:255 aceitar` devolve toast
  "Sem solução" e não grava) e Desenho bloqueia a ferramenta antes do clique (`dwg_ui.rs:483 block_reason`).
- Campo ainda "a escolher" não é erro: o comando fica `Inactive` (B459, `lib.rs:7258`; B477 para face em
  (0,0,0), `lib.rs:7262`; `ARESTA_A_ESCOLHER`). Há também avisos textuais "escolha na viewport as faces…" nos
  braços de Casca, Saída, Espessura e Edição direta (`lib.rs:5255-5258`).
- Sólido inválido com status ok: `validade.rs:17 invalido` (B413) acrescenta aviso depois do comando. Com um
  comando aberto (`cache.leve`) a conferência é adiada e feita ao reaproveitar (`lib.rs:7253`).
- **Sem reparo automático.** O que existe são mensagens sugestivas: filete diz o raio máximo que cabe
  (`lib.rs:6910`, `arestas::max_that_fits`), extrusão de corte diz se "inverter" resolve
  (`lib.rs:4868`, `sentido_corte::aviso`), direção no plano do sketch (K-10, `lib.rs:4847`), perfil de
  revolução cruzando o eixo (`lib.rs:288 check_revolve_profile`).
- Os comandos seguintes **seguem sendo calculados** sobre o corpo que sobrou; não vi parada em cascata
  (inferido de `lib.rs:7086-7330`, sem `break` no `Err`). `PaiFalhou` é só rótulo posterior.

### 1.2 Prévia (como funciona de fato)

1. **Fantasma do resultado completo** (padrão): o comando aberto já está no documento e é calculado pelo
   mesmo `recompute`; `previa.rs:1-6` e `ALFA = 0.34` só mudam como o que o comando criou aparece (translúcido,
   cor do tema; `previa.rs:54 retira` pinta o volume retirado em outra cor). Se o cálculo demora, a tela
   segue com o último resultado (`previa.rs:312 reter_ultima`).
2. **Prévia leve** (`previa_leve.rs:48 cobre`, ADR 0075): o corpo passa intacto e só se desenham caixas ou
   linhas. Cobre **Filete, Chanfro, Casca, Padrão, Espelho, Varredura, Multi-seções** e, quando são o último
   comando, chapa e estrutural (`previa_leve.rs:69 so_no_fim`). **Não cobre**: Extrusão, Revolução, Furo,
   Ângulo de saída, Espessura, Edição direta, Combinar, comandos de superfície→sólido, Plástico, Forma, malha.
   Rosca modelada e furo roscado têm leve própria (`rosca_furo::modelada`, só a posição das linhas).
3. Ao concluir, `request_recompute` roda com `leve = None` e o resultado final nunca vem da prévia leve.

### 1.3 Cache e dependentes (igual para todos)

- **Assinatura por comando** (`lib.rs:7110-7190`): `feature_sig` (JSON do comando sem nome, `hidden`,
  aparência; `lib.rs:1151`) + `vars_sig` (só os parâmetros que o texto cita) + assinatura do corpo de
  destino + assinatura dos comandos, coletores e referências citados (`walk`). Igual ao cache = reaproveita
  corpos, status, ferramentas, roscas, pré-vias (`lib.rs:7195-7232`). Recompute é, portanto, **incremental
  por comando**, não só "a partir do primeiro que mudou": um comando do meio que não leu nada alterado é
  reaproveitado.
- **Invalidação larga**: se o JSON do comando contém `"Face"` ou `"Vertex"` (chaves v2 de Casca, Saída,
  Espessura, Edição direta, Rosca, Furo por pontos, Filete face a face…), a assinatura passa a incluir
  **todos** os corpos (`lib.rs:7150-7152, 7174`). Mudar qualquer corpo refaz esses comandos. Ponto de melhora.
- Malhas por versão do corpo ficam em `Cache::meshes` com teto de 512 MB (`lib.rs:1135`); outra qualidade
  só re-tessela (`lib.rs:7009`).
- **Dependentes**: `Document::dependents` (`doc.rs:5463`), `walk` de cada tipo de referência; `app.rs:3962
  ask_delete` abre diálogo de substituição. `hide_consumed` (`doc.rs:5426`) esconde sketches/curvas
  consumidos ao concluir.
- **Nomeação topológica**: faces e arestas viram `FaceKey`/`EdgeKey` v2 (ADR 0010) promovidas no primeiro
  cálculo (`Promoted`, `lib.rs:7280`); `lib.rs:3440 resolve_faces` e `resolve_edges` reencontram; regras
  gravadas (B95, `selecao_regras.rs`: tangentes, cadeia, concêntricas) crescem a seleção a cada recompute.
- Papel das faces: `apply_roles` (`lib.rs:4604`) carrega dono + papel por face (tampa, lateral) só em
  Extrusão, Revolução, Furo, Varredura, Multi-seções; os demais comandos entram como `None` (papel -1).

---------------------------------------------------------------------------------------------------

## 2. Sólidos

Colunas: **Entradas** (tipo, quantas, referência guardada) · **Painel / faltas** · **Direção, término** ·
**Prévia, 3D** · **Motor** (Rust → kernel · ideia). Linhas de validação/saída/cache só quando fogem de
1.1 a 1.3. Detalhe de widget de seleção: ver auditoria.

### 2.1 Comandos de perfil (sketch) e primitiva

| Comando | Entradas | Painel / faltas | Direção, término | Prévia, 3D | Motor |
|---|---|---|---|---|---|
| **Extrusão** `doc.rs:4066` | 1 sketch (`Id`, pasta de perfis), `dir` opcional (`AxisRef`), alvo (coletor) | `editor.rs:1580`: Perfil, Operação (add/cut/interseção), Extensão, "inverter sentido", direção por linha/aresta/eixo; `feat_ui.rs:1287 extrude_extras`: Início deslocado, deslocamento do término, término do lado 2, ângulo de saída, parede fina (lado 0/1/2). **Falta** vs irmãos: espessura diferente por lado (a parede fina é uma só), mais de um perfil/curva de sketches diferentes, resultado em corpo novo sem trocar o coletor | 8 términos: cego, plano médio, dois sentidos, passante, até o próximo, até o último, até plano, até superfície (`doc.rs:4049`); `reverse` + `dir`. Saída só com término cego (erro `lib.rs:4836`) | Fantasma completo (sem leve). Alças: seta de distância, saída (arco), lado 2, plano médio, seta "até a face" (`handles.rs:70`, `handles/solidos.rs:66,253`) | `lib.rs:4828` → `tool_extrusao` (`termino.rs:191`) → `Shape::extrude_roles` (`kernel.cpp:496`, `BRepPrimAPI_MakePrism`), `extrude_taper` (`K/features.inc:823`), `extrude_until` (`kernel.cpp:861`) · "varre a face pela direção e faz booleana". Aviso se o corte não removeu volume. K-10: direção no plano do sketch vira erro |
| **Revolução** `doc.rs:4111` | 1 sketch + eixo (`AxisRef` por **combo próprio**, não `axis_slot`) | `editor.rs:1601` + `feat_ui.rs:1320`: Perfil, Operação, Eixo, Ângulo, simétrico, 2º ângulo, parede fina. **Falta**: inverter, direção por clique no 3D, término "até" | Só ângulo (0–360, soma dos dois ≤ 360, `lib.rs:4884`) | Fantasma completo; arco no 3D (`handles.rs:457 revolve_handle`) | `lib.rs:4873` → `face.revolve_roles` (`kernel.cpp:511`, `BRepPrimAPI_MakeRevol`) · "gira a face em torno do eixo". Erro se o perfil cruza o eixo |
| **Furo** `doc.rs:4136` | sketch (pontos = centros) **ou** `points: Vec<PointRef>` 3D; `dir` opcional; norma/tamanho | `editor.rs:1769 hole_ui`: Tipo (simples, rebaixado, escareado, contrafurado), Rosca (cosmética/modelada, esquerda), Medida (norma + "Ajuste fino"), Fundo e corpo (ponta em V, conicidade, deslocamento), Centros, Direção, "inverter sentido". Só corta (sempre `BoolOp::Cut`). Furo de montagem abre desabilitado (`editor.rs:1638`) | `Extent` de profundidade; `reverse`; sem direção o furo é normal à superfície no ponto | Fantasma completo; **furo-guia** que segue o cursor (`handles/furo.rs`). **Sem seta de profundidade/diâmetro** (não há braço `Hole` em `handles.rs:65-190`) | `lib.rs:4895-4990` → `Shape::hole_tool_ext` (`kernel.cpp:586`) + `apply_roles(Cut)` · "cilindro/cone com a forma da cabeça, subtraído". Centro fora da peça desce por `ray_hit` até a superfície (furo inteligente). Rosca modelada: `rosca_furo::ferramenta_no_degrau` + `corta_rosca` |
| **Varredura** `doc.rs:3783` (nervura/rasgo) | 1 sketch (perfil) + `spine: CurveRef` (1 guia) | `feat_ui.rs:283`: Tipo (nervura/rasgo, sem interseção), Perfil e guia, Orientação (acompanha, fixa, Frenet), parede fina. **Falta**: inverter, torção, guias secundárias, perfil aberto (descartado com aviso, `lib.rs:5165`), ponto de partida | Sem término (guia inteira) | **Leve** (`previa_leve.rs:48`) = caixa do perfil e do caminho; `varredura_ui::desenhar` (`previa.rs:500`) | `lib.rs:5144` → `varrer` (`lib.rs:3425`) → `Shape::sweep` (`K/features.inc:242`, `BRepOffsetAPI_MakePipeShell`) · "perfil acompanha a guia (trechos retos com junta de esquadria)". Aviso B301 se perfil não normal à guia (>3°) |
| **Multi-seções (Loft)** `doc.rs:3793` | lista de sketches (≥2), ordenados à mão ▲▼ | `feat_ui.rs:298`: Tipo (add/remover), Perfis em ordem, "faces retas", "fechada", torção por seção, condição de ponta Livre/Normal com intensidade 0–3 (`loft_3d::painel`). **Falta**: guias, mais de um contorno por seção (só o 1º entra, com aviso `lib.rs:5188`), inverter | Sem término | **Leve** (caixa) + linha de conexão arrastável (`loft_3d.rs`) e setas de intensidade (`handles.rs:634`) | `lib.rs:5182` → `Shape::loft` (`K/features.inc:442`, `ThruSections`) ou `loft_normal` (`lib.rs:3558`, B513, derivada presa nas pontas) |
| **Nervura (Reforço)** `Stiffener` `doc.rs:3988` | 1 sketch (linhas) | `feat_ui.rs:503`: Linhas, Espessura (campo plástico), "até encontrar o corpo", Profundidade, "para o lado da normal", Lado da parede (2 lados/1/outro). **Falta**: operação (só soma), ângulo de saída, direção, filete na base | `reverse`, `to_body`, `side` | Fantasma completo; seta de profundidade (`handles.rs:135`) | `lib.rs:6583` → `Shape::thick_segments` + `extrude` / `extrude_to_next` · "cada linha vira parede de espessura t, extrudada" |
| **Primitiva** `doc.rs:3873` | 1 plano (`PlaneRef`), 4 medidas | `feat_ui.rs:338`: forma (7), operação (add/remover), plano (`plane_combo`), medidas. **Falta**: interseção | Altura/raios; sem inverter | Fantasma; setas de altura e base (`handles/solidos.rs:84`) | `lib.rs:5243` → `Shape::primitive` (`K/features.inc:452`) |
| **Plástico** (boss, snap, grelha, lip) `recurso_plastico.rs:18` | sketch (centros, raízes ou contorno) | `plastico_rec_ui.rs:67`: seções Suporte, Parâmetros, Furo e inserto, Nervuras, Avançado; os campos nascem da fórmula da regra plástica (B198). Inverter = caixa "para o outro lado do plano" (nome varia por recurso; snap tem também `lado`) | `reverse` por recurso | Fantasma completo; uma seta (`plastico_rec_ui::seta`, `handles.rs:134`) | `plastico_rec.rs:357 plastico_rec` → `boss_solido`, `snap_solido`, barras da grelha, `lip` · composição de prismas, cones e filete por tipo |

### 2.2 Comandos de aresta e face

| Comando | Entradas | Painel / faltas | Direção, término | Prévia, 3D | Motor |
|---|---|---|---|---|---|
| **Filete** / **Chanfro** `EdgeOp` `doc.rs:4323` | arestas (`EdgeSel`: todas, criadas por comando, lista por ponto médio + `EdgeKey`), faces para face-face/completo | `editor.rs:1644` + `feat_ui.rs:1369 edge_mode_ui`. Filete: Constante, Variável (pontos de controle %/raio, transição suave), Face-face, Completo, Assimétrico. Chanfro: Distância, 2 distâncias, Distância+ângulo. Comum: valor, "Todas/Selecionadas", regra gravada (B95), propagação Tangente/Mínima, inverter por aresta, trecho parcial. **Chanfro não tem** variável, face-face nem completo; **filete não tem** distância+ângulo | "inverter por aresta" só com 2 medidas (`EdgeExtra.flips`) | **Leve em todos os modos** (linhas de tangência a 3 mm, `previa.rs` teste `filete_leve_…`); alças: valor, pontos de controle (B49), trecho parcial (B289), seta que inverte a face (`handles/aresta.rs`) | `lib.rs:6790-6925` → `edges_tagged` / `edges_mode_tagged` / `fillet_variable_tagged` / `face_face_fillet_tagged` / `full_round_tagged` (`K/features.inc:995-1130`, `BRepFilletAPI_MakeFillet` e `MakeChamfer`) · "arredonda/chanfra a aresta por rolamento de esfera". Falha tenta valores menores e explica (`explain_edge_failure`) |
| **Casca** `doc.rs:3891` | faces abertas (pontos + `FaceKey`), regras B95; espessura por face | `feat_ui.rs:358`: Espessura (campo plástico), "parede para fora", "esvaziar sem abrir faces", faces (`faces_ui`, **lista antiga, sem Tab**), regra, espessura por face, "corte de seção na prévia" | "para fora" (`outward`) é o inverter | **Leve** (caixa interna) + seta de espessura (`handles.rs:102`) | `lib.rs:5267` → `shell_tagged` / `shell_faces_tagged` (`K/features.inc:504-613`, `MakeThickSolidByJoin`, plano B por booleana). Erros: espessura ≥ menor dimensão; "não esvaziou o corpo" |
| **Ângulo de saída** `doc.rs:3912` | faces + plano neutro (`plane_combo`) | `feat_ui.rs:395`: Ângulo (campo plástico), plano neutro, "inverter a direção de extração", faces, regra | `reverse` | Fantasma completo; arco de saída (`handles/solidos.rs:263,440`) | `lib.rs:5299` → `draft_tagged` (`K/features.inc:659`, `BRepOffsetAPI_DraftAngle`) |
| **Espessura** `doc.rs:3928` | faces + regras | `feat_ui.rs:409`: Distância, rádio Acrescentar/Remover (o sinal negativo também remove), faces, regra | `remove` (rádio, diferente da Casca e do Ângulo, que usam caixa) | Fantasma; seta só em face **plana** (`handles.rs:122`) | `lib.rs:5311` → `faces_offset_tool` (`K/features.inc:690`) + `apply(Add/Cut)` · "prisma de cada face deslocada" |
| **Edição direta** `edicao_direta.rs:36` | faces + regras; 4 operações | `edicao_direta_ui.rs`: Mover (X, Y, Z), Apagar, Deslocar, Raio do filete; só os campos da operação | sinal do valor | Fantasma; uma seta por eixo, deslocar e raio (`handles/solidos.rs:166`) | `S/edicao_direta.rs:24` · apagar = remove faces e estende vizinhas; mover = relevo transladado; deslocar = offset só das faces; todas fecham em `valida` (`:63`, volume e validade) |
| **Rosca** `doc.rs:4014` | 1 face cilíndrica (ponto + `face_key`), norma da tabela | `feat_ui.rs:519`: Norma e tamanho, término (cego/passante/até plano/superfície), Cosmética/Modelada, Início; face por texto + "Usar a face selecionada". Sem regras B95 | só término | Hélice e anéis (`previa.rs:221`), aviso se Ø ≠ nominal; seta de comprimento (`handles.rs:130`) | `lib.rs:6620` → `hardware::thread` + `cylinder_at` + `rosca_furo::ferramenta_no_degrau` + `corta_rosca` · "sulco helicoidal subtraído"; cosmética não muda o corpo |

### 2.3 Transformação e repetição

| Comando | Entradas | Painel / faltas | Direção | Prévia, 3D | Motor |
|---|---|---|---|---|---|
| **Padrão** `doc.rs:3965` | `sources: Vec<Id>` (comandos anteriores, vazio = corpo inteiro), eixo(s)/pontos | `feat_ui.rs:423`: Tipo (linear 1–2 direções, circular, por pontos), Definição, fontes (caixas de marcar), instâncias puladas (B91). **Falta**: inverter, padrão por face/curva, "vincular" a outro padrão | `axis_slot`; sem inverter | **Leve** (caixa por instância); pontos clicáveis, setas de espaçamento, arco e quantidade (`handles/padrao.rs`) | `lib.rs:5326` → `instancias` (`previa_leve.rs:452`) + `replicate` (`lib.rs:4667`): **refaz as ferramentas gravadas** (`steps`) dos comandos fonte por transformação rígida e aplica as booleanas; sem fonte, compõe cópias do corpo |
| **Espelho** `doc.rs:3976` | plano (`plane_combo`) + fontes | `feat_ui.rs:491`. **Falta**: plano por clique, inverter/lado, manter original sim/não | – | **Leve**; plano translúcido com rótulo (`previa.rs:155`); seta do plano deslocado | `lib.rs:5340` → `replicate` com `Xform::Mirror` |
| **Escala** `doc.rs:3982` | `PointRef` (centro) + fator | `feat_ui.rs:498`. Só uniforme e só o corpo inteiro | – | Fantasma; losango do fator (`handles/padrao.rs ESCALA`) | `lib.rs:6574` → `transformed_tagged(Xform::Scale)` (`K/features.inc:158`) · preserva nomes de faces |
| **Mover corpo** `doc.rs:3068` | `BodyXf` 4 modos: eixo, eixo+ângulo, plano (simetria), plano+3 fatores (afinidade) | `feat_ui.rs:1012`; simetria e afinidade usam `plane_combo`. Sem inverter (distância negativa inverte) | sinal | Fantasma; seta, arco, uma seta por fator (`handles/solidos.rs:106`) | `lib.rs:6541` → `transformed_tagged` (rígido, mantém nomes) ou `affinity` (`hybrid.inc:1236`, `gp_GTrsf`; **perde os nomes**, `fresh_body`) |
| **Combinar** `doc.rs:4004` | 1 coletor ferramenta (combo; "clique" só via seleção) | `feat_ui.rs:562`: operação 3 ícones, corpo ferramenta, "manter visível" | – | Fantasma | `lib.rs:6676` → `apply` com o corpo do outro coletor (`boolean_tagged_src`, `BRepAlgoAPI_Fuse/Cut/Common`). Erro: ferramenta do mesmo coletor ou que "vem depois" |

### 2.4 Superfície → sólido

| Comando | Entradas | Painel / faltas | Direção | Prévia, 3D | Motor |
|---|---|---|---|---|---|
| **Espessar superfície** `ThickSurface` `doc.rs:3092` | 1 `SurfRef` (slot) | `feat_ui.rs:1059`: Um lado / Dois lados / Simétrico, espessura(s), direção (normal, eixo/aresta, normal de plano), op (add/remover/interseção). **Falta**: término "até", ângulo, parede variável | seta; `dir` explícita (B536) | Fantasma; seta única (`handles.rs:110`) | `lib.rs:6694` → `Shape::thicken` / `thicken_dir` (`hybrid.inc:838-931`, `MakeOffset` + `MakeThickSolid` ou prisma) |
| **Fechar superfície** `doc.rs:3134` | 1 `SurfRef` | `feat_ui.rs:1066`: tolerância, op | – | Fantasma | `lib.rs:6705` → `closed_solid` (`hybrid.inc:1132`, `BRepBuilderAPI_Sewing` + `MakeSolid`) |
| **Costurar no corpo** `doc.rs:3142` | 1 `SurfRef` | `feat_ui.rs:1073`: Acrescentar/Remover; "flip" só vira seta/dica de lado (`sup3d::dica_de_lado`) | `flip` (sem botão no painel, ver B562) | Fantasma | `lib.rs:6711`: remover = `split_side`; acrescentar = `compound.volume_cells` (`hybrid.inc:1167`, `BOPAlgo_MakerVolume`) |
| **Dividir corpo** `doc.rs:3085` | 1 `SurfRef` (face estendida se for do corpo) | `feat_ui.rs:1053`; só a dica de lado | `flip` | Fantasma | `lib.rs:6687` → `split_side` (`hybrid.inc:406`, `BRepAlgoAPI_Splitter`) · fica um lado |
| **Preencher por fronteiras** `doc.rs:3150` | N `SurfRef` + pontos de células excluídas | `feat_ui.rs:1087`: lista, células excluídas, "+ excluir no ponto selecionado", op | – | Fantasma | `lib.rs:6747` → `volume_cells(exclude)` · células fechadas pelas superfícies |

### 2.5 Forma livre e malha

| Comando | Entradas | Painel / faltas | Prévia, 3D | Motor, cache |
|---|---|---|---|---|
| **Forma livre (subdivisão)** `doc.rs:1949` | gaiola (nós e faces com número persistente), vínculos | `forma_ui.rs:1259 painel`: Superfície (passos), Gaiola, Congelar, Conversão (continuidade G0/G1/G2, tolerância). Abrir um comando convertido **entra na gaiola** (`app.rs:3250 forma_ui::ao_abrir`), fluxo próprio de edição por clique/arraste | Superfície Catmull-Clark em tempo real; gaiola editável | `lib.rs:6724` → `forma_brep_cache::converter` (`:181`, cache em memória e disco) → `forma_brep.rs` (remendos exatos + região por preenchimento nos nós irregulares). Casco aberto vira **superfície comum** (`tools`, B511); "pausada" suspende o cálculo dos comandos seguintes (`lib.rs:7073`) |
| **Malha** (`MeshImport` `doc.rs:3158`, `MeshOp` `malha.rs:96`) | arquivo ou chave em `Document::malhas`; coletor ferramenta (booleana) | `malha_ui.rs:67`: Reparar (solda, não manifold, normais, furos, partes soltas), Decimar (proporção, desvio), Booleana (op, ferramenta, manter), Malha→sólido (modo, tolerância), Sólido→malha (desvio, ângulo), Conferir | Sem prévia leve nem alças | `corpo_malha.rs:274 operar_malha` → `fenkra-malha`: `reparo.rs:340`, `decimar.rs:248` (quádricas, Garland-Heckbert), `booleana.rs:31` (port da Manifold). Cache por **conteúdo** (`obter(chave)`, `chave_de`); `sempre_roda` (`corpo_malha.rs`) força o cálculo de alguns |

---------------------------------------------------------------------------------------------------

## 3. Montagem

Modelo: `Assembly` (`.fka`) com `nodes`, `mates: Vec<Mate>` (`M/lib.rs:300`), `joints: Vec<Joint>`
(`M/kin.rs:102`), `features` (`AsmFeature`), `RigidGroup`. **Não usa `Session`**: o estado é `AsmState`
(`asm_ui.rs`), com `cmd: Option<Pendente>`, `repeticao`, `inserir`, `sim_open`.

### 3.1 Restrições e juntas (comando B244)

| Item | Fato |
|---|---|
| Entradas | 2 `Elemento` (`proposta.rs:19`): `MateRef { comp, geom (Plane/Axis/Point), pick, viva }` (`M/lib.rs:216`). `viva: releitura::Viva` reencontra a face quando a peça muda; **a referência é o componente + geometria local**, não uma chave de topologia da peça. Cada lado aceita 1 elemento (`picks: [Vec<Pick>; 2]`). |
| Tipos | `Tipo` (`proposta.rs`): Automática, Restrição(`MateKind`: Coincidente, Contato, Distância, Ângulo, Paralelismo, Perpendicularidade, Engrenagem/Polia, Pinhão-cremalheira), Alinhar (posiciona e não guarda), Junta automática, Junta(`JointKind` `kin.rs:40`: Revolução, Prismática, Cilíndrica, Parafuso, Esférica, Planar, Rígida, Ponto em curva, Came). `quick_options` (`asm_ui.rs:99`) filtra por par de geometrias; `par_valido` (`M/lib.rs:380`) recusa pares sem sentido. |
| Painel | `asm_cmd.rs` (B244): tipo, valor (expressão com parâmetros da montagem), ângulo inicial, "Girar 90°", deslocamentos X/Y/Z com fórmula, **Inverter alinhamento / Inverter lado** (botões, só quando mudam algo: `Mate::aceita_alinhamento`, `aceita_lado`, `M/lib.rs:322-336`), Inverter (junta), Limites (`asm_limites`), Animar, Avançado (nome, suprimida). Editar uma restrição já existente usa **outro painel**, direto na árvore (`asm_ui.rs:3215-3260`), **sem prévia nem Concluir**: o valor muda na hora. |
| Direção / limites | `Sentido` (Livre/Mesmo/Oposto), `lado_invertido`, `Coincident.flip`; juntas têm `Limites` por grau de liberdade (impostos como desigualdade no solver). |
| Prévia | **Sempre completa e fiel**: `Comando::resolver` (`proposta.rs:288`) resolve numa **cópia** da montagem e mostra a peça na posição final como fantasma, com setas na normal. Cancelar não tem o que desfazer. |
| Validação / falha | `aceitar` (`asm_cmd.rs:255`) refaz a resolução; sem solução → toast e **não grava**; `quadro_de_conflito` (`asm_cmd.rs:~670`) lista as restrições em conflito, que acendem no 3D (B-conflito). Mesmo componente nos dois lados = erro. |
| Motor | `M/solve.rs:352 solve`: Levenberg–Marquardt sobre 6 GDL por corpo livre, resíduo por restrição, **aglomerados** (grupos que se influenciam resolvem à parte, ADR 0046); pares já travados viram um corpo só. `kin.rs` para juntas, `analyze` para GDL. |
| Saídas / cache | Posição (`Placement`) de cada componente; `SolveReport` (resíduo, GDL, falhas, `aglomerados`). **O solver roda a montagem inteira a cada `solve`** (`asm_ui.rs:675`); o cache existente é de carga de peças (`load.rs`, `lod.rs`) e links, não de resultado de solver. `Gatilho::Posicao` evita reler mestres (`asm_ui.rs:689`). Dependentes: restrições citam `comp: Id`; apagar o componente apaga as restrições. |

### 3.2 Inserir, repetir, explodir, lista de peças

| Comando | Entradas | Painel e prévia | Motor |
|---|---|---|---|
| **Inserir** (B262) `asm_inserir.rs`, `M/insercao.rs` | peça ou subconjunto (miniatura, recentes, diálogo) | Peça "na mão" segue o cursor como fantasma desenhado pelo `Painter` (sem reconstruir a cena); cada clique coloca uma cópia e é um passo de desfazer; Esc encerra; a 1ª peça fica fixa na origem | `insercao::colocar` cria o componente; sem validação além do caminho do arquivo |
| **Repetição** (B263) `asm_repeticao.rs`, `M/repeticao.rs` | componentes (caixa clicável), direção/eixo por aresta, eixo ou face (**referência viva**, relida a cada atualização por `sincronizar`) | Painel livre à esquerda (`paineis::mostrar`): Linear (1–2 direções), Circular, **Pelos furos** (segue o padrão de furos de uma peça e respeita o que ele pulou); `Inverter`, quantidade, espaçamento, ângulo, "Segunda direção", cópias puladas por chave (i, j); **Aceitar/Cancelar** e fantasmas das cópias na cor do tema; seta de espaçamento e marca clicável por posição | `Assembly::passos` gera `Passo` (movimento rígido por posição) |
| **Explosão e animação** (B286) `asm_exibicao.rs`, `M/display.rs` | passos `PassoExplosao { comps, modo }`; `ModoPasso`: Linear, Radial, Espaçado | Abas Estados / Explosão / Animação: lista de passos com liga/desliga, seta do manipulador que vira passo, "Gerar passos" (um por peça), "Explodir", "Juntar tudo", "Animar os passos", quadros-chave com câmera. Sem Concluir: tudo é edição direta | `Explosao::fases` (`display.rs:163`): cada passo parte de onde o anterior deixou. O estado de exibição guarda só o resultado e é o que o Desenho lê |
| **Lista de peças (BOM)** `M/bom.rs`, `asm_ui.rs:4264 bom_window` | a montagem | Janela: nível (`Topo`, `Pecas`, `Estruturada`), "Fixar números" (grava no `.fka`), CSV. Número de item estável entre revisões (`assign_items`); configuração/PN por linha (B102), erro quando a configuração não existe | `bom::bom` percorre nós e subconjuntos; é derivada, não guardada |
| **Furo de montagem** (B57) `M/furos.rs`, `asm_ui.rs:2662 serie_ui` | eixo (`EixoFuro` preso a componente), escopo de peças, padrão | Painel de série com "⇅ Inverter sentido" (`asm_ui.rs:2696`); `sincronizar` grava um `Hole` comum em cada peça | Estado desejado → passada de `sincronizar` (ativo, travado, isolado) |
| **Fixadores / interferência** `M/fixadores.rs`, `M/interferencia.rs` | furos das peças, escopo | Janelas próprias (`asm_ui.rs:4350`, `asm_interf.rs`), prévia de pilhas e candidatos | Interferência: caixas → malhas → booleana `Comum` em paralelo; contato de volume zero por seção |
| **Estudo de movimento** `M/estudo.rs`, `asm_estudo.rs` | juntas e leis em `t` | Painel de esquerda com cursor por grau de liberdade; posições nomeadas; gráficos | `kin::analyze`; `estudo` testa cada grau com comando de experiência |

---------------------------------------------------------------------------------------------------

## 4. Desenho e PMI

### 4.1 Desenho (`.fkd`)

Modelo: `Drawing` (`D/lib.rs:737`) com folhas, `View` (`:300`), `Dim` (`:467`), `Note` (`:585`), tabelas,
carimbo, esboços. Estado de interface: `DwgState` (`dwg_ui.rs:90`), **sem `Session`**; o desfazer é uma
**cópia inteira do `Drawing`** por passo (`undo: Vec<Drawing>`).

| Comando | Entradas | Painel / prévia | Motor, cache |
|---|---|---|---|
| **Vistas** (padrão, projetada, isométrica, planificada, esquema, câmera) `ViewKind` `D/lib.rs:259` | origem (`Source`: arquivo `.fkp`/`.fka`, estado de exibição, ocultos), direção, escala (auto por série ISO 5455, `alcas.rs`), vista mãe (projetada, corte, detalhe) | Ferramenta armada na barra; `dwg_ui.rs:483 block_reason` diz o que falta **antes** do clique; `ghost.rs` mostra o contorno e a projeção na escala real presos ao cursor (rascunho de ids `1<<40`, nada entra antes do clique); barra de opções (escala automática, render, ocultas, rótulo) | `Models::proj` (`source.rs:366`): chave = hash da vista (+ mãe + data de cada arquivo lido); igual ao guardado = reaproveita. `compute` (`source.rs:507`) chama `hlr` do kernel (`drawing.inc:72`, `HLRBRep_Algo` exata) ou a rápida por mapa de profundidade (`hlr_rapida.rs`, B393); exata é **serializada** por mutex (`kernel lib.rs:316`) e, em lote, vai a processos (`hlr_processos.rs`). Montagem vai a thread própria (`fundo.rs`, B427) com prévia e depois exata |
| **Cortes** (`Section`) e **detalhe** `corte.rs` | vista mãe obrigatória; linha de vários segmentos (Enter ou duplo clique fecha), meio corte, fatia, peças sem corte; detalhe por círculo ou perfil livre | `corte_ui.rs`: fantasma durante o traçado, alinhamento com a mãe (Ctrl solta), alças dos vértices da linha, alça de inverter lado e de raio (`D/alcas.rs`, um passo de desfazer cada); hachura por estilo (`hatch_ui`) | Prisma reto subtraído (plano de corte contém a direção de observação): `drawing.inc:208` `BRepAlgoAPI_Common`; face cortada = seção do plano recortada pela faixa do segmento |
| **Cotas** `DimKind` `D/lib.rs:442`, `cotas.rs` | 1 a 3 âncoras **3D** (associativas) clicadas nos vértices da projeção; tipo proposto pelo alvo | **Cota inteligente** (B107) `cota_inteligente.rs`: o que está sob o cursor propõe o tipo, Tab troca, cota viva (fantasma) segue o mouse até o clique de afastamento, ferramenta continua até Esc. Duplo clique abre `dim_edit_window` (texto, tolerância tipada, precisão, setas 0–5, estilo) | Âncora sem vértice (`loose`) não é conferível; `refresh_broken` (`dwg_ui.rs:283`) lista cotas e notas com âncora fora da geometria, e há "reatribuir âncora" (`reassign`). Chamada de furo gerada das features (`Models::chamada_furo`, `source.rs`) |
| **Balões** `baloes.rs`, `baloes_ui.rs` | ponto da chamada + posição; automáticos: lado, distância, passo, janela de 2 cliques, Shift tira um | Comando com prévia (B149): painel livre/encaixável; "Atualizar" renumera pela lista de peças | Número vem da BOM da montagem; balão manual é preservado ao renumerar |
| **Estado da vista** | – | `ViewState` Atualizada/Desatualizada/Erro (`D/lib.rs:418`, selo na folha); "recarregar sozinho" (`auto_update`) | `Models::refresh` compara data dos arquivos; vista com erro aparece como erro, não some |
| **Tabelas, marcas de centro, desenho automático, inspeção (FAI)** `tabela.rs`, `centros.rs`, `automatico.rs`, `inspecao.rs` | origem e canto da folha | Fantasma da tabela com as colunas; `Automático` roda regras de contorno, furos, tabela de furos, carimbo, idempotente | Regras no `fenkra-desenho`, sem interface |

### 4.2 PMI 3D

Modelo: `Pmi` (`fenkra-core/src/pmi.rs:1000`) com `kind: PmiKind` (`:845`: Linear, Raio, Diâmetro, Ângulo,
Nota, Rugosidade, Alvo de referência, Solda, Referência, Tolerância geométrica, Legenda 3D, Balão, Ordenada,
Entre círculos, Comprimento de arco, Chamada de feature), `anchors` e `targets: Vec<Option<PmiTarget>>`.
Peça e montagem têm listas próprias; não passa por `Session`.

| Aspecto | Fato |
|---|---|
| Entradas | `app.picks` da viewport (vértice, aresta, face), `PmiKind::anchors()` diz quantas (1, 2 ou 3). `pmi_ui.rs:331 command`: se a seleção já serve, cria na hora (`create`); senão fica pendente (`pmi_pending`) e avisa o que falta. Cadeia/base e balões automáticos esperam Enter. |
| Referência guardada | **Chave geométrica do elemento** (`PmiTarget`: classe, ponto, direção, raio, côncavo), não o ponto: a anotação reencontra o elemento depois que o modelo muda. `Pmi::diagnose` (`pmi.rs:1380`) devolve `PmiState` Ok / Ambígua / Quebrada com mensagem e âncora que falhou. |
| Painel | `pmi_ui.rs:2554 panel_inner`: nome, medida, texto, letra, modelo da legenda, item, tolerâncias, "Aparência" (estilo), pasta; `gdt_ui`, `weld_ui` com campos estruturados (`GdtExtra`, `WeldExtra`, `RugExtra`). Validação de GD&T em `gdt.rs` (`Aviso`, `Compat`, `Correcao`). |
| Direção / inverter | Variantes por Tab (`next_variant`: linear → ordenada X/Y/Z → entre círculos → linear; raio ↔ diâmetro); plano do texto escolhido por candidatos (`plane_cands`), não é "inverter". |
| Prévia | `pending_preview` (`pmi_ui.rs:1163`): anotação pendente desenhada ao vivo sobre o 3D com destaque (`highlight`) dos elementos pegos; sem recompute do modelo. |
| Motor | `Pmi::measure` (distância/ângulo/raio sobre as âncoras reencaixadas); `snap` ao vértice/aresta mais próximo (`pmi_ui.rs:795`); chamadas de feature geradas por `pmi_feature::callout` (as mesmas do Desenho). Exporta pelo STEP (`kernel/src/pmi_xde.rs`). |
| Cache / dependentes | Sem cache próprio: medição refeita no desenho da cena; `Desenho` importa PMI por `Dim.pmi: Option<Id>` e `atualizar` refaz a partir dela (B371). Apagar a geometria deixa a anotação "Quebrada", não a apaga. |

---------------------------------------------------------------------------------------------------

## 5. Tabelas de paridade entre comandos irmãos

Legenda: ✔ tem · ✘ não tem · ◐ tem com diferença · n/a não se aplica.

### 5.1 Comandos de perfil: Extrusão × Revolução × Varredura × Multi-seções × Nervura × Furo × Plástico

| Recurso | Extrusão | Revolução | Varredura | Multi-seções | Nervura | Furo | Plástico |
|---|---|---|---|---|---|---|---|
| Operação adicionar/remover/interseção | ✔ 3 | ✔ 2 (`revolve_extras`) + combo de 3 | ◐ 2 | ◐ 2 | ✘ só soma | ✘ só corta | ✘ |
| Inverter | ✔ caixa | ✘ | ✘ | ✘ | ✔ caixa | ✔ caixa | ◐ caixa com nome diferente |
| Direção por clique | ✔ `axis_slot` | ◐ combo de eixo | ✘ (curva guia) | n/a | ✘ | ✔ `axis_slot` | ✘ |
| Términos (até, passante) | ✔ 8 | ✘ (ângulo) | ✘ | ✘ | ◐ "até o corpo" | ✔ | ✘ |
| Dois lados | ✔ + término por lado | ◐ 2º ângulo | ✘ | ✘ | ◐ lado da parede | ◐ `back` | ✘ |
| Parede fina | ✔ com lado | ✔ com lado | ✔ sem lado | ✘ | n/a | n/a | n/a |
| Ângulo de saída | ✔ | n/a | ✘ | ◐ (condição de ponta) | ✘ | ◐ conicidade | ✔ (regra) |
| Prévia leve | ✘ | ✘ | ✔ | ✔ | ✘ | ✘ | ✘ |
| Seta/alça no 3D | ✔ várias | ✔ arco | ✘ (guia desenhada) | ✔ intensidade e torção | ✔ profundidade | ✘ (só guia no cursor) | ✔ uma |
| Papel de faces (nomeação) | ✔ | ✔ | ✔ | ✔ | ✘ | ✔ | ✘ |
| Perfil aberto | ◐ só com parede fina | ◐ só com parede fina | descartado com aviso | descartado | ✔ (linhas) | n/a | n/a |
| Painel | `editor.rs` | `editor.rs` | `feat_ui.rs` | `feat_ui.rs` | `feat_ui.rs` | `editor.rs` | arquivo próprio |

### 5.2 Extrusão × Espessar superfície × Nervura (o caso citado no B569)

| Recurso | Extrusão (sólido) | Espessar superfície | Nervura | Extrusão de superfície |
|---|---|---|---|---|
| Entrada | sketch (combo) | superfície (`surf_slot`) | sketch (combo) | curva (`curve_slot`) |
| Seções do painel | Perfil, Extensão, Início e términos, Saída e parede fina | "Engrossar superfície", Direção | sem seção nomeada | `geo_ui` próprio |
| Direção | caixa + `axis_slot` | ícones Normal / Eixo / Normal do plano | caixa "para o lado da normal" | `axis_slot` |
| Lados | `reverse`, plano médio, dois sentidos | Um lado / Dois lados / Simétrico (3 ícones) | 2 lados / 1 / outro (rádio) | – |
| Término | 8 tipos | só espessura | profundidade ou "até o corpo" | limites por superfície |
| Operação | 3 ícones (`op_combo`) | 3 ícones (`hyb_op`, **outro texto**: "Acrescentar") | ✘ | n/a |
| Inverter | caixa "inverter sentido" | seta no 3D | caixa | seta no 3D |
| Alça 3D | várias | uma seta | uma seta | seta |

### 5.3 Filete × Chanfro

| Recurso | Filete | Chanfro |
|---|---|---|
| Constante | ✔ | ✔ (Distância) |
| Duas medidas | ◐ Assimétrico (só reta entre planos) | ✔ 2 distâncias |
| Medida e ângulo | ✘ | ✔ |
| Variável com pontos de controle | ✔ | ✘ |
| Face-face | ✔ | ✘ |
| Completo (tritangente) | ✔ | ✘ |
| Todas, regras B95, propagação, trecho parcial, inverter por aresta, prévia leve | ✔ | ✔ |

### 5.4 Comandos de face: Casca × Ângulo de saída × Espessura × Edição direta × Rosca

| Recurso | Casca | Saída | Espessura | Edição direta | Rosca |
|---|---|---|---|---|---|
| Lista de faces | `faces_ui` (antiga) | `faces_ui` | `faces_ui` | `faces_ui` | texto + botão |
| Regra B95 (tangentes/concêntricas) | ✔ | ✔ | ✔ | ✔ | ✘ |
| Inverter | caixa "para fora" | caixa "inverter a direção" | rádio Acrescentar/Remover | sinal do valor | ✘ |
| Prévia leve | ✔ | ✘ | ✘ | ✘ | ◐ (cosmética) |
| Alça no 3D | seta | arco | seta (só face plana) | setas | seta de comprimento |
| Campo plástico (B198) | ✔ | ✔ | ✘ | ✘ | ✘ |
| Plano/ponto/eixo por clique | n/a | `plane_combo` | n/a | n/a | n/a |

### 5.5 Transformações: Padrão × Espelho × Escala × Mover corpo

| Recurso | Padrão | Espelho | Escala | Mover corpo |
|---|---|---|---|---|
| Fontes (comandos) | ✔ | ✔ | ✘ | ✘ |
| Eixo/plano por clique | `axis_slot` | `plane_combo` | `point_slot` | `axis_slot` / `plane_combo` |
| Inverter | ✘ | ✘ | n/a | ✘ (sinal) |
| Quantidade / parâmetros no 3D | ✔ losango, setas, arco | seta do plano deslocado | ✔ losango | ✔ seta, arco, fatores |
| Prévia leve | ✔ | ✔ | ✘ | ✘ |
| Preserva nomes de faces | por ferramenta | por ferramenta | ✔ | ✔ (afinidade ✘) |

### 5.6 Superfície → sólido: Espessar × Fechar × Costurar × Dividir × Preencher por fronteiras

| Recurso | Espessar | Fechar | Costurar | Dividir corpo | Preencher |
|---|---|---|---|---|---|
| Operação | 3 ícones | 3 ícones | Acrescentar/Remover | – | 3 ícones |
| Inverter lado | seta (e lados 1/2) | n/a | `flip` só pela seta | `flip` só pela seta | n/a |
| Entrada | `surf_slot` | `surf_slot` | `surf_slot` | `surf_slot` | `surfs_list` |
| Tolerância / parâmetros | espessuras | tolerância | – | – | células excluídas |

### 5.7 Restrição × Junta × Repetição × Furo de montagem (Montagem)

| Recurso | Restrição/junta (B244) | Edição de restrição na árvore | Repetição (B263) | Explosão | Furo de montagem |
|---|---|---|---|---|---|
| Modelo de edição | rascunho numa cópia, Aceitar/Cancelar | direto, sem Concluir | rascunho, Aceitar/Cancelar | direto | direto na série |
| Prévia fiel | ✔ solver na cópia | ✘ (aplica ao digitar) | ✔ fantasmas | ✔ | ✘ |
| Inverter | 2 botões + "Inverter" | 2 botões | caixa "Inverter" | seta ⇄ | botão "⇅" |
| Bloqueia se inválido | ✔ | ✘ | ✔ (`pronto()`) | – | ✘ |
| Painel | `asm_cmd.rs` (coluna esquerda) | `asm_ui.rs::panel` | `paineis::mostrar` | aba em `asm_exibicao` | `serie_ui` |

---------------------------------------------------------------------------------------------------

## 6. Dez melhorias mais importantes, em ordem

1. **Contrato único com trait/descritor de feature** (campos, validação, prévia, alças, ícone, motor)
   no lugar dos dois `match` (`editor.rs:1539`, `feat_ui.rs:270`) e dos 6 arquivos de alças. Hoje cada
   comando decide sozinho se tem leve, seta, inverter, regras B95 ou campo plástico; sem descritor, a
   paridade é impossível de checar. Base do B565 e da checagem automática pedida.
2. **Validação antes do OK com bloqueio configurável e reparo guiado.** Hoje o Concluir ignora o status
   (`editor.rs:1208`) e só Montagem/Desenho bloqueiam. Padronizar: botão Concluir desabilitado com o
   motivo, ação "Corrigir" quando há sugestão (filete: raio que cabe; corte: inverter; direção no plano),
   e um estado "esperando seleção" único (já existe em `Inactive`/B459) visível no painel.
3. **Entrada de perfil universal** (B561): Extrusão, Revolução, Varredura, Multi-seções, Nervura,
   Plástico, Furo e Chapa aceitando curvas de sketches diferentes, cadeia tangente e junção na hora; o
   eixo da Revolução por `axis_slot` e o perfil pelo mesmo widget em todos (`sketch_combo`/`sketch_pick`
   hoje divergem).
4. **Botão "Inverter direção" único** (B562) e o mesmo conjunto de lados (Um lado, Dois lados, Simétrico)
   em Extrusão, Espessar superfície, Nervura, Revolução, Varredura, Multi-seções, Mover corpo, Padrão,
   Costurar e Dividir. Hoje são caixas, rádios, setas e flags com nomes distintos.
5. **Operação com os mesmos 3 ícones e textos** (`op_combo`, `hyb_op`, `op_choice`, caixas próprias): um só
   componente com Adicionar/Remover/Interseção quando fizer sentido (Varredura, Multi-seções e Nervura
   passam a oferecer interseção; Revolução e Extrusão ganham "corpo novo").
6. **Prévia leve ou rápida para os demais.** Extrusão, Revolução, Furo, Ângulo de saída, Espessura, Edição
   direta e Combinar recalculam a booleana inteira a cada tecla. Estender `previa_leve` (ferramenta
   desenhada em linhas ou fantasma da ferramenta sem a booleana) e fazer o descritor declarar o nível de
   prévia; medir contra o orçamento de RNF-001.
7. **Alças 3D por contrato**: toda feature com valor dimensional declara a seta (profundidade e diâmetro do
   Furo, ângulo da Rosca/saída em todos, distância de Espelho, ponto da Escala). Hoje Furo, Varredura,
   Combinar, Espelho (valor) e Superfície→sólido (exceto Espessar) não têm.
8. **Cache mais fino para referências por chave**: comandos com `"Face"`/`"Vertex"` assinam **todos** os
   corpos (`lib.rs:7150`), então qualquer mudança refaz Casca, Saída, Espessura, Edição direta, Rosca e
   Filete face-face. Assinar só o corpo-alvo e as chaves citadas (`ref_reads_bodies` já distingue); medir
   no kart.
9. **Nomeação de faces para todo comando**: `apply_roles` dá papel só a Extrusão, Revolução, Furo,
   Varredura e Multi-seções; Nervura, Plástico, Primitiva, Espessar, Costurar, Fechar e Preencher entram
   sem papel e Escala/Afinidade/malha perdem nomes. Estender `papeis::por_perfil` e documentar quem
   preserva e quem não preserva (a Afinidade e o MoveBody são o caso mais visível, `fresh_body`).
10. **Montagem e Desenho no mesmo modelo de sessão**: Montagem tem dois fluxos (rascunho B244 × edição
    direta da árvore) e Desenho guarda o desfazer como cópia do `Drawing` inteiro. Unificar em "rascunho com
    prévia fiel + Aceitar/Cancelar" (Repetição e Restrição já são o modelo), desfazer incremental e o
    mesmo bloco de painel livre/encaixável, o mesmo critério de erro (mensagem + causa + ação) e a
    mesma matriz de paridade (inverter, prévia, alça) entre Restrição, Junta, Repetição, Explosão e Furo
    de montagem.

Menções de menor prioridade: `faces_ui` antiga (sem Tab) → caixa universal; `plane_combo` → `plane_slot`
(selecao-auditoria, B561); Chanfro sem variável/face-face/completo; Filete sem distância+ângulo; Revolução
sem término "até"; Combinar por combo em vez de clique no corpo; Forma livre e malha sem painel padrão
(abrem fluxo próprio).

---------------------------------------------------------------------------------------------------

## 7. Fontes de verdade consultadas

`fenkra-core/src/doc.rs` (1843-2260, 3060-3180, 3775-4420), `pmi.rs`; `fenkra-solidos/src/lib.rs`
(4598-5345, 6541-6930, 6970-7330), `previa_leve.rs`, `termino.rs`, `edicao_direta.rs`, `plastico_rec.rs`,
`corpo_malha.rs`, `forma_brep*.rs`; `fenkra-kernel/cpp/*.inc`, `kernel.cpp`; `fenkra-app/src/{editor,feat_ui,
app,previa,handles,handles/*,asm_ui,asm_cmd,asm_repeticao,asm_exibicao,asm_inserir,dwg_ui,dwg_ui/*,pmi_ui,
malha_ui,forma_ui,plastico_rec_ui,edicao_direta_ui}.rs`; `fenkra-montagem/src/{lib,kin,solve,proposta,
display,repeticao,bom,furos,interferencia,estudo,insercao}.rs`; `fenkra-desenho/src/{lib,source,corte,cotas,
baloes,automatico,hlr_rapida,hlr_processos}.rs`. As skills `fenkra-solidos`, `-montagem`, `-desenho` e
`-pmi` são gabaritos sem diretrizes preenchidas (só "lições do kart"), então nada nelas contradiz o que está
acima.
