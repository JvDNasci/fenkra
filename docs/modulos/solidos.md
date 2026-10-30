# Sólidos

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/solidos.md](../benchmark/solidos.md).

## ✅ Pronto

- **Extrusão (adicionar)** (Extrusão) — FeatureKind::Extrude com Extent (incl. UpToSurface), taper, thin, dir; ExtrudeDraftFillet.
- **Recorte por extrusão** (Recorte) — Cut, CutBlind, CutUpTo.
- **Revolução** (Revolução / Ranhura) — Revolve, Groove; teste revolucao_com_comando_de_angulo.
- **Varredura** (Varredura) — FeatureKind::Sweep.
- **Filete de aresta** (Filete) — Fillet, FilletAll, FilletFull, FilletFaceFace.
- **Chanfro** (Chanfro) — Chamfer, ChamferAll, ChamferDistAngle, ChamferTwoDist.
- **Ângulo de saída (draft)** (Draft) — FeatureKind::Draft.
- **Padrões e espelhar** (Padrão linear/circular/por pontos, espelhar) — Pattern, Mirror; PatternPoints.
- **Booleanas entre corpos** (Combinar) — Combine(Add/Cut/Intersect).
- **Dividir corpo** (Dividir corpo) — SplitBody.
- **Primitivas** (Primitivas) — Prim(Caixa, Cilindro, Cone, Cunha, Esfera, Prisma, Toro).
- **Mover / copiar corpo** (Mover corpo) — MoveBody (Translate, Rotate, Symmetry, Affinity).
- **Escala** (Escala) — FeatureKind::Scale.

## 🟡 Parcial

- **Loft (multi-seções)** (Loft) — Loft tem sections, ruled, closed, op; faltam restrição inicial/final, tensão, guias e coletor interativo (DR-07). `DR-07`
- **Furo** (Furo) — HoleKind: Simples, Rebaixado, Escareado, Contrafurado; escareado nasce com 90° fixo; casca falha com furo escareado (DR-02). `DR-02`
- **Rosca** (Rosca) — Thread com modeled e end; sem passo/perfil personalizados, deslocamento inicial nem chanfro; tap com bugs; sem preview (DR-02, DR-03). `DR-03`
- **Filete variável** (Filete variável) — FilletVar existe; sem pontos de controle intermediários editáveis.
- **Casca** (Casca) — Shell só com espessura única; falha com spline e furo escareado; para fora gera raio sem opção reta (DR-02, DR-07). `DR-07`
- **Nervura de reforço** (Nervura) — Stiffener: thickness, depth, reverse, to_body; sem lado/ambos os lados (DR-05). `DR-05`
- **Engrossar / espessura** (Espessura / Engrossar superfície) — Thickness e ThickSurface; falhou em face de sólido (DR-12). `DR-12`
- **Hélice / mola (coil)** (Hélice (wireframe) + varredura) — Hélice no wireframe (RefKind::Helix) + Sweep; sem comando direto de mola. `BM-SOL-02`
- **Referências persistentes** (Ponto clicado + face_owner) — Pontos casados por proximidade; filete se perde ao mudar o perfil (DR-11). Ver ADR 0010. `DR-11`

## ❌ Ainda não existe

- **Relevo e gravação** (—) —  `BM-SOL-01`
- **Modelagem direta** (—) —  `BM-SOL-03`
