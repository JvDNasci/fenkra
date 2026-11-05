# Interface geral, árvore, atualização e gráficos

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Relatório de pesquisa para o Fenkra, a gravar em `docs/pesquisa/interface-e-arvore.md`.
> **Legenda:** **[V]** foi confirmado em fonte pública que li (páginas ou resumos de busca, listados na seção 5). **[C]** é conhecimento geral meu, não confirmado nesta pesquisa.
> **Limite de verificação:** as páginas `help.CAD avançado A.com` não renderizam para o leitor automático. Os fatos do CAD avançado A vêm de resumos de busca dessas páginas e de blogs de revendas (Hawk Ridge, Javelin, CATI, GoEngineer). A ajuda oficial do CAD avançado B também não está pública. As fontes do CAD avançado B são apostilas de universidades (City University), blogs de revendas (CATI, EDS, Technia, 3DCS) e registros de suporte da IBM. O que sai de blog ou fórum está marcado como tal.

---

## 1. Resumo e principais lacunas

1. A **filosofia CAD avançado B** é uma árvore de especificação ordenada, com **um único objeto em trabalho** (sublinhado). Esse objeto decide onde entra o próximo comando e permite retroceder e inserir no meio do histórico **[V]**.
2. O CAD avançado B separa corpos **ordenados** (Part Body, Ordered Geoset: cada comando consome o anterior) de conjuntos **não ordenados** (Geoset: elementos só referenciados, ordem livre) **[V]**.
3. O **update** do CAD avançado B é explícito. Há modo automático ou manual, símbolo de atualização, diagnóstico por feature (Editar, Desativar, Apagar) e "Stop Update" com memorando **[V]**.
4. O CAD avançado A e o CAD avançado C têm o mesmo núcleo, com nomes e acabamento diferentes. A **barra de retrocesso** (CAD avançado A) e o **End of Part** (CAD avançado C) cumprem o papel do objeto em trabalho. O **What's Wrong** e o **Design Doctor** cumprem o papel do Update Diagnosis. O Design Doctor é o mais guiado **[V]**.
5. As funções "inteligentes" de interface se repetem nos CADs: visualização dinâmica de pais e filhos na árvore, seleção contextual (laço, cadeia, tangência, Select Other), filtros de seleção, mini barra de contexto, menu radial ou gestos e busca de comandos. Também há dicas em dois níveis, navegação de câmera configurável e opções por aplicação separadas das opções por documento.
6. O **Fenkra já tem boa parte**:
   - árvore com seções e multisseleção;
   - marcador de retrocesso por coletor;
   - atualização manual ou automática com indicador de "desatualizado";
   - assistente de reparo;
   - menu de contexto 3D por tipo de elemento;
   - personalização de grupos de barra;
   - 8 predefinições de atalhos e 9 estilos de navegação;
   - cubo de vistas;
   - dicas em dois níveis.
7. A principal lacuna é de **modelo, não de tela**. Não existe "objeto em trabalho" único (`active_collector`, `Session` e o marcador são três mecanismos separados). Não existe grafo de pais e filhos navegável. A árvore é recalculada em todo quadro com clone do documento e serialização JSON (custo escalável).
8. O diagnóstico de erro é pobre: mensagem em texto, sem realce da referência perdida, sem "Isolar" e sem mostrar a cadeia da causa-raiz. O CAD avançado B, o CAD avançado A e o CAD avançado C oferecem tudo isso.
9. A seleção é dividida em dois modelos (`Sel` na árvore e `picks` na viewport) e não tem prioridade nem filtro por tipo de entidade. Também não tem Select Other, laço, cadeia nem tangência no uso geral (`tangent_chain` só é usada pela aba curva de chapa, `app.rs:1620`).
10. Os gráficos têm tesselação fixa (`diagonal × 0,0015`, e 0,35 rad angular fixo no kernel), sem controle do usuário, sem LOD e sem opção de qualidade por documento. Desfazer guarda o documento inteiro, sem nome da ação e sem histórico.

**Principais lacunas (ordem de impacto):**
(a) objeto em trabalho e contexto único;
(b) diagnóstico de atualização guiado e causa-raiz;
(c) dependências (pais e filhos) visíveis e consultáveis;
(d) seleção inteligente (filtros, Select Other, laço, cadeia, tangência, prioridade);
(e) qualidade gráfica e desempenho da árvore.

---

## 2. Fichas por comando ou função

### 2.1 Árvore de especificação / FeatureManager / Browser — estrutura e ícones de estado
**(CAD avançado B: Specification Tree / CAD avançado A: FeatureManager Design Tree / CAD avançado C: Model Browser / outros: CAD avançado E Feature List, CAD avançado H PathFinder, Fusion Browser + Timeline / Fenkra: `App::tree`, `feature_row`)**

- **O que faz:** mostra o histórico de construção da peça e serve de ponto de acesso a editar, renomear, ocultar, suprimir, reordenar e diagnosticar.
- **CAD avançado B:**
  - Cada módulo de geometria aparece como um nó: Part Body (sólido, ordenado), Geometrical Set (superfície, não ordenado) e Ordered Geometrical Set **[V]**.
  - O nó **sublinhado** é o objeto em trabalho (ver 2.7) **[V]**.
  - Ao falhar um update, a feature recebe um símbolo amarelo na árvore e um diálogo diagnostica o problema **[V]**.
  - Com update pendente aparece um símbolo de atualização junto ao nome da peça e a geometria fica vermelha clara **[V]**.
  - Feature desativada aparece entre parênteses vermelhos **[V]**.
  - A árvore também carrega parâmetros e relações, que exigem habilitar "Display in the tree" em Infrastructure > Part Infrastructure > Display **[V]**.
  - Contextual components em montagem recebem a "roda verde" quando "Keep link with selected object" está ativo **[V]**.
- **CAD avançado A:**
  - A árvore mostra as features em ordem de reconstrução. Por padrão os sketches ficam absorvidos pela feature que os usa.
  - "Flat Tree View" (Ctrl+T) lista tudo em ordem cronológica, inclusive os sketches **[V]**.
  - O filtro de texto acima do ícone da peça restringe as linhas por nome ou tag **[V]**.
  - A barra de retrocesso fica no fim (ver 2.7).
  - O ícone de semáforo marca feature que precisa ser reconstruída **[V]**.
  - Itens "absorvidos" podem ser expandidos dentro da feature-pai **[V]**.
- **CAD avançado C:**
  - O browser mostra o estado por ícone. Suprimido aparece como caixa cinza **[V]**.
  - O menu de contexto tem Editar feature, Editar sketch, Redefinir sketch, Compartilhar sketch, Suprimir, Visibilidade, Criar nota, Medir e "Consumir entradas" **[V]**.
  - O menu do browser tem preferências de exibição: ocultar UCS, work features, notas, avisos (não oculta falhas), documentos, e "Mostrar nomes estendidos" **[V]**.
  - "Find" procura relações quebradas, itens suprimidos e parâmetros **[V]**.
- **Outros:**
  - **CAD avançado E:** ícone de erro no canto superior (clica e filtra), triângulo amarelo para seleção faltante, indicador azul para sketch não totalmente definido, filtro por comandos de dois pontos (`:errors`, `:suppressed`, `:hidden`, `:folder`) **[V]**.
  - **CAD avançado H:** ícones distintos para sketch ativo/problema e feature suprimida/falhou/retrocedida/"flutuante" (face de apoio sumiu, com ação Reposition Feature) **[V]**.
- **Fenkra hoje:**
  - `tree()` (`app.rs:3309`) desenha o cabeçalho da peça, as seções recolhíveis (Origem 3358, Referências externas 3381, Publicações 3454, PMI, Parâmetros 3515, Vistas 3559, pastas) e os comandos.
  - O usuário pode ocultar seções (`TREE_SECTIONS`, `app.rs:6208`).
  - `feature_row` (3998) marca o estado por texto: `⟳` desatualizado, `…` varredura visual, `✖` erro, `⚠` aviso, riscado para suprimido, itálico para sketch oculto. Há "entradas do comando" como subitens (4019).
  - Falta:
    - ícone por tipo de estado (só caractere colorido);
    - filtro de texto em toda a árvore (só em "Referências externas", 3390);
    - contador e filtro "só erros";
    - tooltip de erro com cadeia da causa.
  - **Bugs prováveis:**
    - A cada quadro, `tree()` faz `self.open.doc.clone()` (3316).
    - `compute_stale` (696) serializa para JSON cada referência e cada feature (715, 725). Em peça grande isso consome muito por quadro.
    - O estado visual `swept` usa tempo (`sweep_t0`, 765) e pode divergir do cálculo real.
- **Melhor prática para o Fenkra:**
  - Fluxo: o estado de cada linha vem do resultado do recompute (`Status`) mais um conjunto "stale" calculado **só quando o documento muda** (hash incremental por feature), não por quadro.
  - Ícones por estado, com legenda:
    - OK;
    - desatualizado;
    - erro;
    - aviso;
    - suprimido;
    - retrocedido;
    - referência externa (ver `->`, `->?`, `->*`, `->x` do ADR 0011);
    - sketch não resolvido.
  - Caixa de busca única no topo da árvore, com sintaxe de filtros (`:erro`, `:suprimido`, `:oculto`, `:pasta`).
  - **Testes:** abrir uma peça com 300 comandos e medir o tempo do quadro da árvore (alvo < 2 ms sem mudanças); cada estado tem captura visual; o filtro `:erro` lista só os com falha.

### 2.2 Pais e filhos (dependências)
**(CAD avançado B: Parents and Children, Edit > Links / CAD avançado A: Parent/Child, Dynamic Reference Visualization / CAD avançado C: Design Doctor, consumir entradas / outros: CAD avançado E Show dependencies / Fenkra: entradas do comando, diálogo de exclusão)**

- **O que faz:** responde "o que essa feature usa?" (pais) e "o que quebra se eu mexer?" (filhos).
- **CAD avançado B:**
  - O comando "Parents and Children" abre uma janela com o grafo de dependências da feature **[V]**.
  - Ele aparece no menu de contexto (por exemplo no GSD) **[V]**.
  - Existe restrição documentada pela IBM: o conteúdo da lista pode mudar ao limpar a seleção, e o contorno é usar o botão Replace **[V]**.
  - O **Isolate** corta o vínculo de uma geometria copiada com a origem **[V]**.
  - Em Geoset, "Auto Sort" ordena os filhos pela relação pai-filho. A CATI recomenda rodar antes de qualquer release **[V]**.
  - "Change Geometrical Set" mexe nos pais: "Move unshared parents" ou "Move all parents" **[V]**.
- **CAD avançado A:**
  - "Dynamic Reference Visualization" desenha setas ao passar o mouse sobre a feature: **azul = pais, roxa = filhos**. Há botões separados na barra de contexto do nó raiz **[V]**.
  - Quando a referência está oculta, a seta aponta para uma caixa de texto com o nome **[V]**.
  - O padrão mudou entre versões (a de 2015 liga por padrão; relatos de 2016 em diante divergem) **[V]**.
  - Além disso existe o diálogo clássico Parent/Child (menu de contexto) **[C]**.
- **CAD avançado C:**
  - "Consume Inputs" faz o sketch ou work feature ser absorvido, e o Design Doctor lista dependentes antes de apagar: "Delete Feature (dependent features are also deleted)" **[V]**.
  - A mensagem de erro lista a hierarquia até a causa-raiz, e a causa aparece como texto vermelho sublinhado **[V]**.
- **Outros:** CAD avançado E tem "Show dependencies": diálogo com pais (acima) e filhos (abaixo), e o tipo da relação ao passar o mouse **[V]**. Selecionar uma face realça as features que a criaram ou afetaram **[V]**.
- **Fenkra hoje:** a árvore mostra as **entradas** (pais diretos) como subitens (`app.rs:4019`) e destaca na viewport ao passar o mouse (`hover_now`, 4144). O diálogo de apagar calcula dependentes (`DeleteDialog`, 48). Não há grafo de filhos nem janela "Pais e filhos". `Document::refs_of` e `walk` já extraem as referências (usadas em 717 e 742).
- **Melhor prática:**
  - Reaproveitar `refs_of`/`walk` para um índice invertido (filhos) mantido pelo documento.
  - Menu de contexto "Pais e filhos…" abre painel com duas colunas e realce na viewport.
  - Setas ao passar o mouse, ligadas por padrão e desligáveis.
  - **Testes:** apagar o sketch-base lista todos os filhos; "Isolar" transforma a referência em geometria fixa e o filho continua calculando; ciclo é rejeitado com mensagem (o CAD avançado B reporta "cycle detected" em Copy/Paste As Special **[V]**).

### 2.3 Reordenar comandos
**(CAD avançado B: Reorder / Graph Tree Reordering / CAD avançado A: arrastar na árvore / CAD avançado C: arrastar no browser / Fenkra: `TreeAction::Move`)**

