# Estado atual dos comandos: Superfícies/wireframe e Sketch (B565 e B569)

Levantamento de 21/11/2026, só de leitura do código (develop, depois do commit 2329e69). Serve de base
para o contrato único de feature (B565), para a padronização de irmãos (B569) e para a revisão do sketch
(B573). Não repete a auditoria dos campos de seleção e do inverter: veja
[selecao-auditoria.md](../../benchmark/detalhe/selecao-auditoria.md) (tabelas de campos, widgets e inverter).
Referências no formato `arquivo:linha`; arquivos sem pasta estão em `crates/fenkra-app/src/` (app),
`crates/fenkra-solidos/src/` (motor), `crates/fenkra-core/src/` (modelo) e `crates/fenkra-kernel/cpp/`.

## 1. Como um comando de Superfícies vive hoje (arquitetura comum)

**Modelo de dados.** Cada comando de Superfícies/wireframe é uma variante de `RefKind` (`doc.rs:613`), guardada
como `RefElement` em `doc.references` (não é `FeatureKind`). Entradas são enums de referência:
`CurveRef` (`doc.rs:146`: eixo da origem, `Ref(id)`, linha/curva de sketch, `Edge(Vec<pts>)` polilinha fixa,
`BodyEdge{pts,key}` aresta com chave ADR 0010, `Segment`), `SurfRef` (`doc.rs:229`: `Plane`, `Face{point,normal,owner,key}`,
`Ref(id)`), `PointRef`, `AxisRef`, `PlaneRef`. Campo ainda vazio é um id sentinela `UNSET_*`
(`slots.rs:19-33`); o cálculo trata como "espera a seleção" e deixa a referência inativa sem erro (B465,
`lib.rs:7431`). O padrão de uma curva solta é `CurveRef::Axis(X)` (`doc.rs:162`).

**Criação e abertura.** `Cmd::Geo(k)` cria `k.make(&seed)` (`app.rs:3084`, `geo_ui.rs:391`), onde `seed` é a
pré-seleção (pontos, curvas, planos, superfície). Abrir/editar usa `start_session` (`app.rs:3226`): guarda um
`snapshot` do documento. **Concluir** só encerra a sessão (`app.rs:3289`); não valida nada (única exceção:
membro estrutural sem linhas). **Cancelar** restaura o snapshot (`app.rs:3322`). Elemento com erro continua na
árvore com status de erro.

**Painel.** Um único `geo_ui::panel` (`geo_ui.rs:1415`, ~1000 linhas num `match` por variante, braços de
`geo_ui.rs:1463` a `2430`) montado dentro de `editor.rs:932` como abas "Tipo", "Definição", "Elemento",
"Nome e exibição", "Relações". Não existe painel flutuante/encaixável próprio nem barra de comando
específica; a barra Concluir/Cancelar/Excluir é a mesma dos recursos (`editor.rs:1229`). "Tipo" troca entre
variantes da mesma classe (B505, `tipos_do_painel`, `geo_ui.rs:1406`) preservando entradas.

**Prévia.** Não há prévia leve para referência: cada edição dispara recálculo completo
(`request_recompute`), e o resultado (`RefViz::Surface(Mesh)`) aparece como a própria superfície; se o valor
atual falha, o app segura a última malha válida (`Session.ultima_valida`, `app.rs:414-422`). A
decoração 3D do comando (cores por papel, normal, setas de lado, bolinhas de solução, regiões) vive em
`sup3d.rs:527` (`decorar`) e `sup3d.rs:609` (`overlay`). `previa_leve::cobre` vale só para `FeatureKind`, nunca
para `RefKind`.

**Manipuladores 3D de valor** (`handles.rs`, `surface_handles` em `handles.rs:58`): só existem para
`SurfOffset` (:229), `SurfExtrude` comprimento 1 e 2 (:234, :410), `SurfCylinder` (:244), `SurfSphere` (:250),
`SurfSweep` com perfil circular (:255), `SurfExtrapolate` sem limite (:263), `SurfFillet` e `SurfEdgeFillet` raio
(:271, :278), `SurfRevolve` ângulo (:646), `SurfFill` condições (:554-630), mais `OffsetPlane`, `LineDir`,
`PointOffset`, `PointOnCurve`. **Sem alça**: Ruled (comprimento), Curva paralela (d1/d2), Blend/Conectar
(tensão no 3D só pelo `sup_loft.rs`), Junção (tolerância), Transformar, Dividir/Aparar (só seta de lado).

**Inverter.** Só a seta clicável do 3D (`sup3d.rs:286` `setas_de_lado`, `:369` `inverter_lado`), existente para
Dividir (`flip`, `more_flip`), Aparar e Filete entre superfícies (`flip_a/b`), Curva paralela, Regrada,
Dividir corpo, Costurar, Espessar. O painel não tem botão: no Filete entre superfícies o painel até descarta os
campos (`geo_ui.rs:1980` `let _ = (flip_a, flip_b)`). Extrusão, revolução, deslocamento, varredura e loft de
superfície não têm inverter (sinal do número é o único meio).

**Motor.** `Ctx::geo` (`lib.rs`, braços de `lib.rs:2304` a `3057`) resolve cada `RefKind` para `Geo::{Point,
Plane, Curve(Curve3), Surface(Shape)}`. `Curve3` (`geo3d.rs:62`) é **sempre uma polilinha densa** (+ círculo
analítico opcional que quase ninguém usa). Superfícies são `Shape` do kernel via fachada cxx
(`fenkra-kernel/src/lib.rs`, `cpp/surfaces.inc`, `cpp/hybrid.inc`, `cpp/features.inc`).

