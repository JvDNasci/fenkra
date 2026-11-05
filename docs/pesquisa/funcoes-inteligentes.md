# Funções inteligentes de outros CADs aplicáveis às bancadas do Fenkra

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Pesquisa de 30/10/2026. Marcação: **[V]** = confirmado em fonte pública consultada nesta pesquisa; **[C]** = conhecimento geral não confirmado.
> Limite da coleta: as páginas de ajuda do CAD avançado A (help.CAD avançado A.com) não renderizam no WebFetch (só devolvem o cabeçalho do site). Para CAD avançado A, usei os resumos dos resultados de busca sobre essas páginas e blogs de revendas. Alguns blogs (Javelin, TriMech) devolveram HTTP 403.
> O que as pesquisas [sketch.md](sketch.md) e [montagem.md](montagem.md) já cobrem não é repetido por inteiro. Cada ficha aponta a seção e acrescenta o que é específico deste tema.

---

## 1. Resumo e principais lacunas

1. A "inteligência" dos CADs de referência cai em quatro famílias:
   - **inferir** a intenção (snaps, relações automáticas, SmartMates, Live Rules);
   - **declarar** a intenção (mate references, iMates, mate connectors, publicações, regras de chapa);
   - **reusar** a intenção (Power Copy, UDF, iFeature, configurações, famílias);
   - **automatizar** a intenção (fixadores inteligentes, Bolted Connection, Frame Generator, Design Accelerator, iLogic, Knowledge).
2. O CAD avançado B é forte em declarar e reusar (publicações, Power Copy, UDF, Design Table, Knowledge Advisor). É fraco em inferir: tem Smart Pick e Quick Constraint, mas pouca "mágica" ao arrastar. O Fenkra deve copiar a disciplina do CAD avançado B e a conveniência do CAD avançado A/CAD avançado C.
3. **Princípio de encaixe:** toda função inteligente deve produzir (a) um objeto visível na árvore (restrição, conjunto de fixadores, recurso de montagem), (b) com referências v2 ou publicadas (ADR 0010), (c) editável pelo mesmo assistente que o criou ("Editar com assistente", como no CAD avançado C), e (d) com estado de diagnóstico (ADR 0011, princípio 1). Inferência sem objeto persistente é aceitável só como pré-visualização.
4. **O que o Fenkra já tem de inteligente:**
   - cota inteligente com prévia (`Tool::Dimension`);
   - restrição rápida na montagem (`quick_options`);
   - alças "Fast 3D" (`handles.rs`);
   - preencher furos com fixadores (`preencher_furos`);
   - regras de chapa por material e espessura (`sheet_suggest`);
   - geradores de engrenagem, eixo e chaveta;
   - macros em texto;
   - reconhecimento geométrico leve (`geom_fit.rs`);
   - filtro de seleção (`Filter`).
5. **Maiores lacunas:**
   - não existem interfaces/mate references nomeadas nem conectores;
   - o sketch quase não infere (só H/V a 2,5°);
   - não existe "Completar sketch";
   - não existem configurações, tabelas de projeto ou famílias;
   - não existem Power Copy, UDF, templates, regras (iLogic/Knowledge) nem parâmetros de montagem de nível "regra";
   - não há edição direta, Live Section, reconhecimento de features nem seleção por intenção (laço, tangência, regras vivas);
   - não há busca de comandos.
6. O furo entre peças está em grande parte no B57/B58 e a restrição avançada no B36. Aqui só se registram os pontos que esses itens não cobrem.
7. Há riscos de bug por inferência silenciosa. Exemplos: `auto_kind` do Fenkra cai em "Coincidente" para qualquer par não plano×plano; o `AlignOnce` pode terminar em meio-caminho se o solver não convergir; a alça `Fast 3D` trava valores ligados a parâmetros em vez de oferecer editar o parâmetro.
8. Ordem sugerida: (1) interfaces nomeadas e restrição por arraste; (2) inferência e Completar sketch; (3) configurações e famílias; (4) Power Copy/UDF; (5) edição direta e seleção por intenção; (6) regras e automação.
9. O Fenkra tem uma vantagem estrutural: o `Document` é serializável e há um interpretador de macros (`macros.rs`). Isso permite implementar Power Copy, configurações e regras como dados e não como código, sem VBA/.NET.
10. Itens de menor custo e maior retorno: busca de comandos, seleção "laço/tangência/cadeia", `Completar sketch`, interfaces nomeadas, tabela de configurações simples.

---

## 2. Fichas por comando/função

### 2.1 Cota inteligente e "Completar sketch" (CAD avançado B: Constraint / Quick Constraint, Auto Constraint / CAD avançado A: Smart Dimension, Fully Define Sketch / CAD avançado C: General Dimension, Automatic Dimensions and Constraints / outros: CAD avançado E/Fusion Dimension / Fenkra: `Tool::Dimension`)

- **O que faz:** deduz o tipo de cota pela seleção e pelo movimento do mouse. Um segundo comando acrescenta as cotas e relações que faltam para chegar a DOF = 0.
- **CAD avançado B — no detalhe:**
  - A restrição rápida cria a primeira dimensão possível pela seleção. As demais saem do menu de contexto. O valor aceita fórmula [V, ver sketch.md 2.15].
  - O *Auto Constraint* examina a seleção e propõe restrições com referências H/V num diálogo [V parcial: tutorial GrabCAD].
  - A cota pode ser desativada e vira referência.
  - A saída fica na árvore como restrição editável, com ícone no sketch.
- **CAD avançado A — no detalhe:**
  - *Smart Dimension* escolhe o tipo pela seleção e pela posição do mouse.
  - *Fully Define Sketch* (Hawk Ridge) escolhe entre todas as entidades ou as selecionadas e tem esquemas de cotagem por cadeia, linha de base e ordenada. Tem *datum* externo (origem/aresta) ou interno, e o botão *Calculate* dá a prévia [V, sketch.md 2.20].
  - Funciona melhor com linhas e pontos; elipses saem mal.
- **CAD avançado C — no detalhe:** *Automatic Dimensions and Constraints* mostra o contador do que falta. Tem caixas separadas para cotas e restrições, aplica a seleção ou o sketch todo e o *Remove* apaga só o que foi automático [V].
- **Outros:** no CAD avançado E e no Fusion a ferramenta de cota também é única e deduz o tipo [C]. O destaque deles é o solver que "tenta mover o mínimo" [V, sketch.md 2.17].
- **Fenkra hoje:**
  - Existe a cota inteligente com prévia (acima/abaixo = horizontal, lado = vertical) [`sketch_ui.rs:586–607`].
  - `auto_dims` cria uma cota a cada elemento desenhado [`sketch_ui.rs:1566`, `settings.rs:323`] e está desligado por padrão.
  - O comando "Completar sketch" **não existe**.
  - Faltam: cota dirigida, nome da cota e cotas de arco (ver sketch.md 2.15).
- **Melhor prática para o Fenkra:**
  - Fluxo em dois comandos separados: "Cota" (suporte → modo → valor/nome → avançado) e "Completar" (escopo → esquemas → datum → prévia → aplicar).
  - Tudo que o Completar criar entra como **grupo desfazível num passo** e fica marcado como automático. O "Remover automáticas" apaga só o grupo.
  - Regras: nunca cotar onde uma relação geométrica já fecha o DOF. Priorizar relações antes de cotas (CAD avançado C). Parar sem aplicar se houver conflito.
  - Testes: perfil em L sem cotas chega a DOF = 0 com número de cotas igual ao contador prévio; remover automáticas devolve o DOF anterior; elipse isolada não é cotada.

### 2.2 Inferências, snaps e relações automáticas (CAD avançado B: Smart Pick, Create Detected Constraints / CAD avançado A: relações automáticas, Quick Snaps / CAD avançado C: inferência + persistência / outros: CAD avançado D/CAD avançado E snaps / Fenkra: `snap_at`, `settings.auto_constraints`)

- **O que faz:** enquanto o usuário desenha, o sistema mostra guias e cria relações (coincidente, ponto sobre curva, tangente, perpendicular, H/V, concêntrico) sem comando explícito.
- **CAD avançado B — no detalhe:**
  - O *Smart Pick* referencia automaticamente extremos, cria linhas paralelas/perpendiculares e aplica tangente e ponto médio. Pode ser desligado em Options > Mechanical > Sketcher > Smartpick [V parcial: cursos City University e fórum].
  - O comando *Create Detected Constraints* (barra Tools) cria as restrições detectadas ao gerar geometria 2D, e *Show Constraints* as exibe [V, nota IBM].
  - Não achei fonte sobre linhas pontilhadas.
- **CAD avançado A — no detalhe:** relações automáticas durante o desenho (horizontal, vertical, coincidente, ponto sobre curva, tangente, concêntrico) mais linhas de inferência. Dá para desligar [C].
- **CAD avançado C — no detalhe:**
  - A **inferência** decide o que é detectado e mostra o símbolo e as linhas pontilhadas de alinhamento.
  - A **persistência** decide se a restrição fica após o sketch. Coincidente em extremos de linha é sempre inferida e persistente, a menos que o usuário a desligue.
  - Em sketches complexos, persistir demais gera conflitos; o conselho é desligar nesses casos [V].
  - O objeto `SketchConstraintSettings` expõe isso na API [V].
- **Outros:** CAD avançado H tem as **Live Rules** (ver 2.6), que são a mesma ideia aplicada à edição de 3D.
- **Fenkra hoje:**
  - `snap_at` acha ponto existente, origem e grade [`sketch_ui.rs:1327–1354`].
  - Ao criar uma linha, só H/V a 2,5° vira restrição [`sketch_ui.rs:1559–1565`].
  - Clicar sobre uma curva cria ponto solto, sem `PointOnCurve`.
  - Não há ponto médio, interseção, centro ou quadrante como snap, nem guias de alinhamento.
  - O texto de configuração "próxima alpha" está defasado (`app.rs:7398`, ver sketch.md 2.16).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (geometria sob o cursor) → modo (snap com prioridade: ponto > médio > interseção > centro > quadrante > sobre a curva > grade) → persistência por tipo → avançado (liga/desliga por tipo).
  - Mostrar o **glifo no cursor antes do clique**. A prévia é a inferência; só o clique persiste.
  - Regras contra bugs: não inferir duas relações que se contradizem (H e perpendicular a V); não criar `PointOnCurve` em arco sem checar o limite angular (bug conhecido em `sketch.rs:924`); nunca inferir quando o ponto já está fixo.
  - Testes: clique sobre uma linha gera `PointOnCurve`; linha a 90° de outra, com guia, gera `Perpendicular`; com a inferência desligada nenhuma restrição é criada; desenhar duas vezes o mesmo traço dá o mesmo conjunto de relações.

### 2.3 Reparo e diagnóstico do sketch (CAD avançado B: Sketch Analysis, Sketch Solving Status / CAD avançado A: Check Sketch for Feature, Repair Sketch, SketchXpert / CAD avançado C: Design Doctor, contador de cotas / outros: CAD avançado E Profile inspector / Fenkra: `Act::SkCheck`)

- **O que faz:** acha perfil aberto, sobreposto, autocruzado ou referência perdida, e oferece correção.
- **CAD avançado B — no detalhe:** a janela *Sketch Analysis* tem páginas de geometria, de projeções/interseções (as arestas externas precisam estar "valid") e de diagnóstico, com mensagens como "perfil autocruzado" [V parcial, ver sketch.md 2.19]. O estado é recalculado em cada atualização e aparece nas cores verde/branco/roxo [V].
- **CAD avançado A — no detalhe:**
  - *Check Sketch for Feature* escolhe o tipo de feature (extrusão base, corte…) e relata se o contorno serve.
  - *Repair Sketch* lista os erros com lupa, funde sobrepostas e remove entidades muito pequenas. Só trata lacunas menores que o valor máximo; maiores ficam como intencionais [V].
  - *SketchXpert* percorre soluções candidatas e deixa o usuário escolher qual relação remover [V].
  - A relação perdida fica amarela (*dangling*) [V, sketch.md 2.17].
