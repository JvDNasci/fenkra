# Estado atual dos comandos: Chapa e Estrutural (base do B565 e do B569)

Levantamento de 21/11/2026, só de leitura do código (develop depois da alpha.41). Não repete a
[auditoria dos campos de seleção](../../benchmark/detalhe/selecao-auditoria.md) (tabelas "Chapa" e
"Estrutural" dela valem como o quadro de *widgets* de seleção); aqui entram painel, prévia, validação,
motor, saídas, cache e problemas. Caminhos relativos a `crates/`. "Não verificado" marca o que veio só
da leitura do código, sem rodar.

## 0. Como todo comando de Chapa e de Estrutural passa pelo mesmo caminho

| Etapa | Onde | O que faz |
|---|---|---|
| Dado | `fenkra-core/src/doc.rs:1843` (`FeatureKind`), structs `Sm*` em 2447–3043, `Struct*` em 2016–2350 | Campos em texto (expressões), pontos `[f64;3]` e chaves `EdgeKey`/`FaceKey`/`Ancora`. |
| Painel | `fenkra-app/src/feat_ui.rs:panel` (a partir de 573; chapa em 596–1011), `chapa_unif_ui.rs`, `chapa_laco_ui.rs`, `chapa_aba_limites_ui.rs`, `chapa_b35_ui.rs`, `chapa_canto_ui.rs`; estrutural em `st_ui.rs:23`, `st_placas_ui.rs`, `st_solda_ui.rs` | Seções por `section(ui, ...)`; "Suporte, Modo, Parâmetros, Avançado" só existe na aba (`chapa_unif_ui::suporte` e `::modos`). |
| Clique no 3D | `feat_ui.rs:24–62` (`wants_faces`, `add_face`) | Comandos que aceitam clique de face: Casca etc. mais `StructPlate`, `StructWeld`, `SmCornerRelief`, `SmCornerClose`, `SmConvert`, `SmArcSlot`, `SmSurfFlange` (grava `face`), `SmFlangedHole` (sem sketch). |
| Cálculo | `fenkra-solidos/src/lib.rs:4698` (`run`) e `:4709` (`run_feature`); `match` dos comandos em 5345–6539 | Devolve `Ok(None)`, `Ok(Some(aviso))` ou `Err(texto)`. |
| Status | `lib.rs:7225–7280` | `Ok` / `Warning` / `Error`; `Error` vira `Inactive` quando a entrada ainda é "a escolher" (`aresta_a_escolher`, `lib.rs:231`; `ARESTA_A_ESCOLHER`). |
| Cache | `lib.rs:7135–7300` | Um registro por comando (`FeatEntry`), com assinatura = JSON do comando + variáveis que ele cita + assinatura do corpo alvo + `last_sheet` + marca "leve" + estado de desdobra parcial; membro depende das linhas de todos os outros membros (7134–7146). Guarda corpos tocados, `sheet_maps`, `previas`, `cut_outlines`, `tool`. |
| Prévia | `fenkra-solidos/src/previa_leve.rs` e `fenkra-app/src/previa_leve_ui.rs` | Ver seção 3. |
| Erro na tela | `fenkra-app/src/editor.rs:1017–1075` | Texto vermelho "✖ mensagem" mais diagnóstico (`erro_acao::diagnosticar`) no painel; o painel **não fecha** sozinho; marca "✖" na árvore (`app.rs:5893`). |

Achados do caminho comum (valem para todos os comandos abaixo):

- **A1. Falha não desfaz o corpo.** `run` (`lib.rs:4698`) não guarda nem restaura `bodies` nem `sheet_maps`
  quando o comando devolve `Err`; o `FeatEntry` grava o que estiver em `cx.bodies` (`lib.rs:7280–7300`).
  Exemplos de efeito colateral antes do ponto de falha: o corte do recuo "material por fora/dentro" da aba
  curva (`lib.rs:4250`) vem antes da união que pode falhar (`lib.rs:4341–4373`); `sheet_tool` empilha o `SheetMap`
  (`lib.rs:4024`) antes de `valida_lateral` (`lib.rs:4040`). Resultado possível: comando em erro com
  corpo parcialmente cortado e dobra fantasma no mapa. Não verificado por teste.
- **A2. Dobrado e planificado são dois cálculos separados.** `SheetParams.flat` (`doc.rs:2894`) entra na
  assinatura global (`lib.rs:7048–7056`, via `sheet_vars`); o botão Planificar só inverte `flat`
  (`app.rs:2702–2706`). Pela leitura, alternar o estado reavalia a árvore toda; não há guarda dos dois
  corpos (B566). `Feature.flat_space` (`doc.rs:1836`) guarda em que estado cada comando foi criado e
  `Ctx::map_point` (`lib.rs:4385`) leva pontos de um estado ao outro.
- **A3. Âncora de aresta/face.** Só a aba de aresta, curva, laço, bainha, joggle (linha), furo/dobra por linha
  e a placa/reforço (ponta de membro) passam por `chapa_aresta` (`lib.rs:3906`), `linha_na_face`
  (`ancoras.rs:83`) ou `membro_ancorado` (`ancoras.rs:93`). Os demais guardam só o ponto.
- **A4. Inverter.** Chapa tem o botão padrão `chapa_unif_ui::inverter` (`chapa_unif_ui.rs:30`, texto
  "⇄ Inverter direção"); o Estrutural usa quatro nomes diferentes (seção 6).

## 1. Comandos de Chapa

Legenda do motor: **Rust** = função em `fenkra-solidos`; **kernel** = função C++ em `fenkra-kernel/cpp/*.inc`
(declarada em `fenkra-kernel/src/ffi.rs`, embrulhada em `fenkra-kernel/src/lib.rs`).

### 1.1 Parede base (`SmWall`, `doc.rs:2988`; painel `feat_ui.rs:596`; motor `lib.rs:5345`)

- **Entradas.** Sketch (`Id`) com um contorno externo fechado (mais furos), ou superfície (`SurfRef`, modo
  "de superfície"). Sem `EdgeKey` (o sketch é a referência). Painel: `sketch_pick` ou `geo_ui::surf_slot`.
- **Painel.** Seção "Parede base": alternador "de sketch"/"de superfície", Perfil ou Superfície, "Afastamento do
  plano" (só sketch), botão Inverter direção, "espessura dividida pelos dois lados". Espessura vem de
  Chapa › Parâmetros. Falta: ponto de partida/posição pela face externa/interna (existe `offset` e `mid`, mas
  não um seletor "face interna/externa" no padrão da aba); sem campo para escolher vários sketches.
