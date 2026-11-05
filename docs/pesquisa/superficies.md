# Superfícies e wireframe (GSD / Surfaces / Freeform)

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Relatório de pesquisa avançada para `docs/pesquisa/superficies.md`. Escrito com palavras próprias.
> **[V]** = confirmado em fonte pública lida (ou trecho de busca citado na seção 5). **[C]** = conhecimento geral, não confirmado nesta rodada.
> Os números `RF-SUP-nn` (2 dígitos) são novos. Eles não substituem os `RF-SUP-0nn` (3 dígitos) de `docs/requisitos/superficies.md`.
> Nenhum arquivo foi criado ou alterado.

---

## 1. Resumo e principais lacunas

**Resumo**

1. O Fenkra já tem uma bancada ampla: cerca de 50 tipos de elemento em `RefKind` (`crates/fenkra-core/src/doc.rs:472-701`). Cobrem pontos, linhas, planos, curvas, superfícies e operações.
2. O nome de cada função existe. O que falta é a **profundidade de cada comando**: continuidade real, "até elemento", relimitadores, acoplamento, propagação, diagnóstico por elemento.
3. A pasta de geometria (`FolderKind::Geometria`, `doc.rs:1289-1296`) é um Geometrical Set **sem ordem**. A avaliação por dependência com memória (`lib.rs:921-927`) faz a ordem física não importar, como no Geoset do CAD avançado B.
4. Não existe Ordered Geometrical Set, nem objeto de trabalho para geometria, nem inserção "depois de".
5. Toda curva do Fenkra é uma **polilinha densa** (`Curve3.pts`, `geo3d.rs:62-67`) com um círculo analítico opcional. Superfícies construídas a partir dela (extrusão, revolução, fill, loft, sweep) herdam essa aproximação. G1/G2/G3 são estimados por diferenças finitas (`geo3d.rs:143-170`).
6. Referências a faces e arestas são **fixas por ponto ou polilinha** (`SurfRef::Face`, `SurfExtract{face}`, `CurveRef::Edge`). Isso viola o princípio 1 do ADR 0011 (estável ou erro visível) sempre que outra face passar pelo mesmo ponto.
7. Há escolhas silenciosas no código: `which` de `CurveBoundary`/`CurveIntersect` cai em "a última" (`lib.rs:1395,1504`). O `Split` ignora elementos de corte que falham (`lib.rs:1351`). A projeção direcional descarta pontos que não acertam (`lib.rs:1426-1430`).
8. As análises (zebra e curvatura) são só shader de tela (`render.rs:81-100`). Não há connect checker, draft, distância/desvio nem pente de curvatura.
9. Os modos do CAD avançado B que o Fenkra não tem e valem a pena: Join com verificações, Healing com diagnóstico, Extrapolate "até elemento" e com borda, Boundary/Extract com propagação gravada, Blend com acoplamento e tensão por lado, Fill com suporte e ponto de passagem.
10. Diferenciais dos outros CADs que o Fenkra deve copiar: Knit com folga visível (CAD avançado A), Stitch com "Find Remaining Free Edges" (CAD avançado C), Fill sem exigir 4 lados (CAD avançado E) e Boundary Patch misturando sketch e arestas (CAD avançado C).

**Principais lacunas** (detalhadas nas fichas):

- (L1) Continuidade G1/G2 só aproximada em polilinhas, sem curva analítica ou NURBS exata.
- (L2) Referências fixas por ponto (faces, arestas) sem nome topológico nem diagnóstico por elemento.
- (L3) Fill, Join, Healing, Split e Trim sem os modos e verificações do CAD avançado B e do CAD avançado A.
- (L4) Falta "até elemento", relimitadores e propagação em Extrapolate, Extrude, Sweep, Boundary e Extract.
- (L5) Sem Ordered Set, In Work Object de geometria, nem análises de continuidade, desvio e draft.

---

## 2. Fichas por comando

Organização:

- **A.** Wireframe (fichas 1–13)
- **B.** Superfícies (14–23)
- **C.** Operações (24–33)
- **D.** Sets e update (34)
- **E.** Análises (35–39)
- **F.** Sólido e Classe A (40–42)

### A. Wireframe

### 1. Ponto de referência (CAD avançado B: Point / CAD avançado A: Reference Point + Sketch Point 3D / CAD avançado C: Work Point / outros: CAD avançado E Mate-connector, CAD avançado F: Datum Point / Fenkra: PointCoords, PointOnCurve, PointOnPlane, PointOnSurface, PointCenter, PointTangent, PointBetween, PointCurvePlane, PointCurveCurve, PointProject, PointProjectCurve, PointOffset)

- **O que faz:** cria uma posição 3D usada como entrada de linhas, planos, centros e relimitadores.
- **CAD avançado B:**
  - Tipos: Coordinates, On curve, On plane, On surface, Circle/sphere center, Tangent on curve, Between. [C] A sequência dos tipos vem de conhecimento geral. O Point on curve tem o campo "Reference point" e referências por distância (comprimento real ou proporção) e permite "repeat object" (vários pontos). [C]
  - Em On surface há direção e distância medidas na superfície (geodésica). [C]
  - Saída: `Point.n` na árvore. Pontos "sem referência" ficam isolados e não se movem; pontos de curva dependem do suporte.
  - Quando a referência some: o ponto fica vermelho (update error). O Update Diagnosis oferece Edit/Replace/Isolate/Deactivate. [C]
  - O texto-base do usuário (`docs/contextos/superficies.md`) pede ponto paramétrico (associativo) por padrão, e coordenadas só como exceção. [V: texto do usuário]
- **CAD avançado A:** Reference Point (arco, centro de face, interseção, projeção, N pontos distribuídos numa aresta). Em sketch 3D há pontos livres e "Sketch Point". [C] Pontos dependem da geometria; se ela some, vira "Dangling"/erro na árvore.
- **CAD avançado C:** Work Point (vértice, interseção de arestas, três planos, centro de toro, ponto em superfície). Em sketch 3D, pontos incluem "Project to Surface". [C]
- **Outros:** o CAD avançado E usa Mate Connector e pontos de sketch; o CAD avançado F usa Datum Point on Curve/Surface com offset. [C]
- **Fenkra hoje:**
  - Nove modos de ponto em `RefKind` (`doc.rs:498-512,546-554`), todos com expressão (`String`) e portanto paramétricos. Avaliados em `lib.rs:1022-1060`.
  - `PointOnCurve` usa `OnCurve::{Distance, Ratio}` sobre a polilinha (`lib.rs:905-918`); erro visível se fora da curva (bom).
  - **Bug provável:** `PointOnSurface` desloca (u, v) **no plano tangente** a partir do ponto central da superfície (`doc.rs:505`, `lib.rs:1024`, `surf_frame` em `lib.rs:892-902`). Em superfície curva o ponto sai da superfície, e o nome "(u, v)" engana.
  - `PointRef::Fixed([f64;3])` e `Origin` são fixos; `CurveVertex{curve,index}` depende de índice (`doc.rs:94`).
  - Faltam: N pontos equidistantes numa curva (repetição), ponto extremo (mín/máx numa direção) e ponto geodésico na superfície.
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte → modo → parâmetros → avançado (ponto de referência e sentido).
  - Todo ponto "clicado" deve gravar o suporte e o parâmetro, não só XYZ. `Fixed` só como "ponto isolado" explícito.
  - `PointOnSurface` deve projetar de volta na superfície real (`surface_near`) e aceitar coordenada UV real quando houver.
  - Testes: ponto sobre cilindro curvo permanece a <1e-6 do cilindro; mudar o raio desloca o ponto; apagar a curva-suporte gera erro nomeado.

### 2. Linha de referência (CAD avançado B: Line / CAD avançado A: Reference Axis, Sketch 3D line / CAD avançado C: Work Axis, 3D line / Fenkra: LinePoints, LineDir, LineAngle, LineNormalSurface, LinePlanes, LineBisector, LineAxis)

- **O que faz:** cria um segmento ou eixo, limitado ou infinito.
- **CAD avançado B:**
  - Tipos: Point-Point, Point-Direction, Angle/Normal to curve, Tangent to curve, Normal to surface, Bisecting. [C]
  - Cada ponta aceita um comprimento ou **"Up-to"** (elemento limitante). [C]
  - Opção Mirrored extent (simétrico). [C]
  - Em Angle/Normal usa-se um suporte (plano ou superfície).
  - Várias soluções aparecem como "Next solution". [C]
  - O texto-base recomenda "linhas limitadas para evitar dependências infinitas". [V: texto do usuário]
- **CAD avançado A:** Reference Axis (uma linha/aresta, dois planos, dois pontos, cilindro, ponto+superfície) e Curve (Through XYZ, Through Reference Points). [C] Linhas de sketch 3D usam planos de apoio.
- **CAD avançado C:** Work Axis (aresta, duas faces, dois pontos, normal a plano por ponto). [C] A 3D line aceita "Bridge" e restrições.
- **Fenkra hoje:**
  - `LinePoints{a,b,start,end}` com extensão nas pontas (`doc.rs:516`), `LineDir`, `LineAngle` (0° = tangente, 90° = normal, no plano de apoio), `LineNormalSurface`, `LinePlanes`, `LineBisector`, `LineAxis{face}` (por ponto clicado).
  - Falta "até elemento" nas pontas: `start`/`end` são só números.
  - Falta tangente por ponto externo à curva. `LineAxis` usa ponto clicado numa face cilíndrica (fixo).
- **Melhor prática:**
  - Ponta com 3 opções: valor, proporção, "até elemento" (plano/superfície/ponto).
  - `LineAxis` deve ser reconhecido por `owner` + papel (ADR 0010), não por ponto.
  - Testes: linha normal numa esfera cruza o centro; linha "até plano" termina no plano ao mover o plano; ângulo 0°/90° em curva planar.

### 3. Plano de referência (CAD avançado B: Plane / CAD avançado A: Reference Plane / CAD avançado C: Work Plane / Fenkra: OffsetPlane, AnglePlane, FacePlane, PlaneParallelPoint, PlaneThreePoints, PlaneTwoLines, PlanePointLine, PlaneCurve, PlaneNormalCurve, PlaneTangentSurface, PlaneEquation, PlaneMeanPoints, PlanePointNormal, PlaneMid)

- **O que faz:** cria um plano de trabalho, apoio de sketch ou limite.
- **CAD avançado B:**
  - Tipos: Offset from plane, Parallel through point, Angle/Normal to plane, Through 3 points, Through 2 lines, Through point and line, Through planar curve, Normal to curve, Tangent to surface, Equation, Mean through points. [C]
  - Offset aceita "Repeat object after OK". [V: padrão visto nas páginas de Translate/Scale/Rotate]
  - O plano é infinito na lógica, finito só na exibição.
  - Planos por três pontos colineares falham; o texto-base prefere "ponto + normal". [V: texto do usuário]
- **CAD avançado A:** Plane com até 3 referências e restrições (Coincident, Parallel, Perpendicular, Angle, Offset, Tangent, Mid-plane). A árvore mostra qual restrição está cumprida. [C]
- **CAD avançado C:** Work Plane (offset, ângulo, três pontos, tangente a superfície e ponto, normal a eixo em ponto, entre dois planos). [C]
- **Outros:** o CAD avançado F tem Datum Plane com restrições Through/Normal/Parallel/Offset/Tangent. [C]
- **Fenkra hoje:**
  - 14 modos (`doc.rs:473-496,560-564`), mais `Spine{planes}` (planos normais à curva).
  - Benchmark SUP-03: `FacePlane` continua vivo quando a face some (DR-21), e falta o plano na curva por ponto projetado (DR-04). `FacePlane{origin,x,normal,owner}` guarda cópia da face, não referência (`doc.rs:478`).
  - `PlaneTangentSurface` projeta o ponto na superfície (`doc.rs:492`).
- **Melhor prática:** plano de face guarda `owner` + papel (ADR 0010) e, se a face some, o plano vira "falhou" (nunca fica "vivo" silenciosamente). Teste: apagar a face-pai produz erro nomeado e diagnóstico "Substituir/Isolar".

### 4. Projeção (CAD avançado B: Projection / CAD avançado A: Projected Curve / CAD avançado C: Project to Surface (3D sketch) / outros: CAD avançado E Project curve / Fenkra: CurveProject, CurveProjectSurf, PointProject)

- **O que faz:** leva ponto ou curva a um plano ou superfície, pela normal ou por uma direção.
- **CAD avançado B:**
  - Entradas: um ou vários pontos/curvas (multi-seleção) e um suporte.
  - Direção: **Normal** ou **Along a direction** (linha ou plano; ou vetor X, Y, Z). [V]
  - Opção **Nearest solution**: quando há várias projeções, fica só a mais próxima. [V]
  - Aviso: a projeção "tem efeito derivativo", então perde continuidade. Entrada G2 gera saída ao menos G1; entrada G1 gera ao menos G0. [V]
  - Saída `Project.n`. Com multi-output, vira um conjunto editável (`Multi Output.n`). [V]
  - Texto-base: "prefira projeção direcional em superfícies complexas"; normal pode ter várias soluções. [V: texto do usuário]
- **CAD avançado A:** Projected Curve com duas formas: Sketch on Faces (esboço projetado ao longo da normal do sketch) e Sketch on Sketch (interseção de dois extrusos). Opção Reverse projection. [C]
- **CAD avançado C:** em sketch 3D há "Project to Surface" com duas opções: ao longo de um vetor ou ao ponto mais próximo (usando normais da superfície). [V: busca fornecedor de CAD avançado]
- **Fenkra hoje:**
  - `CurveProjectSurf{curve,surface,dir}` (`lib.rs:1399-1438`): plano com ou sem direção; superfície sem direção usa `surface_near` ponto a ponto; com direção usa `ray_hit`.
  - **Risco:** na projeção direcional, pontos que não acertam são **descartados em silêncio** (`lib.rs:1426-1430`) e só falha se sobrarem menos de 2. A curva resultante pode ser só um trecho, sem aviso.
  - `CurveProject{curve,plane}` e `PointProject` são casos particulares.
  - Falta "solução mais próxima" para várias interseções, projeção de ponto com vetor e propagação do suporte (sub-elemento).
  - O resultado é uma polilinha nova, não uma curva-na-superfície com parâmetros UV.
- **Melhor prática:**
  - Fluxo: curva → suporte → direção (normal / vetor) → avançado ("solução mais próxima", tolerância).
  - Regra: se algum trecho não acertar, estado "parcial" com contagem de pontos perdidos. Nunca devolver uma curva mais curta sem aviso.
  - Testes: projetar círculo em plano inclinado; projetar linha sobre cilindro nas duas direções; direção paralela ao plano gera erro (já existe em `lib.rs:1413`); curva que sai do suporte mostra o trecho perdido.