- **CAD avançado C — no detalhe:** contador de cotas necessárias, glifos de DOF e aceitar cota redundante como dirigida [V]. Perfil aberto é acusado na feature [C].
- **Outros:** o CAD avançado E lista os problemas de laço aberto, navega Anterior/Próximo e cada item resolvido sai da lista [V].
- **Fenkra hoje:**
  - `Act::SkCheck` desenha círculos nos extremos livres e nos pontos sobrepostos e conta laços [`toolbar.rs:547`, `app.rs:1919`; ver `sketch_ui.rs:1258–1290`].
  - Não há lupa nem lista, nem correção.
  - A tolerância de `chains()` (1e-7) é bem mais apertada que o 0,001 mm do contexto do usuário.
  - O `redundant` do solver não é exibido (sketch.md 2.17).
- **Melhor prática para o Fenkra:**
  - Painel "Analisar sketch" com abas (Perfis, Sobreposições, Autocruzamentos, Referências, Restrições). Cada item é clicável (zoom + destaque) e tem ação de correção com tolerância configurável.
  - Aceitar "para qual feature?" como parâmetro: o mesmo sketch pode estar OK para superfície e inválido para extrusão.
  - O reparo por SketchXpert (escolher qual restrição remover) vira lista de candidatos com prévia.
  - Testes: gap de 0,0005 mm é reparado; duas linhas sobrepostas são fundidas; laço em oito é acusado; cota redundante vira dirigida; corrigir e voltar à tolerância original não deixa resíduo.

### 2.4 Alças diretas na viewport (CAD avançado B: manipuladores de limite / CAD avançado A: Instant3D, Instant2D / CAD avançado C: grips e Edit Using Design Accelerator / outros: CAD avançado D/CAD avançado E manipuladores / Fenkra: `handles.rs` "Fast 3D")

- **O que faz:** arrastar uma seta ou ponto no 3D muda o valor do recurso (extrusão, deslocamento, raio) sem abrir o diálogo.
- **CAD avançado B — no detalhe:** os comandos de Part Design mostram setas de limite que se arrastam, com o valor atualizado no diálogo [C]. Não é o centro da filosofia: o CAD avançado B prefere o diálogo com a árvore.
- **CAD avançado A — no detalhe:**
  - Com **Instant3D** ligado, um clique no recurso expõe as cotas e as alças; com ele desligado exige duplo clique (Modify) [V].
  - **Réguas:** fora da régua o arraste é livre; sobre ela, o valor encaixa nos incrementos [V].
  - **Alt** encaixa a face arrastada na face-alvo [V].
  - A régua aparece também em Edit Feature de extrusão, revolução e aba base [V].
  - **Instant2D** (desde 2016) leva o mesmo comportamento ao sketch [V].
  - Não achei fonte sobre o comportamento de Shift [C].
- **CAD avançado C — no detalhe:** os assistentes (Design Accelerator, Bolted Connection) mostram grips na viewport. O grip de comprimento do parafuso se arrasta ou se clica duas vezes para abrir a lista de comprimentos [V].
- **Outros:** CAD avançado E/Fusion usam manipuladores semelhantes, com campo numérico flutuante [C].
- **Fenkra hoje:**
  - `handles.rs` cobre extrusão/recorte, plano deslocado, altura das primitivas e profundidade do reforço.
  - O arraste vai em passos de 1 mm (Shift: 0,1 mm) e o duplo clique abre a expressão.
  - **Valores ligados a parâmetros ficam travados (🔒)** [`handles.rs:4–5`].
  - Faltam: raio de filete, ângulo, distância de furo/padrão, snap em face (Alt) e régua.
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (recurso aberto) → alça → valor.
  - Trocar o "travado" por **"editar o parâmetro?"**, mostrando o nome e o valor em uso. Travar sem explicar vira bug percebido pelo usuário.
  - Ordem de entrega: filete/chanfro (raio), furo (diâmetro, profundidade), padrão (passo, quantidade), ângulos.
  - Snap Alt para faces.
  - Testes: arrastar e voltar ao valor original restitui o modelo bit a bit; alça de valor com parâmetro não perde o vínculo; alça não aparece em recurso suprimido.

### 2.5 Furo inteligente (CAD avançado B: Hole / CAD avançado A: assistente de furos + Hole Series / CAD avançado C: Hole + Thread.xls + Bolted Connection / outros: Fusion Hole, CAD avançado F Hole / Fenkra: `Hole`, tabelas em `hardware.rs`)

- **O que faz:** escolhe diâmetros de folga, roscas, escareados e rebaixos por norma e posiciona o furo por sketch ou pontos.
- **CAD avançado B — no detalhe:** o diálogo do Hole tem abas de extensão (cego, até o último, até plano/superfície), tipo (simples, cônico, rebaixado, escareado, contra-furo), definição de rosca e o fundo em V com ângulo. A posição vem de um sketch de posicionamento com restrições [V parcial: tutoriais e API]. Cada furo é um recurso paramétrico. Uma nota IBM relata que a rosca pode voltar a "No Standard" ao copiar o furo com PowerCopy [V].
- **CAD avançado A — no detalhe:**
  - O assistente de furos liga os dados do biblioteca de normas (rebaixos, folgas, passo, broca de rosca) ao recurso. Por isso, na tabela de projeto os furos do Wizard têm tamanho controlável [V].
  - Os fixadores inteligentes dimensionam os parafusos a partir desses furos; furos sem Wizard recebem o tipo padrão e podem precisar de ajuste [V, ver 2.16].
- **CAD avançado C — no detalhe:** o furo lê a tabela de rosca (Thread.xls). Furos e roscas ficam como recursos editáveis e o assistente de conexão parafusada pode criá-los [V, ver 2.16].
- **Outros:** o CAD avançado F guarda furos e padrões como recursos de família, com dimensões na tabela [C].
- **Fenkra hoje:**
  - `Hole` tem `standard` (informativo), rosca cosmética ou modelada, direção, fundo em V, cônico, rosca à esquerda [`doc.rs:2552–2584`].
  - O catálogo marca SOL-06 como completo; o B48 trata do ângulo do escareado.
  - O campo `standard` é "só informativo": não altera medidas nem é lido pelos fixadores.
  - A tabela de série existe para o furo flangeado da chapa, com linhas do usuário (`feat_ui.rs:669–700`).
- **Melhor prática para o Fenkra:**
  - Fluxo: posição (pontos/sketch/face) → modo (simples, rebaixo, escareado, roscado) → norma + tamanho (preenche tudo) → avançado (fundo, ângulo, cônico, rosca modelada).
  - Escolher "M8 normal" deve **preencher** as medidas e gravar o vínculo `standard`. Alterar uma medida à mão rebaixa o vínculo para "personalizado".
  - Os mesmos dados alimentam Fixadores, `hole_table` do desenho e a pilha da Conexão parafusada.
  - Testes: mudar a norma de M8 para M10 atualiza diâmetro, rebaixo e tabela de furos; o fixador acompanha; furo personalizado não é sobrescrito.

### 2.6 Edição direta e regras vivas (CAD avançado B: Remove Face, Replace Face [C] / CAD avançado A: Move Face, Delete Face (Delete and Patch), Offset / CAD avançado C: Direct (Edit Solid) [C] / outros: CAD avançado H Synchronous + Live Rules, CAD avançado G Synchronous Technology [C] / Fenkra: ausente, SOL-23)

- **O que faz:** edita faces sem depender do histórico: mover, girar, deslocar, apagar e remendar. É a ferramenta para geometria importada.
- **CAD avançado B — no detalhe:** não é o foco do V5. Há *Remove Face* e *Replace Face* em Part Design [C]. Os módulos de modelagem direta (por exemplo, Imagine & Shape) são separados [C].
- **CAD avançado A — no detalhe:**
  - *Move Face* faz deslocamento (offset), translação e rotação de faces, inclusive em corpos importados, e cria uma feature editável na árvore [V].
  - O offset normal serve para crescer/encolher um furo e tem opção de inverter [V].
  - *Delete Face* tem a variante **Delete and Patch**, útil para remover features de geometria importada e como passo intermediário para o Move Face funcionar [V].
- **CAD avançado C — no detalhe:** a edição direta atua em faces de sólidos sem histórico [C]. Catálogo do Fenkra aponta a página de "sketched features" como referência; não a li.
- **Outros:**
  - **CAD avançado H (Live Rules):** a inteligência está nas regras de relação mantidas durante a edição: perpendicular, concêntrico, paralelo, coplanar, tangente, ortogonal, "canted coplanar axes". O painel mostra verde = ligada e em uso, vermelho = desligada mas afetaria o resultado, laranja = ligada sem efeito agora. O usuário liga e desliga cada regra [V].
  - Com "Maintain Concentric Faces" ligada, selecionar uma face seleciona suas concêntricas [V].
  - O conselho do blog é fazer arredondamentos no fim da árvore como recursos ordenados, porque eles mudam o comportamento síncrono [V].
- **Fenkra hoje:** não existe. O catálogo registra SOL-23 como inexistente (B33). O `geom_fit` já reconhece plano/eixo de faces, base útil para o conjunto de faces.
- **Melhor prática para o Fenkra:**
  - Encaixe CAD avançado B: a edição direta entra como **recurso ordenado** na árvore ("Mover face", "Apagar face e remendar") com faces escolhidas por referência v2. Não é modo sem histórico. Isso mantém recompute e diagnóstico.
  - Conjunto de faces inteligente com regras ligáveis (concêntricas, coplanares, tangentes), semelhante às Live Rules.
  - Fluxo: faces → modo (mover, girar, deslocar, apagar/remendar) → valor → regras mantidas.
  - Testes: mover a face de um furo concêntrico leva o furo junto se a regra estiver ligada; apagar a face de um filete restaura a aresta viva; mover e voltar restitui volume e área; falha mostra diagnóstico, não resultado silencioso.

### 2.7 Reconhecimento de features (CAD avançado A: FeatureWorks, Convert to Sheet Metal / CAD avançado H: reconhecimento síncrono / CAD avançado B: Recognize (Chapa) [C] / CAD avançado C: converter em chapa / Fenkra: `geom_fit.rs`, CHP-16 e B35)

- **O que faz:** transforma geometria sem histórico (STEP, IGES, malha) em recursos editáveis (furos, filetes, extrusões, abas).
- **CAD avançado B — no detalhe:** o Sheet Metal Design tem comandos de reconhecimento para converter sólido em chapa [C, catálogo CHP-16 marca "Recognize"].
- **CAD avançado A — no detalhe:**
  - O FeatureWorks tem modos **automático** e **interativo**, que podem rodar em sequência. Cobre furos, ângulos de saída, padrões de sketch, lofts, varreduras, cascas e features de volume. Em chapa, abas base, abas de aresta, dobras e bainhas [V].
  - Opções interativas: encadear faces de filete/chanfro, fim "até a face" e "reconhecer similares" (cada ocorrência vira uma feature) [V].
  - Adiciona cotas e permite editar a definição depois [V].
  - Limitação: se as faces necessárias forem apagadas depois, a feature reconhecida não é refeita [V].