- **O que faz:** muda a ordem de cálculo.
- **CAD avançado B:**
  - Em corpos ordenados, o comando Reorder move antes, depois ou "inside" de uma referência **[V]**.
  - Em Geoset não há necessidade, porque a ordem não muda o resultado **[V]**.
  - Arrastar e soltar torna o corpo afetado o objeto em trabalho **[V]**.
  - Em produto, só se solta no fim do produto **[V]**.
  - "Reorder Children" abre uma janela para reordenar os filhos **[V]**.
  - Se a nova ordem deixa um filho antes do pai, o CAD avançado B reporta o erro.
- **CAD avançado A:** arrastar a feature para outra posição (a ordem de reconstrução muda). O sistema bloqueia movimentos que deixariam uma feature antes de uma que ela usa **[C]**.
- **CAD avançado C:** arrastar no browser, com a mesma regra **[C]**.
- **Outros:**
  - **CAD avançado H:** arrastar mostra uma seta de destino; se o movimento invalida outras features elas vão para o **Error Assistant** **[V]**.
  - **CAD avançado E:** arrastar uma ou várias features, mesmo não adjacentes, reordena parametricamente **[V]**.
  - **Fusion:** arrastar as operações na timeline (só no modo paramétrico) **[V]**.
- **Fenkra hoje:** `TreeAction::Move(id, to)` (`app.rs:4177`) chama `Document::move_feature`, que devolve erro (toast). O menu tem "◀ antes / depois ▶" (feature_row 4098). Há `drag_item` (6240) e `DragItem` (6169) para arrastar na árvore (pastas e coletores).
- **Falta:** soltar com indicador de destino e prévia de quais filhos ficariam inválidos; ação "Mover para depois de…".
- **Bug provável:** `Move(id, i-1)` com `i > 0` no menu ignora o limite entre coletores (`is_free`/`target`). Testar mover uma feature para antes do primeiro comando do seu corpo.
- **Melhor prática:**
  - O arraste mostra a linha-alvo e, antes de soltar, valida pais e filhos (`refs_of`). Se houver conflito, cursor proibido com dica "usa X, que vem depois".
  - Desfazer restaura a ordem.
  - **Testes:** mover um pocket para antes do seu sketch é recusado; mover entre coletores muda o corpo e atualiza; desfazer volta.

### 2.4 Renomear
**(CAD avançado B: Properties > Feature Name / CAD avançado A: F2 ou clique lento / CAD avançado C: F2 / Fenkra: F2, `renaming`)**

- **O que faz:** nome descritivo.
- **CAD avançado A:** clique lento ou F2 **[V]**. O nome é a base da busca no filtro **[V]**.
- **CAD avançado B:** o nome fica nas propriedades da feature **[C]**. Publicar usa o nome **[C]**.
- **CAD avançado C:** F2 ou clique lento **[C]**.
- **CAD avançado E:** menu de contexto Rename, edição direta na lista **[V]**.
- **Fenkra hoje:** F2 (`app.rs:6548`) ativa `renaming` para `Feature`, `Ref`, `Folder` e `Collector`. Renomear referência publicada muda o nome da publicação (menu 3467).
- **Falta:** renomear peça (`Sel::Part`), parâmetro e vista pela árvore; validar nome duplicado e caracteres inválidos; aviso explícito de que o nome de uma publicação quebra links de outras peças (o ADR 0011 prevê religar por nome).
- **Melhor prática:** F2 em qualquer linha; Enter confirma, Esc cancela. Ao renomear publicação, mostrar quantos documentos a usam (quando o projeto tem índice). **Testes:** nome vazio não é aceito; nome duplicado ganha sufixo ou erro; undo restaura.

### 2.5 Pastas, agrupamento e conjuntos (Geometrical Set / Folder)
**(CAD avançado B: Geometrical Set, Ordered Geometrical Set, Part Body / CAD avançado A: Folders / CAD avançado C: Folders no browser / Fenkra: pastas de geometria e de comandos, coletores)**

- **O que faz:** organiza a árvore sem mudar o modelo, e em alguns CADs muda a semântica (ordenado × não ordenado).
- **CAD avançado B:**
  - Três tipos: Part Body, Geoset e Ordered Geoset **[V]**.
  - Regras de aninhamento: um Geoset só contém outros Geosets; um Ordered Geoset pode conter Part Bodies mas não Geosets não ordenados; um Geoset não pode ser criado dentro de um Ordered Geoset e vice-versa (relatado em APAR) **[V]**.
  - Se o Geoset é o objeto em trabalho, o próximo comando vai ao fim dele; se um Geoset é o objeto em trabalho, ele vira pai padrão de um novo Geoset **[V]**.
  - A fornecedor de CAD avançado recomenda organizar por Part Bodies e Open Bodies em vez de camadas **[V, fórum]**. Camadas dependem de "Visualization Filters" **[V, fórum]**.
  - "Change Geometrical Set" move objetos entre conjuntos **[V]**.
- **CAD avançado A:** pastas manuais e automáticas (por exemplo "Solid Bodies", "Surface Bodies"). Pastas só agrupam; a ordem de reconstrução continua linear **[C]**.
- **CAD avançado C:** pastas no browser agrupam itens consecutivos **[C]**. Origin folder é fixa **[V]**.
- **Outros:**
  - **CAD avançado E:** pastas aninhadas; suprimir a pasta suprime os itens **[V]**.
  - **CAD avançado H:** conjuntos definidos pelo usuário de entradas do mesmo tipo; ordem em coleção síncrona não afeta o modelo **[V]**.
- **Fenkra hoje:**
  - `FolderKind::{Geometria, Comandos}`, `NewGeometryFolder`/`NewCommandFolder` (`app.rs:4260-4266`).
  - Coletores (`BodyCollector`, `doc.rs:1271`) funcionam como "Part Body": têm cor, visibilidade, marcador, publicado, `NewCollector` (4268).
  - `ToFolder`, `ToCollector` e o menu "Organizar" (4107).
  - A pasta de geometria funciona como Geoset **não ordenado** (sketches e referências livres).
- **Falta:** a regra **explícita** de ordenado × não ordenado (o ADR 0011 cita, mas não há mensagem ao tentar soltar um comando de sólido em pasta de geometria); "Auto Sort" por dependência; mover pasta inteira; pasta dentro de pasta; "Suprimir pasta".
- **Melhor prática:**
  - Regra visível: pasta de geometria é livre (ordem não importa, mostra ícone diferente), pasta de comandos é ordenada.
  - Ao soltar item inválido, mostrar o motivo.
  - Oferecer "Ordenar por dependência" na pasta livre.
  - **Testes:** dois sketches trocam de ordem na pasta livre e o resultado não muda; um comando de sólido não entra na pasta livre; suprimir pasta suprime os itens e volta ao reativar.

### 2.6 Ocultar e mostrar (hierárquico)
**(CAD avançado B: Hide/Show e No Show (swap visible space) / CAD avançado A: Hide/Show, Display Pane / CAD avançado C: Visibility / Fenkra: olho, `H`, ambiente de ocultos)**

- **O que faz:** controla a visibilidade sem apagar, em nível de peça, corpo, feature ou componente.
- **CAD avançado B:**
  - Existe um **espaço "No Show"**: o oculto vai para uma "área" separada. O comando Swap Visible Space mostra só os ocultos (fundo verde-claro) **[V, fórum]**.
  - Em produtos, a regra de pai-filho vale: ocultar o pai oculta os filhos; mostrar um item dentro de pai oculto exige abrir o nível correto **[V]**.
  - Há restrições: com o Graph ativo não se pode alternar o espaço **[V]**.
- **CAD avançado A:** o olho vem do Hide/Show por clique; o "Display Pane" mostra colunas por propriedade **[C]**.
- **CAD avançado C:** "Visibility" no menu de contexto; "Hide Work Features" liga e desliga a exibição de todas as features de trabalho e da pasta Origin **[V]**.
- **CAD avançado E:** olho ao lado do nome ou Hide/Show no menu; filtro `:hidden` **[V]**.
- **Fenkra hoje:**
  - Olho só em sketches (`feature_row`, 4033); sólidos aparecem pelo corpo (`CollectorVisible`).
  - `H` alterna a seleção (`toggle_hide_selected`, 5095).
  - `hidden_env` (Shift+H) mostra só os ocultos (162).
  - Filtro de exibição por tipo (`Filter`, `settings.rs:267`).
- **Falta:** ocultar **hierárquico** (pasta oculta todos os filhos; ocultar corpo oculta sketches que só ele usa); resumo "N itens ocultos"; mostrar tudo.
- **Melhor prática:**
  - Estado oculto propaga de pastas para filhos (herança com override explícito).
  - O ambiente de ocultos usa uma cor de fundo distinta (como o fundo verde-claro do CAD avançado B).
  - Contador na barra de status.
  - **Testes:** ocultar pasta oculta os itens; reexibir só um filho funciona; salvar e abrir mantém; ambiente de ocultos permite reexibir e sair com Esc.

### 2.7 Objeto em trabalho × barra de retrocesso × End of Part
**(CAD avançado B: objeto de trabalho (DIWO), Scan / CAD avançado A: Rollback Bar, Roll To / CAD avançado C: End of Part marker / outros: CAD avançado E rollback, Fusion timeline marker, CAD avançado F Insert Here, CAD avançado H rollback, CAD avançado G Edit with Rollback / Fenkra: `active_collector` + `marker` + `Session`)**

- **O que faz:** define onde o **próximo comando** entra e quais features existem durante a edição. É o conceito central da filosofia-alvo.
- **CAD avançado B — como funciona:**
  - O objeto em trabalho é o item **sublinhado** da árvore. Ele decide onde um novo comando é colocado **[V]**.
  - Clicar o botão direito num corpo e escolher "objeto de trabalho" ativa esse corpo. Inserir um novo Body o torna ativo **[V]**.
  - Em Part Bodies e Ordered Geosets pode-se definir uma **feature** como objeto em trabalho: o CAD avançado B mostra a peça só até aquela feature, insere novos comandos ali e atualiza o resto depois **[V]**.
  - **Modo Scan**: Edit > Scan or objeto de trabalho abre uma barra com botões para percorrer as features uma a uma. Os sketches não participam do passo. "Exit" isola a feature atual como objeto local **[V]**.
  - Sem scan: botão direito na feature > objeto de trabalho **[V]**.
  - Se um Geoset é o objeto em trabalho, o comando vai ao fim dele **[V]**.
  - Arrastar e soltar torna o corpo afetado o objeto em trabalho **[V]**.
  - O CAD avançado B continua exibindo o aviso de update quando features posteriores ficam desatualizadas **[C]**.
- **CAD avançado A — no detalhe:**
  - A **barra de retrocesso** azul fica no fim da árvore **[V]**. Arrastá-la para cima **suprime** o que está abaixo (some da viewport e não pode ser editado) **[V]**.
  - Também se move com as setas depois de clicar nela **[V]**.
  - Dá para adicionar e editar features enquanto retrocedido; as novas entram na posição da barra **[V]**.
  - "Roll To End" (menu da feature) devolve a barra ao fim **[V]**.
  - Features absorvidas podem ser "desabsorvidas" via Rollback no item expandido **[V]**.
  - **Inteligência extra:** "Roll to Previous", "Roll to Here" e a "Freeze Bar" (barra de congelamento que impede a edição de features acima) **[C]**.
- **CAD avançado C — no detalhe:**
  - Marcador **End of Part (EOP)** no browser **[V]**.
  - Arrastar o EOP para cima retrocede o modelo; abaixo ficam as features suprimidas, não apagadas **[V]**.
  - Menu: Move EOP (para baixo da feature selecionada), Move EOP to Top, Move EOP to End, **Delete All Features Below EOP** (destrutivo) **[V]**.
  - Esc cancela o movimento com mensagem **[V]**.
  - Em chapa dobrada ou planificada vira EOF **[V]**.
  - Recomendação do help: para achar uma feature com falha, mover o EOP uma feature por vez até aparecer o erro **[V]**.
- **Outros:**
  - **CAD avançado E:** a barra mostra o modelo só até ela; as setas movem passo a passo; "Roll to here" e "Roll to end". O CAD avançado E não libera uma peça enquanto a barra não está no fim **[V, fórum]**.
  - **Fusion:** marcador da timeline. Itens depois do marcador ficam retrocedidos; ao criar com o marcador fora do fim, a feature entra ali e enxerga o modelo retrocedido. Mover para dentro de um grupo recolhido o expande **[V]**.
  - **CAD avançado F:** barra "Insert Here". Forçar a regeneração a partir dela pode limpar falhas **[V]**.
  - **CAD avançado G:** a ordem de atualização é pelo *timestamp* de cada feature **[V, fórum]**. "Edit with Rollback" está nas features **[V]**.
  - **CAD avançado H:** features retrocedidas têm ícone próprio; "Goto" avança **[V]**.
