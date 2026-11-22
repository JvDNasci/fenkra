# Grafo do Fenkra

Mapa navegável de como o Fenkra se organiza: bancadas, crates, ciclo de um comando, comandos por bancada
(entradas, motor, saídas, dependentes) e o modelo do documento. Fonte: os mapas de 21/11/2026 em
[comandos/](comandos/) e a [auditoria de seleção](../benchmark/detalhe/selecao-auditoria.md). O contrato
que estes grafos devem passar a seguir é o [ADR 0107](adr/0107-contrato-de-feature.md).

Convenções: `arquivo:linha` é a linha da `develop` de 21/11 e muda a cada edição; a função é a âncora
estável. Arquivos sem pasta: `fenkra-app/src/` (interface), `fenkra-solidos/src/` (motor, `S/lib.rs`),
`fenkra-core/src/` (modelo, `doc.rs`), `fenkra-kernel/cpp/` (`*.inc`, `kernel.cpp`), todos em `crates/`.
O grafo de código gerado por AST (graphify, sem LLM) fica em `crates/graphify-out/` (fora do git).

Mantido pelo agente `arquiteto`: quem mudar um comando de forma que altere entradas, motor ou saídas
atualiza o mapa da bancada em `comandos/` e o grafo 3 aqui.

---

## 1. Bancadas → crates

```mermaid
flowchart LR
    subgraph Bancadas
      SK[Sketch]
      SO[Sólidos]
      SU[Superfícies e wireframe]
      CH[Chapa metálica]
      ES[Estrutural e soldas]
      MO[Montagem]
      DE[Desenho]
      PM[PMI 3D]
      BI[Biblioteca de normas]
      PL[PDM / PLM]
      IN[Interface]
      KE[Kernel]
      MA[Malha]
    end
    subgraph Crates
      APP["fenkra-app (egui, wgpu, painéis)"]
      CORE["fenkra-core (documento, sketch, solver, referências)"]
      SOL["fenkra-solidos (recompute)"]
      KER["fenkra-kernel (fachada cxx do OCCT)"]
      MAL["fenkra-malha"]
      MON["fenkra-montagem"]
      DES["fenkra-desenho"]
      PRO["fenkra-project"]
      OCCT[("OpenCASCADE 8.0.1")]
    end
    SK --> CORE
    SK --> APP
    SO --> SOL
    SU --> SOL
    CH --> SOL
    ES --> SOL
    BI --> SOL
    MO --> MON
    DE --> DES
    PM --> CORE
    PL --> PRO
    IN --> APP
    KE --> KER
    MA --> MAL
    APP --> CORE
    APP --> SOL
    APP --> MON
    APP --> DES
    APP --> PRO
    SOL --> CORE
    SOL --> KER
    SOL --> MAL
    MON --> SOL
    DES --> SOL
    DES --> KER
    KER --> OCCT
```

| Bancada | Arquivos principais (domínio da skill) |
|---|---|
| Sketch | `fenkra-core/src/sketch.rs`, `fenkra-app/src/sketch_ui.rs`, `sketch_ext.rs`, `sketch_snap.rs` |
| Sólidos | `S/lib.rs`, `profile.rs`, `refs.rs`, `termino.rs`, `previa_leve.rs`; `features.inc`; `feat_ui.rs`, `editor.rs` |
| Superfícies | `geo3d.rs`, `loft.rs`, `regioes.rs`; `surfaces.inc`, `hybrid.inc`, `nurbs.inc`; `geo_ui.rs`, `sup3d.rs` |
| Chapa | `sheet.inc`; trechos de `S/lib.rs`, `chapa_*.rs`; `feat_ui.rs`, `chapa_*_ui.rs` |
| Estrutural | `structural.rs`, `st_*.rs`; `fenkra-montagem/src/estrutura.rs`; `st_ui.rs`, `st_*_ui.rs` |
| Montagem | `fenkra-montagem/src/` (`solve.rs`, `kin.rs`, `proposta.rs`, `bom.rs`); `asm_ui.rs`, `asm_cmd.rs` |
| Desenho | `fenkra-desenho/src/`; `drawing.inc`; `dwg_ui.rs` |
| PMI | `fenkra-core/src/pmi.rs`; `pmi_ui.rs`, `pmi_draw.rs`; `kernel/src/pmi_xde.rs` |
| Biblioteca | `biblioteca.rs`, `hardware.rs`, `fixadores.rs`, `engrenagens.rs` |
| PLM | `fenkra-project/src/` |
| Interface | `toolbar.rs`, `theme.rs`, `icons.rs`, `i18n.rs`, `picking.rs`, `render.rs`, `campos.rs`, `caixa_selecao.rs` |
| Kernel | `fenkra-kernel/src/lib.rs`, `ffi.rs`, `cpp/kernel.cpp`, `cpp/*.inc` |

