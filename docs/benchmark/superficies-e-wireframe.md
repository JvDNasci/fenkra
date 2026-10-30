# Benchmark — Superfícies e wireframe

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#superfícies-e-wireframe).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Workbench dedicada (GSD): wireframe (pontos, linhas, planos, projeções, interseções), superfícies (extrude, revolve, sweep, fill, multi-sections, blend), operações (join, split, trim, extract, healing, extrapolate) e análises. Todo comando segue suporte → modo → parâmetros → avançado, com propagação e tolerâncias explícitas.
- **SOLIDWORKS:** Ferramentas de superfície no mesmo ambiente das peças: Boundary/Lofted/Filled/Knit/Trim/Extend/Ruled/Offset/Thicken; sólidos e superfícies se misturam livremente.
- **Inventor:** Patch, Stitch, Ruled, Trim, Extend e Sculpt; Boundary Patch aceita sketch fechado ou bordas. Stitch com tolerância cria sólido quando fecha.

## O que adotar no Fenkra

- Propagação gravada (ponto/tangência/curvatura) e Extract usável dentro de qualquer comando (CATIA).
- Fill/Boundary Patch aceitando sketch, curvas e arestas misturados (SW/Inventor).
- "Até elemento" em toda extensão de superfície.

## O que evitar

- Comandos que só aceitam faces e ignoram linhas de sketch (falha de Fill/Join no review).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| SUP-01 | Pontos de referência | ✅ | PointCoords, PointOnCurve, PointOnSurface, PointBetween, PointCenter, PointTangent… | DR-49 |
| SUP-02 | Linhas de referência | ✅ | LinePoints, LineDir, LineAngle, LineNormalSurface, LineBisector, LinePlanes, LineAxis. |  |
| SUP-03 | Planos de referência | 🟡 | Muitos modos; plano de face continua vivo quando a face some (DR-21); plano na curva sem modo por ponto projetado (DR-04). | DR-21 |
| SUP-04 | Projeção | ✅ | CurveProject, CurveProjectSurf, PointProject. |  |
| SUP-05 | Interseção | ✅ | CurveIntersect. |  |
| SUP-06 | Curvas de conexão e canto | ✅ | Connect, Corner; teste conector_g3_e_canto_entre_linha_e_spline. |  |
| SUP-07 | Spline 3D, hélice, espiral | ✅ | Spline, SplineControl, Helix, Spiral. |  |
| SUP-08 | Extrair / extrair múltiplo | 🟡 | SurfExtract/SurfMulti; a propagação fica só na interface (extract.rs) e não é refeita no recompute; falta no menu de outros comandos (DR-13, DR-14). | DR-14 |
| SUP-09 | Superfície extrudada e de revolução | 🟡 | SurfExtrude só com comprimentos: falta "até elemento" (DR-24). | DR-24 |
| SUP-10 | Superfície varrida | ✅ | SurfSweep, SurfSweepX (avançada). |  |
| SUP-11 | Superfície por seções (loft) | 🟡 | SurfLoft com uma guia. | BM-SUP-01 |
| SUP-12 | Preenchimento (fill) | 🟡 | SurfFill e BoundaryFill não aceitam linhas de sketch nem seleção em cadeia (DR-13). | DR-13 |
| SUP-13 | Concordância (blend) | ✅ | SurfBlend. |  |
| SUP-14 | Offset de superfície | ✅ | SurfOffset; teste superficies_varredura_avancada_offset_filete… |  |
| SUP-15 | Juntar (join/knit/stitch) | 🟡 | CurveJoin e SurfJoin existem; não juntou 4 linhas de sketch (DR-13). | DR-13 |
| SUP-16 | Dividir e aparar | ✅ | SurfSplit, SurfTrim. |  |
| SUP-17 | Estender / extrapolar | ✅ | SurfExtrapolate. |  |
| SUP-18 | Superfície regrada | 🟡 | SurfConnect cobre parte. | BM-SUP-02 |
| SUP-19 | Filete de superfície | 🟡 | SurfFillet, SurfEdgeFillet; renderização quebra em filetes de superfície (DR-10). | DR-10 |
| SUP-20 | Fechar superfície / costurar ao corpo | ✅ | CloseSurface, SewSurface. |  |
| SUP-21 | Reparo (healing) | ✅ | SurfHeal. |  |
| SUP-22 | Análises (zebra, curvatura, desvio) | 🟡 | Zebra existe; desvio e checagem de continuidade parciais. | BM-SUP-03 |
| SUP-23 | Elementos de cada comando | 🟡 | Painéis com abas e ícones (alpha.25); ordem e "até elemento" ainda não uniformes (DR-25..29). | DR-25 |