- **Fenkra hoje:**
  - **Marcador por coletor**: `BodyCollector.marker: Option<usize>` (`doc.rs:1271`). A UI tem ▲▼⏬ (`app.rs:6289`) e botões ⏮◀▶⏭ na barra de status, que movem o marcador do corpo ativo (5313-5335). Há também "Marcador até aqui" no menu da feature (4093).
  - **Corpo ativo** (`active_collector`, `Activate`, 4166) é o "Part Body em trabalho".
  - **Sessão** (`Session`, 228; `start_session`, 1939): ao abrir um comando, guarda um snapshot; Concluir mantém, Cancelar restaura (`cancel_session`, 1973).
  - Resultado: **três mecanismos** (corpo ativo, marcador, sessão) sem um "objeto em trabalho" único e visível. Não há "Definir como objeto em trabalho" na feature, não há sublinhado e não há modo Scan.
  - **Bugs prováveis:**
    - O marcador é ligado por **coletor**, então retroceder um corpo não retrocede os sketches livres que outro corpo usa, e a viewport pode mostrar um estado misto.
    - `cancel_session` troca `open.doc` pelo snapshot (1975) e descarta edições feitas por fora da sessão (undo feito dentro da sessão). Testar: abrir comando, usar Ctrl+Z, Cancelar.
    - `conclude_session` pede recompute "também no modo manual" (1964) e isso é coerente, mas pode surpreender quem escolheu modo manual.
- **Melhor prática (regras que evitam bugs simples):**
  1. **Um só "objeto em trabalho"** por peça, sempre visível: sublinhado na árvore + texto na barra de status ("Em trabalho: Corpo 1 › depois de Furo 3").
  2. Comando "Definir como objeto em trabalho" na feature e no corpo. Substitui "Marcador até aqui" e `Activate`. O marcador deixa de ser UI própria e vira o próprio objeto em trabalho.
  3. Enquanto retrocedido, **mostrar** as features abaixo esmaecidas (opção) para o usuário saber que existem. No CAD avançado B e no CAD avançado E elas somem, no CAD avançado A ficam suprimidas.
  4. Novos comandos entram **depois** do objeto em trabalho e, ao concluir, o objeto em trabalho passa a ser o novo comando (como no CAD avançado B).
  5. Ao salvar com o objeto em trabalho fora do fim, **avisar** (o CAD avançado E trava a liberação **[V, fórum]**).
  6. "Apagar tudo abaixo" só com confirmação (o CAD avançado C oferece **[V]**).
  7. **Modo Scan** opcional: ◀ ▶ percorrem as features, cada passo atualiza o realce e a viewport.
  - **Testes:** inserir um furo no meio, o restante atualiza; retroceder e voltar ao fim restaura o mesmo resultado; salvar retrocedido avisa; Cancelar desfaz também o objeto em trabalho; mudar de corpo ativo não deixa sketches órfãos.

### 2.8 Suprimir e desativar
**(CAD avançado B: Deactivate / CAD avançado A: Suppress / CAD avançado C: Suppress / Fenkra: `Suppress`, "Suprimir" no assistente)**

- **CAD avançado B:** o diálogo de diagnóstico oferece **Desativar** (a feature falha sai do cálculo) **[V]**. Feature desativada fica entre parênteses vermelhos **[V]**.
- **CAD avançado A:** suprimir exclui do cálculo e da viewport; filhos também são suprimidos **[C]**.
- **CAD avançado C:** suprimir mantém no browser com caixa cinza **[V]**. No Design Doctor: "Suppress Feature (dependent features are also suppressed)" **[V]**.
- **CAD avançado E:** "Dynamic suppression": uma expressão suprime ou reativa a feature automaticamente (ícone de expressão); nome riscado **[V]**.
- **Fenkra hoje:** `Feature.suppressed` (`app.rs:4167`), riscado na árvore (4012). Participa do conjunto "stale" (723). Não há supressão por expressão/configuração. Não há aviso de que suprimir afeta os filhos (o recompute decide).
- **Melhor prática:** ao suprimir, listar os filhos que também serão suprimidos (usa o índice de 2.2); ativar/suprimir por expressão de parâmetro (P3). **Testes:** suprimir o sketch suprime o extrude que o usa; reativar restaura sem erro.

### 2.9 Atualização manual e automática
**(CAD avançado B: Update, modo Automático/Manual, Local Update / CAD avançado A: Rebuild Ctrl+B / CAD avançado C: Update, Local Update, Rebuild All / CAD avançado D e CAD avançado E: automática / Fenkra: `auto_update`, F5)**

- **O que faz:** recalcula a geometria depois de uma mudança.
- **CAD avançado B:**
  - Modo em Tools > Options > Infrastructure > Part Infrastructure > General > Update: **Automatic** (padrão) ou **Manual** **[V]**.
  - No manual, atualiza-se pelo ícone Update, Edit > Update ou o comando de contexto; uma barra de progresso mostra o avanço **[V]**.
  - Pode cancelar pelo botão Cancel do diálogo "Updating" ou interromper com um *stop update* **[V]**.
  - Sinal de pendência: símbolo de atualização junto ao nome da peça e geometria em vermelho brilhante **[V]**.
  - **Local Update** atualiza só a feature selecionada **[V, resumo de APAR]**.
  - IBM APAR indica que, no manual, o Update Diagnosis não deveria rodar sozinho **[V]**.
  - Em montagem: "Edit > Update All" e controle de update por componente **[C]**.
- **CAD avançado A:**
  - A reconstrução é adiada: depois de alterar uma cota aparece o semáforo ao lado da feature e o CAD avançado A **espera** o Ctrl+B, permitindo mudar várias cotas antes **[V]**.
  - **Ctrl+B** reconstrói só o que mudou e seus filhos **[V]**.
  - Evaluate > Statistics mostra o tempo por feature, para suprimir a mais lenta **[V]**.
- **CAD avançado C:**
  - O botão **Update** faz recomputação incremental das features desatualizadas; **Rebuild All** recalcula tudo, mudado ou não **[V, moderador do fórum]**.
  - Em peça, Rebuild All recalcula todas as features; em montagem, restrições, features e componentes adaptativos, mas não as peças internas **[V]**.
  - O "raio" (lightning bolt) de pendência não pode ser suprimido; **Ctrl+D** na caixa About mostra o que está "sujo" **[V]**.
  - Causas de arquivos sempre sujos: features de montagem, derivados, projeto top-down, contexto, adaptatividade **[V]**.
  - O Task Scheduler faz update em lote com "Total Rebuild" **[V]**.
- **Outros:** CAD avançado E e Fusion recalculam sempre, sem modo manual **[C]**.
- **Fenkra hoje:**
  - `auto_update` (`settings.rs:337`, padrão `true`), caixa "auto" na barra de status (`app.rs:5306`) e na página de configurações (7376).
  - **F5** atualiza; **Ctrl+F5** reconstrói do zero (6553, `send_recompute(fresh)`).
  - Recompute em *thread* separada (`Worker`, 60-95), cache incremental (`recompute_cached`) e só o pedido mais recente é processado.
  - Mudança → `needs_recompute` (756) compara com o último `sent_doc`; no modo automático dispara `request_recompute` (6780).
  - `stale_from` e `compute_stale` pintam a árvore (⟳) e a barra de status ("desatualizado — F5 atualiza", 5298).
  - Falta:
    - Update **local** (só esta feature);
    - cancelar um cálculo em curso;
    - "atualizar" por componente na montagem;
    - progresso real (hoje há só `⏳ recalculando…` e a "varredura" visual de 35 ms/linha, 5293).
  - **Bug provável:** o `Worker` só descarta pedidos *enfileirados* (78-81). Um cálculo longo do OpenCascade não é interrompível, então mudar o documento durante o cálculo espera o fim.
- **Melhor prática:**
  - Padrão **automático**, com opção "manual" por **documento** (modelos grandes), como o CAD avançado B.
  - No manual, mostrar o selo "desatualizado" na peça inteira e nas linhas afetadas (já existe).
  - Comando "Atualizar esta feature" (local) com resultado parcial.
  - Botão "Cancelar" real: dividir o recompute em etapas (por feature) e checar um *flag* de cancelamento entre elas.
  - **Testes:** modo manual não recalcula sozinho ao mudar uma cota; F5 recalcula só a partir da primeira feature alterada (cache); Ctrl+F5 dá o mesmo resultado que F5 (propriedade: *determinismo* — comparar volumes e contagem de faces).

### 2.10 Reconstrução forçada (verificação)
**(CAD avançado B: Update All / CAD avançado A: Force Rebuild Ctrl+Q / CAD avançado C: Rebuild All / Fenkra: Ctrl+F5 "Reconstruir")**

- **CAD avançado A:** **Ctrl+Q** reconstrói tudo do início da árvore. Serve de **diagnóstico** para garantir que features novas não introduziram erro em anteriores **[V]**. Em montagens, só nível superior e mates **[V]**. Ctrl+Shift+Q força em todas as configurações **[V]**. A recomendação de revendas é rodar antes de salvar **[V]**.
- **CAD avançado C:** Rebuild All (ver 2.9) — "se precisa dele, há um problema de fundo" **[V]**.
- **Fenkra hoje:** "⟲ Reconstruir" e Ctrl+F5 descartam o cache (`app.rs:5310`). Não há verificação automática de consistência entre cache e reconstrução total.
- **Melhor prática:** teste de corpus (B53) que compara resultado em cache × reconstrução total para cada peça (volume, área, faces) e falha o CI se divergir; botão "Verificar" que faz isso na peça aberta e mostra o diff (qual feature diverge). **Testes:** editar uma cota e voltar → resultado idêntico ao original.

