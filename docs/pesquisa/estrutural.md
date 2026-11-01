# Estrutural e soldas (Structure Design / Weldments / Frame Generator)

> Relatório de pesquisa para gravar em `docs/pesquisa/estrutural.md`. Nenhum arquivo foi criado ou alterado.
> **[V]** = confirmado em fonte lida (lista na seção 5). **[C]** = conhecimento geral ou inferência minha, não confirmado em fonte.
> Limitações da pesquisa:
> - A ajuda online do SolidWorks (`help.solidworks.com`) só devolveu o menu do site, porque o texto é montado por JavaScript. Os detalhes do SW vêm de resumos de busca da própria ajuda, de blogs de revendas (CATI, GoEngineer) e de posts da comunidade.
> - Para a CATIA V5 li o guia oficial *Structure Design* da R14 (PDF), as descrições de produto da 3DS (SR1, WD1, Frame Structure Designer) e a documentação da API (pycatia/CAA).
> - Os espelhos `catiadoc.free.fr` estavam inacessíveis.
> - O guia da R14 é antigo, mas os princípios (suporte, âncora, cutback recalculado, grade) seguem valendo.
> - Onde um achado do código do Fenkra é inferência de leitura, está marcado como "provável" e vem com o teste que o confirma.

---

## 1. Resumo e principais lacunas

**Resumo**

1. O Fenkra tem um módulo estrutural funcional. `StructMember` leva perfis (10 tipos de seção) por linhas do esqueleto, com ancoragem, rotação, deslocamentos e prolongamentos.
2. Cada trecho reto vira um sólido separado dentro do mesmo corpo, via `add_separate` (`fenkra-solidos/src/lib.rs:1722`).
3. Os cantos têm 4 modos: Nenhum, Meia-esquadria, Aparar e Encaixe. Há regra por par de linhas (`StructJoint`) e folga de solda.
4. Placa de extremidade e reforço existem, mas são localizados por um ponto fixo no espaço, não por referência.
5. `structural_groups` agrupa peças iguais por assinatura, numera os itens e alimenta a lista de corte, os balões e `estrutura_para_montagem` (peças `.fkp` + montagem `.fka`).
6. O que o CATIA V5 dá de filosofia (grade, cutback recalculado, âncora nomeada, falha isolada por elemento) está só em parte no Fenkra.
7. O que SW/Inventor dão de inteligência também está só em parte: gerenciador de cantos, aparar até face/corpo, comprimento de corte e ângulos automáticos, `CUTDETAIL`, sub-soldagens, cordões 3D e estado de falha por membro.
8. O ponto mais frágil é a identidade. Placa e reforço apontam para um ponto cru, a assinatura de grupo não vê o formato das pontas, e a numeração dos itens muda quando um comprimento muda.
9. A lista de corte usa o comprimento da linha de centro, não o comprimento de corte (ponta longa). Isso erra o número de barras e a ordem de compra.
10. Soldagem é só símbolo ISO 2553 no desenho/PMI. Não há cordão 3D, preparação de junta, lista de soldas nem submontagem de solda.

**Principais lacunas**

| # | Lacuna | Gravidade |
|---|---|---|
| L1 | Placa e reforço por ponto fixo; `member_end` escolhe a ponta mais próxima sem tolerância (`lib.rs:2148`). | alta |
| L2 | Comprimento de corte e ângulos de ponta ausentes na lista de corte. | alta |
| L3 | Assinatura de grupo cega ao formato das pontas e dependente do sentido da linha; numeração de itens instável. | alta |
| L4 | Sem aparar/estender até face/plano/corpo; só há tratamento nas pontas coincidentes (`shared_end`, `lib.rs:4651`). | alta |
| L5 | Uma linha ruim derruba o comando inteiro; não há estado "falhou" por membro. | alta |
| L6 | Sem gerenciador de cantos, sem grupos de orientação e sem entalhe por perfil. | média |
| L7 | Sem cordão 3D, preparação de solda, lista de soldas e submontagem de solda. Montagem "viva" ainda congelada (B50). | média |
| L8 | Biblioteca de perfis curta (10 famílias, 3–7 tamanhos). O CSV de `docs/contextos` lista 159 designações ABNT/AISC/EN, mas as colunas de dimensão estão vazias. | média |
| L9 | Placas e reforços fora da lista de corte (`cut_list` filtra `StructItem::Member`, `lib.rs:4977`) e, por consequência, sem balões (`item_centers`, `lib.rs:4928`). | média |
| L10 | Sem grade 3D, sem esqueleto como conjunto geométrico ordenado e sem publicação do esqueleto. | média |

### Mapa do código de hoje

| Peça | Onde |
|---|---|
| Tipos | `fenkra-core/src/doc.rs:1402` (`SectionKind`), `:1417` (`CornerMode`), `:1428` (`StructMember`), `:1456` (`StructJoint`), `:1468` (`StructPlate`), `:1477` (`StructGusset`) |
| Seções e biblioteca de fábrica | `fenkra-solidos/src/structural.rs:61` (`builtin`), `:87` (`section`), `:147` (`anchor_point`) |
| Geometria dos membros | `lib.rs:1737` (`members`), `:1782` (`member_shape`), `:1799` (`all_member_lines`), `:1816` (`joint_rule`) |
| Execução do comando | `lib.rs:3785` (membro), `:3878` (placa), `:3893` (reforço) |
| Itens, grupos e lista de corte | `lib.rs:4767`–`4990` (`StructItem`, `StructSignature`, `log_struct`, `group_members`, `structural_groups`, `cut_list`) |
| Montagem | `fenkra-montagem/src/estrutura.rs:49` (`estrutura_para_montagem`) |
| Interface | `fenkra-app/src/st_ui.rs:21` (painel do membro), `:106` (cantos), `app.rs:1516`–`1556` (criação de membro, placa e reforço) |
| Testes | `lib.rs:5279`, `:5354`, `:5507`; `estrutura.rs:143` |

---

## 2. Fichas por comando/função

### 2.1 Esqueleto (linhas guia) e conjuntos geométricos (CATIA: Skeleton / Geometrical Set / "Select Support" / SW: croqui 3D + Structure System / Inventor: peça-esqueleto / outros: Solid Edge path segments, Onshape path sketch / Fenkra: `StructMember.paths: Vec<CurveRef>`)

- **O que faz:** define o "caminho" onde cada barra será colocada. É a fonte de verdade da geometria da estrutura. Mudar o esqueleto deve mudar as barras.

- **CATIA V5 — no detalhe:**
  - O Shape (membro) é criado de duas maneiras [V]: *Select Support* (linha, spline, arco etc.) ou *Point-to-point*. Nos dois casos o suporte pode vir de uma grade, de wireframe ou de outra estrutura.
  - A multisseleção de suportes cria vários membros de uma vez [V].
  - A opção de sessão "Keep link with selected object" mantém a associatividade com o suporte. "Synchronize all external references for update" sincroniza referências externas na atualização [V].
  - Com o vínculo desligado, o membro fica "não restringido". Só então o comando *Stretch* funciona [V].
  - A transição de ponto-a-ponto para suporte é permitida na edição. A inversa não [V].
  - Na prática (boas práticas de CATIA [C]), o esqueleto mora num *Geometrical Set* dedicado, dentro da peça ou numa peça-esqueleto publicada. As barras referenciam elementos **publicados**. Se o esqueleto muda, o update propaga; se uma linha some, só aquele membro fica em erro no diagnóstico de update.
  - A API de CAA/pycatia mostra variantes de colocação [V]:
    - dois pontos;
    - ponto + ponto sobre curva (com razão ou comprimento);
    - ponto + direção + comprimento;
    - ponto + direção + limite;
    - curva;
    - curva normal a uma superfície de referência;
    - interseção de duas superfícies;
    - entre dois membros com plano de referência.

- **SolidWorks — no detalhe:**
  - O esqueleto é um croqui 2D/3D (ou arestas) [V]. Cada segmento ganha um corpo.
  - O *Structure System* (2019+) é um ambiente de croqui 3D para pôr membros primários e secundários e gerenciar cantos [V].
  - Um grupo de membros precisa ser um caminho contínuo ou linhas paralelas [V] (GoEngineer). Pernas verticais soltas formam grupos separados para poderem ser giradas/espelhadas individualmente.

- **Inventor — no detalhe:**
  - Exige uma peça-esqueleto. "Cada membro precisa ter uma linha correspondente no esqueleto" [V]. O esqueleto pode vir de um modelo existente ou ser definido no contexto da montagem, juntando linhas e arestas de várias peças.
  - Duas formas de colocação [V]: sobre arestas (2D/3D, arcos, splines, com seleção em cadeia ou por croqui) ou entre dois pontos. Não se misturam arestas e pontos.
  - Opção *Merge*: várias arestas conectadas viram **um** membro (varredura). Nesse caso não há tratamento automático de pontas [V].
  - Diagnóstico de esqueleto [V]: a lista de erros distingue "esqueleto removido", "linhas de croqui colineares", "geometria de referência ausente" e "membros ausentes".

- **Outros:**
  - Solid Edge [V]: segmentos associativos (linear/curvo/dobrado) mais pontos 3D de conexão (OrientXpres), em ambiente "Frame Design" dentro da montagem.
  - Onshape [V]: croqui de caminho + croqui de perfil, escolhendo qual ponto do perfil fica no caminho.
  - Creo AFX [V]: perfis sobre curva reta, dois pontos ou croqui curvo.

- **Fenkra hoje:**
  - `paths: Vec<CurveRef>` (`doc.rs:1430`) aceita referências geométricas e curvas do app.
  - `members()` (`lib.rs:1737`) divide polilinhas em trechos retos onde o ângulo entre segmentos passa de ~11,5° (`dot < 0.98`, `lib.rs:1754`). Arcos e curvas viram varredura (`member_shape`, `lib.rs:1791`).
  - **Bug provável 1:** se qualquer `CurveRef` falha, `self.curve(p, 0)?` aborta `members()` e o comando inteiro vai a erro. Não há falha por linha (L5).
  - **Bug provável 2:** curvas mais suaves que 11,5° nunca são cortadas num vértice. Uma polilinha com dobra de 8° vira um único sólido varrido (e, na lista de corte, um comprimento de curva).
  - **Bug provável 3:** um polígono fechado (`closed`) não divide no vértice de fechamento, porque só os pontos interiores são considerados (`lib.rs:1752`).
  - Não existe conjunto geométrico "Esqueleto" ordenado/não ordenado, nem publicação do esqueleto, nem grade.

- **Melhor prática para o Fenkra:**
  - Fluxo: **suporte** (linhas/arestas/cadeia/croqui, ou dois pontos) → **modo** (uma barra por linha | cadeia em barras retas | cadeia em um corpo varrido, como o *Merge* do Inventor) → **parâmetros** (perfil, ancoragem, rotação) → **avançado** (grupos, prolongamentos, cantos).
  - Regras:
    - A unidade de erro é a **linha**. Cada linha guarda seu estado (`ok` | `perdida` | `degenerada` | `colinear`) e o comando devolve os corpos que pôde, com aviso.
    - O ângulo de corte de polilinha vira parâmetro do documento (padrão 5°, não 11,5° fixo), e o vértice de fechamento entra no teste.
    - Ao criar o membro, oferecer "criar conjunto Esqueleto" e mover as linhas para ele (ADR 0011, princípio 2).
  - Testes:
    - quadro com 1 linha removida → 3 corpos + aviso nomeando a linha;
    - polilinha com dobra de 8° (decisão explícita: sólido único ou dois trechos);
    - polígono fechado de 4 vértices → 4 trechos com cantos nos 4 vértices.

---

### 2.2 Grade e pontos de referência (CATIA: Grid / SW: Planos e pontos de referência + croqui 3D / Inventor: pontos de trabalho / outros: Creo "datum points" / Fenkra: ausente)

- **O que faz:** cria um reticulado de planos, linhas e pontos para ancorar o esqueleto em espaçamentos regulares (colunas, vãos, andares) e permitir alterar o projeto mexendo na grade.

- **CATIA V5 — no detalhe** [V, guia R14]:
  - Comando *Grid*. O diálogo pede um nome (aparece na árvore) e um sistema **cartesiano ou polar**.
  - **Modo relativo** (padrão): mover um plano desloca os planos que dependem dele. **Modo absoluto**: cada plano se posiciona em relação à origem e pode ser movido independentemente.
  - Entradas: origem (coordenadas ou ponto), espaçamento por eixo e **número de pontos** por eixo.
  - A grade é feita de elementos wireframe (linhas, pontos, planos). Os membros se apoiam em linhas da grade, e "mudanças de projeto entram modificando a grade e atualizando a estrutura".
  - O descritivo do produto fala em colocação por métodos *linear*, *direcional* e *ponto-a-ponto* [V].

- **SolidWorks — no detalhe:** não há grade; usa-se croqui 3D com planos/pontos de referência. O Structure System aceita membros ligados por pontos [C].

- **Inventor — no detalhe:** não há comando de grade; o esqueleto é feito com croquis e pontos de trabalho [C].

- **Outros:** o Creo AFX tem "projeto" com perfis e parâmetros de projeto [V, só títulos]. Nenhum outro CAD pesquisado tem equivalente direto da grade CATIA.