---

## 2. Ciclo de vida de um comando

```mermaid
flowchart TD
    A["Abrir: ícone, atalho ou busca<br/>App::command (app.rs:2468)"] --> A2{"tem sessão?"}
    A2 -- "PMI, Desenho, Montagem, Forma, Medir" --> X["ciclo próprio<br/>pmi_ui, dwg_ui, asm_ui, forma_ui, medir_ui"]
    A2 -- sim --> B["FeatureKind com a seleção atual como seed;<br/>vazio = sentinela UNSET_* (slots.rs:12)"]
    B --> C["Document::add_feature + start_session<br/>(app.rs:3226): snapshot, is_new, corpos_antes"]
    C --> D["Painel: editor::show (editor.rs:686)<br/>feat_ui::panel (feat_ui.rs:270) | feature_ui (editor.rs:1539) | geo_ui::panel (geo_ui.rs:1415)"]
    D --> E["Campo ativo: campos.rs (153,192,234)<br/>+ entradas::campo_ativo (entradas.rs:99) + slots::fill (slots.rs:288)"]
    E --> F["Clique no 3D: picking::pick (picking.rs:255)<br/>cascata de cliques (app.rs:7850-8000)"]
    F --> G["fill_from_selection (app.rs:6695)<br/>ou painel_consome (campos.rs:333)"]
    G --> H["geo_stale_from (app.rs:8677) → request_recompute (app.rs:1380)<br/>→ Worker (app.rs:122)"]
    H --> I["recompute_cached (S/lib.rs:6977), Cache::leve = comando aberto<br/>Ctx::run / run_feature (S/lib.rs:4698/4709) → fenkra-kernel"]
    I --> J["Prévia: previa::da_sessao + decorar (previa.rs:83,127)<br/>Ctx::previa_leve (previa_leve.rs:215), handles/*.rs"]
    J --> K{"Concluir / Esc / outro comando"}
    K -- Concluir --> L["conclude_session (app.rs:3289):<br/>hide_consumed, recompute completo sem leve"]
    K -- Esc --> M["esc_sessao::decidir → cancel_session (app.rs:3322)<br/>restaura snapshot"]
    K -- "sai sem concluir" --> N["sair_da_sessao (app.rs:3276)<br/>descartar_ao_sair (selecao.rs:157)"]
    L --> O["Árvore: feature_row (app.rs:5877), badge por Status<br/>arvore_nivel.rs (gerador contém o gerado)"]
    O --> P["Erro novo → wizard_window (app.rs:4642)<br/>reparo_ui::lacunas (reparo_ui.rs:28), reparo::aplicar (reparo.rs:163)"]
```

Pontos que o ADR 0107 muda neste ciclo: E passa a vir do descritor (uma regra só); F e G viram um caminho
só para 3D e árvore; L passa a esperar o resultado e não fechar em falha; I ganha rollback em `Err`.

---

## 3. Comandos por bancada: entradas → motor → saídas → dependentes

Leitura: cada caixa de comando mostra o `FeatureKind`/`RefKind` e a âncora do motor; a seta para o
kernel cita a função C++ (arquivo `.inc`). Saídas e dependentes à direita.

### 3.1 Sólidos

