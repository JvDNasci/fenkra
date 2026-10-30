# Prendedores aeronáuticos

**Escopo:** prendedores aeronáuticos e militares (AN, MS, NAS, Hi-Lok, Hi-Lite, lockbolts, cegos estruturais, rebites, porcas, arruelas, insertos, pinos, presilhas, clamps) e a inserção em massa por pontos publicados. Data da pesquisa: 2026-10-30.

**Convenção de confiança.**
- **[V]** = confirmado em página que abri, listada na seção 5.
- **[V~]** = confirmado só em parte (rótulo faltando, uma única listagem, ou fontes que divergem).
- **[C]** = conhecimento geral, não confirmado nesta pesquisa.

Tudo o que é **[C]** precisa de segunda fonte antes de entrar na biblioteca. A coluna `confirmado_por` do `biblioteca/fixadores/*.json` já serve para isso.

**Limites da pesquisa (importante).**
- O ASSIST quicksearch exige sessão e não foi aberto. Nenhuma norma NAS/MS/AN original foi lida. Os números vêm de distribuidores e apostilas.
- As páginas de peça da Boeing Distribution, do LAS Aero e da Bombardier PPS devolveram 403 na segunda tentativa. Só os trechos que a busca exibiu entram como **[V~]**.
- As páginas `help.solidworks.com` renderizam em branco para o leitor. O SolidWorks vem de dois artigos de revendas (Hawk Ridge e CATI) mais os resumos da busca sobre a ajuda.
- Para o CATIA V5 não achei tutorial público de instalação de prendedores aeronáuticos. A parte "como o CATIA faz" combina o que é público (FASTJOINT, a API DNB/DELMIA, publicações) com prática de indústria marcada **[C]**.

---

## 1. Resumo e lacunas

1. A biblioteca do Fenkra tem 6 famílias, todas **métricas** (ISO 4762, ISO 4017, DIN 933, ISO 4032, DIN 934, ISO 7089).
   - `TipoFixador` só conhece Allen, sextavado, porca e arruela (`biblioteca.rs:24`).
   - A designação é obrigatoriamente `M<diâmetro>` (`biblioteca.rs:81` e `:139`).
   - Não existe rebite, pino, colar, inserto, nem unidade polegada.
2. O fluxo de hoje (`preencher_furos`, `fenkra-montagem/src/fixadores.rs:106`) parte de furos que já existem. Ele casa o Ø do furo com a tabela métrica (±0,05 mm) ou lê rosca "M…".
   - Furos aeronáuticos, como #10 = 4,9 mm, nunca casam. A folga métrica mais próxima é 4,8 ou 5,3 mm.
   - Isso gera só avisos "sem tamanho compatível".
3. Prendedor aeronáutico não se escolhe por comprimento, e sim por **grip** (espessura fixada). O Fenkra calcula `L = pega + arruelas + porca + 2 passos` e arredonda para a série (`:267-268`). Falta o modelo "dash = grip em 1/16 pol, faixa mín–máx".
4. Não existe a ideia de **ponto/vetor publicado com cor = diâmetro**. Também não existem pasta de junta, grupo de prendedores, modo manual nem assistente de substituição. O grupo é plano ("Fixadores", `:38`).
5. A normal de entrada vem do eixo do furo. Não há leitura da normal da superfície em pontos de rebitagem, onde ainda não há furo.
6. Faltam tolerâncias de furo (ajuste justo, ream, interferência), escareado de 100° e dimple, e o "hardware stack" aeronáutico (colar, arruela de pegada, porca âncora).
7. Há um bug já registrado (B62): o parafuso inserido fica com 1 GDL de rotação. É pior com Hi-Lok, onde a orientação do sextavado do colar importa.
8. Dados: os catálogos públicos são ricos para rebites AD/DD, AN960, MS21045 e MS21919. São fracos para colares HL70/HL86/HL94, Hi-Lite, lockbolts (Huck) e NAS1919/1921, e a norma original exige ASSIST (login). Quase nenhuma família tem duas fontes abertas já conferidas.
9. Nenhum dos três CADs de referência faz instalação nativa de prendedores aeronáuticos em massa por pontos. O CATIA depende de add-ons (FASTJOINT/CENIT) ou de catálogos próprios do fabricante da aeronave. SolidWorks e Inventor cobrem o ISO/ANSI e deixam o aeronáutico para bibliotecas externas. Existe espaço para o Fenkra fazer isso de forma nativa.
10. Proposta central: Conjunto de prendedores > Conjunto da junta > pasta Prendedores, alimentada por pontos publicados com cor = diâmetro, normal detectada, preenchimento automático por equivalência de cor e modo manual (seções 2.C e 3).

**Principais lacunas (ver SUMMARY):** unidade polegada e família por grip; ponto/vetor publicado; furo aeronáutico com tolerância e escareado; estrutura de junta e substituição; dados aeronáuticos em duas fontes.

---

## 2. Fichas

### Parte A. Convenções gerais (valem para quase todas as famílias)

- **Prefixos [V]:**
  - `AN` é o padrão antigo Army-Navy.
  - `MS` é o padrão militar atual.
  - `NAS` é da indústria aeroespacial.
  - Hoje as especificações são emitidas como `NASM`, mas o part number mantém o prefixo MS.
  - Muitos MS substituem AN (tabelas de substituição existem).