- **Fenkra hoje:** não existe. Linhas do esqueleto vêm de `RefKind::LinePoints` com `PointRef::Fixed` ou de croquis. Não há elemento "Grade".

- **Melhor prática:**
  - Fluxo: **suporte** (origem + plano base) → **modo** (cartesiana | polar; relativo | absoluto) → **parâmetros** (espaçamentos como expressões/parâmetros, nº de pontos) → **avançado** (nomes dos eixos "A, B, C / 1, 2, 3").
  - Gera uma coleção de planos/linhas/pontos publicáveis. Não é uma feature de sólido.
  - Regra: o modo relativo é o padrão e o espaçamento é um parâmetro. Mudar `vao = 3000` desloca tudo que está à direita.
  - Teste: grade 3×2×1 com membros nas linhas; mudar o espaçamento X de 1000 para 1200 e conferir que o comprimento dos membros em X muda e nenhum membro fica solto.

---

### 2.3 Membro estrutural por perfil (CATIA: Shape / SW: Structural Member / Inventor: Insert Frame / outros: Solid Edge Frame, Onshape Frame, Creo AFX Profile / Fenkra: `StructMember`)

- **O que faz:** extruda (ou varre) um perfil padronizado ao longo de cada linha do esqueleto, produzindo um corpo por barra.

- **CATIA V5 — no detalhe** [V]:
  - O Shape pede seção (do catálogo, ex.: AISC `L5x5x0.5`), material (par material-classe, recurso de projeto), ponto de ancoragem e orientação.
  - A seção escolhida é copiada para uma pasta "resolvida", que alimenta a lista de seções do diálogo.
  - As propriedades geométricas e mecânicas da seção podem ser consultadas pelo menu de contexto.
  - Cada membro é uma feature própria na árvore (`Shape.n`), com **atributos customizados** vindos do dicionário de features (ex.: *tipo* = viga, coluna, travessa).
  - Modificações na árvore de especificação: *Definition*, *Extend* (deslocamento positivo/negativo nas pontas, ou troca do limite) e *Stretch*. É preciso selecionar na árvore, não na geometria.
  - Membros curvos e torcidos: a opção *Reference* torna a seção normal a uma superfície de varredura/loft.
  - *Split* corta membros por wireframe e o original é redefinido, com um novo elemento que herda features. *Merge* une dois membros cujas especificações têm tangência; o segundo perde suas features.
  - O descritivo do produto lista mover, girar e espelhar, e trocar seção, limites, âncora, espessura, orientação e material [V].

- **SolidWorks — no detalhe:**
  - Diálogo *Structural Member* [V]: padrão (ISO, ANSI etc.) → tipo → tamanho.
  - *Apply corner treatment* pode ser desmarcado e o canto tratado depois (por exemplo no Trim/Extend).
  - **Grupos**: caminho contínuo ou linhas paralelas; cada grupo tem suas próprias configurações; "novo grupo" no mesmo comando.
  - Posicionamento: *Locate Profile* (ponto de perfuração padrão = origem do croqui, ou qualquer vértice/ponto), *espelhar perfil*, *eixo vertical* e *ângulo de rotação*.
  - *Merge arc segments*: segmentos de arco tangentes aos vizinhos viram um só corpo. Desmarcado, cada curva vira um corpo separado, o que afeta a lista de corte [V].
  - Perfis personalizados são uma peça de biblioteca `.sldlfp`. A pasta onde ficam corresponde a *tipo*, a pasta acima a *padrão*; o ponto de perfuração padrão é a origem do croqui; desde 2014 uma biblioteca guarda várias configurações [V].
  - Em 2023 é possível mudar o tamanho de vários membros iguais por configuração [V]. Em 2025 há "favoritos" de perfis [V].

- **Inventor — no detalhe** [V]:
  - Diálogo *Insert*: Padrão / Família / Tamanho / Material / Aparência (do Content Center). O conta-gotas copia tudo, só o membro ou só a orientação de outro membro.
  - Orientação: deslocamento vertical/horizontal, ângulo, *Align* (a face plana, aresta linear, eixo, plano ou linha de croqui) e *Mirror* (indisponível junto com Align).
  - *Custom Point*: ponto de inserção alternativo definido na criação da seção.
  - Cada membro é um **arquivo de peça** próprio, com nome padrão composto de padrão + número de estoque + número único. A pasta *Frame* da montagem ganha uma hierarquia de navegador.
  - A feature *Driven Length* controla o comprimento e fica protegida contra edição acidental.
  - *Reuse*: membros retos idênticos compartilham um único arquivo; *Change Reuse* edita posição/geometria; *Break Reuse Members* torna independente; não vale para membros curvos.
  - *Refresh* atualiza membros depois de mudança na tabela de família.
  - *Change* edita o membro depois. A edição de um perfil publicado **não atualiza** membros já colocados; é preciso "Edit with Frame Generator" (nota de fórum, anedótica).
  - *Promote/Demote* reorganizam membros entre subconjuntos.

- **Outros:**
  - Solid Edge [V]: seção na barra de comando, depois o tratamento de canto; as opções de quadro incluem raio e *coping*.
  - Onshape [V]: perfis ISO/ANSI/AS/AISC/8020 e perfis personalizados de croqui em outro documento; grupos trimados pela hierarquia estrutural (elementos "dominantes" primeiro).
  - NX Structure Designer [V, resumo de busca]: assistente de desenho de quadro 2D, orientação 3D automática por regras do usuário e tratamentos de canto.
  - Creo AFX [V]: perfis curvados sobre curvas, perfis de chapa, padrões de pontos nas extremidades.

- **Fenkra hoje:**
  - Painel em `st_ui.rs:21`. Criação em `app.rs:1516`: família padrão = a primeira, tamanho = índice 3.
  - Um comando = uma família/tamanho/âncora/rotação **para todas** as linhas selecionadas.
  - A orientação vem da regra "up = Z, exceto linhas quase verticais, onde up = X" (`lib.rs:1772`). Ela depende do sentido em que a linha foi desenhada (`z` = fim − início). Duas linhas opostas de um perfil assimétrico (cantoneira) saem espelhadas uma da outra.
  - Não há atributos customizados de membro (tipo viga/coluna), nem espelhar, nem alinhar a uma face/aresta, nem override por linha.
  - O nome dos itens é `Comando.N` (`estrutura.rs:96`), sem nome de usuário por membro.
  - **Bug provável 4 (prolongamento):** `ext1/ext2` entram em **todos** os trechos de uma polilinha (`lib.rs:1777`), inclusive nos vértices internos. Com cantos `Nenhum`, cada trecho invade o vizinho por `ext1`/`ext2`. O correto é prolongar só as pontas livres da cadeia.

- **Melhor prática:**
  - Fluxo: **suporte** → **modo** → **parâmetros** (padrão/família/tamanho/material/aparência) → **avançado** (ancoragem, rotação, espelhar, alinhar a, grupos, atributos).
  - Regras:
    - Orientação por regra pública e visível (a regra CATIA: "alma para Y se o suporte é vertical, senão para Z"), independente do sentido do desenho.
    - Botões *Inverter* e *Redefinir*.
    - Prévia 3D da seção sobre a linha antes de confirmar.
    - Cada linha pode ter override de rotação/espelho (equivalente aos grupos do SW).
    - Atributos de membro (tipo, número de marca) em campo próprio, propagados à lista de corte.
  - Testes:
    - quadro de cantoneiras desenhado horário e anti-horário dá a mesma orientação em relação ao interior;
    - polilinha de 3 trechos com `ext1=10` e cantos `Nenhum` → só o início do primeiro trecho é prolongado;
    - `Nenhum` não cria sobreposição.

---

### 2.4 Catálogo de perfis (CATIA: Section Catalog / SW: Weldment Profiles / Inventor: Content Center + Structural Shape Authoring / outros: Onshape perfil de croqui, Creo AFX Profile Library / Fenkra: `structural::builtin` + `user_profiles`)

- **O que faz:** fornece famílias e tamanhos padronizados (tubos, I, U, L, barras…) com cotas, propriedades de seção e pontos de ancoragem.

- **CATIA V5 — no detalhe** [V]:
  - O catálogo é paramétrico, baseado em tabelas de projeto (*design tables*). Há bibliotecas padrão (I, U, T, L, tubo retangular, tubo redondo etc.) e navegador de catálogo.
  - **Seção do usuário**: desenha-se um croqui no plano xy, em escala real, e salva-se como CATPart na pasta de seções.
  - **Nomear características da seção**: cada aresta recebe um nome padronizado (borda da alma, borda da mesa, face moldada…). Isso permite **trocar um perfil por outro** sem perder referências.
  - **Pontos de ancoragem** definidos por seção do usuário (pontos verdes no diálogo).
  - Catálogos paramétricos podem gerar "peças resolvidas".
  - O Frame Structure Designer (3DEXPERIENCE) calcula comprimento, superfície, volume e peso e gera relatórios por modelo da empresa [V].

- **SolidWorks — no detalhe** [V]: `.sldlfp` com croqui, pontos de perfuração, pasta = padrão/tipo; configurações por tabela de projeto; pastas configuradas em *File Locations → Weldment Profiles*. Pacotes de perfis vêm do SOLIDWORKS Content (GoEngineer).

- **Inventor — no detalhe** [V]: perfis estruturais publicados no Content Center (biblioteca leitura/escrita). O comando *Structural Shape Authoring* publica o perfil, e o entalhe (*notch*) é um perfil de corte desenhado no croqui-base com cotas **ligadas aos parâmetros da família** (ex.: entalhe = altura/3).

- **Outros:** Onshape [V]: perfis padrão ISO/ANSI/AS/AISC/8020, mais perfis personalizados de croqui em documento compartilhado ("biblioteca da empresa"). Creo AFX [V]: aço estrutural, alumínio com T-slot e perfis personalizados (seções de alumínio só a partir da 8.0.5.0).

- **Fenkra hoje:**
  - `builtin()` (`structural.rs:61`): 10 famílias, cada uma com 3 a 7 tamanhos. Os dados são em mm, de "medidas comerciais usuais", sem fonte registrada.
  - O CSV `docs/contextos/perfis-estruturais.csv` tem 159 linhas (ABNT, AISC, EN 10365) **só com a designação**: as colunas de dimensão e de peso estão vazias nas linhas lidas. Não está ligado ao `builtin`.
  - As seções têm **cantos vivos**. Perfis laminados reais têm raios de concordância mesa-alma e mesas inclinadas (U); tubos têm raios externos (~1,5–2,0 t [C]). Isso afasta massa e peso/m do catálogo.
  - Perfis do usuário (`settings.user_profiles`) só variam cotas dos mesmos 10 tipos.
  - Não há perfil desenhado em croqui, nem nome de característica, nem âncoras próprias (a âncora é a grade 3×3 + centróide).

- **Melhor prática:**
  - Fluxo: **norma** → **família** → **designação** → **dados/peso** (conferidos).
  - Cada linha tem fonte, nível A/B/C e licença (B42).
  - Regras:
    - Todo perfil de fábrica tem teste "área × densidade × 1 m ≈ peso/m do catálogo ± 1%".
    - Raios de concordância opcionais (modelo "fiel" vs "simplificado" para desempenho).
    - Perfil de croqui com pontos nomeados (centro, ancoragens, faces de solda) e publicação de arestas nomeadas para trocar perfil sem quebrar referências.
  - Testes:
    - IPE 200, HEA 200, W200x15, tubo 40x40x3: área e massa lineares dentro de 1%;
    - trocar um perfil I por outro I mantém a âncora e as regras de canto.

---

### 2.5 Ancoragem e orientação (CATIA: Anchor point / Orientation / SW: Locate Profile + Mirror + Rotation / Inventor: Insert point + Offset + Angle + Align + Mirror / outros: Onshape profile-point-on-path / Fenkra: `anchor`, `angle`, `off_x/off_y`)

- **O que faz:** define qual ponto da seção cai sobre a linha guia e como a seção gira em torno dela.

- **CATIA V5 — no detalhe** [V]:
  - Âncora escolhida num menu ou clicando nos pontos verdes da prévia da seção (o ponto que "toca" a linha de suporte).
  - Orientação em passos de 90° (passo mudável para 1°), *Flip* (espelha em torno do eixo vertical da seção) e *Reset*.
  - Regra de orientação padrão: se a linha é vertical, a alma (eixo V local) aponta para o eixo Y do documento; se não, para o eixo Z.
  - A seção do usuário traz seus pontos de ancoragem definidos na preparação.

- **SolidWorks — no detalhe** [V]: *Locate Profile* (ponto de perfuração sobre a seção), *espelhar* e *eixo vertical* para inverter o perfil, *ângulo de rotação*. O GoEngineer observa que pernas verticais precisam de grupo próprio para poderem ser giradas/espelhadas individualmente.

- **Inventor — no detalhe** [V]: pontos de inserção (nove, mais "ponto personalizado" se a seção define um), deslocamento vertical/horizontal com sinal (0 = encosta na aresta do modelo), ângulo (manipulador, passos de 45° com Shift), *Align* a face/aresta/eixo/plano/linha, *Mirror*.

