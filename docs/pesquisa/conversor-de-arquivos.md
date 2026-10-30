# Conversor de arquivos: importação e exportação

> Pesquisa para o Fenkra (Rust + OpenCascade 8.0.1, egui em português). Escrita com palavras próprias.
> **[V]** = confirmado em fonte consultada (URL na seção 5, ou leitura do código). **[C]** = conhecimento geral, não confirmado nesta pesquisa.
> Limite de leitura: as páginas de ajuda do SolidWorks (help.solidworks.com) devolvem só o esqueleto do site ao WebFetch. Para o SolidWorks usei resumos de busca, blogs de revendas e a documentação de PDM. Esses itens estão marcados [V] pela fonte citada, mas sem o texto integral da página oficial.

---

## 1. Resumo e principais lacunas

**Resumo (10 linhas)**

1. Hoje o Fenkra grava **STEP e STL** de uma **peça** (todos os corpos em um compound, sem nomes, cores, PMI nem estrutura) e lê **STL/OBJ** como feature "Malha". O `import_step` existe no kernel e **nenhuma tela o chama**. O desenho exporta PDF, SVG e DXF R12.
2. Não existe importação de STEP/IGES, de DXF em sketch, de montagem neutra nem módulo conversor. O menu Arquivo tem dois botões soltos "Exportar STEP…" e "Exportar STL…".
3. O OpenCascade 8.0 (versão que o Fenkra usa) já lê e grava STEP, IGES, BREP, XBF, OBJ, STL, glTF e VRML, e grava PLY, pela camada `DE_Wrapper`. Com XDE ele leva cores, camadas, nomes, estrutura de montagem, propriedades de validação e GD&T/PMI semântico do AP242 [V]. O Fenkra usa só o `STEPControl_Writer` básico, que não carrega nada disso.
4. Fora do OCCT ficam 3MF, DXF/DWG, JT, Parasolid, ACIS, IFC, PRC/U3D e todos os nativos (CATPart, SLDPRT, IPT, Creo, NX). Esses exigem bibliotecas abertas (zip+XML, crate `dxf`, IfcOpenShell), um processo externo (LibreDWG, GPL) ou licenças comerciais (HOOPS Exchange, CAD Exchanger, Spatial 3D InterOp, Datakit, ODA).
5. A filosofia dos três CADs é a mesma: **importar é uma operação com relatório, reparo e destino explícito**, não um "abrir". Inventor e SolidWorks oferecem *Referência* (vínculo vivo com o original) contra *Converter* (cópia independente). CATIA concentra as opções em Tools > Options > Compatibility e tem o Batch Monitor para lote.
6. O reparo na importação é parte do fluxo. O SolidWorks tem o Import Diagnostics (Atribuir "curar tudo", reparar faces/lacunas), o Inventor tem o ambiente Reparar (Find/Heal Errors com tolerância permitida) e o CATIA tem healing nas ferramentas de superfície [V]. O Fenkra já tem `close_surface`, `sewn` e `ShapeFix` no kernel, mas não os aplica depois de ler um STEP.
7. O reconhecimento de features (FeatureWorks no SolidWorks, Synchronous Technology no NX/Solid Edge, Direct Edit no Inventor) devolve intenção de projeto ao corpo "burro". O Fenkra tem a base (furos, filetes, patamares no `fenkra-solidos`), mas nenhuma lógica de reconhecimento.
8. A integração com PDM é o maior diferencial possível. SolidWorks PDM detecta referências de arquivos 3D Interconnect no check-in e tem a tarefa Convert (STEP/PDF/DXF na liberação) [V]. O Inventor com Vault gerencia a árvore AnyCAD [V]. O Fenkra tem armazém por hash (ADR 0009) e o modelo `CAD_LINK` do `plm-vpm.md`, mas nenhum tipo de vínculo "importado de" ou "derivado de".
9. Proposta: um crate `fenkra-conversor` com interface por formato, opções serializáveis, relatório de conversão, fila de lote, execução em processo filho isolado e registro no projeto (original guardado por hash, vínculo, revisão). Na interface, um menu **Arquivo ▸ Importar** e outro **Arquivo ▸ Exportar** com diálogo único por formato, mais a janela "Conversão em lote".
10. Ordem sugerida: (a) corrigir o export atual e abrir caminho de importação STEP via XDE; (b) relatório + reparo; (c) montagem/PMI/cores; (d) malhas 3MF/glTF e DXF 2D; (e) lote + PDM; (f) nativos por biblioteca comercial opcional.

**Principais lacunas do Fenkra neste tema**

| # | Lacuna | Evidência |
|---|---|---|
| L1 | Sem **importação de STEP/IGES** na interface. `import_step` está órfão, sem healing, sem unidades, sem checar `TransferRoots`. | `kernel.cpp:1004-1011`, `kernel/src/lib.rs:699`, nenhuma chamada em `fenkra-app` |
| L2 | **Exportação pobre e com bug de extensão.** Qualquer sufixo ≠ `.stl` vira STEP. Nada de cores, nomes, PMI, estrutura; montagem não exporta. | `fenkra-solidos/src/lib.rs:5159-5161`, `app.rs:880-892`, `kernel.cpp:988-994` |
| L3 | **Sem módulo, relatório, opções por formato nem lote.** Dois botões fixos em Arquivo; nenhum diálogo de opções. | `app.rs:2528-2535` |
| L4 | **Sem reparo/diagnóstico de importação** nem reconhecimento de features. O healing existe só no fluxo de superfícies. | `hybrid.inc:43, 160, 304-335` |
| L5 | **Sem vínculo com PDM.** O arquivo importado não guarda o original, nem hash, nem estado "origem mudou"; a exportação não vira documento derivado de uma revisão. | `fenkra-project/src/lib.rs:72-95, 205-225`, `plm-vpm.md` (`CAD_LINK`) |
| L6 | **2D e malhas incompletos.** DXF só R12 gravado (sem leitura), sem DWG; malha lê só STL/OBJ com teto de 200 mil triângulos; sem 3MF/PLY/glTF; sem unidades. | `export.rs:51`, `mesh_io.rs:11-13` |

---

## 2. Fichas por comando/função

### Menu e diálogo Importar/Exportar (arquitetura da interface) (CATIA: File > Open / Insert > Existing Component / Save As + Tools > Options > Compatibility / SW: Abrir + Opções, Salvar como + Opções, Task Scheduler / Inventor: Abrir/Importar + Opções, Exportar / outros: Onshape Import/Translate / Fenkra: Arquivo ▸ Exportar STEP/STL)

- **O que faz:** é a porta de entrada e saída de dados. Escolhe formato, escopo (peça, montagem, seleção) e opções, e mostra o resultado.
- **CATIA V5 — no detalhe:**
  - Não há menu "Importar": abre-se STEP/IGES por File > Open com o filtro de tipo, ou insere-se um componente existente na montagem. A gravação é por File > Save As com o tipo (stp, igs, wrl, stl, dxf, cgr, 3dxml…) [V parcial: "stp (*.stp)" no Save As].
  - As **opções ficam globais**, não por operação: Tools > Options > General > Compatibility, com abas STEP, IGES etc. [V]. Em STEP, uma seção de Import (o que importar) e outra de Export, onde se escolhe o **Application Protocol** (AP203, AP214, 242 ed1 etc.) [V].
  - Há uma opção "3D Annotations" na seção geral para levar PMI [V]. Uma opção de IGES se chama "Join" e define a tolerância de junção na importação [V, via registro IBM].
  - **Licenças gatilham recursos:** ST1 (STEP Core: geometria e estrutura de AP203/AP214) e SXT (STEP estendido: PMI/FTA, composites, exportação de montagens aninhadas) [V]. Em tela, o item depende da licença ativada em Tools > Options > General > Shareable Products [V].
  - O modo `Batch Monitor` (BATCH-DXF-IGES-STEP) existe para lote (ver ficha de lote).
- **SolidWorks — no detalhe:**
  - Abrir: o diálogo de abertura tem o botão **Opções**, que muda por tipo de arquivo (para STEP/IGES/ACIS abre as opções de importação) [V]. Opções gerais em Ferramentas > Opções > Importar, com "Importar múltiplos corpos como peças" (IGES, STEP, UG, ACIS), "ignorar estrutura de montagem" (criar peça multicorpo) e "rodar o Import Diagnostics automaticamente" [V].
  - Salvar como: escolher tipo e clicar **Opções** abre os parâmetros de exportação do formato [V]. No STEP há "saída como sólidos e superfícies" e AP203/AP214/242 [V parcial]; AP242 com PMI 3D exige o módulo MBD [V, CATI e fórum].
  - Há **Publish to STEP242** como comando dedicado [V].
  - **Task Scheduler**, aplicativo separado, tem a tarefa "Export Files" (DXF, DWG, IGES, PDF, STEP AP203/AP214, JPEG) [V].
- **Inventor — no detalhe:**
  - **Abrir** e **Importar** têm o botão Opções. A referência de opções STEP/IGES do Inventor lista: salvar componentes durante o carregamento, pasta de destino dos componentes, colocar a montagem de topo em pasta separada, **relatório de tradução** (embutido no documento ou salvo em disco), tipos de entidade (sólidos, superfícies, fios, pontos), organização (reparar, montagem como peça única, agrupamentos), unidades e pós-processos [V].
  - Observação: o STEP só abre pelo comando Abrir, não pelo Importar [V]. Há relatos de o botão Opções ficar cinza em versões novas [V, fórum].
  - A exportação fica em Arquivo > Exportar > formato CAD, com opções de AP [C].
- **Outros:**
  - **Onshape** trata tudo como "Importar" no Documents ou no documento, com opção de traduzir; preserva a aba original para retraduzir [V]. Para nativos de montagem exige um .zip com o nome igual ao da montagem de topo [V].
  - **Fusion** envia o arquivo para tradução na nuvem e devolve .f3d [V]; JT sem tradução anunciada [V, fórum].
  - **Creo** tem diálogo próprio com protocolos AP203/AP214/AP242 [V, Creo PMA].
- **Fenkra hoje:**
  - `app.rs:2528-2535` — dois botões em Arquivo chamam `self.export("step")` e `self.export("stl")`.
  - `app.rs:880-892` abre `rfd::FileDialog` com filtro pelo `ext` e chama `fenkra_solidos::export(&doc, None, caminho)`.
  - Não há diálogo de opções, escopo, feedback de progresso nem relatório; o resultado é só um toast "Exportado: …".
  - A importação de malha vive na barra de ferramentas "Malha" (`toolbar.rs:479`, `Cmd::MeshImport`, `app.rs:1754`), que escolhe o arquivo e cria a feature.
- **Melhor prática para o Fenkra:**
  - **Dois itens em Arquivo**, "Importar…" e "Exportar…" (com submenus rápidos por categoria), mais "Conversão em lote…" e "Relatórios de conversão". Isso atende o pedido de menu separado e deixa Salvar/Abrir intactos.
  - **Importar…**: seletor único com filtro por categoria. O formato é detectado por conteúdo (cabeçalho `ISO-10303-21`, `AC1018`, `solid`, `glTF`, ZIP com `3D/3dmodel.model`), e a extensão serve de dica; ao divergir, avisar.
  - Depois do arquivo, um **painel de opções do formato** seguindo o padrão do ADR 0011 (suporte → modo → parâmetros → avançado): *Destino* (nova peça / nova montagem / inserir na peça ativa / inserir na montagem), *Modo* (Referência vinculada / Converter), *O que importar* (sólidos, superfícies, curvas, pontos, PMI, cores, camadas), *Unidades* (detectada / forçar), *Reparo* (nenhum / padrão / agressivo) e *Avançado* (tolerância, costura, unir faces).
  - **Exportar…**: escolhe o formato, depois o escopo (documento ativo, corpo, seleção, montagem inteira com ou sem estrutura) e as opções. Ao final, mostra o relatório (arquivos gravados, tamanho, o que foi omitido) e oferece "Registrar no projeto".
  - **Regras que evitam bugs simples:** (1) nunca decidir o formato pela extensão sem validar (corrige o bug L2); (2) nome de arquivo com acento e espaço sempre testado (o caminho do usuário contém "João"); (3) sempre reportar o que NÃO foi levado; (4) lembrar as últimas opções por formato (como o CATIA, mas por perfil nomeado).
  - **Testes:** `export_ext_desconhecida_erra`, `export_caminho_com_acento`, `detecta_formato_por_conteudo`, `opcoes_por_formato_persistem`, `dialogo_importar_lista_so_formatos_suportados`.

---

### Importar STEP (AP203 / AP214 / AP242) (CATIA: File > Open stp, ST1 / SW: Abrir STEP + 3D Interconnect / Inventor: Abrir STEP (Reference ou Convert) / outros: Onshape Import, Creo Import STEP / Fenkra: `import_step`, sem UI)

