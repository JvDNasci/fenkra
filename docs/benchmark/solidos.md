# Benchmark — Sólidos

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#sólidos).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Separação clara: features por sketch (Pad, Pocket, Shaft, Groove, Rib, Slot, Multi-sections) e de acabamento (fillet, chamfer, draft, shell, thickness). Cada uma pede suporte → tipo de limite (Dimension, Up to next/last/plane/surface) → valores → "More" com opções avançadas. Corpos (Bodies) são combinados por operações booleanas explícitas.
- **SOLIDWORKS:** PropertyManager único por feature com grupos recolhíveis: From (início) → Direction 1/2 (fim) → Thin feature → Selected contours. Adicionar e remover são variantes do mesmo comando. Hole Wizard guiado por norma.
- **Inventor:** Painel de propriedades com entradas, comportamento (distância, até, entre) e saída (unir, cortar, interseção, novo corpo). Thread com comprimento total ou parcial com offset. Coil e Emboss dedicados.

## O que adotar no Fenkra

- O modelo de limites do CATIA (até próximo/último/plano/superfície) disponível em todos os comandos com extensão.
- Grupo "Início" do SW (offset/de superfície) e "parede fina" como opção de qualquer extrusão/revolução.
- Thread do Inventor: comprimento parcial com offset e cosmético/modelado na mesma feature.
- Espessura diferente por face na casca (os três).

## O que evitar

- Referências por índice/ponto que quebram ao mudar o perfil (DR-11): ver ADR 0010.

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| SOL-01 | Extrusão (adicionar) | ✅ | FeatureKind::Extrude com Extent (incl. UpToSurface), taper, thin, dir; ExtrudeDraftFillet. |  |
| SOL-02 | Recorte por extrusão | ✅ | Cut, CutBlind, CutUpTo. |  |
| SOL-03 | Revolução | ✅ | Revolve, Groove; teste revolucao_com_comando_de_angulo. |  |
| SOL-04 | Varredura | ✅ | FeatureKind::Sweep. |  |
| SOL-05 | Loft (multi-seções) | 🟡 | Loft tem sections, ruled, closed, op; faltam restrição inicial/final, tensão, guias e coletor interativo (DR-07). | DR-07 |
| SOL-06 | Furo | 🟡 | HoleKind: Simples, Rebaixado, Escareado, Contrafurado; escareado nasce com 90° fixo; casca falha com furo escareado (DR-02). | DR-02 |
| SOL-07 | Rosca | 🟡 | Thread com modeled e end; sem passo/perfil personalizados, deslocamento inicial nem chanfro; tap com bugs; sem preview (DR-02, DR-03). | DR-03 |
| SOL-08 | Filete de aresta | ✅ | Fillet, FilletAll, FilletFull, FilletFaceFace. |  |
| SOL-09 | Filete variável | 🟡 | FilletVar existe; sem pontos de controle intermediários editáveis. |  |
| SOL-10 | Chanfro | ✅ | Chamfer, ChamferAll, ChamferDistAngle, ChamferTwoDist. |  |
| SOL-11 | Casca | 🟡 | Shell só com espessura única; falha com spline e furo escareado; para fora gera raio sem opção reta (DR-02, DR-07). | DR-07 |
| SOL-12 | Ângulo de saída (draft) | ✅ | FeatureKind::Draft. |  |
| SOL-13 | Nervura de reforço | 🟡 | Stiffener: thickness, depth, reverse, to_body; sem lado/ambos os lados (DR-05). | DR-05 |
| SOL-14 | Engrossar / espessura | 🟡 | Thickness e ThickSurface; falhou em face de sólido (DR-12). | DR-12 |
| SOL-15 | Padrões e espelhar | ✅ | Pattern, Mirror; PatternPoints. |  |
| SOL-16 | Booleanas entre corpos | ✅ | Combine(Add/Cut/Intersect). |  |
| SOL-17 | Dividir corpo | ✅ | SplitBody. |  |
| SOL-18 | Primitivas | ✅ | Prim(Caixa, Cilindro, Cone, Cunha, Esfera, Prisma, Toro). |  |
| SOL-19 | Mover / copiar corpo | ✅ | MoveBody (Translate, Rotate, Symmetry, Affinity). |  |
| SOL-20 | Escala | ✅ | FeatureKind::Scale. |  |
| SOL-21 | Relevo e gravação | ❌ |  | BM-SOL-01 |
| SOL-22 | Hélice / mola (coil) | 🟡 | Hélice no wireframe (RefKind::Helix) + Sweep; sem comando direto de mola. | BM-SOL-02 |
| SOL-23 | Modelagem direta | ❌ |  | BM-SOL-03 |
| SOL-24 | Referências persistentes | 🟡 | Pontos casados por proximidade; filete se perde ao mudar o perfil (DR-11). Ver ADR 0010. | DR-11 |
