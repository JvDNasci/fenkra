# Montagem

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/montagem.md](../benchmark/montagem.md).

## ✅ Pronto

- **Inserir componente e fixar** (Inserir, fixar) — AsmInsert, AsmFix, modos editável/visualização.
- **Restrições padrão** (Restrições) — MateCmd: Coincident, Contact, Distance, Angle, Parallel, Perpendicular, Auto, AlignOnce.
- **Restrição rápida / inteligente** (Restrição rápida) — quick_options, after_view_quick.
- **Juntas cinemáticas** (Juntas) — AsmJoint, kin.rs; limites e drives.
- **Acoplamentos mecânicos** (Engrenagem, cremalheira, polia, came) — MateKind Gear/RackPinion; couplings; teste came_e_seguidor.
- **Graus de liberdade** (GDL na árvore) — dof_per_comp; selos na árvore.
- **Padrão de componentes** (Padrão) — AsmPattern; repeticao_acompanha_a_quantidade.
- **Interferência e colisão** (Interferência) — AsmInterf; colisão ao arrastar.
- **Vista explodida** (Explodida) — Composer; teste explosao_e_animacao.
- **Recursos de montagem** (Recorte pela peça, furo entre peças) — AsmFeat(0/1).
- **Lista de materiais (BOM)** (BOM) — AsmBom; numeração de itens.

## 🟡 Parcial

- **Peça no contexto e links** (Peça no contexto, copiar geometria) — AsmNewPart, AsmLink; peças não 100% editáveis vendo as outras (DR-31). `DR-31`
- **Seleção e edição na árvore** (Árvore da montagem) — Ctrl+clique; sem Alt; ocultar/apagar diferentes das peças (DR-32). `DR-32`
- **Fixadores automáticos** (Fixadores (preencher furos)) — preencher_furos: pilha parafuso/arruela/porca com restrições (B29); pega medida por sondas laterais; sem editar/trocar a pilha depois. `B51`

## ❌ Ainda não existe

- **Restrições avançadas** (—) — Sem largura, simetria ou caminho. `BM-MON-01`
- **Origem da montagem** (—) — Sem origem na montagem (DR-31). `DR-31`
