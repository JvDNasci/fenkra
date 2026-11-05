# Montagem (Assembly Design / Assemblies)

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> **Método e legenda.** [V] = confirmado em fonte lida ou citada pela busca (a URL está na seção 5). [C] = conhecimento geral, não confirmado nesta pesquisa.
> - **CAD avançado B:** a base principal é o manual oficial *Assembly Design V5R14* (PDF do CAD avançado Bdesign.org), que li quase inteiro, mais as apostilas da City University London e do NIAR/WSU.
> - **CAD avançado C e CAD avançado E:** li as páginas de ajuda diretamente.
> - **CAD avançado A:** o help.CAD avançado A.com é renderizado por JavaScript e o WebFetch devolve só o menu do site. Por isso o que marco [V] do CAD avançado A vem dos trechos que a busca retornou das próprias páginas de ajuda e de blogs de revendas (CATI, GoEngineer, Hawk Ridge, Javelin, TriMech). Onde só tenho memória, marco [C].
> - **CAD avançado F, CAD avançado G, CAD avançado H e Fusion:** só trechos de busca. Tudo o que digo sobre eles é complementar.
> - **Código do Fenkra:** li `crates/fenkra-montagem/src/*.rs` e as partes de `crates/fenkra-app/src/asm_ui.rs` que tratam de montagem.

---

## 1. Resumo e principais lacunas

**Resumo.**

1. A montagem do Fenkra é um solver global de 6 graus de liberdade por corpo (Levenberg–Marquardt com jacobiano numérico) sobre uma lista plana de nós. Isso dá uma base funcional: restrições, GDL por componente, juntas, acoplamentos, padrões, explosão, BOM, interferência e arrastar respeitando restrições.
2. A filosofia CAD avançado B é outra. A montagem é uma **árvore de especificação** com estado por restrição (verificada, impossível, a atualizar, quebrada), **update** manual ou automático com diagnóstico, **publicações** como contrato entre peças, **objeto de trabalho** ativo e **links contextuais** com ciclo de vida. O Fenkra tem o primeiro bloco (resolver) e quase nada do segundo (especificar e diagnosticar).
3. Hoje uma restrição nunca fica "quebrada". Se a face some, `refresh_refs` mantém a geometria antiga em silêncio (`load.rs:93-95`). Se o ponto clicado passa a cair em outra face, a restrição pula para ela sem aviso.
4. Substituir componente só troca o `kind` (`lib.rs:651`). Não há opção "todas as instâncias", relatório do que não casou, nem religação por nome publicado.
5. Edição em contexto troca de bancada (`asm_ui.rs:890`). As outras peças aparecem só como arestas esmaecidas, sem seleção, e "Voltar" salva sempre. O B54 já cobre isso.
6. Subconjunto é um corpo rígido opaco. Não dá para restringir uma peça interna (`asm_ui.rs:340`). Não existe subconjunto flexível, e as features do subconjunto não aparecem no pai (`view.rs:97-101`).
7. Features de montagem (furo e corte) não têm dono. O furo guarda ponto e direção no **mundo** (`lib.rs:278`) e vira um prisma de 64 lados (`view.rs:209`), então não é cilíndrico de verdade. Não grava nada nas peças, e a face criada pelo furo não é selecionável de forma confiável.
8. O catálogo marca "limites" nas juntas (MON-04), mas `kin.rs` não tem nenhum limite. "Padrão associado ao padrão de furos" (MON-08) também não existe: só há Linear e Circular.
9. Interferência ignora peças dentro de subconjuntos e não tem folga mínima (clearance), contato versus interferência, lista de ignorados nem filtros (`view.rs:271-292`).
10. A restrição Distância não tem orientação (mesmo sentido/oposto) e vale `0` por padrão (`asm_ui.rs:74`). Isso gera dois bugs prováveis: a peça vira 180° sem aviso, e o jacobiano fica singular em `norm(...) - 0`.

**Principais lacunas, em ordem de impacto.**

- (a) Estado e ciclo de vida das restrições e dos links (quebrada / a atualizar / travada / suprimida), com diagnóstico de conflito.
- (b) Contexto e publicações: o objeto de trabalho ativo sem sair da montagem, referências por nome que sobrevivem à substituição, e esqueleto/layout.
- (c) Estrutura de produto: subconjunto flexível/rígido, restrição em peça interna, "todas as instâncias", instâncias com propriedades distintas.
- (d) Features de montagem com dono (furo em série, corte, divisão) gravadas nas peças com link e com Isolar.
- (e) Restrições avançadas (largura, simetria, limites, tangente, travar rotação, caminho) e convenções de orientação.

---

## 2. Fichas por comando/função

### 2.1 Estrutura de produto: Produto, Peça, Componente, Instância (CAD avançado B: Product/CATProduct, Reference × Instance / CAD avançado A: Assembly, Component / CAD avançado C: .iam, Occurrence / outros: CAD avançado E Instances / Fenkra: `Assembly`, `Node`)

- **O que faz:** define a árvore de componentes. Cada nó é uma instância de um arquivo-referência (peça ou subproduto) com posição própria. Um arquivo pode ter várias instâncias.
- **CAD avançado B — como funciona no detalhe:**
  - Produto e componente são os blocos; um produto contém peças e outros produtos. [V]
  - Cada nó tem **Part Number** (da referência) e **Instance name** (do nó). Na árvore aparece como `Instância [Documento]` no modo visualização e `Produto (Instância)` no modo design. [V]
  - O produto-raiz tem "origem do produto". Uma peça nova pergunta se usa o ponto de origem da montagem ou um ponto escolhido. [V]
  - Tools > Product Management lista part number, arquivo, estado e representação associada, e permite trocar o part number e a representação. [V]
  - Só o **componente ativo** (moldura azul, sublinhado) recebe novas restrições, que valem entre filhos dele. [V]
- **CAD avançado A:**
  - A montagem é uma lista hierárquica no FeatureManager; a pasta *Mates* guarda as restrições. Os mates também aparecem duplicados dentro de cada componente. [V]
  - O primeiro componente entra fixo; os demais "flutuam" (a "(f)" e o "(-)" da árvore são do conhecimento geral [C]).
- **CAD avançado C:** cada ocorrência tem seu navegador; a primeira vai "grounded". A árvore mostra as relações (restrições/juntas) por componente. Subconjuntos podem ser *adaptive* ou *flexible*. [V]
- **Outros:** CAD avançado E trata Part Studios e Assemblies como abas do mesmo documento; as instâncias ficam no assembly. CAD avançado F tem *packaged components* (incluídos sem estar totalmente restritos). [V]
- **Fenkra hoje:**
  - `Node` (`lib.rs:132`) guarda id, nome, número, `kind` (Group/Part/SubAssembly), `parent` (só organização), posição, `fixed`, `visible`, `mode` e cor.
  - Não há distinção entre número de peça e nome de instância; o `name` é livre e único (`unique`, `lib.rs:512`). `unique` não considera `features`, e `name_of` (`lib.rs:497`) também ignora `features`, então o nome de uma feature cai em `#id`.
  - A primeira peça entra fixa (`lib.rs:539`).
  - Não existe "componente ativo". As restrições são globais.
  - Grupo (`Group`) é só pasta, sem geometria nem posição própria. O conceito CAD avançado B de produto-dentro-de-produto com posição só existe via `SubAssembly` (arquivo separado).
- **Melhor prática para o Fenkra:**
  - Separar `Referência` (arquivo) de `Instância` (nó). A instância deve poder sobrepor propriedades (nome, cor, modo de BOM, configuração).
  - Introduzir o conceito de **produto ativo**: restrições só entre filhos do ativo. Isso evita restringir peças de níveis distintos sem querer.
  - Testes: renomear duas instâncias para o mesmo nome (deve gerar sufixo); apagar o pai (filhos sobem de nível, já testado em `json_e_numeracao`); abrir um arquivo com `format_version` maior (deve avisar, não quebrar).

### 2.2 Inserir componente existente, com posicionamento (CAD avançado B: Existing Component / Existing Component with Positioning / CAD avançado A: Insert Components / CAD avançado C: Place Component / Fenkra: AsmInsert)

- **O que faz:** traz um arquivo de peça ou subconjunto para a montagem e o posiciona.
- **CAD avançado B:**
  - *Existing Component* insere na posição do arquivo. [V]
  - *Existing Component with Positioning* abre o **Smart Move** no ato da inserção. Você escolhe geometria do componente novo e do existente para "encaixar", e opcionalmente cria a restrição pela lista de prioridade da Quick Constraint. [V]
  - *Define Multi-Instantiation* (Ctrl+E) e *Fast Multi-Instantiation* (Ctrl+D) repetem o componente selecionado. [V]
- **CAD avançado A:**
  - A inserção permite arrastar para a janela, e o primeiro componente é fixado na origem. [C]
  - Com Alt+arrastar sobre a peça já colocada ocorre o SmartMate (ver 2.9). [V]
  - Parafusos/porcas recebem mates automáticos por reconhecimento (2025 SP3). [V]
- **CAD avançado C:** o diálogo *Place Component* oferece "colocar interativamente com iMates" ou "gerar iMates automaticamente ao colocar". [V] Os iMates casam pelo nome e, se não houver nome igual, pelas propriedades. [V]
- **Outros:** CAD avançado F adiciona restrições automaticamente a cada par de referências válido até o status ficar "totalmente restrito". [V] CAD avançado H usa FlashFit, que escolhe a relação pelo tipo dos elementos clicados. [V]
- **Fenkra hoje:**
  - `AsmInsert` (`asm_ui.rs:424-449`) abre um seletor de arquivos e coloca o componente "um pouco à direita" da última peça (x máximo da malha + 20). Se for a primeira, vai à origem.
  - Em modo relacional, nasce fixo na origem.
  - O diálogo de inserção não posiciona nem restringe; isso exige passos manuais depois.
  - O "x máximo" vem de `r.bodies...positions` e ignora a ocultação da peça. Peças ocultas ou muito longe distorcem o deslocamento inicial.
  - Não há "colocar vários", nem preview, nem inserir em grupo (só `parent` do grupo selecionado).
- **Melhor prática para o Fenkra:**
  1. Escolher arquivo(s).
  2. A peça nova "gruda" no cursor com preview e arredonda sobre faces/arestas (SmartMate de inserção).
  3. Escolher modo (fixo, flutuante, leve).
  4. Opções avançadas: criar restrição pela lista de prioridade; multi-instância (nº, espaçamento, direção).
  - Testes: inserir a mesma peça 3 vezes (nomes `P`, `P.2`, `P.3`, contagem ×3 na árvore); inserir peça cujo arquivo falta (erro visível no nó, não pânico); inserir subconjunto cíclico (profundidade > 16, já protegido em `view.rs:88`).

### 2.3 Novo componente / criar peça na montagem (CAD avançado B: New Part/Product / CAD avançado A: New Part (in-place) / CAD avançado C: Create Component / Fenkra: AsmNewPart)

- **O que faz:** cria um arquivo de peça já inserido na montagem e abre para modelar.
- **CAD avançado B:**
  - O comando *Part* da barra Product Structure Tools pede se a nova peça usa a origem da montagem. [V]
  - Com "Manual Input" ativo, pergunta primeiro o part number. [V]
  - Depois você dá duplo clique na peça para entrar no Part Design e modela sobre as faces dos vizinhos. [V]
  - Com a opção *Keep link with selected object*, nasce um **link contextual**; a árvore mostra uma rodinha verde na peça. [V]
- **CAD avançado A:** *Insert Component > New Part*: você escolhe um plano ou face e a peça abre em contexto, com mates automáticos ("In-Place"). [C]
- **CAD avançado C:** *Create Component* pede nome, template, pasta e restrição inicial (padrão: plano da peça no plano escolhido). [C]
- **Fenkra hoje:**
  - `AsmNewPart` (`asm_ui.rs:450-471`) pede o caminho `.fkp`, grava um documento vazio, insere o nó na origem e já chama `edit_part`.
  - O nó nasce solto. Em modo relacional fica fixo. Não há plano de apoio escolhido nem restrição inicial.
  - Se a gravação falhar depois de `push_undo`, o undo já foi consumido (checar a ordem em `asm_ui.rs:462`).
- **Melhor prática:**
  - Pedir nome e pasta; opção "orientar sobre uma face/plano" (cria restrição inicial coincidente); abrir com as outras peças visíveis como contexto. Registrar o link contextual com estado.
  - Testes: cancelar o diálogo de arquivo não cria nó; falha de disco não deixa nó fantasma; criar sobre face e depois mover a face (a peça acompanha).

### 2.4 Subconjuntos: criar a partir da seleção, rígido × flexível (CAD avançado B: Flexible/Rigid Sub-Assembly / CAD avançado A: Form New Subassembly, Solve as Flexible / CAD avançado C: Create Subassembly, flexible / CAD avançado E: subassembly & rigid / Fenkra: AsmSubFromSel, SubAssembly)

- **O que faz:** agrupa componentes em um arquivo de montagem reaproveitável. Define se, no pai, o subconjunto se move como um corpo só (rígido) ou se as peças dele podem se mover (flexível).
- **CAD avançado B:**
  - Subprodutos **rígidos** ficam sempre sincronizados com o produto original. Os **flexíveis** podem ter suas peças movidas no pai, e a partir da R7 você edita valores, orientação e driving/driven das restrições do flexível sem mudar o original. [V]
  - Alternar para flexível apaga os "sets" de restrições do rígido. [V]
  - Comando contextual *Flexible/Rigid Sub-Assembly* (ou ícone). A rodinha do nó fica roxa para flexível; a propriedade é copiada ao colar. [V]
  - Analyze > Mechanical Structure mostra a estrutura mecânica, que **difere** da estrutura de produto. [V]
  - Flexíveis não entram na análise de GDL e não se movem com *Manipulate*. [V]
- **CAD avançado A:** *Solve as: Flexible* no *Component Properties*. Os mates do flexível são resolvidos **junto** com os do pai. [V] Um teste citado reduziu os mates avaliados no topo de 1292 para 139 ao voltar a rígido, ou seja, flexível custa rebuild. [V]
- **CAD avançado C:** subconjunto pode ser tornado adaptativo ou flexível. [V] LOD substituto é outro assunto (2.29).
- **Outros:** CAD avançado E mostra ícone "Rigid" em subconjuntos rígidos. [V]
- **Fenkra hoje:**
  - `extract_subassembly` (`lib.rs:415-469`) move componentes para um `.fka` novo e leva mates/juntas/rígidos internos.
  - Restrições com componentes de fora são **apagadas** (conta em `dropped`).
  - `features.retain` (`lib.rs:466`) **apaga** as features que tocam os componentes extraídos, sem transferi-las ao subconjunto.
  - `couplings` internos aos `joints` copiados **não** são copiados (perda de dados). `patterns`, `params`, `display_states`, `pmi` e `items` também não vão.
  - O subconjunto é sempre rígido. `mate_ref` recusa peças dentro dele (`asm_ui.rs:340-342`). `comp_mesh` lê o JSON do disco a cada composição e **ignora** as features do subconjunto (`view.rs:97-101`).
- **Melhor prática:**
  - Ao extrair: preservar tudo que for interno e **religar** restrições de fora para o novo nó, em vez de apagar. Mostrar o relatório do que foi religado.
  - Adicionar o modo flexível: o solver inclui as peças do subconjunto no sistema do pai (6 GDL por corpo interno) e guarda sobrescritas de valores de restrição por instância.
  - Testes de regressão:
    - `extract_subassembly` com uma feature entre duas peças extraídas deve acabar no subconjunto;
    - com um acoplamento entre duas juntas extraídas, o acoplamento deve ir junto;
    - com um padrão cuja origem é extraída, o padrão não pode sumir sem aviso.

