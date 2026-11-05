# PMI / FT&A / MBD

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Relatório de pesquisa para o Fenkra. [V] marca o que foi confirmado em fonte pública. [C] marca conhecimento geral não confirmado nesta rodada.
>
> **Limites da pesquisa:**
> - As páginas de ajuda do CAD avançado A (`help.CAD avançado A.com`) não renderizam sem JavaScript. O comportamento do CAD avançado A vem de blogs de revendas (GoEngineer, CATI, Central Innovation, Engineers Rule) e de trechos de busca.
> - O guia oficial CAD avançado B FTA (CAD avançado Bdoc.free.fr) estava fora do ar. O CAD avançado B vem das notas de aula da fornecedor de CAD avançado (V5R19), das ementas de cursos e da apostila NIAR.
> - Nenhum arquivo foi criado ou alterado.

---

## 1. Resumo e principais lacunas

1. O PMI do Fenkra é um **desenho 3D de anotações**. Cada `Pmi` guarda pontos soltos em coordenadas do mundo (`anchors: Vec<[f64;3]>`, `pmi.rs:759`), um deslocamento e um tipo (`PmiKind`).
2. Não existe anotação **semântica**. O texto de um GD&T é uma `String`, os datums são uma `String` ("A B C"), a tolerância é uma `String`. Nada liga a letra do datum a uma face, nem o quadro de tolerância ao elemento tolerado.
3. Associatividade só por **posição**. A cada quadro a âncora é "puxada" ao vértice ou meio de aresta mais próximo, em até 2 mm (`pmi_ui.rs:243`). Se a geometria andou mais que isso, a âncora fica solta no espaço e **a cota mede o ponto antigo, sem aviso**.
4. As cotas medem **distância euclidiana entre dois pontos** (`pmi.rs:811`). Isso não é uma cota CAD de verdade: cota entre duas faces paralelas, cota ao longo de um eixo e cota de furo (centro) ficam erradas depois de qualquer edição.
5. Não há **estado de anotação** (OK, isolada, perdida), nem diagnóstico de atualização, nem verificação do esquema de tolerâncias. O catálogo registra isso como B38, P3.
6. O GD&T tem os 14 símbolos ISO 1101, mas faltam regras de norma:
   - ASME Y14.5-2018 removeu concentricidade e simetria, e o Fenkra ainda as oferece.
   - Não há seleção de norma ativa (ASME ou ISO), perfil dinâmico, "todo o contorno", zona projetada com valor, modificadores por datum nem tolerância não uniforme.
7. Não há planos de anotação nomeados, nem vistas de anotação, nem capturas com corte e estado. A captura guarda só câmera, lista de visíveis e (opcional) o que estava oculto no modelo.
8. A exportação STEP usa `STEPControl_Writer` (`kernel.cpp:988`). Ela **não leva PMI**, nem AP242, nem nomes, nem cores. Também não existe importação de PMI.
9. Rugosidade só tem Ra e processo. Solda só tem ISO 2553 sistema A. Não há alvos de referência (datum targets) nem cotas básicas estruturadas.
10. Direção recomendada: um **modelo semântico em camadas**, e as três lacunas de maior risco na ordem abaixo.
    - **(a)** Âncoras topológicas (chaves da ADR 0010).
    - **(b)** Estado e diagnóstico por anotação.
    - **(c)** Exportação STEP AP242 pelo XCAF, com saída semântica mais gráfica.

**Cinco maiores lacunas** (repetidas no SUMMARY):
1. Âncoras por ponto e medida ponto a ponto.
2. Sem semântica nem ligação datum ↔ face ↔ quadro.
3. Sem estado, diagnóstico nem consultor de tolerâncias.
4. Sem STEP AP242 com PMI.
5. Sem planos de anotação, vistas de anotação nem capturas completas.

---

## 2. Fichas por comando

### F1. Modelo de anotação 3D e associatividade (CAD avançado B: FT&A, anotação semântica e não semântica / CAD avançado A: MBD, DimXpert / CAD avançado C: 3D Annotations, Tolerance Feature / CAD avançado F: GD&T Advisor / CAD avançado G: PMI / Fenkra: `Pmi`, `pmi.rs:754`)

- **O que faz:** liga uma informação de fabricação (cota, tolerância, datum, acabamento) a uma face, aresta ou feature do modelo. A informação precisa sobreviver às edições da geometria e ser lida por máquina (CAM, inspeção).
- **CAD avançado B — como funciona no detalhe:**
  - [V] O workbench é *Functional Tolerancing & Annotations*. As anotações são **semânticas** (conhecem o elemento tolerado e seguem ISO/ASME) ou **não semânticas** (só texto e símbolo).
  - [V] Semânticas: datum, datum target, tolerância geométrica, cotas, cotas enquadradas (básicas), cotas geradas. Rugosidade e nota com chamada aparecem em ambos os grupos nas notas de aula, que se contradizem entre si.
  - [V] Existe a opção *Tools > Options > Mechanical Design > FT&A > Tolerancing > Semantic Control > "Always try to create semantic tolerances and dimensions"*. Por padrão, a tolerância geométrica semântica só é criada pelo Tolerancing Advisor (trecho da busca sobre o 3DCS).
  - [V] A geometria 3D é a referência. Ao modificar a peça, as vistas 2D extraídas e as anotações são atualizadas.
  - [V] Há *Geometry for 3D Annotations*, com **geometria construída** (pontos e linhas auxiliares) e *Geometry Connection Management*. Esses itens aparecem nos índices dos cursos.
  - [C] Quando a geometria some, a anotação fica "isolada" ou "quebrada" e o *Update Diagnosis* a lista.
- **CAD avançado A — no detalhe:**
  - [V] O DimXpert reconhece "features de fabricação" (furo, rasgo, boss, plano) e aplica a cota ou tolerância à feature, não ao ponto.
  - [V] A anotação continua ligada à feature correta mesmo depois de quebrar uma chamada combinada (artigo Engineers Rule). O artigo chama isso de associação semântica, usada por CAMWorks e CheckMate.
  - [V] A anotação "gráfica" pode ser movida e ajustada sem mudar o vínculo.
- **CAD avançado C — no detalhe:**
  - [V] *Tolerance Feature* só existe em peças. Ele anexa quadros GD&T e identificadores de datum a faces ou features.
  - [V] O comando detecta cotas de sketch ou feature relacionadas e **promove** a cota de tolerância (as setas de cota viram o valor nominal da tolerância).
  - [V] As anotações ficam em pastas do navegador, com realce cruzado entre janela e navegador.
- **Outros:**
  - [V] CAD avançado F: o GD&T Advisor guarda tudo no arquivo da peça. Cada anotação nativa fica presa à sua feature, e a seleção da feature na árvore realça a anotação.
  - [V] AP242 (CAx-IF): PMI semântico representa tolerâncias, datums e dimensões como dados. O gráfico é só a aparência (linhas, cor, posição).
- **Fenkra hoje:**
  - `Pmi` (`pmi.rs:754-780`) tem `anchors` (pontos), `offset`, `text`, `tol_plus` e `tol_minus` (strings), `plane` e `style`. O `kind` é um enum fechado.
  - O doc do módulo diz: "as âncoras se reencaixam no vértice/aresta mais próximo e as cotas medem de novo" (`pmi.rs:4-5`).
  - `snap` (`pmi_ui.rs:243-255`) procura vértice ou meio de aresta em até 2 mm (valor fixo). Face e centro de furo nunca "reencaixam".
  - **Bug provável:** cota entre duas faces: `anchors()` usa `Pick::Face { point }` (ponto clicado). A medida (`pmi.rs:811`) é a distância entre os dois pontos clicados, não a distância entre as faces. Se uma face mover, a cota continua mostrando a distância antiga.
  - **Bug provável:** raio/diâmetro: âncora 0 = centro ajustado (`pmi_ui.rs:65-66`). O centro não é vértice nem meio de aresta, então `snap` o deixa parado. Se o furo mudar de lugar ou de tamanho, o valor fica errado.
  - **Bug provável:** na montagem (`overlay`, `pmi_ui.rs:449`), `snap` e `highlight` usam `app.result`, que é o resultado da peça aberta. As âncoras da montagem são comparadas com a malha errada. Isso bate com "realce falha na montagem" do catálogo (PMI-04).
  - Não existe `status` (OK, perdida) por anotação.
- **Melhor prática para o Fenkra:**
  - Separar três camadas em cada anotação:
    - **Semântica:** o quê (cota, tolerância, datum…).
    - **Alvos:** as referências (chaves `FaceKey`, `EdgeKey`, vértice, eixo, ponto construído).
    - **Apresentação:** plano, deslocamento, chamadas e estilo.
  - Resolver os alvos no recompute, como na ADR 0010 (dono → papel → assinatura → ponto como reserva). A anotação ganha `status`: `Ok`, `Isolada` (alvo não resolve) ou `Ambígua`.
  - **Nunca** escolher outra geometria em silêncio (princípio 1 da ADR 0011).
  - O valor da cota vem da **medida entre alvos** (face-face, eixo-eixo, ponto-ponto), não de pontos clicados.
  - Casos de teste:
    - (i) Cota entre duas faces planas paralelas; mudar a espessura; o valor acompanha.
    - (ii) Cota de furo; mover o furo e mudar o diâmetro; o valor acompanha.
    - (iii) Apagar a face; a anotação vira "isolada" e aparece no diagnóstico.
    - (iv) Montagem: anotação de componente continua certa depois de mover o componente.
    - (v) Salvar e abrir mantém alvos e estado.

### F2. Cota linear / de distância (CAD avançado B: Dimension, Distance / CAD avançado A: MBD Dimension, DimXpert Size/Location / CAD avançado C: General Dimension / Fenkra: `PmiKind::Linear`)

- **O que faz:** mede e comunica uma distância entre dois elementos, com linhas de chamada e linha de cota.
- **CAD avançado B — como funciona no detalhe:**
  - [V] A cota semântica é ligada a elemento(s) tolerado(s) por *Limit of Size Definition* (tamanho) ou por localização.
  - [V] A condição de envelope (*Enveloping*) é escolhida na criação.
  - [V] **Cotas enquadradas** (*framed dimensions*, cota básica) devem estar ligadas a referências parciais, áreas restritas ou a uma das tolerâncias.
  - [V] Cotas de ângulo mostram os limites superior e inferior na caixa *Limit of Size*.
  - [C] O texto fica no plano de anotação ativo. As linhas de chamada saem das arestas.
- **CAD avançado A — no detalhe:**
  - [V] *Size Dimension* dimensiona o tamanho de uma feature (ex.: furo). *Location Dimension* mede a posição da feature em relação a um datum (ex.: um plano).
  - [V] Cotas básicas podem ser criadas por *Recreate basic dim* (clique direito numa tolerância de posição na árvore DimXpert), pela opção de documento *Create basic dimensions* ou manualmente, em estilo corrente (chain) ou de base (baseline).
  - [C] Cota de referência (entre parênteses) é diferente da cota dimensionante.
- **CAD avançado C — no detalhe:**
  - [V] As cotas gerais fazem parte do grupo *General Annotations* (aba Annotate). O fluxo é igual ao do desenho: depois de posicionar ou confirmar, o comando permite colocar outro símbolo do mesmo tipo.
  - [V] A unidade e o padrão vêm de *Document Settings > Standard > Active Standard*. A aba Units não vale para anotações 3D.
- **Outros:** [V] CAD avançado G: nota "paralela à tela" e "bloquear tamanho e posição" (configurações do botão direito).
- **Fenkra hoje:** `PmiKind::Linear` (`pmi.rs:695`). Duas âncoras, medida euclidiana (`pmi.rs:811`). Texto em `display()` com `{:.2}` e zeros finais cortados (`pmi.rs:840-844`).
  - O desenho tem linhas de chamada e seta (`pmi_ui.rs:498-551`), com o texto do lado de fora da linha de cota.
  - Faltam:
    - cota **alinhada, horizontal, vertical, ao longo de um eixo** e cota entre planos;
    - cota entre eixo e face;
    - **precisão** configurável (cota 25,40 aparece como "25,4");
    - cota de referência e cota básica;
    - cota repetida (`4×`, só via prefixo de texto em `style.prefix`).
  - A vírgula decimal está fixa no código (`replace('.', ",")`), sem seguir a norma ativa (ASME usa ponto).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (par de elementos: face-face, aresta-aresta, ponto-ponto, eixo-face) → modo (distância mínima, ao longo de eixo X/Y/Z ou eixo da peça, projetada no plano de anotação) → parâmetros (precisão, tolerância, prefixo/sufixo) → avançado (cota de referência, básica, repetida).
  - A medida é calculada entre **alvos resolvidos**, projetada no modo escolhido. Para face-face paralelas, a distância é perpendicular.
  - A precisão e o separador seguem a norma ativa. A precisão da cota acompanha a da tolerância (ASME exige o mesmo número de casas em tolerâncias bilaterais [C]).
  - Casos de teste: cota de espessura de placa (face-face); cota horizontal entre dois furos em vista oblíqua; precisão 2 casas com tolerância ±0,05; cota entre planos em montagem.

