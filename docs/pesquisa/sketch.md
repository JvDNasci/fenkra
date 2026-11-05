# Sketcher

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Relatório de pesquisa para o Fenkra. Marcações: **[V]** confirmado em fonte pública consultada nesta rodada; **[C]** conhecimento geral, não confirmado agora; **[código]** lido no repositório.
>
> **Limite das fontes:** as páginas de ajuda do CAD avançado A (`help.CAD avançado A.com`) devolveram só o menu quando abertas direto. Os fatos de CAD avançado A marcados [V] vêm de blogs de revendas lidos por inteiro (GoEngineer, Hawk Ridge, CATI, CADimensions, Javelin) ou de resumos de busca sobre as páginas oficiais. Várias páginas de CAD avançado B (GrabCAD, Technia, Eng-Tips) deram 403 ou 404. Os fatos de CAD avançado B [V] vêm de notas IBM/fornecedor de CAD avançado (APAR), CERN, Skill-Lync, SlideShare e descrições de cursos. A ajuda do CAD avançado C abriu bem e é a fonte mais forte.

---

## 1. Resumo e principais lacunas

**Resumo (10 linhas)**

1. O sketch do Fenkra é um modelo de pontos compartilhados, curvas (linha, círculo, arco, ponto, spline) e restrições, com solver Levenberg–Marquardt e Jacobiano numérico [código `sketch.rs:1075`].
2. Há 12 restrições geométricas e 8 dimensionais com expressões. A UI cria 11 delas, e a coincidência vira fusão de pontos [código `sketch_ui.rs:424`].
3. O estado do sketch tem três mensagens (conflito, totalmente restrito, sub-restrito) e um contador de graus de liberdade. O solver calcula `redundant`, mas a UI nunca mostra [código `sketch.rs:1189`; `sketch_ui.rs:2448`].
4. Não existe cota de referência (driven) nem aviso de sobre-restrição sem conflito. CAD avançado B, CAD avançado A e CAD avançado C tratam isso como um estado de primeira classe, com cor própria e ferramenta de reparo [V].
5. A inferência ao desenhar só reconhece horizontal e vertical em linhas. Não há snap em curva, ponto médio, interseção, tangente nem perpendicular [código `sketch_ui.rs:1327`, `1559`].
6. Aparar e estender só valem para linhas. Não há quebrar, aparar rápido por arrasto, fechar, complementar, canto, girar, escalar, esticar, padrão nem aparar contra spline [código `sketch_ui.rs:788–996`].
7. Projetar e converter 3D produz polilinhas de pontos fixos. Arco ou spline projetado vira spline aproximada, e só círculo fechado é reconhecido. Não há isolar/quebrar vínculo nem estado de vínculo perdido [código `sketch.rs:514–556`].
8. O sketch aponta para um plano de origem ou plano de referência. Não há origem nem orientação H/V próprias, ao contrário do *positioned sketch* do CAD avançado B [código `doc.rs:42`, `2472`] [V].
9. A cobertura de testes é pequena: 6 testes no solver/UI cobrem retângulo, conflito, rasgo, arrasto, spline, juntar pontos, medir e cota. Operações como filete, espelhar e offset não têm teste.
10. Pelo código, há quatro erros prováveis de modelagem: filete e chanfro mantêm a cota de comprimento da linha encurtada; o aparar cria ponto solto; o espelhar não iguala raios de círculos; `merge_points` não atualiza tangentes de spline nem vínculos. Cada um precisa de teste que o confirme (seções 2.20 e 2.27 a 2.29).

**Cinco maiores lacunas**

1. **Diagnóstico e estados:** só 3 estados e 1 cor de erro. Falta sobre-restrito/redundante, cota de referência, glifos de DOF, Analisar Sketch e Reparar (gaps e sobreposições).
2. **Inferência e restrições automáticas:** só H/V; falta snap e restrição inferida em curva, ponto médio, tangente, perpendicular e concêntrico, mais a caixa "restrições possíveis" do CAD avançado B e a cota inteligente completa.
3. **Edição de relimitação:** aparar, estender, quebrar e fechar só em linhas. Faltam aparar rápido/Power Trim por arrasto, cantos, fechar/complementar arco e manter ou refazer restrições.
4. **Geometria externa:** polilinhas fixas no lugar de arcos e splines reais. Falta isolar/quebrar vínculo, estado de vínculo, interseção com mais de um ponto e política de recuperação de referências.
5. **Posicionamento e criação:** sem origem e orientação do sketch, sem elipse/cônica, retângulo inclinado, rasgo em arco, padrão, spline CV, spline fechada nem tangência de spline com outras curvas.

---

## 2. Fichas por comando

Numeração: 2.1 a 2.3 posicionamento e contexto; 2.4 a 2.13 criação; 2.14 a 2.20 restrições, cotas, solver e diagnóstico; 2.21 a 2.30 operações; 2.31 a 2.35 geometria externa e interação.

---

### 2.1 Posicionamento do sketch: suporte, origem e eixos H/V (CAD avançado B: Sketch / Positioned Sketch / CAD avançado A: Sketch + Edit Sketch Plane / CAD avançado C: Create 2D Sketch + Origin / Fenkra: `Sketch.plane`)

- **O que faz:** define em que plano o sketch vive e onde ficam sua origem e seus eixos horizontal e vertical. Isso decide para onde cada cota e cada extrusão apontam.
- **CAD avançado B — no detalhe:**
  - Há dois tipos. O **sketch comum** (*sliding*) toma como origem a projeção da origem da peça no plano e não deixa mexer em H/V [V].
  - O **positioned sketch** deixa escolher o suporte (plano ou face), o tipo de origem e o tipo de orientação, e permite inverter ou trocar H e V [V].
  - Os tipos de origem e orientação são *implicit* e *explicit*. Em tutoriais aparecem opções como "Reverse H" e origem no baricentro de uma face [V]; os demais nomes são [C].
  - Há notícias IBM de sketch reposicionado no plano errado depois de edição, curadas ao reabrir "Change Sketch Support" [V]. Referência externa de suporte é fonte clássica de falha de update.
  - Quando a face-suporte some, o sketch fica com o suporte quebrado e o *Update Diagnosis* oferece substituir [C].
- **CAD avançado A — no detalhe:**
  - O sketch nasce de um plano ou face plana. A origem do modelo continua visível e sempre dá para ligar geometria a ela.
  - Dá para editar o plano do sketch (*Edit Sketch Plane*) e trocá-lo por outro suporte [C].
  - Relações penduradas ficam amarelas (*dangling*) [V]. Não há origem própria por padrão; a orientação vem do plano [C].
- **CAD avançado C — no detalhe:**
  - O sketch 2D nasce numa face ou num plano de trabalho, com opção de alinhar a vista.
  - Quando a origem de uma face não é a da peça, o usuário projeta a origem (*Project Geometry*) e ancora nela [C].
  - Existe a opção de projetar automaticamente arestas ao criar curva e de adicionar arestas de referência a sketches novos [V].
- **Outros:** CAD avançado E cria sketch em plano ou face e restringe tudo a referências externas; sem ordem de origem própria [C].
- **Fenkra hoje:**
  - `Sketch { plane: PlaneRef, data }` [código `doc.rs:2472`]. `PlaneRef` é `Origin(plano)` ou `Datum(Id)` [código `doc.rs:42`].
  - O quadro (`Frame`) vem do plano de origem ou do plano de referência resolvido [código `sketch_ui.rs:291`].
  - Não há campo de origem nem de orientação, nem "inverter H/V". O ponto (0,0) é sempre a origem do plano. Um plano de referência com origem estranha dá um sketch com cotas "tortas".
- **Melhor prática para o Fenkra:**
  - Fluxo: suporte → origem → orientação → avançado.
  - Modos de origem: projeção da origem da peça (padrão), ponto de referência escolhido, baricentro da face.
  - Modos de orientação: padrão do plano, alinhar a uma aresta/linha de referência, trocar H/V, inverter H, inverter V.
  - Guardar sempre a regra e a chave de referência (ADR 0010), nunca só as coordenadas.
  - Se a referência sumir, o sketch entra no estado "falhou" com diagnóstico (ADR 0011, princípio 1).
  - **Testes:** (a) duplicar sketch com origem no baricentro de uma face, mudar a face e conferir que a geometria acompanha; (b) trocar H/V de sketch totalmente restrito e comparar o sólido; (c) apagar o suporte e esperar erro visível, nunca sketch reposicionado em silêncio.

### 2.2 Entrar, sair e contexto do sketch (CAD avançado B: objeto de trabalho, Exit Workbench / CAD avançado A: Edit Sketch, Exit Sketch / CAD avançado C: Finish Sketch / Fenkra: `begin` / `finish_sketch`)

- **O que faz:** abre o sketch para edição, alinha a vista e devolve o controle ao modelo ao concluir.
- **CAD avançado B — no detalhe:** dar duplo clique no sketch na árvore abre o Sketcher. Sair recalcula o que depende dele. O sketch fica no corpo ativo (*Part Body*) ou num *Geometrical Set*, e o objeto em trabalho decide onde cai o próximo sketch [C, ADR 0011].
- **CAD avançado A — no detalhe:** *Edit Sketch* e *Exit Sketch* na barra de contexto. Há confirmação de saída (*confirmation corner*). Recalcula a partir da feature afetada [C].
- **CAD avançado C — no detalhe:** *Finish Sketch* e o contador de dimensões necessárias na barra de status [V].
- **Fenkra hoje:** `begin` recalcula o relatório e atualiza a geometria externa na posição do 3D [código `sketch_ui.rs:2673–2703`]. Esc cancela por camadas e, por fim, sai do sketch [código `sketch_ui.rs:247`, `2482`].
- **Melhor prática:** ao concluir, rodar o diagnóstico automático (2.19) e perguntar se há perfis abertos quando a feature seguinte exige perfil fechado. **Teste:** concluir sketch com perfil aberto e conferir que o aviso aparece antes da extrusão falhar.

### 2.3 Barra de status e dicas de passo (Fenkra: `status`, `tool.info`)

- **O que faz:** mostra qual ferramenta está ativa, o próximo clique e o estado do sketch.
- **Outros CADs:** o CAD avançado C mostra quantas dimensões faltam para restringir tudo [V]; o CAD avançado A mostra "sub/totalmente/sobre-definido" na barra [V]; o CAD avançado B mostra o estado na árvore com ícone e pela ferramenta *Sketch Solving Status* [V].
- **Fenkra hoje:** a linha "✏ Editando sketch · estado" e a dica "1º canto / canto oposto" ao lado do mouse [código `sketch_ui.rs:2448–2459`, `1799`].
- **Melhor prática:** acrescentar "faltam N cotas" (conta de DOF do solver), "N restrições redundantes" e um ícone na árvore (2.18).

---

### 2.4 Linha e perfil encadeado (CAD avançado B: Line, Profile / CAD avançado A: Line / CAD avançado C: Line / Fenkra: `Tool::Line`)

- **O que faz:** cria segmentos ligados; o clique no primeiro ponto fecha o perfil.
- **CAD avançado B — no detalhe:**
  - O **Profile** é uma ferramenta única que encadeia linha, arco tangente e arco por 3 pontos, trocando de modo pelo painel *Sketch tools* ou arrastando o mouse a partir do ponto final [C].
  - A linha é a ferramenta ativa por padrão [V].
  - A barra de ferramentas *Sketch tools* tem modos como *Geometrical Constraints* e *Dimensional Constraints* (criação de restrições ao desenhar) [C].
  - A saída na árvore é uma curva por segmento, com pontos nas pontas e restrições de coincidência [C].
