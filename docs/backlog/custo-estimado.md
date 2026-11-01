# Custo estimado do backlog em cota do Claude (31/10/2026)

## Calibração (dados reais desta execução)
| Trabalho | Tokens estimados | Custo dos workers | Semanal | Janela de 5 h |
|---|---|---|---|---|
| B28 (engrenagens, eixos e chavetas; 2 builders + o lead) | 400 k | US$ 4,04 | +3 pts | +21 pts |
| B29 (fixadores; 2 scouts + 2 builders + o lead) | 450 k | US$ 4,31 | ≈ +6 pts | — |
| B26 (estrutura → montagem; 4 ondas) | 500 k | US$ 5,26 | ≈ +3 pts | — |
| Pesquisa onda A (8 workers Sonnet, alto esforço) | — | US$ 18,19 | ≈ +9 pts | ≈ +55 pts |
| Pesquisa onda B (7 workers) + síntese (Opus) | — | US$ 17,34 + 4,22 | ≈ +8 pts (bateu 100%) | — |

**Taxa usada:** pontos da cota semanal por 100 k tokens estimados de item, já incluindo o trabalho do lead (revisão, ícones, testes e commits):
- **otimista:** 0,6;
- **provável:** 0,9;
- **pessimista:** 1,3.

**Conversões:**
- 1 janela de 5 h cheia ≈ 23 pts da semanal.
- Um builder grande usa cerca de 13–21 pts da janela de 5 h, ou seja, **cabem uns 5 itens por janela de 5 h**.

## Projeção (105 itens abertos; itens em andamento contam pela metade)
| Onda | Itens | Tokens | Semanal (otimista / provável / pessimista) | Semanas de cota cheia |
|---|---|---|---|---|
| 0 — fundação | 21 | 5,6 M | 33 / **50** / 72 pts | 0,3 / **0,5** / 0,7 |
| 1 — bugs do review | 30 | 5,5 M | 33 / **49** / 71 pts | 0,3 / **0,5** / 0,7 |
| 2 — filosofia base + VPM | 27 | 6,3 M | 38 / **56** / 82 pts | 0,4 / **0,6** / 0,8 |
| 3 — módulos e lacunas | 27 | 6,9 M | 41 / **62** / 90 pts | 0,4 / **0,6** / 0,9 |
| **Total** | **105** | **24,2 M** | 145 / **218** / 314 pts | **1,5 / 2,2 / 3,1** |

**Leitura:**
- O backlog completo cabe em cerca de **2 a 3 semanas de cota usada até o fim**.
- Também pesa a janela de 5 h: são cerca de 5 builders por janela. A onda 0, com 21 itens, precisa de **umas 4 a 5 janelas de 5 h**, ou seja, cerca de 2 a 3 dias corridos de uso intenso.

## Ordem sugerida para a 1ª semana
A onda 0 consome cerca de 50% da semanal. Ela entra nesta ordem, porque cada etapa protege as seguintes:
1. **B53, B72 e B67:** corpus de testes e CI.
2. **B64, B65 e B66:** formato versionado, gravação segura e identidade estável.
3. **B59, B68 e B69:** objeto em trabalho, desfazer e diagnóstico.
4. **B15, B70 e B71:** referências e links com estado.
5. **B54, B55 e B56:** contexto da montagem, publicações e Salvar tudo.
6. **B57, B58 e B62:** furo entre peças e prendedores por pontos coloridos.
7. **B61, B60 e B42:** gráficos, ícones e biblioteca.

O resto da semanal (cerca de 40%) cobre o início da onda 1, que reúne os bugs do design review.

## Margens e riscos
- As estimativas de tokens por item vêm da síntese e do backlog e ainda não foram validadas. Itens de interface (B54, B60) costumam passar do previsto.
- Para cada worker que estoura o prazo ou falha, conte cerca de +30% no item.
- A pesquisa extra que surgir durante a execução não está incluída.