### F3. Raio, diâmetro e cota de furo/rosca (CAD avançado B: Dimension + Thread representation / CAD avançado A: Hole callout, DimXpert Hole / CAD avançado C: Hole/Thread Note / Fenkra: `Radius`, `Diameter`)

- **O que faz:** cota de feature circular. Em CAD de verdade, a anotação "lê" o furo (diâmetro, profundidade, rebaixo, rosca, quantidade).
- **CAD avançado B — como funciona no detalhe:**
  - [V] O fluxo é: selecionar o furo, abrir o Advisor, escolher *Diameter*, informar a tolerância e a condição *Enveloping*, e confirmar em *Limit of Size Definition*.
  - [V] A tolerância de N elementos (N surfaces, *Tab/slot*, *Pattern*) cobre vários furos iguais. *Pattern* propaga com "all same diameter parallel cylinder".
  - [V] O curso cita *Thread Representations* e anotações próprias de rosca.
- **CAD avançado A — no detalhe:**
  - [V] *Hole callout* (cota de furo) com rebaixo, escareado e posição polar (estudo de caso do carcaça de mancal).
  - [V] Uma chamada combinada (furo de mancal com rebaixo) pode deixar ambígua a qual diâmetro a tolerância se refere. A solução foi **quebrar** a chamada.
  - [V] Auto Dimension Scheme aplica posição ao padrão de furos.
- **CAD avançado C — no detalhe:**
  - [V] *Hole/Thread Note* é um comando de anotação geral. Desde a 2019 ele mostra a **quantidade** do padrão. Isso exige o estilo 3DA 2019 na biblioteca ativa.
- **Outros:** [V] CAD avançado F: ao selecionar um furo de um padrão CAD, o Advisor pergunta "adicionar o padrão inteiro" e o mantém **ligado** ao padrão da feature. Mudar a contagem atualiza a tolerância.
- **Fenkra hoje:** `Radius` e `Diameter` (`pmi.rs:697-699`). A âncora 0 é o centro ajustado por `fit_circle` e a âncora 1 é o meio da aresta (`pmi_ui.rs:61-67`).
  - A medida é `d(centro, ponto)` (×2 no diâmetro).
  - O diâmetro desenha a linha atravessando o centro (`pmi_ui.rs:386-392`).
  - Faltam: leitura da feature (profundidade, rebaixo, rosca), quantidade/padrão, e cota de furo a partir de um comando de furo do Fenkra (os furos do modelo não são lidos).
  - Bug provável: ver F1 (centro parado).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (aresta ou face cilíndrica ou **feature de furo**) → modo (raio, diâmetro, cota de furo completa) → parâmetros → avançado (padrão ligado, quantidade, rosca).
  - Quando o alvo é a face cilíndrica, o valor vem do **raio da superfície**, não de pontos.
  - Para padrão linear/circular, oferecer "anotar o padrão inteiro" e manter a ligação.
  - A nota de furo pode ser gerada a partir da feature de furo: `Ø6,5 ▽12 · M8×1,25 · 4×`.
  - Casos de teste: furo cego com rosca (nota completa); padrão de 4 furos, mudar para 6, a quantidade acompanha; mudar o diâmetro do furo e a anotação acompanha.

### F4. Cota angular (CAD avançado B: Angular Dimension / CAD avançado A: Angular DimXpert / CAD avançado C: Angular / Fenkra: `Angle`)

- **O que faz:** mede o ângulo entre duas retas ou planos.
- **CAD avançado B — como funciona no detalhe:** [V] A caixa *Limit of Size* mostra limite superior e inferior como na janela 3D. [C] A cota angular semântica liga duas faces ou eixos.
- **CAD avançado A — no detalhe:** [C] O DimXpert tem *Angular dimension* e a angularidade como tolerância geométrica. Para peças prismáticas, o esquema automático aplica cotas angulares onde há chanfro [C].
- **CAD avançado C — no detalhe:** [C] Cota angular como anotação geral entre duas arestas ou faces.
- **Outros:** não relevante.
- **Fenkra hoje:** `Angle` com três âncoras (vértice e dois pontos), `acos` do ângulo entre os vetores (`pmi.rs:814-820`), faixa 0–180°. Desenho do arco em `pmi_ui.rs:552-580`.
  - Não mede ângulo entre duas arestas, duas faces ou eixo e plano.
  - Sem escolha de ângulo complementar ou externo (reflexo).
  - Os pontos são fixos: mesmo problema de associatividade.
- **Melhor prática para o Fenkra:**
  - Suporte: duas arestas retas, duas faces planas ou eixo+face. O vértice vem da interseção ou do ponto mais próximo.
  - Modos: interno, externo, complementar, ângulo de plano normal à vista.
  - Casos de teste: chanfro de 45° entre duas faces; mudar o ângulo de um chanfro e a cota acompanha; duas arestas que não se cruzam (lados de uma peça torta) retornam erro claro.

### F5. Tolerância dimensional, ajustes e cotas básicas (CAD avançado B: Limit of Size / CAD avançado A: Tolerance/Precision, Plus-Minus / CAD avançado C: Tolerance / Fenkra: `tol_plus`, `tol_minus`)

- **O que faz:** acrescenta tolerância à cota: simétrica, bilateral, unilateral, limites, ajuste ISO 286 (H7, g6), ou torna a cota "básica" (sem tolerância, enquadrada, usada com GD&T).
- **CAD avançado B — como funciona no detalhe:** [V] A tolerância semântica da cota passa por *Limit of Size Definition*. As cotas "enquadradas" (framed) são as básicas e só vivem ligadas a referências parciais, áreas restritas ou tolerâncias geométricas. [V] O Advisor oferece só os tipos permitidos para o elemento selecionado. [C] Classes ISO 286 podem ser escolhidas por tabela.
- **CAD avançado A — no detalhe:**
  - [V] O DimXpert tem tipos de tolerância *Plus and Minus* e *Geometric* no esquema automático. O esquema "geométrico" substitui as tolerâncias de cota por posição e cria cotas básicas (opção *Create basic dimensions*).
  - [V] Os valores padrão vêm de *Document Properties > DimXpert > Geometric Tolerance*.
  - [C] Tolerância geral (ISO 2768) vai na nota geral.
- **CAD avançado C — no detalhe:** [V] Modificadores de dimensão (Continuous Feature, Statistical Tolerance, Envelope Requirement) são acrescentados ao texto da cota e aparecem na ordem de inclusão.
- **Outros:** [V] CAD Exchanger (AP242): a cota semântica guarda valor nominal, limites ±, faixas e **classe ISO 286** (`LimitsAndFitsAttribute`) como atributos separados. [V] CAD avançado F: "Dimension is ambiguous" quando uma cota de offset não tem origem definida.
- **Fenkra hoje:** `tol_plus` e `tol_minus` são `String`s livres (`pmi.rs:765-768`). `display()` monta `±x` se iguais ou `+a/−b` (`pmi.rs:845-851`). `display_parts` empilha a tolerância assimétrica (`pmi.rs:826-836`).
  - Não há número validado: "0,1" e "abc" são aceitos.
  - Sem limites (`25,05 / 24,95`), sem ajuste ISO 286, sem tolerância geral por classe (ISO 2768).
  - Cota básica só por moldura "retângulo" (`PmiFrame::Caixa`), sem vínculo com tolerância de posição.
  - A vírgula decimal é fixa.
- **Melhor prática para o Fenkra:**
  - Modelo tipado: `Tolerance { Nenhuma, Simetrica(v), Bilateral(+a,−b), Limites(min,max), Ajuste{classe,furo/eixo}, Basica, Referencia }`.
  - Validação numérica no painel. Aviso se `|tol| ≥ valor` ou se o sinal for incoerente.
  - Cota básica é um estado da cota. Só é aceita como "basica" se existir um quadro de posição, perfil ou angularidade apontando para ela.
  - Casos de teste: ajuste `Ø20 H7` mostra desvios corretos da tabela ISO 286 (ex.: +0,021/0); tolerância bilateral `+0,1/−0,05` empilhada; cota básica sem quadro de posição vira aviso.

### F6. Nota, texto e chamada (CAD avançado B: Text with leader, Flag Note / CAD avançado A: Note / CAD avançado C: Text and Leaders, General Profile Note / Fenkra: `Note`)

- **O que faz:** texto livre ligado a um ponto ou elemento, geralmente com chamada.
- **CAD avançado B — como funciona no detalhe:** [V] *Flag note* e texto com chamada fazem parte das anotações. [C] As notas gerais ficam num plano próprio ("Notes area").
- **CAD avançado A — no detalhe:** [V] A anotação em vista *Notes Area* guarda notas gerais; as vistas de anotação (*Front, *Right, *Top) mostram só o que pertence àquela vista. [C] Notas podem ter hiperlinks e propriedades personalizadas ($PRP).
- **CAD avançado C — no detalhe:** [V] *General Profile Note* acrescenta informação de fabricação (perfil geral). [V] A nota de furo/rosca é um comando específico.
- **Outros:** [V] CAD avançado F: nota de perfil geral (*General Profile note*, tolerância 0,3) referencia o DRF predominante e cobre as superfícies não anotadas. [V] CAD avançado G: nota "paralela à tela".
- **Fenkra hoje:** `Note` (`pmi.rs:702`), texto multilinha (`pmi_ui.rs:1212-1215`), chamada e patamar em `pmi_ui.rs:434-441` e `pmi_draw.rs:402-426`. A chamada não cruza o quadro (corrigido por `leader`).
  - Falta: uma **seta só** (DR-48 pede várias), campos de propriedades da peça na nota comum (só a legenda 3D tem `{campos}`), notas numeradas, lista de notas gerais.
- **Melhor prática para o Fenkra:**
  - Notas com mais de um alvo, com setas independentes. Campos `{propriedade}` em qualquer nota.
  - Notas gerais ficam numa área própria ("Notas gerais"), numeradas, com um resumo editável.
  - Nota de perfil geral: "superfícies não anotadas: perfil 0,3 em relação a A|B|C", ligada ao DRF.
  - Casos de teste: nota com duas setas; apagar um alvo mantém a nota com uma seta e aviso; campo `{material}` atualiza quando a propriedade muda.

### F7. Referência (datum), alvo de referência (datum target) e sistema de referência (DRF) (CAD avançado B: Datum, Datum Target, Datum System / CAD avançado A: Datum Feature, DimXpert Datum / CAD avançado C: Datum Identifier, DRF / Fenkra: `Datum { letter }`)

- **O que faz:** define que feature serve de referência (A, B, C) e como elas formam o sistema (DRF) usado pelas tolerâncias de orientação, posição, batimento e perfil.
- **CAD avançado B — como funciona no detalhe:**
  - [V] **Datum simples:** selecionar a face, abrir o Advisor, escolher o ícone Datum, digitar o rótulo. A árvore de especificação registra o datum.
  - [V] **Sistema de datum especificado (A, B, C):** os datums simples precisam existir antes. Em *Add*, escolhe-se o primário, depois o secundário e o terciário.
  - [V] **Datum comum (A‑B):** os elementos valem simultaneamente.
  - [V] **Datum targets** e **áreas restritas** (restricted areas) têm tópicos próprios no curso. [V] *Datum Targets* aparecem como anotação semântica e como filtro.
  - [V] Orientação e posição **exigem** um DRF já criado.
- **CAD avançado A — no detalhe:**
  - [V] No esquema automático, escolhem-se datum primário, secundário e terciário. O DimXpert valida o conjunto segundo ASME Y14.5.1M‑1994. Em versões novas dá para escolher datums que já têm símbolo (antes era preciso reselecionar a cada comando).
  - [V] Superfícies cônicas passaram a ser datum válido no MBD 2017.
  - [V] Clicar com o botão direito aceita o datum e passa para a próxima caixa. Um comando de contexto aceita o datum mesmo com o mouse longe.
