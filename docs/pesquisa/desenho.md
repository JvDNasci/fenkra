# Desenho técnico (Drafting / Drawings)

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> **Como ler as marcas.** [V] = confirmado em página que li, ou em trecho de busca que citava a ajuda oficial. [C] = conhecimento geral, não confirmado nesta pesquisa.
>
> **Limite desta pesquisa.** As páginas `help.CAD avançado A.com` são montadas por JavaScript e o WebFetch devolveu só o menu. Todo [V] do CAD avançado A vem dos trechos que a busca extraiu da ajuda e de blogs de revendas, não da página lida por inteiro. CAD avançado B (tutorial da City University) e CAD avançado C (`help.fornecedor de CAD avançado.com`) li direto, página a página.
>
> **Sobre este texto.** Usei o modo de planejamento só para ler e pesquisar, sem alterar arquivos. O relatório inteiro está nesta resposta.

---

## 1. Resumo e principais lacunas

1. O Fenkra já tem o essencial de um 2D associativo ao 3D. Há vistas por projeção com linhas ocultas (HLR), corte total com hachura, detalhe circular, planificada de chapa, cotas, notas e símbolos (rugosidade, solda ISO 2553, GD&T, datum). Há também tabelas (BOM, furos, dobras, lista de corte), balões automáticos, carimbos ISO/ABNT/ANSI e exportação PDF/SVG/DXF.
2. A arquitetura é simples e honesta. O desenho guarda caminhos de arquivo e parâmetros, e `Models::proj` recalcula a projeção quando o arquivo muda no disco (`source.rs:209`).
3. **A associatividade é só de vista, não de cota.** Cada cota guarda pontos 3D soltos (`Dim.anchors`, `lib.rs:327`), obtidos por `unproject` do clique, com profundidade zero (`dwg_ui.rs:947`). Se a peça mudar de tamanho, a cota continua marcando o valor antigo, e nada avisa. O painel admite isso em `dwg_ui.rs:1753`, mas o catálogo marca DES-05 como "✅ completo".
4. Não existe estado de atualização (atualizada, desatualizada, erro, referência quebrada), nem botão Atualizar, nem trava de vista. A tela só recarrega sozinha a cada 1,5 s (`dwg_ui.rs:475`). CAD avançado B, CAD avançado C, CAD avançado H e CAD avançado E mostram o estado e deixam adiar ou travar.
5. **Só existe 1º diedro**, fixo em `Dir::axes` (`lib.rs:197`). A legenda sempre desenha "1º diedro" e "FIRST ANGLE PROJECTION" (`layout.rs:606–618`). Não há herança norma → folha → vista → elemento (DR-35..43). Só a vista passa estilo (`View.ann`) para suas cotas e notas.
6. Faltam muitos tipos de vista. O Fenkra tem Standard, Projected, Section (reta), Detail (círculo), Flat, Schematic e Camera. Faltam:
   - auxiliar;
   - meio corte, corte em desvio e corte alinhado;
   - corte parcial (breakout);
   - seção removida;
   - vista interrompida;
   - vista recortada;
   - posição alternativa;
   - explodida como vista própria.
7. Faltam comandos de cotagem: ordenada, linha de base e cadeia, chanfro, chamada de furo e rosca, importar ou gerar cotas do modelo, cota de referência, tolerância e ajuste estruturados.
8. A hachura é fixa: 45°, passo 2,5 mm, sem padrão por material (`layout.rs:342`). Não há alternância de ângulo entre peças vizinhas num corte de montagem.
9. Faltam tabelas de revisão, solda, punção, colunas configuráveis e tags de furo nas vistas. A tabela de furos usa XYZ do modelo, não a origem da vista.
10. A exportação DXF é R12 ASCII, com 5 camadas e tudo "explodido" (sem entidade de cota e sem hachura nativa). Não há DWG. Não existe MBD → desenho (vistas geradas de capturas PMI).

**Cinco maiores lacunas:** (1) cotas sem referência topológica nem diagnóstico; (2) sem estado de atualização, trava ou isolamento de vista; (3) sem 3º diedro nem herança de norma; (4) catálogo de vistas incompleto (auxiliar, cortes especiais, interrompida, breakout, recorte, explodida); (5) cotagem inteligente ausente (importar e gerar cotas, ordenada, chanfro, chamada de furo).

---

## 2. Fichas por comando

### 2.0 Como o Fenkra funciona hoje (resumo para ler as fichas)

- **Modelo de dados** (`lib.rs:512`). `Drawing` guarda `sheets`, `views`, `dims`, `notes`, `tables` e um único `title` (carimbo) para o desenho todo.
- **Vista** (`lib.rs:265`). Cada `View` guarda `source` (arquivo, estado de exibição e componentes escondidos), `kind`, `pos`, `scale`, `hidden_lines`, `tangent_edges`, `render`, cor, espessura e `ann` (estilo herdado).
- **Projeção** (`source.rs:237`). `compute` carrega a peça, monta a forma e chama `Shape::hlr(toward, x, hidden)`. As linhas saem tipadas: 0 visível, 1 tangente, 2 contorno, 3 oculta, 4 borda do detalhe, 5/6 dobra.
- **Folha** (`layout.rs:298`). `layout::sheet` transforma tudo em primitivas em mm (linhas, preenchimentos, textos). Tela, PDF, SVG e DXF usam as mesmas primitivas.
- **Interface** (`dwg_ui.rs`). É uma máquina de ferramentas por cliques: `command` (220) arma a ferramenta e `click` (856) cria os objetos. `canvas` (466) desenha e recarrega. `panel` (1375) edita propriedades.
- **Pontos fortes:**
  - Desfazer e refazer do desenho inteiro (`undo`/`redo`).
  - Campos automáticos `{codigo}`, `{massa}` e outros (`lib.rs:107`).
  - Notas presas à vista com `anchor3`, que seguem a peça.
  - Balões automáticos ligados à BOM e à lista de corte (B26).
  - Esquema cinemático e câmera do 3D como vista.
  - Estado de exibição e esconder componentes por vista.

---

### 2.1 Folha e formato
(CAD avançado B: Sheet, `File > Page Setup` / CAD avançado A: Sheet Properties, Sheet Format / CAD avançado C: Sheet, Edit Sheet / outros: CAD avançado E sheets / Fenkra: `Sheet`)

- **O que faz.** Define a folha: tamanho (A0–A4, ANSI A–E), orientação, escala padrão e zoneamento. Um desenho tem várias folhas.
- **CAD avançado B.**
  - [V] Ao criar o desenho escolhe-se a norma (ex.: ISO), o que define o formato (ISO dá A0, mas pode-se trocar), a orientação e a escala. O tamanho depende do tipo de norma.
  - [V] `File > Page Setup` muda norma, formato, orientação e escala depois. Trocar a norma faz o campo "Apply on" virar "All sheets" e a norma vale para todas as anotações das folhas.
  - [V] Folha nova herda norma, formato e orientação da primeira.
  - [V] Os nomes das folhas persistem após exclusão: sheet2 continua sheet2 mesmo sem sheet1.
  - [V] O método de projeção (1º ou 3º diedro) fica nas propriedades da folha ("Create projection view using first angle standard").
  - [V] A folha tem duas camadas: Working View (vistas, cotas) e Background View (moldura, carimbo). Está descrito em DR-42, do usuário.
- **CAD avançado A.**
  - [V] A folha tem Sheet Format (fundo com carimbo e moldura, editável à parte), Sheet Properties (escala, tipo de projeção, formato) e várias folhas. O formato da folha é um arquivo `.slddrt`.
  - [V] O padrão de desenho é uma propriedade do documento: ANSI, ISO, DIN, JIS, BSI, GOST, GB, mais padrões próprios salvos em `.sldstd`.
  - [V] O `.sldstd` leva anotações, cotas, linhas de centro, tabelas e vistas. As outras propriedades do documento ficam no modelo (template).
- **CAD avançado C.**
  - [V] A folha vem de templates que trazem Drawing Resources: Sheet Formats, Borders, Title Blocks e Sketched Symbols. Tamanho, carimbo e borda mudam depois de criar.
  - [V] A norma (ANSI, ISO, DIN…) é escolhida no Styles Editor (`Manage > Styles and Standards`).
  - [V] As vistas ortogonais seguem 1º ou 3º diedro conforme o padrão ativo.
- **Outros.** [V] CAD avançado E e Fusion têm folha com formato e orientação, e o Fusion exporta DWG e DXF por folha.
- **Fenkra hoje.**
  - `Sheet {id,name,format,portrait,zones}` (`lib.rs:151`) e 10 formatos (`lib.rs:21`).
  - 9 modelos fixos em `TEMPLATES` (`lib.rs:94`).
  - O carimbo (`TitleBlock`) é do desenho inteiro, não da folha. Não dá para ter folha ISO e folha ANSI no mesmo arquivo.
  - Não há escala padrão da folha nem método de projeção por folha.
  - Não há folha de fundo editável: `frame()` (`layout.rs:465`) gera moldura e carimbo por código.
  - Bug provável: `Drawing::delete` de uma folha move as vistas para a primeira (`lib.rs:625`) em vez de pedir confirmação. Pode empilhar vistas de duas folhas na mesma folha.
- **Melhor prática.**
  - Folha tem: formato, orientação, escala padrão, método de projeção, norma, modelo de carimbo e fundo.
  - Novo desenho segue a ordem: norma/modelo → formato → orientação → escala → projeção.
  - Trocar a norma de uma folha mostra o que vai mudar (carimbo, setas, estilo de texto) e permite "só esta folha" ou "todas".
  - Testes: (a) trocar A3→A4 não deixa vista fora da folha (aviso); (b) apagar folha com vistas pede confirmação; (c) salvar e abrir preserva norma e projeção por folha; (d) folha 2 herda a configuração da 1.

### 2.2 Carimbo (legenda), fundo e campos
(CAD avançado B: Frame & Title Block, Background View / CAD avançado A: Sheet Format, Custom Properties links / CAD avançado C: Title Block, Border / Fenkra: `TitleBlock`, `frame`)

- **O que faz.** Mostra identificação do desenho (empresa, título, código, material, responsáveis, data, revisão, escala, folha) e leva campos ligados às propriedades do modelo.
- **CAD avançado B.**
  - [V] O fundo é a moldura e o carimbo, mais qualquer elemento que se copie de uma folha de outro desenho. Opções `Copy background view` e `First sheet` / `Other drawing`.
  - [V] `New Sheet` abre "Insert Elements into a Sheet", com Browse até um desenho-fonte e Show Preview. Se `Copy background view` estiver desmarcado, só as propriedades da folha são copiadas.
  - [V] A moldura e o carimbo têm criação própria (Frame/Title Block creation) e aceitam inserir imagem.
- **CAD avançado A.** [V] A folha tem o fundo ("Edit Sheet Format"), onde se desenha o carimbo, e os campos são notas ligadas a propriedades personalizadas (`$PRP:"..."`). [C] Isso liga o carimbo às propriedades do modelo e da folha.
- **CAD avançado C.** [V] O carimbo é um recurso (`Manage > Define > Title Block`) feito num ambiente de croqui. Pode ter campos de propriedade (iProperties) e fica no template.
- **Fenkra hoje.**
  - 4 estilos de carimbo (`TitleStyle`, `lib.rs:69`): ISO 7200 (180 mm), ABNT (175 mm), ANSI e Simples.
  - Os campos `{chave}` são resolvidos por `resolve` (`lib.rs:125`) a partir da origem da **primeira vista da folha** (`sheet_fields`, `layout.rs:430`).
  - A massa usa `density` do carimbo, não o material do modelo. É uma massa "do desenho", pode divergir da massa da peça.
  - Não dá para editar o fundo (célula, logotipo, linha) nem salvar como formato reutilizável.
  - `{autor}` vem da variável de ambiente USERNAME, que não é a mesma coisa que o usuário do Fenkra.
  - Zoneamento ISO 5457 existe (`Sheet.zones`).
- **Melhor prática.**
  - Fundo como objeto: lista de células de carimbo editáveis (texto, campo, linha) salvas num "formato de folha" `.fkf` reaproveitável. Veja ADR 0011, princípio 5.
  - Campos oferecem a lista (não só digitação). Campo sem valor mostra `⟨campo?⟩` em cinza, não vazio.
  - A massa deve vir do material da peça (massa específica do material), com a densidade do carimbo só como sobrescrita.
  - Testes: (a) trocar o material da peça muda `{massa}` ao reabrir; (b) desenho multi-folha tem `{folha}/{folhas}` corretos; (c) carimbo com campo inexistente mostra aviso.

### 2.3 Norma de desenho e herança (norma → folha → vista → elemento)
(CAD avançado B: Standards + Properties / CAD avançado A: Document Properties > Drafting Standard / CAD avançado C: Styles and Standards / Fenkra: parcial)

- **O que faz.** Faz com que setas, texto, espessuras, hachura e símbolos sigam ISO, ASME, DIN, ABNT… e que cada elemento herde do nível acima, com sobrescrita local.
- **CAD avançado B.**
  - [V] A norma é escolhida na criação do desenho e pode ser trocada (`Page Setup`). Os estilos (setas, texto, hachura de "Area Fill") vêm dos arquivos de norma (XML).
  - [V] Um registro do IBM diz que o padrão de Area Fill fica no arquivo de norma (`Styles > Area Fill > Default > Pattern`) e só vale para o comando Area Fill, não para vistas geradas. A saída é usar as propriedades de material do corpo.
  - [C] Mudar a norma depois pode deixar anotações antigas com o estilo antigo. O usuário descreve em DR-35/DR-40 a cadeia norma → vista → elemento → 3D → sobrescrita manual.
- **CAD avançado A.**
  - [V] `Document Properties > Drafting Standard`: o padrão geral (ANSI, ISO, DIN, JIS, BSI, GOST, GB) atualiza o padrão base de cada detalhe (anotações, cotas, centros, tabelas, vistas).
  - [V] Dá para salvar e carregar `.sldstd`.
  - [V] A cadeia é padrão do documento → propriedade da anotação (sobrescrita local).
- **CAD avançado C.**
  - [V] O Styles Editor guarda estilos por norma e templates começam de uma norma (ANSI, ISO, DIN). Notas de furo usam o estilo de cota associado; editar o estilo afeta todas as notas ligadas, editar uma nota só muda ela.
  - [V] A visualização de linhas tangentes e a projeção (1º/3º) seguem o padrão ativo.