**Cache.** `ref_sig` (`lib.rs:1196`) = JSON da definição + assinatura recursiva das entradas + parâmetros
citados + donos de chaves (B534); `ref_reads_bodies` (`lib.rs:1157`) decide se muda com a linha do tempo dos
corpos. Resultado guardado como `RefViz` (`lib.rs:1143`); o cache de refs é invalidado na troca de qualidade
de malha (`lib.rs:7012`).

**Saída e dependentes.** Superfícies resultantes são `SurfRef::Ref(id)` para quem vier depois. Comando que
consome superfícies inteiras (filete, junção, aparar) esconde as entradas (`sup3d.rs:483`
`entradas_consumidas`). Notas de cálculo (`RefNote`: setas, soluções, bordas, avisos) voltam para o painel
(`editor.rs:733-740`).

## 2. A cadeia curva → kernel (base do B559) e o caso da spline

Fato central: **toda curva de entrada de superfície chega ao kernel como lista de pontos**.

| Origem da curva | Como vira pontos | Onde |
|---|---|---|
| Sketch (linha/arco/círculo/spline/elipse), via `CurveRef::SketchCurve` | `profile::sample`: círculo 64 segmentos, arco 24, elipse amostrada, spline `spline_samples` (Catmull-Rom centrípeta, 12 por trecho) | `profile.rs:111-150`, `:150-190`; `lib.rs:1664` |
| Referência Spline (wireframe) | `geo3d::spline_tan`/`spline`: Catmull-Rom centrípeta ou Hermite com tangentes, 24 amostras por vão | `geo3d.rs:382`, `:414`; `lib.rs:2349` |
| Spline por controle | B-spline cúbica uniforme amostrada 24 por trecho | `lib.rs:1262` |
| Aresta do modelo (`Edge`/`BodyEdge`/`CurveExtract`/`CurveBoundary`) | `edge_at`: `GCPnts_TangentialDeflection(c, 0.05, 0.05, 2)` | `cpp/hybrid.inc:662-681`; `lib.rs:1448` |
| Conectar/Canto/Hélice/Cônica/Espiral | calculadas em Rust como polilinha (Hermite cúbica, quíntica ou sétima) | `geo3d.rs:448-520`; `lib.rs:2368` |

No kernel, `curve_wire` = `spine_wire` (`features.inc:62`): `guia_suave` (`features.inc:45`) ajusta **uma**
`Geom_BSplineCurve` por `GeomAPI_PointsToBSpline(grau 3 a 8, C2, tol 1e-4)` **só se nenhum canto passar de
0,35 rad (20°)**; qualquer canto maior (ou apenas 2 pontos) faz a curva inteira virar **N segmentos
retos** (`features.inc:62-73`). Isso vale para `extrude_curve`, `revolve_curve`, `curve_shape`, `fill_surface`,
`sweep_surface`, `loft_surface` (cada seção).

Consequências factuais (candidatos para o B559):
1. Spline de sketch com um ponto de curvatura forte (>20° entre amostras vizinhas) vira polilinha inteira; a
   superfície extrudada/varrida sai facetada. Com a curva suave vira uma BSpline aproximada (não passa
   exatamente pelos pontos de controle do usuário, tol 1e-4 sobre amostras que já são aproximação).
2. Círculo e arco de sketch usados como curva de superfície viram 64/24 pontos e depois BSpline
   aproximada: a extrusão **não é um cilindro** exato, o que atrapalha detecção de furo/rosca e deixa a malha de
   exibição dependente do ajuste. `Curve3.circle` (analítico) existe mas `SurfExtrude`/`SurfRevolve`
   (`lib.rs:2612`, `:2641`) usam só `c.pts`.
3. **Três definições diferentes da mesma spline de sketch**: (a) solver, aparar/estender e arrasto usam
   `spline_eval` Hermite com `spline_tangents` (`sketch.rs:357-381`, `sketch_relimita.rs:166`); (b) desenho, prévia
   da ferramenta e `sample()` usam `spline_samples` centrípeta (`sketch.rs:307`, `sketch_ui.rs:2116`,
   `profile.rs:154`); (c) perfil de sólido sem tangentes passa os próprios pontos e o kernel interpola com
   `GeomAPI_Interpolate(arr, periodic=false, 1e-7)` (`cpp/kernel.cpp:364-375`, parametrização padrão do
   kernel). Pontos de corte do Aparar vêm de (a) e não pertencem exatamente à curva (c) do sólido.
4. `edge_at` devolve polilinha com desvio angular 0,05 rad: aresta curva extraída/clicada e reutilizada como
   guia/perfil perde a geometria exata. Contradiz a regra B576 (extraído tem de manter o tipo geométrico).
5. Derivadas de continuidade G1/G2/G3 (Conectar, Blend, Preencher com apoio) saem de `end_tangent`/
   `end_curvature`/`end_jerk` calculadas sobre a polilinha (`geo3d.rs:144-190`); com polilinha esparsa ou aresta
   de `edge_at` o resultado é grosseiro. Blend ainda reamostra cada borda em N=24 pontos (`lib.rs:1785-1789`) e
   monta remendos Bézier (`cpp/surfaces.inc:375`), então a continuidade nominal G2 é da amostra, não da curva.
6. A malha de exibição vem de `mesh_angle(desvio, ângulo)` (`lib.rs:861`) com Normal = desvio 0,15 % da
   diagonal e 20° (`fenkra-core/src/qualidade.rs:21-23`): faces curvas ficam visivelmente faceteadas em peças
   pequenas diante da diagonal e em superfícies de curvatura variável.

