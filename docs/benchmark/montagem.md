# Benchmark — Montagem

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#montagem).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Product com componentes; restrições (coincidência, contato, offset, ângulo, fix, fix together) e Quick Constraint; mecanismos ficam na bancada de cinemática.
- **SOLIDWORKS:** Mates padrão, avançados (largura, simetria, caminho, limite) e mecânicos (engrenagem, cremalheira, came, parafuso); SmartMates ao arrastar; padrões guiados pela feature.
- **Inventor:** Constraints para posicionar e Joints para movimento, com tipo automático pela geometria; Design Accelerator insere conexões parafusadas e engrenagens com restrições.

## O que adotar no Fenkra

- Juntas com tipo automático pela geometria (Inventor) e restrição rápida (CATIA/SW).
- Mates avançados de largura/simetria/caminho (SW).
- Origem própria da montagem e seleção/ocultar iguais às peças (DR-31, DR-32).

## O que evitar

- Dois painéis concorrentes (árvore + editor): já unificado na alpha.25.

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| MON-01 | Inserir componente e fixar | ✅ | AsmInsert, AsmFix, modos editável/visualização. |  |
| MON-02 | Restrições padrão | ✅ | MateCmd: Coincident, Contact, Distance, Angle, Parallel, Perpendicular, Auto, AlignOnce. |  |
| MON-03 | Restrição rápida / inteligente | ✅ | quick_options, after_view_quick. |  |
| MON-04 | Juntas cinemáticas | ✅ | AsmJoint, kin.rs; limites e drives. |  |
| MON-05 | Acoplamentos mecânicos | ✅ | MateKind Gear/RackPinion; couplings; teste came_e_seguidor. |  |
| MON-06 | Restrições avançadas | ❌ | Sem largura, simetria ou caminho. | BM-MON-01 |
| MON-07 | Graus de liberdade | ✅ | dof_per_comp; selos na árvore. |  |
| MON-08 | Padrão de componentes | ✅ | AsmPattern; repeticao_acompanha_a_quantidade. |  |
| MON-09 | Interferência e colisão | ✅ | AsmInterf; colisão ao arrastar. |  |
| MON-10 | Vista explodida | ✅ | Composer; teste explosao_e_animacao. |  |
| MON-11 | Peça no contexto e links | 🟡 | AsmNewPart, AsmLink; peças não 100% editáveis vendo as outras (DR-31). | DR-31 |
| MON-12 | Recursos de montagem | ✅ | AsmFeat(0/1). |  |
| MON-13 | Origem da montagem | ❌ | Sem origem na montagem (DR-31). | DR-31 |
| MON-14 | Seleção e edição na árvore | 🟡 | Ctrl+clique; sem Alt; ocultar/apagar diferentes das peças (DR-32). | DR-32 |
| MON-15 | Lista de materiais (BOM) | ✅ | AsmBom; numeração de itens. |  |
| MON-16 | Fixadores automáticos | 🟡 | preencher_furos: pilha parafuso/arruela/porca com restrições (B29); pega medida por sondas laterais; sem editar/trocar a pilha depois. | B51 |
