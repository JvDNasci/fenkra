# Benchmarking: forma livre, ergonomia, visualizador, render e realidade virtual/aumentada

**Escopo.** Itens avaliados: B152, B153, B154, B156, B157, B162, B139, B140, B146 e B124. Não alterei nenhum arquivo. A pesquisa na web foi feita em 01/11/2026.

**Convenções de evidência.**
- **[V]** = verificado em página lida nesta pesquisa.
- **[C]** = conhecimento prévio, não reverificado agora. Confirme antes de decidir.
- **[N]** = procurei e não encontrei.

Preços de lista são de revendas ou agregadores quando assim indicado. Muitos fornecedores não publicam preço.

**Lacunas.**
- A página do CAD avançado F Illustrate devolveu 403 e a do Cortona3D devolveu 404.
- Não achei preço público do CAD avançado B Human Builder, do Process Simulate Human nem do Imagine & Shape.
- Não achei o produto "Prespective" (provável grafia errada). Em seu lugar apareceram Simumatik/Virtualware (VIROO).
- Não achei avaliações de usuários do Composer em G2/Capterra.
- Os números de tamanho de mercado variam muito entre consultorias. Trato-os como ordem de grandeza.

---

## 0. Resumo executivo

1. **Ordem de entrega.** O que mais rende por token é a base de "apresentação":
   - cena e aparências como nós da árvore;
   - viewport PBR (B139);
   - pacote de visualização (novo);
   - HTML autocontido (B146);
   - instruções animadas (B124);
   - só então o app Visualizador (B156).

   Isso entrega um fluxo completo "modelo → passos → BOM por passo → arquivo para quem não tem o Fenkra" sem render fotorrealista nem VR.
2. **Render (B157).** Não construa path tracer próprio.
   - Fase 1: render em tempo real com acumulação progressiva (suaviza sombra, profundidade de campo e antisserrilhamento) sobre B139.
   - Fase 2 (opcional): Blender/Cycles como **processo externo**, alimentado por B140.
   - Cycles é Apache-2.0 [V] e poderia ser ligado como biblioteca. O custo de build e de peso, e o fato de o modo standalone ser "não pronto para produção" [V], desaconselham isso.
3. **Forma livre.** Tem dois produtos diferentes, e o backlog os mistura.
   - **Subdivisão (B153):** o risco está na conversão SubD → B-rep que ainda aceite filete, casca e booleana.
   - **Polos/NURBS (B154):** o risco está na UX de edição e no ajuste de continuidade.
   - Proponho tirar do B154 uma **análise de qualidade de superfície** (zebra, curvatura, porco-espinho, conector G0–G2). Ela é barata, é útil já para filetes e superfícies existentes, e deve vir primeiro.
4. **Ergonomia (B152).** Nicho real, mas o mercado é pequeno.
   - Diferencial possível: manequim + passo de montagem (B168) + relatório em português ligado à NR-17.
   - O item de 450 k deve ser quebrado em núcleo (manequim, postura, alcance, visão) e análises (RULA/REBA/NIOSH/OWAS e relatório).
5. **VR/AR (B162).**
   - Revisão imersiva por OpenXR em PC-VR (Quest por Link/Steam Link). O app Quest nativo fica para depois.
   - AR no celular não precisa de WebXR: `model-viewer` com GLB (Android) e USDZ (iOS) cobre o essencial [C].
   - O Unity Reflect foi descontinuado em dez/2024 [V], o que abre espaço para uma opção simples e aberta.
6. **Inconsistências no backlog a corrigir.**
   - O B162 chama o app de "Fenkra Visualizador (B157)". O Visualizador é o **B156**; o B157 é Render.
   - O B140 diz "sem renderizador próprio", enquanto o B157 propõe "motor próprio em wgpu ou Cycles". Precisa de decisão: o texto proposto abaixo resolve isso com fases.
   - O B146 depende de "BN21", ou seja, o B139.
   - A pesquisa anterior listava ergonomia e render como "não recomendados/não-objetivo". A decisão de aceitar os dois já foi tomada pelo usuário. Aqui só ajusto o escopo para o mínimo defensável.

---

## 1. B152 — Bancada Ergonomia: manequim editável

### 1.1 Quem oferece

| Produto | Módulo/comando | Licenciamento |
|---|---|---|
| CAD avançado B / plataforma PLM avançada | Human Builder (cria o manequim), Human Measurements Editor, Human Posture Analysis (limites de ângulo de junta), Human Activity Analysis (RULA, NIOSH 81/91, empurrar/puxar Snook & Ciriello, carregar, biomecânica) [V, artigos acadêmicos] | Preço não público [N]. Vendido como produtos/roles à parte. |
| DELMIA Human (fornecedor de CAD avançado) | Simulação de tarefas humanas, Human Task Simulation [C] | Preço não público [N] |
| fornecedor de CAD avançado Process Simulate Human | Manequins Jack/Jill, OWAS, RULA, NIOSH, força estática 3D, análise lombar, envelope de visão e de alcance, bases ANSUR, NHANES e Canadian Land Forces [V, ficha da revenda]. Add-on "Advanced Human" com modelo lombar detalhado [V]. | Preço não público [N]. Uma revenda afirma que o Jack foi descontinuado e substituído por este [V, fonte única, não confirmada pela fornecedor de CAD avançado]. |
| IPS IMMA (Fraunhofer-Chalmers) | Manequim com 162 graus de liberdade [V]. Planejamento automático de trajetória do manequim e da peça (técnica de robótica), famílias de manequins, 56 variáveis anatômicas [V]. Caso de montagem de caminhão na Scania [V]. | Produto comercial pela IPS AB. Preço não público [N]. |
| CAD avançado D, CAD avançado A, CAD avançado C, CAD avançado E, Shapr3D | Nenhum manequim nativo [N]. Em CAD avançado A há "Human" apenas como figura simples [C]. | — |
| Abertos | Não achei biblioteca aberta de RULA/REBA/NIOSH [V, negativo]. Há planilhas (NC State, USF), mas as da USF são protegidas por copyright [V]. MakeHuman: código AGPL, ativos e alvos CC0, saída sem restrição [V]. OpenSim (musculoesquelético, Apache-2.0 [C]) é pesado demais para o MVP. | — |

### 1.2 Como funciona no melhor caso (CAD avançado B/DELMIA e Process Simulate)

**Entradas.**
- Base antropométrica e percentil (P5/P50/P95, masculino/feminino), ou medidas digitadas por segmento.
- Posição inicial na montagem.
- Alvos: mão, pé ou olhar apontando para pontos da montagem.

**Fluxo típico** [V, estudos aplicados em CAD avançado B]:
1. Cria-se o manequim com o Human Builder e escolhe-se a população.
2. Posiciona-se no posto de trabalho (sentado, em pé).
3. Edita-se a postura por juntas ou por cinemática inversa (a mão "pega" um ponto e o corpo acompanha, respeitando limites).
4. A análise de postura (Human Posture Analysis) colore cada junta conforme a zona de conforto.
5. A análise de atividade (Human Activity Analysis) roda RULA sobre a postura, NIOSH sobre o levantamento, e empurrar/puxar.
6. Registra-se a pontuação, altera-se o projeto e reavalia-se. Um estudo relata redução de RULA 6 para 3 após o reprojeto [V].

**O que é inteligente/automático.**
- Famílias de percentis rodando a mesma tarefa.
- Janela de visão do manequim: vista pelos olhos, cone de visão, zonas de alcance.
- No IMMA, o manequim encontra sozinho uma postura e um caminho sem colisão para a peça, minimizando carga biomecânica [V].

**Saídas.** Pontuações, gráficos, relatório, vídeo da simulação (nas versões de tarefas).

### 1.3 O que usuários elogiam e reclamam

Não achei fóruns de usuários sobre isso [N]. Só literatura acadêmica. O que ela sugere:
- O valor está em avaliar o projeto com postura realista antes de existir protótipo.
- O custo e a curva de aprendizado do pacote fornecedor de CAD avançado/fornecedor de CAD avançado são a barreira, e quase nenhuma PME os compra.
- A base ANSUR II representa militares dos EUA e a própria documentação alerta que "pode não ser representativa de outras populações" [V]. Isso importa para o Brasil.

### 1.4 Bibliotecas e padrões

