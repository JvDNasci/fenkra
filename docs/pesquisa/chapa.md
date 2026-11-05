# Chapa metálica (Generative Sheetmetal Design / Sheet Metal)

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Pesquisa de filosofia para o Fenkra. Não alterei nenhum arquivo.
> Legenda: **[V]** = confirmado em fonte pública; **[C]** = conhecimento geral ou inferência minha, não confirmado.
>
> **Limites das fontes**
> - A apostila NIAR R30 devolveu 404. Usei o guia oficial fornecedor de CAD avançado *Generative Sheetmetal Design V5R14* (espelho em CAD avançado Bdesign.org). É antigo: nomes e janelas das versões novas do CAD avançado B podem diferir.
> - A ajuda online do CAD avançado A é renderizada por JavaScript e não abriu. Os fatos de CAD avançado A marcados [V] vêm de trechos de páginas da ajuda mostrados nos resultados de busca e de blogs de revendas (GoEngineer, Hawk Ridge, SolidProfessor, TriMech).
> - A Javelin devolveu 403.
> - O CAD avançado C foi lido direto na ajuda da fornecedor de CAD avançado.
> - CAD avançado E, CAD avançado D, CAD avançado F e CAD avançado H foram lidos nas páginas públicas.

---

## 1. Resumo e principais lacunas

1. A filosofia do CAD avançado B para chapa é: **parâmetros globais** (espessura, raio mínimo e padrão, fator K ou tabela, extremidades de dobra, normas) → **parede de referência** → **paredes por aresta ou varridas**, que são recursos *contextuais* ligados à aresta. As dobras são recursos próprios, entre duas paredes. Tudo isso fica numa árvore de especificação ordenada, com visão **dobrada e planificada concorrentes**. [V]
2. O CAD avançado B registra cada recurso na visão em que foi criado e pede para trocar de visão ao editar. Estampos não podem ser criados na planificada. Dobra, recorte e estampo têm regras próprias de ordem. [V]
3. CAD avançado A e CAD avançado C acrescentam:
   - posição da dobra e comprimento de aba medido por vértice virtual (CAD avançado A) ou por datum de altura (CAD avançado C);
   - regras e estilos de chapa reaproveitáveis (CAD avançado C);
   - tabelas de dobra e de bitola;
   - recorte normal e recorte através da dobra;
   - fechamento de canto com tipos e overlap;
   - ferramentas de forma em biblioteca (forming tool, punch tool);
   - conversão de sólido (Convert, Insert Bends, Rip);
   - ordem de dobra e tabela de dobras no desenho.
4. O Fenkra modela chapa como **sólido OpenCascade regenerado em dois estados**.
   - Um flag global `SheetParams.flat` escolhe dobrado ou planificado.
   - Cada recurso carrega `flat_space`, o estado em que foi criado.
   - Um registro `SheetMap` guarda cada dobra feita por `SmFlange`/`SmHem`, e é ele que leva pontos e ferramentas entre os estados (`lib.rs:439`, `2467`, `2490`).
5. O Fenkra tem cobertura funcional ampla (14 comandos `Sm*`, tabela de materiais, método K/dedução/tolerância, tabela de dobras, DXF planificado). **A base é frágil.** As referências são **pontos clicados ou polilinhas copiadas**, não arestas nomeadas. A planificação só conhece as dobras de aba e bainha. O recorte atravessando o raio não tem mapeamento por "enrolamento".
6. Isso explica os bugs do usuário: DR-19/23 (aba por superfície), DR-20 (recorte), DR-22 (dobra por linha), DR-54..57 (abas em sequência e vizinhas).
7. A melhor estratégia é fazer o **mapa de dobras (flat-map) ser a fonte única de verdade** de toda feature de chapa: aba, perfil, curva, superfície, joggle, dobra, bainha. Sobre ele ficam bem quatro coisas: planificação, DXF, tabela de dobras e recorte através da dobra.
8. As referências devem passar a ser **aresta + dono + papel** (ADR 0010/0011), com propagação por tangência e diagnóstico quando sumirem.
9. O comando de aba passa a ser **unificado**: suporte → modo → parâmetros → avançado.
10. Adotar o diferencial de cada CAD:
    - CAD avançado B: ordem, contexto, `objeto de trabalho` e agregação de corpos.
    - CAD avançado A: posição da dobra por vértice virtual e cantos.
    - CAD avançado C: regras/estilos, tabela de dobra por ângulo × raio com interpolação, e recorte através da dobra.
    - CAD avançado D e CAD avançado E: regra com relief/gap mínimo e tabela de chapa editável.

**Principais lacunas do Fenkra neste tema**

| # | Lacuna | Evidência |
|---|---|---|
| 1 | Referências por ponto ou polilinha (`SmFlange.edge`, `SmCurveFlange.edge`, `SmBend.a/b`, `SmJoggle.a/b`), sem aresta nomeada | `doc.rs:1753`, `1607`, `1509`, `1487` |
| 2 | Planificação incompleta: só `SmFlange` e `SmHem` registram `SheetMap`. Perfil, curva, superfície, dobra por linha e joggle ficam fora das linhas de dobra e do DXF | `lib.rs:2222`, `4380`, `3557`, `3258`, `3176` |
| 3 | Recorte dobrado ↔ planificado usa transformação rígida por região que exclui o arco da dobra; não há mapeamento "enrolado" | `lib.rs:479`, `2490` |
| 4 | Compensação de dobra só global e escalar: sem fator K local por feature, sem tabela ângulo × raio × espessura, sem raio mínimo verificado | `doc.rs:1694`, `1721` |
| 5 | Sem conversão de sólido, sem aba entre perfis (lofted), sem dobrar/desdobrar parcial (Fold/Unfold), sem normas de fabricação (verificações, ordem de dobra) | catálogo CHP-07/15/16 |

---

## 2. Fichas por comando

> Observações gerais de leitura do código (valem para várias fichas):
> - `fn sheet()` (`lib.rs:2181`) avalia espessura, raio e K e converte dedução/tolerância em K equivalente por `sheet_k` (`doc.rs:1721`).
> - O `flat` é doc-wide (`SheetParams.flat`) e alternado por `Cmd::SmUnfold` (`app.rs:1564`). **Não existe recurso "Desdobrar" na árvore.**
> - O DXF planificado recalcula o documento com `flat = true` (`fenkra-desenho/src/source.rs:272-290`).
> - `sheet_tool` (`lib.rs:2192`) tem um bloco morto (`lib.rs:2235-2238`: `let _ = info;`). A checagem "a face lateral tem a espessura da chapa?" **não faz nada**.

### 2.1 Parâmetros de chapa (espessura, raio, regras e estilos)
(CAD avançado B: Sheet Metal Parameters / CAD avançado A: Sheet Metal + gauge table / CAD avançado C: Sheet Metal Rule/Style / outros: Fusion Rules Library, CAD avançado E Sheet metal model / Fenkra: `SheetParams`)

- **O que faz:** define uma vez por peça o material, a espessura, o raio padrão, o método de desenvolvimento (K, dedução, tolerância ou tabela), o alívio e a folga padrão. As features herdam esses valores e podem sobrescrevê-los.
- **CAD avançado B — no detalhe:** [V]
  - Janela com abas **Parameters, Bend Extremities, Bend Allowance** (a quarta guia do R14). Campos: espessura, **raio mínimo de dobra** (raio interno mínimo permitido), **raio de dobra padrão** (interno, usado por padrão ao criar dobras).
  - O raio pode ser 0 (dobra sem raio). Com a norma DIN, o K também vai a 0.
  - A norma de chapa vem de uma **design table** (`SheetMetalStandard`, `Thickness`, `MinimumBendRadius`, `DefaultBendRadius`, `KFactor`, `RadiusTable`), com sub-tabelas por feature (furo de folga, furo de índice, bead, estampo circular e de superfície, furo flangeado, flanged cutout, curve stamp). A espessura é obrigatória na tabela; se houver tabela de raios e raio padrão, só a tabela vale.
  - O botão "Check all bend radii" lista abas e dobras que não respeitam o raio mínimo, sem corrigi-las.
  - Saída na árvore: recurso *Sheet Metal Parameters*. Os parâmetros são literais com fórmula, e cada dobra ganha um K próprio, derivado do global por fórmula (que pode ser desativada). A ordem do recurso é a primeira da peça.
  - Quando a referência some: a design table é externa, e sem espessura os demais campos são ignorados.
  - Em peça reconhecida (Recognize), a espessura e o raio são **deduzidos da geometria** e a espessura não pode ser editada.
- **CAD avançado A — no detalhe:**
  - Os parâmetros ficam no recurso Sheet-Metal (base flange). Espessura, raio padrão e método (K-Factor, Bend Allowance, Bend Deduction, Bend Table, Bend Calculation) e alívio automático (Auto Relief). [V]
  - **Gauge table:** escolhe a bitola e o material e preenche espessura, raio e ângulos de jog. Há opção "Override value" para ângulos de jog; só pode haver uma tabela de dobra por peça. [V]
  - O raio e o tipo de alívio são editáveis por feature desmarcando "Use default radius" / "Use gauge table". [V]
- **CAD avançado C — no detalhe:** [V]
  - **Sheet Metal Rule** (espessura, material, método de desenvolvimento, tamanho de folga e alívios) criada a partir de um template e **reutilizável entre peças**.
  - Cada feature pode seguir os padrões ("Follow Defaults") ou escolher regra e espessura da lista.
  - Parâmetros de sistema como `BendRadius`, `GapSize`, `BendReliefWidth/Depth`, `MinimumRemnant`, `CornerReliefSize`.
- **Outros:** Fusion: biblioteca de regras com espessura, folga mínima, raio, tipo de unfold linear com K, relief de dobra (round/straight/tear, remnant, largura, profundidade), alívio de canto de 2 e 3 dobras. [V] CAD avançado E: K padrão 0,45, K de parede enrolada 0,5, raio, folga mínima (*minimal gap*), tipo de alívio de dobra e de canto com escala, *tabela de chapa* editável por dobra. [V]
- **Fenkra hoje:** `SheetParams { thickness, radius, k, flat, material, method, allowance }` (`doc.rs:1694`). Tabela fixa `SHEET_MATERIALS` (10 materiais: raio mínimo ×t, K, rampa mínima de joggle ×t, `doc.rs:1673`) e `SHEET_GAUGES` (`doc.rs:1687`). `sheet_suggest` devolve raio, K e rampa sugeridos (`doc.rs:1732`). **Falta:**
  - raio mínimo **não é verificado** em nenhuma feature (o dado existe e fica sem uso);
  - sem estilo/regra nomeada e reutilizável, sem tabela de bitola ligada à espessura, sem alívio e folga padrão no nível da peça;
  - o K é único. Não há K por feature (CAD avançado B, CAD avançado A e CAD avançado F permitem);
  - a peça só tem **uma** chapa: `SheetParams` é do documento, não do corpo.
  - **Bug provável:** `bend_table` lê `s.k` direto (`lib.rs:5098`) e ignora `method` e `allowance`, que `sheet()` usa. Com dedução ou tolerância, a tabela de dobras diverge da geometria.
- **Melhor prática para o Fenkra:**
  - Fluxo: material → bitola (preenche espessura, raio e K) → método → **regra de alívio e folga padrão** → avançado (K local, norma/tabela).
  - Regras: espessura é obrigatória antes de qualquer feature; tudo que a feature não define herda da regra; avisar quando raio < raio mínimo do material (sem corrigir em silêncio, como o "Check all bend radii").
  - Casos de teste: mudar bitola atualiza todas as abas; mudar a regra do material muda raio e K; raio abaixo do mínimo gera aviso; `bend_table` coincide com a geometria nos três métodos.

### 2.2 Compensação de dobra (fator K, dedução, tolerância, tabela de dobras)
(CAD avançado B: Bend Allowance tab + Bend Table / CAD avançado A: K-Factor, BA, BD, Bend Table, Bend Calculation / CAD avançado C: Unfold Rule linear, tabela e equação / Fenkra: `sheet_k`)