- **Outros:** Onshape [V]: escolhe-se qual ponto do perfil fica sobre o caminho. Solid Edge [V]: a orientação do segmento pode ser travada paralela a um eixo/plano (OrientXpres).

- **Fenkra hoje:**
  - `anchor` 0–9 (centro, 8 pontos da caixa, centróide), `angle` em graus, `off_x/off_y` (`doc.rs:1436`–`1441`).
  - `anchor_point` (`structural.rs:147`) usa a **caixa envolvente**, então para cantoneira, U e T os pontos 1–8 não coincidem com vértices reais (só o 9 usa o centróide).
  - A referência de rotação é o "up" do mundo (`lib.rs:1772`). Não existe "alinhar a", espelhar, nem passo de 90° na interface.
  - **Bug provável 5:** `placa` e `reforço` usam `max(w,h)/2` como folga e ignoram o deslocamento de âncora e a rotação (`lib.rs:3826`, `3832`, `3901`, `3902`). Num tubo 80×40, a distância usada é 40 mesmo quando a face do canto está a 20.

- **Melhor prática:**
  - Fluxo: **âncora** (pontos nomeados da seção, não só da caixa) → **orientação** (regra pública + Inverter/Redefinir + passo de 90°) → **alinhar a** (face/aresta/plano) → **deslocamentos**.
  - Cada seção expõe âncoras nomeadas. Para o canto, o "profundidade na direção da junta" é calculado da seção real, não de `max(w,h)`.
  - Testes:
    - tubo 80×40 com âncora de canto e Aparar: o membro 2 para exatamente na face do membro 1;
    - cantoneira com âncora 7 e rotação 90°: a perna sobre a linha é a esperada (comparar com geometria de referência).

---

### 2.6 Prolongar e encurtar (CATIA: Extend / Stretch / SW: Allow Extension, offset / Inventor: Lengthen/Shorten / Fenkra: `ext1`, `ext2`)

- **O que faz:** acrescenta ou retira comprimento numa ponta, sem mexer no esqueleto.

- **CATIA V5** [V]: *Extend* na árvore abre "Limits Definition" (valores de deslocamento e/ou troca do limite). *Stretch* só vale para membros lineares não restringidos.

- **SolidWorks** [V]: o *Allow extension* do Trim/Extend permite que o segmento **alongue** até o limite. Desmarcado, só aparam.

- **Inventor** [V]: *Lengthen/Shorten* por uma ou duas pontas (positivo alonga, negativo encurta). Falha se há tratamentos de ponta prévios, que precisam ser suprimidos ou removidos. O fim recebe um nó na árvore.

- **Fenkra hoje:** `ext1/ext2` aplicados a todo o comando. Veja o bug provável 4 (2.3). O comprimento de corte da lista soma o prolongamento explícito (`member_length`, `lib.rs:4945`).

- **Melhor prática:**
  - Fluxo: **suporte** (a ponta) → **modo** (valor | até limite) → **parâmetros** (sinal) → **avançado** (apagar tratamentos prévios).
  - Prolongar é por **ponta**, não por comando. Pontas internas de cadeia não recebem prolongamento.
  - Se o prolongamento conflita com um canto, mostrar o aviso em vez de ignorar.
  - Teste: 4 barras com `ext1=5` na primeira; as outras três mantêm o comprimento; a lista de corte diz 405 para a primeira e 400 para as demais.

---

### 2.7 Canto em meia-esquadria (CATIA: Cutback "Miter cut" / SW: End Miter / Inventor: Miter / outros: Solid Edge Miter, Onshape mitered / Fenkra: `CornerMode::MeiaEsquadria`)

- **O que faz:** corta as duas pontas pela bissetriz para encontrarem-se rentes.

- **CATIA V5** [V]: o *Cutback* "Miter cut" corta o membro com um plano; a diferença de resultado depende de qual membro se seleciona primeiro. Não se aplica miter a um membro **já** mitrado: remove-se o corte ("None") e aplica-se a ambos. Normal/Weld cut podem ser aplicados a um membro já mitrado. Membros curvos não podem ser cortados. O offset vale para normal, weld e miter. O cutback é **recalculado** quando o membro muda.

- **SolidWorks** [V]: *End Miter* no Trim/Extend ou no tratamento de canto da criação. Para seções de tamanhos diferentes, *Angle Bisector*; para tamanhos iguais, *Full Flush*. Há folga de solda. A ordem de corte pode ser invertida. Para três membros num canto, ajusta-se a ordem de corte (ex.: um membro com ordem 2).

- **Inventor** [V]: *Miter* simétrico (dois membros no mesmo ângulo, folga total = *Miter Cut Extension*) ou de um lado (folga só no primeiro membro). *Bi-sect Miter* marcado faz um corte angular simples que **não** une as arestas; desmarcado (padrão) calcula o ângulo para um encontro aresta-a-aresta. Falha com membros paralelos ou colineares. Um membro mitrado fica assimétrico e erra quando reutilizado em espelho (aviso oficial). Com miter nas duas pontas, o comprimento é calculado sozinho.

- **Outros:** Solid Edge [V]: Miter, Butt1, Butt2, Extend/Trim, None. Onshape [V]: mitered/butt/coped, com *Corner Override* por junta.

- **Fenkra hoje:**
  - `lib.rs:3820`: plano na bissetriz `n = unit(di − dj)`, deslocado `gap/2` para cada lado. Equivale ao "bisect" do Inventor/SW: só a linha de centro é bisseccionada.
  - **Não há "rente" (full flush)** para seções diferentes (por exemplo tubo 60 com tubo 40): as faces externas não se encontram.
  - O teste `estrutural_quadro_cantos_placa_reforco` verifica a conservação do volume com meia-esquadria (volume = área × linha de centro).
  - **Bug provável 6:** o comprimento de corte mostrado é o da linha de centro (400 mm), mas o sólido vai de −20 a 420 mm no eixo local (o teste `lib.rs:5389` confirma o *bbox*). O serralheiro precisa de 440 mm de ponta longa (ver 2.15).
  - O padrão ao criar um membro é `MeiaEsquadria` (`app.rs:1537`). Se o canto não é de 90° em planos coerentes, a ordem de criação das linhas altera nada, mas a rotação da seção sim.

- **Melhor prática:**
  - Fluxo: **suporte** (par de pontas ou "todos os cantos do grupo") → **modo** (simétrico | um lado | rente | bisseção) → **parâmetros** (folga) → **avançado** (apagar tratamentos prévios, ordem).
  - Regra: se os tamanhos de seção diferem, perguntar "bisseção ou rente" na hora (a escolha é do usuário, com padrão "rente" para tubos de igual lado).
  - Mostrar ângulos de ponta calculados (em graus) no painel.
  - Testes:
    - quadro 400×300 com tubo 40×3: comprimento de corte 440/340, ângulos 45°/45°;
    - dois tubos de lados 60 e 40: modo rente deixa a face externa alinhada, bisseção deixa degrau.

---

### 2.8 Canto de topo (butt) (CATIA: Cutback "Normal cut" / "Weld cut" / SW: End Butt1, End Butt2 / Inventor: Corner Joint (antes Trim To Frame) / outros: Solid Edge Butt1/Butt2 / Fenkra: `CornerMode::Aparar`, `StructJoint.first`)

- **O que faz:** um membro atravessa (ou vai até a face externa) e o outro para nele. O corte do segundo é plano ou acompanha a face do primeiro.

- **CATIA V5** [V]: "Normal cut" é perpendicular ao eixo do suporte. "Weld cut" usa um plano de corte (que pode ser plano, face, superfície, placa ou outro membro) e deixa a ponta pronta para solda. Os dois vêm com *offset*. O item "Remove" retira do membro a parte que invade o outro. Num anel de 4 membros, o último par precisa escolher o membro 1 como aquele a ser cortado, senão o corte falha.

- **SolidWorks** [V]: *End Butt1* e *End Butt2*. O membro escolhido como "a trimar" para na face do outro. Com *End Trim*, vários corpos podem ser trimados contra um limite (plano/face ou corpos). A ordem de corte (1, 2…) decide quem prevalece no canto; mudar um membro para 2 funde os dois igualmente.

- **Inventor** [V]: *Trim To Frame* (depois renomeado *Corner Joint*): apara ou estende dois membros nas pontas para se encontrarem; aceita perfil de entalhe publicado e deslocamentos vertical/horizontal. Falha se os membros são paralelos/colineares.

- **Outros:** Solid Edge [V]: Butt1 apara contra o **mais curto**, Butt2 contra o **mais longo**. Onshape [V]: escolhe-se o membro dominante e os demais são trimados contra ele numa só operação.

- **Fenkra hoje:**
  - `Aparar | Encaixe if *first*` (`lib.rs:3824`): o membro 1 vai até a face externa do membro 2 por meio de um plano em `p − di·hj` com `hj = max(w,h)/2`.
  - Caso contrário (`lib.rs:3829`): o membro 2 é cortado com `clip_until` pelo sólido do membro 1 (estendido por `hi`), com folga via `offset_surface`.
  - A regra de quem é "membro 1" está em `StructJoint.first` (`doc.rs:1462`) e vale a regra do comando **posterior** (`joint_rule`, `lib.rs:1816`).
  - **Bug provável 7:** o `hj`/`hi` por `max(w,h)/2` é uma aproximação: para tubos retangulares, a face de fora do membro 2 pode estar a `min(w,h)/2`, e o membro 1 passa do ponto ou fica curto. Para membros com âncora fora do centro, o erro cresce.
  - Falta o critério "trimar contra o mais curto/mais longo" como atalho.

- **Melhor prática:**
  - Fluxo: **suporte** (par de membros) → **modo** (topo 1 | topo 2 | passante | mais curto | mais longo) → **parâmetros** (folga) → **avançado** (ordem).
  - A distância até a face é obtida do **sólido** do outro membro (interseção do eixo com a seção), nunca de `max(w,h)`.
  - Testes:
    - tubo 80×40 contra tubo 40×40 em T e em L, nas duas orientações: face de contato rente (distância medida = 0 ± 0,01 mm);
    - mudar `first` inverte quem atravessa e o volume total muda de forma previsível.

---

### 2.9 Aparar e estender até limite (CATIA: Cutback to plane/surface / SW: Trim/Extend (End Trim) / Inventor: Trim/Extend To Face / outros: Solid Edge Extend/Trim, Creo planar trim / Fenkra: ausente)

- **O que faz:** corta ou alonga um membro contra um plano, uma face ou um corpo que não é outro membro de ponta coincidente (viga que termina numa parede, coluna sobre chapa de base, T no meio de outro tubo).

- **CATIA V5** [V]: o suporte do corte pode ser plano, face, superfície, placa ou outro membro, com *offset*. O membro é redefinido e o cutback aparece na árvore. Membros curvos não podem ser cortados.

- **SolidWorks** [V]:
  - *End Trim* com limite **Face/Plano** (mais eficiente) ou **Corpos** (necessário para limites não planos como tubo redondo).
  - *Allow extension*: marcado, alonga ou apara; desmarcado, só apara.
  - Ao selecionar um plano/face, a prévia mostra os pedaços e um clique alterna **manter/descartar** cada um. Descartar funde os segmentos restantes adjacentes.
  - Dica da CATI: a superfície usada como ferramenta deve passar além de todos os lados da peça, senão estenda-a antes.
  - Para a lista de corte, os membros precisam estar trimados na própria peça [V].

- **Inventor** [V]: *Trim/Extend To Face* leva o membro a uma face do modelo com deslocamento. Falha se a face é paralela ao membro. Nesse caso o estado "doente" aparece na árvore ("Trim face is parallel to the member").

- **Outros:** Creo AFX [V]: *planar trims* e cortes entre perfis.

- **Fenkra hoje:** não existe. O tratamento só ocorre onde as pontas de duas linhas **coincidem** a menos de 1e-4 (`shared_end`, `lib.rs:4651`). Um travessa que termina no meio de um montante não recebe tratamento nenhum.
  - Consequência provável (L4): o membro "T" simplesmente atravessa o montante. Como os membros são `compound` (não são fundidos), a massa e o volume do corpo contam a interseção duas vezes.
  - Isso também afeta a lista de corte (o comprimento é o da linha) e o desenho.

- **Melhor prática:**
  - Fluxo: **suporte** (membro(s) + limite: plano/face/corpo/outro membro) → **modo** (aparar | aparar e estender | só estender) → **parâmetros** (deslocamento, lado a manter) → **avançado** (apagar tratamentos prévios).
  - Regras:
    - Se o limite é paralelo ao eixo ou não intercepta, o comando fica "falhou" com o motivo (como o Inventor), nunca ignora.
    - Aviso quando o resultado ainda se sobrepõe a outro membro.
  - Testes:
    - travessa que termina no meio do montante, `Aparar` contra o corpo: volume do conjunto = soma − interseção removida;
    - face paralela ao eixo → estado de erro com mensagem.

---

### 2.10 Entalhe e encaixe (cope/notch) (CATIA: Contextual End Cut / Coping / SW: — (via Trim bodies) / Inventor: Notch / outros: Onshape coped, Solid Edge coping / Fenkra: `CornerMode::Encaixe`)

- **O que faz:** recorta a ponta de um membro com o contorno do outro, para encaixe de tubos e para desviar de mesas de vigas.

