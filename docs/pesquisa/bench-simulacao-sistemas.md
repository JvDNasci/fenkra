# Benchmarking: simulação FEM/CFD e sistemas (chicote, compósitos, plástico)

**Escopo.** Itens avaliados: B159, B160, B161, B132 (simulação), B138 (chicote), B137 (compósitos), B148 e B129 (plástico e moldes). Nenhum arquivo foi criado ou alterado.

**Legenda de confiança.**
- **[V]** significa que li a informação em página ou resultado de busca desta pesquisa.
- **[C]** significa conhecimento prévio ou inferência que ainda precisa de verificação.
- Preços de revenda mudam por região e data. Trate todos como ordem de grandeza.

**Páginas que não consegui ler.** Isso reduz a confiança em alguns pontos:
- O wiki.freecad.org bloqueou a leitura direta (proteção Anubis).
- O site da Autodesk devolveu 403.
- As páginas de ajuda do SOLIDWORKS vieram vazias.
- Os dois PDFs da IBM sobre CATIA não abriram, porque não há leitor de PDF instalado. A descrição do CATIA Harness e do CPD vem de cursos de revendas e ajuda indireta.

---

## 0. Resumo executivo

1. **FEM é o item de maior retorno.** Todo CAD concorrente tem análise estática embutida: Fusion, Onshape, SOLIDWORKS, Inventor, FreeCAD. É o recurso que mais separa "CAD de verdade" de "modelador" para um engenheiro de projeto.
2. **A arquitetura recomendada** é copiar a estrutura do FreeCAD FEM (contêiner na árvore, objetos de material, restrição, carga, malha, solver e resultado). A parte inteligente vem da filosofia do Fenkra:
   - condições de contorno presas a **referências estáveis (B15/ADR 0010)**;
   - resultados marcados como **desatualizados** quando o modelo muda;
   - **diagnóstico** em vez de falha silenciosa;
   - contatos detectados sozinhos.
3. **Licenças viabilizam o caminho.**
   - Netgen é LGPL-2.1 e pode ser embutido [V].
   - CalculiX (GPL-2) e OpenFOAM (GPL-3) ficam só como **processo externo** [V].
   - Gmsh é GPL-2+ e também fica só como processo [V].
   - O formato `.inp` (entrada) e o `.frd` (saída) são formatos de dados, então ler e escrever é seguro. O risco jurídico está em **distribuir o `ccx.exe` junto do instalador** (ver seção 2).
4. **Divida o B159/B132.** Os dois somam 900 k e misturam contêiner, malha, execução e visualização. Proponho:
   - B159 como contêiner, materiais, restrições e cargas;
   - B132 como motor (malha, `.inp`, execução, leitura do `.frd`);
   - resultados, modal/térmica e validação como itens novos.
5. **CFD (B161) é bem mais arriscado que FEM.**
   - O maior obstáculo é instalar o OpenFOAM no Windows. O próprio CfdOF lista BlueCFD, MinGW (só a versão 2212 funciona) e Docker/Podman como opções, com várias ressalvas [V].
   - Recomendo P2, depois do FEM MVP.
6. **Chicote (B138) e compósitos (B137) são nichos de alto valor.** São mercados pequenos, mas nos quais o Fenkra pode ser competitivo, porque as ferramentas líderes custam caro e ficam presas a ecossistemas (CATIA, NX, Creo). Ambos sofrem sem dados de entrada boas:
   - chicote precisa da lista de fios (CSV/KBL);
   - compósito precisa de uma superfície de molde (OML) robusta.
7. **Plástico e moldes são o melhor "custo-benefício" depois do FEM.**
   - Boss, snap-fit, grelha e lip são geometria paramétrica pura.
   - A análise de saída e de undercut é reutilizável no B164 (balanço em impressão 3D) e no B129.
   - O diferencial é a **regra plástica** como objeto do documento, como no Fusion [V].

---

## 1. Panorama transversal de licenças

| Biblioteca / programa | Licença | Uso no Fenkra (LGPL-2.1-or-later) |
|---|---|---|
| **Netgen** (malha tetraédrica) | LGPL-2.1 [V] (página do repositório) | **Pode embutir** (ligação direta). Roda no Windows [V]. |
| **Gmsh** | GPL-2+, com exceção para combinar com TetGen, Netgen, METIS, OCCT, ParaView [V] | Só **processo externo**. Modificações não herdam a exceção [V]. |
| **CalculiX (`ccx`)** | GPL-2 (Gentoo) / GPL-2.0+ (MSYS2) / pacote mistura BSD-3, GPL-2 e domínio público (openSUSE) [V]. Ver o `LICENSE` oficial. | **Processo externo.** Binário para Windows em MSYS2, espelho GE e dhondt.de [V]. |
| **Elmer** | GPL [V, diretório antigo]; GPL-2+ [C] | Só processo externo, para multifísica (eletromagnético, acoplado). |
| **Code_Aster** | GPL (v3 segundo as fontes) [V] | Processo externo. Sem instalador Windows relatado [V]. |
| **Salome (plataforma)** | LGPL [C, não confirmado] | Só estudar. Curva de aprendizado alta [V]. |
| **OpenFOAM** | GPL-3.0-or-later; a marca OpenFOAM® é da OpenCFD [V] | **Processo externo.** Duas linhas: Foundation (v9–13) e OpenCFD (v2006–2512) [V]. |
| **cfMesh** | GPLv3 [V] | Só externo, e é um complemento à parte. |
| **CfdOF** (workbench FreeCAD) | LGPL-3.0 [V] | Estudar. O LGPL-3.0 é compatível com LGPL-2.1+ só se o projeto for "or later" (conferir a cláusula exata). |
| **SU2** | LGPL (provável) [V, fonte fraca]; LGPL-2.1 [C] | CFD compressível/aero. Possível alternativa embutível, a verificar. |
| **MFEM** | BSD (conforme `modulos-novos.md`) | Possível solver embutido, a longo prazo. |
| **deal.II** | LGPL-2.1+ [C] | Idem, mas pesado. |
| **TetGen** | Gratuito só para pesquisa e uso não comercial [V]. Licença comercial sob consulta. | **Evitar.** |
| **fTetWild** | Fonte divergente: README remete ao arquivo de licença; um commit antigo mostra BSD-3 [V]. MPL-2.0 [C]. | Verificar antes de usar. |
| **WireViz / Filare** | GPL-3.0 [V] | Só inspiração (YAML → SVG + BOM). |
| **KBL (prostep ivip)** | Esquema XSD publicado em ecad-wiki.prostep.org [V]; a licença do esquema não foi verificada | Gerar e ler XML pelo Fenkra. |
| **VEC (VDA 4968)** | Padrão prostep ivip [V] | Nenhuma biblioteca aberta encontrada [V]. Destino futuro. |
| **faer** (álgebra esparsa em Rust) | Licença própria não confirmada [V]. KLU é LGPL-2.1+ [V]. | Conferir `LICENSE` antes de depender. |
| **OpenCascade (OCCT)** | LGPL-2.1 com exceção [C] | Já é a base do Fenkra. |

**Regra prática.**
- **Ligar** só LGPL, MIT, BSD e Apache.
- **Chamar como processo** GPL, com caminho configurável.
- **Evitar** AGPL e licenças comerciais.

**Distribuição do `ccx.exe`.** Embalar um programa GPL separado no instalador costuma ser "agregação mera", desde que o Fenkra o chame como programa separado e o instalador traga o texto da licença e a oferta do código-fonte [C]. Como cautela:
- na 1ª versão, o Fenkra detecta o `ccx` configurado (PATH ou caminho nas opções) e oferece um **assistente de download** do site oficial;
- empacotar junto fica como decisão do mantenedor, com revisão jurídica.

---

## 2. B159 — Bancada Simulação FEM (contêiner de análise)

### 2.1 Quem oferece