- **O que faz:** calcula o comprimento desenvolvido de cada dobra e portanto o tamanho do blank.
- **CAD avançado B — no detalhe:** [V]
  - A tolerância de dobra (W) é a largura da dobra na planificada. Fórmula: W = θ·(R + k·T), com θ o ângulo interno em radianos. Com `α` o ângulo de abertura, θ = π·(180−α)/180.
  - **K DIN:** a fórmula DIN usa k' tal que W = θ·(R + k'·T/2), logo k' = 2k, de 0 a 1. Ela pode ser desativada ou editada com o menu de contexto, e reativada com "Apply DIN".
  - **Dedução V:** L = A + B + V, em que A e B são as cotas externas. V é derivada de K e do raio. Ao atualizar uma dobra o CAD avançado B calcula primeiro a dedução (por regra ou por tabela) e depois deduz W.
  - **Tabelas de dobra:** amostras de espessura, raio, ângulo de abertura e dedução, com **interpolação** por ângulo. Quando a dedução vem da tabela, o K não é usado. As tabelas valem **só para o raio padrão** e para Bend e Bend From Flat; outras features usam o K local, que é preciso editar à mão.
  - Cada aba, bainha, gota ou aba de usuário tem uma guia **Bend Allowance** (botão *More*) para K local.
- **CAD avançado A — no detalhe:** [V]
  - Três métodos: K-Factor (padrão), Bend Allowance ou Bend Deduction (valor calculado ou empírico), Bend Calculation (planilha Excel com equações por ângulo) e Bend Table (planilha). Gauge Tables ligam espessura e K.
  - BA = (π/180)·ângulo·(R + K·espessura). BD = 2·tan(ângulo/2)·(R + espessura) − BA (ou OSSB − BA).
  - Aba tem "custom bend allowance" local.
- **CAD avançado C — no detalhe:** [V]
  - **Unfold Rule** com três métodos: *Linear* (K de 0 a 1; comprimento = (R + K·t)·ângulo), *Bend Table* e *Custom Equation*. O fator de spline (padrão 0,5) afeta contornos, rolos e lofts com curvas.
  - **Tabela:** linhas = ângulos, colunas = raios, células = dedução. Interpola entre pontos e tem tolerâncias de espessura, raio e ângulo. Há um **K de reserva** quando o pedido cai fora da tabela. Um arquivo pode ter várias tabelas (uma por espessura), em texto ASCII, com referência de ângulo (dobra ou abertura).
  - A equação personalizada escolhe tipo (compensação, BA, BD ou K), faixa de validade e K de reserva.
- **Outros:** CAD avançado E: K, allowance ou deduction como coluna padrão da tabela de chapa, editável por dobra. [V] CAD avançado F: K, Y e tabela por feature. [V]
- **Fenkra hoje:** `sheet_k(method, k, t, r, allowance)` (`doc.rs:1721`): método 1 = dedução a 90°, método 2 = tolerância a 90°, ambos convertidos num K equivalente fixo (clamp 0..1). Usado em toda feature via `SheetValues.k`. Tabela de dobras em `bend_table` (`lib.rs:5094`): BA = a·(r + k·t); dedução = 2·OSSB − BA. **Falta:**
  - tabela por ângulo × raio × espessura com interpolação;
  - K local por feature;
  - **a dedução de um valor medido a 90° é aplicada a todos os ângulos**, o que erra bastante a 45° ou 135°.
  - **Bugs prováveis:** além do `bend_table` ignorar `method`, as linhas de bainha usam sempre 180° (`lib.rs:5103`), enquanto a gota gera 200° e a enrolada 270° (`lib.rs:3765-3771`). Aba curva, perfil, superfície e joggle **nem entram na tabela**.
- **Melhor prática para o Fenkra:**
  - Fluxo: método → valor ou tabela → K local opcional por feature.
  - Regra única: **uma função só** calcula BA/BD a partir de (t, r, ângulo, K local, tabela). Geometria, linhas de dobra, tabela de dobras e DXF a chamam.
  - Testes: BA = a·(r + k·t) para 45°, 90°, 135°; tabela vs K coincidem nos pontos da tabela; interpolação entre dois ângulos; dedução ↔ tolerância convertem sem perder comprimento total; blank de caixa fechada = soma de planos + BAs.

### 2.3 Extremidades e alívios padrão de dobra
(CAD avançado B: Bend Extremities / CAD avançado A: Auto Relief, Custom Relief Type / CAD avançado C: Relief Shape, Bend Transition / Fenkra: `relief`, `relief_kind`)

- **O que faz:** define como termina o raio de dobra na borda do flange e como se resolve o encontro entre a dobra e uma borda livre.
- **CAD avançado B — no detalhe:** [V] seis tipos:
  - mínimo sem alívio (área comum das paredes de suporte);
  - mínimo com alívio quadrado (L1, L2 editáveis);
  - mínimo com alívio redondo (idem);
  - linear (a dobra é cortada por planos pelos pontos-limite);
  - tangente;
  - máximo (entre as arestas opostas mais distantes).
  - Há definição global na guia *Bend Extremities* e local (esquerda e direita) na dobra. O local vale sobre o global.
  - **Aviso do guia:** alívio quadrado ou redondo é calculado sobre toda a interseção e pode remover mais material do que o esperado. Nesses casos prefere-se um recorte como alívio.
- **CAD avançado A — no detalhe:** [V] Auto Relief (retangular, obround, tear) e *Custom Relief Type* na aba. Dobras e cantos têm tratamento de canto e transição que só aparecem na planificada.
- **CAD avançado C — no detalhe:** [V]
  - **Relief Shape:** Straight, Tear, Round, ou o padrão do estilo. Dimensões: largura A, profundidade B, remanescente mínimo C (resto de chapa menor que isso é removido).
  - **Bend Transition** (na planificada): None (spline), Intersection, Straight Line, Arc (com raio), Trim to Bend. A vista dobrada só mostra None ou Trim.
- **Outros:** CAD avançado E tem relief de dobra retangular/obround/tear com escala de profundidade e largura sobre a espessura. [V] CAD avançado F permite alívio por extremidade de parede (No Relief, Rip, Stretch, Rectangular, Obround). [V]
- **Fenkra hoje:** no `SmFlange`: `relief: Option<String>` (largura) e `relief_kind` 0 retangular, 1 redondo, 2 rasgo (`doc.rs:1753`). Implementado em `lib.rs:3140-3173`, com profundidade fixa = r + t. **Falta:** remanescente mínimo; transição de dobra; alívio padrão herdado da peça; "tear" sem corte. O recuo mínimo `min_gap` de 0,0001 (`lib.rs:3103`) é um truque para não ter faces coincidentes, não uma folga de fabricação.
- **Melhor prática para o Fenkra:**
  - Fluxo: alívio padrão da peça (regra) → por feature "padrão / sobrescrever" → transição de dobra só na planificada.
  - Regras: alívio nunca maior que a margem lateral; remanescente mínimo remove tiras finas; "tear" gera vão zero.
  - Testes: aba com recuo 5 e alívio redondo planifica sem sobreposição; alívio herdado muda ao trocar a regra da peça.

### 2.4 Parede base
(CAD avançado B: Wall, Extrusion / CAD avançado A: Base Flange/Tab / CAD avançado C: Face, Contour Flange, Contour Roll como base / Fenkra: `SmWall`)

- **O que faz:** cria a primeira parede (parede de referência) a partir de um perfil e da espessura.
- **CAD avançado B — no detalhe:** [V]
  - **Wall** a partir de sketch: o lado do material (*Material Side*) vem por padrão para cima. A primeira parede é a **Reference wall**.
  - Sketch com contorno interno gera parede **com furo**, mas o vazio é parte da parede e **não é um recorte editável**.
  - Também há **parede tangente**: perfil coincidente com uma parede existente, ligada por *Tangent to*, que a planificada enxerga mesmo sem dobra entre elas se o suporte puder ser desdobrado; senão o sistema avisa e impede.
  - **Extrusão:** tipos Dimension (Length1 e Length2), Up to plane ou surface, com *Mirrored extent*, *Symmetrical thickness*, *Invert material side*. O sketch pode ser aberto (mínimo uma linha); com arcos o *Up to* não existe e o arco ligado à peça não aparece na planificada.
  - **Rolled wall:** sketch em arco ou círculo, tipo Dimension, referência de planificação (início, fim ou meio). A abertura mínima é 0,5°; só se combina com abas e recortes.
  - Todos os recursos registram o corpo no PartBody ou em corpos agregados (`objeto de trabalho`).
- **CAD avançado A — no detalhe:** *Base Flange/Tab* com perfil aberto ou fechado e espessura. [V] O *Edit Flange Profile* mostra que o perfil aceita contorno fechado com furos internos. [V]
- **CAD avançado C — no detalhe:** Face a partir de perfil; Contour Flange com perfil aberto, *Flip Side* e *Both Sides* (centrar a espessura); *New Solid* ou *Join*. [V]
- **Fenkra hoje:** `SmWall { sketch, reverse, offset, mid, surface }` (`doc.rs:1737`, `lib.rs:3068`). Extruda a face do perfil por `t` (ou meio a meio com `mid`), com `offset` e, alternativamente, uma superfície engrossada (`thicken`). **Falta:**
  - perfil aberto (erro "precisa de um perfil fechado", `lib.rs:3080`), o que RF-CHP-002 já pedia;
  - parede com furo "contida" vs recorte editável: hoje o furo vem do perfil;
  - parede tangente (ligação sem dobra);
  - rolled wall (cilindro aberto).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (sketch ou superfície) → modo (fechado / aberto com espessura) → lado do material (cima, baixo, simétrico) → avançado (afastamento, parede tangente).
  - Regra: a parede base define o corpo e a face fixa da planificação (face de referência).
  - Testes: perfil aberto com espessura; `mid` simétrico; trocar o lado do material não muda o K; superfície engrossada vira chapa com dobras detectadas.

### 2.5 Aba em aresta (Wall On Edge / Edge Flange / Flange)
(CAD avançado B: Wall On Edge / CAD avançado A: Edge Flange / CAD avançado C: Flange / CAD avançado F: Flange wall / Fenkra: `SmFlange`)

- **O que faz:** dobra uma nova parede a partir de uma aresta de uma parede existente.
- **CAD avançado B — no detalhe:** [V]
  - **Seleção:** aresta de uma parede existente; também pode ser aresta de uma aba ou estampo.
  - **Tipo de referência:** *Height* (projeção ortogonal do topo da nova parede até a parede de referência, da face de baixo ou de cima) ou *Length* (valor absoluto da parede sem a dobra).
  - **Relimitação:** primeiro e segundo limite por *Undefined reference* ou *Up To Plane* (com offset e plano), o que gera paredes não retangulares.
  - **Ângulo:** por padrão perpendicular; atualizado dinamicamente.
  - **Perfil:** por padrão, borda superior vai para cima, borda inferior vai para baixo. *Reverse Position* inverte o perfil; *Reverse Side* troca o lado do material.
  - **Clearance:** afasta a parede da aresta; o valor é o raio da dobra naquela aresta. A parede de referência não muda, e quem muda é o comprimento da aba.
  - **With Bend:** cria a dobra junto; sem a opção, só a parede. As opções são **modais** e propostas à próxima parede.
  - **Comportamento no update (muito relevante):** a aba é definida pela **aresta de referência**. Quando outra feature encurta essa aresta (uma dobra numa aba vizinha ou um recorte), a aba é recalculada. Para evitar, cria-se a dobra manualmente ou posterga-se a feature que modifica. Wall On Edge é feature **contextual**: inverter o lado do material numa aba pode obrigar a re-especificar features baseadas nela, mesmo que não estejam diretamente ligadas. Inverter o lado da parede de referência relocaliza as abas filhas. Cortar e colar uma aba com filhas perde os filhos. *Isolate* apaga os dados de atualização e não pode ser desfeito depois de modificar a parede.
  - **Editar:** duplo clique abre o diálogo e pode trocar a aresta; **o sketch da aba não pode ser editado direto**.