- **Diâmetro:**
  - AN/MS: o número logo após a sigla é o diâmetro em **1/16 pol** (AN4 = 1/4") [V].
  - Rebites e cegos: dash em **1/32 pol** (MS20470AD4 = 1/8") [V].
  - Hi-Lok e CherryMAX: 1º dash em **1/32 pol** [V].
  - Porcas pequenas: dash -02/-04/-06/-08 são tamanhos de parafuso de máquina; de -3 em diante é 1/16 pol [V].
- **Comprimento ou grip:**
  - AN3–AN20: dash 1–7 = comprimento em 1/8 pol; 10 em diante = polegadas e oitavos (AN4-12 = 1-1/4") [V].
  - NAS6203–NAS6220: dash = **grip** em 1/16 pol, ±0,010 pol [V]. Exemplos: -6 → 0,375; -26 → 1,615–1,635; -30 → 1,865–1,885.
  - Hi-Lok: 2º dash = **grip máximo** em 1/16 pol [V].
  - Rebite sólido: 2º dash = comprimento em 1/16 pol, com meios (MS20470AD4-6.5 → 0,396–0,416 pol) [V].
- **Material e acabamento:**
  - AN: sem letra = aço liga cadmiado; C = CRES; DD = 2024-T4 [V].
  - Rebites: A (1100), AD (2117-T4), D (2017-T4), DD (2024-T4), B (5056), M/C (monel/inox) [V].
- **Furo no pino:** sufixo H = cabeça furada (arame); sem "A" = furo para cupilha; A = haste sem furo para porca autotravante [V].
- **Marcas de cabeça [V]:**
  - cruz/asterisco = aço liga;
  - traço = CRES;
  - dois traços = 2024-T4;
  - ponto = 2017-T4;
  - triângulo = tolerância justa;
  - M ou tinta laranja = partícula magnética.
- **Pol↔mm:** 1/32 = 0,794 mm; 1/16 = 1,5875 mm; 1 pol = 25,4 mm.
  - Diâmetros nominais em mm: dash 3 = 2,38; 4 = 3,18; 5 = 3,97; 6 = 4,76; 7 = 5,56; 8 = 6,35; 9 = 7,14; 10 = 7,94; 12 = 9,53; 14 = 11,11; 16 = 12,70; 20 = 15,88.
  - Esta é aritmética exata. O furo real é outro número (ver cada família).

**Regra para a biblioteca:** o campo `designacao` não pode mais ser `M<n>`. Ele precisa de um código por família (ex.: `-5`, `AN4`, `04`) mais campos `d_in`, `d_mm`, `grip_min`, `grip_max`, `furo_min`, `furo_max`. O `diametro_nominal()` hoje devolve `None` para qualquer coisa que não comece com `M` (`biblioteca.rs:81`), e `validar()` rejeita (`:139`). Esse é o primeiro bloqueio técnico.

---

### Parte B. Famílias de prendedores

Para cada uma: o que é, uso, convenção, tabela mínima e fontes. As tabelas de modelagem usam polegadas como na norma, com mm entre parênteses.

#### B1. Parafusos AN3–AN20 (CATIA: peça de catálogo/FASTJOINT / SW: Toolbox não traz AN / Inventor: Content Center não traz AN / Fenkra: ❌)

- **O que é:** parafuso de cabeça hexagonal, aço liga cadmiado, rosca fina UNF-3A, usado em estrutura secundária e primária leve (tração e cisalhamento).
- **Variantes [V]:**
  - AN21–AN36: parafusos de clevis (só cisalhamento).
  - AN42–AN49: olhais.
  - AN173–AN186 e NAS6200: tolerância justa.
- **Convenção [V]:**
  - AN4-7 = 1/4" de diâmetro, 7/8" de comprimento.
  - AN4-12 = 1-1/4".
  - Sufixos: H, A.
  - Material: letra C, DD.

| Parafuso | d (pol) | d (mm) | Rosca (UNF-3A) |
|---|---|---|---|
| AN3 | 0,190 | 4,83 | 10-32 [V] |
| AN4 | 0,250 | 6,35 | 1/4-28 [V] |
| AN5 | 0,3125 | 7,94 | 5/16-24 [V] |
| AN6 | 0,375 | 9,53 | 3/8-24 [C] |
| AN7 | 0,4375 | 11,11 | 7/16-20 [C] |
| AN8 | 0,500 | 12,70 | 1/2-20 [C] |
| AN10 | 0,625 | 15,88 | 5/8-18 [C] |
| AN12 | 0,750 | 19,05 | 3/4-16 [C] |

- **Cabeça:** sextavado com largura entre faces próxima de 3/8" (AN3), 7/16" (AN4), 1/2" (AN5), 9/16" (AN6). Altura ≈ 0,11–0,16" **[C]**. A listagem que vi traz um NAS6203 com H = 0,125" e W = 0,410" **[V~]**, que serve de ordem de grandeza.
- **Furo:** furo de passagem nominal = broca letra/número perto de d + 0,003–0,010 pol **[C]**, por exemplo #10 (0,1935 pol = 4,92 mm) para AN3. Para tolerância justa, ver B2.
- **Furo da cupilha:** ≈ 0,07" no AN3 **[V~]** (listagem de distribuidor).
- **Regra de rosca:** deve sobrar pelo menos 1 fio completo além da porca **[C]**.
- **Fontes gratuitas:** DLA FastenerCatalog (rosca AN3–5) [V]; FAA AC 43.13-1B cap. 7 (não consegui abrir o PDF) [C]; apostila de estudo AMT [V]; Aircraft Spruce, Univair e military-fasteners.com como cross-check.
- **Duas fontes?** Rosca de AN3–5: sim (DLA + listagem Boeing). O resto: não.

**Fenkra hoje:** `TipoFixador::ParafusoSextavado` existe, mas `Tamanho` só tem `s`, `k`, `P` e a designação é "M…". AN3 não cabe sem mudar o schema (ver RF-PRD-01).

#### B2. Parafusos de cisalhamento de tolerância justa: NAS6203–NAS6220, NAS464, NAS6603–NAS6620

- **O que é:** parafuso de cabeça sextavada, haste retificada com tolerância ±0,0005", rosca curta. É a base de juntas de cisalhamento de fuselagem e trem de pouso.
- **Faixa [V]:** os dados trazem NAS6203 até NAS6220.
  - O 2º dígito segue o diâmetro em 1/16: NAS6203 = 3/16", NAS6204 = 1/4" **[V~]**.
  - A folha de dados lista NAS6203–6220 em um só documento [V~].
- **Dash = grip em 1/16 pol [V]:** NAS6203-6 → grip 0,375±0,010. NAS6203-26 → 1,615–1,635. NAS6204-30 → 1,865–1,885.
- **Haste:** NAS6203-9 tem haste entre 0,1885 e 0,1895 pol (4,788–4,813 mm) [V].
- **Comprimento total:** NAS6203-6 tem L = 0,698±0,015, ou seja, L ≈ grip + 0,323 pol [V~, uma listagem].
- **Rosca:** NAS6203 = 0,19-32 (fino), NAS6204 = 1/4-28 [V~].
- **Sufixos [C]:** D (haste furada para cupilha), H (cabeça furada), A.
- **Prefixo L:** aparece em NAS6203L6 (distribuidor) e não é explicado nas páginas que li.
- **NAS464:** parafuso de cisalhamento de tolerância justa, 125 ksi (sufixo P) **[C]**.
- **NAS6603–6620:** série de alto desempenho com a mesma lógica; liga de titânio é o que lembro **[C, baixa confiança]**.
- **Furo:** rimado, nominal + 0,0005 a +0,0025 pol **[C]**. Pino justo exige furo com tolerância em centésimos de milímetro, não ±0,05 mm.
- **Fontes:** Boeing Distribution (cpd/NAS6203-9, -26, NAS6204-30 [V]); military-fasteners.com [V~]; folha de dados no Scribd [V~, texto sem valores]. **Duas fontes:** não conferidas.

**Fenkra hoje:** a pilha pede furo "normal/fino" por Ø (`casa_folga`, `:413`), o que não representa classe de ajuste. Falta o campo `ajuste` (justo/normal/interferência).

#### B3. Parafusos de ferramenta interna e 12 pontos: NAS1351/1352, MS20004–MS20024, MS21250, MS20073/74

- **MS20004–MS20024 / NAS144–NAS176 [V]:** parafusos de sextavado interno de alta resistência, usados com arruela **MS20002** (chanfrada).
- **NAS1351/1352:** parafusos de cabeça cilíndrica com sextavado interno, em par (1351 = UNC, 1352 = UNF) **[C]**.
- **MS21250:** parafuso de 12 pontos externos, alta resistência, rosca UNJF **[C]**. Part number tipo `MS21250-04012` (04 = 1/4", 012 = 12/16 pol) **[C]**.
- **MS20073/74:** hexagonal com cabeça furada, fina e grossa **[C]**.
- **Tabela mínima de modelagem:** reaproveitar `allen()` (`fixadores.rs:60`) com `dk`, `k`, `s` em polegada. Valores precisam vir de duas fontes; **não consegui nenhuma nesta pesquisa**.
- **Fontes:** FAA/AMT [V]; ASSIST (NAS144–176, NAS1351/1352, MS21250) para conferir. **Duas fontes:** não.

#### B4. Pinos Hi-Lok e colares (HL10, HL11, HL18, HL19, HL20; HL70, HL77, HL86, HL94)

- **O que é:** pino rosqueado com ranhura de torque (sextavado interno) e colar de quebra que se aperta até romper o sextavado externo. Fixação estrutural com ajuste de folga ou interferência leve, usada em asa e fuselagem.
- **Convenção [V]:** `HL18PB6-18` → 1º dash 6 = Ø3/16" (1/32 pol); 2º dash 18 = grip máximo 18/16 = 1,125".
  - O furo é preparado conforme NAS618 [V~].
  - Cada dash de grip cobre uma faixa de 1/16 pol: grip mínimo = (n−1)/16 **[C]**.
- **Famílias de pino:**
  - HL10 = cabeça protuberante de cisalhamento em titânio [V].
  - HL11 = cabeça rebaixada 100° em titânio [V].
  - HL18 = protuberante de cisalhamento em aço liga [V].
  - HL19 = rebaixada em aço **[C]**.
  - HL20 = família de tração **[C, baixa confiança]**.
  - HL326/HL329 = Hi-Tigue (rosca recuada, interferência) [V~].
- **Pinos:**

| Pino | Ø nominal | Rosca | Comprimento total | Carga de ensaio listada |
|---|---|---|---|---|
| HL18PB5-14 | 5/32" (3,97 mm) | 0,164-32 | 1,187" | 1940 lb [V] |
| HL18PB6-3 | 3/16" (4,76 mm) | 0,19-32 | 0,512" | 2500 lb [V] |
| HL18PB6-2 | 3/16" | 0,19-32 | 0,45" | — [V] |

- **Regra derivada [V]:** L ≈ grip máximo + 0,31 pol (Ø5/32) e + 0,325 pol (Ø3/16). Exemplos: 14/16 + 0,312 = 1,187 e 3/16 + 0,325 = 0,512.
- **Colar:**
  - HL70-5 combina com pino Ø5/32" (dash 5) [V~].
  - HL70TW5, HL86-6, HL70-10 existem nas listagens [V].
  - O HL70 tem rebaixo interno para pinos nominais e de sobremedida [V~].
  - O fórum diz que o HL70 é dimensionado para o pino de rosca longa HL10 [V~].
  - Altura e sextavado do colar: **não consegui valores**. Modelar com uma estimativa marcada.
  - HL77, HL86 (colar de alumínio, mais leve?) e HL94 (alta temperatura?) são **[C, baixa confiança]**. Conferir qual colar casa com qual pino é obrigatório.
- **Furo:** pino de folga: nominal + ~0 a +0,004 pol **[C]**; leia NAS618.
- **Fontes:** Boeing Distribution (HL18PB6-3 [V], HL18PB5-14 [V]); Textron Aviation parts (HL10V5-6, HL11VBJ4-10, HL18PB10-6 [V]); desenho HL13 no Scribd [V~]; Bombardier PPS 2.67 (apenas índice) [V~]; Delta (remoção) [V~]. **Duas fontes:** a regra de dash tem duas (Textron e Scribd); tabelas dimensionais: não.

**Fenkra hoje:** a pilha de hoje é parafuso + arruela + porca (`:243-254`). O Hi-Lok é pino + colar, sem arruela do lado do pino e sem porca. Hoje seria um `TipoFixador` novo (`PinoComColar`).

#### B5. Hi-Lite (HST10, HST11, HST12) e Hi-Tigue (HL32x)

- **O que é:** versão mais leve do Hi-Lok, em aço e titânio, para furos de ajuste leve. O pino é entregue com colar HST108x.
- **Evidência [V]:**
  - `HST10AG-6-5` é um pino protuberante de cisalhamento (1º dash 6 = 3/16"; 2º dash 5 = grip 5/16").
  - `HST1087-5` é um colar (rosca 0,164-32, arruela de aço 300 integrada) [V].
  - Colares tipo `HST79CK8` existem [V].
- **Uso:** comentários em fórum sugerem que o Hi-Lite serve para ajuste leve, mas não equivalem ao Hi-Tigue em fadiga [V~, opinião].
- **HST11/HST12:** HST11 = cabeça rebaixada **[C]**; HST12 não foi determinado.
- **Fontes:** Textron, NSN Sphere, eng-tips. **Duas fontes:** não.

#### B6. Lockbolts (Huck C6L, C50L; MS21140/21141; Alcoa/Huck aeronáuticos)

- **O que é:** pino com ranhuras (lock grooves) e colar que se agarra por swage (compressão radial). Resistente à vibração e sem torque.
- **Part number Huck C50L [V]:** `C50LR-BR16-4` = cabeça redonda, aço carbono, Ø1/2" (16/32), grip nº 4, com o colar encomendado à parte (tabela Tab-Lok no catálogo). C50L é classe 8.8, 12,7 mm [V]. C6L é estrutural até 9,5 mm [V~].
- **MS21140 / MS21141:** familia de fixador de travamento positivo e puxar (pull type): 21140 = rebaixado 100°, 21141 = protuberante, CRES com 95 ksi de cisalhamento [V~].
- **Padrões aeroespaciais de lockbolt (NAS1436…, MS90353…):** não achei confirmação **[C, baixa confiança]**.
- **Tabela de grip:** está na página 4 do folheto Howmet, que não pude extrair (PDF comprimido).
- **Fontes:** Howmet C50L brochure; Titgemeyer C50L; distribuidores Huck. **Duas fontes:** C50L: sim (nome/Ø), tabela de grip: não.

#### B7. Cegos estruturais (CherryMAX, Huck Blind Bolt/Accu-Lok, Composi-Lok, Jo-Bolt, Visu-Lok, NAS1919/1921, NAS1738/1739)

- **CherryMAX [V]:**
  - CR3213 = cabeça universal, bainha de liga com haste de aço.
  - CR3212 = série de ¼" **[V~]**; é cabeça rebaixada 100° **[C]**.
  - CR3242/CR3243 = família com furo maior que o CR3213.
- **Dash do CherryMAX [V]:** `CR3213-4-09` → Ø1/8" (4/32), grip nº 9 → 0,501–0,562 pol. Regra de faixa: (n−1)/16 + 0,001 até n/16 (exemplos: 03 → 0,126–0,187; 04 → 0,188–0,250; 09 → 0,501–0,562).
- **Furo [V]:**
  - CR3213-4: 0,129–0,132 pol; CR3213-5: 0,160–0,164 pol.
  - CR3243-4 (oversize): 0,143–0,146 pol.
  - CR3242-5: 0,176–0,180 pol; CR3242-6: 0,205–0,209 pol.
- **Comprimento [V~]:**
  - CR3213-5-10: L 1,9 pol, cabeça Ø0,312 e altura 0,08.
  - CR3242-6-7: L 1,76, cabeça Ø0,353.
  - CR3243-4-03: L 1,444.
  - NAS9301B4-01: L 1,304, grip 0,025–0,062.
- **Três peças e alta carga:**
  - Jo-Bolt e Visu-Lok são cegos estruturais de três peças (luva, porca de acionamento e miolo) para pilhas grossas e caixas fechadas [V~].
  - Huck "Accu-Lok" tem página no Howmet [V~].
  - Composi-Lok tem grande pegada no lado cego, para laminados de composto [V~, patente].
  - Visu-Lok é para metálicos [V~].
- **NAS1738/NAS1739 [V~]:** NAS1739E4-3 tem cabeça Ø0,221–0,229 e haste Ø0,139–0,143, com anel de travamento em liga de níquel. Um CR2539 atende NAS1738 para grip 0,313–0,375.
- **NAS1919/NAS1921:** não consegui confirmar o que são (Cherry Buck/Cherrylock?). **Não modelar até ver a norma no ASSIST.**
- **Fontes:** Boeing Distribution (CR3213, CR3242, CR3243, NAS9301, NAS1739 [V]); military-fasteners.com; Howmet; USPTO. **Duas fontes:** o grip do CherryMAX tem duas (Boeing + military-fasteners); o resto: não.

**Fenkra hoje:** não existe tipo "cego com grip e furo". Em compensação o conceito de pilha por grip é o mesmo.

#### B8. Rebites sólidos (MS20470, MS20426, NAS1097, MS20615, AN470/AN426)

- **MS20470 (cabeça universal) / AN470 [V]:** diâmetro em 1/32; comprimento em 1/16 (com meios).
- **MS20426 / AN426 [V]:** cabeça rebaixada 100°.
- **NAS1097 [C]:** rebite de cabeça rebaixada rasa ("flush shear") para chapa fina.
- **MS20615 [C]:** rebite sólido de monel/aço. Não confirmei.
- **Tabela MS20470 (cabeça universal), com furo recomendado:**

| Dash | D (pol / mm) | Broca | Furo (mm) | Cabeça Ø (pol) | Altura (pol) |
|---|---|---|---|---|---|
| -3 | 3/32 / 2,38 | #40 | 2,49 | 0,178–0,196 [V] | ≤ 0,050 [V] |
| -4 | 1/8 / 3,18 | #30 | 3,26 | 0,238–0,262 [V] | 0,054–0,064 [V] |
| -5 | 5/32 / 3,97 | #20 | 4,09 | 0,296–0,328 [V] | 0,067–0,077 [V] |
| -6 | 3/16 / 4,76 | #11 | 4,85 | 0,356–0,394 [V] | ≈ 0,085 [C] |
| -8 | 1/4 / 6,35 | F | 6,53 | ≈ 0,49 [C] | ≈ 0,12 [C] |

- **Regra paramétrica [derivada dos dados V]:** cabeça Ø ≈ 2,0·D e altura ≈ 0,47·D.
- **MS20426 (100°):**
  - MS20426AD4-5 aparece com 0,229, 0,042 e 100° [V~, rótulos faltando].
  - A conta confere: altura = (Ø cabeça − D)/2 · cot(50°) = (0,229 − 0,125)/2 · 0,839 = 0,044 ≈ 0,042.
  - Regra para a biblioteca: cabeça Ø ≈ 1,83·D e altura calculada por geometria.
  - Tabela completa ainda sem fonte.
- **Regras de aplicação [V]:**
  - Comprimento L = G + 1,5·D (G = grip total), arredondando para cima ao dash disponível.
  - Cabeça de oficina: altura 0,5·D e Ø 1,5·D.
  - Diâmetro D ≈ 3 × espessura da chapa mais grossa.
  - Distância à borda: universal mín. 2D, padrão 2,5D, máx. 4D; rebaixada mín. 2,5D, padrão 3D.
  - Passo: mín. 3D, padrão 4–6D, máx. 8D. Passo transversal: mín. 2,5D, padrão 3D, máx. 5D.
- **Fontes:** Olander, Boeing Distribution, McMaster (MS20426) e apostila open-exam-prep (regras). **Duas fontes:** MS20470 -3 a -5: sim (Olander e Boeing); regras de layout: apostila apenas.

#### B9. Rebites cegos de uso geral e mecânicos (MS20600/20601, NAS1398/1399, CR3213, NAS1738/1739)

- **MS20600/MS20601 [V~]:** rebite estrutural de puxar (self-plugging), cabeça protuberante e rebaixada. Foram substituídos por NASM20600/NASM20601.
- **NAS1398 / NAS1399 [V~]:** NAS1398 = protuberante, NAS1399 = rebaixado 100°, haste com travamento mecânico. Part numbers tipo `NAS1399CW3A2`, `NAS1398CW5A3`, `NAS1398B6A6`.
  - Nome: material da luva (B, D, C…), W, diâmetro em 1/32, A = tipo de haste, grip.
  - NAS1399CW3A4 tem haste Ø0,094 nominal [V]. A2 → grip 0,063–0,125 [V]. NAS1398CW5A3 → 0,251–0,312 [V].
  - **A regra do dígito de grip não é a mesma nos dois exemplos.** Não inferir; copiar a tabela da norma.
- **CR3213:** ver B7.
- **Fontes:** Boeing Distribution, military-fasteners.com, EverySpec. **Duas fontes:** não.

#### B10. Porcas (MS21042, MS21043, MS21044/21045/21046, NAS1291, AN363/364/365, AN310/320/315/316, AN340)

- **Famílias [V]:**
  - MS21042 = baixo perfil, todo em aço, cisalhamento, 450 °F.
  - MS21043 = baixo perfil, inox prateado, 800 °F (um fornecedor diz aço, divergência).
  - MS21045 = altura cheia, aço, tração; MS21046 = inox prateado, 800 °F.
  - AN363 = tração, 450 °F; AN364 = inserto não metálico de baixa altura (cisalhamento, 250 °F); AN365/MS21044 = altura cheia, inserto não metálico.
  - AN310 = castelo, altura cheia; AN320 = meia altura (cisalhamento); AN315 = hexagonal simples (trava com AN935 ou AN316); AN316 = contraporca; AN340/MS35649 = hexagonal simples, rosca grossa.
- **NAS1291:** não achei dado confiável **[C]**.
- **Dimensões [V]:**

| Rosca | MS21043 (altura / chave) | MS21045 (altura máx. / chave) |
|---|---|---|
| 6-32 | 0,141" / 3/16" | 0,188" / 5/16" |
| 8-32 | 0,188" / 7/32" | 0,250" / 11/32" |
| 10-32 | 0,188" / 1/4" | 0,250" / 3/8" |
| 1/4-28 | 0,219" / 5/16" | 0,328" / 7/16" |
| 5/16-24 | 0,266" / 3/8" | 0,360" / 1/2" |
| 3/8-24 | 0,282" / 7/16" | 0,469" / 9/16" |

- **AN365:** uma tabela de fornecedor dá 6-32 → 0,188" / 5/16"; 10-32 → 0,250" / 3/8"; 1/4-28 → 0,328" / 7/16" [V~].
- **AN310-5:** A = 1/2", B = 37/64", C = 21/64", D = 1/2" (letras sem definição na fonte) [V~].
- **Regra de rosca:** coarse nos #6/#8 e nas simples; fina de #10 em diante [V].
- **Fontes:** flywithspa.com [V]; Aircraft Spruce, Univair, EverySpec. **Duas fontes:** MS21043/MS21045: sim; os demais: não.

#### B11. Porcas âncora e nutplates (MS21047, MS21059, MS21060, NAS1068, K1000)

- **O que é:** porca em placa com duas abas, rebitada à estrutura. Fixa (MS21047, "two-lug"), flutuante (MS21059/MS21060, K1000).
- **Dados achados [V~]:**
  - MS21059-3 (10-32): furo central 7/32 pol, 2 furos de rebite Ø0,098 pol (#40), passo de rebite 11/16 pol (0,6875).
  - MS21059-4 (1/4-28): passo 1,000 pol (McMaster).
  - O Omega diz 0,688 pol para -06 e 1,000 pol para -5.
  - O Eng-Tips menciona máx. de ~0,688 pol para certos tamanhos.
  - **As fontes divergem.** Isso é exatamente o caso de `confirmado_por = 1` com nota.
- **NAS1068:** sem dados.
- **Uso no CAD:** colocar o conjunto (placa + 2 rebites + porca) como um prendedor composto, com orientação relativa à aba.
- **Fontes:** McMaster, Omega, airforums, Eng-Tips. **Duas fontes:** conflito, não confirmado.

#### B12. Arruelas (AN960, NAS1149, MS27183, MS20002, AN935, AN970, AN975)

- **AN960 [V]:** plana, espessura padrão 1/16"; versão fina "L" 1/32". Não empilhar mais de 3 sob uma porca.
- **AN960-10 [V]:** ID 0,193–0,213, OD 0,433–0,458, espessura 0,053–0,073 (nominal 5,16 × 11,13 × 1,6 mm).
- **AN960-6 [V~]:** ID 0,149, OD 0,375, espessura 0,032.
- **NAS1149 [V~]:** `NAS1149F0363P` aparece como equivalente do AN960-10. Decodificação **[C]**: F = aço, 03 = 3/16", 63 = 0,063" de espessura, P = passivado.
- **AN960 maiores [C]:**

| Arruela | ID | OD | Espessura |
|---|---|---|---|
| AN960-416 | 0,266 | 0,562 | 0,063 |
| AN960-516 | 0,328 | 0,688 | 0,063 |
| AN960-616 | 0,391 | 0,812 | 0,063 |

- **Outras [V]:** AN935 = trava de mola (proibida em juntas primárias); AN970 = grande área, ~3× o OD normal, para materiais moles; MS20002 = alta resistência chanfrada, para parafusos de ferramenta interna.
- **MS27183 [C]:** arruela plana de aço (série larga); conferir.
- **Fontes:** military-fasteners.com, Textron, Boeing Distribution, flywithspa. **Duas fontes:** AN960-10: sim.

#### B13. Insertos roscados (Heli-Coil MS21209/NAS1130, Keensert)

- **MS21209 [V~]:** inserto helicoidal.
  - Famílias C (grossa) e F (fina) no part number.
  - O código de comprimento é 10/15/20/25/30 = 1,0D/1,5D/2,0D/2,5D/3,0D (conferido no 4-40: `MS21209C0415L` → 0,168" = 1,5·0,112).
  - Dois estilos de código de tamanho: `04`, `06`, `08` (com zero) são tamanhos numerados; `8` sozinho é 8/16" = 1/2" (`MS21209F8-15L` → 0,750" = 1,5·0,5).
  - O sufixo "L" não foi explicado.
- **Instalação:** broca, macho de inserto, ferramenta de instalar e quebra-ponta [V~, McMaster]. A profundidade mínima de rosqueamento H por comprimento do inserto está na Misumi, mas os números não saíram na página.
- **NAS1130 e Keensert (MS51831 / NASM51831):** nenhum resultado confirmado **[C]**.
- **Regra para o Fenkra:** o inserto precisa de `broca_inserto` própria, diferente da broca de macho da rosca nominal.
- **Fontes:** Boeing Distribution, McMaster, Misumi, military-fasteners.com. **Duas fontes:** código de comprimento: sim.

#### B14. Pinos e cupilhas (MS16555, MS24665, NAS607, MS20392, clevis)

- **MS24665 [V]:** cupilha (cotter pin) para porcas castelo AN310/AN320 e pinos clevis.
- **MS20392 [C]:** pino de cabeça com furo de cupilha (clevis / headed). Part number tipo `MS20392-3C15` (Ø em 1/16 + posição do furo), a confirmar.
- **MS16555 e NAS607:** não consegui determinar a classe (pinos de cisalhamento? rápidos?) **[C, baixa confiança]**. Não modelar antes de ver a norma.
- **Fontes:** flywithspa/FAA [V] para MS24665; o resto sem fonte. **Duas fontes:** não.

#### B15. Presilhas e prendedores rápidos (Dzus, Camloc, Cleco)

- **Dzus [V~]:** quarto de volta, MIL-F-5591; o stud é preso ao painel e o receptáculo de mola fica na estrutura. A Southco lista "size 3.5" com cabeças e acabamentos variados.
- **Camloc (série 2600) [C]:** mesma lógica; sem fonte.
- **Dados mínimos:** diâmetro do stud, comprimento por espessura do painel, tipo de cabeça, Ø do furo no painel, receptáculo. **Não consegui obter** (a Southco não publica o receptáculo).
- **Fontes:** Southco. **Duas fontes:** não.

#### B16. Clamps de cabo e tubo (MS21919)

- **MS21919 [V~]:** clamp tipo laço com amortecedor (cushioned).
  - DG = laço com amortecedor; WDG = com cunha **[V~]**.
  - O número final = Ø do tubo em 1/16 pol (DG7 → 7/16; WDG15 → 15/16; WDG20 → 1-1/4"; WDG22 → 1-3/8") [V].
  - Furo de fixação: 0,199–0,209 pol (para #10 ou AN3) [V~]; 0,204 pol no WDG15 [V].
  - Largura da cinta 0,49–0,51 pol e espessura 0,05–0,063 pol [V~].
- **Regra de modelagem:** perfil de tira em C com furo e amortecedor; altura segue o Ø do tubo.
- **Fontes:** Aircraft Spruce, Boeing Distribution, military-fasteners. **Duas fontes:** Ø por dash: sim.

---

### Parte C. Comandos e funções (fichas completas)

#### C1. Biblioteca de prendedores aeronáuticos (CATIA: catálogo de prendedores / SW: Toolbox / Inventor: Content Center / outros: Creo e NX têm catálogos próprios [C] / Fenkra: 🟡 só ISO/DIN métricos)

- **O que faz:** guarda as famílias com suas tabelas e gera a peça paramétrica sob demanda.
- **CATIA V5 — no detalhe:**
  - A base do V5 não traz catálogo de prendedores aeronáuticos. Um artigo da SAE diz que a falta de padronização de objetos de prendedor aeronáutico no CATIA obriga fabricantes e fornecedores a manter métodos próprios [V~].
  - A solução comercial é o **FASTJOINT (CENIT)**: bancada própria que lê dimensões e propriedades de bibliotecas ou bancos de dados externos, gera dados consistentes no modelo V5 e trata grandes volumes em montagens grandes [V].
  - Quando a referência some, a peça-modelo fica órfã; o tratamento depende da OEM **[C]**.
- **SolidWorks — no detalhe:** o Toolbox cobre ISO, ANSI, DIN, JIS e PEM, entre outros [C]. Aeronáutico (AN/MS/NAS) não vem de fábrica **[C]**; usa-se TraceParts ou biblioteca própria. Cada fixador é uma configuração; mudar a configuração muda comprimento, diâmetro, tipo de rosca [V, texto de contexto do usuário].
- **Inventor — no detalhe:** o Content Center guarda famílias de bolts, washers, nuts e sheet metal nuts [V]. Peças personalizadas entram publicando uma peça ou iPart numa biblioteca de leitura/escrita do projeto; elas aparecem filtradas por rosca e diâmetro [V].
- **Outros:** Creo, NX e Solid Edge têm catálogos de prendedores com o conceito de "intent" **[C]**; Onshape tem componentes padrão **[C]**. Nenhum traz AN/MS/NAS pronto.
- **Fenkra hoje:**
  - As seis famílias JSON são embutidas por `include_str!` (`biblioteca.rs:215`).
  - `Familia::validar` (`:115`) exige designação `M…` (`:139`), valores positivos e série de comprimentos (`:159-170`).
  - `comprimentos_de` limita L a 2d–10d com mínimo 6 mm (`:110`). O AN3-3 (3/8" = 9,5 mm) passa; um rebite não.
  - `fixador_documento` (`solidos/fixadores.rs:26`) recusa L fora de 2d–10d (`:40`).
  - Só há geometria para 4 tipos (`:50-55`).
- **Melhor prática para o Fenkra:**
  1. Suporte → norma → família → tamanho (dash) → grip/comprimento → variantes.
  2. Cada família declara `unidade` (mm/pol), `modelo_de_comprimento` (L ou grip) e `campos` próprios.
  3. Cada tamanho tem faixa de grip (`grip_min`, `grip_max`), `furo_min`, `furo_max`, e tipo de cabeça (protuberante, 100° rebaixada).
  4. Cada valor leva `confirmado_por` e `fontes[]`; abaixo de 2, aparece o selo "uma fonte" na interface (já existe a regra em `biblioteca.rs:394`).
- **Casos de teste:**
  - carregar toda família e validar;
  - gerar todos os tamanhos (teste existente `todos_os_tamanhos_de_todas_as_familias_geram`, `solidos/fixadores.rs:272`);
  - para cada dash, grip_max − grip_min = 1/16 pol;
  - conversão pol↔mm;
  - designações duplicadas rejeitadas.

#### C2. Pontos/vetores publicados com cor = diâmetro (regra proposta) (CATIA: Geometrical Set com pontos e eixos publicados / SW: pontos de croqui e Hole Series / Inventor: "On point" do Bolted Connection / Fenkra: ❌)

- **O que faz:** marca no modelo onde vai cada prendedor, sem criar o prendedor ainda.
- **CATIA V5 — no detalhe:**
  - **Publicações [V]:** `Tools > Publication` publica pontos, linhas, planos, curvas, croquis, corpos, features e sub-elementos (faces, arestas, vértices). Com a opção certa, só elementos publicados servem como referência externa. A opção de renomear tem três modos: Never, Always, Ask. Elementos com o mesmo nome religam-se sozinhos ao substituir um componente.
  - **Define In Work Object** [V~, tutorial 3DS]: botão direito no corpo → torna o objeto em trabalho, sublinhado na árvore. Os pontos novos entram no Conjunto Geométrico em trabalho.
  - **Prática aeronáutica [C]:** o engenheiro cria um *Geometrical Set* de pontos e vetores (um ponto na superfície e um eixo normal) e publica. O módulo de prendedores (FASTJOINT ou catálogo OEM) lê esses pontos. A cor do ponto no CATIA é só atributo gráfico, o que a OEM usa para separar diâmetros.
  - **API DNB/DELMIA [V]:** `create_point_fastener(nome, grupo-pai, lista de peças unidas, tipo de processo, contexto, posição de projeto)`, com posição de 6 valores XYZ + YPR em graus. `create_fastener_group` com peças unidas cria uma junta de montagem. `reset_position(0)` volta à posição de projeto e `(1)` à posição de manufatura salva. O fastener sabe quantas peças junta, suas peças, processos atribuídos e atributos de usuário.
  - **Atualização:** o ponto é uma feature; se a superfície-suporte é refeita, o ponto fica com erro no Update e a referência deve ser substituída **[C]**.
- **SolidWorks — no detalhe:**
  - Não há "ponto de prendedor" nativo. O recurso mais próximo é o **Smart Fasteners**, que lê features do Hole Wizard e do Toolbox [V]. Furos que não são do Hole Wizard recebem o tipo padrão configurado no Toolbox [V]. O **Hole Series** (assistente de furo em série) cobre furos entre peças **[C]**.
  - Pontos de croqui podem posicionar furos do Hole Wizard (posição = ponto).
- **Inventor — no detalhe:**
  - **On point** usa pontos de trabalho de um croqui de **peça**, não de montagem; Ctrl adiciona ou remove pontos; as conexões **seguem** o movimento dos pontos [V].
  - **By hole** com **Follow Pattern** põe uma conexão em cada furo de um padrão de vários centros [V].
- **Outros:** Creo e NX usam pontos de referência ("datum points") para instalar fixadores em massa **[C]**.
- **Fenkra hoje:** não existe. A peça publica referências, parâmetros e corpos; a montagem não tem ponto, eixo, plano nem publicação (catálogo MON-17). Existe `AsmFeatKind::Hole { p, d, dia, depth }` (`montagem/lib.rs:278`) para furo entre peças, com ponto e direção no mundo. O app consegue ler ponto e normal de uma face clicada (`asm_ui.rs:643-644`, `Pick::Face { point, normal, .. }`).
- **Melhor prática para o Fenkra (regra proposta):**
  1. **Objeto:** `PontoPrendedor` = ponto 3D + vetor normal unitário (sentido "de fora para dentro") + chave de diâmetro + referência da face de apoio (chave dono+papel do ADR 0010). Ponto e vetor são uma única entidade publicável; o vetor é desenhado como seta.
  2. **Cor = diâmetro.** A cor vem de uma tabela global editável `diâmetro → cor`, indexada por dash em 1/32 pol (série pol) ou por mm (série métrica). A chave é sempre o **diâmetro nominal**, nunca o part number. O ponto mostra a cor e o rótulo "-5" ou "M5" (acessível a daltonismo).
  3. **Normal detectada:** ao clicar na superfície (face plana ou curva), tomar o ponto e a normal da face naquele ponto. Se o clique é em curva dupla (pele de asa), a normal é a normal local da superfície. O usuário pode inverter (lado da cabeça).
  4. **Tolerância:** o ponto fora da superfície (> 0,05 mm) é projetado para ela; se a projeção falha, o ponto fica "quebrado" (`->x`) e vai para o diagnóstico (ADR 0011, princípio 1).
  5. **Publicação:** o conjunto de pontos é publicado com nome (ex.: `PTS_HL10_5`), e a substituição de uma peça religa pelo nome (como o CATIA).
  6. **Contexto único ativo:** os pontos novos entram no "Conjunto de prendedores" que está em trabalho (princípio 2 do ADR 0011).
- **Casos de teste:** ponto sobre face plana → normal igual à da face; ponto sobre cilindro → normal radial; ponto fora da face → projetado e avisado; substituir a peça mantendo o nome publicado → pontos religam; face removida → ponto "quebrado" com diagnóstico; inverter vetor; cor muda quando o diâmetro muda.

#### C3. Preenchimento automático de furos e pontos (CATIA: FASTJOINT/DNB / SW: Smart Fasteners / Inventor: Bolted Connection Generator / outros: Creo/NX/Solid Edge [C] / Fenkra: 🟡 só furos métricos)

- **O que faz:** instala o prendedor correto em todos os furos ou pontos escolhidos.
- **CATIA V5 — no detalhe:** o FASTJOINT lê os dados do prendedor de uma biblioteca ou banco de dados e gera o modelo [V]. A inserção é por ponto de projeto com 6 graus (XYZ + YPR) [V, DNB]. Grupos de prendedores e junta de montagem agrupam [V]. O prendedor é um objeto leve, não uma peça modelada à mão **[C]**.
- **SolidWorks — no detalhe [V, Hawk Ridge/CATI]:**
  - **Fluxo:** Assembly → Smart Fasteners → selecionar faces, componentes ou features do Hole Wizard → revisar a lista e remover os indesejados → **Add** ou **Populate All** (preenche o conjunto inteiro). Os prendedores entram dimensionados e com mates, agrupados por conjunto de furos.
  - **Orientação:** hardware entra e é mateado à face onde nasceu o croqui original do Hole Wizard.
  - **Furos que não são do Hole Wizard:** começam com o prendedor padrão do Toolbox.
  - **Pilhas:** *Add to Top Stack* (arruelas) e *Add to Bottom Stack* (arruelas e porcas), valem para todo o grupo; furo cego só aceita a pilha superior.
  - **Mates:** concêntrico ao furo e coincidente com a superfície de apoio [V].
  - **Pasta:** os componentes caem numa pasta Smart Fastener na árvore [V].
  - **Padrões:** todos os prendedores do padrão têm o mesmo tipo e tamanho, mas cada um pode ter outra configuração (por exemplo, comprimento maior) [V].
  - **Opção de arruela:** "Exact match" restringe as arruelas às de tamanho exato [V~].
  - **Edição:** botão direito → *Edit Smart Fastener* para mudar tipo, tamanho e comprimento, ou inverter [V~].
  - **Limites:** só furos reconhecíveis (Hole Wizard ou cortes cilíndricos separados) [contexto do usuário]; nada em corpos importados sem feature de furo.
- **Inventor — no detalhe [V]:**
  - **Pré-requisitos:** a montagem tem ao menos um componente, e há conexão com o Content Center.
  - **Tipo:** *Through All* (atravessa tudo) ou *Blind Connection* (só o plano de início).
  - **Posicionamento:** *Linear* (duas arestas), *Concentric* (aresta circular, serve também para cortes de extrusão/revolução), *On Point* (pontos de croqui de peça, seguem o croqui), *By Hole* (limita o Ø máximo do parafuso ao furo; *Follow Pattern* preenche o padrão e substitui conexões já postas).
  - **Planos:** plano de início (define a direção) e plano de término; os dois limitam a área onde os furos são calculados.
  - **Rosca:** tipo e diâmetro vêm da planilha Threads.xls.
  - **Prendedores:** a lista é filtrada por posicionamento e rosca. A prévia mostra uma grip 3D que arrasta o comprimento do parafuso; duplo clique abre a lista de comprimentos válidos.
  - **Biblioteca de modelos:** a conexão pode ser salva como template; os templates são ordenáveis.
  - **Edição:** *Edit Using Design Accelerator*; excluir o parafuso apaga as peças da conexão. Com a opção de submontagem ligada, o solver é automático; desligada (padrão), só manual [V].
  - **Mensagens:** o painel resume cálculos de estado e fadiga [V~].
- **Outros:** nenhum dos três insere rebite ou prendedor aeronáutico em massa de forma nativa e simples (catálogo BIB-10).
- **Fenkra hoje:**
  - `preencher_furos` (`:106`) lê `hole_table` (`solidos/lib.rs:5071`), que devolve `HoleRow { pos, dir, diameter, depth, through, thread }`: sem escareado, sem tolerância, sem dono da feature de furo.
  - Pilhas: furos colineares (0,05 mm / 0,5°) de componentes diferentes (`:167-173`, `colineares` em `:403`). A comparação usa sempre `pilha[0]`, então a ordem de chegada pode partir uma pilha em duas se houver deriva.
  - Pega: mede a espessura por raios paralelos ao eixo, tomando a mediana (`espessura_ao_longo`, `:432`), o que o próprio catálogo marca como frágil (MON-16: "pega medida por sondas laterais").
  - Seleção de tamanho: `casa_folga` e `casa_outra_folga` (`:413-420`) usam só `hardware::METRIC` (`hardware.rs:18`, M3–M20). Roscas "M…" via `tamanho_da_rosca` (`:423`), que **ignora o passo**: "M8x1" e "M8x1.25" viram o mesmo M8.
  - Comprimento: `pega + w1 + w2 + m + 2·P`, arredonda para cima na série (`:267-268`); se a série acaba, avisa mas **usa o último** (`:271-273`) em vez de falhar. Em furo roscado, arredonda para baixo (`:259`).
  - Restrições: eixo (Coincident) e contato, sem travar a rotação (B62 aberto).
  - Evita duplicar: pula pilhas cujo eixo já tem restrição de componente `PartType::Hardware` (`:210`).
  - Estrutura: o grupo "Fixadores" é plano (`:177`, `:281`); as peças vão para `<pasta>/fixadores/` (`arquivo_fixador`, `:363`).
  - Testes existentes: 5 (pilha, placa virada, sem arruela/porca, roscado cego, aviso de tamanho).
  - **Bugs prováveis:**
    - furo #10 aeronáutico nunca casa (4,9 mm vs 4,8/5,3 mm);
    - `tamanho_da_rosca` ignora passo;
    - com só uma peça no `componentes`, o parafuso pode virar "roscado" e perder a porca;
    - a série de comprimentos termina silenciosamente.
- **Melhor prática para o Fenkra:**
  1. **Suporte:** pontos publicados (C2), furos reconhecidos, ou ambos; escopo = quais peças formam a junta.
  2. **Modo:** *Automático por cor* (cada cor → prendedor do conjunto de regras), *Por furo*, *Manual*.
  3. **Parâmetros:** família, grip calculado, arruelas, material/acabamento, sentido (lado da cabeça).
  4. **Avançado:** tolerância de furo, criar o furo nas peças (sim/não), dimple/escareado, tolerância de interferência.
  5. Grip por **face**: medir a distância entre as faces extremas da pilha ao longo do vetor, não por sondas.
  6. Falhar alto: sem dash que cubra o grip → o ponto fica em "falhou" com diagnóstico "grip 7,9 mm fora de -4 a -6 (CR3213-4)"; nunca usar o último da série em silêncio.
- **Casos de teste:** grip exato de fronteira (n/16); grip ligeiramente acima; pilha de três peças; ponto em face inclinada; pilha com peça virada; furo #10 reconhecido; dois pontos na mesma coluna; reexecutar não duplica; GDL = 0 em cada prendedor (B62).

#### C4. Pilha de hardware e cálculo de grip (SW: hardware stacks / Inventor: lista de componentes da conexão / CATIA: FASTJOINT / Fenkra: 🟡)

- **O que faz:** monta a sequência cabeça → arruela → peças → arruela → porca/colar e calcula o comprimento.
- **CATIA V5 — no detalhe:** a sequência é definida no objeto de prendedor do catálogo (peça composta) **[C]**.
- **SolidWorks — no detalhe [V]:** pilha superior e inferior aplicam-se ao grupo todo; cada componente tem mate concêntrico com o parafuso e coincidente com a superfície; a ordem é a de seleção do usuário (ex.: arruela plana, arruela dentada, porca).
- **Inventor — no detalhe [V]:** a conexão é uma lista ordenada de componentes na direita do diálogo, com editar/excluir/trocar conteúdo; a grip 3D ajusta o comprimento.
- **Fenkra hoje:** `:243-254` calcula `w1`, `w2`, `m`; a pilha tem no máximo uma arruela de cada lado e uma porca. Não há pilha editável.
- **Melhor prática:**
  1. Regra aeronáutica para prendedor de rosca: pelo menos 1 fio completo além da porca **[C]**; no máximo 3 arruelas sob a porca [V].
  2. Para Hi-Lok: o pino cobre o grip e sobra a rosca para o colar; usar a regra derivada L ≈ grip_max + 0,31–0,325 pol [V~] como validação.
  3. Para rebite: L = G + 1,5D [V].
  4. Cada componente da pilha guarda "papel" (cabeça, arruela-cabeça, arruela-porca, porca/colar), para permitir trocar um deles sem refazer a pilha.
- **Casos de teste:** grip com chanfro de escareado (descontar profundidade da cabeça rebaixada); Hi-Lok com arruela sob o colar; troca de porca MS21042 → MS21045 mantém o eixo.

#### C5. Preparação do furo para prendedor aeronáutico (CATIA: Hole/furo de projeto / SW: Hole Wizard / Inventor: Hole / Fenkra: 🟡 `Hole` + `HoleKind`)

- **O que faz:** cria o furo com Ø, tolerância, escareado e rebaixo corretos para o prendedor.
- **CATIA / SW / Inventor:**
  - o CATIA tem o *Hole* do Part Design com escareado e rebaixo [C];
  - o SW Hole Wizard lê dados do Toolbox (diâmetro, folga, rosca, escareado) [V~];
  - o Inventor Hole tem escareado/rebaixo [C]. O Bolted Connection cria os furos nas peças da pilha [V].
- **Fenkra hoje:** `HoleKind` tem `Simples`, `Rebaixado`, `Escareado { diameter, angle }`, `Contrafurado` (`core/doc.rs:2543`). `Hole.standard: Option<String>` existe. A tolerância não é guardada, e o `hole_table` não devolve o tipo nem o escareado. O backlog B48 já cobre o ângulo do escareado (82/90/100/120°).
- **Melhor prática:**
  1. O prendedor da biblioteca entrega `furo` = {Ø nominal, tolerância (mín/máx), tipo de cabeça, ângulo, profundidade de escareado}.
  2. O furo recebe a classe de ajuste (folga / justo / interferência leve).
  3. Para rebite rebaixado (100°): a regra de escareado vs. dimple depende da espessura da chapa. A apostila diz que a rebaixada exige borda maior (2,5D) por causa do corte [V].
  4. Para tolerância justa: Ø do furo = faixa do parafuso + 0,0005–0,0025 pol [C]; gravar como par mín/máx.
- **Casos de teste:** Ø e ângulo de escareado conferem com a cabeça; furo reconhecido pelo preenchimento tem o mesmo prendedor que o ponto; tolerância preservada ao salvar/abrir.

#### C6. Estrutura na árvore e junta (CATIA: Geometrical Set / Product / SW: pasta Smart Fastener / Inventor: grupo de conexão e submontagem / Fenkra: 🟡 grupo "Fixadores" plano)

- **O que faz:** organiza prendedores por junta para editar, contar e substituir em bloco.
- **CATIA:** Conjuntos Geométricos (ordenados e não ordenados) agrupam pontos e eixos; o DNB tem `create_fastener_group`, que, com peças unidas, cria uma junta [V].
- **SolidWorks:** cria uma pasta "Smart Fastener" na árvore com o conjunto de prendedores [V]; os grupos podem ser reagrupados [V~].
- **Inventor:** a conexão fica solta na raiz da montagem ou em submontagem (opção); a submontagem permite solver automático [V].
- **Fenkra hoje:** `asm.add_group("Fixadores", None)` único (`:177`, `:281`); sem subgrupo por junta nem referência de volta ao ponto.
- **Melhor prática (estrutura proposta):**

```
Conjunto de prendedores          ← conjunto geométrico (pontos e vetores publicados)
 └─ Conjunto da junta  "J-001 Revestimento × Cavername"
     ├─ Pontos (cada um com cor = diâmetro e vetor)
     └─ pasta Prendedores        ← componentes instanciados
         ├─ HL18PB6-4 + HL70-6 …
         └─ MS20470AD4-5 …
```

  - Cada componente guarda o `id` do ponto de origem.
  - Excluir o ponto remove o prendedor (com confirmação).
  - Cada junta lista as peças que une.
  - A pasta Prendedores é a que entra na lista de materiais.
- **Casos de teste:** mover um ponto move o prendedor; apagar a junta apaga seus prendedores; salvar/abrir preserva as ligações; BOM agrupa por part number.

#### C7. Edição, substituição e diagnóstico (CATIA: Update Diagnosis / SW: Edit Smart Fastener / Inventor: Edit Using Design Accelerator + Design Doctor / Fenkra: ❌)

- **O que faz:** troca família, dash ou arruelas depois da inserção e explica o que falhou.
- **CATIA:** substituir um componente religa pelo nome publicado [V]; Update Diagnosis lista erros **[C]**.
- **SolidWorks:** *Edit Smart Fastener* troca tipo, tamanho e comprimento, e inverte [V~]; grupos podem ser editados [V~].
- **Inventor:** *Edit Using Design Accelerator*: adicionar/remover componentes e pontos de padrão; ícones para trocar conteúdo, editar, excluir [V].
- **Fenkra hoje:** nenhuma. MON-16 diz "sem editar/trocar a pilha depois".
- **Melhor prática:**
  1. Assistente de edição em tabela: uma linha por ponto (cor, ponto, família atual, dash, grip calculado, estado).
  2. Ações: *Trocar por equivalente* (lista de prendedores compatíveis com a cor), *Recalcular grip*, *Isolar*, *Desativar*, *Excluir*.
  3. Estados com os ícones do ADR 0011: `->` ok, `->?` fora de contexto, `->*` travado, `->x` quebrado.
  4. Diagnóstico por causa: ponto sem face, grip fora de faixa, Ø de furo incompatível, prendedor ausente da biblioteca, colisão.
- **Casos de teste:** trocar HL18 por HL10 mantém ponto e furo; grip mudou por espessura nova → estado "grip fora da faixa" e botão recalcular; excluir peça da junta → pontos órfãos listados.

#### C8. Modo manual (SW: Hole Wizard + inserir componente / Inventor: iMates e inserir da biblioteca / CATIA: insert existing component / Fenkra: parcial)

- **O que faz:** instala um prendedor num ponto escolhido sem regra automática.
- **Melhor prática:**
  1. Clicar no ponto (ou criar na hora: C2), escolher família e dash na lista, ver prévia.
  2. Todos os campos calculados podem ser sobrescritos, e o campo mostra que foi alterado à mão.
  3. O prendedor manual entra na mesma estrutura de junta (C6).
- **Casos de teste:** prendedor manual não é substituído pelo preenchimento automático; o botão "voltar ao automático" restaura o valor.

#### C9. Rebitagem por padrão de pontos (CATIA: padrão de pontos em Geometrical Set / SW: padrão linear de croqui / Inventor: Follow Pattern / Fenkra: ❌)

- **O que faz:** gera linhas de rebites com passo, borda e fileiras.
- **Regras [V]:** passo 3D–8D (padrão 4–6D); borda 2D–4D (universal) e 2,5D–4D (rebaixada); fileiras 2,5D–5D (padrão 3D).
- **Melhor prática:**
  1. Selecionar uma curva ou borda na peça, definir passo, borda e fileiras.
  2. Mostrar violações das regras em vermelho antes de gerar os pontos.
  3. O rebite gerado é um ponto C2 normal, com a cor do seu D.
- **Casos de teste:** passo menor que 3D → aviso; distância à borda menor que 2D → aviso; padrão sobre curva fechada fecha sem ponto duplicado.

#### C10. Lista e relatório de prendedores (CATIA: BOM/FASTJOINT / SW: lista de materiais / Inventor: BOM / Fenkra: ✅ AsmBom, sem dados aeronáuticos)

- **O que faz:** conta prendedores por part number, diâmetro e junta.
- **Melhor prática:** BOM agrupa por part number + acabamento; relatório de furação lista Ø, tolerância, quantidade por dash; exporta CSV.
- **Casos de teste:** soma bate com o número de pontos; renomear a família atualiza o BOM.

---

## 3. Requisitos levantados

Prioridades: P0 = bloqueia o tema; P1 = essencial; P2 = importante; P3 = desejável.

### Funcionais (RF-PRD)

| ID | Requisito | Pri. | Critério de aceite |
|---|---|---|---|
| RF-PRD-01 | A biblioteca aceita famílias em polegada, com designação por dash (`-5`, `AN4`, `04`), `d_in`, `d_mm` e faixa de grip. | P0 | Carregar uma família `HL18` com `designacao: "-6"`; `validar()` passa; `diametro_nominal()` devolve 4,76 mm. |
| RF-PRD-02 | Cada tamanho guarda `grip_min`, `grip_max`, `furo_min`, `furo_max` e tipo de cabeça. | P0 | O teste percorre todas as famílias e verifica `grip_max − grip_min = 1/16 pol`. |
| RF-PRD-03 | Cada valor leva `confirmado_por` e lista de fontes; tamanho com 1 fonte exibe selo e nota. | P1 | Teste existente (`biblioteca.rs:394`) estendido às novas famílias. |
| RF-PRD-04 | Famílias mínimas: AN3–AN20, NAS6203–6220, rebite MS20470 e MS20426, arruela AN960, porcas MS21042/43/45, MS21919, Hi-Lok (pino + colar), cegos CR3213/CR3242. | P1 | `familias().len() ≥ 14`; todas geram o sólido. |
| RF-PRD-05 | Famílias avançadas: Hi-Lite, lockbolts, nutplates, helicoil, Dzus/Camloc, cupilha. | P2 | Cada família com 2 fontes ou nota explícita de fonte única. |
| RF-PRD-06 | Tipo `PinoComColar` (Hi-Lok, Hi-Lite, lockbolt): pino + colar + arruela opcional. | P1 | `fixador_documento("hl18","-6", grip 18)` gera 2 peças com Z do pino e do colar coerentes. |
| RF-PRD-07 | Tipo `Rebite` (sólido e cego) com comprimento por grip: L = G + 1,5D, arredondado ao dash. | P1 | G = 0,080 pol e D = 1/8 → L ≥ 0,2675 → -5 (5/16). |
| RF-PRD-08 | Ponto publicado = ponto + vetor normal + chave de diâmetro + face de apoio. | P0 | Criar, salvar, abrir: os campos preservam. |
| RF-PRD-09 | A cor do ponto vem de uma tabela editável diâmetro → cor, com rótulo textual. | P1 | Mudar o Ø muda a cor; o rótulo continua legível sem cor. |
| RF-PRD-10 | A normal é lida da superfície no clique; o usuário pode inverter. | P0 | Clique em face plana → normal igual; em cilindro → radial; inverter troca o sinal. |
| RF-PRD-11 | Ponto fora da superfície é projetado (tolerância 0,05 mm) ou marcado quebrado. | P1 | Ponto a 0,5 mm da face → projetado e avisado; sem face → `->x`. |
| RF-PRD-12 | Estrutura: Conjunto de prendedores > Conjunto da junta > pasta Prendedores. | P0 | Árvore criada; cada prendedor referencia seu ponto. |
| RF-PRD-13 | Preencher por cor: cada cor mapeia para um prendedor (regra editável por junta). | P0 | Dois pontos da mesma cor recebem o mesmo prendedor; outra cor, outro. |
| RF-PRD-14 | O grip é medido pelas faces extremas da pilha ao longo do vetor. | P0 | Pilha de 2 chapas de 2 mm + 3 mm → grip = 5 mm ± 0,01. |
| RF-PRD-15 | Escolha do dash pela faixa de grip; fora da faixa vira erro visível, nunca o último da série. | P0 | Grip sem dash → estado "falhou" com texto de diagnóstico. |
| RF-PRD-16 | Furo criado nas peças com Ø/tolerância/escareado do prendedor (opcional). | P1 | O furo gerado fica dentro de `furo_min..furo_max`. |
| RF-PRD-17 | Reconhecer furos aeronáuticos existentes por Ø dentro da faixa do prendedor. | P1 | Furo de 4,92 mm → candidato a #10/3-16 e não a "sem tamanho". |
| RF-PRD-18 | Travar a rotação do prendedor (GDL = 0), com opção de livre. | P0 | Solver com GDL = 0 em cada prendedor inserido (B62). |
| RF-PRD-19 | Modo manual por ponto com campos sobrescrevíveis. | P1 | O manual não é alterado pelo preenchimento automático. |
| RF-PRD-20 | Assistente de edição/substituição em tabela, com equivalentes por cor. | P1 | Trocar a família de 10 prendedores em uma operação; desfazer volta ao estado anterior. |
| RF-PRD-21 | Diagnóstico por ponto (sem face, grip fora, Ø incompatível, ausente da biblioteca). | P1 | Cada causa tem texto próprio e ação (Editar/Substituir/Isolar/Desativar). |
| RF-PRD-22 | Mover o ponto move o prendedor; apagar o ponto remove o prendedor (com confirmação). | P1 | Teste de mover e apagar. |
| RF-PRD-23 | Padrão de rebites por curva com passo/borda/fileiras e validação de regras. | P2 | Passo < 3D → aviso. |
| RF-PRD-24 | Equivalência pol↔mm explícita entre séries; nunca trocar de série em silêncio. | P1 | Pedido #10 em montagem métrica → proposta M5 com aviso de rosca diferente. |
| RF-PRD-25 | Pilha editável (arruelas, colar, porca) com papéis. | P2 | Trocar a porca MS21042 por MS21045 mantém eixo e contato. |
| RF-PRD-26 | BOM por part number/acabamento e relatório de furação (Ø, tolerância, quantidade). | P2 | A soma do BOM = número de pontos. |
| RF-PRD-27 | Substituir um componente religa os pontos pelo nome publicado. | P1 | Trocar a peça por outra revisão: os pontos continuam ligados. |
| RF-PRD-28 | Furos coaxiais de tamanhos diferentes: aviso. | P2 | Teste existente (`:202-205`) mantido. |
| RF-PRD-29 | Nutplate como prendedor composto (placa + 2 rebites + porca) com furos e orientação. | P2 | O conjunto entra com 3 furos nas peças. |
| RF-PRD-30 | Importar uma lista de pontos (CSV: x, y, z, nx, ny, nz, dash). | P3 | Importar 100 linhas cria 100 pontos coloridos. |

### Não funcionais (RNF-PRD)

| ID | Requisito | Pri. | Critério de aceite |
|---|---|---|---|
| RNF-PRD-01 | Preencher 500 pontos em ≤ 10 s numa montagem de 20 peças. | P1 | Benchmark no corpus (B53). |
| RNF-PRD-02 | Reexecutar o preenchimento é idempotente. | P0 | Segunda execução: 0 novos, N pulados. |
| RNF-PRD-03 | Os dados da biblioteca citam a fonte e a licença (CC BY, como as famílias atuais). | P1 | `validar()` rejeita família sem fonte. |
| RNF-PRD-04 | Nenhum valor [C] entra na biblioteca como confirmado; `confirmado_por` ≥ 2 só com duas fontes abertas. | P1 | Revisão de dados antes do merge. |
| RNF-PRD-05 | Interface em português; rótulos pol/mm visíveis lado a lado. | P2 | Captura visual do painel. |
| RNF-PRD-06 | Cores acessíveis (daltonismo): rótulo textual sempre presente. | P2 | Captura em simulação de deuteranopia. |
| RNF-PRD-07 | Salvar/abrir preserva pontos, vetores, cores, junta e ligações. | P0 | Teste de ida e volta. |
| RNF-PRD-08 | Falhas aparecem como estado, nunca como substituição silenciosa (ADR 0011, princípio 1). | P0 | Nenhum caminho de código usa "o último da série" sem erro. |
| RNF-PRD-09 | Cada família tem peça de teste no corpus. | P1 | Teste "todos os tamanhos geram" cobre as novas famílias. |
| RNF-PRD-10 | Peças dos prendedores ficam em `fixadores/` e são reaproveitadas por nome. | P2 | Dois pontos iguais compartilham o mesmo arquivo. |

---

## 4. Itens de backlog propostos

| Título | O que muda | Pri. | Tam. | Dependências |
|---|---|---|---|---|
| Schema de biblioteca em polegada e dash | `designacao` livre, `d_in/d_mm`, `grip_min/max`, `furo_min/max`, tipo de cabeça, `unidade` (RF-PRD-01/02) | P0 | M | B63 |
| Tipos `PinoComColar` e `Rebite` | Geometria do pino, colar, rebite sólido e cego; L por grip (RF-PRD-06/07) | P1 | G | schema acima |
| Dados AN/NAS/MS em duas fontes | Carga das famílias mínimas de RF-PRD-04, com `confirmado_por` real; extração manual do ASSIST pelo usuário | P1 | G | B43, B63 |
| Dados avançados (Hi-Lite, lockbolt, nutplate, helicoil, Dzus) | RF-PRD-05 e RF-PRD-29 | P2 | G | dados mínimos |
| Ponto/vetor publicado com cor | Objeto `PontoPrendedor`, tabela de cores, normal da superfície (RF-PRD-08..11) | P0 | G | B55 (publicações na montagem) |
| Estrutura de junta | Conjunto de prendedores > Conjunto da junta > pasta Prendedores (RF-PRD-12, 22, 27) | P0 | M | ponto publicado |
| Preenchimento por cor com grip por face | Regra cor → prendedor; grip pelas faces; falha visível (RF-PRD-13..15) | P0 | G | schema; estrutura de junta |
| Furo aeronáutico (tolerância, escareado, dimple) | `HoleRow` com tipo e tolerância; criação do furo pelo prendedor (RF-PRD-16/17) | P1 | M | B48, B57 |
| Travar rotação do prendedor | Fecha o B62; GDL = 0 em todo prendedor | P0 | P | B62 |
| Assistente de edição e substituição | Tabela por ponto, equivalentes, diagnóstico (RF-PRD-20/21/25) | P1 | G | estrutura de junta; B59 |
| Modo manual e padrão de rebites | RF-PRD-19, RF-PRD-23 | P2 | M | ponto publicado |
| Equivalência pol↔mm | Tabela explícita e aviso de rosca diferente (RF-PRD-24) | P2 | P | schema |
| BOM e relatório de furação | RF-PRD-26 | P2 | P | estrutura de junta |
| Corpus de prendedores | Peças de teste por família e por caso de C3 | P1 | M | B53 |

---

## 5. Fontes (URLs realmente usadas)

**Prendedores e dados de peça**
- https://shop.boeing.com/cpd/bdsis4_nas6203-9
- https://shop.boeing.com/cpd/bdsis4_nas6203-26
- https://shop.boeing.com/cpd/bdsis4_nas6204-30
- https://shop.boeing.com/aviation-supply/p/NAS6203-6=28
- https://skygeek.com/national-aerospace-standard-nas6203-6-bolt-hex-hd-st.html
- https://military-fasteners.com/catalog/pdf/NAS6203L6
- https://www.scribd.com/document/444708418/Data-Sheet
- https://shop.boeing.com/cpd/bdi_hl18pb6-3=r5
- https://shop.boeing.com/aviation-supply/p/HL18PB10-18=74
- https://shop.boeing.com/cpd/bdi_hl18pb5-14%3D28
- https://shop.boeing.com/aviation-supply/p/HL70-10=28
- https://shop.boeing.com/aviation-supply/p/HL86-6=74
- https://shop.boeing.com/aviation-supply/p/HL70TW5=28
- https://shop.boeing.com/aviation-supply/p/HST79CK8=28
- https://ww2.txtav.com/Parts/PartSearch/PartsDetail/HL10V5-6
- https://infohub.txtav.com/Parts/PartSearch/PartsDetail/HL11VBJ4-10
- https://infohub.txtav.com/Parts/PartSearch/PartsDetail/HL18PB10-6
- https://ww2.txtav.com/Parts/PartSearch/PartsDetail/HST10AG-6-5
- https://www.nsnsphere.com/nsn/rfq/sps-technologies-ltd/5320016009636/hst1087-5/
- https://www.scribd.com/document/934023707/HL13
- https://supplier.aero.bombardier.com/DHC_isupply/dh/PPS/0267PPS.pdf
- https://www.eng-tips.com/viewthread.cfm?qid=121526
- https://www.eng-tips.com/viewthread.cfm?qid=494403
- https://mocha.deltaintl.com/Hi%20shear/removal/Hi%20Shear%20HLA6005.html
- https://www.howmet.com/wp-content/uploads/sites/3/2023/05/C50L_Brochure.pdf
- https://titgemeyer.com/fileadmin/titgemeyer/downloads/befestigungstechnik/schliessringbolzen/katalog/en/10048EN0723_1_-_HuckBolt_C50L.pdf
- https://www.hfsindustrial.com/c6l90-r8-5g.html
- https://howmet.com/products/aerospace/blind-bolts
- https://dev.hfsindustrial.com/pub/media/resources/images/image/h/u/huck_brochure_en.pdf
- https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/5123792
- https://shop.boeing.com/cpd/bdi_cr3213-5-10=29
- https://shop.boeing.com/aviation-supply/p/CR3213-5-04=29
- https://shop.boeing.com/aviation-supply/p/CR3242-5-04=29
- https://shop.boeing.com/cpd/bdi_cr3242-6-7=29
- https://shop.boeing.com/cpd/bdi_cr3243-4-03%3D29
- https://shop.boeing.com/cpd/bdi_nas9301b4-01%3D28
- https://shop.boeing.com/cpd/bdi_cr3213-4-03%3D29
- https://www.military-fasteners.com/rivets/blind+rivets/CR3242-4-01
- https://vansairforce.net/threads/equivalent-cherry-rivet.244695/
- https://shop.boeing.com/cpd/bpp_nas1739e4-3
- https://shop.boeing.com/cpd/bdsis4_nas1399cw3a4
- https://shop.boeing.com/cpd/bdsis4_nas1399cw3a2
- https://shop.boeing.com/cpd/bdsis4_nas1398cw5a3
- https://military-fasteners.com/catalog/pdf/NAS1398B6A6
- https://everyspec.com/FSC-CODE/5320/?page=2
- https://shop.boeing.com/cpd/bdi_ms20470ad4-6-5%3D28
- https://shop.boeing.com/cpd/bdi_ms20426ad4-6-5%3D28
- https://olander.com/items/MS20470DD5-10
- https://www.olander.com/items/MS20470AD3-5
- https://www.olander.com/items/MS20470AD5-8
- https://www.olander.com/items/MS20470AD6-4
- https://www.mcmaster.com/products/rivets/military-specification~ms20426ad4-5/
- https://www.lasaero.com/products/article/S03YO6O8R
- https://www.lasaero.com/products/article/F00P2HVZ1
- https://open-exam-prep.com/study-guides/faa-amt-airframe/metallic-structures-sheet-metal/rivet-layout-driving-standards
- https://open-exam-prep.com/study-guides/faa-amt-general/materials-processes/aircraft-hardware-fasteners
- https://open-exam-prep.com/study-guides/faa-amt-airframe/welding-structural-fasteners/special-structural-fasteners
- https://flywithspa.com/docs/pbm/toc453317740.html
- https://www.dla.mil/Portals/104/Documents/TroopSupport/IndustrialHardware/Catalogs/FastenerCatalog_1.pdf
- https://shop.boeing.com/cpd/bdsis4_an3-5
- https://shop.boeing.com/cpd/bdsis4_an4-5a
- https://military-fasteners.com/bolts/machine bolts/AN3-5
- https://shop.boeing.com/cpd/bdi_an310-7%3d28
- https://shop.boeing.com/aviation-supply/p/AN310-6=30
- https://www.aircraftspruce.eu/Nut-Castle-AN310-8.htm
- https://skygeek.com/air-force-navy-an310-9-nut-castle-steel.html
- https://shop.boeing.com/cpd/bdi_an960-10%3D0h
- https://military-fasteners.com/catalog/pdf/AN960-10
- https://ww2.txtav.com/Parts/PartSearch/PartsDetail/AN960C616L
- https://www.mcmaster.com/inserts/nut-type~rivet-mount
- https://www.eng-tips.com/threads/nutplate-pitch.209285/
- https://omegatec.net/?p=104214
- https://www.airforums.com/threads/nutplates-etc.1300496/post-14851793
- https://shop.boeing.com/cpd/bdsis4_ms21209f8-15l
- https://www.mcmaster.com/products/threaded-inserts/military-specification~ms21209c0415l/
- https://www.military-fasteners.com/ms-series/inserts/ms21209c04
- https://us.misumi-ec.com/blog/helicoil-tap-and-drill-size-chart/
- https://shop.southco.com/en_us/d4-a35t30zctna
- https://www.aircraftspruce.com/catalog/pnpages/MS21919-DG4.php
- https://shop.boeing.com/cpd/bdi_ms21919wdg15%3d28
- https://www.military-fasteners.com/clamps/loop+clamps/MS21919WDG22

**CAD de referência**
- https://www.3ds.com/partners/products/200000000001731_PDT00000001402_FASTJOINT
- https://saemobilus.sae.org/content/2008-01-2303
- https://pycatia.readthedocs.io/en/0.93/api/pycatia/dnb_fastener_interfaces/dnb_fastener_management.html
- https://pycatia.readthedocs.io/en/0.93/api/pycatia/dnb_fastener_interfaces/point_fastener.html
- https://pycatia.readthedocs.io/en/0.93/api/pycatia/dnb_fastener_interfaces/fastener.html
- https://pycatia.readthedocs.io/en/0.93/api/pycatia/dnb_fastener_interfaces/dnb_fastener_item_services.html
- https://edstechnologies.com/blog/making-assemblies-easier-with-publications-in-catia-v5/
- https://www.scribd.com/document/19738625/Part-Design
- https://hawkridgesys.com/blog/solidworks-toolbox-smart-fasteners
- https://www.cati.com/?p=130205
- https://help.solidworks.com/2022/English/SolidWorks/toolbox/HIDD_TOOLBOX_SMART_FASTENERS.htm (só o resumo da busca; a página não renderizou)
- https://help.solidworks.com/2014/English/SolidWorks/sldworks/t_Editing_Smart_Fasteners_1.htm (idem)
- https://help.solidworks.com/2018/English/SolidWorks/Sldworks/c_Configuring_Bolt_Patterns_in_Smart_Fasteners.htm (idem)
- https://help.autodesk.com/cloudhelp/2025/ENU/Inventor-Help/files/GUID-6E1D4624-B605-4AB1-9DBA-E981495B4933.htm
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-5DD94FE0-0842-4D8D-894D-FDFD029195CA.htm

**Código e documentos do Fenkra lidos:** `biblioteca/fixadores/iso4762.json`, `crates/fenkra-solidos/src/biblioteca.rs`, `crates/fenkra-solidos/src/fixadores.rs`, `crates/fenkra-solidos/src/hardware.rs`, `crates/fenkra-solidos/src/lib.rs:5057-5078`, `crates/fenkra-montagem/src/fixadores.rs`, `crates/fenkra-montagem/src/lib.rs:278`, `crates/fenkra-core/src/doc.rs:2543`, `crates/fenkra-app/src/asm_ui.rs:641`, `docs/contextos/{fixadores,hardwares,smart-fastener}.md`, `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`, `docs/benchmark/catalogo.md` (MON-16/17/18, BIB-02/06/10) e `docs/backlog/backlog.md` (B28, B29, B43, B48, B51, B62, B63).

---

## Handoff

- **Resultado:** relatório completo entregue acima. Nenhum arquivo foi criado ou alterado.
- **Evidência:** código e docs do Fenkra lidos (lista acima). Dezenas de buscas e páginas abertas; os itens confirmados estão marcados [V] ou [V~].
- **Verificações rodadas:** nenhum teste do repositório.
- **Falhas:**
  - ASSIST e PDFs de normas não acessados.
  - Boeing, LAS Aero e DLA devolveram 403 em parte das tentativas.
  - Páginas `help.solidworks.com` não renderizaram.
  - PDF do folheto Huck C50L veio comprimido e ilegível (copiado pelo fetch para a pasta de resultados da sessão).
  - Não achei tutorial público de instalação de prendedores aeronáuticos no CATIA V5.
- **Riscos:**
  - Tabelas marcadas [C] não podem virar dados da biblioteca sem segunda fonte.
  - Alguns significados de norma ficaram em aberto: NAS1291, NAS1919/1921, MS16555, NAS607, HST12, HL19/HL20, HL77/HL86/HL94, MS21140 como lockbolt.
  - Os dados de nutplate têm conflito entre fontes.
  - O modo do plano (restrito a leitura) impediu gravar o plano; a entrega é este relatório.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra em prendedores aeronáuticos são: (1) biblioteca e schema só métricos, sem polegada, dash, grip, rebite, pino com colar nem inserto; (2) ausência de ponto/vetor publicado com cor = diâmetro e normal da superfície; (3) furo aeronáutico sem tolerância/escareado, com reconhecimento por Ø que nunca casa (#10 = 4,9 mm); (4) sem estrutura Conjunto de prendedores > Conjunto da junta > pasta Prendedores, sem assistente de edição/substituição e sem modo manual; (5) dados aeronáuticos sem duas fontes abertas conferidas (ASSIST fora do alcance), com grip tratado como comprimento e série esgotada tolerada em silêncio.