- **ANSUR II.** 93 medidas diretas, 4.082 homens e 1.986 mulheres, CSV público desde 2017. Os escaneamentos 3D não são públicos [V]. A página oficial **não declara licença**; é obra do governo dos EUA (domínio público por regra geral [C]). Cite a fonte e não redistribua os escaneamentos.
- **Métodos.**
  - RULA (McAtamney & Corlett, 1993), REBA (Hignett & McAtamney, 2000), OWAS.
  - Equação de levantamento do NIOSH (revisada, 1991).
  - ISO 11226 (posturas estáticas), ISO 11228 (movimentação manual), EN 1005 [todos C].
  - Os métodos são públicos e devem ser implementados do zero, com testes contra exemplos resolvidos dos artigos. Não copiar planilhas protegidas.
- **MakeHuman.** Pode fornecer malha opcional CC0. O código é AGPL, então só como processo externo ou fora do pacote [V].
- **Cinemática inversa.** FABRIK/CCD em Rust puro (sem dependência). `nalgebra` (Apache-2.0 [C]).
- **Normas brasileiras.** A nova NR-17 (Portaria MTP 423/2021) está em vigor desde 03/01/2022. A AET é exigida quando a avaliação preliminar a indica; ME/EPP de risco 1–2 e MEI são dispensados da AET [V].

### 1.5 Recomendação para o Fenkra

**Fluxo na filosofia CAD avançado B.**
- O manequim é um **componente da montagem** com parâmetros publicados: percentil, população, estatura e demais medidas.
- A postura é um **estado** (conjunto de ângulos de junta) salvo na árvore, e várias posturas por manequim podem coexistir.
- Alvos de mão e olhar são **referências estáveis** a pontos, arestas ou conectores de componentes. Se o ponto muda de lugar, o manequim o acompanha.
- Painéis em sequência: **suporte** (qual manequim e qual posto) → **modo** (editar postura, alcance, visão, avaliar) → **parâmetros** (percentil, população, alvo) → **avançado** (limites articulares, base ANSUR, sobrescritas).

**MVP (núcleo, ~300 k).**
- Esqueleto de ~20 segmentos e ~40 graus de liberdade.
- Segmentos como cápsulas/elipsoides, sem pele.
- Tabela de percentis P5/P50/P95 por sexo a partir do ANSUR II, mais entrada manual e importação de CSV de qualquer base. Isso é essencial: a base é americana.
- Limites articulares com cores de conforto.
- Posturas prontas: sentado, em pé, agachado, alcançando.
- "Pegar/apontar" por IK.
- Envelope de alcance: amostragem das cadeias cinemáticas dentro dos limites, nuvem de pontos e malha por voxel, com zonas coloridas.
- Cone e campo de visão: uma segunda janela com a vista pelos olhos, no estilo da janela de visão do CAD avançado B.
- Folga de acesso para mão/ferramenta: volume varrido, interferência pelo kernel de colisão do Fenkra.

**Análises (item novo, ~200 k).** RULA, REBA, OWAS, NIOSH (índice de levantamento), tabela por postura, relatório em português com campos para NR-17, exportando PDF/CSV.

**Fica para depois.**
- Pele/malha realista.
- Planejamento automático de trajetória no estilo IMMA.
- Força estática e modelo lombar.
- Dinâmica e músculos.
- Mão com dedos.
- Vídeo da tarefa.

**Integrações.**
- **B168:** o manequim executa cada passo de montagem e gera postura, alcance, visão e esforço por passo.
- **B124:** anexa o resultado ergonômico ao passo.
- **B156:** exibe o manequim em modo leitura.
- **B162:** o manequim em escala 1:1 em VR.
- **B139:** só desenha as cores de conforto.

**Riscos técnicos.**
1. Dados de articulação (limites de movimento por junta) vêm da literatura; documentar a fonte de cada valor.
2. IK com limites e balanço: começar sem equilíbrio dinâmico.
3. O envelope de alcance por amostragem precisa de malha estável para exportar.
4. Responsabilidade: o relatório é apoio, não substitui o ergonomista. Dizer isso no relatório.
5. Viés populacional do ANSUR II.

**Testes de aceite.**
- Percentil P50 masculino do ANSUR II reproduz estatura e alcance tabelados dentro de 1 mm.
- RULA/REBA/NIOSH dão exatamente os valores de exemplos resolvidos (≥10 casos por método).
- Todos os limites articulares são respeitados em 1.000 posturas aleatórias.
- O alvo de IK é atingido em ≤1 mm quando alcançável e sinalizado quando não.
- O envelope é determinístico com semente fixa.
- Visual: cores de conforto e janela de visão.

**Tamanho.** B152 de 450 k → **~300 k** (núcleo) + **~200 k** (análises, item novo). Os dois somam acima do original porque o relatório e os testes contra exemplos resolvidos pesam.

---

## 2. B153 — Forma livre por subdivisão (estilo Fusion Form)

### 2.1 Quem oferece

| Produto | Módulo | Licenciamento |
|---|---|---|
| fornecedor de CAD avançado CAD avançado D | Ambiente **Form** (T-Splines) | US$ 680/ano comercial (várias fontes; a página multi-seat da fornecedor de CAD avançado também mostra US$ 680) [V]. Existem também promoções e licença pessoal gratuita limitada [C]. |
| CAD avançado B | Imagine & Shape (IMA), role plataforma PLM avançada [V] | Preço não público [N]. |
| Plasticity | Modelagem por NURBS e **PolySplines** (subdivisão → NURBS), xNURBS, Square | Perpétua, nó único. **Indie US$ 175**, **Studio US$ 299**, ambas com 12 meses de atualização e Blender Bridge. Studio inclui PolySplines, xNURBS, Square, Constraint Modeling [V]. |
| CAD avançado I | SubD + `ToNURBS` | Pago, perpétua [C] |
| Blender | Subdivisão (Catmull-Clark) | Aberto (GPL) [C]. Não é CAD. |
| Shapr3D | Não achei subdivisão [N] | Pro US$ 299/ano por editor, na página oficial [V]. |

### 2.2 Como funciona no melhor caso (Fusion Form)

- A forma é uma **feature da linha do tempo**: cria-se uma Form vazia, edita-se o corpo T-Spline, vinca-se arestas e **finaliza-se**, o que converte para B-rep/NURBS [V, exemplo da API fornecedor de CAD avançado].
- Primitivas: caixa, plano, cilindro, esfera, toro, quadball, tubo, com simetria já na criação [V].
- Simetria: espelho interno, circular, espelho-duplicado, limpar e isolar [V].
- Ferramentas de edição (lista oficial) [V]:
  - Edit Form: mover/girar/escalar faces, arestas, vértices; Alt+arrastar insere geometria.
  - Edit by Curve, Insert Edge (modo Exact preserva a forma), Subdivide, Insert Point.
  - Bridge, Fill Hole, Erase and Fill.
  - Merge Edge, Weld e UnWeld.
  - Crease/UnCrease (para na vizinhança de pontos estrela).
  - Bevel Edge, Slide Edge, Smooth, Cylindrify.
  - Pull (cola vértices numa superfície), Flatten, Straighten.
  - **Match**: alinha arestas do T-Spline a curvas ou arestas de sólidos, com **associatividade**.
  - Interpolate, Thicken, **Freeze/Unfreeze**.

### 2.3 O que usuários elogiam e reclamam

- Elogio recorrente: esculpir formas orgânicas que depois viram corpo sólido, na mesma linha do tempo. Recomenda-se modelar metade e espelhar, e isolar malhas complexas [V, handout da fornecedor de CAD avançado University].
- Reclamação central: **a conversão falha** em malhas complexas, com estrela, T-junction e vinco; as mensagens de erro são vagas para iniciantes [V, fórum fornecedor de CAD avançado].
- Um usuário cita limite de ~50.000 polígonos [V, afirmação de fórum não verificada].
- Em CAD avançado I, conversão SubD → NURBS de malha densa é lenta e pode travar. O PolySpline do Plasticity foi relatado como bem mais rápido [V, fóruns McNeel].
- **Diferencial do líder.** O Form vive dentro do paramétrico e **associativo** (Match, Freeze). O Plasticity ganha em velocidade e em G2 declarado, ao custo de ficar fora de uma árvore paramétrica de engenharia.

### 2.4 Bibliotecas e padrões