### 2.5 Substituir componente e reconectar restrições (CAD avançado B: Replace Component / Manage Representations / Reconnect / CAD avançado A: Replace Components, Replace Mate Entities / CAD avançado C: Replace Component, iMates / Fenkra: AsmReplace)

- **O que faz:** troca a peça de um nó por outra, tentando manter as restrições. Quando não dá, indica o que ficou solto e permite religar.
- **CAD avançado B:**
  - Dialog *Impacts on Replace* com opção de substituir todas as instâncias. [V]
  - A religação só é **garantida** se: (1) a estrutura do componente é a mesma; (2) os nomes das instâncias internas são iguais; (3) as restrições apontam para os **mesmos elementos publicados**. [V]
  - Restrições sobre geometria publicada de GSD que dependem de número/tipo de pais (Intersect, Project) podem não religar. A recomendação é publicar o **resultado** (o vértice), não a feature. [V]
  - Restrição quebrada fica com símbolo amarelo. Em *Constraint Definition > More* há *Disconnected* e *Reconnect…*; o *Reconnect Representation* abre duas janelas (antiga × nova) para casar os elementos. [V]
  - Comando contextual *Refresh Constraint* reavalia o status quando o componente faltante volta. [V]
- **CAD avançado A:**
  - *Replace Components* troca por outra peça ou configuração, com opção "todas as instâncias". [V]
  - *Re-attach mates* abre um pop-up *Missing Entities* com vistas de isolamento, desfazer, apagar, pular e inverter alinhamento. [V]
  - Em mate com erro, *Replace Mate Entities* reatribui a face uma vez para todas as instâncias do componente (a partir de 2016). [V]
  - Se o componente substituído semeia um padrão, os do padrão trocam junto. [V]
- **CAD avançado C:** com **iMates** nos dois componentes, o Replace mantém as relações. Sem iMates, as restrições só sobrevivem se a peça nova veio de uma base comum (Save Copy As). [V] Composite iMates religam várias restrições de uma vez. [V]
- **Outros:** CAD avançado E resolve mates simultaneamente e destaca os problemáticos em vermelho. [V]
- **Fenkra hoje:**
  - `AsmReplace` (`asm_ui.rs:479-491`) pede um arquivo e chama `replace` (`lib.rs:651`), que só muda `kind`. Depois `solve()` roda `refresh_refs`, que reencontra a face pelo `pick`.
  - O `pick` só existe para faces (`asm_ui.rs:370`). Arestas e vértices **não têm** religação e mantêm a geometria da peça antiga.
  - `refresh_refs` ignora falhas de `geom_at` e **mantém** a geometria velha (`load.rs:91-95`). Se o ponto cai em outra face, a restrição pula sem aviso.
  - Só uma peça, sem "todas as instâncias", sem relatório do que não casou. Troca de Peça por Subconjunto é aceita sem checagem de restrições.
- **Melhor prática:**
  - Fluxo: selecionar → arquivo novo → escolher escopo (esta instância, todas) → mostrar **tabela de religação**: restrição, elemento antigo, candidato novo (nome publicado, depois ponto clicado, depois proximidade) e status (religada, ambígua, quebrada).
  - Toda restrição ganha **estado**: `Verificada`, `AAtualizar`, `Quebrada`, `Impossível`, `Suprimida`. Referência não achada → `Quebrada`, nunca geometria silenciosa.
  - Testes:
    - substituir por peça sem a face (restrição vira Quebrada, a peça não se move);
    - substituir por peça onde o ponto clicado cai em outra face (deve avisar "ambígua");
    - substituir todas as instâncias de um arquivo usado 3 vezes;
    - desfazer devolve tudo.

### 2.6 Reordenar, agrupar e organizar (CAD avançado B: Reorder Components, constraint Sets / CAD avançado A: folders / CAD avançado C: folders/Move / Fenkra: reorder, AsmGroup)

- **O que faz:** organiza a árvore sem mudar a geometria.
- **CAD avançado B:**
  - Em *Reorder Constraints* você move qualquer restrição, mesmo suprimida ou quebrada, e agrupa em **Sets** (com Add set, Remove set, Group in new set, Move set after/inside). Nada disso muda a geometria. Não há sets em flexível. [V]
  - A ordem importa para a **leitura**, não para a resolução. [V, por consequência do manual]
- **CAD avançado A:** pastas no FeatureManager; ordem dos mates não importa ao resolver (mates resolvem juntos). [C]
- **CAD avançado E:** "CAD avançado E resolve mates simultaneamente, então a ordem não afeta"; reordenar é só organização. [V]
- **Fenkra hoje:**
  - `reorder` (`lib.rs:658`) move o nó no vetor. `AsmGroup` cria `Group` (pasta).
  - Não há agrupamento de restrições; as restrições aparecem numa lista separada.
  - Repetição: o `parent` das cópias é o do original.
- **Melhor prática:**
  - Permitir pastas para restrições e para componentes, arrastar-e-soltar, ordem persistente.
  - Regra: reordenar nunca dispara solve (teste: ordem muda, `placement` não muda um bit).

### 2.7 Fixar componente e fixação conjunta (CAD avançado B: Fix / Fix Together / CAD avançado A: Fix, Lock mate / CAD avançado C: Ground, Rigid joint / CAD avançado E: Fix, Group, Fastened / Fenkra: AsmFix, AsmRigid)

- **O que faz:** impede que um componente se mova, ou faz um conjunto de componentes se mover como corpo só.
- **CAD avançado B:**
  - *Fix* tem dois sabores. **Fix in space** (padrão): posição absoluta pela origem do produto; o componente movido volta no update. **Fix** relativo (desmarcar *Fix in space* em *More*): fixa em relação aos outros e são os outros que se movem. [V]
  - O símbolo é uma âncora verde; o cadeado aparece só em *Fix in space*. [V]
  - *Fix Together* aceita vários componentes do componente ativo, pede nome (ex.: FT1) e funciona em modo design e visualização. [V]
  - Mover um deles com Shift+bússola ou *With respect to constraints* move os outros; um aviso (*Move Warning*) oferece "Estender seleção" e "não mostrar mais". [V]
  - Pode-se restringir um componente de um conjunto fixado junto, e a restrição afeta o conjunto todo. [V]
  - Conjuntos desativados mostram parênteses vermelhos. [V]
- **CAD avançado A:** *Fix* é só estado "fixo". *Lock mate* fixa posição e orientação relativas de dois componentes, "como um subconjunto rígido". [V]
- **CAD avançado C:** *Ground* fixa na origem; junta **Rígida** remove todos os GDL e tem **Lock** ("elimina todo movimento, mas permite o componente mudar de posição quando os relacionados se movem") e **Protect** (avisa se novas relações violam os GDL). [V]
- **Outros:** Fusion tem *Rigid Group* (muitos componentes num só). [V] CAD avançado E tem mate *Group* (fixa relativamente sem mates). [V]
- **Fenkra hoje:**
  - `fixed` no nó (`lib.rs:146`); `AsmFix` alterna (`asm_ui.rs:503-513`). `RigidGroup` (`lib.rs:291`) com `add_rigid`.
  - `bodies()` (`solve.rs:135-175`): se algum membro do grupo é fixo, todos ficam fixos; caso contrário viram um corpo.
  - Só existe o equivalente de *Fix in space* (a posição vem do estado atual no mundo). Não há fixar relativo ao pai.
  - Não há nome editável do grupo na barra (nome é "Fixação conjunta"), nem aviso ao mover, nem "estender seleção".
  - Cópias de um padrão ficam `fixed: true` (`lib.rs:608`). O botão "Liberar" nelas não tem efeito, porque `update_patterns` reimpõe `fixed` a cada solve.
- **Melhor prática:**
  - Dois modos: **Fixar no espaço** e **Fixar em relação a** (escolhe o referencial). Ícone diferente na árvore.
  - Grupo rígido com nome, mover-junto por padrão, aviso ao mover. Se o grupo contém uma peça fixa e o usuário tenta restringir outra peça do grupo, avisar "já definida".
  - Testes: grupo com membro fixo; grupo com restrições em dois membros em conflito (deve marcar `failed`, não travar); desmanchar grupo devolve 6 GDL para cada peça.

### 2.8 Restrições posicionais básicas: coincidência, contato, distância/offset, ângulo, paralelismo, perpendicularidade (CAD avançado B: Coincidence, Contact, Offset, Angle / CAD avançado A: Coincident, Concentric, Distance, Angle, Parallel, Perpendicular, Tangent, Lock / CAD avançado C: Mate, Flush, Angle, Tangent, Insert / Fenkra: MateKind)

- **O que faz:** relaciona duas geometrias (ponto, eixo/linha, plano) de componentes diferentes e remove GDL.
- **CAD avançado B — no detalhe:**
  - **Coincidência** alinha elementos: pode dar concentricidade, coaxialidade ou coplanaridade conforme os elementos. Tolerância de 10⁻³ mm. Pares válidos: ponto, linha, plano, esfera (centro), cilindro (eixo), cone, sistema de eixos, e entre sistemas de eixos só com mesma direção/orientação. [V]
  - *Orientação*: Indefinida (o sistema escolhe), Mesma, Oposta. Trocar Mesma ↔ Oposta pode mover as peças de modo inesperado em sistema sub-restrito. [V]
  - **Contato** exige superfícies "dirigidas" (com lado interno e externo). A área comum pode ser plano, linha ou ponto. Esfera-esfera e cilindro-cilindro de mesmo raio equivalem a coincidência. [V]
  - **Offset** entre elementos planos exige orientação; o valor aparece ao lado da restrição. Pode-se marcar *Measure* (valor lido da geometria; "Reference" no editor): então a restrição só mede. [V]
  - **Ângulo** oferece Perpendicularidade, Paralelismo (com orientação), Ângulo e Ângulo planar (eixo comum). Com orientação definida aparece o combo *Sector* com 4 setores: ângulo direto, +180°, 180° − ângulo, 360° − ângulo. [V]
  - A restrição guarda **qual componente se move**: no sistema não iso-restrito "você não controla quem se move". [V]
  - *Change Constraint* troca o tipo mantendo o vínculo (quando os elementos de suporte permitem). [V]
  - *Deactivate/Activate* tira a restrição da resolução; parênteses vermelhos no ícone e linha branca na geometria. [V]
- **CAD avançado A:**
  - Padrão: Ângulo, Coincidente, Concêntrico, Distância, Bloqueio, Paralelo, Perpendicular, Tangente. [V]
  - A maioria tem **alinhamento** (alinhado/anti-alinhado), com o botão para alternar; o padrão é o mais próximo da posição atual. [V]
  - Concêntrico tem *Lock Rotation*. Tangente exige pelo menos uma face cilíndrica, cônica ou esférica. [V]
  - Há tabela de pares válidos por entidade ("Standard Mates by Entity"). [V]
  - Aplicar mates a planos/eixos padrão é a forma mais robusta, segundo o curso NWTC. [V]
  - Ao criar, o pop-up deixa trocar o tipo, digitar distância/ângulo, inverter o alinhamento, desfazer ou concluir. [V]