- **CAD avançado B — Swept Walls (Flange):** [V] **Spine** = uma ou mais arestas contíguas, ou uma aresta + **propagação por tangência**. Modos *Basic* (suporte inteiro) ou *Relimited* (Limit 1 e Limit 2: pontos, faces planas, ponto + vértice). Comprimento medido sempre pelo ponto externo mais baixo, ângulo agudo ou obtuso, raio. *Trim Support* só em suporte plano e **não combina com Relimited**. *Reverse Direction* e *Invert Material Side* (esta só com Trim Support).
- **CAD avançado A — no detalhe:**
  - **Posição da aba:** Material Inside, Material Outside, Bend Outside, Bend from Virtual Sharp, Tangent to Bend. [V]
  - **Comprimento:** origem na Outer Virtual Sharp, Inner Virtual Sharp ou Tangent Bend. **Ângulos menores que 90° exigem vértice virtual** (Tangent Bend força ≥ 90°). [V]
  - **Perfil:** *Edit Flange Profile* permite desenhar o contorno real (contorno fechado que usa parte da aresta), inclusive com furos, em vez de aparar um retângulo; resulta em menos features e a mesma planificada. [V]
  - **Outros:** *Trim side bends* (remove material excedente quando a dobra de uma aba miter toca outra dobra; corte automático e não editável) [V]; raio e *custom bend allowance* por feature [V]; alívio personalizado [V].
  - **Evoluções:** desde 2021 aceita arestas circulares e não lineares (antes era swept flange) [V]; 2025 permite comprimentos diferentes por aresta e comprimento por vértice [V].
- **CAD avançado C — no detalhe:** [V]
  - **Edges:** Edge (arestas individuais) ou Loop (todas as arestas de um laço).
  - **Extensão lateral:** Edge (aresta inteira), Width (largura, centrada ou deslocada), Offset (dois deslocamentos a partir de faces/planos/vértices), From–To (entre geometrias que cortam a aresta).
  - **Altura:** Distance (valor) ou To (até uma geometria, com offset). Datum de medida: interseção das faces externas, das internas, plano tangente; *Aligned* ou *Orthogonal*.
  - **Ângulo:** valor ou By Reference (casar com face/plano). Raio padrão = `BendRadius` do estilo.
  - **Posição da dobra:** Inside/Outside of reference plane, Bend from Adjacent Face, Tangent to adjacent face.
  - **Placement:** plano pela aresta parceira, plano pela aresta selecionada ou plano tangente à face lateral; *Position* define em que lado do plano fica o material.
  - **Corner tab:** auto-miter quando os ângulos passam de 90° e *Miter Gap*; formas de canto e alívio.
  - **Unfold Rule** e **Relief Shape** por feature.
  - *Old Method* desabilita os controles novos em peças antigas.
- **Outros:** CAD avançado F oferece posição da dobra: *Profile on Edge, Bend Outside, Offset from Bend Start/Apex, Constrained*; alívio de dobra por extremidade; miter cut entre segmentos; corner treatment. [V] CAD avançado H: alívio de canto Bend only / Bend and face / Bend and face chain, e *Close 2-Bend Corner*. [V]
- **Fenkra hoje:**
  - `SmFlange { edge: [f64;3], length, angle, radius, off1, off2, flip, relief, relief_kind, side, miter, min_gap }` (`doc.rs:1753`; `lib.rs:3092`).
  - O kernel `sm_flange` acha a aresta reta mais próxima do ponto e devolve a ferramenta + `SheetEdgeInfo` (`fenkra-kernel/src/lib.rs:38-59`).
  - `side` 0/1/2 recua a parede (material fora/dentro) cortando uma faixa de `t` ou `t + r` (`lib.rs:3114-3137`).
  - `miter` corta as pontas por semiespaços a 45° (`lib.rs:2199-2211`); `min_gap` impõe 1e-4 de recuo.
  - Cada dobra registra um `SheetMap` (`lib.rs:2222`); o planificado é gerado re-executando em modo `flat`.
  - **O que falta:**
    - altura vs comprimento e posição "vértice virtual";
    - aresta como referência estável (hoje é **ponto clicado**, e "aresta mais próxima" muda de aresta em silêncio se a geometria muda);
    - largura centrada, entre limites;
    - cadeia de arestas com propagação por tangência;
    - K local;
    - perfil personalizado (Edit Flange Profile);
    - validação com abas vizinhas.
  - **Bugs prováveis:**
    - O bloco `if (info.thickness - p.t).abs() > 1e-3 && !p.flat { let _ = info; }` (`lib.rs:2235`) não faz nada: aba em aresta que não é de espessura `t` é aceita sem erro.
    - A meia-esquadria usa o plano bissetor `unit(e·s + d)` (`lib.rs:2208`), correto só para duas abas de 90° perpendiculares; para outros ângulos o corte sai errado.
    - O recuo `1e-4` esconde dois problemas: abas vizinhas sem espaço real e dobras em sequência (DR-55/56/57).
- **Melhor prática para o Fenkra:**
  - **Fluxo recomendado (comando unificado B31):**
    1. *Suporte:* uma ou mais arestas, propagação por tangência, referência = aresta nomeada (ADR 0010/0011).
    2. *Modo:* tipo de comprimento (altura externa, altura interna, comprimento reto) e posição da dobra (material dentro, fora, dobra fora, vértice virtual).
    3. *Parâmetros:* comprimento, ângulo (agudo/obtuso), raio.
    4. *Avançado:* extensão lateral (aresta inteira, centrada, deslocamentos, entre limites), alívio, meia-esquadria com folga, K local, perfil personalizado.
  - **Regras que evitam bugs simples:**
    - validar que a aresta pertence a parede de espessura `t`;
    - validar colisão de abas vizinhas antes de gerar (DR-57), com diagnóstico "falhou: aba colide com a Aba.N";
    - o recuo mínimo vira **folga de fabricação** da regra, não 1e-4;
    - a aresta de referência é recalculada quando o corpo muda (como o CAD avançado B), mas com aviso, nunca escolhendo outra aresta em silêncio.
  - **Testes que deveriam existir:**
    - 4 abas de 90° numa caixa, com e sem miter, planificada = soma de planos + 4 BAs;
    - duas abas lado a lado sem recuo, com validação;
    - dobras em sequência (aba sobre aba) em dobrado e planificado;
    - aba com ângulo 60° e 120°;
    - aresta que deixa de existir → estado "falhou" visível;
    - mudar `t` e voltar.

### 2.6 Aba por contorno / perfil (User Flange / Contour Flange / Miter Flange)
(CAD avançado B: User Flange / CAD avançado A: Miter Flange, Edit Flange Profile / CAD avançado C: Contour Flange / Fenkra: `SmProfileFlange`)

- **O que faz:** varre um perfil aberto (linhas e arcos) ao longo de uma ou mais arestas, gerando abas compostas com dobras automáticas nos cantos.
- **CAD avançado B — no detalhe:** [V] **User Flange**: seleciona-se a aresta (spine) e o perfil. O perfil deve ter **continuidade tangente** com a aresta e **plano normal à spine**. Botões *Remove All* e *Propagate*. K local na guia *Bend Allowance*.
- **CAD avançado A — no detalhe:** *Miter Flange* adiciona uma **série de abas** ao longo de uma ou mais arestas a partir de um sketch (cadeia); cria as pontas em meia-esquadria e o canto numa só etapa, o que dá a planificação correta. [V]
- **CAD avançado C — no detalhe:** [V]
  - **Contour Flange:** perfil aberto num sketch não consumido; *Edges* por aresta ou laço; *Flip Side* e *Both Sides*; *Follow Defaults*.
  - Pode ser o **recurso base** (cria o corpo).
  - Raio padrão do estilo e extensão de largura iguais à da Flange. Dobras são editáveis depois (*Edit Bend*: Edge, Width, Offset, From–To).
  - Cantos e alívios herdam o estilo; há overrides por canto.
- **Fenkra hoje:** `SmProfileFlange { sketch, l1, l2, radius, flip }` (`doc.rs:1594`; `lib.rs:3557`). Cadeia aberta, cantos arredondados no raio de dobra (`round_corners`), extrudada nos dois lados do plano do sketch (`l1`/`l2`). **Falta:** ligação à **aresta** (não seleciona aresta, é um perfil solto no plano); *Both sides*; extensão lateral. **Planificado:** devolve apenas o aviso "a aba por perfil aparece só dobrada" e **não gera nada na planificada** (`lib.rs:3559`): a planificada e o DXF ficam sem essa aba, e nenhuma linha de dobra é registrada.
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (aresta + perfil tangente, validando) → modo (extensão: aresta, centrada, deslocada) → raio, lados → avançado (cantos, alívio).
  - Regra: o perfil precisa começar tangente à aresta; senão erro com diagnóstico.
  - Testes: perfil em L e em Z planificam com 2 e 3 BAs; DXF contém as linhas de dobra de cada canto; perfil não tangente → falha com mensagem.

### 2.7 Aba varrida e cadeia (swept / tangente em curva)
(CAD avançado B: Swept Walls — Flange, Hem, Tear Drop, User Flange / CAD avançado A: Swept Flange, Edge Flange em aresta curva / CAD avançado C: Contour Flange em cadeia / Fenkra: `SmCurveFlange`)

- **O que faz:** aba que acompanha uma cadeia de arestas tangentes (retas e curvas).
- **CAD avançado B — no detalhe:** [V] Selecionar uma aresta e acionar a propagação por tangência traz toda a cadeia; para remover uma aresta só se removem as das extremidades (senão quebra a continuidade). Demais regras como em 2.5.
- **CAD avançado A — no detalhe:** *Swept Flange*: sketch de cadeia conectada de linhas e arcos (continuidade C0), cantos vivos recebem **fillet automático**; se o fillet falha (arestas curtas) a feature falha. Dois raios por lado do caminho. [V] A partir de 2021 o Edge Flange aceita arestas circulares. [V]
- **CAD avançado C — no detalhe:** [V] Contour Flange com Loop; o *spline factor* afeta como contornos com spline desenvolvem.
- **Fenkra hoje:** `SmCurveFlange { edge: Vec<[f64;3]>, length, flip, angle, radius, side }` (`doc.rs:1607`; `lib.rs:3583`, `curve_flange` em `lib.rs:2327`). A "aresta" é uma **polilinha copiada** na hora da escolha. Em dobrado: seção arco + parede varrida (reta = extrusão, curva = sweep). Em planificado: faixa desenvolvida `a·(r + k·t) + len`. **Falta:**
  - referência por aresta e tangência viva (a polilinha não acompanha mudanças do corpo);
  - registro no mapa de dobras (**sem `SheetMap`**, não gera linha de dobra nem entra na tabela);
  - tratamento do caso misto reta + curva (DR-54);
  - ângulo ≠ 90° em curva (a faixa desenvolvida em curva de raio pequeno distorce).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (aresta + propagar por tangência, referência nomeada) → modo (lado do material) → ângulo, raio, comprimento → avançado (K, alívios nas pontas).
  - Regras: o mesmo `SheetMap` generalizado a curvas (linha de dobra como curva, com raio e ângulo) — ver RF-CHAPA-12; cadeia que mistura reta e curva é uma **única feature** com junções suaves.
  - Testes: aresta com reta + curva (DR-54); raio de curva menor que o raio de dobra → erro claro; planificada de aba curva tem comprimento de arco coerente.

### 2.8 Aba entre perfis (Lofted Flange / Lofted Bend / Hopper)
(CAD avançado B: Hopper / CAD avançado A: Lofted Bend / CAD avançado C: Lofted Flange / Fenkra: ❌ — BM-CHP-01)

- **O que faz:** transição de chapa entre dois perfis (quadrado → redondo etc.), planificável.
- **CAD avançado B — no detalhe:** [V]
  - **Hopper superficial:** superfície *regrada* (ou criada por loft, que é recomendado por detectar segmentos canônicos), **reference wire** e **ponto invariante** (para a planificação), **tear wire** (linha de abertura). Opções: inverter lado do material, *Neutral fiber* (a superfície é a fibra neutra), inverter lado fixo.
  - **Hopper canônico** (cônico, cilíndrico ou plano): dois perfis de arcos e retas, cada segmento com par no outro perfil, mais dois pontos da linha de abertura.
  - Só se aplica a superfícies regradas.