- **OpenSubdiv** (Pixar): Apache-2.0 com cláusula extra sobre marcas dos colaboradores [V]. Avalia a superfície-limite de Catmull-Clark em CPU e GPU [V]. **Não achei bindings em Rust** [V, negativo]: exigiria FFI C++ (`cxx`).
- **Alternativa recomendada para o MVP:** Catmull-Clark próprio em Rust (malha half-edge). No caso regular, a superfície Catmull-Clark é exatamente uma B-spline bicúbica uniforme [C], o que dá conversão exata nas regiões regulares.
- **Conversão em pontos extraordinários.** A referência é a opção do CAD avançado I `ToNURBS`: G0, G1, G1x, G1xx, G2 nos vértices extraordinários, com o aviso de que "só podem ser aproximados" e de que forçar G2 gera calombos ou depressões [V]. Cada opção troca desvio por continuidade [V].
- **Patente de T-splines.** A patente US 7,274,364 (BYU/fornecedor de CAD avançado) consta como **expirada em 2024** (Wikipedia/Google Patents; o próprio Google avisa que o status é suposição, não conclusão jurídica) [V]. Patentes posteriores da fornecedor de CAD avançado (ex.: US 8,310,481; US 10,832,471) existem [V] e **não** verifiquei se estão em vigor. Antes de implementar T-splines propriamente ditos, consultar parecer jurídico. **Subdivisão Catmull-Clark** é técnica pública antiga [C] e dispensa isso.
- **OCCT.** `Geom_BSplineSurface`, `BRepBuilderAPI_Sewing` e `ShapeFix` para costurar os remendos [C]. `truck-geometry` (Apache-2.0 na versão 0.4.0 [V]) como apoio matemático.

### 2.5 Recomendação para o Fenkra

**Fluxo na árvore.**
- Nó **"Forma"** (feature, não corpo): contém a malha de controle como parâmetros (vértices, vincos, simetria).
- Saída: um corpo B-rep que **entra como corpo da árvore**, aceita filete, casca e booleana, e se recalcula quando a malha muda.
- Referências estáveis: faces da saída são nomeadas pela **face da malha de origem** (índice persistente), para que um filete aplicado continue válido ao editar a forma. Se a topologia da malha muda (inserir aresta), as faces afetadas ficam "órfãs" e o painel pede reatribuição, como o Fusion faz com "Match".
- Painéis: **suporte** (primitiva ou corpo existente) → **modo** (face/aresta/vértice) → **parâmetros** (suavidade, vinco, simetria) → **avançado** (tolerância de conversão, opção G0/G1/G2 nos pontos extraordinários, como no CAD avançado I).

**MVP (~300 k).**
- Malha half-edge.
- Primitivas caixa, esfera, cilindro, plano.
- Editar vértice/aresta/face com manipulador.
- Extrudar e inserir aresta.
- Vinco por aresta.
- Simetria por plano.
- Visualização suave (Catmull-Clark na GPU ou CPU).
- Congelar (Freeze).

**Conversão SubD → B-rep (item novo, ~250 k).**
- Regiões regulares → remendos bicúbicos exatos.
- Pontos extraordinários → subdividir e aproximar com G1 e relatório de desvio máximo.
- Costura com tolerância → corpo fechado (sólido) ou aberto.
- Diagnóstico claro quando falha (onde e por quê), que é a maior queixa do Form [V].

**Fica para depois.** Match associativo entre aresta de forma e sólido, Bridge entre corpos, Pull/Flatten/Straighten, Thicken nativo (a casca do Fenkra já faz), importação de malha para forma, escultura por pincel.

**Integrações.**
- **B154:** análises de curvatura e zebra se aplicam à saída.
- **B158:** malha → superfície NURBS reaproveita a conversão.
- **B139:** pré-visualização.
- **B124/B157:** nenhuma dependência direta.

**Riscos técnicos.**
1. Filete e casca em remendos com junção só G1 aproximada podem falhar; limitar a tolerância e testar com a suíte de filetes.
2. Estabilidade de nomes de face ao editar topologia.
3. Desempenho em malhas densas (limite sugerido e aviso).
4. Quem espera "T-spline puro" (poucos remendos, sem remessagem) terá mais remendos do que no Fusion.

**Testes de aceite.**
- Caixa subdividida de 2 níveis gera B-rep válido (`BRepCheck_Analyzer`) com desvio ≤ tolerância configurada.
- Esfera quadball: desvio máximo ≤ 0,01 mm em escala de 100 mm.
- Filete de raio 2 mm aplicado numa aresta da saída tem sucesso em ≥ 95% de um conjunto de 50 formas.
- Editar um vértice recalcula e mantém o filete.
- Zebra visual sem quebras em regiões regulares.
- Casos patológicos (valência 3, 5, 8) geram G1 dentro da tolerância declarada.

**Tamanho.** B153 de 450 k → **~300 k** + **~250 k** (conversão, item novo).

---

## 3. B154 — Curvas e superfícies por polos (estilo FreeStyle)

### 3.1 Quem oferece

| Produto | Módulo | Licenciamento |
|---|---|---|
| CAD avançado B | **FreeStyle Shaper 2 (FSS)**, **Optimizer (FSO)**, **Profiler (FSP)** | Preço não público [N] |
| CAD avançado B plataforma PLM avançada | Roles de estilo (Natural Shape, etc.) [V] | Preço não público [N] |
| fornecedor de CAD avançado Alias | AutoStudio, Surface, Concept | Só assinatura; tabelas de revenda: AutoStudio 3 anos US$ 65.437,99 (listagem governamental), Concept renovação anual US$ 5.070 [V]. |
| Plasticity | Curvas e superfícies NURBS com medição de continuidade e curvatura | US$ 175/299 [V] |
| CAD avançado I, ICEM Surf | Superfícies por polos e Classe A [C] | Pago [C] |
| CAD avançado A | Freeform, Surface [C] | Pago [C] |

### 3.2 Como funciona no melhor caso (FreeStyle Shaper 2)

- **Criação:** curvas livres 3D, mesclar e extrudar curvas, preencher, mesclar ou casar superfícies, a partir do zero ou de dados digitalizados [V].
- **Edição:** deformar manipulando pontos de controle e bordas aparadas, com leis de deformação para refinar; controle de continuidade **até G2**; a modificação **se propaga a toda a topologia relacionada** [V].
- **Representação:** unificada e "sem matemática", preservando NURBS e Bézier por baixo [V].
- **Diagnóstico em tempo real** [V]: mapeamento de ambiente, isofotas por pixel, **porco-espinho** de curvatura, plano de corte dinâmico, **verificador de conexão**, analisador de distância, mapas de inclinação (draft) e de curvatura.
- **Match Surface:** restaura a continuidade exigida depois que superfícies vizinhas foram modificadas [V].
- Integra com Part Design e Assembly Design no modo híbrido [V].
- O Optimizer estende o Shaper com **morphing** de formas de várias superfícies [V].

### 3.3 O que usuários elogiam e reclamam

- Elogio: o diagnóstico ao vivo e a propagação de modificações sem refazer tudo.
- Reclamação: curva de aprendizado e custo; o Classe A real exige especialistas e ferramentas dedicadas (Alias/ICEM) [C].
- Na comparação de CAD avançado I SubD e T-splines, usuários citam que NURBS gerado "difere visivelmente" do T-spline [V, fórum McNeel] — aviso para qualquer fluxo de conversão.

### 3.4 Bibliotecas e padrões

- **OCCT** [C]:
  - `GeomLProp_SLProps` (curvaturas principais, Gaussiana, média) e `GeomLProp_CLProps` (curva).
  - `GeomFill_*` (varredura, preenchimento restrito com G1) e `GeomPlate_*` (placa com restrições até G2).
  - `GeomAPI_PointsToBSplineSurface` (ajuste) e `GeomConvert` (reparametrização).
  - `GeomAbs_Shape` já tem G0, G1, G2; **G3 não é nativo** nas rotinas de preenchimento.
- **Rust.** `nalgebra` (Apache-2.0 [C]) para o sistema linear das restrições; `truck-geometry` (Apache-2.0 [V]) para B-splines; `curvo` oferece interpolação, loft, varredura e interseção de curvas [V], licença não confirmada [N] (olhar o repositório antes de adotar).
- **Patentes.** A fornecedor de CAD avançado tem patente de superfícies paramétricas com continuidade Gᵢ requerida que trata vértices abertos [V]. Não é impedimento por si, mas um aviso para não copiar algoritmo descrito: implemente com álgebra linear padrão (mínimos quadrados restritos).

### 3.5 Recomendação para o Fenkra

