# Visão macro do software e requisitos

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

> Pesquisa de plataforma: CAD avançado B × CAD avançado A × CAD avançado C × Fenkra (alpha.27, `develop`). Nada no repositório foi criado ou alterado.
> Convenção: **[V]** = confirmado em fonte pública consultada; **[C]** = conhecimento geral, não confirmado nesta pesquisa; **[R]** = lido por mim no repositório. Muitas fontes públicas são blogs de revendas, fóruns e notas de suporte, e poucas são ajuda oficial. Onde a fonte é secundária, eu digo.

---

## 1. Resumo e principais lacunas

**Resumo (10 linhas).**
1. O Fenkra é um CAD Rust + OpenCASCADE de ≈58 mil linhas, 7 crates e ≈151 testes `#[test]`. Tem 6 bancadas (sólidos, superfícies, montagem, chapa, estrutural, desenho) sobre 3 formatos JSON (`.fkp`, `.fka`, `.fkd`) e um projeto em pasta (`projeto.fenkra.json`) [R].
2. A fundação de modelagem está acima da média de um alpha: árvore com origem/referências/parâmetros/coletores, linha de tempo com rollback, publicações, links com estado e referências v2 (dono + papel + assinatura) [R].
3. A filosofia de plataforma do CAD avançado B é o objeto em trabalho, os contêineres ordenado/não ordenado, as publicações, o Edit Links/Isolate e as configurações em camadas com bloqueio administrativo. No Fenkra ela existe só em documentos (ADR 0010/0011, onda 0 do backlog) e quase nada está em código.
4. O que CAD avançado B, CAD avançado A e CAD avançado C têm como **plataforma** e o Fenkra ainda não tem: projeto com caminhos de busca (CAD avançado C `.ipj`, CAD avançado A *File Locations*), templates, estilos e normas da empresa, configurações em camadas e travadas, API estável, features definidas pelo usuário e iLogic/FeatureScript.
5. Os arquivos não têm migração de versão. `FORMAT_VERSION = 1` está fixo e `from_json` é `serde_json::from_str` puro (`doc.rs:18,2781`). Um campo novo sem `#[serde(default)]` quebra toda peça antiga.
6. Componentes e links são identificados por **caminho**, não por identidade (`NodeKind::Part{path}` em `fenkra-montagem/src/lib.rs:119`; `ExternalRef.path` em `doc.rs:449-469`). A ADR 0009 promete "ID interno estável", mas isso vale só dentro do índice do projeto.
7. O checklist marca como feito o que o código não faz. STEP/IGES/BREP "import/export" é, na prática, **exportar STEP e STL**; `import_step` existe no kernel e **nenhum código o chama** (`fenkra-kernel/src/lib.rs:699`). "Salvamento automático" é só salvar ao trocar de documento (`app.rs:1153-1155`). Não há recuperação após falha.
8. Robustez: o documento é gravado sem escrita atômica (`app.rs:819`; `fenkra-project/src/lib.rs:213-214`, só o índice usa tmp+rename). O desfazer guarda até 200 cópias completas do `Document`, e a cada quadro há `self.open.doc.clone()` (`app.rs:6622`) seguido de comparação (`app.rs:6759`).
9. Qualidade e conformidade: não há CI (`.github` ausente), nem `LICENSE`/`THIRD_PARTY.md` no repositório, embora o README e o Cargo declarem LGPL-2.1-or-later. O build e a configuração do usuário são de fato Windows-only (`%APPDATA%` em `settings.rs:460`, `release.ps1`, vcpkg `x64-windows-release`).
10. O backlog (onda 0, B53–B62) cobre bem contexto, publicações, salvar tudo, diagnóstico e corpus. **Nenhum item** cobre migração de formato, identidade de documento, interoperabilidade 3D real, templates/normas/configurações em camadas, projeto com caminhos de busca, CI/licença/Linux, API headless estável ou autorrecuperação.

**Cinco maiores lacunas** (detalhe nas fichas 1, 11, 14–15, 20, 21 e 23):
1. Formato sem migração e documentos/links sem identidade estável.
2. Interoperabilidade 3D mentirosa no checklist: sem importar STEP/IGES, exportar IGES/BREP/3MF, nem XLSX.
3. Plataforma de empresa ausente: templates, normas, configurações em camadas, bloqueio e projeto com caminhos de busca.
4. Robustez e engenharia ausentes: escrita atômica, autorrecuperação, CI, Linux, licença e `THIRD_PARTY`.
5. Extensibilidade parada em macros próprias: sem API/CLI headless estável e sem features definidas pelo usuário.

---

## 2. Fichas

### 2.0 Mapa macro do Fenkra hoje (base para as fichas)

**Módulos e bancadas** [R]: `fenkra-core` (documento, árvore, sketch+solver, parâmetros, PMI, macros), `fenkra-kernel` (fachada C++ do OCCT via `cxx`, com fallback "sem kernel" em `lib.rs:968-974`), `fenkra-solidos` (recompute de todos os comandos, biblioteca, perfis, fixadores, engrenagens, eixos), `fenkra-montagem` (restrições, solver LM, juntas, BOM, fixadores), `fenkra-desenho` (vistas, cotas, PDF/SVG/DXF R12), `fenkra-project` (versões/revisões/PN), `fenkra-app` (egui+wgpu). As bancadas são o enum `Bench` (`settings.rs:161-170`) e os 29 grupos de comando são o enum `Group` (`settings.rs:18-91`). Os módulos **não** são plugins carregáveis, ao contrário do que a ADR 0005 propõe.

**Arquitetura de dados.**
- **Peça `.fkp`**: JSON único `Document` (`doc.rs:2673`) com propriedades, parâmetros, referências, coletores, pastas, linha de tempo, PMI, aparência, links externos (`external`) e `sheet`.
- **Montagem `.fka`**: JSON `Assembly` (`montagem/lib.rs:327`) com nós que apontam arquivos por caminho, restrições, juntas, padrões, estados de exibição, PMI e `features` de montagem.
- **Desenho `.fkd`**: JSON `Drawing` com vistas cujo `Source{path}` aponta peça/montagem.
- **Projeto**: pasta com `projeto.fenkra.json`, `pecas/<id>.fkp` e `.versoes/<hash>.fkp` (blake3, 16 hex). O índice guarda `DocEntry{id,file,name,pn,state,versions,revisions}` (`fenkra-project/src/lib.rs:72-105`).
- **Publicações**: flag `published` em referências, parâmetros e coletores. `publications(doc)` recalcula a peça e devolve `(nome, LinkedGeo)` (`fenkra-solidos/src/lib.rs:4551`). O link guarda cópia (`snapshot`) e estado `Ativo/Congelado/Quebrado/Desativado` (`doc.rs:438-469`).
- **Revisão**: `release()` marca o hash da última versão como revisão A, B… e `state=Liberado`. O arquivo de trabalho continua editável e volta a "Em trabalho" ao salvar (`lib.rs:221-223,240-251`).

**Fluxos ponta a ponta hoje** [R]:
- *Peça → desenho*: `.fkp` → `.fkd` (vista com `Source.path`) → PDF/SVG/DXF. A vista tem modo ligada/travada de forma parcial (checklist `[~]`).
- *Montagem → PLM*: `.fka` → BOM/CSV (`asm_ui.rs:2694`). O projeto exporta CSV **plano** dos documentos (`lib.rs:267`), não uma estrutura de produto.
- *Importação → fabricação*: só malha STL/OBJ (máx. 200 mil triângulos, `mesh_io.rs:11`) vira `MeshImport`. A saída de fabricação é DXF planificado, lista de corte CSV (`st_ui.rs:396`) e STEP/STL. Não há importação de STEP nem XLSX.

**Divergências documento × código** (devem virar itens): ADR 0009 diz SQLite, o código usa JSON; ADR 0005 diz plugins, o código usa enum fixo; RF-GPR-021 está `[x]`, mas os links são por caminho; RNF-007 `[x]` não tem autorrecuperação; RNF-014 (mm/cm/m/pol) não existe, porque `Unit` só tem mm, ° e adimensional (`doc.rs:1096-1111`).

---