| Produto | Módulo / comando | Licenciamento |
|---|---|---|
| **FreeCAD FEM** | Workbench FEM. Solvers CalculiX (padrão), Elmer, Mystran e Z88 (desde 0.20). Malha por Gmsh e Netgen [V]. | Aberto e gratuito. |
| **SOLIDWORKS Simulation** | Standard, Professional e Premium [V]. Premium acrescenta não linear e dinâmica [V]. | Pago por camada. Único preço público achado: US$ 2.146 por 3 meses (Premium, revenda) [V]. |
| **Fusion Simulation** | Estático linear **grátis** com a assinatura base. A extensão (≈ US$ 1.465/ano em uma página Autodesk, £ 1.428 na página britânica) acrescenta não linear, flambagem, térmico, tensão térmica, modal, evento, resfriamento de eletrônicos e moldagem por injeção [V]. | Assinatura. Pagamento por uso em *tokens*: estudos básicos 3, avançados 6 [V]. |
| **Inventor Nastran** | In-CAD. Só dentro da *Product Design & Manufacturing Collection* [V]. Coleção a ≈ US$ 3.375/ano segundo agregador [V, fonte fraca]. | O Inventor Professional traz só estático e modal [V]. |
| **Onshape Simulation** | Estático linear e modal, na nuvem, assíncrono. Incluído no plano Professional [V]. | Preços de terceiros conflitam (≈ US$ 1.500 Standard e US$ 2.500 Professional por usuário/ano) [V, fonte fraca]. |
| **Ansys Discovery** | Física em tempo real na GPU. A partir da 2025R1 virou Pro, Premium e Enterprise [V]. Alta fidelidade pede licença Mechanical/CFD Pro [V]. | Sem preço público. |
| **SimScale** | FEM e CFD na nuvem. Plano **Community grátis** (≈ 3.000 horas-núcleo, 10 simulações) [V]. | Mechanical, Professional e Enterprise sob cotação [V]. |
| **Salome-Meca / Code_Aster** | Plataforma com geometria, malha, AsterStudy e ParaVis [V]. | Aberto. Curva de aprendizado alta e instalação difícil no Windows [V]. |

### 2.2 Como funciona no melhor de cada um

**SOLIDWORKS Simulation (referência de CAD embutido) [V, parte via revisões].**
- O estudo vive dentro do modelo. Reexecutar e remalhar é simples.
- Malha e contatos entre componentes são automáticos na maioria dos casos. Isso é o que os iniciantes mais elogiam.

**Fusion (referência de fluxo moderno).**
- Contatos automáticos dentro de uma tolerância de detecção, por padrão **colados** (*bonded*). Um relatório de exemplo mostra 0,10 mm. O tipo pode ser trocado em "Manage Contacts".
- Elementos parabólicos disponíveis. Refinamento adaptativo perto das fronteiras do fator de segurança.
- **Alvos de fator de segurança** (superior e inferior) são definidos antes de resolver. O resultado cai em "muito forte", "atende" ou "não suficientemente forte".
- O *Safety Factor* é o resultado padrão. Ele pode passar enquanto outro critério falha.
- Resolução na nuvem com notificação ao terminar.

**Onshape.**
- As **juntas da montagem** guiam a configuração (fixa, deslizante, revoluta).
- Sem malha exposta e sem "defeaturing".
- O modelo deve estar totalmente restrito, ou a simulação falha. Há técnica separada para corpos livres.

**FreeCAD FEM (referência de arquitetura aberta).** O 1.1 trouxe [V]:
- contato duro (*hard*) e colado (*tied*) no CalculiX;
- contato térmico e radiação de cavidade;
- amplitudes no tempo para a maioria das condições;
- membrana, treliça, seções de viga caixa e elíptica;
- Netgen com malha de 2ª ordem, refinamento local e malha conforme;
- filtros de pós-processamento *Calculator* e *Glyph*, e o widget *Data and Extractions*;
- `Fem.frdToVTK`.

O 1.1 não resolveu tudo: o pós-processamento refatorado do CalculiX "não está totalmente desenvolvido" e faltam análises de vários passos [V].

**Objetos do contêiner (modelo FreeCAD) [C, wiki bloqueado].** Análise > material, restrições (fixa, deslocamento, rolamento, simetria), cargas (força, pressão, gravidade, torque, térmica), malha, solver, resultado.

### 2.3 O que os usuários elogiam e reclamam

| Fonte | Elogios | Reclamações |
|---|---|---|
| SOLIDWORKS | Malha e contato automáticos. Fácil mudar restrições e reexecutar [V]. | Pouco controle da malha. Mais difícil conferir convergência que em outros FE. Pesado em recursos [V]. |
| SimScale (G2 ≈ 4,6/5, ≈ 280 avaliações) | Suporte rápido, acesso pela nuvem, preço [V]. | Malha lenta, solução lenta, falta de edição de geometria, interface "desajeitada" [V]. |
| FreeCAD FEM | Aberto, vários solvers. | Falhas do Gmsh com VTK 9, malha "bogus" que o CalculiX recusa, objetos de malha sobrando após cancelar/erro, reparo de malha frágil [V, rastreador de bugs]. |
| Code_Aster / Salome | Poder e custo zero. | Curva de aprendizado, instalação [V]. |

**O que diferencia o líder:**
- **Automatismo** (contatos e malha) sem esconder o que foi feito.
- **Vínculo permanente com o modelo:** o estudo vive no mesmo arquivo e se atualiza com a edição.
- **Resultado interpretável:** o fator de segurança como resposta padrão.

### 2.4 Bibliotecas e padrões
- **Malha:** Netgen embutido (LGPL-2.1). Alternativa: Gmsh como processo, para ter um segundo mesher.
- **Solver:** CalculiX como processo (ver B132).
- **Formatos:** `.inp` (Abaqus-like) para escrever. `.frd` e `.dat` para ler. Escrever também VTK/VTU para abrir no ParaView.
- **Leitura de resultados:** parser próprio em Rust (o `.frd` é texto de largura fixa). Para VTK, procurar crate MIT ou Apache (`vtkio`) [C].

### 2.5 Recomendação para o Fenkra

**Fluxo ideal (estilo CATIA V5: árvore e publicações).**

1. Criar **Análise** na árvore. Ela referencia um corpo ou montagem, não uma cópia da geometria.
2. Objetos filhos:
   - Material;
   - Restrições;
   - Cargas;
   - Malha;
   - Solver;
   - Resultados.
3. Cada **restrição ou carga** abre o painel **suporte → modo → parâmetros → avançado**:
   - *Suporte:* faces ou arestas por referência estável (B15).
   - *Modo:* fixa, deslizante, cilíndrica, simetria, deslocamento prescrito; ou força, pressão, torque, gravidade, centrífuga, rolamento, térmica.
   - *Parâmetros:* valor, direção, sistema local, unidade.
   - *Avançado:* amplitude no tempo e referência local.
4. **Publicações.** Uma peça publica "faces de apoio" e "faces de carga". A análise na montagem referencia a publicação, de modo que trocar a peça mantém as condições. Isso espelha a ideia de publicação do CATIA.
5. **Material.** Vem da tabela de materiais do B92 (E, ν, ρ, escoamento). Falta propriedade ⇒ erro explícito, nunca um valor padrão.
6. **Verificador de modelo antes de rodar**, em lista clicável:
   - peça sem restrição (corpo livre);
   - carga em face que sumiu;
   - material sem módulo de Young;
   - restrição pontual com singularidade provável;
   - unidade estranha.
7. Resultados com **estado de vínculo**:
   - *válido* (modelo, malha e configuração iguais ao hash gravado);
   - *desatualizado* (cinza, nunca exibido como atual);
   - *órfão* (referência perdida).

**MVP do B159 (≈ 250 k):**
- contêiner e painéis;
- material;
- restrições: fixa, deslocamento, simetria;
- cargas: força, pressão, gravidade, torque;
- verificador de modelo;
- persistência no documento (formato versionado, B64);
- estado de atualização.

**Fica para depois:**
- Elmer e eletromagnético;
- montagem multicorpo com contatos (B160);
- casca e viga (usar B114 para vigas);
- análise térmica (novo item);
- otimização topológica (BN17).

### 2.6 Integrações
- **B15/ADR 0010** (referências), **B92** (materiais, massa), **B87** (STEP/XCAF), **B64** (formato), **B101** (parâmetros), **B102** (configurações).
- **B114** (pórtico): ponto de validação cruzada de viga contra FEM 3D.
- **B132** (motor), **B160** (contato etc.), **B137** (seção composta), **B164** (impressão 3D, deformação).

### 2.7 Riscos técnicos
1. **Condições de contorno em faces que mudam.** É o ponto onde o ADR 0010 vira vantagem, mas também o maior risco. Cada condição precisa de teste de sobrevivência a edição.
2. **Resultados enganosos.** FEM dá números bonitos mesmo errados. Aviso de uso, verificador de singularidade e suíte de validação são obrigatórios.
3. **Unidades.** O CalculiX não tem unidades. Use o sistema mm, t, s, N, MPa: densidade em t/mm³ (aço ≈ 7,85e-9). A tabela do B92 está em g/cm³ e exige conversão testada [C].
4. **Suporte.** Qualidade de malha e contato geram muito atendimento [conforme `modulos-novos.md`].

### 2.8 Testes de aceite
- Viga em balanço: flecha = PL³/3EI com erro ≤ 2% com tet10.
- Placa com furo central: Kt ≈ 3 [C].
- Edição que renomeia ou divide a face de apoio: a condição se reanexa, ou o erro nomeia a condição e mostra candidatos no 3D.
- Mudança de geometria: o resultado vira "desatualizado".
- Modelo sem restrição: o verificador bloqueia a execução com mensagem.

### 2.9 Tamanho
B159 reduzido a ~250 k (contêiner, condições, verificador). Somados B132 (450 k), resultados (300 k), modal/térmica (200 k) e validação (150 k), o FEM completo fica em ≈ 1.350 k.

