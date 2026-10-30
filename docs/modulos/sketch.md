# Sketch

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/sketch.md](../benchmark/sketch.md).

## ✅ Pronto

- **Linha e perfil encadeado** (Linha) — Tool::Line; restrições automáticas ao desenhar.
- **Círculo e arco** (Círculo, círculo 3 pontos, arco 3 pontos, arco pelo centro) — Tool::Circle, Circle3, Arc3, ArcCenter.
- **Restrições geométricas** (Restrições) — Kind: Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal, Tangent, Concentric, Midpoint, PointOnCurve, Fixed, Symmetric…
- **Cotas (restrições dimensionais)** (Cota (smart)) — Tool::Dimension (+H, V, ângulo, raio, diâmetro); expressões com parâmetros e vírgula decimal.
- **Solver e estado do sketch** (Solver com estado) — Teste sub_restrito_e_conflito; arrastar_ponto_preso_mantem_restricoes. `BM-SKT-04`
- **Offset de curvas** (Offset) — Tool::Offset.
- **Espelhar e simetria** (Espelhar) — Tool::Mirror.
- **Filete e chanfro de sketch** (Filete, chanfro) — Tool::Fillet, Chamfer.
- **Projetar / converter arestas 3D** (Converter) — Tool::Convert; teste sketch_com_geometria_externa_associativa.

## 🟡 Parcial

- **Retângulo e variantes** (Retângulo, retângulo pelo centro) — Tool::Rect, RectCenter; sem 3 pontos (inclinado) nem paralelogramo. `BM-SKT-01`
- **Spline** (Spline (alças Bézier)) — Tool::Spline com alças e inflexões (alpha.25). Não acha tangência com outros elementos do sketch (DR-02); sem modo por polígono de controle no sketch. `DR-02`
- **Polígono e rasgo** (Polígono, rasgo) — Tool::Polygon e Slot (reto). Sem rasgo em arco no sketch (existe SmArcSlot só na chapa). `BM-SKT-03`
- **Aparar (trim)** (Aparar) — Tool::Trim só em linhas (trim_line). `DR-50`
- **Estender / aparar-estender** (Estender) — Tool::Extend (extend_line) só para linhas; sem modo combinado. `DR-51`
- **Mover, copiar, girar, escalar** (Mover, copiar) — Tool::Move e Copy; sem girar/escalar/esticar dedicados. `BM-SKT-05`
- **Interseção com o 3D** (Interseção) — Tool::Intersect; só 1 ponto por curva (crossing()); falha com perfis de outro plano (DR-06). `DR-06`
- **Seleção por janela e cruzada** (só no 3D) — Existe no 3D (box_mode) mas não dentro do sketch (DR-05). `DR-05`
- **Geometria externa no sketch** (Converter) — Converter cria vínculo; falta a opção de política por sketch (DR-18). `DR-18`
- **Sketch 3D** (Wireframe 3D) — Sem sketch 3D com restrições; há polilinha/spline/hélice 3D no wireframe (RefKind). `BM-SKT-08`

## ❌ Ainda não existe

- **Elipse e cônicas** (—) — O sketch só tem Line/Circle/Arc/Point/Spline. Cônica existe no wireframe 3D (RefKind::Conic). `BM-SKT-02`
- **Aparar rápido / borracha** (—) — Não existe para arcos/círculos/splines. `DR-50`
- **Quebrar (break/split)** (—) —  `DR-51`
- **Fechar arco e complemento** (—) —  `DR-52`
- **Padrão no sketch** (—) —  `BM-SKT-06`
- **Texto no sketch** (—) —  `BM-SKT-07`