## 3. Comandos de Superfícies (um a um)

Legenda das colunas das tabelas: **Ent.** entradas; **Painel** seções/opções; **Dir.** direção/inverter/limites/
continuidade; **3D** prévia e manipuladores; **Motor** função Rust e kernel.

### 3.1 Resumo em tabela

| Comando (`RefKind`) | Ent. | Painel (`geo_ui.rs`) | Dir./limites | 3D (manipulador/seta) | Motor |
|---|---|---|---|---|---|
| Extrair face (`SurfExtract`) :1775 | face (ponto+chave) | face pelo nome, propagação, complementar | – | – | `extract_faces` `lib.rs:1899`; `face_propagate` `hybrid.inc:732` |
| Extrair vários (`SurfMulti`) :2281 | N faces | lista de faces | – | – | `lib.rs:2850`; `face_of_bodies` |
| Deslocada (`SurfOffset`) :1849 | 1 sup | distância, dois lados, ligação | sinal da distância | alça de distância, seta da normal | `offset_surface_j` `surfaces.inc:193` |
| Preencher (`SurfFill`) :2005 | N curvas + pontos | curva a curva com condição G0/G1/G2 e peso, "Aplicar a todas", pontos, tolerância | continuidade por borda | alças de condição | `fill` `lib.rs:2031`; `fill_surface_x` `surfaces.inc:289` |
| Extrudada (`SurfExtrude`) :2042 | 1 curva, 1 direção | curva, direção, limite 1, limite 2 (comprimento ou "até superfície") | sinal do comprimento; sem inverter | alça L1/L2 | `Shape::extrude_curve` `surfaces.inc:457` |
| Revolução (`SurfRevolve`) :2049 | 1 curva, 1 eixo | curva, eixo, ângulo 1, ângulo 2 | sem inverter | alça de ângulo | `revolve_curve` `surfaces.inc:466` |
| Esfera / Cilindro :2055/:2059 | ponto, eixo | raio, comprimentos | início/fim | alças | `sphere_surface`, `cylinder_surface` `hybrid.inc:98-107` |
| Varredura (`SurfSweep`) :2067 | perfil (curva ou círculo), guia | perfil, guia, modo (corrigido, fixo, Frenet) | – | alça do raio do tubo | `sweep_surface` `hybrid.inc:155`; `MakePipeShell` |
| Varredura avançada (`SurfSweepX`) :1875 | 1 a 3 guias, perfil | tipo (8), spine, solução, suavizar, seções | solução por bolinha | bolinhas de solução | `sweep_x` `lib.rs:3661`; `varredura_guias.rs` (cônica G2) |
| Multi-seções (`SurfLoft`) :2082 | N seções, guias, caminho | seções, guias, caminho, regrada, fechar, conectores, tangência | sentido por seção | `sup_loft.rs` (setas, conectores) | `loft::superficie` (Gordon) ou `loft_surface` `hybrid.inc:177` |
| Regrada (`SurfRuled`) :2101 | N curvas, apoio | curvas, modo (normal, tangente, vetor), apoio ou direção, comprimento | seta de faixa | seta | `regrada` `lib.rs:438`; `sewn` |
| Concordância (`SurfBlend`) :2139 | 2 bordas + 2 apoios | bordas, apoios, G0/G1/G2, tensão 1 e 2 | – | `sup_loft.rs` (tensão) | `blend` `lib.rs:1785`; `bezier_faces` |
| Conectar (`SurfConnect`) :1859 | 2 sup | sup 1 e 2, continuidade, tensões, "voltar às bordas" | bolinha troca borda | bolinhas | `bordas_livres` + `blend` |
| Junção (`SurfJoin`) :2166 | N sup + curvas | lista, tolerância | – | relatório | `sewn` `hybrid.inc:43` |
| Curar / Desaparar / Parte :2172/:2306/:2273 | 1 sup | tolerância / índice | – | – | `healed`, `untrim_face`, `nth_face` |
| Dividir (`SurfSplit`) :2177 | sup, N ferramentas, curva | ferramentas + "+", curva de corte | `flip` por corte | seta clicável por corte | `split_side` `hybrid.inc:402` (`BRepAlgoAPI_Splitter`) |
| Aparar (`SurfTrim`) :2208 | 2 sup | sup 1 e 2 | `flip_a/b` | 2 setas | `split_side` ×2 + `sewn` |
| Regiões (`SurfRegions`) :2213 | sup, ferramentas, curva | modo mútuo, remover, estender, pedaços | marcas "Fica/Sai" | caixas clicáveis | `regioes.rs:137`; `split_regions` `hybrid.inc:445` |
| Extrapolar (`SurfExtrapolate`) :2307 | sup, pontos/bordas, limite | comprimento ou "até", curvatura, propagação tangente, normal à borda | – | alça de comprimento | `extrapolate_face` `hybrid.inc:532` |
| Filete entre sup. (`SurfFillet`) :1979 | 2 sup | sup 1 e 2, raio (e raio final) | seta; painel descarta | 2 setas, alça de raio | `surf_fillet` `hybrid.inc:1083` |
| Filete de aresta (`SurfEdgeFillet`) :1987 | sup + arestas (pontos) | "usar seleção", somar, limpar, raio | – | alça de raio | `edge_fillet` `lib.rs:539`; `filete_na_junta` |
| Contorno (`CurveBoundary`) :1794 | sup | número ou aresta clicada, propagação | bolinha de solução | bolinha | `boundary_pts`; `lib.rs:3034` |
| Transformar (`GeoTransform`) :2331 | 1 elemento qualquer | mover, girar, espelhar, escalar | números | – | `xform` `lib.rs:1764`; `affinity` |
| Análises (`SurfCheck`) :2119 | 2 sup ou 1 curva | conexão, desvio, pente | – | cores/etiquetas `sup_analise.rs` | `connect_check`, `signed_distances` |