---

## 3. B132 — Ponte para solver externo (CalculiX)

### 3.1 Concorrentes
Os mesmos da seção 2. O que importa aqui é **como cada um fala com o solver**:
- **FreeCAD** escreve o `.inp` por objeto e chama `ccx`. Depois lê o `.frd`. Há também conversão para VTK (`frdToVTK`) [V].
- **Fusion, Onshape e SimScale** resolvem na nuvem. Resolvem sem o usuário ver o arquivo do solver [V].
- **SOLIDWORKS** embute o solver (FFEPlus e Direct Sparse) [C].

### 3.2 Detalhe do fluxo

1. **Instantâneo da geometria.** O Fenkra congela o B-rep, com **IDs de face** estáveis para o instante da malha.
2. **Malha (Netgen):**
   - entrada via BREP/STEP;
   - tetraedros de 2ª ordem (C3D10);
   - tamanho global e local (por face, aresta ou região);
   - o resultado mantém o **mapa face CAD → faces da malha**.
3. **Escrita do `.inp`:**
   - `*NODE`, `*ELEMENT, TYPE=C3D10`;
   - `*NSET`/`*ELSET`/`*SURFACE` por condição de contorno;
   - `*MATERIAL`, `*SOLID SECTION`;
   - `*BOUNDARY`, `*CLOAD`, `*DLOAD`;
   - `*STEP` com `*STATIC` (ou `*FREQUENCY`);
   - `*NODE FILE`/`*EL FILE` para saída.
4. **Execução:** processo filho `ccx`, assíncrono, com cancelamento. Acompanha `.sta` e `.cvg` para mostrar progresso.
5. **Leitura:** parser de `.frd` (deslocamentos, tensões, von Mises). Carrega num objeto de resultado.
6. **Coloração** no viewport, feita no item de resultados.

**Armadilhas conhecidas [C]:**
- Ordem dos nós do tetraedro de 2ª ordem difere entre Gmsh/Netgen e CalculiX (nós 9 e 10 trocados em alguns casos). Um teste de *patch test* com C3D10 pega isso.
- Jacobiano negativo: o FreeCAD recomenda o framework "CalculiX Standard", que mostra os elementos com Jacobiano não positivo [V].

### 3.3 Elogios e reclamações
O FreeCAD mostra o que acontece quando o encaixe é frágil [V]: malha falsamente bem-sucedida, objetos órfãos após cancelar, dependência de versão de VTK. Em SimScale reclamam de malha lenta [V].

### 3.4 Bibliotecas
- **Netgen** (LGPL-2.1, Windows [V]): embutir. Aceita STEP/IGES por um kernel de geometria e STL/CSG, com API Python [V]. A página não confirma que o kernel é o OCCT [V].
- Isolar o Netgen num **processo filho** (`fenkra-mesh`), no mesmo desenho do `fenkra-conv` do B87. Assim uma falha do mesher não derruba o Fenkra e se evita carregar duas cópias do OCCT.
- **CalculiX:** GPL, processo. Binários Windows disponíveis [V].
- **Gmsh:** só se um segundo mesher fizer falta (GPL, processo).

### 3.5 Recomendação
- **MVP:**
  - malha Netgen tet10 com tamanho global e local;
  - escrita do `.inp` para estático linear;
  - execução assíncrona com cancelamento;
  - leitura do `.frd`;
  - relatório de malha (nós, elementos, qualidade mínima);
  - mensagens de erro do `ccx` traduzidas e ligadas ao objeto culpado.
- **Descoberta do solver.** Configuração do caminho, teste de versão na primeira execução e mensagem clara se faltar.
- **Depois:** modal, térmico, contato (B160), segundo mesher, fila de tarefas.
- **Arquivos de trabalho** numa pasta do documento (`.inp`, `.frd`, log), para depuração e reprodução fora do Fenkra.

### 3.6 Integrações
B159 (contêiner), B92 (massa e densidade), B87 (processo isolado), B64 (formato), B39 (CLI headless para rodar análises em lote), B160.

### 3.7 Riscos
1. **Mapa de faces:** associar faces CAD às faces da malha. O Netgen guarda o índice da face do OCC; o mapeamento tem de ser testado contra reordenação.
2. **Malha ruim em geometria difícil** (superfícies finas, fendas). Fornecer defeaturing guiado (remover filetes pequenos), mas só como opção.
3. **Distribuição do `ccx`** (seção 1).
4. **Windows:** caminhos com espaços, antivírus, permissões. A documentação do CfdOF recomenda instalar fora de `C:\Program Files` por causa do UAC [V]; vale a mesma cautela.

### 3.8 Testes de aceite
- Cubo sob tração: erro de deslocamento ≤ 0,1% contra a solução analítica.
- Patch test com tet10.
- Cancelar a execução: nenhum processo órfão, nenhum arquivo parcial tratado como resultado.
- `ccx` ausente: mensagem com o caminho esperado, sem travar.
- Mesma entrada duas vezes: `.inp` idêntico byte a byte (determinismo).

### 3.9 Tamanho
450 k, como está. Uma divisão natural, se necessário: malha (200 k) e escrita/execução/leitura (250 k).

---

## 4. B160 — Contato, não linear, fadiga e flambagem

### 4.1 Quem oferece

| Capacidade | Quem |
|---|---|
| Contato em montagem | SOLIDWORKS (mais forte no Premium), Fusion (auto, colado por padrão), Inventor Nastran, FreeCAD 1.1 (*hard* e *tied*) [V] |
| Não linear | SW Premium, Fusion (extensão), Nastran [V] |
| Flambagem | Fusion (extensão), Nastran [V], CalculiX `*BUCKLE` [C] |
| Fadiga | SW Simulation [V; a camada que a inclui é incerta nas fontes]. |
| Casos de carga e estudos paramétricos | SW Simulation, Ansys, Fusion [C] |

### 4.2 Como funciona (melhor prática)
- **Contato automático.** Detecta pares de faces próximas dentro de uma tolerância e cria união colada por padrão. O usuário só muda onde importa (Fusion) [V].
- **Tipos:** colado, sem penetração (*hard*), com atrito, e contato térmico (FreeCAD 1.1) [V].
- **Não linear:** plasticidade (curva tensão-deformação), grandes deslocamentos (`*NLGEOM`) e passos incrementais [C].
- **Flambagem linear:** autovalor do fator de carga crítico. Compare com Euler Pcr = π²EI/(KL)².
- **Fadiga simples (S–N):**
  - entrada: histórico de tensão de um caso de carga;
  - curva de Basquin, correção de tensão média (Goodman/Gerber), regra de Miner;
  - saída: mapa de vida ou dano.
  - É só pós-processamento de tensões. Dá para implementar em Rust, sem solver.
- **Casos de carga e combinações:** em estática linear, a combinação é superposição de campos de resultado (α·σ₁ + β·σ₂), sem nova solução.
- **Estudo paramétrico:** variar parâmetros do modelo (B101/B102) e tabular os resultados.

### 4.3 Elogios e reclamações
- Automatismo de contato é elogiado (Fusion, SW). Em contrapartida, "contato e convergência" são fonte comum de falha. A SOLIDWORKS vendeu o contato do 2021 como mais estável e rápido, com estabilização de início [V, texto de fornecedor].
- Faltam tipos de contato avançados, segundo revisões [V].

### 4.4 Bibliotecas
Tudo via CalculiX (processo): `*CONTACT PAIR`, `*SURFACE INTERACTION`, `*SURFACE BEHAVIOR`, `*TIE`, `*PLASTIC`, `*BUCKLE` [C]. Fadiga é código Rust próprio, com tabelas de materiais S–N da biblioteca.

### 4.5 Recomendação
- **Quebrar na execução** (≈ 400 k):
  1. Contato automático com tolerância e tipo por par, mais não linear de material e geometria (≈ 200 k).
  2. Flambagem, fadiga S–N e combinações de casos (≈ 150 k).
  3. Estudo paramétrico ligado à tabela de projeto (≈ 100 k).
- **Contato automático** a partir das relações da montagem, listado na árvore como objetos editáveis.
- **Contatos "órfãos":** o par muda se as peças se afastam; o objeto avisa e permite reparar.
- **Fadiga** só como indicador, com aviso, não como certificação.

### 4.6 Integrações
B159, B132, B99 (interfaces de encaixe da montagem), B101/B102 (parâmetros e tabela de projeto), B92.

### 4.7 Riscos
1. **Convergência:** o CalculiX falha silenciosamente em alguns casos. Interpretar o `.cvg`/`.sta` e traduzir.
2. **Curvas de material não lineares** exigem dados que o usuário geralmente não tem. Oferecer valores típicos *rotulados como aproximados* [C].
3. **Fadiga:** os dados S–N dependem de acabamento, entalhe e confiabilidade. Isso não deve virar "certificação".