- **CAD avançado A — no detalhe:** linha com **inferência** (linhas pontilhadas de alinhamento) e relações automáticas (horizontal, vertical, coincidente, tangente quando se arrasta do ponto final para virar arco) [C]. Linha de construção em modo separado.
- **CAD avançado C — no detalhe:** a inferência cria restrição ao criar a curva; a **persistência** decide se as restrições inferidas ficam. Com persistência desligada, nenhuma restrição automática é criada [V].
- **Fenkra hoje:** `Tool::Line` encadeia pontos, fecha pelo primeiro ponto e aplica H/V automático se o ângulo for menor que 2,5° [código `sketch_ui.rs:1539–1585`]. Com `auto_dims` cria `Length` em cada linha. Não encadeia arco tangente e não traz linhas de alinhamento.
- **Melhor prática:**
  - Suporte → modo (linha livre, linha tangente a partir do ponto final, arco a partir do ponto final) → parâmetros (comprimento e ângulo digitáveis, estilo CAD avançado A/CAD avançado C) → avançado (construção).
  - Fechar o perfil junta os pontos (já feito).
  - **Testes:** (a) três cliques fechando em triângulo dão perfil fechado; (b) clicar a 2° de horizontal cria `Horizontal`, a 3° não cria; (c) linha de comprimento zero nunca é criada.

### 2.5 Retângulos (CAD avançado B: Rectangle, Oriented Rectangle, Centered Rectangle, Parallelogram / CAD avançado A: Corner, Center, 3 Point Corner/Center, Parallelogram / CAD avançado C: Two-Point, Three-Point, Center / Fenkra: `Rect`, `RectCenter`)

- **O que faz:** cria quatro linhas com restrições de ortogonalidade.
- **CAD avançado B — no detalhe:** *Rectangle* (2 cantos), *Oriented Rectangle* (3 pontos), *Parallelogram* (3 pontos) e *Centered Rectangle* (centro e canto) [V parcial: lista do curso]. Os lados saem com restrições H/V e coincidência nos cantos [C].
- **CAD avançado A — no detalhe:** canto, centro, 3 pontos de canto, 3 pontos de centro, paralelogramo; o retângulo de centro cria linha de construção central e ponto médio [C].
- **CAD avançado C — no detalhe:** dois pontos, três pontos, canto e centro; a restrição de rotação do 3-pontos é perpendicular, não H/V [C].
- **Fenkra hoje:** `add_rect` com 4 pontos, 4 linhas, 2 H e 2 V [código `sketch.rs:648–661`]. O `RectCenter` usa linha de construção na diagonal e `Midpoint` do centro [código `sketch_ui.rs:1689–1717`]. Une cantos com pontos existentes por `merge_points`. Sem 3 pontos nem paralelogramo (BM-SKT-01).
- **Melhor prática:**
  - Modos: 2 cantos, centro, 3 pontos (lado + largura), paralelogramo.
  - O retângulo inclinado usa perpendicular, não H/V.
  - **Testes:** (a) retângulo girado a 30° fica totalmente restrito com 2 cotas + 1 ângulo + fixo; (b) arrastar um canto mantém o retângulo; (c) retângulo com `auto_dims` liga largura e altura sem sobre-restringir.

### 2.6 Polígono regular (CAD avançado B: Hexagon, Polygon / CAD avançado A: Polygon / CAD avançado C: Polygon / Fenkra: `Polygon`)

- **O que faz:** n lados iguais, inscritos ou circunscritos.
- **CAD avançado B — no detalhe:** o hexágono é definido pelo centro e pelo ponto médio de um lado; o Polygon aceita nº de lados [V parcial].
- **CAD avançado A / CAD avançado C — no detalhe:** opção **inscrito/circunscrito**, nº de lados, ângulo; o círculo de construção entra como geometria de apoio [C].
- **Fenkra hoje:** círculo de construção com `PointOnCurve` por vértice e `Equal` entre todas as linhas [código `sketch.rs:687–705`]. Número de lados no painel só antes de criar [código `sketch_ui.rs:2527`]. Sem circunscrito e sem editar n depois (mudar n exigiria reconstruir).
- **Melhor prática:** guardar `n` e o tipo de apoio (inscrito/circunscrito) como parâmetro do polígono, para editar depois sem apagar. **Teste:** polígono de 6 lados com raio cotado fica com DOF = 2 (centro) + 1 (rotação) até cotar ou travar um vértice na horizontal.

### 2.7 Círculos (CAD avançado B: Circle, Three Point Circle, Tri-Tangent Circle / CAD avançado A: Circle, Perimeter Circle / CAD avançado C: Center Point, Tangent / Fenkra: `Circle`, `Circle3`)

- **O que faz:** círculo por centro e ponto, por 3 pontos, ou tangente a três curvas.
- **CAD avançado B — no detalhe:** *Circle*, *Three Point Circle* e *Tri-Tangent Circle* [C].
- **CAD avançado A / CAD avançado C — no detalhe:** tangente a 3 curvas, perímetro (3 pontos); o círculo ganha restrição tangente se o ponto clicado está sobre outra curva [C].
- **Fenkra hoje:** `Circle { c, r }` com o raio como variável do solver [código `sketch.rs:1097–1103`]. `Circle3` usa `circum` e cria centro novo [código `sketch_ui.rs:1728`]. Com `auto_dims` recebe `Diameter`. Sem tangente a 3 curvas e sem tangência inferida.
- **Melhor prática:** clicar sobre uma curva existente cria `PointOnCurve` (inferência de 2.16). Tangente a 3 curvas vem como modo do círculo. **Teste:** círculo 3P por três pontos colineares é recusado sem criar geometria.

### 2.8 Arcos (CAD avançado B: Arc, Three Point Arc, Three Point Arc Starting With Limits, Tangent Arc / CAD avançado A: Centerpoint, Tangent, 3 Point / CAD avançado C: Three Point, Tangent, Center Point / Fenkra: `Arc3`, `ArcCenter`)

- **O que faz:** arco por centro e extremos, por 3 pontos, ou tangente ao anterior.
- **CAD avançado B — no detalhe:** arco por centro (circle com limite), por 3 pontos, e por 3 pontos com limites; o arco tangente nasce do perfil [C]. *Close* transforma um arco em círculo; *Complement* gera o resto do arco [V].
- **CAD avançado A / CAD avançado C — no detalhe:** o arco tangente nasce do ponto final de uma linha ou arco e herda a restrição tangente [C].
- **Fenkra hoje:** `Arc { c, s, e }`, sempre anti-horário de `s` a `e`; raio implícito, com a igualdade `|s−c| = |e−c|` imposta como equação extra [código `sketch.rs:29`, `1111–1115`]. `Arc3` calcula o sentido pelo ponto intermediário e `ArcCenter` projeta o fim no raio do início [código `sketch_ui.rs:1734–1773`]. Sem arco tangente e sem arrastar o arco no modo Selecionar (`_ => {}` em `sketch_ui.rs:1450`).
- **Risco de modelagem:** o sentido fixo anti-horário e a ausência de memória de ramo fazem o solver poder "virar" o arco para o complemento ao arrastar pontos. O arco de mais de 180° sofre mais.
- **Melhor prática:** guardar o sentido e o sinal do ângulo varrido como estado, e impedir inversão em silêncio. Arrastar o arco move o centro, muda o raio ou move o ponto, conforme a região clicada. **Testes:** (a) arco de 270° continua a 270° depois de arrastar o centro; (b) tangente de linha a arco permanece ao cotar o raio.

### 2.9 Elipse (CAD avançado B: Ellipse / CAD avançado A: Ellipse, Partial Ellipse / CAD avançado C: Ellipse / Fenkra: ausente)

- **O que faz:** elipse por centro, semieixos e rotação.
- **CAD avançado B — no detalhe:** *Ellipse* e conics na barra do Profile [V parcial: curso Udemy cita elipse, parábola, hipérbole por foco e tangentes].
- **CAD avançado A / CAD avançado C — no detalhe:** centro + eixo maior + eixo menor; elipse parcial; pontos de foco e eixos como geometria de apoio [C]. O *Fully Define* do CAD avançado A funciona mal com elipses [V Hawk Ridge].
- **Fenkra hoje:** não existe no `Curve` [código `sketch.rs:25`]. A cônica existe só no wireframe 3D (BM-SKT-02).
- **Melhor prática:**
  - Novo `Curve::Ellipse { c, major, minor_r }` com ponto de eixo maior.
  - Cotas: semieixo maior, semieixo menor, ângulo do eixo.
  - Restrições: centro coincidente, tangência a linha.
  - Converter para NURBS/ellipse do OpenCascade na extrusão.
  - **Testes:** extrudar elipse cotada; polilinha de amostragem é só para desenho.

### 2.10 Cônicas: parábola, hipérbole e cônica geral (CAD avançado B: Parabola by Focus, Hyperbola by Focus, Conic / CAD avançado A: Parabola, Conic / CAD avançado C: sem direto / Fenkra: ausente)

- **O que faz:** curvas por foco, vértice ou pontos extremos.
- **CAD avançado B — no detalhe:** parábola e hipérbole por foco, e cônica por extremos e tangentes [V parcial]. Os extremos geram limites; a curva pode ser aparada [C].
- **CAD avançado A — no detalhe:** parábola e cônica (3 pontos e valor de rho); curva dirigida por equação (*Equation Driven Curve*) [C].
- **Fenkra hoje:** ausente; só `RefKind::Conic` no wireframe (BM-SKT-02).
- **Melhor prática:** implementar a cônica como NURBS racional (um segmento com pesos) criada por 3 pontos + rho; parábola e hipérbole por foco numa segunda onda. **Teste:** cônica com rho = 0,5 é parábola e a interpolação passa pelos 3 pontos.

### 2.11 Spline (CAD avançado B: Spline, Connect Curve / CAD avançado A: Spline, Style Spline / CAD avançado C: Spline (Fit/CV) / Fenkra: `Spline`)

- **O que faz:** curva suave por pontos de passagem ou por polígono de controle.
- **CAD avançado B — no detalhe:** *Spline* por pontos, com tangentes e curvatura nos pontos; *Connect Curve* liga duas curvas com arco ou spline, com continuidade de ponto, tangente ou curvatura [V parcial, índice de livro]; "Splines com muitos pontos → torção" [contexto do usuário, `docs/contextos/sketcher.md`].
- **CAD avançado A — no detalhe:** *Spline* (fit), *Style Spline* (polígono de controle com pente de curvatura), *Spline on Surface*; pontos de controle e alças de tangência; curvatura e tangência como relações [C]. *Offset* não aceita spline por pontos de ajuste [V].
- **CAD avançado C — no detalhe:** [V, ajuda 2025]
  - Duas famílias: **interpolação** (passa por pontos de ajuste; extremos em quadrado, pontos em losango) e **CV** (polígono de controle; tangente ao quadro nas pontas).
  - Converter entre os tipos gera ajuste aproximado e não restaura os pontos originais.
  - Adicionar uma cota geral numa spline de interpolação cria também uma cota de **comprimento**, travando-o; remover a cota libera.
  - Três métodos de ajuste: Padrão, Energia Mínima e CAD avançado J.
  - Pente de curvatura com densidade e escala.
  - Inserir e apagar pontos; apagar CV só com 4 ou mais pontos.