### 5. Interseção (CAD avançado B: Intersection / CAD avançado A: Intersection Curve, Face Curves / CAD avançado C: Intersection Curve (3D sketch) / Fenkra: CurveIntersect, PointCurvePlane, PointCurveCurve, LinePlanes)

- **O que faz:** gera a curva (ou ponto) comum a dois elementos.
- **CAD avançado B:**
  - Combinações: superfície×superfície, plano×superfície, curva×superfície (ponto), curva×curva (ponto); lista de soluções. [V: kwrug constructors lista `intersect` para esses pares]
  - Opções: extensão linear dos suportes; resultado "Non-connex" vira multi-cell que se separa com Near/Disassemble. [C]
  - Texto-base: "use interseção aproximada para superfícies importadas". [V: texto do usuário]
  - Quando um suporte some, o elemento fica em erro de update e deixa de alimentar os filhos.
- **CAD avançado A:** Intersection Curve (sketch 3D) e Split Line. Em Surface Trim há "Intersection Curve" para pré-visualizar. [C]
- **CAD avançado C:** o Intersection Curve do sketch 3D aceita faces, superfícies, curvas 2D, planos e corpos; atualiza quando os corpos mudam. Os extremos dos fios não são restringidos e o deslocamento pode ser inconsistente. [V]
- **Fenkra hoje:**
  - `CurveIntersect{a,b,which}` (`lib.rs:1393-1398`) usa `section()` do kernel (polilinhas). `which` indexa a lista de curvas.
  - **Bug provável:** `cs.get(*which).or(cs.last())` — se a topologia mudar e o índice não existir mais, o Fenkra pega **a última** curva sem erro. Viola o princípio 1 do ADR 0011.
  - Falta "solução mais próxima de um ponto" (identidade robusta da solução).
- **Melhor prática:**
  - Solução escolhida por **ponto-âncora** (a curva mais próxima do ponto gravado), com erro se nenhuma estiver a menos de uma tolerância.
  - Testes: esfera×plano deslocado gera um círculo; mudar o raio mantém a mesma curva; esfera tangente ao plano gera "interseção vazia" nomeada.

### 6. Curva paralela / offset de curva (CAD avançado B: Parallel Curve / CAD avançado A: Offset Entities (3D), Offset on Surface / CAD avançado C: Offset (sketch) / Fenkra: CurveParallel)

- **O que faz:** desloca uma curva por distância constante ou variável sobre um suporte.
- **CAD avançado B:** [V pelas propriedades da API CAA, via pycatia]
  - Entradas: curva, suporte (plano ou superfície), ponto de passagem opcional, offset.
  - **Modo:** Euclidiano ou **Geodésico** (`geodesic`); direção invertida; **Both sides** (`keep_both_sides`) com lado oposto acessível.
  - **Tipo de canto:** agudo (0) ou arredondado (1).
  - **Lei de offset:** constante, linear, em S ou avançada (de uma lei GSD). Inversão do mapeamento da lei.
  - **Suavização:** nenhuma, G1 ou G2, com desvio máximo.
  - Sem suporte, usa o plano com normal dada.
  - A direção vem de tangente × normal do suporte.
  - Texto-base: "aproximado/ajuste conforme a curvatura". [V: texto do usuário]
  - Quando o suporte some: update error; a lei é um feature separada que precisa existir.
- **CAD avançado A:** Offset Entities no sketch e Offset on Surface em sketch 3D (distância sobre a face). [C]
- **CAD avançado C:** Offset no sketch 2D, associativo, com "constrain offset". Em 3D usa projeção. [C]
- **Fenkra hoje:**
  - `CurveParallel{curve,support,d1,d2,flip,more,both,d_other}` (`lib.rs:1439-1501`): d1→d2 linear (equivale à lei linear), `more` junta várias curvas, `both` cria contorno fechado, `d_other` distância do outro lado.
  - Offset por ponto: `side = normal × tangente` (tangente central entre vizinhos), deslocado `dd` e reprojetado com `surface_near`.
  - **Faltam:** tipo de canto (agudo/arredondado) — em polilinha com ângulo vivo o deslocamento cria auto-interseções ou dobras; modo geodésico; lei S/avançada; suavização; ponto de passagem.
  - **Risco:** curva fechada com `both` dá erro (`lib.rs:1487`).
- **Melhor prática:**
  - Canto **arredondado por padrão** em offsets para fora, e interseção (agudo) para dentro.
  - Detectar auto-interseção do resultado e avisar (estado "degenerado").
  - Testes: offset de retângulo para fora e para dentro (cantos); offset de espiral em superfície cilíndrica (geodésico × euclidiano); lei linear d1=0→d2=10.

### 7. Curva de conexão (CAD avançado B: Connect Curve / CAD avançado A: Fit Spline + Curve Through Points, Blend Curve (sketch 3D) / CAD avançado C: Bridge Curve (3D sketch) / Fenkra: Connect)

- **O que faz:** liga as pontas de duas curvas com continuidade escolhida.
- **CAD avançado B:** [V pela API via pycatia]
  - Conecta dois pares (curva, ponto); continuidade **por lado**: ponto, tangência ou curvatura. Tensão **por lado**.
  - Orientação por lado: igual, invertida ou terceira opção.
  - `connect_type`: normal ou com curva base.
  - **Trim**: se verdadeiro, as curvas conectadas são aparadas no ponto de conexão.
  - Texto-base: "C2 exige superfícies classe A; use C1 para superfícies técnicas". [V: texto do usuário]
- **CAD avançado A:** não há "Connect Curve" direto; usa-se Fit Spline com tangência ou Curve Through Points. O sketch 3D oferece "Spline" com relações de tangência/curvatura. [C]
- **CAD avançado C:** o Bridge Curve (sketch 3D) liga duas curvas com tangência ou continuidade suave. [C; a busca não detalhou]
- **Fenkra hoje:**
  - `Connect{a,a_end,b,b_end,cont,tension}` (`lib.rs:1077-1103`): G0 reta, G1 Hermite cúbica, G2 quíntica com curvatura das pontas, G3 Hermite de grau 7 (`geo3d.rs:403`).
  - Teste citado no benchmark: `conector_g3_e_canto_entre_linha_e_spline`.
  - Tensão única (um número) em vez de uma por lado; sem aparar as curvas; derivadas vêm da **polilinha** (`end_tangent/curvature/jerk`, `geo3d.rs:143-170`), então G2/G3 só valem quanto a polilinha for fina.
  - A UI já rotula G2 como "aprox." em outro contexto (`geo_ui.rs:1526`).
- **Melhor prática:**
  - Tensão por lado (padrão 1.0); opção "aparar".
  - Mostrar na árvore a continuidade realmente atingida (medida), não só a pedida.
  - Testes: linha→arco G1 tem ângulo <0,1° na junção; G2 tem diferença de curvatura <1%; ponta coincidente gera o erro já existente.

### 8. Canto (CAD avançado B: Corner / CAD avançado A: Sketch Fillet, 3D / CAD avançado C: Fillet (sketch) / Fenkra: Corner)

- **O que faz:** arredonda o encontro de duas curvas com um arco de raio dado.
- **CAD avançado B:** [V pela API via pycatia]
  - Dois elementos (o primeiro pode ser ponto, o segundo é sempre curva) e **suporte** (plano ou superfície), ou direção 3D.
  - Cada curva tem **orientação** (+1/−1 relativa a normal×tangente) e há índice de discriminação para várias soluções.
  - **Trim** com modo: nenhum, ambas, só a primeira, só a segunda. Opção **On vertex**.
  - Fórum: o Corner exige que as curvas se alcancem; estender a linha resolve. [V: busca Eng-Tips]
  - Texto-base: "curvas com ângulo muito agudo podem falhar; ajuste o raio". [V: texto do usuário]
- **CAD avançado A / CAD avançado C:** Sketch Fillet/Chamfer no 2D (aparam automaticamente). [C]
- **Fenkra hoje:**
  - `Corner{a,b,radius}` (`lib.rs:1104`, `geo3d.rs:524-573`): para duas linhas, calcula o vértice, o raio e devolve **só o arco**; para curvas quaisquer, `corner_curves` usa deslocamento por `r` e a solução mais próxima do encontro.
  - Falta: aparar as curvas originais, escolher a solução, canto 3D com direção, suporte superfície (só curvas coplanares).
- **Melhor prática:** botão "aparar e juntar" que gera a polilinha final e reaproveita `join_curves` (`lib.rs:270`). Testes: linhas perpendiculares com r=5 (comprimento do arco = πr/2); linhas paralelas geram erro nomeado (já existe).

### 9. Spline 3D (CAD avançado B: Spline / CAD avançado A: Spline (3D sketch), Curve Through XYZ / CAD avançado C: Interpolation + Control Vertex Spline (3D) / Fenkra: Spline, SplineControl)

- **O que faz:** cria uma curva livre por pontos de passagem ou por polígono de controle.
- **CAD avançado B:** [V]
  - Pontos de passagem (lista editável: inserir antes/depois, remover, substituir).
  - **Geometry on support**: projeta a spline num plano/superfície (só funciona se as tangentes estiverem no suporte).
  - Por ponto: tangente (explícita por reta/plano, ou "de curva") com **tensão** (ex.: 2) e **curvatura** (direção + raio); ordem: Tangent Dir. → Tension → Curvature Dir. → Radius. Botões Remove/Reverse Tgt, Remove Cur.
  - **Close Spline** quando a geometria permite.
  - O texto-base recomenda grau 5 para classe A. [V: texto do usuário]
- **CAD avançado A:** spline com pontos de controle ou ajuste, manipuladores de tangente e peso, "Curvature combs" e "Show curvature". [C]
- **CAD avançado C:** dois tipos: Interpolation (2D, 3D e sobre superfície) e Control Vertex (quadro de controle). Converte entre os dois; insere vértices pelo botão direito. [V]
- **Fenkra hoje:**
  - `Spline`: Catmull-Rom centrípeta (`geo3d.rs:331-373`), 24 passos por vão, extremos por reflexão. `SplineControl`: B-spline uniforme (`lib.rs:614`).
  - **Faltam:** tangência/curvatura por ponto, tensão, projeção em suporte, grau, conversão entre tipos, spline sobre superfície.
  - Gera polilinha de ~24 pontos por vão (não uma curva NURBS), então não dá continuidade exata ao ser usada como seção de loft.
- **Melhor prática:**
  - Entregar curva NURBS verdadeira (OCC `Geom_BSplineCurve`) como "curva analítica" ao lado da polilinha de visualização, e decidir já se `Curve3` ganha um campo `nurbs`.
  - Testes: spline por 5 pontos passa por todos a <1e-6; spline fechada não tem salto de tangente; spline com tangente imposta respeita o ângulo.

### 10. Hélice (CAD avançado B: Helix / CAD avançado A: Helix and Spiral / CAD avançado C: Helical Curve / Fenkra: Helix)

- **O que faz:** gera hélice cilíndrica ou cônica, usada como guia de molas e roscas.
- **CAD avançado B:** [V pela API via pycatia]
  - Eixo (reta/aresta retilínea), **ponto inicial** (fora do eixo), passo, altura, número de revoluções, ângulo inicial.
  - Sentido horário/anti-horário; inverter o eixo; **conicidade** (ângulo) com sentido (raio diminui ou cresce); lei de passo (constante, em S...); `profile` (raio variável por perfil).
  - Limitação histórica: o eixo precisava ser linear. Há gambiarra para hélice sobre spline. [V: GrabCAD/Eng-Tips]
- **CAD avançado A:** Helix and Spiral: pitch & revolution, height & revolution, height & pitch, spiral; variable pitch; taper; start angle; clockwise/counterclockwise. [C]
- **CAD avançado C:** Helical Curve: constante ou passo variável; combinações pitch/revolutions/height; flat spiral; conicidade; sentido; **transições de início/fim** (natural ou plana). [V]
- **Fenkra hoje:**
  - `Helix{origin,axis,radius,pitch,height,taper,left}` (`doc.rs:532`, `geo3d.rs:448-469`): 64 pontos por volta, cônica via `tan(taper)` aplicada ao **eixo** (raio = r + h·tan).
  - **Faltam:** ponto inicial/ângulo inicial (a partida é `any_perp(ax)`, arbitrária — a hélice não "começa" onde o usuário espera), número de voltas como entrada, passo variável, transições planas, eixo curvo.
  - **Bug provável:** `any_perp` pode mudar com a orientação do eixo, mudando o ponto de partida entre versões.
- **Melhor prática:** aceitar ponto inicial (ângulo) e fixar a convenção "x do plano de apoio". Entradas: passo+altura, altura+voltas ou passo+voltas. Testes: 1 volta fecha o ângulo (ponto final acima do inicial); mola cônica; mão esquerda × direita.

### 11. Espiral (CAD avançado B: Spiral / CAD avançado A: Spiral / CAD avançado C: Flat Spiral (Helical Curve) / Fenkra: Spiral)

- **O que faz:** curva plana que cresce com o ângulo.
- **CAD avançado B:** parâmetros de centro, plano de apoio, raio inicial, ângulo/revoluções e variação do raio; sentido. [C; a busca não achou página específica]
- **CAD avançado A:** spiral por passo e revoluções, ângulo inicial, sentido. [C]
- **CAD avançado C:** flat spiral no Helical Curve (só com tipo constante). [V]
- **Fenkra hoje:** `Spiral{center,plane,r0,r1,turns}` (`geo3d.rs:471-485`): Arquimediana, 96 pontos por volta. Sem ângulo inicial, sem passo, sem sentido, sem espiral logarítmica.
- **Melhor prática:** entradas por passo+voltas e por r0→r1; ângulo inicial; sentido. Testes: r0=0, r1=10, 2 voltas → comprimento coerente; passo radial constante.

### 12. Combine (CAD avançado B: Combine Curve / CAD avançado A: Projected Curve (Sketch on Sketch), Composite Curve distinct / CAD avançado C: — / Fenkra: ausente)