- **O que faz:** lê um modelo B-rep (sólidos, cascas, superfícies, curvas), a estrutura de montagem, unidades, nomes e, conforme o protocolo, cores, camadas e PMI.
- **CATIA V5 — no detalhe:**
  - Abre-se pela caixa File > Open. O STEP traz o que o AP e a licença permitem: ST1 cobre geometria e estrutura de AP203/AP214 (casca e sólido); SXT acrescenta PMI/FTA e composites [V].
  - Os protocolos listados vão de AP203/AP203 ext/AP214 a AP203 ed2, AP214 ed3 e AP242 ed1–ed3; PMI semântico aparece nas edições mais novas [V parcial].
  - Há dois modos para anotações: *gráfico* (polilinhas importadas como "Result") e *autoral+gráfico* (linhas mais entidades semânticas; cria dois conjuntos de anotações; as semânticas viram FTA editáveis) [V]. Editar o resultado exige licença de Tolerancing e Annotation (V5-FT1/FTA) [V].
  - Um AP203 reimportado mostra só geometria, sem cor nem FT&A; AP214 ed3 com as opções certas preserva cor e FT&A [V, Technia].
  - **Na árvore:** a peça sai com Body/Geometrical Sets do que foi lido; cada produto vira Product/Part. O sólido costuma chegar sem histórico, como corpo isolado ("isolated solid") [V parcial: STEP não leva histórico].
  - **Quando falha:** alguns STEP chegam sem sólido ou com faces faltando e o sólido não se forma [V, IBM]; trata-se em Healing/Join de superfícies.
- **SolidWorks — no detalhe:**
  - O diálogo de abertura tem filtros e **Opções**. Há "Enable Filter" para aplicar filtros antes da importação e trazer só alguns componentes [V]. Um STEP com FACETED_BREP entra como *corpo gráfico* por padrão e "Load Geometry" o carrega como B-rep [V].
  - Com o **3D Interconnect** ligado, a abertura mantém **vínculo associativo com o original** (Open) ou insere sem converter (Insert); é possível atualizar e quebrar o vínculo [V]. STEP, IGES, ACIS, IFC, Inventor, CATIA V5, Creo, NX, Solid Edge e xDesign usam a tecnologia nova mesmo com o 3D Interconnect desmarcado [V].
  - A primeira coisa depois de abrir é o **Import Diagnostics**, que roda automaticamente por padrão; se desmarcado, pergunta a cada importação [V].
  - **Na árvore:** a feature "Imported1" (corpo importado). Para curar, o Interconnect precisa estar desligado ou dissolvido, e a árvore só pode ter corpos importados [V, GoEngineer].
- **Inventor — no detalhe:**
  - Opções (ver ficha do menu) incluem *Importar para o ambiente de Reparo*, *Importar montagem como peça única* (compósito único ou peça de múltiplos sólidos), *Criar superfícies como* (corpos individuais, compósito único/múltiplos, grupos de construção), *Auto Stitch and Promote*, *Enable Advanced Healing* e *Check Parts during Load* [V].
  - O **AnyCAD** oferece **Referência** (vínculo, sem arquivos novos) ou **Converter** (arquivos Inventor novos, sem vínculo). STEP é exceção: cada atualização é uma nova tradução, que pode deixar geometria sem resolver e quebrar edições; use Referência para STEP só se as atualizações forem pequenas [V].
  - Na árvore, nó "3rd Party" com o relatório de tradução; seleção de corpos/componentes com visualização prévia antes de importar [V].
- **Outros:** Onshape lê AP203/AP214/AP242 (geometria e cor de face na importação) e trata Parasolid como formato preferido [V]. NX e Solid Edge, sendo baseados em Parasolid, recebem dado neutro com a Synchronous Technology para edição direta [V parcial].
- **Fenkra hoje:**
  - `kernel.cpp:1004-1011`: `STEPControl_Reader r; r.ReadFile(...)` (checa só `IFSelect_RetDone`), `r.TransferRoots();`, `wrap(r.OneShape())`.
  - Problemas prováveis: (a) não confere o retorno de `TransferRoots` nem `NbShapes`, e `OneShape()` de zero raízes dá forma nula; (b) sem XDE, tudo vira um compound anônimo (sem árvore, nomes, cores); (c) sem unidade explícita (o padrão do OCCT converte para mm [V]); (d) sem healing depois da leitura; (e) caminho com acento sem teste.
  - Nada na UI chama a função (`grep` encontra só kernel e ffi).
- **Melhor prática para o Fenkra:**
  - **Fluxo:** arquivo → relatório de pré-leitura (AP detectado, unidade, nº de produtos, entidades) → opções → leitura via `STEPCAFControl_Reader` (cores, nomes, camadas, props ligados) → reparo → destino.
  - **Parâmetros do leitor** (todos [V] do guia STEP): precisão `read.precision.mode/val` (0,0001 mm por padrão; modo File ou User), `read.maxprecision.val/mode` (Preferred ou Forced; Forced pode gerar forma inválida), `xstep.cascade.unit` (mm), `read.step.assembly.level` (tudo, só estrutura, só forma), `read.step.shape.repr`, `read.step.tessellated` (Off/On/OnNoBRep) e `read.step.product.mode`.
  - **Regras:** (1) sempre usar o `DESTEP_Parameters` por chamada, não as globais `Interface_Static`, que não são seguras para threads [V]; (2) separar "falha de leitura" de "leitura parcial" no relatório; (3) FACETED_BREP/tessellated chega como malha, não B-rep; marcar o corpo como "malha" e oferecer converter; (4) preservar o nome de produto e o ID do produto na feature para a reimportação por correspondência.
  - **Testes:** viagem de ida e volta com sólido do corpus (volume, área, centroide, bbox, nº de faces dentro de tolerância); STEP com 2 instâncias do mesmo produto vira 1 definição + 2 posições; STEP em polegadas; STEP AP203 sem cores; arquivo truncado dá erro legível; STEP com superfícies abertas (casca) vira superfície; STEP "faceted" avisa.

---

### Exportar STEP (AP203/214/242) (CATIA: Save As stp + Compatibility / SW: Salvar como STEP + Opções, Publish to STEP242 / Inventor: Exportar STEP / outros: Creo ap203_e2/ap214_is/ap242 / Fenkra: `export_step`)

- **O que faz:** grava geometria B-rep, e conforme o protocolo, estrutura, cores, camadas, nomes, propriedades e PMI.
- **CATIA V5 — no detalhe:** configura-se em Tools > Options > General > Compatibility > STEP, seção Export, "Application Protocol" (ex.: "242 ed1" para levar PMI) e, na seção geral, "3D Annotations" marcado [V]. A exportação exige ST1 (e FTA para criar as anotações) [V]. Em Save As escolhe-se "stp (*.stp)" [V]. O modo "Authorable & Graphic" grava PMI semântico mais gráfico [V]. A exportação aninhada de montagens vem do SXT [V].
- **SolidWorks — no detalhe:** Salvar como > tipo STEP > Opções: escolhe-se AP203/AP214/AP242, "sólidos e/ou superfícies" e [C] sistema de coordenadas de saída. AP242 com PMI 3D exige o MBD (usuário Professional relatou não exportar AP242) [V, CATI/fórum]; há também o comando Publish to STEP242 [V]. A tarefa agendada "Export Files" faz AP203/AP214 em lote [V].
- **Inventor — no detalhe:** AP203, AP214 e AP242 em Exportar; o AP242 e o `.stpz` compactado são tratados desde o Inventor 2016 [V parcial, ARKANCE]. Detalhes das opções da caixa de exportação: [C].
- **Outros:** o Creo grava assembly como arquivo único ou separado por componente, e leva anotações (GD&T, notas) nos protocolos ap214_is, ap203_e2, ap242 [V]. O OCCT escreve a montagem sempre na forma `representation_relationship_with_transformation` [V].
- **Fenkra hoje:**
  - `kernel.cpp:988-994`: `STEPControl_Writer w; w.Transfer(s.s, STEPControl_AsIs); w.Write(path)`. Esquema padrão do OCCT (AP214CD) [V pelos parâmetros], nome de produto igual à versão do OCCT [V].
  - `fenkra-solidos/src/lib.rs:5144-5161`: roda todas as features, faz `compound` dos corpos de todos os coletores e grava; erro "nada para exportar" se vazio.
  - Falta: nomes de corpos/peça, aparência (`Document.appearance`, `BodyCollector` com cor), PMI (`Document.pmi`), unidade explícita, esquema configurável, montagem (`fenkra-montagem` não tem exportação), validação de propriedades. Não filtra por visibilidade, até onde li [C].
  - **Bug:** `lower.ends_with(".stl")` ou "qualquer outra coisa = STEP". Um `.igs`, `.step.txt` ou `.obj` sai como STEP com extensão errada.
- **Melhor prática para o Fenkra:**
  - **Fluxo:** escopo → protocolo (AP214 padrão; AP242 se houver PMI ou cores avançadas; AP203 para legados) → o que levar (cores, nomes, camadas, PMI, propriedades) → unidade → avançado (precisão `write.precision.val`, `write.surfacecurve.mode` desligado reduz o arquivo [V], `write.step.assembly` Off/On/Auto, `write.step.vertex.mode`, `write.step.tessellated`).
  - **Gravar via XDE:** `STEPCAFControl_Writer` com um `TDocStd_Document` montado a partir do Fenkra (um produto por corpo/peça, localizações das instâncias, cores por face com `XCAFDoc_ColorTool`, nomes com `TDataStd_Name`). Isso é o que o guia XDE recomenda para cor, camada, nome, validação e GD&T [V].
  - **Propriedades de validação** (centroide, volume, área) gravadas permitem ao receptor detectar erro de tradução [V]; ligar sempre.
  - **Metadados** do arquivo: autor, organização, PN, revisão do Fenkra, versão do Fenkra, hash do documento. O OCCT 8 passou a exportar General Attributes e metadados STEP [V].
  - **Regras:** (1) validar o formato pelo conjunto de extensões aceitas (`.step`, `.stp`, `.stpz`); (2) relatar o que foi omitido (ex.: "PMI semântico não suportado no AP203"); (3) nome de produto = PN do Fenkra; (4) leitura de verificação opcional (reabrir e comparar volume).
  - **Testes:** `export_step_ida_e_volta_preserva_volume`, `export_step_cores_por_face`, `export_step_montagem_preserva_instancias`, `export_step_ap242_pmi_semantico_reabre`, `export_ext_errada_erro`.

---

### Dados carregados pelo STEP: PMI semântico, cores, camadas, estrutura e metadados (CATIA: ST1/SXT, FTA / SW: MBD, 3D Interconnect / Inventor: Translation report / outros: Onshape cor de face / Fenkra: `Document.pmi`, `appearance`, árvore da montagem)

- **O que faz:** define o que cada categoria de informação consegue atravessar o formato neutro.
- **CATIA V5 — no detalhe:** geometria e estrutura (ST1); PMI/FTA, composites e aninhamento (SXT) [V]. PMI gráfico vira polilinha; semântico vira FTA editável. A cor e o FT&A só voltam com AP214 ed3+ e as opções certas [V]. Composites só levam a "engineering stacking"; planificados exportam como curvas [V].
- **SolidWorks — no detalhe:** PMI 3D é exportado em STEP 242 com o MBD [V]; o 3D Interconnect importa PMI de nativos [C]. A cor de face e a estrutura de montagem são levadas; no Onshape a importação de STEP traz geometria e cor de face [V].
- **Inventor — no detalhe:** a importação STEP lê sólidos, superfícies, fios e pontos [V]; PMI semântico limitado [C]. O nó 3rd Party guarda o relatório [V].
- **Outros:** O OCCT 8 adicionou PMI scaling e coordenada de conexão na leitura [V]. O guia XDE descreve GD&T semântico e tesselado (apresentação), clipping planes, vistas salvas e notas; **planos de corte e vistas só são gravados em XBF, não em STEP** [V].
- **Fenkra hoje:** nada atravessa. `Document` tem `appearance`, `pmi`, `pmi_captures`, `views` (doc.rs:2673-2712), mas o STEP gerado não leva nenhum deles. `docs/modulos/pmi-3d.md` existe como módulo, sem ponte para STEP.
- **Melhor prática para o Fenkra:**
  - **Matriz de fidelidade** (a mostrar no relatório e no diálogo):

    | Informação | AP203 | AP214 | AP242 | OCCT 8.0.1 |
    |---|---|---|---|---|
    | Geometria B-rep, estrutura | sim | sim | sim | sim (STEPControl/CAF) [V] |
    | Cor por face/corpo | não (pobre) [V] | sim | sim | XDE [V] |
    | Camadas | limitado [C] | sim [C] | sim [C] | XDE [V] |
    | Nomes, ID de produto | sim | sim | sim | XDE [V] |
    | PMI semântico | não | limitado [V] | **sim** | `XCAFDoc_DimTolTool` [V] |
    | PMI de apresentação | não | não | sim | precisa `SetGDTPresentations` [V] |
    | Validação (volume/área/centroide) | sim | sim | sim | XDE [V] |
    | Vistas/clipping/notas | não | não | parcial | só no XBF [V] |

  - **Prioridade de PMI:** o Fenkra já tem anotações 3D. Exportar cota, tolerância geométrica e datum **semânticos e com apresentação** coloca o Fenkra à frente de quem só exporta o gráfico. Começar por cotas lineares/diametrais, datums e tolerâncias de forma/posição; o resto fica registrado como "não suportado" no relatório.
  - **Metadados:** PN, descrição, material, massa, revisão, estado do ciclo de vida como *general attributes* [V que o OCCT 8 exporta], com chaves `fenkra:`.
  - **Testes:** `pmi_semantico_cota_ida_e_volta`, `cor_de_face_sobrevive_ao_step`, `step_com_camadas_reabre_com_nomes`, `metadados_pn_rev_no_cabecalho`.

---

### IGES (import/export) (CATIA: IGES nas opções de Compatibility / SW: Abrir/Salvar IGES, Import Diagnostics / Inventor: Abrir IGES, Convert / outros: Onshape IGES 5.3 / Fenkra: nenhum)