- **CATIA V5** [V]:
  - *End cut* é um objeto paramétrico do catálogo (exemplo: "gatormouth") que se dimensiona pelas fórmulas definidas na criação.
  - **Padrão**: um só insumo. **Contextual**: vários insumos (objeto ativo + objetos externos); o corte se coloca em um só dos objetos.
  - Serve para preparar as pontas "antes de aplicar uma conexão de solda".
  - *Slots* (rasgos para passagem de membros) e *cutouts* (aberturas) também vêm de catálogo.
  - *Coping* (não está na barra por padrão) apara placas e membros pela espessura de elementos que se cruzam, calculando sobre a superfície moldada (espessura ignorada). Estratégias: ponta curta e ponta longa (placas), *Remove* (membros).

- **SolidWorks** [C]: não há comando de entalhe por perfil; usa-se *End Trim* com corpos ou *Combine*.

- **Inventor** [V]: *Notch* corta o membro receptor com a forma do membro-entalhe, usando perfis de entalhe básicos ou personalizados. A geração do perfil é publicada junto da família, com cotas ligadas aos parâmetros. Dependências cíclicas entre tratamentos geram erro ("Cyclic dependency").

- **Outros:** Onshape [V]: corner = mitered, butt ou **coped**.

- **Fenkra hoje:** `Encaixe` (`lib.rs:3824`–`3840`) já recorta a ponta do membro 2 com o contorno do membro 1 (sólido do outro, com folga). Isso equivale a *cope* para casos de pontas coincidentes. Falta:
  - entalhe de perfil nomeado (por exemplo, "boca de jacaré" para tubo redondo, recorte para mesa de viga I);
  - recorte "passante" para T no meio;
  - padrão de furo/ranhura de passagem (*slot*).
  - Backlog B37 já lista "entalhe por perfil".

- **Melhor prática:**
  - Fluxo: **suporte** (membro receptor + membro-forma) → **modo** (cope pelo contorno | perfil de catálogo) → **parâmetros** (folga, profundidade) → **avançado** (entalhe publicado).
  - Regras: o cope deve ser simétrico quando os dois lados são iguais; a folga é aplicada ao **contorno** (offset de superfície), não ao plano.
  - Testes:
    - tubo redondo Ø48,3 contra tubo redondo Ø60,3: o corte tem a forma esperada (volume removido comparado ao de interseção);
    - mudar o diâmetro do outro tubo atualiza o corte (recompute).

---

### 2.11 Gerenciador de cantos e ordem de corte (CATIA: Cutback sequence / SW: Corner Management (Simple/Complex Corner) / Inventor: Miter + Corner Joint por par / outros: Onshape Corner Override, Creo "advanced joints" / Fenkra: `joints: Vec<StructJoint>`)

- **O que faz:** mostra todos os cantos da estrutura, deixa agrupar cantos parecidos, escolher o tratamento e a **ordem** (quem prevalece) em nós de 3 ou mais membros.

- **CATIA V5** [V]: as regras de sequência aparecem como restrições no manual (anel de 4 membros). Os tratamentos são feitos par a par, cada um é um nó na árvore e entra no update.

- **SolidWorks** [V]:
  - *Corner Management* (Structure System, 2023+): os cantos semelhantes são **agrupados** e o tratamento vale para o grupo; é possível tirar um canto do grupo ou dissolver o grupo.
  - Duas famílias: *Simple Corner* (dois membros) e *Complex Corner* (três ou mais), com *Body Trim* e *Planar Trim* (primeiro contato ou contato total), folga, ignorar tratamento e permitir extensão.
  - O programa escolhe um **plano base** que determina a ordem de corte; o usuário altera o plano base, os grupos e o tratamento.
  - 2023: o canto só é trimado quando é selecionado, não sempre que a feature é criada/editada.
  - 2025: o painel foi reorganizado; os balões no gráfico definem tipo, folga e ordem.
  - No weldment clássico: tratar o canto editando a feature e selecionando o ponto magenta; a *Trim Order* define quem prevalece.

- **Inventor** [V]: tratamentos por par de membros; *Delete existing end treatments* na maioria dos comandos; menu de contexto de cada tratamento: *Edit*, *Delete*, *Delete Half of the End Treatment*, *Isolate*. Cada tratamento é um filho do membro na árvore e gera `CUTDETAIL(n)`.

- **Outros:** Onshape [V]: override por junta. Creo AFX [V]: juntas básicas e avançadas.

- **Fenkra hoje:**
  - `st_ui.rs:106` (`joints_ui`) lista os cantos encontrados no último cálculo (`member_joints`, `lib.rs:5014`) e permite escolher o modo por par: padrão | Meia-esquadria | Aparar | Encaixe | Nenhum, com "membro 1" e folga.
  - A regra do comando **posterior** prevalece (`joint_rule`, `lib.rs:1816`), com herança do padrão do comando.
  - Falta: agrupamento de cantos semelhantes, aplicar a vários de uma vez, ordem explícita em nó de 3+ membros (hoje é a ordem de criação), "ignorar", e "desfazer metade do tratamento".
  - **Limitação:** `member_joints` é calculado só quando o painel de um membro está aberto (`st_ui.rs:197`). Não há uma janela de "todos os cantos da estrutura".

- **Melhor prática:**
  - Fluxo: **lista de cantos** (filtro por nó, tipo, perfil) → **seleção múltipla/grupo** → **tratamento** → **ordem/dominante** → **folga**.
  - Regras:
    - A regra de canto fica num registro único por par, com origem visível ("herdado do comando X"/"manual").
    - Em nó de 3+ membros, a dominância é uma lista ordenada e explícita, visível.
    - Operação "aplicar a cantos semelhantes" (mesmo perfil, mesmo ângulo).
  - Testes:
    - nó em T com três membros: mudar o dominante troca quem atravessa;
    - quadro de 4 cantos: aplicar `Aparar` a todos produz 4 cantos consistentes, sem o caso "anel de 4" que falha no CATIA.

---

### 2.12 Placa de extremidade e tampa (CATIA: End Plate / SW: End Cap / Inventor: Insert End Cap / outros: Creo "beam end plates" / Fenkra: `StructPlate`)

- **O que faz:** fecha ou apoia a ponta de um membro com uma chapa (tampa do tubo, placa de base, flange de ligação).

- **CATIA V5** [V]:
  - *End-Plate*: material, comprimento/largura/espessura **no eixo local do perfil** (em uma viga I, largura ao longo da mesa e comprimento ao longo da alma), seleção da ponta, seta de direção de extrusão (inverter), prévia.
  - Seleção múltipla com Ctrl cria várias placas idênticas de uma vez, e a direção/características valem para todas.

- **SolidWorks** [V]:
  - *End Cap*: seleciona-se a face aberta; espessura **para fora**, **para dentro** ou **interna** (dentro do tubo, com recuo).
  - *Offset* da borda do membro até a borda da tampa, com opção de **razão de espessura** (0 a 1) em vez de valor.
  - Chanfro ou filete nos cantos da tampa.
  - O tutorial da CATI diz: razão 0,5 = metade da espessura do material, 0 = tamanho exato.

- **Inventor** [V]: *Insert End Cap* é comando separado; o ambiente de soldagem trata placas como peças normais. Cada tratamento gera `CUTDETAIL`.

- **Outros:** Creo AFX [V]: conectores, placas de ligação, placas de extremidade de viga e equipamentos (escadas, guarda-corpos).

- **Fenkra hoje:**
  - `StructPlate { at: [f64;3], thickness, margin }` (`doc.rs:1468`).
  - Execução (`lib.rs:3878`): `member_end(pl.at, f)?` escolhe a ponta **mais próxima de `at`** entre os membros de comandos **anteriores** (`lib.rs:2148`). A placa é uma caixa `(w + 2·sobra) × (h + 2·sobra) × t`, para fora, na ponta (somando `ext`).
  - **Bug provável 8 (referência silenciosa):** `member_end` nunca testa distância. Se o esqueleto muda e a ponta original se afasta, a placa passa para **outra** ponta qualquer, sem erro (viola ADR 0010/0011, princípio 1). Além disso, `at` é guardado só como `PointRef::Fixed` (`app.rs:1543`): clicar um ponto referenciado vira coordenada fixa.
  - A placa não trata a ponta mitrada (fica reta, perpendicular à linha de centro) e não tem furos, chanfros, filetes nem modo "interna/ para dentro".
  - Não entra na lista de corte (L9); recebe item e peça na montagem, nomeada só "Placa".

- **Melhor prática:**
  - Fluxo: **suporte** (ponta de membro: referência "membro + ponta") → **modo** (para fora | para dentro | interna) → **parâmetros** (espessura, sobra ou razão, material) → **avançado** (cantos chanfrados/filetados, furos, preparação de solda).
  - Regras:
    - A referência é "membro + extremidade (início/fim)", não coordenada. Se não resolve, o comando falha com mensagem; nunca escolhe outra ponta.
    - A descrição do item deve ser "PL 8×50×50 S235" (ou equivalente), com material e massa.
    - A placa herda o nome do membro (`PL-<membro>`).
  - Testes:
    - mover o ponto final do esqueleto 100 mm: a placa acompanha;
    - remover a linha: erro "referência perdida", sem placa em outro lugar;
    - placa em ponta mitrada: aviso "ponta não perpendicular".

---

### 2.13 Reforço (gusset / nervura) (CATIA: Stiffener / SW: Gusset / Inventor: peça normal (sem comando) / outros: Creo "bracing plates" / Fenkra: `StructGusset`)

- **O que faz:** uma chapa triangular ou poligonal que reforça o encontro entre dois membros ou entre membro e placa.

- **CATIA V5** [V]: o *stiffener* da família estrutural é um objeto próprio, com perfil de seção e *âncora*, em geral posto sobre uma placa (normal à placa) ou numa aresta livre. Os limites podem ser plano, placa, outro stiffener ou superfície; há ângulo da alma e deslocamento do ponto de ancoragem.

- **SolidWorks** [V]:
  - *Gusset*: seleciona-se **duas faces planas**. Perfil triangular (distância 1, distância 2, ângulo) ou poligonal, **alívio em chanfro** (para dar passagem ao cordão de solda) com distância e (distância 2 ou ângulo).
  - Espessura **interna**, **ambos os lados** ou **externa**.
  - Posição **no início, no meio ou no fim** do perfil, ou com deslocamento.
  - O gusset **não entra na lista de corte** por padrão (não tem perfil). A solução da GoEngineer é criar uma *bounding box* na pasta e dar PARTNUMBER/DESCRIPTION [V]. O blog da Javelin trata exatamente da descrição de end caps e gussets na tabela [V, só título].

- **Inventor** [V]: não há comando. Gussets e placas são peças comuns na montagem de soldagem.

- **Fenkra hoje:**
  - `StructGusset { at, leg, thickness, chamfer }` (`doc.rs:1477`).
  - `member_pair` (`lib.rs:2165`): ordena as pontas por distância a `at` e pega a primeira e a segunda que estejam a < 1e-4 mm da primeira. A chapa fica no plano das duas linhas guia (`n = da × db`), triângulo de perna `leg` com chanfro de canto opcional (`chamfer`).
  - Funciona só para dois membros que se encontram **em ponta**, e só no plano das linhas.
  - **Bug provável 9:** com três membros no mesmo nó (empate de distância), a ordem fica a cargo da ordem de inserção. O par escolhido não é determinístico.
  - **Bug provável 10:** reforço sempre no plano das duas linhas, centrado no meio dos membros (usa `max(w,h)/2`, igual à placa). Não há reforço **entre face de membro e placa**, nem espessura externa/interna, nem posição ao longo do membro.
  - O alívio é só um chanfro simétrico no canto da perna; falta alívio de solda real (entalhe junto à solda).

- **Melhor prática:**
  - Fluxo: **suporte** (duas faces/membros/placa) → **modo** (triangular | poligonal | retangular) → **parâmetros** (pernas, ângulo, espessura, lado) → **avançado** (alívio de solda, posição ao longo, deslocamento).
  - Referência por "membro A + membro B + nó", com resolução determinística (menor id primeiro) e erro quando há ambiguidade.
  - Testes:
    - nó com três membros → erro de ambiguidade pedindo o par;
    - alívio com chanfro 10 mm: a face do alívio fica a ≥ 10 mm do cordão;
    - reforço com espessura "lado externo" sobre uma face: ocupa só um lado.

---

### 2.14 Cordões de solda, símbolos e preparação (CATIA: Weld Design / SW: Weld Bead + Fillet Bead + Groove Bead / Inventor: Weld environment (Cosmetic/Fillet/Groove) / outros: Creo Welding, Solid Edge weldment assembly / Fenkra: símbolo ISO 2553)

- **O que faz:** representa a solda: cordão 3D (ou cosmético), símbolo no desenho, preparação (chanfro) das peças, massa e lista de soldas.