- **Outros.** [C] CAD avançado F tem "drawing setup file" (`.dtl`), CAD avançado G tem predefinições de Drafting (ex.: zero na linha de base nas ordenadas, [V] fornecedor de CAD avançado KB).
- **Fenkra hoje.**
  - `PmiStyle` herda em dois níveis: elemento `.over(&v.ann)` (`layout.rs:396`, 416).
  - Largura, cor e estilo de texto das cotas vêm do desenho (`text_height`, `decimals`) ou do estilo.
  - Não há objeto "norma" com defaults (ISO tangentes visíveis, ASME ocultas; setas 3×; casas decimais; unidade), nem herança folha → vista.
  - Nenhum campo "origem do valor" (norma / folha / vista / sobrescrito).
- **Melhor prática.**
  - Estrutura `DraftStandard {id, projeção, linhas(espessuras), texto, seta, hachura, tangentes, decimais, unidade, vírgula}` com presets ISO, ASME, DIN, ABNT. A folha referencia a norma, a vista pode sobrescrever e o elemento também.
  - A UI mostra, ao lado de cada campo, "herdado de: folha" com um botão "restaurar". A cadeia é resolvida num só lugar (`resolve_style`), sem copiar valores.
  - Mudar a norma de uma folha muda tudo o que está "herdado" e não toca o que foi sobrescrito.
  - Testes: (a) trocar ISO→ASME muda projeção, setas e tangentes; (b) sobrescrita sobrevive à troca; (c) "restaurar" volta ao herdado; (d) salvar e abrir preserva a origem de cada valor.

### 2.4 Método de projeção: 1º e 3º diedro
(CAD avançado B: Sheet Properties > projection method / CAD avançado A: Sheet Properties > Type of projection / CAD avançado C: Drafting standard / Fenkra: só 1º)

- **O que faz.** Define de que lado da vista principal as vistas projetadas ficam (1º: de cima fica embaixo; 3º: de cima fica acima).
- **CAD avançado B.** [V] Por folha, caixa "Create projection view using first angle standard". Padrão da folha é 1º ângulo, e há relato de usuário que esqueceu de trocar para 3º ao desenhar em ASME. [V] ISO/ANSI e o ângulo são configurações independentes. [V] O assistente de vistas gera front/bottom/left para ISO 1º ângulo, e muda para ANSI 3º.
- **CAD avançado A.** [V] A escolha 1º/3º fica nas propriedades da folha (`Sheet Properties`, "Type of projection"). [C] A escolha vem do padrão do template, e vistas projetadas obedecem.
- **CAD avançado C.** [V] As projeções ortogonais permanecem alinhadas com a mãe e o padrão de desenho ativo determina 1º ou 3º ângulo.
- **Fenkra hoje.**
  - 1º diedro fixo: `Dir::axes` (`lib.rs:197`) e a regra de clique em `dwg_ui.rs:898–912` (abaixo = superior, direita = lateral esquerda).
  - O símbolo e o texto "1º diedro" são sempre impressos (`layout.rs:610–618`), e o quadro ANSI imprime "FIRST ANGLE PROJECTION" (`layout.rs:606`).
  - Bug provável 1: ANSI com 1º diedro fixo contradiz a norma ASME Y14.3, que usa 3º ângulo [C].
  - Bug provável 2: projetar de uma vista Topo para a direita devolve `Dir::Esquerda` (`dwg_ui.rs:905`) e `align_views` alinha só em y (`layout.rs:246`). Mas a lateral e a superior não compartilham um eixo horizontal, então a vista ficaria geometricamente desalinhada.
  - Bug provável 3: projetar de Section, Camera ou Flat usa `Dir` absoluto (`Axes::of(dir)` em `source.rs:296`), não os eixos da mãe, e dá uma vista "projetada" que não está em projeção com a mãe.
- **Melhor prática.**
  - Campo `Sheet.projection: First|Third`, com padrão vindo da norma. A projeção de vista filha deve usar os eixos da mãe (rotacionar as direções da mãe), nunca eixos globais.
  - Tabela única `(lado_do_clique, projeção) → eixo` para as 4 vistas (e as 2 extras) e símbolo desenhado conforme o método.
  - Testes: (a) mesmo clique abaixo da frente gera Topo em 1º e Base em 3º; (b) o símbolo muda; (c) projetar de uma Section dá vista cujo eixo é perpendicular ao da mãe; (d) trocar o método na folha não troca vistas já criadas sem avisar (opção "reprojetar").

---

### 2.5 Vista base (modelo)
(CAD avançado B: Front View / CAD avançado A: Model View / CAD avançado C: Base View / outros: Fusion Base View, CAD avançado E View / Fenkra: `ViewKind::Standard`)

- **O que faz.** Primeira vista, com orientação escolhida e escala. As outras derivam dela.
- **CAD avançado B.** [V] A vista Front usa um plano de referência do 3D; um manipulador posiciona. Há também vistas Isométrica e o assistente com configurações pré-definidas ("Wizard", ex.: front/bottom/right ou front/left/top). [V] Vista de montagem pode ser explodida (ver 2.17).
- **CAD avançado A.** [C] Model View pede uma peça ou montagem, orientação e opções (configuração, estado de exibição, escala, linhas ocultas, tangentes, importar anotações).
- **CAD avançado C.** [V] A vista base é a primeira, define escala e alinhamento das outras, e pode haver várias na folha. A orientação padrão é a da origem do modelo. Pode-se recuperar todas as cotas do modelo ao criar (`All Model Dimensions` na aba Display Options).
- **Outros.** [V] CAD avançado D e CAD avançado E têm Base View ortogonal de uma montagem ou peça.
- **Fenkra hoje.**
  - `add_view` com `Standard(Dir)` (`lib.rs:577`); conjunto de 3 vistas + isométrica (`add_standard_set`, `lib.rs:594`).
  - Orientações: 7 fixas (`Dir::ALL`) e `Camera` (a câmera do 3D).
  - Escala: livre, via `scale_combo`.
  - Estilo: linhas ocultas, tangentes, rótulo, `Render` (arestas, sombreado, sombreado com arestas).
  - Falta: escolher face ou plano nomeado do modelo como "frente" (hoje a frente é sempre −Y). Falta importar anotações do modelo ao colocar a vista. Falta escala herdada da folha. Falta configuração ou variante do modelo por vista.
  - Cache: a chave do `proj` inclui `source`, `kind`, ocultas, tangentes e `render`; **não inclui** `line_width` nem `color` (ok, só afetam o desenho).
- **Melhor prática.** Fluxo: origem (peça/montagem, estado/variante) → orientação (padrão, plano publicado do modelo, câmera) → escala (herda da folha) → opções (ocultas, tangentes, roscas, sombreado) → avançado (importar anotações, mostrar PMI). Orientação por **elemento de referência com nome publicado** (ADR 0010), para não depender do eixo global. Testes: (a) frente por plano publicado persiste ao editar a peça; (b) mudar a origem para outra peça mantém a vista e sinaliza; (c) escala herdada muda ao trocar a escala da folha.

### 2.6 Vistas projetadas
(CAD avançado B: Projection View / CAD avançado A: Projected View / CAD avançado C: Projected View / Fenkra: `ViewKind::Projected`)

- **O que faz.** Gera vistas ortogonais alinhadas à mãe, na direção em que o cursor se afasta dela.
- **CAD avançado B.** [V] O "quadro verde" da vista ativa gera projeções automaticamente (ao mover o cursor ao redor). Por padrão as vistas ficam alinhadas; `Do not align view` solta; `Set relative position` fixa a posição relativa por ponto, linha ou quadro de vista (o ponto verde gira a vista em torno da referência).
- **CAD avançado A.** [C] Projected View: clique ao lado da vista mãe; herda escala e estilo; alinhada por padrão (pode soltar).
- **CAD avançado C.** [V] Várias projeções numa só operação; a posição do cursor decide a direção; herdam escala e estilo da mãe; ortogonais permanecem alinhadas. [V] Para soltar o alinhamento: menu de contexto "Align View", `View Projection: None` ou Ctrl ao clicar.
- **Fenkra hoje.**
  - `c::PROJ` (`dwg_ui.rs:891`) cria uma vista por clique. `align_views` mantém o alinhamento e a escala (`layout.rs:238`). Não há "soltar alinhamento" nem vários de uma vez.
  - Escala de filhas segue a mãe (sempre sobrescrita, `layout.rs:249`): o usuário não consegue escala diferente numa filha projetada. É uma limitação, não um bug.
  - Bugs prováveis: veja 2.4 (projeção a partir de Topo, Section, Camera, Flat). Mover a mãe move as filhas? Não: só `pos[0]` ou `pos[1]` é forçado; a filha fica onde estava no outro eixo.
- **Melhor prática.** Gerar por arraste de cursor com pré-visualização e confirmar com clique. Filha guarda `(parent, lado)` e não `Dir` absoluto. Ctrl solta o alinhamento (flag `aligned: bool`). Testes: (a) mover a mãe move as alinhadas; (b) soltar alinhamento libera; (c) projetar de uma projetada em 3º diedro respeita a regra; (d) apagar a mãe apaga as filhas (já existe em `delete`).

### 2.7 Vista auxiliar
(CAD avançado B: Auxiliary View / CAD avançado A: Auxiliary View / CAD avançado C: Auxiliary View / outros: CAD avançado E Auxiliary / Fenkra: ausente)

- **O que faz.** Mostra a forma verdadeira de uma face inclinada, projetando perpendicular a uma aresta da vista mãe.
- **CAD avançado B.** [V] Clica-se uma aresta (ou ponto e aresta) numa vista adjacente. A aresta vira uma linha-chamada que define o plano de referência, e é **associativa** à geometria 3D. O posicionamento da chamada define a direção; pode-se inverter. A vista fica alinhada por padrão. Há opção em `Tools > Options > Drafting > Layout` para orientar vistas auxiliares e de corte conforme o perfil.
- **CAD avançado A.**
  - [V] Seleciona-se uma **aresta de referência** que não seja horizontal nem vertical (senão sai uma vista projetada comum).
  - [V] A referência pode ser aresta da peça, aresta de silhueta, eixo ou linha de croqui.
  - [V] A vista é "desdobrada" normal à referência. Não está disponível a partir de vista de detalhe.
  - [V] Linha de croqui fica absorvida: para mudar, edita-se o croqui pela seta de vista.
- **CAD avançado C.** [V] Projetada perpendicularmente a uma linha ou aresta, posição pelo cursor, herda escala e estilo. Serve para faces inclinadas.
- **Outros.** [V] CAD avançado E: vista ortogonal dobrada 90° a partir de uma aresta, ou sobre croqui/linha de centro. Detalhe e vista interrompida não podem ser mãe. Uma atualização de 2021 permitiu auxiliar → corte.
- **Fenkra hoje.** Não existe. O catálogo (DES-04) diz "Detalhe, auxiliar" mas só há `Camera` (câmera do 3D), que não é associativa a uma aresta. **Item do catálogo desatualizado.**
- **Melhor prática.**
  - Suporte: aresta ou eixo (de uma vista) ou face plana → modo: direção normal à referência, lado (setas) → parâmetros: rótulo e escala → avançado: alinhamento.
  - Guardar a referência como **chave estável** (dono+papel; ADR 0010), não coordenada, e recalcular a direção no update.
  - Novo `ViewKind::Auxiliary {parent, edge: RefKey, fallback: V2×2, label}`.
  - Testes: (a) aresta inclinada a 30° gera vista com a face verdadeira (medida = valor 3D); (b) editar o ângulo da face na peça gira a vista; (c) aresta que some → estado "referência perdida" (não escolhe outra); (d) aresta horizontal avisa e propõe projetada.

### 2.8 Corte total (e linha de corte)
(CAD avançado B: Offset Section View/Cut / CAD avançado A: Section View / CAD avançado C: Section / Fenkra: `ViewKind::Section`)

- **O que faz.** Corta o modelo por um plano definido numa linha na vista mãe, descarta o lado do observador, hachura a seção e mostra setas e rótulo.
- **CAD avançado B.**
  - [V] Existem **Section View** (resultado como vista) e **Section Cut** (corte "só" da geometria, o `Cut` mantém a cor/estilo 3D); clica-se nos furos/pontos que definem o perfil e dá-se duplo clique para terminar. O perfil é associativo aos furos.
  - [V] O sentido sai da posição do cursor ao colocar a vista; o perfil pode ser invertido ou substituído depois (duplo clique).
  - [V] Propriedades da vista mudam o padrão da hachura. A moldura da vista ativa se adapta ao comprimento do perfil.
  - [V] "Not cut in section views" (no Assembly Design) exclui uma peça do corte.
  - [V] Há ainda corte por superfície plana e com perfil definido em 3D.
- **CAD avançado A.**
  - [V] PropertyManager: método Vertical, Horizontal, Auxiliar ou Alinhado; linha de corte desenhada; `Flip direction`; `Auto hatching`; escala do padrão; `Section Scope` (excluir componentes/nervuras do corte; por padrão exclui fixadores da biblioteca de normas e dá para marcar com a propriedade `IsFastener`).
  - [V] Dá para limitar a profundidade do corte para não cortar a vista toda (não vale para o corte alinhado).
  - [V] A seção muda de ângulo entre peças vizinhas (auto hatching) e a escala pode ser aleatória por material (`Randomize scale`).
- **CAD avançado C.**
  - [V] Cria-se a partir de um croqui de linha de corte (ou desenhando na hora). Método projetado (padrão para segmentos todos a 90°) ou alinhado (padrão se algum segmento não for perpendicular). Aparecem setas na mãe, hachura e rótulo automáticos.
  - [V] Edita-se arrastando pontos, segmentos e setas.
  - [V] `Section Depth`: Full ou Distance; zero vira a menor profundidade (0,000012).
  - [V] `Include Slice` faz corte de espessura zero. `Slice All Parts` sobrescreve a configuração do navegador.
  - [V] Dá para excluir componentes do corte antes de criar e controlar a herança do corte (`Cut Inheritance`).
- **Outros.** [C] CAD avançado E: corte vertical, horizontal ou em ângulo; também a partir de outras vistas.
- **Fenkra hoje.**
  - `ViewKind::Section {parent, a, b, label}` (`lib.rs:229`): 3 cliques (a, b, onde colocar) em `dwg_ui.rs:916`.
  - `compute` (`source.rs:331`): monta o eixo (`n = toward × u`), corta com uma caixa (`box_tool`) e hachura com `section_loops`.
  - Rótulo "CORTE A-A (escala)" e setas nos dois extremos (`layout.rs:361–377`).
  - Inversão: o painel diz para "apagar e traçar no sentido oposto" (`dwg_ui.rs:1429`). Não há `Flip`.
  - Falta: profundidade limitada (distance), excluir componente do corte (existe `hidden`, mas esconde em vez de "não cortar"), setas sem comprimento ajustável, vista de corte **não alinhada** ao traço (só escala é forçada em `align_views`), hachura sem ângulo/padrão (2.31).
  - Bug provável: a profundidade do plano usa o centro da peça (`source.rs:336`), então um corte de montagem pega um ponto "no meio" que pode não ser onde o usuário clicou, em montagens assimétricas. Quando `boolean` falha, o erro é mostrado, mas o resultado vira a peça inteira sem corte, sem hachura (`source.rs:349`).
  - Bug provável: letras: `next_label` só reaproveita A–Z (`dwg_ui.rs:164`); com mais de 26 usa "Z" repetido.