- **CAD avançado C — no detalhe:**
  - [V] Comando *DRF*: diálogo com primário, secundário e terciário e seus modificadores. O DRF aparece na pasta Tolerance Features, e se edita com *Edit DRF*.
  - [V] A lista *Datum identifier* cria **datum targets** a partir de pontos de sketch ou work points mais um work plane. Pontos de sketch são selecionados automaticamente; work points, à mão.
  - [V] Mensagens do Advisor sobre datums: "DRF is not referenced", "DRF specification is incomplete", "Datum object is missing", "Orientation of secondary datum is not allowed", "Should be set as a datum feature or reordered".
- **Outros:** [V] CAD avançado F: o Advisor monta o **DRF predominante** (A|B(M)|C(M)), sugere as primeiras features como datum e limita a lista de DRFs possíveis. Regra geral: o DRF predominante deve restringir os 6 graus de liberdade, com exceção de peças simétricas (esfera, cilindro simples).
- **Fenkra hoje:** `Datum { letter: String }` (`pmi.rs:713`). Âncora = ponto clicado. O símbolo (triângulo cheio mais quadrado com a letra) está em `pmi_draw.rs:518-537`.
  - O quadro de tolerância guarda `datums: String` ("A B C", dividida por espaço, vírgula ou barra, `pmi.rs:865`). **Não há vínculo** com a anotação `Datum`.
  - Letras duplicadas são aceitas; a letra "I", "O" e "Q" (proibidas por norma) também [C].
  - Faltam: datum target (ponto, linha, área), tipo de datum (feature de tamanho ou superfície), DRF como objeto, datum comum "A‑B" estruturado, modificadores por datum, verificação de ordem (primário/secundário/terciário), verificação de graus de liberdade.
  - Apagar um `Datum` deixa o quadro de tolerância apontando para uma letra inexistente, sem aviso.
- **Melhor prática para o Fenkra:**
  - `DatumFeature { letra, alvo }` e `Drf { primario, secundario?, terciario?, modificadores }` como objetos nomeados na árvore.
  - Tolerâncias referenciam `DrfId`, não texto.
  - Validações imediatas:
    - letra única;
    - letras proibidas;
    - o secundário deve restringir graus de liberdade ainda livres;
    - orientação exige ao menos o primário;
    - posição de padrão exige DRF completo.
  - Datum target: ponto (símbolo ×), linha, área; desenhados com moldura de alvo (quadro dividido com letra e número).
  - Casos de teste: DRF A|B|C com três planos ortogonais restringe 6 DOF; apagar o datum B marca o DRF como "inválido" e todas as tolerâncias que o usam recebem aviso; renomear a letra A atualiza todos os quadros.

### F8. Quadro de tolerância geométrica — características e zonas (CAD avançado B: Geometrical Tolerance via Advisor / CAD avançado A: Geometric Tolerance, DimXpert / CAD avançado C: Tolerance Feature / CAD avançado F: Tolerance Feature / CAD avançado G: Feature Control Frame / Fenkra: `Gdt`)

- **O que faz:** cria o quadro de controle (FCF) com característica, zona (valor e forma), modificadores e referências, ligado ao elemento tolerado.
- **CAD avançado B — como funciona no detalhe:**
  - [V] O **Advisor** mostra só as tolerâncias válidas para o elemento. Para um plano, oferece retitude, planeza, perfil de linha e perfil de superfície. Para cilindro, as de forma do cilindro. A lista muda com a seleção (superfície ou aresta).
  - [V] Entradas por tipo: planeza (valor, confirma em *Geometrical Specification*); perpendicularidade (furo, datum A, característica, valores); posição (elemento, Advisor, DRF da lista, valor).
  - [V] **Direção da zona de tolerância** (*Tolerance Zone Direction*) para forma, posição e orientação: clica e escolhe a aresta de referência.
  - [V] **N elementos:** uma superfície; N superfícies (zona comum) com propagação "all same canonicity faces"; *Tab/slot*; *Pattern*.
  - [V] A opção semântica é obrigatória para o Advisor.
- **CAD avançado A — no detalhe:**
  - [V] No esquema automático geométrico: planeza no datum primário, perpendicularidade no secundário em relação a A, posição no terciário em relação a A e B (padrão quando o primário é plano).
  - [V] O quadro pode ser **arrastado e solto** em cima de uma chamada de cota existente e se combina com ela. Ele fica na feature correta depois de quebrar a chamada combinada.
  - [V] A opção *Create basic dimensions* cria cotas básicas para o quadro de posição.
- **CAD avançado C — no detalhe:**
  - [V] A prévia interativa na minibarra permite alterar a geometria ou tipo da feature, anexar identificador de datum, acrescentar uma linha ao quadro. Clicar nos elementos da prévia os edita.
  - [V] Mensagens-chave: "Tolerance value must be smaller than preceding value", "Form tolerance value must be smaller", "Tolerance on subfeature must be smaller than related tolerance", "Zero-value (or inconsistent) tolerance not allowed", "Unequally Disposed profile value not within valid range", "Projected Tolerance Zone value must be greater than zero", "Tolerance feature shares faces".
  - [V] O Advisor de 2024 passou a suportar ASME 2009 e ISO 2012.
- **Outros:**
  - [V] CAD avançado F: *Tolerance Feature* pede nome, esquema, símbolo, valor e datum; sugere símbolos filtrados pela feature. Distingue cota de tamanho de cota de offset (regra 1 só vale para feature de tamanho).
  - [V] CAD avançado G: *Feature Control Frame* pede plano de colocação, objeto terminal da chamada, depois característica, valor e datums; "MBD logical rules" aplicam quadros por regra.
  - [V] ASME Y14.5‑2018 removeu **concentricidade e simetria** (usar posição, batimento ou perfil) e introduziu **perfil dinâmico** (controla forma sem alterar o tamanho). O termo "dual datum" virou *Common Datum Features*, e o *Datum Feature Simulator* voltou a *True Geometric Counterpart*. [V‑parcial] ISO 1101:2017: não confirmei a lista de mudanças em fonte pública nesta rodada.
- **Fenkra hoje:** `GdtChar::ALL` tem 14 características (`pmi.rs:11-44`), com os símbolos Unicode ISO 1101 (`pmi.rs:46-63`).
  - `GdtExtra` (`pmi.rs:345-356`): `zone_dia` (Ø), **um** modificador de texto no valor (Ⓜ Ⓛ Ⓟ Ⓕ Ⓢ Ⓣ Ⓤ, `pmi_ui.rs:1073`), e segunda linha (quadro composto).
  - O valor é `String`. As referências são `String` (`gdt_ui`, `pmi_ui.rs:1055-1094`).
  - O quadro é desenhado por `pmi_draw.rs:473-514`. O símbolo é um **glifo de fonte** (`pmi_draw.rs:479`). Se a fonte do sistema não tiver `⏤ ⏥ ⌭ ⌖ ⌯ ⌰ ⌓`, aparece "tofu" [C, verificar com as fontes carregadas em `app.rs:6358`].
  - Faltam:
    - característica **válida para o elemento** (o Advisor);
    - zona retangular/largura (`⌀` só liga/desliga);
    - perfil **dinâmico**, perfil "todo o contorno" / "todo o entorno";
    - modificadores por datum e modificador do datum (`A Ⓜ`) estruturados;
    - tolerância por unidade de comprimento (0,05/25);
    - zona projetada com valor, "CZ" (zona comum), "entre A↔B";
    - concentricidade/simetria sem aviso da norma ASME 2018.
  - Bug provável: tolerância em string livre. "0,05" e "0.05" são aceitos como textos diferentes, e "abc" é aceito.
- **Melhor prática para o Fenkra:**
  - Fluxo: **suporte** (elemento: face plana, face cilíndrica, eixo, feature de tamanho, padrão) → **característica** (lista **filtrada** pelo tipo do elemento, como o Advisor) → **parâmetros** (valor numérico, forma da zona Ø/largura/esfera, modificador) → **avançado** (DRF, modificadores por datum, CZ, "entre", por unidade, perfil dinâmico, quadro composto ou de segmento único).
  - Matriz de validade (norma ativa): forma sem datum; orientação, posição, perfil e batimento exigem datum (perfil e posição aceitam ausência no caso de perfil de forma).
  - A norma ativa desativa concentricidade e simetria em ASME 2018 (aviso "removida em Y14.5‑2018; use posição/batimento") e as mantém em ISO 1101.
  - Símbolos desenhados como **caminhos vetoriais** do próprio Fenkra, não como glifos de fonte (como já é feito com a solda em `weld_symbol`). Isso evita tofu e permite exportar para DXF/PDF.
  - Casos de teste:
    - planeza em face plana funciona; perpendicularidade sem DRF dá erro;
    - posição de padrão de 4 furos com `Ø0,1 Ⓜ | A | B | C`;
    - perfil de superfície com "todo o contorno";
    - o quadro composto grava duas linhas;
    - trocar a norma ASME 2018 → ISO mostra/oculta concentricidade;
    - todos os símbolos desenham em captura de tela de referência.

### F9. Modificadores, regras e quadros compostos (CAD avançado B: Geometrical Tolerance, modificadores / CAD avançado A: FCF modifiers / CAD avançado C: Feature modifiers / Fenkra: `GdtExtra.modifier`, `second`)

- **O que faz:** representa condição de material (MMC/LMC/RFS), zona projetada, estado livre, característica contínua, estatística, plano tangente, zona desigual, e quadros compostos.
- **CAD avançado B — como funciona no detalhe:** [V] A condição *Enveloping* é escolhida na cota. [C] Modificadores aparecem como opções do diálogo do Advisor, e o quadro composto usa a segunda linha com referências diferentes.
- **CAD avançado A — no detalhe:** [C] O diálogo de tolerância geométrica tem modificadores por valor e por datum, quadro composto e segmento único.
- **CAD avançado C — no detalhe:** [V] Continuous Feature, Statistical Tolerance e Envelope Requirement são anexados ao texto da cota na ordem de inclusão. [V] Os modificadores do DRF são definidos no diálogo do DRF. [V] Mensagem "Gage size does not fall within the feature extents".
- **Outros:** [V] AP242: os modificadores de nível de tolerância (MMR/LMR) são valores enumerados. Os de datum são enumerados ou levam valor (zona projetada, distância). Uma tolerância com limite superior máximo precisa ter também MMR ou LMR. [V] Y14.5‑2018: *Continuous Feature* (CF) dispensa a numeração "nX" em elementos contínuos.
- **Fenkra hoje:** **um** modificador por quadro, texto (`modifier: String`). Segunda linha com `(String, String)` (tolerância, datums) sem modificadores. Modificadores aplicam-se só à zona, não ao datum.
- **Melhor prática para o Fenkra:**
  - Modificadores como `enum` com compatibilidade por característica (Ⓜ/Ⓛ só em feature de tamanho; Ⓟ só em posição/orientação; Ⓕ só em forma).
  - Modificador do datum na posição do datum (`C Ⓜ`).
  - Regra "RFS é o padrão" visível (aviso opcional).
  - Casos de teste: Ⓜ em planeza é recusado; Ⓜ em posição de furo é aceito; CF suprime `nX`.

### F10. Rugosidade / textura superficial (CAD avançado B: Roughness / CAD avançado A: Surface Finish / CAD avançado C: Surface Texture / Fenkra: `Roughness { ra, process }`)

- **O que faz:** indica o acabamento exigido numa superfície: parâmetro (Ra, Rz…), valor, método, direção de lay, sobremetal.
- **CAD avançado B — como funciona no detalhe:** [V] Rugosidade é anotação do FT&A, mas as notas de aula a tratam como **não semântica** em um slide e semântica em outro (contradição na fonte). [C] Valores e símbolo por norma (ISO 1302 / ASME Y14.36).
- **CAD avançado A — no detalhe:** [V] *Surface finish* faz parte do MBD (parte do pacote "dimensões, tolerâncias, acabamento superficial, solda, notas, tabelas"). [C] Inclui parâmetros, lay, sobremetal e processo.
- **CAD avançado C — no detalhe:** [V] *Surface texture symbol* é anotação geral.
- **Outros:** [V] AP242 (CAD Exchanger): `SurfaceFinishComponent` tem requisitos de textura, método de fabricação e **sobremetal de usinagem**. [V] ISO 21920‑1 substituiu a ISO 1302:2002. O símbolo ganhou variantes: raiz com traço acima para especificação segundo ISO 21920, e o losango inclinado para áreas (ISO 25178). A posição do parâmetro e do limite mudou. Assim, "Ra apenas" não basta como modelo.
- **Fenkra hoje:** `Roughness { ra: String, process: String }` (`pmi.rs:704`). Texto "√ Ra x (processo)" (`pmi.rs:859`). Desenho do símbolo em `pmi_draw.rs:541-562`: √ com barra e o texto em cima.
  - Faltam: Rz/Rmax e outros parâmetros, **limite superior/inferior**, "máximo", comprimento de amostragem, direção de lay, sobremetal, símbolo "remoção de material proibida" / "obrigatória" (círculo/triângulo), "todo o contorno", norma (ISO 1302 / 21920 / ASME).
  - A âncora é um ponto. A rugosidade deveria ser atributo da **face** (e ir para o STEP como propriedade da face).