- **CAD avançado C/CAD avançado H:** o CAD avançado C converte em chapa com regras; CAD avançado H reconhece relações ao selecionar (ver 2.6) [C].
- **Fenkra hoje:** `geom_fit` só reconhece círculo, plano e eixo para preencher campos de seleção (`geom_fit.rs:1–3`, usado em `app.rs:4754`). Não há reconhecimento de feature. B35 prevê converter sólido em chapa.
- **Melhor prática para o Fenkra:**
  - Começar pelos reconhecimentos que dão retorno imediato: **furos cilíndricos** (diâmetro, profundidade, direção, rosca provável) e **filetes constantes**. Cada um vira `Hole`/`Fillet` com referências v2.
  - Sempre **dois modos**: prévia (lista de candidatos a confirmar) e aplicar. O reconhecimento nunca altera o sólido original; cria recursos e suprime a geometria de origem.
  - Testes: um bloco com 4 furos importados devolve 4 recursos Hole com Ø correto; o volume final é igual ao original (tolerância); apagar uma face necessária mostra "referência perdida" e não um sólido errado.

### 2.8 Seleção por intenção (CAD avançado B: Selection Traps, Search / CAD avançado A: Select Other, Select Loop, Select Tangency, Select Chain, F5 / CAD avançado H: QuickPick, Live Rules / Fenkra: `Filter`, `SelMode`)

- **O que faz:** escolhe muitos elementos com um gesto: o laço de uma face, as arestas tangentes, a cadeia de segmentos do sketch, o item escondido sob o cursor.
- **CAD avançado B — no detalhe:** traps de seleção (dentro/intersecta) e a Search por nome/tipo/cor no menu Editar [C]. O comando *Isolate* e o contexto de seleção em montagem vivem em outras fichas.
- **CAD avançado A — no detalhe:**
  - **Select Other** percorre o que está sob o cursor, escondendo a face escolhida [V].
  - **Select Tangency** propaga por faces ou arestas tangentes e para na primeira não tangente. Não existe para segmentos de sketch [V].
  - **Select Loop** pega as arestas ao redor de uma face, sem exigir tangência, e o puxador inverte o lado [V].
  - **Select Chain** (só sketch) seleciona segmentos conectados e ajuda a achar lacunas [V].
  - **F5** abre o filtro de seleção; o Selection Manager tem laço fechado, aberto e região, com Auto-OK [V].
  - Truque do laço interno: selecionar a face, Ctrl + uma aresta do laço, depois Convert Entities [V].
- **CAD avançado C — no detalhe:** seleção por prioridade (Part, Feature, Face, Edge) e cadeia de arestas tangentes [C].
- **Outros:** CAD avançado H **QuickPick** lista o que há sob o cursor em áreas densas e a Live Rule concêntrica amplia a seleção de faces [V].
- **Fenkra hoje:** `Filter` liga/desliga tipos (corpos, sketches, pontos, planos, eixos, rótulos) [`settings.rs:265–275`]. `Act::SelMode` escolhe Ctrl+arrastar, janela, cruzada ou pela direção [`toolbar.rs:18`]. Não há laço, tangência, cadeia nem "selecionar outro".
- **Melhor prática para o Fenkra:**
  - Fluxo: clique → menu "Ampliar seleção" (laço, tangente, cadeia, concêntricas, mesma superfície, mesmo raio) → seleção marcada com a regra usada.
  - Para recursos com referência v2, a seleção ampliada grava a **regra** ("tangentes a esta aresta") e não só a lista. Isso a refaz no recompute (alinhado ao B21).
  - Testes: Tangência a partir de uma aresta de filete seleciona toda a cadeia; Laço pega as arestas de uma face de furo; Selecionar outro percorre 3 candidatos sob o cursor; o filtro F5 impede selecionar tipo oculto.

### 2.9 Corte vivo / Live Section (CAD avançado B: Sectioning / CAD avançado A: Section View 3D [C] / CAD avançado C: Section View no modelo / outros: Fusion Section Analysis / Fenkra: ausente)

- **O que faz:** corta o modelo na viewport com um plano arrastável para ver o interior, sem alterar o modelo.
- **CAD avançado B — no detalhe:** o comando Sectioning cria planos de corte com manipulador e dá um perfil/medida da seção [C].
- **CAD avançado A — no detalhe:** a Section View 3D mostra o corte com plano ajustável [C]. Não confirmei por fonte.
- **CAD avançado C — no detalhe:** a vista de seção do modelo guarda os parâmetros numa representação de vista de projeto (design view), então cada seção exige a sua. A mini-barra inverte a direção e troca o tipo, inclusive três quartos [V].
- **Outros:** o Fusion tem controle de profundidade em Section View (anunciado em fórum) e análise de seção [V parcial].
- **Fenkra hoje:** não há corte vivo na viewport. As seções existem só no desenho (`ViewKind::Section`, `app.rs:563`).
- **Melhor prática para o Fenkra:**
  - O corte vivo é **visualização**: o modelo nunca muda (ADR 0011, princípio 7). Guarda-se como "vista salva" (já existe `views`) com plano, lado e tampa colorida da seção.
  - Fluxo: plano (origem, plano de referência ou face) → posição (arraste) → lado mantido → avançado (múltiplos planos, quarto).
  - Reaproveita o plano para o desenho (cria a vista de corte a partir dele).
  - Testes: o corte não altera o recompute nem os volumes medidos; a vista salva reabre igual; a tampa fecha a seção sem buraco em sólido válido.

### 2.10 Regras de chapa (CAD avançado B: Sheet Metal Parameters, tabelas de empresa / CAD avançado A: Sheet Metal Gauge Table, Bend Table / CAD avançado C: Sheet Metal Rule + Unfold Rule / Fenkra: `SheetParams`, `sheet_suggest`)

- **O que faz:** define de uma vez espessura, raio, fator K e alívios por material. Os recursos novos herdam os valores.
- **CAD avançado B — no detalhe:** a aba de parâmetros define espessura, raio e K; o K padrão pode ser calculado por fórmula DIN, que se pode desativar ou editar no Knowledge Advisor. Empresas usam tabelas próprias de tolerância de dobra [V parcial].
- **CAD avançado A — no detalhe:**
  - A **tabela de bitola** junta espessura, raio padrão e K; a tabela de dobra controla só tolerância/dedução/K e deixa espessura e raio livres.
  - Na aba base, "Usar tabela de bitola" escolhe a tabela; é possível desmarcar o raio padrão. As bitolas devem ser listadas da menor para a maior espessura.
  - Cuidados: K em branco na tabela mantém os valores da última dobra; há relato de o K da tabela não ser aplicado [V].
- **CAD avançado C — no detalhe:** a **Sheet Metal Rule** reúne material, espessura, raio, alívio e a regra de planificação padrão. A **Unfold Rule** pode ser por K, tabela de dobra ou equação. Erro comum: peças novas voltam ao K padrão em vez do K da regra, e a correção é ajustar o modelo-base (template) [V].
- **Fenkra hoje:**
  - `SheetParams` tem espessura, raio, K, material, método (K, dedução, tolerância) e `allowance` [`doc.rs:1694–1712`].
  - `sheet_suggest(material, t)` devolve raio e K de uma tabela [`doc.rs:1731`]. A seção "Tabela de dobras" existe na UI (`app.rs:3165`).
  - Falta: regras nomeadas por empresa/processo, alívios padrão na regra, e regra por **bitola** como entidade reutilizável.
- **Melhor prática para o Fenkra:**
  - Fluxo: regra (nome) → material + espessura → raio, K/dedução, alívio → avançado (tabela por ângulo). Os recursos da peça **referenciam a regra por nome** (princípio CAD avançado B: referência estável). Trocar a regra recalcula tudo.
  - Mudança de regra em peça com bainhas/estampos mostra diagnóstico por recurso (por exemplo, raio menor que a espessura).
  - Teste: trocar de "Aço 1020 / 2 mm" para "Inox / 3 mm" atualiza a planificação conforme a tabela; o comprimento planificado confere com o cálculo manual; arquivos antigos migram sem mudar a geometria.

### 2.11 Estampos e ferramentas de forma (CAD avançado B: Stamping / CAD avançado A: Forming Tools / CAD avançado C: Punch Tool (iFeature) / Fenkra: `SmStamp`, `SmLouver`, `SmFlangedHole`)

- **O que faz:** aplica, na chapa, formas prontas (veneziana, gota, nervura, saliência) posicionadas por pontos ou centros.
- **CAD avançado B — no detalhe:** o Stamping reúne Louver, Bead, Bridge, Flanged Hole e outros, cada um com seu diálogo [catálogo CHP-13].
- **CAD avançado A — no detalhe:**
  - As ferramentas de forma ficam em pastas marcadas como *Forming Tools Folder* na Design Library; se não, são tratadas como peça comum e dão aviso.
  - O usuário escolhe a **face de colocação**: face única coloca uma instância; sketch com pontos coloca uma por ponto. Há Flip, ângulo de rotação e opção de mostrar punção e perfil na planificação.
  - Falhas típicas: raio côncavo menor que a espessura e ferramenta que cruza uma dobra [V].
- **CAD avançado C — no detalhe:** o **Punch Tool** é uma iFeature de chapa. O sketch da ferramenta deve ter **exatamente um centro**; os parâmetros de tamanho que podem variar ficam numa tabela (o resto fica fixo); pode virar tabela de família; na peça a face precisa ter marcas de centro livres [V].
- **Fenkra hoje:** existem os comandos fixos de estampo, veneziana e furo flangeado, e o furo flangeado tem tabela de série com linhas do usuário (`feat_ui.rs:669–700`). Não há biblioteca de ferramentas de forma do usuário.
- **Melhor prática para o Fenkra:**
  - Fluxo: ferramenta (biblioteca) → pontos de colocação (sketch/face) → parâmetros expostos → avançado (inverter, girar, mostrar na planificação).
  - É caso de uso de Power Copy/UDF (2.20): a ferramenta é um recurso reutilizável com **um ponto de entrada e parâmetros publicados**.
  - Validar antes de aplicar: raio côncavo ≥ espessura, longe das dobras, dentro da face.
  - Testes: ferramenta em 3 pontos cria 3 instâncias; fora da face falha com mensagem; planificação mostra o furo da punção.

### 2.12 Gerador de estrutura e tratamentos de ponta (CAD avançado B: Structure Design (Profile, Endcut) / CAD avançado A: Structural Member + Trim/Extend / CAD avançado C: Frame Generator / Fenkra: `StMember`, `CornerMode`)

- **O que faz:** aplica perfis de catálogo a linhas ou arestas de um esqueleto e resolve os cantos (esquadria, topo, entalhe).
- **CAD avançado B — no detalhe:** o Structure Design usa perfis de catálogo e "endcuts" (snipe, trim, weld) nos membros [catálogo EST-02, pycatia].
- **CAD avançado A — no detalhe:**
  - Os tipos de canto são End Trim, End Miter, End Butt1 e End Butt2.
  - A **ordem de trim** decide quem corta quem: grupo de número menor apara o maior; números iguais fazem esquadria entre si.
  - Para três membros no mesmo vértice é preciso ajustar a ordem para obter a esquadria tripla.
  - Antes da lista de corte, todos os cantos devem estar aparados; há *Simple cut* e *Merge miter trimmed bodies* para o comprimento sair certo. O espaço de solda encurta os segmentos [V].
- **CAD avançado C — no detalhe:**
  - É preciso uma peça **esqueleto**; os membros recebem a linha correspondente. A seleção aceita linhas, arcos, elipses, splines e arestas de várias peças [V].
  - Tratamentos: Miter, Notch, Corner Joint e Trim/Extend. Eles **se acumulam e se misturam**; o comprimento se recalcula sozinho (Driven Length) [V].
  - Miter não suporta dois membros curvos ou mesclados; para esses, usar os comandos normais [V].
  - Erros típicos: membros paralelos/colineares no Miter; conflito com tratamentos anteriores no Notch (suprimir e reativar para achar) [V].
  - Reuso: mudar o tratamento do membro-fonte **antes** de reusar. *Remove End Treatments* restaura o membro [V].
  - A lista de peças e os balões atualizam sozinhos.