- **Fenkra hoje:**
  - `Curve::Spline { pts }` mais `SplineTan` (alças Bézier) [código `sketch.rs:33`, `271`].
  - A curva passa pelos pontos. A forma desenhada é Catmull-Rom centrípeta, e com tangentes próprias usa Hermite cúbica [código `sketch.rs:230`, `304`].
  - Há inserir/remover ponto e marcar inflexões [código `sketch.rs:384`, `326`]. O teste `spline_com_tangente_e_pontos` cobre isso.
  - Faltam: spline fechada, modo CV, pente de curvatura (só inflexões), restrição tangente com outras curvas (DR-02), `PointOnCurve` sobre a spline (a equação é silenciosamente ignorada em `sketch.rs:917–927`), comprimento travável e aparar/estender spline.
- **Melhor prática:**
  - Suporte → tipo (por pontos / CV) → fechar → avançado (tangente nas pontas, curvatura, comprimento).
  - Mostrar o pente de curvatura e avisar quando pontos muito próximos geram oscilação.
  - **Testes:** (a) `PointOnCurve` ponto–spline realmente restringe (hoje não); (b) mover um ponto não cria cantos nem laços; (c) spline fechada gera perfil fechado.

### 2.12 Rasgo e furos oblongos (CAD avançado B: Elongated Hole, Cylindrical Elongated Hole, Keyhole / CAD avançado A: Straight Slot, Centerpoint Slot, 3 Point Arc Slot, Centerpoint Arc Slot / CAD avançado C: Slot (Center to Center, Overall, Center Point, 3 Point Arc, Center Point Arc) / Fenkra: `Slot`)

- **O que faz:** contorno com extremos arredondados.
- **CAD avançado B — no detalhe:** *Elongated Hole* (dois centros, o terceiro ponto dá o raio), *Cylindrical Elongated Hole* (o rasgo curvo em torno de um círculo) e *Keyhole* (dois raios) [V parcial].
- **CAD avançado A — no detalhe:** quatro tipos: reto por extremos, reto por centros, arco por 3 pontos e arco por centro. Opção de adicionar cotas e de tornar a linha central construção [C].
- **CAD avançado C — no detalhe:** vários modos, entre eles centro a centro e comprimento total; as cotas saem automaticamente se pedido [C, sem confirmação de fonte: a busca não encontrou a página].
- **Fenkra hoje:** `add_slot`: dois arcos, duas linhas, linha central de construção e tangência/igualdade de raios [código `sketch.rs:664–684`]. Cria `Distance` e `Radius` com `auto_dims` [código `sketch_ui.rs:1774–1786`]. O teste `rasgo_tangente_e_circulo` cobre o caso. Faltam rasgo em arco e keyhole (BM-SKT-03).
- **Melhor prática:** modos centro a centro, comprimento total, arco; o centro-a-centro vira cota `Distance` e o raio é uma cota que o usuário pode trocar por largura. **Teste:** rasgo com raio maior que metade da distância entre centros é recusado.

### 2.13 Ponto, eixo e linhas de construção (CAD avançado B: Point, Axis, Construction/Standard Element / CAD avançado A: Point, Centerline, Construction Geometry / CAD avançado C: Point, Center Point, Construction toggle / Fenkra: `Point`, `construction`)

- **O que faz:** pontos isolados (centros de furos), eixos e geometria de apoio que não entra no perfil.
- **CAD avançado B — no detalhe:** o *Standard Element / Construction Element* alterna cada curva entre geometria de perfil e de apoio; o **Axis** do sketch é usado como eixo de revolução; pontos por coordenada, interseção, projeção, ponto médio e proporcional [contexto do usuário; V parcial].
- **CAD avançado A — no detalhe:** *Centerline* serve de eixo de revolução e de simetria; qualquer entidade vira construção por propriedade [C].
- **CAD avançado C — no detalhe:** alternar construção; pontos de centro [C].
- **Fenkra hoje:** `Curve::Point` vira centro de furo quando não é de construção [código `sketch.rs:1360`]. A flag `construction` desenha tracejado em cor própria [código `sketch_ui.rs:1120`, `1157`]. A tecla de construção está em `keymap` [código `sketch_ui.rs:2490`]. Um eixo de revolução é "uma linha de sketch" (`AxisRef`) [código `doc.rs:53`].
- **Melhor prática:** marcar uma linha de construção como "eixo" explícito (um único por sketch no estilo CAD avançado B) e mostrar o ícone. Pontos por interseção e por proporção são próximo passo. **Teste:** revolução com a linha de construção como eixo não mistura com a casca do perfil.

---

### 2.14 Restrições geométricas (CAD avançado B: Constraint / Constraints Defined in Dialog Box / CAD avançado A: Add Relation / CAD avançado C: Constraint toolbar / Fenkra: `CKind`, `Kind`)

- **O que faz:** relações entre elementos que o solver mantém.
- **CAD avançado B — no detalhe:**
  - A **caixa de diálogo** (*Constraint Defined in Dialog Box*) aceita multisseleção e mostra só as restrições aplicáveis. Para uma linha: comprimento, fixar, horizontal, vertical; para dois elementos: distância, ângulo, coincidência, paralelismo, perpendicularidade. As já existentes aparecem marcadas [V].
  - A **restrição rápida** tem prioridade dimensional; o menu de contexto oferece as outras [V].
  - Restrições aparecem como ícones; clique duplo edita (ex.: posição de furo guardada em sketch) [V].
- **CAD avançado A — no detalhe:** *Add Relation* lista as relações possíveis para a seleção; tipos de coincidente, colinear, concêntrico, igual, fixo, horizontal, vertical, paralelo, perpendicular, ponto médio, simétrico, tangente, intersecção, fusão [C]. Os *callouts* ao selecionar mostram as relações da entidade, com opção de excluir [V].
- **CAD avançado C — no detalhe:** restrições geométricas criadas ao desenhar ou depois; símbolo e linhas pontilhadas de alinhamento aparecem na inferência; *Show Constraints* mostra as da seleção; *Show All Constraints* na barra de status [V].
- **Fenkra hoje:**
  - 11 tipos pela UI (`CKind`) [código `sketch_ui.rs:55`]. `Kind` tem ainda `PointOnCurve`, `HorizontalPts`, `VerticalPts` [código `sketch.rs:44`].
  - `applicable()` prova cada tipo tentando aplicá-lo numa cópia do sketch, o que dá a lista de possíveis [código `sketch_ui.rs:768`].
  - **Coincidente de dois pontos vira fusão** (`merge_points`), então não existe restrição visível nem removível; o ponto "sumiu" [código `sketch_ui.rs:424–427`].
  - **Lacunas:**
    - sem colinear, sem tangente com spline;
    - sem simetria em relação a ponto;
    - `Midpoint` só em linha;
    - `Tangent` entre círculos só externo (`dist = r1 + r2`) [código `sketch.rs:903–906`];
    - `PointOnCurve` em arco ignora o limite angular e trata como círculo inteiro [código `sketch.rs:924`];
    - `Equal` entre linha e círculo cairia no raio de linha = 0 se chamado (a UI evita) [código `sketch.rs:880–883`].
- **Melhor prática:**
  - Fluxo: seleção → lista de aplicáveis (já existe) → aplicar.
  - Guardar coincidência como restrição `Coincident` (já existe em `Kind`), com ponto único opcional ao "resolver", para poder listar, remover e diagnosticar.
  - Criar `Colinear`, `TangentInternal`, `PointOnArc` com limite, e `SymmetricAboutPoint`.
  - **Testes:** (a) tangente interna entre círculos; (b) ponto sobre arco só dentro do arco; (c) remover coincidência separa os pontos sem perder a geometria.

### 2.15 Cotas (restrições dimensionais) e Cota Inteligente (CAD avançado B: Constraint / CAD avançado A: Smart Dimension / CAD avançado C: General Dimension / Fenkra: `Tool::Dimension`)

- **O que faz:** fixa medidas e liga valores a expressões e parâmetros.
- **CAD avançado B — no detalhe:** a restrição rápida cria a dimensão por prioridade (comprimento, distância, ângulo, raio, diâmetro) conforme a seleção; o valor aceita **fórmula** (*Tools › Formula*), por exemplo `R2 = R1`, com os parâmetros renomeáveis [V Skill-Lync]. A cota pode ser desativada (vira referência) pelo menu de contexto [V City].
- **CAD avançado A — no detalhe:**
  - *Smart Dimension* deduz o tipo pela seleção e pelo movimento do mouse; o tipo de cota linear (alinhada, horizontal, vertical) pode ser travado [V].
  - Em arcos e círculos o padrão é o **centro**; segurar **Shift** ao clicar o interior escolhe min ou max; também há a aba Leaders > Arc Condition (centro, min, max); com dois círculos pode-se escolher min/min, max/max, centro/max [V].
  - Comprimento de arco: arco e dois extremos. Ângulo de arco: dois extremos e o centro [V].
  - Cota redundante vira cinza com "tornar esta cota dirigida" [V].
- **CAD avançado C — no detalhe:** [V]
  - Cotas **normais** comandam a geometria; **dirigidas** (referência) ficam entre parênteses e não são editáveis.
  - Cota que sobre-restringe oferece **aceitar** (vira referência) ou **cancelar**.
  - Valores aceitam constantes, variáveis de equação e parâmetros.
  - Dica oficial: estabilize a forma com restrições geométricas antes das cotas e cote os elementos grandes antes dos pequenos.
- **Fenkra hoje:**
  - Oito tipos de cota, valor como expressão avaliada com variáveis do documento [código `sketch.rs:44–69`, `1059`].
  - `Tool::Dimension` mostra uma cota em prévia que segue o mouse: acima/abaixo = horizontal, lado = vertical, diagonal = alinhada; Ctrl/Alt forçam H/V; botão direito lista todas [código `sketch_ui.rs:586–607`, `110`].
  - Duas linhas dão ângulo (ou distância se quase paralelas) [código `sketch_ui.rs:528–543`].
  - O texto da cota se arrasta, guardado em `label` [código `sketch.rs:142`].
  - **Lacunas:**
    - sem cota dirigida/de referência;
    - sem nome na cota (`Constraint` não tem campo de nome, então não dá para referenciar "d1" em outra expressão);
    - `DistanceX`/`DistanceY` usam `abs` e perdem o sentido [código `sketch.rs:940–941`];
    - `Angle` é sem sinal de 0 a 180° e depende da ordem dos extremos [código `sketch.rs:949–957`];
    - sem cota de comprimento de arco, ângulo de arco, tangente a círculo (min/max), linha–horizontal ou ponto–origem inteligente;
    - o rótulo de `auto_dims` cria cota em tudo que se desenha (pode sobre-restringir e polui).
- **Melhor prática:**
  - Fluxo: suporte (1 ou 2 entidades) → modo (a sugestão inteligente) → parâmetros (valor e nome) → avançado (min/max, dirigida).
  - Toda cota ganha nome (`d1…`) e pode ser referenciada em expressões; cota redundante vira dirigida em vez de dar conflito; sinal preservado onde importa.
  - Deixar `auto_dims` desligado por padrão.
  - **Testes:** (a) cotar com sinal e conferir que uma cota X negativa mantém o lado; (b) cota duplicada vira dirigida sem mudar a geometria; (c) arco: raio, comprimento e ângulo do arco convergem para o mesmo arco; (d) renomear `d1` e referenciar em `d2 = d1/2`.

### 2.16 Restrições automáticas e inferência (CAD avançado B: Geometrical Constraint on creation, Auto Constraint / CAD avançado A: Automatic Relations, inferência / CAD avançado C: Inferred constraints, Persistence, Automatic Dimensions and Constraints / Fenkra: `settings.auto_constraints`)