```mermaid
flowchart LR
    subgraph Entradas
      sk[Sketch inteiro: Id]
      ax[AxisRef / PointRef]
      fa["Faces: ponto + FaceKey"]
      ed["Arestas: EdgeSel + EdgeKey"]
      pl[PlaneRef]
      sf[SurfRef]
      co[Coletor ferramenta]
    end
    subgraph Comandos
      ext["Extrude (doc.rs:4066)<br/>S/lib.rs:4828 → termino.rs:191"]
      rev["Revolve (doc.rs:4111)<br/>S/lib.rs:4873"]
      hol["Hole (doc.rs:4136)<br/>S/lib.rs:4895"]
      swp["Sweep (doc.rs:3783)<br/>S/lib.rs:5144 → varrer"]
      lof["Loft (doc.rs:3793)<br/>S/lib.rs:5182"]
      edg["Fillet/Chamfer EdgeOp (doc.rs:4323)<br/>S/lib.rs:6790-6925"]
      she["Shell (doc.rs:3891)<br/>S/lib.rs:5267"]
      dra["Draft (doc.rs:3912)<br/>S/lib.rs:5299"]
      pat["Pattern / Mirror (doc.rs:3965/3976)<br/>replicate (S/lib.rs:4667)"]
      thk["ThickSurface (doc.rs:3092)<br/>S/lib.rs:6694"]
      cmb["Combine (doc.rs:4004)<br/>S/lib.rs:6676"]
    end
    subgraph Kernel
      k1["extrude_roles / extrude_taper / extrude_until<br/>kernel.cpp:496, features.inc:823, kernel.cpp:861"]
      k2["revolve_roles (kernel.cpp:511)"]
      k3["hole_tool_ext (kernel.cpp:586)"]
      k4["sweep (features.inc:242)"]
      k5["loft (features.inc:442)"]
      k6["edges_tagged / fillet / chamfer (features.inc:995-1130)"]
      k7["shell_tagged (features.inc:504-613)"]
      k8["draft_tagged (features.inc:659)"]
      k9["thicken (hybrid.inc:838-931)"]
      kb["boolean_tagged_src (kernel lib.rs:618)"]
    end
    subgraph Saídas
      body["Corpo no coletor: Shape + donos + papéis"]
      prom["Promoted: chaves v2"]
      steps["steps / tools (ferramentas gravadas)"]
    end
    subgraph Dependentes
      dep["Filete, casca, padrão, furos seguintes,<br/>sketch em face, desenho, PMI"]
    end
    sk --> ext & rev & swp & lof & hol
    ax --> ext & rev & hol & pat
    fa --> she & dra & hol
    ed --> edg
    pl --> pat & dra
    sf --> thk
    co --> cmb
    ext --> k1
    rev --> k2
    hol --> k3
    swp --> k4
    lof --> k5
    edg --> k6
    she --> k7
    dra --> k8
    thk --> k9
    cmb --> kb
    k1 & k2 & k3 & k4 & k5 --> kb
    kb --> body
    k6 & k7 & k8 & k9 --> body
    ext & hol --> steps
    steps --> pat
    body --> prom
    body --> dep
```

### 3.2 Superfícies e wireframe (`RefKind`, pasta Referências)