**Fluxo na árvore.**
- Features **"Curva livre"** e **"Superfície livre"** com polos como parâmetros editáveis (grau, número de polos, nós).
- **Match** é uma feature que referencia **duas arestas**: fonte e alvo, com continuidade G0/G1/G2 e opção "mudar só a segunda".
- A simetria é um plano de referência.
- Referências estáveis: bordas e vértices da superfície livre recebem nomes pelos índices de polo/limite, não por posição.
- Publicar parâmetros: grau, nº de polos e continuidade.
- Painéis: **suporte** (curvas ou arestas de partida) → **modo** (polos / ajuste / casar) → **parâmetros** (grau, continuidade, tolerância) → **avançado** (leis de deformação, pesos).

**MVP (~300 k, depois da análise).**
- Curva e superfície B-spline/NURBS por polos, com grau e número de polos.
- Mover polos com manipulador.
- Inserir/remover nó.
- Simetria.
- **Match de borda G0–G2** por mínimos quadrados restritos.
- Conector de continuidade: lacuna, ângulo e diferença de curvatura ao longo da aresta.

**Análise de qualidade de superfície (item novo, ~150 k).** Funciona em **qualquer** face ou curva, inclusive filetes e importados:
- zebra/reflexo e isofotas por sombreador (basta normal e direção de visão);
- mapa de curvatura (Gaussiana, média, mínima);
- porco-espinho de curvatura em curvas e em seções;
- plano de corte dinâmico;
- mapa de inclinação (draft).

**Fica para depois.** G3, morphing de várias superfícies (Optimizer), perfil varrido por múltiplas guias (Profiler), reflexo de ambiente HDR customizado, fluxo Classe A com normas de OEM.

**Integrações.**
- **B153:** analisa a saída.
- **B158:** ajuste de NURBS a malha reaproveita o ajuste.
- **B139:** a zebra é uma variante do sombreador.
- **PMI/Desenho:** o relatório de continuidade pode ir ao PMI 3D.

**Riscos técnicos.**
1. UX de manipular centenas de polos em 3D; começar com superfície de poucos polos e seleção por linha/coluna.
2. Match G2 em superfícies de baixo grau pode ser inviável: o conjunto de restrições é sobredeterminado; avisar e sugerir aumentar o grau.
3. Persistência de nomes quando se muda o número de polos.
4. O Classe A é referência de qualidade, não promessa de produto.

**Testes de aceite.**
- Duas superfícies casadas G1: ângulo máximo entre normais ao longo da aresta ≤ 0,01°.
- G2: diferença de curvatura relativa ≤ 1%.
- Curvatura numérica do OCCT bate com a analítica de um círculo/esfera em ≤ 1e-6.
- O porco-espinho de uma circunferência tem comprimento constante.
- Zebra de uma esfera sai com faixas paralelas (comparação visual contra imagem de referência).

**Tamanho.** B154 de 400 k → **~300 k** + **~150 k** (análise, item novo).

---

## 4. B156 — Fenkra Visualizador (app de roteiros, animações e visualização)

### 4.1 Quem oferece

| Produto | O que entrega | Licenciamento |
|---|---|---|
| CAD avançado B Composer | Ilustrações técnicas associativas 2D/3D, animações, procedimentos interativos. **Composer Player** gratuito, **Player Pro** com API em tempo real, **Sync** e **Enterprise Sync** (XML, CLI) [V] | Pago, preço não público [N] |
| CAD avançado A Composer | Mesmo núcleo. Um ponto de preço: US$ 5.765 (revenda CATI, tipo de licença não declarado) e ¥ 807.000 (Hitachi) [V, pontos isolados] | Pago |
| CAD avançado F Illustrate / CAD avançado F View | Ilustrações interativas, PVZ para distribuição, integração PLM avançado [V] | Preço não publicado [V] |
| Cortona3D RapidAuthor/RapidManual | Manuais 3D interativos, S1000D, DITA, ATA2200 [V] | Modular, preço sob consulta; perfil aeroespacial/defesa [V] |
| CAD avançado A eDrawings | Visualizador gratuito com medição e marcação; recursos Professional acima disso [V] | Gratuito + licença |
| CAD avançado E | Visualizador nativo no navegador [C] | Plano |
| Plasticity Share | Pré-visualização de malha na nuvem sem expor o CAD [V] | Incluído |
| Shapr3D | Links seguros de revisão [V] | Incluído no Pro |

### 4.2 Como funciona no melhor caso

O Composer separa **autoria** (cara) de **consumo** (Player gratuito), e o Sync mantém a associatividade com o CAD e com o BOM. O CAD avançado F Illustrate separa **sequência** de **animação**: uma figura é uma ou outra, não as duas [V]. Chamadas de BOM podem ser arrastadas do eBOM para um sBOM de serviço [V].

### 4.3 O que usuários elogiam e reclamam

O valor reconhecido é reaproveitar o CAD para documentação sem redesenhar [V]. As queixas típicas são preço e o mercado concentrado em grandes empresas (Cortona3D tem 2026 descrito como pouco leve para o mercado médio [V]). Não achei avaliações do Composer em G2/Capterra [N].

### 4.4 Bibliotecas e padrões

- **glTF 2.0** (Khronos, formato aberto [C]) com extensões; compressão por meshopt ou Draco; texturas KTX2/Basis [V, listadas em fonte secundária].
- **three.js** (MIT [C]), **model-viewer** (Apache-2.0 [C]), **Babylon.js** (Apache-2.0 [C]).
- **wgpu** (MIT/Apache [C]) compila para nativo, WASM/WebGPU e WebGL2 [C]. Verificar suporte WebGPU nos navegadores-alvo antes de prometer [C].
- **Cuidado com OCCT em WASM.** Um mantenedor do OCCT respondeu que a LGPL não tem exceção para WASM (da pesquisa anterior); o visualizador **não** deve carregar kernel. Só malha.

### 4.5 Recomendação para o Fenkra

**Princípio.** Um **pacote de visualização** versionado (item novo) é o contrato. O B146 (HTML), o B156 (app), o B124 (passos) e o B162 (VR/AR) leem o mesmo formato.

**Conteúdo do pacote.**
- Malhas glTF (meshopt).
- Árvore da montagem com IDs estáveis do Fenkra em `extras`.
- Estados de exibição.
- PMI.
- Câmeras salvas.
- Passos de instrução (B124).
- Balões ligados ao BOM.
- Pontos de captura exatos (vértices, centros e raios de furo, pontos médios) para medição.
- Miniatura.

**Fluxo na árvore.** Nó **"Publicação"** com lista de estados e passos que entram no pacote. Nada é copiado: a publicação referencia a árvore, e "Atualizar" regenera.

**MVP (~300 k).**
- App desktop nativo só leitura, reaproveitando o núcleo de visualização do Fenkra.
- Abre o pacote.
- Árvore, visibilidade, corte, medição por pontos de captura.
- Reproduz passos do B124.
- Exporta imagens.

**Depois.**
- Versão WASM/WebGPU.
- App móvel.
- Exportar vídeo.
- Anotações do leitor.
- Sincronização com o Fenkra (Sync).

**Decisão importante.** Não construir app móvel nativo na primeira volta. O B146 já cobre celular pelo navegador. Estimativa: B156 de 450 k → **~300 k** só para o desktop; web e móvel nativos ficam fora.

**Integrações.** B146 (formato e JS), B124 (passos), B152 (manequim em modo leitura), B162 (VR/AR parte do mesmo pacote), B139 (sombreador), item novo de cena (câmeras/luzes).

**Riscos.**
1. Dois visualizadores (JS no B146 e Rust no B156) divergem. Mitigação: o pacote é o contrato e um teste de conformidade abre os mesmos 5 pacotes nos dois.
2. Proteção de propriedade intelectual: oferecer decimação e "sem arestas exatas".
3. Peso do pacote em montagens grandes.

**Testes de aceite.**
- Abrir 5 pacotes de referência (peça, montagem pequena, grande, com passos, com PMI) nos dois visualizadores com a mesma imagem final por captura (tolerância).
- Medição entre dois centros de furo exata a 1e-6 do valor do CAD.
- Reprodução do passo n mostra exatamente as peças do estado n.

---

## 5. B157 — Bancada Render: iluminação, ambientes, materiais e imagem final

### 5.1 Quem oferece

