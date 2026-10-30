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

## Onda B (pendente: depois do reset da cota semanal)
Ainda faltam estes relatórios:
- superfícies;
- chapa;
- estrutural e soldas;
- desenho;
- PMI;
- visão macro e requisitos consolidados (`requisitos-macro.md`);
- funções inteligentes de outros CADs (`funcoes-inteligentes.md`).

Depois da onda B vem a **síntese**, que entrega três coisas:
- o relatório executivo;
- a matriz requisito → backlog;
- a revisão do backlog: novos itens, prioridades e onda 0.