- **O que faz:** cria restrições enquanto se desenha, a partir da posição do mouse e da geometria vizinha.
- **CAD avançado B — no detalhe:** o modo de criação de restrições ao desenhar (geométricas e dimensionais) liga-se na barra de ferramentas; **Auto Constraint** examina a seleção e propõe restrições com referências H/V [V parcial]; avisos IBM tratam de restrições não criadas quando a opção "Create constraint" está desligada [V].
- **CAD avançado A — no detalhe:** relações automáticas durante o desenho (horizontal, vertical, coincidente, ponto sobre curva, tangente, concêntrico) mais **linhas de inferência** pontilhadas; dá para desligar [C]. *Fully Define Sketch* adiciona só as relações que já correspondem à geometria [V].
- **CAD avançado C — no detalhe:** a **inferência** decide o que é detectado; a **persistência** decide se fica após a criação [V]. *Automatic Dimensions and Constraints* mostra o contador de itens necessários, tem caixas para dimensões e restrições, e **Remover** desfaz só o que foi automático [V].
- **Fenkra hoje:** só H/V para linha, a 2,5° [código `sketch_ui.rs:1559–1565`]. O snap só encontra pontos existentes, a origem e a grade [código `sketch_ui.rs:1327–1354`]. Clicar sobre uma curva cria um ponto solto, sem `PointOnCurve`. O rótulo das configurações ainda diz "Aplicado pelo editor de sketch interativo (próxima alpha)" [código `app.rs:7398`], texto defasado.
- **Melhor prática:**
  - Snaps com prioridade: ponto existente > ponto médio > interseção > centro > quadrante > sobre a curva > grade.
  - Restrições inferidas: coincidente/ponto sobre curva, tangente (ao sair do ponto final de arco), perpendicular/paralela (linhas de alinhamento), concêntrico, H/V.
  - Mostrar o glifo no cursor antes de clicar; permitir desligar por tipo; um modo "persistência" separado.
  - **Testes:** (a) clicar sobre uma linha gera `PointOnCurve`; (b) desenhar uma linha a 90° de outra, com guia, gera `Perpendicular`; (c) com a inferência desligada nenhuma restrição é criada.

### 2.17 Solver e estados do sketch (CAD avançado B: iso/sub/sobre-restrito e cores / CAD avançado A: Under/Fully/Over defined / CAD avançado C: contador de DOF / Fenkra: `solve`, `SolveReport`)

- **O que faz:** resolve as equações e diz o que está livre.
- **CAD avançado B — no detalhe:** [V]
  - Cores: **verde** = iso-constrained (totalmente restrito), **branco** = sub-restrito, **roxo/magenta** = sobre-restrito. Nota IBM define roxo como sobre-restrito e recomenda remover uma ou mais dimensionais.
  - Há opção do Sketcher "Visualization of Diagnosis-colors"; desmarcada, o verde some.
  - Relatos IBM de cor que se perde ao adicionar ponto ou ao clicar (voltam ao reentrar) mostram que a cor é um estado calculado a cada atualização.
- **CAD avançado A — no detalhe:** [V] **azul** = sub-definido, **preto** = totalmente definido, **vermelho** = sobre-definido ou conflitante, **amarelo** = relação perdida (*dangling*). Arrastar uma entidade azul revela o que falta. O estado também aparece na barra de status.
- **CAD avançado C — no detalhe:** [V] glifos de DOF por geometria: **translação** (seta única), **translação 2D** (ponto livre no plano) e **rotação** (curva gira em torno de um ponto). Cada restrição remove um glifo; sem glifos, a geometria está restrita. Ligar com *Show All Degrees of Freedom* na barra de status ou no menu de contexto.
- **Outros:** CAD avançado E usa azul/preto/vermelho, ícones de restrição pretos sobre cinza quando bons e brancos sobre vermelho quando em conflito [V]. No CAD avançado E o solver não sabe qual entidade é a "mestre" e tenta mover o mínimo [V].
- **Fenkra hoje:**
  - Variáveis: x e y de cada ponto livre mais o raio de cada círculo. Levenberg–Marquardt com no máximo 100 iterações, 12 tentativas de λ por iteração e Jacobiano numérico por diferença central [código `sketch.rs:1075–1170`].
  - Converge se o resíduo for menor que 1e-6.
  - DOF = `n − posto` por RREF com tolerância 1e-7, e `fixed_points` determinados por colunas livres [código `sketch.rs:1184–1203`].
  - `redundant = m − posto` é calculado [código `sketch.rs:1189`], mas **nenhuma tela o lê**. Um sketch com cota duplicada e consistente aparece como "Totalmente restrito" sem alerta.
  - Cores: `defined` (todos os pontos determinados), `FREE` (azul), `BAD` (curva com restrição falhando) e `CONSTR` (construção) [código `sketch_ui.rs:1030–1033`, `1116–1126`]. Não há glifo de DOF, nem estado "sobre-restrito sem conflito", nem "perdido".
  - **Desempenho:** matriz densa n×n e Jacobiano numérico recalculado a cada iteração, a cada quadro de arrasto [código `sketch_ui.rs:1485`]. Cresce com o quadrado dos pontos; ainda não medido.
  - **Falha de convergência:** a solução parcial é aplicada ("fica o melhor ajuste"), o que pode deformar o sketch ao errar uma cota [código `sketch.rs:1205`].
- **Melhor prática:**
  - Estados explícitos na estrutura: `Sub`, `Totalmente`, `Redundante`, `Conflito`, `Vínculo perdido`.
  - Cores fixas (azul, verde/preto, laranja ou roxo, vermelho, amarelo) e glifos de DOF opcionais.
  - Conflito: **não aplicar** a solução parcial ao modelo; manter a última boa e destacar as restrições culpadas.
  - Avaliar solver esparso (ou Jacobiano analítico) quando houver mais de ~200 pontos.
  - **Testes:** (a) cota duplicada e consistente dá "Redundante" e não "Totalmente restrito"; (b) cota contraditória dá "Conflito" com a lista das duas restrições e mantém a geometria anterior; (c) 500 pontos em cadeia resolve em tempo fixado (orçamento a definir); (d) o sketch totalmente restrito não muda ao reabrir.

### 2.18 Estado na árvore e ícone de diagnóstico (CAD avançado B: Sketch Solving Status / CAD avançado A: ícone de estado / CAD avançado C / Fenkra: ausente)

- **O que faz:** o estado do sketch aparece sem abri-lo.
- **CAD avançado B — no detalhe:** botão **Sketch Solving Status** (ícone de peça de quebra-cabeça) que abre um relatório do estado do sketch [V]. O CERN descreve ler o estado de resolução na árvore por indicadores visuais [V].
- **CAD avançado A — no detalhe:** sinal de estado ao lado do nome do sketch (+, −, ?) na árvore [C].
- **Fenkra hoje:** nada na árvore; o estado vive só no editor.
- **Melhor prática:** gravar o último `SolveReport` resumido (estado, DOF, redundantes) por sketch e mostrar um selo na árvore. **Teste:** sketch com perfil aberto exibe selo de aviso mesmo fechado.

### 2.19 Diagnóstico e reparo do sketch (CAD avançado B: Sketch Analysis / CAD avançado A: Check Sketch for Feature, Repair Sketch, SketchXpert / CAD avançado C: Design Doctor, profile checks / Fenkra: `Act::SkCheck`)

- **O que faz:** acha perfis abertos, sobrepostos, cruzados e referências inválidas, e sugere correção.
- **CAD avançado B — no detalhe:** *Sketch Analysis* tem uma página de geometria (perfis, abertos, autocruzamento, lista de elementos), uma de **projeções/interseções** (as arestas externas devem estar "valid") e uma de **diagnóstico** [V parcial: Eng-Tips e APAR IBM]. O relatório traz um estado geral (por exemplo, "topologia não variedade") e detalhes como "perfil autocruzado" [V]. Serve para confirmar fechamento antes de Pad/Pocket [V].
- **CAD avançado A — no detalhe:**
  - **Check Sketch for Feature:** escolhe o tipo de feature (extrusão base, corte…), diz que contorno é exigido e relata. Atenção ao texto completo: um exemplo passou com contorno aberto "sem problema" [V].
  - **Repair Sketch:** lista erros, usa lupa para mostrar onde; remove entidades muito pequenas e funde sobrepostas; só trata lacunas menores que o valor máximo, deixando as maiores como intencionais [V, resumo de ajuda].
  - **SketchXpert:** *Diagnose* percorre soluções candidatas e *Manual Repair* mostra as relações em conflito para o usuário escolher qual remover [V].
- **CAD avançado C — no detalhe:** o contador de cotas necessárias, os glifos de DOF e a aceitação de cota como referência cobrem o diagnóstico de sub e sobre-restrição [V]. A verificação de perfil aberto fica por conta da feature [C].
- **CAD avançado E:** o *Profile inspector* lista problemas como laços abertos; clicar destaca; Anterior/Próximo; cada item resolvido sai da lista [V].
- **Fenkra hoje:**
  - `edit.check` desenha círculos vermelhos em extremos de grau 1 (só linhas e arcos), laranja em pontos sobrepostos (O(n²)) e conta laços [código `sketch_ui.rs:1258–1290`].
  - Sem lupa, sem lista, sem correção. Splines ficam fora da verificação de abertura.
  - `chains()` une extremos coincidentes por tolerância `1e-7·escala + 1e-9`, bem mais apertada que os 0,001 mm do contexto do usuário [código `sketch.rs:1278–1279`; `docs/contextos/sketcher.md`]. Gap de 0,0005 mm numa importação deixa o perfil aberto.
- **Melhor prática:**
  - Painel "Analisar sketch" com abas: Perfis (abertos, fechados, isolados), Sobreposições, Autocruzamentos, Referências externas (válida / perdida), Restrições (redundantes/conflitantes).
  - Cada item clicável (zoom e destaque) e com ação de correção: fechar lacuna (até uma tolerância configurável), fundir sobrepostos, remover curtas.
  - Aceitar a feature seguinte como parâmetro ("perfil fechado exigido") como no Check Sketch for Feature.
  - **Testes:** (a) gap de 0,0005 mm é reparado e o perfil fecha; (b) duas linhas sobrepostas são fundidas; (c) um laço em figura de oito é apontado como autocruzado; (d) spline com extremo solto aparece como aberta.

### 2.20 Definir tudo / cotas automáticas (CAD avançado B: Auto Constraint / CAD avançado A: Fully Define Sketch / CAD avançado C: Automatic Dimensions and Constraints / Fenkra: ausente como comando)

- **O que faz:** calcula e aplica cotas e restrições que faltam.
- **CAD avançado A — no detalhe:** [V Hawk Ridge] escolhe entre todas as entidades ou as selecionadas; as caixas de relações apenas **limitam** as que podem ser usadas; esquemas de cotagem horizontal e vertical separados (**cadeia**, **linha de base**, **ordenada**); **datum** externo (origem, aresta) ou interno (ponto do sketch). Funciona melhor com linhas e pontos; elipses vão mal; arcos ficam separados se não houver `Equal`. *Calculate* prévia o resultado e só *Ctrl-Z* desfaz.
- **CAD avançado C — no detalhe:** [V] seleção opcional; caixas de dimensões e de restrições; contador do que falta; *Apply*, *Remove* (só do que for automático) e *Done*. Recomenda cotar o essencial à mão antes.
- **Fenkra hoje:** não existe como comando. As cotas automáticas são criadas no momento do desenho (`auto_dims`).
- **Melhor prática:** comando "Completar sketch" com prévia, esquemas (cadeia, linha de base), datum e lista do que foi criado como grupo (desfazer num passo). Nada de cota onde a restrição geométrica já basta. **Teste:** perfil em L sem cotas chega a DOF = 0 com o número de cotas igual ao contador antes da execução.