- **Melhor prática.** Suporte: vista mãe + linha de corte (ou aresta/eixo/plano publicado) → modo (total, meio, alinhado, desvio) → parâmetros (letra, sentido, profundidade, escala) → avançado (componentes não cortados, hachura, arestas de corte). Regras: o plano é recalculado do **traço** e não do centro da peça; a vista de corte é alinhada ao traço (movimento só ao longo da perpendicular), `Flip` na UI; componentes não cortados (fixadores, eixos, nervuras) por regra e por propriedade. Testes: (a) corte de um bloco com furo: hachura sem preencher o furo; (b) `Flip` troca o lado visto; (c) montagem com parafuso: parafuso não hachurado; (d) mudar a posição do traço no 3D move o corte; (e) mover a vista de corte fora do alinhamento é bloqueado.

### 2.9 Meio corte (half section)
(CAD avançado B: Offset/Aligned com perfil em L / CAD avançado A: Half Section / CAD avançado C: Section com 2 segmentos perpendiculares / Fenkra: ausente)

- **O que faz.** Corta só um quarto ou meia peça simétrica. Metade mostra o interior e a outra o exterior.
- **CAD avançado B.** [C] Faz-se um perfil de corte em ângulo reto (offset). Não há comando "half" dedicado.
- **CAD avançado A.** [V] No PropertyManager de Section View, a aba Half Section tem orientações: lado superior (direita ou esquerda), lado esquerdo (para cima ou baixo) e lado direito (para cima ou baixo). Há relatos de que, depois de criada, não dá para inverter a direção (versão 2013).
- **CAD avançado C.** [V] O método é inferido da geometria: um segmento = total; dois perpendiculares = meio; dois não perpendiculares = alinhado; três ou mais = desvio (API `AddSectionView2`).
- **Fenkra hoje.** Ausente. Linha de corte é um segmento só.
- **Melhor prática.** Modelar o corte como **polilinha** (`cut: Vec<V2>` com ≥ 2 pontos) e inferir o tipo como o CAD avançado C faz, com `kind` calculado e mostrado. Meio corte: o quadrante removido é o que fica no lado do observador, com linha de centro como limite entre meio exterior e meio corte (sem aresta de corte). Testes: (a) eixo em meio corte: metade com hachura e metade sem; (b) a linha de simetria é traço-ponto; (c) trocar o quadrante não deixa lacuna.

### 2.10 Corte em desvio (offset) e corte alinhado
(CAD avançado B: Offset/Aligned Section View/Cut / CAD avançado A: Aligned Section / CAD avançado C: Projected/Aligned method / Fenkra: ausente)

- **O que faz.** Corta por um plano dobrado (desvio) ou por planos que se cruzam em ângulo, rebatendo a parte inclinada para o plano de projeção (alinhado).
- **CAD avançado B.** [V] Perfil de planos paralelos (offset) ou **não paralelos** (alinhado), em que o plano é "dobrado" e o trecho inclinado é girado para o plano original. Perfil associativo a furos/pontos.
- **CAD avançado A.** [V] A linha do corte alinhado tem duas ou mais linhas conectadas em ângulo, sem contornos múltiplos. [V] Profundidade limitada não vale para alinhado.
- **CAD avançado C.** [V] Alinhado: a vista é criada perpendicular à linha de projeção, "as linhas de corte do corpo não aparecem", e só vale com `Include Slice` marcado e não para vistas filhas dependentes. Padrão quando algum segmento é não perpendicular. Projetado: usa a projeção da linha; padrão quando todos os segmentos são 90°.
- **Fenkra hoje.** Ausente.
- **Melhor prática.** Com a polilinha de 2.9: segmentos paralelos entre si → desvio (um único plano de projeção, trechos em profundidades diferentes, a seção é "dobrada" na hachura); segmentos concorrentes → alinhado (rotaciona o trecho inclinado em torno do ponto de quebra antes de projetar). Implementar em duas fases: (1) desvio por união dos cortes em planos paralelos; (2) alinhado por rotação do sólido recortado. Testes: (a) flange com furos em círculo de parafusos: corte alinhado mostra 2 furos na mesma vista com o tamanho real; (b) corte em desvio mostra furos em profundidades diferentes; (c) a seta e o traço mostram a quebra com marcas grossas.

### 2.11 Corte parcial (break-out / broken-out section)
(CAD avançado B: Breakout View / CAD avançado A: Broken-out Section / CAD avançado C: Break Out / Fenkra: ausente; catálogo cita mas não há)

- **O que faz.** Remove material localmente dentro de um contorno fechado para expor o interior, mantendo o resto da vista.
- **CAD avançado B.**
  - [V] A vista tem que estar ativa. Define-se um perfil (cliques, duplo clique fecha); depois de criado o perfil não pode ser modificado.
  - [V] A profundidade vem de dois planos num visualizador 3D: um fixo e um móvel, arrastado ou "pego" numa aresta/círculo de uma vista perpendicular. A opção Animate gira a peça.
  - [V] A geometria do breakout **não é associativa** às vistas geradas. Pode-se aplicar a outras vistas (`Apply Breakout To`) e remover. Uma vista não pode ter breakout e vista interrompida ao mesmo tempo. A opção `Propagation of broken and breakout specification` leva o breakout para as vistas geradas depois.
- **CAD avançado A.**
  - [V] A profundidade usa uma referência: aresta ou eixo, que pode acompanhar o centro de um círculo ou a silhueta de um cilindro. Há pré-visualização ao vivo e os planos de quebra aparecem nas outras vistas. `Auto hatching` só em montagens.
  - [V] Desde 2018, pode ser feita em vista de posição alternativa, detalhe e corte.
- **CAD avançado C.**
  - [V] Precisa de um croqui **fechado** dentro da vista (não no nível da folha).
  - [V] Profundidade: From Point, To Sketch (um segundo croqui numa vista direta dá a profundidade), Through Part ou To Hole.
- **Fenkra hoje.** Ausente. O catálogo diz "sem corte parcial" (DES-04, backlog B30).
- **Melhor prática.**
  - Contorno: polilinha fechada na vista mãe, mais profundidade (de ponto, até aresta/eixo publicado, até furo ou atravessando).
  - Cálculo: corte da peça com um prisma (contorno extrudado até a profundidade) e hachura só na face de corte. O resto da vista permanece.
  - A **referência de profundidade** deve ser chave estável. Se sumir → estado "referência perdida" com Editar ou Reatribuir.
  - Testes: (a) caixa com furo cego: breakout até o eixo do furo mostra o furo; (b) profundidade zero → erro claro; (c) apagar a aresta de referência marca o breakout em vermelho; (d) breakout propaga para a projetada quando a opção está ligada.

### 2.12 Seção removida / seção girada
(CAD avançado B: section cut/profile / CAD avançado A: Section View (fora da vista) / CAD avançado C: Section com vista deslocada / Fenkra: ausente)

- **O que faz.** Mostra só o contorno do corte (sem o que está atrás), perto do local, comum em eixos, perfis e braços.
- **Pesquisa.** [C] CAD avançado A e CAD avançado C tratam como corte cuja vista é posta fora de alinhamento e com profundidade pequena (`Section Depth: Distance` no CAD avançado C [V], `Slice` [V] = espessura zero). No CAD avançado B, é o *Section Cut* [V].
- **Fenkra hoje.** Ausente. `Section` sempre mostra o que está atrás do plano.
- **Melhor prática.** Opção `profundidade: Total | Distância(d) | Fatia(0)` no corte (2.8). Para "seção girada sobre a vista" (revolved), colocar o contorno na própria vista mãe com linha de contorno fina. Testes: (a) fatia de 0 mm de um eixo escalonado mostra só o círculo hachurado; (b) distância d mostra só o que está até d.

### 2.13 Vista de detalhe
(CAD avançado B: Detail View / Quick Detail / CAD avançado A: Detail View / CAD avançado C: Detail View / Fenkra: `ViewKind::Detail`)

- **O que faz.** Amplia uma região (círculo ou contorno) de uma vista mãe, com rótulo na mãe e na vista.
- **CAD avançado B.**
  - [V] Chamada circular ou perfil poligonal. O **Detail View** faz uma operação booleana no 3D; o **Quick Detail** calcula direto da projeção 2D (representação diferente, sem os dress-up do 3D). Escala padrão ×2, mudável em Properties; a borda também é editável.
  - [V] Rótulo com letra, que pode ser movido depois.
  - [V] Limitação: um detalhe feito a partir de um corte não atualiza se o perfil de corte mudar de forma que o detalhe deixe de ser válido.
- **CAD avançado A.**
  - [V] Estilo da chamada: Per Standard, With Leader, No Leader, Connected. Circle ou Profile (Profile só em alguns estilos). `Full outline`/`No outline`.
  - [V] `Scale hatch pattern`: a hachura do detalhe pode seguir a escala do detalhe e não a do corte.
  - [V] A borda pode ser irregular (jagged outline, com intensidade ajustável).
- **CAD avançado C.** [V] Escala padrão ×2, **não alinhado** à mãe; rotula vista e área; cerca circular ou retangular.
- **Outros.** [V] CAD avançado D e CAD avançado E (detalhe de corte desde 2016/17, segundo fóruns) têm Detail.
- **Fenkra hoje.**
  - `Detail {parent, center, radius, label}` (`lib.rs:231`); escala = 2× a da mãe na criação (`dwg_ui.rs:932`).
  - **Não recalcula** do 3D: recorta as polilinhas da mãe (`clip_circle`, `source.rs:247`). Quando a mãe mostra tudo com a tolerância do tessellation dela, o detalhe herda a facetização. Arestas curvas podem aparecer poligonais em escalas altas (bug provável).
  - A hachura da mãe vira só contorno (`source.rs:252–256` empurra `lp` como linhas, não como hachura). Detalhe de corte perde a hachura.
  - Marca na mãe: círculo fino e letra (`layout.rs:378`).
  - Falta: perfil poligonal, estilo da chamada (com ou sem linha), contorno visível ou não, alinhamento opcional.
- **Melhor prática.** O detalhe deve ser calculado como recorte do **HLR da mãe com tolerância refinada** ou, melhor, nova projeção com o mesmo plano e uma caixa de recorte. Guardar o raio em mm do modelo. Hachura do detalhe: recortar a região hachurável e re-hachurar. Testes: (a) detalhe de raio R em escala 10:1 tem arco liso (flecha < 0,01 mm no tamanho impresso); (b) detalhe de corte hachurado; (c) mover o centro do detalhe na mãe atualiza; (d) a mãe sem a geometria no círculo → "detalhe vazio" (aviso).

### 2.14 Vista recortada (clipping / crop)
(CAD avançado B: Clipping View / CAD avançado A: Crop View / CAD avançado C: Crop / Fenkra: ausente)

- **O que faz.** Mostra só uma parte de uma vista (sem ampliar), cortando a moldura.
- **CAD avançado B.** [V] Círculo ou polígono na vista ativa. Cotas fora da região e anotações desligadas viram NoShow automaticamente; o ícone `Swap visible space` as mostra para revisão. Cota que se quer de volta é religada por Hide/Show. `Unclip` **não** restaura as cotas escondidas.
- **CAD avançado A.** [C] Crop View: contorno croqui sobre a vista.
- **CAD avançado C.** [V] `Crop` controla o limite da vista.
- **Fenkra hoje.** Ausente.
- **Melhor prática.** Propriedade da vista `clip: Option<Polygon>` aplicada em `to_sheet`; cotas fora ficam ocultas **com aviso** (contador "n ocultas"), não apagadas. Testes: (a) recortar e desfazer o recorte restaura as cotas; (b) a vista alinhada continua alinhada.

### 2.15 Vista interrompida (broken view)
(CAD avançado B: Broken View / CAD avançado A: Broken View / CAD avançado C: Break / Fusion: Break View / Fenkra: ausente)

- **O que faz.** Encurta peças longas removendo um trecho do meio e marcando a quebra. A cota através da quebra mostra o comprimento real.
- **CAD avançado B.** [V] Exige uma vista gerada, ativa e atualizada, e dois perfis em extremidades da vista. [V] Quebra e breakout não coexistem na mesma vista; a propagação para vistas filhas é uma opção.
- **CAD avançado A.**
  - [V] Estilo da linha de quebra (o mesmo em todas as quebras da vista); largura da lacuna (gap) editável.
  - [V] Quebras horizontais e verticais podem coexistir. `Un-Break View` desfaz, e as linhas de quebra permanecem para serem apagadas.
  - [V] Cotas que cruzam a quebra são quebradas automaticamente, com opção de zigue-zague em Document Properties.
- **CAD avançado C.** [V] Uma quebra por sessão do diálogo; orientação horizontal, vertical ou alinhada à projeção; Gap; estilo estruturado com 1, 2 ou 3 símbolos por quebra.
- **Outros.** [V] Fusion: Break View aplica-se a projetadas e à base, e vice-versa.
- **Fenkra hoje.** Ausente.
- **Melhor prática.** `View.breaks: Vec<Break{dir, a, b, style, gap}>`; a projeção 2D comprime o eixo entre `a` e `b`. A cota **usa valores do modelo** (nunca o 2D comprimido). Quebra se propaga às filhas. Testes: (a) eixo de 1000 mm com quebra mostra cota 1000; (b) filha projetada herda a quebra; (c) desfazer a quebra restaura as linhas.

### 2.16 Vista de montagem (componentes, estados, corte de montagem)
(CAD avançado B: product views / CAD avançado A: configurações + display states / CAD avançado C: representations, Overlay / Fenkra: `Source.display_state`, `hidden`)