- **CAD avançado A — no detalhe:** [V] *Lofted Bend*: **dois sketches abertos** ligados por loft, sem base flange. Métodos **Bent** (dobras reais com arcos facetados, planificada com linhas de dobra) e **Formed** (geometria formada sem linhas de dobra e sem K). Os perfis precisam de segmentos do mesmo tipo e sem arestas vivas, abertura alinhada; nas versões novas os planos podem ser não paralelos.
- **CAD avançado C — no detalhe:** [V]
  - **Lofted Flange:** dois perfis; saída **Die Formed** (transição lisa) ou **Press Brake** (faces planas + faces de dobra cilíndricas, com *Converge* opcional e controle de facetas por tolerância de corda, ângulo ou distância). **Tipo de saída não editável depois.**
  - Raio e unfold rule por feature e editáveis por dobra (*Edit Bends*).
  - Há relatos de planificada com dobras cruzadas inesperadas, resolvidas dividindo o modelo. [V, fórum]
- **Fenkra hoje:** não existe. Há `Shape::sweep`, loft e superfícies no kernel, que dão base. **Falta tudo.**
- **Melhor prática para o Fenkra:**
  - Fluxo: dois perfis abertos → saída (Formado / Dobrado com facetas) → espessura, raio → abertura (tear) e ponto fixo.
  - Regras: perfis compatíveis (mesmo número e tipo de segmentos), com validação antes de gerar; saída *Dobrado* gera `SheetMap` por faceta.
  - Testes: quadrado → redondo planificado sem sobreposição (usar o *Check Overlapping* do CAD avançado B como modelo); perfis incompatíveis → mensagem.

### 2.9 Aba por superfície (suporte)
(CAD avançado B: Surfacic Flange — Aerospace SMD / CAD avançado A: — / CAD avançado C: — / Fenkra: `SmSurfFlange`)

- **O que faz:** aba apoiada em uma superfície de suporte; relimita a base pela superfície.
- **CAD avançado B — no detalhe:** [V parcial] O guia R14 (workbench genérico) só cita a aba superficial nos trechos de alívio de canto: tipos de suporte incluem **Angle**, a aba tem **lados** (não pode ser "None" se houver alívio com *Supports Redefinition*), a base tem **limites desdobrados** e a feature só se aplica dentro deles. Detalhes de OML/IML/BTL aparecem em *Bend From Flat* (extrapolação da linha: *Axis, BTL, IML, OML*), onde BTL é a linha dos limites do raio, IML é a interseção das superfícies internas e OML a interseção do suporte com um plano normal à parede. O restante do comando fica em [C] (bancada Aerospace).
- **CAD avançado A / CAD avançado C:** não têm equivalente direto. A aproximação é Contour Flange com perfil derivado de interseção, ou loft. [C]
- **Fenkra hoje:** `SmSurfFlange { support, face, length, radius, flip, keep_base, mode, angle }` (`doc.rs:1639`; `lib.rs:3597`). Calcula a OML = suporte ∩ plano da face base, relimita a web (`split_side`), e chama `curve_flange`. Modos: 0 exato, 1 plano tangente no meio, 2 ângulo fixo. **Falta / bugs prováveis:**
  - falhou nos testes do usuário (DR-19/23, nervura de asa);
  - a OML é amostrada por polilinha (passo `dist/200`), sem referência viva ao suporte;
  - o ponto `face` escolhe o plano da web (frágil);
  - sem `SheetMap`: nenhuma linha de dobra, nem planificada exata (a faixa desenvolvida em curva é aproximada);
  - ângulo constante no modo 0 vem de um único ponto do meio (`mid`), mas a aba exata varia ao longo da OML.
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (superfície nomeada) + web (face nomeada) → modo (exato, aproximação, ângulo) → comprimento (EOP) e raio → avançado (relimitar, manter base).
  - Regras: a OML vem da interseção **viva** e reportada na árvore; se não houver interseção perto da web, estado "falhou" com diagnóstico.
  - Testes: nervura de asa com suporte curvo (caso do usuário); suporte plano = aba reta; trocar o suporte por plano tangente reproduz o modo 1.

### 2.10 Bainha (Hem) e gota (Tear Drop)
(CAD avançado B: Hem, Tear Drop / CAD avançado A: Hem / CAD avançado C: Hem / Fenkra: `SmHem`)

- **O que faz:** dobra a borda sobre si mesma (fechada, aberta, gota, enrolada).
- **CAD avançado B — no detalhe:** [V] **Hem** e **Tear Drop** são paredes varridas com spine, comprimento e raio; *Basic* ou *Relimited*, *Trim Support*, *Reverse Direction*, K local. A gota tem raio e comprimento próprios.
- **CAD avançado A — no detalhe:** [V] Tipos **closed, open, tear drop, rolled**. Comprimento (fechada e aberta), *Gap Distance* (só aberta), ângulo e raio (gota e enrolada). Material dentro ou dobra fora, inverter direção. Hems que se cruzam recebem miter automático com *miter gap*.
- **CAD avançado C — no detalhe:** [V] Tipos **Single, Double, Teardrop, Rolled**. Single e Double: *Gap* (entre faces internas) e *Length*. Rolled e Teardrop: *Radius* e *Angle*. Padrões vêm do estilo.
- **Outros:** CAD avançado E: hem reto (achatado ou raio), enrolado (ângulo > 180°), gota (folga mínima ou valor), alinhamento *Outer* ou *In place*, canto *Simple* ou *Closed*. [V]
- **Fenkra hoje:** `SmHem { edge, length, gap, flip, kind }` (`doc.rs:1788`; `lib.rs:3762`): usa `sm_flange` com ângulo 180° (kind 0), 200° (gota) ou 270° (enrolada), raio = `gap/2` (mín. 0,01 / 0,5·t / t). **Falta:** hem duplo; canto fechado entre bainhas; alinhamento externo; K específico (um hem fechado usa K bem menor). **Bug provável:** `bend_table` registra bainhas sempre como 180° (`lib.rs:5103`) — tabela diverge da geometria para gota e enrolada.
- **Melhor prática para o Fenkra:**
  - Fluxo: aresta(s) → tipo (fechada, aberta, gota, enrolada, dupla) → comprimento, folga ou raio e ângulo → avançado (alinhamento, canto).
  - Testes: bainha fechada tem espessura total ≈ 2t e comprimento planificado = comprimento + BA(180°); duas bainhas em cantos adjacentes sem colisão.

### 2.11 Joggle / Jog
(CAD avançado B: Jog / CAD avançado A: Jog / CAD avançado C: — (aba dupla) / Fenkra: `SmJoggle`)

- **O que faz:** degrau na chapa, com duas dobras inversas.
- **CAD avançado B:** o guia R14 não tem Jog nesta lista, só no catálogo do usuário. [C]
- **CAD avançado A — no detalhe:** [V] Escolhe-se a **fixed face** e a posição da cota: **Outside Offset, Inside Offset, Overall Dimension**. O *Jog Offset* tem condição de término e distância (*Reverse offset* só com *Offset From Surface*). **Fix projected length** mantém o comprimento projetado da face do jog. O ângulo do jog tem presets que a gauge table pode sobrescrever. Raio padrão editável por feature.
- **CAD avançado C:** sem comando de jog; faz-se com duas abas ou com Fold. [C]
- **Fenkra hoje:** `SmJoggle { a, b, depth, run, flip, angle, plane, keep_length }` (`doc.rs:1487`; `lib.rs:3176`). Divide a chapa pela linha a–b, desloca a parte distante por `depth` e liga com um **paralelogramo reto** (rampa), sem raios. `keep_length` compensa com `sqrt(run² + depth²) − run`. **Bugs prováveis (leitura do código):**
  - **não há ramo `sp.flat`**: no planificado o joggle continua deslocado e a planificada/DXF fica errada;
  - a rampa tem cantos vivos (sem raio de dobra), então não há BA de verdade;
  - **não registra `SheetMap`**, então nada posterior mapeia pelo joggle;
  - `a`/`b` são pontos soltos, e a linha pelo plano gera `10 mm` de linha fixa (`lib.rs:3202`).
- **Melhor prática para o Fenkra:**
  - Fluxo: linha de referência (aresta ou sketch) + face fixa → posição da cota (externo, interno, total) → desvio, ângulo e raio → avançado (manter comprimento projetado).
  - Regras: dois `SheetMap` (um por dobra, sinais opostos); rampa com raio de dobra; validação da rampa mínima do material (`SHEET_MATERIALS`, 4º campo, já existe).
  - Testes: planificado tem comprimento = soma + 2 BAs; rampa < mínima avisa; joggle entre duas abas.

### 2.12 Dobra a partir de sketch (Sketched Bend / Bend From Flat / Fold)
(CAD avançado B: Bend From Flat / CAD avançado A: Sketched Bend / CAD avançado C: Fold / Fenkra: `SmBend`)

- **O que faz:** dobra uma chapa plana ao longo de uma linha desenhada, sem adicionar material.
- **CAD avançado B — no detalhe:** [V]
  - Seleciona-se a parede de referência e depois o **sketch com uma ou várias linhas**. Extrapolação da linha: Axis, **BTL** (limites do raio), **IML**, **OML**. Raio e K vêm dos parâmetros (fórmula desativável) e podem ser 0. Ângulo entre a parede gerada e a de referência.
  - Numa parede de referência, uma seta indica qual parte dobra.
  - **Regras:** as linhas não podem cruzar áreas com feature 3D (chanfro longitudinal, estampo punch-and-die). Dobre **em paredes de ponta** ou antes de criar mais paredes sobre a dobrada. **Dobre antes de criar estampos**, porque estampos não são retidos quando a peça dobra.
  - Pode agregar corpos (*objeto de trabalho* + Aggregating Bodies).
- **CAD avançado A — no detalhe:** [V] **Fixed face** (a que não se move), posição da dobra (*Bend Centerline, Material Inside, Material Outside, Bend Outside*), ângulo, raio e *bend allowance* custom. Só **linhas** no sketch, e a linha não precisa cobrir a face toda. Linhas colineares podem deixar uma aba central sem dobrar.
- **CAD avançado C — no detalhe:** [V] **Fold:** dobra em torno de uma linha de dobra reta que termina nas arestas da face; **não adiciona material**. *Fold Location*: centro, início ou fim da dobra. Raio e ângulo, *Flip Side* e *Flip Direction*.
- **Fenkra hoje:** `SmBend { a, b, angle, radius, flip, down }` (`doc.rs:1509`; `lib.rs:3258`). Corta a chapa em duas metades por semiespaços, mantém a seção, revolve-a (`revolve`) com o ângulo e desloca a parte móvel. A linha é um par de **pontos** (a, b) — **só reta**. **Bugs prováveis / falta:**
  - não aceita sketch com spline (DR-17/22, backlog B03) e **não emenda os raios** (a dobra por segmentos é aproximada por uma só);
  - em planificado devolve "planificada: dobra desfeita" e **não registra `SheetMap`**, então recortes, abas e linhas de dobra posteriores não passam por ela (`lib.rs:3260`);
  - não há *posição da dobra* (centro/início/fim) nem escolha da face fixa por seleção, só `flip`.
- **Melhor prática para o Fenkra:**
  - Fluxo: face fixa → sketch de linha(s) (reta ou spline) → posição da dobra (centro, início, fim, vértice) → ângulo, raio → avançado (K local, agregar corpos).
  - Regras: a dobra é **recurso de chapa** como a aba (registra `SheetMap`); linhas não podem cruzar estampos e a UI avisa para dobrar antes de estampar; linha que não atravessa a chapa → erro explicativo.
  - Testes: dobra a 90° no meio de uma placa, comprimento planificado = soma + BA; duas linhas paralelas (zig-zag); linha oblíqua; spline sai como cilindro tangente.

### 2.13 Dobrar / desdobrar e planificação
(CAD avançado B: Unfold, Multi-view, Fold/Unfold local / CAD avançado A: Flatten, Unfold, Fold / CAD avançado C: Flat Pattern, Unfold/Refold / Fenkra: `Cmd::SmUnfold`)