### 1) Modelo de documento e formato de arquivo
**(CAD avançado B: CATPart/CATProduct/CATDrawing, tipos fixos / CAD avançado A: SLDPRT/SLDASM/SLDDRW / CAD avançado C: IPT/IAM/IDW / outros: CAD avançado E sem arquivo / Fenkra: `.fkp/.fka/.fkd`)**
- **O que faz:** define o que é um documento, como se identifica e como evolui entre versões do programa.
- **CAD avançado B:** três tipos principais, com o Product como "contêiner de instâncias" que referencia partes por arquivo [V: EDS lista o *instance link* guardado no CATProduct com nome e caminho do arquivo]. O CATDrawing guarda um *view link* com nome e caminho da fonte e avisa quando a peça mudou [V].
- **CAD avançado A:** formatos por tipo e referências por nome de arquivo, reparáveis na caixa *References*/*Find References*. Só se reassocia peça com peça e montagem com montagem [V].
- **CAD avançado C:** referência por caminho resolvido pelo projeto `.ipj` (ficha 11) [V]. Iniciar uma sessão exige um projeto ativo.
- **Outros:** CAD avançado E guarda tudo num "documento" na nuvem, com versões e ramos, sem caminho de arquivo [C].
- **Fenkra hoje:** `Document.format_version = 1` (`doc.rs:18,2674`); `from_json` é serde direto, sem `migrate` (`doc.rs:2781`). Uma peça aberta de uma alpha anterior só funciona se todo campo novo tiver `#[serde(default)]`, e nada garante isso. Arquivo de versão **maior** não é detectado. `Assembly` e `Drawing` repetem `format_version: 1` hardcoded (`montagem/lib.rs:472`, `desenho/lib.rs:540`). `open_file` filtra só `.fkp` (`app.rs:832`) embora `open_path` trate `.fka/.fkd`. O tipo "geometria mestre" (RF-NUC-001) não existe como tipo: a bancada de chapa é inferida por `d.sheet.is_some()` (`app.rs:867`).
- **Melhor prática:** (1) cada documento grava `kind`, `uuid`, `format_version` e `app_version`; (2) cadeia de migrações `vN→vN+1` testada por fixtures de cada alpha; (3) arquivo de versão futura abre em leitura, com aviso, e nunca é sobrescrito; (4) `uuid` é a chave de links e componentes, com caminho só como dica. **Testes:** fixtures `.fkp/.fka/.fkd` congeladas por alpha abrem e recomputam igual; arquivo com `format_version=99` abre somente leitura; renomear o arquivo não quebra a referência.

### 2) Árvore de especificação e objeto em trabalho (objeto de trabalho)
**(CAD avançado B: DIWO / CAD avançado A: linha *Rollback bar* / CAD avançado C: *End of Part marker* / Fenkra: marcador `rollback`)**
- **O que faz:** define **onde entra o próximo comando** e qual parte da árvore está ativa.
- **CAD avançado B:** o objeto em trabalho é o único item sublinhado da árvore. Se um Geoset é o IWO, o próximo comando entra no fim dele. Em Part Body ou Ordered Geoset, um comando pode virar IWO e "voltar" o projeto para inserir no meio, e tudo depois é afetado. Arrastar e soltar define o IWO automaticamente [V: cati.com]. A operação booleana depende do IWO no momento da criação [V: nota IBM, via busca]. Um Geoset definido como IWO vira "pai" por padrão [V: Eng-Tips FAQ, via busca].
- **CAD avançado A:** barra de rollback na FeatureManager, que insere antes do ponto e suprime o que vem depois [C].
- **CAD avançado C:** marcador *End of Part* na árvore do navegador; criar após mover o marcador insere no meio [C].
- **Outros:** o *Timeline* do Fusion tem marcador equivalente [C].
- **Fenkra hoje:** `Document.rollback` (`doc.rs:2710-2711`) e `BodyCollector.marker` por coletor (`doc.rs:1269-1271`). `is_active` combina supressão, marcador geral e marcador do coletor (`doc.rs:2832-2848`). Não há conceito de "objeto em trabalho" visível: o destino do comando é `active_collector`. Sketches livres (pasta de geometria) ficam fora da linha de tempo (`doc.rs:2827`). Falta indicador, barra de fim de peça e inserção no meio por padrão (B59).
- **Melhor prática:** um único "em trabalho" por documento, mostrado sublinhado; criar comando = inserir **depois** dele; mover o marcador reavalia só o necessário. Regras que evitam bug: o comando criado com marcador no meio nunca usa entidades criadas depois dele; ao sair do rollback, comandos "futuros" voltam sem perder referências. **Testes:** inserir filete antes de um furo e voltar o marcador conserva ambos; apagar o IWO move o IWO para o pai.

### 3) Contêineres ordenado × não ordenado e elementos de referência
**(CAD avançado B: Part Body / Geometrical Set / Ordered Geometrical Set / CAD avançado A: pastas + *Surface Bodies* / CAD avançado C: *Work Features* e *Folders* / Fenkra: coletores de corpos, pasta Referências, pastas de geometria)**
- **O que faz:** separa geometria que **consome** a anterior (sólido, ordenada) de geometria que apenas **referencia** (superfícies/arames, não ordenada), e guarda os elementos de referência.
- **CAD avançado B:** Part Body é ordenado de cima para baixo (cada comando consome o anterior) e pode conter Ordered Geosets e outros Part Bodies. Geoset é não ordenado (comandos referenciam, não consomem) e só pode conter Geosets. Ordered Geoset é ordenado, pode conter Part Bodies e não aceita Geoset. Reordenar um Geoset não muda a geometria; reordenar corpo ordenado pode mudar. Há *Auto Sort* só para Geosets, *Change Geometrical Set* com "mover pais não compartilhados/todos os pais" e arrastar entre conjuntos [V: cati.com].
- **CAD avançado A:** não tem o conceito de ordenado/não ordenado. Planos/eixos/pontos vão para a árvore na ordem de criação, com pastas só para organizar [C].
- **CAD avançado C:** *Work planes/axes/points* são objetos da árvore, com visibilidade própria e pastas [C].
- **Fenkra hoje:** a árvore fixa traz Origem, `references: Vec<RefElement>` com planos/pontos/curvas/superfícies (`doc.rs:384`, `RefKind` em `doc.rs:472+`), `collectors` e `folders` com `FolderKind::{Comandos, Geometria}` (`doc.rs:1278-1296`). A pasta **Geometria** é na prática o "Geoset não ordenado", e os coletores com linha de tempo são o "Part Body". Faltam as regras de aninhamento (qual contêiner aceita qual) e o "Reordenar/Auto Sort" para o conjunto não ordenado (B59).
- **Melhor prática:** adotar a regra: **só sólido/consumo é ordenado; referências são livres e podem ser reordenadas sem recomputar sólido**. Impedir que um sketch livre dependa de feature posterior (ou avisar). **Testes:** reordenar pasta de geometria não muda volume; mover um plano para dentro de coletor ordenado é recusado com explicação.

### 4) Publicações e interface pública do documento
**(CAD avançado B: Publication / CAD avançado A: nenhum equivalente direto (usa *mate references*, *Interface*) / CAD avançado C: *Publish* para iLogic e pontos de montagem de fixação / Fenkra: `published`)**
- **O que faz:** declara o que outros documentos podem referenciar, com nome estável.
- **CAD avançado B:** publicação dá um nome ao elemento, e restrições de montagem sobre elementos publicados **reconectam sozinhas** quando a peça é substituída [V: EDS]. Publicar torna os projetos reutilizáveis e permite gerenciar os links [V: City Univ.]. Há relatos de links de publicação quebrando quando a peça é substituída em versões antigas [V: notas IBM, via busca].
- **CAD avançado A:** referências de acoplamento (*mate references*) e o recurso *Replace Components* religam por nome/geometria equivalente [C].
- **CAD avançado C:** pontos/vetores publicados e parâmetros exportados (*exported parameters*) formam a interface entre arquivos [V: ajuda "About Parameters in Models"].
- **Fenkra hoje:** `Parameter.published`, `RefElement.published`, `BodyCollector.published`. `publications()` recalcula a peça inteira em cada consulta (`solidos/lib.rs:4551-4602`) e cria um `Ctx` por chamada. Não há tipos de publicação (ponto+vetor+cor de furo, B58), nem publicação automática a partir da montagem (ADR 0006 item 9, B55). `refresh_links` relê o arquivo por caminho e compara pelo **nome** da publicação (`app.rs:2278-2293`), o que é correto para renomear dentro da peça de origem mas sem aviso de "publicação removida" (só conta "quebrados").
- **Melhor prática:** publicação = nome + tipo + (opcional) papel/cor; o nome é a chave pública; renomear exige "religar dependentes" ou alias. Listar "usado por" antes de remover. **Testes:** remover publicação usada → link vai a Quebrado com cache e mensagem; renomear com alias mantém os dependentes.

### 5) Links externos: estados, travar, isolar, quebrar, reparar
**(CAD avançado B: Edit > Links, Isolate / CAD avançado A: `->`, `->?`, `->*`, `->x`, Lock/Break, Find References / CAD avançado C: *Adaptive*, *Break link*, *Derived* / Fenkra: `LinkState`)**
- **O que faz:** mantém e mostra o vínculo entre arquivos, e permite cortá-lo conscientemente.
- **CAD avançado B:** cinco tipos de link: CCP (colar como resultado com link), KWE (parâmetro), instância, restrição e vista. Links são unidirecionais, de filho para pai. **Edit > Links** lista os documentos apontados. Restrições de montagem **não** aparecem ali. **Isolate** corta a conexão entre referência externa e origem. A opção *Keep link with selected object* mantém o vínculo ao editar cópia. A sincronização é automática no update ou manual [V: EDS, City Univ., IBM]. Arquivo movido gera erro "File could not be found"; resolve-se com *Desk* ou clicando com o botão direito na peça vermelha e usando *Find* [V: EDS].
- **CAD avançado A:** sufixos de estado na árvore: `->` em contexto, `->?` fora de contexto (caminho de atualização indisponível; abra a montagem), `->*` travada (não atualiza no rebuild), `->x` quebrada (permanente, não reativa). Podem combinar (`->*x`) [V: ajuda CAD avançado A 2019 e Hawk Ridge, via busca]. *Lock*/*Break* na caixa *External References* [V: ajuda CAD avançado A 2025, título].
- **CAD avançado C:** parâmetros vinculados a Excel ou a arquivo de parâmetros globais; peças derivadas (*Derived Part*); qualquer tipo de modelo pode vincular a outro [V: ajuda "About Parameters in Models"].
- **Fenkra hoje:** `ExternalRef` com `path`, `snapshot` e `state` (`doc.rs:449-469`). Estados existem mas **não aparecem como sufixo na árvore** (B54). Não há política de atualização por link (RF-LNK-012: ao abrir/manual); a única ação é `refresh_links` para tudo (`app.rs:2278`). Não há detecção de ciclo **entre documentos**; os únicos testes de ciclo são de parâmetros (`doc.rs:3179-3200`) e de subconjuntos (`montagem/view.rs:89`). Não há "Isolar" explícito, só relink por assistente (`app.rs:2309-2320`).
- **Melhor prática:** estados visíveis com os 4 sufixos; ações **Travar/Destravar/Quebrar/Isolar** no menu do item; política por link (automática, ao abrir, manual); verificação de ciclo ao criar o link, com a cadeia no erro. **Testes:** A→B→A é bloqueado com o ciclo descrito; link travado não muda após alterar a origem; quebrado mantém o cache e o comando dependente continua calculando.

