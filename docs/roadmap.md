# Roadmap — SolidCadCraft

> As fases são ordem de prioridade, não calendário. Estratégia: **fundação sólida primeiro**, depois
> **uma versão funcional de cada módulo** com menu minimalista e features completas em opções.

## Fase 0 — Especificação (agora)
- Visão, requisitos, ADRs
- Fichas das features da fase 1
- Esquema da biblioteca pública e primeiros dados (roscas, materiais)

## Fase 1 — Fundação + Modelagem Sólida
A fundação é usada por todos os módulos e não pode ser refeita depois.
- Núcleo: documento, **árvore (Origem, Referências, Parâmetros, pastas, coletores de corpos) e linha de tempo de comandos**, recompute incremental em background, desfazer
- Naming topológico persistente ([ADR 0004](adr/0004-naming-topologico.md))
- **Parâmetros, expressões e propriedades** (PN, descrição, material, tipo, personalizadas)
- **Geometria mestre, publicações e links** com gerenciador ([ADR 0006](adr/0006-links-e-projeto-relacional.md))
- Projeto em pasta com versões automáticas ([ADR 0009](adr/0009-projeto-e-revisoes.md))
- Biblioteca: roscas, materiais ([ADR 0008](adr/0008-biblioteca-publica.md))
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

**Fim do MVP:** os critérios de sucesso da [visão](00-visao.md#10-critérios-de-sucesso-do-mvp).

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