- **O que faz.** Vista de uma montagem escolhendo o que aparece (estado de exibição, componentes escondidos) e, nos cortes, o que é cortado.
- **CAD avançado B.** [V] Vistas de montagem a partir do produto, explodidas ou não. [V] "Not cut in section views" por peça (Assembly Design). [V] Cota/anotação de produto é gerada pelas restrições de montagem. [V] Informações de BOM podem ir à vista ativa.
- **CAD avançado A.** [V] Vistas mostram configuração e estado de exibição; `Section Scope` exclui componentes; `Alternate Position View` sobrepõe, em fonte fantasma, uma segunda configuração da montagem para mostrar faixa de movimento. Não está disponível em vistas interrompidas, de corte, recortadas ou de detalhe. Desde 2020 vale também para peças (duas configurações).
- **CAD avançado C.** [V] A vista de sobreposição (**Overlay**) junta várias representações posicionais numa só vista. [V] Componentes podem ser excluídos do corte.
- **Fenkra hoje.** `Source {path, display_state, hidden}` (`lib.rs:172`). `hidden` esconde componentes **só nesta vista** (`source.rs:185`); a UI tem checklist (`dwg_ui.rs:1458`). Estado de exibição é digitado como texto (`dwg_ui.rs:1449`), sem lista. Falta: "não cortar" por componente, sobreposição de posições, e lista de estados.
- **Melhor prática.** Estado de exibição escolhido numa lista (lida da montagem); regra "componente não cortado" em duas camadas (propriedade do componente e sobrescrita na vista). Sobreposição como segunda `Source` com fonte fantasma. Testes: (a) estado removido do `.fka` → vista mostra erro claro e não "como está"; (b) fixador marcado "não cortar" aparece inteiro no corte.

### 2.17 Vista explodida
(CAD avançado B: Exploded View / CAD avançado A: Exploded view in drawing / CAD avançado C: Presentation / Fenkra: via `display_state`)

- **O que faz.** Mostra a montagem explodida em vista isométrica, com linhas de explosão e balões.
- **CAD avançado B.** [V] Define-se a cena e a explosão no DMU Navigator; no Drafting usa-se Isometric View, escolhe-se o produto e um plano, e clica-se para colocar. A explodida vira isométrica explodida; a não explodida, isométrica comum. [V] BOM pode ir à vista.
- **CAD avançado A.** [C] A vista de desenho tem a opção "Show in exploded or model break state", que usa a explosão da configuração. (Não confirmei esta opção na ajuda; a busca não a encontrou.)
- **CAD avançado C.** [C] Vista de apresentação (`.ipn`) com trilhas, e vistas de desenho feitas dela.
- **Fenkra hoje.** O catálogo e o painel orientam a usar `display_state` "vista explodida" do compositor (`dwg_ui.rs:1451`). Não há trilhas nem lista de estados.
- **Melhor prática.** Estado de explosão como entidade da montagem; vista de desenho seleciona-o por lista; linhas de explosão (trilhas) como linhas finas traço-ponto geradas do caminho de cada componente. Testes: (a) mudar a distância de explosão atualiza a vista; (b) balões continuam apontando para a peça.

### 2.18 Vista planificada (chapa)
(CAD avançado B: Unfolded View / CAD avançado A: Flat Pattern view / CAD avançado C: Flat Pattern view / Fenkra: `ViewKind::Flat`)

- **O que faz.** Mostra a chapa desdobrada, com linhas de dobra e tabela de dobras.
- **CAD avançado B.** [V] Exige licença de Sheet Metal Design; é uma vista projetada da chapa. Eixos e limites de dobra tracejados, e eixos de furos. Seleciona-se uma face (usa o plano da parede de referência); manipuladores nas linhas de eixo.
- **CAD avançado A.** [V] Tabela de dobras substitui os rótulos individuais, com direção, ângulo e raio. A tabela de punção é para formas.
- **CAD avançado C.** [V] Notas de dobra e de punção em vistas planificadas; tabela de dobras pelo comando General Table; é possível renumerar dobras e inverter o sentido.
- **Fenkra hoje.** `Flat` (`source.rs:273`): copia o documento com `s.flat = true`, usa `recompute` para `bend_lines`, e rotula "↑ 90° R2". Escolhe o eixo pela menor dimensão (`source.rs:307`). Teste existe (`lib.rs:779–799`).
  - Bugs prováveis: (a) chapas em L/U com a menor extensão ambígua escolhem eixo errado (comparação de bbox); (b) não há bounding box com acréscimo de dobra impressa; (c) a tabela de dobras (`bend_table`) não referencia as etiquetas da vista.
- **Melhor prática.** Eixo da planificada = normal da face de referência da chapa (publicada), não a menor dimensão. Etiquetas das dobras (B1, B2…) iguais às da tabela (RF-DES-33). Exportar a planificada em DXF **1:1** na camada de corte e de dobra. Testes: (a) chapa com 4 dobras: 4 rótulos e 4 linhas na tabela com os mesmos nomes; (b) DXF 1:1 mede o comprimento desenvolvido.

### 2.19 Atualização e estado das vistas (update, travar, isolar, quebrado)
(CAD avançado B: Update / Lock / Isolate / Show Geometry / CAD avançado A: Rebuild, Lock View Position / CAD avançado C: Update, Defer Update / CAD avançado H: Update Views / CAD avançado E: Update Drawing / Fenkra: ❌)

- **O que faz.** Detecta que o 3D mudou, marca o que está desatualizado e deixa atualizar, adiar, travar ou desconectar.
- **CAD avançado B.**
  - [V] Modo padrão de atualização é automático; o manual usa o ícone Update (fornecedor de CAD avançado/PLM avançado).
  - [V] **Lock View** (propriedades da vista): impede criar vistas filhas, anotar, **atualizar**, apagar ou cortar; mover continua. Fica marcado na árvore.
  - [V] **Isolate**: rompe o vínculo com o CATPart e converte em vista interativa; irreversível; centros e eixos ficam presos a pontos; cotas ficam sem associação; alguns dress-ups e anotações podem ser apagados.
  - [V] **Show Geometry in All Viewpoints**: ao passar o mouse numa aresta da vista, destaca a aresta e as faces vizinhas em todas as vistas e na janela 3D.
  - [V] A cota gerada pode **dirigir** a restrição 3D (editar a cota muda a peça), com opção administrativa para proibir.
  - [C] O usuário (DR-34) descreve vistas "não atualizadas", linhas vermelhas e cotas amarelas.
- **CAD avançado A.**
  - [V] Cotas penduradas (dangling) ficam vermelhas após reconstruir; relatos de usuários em cortes derivados de outra vista.
  - [V] Dá para travar a posição da vista. [C] Não há "estado desatualizado" de vista como no CAD avançado B: o desenho é reconstruído com o modelo.
- **CAD avançado C.**
  - [V] Atualiza automaticamente por padrão. Ícone **View Out of Date** no navegador. **Defer Update** adia: vistas, anotações e iProperties dependentes ficam fixas; só se editam anotações que não vêm do modelo (não se pode recuperar dados do modelo, nem criar ou editar vistas, nem mexer em listas e balões).
  - [V] Alterações grandes podem soltar anotações; é preciso religá-las. O comando Update fica esmaecido quando tudo está em dia.
  - [V] Opções: só a folha ativa, todas, ou uma.
- **CAD avançado H.** [V] Vistas desatualizadas ganham **borda cinza** e há o comando Update Views. [V] `Retrieve dimensions on update` é propriedade da vista.
- **CAD avançado E.** [V] O ícone Update Drawing ativa quando a parte muda (mesmo uma cota de croqui); atualiza só as vistas; anotações que perdem referência ficam vermelhas (entidades penduradas). Versionar o documento "congela" o desenho.
- **CAD avançado G.** [V] Preferência *Inherit PMI Tracking: Annotation Location* para a PMI do desenho seguir a modelagem.
- **Fenkra hoje.**
  - `refresh()` compara data de arquivo a cada 1,5 s (`source.rs:128`, `dwg_ui.rs:475`) e invalida. Recalcula **sem avisar**.
  - Erro de vista aparece como texto "! …" acima da vista (`layout.rs:347`).
  - `F5` limpa tudo (`dwg_ui.rs:455`).
  - Cotas: valor calculado dos âncoras (2.21), nunca mudam sozinhas.
  - Não há: estado por vista, adiar, travar, isolar, relatório de "o que mudou", nem conferência de referências quebradas.
  - Risco: salvar a peça no meio de uma edição (estado inconsistente) já recalcula a folha. Projeções caras (HLR) rodam na thread de UI.
- **Melhor prática.**
  - Estado por elemento: `Atualizado | Desatualizado | Erro | ReferênciaQuebrada | Travado | Isolado`, derivado de assinatura (hash da forma final + carimbos de arquivo). Mostrar em cor na árvore e na folha.
  - Barra "Atualizar (folha ativa / todas)" e chave "Atualização automática" por desenho (padrão ligada, como CAD avançado C).
  - Painel de diagnóstico (ADR 0011, princípio 1): lista de itens com problema e ações Editar, Substituir, Isolar, Desativar.
  - Cálculo de projeção fora da thread de UI com cancelamento.
  - Testes: (a) mudar a largura da peça marca a vista como desatualizada e a cota como "valor mudou"; (b) vista travada não muda; (c) isolar mantém linhas e perde associação; (d) remover o arquivo-fonte → todas as vistas "ReferênciaQuebrada", nada some; (e) abrir o desenho com a peça alterada fora do app mostra "desatualizado".

### 2.20 Propriedades da vista, alinhamento e posição relativa
(CAD avançado B: Properties, Set Relative Position, Do not align / CAD avançado A: view properties, lock / CAD avançado C: Align / Fenkra: painel da vista)

- **O que faz.** Controla escala, linhas ocultas, tangentes, rótulo, alinhamento e a posição entre vistas.
- **CAD avançado B.** [V] `Set Relative Position`: por ponto, linha ou quadro de vista (alinha pelo baricentro); o ponto verde gira a vista em torno da referência; comprimento da linha se mantém nos movimentos seguintes. Escala padrão de vista em propriedades; rename de vista; `Do not align view`.
- **CAD avançado A.** [C] Alinhamento por padrão; "break alignment"; opções de linhas ocultas e tangentes por vista.
- **CAD avançado C.** [V] Alinhamento desligado por menu, por `View Projection: None` ou Ctrl.
- **Fenkra hoje.** Painel: nome, escala, ocultas, tangentes, rótulo, renderização, cor, espessura (`dwg_ui.rs:1386–1409`). Alinhamento sempre ligado nas projetadas (`align_views`). Rótulo sem texto customizável; sem ângulo de rotação da vista (CAD avançado B e CAD avançado A têm).
- **Melhor prática.** Adicionar `angle` (rotação da vista), `aligned: bool` e rótulo editável com variáveis. Testes: (a) rotação de 90° gira cotas junto; (b) soltar alinhamento e mover; (c) escala por vista não afeta as filhas se "escala própria".

---

### 2.21 Cotas associativas (linear, raio, diâmetro, ângulo)
(CAD avançado B: Dimensions / CAD avançado A: Smart Dimension / CAD avançado C: General Dimension / Fenkra: `Dim`)

- **O que faz.** Mede entre geometrias da vista e permanece ligada ao modelo.
- **CAD avançado B.**
  - [V] Cotas geradas são associativas aos elementos de uma peça ou montagem.
  - [V] Dá para posicionar as cotas vista a vista e analisar cotas que se interferem.
  - [V] A "Dimension Properties" traz tolerância, estilo de texto/seta, e o estado de consistência (DR-38).
- **CAD avançado A.** [C] Smart Dimension escolhe o tipo pelos objetos clicados (linear, raio, diâmetro, ângulo, arco). Cotas inseridas diretamente no desenho são de **referência** (não dirigem o modelo). Cotas importadas do modelo (2.22) dirigem o modelo.
- **CAD avançado C.**
  - [V] Cotas de desenho (General Dimension) não afetam peças. Texto sobrescrito, precisão, tolerância, seta e linha de chamada editáveis sem mudar o modelo.
  - [V] Retrieve Dimensions traz cotas do modelo, e editá-las muda a peça.
- **Outros.** [V] Fusion: relato de terceiros diz que cotas feitas no desenho entram como "anotações desassociadas" que não são reassociáveis.
- **Fenkra hoje.**
  - Tipos: H, V, Alinhada, Raio, Diâmetro, Ângulo (`DimKind`, `lib.rs:298`).
  - O valor é calculado dos `anchors` (3D) projetados na vista (`dim_value`, `layout.rs:262`).
  - **Os âncoras são pontos soltos.** Criados com `ax.unproject(from_sheet(clique))` (`dwg_ui.rs:947`). O snap pega vértices de aresta, mas o dado guardado é só a coordenada.
  - Consequência: se a peça muda de 100 para 120, a aresta cresce mas a cota continua marcando 100 e aponta para um ponto no ar. Isso é o contrário do pretendido (RF: "mudou o 3D, o desenho acompanha").
  - Raio/diâmetro: acha o arco mais próximo por `arc_center` nas polilinhas (`dwg_ui.rs:795`), aproximado por três pontos; vale para arcos facetados, com erro de tessellation no valor (bug provável: cota de Ø da ordem de 1e-3 de erro).
  - Falta: tipo da cota pelo contexto (smart), cota de referência `(…)`, arco, comprimento de arco, simetria, ordenada, chanfro, linha de base, quebra de cota.
  - Formatação: `decimals` global; sem precisão por cota; `tol` é texto livre (`dim_text`, `layout.rs:281`).
  - Setas e texto arrastável são bons (`text_pos`, `arrows`).
- **Melhor prática.**
  - **Âncora = referência estável + fallback**: `Anchor::Edge{key, t}`, `Anchor::Vertex{key}`, `Anchor::Point(V3)`. `key` = chave dono+papel (ADR 0010). No update, resolve a chave → ponto atual. Se não resolve: cota fica em vermelho, `ReferênciaQuebrada`, com Reatribuir (clicar de novo) e nunca escolhe outra aresta em silêncio.
  - Raio e diâmetro devem ler o valor da **superfície analítica** (cilindro/cone/toro) no kernel, não da polilinha.
  - Smart: um clique numa aresta = comprimento; duas arestas paralelas = distância; círculo = Ø; duas arestas não paralelas = ângulo.
  - Estado de consistência (verde, amarelo, vermelho) como em DR-38.
  - Testes: (a) cotar a largura de uma placa, depois mudar a largura 100→120: a cota mostra 120 após atualizar; (b) apagar a aresta cotada: cota vermelha, não "100"; (c) cota de Ø em furo facetado dá exatamente o Ø do modelo; (d) cota de referência `(120)` não dirige nada; (e) desfazer/refazer preserva a âncora.

### 2.22 Importar e gerar cotas do modelo
(CAD avançado B: Generate Dimensions / CAD avançado A: Model Items, Auto Dimension / CAD avançado C: Retrieve Model Annotations / CAD avançado H: Retrieve Dimensions / Fenkra: ausente)