- **Direção e limites.** `reverse` (inverter) e `mid` (meio). Sem limites.
- **Prévia.** Completa (saiu da leve em B470, `previa_leve.rs:33–40`). Sem alças 3D.
- **Validação.** Sem perfil fechado: "a parede base precisa de um perfil fechado" (`lib.rs:5357`); mais de um
  contorno externo disjunto: erro (B553, `lib.rs:5363`); segunda parede que não encosta: erro (`lib.rs:5378`).
  Painel permanece aberto com o erro.
- **Motor.** Rust `lib.rs:5345`; kernel `Shape::face` + `extrude_roles` (face do sketch extrudada pela
  espessura, com papel por face); de superfície: `cut_shape` + `thicken` (`hybrid.inc:868`). Ideia: extrusão do
  perfil por `t` (ou `t/2` para cada lado) e fusão com o corpo.
- **Saídas.** Corpo (papéis para nomear arestas). Parede de superfície não gera `SheetMap`.
- **Problemas.** B558: perfil de asa em spline fechada sai como fita deformada; B559: spline chega ao kernel
  facetada (N segmentos, `Seg2::Spline` existe em `fenkra-kernel/src/lib.rs:122`, mas o caminho do sketch não
  garante uma aresta só).

### 1.2 Aba de aresta (`SmFlange`, `doc.rs:3004`; painel `feat_ui.rs:621`; motor `chapa_laco.rs:226`)

- **Entradas.** 1 aresta reta (`edge` ponto + `edge_key: EdgeKey` dono+papel). Widget `campo_aresta`
  (`chapa_unif_ui.rs:92`); clique em curva ou face troca de modo (`slots::aresta_clicada`).
- **Painel.** "Suporte" (campo de aresta) > "Modo da aba" (miniaturas) > "Parâmetros" (Comprimento reto, Ângulo,
  "raio próprio" com cadeado de estilo, "Lado do material" 3 opções, Inverter direção) > "Avançado" (Recuo
  início, Recuo fim, "pontas em meia-esquadria", "recuo mínimo", "alívio de canto" com Largura e Tipo e cadeado).
- **Direção e posição.** `flip`; `side` 0 dobra por fora / 1 material por fora / 2 material por dentro
  (recua `t` ou `t+r`, `chapa_laco.rs:268`). Sem limites.
- **Prévia.** Leve "por dentro" (`previa_leve.rs:96`, `por_dentro`): faz a ferramenta e a linha de dobra, pula
  colisão e booleana (`lib.rs:3984–4060`, `previa_da_ferramenta`). Alças 3D: ângulo (arco), raio, recuo 1 e
  recuo 2 (`chapa_manip.rs:157–181`).