| Produto | Detalhe | Licenciamento |
|---|---|---|
| **renderizador avançado Studio** | Líder em render de produto. Planos Professional/Business/Enterprise com **todos os recursos**, diferindo só em suporte e gestão; **Studio VR** US$ 99/mês e **Studio Web** US$ 39/mês como complementos [V]. | **US$ 1.299/ano** (página oficial; G2 mostra US$ 1.188 em outra página) [V]. Perpétua descontinuada em 2023 segundo fonte de terceiros [V, não confirmado]. |
| **CAD avançado A Visualize** | Iray (NVIDIA), físico-realista. **Standard** vem com CAD avançado A Professional/Premium em assinatura ativa; **Professional** acrescenta animação, iluminação customizada, VR e 360 interativo; **Boost** distribui renders em outras máquinas (só Professional) [V]. | Standard avulso ~£170 + VAT; Professional ~£1.135/12 meses em uma revenda; sem lista pública [V]. |
| **Fusion (workspace Render)** | Render local ou na nuvem com créditos; turntable e re-renders só na nuvem (forum antigo) [V]. | Incluído na assinatura US$ 680/ano [V] |
| **CAD avançado E Render Studio** | No navegador. Incluído nos planos Professional e Enterprise, 2 MP; 100 MP custa **US$ 1.000/usuário/ano** [V]. Câmera, luzes, ambientes e conjuntos de aparência; Iray na nuvem (página japonesa) [V]. | Incluído / adicional |
| **Twinmotion** (Epic) | Tempo real. **Grátis** para empresas com receita < US$ 1 milhão/ano, estudantes e hobby; **US$ 445/assento/ano** acima disso [V]. A partir de 27/05/2026 as assinaturas Unreal não incluem mais Twinmotion [V]. | Veja ao lado |
| **Blender (Cycles)** | Cycles é **Apache-2.0** [V]. Blender em si é GPL [C]. | Gratuito |
| **FreeCAD Render** | Usa renderizadores externos (Cycles standalone, LuxCore, Pov-Ray etc.); mantenedor parou em nov/2025 (pesquisa anterior) [V]. | Gratuito |

### 5.2 Como funciona no melhor caso (renderizador avançado, Visualize)

- **Entradas.** Modelo importado, biblioteca de materiais, ambientes HDRI, câmera.
- **Fluxo.** Arrastar material sobre peça → escolher ambiente HDRI → ajustar câmera (distância focal, profundidade de campo) → render progressivo que **refina até o usuário parar**.
- **Saídas.** Imagem de alta resolução, turntable, animação, configurador, painel 360.
- **Automático.** Atualização ao vivo das aparências, denoise por IA, animação de produto com um clique [V, resumo de terceiros].

### 5.3 O que usuários elogiam e reclamam

- renderizador avançado: velocidade de render, interface, ampla biblioteca de materiais, bom controle de luz, curva de aprendizado curta [V, TrustRadius/Capterra].
- Reclamações: preço (caro, mas "vale a pena"), instalação de licença flutuante complicada, poucos materiais e erros em algumas avaliações isoladas [V].
- Fusion: créditos de nuvem, limites de resolução em render local [V, fóruns antigos].
- CAD avançado E: limite de 2 MP e preço do pacote de 100 MP.

### 5.4 Bibliotecas e padrões

- **Cycles** (Apache-2.0 [V]).
  - O modo standalone é "trabalho em andamento, não pronto para produção"; a doc recomenda a **API C++** para integração [V].
  - O XML do standalone é prova de conceito e não expõe todas as configurações [V].
  - Faltam 4 nós (RGB Curves, Vector Curves, RGB Ramp, Convert RGB→BW) [V].
- **Embree** (Apache-2.0, pesquisa anterior [V]), **OIDN** (Apache-2.0 [C]).
- **Compatibilidade de licença.** Apache-2.0 é incompatível com GPLv2-only, mas o Fenkra é "LGPL-2.1-**or-later**" e pode ser usado sob LGPLv3, que é compatível com Apache-2.0 [C]. Ainda assim, **processo externo** é o caminho mais limpo para o Blender (GPL).
- **Materiais em glTF:** `KHR_materials_transmission`, `ior`, `clearcoat`, `volume`, `emissive_strength` etc. [C; a pesquisa confirmou apenas que o suporte varia entre bibliotecas e que `pbrSpecularGlossiness` está arquivado [V]].
- **Ativos.** Usar HDRIs e texturas CC0 (por ex. Poly Haven [C]); empacotar um conjunto pequeno.
- **Motores Rust em wgpu** (renderling, scena, Penumbra) aparecem com PBR, IBL, SSAO e sombras [V], mas não vi licença nem maturidade; tratá-los como **referência**, não como dependência.

### 5.5 Recomendação para o Fenkra

**Princípio.** Render fotorrealista não é a vantagem do Fenkra. A recomendação é render **bom o bastante** para proposta comercial e catálogo, com um caminho de saída para quem quer mais.

**Fluxo na árvore.**
- Nó **"Cena"** (item novo) contém: câmeras salvas, luzes, ambiente (HDRI/degradê/cor/chão com sombra e reflexo), **aparências por componente ou face** (referência estável), decalques.
- É **publicável**: o B156/B146/B124 reaproveitam a mesma cena.
- Painéis: **suporte** (câmera e escopo) → **modo** (ambiente / luz / material / câmera) → **parâmetros** → **avançado** (amostras, profundidade de campo, denoise).

**Fase 1 (~300 k): render em tempo real com acumulação progressiva.**
- Sobre o B139: IBL + sombra suave + SSAO.
- Acumular quadros com sub-pixel jitter para antisserrilhamento, sombra suave por amostragem da luz e **profundidade de campo**, convergindo para uma imagem limpa **sem path tracing**.
- Chão de reflexo/sombra, fundo (cor, degradê, imagem), biblioteca PBR por aparência (metais, plásticos, vidro simplificado), câmeras salvas, resolução alta por ladrilhos (tiles), turntable.
- Decalques por projeção planar/cilíndrica numa face.

**Fase 2 (item novo, ~250 k): path tracing via Cycles como processo externo.**
- Gera glTF/USD + script Python (B140), localiza o Blender/Cycles instalado, executa em segundo plano, devolve progresso e imagem.
- Se não houver Blender, a fase 1 continua funcionando.

**Fica fora.** Path tracer próprio em wgpu (alto risco, ~400 k+, qualidade inferior ao Iray/Cycles), nuvem de render, configurador de produto, animação de câmera complexa.

**Integrações.** B139 (base), B140 (saída para Cycles), B124 (imagens de passo usam a mesma câmera/cena), B156/B146 (a cena viaja no pacote), B92 (materiais/aparências).

**Riscos.**
1. Escopo: o "não-objetivo" de render foi aceito explicitamente pelo usuário, mas controlar o tamanho.
2. Qualidade de vidro e transmissão em raster.
3. Peso da biblioteca de materiais e HDRIs no instalador.
4. Consistência de cor entre viewport e imagem final.

**Testes de aceite.**
- Imagem de referência (golden) para 5 cenas com tolerância por PSNR/SSIM, usando adaptador wgpu por software.
- A acumulação converge: diferença entre 64 e 256 amostras cai abaixo de um limiar.
- A mesma cena renderizada duas vezes com a mesma semente é idêntica.
- Turntable de 36 quadros sem vazamento de memória.
- Visual: material metálico, plástico e vidro reconhecíveis.

**Tamanho.** B157 de 450 k → **~300 k** (fase 1) + **~250 k** (Cycles, item novo) + **~150 k** (Cena, item novo).

---

## 6. B139 — Viewport com sombra suave, oclusão e PBR

### 6.1 Quem oferece

Quase todo CAD tem um modo "realista" em tempo real [C]:
- CAD avançado A: RealView e visual avançado.
- Fusion: sombreamento realista com ambiente.
- CAD avançado E: render no navegador, e Render Studio à parte [V].
- Shapr3D: renderizações em tempo real com 100+ materiais no plano Pro [V].

O diferencial hoje é **PBR correto** (cor base, metálico, rugosidade) ligado ao material do CAD, de modo que o viewport e o glTF exportado coincidem.

### 6.2 Como funciona

- Modelo físico GGX metallic-roughness, iluminação por imagem (IBL: mapa de irradiância e pré-filtrado) [C].
- Sombra por mapa com filtro (PCF/PCSS) e SSAO/GTAO em meia resolução.
- Entradas: material do B92 (densidade + aparência), ambiente.