- **O que faz:** formato mais antigo, forte em **superfícies e curvas**; fraco em sólidos e montagens.
- **CATIA V5 — no detalhe:** há aba IGES em Compatibility; a opção "Join" define a tolerância de junção das superfícies importadas [V]. Lote pelo `BATCH-DXF-IGES-STEP` [V]. Na árvore, as superfícies entram em Geometrical Sets, a juntar/curar [V parcial].
- **SolidWorks — no detalhe:** o diálogo "IGES-In Surfaces, Curves, and Levels" aparece se o arquivo traz curvas ou níveis diferentes [V]. Em IGES, "múltiplos corpos como peças" vale [V]. A saída IGES tem tipos de entidade configuráveis e a representação de superfície (a tarefa de conversão do PDM expõe 3D curves, entity types, surface representation) [V].
- **Inventor — no detalhe:** as opções STEP/IGES são compartilhadas; "Criar superfícies como" e agrupamento por **nível/camada IGES** valem para IGES [V]; sem reparo, IGES de superfícies costuma precisar de Auto Stitch [V].
- **Outros:** Onshape lê IGES até 5.3 [V].
- **Fenkra hoje:** não há leitor nem escritor IGES. O OCCT tem `IGESControl`/`IGESCAFControl` e `DEIGES` (leitura/gravação, **não thread-safe** [V]).
- **Melhor prática para o Fenkra:**
  - **Importação:** IGES entra como **superfícies/curvas** em "Referências" do Fenkra (equivalente a Geometrical Set), nunca direto em sólido; depois, sequência *Costurar → Fechar → Sólido* já existente (`sewn`, `close_surface`). O OCCT traduz só geometria 3D, pula o resto e diz quais tipos pulou; comprimentos em outras unidades são convertidos para mm [V]. Transformações soltas não são traduzidas [V]. Registrar esses avisos.
  - **Exportação:** modo *Faces* (padrão do OCCT, superfícies aparadas tipo 144) contra modo *BRep* (tipo 510, estrutura completa) [V]. Padrão do Fenkra: BRep para sólidos; Faces só se o destino for legado. Unidade configurável (`write.iges.unit`) [V].
  - **Cor/camada:** via `IGESCAFControl` com `SetColorMode`/`SetNameMode` [V].
  - **Regra de concorrência:** serializar todas as leituras/gravações IGES (mutex global) ou rodá-las em processo filho [V: não thread-safe].
  - **Testes:** `iges_superficie_vira_referencia`, `iges_costura_forma_solido_fechado`, `iges_unidade_polegada_converte`, `iges_concorrente_nao_corrompe` (dois lotes em paralelo).

---

### Parasolid (x_t/x_b) e ACIS (sat) (CATIA: nenhum nativo / SW: Salvar/Abrir Parasolid, ACIS / Inventor: Convert SAT, Parasolid / outros: Onshape (Parasolid preferido) / Fenkra: nenhum)

- **O que faz:** troca de B-rep exato entre kernels (Parasolid: SolidWorks/NX/Solid Edge/Onshape; ACIS: AutoCAD/SolidWorks/Inventor/Spatial).
- **CATIA V5 — no detalhe:** não lê Parasolid/ACIS direto; usa STEP/IGES [C].
- **SolidWorks — no detalhe:** abre e grava `.x_t/.x_b` e `.sat`; a tarefa Export Files lista Parasolid [V]; o PDM tem opções avançadas de conversão para ACIS e Parasolid [V].
- **Inventor — no detalhe:** AnyCAD *Convert* aceita Parasolid e SAT [V]; Referência não lista Parasolid/SAT [V].
- **Outros:** Onshape importa Parasolid v10.0–v38.1 (e malha 28–37) e ACIS até 2023 e grava Parasolid v25–37 e ACIS v5 [V]. Siemens recomenda `x_b` como formato estável [V]. O XT guarda **só B-rep exato**, sem facetas, com unidades **sempre em metros** [V, Pointwise]. A referência do formato XT é pública (PDF de 2006), mas a licença de redistribuição não ficou confirmada [V parcial]. Gravar/ler com fidelidade exige o kernel (Parasolid) ou a biblioteca comercial (HOOPS, Spatial) [C].
- **Fenkra hoje:** nada. O OCCT **não** tem DE para Parasolid/ACIS [V: ausente da tabela do DE_Wrapper].
- **Melhor prática para o Fenkra:**
  - Não prometer Parasolid/ACIS na primeira onda. **Via de menor custo:** STEP AP214/242 já resolve a troca entre esses kernels.
  - Se o usuário pedir (fábricas que usam Parasolid), oferecer o formato como **plugin comercial opcional** (HOOPS Exchange/CAD Exchanger/Spatial) atrás da interface de provedor do módulo conversor. Nota: ACIS e Parasolid dependem de licença do kernel, e o licenciamento do HOOPS/CAD Exchanger não ficou verificado.
  - **Teste:** quando houver provedor, round-trip x_b ↔ B-rep com tolerância de volume.

---

### JT (CATIA: via Datakit/HOOPS / SW: não nativo (exportação por add-in) / Inventor: Convert JT / outros: NX/Teamcenter, Onshape JT 11 / Fenkra: nenhum)

- **O que faz:** formato leve de visualização (ISO 14306), com **LODs** de tesselação, estrutura de produto e PMI, mais B-rep opcional.
- **CATIA V5 — no detalhe:** sem JT nativo; terceiros fazem a ponte [C].
- **SolidWorks — no detalhe:** sem JT nativo [C].
- **Inventor — no detalhe:** *Convert* aceita JT [V].
- **Outros:** o JT é um padrão ISO 14306 com referência gratuita em jtopen.com [V]. Carrega múltiplas tesselações (LOD), PMI compatível com o esquema PLM XML e estrutura de produto, monolítica ou em arquivos ligados (PER_PART, com PMI de montagem no arquivo de topo e PMI de peça nos arquivos de componente) [V]. HOOPS Exchange grava JT 8.1/9.5/10 [V]. Onshape importa e exporta JT até 11.0 [V]. O Fusion não traduz JT [V, fórum Autodesk].
- **Fenkra hoje:** nada; o OCCT não lê JT [V: ausente].
- **Melhor prática para o Fenkra:** deixar JT para uma **onda comercial** (leitura via HOOPS/CAD Exchanger). Para visualização leve de montagens grandes, o Fenkra tem melhor retorno em **glTF** (aberto, RW no OCCT [V]) como formato de exportação leve. Registrar no relatório "JT não suportado, use STEP AP242".

---

### Nativos de outros CADs: CATPart/CATProduct, SLDPRT/SLDASM, IPT/IAM, Creo, NX, Solid Edge (CATIA: leitura de SW via módulos/tradutores / SW: 3D Interconnect / Inventor: AnyCAD / outros: Onshape, HOOPS, CAD Exchanger, Datakit, Spatial / Fenkra: fora do escopo (visão.md §6))

- **O que faz:** abre arquivos nativos sem passar por formato neutro, com mais fidelidade (histórico opcional, PMI, cor, propriedades).
- **CATIA V5 — no detalhe:** abre CATPart/CATProduct; para outros CADs depende de licenças e tradutores específicos; a troca habitual é STEP. [C]
- **SolidWorks — no detalhe:**
  - O **3D Interconnect** abre/insere nativos de CATIA V5 (V5 R8 a V5-6R2023), Creo (Pro/E 16 a Creo 10), NX (UG11 a NX 2406), Inventor (IPT/IAM), Solid Edge, entre outros [V, ajuda 2026 via busca]. O CATIA V5 exige SolidWorks Premium [V].
  - **Abrir** mantém vínculo associativo com a peça original; **Inserir** põe o dado de terceiros na montagem sem converter; é possível atualizar, quebrar o vínculo, e as features SolidWorks seguintes se atualizam junto [V].
  - O PDM reconhece essas referências no check-in e as lista como referências CAD, inclusive a submontagem Inventor com os filhos [V].
- **Inventor — no detalhe:** o AnyCAD (Referência/Converter) suporta Fusion, CATIA, SolidWorks, Solid Edge, Creo, NX, JT, Alias, Rhino, IGES, Parasolid, STEP, SAT [V]; Referência vale para CATIA, SolidWorks, NX, STEP, Solid Edge, Creo etc. [V]. Em peça: edição de geometria importada, Direct Edit e "Copy to" funcionam; em montagem, restrições a geometria importada e BOM sim, mas não se modifica a estrutura importada nem espelha [V]. Arquivos Mechanical Desktop não mantêm vínculos [V].
- **Outros:**
  - **Onshape** importa nativos: CATIA v5 (R7–R33, V5-6R2026), SolidWorks (1999–2026), Inventor (9–2026), Creo (2000i–12), NX (UG15–NX 2512), Solid Edge (v10–2026). Montagem nativa vai em .zip com nome igual ao da montagem de topo [V].
  - **Bibliotecas comerciais:**
    - HOOPS Exchange: CATIA V5 até V5-6R2024, Creo até 11, NX até 2412, Parasolid até v37.1, SolidWorks 97–2025, JT até 10.9, STEP AP203/214/242, Inventor [V, lista da Communicator].
    - CAD Exchanger SDK: SolidWorks, STEP, JT, STL e "30+" formatos, com PMI; C++/C#/Java/Python/JS [V, site].
    - Spatial 3D InterOp: lê CATPart/CATProduct e SolidWorks sem licença do CAD de origem, mais NX/Solid Edge/SolidWorks via Parasolid embutido [V].
    - Datakit CrossCad/Ware: SDK para editores de software, com CATIA V5 R/W, SolidWorks, Inventor, NX, Creo [V].
  - **Todas são pagas** e o preço não é público [V, Datakit/Spatial sem tabela].
- **Fenkra hoje:** visão.md §6 declara a leitura de nativos fechados como **não-objetivo** ("A troca é feita por formatos neutros"). Nada implementado.
- **Melhor prática para o Fenkra:**
  - Manter o não-objetivo para a fase atual e **desenhar a interface de provedor** (trait) para que um plugin comercial (HOOPS/CAD Exchanger) possa aparecer depois sem mudar diálogos, relatórios nem PDM.
  - Na tela de importação, ao detectar um nativo sem provedor, mostrar uma **mensagem de orientação**: "Arquivo SLDPRT. O Fenkra lê formatos neutros. No CAD de origem, exporte STEP AP214/AP242 (opções recomendadas: …)", com um guia curto por CAD de origem. Isso evita o beco sem saída.
  - **Reconhecer o vínculo** (Referência/Converter) no modelo de dados desde já (ver ficha de vínculo), mesmo antes de ter nativos.
  - **Teste:** `arquivo_nativo_sem_provedor_orienta_exportacao_neutra`.

---

### DXF e DWG (2D) (CATIA: Save As dxf/dwg em Drafting / SW: Salvar como DXF/DWG; Importar DXF/DWG para sketch ou desenho / Inventor: DWG/DXF File Wizard, Export DWG / outros: Onshape DXF/DWG até 2018 / Fenkra: DXF R12 só gravação no desenho)

- **O que faz:** troca desenhos 2D e perfis de corte (chapa planificada, laser/plasma).
- **CATIA V5 — no detalhe:** Save As dxf/dwg na Drafting; lote pelo BATCH-DXF-IGES-STEP [V]. Mapeamento de camadas/linhas por arquivo de configuração [C].
- **SolidWorks — no detalhe:** Salvar como DXF/DWG de desenho e de chapa planificada; a tarefa Export Files faz DXF/DWG em lote (de desenhos) [V]. Importar: Assistente de Importação de DXF/DWG com destino (desenho, peça, sketch) e mapeamento de camadas [C].
- **Inventor — no detalhe:** DWG/DXF File Wizard e abertura como **underlay DWG associativo**, que acompanha as mudanças no AutoCAD [V]; DXF, OBJ, STL, DWF Markup e IDF são não associativos e geram relatório de tradução [V].
- **Outros:** Onshape importa DWG até 2018 e DXF até 2013/2018 e grava PDF, DWG, DXF, DWT, SVG, PNG, JPEG [V].
- **Fenkra hoje:**
  - Gravação: `export.rs:51-150` gera **DXF R12 (ASCII)** com camadas por tipo de linha; `export.rs:265-283` exporta por sufixo; `dwg_ui.rs:242` liga o botão. Teste em `fenkra-desenho/src/lib.rs:821`.
  - R12 é de 1992: sem spline nativa, sem cor verdadeira, sem espessura de linha moderna e sem Unicode (usa `%%c %%d %%p`, `export.rs:153`) [C].
  - **Não existe leitura de DXF** (RF-NUC-012 aberto), nem DWG. A planificação de chapa vai para DXF só pelo desenho.
- **Melhor prática para o Fenkra:**
  - **Leitura de DXF** como importação para sketch (linhas, arcos, círculos, polilinhas, splines, textos), com mapeamento de camadas e **unidade** explícitos, destino "novo sketch" ou "no sketch ativo", e juntar entidades quase conectadas com tolerância. Biblioteca candidata: crate Rust `dxf` (leitura/gravação DXF/DXB, versão 0.6.0 de nov/2024 [V]; licença não confirmada, verificar antes).
  - **Escrita:** subir de R12 para R2000/R2010 (spline, LWPOLYLINE, ACI/true color, Unicode) mantendo R12 como opção "compatibilidade máxima".
  - **DWG:** ODA/Teigha é paga (associação "Sustaining" listada por US$ 7.500 no primeiro ano e US$ 4.500 recorrentes, a confirmar no PDF de 2026 [V parcial]). LibreDWG é **GPL** (incompatível com a licença do Fenkra; só como **processo separado**, conforme `reuso-opensource.md`) e a escrita só chega às versões antigas [V]. **Recomendação:** DWG = *Conversor externo opcional* (usuário aponta o executável), nunca no binário.
  - **Regras:** (1) sempre perguntar a unidade quando o DXF não tem `$INSUNITS`; (2) preservar camada como atributo; (3) relatar entidades ignoradas (3DFACE, hatch, bloco aninhado).
  - **Testes:** `dxf_ida_e_volta_preserva_comprimentos`, `dxf_sem_insunits_pede_unidade`, `dxf_spline_vira_spline`, `dxf_layers_mapeadas`, `planificacao_dxf_abre_em_cam_de_referencia` (visual).