- **CAD avançado C:**
  - **Mate** posiciona faces normais entre si e coincidentes; **Flush** alinha com as normais no mesmo sentido. [V]
  - Para dois eixos: *Opposed*, *Aligned* e *Undirected*. [V]
  - **Ângulo**: *Directed* (regra da mão direita, sem limites), *Undirected* (ajuda quando a peça vira ao arrastar) e *Explicit Reference Vector* (terceira seleção que fixa o sentido; é o padrão). [V]
  - **Tangente**: *Inside*/*Outside* (padrão Outside). [V]
  - **Insert** = mate de face + mate de eixo (um parafuso no furo), com Opposed/Aligned. Um GDL de rotação permanece. [V]
  - Opções: offset com sinal (a primeira peça escolhida define o positivo), *Predict Offset and Orientation* (infere offset/orientação quando o campo está vazio), *Show Preview*, *Pick Part First*, *Use Offset as Resting Position*, **máximo e mínimo** (limites que podem ser ligados e desligados sem perder o valor). [V]
- **Outros:** CAD avançado F — *Allow Assumptions* (padrão ligado) permite que duas coincidências deixem o parafuso "totalmente restrito"; desligado, exige um terceiro. [V] CAD avançado G — *Touch/Align* com "Prefer Touch" apresenta Align quando Touch superdefiniria. [V] CAD avançado E não usa essas restrições: usa **mate connectors** (ver 2.10).
- **Fenkra hoje:**
  - `MateKind` (`lib.rs:182-209`): Coincident{flip}, Contact, Distance, Angle, Parallel, Perpendicular, Gear, RackPinion.
  - Os resíduos estão em `solve.rs:45-124`:
    - Coincident plano-plano: `na − s·nb` e distância do ponto. Orienta pelo sinal `flip`.
    - Contato plano-plano: `na + nb`.
    - Distância plano-plano: `cross(na,nb)=0` (aceita **paralelo ou antiparalelo**) e `dot(pb−pa, na) − d`. Não há orientação.
    - Ângulo entre eixos/planos: `dot(u,v) − cos(a)` (sem setor; `a` e `−a` satisfazem). Misto plano-eixo usa `|dot|`.
  - Pares sem caso (ex.: Coincident Point×Axis existe; Angle com Point; Tangente) caem em "orientação" ou são ignorados silenciosamente: o `(k,a,b)` de `solve.rs:102` devolve sem resíduo se for Point.
  - O padrão da Distância é `"0"` (`asm_ui.rs:74`). Com 0, `norm(perp(...)) − 0` e `norm(sub(pb,pa)) − 0` têm gradiente nulo/indefinido no ponto resolvido, e o `dof` pode ser superestimado.
  - **Não há** Tangente, Bloqueio, Travar rotação (concêntrico), Largura, Simetria, Caminho, Limites; nem *Measure/Reference*; nem setor/alinhamento além do `flip`.
  - Contato Axis×Axis e Axis×Plane existem (`solve.rs:77-84`), mas Contato Ponto×Ponto aceita sem checar lado.
  - `seed_flips` (`solve.rs:263-308`) gira 180° antes de resolver restrições invertidas: bom remendo, mas só para Coincident e Contact plano/eixo.
- **Melhor prática (fluxo):**
  1. Seleção: 2 elementos (dentro do produto ativo).
  2. Modo: a lista curta dos tipos válidos para o par (tabela de pares), com o mais provável pré-selecionado.
  3. Parâmetros: valor (com expressão), **orientação** (Indefinida/Mesma/Oposta ou Alinhado/Anti-alinhado) com preview, setor do ângulo (4).
  4. Avançado: *Medir* (referência), limites mín/máx, travar rotação, nome, quem se move (fixo preferencial).
  - Regras anti-bug:
    - nenhuma restrição com valor 0 usa `norm`; Distância 0 vira Coincidência/Contato;
    - nenhum par sem resíduo definido pode ser aceito (rejeitar na criação);
    - orientação é sempre explícita, nunca "qualquer lado".
  - Testes:
    - Distância 5 entre faces antiparalelas e paralelas, ambas permitidas? Não: uma só.
    - Ângulo 30° e 330° dão peças diferentes.
    - Coincidência de eixos com `flip` não gira a peça de volta.
    - Tangente cilindro-plano.
    - Resolver 1000× o mesmo sistema dá a mesma posição (determinismo).

### 2.9 Restrição rápida / inteligente (CAD avançado B: Quick Constraint, Smart Move / CAD avançado A: SmartMates, Mate PM context / CAD avançado C: Assemble, Predict Offset / Fenkra: quick_options, AsmMate(Auto))

- **O que faz:** deduz a restrição pelos elementos escolhidos, ou ao arrastar.
- **CAD avançado B:**
  - *Quick Constraint* cria **a primeira restrição possível** de uma lista ordenada e editável: padrão *Surface contact, Coincidence, Offset, Angle, Parallelism*. Clicando em dois eixos cria Coincidência (contato impossível); depois, em duas faces, cria Contato. [V]
  - *Smart Move* combina Manipular e Snap e, com *Automatic constraint creation*, cria a restrição ao soltar, com setas verdes para inverter o sentido. A janela *Viewer* deixa selecionar com precisão. [V]
  - *Snap* usa uma tabela: ponto→ponto, ponto→linha, ponto→plano, linha→linha (colineares), linha→plano, plano→plano (paralelos). O **primeiro elemento é sempre o que se move**. [V]
  - Modos de criação (padrão, **cadeia** — reusa a última face; **pilha** — reusa a primeira) tornam encadeamentos rápidos. [V]
- **CAD avançado A:**
  - **SmartMates**: Alt+arrastar uma entidade até a parceira; o ponteiro mostra o mate que será criado; Tab alterna alinhado/anti-alinhado (soltar Alt antes de Tab). [V]
  - Pré-seleção com Ctrl + *Mate* escolhe o tipo (dois cilindros → Concêntrico; quatro faces → Largura). [V]
  - **Mate references** (ver 2.11) ativam-se também com Alt+arrastar. [V]
- **CAD avançado C:** *Predict Offset and Orientation* e *Show Preview* (a peça sub-restrita vai à posição ao escolher o segundo elemento). [V] Com componente adaptativo não há preview. [V]
- **Outros:** CAD avançado H FlashFit escolhe a relação pela geometria (planos → Mate ou Planar Align; arestas → Axial Align; pontos → Connect) e não aceita aresta circular, por ambiguidade. [V]
- **Fenkra hoje:**
  - Duas seleções abrem uma barra com opções (`quick_options`, `asm_ui.rs:97-105`) e o menu de contexto lista as mesmas.
  - `auto_kind` (`asm_ui.rs:375-387`): plano×plano → Contato se normais opostas, Coincidente se mesmo sentido; qualquer outra combinação → Coincidente.
  - `AlignOnce` posiciona sem guardar (`asm_ui.rs:713-716`).
  - Não há lista de prioridade configurável, modos cadeia/pilha, nem arrastar-e-soltar (Alt+arrasta). Não deduz Concêntrico/Insert (eixo+face) como par de restrições.
  - `AlignOnce` apaga a restrição **depois** do `solve`, mas a posição final só vale se o solver convergiu. Se não convergiu a peça fica em meio-caminho sem aviso.
- **Melhor prática:**
  - Modo "Arrastar para encaixar" (Alt+arrastar ou botão): o cursor mostra o tipo previsto; Tab alterna; solta e cria 1–2 restrições (ex.: eixo coincidente + contato de face = "Inserir").
  - Lista de prioridade nas preferências.
  - Modos Cadeia/Pilha para criar várias restrições seguidas.
  - Testes: eixo+eixo → Coincidência; face+face de frente → Contato; eixo cilíndrico + face anular → par "Inserir"; `AlignOnce` com conflito → aviso e reversão da posição.

### 2.10 Mover e arrastar componentes (CAD avançado B: Translate/Rotate/Manipulate/Snap/Smart Move, Stop Manipulation on Clash / CAD avançado A: Move/Rotate Component, Collision Detection / CAD avançado C: Free Move/Drag / Fenkra: gizmo, respect, stop_collision)

- **O que faz:** posiciona o componente fora de restrições, ou testa a mobilidade.
- **CAD avançado B:**
  - *Translate*/*Rotate* por valores ou por elementos geométricos (distância e direção a partir de uma face/linha).
  - *Manipulate* com 3 linhas de ícones: translações em x/y/z e planos xy/yz/xz, rotações em torno de x/y/z, e uma quarta coluna para direção/eixo escolhido. A opção **With respect to constraints** impede movimentos que violem restrições. [V]
  - Com Shift + bússola, componentes restritos se movem respeitando as restrições. [V]
  - *Stop Manipulation on Clash*: para o movimento antes do choque. Quanto mais devagar você move, menor a folga final (porque a checagem é discreta). A precisão depende da tesselação (SAG 0,2 mm por padrão). [V]
- **CAD avançado A:** *Move Component* com opções *Standard Drag*, *Collision Detection*, *Physical Dynamics*. [C] Mates são respeitados ao arrastar um componente sub-restrito. [C]
- **CAD avançado C:** arrasto livre respeitando restrições e juntas; *Contact Solver* e *Contact Set* simulam colisão. [V parcial: CAD avançado D/fornecedor de CAD avançado citam contact sets]
- **Fenkra hoje:**
  - `gizmo` (`asm_ui.rs:2331`) com setas/quadrados/anéis. `solve_toward` (`solve.rs:317`) puxa o componente ao alvo com peso pequeno, e o que as restrições permitem acompanha o mouse (arraste "respeitando restrições").
  - `stop_collision` usa `collides` (`view.rs:228`): boolean `Common` do componente contra todos os outros visíveis e editáveis a cada passo (custo alto; só peças dentro do mesmo nível; ignora subconjuntos).
  - O peso `0.05` e o fator `20` na orientação estão fixos em `solve.rs:338`.
- **Melhor prática:**
  - Arraste = `solve_toward` com *warm start* da última solução (já é o caso) + clamp do passo máximo por quadro, para a colisão não "pular" paredes finas.
  - Para colisão rápida: bounding boxes + distância mínima (não boolean) no arraste, e boolean só ao soltar.
  - Testes: arrastar peça com uma restrição de eixo (gira e desliza só no eixo); arrastar além do limite com colisão ligada (para antes de interferir); soltar e desfazer restitui a posição original.

### 2.11 Referências de encaixe: mate references, iMates, conectores (CAD avançado B: Publications / CAD avançado A: Mate Reference / CAD avançado C: iMates (single/composite) / CAD avançado E: Mate connector / Fenkra: —)

- **O que faz:** a própria peça declara "como se encaixa". Ao inserir, o sistema casa as referências de duas peças e cria as restrições sozinho; ao substituir, religa.
- **CAD avançado B:** o mecanismo equivalente é a **publicação** nomeada (ver 2.22). A restrição "aponta" para o nome publicado e sobrevive à substituição. Não há um objeto "iMate" separado. [V]
- **CAD avançado A:** uma **mate reference** em uma peça guarda a geometria primária/secundária/terciária e o tipo de mate; as duas peças precisam ter referências com o **mesmo nome**, e na ordem das entidades. Referências simples não suportam mates avançados como Largura. [V] Esquecendo de criar antes, o Alt+arrastar ativa as referências. [V]
- **CAD avançado C:** **iMate** é a metade de um par de restrições carregada pelo componente. Tipos: **simples** e **composto** (grupo nomeado: precisa do mesmo nome e do mesmo número de membros). O casamento usa o **nome** primeiro e, não havendo nome igual, as propriedades. Ao colocar, a metade casa com uma já existente, o componente é posicionado e cria-se um *iMate result* (restrição consumida; um iMate consumido não é reutilizado). Pode-se **inferir** iMates de features (furo, extrusão com laço circular) ou de restrições existentes. Benefício: o Replace Component mantém as relações. [V]
- **CAD avançado E:** **mate connectors**: sistemas de coordenadas locais em face/aresta/ponto; os *implícitos* seguem a geometria e os *explícitos* podem ser criados na Part Studio ou no Assembly. Conectores de Part Studio só se editam na origem. [V]
- **Fenkra hoje:** não existe. O `pick` (`MateRef.pick`, `lib.rs:177`) é só um ponto clicado para reencontrar uma face; não há nome, ordem ou tipo.
- **Melhor prática:**
  - Entidade **Interface** (nome, tipo, geometria referenciada, offset) armazenada na **peça** (e na montagem), com interface composta (parafuso = eixo + face anelar). É o mesmo objeto da publicação (ver 2.22): uma publicação do tipo "conector".
  - Casamento por nome → tipo → ordem; inserção mostra a lista dos casamentos possíveis.
  - Testes: peça A e B com interface "FuroM6"; inserir B cria 2 restrições; substituir B por B′ (mesmas interfaces) mantém 0 quebradas; interface faltante em B′ lista "FuroM6" como não casada.

### 2.12 Restrições avançadas (largura, simetria, caminho, limite, perfil central, tangente, bloqueio de rotação) (CAD avançado B: —; Offset+parâmetros / CAD avançado A: Advanced Mates / CAD avançado C: Symmetry, Transitional, Motion / CAD avançado E: Width, Tangent / Fenkra: ausente)

- **O que faz:** cobre relações recorrentes que não se escrevem bem com coincidência/offset.
- **CAD avançado B:** não tem um comando "largura/simetria". Obtém-se com restrições Offset iguais, medidas e fórmulas (Knowledge). Há **Symmetry** como *feature de montagem* (ver 2.25), que espelha componentes, não restringe. [V]
- **CAD avançado A:** *Advanced Mates*: **Limite** (faixa de valores de distância/ângulo: valor inicial, mínimo e máximo), **Linear/Linear Coupler**, **Caminho** (ponto de um componente num caminho), **Perfil Central**, **Simétrico** (duas entidades iguais simétricas sobre um plano) e **Largura** (centra uma aba entre duas faces; opções Centrada, Livre, Dimensão e Porcentagem). [V]
- **CAD avançado C:** *Symmetry* (dois elementos equidistantes de um plano) e *Transitional* (came/ranhura), *Motion* (rotação/rotação-translação). Limites mín/máx em Mate/Flush/Angle. [V]
- **CAD avançado E:** mate **Width** (centra até duas abas entre dois conectores) e **Tangent**; limites em mates de GDL livres. [V]
- **Fenkra hoje:** MON-06 está marcado ❌ com B36 no backlog. No solver não há nenhum caso. `Joint` também não tem limites (confirmado: nenhum campo/consulta de limite em `kin.rs`), embora o catálogo diga "limites" em MON-04.
- **Melhor prática:**
  - Ordem de implementação por valor: (1) **limites** em Distância/Ângulo/juntas (desigualdade); (2) **Largura/Simetria**; (3) **Tangente**; (4) **Travar rotação**; (5) **Caminho**.
  - Resíduos de limite exigem tratamento de desigualdade no LM: usar `max(0, v−max) + max(0, min−v)` com piso suavizado, e guardar o valor "de repouso" (CAD avançado C: *resting position*).
  - Testes: Distância com mín 2 / máx 8 arrastando além (para em 8); Largura com aba mais estreita que a ranhura (centrada, folga igual); Simetria depois de mover o plano.

### 2.13 Editar restrição: status, medir, suprimir, mudar tipo, reconectar (CAD avançado B: Constraint Definition/Properties, Reconnect / CAD avançado A: Mate PM, Suppress, Replace Mate Entities / CAD avançado C: Edit constraint, Suppress / Fenkra: painel Restrição)

- **O que faz:** corrige, desativa ou converte restrições existentes.
- **CAD avançado B:**
  - Duplo clique abre a definição; *More>>* mostra um **semáforo** de status (verificada, impossível, a atualizar, quebrada) e os elementos de suporte, com *Reframe on* e *Center Graph*. [V]
  - Aba **Mechanical**: *Deactivated*, *To update*, *Unresolved*. Aba **Graphic**: cor, tipo e espessura da linha. [V]
  - Cores padrão por status: resolvida verde, não resolvida verde-escuro, superestressada (*overstrained*) roxa, geometria inválida vermelho-escuro. [V]
  - *Copy/Cut/Paste* de valor entre restrições do mesmo tipo (offset/ângulo). [V]
  - *Component Constraints*: seleciona todas as restrições de um componente. [V]
  - Filtro de restrições por status, produto e tipo (dimensões, ângulos, ângulos fixos). [V]
  - O manual avisa: ao projetar a peça no contexto, a restrição pode apontar para uma geometria do histórico que não é a atual do objeto em trabalho (restrição "parece errada"). [V]
- **CAD avançado A:** suprimir/editar na pasta Mates; **MateXpert** (ver 2.14) e *Replace Mate Entities* para mates pendurados. [V]
- **CAD avançado C:** suprimir restrição desliga o preview; nome do browser; *Use Offset as Resting Position*. [V]
- **CAD avançado E:** cores azul/branco (bom), cinza (suprimido), vermelho (problemático). [V]
- **Fenkra hoje:** painel `asm_ui.rs:1943-2006`: renomear, trocar tipo no combo (mantém o vínculo), valor com fórmulas, `invertida`, `suprimida`, "posição atual = giro zero". O chip de status só distingue suprimida/falhou/ok (`asm_ui.rs:1609-1613`) e vem do último `SolveReport`. Não há "a atualizar", "quebrada" ou "impossível". Não há reconectar/trocar um dos elementos. Não há "medir".
- **Melhor prática:**
  - Estados persistidos e computados: `Verificada | AAtualizar | Impossível | Quebrada | Suprimida | Medida`.
  - Operação **Reconectar**: escolher o novo elemento sem recriar a restrição (preserva nome, valor, ordem).
  - Modo **Medir**: a restrição vira cota de referência (não restringe).
  - Testes: trocar Distância→Ângulo com elementos incompatíveis (recusa com motivo); suprimir e reativar devolve a mesma posição; "medir" nunca altera `dof`.

### 2.14 Update, diagnóstico de conflito, restrições superdefinidas (CAD avançado B: Update All / Update Diagnosis / Analyze > Update / CAD avançado A: MateXpert, Force Mate / CAD avançado C: Over-constrained warnings / CAD avançado E: Solve, red mates / Fenkra: solve, SolveReport.failed)

- **O que faz:** recalcula a montagem e explica por que não fecha.
- **CAD avançado B:**
  - **Update** automático ou manual (opção Assembly Design > Update); a **propagação** do update é configurável; *Compute exact update status at open* pode ser manual. [V]
  - Em modo visualização, o status de update é **desconhecido** até o usuário clicar no ícone; o sistema carrega o mínimo necessário e avisa (*Update Warning*). [V]
  - Restrições que precisam de update ficam em preto (cor configurável) e podem ser atualizadas **uma a uma** (Update contextual; Ctrl+clique para várias). [V]
  - Em sistema inconsistente/superdefinido o update abre o **Update Diagnosis**: indica a restrição culpada (ex.: Offset.7) e as válidas mas envolvidas (Offset.6), com *Editar, Desativar, Isolar* ou *Apagar*. [V]
  - *Analyze > Constraints* conta: não restritos, verificadas, impossíveis, não atualizadas, quebradas, desativadas, em modo medida, fixadas juntas; e também para subconjuntos. **Não** mostra superdefinição (isso só no update). [V]
  - *Analyze > Update* lista o que precisa atualizar (restrições, filhos, representações) e permite atualizar parte sem atualizar tudo. [V]
  - *Analyze > Dependencies*: árvore (restrições, **associatividade** com linhas verdes para componentes contextuais, relações/fórmulas), expansão nó a nó, "set as new root". [V]