- **O que faz:** mostra a peça planificada e permite desdobrar só algumas dobras para criar features que as atravessam.
- **CAD avançado B — no detalhe:** [V]
  - **Unfold** desdobra a peça inteira relativa à parede de referência; segundo clique redobra. Cada feature é criada numa visão (dobrada ou planificada), e **editar exige estar na visão de definição** — senão o CAD avançado B avisa e pede para trocar. Dobras e estampos aparecem na planificada, mas recortes sobre estampos não.
  - **Multi-view:** abre a planificada em outra janela; qualquer modificação aparece nas duas. Só peças com dobras; faces de corte e de abertura não aparecem.
  - **Fold/Unfold local:** desdobra faces cilíndricas ou cônicas (flange, dobra, faces reconhecidas) **só na visão dobrada**, sem mudar a planificada: *Reference face* (parte fixa), *Unfold faces* (dobras) e, na dobra, tipo de ângulo *Natural* (90°), *Defined* e *Spring back*. Não permite dobrar/desdobrar: faces em domínios diferentes; faces com raio zero; cilindros de estampos ou paredes enroladas. Cada elemento da dobra com ciclo precisa ser selecionado individualmente.
  - **Check Overlapping:** na planificada, destaca áreas sobrepostas, informa o número e gera uma curva para resolver.
  - **Mapping:** pontos e curvas (logotipos, áreas de usinagem química, recortes) mapeados pela dobra, com *Mapping Context* (por padrão a última feature).
- **CAD avançado A — no detalhe:** [V]
  - **Flatten** (ou suprimir *Process-Bends*) aplica o tratamento de canto só no estado plano; editar o *Flat-Pattern* pode ser suprimido junto.
  - **Unfold/Fold** selecionam dobras individuais. A recomendação do CAD avançado A é desdobrar **só as dobras necessárias** para o desempenho. O uso típico é cortar através da dobra: desdobra a dobra, desenha o perfil fechado sobre a dobra planificada, extrusão passante e redobra.
- **CAD avançado C — no detalhe:** [V] **Unfold/Refold** com *Reference* (face ou plano), *Geometry* (dobras individuais ou todas) e *Sketches* (esboços não consumidos que acompanham). Requer pelo menos uma face plana (ou plano temporário para rolos). O **Flat Pattern** é um estado de documentação com comandos próprios. Há **Bend Order Annotation** (ordem de dobra: reordenar direto, sequencial ou individualmente) para a tabela de dobras do desenho, e *Project Flat Pattern* + *Slice Graphics* para esboçar sobre as faces desdobradas.
- **Fenkra hoje:** o planificado é um **flag global** do documento (`SheetParams.flat`) alternado por `Cmd::SmUnfold` (`app.rs:1564`). Cada feature de chapa tem ramos `if sp.flat`. Não há recurso *Desdobrar* na árvore e **não há desdobrar parcial** (BM-CHP-02). A visão planificada do desenho clona o doc, liga `flat` e extrai o formato e as linhas de dobra de `sheet_maps` (`source.rs:272-290`; `lib.rs:4380`). **Bugs prováveis / falta:**
  - só as dobras em `sheet_maps` entram nas linhas (aba e bainha); curva, perfil, superfície, `SmBend` e joggle ficam fora;
  - `ProfileFlange`, `Louver` e `Stamp kind 3` em planificado só avisam e não geram geometria (`lib.rs:3317`, `3422`, `3559`);
  - sem Check Overlapping nem detector de sobreposição;
  - sem ordem de dobra.
- **Melhor prática para o Fenkra:**
  - Fluxo: *Desdobrar* como **recurso na árvore** com: face de referência → dobras (todas ou escolhidas) → sketches/pontos que acompanham.
  - Regras: a planificação completa e o DXF usam o **mesmo** mapa de dobras que o desdobrar parcial; `Check Overlapping` roda na planificada e lista sobreposições.
  - Testes: caixa com 4 abas planifica em cruz sem sobreposição; desdobrar só a dobra 2, cortar, redobrar → o corte aparece nas duas visões; peça com 2 abas coplanares vizinhas sobrepostas é detectada.

### 2.14 Cantos (alívio de canto, fechamento, arredondar, chanfrar)
(CAD avançado B: Corner Relief, Corner, Chamfer / CAD avançado A: Corner Relief, Closed Corner, Welded Corner, Break Corner / CAD avançado C: Corner Seam, Corner Round, Corner Chamfer / Fenkra: `SmCornerRelief`)

- **O que faz:** resolve o encontro de duas ou três dobras num canto.
- **CAD avançado B — no detalhe:** [V]
  - **Corner Relief** local: precisa da peça **planificada**. Perfil do usuário (sketch, seta indica o lado da matéria removida) ou **circular** (raio padrão = raio de dobra + espessura, centro padrão na interseção dos eixos de dobra). *Supports Redefinition* substitui o lado das abas superficiais e adiciona matéria aos suportes, mas não é possível se o perfil adiciona matéria à web ou fica fora dos limites desdobrados.
  - **Corner** (arredondar): raio, arestas convexas e/ou côncavas, *Select All*; funciona dobrado ou planificado e **só num suporte**. Arestas criadas depois não são arredondadas automaticamente.
  - **Chamfer:** duas medidas ou medida + ângulo; propaga pelas arestas tangentes.
- **CAD avançado A — no detalhe:** [V] **Closed Corner:** tipos Butt, Overlap, Underlap; *Gap distance*; razão overlap/underlap; *Open bend region*; *Narrow corner* (algoritmo para raios grandes); *Coplanar faces*. **Welded Corner** adiciona cordão. O tratamento de canto só aparece na planificada.
- **CAD avançado C — no detalhe:** [V]
  - **Corner Seam:** entre faces coplanares ("Miter") ou não ("Seam"); tipos de sobreposição 45°, Symmetric Gap, No Overlap, **Overlap**, **Reverse Overlap**, com *Percent Overlap* (0 a 1) e **gap** (medido como gap máximo ou face/aresta). Extensão *Aligned* ou *Perpendicular*.
  - **Corner Rip** abre cantos de peça convertida.
  - **Cantos de 2 dobras:** Round, Square, Tear, Trim to Bend, soldas (linear, arco, laser). **Cantos de 3 dobras:** No Replacement, Intersection, Full Round, Round with Radius (só na planificada). Edição por canto e *Reset All Corners*. **Auto-mitering** se os ângulos passam de 90°.
- **Outros:** CAD avançado E tem *Corner break* (fillet/chamfer) e tipos de alívio de canto (Square, Rectangle, Round, Closed, Simple) com escala. [V] CAD avançado F: *Corner treatment (edge rip)*: Open, Gap, Blind, Overlap. [V] CAD avançado H: **Close 2-Bend Corner** com distância entre as bordas internas; sem entalhe quadrado sobra um furo, e com entalhe sobra pequena sobreposição. [V]
- **Fenkra hoje:** `SmCornerRelief { at, size, kind }` (`doc.rs:1664`; `lib.rs:3725`): corta um furo redondo, quadrado ou "gota" no ponto do canto. Mais o alívio dentro da aba (§2.3). **Falta:** fechamento de canto entre abas (overlap, underlap, gap), cantos de 2/3 dobras com tratamento próprio na planificada, arredondar/chanfrar cantos de chapa por seleção, auto-miter por ângulo. **Bug provável:** o ponto `at` é clicado e o sentido "para dentro" usa o centro do bbox da chapa inteira (`lib.rs:3732`): numa peça em L, o "para dentro" pode apontar para fora da chapa.
- **Melhor prática para o Fenkra:**
  - Fluxo: detectar canto (2 ou 3 dobras) → tipo de tratamento (seleção na lista do CAD avançado B/CAD avançado C) → tamanho → por canto, sobrescrever.
  - Regras: o canto de 3 dobras só tem forma na planificada; folga mínima vem da regra da peça.
  - Testes: caixa com 4 cantos e abas de 90°, planificada sem sobreposição com cada tipo; "overlap 50%" gera 2 abas com metade da espessura cada.

### 2.15 Recorte através da dobra (Cut Out / Cut Across Bend / Normal Cut)
(CAD avançado B: Cutout, Hole / CAD avançado A: Extruded Cut + Normal Cut / CAD avançado C: Cut → Cut Across Bend / Fenkra: `SmCut`)

- **O que faz:** corta a chapa de modo que a forma planificada seja a desenhada, mesmo atravessando a dobra.
- **CAD avançado B — no detalhe:** [V]
  - **Cutout:** perfil extrudado e removido. *Reverse Side* (remove o que fica fora do perfil), *Reverse Direction*. Limites: Up to next, Up to last, Dimension. Pode ter **vários suportes**, evitando confusão em sobreposições. O tipo *Sheetmetal pocket* tem profundidade menor que a espessura. *Lying on skin* faz o corte projetado na pele.
  - **Direção:** por padrão normal ao perfil; pode ser outra linha. Se o prisma é tangente à pele, a projeção é inválida.
  - O pocket só em suporte **plano e único**, não em estampo ou aba superficial.
  - **Hole** pode ser feito na planificada e em dobras.
- **CAD avançado A — no detalhe:** [V]
  - **Normal Cut** (desde 2018): torna normais à chapa as paredes de cortes não normais, para que a peça dobre e planifique onde um corte comum falharia. Pode selecionar várias faces e tem propagação automática e **grupos** (se uma mudança de projeto causar erro, edita-se e cria-se novo grupo).
  - **Cut across bends:** desdobra só a dobra, sketch fechado sobre ela, passante, redobra.
- **CAD avançado C — no detalhe:** [V] **Cut → Cut Across Bend:** o perfil "envolve" o raio até a próxima face e corta; na planificada o corte tem a forma do sketch. **Cut Normal:** perpendicular ao plano do sketch. Profundidade: distância (padrão = espessura, ou Thickness/n), Through All, To, To Next. *Between* para dois planos.
- **Fenkra hoje:** `SmCut { sketch, mode, offset, draft, height, flip }` (`doc.rs:1578`; `lib.rs:3516`). Modos 0 normal, 1 com saída (ângulo), 2 flangeado. Extruda o perfil ao longo da normal do sketch (`big = 4t + 2`). Dobrado ↔ planificado usa `map_tool` (`lib.rs:2490`). **Bug provável de raiz (DR-20):** `SheetMap::region` cobre o **flange reto** no dobrado (`u ∈ [0, length]`), mas **não o arco da dobra**; no planificado cobre também a zona desenvolvida `ba`. A parte de um corte que cai na zona do raio é movida como corpo rígido (translada e gira), o que **não representa** o enrolamento do raio. Daí "dobrado → planificado funciona, planificado → dobrado falha quando pega o raio".
- **Melhor prática para o Fenkra:**
  - Fluxo: sketch **desenhado no estado em que o usuário vê** → modo (normal à pele, **através da dobra**) → profundidade → lado → avançado (saída, flangeado, folga).
  - **Regra de ouro (CAD avançado C/CAD avançado A):** o corte através da dobra é definido **na planificada**; o Fenkra deve **mapear o perfil** (curvas 2D) pelo `SheetMap` — o comprimento ao longo da dobra segue o arco `r + k·t` — e extrudar **normal à superfície local**. Não deve mover o sólido rígido.
  - Testes: furo oblongo atravessando uma dobra de 90° criado no planificado e visto no dobrado e vice-versa; área do furo na planificada = área do sketch; corte em duas dobras em sequência; corte tangente à dobra.

### 2.16 Estampos e formas (louver, bridge, flanged hole, dimple, bead, rib, estampo do usuário)
(CAD avançado B: Stamping / CAD avançado A: Forming Tools, Lance, Louver / CAD avançado C: Punch Tool / Fenkra: `SmStamp`, `SmLouver`, `SmFlangedHole`)