### 3.2 Notas por comando (o que importa para o contrato)

**Extrair (face, várias, contorno, curva).**
- Entrada: ponto clicado + chave v2 (`FaceKey`, ADR 0010); dono gravado; `expected` (nº de faces) vira aviso se
  mudar (`lib.rs:2531`). `ref_sig` inclui o dono (B534).
- Painel: nome da face, "Usar a face selecionada", propagação (nenhuma/ponto/tangente/completo), complementar.
  Falta: caixa de seleção em árvore (só botão), pré-visualização da propagação antes do OK.
- Regra B576: a face extraída continua `SurfExtract` (correto). A aresta extraída vira `CurveExtract` e a curva de
  sketch extraída vira `CurveCopy` (`app.rs:2029`); mas o **resultado é `Curve3` polilinha** (`lib.rs:2515-2526`,
  `edge_extract` `lib.rs:1996`): mantém o recurso Extrair, perde o tipo geométrico (linha/arco/spline exatos). O
  eixo extraído de aresta reta também vira `CurveExtract` (`app.rs:2067`).
- Falha: erro de texto no painel; `expected` diferente só avisa.

**Deslocada.** Entrada 1 `surf_slot`. Painel: distância, "dos dois lados", ligação (automática/arredondada/
cantos vivos). Sem botão inverter (sinal). Motor desloca o **casco costurado** (`casco_costurado`, tolerância
1e-4) com `confere_raio` antes do OCCT (recusa se a distância passa do raio de curvatura e informa o raio
mínimo, `surfaces.inc:106-120`) e `BRepOffsetAPI_MakeOffsetShape::PerformByJoin`. A seta mostra a normal do
casco (`RefNote.arrow`, `lib.rs:2548`). É o comando mais completo em validação e mensagens de erro.

**Preencher.** `curves_list`-like próprio (cada curva com sua condição; "+ curva" empurra
`CurveRef::Axis(X)` como valor real, `geo_ui.rs:2030`, **não** o sentinela UNSET: pode criar uma curva válida e
errada). Motor: `BRepOffsetAPI_MakeFilling` (graus 3, iterações 15 ou 30 com tangência, `fill_surface_x`) com arestas de apoio
detectadas por `support_edges`, que só vale se cobrirem 98 % da curva (`surfaces.inc:262-285`). Erros do construtor
(G0 em mm, G1 em rad) voltam por `errs`. Cadeia automática só dentro da mesma origem (`App::chained`,
`app.rs:2312`).

**Extrudada.** Dois limites (comprimento ou até superfície) e direção por eixo. Diferente da extrusão sólida
(`doc.rs:4066`): faltam inverter, simétrico (plano médio), ângulo de saída, início deslocado, deslocamento do término,
término "até o próximo/último/plano", "direção padrão pela normal do plano da curva" e tampas. O "até superfície"
estende um prisma de `model_size()` e corta por `clip_surface` (`lib.rs:2612-2640`). Limite 1 zero e 2 zero dá
erro "os dois limites são zero". Entrada: **uma só curva** (sem cadeia de curvas tangentes de origens diferentes,
B561); contorno de várias curvas exige `CurveJoin` manual.

**Revolução.** Mesmos limites de irmão que a extrusão: sem inverter, sem "simétrico", sem eixo por clique em
aresta pelo mesmo widget da Revolução sólida (que usa combo próprio `editor.rs:1608`). Soma de ângulos > 360°
dá erro (`lib.rs:2641-2652`); ângulo 2 gira a curva para trás antes de revolver.

**Varredura (simples e avançada).** Perfil e guia 1 curva cada; `SurfSweep` recusa guia com triedro instável
(Frenet vira "corrigido" no círculo, B386). Guia passa por `spine_wire` (suave = BSpline C2; cantos >20° =
polilinha). `SurfSweepX` calcula soluções múltiplas e publica bolinhas (`publicar_solucoes`, `lib.rs:2572`). Sem
inverter, sem "torção"/"escala" ao longo da guia, sem seleção de perfil por cadeia.

**Multi-seções.** O mais rico em manipulação 3D (`sup_loft.rs`): ordem das seções, sentido por seção, conectores,
tangência nas pontas. Dois caminhos de motor: com controle (guias, conectores, reversão, tangente) usa
`loft::superficie` (soma de Gordon `S = Sv + Su − Suv`, `loft.rs`); sem controle usa `loft_surface`
(`BRepOffsetAPI_ThruSections(solid=false)`, `hybrid.inc:177-190`), cuja torção depende do ponto de partida
escolhido pelo kernel. Painel sem lista ▲▼ comparável à do Loft sólido, e o painel ainda mistura guias, caminho
e regrada no mesmo bloco.

**Regrada, Concordância, Conectar.** Regrada soma faixas por curva e costura (`sewn(1e-3)`): sem botão de
inverter (só seta). Concordância/Conectar compartilham `blend` com N=24 amostras e remendos Bézier; "Conectar"
escolhe as bordas por `escolher_borda` e expõe bolinhas para trocar.