- **Fenkra hoje:** `StMember` já cobre âncora, ângulo e deslocamentos, e cantos com `CornerMode::{Nenhum, MeiaEsquadria,…}` (`fenkra-solidos/src/lib.rs:1816, 3791–3820`). Há lista de corte e geração de montagem. Faltam: entalhe (notch) por perfil (B37), ordem de trim explícita, reuso por esqueleto de peça, e remoção de tratamentos.
- **Melhor prática para o Fenkra:**
  - Fluxo: esqueleto → perfil → ancoragem → (avançado) cantos por par com **ordem explícita** e folga de solda.
  - Tratamentos viram objetos na árvore, por canto, com diagnóstico (paralelos/colineares → mensagem clara em vez de erro opaco).
  - Teste: 3 membros em um vértice com ordem 1-2-2 dão esquadria tripla; trocar a ordem muda quem corta; o comprimento da lista de corte soma a geometria real; remover tratamento restaura o corte reto.

### 2.13 Restrição inteligente na montagem (CAD avançado B: Quick Constraint, Smart Move / CAD avançado A: SmartMates, Magnetic Mates, Mate pop-up / CAD avançado C: Assemble, Predict Offset / CAD avançado H: FlashFit / Fenkra: `quick_options`, `auto_kind`)

- **O que faz:** deduz uma ou mais restrições a partir da seleção ou do arraste, para posicionar a peça sem abrir o diálogo.
- **CAD avançado B — no detalhe:** ver montagem.md 2.9 (lista de prioridade editável, modos cadeia/pilha, Smart Move com criação automática ao soltar) [V].
- **CAD avançado A — no detalhe:**
  - **SmartMates:** Alt + arrastar uma entidade até a parceira. O cursor mostra qual mate será criado. Tab alterna alinhado/anti-alinhado (soltar o Alt antes), e o pop-up confirma. Entidades válidas: arestas lineares/circulares, eixos temporários, vértices, faces planas, cilíndricas e cônicas, origens e sistemas de coordenadas [V].
  - Em sessões remotas, Alt/Tab podem não chegar; o CATI sugere iniciar pelo Move Component [V].
  - **Magnetic Mates** (desde 2017): arrastar o componente até um ponto de conexão publicado o encaixa. O Asset Publisher cria as referências. Em 2018 vieram ciclar entre pontos ativos, Tab para girar 90° e ícone de trava. Em 2019, uso no modo Large Design Review. A sensibilidade do encaixe é ajustável por distância de proximidade [V].
  - Mate References (ver 2.14) ativam-se também com Alt+arrastar [V].
- **CAD avançado C — no detalhe:** *Predict Offset and Orientation* preenche o deslocamento pela posição atual, e o *Preview* mostra o resultado. Desmarcar o Predict devolve offset zero, mas exige escolher a orientação correta [V].
- **Outros:** CAD avançado H **FlashFit** escolhe Mate, Planar Align ou Axial Align pela geometria clicada; ponto sugere Connect, aresta linear sugere Axial. Aresta circular é recusada por ambiguidade. **Capture Fit** guarda as relações usadas e reaplica na próxima colocação, mas não captura ângulo [V].
- **Fenkra hoje:**
  - Duas seleções abrem `quick_options` por par de geometria [`asm_ui.rs:97–105`].
  - `auto_kind` só distingue plano×plano (normais opostas = Contato, senão Coincidente); qualquer outro par cai em Coincidente [`asm_ui.rs:375–387`].
  - Não há arraste-e-encaixe, nem Tab, nem prévia da peça no lugar, nem lista de prioridade, nem modos cadeia/pilha.
  - O `AlignOnce` pode deixar a peça no meio do caminho sem aviso (montagem.md 2.9).
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte (par de geometrias) → modo (restrição prevista, mostrada no cursor) → parâmetros (alinhamento por Tab, offset predito) → avançado (prioridade, cadeia/pilha).
  - **Pares compostos:** eixo cilíndrico + face anelar = "Inserir" (concêntrica + contato). Aresta circular + aresta circular = concêntrica + coincidente (resolvendo a ambiguidade do FlashFit por **par de restrições**, não por escolha oculta).
  - Magnetic Mates é a especialização do mesmo mecanismo: encaixe por proximidade em **interfaces nomeadas** (2.14). Não vale fazer separado.
  - Regras: o arraste só cria a restrição ao soltar; se o solver não convergir, **reverter a posição** e avisar.
  - Testes: eixo+eixo → Coincidência; face+face de frente → Contato; cilindro+face anelar → par Inserir; Tab inverte sem criar restrição duplicada; conflito reverte a posição.

### 2.14 Referências de encaixe: interfaces nomeadas (CAD avançado B: Publications / CAD avançado A: Mate References, Asset Publisher / CAD avançado C: iMates / CAD avançado E: Mate Connectors / Fusion: Joint Origins / Fenkra: ausente)

- **O que faz:** a peça declara "como se encaixa". Ao inserir ou substituir, o sistema casa as referências e cria as restrições.
- **CAD avançado B — no detalhe:** a publicação nomeada faz esse papel. A restrição aponta para o nome e sobrevive à substituição do componente (ver montagem.md 2.22) [V].
- **CAD avançado A — no detalhe:** uma mate reference guarda geometria primária/secundária/terciária e o tipo. As duas peças precisam do **mesmo nome e da mesma ordem de entidades**. Só admite mates simples (coincidente, concêntrico, paralelo, tangente), não Largura [V].
- **CAD avançado C — no detalhe:**
  - O iMate é metade de um par de restrições na peça. Tipos de restrição: Mate, Angle, Tangent, Insert; com offset ou ângulo, limites (máximo, mínimo, posição de repouso) e supressão [V].
  - O casamento usa o **nome primeiro** e depois as propriedades. Uma lista de nomes de matching define a ordem de consumo [V].
  - **Composto:** vários iMates agrupados; casa com outro composto de mesmo nome e mesmo número de membros e resolve tudo de uma vez. Trocar peças que usam o composto as posiciona sozinha [V].
  - **Inferir iMates** de um componente, de uma restrição ou de features (furo, revolução, extrusão circular, só laços fechados). O CAD avançado C escolhe a aresta que julga mais útil; para outra, apagar e criar à mão [V].
  - Na colocação: *Interactively place with iMates* mantém o comando ativo; *Automatically generate iMates on place* aplica todas as soluções. A peça fica transparente até aceitar o primeiro par; Ctrl+I liga/desliga, Espaço pula o resto, Ctrl+Enter gera o restante e as setas ciclam [V].
- **Outros:**
  - **CAD avançado E:** o *mate connector* é um sistema de coordenadas local. O **implícito** nasce no momento do mate e fica sob ele. O **explícito** é criado antes (Ctrl+M), aparece no topo da lista de features e pode estar na Part Studio, sendo reaproveitado em todas as montagens. Tem eixo Z primário, "Reorient" do secundário, "Flip primary axis", dono (peça/sketch/curva) e Move com expressões [V].
  - **Fusion:** *Joint Origin* e *As-Built Joint* (modo simples com ponto de snap, entre duas faces, interseção de arestas) [V].
- **Fenkra hoje:** `MateRef` tem componente, geometria e `pick` (ponto clicado) [`fenkra-montagem/src/lib.rs:174–179`]. Não há nome, ordem ou tipo.
- **Melhor prática para o Fenkra:**
  - Criar a entidade **Interface** (nome, tipo, geometria por referência v2, offset, eixos primário/secundário). É uma **publicação do tipo "conector"** (ADR 0010/B55).
  - Fluxo: criar (explícita, ou inferida de furo/eixo/face) → nomear → agrupar (composta) → inserir (casamento por nome → tipo → ordem, com prévia e lista de pares) → avançado (limites, offset, lista de matching).
  - Implícita pode existir como **pré-visualização** no mate; só a explícita persiste.
  - Regras: um conector consumido não é reutilizado; se faltar uma interface na peça substituta, listar "não casada" em vez de escolher outra; inferir só de laços fechados.
  - Testes: peças A e B com "FuroM6"; inserir B cria 2 restrições; substituir B por B′ (mesmas interfaces) mantém 0 quebradas; interface ausente em B′ é listada; mover a geometria na peça mantém o conector.

### 2.15 Furo de montagem e Série de furos (CAD avançado B: Assembly Hole / CAD avançado A: Hole Series, Assembly Feature / CAD avançado C: assembly features + Bolted Connection / Fenkra: `AsmFeat(1)`, B57)

- **O que faz:** um furo único atravessa as peças do eixo, com tipo por peça e propagação.
- **CAD avançado B — no detalhe:** o *Assembly Hole* (Assembly Features) parte de uma face. O diálogo lista as peças candidatas (nome e caminho) e a lista "Affected parts" é editável, com realce. Limites: valor, cego, até o último, até plano, até superfície. Diâmetro, direção normal à face ou por linha, fundo em V, abas de tipo (simples, cônico, rebaixado, escareado, contra-furo) e de rosca. **Na árvore:** surge "Assembly features" com "Assembly Hole.1" e cada peça afetada ganha o furo com seta de vínculo. *Isolate* quebra o vínculo e deixa um furo comum. Pode reaproveitar um furo de Part Design (herda as especificações) ou um furo de montagem existente (só escolhe as peças) [V].
- **CAD avançado A — no detalhe:**
  - A *Hole Series* atravessa cada componente **não suprimido** cuja geometria cruza o eixo, mesmo sem tocar. Quem não deve ser furado precisa ser suprimido. A primeira peça, as do meio e a última têm papéis distintos; a última contém a rosca ou o fundo do furo cego [V].
  - Editar o furo só no nível da montagem; "underive" numa peça o transforma em assistente de furos sem referência à montagem [V].
  - Um assistente de furos existente pode servir de semente [V].
  - A aba fixadores inteligentes só existe com o biblioteca de normas ativo; com "auto size" o fixador acompanha o furo inicial [V].
- **CAD avançado C — no detalhe:** a feature fica na montagem com participantes (não grava furo nas peças) [catálogo MON-12]; a Bolted Connection é o caminho que cria os furos nas peças (ver 2.16).
- **Fenkra hoje:** `AsmFeatKind::Hole {p, d, dia, depth}` corta a **forma exibida** na montagem. Não grava furo nas peças, não entra no `hole_table` nem nos Fixadores. Posição só por clique e Ø padrão 6 [`fenkra-montagem/src/lib.rs:274–279`; catálogo MON-12]. É o B57.
- **Melhor prática para o Fenkra:**
  - Seguir o CAD avançado B para a árvore: **recurso de montagem + furo real em cada peça com seta (link)** e *Isolar*. Seguir o CAD avançado A para o escopo por eixo e os papéis primeira/meio/última.
  - Fluxo: posição (ponto publicado, sketch da montagem, cotas) → escopo (peças atravessadas, editável, com realce) → tipo por peça (folga na de cima, rosca na de baixo) → avançado (padrão, só na montagem).
  - Regras: toda peça do escopo precisa receber o furo ou aparecer na lista de falhas; se a peça está suprimida ou "fora do eixo", avisar; mudar a posição move os furos em todas as peças.
  - Testes: eixo atravessando 3 chapas → 3 furos reais; `hole_table` e Fixadores os enxergam; Isolar deixa 1 furo editável; mover o ponto atualiza os três.

### 2.16 Fixadores inteligentes e Conexão parafusada (CAD avançado B: catálogo de peças padrão / CAD avançado A: fixadores inteligentes, biblioteca de normas / CAD avançado C: Bolted Connection (Design Accelerator) / Fenkra: `preencher_furos`, B58)