- **O que faz:** cria a curva 3D resultante da interseção das extrusões de duas curvas em direções dadas.
- **CAD avançado B:** duas curvas e duas direções (padrão: normais aos planos das curvas); o resultado só existe onde as extrusões se cruzam. [V pela descrição em guia terceiro; opções detalhadas [C]]
- **CAD avançado A:** é o "Sketch on Sketch" do Projected Curve: dois esboços em planos diferentes viram uma curva 3D (útil para formas de arame e hélices projetadas). [C]
- **Fenkra hoje:** não existe. Dá para obter com `SurfExtrude` das duas curvas + `CurveIntersect`, mas sem associação direta.
- **Melhor prática:** implementar como atalho que cria duas extrusões ocultas e a interseção. Teste: vista frontal + vista lateral de um arco → curva 3D com projeções iguais às vistas.

### 13. Reflect line / silhueta (CAD avançado B: Reflect Line / CAD avançado A: Split Line (Silhouette) / CAD avançado C: Silhouette Curve (3D sketch) / Fenkra: ausente)

- **O que faz:** curva onde a normal da superfície faz ângulo constante com uma direção (linha de partição, de reflexo ou de silhueta).
- **CAD avançado B:** [V pela API via pycatia]
  - Suporte (superfície), fonte **infinita** (cilíndrica, com direção) ou **pontual** (cônica, com origem), ângulo, medido a partir da normal ou do plano tangente.
  - Orientação do suporte e da direção.
- **CAD avançado A:** Split Line tipo Silhouette (direção de puxar) gera a linha de partição de molde. [C]
- **CAD avançado C:** Silhouette Curve (3D sketch) é associativa, com direção de puxada por plano, face, aresta ou eixo, excluindo faces retas/internas. [V]
- **Fenkra hoje:** não existe. Uma função de "linha de partição" ligaria o módulo ao ângulo de saída (draft).
- **Melhor prática:** `ReflectLine{support, direction, angle}` que amostra a normal na malha fina e extrai a curva de nível, depois refina com `surface_near`. Teste: esfera com direção Z e 90° → equador.

### B. Superfícies

### 14. Extrusão de superfície (CAD avançado B: Extrude / CAD avançado A: Extruded Surface / CAD avançado C: Extrude (Surface) / Fenkra: SurfExtrude)

- **O que faz:** arrasta uma curva numa direção e gera superfície.
- **CAD avançado B:** [V]
  - Entrada: perfil, direção (reta → sua orientação; plano → sua normal; ou vetor X, Y, Z).
  - Limites **início e fim** (valores ou manipuladores); botão Reverse Direction. [V]
  - Em Part Design e GSD há limite "Up-to element". [C]
  - Texto-base: curvas fechadas dão superfícies tubulares; prefira normal à curva. [V: texto do usuário]
- **CAD avançado A:** Extruded Surface com End Conditions (Blind, Up to Vertex, Up to Surface, Offset from Surface, Up to Body, Mid Plane) e direção 2. [C]
- **CAD avançado C:** Extrude com output Surface; extents Distance, To, To Next, Between, All. [C]
- **Fenkra hoje:**
  - `SurfExtrude{curve,dir,length,length2}` (`lib.rs:1265-1273`): `length2` recua a curva e soma ao comprimento (`MakePrism`, `surfaces.inc:70`). Só comprimentos; sem "até elemento" (benchmark SUP-09, B14).
  - Bug provável: com `length2` o prisma sai da curva recuada — correto, mas `length` e `length2` iguais a zero gera o erro "comprimento zero" sem dizer qual campo.
  - Manipulador existe (`handles.rs:134,234`).
- **Melhor prática:** limite 1 e limite 2 com 3 tipos (valor, até plano/superfície, até ponto). Resolver "até" com `extended_faces` (já existe em `surfaces.inc:188`). Testes: extrudar até plano inclinado e mover o plano; limite 2 simétrico; curva fechada gera tubo aberto nas duas pontas.

### 15. Revolução de superfície (CAD avançado B: Revolve / CAD avançado A: Revolved Surface / CAD avançado C: Revolve (Surface) / Fenkra: SurfRevolve)

- **O que faz:** gira uma curva em torno de um eixo.
- **CAD avançado B:** perfil, eixo, **ângulos 1 e 2** (cada lado); sentido. [C]
- **CAD avançado A:** Revolved Surface com ângulo, direção 2, simétrico. [C]
- **CAD avançado C:** Revolve com output Surface; ângulo; full, to, between. [C]
- **Fenkra hoje:** `SurfRevolve{curve,axis,angle}` (`lib.rs:1274`, `surfaces.inc:79`). Só um ângulo; ângulo ≥360° usa revolução completa. Sem segundo ângulo, sem "até".
- **Melhor prática:** ângulo 1 e 2, "completa", e erro nomeado quando a curva toca o eixo em ponto coincidente (superfície degenerada, citado no texto-base). Teste: semicírculo revolvido 360° gera esfera fechada (1 face, sem aresta livre).

### 16. Esfera (CAD avançado B: Sphere / CAD avançado A: Dome, Revolved Surface / CAD avançado C: — / Fenkra: SurfSphere)

- **O que faz:** superfície esférica por centro e raio, com limites angulares.
- **CAD avançado B:** centro, raio (ou ponto), limites de longitude e latitude (ângulos 1/2). [C]
- **Fenkra hoje:** `SurfSphere{center,radius}` (`hybrid.inc:97`) devolve o casco completo. Sem limites angulares (para uma meia-esfera usa-se `SurfSplit`, como no teste de `lib.rs:6320-6322`).
- **Melhor prática:** limites de latitude/longitude; eixo da esfera selecionável. Teste: metade superior com latitude 0°→90° fecha na borda circular exata.

### 17. Cilindro (CAD avançado B: Cylinder / CAD avançado A: Extruded Surface de círculo / CAD avançado C: — / Fenkra: SurfCylinder)

- **O que faz:** só a face lateral de um cilindro.
- **CAD avançado B:** ponto, direção, raio, comprimentos 1 e 2. [C]
- **Fenkra hoje:** `SurfCylinder{point,dir,radius,start,end}` (`lib.rs:1280`, `hybrid.inc:106`): face analítica real (`Geom_CylindricalSurface`), com alturas negativas permitidas (`-start`). É um bom padrão: **superfície analítica** em vez de aproximação.
- **Melhor prática:** estender o padrão — círculo + `SurfExtrude` deveria reconhecer o círculo analítico (`Curve3.circle`) e devolver cilindro de verdade, e não prisma de polilinha. Teste: `SurfExtrude` de círculo gera face de tipo cilindro (`face_props.kind`).

### 18. Offset de superfície (CAD avançado B: Offset / CAD avançado A: Offset Surface / CAD avançado C: Thicken/Offset (Surface) / outros: CAD avançado E Offset surface / Fenkra: SurfOffset)

- **O que faz:** cria cópia paralela à superfície a uma distância.
- **CAD avançado B:** [V]
  - Seleciona a superfície, valor ou manipulador, setinha de direção, **Both sides** (uma de cada lado), **Repeat object after OK** (vários offsets, múltiplos do valor).
  - Avisos: geometria degenerada (valor grande demais) mostra sinal de alerta na geometria e trava o manipulador, exigindo corrigir no diálogo.
  - Saída `Offset.n`.
- **CAD avançado A:** Offset Surface; distância 0 = cópia no lugar. [V: CAD avançado E confirma essa convenção no seu equivalente; CAD avançado A [C]]
- **CAD avançado C:** o Thicken/Offset com saída Surface faz offset de faces ou superfícies; o resultado Solid é o espessamento. [V]
- **Outros:** o CAD avançado E também aceita distância 0 como cópia. [V]
- **Fenkra hoje:** `SurfOffset{surface,dist,both,join}` (`lib.rs:1217-1231`): `BRepOffsetAPI_MakeOffsetShape` com junção Arco, Interseção ou Automático (`surfaces.inc:29-52`). `both` junta as duas faces num composto.
  - Faltam: repetição, distância 0 (cópia) — a OCC pode falhar com 0 —, e erro específico de "raio de curvatura menor que o offset".
  - `surface_shape` recusa plano ("use 'Plano deslocado'", `lib.rs:783`) — correto, mas CAD avançado A/CAD avançado E aceitam offset de face plana.
- **Melhor prática:** distância 0 vira cópia; aviso de degenerado com o **raio mínimo de curvatura**; repetir N vezes. Teste: offset de cilindro r=10 por +2 e −2; por −11 falha com a mensagem do raio mínimo.

### 19. Sweep / varredura (CAD avançado B: Sweep (Explicit, Line, Circle, Conic) / CAD avançado A: Swept Surface / CAD avançado C: Sweep (Surface) / Fenkra: SurfSweep, SurfSweepX)

- **O que faz:** gera superfície movendo um perfil (ou uma geratriz) ao longo de guias.
- **CAD avançado B — sweep explícito:** [V]
  - Modos: perfil + 1 guia (a guia é spine), perfil + 2 guias (a 1ª é spine), perfil + guia(s) + **spine separada** (perfil varre planos normais à spine).
  - Seleções (ordem): perfil planar → guia → spine (opcional) → 2ª guia → **superfície de referência** (opcional; controla a orientação do perfil; pode impor ângulo) → **relimitadores** (pontos ou planos que encurtam a varredura).
  - **Correção angular** suaviza desvios <0,5°; **posição do perfil** (ponto de posicionamento, eixo X, inversões, ponto âncora; só no P2).
  - Aviso: relimitador em curva fechada deve ser ponto, não plano (o plano cruza várias vezes). Continuidade cai um nível (spine G2 → superfície ≥G1).
  - API: `solution_no` (várias soluções), `canonical_detection`, `smooth_activity`, `guide_deviation`, `pulling_direction`, `angle_law`. [V]
- **CAD avançado B — tipos Line / Circle / Conic:** existem em famílias (Line: duas guias, guia+direção+ângulo, etc.; Circle: 3 guias, 2 guias+raio, centro+raio...; Conic: cinco condições) [C; a mesma lista aparece no Fenkra].
- **CAD avançado A:** Swept Surface com Profile and Path, Guide Curves, Orientation (Follow Path, Follow Path and First Guide Curve, Keep Normal Constant, Twist...), Start/End Tangency, Twist along path. [C]
- **CAD avançado C:** Sweep com Path, Path & Guide Rail, Path & Guide Surface (orientação perpendicular ao caminho, paralela, ou guiada por superfície), taper e twist. [C]
- **Fenkra hoje:**
  - `SurfSweep{profile,spine,mode}` (`lib.rs:1286-1308`): `MakePipeShell`, modos Corrigido/Fixo/Frenet (`hybrid.inc:116-127`). `SweepProfile::Circle` gera um **polígono de 48 pontos** (`lib.rs:1294-1299`).
  - `SurfSweepX{kind,spine,solution,smooth,sections}` (`lib.rs:1238`, `sweep_x` em `lib.rs:1973-2100+`): sete tipos (`doc.rs:274-289`) — Explicit (1 ou 2 guias, transformação de semelhança), Linear, LinearDir, Circle3, CircleRadius, CircleCenter, Conic. Referencial por dupla reflexão, `sections` de 4 a 200 e loft entre seções amostradas.
  - **Riscos:**
    - o tubo `Circle` sai de um polígono e provavelmente vira B-spline interpolada, não cilindro. Isso quebra espessar, medir raio e reconhecer o cilindro (provável; depende de `spine_wire`, não lido na íntegra);
    - o modo "Fixo" fixa `gp_Dir(0,0,1)` (`hybrid.inc:120`) em vez de direção escolhida;
    - sem relimitadores, sem superfície de referência, sem ângulo, sem correção angular;
    - `sections` expõe a amostragem ao usuário (artefato interno).
  - Teste existente: `superficies_varredura_avancada_offset_filete…` (benchmark SUP-10/14; `lib.rs:5646-5680`).
- **Melhor prática:**
  - Fluxo: modo/tipo → perfil → guia(s) → spine → superfície de referência → relimitadores → avançado.
  - Perfil circular analítico; "até elemento" nas pontas; ocultar `sections` (automático por tolerância de desvio).
  - Testes: tubo r=2 por spine reta → face cilíndrica; spine em S sem torção (mínima rotação); duas guias com escala; relimitador por plano.

### 20. Preenchimento / fill (CAD avançado B: Fill / CAD avançado A: Filled Surface / CAD avançado C: Boundary Patch / outros: CAD avançado E Fill, CAD avançado G N-Sided / Fenkra: SurfFill)

- **O que faz:** cria superfície dentro de um contorno fechado, opcionalmente tangente/curva às vizinhas.
- **CAD avançado B:** [V]
  - Contorno: curvas ou arestas formando laço fechado, lista editável (inserir antes/depois, remover, substituir).
  - **Suporte por curva/aresta** e continuidade **Point ou Tangent** por aresta (via API também curvatura, global ou por aresta).
  - **Ponto de passagem** (deve ficar dentro da área, senão resultado inconsistente).
  - API: modo tolerante (nenhum/automático/manual), desvio máximo de suavização, detecção canônica, "plane only".
- **CAD avançado A:** [V por snippets/Hawk Ridge]
  - Boundary: arestas, sketch ou curvas; continuidade por aresta Contact/Tangent/Curvature; **Apply to all edges**; **Alternate face** para escolher a face-mãe.
  - Sketch só aceita Contact.
  - **Constraint curves** (controle de inclinação), **Optimize surface** (2 ou 4 lados, estilo loft), **Resolution control** (só sem optimize), **Fix up boundary** (constrói/apara partes), **Merge result**, **Create solid** se fechar.
- **CAD avançado C:** [V] Boundary Patch: sketch 2D fechado (plano), sketch 2D/3D (não plano), arestas do modelo (cadeias com Automatic Edge Chain), ou mistos.
  - Condições por aresta: Contact, Tangent, Smooth com **Weight** (0–1) e **Guide Rails**.
  - Regras: arestas contínuas compartilham condição; múltiplos laços = ilhas; evitar patch em geometria de construção de um segmento (círculo/elipse/spline fechada).
  - Dica da fornecedor de CAD avançado: costurar o patch às vizinhas antes de sculpt/shell/thicken.
- **Outros:** CAD avançado E Fill não exige quatro lados; guias podem ser vértices ou curvas (precisas ou amostradas); continuidade por aresta (Position/Tangency/Curvature) com restrições de tipo de curva; New/Add (e Add fechando volume gera sólido). [V] CAD avançado G: N-Sided com Constraint Faces G1 e Trim to Boundary. [V: fórum/tutorial]
- **Fenkra hoje:**
  - `SurfFill{curves}` (`lib.rs:1261`) → `BRepOffsetAPI_MakeFilling` com **`GeomAbs_C0` em todas as arestas** (`surfaces.inc:54-68`). Ignora suporte, tangência, ponto de passagem e guias.
  - `CurveRef` não aceita aresta de superfície com a face-mãe associada; sketch entra só como `SketchLine/SketchCurve` e a cadeia tem de ser montada à mão (DR-13, B09).
  - Se o laço não fechar, o erro é o genérico "as curvas não formam um contorno fechado".
  - O texto da UI já anuncia G1/G2 para o Blend (`geo_ui.rs:183`), mas o Fill não os usa.
