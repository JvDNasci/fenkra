# Registro de procedência

Registre aqui toda fonte consultada para escrever especificações ou código. Isso documenta que
a implementação partiu de especificações próprias.

**Regras:** não copiar texto, imagens, ícones ou screenshots de documentação de terceiros.
Resumir com nossas palavras. Documentação que exige login: ler os termos de uso antes.

| Data | Spec / módulo | Fonte (título + URL) | Tipo | Licença / termos | O que foi extraído (resumo) |
|---|---|---|---|---|---|
| 2026-10-29 | Benchmark (todos os módulos) | 33 páginas da ajuda pública do SOLIDWORKS (help.solidworks.com) e do Inventor (help.autodesk.com) — lista em [benchmark/fontes.md](../benchmark/fontes.md) | doc. fabricante | acesso público, sem login | Nomes dos comandos e comportamento geral (entradas, modos, opções), reescritos |
| 2026-10-29 | Benchmark (CATIA V5) | Apostilas abertas da Wichita State NIAR, City University London, docs da API pública (pycatia), tutoriais GrabCAD/PTC — lista em [benchmark/fontes.md](../benchmark/fontes.md) | material didático / doc. de API | acesso público; doc. oficial do V5 exige login e não foi usada | Nomes das barras e comandos, ordem do fluxo, reescritos |
| 2026-10-29 | Benchmark (PDM/PLM) | Comparativos G2 (Vault × SOLIDWORKS PDM) | artigo | público | Só a lista de capacidades (check-in/out, estados); fonte fraca |
| 2026-11-04 | Benchmark de detalhe (chapa) | help.solidworks.com (Edge Flange, Sketched Bend, Hem, Jogs, Closed Corner), help.autodesk.com Inventor (Flange, Cuts, Bend Order, Sheet Metal Defaults, Corners), goengineer.com jog, cad.onshape.com sheetmetalflange, wiki.freecad.org SheetMetal_Workbench e SheetMetal_Unfold | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/chapa.md |
| 2026-11-04 | Benchmark de detalhe (biblioteca) | help.solidworks.com (Smart Fasteners, Hole Series), help.autodesk.com Inventor (Bolted Connection, Spur Gears, Content Center, Shaft, Retaining Ring) e Fusion (Insert Fastener), cad.onshape.com standard_content, wiki.freecad.org Fasteners_Workbench e Fasteners_Search | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/biblioteca.md |
| 2026-11-04 | Benchmark de detalhe (pmi) | help.autodesk.com Inventor (General Annotations), autodesk.com/learn (3D annotations), help.solidworks.com (DimXpert Size e Location), blogs.sw.siemens.com (NX PMI), support.ptc.com Creo (Annotation Orientation, Driving Dimension), wichita.edu NIAR (3D Tolerancing), 3dswym (FTA), wiki.freecad.org TechDraw | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/pmi.md |
| 2026-11-04 | Benchmark de detalhe (estrutural) | help.solidworks.com (Structural Member, Corner Management, Trim/Extend, Gusset, End Cap, Weld Bead, Pierce Points, Cut Lists), help.autodesk.com Inventor (Frame Generator, End Treatments, Weld Bead), cad.onshape.com frame, wiki.freecad.org Arch_Structure e Arch_Profile | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/estrutural.md |
| 2026-11-04 | Benchmark de detalhe (desenho) | wiki.freecad.org TechDraw_View e TechDraw_Dimension, help.autodesk.com Inventor (Drawing View, Section Views, Edit Dimension, Balloons), help.solidworks.com (Ordinate, Dimension Value, Detail View, Hole Callouts), cad.onshape.com Drawing (callout_balloon, construction tools), forum.onshape.com | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/desenho.md |
| 2026-11-04 | Benchmark de detalhe (plm) | pypi b123d-recognisers, cadquery-mcp, pycatia; dspace.kaist.ac.kr 151839; iso.org 34697; anvil-wiki AnalysisSitus; OCCTSwift FeatureRecognition; github hedless/onshape-mcp, AutodeskAILab/Fusion360GalleryDataset; onshape-public docs featureaccess; archestra.ai MCP; arxiv 2410.05340, 2105.09492; help.solidworks.com API; help.autodesk.com Inventor-API; wiki.freecad.org Part_Import/Export; community.ptc.com; store.trimech.com; amsterdam.aitinkerers.org; 3dswym license-terms; qdchenshi.pages.dev | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/plm.md |
| 2026-11-04 | Benchmark de detalhe (solidos) | help.autodesk.com Fusion (Extrude, Offset Plane) e Inventor (Fillet), help.solidworks.com (Extrude, Revolve, Shell), cad.onshape.com (fillet, hole, shell, draft, revolve, linear pattern, loft, chamfer, boolean), onshape.com tech-tips (loft twist, chamfer), wiki.freecad.org PartDesign_Pad e PartDesign_Pocket | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/solidos.md |
| 2026-11-04 | Benchmark de detalhe (interface) | docs.blender.org (pie menus, fields, help, undo, search), help.autodesk.com Inventor (Marking Menu, toolclips, API tooltips) e Fusion API RadialMarkingMenu, wiki.freecad.org Part_Extrude, docs.rs egui_extras | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/interface.md |
| 2026-11-04 | Benchmark de detalhe (sketch) | help.autodesk.com Inventor (Dynamic Input, Split/Trim/Extend, Constraints, Sketch Tab) e Fusion (Sketch Palette, Trim/Extend), forums.autodesk.com (offset), knowledge.autodesk.com, cad.onshape.com sketch-tools-dimension, help.solidworks.com (Numeric Input, Lines, Fillet, Offset, Polygon, Shortcut Bars), javelin-tech.com, wiki.freecad.org Sketcher_Preferences, reqrefusion.github.io FreeCAD-Documentation | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/sketch.md |
| 2026-11-04 | Benchmark de detalhe (montagem) | help.solidworks.com (Mate Alignment, Interference, Component Patterns, Explode, Collision, Mate Controller), help.autodesk.com Inventor (Place Constraint, Joints, Bolted Connection, BOM) e Fusion (Joint), cad.onshape.com (mate connector, fastened), wiki.freecad.org Assembly_CreateJointFixed | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/montagem.md |
| 2026-11-04 | Benchmark de detalhe (superficies) | docs.blender.org (Proportional Editing, Subdivision, Mirror, Loop Cut), wiki.freecad.org (Surface, GeomFillSurface, Curves), github Roy-043 FreeCAD-documentation, help.autodesk.com Fusion (Sculpt, Patch) e AutoCAD SURFOFFSET, cad.onshape.com (loft, fill, move boundary, boundary), onshape.com tech-tips, help.solidworks.com (Offset, Boundary, Knit, Trim), support.ptc.com Creo Offset, docs.mcneel.com MatchSrf, rhino3d.com continuity, discourse.mcneel.com, core.ac.uk 77409617, help.altair.com NURBS Editing | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/superficies.md |
| 2026-11-04 | Benchmark de detalhe (kernel) | dev.opencascade.org refman 8.0.0 (IMeshTools_Parameters, OSD_ThreadPool), occt3d.com guia de malha, fórum OCCT, github emilk/egui discussão 4062, ref.rerun.io wgpu Backends, learn.microsoft.com deployment Visual C++, doc.rust-lang.org runtime, wiki.freecad.org PartDesign Preferences e FAQ, tracker.freecad.org 216 e 2002, forum.onshape.com 282 | ajuda pública, artigo, fórum | público | Interação, campos e prévia, resumidos com nossas palavras em docs/benchmark/detalhe/kernel.md |
| 2026-11-05 | Estilo visual (B311) | digitalengineering247.com (Engineering the software user experience), extensions.blender.org (temas Minimal Dark e Plasticity), discourse.shapr3d.com (tema escuro) | artigo, página pública, fórum | público | Ideias gerais: revelar aos poucos, menos distração, contraste legível; sem copiar telas |