- **Melhor prática para o Fenkra:**
  - Modelo: `SurfaceTexture { requisito: Qualquer|RemocaoObrigatoria|RemocaoProibida, parametros: Vec<{simbolo,valor,limite,regra}>, metodo, lay, sobremetal, norma }`.
  - Alvo = uma ou mais faces; a mesma rugosidade pode valer para várias ("todas as faces").
  - Rugosidade geral na legenda ("exceto indicado: Ra 3,2 (√)").
  - Casos de teste: Ra 1,6 e Rz 6,3 no mesmo símbolo; símbolo "remoção proibida"; face removida → anotação isolada; rugosidade geral aparece na legenda.

### F11. Símbolo de solda (CAD avançado B: Welding (FT&A / Welding Design) / CAD avançado A: Weld Symbol (MBD) / CAD avançado C: Weld symbols (desenho) / Fenkra: `Weld`)

- **O que faz:** indica o tipo, dimensão e extensão da solda numa junta, segundo ISO 2553 ou AWS A2.4.
- **CAD avançado B — como funciona no detalhe:** [C] CAD avançado B tem símbolos de solda no desenho e módulos próprios de soldagem; não confirmei uso no FT&A nesta rodada.
- **CAD avançado A — no detalhe:** [V] O MBD aceita símbolos de solda em peças e **montagens**.
- **CAD avançado C — no detalhe:** [C] Símbolos de solda ficam no desenho; no 3D, a anotação de solda é limitada. Não confirmei.
- **Outros:** [V] ISO 2553 permite dois sistemas: **A** (linha cheia = lado da seta, tracejada = outro lado) e **B** (como AWS). A edição de 2019 reconhece os dois. [V] O processo vai na cauda pelo código ISO 4063 (ex.: 135 = MAG). [V] ISO usa garganta *a* e AWS usa perna *z* (fontes de blog; conferir na norma). [V] No AP242, o PMI semântico cobre dimensão, tolerância, datum e acabamento; não encontrei **símbolo de solda semântico** nas fontes lidas.
- **Fenkra hoje:** `Weld { kind, size, all_around, extra }` com 11 tipos, ISO 2553 sistema A (`pmi.rs:85-342`). `weld_symbol` desenha linha de referência, tracejada, símbolo dos dois lados, bandeira de campo, contorno (plano, convexo, côncavo), comprimento, cauda com processo. Há testes com nome (`simbolo_de_solda_iso_2553`, `pmi.rs:921`).
  - Faltam: **sistema B / AWS A2.4**, leitura de **cordão 3D** do módulo estrutural (a solda não conhece as peças soldadas), múltiplas setas (junta com duas peças), referência a WPS, e solda intermitente com passo estruturado (só string `length`).
  - O catálogo registra: "sem cordão 3D" (`catalogo.md:844`).
- **Melhor prática para o Fenkra:**
  - O alvo da solda passa a ser a **junta** (duas faces/arestas) e, no módulo estrutural, o cordão. A seta toca a junta.
  - Escolha de sistema (A/B) e unidades (garganta/perna) pela norma ativa.
  - Gerar PMI de solda a partir do cordão (alinhado com `docs/contextos/estrutural.md`, "PMI3DGenerator": notas de solda por grupo estrutural).
  - Casos de teste: filete a5 de um lado só; X dos dois lados; solda de campo; mudar a norma ativa para AWS troca o lado da seta mantendo a mesma solda.

### F12. Planos de anotação e orientação do texto (CAD avançado B: View/Annotation Plane / CAD avançado A: Annotation View / CAD avançado C: View representation / CAD avançado F: Annotation Plane / Fenkra: `Pmi.plane`)

- **O que faz:** define o plano em que a anotação é desenhada e lida. A anotação aparece legível quando se olha de frente para esse plano.
- **CAD avançado B — como funciona no detalhe:**
  - [V] Tipos: **Front View** (eixo azul), **Section View** (eixo verde), **Section Cut**; também **Aligned** (perfil de sketch só com linhas não perpendiculares) e **Offset** (só linhas perpendiculares ou paralelas, segmentos inicial e final paralelos).
  - [V] Os planos aparecem tracejados no 3D com origem e eixos. O **plano ativo** recebe as anotações novas. Se a anotação não couber nele, o CAD avançado B pergunta se deve criar um plano.
  - [V] Regra de posição: o plano **deve cruzar a geometria**. Nas vistas frontais, as anotações vão em planos paralelos, na frente ou no fundo. Nas seções, vão no espaço de fundo limitado pelo plano.
  - [V] **Transfer To View/Annotation Plane** (menu de contexto) move a anotação; *Select Annotations* / *Activate View* (botão direito no plano).
  - [V] A vista de anotação alimenta o *View From 3D* do desenho (modo *Exact View*); anotações fora da vista geram "cruz vermelha" a menos que a opção seja desligada.
- **CAD avançado A — no detalhe:** [V] **Vistas de anotação** (*Front, *Right, *Top, Notes Area): as anotações aparecem e somem sozinhas conforme o modelo gira (anotações de vista dinâmicas). *Dynamic Annotation Views* está na barra do MBD.
- **CAD avançado C — no detalhe:** [V] As anotações 3D pertencem a *view representations*. Por padrão, uma anotação criada na vista ativa é adicionada a **todas** as design views desbloqueadas.
- **Outros:**
  - [V] CAD avançado F: o plano da anotação é perpendicular ao plano da feature e legível na vista atual; o usuário pode mudar o plano e a posição da chamada.
  - [V] CAD avançado G: nota "paralela à tela".
- **Fenkra hoje:** `plane: Option<(normal, x)>` por anotação (`pmi.rs:773`). `None` = sempre de frente para a tela. O menu de contexto alterna "de frente / deitar no plano da vista atual" (`pmi_ui.rs:683`, `734-747`). O painel oferece XY, XZ, YZ e "plano/face selecionado" (`pmi_ui.rs:1308-1317`).
  - O plano **não é um objeto**: cada anotação guarda sua própria normal. Não há "plano ativo" nem "transferir para o plano", nem agrupamento.
  - `create()` usa `offset` em coordenadas do mundo (`[0,0,12]`, `pmi_ui.rs:121-125`), sem relação com a normal da face.
  - Bug provável: `Linear` no plano calcula o sentido de leitura pela câmera atual (`pmi_ui.rs:522-527`): o texto pode virar de lado depois de girar a vista.
- **Melhor prática para o Fenkra:**
  - `AnnotationPlane { id, nome, origem, normal, x, tipo: Frontal|Seção|CorteDeSeção, profundidade_visivel }` como objeto na árvore, com "plano ativo". Anotações novas herdam o plano ativo e o `offset` é local ao plano.
  - Comandos: *Transferir para o plano*, *Ativar o plano*, *Selecionar as anotações do plano*.
  - Regra de visibilidade (como a vista de anotação do CAD avançado A): quando a câmera olha de frente para o plano ou quando a vista é capturada, as anotações aparecem; fora disso ficam esmaecidas ou ocultas por opção.
  - Casos de teste: criar plano frontal na face do topo, anotar, girar; transferir uma cota para outro plano; plano que não cruza o modelo avisa.

### F13. Capturas / vistas anotadas / estados (CAD avançado B: Capture / CAD avançado A: Capture 3D View / CAD avançado C: view representation / Fenkra: `PmiCapture`)

- **O que faz:** salva uma vista (câmera mais anotações visíveis mais estado do modelo) para reaparecer depois e ser publicada.
- **CAD avançado B — como funciona no detalhe:**
  - [V] Cria-se pelo ícone **Capture**, informando o nome. Isso abre o *Tolerancing Capture Workbench*.
  - [V] Opções: adicionar novas anotações à captura, associar uma câmera já criada, ativar *clipping plane* (que ativa o view state) e ativar a vista associada na exibição.
  - [V] Orienta-se o modelo (por exemplo, normal a um plano de anotação) ou usa-se uma *Named View*.
  - [V] *Show/Hide* e filtro escolhem as anotações visíveis. Para ver: botão direito na captura na árvore → *Display Capture*.
- **CAD avançado A — no detalhe:**
  - [V] *Capture 3D View*: nome único, **configuração**, **display state**, **vistas de anotação** e, opcionalmente, **vista de corte**. Guarda também vistas explodidas e vistas de quebra, cotas de referência, anotações DimXpert e posição de zoom.
  - [V] *Recapture View* atualiza; clicar direito renomeia ou apaga; o hover mostra configuração/estado/vistas.
  - [V] As vistas capturadas viram opções ao publicar (3D PDF, eDrawings, STEP 242).
- **CAD avançado C — no detalhe:** [V] As *view representations* controlam visibilidade e escala das anotações, com a câmera salva. Há *section view representations* para detalhar interiores. O filtro *3D Annotations Object Visibility* liga e desliga todas.
- **Outros:** [V] AP242 / CAD Exchanger: *SavedView* = nome, câmera e conjunto de elementos PMI visíveis. [V] O STEP 242 do CAD avançado A leva as vistas capturadas.
- **Fenkra hoje:** `PmiCapture` (`pmi.rs:783-797`): `view` (`NamedView`), `shown: Vec<Id>`, `hidden: Vec<Id>`, `with_model: bool`. Criar/atualizar/aplicar/apagar na árvore (`pmi_ui.rs:910-1024`).
  - Pontos fortes: guarda visibilidade do modelo (comandos, coletores, referências, componentes) e funciona em peça e montagem.
  - Faltam: **corte** (seção) na captura, configuração/estado de exibição, estado explodido, escala de anotação por captura, captura "ativa" (aplicar muda a visibilidade de verdade e **não reverte**), miniatura, nome editável.
  - Anotações criadas **depois** da captura não estão em `shown` e somem ao aplicar (comportamento esperado, mas sem aviso).
  - Bug provável: atualizar uma captura (`k == 1`) grava `c.hidden = hidden` mesmo quando `with_model == false` (`pmi_ui.rs:1002-1006`). Se o usuário depois ligar "guardar visibilidade do modelo", o estado antigo é substituído (ok), mas o ato de atualizar sem `with_model` já poluiu `hidden` com um estado que nunca será aplicado.
  - Bug provável: nome da captura = `Captura {n}` com `n = len+1` (`pmi_ui.rs:1000`): apagar a 1 e criar outra repete "Captura 2".
- **Melhor prática para o Fenkra:**
  - `PmiView { nome, camera, plano_de_anotacao?, corte?, anotacoes: Filtro|Lista, estado_modelo?, escala }`. Aplicar uma vista é **temporário** (modo "vista ativa") e voltar ao estado anterior é um clique.
  - Conjunto de vistas padrão por norma (frontal, topo, direita, isométrica, notas gerais).
  - Cada vista é publicável (STEP AP242, imagem, PDF).
  - Casos de teste: criar captura com corte e aplicar; voltar ao estado anterior; criar anotação depois e verificar se aparece (opção "novas anotações entram nesta vista"); nomes únicos.

### F14. Organização, filtros e visibilidade (CAD avançado B: Annotation Set / Filter / CAD avançado A: PMI Manager / CAD avançado C: Annotations folder, Select Annotations / Fenkra: pastas, `visible`)

- **O que faz:** agrupa, filtra e liga/desliga anotações, para que o modelo não vire uma sopa de textos.
- **CAD avançado B — como funciona no detalhe:** [V] O filtro de anotações aceita critério por **tipo** (não semântico), **subtipo** (texto), datums, datum targets e tolerâncias geométricas, por **elemento ou feature** e por **plano de anotação**. O padrão é ALL; também há busca por *Edit+Search*. [C] As anotações ficam sob um *Annotation Set* na árvore.
- **CAD avançado A — no detalhe:** [V] Gerenciamento por vista de anotação; "mostrar/ocultar anotações" por vista. Árvore DimXpert com os estados (+/−).
- **CAD avançado C — no detalhe:** [V] Pasta do navegador com realce cruzado; filtro *Select Annotations* na QAT, com seleção por janela e *All in Camera*.
- **Outros:** [V] CAD avançado G: regras lógicas de MBD aplicam e organizam anotações em lote.
- **Fenkra hoje:** `folder: String` por anotação (`pmi.rs:778`); pastas na árvore com mostrar, ocultar e desfazer (`pmi_ui.rs:860-882`); "mostrar todas / ocultar todas" (`pmi_ui.rs:901-908`); ocultar é por anotação (`visible`).
  - Faltam: filtro por tipo/plano/elemento, **conjuntos** de anotações (um Annotation Set por nível), seleção "todas as anotações de uma face", alternância por vista.
  - Ocultar uma anotação não aparece como estado de captura.
  - Apagar não pede confirmação e o menu de contexto remove direto (`pmi_ui.rs:722-728`); [C] não conferi se há desfazer.