### 4.8 Testes de aceite
- Dois blocos em contato colado ⇒ resultado igual ao de um bloco único (erro < 1%).
- Flambagem de coluna: fator crítico vs. Euler (erro ≤ 5%).
- Plasticidade: barra com plasticidade perfeita; reação no patamar = escoamento × área.
- Combinação de casos: α·caso1 + β·caso2 = resolver o caso combinado (erro numérico pequeno).

### 4.9 Tamanho
400 k (como está), em 3 execuções de 200/150/100.

---

## 5. B161 — Bancada CFD via OpenFOAM externo

### 5.1 Quem oferece

| Produto | Módulo | Licenciamento |
|---|---|---|
| **FreeCAD CfdOF** | Workbench. Escoamento incompressível laminar/turbulento (RANS, LES, DES; simpleFoam e pimpleFoam), superfície livre (interFoam), compressível, meios porosos, AMI [V]. | LGPL-3.0, gratuito [V]. |
| **SOLIDWORKS Flow Simulation** | Assistente (*wizard*) com análise interna ou externa, domínio computacional, tampas (*lids*), metas (*goals*) [V]. | Módulo pago à parte [C]. |
| **Fusion** | Resfriamento de eletrônicos na extensão; CFD geral não confirmado nesta pesquisa. | — |
| **SimScale** | CFD e FEM na nuvem [V]. | Community grátis, Professional sob cotação [V]. |
| **Ansys Discovery** | Fluidos em tempo real na GPU [V]. | Sem preço público [V]. |

### 5.2 Como funciona no detalhe

**SOLIDWORKS Flow Simulation (referência de fluxo de CAD) [V, via revendas].**
1. **Assistente:** nome, configuração, unidades, **interno ou externo**, fluido.
2. **Domínio computacional:** caixa em torno do modelo, redimensionável. Existe "RESET" quando a geometria muda.
3. **Tampas (*lids*):** uma análise interna exige volume fechado. As aberturas são cobertas por corpos "tampa". O comando cria tampas sozinho, mas **só em faces planas** [V].
4. **Condições:** entrada (vazão ou velocidade), saída (pressão), paredes.
5. **Metas (*goals*):** grandezas monitoradas (perda de carga, vazão, temperatura média, força numa face). O solver para quando as metas ficam constantes.
6. **Malha automática** com refinamento adaptativo à solução:
   - o controle "Result resolution" nível ≥ 6 liga o refinamento adaptativo sozinho;
   - os gatilhos de refino são Periódico, Tabular ou Manual;
   - a limitação conhecida é que meta convergida não prova independência de malha [V].

**CfdOF (fluxo aberto) [V].**
- Pré-processamento, malha, solução e pós-processamento.
- Condições atribuídas a faces: entrada com velocidade uniforme, saída com pressão estática, parede, *slip*. Faces sem condição ficam em *slip* (tutorial) [V].
- Malha por **cfMesh** (cortes cartesianos com camadas-limite), **snappyHexMesh** (com *baffles*) ou **Gmsh** (tetraedros) [V].
- Pós-processamento no ParaView, com funções de força e sondas [V].

### 5.3 Elogios e reclamações
- **SW Flow:** muito citado pela facilidade do assistente. As reclamações vêm da dificuldade das tampas fora de superfícies planas e de metas que continuam mudando a cada refino [V].
- **CfdOF:** a instalação é o problema. Exige OpenFOAM em versão específica, MPI, e no Windows há ressalvas [V]. A comunidade usa cfMesh para camadas-limite; o snappyHexMesh é visto como a escolha padrão [V, anedótico].
- **SimScale:** malha lenta e interface "desajeitada" [V].

### 5.4 Bibliotecas
- **OpenFOAM** (GPL-3): só processo. Duas linhas incompatíveis entre si: **Foundation** (v9–13) e **OpenCFD** (v2006–2512). A marca é da OpenCFD [V].
- **snappyHexMesh** já vem com o OpenFOAM (nada extra para instalar). O cfMesh é GPLv3 e à parte [V].
- **Alternativa embutível:** SU2 (LGPL, a verificar), mas é voltado a aeronáutica compressível.
- **Pós-processamento próprio:** ler VTK de `foamToVTK` e calcular linhas de corrente em Rust (integração RK4 no campo interpolado). Evita depender do ParaView.

### 5.5 Recomendação

**Fluxo ideal:**
1. **Bancada CFD** > Novo estudo > escolher **interno ou externo**.
2. **Domínio de fluido.**
   - *Interno:* a cavidade a partir do volume interno, com tampas nas aberturas.
   - *Externo:* caixa com folgas proporcionais ao comprimento característico, editável.
   - O domínio é uma **feature da árvore** (corpo derivado, B-rep) e atualiza com o modelo.
3. **Condições** com o painel suporte → modo → parâmetros → avançado:
   - entrada (velocidade, vazão, pressão total);
   - saída (pressão estática);
   - parede (sem escorregamento, rugosidade, deslizante);
   - simetria.
4. **Metas** (estilo SW): perda de carga, vazão, força, temperatura média.
5. **Malha snappyHexMesh** com refinamentos por face e por região, e camadas-limite.
6. **Execução** assíncrona do OpenFOAM, com resíduos e metas em gráfico ao vivo.
7. **Pós:** planos de corte, linhas de corrente, vetores, isosuperfícies. Pressão e velocidade no viewport.

**MVP (≈ 450 k, que recomendo quebrar):**
- escoamento incompressível estacionário (simpleFoam, k–ω SST);
- interno e externo;
- domínio automático e tampas planas;
- snappyHexMesh;
- metas;
- campos escalares e linhas de corrente.

**Depois:**
- transiente;
- térmico com condução no sólido (chtMultiRegionFoam);
- rotação;
- superfície livre;
- cfMesh.

### 5.6 Integrações
B159 (infraestrutura de execução, resultados, relatórios), B87 (exportar malha de superfície para STL por face), B09 (Fill/Join para fechar aberturas), B92, B146 (visualizador glTF para compartilhar resultados).

### 5.7 Riscos técnicos
1. **Instalação do OpenFOAM no Windows** é o maior risco. O CfdOF lista duas rotas nativas: BlueCFD Core (2024-1) e o pacote MinGW da OpenCFD, onde "só a versão 2212 funciona". Docker/Podman é a alternativa [V]. Decidir uma rota suportada por plataforma.
2. **Fechar o volume:** tampas em superfícies curvas e geometria suja. Entregar um relatório de estanqueidade com as arestas livres.
3. **Qualidade de malha e camadas-limite** do snappyHexMesh. Um relato de 2014 chama o algoritmo de camadas de "fraco" para casos viscosos [V, antigo].
4. **Expectativa do usuário:** CFD dá resultado bonito mesmo errado. Exigir verificação de convergência e independência de malha.

### 5.8 Testes de aceite
- Tubo reto de seção circular, escoamento laminar: perfil de Poiseuille e perda de carga contra a solução analítica (erro ≤ 5%).
- Tubo turbulento: fator de atrito contra Moody/Blasius (erro ≤ 10%) [C].
- Cilindro em escoamento externo: Cd na faixa de referência [C].
- Modelo com abertura não fechada: o estudo falha com a lista das arestas livres.
- OpenFOAM ausente ou versão não suportada: mensagem clara.

### 5.9 Tamanho
450 k, mas recomendo dividir em **domínio, tampas, condições e metas (≈ 300 k)** (item novo) e **malha, execução e pós (≈ 350–450 k)** (o próprio B161). Prioridade P2, depois do FEM.

---

## 6. B138 — Chicote: rota, lista e achatamento

### 6.1 Quem oferece

| Produto | Módulo | Notas |
|---|---|---|
| **CATIA V5** | *Electrical Harness Assembly* e *Installation* (segmentos, ramos e pontos de ramificação); *Electrical Harness Flattening* (extrair, achatar, sincronizar) [V, cursos]. | Pago. |
| **SOLIDWORKS Electrical 3D** | *Route Harnesses* (spline ou linha, tangência) e *Flatten Route* (anotação ou fabricação). O desenho do achatado se atualiza com as edições [V]. | Pago. |
| **Inventor Cable & Harness** | Ambiente *nailboard* (prancha de pregos) [V]. | Pago. |
| **Zuken E3.series** | Ponte 3D para Solid Edge e outros MCAD. *Harness Builder* 2025 com copiloto de *formboard* [V]. | Sem preço público [V]. |
| **Siemens Capital Harness** | Não confirmado nesta pesquisa. | — |
| **WireViz / Filare** | YAML → SVG + BOM. GPL-3 [V]. | Documentação, não projeto 3D. |
| **KBL / VEC** | Padrões de troca (prostep ivip). Sem biblioteca aberta achada [V]. | — |

### 6.2 Como funciona no detalhe