---

### 2.21 Aparar e aparar rápido (CAD avançado B: Quick Trim, Trim / CAD avançado A: Trim Entities (Power Trim, Trim to Closest, Corner, Inside, Outside) / CAD avançado C: Trim / Fenkra: `Tool::Trim`)

- **O que faz:** remove o trecho de uma curva entre interseções.
- **CAD avançado B — no detalhe:**
  - O *Quick Trim* fica no grupo de relimitações. Clica-se no trecho a remover; o resultado depende de onde o cursor cai [V].
  - Modos de *Quick Trim*: *Break & Rubber Out*, *Break & Rubber In*, *Break & Keep*. Duas fontes discordam sobre o sentido de "Rubber In/Out" [V], então defina o comportamento por teste no produto, não pelas fontes.
  - O *Trim* clássico tem o modo "cortar todos os elementos" ou "só o primeiro" [V parcial]. Há ainda o modo de seleção por mão livre (*Free Hand*) combinado com os modos acima [V CERN].
- **CAD avançado A — no detalhe:** [V GoEngineer]
  - **Power Trim:** arrasta-se um caminho e tudo que o caminho toca some. Se só o trecho central for removido, a linha original é dividida e as partes restantes ganham relações para continuarem posicionadas. Também **estende** (clicar, mover até a ponta, clicar de novo, resultado numa entidade).
  - **Corner:** estende ou apara duas entidades até o encontro; sobra além do canto é removida.
  - **Trim Away Inside / Outside:** duas entidades de fronteira; remove dentro ou fora.
  - **Trim to Closest:** clique por segmento, sem arrasto; mais seguro em sketch denso.
  - O sketch aparado precisa ser aberto e cada entidade cruza a fronteira no máximo uma vez [V].
- **CAD avançado C — no detalhe:** [V] aparar até a interseção mais próxima ou até fronteiras definidas, criando restrição **coincidente** na ponta nova. Curva sem interseção é **apagada**. **Shift** alterna entre aparar e estender; **Ctrl** seleciona geometria inteira como fronteira; **arrastar** apara tudo por onde o mouse passa (*dynamic*).
- **Fenkra hoje:**
  - `trim_line` só em linhas: corta nas interseções com todas as outras curvas (linhas, círculos, arcos; splines ignoradas) e remove o trecho clicado [código `sketch_ui.rs:788–887`].
  - **Problemas prováveis:**
    - O novo ponto de corte fica livre. **Não há `PointOnCurve` nem coincidência** com a curva cortadora (o CAD avançado C cria) [código `sketch_ui.rs:870–883`], então mover a curva cortadora deixa o extremo para trás.
    - Ao dividir em duas, só a primeira metade conserva o id da linha, e **as restrições H/V/paralela da linha ficam só nela**; a segunda metade nasce sem nada (o CAD avançado C mantém H, V, paralela, perpendicular e colinear nas duas partes [V]).
    - `set_line` apaga cotas `Length` da linha, silenciosamente [código `sketch_ui.rs:894`].
    - A busca de interseção inclui geometria de construção [código `sketch_ui.rs:791`], o que pode aparar contra linha de apoio sem o usuário perceber [C: cada CAD tem política própria].
    - Não apara arco, círculo nem spline.
- **Melhor prática:**
  - Modos: clique por segmento (aparar ao mais próximo), arrasto (Power Trim), canto, dentro/fora.
  - Fronteira: por padrão toda geometria de perfil; Ctrl restringe; construção só se pedida.
  - Saída: o ponto novo ganha `Coincident`/`PointOnCurve` com a fronteira; as restrições geométricas da curva original passam para as duas partes quando fizerem sentido; cotas de comprimento são removidas **com aviso** listando o que sumiu.
  - Círculo aparado vira arco; arco aparado continua arco; spline aparada vira spline com pontos novos.
  - **Testes:** (a) `aparar_e_estender` existente + (b) aparar e mover a fronteira, a ponta acompanha; (c) linha horizontal dividida mantém `Horizontal` nas duas partes; (d) aparar círculo cruzado por duas linhas gera arco; (e) aparar curva sem interseção apaga; (f) desfazer restaura cotas.

### 2.22 Quebrar / dividir (CAD avançado B: Break / CAD avançado A: Split Entities / CAD avançado C: Split / Fenkra: ausente)

- **O que faz:** divide uma curva em duas num ponto, sem apagar nada.
- **CAD avançado B — no detalhe:** *Break* separa a curva em dois pontos de ruptura; o grupo de relimitações também tem *Close* e *Complement* [V parcial SlideShare].
- **CAD avançado A — no detalhe:** o ponto de divisão pode ser removido para refundir; círculo, elipse inteira e spline fechada exigem **dois** pontos [V].
- **CAD avançado C — no detalhe:** [V] divide ao nome mais próximo e **mantém as cotas**; H, V, paralela, perpendicular e colinear permanecem nas duas partes; igual e simétrica são quebradas quando necessário.
- **Fenkra hoje:** ausente (DR-51).
- **Melhor prática:** dividir em ponto escolhido, em interseção ou no meio; o ponto novo é compartilhado pelas duas metades (sem lacuna); círculo pede dois pontos; refundir desfaz. **Teste:** dividir e refundir devolve a curva original, com as mesmas restrições.

### 2.23 Estender e relimitar (CAD avançado B: Quick Trim/Trim com extensão, Corner / CAD avançado A: Extend Entities, Power Trim estender / CAD avançado C: Extend / Fenkra: `Tool::Extend`)

- **O que faz:** prolonga uma curva até a próxima interseção.
- **CAD avançado B — no detalhe:** o *Trim* (primeiro/todos os elementos) estende ou encurta até a interseção [V parcial].
- **CAD avançado A — no detalhe:** *Corner* e *Power Trim* estendem; "Estender" se faz com uma entidade só, sem criar cota nova [V].
- **CAD avançado C — no detalhe:** [V] até a curva mais próxima ou fronteiras, com coincidente na ponta; **Shift** troca com Trim.
- **Fenkra hoje:** `extend_line` só linhas; a ponta mais perto do clique vai até a próxima curva na direção; sem interseção, erro de texto [código `sketch_ui.rs:976–996`]. O ponto novo é livre (mesma questão da seção 2.21).
- **Melhor prática:** estender arcos (mesmo raio), splines (continuação tangente) e **estender até o encontro de duas linhas** (canto). Prender o extremo na curva alvo. **Teste:** estender linha não paralela até outra que depois muda de posição, a ponta acompanha.

### 2.24 Fechar e complementar arco (CAD avançado B: Close, Complement / CAD avançado A: —(arco completa pelo círculo) / CAD avançado C: — / Fenkra: ausente)

- **O que faz:** *Close* transforma um arco em círculo inteiro; *Complement* cria o arco que falta [V parcial: descrições de curso].
- **Fenkra hoje:** ausente (DR-52). Usuários de CAD avançado B usam isto para perfis com furos de arcos.
- **Melhor prática:** *Complementar* cria o arco do mesmo centro e raio entre os extremos, com os mesmos pontos compartilhados e um `Equal` ou, melhor, **os mesmos pontos de centro**; *Fechar* troca o arco por círculo. **Teste:** arco + complemento fecham perfil de 360° sem sobrepor.

### 2.25 Offset (CAD avançado B: Offset / CAD avançado A: Offset Entities / CAD avançado C: Offset / Fenkra: `Tool::Offset`)

- **O que faz:** cria curvas paralelas a uma distância.
- **CAD avançado B — no detalhe:** *Offset* com modos *No Propagation*, *Tangent Propagation* e *Point Propagation*, e opção de repetir a ação e de criar em vários lados [C]. Os resultados são geometria nova com restrição de offset.
- **CAD avançado A — no detalhe:** [V] **Select chain** (todas as entidades contíguas); **Bi-directional**; **construção** para base, offset ou ambos; **Cap ends** com arco ou linha ao oferecer duas direções; existe uma relação de offset que mantém a distância se o original mudar. **Não** aceita splines de ajuste, splines já deslocadas, ou casos que se autointerceptam.
- **CAD avançado C — no detalhe:** [V] *Constrain Offset* aplica restrições de modo que **uma** cota posiciona o conjunto, e *Loop Select* seleciona toda a geometria plana conectada.
- **Fenkra hoje:** uma curva por vez (linha, círculo ou arco). Linha: nova linha paralela + `Parallel` + `PointLineDistance` de **uma ponta só**; o comprimento do offset fica livre [código `sketch_ui.rs:1975–1987`]. Círculo: novo círculo no mesmo centro + `Diameter` [código `sketch_ui.rs:1988–1993`]. Arco: mesmos centros e `Radius`; os extremos novos **não** ficam presos às semirretas dos extremos antigos, então o arco pode deslizar [código `sketch_ui.rs:1994–2007`]. Sem cadeia, sem spline, sem os dois lados, sem fechamento (*cap*) e sem manter a relação "offset de".
- **Melhor prática:**
  - Suporte: curva ou cadeia (Loop Select) → modo (uma direção, bi-direcional, cap) → distância → avançado (construção, propagação por tangência).
  - Salvar `OffsetOf { src, dist }` como restrição única, para atualizar com a base.
  - Para spline, aproximar por polilinha só com aviso, ou recusar.
  - Pontas do offset ligadas às da base por perpendicularidade, para o comprimento seguir.
  - **Testes:** (a) offset de retângulo fechado com uma cota move os quatro lados; (b) aumentar a base atualiza o offset; (c) offset de arco mantém ângulo varrido; (d) offset autointerceptante é recusado.

### 2.26 Espelhar e simetria (CAD avançado B: Mirror, Symmetry / CAD avançado A: Mirror Entities, Dynamic Mirror / CAD avançado C: Mirror / Fenkra: `Tool::Mirror`)

- **O que faz:** duplica a geometria em relação a um eixo, com ou sem manter a ligação.
- **CAD avançado B — no detalhe:** *Mirror* copia (com restrições de simetria) e *Symmetry* move (sem copiar) [V parcial: título do tutorial GrabCAD]. O contexto do usuário alerta: restrições duplicadas geram conflito; desative "copy constraints" em perfis complexos [`docs/contextos/sketcher.md`].
- **CAD avançado A — no detalhe:** *Mirror Entities* e *Dynamic Mirror*; a cópia mantém relação de simetria; a linha de espelho pode ser construção [C].
- **CAD avançado C — no detalhe:** espelho de seleção em relação a uma linha, com restrições de simetria [C, não confirmado].
- **Fenkra hoje:** copia pontos refletidos, copia as curvas (arco inverte `s` e `e`) e cria `Symmetric(p, q, eixo)` para **cada par de pontos** [código `sketch_ui.rs:1921–1959`]. Limites:
  - o raio do círculo copiado é uma **variável independente**: sem `Equal` entre o original e o espelhado (a simetria de centros não iguala o raio);
  - as tangentes de spline (`SplineTan`) não são copiadas;
  - as restrições internas (H, V, Equal, Tangent, cotas) da seleção não são copiadas;
  - a linha de espelho não pode estar na seleção.
- **Melhor prática:** espelhar copia também restrições internas por correspondência e acrescenta `Equal` para raios; opção de "só simetria"; dinâmico (espelhar durante o desenho). **Testes:** (a) círculo espelhado mantém raio ao mudar o original; (b) perfil espelhado com `Tangent` mantém tangência; (c) espelhar spline preserva tangentes.