**Dividir, Aparar, Regiões, Filete.** Família "cortar": `split_side` por `BRepAlgoAPI_Splitter`, mantendo o
lado da normal da ferramenta (`keep_pos`), erros claros ("nada sobra desse lado — inverta o lado",
`hybrid.inc:431`). Plano vira face grande `model_size()` (`lib.rs:1741`). O lado é inversível só pela seta do 3D.
Regiões guarda âncora espacial + lado (`regioes.rs:1-12`).

**Extrapolar.** Seleção de borda por ponto próximo ou arestas clicadas; `tangent_prop` usa `tangent_chain`
(`geo3d.rs:952`); alça só sem `until`. Falta caixa de seleção em árvore e inverter (direção para fora é
automática).

**Transformar / Análises / Planos e pontos.** Sem painel de "elemento a transformar" por caixa; análises escrevem
`RefNote` e o resultado é o ponto do pior caso (`Geo::Point`, `lib.rs:2744-2776`).

### 3.3 Planos, pontos, linhas e curvas (wireframe)

| Grupo | Variantes (`doc.rs`) | Entrada principal | Observações de estado |
|---|---|---|---|
| Planos (14, `PLANES`) | OffsetPlane, AnglePlane, FacePlane, PlaneParallelPoint, PlaneThreePoints, AxisSystem, PlaneTwoLines, PlanePointLine, PlaneCurve, PlaneNormalCurve, PlaneTangentSurface, PlaneEquation, PlaneMeanPoints, PlanePointNormal, PlaneMid | `plane_slot`, `point_slot`, `curve_slot` | Só `OffsetPlane` tem alça (`handles.rs:169`); `FacePlane` é "fixo nesta versão" no painel (`geo_ui.rs:1463`) mas aceita chave v2; plano médio e equação são números |
| Pontos (8, `POINTS`) | PointCoords, OnCurve, OnPlane, OnSurface, Center, Tangent, Between, Offset (+ interseções e projeções em outras famílias) | slots | `PointOnSurface` agora parte do centroide da face (B524); `PointTangent` e `CurveIntersect` escolhem solução por bolinha |
| Linhas (9, `LINES`) | LinePoints, LineDir, LineAngle, LineNormalSurface, LinePlanes, LineBisector, LineAxis/AxisLine, Polyline | slots; Polyline usa `points_list` | `LineAxis` mantém ponto clicado (sem chave) |
| Curvas (11, `CURVES`) | Circle, Circle3, Conic, Spline, SplineControl, Connect, Corner, Helix, Spiral, Spine, CurveParallel | slots e listas | todas viram `Curve3` polilinha (seção 2) |
| Extrações (4) / Junções (3) | CurveCopy/CurveExtract, SurfExtract, SurfMulti, CurveBoundary; CurveJoin, SurfJoin, SurfHeal | clique/slots | `CurveJoin` (`lib.rs:2527`, `join_curves` `lib.rs:550`) exige pontas que se tocam; a ordem não importa |
| Projeções (4) | CurveProject (plano), CurveProjectSurf (normal ou direção), PointProject, PointProjectCurve | slots | `CurveProjectSurf` (`lib.rs:2926`) projeta polilinha ponto a ponto |
| Intersecções (3) | PointCurvePlane, PointCurveCurve, CurveIntersect | slots, `which` + âncora `near` | `GeomAPI_IntSS` (`hybrid.inc:1382`), fallback para segmentos retos quando não passa por todos os vértices (`hybrid.inc:1353`) |

**Spline (referência).** Entrada: lista de pontos de referência (`points_list`, mín. 2), fechada, tangentes por
ponto (`SplineTan`, `doc.rs:699`). Painel: `sup_spline::painel` (pente de curvatura, alças de tangente, polígono)
e overlay no 3D (B291). Motor: `spline_tan` (Hermite sobre derivadas Catmull-Rom centrípeta, 24 amostras/vão,
`geo3d.rs:382`). **Não há `Geom_BSplineCurve` armazenada**: ver seção 2. `SplineControl` é B-spline cúbica
uniforme de nós presos (`lib.rs:1262`), também amostrada.

**Polilinha.** `points_list` mín. 2, fecha; uma `Polyline` com poucas voltas suaves (<20° entre segmentos) vira
BSpline arredondada no kernel (`guia_suave`), o que contradiz a intenção de "polilinha reta".

**Curva paralela (`CurveParallel`, `lib.rs:2971`).** Distância constante ou variável d1→d2, dois lados, vários
trechos; apoio plano/superfície; seta de lado (`sup3d.rs:361`). Sem alça de distância.

## 4. Padrão de entrada, validação e falha (resumo do estado)

- **Validação antes do OK:** nenhuma. O app só mostra a faixa de entradas vazias para `FeatureKind`
  (`entradas::pede_atencao`, `editor.rs:1068`), não para `RefKind`. Erros aparecem como caixa vermelha no topo do
  painel (`geo_ui.rs:1424`) e como etiqueta no 3D (`previa::etiqueta_de_erro`), depois do recálculo.
- **Mensagens:** as do motor são específicas e em português (ex.: "nada sobra desse lado — inverta o lado",
  "passa do menor raio de curvatura (X mm): use menos que Y mm", "a curva escolhida não é borda da superfície"),
  mas algumas são genéricas do kernel ("não foi possível dividir").
- **Quando falha:** a superfície some (ou fica a última válida na prévia); dependentes também falham em cadeia.
  Não há "modo relatório" parcial para referência, exceto a espera por campo vazio (B465).
- **Entradas por cadeia:** só `App::chained` (`app.rs:2312`) para Preencher/Junção/Junção de curvas, dentro da
  mesma origem (sketch, wireframe ou corpo).

## 5. Sketch: estado atual e divergências (base do B573)

### 5.1 Ferramentas e máquina de estados