- **Melhor prática para o Fenkra:**
  - Filtros combináveis (tipo, plano, vista, estado, elemento selecionado). "Selecionar anotações da face" a partir do clique na face.
  - Pasta vira atributo secundário. O agrupamento principal é por **plano** e **vista**.
  - Nome único automático (`max(n)+1`, não `count+1`).
  - Casos de teste: filtro "somente GD&T"; apagar e desfazer; nome sem colisão depois de apagar a do meio.

### F15. Estilo, norma ativa e escala de anotação (CAD avançado B: Styles/Standards, Annotation settings / CAD avançado A: Document Properties > Annotations / CAD avançado C: Active Standard, Annotation Scale / Fenkra: `PmiStyle`, `settings.pmi_style`)

- **O que faz:** define aparência (texto, setas, molduras), a norma (ISO/ASME) e a escala de leitura.
- **CAD avançado B — como funciona no detalhe:** [V] Cursos e guias listam configurações FTA de tolerância, exibição, geometria construída, manipuladores, dimensões, anotações e planos de vista. [V] Normas citadas nas notas: ASME Y14.41‑2003 e ISO/TC 213 WG 14.
- **CAD avançado A — no detalhe:** [V] Normas suportadas: MIL‑STD‑31000A, ASME Y14.41, ISO 16792 e DIN ISO 16792. [V] As opções de tolerância do DimXpert ficam nas propriedades do documento.
- **CAD avançado C — no detalhe:** [V] A unidade e o padrão das anotações 3D vêm de *Active Standard*. [V] **Annotation Scale** por vista (automático ou personalizado). [V] Atualização de estilos (`Manage > Update`) para a biblioteca 3DA.
- **Outros:** [V] Y14.41 (edições 2003, 2012, 2019, 2026) e ISO 16792:2006 (modelo apenas ou modelo mais desenho).
- **Fenkra hoje:** `PmiStyle` (`pmi.rs:547-617`) com mais de 25 campos e herança `over()` (anotação → grupo/vista → geral). `PmiStyle::apply_scale` ("o tamanho acompanha o zoom") existe. Janela de estilo geral e três predefinições (`pmi_ui.rs:1153-1178`).
  - **Não existe norma ativa.** Os símbolos, a vírgula decimal, a solda e as regras de GD&T não mudam com ISO/ASME. Não há escala de anotação por vista.
  - `over()` tem um desvio: `nudge` não herda (`pmi.rs:650`), `bold` e `mirror` só ligam (`||`), então um estilo de anotação não consegue **desligar** um padrão geral em negrito. Bug provável de estilo.
- **Melhor prática para o Fenkra:**
  - `Norma { ISO, ASME, DIN, ABNT }` no documento, com perfil que define: separador decimal, símbolos, sistema de solda, caracteres de GD&T válidos, texto de modificadores e unidades.
  - Estilo por **tipo** (cota, GD&T, nota…) além do geral.
  - Casos de teste: trocar de norma muda decimal e GD&T válido; desligar negrito herdado.

### F16. Realce e chamadas inteligentes (CAD avançado B: Highlight de geometria associada / CAD avançado A: Leaders / CAD avançado C: Leaders / Fenkra: `highlight`, `leader`)

- **O que faz:** mostra o que a anotação marca (face, aresta) e liga a anotação ao elemento por chamadas sem cruzar o próprio quadro.
- **CAD avançado B — como funciona no detalhe:** [C] A seleção de uma anotação na árvore realça o elemento tolerado; a seleção do elemento realça as anotações.
- **CAD avançado A — no detalhe:** [V] Chamadas com texto personalizado e posição (opção *Custom Text Position* na aba Leaders). [V] A seleção do elemento destaca a anotação associada.
- **CAD avançado C — no detalhe:** [V] **Realce cruzado** entre a janela gráfica e o navegador. Para o Tolerance Feature, *Align to Geometry* orienta a chamada por aresta ou eixo.
- **Outros:** [V] CAD avançado F: selecionar a feature na árvore realça a anotação.
- **Fenkra hoje:** `highlight` (`pmi_ui.rs:345-373`) acha vértice, meio de aresta ou face sob cada âncora. `leader` (`pmi_draw.rs:402-426`) sai do lado do quadro mais próximo, com patamar. As âncoras viram **alças arrastáveis** (`pmi_ui.rs:617-646`): soltar numa face, aresta ou vértice reancora.
  - Faltam: **várias chamadas** por anotação (DR-48; B23), realce reverso (selecionar o elemento realça as anotações), realce na montagem (usa a malha da peça).
  - A reancoragem do rótulo compensa o `offset` apenas para a âncora 0 (`pmi_ui.rs:697-701`). Reancorar as outras move o rótulo.
- **Melhor prática para o Fenkra:**
  - Realce resolvido pelos **alvos topológicos** (não por "ponto sob a âncora"). O realce funciona na peça e na montagem (malha do componente certo).
  - Seleção reversa: clicar numa face destaca todas as anotações que a usam.
  - Várias chamadas (uma por alvo, mais derivadas) e chamada que nunca atravessa o quadro nem o próprio texto.
  - Casos de teste: anotação na montagem realça a face do componente certo; selecionar a face mostra a lista de anotações; GD&T de posição de padrão com setas para cada furo.

### F17. Legenda 3D, balões e tabelas (CAD avançado B: Text/Table, 3D Master / CAD avançado A: Tables, BOM in MBD / CAD avançado C: Notes / Fenkra: `TitleBlock`, `Balloon`)

- **O que faz:** blocos informativos no modelo (legenda com campos da peça, balões de lista de peças).
- **CAD avançado B — como funciona no detalhe:** [C] Textos com parâmetros e tabelas; ligação por parâmetros de conhecimento. [V] As fórmulas de conhecimento (*knowledge formulas*) em tolerâncias têm tópico próprio.
- **CAD avançado A — no detalhe:** [V] O MBD inclui **BOM** e tabelas em 3D, e permite PMI de montagem (cotas, soldas, BOM).
- **CAD avançado C — no detalhe:** [C] Balões e listas ficam no desenho.
- **Outros:** [V] CAD avançado G: regras de MBD, template de campos.
- **Fenkra hoje:** `TitleBlock { template }` com campos `{nome}`, `{codigo}`, `{descricao}`, `{material}`, `{massa}`, `{volume}`, `{data}`, `{autor}` e propriedades personalizadas (`pmi_ui.rs:313-342`). Na montagem, material/massa/volume vêm como "—". **Balões automáticos** a partir da lista de peças, escolhendo a face mais voltada para a câmera (`pmi_ui.rs:140-230`).
  - Bugs prováveis: balões automáticos refazem tudo e apagam balões editados à mão (`retain` em `pmi_ui.rs:204`). A massa de montagem é "—". Os balões não funcionam em item estrutural/lista de corte (DR-33).
  - Falta: tabela de lista de peças 3D, tabela de furos, linha de revisão.
- **Melhor prática para o Fenkra:**
  - Balões automáticos **incrementais** (não apagam os manuais; só criam os que faltam e atualizam os números).
  - Massa e material da montagem calculados pela soma dos componentes.
  - Casos de teste: refazer balões preserva uma nota manual no balão; adicionar componente cria só um balão novo; massa da montagem = soma.

### F18. Consultor e validação do esquema de tolerâncias (CAD avançado B: Tolerancing Advisor, Check Report / CAD avançado A: Show Tolerance Status, DimXpert / CAD avançado C: Tolerance Advisor / CAD avançado F: GD&T Advisor / CAD avançado G: PMI Advisor / Fenkra: ❌, B38)

- **O que faz:** verifica se as informações de fabricação estão completas e coerentes: faces restringidas, DRF completo, valores consistentes.
- **CAD avançado B — como funciona no detalhe:** [V] O Tolerancing Advisor **impede** tolerâncias inválidas (às custas de um processo mais lento, segundo a apostila NIAR). [V] O índice cita *Generating Check Report*. [V] Mostra "somente as tolerâncias permitidas" para a seleção.
- **CAD avançado A — no detalhe:**
  - [V] **Show Tolerance Status** (aba MBD): faces **verdes** = totalmente restringidas, **amarelas** = sub-restringidas, **vermelhas** = sobre-restringidas, cor original = sem dados GD&T. Na árvore: preto = restringido, azul com (−) = sub, vermelho com (+) = sobre. As cores se mudam em *Tools > Options > System Options > Colors*.
  - [V] "O que importa é que haja informação suficiente na vista 3D para fabricar a peça" (restringir ≠ definir completamente). [C] Causas e correções não explicadas na fonte lida.
  - [V] Aplicar o esquema automático valida o conjunto de datums contra ASME Y14.5.1M‑1994.
- **CAD avançado C — no detalhe:**
  - [V] O **Tolerance Advisor** é um painel do navegador (liga/desliga em *Annotate > Manage > Tolerance Advisor*). Cada mensagem tem ícone; clicar direito → *More Information* abre a ajuda com a solução.
  - [V] **Face Status Coloring** (desde a 2019): mostra no modelo o estado de restrição de cada face.
  - [V] A lista de mensagens tem 39 títulos. Os mais úteis para uma primeira versão: *Datum object is missing*; *DRF failed / invalid / not referenced / specification incomplete / may have poor repeatability*; *Form tolerance value must be smaller*; *Geometric tolerance is over-constraining*; *Insufficient opposing region to be a feature of size*; *Location / Orientation / Size / Form not fully constrained*; *Not all part DOF are constrained*; *One or more surfaces are unconstrained*; *Orientation of secondary/tertiary datum is not allowed*; *Pattern is not valid*; *Primary datum feature may be too small*; *Should be set as a datum feature or reordered*; *Target reference is missing*; *Tolerance Feature failed*; *Tolerance Feature and modeling occurrence count do not match*; *Tolerance feature shares faces*; *Tolerance specification is incomplete*; *Tolerance value must be smaller than preceding value*; *Unspecified tolerances for some dimensions*; *Zero-value (or inconsistent) tolerance not allowed*; *System of units not compatible with tolerancing standard*; *Model contains external annotation elements*.
- **Outros:**
  - [V] CAD avançado F: **Advisor Tree** com as pendências (graus de liberdade não restringidos, superfícies sem restrição). **Constraint State Display**: verde = restringida por feature definida, cinza = sem referência, azul = coberta pela nota de perfil geral. O estado é sincronizado em três momentos: ao abrir o Advisor, ao regenerar e ao clicar *Update*.
  - [V] CAD avançado G: **PMI Advisor** valida conformidade com normas da indústria e da empresa, e aceita **verificações personalizadas** em versões novas.
  - [V] CAD avançado C e outros: análise de empilhamento (*Tolerance Analysis*) depois das tolerâncias.
- **Fenkra hoje:** não existe. O catálogo tem BM‑PMI‑01 / PMI‑05 e o backlog B38 (P3, 120k tokens): "verificar se a peça está totalmente cotada e tolerada".
- **Melhor prática para o Fenkra:**
  - Três camadas de verificação, em ordem de custo:
    1. **Sintática** (por anotação, sempre ligada): valor numérico, tolerância < valor, DRF existe, datum com letra única, Ⓜ só em feature de tamanho.
    2. **De esquema** (por peça, sob demanda): DOF do DRF predominante, faces sem requisito, cotas sem tolerância, quadros que compartilham faces.
    3. **De norma** (perfil ASME/ISO): características removidas, unidade incompatível, símbolo não aceito.
  - Saída: lista de mensagens com severidade (erro, aviso, info), alvo clicável (realça o elemento) e **colorir faces** (verde/amarelo/vermelho/cinza) como a visão de estado do CAD avançado A/CAD avançado F.
  - Regra de ouro (do CAD avançado A): "completo" é uma meta opcional. Oferecer o modo "tudo explícito" e o modo "mínimo para fabricar".
  - Casos de teste: bloco com DRF A|B|C e tolerância de forma em todas as faces fica todo verde; remover o datum B deixa a peça amarela com a mensagem "DOF não restringidos"; tolerância de forma maior que a de tamanho gera aviso; duas tolerâncias na mesma face geram "sobre-restringida".