---

### Malhas: STL, 3MF, OBJ, PLY, glTF, VRML (CATIA: Save As stl/wrl/cgr / SW: STL, 3MF, VRML / Inventor: STL, OBJ, 3MF / outros: Onshape glTF, 3MF, OBJ / Fenkra: STL (ex.), STL/OBJ (imp.))

- **O que faz:** troca facetada para impressão 3D, visualização e renderização.
- **CATIA V5 — no detalhe:** grava STL e VRML (e CGR, formato leve próprio) pelo Save As; STL com tolerância/"sag" configurável [C].
- **SolidWorks — no detalhe:** STL e 3MF com opções de resolução (grosseira, fina, personalizada: desvio e ângulo), unidades e "salvar todos os componentes em um arquivo" [C]; VRML com cor.
- **Inventor — no detalhe:** STL com unidades, resolução e binário/ASCII; OBJ e 3MF na importação e exportação [C]; importação de OBJ/STL é não associativa e gera relatório [V].
- **Outros:**
  - **OCCT:** STL RW (DESTL), OBJ RW (DEOBJ), glTF/GLB RW (DEGLTF), VRML RW (DEVRML), PLY só gravação (DEPLY) [V].
  - **3MF** não está no OCCT [V: ausente]. Especificação: Core v1.3.0, Materiais v1.2.1, Produção v1.1.2, Beam Lattice v1.2.0, e é a norma ISO/IEC 25422:2025 com patentes licenciadas royalty-free [V]. Um 3MF é ZIP + XML [C]; carrega **unidades, múltiplos objetos, transformações, cores/materiais** [C].
  - Importar malha vira "corpo de malha" no CATIA/SW/Inventor, mas o Fenkra converte em B-rep facetado (feature Malha).
- **Fenkra hoje:**
  - Exportar STL: `kernel.cpp:996-1002` — `BRepMesh_IncrementalMesh(s, 0.05, False, 0.3, True)` (deflexão linear **absoluta fixa de 0,05**, angular 0,3 rad) e `StlAPI_Writer` com modo padrão (ASCII é o padrão do OCCT [C], gera arquivo grande). Qualidade fixa, ligada ao tema B61.
  - Importar: `mesh_io.rs` lê STL binário/texto e OBJ (leque de triângulos para polígonos); **teto de 200 mil triângulos** (`mesh_io.rs:11`); feature `MeshImport` (`doc.rs:1856`) guarda **caminho absoluto/relativo** e `solid`; `mesh_shape` (`hybrid.inc:370-401`) costura com tolerância 1e-6 e tenta virar sólido (`ShapeFix_Solid`).
  - Problemas prováveis: (a) STL não tem unidade e o Fenkra assume a do arquivo sem perguntar; (b) a feature referencia o arquivo externo (mover/renomear quebra o recompute; hoje não há "vínculo quebrado" visível); (c) 1e-6 de costura falha em STL com vértices arredondados (float32), o que dá "superfície aberta" em vez de sólido; (d) sem cor, sem normais, sem 3MF/PLY/glTF.
- **Melhor prática para o Fenkra:**
  - **Exportação:** presets "Rascunho / Impressão 3D / Alta fidelidade" (+ personalizado: deflexão linear, angular, relativa/absoluta), binário por padrão, unidade (mm/pol), escopo (selecionado/visível/tudo), "um arquivo por corpo". Aplicar a malha em **cópia** da forma (a malha não pode alterar o modelo; princípio do B61). Verificar **malha fechada** (2-manifold) e avisar.
  - **3MF** (alvo de impressão 3D): gravar com `zip` + XML, unidade, cor, vários objetos; leitura mínima. Precisa de verificação de licença do pacote escolhido.
  - **glTF/GLB:** via `RWGltf` [V], para visualização, com cores/nomes. **PLY** só exportação (como o OCCT).
  - **Importação:** assistente com unidade, "tentar sólido" e tolerância de costura (`1e-6` → relativa ao tamanho da malha, ex. 1e-5 × diagonal), soldar vértices próximos antes de costurar, e **embutir a malha no projeto** (hash) em vez de depender do caminho externo.
  - **Testes:** `stl_binario_e_ascii_mesma_geometria`, `stl_float32_vertices_soldados_viram_solido`, `stl_exportado_e_malha_fechada`, `malha_movida_marca_vinculo_quebrado`, `3mf_ida_e_volta_com_unidade`.

---

### PDF 3D (PRC / U3D) (CATIA: 3D PDF via 3DEXPERIENCE / SW: Publish to 3D PDF (MBD) / Inventor: Export 3D PDF / outros: HOOPS Publish / Fenkra: PDF 2D vetorial)

- **O que faz:** embute um modelo 3D (e PMI) dentro de um PDF para revisão sem CAD.
- **CATIA/SW/Inventor:** o SW e o Inventor publicam 3D PDF a partir do MBD/templates [C]; detalhes de cada um não confirmados.
- **Outros:** **PRC** é o ISO 14739-1:2014 e foi incorporado ao PDF 2.0; **U3D** é o ECMA-363, do PDF 1.7 [V]. A diferença de conteúdo (B-rep exato × malha; PMI) não ficou confirmada nesta pesquisa [V parcial]. O HOOPS Publish grava 3D PDF [V parcial]. Extensões ISO acrescentaram STEP AP242 e glTF em PDF [V].
- **Fenkra hoje:** PDF 2D vetorial gerado em `export.rs` (`pdf(&pages)`); sem 3D.
- **Melhor prática para o Fenkra:** prioridade baixa (P3). Se feito, **U3D com malha + vistas de PMI do Fenkra** (as `pmi_captures`) embutido no PDF do desenho, gerado por código próprio (a estrutura PDF 3D é um *annotation* + stream U3D). Alternativa de custo mínimo: anexar o **glTF/STEP como arquivo anexo** do PDF. Teste: `pdf3d_abre_no_leitor_de_referencia` (manual).

---

### VRML (CATIA: Save As wrl / SW: VRML / Inventor: — / outros: Onshape — / Fenkra: nenhum)

- **O que faz:** formato de malha colorida antigo, ainda usado em visualização e impressão colorida.
- **Detalhe por CAD:** CATIA e SolidWorks gravam `.wrl` [C]. O PDM do SolidWorks tem opções avançadas de conversão VRML [V].
- **Fenkra hoje:** nada. O OCCT tem `DEVRML` RW (saída malha) [V].
- **Melhor prática para o Fenkra:** P3. Reaproveitar o gerador de malha do STL/glTF; entrada VRML não é prioridade (usar glTF/3MF). Teste: `vrml_cor_por_corpo`.

---

### IFC (BIM) (CATIA: — / SW: Abrir IFC (3D Interconnect) / Inventor: — / outros: HOOPS IFC XML, Revit / Fenkra: nenhum, "fase BIM" em reuso-opensource.md)

- **O que faz:** troca de modelos de edificação (estrutura, instalações), com classes semânticas (IfcBeam, IfcWall) e geometria implícita.
- **SolidWorks:** abre IFC pelo 3D Interconnect [V]. **HOOPS** lê/grava IFC [V parcial].
- **Outros:** o **IfcOpenShell** usa o OCCT para converter geometria implícita em B-rep, inclusive para STEP/OBJ/DAE; o iterador entrega formas OCCT sem triangular (`DISABLE_TRIANGULATION` + `USE_BREP_DATA`) [V]. A conversão **STEP→IFC** não é automática, pois falta semântica; exige construir a hierarquia projeto/local/edificação [V]. O IfcOpenShell é LGPL-3.0 (`reuso-opensource.md`) [V no repositório do Fenkra].
- **Fenkra hoje:** BIM é **não-objetivo** (visão.md §6), mas o módulo Estrutural (perfis, soldas) pode se beneficiar de IFC.
- **Melhor prática para o Fenkra:** P3, fase posterior. Importação de IFC como **referência somente leitura** (geometria via IfcOpenShell → forma OCCT), e exportação de perfis estruturais como `IfcBeam/IfcMember` com perfil e material (valor para escritórios de estrutura metálica). Teste: `ifc_viga_i_ida_e_volta_conserva_perfil`.

---

### Escolha das bibliotecas: OpenCascade × comerciais × abertas (CATIA/SW/Inventor: motores próprios + Datakit/Spatial/HOOPS / outros: Onshape (Parasolid), FreeCAD / Fenkra: OCCT)

- **O que faz:** define quem lê e grava cada formato.
- **Como os três fazem:** SolidWorks e Inventor combinam tradutores próprios, o kernel Parasolid/ACIS e componentes de terceiros [C]. O CATIA usa tradutores próprios (CGM) e o Datakit atua como parceiro em integrações [V parcial]. Onshape mostra que dá para ter um catálogo enorme de nativos com um motor central (Parasolid + tradutores) [V].
- **Panorama:**

  | Camada | O que cobre | Licença/custo | Fenkra |
  |---|---|---|---|
  | **OCCT 8.0.1 DE** | STEP, IGES, BREP, XBF, OBJ, STL, PLY(w), glTF, VRML [V] | LGPL-2.1 + exceção [V] | núcleo |
  | Rust abertos (`zip`+XML, `dxf`, `gltf`, `stl_io`/`tobj`) | 3MF, DXF, glTF, STL/OBJ [C] | a verificar (MIT/Apache típico) [C] | complementar |
  | IfcOpenShell | IFC | LGPL-3.0 [V] | futuro |
  | LibreDWG | DWG | **GPL-3.0**; escrita limitada [V] | só processo externo |
  | ODA (Teigha) | DWG/DGN/IFC | associação paga [V parcial] | descartar no núcleo |
  | HOOPS Exchange / CAD Exchanger / Spatial 3D InterOp / Datakit | nativos CATIA/SW/Inventor/Creo/NX/JT/Parasolid | comercial, preço não público [V] | plugin opcional |

- **Fenkra hoje:** `build.rs` liga todos os `TK*` do OCCT do vcpkg (`build.rs:38-44`), então os toolkits de DE já estão linkados. Só o `STEPControl_Reader/Writer` e o `StlAPI_Writer` são usados (`kernel.cpp:105-110`).
- **Melhor prática para o Fenkra:**
  - **Núcleo = OCCT DE_Wrapper** (um nó de configuração por formato, carregado de um recurso de texto, com prioridade de fornecedor [V]). Isso dá, de graça, uma superfície uniforme de leitura/gravação.
  - **Interface `Provedor`** própria no crate conversor: `id`, formatos, `ler(opcoes) -> Importado`, `gravar(modelo, opcoes) -> Relatorio`. OCCT é o provedor padrão; outros se registram (comerciais carregados como DLL em tempo de execução, nunca no binário principal).
  - **Licenças:** conferir a compatibilidade com a licença do Fenkra (ADR 0003 rejeitou GPL para o núcleo [V]). Tudo que for GPL fica em executável separado, chamado por linha de comando.
  - **Teste:** `provedor_ausente_gera_mensagem_clara`, `registro_de_provedores_lista_formatos`.

---

### Reparo na importação: healing, costura, tolerâncias e unidades (CATIA: Healing/Join, GSD / SW: Import Diagnostics, Check / Inventor: Repair, Heal Errors, Auto Stitch / outros: Creo Import DataDoctor [C], NX / Fenkra: ShapeFix e Sewing em superfícies)

- **O que faz:** corrige lacunas, faces inválidas, tolerâncias incoerentes e conversão de unidades logo depois da leitura, para obter um sólido utilizável.
- **CATIA V5 — no detalhe:** a opção "Join" do IGES define a tolerância de junção [V]; valores de tolerância do arquivo CATIA são frequentemente pequenos demais para outros sistemas (cerca de uma ordem de grandeza) [V, Altair]. O healing de superfícies é feito em Generative Shape Design (Healing / Join com tolerância e ângulo) [C]. Falhas de STEP sem sólido ou faltando faces ficam registradas pela IBM [V].
- **SolidWorks — no detalhe:**
  - **Import Diagnostics** procura faces com defeito e **lacunas (gaps)**, e oferece: *Atribuir curar tudo*, reparar faces, **Heal Gap** (ajusta faces vizinhas, senão costura uma face nova), **Remove Gap** (remove as faces adjacentes) e **Delete Face** como último recurso [V]. A ordem influencia o resultado ("Attempt to Heal All é dependente da ordem") [V]. Sucesso = mensagem verde "nenhuma face com defeito ou lacuna" [V].
  - Só roda quando a árvore tem apenas corpos importados (sem features) e o 3D Interconnect desligado ou dissolvido [V]. Para montagens: Avaliar > Verificar, selecionando cada falha para localizar o componente [V].