`Tool` (`sketch_ui.rs:21-61`): Select, Line, Rect, RectCenter, Rect3, Parallelogram, Circle, Circle3,
CircleTan3, Arc3, ArcCenter, Slot, Polygon, Point, Dimension, DimH/V/Angle/Radius/Diameter, Move, Copy, Mirror,
Offset, Fillet, Chamfer, Spline, Trim, Extend, Convert, Intersect, Constr(CKind: Coincident, Horizontal,
Vertical, Parallel, Perpendicular, Tangent, Equal, Concentric, Midpoint, Fixed, Symmetric).

| Ferramenta | Passos (`clicks_needed`, `sketch_ui.rs:138`) | Término do elemento | Preso (duplo clique no ícone) |
|---|---|---|---|
| Linha | 1º clique = ponto inicial, depois 1 clique por segmento; fecha no 1º ponto; Esc, botão direito ou duplo clique encerram (`sketch_ui.rs:3117`) | sem estar presa, **o 1º segmento já termina** (`fim_do_elemento` zera `chain_last`, `sketch_ui.rs:620`) | encadeia até Esc |
| Retângulo, centro, 3P, paralelogramo, círculo, 3P, tangente a 3, arco 3P, arco centro, rasgo, polígono, ponto | N cliques fixos (2 ou 3, ponto 1); `build_shape` no último (`sketch_ui.rs:3200+`, `:1462`) | volta à seleção | repete |
| Spline | 1 clique por ponto; Enter, duplo clique ou botão direito termina (`sketch_ui.rs:3168`) | `criou_elemento` só depois do Enter | repete |
| Cotas (6) e restrições (11) | seleção prévia ou cliques; prévia da cota segue o mouse; clique no vazio posiciona | a restrição ou cota é o elemento | repete |
| Mover, Copiar, Espelhar, Offset, Filete, Chanfro, Aparar, Estender, Converter, Intersecionar | seleção prévia aproveitada (`sketch_fluxo::usa_selecao`) + cliques | **não estão em `de_um_so`** (`preso.rs:59`): ficam ativas até Esc; `repete()` retorna true para tudo exceto Select (`sketch_fluxo.rs:100`) | – |

`SketchEdit` (`sketch_ui.rs:200`) guarda `clicks`, `chain_last/first`, `pending` (cota em prévia), `entry`
(campos de valor junto ao cursor), `fim_pendente`, `preso`, `typed`. A função pura `Preso::ao_concluir`
(`preso.rs:48`) decide `Reabrir` ou `Selecao`; `criou_elemento` (`preso.rs:99`) só conta curva que não é ponto
solto nas ferramentas de forma, porque o 1º clique da Linha já cria o ponto e, sobre uma curva, o vínculo
(B572). Esc: `cancel` (`sketch_ui.rs:682`) desfaz em camadas (origem no 3D, menu, campo de valor, cota em
prévia, pincel, passo, ferramenta, seleção).

**Divergência de convenção:** a Linha sem estar presa termina no 1º segmento, mas o texto "Cada clique cria um
segmento ligado ao anterior" (`sketch_ui.rs:93`) e `repete()` dizem o contrário; em softwares CAD avançados a
linha é polilinha contínua até Esc/duplo clique, e o "preso" seria só para repetir o comando depois de
terminar. A regra B552 inverteu isso para toda a ferramenta de forma.

### 5.2 Encaixes e inferências

`sketch_snap.rs`: `TipoSnap` = Livre, Grade, Origem, Ponta, Centro, Meio, Quadrante, Interseção, SobreCurva
(`:30`); `achar` (`:86`) escolhe o alvo sob o cursor; `vincula` (`:165`) transforma o apoio em restrição quando
"restrições automáticas" está ligado (`settings.auto_constraints`, padrão true); `inferir` (`:288`) dá
paralela/perpendicular/horizontal/vertical com linha-guia (`desenha_inferencia`, `:358`). H e V forçam eixo
(B548), Ctrl desliga encaixe. Campos de valor (comprimento, ângulo) digitáveis junto ao cursor (`ValueEntry`,
`sketch_ui.rs:316`); `dim_on_type` (padrão true) cria cota com o digitado.
Faltam, comparados a um sketch CAD avançado: inferência de extensão/alinhamento com pontos distantes (só
paralela/perpendicular/H/V), tangente/perpendicular automática ao continuar a partir de arco, e rastreio polar com
incrementos configuráveis.

### 5.3 Solver, graus de liberdade, restrições e cotas

- **Solver** (`sketch.rs:1592` `solve_ate`): Levenberg–Marquardt com **Jacobiano numérico denso** (`jacobian`
  `:1461`, `solve_dense` `:1507`, até 100 iterações). Sem decomposição em grupos rígidos nem Jacobiano
  analítico: o custo cresce rápido; por isso há `prazo` (B273) e a prévia de restrição cai para só destacar quando
  estoura (`sketch_barra.rs`). DOF e movimentos livres por espaço nulo/RREF (`rref` `:1535`, `SolveReport.liberdade`
  `:206-222`); setas de DOF no arrasto (`sketch_arraste.rs`).
- **Restrições** (`Kind`, `sketch.rs:54-82`): Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal,
  Tangent, Concentric, Midpoint, PointOnCurve, Fixed, Symmetric, HorizontalPts, VerticalPts. **Ausentes** (não
  achei no `Kind`): colinear (linha sobre linha), curvatura contínua/suave entre spline e curva, mesmo
  comprimento de arco, interseção de ponto com duas curvas como restrição própria, bloquear forma.