- **O que faz:** acha furos, escolhe o parafuso e monta a pilha (arruelas, porcas), com restrições.
- **CAD avançado B — no detalhe:** não há automação nativa; usa-se catálogo e restrições manuais (FASTJOINT é add-on) [catálogo MON-16/18].
- **CAD avançado A — no detalhe:**
  - O comando fica na aba Assembly. Seleciona-se **face** (pega os furos dela), **componente** ou furos do Wizard. Revisa-se a lista, remove-se o que não se quer e usa-se *Add* (selecionados) ou *Populate All* (todos). Os fixadores saem agrupados por conjunto de furos e a janela fica aberta para edição [V].
  - Dimensionamento: os dados do biblioteca de normas (rebaixo, folga, passo, broca) ligam o fixador ao assistente de furos. Furos fora do Wizard recebem o tipo padrão das configurações [V].
  - Pilhas: *Top Stack* aceita arruelas; *Bottom Stack* arruelas e porcas; furo cego só tem pilha superior. A pilha vale para o grupo inteiro [V].
  - Mates concêntrico e coincidente são criados sozinhos [V].
  - Limites: não reconhece furos em corpos derivados ou importados; o furo deve atravessar pelo menos dois componentes; furos desalinhados não são reconhecidos (há *Hole Alignment* para achar o desalinhamento) [V].
- **CAD avançado C — no detalhe:** a Bolted Connection tem quatro modos de posição: *Linear* (plano inicial, duas arestas, plano final), *Concêntrica*, *Em ponto* (pontos de sketch; Ctrl adiciona/remove) e *Por furo* (com *Follow Pattern* para furos em padrão). Cria os furos e escolhe fixadores do Content Center filtrados por rosca e diâmetro. O comprimento é ajustado pelo grip ou pela lista, **sem escolha automática descrita**. *Edit Using Design Accelerator* reabre; apagar o parafuso apaga todos os seus fixadores; mover o ponto do sketch move a conexão. Por padrão as peças ficam na raiz e exigem solução manual; a opção de subconjunto permite solução automática. Limitações: padrões só no ambiente de peça e pontos de sketch criados na montagem não servem [V].
- **Fenkra hoje:** `preencher_furos` monta pilha parafuso/arruela/porca com restrições. A pega é medida por sondas laterais; não se edita a pilha depois (catálogo MON-16). Há o bug do B62 (1 GDL de rotação no parafuso).
- **Melhor prática para o Fenkra:**
  - Fluxo: escopo (face/componente/furo) → modo de posição → família + tamanho (herdado do furo, 2.5) → pilha superior/inferior → comprimento pela pega → avançado.
  - Resultado como **Conjunto de fixadores** na árvore com "Editar com assistente" (B58).
  - Regras: toda pilha inserida com GDL = 0 (B62); comprimento = pega + arruelas + porca + saliência de rosca, arredondado ao comprimento comercial; furo desalinhado dá aviso com a distância.
  - Testes: furo M8 passante em 2 chapas → parafuso + 2 arruelas + porca com GDL 0; trocar o comprimento recalcula; furo desalinhado de 0,2 mm lista o motivo; apagar o conjunto não apaga os furos.

### 2.17 Geradores e calculadores de componentes (CAD avançado C: Design Accelerator / CAD avançado A: biblioteca de normas (engrenagens, rolamentos) / CAD avançado B: catálogos / Fenkra: `engrenagens.rs`, `eixos.rs`, `AsmGears`)

- **O que faz:** gera peças de máquina por parâmetros e verifica o projeto por cálculo.
- **CAD avançado C — no detalhe:** **geradores** (conexão parafusada, eixo, estrias, chavetas, cames, engrenagens cilíndricas/cônicas/sem-fim, rolamentos, correias V e sincronizadas, correntes, pinos, anéis O) e **calculadores** (soldas, juntas soldadas e de solda mole, cubos, ajustes e tolerâncias, prensagem, mancal, fuso de potência, viga/coluna, placa, freios). Só um gerador ou calculador aberto por vez. Campos aceitam fórmulas e unidades; a prévia tem grips; "Calculate" roda; campos com erro ficam em vermelho e resultados esmaecem até recalcular. A mensagem de resumo usa azul (informação), vermelho (falha) e fundo vermelho (estado crítico). OK insere os componentes na montagem; o comando Update funciona; os componentes vêm do Content Center; o Component Authoring define iMates e parâmetros para peças próprias [V].
- **CAD avançado A/CAD avançado B:** o biblioteca de normas traz engrenagens e rolamentos por norma [catálogo BIB-03/07]; o CAD avançado B usa catálogos [C].
- **Fenkra hoje:** a janela de engrenagens gera pares e peças avulsas com cálculo ao vivo, e há eixo escalonado e chaveta DIN 6885 (B28 feito). B52 prevê paramétrico completo. Não há calculadores de ajuste, viga, solda nem rolamento (B44).
- **Melhor prática para o Fenkra:**
  - Padronizar o ciclo do CAD avançado C: **entradas → Calcular → resultados → OK insere** e "Editar com assistente" depois. Isso vale para engrenagens, eixos e futuros geradores.
  - Resultado = recurso na árvore com os parâmetros guardados (ver 2.19 e 2.20: um gerador é um tipo de Power Copy com cálculo).
  - Regras: nunca inserir com campo em erro; resultados "obsoletos" visíveis até recalcular; unidades explícitas.
  - Testes: mudar o módulo recalcula o diâmetro primitivo e a distância entre centros; reabrir "Editar com assistente" restaura as entradas; par com dentes insuficientes bloqueia o OK.

### 2.18 Diagnóstico e substituição de restrições (CAD avançado B: Update Diagnosis, Reconnect / CAD avançado A: MateXpert, Replace Mate Entities / CAD avançado C: Design Doctor, substituição por iMate / Fenkra: solver + janela de reparo)

- **O que faz:** quando a geometria muda e a restrição fica sem alvo, o sistema explica e propõe saídas.
- **CAD avançado B — no detalhe:** semáforo de status da restrição (verificada, impossível, a atualizar, quebrada) e *Reconnect* [V, montagem.md 2.13–2.14].
- **CAD avançado A — no detalhe:** o mate fica pendente (*dangling*) quando a geometria do componente muda e ele não pode ser satisfeito. O MateXpert (*Diagnose*) lista o mate e a geometria que falta; as saídas são apagar, editar com outras entidades ou *Replace Mate Entities*. A causa típica é o componente referenciado ter sido suprimido, apagado ou alterado, e a correção usual é escolher outra referência [V].
- **CAD avançado C — no detalhe:** a substituição de componente mantém as relações se houver iMates com os mesmos nomes (2.14) [V].
- **Fenkra hoje:** o solver devolve `failed` por restrição e há janela de reparo para comandos de peça, sem candidatos no 3D (GER-05, GER-11).
- **Melhor prática para o Fenkra:**
  - Todo estado de restrição é um enum visível (`ok`, `redundante`, `conflito`, `sem alvo`, `suprimida`) e a lista de diagnóstico oferece **Substituir entidade, Editar, Suprimir, Isolar**, com candidatos destacados no 3D.
  - O conserto fica registrado (quem foi trocado por quem) para poder desfazer.
  - Testes: apagar a face usada → restrição "sem alvo" (não some); substituir por outra face a restaura; substituir componente por outro com mesmas interfaces mantém 0 quebradas.

### 2.19 Configurações e tabelas de projeto (CAD avançado B: Design Table, Part Family / CAD avançado A: Configurations, Design Table / CAD avançado C: iPart, iAssembly / outros: CAD avançado F Family Table, CAD avançado E Configurations / Fenkra: ausente)

- **O que faz:** uma peça ou montagem gera variantes (tamanhos, suprimidos, propriedades) a partir de uma tabela.
- **CAD avançado B — no detalhe:** a *Design Table* é um arquivo texto/CSV ou planilha com conjuntos de valores de parâmetros (as configurações) e fica sob o nó Relations da árvore. Pode ser criada a partir dos parâmetros atuais e exportada para o Excel. Catálogos de família referenciam as variantes (capítulos e descrições) [V].
- **CAD avançado A — no detalhe:** a tabela de projeto da peça controla **dimensões, supressão de recursos e tamanho de furos do assistente de furos**, além de propriedades de configuração (número de peça para BOM, material, propriedades personalizadas, equações, relações de sketch, comentários, configurações derivadas). Exige o Excel; a tabela fica **embutida no modelo** (pode ser ligada a um arquivo externo). Cada linha é uma configuração; nas colunas de recursos, "S" suprime e "U" reativa [V].
- **CAD avançado C — no detalhe:** *Create iPart* e *iAssembly Author*: cada linha é um membro, cada coluna um atributo (limite de 256 colunas em iAssembly). Edição via planilha: não renomear cabeçalhos (quebra todos os membros); novo membro = copiar uma linha e mudar. A fábrica ligada a Excel externo é desaconselhada pela comunidade (pouco confiável; usar iLogic) [V].
- **Outros:**
  - **CAD avançado E:** entradas de configuração como tabela (cada linha é uma opção), variável de configuração, caixa de seleção e lista de entradas de feature. Regras de visibilidade entre entradas. Valem também para FeatureScript, números de peça, cor e material [V].
  - **CAD avançado F:** a tabela de família tem objeto-base genérico, itens controlados (inclusive UDFs e padrões) e linhas-instância, podendo ser aninhada. Cada instância tem os seus valores [V].
- **Fenkra hoje:** `Document.params` guarda parâmetros e expressões [`doc.rs:1114`, `2673`]. A montagem tem `AsmParam` [`fenkra-montagem/src/lib.rs:321`]. **Não existem configurações nem tabelas.**
- **Melhor prática para o Fenkra:**
  - Encaixe CAD avançado B: uma **Tabela de projeto** como objeto na árvore sob Parâmetros, com colunas = parâmetros, supressões, material e propriedades; linhas = configurações nomeadas. A configuração ativa é parte do estado do documento.
  - Fluxo: criar a partir dos parâmetros atuais → escolher colunas → editar linhas → ativar configuração → (avançado) derivar, ligar a CSV externo.
  - Na montagem, o componente aponta para (arquivo, **configuração por nome**). Configuração inexistente = erro visível, nunca a "primeira".
  - Regras: nomes de coluna estáveis (renomear atualiza a tabela); valor inválido não aplica a linha inteira; BOM e PN (PLM-06) lêem a configuração.
  - Testes: 3 linhas geram 3 variantes com volumes diferentes e corretos; trocar a configuração na montagem muda a peça e mantém as restrições; apagar uma configuração usada gera diagnóstico.

### 2.20 Reuso de recursos: Power Copy, UDF, iFeature, modelos de documento (CAD avançado B: PowerCopy, UDF, Document Template / CAD avançado A: Library Features / CAD avançado C: iFeature / CAD avançado F: UDF / CAD avançado E: Custom Features (FeatureScript) / Fenkra: ausente; só copiar/colar e macros)

- **O que faz:** salva um grupo de recursos com entradas e parâmetros e o instancia em outro lugar.
- **CAD avançado B — no detalhe:**
  - A **PowerCopy** se cria em Insert > Advanced Replication Tools: escolhe-se os elementos na árvore, dá-se nome, definem-se as referências (entradas) e um ícone [V].
  - A instanciação vai por *Instantiate From Document* (arquivo ou catálogo) ou pelo menu de contexto da PowerCopy. Escolhe-se a PowerCopy na lista, **seleciona-se a geometria de cada entrada**, e há "Usar nomes idênticos" para entradas repetidas, edição de parâmetros e "Criar fórmulas" para ligar os de mesmo nome. O botão Documentos lista arquivos ligados (por exemplo, design tables) e permite trocá-los. O resultado nasce **em contexto**, com limites redefinidos pelos elementos de entrada [V].
  - O *Replace Viewer* mapeia arestas quando o elemento substituto tem subelementos diferentes; a ordem proposta importa, senão o Update Diagnosis aparece [V].
  - A **UDF** (User Defined Feature) tem parâmetros publicados que dirigem o resultado; há restrição permanente: um parâmetro publicado não pode comandar uma regra que desativa uma saída da UDF (usar PowerCopy). Saídas extras podem ganhar sufixo ao instanciar em outra versão do CAD avançado B, quebrando scripts que dependem do nome [V].
  - Há falhas conhecidas: PowerCopy de filete/chanfro de aresta não instancia em outra aresta [V].
  - O **Document Template** e o *Product Knowledge Template* aparecem em cursos como assunto à parte da PowerCopy [V parcial: ementa GoEngineer].