### 6.3 Elogios e reclamações

Usuários valorizam que a peça "pareça o material" já no modelamento. Reclamam de queda de desempenho em montagens grandes [V, relatos gerais de desempenho em montagens grandes].

### 6.4 Bibliotecas

- wgpu e egui já usados no Fenkra [C].
- Crates Rust de glTF (`gltf`, MIT/Apache [C]) para exportar.
- **glTF-Validator** (Khronos, Apache-2.0 [C]) como teste.
- **Draco/meshopt** opcionais [V, extensões existem].

### 6.5 Recomendação

**Fluxo.** Nada na árvore. É configuração de **Visual** por documento e por vista, com níveis: Rápido (sem sombra/AO) / Padrão / Qualidade. O material do B92 carrega **aparência PBR** (cor base, metálico, rugosidade, opcional clearcoat), editável e publicável.

**MVP (~200 k).**
- PBR GGX com IBL embutido (HDRI pequeno).
- Sombra com PCF.
- GTAO em meia resolução.
- Realce de arestas existente preservado.
- Exportar glTF com os mesmos materiais.
- Seleção do nível de qualidade e FPS mínimo automático.

**Depois.** Reflexos de tela, sombra em cascata, transparência ordenada correta, instâncias com LOD.

**Integrações.** B157 (base do render), B140 (mesmo mapeamento de materiais), B146/B156 (mesmo sombreador), B154 (zebra), B152 (cores de conforto).

**Riscos.**
1. Transparência (vidro) em tempo real.
2. Desempenho com milhares de componentes: instâncias e culling.
3. Cor consistente entre tela e exportação.

**Aceite.**
- Golden images.
- Mesma cena exportada para glTF e aberta num visualizador padrão fica visualmente equivalente.
- glTF-Validator sem erros.
- FPS ≥ 30 em montagem de 5.000 componentes na máquina de referência.

---

## 7. B140 — Exportação de cena para Cycles/Blender

### 7.1 Quem oferece

- Plasticity: **Blender Bridge** incluído nas duas licenças [V].
- CAD avançado D e CAD avançado A têm pontes para renderizador avançado/Visualize [C].
- Blender importa glTF e USD nativamente [C].

### 7.2 Como funciona

Exportar a cena (geometria + materiais + câmeras + luzes + ambiente) para um formato aberto, abrir no Blender e renderizar com Cycles.

### 7.3 Pontos de atenção

- Unidades: mm no CAD, m em glTF (fator 0,001).
- Eixo: Z-up no CAD, Y-up em glTF.
- Normais suaves com arestas vivas: dividir vértices nas arestas do B-rep.
- Instâncias de componentes repetidos.
- Nomes estáveis: levar o ID do Fenkra em `extras`.

### 7.4 Bibliotecas

glTF; USD (Apache-2.0 modificada [C]), com escrita **USDA/USDZ** simples e própria possível [C]; `UsdPreviewSurface` e MaterialX.

### 7.5 Recomendação

**MVP (~150 k).**
- glTF/GLB com `KHR_materials_*` (ior, transmission, clearcoat, emissive_strength).
- Câmeras e luzes (extensão `KHR_lights_punctual` [C]).
- Script Python que monta a cena no Blender: ambiente, câmera, resolução, amostras. Em modo opcional, o próprio Fenkra chama o Blender (a fase 2 do B157).
- Nós do Fenkra preservados em `extras`.

**Depois.** USD completo, MaterialX, animações de montagem como keyframes.

**Prioridade.** P2 em vez de P3, porque a fase 2 do render depende dele. Se a fase 2 não for feita, P3.

**Aceite.** Round-trip: o GLB reimportado no Blender tem a mesma contagem de componentes, mesmas posições (≤ 1e-4 mm) e materiais equivalentes; script gera imagem sem erro.

---

## 8. B162 — Realidade virtual e aumentada

### 8.1 Quem oferece

| Produto | Detalhe | Licenciamento |
|---|---|---|
| **plataforma PLM avançada / CAD avançado B VR Reviewer** | Revisão 1:1 estereoscópica **sem exportar**, dentro das roles CAD avançado B; HMD e Immersive Desktop; sessões compartilhadas multiusuário [V]. | Parte das roles; preço não público [N] |
| **Mindesk (Vection)** | Link ao vivo bidirecional com CAD avançado I, Grasshopper, CAD avançado A e software BIM avançado; Suite 2024.0.1 (jul/2024) cita Quest 3 e Quest Pro [V]. Status da empresa incerto ("adquirida") [V]. | Assinatura (ex.: Suite Enterprise na NovEdge) [V, existência] |
| **Shapr3D** | Vision Pro com **todas as funções de modelagem**; visualizador AR no plano gratuito; AR integrado no iPad no Pro; **revisão imersiva Vision Pro só no Enterprise** [V]. | Pro US$ 299/editor/ano [V] |
| **CAD avançado A eDrawings** | AR citada como recurso da camada Professional em uma comparação; fontes conflitam [V]. | Gratuito/Professional |
| **Unity Reflect** | **Descontinuado**; indisponível desde dez/2024 [V]. | — |
| **Unity Pixyz Review, CAD Schroer i4** | Revisão VR com importação e compressão automática de STEP; mapa de visão geral para modelos grandes [V]. | Comerciais, preço não público [N] |
| **Simumatik/Virtualware (VIROO)** | Comissionamento virtual e VR industrial [V]. "Prespective" não encontrado [N]. | — |
| **Vection Technologies** | AR/VR industrial; comprou Mindesk em 2020 [V]. | — |
| **OpenXR / WebXR** | Padrões abertos (veja abaixo). | — |

### 8.2 Como funciona no melhor caso

No VR Reviewer do plataforma PLM avançada, o modelo aparece em escala 1:1 diretamente da plataforma, com navegação, corte, medição e reunião multiusuário [V]. O Mindesk permite **editar no CAD e ver no capacete ao mesmo tempo** (link bidirecional) [V].

### 8.3 O que usuários e estudos dizem

- A revisão em VR encontrou um pouco mais de falhas que a em tela, mas **complementa**, não substitui [V, estudo].
- Barreiras: o esforço de **preparar/converter** os dados para VR, e o desempenho com montagens grandes [V].
- Reclamação de movimento/enjoo e tesselação: não achei fontes específicas [N].

### 8.4 Bibliotecas e padrões

- **OpenXR** (Khronos): a API aberta para capacetes [C]. Crate Rust **`openxr`** 0.21.1, licença **MIT ou Apache-2.0** [V]. Existe `bevy_mod_openxr` como integração com Bevy [V, indireto].
- **Risco:** integrar OpenXR com **wgpu** exige interoperar com imagens de swapchain Vulkan/D3D12 via `wgpu-hal` [C]; é o ponto técnico mais delicado.
- **WebXR:**
  - O Safari no iPhone/iPad **não** implementa WebXR [V].
  - Chrome no Android oferece AR por ARCore [V].
  - No Vision Pro, o suporte a WebXR imersivo varia por versão [V, fontes discordam].
  - Logo, **o caminho prático no iOS é AR Quick Look com USDZ**, e no Android o Scene Viewer ou WebXR [V, secundário].
- **model-viewer** (Apache-2.0 [C]) escolhe o modo certo por plataforma.

### 8.5 Recomendação para o Fenkra

**Princípio.** Menos ambição que fornecedor de CAD avançado/Mindesk. Foco em **revisar o que já está publicado**, sem edição imersiva.

**Fluxo na árvore.** Nenhum nó novo. VR e AR leem a **Publicação** (pacote do B156) e o manequim do B152.

**MVP VR (~300 k).**
- Binário `fenkra-vr` separado (OpenXR), para **PC-VR** (Quest via Link/Air Link/Steam Link, SteamVR, Windows MR).
- Abre um pacote.
- Escala 1:1 ou reduzida.
- Teletransporte.
- Agarrar e escalar.
- Plano de corte e medição por raio do controle.
- Explodida por controle deslizante.
- Passos do B124.
- Manequim do B152 em escala real para conferir alcance e visão.

**MVP AR (item novo, ~150 k).**
- Exportar GLB e USDZ com escala real.
- Código QR e página HTML com `<model-viewer ar>`.
- Foco em peça única ou montagem pequena com limite de tamanho.

**Fica para depois.** App Quest nativo (Android + Vulkan), multiusuário, edição em VR, âncoras no local de instalação, hand tracking.