- **Cotas:** Distance, DistanceX/Y, Length, Radius, Diameter, Angle, AngleSupp, PointLineDistance, LineAngle;
  `reference` alterna cota de referência (`sketch.rs:196`, `:1084`). Ausentes: comprimento de arco, cota ordenada,
  cotas em cadeia/linha de base, cota simétrica (diâmetro de revolução), distância curva-curva.
- **Origem e eixos** são entidades fixas (ADR 0092, `sketch_origem.rs`); ferramentas de edição filtram `e_eixo`.
- **Splines:** pontos de passagem + tangente própria por ponto (`SplineTan`); alças Bézier (B291), polígono de
  controle C2 (`sketch_controle.rs`, B549), pente e inflexões. Tangência com outra curva só pela restrição
  `Tangent` (`apply_spline_tangency`, `sketch.rs:1900`), após resolver os pontos (não é variável do solver).
- **Perfis:** `chains()` (`sketch.rs:1941`) liga curvas por pontas coincidentes (tolerância 1e-7 relativa);
  `smooth_runs` (`:264`) usa corte de 26° para cantos vivos.

### 5.4 Edição e o que falta

Existem: aparar (clique ou pincel arrastado, B551), estender, offset (com cadeia), filete e chanfro (com
repetição por igualdade e "manter canto"), mover, copiar (cópias repetidas), espelhar (por simetria), converter e
interseccionar (associativos, `sketch_ext.rs`), seleção por janela (esquerda toca/direita contém,
`sketch_ui.rs:2313`), região pela sombra, cadeia por duplo clique.
**Não existem** (busca nos `Tool`): elipse desenhada (só importada via Converter; `Curve::Ellipse` não é
editável), cônica, padrão linear e circular, girar e escalar seleção, esticar (stretch), quebrar/dividir curva,
unir/mesclar curvas, texto, imagem de fundo, bloco, sketch 3D, arco tangente a partir da linha, estender
"até" um elemento escolhido. Espelhar não é dinâmico (cria cópia com simetria, não uma instância viva) e
recusa elipse com flip (`sketch_ui.rs:1671`).

### 5.5 Divergências priorizadas do sketch (entrada do B573)

1. Spline com três definições distintas (seção 2, item 3): o que se vê e o que o solver corta não é o que o
   sólido usa.
2. Linha solta termina no 1º segmento (convenção invertida).
3. Solver numérico denso sem particionamento: sketches grandes perdem a prévia de restrições.
4. Falta de restrições de colinear e de continuidade de spline; falta de cotas de arco.
5. Sem padrões, girar, escalar, elipse desenhada, quebrar.
6. Teste de comportamento por eventos crus só para o 1º clique (`sketch_primeiro_clique_testes.rs`,
   `sketch_ui.rs:4331+`); as demais ferramentas têm testes de função pura, pouco de `overlay`.
7. Seleção no sketch continua com `Ent` próprio, fora da caixa universal (B561).

## 6. Paridade entre comandos irmãos

Coluna "S" = sólido, "Sup" = superfície; ✔ tem, ✖ não tem, ~ parcial.

| Capacidade | Extrusão S (`Extrude`) | Extrusão Sup | Revolução S | Revolução Sup | Engrossar (`ThickSurface`) | Deslocada Sup | Varredura S | Varredura Sup | Loft S | Loft Sup |
|---|---|---|---|---|---|---|---|---|---|---|
| Entrada de perfil por sketch ou curva | sketch | curva | sketch | curva | superfície | superfície | sketch + guia | curva + guia | sketches | curvas |
| Cadeia de curvas de origens diferentes | ✖ | ✖ | ✖ | ✖ | – | – | ✖ | ✖ | ✖ | ✖ |
| Direção por eixo/aresta | ✔ | ✔ | ✔ (combo) | ✔ | ✔ (B536) | normal | – | – | – | – |
| Botão inverter | ✔ (caixa) | ✖ | ✖ | ✖ | seta | ✖ (sinal) | ✖ | ✖ | ✖ | ✖ |
| Simétrico / dois lados | ✔ | ~ (L2) | ✔ | ~ (ângulo 2) | ✔ | ✔ (both) | – | – | – | – |
| Até plano/superfície/próximo/último | ✔ | ~ (só sup.) | ✖ | ✖ | – | – | – | – | – | – |
| Deslocamento de início | ✔ | ✖ | ✖ | ✖ | – | – | – | – | – | – |
| Ângulo de saída | ✔ | ✖ | – | – | – | – | – | – | – | – |
| Parede fina | ✔ | – | ✔ | – | ✔ (é o comando) | – | ✖ | ✖ | ✖ | – |
| Alça 3D de valor | ✔ (`handles/solidos.rs`) | ✔ L1/L2 | ✔ | ✔ | ✖ | ✔ | ✖ | ~ (círculo) | ✖ | `sup_loft.rs` |
| Seta de lado no 3D | ✖ | ✖ | ✖ | ✖ | ✔ | ✔ (normal) | ✖ | ✖ | ✖ | ✔ (sentido) |
| Operação booleana (juntar/cortar) | ✔ | n/a | ✔ | n/a | ✔ | n/a | ✔ | n/a | ✔ | n/a |
| Validação antes do OK | ~ (faixa de entradas) | ✖ | ~ | ✖ | ~ | ✖ | ~ | ✖ | ~ | ✖ |
| Prévia leve | ✔ em alguns | ✖ | ✖ | ✖ | ✖ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Estrutura do painel (seções) | Perfil, Extensão, Direção, Extras | só "Definição" | Perfil, Eixo | só "Definição" | "Engrossar superfície" + lados + direção | só "Definição" | – | – | – | – |
| Extrair equivalente (B576) | – | – | – | – | – | – | – | – | – | – |