- **CAD avançado A — no detalhe:** *Library Features* guardam recursos na Design Library; as ferramentas de forma de chapa são um caso especial (2.11) [C].
- **CAD avançado C — no detalhe:** *Extract iFeature* (aba Manage > Author) com tipos como Sheet Metal Punch. Os parâmetros da tabela de tamanho podem variar na colocação; o resto fica fixo; pode virar tabela de família e depois não aceita acrescentar ou remover parâmetros [V].
- **Outros:**
  - **CAD avançado F UDF:** recursos selecionados com suas cotas, relações entre eles e lista de referências para colocação. Relações no nível do recurso/sketch vão para a peça destino; relações e parâmetros de nível da peça não vão. Parâmetros não usados em relações não são copiados [V].
  - **CAD avançado E:** custom features em FeatureScript (linguagem tipo JavaScript, escritas num Feature Studio). Comportam-se como recursos normais (editar, suprimir, ocultar, apagar) e só existem em Part Studio [V].
- **Fenkra hoje:** copiar/colar/duplicar na árvore (`clip_menu`, `app.rs:6253`) e **macros em texto** que criam parâmetros, planos, sketches, extrusões, furos, padrões e laços (`macros.rs:1–30`, `run` em `:32`). Não há entradas nomeadas nem instanciação em contexto.
- **Melhor prática para o Fenkra:**
  - Encaixe CAD avançado B: **Modelo de recurso (PowerCopy)** = lista de recursos + **entradas** (referências v2 ou publicadas) + **parâmetros publicados** + ícone. Pode ser salvo em arquivo/biblioteca. Usa a mesma sintaxe de `macros.rs` para ser legível e versionável.
  - Fluxo de criação: selecionar recursos → o sistema calcula as entradas automáticas (tudo que o grupo referencia e não contém) → nomear entradas e parâmetros → ícone → salvar.
  - Fluxo de instanciação: escolher o modelo → apontar cada entrada (nomes idênticos) → editar parâmetros → instanciar em contexto. Resultado na árvore como **grupo**, com "Isolar" para quebrar o vínculo e "Editar modelo".
  - Regras: entrada faltante = erro de instanciação listando o nome; modelo que não instancia numa aresta (caso do filete) falha com mensagem, sem criar lixo; versionar o modelo (instâncias antigas avisam que o modelo mudou).
  - Testes: modelo "furo com rebaixo" instanciado em 3 faces dá 3 grupos independentes; mudar um parâmetro publicado muda só aquele; apagar a face de uma entrada marca o grupo como sem alvo.

### 2.21 Parâmetros, fórmulas e regras (CAD avançado B: Parameters/Formulas, Knowledge Advisor (Rules, Checks, Reactions, Design Tables) / CAD avançado A: Equations / CAD avançado C: Parameters + iLogic / CAD avançado F: Relations / Fenkra: `Parameter`, `AsmParam`, macros)

- **O que faz:** liga valores por fórmula e, no nível seguinte, executa regras condicionais que mudam o modelo.
- **CAD avançado B — no detalhe:** relações são fórmulas, regras (instruções condicionais), checagens (*checks*, alertas) e tabelas de projeto. As **reações** respondem a eventos e podem chamar macros. O *Knowledge Inspector* tem "What if" (como os parâmetros interagem) e "How to" (como obter um resultado desejado) [V].
- **CAD avançado A — no detalhe:** equações com variáveis globais ligam cotas. Também aparecem nas configurações (2.19) [C].
- **CAD avançado C — no detalhe:**
  - Uma **regra iLogic** é um pequeno programa VB.NET que monitora e controla parâmetros, features e componentes. É guardada dentro do documento; as externas ficam em arquivos de texto/VB.
  - Dispara quando os parâmetros usados mudam e por **eventos** (novo documento, depois de abrir, antes/depois de salvar, fechar, mudança de supressão de feature ou componente, mudança de geometria da peça). Mudanças de iProperty feitas fora do CAD avançado C não disparam.
  - **Formulários** (formulários) são interfaces personalizadas ligadas a parâmetros, propriedades e regras, montadas por arrastar e soltar, sem código.
  - **Assistentes** sem código: regra para diálogo, capturar vista atual, caixa de mensagem e **limites de parâmetro** (mínimo/máximo) [V].
- **CAD avançado F:** Relations em texto por feature e por peça (ver 2.20) [V].
- **Fenkra hoje:**
  - `Parameter {name, expr, unit, comment, published}` [`doc.rs:1114`] com avaliador de expressões (`fenkra-core/src/expr.rs`).
  - `AsmParam {name, expr}` na montagem.
  - Macros em texto com `para … fim` [`macros.rs`]. Executam sob demanda; **não há regras reativas, checagens, limites de parâmetro nem formulários**.
- **Melhor prática para o Fenkra:**
  - Nível 1 (barato, alto retorno): **limites de parâmetro** (mín/máx/lista de valores permitidos) e **checagens** ("espessura ≥ 1,5") que geram aviso na árvore, sem código.
  - Nível 2: **regras reativas** na sintaxe das macros, `se … então …`, disparadas por mudança de parâmetro e por salvar, com registro do que mudaram. Resultado sempre auditável (lista do que a regra alterou).
  - Nível 3: formulário simples por documento (campos = parâmetros publicados).
  - Regras: ciclo entre parâmetros dá erro com o laço apontado; a regra não toca um parâmetro travado por tabela de projeto; efeito de regra é desfeito junto com o undo.
  - Testes: limite mínimo bloqueia valor fora; checagem falha mostra mensagem e não impede recompute; regra "furos = f(largura)" muda a quantidade ao editar a largura; ciclo detectado.

### 2.22 Desenho: itens do modelo, chamadas de furo e tabelas automáticas (CAD avançado B: Generate Dimensions [C] / CAD avançado A: Model Items, Auto Balloon, Hole Table / CAD avançado C: Retrieve Dimensions, Auto Balloon / Fenkra: `dwg_ui.rs`, `hole_table`)

- **O que faz:** traz as cotas, chamadas de furo e tabelas do modelo para o desenho com um comando, mantendo o vínculo.
- **CAD avançado A — no detalhe:** *Model Items* importa cotas de features selecionadas ou do modelo todo, filtradas por "marcadas para desenho" ou não. Importa também a localização e a chamada de furos do assistente de furos. A colocação sai desalinhada, então há "usar a posição da cota do sketch". As cotas importadas continuam **ligadas ao modelo**: relatos de que alterar a cota no desenho altera o modelo [V].
- **CAD avançado B/CAD avançado C:** o CAD avançado B tem geração de cotas a partir do 3D e o CAD avançado C tem *Retrieve Dimensions* [C].
- **Fenkra hoje:** a tabela de furos existe (`fenkra_solidos::hole_table`, usada em `fenkra-desenho/src/source.rs:461`). DES-05 está completo (cotas associativas) e DES-08 (balões, BOM) também (catálogo). Importar cotas do modelo como comando de lote não foi encontrado nas fichas.
- **Melhor prática para o Fenkra:**
  - Fluxo: vista → fonte (recursos selecionados/modelo) → filtro (cotas marcadas, furos, roscas) → alinhamento automático → importar.
  - Cotas importadas são **dirigidas pelo modelo (somente leitura)**, a menos que o usuário opte por "cota condutora". Esse é o cuidado contra o bug relatado no CAD avançado A.
  - Chamada de furo lê a norma do furo (2.5): "4× Ø6,6 ▽ 12 / M8" e atualiza com o modelo.
  - Testes: importar do recurso de furo traz a chamada correta; mudar o furo atualiza o desenho; editar a cota no desenho não altera o modelo.

### 2.23 PMI automático e consultor de tolerância (CAD avançado A: DimXpert, Auto Dimension Scheme / CAD avançado B: FTA / CAD avançado C: 3D annotation / Fenkra: `Pmi(u8)`, B38)

- **O que faz:** reconhece features de fabricação e aplica cotas e tolerâncias de forma sistemática.
- **CAD avançado A — no detalhe:**
  - O *Auto Dimension Scheme* aplica cotas/tolerâncias às features de fabricação, no escopo da peça inteira ou das features escolhidas (útil com vários esquemas de datum).
  - Tipo de peça: prismática ou torneada. Tolerância geométrica dá posição para furos/bossas (prismática) e batimento circular (torneada); o modo mais/menos usa cotas lineares.
  - Passos: reconhece features (as de modelagem, como assistente de furos, têm prioridade sobre as de topologia; as complexas como rasgos, sobre as simples), gera o esquema e aplica. Compound holes, largura e geometria de interseção não são reconhecidos e exigem definição prévia.
  - Reconhecimento por modelo acompanha edições posteriores; o de topologia é o reserva, acha rasgos, entalhes e bolsos mas não acrescenta instâncias novas a padrões; corpos importados só usam topologia.
  - Datums: primário = feature de contato, secundário = de localização, terciário = de alinhamento [V].
  - O blog de MBD avisa que gera muito PMI, então recomenda uso seletivo [V].
- **Fenkra hoje:** `Pmi(0…8)` cria anotações 3D manualmente (linear … tolerância geométrica). O consultor de tolerância (PMI-05) é inexistente (B38).
- **Melhor prática para o Fenkra:**
  - Depende do reconhecimento (2.7): primeiro furos e rasgos, depois cotagem por esquema.
  - Fluxo: escopo → tipo de peça → esquema (datums A/B/C) → prévia → aplicar como grupo "PMI automático" desfazível e removível.
  - Consultor: contador de features sem cota/tolerância (como o contador do CAD avançado C para sketches).
  - Testes: peça prismática com 4 furos recebe 1 cota de localização por furo mais 1 posição; reaplicar não duplica; remover o grupo restaura o PMI manual.

### 2.24 Catálogos e famílias de peças (CAD avançado B: Catalog, Part Family / CAD avançado A: biblioteca de normas / CAD avançado C: Content Center / Fenkra: biblioteca, B42)

- **O que faz:** busca uma peça normalizada por família e tamanho e a insere já configurada.
- **CAD avançado B — no detalhe:** o catálogo é um documento em árvore (capítulos e descrições) que referencia peças e recursos, inclusive famílias paramétricas geradas de uma Design Table. Instanciar pelo navegador de catálogo [V].
- **CAD avançado A — no detalhe:** o biblioteca de normas liga os fixadores inteligentes e o assistente de furos (2.5, 2.16). Exige ativar o suplemento [V].
- **CAD avançado C — no detalhe:** o Content Center é pré-requisito para Bolted Connection e geradores (sem conexão, a área de fixadores fica esmaecida). Peças próprias podem ser publicadas com iMates via Component Authoring [V].
- **Fenkra hoje:** biblioteca com camadas pública/empresa/usuário em construção (B42), `hardware.rs`, fixadores industriais e engrenagens. Aeronáuticos no B63.
- **Melhor prática para o Fenkra:**
  - Cada item da biblioteca traz **interfaces** (2.14) e **parâmetros publicados** (2.20/2.21), para inserir já posicionado. Isso substitui a dependência do Content Center por dados abertos.
  - Sem a biblioteca, os assistentes (2.16/2.17) **não ficam esmaecidos em silêncio**: dizem "falta a família X" e oferecem o cadastro local.
  - Testes: família sem tamanho exato mostra o vizinho e não escolhe sozinha; inserir um parafuso M8 traz interfaces "eixo" e "face de apoio".