- **O que faz.** Traz para o desenho as cotas e restrições que o projetista já escreveu no 3D.
- **CAD avançado B.**
  - [V] *Generate dimensions* em um passo ou semi-automático (passo a passo), usando restrições de distância, comprimento, ângulo, raio e diâmetro. Filtros em diálogo: restrições do Sketcher, 3D (ex.: offset entre planos), de produto e medidas. Para vistas de superfícies, só restrições de croqui geram cotas.
  - [V] Opções: `Filters before generation`, `Analysis after generation`; a análise realça as restrições e as cotas geradas.
  - [V] Só gera cotas de elementos 3D **visíveis** na vista; cotas de referência podem não ser criadas (nota IBM).
  - [V] O usuário pode **dirigir** a restrição 3D pela cota gerada (opção administrativa para impedir).
  - [V] Cota de rosca associativa (`Thread Dimension`): na vista frontal dá o diâmetro, no corte dá diâmetro e profundidade.
- **CAD avançado A.**
  - [V] *Model Items*: Origem Entire model ou Selected feature; filtros "Marked for drawing"/"Not marked"; `Eliminate duplicates`; "Import items into all views"; "Include items from hidden features". Para montagens: componente selecionado ou só montagem.
  - [V] Os blogs avisam que as cotas inseridas podem chegar em posições soltas e desalinhadas.
  - [V] *Autodimension* no desenho: esquemas horizontal e vertical de **linha de base, cadeia ou ordenada**, origem escolhível (padrão: a primeira entidade em x0,y0). *cotas automáticas Auto Dimension Scheme* (hoje MBD Dimension) restringe completamente features de manufatura contra datums.
- **CAD avançado C.**
  - [V] *Retrieve Model Annotations* (aba Annotate): escolhe a vista e quais cotas manter. Abas "Sketch and Feature Dimension" (Select Source/Features/Parts) e "3D Annotations" (filtros: cotas gerais, tolerância, texto de chamada, nota de furo e rosca, textura, nota geral; escolha de Design View).
  - [V] Só cotas **paralelas ao plano da vista**; editá-las muda a peça. Uma cota de modelo não aparece em duas vistas da mesma folha. Opção de preferências para recuperar todas ao colocar a vista; erros relatados por usuários.
- **CAD avançado H.** [V] *Retrieve Dimensions* traz as cotas do usuário normais à vista; costuma exigir limpeza.
- **Fenkra hoje.** Ausente. As cotas do modelo (de croqui) não vão para o desenho; só PMI (via `Note` ancorada) e cotas manuais.
- **Melhor prática.**
  - Fluxo: vista → fonte (croqui, feature, PMI) → filtros (só visíveis na vista, só paralelas ao plano, eliminar duplicatas) → pré-visualização com seleção → aplicar, com layout automático (empilhamento por afastamento crescente).
  - Cotas importadas guardam o **identificador do parâmetro** (`param: ParamKey`) e o valor mostrado vem do parâmetro. Modo "dirigir" (editar a cota muda a peça) opcional e **desligado por padrão**.
  - Testes: (a) importar de um croqui retangular dá 2 cotas e não duplica ao repetir; (b) mudar o parâmetro `largura` no 3D atualiza a cota; (c) cota de furo cego só aparece na vista lateral se o furo estiver visível lá.

### 2.23 Cotas ordenadas, linha de base e cadeia
(CAD avançado B: Cumulated/Stacked/Coordinate / CAD avançado A: Ordinate/Baseline/Chain / CAD avançado C: Baseline, Ordinate (Set) / Fenkra: ausente)

- **O que faz.** Cotagem a partir de uma origem comum (ordenada e linha de base) ou de ponta a ponta (cadeia).
- **CAD avançado B.** [V] Os comandos existem (Cumulated, Stacked, Coordinate, Chamfer) no guia de cotagem interativo e nos manuais Wichita.
- **CAD avançado A.** [V] Autodimension: horizontal e vertical, com tipo baseline/chain/ordinate, origem escolhível, posição acima ou abaixo (horizontal) e esquerda ou direita (vertical).
- **CAD avançado C.** [V] *Ordinate Dimension Set* cria várias de uma vez e as alinha sozinhas; *Baseline Dimension Set* usa a primeira seleção como linha de base. O marcador de origem é colocado na primeira ordenada.
- **Fusion.** [V] Ordinate, Baseline e Chain, mais *Dimension Break*.
- **CAD avançado G.** [V] Preferência para mostrar ou não o zero na linha de base de cada ordenada; rótulo da origem editável.
- **Fenkra hoje.** Ausente.
- **Melhor prática.** Cota-conjunto `DimSet {kind: Ordinate|Baseline|Chain, origin: Anchor, items: Vec<Anchor>, dir}`. Arrastar a origem recalcula todas. Texto da ordenada com valor relativo à origem, sem seta; linha de chamada com cotovelo. Testes: (a) conjunto ordenado de 5 furos: valores corretos e sem texto sobreposto; (b) mover a origem muda todos os valores; (c) remover um item não quebra os outros.

### 2.24 Cota de chanfro
(CAD avançado B: Chamfer Dimension / CAD avançado A: Chamfer Dimension / CAD avançado C: Chamfer Note / Fenkra: ausente)

- **O que faz.** Cota um chanfro com distância e ângulo (ex.: `1,5 × 45°`), com chamada.
- **CAD avançado B.** [V] O comando de cota de chanfro existe (Wichita). [C] Valor e formato vêm do padrão (x). [C]
- **CAD avançado A.** [V] Existe, mas a busca não trouxe seus detalhes. [C] Marca-se a aresta do chanfro e outra aresta; formato `d × ângulo` conforme padrão.
- **CAD avançado C.** [V] *Chamfer Notes*: distância e ângulo para arestas de modelo ou linhas de croqui, anexáveis a arestas inclinadas em vistas e croquis.
- **Fenkra hoje.** Ausente. O chanfro existe como feature de Sólidos, mas o desenho não o conhece.
- **Melhor prática.** Ler distância e ângulo da **feature Chanfro** (parâmetros) e não medir a aresta. Formatos: `C1,5` (ISO 45°), `1,5×45°`, `1,5×2` (assimétrico). Ancorar na aresta do chanfro. Testes: (a) chanfro 2×45 → `2×45°`; (b) mudar para 3 atualiza; (c) feature suprimida → nota vermelha.

### 2.25 Chamada de furo e rosca (hole callout)
(CAD avançado B: Thread Dimension / CAD avançado A: Hole Callout / CAD avançado C: Hole and Thread Notes / Fenkra: ausente)

- **O que faz.** Mostra `Ø8,5 ↧ 20 / M10×1,5 – 6H` lendo da feature de furo.
- **CAD avançado B.** [V] *Thread Dimension* associativa; só gera rosca definida em furos 3D.
- **CAD avançado A.** [V] *Hole Callout* via Model Items (assistente de furos Locations → Hole Callout) [V: GoEngineer]. [C] Lê Ø, profundidade, rosca, escareado/rebaixo e mostra no formato do padrão.
- **CAD avançado C.**
  - [V] *Hole and Thread* acrescenta nota com chamada, lendo diâmetro, profundidade e rosca do modelo. O formato vem do **estilo de cota** (aba Notes and Leaders), definido por tipo de furo. Pode-se trocar o estilo de uma nota, editar uma nota (não muda o estilo) ou editar o estilo (muda todas).
  - [V] Cobre furos, cortes extrudados (exceto plano médio), furos de iFeature, padrões e planificadas de chapa. Tem unidade dupla.
  - [V] Se a aresta circular some no update, a nota se reanexa à aresta circular mais externa do furo; em vista lateral, à aresta mais próxima do ponto original. Não vale para vista em fatia.
- **Fenkra hoje.** Ausente. O que existe: cota de Ø com texto livre `4x <> passante` (`dwg_ui.rs:1498`) e `hole_table` (tabela). O dado do furo (`fenkra_solidos::hole_table`) já existe.
- **Melhor prática.**
  - `Note::HoleCallout {hole: Ref, fmt}` com formato por tipo (passante, cego, rosca, escareado, rebaixo, oblongo) vindo da **norma** (2.3) e editável. Contagem automática `4×` para padrão.
  - No update: resolve a chave do furo; se a aresta somar, reanexa à mais externa (regra do CAD avançado C), senão vermelho.
  - Testes: (a) furo M10 passante gera `M10 ▽ passante` (ou o formato da norma); (b) padrão de 4 furos gera `4×`; (c) trocar o furo para cego atualiza a profundidade; (d) rosca fina `M10×1` correta.

### 2.26 Tolerâncias e ajustes na cota
(CAD avançado B: Dimension Properties / CAD avançado A: Tolerance/Precision / CAD avançado C: Edit Dimension / Fenkra: `tol` texto)

- **O que faz.** Mostra tolerância simétrica, limites, ajuste ISO (`H7`, `g6`) e precisão da cota.
- **Fontes.** [V] CAD avançado C: texto sobrescrito, precisão, tolerância e unidade alternativa por cota. [C] CAD avançado A/CAD avançado B: tipos Simétrica, Bilateral, Limites, Ajuste, Básica; ajustes com desvios calculados das tabelas ISO 286.
- **Fenkra hoje.** `tol: String` concatenado ao texto (`layout.rs:290`); botões rápidos `±0,1`, `+0,1/0`, `H7`, `h6`, `g6` (`dwg_ui.rs:1504`). Não calcula os desvios do ajuste, não empilha, não há precisão por cota.
- **Melhor prática.** `Tolerance {kind, upper, lower, fit: Option<ISOFit>, stacked}`. Ajuste ISO 286 calcula desvios pelo diâmetro nominal [C]. Modo "mostrar ajuste", "mostrar desvios" e "os dois". Testes: (a) Ø20 H7 mostra +0,021/0; (b) limites mostram `20,021 / 20,000`; (c) precisão por cota independe do desenho.

### 2.27 Notas, linhas de centro e marcas de centro
(CAD avançado B: Text, Axis line, Center line, Thread / CAD avançado A: Note, Centerline, Center Mark / CAD avançado C: Center Mark, Centerline, Text / Fusion: Center Line, Center Mark Pattern / Fenkra: `Text`, `CenterMark`)

- **O que faz.** Texto livre com chamada, marcas e eixos que mostram a simetria de furos e cilindros.
- **CAD avançado B.** [V] Centros e eixos são elementos de acabamento (*dress-up*) ligados à geometria: ao isolar, centros ficam presos a um ponto e eixos a dois pontos.
- **CAD avançado C.** [V] Marcas de centro (cruz em arcos/círculos, extensão ajustada); linhas de centro: bissetriz, padrão centrado ou axial, e circulares para padrões circulares. Texto e Leader Text: o texto com chamada anda e se apaga com a vista.
- **Fusion.** [V] Center Line, Center Mark, Center Mark Pattern, Edge Extension.
- **Fenkra hoje.** `CenterMark` é uma nota com "raio da marca" digitado (`dwg_ui.rs:987`); só marca. Não há centro automático para furos, nem eixo de cilindros. Notas dentro de vista ficam presas à vista (`view_at`).
- **Melhor prática.** Comando "Centros automáticos" por vista: marca em todo círculo/arco inteiro; eixo em todo cilindro/cone visto de lado; padrão circular dos furos. Elementos associativos (chave da feature). Eixos em camada própria. Testes: (a) vista de placa com 4 furos: 4 marcas; (b) vista lateral de eixo: linha de centro passando pelo eixo, saindo ~3 mm além; (c) padrão circular desenha o círculo de centros.

---

### 2.28 Rugosidade (acabamento superficial)
(CAD avançado B: Roughness / CAD avançado A: Surface Finish / CAD avançado C: Surface Texture / Fusion: Surface Texture / Fenkra: `NoteKind::Roughness`)

- **O que faz.** Símbolo de textura (ISO 21920/1302, ASME Y14.36 [C]) preso a uma aresta/face.
- **Fontes.** [V] CAD avançado C: *Surface Texture* entre os símbolos, com ou sem chamada; também recuperável de anotações 3D (filtro "Surface Texture"). [V] Fusion: Surface Texture.
- **Fenkra hoje.** Nota com texto `Ra 1,6` (`dwg_ui.rs:1012`); símbolo desenhado em `note()` (`layout.rs:1007`). Ponta presa à vista com `anchor3` (acompanha a peça, mas não a aresta, 2.21). Sem valores estruturados (Ra/Rz, processo, direção de usinagem, comprimento de amostragem, "todas as superfícies").
- **Melhor prática.** Campos estruturados (processo, Ra/Rz, corte-off, direção, sobremetal) como PMI; símbolo "geral" no canto, como `(✓)`. Testes: (a) símbolo ligado a uma face acompanha a mudança de posição da face; (b) símbolo geral no canto da folha.

### 2.29 Solda
(CAD avançado B: Weld / CAD avançado A: Weld Symbol, Weld Table / CAD avançado C: Welding symbol, Caterpillar, End Fill / Fenkra: `NoteKind::Weld`)

- **O que faz.** Símbolos ISO 2553/AWS A2.4 [C], e tabela/lagarta de solda.
- **CAD avançado A.** [V] *Weld Table*: quantidade, tamanho, símbolo, comprimento e propriedades do cordão; preenche automaticamente se o cordão for modelado; se só houver símbolos nas vistas, uma opção inclui as anotações do desenho.
- **CAD avançado C.** [V] Símbolos, *Caterpillar* (símbolo do cordão em 2D, manual ou automático das features de solda) e *End Fill*.
- **Fenkra hoje.** `DwgWeld {kind, all_around, extra}` (`lib.rs:420`), símbolo ISO 2553 completo no desenho e PMI (catálogo). Sem cordão 3D, sem tabela de solda, sem lagarta.
- **Melhor prática.** Quando existir cordão 3D (B-solda), gerar símbolo e tabela dele; hoje, tabela a partir das notas de solda do desenho. Testes: (a) 3 símbolos de filete → linha agrupada com quantidade e comprimento; (b) all-around mostra o círculo.

### 2.30 GD&T e referências (datums)
(CAD avançado B: Geometrical Tolerance, Datum Feature / CAD avançado A: Geometric Tolerance, Datum / CAD avançado C: Feature Control Frame, Datum Identifier / Fenkra: `Gdt`, `Datum`)