- **CAD avançado A:** **MateXpert**: *Diagnose* lista os mates com falha (em negrito os que estão em "Analisar problema" e "Não satisfeitos"); seleciona e mostra a distância/ângulo faltante; sugere editar, suprimir, alternar alinhado/anti-alinhado, *Replace Mate Entities*; identifica **grupos superdefinidos**; analisa uma pasta Mates por vez. [V] *Force Mate* na bolha vermelha força o mate quebrando outro. [V]
- **CAD avançado C:** o *Degrees of Freedom* ajuda; restrições conflitantes ficam em vermelho no browser. [V parcial]
- **CAD avançado E:** *Solve* explícito; mates em vermelho; ícone tríade em peça livre. [V]
- **Fenkra hoje:**
  - O solve roda a cada mudança (`AsmState::solve`, `asm_ui.rs:307`) e devolve `SolveReport { residual, iterations, dof, failed, converged }` (`solve.rs:10-20`).
  - `failed` marca a restrição cujo resíduo final passa de 1e-6 (`solve.rs:405-412`). **Não** diz quais restrições formam o conflito: o LM fecha em mínimos quadrados e "distribui" o erro entre as restrições. Assim a restrição marcada como `failed` pode ser inocente.
  - Não há detecção de redundância (`rank(J)` < linhas) separada de conflito, nem núcleo mínimo.
  - O limite de 200 iterações e `lambda` não escalam com o tamanho do sistema. O jacobiano é diferença central (h = 1e-7) com custo O(colunas × avaliação completa): `place` e `all_residuals` varrem **todos** os nós por coluna (`solve.rs:487-495`). O sistema denso (JᵀJ) é n×n e `solve_linear` é O(n³) por tentativa, até 12 por iteração.
  - Não há decomposição em componentes conexos nem reuso de resultado quando só um componente mudou.
- **Melhor prática:**
  - **Dois níveis:** (a) `Update` incremental (resolver só o aglomerado conectado que mudou); (b) `Diagnóstico`: quando não converge, achar o **conjunto mínimo em conflito** (remover uma restrição por vez em ordem de criação e repetir, ou análise do posto do jacobiano por blocos) e listar "culpada" + "envolvidas" como no CAD avançado B.
  - Marcar como `Redundante` quando o posto não cresce e o resíduo é nulo (CAD avançado A "superdefinida" sem erro).
  - Modo manual/automático e **propagação** (só este nível / todos).
  - Testes:
    - o teste existente `fixacao_conjunta_e_conflito` (duas distâncias em conflito) deve identificar as **duas** restrições e não só uma;
    - restrição redundante (mesma coincidência duas vezes) → `Redundante`, converge;
    - um sistema de 200 corpos resolve em tempo limitado (RNF).

### 2.15 Graus de liberdade e análise (CAD avançado B: Analyze > Degrees of Freedom / CAD avançado A: under defined (−), Mate status / CAD avançado C: DOF glyphs / Fenkra: dof_per_comp)

- **O que faz:** mostra o que ainda se move em cada componente.
- **CAD avançado B:** a análise vem **só de restrições de montagem** (não conta padrões nem projeto em contexto) e só quando todas estão verificadas. Mostra cada **rotação** (vetor + centro) e **translação** possível, destacadas em amarelo na geometria, com botões (*Rotation_2*, *Translation_2*); planos de translação podem ser dados por dois vetores não ortonormais. Selecionar um subcomponente analisa em relação ao ativo; flexíveis ficam de fora. [V]
- **CAD avançado A:** o "(−)" de sub-definido na árvore e o prefixo "(f)" de fixo [C]; arrastar mostra o movimento livre. [C]
- **CAD avançado C:** glifos de GDL por componente (ligue ao começar a restringir); *Insert* remove 5 GDL e deixa a rotação sobre o eixo (exemplo de comunidade). [V]
- **Fenkra hoje:** `dof_per_comp` (`solve.rs:426-455`) calcula o posto do jacobiano no **ponto atual** (`x = 0`), coluna de 6 por corpo, e a árvore mostra ■ / ◐k / ○ (`asm_ui.rs:1138-1146`). Não mostra **quais** movimentos sobram (eixos/direções), só a contagem. Linearização em `x=0` pode errar em configurações singulares (ex.: Distância 0 acima). Peças fixas não aparecem como "0 GDL" explícito.
- **Melhor prática:**
  - Extrair do núcleo do jacobiano os **modos livres** (SVD ou eliminação) e classificá-los em translações/rotações com eixo e centro; desenhar setas na viewport.
  - Mostrar "totalmente definido" só quando o resíduo é nulo.
  - Testes: eixo coincidente → 1 rotação + 1 translação (já testado: `dof == 2` no teste de eixo); pino em furo com contato → 1 rotação; cubo em 3 faces → 0; casos singulares (dois eixos paralelos coincidentes) sem falso "0".

### 2.16 Juntas cinemáticas (CAD avançado B: Kinematics / DMU Kinematics joints / CAD avançado A: mechanical mates, Hinge, Slot / CAD avançado C: Joints / CAD avançado E: Mates / Fusion: Joints / Fenkra: kin.rs)

- **O que faz:** define explicitamente o movimento permitido (giro, deslize, etc.), em vez de montar restrição a restrição.
- **CAD avançado B:** as juntas do DMU Kinematics não fazem parte do Assembly Design, e a conversão de restrições em juntas é feita pelo Kinematics (*Assembly Constraints Conversion*). [C]
- **CAD avançado A:** *Mechanical Mates*: came, ranhura, dobradiça, engrenagem, cremalheira, parafuso, junta universal. [V]
- **CAD avançado C:** o tipo da junta sai **automaticamente** da geometria escolhida (círculos → Rotacional; pontos num cilindro → Cilíndrica; esfera → Esférica; senão Rígida). Tipos: Rígida, Rotacional, Deslizante, Cilíndrica, Planar, Esférica. [V]
  - Abas *Connect* (1ª e 2ª origem; *Flip*; *Pick part first*), *Align* (alinhamento de direções; *Invert*), *Limits* (angular/linear, Start/Current/End, ligáveis sem perder valor, **não** para Rígida), *Gap* (folga), *Animate* (preview). [V]
  - Origem *Between Two Faces* (ponto virtual no meio) e *Offset Origin* (com manipulador). [V]
  - **Lock** e **Protect** por relação. [V]
- **CAD avançado E:** Fastened, Revolute, Slider, Cylindrical, Planar, Pin Slot, Ball, Parallel. Cada um com seus GDL no eixo Z do conector (ex.: Revolute = Rz; Slider = Tz; Cylindrical = Tz+Rz; Planar = Tx, Ty, Rz; Pin slot = Tx + Rz; Ball = Rx, Ry, Rz). Offsets e **limites** com ícone na lista; "Aplicar posição limite"; Animate usa os limites. [V]
- **Fusion:** *Joints* (movimento + posição de referência) × *As-built joints* (posiciona + movimento), **Rigid groups**, *Contact sets* quando junta não representa o movimento. [V]
- **Fenkra hoje:**
  - `JointKind` (`kin.rs:39-60`): Revolute, Prismatic, Cylindrical, Screw{pitch}, Spherical, Planar, Rigid, PointCurve, Cam.
  - `Joint` (`kin.rs:88`) tem `angle0`/`len0` medidos na criação e comandos `drive_angle`/`drive_len` como expressões com `t`. **Sem limites**, sem folga/offset, sem *Flip*.
  - `joints_from_mates` converte restrições em juntas (`kin.rs:185-219`).
  - Criação por `create_joint` (`asm_ui.rs:726-844`) usa `mate_ref` (então tem a mesma recusa de peças em subconjunto).
  - Mexer no eixo de uma junta existente não é possível sem apagar.
- **Melhor prática:**
  - Adicionar: limites, rótulo de "posição de repouso", offset de origem, `Between Two Faces`, travar/proteger.
  - Teste: Revolução com limites ±90 e comando de 120 deve parar em 90 e marcar "no limite", não "falhou".

### 2.17 Restrições mecânicas: engrenagem, cremalheira, polia, came, parafuso, cabo (CAD avançado B: Gear/Rack/Cable joints / CAD avançado A: Gear, Rack Pinion, Screw, Cam, Slot, Universal / CAD avançado C: Motion constraints / CAD avançado E: Relations / Fenkra: MateKind::Gear/RackPinion, Coupling)

- **O que faz:** relaciona o movimento de duas juntas por uma razão.
- **CAD avançado B:** *Gear*, *Rack*, *Cable* no DMU Kinematics. [C]
- **CAD avançado A:** *Gear*, *Rack Pinion*, *Screw*, *Cam*, *Slot*, *Hinge*, *Universal Joint*. [V]
- **CAD avançado C:** *Motion constraints*: *Rotation* e *Rotation-Translation*. [V parcial]
- **CAD avançado E:** **Relações** (gear, rack and pinion, screw, linear) ligam mates; elas "movem matematicamente e não detectam colisão", então engrenagens podem se cruzar sem aviso. [V]
- **Fenkra hoje:**
  - Duas formas coexistem. (1) `MateKind::Gear`/`RackPinion` em `Mate` (`lib.rs:195-208`), que usam direções de referência `refs` (4 vetores) para medir o giro. (2) `Coupling` entre juntas (`kin.rs:110-125`): Gear, RackPinion, Cable.
  - O giro é medido por `turn` com `atan2` (`solve.rs:207-210`) e embrulhado por `wrap180` (`solve.rs:194`): a relação só vale **dentro de ±180° de giro relativo**. Para mais de meia volta, a engrenagem "salta" de solução (o LM pode escolher a volta mais curta).
  - O acoplamento de junta usa `measure` com `wrap180` também (`kin.rs:127-136`, `153-156`).
  - `simulate` (`kin.rs:330`) refaz o solve a cada quadro, a partir do estado anterior? Não: `solve_with` parte do estado atual do `a` (que é mutado entre quadros), e o `keep` restaura no fim. Funciona, mas sem aviso de "salto de ramo".
  - A razão é uma expressão avaliada ao resolver. Se a expressão não avalia, vira `0.` silenciosamente (`unwrap_or(0.)`, `solve.rs:47`): razão 0 trava a segunda engrenagem sem erro.
- **Melhor prática:**
  - Acumular o ângulo (desenrolar a fase entre quadros) em vez de `wrap180`, para suportar várias voltas.
  - Erro explícito quando uma expressão não resolve (marcar a restrição como `Impossível` com a mensagem).
  - Teste: engrenagem 1:−2 com comando de 720° (G2 deve estar em −360°, e não em 0°); razão `"z1/0"` (erro, não trava).

### 2.18 Movimento, simulação e posições salvas (CAD avançado B: DMU Kinematics, Scenes / CAD avançado A: Motion Study, Mate Controller / CAD avançado C: Dynamic simulation, Animate / Fenkra: sim_window, simulate)

- **O que faz:** anima o mecanismo por comando ou lei no tempo.
- **CAD avançado A:** **Mate Controller** guarda posições de componentes em valores diferentes de mate (suporta Distância, Ângulo, Limites, Caminho), sem criar configurações; cada posição vira um quadro-chave; pode salvar `.avi`. Os mates ficam "dirigidos" pelo valor, e para arrastar é preciso destravar. [V]
- **CAD avançado E:** *Animate* usa um só GDL e os limites como início/fim; Fastened/Tangent/Ball não são recomendados como mates que dirigem. [V]
- **CAD avançado C:** a aba *Animate* de junta e as posições (*Positional representations*) com sobrescrita de restrição. [V]
- **Fenkra hoje:** `simulate` (`kin.rs:330-342`) varia `t`; `kinematics_of` calcula velocidade/aceleração por diferenças finitas; `Frame.ok` marca convergência. Janela `sim_window` (`asm_ui.rs:2461`). Sem "posições nomeadas" por valores de restrição (existe `DisplayState`, mas só para visibilidade/cor/explosão/câmera, `display.rs:22-34`).
- **Melhor prática:** *Posições* nomeadas (conjunto de valores de restrição/comando) reaproveitáveis em desenho e animação; sincronizar com representações posicionais (2.29).
- **Teste:** posição "aberta" e "fechada" de uma tampa, alternadas 100×, voltam ao mesmo `placement` (±1e-9).

### 2.19 Edição em contexto: Definir objeto de trabalho / Editar peça / Edit in Place (CAD avançado B: objeto de trabalho / CAD avançado A: Edit Component / CAD avançado C: Edit (in place), Return)

- **O que faz:** entra na peça sem sair da montagem, para modelar com as vizinhas visíveis e referenciáveis.
- **CAD avançado B:**
  - Duplo clique na peça abre o Part Design; ao editar, uma montagem em modo visualização muda sozinha para design (a peça só existe em modo design). [V]
  - *objeto de trabalho* é a geometria/feature ativa (sublinhada) onde o próximo comando será inserido; o manual diz que operações são sobre o objeto em trabalho. [V]
  - Ao voltar duplo-clicando na montagem a árvore não muda. Ao projetar geometria de um vizinho aparece a roda verde na peça (se *Keep link with selected object* estiver ligado). [V]
  - Componentes contextuais são **filhos** dos que serviram de apoio: ao apagar o apoio, decide-se apagar os filhos (*Delete all children*). [V]
  - *Paste Special As Result With Link* não pode formar ciclo (mensagem). [V]
  - Features de montagem são **sempre** em contexto, mesmo com a opção desligada. [V]
- **CAD avançado A:** edita-se a peça na janela da montagem; os outros componentes ficam visíveis e viram referência externa (ver 2.20). Evitar criar mates com geometria criada em contexto, para não gerar rebuilds lentos e geometria instável. [V]
- **CAD avançado C:** o componente ativo aparece sombreado, os demais em wireframe (e o inverso no modo wireframe). Dentro de um subconjunto ativo não se edita peça diretamente: ativa-se o subconjunto, depois a peça. *Return*, *Return to Parent*, *Return to Top*, *Finish Edit*. Mudanças valem para todas as ocorrências e para outras montagens. [V]
- **Outros:** CAD avançado E cria peças diretamente no Part Studio com sketch de referência. [C]
- **Fenkra hoje:**
  - `edit_part` (`asm_ui.rs:890-906`) abre o arquivo na bancada Sólidos e marca `editing_part`. `context_overlay` (`asm_ui.rs:2286-2309`) desenha **só as arestas** das outras peças (até 40.000 segmentos) no sistema da peça ativa, em azul claro.
  - Não há seleção, projeção ou referência a essas arestas dentro do sketch/feature da peça. Só existe `AsmLink` feito **antes**, no modo montagem, e apenas com **face plana** (`link_in_context`).
  - `back_to_assembly` (`asm_ui.rs:909-918`) chama `save_pub` **sempre** e depois `forget` + `solve`. Sem descartar, sem aviso quando a peça tem outras instâncias, sem "Voltar ao pai/topo".
  - Se o `.fkp` está aberto por dois nós (duas instâncias), a edição vale para ambas sem nenhum aviso.
- **Melhor prática (o que o B54/ADR 0011 já indica):**
  1. Duplo clique ativa a peça **sem trocar de bancada**: o resto fica esmaecido, selecionável.
  2. Só um ativo por vez; o ativo ganha cor/moldura e aparece no cabeçalho ("Montagem > Sub > Peça").
  3. Voltar / Voltar ao pai / Voltar ao topo; Salvar ou descartar ao sair; aviso de N instâncias.
  4. Referência a geometria de vizinhas só por **publicação** (padrão), com opção "qualquer geometria".
  - Testes: editar peça com 3 instâncias (aviso exibido); descartar edição restaura o arquivo; a edição de uma peça não permite referenciar outra peça que depende dela (ciclo).