- **CATIA V5 — no detalhe** [V]:
  - O produto *Weld Design 1 (WD1)* tem 15 tipos de solda do padrão ISO: 1 de filete, 7 de ranhura e 7 de duplo cordão. É possível declarar **qualquer peça** como representação de solda (para desenho, massa e material).
  - O diálogo é guiado: dependendo do tipo, o usuário escolhe de 2 a 4 geometrias; o mesmo diálogo modifica a solda depois.
  - **Preparação automática** (chanfros nas peças), com comprimento de preparação ajustável (várias peças soldadas na mesma aresta) e opção de limitar à zona de contato.
  - Tudo é **associativo**: mover, substituir, apagar ou modificar peças propaga para a solda e a preparação, criando *features de montagem* associativas ao lado do elemento físico da solda.
  - Massa e inércia da solda entram nos totais quando o material é informado; a solda participa de verificação de colisão; atributos podem alimentar o Knowledgeware.
  - Anotações 2D de solda aparecem automáticas nas vistas, cortes e seções.

- **SolidWorks — no detalhe** [V]:
  - *Weld Bead* (cosmético) é o recomendado; o *Fillet Bead* antigo aceita tipos por lado: contínuo, intermitente, alternado, com tamanho (perna), passo e *propagação por tangência*.
  - O *Weld Bead* tem **geometria de solda** (propagação por tangência), intermitente por "comprimento + folga" ou "comprimento + passo" (passo = comprimento + folga), tamanho e **símbolo de solda** associado.
  - Os cordões ficam numa pasta *Weld* na árvore.
  - SW 2025 traz o *Groove Bead*, para soldas entre faces que não se interceptam (opostas de uma abertura).
  - Na lista de corte, o artigo da CATI diz que os cordões de filete **não são agrupados** e ficam listados individualmente no fim; outra descrição da mesma revenda diz que cordões são excluídos da lista. As fontes divergem; confirmar na versão.
  - No desenho, símbolos de solda entram por *Model Items* [V].

- **Inventor — no detalhe** [V]:
  - O ambiente de soldagem tem três grupos de features: **preparação**, **cordão** e **usinagem pós-solda**.
  - Tipos: **cosmético** (selecionam-se arestas, sem geometria, massa aproximada), **filete sólido** e **ranhura sólida** (selecionam-se faces).
  - Filete: conjuntos de faces, cadeia, orientação, pernas, contorno (convexo/côncavo com deslocamento), intermitente conforme a norma ativa, extensões (Tudo | De-Até | Início-Comprimento).
  - Ranhura: conjuntos de faces 1 e 2, *Full Face Weld*, *Ignore Internal Loops*, direção de preenchimento, preenchimento radial.
  - Símbolos de solda ligados aos valores do filete (a ligação pode ser quebrada). Cordões são afetados pela usinagem pós-solda e **não** pela preparação.
  - Estratégia oficial: preferir cosméticos para desempenho; a massa aproximada entra com "Include Cosmetic Welds" nas iProperties. Filete sólido atualiza **símbolos e anotações** do desenho; cosmético só os símbolos.
  - **Vistas de estado de soldagem** (preparação, solda, usinagem) saem do agrupamento das features.

- **Outros:** Solid Edge [V]: montagem de soldagem para preparação, cordões e pós-solda. Creo AFX [V]: *Weld Groups* com desenhos e BOM próprios. NX [V]: o Weld Assistant é comparado em fórum PTC, sem detalhes lidos.

- **Fenkra hoje:**
  - O catálogo registra "símbolo ISO 2553 no desenho/PMI" (PMI-02/EST-05) como 🟡: **não há cordão 3D**, nem preparação, nem lista de soldas, nem vínculo entre símbolo e geometria.
  - O parâmetro `gap` (folga de solda) existe só como afastamento de montagem entre membros.
  - Há `stock_length` e a nota de emenda, mas nenhum dado de processo (WPS, tratamento térmico) — a visão de `docs/contextos/estrutural.md` (§4.2) prevê processo e tratamento térmico por submontagem.

- **Melhor prática:**
  - Fluxo: **suporte** (arestas para cosmético; faces para sólido) → **modo** (cosmético | filete | ranhura; contínuo | intermitente | alternado) → **parâmetros** (perna/garganta, passo, comprimento, processo, material) → **avançado** (preparação, símbolo, solda de campo, propagação por tangência).
  - Regras:
    - O cordão cosmético é o padrão (barato, entra na lista de soldas e na massa aproximada); o sólido é opção por desempenho.
    - Todo cordão tem referência por **par de faces/arestas** (ADR 0010) e falha explícita.
    - O símbolo é derivado do cordão. Não existem símbolos "soltos" para soldas modeladas.
  - Testes:
    - filete cosmético de 5 mm em 4 cantos do quadro: lista de soldas com 4 linhas, comprimento total correto;
    - mudar o comprimento do membro atualiza o comprimento do cordão;
    - remover a aresta de referência → cordão "falhou" com motivo.

---

### 2.15 Lista de corte e propriedades (CATIA: BOM/relatórios da estrutura / SW: Cut List + propriedades customizadas / Inventor: Frame Member Info + Parts List (cut length) / outros: Creo "optimized cut list tables", Onshape Cut List, Solid Edge cut lengths list / Fenkra: `cut_list`, `CutRow`)

- **O que faz:** agrupa os membros iguais e lista, por item, perfil, comprimento de corte, ângulos de ponta, quantidade, material, massa e outros atributos, para compra, corte e desenho.

- **CATIA V5** [V]:
  - O produto gera BOM customizável. O Frame Structure Designer (3DEXPERIENCE) calcula comprimento, superfície, volume e peso e gera relatórios por modelo da empresa.
  - O atributo *tipo* (viga/coluna/travessa) vem do dicionário de features e pode ser editado no diálogo ou nas propriedades.

- **SolidWorks — no detalhe** [V]:
  - A pasta *Cut List* é gerada na criação e **atualizada manualmente**; o usuário escolhe manual ou automático (menu de contexto).
  - Corpos idênticos ficam agrupados em uma pasta (item).
  - **Propriedades automáticas**: LENGTH, ANGLE1, ANGLE2, DESCRIPTION (herdada do perfil), MATERIAL (quando atribuído).
  - **Caixa envolvente (bounding box)** com espessura, largura, comprimento e volume, para chapas e outros sólidos (descrição "Plate"/"Sheet" configurável).
  - Propriedades customizadas (PARTNUMBER, DESCRIPTION…) por item, ou definidas na feature de soldagem e propagadas a todos os itens.
  - Só propriedades listadas na aba *Cut List Summary* aparecem na tabela do desenho.
  - Falha conhecida: propriedades perdidas em configurações [V, fórum].
  - Sub-soldagens aparecem como subpastas.

- **Inventor — no detalhe** [V]:
  - *Frame Member Info* mostra seção, propriedades físicas e de engenharia, posição de inserção e orientação.
  - O comprimento do membro com miter nas duas pontas é calculado automaticamente (*Driven Length*).
  - A lista de peças sai do *Drawing Manager Parts List*. Cada tratamento grava `CUTDETAIL(n)` — textos que viram notas de fabricação na lista.

- **Outros:** Creo AFX [V]: tabelas de comprimento de estoque e tabelas de **corte otimizado**. Solid Edge [V]: lista de **comprimentos de corte** com balões, opção *Frame Rough-cut End Clearance* que soma folga de corte bruto. Onshape [V]: cut list embutida alimenta a BOM; um usuário reporta falha quando ambas as pontas foram editadas (comunidade, anedótico).

- **Fenkra hoje:**
  - `CutRow { feature, designation, length, count, bars, item, members }` (`lib.rs:4743`).
  - `cut_list` (`lib.rs:4954`) tira os grupos de `structural_groups`, filtra `Member`, e usa `signature.length_mm()`, que vem de `member_length` = distância da linha de centro + prolongamentos explícitos (`lib.rs:4945`).
  - `bars = ceil(len / stock)` com um único `stock_length` global do documento (`lib.rs:4971`–`4972`). Não há perda de serra, sobra, nem agrupamento de cortes em barras (EST-09 🟡).
  - Há aviso de emenda (`lib.rs:3847`).
  - **Bug 11 (confirmado por leitura):** comprimento = linha de centro; o corte real é a ponta longa (ver 2.7). Não há `ANGLE1/ANGLE2`, nem material, nem massa, nem bounding box por item, nem `CUTDETAIL`.
  - **Bug 12:** placas e reforços ficam de fora de `cut_list` (L9). Como `item_centers` (`lib.rs:4928`) parte de `cut_list`, **não têm balões** (DR-33).
  - **Risco de desempenho 13:** `cut_list` chama `structural_groups` (que recalcula o documento inteiro com `final_shape`); `item_centers` chama `structural_groups` **e** `cut_list`, ou seja, dois recomputes completos por chamada. Chamado de `dwg_ui.rs:352` — medir.

- **Melhor prática:**
  - Fluxo: **agrupamento** (perfil + comprimento de corte + formato das pontas + material) → **propriedades por item** (descrição, material, massa, comprimento de corte, ângulos de ponta, detalhes de corte) → **matéria-prima** (barra padrão por família, perda de serra) → **plano de corte** → **exportar** (CSV/tabela no desenho).
  - Regras:
    - Comprimento de corte = **ponta longa** (maior distância entre as pontas ao longo do eixo); mostrar também comprimento da linha de centro.
    - Ângulos de ponta (corte no plano da seção e no plano normal) calculados da geometria final.
    - Placas, reforços e (opcional) cordões entram como itens, com bounding box e descrição.
    - Cache de `structural_members` por versão do documento (calcular uma vez).
  - Testes:
    - quadro 400×300 com tubo 40×3 em meia-esquadria: itens 440 (×2) e 340 (×2), ANGLE1=ANGLE2=45°;
    - mesmo quadro com `Aparar`: comprimentos diferentes entre os membros 1 e 2;
    - placa 50×50×8 entra na lista com massa 0,157 kg (aço 7,85 g/cm³);
    - doc com 100 membros: `item_centers` faz um único recompute.

---

### 2.16 Corpos iguais, itens e reutilização (CATIA: instâncias de Product / SW: pastas da Cut List / Inventor: Reuse / outros: — / Fenkra: `StructSignature`, `group_members`)

- **O que faz:** detecta membros idênticos para virarem **uma peça com N ocorrências**, com numeração de item estável.

- **CATIA V5** [C]: não existe detecção; o usuário monta um `Product` e instancia a mesma peça. O guia fala em "caches" de estrutura e "peças resolvidas" para as seções [V].

- **SolidWorks** [V]: a pasta da lista de corte agrupa corpos de mesmo perfil/comprimento/ângulos. As propriedades vivem na pasta, não no corpo.

- **Inventor** [V]: *Reuse* só vale para membros **retos**, com a mesma geometria, e compartilha um único arquivo; *Break Reuse* torna independente. A ajuda não descreve o critério de identidade [V, explícito]. Há a nota de que um membro com miter fica assimétrico e erra se reutilizado em espelho [V].

- **Fenkra hoje:**
  - `StructSignature { kind, profile, length, volume, bbox }` (`lib.rs:4776`), calculada **no referencial local** do membro: Z ao longo do eixo, origem na ponta inicial (`log_struct`, `lib.rs:4871`). Comprimento, volume e *bbox* em décimos de mm (`q10`).
  - `group_members` ordena num `BTreeMap` pela assinatura e numera 1, 2, … (`lib.rs:4904`).
  - **Bug provável 14 (agrupar errado):** a assinatura não "enxerga" o formato das pontas. Um trapézio (miter nos dois lados em sentidos opostos) e um paralelogramo (mesmo sentido) têm volume e *bbox* locais iguais [C, argumento geométrico: o miter conserva volume e a caixa é simétrica em x]. Seriam fundidos num item.
  - **Bug provável 15 (separar demais):** um membro e o mesmo membro com o sentido da linha invertido têm *bbox* diferente (o canto longo fica em z<0 num e em z>L noutro). Viram dois itens, embora sejam a mesma peça virada. O teste `estrutura_agrupa_pecas_iguais` só cobre quatro lados de um retângulo.
  - **Instabilidade de numeração (L3):** o número do item é a posição na ordem de uma `BTreeMap` cuja chave começa por `kind` e pelo texto do perfil. Mudar um comprimento reordena e **renumera** itens. O nome do arquivo da montagem (`item{NN}_…`, `estrutura.rs:68`) muda junto, deixando arquivos órfãos (B50 já fala em limpeza).

- **Melhor prática:**
  - Fluxo: **normalização** (canonizar o sentido da linha: do menor para o maior ponto de ponta longa ou por regra fixa) → **assinatura** (perfil, material, comprimento de corte, **ângulos e normais dos planos de ponta**, furos/entalhes) → **agrupar** → **numerar de forma estável**.
  - Regras:
    - A numeração é guardada no documento: mapa `assinatura → número`. Itens novos recebem o próximo número livre; itens que somem liberam o número só ao "compactar" (comando explícito).
    - Tolerância configurável (0,1 mm padrão).
  - Testes:
    - paralelogramo ≠ trapézio;
    - membro invertido == membro original;
    - alterar o comprimento de um item não renumera os outros;
    - gerar a montagem duas vezes dá os mesmos nomes de arquivo.

---

### 2.17 Submontagem de solda / sub-soldagem (CATIA: Product/Weld Design assembly / SW: Sub-weldment / Inventor: Weldment assembly / outros: Creo Weld Group / Fenkra: ausente)

