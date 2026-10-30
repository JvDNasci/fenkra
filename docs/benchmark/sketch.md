# Benchmark — Sketch

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#sketch).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Sketch num plano/face → perfil desenhado com restrições automáticas → restrições pela caixa de diálogo (lista as possíveis para a seleção) → ferramentas de relimitação (Trim, Quick Trim, Break, Close, Complement) → Sketch Analysis antes de sair. O sketch é visto como sistema de equações e o objetivo é ficar totalmente restrito.
- **SOLIDWORKS:** Sketch → entidades com relações automáticas → Smart Dimension deduz o tipo de cota → Power Trim (arrastar cruzando) para limpar → cores indicam sub/totalmente/sobre-definido → Convert/Offset trazem geometria do modelo.
- **Inventor:** Sketch com restrições automáticas por inferência → Dimension → Trim/Extend/Split alternáveis com Shift ou menu → Project Geometry / Project Cut Edges trazem o 3D → contador de graus de liberdade.

## O que adotar no Fenkra

- Relimitação completa do CATIA (Quick Trim, Break, Close, Complement) com o arrasto do Power Trim do SW.
- Caixa de restrições possíveis para a seleção (CATIA) junto com a cota inteligente (SW/Inventor).
- Análise de sketch antes de sair: perfis abertos, sobrepostos e isolados.

## O que evitar

- Projeções sem associatividade (o Inventor não deixa aparar o projetado sem quebrar o vínculo).
- Perfis automáticos que criam restrições demais (alerta do contexto do sketcher).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| SKT-01 | Linha e perfil encadeado | ✅ | Tool::Line; restrições automáticas ao desenhar. |  |
| SKT-02 | Retângulo e variantes | 🟡 | Tool::Rect, RectCenter; sem 3 pontos (inclinado) nem paralelogramo. | BM-SKT-01 |
| SKT-03 | Círculo e arco | ✅ | Tool::Circle, Circle3, Arc3, ArcCenter. |  |
| SKT-04 | Spline | 🟡 | Tool::Spline com alças e inflexões (alpha.25). Não acha tangência com outros elementos do sketch (DR-02); sem modo por polígono de controle no sketch. | DR-02 |
| SKT-05 | Elipse e cônicas | ❌ | O sketch só tem Line/Circle/Arc/Point/Spline. Cônica existe no wireframe 3D (RefKind::Conic). | BM-SKT-02 |
| SKT-06 | Polígono e rasgo | 🟡 | Tool::Polygon e Slot (reto). Sem rasgo em arco no sketch (existe SmArcSlot só na chapa). | BM-SKT-03 |
| SKT-07 | Restrições geométricas | ✅ | Kind: Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal, Tangent, Concentric, Midpoint, PointOnCurve, Fixed, Symmetric… |  |
| SKT-08 | Cotas (restrições dimensionais) | ✅ | Tool::Dimension (+H, V, ângulo, raio, diâmetro); expressões com parâmetros e vírgula decimal. |  |
| SKT-09 | Solver e estado do sketch | ✅ | Teste sub_restrito_e_conflito; arrastar_ponto_preso_mantem_restricoes. | BM-SKT-04 |
| SKT-10 | Aparar (trim) | 🟡 | Tool::Trim só em linhas (trim_line). | DR-50 |
| SKT-11 | Aparar rápido / borracha | ❌ | Não existe para arcos/círculos/splines. | DR-50 |
| SKT-12 | Quebrar (break/split) | ❌ |  | DR-51 |
| SKT-13 | Estender / aparar-estender | 🟡 | Tool::Extend (extend_line) só para linhas; sem modo combinado. | DR-51 |
| SKT-14 | Fechar arco e complemento | ❌ |  | DR-52 |
| SKT-15 | Offset de curvas | ✅ | Tool::Offset. |  |
| SKT-16 | Espelhar e simetria | ✅ | Tool::Mirror. |  |
| SKT-17 | Mover, copiar, girar, escalar | 🟡 | Tool::Move e Copy; sem girar/escalar/esticar dedicados. | BM-SKT-05 |
| SKT-18 | Filete e chanfro de sketch | ✅ | Tool::Fillet, Chamfer. |  |
| SKT-19 | Padrão no sketch | ❌ |  | BM-SKT-06 |
| SKT-20 | Projetar / converter arestas 3D | ✅ | Tool::Convert; teste sketch_com_geometria_externa_associativa. |  |
| SKT-21 | Interseção com o 3D | 🟡 | Tool::Intersect; só 1 ponto por curva (crossing()); falha com perfis de outro plano (DR-06). | DR-06 |
| SKT-22 | Seleção por janela e cruzada | 🟡 | Existe no 3D (box_mode) mas não dentro do sketch (DR-05). | DR-05 |
| SKT-23 | Geometria externa no sketch | 🟡 | Converter cria vínculo; falta a opção de política por sketch (DR-18). | DR-18 |
| SKT-24 | Texto no sketch | ❌ |  | BM-SKT-07 |
| SKT-25 | Sketch 3D | 🟡 | Sem sketch 3D com restrições; há polilinha/spline/hélice 3D no wireframe (RefKind). | BM-SKT-08 |