```mermaid
flowchart LR
    subgraph Entradas
      cr["CurveRef: SketchCurve, Ref, BodyEdge{key}, Edge(pts)"]
      sr["SurfRef: Plane, Face{key}, Ref"]
      pr[PointRef / AxisRef / PlaneRef]
    end
    subgraph Curva
      c3["Curve3 = polilinha densa (geo3d.rs:62)<br/>profile::sample, spline_tan, edge_at (hybrid.inc:662)"]
      sw["spine_wire / guia_suave (features.inc:45-73)<br/>BSpline só se nenhum canto > 20°"]
    end
    subgraph Comandos
      sx["SurfExtract / SurfMulti / CurveExtract<br/>S/lib.rs:1899, 2850, 1996"]
      se["SurfExtrude / SurfRevolve<br/>S/lib.rs:2612, 2641"]
      ss["SurfSweep / SurfSweepX<br/>S/lib.rs:3661"]
      sl["SurfLoft → loft::superficie ou loft_surface"]
      sf2["SurfFill → fill (S/lib.rs:2031)"]
      so["SurfOffset"]
      sb["SurfBlend / SurfConnect → blend (S/lib.rs:1785)"]
      st["SurfSplit / SurfTrim / SurfRegions"]
      sj["SurfJoin / CurveJoin (join_curves S/lib.rs:550)"]
    end
    subgraph Kernel
      ka["extrude_curve / revolve_curve (surfaces.inc:457/466)"]
      kb2["sweep_surface (hybrid.inc:155)"]
      kc["loft_surface (hybrid.inc:177)"]
      kd["fill_surface_x (surfaces.inc:289)"]
      ke["offset_surface_j (surfaces.inc:193)"]
      kf["bezier_faces (surfaces.inc:375)"]
      kg["split_side / split_regions (hybrid.inc:402/445)"]
      kh["sewn (hybrid.inc:43)"]
      ki["face_propagate (hybrid.inc:732)"]
    end
    subgraph Saídas
      out["Geo::Surface(Shape) / Geo::Curve / Point / Plane<br/>RefViz + RefNote (setas, soluções)"]
    end
    subgraph Dependentes
      dd["SurfRef::Ref(id) em: espessar, costurar, dividir corpo,<br/>aba por superfície, extrusão até superfície, outras superfícies"]
    end
    cr --> c3 --> sw
    sw --> ka & kb2 & kc & kd
    sr --> so & st & sj & sx
    pr --> se
    sx --> ki
    se --> ka
    ss --> kb2
    sl --> kc
    sf2 --> kd
    so --> ke
    sb --> kf
    st --> kg
    sj --> kh
    ka & kb2 & kc & kd & ke & kf & kg & kh & ki --> out
    out --> dd
```

O nó "Curva" é o ponto do B559: a polilinha densa entre a entrada e o kernel. O ADR 0107 (seção 4) troca
por curva exata.

### 3.3 Chapa metálica

```mermaid
flowchart LR
    subgraph Entradas
      sks[Sketch: Id]
      eds["Aresta: ponto + edge_key"]
      lps["Arestas do laço: edge_keys"]
      sus["Suporte SurfRef + face (só ponto) + LimitesAba"]
      lin["Linha a-b ancorada / PlaneRef"]
      par["SheetParams: t, raio, K, material"]
    end
    subgraph Comandos
      wal["SmWall (doc.rs:2988) S/lib.rs:5345"]
      fla["SmFlange (doc.rs:3004) run_flange → sheet_tool (S/lib.rs:3984)"]
      cfl["SmCurveFlange (doc.rs:2665) curve_flange_lim (S/lib.rs:4196)"]
      flp["SmFlangeLoop (doc.rs:2630) chapa_laco.rs:335"]
      pfl["SmProfileFlange (doc.rs:2600) S/lib.rs:5913"]
      sfl["SmSurfFlange (doc.rs:2708) S/lib.rs:5966-6133"]
      hem["SmHem (doc.rs:3043) S/lib.rs:6181"]
      ben["SmBend / SmJoggle / SmLouver<br/>bend_line (S/lib.rs:3926), 5391, 5626"]
      cut["SmCut / SmStamp / SmFlangedHole<br/>S/lib.rs:5860, 5661, 5812"]
      cnv["SmConvert chapa_converte.rs:227"]
      cor["SmCornerRelief / SmCornerClose / SmArcSlot<br/>chapa_canto.rs"]
    end
    subgraph Kernel
      kf1["Shape::face + extrude_roles"]
      kf2["sm_flange (sheet.inc:51)"]
      kf3["sm_chain_sweep (sheet.inc:154)"]
      kf4["poly_prism (sheet.inc:260)"]
      kf5["section + split_side (hybrid.inc:402)"]
      kf6["thicken (hybrid.inc:868)"]
    end
    subgraph Saídas
      bod[Corpo de chapa com papéis]
      smap["SheetMap por dobra (FeatEntry::sheet_maps)"]
      lst[last_sheet]
      cuto[cut_outlines]
    end
    subgraph Dependentes
      dps["Bainha, cantos, desdobrar parcial,<br/>recorte através da dobra, planificada, DXF,<br/>tabela de dobras, desenho planificado"]
    end
    par --> wal & fla & cfl & flp & pfl & sfl & hem & ben
    sks --> wal & pfl & cut & ben
    eds --> fla & cfl & hem
    lps --> flp
    sus --> sfl
    lin --> ben
    wal --> kf1
    wal -. de superfície .-> kf6
    fla & hem --> kf2
    flp --> fla
    flp --> cfl
    cfl --> kf3
    sfl --> kf5 --> cfl
    ben --> kf4
    cut --> kf4
    kf1 & kf2 & kf3 & kf4 --> bod
    fla & cfl & hem & ben --> smap
    fla --> lst
    cut --> cuto
    cnv --> smap
    bod --> dps
    smap --> dps
    lst --> cor
```