**CATIA V5 [V, cursos de revenda e artigos].**
- Dois ambientes de projeto:
  - *Assembly* trabalha o feixe geométrico como um todo.
  - *Installation* define o segmento de feixe e o roteia.
- Segmentos: pontos de início e fim, **ramos** (*branch*) e **pontos de ramificação**.
- Cobertura de proteção aplicada a trechos.
- Verificação de conectividade (atributo "totalmente conectado").
- *Flattening* é outro ambiente: extrai o feixe para um produto novo, o achata, e **sincroniza** com as mudanças do 3D.
- Convenção: feixe em linha simples; revestimento protetor em linha dupla.
- Mudanças de projeto sincronizam do projeto para a manufatura.

**SOLIDWORKS Electrical 3D [V].**
- O esquema elétrico (diagrama unifilar) vem do módulo de esquemático. O 3D roteia.
- Saídas do *Flatten Route*: uma configuração achatada do modelo 3D e um desenho do harness achatado.
- Cinco ferramentas editam a rota achatada, e o desenho acompanha.
- Uma reclamação: o achatado padrão fica difícil de ler em chicotes com muitos ramos [V]; há patente específica sobre isso (tronco principal, *trunk line*) [V].

**Inventor nailboard [V].**
- O 3D pode ser rearranjado, mas o **comprimento é preservado**.
- Cada ponta de fio gera um ponto separado. Pontas no mesmo pino são distribuídas sem sobreposição.
- Cabos "em leque" saem da ponta do segmento (*Fan Out*). *Fan In* colapsa para reduzir ruído.
- A prancha traz BOM, lista de fios, imagens de conectores, cotas e números de pino. Existe tabela de condutores (*net wire list*).

### 6.3 Elogios e reclamações
Pouco material de usuário público. O que apareceu: legibilidade ruim do achatado padrão (SW) [V]. O gargalo clássico do setor, que as fontes confirmam indiretamente, é o **fluxo de dados** ECAD → MCAD, não a geometria (Zuken e Siemens vendem pontes dedicadas) [V].

### 6.4 Bibliotecas e padrões
- **KBL** (prostep ivip PSI 19): XML. Versões 2.4 e 2.5, com esquema XSD no ecad-wiki. Pode haver "dialetos" por empresa, o que exige guias de implementação [V]. O Volkswagen documentou que o 2.5 ainda não é suportado em todo o grupo [V].
- **VEC (VDA 4968):** sucessor, mais amplo. Esquema público, licença não verificada [V; `modulos-novos.md`].
- Nenhuma biblioteca aberta de KBL/VEC encontrada [V]. Escrever e ler XML em Rust com `quick-xml` (MIT) [C].
- Entrada simples: **CSV da lista de fios**, como já prevê o backlog.

### 6.5 Recomendação

**Fluxo ideal:**
1. **Conectores** são componentes da montagem com **interface** de pinos (B99). Cada pino tem nome, tipo de terminal e posição.
2. **Lista de fios** (tabela na árvore): ID, de (conector.pino), para (conector.pino), seção (mm²), cor, tipo, nota. Importa e exporta CSV.
3. **Rota do feixe:** uma "linha de rota" 3D por segmento, desenhada num sketch 3D (B32) por pontos de passagem, **suportes** (presilhas, furos) e raio mínimo de curvatura. Cada segmento tem diâmetro calculado.
4. **Fios:** cada fio é roteado como caminho nos segmentos (menor caminho no grafo). O comprimento do fio é a soma dos trechos, mais folga e tolerância de terminal.
5. **Cobertura** aplicada a trechos (corrugado, termorretrátil, fita).
6. **Verificações:**
   - raio mínimo de curvatura (por exemplo, múltiplo do diâmetro; valor configurável por norma) [C];
   - colisão do tubo de diâmetro d ao longo da rota contra a montagem;
   - fio sem rota;
   - pino duplicado.
7. **Achatar (formboard):**
   - planifica a árvore do feixe começando pelo tronco (caminho mais longo);
   - cada segmento é uma reta do seu comprimento real;
   - ramos saem no ângulo projetado do 3D, com opção de leque para resolver sobreposição.
8. **Saídas:** lista de fios CSV, lista de corte, desenho do formboard em DXF (B105, B86), BOM.

**MVP (≈ 400 k, sem ECAD):**
- modelo de dados;
- importação de CSV;
- rota por sketch 3D;
- comprimentos;
- achatamento básico;
- DXF;
- tabelas.

**Depois:**
- KBL e VEC;
- verificação de colisão do feixe;
- etiquetas;
- teste de continuidade;
- bibliotecas de conectores.

### 6.6 Integrações
B32 (sketch 3D), B105 (DXF), B99 (interfaces), B15 (referências), B127 (rota de tubo), B163 (caminho em tubo), B101/B102.

**Observação de arquitetura.** Rota de tubo (B127), caminho de corte (B163) e feixe (B138) compartilham a mesma base: **uma polilinha 3D com raio, suportes, comprimento e achatamento**. Construir uma "linha de rota" comum evita três implementações paralelas.

### 6.7 Riscos
1. **Legibilidade do achatado.** O algoritmo ingênuo gera sobreposição.
2. **Dialetos do KBL:** a interoperabilidade real exige testes com arquivos de terceiros.
3. **Demanda concentrada** em setores específicos. Sem ECAD embutido, o fluxo depende de importar a lista.
4. **Normas:** raios e folgas dependem de norma (aeronáutica, automotiva), e dados com licença fechada entram como biblioteca de nível B/C (regra do B42).

### 6.8 Testes de aceite
- Comprimento de cada segmento achatado = comprimento 3D (erro ≤ 0,01 mm).
- Soma dos comprimentos dos fios = soma dos trechos percorridos mais folgas.
- Chicote em "Y": os dois ramos saem do ponto de ramificação sem sobreposição.
- Alterar o caminho 3D ⇒ o formboard atualiza e a tabela muda.
- Round-trip CSV ⇒ idêntico.

### 6.9 Tamanho
400 k é realista para o MVP. KBL/VEC fica como item posterior (≈ 250 k).

---

## 7. B137 — Laminado: camadas e ply book

### 7.1 Quem oferece

| Produto | Módulo | Notas |
|---|---|---|
| **CATIA** | *Composites Design* (CPD): método de grade (*grid*), ponto e curva-semente, análise de produzibilidade, *draping* dinâmico, ply book por desenho [V]. | Pago. |
| **Siemens Fibersim** | Pro e Elite, dentro de NX, CATIA V5 e Creo: ply table, simulação de produzibilidade, divisão por *darts* e emendas, exportação de padrão plano para nesting [V]. | Sem preço público [V]. |
| **Creo Composite Design** | *Draping* cinemático, padrão plano, export DXF/DWG, ply book [V]. | Pago. |
| **Ansys ACP** | Pré e pós. Conjuntos de seleção orientados, rosetas, plies de modelagem, *draping*. Entrega ao Mechanical [V]. | Pago. |

### 7.2 Como funciona no detalhe

**Creo (algoritmo mais bem documentado) [V].**
- **Entradas:** direção do rolo do ply, superfície do molde ou do ply abaixo, **ponto-semente**, fibras fixas de urdidura e trama (*warp/weft*).
- **Algoritmo cinemático ("fishnet"):**
  - calcula pontos de *drape* (cruzamentos das fibras) que têm de estar sobre a superfície;
  - as fibras são inextensíveis, então o espaçamento entre pontos vizinhos é constante;
  - as fibras fixas dividem o ply em quatro quadrantes, resolvidos de forma independente.
- **Parâmetros:** passo (*step length*), deslocamento do ângulo de *drape*, ângulo de simplificação.
- **Saídas:**
  - cisalhamento (ângulo = 90° − α₁);
  - orientações previstas das fibras;
  - espessura após *draping* (área do elemento cisalhado sobre a do quadrado);
  - padrão plano para corte.
- **Semáforo** de cisalhamento contra os ângulos de aviso e limite do material.
- Pré-visualização do padrão plano ply a ply e exportação DXF/DWG para nesting.

**CATIA CPD [V, parcial].**
- Plies definidos por zona, na sequência de empilhamento.
- A **posição da semente** muda o padrão plano.
- Produzibilidade = análise de *draping*. Documentação de chão de fábrica via ply book em desenho.

**Fibersim [V].**
- Produzibilidade → padrão plano com *darts* e emendas → exportação para nesting.
- Dados não geométricos (nome, orientação, sequência, material, área do padrão plano, custo, massa do laminado) vão para outros sistemas.

**Ansys ACP [V].**
- Plies se aplicam a **conjuntos de seleção orientados** (OSS), não a conjuntos de elementos.
- Cada OSS tem lado da casca e direção de referência (0°), dada por uma **roseta**.
- Sem *draping*, a roseta é só projetada; com *draping*, as orientações cisalham pela forma.
- O ACP só prepara; a solução roda em Mechanical.