### 2.20 Links contextuais, estados e controle (CAD avançado B: External references, Isolate, Edit > Links / CAD avançado A: External references ->, ->?, ->*, ->x, Lock/Break / CAD avançado C: Adaptive, Derive, Break link / Fenkra: AsmLink)

- **O que faz:** liga geometria de uma peça à de outra, com ciclo de vida visível.
- **CAD avançado B:**
  - Links geométricos aparecem como *External references* e os paramétricos como *External parameters*. [V]
  - *Keep link with selected object* mantém os links; *Activate/Deactivate link* liga/desliga um a um; *Edit > Links* para ver e alterar; *Isolate* quebra o vínculo e a feature vira independente. [V]
  - Sincronização no update (automática ou manual). [V]
  - O update dos links é onde ciclos surgem. Para evitá-los usa-se o método do esqueleto (2.23). [V parcial]
  - Constraints não aparecem em Edit > Links. [V]
- **CAD avançado A:** estados no FeatureManager: `->` (em contexto, resolvido), `->?` (fora de contexto: precisa abrir a montagem do caminho de update), `->*` (travada), `->x` (quebrada). **Travar** impede atualizar e o bloqueio é reversível; **Quebrar** é permanente. Estado "dangling" separado. [V]
- **CAD avançado C:** o link adaptativo: apenas uma ocorrência por montagem pode ser adaptativa; "uma peça só é adaptativa em uma montagem por vez". [V]
- **Fenkra hoje:**
  - `link_in_context` (`asm_ui.rs:2152-2198`): publica automaticamente a face na peça de origem ("PublicaçãoN"), grava o link na peça de destino com `LinkXf` calculado **uma vez** (`rel = nt.placement.inverse().then(&ns.placement)`, `asm_ui.rs:2189`).
  - Consequência: se uma restrição move a origem depois, o link não acompanha (posição congelada em constante). [C, derivado da leitura do código]
  - Detecta ciclo (`depends_on`, profundidade 8) e instâncias do mesmo arquivo. Sem estados, sem lista de links por peça, sem Travar/Quebrar/Isolar.
- **Melhor prática:**
  - Estado explícito: `Em contexto | Fora de contexto | Travado | Quebrado | Pendurado`, ícone na árvore e na peça.
  - Ações: Travar/Destravar, Quebrar (com confirmação), Atualizar, Isolar, Listar links.
  - O transformador do link é **derivado da posição atual** das duas peças no momento do update, não congelado.
  - Testes: mover a origem 10 mm → o destino acompanha após update; abrir a peça fora da montagem → `Fora de contexto`; apagar a face-origem → `Pendurado`; travar impede a atualização.

### 2.21 Origem e geometria de referência da montagem (CAD avançado B: produto sem planos próprios; usa peça esqueleto / CAD avançado A: Origin + planos da montagem / CAD avançado C: Origin, work features / Fenkra: ausente)

- **O que faz:** planos, eixos, pontos e sistemas de eixos que pertencem à montagem e servem de referência de restrição e de contexto.
- **CAD avançado B:** o produto não tem planos próprios; a convenção é um componente "geometria de referência" ou esqueleto. [C]
- **CAD avançado A:** planos, eixo e origem padrão da montagem; geometria de referência e *layout sketch*. [V parcial]
- **CAD avançado C:** origem, planos e eixos de trabalho na montagem e *work features*. [V]
- **Fenkra hoje:** MON-13 ❌ (confirmado: `Assembly` não tem nenhum campo de referência, só `nodes/mates/...`, `lib.rs:326-367`). A "origem da montagem" é o `[0,0,0]` do mundo.
- **Melhor prática:** estrutura `RefGeom` na montagem (Plano, Eixo, Ponto, Sistema) disponível como alvo de restrição e como publicação do nível; padrão: 3 planos e 3 eixos fixos. Teste: restringir uma peça ao plano da montagem; mover a montagem inteira não altera a restrição.

### 2.22 Publicações (CAD avançado B: Publication / CAD avançado A: Mate references, Mate connectors, named selections / CAD avançado C: iMates, Work features, Parameter export / Fenkra: publicação em peça)

- **O que faz:** dá nome estável a um elemento (ponto, linha, plano, sketch, corpo, face, aresta, vértice, parâmetro) para outros o usarem como interface.
- **CAD avançado B:**
  - Tools > Publication: publicar, editar o nome, **substituir a geometria** ligada a um nome (mostra as orientações velha/nova e pede confirmação), criar/exportar/importar lista (`.txt`), apagar. [V]
  - Publicáveis: pontos, linhas, curvas, planos, sketches, corpos, features GSD, free style, **parâmetros**, sub-elementos (face/aresta/vértice/eixo/extremidade; eixos via *Other Selection* na face cilíndrica) e features do Part Design (se a opção estiver ligada). [V]
  - Renomear: *Never* (padrão), *Always*, *Ask*; não se renomeiam eixos, arestas e faces; caractere `!` proibido. [V]
  - No Assembly Design há **Browse**, que lista só as publicações dos níveis inferiores ao ativo. [V]
  - Uma restrição apontando para publicação num componente ausente fica quebrada e **não se reavalia sozinha** ao inserir o componente: usa-se *Refresh Constraint*. [V]
  - Ao substituir, publicações com o mesmo nome religam sozinhas; as sem publicação religam à mão. É possível exigir que só publicados sirvam como referência externa. [V]
  - Um nome publicado vale no contexto onde foi criado; em outro "defined in work object" o sistema não o reconhece pelo nome. [V]
- **CAD avançado C:** parâmetros marcados *Export*, `Make Part` deriva só os selecionados. [V]
- **Fenkra hoje:** a **peça** publica referência/parâmetro/corpo (catálogo MON-17: "montagem não tem sketch, ponto, eixo, plano nem publicação"). `link_in_context` cria publicações automáticas de face plana (`Publicação1…`), sem o usuário nomear. `MateRef` não usa nome, apenas geometria e `pick`.
- **Melhor prática:**
  - Restrição aponta para **nome publicado** (e não geometria) sempre que existir; a geometria é resolvida a cada update.
  - Operação *Substituir geometria do nome* mantém os links.
  - Publicar parâmetro. Lista exportável. Filtro "só publicados" (padrão ligado).
  - Testes: publicar a face `Topo`, restringir, substituir a peça por outra com `Topo` → 0 quebradas; sem `Topo` → quebrada e listada; apagar a publicação em uso pede confirmação.

### 2.23 Peça esqueleto / layout / projeto top-down (CAD avançado B: Skeleton, Publications / CAD avançado A: Layout sketch, Make Assembly from Layout, Insert Part / CAD avançado C: Skeletal modeling, Make Part, Make Components / Fenkra: modo relacional)

- **O que faz:** uma peça (ou sketch) central carrega os critérios do projeto e as demais derivam dela.
- **CAD avançado B:** uma peça "esqueleto" contém os elementos que dirigem posições e dimensões principais; a informação vai **só para baixo**, via publicações e referências externas, o que evita laços de update. [V parcial (material de treinamento DS)]
- **CAD avançado A:** a montagem pode nascer de um *layout sketch* com posições e cotas-chave; as peças 3D seguem o sketch. Referências externas são geradas ao usar geometria de outros componentes. [V]
- **CAD avançado C:** dois fluxos:
  1. **Origem comum**: peça-esqueleto com sketches, superfícies de construção, work features e sólidos; parâmetros marcados *Export*; *Make Part* deriva os itens selecionados e fixa o componente na origem; sub-esqueletos para subconjuntos.
  2. **Independente de posição**: peça de layout com 2D e **blocos de sketch**; *Make Components* deriva os blocos e cria restrições *Layout*; os blocos controlam forma e as restrições, posição; bom para cinemática. [V]
- **Fenkra hoje:** `Assembly.relational` (`lib.rs:365`) faz peças novas nascerem fixas na origem; a forma vem de links entre peças. Não há peça esqueleto formal, sketch de layout, nem "Fazer peça a partir de".
- **Melhor prática (B55):** documentar o fluxo canônico: Esqueleto (peça com publicações) → componentes fixos na origem comum → links contextuais por publicação. Bloquear referência para cima (esqueleto não depende de filhos). Teste: editar o esqueleto e atualizar 30 componentes; criar ciclo (filho→esqueleto) deve ser recusado.

### 2.24 Features de montagem: furo, série de furos, corte, divisão, adicionar/remover (CAD avançado B: Assembly Features / CAD avançado A: Assembly Feature, Hole Series, Propagate / CAD avançado C: Assembly features (cut, hole) / Fenkra: AsmFeat 0/1)

- **O que faz:** uma operação que atinge várias peças de uma vez (furo passante, corte, divisão).
- **CAD avançado B:**
  - Regras: só entre filhos do produto ativo (mínimo duas peças); não entre elementos do mesmo componente; só em componente que permite criar feature em contexto (exclui referências cruzadas e referências externas). [V]
  - Tipos: **Split**, **Hole**, **Hole Series**, **Pocket**, **Remove**, **Add**. [V]
  - Cada um tem duas listas: *Parts possibly affected* e *Affected parts*, mover com setas ou duplo clique; *Highlight affected parts*. [V]
  - O nó "Assembly Features" contém "Assembly Split.1", etc. Em cada peça afetada é gerada uma feature **real** com um símbolo de seta (link). Se a peça é instância de uma original, a feature aparece em cada instância. [V]
  - **Hole**: limites *Blind, Up to Last, Up to Plane, Up to Surface* (Up to next não serve para furo de montagem); direção normal à superfície ou por aresta/linha; fundo em V; tipos Simples, Cônico, Rebaixado, Escareado, Rebaixo-furo; rosca. [V]
  - Edição: duplo clique permite mudar a lista de peças afetadas ou o furo; o sketch de posição se edita na peça de origem. **Isolate** quebra o link da feature gerada e a deixa como furo comum. [V]
  - Reutilizar furos do Part Design: só se a referência for plana; não "Up to next"; não se refura onde já existe furo, exceto se o diâmetro for maior ou a posição diferir ("não se fura vácuo"). [V]
- **CAD avançado A:**
  - **Assembly Feature** vale só na montagem, salvo ligar *Propagate features to parts* em *Feature Scope*: a feature é transmitida à peça, fica **vinculada** (referência externa), é visível mas editável só em contexto, e exige permissão de escrita nos dois arquivos (em PDM, ambos em check-out). [V]
  - **Hole Series**: assistente em passos (posição → tipo do furo inicial → intermediário → final). A posição vem de sketch, de assistente de furos ou Hole Series existente. O furo intermediário dimensiona-se por padrão como o inicial (desmarcar *Auto size based on start hole*). Atravessa os componentes não suprimidos que o eixo cruza, mesmo sem tocar-se; os furos residem nas peças como features em contexto; editar a série na montagem atualiza as peças. [V]
  - Espelhar feature de montagem só chega ao nível da peça se a feature-pai tiver sido propagada. [V]
- **CAD avançado C:** features de montagem ficam **somente no arquivo da montagem** ("as definições não são passadas ao componente"). Usam o mesmo fluxo das de peça, mas atingem vários **participantes** (nós sob a feature; pode-se adicionar/remover). Operações: chanfro, filete, varredura, revolução, extrusão, furo, rasgo; também *rollback*; aparecem em desenhos de montagem, não nos das peças; uma feature em A não pode ser restringida à mesma feature em B. Uso típico: furação conjugada pós-montagem. [V]
- **Fenkra hoje:**
  - `AsmFeature` (`lib.rs:262`) com `AsmFeatKind::Cut { tool, clearance }` e `Hole { p, d, dia, depth }` (`lib.rs:274-279`).
  - **Corte:** `apply_features` (`view.rs:189-225`) faz `boolean(Cut)` da forma da peça-ferramenta posicionada. `clearance` existe no tipo mas **não é usada** no corte (o campo é lido apenas na UI, nenhuma folga é aplicada em `view.rs`). [C: não vi aplicação em `view.rs`]
  - **Furo:** ponto e direção **em coordenadas do mundo**, diâmetro padrão 6 (`asm_ui.rs:652`), profundidade vazia = passante; tool é `poly_prism` de **64 lados** (`view.rs:209-215`). O furo resulta poligonal e ligeiramente menor que o nominal; não é reconhecido como cilíndrico por `geom_at`.
  - Como é em coordenadas do mundo, o furo **não acompanha** a movimentação das peças. O texto da barra diz "Segue as posições (associativo)" (`toolbar.rs:233`) — vale para o Corte (a ferramenta é um componente), não para o Furo.
  - Não grava nada nas peças. Não entra no `hole_table` nem nos Fixadores (catálogo MON-12).
  - Cada recomposição reexecuta os booleanos para todos os componentes afetados (sem cache).
  - Face criada pelo furo não é restringível de forma fiável: `mate_ref` consulta a forma **crua** da peça (`asm_ui.rs:363-367`), onde essa face não existe, e pode devolver a face mais próxima errada. [C]
  - `delete` limpa as features que dependem de um nó (`lib.rs:626-629`) — correto. `extract_subassembly` apaga as features envolvidas sem transferir (ver 2.4).
- **Melhor prática (B57, ADR 0011 item 4):**
  1. Suporte: **eixo** (de um furo existente, de sketch/ponto publicado) ou face + ponto.
  2. Modo: **Gravar nas peças** (padrão: cria um furo real em cada peça do escopo com link; editável só pela montagem; Isolar quebra) ou **Só na montagem** (com participantes explícitos).
  3. Parâmetros: escopo (lista "possivelmente afetadas" × "afetadas", como o CAD avançado B), tipo por peça (folga na de cima; roscado/cego na de baixo), diâmetro por norma, limites (Cego/Até o último/Até plano/Até superfície), normal à face ou direção.
  4. Avançado: padrão do furo, posição por ponto publicado, tolerância de ajuste.
  - Regras: o furo é um cilindro verdadeiro (não polígono); validar que **toda** peça do escopo recebeu o furo e avisar das que não receberam; não furar onde já existe furo igual.
  - Testes:
    - furo na placa A e B e depois mover B 2 mm (o furo segue o eixo, não o mundo);
    - desfazer remove o furo das duas;
    - "Isolar" na peça B permite editá-lo sem afetar A;
    - a tabela de furos lista os furos gerados;
    - peça fora do escopo permanece intacta;
    - diâmetro 6,000 produz face cilíndrica de r=3,000 (reconhecida por `geom_at` como `Axis`).

### 2.25 Padrões de componentes (CAD avançado B: Reuse Pattern, Define/Fast Multi-Instantiation / CAD avançado A: Linear/Circular Component Pattern, Pattern Driven, Feature Driven, Chain / CAD avançado C: Pattern Components (associative, rectangular, circular) / Fenkra: AsmPattern, CompPattern)