### 2.25 Busca de comandos e atalhos (CAD avançado A: Search Commands (tecla W, atalhos de busca) / CAD avançado E: Search tools [C] / Fusion: busca de comandos [C] / CAD avançado H e CAD avançado G: Command Finder [C] / CAD avançado B: Search / Fenkra: ausente)

- **O que faz:** digitar parte do nome e executar o comando, sem procurar nos menus.
- **CAD avançado A — no detalhe:** a tecla **W** ativa a busca de comandos (ou a lista de busca no canto superior com a opção Comandos); a tecla **S** abre a barra de atalho, que também aciona a busca. Enter ou clique executa. O usuário define **atalhos de busca** (por exemplo, "pat" para Linear Pattern) em Tools > Customize > Keyboard. O comando executado a partir de uma consulta é lembrado e sobe ao topo na próxima vez [V].
- **Outros:** CAD avançado E, CAD avançado D, CAD avançado H e CAD avançado G têm busca equivalente [C]. Não consegui confirmar teclas de CAD avançado E/Fusion.
- **Fenkra hoje:** não há paleta de comandos. Há atalhos configuráveis (`keymap.rs`, `ViewMenu` em Espaço) e a barra por grupos (`toolbar.rs`). O enum `Cmd` (`app.rs:5911`) já lista todos os comandos, e `toolbar::entry_title` dá nome e descrição.
- **Melhor prática para o Fenkra:**
  - Paleta única (Ctrl+K ou `/`) alimentada pelo registro de comandos do B60 (nome, sinônimos EN/PT, descrição e bancada). Busca tolerante a acento e a idioma.
  - Aprende com o uso (último comando executado por consulta sobe) e aceita **apelidos do usuário**.
  - Comandos indisponíveis na bancada atual aparecem esmaecidos com o motivo ("entre em um sketch").
  - Testes: "furo" lista Furo, Furo escareado, Furo roscado; "hole" também; executar o comando fora da bancada correta explica em vez de falhar; apelido do usuário sobrevive ao reiniciar.

---

## 3. Requisitos levantados

Prioridade: P0 quebra o uso · P1 importante · P2 desejável · P3 futuro.

### Funcionais (RF-INT)

| ID | Requisito | P | Critério de aceite |
|---|---|---|---|
| RF-INT-01 | Toda função inteligente cria objeto visível na árvore e editável pelo mesmo assistente ("Editar com assistente"). | P0 | Para cada função, existe teste que cria, reabre o assistente e confere as entradas restauradas. |
| RF-INT-02 | Inferência mostra glifo/guia no cursor antes do clique; só o clique persiste a relação. | P1 | Mover o cursor sobre uma linha mostra o glifo e não altera o sketch; clicar cria `PointOnCurve`. |
| RF-INT-03 | Snaps com prioridade (ponto > médio > interseção > centro > quadrante > curva > grade), ligáveis por tipo. | P1 | Em teste com 3 candidatos no raio, vence o de maior prioridade; desligar o tipo o remove. |
| RF-INT-04 | Inferência gera H/V, perpendicular, paralela, tangente e concêntrica, sem relações contraditórias. | P1 | Desenhar L e "C" com inferência ligada não gera conflito (`redundant == 0`, `failed == 0`). |
| RF-INT-05 | Comando "Completar sketch" com escopo, esquemas (cadeia, linha de base), datum, prévia e grupo desfazível. | P1 | Perfil em L chega a DOF = 0; "Remover automáticas" devolve o DOF anterior. |
| RF-INT-06 | Cota dirigida/de referência; cota redundante vira dirigida em vez de conflito. | P1 | Cotar duas vezes o mesmo comprimento mantém a geometria e marca a segunda como dirigida. |
| RF-INT-07 | Painel "Analisar sketch" com abas e correções (fechar lacunas com tolerância, fundir, remover curtas). | P1 | Gap de 0,0005 mm é reparado; laço em oito é acusado; cada item aplica zoom e destaque. |
| RF-INT-08 | Verificação "sketch válido para feature X" (aberto, autocruzado, fechado). | P2 | O mesmo sketch passa para superfície e falha para extrusão com mensagem específica. |
| RF-INT-09 | Alças diretas para raio, ângulo, furo e padrão, com snap Alt em face. | P2 | Arrastar e voltar restitui o modelo; alça de valor com parâmetro oferece editar o parâmetro. |
| RF-INT-10 | Furo inteligente: escolher norma e tamanho preenche medidas e grava o vínculo `standard`; edição manual rebaixa para "personalizado". | P1 | M8→M10 atualiza diâmetro, rebaixo, `hole_table` e fixador; furo personalizado não é sobrescrito. |
| RF-INT-11 | Edição direta como recurso ordenado (mover, girar, deslocar, apagar e remendar face) com referências v2. | P2 | Mover a face de um furo concêntrico leva o furo se a regra estiver ligada; falha mostra diagnóstico. |
| RF-INT-12 | Regras de seleção/edição ligáveis (concêntricas, coplanares, tangentes) com indicação de estado. | P3 | Painel mostra o estado de cada regra; desligar a regra muda o resultado do teste. |
| RF-INT-13 | Reconhecimento interativo de furos e filetes em geometria importada, com prévia e sem alterar o original. | P2 | Bloco importado com 4 furos devolve 4 `Hole`; volume final igual ao original dentro da tolerância. |
| RF-INT-14 | Seleção ampliada: laço, tangência, cadeia, concêntricas, mesmo raio, "selecionar outro". | P1 | Tangência a partir de uma aresta seleciona a cadeia de filete; Selecionar outro percorre os candidatos sob o cursor. |
| RF-INT-15 | A seleção ampliada grava a regra usada em recursos com referência v2 e a refaz no recompute. | P2 | Mudar o perfil e recomputar mantém "tangentes a esta aresta" com a nova cadeia. |
| RF-INT-16 | Corte vivo na viewport com plano arrastável, lado mantido e tampa de seção; guardado como vista salva. | P2 | O corte não altera volume nem recompute; a vista salva reabre igual. |
| RF-INT-17 | Regras de chapa nomeadas (material, espessura, raio, K/dedução, alívio) referenciadas por nome; trocar a regra recalcula. | P1 | Trocar a regra atualiza a planificação conforme a tabela; peça antiga migra sem mudar a geometria. |
| RF-INT-18 | Biblioteca de ferramentas de forma de chapa com um ponto de colocação e validações (raio côncavo, distância de dobra). | P2 | Ferramenta em 3 pontos cria 3 instâncias; fora da face falha com mensagem. |
| RF-INT-19 | Tratamentos de canto estrutural como objetos, com ordem de trim explícita e diagnóstico (paralelos/colineares). | P2 | 3 membros em um vértice com ordem 1-2-2 dão esquadria tripla; trocar a ordem muda quem corta. |
| RF-INT-20 | Restrição por arraste-e-encaixe com tipo previsto no cursor, Tab para alinhar/inverter e criação ao soltar. | P1 | Arrastar um cilindro até um furo cria Concêntrica + Contato; Tab inverte sem duplicar. |
| RF-INT-21 | Pares compostos de restrição ("Inserir": eixo + face anelar). | P1 | Selecionar cilindro e face anelar cria 2 restrições com GDL residual correto. |
| RF-INT-22 | Falha do solver ao soltar reverte a posição e avisa (inclusive no `AlignOnce`). | P0 | Teste com conflito deixa o componente na posição anterior e mostra o motivo. |
| RF-INT-23 | Entidade **Interface** (nome, tipo, geometria por referência v2, offset, eixos) na peça e na montagem; composta; inferida de furo, eixo ou face. | P0 | Criar, nomear e salvar; reabrir mantém; mover a geometria na peça mantém a interface. |
| RF-INT-24 | Inserção com casamento por nome → tipo → ordem, prévia e lista de pares; "não casada" lista a falta. | P0 | Peças A e B com "FuroM6": inserir B cria 2 restrições; B′ sem a interface lista "FuroM6". |
| RF-INT-25 | Substituir componente religa por interface; mostra as quebradas. | P0 | Substituir B por B′ com mesmas interfaces deixa 0 quebradas. |
| RF-INT-26 | Furo de montagem grava furo real em cada peça do escopo, com vínculo e *Isolar*; escopo por eixo editável. | P0 | Eixo atravessando 3 chapas gera 3 furos; `hole_table` e Fixadores os enxergam. |
| RF-INT-27 | Fixadores inteligentes por face, componente ou furo, com pilha superior/inferior e comprimento pela pega; conjunto editável. | P1 | Furo M8 passante em 2 chapas → parafuso + arruelas + porca com GDL 0; trocar o comprimento recalcula. |
| RF-INT-28 | Aviso explícito quando falta biblioteca ou o furo está desalinhado, com a distância. | P1 | Furo desalinhado de 0,2 mm lista o motivo; família ausente oferece cadastro local. |
| RF-INT-29 | Geradores com ciclo entradas → calcular → resultados → inserir, reabertos por "Editar com assistente". | P2 | Reabrir restaura as entradas; par com dentes insuficientes bloqueia o OK. |
| RF-INT-30 | Estado de cada restrição como enum visível (`ok`, `redundante`, `conflito`, `sem alvo`, `suprimida`) e diagnóstico com Substituir, Editar, Suprimir e Isolar. | P0 | Apagar a face usada marca "sem alvo" e a restrição não some; substituir a restaura. |
| RF-INT-31 | Tabela de projeto/configurações como objeto da árvore; configuração ativa; referência por nome na montagem. | P1 | 3 linhas geram 3 variantes corretas; trocar a configuração na montagem mantém as restrições. |
| RF-INT-32 | Configuração inexistente gera erro visível, nunca a "primeira". | P0 | Apagar uma configuração usada mostra diagnóstico e não troca a peça. |
| RF-INT-33 | Modelo de recurso (PowerCopy): criação com entradas automáticas e parâmetros publicados; instanciação em contexto; *Isolar*. | P1 | Modelo "furo com rebaixo" instanciado em 3 faces dá 3 grupos independentes. |
| RF-INT-34 | Instanciação com entrada faltante ou não aplicável falha com mensagem e sem criar recursos parciais. | P1 | Instanciar um modelo de filete em aresta inválida não deixa lixo na árvore. |
| RF-INT-35 | Limites e checagens de parâmetro (mín/máx/lista; "espessura ≥ x") com aviso na árvore. | P1 | Valor fora do limite é bloqueado; checagem falha mostra mensagem sem impedir o recompute. |
| RF-INT-36 | Regras reativas na sintaxe das macros, disparadas por mudança de parâmetro e por salvar, com registro do que alteraram. | P2 | Regra "furos = f(largura)" muda a quantidade; ciclo entre parâmetros é detectado com o laço apontado. |
| RF-INT-37 | Importar cotas, chamadas de furo e tabelas do modelo para o desenho em lote, com cotas somente leitura por padrão. | P2 | Importar do recurso de furo traz a chamada correta; editar a cota no desenho não altera o modelo. |
| RF-INT-38 | PMI automático por esquema (datums A/B/C), em grupo desfazível, com contador do que falta. | P3 | Peça prismática com 4 furos recebe 1 localização por furo; reaplicar não duplica. |
| RF-INT-39 | Paleta de busca de comandos com sinônimos PT/EN, apelidos do usuário e aprendizado por uso. | P1 | "furo" e "hole" listam os mesmos comandos; comando indisponível explica o motivo; apelido persiste. |
| RF-INT-40 | Catálogo: cada item traz interfaces e parâmetros publicados; busca sem tamanho exato mostra o vizinho sem escolher sozinha. | P2 | Inserir parafuso M8 traz as interfaces "eixo" e "face de apoio". |

### Não funcionais (RNF-INT)