### 7.3 Elogios e reclamações
Material de usuário público é escasso. Ficou claro apenas que o resultado depende da semente [V] e que o aspecto mais difícil é a superfície do molde (OML), conforme `modulos-novos.md` (nicho aeronáutico, P3).

### 7.4 Bibliotecas
- Nenhuma biblioteca aberta madura [V; `modulos-novos.md`].
- O método *fishnet* (modelo cinemático clássico) é implementável. Referência de origem: Mack & Taylor (1956) [C].
- Teoria clássica de laminados (matriz ABD) é álgebra pequena em Rust [C].

### 7.5 Recomendação

**Fluxo ideal:**
1. **Laminado** na árvore, ligado a uma **superfície de molde** (OML). Material de ply: UD ou tecido, espessura, densidade, limite de cisalhamento.
2. **Roseta** (sistema de referência 0°), como no ACP, com projeção sobre a superfície.
3. **Plies** por zona: contorno (curva ou face), material, ângulo, posição na sequência. Comandos de espelho para sequências simétricas.
4. **Tabela de empilhamento** por zona, com espessura total calculada e **massa** por ply e do laminado.
5. **Sólido ou casca** do laminado, gerado por deslocamento da superfície pela espessura acumulada.
6. **Padrão plano** (item posterior): *draping* por semente.
7. **Ply book:** tabela (nome, material, ângulo, área, massa, custo) e DXF por ply, via B86.

**MVP (≈ 450 k, dividido):**
- **Esta etapa:** plies, roseta, zonas, empilhamento, espessura e massa, ply book em tabela e desenho, sólido do laminado.
- **Itens novos:** *draping* e padrão plano (≈ 300 k); teoria de laminados e seção composta para FEM (≈ 200 k).

### 7.6 Integrações
B04 (aba por superfície, OML viva), B34 (curvatura, superfícies), B86 (DXF), B92 (massa), BN04 (nesting), B159/B160 (seção composta no CalculiX: `*SHELL SECTION, COMPOSITE` [C]), B107 (desenho).

### 7.7 Riscos
1. **Superfície de molde** com continuidade e deslocamento robustos (B34).
2. **Convergência do *draping*:** é sensível à semente, e a superfície precisa ser suave.
3. **Descontinuidades de espessura** (drop-offs): regras de rampa dependem da norma do cliente [C].
4. **Dados de material** (limite de cisalhamento) são proprietários.

### 7.8 Testes de aceite
- Cilindro ou cone (desenvolvíveis): cisalhamento ≈ 0 e área do padrão = área da superfície.
- Calota esférica: cisalhamento máximo concorda com o modelo cinemático de referência (erro a definir na implementação).
- Massa do laminado = Σ(área × espessura × densidade).
- Mudar a superfície ⇒ os plies se reanexam, ou o estado "falhou" é explícito.

### 7.9 Tamanho
A parte de plies, zonas e ply book fica em ~350 k. O total com *draping* e CLT/FEM chega a ~850 k.

---

## 8. B148 — Recursos de plástico (boss, snap, grelha, lip)

### 8.1 Quem oferece

| Produto | O que tem |
|---|---|
| **Fusion (Product Design Extension)** | **Regras plásticas**: espessura, ângulo de saída e raio nominal como parâmetros consumidos por Extrude, Rib, Web, Boss, Snap Fit, Emboss, Thicken, Shell e Draft. Boa prática: atribuir a regra antes de modelar. Regras base métricas e imperiais; criação local ou na nuvem [V]. Comando Boss com nervuras, furo e fixadores automáticos [V]. Snap-fit em balanço [V]. *Web* com filetes e saída automáticos [V]. **Lip & groove não existe** (pedido em fórum) [V]. |
| **Inventor** | Painel de plástico **desligado por padrão**: Boss (cabeça e rosca, nervuras de fixação), Rest, Rule Fillet, Grille, Snap Fit, Lip [V]. |
| **SOLIDWORKS** | Lip/Groove, Snap Hook, Vent, Mounting Boss [C]. |

### 8.2 Como funciona (melhor prática)
- **Regras de projeto** que viram parâmetros: nervura = 0,5 × a parede principal; filete na base da nervura > ¼ da espessura da nervura [V, apresentação de regras].
- **Boss:** nervuras são úteis quando o comprimento do boss passa de 3 × o diâmetro [V, ajuda Autodesk].
- **Snap-fit em balanço:** a deformação admissível é o critério central: ε ≈ 1,5·t·y/L² para seção constante [C].
- **Grade/respiro:** padrão de ranhuras com nervuras, borda e saída.
- **Lip & groove:** junta entre carcaças com folga.

### 8.3 Elogios e reclamações
- A existência do painel escondido no Inventor mostra um problema de **descoberta** [V].
- No Fusion, usuários pedem Lip & Groove [V].

### 8.4 Bibliotecas
Nenhuma. É geometria paramétrica sobre o OCCT.

### 8.5 Recomendação

**Fluxo ideal:**
1. **Regra plástica** como objeto do documento (item novo): espessura nominal, saída, raio nominal, limites. Ela vem da biblioteca de materiais e é editável.
2. Cada recurso lê a regra por padrão e mostra "sobrescrever" como opção avançada.
3. **Boss:** suporte (ponto ou face), diâmetros externo e interno, altura, saída, filete na base, nervuras (número e espessura), rosca ou inserto.
4. **Snap-fit:** suporte, comprimento, espessura, profundidade do gancho, ângulo de entrada. **Verifica a deformação** e avisa quando passa do limite.
5. **Grelha:** face, tamanho, passo, orientação, borda.
6. **Lip & groove:** curva da junta, folga, altura e saída.
7. Todos entram na árvore, usam referências estáveis (B15) e dão erro nomeado quando o suporte some (B75).

**MVP (≈ 250 k):** Boss com nervuras, Grelha, Lip & groove, Snap-fit em balanço com verificação de deformação. Diferencial: **regra plástica integrada** (item novo, ≈ 150 k).

### 8.6 Integrações
B06 (casca), B11 (nervura), B48 (furo/escareado), B129/novo (saída e undercut), B15, B75.

### 8.7 Riscos
Interferência entre recursos (boss encostando na parede); saída aplicada a faces curvas; falha da casca com bosses, que o B06 deve resolver.

### 8.8 Testes de aceite
- Boss com nervuras: espessuras e filetes seguem a regra (tolerância de 1e-6).
- Mudar a espessura da regra ⇒ todos os recursos atualizam.
- Snap-fit com deformação acima do limite ⇒ aviso na árvore.
- Suporte apagado ⇒ estado "falhou" com nome do recurso.

### 8.9 Tamanho
250 k (como está), mais 150 k para a regra plástica.

---

## 9. B129 — Análise de moldagem (saída, undercut, partição)

### 9.1 Quem oferece

| Produto | O que tem |
|---|---|
| **SOLIDWORKS Mold Tools** | Análise de saída, de *undercut*, de linha de partição; linha de partição; **superfícies de fechamento (*shut-off*)** (sem preenchimento, tangente, contato); superfície de partição; **Tooling Split** com tamanho do bloco. Análises feitas na GPU [V]. |
| **Creo EMX** | Projeto de base de molde a partir de montagem, bibliotecas de 17+ fornecedores, posicionamento automático de extratores e canais, interferência, abertura animada [V]. |
| **NX Mold Wizard** | Fluxo passo a passo, partição, verificação de moldabilidade, resfriamento, BOM e desenho automáticos [V]. |
| **Inventor Mold Design** | Mencionado no `modulos-novos.md` [C]. |

### 9.2 Como funciona (SW, a referência) [V]
1. **Análise de saída:** marca faces sem saída suficiente.
2. **Análise de *undercut*:** áreas presas que impedem a ejeção.
3. **Linha de partição:** o contorno entre faces com saída positiva e negativa. Há análise de linhas candidatas.
4. **Superfícies de fechamento:** fecham furos passantes para o núcleo e a cavidade se vedarem. A ferramenta detecta conjuntos de furos sozinha.
5. **Superfície de partição:** extrudada a partir da linha.
6. **Tooling Split:** gera núcleo e cavidade com os blocos dimensionados.

### 9.3 Elogios e reclamações
Pouco material de usuário. O ponto crítico conhecido é a linha de partição em faces curvas (a face precisa ser dividida na silhueta) [C].

### 9.4 Bibliotecas
Nenhuma específica [`modulos-novos.md`]. Catálogos de base de molde (HASCO, DME) têm direitos autorais e entram como dados de nível B/C.

### 9.5 Recomendação