- **O que faz.** Quadros de controle (ISO 1101, ASME Y14.5 [C]) e identificadores de referência.
- **Fontes.** [V] CAD avançado D e CAD avançado C oferecem Feature Control Frame, Datum Identifier/Feature e Datum Target. [V] CAD avançado C recupera tolerâncias das anotações 3D (filtro "Tolerance Feature").
- **Fenkra hoje.** Quadro completo reaproveitado do PMI (`DwgGdt`, `crate::pmi_ui::gdt_ui`), letra de datum com próxima letra livre automática (`dwg_ui.rs:1016`). Ponta presa à vista por `anchor3` (teste existe, `lib.rs:801`).
- **Falta.** Validar que a letra citada no quadro existe como datum na folha (aviso se não). Alvos de datum (datum targets). Moldura de composição. Associar ao PMI 3D (ver 2.40).
- **Melhor prática.** Verificação: letra usada no quadro sem datum correspondente → aviso amarelo; datum duplicado → vermelho. Testes: (a) apagar o datum A deixa o quadro "A" com aviso; (b) letra I, O, Q evitadas (ASME) [C].

### 2.31 Hachuras (seção e área)
(CAD avançado B: Area Fill + material / CAD avançado A: Area Hatch/Fill, Material Crosshatch / CAD avançado C: Hatch / Fenkra: `hatch`)

- **O que faz.** Preenche as regiões cortadas segundo o material (padrão, ângulo, espaçamento) e permite hachurar áreas.
- **CAD avançado B.** [V] *Area Fill*: região fechada com hachura, pontilhado ou cor, definida automaticamente ou por perfil. Se o espaçamento for maior que a região, a hachura não aparece e o contorno é engrossado. [V] O padrão padrão do Area Fill fica na norma, mas vistas geradas não o herdam; a saída é o material do corpo. [V] Propriedades da vista permitem mudar o padrão e atributos gráficos do corte. [V] Há registro de hachura em regiões indevidas por folga maior que a tolerância (corrigido marcando "Imported from external format").
- **CAD avançado A.**
  - [V] *Material crosshatch* pega padrão, escala e ângulo do material da peça.
  - [V] *Apply to*: Component, Region, View ou Body (em cortes). *Auto hatching* alterna o ângulo entre faces vizinhas. *Randomize scale* randomiza a escala por material.
  - [V] Padrão, escala e ângulo manuais; padrões personalizados em `.ptn`.
  - [V] No detalhe de corte, `Scale hatch pattern` usa a escala do detalhe.
- **CAD avançado C.** [V] A hachura, rótulo e linhas de corte saem automáticos no corte; a herança do corte é configurável.
- **Fenkra hoje.** `layout::hatch(loops, 2.5)` (`layout.rs:622`, uso em `342`): 45° fixo, passo 2,5 mm, regra par/ímpar. Contorno da seção em linha visível; hachura em linha fina. Sem padrão por material, sem ângulo, sem alternância em montagem (todas as peças com o mesmo ângulo → seções vizinhas ficam ilegíveis), sem hachura de área livre.
- **Melhor prática.**
  - `Hatch {pattern, angle, spacing, color, line}`, vindo do **material** da peça (propriedade no modelo) ou da norma, com sobrescrita na vista e por região.
  - Em montagem: ângulos alternados (45°, 135°, 45° com passo maior) e `randomize` opcional.
  - Hachura se desliga abaixo de um limite de espaçamento (como no CAD avançado B) e vira preenchimento sólido (para chapas finas, [C] ISO/ASME).
  - Padrões ISO 128-50 (metal, plástico, madeira…) [C].
  - Testes: (a) corte de montagem com 2 peças vizinhas: ângulos diferentes; (b) alterar o passo na vista muda só essa vista; (c) a hachura não passa por dentro de furos; (d) peça fina (< 1 mm no papel) vira sólido.

---

### 2.32 Lista de materiais (BOM / lista de peças)
(CAD avançado B: Bill of Material / CAD avançado A: BOM Table / CAD avançado C: Parts List / Fusion: Table (parts list) / Fenkra: `TableKind::Bom`)

- **O que faz.** Lista itens, códigos, descrições, materiais e quantidades da montagem.
- **CAD avançado B.** [V] Inserida no fundo ou na vista de trabalho; as células são associadas aos atributos da estrutura do produto; renomear o número de peça e atualizar o desenho troca o nome na lista; adicionar ou remover produto e atualizar altera as linhas. O conteúdo vem do relatório de listagem da estrutura de produto; aparência (largura de colunas) por arquivo `CATAsmBOM.CATRsc`.
- **CAD avançado A.** [V] Altamente configurável (propriedades mostradas, fontes), salva como modelo de tabela e como Excel (para ERP/MRP).
- **CAD avançado C.** [V] Tabelas de dados da BOM da montagem, com colunas e valores sobrescritos, e itens personalizados (tinta, graxa). Tabelas gerais importam `.xls`, `.xlsx`, `.csv`.
- **Fusion.** [V] *Table* cria a lista de peças e `Renumber`, `Align Balloon` mantêm balões e lista coerentes. Exporta CSV.
- **Fenkra hoje.**
  - 5 colunas fixas: Item, Código, Descrição, Material, Qtd. (`source.rs:432`). Nível Topo/Peças/Estruturada (`BomLevel`).
  - Os números de item vêm da montagem e ficam fixos entre revisões.
  - Exporta CSV pelo painel (`dwg_ui.rs:1617`).
  - Sem colunas configuráveis, ordenação, itens custom (linha manual), massa por linha, nem divisão da tabela em duas colunas quando alta.
  - A tabela é por origem (`Source`), não vinculada a uma vista. Em BOM de peça (`Model::Part`), mostra 1 linha.
- **Melhor prática.** `Table.columns: Vec<ColSpec>` (propriedade, título, largura, alinhamento) e modelo reutilizável; linhas manuais; ordenação; quebra automática na altura da folha; massa por item e total. Números de item **estáveis** (já existe) com política de reuso. Testes: (a) adicionar um componente gera a linha nova depois de Atualizar; (b) mover a coluna "Qtd." salva no modelo; (c) a BOM em duas folhas continua com os mesmos números nos balões.

### 2.33 Balões
(CAD avançado B: Generate Balloons / CAD avançado A: AutoBalloon / CAD avançado C: Balloons / Fusion: Balloon, Renumber, Align / Fenkra: `NoteKind::Balloon`, `auto_balloons`)

- **O que faz.** Numera os componentes na vista e liga cada balão a um item da lista.
- **CAD avançado B.** [V] *Generate Balloons* na vista ativa coloca os balões criados antes no produto. Edição: tamanho de fonte e posição. A página não diz se ficam associativos nem como numera.
- **CAD avançado A.**
  - [V] Layouts Square, Circular, Top; linhas magnéticas (inseridas automaticamente quando o padrão não é circular); `Order sequentially` com início e incremento; `Ignore multiple instances` (um balão só por componente com várias instâncias).
  - [C] Balões empilhados existem; não confirmei.
- **CAD avançado C.** [V] Um por componente, ou todos de uma vista; vários balões podem compartilhar uma chamada.
- **Fusion.** [V] `Renumber` e `Align Balloon`.
- **Fenkra hoje.**
  - `auto_balloons` (`dwg_ui.rs:374`): usa a **primeira vista de montagem da folha**. Calcula o centro da bounding box de cada peça (`dwg_ui.rs:407`) e põe o balão ao lado, no lado mais próximo (altura `tip.y + 6·(k%3)`).
  - Balão manual numera por contagem de balões existentes (`dwg_ui.rs:1012`), **sem ligação com a BOM**: o texto é string.
  - Ponta presa ao modelo por `anchor3`, pelo centro da bbox (não pela superfície visível: a seta pode cair em outro componente ou no vazio; bug provável em montagens com peças ocas ou aninhadas).
  - Balões automáticos não evitam sobreposição entre si nem com cotas.
- **Melhor prática.**
  - Balão guarda `item_ref` (chave do componente), não texto; o número vem da BOM. Se a BOM renumera, o balão acompanha.
  - Ponta num ponto **visível** da peça na vista (raycast na projeção ou um ponto da silhueta interior), não no centro da bbox.
  - Layout: círculo ou retângulo em volta da vista com linhas magnéticas, sem cruzar chamadas.
  - `Renumber` e `Align`.
  - Testes: (a) balão de componente removido fica vermelho; (b) trocar o item na BOM atualiza o balão; (c) duas instâncias do mesmo parafuso geram 1 balão com `×2`; (d) nenhuma ponta cai fora do contorno de peça.

### 2.34 Tabela de furos e tags
(CAD avançado B: — / CAD avançado A: Hole Table / CAD avançado C: Hole Table / CAD avançado F: Hole Table / Fenkra: `TableKind::Holes`)

- **O que faz.** Lista a posição (X, Y a partir de uma origem) e o tamanho dos furos, com tag em cada furo na vista.
- **CAD avançado A.** [V] Origem datum escolhida ao inserir (vértice ou eixos X e Y); pode ser movida depois e a tabela se atualiza. Tags alfabéticas ou numéricas, células iguais mescláveis. Ordenação por coluna. Seleção por arestas ou por face (todos os loops). Furos cegos da face oposta com "FAR SIDE" (precisa de vista em wireframe/ocultas). Uma tabela pode ter várias vistas, com um datum por vista. Editar oculta indicador de origem, centros e tags.
- **CAD avançado C.** [V] Furos escolhidos individualmente, de uma feature ou de toda a vista; inclui cortes extrudados, iFeatures, padrões, marcas de centro e planificadas. Origem presa à vista, criada na primeira tabela (oculta, mas não apagável). Tags com autoindexação alfanumérica (caracteres excluíveis). Colunas e unidades vêm do estilo. Atualiza com o desenho. Pode virar tabela de punção.
- **CAD avançado F.** [V] "Hole Tables" têm tópico próprio na ajuda do fornecedor de CAD avançado.
- **Fenkra hoje.** `fenkra_solidos::hole_table(doc)` (`source.rs:459`): colunas Furo (A1…), X, Y, Z, Ø, Prof., Rosca. Coordenadas do **modelo**, não da vista. Só funciona com peça (`Model::Part`); com montagem mostra "escolha uma peça" (`source.rs:486`). **Não há tag nas vistas**: a letra A1 existe na tabela mas não aparece na vista, então a tabela não casa com o desenho.
- **Melhor prática.** Tabela por **vista** (origem e eixos da vista); tags colocadas automaticamente nos furos visíveis; origem arrastável; colunas X, Y, Ø/rosca, profundidade, quantidade; agrupar por tipo (`A (4×)`). Testes: (a) 4 furos iguais → 1 linha "A ×4" (opção) ou 4 linhas, com 4 tags; (b) mover a origem muda X/Y; (c) furo suprimido some da tabela e da vista; (d) tag em furo cego da face oposta = "FAR SIDE".

### 2.35 Tabela de dobras e de punção
(CAD avançado B: — / CAD avançado A: Bend Table, Punch Table / CAD avançado C: Bend Table, Punch Notes / Fusion: Bend Identifier / Fenkra: `TableKind::Bends`)

- **CAD avançado A.** [V] Bend Table substitui os rótulos por tabela com direção, ângulo e raio. *Punch Table* é parecida com a de furos, mas para formas, mostrando posição na planificada, ID, quantidade e ângulo do eixo X com a ferramenta.
- **CAD avançado C.** [V] Pelo comando General Table; as etiquetas de dobra são colocadas na vista; pode-se renumerar e inverter sentido. Não confundir com as "bend tables" de material para cálculo de desenvolvimento. [V] Notas de dobra, rolagem de contorno e linhas de centro cosméticas na planificada.
- **Fenkra hoje.** `bend_table` com Dobra, Ângulo, Raio, Desenvolvido, Dedução (`source.rs:467`). Rótulos "↑ 90° R2" na planificada (`source.rs:392`). Sem sentido (cima/baixo) na tabela, sem ID igual ao rótulo da vista, sem punção.
- **Melhor prática.** Rótulo da vista = nome na tabela (B1…); coluna Sentido (cima/baixo) e K-factor usado; tabela de punção quando existir feature de forma. Testes: ver 2.18.

### 2.36 Tabela de revisões
(CAD avançado B: Revision block (carimbo) / CAD avançado A: Revision Table / CAD avançado C: Revision Table & Tags / Fenkra: ausente)

- **O que faz.** Registra descrição da mudança, data, quem mudou e o símbolo da revisão; liga ao carimbo.
- **CAD avançado A.** [V] Representa a revisão mais recente, mostra descrição, data, autor e símbolo, e pode atualizar o bloco de revisão do carimbo. Tabelas de bloco de título ligadas às propriedades por configuração.
- **CAD avançado C.** [V] Para o desenho todo ou uma folha; *Revision Tags* marcam objetos alterados e, por padrão, usam o nível de revisão mais recente.
- **Fenkra hoje.** Só `TitleBlock.revision` (uma letra, `lib.rs:473`) e o campo `{revisao}`. Sem tabela, sem histórico.
- **Melhor prática.** `RevisionTable {rows: Vec<{rev, descr, data, autor, aprovado}>}` com a última linha alimentando `{revisao}`; tags de revisão (triângulo com letra) ligadas a elementos. Integra com PLM (contexto `plm-vpm.md`). Testes: (a) adicionar revisão B atualiza o carimbo; (b) tag de revisão acompanha a vista; (c) exportar PDF mantém a tabela.

### 2.37 Lista de corte e desenho de estrutura soldada
(CAD avançado B: — / CAD avançado A: Weldment Cut List, body views / CAD avançado C: Frame drawings / Fenkra: `TableKind::CutList`, B26/B50)

- **O que faz.** Lista perfis, comprimentos e quantidades de uma estrutura soldada, com balões por item e vistas por corpo.
- **CAD avançado A.** [V] *Weldment Cut List* funciona como BOM, mostra comprimentos de corte de perfis estruturais, com modelo e exportação Excel, e só vale se a peça for soldada.
- **Fenkra hoje.**
  - Lista de corte de peça (`fenkra_solidos::cut_list`) e de montagem (acumulando por perfil+comprimento, `source.rs:435`). Itens com o mesmo número do grupo estrutural; teste `lista_de_corte_usa_o_numero_do_item…`.
  - Balões por item no centro do primeiro membro (`struct_balloons`, `dwg_ui.rs:350`).
  - Falta (B50): vista própria por corpo/item, detalhe de ponta (corte em meia-esquadria), cota de comprimento de corte, plano de corte a plasma/laser (DR-33).
- **Melhor prática.** `ViewKind::Body{body: Ref}` (isola o corpo), cota de comprimento herdada da lista; vista de ponta (detalhe) com ângulo de corte. Testes: ver B50 e os do 2.32.

---

