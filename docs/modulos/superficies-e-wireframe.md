# Superfícies e wireframe

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/superficies-e-wireframe.md](../benchmark/superficies-e-wireframe.md).

## ✅ Pronto

- **Pontos de referência** (Ponto) — PointCoords, PointOnCurve, PointOnSurface, PointBetween, PointCenter, PointTangent… `DR-49`
- **Linhas de referência** (Linha) — LinePoints, LineDir, LineAngle, LineNormalSurface, LineBisector, LinePlanes, LineAxis.
- **Projeção** (Projeção) — CurveProject, CurveProjectSurf, PointProject.
- **Interseção** (Interseção) — CurveIntersect.
- **Curvas de conexão e canto** (Conectar, canto) — Connect, Corner; teste conector_g3_e_canto_entre_linha_e_spline.
- **Spline 3D, hélice, espiral** (Spline, hélice, espiral) — Spline, SplineControl, Helix, Spiral.
- **Superfície varrida** (Varredura de superfície) — SurfSweep, SurfSweepX (avançada).
- **Concordância (blend)** (Blend) — SurfBlend.
- **Offset de superfície** (Offset) — SurfOffset; teste superficies_varredura_avancada_offset_filete…
- **Dividir e aparar** (Dividir, aparar) — SurfSplit, SurfTrim.
- **Estender / extrapolar** (Extrapolar) — SurfExtrapolate.
- **Fechar superfície / costurar ao corpo** (Fechar superfície, costurar) — CloseSurface, SewSurface.
- **Reparo (healing)** (Reparar) — SurfHeal.

## 🟡 Parcial

- **Planos de referência** (Plano) — Muitos modos; plano de face continua vivo quando a face some (DR-21); plano na curva sem modo por ponto projetado (DR-04). `DR-21`
- **Extrair / extrair múltiplo** (Extrair) — SurfExtract/SurfMulti; a propagação fica só na interface (extract.rs) e não é refeita no recompute; falta no menu de outros comandos (DR-13, DR-14). `DR-14`
- **Superfície extrudada e de revolução** (Extrusão / revolução de superfície) — SurfExtrude só com comprimentos: falta "até elemento" (DR-24). `DR-24`
- **Superfície por seções (loft)** (Loft de superfície) — SurfLoft com uma guia. `BM-SUP-01`
- **Preenchimento (fill)** (Preencher / Preencher fronteiras) — SurfFill e BoundaryFill não aceitam linhas de sketch nem seleção em cadeia (DR-13). `DR-13`
- **Juntar (join/knit/stitch)** (Juntar) — CurveJoin e SurfJoin existem; não juntou 4 linhas de sketch (DR-13). `DR-13`
- **Superfície regrada** (Conectar superfícies) — SurfConnect cobre parte. `BM-SUP-02`
- **Filete de superfície** (Filete de superfície) — SurfFillet, SurfEdgeFillet; renderização quebra em filetes de superfície (DR-10). `DR-10`
- **Análises (zebra, curvatura, desvio)** (Zebra, curvatura) — Zebra existe; desvio e checagem de continuidade parciais. `BM-SUP-03`
- **Elementos de cada comando** (Painel com abas) — Painéis com abas e ícones (alpha.25); ordem e "até elemento" ainda não uniformes (DR-25..29). `DR-25`
