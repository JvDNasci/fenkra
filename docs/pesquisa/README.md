# Pesquisa avançada: comando a comando (CAD avançado B + CAD avançado A + CAD avançado C + outros)

**Objetivo:** adotar a filosofia do CAD avançado B com as funções inteligentes do CAD avançado A, do CAD avançado C e de outros CADs, dentro das bancadas que o Fenkra já tem. A base é o [ADR 0011](../arquitetura/adr/0011-filosofia-de-modelagem.md).

**Estrutura de cada relatório:**
1. resumo e lacunas;
2. uma ficha por comando: CAD avançado B, CAD avançado A, CAD avançado C e outros no detalhe, **Fenkra hoje** (com arquivo:linha) e **melhor prática**;
3. requisitos RF e RNF com critério de aceite;
4. itens de backlog propostos;
5. fontes.

**Marcação:** [V] indica que foi confirmado em fonte pública, e [C] que é conhecimento geral ainda a confirmar.

**Como foi feito:** a pesquisa foi orquestrada em 30/10/2026 com workers Sonnet (esforço alto). Eles fizeram busca e leitura na web e leram o código do Fenkra. Os relatórios foram gravados como cada worker os entregou.

## Onda A (feita)
| Relatório | Tema | Requisitos |
|---|---|---|
| [sketch.md](sketch.md) | Sketcher | 47 |
| [solidos.md](solidos.md) | Part Design / features de sólido | 48 |
| [interface-e-arvore.md](interface-e-arvore.md) | Árvore, objeto de trabalho, atualização, seleção, painéis, tooltips, gráficos | 36 |
| [acessibilidade.md](acessibilidade.md) | Modo daltônico, contraste, teclado, movimento e leitor de tela (B320) | 12 |
| [montagem.md](montagem.md) | Montagem, edição em contexto, publicações, features de montagem | 42 |
| [prendedores-aeronauticos.md](prendedores-aeronauticos.md) | NAS/MS/AN, Hi-Lok, Hi-Lite, lockbolts, cegos, rebites, porcas âncora, insertos; regra de pontos coloridos | 43 |
| [prendedores-industriais.md](prendedores-industriais.md) | ISO/DIN/ASME, pinos, anéis, chavetas, insertos, rebites, PEM | 25 |
| [conversor-de-arquivos.md](conversor-de-arquivos.md) | Importar/exportar (STEP AP242, IGES, JT, nativos, malhas), menu separado, ligação com VPM | 42 |
| [plm-vpm-e-save.md](plm-vpm-e-save.md) | Save management, Salvar tudo, links, VPM/PDM/Vault, trava em pasta | 48 |

## Onda B (feita)
| Relatório | Tema |
|---|---|
| [superficies.md](superficies.md) | Superfícies e wireframe |
| [chapa.md](chapa.md) | Chapa metálica |
| [estrutural.md](estrutural.md) | Estrutural e soldas |
| [desenho.md](desenho.md) | Desenho técnico |
| [pmi.md](pmi.md) | PMI / FT&A / MBD |
| [requisitos-macro.md](requisitos-macro.md) | Visão macro e requisitos consolidados |
| [funcoes-inteligentes.md](funcoes-inteligentes.md) | Funções inteligentes de outros CADs |

## Síntese
[sintese.md](sintese.md) traz:
- visão macro e 10 princípios;
- lacunas por bancada;
- 148 requisitos P0/P1 sem duplicatas, com rastreabilidade;
- roteiro em ondas.

Ela gerou 37 alterações e 45 itens novos (B64–B108) no backlog. O custo em cota está em [../backlog/custo-estimado.md](../backlog/custo-estimado.md).

## Pesquisa final (01/11/2026)
| Relatório | Tema |
|---|---|
| [modulos-novos.md](modulos-novos.md) | Módulos e recursos de CADs similares (FreeCAD, CAD avançado E, CAD avançado D, CAD avançado H, CAD avançado A, CAD avançado C, CAD avançado B, CAD avançado F, CAD avançado G…), com licenças das bibliotecas abertas e 41 itens propostos (onda 4: B110–B150) |
| [propriedade-intelectual.md](propriedade-intelectual.md) | Auditoria de propriedade intelectual: licenças, marcas, patentes, dados de normas, ícones; checklist e perguntas para o advogado (gerou o B151). **Não é parecer jurídico.** |

## Benchmarking 2: itens novos (01/11/2026)
Mercado e funcionalidades, item a item, com MVP, integrações e riscos:

| Relatório | Itens |
|---|---|
| [bench-forma-ergonomia-visualizacao.md](bench-forma-ergonomia-visualizacao.md) | forma livre (subdivisão e polos), ergonomia com manequim editável, Visualizador, Render, VR/AR |
| [bench-fabricacao.md](bench-fabricacao.md) | corte de encaixe e caminho de corte em tubos, manufatura aditiva, simulação de manufatura (integrada à ergonomia), STL → STEP, engenharia reversa |
| [bench-simulacao-sistemas.md](bench-simulacao-sistemas.md) | FEM e CFD no formato do FreeCAD, chicote, compósitos, plástico e moldes |
| [bench-pmi-cotas.md](bench-pmi-cotas.md) | catálogo completo de tipos de cota e anotação do PMI × CAD avançado B FT&A, CAD avançado A MBD, CAD avançado C, CAD avançado F, CAD avançado G |

Gerou 40 revisões e 31 itens novos (B170–B200). Os itens pedidos pelo usuário ficam com prioridade mínima P1.