### 2.38 Exportação PDF
(CAD avançado B: Save As PDF / CAD avançado A: Save As PDF / CAD avançado C: Export PDF / Fusion: Export PDF / Fenkra: `export::pdf`)

- **O que faz.** Gera PDF vetorial multi-folha com escala correta.
- **Fenkra hoje.** PDF 1.4 com todas as folhas (`export.rs:179`, `266`), texto em WinAnsi (`pdf_str`, `export.rs:158`; fora do Latin-1 vira `?`). Fontes não incorporadas (Helvetica padrão). Teste confere cabeçalho `%PDF-1.4` e `%%EOF`.
- **Falta.** Escolha de folhas, camadas/marcadores (bookmarks) por folha, texto pesquisável com fontes incorporadas (Unicode), cor/espessura por estilo de impressão (CTB) e conferência de escala 1:1 no papel. Metadados (título, autor, revisão).
- **Melhor prática.** Exportar com opção "folhas selecionadas", marcadores por nome de folha, metadados do carimbo e teste de medida: um quadrado de 100 mm no desenho mede 100 mm no PDF (≈283,46 pt). Testes: (a) PDF abre e tem N páginas; (b) caracteres `Ø ° ± ≥` aparecem; (c) A3 tem MediaBox 1190,55×841,89 pt.

### 2.39 Exportação DXF e DWG
(CAD avançado B: Save As DXF/DWG / CAD avançado A: Save As DXF/DWG com mapeamento / CAD avançado C: Export / Fusion: Export DWG, Export Sheet as DXF / Fenkra: `export::dxf`)

- **O que faz.** Entrega o desenho a quem usa CAD avançado J, a corte a laser/plasma e a CAM.
- **CAD avançado A.**
  - [V] O diálogo de mapeamento define camadas, cores e estilos de linha de saída; arquivos de mapa (`Load/Save Map File`); "Only map entities not in CAD avançado A layers"; "Don't show mapping on each save".
  - [V] `Scale output 1:1` (só desenhos), opção de exportar splines como polilinhas.
  - [V] Para chapa há mapeamento das linhas de dobra para camadas (blog Javelin).
- **Fusion.** [V] Tem `Export DWG`, `Export Sheet as DXF` e `Export CSV` das tabelas.
- **Fenkra hoje.** DXF R12 ASCII (`export.rs:52`):
  - Camadas fixas: VISIVEL, FINA, OCULTA, CENTRO, MOLDURA.
  - Tudo vira `LINE`; arcos e círculos viram segmentos (polilinhas facetadas).
  - Setas são `SOLID`; textos na camada FINA, sem estilo de texto.
  - Cotas, hachuras, tabelas e símbolos são **explodidos**: o usuário de CAD avançado J não consegue editar uma cota.
  - Sem DWG.
  - Sem escala 1:1 ao modelo (a unidade é a do papel).
  - Bugs prováveis: círculos facetados com poucos segmentos; texto girado e centralizado com `72` e `11` sem `73`; sem `$INSUNITS` nem `$MEASUREMENT`; unidade ambígua para quem importa.
- **Melhor prática.**
  - Passar de R12 a **DXF R2000+** (ou usar uma biblioteca) para ter `ARC`, `CIRCLE`, `LWPOLYLINE`, `HATCH`, `DIMENSION` (opcional, com fallback explodido), `MTEXT`, `$INSUNITS = 4` (mm).
  - Camadas por norma (ISO 13567 [C]) configuráveis por arquivo de mapa.
  - DWG: não há formato aberto; opções são o conversor ODA File Converter externo ou biblioteca licenciada [C]; manter DXF como saída principal e DWG via conversor como opcional, com aviso de licença.
  - Modo "1:1 do modelo": exporta a vista na escala 1:1 (a planificada para corte).
  - Testes: (a) reimportar o DXF no LibreCAD/fornecedor de CAD avançado Viewer e medir um furo de Ø10: 10,000 ± 0,001; (b) arcos continuam arcos; (c) a camada de dobra existe só na planificada; (d) acentos e `Ø` corretos.

### 2.40 Vistas geradas a partir de PMI (MBD → desenho)
(CAD avançado B: FD&T View / View From 3D / CAD avançado A: 3D View → drawing, Insert Model Items / CAD avançado G: PMI in drawing / CAD avançado H: Include PMI dimensions / Fenkra: ausente; PMI 3D existe)

- **O que faz.** Cria no desenho a vista com as anotações (PMI) de uma captura 3D, sem redigitar.
- **CAD avançado B.**
  - [V] *View From 3D* (barra Views, submenu Projections): escolhe-se uma vista 3D (na árvore ou na peça) e o sistema pré-visualiza a vista com anotações; clica-se na folha para criar.
  - [V] Pré-requisito: a norma do CATDrawing e do CATPart tem que ser a mesma, senão a vista e as anotações podem não ser geradas. Ativar o comando Hide/Show de anotações 3D.
  - [V] As anotações podem ser movidas e editadas, mas **não são associativas** à geometria: *Force Update* atualiza a vista e **descarta** as edições; a recomendação é mudar no 3D.
  - [V] Vistas FD&T **não giram** (Angle desabilitado).
  - [V] Escala da vista 2D é o inverso da razão da vista 3D (registro IBM).
  - [V] Planos de anotação e capturas são tópicos próprios do curso FTA.
- **CAD avançado A.**
  - [V] Desde 2018, uma vista 3D capturada no MBD vai para a folha arrastando-a da *View Palette*; nas opções de importação traz as anotações cotas automáticas/MBD e as ortogonais à vista. As vistas capturadas não são editáveis. O cotas automáticas foi renomeado MBD Dimension em 2019.
- **CAD avançado C.** [V] *Retrieve Model Annotations > 3D Annotations*: filtros (cotas, tolerâncias, notas de furo, textura, nota geral) e escolha de *Design View*. Anotações 3D de um único componente podem ser recuperadas em vistas de montagem.
- **CAD avançado H.** [V] Opção "Include PMI Dimensions from model views" ao colocar a vista.
- **CAD avançado G.** [V] PMI no desenho depende da preferência de rastreio de localização; há registro antigo de que PMI não aparece em vistas de corte ou interrompidas.
- **Fenkra hoje.** Há PMI 3D (cotas, GD&T, rugosidade, solda, capturas — catálogo PMI-01..03). O desenho não consome as capturas. `Note.anchor3` existe, mas só para anotações do próprio desenho.
- **Melhor prática.**
  - `ViewKind::FromCapture{capture: Ref}`: a vista nasce da captura (câmera + PMI visível), e as anotações entram como **referências vivas** ao PMI (não cópias). Editar no 3D atualiza; editar no desenho só sobrescreve a posição (como o CAD avançado B recomenda, mas sem descartar ao atualizar).
  - Importar também em vistas projetadas, com filtro "paralelas ao plano" (CAD avançado C).
  - Anotação cujo plano não é paralelo à vista fica fora, com contador.
  - Testes: (a) capturar a vista Frente com 3 cotas PMI, criar a vista do desenho: 3 cotas aparecem; (b) mudar uma cota PMI atualiza o desenho; (c) apagar o PMI deixa a anotação do desenho vermelha; (d) a posição manual sobrevive a atualizar.

---

## 3. Requisitos levantados

Prioridade: P0 = bloqueia a promessa "o desenho acompanha o 3D"; P1 = esperado de um CAD profissional; P2 = produtividade; P3 = diferencial.

### Funcionais (RF-DES)

| ID | Requisito | Prio | Critério de aceite testável |
|---|---|---|---|
| RF-DES-01 | Cota guarda âncora estável (chave de vértice/aresta + fallback), não coordenada solta | P0 | Cotar a largura de uma placa 100 mm; mudar para 120 e atualizar → a cota mostra 120 e a ponta fica na aresta |
| RF-DES-02 | Cota cuja referência some fica `ReferênciaQuebrada` (vermelha) e nunca escolhe outra geometria | P0 | Suprimir a feature cotada → a cota fica vermelha com ação Reatribuir; o valor antigo não é exibido como se fosse atual |
| RF-DES-03 | Estado por vista/elemento: Atualizado, Desatualizado, Erro, Quebrado, Travado, Isolado | P0 | Alterar o arquivo-fonte fora do app → a vista aparece Desatualizada ao reabrir/focar, sem recalcular em silêncio |
| RF-DES-04 | Comando Atualizar (folha ativa / todas) e chave Atualização automática por desenho | P0 | Com automático desligado, o 3D muda e o desenho não; Atualizar aplica; o contador de pendências zera |
| RF-DES-05 | Travar vista (não atualiza, mantém anotações) e Isolar (corta o vínculo, irreversível com confirmação) | P1 | Vista travada ignora mudança do 3D; isolar pede confirmação e marca cotas como sem associação |
| RF-DES-06 | Painel de diagnóstico do desenho com ações Editar, Reatribuir, Substituir, Isolar, Desativar | P1 | Desenho com 3 problemas lista os 3 com o motivo; "Reatribuir" religa e remove o item |
| RF-DES-07 | Método de projeção por folha (1º/3º diedro) e símbolo correspondente | P0 | Mesmo clique abaixo da frente → Superior em 1º e Inferior em 3º; símbolo e texto da legenda mudam |
| RF-DES-08 | Vista projetada deriva dos eixos da mãe (inclui mãe Section/Camera/Flat) | P0 | Projetar de um corte A-A dá vista com eixo perpendicular ao da mãe e alinhada |
| RF-DES-09 | Objeto Norma de desenho (ISO, ASME, DIN, ABNT) com herança norma → folha → vista → elemento e indicação da origem do valor | P1 | Trocar ISO→ASME muda projeção, setas e tangentes; sobrescrita local sobrevive; "restaurar" volta ao herdado |
| RF-DES-10 | Vista auxiliar a partir de aresta/eixo/face, associativa | P1 | Face a 30° mostra forma verdadeira (área = 3D); mudar o ângulo gira a vista |
| RF-DES-11 | Linha de corte como polilinha; tipo inferido (total, meio, alinhado, desvio) e `Flip` | P1 | Corte de eixo em L: meio corte; 2 segmentos não perpendiculares: alinhado; `Flip` troca o lado visto |
| RF-DES-12 | Profundidade do corte: Total, Distância, Fatia (seção removida) | P2 | Fatia de 0 mm de eixo escalonado mostra só o contorno hachurado |
| RF-DES-13 | Componentes "não cortados" (fixadores, eixos, nervuras) por propriedade e por vista | P1 | Parafuso numa montagem em corte aparece inteiro, sem hachura |
| RF-DES-14 | Vista de corte alinhada ao traço (movimento só na perpendicular) | P1 | Arrastar a vista de corte na direção do traço é bloqueado/ajustado |
| RF-DES-15 | Corte parcial (breakout) com contorno fechado e profundidade (ponto, aresta/eixo, furo, atravessando) | P1 | Breakout até o eixo de um furo cego mostra o furo; profundidade 0 dá erro claro |
| RF-DES-16 | Vista interrompida (horizontal/vertical, estilo, gap) com cota de valor real | P1 | Eixo de 1000 mm com quebra mostra cota 1000; filha herda a quebra |
| RF-DES-17 | Vista recortada (clip) por círculo/polígono, com contagem de cotas ocultadas | P2 | Recortar e desfazer restaura as cotas; contador visível |
| RF-DES-18 | Detalhe: perfil poligonal, estilo da chamada, geometria refinada (não só recorte da mãe) | P1 | Detalhe 10:1 de raio R5 tem arco liso; detalhe de corte mantém a hachura |
| RF-DES-19 | Vista de montagem: estado de exibição por lista, posição alternativa/sobreposição | P2 | Estado excluído da montagem gera erro claro; sobreposição mostra 2 posições |
| RF-DES-20 | Vista explodida com linhas de explosão | P2 | Mudar a distância de explosão atualiza a vista e os balões |
| RF-DES-21 | Planificada: eixo pela face de referência; etiquetas de dobra iguais às da tabela | P1 | Chapa com 4 dobras: 4 rótulos e 4 linhas com os mesmos nomes |
| RF-DES-22 | Rotação e alinhamento opcional da vista (flag `aligned`, ângulo) | P2 | Rotacionar 90° gira as cotas; soltar o alinhamento permite mover livre |
| RF-DES-23 | Cota inteligente: tipo pelo contexto de seleção (aresta → comprimento; círculo → Ø; arestas → distância/ângulo) | P1 | Um clique em círculo cria Ø; duas arestas paralelas, distância |
| RF-DES-24 | Raio/diâmetro lidos da superfície analítica, não da polilinha | P1 | Ø de furo facetado exatamente igual ao Ø do modelo (erro < 1e-6) |
| RF-DES-25 | Cota de referência `(120)` que não dirige nada | P2 | Cota de referência exibe parênteses e muda com o modelo |
| RF-DES-26 | Importar cotas do modelo (croqui, feature, PMI) com filtros, pré-visualização e eliminação de duplicatas | P1 | Importar de croqui retangular dá 2 cotas; repetir não duplica |
| RF-DES-27 | Gerar cotas automáticas (esquema de linha de base/cadeia/ordenada) | P2 | Esquema ordenado de 5 furos sem sobreposição de texto |
| RF-DES-28 | Cotas ordenadas, linha de base e cadeia como conjuntos | P1 | Mover a origem recalcula todas; remover um item não quebra as outras |
| RF-DES-29 | Cota de chanfro lida da feature (`1,5×45°`) | P2 | Mudar o chanfro para 3 mm atualiza a nota; feature suprimida deixa a nota vermelha |
| RF-DES-30 | Chamada de furo e rosca por norma, com contagem `4×` e reancoragem | P1 | Furo M10 passante → `M10 ▽ passante`; padrão de 4 → `4×` |
| RF-DES-31 | Tolerância estruturada (simétrica, limites, ajuste ISO 286) e precisão por cota | P2 | Ø20 H7 mostra +0,021/0; limites mostram 20,021/20,000 |
| RF-DES-32 | Centros automáticos: marcas, eixos de cilindros, padrão circular | P1 | Placa com 4 furos → 4 marcas; eixo lateral com linha de centro saindo ~3 mm |
| RF-DES-33 | Hachura por material/norma (padrão, ângulo, passo), com sobrescrita; alternância em montagem | P1 | Corte de 2 peças vizinhas: ângulos diferentes; peça fina → preenchimento sólido |
| RF-DES-34 | Rugosidade estruturada e símbolo geral | P2 | Símbolo de uma face acompanha a mudança de posição da face |
| RF-DES-35 | Verificação de datums (letra citada sem datum → aviso; duplicado → erro) | P2 | Apagar o datum A deixa o quadro que o cita com aviso |
| RF-DES-36 | BOM com colunas configuráveis, linhas manuais, ordenação, massa e quebra por altura | P1 | Reordenar colunas persiste; tabela alta quebra em duas colunas |
| RF-DES-37 | Balão referencia o item (chave do componente) e a ponta cai em ponto visível da peça | P0 | Trocar a numeração na BOM atualiza o balão; nenhuma ponta cai no vazio |
| RF-DES-38 | Balões automáticos com layout (círculo/retângulo/linhas magnéticas), `Renumber`, `Align` | P2 | Balões em volta da vista sem cruzar chamadas |
| RF-DES-39 | Tabela de furos por vista, com origem arrastável e tags automáticas nos furos | P1 | 4 furos iguais → "A ×4" + 4 tags; mover a origem muda X/Y |
| RF-DES-40 | Tabela de dobras/punção com sentido e ID igual ao da vista | P2 | IDs da vista e da tabela coincidem |
| RF-DES-41 | Tabela de revisões e tags de revisão ligadas ao carimbo | P1 | Adicionar revisão B atualiza `{revisao}` e o carimbo |
| RF-DES-42 | Tabela de solda | P3 | 3 símbolos de filete → linha agrupada com quantidade e comprimento |
| RF-DES-43 | Vista por corpo/item para estrutura soldada, com detalhe de ponta | P1 | Vista do item 2 mostra só o perfil do item 2 com cota de comprimento |
| RF-DES-44 | Fundo de folha editável e reutilizável (formato de folha) | P1 | Salvar o fundo como `.fkf`, aplicar a nova folha e manter células e logotipo |
| RF-DES-45 | Massa do carimbo vem do material do modelo | P1 | Trocar o material muda `{massa}` ao reabrir |
| RF-DES-46 | Vista gerada de captura PMI (MBD→desenho), com referência viva | P1 | Captura com 3 cotas PMI → 3 cotas no desenho; editar no 3D atualiza |
| RF-DES-47 | Exportar PDF (folhas selecionadas, marcadores, fontes incorporadas, metadados) | P2 | PDF A3 com MediaBox 1190,55×841,89 pt; `Ø ° ±` visíveis; marcadores por folha |
| RF-DES-48 | Exportar DXF R2000+ com ARC/CIRCLE/HATCH/MTEXT, camadas mapeáveis e unidades em mm | P1 | Reabrir no visualizador e medir Ø10 = 10,000 ± 0,001; arcos continuam arcos |
| RF-DES-49 | Exportar DXF da planificada em 1:1 com camadas de corte e dobra | P1 | Comprimento desenvolvido medido no DXF = valor da tabela |
| RF-DES-50 | Exportar DWG (conversor externo opcional) | P3 | Arquivo abre no CAD avançado J sem aviso de reparo |