- **O que faz:** deforma localmente a chapa com formas padrão ou definidas pelo usuário.
- **CAD avançado B — no detalhe:** [V]
  - **Regras gerais:** estampos são criados em paredes (ou paredes em aresta); **exceção:** stiffening rib vai na dobra. **Não se criam estampos na planificada.** Estampo sobre o limite de vários suportes não aparece na planificada, que só retém a maior marca do estampo. Todos consultam *standard files* para tamanhos nomeados.
  - **Flanged Hole:** ponto na face, com grade de apoio; parâmetros por *Major Diameter*, *Minor Diameter*, *Two Diameters* ou *Punch & Die*; **com ou sem cone**; altura H, raio R, ângulo A, diâmetros D e d; arquivo de norma.
  - **Bead:** perfil spine; altura H, raio R, raio de seção R1, raio de extremidade R2; R = 0 desativa o fillet.
  - **Circular Stamp:** H, R1, R2, A, D; opção sem fillet. **Half pierce** disponível para circular, curva e superfície.
  - **Surface Stamp:** sketch com vários contornos internos, curva 3D ou punch-and-die; H, R1, R2, A no topo ou na base; **opening edges**.
  - **Bridge:** ponto + aresta de direção; H, R1, R2, A, L1, L2.
  - **Flanged Cutout:** perfil; H, R, A; se o perfil não é tangente contínuo, não se criam outras features sobre ele.
  - **Stiffening Rib:** superfície externa da dobra; sempre **centrada no raio**; L, R1, R2, A.
  - **Curve Stamp:** H total, R1 (raio externo), R2 (raio interno), A (saída), L (largura máxima), **Obround**.
  - **Louver:** sketch + opening line; H, R1, R2, A1, A2; a face de abertura é o único elemento sem tangência com os demais.
  - **Reconhecer estampos:** circular, curva, superfície, bead, bridge, louver; sobre suporte plano e único. Não reconhece rib nem estampos com contornos internos.
  - **User-Defined Stamp:** **punch com die** (a forma é a diferença dos dois) ou **punch com faces de abertura**. O punch é um corpo no eixo do documento (direção do punch = Z); aplica-se casando a direção com a normal da parede e o plano (x, y) com a face; raio de dobra, referência de rotação, ângulo, origem; *On context* usa os eixos reais. Punch e die ficam ocultos. Pode editar (punch, die, posição, restrições). Se o punch é editado, o estampo com faces de abertura **não atualiza sozinho** (precisa reeditar). A altura do punch não pode superar a da base. A planificada mostra só o sketch do estampo.
- **CAD avançado A — no detalhe:** [V] **Forming tool:** peça especial que age como matriz. Cria-se com o comando *Forming Tool*: **Stopping Face** (profundidade), **Faces to Remove** (aberturas, tipo lance/louver), **ponto de inserção**; salva-se na pasta *Forming Tools* da Design Library e arrasta-se sobre uma face plana. **Corpo único.** Falha se o raio côncavo da ferramenta é menor que a espessura.
- **CAD avançado C — no detalhe:** [V] **Punch Tool:** iFeature 3D com um centro de furo. *Centers* (uma instância por centro), *Angle*, parâmetros editáveis, **Across Bend** (desligado: o punch para na dobra). Representação alternativa na planificada (camadas `IV_ALTREP_*` no DXF) e marcas de centro (`IV_TOOL_CENTER`).
- **Fenkra hoje:**
  - **SmStamp** (`doc.rs:1539`; `lib.rs:3344`), 5 tipos: perfil fechado, nervura (trapézio varrido), circular (tronco de cone), reforço de dobra, punção (corpo → casca, `shell_tagged`).
  - **SmLouver** (`doc.rs:1525`; `lib.rs:3314`): corte na linha a–b e rampa.
  - **SmFlangedHole** (`doc.rs:1561`; `lib.rs:3470`), com tabela `FLANGED_HOLES` (M3..M10, `doc.rs:1690`).
  - **O que falta e bugs prováveis:**
    - sem raios R1/R2 nem ângulo de saída configurável (seções trapezoidais retas);
    - sem *half pierce*, *bridge*, *opening edge* nem curva 3D;
    - parâmetros por "diâmetro maior/menor/punch & die" e norma de estampo só existem para o furo flangeado;
    - estampo sobre dobra ou fora do suporte plano não é detectado;
    - **não impede estampo na planificada** (CAD avançado B proíbe); avisa em alguns tipos e ignora em outros;
    - punção: o corpo do punch é lido como coletor sem *link* editável (o CAD avançado B avisa que editar o punch não atualiza o estampo);
    - a planificada não mostra o estampo (só o corte).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (face plana, ponto ou sketch) → tipo → parâmetros (H, R1, R2, A, L) → avançado (opening, half pierce, norma, face de início).
  - Regras: estampo **só no estado em que foi criado**, com aviso ao editar na outra visão; valida suporte plano e único; **verificar** se a face de início tem espaço (distância às bordas e às dobras) usando a regra da regra de chapa.
  - Testes: furo flangeado M5 numa chapa de 1,5 mm; louver com raio zero e com raio; estampo circular próximo à dobra → erro; punção com die; planificada com marca do estampo.

### 2.17 Padrões e espelho de features de chapa
(CAD avançado B: Rectangular/Circular/User Pattern, Mirror / CAD avançado A: Linear/Circular/Mirror / CAD avançado C: Rectangular/Circular/Mirror / Fenkra: `Pattern`, `Mirror`)

- **O que faz:** repete cortes, estampos e abas.
- **CAD avançado B — no detalhe:** [V]
  - Padrões **retangular, circular e do usuário**, que se atualizam também na planificada. *Simplified representation* alivia padrões grandes (> 10 instâncias). O padrão **não deve ultrapassar o modelo**.
  - **Padrão de aba (flange pattern):** só em aba tangente à aresta da parede (a direção do padrão tem de ser paralela à spine), e todas as instâncias na mesma face.
  - **Mirror** de feature de chapa por plano.
- **CAD avançado A / CAD avançado C:** padrões lineares, circulares e espelho aplicam-se a features de chapa (cortes, estampos, abas) com as mesmas regras. [C]
- **Fenkra hoje:** `FeatureKind::Pattern` e `Mirror` genéricos (`lib.rs:3060-3066`, `replicate`). **Falta:** padrão que entenda `SheetMap` (instância de aba cria uma nova dobra e uma nova linha de dobra), padrão de abas com validação de colisão.
- **Melhor prática para o Fenkra:**
  - Regra: padronizar abas gera N `SheetMap`s; a planificada e a tabela contêm N dobras.
  - Testes: 5 abas em padrão linear numa borda, planificada com 5 dobras; espelho de uma aba em chapa simétrica.

### 2.18 Converter sólido em chapa (Recognize / Convert to Sheet Metal / Insert Bends)
(CAD avançado B: Recognize / CAD avançado A: Convert to Sheet Metal, Insert Bends, Rip / CAD avançado C: Convert to Sheet Metal, Rip / Fenkra: ❌ — BM-CHP-03)

- **O que faz:** reconhece num sólido de espessura constante as paredes e dobras.
- **CAD avançado B — no detalhe:** [V]
  - **Recognize** reconhece **paredes e dobras** (bainhas viram paredes). Escolhe-se a **face de referência** (que também define os parâmetros padrão e a face de planificação); reconhecimento total ou manual, com faces para parede e para dobra. Também reconhece estampos (circular, curva, superfície, bead, bridge, louver), exige licença extra e planos únicos.
  - Em caixas é preciso escolher duas faces internas opostas e externas nas outras duas para evitar sobreposição.
  - Os parâmetros são **deduzidos da geometria**: espessura (não editável), raio padrão (por exemplo, 2× a espessura) e extremidades mínimas; K e alívio de canto continuam editáveis.
- **CAD avançado A — no detalhe:** [V]
  - **Convert to Sheet Metal:** escolhe-se a **face fixa**, a espessura (não precisa de shell; pode sobrescrever a importada), as **arestas que viram dobras**; as demais conectadas viram **rips**, gerados e pré-visualizados automaticamente, com etiquetas para ajustar cada rip e a opção de usar sketches lineares como rips. A feature continua editável na árvore, e *Keep body* mantém o sólido original (multicorpo).
  - **Insert Bends:** exige shell primeiro (a espessura vem do shell); escolhem-se os rips, e as dobras vêm automáticas. Cria *Flatten-Bends* e *Process-Bends*. A diferença visível: alívio retangular (Insert Bends) vs triangular (Convert).
  - **Rip:** só em arestas ou sketches lineares, com lado e vão.
- **CAD avançado C — no detalhe:** [V]
  - Uma peça de espessura constante pode ser convertida (inclusive de outros sistemas); a espessura tem de ser uniforme e igual à do estilo.
  - **Corner Rip** abre cantos quadrados para criar costura de canto. Dobras e *corner seams* são aplicados às arestas.
  - Não há comando mágico de shell para chapa. [V, Arkance]
- **Outros:** CAD avançado E: *Convert* e *Thicken*, arestas não selecionadas viram rips (ou juntas tangentes em arcos e splines), *Make joint* e *Modify joint*, folga mínima de rip. [V] CAD avançado H: *Rip Corner*. [V]
- **Fenkra hoje:** não existe. Há `shell_tagged`, `offset_face`, `Shape::section`, detecção de superfície por `surface_near` e a tabela de materiais, que ajudam. **Falta tudo.**
- **Melhor prática para o Fenkra:**
  - Fluxo: face fixa → espessura (detectada e editável) → arestas de dobra (propostas automaticamente por cilindros de raio r e r + t) → rips (propostos nos cantos) → parâmetros inferidos.
  - Regras: espessura constante com tolerância; deduzir raio e K (K padrão do material); erro explicativo se houver faces que não são plano/cilindro.
  - Testes: caixa de 4 abas convertida planifica; "U" convertido tem 2 dobras; peça com furo e pescoço; espessura variável → rejeita.

### 2.19 Desenho planificado e DXF
(CAD avançado B: Unfolded View + Save As DXF / CAD avançado A: Flat Pattern → DXF / CAD avançado C: Flat Pattern → Save Copy As DXF / Fenkra: vista `Flat` + DXF)

- **O que faz:** exporta o contorno planificado, os furos e as linhas de dobra para o corte (laser, punção).
- **CAD avançado B — no detalhe:** [V] Na Generative Drafting, **Unfolded View** pelo plano da árvore, com eixos e limites de dobra quando existirem. **Save As DXF** na própria bancada, abrindo no Drafting porque o DXF é reconhecido. As configurações DXF/DWG vêm da infraestrutura.
- **CAD avançado A — no detalhe:** exporta o flat pattern por DXF com camadas de contorno e dobras; opções de linhas de dobra e alívio. [C]
- **CAD avançado C — no detalhe:** [V] **Save Copy As → DXF** do nó *Flat Pattern*; escolha de versão (R12 é o mais aceito). Camadas: `IV_TANGENT` (limites de dobra), `IV_BEND` e `IV_BEND_DOWN` (eixos de dobra), `IV_TOOL_CENTER`, `IV_ALTREP_FRONT/BACK` (representação do punch). *Inner profiles* vêm de cortes e *feature profiles* de punches.
- **Fenkra hoje:** `fenkra-desenho/src/source.rs:272-290` clona o documento com `flat = true`, calcula `bend_lines` (centro, início e fim do arco, ângulo, raio, `up`) e usa a forma final como contorno. Há tabela de dobras no desenho (`source.rs:469`). **Falta / bugs:**
  - só as dobras com `SheetMap` aparecem, e perfil, curva, superfície, `SmBend` e joggle não;
  - sem camadas nomeadas por tipo (dobra para cima, para baixo, limites, ferramenta);
  - sem ordem de dobra;
  - sem verificação de sobreposição antes de exportar;
  - o contorno inclui a espessura (silhueta da face), com risco de linhas internas de dobra duplicadas; não há rotina de "encontrar a maior face plana" para extrair o contorno limpo.
- **Melhor prática para o Fenkra:**
  - Fluxo: peça → planificada → seleção de camadas → versão do DXF → exporta.
  - Regras: contorno externo, furos e linhas em camadas fixas e documentadas (`FK_BEND_UP`, `FK_BEND_DOWN`, `FK_TANGENT`, `FK_CUT`, `FK_TOOL`); aborta com erro se houver sobreposição ou dobra sem registro.
  - Testes: DXF de caixa com 4 abas tem 1 contorno externo + 4 linhas de dobra; coordenadas conferem com o blank da tabela; abrir o DXF e medir.

### 2.20 Tabela de dobras, ordem de dobra e normas de fabricação
(CAD avançado B: Check Overlapping, standards / CAD avançado A: bend table no desenho, Part Reviewer / CAD avançado C: Bend Order Annotation, Bend Table / Fenkra: `bend_table`)