### F19. Esquema automático e regras de PMI (CAD avançado A: Auto Dimension Scheme / CAD avançado G: MBD logical rules / CAD avançado F: Auto datum / Fenkra: ❌, `docs/contextos/estrutural.md` §3.1.6)

- **O que faz:** gera cotas, tolerâncias e datums de uma só vez, a partir de uma regra ou assistente.
- **CAD avançado B — como funciona no detalhe:** [C] O FT&A tem geração semi-automática por *3D annotations on knowledge*; não confirmei um equivalente do Auto Dimension Scheme. [V] "Anotações 3D com knowledge" está no índice do curso.
- **CAD avançado A — no detalhe:**
  - [V] *Auto Dimension Scheme* (DimXpertManager ou *Tools > MBD Dimension*). Passos: escolher configurações de dimensão, definir as **features de referência** (datum primário, secundário, terciário), selecionar as features, revisar o resultado.
  - [V] Configurações: tipo de peça (exemplo da fonte: *Turned*; o texto de busca cita *Prismatic*), tipo de tolerância (*Plus and Minus* ou *Geometric*), tipo de dimensão de padrão (exemplo: *Linear*).
  - [V] O esquema geométrico impõe a sequência: planeza no primário; perpendicularidade no secundário em relação a A; posição no terciário em relação a A e B; e usa os valores padrão das propriedades do documento.
  - [V] Dicas: clicar com o botão direito aceita cada datum; o botão OK roda o esquema.
- **CAD avançado C — no detalhe:** [C] Não tem esquema automático equivalente nas fontes lidas; o fluxo é o Tolerance Feature interativo mais Advisor.
- **Outros:**
  - [V] CAD avançado G: **MBD logical rules** na biblioteca de reutilização, organizadas em pastas (Dimensions, Datums, Feature Control Frames); duplo clique na regra e responder aos prompts aplica as anotações. Isso mantém a consistência entre projetos.
  - [V] CAD avançado F: atribuição automática de datums para as primeiras features que restringem a peça.
- **Fenkra hoje:** ❌. Só "balões automáticos" na montagem (`pmi_ui.rs:140`). Os contextos do usuário pedem geração automática de PMI por peça e por grupo estrutural (`estrutural.md`, "PMI3DGenerator": vistas 3D, cotas, notas, símbolos, metadados; para grupos: vistas de conjunto, notas de solda, identificadores).
- **Melhor prática para o Fenkra:**
  - Dois níveis:
    1. **Assistente de DRF e esquema** (para uma peça): escolher A, B, C → gera planeza/perpendicularidade/posição sugeridas, valores padrão do documento, e deixa tudo editável e no estado "gerado".
    2. **Regras de PMI** reutilizáveis (por tipo de feature): "furo passante → Ø com tolerância H12 e nota de rosca".
  - Tudo que for gerado fica marcado com a origem (regra) para poder **regenerar** sem apagar edições manuais (mesma lógica dos balões incrementais).
  - Casos de teste: bloco com 4 furos recebe cotas e posição ligada a A|B|C; mudar um furo atualiza; regenerar não duplica.

### F20. PMI em montagem e herança (CAD avançado B: Product-level annotations / CAD avançado A: assembly MBD / Fenkra: `asm.pmi`)

- **O que faz:** anotações de nível de montagem (soldas, cotas entre componentes, balões, notas), e visibilidade das anotações das peças.
- **CAD avançado B — como funciona no detalhe:** [V] O curso cobre "anotações em montagens" (*Annotations in Assemblies*). [C] As anotações de peça continuam na peça e a montagem as exibe.
- **CAD avançado A — no detalhe:** [V] O MBD aplica PMI a partes e montagens, incluindo cotas e soldas no nível da montagem e BOM. [V] Há opções para esconder ou mostrar anotações da peça na montagem (o título da página da ajuda indica isso).
- **CAD avançado C — no detalhe:** [V] Anotações da montagem e Tolerance Features só em peças.
- **Outros:** [V] CAD avançado F: PMI de montagem com o mesmo Advisor.
- **Fenkra hoje:** cada nível tem sua lista (`list`/`list_mut`, `pmi_ui.rs:33-46`): `Assembly.pmi` e `Document.pmi`. Não há herança: as anotações da peça **não** aparecem na montagem, e as da montagem apontam para posições do mundo, sem componente-dono.
  - Âncora sem componente: mover o componente (mate) não move a anotação.
- **Melhor prática para o Fenkra:**
  - Alvo = (componente, chave topológica). A anotação acompanha o componente.
  - Opção "mostrar anotações das peças" na montagem, por componente. As anotações de peça seguem a transformação do componente.
  - Casos de teste: mover um componente com mate; anotação da peça aparece na montagem na posição correta; ocultar componente oculta suas anotações.

### F21. Exportação e importação STEP AP242 com PMI (CAD avançado A: Publish STEP 242 File / CAD avançado C: STEP export / CAD avançado G, CAD avançado F / Fenkra: `export_step`)

- **O que faz:** transmite o modelo e o PMI para outro sistema: CAM, inspeção, fornecedor.
- **CAD avançado B — como funciona no detalhe:** [C] CAD avançado B exporta STEP AP242 com PMI semântico e gráfico via conversor; não confirmei os detalhes nesta rodada.
- **CAD avançado A — no detalhe:**
  - [V] *Publish STEP 242 File* na barra MBD. As vistas 3D capturadas viram opções do arquivo. O 3D PDF pode anexar o STEP 242 (opção *Create and attach STEP 242*).
  - [V] Importação e exportação de AP242 nas Editions 1, 2 e 3. Importa PMI de CAD avançado F, CAD avançado G e CAD avançado B (CAD avançado B exige CAD avançado A Premium) com 3D Interconnect.
  - [V] Teste de usuários: o AP242 do MBD foi lido por CAD avançado F e CAD avançado A, mas **o CAD avançado E não lia** PMI de AP242 (fórum, 2023 ou antes).
- **CAD avançado C — no detalhe:** [V‑fraco] Uma ideia no fórum fornecedor de CAD avançado diz que só tolerâncias de forma e posição são exportadas como propriedades no STEP, e pede mais. [V] A importação de PMI gráfico existe desde a 2019.1 (opção *Graphic PMI*). [V] A aPriori lê tolerâncias semânticas de arquivos do CAD avançado C 2024 em diante.
- **Outros:**
  - [V] CAx‑IF Recommended Practices for PMI (AP242) v4.1, 20/06/2024 (157 páginas, com *Validation Properties for Semantic PMI Representation*, seção 10.1). O NIST **STEP File Analyzer** gera planilha de entidades, analisa PMI semântico/gráfico e propriedades de validação e checa erros de formato.
  - [V] Estrutura de dados do AP242: `geometric_tolerance` com subtipos por tipo de restrição e subtipos extras (datums, modificadores, valor máximo, unidade, disposição desigual); sistema de datum = lista ordenada de datums (cada um num *compartment*, com modificadores; "A‑B" = compartimento comum); o alvo da tolerância é `dimensional_size`, `dimensional_location`, `shape_aspect`, `product_definition_shape` ou relação de shape aspect.
  - [V] Apresentação gráfica pode ser **tesselada** (cada anotação quebrada em elementos de geometria).
  - [V] OCCT: o tratamento de GD&T segue as práticas recomendadas do CAx‑IF. A classe `XCAFDoc_DimTolTool` gerencia dimensões, tolerâncias e datums; o `STEPCAFControl_Writer` exporta o documento XDE. Relatos de fórum: precisa de `write.step.schema = AP242DIS` e `SetDimTolMode(true)`; para apresentação tesselada, `XCAFDoc_DimTolTool::SetGDTPresentations`. (fórum, 7.4 a 7.9)
- **Fenkra hoje:** `export_step` (`kernel.cpp:988-994`) usa `STEPControl_Writer.Transfer(s, STEPControl_AsIs)` e `Write`. Nada de XCAF, nem de AP242, nem de cor/nome. Chamado em `fenkra-solidos/src/lib.rs:5160` (STL se a extensão for `.stl`, senão STEP).
  - O PMI só existe no arquivo nativo (serde). Não há importação de PMI.
- **Melhor prática para o Fenkra:**
  - Etapa 1: exportar **PMI gráfico** (e vistas salvas) com XDE: a geometria dos símbolos já existe em vetores (`WeldPrim`, molduras) ou pode ser tesselada.
  - Etapa 2: **PMI semântico** (dimensão, tolerância geométrica, datum, DRF) ligado às faces por `shape_aspect`. Depende da camada semântica de F1, F7, F8 e F10.
  - Etapa 3: propriedades de validação (centroide, volume, área) e checagem com o NIST Analyzer e com a leitura no próprio Fenkra.
  - Marcar no diálogo: "PMI: nenhum / gráfico / gráfico + semântico".
  - Casos de teste: exportar corpus B53 e reler com o NIST Analyzer (sem erro de formato); reimportar no Fenkra e comparar contagem de PMI; abrir no FreeCAD/CAD Assistant [C] para ver o PMI.

### F22. Publicação e saída (CAD avançado A: Publish 3D PDF / eDrawings / Templates; CAD avançado B: 3D to 2D; CAD avançado C: 3D PDF / Fenkra: desenho 2D)

- **O que faz:** gera as saídas para quem não tem o CAD: PDF 3D, visualizador, desenho 2D com as anotações do modelo, comparação de revisões.
- **CAD avançado B — como funciona no detalhe:** [V] O *View From 3D* do Drafting extrai a vista de anotação (modo *Exact View*, mesmo padrão ISO/ASME no CATPart e no CATDrawing). [V] A apostila lista a barra **Reporting** [V‑fraco].
- **CAD avançado A — no detalhe:**
  - [V] *Publish to 3D PDF*, *Publish to eDrawings*, *Publish STEP 242*. *3D PDF Template Editor* para os modelos de saída (por departamento: operações, manufatura, qualidade, compras).
  - [V] *3D PMI Compare* na barra do MBD (comparar PMI entre versões; não li o detalhe).
  - [V] Importação de PMI para **desenhos 2D**: coloca as vistas 3D com as cotas DimXpert numa folha.
- **CAD avançado C — no detalhe:** [C] Publicação em 3D PDF e STEP; não li os detalhes.
- **Outros:** nenhum relevante.
- **Fenkra hoje:** `fenkra-desenho` usa `PmiStyle`, `GdtChar`, `GdtExtra`, `WeldKind` e `WeldExtra`, mas **não importa a lista de PMI** do modelo para o desenho (apenas os tipos). Cotas do desenho são dos desenhos.
- **Melhor prática para o Fenkra:**
  - Etapa 1: *Recuperar anotações do modelo* na vista de desenho (com filtro por vista de anotação).
  - Etapa 2: exportação de imagem/PDF da vista capturada.
  - Etapa 3: 3D PDF [C: fora do alvo imediato, depende de bibliotecas].
  - Casos de teste: a vista frontal do desenho mostra as cotas da vista de anotação frontal; mudar a cota no 3D atualiza o desenho.

### F23. Atualização e diagnóstico quando a referência some (CAD avançado B: Update Diagnosis / CAD avançado A: What's Wrong / CAD avançado C: Tolerance Advisor, Design Doctor / CAD avançado F: Update, Manage Annotations / Fenkra: ❌)