- **O que faz:** seleciona um subconjunto de membros/placas que formam um módulo soldado (painel, pé, cavalete) e o trata como submontagem: lista própria, desenho próprio, processo de solda, tratamento térmico.

- **CATIA V5** [V]: WD1 trabalha em montagem: peças soldadas num *Product*, com features de preparação e soldas associativas.

- **SolidWorks — no detalhe** [V]:
  - Selecionam-se corpos na lista de corte e cria-se uma **sub-soldagem**; ela vira uma subpasta da lista de corte (e na tabela do desenho aparece uma linha por sub-soldagem).
  - *Insert into New Part* gera uma peça separada, **ligada** à original para atualizar; as propriedades de lista de corte acompanham.
  - Para mostrar os membros individuais no desenho principal, faz-se uma lista de corte da própria sub-soldagem depois de inserida em nova peça (blog Javelin).
  - A CATI recomenda atualizar a lista de corte novamente após criar a sub-soldagem. Há relato de perda de propriedades (fórum).

- **Inventor** [V]: ambiente de soldagem de montagem (preparação, cordão, usinagem). Pode haver sub-montagens de quadro com *Promote/Demote*; os membros passados fora do subconjunto saem da solução do Frame Generator.

- **Outros:** Creo AFX [V]: *Weld Groups* com desenhos e BOM próprios.

- **Fenkra hoje:** não existe. `docs/contextos/estrutural.md` (§4) pede submontagens de solda selecionáveis com nome, código, material, processo e tratamento térmico. A montagem gerada tem apenas grupos por item (`asm.add_group("Item NN · …")`, `estrutura.rs:93`). O catálogo (EST-08) e o B50 listam "sem submontagens de solda".

- **Melhor prática:**
  - Fluxo: **seleção** (membros/placas/reforços, em 3D ou pela lista) → **criar subgrupo** (nome, código, material, processo) → **resultado**: um objeto "Sub-soldagem" no esqueleto, com filhos referenciados por item (não por cópia).
  - Regras:
    - Um corpo pertence a **no máximo um** subgrupo (aviso se o usuário tenta repetir).
    - O subgrupo gera uma submontagem `.fka` e uma lista própria; o desenho principal mostra uma linha por subgrupo.
  - Testes:
    - dois subgrupos que partilham um membro → erro;
    - mudar uma cota do pai atualiza a submontagem (com link vivo);
    - remover um membro do subgrupo atualiza a lista.

---

### 2.18 Gerar montagem e subpeças a partir da estrutura (CATIA: Product a partir de Part / SW: Insert into New Part, Save Bodies / Inventor: Frame Generator assembly (um arquivo por membro) / Fenkra: `estrutura_para_montagem`)

- **O que faz:** transforma a estrutura mestre (Pai) em montagem + peças individuais para detalhar, orçar e fabricar.

- **CATIA V5** [C]: o fluxo é manual: `Product` com instâncias da peça e *publicações*; no modo de projeto em contexto (*Keep link*), as peças referenciam o pai.

- **SolidWorks** [V]: *Save Bodies* e *Insert into New Part* geram peças ligadas; a lista de corte vira BOM no desenho.

- **Inventor** [V]: o Frame Generator é, por natureza, "montagem + peças por membro": cada membro é um arquivo, com nomes configuráveis (padrão + número de estoque + número único); o `Reuse` evita arquivos duplicados; `Refresh` e `Edit with Frame Generator` mantêm a ligação.

- **Fenkra hoje (`fenkra-montagem/src/estrutura.rs:49`):**
  - Lê o pai do disco, chama `structural_groups`, e para cada grupo gera uma `.fkp` (`LinkedBody` com o B-Rep do primeiro membro no referencial local, `estrutura.rs:72`–`88`) e uma instância fixa por membro com `Placement` do referencial (`estrutura.rs:95`).
  - O link para o pai é criado **congelado** (`LinkState::Congelado`, `estrutura.rs:85`) porque "item NN" não é publicação do pai. A atualização é "gerar de novo".
  - Propriedades gravadas: descrição, material, PN `pai-NN`, `origem=estrutura_pai`, `estrutura_grupo` (chave da assinatura), `estrutura_item`.
  - O teste `estrutura_vira_montagem_com_pecas_agrupadas` valida arquivos, grupos, link relativo, volume total e caixa.
  - **Faltas:**
    - link vivo (B50);
    - nomes estáveis;
    - limpeza de órfãos;
    - detalhe das pontas e furos como features na peça filha;
    - PMI e desenho por peça;
    - material/PN por perfil;
    - submontagens de solda.
  - **Risco 16:** a peça filha carrega a geometria *embutida* (B-Rep), então editar a filha não volta ao pai e editar o pai exige regenerar.

- **Melhor prática:**
  - Fluxo: **estrutura pai** → **grupos estáveis** → **peça filha por item** (link publicado "item NN" **no pai**) → **montagem com instâncias** → **desenho por peça**.
  - Regras:
    - O pai passa a **publicar** "item NN" (ou chave da assinatura). Aí o link nasce vivo e `atualizar links` funciona (princípio 3 do ADR 0011).
    - Arquivo = `PN-NN.fkp` com PN estável. Regerar apaga os órfãos listados (pedindo confirmação).
    - Peça filha tem os cortes de ponta e a descrição do pai.
  - Testes:
    - alterar o comprimento do pai de 400 para 450 → 'atualizar links' atualiza as filhas, sem regerar, e mantém a quantidade;
    - remover um item → o arquivo vira órfão listado.

---

### 2.19 Desenho de estrutura soldada (CATIA: Drafting + Weld annotations / SW: Weldment drawing / Inventor: Frame drawings + weld state views / outros: Creo AFX component drawings / Fenkra: DES-11, balões B26)

- **O que faz:** gera vistas do conjunto soldado e dos itens, com tabela de corte, balões e símbolos de solda.

- **CATIA V5** [V]: as anotações de solda 2D aparecem automáticas nas vistas, cortes e seções; WD1 gera as soldas no desenho. Há um caso conhecido (IBM APAR) em que stiffeners de estruturas não eram projetados no desenho, corrigido em V5R20 SP6/V5R21 SP1.

- **SolidWorks** [V]:
  - Desenho a partir da peça soldada, com vista isométrica e tabela *Weldment Cut List* (Inserir → Tabelas).
  - Vistas por corpo via *Relative View* com escopo "corpos selecionados" (escala personalizada).
  - Símbolos via *Model Items* (fonte = modelo inteiro, desmarcando "marked for drawing").
  - *Auto Balloon* na vista isométrica.
  - A tabela só mostra propriedades listadas em *Cut List Summary*.

- **Inventor** [V]: desenho com lista de peças e balões; **vistas de estado de soldagem** (preparação, solda, usinagem) quando as features estão nos grupos corretos; cordão sólido atualiza símbolo e anotação.

- **Outros:** Creo AFX [V]: criação automática de desenhos de componentes, cotagem de perfis e chapas, tabelas de estoque e de corte otimizado, notas com número de item.

- **Fenkra hoje:** balões automáticos ligados à lista de corte e a `item_centers` (✅ B26); DES-11 🟡. **Faltam:** vista própria por item/corpo, detalhe de ponta, cortes específicos, vistas de orientação do perfil (DR-45), tabela de soldas, estados de soldagem e **balões para placas e reforços** (ver 2.15).

- **Melhor prática:**
  - Fluxo: **folha geral** (isométrica + vistas principais + lista de corte + balões) → **folhas por item** (vista do eixo com cotas de comprimento e ângulos de ponta, corte da seção, detalhe da ponta, orientação do perfil) → **folhas de submontagem**.
  - Regras: as cotas de comprimento e ângulo vêm da propriedade do item (2.15), nunca de medição de pixel.
  - Testes: desenho do quadro com 2 itens + 1 placa: 3 balões; mudar o comprimento atualiza a tabela e a cota da folha do item.

---

### 2.20 Atualização, diagnóstico e referência perdida (CATIA: Update/Diagnosis, árvore de especificação / SW: What's Wrong / Inventor: sick elements + Frame Generator errors / Fenkra: ADR 0010/0011)

- **O que faz:** quando algo no esqueleto muda ou some, mostra o que quebrou, onde e por quê, e oferece correção, sem destruir o resto.

- **CATIA V5** [V]:
  - Cada membro e cada *cutback* é um nó da árvore, recalculado no update. O cutback "é recomputado automaticamente" se o membro muda.
  - O estado de update por feature e o diálogo de diagnóstico aparecem na árvore [C].
  - Regras de dependência explícitas: *Keep link with selected object*, *Synchronize external references*.

- **SolidWorks** [V]: o canto só é trimado quando selecionado (2023); a lista de corte atualiza manualmente ou em automático.

- **Inventor — no detalhe** [V]:
  - Elementos "doentes" ganham ícone com **motivo ao passar o mouse**. O manual lista, por comando, as causas:
    - esqueleto removido;
    - geometria de referência ausente;
    - linhas colineares no esqueleto;
    - membros ausentes;
    - face de corte paralela ao membro;
    - **dependência cíclica** entre entalhes;
    - tratamentos prévios conflitantes (suprimir/reativar para isolar);
    - falha de varredura por autointerseção;
    - diretório sem permissão de escrita ao promover/rebaixar.
  - Para membros ausentes: reconstruir, remover ou ignorar (ignorar pode desassociar cortes relacionados).
  - Há *Isolate* para ver só os membros de um tratamento.
  - Uma falha **não derruba** os outros membros.

- **Fenkra hoje:**
  - Faces/arestas têm "referência perdida" (ADR 0010, fase 1) com mensagem de `Ctx::lost` (`lib.rs:1834`).
  - No estrutural: `member_end` e `member_pair` devolvem erro só se **não existe nenhum** membro (`lib.rs:2161`) ou o par não existe (`lib.rs:2176`); fora isso, escolhem em silêncio (L1).
  - Erros de linha abortam o comando todo (L5).
  - Sem `isolar`, sem `desativar tratamento` por par (só "Nenhum"), sem painel de diagnóstico estrutural (B59 prevê um geral).

- **Melhor prática:**
  - Fluxo: **status por linha/por canto/por item** → **diagnóstico** (motivo e geometria envolvida em destaque) → **ações** (Editar, Substituir, Isolar, Desativar, Reordenar), como no ADR 0011.
  - Mensagens padronizadas, em português, nomeando membro e linha ("Linha 3 de 'Quadro' ficou com comprimento zero").
  - Testes:
    - esqueleto com 1 linha de comprimento zero → só esse membro em erro;
    - ciclo de entalhes A↔B → erro de dependência nomeando os dois;
    - face de corte paralela → erro com a causa.

---

## 3. Requisitos levantados

Prioridades: P0 = corrige erro de identidade/dado; P1 = lacuna de funcionalidade essencial; P2 = importante; P3 = desejável.

### Funcionais

