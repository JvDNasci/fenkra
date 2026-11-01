# Pesquisa avançada: comando a comando (CATIA V5 + SolidWorks + Inventor + outros)

**Objetivo:** adotar a filosofia do CATIA V5 com as funções inteligentes do SolidWorks, do Inventor e de outros CADs, dentro das bancadas que o Fenkra já tem. A base é o [ADR 0011](../arquitetura/adr/0011-filosofia-de-modelagem.md).

**Estrutura de cada relatório:**
1. resumo e lacunas;
2. uma ficha por comando: CATIA, SW, Inventor e outros no detalhe, **Fenkra hoje** (com arquivo:linha) e **melhor prática**;
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
| [interface-e-arvore.md](interface-e-arvore.md) | Árvore, Define In Work Object, atualização, seleção, painéis, tooltips, gráficos | 36 |
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