| Capacidade (família cortar/cantos) | Dividir | Aparar | Regiões | Filete sup. | Filete aresta sup. | Dividir corpo | Costurar | Espessar |
|---|---|---|---|---|---|---|---|---|
| Seta de lado no 3D | ✔ | ✔ | marcas | ✔ | – | ✔ | ✔ | ✔ |
| Botão no painel para inverter | ✖ | ✖ | ✖ | ✖ (descarta o campo) | – | ✖ | ✖ | ✖ |
| Alça de valor no 3D | – | – | – | ✔ raio | ✔ raio | – | – | ✖ |
| Esconde entradas ao consumir | ✔ | ✔ | ✔ | ✔ | ✖ | – | – | – |
| Caixa de seleção em árvore | ✖ | ✖ | ✖ | ✖ | ✖ (botões) | ✖ | ✖ | ✖ |

| Capacidade (extrair) | Face | Várias faces | Aresta/Curva | Contorno | Eixo de aresta/face |
|---|---|---|---|---|---|
| Continua sendo Extrair (B576) | ✔ `SurfExtract` | ✔ `SurfMulti` | ~ `CurveExtract`/`CurveCopy` | ✔ `CurveBoundary` | ~ vira `CurveExtract`/`LineAxis` |
| Mantém tipo geométrico exato | ✔ (face B-rep) | ✔ | ✖ polilinha 0,05 rad | ✖ polilinha | ✖ linha por 2 pontos |
| Segue a peça (associativo) | ✔ chave v2 | ✔ | ✔ chave (`BodyEdge`) | ✔ `near` + `expected` | ~ |
| Propagação tangente/ponto | ✔ | – | ✔ | ✔ | – |

## 7. As 10 melhorias mais importantes (em ordem)

1. **Entrada de curva como cadeia universal** (B561 + B565): caixa de curvas que aceita vários sketches/arestas
   tangentes, com junção na hora, em Extrudada, Revolução, Varredura, Concordância, Preencher, e nos sólidos de
   perfil. Resolve o caso "duas curvas tangentes dão a volta inteira". Base: `geo_ui::bag`, `App::chained`,
   `CurveJoin` escondido ou lista no comando (decisão aberta do B561).
2. **Curva exata no kernel (B559):** trocar `Curve3` polilinha por uma curva paramétrica (`Geom_Curve`/BSpline
   ou NURBS) para sketch, spline e aresta; `extrude_curve`/`revolve_curve`/`sweep`/`loft` recebem a aresta
   exata (círculo e arco como `Geom_Circle`, spline como `Geom_BSplineCurve` por `GeomAPI_Interpolate`
   com a mesma parametrização do sketch). Extrair mantém o tipo geométrico (B576).
3. **Uma definição única de spline** em sketch, wireframe e kernel (mesma parametrização e tangentes);
   hoje há Hermite no solver, centrípeta no desenho e `GeomAPI_Interpolate` (corda) no sólido.
4. **Contrato de feature (B565):** painel único de seções fixas (Entradas, Direção/limites, Opções, Avançado),
   barra de comando com validação antes do OK, prévia leve para `RefKind`, e todo comando de superfície passando
   por ele (hoje um `match` de 1000 linhas em `geo_ui.rs:1463-2430`).
5. **Paridade das extrusões e revoluções** (B569): Extrusão de superfície ganha inverter, simétrico,
   ângulo de saída, início deslocado, "até plano/próximo/último", direção padrão pela normal; Revolução de
   superfície ganha inverter, simétrico e eixo pelo mesmo widget da sólida; Engrossar e Deslocada passam a
   usar o mesmo bloco de lados.
6. **Botão Inverter padrão em todo comando com direção** (B562): ligado ao mesmo campo da seta
   (`inverter_lado`), com teste por comando conferindo a caixa envolvente invertida; corrige o painel do Filete
   que descarta `flip_a/b` (`geo_ui.rs:1980`).
7. **Validação na hora e sentinela único:** trocar `CurveRef::Axis(X)` por `slots::curve()` em "+ curva"
   (`geo_ui.rs:1245`, `:2030`) e no caminho do Loft; incluir `RefKind` em `entradas::pede_atencao`; mensagens
   uniformes antes do OK, em vez de erro depois do recálculo.
8. **Qualidade de malha e continuidade medida:** `mesh_angle` com desvio mais fino para faces curvas de
   BSpline, e G1/G2 de Conectar/Blend/Preencher calculados na curva exata, não na polilinha (`end_tangent`,
   `end_curvature`, reamostra N=24); Blend em NURBS por `GeomFill`/`BRepFill` em vez de remendos Bézier de
   amostras.
9. **Sketch:** (a) Linha como polilinha contínua até Esc/duplo clique (preso só repete o comando);
   (b) restrições colinear e continuidade de spline, cota de arco; (c) padrão linear/circular, girar, escalar,
   quebrar e elipse desenhada; (d) Jacobiano analítico ou particionado para sketches grandes; (e) roteiro de
   teste por `overlay` com eventos crus para todas as ferramentas (B573).
10. **Manipuladores 3D completos e consistentes:** alça de valor para Regrada (comprimento), Curva paralela
    (d1/d2), Blend (tensão), Junção (tolerância) e Engrossar (espessura), mais seta de lado e de direção nas
    extrusões e revoluções (hoje só nas de cortar), todas com a cor do papel e o mesmo gesto de clique para
    inverter.