| ID | Requisito | Pri. | Critério de aceite (testável) |
|---|---|---|---|
| RF-EST-01 | Falha isolada por linha: se uma linha do esqueleto some, fica degenerada ou colinear, só o membro dela vai a "falhou", com motivo; os demais são gerados. | P0 | Quadro de 4 linhas, remover a linha 2 → 3 corpos, estado do comando = aviso, mensagem cita "linha 2". |
| RF-EST-02 | Placa e reforço referenciam "membro + extremidade" (e "membro A + membro B + nó"), não coordenada. Sem tolerância silenciosa. | P0 | Mover o ponto final do esqueleto 100 mm → a placa acompanha. Apagar a linha → erro "referência perdida"; nenhuma placa em outra ponta. |
| RF-EST-03 | Lista de corte com comprimento de corte (ponta longa), comprimento da linha de centro e ângulos de ponta. | P0 | Quadro 400×300, tubo 40×3, meia-esquadria: 440/340 e 45°/45°. Com `Aparar`: comprimentos distintos entre membro 1 e 2. |
| RF-EST-04 | Assinatura de grupo por congruência: inclui geometria das pontas e normaliza o sentido da linha. | P0 | Trapézio ≠ paralelogramo (itens separados). Membro invertido == original (mesmo item). |
| RF-EST-05 | Numeração de item estável, guardada no documento. | P1 | Alterar o comprimento de um item não renumera os demais; regerar a montagem mantém os nomes de arquivo. |
| RF-EST-06 | Placas e reforços entram na lista de corte e recebem balões no desenho. | P1 | Quadro + placa + reforço: a lista tem 3 tipos de linha e o desenho mostra os balões dos 3. |
| RF-EST-07 | Detecção de cantos com tolerância configurável e aviso "ponta quase encostando". | P1 | Esqueleto com folga de 0,05 mm entre pontas: com tolerância 0,1 há canto; com 0,01 há aviso. |
| RF-EST-08 | Aparar/estender até plano, face, corpo ou outro membro, com deslocamento e opção de só aparar. | P1 | Travessa terminando no meio do montante: sólido sem interseção; face paralela → erro com motivo. |
| RF-EST-09 | Meia-esquadria com modos bisseção e rente (full flush) para seções de tamanhos diferentes. | P1 | Tubos 60 e 40: modo rente alinha a face externa (≤ 0,01 mm); bisseção deixa degrau. |
| RF-EST-10 | Distância até a face do outro membro calculada do sólido, não de `max(w,h)/2`. | P1 | Tubo 80×40 em T e L, ambas as orientações: face de contato com erro ≤ 0,01 mm. |
| RF-EST-11 | Entalhe por perfil publicado (ex.: boca de jacaré, recorte de mesa) além do encaixe pelo contorno. | P2 | Tubo redondo contra tubo redondo: volume removido bate com a interseção ±1%; mudar o diâmetro do outro atualiza o corte. |
| RF-EST-12 | Janela "Cantos da estrutura": todos os cantos, agrupamento de semelhantes, tratamento em lote, ordem/dominância em nós de 3+ membros, "ignorar". | P1 | Aplicar `Aparar` aos 4 cantos do quadro de uma vez; mudar o dominante num nó T com 3 membros troca quem atravessa. |
| RF-EST-13 | Grupos de orientação: rotação/espelho/ancoragem por linha ou grupo no mesmo comando. | P1 | Cantoneiras de um quadro orientadas para dentro independentemente do sentido de desenho das linhas. |
| RF-EST-14 | Prolongar/encurtar por ponta; o prolongamento não se aplica a vértices internos de cadeia. | P1 | Polilinha de 3 trechos com `ext1=10` e cantos `Nenhum`: só o início do 1º trecho muda. |
| RF-EST-15 | Ângulo mínimo de quebra de polilinha configurável; vértice de fechamento tratado. | P2 | Polilinha com dobra de 8° respeita a configuração; polígono fechado gera N trechos e N cantos. |
| RF-EST-16 | Dados completos de perfis ABNT/AISC/EN com fonte e nível; peso/m conferido. | P1 | Para cada perfil de fábrica, massa linear calculada ≈ catálogo ±1% em teste automático. |
| RF-EST-17 | Perfil personalizado por croqui, com âncoras e arestas nomeadas. | P2 | Criar L assimétrico por croqui e trocá-lo por outro L: âncora e regras de canto preservadas. |
| RF-EST-18 | Conjunto geométrico "Esqueleto" e publicação das linhas. | P1 | Criar membro oferece mover linhas ao conjunto; a estrutura referencia linhas publicadas. |
| RF-EST-19 | Grade 3D (cartesiana/polar; relativo/absoluto) com planos, linhas e pontos referenciáveis. | P2 | Grade 3×2×1, membros nas linhas; mudar o espaçamento X atualiza os comprimentos e nenhum membro fica solto. |
| RF-EST-20 | Placa/tampa com modos fora/dentro/interna, razão de espessura, chanfro/filete de cantos e furos. | P1 | Tampa interna num tubo 40×3 com razão 1: encaixa no furo interno ±0,01 mm. |
| RF-EST-21 | Reforço com perfil triangular/poligonal, lado (interno/ambos/externo), posição ao longo e alívio de solda. | P1 | Reforço "lado externo" ocupa só um lado; alívio de 10 mm deixa a face a ≥ 10 mm do cordão; nó com 3 membros → erro de ambiguidade. |
| RF-EST-22 | Cordão cosmético e sólido (filete/ranhura), contínuo/intermitente, ligado a arestas/faces, com comprimento e massa. | P2 | 4 filetes de 5 mm no quadro → lista de soldas com 4 linhas e comprimento total correto; mudar o membro atualiza. |
| RF-EST-23 | Símbolo de solda derivado do cordão (ISO 2553/AWS), com solda de campo. | P1 | Alterar a perna do cordão atualiza o símbolo no desenho. |
| RF-EST-24 | Preparação de junta (chanfro) automática conforme o tipo de solda. | P3 | Ranhura em V gera chanfro de 30°±1° nas duas peças. |
| RF-EST-25 | Propriedades por item: descrição, material, massa, comprimento, ângulos, caixa envolvente, atributo de tipo e detalhes de corte. | P1 | Item mostra todas as propriedades; as de desenho vêm da mesma fonte. |
| RF-EST-26 | Plano de corte 1D (barra padrão por família, perda de serra, sobra) e exportação CSV. | P1 | 10 cortes de comprimentos conhecidos, barra 6000, serra 3: número de barras = ótimo ou ≤ ótimo+1; soma de sobras confere. |
| RF-EST-27 | Sub-soldagem: seleção de membros/placas/reforços vira subgrupo com nome, código, material e processo, que gera submontagem e lista próprias. | P1 | Dois subgrupos sem membros em comum; um membro em dois subgrupos → erro. |
| RF-EST-28 | Montagem viva: o pai publica "item NN"; filhas atualizam por `atualizar links`. | P1 | Mudar o comprimento de 400 para 450: filhas atualizam sem regerar; quantidade preservada. |
| RF-EST-29 | Nomes de arquivo estáveis na geração; órfãos listados e removíveis com confirmação. | P1 | Regerar duas vezes → mesmos nomes; item removido → arquivo listado como órfão. |
| RF-EST-30 | Desenho por item: eixo com comprimento e ângulos de ponta, corte da seção, detalhe de ponta, orientação do perfil. | P1 | Folha do item mostra as cotas da propriedade 2.15; mudar o comprimento atualiza a cota. |
| RF-EST-31 | Vistas de estado de soldagem (preparação, solda, usinagem) e tabela de soldas no desenho. | P2 | Solda sólida atualiza símbolo e anotação; cosmética só o símbolo. |
| RF-EST-32 | Diagnóstico estrutural com motivo (colinear, linha perdida, face paralela, ciclo) e ações Editar/Substituir/Isolar/Desativar. | P1 | Cada causa da lista (2.20) gera mensagem específica nomeando membro e linha. |
| RF-EST-33 | Isolar um tratamento e "remover metade" do tratamento de um canto. | P2 | Isolar mostra só os membros do canto; remover metade deixa o outro membro tratado. |
| RF-EST-34 | Aviso de sobreposição entre membros não resolvida (interferência). | P2 | Membro em T sem tratamento mostra aviso "interferência de 12 cm³ com Membro.2". |
| RF-EST-35 | Massa e propriedades físicas da estrutura sem dupla contagem; massa dos cordões somada. | P2 | Massa da estrutura = soma das peças; T com `Aparar` não duplica. |
| RF-EST-36 | Trocar o perfil de um membro (replace) mantendo âncora, orientação e cantos. | P2 | Trocar tubo 40×3 por 50×3: cantos refeitos, referências preservadas. |
| RF-EST-37 | Exportação de dados de aço (DSTV/SDNF/IFC) e de plano de corte. | P3 | Arquivo de exportação reabre com os mesmos perfis e comprimentos. |
| RF-EST-38 | Membro por dois pontos / ponto + direção + comprimento / ponto até limite. | P2 | Cada modo gera a mesma barra do equivalente por linha. |
| RF-EST-39 | Membro curvo/torcido normal a uma superfície de referência. | P3 | Perfil sobre varredura mantém a seção normal à superfície ao longo do caminho. |
| RF-EST-40 | Cortes de catálogo paramétricos (end cut, slot, cutout) com insumos contextuais. | P3 | Boca de jacaré dimensiona-se pelas fórmulas e se refaz quando o membro muda. |
| RF-EST-41 | Atributos customizados de membro (tipo: viga, coluna, travessa, contraventamento). | P1 | Atributo editável e visível na lista de corte e no desenho. |
| RF-EST-42 | Regra de orientação pública e determinística, com Inverter e Redefinir e prévia 3D. | P1 | Mesma estrutura desenhada em sentidos opostos dá orientação idêntica; Inverter troca e Redefinir volta. |
| RF-EST-43 | Dividir e unir membros (split/merge). | P2 | Dividir por plano gera dois membros com features herdadas; unir exige tangência. |
| RF-EST-44 | Quebrar reuso: tornar a peça de um item independente do pai. | P2 | Após quebrar, editar o pai não altera a peça; o link vira "quebrado" visível. |
| RF-EST-45 | Tamanho de perfil por configuração. | P3 | Configuração "leve" troca 40×3 por 30×2 em todos os membros do grupo, e a massa muda. |

### Não funcionais

| ID | Requisito | Pri. | Critério de aceite |
|---|---|---|---|
| RNF-EST-01 | Determinismo: nenhuma escolha depende de ordem de inserção ou de empate de distância. | P0 | Executar 100× o mesmo documento produz a mesma lista, o mesmo item e os mesmos nomes. |
| RNF-EST-02 | Desempenho: sem recomputes redundantes; `structural_members` em cache por versão do documento; evitar O(n²) em `all_member_lines`. | P1 | 200 membros retos em meia-esquadria: recálculo < 2 s e `item_centers` dispara 1 recompute (meta [C]; medir antes de fixar). |
| RNF-EST-03 | Tolerâncias explícitas, nomeadas e configuráveis (ponta coincidente, agrupamento, ângulo de quebra). | P1 | Nenhum limite numérico mágico no código sem constante nomeada e documentada. |
| RNF-EST-04 | Nunca escolher em silêncio (ADR 0011 princípio 1). | P0 | Teste: toda referência estrutural não resolvida gera estado "falhou" com texto. |
| RNF-EST-05 | Compatibilidade de arquivos: novos campos com `serde(default)`; arquivos antigos abrem e são migrados no 1º recompute. | P0 | Abrir arquivo gravado hoje: mesma geometria e mesma lista. |
| RNF-EST-06 | Massa e volume sem dupla contagem de sólidos sobrepostos em `compound`. | P1 | Dois membros sobrepostos: aviso de interferência, massa total não soma a interseção duas vezes. |
| RNF-EST-07 | Textos em português do Brasil, com i18n; unidades mm/kg. | P2 | Nenhuma string estrutural fora de `tr`. |
| RNF-EST-08 | Corpus de testes (B53): quadro, cavalete, treliça, guarda-corpo, escada, pórtico com mudança de cota "ida e volta". | P1 | Cada peça recomputa, mede, salva/abre e mantém lista de corte idêntica após mudar e voltar uma cota. |
| RNF-EST-09 | Fidelidade dos perfis: massa linear ±1% do catálogo. | P1 | Teste automático para todas as designações de fábrica. |
| RNF-EST-10 | Rastreabilidade das fontes dos perfis (norma, edição, nível, licença). | P1 | Cada linha da biblioteca tem fonte; teste que falha se faltar. |
| RNF-EST-11 | Desfazer/refazer e salvar/abrir mantêm ids estáveis de membros, itens e cantos. | P1 | Salvar, abrir, desfazer 5 passos: a lista de corte e os números de item não mudam. |
| RNF-EST-12 | Painéis seguem suporte → modo → parâmetros → avançado (B20), com prévia 3D. | P2 | Revisão visual de cada painel estrutural conforme o padrão. |

---

## 4. Itens de backlog propostos

IDs provisórios `BE-nn` (o lead renumera). Tamanho: P ≤ 100 k tokens, M ≤ 250 k, G > 250 k (escala do backlog atual).

