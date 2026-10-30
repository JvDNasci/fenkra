# Catálogo de features

158 features em 12 módulos, com o nome em cada ferramenta, como funcionam em geral e o status no Fenkra (alpha.26): ✅ 69 completas · 🟡 60 parciais · ❌ 29 inexistentes.

> Gerado por `scripts/benchmark/gerar.py` a partir de `scripts/benchmark/dados_*.py`. Não edite à mão.
> Nomes de outros produtos são marcas dos respectivos donos e aparecem só para comparação; textos próprios,
> sem trechos copiados. Fontes em [fontes.md](fontes.md).

| Módulo | ✅ | 🟡 | ❌ | Workflow |
|---|---|---|---|---|
| [Sketch](#sketch) | 9 | 10 | 6 | [Sketch](sketch.md) |
| [Sólidos](#sólidos) | 13 | 9 | 2 | [Sólidos](solidos.md) |
| [Superfícies e wireframe](#superfícies-e-wireframe) | 13 | 10 | 0 | [Superfícies e wireframe](superficies-e-wireframe.md) |
| [Chapa metálica](#chapa-metálica) | 8 | 6 | 3 | [Chapa metálica](chapa-metalica.md) |
| [Estrutural e soldas](#estrutural-e-soldas) | 2 | 5 | 2 | [Estrutural e soldas](estrutural-e-soldas.md) |
| [Montagem](#montagem) | 11 | 2 | 3 | [Montagem](montagem.md) |
| [Desenho técnico](#desenho-técnico) | 5 | 5 | 2 | [Desenho técnico](desenho-tecnico.md) |
| [PMI 3D](#pmi-3d) | 3 | 1 | 1 | [PMI 3D](pmi-3d.md) |
| [Navegação](#navegação) | 3 | 2 | 1 | [Navegação](navegacao.md) |
| [Biblioteca de normas](#biblioteca-de-normas) | 1 | 1 | 7 | [Biblioteca de normas](biblioteca-de-normas.md) |
| [PDM / PLM](#pdm--plm) | 1 | 2 | 2 | [PDM / PLM](pdm-plm.md) |
| [Geral](#geral) | 0 | 7 | 0 | [Geral](geral.md) |

## Sketch

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| SKT-01 | **Linha e perfil encadeado** | Line / Profile | Line | Line | ✅ Linha |  |
| SKT-02 | **Retângulo e variantes** | Rectangle / Centered / Oriented / Parallelogram | Corner/Center/3 Point Rectangle, Parallelogram | Two Point / Three Point / Center Rectangle | 🟡 Retângulo, retângulo pelo centro | BM-SKT-01 |
| SKT-03 | **Círculo e arco** | Circle / Three Point Circle / Arc / Three Point Arc | Circle / Perimeter Circle / Arcs | Circle / Arc (3 pontos, tangente, centro) | ✅ Círculo, círculo 3 pontos, arco 3 pontos, arco pelo centro |  |
| SKT-04 | **Spline** | Spline | Spline / Style Spline | Spline (interpolação / controle) | 🟡 Spline (alças Bézier) | DR-02 |
| SKT-05 | **Elipse e cônicas** | Ellipse / Parabola / Hyperbola / Conic | Ellipse / Partial Ellipse / Parabola / Conic | Ellipse | ❌ — | BM-SKT-02 |
| SKT-06 | **Polígono e rasgo** | Hexagon / Elongated Hole / Cylindrical Elongated Hole | Polygon / Straight & Arc Slot | Polygon / Slot (5 tipos) | 🟡 Polígono, rasgo | BM-SKT-03 |
| SKT-07 | **Restrições geométricas** | Constraints Defined in Dialog Box | Add Relations | Geometric Constraints | ✅ Restrições |  |
| SKT-08 | **Cotas (restrições dimensionais)** | Constraint | Smart Dimension | Dimension | ✅ Cota (smart) |  |
| SKT-09 | **Solver e estado do sketch** | Sketch Solving Status / Sketch Analysis | Fully/Under/Over Defined | Fully constrained / DOF | ✅ Solver com estado | BM-SKT-04 |
| SKT-10 | **Aparar (trim)** | Trim | Trim Entities (corner, inside, outside, closest) | Trim | 🟡 Aparar | DR-50 |
| SKT-11 | **Aparar rápido / borracha** | Quick Trim | Power Trim | Trim (clique/arrasto) | ❌ — | DR-50 |
| SKT-12 | **Quebrar (break/split)** | Break | Split Entities | Split | ❌ — | DR-51 |
| SKT-13 | **Estender / aparar-estender** | Trim (estende) / Relimitation | Extend Entities / Power Trim | Extend | 🟡 Estender | DR-51 |
| SKT-14 | **Fechar arco e complemento** | Close / Complement | — (via edição) | — | ❌ — | DR-52 |
| SKT-15 | **Offset de curvas** | Offset | Offset Entities | Offset | ✅ Offset |  |
| SKT-16 | **Espelhar e simetria** | Mirror / Symmetry | Mirror Entities / Dynamic Mirror | Mirror | ✅ Espelhar |  |
| SKT-17 | **Mover, copiar, girar, escalar** | Translate / Rotate / Scale | Move / Copy / Rotate / Scale / Stretch | Move / Copy / Rotate / Scale / Stretch | 🟡 Mover, copiar | BM-SKT-05 |
| SKT-18 | **Filete e chanfro de sketch** | Corner / Chamfer | Sketch Fillet / Sketch Chamfer | Fillet / Chamfer | ✅ Filete, chanfro |  |
| SKT-19 | **Padrão no sketch** | — (via Translate/Rotate com cópias) | Linear / Circular Sketch Pattern | Rectangular / Circular Pattern | ❌ — | BM-SKT-06 |
| SKT-20 | **Projetar / converter arestas 3D** | Project 3D Elements | Convert Entities | Project Geometry | ✅ Converter |  |
| SKT-21 | **Interseção com o 3D** | Intersect 3D Elements | Intersection Curve | Project Cut Edges / Intersection Curve | 🟡 Interseção | DR-06 |
| SKT-22 | **Seleção por janela e cruzada** | Selection trap (dentro/intersecta) | Box / Cross Select | Window / Crossing | 🟡 só no 3D | DR-05 |
| SKT-23 | **Geometria externa no sketch** | Project / Use Edge com associatividade | Convert Entities / relações externas | Project Geometry | 🟡 Converter | DR-18 |
| SKT-24 | **Texto no sketch** | — (Text via Drafting) | Sketch Text | Text / Geometry Text | ❌ — | BM-SKT-07 |
| SKT-25 | **Sketch 3D** | — (wireframe do GSD) | 3D Sketch | 3D Sketch | 🟡 Wireframe 3D | BM-SKT-08 |

### SKT-01 — Linha e perfil encadeado

- **O que faz:** Desenha segmentos retos, isolados ou em sequência, com arcos tangentes no meio.
- **Como funciona:** Suporte: plano do sketch → modo: dois pontos, encadeado (profile), horizontal/vertical → parâmetros: comprimento, ângulo → avançado: restrições automáticas ao desenhar.
- **Diferenças:** CATIA separa Line de Profile (polilinha com arcos tangentes). SW e Inventor fazem a transição para arco tangente arrastando de volta no último ponto.
- **Fenkra:** ✅ completo. Tool::Line; restrições automáticas ao desenhar.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Trimming with Power Trim](https://help.solidworks.com/2017/english/solidworks/sldworks/t_Trimming_with_Power_Trim.htm) · [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm)

### SKT-02 — Retângulo e variantes

- **O que faz:** Cria retângulos por canto, centro, 3 pontos (inclinado) e paralelogramo.
- **Como funciona:** Suporte: plano → modo: 2 cantos, centro, 3 pontos, paralelogramo → parâmetros: largura, altura, ângulo → avançado: linhas de construção diagonais.
- **Diferenças:** Os três oferecem a versão inclinada por 3 pontos.
- **Fenkra:** 🟡 parcial. Tool::Rect, RectCenter; sem 3 pontos (inclinado) nem paralelogramo.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Trimming with Power Trim](https://help.solidworks.com/2017/english/solidworks/sldworks/t_Trimming_with_Power_Trim.htm)

### SKT-03 — Círculo e arco

- **O que faz:** Círculos e arcos por centro+raio, 3 pontos, tangentes.
- **Como funciona:** Suporte: plano → modo: centro, 3 pontos, tangente a 3 elementos → parâmetros: raio/diâmetro, ângulos → avançado: arco tangente ao fim de linha.
- **Diferenças:** Círculo tangente a 3 elementos existe no CATIA e no Inventor.
- **Fenkra:** ✅ completo. Tool::Circle, Circle3, Arc3, ArcCenter.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm)

### SKT-04 — Spline

- **O que faz:** Curva livre por pontos de passagem ou polígono de controle, com tangências.
- **Como funciona:** Suporte: plano → modo: interpolada (passa pelos pontos) ou por controle → parâmetros: tangente/curvatura em pontos → avançado: tensão, grau, análise de curvatura (pente).
- **Diferenças:** SW tem Style Spline (Bézier de grau definido); Inventor tem os dois modos; CATIA tem tensão por ponto.
- **Fenkra:** 🟡 parcial. Tool::Spline com alças e inflexões (alpha.25). Não acha tangência com outros elementos do sketch (DR-02); sem modo por polígono de controle no sketch.
- **Fontes:** [Contexto: sketcher](../contextos/sketcher.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-05 — Elipse e cônicas

- **O que faz:** Elipse, parábola, hipérbole e cônica geral.
- **Como funciona:** Suporte: plano → modo: centro+eixos, por pontos → parâmetros: semieixos, rho (cônica).
- **Diferenças:** CATIA é o mais completo; Inventor só elipse.
- **Fenkra:** ❌ não existe. O sketch só tem Line/Circle/Arc/Point/Spline. Cônica existe no wireframe 3D (RefKind::Conic).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SKT-06 — Polígono e rasgo

- **O que faz:** Polígono regular e rasgo (oblongo) reto ou em arco.
- **Como funciona:** Suporte: plano → modo: inscrito/circunscrito; rasgo reto, pelo centro, em arco → parâmetros: lados, largura.
- **Fenkra:** 🟡 parcial. Tool::Polygon e Slot (reto). Sem rasgo em arco no sketch (existe SmArcSlot só na chapa).
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SKT-07 — Restrições geométricas

- **O que faz:** Relações entre elementos: coincidente, paralelo, tangente, simétrico etc.
- **Como funciona:** Seleção de 1–3 elementos → o sistema propõe as restrições possíveis → aplicar; ícones na tela; ocultar/mostrar.
- **Diferenças:** CATIA lista as restrições possíveis numa caixa para a seleção; SW propõe no PropertyManager; Inventor por ícones.
- **Fenkra:** ✅ completo. Kind: Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal, Tangent, Concentric, Midpoint, PointOnCurve, Fixed, Symmetric…
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Trimming with Power Trim](https://help.solidworks.com/2017/english/solidworks/sldworks/t_Trimming_with_Power_Trim.htm)

### SKT-08 — Cotas (restrições dimensionais)

- **O que faz:** Cotas que dirigem a geometria: distância, ângulo, raio, diâmetro.
- **Como funciona:** Seleção → tipo deduzido pela seleção (cota inteligente) → valor ou expressão → avançado: cota de referência (só leitura).
- **Diferenças:** Todos têm cota dirigida e de referência.
- **Fenkra:** ✅ completo. Tool::Dimension (+H, V, ângulo, raio, diâmetro); expressões com parâmetros e vírgula decimal.
- **Fontes:** [SOLIDWORKS Help — Trimming with Power Trim](https://help.solidworks.com/2017/english/solidworks/sldworks/t_Trimming_with_Power_Trim.htm)

### SKT-09 — Solver e estado do sketch

- **O que faz:** Resolve as restrições e mostra sub/total/sobre-restrito.
- **Como funciona:** Cores por estado; diagnóstico de conflitos; arrastar pontos livres respeitando restrições.
- **Diferenças:** CATIA tem Sketch Analysis (perfis abertos, sobrepostos, isolados).
- **Fenkra:** ✅ completo. Teste sub_restrito_e_conflito; arrastar_ponto_preso_mantem_restricoes.
- **Fontes:** [Contexto: sketcher](../contextos/sketcher.md)

### SKT-10 — Aparar (trim)

- **O que faz:** Corta elementos até interseções.
- **Como funciona:** Seleção do trecho → corta até a interseção mais próxima; modos: dois elementos, canto, dentro/fora.
- **Diferenças:** SW tem 5 modos; Inventor alterna Trim/Extend com Shift.
- **Fenkra:** 🟡 parcial. Tool::Trim só em linhas (trim_line).
- **Fontes:** [SOLIDWORKS Help — Trimming with Power Trim](https://help.solidworks.com/2017/english/solidworks/sldworks/t_Trimming_with_Power_Trim.htm) · [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm)

### SKT-11 — Aparar rápido / borracha

- **O que faz:** Apaga só o trecho clicado entre interseções; arrastar cruzando apaga vários.
- **Como funciona:** Detecta interseções → segmentos virtuais → apaga o clicado → mantém restrições possíveis.
- **Diferenças:** Power Trim do SW também estende ao arrastar.
- **Fenkra:** ❌ não existe. Não existe para arcos/círculos/splines.
- **Fontes:** [SOLIDWORKS Help — Trimming with Power Trim](https://help.solidworks.com/2017/english/solidworks/sldworks/t_Trimming_with_Power_Trim.htm) · [GrabCAD — CATIA V5 tutorial: Trim, Break, Mirror…](https://grabcad.com/tutorials/7-catia-v5-complete-tutorial-for-beginners-trim-break-mirror-symmetry-translate-rotate-scale) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-12 — Quebrar (break/split)

- **O que faz:** Divide um elemento num ponto, criando dois independentes.
- **Como funciona:** Seleção do elemento → ponto de quebra (ou elemento cortante) → dois segmentos com coincidência.
- **Fenkra:** ❌ não existe.
- **Fontes:** [GrabCAD — CATIA V5 tutorial: Trim, Break, Mirror…](https://grabcad.com/tutorials/7-catia-v5-complete-tutorial-for-beginners-trim-break-mirror-symmetry-translate-rotate-scale) · [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm)

### SKT-13 — Estender / aparar-estender

- **O que faz:** Leva o elemento até o limite mais próximo, estendendo ou cortando.
- **Como funciona:** Seleção do lado → projeção da direção → interseção com o limite → ajusta o comprimento.
- **Fenkra:** 🟡 parcial. Tool::Extend (extend_line) só para linhas; sem modo combinado.
- **Fontes:** [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-14 — Fechar arco e complemento

- **O que faz:** Transforma arco em círculo ou cria o arco complementar.
- **Como funciona:** Seleção do arco → mesmo centro e raio → círculo completo / arco oposto.
- **Diferenças:** Exclusivo do CATIA.
- **Fenkra:** ❌ não existe.
- **Fontes:** [GrabCAD — CATIA V5 tutorial: Trim, Break, Mirror…](https://grabcad.com/tutorials/7-catia-v5-complete-tutorial-for-beginners-trim-break-mirror-symmetry-translate-rotate-scale) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-15 — Offset de curvas

- **O que faz:** Cópia paralela de curvas a uma distância.
- **Como funciona:** Seleção (com propagação por cadeia) → distância → lado/ambos → avançado: tampas, cópias múltiplas.
- **Diferenças:** CATIA/SW têm propagação por tangência e várias cópias.
- **Fenkra:** ✅ completo. Tool::Offset.
- **Fontes:** [Contexto: sketcher](../contextos/sketcher.md)

### SKT-16 — Espelhar e simetria

- **O que faz:** Espelha geometria por uma linha, com restrição de simetria.
- **Como funciona:** Seleção → eixo → cópia com restrição; avançado: copiar ou não as restrições.
- **Fenkra:** ✅ completo. Tool::Mirror.
- **Fontes:** [GrabCAD — CATIA V5 tutorial: Trim, Break, Mirror…](https://grabcad.com/tutorials/7-catia-v5-complete-tutorial-for-beginners-trim-break-mirror-symmetry-translate-rotate-scale)

### SKT-17 — Mover, copiar, girar, escalar

- **O que faz:** Transformações no sketch.
- **Como funciona:** Seleção → vetor/ângulo/fator → cópias e restrições.
- **Fenkra:** 🟡 parcial. Tool::Move e Copy; sem girar/escalar/esticar dedicados.
- **Fontes:** [GrabCAD — CATIA V5 tutorial: Trim, Break, Mirror…](https://grabcad.com/tutorials/7-catia-v5-complete-tutorial-for-beginners-trim-break-mirror-symmetry-translate-rotate-scale)

### SKT-18 — Filete e chanfro de sketch

- **O que faz:** Arredonda ou chanfra o canto entre dois elementos.
- **Como funciona:** Seleção de 2 elementos → raio ou distâncias → modo de corte dos originais.
- **Diferenças:** CATIA tem 6 modos de corte para o Corner.
- **Fenkra:** ✅ completo. Tool::Fillet, Chamfer.
- **Fontes:** [Contexto: sketcher](../contextos/sketcher.md)

### SKT-19 — Padrão no sketch

- **O que faz:** Repete geometria em linha ou círculo dentro do sketch.
- **Como funciona:** Seleção → direção/eixo → número e espaçamento.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SKT-20 — Projetar / converter arestas 3D

- **O que faz:** Traz arestas e contornos do modelo para o sketch, associativos.
- **Como funciona:** Seleção de arestas/faces → projeção no plano → vínculo mantido.
- **Fenkra:** ✅ completo. Tool::Convert; teste sketch_com_geometria_externa_associativa.
- **Fontes:** [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm)

### SKT-21 — Interseção com o 3D

- **O que faz:** Cria no sketch as curvas onde o plano corta faces, sólidos ou curvas de outro plano.
- **Como funciona:** Seleção do elemento 3D → interseção com o plano do sketch → curvas associativas.
- **Fenkra:** 🟡 parcial. Tool::Intersect; só 1 ponto por curva (crossing()); falha com perfis de outro plano (DR-06).
- **Fontes:** [Inventor Help — Split, Trim or Extend Curves](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DD00037F-6BBA-4B95-AA90-4F99DCEB667D.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-22 — Seleção por janela e cruzada

- **O que faz:** Seleciona arrastando: da esquerda (dentro) ou da direita (cruzando).
- **Como funciona:** Arrasto → sentido define janela/cruzada → filtro de tipo.
- **Fenkra:** 🟡 parcial. Existe no 3D (box_mode) mas não dentro do sketch (DR-05).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-23 — Geometria externa no sketch

- **O que faz:** Usar arestas/pontos de fora como referência, com política: só publicados ou livre.
- **Como funciona:** Ao selecionar fora do plano → cria vínculo (ou publicação automática) ou bloqueia.
- **Fenkra:** 🟡 parcial. Converter cria vínculo; falta a opção de política por sketch (DR-18).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SKT-24 — Texto no sketch

- **O que faz:** Contornos de texto para gravação/relevo.
- **Como funciona:** Texto, fonte, altura → curvas fechadas.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SKT-25 — Sketch 3D

- **O que faz:** Linhas e splines livres no espaço, com restrições 3D.
- **Como funciona:** Pontos 3D → planos de trabalho dinâmicos → restrições.
- **Fenkra:** 🟡 parcial. Sem sketch 3D com restrições; há polilinha/spline/hélice 3D no wireframe (RefKind).
- **Fontes:** [SOLIDWORKS Help — Features Overview](https://help.solidworks.com/2012/English/SolidWorks/sldworks/Features_Overview.htm)


## Sólidos

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| SOL-01 | **Extrusão (adicionar)** | Pad | Extruded Boss/Base | Extrude | ✅ Extrusão |  |
| SOL-02 | **Recorte por extrusão** | Pocket | Extruded Cut | Extrude (Cut) | ✅ Recorte |  |
| SOL-03 | **Revolução** | Shaft / Groove | Revolved Boss / Cut | Revolve | ✅ Revolução / Ranhura |  |
| SOL-04 | **Varredura** | Rib / Slot | Swept Boss / Cut | Sweep | ✅ Varredura |  |
| SOL-05 | **Loft (multi-seções)** | Multi-Sections Solid | Lofted Boss/Base | Loft | 🟡 Loft | DR-07 |
| SOL-06 | **Furo** | Hole | Hole Wizard / Advanced Hole | Hole | 🟡 Furo | DR-02 |
| SOL-07 | **Rosca** | Thread/Tap | Cosmetic Thread / Thread | Thread | 🟡 Rosca | DR-03 |
| SOL-08 | **Filete de aresta** | Edge Fillet | Fillet (constant) | Fillet (edge) | ✅ Filete |  |
| SOL-09 | **Filete variável** | Variable Radius Fillet | Variable Size Fillet | Fillet (variable) | 🟡 Filete variável |  |
| SOL-10 | **Chanfro** | Chamfer | Chamfer | Chamfer | ✅ Chanfro |  |
| SOL-11 | **Casca** | Shell | Shell | Shell | 🟡 Casca | DR-07 |
| SOL-12 | **Ângulo de saída (draft)** | Draft Angle / Variable Draft | Draft | Face Draft | ✅ Draft |  |
| SOL-13 | **Nervura de reforço** | Stiffener | Rib | Rib / Web | 🟡 Nervura | DR-05 |
| SOL-14 | **Engrossar / espessura** | Thickness / Thick Surface | Thicken | Thicken/Offset | 🟡 Espessura / Engrossar superfície | DR-12 |
| SOL-15 | **Padrões e espelhar** | Rectangular / Circular / User Pattern / Mirror | Linear / Circular / Curve / Sketch-driven Pattern, Mirror | Rectangular / Circular / Sketch Pattern, Mirror | ✅ Padrão linear/circular/por pontos, espelhar |  |
| SOL-16 | **Booleanas entre corpos** | Add / Remove / Intersect / Assemble | Combine | Combine | ✅ Combinar |  |
| SOL-17 | **Dividir corpo** | Split | Split / Split Body | Split | ✅ Dividir corpo |  |
| SOL-18 | **Primitivas** | — (via sketch) | — (via sketch) | Primitives (Box, Cylinder…) | ✅ Primitivas |  |
| SOL-19 | **Mover / copiar corpo** | Translation / Rotation / Symmetry / Affinity | Move/Copy Body | Move Bodies | ✅ Mover corpo |  |
| SOL-20 | **Escala** | Scaling / Affinity | Scale | Scale | ✅ Escala |  |
| SOL-21 | **Relevo e gravação** | — (Pad/Pocket de texto) | Wrap | Emboss | ❌ — | BM-SOL-01 |
| SOL-22 | **Hélice / mola (coil)** | Helix + Rib | Helix + Sweep | Coil | 🟡 Hélice (wireframe) + varredura | BM-SOL-02 |
| SOL-23 | **Modelagem direta** | — (Functional / Part Design limitado) | Move Face / Delete Face | Direct Edit | ❌ — | BM-SOL-03 |
| SOL-24 | **Referências persistentes** | Naming por BRep (com reconexão) | Rebuild com referências perdidas | Reattach / Repair | 🟡 Ponto clicado + face_owner | DR-11 |

### SOL-01 — Extrusão (adicionar)

- **O que faz:** Dá espessura a um perfil ao longo de uma direção.
- **Como funciona:** Suporte: perfil fechado (sketch, faces) → modo: cego, simétrico, dois lados, até o próximo/último/plano/superfície → parâmetros: comprimento(s), saída (draft) → avançado: parede fina, direção não normal, filetes no topo.
- **Diferenças:** CATIA separa Pad/Pocket e tem Drafted Filleted Pad; SW/Inventor juntam adicionar/remover num comando.
- **Fenkra:** ✅ completo. FeatureKind::Extrude com Extent (incl. UpToSurface), taper, thin, dir; ExtrudeDraftFillet.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Features Overview](https://help.solidworks.com/2012/English/SolidWorks/sldworks/Features_Overview.htm) · [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-02 — Recorte por extrusão

- **O que faz:** Remove material extrudando um perfil.
- **Como funciona:** Igual à extrusão, com operação remover; até passante.
- **Fenkra:** ✅ completo. Cut, CutBlind, CutUpTo.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Features Overview](https://help.solidworks.com/2012/English/SolidWorks/sldworks/Features_Overview.htm)

### SOL-03 — Revolução

- **O que faz:** Gira um perfil em torno de um eixo.
- **Como funciona:** Suporte: perfil + eixo (linha de sketch, aresta, eixo) → modo: ângulo, simétrico, até elemento → parâmetros: ângulos → avançado: parede fina.
- **Fenkra:** ✅ completo. Revolve, Groove; teste revolucao_com_comando_de_angulo.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-04 — Varredura

- **O que faz:** Move um perfil ao longo de um caminho.
- **Como funciona:** Suporte: perfil + caminho (+ guias) → modo: orientação seguindo o caminho, fixa, por guia/spine → parâmetros: torção, escala → avançado: alinhamento, faces tangentes.
- **Diferenças:** CATIA chama de Rib/Slot (não confundir com nervura); SW/Inventor têm torção e guias.
- **Fenkra:** ✅ completo. FeatureKind::Sweep.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-05 — Loft (multi-seções)

- **O que faz:** Sólido de transição entre várias seções.
- **Como funciona:** Suporte: seções ordenadas (+ guias, spine) → modo: acoplamento por pontos/vértices → parâmetros: tangência/curvatura inicial e final, tensão → avançado: fechado, regrado.
- **Diferenças:** Inventor e SW têm condições de início/fim por seção; CATIA tem couplings.
- **Fenkra:** 🟡 parcial. Loft tem sections, ruled, closed, op; faltam restrição inicial/final, tensão, guias e coletor interativo (DR-07).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SOL-06 — Furo

- **O que faz:** Furo paramétrico simples, rebaixado, escareado, roscado.
- **Como funciona:** Suporte: face + ponto(s) (ou sketch) → modo: tipo de furo, término (cego, passante, até) → parâmetros: diâmetro, profundidade, ângulo da ponta, rosca → avançado: norma/tamanho, folga.
- **Diferenças:** SW Advanced Hole empilha elementos; Hole Wizard ligado à Toolbox.
- **Fenkra:** 🟡 parcial. HoleKind: Simples, Rebaixado, Escareado, Contrafurado; escareado nasce com 90° fixo; casca falha com furo escareado (DR-02).
- **Fontes:** [SOLIDWORKS Help — Hole Wizard (Toolbox)](https://help.solidworks.com/2022/English/SolidWorks/toolbox/HIDD_TOOLBOX_SELECT_HARDWARE.htm) · [Inventor Help — To Create Threads](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-C0848D58-12CB-4605-8165-0836232927F0.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SOL-07 — Rosca

- **O que faz:** Rosca cosmética ou modelada em furo/eixo.
- **Como funciona:** Suporte: face cilíndrica → modo: cosmética/modelada, comprimento total ou parcial → parâmetros: norma, tamanho, passo, classe, deslocamento inicial → avançado: sentido, chanfro de entrada.
- **Diferenças:** Inventor/SW: comprimento parcial com offset; SW Thread modela com perfil próprio.
- **Fenkra:** 🟡 parcial. Thread com modeled e end; sem passo/perfil personalizados, deslocamento inicial nem chanfro; tap com bugs; sem preview (DR-02, DR-03).
- **Fontes:** [Inventor Help — To Create Threads](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-C0848D58-12CB-4605-8165-0836232927F0.htm) · [SOLIDWORKS Help — Cosmetic Threads](https://help.solidworks.com/2024/English/SolidWorks/Sldworks/c_cosmetic_threads.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SOL-08 — Filete de aresta

- **O que faz:** Arredonda arestas com raio constante.
- **Como funciona:** Suporte: arestas (ou faces/feature) → modo: propagação por tangência → parâmetros: raio → avançado: setback, conservar arestas, overflow.
- **Fenkra:** ✅ completo. Fillet, FilletAll, FilletFull, FilletFaceFace.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Features Overview](https://help.solidworks.com/2012/English/SolidWorks/sldworks/Features_Overview.htm)

### SOL-09 — Filete variável

- **O que faz:** Raio que varia ao longo da aresta.
- **Como funciona:** Arestas → pontos de controle com raios → transição linear/suave.
- **Fenkra:** 🟡 parcial. FilletVar existe; sem pontos de controle intermediários editáveis.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SOL-10 — Chanfro

- **O que faz:** Corta a aresta com distância×ângulo ou duas distâncias.
- **Como funciona:** Arestas → modo (d×ang, d1×d2, igual) → valores.
- **Fenkra:** ✅ completo. Chamfer, ChamferAll, ChamferDistAngle, ChamferTwoDist.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SOL-11 — Casca

- **O que faz:** Esvazia o sólido com espessura, removendo faces.
- **Como funciona:** Suporte: faces a remover → parâmetros: espessura interna/externa → avançado: espessura por face.
- **Diferenças:** Todos têm espessura diferente por face.
- **Fenkra:** 🟡 parcial. Shell só com espessura única; falha com spline e furo escareado; para fora gera raio sem opção reta (DR-02, DR-07).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SOL-12 — Ângulo de saída (draft)

- **O que faz:** Inclina faces em relação a uma direção de desmoldagem.
- **Como funciona:** Faces + elemento neutro + direção → ângulo → avançado: linha de partição, propagação.
- **Fenkra:** ✅ completo. FeatureKind::Draft.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SOL-13 — Nervura de reforço

- **O que faz:** Parede fina a partir de um perfil aberto até o corpo.
- **Como funciona:** Perfil aberto → direção (no plano/normal) → espessura (um lado, dois lados, simétrica) → avançado: até o próximo, saída.
- **Diferenças:** CATIA tem modo From Side / From Top.
- **Fenkra:** 🟡 parcial. Stiffener: thickness, depth, reverse, to_body; sem lado/ambos os lados (DR-05).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SOL-14 — Engrossar / espessura

- **O que faz:** Adiciona espessura a faces de sólido ou a superfícies.
- **Como funciona:** Faces/superfície → espessura e lado(s) → novo corpo ou junta.
- **Fenkra:** 🟡 parcial. Thickness e ThickSurface; falhou em face de sólido (DR-12).
- **Fontes:** [SOLIDWORKS Help — Filled Surface](https://help.solidworks.com/2024/english/SolidWorks/sldworks/c_Filled_Surface.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SOL-15 — Padrões e espelhar

- **O que faz:** Repete features ou corpos em linha, círculo, curva ou pontos; espelha.
- **Como funciona:** Seleção → direção(ões)/eixo/curva/pontos → quantidade e espaçamento → avançado: pular instâncias, variar.
- **Diferenças:** SW tem padrão por tabela e por curva.
- **Fenkra:** ✅ completo. Pattern, Mirror; PatternPoints.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf) · [SOLIDWORKS Help — Features Overview](https://help.solidworks.com/2012/English/SolidWorks/sldworks/Features_Overview.htm)

### SOL-16 — Booleanas entre corpos

- **O que faz:** União, subtração e interseção de corpos.
- **Como funciona:** Corpos alvo e ferramenta → operação → manter ferramenta.
- **Diferenças:** CATIA usa Bodies inseridos com operação (Assemble).
- **Fenkra:** ✅ completo. Combine(Add/Cut/Intersect).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SOL-17 — Dividir corpo

- **O que faz:** Corta um corpo por plano ou superfície.
- **Como funciona:** Corpo + elemento de corte → lado(s) a manter.
- **Fenkra:** ✅ completo. SplitBody.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SOL-18 — Primitivas

- **O que faz:** Caixa, cilindro, esfera, cone, toro, cunha.
- **Como funciona:** Ponto/plano de origem → dimensões.
- **Fenkra:** ✅ completo. Prim(Caixa, Cilindro, Cone, Cunha, Esfera, Prisma, Toro).
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-19 — Mover / copiar corpo

- **O que faz:** Translada, gira, espelha ou aplica afinidade a um corpo.
- **Como funciona:** Corpo → transformação → copiar ou mover.
- **Fenkra:** ✅ completo. MoveBody (Translate, Rotate, Symmetry, Affinity).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Part Design & Sketcher (apostila)](https://www.wichita.edu/industry_and_defense/NIAR/Documents/Part-Design-R30.pdf)

### SOL-20 — Escala

- **O que faz:** Escala um corpo uniforme ou por eixo.
- **Como funciona:** Corpo → ponto de referência → fator(es).
- **Fenkra:** ✅ completo. FeatureKind::Scale.
- **Fontes:** [SOLIDWORKS Help — Features Overview](https://help.solidworks.com/2012/English/SolidWorks/sldworks/Features_Overview.htm)

### SOL-21 — Relevo e gravação

- **O que faz:** Texto ou perfil em relevo/rebaixo sobre a face.
- **Como funciona:** Perfil/texto → face → profundidade, envolver ou projetar.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-22 — Hélice / mola (coil)

- **O que faz:** Sólido helicoidal (mola, rosca modelada).
- **Como funciona:** Perfil + eixo → passo, voltas, altura → extremidades.
- **Diferenças:** Inventor tem Coil com extremidades planas.
- **Fenkra:** 🟡 parcial. Hélice no wireframe (RefKind::Helix) + Sweep; sem comando direto de mola.
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-23 — Modelagem direta

- **O que faz:** Mover/deslocar faces sem histórico.
- **Como funciona:** Faces → mover, deslocar, apagar, substituir.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Sketched features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-236B12FC-DC32-4BCA-A53D-FCB17382EEB6.htm)

### SOL-24 — Referências persistentes

- **O que faz:** Features continuam ligadas às arestas/faces mesmo após mudanças anteriores.
- **Como funciona:** Cada referência guarda origem/papel; ao mudar, re-anexa ou mostra erro com reparo.
- **Diferenças:** Os três reconstroem por histórico interno e pedem reparo quando falham.
- **Fenkra:** 🟡 parcial. Pontos casados por proximidade; filete se perde ao mudar o perfil (DR-11). Ver ADR 0010.
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)


## Superfícies e wireframe

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| SUP-01 | **Pontos de referência** | Point | Reference Point | Work Point | ✅ Ponto | DR-49 |
| SUP-02 | **Linhas de referência** | Line | Reference Axis / 3D Sketch line | Work Axis | ✅ Linha |  |
| SUP-03 | **Planos de referência** | Plane | Reference Plane | Work Plane | 🟡 Plano | DR-21 |
| SUP-04 | **Projeção** | Projection | Projected Curve | Project to Surface | ✅ Projeção |  |
| SUP-05 | **Interseção** | Intersection | Intersection Curve | Intersection Curve | ✅ Interseção |  |
| SUP-06 | **Curvas de conexão e canto** | Connect Curve / Corner | — / Fillet 3D sketch | — / Bridge Curve | ✅ Conectar, canto |  |
| SUP-07 | **Spline 3D, hélice, espiral** | Spline / Helix / Spiral | 3D Spline / Helix and Spiral | 3D Sketch / Helical Curve | ✅ Spline, hélice, espiral |  |
| SUP-08 | **Extrair / extrair múltiplo** | Extract / Multiple Extract | Offset 0 / Delete Face (indireto) | Copy Object / Extract | 🟡 Extrair | DR-14 |
| SUP-09 | **Superfície extrudada e de revolução** | Extrude / Revolve | Extruded / Revolved Surface | Extrude / Revolve (surface) | 🟡 Extrusão / revolução de superfície | DR-24 |
| SUP-10 | **Superfície varrida** | Sweep | Swept Surface | Sweep (surface) | ✅ Varredura de superfície |  |
| SUP-11 | **Superfície por seções (loft)** | Multi-Sections Surface | Lofted Surface / Boundary Surface | Loft (surface) | 🟡 Loft de superfície | BM-SUP-01 |
| SUP-12 | **Preenchimento (fill)** | Fill | Filled Surface | Boundary Patch | 🟡 Preencher / Preencher fronteiras | DR-13 |
| SUP-13 | **Concordância (blend)** | Blend | Boundary / Lofted com tangência | Loft / Boundary Patch | ✅ Blend |  |
| SUP-14 | **Offset de superfície** | Offset / Variable Offset | Offset Surface | Thicken/Offset | ✅ Offset |  |
| SUP-15 | **Juntar (join/knit/stitch)** | Join | Knit Surface / Composite Curve | Stitch | 🟡 Juntar | DR-13 |
| SUP-16 | **Dividir e aparar** | Split / Trim | Trim Surface | Trim / Split | ✅ Dividir, aparar |  |
| SUP-17 | **Estender / extrapolar** | Extrapolate | Extend Surface | Extend | ✅ Extrapolar |  |
| SUP-18 | **Superfície regrada** | — (Sweep linha) | Ruled Surface | Ruled Surface | 🟡 Conectar superfícies | BM-SUP-02 |
| SUP-19 | **Filete de superfície** | Shape Fillet / Edge Fillet | Fillet (face) | Fillet (face) | 🟡 Filete de superfície | DR-10 |
| SUP-20 | **Fechar superfície / costurar ao corpo** | Close Surface / Sew Surface | Thicken (create solid) / Replace Face | Sculpt | ✅ Fechar superfície, costurar |  |
| SUP-21 | **Reparo (healing)** | Healing | Import Diagnostics | Repair Bodies | ✅ Reparar |  |
| SUP-22 | **Análises (zebra, curvatura, desvio)** | Connect Checker / Curvature / Distance Analysis | Zebra / Curvature / Deviation | Zebra / Curvature / Surface Analysis | 🟡 Zebra, curvatura | BM-SUP-03 |
| SUP-23 | **Elementos de cada comando** | Padrão do GSD | PropertyManager por grupos | Property panel | 🟡 Painel com abas | DR-25 |

### SUP-01 — Pontos de referência

- **O que faz:** Pontos por coordenadas, em curva, em superfície, entre pontos, centro, tangente.
- **Como funciona:** Suporte: elemento → modo (coordenadas, na curva, na superfície, entre…) → parâmetros: distância/razão → avançado: referencial.
- **Diferenças:** CATIA é o mais completo; o review pede setas 3D do referencial (DR-49).
- **Fenkra:** ✅ completo. PointCoords, PointOnCurve, PointOnSurface, PointBetween, PointCenter, PointTangent…
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-02 — Linhas de referência

- **O que faz:** Linha por 2 pontos, ponto+direção, ângulo, normal à superfície, bissetriz.
- **Como funciona:** Suporte → modo → comprimento/limites → avançado: infinita ou limitada.
- **Fenkra:** ✅ completo. LinePoints, LineDir, LineAngle, LineNormalSurface, LineBisector, LinePlanes, LineAxis.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-03 — Planos de referência

- **O que faz:** Plano offset, ângulo, 3 pontos, 2 linhas, normal à curva, tangente, equação.
- **Como funciona:** Suporte → modo → offset/ângulo → avançado: inverter normal.
- **Fenkra:** 🟡 parcial. Muitos modos; plano de face continua vivo quando a face some (DR-21); plano na curva sem modo por ponto projetado (DR-04).
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-04 — Projeção

- **O que faz:** Projeta curva/ponto numa superfície, normal ou por direção.
- **Como funciona:** Elemento + suporte → normal/direção → solução mais próxima.
- **Fenkra:** ✅ completo. CurveProject, CurveProjectSurf, PointProject.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-05 — Interseção

- **O que faz:** Curva/ponto de interseção entre elementos.
- **Como funciona:** Dois elementos → curva → avançado: estender, manter tudo.
- **Fenkra:** ✅ completo. CurveIntersect.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-06 — Curvas de conexão e canto

- **O que faz:** Conecta duas curvas com continuidade G0–G3; canto com raio.
- **Como funciona:** Duas curvas + pontos → continuidade e tensão; canto → raio, aparar.
- **Fenkra:** ✅ completo. Connect, Corner; teste conector_g3_e_canto_entre_linha_e_spline.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-07 — Spline 3D, hélice, espiral

- **O que faz:** Curvas livres e paramétricas no espaço.
- **Como funciona:** Pontos/eixo → passo, voltas, raio, tangências.
- **Fenkra:** ✅ completo. Spline, SplineControl, Helix, Spiral.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-08 — Extrair / extrair múltiplo

- **O que faz:** Extrai faces, arestas ou curvas como novo elemento, com propagação.
- **Como funciona:** Seleção → propagação (nenhuma, ponto, tangência, curvatura) → limiares de distância/ângulo → avançado: complementar, federação.
- **Diferenças:** Recurso central do CATIA; SW/Inventor usam cópia de faces.
- **Fenkra:** 🟡 parcial. SurfExtract/SurfMulti; a propagação fica só na interface (extract.rs) e não é refeita no recompute; falta no menu de outros comandos (DR-13, DR-14).
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-09 — Superfície extrudada e de revolução

- **O que faz:** Superfícies por extrusão e revolução de curvas.
- **Como funciona:** Curva + direção/eixo → limites: comprimento ou até elemento.
- **Diferenças:** CATIA e SW oferecem até superfície/vértice.
- **Fenkra:** 🟡 parcial. SurfExtrude só com comprimentos: falta "até elemento" (DR-24).
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md) · [SOLIDWORKS Help — Surfaces Toolbar](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_surfaces_toolbar.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-10 — Superfície varrida

- **O que faz:** Varre perfil por guias/spine.
- **Como funciona:** Seção + guia(s) + spine → modo (explícito, linha, círculo, cônica) → torção, escala.
- **Diferenças:** CATIA tem modos por seção implícita (linha, círculo, cônica).
- **Fenkra:** ✅ completo. SurfSweep, SurfSweepX (avançada).
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-11 — Superfície por seções (loft)

- **O que faz:** Superfície entre seções com guias.
- **Como funciona:** Seções + guias + continuidade nas bordas.
- **Diferenças:** SW Boundary trata as duas direções igualmente.
- **Fenkra:** 🟡 parcial. SurfLoft com uma guia.
- **Fontes:** [SOLIDWORKS Help — Surfaces Toolbar](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_surfaces_toolbar.htm) · [Contexto: superfícies](../contextos/superficies.md)

### SUP-12 — Preenchimento (fill)

- **O que faz:** Fecha um contorno com superfície.
- **Como funciona:** Contorno de curvas/arestas (encadeado) → continuidade G0/G1/G2 com faces vizinhas → avançado: ponto de passagem.
- **Diferenças:** Os três aceitam arestas, curvas e sketch.
- **Fenkra:** 🟡 parcial. SurfFill e BoundaryFill não aceitam linhas de sketch nem seleção em cadeia (DR-13).
- **Fontes:** [SOLIDWORKS Help — Filled Surface](https://help.solidworks.com/2024/english/SolidWorks/sldworks/c_Filled_Surface.htm) · [Inventor Help — About Boundary Patches](https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-C9BB978A-D25B-4820-912F-B39CBB6CC7C3.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-13 — Concordância (blend)

- **O que faz:** Superfície de transição entre duas curvas/bordas.
- **Como funciona:** Duas bordas + suportes → continuidade e tensão.
- **Fenkra:** ✅ completo. SurfBlend.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-14 — Offset de superfície

- **O que faz:** Cópia paralela de superfície.
- **Como funciona:** Superfície → distância → lado → avançado: remover subelementos que falham.
- **Fenkra:** ✅ completo. SurfOffset; teste superficies_varredura_avancada_offset_filete…
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-15 — Juntar (join/knit/stitch)

- **O que faz:** Une curvas ou superfícies num elemento só.
- **Como funciona:** Elementos → tolerância de fusão → verificar tangência/conexidade → avançado: simplificar, criar sólido.
- **Fenkra:** 🟡 parcial. CurveJoin e SurfJoin existem; não juntou 4 linhas de sketch (DR-13).
- **Fontes:** [SOLIDWORKS Help — Filled Surface](https://help.solidworks.com/2024/english/SolidWorks/sldworks/c_Filled_Surface.htm) · [Inventor Help — Patch, Stitch, Ruled Surfaces](https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-5A8568DF-5BDA-47B9-97FC-2722B8A78A89.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-16 — Dividir e aparar

- **O que faz:** Corta superfícies/curvas por outro elemento; apara duas mutuamente.
- **Como funciona:** Elemento + ferramenta → lado a manter → avançado: manter os dois lados.
- **Fenkra:** ✅ completo. SurfSplit, SurfTrim.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md) · [Inventor Help — Trim, Extend or Sculpt Surfaces](https://help.autodesk.com/cloudhelp/2022/ENU/Inventor-Help/files/GUID-7661F5F8-2C6C-42E9-8B7F-E7B62F2C7422.htm)

### SUP-17 — Estender / extrapolar

- **O que faz:** Prolonga a borda de uma superfície.
- **Como funciona:** Borda → comprimento ou até elemento → tangente/curvatura.
- **Fenkra:** ✅ completo. SurfExtrapolate.
- **Fontes:** [Inventor Help — Trim, Extend or Sculpt Surfaces](https://help.autodesk.com/cloudhelp/2022/ENU/Inventor-Help/files/GUID-7661F5F8-2C6C-42E9-8B7F-E7B62F2C7422.htm)

### SUP-18 — Superfície regrada

- **O que faz:** Superfície a partir de arestas numa direção (normal, tangente, vetor).
- **Como funciona:** Arestas → tipo e distância.
- **Fenkra:** 🟡 parcial. SurfConnect cobre parte.
- **Fontes:** [Inventor Help — Patch, Stitch, Ruled Surfaces](https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-5A8568DF-5BDA-47B9-97FC-2722B8A78A89.htm)

### SUP-19 — Filete de superfície

- **O que faz:** Filete entre superfícies e em arestas de superfície.
- **Como funciona:** Duas superfícies → raio → aparar suportes.
- **Fenkra:** 🟡 parcial. SurfFillet, SurfEdgeFillet; renderização quebra em filetes de superfície (DR-10).
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### SUP-20 — Fechar superfície / costurar ao corpo

- **O que faz:** Transforma volume fechado em sólido ou costura superfície num corpo.
- **Como funciona:** Superfície fechada → sólido; ou superfície + corpo → substituir faces.
- **Diferenças:** Inventor Sculpt adiciona e remove material ao mesmo tempo.
- **Fenkra:** ✅ completo. CloseSurface, SewSurface.
- **Fontes:** [Inventor Help — Trim, Extend or Sculpt Surfaces](https://help.autodesk.com/cloudhelp/2022/ENU/Inventor-Help/files/GUID-7661F5F8-2C6C-42E9-8B7F-E7B62F2C7422.htm)

### SUP-21 — Reparo (healing)

- **O que faz:** Corrige folgas e bordas de superfícies importadas.
- **Como funciona:** Superfícies → tolerâncias → fechar folgas.
- **Fenkra:** ✅ completo. SurfHeal.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-22 — Análises (zebra, curvatura, desvio)

- **O que faz:** Verifica continuidade e qualidade.
- **Como funciona:** Superfície → tipo de análise → escala de cores.
- **Fenkra:** 🟡 parcial. Zebra existe; desvio e checagem de continuidade parciais.
- **Fontes:** [Contexto: superfícies](../contextos/superficies.md)

### SUP-23 — Elementos de cada comando

- **O que faz:** Filosofia: suporte → modo → parâmetros → avançado; extensões com "até elemento".
- **Como funciona:** Todo comando pergunta primeiro o onde, depois o como, o quanto e o refinamento.
- **Fenkra:** 🟡 parcial. Painéis com abas e ícones (alpha.25); ordem e "até elemento" ainda não uniformes (DR-25..29).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)


## Chapa metálica

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| CHP-01 | **Parâmetros de chapa** | Sheet Metal Parameters | Sheet Metal (gauge table, K-factor, bend allowance) | Sheet Metal Defaults / Rules | ✅ Parâmetros de chapa |  |
| CHP-02 | **Parede base** | Wall | Base Flange/Tab | Face | ✅ Parede base |  |
| CHP-03 | **Aba em aresta** | Wall On Edge / Flange | Edge Flange | Flange | 🟡 Aba | DR-55 |
| CHP-04 | **Aba em cadeia (reta + curva)** | Flange (sweep walls) | Miter Flange / Swept Flange | Contour Flange / Flange em loop | 🟡 Aba curva | DR-54 |
| CHP-05 | **Aba por perfil (contorno)** | User Flange | Miter Flange | Contour Flange | ✅ Aba por perfil |  |
| CHP-06 | **Aba por superfície (suporte)** | Surfacic Flange (Aerospace SM) | — | — | 🟡 Aba por superfície | DR-19 |
| CHP-07 | **Aba entre perfis (lofted)** | — (Hopper / Generative) | Lofted Bend | Lofted Flange | ❌ — | BM-CHP-01 |
| CHP-08 | **Bainha e gota** | Hem / Tear Drop | Hem | Hem | ✅ Bainha |  |
| CHP-09 | **Joggle** | Jog | Jog | — (Flange duplo) | ✅ Joggle |  |
| CHP-10 | **Dobra por linha** | Bend From Flat | Sketched Bend | Fold | 🟡 Dobra por linha | DR-22 |
| CHP-11 | **Recorte na chapa** | Cut Out | Extruded Cut (Normal cut) | Cut (Cut Across Bend) | 🟡 Recorte | DR-20 |
| CHP-12 | **Alívios de canto e dobra** | Corner Relief | Corner Relief / Closed Corner | Corner Round / Corner Seam | ✅ Alívio de canto |  |
| CHP-13 | **Estampos, veneziana e furo flangeado** | Stamping (Louver, Bead, Bridge, Flanged Hole…) | Forming Tools | Punch Tool | ✅ Estampo, veneziana, furo flangeado |  |
| CHP-14 | **Planificação e DXF** | Fold/Unfold / Save As DXF | Flatten / Flat Pattern | Flat Pattern | ✅ Dobrar/Desdobrar |  |
| CHP-15 | **Desdobrar parcial** | Unfold / Fold | Unfold / Fold | Unfold / Refold | ❌ — | BM-CHP-02 |
| CHP-16 | **Converter sólido em chapa** | Recognize | Convert to Sheet Metal / Insert Bends | Convert to Sheet Metal | ❌ — | BM-CHP-03 |
| CHP-17 | **Comando de aba unificado** | Wall On Edge (tipos) | Edge/Miter/Swept separados | Flange/Contour separados | 🟡 vários comandos | DR-54 |

### CHP-01 — Parâmetros de chapa

- **O que faz:** Espessura, raio, fator K/tabela de dobra, alívio padrão, material.
- **Como funciona:** Definidos uma vez por peça (regra/estilo) → herdados pelas features → sobrescrita por feature.
- **Diferenças:** Inventor usa Regras e Estilos reaproveitáveis entre peças.
- **Fenkra:** ✅ completo. SheetParams: material, bitolas, método/fator K, sugestão por tabela.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm) · [Inventor Help — Contour Flange Property Panel](https://help.autodesk.com/cloudhelp/2027/ENU/Inventor-Help/files/Contour-Flange-Property-Panel-Reference.htm)

### CHP-02 — Parede base

- **O que faz:** Primeira parede a partir de um perfil fechado (ou aberto com espessura).
- **Como funciona:** Perfil → lado da espessura → avançado: offset, plano médio.
- **Fenkra:** ✅ completo. SmWall (sketch ou superfície, offset, mid).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-03 — Aba em aresta

- **O que faz:** Dobra uma parede a partir de uma aresta reta.
- **Como funciona:** Aresta(s) → modo de comprimento (externo/interno) e posição da dobra → ângulo, raio → avançado: recuos laterais, alívio, inverter, meia-esquadria com vizinha.
- **Diferenças:** SW define a posição da dobra (material dentro/fora, dobra fora, virtual sharp); Inventor aceita cadeia de arestas.
- **Fenkra:** 🟡 parcial. SmFlange com lado, meia-esquadria, recuo mínimo; bugs em dobras em sequência, múltiplas dobras e abas vizinhas compostas (DR-55..57).
- **Fontes:** [SOLIDWORKS Help — Bend Positions](https://help.solidworks.com/2021/english/SolidWorks/sldworks/c_Bend_Positions.htm) · [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### CHP-04 — Aba em cadeia (reta + curva)

- **O que faz:** Aba que acompanha arestas tangentes, retas e curvas.
- **Como funciona:** Cadeia por tangência → raio, ângulo → planifica pelo comprimento desenvolvido.
- **Diferenças:** CATIA tem Swept Walls: Flange, Hem, Tear Drop, User Flange.
- **Fenkra:** 🟡 parcial. SmCurveFlange (alpha.24–26); bug ao gerar em aresta com curva e reta (DR-54).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### CHP-05 — Aba por perfil (contorno)

- **O que faz:** Parede de perfil aberto ao longo de aresta(s).
- **Como funciona:** Perfil aberto + arestas → raios automáticos → comprimento/extensão.
- **Fenkra:** ✅ completo. SmProfileFlange.
- **Fontes:** [Inventor Help — Contour Flange Property Panel](https://help.autodesk.com/cloudhelp/2027/ENU/Inventor-Help/files/Contour-Flange-Property-Panel-Reference.htm) · [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf)

### CHP-06 — Aba por superfície (suporte)

- **O que faz:** Aba que se apoia numa superfície de suporte; relimita a base.
- **Como funciona:** Suporte + web → modo exato/aproximação/ângulo → EOP (comprimento), raio, lado do material.
- **Diferenças:** Exclusivo do CATIA Aerospace Sheet Metal.
- **Fenkra:** 🟡 parcial. SmSurfFlange com 3 modos (alpha.26); falhou nos testes do usuário, inclusive nervura de asa (DR-19, DR-23).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md) · [Contexto: aba por superfície (pedido M8 do backlog)](../backlog/historico/backlog-alpha14.md)

### CHP-07 — Aba entre perfis (lofted)

- **O que faz:** Transição de chapa entre dois perfis (funis, adaptadores).
- **Como funciona:** Dois perfis → saída por dobradeira (facetada) ou estampada (lisa).
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Lofted Flange Features](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-B6271A15-FD68-4672-9461-C86F8D55DA87.htm)

### CHP-08 — Bainha e gota

- **O que faz:** Dobra a borda sobre si (fechada, aberta, gota, rolada).
- **Como funciona:** Aresta → tipo → comprimento, folga, raio.
- **Fenkra:** ✅ completo. SmHem (tipos).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-09 — Joggle

- **O que faz:** Desvio em Z por duas dobras.
- **Como funciona:** Linha no sketch → altura, ângulo → manter comprimento.
- **Fenkra:** ✅ completo. SmJoggle (ângulo, plano, keep_length).
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-10 — Dobra por linha

- **O que faz:** Dobra uma face plana ao longo de uma linha de sketch.
- **Como funciona:** Linha → lado fixo → ângulo, raio, posição da linha.
- **Fenkra:** 🟡 parcial. SmBend só com reta a–b; não gera em spline e não emenda os raios (DR-17, DR-22).
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### CHP-11 — Recorte na chapa

- **O que faz:** Corte normal à chapa, válido dobrado e planificado.
- **Como funciona:** Perfil → feito no dobrado ou no planificado → profundidade.
- **Diferenças:** Inventor tem "Cut Across Bend".
- **Fenkra:** 🟡 parcial. SmCut; planificado→dobrado falha quando atravessa o raio (DR-20).
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### CHP-12 — Alívios de canto e dobra

- **O que faz:** Alívio no encontro de abas e na ponta das dobras.
- **Como funciona:** Canto/dobra → forma (redondo, quadrado, gota, rasgo) → tamanho.
- **Diferenças:** SW tem Closed Corner; Inventor tem Corner Seam.
- **Fenkra:** ✅ completo. SmCornerRelief; relief_kind na aba.
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-13 — Estampos, veneziana e furo flangeado

- **O que faz:** Formas de estampagem.
- **Como funciona:** Ponto/face → ferramenta → dimensões.
- **Fenkra:** ✅ completo. SmStamp, SmLouver, SmFlangedHole.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Sheet Metal Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf) · [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-14 — Planificação e DXF

- **O que faz:** Estado planificado com linhas de dobra e exportação.
- **Como funciona:** Alternar dobrado/planificado → tabela de dobras → DXF.
- **Fenkra:** ✅ completo. SmUnfold; bend_lines; tabela de dobras; DXF no desenho.
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-15 — Desdobrar parcial

- **O que faz:** Desdobra só algumas dobras para editar e redobra.
- **Como funciona:** Dobras escolhidas → editar → redobrar.
- **Fenkra:** ❌ não existe.
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-16 — Converter sólido em chapa

- **O que faz:** Reconhece dobras num sólido de espessura constante.
- **Como funciona:** Face fixa → espessura → arestas de dobra.
- **Fenkra:** ❌ não existe.
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### CHP-17 — Comando de aba unificado

- **O que faz:** Um comando com tipo (aresta, cadeia, perfil, superfície) que abre os campos do tipo.
- **Como funciona:** Escolha do tipo → caixas do tipo se abrem abaixo.
- **Fenkra:** 🟡 parcial. Hoje são comandos separados (DR-54).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)


## Estrutural e soldas

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| EST-01 | **Membro estrutural** | Structure Design: Profile | Structural Member | Frame Generator: Insert Frame | ✅ Membro |  |
| EST-02 | **Tratamento de cantos e uniões** | Endcut (Snipe, Trim, Weld) | Trim/Extend (End Miter, Butt1/2) | Miter / Notch / Corner Joint / Trim-Extend | 🟡 Cantos e uniões | BM-EST-01 |
| EST-03 | **Biblioteca de perfis** | Catálogos de seções | Weldment Profiles | Content Center (estrutural) | 🟡 Biblioteca de perfis | CTX-PRF |
| EST-04 | **Chapas, reforços e tampas** | Plate / Stiffener | Gusset / End Cap | — (peças normais) | ✅ Placa, reforço |  |
| EST-05 | **Cordões de solda** | Weld Design | Weld Bead / Fillet Bead | Weldment: Welds | 🟡 Símbolo de solda | BM-EST-02 |
| EST-06 | **Lista de corte** | BOM de estrutura | Cut List | Frame Member Info / Parts List | 🟡 Lista de corte | DR-33 |
| EST-07 | **Corpos iguais → peça + instâncias** | — (via Product) | Cut List folders | Frame Generator (peças por membro) | ❌ — | CTX-EST |
| EST-08 | **Estrutura Pai → montagem dinâmica** | Product a partir de Part (manual) | Insert into New Part / Save Bodies | Frame Generator assembly | ❌ — | CTX-EST |
| EST-09 | **Otimização de barras** | — | — (add-ins) | — | 🟡 Barras (cálculo) | BM-EST-03 |

### EST-01 — Membro estrutural

- **O que faz:** Perfil aplicado a linhas/arestas de um esqueleto.
- **Como funciona:** Esqueleto (sketch 3D, arestas) → família/tamanho → ancoragem, rotação, espelhar → avançado: grupos, ponto de inserção.
- **Fenkra:** ✅ completo. StructMember (paths, anchor, angle, offsets).
- **Fontes:** [SOLIDWORKS Help — Weldments Toolbar](https://help.solidworks.com/2021/english/solidworks/sldworks/r_weldments_toolbar.htm) · [Inventor Help — Frame Generator overview](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-953F560A-C2D3-4031-8348-762054C7C779.htm) · [pycatia — API de Structure Design (endcuts)](https://pycatia.readthedocs.io/en/latest/api/pycatia/cat_str_functional_interfaces/sfm_endcut.html)

### EST-02 — Tratamento de cantos e uniões

- **O que faz:** Meia-esquadria, topo, aparar/estender, entalhe (notch).
- **Como funciona:** Par de membros → tipo de canto → folga de solda → ordem.
- **Diferenças:** Inventor separa Notch e Corner Joint; SW tem folga de solda.
- **Fenkra:** 🟡 parcial. CornerMode e joints; sem entalhe (notch) por perfil.
- **Fontes:** [SOLIDWORKS Help — Trimming Segments (weldments)](https://help.solidworks.com/2025/English/SolidWorks/sldworks/t_Trimming_Segments.htm) · [Inventor Help — End treatments to frames](https://help.autodesk.com/cloudhelp/2017/ENU/Inventor-Help/files/GUID-BE98918A-2419-4432-AAE5-46A3E56552FE.htm)

### EST-03 — Biblioteca de perfis

- **O que faz:** Famílias por norma com dimensões e peso/m.
- **Como funciona:** Norma → família → designação → dados.
- **Fenkra:** 🟡 parcial. Poucas famílias (tubos, W, U, L, barras, T); faltam IPE/HEA/HEB/RHS/CHS EN e ABNT com dimensões (CTX-PRF).
- **Fontes:** [Contexto: estrutural](../contextos/estrutural.md) · [Inventor Help — Frame Generator overview](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-953F560A-C2D3-4031-8348-762054C7C779.htm)

### EST-04 — Chapas, reforços e tampas

- **O que faz:** Gussets, tampas de ponta e placas de base.
- **Como funciona:** Membros/faces → forma → espessura, chanfro.
- **Fenkra:** ✅ completo. StructPlate, StructGusset; teste estrutural_quadro_cantos_placa_reforco.
- **Fontes:** [SOLIDWORKS Help — Weldments Toolbar](https://help.solidworks.com/2021/english/solidworks/sldworks/r_weldments_toolbar.htm)

### EST-05 — Cordões de solda

- **O que faz:** Cordões 3D e símbolos de solda.
- **Como funciona:** Arestas/faces → tipo e tamanho → símbolo.
- **Fenkra:** 🟡 parcial. Símbolo ISO 2553 no desenho/PMI; sem cordão 3D.
- **Fontes:** [SOLIDWORKS Help — Weldments Toolbar](https://help.solidworks.com/2021/english/solidworks/sldworks/r_weldments_toolbar.htm)

### EST-06 — Lista de corte

- **O que faz:** Agrupa membros iguais com comprimento, ângulos e peso.
- **Como funciona:** Agrupamento automático → propriedades por item → tabela no desenho.
- **Fenkra:** 🟡 parcial. cut_list() agrupa por designação + comprimento; balões não funcionam para estrutural (DR-33).
- **Fontes:** [SOLIDWORKS Help — Weldment Drawings](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Weldment_Drawings.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### EST-07 — Corpos iguais → peça + instâncias

- **O que faz:** Detecta corpos idênticos e gera uma peça com várias instâncias.
- **Como funciona:** Comparação (perfil, comprimento, cortes, material) → grupos → peça base + ocorrências ligadas.
- **Diferenças:** Inventor gera arquivos por membro; SW agrupa só na lista.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Contexto: estrutural](../contextos/estrutural.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### EST-08 — Estrutura Pai → montagem dinâmica

- **O que faz:** Gera montagem, subpeças linkadas e submontagens de solda a partir do estrutural.
- **Como funciona:** Estrutura Pai → grupos/submontagens → subparts linkados → PMI e desenho por peça.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Contexto: estrutural](../contextos/estrutural.md) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### EST-09 — Otimização de barras

- **O que faz:** Distribui cortes em barras comerciais.
- **Como funciona:** Lista de corte + barra padrão → plano de corte.
- **Fenkra:** 🟡 parcial. Cálculo de barras na lista de corte; sem plano de corte otimizado.
- **Fontes:** [Contexto: estrutural](../contextos/estrutural.md)


## Montagem

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| MON-01 | **Inserir componente e fixar** | Existing Component / Fix | Insert Component / Fix | Place / Ground | ✅ Inserir, fixar |  |
| MON-02 | **Restrições padrão** | Coincidence / Contact / Offset / Angle | Standard Mates | Constraints (Mate, Flush, Angle, Tangent, Insert) | ✅ Restrições |  |
| MON-03 | **Restrição rápida / inteligente** | Quick Constraint | SmartMates | Assemble / Auto | ✅ Restrição rápida |  |
| MON-04 | **Juntas cinemáticas** | Kinematics: Joints | Mechanical mates / Hinge | Joints | ✅ Juntas |  |
| MON-05 | **Acoplamentos mecânicos** | Gear / Rack / Cable joints | Gear / Rack and Pinion / Cam / Screw mates | Motion: Rotation / Rotation-Translation | ✅ Engrenagem, cremalheira, polia, came |  |
| MON-06 | **Restrições avançadas** | — (Offset + parâmetros) | Width / Symmetric / Path / Limit | Symmetry / Transitional | ❌ — | BM-MON-01 |
| MON-07 | **Graus de liberdade** | Degrees of Freedom | Under defined (-) | Degrees of Freedom | ✅ GDL na árvore |  |
| MON-08 | **Padrão de componentes** | Reuse Pattern | Component Pattern (feature-driven) | Pattern Component | ✅ Padrão |  |
| MON-09 | **Interferência e colisão** | Clash | Interference / Collision Detection | Analyze Interference | ✅ Interferência |  |
| MON-10 | **Vista explodida** | Explode | Exploded View | Presentation | ✅ Explodida |  |
| MON-11 | **Peça no contexto e links** | Contextual design / Publications | In-context (external refs) | Adaptive / Derived | 🟡 Peça no contexto, copiar geometria | DR-31 |
| MON-12 | **Recursos de montagem** | Assembly Features | Assembly Feature (Hole/Cut) | Assembly Features | ✅ Recorte pela peça, furo entre peças |  |
| MON-13 | **Origem da montagem** | — (produto sem origem; usa peças) | Origin + reference planes | Origin | ❌ — | DR-31 |
| MON-14 | **Seleção e edição na árvore** | Product tree | FeatureManager | Model browser | 🟡 Árvore da montagem | DR-32 |
| MON-15 | **Lista de materiais (BOM)** | Bill of Material | BOM | Bill of Materials | ✅ BOM |  |
| MON-16 | **Fixadores automáticos** | — (Catalog) | Smart Fasteners | Bolted Connection Generator | ❌ — | CTX-FIX |

### MON-01 — Inserir componente e fixar

- **O que faz:** Insere peças/submontagens; fixa a primeira.
- **Como funciona:** Arquivo → posição → fixo/flutuante → modo leve.
- **Fenkra:** ✅ completo. AsmInsert, AsmFix, modos editável/visualização.
- **Fontes:** [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm) · [Wichita State NIAR — CATIA V5R30 Assembly Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Assembly-Design-R30.pdf)

### MON-02 — Restrições padrão

- **O que faz:** Coincidente, contato, distância, ângulo, paralelo, perpendicular, concêntrico.
- **Como funciona:** Duas entidades → tipo (ou automático) → valor → inverter alinhamento.
- **Diferenças:** Inventor tem Insert (concêntrico + plano numa só).
- **Fenkra:** ✅ completo. MateCmd: Coincident, Contact, Distance, Angle, Parallel, Perpendicular, Auto, AlignOnce.
- **Fontes:** [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm) · [City University London — CATIA Assembly Design: Constraints toolbar](https://www.staff.city.ac.uk/~ra600/ME2105/Catia%20course/CATIA%20Tutorials/asmug_C2/asmugwd0400.htm)

### MON-03 — Restrição rápida / inteligente

- **O que faz:** Deduz a restrição pela seleção ou ao arrastar.
- **Como funciona:** Seleção de 2 entidades → proposta → confirmar.
- **Fenkra:** ✅ completo. quick_options, after_view_quick.
- **Fontes:** [City University London — CATIA Assembly Design: Constraints toolbar](https://www.staff.city.ac.uk/~ra600/ME2105/Catia%20course/CATIA%20Tutorials/asmug_C2/asmugwd0400.htm) · [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm)

### MON-04 — Juntas cinemáticas

- **O que faz:** Rígida, rotação, deslizante, cilíndrica, planar, esférica.
- **Como funciona:** Duas origens (frames) → tipo → limites → drive.
- **Diferenças:** Inventor escolhe a junta pela geometria (automático).
- **Fenkra:** ✅ completo. AsmJoint, kin.rs; limites e drives.
- **Fontes:** [Inventor Help — Create Joints Reference](https://help.autodesk.com/cloudhelp/2023/ENU/Inventor-Help/files/GUID-6AA68E8F-7C97-4806-8483-3941DE915E70.htm) · [Inventor Help — Define and Manage Joint Relationships](https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-21DC3336-5C51-42C1-90FB-4299CD66E0C6.htm)

### MON-05 — Acoplamentos mecânicos

- **O que faz:** Engrenagem, cremalheira, polia, came, parafuso.
- **Como funciona:** Duas juntas/eixos → razão.
- **Fenkra:** ✅ completo. MateKind Gear/RackPinion; couplings; teste came_e_seguidor.
- **Fontes:** [SOLIDWORKS Help — Mechanical Mates](https://help.solidworks.com/2022/English/SolidWorks/sldworks/c_Mechanical_Mates.htm)

### MON-06 — Restrições avançadas

- **O que faz:** Largura (centrar), simetria, caminho, limite.
- **Como funciona:** Pares de faces/curva → regra.
- **Fenkra:** ❌ não existe. Sem largura, simetria ou caminho.
- **Fontes:** [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm)

### MON-07 — Graus de liberdade

- **O que faz:** Mostra o que falta restringir em cada componente.
- **Como funciona:** Solver → GDL por peça → ícones na árvore.
- **Fenkra:** ✅ completo. dof_per_comp; selos na árvore.
- **Fontes:** [Wichita State NIAR — CATIA V5R30 Assembly Design (apostila)](https://wichita.edu/industry_and_defense/NIAR/Documents/Assembly-Design-R30.pdf)

### MON-08 — Padrão de componentes

- **O que faz:** Repete componentes por padrão da feature ou linear/circular.
- **Como funciona:** Componente → padrão → associado ao padrão de furos.
- **Fenkra:** ✅ completo. AsmPattern; repeticao_acompanha_a_quantidade.
- **Fontes:** [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm)

### MON-09 — Interferência e colisão

- **O que faz:** Detecta volumes sobrepostos; colisão ao mover.
- **Como funciona:** Componentes → análise → lista.
- **Fenkra:** ✅ completo. AsmInterf; colisão ao arrastar.
- **Fontes:** [SOLIDWORKS Help — Mechanical Mates](https://help.solidworks.com/2022/English/SolidWorks/sldworks/c_Mechanical_Mates.htm)

### MON-10 — Vista explodida

- **O que faz:** Passos de explosão e animação.
- **Como funciona:** Componentes → direção/distância → passos.
- **Fenkra:** ✅ completo. Composer; teste explosao_e_animacao.
- **Fontes:** [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm)

### MON-11 — Peça no contexto e links

- **O que faz:** Cria/edita peça dentro da montagem referenciando outras.
- **Como funciona:** Editar no lugar → referências externas → controle de links.
- **Fenkra:** 🟡 parcial. AsmNewPart, AsmLink; peças não 100% editáveis vendo as outras (DR-31).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### MON-12 — Recursos de montagem

- **O que faz:** Furo e corte que atravessam várias peças.
- **Como funciona:** Feature na montagem → peças afetadas.
- **Fenkra:** ✅ completo. AsmFeat(0/1).
- **Fontes:** [SOLIDWORKS Help — Types of Mates](https://help.solidworks.com/2021/english/SolidWorks/sldworks/r_Types_of_Mates_SWassy.htm)

### MON-13 — Origem da montagem

- **O que faz:** Planos/eixos de referência próprios da montagem.
- **Como funciona:** Sempre presentes; usados para fixar e restringir.
- **Fenkra:** ❌ não existe. Sem origem na montagem (DR-31).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### MON-14 — Seleção e edição na árvore

- **O que faz:** Ctrl/Shift/Alt, ocultar, apagar, isolar seguindo a mesma lógica das peças.
- **Como funciona:** Seleção múltipla → ações em lote → isolar.
- **Fenkra:** 🟡 parcial. Ctrl+clique; sem Alt; ocultar/apagar diferentes das peças (DR-32).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### MON-15 — Lista de materiais (BOM)

- **O que faz:** Estrutura e lista de peças com quantidades e propriedades.
- **Como funciona:** Montagem → níveis → colunas.
- **Fenkra:** ✅ completo. AsmBom; numeração de itens.
- **Fontes:** [SOLIDWORKS Help — Weldment Drawings](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Weldment_Drawings.htm)

### MON-16 — Fixadores automáticos

- **O que faz:** Insere parafusos/porcas/arruelas nos furos com restrições.
- **Como funciona:** Furos reconhecidos → norma/tamanho → pilha → restrições.
- **Fenkra:** ❌ não existe.
- **Fontes:** [SOLIDWORKS Help — Smart Fasteners Overview](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Smart_Fasteners_Overview.htm) · [Inventor Help — Design Accelerator](https://help.autodesk.com/cloudhelp/2015/ENU/Inventor-Help/files/GUID-24104648-BE41-49C7-9DD0-1AF2BAFC102E.htm) · [Contexto: smart fastener](../contextos/smart-fastener.md)


## Desenho técnico

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| DES-01 | **Folhas, formatos e carimbo** | Sheet / Background View | Sheet Format | Sheet / Border / Title Block | ✅ Folhas, carimbo |  |
| DES-02 | **Vistas base e projetadas** | Front / Projection View | Model / Projected View | Base / Projected View | 🟡 Vistas | BM-DES-01 |
| DES-03 | **Cortes e seções** | Section View / Cut | Section View | Section View | ✅ Corte |  |
| DES-04 | **Detalhe, auxiliar, quebrada, corte parcial** | Detail / Auxiliary / Broken / Breakout | Detail / Auxiliary / Break / Broken-out Section | Detail / Auxiliary / Break / Break Out | 🟡 Detalhe, auxiliar | BM-DES-02 |
| DES-05 | **Cotas associativas** | Dimensions | Smart Dimension / Model Items | Dimension / Retrieve | ✅ Cotas |  |
| DES-06 | **Hachuras** | Area Fill / Pattern | Area Hatch | Hatch | 🟡 Hachura | BM-DES-03 |
| DES-07 | **Atualização e estado da vista** | Update / Update Status | Rebuild / Out of date | Update | ❌ Recarrega silencioso | DR-34 |
| DES-08 | **Balões e lista de peças** | Balloon / BOM | AutoBalloon / BOM / Cut List | Auto Balloon / Parts List | 🟡 Balões, BOM | DR-33 |
| DES-09 | **Tabelas** | Tables | Hole / Bend / Revision / Cut List Table | Hole / Revision / General Table | ✅ Tabelas |  |
| DES-10 | **Propriedades por norma e herança** | Standards + Properties | Document Properties (Drafting Standard) | Styles and Standards | 🟡 Estilos | DR-35 |
| DES-11 | **Desenho de estrutura soldada** | — | Weldment drawings (body views) | Frame drawings | ❌ — | DR-45 |
| DES-12 | **Exportação** | Save As (PDF, DXF, DWG) | Save As | Export | ✅ PDF, SVG, DXF |  |

### DES-01 — Folhas, formatos e carimbo

- **O que faz:** Várias folhas, formato, norma, carimbo e fundo.
- **Como funciona:** Folha → formato/norma → carimbo (fundo separado do trabalho).
- **Fenkra:** ✅ completo. Formatos ISO/ANSI, carimbos ISO 7200/ABNT, zonas ISO 5457.
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)

### DES-02 — Vistas base e projetadas

- **O que faz:** Vistas ortográficas a partir do 3D.
- **Como funciona:** Modelo → vista base → projetadas → escala/estilo.
- **Fenkra:** 🟡 parcial. Só 1º diedro; sem 3º diedro (ASME).
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)

### DES-03 — Cortes e seções

- **O que faz:** Vista em corte (total, meio, alinhado) e seção removida.
- **Como funciona:** Linha de corte → tipo → hachura.
- **Fenkra:** ✅ completo. Teste vistas_corte_detalhe_cotas_tabelas_e_exportacao.
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)

### DES-04 — Detalhe, auxiliar, quebrada, corte parcial

- **O que faz:** Ampliação, vista auxiliar, vista interrompida e break-out.
- **Como funciona:** Região/aresta → tipo → escala.
- **Fenkra:** 🟡 parcial. Detalhe e auxiliar; sem vista interrompida nem corte parcial.
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)

### DES-05 — Cotas associativas

- **O que faz:** Cotas ligadas à geometria do 3D.
- **Como funciona:** Seleção → tipo → estilo → tolerância.
- **Fenkra:** ✅ completo. Cotas com estilo, setas e posição do texto.
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)

### DES-06 — Hachuras

- **O que faz:** Padrão, ângulo, espaçamento por material.
- **Como funciona:** Região de corte → padrão do material → sobrescrita.
- **Fenkra:** 🟡 parcial. Passo fixo 2,5 sem propriedades de padrão/ângulo (layout.rs).
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)

### DES-07 — Atualização e estado da vista

- **O que faz:** Vista desatualizada, links quebrados, atualizar manual/automático.
- **Como funciona:** Mudança no 3D → marca vistas → Update.
- **Fenkra:** ❌ não existe. Models::refresh() recarrega por data do arquivo; sem estado visível.
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### DES-08 — Balões e lista de peças

- **O que faz:** Balões automáticos ligados à BOM/lista de corte.
- **Como funciona:** Vista → balões automáticos → tabela.
- **Fenkra:** 🟡 parcial. Balões ligados à BOM; não funciona para estrutural (DR-33).
- **Fontes:** [SOLIDWORKS Help — Weldment Drawings](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Weldment_Drawings.htm) · [Inventor Help — Add balloons to drawing views](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-491D178A-5C7A-485F-902A-46B1A02D1D09.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### DES-09 — Tabelas

- **O que faz:** Furos, dobras, revisões, lista de corte.
- **Como funciona:** Vista/modelo → tabela → colunas.
- **Fenkra:** ✅ completo. TableKind (BOM, CutList, …).
- **Fontes:** [SOLIDWORKS Help — Weldment Drawings](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Weldment_Drawings.htm)

### DES-10 — Propriedades por norma e herança

- **O que faz:** Norma → folha → vista → elemento, com sobrescrita.
- **Como funciona:** Propriedade definida no nível mais alto e herdada.
- **Fenkra:** 🟡 parcial. Estilos por cota/nota; sem cadeia norma→folha→vista completa (DR-35..43).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### DES-11 — Desenho de estrutura soldada

- **O que faz:** Vistas por corpo, lista de corte, detalhes de ponta.
- **Como funciona:** Corpo → vista própria → item da lista.
- **Fenkra:** ❌ não existe. Sem vistas por corpo ligadas à lista (DR-44, DR-45).
- **Fontes:** [SOLIDWORKS Help — Weldment Drawings](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Weldment_Drawings.htm) · [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### DES-12 — Exportação

- **O que faz:** PDF, DXF/DWG, SVG, imagens.
- **Como funciona:** Folha(s) → formato → escala/camadas.
- **Fenkra:** ✅ completo. export.rs; teste de exportação.
- **Fontes:** [Wichita State NIAR — ementas de CATIA (Drafting, FT&A, GSD)](https://www.wichita.edu/research/NIAR/Laboratories/cad-cam/course-descriptions.php)


## PMI 3D

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| PMI-01 | **Cotas e notas 3D** | FT&A: Dimensions / Text | DimXpert / MBD Dimension | 3D Annotations | ✅ Cota, nota |  |
| PMI-02 | **GD&T, referências e rugosidade** | Geometrical Tolerance / Datum / Roughness | Geometric Tolerance / Datum / Surface Finish | Tolerance Feature / Datum | ✅ GD&T, datum, rugosidade, solda |  |
| PMI-03 | **Vistas e capturas** | Annotation Planes / Captures | Annotation Views / 3D Views | Model States / Views | ✅ Capturas |  |
| PMI-04 | **Realce e chamadas inteligentes** | Leaders / Highlight | Leaders | Leaders | 🟡 Seta | DR-48 |
| PMI-05 | **Consultor de tolerância** | — | DimXpert (auto dimension scheme) | Tolerance Advisor | ❌ — | BM-PMI-01 |

### PMI-01 — Cotas e notas 3D

- **O que faz:** Cotas, notas e textos no modelo.
- **Como funciona:** Entidades → tipo → plano de anotação → vista.
- **Fenkra:** ✅ completo. Pmi(0..10).
- **Fontes:** [SOLIDWORKS Help — MBD annotations](https://help.solidworks.com/2025/English/SolidWorks/Sldworks/t_mbd_hiding_showing_annot_parts_assemb.htm) · [Autodesk — GD&T and Model-Based Definition](https://www.autodesk.com/blogs/design-and-manufacturing/gdt-guide-how-geometric-dimensioning-and-tolerancing-supports-model-based-definition/)

### PMI-02 — GD&T, referências e rugosidade

- **O que faz:** Quadros de tolerância, datums, acabamento, solda.
- **Como funciona:** Face/feature → quadro → datum.
- **Fenkra:** ✅ completo. NoteKind Gdt/Datum; símbolo de solda ISO 2553.
- **Fontes:** [SOLIDWORKS Help — MBD annotations](https://help.solidworks.com/2025/English/SolidWorks/Sldworks/t_mbd_hiding_showing_annot_parts_assemb.htm)

### PMI-03 — Vistas e capturas

- **O que faz:** Vistas 3D com conjunto de anotações, câmera e estado.
- **Como funciona:** Plano de anotação → captura → câmera.
- **Fenkra:** ✅ completo. Capturas na árvore.
- **Fontes:** [SOLIDWORKS Help — MBD annotations](https://help.solidworks.com/2025/English/SolidWorks/Sldworks/t_mbd_hiding_showing_annot_parts_assemb.htm)

### PMI-04 — Realce e chamadas inteligentes

- **O que faz:** Realçar as entidades da anotação e várias chamadas (leaders).
- **Como funciona:** Selecionar entidades → realce → adicionar setas.
- **Fenkra:** 🟡 parcial. Realce falha na montagem; uma seta só; seta atravessa a anotação (DR-31, DR-47, DR-48).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### PMI-05 — Consultor de tolerância

- **O que faz:** Verifica se a peça está totalmente cotada/tolerada.
- **Como funciona:** Análise das features → avisos.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Autodesk — GD&T and Model-Based Definition](https://www.autodesk.com/blogs/design-and-manufacturing/gdt-guide-how-geometric-dimensioning-and-tolerancing-supports-model-based-definition/)


## Navegação

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| NAV-01 | **Órbita em torno de alvo** | Rotate (target fixo) | Rotate View | Free Orbit | ✅ Órbita |  |
| NAV-02 | **Recentrar no ponto sob o cursor** | Clique do meio no ponto / Center Graph | Middle-click vertex/edge/face | Clique durante o Orbit / Center (SteeringWheels) | 🟡 Menu "Centro de órbita aqui" | CTX-CAM |
| NAV-03 | **Órbita com eixo vertical (turntable)** | — | — (Rotate about Up) | Constrained Orbit | ❌ — | CTX-CAM |
| NAV-04 | **Zoom no cursor e zoom por área** | Zoom (CTRL+meio) | Zoom to pointer / Zoom to Area | Zoom at cursor / Zoom Window | 🟡 Zoom (distância) | CTX-CAM |
| NAV-05 | **Estilos de navegação** | — | — | — | ✅ Estilos: Fenkra, SolidWorks, Fusion, CATIA, Blender, Onshape, FreeCAD, Inventor, Rhino |  |
| NAV-06 | **Cubo de vistas e vistas nomeadas** | Quick view / Named views | View Orientation / View Selector | ViewCube | ✅ Cubo de vistas, estados de exibição |  |

### NAV-01 — Órbita em torno de alvo

- **O que faz:** Gira a vista em torno de um ponto alvo fixo.
- **Como funciona:** Botão/combinação → alvo → arrasto.
- **Fenkra:** ✅ completo. camera.target; estilos de botões por NavStyle.
- **Fontes:** [Contexto: câmera](../contextos/camera.md) · [Inventor Help — More about Orbit](https://help.autodesk.com/cloudhelp/2014/ENU/Inventor/files/GUID-5E6B1A06-1043-45C4-A3C3-F0959939EB3B.htm)

### NAV-02 — Recentrar no ponto sob o cursor

- **O que faz:** Define o centro da órbita no ponto 3D apontado.
- **Como funciona:** Gesto (duplo clique do meio, ou clique durante a órbita) → raycast → novo alvo.
- **Fenkra:** 🟡 parcial. Só pelo menu do botão direito; sem gesto nem pivô automático sob o cursor.
- **Fontes:** [Contexto: câmera](../contextos/camera.md) · [SOLIDWORKS Help — Middle Mouse Button Functions](https://help.solidworks.com/2021/English/SolidWorks/sldworks/r_Middle_Mouse_Button.htm) · [Inventor Help — SteeringWheels Reference](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-5D371AC3-EBC4-4B8D-9A9B-4B9B2B290D8B.htm)

### NAV-03 — Órbita com eixo vertical (turntable)

- **O que faz:** Gira em torno do eixo vertical do modelo.
- **Como funciona:** Modo restrito → eixo Up.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — More about Orbit](https://help.autodesk.com/cloudhelp/2014/ENU/Inventor/files/GUID-5E6B1A06-1043-45C4-A3C3-F0959939EB3B.htm)

### NAV-04 — Zoom no cursor e zoom por área

- **O que faz:** Aproxima em direção ao ponteiro; janela de zoom.
- **Como funciona:** Roda/arrasto → ponto sob o cursor.
- **Fenkra:** 🟡 parcial. Zoom só muda a distância ao alvo.
- **Fontes:** [SOLIDWORKS Help — Middle Mouse Button Functions](https://help.solidworks.com/2021/English/SolidWorks/sldworks/r_Middle_Mouse_Button.htm) · [PTC Creo View — modo de navegação compatível com CATIA V5](https://support.ptc.com/help/creo/view/r6.0/en/creo_view/Using_the_Mouse_in_Catia_V5_Mode.html)

### NAV-05 — Estilos de navegação

- **O que faz:** Mapeamento de botões imitando outros CADs.
- **Como funciona:** Preferência → estilo.
- **Fenkra:** ✅ completo. NavStyle (settings.rs).
- **Fontes:** [Contexto: câmera](../contextos/camera.md)

### NAV-06 — Cubo de vistas e vistas nomeadas

- **O que faz:** Orientações padrão e salvas.
- **Como funciona:** Clique no cubo → vista; salvar vista.
- **Fenkra:** ✅ completo. Cubo e estados de exibição.
- **Fontes:** [Inventor Help — More about Orbit](https://help.autodesk.com/cloudhelp/2014/ENU/Inventor/files/GUID-5E6B1A06-1043-45C4-A3C3-F0959939EB3B.htm)


## Biblioteca de normas

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| BIB-01 | **Tabelas de furos e roscas** | Hole standards | Hole Wizard data (Toolbox) | Thread.xls / Hole table | 🟡 hardware.rs | BM-BIB-01 |
| BIB-02 | **Fixadores padrão** | Catalog (standard parts) | Toolbox | Content Center | ❌ — | CTX-FXL |
| BIB-03 | **Gerador de engrenagens** | — (Gear add-ons) | Toolbox Gears | Spur/Helical/Bevel/Worm Gear Generator | ❌ — | CTX-ENG |
| BIB-04 | **Gerador de eixos e chavetas** | — | — | Shaft Generator / Key Connection | ❌ — | CTX-ENG |
| BIB-05 | **Materiais e bitolas** | Material Library | Materials / Gauge Tables | Material / Sheet Metal Styles | ✅ Materiais, bitolas |  |
| BIB-06 | **Fixadores aeronáuticos e militares** | Catalog (aero) | Toolbox (normas ANSI; aero por add-in) | Content Center (ANSI/aero limitado) | ❌ — | CTX-HW |
| BIB-07 | **Rolamentos** | Catalog | Toolbox Bearings | Content Center Bearings / Bearing Generator | ❌ — | CTX-HW |
| BIB-08 | **Cilindros pneumáticos** | — (catálogos de fabricante) | — (catálogos de fabricante) | — (catálogos de fabricante) | ❌ — | CTX-HW |
| BIB-09 | **Guias lineares e fusos** | — (catálogos de fabricante) | — (catálogos de fabricante) | — (catálogos de fabricante) | ❌ — | CTX-HW |

### BIB-01 — Tabelas de furos e roscas

- **O que faz:** Dimensões de furação, folga, rebaixo, escareado, rosca por norma.
- **Como funciona:** Norma → tamanho → dados.
- **Fenkra:** 🟡 parcial. METRIC M3–M20 "preliminar" e 7 normas de rosca.
- **Fontes:** [SOLIDWORKS Help — Hole Wizard (Toolbox)](https://help.solidworks.com/2022/English/SolidWorks/toolbox/HIDD_TOOLBOX_SELECT_HARDWARE.htm) · [Inventor Help — To Create Threads](https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-C0848D58-12CB-4605-8165-0836232927F0.htm)

### BIB-02 — Fixadores padrão

- **O que faz:** Parafusos, porcas, arruelas, pinos por norma (ISO/DIN/ASTM).
- **Como funciona:** Norma → tipo → tamanho/comprimento → peça.
- **Fenkra:** ❌ não existe. Lista de normas em contextos/fixadores.md.
- **Fontes:** [SOLIDWORKS Help — Smart Fasteners Overview](https://help.solidworks.com/2017/English/SolidWorks/sldworks/c_Smart_Fasteners_Overview.htm) · [Inventor Help — Design Accelerator](https://help.autodesk.com/cloudhelp/2015/ENU/Inventor-Help/files/GUID-24104648-BE41-49C7-9DD0-1AF2BAFC102E.htm) · [Contexto: lista de fixadores](../contextos/fixadores.md)

### BIB-03 — Gerador de engrenagens

- **O que faz:** Engrenagens reta, helicoidal, cônica, coroa/sem-fim, cremalheira com cálculo.
- **Como funciona:** Tipo → módulo, dentes, ângulo de pressão, largura → cálculo → peças + restrições.
- **Diferenças:** Inventor calcula e insere com restrições de movimento.
- **Fenkra:** ❌ não existe. Só acoplamento de engrenagem na montagem.
- **Fontes:** [Inventor Help — Design Accelerator](https://help.autodesk.com/cloudhelp/2015/ENU/Inventor-Help/files/GUID-24104648-BE41-49C7-9DD0-1AF2BAFC102E.htm) · [Contexto: engrenagens](../contextos/engrenagens.md)

### BIB-04 — Gerador de eixos e chavetas

- **O que faz:** Eixo com seções, chavetas (DIN 6885), ranhuras.
- **Como funciona:** Seções → elementos → cálculo.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Design Accelerator](https://help.autodesk.com/cloudhelp/2015/ENU/Inventor-Help/files/GUID-24104648-BE41-49C7-9DD0-1AF2BAFC102E.htm) · [Contexto: engrenagens](../contextos/engrenagens.md)

### BIB-05 — Materiais e bitolas

- **O que faz:** Densidade, propriedades e bitolas de chapa.
- **Como funciona:** Material → propriedades.
- **Fenkra:** ✅ completo. MATERIALS, SHEET_MATERIALS, SHEET_GAUGES.
- **Fontes:** [SOLIDWORKS Help — Sheet Metal Toolbar](https://help.solidworks.com/2022/English/SolidWorks/sldworks/r_Sheet_Metal_Toolbar.htm)

### BIB-06 — Fixadores aeronáuticos e militares

- **O que faz:** Parafusos, porcas, arruelas, pinos e rebites AN/MS/NAS/MIL.
- **Como funciona:** Norma → tipo → código de tamanho/comprimento → peça.
- **Fenkra:** ❌ não existe. Fontes AN/MS/MIL de domínio público no ASSIST (ver biblioteca/fontes-de-dados.md).
- **Fontes:** [Contexto: fontes de perfis e hardware](../contextos/hardwares.md)

### BIB-07 — Rolamentos

- **O que faz:** Rolamentos de esferas e rolos pelas medidas de contorno ISO 15.
- **Como funciona:** Série → diâmetro interno → d, D, B, r → peça simplificada.
- **Diferenças:** Inventor calcula vida útil no Bearing Generator.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Inventor Help — Design Accelerator](https://help.autodesk.com/cloudhelp/2015/ENU/Inventor-Help/files/GUID-24104648-BE41-49C7-9DD0-1AF2BAFC102E.htm) · [Contexto: fontes de perfis e hardware](../contextos/hardwares.md)

### BIB-08 — Cilindros pneumáticos

- **O que faz:** Cilindros ISO 15552/6432 com curso e fixações.
- **Como funciona:** Diâmetro → curso → fixações → peça/montagem.
- **Diferenças:** Normalmente vem de modelos baixados do fabricante.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Contexto: fontes de perfis e hardware](../contextos/hardwares.md)

### BIB-09 — Guias lineares e fusos

- **O que faz:** Trilhos, patins e fusos de esferas por série de fabricante.
- **Como funciona:** Série → tamanho → comprimento do trilho.
- **Fenkra:** ❌ não existe.
- **Fontes:** [Contexto: fontes de perfis e hardware](../contextos/hardwares.md)


## PDM / PLM

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| PLM-01 | **Versões e revisões** | ENOVIA / VPM | SOLIDWORKS PDM | Vault | ✅ Projeto em pasta |  |
| PLM-02 | **Estados de ciclo de vida** | Maturity | Workflow states | Lifecycle states | 🟡 Estados | CTX-PLM |
| PLM-03 | **Banco de produtos e estrutura (BOM)** | VPM | PDM vault (SQL) | Vault (SQL) | 🟡 Índice JSON | CTX-PLM |
| PLM-04 | **Check-in/out e multiusuário** | VPM sessions | Check Out / Check In | Check Out / Check In | ❌ — | CTX-PLM |
| PLM-05 | **Onde é usado e comparação** | Where-used / Compare | Where Used / Compare | Uses / Where Used | ❌ — | CTX-PLM |

### PLM-01 — Versões e revisões

- **O que faz:** Versões automáticas e revisões congeladas (A, B, C).
- **Como funciona:** Salvar → versão; liberar → revisão.
- **Fenkra:** ✅ completo. fenkra-project: Version, Revision, next_label.
- **Fontes:** [G2 — Autodesk Vault vs SOLIDWORKS PDM](https://www.g2.com/compare/autodesk-vault-vs-solidworks-pdm)

### PLM-02 — Estados de ciclo de vida

- **O que faz:** Em trabalho, liberado, obsoleto, com fluxo.
- **Como funciona:** Transição → aprovação.
- **Fenkra:** 🟡 parcial. DocState sem fluxo de aprovação.
- **Fontes:** [G2 — Autodesk Vault vs SOLIDWORKS PDM](https://www.g2.com/compare/autodesk-vault-vs-solidworks-pdm)

### PLM-03 — Banco de produtos e estrutura (BOM)

- **O que faz:** Produtos, revisões, documentos e estrutura num banco.
- **Como funciona:** Banco local (SQLite) → servidor.
- **Fenkra:** 🟡 parcial. Índice JSON com hash blake3; sem banco nem estrutura de produto.
- **Fontes:** [Contexto: PLM/VPM](../contextos/plm-vpm.md)

### PLM-04 — Check-in/out e multiusuário

- **O que faz:** Bloqueio para edição e trabalho em equipe.
- **Como funciona:** Check-out → editar → check-in.
- **Fenkra:** ❌ não existe.
- **Fontes:** [G2 — Autodesk Vault vs SOLIDWORKS PDM](https://www.g2.com/compare/autodesk-vault-vs-solidworks-pdm) · [Contexto: PLM/VPM](../contextos/plm-vpm.md)

### PLM-05 — Onde é usado e comparação

- **O que faz:** Onde a peça é usada e o que mudou entre revisões.
- **Como funciona:** Consulta → árvore de uso / diferenças.
- **Fenkra:** ❌ não existe.
- **Fontes:** [G2 — Autodesk Vault vs SOLIDWORKS PDM](https://www.g2.com/compare/autodesk-vault-vs-solidworks-pdm) · [Contexto: PLM/VPM](../contextos/plm-vpm.md)


## Geral

| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |
|---|---|---|---|---|---|---|
| GER-01 | **Ocultar hierárquico** | Hide/Show hierárquico | Hide/Show (com pastas) | Visibility | 🟡 Olho por item | DR-30 |
| GER-02 | **Pré-visualização ao vivo** | Preview | Preview | Preview | 🟡 Recompute ao vivo | DR-02 |
| GER-03 | **Direção normal a plano/superfície** | Direction (normal to) | Direction (normal) | Direction | 🟡 Direção | DR-09 |
| GER-04 | **Pastas e organização da árvore** | Geometrical Set (Insert in new) | Add to New Folder | Create Folder | 🟡 Pastas de geometria | DR-46 |
| GER-05 | **Assistente de reparo** | Update diagnosis / Replace | What's Wrong / Repair | Design Doctor | 🟡 Assistente de reparo | DR-11 |
| GER-06 | **Renderização de arestas** | Shading with edges | Shaded with edges / tangent edges | Shaded with edges | 🟡 Arestas vivas | DR-08 |
| GER-07 | **API de automação** | VBA / CAA | VBA / C# API | iLogic / VBA / .NET | 🟡 Macros | BM-GER-01 |

### GER-01 — Ocultar hierárquico

- **O que faz:** Ocultar o pai esconde os filhos sem perder o estado de cada um.
- **Como funciona:** Estado próprio + estado herdado.
- **Fenkra:** 🟡 parcial. FolderShown sobrescreve o estado dos filhos; origem só pelo menu (DR-15, DR-30).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### GER-02 — Pré-visualização ao vivo

- **O que faz:** Mostra o resultado enquanto edita o comando.
- **Como funciona:** Edição → preview → OK gera.
- **Fenkra:** 🟡 parcial. Recompute ao vivo; rosca modelada sem preview (DR-02).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### GER-03 — Direção normal a plano/superfície

- **O que faz:** Qualquer comando com direção aceita normal a plano ou superfície.
- **Como funciona:** Campo direção → escolher plano/face.
- **Fenkra:** 🟡 parcial. Nem todos os comandos aceitam (DR-09).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### GER-04 — Pastas e organização da árvore

- **O que faz:** Mover selecionados para nova pasta escolhendo o pai.
- **Como funciona:** Seleção → nova pasta → pai.
- **Fenkra:** 🟡 parcial. Sem "selecionados → nova pasta" com escolha do pai (DR-46).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### GER-05 — Assistente de reparo

- **O que faz:** Mostra referências perdidas e permite reatribuir.
- **Como funciona:** Erro → candidatos → escolher.
- **Fenkra:** 🟡 parcial. Janela de reparo para comandos com erro; sem candidatos no 3D (ADR 0010).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### GER-06 — Renderização de arestas

- **O que faz:** Arestas vivas visíveis, tangências ocultas, sem riscos indevidos.
- **Como funciona:** Estilo de exibição → arestas.
- **Fenkra:** 🟡 parcial. edge_smooth oculta tangências; linhas estranhas no render e em filetes de superfície (DR-08, DR-10).
- **Fontes:** [Design review do usuário (DR-01…DR-58)](../contextos/design-review.md)

### GER-07 — API de automação

- **O que faz:** Scripts para criar e alterar modelos.
- **Como funciona:** Linguagem → objetos do modelo.
- **Fenkra:** 🟡 parcial. Macros próprias; sem API Python (RNF-011).
- **Fontes:** [Contexto: PLM/VPM](../contextos/plm-vpm.md)