- **O que faz:** repete um componente (parafuso, pino) sobre um padrão linear, circular ou sobre um padrão de furos já existente.
- **CAD avançado B:**
  - **Multi-Instantiation**: três modos de parâmetro (*Instances & Spacing*, *Instances & Length*, *Spacing & Length*); direção por eixo ou por linha/aresta; *Reverse*; *Define as Default* para reusar (Ctrl+D = Fast). [V]
  - **Reuse Pattern**: reusa um padrão do Part Design (retangular, circular ou **User pattern**). Com *Keep link with the pattern* cria associatividade; apagar uma instância fora da definição faz com que ela seja recriada no próximo update. A primeira instância pode: reusar o original, criar nova instância, ou recortar-e-colar o original. Opção *Put new instances in a flexible component* agrupa em "Gathered Part2 on RectPattern.1". Pode-se **reusar as restrições** originais (lista com *All/Clear*). A feature "Reused Rectangular Pattern.1" fica em Assembly features, com *Definition* (atualização local das instâncias não verificadas) e *Deactivate/Activate*. Mudar o padrão para 5×5 refaz 17 instâncias, mantendo a associatividade. [V]
- **CAD avançado A:** *Pattern Driven Component Pattern* e *Feature Driven*: copia um componente já colocado e restringido sobre as posições de um padrão (tipicamente de assistente de furos); as cópias herdam os mates. [V] Substituir o semente substitui todos do padrão. [V]
- **CAD avançado C:** **Pattern Components**: aba *Associative* liga ao padrão de features da peça (as relações dos patternados são copiadas); abas Retangular (colunas × linhas, direção, flip, contagem padrão 2, espaçamento, negativo inverte) e Circular (eixo de rotação, contagem padrão 4, ângulo padrão 90°). Pode-se suprimir ocorrência ou torná-la independente. A associação não funciona com peças derivadas/espelhadas (ideia de comunidade). [V]
- **Fenkra hoje:**
  - `CompPattern` (`lib.rs:301`) com `PatternKind::Linear{dir,count,spacing}` e `Circular{origin,axis,count,angle}` (`lib.rs:312-317`), expressões nos parâmetros. `update_patterns` (`lib.rs:569-613`) cria/apaga cópias.
  - As cópias são nós **fixos** (`fixed: true`), sem número e sem restrições próprias. Não herdam restrições do original. Não há padrão associado a padrão de furos/feature (MON-08 diz "associado ao padrão de furos", mas o código não implementa).
  - `count` inválido vira 1 (`unwrap_or(1.)`) sem erro; não há limite superior (um `count = 1e6` aloca um milhão de nós).
  - Apagar a instância não é possível "sem apagar o padrão" (o `delete` do original apaga o padrão).
  - Direção e centro estão **no mundo**, não amarrados a geometria. Mover o componente de referência não move o eixo do padrão circular.
  - `renumber` e BOM contam as cópias (bom: qty correto).
  - O padrão é refeito a cada `solve` e `add_pattern`, não na leitura do arquivo: abrir um `.fka` antigo mostra o que foi salvo.
- **Melhor prática:**
  - Fluxo: componente semente → **fonte** (Linear / Circular / "Padrão de furos da peça X" / Pontos publicados) → parâmetros (contagem+espaçamento, contagem+comprimento, espaçamento+comprimento) → avançado (reusar restrições, suprimir ocorrências, agrupar em componente flexível).
  - Cópias herdam as restrições do semente (movendo-as para a instância equivalente) e podem ser suprimidas individualmente.
  - Limite de segurança (ex.: 10.000) com mensagem.
  - Testes: contagem 3→5→2 (já testado); contagem "abc" → erro visível; padrão por furos: mudar o furo de 4 para 6 muda o padrão; suprimir ocorrência 2 mantém as outras posições.

### 2.26 Espelhar componentes (CAD avançado B: Assembly Symmetry / CAD avançado A: Mirror Components / CAD avançado C: Mirror Components / Fenkra: ausente)

- **O que faz:** cria o componente simétrico em relação a um plano: nova instância (se a peça é simétrica) ou nova peça "mão oposta".
- **CAD avançado B:** assistente (*Assembly Symmetry Wizard*) com plano de referência e componentes; opções *Mirror, new component*, *Translation, new instance*, *Rotation, new instance*; **Keep link in position** (as simétricas herdam mudanças de posição do original) e **Keep link with geometry** (herdam mudanças de forma; só para Part Bodies ou External View). Sem licença de Part Design, essas opções ficam cinza. Modificar a simetria depois reabre o assistente; "Rotation, same instance" não permite as opções de associatividade. [V]
- **CAD avançado A:** produz **cópia** ou **versão de mão oposta**. Em peças não simétricas, nenhuma orientação é verdadeiro espelho; para espelho de verdade, crie a versão de mão oposta. Podem ser novos arquivos ou configurações derivadas na semente. Mudanças na semente propagam se as referências não foram quebradas. Os espelhados ficam posicionados em relação à semente sem mates extras. biblioteca de normas só permite cópia. [V]
- **CAD avançado C:** estados por componente: **Mirror** (arquivo novo), **Reuse** (instância), **Exclude**, **Mixed** (pai com filhos diferentes). Filhos herdam o estado do pai. Preview: verde (espelhado) e amarelo (reusado). Componentes em negrito são impróprios para reuso (assimétricos). *Mirror Relationships* leva as restrições. A versão 2026 tem espelho associativo. [V]
- **Fenkra hoje:** não existe (`grep` não encontra espelho em `fenkra-montagem`). O geometrista da peça pode ter espelho de corpo, mas não da montagem.
- **Melhor prática:**
  - Fluxo: plano → componentes (árvore inteira) → por componente: Espelhar (nova peça espelhada, link de forma), Reusar (instância; só se simétrica, com teste automático de simetria), Excluir → opções: levar restrições; manter link de posição.
  - Testes: peça com furo assimétrico (avisar "não é simétrica"); espelhar duas vezes volta à posição original; trocar o plano atualiza as cópias.

### 2.27 Explosão e cenas (CAD avançado B: Explode, Scene / CAD avançado A: Exploded View, Explode Lines, Animate / CAD avançado C: Presentation (tweaks) / Fenkra: Composer)

- **O que faz:** separa os componentes para mostrar a estrutura, guardada como vista/cena.
- **CAD avançado B:**
  - Comando *Explode*: **Depth** (todos os níveis / primeiro nível); tipo **3D**, **2D** ou **Constrained** (respeita as restrições axial/axial e plano/plano, com um produto fixo escolhido). *Scroll Explode* mostra o avanço; componentes podem ser movidos depois com a bússola; **OK** mantém a vista (aviso), **Cancel** restaura; *Reset* volta. [V]
  - A explosão serve para criar cenas, imprimir, arquivar ou gerar desenho. Cenas aprimoradas (*Enhanced Scenes*) guardam visibilidade, posição e câmera (DMU Navigator). [V]
- **CAD avançado A:** vista explodida = sequência de **passos** (regular: translação/rotação; **radial**: componentes alinhados ao redor de um eixo num passo só), *Auto-space* ao arrastar (distribui igualmente ao longo de um eixo, ordem pelo centro/trás/frente da caixa), linhas de explosão (*Smart Explode Lines*), animar explodir/colapsar; fica armazenada com a configuração (uma por configuração). [V]
- **CAD avançado C:** **Presentation** com *Tweak Components*; sequências e trilhas; ligada à LOD escolhida; tweaks são excluídos se a peça sair. Substituto LOD não pode ser explodido. [V]
- **Fenkra hoje:**
  - `DisplayState` com `offsets: Vec<(Id, V3)>` (só **translação**), `hidden`, `colors`, `camera` (`display.rs:22-34`).
  - `auto_explode` (`display.rs:117-136`) afasta cada componente do **centro** da montagem (centros dos componentes) por um fator. Ignora restrições e direções naturais (um parafuso não sai pelo eixo do furo). Fixos ficam parados.
  - `Animation::at` interpola offsets e câmera por quadros-chave (curva suave); visibilidade e cor vêm do quadro anterior.
  - Não há passos nomeados, rotação, linhas de explosão, trilhas, explosão por restrição, nem explosão de subconjuntos em níveis.
  - Explosão se aplica via `with_state` somando ao `placement.origin` (`display.rs:149-151`), portanto subconjuntos explodem como um bloco.
- **Melhor prática:**
  - Explosão **por restrição**: para cada componente restrito por eixo/plano, a direção é o eixo/normal da restrição e a distância é proporcional ao tamanho (a mesma ideia do tipo "Constrained" do CAD avançado B).
  - Passos: lista ordenada de movimentos (translação/rotação, grupo de componentes), cada passo editável; linhas de explosão derivadas.
  - Nível: todos / primeiro.
  - Testes: parafuso M6 numa chapa sai ao longo do seu eixo; explodir e recolher devolve posições exatas; duas cenas guardadas diferentes alternam sem mudar `placement`.

### 2.28 Interferência, contato e folga (CAD avançado B: Clash/Contact/Clearance / CAD avançado A: Interference Detection, Clearance Verification, Dynamic Clearance, Collision Detection / CAD avançado C: Analyze Interference / Fenkra: AsmInterf, collides)

- **O que faz:** acha sólidos que se sobrepõem, se tocam ou estão mais próximos que o mínimo.
- **CAD avançado B:** *Compute Clash*: resultado por **cor do ícone**: vermelho = interferência (duas áreas vermelhas na geometria), amarelo = contato, verde = nenhuma. O modo **Clearance** recebe o valor mínimo (ex.: 50 mm) e marca violação se a distância for menor. A precisão depende da tesselação (SAG 0,2 mm). *Stop Manipulation on Clash* (2.10). [V]
- **CAD avançado A:**
  - *Interference Detection* com opções: tratar **coincidência como interferência**, **mostrar interferências ignoradas** (ícone cinza), **incluir interferências de multicorpo**, **tratar subconjuntos como componentes** (não reporta internas), separar **fixadores** numa pasta, pasta de roscas cosméticas coincidentes. [V]
  - *Clearance Verification*: distância mínima aceitável entre selecionados ou selecionados × resto; lista falhas; ignora folgas iguais a um valor; pasta de fixadores. [V]
  - *Dynamic Clearance*: distância ao vivo ao arrastar; o valor entre colchetes é o mínimo alcançado; só em superfícies planas, cilíndricas, cônicas, esféricas e toroidais. [V]
- **CAD avançado C:** conjuntos 1 e 2 (ou dentro de um só conjunto); sólidos temporários de interferência; relatório com **volume e centroide**; duplo clique dá zoom; **ignorar** um volume ou todos abaixo de um valor; filtros por tipo (geral, roscas, Content Center sem rosca, AnyCAD); "tratar subconjuntos como componentes"; recomenda checar poucos componentes próximos e agendar análises grandes fora do horário. [V]
- **Fenkra hoje:**
  - `interferences` (`view.rs:271-292`): para pares de **Parts editáveis e visíveis** faz `boolean(Common)` e reporta se o volume passa de `tol` (0,01 mm³, `asm_ui.rs:681`). Filtra por caixa antes.
  - **Peças dentro de subconjuntos nunca entram** (`NodeKind::Part` apenas); componentes em modo "visualização" também não.
  - Sem folga mínima, sem classificação de contato, sem lista de ignorados, sem relatório de centroide, sem destaque do volume na viewport (só `st.interf`), sem separar fixadores/roscas, sem escopo (conjunto 1 × 2).
  - `collides` repete `placed_shape` (cadeia de cortes) por pares e a cada passo do arraste.
  - O resultado é um número: nenhum sólido de interferência é gerado para visualização (a `Common` é descartada depois de `mass().volume`).
- **Melhor prática:**
  1. Escopo: todos / A × B / dentro de A.
  2. Modo: interferência, contato (toque), folga (valor mínimo).
  3. Opções: tratar subconjunto como bloco; incluir fixadores; ignorar abaixo de V mm³; ignorar par.
  4. Resultado: lista (par, volume, centroide), clique → zoom + o sólido de interseção como sobreposição colorida; "ignorados" persistentes (salvos na montagem com o par e o volume aceito).
  - Testes: parafuso com folga 0,2 em furo (nenhum reporte com folga ≥0), pino 0,1 maior que o furo (interferência 0,1 de raio), peças encostadas (contato, não interferência), peça dentro de subconjunto interferindo com peça do pai (**hoje não detecta**).

### 2.29 Lista de materiais (BOM) e numeração (CAD avançado B: Bill of Material / CAD avançado A: BOM (Top-level, Parts only, Indented) / CAD avançado C: Bill of Materials, BOM Structure / Fenkra: bom.rs, AsmBom)

- **O que faz:** lista estruturada ou plana dos itens, quantidades e propriedades, com números de item estáveis.
- **CAD avançado B:** Analyze > Bill of Material: seção *Bill of Material* (lista as peças e subprodutos), *Recapitulation* (total de peças usadas), *Define formats*; aba *Listing Report* mostra a árvore com indentação; salva em HTML/TXT. [V]
- **CAD avançado A:** três tipos: **Somente nível superior**, **Somente peças** (todas as peças, em qualquer profundidade, sem subconjuntos) e **Indentada** (subconjuntos expandem). Números de item e balões podem ter problemas de asterisco em "Top-level only". [V]
- **CAD avançado C:** abas **Estruturada** (um ou todos os níveis) e **Somente peças** (achatada, somando parafusos). Tipos de **estrutura da BOM** por componente: **Normal** (padrão), **Phantom** (existe no projeto, não é linha da BOM; filhos sobem um nível e multiplicam a quantidade), **Reference** (ignorado, filhos incluídos, sem massa), **Purchased** (uma linha mesmo se for montagem; filhos fora), **Inseparable** (soldado/prensado: uma linha, filhos fazem parte). Componentes virtuais (sem geometria) para itens intangíveis (embalagem, software). A lista de peças do desenho usa sempre a LOD **Master**. [V]
- **Fenkra hoje:**
  - `bom` (`bom.rs:139-174`) com três níveis (`BomLevel::Topo`, `Pecas`, `Estruturada`) e números de item **persistentes por arquivo** (`ItemNo`, `assign_items`, `bom.rs:108-119`): um item mantém o número entre revisões e números de itens removidos **não são reutilizados** (testado em `numeros_de_item_ficam_entre_revisoes`).
  - A chave é o **caminho do arquivo**: duas instâncias do mesmo arquivo somam a quantidade (bom), mas **não há variantes** (mesma peça, material diferente) nem sobrescrita por instância.
  - `BomRow` (`bom.rs:41-50`) só tem item, profundidade, chave, caminho, subconjunto, quantidade, nome (o nome é o da primeira instância encontrada). Sem número de peça, descrição, material, massa; colunas ficam a cargo da UI.
  - Nenhuma noção de Phantom/Reference/Purchased/Inseparable ou de "excluir da BOM". Componentes ocultos e suprimidos contam.
  - `load_sub` relê o JSON do subconjunto do disco a cada chamada, sem cache, e `flat` ainda lê a cada nível. Grandes montagens pagam isso toda vez que a janela é redesenhada (a chamada é na UI a cada quadro? `bom_window` chama `bom(...)` diretamente em `asm_ui.rs:2649` — se for por quadro, é custo por quadro).
- **Melhor prática:**
  - Propriedade por componente `Estrutura da BOM: Normal | Fantasma | Referência | Comprado | Inseparável` e `Incluir na BOM`.
  - Chave = (arquivo, configuração/variante) e propriedades da peça (PN, descrição, material, massa).
  - Cache da BOM invalidado só quando a árvore ou os arquivos mudam.
  - Exportar CSV/HTML.
  - Testes: parafuso ×8 em 2 níveis soma 16 em "Só peças" e 8+8 em "Estruturada"; fantasma promove os filhos com qtd multiplicada; numeração estável após apagar e inserir.