- **O que faz:** informa por dobra o ângulo, o raio, o comprimento desenvolvido e a ordem, e verifica regras de fabricação.
- **CAD avançado B — no detalhe:** [V] A planificada mostra limites de dobra e eixos; *Check Overlapping* detecta sobreposição; as normas de furo, estampo e raio vêm de **design tables** e do raio mínimo (*Check all bend radii*). O guia cita o produto *Sheet Metal Production* para verificar sobreposições. Cada recurso de chapa guarda seus parâmetros no *spec tree*, o que permite regras de conhecimento (Knowledge Advisor).
- **CAD avançado A — no detalhe:** a ordem de dobra pode ser reordenada (blog GoEngineer); *Part Reviewer* checa normal cut. [V parcial]
- **CAD avançado C — no detalhe:** [V] **Bend Order Annotation** (sequência no estado planificado, extraída para a tabela de dobras do desenho). Bend tables e unfold rules são arquivos reutilizáveis.
- **Outros:** CAD avançado E lista dobras e rips na tabela de chapa, editável (raio de canto, tipo de junta) e usável no desenho; hems aparecem com raio e ângulo, reordenáveis. [V]
- **Fenkra hoje:** `BendRow { feature, name, angle, radius, allowance, deduction }` (`lib.rs:5082`; `bend_table` em `5094`), exibida em `app.rs:3097` e no desenho. **Falta:** ordem de dobra, ferramenta, verificação de fabricação (dobras muito próximas, furo perto de dobra, raio < mínimo, comprimento mínimo de aba — RF-CHP-015), propriedades automáticas do blank (RF-CHP-016). **Bugs prováveis:** ver §2.2 (método ignorado, bainhas e curvas ausentes).
- **Melhor prática para o Fenkra:**
  - Fluxo: tabela única `BendRow` gerada do **mapa de dobras** (não da lista de features), com colunas ordem, ângulo, raio, BA/BD, direção (cima/baixo).
  - Verificações padrão (todas configuráveis na regra): raio ≥ mínimo do material; aba ≥ 4×t + r [C: regra prática de oficina]; distância furo–dobra ≥ 2×t + r [C]; dobras a menos de N×t entre si; sobreposição na planificada.
  - Testes: peça com furo a 1 mm da dobra dispara aviso; tabela com 6 dobras de tipos diferentes tem 6 linhas e BAs corretas.

### 2.21 Transversal: ordem, contexto, update e diagnóstico (filosofia CAD avançado B)

- **CAD avançado B:** [V]
  - Aba em aresta é **contextual** (depende da aresta); mudar a aresta de origem recalcula, e inverter lado de uma parede pai pode exigir refazer filhos.
  - Cada feature é criada numa visão; editar exige essa visão.
  - Bend e Bend From Flat podem **agregar corpos** (*objeto de trabalho*): a dobra aparece sob o corpo atual e agrega o corpo da segunda parede. Em contexto híbrido, curvas geradas ficam sob o PartBody.
  - Mapeamento por contexto (*Mapping Context*).
  - *Isolate* desliga a atualização.
- **Lições para o Fenkra (alinhadas ao ADR 0011):**
  - Princípio 1 (referência estável ou erro visível) aplicado à aresta de aba.
  - Princípio 2 (um contexto ativo): a bancada de chapa mostra a "visão de definição" (dobrada/planificada) e o objeto em trabalho.
  - Princípio 7 (toda feature tem peça de teste): ver as listas por ficha e o corpus B53.

---

## 3. Requisitos levantados

> Nomenclatura: RF-CHAPA-nn / RNF-CHAPA-nn. Eles refinam RF-CHP-001..016 de `docs/requisitos/chapas.md`. Prioridades P0 (bloqueia o uso) a P3 (desejável).

### Funcionais

| ID | Requisito | Prior. | Critério de aceite testável |
|---|---|---|---|
| RF-CHAPA-01 | **Regra de chapa** nomeada: material, espessura, raio padrão, raio mínimo, método (K/BD/BA/tabela), alívio e folga padrão; herdada pelas features | P1 | Criar regra "Aço 2 mm"; 3 abas herdam o raio; mudar a regra muda as 3 |
| RF-CHAPA-02 | Verificar raio < raio mínimo do material (aviso visível, sem corrigir em silêncio) | P1 | Raio 0,5 em aço 2 mm → aviso na árvore e no painel |
| RF-CHAPA-03 | **Uma função de compensação** (t, r, ângulo, K local, tabela) usada por geometria, bend_lines, tabela de dobras e DXF | P0 | Teste: blank de caixa idêntico nos 3 métodos; `bend_table` = geometria |
| RF-CHAPA-04 | K local por feature (aba, bainha, perfil, dobra) | P2 | Aba com K 0,33 muda só o seu BA |
| RF-CHAPA-05 | **Tabela de dobra** ângulo × raio × espessura com interpolação e K de reserva | P2 | Tabela com 2 ângulos; 60° interpola; fora da faixa usa K de reserva |
| RF-CHAPA-06 | Parede base com perfil **aberto** + espessura (lado e simétrica) | P1 | Perfil em U aberto gera chapa; planifica |
| RF-CHAPA-07 | **Comando de aba unificado** (B31) com tipo: aresta, cadeia, perfil, superfície, entre perfis | P2 | Um comando abre as caixas do tipo escolhido; salvar/abrir mantém o tipo |
| RF-CHAPA-08 | Aba em aresta por **referência estável** (aresta nomeada), com diagnóstico quando some | P0 | Mudar o perfil da parede; a aba segue a aresta ou entra em estado "falhou" com Editar/Substituir; nunca muda de aresta em silêncio |
| RF-CHAPA-09 | Posição da dobra e referência de comprimento (altura/comprimento externo/interno, material dentro/fora, dobra fora, vértice virtual) | P1 | Aba de 60° com comprimento até o vértice virtual externo mede o esperado na peça |
| RF-CHAPA-10 | Extensão lateral: aresta inteira, centrada, deslocada, entre limites | P1 | Aba centrada de 30 mm numa aresta de 100 mm fica a 35 mm de cada ponta |
| RF-CHAPA-11 | Propagar seleção por tangência (cadeia reta + curva) numa única feature | P0 | Aresta com reta e curva: 1 feature, 1 clique, planificada coerente (DR-54) |
| RF-CHAPA-12 | **Mapa de dobras geral**: toda feature de dobra (aba, curva, perfil, superfície, joggle, dobra por linha, bainha) registra dobras (reta ou curva) com raio, ângulo, BA | P0 | `bend_lines` e `bend_table` contêm todas as dobras de uma peça com os 7 tipos |
| RF-CHAPA-13 | Validar abas vizinhas antes de gerar (colisão, folga) e diagnosticar | P0 | Duas abas lado a lado sem folga: erro "colide com Aba.N" (DR-57) |
| RF-CHAPA-14 | Meia-esquadria correta para ângulos diferentes de 90° e folga de fabricação real | P1 | Duas abas de 60° e 120° fecham com folga configurada; planificada sem sobreposição |
| RF-CHAPA-15 | Ao criar aba, validar que a aresta pertence a parede de espessura `t` | P0 | Aresta com 3 mm de lateral numa chapa de 2 mm → erro |
| RF-CHAPA-16 | Aba por perfil ligada a uma aresta, com tangência validada e planificada | P1 | Perfil em Z: planificada tem 3 BAs e DXF tem 3 linhas de dobra |
| RF-CHAPA-17 | Aba por superfície com OML viva e estado "falhou" explícito | P0 | Nervura de asa (caso DR-19) gera aba; remover o suporte → "falhou" visível |
| RF-CHAPA-18 | Aba entre perfis (loft) com saídas Formado e Dobrado e perfis validados | P2 | Quadrado → redondo planifica sem sobreposição |
| RF-CHAPA-19 | Bainha: fechada, aberta, gota, enrolada e dupla; tabela com ângulo real (180/200/270°) | P1 | Tabela de dobras de gota mostra 200°; bainha fechada ≈ 2t |
| RF-CHAPA-20 | Joggle com raios de dobra, planificação desenvolvida e registro no mapa | P1 | Planificado de um joggle tem comprimento = soma + 2 BAs |
| RF-CHAPA-21 | Dobra por sketch (reta e spline, várias linhas) com posição da dobra e registro no mapa; linhas emendadas | P0 | Spline gera dobra contínua (DR-17/22); planificada tem a linha de dobra |
| RF-CHAPA-22 | Recurso **Desdobrar/Redobrar** na árvore, total ou parcial, com sketches que acompanham | P1 | Desdobrar a dobra 2, cortar, redobrar; o corte aparece nas duas visões |
| RF-CHAPA-23 | Recorte através da dobra **por mapeamento do perfil** (não por movimento rígido), em ambas as direções | P0 | Furo no raio criado no planificado e no dobrado: mesma área plana; sem falha (DR-20) |
| RF-CHAPA-24 | Cantos: tratamento de 2 e 3 dobras (redondo, quadrado, tear, trim, soldas; interseção, full round) e fechamento com overlap/underlap/gap | P1 | Caixa com 4 cantos e cada tipo planifica sem sobreposição |
| RF-CHAPA-25 | Arredondar e chanfrar cantos de chapa por seleção | P3 | 4 cantos do blank arredondados no planificado e no dobrado |
| RF-CHAPA-26 | Estampos com H, R1, R2, A, L; bead, bridge, circular, curva, superfície, rib, flanged cutout e flanged hole (com/sem cone) | P2 | Cada estampo tem teste com medida de altura e raio |
| RF-CHAPA-27 | Estampo do usuário (punch+die, punch com faces de abertura) com link ao corpo e aviso de desatualizado | P2 | Editar o punch marca o estampo "desatualizado" e oferece reaplicar |
| RF-CHAPA-28 | Estampos só no estado em que foram criados; editar no outro estado avisa | P1 | Estampo na planificada recusado com mensagem |
| RF-CHAPA-29 | Verificação de espaço de estampo e furo: distância a bordas e dobras (da regra) | P2 | Furo a 1 mm da dobra dispara aviso |
| RF-CHAPA-30 | Padrão e espelho de features de chapa (abas, cortes, estampos) com N dobras no mapa | P2 | 5 abas em padrão: tabela tem 5 dobras |
| RF-CHAPA-31 | Converter sólido em chapa (face fixa, espessura detectada, dobras e rips propostos) | P2 | Caixa e "U" convertidos planificam; espessura variável é rejeitada |
| RF-CHAPA-32 | Planificação **completa e exata** (nenhum recurso some na planificada), com *Check Overlapping* | P0 | Todo `Sm*` gera geometria na planificada; sobreposição detectada e listada |
| RF-CHAPA-33 | DXF com camadas fixas (contorno, furos, dobra ↑/↓, limites, ferramenta) e versão (R12) | P1 | DXF aberto em outro CAD tem as camadas e o blank confere |
| RF-CHAPA-34 | Tabela de dobras do mapa, com ordem de dobra, direção, BA/BD, ferramenta | P1 | 6 dobras → 6 linhas ordenadas; reordenar atualiza o desenho |
| RF-CHAPA-35 | Verificações de fabricação configuráveis (raio mínimo, aba mínima, furo–dobra, dobras próximas) | P2 | Cada verificação tem peça de teste que dispara aviso |
| RF-CHAPA-36 | Propriedades automáticas: blank, área, massa, número de dobras | P1 | Valores batem com a planificada (RF-CHP-016) |
| RF-CHAPA-37 | Cada feature de chapa expõe diagnóstico de update (Editar, Substituir, Isolar, Desativar, Reordenar) | P1 | Feature "falhou" mostra as 5 ações |
| RF-CHAPA-38 | Agregar corpos ao criar dobras e abas (contexto *objeto de trabalho*) | P3 | Dobra aparece sob o corpo atual |

### Não funcionais