### 6) Atualização e diagnóstico
**(CAD avançado B: Update / Update Diagnosis / CAD avançado A: Rebuild + *What's Wrong* / CAD avançado C: *Update* + *Design Doctor* / Fenkra: recompute em *worker* + janela de reparo)**
- **O que faz:** recalcula o modelo e explica por que um comando falhou.
- **CAD avançado B:** atualização automática ou manual por documento; objeto desatualizado ganha marca; o diagnóstico oferece Editar, Isolar, Desativar, Reordenar [C — o ADR 0011 cita o fluxo; esta pesquisa não achou fonte oficial]. O cache de documentos (*Work with the cache system*) influencia a ativação de referências externas [V: nota IBM].
- **CAD avançado A:** *What's Wrong* lista erros e avisos de rebuild; *Force Rebuild* ignora otimizações [C].
- **CAD avançado C:** *Design Doctor* é o assistente de reparo [C].
- **Fenkra hoje:** `Worker::spawn` roda `recompute_cached` numa thread, descartando pedidos obsoletos, com `Cache` que refaz do primeiro comando alterado (`app.rs:60-95`). Há `auto_update` (`app.rs:6780`). `RecomputeResult` guarda `status` por comando e `elapsed_ms` **total** (`solidos/lib.rs:4532-4546`). A montagem usa `PartCache::get`, que chama `recompute` **síncrono** dentro do `get` (`load.rs:47`); se isso corre na thread da UI não foi verificado, mas o desenho do código sugere que sim ao abrir a montagem. A janela de reparo só edita (GER-11).
- **Melhor prática:** cada comando tem estado `ok | desatualizado | falhou | suprimido` e um motivo textual; update manual com contagem de pendentes; diagnóstico com 5 ações. Medir tempo **por comando** e registrar. Regra: falha em um comando **não invalida** os independentes. **Testes:** quebrar a referência de um filete deixa só o filete em erro, e os demais continuam com volume correto; "Isolar" congela a geometria atual.

### 7) Parâmetros, fórmulas e conhecimento embutido
**(CAD avançado B: Parameters/Formulas, Knowledge Advisor (rules, checks, reactions), Design Table / CAD avançado A: Equations, Global Variables, Design Table / CAD avançado C: Parameters, iLogic, Excel link / Fenkra: `params`+`expr`)**
- **O que faz:** expressões nas cotas, dependências entre parâmetros e regras.
- **CAD avançado B:** parâmetros e fórmulas valem para todos os usuários; **regras e verificações exigem o produto Knowledge Advisor** [V: página 3DS KWA]. Regra = instruções condicionais sobre parâmetros; também há reações e tabelas de projeto (criar da tabela atual, de arquivo existente, guardar em catálogo ou PowerCopy) [V: GoEngineer e guia City Univ.].
- **CAD avançado A:** *Equations*/*Global Variables* + tabela de projeto Excel para configurações [C].
- **CAD avançado C:** parâmetros de modelo, de usuário e **vinculados** a planilha Excel, parâmetros exportados entre arquivos e peça-esqueleto derivada [V: ajuda]. O iLogic é um *add-in* sobre a API, com IDE em VB.NET [V: AU 2016].
- **Fenkra hoje:** `Parameter{name,expr,unit,comment,published}` (`doc.rs:1114`), avaliação com ciclo detectado (`doc.rs:3179-3200`) e `expr.rs` (278 linhas, 3 testes). `Unit` só tem Comprimento/Angulo/Numero e exibe sempre "mm" (`doc.rs:1096-1111`): não há pol, cm, m, nem tipos texto, booleano ou lista (RF-PAR-004). Não há tabela de configuração (RF-PAR-007) nem regras/verificações (RF-PAR-009).
- **Melhor prática:** unidade guardada no documento e conversão só na exibição; tipos completos; limites mín./máx. com aviso (verificação simples); família por tabela como camada **sobre** parâmetros. **Testes:** converter mm↔pol sem mudar o modelo; limite violado mostra aviso e não bloqueia; linha 2 da tabela gera outra variante sem tocar a linha 1.

### 8) Propriedades e metadados
**(CAD avançado B: Properties + `Product` attributes / CAD avançado A: Custom Properties, Configuration-specific / CAD avançado C: iProperties / Fenkra: `Properties`)**
- **O que faz:** PN, descrição, material, tipo e campos livres, que alimentam BOM, carimbo e PLM.
- **CAD avançado B:** atributos no Product (nº de peça, descrição) lidos pelo CATDrawing/Catalog [C].
- **CAD avançado A:** propriedades por documento e por configuração; modelos de propriedade no *Property Tab Builder* [C].
- **CAD avançado C:** iProperties são metadados que o Vault indexa; propriedades *prompted* **não** são indexáveis, e é preciso mapear iProperties a UDPs do Vault [V: AU e Imaginit].
- **Fenkra hoje:** `Properties{pn,description,material,part_type,custom,stock_length}` (`doc.rs:1158-1181`), padrão de material "Aço 1020". `PartType` tem 6 tipos (sem "montagem", embora RF-PAR-021 a liste). `custom` é `Vec<(String,String)>` só texto, sem tipo nem expressão (RF-PAR-022). Não há modelos de propriedades por empresa (RF-PAR-023).
- **Melhor prática:** campos tipados (texto, número, data, lista), com origem (manual, expressão, calculada da geometria) e um **esquema** de campos obrigatórios por empresa; o mesmo esquema alimenta carimbo, BOM e índice PLM. **Testes:** campo obrigatório vazio bloqueia "Liberar"; propriedade com expressão acompanha o parâmetro.

### 9) Configurações e famílias de peça
**(CAD avançado B: Design Table / Catalog / CAD avançado A: Configurations / CAD avançado C: iPart/iAssembly / Fenkra: nenhuma)**
- **O que faz:** várias variantes (tamanhos, versões) de um mesmo modelo.
- **CAD avançado B:** tabela de projeto criada da peça, de arquivo existente ou guardada em catálogo; também *Part Family Catalog* [V: GoEngineer, City Univ.].
- **CAD avançado A:** *Configurations* + tabela de projeto Excel; cada configuração tem propriedades próprias [C].
- **CAD avançado C:** iPart (fábrica+membros), alimentável por tabela externa, com relato de que a atualização de tabela Excel externa pode ser pouco confiável [V: fórum fornecedor de CAD avançado, anedótico].
- **Fenkra hoje:** não existe. A biblioteca de fixadores/perfis gera geometria por família (`solidos/biblioteca.rs`, `fixadores.rs`), mas uma peça do usuário não pode ter variantes. RF-PAR-007 está sem backlog (B51 trata só fixadores).
- **Melhor prática:** variantes como **conjunto de valores de parâmetros** (por linha), nomeadas, com PN/revisão por variante; a BOM escolhe a variante por instância. **Testes:** 3 tamanhos de flange com volumes esperados; mudar a linha ativa recomputa e salva/abre idêntico.

### 10) Estrutura de produto e lista de peças
**(CAD avançado B: Product Structure / CAD avançado A: Assembly + BOM / CAD avançado C: Assembly + BOM Editor / Fenkra: `Assembly` + `bom.rs`)**
- **O que faz:** árvore de componentes com quantidades, PN e revisão.
- **CAD avançado B:** o CATProduct guarda instâncias com seus caminhos; PLM avançado/VPM mantém a estrutura num banco central [V: EDS; benchmark do próprio repositório].
- **CAD avançado A:** BOM da montagem com configurações; PDM mantém "Where used" e estados [C].
- **CAD avançado C:** BOM estruturada, nº de itens fixos e Vault para o ciclo de vida [C].
- **Fenkra hoje:** `Assembly.items: Vec<ItemNo>` fixa números de item por arquivo (`montagem/lib.rs:355-357`). `bom.rs` (228 linhas) gera a lista. O projeto exporta só uma lista plana de documentos (`fenkra-project/src/lib.rs:267-275`), sem estrutura pai→filho nem revisão por componente. RF-GPR-002/003 (itens sem 3D) e "onde é usado" estão sem implementação.
- **Melhor prática:** estrutura persistida no índice (pai, filho, qtd, revisão-alvo), reconstruída ao salvar a montagem; "itens lógicos" sem modelo; BOM exportável (CSV agora, XLSX depois). **Testes:** mudar a quantidade de um componente atualiza a BOM e o desenho; "onde é usado" lista as montagens corretas.

### 11) Projeto, caminhos de busca e resolução de arquivos
**(CAD avançado B: CATReffilesPath / Desk / CAD avançado A: File Locations + Find References / CAD avançado C: `.ipj` / Fenkra: pasta com índice)**
- **O que faz:** diz onde estão os arquivos, os templates, os estilos e as bibliotecas.
- **CAD avançado B:** resolução por caminho de arquivos e *Desk*; no erro, *Find* manual [V: EDS].
- **CAD avançado A:** *File Locations* tem tipos de pasta com adicionar/remover/ordem; os templates ficam, por padrão, em `ProgramData\CAD avançado A\CAD avançado A 20xx\templates` e podem ficar em qualquer pasta [V: ajuda e Javelin]. Uma instalação nova herda os locais personalizados da anterior [V].
- **CAD avançado C:** o `.ipj` define workspace, grupos de trabalho, bibliotecas, *Styles Library*, *Templates* e *Content Center*; "Default" no projeto delega às *Application Options*, e um caminho explícito as ignora [V: Arkance e API]. Há tipos de projeto (único, Vault, semi-isolado, compartilhado); os legados vêm ocultos e o fornecedor de CAD avançado recomenda Vault para multiusuário [V: ajuda 2026]. Um projeto pode **incluir** outro, e o tipo do incluído prevalece [V]. Uso da biblioteca de estilos: leitura/escrita ou só leitura [V].
- **Fenkra hoje:** o "projeto" é a pasta com índice (`Project::create`, `lib.rs:138`). Não há caminhos de busca, templates, estilos, bibliotecas por projeto (a biblioteca é interna: `solidos/biblioteca.rs`, e `biblioteca/` no repositório). Componentes relativos à pasta da montagem (`load.rs:27-33`); peça fora da pasta fica com caminho absoluto, que quebra em outra máquina.
- **Melhor prática:** um arquivo de projeto (`projeto.fenkra.json`) declarando `search_paths`, `library_layers`, `templates`, `standards` e `lock_policy`, com herança "programa → empresa → projeto". Resolver por `uuid` e usar o caminho como dica. **Testes:** copiar a pasta para outro local e abrir a montagem sem erros; arquivo ausente lista o nome e oferece "localizar".

### 12) Revisões, ciclo de vida e cofre (PDM/PLM)
**(CAD avançado B: PLM avançado VPM / CAD avançado A: PDM Standard/Professional / CAD avançado C: Vault / Fenkra: `fenkra-project` + B25)**
- **O que faz:** versões, revisões congeladas, estados e controle de acesso.
- **CAD avançado B:** VPM guarda produto, revisões, maturidade e estrutura [V: benchmark do repositório].
- **CAD avançado A:** o PDM Standard permite **um** fluxo com até 10 estados; o Professional, vários fluxos e estados ilimitados [V: Javelin]. A tarefa *Design Checker* valida documentos contra arquivos `.swstd` (Professional) [V: ajuda 2018, via busca].
- **CAD avançado C:** o Vault faz check-in/out, esquemas de revisão e ciclo de vida [C].
- **Fenkra hoje:** versões imutáveis endereçadas por hash e `release()` (`lib.rs:205-251`); a revisão congelada abre como somente leitura (`app.rs:6760-6763`). Falta: liberar **montagem inteira** (RF-GPR-013), comparar revisões, check-out/lock, "onde é usado", estados configuráveis e GC de versões (ADR 0009: "prever limpeza"). O PN é `FK-0001` com contador único (`lib.rs:169`); só há detecção de duplicata **dentro do projeto**. O arquivo de versão é gravado sem fsync nem tmp+rename (`lib.rs:210-214`).
- **Melhor prática:** SQLite WAL como índice, e o cofre por hash continua; *lock* por arquivo em pasta compartilhada (`lock.json`, B25); liberação em conjunto cria revisões atômicas; um estado só pode avançar se as propriedades obrigatórias existirem. **Testes:** liberar montagem congela todos os componentes numa só transação; dois processos pedindo lock → o segundo recebe "em uso por X".

### 13) Salvar, Salvar tudo, Salvar como, Pack and Go
**(CAD avançado B: Save/Save All/Save Management / CAD avançado A: Save All, Pack and Go / CAD avançado C: Save All, Pack and Go / Fenkra: três botões Salvar)**
- **O que faz:** persiste o trabalho sem deixar nada modificado escondido, e empacota para terceiros.
- **CAD avançado B:** *Save Management* mostra o estado de cada documento e permite redefinir caminho antes de gravar [C].
- **CAD avançado A:** *Find References* abre o *Pack and Go* pela opção Copy Files; relatos de arquivos referenciados que ficam de fora, de extras que entram, e de referências externas que não são atualizadas na cópia [V: fóruns CAD avançado A].
- **CAD avançado C:** o *Pack and Go* também depende do projeto ativo, e via iLogic relatam que gera só a montagem sem as peças [V: fórum fornecedor de CAD avançado, anedótico].
- **Fenkra hoje:** `save()` bifurca em projeto × arquivo (`app.rs:800-829`); a escrita é `std::fs::write` direto (`app.rs:819`). Na montagem, "Gravar a montagem; as peças ficam nos arquivos delas" (`toolbar.rs:224`). O salvamento "automático" é só ao trocar de documento do projeto (`app.rs:1153-1155`). Sem Salvar tudo, Salvar como no projeto nem Empacotar (B56).
- **Melhor prática:** `salvar(doc)` sempre atômico (tmp → fsync → rename), com cópia `.bak` opcional; Salvar tudo lista arquivo + motivo; Empacotar percorre o grafo de referências **real** (não só o que a UI mostra) e reescreve os caminhos para relativos. **Testes:** matar o processo na metade da escrita → o arquivo anterior permanece válido; o pacote abre em pasta vazia sem avisos.

### 14) Templates, estilos e normas da empresa
**(CAD avançado B: Standards (ISO/ASME/JIS…) + Catalogs / CAD avançado A: Document Templates + Drafting standard / CAD avançado C: Templates + Styles Library + Standards / Fenkra: normas no desenho)**
- **O que faz:** pontos de partida padronizados (unidades, material, carimbo, normas, estilos).
- **CAD avançado B:** o administrador define padrões de desenho e configurações padrão no modo Administração [V: EDS e 3DS, via busca].
- **CAD avançado A:** *Default Templates* são usados ao criar arquivos no modo Novice e peças virtuais; os modelos podem ficar em qualquer pasta [V: Javelin].
- **CAD avançado C:** pasta de *Templates* e *Styles Library* por projeto; estilos podem ser lidos de uma biblioteca compartilhada [V: ajuda 2026].
- **Fenkra hoje:** o desenho tem folhas, carimbo e normas (`fenkra-desenho`, checklist "Folhas, carimbo, normas"). Não há templates de peça/montagem/desenho, nem "novo a partir de modelo", nem biblioteca de estilos. A tela inicial só oferece exemplos (`examples.rs`). B30 trata da herança norma → folha → vista, B42 das camadas da biblioteca. RF-PAR-023 e templates ficam sem dono.
- **Melhor prática:** template = documento vazio com unidades, material, propriedades obrigatórias, parâmetros-padrão, normas e estilo; "Novo" pergunta o template do projeto. Estilos nomeados (texto, cota, linha, hachura) vivem na camada empresa e são copiados ao documento com **cache** (como os links). **Testes:** criar a partir do template herda unidade/material; alterar o estilo da empresa não muda desenho liberado.

### 15) Configurações de usuário e de empresa (CATSettings, System Options)
**(CAD avançado B: CATSettings user/admin + locks / CAD avançado A: System Options × Document Properties / CAD avançado C: Application Options × Document Settings / Fenkra: `config.json`)**
- **O que faz:** separa preferência do usuário, padrão da empresa e propriedade do documento.
- **CAD avançado B:** as personalizações ficam em *CATSettings*; há modo Usuário e modo Administração, e no último cada opção em *Tools > Options* mostra um cadeado que trava o valor; recomenda-se distribuir pelo `CATReferenceSettingPath` e não pelo `CATUserSettingPath`, e um valor não travado pode ser sobrescrito pelo usuário. Há ferramentas de importar/exportar e *dump* [V: 3DS, EDS, IBM e comunidade 3DS, via busca; fonte oficial completa é um PDF não lido].
- **CAD avançado A:** opções de sistema (por máquina/usuário) × propriedades do documento (dentro do arquivo); *Backup/Recover* nas opções de sistema [V].
- **CAD avançado C:** *Application Options* × *Document Settings*, e o `.ipj` sobrepõe pastas [V].
- **Fenkra hoje:** `Settings` único em `%APPDATA%\Fenkra\config.json` (`settings.rs:458-461`); sem `APPDATA` (Linux/macOS) devolve `None` e nada persiste. Tema, doca de grupos, atalhos (8 *presets*: Fenkra, CAD avançado A, CAD avançado B, FreeCAD, CAD avançado D, Blender, CAD avançado E, CAD avançado C, `keymap.rs:168-182`), recentes e bancada. Não há camada de empresa nem trava. O eframe é compilado com `persistence`, mas o app usa seu próprio arquivo.
- **Melhor prática:** 4 camadas (programa < empresa < projeto < usuário), cada chave com `locked: bool` na camada que a define; dir. de config por plataforma (XDG/Known Folders); botão "Restaurar padrão da empresa". **Testes:** chave travada pela empresa não é alterável na UI; no Linux o arquivo vai para `~/.config/fenkra`.

### 16) Bancadas, módulos e licenciamento
**(CAD avançado B: workbenches + configurações de licença DSLS / CAD avançado A: add-ins e edições / CAD avançado C: Add-ins Manager / Fenkra: enum `Bench`)**
- **O que faz:** liga/desliga conjuntos de comandos por perfil de licença ou de usuário.
- **CAD avançado B:** a disponibilidade dos *workbenches* depende das licenças do usuário; componentes são empacotados em *configurations* e *products*, e o PLM Express oferece configurações por solução [V: CATI, Technia]. O servidor de licenças (DSLS) precisa estar instalado antes; a aba *Usage Per Feature* mostra o uso dos assentos [V: CATI, Hawk Ridge]. Licenciamento concorrente descrito num *white paper* 3DS não lido por inteiro [V parcial].
- **CAD avançado A / CAD avançado C:** edições (Standard/Pro/Premium) e *add-ins* carregáveis [C].
- **Fenkra hoje:** `Bench` (6 valores) e `Group` (29) fixos no código (`settings.rs`). Há "Personalizar comandos" (`app.rs` `customize_open`) para mover grupos de doca. Como é livre e aberto, **não há licenciamento comercial**; o equivalente útil é "perfil de bancadas" (ex.: só chapa) para reduzir ruído. RNF-015 (módulos como plugin e desativáveis) está `[~]`.
- **Melhor prática:** manter o produto sem licença, mas dar **perfis** (mostrar/ocultar bancadas) e um registro de comandos (`id`, bancada, ícone, atalho, tooltip, tradução). **Testes:** um perfil "chapa" esconde sólidos/superfícies; todo comando registrado tem ícone, tooltip e tradução (B60).

### 17) Personalização da interface e atalhos
**(CAD avançado B: Tools > Customize / CAD avançado A: Customize / CAD avançado C: Customize + Ribbon / Fenkra: `Personalizar comandos`, keymap)**
- **O que faz:** adapta barras, menus, atalhos e *presets* de outros CADs.
- **Fenkra hoje:** atalhos configuráveis com 8 *presets* e um teste (`keymap.rs:322`); grupos arrastáveis entre docas (`app.rs` `DockDrag`); paleta de comandos (`palette.rs`, 48 linhas); temas (`theme.rs`, `FENKRA_DEMO_TEMA`). A tela de atalhos está implementada, mas não há exportar/importar perfil nem conflito entre *presets* detectado em teste.
- **Melhor prática:** perfil de interface exportável (JSON) compartilhável na camada empresa; detecção de conflito de atalhos; paleta com sinônimos (nomes CAD avançado B/CAD avançado A/CAD avançado C) para o usuário migrar. **Testes:** dois comandos com o mesmo atalho geram aviso; importar um perfil restaura docas.

### 18) API, macros, automação e features definidas pelo usuário
**(CAD avançado B: Automation (VBA/VBScript via COM), CAA C++, Power Copy/UDF / CAD avançado A: API COM + macros + add-ins / CAD avançado C: API COM, VBA, add-ins, iLogic / CAD avançado E: FeatureScript / Fenkra: macros em texto)**
- **O que faz:** automatiza, estende e empacota conhecimento.
- **CAD avançado B:** a API é COM (`Application` → `Documents`), usada por VBScript/VBA [V: GoEngineer]; a CAA em C++ é a via oficial para extensões profundas [C]. UDF/PowerCopy empacotam geometria+regras [V: parcial].
- **CAD avançado A:** API COM com macros (.swp), add-ins .NET e eventos; PDM tem API própria e tarefas customizadas [V: parcial via busca].
- **CAD avançado C:** quatro caminhos: **VBA** (embutido, em processo), **Add-Ins** (DLL carregada na abertura, podem estender a UI e reagir a eventos, não em VBA), **EXE** externo e **Apprentice Server** (leitura sem UI, grátis); objetos têm propriedades, métodos e eventos; Python/Java também são possíveis [V: ajuda API 2018]. O iLogic roda regras sobre a API e atualiza iProperties ou substitui componentes automaticamente [V: AU e blog fornecedor de CAD avançado].
- **Outros:** CAD avançado E permite *custom features* em FeatureScript, versionadas e publicáveis numa biblioteca; cada feature referencia uma versão da biblioteca padrão [V: página CAD avançado E].
- **Fenkra hoje:** `macros.rs` (342 linhas) é uma DSL de uma instrução por linha (`param`, `sketch`, `extrusao`, `furo`, `filete`, `padrao`, `para i de 1 a N`) com erro por número de linha (`macros.rs:1-63`). O núcleo é headless (`fenkra-core` só depende de serde/thiserror; o kernel tem *fallback* sem OCCT), mas não há binário CLI nem API estável. A API Python (B39, P3) é a única previsão. Sem eventos, sem gravador de macro, sem UDF.
- **Melhor prática:** (1) um binário `fenkra-cli` (abrir, recomputar, listar parâmetros/BOM, aplicar macro, exportar), que dá CI, lote e *golden tests* de graça; (2) API de biblioteca Rust com SemVer; (3) depois Python/WASM sobre a mesma API; (4) UDF = "grupo de comandos parametrizado" salvo e reinserido. **Testes:** a mesma macro gera o mesmo volume via CLI e via UI; uma macro com erro informa a linha.

### 19) Desempenho e montagens grandes
**(CAD avançado B: Visualization mode / cache / CAD avançado A: Lightweight, Large Design Review, Large Assembly Mode, SpeedPak / CAD avançado C: Express mode, Level of Detail / Fenkra: modos visualizar/contexto/editar)**
- **O que faz:** abrir e navegar modelos grandes sem carregar o histórico.
- **CAD avançado B:** o uso do cache de documentos altera a ativação de referências externas [V].
- **CAD avançado A:** *Lightweight* carrega só um subconjunto dos dados e busca o resto sob demanda; *Fully Resolved* carrega tudo. *Large Design Review* abre rápido, mantém zoom/pan/medir/isolar/passeio, e restringe a edição. *Large Assembly Mode* é um conjunto de opções que pode disparar por contagem de componentes. *SpeedPak* é uma configuração simplificada [V: CATI e blog CAD avançado A]. *Performance Evaluation* informa componentes resolvidos [V].
- **CAD avançado C:** *Express mode* e *Level of Detail* [C].
- **Fenkra hoje:** `PartCache::get(full, with_shape)` guarda a forma só no modo editável (`load.rs:36-52`), com chave por caminho e `mtime`. `app.rs:6622` clona o `Document` inteiro **a cada quadro** e compara no fim (`app.rs:6759`); o desfazer guarda até 200 clones (`app.rs:6767-6770`; 100 na montagem, `asm_ui.rs:275`). Não há *benchmark* no repositório para RNF-003/004/016/017, então esses números não estão verificados.
- **Melhor prática:** (1) *dirty flag* por edição em vez de comparar documentos por quadro; (2) desfazer por comando (delta) ou com limite de memória, não por contagem; (3) orçamento de desempenho como teste (abrir montagem de 1.000 componentes < 10 s). **Testes:** montagem sintética de 1.000 instâncias da mesma peça abre em modo visualização em < 10 s; o quadro ocioso não clona o documento.

### 20) Robustez: desfazer, autorrecuperação, backup
**(CAD avançado B: Backup/Save settings / CAD avançado A: AutoRecover + Backup / CAD avançado C: autosave / Fenkra: desfazer em memória)**
- **O que faz:** garante que uma falha não perca trabalho.
- **CAD avançado A:** *Auto-Recover* salva em intervalo definido, **só depois do primeiro salvamento** do documento; *Backup* (desligado por padrão) guarda a versão anterior à última gravação; os recuperáveis aparecem no painel de Recuperação; não funciona com caminho de rede [V: Javelin e Solid Solutions, via busca].
- **CAD avançado B / CAD avançado C:** têm opções equivalentes [C].
- **Fenkra hoje:** nada periódico. O "salvamento automático" é só ao trocar de documento no projeto (`app.rs:1153-1155`). Um *panic* em recompute ou na UI perde o trabalho aberto. Escrita não atômica (`app.rs:819`). O checklist marca RNF-007 como implementado.
- **Melhor prática:** *autosave* a cada N minutos em `<dados>/recuperacao/<uuid>.fkp` (inclusive de documento nunca salvo), recuperação na abertura, *panic hook* que grava um dump, escrita atômica em tudo. **Testes:** forçar `panic` e reabrir oferece recuperar; o arquivo recuperado é igual ao último estado.

### 21) Formatos de arquivo e interoperabilidade
**(CAD avançado B/CAD avançado A/CAD avançado C: STEP AP203/214/242, IGES, nativo de outros por tradutores / Fenkra: STEP/STL de saída)**
- **O que faz:** troca de dados com outros CADs e com fabricação.
- **Plataformas:** os três leem/gravam STEP e IGES; os formatos nativos do concorrente exigem licença ou tradutor [C]. Esta pesquisa não confirmou em fonte pública o detalhe de AP242/PMI.
- **Fenkra hoje:** exportação de peça só **STEP e STL**: o menu tem "Exportar STEP…" e "Exportar STL…" (`app.rs:2528-2535`), e `export()` decide por extensão: `.stl` → STL, **qualquer outra → STEP** (`solidos/lib.rs:5160`). A importação de STEP existe no kernel (`lib.rs:699`) e na ponte (`ffi.rs:197`), mas **nenhum chamador** a usa [R: grep em todo o repositório]. Malha: STL/OBJ de entrada até 200 mil triângulos (`mesh_io.rs:11`). Desenho: PDF vetorial (escrito à mão), SVG e DXF R12 (`desenho/export.rs`). Não há IGES, BREP, 3MF, PLY nem XLSX no código, embora a visão (§7) e o checklist os listem. O RF-NUC-010/011 está marcado `[x]`, o que é incorreto.
- **Melhor prática:** funil de importação com **relatório**: unidade, escala, nº de sólidos, falhas, reparos (*healing*), PN/nome vindos do STEP; "corpo importado" como coletor de referência; exportação com diálogo de opções (AP214/AP242, unidade, só visíveis) e erro explícito se a extensão não for suportada. **Testes:** *round-trip* STEP (exportar → importar → volume/bbox iguais com tolerância); `.iges` sem suporte → erro claro, não STEP gravado como `.iges`.

### 22) Internacionalização, acessibilidade e unidades
- **O que faz:** idioma, leitura por todos e unidades.
- **Fenkra hoje:** `tr()` consulta um `HashMap` global cuja **chave é o texto em português** (`i18n.rs:24-31`); texto sem tradução cai no português **em silêncio**; 2.605 linhas de tabela; idioma é um `AtomicBool` global (`i18n.rs:8`), então só PT/EN. Há 1 teste em i18n. Nada sobre acessibilidade foi encontrado no código: não há escala de fonte global, modo de alto contraste ou navegação por teclado verificada [R: busca sem resultado]; não verifiquei se o egui/eframe expõe accesskit nesta versão (0.37) [C]. Unidades: só mm/° (ver ficha 7).
- **Melhor prática:** chaves estáveis (id) em vez do texto fonte; teste que lista textos sem tradução; escala de UI e tema de alto contraste; estado nunca só por cor (sufixos/ícones nos links e erros). **Testes:** `cargo test` falha se um comando registrado não tem tradução (casa com B60); o app funciona só com teclado nos diálogos principais.

### 23) Testes, integração contínua e distribuição
- **Fenkra hoje:** ≈151 `#[test]` (`fenkra-solidos/src/lib.rs` concentra 43), no padrão "montar o documento em código, recalcular e conferir status/volume/bbox" (`docs/desenvolvimento.md`). `exemplos_recalculam_sem_erro` cobre os exemplos da tela inicial. Não há `.github` (sem CI), nem matriz de plataforma, nem teste de integração do app (só variáveis `FENKRA_DEMO_*` para captura manual). O corpus (B53) está "a fazer". `release.ps1` gera um `.zip` Windows.
- **Melhor prática:** CI com `fmt`+`clippy`+`test` (kernel real e modo "sem kernel"), corpus de 16 peças, *golden tests* de volume/área/bbox e captura visual com tolerância; artefatos de release por plataforma. **Testes:** o corpus roda em cada commit; um PR que altera o volume de uma peça de referência falha.

### 24) Licenças e conformidade
- **Fenkra hoje:** `Cargo.toml:16` declara `LGPL-2.1-or-later`. Não há arquivo `LICENSE`, `THIRD_PARTY.md` (previsto em `reuso-opensource.md`) ou verificação de licenças em `git ls-files`. A ADR 0003 continua "proposta". O OCCT é LGPL-2.1 com exceção; `eframe/egui/wgpu` costumam ser MIT/Apache [C]. A pasta `biblioteca/` e os dados de fixadores vêm de fontes com níveis A/B/C/X (`docs/biblioteca/fontes-de-dados.md`); dados do FreeCAD Fasteners (GPLv2) só servem para conferência (ADR 0008).
- **Melhor prática:** `LICENSE` + `THIRD_PARTY.md` gerado (`cargo about`/`cargo deny`) + aviso de ligação dinâmica do OCCT no pacote + marca "fontes dos dados" em cada família. **Testes:** o CI falha se uma dependência nova tiver licença fora da lista permitida.

### 25) Fluxos ponta a ponta (peça→desenho, montagem→PLM, importação→fabricação)
- **CAD avançado B/CAD avançado A/CAD avançado C:** o desenho referencia a peça/montagem pelo caminho e atualiza ao abrir (CAD avançado B avisa "update needed" [V: EDS]); a liberação ocorre no cofre (VPM/PDM/Vault), com estados e revisão do desenho junto com o modelo [C].
- **Fenkra hoje:** *peça→desenho* funciona sem identidade estável (caminho em `Source`), com PDF/SVG/DXF; *montagem→PLM* é CSV; *importação→fabricação* depende de malha, com saída DXF planificado, lista de corte e STEP/STL.
- **Melhor prática:** o **grafo do projeto** (peça ← montagem ← desenho) como estrutura única; o aviso de "desenho desatualizado" ao liberar (ADR 0007); a revisão do desenho cita a revisão do modelo. **Testes:** liberar uma montagem com desenho desatualizado pergunta antes; o pacote do projeto traz peças, montagem, desenho e PDFs coerentes.

---

## 3. Requisitos levantados

**Prefixo:** `RF-MAC-nn` / `RNF-MAC-nn` (MAC = visão macro). **Estado:** feito / parcial / falta. **Backlog:** item existente (B..) ou proposto (MAC-.., seção 4).

### 3.1 Requisitos funcionais

| ID | Requisito | P | Estado | Critério de aceite | Backlog |
|---|---|---|---|---|---|
| RF-MAC-01 | Migração de formato: abrir arquivos de versões anteriores por cadeia de migrações; versão futura abre somente leitura com aviso | P0 | falta | fixtures `.fkp/.fka/.fkd` de cada alpha recomputam com volume igual; `format_version` maior não é sobrescrito | MAC-01 |
| RF-MAC-02 | Escrita atômica e verificação de integridade em todo arquivo (documento, versão, índice) | P0 | parcial (só índice) | injeção de falha no meio da escrita deixa o arquivo anterior legível; hash da versão confere ao abrir | MAC-02 |
| RF-MAC-03 | Cada documento declara tipo/papel (peça, chapa, estrutura, mestre, montagem, desenho); abrir escolhe a bancada | P1 | parcial | duplo clique em chapa abre na bancada Chapa sem inferir por `sheet`; mestre abre em modo de publicação | MAC-05 |
| RF-MAC-04 | Identidade estável (UUID) gravada no arquivo e usada por componentes, links e vistas; caminho só como dica | P0 | falta | renomear/mover arquivo na pasta do projeto não quebra montagem, link nem desenho | MAC-05 |
| RF-MAC-05 | Objeto em trabalho visível e barra de fim de peça; comando novo entra depois dele | P0 | falta | com o marcador no meio, criar filete insere ali; o IWO é único e sublinhado | B59 |
| RF-MAC-06 | Contêineres ordenado × não ordenado com regras de aninhamento e reordenar/auto-ordenar o não ordenado | P1 | parcial | reordenar pasta de geometria não muda volume; aninhamento inválido é recusado com motivo | B59 |
| RF-MAC-07 | Um componente ativo por vez na montagem; editar peça sem sair da montagem | P0 | falta | duplo clique ativa a peça com o resto esmaecido e selecionável; Voltar ao topo restaura | B54 |
| RF-MAC-08 | Sketch, planos, eixos e pontos no nível da montagem; esqueleto e "fazer peça a partir dele" | P0 | falta | peça-filha criada do esqueleto herda a origem e atualiza ao mover o esqueleto | B55 |
| RF-MAC-09 | Publicação com nome, tipo e papel; interface pública do documento; "usado por" antes de remover | P0 | parcial | remover publicação usada lista dependentes; renomear com alias mantém links | B55 |
| RF-MAC-10 | Estado do link na árvore (`->`, `->?`, `->*`, `->x`) e ações Travar, Destravar, Quebrar, Isolar | P0 | parcial | link travado não muda ao editar a origem; quebrado mantém cache e calcula | B54 |
| RF-MAC-11 | Política de atualização por link (automática, ao abrir, manual) | P1 | falta | link manual só muda após "Atualizar"; ao abrir mostra lista do que mudou | MAC-07 |
| RF-MAC-12 | Grafo de documentos acíclico, verificado ao criar link/componente | P1 | falta | A→B→A é bloqueado e a cadeia aparece no erro | MAC-07 |
| RF-MAC-13 | Localizar referências e reparar links em lote (por pasta, PN ou nome) | P1 | parcial | pasta movida: "localizar" reconecta todas as referências com um clique | B56, MAC-07 |
| RF-MAC-14 | Substituir componente religando por nome publicado e listando o que não casou | P1 | parcial | trocar parafuso M8→M10 mantém as restrições que casam; as demais ficam listadas | B55 |
| RF-MAC-15 | Estado por comando (ok/desatualizado/falhou/suprimido) e diagnóstico com Editar, Substituir, Isolar, Desativar, Reordenar | P0 | parcial | falha de um comando não invalida os independentes; cada ação do diálogo funciona | B59 |
| RF-MAC-16 | Atualização manual × automática por documento, com contagem de pendentes | P1 | parcial | modo manual não recalcula sozinho; o botão mostra o número de pendentes | B59 |
| RF-MAC-17 | Registro de atualização com tempo e motivo por comando | P2 | falta | `RecomputeResult` expõe `ms` por comando; painel ordena por tempo | MAC-20 |
| RF-MAC-18 | Unidades reais (mm, cm, m, pol) e tipos de parâmetro (texto, booleano, lista) | P1 | falta | mudar a unidade do documento não altera a geometria; parâmetro de lista valida opções | MAC-08 |
| RF-MAC-19 | Tabelas de configuração: variantes por linha com PN próprio | P2 | falta | 3 linhas geram 3 volumes esperados; a BOM escolhe a variante por instância | MAC-08 |
| RF-MAC-20 | Verificações de projeto (limites, regras simples) com aviso | P2 | falta | parâmetro fora do limite mostra aviso sem bloquear; liberação lista avisos | MAC-21 |
| RF-MAC-21 | Esquema de propriedades da empresa (campos tipados e obrigatórios) | P1 | falta | campo obrigatório vazio bloqueia "Liberar" | MAC-09 |
| RF-MAC-22 | Propriedades alimentam carimbo, notas e colunas da BOM | P1 | parcial (carimbo não verificado) | mudar o PN atualiza carimbo e BOM do desenho | MAC-09 |
| RF-MAC-23 | Estrutura de produto persistida (pai, filho, qtd, revisão-alvo) e "onde é usado" | P0 | parcial | trocar qtd atualiza BOM e desenho; "onde é usado" lista as montagens | B25, B50 |
| RF-MAC-24 | Itens lógicos sem modelo 3D na estrutura | P2 | falta | item "tinta" aparece na BOM sem arquivo | B25 |
| RF-MAC-25 | Arquivo de projeto com caminhos de busca, camadas de biblioteca, templates e normas | P1 | falta | copiar a pasta para outra máquina abre tudo sem erro | MAC-11 |
| RF-MAC-26 | Pacote do projeto (Pack and Go) com verificação de referências e caminhos relativos | P0 | falta | o pacote abre em pasta vazia sem avisos | B56 |
| RF-MAC-27 | Salvar como projeto / copiar com dependências, escolhendo copiar × reutilizar | P1 | falta | cópia mantém links internos e preserva os externos escolhidos | MAC-12 |
| RF-MAC-28 | Mover/renomear atualiza todos os referenciadores | P1 | parcial | renomear dentro do app atualiza `.fka`/`.fkd` abertos e fechados | MAC-05 |
| RF-MAC-29 | Índice SQLite + cofre por hash; migração do JSON | P1 | parcial | migração preserva versões e revisões; abrir 10 mil documentos < 1 s | B25 |
| RF-MAC-30 | Trava de edição em pasta compartilhada com detecção de conflito | P1 | falta | segundo processo recebe "em uso por X"; trava obsoleta expira | B25 |
| RF-MAC-31 | Liberação em conjunto e estados de ciclo de vida configuráveis | P1 | falta | liberar montagem congela todos os componentes numa transação | MAC-12 |
| RF-MAC-32 | Comparação visual de revisões | P2 | falta | A × B mostra volume acrescentado/removido em cores | MAC-12 |
| RF-MAC-33 | Regras de PN com campos e ambientes | P1 | parcial | `AERO-[NORMA]-[CODIGO]-[REV]` gera `AERO-NAS-620-3-B` | B47 |
| RF-MAC-34 | Salvar unificado, Salvar tudo com motivo, Salvar como | P0 | falta | a lista mostra editado/novo/dependência; Ctrl+S salva só o ativo | B56 |
| RF-MAC-35 | Importar STEP/IGES com relatório (unidade, escala, falhas, reparo) | P0 | falta | *round-trip* STEP mantém volume/bbox; falha lista as faces problemáticas | MAC-06 |
| RF-MAC-36 | Exportar IGES, BREP, 3MF, OBJ; recusar extensão não suportada | P1 | falta | `.iges` sem suporte → erro claro; 3MF abre em outro programa | MAC-06 |
| RF-MAC-37 | Importar/exportar CSV/XLSX de parâmetros e BOM | P2 | parcial (CSV) | XLSX abre no Excel; CSV usa separador configurável | MAC-08 |
| RF-MAC-38 | Importar DXF/SVG para sketch e desenho | P2 | parcial | DXF R12 importa linhas/arcos/círculos com escala correta | MAC-06 |
| RF-MAC-39 | Templates de peça, montagem e desenho | P1 | falta | "Novo" lista os templates do projeto; o documento nasce com unidade/material | MAC-09 |
| RF-MAC-40 | Normas e estilos da empresa em camadas, com cache no documento | P1 | parcial | alterar o estilo da empresa não muda desenho liberado | B30, B42, MAC-10 |
| RF-MAC-41 | Configurações em camadas (programa < empresa < projeto < usuário) com chaves travadas | P1 | falta | chave travada é somente leitura na UI | MAC-10 |
| RF-MAC-42 | Desenho: estado de atualização por vista e aviso ao liberar | P1 | parcial | vista desatualizada mostra marca; liberar pergunta | B30 |
| RF-MAC-43 | Bancadas ativáveis por perfil; módulos como plugins | P2 | parcial | perfil "chapa" oculta sólidos e superfícies | MAC-18 |
| RF-MAC-44 | Registro de comandos com ID estável (atalho, ícone, tooltip, tradução) | P2 | parcial | teste falha se faltar ícone, tooltip ou tradução | B60 |
| RF-MAC-45 | CLI/API headless estável: abrir, recomputar, parâmetros, BOM, macro, exportar | P1 | falta | `fenkra-cli recompute x.fkp --json` devolve volume e status | MAC-13 |
| RF-MAC-46 | API Python sobre a API estável | P3 | falta | script abre peça, muda parâmetro e exporta STEP | B39 |
| RF-MAC-47 | Macros: gravador, eventos (antes de salvar/ao atualizar) | P2 | parcial | gravar 5 ações gera macro que repete o resultado | MAC-13 |
| RF-MAC-48 | Features definidas pelo usuário (grupo de comandos parametrizado) | P3 | falta | salvar "furação 4x" e reinserir com outros valores | MAC-19 |
| RF-MAC-49 | Modos de carga por componente (visualizar leve × editar) | P1 | parcial | modo visualização não carrega histórico nem forma | B54, MAC-15 |
| RF-MAC-50 | Camadas de biblioteca (pública/empresa/usuário) com versão fixada | P1 | parcial | projeto abre com a versão da biblioteca que usou | B42 |

### 3.2 Requisitos não funcionais

| ID | Requisito | P | Estado | Critério de aceite | Backlog |
|---|---|---|---|---|---|
| RNF-MAC-01 | A UI nunca bloqueia: nenhum recompute na thread da UI (inclui carga de componentes da montagem) | P0 | parcial (peça feita; montagem não verificada, `load.rs:47`) | abrir montagem de 200 peças: nenhum quadro > 100 ms | MAC-15 |
| RNF-MAC-02 | Recompute incremental, inclusive entre documentos ligados | P1 | parcial (só dentro da peça) | alterar uma cota da peça-mestre recalcula só as dependentes | MAC-07 |
| RNF-MAC-03 | Viewport ≥ 60 fps com 1 M triângulos | P1 | não medido | *benchmark* automatizado registra fps por modelo | MAC-15 |
| RNF-MAC-04 | Montagem de 1.000 componentes abre em modo visualização < 10 s | P1 | não medido | teste sintético passa em CI | MAC-15 |
| RNF-MAC-05 | Memória do desfazer limitada; sem clonar o documento por quadro | P1 | falta (`app.rs:6622`, `6767`) | quadro ocioso sem alocação do documento; 200 desfazeres < N MB | MAC-16 |
| RNF-MAC-06 | Autorrecuperação periódica e recuperação ao abrir | P0 | falta | `panic` forçado → reabrir oferece recuperar o último estado | MAC-02 |
| RNF-MAC-07 | Linux x86_64 suportado (build, config XDG, release) | P1 | falta | CI constrói e roda testes no Linux; config em `~/.config/fenkra` | MAC-14 |
| RNF-MAC-08 | CI em cada commit: fmt, clippy, test, corpus | P0 | falta | PR falha se um teste ou o corpus falhar | MAC-03 |
| RNF-MAC-09 | Corpus de peças e cobertura feature → teste | P0 | falta | script lista features sem teste e falha acima do limite | B53 |
| RNF-MAC-10 | Determinismo: mesmo arquivo → mesmos números em plataformas e *builds* | P1 | falta | volume/área/bbox dentro de tolerância em Windows e Linux | MAC-03 |
| RNF-MAC-11 | Licenças e procedência: `LICENSE`, `THIRD_PARTY.md`, verificação no CI | P0 | falta | `cargo deny` passa; arquivos presentes no pacote | MAC-04 |
| RNF-MAC-12 | i18n completa e verificável (sem texto sem tradução) | P1 | parcial | teste lista textos sem tradução e falha | MAC-17 |
| RNF-MAC-13 | Acessibilidade: escala de UI, alto contraste, estado não só por cor, teclado | P2 | falta | tema de alto contraste e escala 150% sem corte; diálogos navegáveis | MAC-17 |
| RNF-MAC-14 | Offline e privacidade: nenhuma conexão de saída por padrão | P2 | feito (sem dependência de rede) | app roda sem rede; lista de dependências sem cliente HTTP | — |
| RNF-MAC-15 | Log de sessão e relatório de falha local (nada enviado) | P1 | falta | falha gera arquivo com versão, passos e pilha | MAC-20 |
| RNF-MAC-16 | Parsers de entrada com limites e testes de fuzz | P2 | parcial (200 mil triângulos) | arquivo corrompido nunca derruba o app | MAC-23 |
| RNF-MAC-17 | API pública com SemVer e documentação | P2 | falta | `cargo semver-checks` no CI | MAC-13 |
| RNF-MAC-18 | Limpeza de versões não marcadas como revisão | P2 | falta | GC mantém revisões e as N últimas versões | B25, MAC-12 |
| RNF-MAC-19 | Inicialização < 3 s | P2 | não medido | medição registrada por *release* | MAC-15 |
| RNF-MAC-20 | Distribuição por plataforma (portátil/instalador) com versão no arquivo | P2 | parcial (`release.ps1`) | artefatos Windows e Linux no CI; `app_version` gravada no documento | MAC-14 |

### 3.3 Matriz de rastreabilidade (requisito → item de backlog)

**Cobertos por itens existentes** (item → requisitos):

| Item | Requisitos |
|---|---|
| B25 | RF-MAC-23, 24, 29, 30; RNF-MAC-18 |
| B30 | RF-MAC-40, 42 |
| B39 | RF-MAC-46 |
| B42 | RF-MAC-40, 50 |
| B47 | RF-MAC-33 |
| B50 | RF-MAC-23 |
| B53 | RNF-MAC-09 |
| B54 | RF-MAC-07, 10, 49 |
| B55 | RF-MAC-08, 09, 14 |
| B56 | RF-MAC-13 (parte), 26, 34 |
| B59 | RF-MAC-05, 06, 15, 16 |
| B60 | RF-MAC-44 |

**Sem cobertura em nenhum item existente** (lacunas de requisito): RF-MAC-01, 02, 03, 04, 11, 12, 17, 18, 19, 20, 21, 22, 25, 27, 28, 31, 32, 35, 36, 37, 38, 39, 41, 43, 45, 47, 48; RNF-MAC-01 a 08, 10 a 13, 15 a 17, 19, 20. Esses requisitos são atendidos pelos itens **MAC-01…MAC-23** abaixo. Itens existentes que atendem a parte do requisito mas deixam lacuna: B56 (reparo em lote), B25 (comparação de revisões), B54 (modos de carga).

---

## 4. Itens de backlog propostos

Tamanho: P ≈ ≤ 100 k tokens, M ≈ 100–300 k, G ≈ > 300 k. Prioridade segue P0–P3 do backlog.

| ID | Título | O que muda | Prio. | Tam. | Dependências |
|---|---|---|---|---|---|
| MAC-01 | Migração e versionamento de formato | `format_version` por tipo; cadeia `migrate(vN→vN+1)`; versão futura em leitura; fixtures de cada alpha; `app_version` gravada | P0 | M | — (fazer antes de B54/B55, que mudam `Assembly`) |
| MAC-02 | Escrita atômica, integridade e autorrecuperação | tmp+fsync+rename em todo `write`; hash verificado; autosave periódico em pasta de recuperação; recuperação ao abrir; *panic hook* | P0 | M | MAC-01 |
| MAC-03 | CI e determinismo entre plataformas | GitHub Actions (fmt, clippy, test com e sem OCCT, corpus); tolerância de volume/bbox; artefatos | P0 | M | B53 |
| MAC-04 | Licenças e procedência | `LICENSE`, `THIRD_PARTY.md`, `cargo deny`, ADR 0003 para "aceita", aviso do OCCT no pacote | P0 | P | — |
| MAC-05 | Identidade estável de documento e tipo declarado | `uuid` e `kind` no arquivo; componentes/links/vistas por `uuid` com caminho como dica; renomear/mover atualiza referenciadores | P0 | G | MAC-01; alimenta B54, B55, B25 |
| MAC-06 | Interoperabilidade 3D real | ligar `import_step`; IGES; BREP; 3MF/OBJ de saída; relatório de importação; erro para extensão não suportada; corrigir o `export()` que grava STEP em qualquer extensão; DXF/SVG para sketch | P0 | G | — |
| MAC-07 | Ciclo de vida dos links | política por link; ciclo entre documentos; localizar/reparar em lote; aviso "publicação removida"; recompute incremental entre documentos | P1 | M | MAC-05, B55 |
| MAC-08 | Parâmetros com unidades, tipos e configurações | `Unit` real (mm/cm/m/pol); tipos texto/booleano/lista; tabela de configuração por linha; CSV/XLSX | P1 | G | MAC-01 |
| MAC-09 | Templates e esquema de propriedades da empresa | "Novo a partir de modelo"; propriedades tipadas e obrigatórias; ligação com carimbo/BOM | P1 | M | MAC-10 |
| MAC-10 | Configurações em camadas e travadas | camadas programa/empresa/projeto/usuário com `locked`; dir. de config por plataforma; exportar/importar perfil | P1 | M | MAC-14 (config dir) |
| MAC-11 | Arquivo de projeto com caminhos e camadas | `search_paths`, camadas de biblioteca, templates, normas; herança por inclusão | P1 | M | MAC-10, B25 |
| MAC-12 | Liberação em conjunto, comparação e salvar como projeto | revisões atômicas de montagem; estados configuráveis; comparar A × B; copiar projeto com dependências; GC de versões | P1 | G | B25, MAC-05 |
| MAC-13 | CLI e API headless estável | `fenkra-cli` (abrir, recomputar, parâmetros, BOM, macro, exportar); SemVer; eventos e gravador de macro | P1 | M | MAC-01; precursor de B39 |
| MAC-14 | Plataforma Linux e distribuição | config XDG; build via vcpkg/gerenciador; `release.sh`; CI Linux | P1 | M | MAC-03 |
| MAC-15 | Orçamento de desempenho | *benchmarks* de viewport, abertura de 1.000 componentes, partida; recompute de montagem fora da UI | P1 | M | B53 |
| MAC-16 | Desfazer sem clonar por quadro | *dirty flag*; desfazer por delta/limite de memória; sem `doc.clone()` por quadro | P1 | M | — |
| MAC-17 | i18n verificável e acessibilidade | chaves estáveis ou teste de textos sem tradução; escala de UI; alto contraste; estado não só por cor | P2 | M | B60 |
| MAC-18 | Bancadas como perfis e registro de comandos | perfil de bancadas; ID de comando; ativar/desativar módulos | P2 | G | B60 |
| MAC-19 | Features definidas pelo usuário | salvar/reinserir grupo de comandos parametrizado (estilo UDF/PowerCopy/FeatureScript) | P3 | G | MAC-13, B59 |
| MAC-20 | Registro de atualização e relatório de falha | tempo/motivo por comando; log de sessão; relatório local | P1 | P | — |
| MAC-21 | Verificações de projeto | limites e regras de parâmetro; avisos na liberação | P2 | M | MAC-08 |
| MAC-23 | Segurança de entrada | limites e fuzz nos leitores de arquivo (STL/OBJ/DXF/JSON) | P2 | M | MAC-06 |

*(MAC-22 não existe: numeração reservada; o item "Itens lógicos sem modelo" foi incorporado ao B25.)*

**Ordem recomendada:** MAC-04 e MAC-01 primeiro, por serem baratos e destravarem B54/B55. Depois MAC-02, MAC-03 e MAC-05, que são a base da onda 0. MAC-06 e MAC-16 em paralelo. O restante vai para as ondas 2 e 3.

---

## 5. Fontes

**CAD avançado B**
- (fonte em procedencia.md) (contêineres, IWO, reordenação) — lida
- https://edstechnologies.com/2020/12/ (tipos de link, Edit > Links, referência perdida) — lida
- (fonte em procedencia.md) (Isolate, publicação, sincronização) — lida
- (fonte em procedencia.md) (CATSettings; só a introdução) — lida
- (fonte em procedencia.md) (licenças; só a introdução) — lida
- https://www.eng-tips.com/faqs.cfm?fid=679 (IWO como pai; só trecho da busca)
- (fonte em procedencia.md) (só trecho da busca)
- https://www.ibm.com/support/pages/apar/HD91158 e https://www.ibm.com/support/pages/apar/HD08465 (settings e links; só trecho da busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) (CATSettings; só trecho da busca)
- (fonte em procedencia.md) e https://www.technia.nl/?p=4239 e (fonte em procedencia.md) (licenças/configurações; trechos da busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) e (fonte em procedencia.md) (Knowledge Advisor; trechos da busca)
- (fonte em procedencia.md) (automação; trecho da busca)

**CAD avançado A**
- (fonte em procedencia.md) e (fonte em procedencia.md) (File Locations, templates; trechos da busca — a página de ajuda não carregou o corpo)
- (fonte em procedencia.md) e (fonte em procedencia.md) e (fonte em procedencia.md) (estados de referência externa; trechos da busca)
- (fonte em procedencia.md) (Find References/References) — lida
- (fonte em procedencia.md) — lida; (fonte em procedencia.md) e (fonte em procedencia.md) (trechos da busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) (AutoRecover/Backup; trechos da busca)
- https://www.javelin-tech.com/blog/?p=16894 e (fonte em procedencia.md) (PDM Standard × Professional, Design Checker; trechos da busca)
- https://www.cati.com/?p=119882 e (fonte em procedencia.md) (Pack and Go; relatos de fórum)

**CAD avançado C / fornecedor de CAD avançado**
- (fonte em procedencia.md) (projetos) — lida
- (fonte em procedencia.md) (API) — lida
- (fonte em procedencia.md) (DesignProject; trecho da busca)
- (fonte em procedencia.md) (precedência .ipj × opções; só trecho da busca, página deu 403)
- (fonte em procedencia.md) (parâmetros; trecho da busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) (iLogic; trechos da busca)
- (fonte em procedencia.md) (iProperties × Vault; trecho da busca)
- (fonte em procedencia.md) e (fonte em procedencia.md) (Pack and Go, caminhos relativos; fóruns)

**Outros**
- (fonte em procedencia.md) e (fonte em procedencia.md) (FeatureScript; trechos da busca)

**Repositório [R]** (lido): `Cargo.toml`; `README.md`; `docs/produto/{visao,roadmap}.md`; `docs/requisitos/*`; `docs/arquitetura/adr/0001…0011`; `docs/arquitetura/reuso-opensource.md`; `docs/backlog/backlog.md`; `docs/modulos/{README,geral,pdm-plm}.md`; `docs/benchmark/{geral,pdm-plm}.md`; `docs/contextos/{plm-vpm,plm-pn-engine}.md`; `docs/desenvolvimento.md`; código em `crates/*/src` (referências `arquivo:linha` nas fichas).

---

**Entrega:** relatório completo acima. **Arquivos alterados:** nenhum (nem plano). **Verificações feitas:** leitura de docs e código, `grep` de usos (`import_step`, `LICENSE`, `.github`, `xlsx`), contagem de `#[test]` (≈151). Nada foi compilado nem executado. **Falhas:** várias páginas oficiais deram 403 ou só devolveram a navegação (ajuda do CAD avançado A, Arkance, Eng-Tips, Hawk Ridge); usei os trechos das buscas, marcados no texto. **Riscos:** parte da comparação com CAD avançado B/CAD avançado A/CAD avançado C vem de blogs de revendas, fóruns e notas de suporte, e as afirmações [C] precisam ser validadas antes de virarem regra. Não verifiquei se `PartCache::get` corre na thread da UI, nem o suporte a acessibilidade do egui 0.37. A contagem de testes é por linhas com `#[test]` nos arquivos de topo de cada crate.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra são (1) formato sem migração e documentos/links por caminho em vez de identidade estável, (2) interoperabilidade 3D que o checklist dá como feita mas só exporta STEP/STL (a importação de STEP existe no kernel e ninguém a chama), (3) plataforma de empresa ausente (templates, normas, configurações em camadas e travadas, projeto com caminhos de busca), (4) robustez e engenharia ausentes (escrita atômica, autorrecuperação, CI, Linux, LICENSE/THIRD_PARTY) e (5) extensibilidade limitada a macros próprias, sem API/CLI headless estável nem features definidas pelo usuário.