### 2.30 Representações e desempenho: modos, LOD, estados de exibição, grandes montagens (CAD avançado B: Design/Visualization mode, Manage Representations, cache / CAD avançado A: Resolved/Lightweight/LDR, Display States, Configurations, SpeedPak, Envelope / CAD avançado C: LOD, View/Positional representations, Express mode / Fenkra: LoadMode, DisplayState)

- **O que faz:** controla o que está carregado em memória, o que é visível e em que posição, para abrir e trabalhar montagens grandes.
- **CAD avançado B:**
  - **Modo Design** (geometria completa) × **Modo Visualização** (documentos `.cgr`, só a aparência). Modo por componente: Representations > Visualization/Design Mode. Com o cache ligado, o carregamento é bem mais rápido. [V]
  - Em visualização, restrições são resolvidas se os dados vieram da R10+ e, para elementos publicados, da R11+. Passar o cursor sobre um elemento mostra um **olho**: ele pode ser restringido, mas a peça será carregada. [V]
  - Editar a peça muda sozinho para design. *Product Management* troca a **representação** associada. [V]
  - Massa em modo visualização é aproximada (com aviso). [V]
  - *Generate CATPart from Product* cria um CATPart único com um corpo por representação (opção de mesclar). [V]
- **CAD avançado A:** **Resolved** (tudo), **Lightweight** (gráficos + geometria; features sob demanda), **Large Assembly Mode** (leve com opções extras, automático acima de um nº de componentes), **Large Design Review** (só representação gráfica; mede, corta, oculta; comandos que exigem dados saem do LDR). **Display States** mudam só aparência (mais rápido) × **Configurações** mudam o conteúdo (suprimem componentes; BOM diferente; mais lento). **SpeedPak** (configuração simplificada: *mated* mantém geometria resolvida, *graphics* não). [V]
- **CAD avançado C:** **LOD** (Master, sistema: todos os componentes suprimidos / todas as peças / todo Content Center; nomeadas; **substituto**: parte única, Shrinkwrap padrão; temporária). Supressão descarrega componentes; restrições transicionais não resolvem com suprimido; massa pergunta se carrega temporariamente. **Representações de vista** (visibilidade) e **posicionais** (posições/restrições sem afetar o master). Tipos independentes, combináveis. [V]
- **Fenkra hoje:** `LoadMode::{Editavel, Visualizacao}` por nó (`lib.rs:126`): visualização não guarda a forma e **não é lida se estiver oculta** (`view.rs:120-122`). `DisplayState` (visibilidade/cor/explosão/câmera). Não há LOD, substituto, posição nomeada, nem configurações de componente. `PartCache` guarda a peça por caminho e data de modificação (`load.rs:21-63`) mas não há limite de memória nem descarte (LRU).
- **Melhor prática:**
  - Três eixos independentes, como nos CADs: **Carregamento** (completo / leve / suprimido), **Exibição** (estados) e **Posição** (posições nomeadas).
  - Cache LRU com teto de memória e métrica visível.
  - Abrir com "tudo suprimido" e carregar sob demanda (por seleção).
  - Testes: abrir montagem de 2.000 instâncias do mesmo arquivo carrega o arquivo 1× (cache); suprimir/carregar não muda posições; mudar estado de exibição não dispara solve.

### 2.31 Salvar e gerenciar os arquivos da montagem (CAD avançado B: Save/Save Management, Product Management / CAD avançado A: Save All, Pack and Go / CAD avançado C: Save All, Project / Fenkra: AsmSave)

- **O que faz:** grava a montagem e suas dependências, mostrando o que mudou e o que depende do quê.
- **CAD avançado B:** *Product Management* lista part number, arquivo, estado e representação. Documentos contextuais são filhos de seus apoios; apagar apoio pergunta se apaga os dependentes. *Edit > Links* dá visão dos vínculos. [V]
- **CAD avançado A / CAD avançado C:** *Save All* lista arquivos modificados e as dependências; *Pack and Go* (CAD avançado A) e o Projeto (CAD avançado C) empacotam com referências. [C]
- **Fenkra hoje:**
  - `save` (`asm_ui.rs:920-949`) grava só o `.fka` com `std::fs::write` direto (sem arquivo temporário + renomear: queda no meio da gravação corrompe o arquivo).
  - Converte caminhos absolutos em relativos quando dentro da pasta. Atribui números de item.
  - `modified` é ligado só por `push_undo`; mudanças que não passam por ele (renomear, parâmetros) podem deixar o aviso de "não salvo" desligado. [C]
  - Peças editadas no contexto são salvas por `back_to_assembly`/`save_pub`; não há "salvar tudo", lista de modificados, motivo (editado/novo/dependência) nem Pack and Go (B56).
  - Arquivo `.fka` sem `format_version` compatível não é verificado em `from_json`.
- **Melhor prática:** gravação atômica (tmp + rename + backup `.bak`); **Salvar tudo** com diálogo (arquivo, motivo, nome para os novos); verificação de referências ausentes ao abrir (lista no relatório, não só `errors` por nó); Empacotar (copiar dependências e reescrever caminhos).
- **Testes:** matar o processo durante o save não deixa arquivo truncado; mover a pasta inteira mantém os caminhos relativos; abrir com 3 peças faltando lista as 3.

### 2.32 Fixadores automáticos e conexão parafusada (CAD avançado B: Mechanical Standard Parts catalog / CAD avançado A: fixadores inteligentes, Hole Series + biblioteca de normas / CAD avançado C: Bolted Connection Generator, Content Center / Fenkra: preencher_furos)

- **O que faz:** preenche furos com pilha parafuso/arruela/porca, já restringida, ou cria os furos e a pilha numa só operação.
- **CAD avançado B:** peças de catálogo são **copiadas** para a montagem (ISO/EN/JIS/US) e perdem a ligação com o catálogo; famílias via design table. Hole Series é o furo de montagem. [V]
- **CAD avançado A:** *fixadores inteligentes* reconhece furos e insere a pilha; reconhecimento por IA de parafusos/porcas/arruelas (2025 SP3) para SmartMates. [V]
- **CAD avançado C:** Bolted Connection edita-se depois pelo mesmo assistente (catálogo MON-18). [V parcial]
- **Fenkra hoje:** `preencher_furos` (`fixadores.rs:106`) acha furos por eixos coaxiais de restrições (tolerâncias `fixadores.rs:31-37`), mede a profundidade por **sondas laterais** (mediana de acertos, `fixadores.rs:429-467`), cria as peças de hardware em `fixadores/` e as restrições concêntrica+contato. Não edita nem troca a pilha depois; não cria furos (MON-18 ❌).
- **Melhor prática:** depende de 2.24: com furos de montagem reais e `hole_table`, o preenchimento deixa de depender de sondas. Pilha como objeto editável (trocar norma/tamanho/comprimento religa).
- **Testes:** trocar M6x20 por M6x25 mantém as restrições; furo passante em 3 chapas escolhe comprimento ≥ espessura + porca.

---

## 3. Requisitos levantados

Prioridades: P0 = bloqueia a filosofia-alvo / risco de dado errado; P1 = grande valor; P2 = importante; P3 = desejável.

### Requisitos funcionais (RF-MONT)

1. **RF-MONT-01 (P0) Estado de restrição.** Cada restrição tem estado `Verificada | AAtualizar | Impossível | Quebrada | Suprimida | Medida`, mostrado na árvore. *Aceite:* apagar a face de apoio e recarregar a montagem marca a restrição como Quebrada; a peça **não** se move; nenhuma geometria antiga é usada.
2. **RF-MONT-02 (P0) Reconectar.** Reconectar um elemento de uma restrição sem recriá-la (mantém nome, valor, ordem). *Aceite:* reconectar uma Distância a outra face mantém o valor e o id; desfazer restaura.
3. **RF-MONT-03 (P0) Substituir com relatório.** Substituir componente com escopo (esta / todas as instâncias) e tabela de religação. *Aceite:* substituir uma peça sem uma das faces lista a restrição como Quebrada, com o motivo; `n` instâncias trocam num único desfazer.
4. **RF-MONT-04 (P0) Religação por publicação.** A restrição guarda o **nome publicado** além da geometria e prefere o nome na religação. *Aceite:* a peça nova com a mesma publicação religa com 0 quebradas mesmo se a face tiver outro ponto clicado.
5. **RF-MONT-05 (P0) Orientação explícita nas restrições.** Distância/Offset e Ângulo têm orientação (Mesma/Oposta/Indefinida) e setor de ângulo (1–4). *Aceite:* Distância 5 com "Oposta" nunca termina com as normais no mesmo sentido; Ângulo 30° e 330° produzem posições distintas e estáveis.
6. **RF-MONT-06 (P0) Distância zero.** Distância 0 entre planos vira Coincidência/Contato; eixo-eixo e ponto-ponto com 0 usam coincidência, nunca `norm(..)−0`. *Aceite:* o `dof` e a convergência são idênticos para Distância "0" e para Coincidência.
7. **RF-MONT-07 (P0) Pares inválidos recusados.** O tipo de restrição só é criado para pares com resíduo definido (tabela de pares). *Aceite:* criar Ângulo entre dois pontos retorna erro explícito; nenhuma restrição "sem efeito" fica na lista.
8. **RF-MONT-08 (P0) Edição em contexto sem trocar de bancada (B54).** Ativar peça com as outras esmaecidas e selecionáveis; um ativo por vez; Voltar / pai / topo; salvar ou descartar; aviso de instâncias. *Aceite:* editar uma peça com 3 instâncias mostra aviso; descartar restaura o arquivo; as outras peças aceitam seleção para referência.
9. **RF-MONT-09 (P0) Links contextuais com estado (B54).** Estados `Em contexto | Fora | Travado | Quebrado | Pendurado`; Travar/Destravar/Quebrar/Atualizar/Isolar; lista de links por peça. *Aceite:* abrir a peça fora da montagem mostra "Fora de contexto"; travar impede atualização.
10. **RF-MONT-10 (P0) Link com posição viva.** A transformação do link é derivada das posições atuais no update. *Aceite:* mover a peça de origem 10 mm e atualizar move a geometria linkada 10 mm.
11. **RF-MONT-11 (P0) Furo de montagem com dono (B57).** Furo/série de furos definido por eixo/ponto publicado; grava um furo cilíndrico real em cada peça do escopo, com link. *Aceite:* mover uma das peças 2 mm mantém o furo alinhado ao eixo; a tabela de furos lista os furos; "Isolar" permite editar na peça.
12. **RF-MONT-12 (P0) Furo cilíndrico verdadeiro.** O furo de montagem gera face cilíndrica exata. *Aceite:* `geom_at` na parede do furo devolve `Axis` com raio nominal ± 1e-6.
13. **RF-MONT-13 (P1) Publicações na montagem (B55).** Sketch, ponto, eixo, plano, sistema de eixos e publicação no nível da montagem; publicação com nome, substituição da geometria do nome e lista exportável. *Aceite:* substituir a geometria de um nome publicado mantém todas as restrições/links que o usam.
14. **RF-MONT-14 (P1) Produto ativo.** Restrições e features só entre filhos do componente ativo. *Aceite:* tentar restringir peças de níveis diferentes sem ativar o pai produz erro com a dica.
15. **RF-MONT-15 (P1) Subconjunto flexível.** Alternar rígido/flexível; flexível inclui as peças internas no solver do pai e guarda sobrescritas de valores de restrição por instância. *Aceite:* editar o valor de um ângulo numa instância flexível não muda a outra; rígido restaura a sincronia.
16. **RF-MONT-16 (P1) Restrição em peça de subconjunto.** Permitir escolher geometria de uma peça interna a um subconjunto (rígido: religa ao corpo do subconjunto). *Aceite:* restringir um parafuso dentro de um subconjunto sem abri-lo.
17. **RF-MONT-17 (P1) Extração de subconjunto sem perdas.** Features, acoplamentos, padrões, parâmetros e estados de exibição internos vão para o subconjunto; restrições com o exterior são religadas ao nó do subconjunto. *Aceite:* extrair duas peças ligadas por feature e por acoplamento preserva os dois no `.fka` novo; relatório lista o religado.
18. **RF-MONT-18 (P1) Diagnóstico de conflito.** Ao não convergir, listar a restrição culpada e as envolvidas (conjunto mínimo) e marcar redundantes. *Aceite:* duas Distâncias contraditórias entre os mesmos planos identificam as duas; coincidência duplicada vira `Redundante` sem erro.
19. **RF-MONT-19 (P1) Update manual/automático e parcial.** Modo manual com ícone "a atualizar"; atualizar uma restrição/aglomerado. *Aceite:* alterar um valor em modo manual não move nada até Atualizar; atualizar uma restrição move só o seu aglomerado.
20. **RF-MONT-20 (P1) Modos livres de GDL.** Mostrar quais translações e rotações restam (eixo/centro) na viewport. *Aceite:* eixo coincidente mostra 1 rotação + 1 translação com os vetores corretos (±1e-6).
21. **RF-MONT-21 (P1) Limites em restrições e juntas.** Mín/máx para Distância, Ângulo e juntas, com posição de repouso. *Aceite:* Revolução com limites ±90° e comando 120° para em 90° e marca "no limite" (não "falhou").
22. **RF-MONT-22 (P1) Restrições avançadas.** Tangente, travar rotação (concêntrica), Largura, Simetria (B36), em ordem de valor. *Aceite:* Largura centra uma aba de 10 numa ranhura de 14 (folga 2 de cada lado).
23. **RF-MONT-23 (P1) Padrão por fonte.** Padrão de componentes a partir de padrão de furos/feature ou pontos publicados; contagem+espaçamento / contagem+comprimento / espaçamento+comprimento; herdar restrições; suprimir ocorrência. *Aceite:* mudar o padrão de furos de 4 para 6 muda as cópias para 6; suprimir a ocorrência 3 mantém as demais.
24. **RF-MONT-24 (P1) Espelhar componentes.** Assistente com plano e estado por componente (Espelhar/Reusar/Excluir), teste de simetria e link de posição/forma. *Aceite:* peça assimétrica não pode ser "Reusada" (aviso); espelhar duas vezes devolve a posição original.
25. **RF-MONT-25 (P1) Interferência completa.** Escopo A×B, interferência/contato/folga mínima, subconjuntos como bloco ou abertos, ignorados persistentes, centroide e sólido de interseção. *Aceite:* pino 0,1 maior que o furo → interferência; folga mínima 0,5 com 0,3 de folga real → violação; peça dentro de subconjunto detectada.
26. **RF-MONT-26 (P2) Explosão por restrição e passos.** Direção por eixo/normal da restrição, passos editáveis, rotação e linhas de explosão. *Aceite:* parafuso sai ao longo do seu eixo; recolher devolve posições exatas.
27. **RF-MONT-27 (P2) BOM com estrutura.** Estrutura da BOM por componente (Normal/Fantasma/Referência/Comprado/Inseparável), chave por variante, propriedades (PN, descrição, material, massa). *Aceite:* fantasma promove filhos multiplicando a quantidade.
28. **RF-MONT-28 (P2) Posições nomeadas.** Valores de restrição/junta nomeados e reaplicáveis, ligados a estado de exibição e animação. *Aceite:* alternar 100× entre "aberta" e "fechada" retorna ao mesmo placement (±1e-9).
29. **RF-MONT-29 (P2) Smart Move / encaixe por arraste.** Arrastar com Alt prevê e cria a(s) restrição(ões); lista de prioridade configurável; modos Cadeia/Pilha. *Aceite:* eixo+face anelar criam o par "Inserir"; Tab inverte o alinhamento.
30. **RF-MONT-30 (P2) Interfaces de encaixe.** Interface (nome, tipo, geometria) na peça; casamento por nome/tipo/ordem ao inserir e ao substituir; composta (parafuso = eixo + face). *Aceite:* inserir a peça B cria N restrições; faltante é listada.
31. **RF-MONT-31 (P2) Peça esqueleto e fluxo top-down.** Documentar e validar: esqueleto não depende de filhos; Fazer peça a partir de publicações. *Aceite:* criar dependência filho→esqueleto é recusado.
32. **RF-MONT-32 (P2) Salvar tudo e Empacotar (B56).** Lista de modificados com motivo; gravação atômica. *Aceite:* interromper a gravação não deixa arquivo truncado; Empacotar reescreve caminhos relativos.
33. **RF-MONT-33 (P2) Representações.** Suprimir/carregar sob demanda (LOD), posição nomeada, estados de exibição independentes. *Aceite:* abrir com tudo suprimido não lê nenhum `.fkp` além do mínimo; carregar uma peça não muda posições.
34. **RF-MONT-34 (P2) Reordenar com pastas de restrições.** Pastas/sets para restrições e componentes, sem efeito geométrico. *Aceite:* reordenar não muda nenhum placement (bit a bit).
35. **RF-MONT-35 (P2) Fixar em relação a.** Além de "fixar no espaço", fixar relativo a um referencial. *Aceite:* mover o referencial move o fixo; o inverso não.
36. **RF-MONT-36 (P2) Fixação conjunta nomeada com aviso de movimento.** *Aceite:* mover uma peça do grupo oferece "mover todas"; nome editável.
37. **RF-MONT-37 (P3) Acoplamentos multivolta.** Engrenagem/cremalheira/cabo suportam >180° (desenrolar a fase). *Aceite:* comando 720° dá −360° na segunda engrenagem 1:−0,5.
38. **RF-MONT-38 (P3) Erros de expressão visíveis.** Expressão que não avalia marca a restrição como Impossível com mensagem. *Aceite:* razão "z1/0" não trava silenciosamente.
39. **RF-MONT-39 (P3) Geometria de referência da montagem (B22).** Origem, planos e eixos próprios. *Aceite:* restringir ao plano da montagem; mover a montagem não altera.
40. **RF-MONT-40 (P3) Conexão parafusada editável (B58).** Cria furos + pilha, e edita depois. *Aceite:* trocar o comprimento religa tudo.
41. **RF-MONT-41 (P3) Corte com folga.** O campo `clearance` do Corte pela peça é aplicado. *Aceite:* corte com folga 0,2 deixa o rasgo 0,2 maior em todos os lados.
42. **RF-MONT-42 (P3) Dependências.** Árvore de dependências (restrições, associatividade, relações) por componente. *Aceite:* lista os links contextuais e as restrições do componente.