## Legenda dos rótulos neutros do benchmark

Fora deste arquivo e de `docs/benchmark/fontes.md`, os documentos não citam produtos de terceiros
(regra do projeto, B310). Onde o benchmark compara "CAD avançado A, B, C…", a correspondência é:

| Rótulo | Software consultado (documentação pública) |
|---|---|
| CAD avançado A | SolidWorks |
| CAD avançado B | CATIA V5 |
| CAD avançado C | Inventor |
| CAD avançado D | Fusion 360 |
| CAD avançado E | Onshape |
| CAD avançado F | Creo |
| CAD avançado G | NX |
| CAD avançado H | Solid Edge |
| CAD avançado I | Rhino |
| CAD avançado J | AutoCAD |

As chaves de fonte (`sw-…`, `inv-…`, `cb-…`) estão detalhadas em `docs/benchmark/fontes.md`.
O teste `crates/fenkra-app/tests/sem_marcas.rs` falha se um nome de produto aparecer fora desses arquivos.
| 2026-11-05 | Malha e cache (B219, B220, B234) | Documentação e fontes do OpenCASCADE 8.0.1 (dev.opencascade.org) | referência | LGPL-2.1 com exceção | Parâmetros do malhador e pool de fios, usados com nossas palavras |
| 2026-11-05 | Exportação STL e STEP (B84) | Fontes do OpenCASCADE 8.0.1 (StlAPI_Writer, RWStl, OSD_OpenFile, BRepBuilderAPI_Copy) | referência | LGPL-2.1 com exceção | Como cada gravador abre o caminho e o modo binário, usados com nossas palavras |