**Fluxo ideal (painéis suporte → modo → parâmetros → avançado):**
1. **Direção de abertura** (suporte: eixo, face ou sistema de coordenadas).
2. **Análise** (item novo): saída por face (faixas de cor), *undercut* por visibilidade, classificação em cavidade, núcleo ou lateral.
3. **Linha de partição:** automática pela silhueta, com divisão das faces curvas, editável.
4. **Fechamento:** detecção automática de furos passantes, preenchidos com plano (o Fenkra já tem Fill, B09).
5. **Superfície de partição:** estendida da linha (reta, tangente ou raio).
6. **Divisão:** bloco com folgas → núcleo e cavidade como corpos, com abertura animada.

**MVP (≈ 350 k):** classificação, linha de partição por silhueta, fechamento automático de furos, superfície de partição reta, Tooling Split. A análise de saída e *undercut* vira item novo (≈ 180 k), reutilizável no B164 (balanço) e no B148.

**Depois:** base de molde com catálogo, extratores, canais de refrigeração, laterais e levantadores.

### 9.6 Integrações
B09 (Fill/Join), B06 (casca), B75 (erros nomeados), B164 (balanço em impressão 3D), B148, B119.

### 9.7 Riscos
Silhueta em geometria complexa; furos não passantes ou irregulares; modelos com saída zero (planos verticais) geram ambiguidade na classificação.

### 9.8 Testes de aceite
- Caixa com saída de 2°: todas as faces laterais classificadas como "saída OK".
- Peça com furo lateral ⇒ *undercut* detectado.
- Núcleo + cavidade + peça = bloco (volume conservado em 1e-6).
- Furo passante ⇒ fechamento automático listado.

### 9.9 Tamanho
350 k (como está); a análise sai como item próprio (≈ 180 k) e o B129 fica em ≈ 250 k.

---

## 10. Mercado

### 10.1 Dimensão
As estimativas variam muito por definição:
- Software de simulação em geral: ≈ US$ 21,9 bi em 2025 (SNS Insider), 18,5 bi em 2024 (SkyQuest). A ABI Research é mais conservadora: US$ 14,65 bi em 2035 [V].
- A ABI atribui 80,4% da receita de 2025 à simulação de produto. A nuvem é o segmento de crescimento mais rápido (≈ 15,6% ao ano) [V].
- *Software* de projeto de chicote elétrico: ≈ US$ 245,7 milhões em 2025 numa definição estreita; outras fontes falam em US$ 1,4–2,1 bi para CAD de chicote automotivo [V]. As definições não são comparáveis.
- Não achei números confiáveis para **compósitos** nem para **simulação de moldagem** nesta pesquisa. Não vou inventá-los.

**Limite desses números:** são relatórios de pesquisa de mercado sem metodologia verificável. Servem para ordem de grandeza, não para planejamento.

### 10.2 Quem compra

| Segmento | O que compra | Observações |
|---|---|---|
| **Projetistas de PME** (máquinas, produtos) | FEM linear embutido no CAD, validação rápida | O maior volume. É o público do SOLIDWORKS Simulation e do Fusion. |
| **Aeronáutico e defesa** | Compósitos, chicote (EWIS), FEM, inspeção | Exigem rastreabilidade e muitas vezes **operação local, sem nuvem** [C]. Ferramentas líderes são CATIA, NX e Fibersim. Um CAD aberto não substitui ferramentas validadas, mas atende suprimentos e protótipos. |
| **Caldeiraria e estruturas** | Pórtico (B114), FEM de suportes, corte de tubo | Casa com a bancada Estrutural. Vasos de pressão exigem linearização de tensões por norma [C]. |
| **Impressão 3D** | Balanço, orientação, partição, otimização topológica | B164/B165. FEM ajuda em rigidez e deformação. |
| **Plástico e ferramentarias** | Recursos plásticos, saída, partição | Há polos de ferramentaria no país (por exemplo, no Sul) [C, a confirmar]. Simulação de injeção (Moldflow, SW Plastics) é cara: SW Plastics a ≈ US$ 3 mil (Standard) a US$ 12 mil (Premium) por ano em revenda americana, e Moldflow Adviser a ≈ NZ$ 13 mil por ano [V]. |
| **Educação** | CAD gratuito e aberto, simulação básica | O FreeCAD é a referência. **Equipes de competição** (Baja, Fórmula SAE, AeroDesign) precisam de FEM e CFD simples [C, mercado local]. |
| **Makers** | FEM básico, 3MF | Preço zero. |

### 10.3 Onde o Fenkra pode se destacar
1. **Simulação que acompanha a edição.** Condições de contorno em referências estáveis e resultados com estado *desatualizado*. É o ponto fraco do FreeCAD (objetos órfãos, malhas falsas) e um incômodo recorrente nos CAD pagos [V parcial].
2. **Diagnóstico explícito.** O verificador de modelo antes de rodar, em português, e a regra de "nada ignorado em silêncio" (B75).
3. **Operação 100% local** e arquivos abertos (`.inp`, VTK, 3MF, KBL, DXF). Relevante para defesa e aeronáutica e para quem não aceita nuvem.
4. **Cadeia integrada da Estrutura:** esqueleto → membros → pórtico (B114) → FEM → corte de tubo (B155/B163). Nenhum concorrente aberto tem isso.
5. **Português e normas brasileiras** (perfis Gerdau, NBR 8800).
6. **Preço:** as extensões pagas custam de ≈ US$ 1,5 mil a vários milhares por ano; o Fenkra é LGPL.

**Onde NÃO competir:** FEM de alta fidelidade (Ansys, Abaqus, Nastran), certificação aeronáutica, simulação de injeção completa, CFD multifísico. Declarar isso no produto.

---

## 11. Fontes (URLs)

**FEM e solvers**
- https://wiki.freecad.org/FEM_Workbench (e subpáginas FEM_CompEmConstraints, FEM_SolverCalculiX, FEM_CalculiX; bloqueadas na leitura direta, só resultados de busca)
- https://blog.freecad.org/2025/09/09/what-is-new-in-fem-for-freecad-1-1/
- https://wiki.freecad.org/Translations:Release_notes_1.1/56/en
- https://industryinsider.eu/3d-designing-and-simulation/freecad-1-1-improvements-following-the-latest-major-update/
- https://github.com/FreeCAD/FreeCAD/issues/6553
- https://github.com/FreeCAD/FreeCAD/issues/5691
- https://bugs.archlinux.org/task/74039
- https://bugs.archlinux.org/task/73154
- https://gitlab.alpinelinux.org/alpine/aports/-/issues/13252
- https://github.com/NGSolve/netgen
- https://onelab.info/pipermail/gmsh/2019/013350.html
- https://onelab.info/pipermail/gmsh/2010/005322.html
- https://packages.gentoo.org/packages/sci-libs/calculix-ccx
- https://packages.msys2.org/packages/mingw-w64-ucrt-x86_64-calculix-ccx
- https://github.com/GeneralElectric/CalculiX
- https://calculix.discourse.group/t/need-for-comprehensive-updated-calculix-2-20-windows-install-guide-and-windows-install-help/1415
- https://github.com/wildmeshing/fTetWild
- https://wiki.osarch.org/salome-meca/
- https://wiki.osarch.org/index.php/Code_Aster
- https://zoftwarehub.com/products/salome-meca/reviews

**Produtos comerciais de simulação**
- https://www.autodesk.com/products/fusion-360/simulation-extension
- https://help.autodesk.com/cloudhelp/ENU/Fusion-Extensions/files/EXT-SIMULATION.htm
- https://www.autodesk.com/products/fusion-360/blog/static-stress-simulations-autodesk-fusion/
- https://www.hagerman.com/software/fusion-simulation-extension
- https://www.cati.com/blog/comparing-solidworks-packages-standard-vs-professional-premium/
- https://www.javelin-tech.com/blog/2022/12/solidworks-simulation-solutions-premium-vs-standard/
- https://store.trimech.com/products/solidworks-simulation-premium-3-month-term-license/page/5
- https://blogs.solidworks.com/products/solidworks/solidworks-simulation-2021-enhanced-contact-for-faster-more-stable-and-accurate-results/
- https://g2.com/products/solidworks-solidworks-simulation/reviews
- https://capterra.com/p/124171/SolidWorks-Simulation-Premium/reviews/
- https://www.onshape.com/en/blog/fea-design-engineers-cad-integrated-linear-static-analysis
- https://www.onshape.com/de/blog/introducing-modal-analysis-onshape-simulation
- https://toolradar.com/tools/onshape/pricing
- https://help.autodesk.com/cloudhelp/2026/ENU/NINCAD-UsersGuide/files/GUID-538CBCDD-C88A-460A-ABD1-1BA7319BF65D.htm
- https://autodesk.com/products/inventor-nastran/overview
- https://blogs.autodesk.com/simulation/2017/08/16/inventor_professional_or_nastran_in-cad
- https://www.symetri.co.uk/insights/blog/nastran-vs-stress-analysis
- https://us.fitgap.com/products/004967/ansys-discovery
- https://simutechgroup.com/?p=19132
- https://www.simscale.com/?p=111634
- https://capterra.com/p/145960/SimScale/pricing
- https://www.g2.com/products/simscale/reviews

