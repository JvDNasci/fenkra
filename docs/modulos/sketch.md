# Sketch

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/sketch.md](../benchmark/sketch.md).

## ✅ Pronto

- **Linha e perfil encadeado** (Linha) — Tool::Line; restrições automáticas ao desenhar.
- **Círculo e arco** (Círculo, círculo 3 pontos, arco 3 pontos, arco pelo centro) — Tool::Circle, Circle3, Arc3, ArcCenter.
- **Restrições geométricas** (Restrições) — Kind: Coincident, Horizontal, Vertical, Parallel, Perpendicular, Equal, Tangent, Concentric, Midpoint, PointOnCurve, Fixed, Symmetric…
- **Cotas (restrições dimensionais)** (Cota (smart)) — Tool::Dimension (+H, V, ângulo, raio, diâmetro); expressões com parâmetros e vírgula decimal.
- **Solver e estado do sketch** (Solver com estado) — Teste sub_restrito_e_conflito; arrastar_ponto_preso_mantem_restricoes. `BM-SKT-04`
- **Snaps e inferência** (todas as ferramentas de desenho) — Glifo por tipo de snap, também ao arrastar; linhas de inferência H, V, ⊥, ∥ e alinhamento; cota troca raio e diâmetro e ângulo e suplemento pela posição (`sketch_snap.rs`, `sketch_cota.rs`).
- **Painel do comando** (todas as ferramentas) — Estado do passo, modo, parâmetros e avançado; Retângulo, Círculo, Arco e Canto são comandos com modo, a tecla M percorre e os campos são os da viewport (`sketch_painel.rs`).
- **Arraste direto** (Selecionar) — Arco e spline pelo corpo, mantendo restrições; setas de grau de liberdade vindas do espaço nulo do Jacobiano; pega de valor em filete e offset (`sketch_arraste.rs`).
- **Barra de restrições** (Selecionar) — Abre ao selecionar e fica aberta; fantasma resolvido, vermelho se conflitar, com orçamento de tempo (`sketch_barra.rs`).
- **Perfil sombreado e o que mostrar** (painel, Mostrar) — Contorno fechado sombreado por `loops`; caixas de perfil, pontos, cotas, restrições, construção e projetadas; fatiar corpos no plano só na tela (`sketch_sombra.rs`).
- **Offset de curvas** (Offset) — Tool::Offset.
- **Espelhar e simetria** (Espelhar) — Tool::Mirror.
- **Filete e chanfro de sketch** (Filete, chanfro) — Tool::Fillet, Chamfer.
- **Projetar / converter arestas 3D** (Converter) — Tool::Convert reconhece linha, arco, círculo e elipse exatos pela fórmula da aresta (`projecao.rs`, ADR 0018); política associativa, isolada ou instantânea e estado do vínculo no painel.

## 🟡 Parcial

- **Retângulo e variantes** (Retângulo, retângulo pelo centro) — Tool::Rect, RectCenter; sem 3 pontos (inclinado) nem paralelogramo. `BM-SKT-01`
- **Spline** (Spline (alças Bézier)) — Tool::Spline com alças e inflexões (alpha.25). Não acha tangência com outros elementos do sketch (DR-02); sem modo por polígono de controle no sketch. `DR-02`
- **Polígono e rasgo** (Polígono, rasgo) — Tool::Polygon e Slot (reto). Sem rasgo em arco no sketch (existe SmArcSlot só na chapa). `BM-SKT-03`
- **Aparar (trim)** (Aparar) — Tool::Trim em linha, arco, círculo e spline (`sketch_relimita.rs`), com trecho vermelho tracejado e cortes marcados antes do clique; Shift alterna com Estender. `DR-50`
- **Estender / aparar-estender** (Estender) — Tool::Extend em linha e arco, com prévia até o alvo; Shift alterna com Aparar. Spline não estende. `DR-51`
- **Mover, copiar, girar, escalar** (Mover, copiar) — Tool::Move e Copy; sem girar/escalar/esticar dedicados. `BM-SKT-05`
- **Interseção com o 3D** (Interseção) — Tool::Intersect acha todos os pontos, exatos pela fórmula ou pelo kernel; a quantidade de pontos é fixada ao criar. `DR-06`
- **Seleção por janela e cruzada** (Selecionar) — Arrastar no vazio: para a direita pega o que está dentro, para a esquerda também o que toca. Duplo clique pega a cadeia conectada, usada por Offset, Filete e Chanfro (`sketch_cadeia.rs`). `DR-05`
- **Geometria externa no sketch** (Converter) — Converter cria vínculo com política por sketch e por vínculo; o sketch tem origem e orientação próprias. Sem filtro de só publicados. `DR-18`
- **Sketch 3D** (Wireframe 3D) — Sem sketch 3D com restrições; há polilinha/spline/hélice 3D no wireframe (RefKind). `BM-SKT-08`

## ❌ Ainda não existe

- **Elipse e cônicas** (—) — Elipse entra só como geometria externa (Curve::Ellipse), sem ferramenta de desenho. Cônica existe no wireframe 3D (RefKind::Conic). `BM-SKT-02`
- **Quebrar (break/split)** (—) —  `DR-51`
- **Fechar arco e complemento** (—) —  `DR-52`
- **Padrão no sketch** (—) —  `BM-SKT-06`
- **Texto no sketch** (—) —  `BM-SKT-07`