- **O que faz:** quando a geometria muda, cada anotação decide: continua, se reanexa ou falha, e o usuário vê, corrige e reordena.
- **CAD avançado B — como funciona no detalhe:** [V] "On Update: 2D geometry and annotations are modified" para a extração 2D. [V] *Geometry Connection Management* e *Geometry for 3D Annotations* ajudam a gerir ligações. [C] O estado "isolada" e o *Update Diagnosis* da ADR 0011 são nomes do CAD avançado B.
- **CAD avançado A — no detalhe:** [V] Se uma cota combinada fica ambígua, o usuário quebra a chamada e a tolerância continua na feature certa. [V] A ferramenta *Recreate basic dim* **repara** o esquema de cotas básicas de uma tolerância de posição. [V] A atualização de uma vista capturada é manual (*Recapture View*).
- **CAD avançado C — no detalhe:** [V] Mensagens de estado: *Tolerance Feature Failed*, *Geometry references not valid*, *Failed objects in the model*, *Tolerance Feature and modeling occurrence count do not match* (quantidade de ocorrências do padrão mudou), *Dimension reference feature is not valid*.
- **Outros:** [V] CAD avançado F: a atualização roda ao abrir o Advisor, ao regenerar e ao clicar *Update*. Conflitos aparecem na janela *Manage Annotations* ou como mensagens do Advisor. Um padrão ligado segue o padrão CAD; um não ligado não atualiza sozinho.
- **Fenkra hoje:** ❌. `snap` aceita qualquer coisa. Não há estado de falha. A ADR 0010 trata disso para filetes, chanfros etc., mas o PMI não usa `FaceKey/EdgeKey`.
- **Melhor prática para o Fenkra:**
  - Pipeline por recompute: resolver alvos → recalcular medida → validar → atribuir `status`. Falha não apaga.
  - Painel de diagnóstico: **Editar**, **Substituir alvo**, **Isolar** (manter a anotação sem ligação), **Desativar**. É a mesma lista da ADR 0011.
  - Cada anotação mostra o estado na árvore (ícone). Ao abrir o arquivo, anotações perdidas são contadas na barra de status.
  - Casos de teste (regressão "muda o perfil, a anotação continua"): mudar a espessura; remover a face; dividir a face em duas (booleana); trocar a ordem das features; reabrir o arquivo antigo (promoção das âncoras antigas para chaves no primeiro recompute bem-sucedido).

---

## 3. Requisitos

**Prioridades:** P0 = corrige resultado errado ou perda silenciosa. P1 = fundamento da filosofia. P2 = paridade importante. P3 = evolução.

### 3.1 Funcionais (RF-PMI)

| ID | Requisito | Prior. | Critério de aceite |
|---|---|---|---|
| RF-PMI-01 | A cota entre duas faces paralelas mede a distância perpendicular entre as **faces**, não entre os pontos clicados. | P0 | Placa de 20 mm de espessura: cota face-face = 20. Mudar para 25: a cota passa a 25 sem editar. |
| RF-PMI-02 | Raio/diâmetro seguem o furo: o valor e o centro se recalculam quando o furo muda de lugar ou de tamanho. | P0 | Furo Ø10 → Ø12 e deslocado 5 mm: a cota mostra Ø12 e o centro acompanha. |
| RF-PMI-03 | Toda anotação tem `status` (OK, Isolada, Ambígua) visível na árvore e na viewport. | P0 | Apagar a face-alvo: a anotação fica "Isolada" (ícone e texto) e continua no arquivo. |
| RF-PMI-04 | Os alvos das anotações são chaves topológicas (dono → papel → assinatura, ponto como reserva), como a ADR 0010. | P1 | Teste "muda o perfil, a anotação continua": alterar o perfil do sketch mantém a cota na mesma face. |
| RF-PMI-05 | O `snap` de 2 mm é substituído por resolução de chave. Sem chave resolvida, nunca escolher outra geometria. | P0 | Mover uma aresta 10 mm: a anotação não pula para o vértice vizinho. |
| RF-PMI-06 | Em montagem, o realce e a resolução usam o componente-dono do alvo. | P0 | Anotação em face de um componente movido por mate realça a face correta. |
| RF-PMI-07 | Painel de diagnóstico de PMI: Editar, Substituir alvo, Isolar, Desativar, com candidatos realçados no 3D. | P1 | A partir de uma anotação isolada, "Substituir" permite clicar em outra face e reabilita a anotação. |
| RF-PMI-08 | Modos da cota linear: face-face, ponto-ponto, ao longo de eixo (X, Y, Z ou eixo local), projetada no plano de anotação, e eixo-face. | P1 | Cada modo tem um caso no corpus; o valor muda quando a projeção muda. |
| RF-PMI-09 | Precisão (casas decimais) e separador decimal configuráveis por documento e por norma, sem corte de zeros quando houver tolerância. | P1 | 25,40 ±0,05 aparece com duas casas; ASME mostra ponto. |
| RF-PMI-10 | Tolerância tipada (simétrica, bilateral, limites, ajuste ISO 286, básica, referência), com validação numérica. | P1 | "abc" é recusado; `Ø20 H7` mostra +0,021/0. |
| RF-PMI-11 | Cota angular entre duas arestas/faces/eixo-face, com modos interno/externo/complementar. | P2 | Chanfro de 45° medido entre faces dá 45; trocar para complementar dá 135. |
| RF-PMI-12 | Cota de furo lê a feature de furo (diâmetro, profundidade, rebaixo, rosca, quantidade do padrão). | P2 | Padrão de 4 furos M8: nota `4× M8×1,25 ▽12`; mudar para 6 atualiza a quantidade. |
| RF-PMI-13 | Objeto **Datum** com letra única, tipo (feature de tamanho ou superfície) e alvo. Letras I, O e Q são recusadas na norma ASME [C]. | P1 | Criar duas vezes "A" gera erro; renomear "A" para "D" atualiza os quadros que a usam. |
| RF-PMI-14 | Objeto **DRF** (primário, secundário, terciário, modificadores) nomeado na árvore; tolerâncias referenciam o DRF, não texto. | P1 | Apagar o datum B marca o DRF como inválido e avisa cada tolerância dependente. |
| RF-PMI-15 | Alvos de referência (datum targets): ponto, linha, área, com símbolo e numeração. | P2 | Criar três alvos pontuais no datum A mostra `A1 A2 A3`. |
| RF-PMI-16 | O quadro de tolerância oferece apenas características válidas para o elemento (Advisor). | P1 | Em face plana: forma de plano, perfil, orientação, posição; nunca cilindricidade. |
| RF-PMI-17 | Matriz de validade por característica (exige datum? aceita modificador? aceita Ø?). | P1 | Planeza com datum é recusada; perpendicularidade sem DRF é recusada. |
| RF-PMI-18 | Valor da tolerância numérico, forma da zona (Ø, largura, esfera), por unidade de comprimento. | P1 | `0,05/25` é aceito em planeza; texto livre é recusado. |
| RF-PMI-19 | Modificadores como enum, com compatibilidade por característica; modificador por datum. | P1 | Ⓜ em planeza é recusado; `C Ⓜ` é aceito em posição. |
| RF-PMI-20 | Perfil dinâmico, perfil "todo o contorno/ todo o entorno", zona comum (CZ), "entre A↔B". | P2 | Cada símbolo desenha e sai no STEP gráfico. |
| RF-PMI-21 | Norma ativa (ISO 1101 / ASME Y14.5‑2018 / DIN / ABNT) define símbolos, características válidas, decimal, solda. | P1 | Em ASME 2018, concentricidade e simetria geram aviso; em ISO, são aceitas. |
| RF-PMI-22 | Símbolos de GD&T desenhados como vetores próprios, sem depender de glifos de fonte. | P1 | Os 14 símbolos aparecem corretos num sistema sem as fontes Unicode (captura de tela de referência). |
| RF-PMI-23 | Rugosidade estruturada: parâmetro(s), limites, método, lay, sobremetal, requisito de remoção, norma (ISO 1302/21920, ASME). | P2 | Símbolo com Ra 1,6 e Rz 6,3; "remoção proibida" desenha o círculo. |
| RF-PMI-24 | Rugosidade geral na legenda ("exceto indicado"). | P3 | A legenda lista o símbolo geral e as faces sem requisito próprio. |
| RF-PMI-25 | Solda: sistema B / AWS A2.4, junta como alvo, cordão do módulo estrutural, setas múltiplas. | P2 | Trocar a norma para AWS inverte o lado da seta mantendo a mesma solda. |
| RF-PMI-26 | Planos de anotação como objetos (frontal, seção, corte), com plano ativo e transferência de anotação. | P1 | Criar plano na face do topo, anotar, "transferir para o plano X" move a anotação. |
| RF-PMI-27 | `offset` das anotações local ao plano de anotação (não em coordenadas do mundo). | P1 | Anotações criadas em faces diferentes aparecem afastadas da própria face. |
| RF-PMI-28 | Vistas anotadas (`PmiView`): câmera, plano, corte, filtro de anotações, estado do modelo, escala; aplicar é reversível. | P1 | Aplicar vista e "voltar" restaura visibilidade e câmera anteriores. |
| RF-PMI-29 | Anotações novas entram opcionalmente na vista ativa; nomes de vistas e anotações únicos. | P2 | Apagar "Captura 1" e criar outra não repete nomes. |
| RF-PMI-30 | Filtros de anotação por tipo, plano, vista, estado e elemento; "selecionar anotações desta face". | P2 | Clicar numa face seleciona as 3 anotações que a usam. |
| RF-PMI-31 | Várias chamadas por anotação; a chamada nunca atravessa o quadro (DR‑47, DR‑48). | P1 | GD&T de posição de padrão com setas para 4 furos. |
| RF-PMI-32 | Balões automáticos incrementais (preservam os manuais) e massa/material de montagem na legenda. | P2 | Refazer balões mantém uma nota manual; massa de montagem = soma. |
| RF-PMI-33 | Camada de validação sintática (sempre ligada) por anotação. | P1 | Tolerância ≥ valor da cota, DRF inexistente e letra repetida geram aviso em tempo real. |
| RF-PMI-34 | Consultor de tolerâncias da peça (B38): graus de liberdade do DRF, faces sem requisito, cores verde/amarelo/vermelho/cinza. | P2 | Bloco totalmente tolerado fica verde; remover o datum B deixa amarelo com mensagem. |
| RF-PMI-35 | Mensagens do consultor com severidade, alvo clicável e ajuda ("mais informações"). | P2 | Clicar na mensagem realça o elemento. |
| RF-PMI-36 | Assistente de DRF/esquema (inspirado no Auto Dimension Scheme): gera planeza, perpendicularidade e posição sugeridas, marcados como "gerados". | P3 | Bloco com 4 furos recebe o esquema; regenerar não duplica. |
| RF-PMI-37 | Regras de PMI reutilizáveis por tipo de feature (inspiradas nas MBD logical rules do CAD avançado G). | P3 | Regra "furo passante" aplicada em 5 furos cria 5 cotas e uma nota. |
| RF-PMI-38 | PMI de montagem acompanha o componente; opção de mostrar as anotações das peças na montagem. | P1 | Mover componente move a anotação; ocultar componente oculta a anotação. |
| RF-PMI-39 | Exportação STEP via XDE (`STEPCAFControl_Writer`), esquema AP242, com nomes, cores e **PMI gráfico + vistas salvas**. | P2 | O NIST STEP File Analyzer lê o arquivo sem erro e lista as anotações. |
| RF-PMI-40 | Exportação do **PMI semântico** (dimensão, tolerância, datum, DRF) ligado a `shape_aspect` das faces. | P2 | O NIST Analyzer reporta as tolerâncias e as ligações com as faces. |
| RF-PMI-41 | Importação de PMI de AP242 (gráfico no mínimo). | P3 | Abrir um STEP com PMI cria as anotações no Fenkra. |
| RF-PMI-42 | Recuperar anotações do modelo no desenho 2D, por vista de anotação. | P2 | A vista frontal do desenho mostra as cotas da vista de anotação frontal e atualiza com o modelo. |
| RF-PMI-43 | Propriedades de validação no STEP (volume, área, centroide) para PMI semântico. | P3 | Valores batem com os do Fenkra dentro de 1e-6 relativo. |
| RF-PMI-44 | Confirmação (ou desfazer) para apagar anotações. | P1 | Apagar via menu e desfazer (Ctrl+Z) restaura. |
| RF-PMI-45 | Herança de estilo corrigida: campos booleanos podem ser desligados na anotação; estilo por tipo. | P3 | Estilo geral em negrito e anotação com negrito "desligado" mostra sem negrito. |

### 3.2 Não funcionais (RNF-PMI)