### 2.11 Diagnóstico de falhas (Update Diagnosis / What's Wrong / Design Doctor)
**(CAD avançado B: Update Diagnosis, Feature Definition Error / CAD avançado A: What's Wrong, Rebuild Errors / CAD avançado C: Design Doctor, Sketch Doctor, Recover / CAD avançado F: Failure Diagnostics, Resolve / CAD avançado H: Error Assistant / Fenkra: assistente de reparo `wizard_window`)**

- **O que faz:** quando o cálculo falha, localiza a causa e oferece reparos.
- **CAD avançado B:**
  - Causas comuns: valor de edição inadequado ou elemento geométrico apagado **[V]**.
  - A feature falha ganha símbolo amarelo, o diálogo diagnostica e **o preview deixa de mostrar a feature** **[V]**.
  - O diálogo oferece, por feature: **Editar**, **Desativar** ou **Apagar** **[V]**.
  - "Editar" abre o *Feature Definition Error*: faces removidas ficam **amarelas**, rotuladas "Removed Face"; seleciona-se a face substituta e OK reconstrói **[V]**.
  - **Stop Update** (ver 2.12).
  - No manual, o Update Diagnosis não deveria rodar sozinho **[V, APAR]**; há relatos de aparecer sem necessidade ao sair do sketcher **[V, APAR]**.
- **CAD avançado A:**
  - **What's Wrong** (botão direito na peça, feature ou montagem): lista erros e avisos da reconstrução **[V]**.
  - "When rebuild error occurs": **Stop / Continue / Prompt**; "Show errors every rebuild" decide se o diálogo reaparece **[V]**.
  - Causas típicas: cota ou relação a entidade que não existe mais, filete grande demais **[V]**.
  - Ctrl+Q expõe erros que Ctrl+B não vê **[V]**.
- **CAD avançado C:**
  - **Design Doctor** abre pelo ícone no diálogo de mensagem, pelo **Recover** no browser ou pela cruz vermelha da barra de acesso rápido **[V]**.
  - Variantes: **Sketch Doctor** (loops abertos, curvas sobrepostas, pontos redundantes, coincidências faltando), **Feature/Weldments Doctor**, **Assembly Constraints Doctor**, **Export derived objects** **[V]**.
  - Tratamentos por tipo: Fechar loop, Editar sketch; Editar, **Apagar** (dependentes também) ou **Suprimir** feature (dependentes também); em restrições: Editar, Apagar, Suprimir, **Isolar e editar** (oculta tudo, exceto os dois componentes), **Isolar falhas relacionadas**, **Diagnosticar** (Relationship Conflict Analysis) **[V]**.
  - Controles: Next, **Highlight/Unhighlight Problem**, **Continue Examination** (reexecuta o histórico para achar a origem quando a geometria sumiu), Finish **[V]**.
  - A lista de erros é hierárquica e a causa-raiz é texto vermelho sublinhado; ao selecionar, destaca-se a posição na viewport **[V]**.
  - Corrija **de cima para baixo**, porque dependentes ficam abaixo **[V]**.
  - Pode-se fechar o Doctor e voltar depois via Recover **[V]**.
  - Há bug conhecido no fluxo "Invalid axis/origin" **[V, fórum 2025]**.
- **Outros:** CAD avançado F abre o *Resolve environment* com *Failure Diagnostics* e menu RESOLVE FEAT **[V]**; CAD avançado H manda as features invalidadas para o *Error Assistant* **[V]**; CAD avançado E tem filtro de erros e triângulo amarelo para seleção faltante **[V]**.
- **Fenkra hoje:**
  - `Status::{Error(msg), Warning(msg), Inactive}` (de `fenkra_solidos`). A árvore mostra `✖`/`⚠` e a mensagem no *hover* (`app.rs:4048`).
  - `wizard_window` (3190) lista erros (features e referências) com **Refazer referência** (abre a sessão do comando), **Suprimir** e **Apagar**. O botão "✖ N com erro — reparar" abre o assistente (5343). Nada quebrado → mensagem verde.
  - Para links externos: ações Congelar, Desativar, **Isolar** (`isolate_link`, 6799) e Religar.
  - Falta:
    - realçar na viewport a referência/face que sumiu (o "Removed Face" amarelo);
    - cadeia de causa (qual pai falhou primeiro);
    - ordenar de cima para baixo;
    - "Isolar" para referências de geometria;
    - Continue Examination;
    - classificação (sketch, feature, referência, link);
    - política Stop/Continue/Prompt;
    - dica de reparo por tipo de erro.
  - **Bugs prováveis:** `errors_prev` (208) conta erros do último resultado. "Desativar de propósito não deve abrir o assistente" é tratado por truque (`errors_prev = usize::MAX`, 6792); frágil. A mensagem `msg` é livre, sem código de erro estável, então não dá para sugerir reparo específico.
- **Melhor prática (alinhada ao princípio 1 do ADR 0011):**
  1. **Estado "falhou" com diagnóstico**, nunca troca silenciosa de geometria.
  2. Cada erro tem **código estável** (`ErrorKind`: RefPerdida, FaceMudou, EsbocoAberto, ValorInvalido, Ciclo, KernelFalhou, LinkQuebrado) + mensagem + **causa-raiz** (id do primeiro filho-pai que falhou).
  3. Painel "Diagnóstico" (substitui o assistente): lista de cima para baixo, hierarquia de causa, botões **Realçar**, **Editar**, **Substituir referência**, **Isolar**, **Suprimir**, **Apagar** (com lista de dependentes).
  4. Em "Realçar", pintar na viewport o que sumiu (amarelo) e o que resta.
  5. Política de erro de reconstrução: **Parar / Continuar / Perguntar** nas configurações.
  6. "Continuar exame": retrocede o objeto em trabalho feature a feature até o erro aparecer.
  - **Testes (corpus B53):** apagar a face de apoio de um furo → erro `RefPerdida`, "Substituir" abre seleção de face e reconstrói; sketch aberto → o painel oferece "Fechar loop"; ciclo de referência → recusa; erro sem causa conhecida mostra texto cru, mas nunca vazio.

### 2.12 Parada de atualização (Stop Update)
**(CAD avançado B: Stop Update / CAD avançado A: freeze bar, Rollback / CAD avançado C: EOP / Fenkra: —)**

- **CAD avançado B:** botão direito > Properties > "Associate stop update" + memorando. Aparece uma entidade **Stop Update** abaixo da feature; o update para ali e mostra o memorando, e o usuário escolhe parar ou continuar. Funciona em modo manual e automático. "Deactivate stop update" (parênteses vermelhos) a desliga **[V]**.
- **Utilidade:** marcar pontos de checagem em modelos longos (por exemplo "confira a espessura aqui").
- **Fenkra hoje:** não existe.
- **Melhor prática:** P3. Reaproveitar o marcador: "Ponto de parada" com nota, que o recompute respeita e informa na barra de status. **Teste:** o update para na marca, mostra a nota e continua a pedido.

### 2.13 Desfazer e refazer
**(CAD avançado B: Undo/Redo, Undo with History / CAD avançado A: Ctrl+Z, Undo list / CAD avançado C: Ctrl+Z / Fenkra: `undo`/`redo`)**

- **O que faz:** desfaz mudanças de modelagem.
- **CAD avançado B:** pilha por documento, com número configurável em Tools > Options > General > Performances (de 0 a 99 na doc do R16). Ações "globais" aparecem como entradas "Start…" em "Undo with history" **[V, IBM, R16 — pode ter mudado]**. O limite não vale para ações globais **[V]**.
- **CAD avançado A:** lista de desfazer pela setinha, com nome da ação; desfaz de modelagem, não de seleção **[C]**.
- **CAD avançado C:** desfaz em níveis limitados; ligado a um registro não documentado para ampliar (relato de fórum, não confirmado) **[V, fórum, fraco]**.
- **Fenkra hoje:**
  - `undo: Vec<Document>` e `redo` (`app.rs:149-150`); **clone do documento inteiro** a cada passo (6767), máximo 200 (6768).
  - **Coalescência:** só empilha se passaram mais de 0,8 s desde a última edição (6766). Edições rápidas sucessivas viram **um** passo.
  - Montagem tem pilha própria (6468). Desenho tem a dele (`dwg_ui`).
  - `load_doc` limpa as pilhas (787).
  - **Falta:** nome do passo ("Desfazer: Extrusão 2"), lista de histórico, desfazer de seleção/câmera (não deve existir), agrupar por comando, limite por memória.
  - **Bugs prováveis:**
    - A janela de 0,8 s agrupa ações distintas feitas em sequência rápida (por exemplo apagar e logo mudar uma cota, num script/macro).
    - Documento grande × 200 clones → uso alto de memória (RNF).
    - `cancel_session` troca o documento por snapshot (1975) e deixa `undo` apontando para estados que incluem a sessão; Ctrl+Z após Cancelar pode "ressuscitar" o comando cancelado.
    - Desfazer dentro de uma sessão aberta mexe no documento enquanto `Session.snapshot` guarda o antigo.
- **Melhor prática:**
  - Modelo de **comando** (passo nomeado com `Document` inteiro como *fallback*, ou *diff* estrutural quando disponível).
  - Cada comando concluído = 1 passo; edição contínua (arrastar cota) = 1 passo ao soltar (substitui o tempo de 0,8 s).
  - Cancelar sessão **não** cria passo; Concluir cria 1 passo com o nome do comando.
  - Lista de histórico na seta ao lado do botão. Limite em MB além de em contagem.
  - **Testes:** 3 edições rápidas = 3 passos; abrir comando, Cancelar, Ctrl+Z → desfaz a ação anterior ao comando (não "ressuscita"); 200 passos numa peça de 2 000 comandos permanece abaixo de um teto de memória.

### 2.14 Realce, pré-seleção e "Select Other"
**(CAD avançado B: Preselection Navigator Ctrl+Shift+F11, Preselect in geometry view / CAD avançado A: Select Other, tooltip de seleção / CAD avançado C: Enable Prehighlight, Select Other / Fenkra: `hover_pick`)**

- **O que faz:** mostra o que seria selecionado antes do clique e resolve ambiguidade quando vários elementos estão sob o cursor.
- **CAD avançado B:**
  - **Preselection Navigator** (Ctrl+Shift+F11): lista em forma de árvore os candidatos sob o cursor; **Ctrl+F11** devolve o foco à área de geometria para usar o navegador com diálogo aberto **[V]**.
  - "Display auxiliary viewer for preselection navigator" (Tools > Options > General > Display > Navigation) abre uma vista auxiliar **[V]**.
  - "Preselect in geometry view" ativa o realce do que o cursor vai pegar **[V]**.
  - O navegador não combina com filtros de seleção **[V, Eng-Tips FAQ]**.
  - A **Selection Trap** (ícone perto da bússola) ajuda a pegar pontos escondidos ou apinhados **[V]**.
- **CAD avançado A:** clique direito > **Select Other** lista os elementos sob o cursor. Com o comando ativo, clique direito **esconde** a face à frente até terminar **[V]**. Útil, por exemplo, para a face de baixo de uma arruela ao aplicar mate **[V]**.
- **CAD avançado C:** "Enable Prehighlight" (ligado por padrão; opção em Application Options > General) **[V]**. Select Other sempre mostra o pré-realce **[V]**. **Shift + botão direito** abre o menu de prioridade no cursor **[V, fonte secundária]**.
- **Fenkra hoje:** `hover_pick` (`app.rs:5582`) calcula o melhor elemento por quadro (`picking::pick`, 185) e o desenha realçado (5769). Não há lista de candidatos. Clique em ponto sem elemento limpa a seleção (5678).
- **Falta:** Select Other / navegador; esconder face da frente temporariamente; ciclar por candidatos (Tab ou roda); tolerância de pick configurável.
- **Bug provável:** o pick usa a malha de exibição (tesselada), então com tesselação grossa a aresta/face "errada" pode ser pega (ver 2.27).
- **Melhor prática:**
  - Clique direito > "Selecionar outro…" lista as 8 primeiras camadas por profundidade (faces, arestas, vértices, sketches, planos), com realce ao passar a lista.
  - **Alt+roda** cicla candidatos (convenção comum **[C]**).
  - O realce deve usar cores consistentes: pré-realce, selecionado, erro.
  - **Testes:** sobrepor um plano de origem e uma face → Select Other lista os dois na ordem de profundidade; esconder a face da frente deixa pegar a de trás.

### 2.15 Filtros de seleção e prioridade
**(CAD avançado B: User Selection Filter / CAD avançado A: Selection Filter F5, F6, E/X/V / CAD avançado C: Select Priority (Face and Edge, Feature, Part, Component) / Fusion: Selection Priority / Fenkra: `sel_faces/edges/vertices`)**

- **CAD avançado B:**
  - Barra *User Selection Filter* (View > Toolbars) **[V]**. Ícones à esquerda: tipos de elemento (ponto, linha, plano…). Clicar num tipo restringe a seleção e **proíbe** pegar o tipo desligado **[V]**.
  - Ícones à direita: **Feature Element Filter** (seleciona o sketch inteiro) e **Geometrical Element Filter** (parte do sketch). Com os dois ativos, o filtro de feature prevalece **[V]**.
  - Bugs conhecidos documentados pela IBM: lista vazia deixa tudo ou nada selecionável; uma linha ainda pode ser pega pelo filtro de curva; filtros não se aplicam a elementos da multisseleção **[V]**.
- **CAD avançado A:** *Selection Filter* limita o cursor a tipos de entidade; o **funil roxo** ao lado do cursor mostra o filtro ativo. Atalhos: **E** arestas, **X** faces, **V** vértices, **F5** mostra a caixa, **F6** liga/desliga **[V]**. "Selection Sets" guardam seleções e "Power Select"/"Advanced Select" selecionam por característica (tipo, ângulo da aresta, cor) **[V]**.
- **CAD avançado C:** Select com prioridades: **Face and Edge**, **Feature**, **Part**, **Component** (e Leaf Part em apresentação) **[V]**. "Select Visible Only", "Compact Selection Set" e "Percentage Visible" (Externo, Interno, Tudo na câmera) controlam seleção em montagem **[V]**. Mini barra pode trocar a prioridade sem fechar o comando **[V, fórum]**.
- **Fusion:** *Selection Priority*: Component, Body, Face, Edge. Escolhe-se um e o resto é excluído até desligar **[V]**.
- **Fenkra hoje:** três caixas (faces, arestas, vértices) em `Settings` (`settings.rs:304-306`) na página de configurações (`app.rs:7342`). `picking::Filter` ainda tem `planes: true` fixo e `exclude_sketch`. Não há filtro na barra, nem de corpo, sketch, componente, referência.
- **Melhor prática:**
  - Barra "Seleção" já existe (`Group::Selecao`); ali, botões de filtro (Tudo, Corpo, Face, Aresta, Vértice, Sketch, Referência, Anotação) + indicador no cursor.
  - Em montagem, prioridade Componente / Peça / Feature / Face.
  - Comandos que pedem um tipo (por exemplo "Filete") aplicam o filtro **temporário** automaticamente e restauram ao terminar (o CAD avançado C e o CAD avançado A fazem algo equivalente **[C]**).
  - **Testes:** filtro Face ignora arestas; ao abrir "Filete" só arestas são pegáveis; fechar restaura o filtro anterior; filtro vazio não bloqueia tudo (bug do CAD avançado B).

### 2.16 Seleção por caixa e laço
**(CAD avançado A: Box/Cross Selection, Lasso / CAD avançado B: Selection Trap / CAD avançado C: janela / Fusion: janela e pintura / Fenkra: `box_mode`)**

- **CAD avançado A:** arrastar da **esquerda para a direita** = *janela* (só o que está totalmente dentro; contorno azul contínuo); da **direita para a esquerda** = *cruzada* (tudo que a caixa toca; verde tracejado) **[V]**. Entidade padrão: peça → arestas; montagem → componentes; desenho → entidades **[V]**. Shift soma, Ctrl inverte (help de 2011) **[V]**. Laço seleciona por contorno desenhado **[V]**.
- **CAD avançado B:** a Selection Trap seleciona por região em torno da bússola; a forma exata do laço é configurável **[V parcial]**.
- **CAD avançado C:** a seleção por janela não existe em todos os comandos; só em alguns (por exemplo delete face, a partir de 2016) **[V]**.
- **Fenkra hoje:** `box_mode` (`settings.rs:354`): 0 Ctrl+arrastar (direção do arrasto), 1 janela, 2 cruzada, 3 pela direção sem Ctrl. Desenho: retângulo azul sólido (janela) ou verde tracejado (cruzada) com rótulo "cruzada (toca)"/"janela (inteiro dentro)" e toast com a contagem (`app.rs:5686-5718`). `picking::box_select` (371) respeita o filtro.
  - **Bug provável:** `box_mode` > 0 intercepta o botão esquerdo no 3D; no estilo de navegação Fenkra o esquerdo também gira (`N::Fenkra`: `left && !shift`, 5389). O conflito é tratado por `!boxing`, mas arrastar começando em cima de um elemento selecionado (e com `box_mode == 3`) pode impedir girar. Testar.
- **Melhor prática:** o padrão deve ser a **direção do arrasto** (convenção CAD avançado A e CAD avançado J **[C]**) sem precisar de Ctrl, com laço opcional. **Testes:** janela só pega elementos inteiros; cruzada pega parciais; Shift soma; o filtro vale para a caixa.

### 2.17 Seleção em cadeia, laço de face e tangência
**(CAD avançado A: Select Chain / Select Loop / Select Tangency / Select Partial Loop / Fenkra: `tangent_chain` só em chapa)**

- **CAD avançado A (3 comandos distintos) [V]:**
  - **Select Chain** (sketch): seleciona entidades conectadas **do mesmo tipo** (geometria ou construção) nos dois sentidos. Para em ramificação ou na troca de tipo; não exige tangência; Ctrl seleciona várias cadeias.
  - **Select Loop** (modelo): arestas que formam um laço em volta de uma face; não exige tangência; uma **alça** troca para a face vizinha; Ctrl para vários.
  - **Select Tangency**: curvas/arestas/faces conectadas **e tangentes**; para onde a tangência acaba. Útil para selecionar um lado de peça de chapa.
  - **Select Partial Loop**: primeira aresta, depois clique direito na última **[V]**.
- **CAD avançado B:** "Propagation: Tangent continuity/Point continuity" nos diálogos de filete e extração **[C]**.
- **CAD avançado C:** "tangent selection set" com Ctrl para várias faces ou arestas **[V]**.
- **Fenkra hoje:** `picking::tangent_chain` (`picking.rs:506`) só serve ao comando de aba curva de chapa (`app.rs:1620`). O filete (`EdgeSel::Edges`) escolhe aresta por aresta (5632). Há Extract com propagação (B21 planejado).
- **Falta:** menu "Selecionar laço / tangência / cadeia" no 3D; propagação marcada como atributo do comando (ao recalcular, recalcula a propagação, conforme B21).
- **Melhor prática:** itens no menu de contexto da aresta/face: "Laço", "Tangência", "Cadeia" (sketch), e **Ctrl** para somar. A seleção gravada no comando guarda a **regra** (propagação tangente a partir da semente), não só a lista, para sobreviver a mudança (princípio 1). **Testes:** filete em cantos arredondados seleciona a cadeia tangente completa; mudar o raio do canto não perde nenhuma aresta; laço troca de face pela alça.

### 2.18 Multisseleção e teclas modificadoras
**(Ctrl/Shift/Alt no CAD avançado B, CAD avançado A, CAD avançado C / Fenkra: `multi` na árvore, `picks` na viewport)**

- **CAD avançado H (árvore):** hover realça a feature; clique seleciona; Ctrl-clique soma/tira; Shift-clique seleciona o intervalo **[V]**.
- **CAD avançado A e CAD avançado C:** Ctrl soma/tira; Shift é intervalo na árvore **[C]**; Ctrl com "tangent selection set" no CAD avançado C **[V]**.
- **CAD avançado B:** Ctrl soma; Shift em árvore seleciona faixa **[C]**. Em diálogos, a seleção é guiada (campo ativo) **[C]**.
- **Fenkra hoje:**
  - Árvore: `multi` (Ctrl soma/tira, Shift faixa) e `tree_order` (`app.rs:190-194`).
  - Viewport: `picks` com Ctrl soma/tira, Shift soma, clique em único elemento selecionado limpa (5656-5677).
  - `Sel` (23) é um único item, `Copy`.
  - **Inconsistência:** são **dois modelos** de seleção sem ponte. O que se seleciona na árvore (`multi`) não vira `picks` e vice-versa, exceto casos tratados (`fill_from_selection`, 4660).
  - **Bug provável:** "clique em único selecionado limpa" (5671) difere do CAD avançado A e do CAD avançado C (que mantêm) e surpreende.
  - Falta Alt (Select Other ou desmarcar).
- **Melhor prática:** um **conjunto de seleção único** (`Selection { items: Vec<SelItem> }` com `SelItem::{Tree(Id), Pick(Pick)}`), com regras uniformes: clique troca; Ctrl soma/tira; Shift soma (3D) ou intervalo (árvore); Esc limpa; clique no vazio limpa; **clique no único selecionado não limpa**. **Testes:** selecionar na árvore realça na viewport e vice-versa; Esc limpa os dois.

### 2.19 Painéis de comando
**(CAD avançado A: PropertyManager / CAD avançado B: diálogos (dialog boxes) com campos de seleção / CAD avançado C: caixas de diálogo e mini barra / Fenkra: `Session` + `feat_ui`/`geo_ui` + "Concluir/Cancelar")**

- **CAD avançado A:** o PropertyManager ocupa o painel esquerdo no lugar da árvore. Caixas de seleção com lista **expansível** (arrastar a borda inferior) **[V]**. O "D" traz o ✔ e ✘ ao cursor **[V]**. Seleções são guiadas: a caixa ativa recebe o próximo clique **[C]**.
- **CAD avançado B:** diálogo flutuante com campos "Support / Profile / …", cada um ativo ao clicar. Dentro dos diálogos, o preview mostra o resultado. "More>>" revela opções avançadas **[C]**. Selecionar na árvore ou na geometria preenche o campo **[C]**. "Preview" liga e desliga **[C]**.
- **CAD avançado C:** diálogo com abas (Forma, Mais) e a mini barra com OK/Cancelar perto do cursor **[C]**.
- **Fenkra hoje:**
  - `Session` (228) abre o comando como **ambiente**; painel de edição à esquerda ou à direita (`editor_left`, `settings.rs:339`); árvore ao lado (`tree_peek`, 173).
  - `conclude_bar`/`done_bar` (4569, 6201) com Concluir/Cancelar; Enter/Esc (6501-6506).
  - `fill_from_selection` (4660) preenche campos a partir da seleção.
  - `slots.rs` e `geo_alternatives` (4753) oferecem candidatos por campo.
  - B20 (suporte → modo → parâmetros → avançado) está no backlog.
- **Falta:** lista de seleção expansível; preview ligável; tecla D/Enter no cursor; "próximo campo" automático.
- **Melhor prática (B20):** ordem fixa **Suporte → Modo → Parâmetros → Avançado** (recolhido). Campo ativo destacado; o clique na viewport preenche o campo ativo e passa ao próximo quando válido. Painel do comando mostra "o que falta" (campos obrigatórios) na barra de status. Preview sempre ligado, com opção de desligar para modelos pesados. **Testes:** todo comando tem os 4 blocos na ordem; Enter conclui só se válido; Esc cancela sem criar passo de undo.

### 2.20 Mini barra e barra de contexto
**(CAD avançado A: Context Toolbar / CAD avançado C: Mini-toolbar / CAD avançado B: barra de contexto limitada / Fenkra: —)**

- **CAD avançado A:** ao selecionar itens na viewport ou na árvore, aparece uma barra de contexto próxima ao cursor com os comandos aplicáveis (adicionar relação, mate, ocultar, transparência, construção) **[V]**. É personalizável (botão direito > Customize) **[V]**. A barra do atalho "S" depende do **modo** (peça, montagem, sketch, desenho), não do objeto **[V]**.
- **CAD avançado C:** a mini barra aparece em alguns comandos e muda o filtro de prioridade sem fechá-lo **[V, fórum]**.
- **Fenkra hoje:** não há barra flutuante. O clique direito abre `viewport_menu` (4422) com comandos por tipo.
- **Melhor prática:** P2. Após selecionar, mostrar 3 a 5 botões (os mais usados do menu de contexto, por tipo). **Teste:** selecionar uma face oferece Sketch, Furo, Extrair, Casca.

### 2.21 Menus de contexto, marking menu e gestos
**(CAD avançado C: Marking Menu / CAD avançado A: Mouse Gestures e S-key / CAD avançado B: menu de contexto / Fenkra: `viewport_menu`, menus da árvore)**

- **CAD avançado C:** botão direito abre menu **radial** no cursor, com setores por comando. Em modo *mark* basta traçar em direção ao item e soltar rápido. Os comandos mudam com o ambiente (sketch: círculo, Finish Sketch; peça: furo, extrusão; montagem: restrição, mover componente). Um menu de contexto curto aparece em cima ou embaixo. Personalizável na aba Marking Menu **[V]**.
- **CAD avançado A:** **gestos do mouse**: clique direito e arrastar na direção do comando; até 12 comandos; conjuntos separados para peça, montagem, desenho e sketch **[V]**. Menu de contexto personalizável, com itens menos usados atrás de um botão "mais" **[V]**.
- **CAD avançado B:** menu de contexto por objeto, sem radial **[C]**.
- **Fenkra hoje:** `viewport_menu` já é por tipo (face, aresta, vértice, referência, curva do sketch, ponto, plano, vazio), com "criada por…" e "Editar o comando que criou" (`app.rs:4473-4477`) — bom. As ações vão para `ctx_actions` e executam depois (5729, 6814). Tem "⊙ Centro de órbita aqui".
  - Falta: menu configurável; radial/gestos; ordem por uso.
- **Melhor prática:** manter o menu por tipo (já bom); adicionar o campo "Editar o comando que criou" também para arestas e vértices; marking menu como P3.

### 2.22 Bancadas (workbenches / módulos)
**(CAD avançado B: Start > workbench / CAD avançado A: CommandManager por aba e contexto / CAD avançado C: ambientes / Fenkra: `Bench`, `Group::in_bench`)**

- **CAD avançado B:** cada workbench traz suas barras; a Start menu e as preferências definem quais aparecem; mudar de bancada não muda a árvore da peça **[C]**.
- **Fenkra hoje:** `Bench` (`settings.rs:161`): Sólidos, Superfícies, Montagem, Chapa, Estrutural, Desenho. `Group::in_bench` (94) define quais grupos aparecem. Modelo único por documento. Há abas de bancada (`bench_tab`, `app.rs:7404`).
  - **Bug provável:** `Group::Referencias => b == Bench::Solidos` (114) e "na bancada de superfícies o Wireframe mostra as referências abertas" cria dois caminhos para o mesmo conceito (o CAD avançado B usa Wireframe & Surface). Testar: criar um plano na bancada de Superfícies e vê-lo em Sólidos.
- **Melhor prática:** o item de comando tem uma lista de bancadas em que aparece (já implícita), e uma **busca de comandos** global (ver 2.24) que encontra comandos de outras bancadas e **troca de bancada** ao executar. **Teste:** buscar "Filete" na bancada de Superfícies oferece o comando e muda a bancada sem perder a seleção.

### 2.23 Personalização de barras e grupos
**(CAD avançado B: Tools > Customize / CAD avançado A: Customize, barras de atalho / CAD avançado C: Customize, ribbon / Fenkra: `Customization`, `customize_window`)**

- **CAD avançado B:** Tools > Customize > Commands permite atribuir atalhos e montar barras **[V parcial]**.
- **CAD avançado A:** CommandManager, barras de atalho (S), gestos, barra de contexto e menu de contexto são personalizáveis **[V]**.
- **CAD avançado C:** aba Customize, Marking Menu, atalhos **[V]**.
- **Fenkra hoje:** `Customization { hidden, order, groups }` (`settings.rs:372`): ocultar entradas, ordenar dentro do grupo, grupos próprios (`CustomGroup`, 383). Cada grupo pode ficar em Topo, Lateral, Direita, Flutuante ou Oculto (`Dock`, 242), com arraste animado (`DockDrag`). Famílias de comandos lembram a última variação (`Settings.families`). `customize_window` (`app.rs:2121`).
  - **Falta:** exportar/importar a configuração (o CAD avançado C tem **[V]**: import/export de Application Options); restaurar padrões por grupo; perfis por bancada.
  - **Bug provável:** `entry_key` usa `ícone|nome` (`toolbar.rs:66`). Renomear um comando ou trocar o ícone quebra a personalização guardada de todos os usuários (chave instável).
- **Melhor prática:** chave **estável** (id do comando, não texto); versão do esquema em `config.json`; migração. Exportar/importar perfil. **Testes:** renomear um item no código não perde a ordem do usuário (teste de regressão com `config.json` antigo).

### 2.24 Atalhos de teclado e busca de comandos
**(CAD avançado A: Customize > Keyboard, tecla S, busca / CAD avançado B: Customize > Commands / CAD avançado C: atalhos customizados / Fenkra: `Keymap`, 8 predefinições)**

- **CAD avançado A:** praticamente qualquer comando recebe atalho (Tools > Customize > Keyboard); sugerem limpar os que não usa **[V]**. **F** = zoom to fit; clique duplo na roda também **[V]**. Tecla S: busca de comandos (digitar) e barra de atalho **[V]**.
- **CAD avançado B:** atalhos pela Customize; poucos de fábrica **[V parcial]**.
- **Fenkra hoje:** `KeyAction` com **38 ações** (`keymap.rs:10-59`), `Preset` com 8 predefinições (CAD avançado A, CAD avançado B, FreeCAD, CAD avançado D, Blender, CAD avançado E, CAD avançado C + Fenkra) que também escolhem o estilo de navegação (`Preset::nav`, 198). `Keymap.custom` guarda trocas; `conflicts` (301) avisa por contexto (sketch × geral). Há um teste que valida todas as predefinições e ausência de conflitos (311).
  - **Falta:**
    - Atalhos de **qualquer** comando da barra (só 38 ações, não os mais de cem de `toolbar.rs`).
    - F2, F5, Ctrl+F5, Esc, Enter e Del estão **fixos** no código (`app.rs:6548-6556`) e fora do mapa; as dicas de menu escrevem "H" e "N" fixos ("Mostrar   H", `app.rs:4067`) e **divergem** quando o usuário troca o atalho.
    - Sem busca de comandos.
  - **Bug provável:** `Preset::CAD avançado C` define `SkSelect = "Escape"` e depois o remove com `retain` (`keymap.rs:266-267`). Código morto que confunde. `ViewMenu = "Space"` em quase todos conflita com o uso comum de Space para outras coisas (por exemplo ocultar no FreeCAD, que o código trata trocando para `Shift+Space`).
- **Melhor prática:**
  - Toda `Act` do `toolbar.rs` ganha id estável e entra no `Keymap`.
  - As dicas lêem o atalho **atual** do mapa.
  - **Busca de comandos** (Ctrl+K ou tecla S): lista nome + bancada + atalho, executa e troca de bancada.
  - **Testes:** nenhum atalho fixo no código sem estar no mapa; trocar "Ocultar" para outra tecla muda o texto do menu; busca "fur" acha "Furo".

### 2.25 Dicas (tooltips) e ícones
**(CAD avançado C: tooltips de 2 níveis + ToolClips / CAD avançado A: dicas com figura / CAD avançado B: dica de uma linha + ajuda / Fenkra: `hover_help`, `tool_button`)**

- **CAD avançado C:** a aba General das Application Options tem "Show Tooltips" e a duração da dica de **segundo nível**; ToolClips são vídeos dentro dela **[V]**.
- **Fenkra hoje:** `hover_help` (`icons.rs:403`): dica imediata com título + descrição breve; após **5 s** parado, a descrição avançada. Só texto. `tool_button` (372) desenha ícone grande + nome. O catálogo de ícones é um `match` gigante de SVG por nome (`icons::get`, 7), com o ícone `config.svg` como fallback silencioso (365) — um nome errado não acusa erro.
  - **Falta:** figura de entrada → operação → resultado (ADR 0011, princípio 6; B60); atalho na dica; configurar o atraso; dica na barra de status.
  - **Bug provável:** `_ => config.svg` esconde ícones faltantes. Um teste de CI deve falhar se algum `Item.icon` cair no fallback.
- **Melhor prática:** dois estágios (nome + atalho + 1 linha → após o atraso, mini figura de 3 painéis); atraso configurável (padrão 1,5 s, em vez de 5 s); a barra de status mostra a descrição breve ao passar o mouse. **Teste:** percorrer `all_entries()` e exigir que cada ícone exista e cada `brief` seja não vazio e traduzido.

### 2.26 Navegação de câmera e cubo de vistas
**(CAD avançado B: bússola, Reframe On, Fit All In, Zoom Area / CAD avançado A: Heads-up View, F, Space / CAD avançado C: ViewCube, SteeringWheel / Fenkra: `NavStyle`, `Camera`, `viewcube`)**

- **CAD avançado B:** botão do meio = pan; meio + esquerdo/direito = zoom/rotação **[V]**. A **bússola** orienta o modelo; clicar num rótulo X/Y/Z vira para o eixo **[V]**. **Reframe On** centra o componente selecionado e dá zoom **[V]**. **Fit All In** enquadra tudo; **Zoom Area** com caixa **[V]**. Navegação compatível do CAD avançado B existe em outros programas (CAD avançado F View, Illustrate) **[V]**.
- **CAD avançado A:** **F** = ajusta; duplo clique na roda também; **Space** abre o seletor de orientação **[V]**.
- **CAD avançado C:** ViewCube + roda de navegação **[C]**.
- **Fenkra hoje:**
  - 9 estilos (`NavStyle`, `settings.rs:187`), implementados em `app.rs:5388-5398`, com a ajuda na barra de status (5353).
  - Órbita: `yaw -= dx·0,008`, `pitch` limitado a ±1,5706 (5409-5410).
  - Zoom pela roda em relação ao **alvo** (`dist·exp(±scroll·0,0015)`, 5415), limitado a 0,5…1e6.
  - Duplo clique ajusta se não há elemento sob o cursor (5417).
  - Animação de 0,45 s com ease-out (`render.rs:462-501`), e o usuário interrompe (`stop_anim`).
  - Cubo (`viewcube.rs`) com 6 faces, arestas e cantos, arrastável; seletor grande (Espaço) com vistas padrão, dimétrica, trimétrica e vistas salvas.
  - Ortogonal/perspectiva (`Camera.ortho`, FOV fixo 0,6 rad). "Centro de órbita aqui" no menu 3D.
  - **Bugs prováveis:**
    - **Pan** usa `k = dist·0,0012` (5404) independente da altura da viewport e do FOV: o modelo "escorrega" em relação ao cursor.
    - **Zoom ao cursor** não existe (zoom só ao alvo).
    - O `pitch` não passa de ±90°, então não dá para "virar de cabeça para baixo" (alguns CADs permitem).
    - `Camera::fit` usa a diagonal × 1,6 (375) mesmo em perspectiva/ortogonal, e o plano de corte é `dist·0,01…dist·50` (363): com `dist` muito pequeno após zoom, faces do modelo longe podem ser cortadas.
    - Inverter zoom: `invert_zoom != (nav == CAD avançado A)` (5413) — dois sinais combinados; difícil de explicar ao usuário.
  - Falta: rotacionar em torno da seleção automaticamente; "Reframe On" da seleção (existe só "Normal à seleção" `N` e "Ajustar tudo"); zoom por caixa; vista anterior/seguinte; velocidades configuráveis.
- **Melhor prática:** pan proporcional (`2·dist·tan(fov/2)/altura_px`), zoom ao cursor opcional (padrão ligado, como em CAD avançado A/Fusion **[C]**), "Ajustar à seleção" (Reframe On), vista anterior (Alt+←). **Testes:** pan de 100 px move o ponto sob o cursor por 100 px na tela em qualquer zoom (teste numérico com `project`); zoom ao cursor mantém o ponto sob o cursor; cubo: clicar em cada face/aresta/canto leva à vista esperada (26 direções).

### 2.27 Qualidade gráfica (tesselação, LOD, anti-aliasing, realismo)
**(CAD avançado B: Display > Performance (3D accuracy), Pixel culling, LOD, Occlusion Culling / CAD avançado A: Image Quality (Document Properties) / CAD avançado C: Visual Styles, Display options / Fenkra: `DisplayMode`, `SAMPLES`, `defl`)**

- **CAD avançado B:**
  - **3D Accuracy** (Tools > Options > Display > Performance): *Fixed* (um valor de flecha para todos, de 0,01 a 10) ou *Proportional* (depende do tamanho: objetos grandes ficam mais grossos) **[V]**. O valor de flecha ("sag") vale para curvas e superfícies **[V]**.
  - Fixed com valor muito pequeno (0,02) pode impedir exibição de superfície por excesso de triângulos (relato IBM) **[V]**. Círculos pequenos "poligonais" se corrigem com Proportional e mínimo 0,01 **[V, GrabCAD]**.
  - **Anti-aliasing** em Tools > Options > General > Display > Visualization; desligar melhora desempenho **[V]**.
  - **Pixel culling** (tamanho mínimo em pixels): objetos menores ficam estáticos durante a manipulação **[V]**.
  - **LOD** reduz polígonos de objetos distantes; **Occlusion culling** só desenha o visível, recomendado para montagens grandes **[V]**.
  - **Cache CGR** para montagens grandes **[V]**.
- **CAD avançado A:** Document Properties > **Image Quality**: controle deslizante de resolução sombreada (ligada a um valor de **desvio** inversamente proporcional), qualidade das arestas em vista de desenho, "Optimize edge length" (mais qualidade, mais lento e arquivo maior) **[V]**. O mesmo item avisa: a exibição **não altera a tolerância do modelo** **[V]**.
- **CAD avançado C:** estilos visuais e opções de exibição por aplicação; qualidade por documento **[C]**.
- **Fenkra hoje:**
  - `DisplayMode` com 8 modos (`render.rs:190`): sombreado com arestas, sombreado, linhas ocultas, arame, raio-X, semi-renderizado, zebra, curvatura.
  - **MSAA 4×** (`SAMPLES: u32 = 4`, 451), `Depth32Float`, `Rgba8UnormSrgb`, shader próprio (WGSL) com texturas triplanares e linhas largas.
  - **Tesselação:** em `fenkra-solidos/src/lib.rs:1644` `defl = (diagonal × 0,0015).clamp(0,005; 1,0)`; no kernel `BRepMesh_IncrementalMesh(forma, defl, relativo=false, 0,35 rad, paralelo=true)` (`kernel.cpp:833`). Outros caminhos usam fatores diferentes: superfícies 0,002 (4422), PMI 0,01 (`pmi_ui.rs:173`), montagem `size·1e-3` com mínimo 0,01 (`view.rs:127`), desenho 0,002 (`source.rs:357`).
  - **Bugs prováveis:**
    - O ângulo de 0,35 rad (~20°) é fixo: furos pequenos em peça grande mostram polígonos. O teto de 1,0 mm limita peças grandes.
    - A malha de exibição também alimenta `pick`/`box_select` (`picking.rs:166`), então a qualidade gráfica afeta a **seleção**. O princípio 7 do ADR 0011 diz que gráfica "nunca altera o modelo, só a vista"; o código atual não separa as duas malhas.
    - Fatores de flecha diferentes por caminho dão aparência inconsistente entre peça, montagem, PMI e desenho.
  - Falta: controle do usuário (nível, em configurações por **aplicação** e por **documento**); LOD e *pixel culling* em montagem; seletor de MSAA; qualidade "boa" na captura/exportação.
- **Melhor prática (B61):**
  1. Um só `MeshQuality { linear_rel, angular, min_abs, max_abs }` com níveis Rascunho / Normal / Alto / Máximo.
  2. Padrão por aplicação, **sobreposição** por documento (como o CAD avançado A, que guarda no documento **[V]**).
  3. Malha de exibição **separada** da malha de medida e de exportação (exportação STL/mesh e medições usam qualidade própria, nunca a da tela).
  4. Em montagem, LOD por distância em pixels e *pixel culling* durante a órbita.
  5. MSAA 2/4/8 configurável com fallback.
  - **Testes:** malha fechada e sem buracos em todos os níveis (corpus B53); um furo de Ø 2 mm em peça de 2 m tem ≥ 16 segmentos no nível Alto; mudar o nível **não** muda volume, área nem medidas (compara com tolerância 0); captura visual de referência por nível.

### 2.28 Configurações por aplicação × por documento
**(CAD avançado A: System Options × Document Properties / CAD avançado C: Application Options × Document Settings / CAD avançado B: Tools > Options (por usuário/ambiente) / Fenkra: `Settings` em `%APPDATA%\Fenkra\config.json`)**

- **CAD avançado A:** opções de sistema ficam no registro e **não** estão no documento; propriedades do documento ficam **no arquivo** e podem vir de modelos (templates) **[V]**. Algumas propriedades de desenho não vão para o template **[V, comentário de usuário]**.
- **CAD avançado C:** Application Options (global, em `UserApplicationOptions.xml`) × Document Settings (no arquivo; para novos arquivos, mudar nos templates) **[V]**. Há importação e exportação das opções **[V]**.
- **CAD avançado B:** Tools > Options é por usuário e pode ser bloqueado (admin) **[V parcial]**; algumas escolhas vão no documento (por exemplo parâmetros, unidades) **[C]**.
- **Fenkra hoje:** tudo em `Settings` global (`settings.rs:285`), um JSON por usuário. Dentro dele misturam-se preferências de aplicação (tema, atalhos, docas) e valores que **deveriam** ser do documento: `dim_standard`, `dim_decimals`, `dim_text_height` (cotagem), `grid_step`, `point_style`, `ref_line_width`, `pmi_style`, `flanged_holes`, `user_profiles`. `save()` ignora o disco durante demos/testes (`FENKRA_DEMO*`, 470-474).
  - **Bugs prováveis:** abrir a peça de um colega com outro padrão de cotagem mostra cotas no padrão **do usuário**, não do arquivo. `Settings::load` ignora erro de parse e usa o padrão (`unwrap_or_default`, 467): um `config.json` corrompido **apaga** silenciosamente toda a personalização no próximo `save`.
- **Melhor prática:**
  - Separar em **Preferências** (aplicação) e **Propriedades do documento** (no `.fkp`, com valores iniciais vindos do modelo de documento).
  - Tabela única que classifica cada opção (A = aplicação, D = documento, AD = padrão da aplicação + sobreposição).
  - Em caso de `config.json` inválido, fazer **cópia de segurança** antes de recriar.
  - **Testes:** abrir o mesmo arquivo com dois `config.json` diferentes produz as mesmas cotas; `config.json` corrompido gera `config.json.bak` e aviso.

### 2.29 Mensagens e barra de status
**(CAD avançado A: barra de status, mensagens de reconstrução / CAD avançado C: barra de status, mensagens com hierarquia / CAD avançado B: linha de mensagem / Fenkra: painel inferior `timeline`, toasts)**

- **CAD avançado C:** os diálogos de erro mostram hierarquia até a causa; a cruz vermelha na barra de acesso rápido indica erros não resolvidos; avisos persistem até a causa sumir; prompts podem ser suprimidos em Application Options > Prompts **[V]**.
- **CAD avançado A:** a barra de status mostra a descrição do comando sob o cursor e o estado (reconstruindo, GDL do sketch) **[C]**.
- **Fenkra hoje:** painel inferior `status` (`app.rs:5287`): estado do cálculo (⏳/⟳/▶/✔ com ms), caixa "auto", Atualizar, Reconstruir, controles de passo ⏮◀▶⏭, GDL livres da montagem, botão de erros, nome do kernel, erro de parâmetro e a ajuda de navegação à direita. **Toasts** (`toast`, 772; `toasts`, 5890) para avisos efêmeros.
  - **Falta:** "próximo passo" do comando ativo (ADR 0011, princípio 6); descrição do comando sob o cursor; histórico de mensagens; severidade nos toasts (hoje só `error: bool`); mensagens com ação (por exemplo "Desfazer").
  - **Bug provável:** toasts efêmeros perdem erro importante se o usuário olhou para outro lugar. Erros devem ir também para um registro consultável.
  - **Textos fixos em português:** vários `toast("...")` não passam por `tr()` (por exemplo 797, 6566), então o modo inglês (`english`) mistura idiomas.
- **Melhor prática:** barra de status em 3 zonas — (esquerda) próximo passo / descrição, (centro) estado do cálculo e objeto em trabalho, (direita) contadores (erros, avisos, ocultos, selecionados). Registro de mensagens em painel. **Teste:** todo texto visível passa por `tr()` (teste que varre `toast(` e literais); mensagens de erro têm código.

### 2.30 Busca e filtro na árvore
**(CAD avançado E: filtro e comandos `:erro` / CAD avançado A: FeatureManager Filter / CAD avançado C: Find / CAD avançado B: Edit > Search / Fenkra: só em referências externas)**

- **CAD avançado A:** caixa de filtro acima do ícone da peça **[V]**, mais tags **[V]**.
- **CAD avançado C:** Find (relações quebradas, itens suprimidos, valores de parâmetros, critérios combinados) **[V]**.
- **CAD avançado E:** busca por nome, tipo, pasta ou peça; comandos `:part`, `:type`, `:name`, `:errors`, `:warnings`, `:folder`, `:variable`, `:suppressed`, `:hidden`, `:shown` **[V]**.
- **CAD avançado B:** Edit > Search seleciona por critério **[V parcial, GrabCAD "search and automatically select"]**.
- **Fenkra hoje:** filtro de texto e "só quebrados" só em Referências externas (`app.rs:3390`).
- **Melhor prática:** caixa no topo da árvore com mesma sintaxe de `:`; resultados selecionam e realçam; combina com a seleção única (2.18). **Teste:** `:erro` mostra só as linhas com `✖`; `:suprimido` só riscadas.

---

## 3. Requisitos levantados

Prefixo **INT** (interface e árvore). Verificar colisão com os `RF-UI-xxx` já usados nos requisitos existentes antes de gravar.

### Funcionais

| ID | Requisito | Prio. | Critério de aceite |
|---|---|---|---|
| RF-INT-01 | Existe **um único objeto em trabalho** por peça, sublinhado na árvore e citado na barra de status. O próximo comando entra depois dele. | P0 | Teste: definir o objeto em trabalho no meio; criar um furo; o furo aparece logo depois dele e as features seguintes atualizam. |
| RF-INT-02 | Comando "Definir como objeto em trabalho" (feature, corpo, pasta) e modo Scan (◀ ▶). | P1 | Scan percorre N features em N passos e a viewport mostra o estado de cada passo. |
| RF-INT-03 | Salvar com o objeto em trabalho fora do fim exibe aviso com ação "Mover para o fim". | P1 | Teste de UI: salvar retrocedido mostra o aviso; confirmar leva ao fim e salva. |
| RF-INT-04 | Painel **Diagnóstico** (substitui o assistente): erros de cima para baixo, com causa-raiz, e ações Realçar, Editar, Substituir referência, Isolar, Suprimir, Apagar. | P0 | Corpus: apagar a face de apoio de um furo → erro `RefPerdida` na causa; "Substituir" reconstrói; a face removida é realçada na viewport. |
| RF-INT-05 | Cada erro tem **código estável** (`ErrorKind`) e referência ao primeiro pai que falhou. | P0 | Teste unitário: cada falha do corpus produz um `ErrorKind` não genérico; a causa-raiz aponta para o pai correto. |
| RF-INT-06 | Política de erro na reconstrução: Parar / Continuar / Perguntar, nas configurações. | P2 | Com "Parar", o cálculo para na primeira falha; com "Continuar", calcula o resto e lista todas. |
| RF-INT-07 | Janela/painel "Pais e filhos" para qualquer item, com realce na viewport, e setas dinâmicas ao passar o mouse (liga/desliga). | P1 | Apagar um sketch lista todos os filhos; o realce corresponde; desligar as setas persiste. |
| RF-INT-08 | "Isolar" corta o vínculo de uma referência de geometria (não só de links externos), mantendo a geometria fixa. | P1 | Isolar uma referência; apagar o pai; o filho continua calculando com o mesmo resultado. |
| RF-INT-09 | Atualização: modo automático ou manual **por documento**, "Atualizar esta feature" (local) e **Cancelar** real. | P1 | Em modo manual nada recalcula sozinho; "local" recalcula só a feature e seus filhos; Cancelar interrompe em ≤ 500 ms entre features. |
| RF-INT-10 | Botão "Verificar" que compara cache × reconstrução total e mostra o diff por feature. | P2 | Peça com cache corrompido de propósito: o diff aponta a feature divergente. |
| RF-INT-11 | Desfazer/refazer **nomeados**, com lista de histórico; um comando concluído = 1 passo; cancelar sessão não cria passo. | P0 | Abrir comando, Cancelar, Ctrl+Z → desfaz o passo anterior ao comando, nunca "ressuscita" o comando cancelado. |
| RF-INT-12 | Edição contínua (arrastar cota/alça) vira 1 passo ao soltar, substituindo a janela de 0,8 s. | P1 | 3 edições rápidas = 3 passos; 1 arraste longo = 1 passo. |
| RF-INT-13 | Seleção **única** unificada (árvore + viewport) com regras uniformes (clique troca; Ctrl soma/tira; Shift soma/intervalo; Esc limpa; clicar no único selecionado não limpa). | P1 | Selecionar na árvore realça na viewport e vice-versa; Esc limpa ambos. |
| RF-INT-14 | Filtros de seleção na barra (Tudo, Corpo, Face, Aresta, Vértice, Sketch, Referência, Anotação), com indicador no cursor; comandos aplicam filtro temporário e restauram. | P1 | Abrir "Filete" restringe a arestas; ao concluir o filtro anterior volta; filtro vazio não bloqueia a seleção. |
| RF-INT-15 | "Selecionar outro…" no menu de contexto 3D (lista de candidatos por profundidade) e ciclar por candidatos (Alt+roda). | P1 | Plano de origem sobreposto a uma face: a lista traz os dois; escolher o de trás seleciona o de trás. |
| RF-INT-16 | Seleção de **laço de face**, **tangência** e **cadeia de sketch** no menu 3D, com Ctrl para somar; o comando grava a **regra** de propagação. | P1 | Filete em canto arredondado seleciona a cadeia tangente completa; mudar o raio do canto não perde arestas. |
| RF-INT-17 | Seleção por caixa pela **direção do arrasto** como padrão, com janela e cruzada visualmente distintas e laço opcional. | P2 | Janela só pega inteiros; cruzada pega parciais; o filtro vale. |
| RF-INT-18 | Painel de comando segue **Suporte → Modo → Parâmetros → Avançado** (B20), com campo ativo destacado e avanço automático. | P1 | Revisão visual: 100% dos comandos seguem a ordem; lista de seleção expansível. |
| RF-INT-19 | Barra de contexto flutuante com 3–5 ações após selecionar. | P3 | Selecionar uma face mostra Sketch, Furo, Extrair, Casca. |
| RF-INT-20 | Busca de comandos global (Ctrl+K) que executa e troca de bancada. | P2 | Digitar "fur" lista "Furo" com atalho e bancada; Enter executa. |
| RF-INT-21 | Todo comando da barra é atribuível a atalho; atalhos fixos (F2, F5, Esc, Enter, Del) entram no mapa; dicas leem o atalho atual. | P1 | Trocar "Ocultar" para outra tecla atualiza o texto do menu; varredura de código não acha tecla fixa fora do mapa. |
| RF-INT-22 | Chaves de personalização **estáveis** (id do comando), com migração de `config.json`. | P1 | `config.json` antigo com chave "ícone\|nome" migra sem perder ordem nem ocultos. |
| RF-INT-23 | Exportar/importar perfil de interface (barras, atalhos, docas). | P3 | Exportar, apagar `config.json`, importar → interface idêntica. |
| RF-INT-24 | Dicas em dois estágios com figura (entrada → operação → resultado), atraso configurável (padrão ≤ 1,5 s) e atalho. | P1 | Todo item de `all_entries()` tem figura; o atraso é editável. |
| RF-INT-25 | Descrição do comando sob o cursor e "próximo passo" do comando ativo na barra de status. | P2 | Passar o mouse num botão mostra a descrição breve; num comando aberto, a barra diz o que falta. |
| RF-INT-26 | Ocultar **hierárquico** (pasta/corpo oculta filhos, com override) e contador de ocultos. | P2 | Ocultar a pasta oculta os itens; reexibir só um funciona; salvar e abrir mantém. |
| RF-INT-27 | Regra explícita ordenado × não ordenado (pasta de comandos × pasta livre) com mensagem ao soltar item inválido e "Ordenar por dependência". | P2 | Soltar comando de sólido em pasta livre mostra o motivo; ordenar muda a ordem sem mudar o resultado. |
| RF-INT-28 | Reordenar por arraste com indicador de destino e validação de pais/filhos antes de soltar. | P1 | Mover um pocket para antes do sketch é recusado com a dica "usa X, que vem depois". |
| RF-INT-29 | Câmera: pan proporcional, zoom ao cursor, "Ajustar à seleção", vista anterior. | P1 | Teste numérico: pan de 100 px move o ponto sob o cursor 100 px; zoom ao cursor preserva o ponto. |
| RF-INT-30 | Qualidade gráfica: `MeshQuality` com 4 níveis, padrão da aplicação + sobreposição por documento. | P0 | Alterar o nível não altera volume, área nem medidas; ver RNF-INT-05. |
| RF-INT-31 | Malha de exibição separada da malha de medida/exportação. | P1 | Exportar STL/medir com o nível Rascunho dá o mesmo resultado que com o Máximo. |
| RF-INT-32 | Configurações classificadas em **aplicação** × **documento**; as de documento (cotagem, grade, PMI) vão no arquivo. | P1 | Abrir o mesmo arquivo com dois `config.json` diferentes mostra as mesmas cotas. |
| RF-INT-33 | Registro de mensagens consultável, com severidade e ação (por exemplo "Desfazer"). | P2 | Erro some do toast mas continua listado no registro. |
| RF-INT-34 | Filtro/busca da árvore com sintaxe `:erro`, `:suprimido`, `:oculto`, `:pasta`. | P1 | `:erro` mostra só linhas com `✖`. |
| RF-INT-35 | Ponto de parada (Stop Update) com nota, respeitado pelo recompute. | P3 | O update para na marca, mostra a nota e continua a pedido. |
| RF-INT-36 | Supressão por expressão de parâmetro. | P3 | Parâmetro `usa_furo = 0` suprime o furo; `1` reativa. |

### Não funcionais

| ID | Requisito | Prio. | Critério de aceite |
|---|---|---|---|
| RNF-INT-01 | O desenho da árvore não clona o documento nem serializa features a cada quadro; o estado de "desatualizado" é calculado só quando o documento muda. | P0 | Peça com 300 comandos parada: custo da árvore < 2 ms/quadro; medido em teste de benchmark. |
| RNF-INT-02 | A interface não bloqueia durante recompute (já existe `Worker`); cancelamento responde em ≤ 500 ms. | P1 | Rodar peça pesada, pedir Cancelar, medir o tempo até a UI reagir. |
| RNF-INT-03 | Memória do desfazer limitada (MB), não só contagem (200). | P1 | Peça de 2 000 comandos: 200 passos < teto definido (a fixar com o corpus). |
| RNF-INT-04 | Textos visíveis passam por `tr()`; nenhum literal solto em `toast`. | P2 | Teste de varredura falha se achar literal sem `tr`. |
| RNF-INT-05 | Qualidade gráfica nunca altera o modelo (princípio 7 do ADR 0011). | P0 | Teste de propriedade: volume/área/medidas idênticos em todos os níveis. |
| RNF-INT-06 | Falha ao ler `config.json` gera backup e aviso; nunca apaga em silêncio. | P1 | `config.json` truncado → `config.json.bak` + aviso; a interface abre com padrões. |
| RNF-INT-07 | Todo ícone referenciado existe; o fallback `config.svg` falha no teste de CI. | P2 | Varrer `all_entries()`: nenhum ícone cai no `_ =>`. |
| RNF-INT-08 | A interface funciona de 1280×720 até 4K e `ui_scale` 0,7–2,0 sem sobreposição de grupos. | P2 | Capturas nas resoluções-alvo (corpus visual). |
| RNF-INT-09 | Nenhuma ação de interface muda a seleção do modelo em silêncio (o ADR 0011, princípio 1, vale também para a UI). | P1 | Teste: aplicar filtro/ocultar/reordenar não altera a seleção atual. |

---

## 4. Itens de backlog propostos

Os itens B54, B59, B60, B61 e B20 já existem. Proponho **estender** os existentes e abrir itens novos onde a lacuna é própria deste tema. Tamanhos: P ≤ 60 k tokens, M ≈ 60–150 k, G > 150 k.

| Título | O que muda | Prio. | Tam. | Dependências |
|---|---|---|---|---|
| **Desempenho da árvore (quadro a quadro)** | Remove `doc.clone()` e a serialização JSON por quadro (`tree()`, `compute_stale`); índice incremental de mudança por feature. | P0 | M | — (base de B59) |
| **Objeto em trabalho único** | Unifica `active_collector`, `marker` e `Session` num só objeto em trabalho; sublinhado na árvore; "Definir como objeto em trabalho"; modo Scan; aviso ao salvar retrocedido. | P0 | G | B54, B59 |
| **Diagnóstico guiado (estende B59)** | `ErrorKind` estável + causa-raiz + painel Diagnóstico (Realçar, Editar, Substituir, Isolar, Suprimir, Apagar); política Parar/Continuar/Perguntar; "Continuar exame". | P0 | G | Desempenho da árvore |
| **Pais e filhos** | Índice invertido de dependências; janela "Pais e filhos"; setas dinâmicas; "Isolar" para referências de geometria; lista de dependentes ao suprimir. | P1 | M | Diagnóstico guiado |
| **Desfazer por comando** | Passos nomeados, histórico, edição contínua = 1 passo, sessão cancelada sem passo, limite em MB. | P0 | M | Objeto em trabalho único |
| **Atualização: local, cancelar, verificar** | Atualizar esta feature; cancelamento cooperativo entre features; modo manual por documento; "Verificar" cache × reconstrução. | P1 | M | Desempenho da árvore |
| **Seleção unificada e filtros** | Conjunto único de seleção; filtros por tipo na barra com indicador; filtro temporário nos comandos; "Selecionar outro…"; Alt+roda. | P1 | G | — |
| **Seleção inteligente (laço, tangência, cadeia)** | Menu 3D com Laço, Tangência e Cadeia; regra de propagação gravada no comando (casa com B21); caixa pela direção do arrasto. | P1 | M | Seleção unificada, B21 |
| **Reordenar com validação e regra ordenado × livre** | Indicador de destino, validação de pais/filhos, mensagens, "Ordenar por dependência". | P1 | M | Pais e filhos |
| **Ocultar hierárquico e busca na árvore** | Herança de visibilidade, contador, filtro/busca com `:erro` etc. | P2 | M | Desempenho da árvore |
| **Atalhos completos e busca de comandos** | Id estável por `Act`, tudo no `Keymap`, dicas leem o atalho atual, Ctrl+K com troca de bancada, migração de chaves de personalização. | P1 | M | — |
| **Dicas e ícones (estende B60)** | Dica em 2 estágios com figura, atraso configurável, teste de ícones faltantes, descrição na barra de status. | P1 | M | Atalhos completos |
| **Câmera: pan, zoom ao cursor, reencaixe** | Pan proporcional, zoom ao cursor, "Ajustar à seleção", vista anterior, rotação em torno da seleção. | P1 | P | — |
| **Qualidade gráfica (estende B61)** | `MeshQuality` 4 níveis, padrão + sobreposição por documento, malha de exibição separada da de medida, LOD e *pixel culling* em montagem, MSAA configurável. | P0 | G | Objeto em trabalho único (não bloqueia), B53 |
| **Configurações: aplicação × documento** | Classifica cada opção; opções de documento vão para o arquivo; backup de `config.json`; exportar/importar perfil. | P1 | M | — |
| **Mensagens e barra de status** | Registro de mensagens com severidade e ação; "próximo passo"; `tr()` em todos os toasts. | P2 | P | — |
| **Painéis de comando (estende B20)** | Ordem Suporte → Modo → Parâmetros → Avançado; campo ativo; lista expansível; "o que falta" na barra de status. | P1 | G | Seleção unificada |
| **Barra de contexto e menu radial** | Mini barra após selecionar; marking menu/gestos configuráveis. | P3 | M | Atalhos completos |
| **Ponto de parada e supressão por expressão** | Stop Update com nota; supressão por parâmetro. | P3 | P | Objeto em trabalho único |

---

## 5. Fontes

**CAD avançado B**
- (fonte em procedencia.md) (Scan e objeto de trabalho)
- (fonte em procedencia.md) (Update, diagnóstico, Stop Update)
- (fonte em procedencia.md) (User Selection Filter)
- (fonte em procedencia.md) (editar peça no contexto de montagem)
- (fonte em procedencia.md) (Part Body, Geoset, Ordered Geoset, reordenação, Auto Sort)
- (fonte em procedencia.md) (3D accuracy, anti-aliasing, pixel culling, LOD, oclusão, cache)
- https://www.ibm.com/support/pages/node/5498133 e https://www.ibm.com/support/pages/apar/HD54039 (Update Diagnosis no modo manual)
- https://www.ibm.com/support/pages/node/5443407 (Parents/Children)
- https://www.ibm.com/support/pages/apar/HD61844 (pilha de desfazer, R16)
- https://www.ibm.com/support/pages/apar/HD21508 e https://www.ibm.com/support/pages/node/5470425 (Keep link with selected object)
- https://www.ibm.com/support/pages/apar/HD60765 (parâmetros na árvore, Part Infrastructure > Display)
- (fonte em procedencia.md) e (fonte em procedencia.md) (pré-seleção, Selection Trap; lidos apenas pelos resumos da busca)
- https://community.3dcs.com/help_manual/mouse_2.htm e https://community.3dcs.com/help_manual/graphicmanipulation_2.htm (mouse, Fit All In, Zoom Area, Reframe On; resumos da busca)
- https://technia.nl/?p=4448 (bússola; resumo da busca)
- https://community.3dcs.com/help_manual/hideshowpartsproducts_2.htm e (fonte em procedencia.md) (Hide/Show e swap visible space; resumos da busca)
- https://eng-tips.com/threads/layering-in-v5.73209 (camadas e filtros de visualização; resumo da busca)

**CAD avançado A**
- (fonte em procedencia.md) (resumo da busca; a página não renderizou)
- (fonte em procedencia.md)
- https://hawkridgesys.com/blog/rebuild-vs-forced-rebuild
- https://www.cati.com/?p=121157 (Dynamic Reference Visualization, caixas expansíveis do PropertyManager)
- https://www.cati.com/?p=131637 (atalhos, tecla S, gestos, barra de contexto)
- (fonte em procedencia.md)
- https://knowledge.cadimensions.com/knowledge/selection-methods-chain-vs.-loop-vs.-tangency
- https://www.javelin-tech.com/blog/2011/09/box-select/ (resumo da busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) (What's Wrong, opções de erro; resumos da busca)
- (fonte em procedencia.md) (Image Quality; resumo da busca)
- (fonte em procedencia.md) (opções de sistema × propriedades do documento; resumo da busca)

**CAD avançado C / Fusion**
- (fonte em procedencia.md) (Browser, EOP)
- (fonte em procedencia.md) (Design Doctor)
- (fonte em procedencia.md) (erros e mensagens)
- (fonte em procedencia.md) (Marking Menus)
- (fonte em procedencia.md) (Select Command Reference)
- (fonte em procedencia.md) (tooltips e ToolClips; resumo da busca)
- (fonte em procedencia.md) (Application × Document Settings; resumo da busca)
- (fonte em procedencia.md) (Update × Rebuild All; resumo da busca)
- (fonte em procedencia.md) (mini barra e prioridade; resumo da busca)
- https://cadtips.cadalyst.com/menus/part-priority-quick-access (Shift+botão direito; resumo da busca)
- (fonte em procedencia.md) (CAD avançado D, prioridade de seleção)
- (fonte em procedencia.md) e (fonte em procedencia.md) (timeline do Fusion; fontes de terceiros, resumos da busca)

**Outros**
- (fonte em procedencia.md) (CAD avançado E Feature List)
- (fonte em procedencia.md) (barra de retrocesso e liberação; resumo da busca)
- https://soliddna.com/SEHelp/ST6/EN/create_part/pthfndb2a.htm (CAD avançado H PathFinder)
- (fonte em procedencia.md) e (fonte em procedencia.md) (CAD avançado F Insert Here e falhas; resumos da busca)
- (fonte em procedencia.md) (CAD avançado G; resumo da busca)

**Código do Fenkra lido**
`crates/fenkra-app/src/app.rs`, `toolbar.rs`, `icons.rs`, `settings.rs`, `render.rs`, `keymap.rs`, `picking.rs`, `viewcube.rs`. Também `fenkra-solidos/src/lib.rs` (linhas de tesselação), `fenkra-kernel/cpp/kernel.cpp:829-833` e `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`.
Não li `i18n.rs` por inteiro (2 605 linhas). Só confirmei que os textos passam por `tr()` e que há literais soltos em `toast(...)`.

---

## 6. Entrega (handoff)

- **Resultado:** relatório completo, com 30 fichas, 36 requisitos funcionais e 9 não funcionais com critério de aceite, e 19 itens de backlog propostos.
- **Evidência:** referências `arquivo:linha` do código lido e as URLs da seção 5. Cada afirmação sobre os CADs está marcada [V] ou [C].
- **Arquivos alterados:** nenhum.
- **Verificações executadas:** só leitura de código e pesquisa web. Não compilei nem rodei o Fenkra, então todos os "bugs prováveis" são hipóteses por leitura de código e precisam de teste.
- **Falhas e limites:**
  - `help.CAD avançado A.com` não renderiza para o leitor automático, e vários blogs (Javelin, TriMech, SolidSolutions) responderam 403. As afirmações sobre CAD avançado A vêm de resumos de busca e de blogs de revendas (CATI, Hawk Ridge, GoEngineer, cadimensions).
  - Não achei documentação oficial pública do CAD avançado B. As fontes são apostilas de universidade, revendas e registros da IBM, quase todos de releases antigas (R16–R18).
  - O PDF NIAR "Basic Concepts R30" respondeu 404.
  - As páginas GrabCAD de seleção do CAD avançado B responderam 403 (só os resumos da busca).
  - Não encontrei fonte para "Selection Traces" do CAD avançado B, para o ajuste de limite da pilha de desfazer do CAD avançado B atual, nem para a pilha de desfazer do CAD avançado C. Esses pontos estão marcados [C] ou omitidos.
- **Riscos:**
  - Os números de prioridade e tamanho são estimativas minhas.
  - O prefixo `RF-INT` pode precisar de ajuste para evitar colisão com requisitos existentes.
  - Não li o texto completo de B54, B59, B60, B61 e B20 (linhas longas omitidas na leitura do backlog). As "extensões" propostas assumem o que o ADR 0011 diz sobre eles.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra neste tema são: (1) não há um "objeto em trabalho" único (corpo ativo, marcador e sessão são três mecanismos separados); (2) o diagnóstico de falha é só texto, sem causa-raiz, realce da referência perdida nem "Isolar"; (3) não há grafo de pais e filhos nem seleção inteligente (filtros por tipo, Select Other, laço, cadeia, tangência), e a seleção é dividida entre árvore e viewport; (4) a árvore clona o documento e serializa features em todo quadro, e o desfazer clona o documento inteiro, sem nome de ação nem histórico; (5) a tesselação é fixa, usada também no pick, e as configurações de documento estão misturadas às da aplicação.
