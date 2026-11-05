# Montagem

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/montagem.md](../benchmark/montagem.md).

## ✅ Pronto

- **Inserir componente e fixar** (Inserir, fixar) — AsmInsert, AsmFix, modos editável/visualização.
- **Restrições padrão** (Restrições) — MateCmd: Coincident, Contact, Distance, Angle, Parallel, Perpendicular, Auto, AlignOnce. Distância e Ângulo nascem com a medida atual; Inverter alinhamento e Inverter lado separados (B71).
- **Subconjunto da seleção** (Subconjunto) — extract_subassembly leva o que é de dentro, religa ao nó o que liga com o exterior e relata o resto; inserir_de_volta desfaz sem perda (B73).
- **Restrição rápida / inteligente** (Restrição rápida) — quick_options, after_view_quick.
- **Juntas cinemáticas** (Juntas) — AsmJoint, kin.rs; limites e drives.
- **Acoplamentos mecânicos** (Engrenagem, cremalheira, polia, came) — MateKind Gear/RackPinion; couplings; teste came_e_seguidor.
- **Graus de liberdade** (GDL na árvore) — dof_per_comp; selos na árvore.
- **Padrão de componentes** (Padrão) — AsmPattern; repeticao_acompanha_a_quantidade.
- **Interferência e colisão** (Interferência) — AsmInterf; colisão ao arrastar.
- **Vista explodida** (Explodida) — Composer; teste explosao_e_animacao.
- **Lista de materiais (BOM)** (BOM) — AsmBom; numeração de itens.

## 🟡 Parcial

- **Peça no contexto e links** (Peça no contexto, copiar geometria) — AsmNewPart, AsmLink; links com estado, política por link, transformação viva e grafo acíclico em `links.rs` (B70, ADR 0013); peças não 100% editáveis vendo as outras (DR-31). `DR-31`
- **Recursos de montagem** (Recorte pela peça, furo entre peças) — AsmFeat(0/1) corta só a forma exibida na montagem: não grava furo nas peças, não entra no hole_table nem nos Fixadores; posição só pelo clique, Ø padrão 6. `B57`
- **Seleção e edição na árvore** (Árvore da montagem) — Ctrl+clique; sem Alt; ocultar/apagar diferentes das peças (DR-32). `DR-32`
- **Fixadores automáticos** (Fixadores (preencher furos)) — preencher_furos: pilha parafuso/arruela/porca com restrições (B29) e trava de giro, 0 GDL por padrão (B62); pega medida por sondas laterais; sem editar/trocar a pilha depois. `B51`
- **Publicações e referências na montagem** (Publicação em peça (sem montagem)) — Peça publica ref/parâmetro/corpo; montagem não tem sketch, ponto, eixo, plano nem publicação. `B55`
- **Editar peça no contexto** (Editar peça (troca de bancada)) — edit_part troca para a bancada Sólidos; outras peças só como arestas; Voltar salva sempre; links sem estado. `B54`

## ❌ Ainda não existe

- **Restrições avançadas** (—) — Sem largura, simetria ou caminho. `BM-MON-01`
- **Origem da montagem** (—) — Sem origem na montagem (DR-31). `DR-31`
- **Conexão parafusada** (—) — Fixadores só preenchem furos já existentes. `B58`