- **Inventor — no detalhe:** ambiente **Reparar**: *Find Errors* examina topologia, geometria e modelagem; *Heal Errors* usa uma **tolerância permitida** (e a ajuda recomenda *Analyze Selected Bodies* para prever quantos erros a tolerância corrige antes de aplicar) [V]. Na importação: *Check Parts during Load*, *Auto Stitch and Promote*, *Enable Advanced Healing* (permite pequenas alterações de superfície para costurar) e *Importar para ambiente de Reparo* [V]. Se Heal Errors falha, as ferramentas de superfície ou apagar e refazer a geometria [V].
- **Outros:** o guia STEP do OCCT afirma que o shape processing (ShapeFix) roda automaticamente depois da leitura e antes da gravação, usando `ShapeFix_Wire` (auto-interseção, conexões) e `ShapeFix_Edge::SameParameter`; as tolerâncias são ajustadas conforme a geometria local, podendo ficar maiores ou menores que a do arquivo; no modo Forced a forma pode sair inválida [V]. Um fórum do OCCT confirma que o *UnifySameDomain* só funcionou depois do round-trip STEP (por causa do shape processing) e que faces em cascas separadas sem aresta comum nunca se unem (precisa costurar antes) [V].
- **Fenkra hoje:**
  - Há ingredientes no kernel: `sewn(...)` e `close_surface` (`hybrid.inc:43, 304-335`, costura + tampas planas/preenchimento + `ShapeFix_Solid`), `ShapeFix_Shape` + `UnifySameDomain` (`hybrid.inc:160-164`), `unify` (`kernel.cpp:147`).
  - **Não** são aplicados ao resultado de `import_step`, e não há relatório de erros (`BRepCheck_Analyzer`), nem tolerância configurável por importação, nem unidade.
- **Melhor prática para o Fenkra:**
  - **Pipeline fixo, em estágios que gravam no relatório:** (1) unidades; (2) leitura com precisão do arquivo; (3) `ShapeAnalysis`/`BRepCheck_Analyzer` → lista de defeitos (faces, arestas, lacunas, cascas abertas, sólidos invertidos, volume < 0); (4) `ShapeFix_Shape` com tolerâncias; (5) costura (`BRepBuilderAPI_Sewing`) quando há cascas soltas; (6) fechamento em sólido (`ShapeFix_Solid`, como no Fenkra); (7) `UnifySameDomain` (opcional, "simplificar"); (8) reavaliação (mesma lista) e comparação de volume/área antes e depois.
  - **Níveis:** *Nenhum* (somente leitura), *Padrão* (ShapeFix do leitor), *Agressivo* (aumenta a tolerância em passos e mostra o quanto subiu, como a tolerância permitida do Inventor).
  - **Diagnóstico interativo:** lista de falhas (faces/lacunas) com realce na viewport e ações *Reparar / Remover / Costurar / Ignorar*, no padrão Editar/Substituir/Isolar/Desativar do ADR 0011 (princípio 1). Nunca "escolher outra geometria em silêncio".
  - **Unidades:** ler a unidade do arquivo; se for diferente da do Fenkra, **perguntar** (padrão: converter) e registrar. Aviso se o bounding box ficar absurdo (< 0,01 mm ou > 100 m).
  - **Testes:** `import_com_lacuna_aparece_no_diagnostico`, `heal_agressivo_nao_muda_volume_alem_de_1e-4`, `import_polegadas_converte_para_mm`, `cascas_soltas_costuram_em_um_solido`, `unir_faces_nao_remove_arestas_de_costura_da_peça_original`.

---

### Relatório de conversão e diagnóstico (CATIA: Update Diagnosis/Analyze / SW: What's Wrong, Import Diagnostics / Inventor: Translation Report, Design Doctor / outros: Onshape aba original / Fenkra: toast de uma linha)

- **O que faz:** explica o que foi lido, o que foi perdido e o que foi alterado.
- **CATIA V5:** não há relatório gráfico padrão da tradução; o resultado aparece na árvore e nos Geometrical Sets, e o diagnóstico é manual [C].
- **SolidWorks:** o Import Diagnostics lista falhas; a janela de resultado mostra "sem faces ou lacunas" [V]. Mensagens de entidades ignoradas na importação IGES [V].
- **Inventor:** o **relatório de tradução** pode ser embutido (ícone sob o nó 3rd Party; duplo clique para ver) ou salvo em disco (HTML) [V].
- **Outros:** o OCCT informa as entidades não traduzidas do IGES [V] e permite checar o modelo antes da transferência no leitor STEP [V].
- **Fenkra hoje:** `app.rs:889` mostra só "Exportado: caminho" ou "Falha ao exportar: …". A feature `MeshImport` devolve texto de status ("N triângulos → sólido", `lib.rs:4085, 4092`), que é o embrião de relatório.
- **Melhor prática para o Fenkra:**
  - **Estrutura `Relatorio`:** cabeçalho (arquivo, hash, formato, AP/versão, tamanho, tempo, versão do Fenkra/OCCT), unidades detectadas e aplicadas, contagens (produtos, instâncias, corpos, faces, arestas, superfícies, curvas, pontos), **o que foi ignorado** por categoria com contagem e exemplo, avisos de reparo (antes/depois), tolerâncias, resultado final por corpo (sólido/casca/malha) e links "ir para o corpo". Mensagens em três níveis: info, aviso, erro.
  - **Persistência:** embutido na peça (como o Inventor), em "Relatórios" na árvore do projeto, e exportável (HTML/JSON).
  - **Exportação também tem relatório:** o que foi levado/omitido, tamanho, hash do arquivo gerado, esquema usado.
  - **Teste:** `relatorio_lista_entidades_ignoradas`, `relatorio_serializa_e_reabre`, `relatorio_de_exportacao_traz_hash`.

---

### Reconhecimento de features após importar e modelagem direta (CATIA: Power Copy / Healing + "extract history" [C] / SW: FeatureWorks / Inventor: Direct Edit, Copy to / outros: NX e Solid Edge Synchronous Technology, Fusion direct, Creo Flexible Modeling [C] / Fenkra: nenhuma)

- **O que faz:** devolve parametria (furos, filetes, extrusões) a um corpo importado ou permite editá-lo sem histórico.
- **CATIA V5 — no detalhe:** o STEP chega sem histórico, o sólido fica como corpo isolado [V]. Um tutorial de comunidade descreve "extrair histórico" de peças importadas com ferramenta de reconhecimento de features, mas não pude ler os passos [V parcial]. Power Copy não instancia com sólido isolado como entrada [V, IBM].
- **SolidWorks — no detalhe (FeatureWorks):**
  - Reconhece features em corpos sólidos importados e os transforma em features nativas editáveis [V].
  - Modos: **automático**, **interativo** (escolhe faces, tipo de feature e marca "Recognize similar"/"Recognize pattern") e **passo a passo** (reconhece, salva e volta depois) [V].
  - Furos simples, cônicos, escareados, com rebaixo e contrafuro nos modos automático e interativo; lofts e varreduras só interativos; padrões de sketch só interativos [V]. Opção "Chain fillet faces" nos filetes [V].
  - Multicorpo: um corpo por vez, pelo menu do corpo importado [V].
  - Features não suportadas: apagar (com os dependentes) ou mapear como *Thicken* (mantém a geometria; só para certos lofts, varreduras e furos) [V]. Limites: imprints parciais e padrão de padrão não são reconhecidos [V].
- **Inventor — no detalhe:** Direct Edit e "Copy to" funcionam em peça importada [V]; no AnyCAD em peça, a edição direta da geometria importada é permitida (mas não o Solid Edit nem o Reparo em Referência) [V]. Reconhecimento automático de histórico: [C] limitado (conversão para chapa metálica reconhece dobras [C]).
- **Outros:** NX e Solid Edge usam a **Synchronous Technology** para extrair intenção de projeto de dado importado e editar sem histórico [V parcial]. Fusion edita corpos importados direto [C].
- **Fenkra hoje:** nada. Existem features paramétricas próprias (furo, filete, chanfro, casca) e `smart-fastener.md` registra a limitação conhecida: *"não reconhece furos em corpos importados quando o cilindro não é um recurso separado"* [V no repositório].
- **Melhor prática para o Fenkra:**
  - **Fase 1 — analisar, não reescrever:** varrer as faces do corpo importado (cilindros côncavos fechados = furos; planos paralelos = patamares; toroides tangentes = filetes; cones coaxiais = escareados) e **marcar** (anotação, não feature): "Furo Ø8 passante" selecionável pelo Assistente de fixação (corrige o gargalo de `smart-fastener.md`). Também alimenta DFM e a tabela de furos do desenho.
  - **Fase 2 — conversão a pedido:** "Reconhecer features" cria features Fenkra (furo, filete) **sobre o corpo importado**, no padrão do FeatureWorks: automático, interativo e passo a passo, com a opção de apagar/mapear as não suportadas. Cada feature reconhecida remove a geometria do corpo e a recria.
  - **Fase 3 — edição direta:** mover/apagar face, offset de face e raio de filete sem histórico (sobre `BRepOffsetAPI_*`/`BRepAlgoAPI`, já usados no kernel). Evitar prometer histórico completo.
  - **Regra:** o corpo importado fica **"travado" com selo da origem**; as features reconhecidas ficam depois dele na árvore e marcadas como "derivadas da importação".
  - **Testes:** `reconhece_furo_passante_em_cilindro_importado`, `reconhece_filete_constante`, `nao_reconhece_filete_variavel_e_reporta`, `corpo_reconhecido_mantem_volume`, `fixador_sobre_furo_reconhecido_gera_gdl_zero`.

---

### Destino da importação: peça, montagem, superfícies e estrutura (CATIA: Product/Part/Geometrical Set, Insert Existing Component / SW: peça multicorpo × montagem, Opções gerais / Inventor: "Importar montagem como peça única", destino dos arquivos / outros: Onshape zip / Fenkra: coletores de corpos, `fenkra-montagem`)

- **O que faz:** decide em que estrutura de documento o conteúdo lido se transforma.
- **CATIA V5:** STEP de montagem vira Product com Parts; superfícies de IGES vão para Geometrical Sets; sólidos para Part Bodies [V parcial]. Insert Existing Component traz o arquivo para uma montagem [C].
- **SolidWorks:** a opção "ignorar estrutura de montagem" cria peça multicorpo; "múltiplos corpos como peças" divide [V]. O Enable Filter seleciona componentes [V].
- **Inventor:**
  - *Importar montagem como peça única* (compósito único ou peça de múltiplos sólidos) [V].
  - *Salvar componentes durante o carregamento*, *Pasta de destino dos componentes* e *Montagem de topo em pasta separada* [V].
  - Reorganização dos nós de navegador (Rename Browser Nodes: arquivo × PN) [V].
- **Outros:** Onshape cria Part Studios/Assemblies a partir do arquivo [V].
- **Fenkra hoje:** `Document.collectors` (corpos) e `fenkra-montagem::Assembly::add_component(name, kind, placement, parent)` (`montagem/lib.rs:536`). ADR 0011 (princípio 5) manda: "componente novo exige nome e pasta".
- **Melhor prática para o Fenkra:**
  - **Opções de destino** (padrão conforme o conteúdo): *Um corpo → peça nova*; *vários corpos, mesmo produto → peça multicorpo (coletores)*; *vários produtos → montagem + um arquivo de peça por produto, no projeto, com PN gerado pelo motor de PN*; *só superfícies/curvas → referências na peça*; *"montagem como uma peça só"* para simplificar.
  - **Antes de gravar:** mostrar a lista "Será criado: N arquivos em <pasta>" e permitir renomear (como o Inventor); nunca gravar escondido.
  - **Instâncias:** produtos repetidos viram uma peça + N posições (sem duplicar geometria).
  - **Testes:** `import_montagem_cria_n_pecas_e_posicoes`, `import_instancias_repetidas_compartilham_peca`, `import_exige_pasta_e_nome`, `import_como_peça_unica_gera_multicorpo`.

---

### Conversão em lote (CATIA: Batch Monitor BATCH-DXF-IGES-STEP / SW: Task Scheduler (Export Files, Convert Files), PDM Convert task / Inventor: Task Scheduler + Vault / outros: Onshape API / Fenkra: nenhum)