### 2.27 Mover, copiar, girar, escalar, esticar (CAD avançado B: Translate, Rotate, Scale, Symmetry / CAD avançado A: Move, Copy, Rotate, Scale, Stretch Entities / CAD avançado C: Move, Copy, Rotate, Scale, Stretch / Fenkra: `Move`, `Copy`)

- **O que faz:** transforma geometria selecionada.
- **CAD avançado B — no detalhe:** *Translate*, *Rotate*, *Scale* e *Symmetry* no mesmo grupo, com opção de **duplicar** o resultado e de manter ou não as restrições [V parcial].
- **CAD avançado A — no detalhe:** [V] *Move/Copy* e *Rotate/Scale/Stretch*. A caixa **Keep relations** decide se as relações viajam: desmarcada, rompem-se só as relações entre selecionados e não selecionados. As cotas acompanham. *Rotate* usa centro e ângulo; *Scale* usa ponto-base e fator, com cópia opcional; *Stretch* move pontos-base.
- **CAD avançado C — no detalhe:** [C] move, copia, gira, escala, estica; as restrições dos copiados ficam se a opção estiver ligada.
- **Fenkra hoje:**
  - **Mover:** soma o deslocamento aos pontos não fixos e resolve com esses pontos presos [código `sketch_ui.rs:1889–1898`].
  - **Copiar:** cria pontos e curvas novos; **nenhuma restrição ou tangente de spline é copiada** [código `sketch_ui.rs:1899–1917`].
  - A prévia só desenha linhas [código `sketch_ui.rs:1873–1877`].
  - Sem girar, escalar, esticar nem entrada numérica de deslocamento.
- **Melhor prática:** painel com modo (mover, copiar, girar, escalar, esticar), base e destino com entrada numérica, caixa "manter restrições", prévia de todas as curvas (arco, círculo, spline). **Testes:** (a) copiar retângulo mantém H/V; (b) girar 90° um perfil totalmente restrito mantém o estado; (c) escalar 2× atualiza cotas só se o usuário pedir.

### 2.28 Padrão no sketch (CAD avançado B: — (usa feature) / CAD avançado A: Linear/Circular Sketch Pattern / CAD avançado C: Rectangular, Circular, Mirror / Fenkra: ausente)

- **O que faz:** repete geometria em grade ou em arco com cotas de espaçamento.
- **CAD avançado A — no detalhe:** [V] padrões linear e circular com espaçamento, ângulo e número; *Instances to Skip* omite ocorrências.
- **CAD avançado C — no detalhe:** [V] as cotas de espaçamento de padrão retangular podem ser **de referência**; um usuário relata pontas que não se ligam após o padrão (opinião, não documentação).
- **Fenkra hoje:** ausente (BM-SKT-06). O padrão 3D existe como feature; para furos o usuário depende dele.
- **Melhor prática:** o padrão de sketch é uma **entidade paramétrica**: guarda a geometria-semente e (n, passo) ou (n, ângulo). As cópias são geradas, não editáveis, e a semente ganha pontos de controle. Pular ocorrências. Preferir padrão de feature quando a repetição é de sólido. **Teste:** mudar n de 4 para 6 atualiza o perfil e o sólido sem refazer cotas.

### 2.29 Filete de sketch (CAD avançado B: Corner / CAD avançado A: Sketch Fillet / CAD avançado C: Fillet / Fenkra: `Tool::Fillet`)

- **O que faz:** arredonda o canto entre duas curvas com um arco tangente.
- **CAD avançado B — no detalhe:** *Corner* tem modos de aparar/manter as linhas originais, e gera arco tangente [V parcial: curso cita opções de trim].
- **CAD avançado A — no detalhe:** apara o canto e cria o arco tangente; a cota do raio é pedida na mesma hora [V parcial].
- **CAD avançado C — no detalhe:** [C] filete entre duas curvas, com raio igual opcional para os vários cantos.
- **Fenkra hoje:** clica-se no ponto do canto com exatamente duas linhas; o raio inicial é `0,3·min(l1, l2)·tan(θ/2)`; as linhas passam a terminar nos pontos de tangência; cria `Tangent` ×2 e `Radius`, abre a cota para editar e remove o ponto do canto [código `sketch_ui.rs:2065–2125`].
- **Erros prováveis:**
  - **O código não apaga as cotas `Length` das linhas encurtadas** (o aparar apaga em `set_line`, mas o filete e o chanfro mexem em `a`/`b` direto). Num retângulo com `auto_dims`, a linha de 40 mm continua a medir 40 mm depois do filete, e o solver **aumenta o retângulo** para satisfazer a cota.
  - O ponto do canto antigo é removido, e `prune_constraints` apaga qualquer restrição que o usava (cota de distância, `PointOnCurve`) sem aviso.
  - Só funciona com duas linhas; arco com linha, arco com arco e spline ficam de fora.
- **Melhor prática:**
  - Fluxo: canto, ou seleção de duas curvas → raio (digitado) → avançado (cantos múltiplos com o mesmo raio, "manter canto" como construção).
  - Ao encurtar linhas, migrar a cota de comprimento para uma cota virtual do comprimento até o canto original, que vira linha de construção (o chanfro já faz algo assim) e avisar.
  - **Testes:** (a) filetar os quatro cantos de um retângulo 40×20 mantém 40×20 externo; (b) filete entre linha e arco; (c) filete em canto com cota de distância mantém a cota; (d) raio maior que o lado permitido é recusado.

### 2.30 Chanfro de sketch (CAD avançado B: Chamfer / CAD avançado A: Sketch Chamfer / CAD avançado C: Chamfer / Fenkra: `Tool::Chamfer`)

- **O que faz:** corta o canto com uma linha reta.
- **CAD avançado B / CAD avançado A / CAD avançado C:** opções de distância–distância, distância–ângulo e dois valores; cotas criadas automaticamente [C].
- **Fenkra hoje:** só distâncias iguais (`t = 0,3·min`), com canto antigo virando duas linhas de construção, `Equal`, `Parallel`, `PointOnCurve` e `Length` do cateto [código `sketch_ui.rs:2014–2064`]. Mesmo risco de cotas de comprimento (seção 2.29).
- **Melhor prática:** acrescentar "distância–ângulo" e "duas distâncias". **Teste:** chanfro 5×5 num canto de 40×20 não altera o tamanho externo.

---

### 2.31 Projetar e converter geometria 3D (CAD avançado B: Project 3D Elements, Project 3D Silhouette Edges / CAD avançado A: Convert Entities / CAD avançado C: Project Geometry, Project Cut Edges / Fusion: Project, Include 3D Geometry / CAD avançado E: Use / Fenkra: `Tool::Convert`)

- **O que faz:** traz arestas, vértices e curvas do modelo para o plano do sketch, com vínculo.
- **CAD avançado B — no detalhe:** [V]
  - A projeção aparece em **amarelo** e é **associativa** ao 3D.
  - Escolher uma face ou um pad projeta suas **arestas**.
  - *Project 3D Silhouette Edges* projeta o contorno de superfícies canônicas (cilindro).
  - Para soltar o vínculo: menu de contexto > objeto marcado > **Isolate**; a linha fica **branca** (independente) e precisa ser restringida para o sketch continuar totalmente definido.
  - Mudanças no corpo-pai "se propagam" ao sketch novo e podem causar efeitos inesperados.
- **CAD avançado A — no detalhe:** *Convert Entities* projeta arestas no plano; o vínculo mantém o sketch atualizado quando a fonte muda [V resumo]. Relação perdida fica amarela [V]. Seleção de cadeia e de laços internos/externos é [C].
- **CAD avançado C — no detalhe:** [V]
  - Geometria projetada é **associativa** e fica fixa em relação ao sketch de origem. O usuário pode **quebrar o vínculo** (*Edit Projected Geometry*) para editá-la como geometria comum.
  - Splines vindas do ambiente de construção viram splines de referência não associativas.
  - As **arestas de corte projetadas** (*Project Cut Edges*) de uma seção de montagem **não são associativas**: são um instantâneo.
  - Opções de aplicação para autoprojetar arestas ao criar curva.
- **Fusion:** *Include 3D Geometry* traz arestas, geometria de trabalho e curvas de sketch; *Project to Surface* projeta sobre uma face; *Intersection Curve* gera curva 3D [V].
- **CAD avançado E:** *Use* cria geometria real mais uma restrição ao vínculo; se a fonte mudar de tipo (círculo → elipse), o *Use* falha; aresta apagada deixa a entidade *dangling* [V].
- **Fenkra hoje:**
  - Cria um `SkLink { source, intersect, shape }` com fontes `Ref`, `SketchCurve`, `Edge{mid}`, `Vertex{pos}`, `Face{point}` [código `sketch.rs:430–461`].
  - **A forma é uma polilinha de pontos fixos**: círculo fechado com ≥ 8 amostras vira círculo; qualquer outra curva vira trechos retos ou **splines** conforme o giro (`smooth_runs`, limiar 0,45 rad) [código `sketch.rs:187`, `489`, `514–556`]. Um arco projetado **não é arco**: não tem centro nem raio para cotar.
  - Atualização: `update_link` reamostra a polilinha em n pontos iguais e reescreve as coordenadas por índice [código `sketch.rs:559–588`]. Se a topologia mudar, a correspondência por índice pode inverter ou torcer a curva.
  - `Edge { mid }` reencontra a aresta pelo ponto médio: se outra aresta passar a ocupar esse lugar, **o vínculo troca de geometria em silêncio**, contra o princípio 1 da ADR 0011 [código `sketch.rs:438`].
  - Sem isolar/quebrar vínculo; apagar os pontos apaga o vínculo [código `sketch_ui.rs:999–1007`].
  - Sem cor de vínculo (amarelo), sem estado "perdido".
- **Melhor prática:**
  - Fluxo: suporte (aresta, face, laço, sketch, plano) → modo (projetar / só referência / cadeia) → parâmetros (como construção) → avançado (isolar).
  - Reconhecer **linha, arco, círculo e elipse** exatamente (reta por 2 pontos, arco por 3 pontos e centro) e ligar o vínculo ao parâmetro (centro, raio), nunca à polilinha.
  - Reencontrar por chave estável (dono + papel, ADR 0010), com diagnóstico **"vínculo perdido"** (amarelo) em vez de substituir.
  - Comandos *Isolar vínculo* e *Reunir*.
  - Cortes de montagem: instantâneo marcado como tal.
  - **Testes:** (a) projetar um arco e cotar o raio; (b) mover a aresta fonte e conferir o sketch; (c) apagar a fonte e esperar estado "perdido", não troca; (d) isolar e conferir que o 3D não afeta mais; (e) o círculo com menos de 8 amostras continua sendo círculo.

### 2.32 Interseção com o 3D (CAD avançado B: Intersect 3D Elements / CAD avançado A: Intersection Curve / CAD avançado C: Project Cut Edges, 3D Intersection / Fenkra: `Tool::Intersect`)

- **O que faz:** gera a curva ou o ponto em que uma face, superfície ou aresta cruza o plano do sketch.
- **CAD avançado B — no detalhe:** *Intersect 3D Elements* funciona como a projeção, dando a linha de corte; as marcas são amarelas e associativas [V].
- **CAD avançado A — no detalhe:** *Intersection Curve* cria curva de sketch na interseção de planos, corpos sólidos e superfícies [V resumo].
- **CAD avançado C — no detalhe:** *Project Cut Edges* usa seção de montagem e não é associativa [V].
- **Fenkra hoje:** `Tool::Intersect` corta a malha por polígonos (`mesh_cut`) e encadeia segmentos; `crossing()` acha **um único ponto** por curva [código `sketch_ext.rs:39–93`]. Falha com perfis em outro plano (DR-06).
- **Melhor prática:** interseção exata no kernel (OpenCascade) em vez de malha; devolver todos os pontos e curvas; mesmo tratamento de vínculo da seção 2.31. **Teste:** cilindro inclinado cortado pelo plano gera elipse com semieixos corretos, e o plano que cruza três vezes uma aresta devolve três pontos.