- **Melhor prática:**
  - Entrada: seleção em cadeia (sketch, curvas, arestas) com *Automatic chain*; condição por aresta (G0/G1/G2 + peso); ponto de passagem; "Fix up boundary"; "Merge/solid".
  - Para quatro lados, oferecer "otimizar" (loft).
  - Mostrar as folgas do contorno (pontos vermelhos, como o CAD avançado E).
  - Testes: fill de quadrado planar vira face planar; fill de 4 linhas de sketch (o caso que falhou na revisão); fill tangente a duas faces tem desvio angular <1° na borda; ponto de passagem fora da área gera aviso.

### 21. Multi-seções / loft de superfície (CAD avançado B: Multi-sections Surface / CAD avançado A: Lofted Surface, Boundary Surface / CAD avançado C: Loft (Surface) / outros: CAD avançado G Through Curves, CAD avançado F Boundary Blend / Fenkra: SurfLoft)

- **O que faz:** cria superfície passando por seções, com guias e/ou spine.
- **CAD avançado B:** [V]
  - Seleções: seções (as páginas do guia dizem 1 ou 2 como mínimo/máximo de seções planares ao listar tangências; a API aceita lista), **guias** opcionais (uma ou várias).
  - **Tangência** em seções e guias: direção do vetor tangente (ex.: normal de um plano) ou superfície/plano de apoio (as seções devem então ser tangentes ao suporte).
  - **Ponto de fechamento** (vértice ou extremo) em seções fechadas; por padrão ligam-se os pontos de fechamento; mudar evita torção.
  - **Spine** automática ou imposta.
  - **Relimitação** no início, no fim, nos dois ou em nenhum; sem relimitação, o limite depende do tipo de spine (spine do usuário: extremidades da spine; automática sem guias: seções de início e fim; automática com guias: extremidades das guias).
  - **Acoplamento**: seções consecutivas por abscissa curvilínea; guias via spine. Tipos: Ratio, Tangency, Tangency then curvature, Vertices (os dois do meio exigem o mesmo número de pontos).
  - Guia composta de várias curvas gera várias superfícies.
  - **Detecção de planos** (canônicas), suavização por ângulo e desvio, lei de área, saída superfície ou volume.
  - Erros comuns: terceira guia dá "plano móvel e guias nem sempre se cruzam" (guias curtas, paralelas ao plano móvel, spine muito curva). [V: fórum 3DS]
- **CAD avançado A:** [V parcial]
  - Lofted Surface: perfis, Start/End Constraints (None, Direction Vector, Normal to Profile, Tangency to Face, Curvature to Face), guide curves, centerline parameters.
  - Boundary Surface: direções 1 e 2, tipos de tangência (Tangency/Curvature to Face, Normal to Profile, Direction Vector, None), **comprimento e influência da tangência**, influência da curva (Global, To Next Curve, To Next Sharp, To Next Edge, Linear), Merge tangent faces, Close.
- **CAD avançado C:** Loft com rails, center line, area loft, condições de ponta (tangent, smooth, direction). [C]
- **Fenkra hoje:**
  - `SurfLoft{sections,guide,ruled,closed}` (`lib.rs:1309`, `hybrid.inc:130-150`): sem guia usa `ThruSections` (`CheckCompatibility`); **com guia usa PipeShell onde a guia é a spine**, não uma guia de verdade.
  - **Faltam:** várias guias, acoplamento, ponto de fechamento (torção), tangência nas pontas, spine, relimitação, orientação por seção (reverter), ordem das seções editável.
  - Erros genéricos ("confira a ordem e o sentido").
  - Benchmark SUP-11 e B12/B34 já pedem guias e restrições.
- **Melhor prática:**
  - Fluxo: seções (ordem e sentido visíveis) → guias → spine → condições nas pontas → acoplamento/pontos de fechamento → relimitação.
  - Mostrar a orientação (setas) e permitir inverter por seção; sempre mostrar o ponto de fechamento.
  - Regra para evitar torção: alinhar pontos de fechamento automaticamente pelo mínimo de distância.
  - Testes: loft círculo→círculo (cilindro); círculo→quadrado com pontos alinhados vs. desalinhados; loft com duas guias; seções em ordem trocada geram aviso de auto-interseção.

### 22. Concordância / blend (CAD avançado B: Blend / CAD avançado A: Boundary Surface (2 curvas com tangência) / CAD avançado C: Loft com rails + Bridge / Fenkra: SurfBlend, SurfConnect)

- **O que faz:** cria superfície entre duas curvas ou bordas com continuidade e tensão.
- **CAD avançado B:** [V]
  - Duas curvas (ou bordas), cada uma com **suporte** opcional.
  - **Continuidade** por limite: Ponto, Tangência, Curvatura.
  - **Trim do suporte**: aparar e juntar ao blend.
  - **Tangência na borda**: ambos os extremos, nenhum, só início, só fim.
  - **Tensão**: constante ou linear (por limite, independente); a API aceita tipos padrão, constante, linear e em S, com 2 valores no linear.
  - Pontos de fechamento e de acoplamento; acoplamento: razão, tangência, tangência-curvatura, vértices ou **spine**.
  - Modo "ruled developable surface" e suavização.
- **CAD avançado A:** em geral usa Boundary Surface com Tangency/Curvature to Face. [C]
- **CAD avançado C:** Loft com condições de ponta a partir das faces vizinhas. [C]
- **Fenkra hoje:**
  - `SurfBlend{a,sa,b,sb,cont,tension}` (`lib.rs:1317`, `blend` em `lib.rs:830-878`): reamostra as duas curvas em 24 seções, Hermite cúbica (G1) ou quíntica (G2 com curvatura **nula** nas pontas), `loft_surface`.
  - `SurfConnect{a,b,cont,tension}` (`lib.rs:1232`) escolhe sozinho as bordas que "se olham" (`facing_edge`, `lib.rs:199`) pelo menor desvio médio. Há detecção de cantos por ângulo (<0,87 rad cosseno).
  - **Riscos:**
    - o G2 do Blend é aproximado (não casa a curvatura real da superfície de apoio; a UI diz "aprox." em `geo_ui.rs:1526`);
    - a tensão é única;
    - o suporte não é aparado nem juntado;
    - o 'facing_edge' pode escolher a borda errada quando as bordas têm distância parecida;
    - sem pontos de fechamento ou acoplamento.
  - Teste existente: `lib.rs:6339-6352` (loft + blend + join + boundary).
- **Melhor prática:** tensão por lado, trim do suporte (opcional) com junção, acoplamento por razão/vértices, escolha de borda confirmada visualmente em `SurfConnect`. Testes: blend G1 entre dois planos com desvio angular medido <1°; G2 comparando curvatura; borda errada gera aviso quando duas candidatas têm distância média dentro de 10%.

### 23. Superfície regrada (CAD avançado B: Sweep Line / Ruled developable / CAD avançado A: Ruled Surface / CAD avançado C: Ruled Surface / Fenkra: SurfLoft `ruled`, SurfConnect G0, SweepKind::Linear)

- **O que faz:** superfície por retas entre bordas ou saindo de uma borda numa direção.
- **CAD avançado A:** [V parcial] Ruled Surface: Tangent to Surface, Normal to Surface, Tapered to Vector, Perpendicular to Vector, Sweep; estende a partir das arestas selecionadas. [V: snippet 2012]
- **CAD avançado C:** [V] Ruled Surface: tipos Normal, Tangent, Vector; arestas (sketch não vale); alternar faces de referência; distância e ângulo opcionais; usada em superfícies de partição de fundição.
- **CAD avançado B:** o blend tem modo "ruled developable" (`ruled_developable_surface`), e o Sweep Line cobre a regrada. [V pela API]
- **Fenkra hoje:** não há comando único. Há `ruled` no loft (`lib.rs:1309`), G0 no `SurfConnect` e `SweepKind::Linear/LinearDir` (`doc.rs:278-280`). Backlog BM-SUP-02 pede a regrada.
- **Melhor prática:** um comando "Regrada" com tipos Normal/Tangente/Vetor/Entre arestas, reaproveitando `LinearDir` (comprimento + ângulo) que já existe. Teste: regrada normal a uma aresta de cilindro = plano da aba; tangente = continuidade G1 medida.

### C. Operações

### 24. Juntar / join / knit / stitch (CAD avançado B: Join / CAD avançado A: Knit Surface / CAD avançado C: Stitch / outros: Fusion Stitch, CAD avançado G Sew, CAD avançado E Add / Fenkra: SurfJoin, CurveJoin, `join_curves`)

- **O que faz:** une elementos que se tocam numa só entidade.
- **CAD avançado B:** [V]
  - Opções: **Check tangency** (erro se não tangentes), **Check connexity** (conta os domínios conexos; ao aplicar, destaca as bordas livres), **Check manifold** (só curvas; liga connexity), **Simplify**, **Ignore erroneous elements**, **Merging distance**, **Angle tolerance** (impede juntar sobrepostos; erros marcados na geometria).
  - Comandos de contexto: **Check Solution** (interseções entre elementos), **Propagate** (elementos de mesma dimensão; forquilhas param a propagação).
  - Aba **Sub-elements to remove** e "Create join with sub-elements" (cria um segundo join com o que foi excluído).
  - API: modo de federação (`propagation`: nenhum, todos, continuidade, tangência), healing de células fundidas, desvio.
  - IBM APAR: juntar todas as superfícies com Check Tangency pode falhar com "Tangency Error". [V]
  - Texto-base: "Curvas desconexas não podem ser unidas; use Join antes de operações de superfície". [V: texto do usuário]
- **CAD avançado A:** [V parcial] Knit Surface: junta faces e superfícies; **Merge entities**; **Try to form solid** (ajusta a tolerância para tentar fechar); **Gap control** mostra pares de arestas com folga e permite subir a tolerância acima da folga. Absorve os corpos de origem.
- **CAD avançado C:** [V] Stitch: seleção (ou Select All), aba Analyze (preto = costurada, vermelho = não; magenta = quase-tangente), **Maximum Tolerance**, **Maintain As Surface**, **Find Remaining Free Edges** (lista de pares com folga máxima; "Set as Tolerance" copia o valor), iteração até zerar. Consome os insumos por padrão.
- **Outros:** o Fusion usa Stitch (surface→quilt→sólido ao fechar) e Unstitch. [V] O CAD avançado E tem New/Add (merge scope) e converte em sólido ao fechar volume. [V]
- **Fenkra hoje:**
  - `SurfJoin{items,tol}` (`lib.rs:1318-1328`): compõe e `sewn(tol)` (`hybrid.inc:152`). `CurveJoin{curves}` (`lib.rs:1212`, `join_curves` em `lib.rs:270-325`) encadeia pelas pontas com tolerância `max(1e-4·tamanho, 1e-3)` e inverte as que vierem ao contrário (bom).
  - `items` aceita só `SurfRef`; **não junta curvas de sketch** a superfícies, e quatro linhas de sketch não unem (benchmark SUP-15, B09).
  - Faltam: verificar conectividade (quantos domínios), verificar tangência, "ignorar elementos com erro", ângulo-limite, mostrar bordas livres restantes, sub-elementos, propagação.
  - O erro de `join_curves` já informa quantas curvas não tocam e a tolerância (bom padrão).
- **Melhor prática:**
  - Resultado do Join sempre relata: nº de domínios, nº de bordas livres, folga máxima. Se há bordas livres, lista com localização.
  - Modo de entrada misto (sketch + curva + face).
  - Testes: 4 linhas de sketch → curva fechada; duas esferas a 15 mm com tol 0,01 → 2 domínios (teste atual em `lib.rs:6184`); folga 0,05 com tol 0,01 → erro com "folga máxima 0,05".

### 25. Reparo / healing (CAD avançado B: Healing / CAD avançado A: Knit gap control, Import Diagnosis / CAD avançado C: Repair Bodies (construção), Stitch / Fenkra: SurfHeal)

- **O que faz:** fecha frestas e descontinuidades pequenas entre superfícies.
- **CAD avançado B:** [V]
  - Seleção por lista (modos Add/Remove persistentes por duplo clique).
  - **Merging distance** (folga máxima tratada; ex.: 1 mm) e **Distance objective** (folga máxima aceita após curar; padrão 0,001, subindo a 0,1).
  - Aba **Sharpness**: bordas vivas mantidas (só no modo tangente) e ângulo de aresta viva (padrão 0,5°).
  - Aba **Visualization**: mensagens de descontinuidade restante/alterada (ponto ><, tangência ^), filtro (todas, só não corrigidas, nenhuma), modo interativo ou sequencial.
  - API: continuidade 0 ou 1, **tangency angle/objective**, lista de elementos a **congelar** (freeze), bordas a manter vivas.
  - Dica: ativar "Surfaces' boundaries" no display para ver as folgas.
  - Texto-base: "sempre analise a curvatura após healing". [V: texto do usuário]
  - IBM: a tolerância 0,15 mm pode revelar auto-interseção; 0,2 mm esconde a irregularidade sem corrigir. [V]
- **CAD avançado A / CAD avançado C:** o reparo é feito por Knit com tolerância/Gap Control, Delete Face (Delete and Patch/Heal) e Fill; o CAD avançado C tem Repair Bodies no ambiente de construção. [C]
- **Fenkra hoje:** `SurfHeal{surface,tol}` (`lib.rs:1329`, `hybrid.inc:157-168`): costura com `tol`, `ShapeFix_Shape` (precisão `tol`, máx. `10·tol`) e `UnifySameDomain`.
  - Falta: dois parâmetros separados (distância de fusão × objetivo), tangência, bordas vivas, congelar, e **relatório do que mudou e do que restou**. Fica opaco ("curou" sem dizer o quê).
  - `max_tolerance = tol×10` pode fundir bordas legítimas — risco de alterar a geometria.
- **Melhor prática:** relatório pós-cura (folgas fechadas, folgas restantes com posição, maior deslocamento aplicado) e limite duro de deslocamento. Teste: duas faces com folga 0,02 curam com tol 0,05; folga 0,5 permanece e é listada.

### 26. Dividir / split (CAD avançado B: Split / CAD avançado A: Split Line (curva), Surface Split via Trim / CAD avançado C: Split / Fenkra: SurfSplit, SplitBody)