- **O que faz:** converte muitos arquivos sem interação, com fila, registro e reexecução.
- **CATIA V5 — no detalhe:** ferramenta `Batch Monitor` (pasta Tools do CATIA, ou `CATUTIL.exe`) com a opção BATCH-DXF-IGES-STEP; **escolhe-se o formato de saída antes de adicionar os arquivos** (não dá para mudar depois sem remover o arquivo) [V]. Restrições conhecidas: com o **cache ligado** o STEP em lote pode sair errado (desligar o cache) [V, IBM]; licenças STEP selecionadas ao mesmo tempo no lote e no interativo conflitam (usar a ST1 compartilhada no lote e deixar as opções interativas) [V, IBM]; relato de peças faltando em montagens grandes no lote [V, GrabCAD].
- **SolidWorks — no detalhe:** o **Task Scheduler** (aplicativo separado) tem a tarefa *Export Files* (DXF, DWG, IGES, PDF, STEP AP203/AP214, JPEG; desenhos para DXF/DWG/PDF, peças e montagens para IGES/STEP), com arquivos ou pastas (com exclusão de subpastas), saída ao lado do original ou em outra pasta, agendamento único/diário/semanal/mensal, pasta de trabalho e tempo limite; cada arquivo vira uma subtarefa [V]. *Convert Files* atualiza arquivos de versões antigas (qualquer licença); as demais exigem Professional/Premium [V]. A tarefa **Convert do PDM** gera STEP/IGES/DWG/DXF/PDF/ACIS/Parasolid/STL/VRML/eDrawings com opções avançadas por formato [V].
- **Inventor — no detalhe:** tarefas em lote por Task Scheduler/Vault [C].
- **Outros:** Onshape expõe tradução por API [V parcial].
- **Fenkra hoje:** nada. `RF-DES-045` pede "exportar várias folhas em lote" (`desenho.md:60`), e `RF-GPR-006` exportar estrutura para CSV (`gestao-projeto.md:15`); só o CSV existe (`fenkra-project/src/lib.rs:267`).
- **Melhor prática para o Fenkra:**
  - **Entidade `Lote`:** entradas (arquivos/pastas/documentos do projeto/seleção da árvore), perfil de saída (formato + opções), regra de nome (`{PN}_{REV}.{ext}`), pasta de saída, política para existente (pular/sobrescrever/versionar), **simulação** (lista o que faria).
  - **Execução:** fila em *threads* para formatos seguros (STEP por instância própria [V], glTF/STL/OBJ/PLY/VRML seguros [V]) e **sequencial para IGES** [V], ou **processos filhos** (um por arquivo, com tempo limite e memória máxima), de modo que uma falha do OCCT não derrube o app.
  - **Resultado:** uma tabela por arquivo (ok/aviso/erro, duração, saída, hash), relatório consolidado (CSV/HTML), "reexecutar só os que falharam" e **idempotência** (hash de entrada + perfil → pular se igual).
  - **Quatro modos:** exportar documentos do projeto, importar pasta de neutros para o projeto, converter neutro→neutro (ex.: STEP→glTF), e "gerar pacote de liberação" (STEP+PDF+DXF da revisão).
  - **Testes:** `lote_continua_apos_falha_de_um_arquivo`, `lote_iges_nao_paralelo`, `lote_idempotente_pula_inalterado`, `lote_simulacao_nao_grava`, `lote_respeita_tempo_limite`.

---

### Vínculo com o original: Referência × Converter e atualização (CATIA: Insert Existing Component, links externos (->) / SW: 3D Interconnect Open/Insert, atualizar, quebrar vínculo / Inventor: AnyCAD Referência/Converter, ícone Atualizar / outros: Onshape retradução / Fenkra: ADR 0010 (links ->, ->?, ->*, ->x))

- **O que faz:** mantém ou corta a ligação entre o modelo no CAD e o arquivo de origem.
- **CATIA V5:** não há vínculo associativo com STEP; a importação cria uma cópia (isolada) [C]; para CATPart usa links entre documentos.
- **SolidWorks — no detalhe:** com o 3D Interconnect, **Abrir** cria vínculo associativo; ao mudar o original, atualiza o modelo e as features seguintes; **quebrar vínculo** converte em peça comum; **Inserir** põe sem converter [V]. A coluna "Update Available?" da tabela de formatos diz quais nativos atualizam [V].
- **Inventor — no detalhe:** **Referência** monitora o arquivo e mostra um ícone Atualizar na barra de acesso rápido; **Converter** não tem vínculo e é independente [V]. STEP referenciado: cada atualização é uma nova tradução — risco para as features construídas sobre ele [V]. Alterações no Inventor **não voltam** ao STEP, que deve ser reexportado [V, fórum].
- **Outros:** o Onshape mantém a aba do arquivo original e permite retraduzir [V].
- **Fenkra hoje:** o ADR 0010/0011 já define estados de link para referências externas entre peças (`->`, `->?`, `->*`, `->x`) e travar/destravar/quebrar [V]. A feature `MeshImport` guarda um `path` sem hash nem estado.
- **Melhor prática para o Fenkra:**
  - **Reutilizar os estados do ADR 0011** para o arquivo de origem: `->` sincronizado, `->?` origem mudou (hash diferente), `->*` travado, `->x` origem ausente.
  - **Modelo de dados:** feature `CorpoImportado { origem: OrigemRef { documento_id, hash, formato, versão_AP, unidades, opções }, corpo_embutido: BREP }`. O corpo vive **dentro do documento** (já há `to_brep` no kernel), então o arquivo abre sem a origem; a origem é só para atualizar.
  - **Atualizar:** reimportar a origem, recalcular o relatório e **recompor as referências por correspondência** (nome do produto/ID STEP primeiro; assinatura geométrica das faces depois, ligada ao naming topológico do ADR 0004). Face sem par → estado "falhou" com diagnóstico e as ações do ADR 0011 (Editar/Substituir/Isolar/Desativar), nunca outra face em silêncio.
  - **Padrão por formato:** STEP/IGES/neutros → *Converter* (como a ajuda do Inventor recomenda para STEP [V]); malha → *Converter* com origem embutida; *Referência* só para formatos com identidade estável.
  - **Quebrar vínculo:** vira corpo comum, mantém o selo "importado de X em data".
  - **Testes:** `origem_alterada_marca_arrow_interrogacao`, `atualizar_preserva_features_com_faces_estaveis`, `atualizar_face_removida_falha_com_diagnostico`, `quebrar_vinculo_mantem_geometria`, `abrir_sem_origem_funciona`.

---

### Integração com save management, PDM e VPM (CATIA: Save Management, Propagate Directory / SW: PDM (referências de 3D Interconnect, tarefa Convert), Salvar tudo / Inventor: Vault + AnyCAD, Component Destination Folder / outros: Teamcenter, Onshape / Fenkra: `fenkra-project`, ADR 0009, `plm-vpm.md`)

- **O que faz:** garante que o arquivo importado, seus derivados e as exportações entrem no controle de revisão, e que nada seja gravado às escondidas.
- **CATIA V5:** o **Save Management** lista todos os documentos modificados/novos e permite escolher pasta e propagar o diretório; ao importar uma montagem, os componentes novos passam por essa janela [C]. No ENOVIA/VPM os documentos importados são carregados como novos objetos [C].
- **SolidWorks — no detalhe:** o PDM detecta no **check-in** as referências a arquivos não-SolidWorks (CATIA V5, Creo e outros via 3D Interconnect) e as lista como referências CAD, inclusive a hierarquia (a submontagem Inventor liga ao pai e aos próprios filhos) [V]. A tarefa **Convert** gera STEP/PDF/DXF etc. a partir de documentos do cofre e guarda o derivado; as opções avançadas são por formato [V]. O vínculo fica no PDM; o nativo permanece como referência. (Execução exata da tarefa Convert: gatilho, nome, revisão — não confirmada nesta pesquisa.)
- **Inventor — no detalhe:** o Vault gerencia terceiros "no contexto do modelo Inventor" via AnyCAD; o add-in faz check-in com referências e estrutura; não suporta todos os formatos [V]. Como o Vault versiona/revisa o arquivo original anexado **não ficou confirmado** [V parcial].
- **Outros:** na prática, sistemas PLM guardam o neutro importado como **documento de origem** (não editável) ligado ao item gerado [C].
- **Fenkra hoje:**
  - `fenkra-project`: `add_document`, `save`, `release`, `duplicate`, versões imutáveis por hash `blake3` em `.versoes/<hash>.fkp` (`lib.rs:176-260`); `DocEntry` com `pn`, `state`, `versions`, `revisions`. **Só guarda documentos `.fkp`**.
  - `plm-vpm.md`: modelo `DOCUMENT` com `doc_type` (inclui `step`, `pdf`), `hash`, e `CAD_LINK` com `link_type` (contextual, publication, external_ref).
  - **Nenhum tipo de vínculo "importado de" / "derivado de", nenhum armazenamento de originais, nenhuma exportação registrada.**
- **Melhor prática para o Fenkra:**
  - **Importação registra:** (1) o **original** como documento de tipo `origem_neutra` (conteúdo no armazém por hash, imutável, com formato/versão/unidade e o relatório); (2) os **documentos Fenkra** gerados (peça/montagem) com PN pelo motor de PN; (3) `CAD_LINK` do tipo **`derivado_de`** (peça → origem) com os metadados de opções. Mesma janela "Salvar tudo" (B56): lista de arquivos novos com nome e pasta, motivo "importação de X".
  - **Exportação registra:** o arquivo gerado como documento `step`/`pdf`/`dxf`/`stl`/`glb` ligado à **revisão** (campo `revision_id` do `plm-vpm`), com hash e perfil usado; `CAD_LINK` **`exportado_de`**. Estado **obsoleto** quando a revisão fonte muda (hash difere), como o `dirty_since_release` já faz (`lib.rs:88`).
  - **Liberar revisão → perfil de liberação** (STEP AP242 + PDF do desenho + DXF da planificação), gerando o pacote em lote e anexando-o à revisão — equivalente à tarefa Convert do PDM [V], porém embutido. Renomeação automática `{PN}_{REV}`.
  - **Revisão de importado:** documento importado nasce "Em trabalho" e herda a rastreabilidade (hash da origem). Reimportar uma versão nova da origem gera **nova versão** (não nova revisão); liberar continua sendo ato do usuário (ADR 0009).
  - **VPM servidor (futuro):** `doc_type` e `CAD_LINK` já comportam; o conversor roda no **cliente** (ou num serviço de conversão) e envia o resultado + relatório.
  - **Regras:** o original nunca é sobrescrito; arquivos duplicados (mesmo hash) são compartilhados; limpar versões não marcadas continua valendo (ADR 0009).
  - **Testes:** `importacao_guarda_original_por_hash`, `importar_o_mesmo_arquivo_duas_vezes_nao_duplica_original`, `export_registra_documento_ligado_a_revisao`, `export_fica_obsoleto_quando_revisao_muda`, `liberar_gera_pacote_de_perfil`.

---

### Módulo conversor (crate) e execução isolada (CATIA: CAA tradutores / SW: add-ins de tradução / Inventor: Translators / outros: HOOPS, CAD Exchanger / Fenkra: proposta `fenkra-conversor`)

- **O que faz:** concentra formatos, opções, relatórios, lote e a ponte com o projeto num único lugar testável, independente da interface.
- **CATIA/SW/Inventor:** os tradutores são módulos separados do núcleo (o PDM/Task Scheduler os chamam por fora) [C]. O Task Scheduler do SW é aplicativo separado, que roda fora da janela do CAD [V].
- **Outros:** HOOPS e CAD Exchanger oferecem API em C++/C#/Java/Python/JS e trabalham fora da interface do CAD [V parcial].
- **Fenkra hoje:** funções soltas em `fenkra-kernel` (C++), `fenkra-solidos::export`, `fenkra-solidos::mesh_io`, `fenkra-desenho::export`, sem interface comum.
- **Melhor prática para o Fenkra — arquitetura proposta:**
  - **Crate `fenkra-conversor`** (depende de `fenkra-core`, `fenkra-kernel`, `fenkra-project`; **não** depende do app):
    - `formato.rs`: enum `Formato` e tabela (extensões, magic bytes, categoria, direção).
    - `opcoes.rs`: `OpcoesImport` e `OpcoesExport` por formato, `serde`, com presets nomeados.
    - `relatorio.rs`: `Relatorio`, `Aviso`, `EntidadeIgnorada`.
    - `provedor.rs`: `trait Provedor { fn formatos(); fn ler(&self, caminho, &OpcoesImport) -> Result<ModeloNeutro, ErroConv>; fn gravar(&self, &ModeloNeutro, caminho, &OpcoesExport) -> Result<Relatorio,…>; }`.
    - `occt.rs`: provedor OCCT (STEP/IGES/BREP/glTF/OBJ/STL/PLY/VRML) por `DE_Wrapper`.
    - `malha.rs`, `dxf.rs`, `tres_mf.rs`: formatos próprios/abertos.
    - `reparo.rs`: pipeline de reparo e diagnóstico.
    - `lote.rs`: fila, perfis, simulação, idempotência.
    - `projeto.rs`: registro no `fenkra-project` (originais, `derivado_de`, `exportado_de`).
  - **`ModeloNeutro`:** árvore de produtos (nome, id, instâncias com matriz), corpos (forma + tipo sólido/casca/malha), aparências, camadas, PMI, propriedades. É o que o Fenkra converte de/para `Document`/`Assembly`.
  - **Ponte C++ nova:** `read_xcaf(path, opts) -> XcafHandle`, `write_xcaf(handle, path, opts)`, mantendo o `import_step/export_step` atuais como compatibilidade até migrar. Em OCCT 8.0, STEP é *stream-based* e thread-safe por instância [V].
  - **Processo filho `fenkra-conv.exe`** (mesmo crate, binário): recebe um *job* JSON (stdin) e devolve relatório JSON; o app aplica tempo limite e memória máxima. Motivos: IGES não thread-safe [V], OCCT pode travar em arquivos hostis [C], atende o lote e é o único jeito seguro de chamar **LibreDWG/GPL** [V ADR].
  - **Nada de `unwrap`** em caminhos de E/S; erros de OCCT (`Standard_Failure`, derivado de `std::exception` no 8.0 [V]) já são capturados por `guard(...)` no kernel (`kernel.cpp:988`).
  - **Testes:** o conjunto de testes do crate usa o **corpus B53** + um corpus de arquivos neutros pequenos (STEP de 3 CADs, IGES de superfícies, STL, DXF) com resultados esperados (contagem, volume, bbox).

---

## 3. Requisitos levantados

Prioridades: **P0** quebra o uso · **P1** importante · **P2** desejável · **P3** futuro.

### 3.1 Funcionais (RF-CNV-nn)