### 2.33 Geometria externa e política de associatividade (CAD avançado B: Sketch → Isolate, Update Diagnosis / CAD avançado A: Display/Delete Relations, Replace / CAD avançado C: Break Link / Fenkra: `links`, DR-18)

- **O que faz:** define o que acontece com a geometria externa quando a fonte muda ou some.
- **CAD avançado B — no detalhe:** a geometria projetada **segue o pai**; **Isolate** a torna independente; se o pai some, o diagnóstico de atualização pede para substituir ou isolar [V parcial; C para o último ponto].
- **CAD avançado A — no detalhe:** [V] relação perdida fica amarela; *Display/Delete Relations* lista e o *Replace* reaponta; recomendação do CADimensions: reanexar, substituir ou reparar.
- **CAD avançado C — no detalhe:** [V] quebrar vínculo; cortes de montagem são instantâneos. A pesquisa do benchmark do Fenkra já recomenda evitar projeções sem associatividade [`docs/benchmark/sketch.md`].
- **Fenkra hoje:** vínculo sempre associativo; sem política por sketch (DR-18).
- **Melhor prática:**
  - Política por vínculo: *associativo*, *isolado* e *instantâneo*, com estado visível (`->`, `->?`, `->*`, `->x`, ADR 0011 princípio 3).
  - Ação em "perdido": Substituir, Isolar, Excluir.
  - **Teste:** apagar a aresta fonte gera `->x` e mostra opções; "Isolar" converte para geometria comum e cota normalmente.

### 2.34 Sketch 3D e texto/imagem (CAD avançado B: 3D Wireframe / CAD avançado A: 3D Sketch, Text, Sketch Picture / CAD avançado C: 3D Sketch, Text, Image / Fenkra: ausente)

- **O que faz:** curvas no espaço e anotações no sketch.
- **CAD avançado C — no detalhe:** [V] o *3D sketch* aceita Split/Trim/Extend contra face, plano de trabalho e superfície, e projeção de curvas 2D sobre faces.
- **Fenkra hoje:** sem sketch 3D com restrições (BM-SKT-08) e sem texto/imagem de referência (BM-SKT-07). Existem polilinhas, spline e hélice 3D no wireframe.
- **Melhor prática:** sketch 3D em segunda onda, com restrições de plano e de eixo; texto como contorno paramétrico (fonte, altura) e imagem como referência não exportada. **Teste:** texto vira perfil fechado extrudável.

### 2.35 Seleção, arrasto e manipulação direta (CAD avançado B: Select, Drag / CAD avançado A: drag / CAD avançado C: drag, cursor menu / Fenkra: `Tool::Select`)

- **O que faz:** selecionar e arrastar elementos mantendo restrições.
- **CAD avançado A — no detalhe:** arrastar uma entidade azul mostra o que falta [V]. Seleção por janela e cruzada [C].
- **CAD avançado C — no detalhe:** arrastar curva sub-restrita revela graus de liberdade [C].
- **Fenkra hoje:** arrasta ponto, linha (pelos dois pontos) e círculo (pelo raio); arrasto de **texto de cota** também [código `sketch_ui.rs:1441–1498`]. O arco e a spline não arrastam; seleção por janela/cruzada ausente (DR-05); sem seleção em cadeia nem "todos os conectados". O botão direito abre menu rápido com cotas, restrições, construção, excluir e ferramentas [código `sketch_ui.rs:197–209`].
- **Melhor prática:** arrastar o arco (centro, raio, ponto), selecionar por janela e cruzada, duplo clique para a cadeia, glifos de DOF ao passar o mouse. **Testes:** (a) arrastar um arco tangente não quebra a tangência; (b) seleção cruzada pega curvas parcialmente dentro.

---

## 3. Requisitos levantados

Prioridade P0 = corrige erro de modelagem ou perde dado; P1 = paridade essencial com CAD avançado B/CAD avançado A/CAD avançado C; P2 = produtividade; P3 = futuro.

### Funcionais (RF-SKT)

| # | Requisito | Prio | Critério de aceite testável |
|---|---|---|---|
| RF-SKT-01 | Estados do sketch: sub, totalmente, **redundante**, **conflito** e **vínculo perdido**, com cores e texto na barra, no painel e na árvore | P0 | Sketch com cota duplicada consistente exibe "Redundante"; contradição exibe "Conflito" com as 2 restrições; teste de unidade sobre `SolveReport` |
| RF-SKT-02 | Cota de **referência** (dirigida): cota redundante vira referência sem alterar a geometria | P0 | Adicionar cota redundante oferece "tornar referência"; valor aparece entre parênteses; o DOF não muda |
| RF-SKT-03 | Em conflito, **não aplicar** a solução parcial; manter a última geometria válida | P0 | Cota contraditória: coordenadas dos pontos iguais às anteriores; restrições culpadas destacadas |
| RF-SKT-04 | Filete e chanfro preservam as dimensões externas (migrar ou avisar sobre cotas de comprimento das linhas encurtadas) | P0 | Retângulo 40×20 com filetes nos 4 cantos mantém 40×20 externo; aviso lista a cota migrada |
| RF-SKT-05 | Aparar/estender prendem o novo extremo na curva alvo (`PointOnCurve`/`Coincident`) e propagam as restrições da curva original às duas partes | P0 | Aparar e mover a fronteira: a ponta acompanha; a linha dividida mantém `Horizontal` nas duas |
| RF-SKT-06 | Espelhar/copiar levam restrições internas, `Equal` de raios e tangentes de spline | P0 | Círculo espelhado muda de raio junto com o original; spline espelhada mantém `SplineTan` |
| RF-SKT-07 | `merge_points` atualiza `spline_tan` e `links`; `remove_curve` limpa `spline_tan` órfão | P0 | Teste de unidade: fundir ponto de spline com tangente própria não deixa referência pendente |
| RF-SKT-08 | `PointOnCurve` funciona com spline e respeita o limite angular de arco | P0 | Ponto sobre spline mantém distância ≈ 0 após arrastar; ponto sobre arco recusa posição fora do arco |
| RF-SKT-09 | Coincidência explícita: `Coincident` listada, removível, com os dois pontos preservados ou fundidos conforme o modo | P1 | Remover coincidência separa os pontos; perfil volta a abrir |
| RF-SKT-10 | Inferência ao desenhar: snap em ponto médio, interseção, centro, quadrante, sobre a curva; restrições inferidas (coincidente, tangente, perpendicular, paralela, concêntrico) com glifo e desligável por tipo | P1 | Clicar sobre linha cria `PointOnCurve`; guia de 90° cria `Perpendicular`; com inferência desligada nenhuma restrição é criada |
| RF-SKT-11 | Cota inteligente: modos min/max/centro para arcos e círculos, comprimento e ângulo de arco, ângulo contra horizontal, ponto–origem, cota com sinal | P1 | Cotar arco por raio, comprimento e ângulo converge para o mesmo arco; cota X negativa preserva o lado |
| RF-SKT-12 | **Nome** em toda cota (`d1…`) e referência em expressões | P1 | `d2 = d1/2` atualiza ao editar `d1`; renomear mantém a referência |
| RF-SKT-13 | Completar sketch (cotas e restrições automáticas com prévia, esquemas cadeia/linha de base, datum) e contador "faltam N" | P1 | Perfil em L chega a DOF = 0 aplicando exatamente o número de itens mostrado antes |
| RF-SKT-14 | Analisar sketch: painel com perfis abertos/fechados, sobreposições, autocruzamentos, referências externas e restrições, com zoom e correção | P1 | Gap de 0,0005 mm é reparado; duas linhas sobrepostas são fundidas; figura de oito é apontada |
| RF-SKT-15 | Aparar rápido (clique e arrasto), aparar a curva mais próxima, canto, dentro/fora; vale para linha, arco, círculo e spline | P1 | Círculo cruzado por duas linhas vira arco ao aparar; arrasto remove todos os trechos tocados |
| RF-SKT-16 | Quebrar/dividir (com refundir), fechar e complementar arco | P1 | Dividir e refundir devolve a curva e as restrições originais |
| RF-SKT-17 | Estender arco, spline e "até o encontro" (canto) | P1 | Estender duas linhas não paralelas até o cruzamento; arco estendido mantém centro e raio |
| RF-SKT-18 | Offset de cadeia, bi-direcional, com tampa e construção; relação "offset de" que acompanha a base | P1 | Offset de retângulo fechado com uma cota; aumentar a base atualiza o offset |
| RF-SKT-19 | Mover/copiar/girar/escalar/esticar com entrada numérica, prévia completa e opção "manter restrições" | P1 | Copiar retângulo mantém H/V; girar 90° mantém o estado |
| RF-SKT-20 | Padrão linear e circular paramétrico no sketch com ocorrências puladas | P2 | Mudar n de 4 para 6 atualiza o perfil sem refazer cotas |
| RF-SKT-21 | Projeção/conversão reconhece linha, arco, círculo e elipse exatos e liga o vínculo aos parâmetros | P1 | Projetar um arco permite cotar raio; mover a aresta fonte move o arco |
| RF-SKT-22 | Vínculos com política e estado visível: associativo, isolado, instantâneo; "perdido" nunca troca geometria | P1 | Apagar a aresta fonte gera `->x` com Substituir/Isolar/Excluir; nada é reescolhido em silêncio |
| RF-SKT-23 | Interseção exata no kernel com todos os pontos e curvas | P1 | Plano que cruza uma aresta 3 vezes devolve 3 pontos |
| RF-SKT-24 | Posicionamento do sketch: origem (projeção, ponto, baricentro) e orientação (padrão, alinhar a aresta, trocar/inverter H e V) | P1 | Trocar H/V de sketch restrito gira o sólido como esperado; apagar o suporte gera erro visível |
| RF-SKT-25 | Elipse, parábola/hipérbole/cônica e rasgo em arco/keyhole | P2 | Extrudar elipse cotada gera sólido de volume πab·h; cônica com rho = 0,5 é parábola |
| RF-SKT-26 | Spline: fechada, modo CV, pente de curvatura, tangência com outras curvas e comprimento travável | P2 | Spline fechada gera perfil fechado; cota geral trava o comprimento |
| RF-SKT-27 | Arrasto de arco e spline; seleção por janela e cruzada; seleção de cadeia | P2 | Arrastar arco tangente não quebra a tangência; seleção cruzada pega curvas parciais |
| RF-SKT-28 | Retângulo de 3 pontos e paralelogramo; polígono com n e tipo editáveis | P2 | Retângulo a 30° totalmente restrito com 2 cotas, 1 ângulo e fixação |
| RF-SKT-29 | Glifos de DOF por geometria (opcionais) e arrastar mostra o que falta | P2 | Linha com ponta fixa mostra apenas o glifo de rotação |
| RF-SKT-30 | Texto e imagem de referência no sketch; sketch 3D com restrições | P3 | Texto vira perfil fechado extrudável |
| RF-SKT-31 | Tangência interna entre círculos, colinear, simetria em relação a ponto, ponto médio em arco | P2 | Tangente interna fecha com `dist = |r1 − r2|`; colinear mantém duas linhas na mesma reta |

### Não funcionais (RNF-SKT)