### Não funcionais (RNF-DES)

| ID | Requisito | Prio | Critério de aceite testável |
|---|---|---|---|
| RNF-DES-01 | Projeção (HLR) e atualização fora da thread de interface, canceláveis | P0 | Com peça de 5 000 faces, arrastar a vista não trava (> 30 fps de redesenho) e Esc cancela |
| RNF-DES-02 | Nenhuma atualização silenciosa: toda mudança de geometria vira estado visível | P0 | Teste de regressão: 0 casos em que `proj` recalcula sem alterar o estado da vista |
| RNF-DES-03 | Precisão: valor da cota igual ao valor do modelo (erro < 1e-6 mm) | P0 | Corpus B53: cotas de 16 peças conferem com medidas do kernel |
| RNF-DES-04 | Salvar/abrir reproduz o desenho idêntico (formato versionado e retrocompatível) | P0 | `from_json(to_json(d)) == d` para desenhos do corpus, inclusive versões antigas |
| RNF-DES-05 | Mensagens e rótulos em português (i18n com par em inglês) | P1 | Auditoria: 0 strings novas sem entrada no `i18n.rs` |
| RNF-DES-06 | Toda feature nova entra no corpus de peças de referência com teste nomeado | P1 | Script de cobertura feature → teste lista 0 lacunas para DES |
| RNF-DES-07 | Escala de impressão correta (1:1 no papel) em PDF/SVG | P1 | Quadrado de 100 mm mede 100 mm no PDF |
| RNF-DES-08 | Desempenho de abertura: desenho com 20 vistas abre em < 5 s, usando cache de projeção por assinatura | P2 | Benchmark do corpus abaixo do limite |
| RNF-DES-09 | Acessibilidade de cor: estados também por ícone/texto, não só cor | P2 | Estado "Desatualizado" tem ícone e texto na árvore |
| RNF-DES-10 | Desfazer/refazer cobre toda criação/edição/apagar de desenho, inclusive Atualizar | P1 | Atualizar → Ctrl+Z restaura o estado anterior das vistas |

---

## 4. Itens de backlog propostos

Tamanho: P (≤ 80 k tokens), M (80–200 k), G (> 200 k). Eu defini esses limites; o backlog do projeto usa k de tokens, então ajuste ao calibrar.

| # | Título | O que muda | Prio | Tam. | Dependências |
|---|---|---|---|---|---|
| D1 | **Âncoras estáveis nas cotas** | `Dim.anchors` passa a `Anchor` (chave estável + fallback); resolve no update; estado `ReferênciaQuebrada`; raio/Ø lidos da superfície analítica; testes no corpus | P0 | G | ADR 0010 (referências v2), B53 |
| D2 | **Estado e atualização de vistas** | Estado por vista/elemento, comando Atualizar, atualização automática, cálculo fora da thread, assinatura por vista, painel de diagnóstico, Travar e Isolar | P0 | G | D1, B30 (DR-34), B53 |
| D3 | **Projeção 1º/3º diedro e vistas filhas pelos eixos da mãe** | `Sheet.projection`, tabela lado→eixo, símbolo, correção de projeção a partir de Topo/Section/Camera/Flat | P0 | M | — |
| D4 | **Norma de desenho e herança** | Objeto `DraftStandard` (ISO, ASME, DIN, ABNT), cadeia norma→folha→vista→elemento, origem do valor na UI, presets | P1 | G | D3, B30 (DR-35..43) |
| D5 | **Cortes completos** | Linha de corte como polilinha; meio corte, desvio, alinhado; `Flip`; profundidade (total, distância, fatia); componentes não cortados; vista alinhada ao traço | P1 | G | D3 |
| D6 | **Vista auxiliar e vista recortada** | `Auxiliary` por aresta/eixo/face (associativa); `clip` por círculo/polígono com contador de cotas ocultas | P1 | M | D1 |
| D7 | **Corte parcial e vista interrompida** | Breakout (contorno + profundidade por referência), `breaks` com cota de valor real e propagação | P1 | G | D1, D5 |
| D8 | **Detalhe de qualidade** | Detalhe poligonal, estilo da chamada, geometria refinada, hachura preservada | P1 | M | D5 |
| D9 | **Hachura por material/norma** | Padrão/ângulo/passo, alternância em montagem, preenchimento sólido para peça fina, padrões ISO 128-50 | P1 | M | D4 |
| D10 | **Cotagem inteligente** | Cota pelo contexto, conjuntos ordenada/linha de base/cadeia, importar cotas do modelo e gerar automáticas, cota de referência | P1 | G | D1 |
| D11 | **Chamadas de furo, chanfro e rosca; tolerâncias** | Notas lidas da feature com formato por norma, `4×`, reancoragem; tolerância estruturada e ajuste ISO 286 | P1 | M | D1, D4 |
| D12 | **Centros automáticos** | Marcas e eixos associativos por vista, padrão circular | P1 | P | D1 |
| D13 | **Balões ligados à BOM e tabelas configuráveis** | Balão com `item_ref`, ponta em ponto visível, layout/Renumber/Align; BOM com colunas, linhas manuais, ordenação, massa; tabela de furos por vista com tags; tabela de dobras com IDs; tabela de revisões | P1 | G | D2, B26, B50 |
| D14 | **Fundo de folha e carimbo editáveis** | Formato de folha `.fkf`, células editáveis, carimbo por folha, massa pelo material | P1 | M | D4 |
| D15 | **Exportação profissional** | PDF com folhas selecionadas, marcadores, fontes incorporadas; DXF R2000+ com ARC/CIRCLE/HATCH/MTEXT/unidades; DXF 1:1 da planificada; mapa de camadas; DWG opcional via conversor | P1 | G | — |
| D16 | **MBD → desenho** | `FromCapture` com anotações PMI vivas, filtro por plano, sobrescrita de posição | P1 | G | D1, PMI-03 |
| D17 | **Estrutura soldada: vista por corpo e detalhe de ponta** | `ViewKind::Body`, cota de comprimento, detalhe de meia-esquadria (continua B50) | P1 | M | B50, D6 |
| D18 | **Vistas de montagem avançadas** | Lista de estados, posição alternativa/sobreposição, explodida com trilhas, tabela de solda | P2 | G | D2 |

Ordem sugerida: D3 → D1 → D2 (a base de confiança), depois D5/D6/D8/D9/D10 em paralelo, e D15/D16 por último.

---

## 5. Fontes

**CAD avançado B (City University — tutoriais de Generative Drafting)**
- (fonte em procedencia.md) (tipos de vista)
- (fonte em procedencia.md) (índice)
- .../draugbt0301.htm (definir folha), 0305 (vista de fundo), 0403 (auxiliar), 0404 (corte em desvio), 0405 (corte alinhado), 0408 (detalhe), 0409 (recorte), 0412 (planificada), 0413 (explodida), 0414 (mostrar geometria), 0417 (isolar), 0421 (breakout), 0422 (vista FD&T), 0504 (posição relativa), 0508 (travar), 0509 (balões), 0802 (BOM), 1003 (cota de rosca), 1004 (cotas que dirigem restrições), 1100 (geração de cotas) — todas em `.../draug_C2/draugbtNNNN.htm`
- (fonte em procedencia.md) (assistente), draugbt1001/1002/1014/1015 (geração e filtros, só citados em buscas)
- https://wichita.edu/industry_and_defense/NIAR/Documents/Drafting-R30.pdf e https://www.wichita.edu/industry_and_defense/NIAR/Documents/Drafting-2019x.pdf (só índices em buscas; o WebFetch devolveu 404)
- (fonte em procedencia.md) (modo de atualização automático)
- https://www.ibm.com/support/pages/apar/HD78723 e https://www.ibm.com/support/pages/apar/HD70182 (Area Fill)
- https://www.ibm.com/support/pages/apar/HD53303 (limitação da geração de cotas)

**CAD avançado C (fornecedor de CAD avançado Help)**
- (fonte em procedencia.md) (cortes)
- (fonte em procedencia.md) (fundamentos de vistas)
- (fonte em procedencia.md) (tipos de vista)
- (fonte em procedencia.md) (atualizações automáticas)
- (fonte em procedencia.md) (recuperar anotações do modelo)
- (fonte em procedencia.md) (notas de furo e rosca)
- (fonte em procedencia.md) (tabelas de furos)
- (fonte em procedencia.md) (tipos de anotação)
- (fonte em procedencia.md) e (fonte em procedencia.md) (quebra e breakout; só trechos de busca)
- (fonte em procedencia.md) (tipo de corte pela geometria)
- (fonte em procedencia.md) (tabela de dobras)
- (fonte em procedencia.md) (relato de usuários)

**CAD avançado A (trechos extraídos de help.CAD avançado A.com e blogs; páginas de ajuda não legíveis pelo WebFetch)**
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
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://cati.com/?p=120622 (tipos de tabela; lido)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) e (fonte em procedencia.md) (só trechos de busca; WebFetch deu 403)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (cotas penduradas, relato de usuário)

**Outros CADs**
- (fonte em procedencia.md) (lido) e (fonte em procedencia.md)
- (fonte em procedencia.md) (lido)
- (fonte em procedencia.md) e https://www.soliddna.com/SEHelp/ST5/EN/a_h/getstarteddwg1p.htm (CAD avançado H; trechos de busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) (CAD avançado G)
- (fonte em procedencia.md) e (fonte em procedencia.md) (CAD avançado F; só índice)

**Normas e contexto do projeto**
- https://www.vetorial.design.ufrn.br/src/pdfs/Normas.pdf (lista que cita as NBR 16861, 17006, 17067, 17068 e 16752 como substitutas das antigas; secundária)
- Do repositório: `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`, `docs/benchmark/catalogo.md` (DES-01..12), `docs/backlog/backlog.md` (B30, B50), `docs/contextos/design-review.md` (DR-33..45)

---

## Handoff

- **Resultado.** Relatório completo (fichas de 40 funções, 50 RF, 10 RNF, 18 itens de backlog), pronto para o lead gravar em `docs/pesquisa/desenho.md`.
- **Evidência.** Li `lib.rs`, trechos de `source.rs`, `layout.rs`, `export.rs` e `dwg_ui.rs`, além do catálogo, do ADR 0011 e de DR-34..45. As referências de linha vêm dessa leitura.
- **Arquivos alterados.** Nenhum. Nada foi compilado nem executado.
- **Falhas e limites.**
  - A ajuda do CAD avançado A não pôde ser lida pelo WebFetch (páginas renderizadas por JavaScript). Os [V] do CAD avançado A vêm de trechos da busca e de blogs.
  - Os PDFs da Wichita State devolveram 404.
  - Não confirmei na ajuda os comandos de cota de chanfro do CAD avançado A e do CAD avançado B, "exploded state" do CAD avançado A, balões empilhados e detalhes de CAD avançado F, CAD avançado G e CAD avançado H. Estão marcados como [C] ou como "só índice".
  - Os bugs "prováveis" do Fenkra são inferências da leitura do código, não reproduzidos. Os de maior risco para conferir primeiro são a projeção a partir de Topo/Section/Camera/Flat (`dwg_ui.rs:894–912`, `source.rs:296`) e a âncora de cota sem referência (`dwg_ui.rs:947`).
- **Riscos.**
  - O catálogo marca DES-03, DES-05 e DES-12 como "✅ completo" e DES-04 cita "auxiliar", que não existe no código. Vale reclassificar para 🟡 antes de planejar.
  - Os tamanhos P/M/G do backlog são estimativas minhas, sem calibrar com o histórico do projeto.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As cinco maiores lacunas do desenho técnico do Fenkra são cotas sem referência estável nem diagnóstico, ausência de estado, atualização e trava de vistas, só 1º diedro sem herança de norma, catálogo incompleto de vistas (auxiliar, cortes especiais, interrompida, breakout, recorte, explodida) e cotagem inteligente inexistente (importar ou gerar cotas, ordenada, chanfro, chamada de furo).