| ID | Requisito | Prioridade | Critério de aceite testável |
|---|---|---|---|
| RF-CNV-01 | O menu **Arquivo** tem os itens **Importar…** e **Exportar…** separados, com submenus por categoria (3D sólido, 3D malha, 2D, dados) e a janela de **Conversão em lote…**. | P1 | Teste de UI (captura): o menu lista os 3 itens; "Exportar STEP…/STL…" soltos deixam de existir. |
| RF-CNV-02 | **Exportar** valida o formato pela extensão escolhida e recusa extensão desconhecida; nunca grava STEP com outro sufixo. | **P0** | `export_ext_desconhecida_erra`: `.igs` sem provedor → erro; `.stl` → STL; `.step/.stp` → STEP. |
| RF-CNV-03 | **Importar STEP** (AP203/214/242) pelo Fenkra, com relatório, via `STEPCAFControl`: estrutura, nomes, cores, camadas, props. | P0 | Importar STEP do corpus: volume e bbox ±1e-6 relativo; N produtos = N peças; nomes e cores preservados. |
| RF-CNV-04 | A importação detecta o formato por conteúdo e avisa se a extensão diverge. | P2 | `detecta_formato_por_conteudo`: STEP renomeado `.dat` abre; texto qualquer é rejeitado com mensagem. |
| RF-CNV-05 | Diálogo de importação com *Destino*, *Modo* (Referência/Converter), *O que importar*, *Unidades*, *Reparo*, *Avançado*, memorizando as últimas opções por formato. | P1 | Reabrir o diálogo restaura as opções; mudar o formato troca o painel. |
| RF-CNV-06 | A importação lê a **unidade** do arquivo, converte para a unidade do documento e permite forçar; avisa em tamanhos absurdos. | **P0** | STEP em polegadas → peça de 25,4 mm vira 25,4 mm; bbox < 0,01 mm ou > 100 m gera aviso. |
| RF-CNV-07 | **Reparo na importação** em níveis Nenhum/Padrão/Agressivo, com estágios, comparação de volume/área antes/depois e lista de defeitos. | P1 | Peça com lacuna do corpus → defeitos listados; `Padrão` fecha; volume muda < 1e-4 relativo. |
| RF-CNV-08 | **Diagnóstico interativo** de importação (faces/lacunas com realce) com ações Reparar/Remover/Costurar/Ignorar. | P2 | Seleção de item destaca a geometria; ação altera o relatório; sem ação, nada muda em silêncio. |
| RF-CNV-09 | **Relatório de conversão** por importação e por exportação (formato, hash, unidades, contagens, ignorados, avisos, tempo), embutido e exportável (HTML/JSON). | P1 | Relatório reabre com o documento; lista ≥ 1 item ignorado no caso de teste de PMI não suportado. |
| RF-CNV-10 | **Exportar STEP via XDE** com protocolo selecionável (AP203/214/242), nomes, cores, camadas, estrutura da montagem, propriedades de validação e metadados Fenkra. | P0 | Ida e volta com 2 instâncias: 1 definição + 2 posições; cor de face; PN no cabeçalho; volume validado. |
| RF-CNV-11 | **Exportar montagem** (hoje inexistente) em STEP com estrutura ou achatada, e em glTF/3MF. | P1 | Montagem do corpus (3 níveis) reabre com a mesma árvore e matrizes (±1e-9). |
| RF-CNV-12 | **PMI semântico** (cotas, tolerâncias, datums) e de apresentação exportados em AP242 e reimportados. | P2 | Cota Ø10 ±0,1 e um datum voltam como PMI do Fenkra com valor e alvo corretos. |
| RF-CNV-13 | **IGES**: importar como superfícies/curvas em Referências; exportar em modo BRep ou Faces; unidades; cor/camada opcionais. | P1 | IGES de superfícies → superfícies; costura+fechar → sólido; unidade em polegadas converte. |
| RF-CNV-14 | **BREP** (OCCT) importar/exportar. | P2 | Ida e volta bit-a-bit equivalente em volume/área. |
| RF-CNV-15 | **STL**: exportação com presets de qualidade, binário padrão, unidade, escopo, malha fechada verificada; importação com unidade e tolerância de costura relativa. | P1 | STL binário < 30% do ASCII; sem arestas abertas; unidade honrada. |
| RF-CNV-16 | **3MF** exportação e importação (unidade, cor, vários objetos, transformações). | P2 | Ida e volta 3MF preserva N objetos, cor e unidade. |
| RF-CNV-17 | **OBJ / PLY / glTF(GLB) / VRML**: OBJ e glTF import/export, PLY só export, VRML export. | P2 | glTF reabre com nós e cores; PLY é aceito por visualizador de referência. |
| RF-CNV-18 | **DXF**: ler para sketch (linha, arco, círculo, polilinha, spline, texto) com mapeamento de camadas e unidade; gravar R2000+ (R12 como opção). | P1 | DXF do corpus gera sketch com comprimentos ±1e-6; sem `$INSUNITS` o app pergunta. |
| RF-CNV-19 | **DWG** apenas por **conversor externo opcional** (processo), nunca embutido. | P3 | Sem o executável configurado: mensagem de orientação; com ele: converte DWG→DXF e abre. |
| RF-CNV-20 | **Nativos** (CATPart, SLDPRT, IPT, Creo, NX): interface de provedor pronta; sem provedor, orientação de exportação neutra por CAD. | P2 | `arquivo_nativo_sem_provedor_orienta`: mensagem cita STEP AP214/AP242 e as opções. |
| RF-CNV-21 | **Parasolid / ACIS / JT / PRC-U3D / IFC / VRML**: mapeados na tabela de formatos como "não suportado", "plugin" ou "futuro", com mensagem clara. | P3 | Tabela de formatos testada; cada item aponta o caminho alternativo. |
| RF-CNV-22 | **Reconhecimento de furos/filetes** em corpos importados (anotação), alimentando o Assistente de fixação e a tabela de furos. | P2 | Furo Ø8 passante importado reconhecido; fixador entra com GDL = 0 (B62). |
| RF-CNV-23 | **Reconhecer features** (a pedido, automático/interativo/passo a passo) com opção de apagar/mapear as não suportadas. | P3 | Furo reconhecido vira feature `Furo` editável; volume do corpo igual. |
| RF-CNV-24 | **Edição direta** de corpo importado (mover/apagar face, offset). | P3 | Offset de face em corpo importado altera a medida esperada sem erro de topologia. |
| RF-CNV-25 | **Destino da importação**: peça nova, multicorpo, montagem + peças, referências (superfícies), inserir na peça/montagem ativa. | P1 | Casos do corpus criam a estrutura esperada; lista de arquivos antes de gravar. |
| RF-CNV-26 | A importação de montagem **compartilha instâncias** (1 peça + N posições). | P1 | STEP com 4 parafusos iguais cria 1 peça e 4 instâncias. |
| RF-CNV-27 | **Vínculo com o original**: estados `->`, `->?`, `->*`, `->x`; Atualizar, Travar, Quebrar. | P1 | Alterar o STEP original → `->?`; Atualizar reaplica; Quebrar mantém geometria. |
| RF-CNV-28 | O corpo importado é **embutido** no documento (BREP) e o documento abre sem a origem. | P0 | Apagar o STEP original → o `.fkp` abre idêntico; `->x` apenas informativo. |
| RF-CNV-29 | **Atualizar reimportando** recompõe referências por nome/ID do produto e assinatura geométrica; face sem par → estado "falhou" com diagnóstico (nunca outra face). | P1 | Face removida na nova versão → feature dependente "falhou"; ações Editar/Substituir/Isolar/Desativar. |
| RF-CNV-30 | **Conversão em lote** com perfil, regra de nome, pasta, política de existente, simulação, idempotência e relatório consolidado. | P1 | Lote de 10 arquivos com 1 inválido: 9 convertidos, relatório mostra o erro; reexecução pula os iguais. |
| RF-CNV-31 | O lote roda formatos thread-safe em paralelo e **IGES em série** (ou processos). | P1 | `lote_iges_nao_paralelo`: execução concorrente sem corrupção. |
| RF-CNV-32 | **Registro no projeto**: original guardado por hash, `derivado_de`, `exportado_de`, documentos novos pela janela "Salvar tudo" (B56). | P1 | Importar duas vezes o mesmo arquivo → um único original no armazém. |
| RF-CNV-33 | **Exportação registrada** como documento ligado à revisão, marcada obsoleta se a revisão muda. | P2 | Liberar A, exportar STEP; alterar e liberar B → STEP de A continua; STEP "atual" marcado obsoleto até regerar. |
| RF-CNV-34 | **Perfil de liberação** (conjunto de exportações) gerado ao liberar a revisão, nome `{PN}_{REV}`. | P2 | Liberar gera STEP+PDF+DXF no projeto e os liga à revisão. |
| RF-CNV-35 | O módulo conversor é um **crate** com a interface `Provedor`, opções serializáveis e CLI/processo filho. | P1 | `cargo test -p fenkra-conversor` roda sem o app; o binário `fenkra-conv` processa um job JSON. |
| RF-CNV-36 | Mensagem de **orientação** quando o formato não é suportado, com a rota alternativa (ex.: "exporte STEP AP214 do SolidWorks"). | P2 | Cada formato desconhecido tem texto de ajuda na tabela. |
| RF-CNV-37 | **Exportar DXF/PDF/SVG do desenho** passa a usar o mesmo diálogo e perfis do conversor, em lote por folhas (RF-DES-045). | P2 | Exportar 3 folhas → 3 arquivos (DXF/SVG) ou 1 PDF multipágina, por perfil. |
| RF-CNV-38 | **Reabrir verificação** opcional após exportar (relê e compara volume/área/contagem). | P3 | Ativar: exportação com diferença de volume > tolerância acusa erro. |
| RF-CNV-39 | **Textos e caminhos** com acentos e espaços funcionam em importação/exportação (incl. nomes como `C:\Users\João\…`). | **P0** | `export_caminho_com_acento` e `import_caminho_com_acento` passam no Windows. |
| RF-CNV-40 | A interface mostra **progresso e cancelamento** em conversões longas. | P2 | Importar arquivo grande exibe barra e o botão Cancelar encerra em < 2 s sem travar o app. |

### 3.2 Não funcionais (RNF-CNV-nn)

| ID | Requisito | Prioridade | Critério de aceite testável |
|---|---|---|---|
| RNF-CNV-01 | **Isolamento:** falha, travamento ou estouro de memória do OCCT durante a conversão não derruba o app. | P0 | Arquivo hostil/corrompido do corpus → erro tratado; o processo principal continua. |
| RNF-CNV-02 | **Tempo limite e memória máxima** por job, configuráveis. | P1 | Job com limite de 1 s sobre arquivo grande termina com "tempo esgotado". |
| RNF-CNV-03 | **Thread-safety:** STEP com parâmetros por instância; IGES serializado. | P1 | Teste de estresse com 8 jobs paralelos STEP e IGES sem corrupção. |
| RNF-CNV-04 | **Determinismo:** mesma entrada e opções → mesmo relatório e mesmo hash de saída (exceto carimbos de tempo). | P1 | Duas execuções geram hash igual, com metadados de tempo excluídos. |
| RNF-CNV-05 | **Fidelidade geométrica:** ida e volta STEP preserva volume, área e centroide dentro de 1e-6 relativo (sólidos). | P0 | Corpus B53 roda no CI; falha acima da tolerância. |
| RNF-CNV-06 | **Licenças:** nenhum componente GPL no binário; GPL só em processo separado; componentes comerciais como DLL opcional. | P0 | Auditoria de dependências (cargo-deny + lista manual) sem GPL no binário. |
| RNF-CNV-07 | **Desempenho:** importar um STEP de 50 MB em < 60 s em máquina de referência, sem congelar a interface. | P2 | Medição no corpus de desempenho; thread da UI sem bloqueio > 100 ms. |
| RNF-CNV-08 | **Tamanho de arquivos:** STL binário por padrão; STEP sem duplicar entidades. | P2 | STL binário < 30% do tamanho ASCII; STEP com `write.surfacecurve.mode` desligado menor que ligado. |
| RNF-CNV-09 | **Segurança de entrada:** limites de tamanho/profundidade/contagem, sem `unwrap`, sem leitura fora do projeto fora de diálogo. | P1 | Fuzz simples (arquivos truncados/aleatórios) não causa pânico. |
| RNF-CNV-10 | **Rastreabilidade:** todo arquivo importado/exportado carrega versão do Fenkra, versão do OCCT, hash da origem e opções. | P1 | Cabeçalho STEP e relatório trazem os campos. |
| RNF-CNV-11 | **Idioma:** mensagens, relatórios e diálogos em português, com i18n para inglês (padrão `tr()`). | P2 | Todas as strings novas passam pelo `tr()` e têm tradução em `i18n.rs`. |
| RNF-CNV-12 | **Cobertura de testes:** cada formato suportado tem pelo menos 1 teste de ida e volta e 1 de erro, e roda no corpus em cada commit (princípio 7, ADR 0011). | P1 | Relatório de cobertura por formato lista 100% com teste nomeado. |

---

## 4. Itens de backlog propostos

IDs provisórios `CNV-xx` (o lead numera). Tamanhos: P ≤ 80 k tokens · M ~100–250 k · G > 250 k, no padrão do backlog atual.