### Requisitos não funcionais (RNF-MONT)

1. **RNF-MONT-01 (P0) Determinismo.** O mesmo documento resolve para as mesmas posições (±1e-9) em execuções repetidas e independentemente da ordem das restrições. *Aceite:* teste com 100 permutações aleatórias da ordem.
2. **RNF-MONT-02 (P0) Nunca perder dado em silêncio.** Operações de estrutura (extrair, substituir, apagar, abrir) não descartam dados sem relatório. *Aceite:* cada descarte aparece em log/aviso; testes de ida e volta (extrair + inserir de volta = igual).
3. **RNF-MONT-03 (P0) Gravação atômica.** *Aceite:* falha simulada entre escrita e renomeação preserva o arquivo anterior.
4. **RNF-MONT-04 (P1) Escala do solver.** Resolver por **aglomerados conectados** e com jacobiano esparso/analítico. *Aceite:* 200 corpos livres em 10 aglomerados resolvem em <1 s; arrastar mantém ≥30 quadros/s com 50 corpos.
5. **RNF-MONT-05 (P1) Reuso de cálculo.** Mudar uma restrição só resolve o seu aglomerado; recomposição só dos componentes alterados. *Aceite:* medição de nº de avaliações por mudança local ≤ tamanho do aglomerado.
6. **RNF-MONT-06 (P1) Cache de arquivos.** Subconjuntos e peças lidos uma vez por data de modificação; teto de memória. *Aceite:* abrir uma montagem com 2.000 instâncias do mesmo arquivo lê o arquivo 1×.
7. **RNF-MONT-07 (P1) Latência de feedback.** Preview de restrição <100 ms para 50 componentes. *Aceite:* benchmark automatizado.
8. **RNF-MONT-08 (P1) Compatibilidade de formato.** Campos novos com `#[serde(default)]`; abrir arquivo mais novo avisa e não destrói. *Aceite:* abrir fixtures dos formatos anteriores continua igual.
9. **RNF-MONT-09 (P2) Mensagens acionáveis.** Todo erro de montagem diz o que fazer e aponta o elemento. *Aceite:* revisão de textos + teste de snapshot.
10. **RNF-MONT-10 (P2) Cobertura de testes.** Cada ficha acima tem ao menos um teste automatizado (os "casos de teste" listados). *Aceite:* relatório de cobertura por requisito.
11. **RNF-MONT-11 (P2) Acessibilidade das cores de status.** Estados distinguíveis sem depender só de cor (ícone+texto). *Aceite:* revisão com simulação de daltonismo.
12. **RNF-MONT-12 (P3) Segurança de limites.** Padrões, explosões e expansões têm teto configurável (ex.: 10.000 instâncias). *Aceite:* contagem 1e6 recusada com mensagem.

---

## 4. Itens de backlog propostos

| # | Título | O que muda | Prioridade | Tamanho | Dependências |
|---|---|---|---|---|---|
| M1 | Estado e ciclo de vida das restrições | `Mate.status`, referências quebradas, reconectar, semáforo na árvore, "Verificada/AAtualizar/Quebrada/Impossível/Medida" | P0 | G | — |
| M2 | Orientação, setor e pares válidos | Distância/Ângulo com orientação e setor; Distância 0 → Coincidência; tabela de pares; recusa de pares sem resíduo | P0 | M | — |
| M3 | Substituir com religação e relatório | Escopo "todas as instâncias", tabela de religação, ordem nome→ponto→proximidade, desfazer único | P0 | M | M1, M5 |
| M4 | Edição em contexto (B54) | Peça ativa sem trocar de bancada, esmaecimento selecionável, Voltar/pai/topo, descartar, aviso de instâncias | P0 | G | M1 |
| M5 | Publicações e referências da montagem (B55) | Sketch/ponto/eixo/plano/sistema; publicação nomeada; substituir geometria do nome; restrição por nome | P0 | G | M1 |
| M6 | Links contextuais com estado e posição viva | Estados `->`/`->?`/`->*`/`->x`, Travar/Quebrar/Isolar, lista de links, transformação derivada no update | P0 | G | M4, M5 |
| M7 | Furo de montagem com dono e cilindro exato (B57) | Furo/série por eixo/ponto publicado; grava furo real nas peças com link; hole_table e Fixadores enxergam | P0 | G | M5, M6 |
| M8 | Extração de subconjunto sem perda | Features, acoplamentos, padrões, parâmetros e estados vão junto; religar restrições externas; relatório | P0 | M | M1 |
| M9 | Gravação atômica e Salvar tudo (B56) | tmp+rename+.bak; lista de modificados com motivo; Empacotar | P1 | M | — |
| M10 | Diagnóstico de conflito e redundância | Conjunto mínimo culpado/envolvido; `Redundante`; update manual/automático e parcial | P1 | G | M1 |
| M11 | Solver por aglomerados e escala | Decomposição em componentes conexos, jacobiano esparso/analítico, reaproveitar a solução anterior | P1 | G | — |
| M12 | Subconjunto flexível e restrição em peça interna | Alternar rígido/flexível; solver inclui corpos internos; sobrescrita de valores por instância | P1 | G | M8, M11 |
| M13 | Modos livres de GDL | Extrair translações/rotações livres do núcleo do jacobiano e desenhar na viewport | P1 | M | M11 |
| M14 | Limites e restrições avançadas (B36) | Limites mín/máx em restrições e juntas; Tangente, travar rotação, Largura, Simetria; depois Caminho | P1 | G | M2 |
| M15 | Interferência, contato e folga | Escopo A×B, contato/folga mínima, subconjuntos, ignorados persistentes, centroide e sólido de interseção | P1 | G | — |
| M16 | Padrão de componentes por fonte | Padrão de furos/feature/pontos publicados; 3 modos de parâmetros; herdar restrições; suprimir ocorrência | P1 | M | M7 |
| M17 | Espelhar componentes | Assistente com Espelhar/Reusar/Excluir; teste de simetria; link de posição e forma | P1 | G | M12 |
| M18 | Encaixe por arraste (Smart Move) | Alt+arrastar com preview, Tab, lista de prioridade, modos Cadeia/Pilha, par "Inserir" | P2 | M | M2 |
| M19 | Interfaces de encaixe (mate refs/iMates) | Interface na peça, casamento por nome/tipo/ordem, composta, uso na inserção e na substituição | P2 | G | M5 |
| M20 | Explosão por restrição e passos | Direção por eixo/normal da restrição, passos editáveis, rotação, linhas, níveis | P2 | M | — |
| M21 | BOM com estrutura e variantes | Normal/Fantasma/Referência/Comprado/Inseparável; chave por variante; PN, descrição, material, massa; cache | P2 | M | — |
| M22 | Posições nomeadas e representações | Posições (valores de restrição), LOD/supressão sob demanda, cache LRU | P2 | G | M11 |
| M23 | Produto ativo e fixar em relação a | Restrições só entre filhos do ativo; Fixar no espaço × relativo; grupo nomeado com aviso | P2 | M | M1 |
| M24 | Peça esqueleto / top-down | Fluxo documentado, proibição de dependência para cima, "Fazer peça de publicações" | P2 | M | M5, M6 |
| M25 | Acoplamentos multivolta e erros de expressão | Desenrolar a fase; erro visível para expressões inválidas; teto de contagem | P3 | P | — |
| M26 | Origem e referências da montagem (B22) | Planos/eixos próprios; Ctrl/Alt e ocultar/apagar como nas peças | P3 | M | M5 |
| M27 | Corte com folga e conexão parafusada (B58) | Aplicar `clearance`; furos + pilha editável | P3 | M | M7 |

**Ordem sugerida de ondas:** M2 e M25 (quick wins sem dependências, corrigem bugs prováveis) → M1 → M8, M9 → M5, M4, M6 → M7 → M10, M11 → restante.

---

## 5. Fontes

CAD avançado B
- (fonte em procedencia.md) (manual oficial Assembly Design V5R14; texto extraído e lido)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://wichita.edu/industry_and_defense/NIAR/Documents/Assembly-Design-R30.pdf (apenas citada em resultados; não abriu)
- https://community.3dcs.com/help_manual/v5_assembly_constraints.htm
- https://www.ibm.com/support/pages/apar/HD21508 e https://www.ibm.com/support/pages/node/5470425 (Keep link / Isolate)
- (fonte em procedencia.md)
- (fonte em procedencia.md)

CAD avançado A (trechos retornados pela busca)
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
- https://goengineer.com/blog/propagating-assembly-features-parts
- (fonte em procedencia.md)
- https://www.cati.com/?p=125090 (Hole Series)
- https://www.cati.com/?p=132596 (MateXpert)
- https://www.cati.com/?p=120210 (Interference Detection) e https://www.cati.com/?p=120215 (Clearance Verification)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://support.hawkridgesys.com/hc/en-us/articles/203486053 (modos de abertura)
- (fonte em procedencia.md))/07%3A_Assembly_Modeling/7.03%3A_Standard_Mates
- https://www.cati.com/?p=119085 (Configurations × Display States)

CAD avançado C
- (fonte em procedencia.md) (Place Constraint)
- (fonte em procedencia.md) (Joints)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (iMates)
- (fonte em procedencia.md) (Create iMates)
- (fonte em procedencia.md) (Edit in place)
- (fonte em procedencia.md) (Assembly features)
- (fonte em procedencia.md) (Skeletal modeling)
- (fonte em procedencia.md) (LOD)
- (fonte em procedencia.md) (Interferência)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (Mirror Components)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (Pattern Components)
- (fonte em procedencia.md) (BOM Structure)
- (fonte em procedencia.md) (Adaptivity, via busca)
- (fonte em procedencia.md) (What's New: iMates)

Outros
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (via busca)
- (fonte em procedencia.md) (CAD avançado F, via busca)
- (fonte em procedencia.md)
- https://soliddna.com/SEHelp/ST6/EN/assemble_parts/flashfit_placement_logic.htm (CAD avançado H, via busca)
- (fonte em procedencia.md)
- https://community.3dcs.com/help_manual/mouse-operations_2.htm e https://www.cgeo.uliege.be/CAO/CAD avançado G/41_Tutorial_Tank_en.pdf (CAD avançado G, via busca)

Código do Fenkra (leitura): `crates/fenkra-montagem/src/{lib,solve,kin,view,bom,display,load,estrutura,fixadores}.rs`, `crates/fenkra-app/src/asm_ui.rs`, `docs/benchmark/catalogo.md` (MON-01…19), `docs/backlog/backlog.md`, `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`.

---

**Handoff.**
- **Resultado:** o relatório acima é a entrega (nenhum arquivo do repositório foi criado ou alterado). O único arquivo gerado foi um texto temporário extraído do PDF do CAD avançado B, no diretório de rascunho da sessão.
- **Evidência:** leitura integral dos módulos de montagem e das partes de `asm_ui.rs` citadas (referências arquivo:linha no texto); manual oficial CAD avançado B lido via extração de texto; ajuda do CAD avançado C e do CAD avançado E lida diretamente.
- **Verificações executadas:** apenas leitura. Não rodei `cargo test` nem compilei nada. Os "bugs prováveis" vêm de leitura de código e **não foram reproduzidos**: orientação livre na Distância, Distância 0 singular, religação silenciosa, hole poligonal, furo em coordenadas do mundo, `clearance` sem uso, link com transformação congelada, perda de features/acoplamentos na extração, interferência ignorando subconjuntos.
- **Falhas e riscos:**
  - help.CAD avançado A.com não é legível pelo WebFetch (renderizado por JavaScript), e Javelin e TriMech retornaram 403. O conteúdo do CAD avançado A vem de trechos de busca e de blogs de revendas. Confirmar no help oficial antes de copiar nomes de opções.
  - CAD avançado F, CAD avançado G, CAD avançado H e Fusion vêm só de trechos de busca; use como contexto, não como especificação.
  - Duas afirmações do catálogo (`catalogo.md`) divergem do código: MON-04 ("limites") e MON-08 ("associado ao padrão de furos").
  - `clearance` sem uso e o link com transformação congelada estão marcados [C], pois não rastreei todos os usos no app.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Relatório de Montagem entregue com 32 fichas, 42 RF, 12 RNF e 27 itens de backlog; as cinco maiores lacunas são (1) restrições sem estado nem reconexão (referência perdida é silenciosa), (2) edição em contexto, links contextuais e publicações ausentes ou rasos, (3) subconjunto sem modo flexível nem restrição em peça interna e com extração que perde dados, (4) features de montagem sem dono, furo poligonal em coordenadas do mundo e não gravado nas peças, (5) solver sem diagnóstico de conflito, sem decomposição por aglomerados, sem limites nem restrições avançadas e com Distância sem orientação.