- **Validação.** Aresta em (0,0,0) = "a escolher", inativo sem erro; faixas válidas (`ev_faixa`: ângulo
  0,01–180, comprimento ≥ 0, raio ≥ 0,01); recuos maiores que a aresta ("os recuos laterais são maiores que
  a aresta", `sheet.inc:109`); colisão com aba vizinha (`chapa::colisao`, `lib.rs:4003`); lateral diferente da
  espessura (`valida_lateral`).
- **Motor.** Rust `run_flange` → `sheet_tool` (`lib.rs:3984`); kernel `sm_flange` (`sheet.inc:51`): acha a aresta
  reta mais perto do ponto, gira a tira lateral em torno do eixo no raio interno (`MakeRevol`) e prolonga por
  prisma; planificado: prisma da tira com comprimento desenvolvido `ângulo·(r+k·t)+reto`. Mitra: dois meios
  espaços a 45° (`lib.rs:3995`); alívio: caixa, oblongo ou rasgo cortados (`chapa_laco.rs:291–326`).
- **Saídas.** Corpo; `SheetMap` (mapa de dobras, tabela, planificada); papéis `papeis::aba_topo` (arestas da
  aba ganham nome); `last_sheet` (usado por bainha e cantos). Dependentes: bainha, fechamento de canto,
  desdobrar parcial, recortes "através da dobra".
- **Problemas.** B560 (modos Aresta, Curva, Laço devem virar um só). Não aceita aresta curva (erro do kernel
  "a aba precisa de uma aresta reta", `sheet.inc:69`); a interface contorna trocando para aba curva.

### 1.3 Aba curva (`SmCurveFlange`, `doc.rs:2665`; painel `feat_ui.rs:924`; motor `lib.rs:5942` → `curve_flange_lim` `lib.rs:4196`)

- **Entradas.** 1 aresta de uma cadeia tangente: `edge: Vec<V3>` (polilinha, só cache), `edge_key` (uma aresta
  da cadeia), `so_aresta` (não seguir tangentes), `reta` (aba reta guardada B531). Widget `campo_aresta` e
  caixa "seguir as arestas tangentes". Cadeia refeita do B-rep por `cadeia_tangente` (`chapa_dobra_curva.rs:51`).
- **Painel.** Suporte, Modo, Parâmetros: Comprimento, Ângulo (vazio vira 90), "raio próprio" (sem cadeado de
  estilo, diferente da aba de aresta), "Lado do material" (3 opções com dica), Inverter direção.
- **Direção e posição.** `flip`, `side` 0/1/2 (recuo `r·tan(a/2)` ou `(r+t)·tan(a/2)`, `lib.rs:4231`). Sem limites.
- **Prévia.** Completa (saiu da leve em B540). Sem alças 3D. Tempo conhecido: pequeno (varredura mais uma
  booleana), mas até 4 tentativas de união com seção entrando 0,05 / 0,3 / 0,5 mm (`lib.rs:4341`).
- **Validação.** Ângulo 1–179° (`lib.rs:4200`), aresta curta, "a união da aba com a chapa saiu inválida ou
  vazia" (B529, `lib.rs:4373`); sem aresta: "escolha uma aresta (reta ou curva)".
- **Motor.** Rust `curve_flange_lim`; kernel `sm_chain_sweep` (`sheet.inc:154`): varre a seção (arco interno,
  parede, arco externo) ao longo da cadeia segmentada (reta, arco ou spline) com a normal da chapa fixa;
  cadeia reta usa extrusão. Planificado: faixa `ba + comprimento` extrudada para fora (`lib.rs:4262`).
- **Saídas.** Corpo; `marca_cadeia` (`chapa_dobra_curva.rs:43`) registra linhas de dobra no mapa (sem giro).
- **Faltam, comparado à aba de aresta.** Recuos início/fim, alívio, mitra, cadeado do raio, alças 3D.
- **Problemas.** B560, B559 (curva chega facetada), B566 (planificar).

### 1.4 Aba por laço (`SmFlangeLoop`, `doc.rs:2630`; painel `chapa_laco_ui.rs:44`; motor `chapa_laco.rs:335`)

- **Entradas.** V arestas (`edges` pontos + `edge_keys`), ou o laço da face (`loop_all`), mais `extent`
  (`ExtensaoAba`: inteira, centrada, deslocada, entre duas referências), `invert`. Widget `geo_ui::bag`
  (`chapa_unif_ui.rs:55`).
- **Painel.** Suporte (lista de arestas, clique na face pega o contorno, "usar o laço da face inteira") >
  Parâmetros ("Extensão lateral" 4 modos, "medir a partir da outra ponta", Comprimento, Ângulo, raio próprio
  com cadeado, Lado do material, Inverter) > Avançado (mitra, alívio com Largura e Tipo e cadeado).
- **Direção e posição.** `flip`, `side` 0/1/2 (igual à aba de aresta). Sem limites.
- **Prévia.** Leve por zona (`previa_leve.rs:66`, `so_no_fim`): caixa dos pontos das arestas inflada por
  `comprimento+t+raio`; só quando é o último comando ativo. Sem alças 3D.
- **Validação.** "escolha pelo menos uma aresta"; "aba i de n: ..." com o erro da aresta; "as arestas do laço
  não se emendam".
- **Motor.** Rust `run_flange_loop` → uma `run_flange` por aresta reta (com `ext`); trechos com curva viram
  `curve_flange` (`chapa_laco.rs:400–429`). **Limite conhecido do código:** nas cadeias curvas a extensão
  lateral, o alívio e a mitra são ignorados (`curve_flange` recebe só ângulo, raio, comprimento, `flip` e `side`).
- **Saídas.** Várias abas num comando, cada uma com `SheetMap`; promoção das chaves restaurada ao final.
- **Problemas.** B560.

### 1.5 Aba por perfil (`SmProfileFlange`, `doc.rs:2600`; painel `feat_ui.rs:910`; motor `lib.rs:5913`)

- **Entradas.** Sketch com cadeia aberta (`sketch_pick`). Sem aresta guardada.
- **Painel.** Parâmetros: Comprimento lado 1, Comprimento lado 2, raio próprio (sem cadeado), Inverter direção.
  Falta: lado do material (posição), ângulo (vem do desenho), recuos, alívio, limites.
- **Direção.** `flip` = lado da espessura em relação à cadeia.
- **Prévia.** Leve por zona (caixa dos pontos do sketch inflada, `previa_leve.rs:382`).
- **Validação.** "a aba por perfil precisa de uma cadeia aberta (linhas e arcos) no sketch".
- **Motor.** Rust `lib.rs:5913`: cantos arredondados `round_corners(r + t/2)`, `profile_face` (parede fina) e
  `extrude(normal, l1, l2)`; planificada `aba_perfil_planificada` (`chapa_flat.rs:205`, tira desenvolvida por
  `desenvolver_perfil`). Ideia: parede fina varrida em linha reta ao longo da normal do plano do sketch.
- **Saídas.** Corpo; linhas de dobra por canto (planificada).
- **Problemas.** Cadeia só com linhas e arcos (spline não arredonda canto; B559).

### 1.6 Aba por superfície (`SmSurfFlange`, `doc.rs:2708`; painel `feat_ui.rs:951`; motor `lib.rs:5966`)

- **Entradas.** Suporte (`SurfRef`, `surf_slot`), face da web (`face: [f64;3]`, **só ponto**, sem `FaceKey`, mostrado como
  texto: `feat_ui.rs:259`), limites opcionais (`LimitesAba`, `doc.rs:~2760`: lado 1, lado 2, borda final como
  `LimiteAba::Superficie|Curva`; cantos filete/chanfro).
- **Painel.** Suporte (texto da face + Superfície) > Parâmetros (Tipo: Exato, Aproximação, Ângulo; Ângulo só no
  modo Ângulo; Comprimento (EOP); raio de dobra próprio **sem cadeado**; Inverter direção; "Limites da aba":
  três campos com liga/desliga e "plano ou superfície / curva ou aresta"; "Cantos") > Avançado ("não relimitar a
  web pelo suporte"). **Faltam:** Lado do material / posição (o código passa `side = 0` fixo,
  `lib.rs:6133`), recuos, alívio, mitra, extensão lateral, alças 3D, ponta de referência da face como campo
  de seleção.
- **Direção e limites.** `flip` vira `d = ±n` e é passado como `want` (`lib.rs:6133`), o que ignora o
  `flip` interno de `curve_flange_lim`. Único comando com lado 1, lado 2 e borda final.
- **Prévia.** Completa (B540); lenta com suporte grande e curvo (B564). Sem cache da superfície de suporte
  entre quadros (`cut_shape` refeito a cada cálculo, `lib.rs:5986`).
- **Validação.** "o suporte da aba por superfície não está disponível (...): escolha o suporte de novo";
  "o suporte não corta o plano da web perto dela"; "o suporte não relimita a base: a linha de dobra não atravessa
  a placa"; "os limites dos lados não deixam trecho da aba sobre a web"; "o limite da borda final não intercepta a
  aba (nem estendida)". Face em (0,0,0): aviso "clique na face da web".
- **Motor.** Rust `lib.rs:5966–6133`: OML = `Shape::section(suporte, plano da web)`, amostrada em ~200 pontos
  (`lib.rs:5986`), trecho perto da web; relimita a web por `split_side` (`hybrid.inc:402`) ou corte por lado;
  ângulo vem de `cross(ns, tan)`; depois `curve_flange_lim` varre a seção (`sm_chain_sweep`). Limites:
  `chapa_aba_limites.rs` (`aparar_oml`, `aba_cortador_ponta`, `aba_pos_ferramenta`). **A OML sai como polilinha
  densa e o kernel a refaz em retas, arcos ou spline por heurística** (`chapa::chain_segs`): é a causa provável
  da faceta do B559.
- **Saídas.** Corpo; `marca_cadeia`; sem ramo próprio no estado planificado (nenhum `sp.flat` no trecho
  5966–6133 além de `ponto_na_face`): depende do ramo plano de `curve_flange_lim` (B566, "não planificam").
- **Problemas.** B563 (posição, inversão, contato: não verificado em bancada), B564, B566, B559.
- **B559 (22/11).** Aba curva e aba por superfície tentam primeiro a cadeia exata (`cadeia_exata`: as
  arestas da chapa sobre a cadeia, cada uma com a curva do B-rep) e varrem por `sm_chain_sweep_c`; a
  polilinha segue só quando a aba recua (lado 1/2) ou quando a OML aparada não coincide com a aresta da
  web (limites dos lados). Perfil de asa: 6 faces novas em vez de 18. Membro curvo de um trecho exato
  varre pela curva exata.

### 1.7 Bainha (`SmHem`, `doc.rs:3043`; painel `feat_ui.rs:672`; motor `lib.rs:6181`)

- **Entradas.** 1 aresta (`edge` + `edge_key`), mostrada por `edge_line` (rótulo), sem widget de campo; vem do
  clique.
- **Painel.** Seção "Bainha": Tipo (fechada/aberta, gota, enrolada), Comprimento, Folga interna, Inverter direção.
  Falta o que a aba de aresta tem: lado do material, recuos, mitra, alívio, cadeado de estilo, widget `campo_aresta`.
- **Prévia.** Leve por dentro (`por_dentro`); alça 3D de folga (`chapa_manip.rs:163`).
- **Validação.** Mesma da aresta; `colide = false` (a bainha aperta a chapa de propósito, `lib.rs:4000`).
- **Motor.** Rust `lib.rs:6181` → `sheet_tool` → kernel `sm_flange` com ângulo 180°/200°/270° e raio = folga/2.
- **Saídas.** Corpo; `SheetMap`; animação a trata como "fixo" (`chapa_animacao.rs`).

### 1.8 Joggle / desvio em Z (`SmJoggle`, `doc.rs:2447`; painel `feat_ui.rs:690`; motor `lib.rs:5391`)

- **Entradas.** Linha a–b na face (2 pontos, ancorada por `linha_na_face`) **ou** plano (`PlaneRef`, `plane_combo`).
- **Painel.** Seção "Desvio em Z": "por dois pontos"/"por plano", Desvio, "rampa pelo ângulo" (Ângulo ou Rampa),
  "compensação", Inverter direção. A linha é desenhada no 3D (`chapa_manip_ui.rs:283`).
- **Prévia.** Leve por zona (pontos a, b inflados); alças 3D: Desvio, Ângulo da rampa, Rampa (`chapa_manip.rs:185`).
- **Validação.** Esperando linha (`a == b`): aviso, sem erro (`lib.rs:5262`); "ângulo da rampa entre 1° e 89°";
  "o plano não corta a face".
- **Motor.** Rust `lib.rs:5391–5501`: meios espaços (caixas) separam os dois lados, `poly_prism` faz rampa e ponte
  (`sheet.inc:260`); chapa deslocada por translação. Planificada: marca no mapa de dobras como linha
  (`chapa_dobra_curva.rs`).

### 1.9 Dobra por linha (`SmBend`, `doc.rs:2469`; painel `feat_ui.rs:722`; motor `lib.rs:5502` + `bend_line` `lib.rs:3926`)

- **Entradas.** Linha a–b (ancorada) **ou** sketch (várias retas paralelas, ou uma curva).
- **Painel.** "Dobra por linha": checkbox "linhas de um sketch", Sketch ou texto da linha, "A linha marca" (4
  posições: centro, início, fim, vértice externo), Ângulo, raio próprio (sem cadeado), Inverter direção,
  "dobrar para o outro lado" (`down`), "dobra cega" com "Furo de alívio".
- **Prévia.** Leve por dentro; alças 3D de Ângulo e Raio e **minibarra de posição** (`chapa_manip_ui.rs:492`).
- **Validação.** "ângulo da dobra entre 0,1° e 179,9°"; "as linhas de uma dobra precisam ser paralelas";
  "as linhas de dobra estão a X mm uma da outra, menos que a zona dobrada".
- **Motor.** Rust `bend_line`: separa o corpo em lado fixo, seção (`Common` com plano) e lado móvel (`Common` com
  meio espaço); gira a seção por `revolve` (zona do raio) e o lado móvel por `Xform::Rotate` depois de recuar
  `ba`. Ideia: o comprimento desenvolvido sai da zona dobrada (fator K). Cega: `chapa_cega.rs`.
- **Saídas.** `SheetMap` por linha; planificada desfaz a dobra e marca a linha.
- **Problemas.** Aviso de B469: dobra nasce sem linha (`a == b`).

### 1.10 Dobra cônica (`SmCone`, `doc.rs:2498`; painel `feat_ui.rs:766`; motor `chapa_cone.rs:97`)

- **Entradas.** Números: r1, r2, altura, centro `[f64;3]` (digitado), fator K. Sem seleção no 3D.
- **Prévia.** Completa (B470); sem alças. **Validação.** r2 > r1, aviso de volume plano diferente.
- **Motor.** Rust `run_cone`; kernel `revolve` de um retângulo (geratriz × espessura); planificada: setor de coroa
  exato (`chapa_cone.rs`). **Saídas.** Corpo; mapa de dobras como setor. Sem inverter (o eixo é Z fixo).

### 1.11 Veneziana (`SmLouver`, `doc.rs:2526`; painel `feat_ui.rs:787`; motor `lib.rs:5626`)

- **Entradas.** Linha a–b (como a dobra por linha). **Painel.** Largura da rampa, Altura da abertura, Inverter
  direção, "levantar para o outro lado". **Prévia.** Leve por zona; alça 3D só de Largura (`chapa_manip.rs:210`).
- **Validação.** "a altura da veneziana precisa ser maior que a espessura".
- **Motor.** Rust: `poly_prism` do corte e do capuz (`sheet.inc:260`), `Cut` + `Add`; planificada:
  `veneziana_planificada` só registra o contorno da ferramenta (`chapa_flat.rs:109`, `cut_outlines`).

### 1.12 Estampo (`SmStamp`, `doc.rs:2540`; painel `feat_ui.rs:796`; motor `lib.rs:5661`)

- **Entradas.** Sketch (perfil fechado, curvas ou pontos conforme o tipo) ou corpo-punção (coletor). Cinco tipos
  (`SM_STAMP_KINDS`). **Painel.** Tipo, Perfil/Curvas/Pontos ou Corpo do punção, Profundidade (ou "Aba do
  reforço"), Largura/Diâmetro (tipos 1–3), Inverter direção (menos no tipo 3).
- **Prévia.** Leve por zona. **Validação.** Um erro por tipo ("o estampo precisa de um perfil fechado", "a nervura
  precisa de linhas ou curvas", ...).
- **Motor.** Rust: perfil = `offset_face(t)` extrudado menos a face extrudada (`Add` + `Cut`); nervura =
  `varrer` de trapézio ao longo da curva; circular = `revolve` de trapézio; reforço de dobra = `poly_prism`;
  punção = `puncao_planificado` na planificada. Sem cache próprio.

### 1.13 Furo flangeado (`SmFlangedHole`, `doc.rs:2562`; painel `feat_ui.rs:840`; motor `lib.rs:5812`)

- **Entradas.** Centros por sketch **ou** lista de pontos clicados na face (`points`, sem âncora), tabela de série
  (`FLANGED_HOLES`) ou medidas à mão, "＋ Cadastrar na tabela". **Painel.** Centros, tabela, Diâmetro, Altura do
  colar, Inverter. **Motor.** `hole_tool` (`ffi.rs:204`): anel externo (`Add`) menos furo (`Cut`).
  **Prévia.** Leve por zona.

### 1.14 Recorte na chapa (`SmCut`, `doc.rs:2579`; painel `feat_ui.rs:885`; motor `lib.rs:5860`)

- **Entradas.** Sketch fechado (`sketch_pick`). **Painel.** Modo (normal, com saída, flangeado), Folga, Ângulo de
  saída ou Altura do colar, Inverter direção, "através da dobra" / "normal à face".
- **Prévia.** Completa (o contorno nos dois estados só o cálculo real dá, `previa_leve.rs:96`); a janela de
  sentido corrige o lado sem material (`sentido_corte_ui`, B392).
- **Motor.** `offset_face` + `extrude` / `extrude_taper` (`ffi.rs:251`) + `Cut`; "através da dobra" leva o
  contorno de um estado ao outro por `sheet_maps` (`lib.rs:4437`, `map_shape`). `sheet_mismatch` (`lib.rs:4490`)
  decide quando um comando não-chapa precisa ser levado ao estado atual.

### 1.15 Aba entre perfis (`SmLoftFlange`, `doc.rs:2813`; painel `chapa_b35_ui.rs:15`; motor `chapa_perfis.rs:15`)

- **Entradas.** 2 sketches fechados (`sketch_pick` ×2). **Painel.** "Perfis", "Transição facetada". Sem inverter
  no painel (campo `flip` existe no dado, não na UI: não verificado).
- **Validação.** "o perfil precisa ser um contorno fechado"; "os dois perfis estão no mesmo lugar"; "a espessura de
  X mm não cabe na transição".
- **Motor.** `loft_tool` (`ffi.rs:233`) + `shell_tagged` (esvazia pela espessura); planificada por triangulação
  (`chapa_perfis_plana.rs`): aviso "sem planificação exata". Prévia leve por zona (caixa dos dois perfis).

### 1.16 Desdobrar/redobrar (global e parcial)

- **Global.** `Cmd::SmUnfold` inverte `SheetParams.flat` (`app.rs:2702`); selo "Chapa planificada" (`chapa_estado.rs`);
  animação 0–100 % por reescrita do ângulo dos comandos (`chapa_animacao.rs`, `chapa_anim_ui.rs`). Reavalia a árvore
  (A2).
- **Parcial.** `SmUnfoldPartial` (`doc.rs:2827`; painel `chapa_b35_ui.rs:27`; motor `chapa_desdobra.rs:58`): lista de
  caixas de marcar das abas; `refold` troca desdobrar por redobrar. Erros: "escolha pelo menos uma aba",
  "só abas e bainhas desdobram", "vem depois deste comando". Não tem prévia leve (`previa_leve.rs:33`); a assinatura de
  cada aba ganha `d{true|false}` (`lib.rs:7126`). Inverter: refazer (sem botão).

### 1.17 Converter em chapa (`SmConvert`, `doc.rs:2837`; painel `chapa_b35_ui.rs:55`; motor `chapa_converte.rs:227`)

- **Entradas.** Face fixa (ponto, sem `FaceKey`; clique grava `face`, `feat_ui.rs:60`), espessura, "sólido maciço:
  esvaziar em chapa" com lista de faces abertas (`open_faces`, pontos, ✕ para tirar), botão "Medir o corpo".
- **Prévia.** Leve: caixa do corpo inteiro. **Validação.** "o corpo tem X mm, e não os Y pedidos", "escolha ao menos
  uma face a abrir", "o corpo não se planifica".
- **Motor.** Reconhecimento pela malha (`chapa_converte.rs`, faces paralelas de costas, cilindros coaxiais),
  mapa de dobras por grafo (`chapa_converte_mapa.rs`), corpos de cantos vivos por busca em largura com rasgo
  e alívio (`chapa_converte_vivas.rs`); maciço: `shell_tagged` antes. Sem cache próprio além do geral.

### 1.18 Cantos: alívio, fechamento, rasgo em arco

- **Alívio de canto** (`SmCornerRelief`, `doc.rs:2855`; `feat_ui.rs:998`; `chapa_canto.rs:300`): ponto de canto por
  clique (`at`, sem âncora), tipo (redondo, quadrado, gota, aeronáutico, rasgo), Tamanho; `Cut`. Sem inverter.
- **Fechamento de canto** (`SmCornerClose`, `doc.rs:2780`; `chapa_canto_ui.rs:25`; `chapa_canto.rs:349`): vértice
  por clique; Tipo (topo, sobreposto, sob, aberto), Folga (alça 3D, `chapa_manip.rs:202`), Razão, canto estreito,
  "propagar a todos os cantos", "inverter qual aba cobre", alívio no vértice. Erros: "precisa de duas abas de 90°
  antes dele", "nenhum canto entre abas perto do ponto escolhido". Lê `sheet_maps` (abas já feitas); peças
  geradas por `pecas` (`chapa_canto.rs:164`) e unidas.
- **Rasgo em arco** (`SmArcSlot`, `doc.rs:2696`; `feat_ui.rs:989`): centro por clique, Raio, Largura, Ângulo inicial,
  Varredura; `face` + `extrude` + `Cut`. Erro: "raio maior que meia largura e varredura diferente de zero".
- Os três: prévia leve por zona; sem inverter (o fechamento tem a caixa própria).

### 1.19 Material, estilos e fabricação (apoio de todos)

`SheetParams` (`doc.rs:2886`): espessura, raio, fator K, método (K, dedução, tolerância), material (tabela
`SHEET_MATERIALS`), estilo com cadeado por chave (`overrides`), alívio e canto padrão, ordem de dobras. Fica em
Chapa › Parâmetros, não no comando. Lista de dobras, DXF e planificada exata: `planificada.rs`; simulação de
sequência: `chapa_dobra_sim.rs`; nesting: `nesting.rs`; DFM: `dfm*.rs`.

## 2. Comandos do Estrutural

### 2.1 Membro (`StructMember`, `doc.rs:2016`; painel `st_ui.rs:23`; motor `lib.rs:6205`)

- **Entradas.** `paths: Vec<CurveRef>` (linhas guia) em `geo_ui::curves_list` (cada linha vira um membro no
  cálculo; polilinha é fatiada em cantos de mais de ~11°, `lib.rs:3317–3329`, o que gera N membros soltos:
  B575). Família, designação e cotas da seção (`st_painel::seletor`).
- **Painel.** "Perfil" (família e cotas), "Linhas guia (esqueleto)" (caixa de curvas), "Ancoragem e orientação"
  (âncora, Rotação, Deslocamento X e Y, Prolongar início e fim), "Cantos (padrão)" (4 ícones, Folga de solda,
  Material), "Cantos com outros membros" (combo por canto, quem continua, folga, dobra de tubo), ficha do tubo.
  Inverter: "espelhar seção" (`mirror`, caixa). Falta: botão padrão de inverter, Limites (lado 1/2), campo
  universal para várias guias como uma cadeia só.
- **Referência.** `CurveRef::Edge(pts)` promovida a chave por `promover_caminhos` (`ancoras.rs:156`, B516); canto
  guardado por (comando, trecho) em `StructJoint` (`doc.rs:2050`) no comando posterior do par.
- **Prévia.** Leve: cada membro como caixa orientada da seção ao longo da linha (`previa_leve.rs:372`,
  `caixa_do_membro`); só quando é o último comando; alças 3D via `st_frames.rs` e `st_esqueleto.rs`.
- **Validação.** Falha de uma linha não derruba as outras: vira aviso "MEMBRO_FALHOU n de m linhas não geraram
  sólido" (`lib.rs:6502`); erro só se nenhuma linha gera (`lib.rs:6460`). Avisos: ponta no meio de outro membro sem
  Aparar/Encaixe (K-09), barra maior que a matéria-prima, avisos de dobra de tubo.
- **Motor.** Rust `members()` (`lib.rs:3304`) monta o quadro de cada trecho (z = tangente, x pela normal do plano,
  `angle`); `member_shape` (`lib.rs:3364`): `Shape::face(seção)` + `extrude` (reta) ou `varrer` (`lib.rs:3425`, curva
  por `sweep_tool`, `ffi.rs:231`). Cantos: meia-esquadria = `Common` com meio espaço na bissetriz; Aparar/Encaixe =
  `clip_until` (`surfaces.inc:594`) contra o outro membro (com `offset_surface` da folga). Dobra de tubo: arco por
  `arco_do_no`, modos V, gomos (`st_dobra*.rs`). **O encaixe sólido é booleano; a curva de corte normal à parede
  (laser) existe só como cálculo de fabricação** (`st_encaixe.rs`, B155) e não dirige o sólido (B574).
- **Saídas.** Peças separadas no corpo (`add_separate` não funde); itens (`log_struct`) para lista de corte
  (`cut_list`, `lib.rs:8190`, `CutRow` com ângulos de ponta, encaixe, tubo dobrado), `MemberFailure` (marca no 3D),
  caminhos de laser (`st_caminho_tubo.rs`), DSTV/IFC (`st_dstv.rs`, `st_ifc.rs`). Dependentes: placa, reforço, solda,
  cantos, montagem de estrutura (`fenkra-montagem/src/estrutura.rs`).
- **Cache.** Assinatura inclui as linhas de **todos** os outros membros (`lib.rs:7134–7146`): mexer num membro
  recalcula os demais.
- **Problemas.** B490 (grupo/coletor com subárvore e troca de família), B575 (tubo curvo, spline, cadeia de guias;
  tabela de dobras CNC), B574 (encaixe normal e nó com vários tubos), B576 (guia como Extrair, não Linha).

### 2.2 Cantos / `StructJoint` (`doc.rs:2050`; janela `st_cantos_ui.rs`; motor `lib.rs:3381–3410`, `st_cantos.rs`)

- **Entradas.** Par de linhas que dividem uma ponta (`shared_end`, `t_junction`, `lib.rs:7712–7727`); regra
  em `StructMember.joints` do comando posterior (`mine`, `other`, `other_path`, `mode`, `first`, `gap`, `bend`).
- **Painel.** Dentro do membro ("Cantos com outros membros") e janela "Cantos da estrutura" (classes por cor,
  modo automático, "Limpar tudo"). Opções: Nenhum, Meia-esquadria, Aparar, Encaixe, Dobra de tubo (só tubo do mesmo
  comando). Sem inverter além de "este/o outro" continua.
- **Validação.** T sem Aparar/Encaixe vira aviso, não erro. **Motor.** Ver 2.1 (`joint_rule`). Sem prévia
  própria. Problema: B574 (vários tubos no nó).

### 2.3 Placa de ponta (`StructPlate`, `doc.rs:2196`; painel `st_placas_ui.rs:44`; motor `st_placas.rs:127`)

- **Entradas.** Ponto na face da ponta (`at`); âncora `PontaMembro{membro, trecho, fim}` (`ancoras.rs:93`); clique na
  face (`wants_faces`) ou "Usar o vértice selecionado". Sem caixa de seleção.
- **Painel.** "Placa de extremidade": rótulo da ponta, Sentido (Externo/Interno), Recuo da face (interno),
  Espessura, Sobra (mm ou razão da parede), "Furos de fixação" (Diâmetro, Furo até a borda).
  Inverter: o rádio Externo/Interno. Faltam: Material, formato (a placa é sempre retângulo do tamanho da
  caixa da seção mais a sobra, também em tubo redondo), furos em padrão, contorno por sketch.
- **Prévia.** Leve por zona (`previa_leve.rs:404`). **Validação.** "a espessura da placa precisa ser maior que 0",
  "o perfil é maciço e não tem parede: dê a sobra em mm", "o furo sai da placa", "os furos se tocam ou ficam fora".
- **Motor.** Rust `struct_plate`: `Shape::face` (retângulo + 4 círculos) + `extrude`. **Saída.** `add_separate`
  + item "Placa" na lista (material em branco, `log_struct`, `st_placas.rs:177`). Não entra na lista de corte.

### 2.4 Esquadro / reforço (`StructGusset`, `doc.rs:2263`; painel `st_placas_ui.rs:92`; motor `st_placas.rs:189`)

- **Entradas.** Ponto do encontro (`at`); só vira âncora se for ponta de membro (`membro_ancorado` só no ramo
  `true`, senão `member_pair` por proximidade, `lib.rs:3874`); clique na aresta de encontro ou "Usar a aresta selecionada".
- **Painel.** "Reforço", "Abas" (D1, "Aba D2 diferente da D1", "Inverter D1 e D2", Espessura), "Perfil do reforço"
  (Triangular, Chanfrado, Poligonal + Chanfro de alívio e vértices), "Posição ao longo da aresta" (Interno/Dois
  lados/Externo, Início/Meio/Fim, Afastamento, Repetição por quantidade ou espaçamento).
- **Prévia.** Leve por zona. **Validação.** "membros colineares não levam reforço", "o ponto precisa ser o encontro
  de dois membros", D1/D2 ≤ 0, quantidade inteira, faixas fora da aresta.
- **Motor.** Rust `struct_gusset`: `poly_prism` do perfil em (u, v) ao longo dos dois membros, extrudado pela
  espessura nas faixas calculadas (`faixas_do_reforco`); `add_separate`. **Saída.** Item "Reforço" (sem material).

### 2.5 Solda (`StructWeld`, `doc.rs:2350`; painel `st_solda_ui.rs:84`; motor `st_solda.rs:307`)

- **Entradas.** Dois conjuntos de faces (`faces` pontos + `face_keys: Vec<Option<FaceKey>>`, `split`); lista própria
  `lista_de_faces` (`st_solda_ui.rs:65`), clique soma, Shift+clique tira; "Propagar tangentes". É o único comando
  estrutural com chave de face v2.
- **Painel.** "Conjuntos de faces" (alternar conjunto 1 e 2, Limpar), "Caminhos" (comprimentos, realce ao passar o
  mouse), "Cordão" (Perna, Plano/Convexo/Côncavo, Cosmético/Sólido, Contínuo / Trecho e vão / Trecho e passo,
  Alternado, "Terminar em um plano" e "Manter o lado para onde a normal aponta"). Sem inverter padrão.
- **Prévia.** Leve por zona (caixa das faces escolhidas inflada pela perna); caminhos no 3D por `overlay`
  (`st_solda_ui.rs:193`), lidos do cache do cálculo.
- **Validação.** Sem faces: aviso "escolha as faces..."; "a perna do filete precisa ser maior que 0"; "os dois
  conjuntos de faces não se encontram: ... a cerca de X mm" (`st_solda.rs:~345`); "o plano de término deixou o cordão
  sem nenhum caminho"; "nenhum trecho cabe no caminho".
- **Motor.** Rust `struct_weld`: `Shape::section(conjunto1, conjunto2)` (kernel `section_pts`, `ffi.rs:316`) dá os
  caminhos; pernas por normais (`pernas`); sólido por malha fechada com esquadria (`malha_do_cordao`,
  `Shape::from_mesh`). Cosmético = só caminhos (símbolo no desenho, `weld_beads`).
- **Saídas.** `CordaoCalculado` (comprimento, trechos, volume), itens "Cordão zN", material do metal de adição.

### 2.6 Lista de corte (`cut_list`, `lib.rs:8190`; janela `st_ui.rs:405`)

Não é comando da árvore: lê os membros calculados. Agrupa por perfil e comprimento (`structural_groups`,
`lib.rs:8077`), traz ângulos das duas pontas, encaixe, barras de matéria-prima, tubo dobrado (comprimento
desenvolvido, `st_dobra.rs`); exporta NC, IFC, DSTV (`st_ui.rs:640–658`). Placa, reforço e solda entram como itens
do `log_struct`, mas sem comprimento de corte. Falta: tabela de dobras do tubo para dobradeira CNC (B575).

## 3. Prévia leve: quem tem

Fonte: `previa_leve.rs:44–113`. **Leve por zona** (só se for o último comando ativo): `SmFlange`, `SmHem`,
`SmJoggle`, `SmBend`, `SmLouver`, `SmStamp`, `SmFlangedHole`, `SmProfileFlange`, `SmFlangeLoop`, `SmCornerRelief`,
`SmCornerClose`, `SmLoftFlange`, `SmConvert`, `SmArcSlot`, `StructMember/Plate/Gusset/Weld`. **Leve por dentro**
(faz só a ferramenta, pula colisão e booleana): `SmFlange`, `SmHem`, `SmBend`. **Prévia completa:** `SmWall`, `SmCone`,
`SmCurveFlange`, `SmSurfFlange`, `SmCut`, `SmUnfoldPartial`. Nenhuma das prévias completas é cacheada entre quadros; a
do `SmSurfFlange` refaz suporte, OML e limites a cada edição (B564).

## 4. Problemas conhecidos (backlog de 21/11, só leitura de `scripts/benchmark/backlog.py`)

| ID | Linha | Resumo | Onde bate |
|---|---|---|---|
| B558 | 1134 | Parede base com spline fechada sai como fita | 1.1 |
| B559 | 1136 | Spline vira trechos retos | 1.1, 1.3, 1.6 |
| B560 | 1138 | Aresta, curva e laço viram um modo só | 1.2, 1.3, 1.4 |
| B563 | 1144 | Aba por superfície sem posição, inversão e contato | 1.6 |
| B564 | 1146 | Prévia lenta da aba por superfície | 1.6, seção 3 |
| B566 | 1150 | Dobrado e planificado juntos, com cache | A2, 1.6, 1.16 |
| B565 / B569 | 1148 / 1156 | Contrato de feature e estudo entre bancadas | este documento |
| B490 | 998 | Membro como coletor com subárvore | 2.1 |
| B574 | 1166 | Encaixe de tubo com corte normal e nós de vários tubos | 2.1, 2.2 |
| B575 | 1168 | Tubo curvo, spline, cadeia de guias, tabela de dobras | 2.1, 2.6 |
| B576 | 1170 | Extrair continua sendo Extrair | 2.1 e toda entrada por curva |

## 5. Tabela de paridade: família da aba

`Sim` = existe com campo próprio; `—` = não existe; `fixo` = valor fixo no código; `auto` = automático.

| Opção | Aresta | Curva | Laço | Perfil | Superfície | Bainha |
|---|---|---|---|---|---|---|
| Suporte pelo campo padrão (lista, chave) | Sim | Sim | Sim (V) | `sketch_pick` | `surf_slot` + face só texto | texto da aresta |
| Comprimento | Sim | Sim | Sim | Sim (lado 1 e 2) | Sim (EOP) | Sim |
| Ângulo | Sim | Sim (vazio = 90) | Sim | — (vem do desenho) | só no tipo Ângulo | fixo por tipo |
| Raio próprio | Sim | Sim | Sim | Sim | Sim | por folga |
| Cadeado do raio (estilo) | Sim | — | Sim | — | — | — |
| Lado do material (0/1/2) | Sim | Sim | Sim | — | **fixo 0** | — |
| Inverter direção | Sim | Sim | Sim | Sim | Sim | Sim |
| Recuo início e fim | Sim | — | via Extensão | — | — | — |
| Extensão lateral (4 modos) | — | — | Sim | — | — | — |
| Alívio de canto e tipo | Sim | — | Sim (só reta) | — | — | — |
| Meia-esquadria | Sim | — | Sim (só reta) | — | — | — |
| Recuo mínimo (anti-sobreposição) | Sim | — | fixo | — | — | — |
| Seguir tangentes | auto (vira curva) | Sim | auto | — | — | — |
| Várias arestas | — | — | Sim | — | — | — |
| Limites lado 1, lado 2, borda final | — | — | — | — | **Sim** | — |
| Cantos filete/chanfro | — | — | — | — | **Sim** | — |
| Tipos (exato, aproximação, ângulo) | — | — | — | — | **Sim** | gota/enrolada |
| Alças 3D | ângulo, raio, 2 recuos | — | — | — | — | folga |
| Prévia | leve por dentro | completa | leve por zona | leve por zona | completa | leve por dentro |
| Planificada própria | tira por prisma | faixa extrudada | por aresta | `aba_perfil_planificada` | **sem ramo próprio** (B566) | tira |
| Ancoragem estável (chave v2) | `edge_key` | `edge_key` (uma aresta) | `edge_keys` | sketch | só ponto da face | `edge_key` |

Outros irmãos de chapa:

| Opção | Dobra por linha | Joggle | Veneziana | Estampo | Recorte | Furo flangeado |
|---|---|---|---|---|---|---|
| Linha por sketch | Sim | — | — | Sim (curvas) | — | — |
| Linha por plano | — | Sim | — | — | — | — |
| Posição da linha (centro/início/fim/vértice) | Sim | — | — | — | — | — |
| Inverter direção | Sim | Sim | Sim | Sim (menos tipo 3) | Sim | Sim |
| Segundo inverter ("para baixo") | Sim | — | Sim | — | — | — |
| Raio próprio | Sim | — | — | — | — | — |
| Alças 3D | Sim | Sim | Sim (largura) | — | — | — |
| Âncora de linha (B516) | Sim | Sim | Sim | — | — | pontos soltos |
| Prévia | leve por dentro | zona | zona | zona | completa | zona |

Estrutural, membro × placa × esquadro × solda:

| Opção | Membro | Placa | Esquadro | Solda |
|---|---|---|---|---|
| Seleção | caixa de curvas (V) | face da ponta: texto + clique | aresta de encontro: texto + clique | lista própria de faces (2 conjuntos) |
| Referência guardada | `CurveRef` + promoção | `Ancora PontaMembro` | ponto (âncora só se ponta) | `FaceKey` por face |
| Inverter | "espelhar seção" (caixa) | Externo/Interno (rádio) | "Inverter D1 e D2" (caixa) | "manter o lado da normal" (caixa) |
| Material próprio | Sim | — | — | Sim |
| Posição ao longo/lado | âncora da seção, folga | Recuo (só interna) | início/meio/fim + lado | termina em plano |
| Repetição | — | — | quantidade, espaçamento | contínuo, trecho e vão, alternado |
| Perfil próprio | seções do catálogo | retângulo fixo | triangular, chanfrado, poligonal | plano, convexo, côncavo |
| Furos | — | 4 furos | — | — |
| Prévia | caixa orientada | zona | zona | zona + caminhos |
| Saída na lista de corte | Sim (barra, ângulos) | item sem comprimento | item sem comprimento | item com comprimento do cordão |
| Botão padrão de inverter (`chapa_unif_ui`) | — | — | — | — |

## 6. As 10 melhorias mais importantes, em ordem

1. **Desfazer parcial na falha (A1).** `run` guarda `bodies`, `sheet_maps`, `previas` e `last_sheet` antes do comando e
   restaura em `Err`. Evita corpo cortado e dobra fantasma quando a aba falha. Barato e vale para todas as bancadas.
2. **Aba por superfície no padrão da aba (B563).** Lado do material/posição (face interna, externa, começo do
   raio) no lugar do `0` fixo (`lib.rs:6133`), campo de face com `FaceKey`, cadeado do raio, e o `flip` passando
   só por um caminho (hoje `want` ignora `flip`, `lib.rs:4225`).
3. **Um modo "Arestas" (B560).** Aresta, curva e laço num só `FeatureKind` ou numa função única de aba, com
   recuos, alívio, mitra e extensão valendo também nos trechos curvos (hoje descartados em `chapa_laco.rs:429`).
4. **OML como curva exata (B559, B558).** Levar a interseção suporte × plano da web como BSpline ao `sm_chain_sweep`,
   em vez de ~200 pontos refeitos por heurística; mesma garantia de "uma aresta" para a spline do sketch na
   parede base.
5. **Dobrado e planificado juntos, com cache (B566, A2).** Cada `FeatEntry` guarda os dois corpos; `flat` sai da
   assinatura global; `SmSurfFlange` ganha ramo planificado próprio; validação antes ("vai planificar?").
6. **Contrato de campo de seleção único (B561).** Aba de superfície, bainha, joggle, conversão, cantos, placa e
   esquadro passam a usar o campo de lista padrão com chave v2 (hoje: texto com coordenada e botão).
7. **Botão de inverter único (B562).** Substituir "espelhar seção", Externo/Interno, "Inverter D1 e D2" e "manter o
   lado da normal" por `inverter`; acrescentar na aba entre perfis e nos cantos.
8. **Prévia leve única e cacheada.** Uma só regra (`previa_leve.rs`) cobrindo `SmCurveFlange`, `SmSurfFlange`, `SmCut`
   e parede base com malha grossa e cache da superfície de suporte entre quadros (B564; meta de 300 ms).
9. **Membro como coletor e cadeia de guias (B490, B575, B576).** Um membro com várias guias unidas em cadeia (tubo
   dobrado, spline), coletor com subárvore e troca de família; guia aceita `Extrair` sem converter; tabela de dobras
   CNC na lista de corte.
10. **Encaixe normal à parede no sólido e nó de vários tubos (B574).** Fazer `clip_until` seguir a curva de
    `st_encaixe.rs` (corte normal), com referência externa, interna ou meio e conferência de vão e interferência.
    Em seguida, alças 3D e a mesma estrutura de painel (Suporte, Modo, Parâmetros, Avançado) em placa, esquadro e
    solda.