| # | Requisito | Prio | Critério de aceite testável |
|---|---|---|---|
| RNF-SKT-01 | Tolerância de coincidência configurável por documento, padrão 0,001 mm, usada de modo único em `chains()`, verificação e reparo | P0 | `loops()` fecha perfil com gap de 0,0005 mm e não fecha com 0,01 mm |
| RNF-SKT-02 | Solver com orçamento de tempo: ≤ 16 ms por arrasto até 200 pontos; esparso ou analítico acima disso | P1 | Benchmark de 200 e 500 pontos em cadeia; falha o CI se passar do orçamento definido |
| RNF-SKT-03 | Determinismo: o mesmo sketch resolve ao mesmo resultado entre execuções e após salvar/abrir | P0 | Sketch totalmente restrito não muda (< 1e-9) em ciclo salvar–abrir–recalcular |
| RNF-SKT-04 | Corpus de testes por comando (B53): cada ferramenta tem teste nomeado de criar, mudar cota e voltar | P0 | Cada item da seção 2 com pelo menos 1 teste; CI roda `cargo test` e falha se faltar |
| RNF-SKT-05 | Nenhuma operação apaga cota ou restrição em silêncio; toda remoção é listada e desfazível num passo | P0 | Aparar/filetar/apagar ponto mostram o aviso com a lista e o desfazer restaura |
| RNF-SKT-06 | Compatibilidade de arquivo: sketches antigos abrem com migração (`serde(default)` nos campos novos) | P0 | Abrir `.fkp` anterior sem perdas; campos novos recebem padrão |
| RNF-SKT-07 | Mensagens e dicas em português com fluxo suporte → modo → parâmetros → avançado | P2 | Revisão de i18n; toda ferramenta tem as duas dicas (ADR 0011, princípio 6) |
| RNF-SKT-08 | Falha de convergência não derruba a UI nem corrompe o documento | P0 | Sketch com restrições impossíveis fica interativo e reversível |
| RNF-SKT-09 | Acessibilidade visual dos estados: cor + forma (cor sozinha não basta) | P3 | Cada estado tem ícone ou padrão além da cor |

---

## 4. Itens de backlog propostos

Os códigos "SK-xx" são provisórios; o lead pode renumerar. Os itens existentes citados vêm de `docs/benchmark/sketch.md`.

| Título | O que muda | Prio | Tam. | Dependências |
|---|---|---|---|---|
| SK-01 Estados de solver e cota de referência | Expor `redundant`, estados Sub/Totalmente/Redundante/Conflito/Perdido, cores, selo na árvore, cota dirigida; conflito não aplica solução parcial (RF-SKT-01/02/03) | P0 | M | B59 (árvore e diagnóstico) |
| SK-02 Correção de erros de modelagem | Filete/chanfro preservam o tamanho externo; aparar/estender prendem a ponta; espelhar/copiar levam restrições e raios; `merge_points`/`remove_curve` mantêm `spline_tan` e `links`; `PointOnCurve` com spline e arco (RF-SKT-04 a 08) | P0 | M | B53 (corpus) |
| SK-03 Testes por comando do sketch | Um teste nomeado para cada ficha da seção 2; filete, espelhar, offset, converter (RNF-SKT-04) | P0 | M | B53 |
| SK-04 Tolerância e nada em silêncio | Tolerância única de 0,001 mm; avisos de remoção de cotas/restrições com desfazer (RNF-SKT-01/05) | P0 | P | — |
| SK-05 Inferência e snaps | Snaps em curva, ponto médio, interseção, centro, quadrante; restrições inferidas por tipo com glifo (RF-SKT-10) | P1 | G | SK-02 |
| SK-06 Cota inteligente 2.0 | Min/max, arco (comprimento e ângulo), cota com sinal, nomes `d1…` e referência em expressões, completar sketch (RF-SKT-11/12/13) | P1 | G | SK-01 |
| SK-07 Analisar e reparar sketch | Painel com perfis, sobreposições, autocruzamento, referências e reparo por tolerância (RF-SKT-14) | P1 | M | SK-04, B59 |
| SK-08 Relimitação completa | Aparar rápido por arrasto, canto, dentro/fora, quebrar, fechar, complementar, estender arco e spline (RF-SKT-15/16/17); absorve DR-50, DR-51, DR-52 | P1 | G | SK-02 |
| SK-09 Offset e transformações | Offset de cadeia, bi-direcional, tampa e relação; mover/copiar/girar/escalar/esticar com entrada numérica (RF-SKT-18/19) | P1 | M | SK-02 |
| SK-10 Projeção exata e vínculos com estado | Reconhecer linha/arco/círculo/elipse, ligar o vínculo aos parâmetros, chave estável, política associativo/isolado/instantâneo, estado `->x`, interseção exata (RF-SKT-21/22/23); absorve DR-06 e DR-18 | P1 | G | ADR 0010, B55 |
| SK-11 Posicionamento do sketch | Origem e orientação do sketch, trocar/inverter H/V, sketch em referência (RF-SKT-24) | P1 | M | ADR 0010 |
| SK-12 Padrão de sketch | Linear e circular paramétrico com ocorrências puladas (RF-SKT-20); BM-SKT-06 | P2 | M | SK-09 |
| SK-13 Curvas novas | Elipse, cônicas, rasgo em arco, keyhole, retângulo de 3 pontos e paralelogramo (RF-SKT-25/28); BM-SKT-01/02/03 | P2 | G | kernel |
| SK-14 Spline avançada | Fechada, modo CV, pente de curvatura, tangência com outras curvas e comprimento (RF-SKT-26); DR-02 | P2 | G | SK-02 |
| SK-15 Manipulação direta | Arrasto de arco/spline, seleção por janela e cruzada, glifos de DOF (RF-SKT-27/29); DR-05 | P2 | M | SK-01 |
| SK-16 Restrições extras | Tangência interna, colinear, simetria por ponto, ponto médio em arco (RF-SKT-31) | P2 | P | SK-02 |
| SK-17 Desempenho do solver | Jacobiano analítico ou esparso, orçamento de tempo (RNF-SKT-02) | P1 | G | SK-03 |
| SK-18 Texto, imagem e sketch 3D | Texto como contorno, imagem de referência e sketch 3D com restrições (RF-SKT-30); BM-SKT-07/08 | P3 | G | SK-11 |

---

## 5. Fontes

Páginas lidas por inteiro (WebFetch com conteúdo):
- CAD avançado C — Restrições de sketch: (fonte em procedencia.md)
- CAD avançado C — Glifos de DOF: (fonte em procedencia.md)
- CAD avançado C — Split/Trim/Extend: (fonte em procedencia.md)
- CAD avançado C — Dimensões automáticas: (fonte em procedencia.md)
- CAD avançado C — Splines: (fonte em procedencia.md)
- CAD avançado C — Geometria projetada: (fonte em procedencia.md)
- GoEngineer — Trim Entities: (fonte em procedencia.md)
- Hawk Ridge — Fully Define Sketch: (fonte em procedencia.md)
- CATI — Check Sketch / Repair Sketch: https://www.cati.com/blog/tools-to-check-and-repair-sketchs/
- CADimensions — guia de problemas de sketch: https://knowledge.cadimensions.com/knowledge/sketch-troubleshooting-guide
- Blog CAD avançado A — Smart Dimension em arcos e círculos: (fonte em procedencia.md)
- CAD avançado E — solução de problemas de geometria de sketch: (fonte em procedencia.md)
- CAD avançado E — bastidores dos sketches: (fonte em procedencia.md)
- IBM APAR HC54475 (roxo = sobre-restrito no CAD avançado B): https://www.ibm.com/support/pages/apar/HC54475
- Skill-Lync — CAD avançado B Sketcher avançado (fórmulas, projetar, isolar): https://skill-lync.com/blogs/technical-blogs/design-what-do-you-need-to-know-about-sketcher-at-an-advanced-level
- SlideShare — notas de CAD avançado B: (fonte em procedencia.md)
- Universidade City (Londres) — documentação CAD avançado B (posicionar furo, restrições na Part Design): (fonte em procedencia.md) e (fonte em procedencia.md)

Fatos tirados de resumos de busca (a página em si não abriu ou só devolveu o menu), tratados com menos confiança:
- CAD avançado A, estados do sketch (cores, barra): (fonte em procedencia.md)
- CAD avançado A, Repair Sketch: (fonte em procedencia.md)
- CAD avançado A, Offset Entities: (fonte em procedencia.md)
- CAD avançado A, Move/Copy/Rotate/Scale: (fonte em procedencia.md)
- CAD avançado A, Troubleshooting (SketchXpert): (fonte em procedencia.md)
- Javelin — erros de sketch e Fully Define Sketch: (fonte em procedencia.md) e (fonte em procedencia.md)
- CAD avançado B, Sketch Analysis (GrabCAD, Eng-Tips, IBM): (fonte em procedencia.md) · https://www.eng-tips.com/goto/post?id=9209455 · https://www.ibm.com/support/pages/apar/HC92235
- CAD avançado B, sketch posicionado: (fonte em procedencia.md) · (fonte em procedencia.md)
- CAD avançado B, Quick Trim e mão livre (CERN): https://cad-support.web.cern.ch/node/575
- CAD avançado B, Sketch Solving Status (CERN): https://cad-support.web.cern.ch/node/568
- CAD avançado B, projeção 3D (Scribd): https://www.scribd.com/presentation/315244102/Part
- CAD avançado B, trim/break/mirror (GrabCAD): (fonte em procedencia.md)
- CAD avançado C, Offset (Constrain Offset / Loop Select): (fonte em procedencia.md)
- CAD avançado D, Include 3D Geometry e Project: (fonte em procedencia.md) · (fonte em procedencia.md)
- Código e documentos do repositório: `crates/fenkra-core/src/sketch.rs`, `crates/fenkra-app/src/sketch_ui.rs`, `crates/fenkra-app/src/sketch_ext.rs`, `crates/fenkra-core/src/doc.rs`, `docs/contextos/sketcher.md`, `docs/benchmark/sketch.md`, `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`

---

## Passagem de bastão

- **Resultado:** relatório completo em Markdown com 35 fichas, 31 RF e 9 RNF, 18 itens de backlog e as fontes.
- **Evidência:** leitura integral de `sketch.rs` e dos trechos de `sketch_ui.rs`, `sketch_ext.rs`, `slots.rs` e `doc.rs` relevantes (tudo marcado [código]); páginas da ajuda do CAD avançado C e blogs de revendas lidos por inteiro. O Fenkra não foi executado.
- **Arquivos alterados:** nenhum. Não criei o arquivo de plano, porque o contrato pede que a resposta final seja o relatório.
- **Verificações:** nenhum teste do Fenkra foi rodado. Os seis erros prováveis (seções 2.17, 2.21, 2.26, 2.29 e itens de RF-SKT-04 a 08) vêm de leitura de código e precisam de um teste que os reproduza antes de virar correção.
- **Falhas e riscos:**
  - Dois níveis de confiança: o CAD avançado B tem menos [V] do que o CAD avançado C, porque o GrabCAD, o Technia e o Eng-Tips bloquearam o acesso (403/404) e a ajuda do CAD avançado A devolveu só o menu.
  - Os sentidos de "Rubber In" e "Rubber Out" no *Quick Trim* do CAD avançado B têm fontes conflitantes. Defina por teste no produto.
  - Os tamanhos P/M/G do backlog são estimativas.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As cinco maiores lacunas do Sketcher do Fenkra são estados e diagnóstico (sem redundante, cota de referência nem Analisar Sketch), inferência só H/V, relimitação só em linhas, projeção 3D que vira polilinha sem isolar e sem estado de vínculo, e ausência de origem e orientação do sketch.