- **O que faz:** corta um elemento por outro e mantém um lado (ou os dois).
- **CAD avançado B:** [V pela API]
  - Elemento a cortar, **elementos de corte** (superfície, curva ou ponto; sub-elementos como face/aresta/vértice) com **orientação** por corte; **both sides** (calcula os dois lados); **intersection computation**; **automatic extrapolation** (nenhuma, tangente, curvatura); suporte opcional; resultado superfície ou volume; listas "manter" e "remover" de sub-elementos.
  - Texto-base: "prefira curvas limpas; splits em superfícies degeneradas falham". [V: texto do usuário]
- **CAD avançado A:** o Trim Surface faz o papel (ver ficha 27); Split Line separa a face por uma curva projetada. [C]
- **CAD avançado C:** Split divide partes/faces por plano, superfície ou caminho. [C]
- **Fenkra hoje:**
  - `SurfSplit{surface,tool,flip,more,curve}` (`lib.rs:1330-1356`): plano/superfície de corte aplicados em sequência com `split_side` (`hybrid.inc:172-…`); corte por curva vira uma faixa extrudada na normal.
  - **Riscos reais:**
    - `Err(_) if cut_any => {}` (`lib.rs:1351`): depois do primeiro corte bem-sucedido, **qualquer plano que falhe é ignorado em silêncio** (viola o princípio 1);
    - o corte por curva usa **uma única normal** tomada no ponto do meio (`lib.rs:1336-1340`): em superfície curva a faixa não acompanha a curva — o corte sai deslocado;
    - não há "manter os dois lados" (só `flip`);
    - a extrapolação do corte é fixa (`extend` só para `SurfRef::Face`).
  - `SplitBody` (`doc.rs:1815`) divide corpos por superfície.
- **Melhor prática:**
  - Cada elemento de corte tem estado (ok / não cruza / falhou) visível.
  - Curva: projetar na superfície (ponto a ponto, com `surface_near`) antes de cortar.
  - "Manter ambos" gera dois elementos.
  - Testes: esfera dividida por plano (metade); superfície curva cortada por curva projetada (borda dentro de 1e-3 da curva); plano que não cruza → erro nomeado.

### 27. Aparar / trim (CAD avançado B: Trim / CAD avançado A: Trim Surface / CAD avançado C: Trim / outros: Fusion Trim, CAD avançado E Mutual Trim / Fenkra: SurfTrim)

- **O que faz:** corta dois elementos um pelo outro, mantendo os lados escolhidos, e junta.
- **CAD avançado B:** dois elementos; **lado** de cada um (trocável com "Other side", "Next/Previous element" para escolher a célula); resultado montado (assemble) ou separado. [C; a página não abriu, o resumo vem da busca e da API do Split]
- **CAD avançado A:** [V parcial]
  - **Standard** (ferramenta: superfície, sketch, curva, plano) e **Mutual** (superfícies se cortam entre si).
  - **Keep selections** ou **Remove selections**: com Keep, o que não foi listado é descartado; com Remove, o que não foi listado fica.
  - Em **Surface Split Options**: **Natural** e **Linear** (só importam quando a ferramenta não atravessa a superfície; o Linear estende os limites a partir das pontas até a aresta mais próxima).
  - Pré-visualização dinâmica no Mutual. **Merge result.**
- **CAD avançado C:** [V] Trim: ferramenta pode ser quilt, face única, caminho de curvas 2D sem interseção ou plano de trabalho; seleciona as regiões a remover (a primeira define o corpo); Invert Selection. Cada trim age em um único corpo de superfície.
- **Outros:** o Fusion usa uma superfície como ferramenta e escolhe o lado; aceita sólido como ferramenta. [V] O CAD avançado E tem Mutual Trim e Move Boundary com distância negativa. [V]
- **Fenkra hoje:** `SurfTrim{a,b,flip_a,flip_b}` (`lib.rs:1357-1362`): divide A por B e B por A, mantém um lado de cada (`flip`) e costura com `sewn(1e-3)`.
  - Faltam: escolha por **célula** (quando há mais de duas regiões, `flip` é binário e não cobre), Keep/Remove, Natural/Linear (extensão), ferramenta curva ou sketch, resultado separado.
  - Tolerância de costura fixa `1e-3` (pode deixar bordas livres em modelos grandes).
- **Melhor prática:** escolher regiões clicando (lista de células, com cor), "manter"/"remover", extensão natural/linear. Testes: dois planos que se cruzam (4 regiões, manter 1); corte mútuo de duas esferas; ferramenta que não atravessa → aviso de extensão.

### 28. Contorno / boundary (CAD avançado B: Boundary / CAD avançado A: Split Line, Edge Curves / CAD avançado C: Include Geometry / Fenkra: CurveBoundary)

- **O que faz:** cria a curva da borda livre de uma superfície.
- **CAD avançado B:** [V] Seleciona uma aresta e escolhe a **propagação**: Complete boundary, Point continuity, Tangent continuity ou No propagation. Opcionalmente limita a curva por dois elementos (ex.: pontos). Selecionando a superfície inteira gera o contorno completo e a propagação some. Só limita se o contorno gerado for contínuo; setas invertem o trecho. Não se copia o Boundary para outro documento (copie a superfície primeiro).
- **CAD avançado A / CAD avançado C:** Convert Entities / Include Geometry no sketch 3D (associativo) copiam bordas. [C]
- **Fenkra hoje:** `CurveBoundary{surface,which}` (`lib.rs:1502-1507`) devolve o laço de borda livre de índice `which`. **Mesma armadilha da Interseção:** `cs.get(*which).or(cs.last())` cai em "a última" se o índice não existir. Não há aresta específica nem propagação.
- **Melhor prática:** escolher a aresta (âncora), propagação gravada (completo / ponto / tangente / nenhuma), limites. Teste: cilindro aberto tem 2 laços; trocar entre eles pela âncora; apagar o laço escolhido gera erro (não pega "o último").

### 29. Extrair com propagação (CAD avançado B: Extract / CAD avançado A: Surface Offset=0, Copy Surface, Delete Face (Keep) / CAD avançado C: Derive/Copy Object, "Select Other" / Fenkra: SurfExtract, SurfMulti, SurfPart, extract.rs)

- **O que faz:** copia uma face, aresta ou vértice de um corpo para o conjunto geométrico, mantendo ou não a associação.
- **CAD avançado B:** [V]
  - Seleciona aresta ou face; **Propagation type**: Point continuity, Tangent continuity ou No propagation.
  - **Complementary mode**: seleciona o que NÃO foi escolhido.
  - Campo **Support** aparece quando a propagação é ambígua (precisa escolher uma face).
  - Multi-seleção.
  - Limitação: extraídos não se copiam sozinhos entre documentos.
  - O Extract é usado dentro de outros comandos (ex.: via menu de contexto em qualquer campo) — o "Extract usável dentro de qualquer comando". [V pelo benchmark; menu de contexto [C]]
- **CAD avançado A:** Offset Surface com distância 0 ou "Copy Surface"; Delete Face com Keep. [V: padrão do zero no CAD avançado E; CAD avançado A [C]]
- **CAD avançado C:** o "Derive"/"Copy Object" de faces; Select Other para escolher entre sobrepostas. [C]
- **Fenkra hoje:**
  - `SurfExtract{face:[f64;3], owner}` (`lib.rs:1216` → `face_of_bodies`, `lib.rs:881`): busca o corpo cuja superfície passa pelo ponto a <1e-3 e copia a face.
  - A propagação existe só na **interface** (`extract.rs:71-160`: `Prop::{Nenhuma, Ponto, Tangente, Curvatura}` sobre a malha) e o resultado é gravado como um `SurfMulti{faces:Vec<[f64;3]>}` (`app.rs:4910`). **Não é refeita no recompute**: se o corpo muda e uma face tangente nova aparece, ela não entra.
  - Risco: pontos fixos podem pegar a face errada se duas faces passarem pelo mesmo ponto (aresta compartilhada).
  - `SurfPart{surface,index}` e `SurfUntrim` cobrem Disassemble/Untrim (ver ficha 33).
- **Melhor prática:**
  - Gravar `(owner, papel de face, propagação)` e recalcular a propagação no recompute com o mesmo critério; mostrar quantas faces entraram e avisar quando mudar.
  - "Complementar" e "Extract" em qualquer campo de seleção (menu de contexto).
  - Testes: extrair face + tangentes de um corpo com filete, aumentar o raio do filete e conferir que a contagem permanece; apagar a face-mãe gera "falhou".

### 30. Filete de superfície (CAD avançado B: Shape Fillet, Edge Fillet, Face-Face Fillet / CAD avançado A: Fillet (em superfícies) / CAD avançado C: Fillet em superfície/quilt / Fenkra: SurfFillet, SurfEdgeFillet)

- **O que faz:** arredonda a junta de duas superfícies ou uma aresta entre faces de superfície.
- **CAD avançado B:** Shape Fillet (aresta entre dois suportes), Edge Fillet, Variable Radius Fillet, Face-Face Fillet, Tritangent. Opções: raio, extremidades (liso, reto, máx., mín.), tangência, trim de suportes. [C; apostila mencionada, sem página aberta]
- **CAD avançado A / CAD avançado C:** Fillet/Face Fillet com raio constante, variável ou por corda; opção "Trim and attach". [C]
- **Fenkra hoje:**
  - `SurfFillet{a,b,flip_a,flip_b,radius,radius2}` (`lib.rs:1242-1253`): acha a maior curva de interseção, toma o ponto médio, apara cada superfície pela outra **estendida** (`split_side(..., extend=true)`), costura (`sewn(1e-3)`) e chama `edge_fillet` (`lib.rs:259-267`).
  - `SurfEdgeFillet` escolhe arestas por **pontos médios** (`edges: Vec<[f64;3]>`) — frágil se a malha muda.
  - Erro específico quando raio é grande: "raio grande demais ou arestas que não ligam duas faces".
  - Benchmark SUP-19 / DR-10: a renderização quebra em filetes de superfície; B10.
  - A tolerância das arestas `tol = 5% da diagonal` (mín. 1), usada na seleção.
- **Melhor prática:** escolher arestas por âncora (owner + papel) e testar o raio máximo possível antes de falhar ("máx. viável ≈ X mm"). Testes: filete r=5 entre duas paredes perpendiculares (teste atual em `lib.rs:5662-5667`); raio variável 2→6; r maior que a parede → erro com o máximo estimado.

### 31. Transformações: translate, rotate, symmetry, scale, affinity, axis-to-axis (CAD avançado B: Transformations / CAD avançado A: Move/Copy, Mirror, Scale / CAD avançado C: Move, Mirror, Scale / Fenkra: GeoTransform, BodyXf)

- **O que faz:** cria uma cópia transformada de um ou mais elementos, sem alterar o original.
- **CAD avançado B:** [V]
  - **Translate**: vetor por direção+distância, ponto a ponto ou coordenadas; **Repeat object after OK** (cópias a múltiplos do valor).
  - **Rotate**: eixo (linha), ângulo, repetição.
  - **Symmetry**: ponto, linha ou plano.
  - **Scale**: referência ponto, plano ou superfície plana; razão; repetição.
  - **Affinity**: sistema de eixos (origem, plano xy, eixo x) e razões em X, Y, Z.
  - **Axis to axis**: leva o elemento de um sistema de eixos para outro; opção de ocultar/mostrar o inicial.
  - Todos aceitam multi-seleção e multi-output (agrupamento editável); o original não muda.
  - Texto-base: "prefira transformações paramétricas". [V: texto do usuário]