| ID | Título | O que muda | Pri. | Tam. | Dependências |
|---|---|---|---|---|---|
| BE-01 | **Falha isolada por linha e diagnóstico estrutural** | `members()` devolve estado por linha; comando entrega corpos válidos + aviso; mensagens nomeadas (colinear, linha perdida, face paralela, ciclo). | P0 | M | B59, ADR 0011 |
| BE-02 | **Placa/reforço por referência** | `StructPlate`/`StructGusset` passam a guardar membro+extremidade (e A+B+nó); remove `member_end` por proximidade; migração dos arquivos antigos. | P0 | M | ADR 0010, BE-01 |
| BE-03 | **Comprimento de corte e ângulos de ponta** | `CutRow` ganha ponta longa, linha de centro, ANGLE1/2; coluna na janela, no desenho e na montagem. | P0 | M | — |
| BE-04 | **Assinatura por congruência e item estável** | Normaliza sentido, inclui geometria das pontas; mapa `assinatura→número` guardado no documento; nomes de arquivo estáveis. | P0 | G | BE-03, B50 |
| BE-05 | **Placas e reforços na lista e nos balões** | `cut_list` inclui `Plate`/`Gusset` com descrição "PL e×l×c", material e massa; `item_centers` os cobre. | P1 | P | BE-04 |
| BE-06 | **Aparar/estender até limite** | Novo `StructTrim`: plano, face, corpo ou membro; deslocamento; "só aparar"; T no meio do membro; erro de face paralela. | P1 | G | BE-01 |
| BE-07 | **Cantos: rente, topo por sólido, tolerância e janela de cantos** | Modo rente; distância à face do sólido (não `max(w,h)`); tolerância configurável; janela "Cantos da estrutura" com lote, ordem e ignorar. | P1 | G | BE-06 |
| BE-08 | **Grupos de orientação e regra de orientação pública** | Rotação/espelho/ancoragem por linha ou grupo; regra de alma fixa e independente do sentido; Inverter/Redefinir; prévia. | P1 | M | BE-01 |
| BE-09 | **Prolongamento por ponta e quebra de polilinha** | `ext` só em pontas livres; ângulo mínimo configurável; fechamento de polígono. | P1 | P | — |
| BE-10 | **Biblioteca de perfis completa (ABNT/AISC/EN)** | Preencher dimensões/peso do CSV com fonte e nível; raios de concordância; teste de massa linear; ligar ao `builtin`. | P1 | G | B42, CTX-PRF |
| BE-11 | **Perfil por croqui com âncoras nomeadas** | Seção arbitrária a partir de croqui, características nomeadas, troca de perfil preservando referências. | P2 | G | BE-10 |
| BE-12 | **Esqueleto como conjunto geométrico e grade** | Conjunto "Esqueleto" ordenado, publicação de linhas, comando Grade (cartesiana/polar, relativo/absoluto). | P1 | M | B55 |
| BE-13 | **Tampa e reforço avançados** | Fora/dentro/interna, razão de espessura, chanfro/filete, furos; reforço triangular/poligonal, lado, posição, alívio de solda. | P1 | M | BE-02 |
| BE-14 | **Cordões de solda e lista de soldas** | `Weld` cosmético e sólido (filete/ranhura), contínuo/intermitente, por par de faces/arestas, massa, símbolo derivado, lista de soldas. | P2 | G | BE-02, PMI-02 |
| BE-15 | **Preparação de junta e estados de soldagem** | Chanfro automático por tipo de solda; vistas prep/solda/usinagem no desenho. | P3 | G | BE-14 |
| BE-16 | **Plano de corte 1D e propriedades por item** | Barra padrão por família, perda de serra, sobras, CSV; propriedades (material, massa, bbox, tipo, detalhes de corte). | P1 | M | BE-03 |
| BE-17 | **Sub-soldagens e montagem viva** | Subgrupo (nome, código, processo) gera submontagem e lista própria; pai publica "item NN"; links vivos; limpeza de órfãos. | P1 | G | B50, BE-04 |
| BE-18 | **Desenho por item** | Folha por item com comprimento/ângulos, corte e detalhe de ponta; orientação do perfil; tabela de soldas. | P1 | G | BE-03, BE-17, B50 |
| BE-19 | **Entalhe por perfil publicado** | Entalhes de catálogo (boca de jacaré, recorte de mesa); fecha o B37. | P2 | M | BE-07 |
| BE-20 | **Desempenho e cache estrutural** | `structural_members` em cache por versão; `all_member_lines`/`member_end` sem O(n²); medidas com 200 membros. | P1 | M | B53 |
| BE-21 | **Corpus estrutural de referência** | Quadro, cavalete, treliça, guarda-corpo, escada com testes "ida e volta" de cota. | P1 | M | B53 |
| BE-22 | **Troca de perfil, divisão/união e tamanhos por configuração** | Replace; split/merge; tamanho por configuração. | P3 | M | BE-11 |

---

## 5. Fontes

**Código e documentos do repositório**
- `crates/fenkra-solidos/src/lib.rs`, `structural.rs`; `crates/fenkra-core/src/doc.rs`; `crates/fenkra-montagem/src/estrutura.rs`; `crates/fenkra-app/src/st_ui.rs`, `app.rs`
- `docs/contextos/estrutural.md`, `design-review.md` (DR-33, DR-44, DR-45), `perfis-estruturais.csv`
- `docs/arquitetura/adr/0010-referencias-v2.md`, `0011-filosofia-de-modelagem.md`
- `docs/benchmark/catalogo.md` (seção Estrutural e soldas), `docs/backlog/backlog.md`

**CATIA V5 / 3DEXPERIENCE**
- https://catiadesign.org/_doc/docv5r14/Structure%20Design.pdf (guia oficial V5R14; texto extraído localmente)
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/catia-structure-design-1-sr1_ik_lvG81Q0Whzkyg18dQBA
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/catia-weld-design-1-wd1_-AUYFGDhQ7ubVvuHzco8Bg
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/catia-frame-structure-designer_mF57L34-RHOJIkaqskVgnw
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/2023x-ga-catia-steel-structures-designer_MZKfv7LOQpSVP6vz3s22DA
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/catia-structure-functional-design-2-sfd_hG1VCg5cRF-N1WHmZXTteQ (resumo de busca)
- https://pycatia.readthedocs.io/en/stable/api/pycatia/cat_str_functional_interfaces/sfm_factory.html
- https://pycatia.readthedocs.io/en/latest/api/pycatia/cat_str_functional_interfaces/sfm_endcut.html
- https://maruf.ca/files/caadoc/CAAStrUseCases/CAAStrCreateStrObject.htm
- https://www.eng-tips.com/threads/catia-frame.255305 (comentário anedótico; acesso bloqueado, usado só o resumo de busca)
- https://www.ibm.com/support/pages/apar/HE05602 (caso de stiffeners no desenho; resumo de busca)

**SolidWorks** (ajuda online só como resumo de busca; texto lido nos blogs)
- https://help.solidworks.com/2017/english/solidworks/sldworks/hidd_dve_feat_weld_member.htm
- https://help.solidworks.com/2017/english/SolidWorks/Sldworks/HIDD_DVE_FEAT_WELD_CORNER.htm
- https://help.solidworks.com/2024/English/SolidWorks/Sldworks/t_applying_Corner_trtmnt.htm
- https://help.solidworks.com/2021/English/SolidWorks/Sldworks/r_complex_corner_propertymanager.htm
- https://help.solidworks.com/2024/English/SolidWorks/sldworks/r_simple_corner_propertymanager.htm
- https://help.solidworks.com/2020/English/SolidWorks/sldworks/HIDD_DVE_FEAT_END_CAP.htm
- https://help.solidworks.com/2017/english/Solidworks/sldworks/HIDD_DVE_FEAT_GUSSET.htm
- https://help.solidworks.com/2017/English/SolidWorks/Sldworks/HIDD_DVE_FEAT_WELD_BEAD.htm
- https://help.solidworks.com/2017/English/SolidWorks/sldworks/HIDD_DVE_COSMETIC_WELD_BEAD.htm
- https://help.solidworks.com/2022/English/SolidWorks/Sldworks/c_Sub_weldments.htm
- https://help.solidworks.com/2018/English/SolidWorks/Sldworks/c_Bounding_Boxes_in_Weldments.htm
- https://help.solidworks.com/2023/english/SolidWorks/sldworks/t_custom_props_weldment_cut_lists_2.htm
- https://help.solidworks.com/2023/english/solidworks/sldworks/t_Weldments_Creating_Custom_Profile.htm
- https://www.cati.com/?p=119988 · https://www.cati.com/?p=119989 · https://www.cati.com/?p=126879 · https://www.cati.com/?p=192040 · https://www.cati.com/?p=122657 · https://www.cati.com/?p=119524
- https://www.goengineer.com/blog/solidworks-weldment-profiles-weldment-tools-guide
- https://www.goengineer.com/blog/solidworks-weldments-corners-gussets-bounding-box
- https://www.goengineer.com/en-us/blog/solidworks-2025-weldments-whats-new
- https://www.goengineer.com/en-us/blog/solidworks-cut-list-properties (resumo de busca)
- https://www.javelin-tech.com/blog/2023/01/solidworks-structure-system-corner-management/ (resumo de busca; acesso bloqueado)
- https://www.javelin-tech.com/blog/2017/11/solidworks-sub-weldment-cut-list/ (resumo de busca)
- https://www.javelin-tech.com/blog/2026/04/how-to-add-descriptions-for-end-caps-and-gussets-in-a-weldments-cut-list-table/ (só título)
- https://store.trimech.com/?p=29241 · https://blogs.solidworks.com/tech/2017/09/create-custom-weldment-profiles-solidworks-2.html (resumos de busca)
- https://3dswym.3dexperience.3ds.com/post/3dexperience-edu-students/weldment-feature_tTCJ6BJBTbyQa_wiAw0Tyg

**Inventor**
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-953F560A-C2D3-4031-8348-762054C7C779.htm (visão geral do Frame Generator)
- https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-270DE367-680C-4144-9079-EB4F0B2BA091.htm (diálogo Insert)
- https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-FAF2FFB2-8965-482C-9691-530954AF59E1.htm (criar quadros básicos)
- https://help.autodesk.com/cloudhelp/2018/ENU/Inventor-Help/files/GUID-70C1E7D6-AE7D-4275-BC11-A9CB0325EC45.htm (Frame Member Info)
- https://help.autodesk.com/cloudhelp/2017/ENU/Inventor-Help/files/GUID-BE98918A-2419-4432-AAE5-46A3E56552FE.htm (tratamentos de ponta)
- https://help.autodesk.com/cloudhelp/2019/ENU/Inventor-Help/files/GUID-181E350C-A869-4A83-899F-7A0CD820D6D4.htm (Miter)
- https://help.autodesk.com/cloudhelp/2021/ENU/Inventor-Help/files/GUID-A70296F0-7D19-47B4-908D-1D8F417ADC62.htm (Reuse)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-62C5DC21-0741-4AB6-9866-00FD11C9926E.htm (entalhes de autoria)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-36AE9264-912D-4D3C-BB24-7EC16FA8777E.htm (CUTDETAIL, Promote/Demote)
- https://help.autodesk.com/cloudhelp/2023/ENU/Inventor-Help/files/GUID-4ADAC958-F42D-4E38-8114-01BD0E98257A.htm (mensagens de erro)
- https://help.autodesk.com/cloudhelp/2019/ENU/Inventor-Help/files/GUID-E197235B-2A86-46FD-BD27-390135E72026.htm (migração)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-019B1B74-A92A-4761-BA4B-58A4583B1FDC.htm (cordões)
- https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-95347D85-B23B-40A8-9268-7E38BF2ACF9B.htm (estratégias de soldagem)
- https://help.autodesk.com/cloudhelp/2024/ENU/Inventor-Help/files/GUID-5986183A-97D5-4E2D-AE00-C1AE0D1C53BB.htm (ambiente de soldagem, resumo de busca)
- https://www.autodesk.com/learn/ondemand/module/introduction-to-inventor-frame-generator
- https://forums.autodesk.com/t5/inventor-forum/custom-profile-for-the-frame-generator/m-p/7109064/highlight/true (fórum; resumo de busca)

**Outros CADs**
- Solid Edge: https://www.soliddna.com/SEHelp/ST5/EN/a_h/frames1a.htm
- Onshape: https://www.onshape.com/en/features/frames · https://www.onshape.com/fr/blog/announcing-onshape-frames
- Creo AFX: https://support.ptc.com/help/creo/creo_pma/r11.0/usascii/advanced_framework.html · https://support.ptc.com/help/creo/creo_pma/r8.0/usascii/whats_new_pma/afx_aluminum_sections_profile_lib.html
- NX: https://blogs.sw.siemens.com/partners/how-do-i-improve-my-structural-steel-design-in-nx/ (descreve o add-in SolidSteel) e a menção ao NX Structure Designer em resumo de busca
- Fusion: https://forums.autodesk.com/t5/fusion-360-ideastation-archived/fusion-360-frame-generator/idc-p/7596076/highlight/true (resumo de busca; sem frame generator nativo)

---

### Handoff

- **Resultado:** relatório completo gravável em `docs/pesquisa/estrutural.md`. Contém 20 fichas, 45 RF, 12 RNF e 22 itens de backlog propostos.
- **Evidência:**
  - Código lido: `fenkra-solidos/src/lib.rs` (trechos `1690`–`2180`, `3775`–`3920`, `4620`–`5030`, `5279`–`5540`), `structural.rs` inteiro, `estrutura.rs` inteiro, `doc.rs:1385`–`1485`, `st_ui.rs:21`–`200`, `app.rs:1500`–`1575`.
  - Fontes web citadas na seção 5.
  - O guia CATIA R14 foi lido do PDF extraído localmente (zlib). Isso gerou um arquivo temporário no diretório de resultados de ferramenta e `sd.txt`/`sd2.txt` no scratchpad. Nada foi criado no repositório.
- **Arquivos alterados:** nenhum.
- **Checagens feitas:** leitura de código e contagem do CSV (159 linhas com designação e dimensões vazias). Nenhum teste foi executado: o modo atual proíbe execução que altere o sistema.
- **Falhas e limites:**
  - A ajuda online do SolidWorks e os espelhos `catiadoc.free.fr` ficaram inacessíveis.
  - As páginas `javelin-tech.com`, `eng-tips.com` e uma de comunidade bloquearam o acesso (403/404); usei só os resumos de busca delas.
  - Os "bugs prováveis" (4–10, 14, 15) são inferências de leitura de código. Cada um traz o teste que o confirma ou refuta. Os de agrupamento (14, 15) são argumentos geométricos, não reproduzidos.
  - O ponto 13 (recomputes duplicados) precisa de medição.
- **Riscos:**
  - O guia CATIA é da R14 (antigo) e o comportamento de versões recentes pode ter mudado.
  - Há divergência entre fontes sobre se os cordões entram ou não na lista de corte do SolidWorks.
  - O ritmo dos tamanhos P/M/G é estimativa minha, na escala do backlog atual.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra em estrutural são (1) placa e reforço ligados a um ponto fixo, com escolha silenciosa da ponta mais próxima; (2) lista de corte com comprimento da linha de centro e sem ângulos de ponta; (3) agrupamento por assinatura cega ao formato das pontas, com numeração de itens instável; (4) ausência de aparar/estender até face/corpo/T e de falha isolada por linha ou membro; (5) ausência de soldagem real (cordão 3D, lista de soldas, submontagem de solda e montagem viva).