| ID | Requisito | Prior. | Critério de aceite |
|---|---|---|---|
| RNF-PMI-01 | Nenhuma anotação some, muda de alvo ou muda de valor por heurística silenciosa. | P0 | Teste de fuzz de edição: toda anotação fica OK ou com `status` de falha explícito. |
| RNF-PMI-02 | Compatibilidade com arquivos antigos: âncoras de ponto são promovidas a chaves no primeiro recompute bem-sucedido. | P0 | Corpus de arquivos alpha.27 abre; anotações mantêm valor; salvar grava chaves. |
| RNF-PMI-03 | Recompute de PMI não passa de 50 ms para 200 anotações em peça de 5.000 faces. | P2 | Benchmark no corpus B53. |
| RNF-PMI-04 | O corpus B53 inclui peças com PMI e roda a cada commit: recompute, mudar e voltar cada cota, salvar e abrir, captura visual. | P1 | CI falha se qualquer anotação do corpus mudar de valor após "muda e volta". |
| RNF-PMI-05 | Cada símbolo de GD&T, solda e rugosidade tem teste de desenho com nome próprio. | P1 | Testes `simbolo_*` para os 14 símbolos e os tipos de solda. |
| RNF-PMI-06 | Textos e mensagens em português com chave de i18n (`tr`). | P2 | Nenhuma string de UI nova fica fora do `tr`. |
| RNF-PMI-07 | Formato de arquivo versionado para a nova camada semântica, com migração. | P1 | Teste de leitura de versões N−1 e N. |
| RNF-PMI-08 | Dados de PMI independem de fonte do sistema para exibição e exportação. | P1 | Captura com fontes mínimas é idêntica. |
| RNF-PMI-09 | Exportação STEP verificada por ferramenta externa (NIST Analyzer) em teste de integração. | P2 | Teste de integração passa com o analisador em modo linha de comando [C: verificar se existe]. |
| RNF-PMI-10 | Qualidade gráfica (B61) nunca altera o modelo; só a vista. As anotações não mudam de valor com a tesselação. | P1 | Medidas iguais em baixa e alta qualidade. |
| RNF-PMI-11 | Operações de PMI são desfazíveis (Ctrl+Z) como as demais do documento. | P1 | Criar, mover, apagar e editar PMI voltam um passo. |
| RNF-PMI-12 | Documentação do módulo (`docs/modulos/pmi-3d.md`) e catálogo atualizados com as fichas implementadas. | P3 | Paridade PMI‑01 a PMI‑0N atualizada no PR. |

---

## 4. Itens de backlog propostos

IDs provisórios (`N-PMI-nn`). Os existentes B23 (realce e chamadas) e B38 (consultor) são mantidos e referenciados.

| ID | Título | O que muda | Prior. | Tam. | Depende de |
|---|---|---|---|---|---|
| N-PMI-01 | Âncoras topológicas e medida por alvo | `Pmi.targets` com chaves da ADR 0010; `measure` por face-face, centro de furo e eixo; `snap` de 2 mm sai; migração de arquivos antigos | P0 | G | ADR 0010 fase 2, B53 |
| N-PMI-02 | Estado e diagnóstico de anotação | `status` OK/Isolada/Ambígua, ícones, painel Editar/Substituir/Isolar/Desativar, contagem na barra de status | P0 | M | N-PMI-01, B59 |
| N-PMI-03 | PMI na montagem com componente-dono | Alvo (componente, chave); realce e snap com a malha certa; anotações seguem mates | P0 | M | N-PMI-01, B54 |
| N-PMI-04 | Corpus de teste de PMI | Peças de referência com PMI; "muda e volta"; salvar e abrir; captura visual | P1 | M | B53 |
| N-PMI-05 | Modelo semântico de cota e tolerância dimensional | `Tolerance` tipada, precisão, separador decimal, ajuste ISO 286, cota básica e de referência, validação numérica | P1 | M | N-PMI-01 |
| N-PMI-06 | Modos da cota linear e cota angular entre elementos | Face-face, ao longo de eixo, projetada, eixo-face; ângulo entre arestas/faces; cota de furo lendo a feature | P1 | M | N-PMI-01 |
| N-PMI-07 | Datum e DRF como objetos | `DatumFeature`, `Drf`, validações (letra única, ordem, DOF), datum comum, tolerâncias referenciam DRF | P1 | M | N-PMI-01 |
| N-PMI-08 | Advisor de GD&T | Características filtradas pelo elemento, matriz de validade, zona (Ø, largura, esfera), modificadores enum e por datum, perfil dinâmico, "todo o contorno", CZ | P1 | G | N-PMI-07 |
| N-PMI-09 | Norma ativa (ISO/ASME) | Perfil de norma no documento: símbolos, decimal, características válidas, solda, mensagens | P1 | M | N-PMI-08 |
| N-PMI-10 | Símbolos vetoriais de GD&T | Substituir glifos de fonte por caminhos próprios (como `weld_symbol`); testes de desenho | P1 | P | — |
| N-PMI-11 | Planos de anotação como objetos | `AnnotationPlane` com plano ativo, transferir, `offset` local ao plano; corrige leitura invertida do texto | P1 | M | N-PMI-01 |
| N-PMI-12 | Vistas anotadas | Substituir `PmiCapture` por `PmiView` (câmera, plano, corte, filtro, escala, estado do modelo, aplicar reversível, nomes únicos) | P1 | M | N-PMI-11 |
| N-PMI-13 | Realce e chamadas (B23) | Realce reverso, várias chamadas, realce na montagem; absorve DR‑31/47/48 | P1 | M | N-PMI-01, N-PMI-03 |
| N-PMI-14 | Camada de validação sintática | Regras por anotação em tempo real (valores, tolerâncias, DRF, letras) | P1 | P | N-PMI-05, N-PMI-07 |
| N-PMI-15 | Consultor de tolerâncias da peça (B38) | Graus de liberdade do DRF, faces sem requisito, coloração de faces, mensagens clicáveis, perfil de norma | P2 | G | N-PMI-07, N-PMI-08, N-PMI-14 |
| N-PMI-16 | Exportação STEP AP242 pelo XDE | Trocar `STEPControl_Writer` por `STEPCAFControl_Writer`, esquema AP242, nomes, cores, PMI gráfico, vistas salvas | P2 | G | N-PMI-12 |
| N-PMI-17 | STEP AP242 com PMI semântico | `XCAFDoc_DimTolTool`: dimensão, tolerância, datum, DRF ligados a `shape_aspect`; validação com NIST Analyzer | P2 | G | N-PMI-16, N-PMI-05, N-PMI-07, N-PMI-08 |
| N-PMI-18 | Rugosidade estruturada | Parâmetros, limites, método, lay, sobremetal, requisito de remoção, norma, rugosidade geral na legenda; alvo = faces | P2 | M | N-PMI-01, N-PMI-09 |
| N-PMI-19 | Solda: sistema B/AWS e cordão | Sistema B, junta como alvo, geração a partir do cordão estrutural, setas múltiplas | P2 | M | N-PMI-09, módulo estrutural |
| N-PMI-20 | Recuperar anotações no desenho | *Model Items*: trazer o PMI da vista de anotação para a vista de desenho, com atualização | P2 | M | N-PMI-12, fenkra-desenho |
| N-PMI-21 | Balões incrementais e legenda de montagem | Preservar manuais, massa e material somados, balão de item estrutural (DR‑33) | P2 | P | — |
| N-PMI-22 | Filtros, seleção por face e desfazer | Filtro por tipo/plano/vista/estado, "anotações desta face", confirmação e desfazer na exclusão, nomes únicos | P2 | P | N-PMI-02 |
| N-PMI-23 | Datum targets e áreas restritas | Alvos pontual, linhas e áreas, símbolo e numeração, cotas enquadradas ligadas | P2 | M | N-PMI-07 |
| N-PMI-24 | Esquema automático e regras de PMI | Assistente de DRF e esquema; regras por feature; marcação "gerado" e regeneração incremental; PMI3DGenerator do estrutural | P3 | G | N-PMI-15, N-PMI-06 |
| N-PMI-25 | Importação de PMI de STEP e comparação | Ler PMI gráfico (e semântico) de AP242; comparar PMI entre revisões | P3 | G | N-PMI-17 |
| N-PMI-26 | Herança de estilo e estilo por tipo | Corrigir `over()` para campos booleanos; estilo por tipo de anotação | P3 | P | — |

Ordem sugerida: **N‑PMI‑01 → 02 → 03/04** (corrigir o que dá resultado errado), depois **05–07, 10, 11, 12**, depois **08/09/13/14**, então **15–19**, e por fim **20–26**.

---

## 5. Fontes

**CAD avançado B / plataforma PLM avançada**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://community.3dcs.com/help_manual/useful_fta_settings.htm (trecho de busca; a página retornou 403)
- (fonte em procedencia.md)
- https://www.wichita.edu/industry_and_defense/NIAR/Documents/3D-Tolerancing-and-Annotation-2022x.pdf (trecho de busca; a página retornou 404 no fetch)
- (fonte em procedencia.md) (trecho de busca)

**CAD avançado A MBD / DimXpert**
- https://centralinnovation.com/resources/auto-dimension-scheme
- https://www.engineersrule.com/tips-tricks-dimensioning-managing-hole-callouts-mbd/
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (via busca; conteúdo não renderizado)
- (fonte em procedencia.md) (via busca)
- (fonte em procedencia.md) (via busca)
- (fonte em procedencia.md) (via busca)

**CAD avançado C**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (via busca)

**CAD avançado F, CAD avançado G, CAD avançado E, Fusion**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (via busca)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (via busca)

**STEP AP242 / OCCT**
- https://www.mbx-if.org/home/wp-content/uploads/2024/06/rec_pracs_pmi_v41.pdf (PDF binário; só metadados e a busca)
- https://www.mbx-if.org/home/?p=1331 (NIST STEP File Analyzer; via busca)
- https://docs.cadexsoft.com/mtk/mtk_data_model_pmi
- https://downloads.steptools.com/docs/stix/stix_tolerance.html
- https://downloads.steptools.com/docs/stp_aim/html/t_geometric_tolerance_target.html (via busca)
- https://dev.opencascade.org/doc/occt-7.8.0/refman/html/STEPCAFControl__Writer_8hxx.html (via busca)
- https://dev.opencascade.org/content/missing-presentation-pmi-step-export-stepcafcontrolwriter (via busca)
- https://dev.opencascade.org/content/how-do-i-write-step-gdts (via busca)

**Normas**
- https://www.gdandtbasics.com/?p=5546 (ASME Y14.5‑2018 vs 2009)
- https://digitalsurf.com/blog/what-are-the-differences-between-iso-4287-and-iso-21920 (via busca)
- https://materialwelding.com/difference-between-iso-2553-and-aws-d1-1-welding-symbols/ (via busca)
- https://en.wikipedia.org/wiki/ASME_Y14.41 (via busca)
- https://iteh.es/catalog/standards/iso/4d1d542d-4c17-40be-beff-be3645fd4235/iso-16792-2006 (via busca)

**Código e documentação do Fenkra lidos:**
- `crates/fenkra-core/src/pmi.rs`
- `crates/fenkra-app/src/pmi_ui.rs`
- `crates/fenkra-app/src/pmi_draw.rs` (linhas 400–609)
- `crates/fenkra-kernel/cpp/kernel.cpp` (linhas 985–1003)
- `docs/arquitetura/adr/0010-referencias-v2.md`
- `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`
- `docs/benchmark/pmi-3d.md`
- `docs/modulos/pmi-3d.md`
- `docs/contextos/design-review.md` (DR‑31, DR‑33)
- `docs/contextos/estrutural.md` (§3.1.6)

---

## Handoff

- **Resultado:** relatório completo, com 23 fichas, 45 RF, 12 RNF e 26 itens de backlog propostos.
- **Evidência:**
  - Leitura integral de `pmi.rs` e `pmi_ui.rs`, mais `pmi_draw.rs` (400–609), `kernel.cpp` (985–1003) e as ADRs 0010 e 0011.
  - Consulta a ajudas e guias públicos de CAD avançado B, CAD avançado A, CAD avançado C, CAD avançado F, CAD avançado G, AP242 e OCCT, marcadas [V] ou [C] ao longo do texto.
- **Arquivos alterados:** nenhum.
- **Verificações executadas:** só leitura e pesquisa web. Nenhum teste rodou e nada foi compilado, então os "bugs prováveis" vêm da leitura do código e **não foram reproduzidos**.
- **Falhas e limites:**
  - As páginas da ajuda do CAD avançado A não renderizam sem JavaScript, e `CAD avançado Bdoc.free.fr` estava fora do ar.
  - Os PDFs do CAx‑IF e do guia V5R12 voltaram binários.
  - Não consegui confirmar a lista de mudanças da ISO 1101:2017 nem o uso de solda no FT&A do CAD avançado B.
- **Riscos:**
  - A classificação semântico/gráfico de rugosidade no CAD avançado B é contraditória na própria fonte.
  - Parte do comportamento do CAD avançado A e do CAD avançado C foi confirmada só por blogs de revendas e resumos de busca.
  - A ordem de dependências do backlog assume que a ADR 0010 fase 2 e o corpus B53 andam antes.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Maiores lacunas do PMI do Fenkra: âncoras por ponto com medida ponto a ponto (cotas erram em silêncio), ausência de semântica e vínculo datum↔face↔quadro, sem estado/diagnóstico/consultor de tolerâncias, sem STEP AP242 com PMI, e sem planos de anotação nem vistas anotadas completas.