- **CAD avançado A:** Move/Copy Bodies (translate/rotate), Mirror, Scale (ponto de origem; uniforme/não uniforme) com expressões. [C]
- **CAD avançado C:** Move Bodies, Mirror, Scale (Uniform / Non-Uniform), com eixos de trabalho. [C]
- **Fenkra hoje:**
  - `GeoTransform{source,class,xf}` (`doc.rs:671`, `lib.rs:1384-1392`): Translate, Rotate, Mirror, Scale sobre ponto, plano, curva (polilinha) ou superfície (`Shape::transformed`).
  - `BodyXf` inclui Translate/Rotate/Symmetry/**Affinity** (`doc.rs:1801-1807`), mas o wireframe **não tem Affinity** nem Axis-to-Axis.
  - Ponto ruim: `xform_dir` em planos transformados por mirror pode inverter a orientação de `normal`/`x` sem corrigir a mão — risco de um plano espelhado ter eixos de mão trocada (não testado).
  - Falta repetição (padrão de cópias).
- **Melhor prática:** repetição N (como o CAD avançado B), Affinity e Axis-to-Axis; ao espelhar planos, recompor `x` e `normal` para manter mão direita. Testes: espelhar um plano e sketchar nele (X não pode inverter); escala 2 por ponto fora da origem; rotação de superfície 90° (bbox coerente).

### 32. Extrapolar / estender (CAD avançado B: Extrapolate / CAD avançado A: Extend Surface / CAD avançado C: Extend / outros: Fusion Extend, CAD avançado E Move Boundary / Fenkra: SurfExtrapolate)

- **O que faz:** prolonga a borda de uma curva ou superfície.
- **CAD avançado B:** [V pela API via pycatia]
  - Elemento, **borda(s)** (ponto para curvas, curva para superfícies), **limite**: comprimento ou **até elemento** (superfície/volume); **continuidade**: tangente ou curvatura (por borda).
  - **Border type** na superfície: normal à borda ou tangente às arestas vizinhas.
  - **Assemble** (junta com o original); **Extend edges** (para tangente/borda tangente/montado); **constant length mode**; vários extrapolamentos em lista, com opção de usar a mesma regra nos dois lados de fios; propagação em modo curvatura de casca.
  - IBM: bugs históricos ao alternar curvatura/tangente e extrapolar até um limite longo. [V]
- **CAD avançado A:** [V] bordas ou face; **Propagate to tangent faces** (só com arestas); condição final: Distance, Up to point, Up to surface; tipo: **Same surface** (segue a geometria) ou **Linear** (tangente). Dica Javelin: o Extend costuma falhar com modelos a que faltam arestas livres planares [conteúdo não lido; 403].
- **CAD avançado C:** [V] só bordas livres; Distance ou To (face/plano); **Extend** (continua a curva das arestas vizinhas) ou **Stretch** (reta a partir das vizinhas); faces de peça não podem ser estendidas.
- **Outros:** CAD avançado E Move Boundary com Blind/Up to face/part/vertex, **Tangent propagation**, **Maintain curvature**, distância negativa apara. [V] O Fusion Extend estima a continuação pela curvatura U/V. [V]
- **Fenkra hoje:** `SurfExtrapolate{surface,near,length,curvature,more}` (`lib.rs:1376-1383`): borda mais próxima do ponto `near`, comprimento fixo, `curvature` (mesma superfície) ou tangente linear; `more` repete.
  - Faltam: **até elemento** (DR-24/B14), tipo de borda (normal × tangente às vizinhas), propagação de tangência, montar ou não, borda escolhida por âncora (não por ponto "perto").
  - O ponto `near` pode escolher a borda errada quando duas bordas estão à mesma distância.
- **Melhor prática:** `Distância` / `Até elemento` / `Até ponto`; tipo `Mesma superfície` × `Linear`; propagação opcional. Testes: estender plano 20 mm; estender cilindro por curvatura mantém raio; até plano inclinado.

### 33. Near, Disassemble, Untrim, Invert (CAD avançado B: Near, Disassemble, Untrim/Restore, Invert / CAD avançado A: Untrim Surface, Flip / CAD avançado C: Untrim / Fenkra: SurfPart, SurfUntrim; faltam Near, Invert)

- **O que faz:** utilitários para escolher uma parte, separar células, desfazer aparagens e inverter a orientação.
- **CAD avançado B:** [V]
  - **Near:** cria o sub-elemento de um multi-elemento mais próximo de um elemento de referência; abre sozinho quando detecta não conexo.
  - **Disassemble:** modos **All Cells** (cada célula vira elemento) e **Domains Only** (células conexas ficam juntas).
  - **Restore (untrim):** devolve ao limite inicial da superfície, não a um estado intermediário (para parcial, usar Undo logo após o trim).
  - **Invert Orientation:** inverte a orientação de superfície ou curva; botão vira "Reset Initial".
- **CAD avançado A/CAD avançado D/CAD avançado C:** Untrim Surface (CAD avançado A), Untrim (Fusion: todas as bordas, internas, externas ou manual). [V: Fusion; CAD avançado A [C]]
- **Fenkra hoje:** `SurfPart{surface,index}` (por **índice**, `lib.rs:1374`) e `SurfUntrim` (`lib.rs:1375`). O índice é frágil se a ordem das faces mudar após editar o pai. Não existem Near e Invert.
- **Melhor prática:** Near como padrão para escolher partes (âncora espacial) em vez de índice; Disassemble com os dois modos; orientação visível (setas) e Invert. Teste: desmontar um join de 3 elementos; extrair parte por Near ao mudar a ordem das faces.

### D. Sets, ordem e update

### 34. Geometrical Set / Ordered Geometrical Set / Part Body / In Work Object / Update e diagnóstico (CAD avançado B: Geometrical Set, Ordered Geometrical Set, Part Body, objeto de trabalho, Update, Update Diagnosis, Isolate, Publication / CAD avançado A: Surfaces folder, Rollback bar, What's Wrong / CAD avançado C: Construction folder, End of Part, Design Doctor / Fenkra: `FolderKind::Geometria`, `RefElement.folder`, `published`, `Isolated`, `Linked`)

- **O que faz:** organiza a geometria e define **onde entra o próximo comando**, em que ordem se recalcula e o que acontece quando algo falha.
- **CAD avançado B:** [V]
  - **In Work Object:** o item sublinhado define onde o próximo comando entra. Num Geoset, vai para o **fim**. Em Part Body e Ordered Geoset pode-se definir um comando como In Work Object, voltar o projeto àquele ponto e inserir no meio da pilha, o que muda toda a geometria posterior.
  - **Part Body:** geometria sólida em ordem de cima para baixo; reordenar pode mudar o resultado.
  - **Geoset (não ordenado):** geometria de superfície sem ordem fixa; os comandos são **referenciados, não consumidos**, então mudar a ordem dos filhos não muda o resultado. Só contém outros Geosets.
  - **Ordered Geoset:** de cima para baixo, como o Part Body; pode conter Part Bodies, mas não Geosets não ordenados.
  - **Reordenação:** Reorder Children (Geoset), Reorder (corpos ordenados, um por vez: After/Before/Inside), Change Geometrical Set (move entre Geosets com opções de pais), **Auto Sort** (reordena por pai/filho; recomendado antes de entregar a outro projetista).
  - **Update:** modo automático ou manual; o símbolo de update aparece ao lado da peça e a geometria afetada fica vermelha; **Local Update** atualiza um só elemento; **Stop Update** (propriedade "Associate stop update") para o recálculo num elemento com mensagem. [V]
  - **Visualização local** do update: cada elemento aparece conforme atualiza. [V]
  - **Diagnóstico e isolar:** o Update Diagnosis lista os elementos com erro; Edit>Links; **Isolate** quebra o link e deixa a geometria sem pai. O Isolate não pega todos os links (APARs). [V pela busca; opções do diálogo [C]]
  - **Multi-output:** agrupa as saídas de um comando que age em vários elementos; se não puder ser criado, é desativado automaticamente. [V]
  - **Copy/Paste:** boundaries e extracts não podem ser copiados sozinhos (copie o pai). [V]
- **CAD avançado A:** a barra de rollback e "What's Wrong" mostram o erro; superfícies e sólidos ficam na mesma árvore, em ordem de criação (Surface Bodies folder). Pastas organizam a árvore. [V parcial: dica GoEngineer sobre pastas] "Insert Part" e "Delete Bodies" como feature suprimível. [V]
- **CAD avançado C:** a barra "End of Part" marca o ponto de inserção; o Design Doctor lista falhas; as superfícies ficam em **Construction** e no ambiente de construção as análises são temporárias. [V parcial: análises no construction]
- **Fenkra hoje:**
  - Elementos de referência (`RefElement`, `doc.rs:384-398`) com `folder` (pasta de geometria) e `published`. Não há linha do tempo na pasta (`doc.rs:1281`): é um Geoset sem ordem.
  - Avaliação por demanda com **memória** (`lib.rs:921-927`) e profundidade; a ordem física é irrelevante (bom).
  - `Linked{ext}` e `Isolated{geo}` (`doc.rs:703-706`) são o par "referência externa / isolada", análogo a links + Isolate. O link tem estado (`LinkState`: Ativo, Congelado, Quebrado, Desativado — `doc.rs:437-445`) como pede o ADR 0011.
  - Faltam: **Ordered Set** (para sequências em que o resultado depende da ordem), **In Work Object** de geometria (colocar o novo elemento depois de X), **Auto Sort**, **Local Update**, **Stop Update**, e diagnóstico por elemento (B59).
  - Erros viram `String` nomeada (ex.: "'X' não é uma superfície", `lib.rs:774`) — um bom começo para o diagnóstico, mas sem as ações Editar/Substituir/Isolar/Desativar/Reordenar do princípio 1.
- **Melhor prática:**
  - Manter o Geoset (sem ordem) como padrão e acrescentar o **Conjunto Ordenado** para quando a ordem importa (ex.: trims sucessivos).
  - Barra "em trabalho" visível, com o item sublinhado, e inserção depois dele.
  - Diagnóstico por elemento com o motivo e as cinco ações; Isolar converte para `Isolated`.
  - Testes: apagar o pai gera "falhou" com a lista dos filhos afetados; Isolar mantém a geometria; reordenar no Geoset não muda o resultado; reordenar no Ordenado muda.

### E. Análises

### 35. Curvatura de superfície e pente de curvatura (CAD avançado B: Surfacic Curvature Analysis, Porcupine Curvature Analysis / CAD avançado A: Curvature display, Curvature Comb / CAD avançado C: Surface Analysis (Gaussian/Mean/Max), Curvature Comb / Fenkra: modo Curvatura do shader)

- **O que faz:** mostra a curvatura como mapa de cores (superfície) ou pente (curva).
- **CAD avançado B:** [V parcial] Surfacic Curvature mapeia a curvatura na superfície. Porcupine pode ser aplicado a curvas ou bordas de superfícies; mostra curvatura ou raio; parâmetros **Type, Density, Amplitude**. APARs: cores erradas quando a precisão 3D fixa é 0,02/0,01; amplitude muda após Undo.
- **CAD avançado A:** [V] Curvature Display (View>Display>Curvature) pinta pela curvatura local; gradiente suave na aresta indica C2, salto indica C0/C1; a faixa de cores é ajustável em Document Properties.
- **CAD avançado C:** [V] Surface Analysis com Gaussian, Mean ou Max; **Curvature Comb** com espinhos mais longos onde há mais curvatura; funciona em faces, superfícies, curvas de sketch e arestas; a densidade é ajustável; mudar a qualidade de exibição afeta todas as análises salvas.
- **Fenkra hoje:** `DisplayMode::Curvatura` (`render.rs:199-224`; shader em `render.rs:81-91`): usa `k = |fwidth(normal)| / |fwidth(pos)|` por pixel, escala logarítmica e rampa de 5 cores.
  - É uma **estimativa em espaço de tela**: depende da resolução e do zoom; sem legenda, sem tipo (Gaussiana/Média/Máx.), sem unidade. Sem pente em curvas.
  - Aviso do ADR 0011: "a qualidade gráfica nunca altera o modelo" — aqui o contrário: a qualidade da malha altera a análise.
- **Melhor prática:** curvatura calculada no kernel por face (`BRepLProp`/`GeomLProp_SLProps`) e enviada como atributo por vértice; legenda com min/máx e unidade 1/mm; tipos Gaussiana/Média/Máx./Mín.; pente com densidade e escala. Testes: esfera r=10 → curvatura média 0,1 mm⁻¹ constante; cilindro → Gaussiana 0; plano → 0.

### 36. Zebra (CAD avançado B: Zebra via Shading with material / CAD avançado A: Zebra Stripes / CAD avançado C: Zebra (Smoothness) / Fenkra: modo Zebra)

- **O que faz:** projeta listras refletidas para ver a continuidade de superfícies.
- **CAD avançado B:** [V parcial] as análises exigem "Shading with Material"; o texto-base manda usar zebra para validar continuidade. [V: texto do usuário]
- **CAD avançado A:** [V] View>Display>Zebra Stripes; as listras aparecem **uma ordem abaixo** da continuidade testada: C0 as listras não se alinham; C1 alinham com quina; C2 alinham e passam suaves.
- **CAD avançado C:** [V] Zebra (Smoothness): G0 visível, G1 mesma direção com mudança de curvatura notável, G2 liso (tangência e curvatura iguais).
- **Fenkra hoje:** `DisplayMode::Zebra` (`render.rs:92-100`): faixas em `reflect(-v, n).z × 9`, suavizadas por `fwidth`. Número, direção e largura de listras fixos; o ambiente é "horizontal em Z".
- **Melhor prática:** parâmetros de densidade, direção (X/Y/Z/radial), largura e opção de ambiente esférico; **ficar fixa ao modelo**, não à câmera (opção). Teste de captura visual (B53): duas faces G1 e G2 produzem listras com quina × contínuas na junção.

### 37. Análise de ângulo de saída / draft (CAD avançado B: Draft Analysis / CAD avançado A: Draft Analysis / CAD avançado C: Draft Analysis / Fenkra: ausente)

- **O que faz:** colore faces conforme o ângulo entre a normal e a direção de desmoldagem.
- **CAD avançado B:** [V parcial] exibe só em Shading with Material; resultado incorreto com ângulo ≤3° (APAR).
- **CAD avançado A:** [V] o ângulo mínimo aceitável vem de uma caixa; menores aparecem como "Requires Draft"; as cores são temporárias.
- **CAD avançado C:** [V] verifica a inclinação para fundição segundo a direção de puxada; espectro de cor; testa direções alternativas.
- **Fenkra hoje:** não existe; o draft de sólido existe (`draft_tagged`, `fenkra-kernel/src/lib.rs:412`) mas sem análise.
- **Melhor prática:** análise por normal de face × direção (sem depender da malha), com limiar, "undercut" e escolha da direção por plano/aresta/eixo. Teste: cubo com faces laterais a 0° → todas "precisa de saída"; com 2° e limiar 1° → OK.

### 38. Connect checker / verificação de continuidade (CAD avançado B: Connect Checker / CAD avançado A: Deviation Analysis, zebra / CAD avançado C: Zebra / Fenkra: ausente)

- **O que faz:** mede distância (G0), ângulo (G1) e diferença de curvatura (G2) entre superfícies ou curvas vizinhas.
- **CAD avançado B:** [V parcial] o Connect Checker mede a distância mínima entre vértices, o ângulo entre as superfícies e a diferença de curvatura; há versão para curvas; a curvatura aparece em % (máx. 200%, documentação pouco clara). O Healing Assistant tem limites em mm, graus e %. APARs mostram que o resultado depende de um limiar angular.
- **CAD avançado A:** [V] **Deviation Analysis**: seleciona as superfícies que se encontram numa aresta e calcula ângulo mín./médio/máx.; para C1 todos devem ser ≈0. Só dá número de tangência, não de curvatura (comentário externo).
- **CAD avançado C:** [V] a zebra classifica G0/G1/G2 visualmente; não há medidor numérico equivalente [C].
- **Fenkra hoje:** não existe; a propagação de `extract.rs` (`end_props`, limiar de tangência cos > 0,9986 ≈ 3° e curvatura ±15%) é um embrião de métrica, mas só para propagar seleção.
- **Melhor prática:** `analise_continuidade(aresta)` no kernel (amostra a borda, mede gap, ângulo e Δ curvatura), com limites configuráveis e legenda verde/amarelo/vermelho sobre a aresta. Testes: G0 (gap 0), G1 (ângulo <0,5°), G2 (Δk <5%); dois planos a 90° → ângulo 90°.

### 39. Distância / desvio (CAD avançado B: Distance Analysis / CAD avançado A: Compare, Deviation / Fenkra: ausente; existe `measure.rs`)

- **O que faz:** mapa de cores com a distância entre dois conjuntos (superfície×superfície, ponto×superfície).
- **CAD avançado B:** o Distance Analysis aparece junto de connect checker e porcupine como ferramenta para otimizar curvatura e tolerância. [V parcial: apenas listagem de curso]
- **CAD avançado A:** o Deviation Analysis do CAD avançado A é angular; a distância é feita por Measure/Check Entity [V parcial].
- **Fenkra hoje:** `measure.rs` mede distâncias pontuais; não há mapa de cores de distância. Requisito RF-SUP-021 de `docs/requisitos/superficies.md` pede o desvio com mapa de cores.
- **Melhor prática:** grade de amostras na superfície A com distância à B (`BRepExtrema_DistShapeShape`), mapa de cores com legenda e máximo/mínimo/média. Teste: duas esferas concêntricas r=10 e r=12 → distância uniforme 2.

### F. Sólido e Classe A

### 40. Espessamento (CAD avançado B: Thick Surface (Part Design) / CAD avançado A: Thicken / CAD avançado C: Thicken/Offset (Solid) / outros: CAD avançado E Thicken / Fenkra: FeatureKind::ThickSurface)

- **O que faz:** transforma uma superfície em sólido com espessura em um ou dois lados.
- **CAD avançado A:** [V] Thicken: lado 1, lado 2 ou ambos; **Direction of Thicken** (linear, planos, eixos, cilindros — desde a versão 2020); **Create solid from enclosed volume** só quando as superfícies fecham um volume; exige costurar antes se há várias peças; Thicken Cut. 
- **CAD avançado C:** [V] o resultado Solid do Thicken/Offset cria o espessamento; fóruns relatam que o espessamento falha por faces e que combinar com Patch/Stitch antes resolve.
- **Outros:** CAD avançado E Thicken converte em sólido e adiciona/remove material; o Fusion recomenda aparar/estender até o plano de partição antes do Thicken. [V]
- **Fenkra hoje:** `ThickSurface{surface,t1,t2,op}` (`doc.rs:1822-1829`, `lib.rs:4041-4046`): `thicken(t1,t2)` e depois `apply(op)`. Erro específico se o raio de curvatura é menor que a espessura (`hybrid.inc:295`).
  - Faltam: direção diferente da normal, "espessar só células fechadas" e um teste de lado para superfície com normal invertida (relação `flip`).
- **Melhor prática:** direção escolhida, soma opcional com sólido existente, aviso "raio mínimo = X". Testes: cilindro aberto t=2 (teste atual `lib.rs:6372`); t maior que o raio interno → erro com raio mínimo.

### 41. Fechamento e costura em sólido (CAD avançado B: Close Surface, Sew Surface / CAD avançado A: Knit→Create solid, Cut With Surface / CAD avançado C: Stitch→solid, Sculpt / outros: Fusion Stitch, CAD avançado E Enclose / Fenkra: CloseSurface, SewSurface, BoundaryFill, LinkedBody)

- **O que faz:** converte um conjunto de superfícies que fecham um volume em sólido e o combina com o corpo.
- **CAD avançado B:** Close Surface (gera sólido de superfície fechada) e Sew Surface (costura ao corpo, adicionando ou removendo material). [C]
- **CAD avançado A:** Knit com "Create solid" / "Try to form solid"; Surface Cut. [V parcial]
- **CAD avançado C:** [V] Stitch converte para sólido ao fechar (a menos que "Maintain As Surface"); **Sculpt** com Add, Remove, New Solid, ladeamento por superfície, pré-visualização verde.
- **Outros:** CAD avançado E **Enclose**: seleciona todas as fronteiras ao redor de um espaço vazio para formar sólido; "Keep tools". Fusion: Stitch fecha e converte. [V]
- **Fenkra hoje:** `CloseSurface{surface,tol,op}` (`lib.rs:4047`), `SewSurface{surface,remove,flip}` (`lib.rs:4053-4065`: remove usa `split_side`; adicionar usa `volume_cells` do composto corpo+superfície), `BoundaryFill{tools,exclude,op}` (`lib.rs:4066`: células de volume, com pontos de exclusão — equivale ao Enclose / Boundary Fill do CAD avançado B), `LinkedBody` (costura casca fechada com tol fixa 0,01, `lib.rs:4098`).
  - Ponto forte: `BoundaryFill` com **células excluídas por ponto**. Fraco: os pontos de exclusão são fixos e mudam de significado quando o modelo muda.
  - O Sew "adicionar" usa `volume_cells(&[])`; se houver mais de uma célula, todas entram.
- **Melhor prática:** mostrar as células (cores) para escolher, gravar âncora e não coordenada, e relatar "volume fechado? sim/não, bordas livres restantes". Testes: caixa de 6 faces costurada vira sólido; faltando uma face → relatório de borda livre; exclusão de célula interna.

### 42. Superfícies de classe A / FreeStyle (CAD avançado B: FreeStyle, Imagine & Shape / CAD avançado F: ISDX Style, Freestyle / CAD avançado G: Studio Surface / Fusion: Form / Fenkra: fora do escopo; só referência)

- **O que faz:** modelagem de superfícies de estilo (carroceria, design industrial) por curvas e pontos de controle, com continuidade G2/G3 estrita e controle de curvatura.
- **Referências:**
  - O CAD avançado B tem o FreeStyle Profiler 2 (FSP) como espaço de trabalho dedicado. [V: link na busca]
  - O CAD avançado F tem Style (ISDX) e Freestyle, esta baseada em subdivisão e que "serve para formas conceituais, mas é mais difícil em superfícies analíticas e arestas vivas". [V: busca]
  - O CAD avançado G usa Through Curves Mesh com tangência, N-Sided e Studio Surface. [V parcial]
  - O Fusion tem Form (T-Splines), com "Top 5 Surfacing Tips for Class A". [V: título]
  - O texto-base recomenda grau 5 para classe A. [V: texto do usuário]
- **Decisão sugerida:** manter fora do escopo agora. O futuro está em `RF-SUP-030..033` (gaiola de controle e verificação de patentes). Só garantir que a infraestrutura NURBS exata (RF-SUP-01) exista para não bloquear.

---

## 3. Requisitos levantados

Prioridades: P0 = corrige bug/violação de princípio; P1 = paridade essencial; P2 = melhora importante; P3 = desejável.

### Funcionais

| ID | Requisito | Prior. | Critério de aceite |
|---|---|---|---|
| RF-SUP-01 | Curvas com representação analítica (círculo, NURBS) preservada ao passar por extrusão, revolução, sweep e loft. | P1 | Extrudar um `Curve3` com `circle` gera face de tipo cilindro (`face_props.kind`); um tubo `SweepProfile::Circle` também. |
| RF-SUP-02 | `which` de `CurveIntersect` e `CurveBoundary` troca o índice por âncora espacial; nunca cai em "a última". | P0 | Teste: duas curvas, apagar a escolhida → erro nomeado; antes dava a última silenciosamente (`lib.rs:1395,1504`). |
| RF-SUP-03 | `SurfSplit` não ignora corte que falha; cada elemento de corte tem estado visível (ok/não cruza/falhou). | P0 | Corte com plano que não cruza, depois de um corte ok, retorna aviso por elemento (hoje `lib.rs:1351` ignora). |
| RF-SUP-04 | Corte por curva em `SurfSplit` projeta a curva na superfície ponto a ponto. | P1 | Curva sobre cilindro corta a ≤1e-3 da curva; hoje usa uma normal única (`lib.rs:1336-1340`). |
| RF-SUP-05 | Projeção direcional informa pontos perdidos; resultado parcial vira estado "parcial". | P0 | Projeção de curva maior que o suporte mostra "N de M pontos"; hoje descarta em silêncio (`lib.rs:1426-1430`). |
| RF-SUP-06 | `PointOnSurface` projeta de volta na superfície e aceita (u, v) reais. | P1 | Ponto em cilindro a (u,v) está a <1e-6 do cilindro; mudar o raio mantém o ponto sobre ele. |
| RF-SUP-07 | Referências a face/aresta usam `owner` + papel (ADR 0010). `SurfRef::Face`, `SurfExtract`, `SurfEdgeFillet`, `CurveRef::Edge`. | P0 | Apagar a face-mãe gera "falhou" com diagnóstico; duas faces no mesmo ponto não são confundidas. |
| RF-SUP-08 | Estado de erro por elemento na pasta de geometria (falhou / parcial / ok) com ações Editar, Substituir, Isolar, Desativar, Reordenar. | P0 | Apagar o pai de um fill mostra o fill em vermelho com as 5 ações; Isolar converte para `Isolated`. |
| RF-SUP-09 | Conjunto Ordenado e "em trabalho" para geometria. | P2 | Inserir um elemento "depois de X"; reordenar no Ordenado muda o resultado, no Geoset não. |
| RF-SUP-10 | Auto Sort e Local Update para geometria. | P3 | Auto Sort reordena por pai/filho; Local Update recalcula só um elemento e seus filhos. |
| RF-SUP-11 | Ponta de linha e limites de superfície aceitam "até elemento" (plano, superfície, ponto). | P1 | `LinePoints` até plano; `SurfExtrude`, `SurfExtrapolate` até plano inclinado seguem o plano ao movê-lo (B14). |
| RF-SUP-12 | Extrusão e revolução com limites 1 e 2 (e ângulo 1 e 2). | P1 | Extrusão assimétrica 10/5 → comprimento total 15; revolução 90°/90°. |
| RF-SUP-13 | `SurfFill` aceita sketch, curvas e arestas em cadeia, com condição por aresta (G0/G1/G2 + peso) e ponto de passagem. | P0 | Quatro linhas de sketch geram fill; borda tangente a duas faces com desvio angular <1° (B09, DR-13). |
| RF-SUP-14 | Fill com "Fix up boundary", pontos vermelhos nas folgas e opção de mesclar/fechar sólido. | P2 | Contorno com folga 0,1 mostra as pontas e a correção aplicada. |
| RF-SUP-15 | Loft com várias guias, spine, ponto de fechamento e acoplamento (ratio, tangência, vértices). | P1 | Loft círculo→quadrado sem torção com pontos de fechamento alinhados; duas guias respeitadas a <1e-3 (B34). |
| RF-SUP-16 | Loft/Blend com condição de ponta (tangente à face) e tensão por lado. | P1 | Desvio angular medido na borda <1° (G1). |
| RF-SUP-17 | Blend: trim e junção opcional do suporte; G2 usando curvatura real do suporte. | P2 | Diferença de curvatura na junção <5%. |
| RF-SUP-18 | Sweep com superfície de referência, ângulo, relimitadores (ponto ou plano) e posição do perfil. | P2 | Perfil mantém o ângulo com a superfície de referência; varredura termina no plano relimitador. |
| RF-SUP-19 | Modo "Fixo" do sweep com direção escolhida (hoje Z fixo, `hybrid.inc:120`). | P1 | Sweep fixo com direção X mantém o perfil com a mesma orientação. |
| RF-SUP-20 | Comando Regrada com Normal/Tangente/Vetor/Entre arestas (BM-SUP-02). | P2 | Regrada normal a aresta de cilindro = superfície plana; tangente com G1 medido. |
| RF-SUP-21 | Join relata nº de domínios conexos, bordas livres e folga máxima; opções Check tangency/connexity e ignorar erros. | P1 | Duas esferas separadas → 2 domínios; folga acima da tolerância → erro com a folga. |
| RF-SUP-22 | Join aceita curvas de sketch e faces misturadas (B09). | P0 | 4 linhas de sketch → curva fechada utilizável como perfil. |
| RF-SUP-23 | Healing com distância de fusão × objetivo, bordas vivas, congelar e relatório do que mudou/restou. | P2 | Após curar, lista as folgas restantes com posição; deslocamento máximo ≤ limite. |
| RF-SUP-24 | Trim escolhe regiões (células) e permite manter ou remover; extensão natural × linear. | P1 | Dois planos que se cruzam: 4 regiões, manter 1; ferramenta curta com extensão linear corta. |
| RF-SUP-25 | Boundary com propagação gravada (completo, ponto, tangente, nenhuma) e limites. | P1 | Extrair contorno tangente de superfície com filete; mudar o raio mantém a cadeia. |
| RF-SUP-26 | Extract refaz a propagação no recompute e informa a contagem de faces/arestas. | P0 | Aumentar o raio do filete mantém a mesma contagem; se mudar, aviso (DR-14). |
| RF-SUP-27 | "Extrair" disponível no menu de contexto de qualquer campo de seleção, com modo Complementar. | P2 | Em Fill e Join, botão direito → Extrair adiciona o elemento à lista. |
| RF-SUP-28 | Filete de superfície: seleção de arestas por âncora e aviso de raio máximo viável. | P1 | Raio maior que a parede → erro com o máximo estimado; render sem riscos (B10). |
| RF-SUP-29 | Extrapolar com "até elemento", tipo de borda (normal × tangente às vizinhas), propagação e montar. | P1 | Estender até plano inclinado; estender cilindro por curvatura mantém raio. |
| RF-SUP-30 | Near, Disassemble (tudo/domínios) e Invert orientação. | P2 | Desmontar join de 3 elementos; `SurfPart` por Near sobrevive à reordenação. |
| RF-SUP-31 | Transformações de wireframe: Affinity, Axis-to-Axis e repetição N; plano espelhado mantém mão direita. | P2 | Sketch num plano espelhado não inverte X; repetir translação 5 vezes gera 5 cópias. |
| RF-SUP-32 | Curva paralela com tipo de canto (agudo/arredondado), modo geodésico e lei (linear/S). | P1 | Offset externo de retângulo tem cantos arredondados; no cilindro, geodésico e euclidiano diferem. |
| RF-SUP-33 | Spline com tangente/curvatura por ponto, tensão, grau e "geometria no suporte". | P1 | Spline com tangente imposta respeita o ângulo ±0,5°; projeção sobre plano coincide com o plano. |
| RF-SUP-34 | Hélice com ponto/ângulo inicial, entrada por voltas, passo variável e transições. | P2 | Hélice de 3 voltas termina no ângulo esperado; início escolhido pelo usuário. |
| RF-SUP-35 | Espiral com ângulo inicial, passo e sentido. | P3 | Espiral com passo radial constante medido. |
| RF-SUP-36 | Combine Curve e Reflect Line (linha de partição). | P3 | Combine de duas vistas gera curva cujas projeções coincidem; Reflect na esfera = equador. |
| RF-SUP-37 | Pontos repetidos numa curva, ponto extremo e ponto geodésico. | P3 | 5 pontos equidistantes em curva de 100 mm a 25 mm entre si. |
| RF-SUP-38 | Offset de superfície com distância 0 (cópia), repetição e aviso de raio mínimo. | P2 | Offset 0 devolve a mesma área; offset maior que o raio dá erro com o raio mínimo. |
| RF-SUP-39 | Curvatura por face calculada no kernel (Gaussiana, Média, Máx., Mín.) com legenda e unidade. | P1 | Esfera r=10 → 0,1 mm⁻¹; cilindro → Gaussiana 0; resultado não depende do zoom. |
| RF-SUP-40 | Pente de curvatura em curvas e bordas, com densidade e amplitude. | P2 | Círculo r=20 → espinhos de comprimento uniforme proporcional a 0,05 mm⁻¹. |
| RF-SUP-41 | Zebra com densidade, direção e largura; opção de fixar ao modelo. | P2 | Duas faces G1 e G2: listras com quina × contínuas (captura visual do corpus B53). |
| RF-SUP-42 | Análise de ângulo de saída (draft) por direção, limiar e undercut. | P2 | Cubo com laterais verticais, limiar 1° → laterais "precisam de saída". |
| RF-SUP-43 | Connect checker numérico (gap, ângulo, Δ curvatura) por aresta. | P1 | Dois planos a 90° → 90°; G2 verifica Δk<5%. |
| RF-SUP-44 | Mapa de distância/desvio entre superfícies (RF-SUP-021). | P2 | Esferas concêntricas r=10 e 12 → 2 mm uniforme. |
| RF-SUP-45 | Espessamento com direção escolhida e aviso do raio mínimo; fechamento/costura relatam volume fechado e bordas livres. | P2 | Caixa de 6 faces → sólido; faltando uma → relatório da borda livre. |
| RF-SUP-46 | `BoundaryFill`/`SewSurface` gravam âncoras das células, não coordenadas, e mostram as células. | P2 | Mover a esfera interna mantém a célula excluída. |
| RF-SUP-47 | Corpus de teste do tema (B53): uma peça por comando com recompute, medidas, mudar-e-voltar e captura. | P0 | Cada comando da seção 2 tem ao menos uma peça de corpus verde. |

### Não funcionais

| ID | Requisito | Prior. | Critério de aceite |
|---|---|---|---|
| RNF-SUP-01 | Nenhum comando escolhe outra geometria em silêncio; falha é estado visível. | P0 | Revisão de código: nenhum `.or(cs.last())`, `Err(_) => {}` ou descarte sem aviso no avaliador de referências. |
| RNF-SUP-02 | Recompute de superfícies determinístico: mesmas entradas → mesma topologia e mesma ordem de faces. | P1 | Recompute duas vezes dá o mesmo hash de contagem de faces e área. |
| RNF-SUP-03 | Tolerâncias explícitas e proporcionais ao tamanho do modelo (nada de `1e-3` fixo). | P1 | `sewn(1e-3)` em `SurfTrim`/`SurfFillet` passa a ser derivada de `model_size`. |
| RNF-SUP-04 | Qualidade de malha/exibição não altera o modelo nem as análises. | P1 | Mudar a qualidade gráfica não altera o mapa de curvatura. |
| RNF-SUP-05 | Mensagens de erro nomeiam o campo e sugerem a ação. | P1 | Ex.: "comprimento 2 = 0" em vez de "comprimento zero". |
| RNF-SUP-06 | Recompute de uma peça com 100 elementos de superfície em <2 s (memória de avaliação mantida). | P2 | Medição no corpus. |
| RNF-SUP-07 | Arquivos salvos antigos continuam abrindo (campos novos com `#[serde(default)]`). | P0 | Abrir um `.fenkra` anterior sem erro (padrão já usado em `doc.rs`). |
| RNF-SUP-08 | Painéis na ordem suporte → modo → parâmetros → avançado (B20). | P1 | Revisão visual de todos os painéis de geometria. |
| RNF-SUP-09 | Textos em português, ícones e dicas em dois estágios (B60). | P2 | Auditoria de `i18n.rs` para o tema. |
| RNF-SUP-10 | Análises em tempo interativo (zebra/curvatura a 60 fps em malha de 1 M de triângulos). | P2 | Medição no corpus. |

---

## 4. Itens de backlog propostos

Tamanho: P = pequeno (≤1 dia), M = médio (2–4 dias), G = grande (>1 semana). IDs "Bnn" são os do `docs/backlog/backlog.md`; os novos usam `SUP-nn` até a numeração oficial.

| Título | O que muda | Prior. | Tam. | Dependências |
|---|---|---|---|---|
| SUP-01 Fim das escolhas silenciosas | `which` por âncora (Intersect/Boundary), Split sem ignorar falhas, projeção com contagem de perdas (RF-SUP-02/03/05, RNF-SUP-01) | P0 | M | B59 |
| SUP-02 Referências estáveis de face/aresta | `owner`+papel em `SurfRef::Face`, `SurfExtract`, `SurfEdgeFillet`, `CurveRef::Edge` (RF-SUP-07) | P0 | G | ADR 0010, 0004 |
| SUP-03 Diagnóstico por elemento da geometria | Estado falhou/parcial/ok e as 5 ações (RF-SUP-08) | P0 | M | B59 |
| SUP-04 Fill/Join com sketch e cadeia | Conclui B09 e acrescenta condição por aresta e ponto de passagem (RF-SUP-13/22) | P0 | G | B09 |
| SUP-05 Extract com propagação refeita | Propagação gravada e refeita no recompute (RF-SUP-25/26/27) | P0 | M | DR-14 |
| SUP-06 Curva analítica | `Curve3` com NURBS/círculo exato, spline verdadeira, cilindro de extrusão de círculo (RF-SUP-01/33) | P1 | G | SUP-02 |
| SUP-07 Limites "até elemento" | Linhas, extrusão, revolução e extrapolação (RF-SUP-11/12/29) | P1 | M | B14 |
| SUP-08 Loft completo | Guias, spine, fechamento, acoplamento, tangência (RF-SUP-15/16) | P1 | G | B12, B34 |
| SUP-09 Blend com trim e G2 real | Tensão por lado, trim do suporte, curvatura real (RF-SUP-16/17) | P2 | M | SUP-06 |
| SUP-10 Sweep avançado | Superfície de referência, ângulo, relimitadores, Fixo com direção (RF-SUP-18/19) | P2 | G | SUP-06 |
| SUP-11 Join com relatório | Domínios, bordas livres, verificações (RF-SUP-21) | P1 | M | SUP-04 |
| SUP-12 Healing com diagnóstico | Dois parâmetros, bordas vivas, relatório (RF-SUP-23) | P2 | M | SUP-11 |
| SUP-13 Trim/Split por região | Células, manter/remover, natural × linear, projeção da curva (RF-SUP-04/24) | P1 | G | SUP-01 |
| SUP-14 Curva paralela completa | Cantos, geodésico, lei (RF-SUP-32) | P1 | M | SUP-06 |
| SUP-15 Hélice, espiral e spline | Ponto inicial, voltas, passo variável, tangência/curvatura por ponto (RF-SUP-33/34/35) | P2 | M | SUP-06 |
| SUP-16 Regrada, Near, Disassemble, Invert | Comandos novos (RF-SUP-20/30) | P2 | M | SUP-05 |
| SUP-17 Transformações completas | Affinity, Axis-to-Axis, repetição, plano espelhado (RF-SUP-31) | P2 | P | — |
| SUP-18 Combine e Reflect Line | Curvas novas (RF-SUP-36) | P3 | M | SUP-06 |
| SUP-19 Curvatura e pente no kernel | Curvatura por face e pente com legenda (RF-SUP-39/40) | P1 | G | RNF-SUP-04 |
| SUP-20 Connect checker e desvio | Gap/ângulo/curvatura por aresta e mapa de distância (RF-SUP-43/44) | P1 | G | SUP-19 |
| SUP-21 Zebra e draft configuráveis | Parâmetros e análise de ângulo de saída (RF-SUP-41/42) | P2 | M | SUP-19 |
| SUP-22 Conjuntos ordenados | Ordered Set, em trabalho, Auto Sort (RF-SUP-09/10) | P2 | G | B54, B59 |
| SUP-23 Sólido a partir de superfície | Direção do espessamento, relatório de fechamento, âncoras de células (RF-SUP-45/46) | P2 | M | SUP-02 |
| SUP-24 Corpus de superfícies | Uma peça por comando, com medidas e captura (RF-SUP-47) | P0 | G | B53 |

---

## 5. Fontes

Todas lidas nesta rodada, salvo onde indicado como "só trecho de busca".

**CAD avançado B (guia da City University, espelho da ajuda oficial de Wireframe & Surface)**
- (fonte em procedencia.md) (Multi-sections)
- …/wfsug_C2/wfsugbt0403.htm (Join)
- …/wfsug_C2/wfsugbt0204.htm (Offset), …/wfsugbt0206.htm (Fill), …/wfsugbt0203.htm (Sweep), …/wfsugbt0202.htm (Blend), …/wfsugbt0201.htm (Extrude)
- …/wfsug_C2/wfsugbt0207.htm (Extract), …/wfsugbt0106.htm (Boundary), …/wfsugbt0101.htm (Projection), …/wfsugbt0102.htm (Spline)
- …/wfsug_C2/wfsugbt0402.htm (Invert), …/wfsugbt0404.htm (Translate), …/wfsugbt0405.htm (Symmetry), …/wfsugbt0406.htm (Scale), …/wfsugbt0407.htm (Rotate), …/wfsugbt0408.htm (Affinity), …/wfsugbt0409.htm (Near)
- …/wfsug_C2/wfsugbt0412.htm (Restore), …/wfsugbt0413.htm (Disassemble), …/wfsugbt0414.htm (Healing), …/wfsugbt0415.htm (Axis to axis)
- …/wfsug_C2/wfsugbt0301.htm (Update), …/wfsugbt0302.htm (Work on Support), …/wfsugbt0303.htm (Multi-output), …/wfsugbt0306.htm (Copy/Paste)
- …/wfsug_C2/wfsugbt0400.htm (índice de operações: Split, Trim, Extrapolate, Untrim)
- …/kwrug_C2/kwrughlpWfConstructors.htm (assinaturas de intersect, curveparallel, corner, spline)

**CAD avançado B (API CAA via pycatia, para opções avançadas)**
- (fonte em procedencia.md) com as páginas: `hybrid_shape_curve_par.html`, `hybrid_shape_extrapol.html`, `hybrid_shape_corner.html`, `hybrid_shape_connect.html`, `hybrid_shape_healing.html`, `hybrid_shape_loft.html`, `hybrid_shape_blend.html`, `hybrid_shape_fill.html`, `hybrid_shape_assemble.html`, `hybrid_shape_sweep_explicit.html`, `hybrid_shape_split.html`, `hybrid_shape_helix.html`, `hybrid_shape_reflect_line.html`

**CAD avançado B (outros)**
- (fonte em procedencia.md) (In Work Object, Geoset, Ordered Geoset)
- Só trecho de busca: (fonte em procedencia.md) ; https://www.ibm.com/support/pages/apar/HD62498 ; …/HD78478 ; …/HD65362 ; …/HD45548 ; …/HD83718 ; …/HD76702 ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md)

**CAD avançado A** (help.CAD avançado A.com carrega por JavaScript e não pôde ser lido; usei os trechos das buscas e blogs)
- Só trecho de busca: (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md)
- (fonte em procedencia.md) (zebra, curvatura, Deviation Analysis)
- (fonte em procedencia.md)
- (fonte em procedencia.md)

**CAD avançado C**
- (fonte em procedencia.md) (Boundary Patch)
- (fonte em procedencia.md) (Patch, Stitch, Ruled)
- (fonte em procedencia.md) (Trim, Extend, Sculpt)
- (fonte em procedencia.md) (curvas 3D)
- (fonte em procedencia.md) (análises)
- Só trecho de busca: (fonte em procedencia.md) ; (fonte em procedencia.md)

**Outros**
- (fonte em procedencia.md) (CAD avançado D)
- (fonte em procedencia.md) ; (fonte em procedencia.md) (CAD avançado E)
- Só trecho de busca: (fonte em procedencia.md) ; (fonte em procedencia.md) ; (fonte em procedencia.md) ; https://mechanical-engineering.com/nx-tutorial-surface-n-sided-through-curves-commands/ ; (fonte em procedencia.md) ; (fonte em procedencia.md)

**Código e documentos do repositório**
- `crates/fenkra-solidos/src/lib.rs`, `geo3d.rs`; `crates/fenkra-core/src/doc.rs`; `crates/fenkra-kernel/cpp/surfaces.inc`, `hybrid.inc`, `kernel.cpp`; `crates/fenkra-app/src/render.rs`, `extract.rs`, `geo_ui.rs`
- `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`; `docs/benchmark/superficies-e-wireframe.md`; `docs/contextos/superficies.md`; `docs/requisitos/superficies.md`; `docs/backlog/backlog.md`

---

## 6. Handoff

- **Resultado:** relatório completo com 42 fichas, 47 RF, 10 RNF, 24 itens de backlog e fontes.
- **Evidência:** código lido em `lib.rs` (1195–1540, 4035–4105, 1973–2100), `geo3d.rs` (55–140, 325–590), `doc.rs` (60–720, 1262–1300, 1800–1870), `surfaces.inc`, `hybrid.inc`, `kernel.cpp` (160–230), `render.rs` (70–120), `extract.rs` (1–120). As fichas citam arquivo:linha.
- **Arquivos alterados:** nenhum. O plano de leitura pedia só consulta, e o relatório é a resposta.
- **Checagens rodadas:** nenhuma de build ou teste (só leitura).
- **Falhas e limites:**
  - `help.CAD avançado A.com` é renderizado por JavaScript e não abriu; usei trechos das buscas e blogs. Javelin e GrabCAD deram 403.
  - Os PDFs do NIAR (Wichita) deram 404.
  - As páginas de Split, Trim e Extrapolate da City University também deram 404; para elas usei a API CAA (pycatia) e as listagens.
  - `spine_wire` (kernel) não foi lido na íntegra; a afirmação de que o círculo do `Sweep` vira B-spline interpolada, e não cilindro, é **provável**, não confirmada.
  - Itens sobre CAD avançado F e CAD avançado G vêm só de resumos de busca, de fonte secundária.
- **Riscos:** os achados de "bug provável" (silêncios em `which`/Split/projeção, `PointOnSurface` no plano tangente, corte por curva com normal única, ponto inicial da hélice) foram deduzidos da leitura e **não reproduzidos**. Convém reproduzi-los com testes antes de priorizar.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra em superfícies e wireframe são: (1) curvas só como polilinhas, sem representação analítica ou NURBS exata, então G1/G2 são aproximados; (2) referências de face e aresta fixas por ponto, sem nome topológico nem diagnóstico por elemento; (3) escolhas silenciosas no avaliador (`which` em Boundary/Interseção, Split que ignora falhas, projeção que descarta pontos); (4) Fill, Join, Healing, Trim, Extrapolate, Loft e Sweep sem os modos de CAD avançado B/CAD avançado A/CAD avançado C (suporte e continuidade no Fill, "até elemento", relimitadores, guias, acoplamento, relatório de conectividade); (5) sem Ordered Set, In Work Object de geometria nem análises de continuidade, draft e desvio (zebra e curvatura são só shader de tela).