A aba por superfície reaproveita `curve_flange_lim` depois de cortar o suporte pelo plano da web; a OML
sai como polilinha de ~200 pontos (causa provável do facetado do B559 na chapa).

### 3.4 Estrutural e soldas

```mermaid
flowchart LR
    subgraph Entradas
      gui["Linhas guia: Vec CurveRef (curves_list)"]
      sec["Seção do catálogo (st_painel::seletor)"]
      pon["Ponta do membro: Ancora PontaMembro"]
      enc["Ponto do encontro (âncora só se ponta)"]
      fws["Dois conjuntos de faces + FaceKey"]
    end
    subgraph Comandos
      mem["StructMember (doc.rs:2016)<br/>members() S/lib.rs:3304, member_shape 3364"]
      joi["StructJoint (doc.rs:2050)<br/>joint_rule, st_cantos.rs"]
      pla["StructPlate (doc.rs:2196) st_placas.rs:127"]
      gus["StructGusset (doc.rs:2263) st_placas.rs:189"]
      wel["StructWeld (doc.rs:2350) st_solda.rs:307"]
    end
    subgraph Kernel
      ks1["Shape::face + extrude / sweep_tool (ffi.rs:231)"]
      ks2["clip_until (surfaces.inc:594) + offset_surface"]
      ks3["poly_prism"]
      ks4["section_pts (ffi.rs:316) + from_mesh"]
    end
    subgraph Saídas
      pec["Peças separadas (add_separate)"]
      log["log_struct → cut_list (S/lib.rs:8190)"]
      fab["Laser, DSTV, IFC, encaixe (st_encaixe.rs)"]
      cord[CordaoCalculado]
    end
    subgraph Dependentes
      dpe["Placa, reforço, solda, cantos,<br/>montagem de estrutura (fenkra-montagem/src/estrutura.rs), desenho"]
    end
    gui --> mem
    sec --> mem
    mem --> joi
    pon --> pla
    enc --> gus
    fws --> wel
    mem --> ks1
    joi --> ks2
    pla --> ks1
    gus --> ks3
    wel --> ks4
    ks1 & ks2 & ks3 --> pec
    ks4 --> cord
    pec --> log --> fab
    pec --> dpe
```

### 3.5 Montagem, Desenho e PMI (ciclos próprios)

```mermaid
flowchart LR
    subgraph Montagem
      mr["MateRef{comp, geom, viva} x2<br/>proposta.rs:19"] --> mc["Comando::resolver em cópia (proposta.rs:288)"]
      mc --> ms["solve (fenkra-montagem/src/solve.rs:352)<br/>LM por aglomerados (ADR 0046)"]
      ms --> mp["Placement por componente + SolveReport"]
      mp --> mb["BOM, explosão, estudo, desenho"]
    end
    subgraph Desenho
      dv["View: Source (.fkp/.fka), direção, escala"] --> dp["Models::proj (source.rs:366)"]
      dp --> dh["hlr exata (drawing.inc:72) ou rápida (hlr_rapida.rs)"]
      dh --> dd2["Arestas visíveis/ocultas, cortes (drawing.inc:208)"]
      dd2 --> dc["Cotas com âncoras 3D, balões pela BOM"]
    end
    subgraph PMI
      pt["app.picks → PmiTarget (classe, ponto, direção, raio)"] --> pm["Pmi::measure / diagnose (pmi.rs:1380)"]
      pm --> px["STEP (kernel/src/pmi_xde.rs), Dim.pmi no desenho"]
    end
```

---

## 4. Documento e referências

