# Montagem

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/montagem.md](../benchmark/montagem.md).

## ✅ Pronto

- **Inserir componente e fixar** (Inserir, fixar) — `asm_inserir.rs`, `insercao.rs`: painel com miniaturas, peça no cursor, uma cópia por clique, primeira fixa na origem (B262); AsmFix, modos editável/visualização.
- **Restrições padrão** (Restrições) — MateCmd: Coincident, Contact, Distance, Angle, Parallel, Perpendicular, Auto, AlignOnce. Distância e Ângulo nascem com a medida atual; Inverter alinhamento e Inverter lado separados (B71).
- **Pontos de encaixe** (Restrições e juntas) — `encaixe.rs`, `asm_encaixe.rs`: candidatos da face sob o cursor com sistema de eixos, tecla L trava, giro de 90°, Inverter e deslocamento com fórmula (B99).
- **Prévia e confirmar** (Restrições e juntas) — `proposta::Comando` resolve numa cópia; fantasma na posição final, Enter aceita, Tab inverte, Esc cancela (B244, ADR 0018).
- **Símbolos no modelo** (Restrições e juntas) — `simbolos.rs`, `asm_simbolos.rs`: ancorados na geometria, cota e arco arrastáveis, ocultar por tipo, grade para montagens grandes (B245).
- **Origem da montagem** (Árvore) — `ORIGEM`, planos, eixos e ponto usados como referência (B22, ADR 0018).
- **Manipulador** (Mover e girar) — `manipulador.rs`, `solve::liberdade`: referência, pivô, passo, colisão com escopo, alças só do que as restrições deixam, valor digitado (B261).
- **Subconjunto da seleção** (Subconjunto) — extract_subassembly leva o que é de dentro, religa ao nó o que liga com o exterior e relata o resto; inserir_de_volta desfaz sem perda (B73).
- **Restrição rápida / inteligente** (Restrição rápida) — quick_options, after_view_quick.
- **Juntas cinemáticas** (Juntas) — AsmJoint, kin.rs; comandos; tipo automático, modos de origem, alinhamento e Animar na criação (B260).
- **Limites de junta** (Juntas) — `kin.rs`, `asm_limites.rs`: Mínimo, Atual e Máximo por grau de liberdade, caixa por extremo que guarda o valor, marca na árvore, arco e trilho arrastáveis no 3D. O solucionador impõe a faixa: comando e arraste param no batente, sem contar como falha (B36).
- **Acoplamentos mecânicos** (Engrenagem, cremalheira, polia, came) — MateKind Gear/RackPinion; couplings; teste came_e_seguidor.
- **Graus de liberdade** (GDL na árvore) — dof_per_comp; selos na árvore.
- **Padrão de componentes** (Repetição) — `repeticao.rs`, `asm_repeticao.rs`: direção por aresta, eixo ou face, Pelos furos, vários componentes, segunda direção, fantasmas, seta e marcas para pular (B263).
- **Interferência e colisão** (Interferência) — `interferencia.rs`, `asm_interf.rs`: painel próprio, escopo, coincidência, subconjunto como peça única, realce do volume comum, ignorados salvos na montagem (B264); colisão ao arrastar.
- **Vista explodida** (Explodida) — Estados de exibição; teste explosao_e_animacao.
- **Lista de materiais (BOM)** (BOM) — AsmBom; numeração de itens.

## 🟡 Parcial

- **Peça no contexto e links** (Peça no contexto, copiar geometria) — AsmNewPart, AsmLink; links com estado, política por link, transformação viva e grafo acíclico em `links.rs` (B70, ADR 0013); peças não 100% editáveis vendo as outras (DR-31). `DR-31`
- **Recursos de montagem** (Recorte pela peça, furo entre peças) — AsmFeat(0/1) corta só a forma exibida na montagem: não grava furo nas peças, não entra no hole_table nem nos Fixadores; posição só pelo clique, Ø padrão 6. `B57`
- **Seleção e edição na árvore** (Árvore da montagem) — Ctrl, Shift e Alt; ocultar, apagar e isolar valem para a seleção (B22). `DR-32`
- **Fixadores automáticos** (Fixadores, Assistente de prendedores) — preencher_furos: pilha parafuso/arruela/porca com restrições (B29) e trava de giro, 0 GDL por padrão (B62); `pilhas.rs`, `asm_fixadores.rs`: pilha recalculável ligada ao furo, tabela, ponto manual e diagnóstico por ponto (B98, ADR 0034). `B51`
- **Publicações e referências na montagem** (Publicação em peça (sem montagem)) — Peça publica ref/parâmetro/corpo; montagem não tem sketch, ponto, eixo, plano nem publicação. `B55`
- **Editar peça no contexto** (Editar peça (troca de bancada)) — edit_part troca para a bancada Sólidos; outras peças só como arestas; Voltar salva sempre; links sem estado. `B54`

## ❌ Ainda não existe

- **Restrições avançadas** (—) — Sem largura, simetria ou caminho; o limite existe nas juntas, não em Distância e Ângulo. `BM-MON-01`
- **Origem da montagem** (—) — Sem origem na montagem (DR-31). `DR-31`
- **Conexão parafusada** (—) — Fixadores só preenchem furos já existentes. `B58`