| Título | O que muda | Prioridade | Tam. | Dependências |
|---|---|---|---|---|
| **CNV-01 Corrigir exportação atual** | `export()` valida a extensão; erro para desconhecidas; teste de caminho com acento; STL binário; presets de qualidade (preparação para B61). | P0 | P | — |
| **CNV-02 Crate `fenkra-conversor` (esqueleto)** | Formato/opções/relatório/trait `Provedor`; migra `fenkra_solidos::export` e `mesh_io`; sem mudar o comportamento. | P1 | M | CNV-01 |
| **CNV-03 Ponte XDE (leitura/gravação STEP)** | `read_xcaf/write_xcaf` em C++; `STEPCAFControl`; parâmetros por instância; `ModeloNeutro`. | P0 | G | CNV-02 |
| **CNV-04 Importar STEP/BREP na interface** | Diálogo de importação; unidades; destino peça/multicorpo; relatório; cor e nomes. | P0 | G | CNV-03, B56 |
| **CNV-05 Menu Importar/Exportar e diálogos por formato** | Arquivo ▸ Importar…/Exportar…; painel de opções (suporte → modo → parâmetros → avançado); memória de opções. | P1 | M | CNV-02 |
| **CNV-06 Reparo e diagnóstico de importação** | Pipeline (check → ShapeFix → sewing → solid → unify); níveis; lista de defeitos com realce; antes/depois. | P1 | G | CNV-04, B59 |
| **CNV-07 Relatório de conversão** | Estrutura `Relatorio`; embutido no documento; HTML/JSON; ignorados por categoria. | P1 | M | CNV-02 |
| **CNV-08 Importar/exportar montagem** | Árvore de produtos, instâncias compartilhadas, arquivos de peça por produto no projeto, PN pelo motor; exporta montagem em STEP/glTF. | P1 | G | CNV-04, B54, B56 |
| **CNV-09 Origem e vínculo** | `CorpoImportado` com BREP embutido; estados `->/->?/->*/->x`; Atualizar/Travar/Quebrar; recomposição por nome e assinatura. | P1 | G | CNV-04, ADR 0010/0011 |
| **CNV-10 Integração com projeto e PDM** | Original por hash; `derivado_de`/`exportado_de`; exportação ligada à revisão; obsolescência; perfil de liberação. | P1 | M | CNV-04, `plm-vpm.md` |
| **CNV-11 Conversão em lote** | Entidade `Lote`; fila; processo filho `fenkra-conv`; simulação; idempotência; relatório consolidado. | P1 | G | CNV-02, CNV-07 |
| **CNV-12 IGES** | `IGESCAFControl`; superfícies → Referências; modos BRep/Faces; mutex; testes. | P1 | M | CNV-03 |
| **CNV-13 DXF 2D (leitura em sketch e escrita R2000+)** | Crate `dxf` (licença a verificar); camadas e unidade; spline; DXF do desenho por perfil. | P1 | M | CNV-02 |
| **CNV-14 Malhas: 3MF, glTF, PLY, OBJ export, STL relativo** | `zip`+XML para 3MF; `RWGltf` via DE; PLY (w); OBJ; malha embutida no projeto; costura relativa. | P2 | M | CNV-02 |
| **CNV-15 PMI e propriedades no STEP (AP242)** | Cotas/tolerâncias/datums semânticos + apresentação; metadados Fenkra; validação por reabertura. | P2 | G | CNV-03, `pmi-3d.md` |
| **CNV-16 Reconhecimento de furos e filetes** | Análise de faces; anotações selecionáveis; integração com fixadores e tabela de furos. | P2 | M | CNV-04, B62 |
| **CNV-17 Reconhecer features e edição direta** | Conversão a pedido em Furo/Filete; modos automático/interativo/passo a passo; offset/mover/apagar face. | P3 | G | CNV-16 |
| **CNV-18 Provedores externos (plugin)** | Carregamento de DLL de HOOPS/CAD Exchanger (nativos, JT, Parasolid); DWG por processo externo; mensagens de orientação. | P3 | G | CNV-02, decisão de licença |
| **CNV-19 3D PDF e IFC** | U3D com vistas de PMI; IFC (IfcOpenShell) como referência e vigas estruturais. | P3 | G | CNV-14, módulo Estrutural |
| **CNV-20 Corpus de arquivos neutros e testes de formato** | Corpus (STEP de 3 CADs, IGES, STL, DXF, 3MF, glTF) + testes de ida e volta no CI. | P0 | M | B53 |

Ordem sugerida: CNV-01 → CNV-20 (paralelo) → CNV-02 → CNV-03 → CNV-04/05/07 → CNV-06/12/13 → CNV-08/09/10 → CNV-11/14/15 → CNV-16 → CNV-17/18/19.

---

## 5. Fontes

**OpenCascade (documentação e releases)**
- https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/dox/user_guides/step/step.md
- https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/dox/user_guides/de_wrapper/de_wrapper.md
- https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/dox/user_guides/iges/iges.md
- https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/dox/user_guides/xde/xde.md
- https://occt3d.com/dev/content/open-cascade-technology-800-release/index.html (OCCT 8.0.0)
- https://dev.opencascade.org/content/open-cascade-technology-800-release-candidate-3
- https://dev.opencascade.org/doc/occt-8.0.0/refman/html/class_x_c_a_f_doc___dim_tol_tool.html
- https://dev.opencascade.org/content/missing-presentation-pmi-step-export-stepcafcontrolwriter
- https://dev.opencascade.org/content/shapeupgradeunifysamedomain-do-good-after-save-step-and-reload-it-backwhy
- https://dev.opencascade.org/doc/occt-8.0.0/refman/html/class_shape_upgrade___unify_same_domain.html

**CATIA V5**
- https://www.goengineer.com/blog/catia-step-exchange-3d-models-with-pmi-or-composites-data
- https://www.cati.com/blog/export-catia-v5-parts-with-pmi-using-step-ap242/
- https://www.technia.com/step-file-stp-step-not-importing-all-the-data-wanted/ (via resumo de busca)
- https://technia.com/blog/batch-convert-catpart-catproduct-catdrawing-to-3dxml-step-iges-dxf-dwg-or-reverse
- https://ibm.com/support/pages/apar/HE08498 · https://www.ibm.com/support/pages/apar/HD92639 · https://www.ibm.com/support/pages/node/5254617 (resumos de busca)
- https://2017.help.altair.com/2017/hm/to_import_geometry.htm · https://cad.grabcad.com/tutorials/extracting-part-history-from-imported-parts-in-catia-v5 (resumo de busca)

**SolidWorks**
- https://help.solidworks.com/2024/English/SolidWorks/Sldworks/t_reading_step_iges_acis_sw.htm (resumo de busca; página não renderizou)
- https://help.solidworks.com/2017/English/SolidWorks/sldworks/HIDD_OPTIONS_IMPORT_GENERAL.htm (idem)
- https://help.solidworks.com/2026/English/SolidWorks/Sldworks/c_sw_3d_interconnect.htm · https://help.solidworks.com/2024/English/SolidWorks/Sldworks/c_sw_3d_interconnect.htm (resumos)
- https://help.solidworks.com/2018/English/EnterprisePDM/FileExplorer/c_3d_interconnect_PDM.htm · https://www.javelin-tech.com/blog/2018/01/solidworks-pdm-3d-interconnect/ (resumo)
- https://help.solidworks.com/2021/English/SolidWorks/fworks/t_Recognizing_Features_Using_step_by_step.htm (FeatureWorks, resumo)
- https://www.goengineer.com/blog/how-to-use-import-diagnostics-tool-solidworks
- https://help.solidworks.com/2025/English/swconntutorial/t_reparinggaps_impexptut.htm (resumo)
- https://help.solidworks.com/2026/English/SolidWorks/Sldworks/HIDD_EXPORT_OPTIONS_STEP.htm · https://help.solidworks.com/2025/English/SolidWorks/sldworks/r_publish_step242.htm (resumos)
- http://blogs.solidworks.com/products/solidworks/automate-file-exports-solidworks-task-scheduler
- https://help.solidworks.com/2024/English/SolidWorks/Sldworks/HIDD_TASK_FILE_EXPORT.htm (resumo)
- https://help.solidworks.com/2016/English/EnterprisePDM/Admin/IDD_CONVOPSTEPDLG.htm · https://help.solidworks.com/2026/english/EnterprisePDM/admin/c_IGES_3D_curves.htm (resumos)

**Inventor / Fusion / Onshape**
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-16C49D61-6796-4169-9A6B-64F588360813.htm (opções STEP/IGES)
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-7FACB50F-ED00-419D-8C23-F98283CCD8F8.htm (AnyCAD)
- https://help.autodesk.com/cloudhelp/2014/ENU/Inventor/files/GUID-1B0D704D-44B8-4ACD-B854-00E2B08A22B9.htm (Repair, resumo)
- https://forums.autodesk.com/t5/inventor-forum/update-step-files-from-original/td-p/7024470
- https://www.cadforum.cz/cz/podpora-step-ap242-a-stpz-v-inventoru-2016-tip10494 (resumo)
- https://www.autodesk.com/products/fusion-360/blog/data-exchange-in-fusion-360-part-1/ · https://forums.autodesk.com/t5/fusion-support-forum/import-jt-file/m-p/8544798 (resumos)
- https://cad.onshape.com/help/Content/File/supported_file_formats.htm

**Bibliotecas comerciais e abertas**
- https://docs.techsoft3d.com/communicator/latest/overview/supported-formats.html · https://docs.techsoft3d.com/exchange/latest/start/supported-formats.html (resumos)
- https://cadexchanger.com/ · https://docs.cadexchanger.com/sdk/sdk_key_features_benefits.html (resumo)
- https://www.spatial.com/products/3dinterop (resumo) · https://www.datakit.com/en/product.php · https://datakit.com/en/crosscad_ware.php (resumos)
- https://en.wikipedia.org/wiki/LibreDWG · https://www.opendesign.com/pricing.md (resumos)
- https://3mf.io/spec/ · https://3mf.io/wp-content/uploads/sites/106/2025/02/3MF_Beam_Lattice_Extension_v1.2.0.pdf
- https://docs.rs/crate/dxf/latest/source · https://lib.rs/crates/dxf
- https://docs.ifcopenshell.org/ifcopenshell/geometry_iterator.html
- https://pdfa.org/resource/3d-formats/ · https://www.3dmag.com/3d-wikipedia/prc-file-format-explained/ (resumos)
- https://en.wikipedia.org/wiki/JT_(visualization_format) · https://support.sw.siemens.com/en-US/okba/PL8658242/What-are-the-differences-in-Parasolid-file-formats/index.html · https://pointwise.com/doc/user-manual/file/import/database/parasolid-xt.html (resumos)
- https://support.ptc.com/help/creo/creo_pma/r12/usascii/data_exchange/interface/Exporting_Part_or_Assembly_to_STEP.html (resumo)
- https://blogs.sw.siemens.com/solidedge/?p=7554 (Synchronous Technology, resumo)

**Código e documentos do Fenkra lidos**
`crates/fenkra-kernel/cpp/kernel.cpp:988-1011, 147`; `hybrid.inc:43, 160-164, 304-335, 370-401`; `kernel/src/lib.rs:693-700`, `ffi.rs:195-197`, `build.rs`; `fenkra-solidos/src/lib.rs:4079-4094, 5144-5161`, `mesh_io.rs`; `fenkra-core/src/doc.rs:1856, 2673-2712`; `fenkra-app/src/app.rs:880-892, 2510-2545`, `toolbar.rs:479`, `dwg_ui.rs:242`; `fenkra-desenho/src/export.rs:51, 265-283`; `fenkra-project/src/lib.rs:1-260`; `fenkra-montagem/src/lib.rs:536`; `docs/contextos/plm-vpm.md`, `docs/arquitetura/adr/0009, 0011`, `docs/produto/visao.md`, `docs/arquitetura/reuso-opensource.md`, `docs/requisitos/nucleo.md`, `docs/desenvolvimento.md`, `docs/contextos/smart-fastener.md`.

---

## Nota de entrega

- **Resultado:** relatório completo conforme a estrutura pedida (resumo e lacunas, fichas, requisitos, backlog, fontes).
- **Evidências:** leituras de código com `arquivo:linha` e as fontes acima. Itens não confirmados estão marcados [C] ou [V parcial].
- **Arquivos alterados:** nenhum. O plano de modo-planejamento não foi gravado, porque você pediu para não criar arquivos e a ferramenta de saída do plano não estava disponível.
- **Verificações executadas:** nenhuma de compilação ou teste; foi pesquisa e leitura apenas.
- **Falhas e limites:**
  - As páginas help.solidworks.com não renderizaram no WebFetch, então os detalhes do SolidWorks vêm de resumos de busca e de blogs de revendas.
  - Algumas fontes deram 403/404 (Javelin PDM, Solid Solutions, Fusion blog).
  - Não confirmei: o gatilho exato da tarefa Convert do PDM; o tratamento de revisão do original no Vault; as diferenças PRC × U3D; a licença do crate `dxf`; o preço de HOOPS, CAD Exchanger, Spatial e Datakit; o comportamento do OCCT com caminhos com acento no Windows.
- **Riscos:** o OCCT 8.0.1 é recente, então os nomes de classe e parâmetros da camada DE devem ser conferidos nos cabeçalhos instalados antes de implementar. Definir a licença do Fenkra quanto a plugins comerciais antes do CNV-18.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas são: (1) sem importação de STEP/IGES na interface, com `import_step` órfão e sem reparo, unidades nem relatório; (2) exportação mínima, sem XDE (cores, nomes, PMI, montagem) e com bug de extensão; (3) sem módulo conversor, opções por formato, relatório nem lote; (4) sem vínculo da origem e dos derivados com o save management/PDM (hash, `derivado_de`, revisão); (5) sem reconhecimento de features nem diagnóstico de corpos importados.