**CFD**
- https://github.com/jaheyns/CfdOF
- https://wiki.freecad.org/CfdOF_Features
- https://wiki.freecad.org/CfdOF_CFD_Mesh
- https://www.slideshare.net/slideshow/cfd-tutorial-1-freecad-computational-fluid-dynamics/266820052
- https://redlib.hackliberty.org/r/CFD/comments/1laddmw/opensource_openfoam_workflow
- https://sourceforge.net/p/cfmesh/
- https://bugs.openfoam.org/view.php?id=1376
- https://en.wikipedia.org/wiki/OpenFOAM
- https://cfd.direct/openfoam/about/
- https://www.cati.com/?p=130349
- https://www.goengineer.com/en-us/blog/creating-lids-solidworks-flow-simulation
- https://www.goengineer.com/en-us/blog/solidworks-flow-simulation-solution-adaptive-mesh-refinement
- https://solidsolutions.co.uk/blog/2019/05/finding-a-good-mesh-fast-in-SOLIDWORKS-flow-simulation
- https://3dswym.3dexperience.3ds.com/question/solidworks-user-forum/is-having-more-cells-across-a-tight-restriction-better_7WNCF1KBQJO5UsNQERPe0A

**Chicote**
- https://www.goengineer.com/en-us/training/catia-v5-electrical-harness-flattening
- https://wichita.edu/industry_and_defense/NIAR/Documents/Electrical-Harness-Design-R30.pdf
- https://icas.org/ICAS_ARCHIVE/ICAS2012/PAPERS/800.PDF
- https://public.dhe.ibm.com/software/plm/uk/catia_electrical_process.pdf (não consegui abrir)
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/catia-electrical-harness-installation-2-ehi_yhoHvpJrSDqzC-27gBlJHA
- https://help.solidworks.com/2024/English/swelec/r_swelec_route_harness.htm (página vazia na leitura; usei os resultados de busca)
- https://www.goengineer.com/en-us/blog/edit-flattened-harness-routes-solidworks-electrical-3d
- https://patents.google.com/patent/US11755811
- https://help.autodesk.com/cloudhelp/2020/ENU/Inventor-Help/files/GUID-3AF3D756-F600-404A-9E64-E2EB0B39333C.htm
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-C8729954-E17D-442F-9AC8-1956D4E49427.htm
- https://blogs.sw.siemens.com/solidedge/e3-series-combined-with-solid-edge-ensures-seamless-wire-harness-design/
- https://www.digitalengineering247.com/article/zuken-introduces-harness-builder-2025/zuken
- https://zuken.com/en/product/e3series/wire-harness-design-and-manufacturing-ecosystem
- https://ecad-wiki.prostep.org/specifications/kbl/
- https://ecad-wiki.prostep.org/specifications/kbl/v24/
- https://www.iana.org/assignments/media-types/application/kbl%2Bxml
- https://pypi.org/project/filare/
- https://www.researchandmarkets.com/reports/6090408/electrical-wiring-harness-design-software
- https://marketintelo.com/report/automotive-wire-harness-cad-software-market

**Compósitos**
- https://support.ptc.com/help/creo/creo_pma/r12/usascii/composite_design/about_draping_simulation.html
- https://support.ptc.com/help/creo/creo_pma/r12/usascii/composite_design/draping_simulation.html
- https://support.ptc.com/help/creo/creo_pma/r13/usascii/composite_design/create_flat_plies.html
- https://support.ptc.com/help/creo/creo_pma/r11.0/usascii/whats_new_pma/composite_draping_algorithm.html
- https://www.technia.com/en/training/catia-v5-composite-part-design/
- https://www.goengineer.com/training/catia-v5-composites-design-manufacturing
- https://3dswym.3dexperience.3ds.com/wiki/catia-user-community/catia-composites-manufacturing-engineer-for-transportation-mobility_G62Zw_avQGSBzbFUInzrWg
- https://www.siemens.com/en-us/products/designcenter/cad-software/offerings/fibersim-composites/
- https://ata-e.com/wp-content/uploads/2024/10/Siemens-PLM-Fibersim-Pro-and-Elite-Fact-Sheet.pdf
- https://www.femto.eu/wp-content/uploads/2023/08/Siemens-SW-Fibersim-17-Fact-Sheet.pdf
- https://ansyshelp.ansys.com/public/Views/Secured/corp/v251/en/acp_ug/acp_oes.html
- https://ansyshelp.ansys.com/public/Views/Secured/corp/v261/en/acp_tut/acp_tut7_modeling.html
- https://ansys.com/training-center/course-catalog/structures/draping-in-composites
- https://innovationspace.ansys.com/forum/?p=138950

**Plástico e moldes**
- https://www.autodesk.com/products/fusion-360/blog/plastic-rules-fusion-product-design-extension/ (403 na leitura; usei o resumo de busca)
- https://www.autodesk.com/products/fusion-360/blog/how-to-manage-plastic-rules-fusion-360-product-design-extension/
- https://www.symetri.co.uk/insights/blog/autodesk-fusion-360-product-design-extension
- https://forums.autodesk.com/t5/fusion-360-feedback-hub/open-plastic-parts-discussion/m-p/10283415/highlight/true
- https://help.autodesk.com/cloudhelp/2023/ENU/Inventor-Help/files/GUID-29104E83-91D9-4B20-A6C9-C19DA7AABD6E.htm
- https://forums.autodesk.com/t5/inventor-forum/plastic-part-features/td-p/7633529
- https://static.au-uw2-stg.autodesk.com/Class_Presentation_CP502618_ClassPresentationCP502618-Odugu-AU2022.pdf
- https://help.solidworks.com/2023/English/solidworks/sldworks/c_Mold_Analysis_Tools.htm
- https://www.goengineer.com/blog/solidworks-mold-tools-shut-off-surfaces-parting-surfaces-tooling-split
- https://help.solidworks.com/2022/English/SolidWorks/sldworks/c_Mold_Design.htm
- https://www.hitachi.co.jp/products/it/industry/solution/swplastics/price.html
- https://store.trimech.com/products/solidworks-plastics-standard
- https://visiativ.co.uk/solutions/solidworks-plastics-simulation/
- https://www.autodesk.com/products/moldflow
- https://www.ki-syndikat.de/tools/autodesk-moldflow/
- https://www.cdwg.com/product/autodesk-moldflow-insight-premium-subscription-renewal-annual-1-seat/9116016
- https://support.ptc.com/help/emx/r15.0.0.0/en/EMX/docu_workflow_the-principal-workflow.html
- https://www.ptc.com/en/store/products/creo/creo-expert-moldbase-extension
- https://blogs.sw.siemens.com/nx-manufacturing/webinar-improve-mold-design-productivity-through-digitalization-knowledge-driven-automation/

**Mercado**
- https://www.globenewswire.com/fr/news-release/2025/11/19/3191125/0/en/Simulation-Software-Market-to-Reach-USD-56-13-Billion-by-2033-Driven-by-Rising-Demand-for-Cost-Efficient-Product-Design-and-Testing-Research-by-SNS-Insider.html
- https://www.abiresearch.com/news-resources/chart-data/simulation-sofware-market-size-global-outlook
- https://technavio.com/report/simulation-software-market-industry-analysis
- https://www.marketsandmarkets.com/Market-Reports/simulation-software-market-1356.html

---

**Handoff**
- **Resultado:** relatório de benchmarking para os 8 itens, com a seção de mercado e o JSON de ajustes.
- **Evidência:** ~45 buscas e leituras web. Os itens de maior confiança são licenças (Netgen, CfdOF, Gmsh, OpenFOAM, cfMesh, WireViz), preços da Fusion e dos planos de SW Plastics, e o algoritmo de *draping* da Creo.
- **Arquivos alterados:** nenhum.
- **Verificações feitas:** leitura de `backlog.md`, `modulos-novos.md` e áreas do `backlog.json` (somente leitura).
- **Falhas:**
  - o wiki do FreeCAD e o site da Autodesk bloquearam a leitura direta;
  - as páginas de ajuda do SOLIDWORKS vieram vazias;
  - os PDFs da IBM não abriram (sem leitor de PDF);
  - o conteúdo de CATIA Harness/CPD vem de cursos e resumos de busca.
- **Riscos:**
  - preços de revenda, que variam por região;
  - a licença exata do CalculiX (GPL-2 só ou 2+) e do fTetWild;
  - `faer` não verificado;
  - ausência de números de mercado de compósitos e de moldagem;
  - a decisão sobre qual distribuição do OpenFOAM suportar no Windows depende de teste de instalação.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Relatório de benchmarking de FEM/CFD, chicote, compósitos, plástico e moldes entregue com mercado, fontes e ajustes de backlog (8 alterações e 8 itens novos); algumas páginas oficiais não puderam ser lidas e vários preços e licenças ficam marcados para verificação.