**Integrações.** B156/B146 (pacote), B124 (passos), B152 (manequim), B139 (sombreador), B168 (simulação).

**Riscos.**
1. Interop wgpu ↔ OpenXR.
2. Desempenho: 90 Hz exigem ~11 ms por quadro, com decimação e instâncias.
3. Testar VR sem hardware em CI: usar runtime simulado/headless [C].
4. Dependência de aparelhos e drivers.
5. Conforto (movimento): teletransporte por padrão.

**Testes de aceite.**
- Automático: matemática de teletransporte, escala e corte; carga do pacote.
- Visual/manual: checklist em ao menos um capacete.
- GLB/USDZ: validação com glTF-Validator e abertura em Quick Look e Scene Viewer.
- Quadro médio ≤ 11 ms em montagem de referência.

**Tamanho.** B162 de 450 k → **~300 k** (VR) + **~150 k** (AR, item novo).

**Correção necessária:** o texto atual cita "Fenkra Visualizador (B157)"; deve ser **B156**.

---

## 9. B146 — Visualizador HTML autocontido

### 9.1 Quem oferece

- CAD avançado E: web nativo [C].
- eDrawings: visualizador gratuito com medição e marcação [V].
- Composer: publica para HTML e Player [C].
- cadasio: ilustrações interativas (pesquisa anterior).
- Plasticity Share: pré-visualização de malha na nuvem [V].
- Shapr3D: links seguros de revisão [V].
- Sketchfab, CAD avançado F View [C].

### 9.2 Como funciona

Um único arquivo `.html` com JS inline e o modelo embutido (base64), abre offline.

**Funções mínimas:**
- órbita/zoom/pan com toque;
- árvore com visibilidade e isolar;
- seção por planos (`clippingPlanes` do three.js [C]);
- medição;
- explodida por controle;
- PMI e vistas salvas;
- passos do B124.

### 9.3 Bibliotecas

- **three.js** (MIT [C]) é a escolha para o MVP: ~700 KB minificado [C].
- **model-viewer** (Apache-2.0 [C]) se bastar um visualizador simples.
- **Babylon.js** (Apache-2.0 [C]).
- **meshopt** e **Draco**: decodificadores WASM pequenos [C].

### 9.4 Recomendação

**Medição sem kernel.** O arquivo carrega, no pacote, os **pontos de captura exatos** do B-rep (vértices, centros e raios de furo, pontos médios, eixos). A medição no navegador usa esses pontos e fica exata, enquanto a malha só serve para a imagem. Este é o diferencial em relação a viewers que medem na malha.

**MVP (~250 k).**
- Árvore.
- Seção.
- Medição por pontos de captura.
- Explodida.
- Vistas salvas.
- Passos do B124.
- Tema claro/escuro e toque.
- Opção de **decimação** para proteger propriedade intelectual.
- Limite e aviso de tamanho (alvo ≤ 10 MB).

**Depois.** Comentários locais, impressão de captura, modo AR (`model-viewer`), carregamento progressivo.

**Riscos.**
1. Tamanho do HTML em montagens grandes.
2. WebGL em celulares antigos.
3. Duplicidade com o B156 (resolvida pelo pacote).
4. Segurança: nenhum acesso de rede, arquivo estático.

**Aceite.**
- Abre offline em Chromium e Firefox sem requisições de rede.
- Medição de dois furos coincide com o CAD em 1e-6.
- Captura de tela por navegador headless bate com a referência.
- Tamanho final dentro do orçamento nos 5 pacotes de referência.

---

## 10. B124 — Instruções de montagem animadas

### 10.1 Quem oferece

| Produto | Detalhe | Licenciamento |
|---|---|---|
| CAD avançado B / CAD avançado A Composer | Instruções animadas, ilustrações associativas, Player gratuito, Sync [V] | Pago; ~US$ 5.765 em um ponto de revenda [V] |
| CAD avançado F Illustrate | Explodidas com linhas, **chamadas e lista de itens por figura** (renumerar/combinar), animação ou sequência, eBOM → sBOM por arrastar, PVZ [V] | Não publicado [V] |
| Cortona3D RapidManual/RapidAuthor | Procedimentos animados, S1000D, DITA, ATA2200 [V] | Modular, sob consulta [V] |
| IsoDraw, Fusion Animation | Ilustração vetorial, storyboard [C] | Pago [C] |
| Open source | Nenhum equivalente integrado ao CAD [N] | — |

### 10.2 Como funciona no melhor caso

- Passo = título + texto + peças mostradas/ocultas + movimentos (translação/rotação ao longo de trajetória) + câmera + chamadas/balões + ferramenta + aviso + duração.
- Saídas: vídeo, HTML interativo, PDF com figuras por passo e lista de peças, imagens [V].
- O Composer mantém associatividade por **Sync** (XML) quando o CAD muda [V].

### 10.3 O que usuários elogiam e reclamam

Valor: documentação sem redesenhar e que se atualiza com o projeto [V]. Queixas: preço e ferramental pesado; o perfil de uso é de grandes empresas [V].

### 10.4 Recomendação para o Fenkra

**Fluxo na árvore.**
- Nó **"Instruções"** → **"Roteiro"** → **"Passo n"**.
- Cada passo referencia **instâncias de componente** por ID estável, mais um **estado de exibição** (publicação). Se o componente some ou é substituído, o passo mostra um aviso, não quebra.
- O BOM por passo é **calculado** (peças que entram no passo, quantidades, fixadores agrupados) e nunca digitado.
- Painéis: **suporte** (montagem e roteiro) → **modo** (editar passo / explodir / câmera / texto) → **parâmetros** (distância, direção, duração) → **avançado** (trajetória livre, ferramentas, avisos).

**Parte inteligente (diferencial).**
- **Sequência automática.** Calcular a direção de remoção de cada peça a partir dos graus de liberdade livres das restrições do Fenkra, testar colisão ao longo dessa direção e gerar a ordem de desmontagem; a montagem é o inverso. Isso é o grafo de bloqueio clássico [C] e reaproveita as restrições existentes.
- **Explodida** ligada ao roteiro: cada passo reaproveita a distância e a direção da explodida.
- **Agrupamento automático** de fixadores iguais num passo.
- **Vistas vetoriais.** Reaproveitar a remoção de linhas ocultas do desenho técnico para gerar figuras isométricas **vetoriais** (SVG/PDF) por passo, com balões numerados. Isso é um diferencial frente a ferramentas que só geram imagem raster.

**MVP (~300 k, em duas etapas).**
- Etapa 1 (~200 k): passos manuais com explodida, BOM por passo, balões, saída HTML (usando o B146), PDF vetorial e PNG.
- Etapa 2 (~100 k): sequência automática, vídeo.

**Vídeo.** Gerar quadros (PNG) e, como opção, chamar o **ffmpeg externo** (o ffmpeg pode ser GPL conforme a compilação [C]; processo externo e fornecido pelo usuário).

**Depois.** S1000D/DITA, ferramentas e torques, traduções, sincronização automática com mudanças do CAD (Sync), animação de cabo flexível.

**Integrações.** B146 (HTML), B156 (app), B168 (sequência e ferramentas), B152 (postura por passo), B157 (cena e câmera), BN08 (citado como dependência), B92 (BOM).

**Riscos.**
1. Detecção de direção de remoção em montagens com peças encaixadas ou flexíveis.
2. Estabilidade dos passos quando a montagem muda.
3. Qualidade tipográfica do PDF vetorial.
4. Escopo de S1000D: não aceitar na primeira volta.

**Aceite.**
- A soma do BOM por passo é igual ao BOM total.
- Mover o componente do passo n reproduz exatamente a transformação armazenada.
- Regenerar após editar a montagem mantém os passos cuja referência existe e sinaliza os demais.
- A sequência automática em 3 montagens de referência é válida (sem colisão).
- Visual: PDF com balões legíveis.

---

## 11. Mercado

### 11.1 Dimensão (ordem de grandeza)

| Segmento | Faixa de 2025 | Qualidade |
|---|---|---|
| Software de renderização/visualização 3D | US$ 3,5–6,8 bi; CAGR 16–20% [V] | Escopos diferentes entre consultorias; inclui mídia e arquitetura |
| Software de ilustração técnica / documentação 3D interativa | US$ 0,8–3,7 bi; CAGR 3,5–8,5% [V] | Estimativas divergentes entre si |
| AR/VR na manufatura | US$ 5–16 bi; CAGR 19–23% [V] | Alguns relatórios com erro de unidade; ignorei |
| Modelagem humana digital (ergonomia de engenharia) | **Sem número confiável** [V, negativo] | Os relatórios "digital human" tratam de avatares e entretenimento |