```mermaid
classDiagram
    class Document {
      features: Vec~Feature~  linha de tempo (doc.rs:4461)
      references: Vec~RefElement~  pasta Referências
      collectors: Vec~BodyCollector~
      params: Vec~Parameter~
      sheet: Option~SheetParams~
      rollback: usize
      refs_of() doc.rs:5333
      dependents() doc.rs:5463
      delete() doc.rs:5531
    }
    class Feature {
      id, name, kind: FeatureKind
      suppressed, suppress_if
      target: Id  coletor
      ancoras: Vec~Ancora~
      flat_space
    }
    class FeatureKind {
      cerca de 89 variantes (doc.rs:1843)
      Sketch Extrude Revolve Hole EdgeOp Shell ...
      Sm* chapa, Struct* estrutural, Forma
      walk() doc.rs:3322
    }
    class RefElement {
      id, name, kind: RefKind, visible
    }
    class RefKind {
      planos, pontos, linhas, curvas (doc.rs:613)
      SurfExtract CurveExtract CurveBoundary
      Surf* CurveJoin SurfJoin
    }
    class PlaneRef {
      Origin
      Datum(Id)
    }
    class AxisRef {
      Origin, SketchLine, Ref, Fixed
      Face(point, key), Edge(mid, key), Planes
    }
    class PointRef {
      Origin, Ref, Fixed, SketchPoint, CurveVertex
      Vertex(point, edge), Center(point, edge), OnFace(point, key)
    }
    class CurveRef {
      Axis, Ref, SketchLine, SketchCurve, Segment
      Edge(pts), BodyEdge(pts, key)
    }
    class SurfRef {
      Plane, Ref
      Face(point, normal, owner, key)
    }
    class FaceKey {
      owner: Id
      role: i32
      kind, normal_or_axis, area
    }
    class EdgeKey {
      a: FaceKey
      b: FaceKey
      edge: Option~u32~
    }
    class Ancora {
      campo, ponto
      chave: ChaveAncora
    }
    class Promoted {
      Edges, Faces, Furo, Ancoras
      Ate, Caminhos, Referencia
    }
    Document "1" --> "*" Feature
    Document "1" --> "*" RefElement
    Feature --> FeatureKind
    Feature --> Ancora
    RefElement --> RefKind
    FeatureKind ..> PlaneRef
    FeatureKind ..> AxisRef
    FeatureKind ..> PointRef
    FeatureKind ..> CurveRef
    FeatureKind ..> SurfRef
    RefKind ..> CurveRef
    RefKind ..> SurfRef
    CurveRef ..> EdgeKey
    SurfRef ..> FaceKey
    AxisRef ..> FaceKey
    EdgeKey --> FaceKey
    Ancora ..> EdgeKey
    Promoted ..> FaceKey : recompute grava chaves novas
```

Chaves v2 (ADR 0010): a face é reencontrada por **dono** (comando que a criou) → **papel** (tampa 0,
final 1, lateral 2+i, `PAPEL_SUPERFICIE + i`) → **assinatura** (tipo, normal ou eixo, área) → ponto
clicado só como reserva (`refs::find_face`/`find_edge`, refs.rs:205/225). Se não sobra exatamente uma
entidade, o comando vai a erro com sugestão (`Ctx::resolve_faces`, S/lib.rs:3440). Os papéis nascem em
`extrude_roles`/`revolve_roles`, `boolean_tagged_src` e `papeis.rs`.

Hoje a chave é guardada de cinco jeitos (listas paralelas, `*_key`, dentro do `*Ref`, `Feature::ancoras`,
`RefKind{key}`); o ADR 0107 (seção 1) unifica num só.

```mermaid
flowchart LR
    D[Document] --> O["order: sketches livres, depois features"]
    O --> SIG["sig por comando (S/lib.rs:7177): feature_sig + vars_sig<br/>+ body_sig + sig dos citados (walk) + all_bodies se cita Face/Vertex"]
    SIG --> C{"Cache::feats[id].sig igual?"}
    C -- sim --> R[reaproveita corpo, status, promoted, sheet_maps]
    C -- não --> RUN["Ctx::run → kernel"]
    D --> REFS["references: Ctx::geo sob demanda (S/lib.rs:2183)<br/>+ laço próprio (S/lib.rs:7358-7430), Cache::refs"]
    D --> G["global: external, sheet (inclui flat), coletores<br/>mudou → refaz tudo"]
    G --> SIG
```