| ID | Requisito | P | Critério de aceite |
|---|---|---|---|
| RNF-INT-01 | Inferência e prévia de snap respondem em até 1 quadro de 16 ms em sketches de até 500 pontos. | P1 | Benchmark do corpus (B53) abaixo do orçamento. |
| RNF-INT-02 | Funções inteligentes são determinísticas: mesma entrada, mesmo resultado. | P0 | Repetir 1000× a mesma operação gera arquivos idênticos. |
| RNF-INT-03 | Nenhuma função altera o modelo sem objeto na árvore, nem escolhe outra geometria em silêncio. | P0 | Auditoria: todo teste de falha termina em estado "falhou" com diagnóstico (ADR 0011, princípio 1). |
| RNF-INT-04 | Tudo que é automático é desfazível em um passo e removível sem tocar no que o usuário fez à mão. | P1 | Desfazer e "Remover automáticas" não alteram itens manuais (teste de comparação). |
| RNF-INT-05 | Formato dos modelos de recurso, regras e tabelas é texto aberto e versionado. | P1 | Arquivo legível, com `format_version`; abrir versão antiga migra sem perda. |
| RNF-INT-06 | Mensagens de falha dizem a causa e o próximo passo, em português e inglês. | P1 | Teste de i18n falha se faltar tradução (alinhado ao B60). |
| RNF-INT-07 | Toda função nova entra no corpus de peças (B53) com valores esperados. | P0 | Script de cobertura feature → teste falha sem a peça. |
| RNF-INT-08 | Geradores e bibliotecas com dados de normas citam fonte e nível (A/B/C) e checam propriedades. | P2 | Teste confere propriedades calculadas com as publicadas (B42). |
| RNF-INT-09 | Tabelas de projeto e regras não dependem de software externo (sem Excel obrigatório). | P1 | Funciona em máquina sem Office; importar/exportar CSV disponível. |
| RNF-INT-10 | Pré-visualizações (alça, corte vivo, snap) nunca alteram o modelo salvo. | P0 | Sair sem confirmar não altera o hash do arquivo. |

---

## 4. Itens de backlog propostos

Tamanho: P ≤ 80 k tokens · M ≤ 250 k · G acima disso. Os itens B-nn existentes aparecem como dependência.

| # | Título | O que muda | Prioridade | Tamanho | Dependências |
|---|---|---|---|---|---|
| N1 | Interfaces de encaixe (conectores) | Entidade Interface na peça/montagem (nome, tipo, geometria v2, offset, composta, inferida de furo/eixo/face); casamento por nome → tipo → ordem; prévia e lista de pares; liga a publicações. | P0 | G | B55, B15 |
| N2 | Substituir componente por interface | Religar restrições por nome; relatório de não casadas; usa os estados de N5. | P0 | M | N1, B54 |
| N3 | Restrição por arraste-e-encaixe | Arraste com tipo previsto, Tab, criação ao soltar, pares compostos (Inserir), reversão se o solver falhar; "magnetic" por proximidade das interfaces. | P1 | M | N1, B62 |
| N4 | Corrigir `auto_kind` e `AlignOnce` | `auto_kind` por par de geometria completo; `AlignOnce` reverte e avisa se o solver não convergir. | P0 | P | — |
| N5 | Estado e diagnóstico de restrições | Enum de estado, lista com Substituir/Editar/Suprimir/Isolar, candidatos no 3D. | P0 | M | B59 |
| N6 | Inferência de sketch | Snaps priorizados, glifo no cursor, guias, relações automáticas por tipo, persistência configurável. | P1 | M | B18, B19 |
| N7 | Completar sketch | Comando com escopo, esquemas, datum, prévia, grupo desfazível; cota dirigida. | P1 | M | N6, B32 |
| N8 | Analisar e reparar sketch | Painel com abas, correções com tolerância, "sketch válido para feature X", selo de estado na árvore. | P1 | M | — |
| N9 | Alças diretas ampliadas | Raio, ângulo, furo, padrão; snap Alt; "editar parâmetro" em vez de travar. | P2 | M | — |
| N10 | Furo com norma ligada | `standard` passa a preencher e vincular medidas; tabela do usuário; liga a fixadores e `hole_table`. | P1 | M | B48, B29 |
| N11 | Furo de montagem com furo real | Furo grava nas peças com vínculo e Isolar; escopo por eixo editável. | P0 | G | B57, N1 |
| N12 | Conjunto de fixadores editável | Escopo por face/componente/furo, pilha, comprimento pela pega, "Editar com assistente". | P1 | G | B58, B62, N10 |
| N13 | Seleção por intenção | Laço, tangência, cadeia, concêntricas, selecionar outro; regra gravada na referência v2. | P1 | M | B15, B21 |
| N14 | Edição direta como recurso | Mover/girar/deslocar/apagar+remendar face, com regras ligáveis. | P2 | G | B33, B15 |
| N15 | Reconhecimento de furos e filetes | Prévia de candidatos, conversão em `Hole`/`Fillet`, sem alterar o original. | P2 | M | N14 |
| N16 | Corte vivo | Plano arrastável, lado, tampa; vista salva; plano reaproveitado no desenho. | P2 | M | B61 |
| N17 | Regras de chapa nomeadas | Regra (material, espessura, raio, K, alívio) referenciada por nome; migração. | P1 | M | B31, B35 |
| N18 | Biblioteca de ferramentas de forma | Ferramentas com um ponto de colocação e validações; usa N20. | P2 | M | N20 |
| N19 | Tratamentos de canto estrutural | Objetos por canto, ordem de trim, notch, diagnóstico. | P2 | M | B37 |
| N20 | Modelos de recurso (PowerCopy) | Criação com entradas automáticas, parâmetros publicados, instanciação em contexto, Isolar, versionamento. | P1 | G | B15, B55 |
| N21 | Tabela de projeto e configurações | Objeto da árvore, configuração ativa, configuração por nome na montagem, CSV, sem Excel. | P1 | G | B25, B47 |
| N22 | Limites e checagens de parâmetro | Mín/máx/lista e avisos na árvore. | P1 | P | — |
| N23 | Regras reativas | Sintaxe das macros com `se`, disparo por parâmetro/salvar, registro de efeitos, detecção de ciclo. | P2 | M | N22, B39 |
| N24 | Importar itens do modelo no desenho | Cotas, chamadas de furo e tabelas em lote; somente leitura por padrão. | P2 | M | B30, N10 |
| N25 | PMI automático e consultor | Esquema por datums, grupo desfazível, contador do que falta. | P3 | M | N15, B38 |
| N26 | Paleta de busca de comandos | Registro único com sinônimos PT/EN, apelidos, aprendizado; comandos indisponíveis explicam o motivo. | P1 | P | B60 |
| N27 | Gerador com ciclo de assistente | Padronizar entradas → calcular → inserir → editar nos geradores; calculadores simples (ajustes, viga). | P2 | M | B52, N20 |

---

## 5. Fontes

CAD avançado B:
- (fonte em procedencia.md) (PowerCopy: criação)
- (fonte em procedencia.md) (PowerCopy: instanciação, Replace Viewer)
- (fonte em procedencia.md) (Assembly Hole)
- (fonte em procedencia.md) (restrições via SmartPick)
- (fonte em procedencia.md) (Knowledgeware)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.ibm.com/support/pages/apar/HD06015 (restrição de UDF)
- https://www.ibm.com/support/pages/apar/HD93258 (nomes de saída de UDF)
- https://www.ibm.com/support/pages/apar/HD93518 (PowerCopy de filete/chanfro)
- https://www.ibm.com/support/pages/apar/HC95142 (restrições detectadas)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://eng-tips.com/threads/incorrect-k-factor.126032
- https://www.ibm.com/support/pages/apar/HD77386

CAD avançado A:
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.javelin-tech.com/blog/2020/09/adjusting-a-sketch-without-having-to-use-edit-sketch/
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.cati.com/?p=152896 (SmartMates sem Alt)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://hawkridgesys.com/blog/adjusting-trim-order-weldments
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.javelin-tech.com/blog/2012/02/command-search/
- (fonte em procedencia.md) (Auto Dimension Scheme)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)

CAD avançado C:
- (fonte em procedencia.md) (Design Accelerator)
- (fonte em procedencia.md) (Bolted Connection)
- (fonte em procedencia.md) (Frame Generator)
- (fonte em procedencia.md) (tratamentos de ponta)
- (fonte em procedencia.md) (Create iMates)
- (fonte em procedencia.md) (colocar com iMates)
- (fonte em procedencia.md) (iLogic: regras e formulários)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (regra de planificação)
- (fonte em procedencia.md) (iPart)
- (fonte em procedencia.md) (iAssembly Author)
- (fonte em procedencia.md) (restrições de sketch)
- (fonte em procedencia.md) (Automatic Dimensions and Constraints)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (vistas de seção)
- (fonte em procedencia.md) (Predict Offset)

Outros:
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://soliddna.com/SEHelp/ST6/EN/assemble_parts/const1a.htm (FlashFit)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)

Código e documentos do Fenkra lidos: `crates/fenkra-app/src/{app.rs, handles.rs, geom_fit.rs, sketch_ui.rs, asm_ui.rs, feat_ui.rs, toolbar.rs, settings.rs, keymap.rs}`, `crates/fenkra-core/src/{doc.rs, macros.rs}`, `crates/fenkra-montagem/src/{lib.rs, fixadores.rs}`, `crates/fenkra-solidos/src/lib.rs`, `crates/fenkra-desenho/src/source.rs`, `docs/arquitetura/adr/{0010,0011}`, `docs/benchmark/catalogo.md`, `docs/backlog/backlog.md`, `docs/pesquisa/{sketch,montagem}.md`.

---

## Passagem de bastão

- **Resultado:** relatório completo, com 25 fichas, 40 RF, 10 RNF e 27 itens de backlog propostos.
- **Evidência:** fontes públicas lidas por WebFetch (fornecedor de CAD avançado CAD avançado C, CAD avançado E, City University London) e por resultados de busca (CAD avançado A e outras). Código do Fenkra lido nos arquivos citados.
- **Arquivos alterados:** nenhum. Não escrevi `docs/pesquisa/funcoes-inteligentes.md`; o lead grava.
- **Verificações:** nenhum teste ou build rodado, porque a tarefa era só de pesquisa.
- **Falhas e lacunas:**
  - `help.CAD avançado A.com` só devolveu o cabeçalho do site no WebFetch. Para CAD avançado A usei resumos de busca sobre essas páginas e blogs de revendas. Javelin (Hole Series) e TriMech (Mate References) deram 403.
  - Sem fonte confirmada: Shift no Instant3D; Live Section do CAD avançado A e do CAD avançado B; dados do CAD avançado B Hole (abas de posicionamento e extensão); CAD avançado G Synchronous e Command Finder; CAD avançado D/CAD avançado E para busca de comandos; Document Template do CAD avançado B além da ementa de curso. Todos estão marcados [C] no texto.
  - Não vi vídeos.
- **Riscos:**
  - O `auto_kind` e o `AlignOnce` (ficha 2.13) podem produzir restrições erradas ou meio-caminho hoje, antes de qualquer função nova.
  - Alças de valores com parâmetro ficam travadas, o que o usuário pode ler como bug.
  - O item N11 (furo de montagem com furo real) e o N1 (interfaces) são grandes e dependem do modelo de montagem do B55.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Maiores lacunas: (1) sem interfaces/mate references nomeadas nem restrição por arraste-e-encaixe; (2) sem inferência de sketch nem "Completar sketch"/reparo; (3) sem configurações/tabelas de projeto nem Power Copy/UDF/regras reativas; (4) sem edição direta, reconhecimento de features, Live Section, seleção por intenção e busca de comandos; (5) furo de montagem sem furo real nas peças e fixadores sem conjunto editável. Parte das fontes do CAD avançado A ficou só em resumos de busca, porque help.CAD avançado A.com não renderiza no WebFetch.