Esses números são direcionais e vêm de relatórios pagos. **Não usá-los em materiais públicos** sem fonte primária.

### 11.2 Quem compra

| Perfil | O que compra | Observações |
|---|---|---|
| OEM e fornecedores automotivos e aeroespaciais | CAD avançado B/DELMIA Human, Process Simulate Human, IMMA, VR Reviewer | Já têm pacotes caros; o Fenkra não os substitui, mas pode atender fornecedores menores |
| Máquinas, implementos, carrocerias, caldeiraria e estruturas | Documentação de montagem/manutenção, imagens para proposta, revisão de projeto | Sensíveis a preço; usam CAD avançado A/CAD avançado C; Composer/renderizador avançado são caros para esse porte |
| Design de produto e impressão 3D | Forma livre, render, visualização | Plasticity e Fusion Form dominam o baixo custo |
| Aeronáutico e defesa | Publicações técnicas S1000D/ATA (Cortona3D) | Barreira alta de normas; não é alvo do MVP |
| Educação | Visualizadores gratuitos, manequim, forma livre | Shapr3D é grátis para estudantes [V]; Twinmotion grátis para estudantes [V]; o Fenkra aberto compete bem |
| Segurança do trabalho (Brasil) | Laudos de AET e avaliação preliminar da NR-17 [V] | Comprador é consultoria/SST, não o projetista; ME/EPP de baixo risco dispensados de AET [V] |

### 11.3 Onde o Fenkra pode se destacar

1. **Cadeia integrada numa árvore com referências estáveis:** modelo → sequência automática → conferência com manequim → instruções com BOM por passo → arquivo para quem não tem o Fenkra. Os concorrentes dependem de produtos separados e de "Sync".
2. **Figuras vetoriais de instrução** com balões e BOM por passo, a partir do desenho técnico: pouco comum fora do Illustrate/IsoDraw.
3. **Visualizador gratuito e offline**, com medição exata via pontos de captura. Contrasta com as soluções em nuvem (Plasticity Share, Shapr3D, CAD avançado E) e com o preço do Composer.
4. **Português e normas locais:** relatório ergonômico alinhado à NR-17, sem substituir o ergonomista.
5. **Licença aberta e preço zero** diante de US$ 1.299/ano (renderizador avançado), US$ 680/ano (CAD avançado D) ou US$ 299 (Plasticity Studio).
6. **Janela de oportunidade em VR:** o Unity Reflect foi descontinuado [V]; Mindesk tem futuro incerto [V]. Uma revisão VR simples e aberta tem espaço.

**Onde não competir (agora):** render fotorrealista (renderizador avançado, Visualize), Classe A real (Alias, ICEM, CAD avançado B), fisiologia humana completa (Jack, IMMA), publicações S1000D.

### 11.4 Sequência sugerida

1. Cena (novo) + B139.
2. Pacote de visualização (novo) + B146.
3. B124 (etapa 1), em paralelo com a análise de superfície (novo).
4. B152 núcleo → análises (novo) → B168.
5. B154 → B153 + conversão (novo).
6. B156 (desktop).
7. B157 fase 1 → B140 → Cycles (novo).
8. B162 VR → AR (novo).

---

## 12. Fontes (URLs)

**Forma livre**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://doc.plasticity.xyz/getting-started/license-types-and-features
- https://www.cgchannel.com/2025/12/plastic-software-releases-plasticity-2025-3/
- https://discourse.mcneel.com/t/subd-vs-t-splines-surface-conversion/117916
- (fonte em procedencia.md)
- https://www.shapr3d.com/pricing
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://graphics.pixar.com/opensubdiv/docs/intro.html
- https://patents.google.com/patent/US7274364
- https://en.wikipedia.org/wiki/T-spline
- https://patents.justia.com/patent/7400323
- https://docs.rs/curvo
- https://docs.rs/crate/truck-geometry/latest/

**Ergonomia**
- https://www.prevencionintegral.com/node/17227
- https://4dsysco.com/?p=1995
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://pmcorp.com/simulation/jack/
- https://fcc.chalmers.se/software/ips/ips-imma/
- https://research.chalmers.se/publication/511515
- https://research.chalmers.se/en/publication/171898
- https://www.openlab.psu.edu/ansur2/
- https://ph.health.mil/topics/workplacehealth/ergo/Pages/Anthropometric-Database.aspx
- https://ergocenter.ncsu.edu/resources-tools/analysis-tools/
- https://health.usf.edu/publichealth/tbernard/ergotools
- https://static.makehumancommunity.org/about/license.html
- https://www.gov.br/trabalho-e-emprego/pt-br/assuntos/inspecao-do-trabalho/seguranca-e-saude-no-trabalho/canpat-2/canpat-2021/nova-nr17.pdf
- https://www.rsdata.com.br/nr17-nova-redacao-como-sera/

**Visualizador e instruções**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.hitachi.co.jp/products/it/industry/solution/3dviacomposer/price.html
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://cortona3d.com/en/rapidauthor
- https://www.cortona3d.com/rapidmanual
- https://cortona3d.com/de/rapidauthor
- (fonte em procedencia.md)
- (fonte em procedencia.md)

**Render**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.twinmotion.com/en-US/license
- https://www.unrealengine.com/en-US/blog/we-are-updating-unreal-engine-twinmotion-and-realitycapture-pricing-in-late-april
- https://developer.blender.org/docs/features/cycles/standalone/
- https://wiki.blender.org/features/cycles/standalone/
- https://docs.rs/crate/renderling/latest

**VR/AR**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.vection-technologies.com/solutions/products/mindesk/overview
- https://mindeskvr.com
- https://vection-technologies.com/about-us/
- https://www.shapr3d.com/blog/introducing-shapr3d-for-apple-vision-pro
- https://support.shapr3d.com/hc/en-us/articles/14852772513180
- https://docs.unity.com/reflect/en/manual/Home
- https://unity.com/products/pixyz-review
- https://www.cad-schroer.com/?p=51597
- https://www.virtualwareco.com/?p=41544
- https://github.com/Ralith/openxrs
- https://docs.rs/crate/openxr/latest
- https://www.issoh.co.jp/tech/details/7076/
- https://abratabia.com/webxr-games/webxr-device-support.php
- https://www.cambridge.org/core/services/aop-cambridge-core/content/view/F906024788EB60F52D2D92A35E761968/S2220434219001987a.pdf/comparing-virtual-reality-and-desktop-interface-for-reviewing-3d-cad-models.pdf

**Mercado**
- https://www.grandviewresearch.com/horizon/statistics/3d-rendering-market/component/software/global
- https://imarcgroup.com/visualization-3d-rendering-software-market
- https://www.researchnester.com/reports/3d-rendering-market/6300
- https://www.verifiedmarketresearch.com/product/technical-illustration-software-market/
- https://markwideresearch.com/global-technical-illustration-software-market
- https://www.precedenceresearch.com/augmented-reality-and-virtual-reality-in-manufacturing-market
- https://www.wiseguyreports.com/reports/3d-human-model-market

**Documento interno**
- docs/pesquisa/modulos-novos.md (seções 2.21, 2.24 e 2.26)

---

STATUS: DONE_WITH_CONCERNS
SUMMARY: Relatório de benchmark dos 10 itens entregue, com 7 novos itens para quebrar B152, B153, B154, B156/B162 e B157, mas com lacunas de preço, fontes que não abriram (CAD avançado F Illustrate 403, Cortona3D 404), "Prespective" não localizado, tamanhos de mercado divergentes e licenças de `curvo`, OIDN e three.js só por conhecimento prévio.

**Handoff.**
- **Resultado:** o relatório acima.
- **Arquivos alterados:** nenhum. O modo de planejamento estava ativo e você pediu para não criar nem alterar arquivos, então não gravei arquivo de plano; ExitPlanMode não está disponível neste ambiente.
- **Verificações:** só leitura do backlog.md, do backlog.json (áreas válidas) e de modulos-novos.md.
- **Falhas:** os dois fetches citados acima falharam; a pesquisa anterior só confirma Composer e Illustrate por busca e página da 3DS.
- **Riscos:** os pontos marcados [C] não foram reverificados; os números de mercado são direcionais.
