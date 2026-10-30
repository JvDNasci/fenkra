# Roadmap — Fenkra

> As fases são ordem de prioridade, não calendário. Estratégia: **fundação sólida primeiro**, depois
> **uma versão funcional de cada módulo** com menu minimalista e features completas em opções.

## Onde estamos (29/10/2026, alpha.26)

| Fase | Estado |
|---|---|
| 0 — Especificação | feita (requisitos, ADRs); fichas em `specs/` ficaram para trás |
| 1 — Fundação + Sólidos | quase toda pronta; falta naming topológico robusto (ADR 0010) e API Python |
| 2 — MVP dos módulos | todos os módulos têm versão funcional; lacunas no [checklist](../requisitos/checklist.md) |
| 3 — Aprofundar | em andamento: bainha, estampos, acoplamentos, explodida, GD&T já feitos |
| 4–5 | não iniciadas |

O próximo trabalho sai do [backlog](../backlog/backlog.md) (design review + [benchmark](../benchmark/)).

## Fase 0 — Especificação (feita)
- Visão, requisitos, ADRs
- Fichas das features da fase 1
- Esquema da biblioteca pública e primeiros dados (roscas, materiais)

## Fase 1 — Fundação + Modelagem Sólida
A fundação é usada por todos os módulos e não pode ser refeita depois.
- Núcleo: documento, **árvore (Origem, Referências, Parâmetros, pastas, coletores de corpos) e linha de tempo de comandos**, recompute incremental em background, desfazer
- Naming topológico persistente ([ADR 0004](../arquitetura/adr/0004-naming-topologico.md))
- **Parâmetros, expressões e propriedades** (PN, descrição, material, tipo, personalizadas)
- **Geometria mestre, publicações e links** com gerenciador ([ADR 0006](../arquitetura/adr/0006-links-e-projeto-relacional.md))
- Projeto em pasta com versões automáticas ([ADR 0009](../arquitetura/adr/0009-projeto-e-revisoes.md))
- Biblioteca: roscas, materiais ([ADR 0008](../arquitetura/adr/0008-biblioteca-publica.md))
- Interface minimalista, viewport própria
- Sketch 2D com solver
- **Modelagem Sólida**: extrusão, revolução, recorte, draft, nervura por caminho, furo, furo inteligente, rosca paramétrica, filete, chanfro
- STEP / IGES / STL / 3MF

## Fase 2 — Uma versão de cada módulo (MVP)
- **Desenho Técnico**: vistas como blocos ligados/travados, cortes, seções, cotas, geometria 2D, BOM, PDF/DXF
- **Montagem e Mecanismos**: restrições, juntas, fixo/GDL, drives, rosca entre peças, modos visualizar/contexto/editar
- **Superfícies**: curvas 3D, loft, fill, aparar/costurar, zebra e desvio
- **Chapas**: flange base, flange de aresta, dobra, alívios, planificação
- **Estruturas Soldadas**: membros, biblioteca de perfis com peso, uniões, lista de corte
- **Gestão de Projeto**: estrutura de produto, revisões congeladas, gestão de links, pacote
- Biblioteca: perfis estruturais, fixadores, chapas
- Sketch 3D, padrões, casca, booleanas, API Python

**Fim do MVP:** os critérios de sucesso da [visão](visao.md#10-critérios-de-sucesso-do-mvp).

## Fase 3 — Aprofundar os módulos
- Chapas: flange contínua sobre superfícies, bainha, estampos, converter sólido em chapa
- Soldas: gussets, tampas, cordões e simbologia
- Montagem: acoplamentos (engrenagem, cremalheira), colisão dinâmica, explodida, fixadores automáticos
- Desenho: GD&T, blocos 2D, restrições 2D
- Superfícies: blend, mapas de curvatura
- Gestão: comparação visual de revisões, bloqueio em pasta compartilhada

## Fase 4 — Ecossistema
- Biblioteca: elementos de máquina, edição comunitária pela interface
- Malha → sólido nível 3, otimização de barras, DWG (avaliar)

## Fase 5 — Avançado
- Superfícies por pontos de controle (estilo subdivisão) integradas ao B-rep
- Outros módulos sob demanda (CAM, simulação, BIM — hoje são não-objetivos)