| ID | Requisito | Prior. | Critério de aceite |
|---|---|---|---|
| RNF-CHAPA-01 | Regeneração de chapa com 20 features em < 1 s (cache por feature já existente) | P1 | Medição no corpus B53 |
| RNF-CHAPA-02 | Toda feature `Sm*` tem pelo menos um teste nomeado (dobrado e planificado) | P0 | Lista de testes cobre 100% dos `FeatureKind::Sm*` |
| RNF-CHAPA-03 | Mudar uma cota e voltar reproduz exatamente a geometria (blank e volume) | P0 | Teste de ida e volta |
| RNF-CHAPA-04 | Nenhum erro silencioso: feature que falha vira estado visível com mensagem em português | P0 | Auditoria: nenhum `let _ =` descartando validação (ex.: `lib.rs:2235`) |
| RNF-CHAPA-05 | Salvar e abrir preserva parâmetros, regras, estado `flat_space` e referências | P0 | Teste salvar/abrir |
| RNF-CHAPA-06 | Precisão do blank ≤ 0,01 mm por dobra para o modelo K linear | P1 | Comparação com fórmula analítica |
| RNF-CHAPA-07 | Interface do comando segue suporte → modo → parâmetros → avançado (B20) | P1 | Revisão visual por comando |
| RNF-CHAPA-08 | Textos e tooltips em dois estágios (B60) e i18n | P2 | Verificação das chaves em `i18n.rs` |
| RNF-CHAPA-09 | Malha fechada e sólido válido após cada feature | P0 | `check` do kernel no corpus |
| RNF-CHAPA-10 | Normas de chapa e tabelas em dados (arquivo), não em código | P2 | `SHEET_MATERIALS` carregado de dados |

---

## 4. Itens de backlog propostos

> Os IDs B01..B04, B31, B35 são do backlog atual. "CHAPA-xx" são novos.

| ID | Título | O que muda | Prior. | Tam. | Dependências |
|---|---|---|---|---|---|
| CHAPA-01 | Mapa de dobras geral | `SheetMap` passa a ser registrado por **todas** as features de dobra, incluindo curvas (linha de dobra como curva com raio/ângulo/BA). Base de planificação, DXF e tabela | P0 | G | — |
| CHAPA-02 | Referência de aresta estável para aba | `SmFlange`/`SmCurveFlange`/`SmHem` usam aresta nomeada (ADR 0010) com propagação por tangência; polilinha só como cache | P0 | G | CHAPA-01, B59 |
| CHAPA-03 | Validação de espessura e vizinhas | Remove o código morto de `lib.rs:2235`, valida lateral = t e colisão entre abas; folga real em vez de 1e-4 | P0 | M | CHAPA-02 (B01) |
| CHAPA-04 | Compensação única | Função única BA/BD usada em geometria, linhas e tabela; corrige `bend_table` (método ignorado, bainha 180°, tipos ausentes) | P0 | M | CHAPA-01 |
| CHAPA-05 | Recorte por mapeamento do perfil | Substitui o movimento rígido de `map_tool` por mapeamento do perfil pelo arco; extrusão normal à superfície | P0 | G | CHAPA-01 (B02) |
| CHAPA-06 | Dobra por sketch (spline e várias linhas) | `SmBend` por sketch, posição da dobra, emenda de raios, `SheetMap` | P0 | M | CHAPA-01 (B03) |
| CHAPA-07 | Aba por superfície robusta | OML viva, estado "falhou", registro no mapa, casos de nervura de asa | P0 | G | CHAPA-01, CHAPA-02 (B04) |
| CHAPA-08 | Planificação exata | Nenhuma feature some na planificada (perfil, louver, estampos), Check Overlapping | P0 | G | CHAPA-01 |
| CHAPA-09 | Joggle com raios e planificação | Raios de dobra, ramo `flat`, dois `SheetMap` | P1 | M | CHAPA-01 |
| CHAPA-10 | Comando de aba unificado | Posição da dobra, referência de comprimento, extensão lateral, cadeia; vira B31 | P2 | G | CHAPA-02, CHAPA-03 (B31) |
| CHAPA-11 | Regra de chapa e raio mínimo | Regra nomeada, alívio/folga padrão, aviso de raio mínimo, K local | P1 | M | — |
| CHAPA-12 | Tabela de dobra ângulo × raio × espessura | Arquivo de tabela, interpolação, K de reserva | P2 | M | CHAPA-04 |
| CHAPA-13 | Cantos | Tratamento de 2/3 dobras, fechamento (overlap, underlap, gap), auto-miter por ângulo, corner break | P1 | G | CHAPA-01 |
| CHAPA-14 | Desdobrar/Redobrar parcial | Recurso na árvore, face de referência, dobras escolhidas, sketches que acompanham | P1 | G | CHAPA-01, CHAPA-05 (B35) |
| CHAPA-15 | Parede base aberta e tangente | Perfil aberto com espessura, parede tangente, parede enrolada | P1 | M | — |
| CHAPA-16 | Aba por perfil na aresta | Liga o perfil à aresta, tangência validada, planificada | P1 | M | CHAPA-02 |
| CHAPA-17 | Aba entre perfis | Loft/hopper, saídas Formado e Dobrado | P2 | G | CHAPA-01 (B35) |
| CHAPA-18 | Bainha completa | Hem duplo, canto fechado, tabela com ângulo real | P1 | M | CHAPA-04 |
| CHAPA-19 | Estampos paramétricos | H, R1, R2, A, L; bridge, half pierce, opening edge, normas | P2 | G | — |
| CHAPA-20 | Estampo do usuário e punch tool | Punch+die com link, aviso de desatualizado, Across Bend | P2 | G | CHAPA-19 |
| CHAPA-21 | Converter sólido em chapa | Face fixa, espessura detectada, dobras e rips propostos | P2 | G | CHAPA-01, CHAPA-04 (B35) |
| CHAPA-22 | DXF de chapa | Camadas fixas, versão R12, verificação prévia | P1 | M | CHAPA-08 |
| CHAPA-23 | Tabela de dobras e ordem de dobra | Gerada do mapa, ordem, direção, ferramenta | P1 | M | CHAPA-01 |
| CHAPA-24 | Verificações de fabricação | Raio mínimo, aba mínima, furo–dobra, dobras próximas | P2 | M | CHAPA-11 |
| CHAPA-25 | Padrão e espelho de abas | Padrão aware do `SheetMap` | P2 | M | CHAPA-01 |
| CHAPA-26 | Corpus de chapa (B53) | Uma peça de teste por `Sm*` em dobrado e planificado, ida e volta de cotas | P0 | M | — |
| CHAPA-27 | Diagnóstico de update de chapa | Estado "falhou" com Editar/Substituir/Isolar/Desativar/Reordenar | P1 | M | B59 |

---

## 5. Fontes

**CAD avançado B**
- Guia oficial *Generative Sheetmetal Design V5R14* (espelho): (fonte em procedencia.md)
- Apostila NIAR R30 (indicada na busca; o PDF devolveu 404): https://www.wichita.edu/industry_and_defense/NIAR/Documents/Sheet-Metal-Design-R30.pdf
- Resultados de busca com cursos e manuais de CAD avançado B (títulos, não lidos a fundo):
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - https://www.ibm.com/support/pages/apar/HC82197

**CAD avançado A**
- Edge-Flange PropertyManager (ajuda; só o trecho exibido na busca): (fonte em procedencia.md)
- Bend Positions (ajuda; só o trecho exibido na busca): (fonte em procedencia.md)
- Sketched Bend, Sketched Bends (ajuda; só o trecho exibido na busca): (fonte em procedencia.md) e (fonte em procedencia.md)
- Jogs, Hem, Closed Corners, Lofted Bends, Normal Cut (ajuda; só trechos de busca):
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
- GoEngineer: (fonte em procedencia.md) · (fonte em procedencia.md) · (fonte em procedencia.md) · (fonte em procedencia.md) · (fonte em procedencia.md) · (fonte em procedencia.md)
- Hawk Ridge: https://hawkridgesys.com/blog/shaping-sheet-metal-edge-flange · https://support.hawkridgesys.com/hc/en-us/articles/231224007-How-the-Tangent-Bend-Option-affects-your-edge-flange-s-angle (403; só o trecho da busca)
- SolidProfessor: (fonte em procedencia.md)
- CAD avançado A blog (bend allowance): (fonte em procedencia.md)
- Javelin (403; só o trecho da busca): https://www.javelin-tech.com/blog/?p=57275 · (fonte em procedencia.md)

**CAD avançado C**
- Flange: (fonte em procedencia.md) · (fonte em procedencia.md)
- Contour Flange: (fonte em procedencia.md)
- Bend Edit: (fonte em procedencia.md)
- Bend tab (relief e transition): (fonte em procedencia.md)
- Cuts: (fonte em procedencia.md)
- Unfold Rule e bend tables: (fonte em procedencia.md) · (fonte em procedencia.md)
- Lofted Flange: (fonte em procedencia.md)
- Punch Tool: (fonte em procedencia.md)
- Corner Seam e Corners: (fonte em procedencia.md) · (fonte em procedencia.md)
- Unfold/Refold e peças de chapa: (fonte em procedencia.md) · (fonte em procedencia.md)
- Fold, Hem, Bend Order e Export DXF (só os trechos exibidos na busca):
  - (fonte em procedencia.md) (404 no fetch)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
  - (fonte em procedencia.md)
- Conversão: (fonte em procedencia.md) · (fonte em procedencia.md)

**Outros**
- Fusion — regras de chapa: (fonte em procedencia.md)
- CAD avançado E — hem: (fonte em procedencia.md) · modelo de chapa: (fonte em procedencia.md) · bend relief: (fonte em procedencia.md)
- CAD avançado F — flange wall: (fonte em procedencia.md)
- CAD avançado H — reliefs e cantos: (fonte em procedencia.md)

**Código e docs do Fenkra lidos**
- `crates/fenkra-solidos/src/lib.rs` (linhas 439–491, 2181–2512, 3068–3784, 4380–4398, 5045, 5080–5113; testes em 5577, 5925, 6002, 6114, 7159–7416)
- `crates/fenkra-core/src/doc.rs` (linhas 1319, 1365–1391, 1487–1797, 1911–1979, 3304)
- `crates/fenkra-kernel/src/lib.rs` (linhas 38–59)
- `crates/fenkra-app/src/app.rs` (linhas 1564–1570)
- `crates/fenkra-desenho/src/source.rs` (linhas 272–290)
- `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`
- `docs/benchmark/chapa-metalica.md`
- `docs/benchmark/catalogo.md` (seção Chapa metálica)
- `docs/requisitos/chapas.md`
- `docs/modulos/chapa-metalica.md`
- `docs/backlog/backlog.md`
- `docs/contextos/design-review.md` (DR-19, DR-20, DR-22, DR-23, DR-55, DR-56, DR-57)

---

**Handoff**
- **Resultado:** relatório completo, sem alterar arquivos do repositório. Também não criei o arquivo de plano que o modo de planejamento sugeria, porque pediram para não criar arquivos. O relatório é a própria resposta.
- **Evidência:** código lido nas linhas citadas; trechos de ajuda lidos, mais o guia oficial CAD avançado B convertido para texto no scratchpad. O texto convertido ficou em `/tmp/claude/C--Users-Jo-o-Projetos-projeto-cad/45b47737-6a30-4247-8a66-37f76a2cfd81/scratchpad/gsd.txt`, no scratchpad da sessão.
- **Checks executados:** nenhum build ou teste.
- **Falhas e riscos:**
  - A ajuda online do CAD avançado A e a apostila NIAR não abriram; usei trechos de busca e blogs, e o CAD avançado B vem de uma versão antiga (R14).
  - Os "bugs prováveis" do Fenkra vêm de **leitura estática do código**; não rodei nada para reproduzi-los.
  - As regras práticas de fabricação marcadas [C] (aba mínima, furo–dobra) devem ser validadas com uma norma ou com o fabricante antes de virar regra.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra em chapa são: (1) referências por ponto ou polilinha em vez de aresta nomeada; (2) mapa de dobras que só cobre aba e bainha, deixando perfil, curva, superfície, dobra por linha e joggle fora da planificação, do DXF e da tabela; (3) recorte através do raio feito por movimento rígido de sólido, em vez de mapear o perfil pelo arco; (4) compensação de dobra só escalar e global, sem tabela ângulo × raio, sem K local e sem raio mínimo verificado, com `bend_table` divergindo da geometria; (5) ausência de conversão de sólido, aba entre perfis, desdobrar parcial e verificações e ordem de fabricação.
