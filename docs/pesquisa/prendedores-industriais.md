# PRENDEDORES E ELEMENTOS DE MÁQUINA INDUSTRIAIS

> Pesquisa de 30/10/2026. **[V]** = confirmado em fonte aberta nesta pesquisa. **[C]** = conhecimento geral, não confirmado agora.
> Texto próprio, sem cópia de trechos. As medidas citadas são fatos dimensionais (ADR 0008). Quase todas vêm de **uma** fonte aberta (fasten.it). A segunda fonte exigida pelo nível C ainda precisa ser aberta e está nomeada em cada ficha.

---

## 1. Resumo e principais lacunas

**Resumo**

1. O Fenkra tem 6 famílias de fixadores (Allen ISO 4762, sextavado ISO 4017/DIN 933, porca ISO 4032/DIN 934, arruela ISO 7089), todas M3–M20, mais a chaveta DIN 6885 A. O tipo é um `enum` fechado de 4 valores (`biblioteca.rs:24-29`). Cada tipo tem um gerador escrito à mão (`fixadores.rs:50-55`).
2. Isso é o oposto do que CAD avançado A biblioteca de normas, CAD avançado C Content Center e catálogos CAD avançado B fazem. Nesses três, a família é dado (tabela + chaves + propriedades) e o gerador é um modelo paramétrico reaproveitado. Para o Fenkra passar de 6 para 100+ famílias sem escrever uma função Rust por norma, é preciso um modelo de dados **genérico**. Ele deve ter famílias com campos livres, geradores nomeados, designações não métricas, propriedades (PN, descrição, material, classe de resistência) e publicações do fixador.
3. O preenchedor de furos (`preencher_furos`) é bom em pilha, pega, comprimento e restrições. Mas só entende rosca métrica M3–M20. Não vê rebaixo nem escareado (`HoleRow` não traz esses dados, `lib.rs:5059`). Sempre põe Allen + arruela, e não deixa trocar a pilha depois.
4. A tabela de comprimentos aceita só `2·d ≤ L ≤ 10·d`. Isso corta comprimentos reais de catálogo (M6 vai até 100 mm ou mais; 10·d = 60) e não serve a parafusos curtos de cabeça baixa.
5. Não existem: pinos, anéis elásticos, chaveta Woodruff/cônica, prisioneiros, autotravantes, flangeados, Belleville/dentada/pressão, insertos, rebites, clinch PEM, nem polegada ASME/UNC.
6. Os fixadores nascem sem PN, sem classe de resistência, sem massa verificada e sempre com "Aço 1020" (`fixadores.rs:129-131`). Isso derruba BOM, desenho e massa.
7. O arquivo `fixadores/<id>_<tam>x<L>.fkp` é reaproveitado se já existir (`montagem/fixadores.rs:374`). Corrigir um dado da biblioteca não atualiza projetos antigos, e o arquivo não guarda versão da biblioteca.
8. Fontes gratuitas existem para quase tudo. fasten.it cobre ISO/DIN com tabelas legíveis. PEM e Nord-Lock publicam suas medidas. Dados ASME em polegada são mais difíceis de achar em tabela aberta única. O 2º conferente precisa ser aberto família por família.
9. Há erros de rótulo de norma em `docs/contextos/fixadores.md`. Por exemplo, "Rivet Nut = ISO 15977" é incoerente com a série ISO 15977–15984, que trata de rebites cegos (fasten.it mostra a ISO 15983 como rebite cego). "Drive Rivet = DIN 7337" também precisa de conferência.
10. Proposta: 16 itens de backlog (seção 4). O de maior retorno é o **modelo de família genérico + propriedades + geradores nomeados**, antes de qualquer família nova.

**Principais lacunas** (resumidas também em SUMMARY): (1) modelo de família fechado e só métrico; (2) sem propriedades (PN, material, classe, massa) nos fixadores; (3) preenchedor de furos cego a rebaixo/escareado/polegada e sem edição da pilha; (4) regra de comprimento `2d–10d` e arquivo cacheado sem versão; (5) ausência de pinos, anéis, chavetas além da A, insertos, rebites e clinch.

---

## 2. Como as três ferramentas organizam fixadores (base comum das fichas)

### 2.0 Modelo de dados nos CADs de referência

**CAD avançado A biblioteca de normas [V]:**
- O conteúdo é agrupado por **norma** e, dentro dela, por **pastas/tipos** (ex.: ISO › parafusos de cabeça sextavada).
- Cada componente tem lista de **tamanhos** e **comprimentos**. O administrador pode desligar opções ou acrescentar customizadas.
- Os dados de rosca são geridos à parte: um tamanho novo de parafuso costuma exigir entrada nova de rosca.
- Cada tamanho vira uma **configuração** com nome gerado.
- Há dois modos de gerar arquivos:
  - "Criar configurações" guarda muitos tamanhos num só arquivo, que cresce e deixa montagens grandes lentas.
  - "Criar peças" gera um arquivo pequeno por tamanho, com muitos arquivos.
  - Existe também a combinação "criar peças ao arrastar com Ctrl".
- Todo componente tem três campos: **Part Number, Descrição e Comentário**. Eles alimentam a lista de materiais e são editáveis em lote por planilha exportada, sem vínculo vivo (precisa reimportar).
- **Propriedades personalizadas** podem ser texto ou lista, específicas da configuração ou não. Podem aparecer no PropertyManager e como colunas da BOM.
- O **material** é uma propriedade em lista ligada à biblioteca de materiais.
- Há "hardware definido pelo usuário": copia-se um componente e acrescentam-se features abaixo da árvore existente.
- O **assistente de furos** usa os mesmos dados do biblioteca de normas (diâmetro de furo, rebaixo, rosca, broca).
- O fixadores inteligentes lê esses furos. Seus ajustes cobrem três coisas: dimensionamento de arruela no empilhamento superior, regras de cálculo de comprimento, e o fixador padrão usado em furos que não vêm do assistente de furos.

**CAD avançado C Content Center [V]:**
- A hierarquia é biblioteca (somente leitura ou leitura/escrita), **categoria**, **família**.
- A caixa **Família** (ao inserir, ou "Mudar tamanho" no clique direito) tem:
  - aba Seleção, com os valores de cada parâmetro e miniatura;
  - Visão de tabela, com uma linha por membro e filtros;
  - aba Informação da família, somente leitura: nome, descrição, organização de norma, norma, revisão, fabricante (gravado na iProperty Manufacturer) e biblioteca de origem.
- Opções de inserção:
  - *Como Personalizado* ou *Como Padrão*;
  - *Usar iMates*;
  - *Substituir todos*, que troca todas as instâncias na montagem;
  - *Favoritos*.
- Para publicar uma peça própria, o **Publish Guide** pede, na ordem:
  1. biblioteca e idioma;
  2. categoria;
  3. **mapeamento das colunas da tabela de família para os parâmetros da categoria** (rosca costuma mapear sozinha);
  4. pelo menos uma **coluna-chave**, que identifica cada membro;
  5. dados da família: nome, descrição, pasta onde os membros são salvos ao entrar na montagem, norma e revisão;
  6. miniatura;
  7. publicar.
- Numa coluna de expressão da tabela de família, `{PARTNUMBER}` pode alimentar a Descrição (relatado em apostila da fornecedor de CAD avançado) [V, fonte secundária].
- A família pode ter o **modelo** substituído ("Replace Family Template"). O CAD avançado C verifica se a peça serve de molde; se houver erro no mapeamento, mantém o modelo original.

**CAD avançado B [V parcial, fontes de comunidade e universidade; resto C]:**
- Peça padrão = uma **CATPart paramétrica + tabela de projeto** (Excel/CSV) cujas primeiras colunas costumam ser `PartNumber` e `PartName` (City University, ainda do V5R5).
- A tabela vira uma **família de componentes** dentro de um **catálogo** (capítulos → famílias → componentes). O catálogo é "resolvido" e os componentes são instanciados pelo Catalog Browser.
- Um fórum relata um problema típico. Ao instanciar um membro de família de tabela de projeto, o CAD avançado B cria uma CATPart nova a salvar, em vez de manter vínculo com o catálogo.
- O V5 não traz "fixador inteligente" de série. O catálogo anterior do projeto cita um complemento FASTJOINT [C].
- Os recursos de referência de que o fixador precisa são **elementos de referência publicados** (eixo, plano de apoio, ponto), que a montagem usa em vez de geometria solta. O ADR 0010 do Fenkra já segue essa direção.
- No Part Design, o **Hole** do V5 define diâmetro, rosca, escareado/rebaixo e profundidade, e é a fonte de dados para o fixador [C].

**Outros:**
- **CAD avançado H [V]:** o comando *Fastener System* escolhe o furo de cima e o de baixo (eles dão diâmetro e direção). Filtra o banco por norma (ANSI/DIN/JIS) e monta o parafuso, o empilhamento de cima e o de baixo. O comprimento pode ser por extensão mínima (passante), engajamento mínimo (cego roscado) ou valor do usuário. Os dois primeiros se atualizam sozinhos. O fixador é uma feature no PathFinder, com Error Assistant se o comprimento deixar de servir. Permite exclusão por membro de família de montagem alternativa. Exige o banco Standard Parts configurado.
- **CAD avançado F [V]:** a Intelligent Fastener Extension (IFX) trata parafusos, arruelas, porcas, **insertos** (prensado e roscado) e pinos. Cria os furos junto com a instalação e tem biblioteca customizável. Por padrão referencia geometria externa, e dá para desligar. Uma versão limitada e gratuita vem com o CAD avançado F.
- **CAD avançado G [V parcial]:** existe `FastenerAssy` com posição, direção, diâmetro, extensão e flag de cego. Em base de fórum, a biblioteca de máquinas precisa ser registrada.
- **Fusion [V]:** insere componentes do site McMaster-Carr e tem uma biblioteca nativa de fixadores.
- **CAD avançado E [V]:** o conteúdo padrão é nativo. Cada item traz um *mate connector* embutido, dá para inserir vários selecionando uma face, e editar PN/Descrição vale para toda a empresa na hora.

### 2.0b O que o Fenkra faz HOJE (resumo técnico)

- **Dados.** `biblioteca/fixadores/*.json` tem um arquivo por família: `id`, `nome`, `norma`, `tipo` (4 valores), `nivel` A/B/C/X, `licenca`, `fontes[]`, `campos[]`, `tamanhos[]` e `comprimentos[]`.
  - Cada tamanho traz `designacao` ("M6"), `valores{}`, `confirmado_por` e `nota`.
  - Os arquivos são embutidos por `include_str!` (`biblioteca.rs:215-222`).
  - `Familia::validar` (`biblioteca.rs:115-173`) exige campos por tipo, valores positivos, um valor por campo, designação "M…" e comprimentos crescentes.
- **Geração.** `fixador_documento` (`fixadores.rs:26`) cria um documento com parâmetros (`d`, `L`, `dk`, `k`, `s`…) e primitivas (cilindro/prisma), sem rosca, chanfro nem filete.
  - Convenção: eixo Z, face de apoio em z=0, corpo para –Z. O sextavado tem vértice em +X.
  - O corpo do documento é "Hardware", com PN vazio e material "Aço 1020" fixos (`:129-131`).
- **Preenchimento.** `preencher_furos` (`fenkra-montagem/src/fixadores.rs:106`):
  1. lê a `hole_table` de cada peça visível;
  2. casa o diâmetro com a folga M3–M20 de `hardware::METRIC` (±0,05 mm) ou lê a rosca "M6/M8x1.25";
  3. agrupa furos colineares (0,05 mm/0,5°) em pilha;
  4. mede a espessura por raios ao lado do furo (`espessura_ao_longo`);
  5. escolhe L = pega + arruelas + porca + 2·P, arredondado para cima na série;
  6. grava as peças em `fixadores/` e cria restrições (concêntrico + contato).
- **Chaveta.** `chaveta_para_eixo` (`biblioteca.rs:323`) devolve b, h, t1, t2 por faixa de Ø, de 6 a 85 mm. É usada em eixos e engrenagens.

---

## 3. Fichas por família / função

Convenção de cada ficha: **Dados** = designação, campos mínimos e fontes. **CAD avançado B/CAD avançado A/Inv.** abreviam o comportamento descrito em 2.0 e dizem só o que é específico da família. **Hoje** cita arquivo:linha.

### 3.1 Parafusos

### Parafuso sextavado, rosca parcial (CAD avançado B: catálogo/CATPart / CAD avançado A: biblioteca de normas ISO 4014 / CAD avançado C: CC ISO 4014 / Fenkra: ❌)
- **O que faz:** parafuso de cabeça hexagonal com haste lisa e rosca no fim. É o "bolt" estrutural e de máquina geral. Normas: ISO 4014 / DIN 931 (e ABNT NBR 5580/6323 como referência [C]).
- **Dados** [V, fasten.it, DIN EN ISO 4014]. Campos: d, P, s, e (min A/B), k, b (três faixas por L), faixa de L.

  | d | s | e min A | k | b (L≤125) | b (125<L≤200) | b (L>200) |
  |---|---|---|---|---|---|---|
  | M6 | 10 | 11,05 | 4 | 18 | 24 | 37 |
  | M8 | 13 | 14,38 | 5,3 | 22 | 28 | 41 |
  | M10 | 16 | 17,77 | 6,4 | 26 | 32 | 45 |

  O 2º conferente ainda deve ser aberto (wermac.org, torqbolt, Ciser, Würth).
- **CAD avançado B:** uma CATPart paramétrica com tabela de projeto (L, d como colunas). **CAD avançado A:** configuração por tamanho e comprimento, com rosca cosmética. **CAD avançado C:** família com colunas-chave (d, L), a rosca vem da categoria. Nos três, a rosca é opção de exibição, não geometria.
- **Fenkra hoje:** não há família. `b` varia com L (o `b` fixo de `iso4762.json` só vale para L≤125).
- **Melhor prática:** acrescentar `b1/b2/b3` como campos opcionais e escolher conforme L. Usar o mesmo gerador "hex_head" de ISO 4017 com um parâmetro `comprimento_rosca`. Teste: M10×30 tem rosca de 26 mm; M10×150 tem 32 mm.

### Parafuso sextavado, rosca total (CAD avançado B: catálogo / CAD avançado A: biblioteca de normas ISO 4017 / CAD avançado C: CC / Fenkra: ✅ parcial)
- **O que faz:** igual ao anterior, mas rosca até a cabeça. ISO 4017 / DIN 933.
- **Dados:** campos hoje: P, s, k. A diferença DIN × ISO em M10 e M12 (s=17 contra 16) já está nos testes (`biblioteca.rs:386-387`) e bate com o ISO 4014 [V].
- **CAD avançado B/CAD avançado A/Inv.:** igual à ficha anterior; a série de comprimentos vem da família.
- **Fenkra hoje:** `sextavado` (`fixadores.rs:82-93`) gera cabeça `cc=s/cos(30)` e haste. Faltam: o chanfro da cabeça (o canto de uma peça real é menor que s/cos30, por exemplo e min 11,05 para s=10, enquanto o prisma dá 11,55, o que superestima o volume da cabeça e a interferência) e o filete sob a cabeça.
- **Melhor prática:** chanfro cônico 30° do topo (parâmetro `e_min` do catálogo) e comparar a massa com a tabela do catálogo (tolerância 5%).

### Parafuso Allen, cabeça cilíndrica (ISO 4762 / DIN 912 — ✅)
- **O que faz:** parafuso de cabeça cilíndrica com sextavado interno. É o mais comum em mecânica e máquinas.
- **Dados** [V, fasten.it]. Campos: P, dk, k, s, t min, b, w min.

  | d | P | dk max | k | s | t min | b |
  |---|---|---|---|---|---|---|
  | M6 | 1 | 10 | 6 | 5 | 3 | 24 |
  | M8 | 1,25 | 13 | 8 | 6 | 4 | 28 |
  | M10 | 1,5 | 16 | 10 | 8 | 5 | 32 |

  Todos batem com `iso4762.json`. Ainda faltam t (profundidade mínima do soquete) e dk com recartilha. Fontes no arquivo: fasten.it, tspi.at, opac.net.
- **CAD avançado B/CAD avançado A/Inv.:** como em 2.0. O CAD avançado C guarda a coluna "comprimento" como chave; o CAD avançado A tem a rosca cosmética da configuração.
- **Fenkra hoje:** `allen` (`fixadores.rs:60-79`). A profundidade do soquete é `k/2` fixa, enquanto o catálogo dá `t min` real (M6: 3 = k/2, M10: 5 = k/2, mas M3/M4/M5 e M16/M20 não seguem exatamente [C]). `b` de M16/M20 vem de regra + 1 fonte (`confirmado_por: 1`).
- **Melhor prática:** trocar `prof=k/2` por campo `t`. Teste de volume contra massa de catálogo. O sextavado interno acima do topo (`prof+1`) deve virar corte passante para evitar sólido residual.

### Parafuso Allen abaulado (botão) — ISO 7380-1/-2 (❌)
- **Dados** [C, a conferir]: M6 dk≈10,5 / k≈3,3 / s=4; M8 dk≈14 / k≈4,4 / s=5; M10 dk≈17,5 / k≈5,5 / s=6. A página fasten.it ISO 7380 não abriu por endereço direto; buscar o slug correto e Würth/Bossard.
- **O que faz:** cabeça em calota baixa, sextavado interno. Uso em carenagens e acabamento.
- **CAD avançado B/CAD avançado A/Inv.:** a cabeça é perfil revolucionado (arco + base). O CAD avançado A/CAD avançado C tratam como outro tipo de cabeça da mesma família de comprimentos.
- **Fenkra hoje:** não existe; só há prisma e cilindro. O gerador "calota" depende de uma primitiva esférica/revolução.
- **Melhor prática:** gerador `cabeca_calota` com `dk`, `k`, raio calculado (`r=(dk²/4+k²)/(2k)`). Teste: altura k e diâmetro dk no bounding box.

### Parafuso Allen escareado (ISO 10642 / DIN 7991 — ❌)
- **Dados** [V, fasten.it DIN EN ISO 10642, ed. 02/2020]:

  | d | P | b | dk max | k max | α | t min | s |
  |---|---|---|---|---|---|---|---|
  | M5 | 0,8 | 22 | 11,20 | 3,1 | 90° | 1,75 | 3 |
  | M6 | 1 | 24 | 13,44 | 3,72 | 90° | 2,2 | 4 |
  | M8 | 1,25 | 28 | 17,92 | 4,96 | 90° | 2,9 | 5 |
  | M10 | 1,5 | 32 | 22,4 | 6,2 | 90° | 3,5 | 6 |

  A página também lista as edições antigas (04/2013 e 06/2004), em que o `t min` de M5, M8 e M10 era um pouco diferente (1,9; 3; 3,6). **Atenção:** a edição vigente muda o dado, então a família deve guardar a **revisão** (ver RNF-PRD-03).
- **CAD avançado B/CAD avançado A/Inv.:** o furo escareado vem do assistente de furos (CAD avançado A), do Hole (CAD avançado B) ou da categoria (CAD avançado C); o parafuso se apoia no cone. O CAD avançado C trata a face de apoio como o cone, não como plano.
- **Fenkra hoje:** não existe. `hardware::METRIC` tem `csink_diameter` (M6: 13,7), mas `HoleRow` não carrega tipo de furo, e `preencher_furos` sempre assenta a cabeça sobre a face.
- **Melhor prática:** `HoleRow` precisa de `kind` (reto/rebaixo/escareado), `cbore_d`, `cbore_depth`, `csk_d`, `csk_angle`. A família diz qual apoio usa (`plano`, `cone`). O parafuso escareado publica o plano de topo (rente) e o ponto do cone. Teste: cabeça rente à face ±0,05 mm.

### Parafusos de cabeça panela, escareada e abaulada com fenda/Phillips/Pozidriv (ISO 7045/7046/1580/2010 — ❌)
- **Dados** [V parcial]. ISO 7045 (M4: dk 8, k 3,1, recesso 2; M5: 10 / 3,8 / 2; M6: 12 / 4,6 / 3; M8: 16 / 6 / 4). **Cuidado:** o título da página fasten.it diz "countersunk flat head", mas os valores são de cabeça panela; o dado está certo, o rótulo do site é que parece trocado. Conferir em segunda fonte. ISO 7046 (escareada), 1580 (fenda panela), 2010 (escareada abaulada): [C].
- **O que faz:** parafusos de máquina de baixa carga.
- **CAD avançado C/CAD avançado A:** o recesso (cruz) é perfil extrudado; muitos catálogos usam um símbolo simplificado. No CAD avançado B costuma ser o mesmo perfil de revolução da calota.
- **Fenkra hoje:** não existe.
- **Melhor prática:** o recesso cruciforme é desenho (perfil fixo por número do recesso: 1, 2, 3, 4), não tabela. Gerar um corte simples e guardar o número do recesso como propriedade. Teste: a massa não pode variar mais de 3% pelo recesso.

### Parafusos Torx / hexalobular (ISO 14583 panela, 14584 escareado, 14579 cilíndrico — ❌)
- **Dados:** [C] designações T10, T20, T25, T30, T40 por tamanho; tabela A/Ai de abertura. Fonte a abrir: fasten.it, ISO 10664 (tamanhos de recesso).
- **O que faz:** soquete de seis lóbulos; mais torque, menos desgaste.
- **CAD avançado A/CAD avançado C:** é tipo de cabeça + tamanho de drive. Em ambos aparece como propriedade, o desenho do lóbulo é simplificado.
- **Fenkra hoje:** não existe.
- **Melhor prática:** o drive vira **campo textual** (`drive: "T25"`) na tabela, sem geometria própria (hexágono aproximado). Isso exige permitir valores texto em `valores{}` (hoje só `f64`, `Tamanho.valores: BTreeMap<String,f64>` em `biblioteca.rs:67`).

### Parafuso flangeado (ISO 4162 / DIN 6921 — ❌)
- **Dados** [C]. M6: s=10, dc≈14,2, k≈6,6 (DIN 6921). O flange do parafuso segue o da porca DIN 6923 [V]: M6 dc 14,2; M8 17,9; M10 21,8; M12 26. Conferir cabeça DIN 6921 em fasten.it.
- **O que faz:** cabeça sextavada com flange integrado; dispensa arruela.
- **CAD avançado C/CAD avançado A:** o flange é cone mais cilindro sob a cabeça; as configurações carregam `dc`.
- **Fenkra hoje:** a pilha sempre põe arruela sob a cabeça (`preencher_furos`, `op.arruela`).
- **Melhor prática:** o campo da família `inclui_arruela: true` desliga a arruela automática. Teste: pilha M8 flangeado não leva ISO 7089.

### Prisioneiro (DIN 938/939/835, ISO 15) (❌)
- **Dados** [C]. Comprimento de rosca no lado fixo: DIN 938 ≈ 1·d (aço), DIN 939 ≈ 1,25·d (ferro fundido), DIN 835 ≈ 2·d. Duas extremidades rosqueadas, sem cabeça. Rosca do lado da porca b e do lado fixo bm.
- **O que faz:** haste rosqueada nas duas pontas; fixa em furo roscado e recebe porca.
- **CAD avançado C/CAD avançado A:** o CAD avançado A trata como parafuso sem cabeça com configurações bm. O CAD avançado C tem categoria própria.
- **Fenkra hoje:** `preencher_furos` trata furo roscado como "parafuso sem porca" (`com_porca = op.porca && !roscada`). Com prisioneiro, o furo roscado recebe **também** porca.
- **Melhor prática:** `bm` define o engajamento no furo cego; o preenchedor calcula L = bm + pega + arruela + porca + 2P. O prisioneiro publica **dois** planos de apoio (lado fixo e lado livre).

### Parafuso de ombro (ISO 7379 — ❌)
- **Dados:** [C] diâmetro do ombro d1 (6, 8, 10, 12, 16, 20), rosca menor (M5, M6, M8, M10, M12, M16), cabeça Allen. Fonte: fasten.it, Misumi, McMaster. Página `din_en_iso_7379` retornou 404; achar o slug certo.
- **O que faz:** ombro retificado como eixo de articulação ou guia, com rosca menor que o ombro.
- **CAD avançado C/CAD avançado A:** o ombro é a referência de concêntrico; o comprimento é o do ombro, não o total.
- **Fenkra hoje:** o parafuso tem `L` único. Não há "comprimento do ombro" nem "rosca menor".
- **Melhor prática:** parâmetros `d1`, `L1` (ombro), `d_rosca`, `L_rosca`; o fixador publica **eixo** e **plano do ombro** para o contato.

### Parafusos autoatarraxantes, para chapa e autobrocantes (ISO 1481/7049/7050, DIN 7976, DIN 7504 — ❌)
- **Dados:** [C]. Diâmetros ST2,2 a ST6,3 (ex.: ST4,2; ST4,8). Ponta A/B/C, cabeça panela/sextavada/escareada, tabela de pré-furo na chapa por espessura.
- **O que faz:** formam a própria rosca na chapa; os autobrocantes (DIN 7504) fazem também o furo.
- **CAD avançado A/CAD avançado C:** o furo de pré-furação é o dado mais importante (tabela por material). O CAD avançado A inclui no biblioteca de normas o furo de chapa recomendado.
- **Fenkra hoje:** rosca "ST" não passa em `diametro_nominal` (só "M…", `biblioteca.rs:81`).
- **Melhor prática:** designação genérica + `diametro_nominal` como campo, não derivado da string. Associar o furo recomendado como propriedade da família.

### Parafuso de pressão (sem cabeça) — ISO 4026–4029 / DIN 913–916 (❌)
- **Dados:** [C]. Hexágono interno s por d (M6: 3; M8: 4; M10: 5), pontas plana, cone, cavidade e ponta de apoio.
- **O que faz:** trava um cubo no eixo.
- **Fenkra hoje:** não existe; `comprimentos_de` exige L≥2d, mas esses parafusos podem ter L<2d (ex.: M6×6 = 1d).
- **Melhor prática:** o intervalo de comprimentos é dado da **família**, não regra global (ver RF-PRD-04).

### Parafusos especiais: carroceria (DIN 603), olhal (DIN 444), em U (DIN 3570), sextavado de madeira (DIN 571), estrutural (ASTM A325/A490) (❌)
- **Dados:** [C], uma ficha própria cada. A325/A490 usam cabeça "heavy hex" e arruela estrutural (ASTM F436), tabelas ASME B18.2.6.
- **CAD avançado A/CAD avançado C:** estão no biblioteca de normas/CC como famílias comuns. O CAD avançado B costuma usar a biblioteca de estrutura própria.
- **Fenkra hoje:** não existe.
- **Melhor prática:** priorizar P2/P3; entram quando houver o gerador genérico.

### 3.2 Porcas

### Porca sextavada normal e baixa (ISO 4032/4035, DIN 934/439 — ✅ parcial)
- **Dados:** campos s, m. Fenkra tem ISO 4032 e DIN 934 como arquivos separados, com M12 s=19 na DIN 934 contra 18 na ISO (`biblioteca.rs:388`). A baixa (ISO 4035 / DIN 439) ✘.
- **Fenkra hoje:** `porca` (`fixadores.rs:96-104`) = prisma + furo Ø d, sem chanfro nem rosca.
- **Melhor prática:** duas normas com tabelas quase iguais devem compartilhar um **gerador** e diferir só por override de tamanhos. Teste: massa × catálogo.

### Porca autotravante com inserto de nylon (DIN 985 / ISO 10511 / ISO 7040)
- **Dados** [V, fasten.it DIN 985, cabeçalhos da página ligeiramente confusos]:

  | d | P | e min | s | m min | h (max/min) |
  |---|---|---|---|---|---|
  | M5 | 0,8 | 8,87 | 8 | 3,2 | 5 / 4,7 |
  | M6 | 1 | 11,05 | 10 | 3,4 | 6 / 5,7 |
  | M8 | 1 / 1,25 | 14,38 | 13 | 4,7 | 8 / 7,64 |
  | M10 | 1 / 1,25 / 1,75 | 18,90 | 17 | 6 | 10 / 9,64 |

  Note que a altura total h inclui o anel de nylon: a tabela tem duas alturas (de rosca m e total h). O campo `m` único do tipo `Porca` não basta. A ISO 7040 (tipo alto) tem alturas diferentes, [C].
- **CAD avançado B/CAD avançado A/Inv.:** em todos, o anel é um cilindro simplificado ou nem aparece.
- **Fenkra hoje:** não existe; `Porca` exige só `s`, `m`.
- **Melhor prática:** campos `h` e `m` separados, nylon como segundo corpo (cor/material próprio) para BOM. Teste: altura total = h.

### Porca autotravante toda em metal (DIN 980 / ISO 7042)
- **Dados** [V, fasten.it DIN 980, retirada, ed. 05/1987]:

  | d | P grossa / fina | s | e min | h (min / max) |
  |---|---|---|---|---|
  | M6 | 1 / – | 10 | 11,05 | 5,7 / 6 |
  | M8 | 1,25 / 1 | 13 | 14,38 | 7,5 / 8 |
  | M10 | 1,5 / 1–1,25 | 17 | 18,9 | 9 / 10 |
  | M12 | 1,75 / 1,25–1,5 | 19 | 21,1 | 11 / 12 |

  Note que s=17 em M10 e 19 em M12 seguem a série DIN, não a ISO. A norma está retirada; a ISO 7042 é a vigente [C].
- **Fenkra hoje:** não existe.
- **Melhor prática:** a família cita a norma vigente e a retirada como `norma_equivalente`.

### Porca flangeada (DIN 6923 / ISO 4161)
- **Dados** [V, fasten.it DIN 6923 (retirada, 06/1983)]: M6 m 5,7–6, dc 14,2, s 10; M8 m 7,6–8, dc 17,9, s 13; M10 m 9,6–10, dc 21,8, s 15; M12 m 11,6–12, dc 26, s 18.
- **CAD avançado A/CAD avançado C:** o flange é um disco sob o sextavado.
- **Fenkra hoje:** não existe. Com porca flangeada a arruela sob a porca é supérflua (`h2` em `preencher_furos`).
- **Melhor prática:** flag `inclui_arruela`, como no parafuso flangeado.

### Porca castelo (DIN 935 / ISO 7035) e cupilha
- **Dados:** campos s, m, largura da fenda n, diâmetro do furo da cupilha. Os valores numéricos não foram abertos (fasten.it `din_935` deu 404); buscar DIN 935-1 (Würth/Bossard).
- **O que faz:** porca com ranhuras que recebe cupilha (DIN 94) para travamento positivo.
- **CAD avançado C/CAD avançado A:** a cupilha é peça separada no stack inferior, não é automática.
- **Fenkra hoje:** não existe.
- **Melhor prática:** o parafuso para castelo tem **furo transversal** (campo `d_furo_cupilha`). A pilha automática não deve oferecer castelo se o parafuso não tiver o furo.

### Porca cega (DIN 1587), borboleta (DIN 315), quadrada (DIN 557), de acoplamento (DIN 6334) (❌)
- **Dados:** [C], tabelas a abrir em fasten.it.
- **Melhor prática:** P3; cada uma é uma variante de forma de gerador (perfil de revolução, asas, prisma quadrado).

### Rebite-porca (blind rivet nut / "rivnut")
- **Dados** [V, só fabricantes, sem a norma aberta]. M6, formato redondo, cabeça plana: corpo Ø9,0; furo na chapa Ø9,1; flange Ø12–14 conforme modelo; faixa de aperto 0,5–3,5 (Gesipa) ou 0,7–4,2 (RivetKing); cabeça avulsa de 10 a 12,7. **Os valores variam entre fabricantes**, então a família pede designação do fabricante, não uma tabela só.
  - **Rótulo:** `docs/contextos/fixadores.md` chama de "ISO 15977", mas essa série é de rebites cegos [V: a ISO 15983 é rebite cego]. A norma de rebite-porca é outra (provável ISO 16583) [C].
- **CAD avançado B/CAD avançado A/Inv.:** costuma ser peça de fabricante (McMaster/TraceParts). O CAD avançado C tem categoria de porcas de chapa.
- **Fenkra hoje:** não existe.
- **Melhor prática:** parâmetro-chave = **faixa de aperto** (espessura de chapa), o que o preenchedor já tem como `pega`. Escolher a variante cujo intervalo contém a pega.

### Porca soldável (DIN 928 quadrada, DIN 929 sextavada)
- **Dados:** [C], tabelas a abrir. Projeções de solda no lado da chapa.
- **O que faz:** porca soldada à chapa para formar rosca fixa.
- **Melhor prática:** gerar com 3–4 projeções cilíndricas; publicar plano de apoio e eixo.

### 3.3 Arruelas

### Arruela lisa e larga (ISO 7089/7090/7093, DIN 125/9021 — ✅ parcial)
- **Dados:** a ISO 7089 está em `iso7089.json` (d1, d2, h). DIN 9021/ISO 7093 (larga): [C] M6 6,4 / 18 / 1,6; M8 8,4 / 24 / 2; M10 10,5 / 30 / 2,5; confirmar.
- **Fenkra hoje:** `arruela` (`fixadores.rs:107-115`) é um anel simples. Faltam o chanfro de ISO 7090 e a variante série fina.
- **Melhor prática:** `chanfro` opcional como campo. A chave do preenchedor deve incluir a **série** (normal, larga, fina), hoje fixa por `familia_arruela`.

### Arruela de pressão (DIN 127, DIN 7980, ASME B18.21.1)
- **Dados** [V, fasten.it DIN 127]: M6 d1 6,1 / d2 11,8 / b 2,5 / s 1,6; M8 8,1 / 14,8 / 3 / 2; M10 10,2 / 18,1 / 3,5 / 2,2; M12 12,2 / 21,1 / 4 / 2,5; altura livre h (tipo A) 3,6–4,2 / 4,6–5,4 / 5–5,9 / 5,8–6,6. A página mistura colunas k/r, então a leitura exige conferência.
  - Inch [V parcial]: a ASME B18.21.1-2009 consolidou as normas de arruela de pressão e lisa; um catálogo dá #2: ID 0,094", OD 0,172", espessura 0,020", e 3/8: ID 0,385", OD 0,680".
- **O que faz:** anel aberto helicoidal; o efeito "trava" é discutido, mas é padrão de BOM.
- **CAD avançado B/CAD avançado A/Inv.:** o CAD avançado A/CAD avançado C modelam como anel plano com fenda; algumas bibliotecas deixam hélice real.
- **Fenkra hoje:** não existe. O preenchedor tem só uma arruela por lado.
- **Melhor prática:** o empilhamento precisa de lista ordenada de peças (`[lisa, pressão]`), editável por grupo.

### Arruela dentada (DIN 6797 externa / J interna, DIN 6798 A/J/V)
- **Dados** [V, fasten.it DIN 6798, retirada; o rótulo da página diz forma V, mas a tabela traz formas A e J]: M6 d1 6,4 / d2 11 / s 0,7 / dentes A=12, J=9; M8 8,4 / 15 / 0,8 / 14, 10; M10 10,5 / 18 / 0,9 / 16, 12; M12 13 / 20,5 / 1 / 16, 12. A norma está retirada; na DIN 6797 [C] o dado é semelhante.
- **Melhor prática:** o número de dentes é valor, mas o perfil de dente é desenho simplificado (padrão polar de cunhas). Teste: nº de dentes = tabela.

### Arruela Belleville / cônica (DIN 6796 de fixação; DIN 2093 de mola)
- **Dados** [V, fasten.it DIN 6796]: M6 d 6,4 / d1 14 / s 1,5 / h 1,7–2 / carga a plano 8590 N; M8 8,4 / 18 / 2 / 2,24–2,6 / 14900; M10 10,5 / 23 / 2,5 / 2,8–3,2 / 22100; M12 13 / 29 / 3 / 3,43–3,95 / 34100.
- **O que faz:** mola em prato: DIN 6796 é para apertos de alta carga; DIN 2093 é mola mecânica com séries A/B/C e empilhamento em série e paralelo.
- **CAD avançado A/CAD avançado C:** a Belleville é cone por revolução. **Atenção:** a pilha tem lado (convexo para cima ou para baixo), que importa para o contato.
- **Fenkra hoje:** não existe. O contato do preenchedor supõe arruela plana de duas faces paralelas.
- **Melhor prática:** gerador `cone` com `d`, `d1`, `s`, `h`. Publicar o plano de apoio do **anel externo** e do **interno** como duas referências distintas. A carga a plano é propriedade (para o futuro cálculo de pré-carga).

### Arruela Nord-Lock (cunha, DIN 25201)
- **Dados** [V parcial, revendas]: NL6 ID 6,5 / OD 10,8 / esp. 1,8 (um revendedor traz 0,10" = 2,5 mm, valor incoerente); NL8 8,7 / 13,5 / 2,5 (inox: 2,0); NL10 10,7 / 16,6 / 2,5. São sempre **pares** (duas peças com cames). Variantes "sp" têm OD maior.
- **O que faz:** par de arruelas com cames que trava por tração, não por atrito.
- **Fenkra hoje:** não existe. O contato do preenchedor não tem a noção de "par".
- **Melhor prática:** uma peça com **dois corpos** ou duas peças com a mesma referência de família; a espessura do par entra no cálculo de pega.

### Arruela ondulada (DIN 137), estrutural (ASTM F436), aeronáutica (NAS1149)
- **Dados:** [C]. F436 usa arruela mais espessa e larga para parafusos A325/A490.
- **Melhor prática:** P3; reaproveita o gerador de anel.

### 3.4 Pinos

### Pino cilíndrico (ISO 2338 / 8734, DIN 7 / 6325)
- **Dados** [V, fasten.it]. DIN 7 (retirado): d 4/6/8/10 → chanfro c máx 0,6/0,9/1,2/1,5. ISO 8734 (temperado): c 0,63/1,2/1,6/2 para os mesmos d. Comprimentos de 3 a 100 mm; a grade por diâmetro estava ilegível na extração. Tolerâncias (m6 e h8) pesam: o pino é **ajustado**.
- **CAD avançado B/CAD avançado A/Inv.:** em todos há ajuste (m6) como propriedade, e o assistente de furos do CAD avançado A tem furo de pino.
- **Fenkra hoje:** não existe; a designação "Ø6×20" não passa em `diametro_nominal` (`:81`).
- **Melhor prática:** a designação passa a ser texto livre + campo numérico `d`. O ajuste (h8/m6) vira campo textual. Teste: o furo do pino na montagem usa a tolerância do furo.

### Pino cônico (ISO 2339 / DIN 1, conicidade 1:50)
- **Dados:** DIN 1 (cancelado, substituído pela EN 22339) [V]; a tabela de comprimentos estava incompleta na extração (d=8 começa em 28 mm, d=10 em 36 mm, valores incertos). Geometria: diâmetro **menor** nominal, conicidade 1:50 [C].
- **Fenkra hoje:** não existe.
- **Melhor prática:** o furo cônico exige **rimador cônico** (furo cônico no Hole). Gerar o corpo como cone com `d_pequeno` e `L`. Isso exige que o recurso de furo suporte cone.

### Pino elástico (ISO 8752 ranhurado, ISO 8750/8748 espiral, DIN 1481)
- **Dados** [V, fasten.it DIN 1481, substituído pela ISO 8752]: d 4/6/8/10 → chanfro A 0,65/1,2/2/2, parede S 0,8/1,2/1,5/2, cisalhamento duplo 11,24/26,04/42,76/70,16 kN.
- **O que faz:** tubo fendido que se comprime no furo.
- **CAD avançado A/CAD avançado C:** modelado como tubo com fenda longitudinal ou cilindro maciço simplificado.
- **Melhor prática:** gerar com fenda real apenas em nível de detalhe alto; senão, cilindro maciço de mesma massa efetiva.

### Cupilha (ISO 1234 / DIN 94)
- **Dados** [V, fasten.it DIN 94, cancelada]:

  | d nom | d min–max | a máx | b ≈ | c min–max |
  |---|---|---|---|---|
  | 3,2 | 2,7–2,9 | 3,2 | 6,4 | 5,1–5,8 |
  | 4 | 3,5–3,7 | 4 | 8 | 6,5–7,4 |
  | 5 | 4,4–4,6 | 4 | 10 | 8–9,2 |
  | 6,3 | 5,7–5,9 | 4 | 12,6 | 10,3–11,8 |

  O d nominal é o **furo**, não o fio.
- **Fenkra hoje:** não existe.
- **Melhor prática:** geometria de fio dobrado (2 meias-seções + olho), simplificada para duas hastes e um anel. Ligar à porca castelo.

### Pino guia / posicionador / com cabeça (ISO 8735 com rosca interna, ISO 2341 clevis)
- **Dados:** [C]; ISO 8735 (pino com rosca interna para extração), ISO 2341 (pino com cabeça), DIN 1444. Abrir em fasten.it.
- **Fenkra hoje:** não existe.
- **Melhor prática:** o pino-guia vem com **furo para o outro lado**; criar par de furos de referência (gabarito) na montagem. Publicar `eixo` e `plano_de_cabeça`.

### 3.5 Anéis elásticos e clipes

### Anel elástico externo (DIN 471) e interno (DIN 472)
- **Dados** [V, fasten.it].
  - DIN 471, tipo normal: eixo 10 → s 1 / d3 9,3 / d2 (fundo do canal) 9,6 / m (largura do canal) 1,1 / n 0,6; eixo 20 → 1,2 / 18,5 / 19 / 1,3 / 1,5; eixo 30 → 1,5 / 27,9 / 28,6 / 1,6 / 2,1; eixo 40 → 1,75 / 36,5 / 37,5 / 1,85 / 3,8.
  - DIN 472, tipo normal: furo 10 → s 1 / d3 10,8 / d2 10,4 / m 1,1 / n 0,6; furo 20 → 1 / 21,5 / 21 / 1,1 / 1,5; furo 30 → 1,2 / 32,1 / 31,4 / 1,3 / 2,1; furo 40 → 1,75 / 43,5 / 42,5 / 1,85 / 3,8.
  - Existe também o tipo pesado (heavy).
- **O que faz:** anel aberto que trava peças axialmente num canal de eixo ou furo.
- **CAD avançado B/CAD avançado A/Inv.:** o CAD avançado A biblioteca de normas tem ranhura de anel como recurso de torneamento; o CAD avançado C Shaft Generator cria ranhura DIN 471 com o anel. O anel é um perfil aberto extrudado com olhais.
- **Fenkra hoje:** não existe. O backlog B52 já cita "eixo com ranhuras de anel" (`biblioteca-de-normas.md`, BIB-04).
- **Melhor prática:**
  - Fluxo: eixo → ranhura (usa `d2`, `m`) → anel (usa `d3`, `s`).
  - A ranhura é **recurso do eixo** com parâmetros derivados do diâmetro (tabela por faixa), e o anel é peça separada que se apoia nela.
  - Regra: a tabela de ranhura é a mesma que o anel (uma só fonte de dados).
  - Teste: `d2` do canal < d do eixo; `d3` do anel livre > `d2`.
  - Conferência cruzada: ranhura × anel pelos mesmos `d1`.

### Clipe E (DIN 6799) e outros (R-clip, mola de trava, DIN 11024)
- **Dados:** [C]. DIN 6799 define o diâmetro do canal por tamanho de anel (1,2 a 30). Abrir em fasten.it `din_6799`.
- **Melhor prática:** P3, mesmo mecanismo de ranhura que o DIN 471.

### 3.6 Chavetas

### Chaveta paralela (DIN 6885 forma A, ISO 773 — ✅ parcial)
- **Dados:** `biblioteca/chavetas/din6885a.json`: 13 faixas de Ø 6–85, 28 comprimentos, nível C. O código marca **t2 sem segunda fonte** (nota em BIB-04). A página fasten.it DIN 6885 abriu sem tabela; usar tabelas de eixos (tspi.at, roymech sem URL fixo).
- **Fenkra hoje:** `Chaveta` (`biblioteca.rs:247-258`) com `b,h,t1,t2`; `chaveta_para_eixo(d)` busca faixa onde `d_acima < d ≤ d_ate` (`:323-325`), com testes de limite. Usada em eixos e engrenagens (`eixos.rs:119,308`; `engrenagens.rs:352,396`).
  - Falta: formas B, C (cantos arredondados), DIN 6885-2/-3, comprimentos por faixa (a série única de 28 comprimentos permite 4×4 em Ø6, o que é inválido) e tolerâncias do rasgo (P9/N9/JS9).
- **CAD avançado B/CAD avançado A/Inv.:** o CAD avançado C (Design Accelerator) cria chaveta + rasgo de eixo e de cubo juntos e faz o cálculo de pressão. O CAD avançado A usa o biblioteca de normas para o rasgo.
- **Melhor prática:** o comprimento mínimo/máximo deve depender da seção (b×h). Teste: nenhuma chaveta do catálogo sai com L < 2,5·b nem acima do limite da série.

### Chaveta Woodruff (DIN 6888)
- **Dados** [V, fasten.it, norma cancelada, ed. 08/1956]: b 5, h 6,5, d2 16, l≈15,72, Ø de eixo forma A 12–17 e forma B 22–30; b 5, h 7,5, d2 19; b 5, h 9, d2 22; b 6, h 7,5 / 9 / 10 / 11, d2 19 / 22 / 25 / 28; b 8, h 9 / 11 / 13, d2 22 / 28 / 32. **Falta** a profundidade t1 do rasgo no eixo, que a página não traz; a ISO 3912 / BS 4235-2 é a referência. Usar um catálogo (McMaster, Fabory, opac.net) com t1/t2 e abrir uma segunda fonte.
- **O que faz:** chaveta em segmento circular, assentada num rasgo fresado em disco; permite oscilação angular.
- **Fenkra hoje:** não existe; o eixo só gera rasgo reto de chaveta A.
- **Melhor prática:** o rasgo no eixo é **arco** (fresa de disco `d2`), não retângulo. Reutilizar o recurso de rasgo com perfil circular.

### Chaveta cônica (DIN 6886 com cabeça, DIN 6887, conicidade 1:100)
- **Dados:** [C]; tabela a abrir.
- **Melhor prática:** P3; rasgo com fundo inclinado no cubo; o conjunto é montado por martelo, então sem restrição de grau de liberdade.

### 3.7 Insertos roscados

### Inserto de arame helicoidal (Helicoil, DIN 8140)
- **Dados** [V parcial, McMaster e Böllhoff]:
  - Broca de furo: M6×1 → 6,3 mm; M8×1,25 → 8,4 mm; M10 → 13/32" (10,3 mm, indicado pela McMaster).
  - Comprimentos usuais 1D, 1,5D, 2D, 2,5D, 3D (M6: 6, 9, 12, 15 mm).
  - Atenção: a **Helicoil Plus** (ferramenta combinada) usa outro furo (7,3 / 9,7 / 12,0 mm). Não misturar.
  - A norma DIN 8140-2 define a classe 5H para a rosca final.
- **O que faz:** mola de aço inox que restaura ou reforça rosca em material mole.
- **CAD avançado B/CAD avançado A/Inv.:** em geral o inserto é um recurso do furo (CAD avançado A: tipo "Helicoil" em algumas bibliotecas de fabricante). CAD avançado F IFX trata inserto como tipo de fixador (prensado e roscado) [V].
- **Fenkra hoje:** `HoleRow.thread` guarda a rosca como texto; sem inserto. O preenchedor mede a profundidade útil do furo roscado.
- **Melhor prática:** o furo ganha atributo `inserto: {tipo, comprimento}`, que muda a **broca** e o diâmetro nominal da rosca vista pelo parafuso. Teste: M8 com Helicoil 1,5D → furo Ø8,4 e rosca interna M8.

### Insertos sólidos (Ensat, Kerb-Konus, Time-Sert, rosca de moldagem)
- **Dados:** [C]. Cada fabricante tem tabela própria (furo, Ø externo, comprimento). Camada de fabricante, não norma.
- **Melhor prática:** camada "fabricante" da biblioteca (ADR 0008 já prevê); não misturar com ISO/DIN.

### 3.8 Rebites

### Rebite maciço (DIN 660 cabeça redonda, 661 escareada, 662 cogumelo, 124 >10 mm; MS20470 aeronáutico)
- **Dados** [V, fasten.it DIN 660, ed. 2012]:

  | d1 | dk | d min | k | e máx |
  |---|---|---|---|---|
  | 3 | 5,2 | 2,87 | 1,8 | 1,5 |
  | 4 | 7 | 3,87 | 2,4 | 2 |
  | 5 | 8,8 | 4,82 | 3 | 2,5 |
  | 6 | 10,5 | 5,82 | 3,6 | 3 |
  | 8 | 14 | 7,76 | 4,8 | 4 |

  DIN 661 (escareada): só massas por comprimento foram extraídas [V]; dimensões da cabeça [C].
- **O que faz:** une chapas permanentemente; a cabeça "de fechamento" é formada no assentamento.
- **CAD avançado B/CAD avançado A/Inv.:** o catálogo traz o rebite **antes** da fixação. Nenhum dos três insere rebite por ponto de forma nativa e simples (como já registrado em BIB-10, `catalogo.md`).
- **Fenkra hoje:** não existe (B58 prevê).
- **Melhor prática:** o rebite é um fixador com **dois estados**: antes e depois da cravação. A cabeça de fechamento cresce a partir de `e` (ou `1,5·d`). Teste: comprimento = pega + 1,5·d.

### Rebite cego (ISO 15977–15984)
- **Dados** [V, fasten.it ISO 15983, aberto, cabeça saliente, A2/A2]:

  | d nom | dk máx / mín | k máx | d1 máx | p | b máx |
  |---|---|---|---|---|---|
  | 3,2 | 6,7 / 5,8 | 1,3 | 2 | 25 | l máx + 4 |
  | 4 | 8,4 / 6,9 | 1,7 | 2,45 | 25 | l máx + 4,5 |
  | 4,8 | 10,1 / 8,3 | 2 | 2,95 | 27 | l máx + 5 |

  A coluna `d1` pode ser o mandril, mas a página não rotula. Diâmetros: 3, 3,2, 4, 4,8 e 5; não há 6,4 nesta norma. Fontes: Würth (ISO 15983, já citada no catálogo).
- **CAD avançado C/CAD avançado A:** o rebite cego costuma ser um corpo tubular + mandril, o mandril some após a cravação.
- **Fenkra hoje:** não existe.
- **Melhor prática:** gerar no estado **cravado**: tubo com cabeça + cabeça cega + resto do mandril. Campos: faixa de aperto, furo recomendado. Escolher L pela pega (como o preenchedor faz).

### Rebites estruturais (Huck, Cherry, Hi-Lok, drive DIN 7337 etc.)
- **Dados:** [C]. A norma DIN 7337 é para rebite cego [C]; o rótulo "Drive rivet" no contexto parece trocado. Conferir.
- **Melhor prática:** P3; camada aeronáutica (MS/NAS/AN no ASSIST, nível A).

### 3.9 Clinch e fixadores de chapa

### Porca/prisioneiro/espaçador auto-cravado (PEM CLS, S, FH, SO — NASM45938)
- **Dados** [V, PEM e revenda, família CLS]:

  | Peça | Furo na chapa | E (cabeça) | T (altura) | C máx | Distância ao bordo |
  |---|---|---|---|---|---|
  | CLS-M3-1 | 4,22 | 6,35 | 1,5 | 4,2 | 4,8 |
  | CLS-M4-2 | 5,41 | 7,87 | 2 | 5,38 | 6,9 |
  | CLS-M6-1 | 8,75 | 11,18 | 4,08 | 8,73 | 8,6 |
  | CLS-M8-2 | 10,5 | 12,7 | 5,47 | 10,47 | 9,7 |
  | CLS-M10-2 | 14 | 17,35 | 7,48 | 13,97 | – |

  - A espessura mínima de chapa aparece só na revenda (M10-2: 3,18 mm).
  - Dureza máxima da chapa: HRB 70 / HB 125 para CLS; S e SS toleram HRB 80 / HB 150.
  - A designação segue o padrão `CLS-<rosca>-<código de espessura>`.
  - Fonte: pemnet.com. Segunda fonte: revendas autorizadas (Swaco) e o catálogo PEM.
- **O que faz:** é pressionada na chapa e se deforma em torno do furo, formando rosca fixa sem solda.
- **CAD avançado B/CAD avançado A/Inv.:** em CAD avançado A/CAD avançado C o clinch é peça de fabricante (TraceParts/PEM). O CAD avançado C mantém "porcas de chapa" como único fixador de chapa do Bolted Connection [V].
- **Fenkra hoje:** não existe. O módulo de chapa existe, mas sem fixadores de chapa.
- **Melhor prática:**
  - O fixador **cria o furo** na chapa (diâmetro, tolerância +0,08/–0) e valida distância ao bordo e dureza.
  - Regra: se a chapa for mais mole que o limite de dureza, avisar.
  - Regra: se o centro do furo estiver a menos que "distância ao bordo" do bordo, **erro visível** (princípio 1 do ADR 0011).
  - Teste: um furo CLS-M4-2 a 5 mm do bordo falha.

### 3.10 Padrões em polegada (UNC/UNF, ASME B18)

### Rosca e fixadores ASME B18 (parafusos B18.2.1/B18.3, porcas B18.2.2, arruelas B18.21.1/B18.22.1)
- **Dados** [V parcial, vendors]:
  - Rosca: `hardware.rs` já tem UNC/UNF (ex.: ¼-20, 5/16-18, 3/8-16 etc.).
  - B18.3 socket cap: ¼-20 → dk 0,375", k 0,25", hex 3/16"; 5/16-18 → 0,469" / 0,312"; 3/8-16 → 0,562" / 0,375" / 5/16". Valores de uma revenda, a conferir.
  - Porca hex ¼: AF 0,428–0,438", espessura 0,212–0,226", jam 0,150–0,163". Heavy hex ¼: AF 0,488–0,500", espessura 0,218–0,250".
  - A tabela completa B18.2.2 não apareceu em fonte única; amesweb.info e engineersedge.com têm tabelas completas por tamanho a abrir.
- **CAD avançado B/CAD avançado A/Inv.:** as três têm normas ANSI/inch como categorias separadas. O biblioteca de normas separa "ANSI Inch" e "ANSI Metric".
- **Fenkra hoje:** `diametro_nominal` só entende "M…" (`biblioteca.rs:81`); `fixador_documento` converte `d` para número. UNC "1/4-20" e "#10-24" não passam. O preenchedor só olha `hardware::METRIC`.
- **Melhor prática:**
  - Campos `d_mm` e `unidade_origem` (pol), com medidas guardadas em mm e o **valor original em pol** para exibir.
  - Designação ("1/4-20 UNC") como texto.
  - Teste: converter 0,375" para 9,525 mm sem erro de arredondamento no rótulo.

### 3.11 Funções inteligentes

### Família parametrizada genérica (modelo de dados)
- **O que faz:** é o equivalente à família do Content Center / ao catálogo CAD avançado B / à configuração do biblioteca de normas.
- **CAD avançado B/CAD avançado A/Inv.:** ver 2.0. O que há de inteligente:
  - CAD avançado A: configurações com nomes gerados; PN/descrição/comentário e propriedades em lista.
  - CAD avançado C: coluna-chave + mapeamento de parâmetros + revisão da norma.
  - CAD avançado B: tabela de projeto com PartNumber e PartName.
- **Fenkra hoje:** família com `tipo` fechado, `campos` de `f64`, tamanhos "M…" e comprimentos globais (`biblioteca.rs:56-101`).
- **Melhor prática (esquema proposto):**
  - `id`, `norma`, `revisao`, `equivalentes[]` (DIN↔ISO), `gerador` (nome: `cabeca_sextavada`, `cabeca_calota`, `anel`, `pino`…).
  - `campos[]` com tipo (número/texto/lista) e unidade.
  - `chave[]` (designação, comprimento).
  - `comprimentos` por tamanho ou por faixa.
  - `propriedades` (material padrão, classe, acabamento, norma de rosca).
  - `publicacoes` do fixador (eixo, plano de apoio, ponto de ponta).
  - `camada` (pública/empresa/usuário) e `fontes[]` por tamanho.
  - Teste: carregar todos os JSON e gerar **todos** os tamanhos (já existe em `fixadores.rs:276`; manter).

### Preenchimento automático de furos (fixadores inteligentes)
- **CAD avançado B:** não tem de série [C]; usar Power Copy de furo+fixador com publicações.
- **CAD avançado A [V]:**
  - Seleciona um furo, uma série de furos, um componente ou a montagem toda ("Adicionar" / "Populate All").
  - Agrupa por conjuntos de furos iguais; cada grupo pode ter outro tipo, tamanho e comprimento.
  - Top stack = arruelas sob a cabeça; bottom stack = arruelas e porcas na ponta. Furo cego só recebe top stack.
  - Regra de comprimento padrão: próximo menor da série para furo cego e próximo maior para passante; se o furo for mais fundo que o maior fixador, usa o maior. Opção de atualizar o comprimento ao mudar o empilhamento. Há ajustes de engajamento mínimo e de roscas além da porca.
  - Furos que não vêm do assistente de furos recebem o fixador padrão do biblioteca de normas e pedem conferência.
- **CAD avançado C [V]:** Bolted Connection (ver abaixo).
- **CAD avançado H [V]:** ver 2.0 (extensão mínima / engajamento).
- **Fenkra hoje:** `preencher_furos`. Acertos: pilha por colinearidade com tolerância, pega por raios laterais e mediana, comprimento = pega + arruelas + porca + 2P, pula eixos que já têm hardware, relatório com avisos. Limites:
  - `HoleRow` sem tipo de furo (`lib.rs:5059`), então cabeça sempre assenta na face.
  - Só métrico M3–M20; `tamanho_da_rosca` ignora o passo (M8x1 e M8x1,25 viram "M8").
  - Só Allen/sextavado/ISO 4032/ISO 7089 (opções de `familia_*`); sem agrupar por série.
  - A regra "2d–10d" corta comprimentos.
  - Sem edição da pilha depois (B51).
  - `!destino.exists()` reaproveita arquivo antigo (`:374`).
  - Dedupe "um membro por componente" (`:181-187`) descarta o segundo furo do mesmo componente no mesmo eixo (por exemplo, rebaixo + passante modelados como dois cortes).
- **Melhor prática:** fluxo suporte → modo → parâmetros → avançado:
  1. escolher o **escopo** (furo / série / componente / montagem);
  2. mostrar a lista de grupos antes de inserir (como o CAD avançado A);
  3. cada grupo: família de parafuso, empilhamento superior e inferior (lista ordenada), regra de comprimento (próximo maior, engajamento, saliência);
  4. avançado: folga, arruela larga, travamento.
  - Casos de teste:
    - furo passante em 3 chapas;
    - furo roscado cego;
    - rebaixo (cabeça Allen assenta no fundo do rebaixo);
    - escareado (parafuso ISO 10642 rente);
    - série de 4 furos em padrão com comprimentos diferentes;
    - polegada;
    - furo sem tamanho (aviso).

### Conexão parafusada assistida (Bolted Connection, Hole Series)
- **O que faz:** cria furos nas peças e insere os fixadores num só passo.
- **CAD avançado C [V]:**
  - Tipo: passante ou cego.
  - Método de posição: linear (2 arestas), concêntrico (aresta circular), sobre ponto, por furo existente (com "seguir padrão").
  - O filtro de rosca restringe os fixadores da lista.
  - O comprimento é ajustado por alça 3D, que só admite comprimentos existentes, ou por lista.
  - Opção de criar subconjunto (permite solução automática).
  - Modelos de conexão salvos.
  - Excluir o parafuso apaga os fixadores associados.
- **CAD avançado A:** Hole Series e fixadores inteligentes: planos de início e fim, furos nas peças, fixadores [V no catálogo].
- **CAD avançado B:** não tem [C].
- **Fenkra hoje:** ❌ (B57/B58 previstos). O "furo de montagem" só existe na montagem (`catalogo.md` linha 985).
- **Melhor prática:** depende do B57 (furo entre peças com link). Começar pelo modo "por furo existente".

### Fixador por ponto / padrão (rebites por pontos, parafusos em padrão)
- **O que faz:** insere um fixador em cada ponto de uma lista ou padrão.
- **CAD avançado C [V]:** o modo "sobre ponto" aceita pontos de esboço da peça (não os criados na montagem); "seguir padrão" copia o padrão do furo. Se inserir em um padrão, substitui as conexões que já havia.
- **CAD avançado A:** o fixador acompanha o padrão de furos; permite configuração diferente para um membro [V no contexto].
- **Fenkra hoje:** não há; o preenchedor trata cada furo/eixo isoladamente.
- **Melhor prática:** fixador **derivado do padrão** (herda quantidade e posições, com exceções por instância). Teste: apagar 1 instância do padrão remove 1 fixador.

### Propriedades, PN, material e BOM dos fixadores
- **Hoje:** `doc.props.pn = ""`; descrição = "ISO 4762 M6x20"; material = "Aço 1020" (`fixadores.rs:128-132`).
- **CAD avançado A:** PN/descrição/comentário por configuração; material em lista [V].
- **CAD avançado C:** PN pela expressão `{PARTNUMBER}`; manufacturer, norma e revisão em iProperties [V].
- **CAD avançado B:** PartNumber e PartName na tabela de projeto [V].
- **Melhor prática:**
  - PN gerado: `<norma>-<designação>-<L>-<classe>-<acabamento>`.
  - Descrição legível ("Parafuso Allen ISO 4762 M6×20 – 12.9").
  - Material derivado da classe (8.8 / 10.9 / 12.9 / A2-70 / A4-80).
  - Massa calculada e conferida contra a tabela do catálogo (os pesos kg/1000 estão nas páginas fasten.it: DIN 985 M6 = 2,4 g; DIN 6796 M6 = 1,43 g; DIN 6798 M6 = 0,36 g; DIN 127 M6 = 0,83 g [V]).
  - Teste: massa do modelo dentro de 5% do peso de catálogo.

### Trocar / atualizar / diagnosticar uma pilha
- **CAD avançado A [V]:** "Editar fixadores inteligentes" troca tipo, comprimento, empilhamento. Aviso do CAD avançado A: não edite o fixador do biblioteca de normas por esboço/feature direto, pois a base não atualiza.
- **CAD avançado H [V]:** fixador vira feature; ao mudar o projeto e o comprimento deixar de servir, o PathFinder sinaliza e o Error Assistant atua.
- **CAD avançado C [V]:** Substituir todos / Mudar tamanho.
- **CAD avançado B:** o diagnóstico de atualização lista a referência quebrada [C]; o catálogo vincula por referência.
- **Fenkra hoje:** B51 prevê "trocar norma/comprimento de uma pilha". Hoje o fixador é só um componente + restrições, sem marca de que pertence a uma pilha.
- **Melhor prática:** gravar no grupo "Fixadores" uma **pilha** como objeto (furos de origem, família, empilhamento) para poder recalcular quando a espessura mudar. Quando a pega não cabe, o estado é "falhou" com sugestão (princípio 1).

---

## 4. Requisitos levantados

Prefixo **PRD** (prendedores). Prioridades P0 (bloqueia) a P3 (desejável).

### Funcionais

| ID | Requisito | Prio. | Critério de aceite |
|---|---|---|---|
| RF-PRD-01 | Família de fixador definida por dados: `gerador` nomeado, `campos` tipados (número/texto/lista) e `chave`, sem `enum` fechado de tipo | P0 | Adicionar uma família nova (ex.: ISO 7380) só com um JSON e um gerador existente; nenhum `match` em Rust muda |
| RF-PRD-02 | Designação não métrica: "1/4-20 UNC", "Ø6 h8", "ST4,2", "CLS-M4-2" | P0 | `validar` aceita as quatro; `diametro_nominal` vem de campo, não de `strip_prefix('M')` |
| RF-PRD-03 | Série de comprimentos por família e por tamanho (mín/máx), não a regra `2d–10d` | P0 | M6×100 Allen disponível; M6×6 prisioneiro/pressão disponível; teste de limites por família |
| RF-PRD-04 | Propriedades do fixador: PN gerado, descrição, material derivado da classe (8.8/10.9/12.9/A2-70), acabamento, norma+revisão | P0 | `doc.props.pn` e `material` preenchidos; BOM lista PN distinto por comprimento |
| RF-PRD-05 | Massa de cada família conferida com o peso de catálogo | P1 | Teste automático ±5% para as famílias com peso publicado (DIN 985, 6796, 6798, 127, 661…) |
| RF-PRD-06 | Cabeças: sextavada com chanfro, calota (ISO 7380), escareada (ISO 10642), panela com recesso, flangeada | P1 | Cada família gera sólido válido e fechado; bounding box = dk×(k+L) |
| RF-PRD-07 | Prisioneiro com dois comprimentos de rosca (`bm`, `b`) e dois planos de apoio | P1 | Preenchedor monta prisioneiro + porca em furo roscado cego; L calculado com `bm` |
| RF-PRD-08 | Porcas: baixa, nylon (h≠m), metálica, flangeada, castelo, cega | P1 | Altura total = h da tabela; flangeada não recebe arruela automática |
| RF-PRD-09 | Arruelas: larga, pressão, dentada, Belleville, Nord-Lock (par) | P1 | Pilha aceita lista ordenada; par Nord-Lock conta duas peças na pega |
| RF-PRD-10 | Pinos: cilíndrico, cônico, elástico, cupilha, guia | P1 | Pino m6 num furo H7 gera ajuste; cônico exige furo cônico |
| RF-PRD-11 | Ranhura de anel (DIN 471/472) como recurso do eixo/furo + anel como peça | P1 | `d2`/`m` do canal e `d3`/`s` do anel saem da mesma linha de tabela |
| RF-PRD-12 | Chavetas: formas A/B/C, Woodruff (rasgo em arco), cônica; comprimento por seção | P1 | Nenhum comprimento fora do limite da seção; rasgo Woodruff com `d2` |
| RF-PRD-13 | Insertos roscados: atributo do furo (tipo, comprimento) que altera broca e rosca | P2 | M8 + Helicoil 1,5D → Ø8,4 e rosca M8 |
| RF-PRD-14 | Rebites sólidos e cegos no estado cravado, escolha de L pela pega e pela faixa de aperto | P2 | Rebite 4 mm numa pega de 3 mm escolhe o L da série; cabeça cravada = 1,5·d |
| RF-PRD-15 | Fixadores de chapa (PEM): cria o furo na chapa, valida bordo, espessura e dureza | P2 | CLS-M4-2 a 5 mm do bordo falha com mensagem; furo Ø5,41 +0,08/–0 |
| RF-PRD-16 | Polegada (UNC/UNF, ASME B18.2.1/B18.2.2/B18.3/B18.21.1) com valor original em pol | P2 | ¼-20 UNC Allen gera dk 9,525 mm; rótulo mostra 0,375" |
| RF-PRD-17 | `HoleRow` com tipo de furo (reto/rebaixo/escareado), diâmetros e ângulo; o preenchedor assenta a cabeça no fundo do rebaixo ou no cone | P0 | Allen no rebaixo: topo da cabeça rente ou abaixo da face; flat head escareado: rente |
| RF-PRD-18 | Preenchedor com grupos, escopo (furo/série/componente/montagem), empilhamento superior/inferior editável e regra de comprimento escolhida | P1 | Trocar a porca de um grupo recalcula só aquele grupo; regra "próximo maior/menor" selecionável |
| RF-PRD-19 | Pilha como objeto recalculável (referência aos furos de origem); falha visível se a pega não cabe | P1 | Engrossar uma chapa 3 mm marca a pilha "falhou" com sugestão de L; não escolhe outro furo em silêncio |
| RF-PRD-20 | Fixador em padrão derivado do padrão de furos, com exceção por instância | P2 | Padrão de 6 furos → 6 fixadores; remover 1 instância remove 1 fixador |
| RF-PRD-21 | Publicações do fixador (eixo, plano de apoio, ponto de ponta) usadas pelas restrições, não geometria solta | P1 | Substituir M6×20 por M6×25 mantém as restrições (ADR 0010) |
| RF-PRD-22 | Equivalência de normas (DIN↔ISO) na mesma família com sobreposições por tamanho | P2 | `iso4032` e `din934` partilham gerador; M12 s=18 vs 19 vem de override |
| RF-PRD-23 | Camadas pública/empresa/usuário (ADR 0008) com sobreposição por `id` | P2 | Família da empresa com mesmo `id` substitui a pública |
| RF-PRD-24 | Peças geradas guardam versão da biblioteca e se renovam quando os dados mudam | P1 | Alterar `dk` do M6 e abrir o projeto antigo avisa "fixador desatualizado" |

### Não funcionais

| ID | Requisito | Prio. | Critério de aceite |
|---|---|---|---|
| RNF-PRD-01 | Cada tamanho com ≥ 2 fontes independentes ou nota de fonte única | P0 | Teste existente (`biblioteca.rs:394-398`) estendido a todas as famílias novas |
| RNF-PRD-02 | Medidas são fatos; nada de cópia de texto, figura ou tabela formatada | P0 | Revisão de licença em cada PR de dados; `licenca` obrigatória |
| RNF-PRD-03 | Revisão da norma guardada (a ISO 10642 mudou `t min` entre 2004, 2013 e 2020) | P1 | Campo `revisao` obrigatório; teste falha se faltar |
| RNF-PRD-04 | Gerar todos os tamanhos de todas as famílias em CI (recompute, malha fechada, 1 corpo) | P0 | Teste equivalente a `fixadores.rs:276` cobre 100% das famílias |
| RNF-PRD-05 | Tempo de preenchimento: 200 furos em < 5 s | P2 | Benchmark no corpus B53 |
| RNF-PRD-06 | Arquivos de fixador reutilizados sem proliferar (nome canônico, uma peça por tamanho×L) | P1 | 100 parafusos iguais criam 1 arquivo |
| RNF-PRD-07 | Dado duplicado tem uma só origem (hoje `P` em `hardware::METRIC` e em cada JSON) | P1 | Teste de igualdade `METRIC.pitch == família.P` |
| RNF-PRD-08 | Dicas e relatório do preenchedor em português, com próximo passo | P2 | Cada aviso diz o que fazer |

---

## 5. Itens de backlog propostos

Tamanho: P (≤ 1 dia), M (2–5 dias), G (> 1 semana).

| # | Título | O que muda | Prio. | Tam. | Dependências |
|---|---|---|---|---|---|
| N1 | Modelo de família genérico | Substitui `TipoFixador` por `gerador` nomeado + campos tipados + chave + revisão; migra as 6 famílias | P0 | G | B29 (feito) |
| N2 | Designações e unidades gerais | Aceita UNC/UNF, Ø de pino, ST, fabricante; guarda mm + valor original | P0 | M | N1 |
| N3 | Comprimentos por família | Substitui `2d–10d`; faixa por tamanho; teste de limites | P0 | P | N1 |
| N4 | Propriedades do fixador | PN gerado, descrição, material por classe, acabamento, massa conferida com catálogo | P0 | M | N1 |
| N5 | `HoleRow` completo | Tipo de furo, diâmetros e ângulo; preenchedor assenta cabeça no rebaixo/escareado | P0 | M | B48 (escareado), B57 |
| N6 | Cabeças e porcas (família 1) | ISO 4014, 7380, 10642, 7045/7046, 4162, 4035, 985, 980, 6923, 935, 1587 | P1 | G | N1, N3 |
| N7 | Arruelas e travamentos | ISO 7093/DIN 9021, 127, 6798, 6796/2093, Nord-Lock, pilha ordenada | P1 | M | N1, N8 |
| N8 | Pilha editável e recalculável | Objeto de pilha; trocar norma/L; falha visível; grupos e escopo (amplia B51) | P1 | G | N5, B54/B55 |
| N9 | Pinos e anéis | ISO 2338/8734/2339/8752/1234, ranhura DIN 471/472 no eixo e no furo | P1 | G | N1, B52 |
| N10 | Chavetas completas | Formas B/C, Woodruff (rasgo em arco), cônica; comprimento por seção; 2ª fonte de t2 | P1 | M | B52 |
| N11 | Prisioneiros e parafusos de ombro | `bm`/`b`, dois planos de apoio; ombro e rosca menor | P1 | M | N1, N8 |
| N12 | Insertos e rebite-porca | Atributo de furo (Helicoil, Ensat); rebite-porca com faixa de aperto | P2 | M | N5 |
| N13 | Rebites sólidos e cegos | Estado cravado; escolha por pega e faixa; fixação por pontos | P2 | G | B58 |
| N14 | Fixadores de chapa PEM | Cria furo, valida bordo, dureza e espessura; CLS/S/FH/SO | P2 | M | módulo de chapa, N1 |
| N15 | Polegada ASME | Famílias B18.2.1, B18.2.2, B18.3, B18.21.1; preenchedor reconhece UNC/UNF | P2 | G | N2, N5 |
| N16 | Versão da biblioteca nas peças geradas | Marca de versão nos `.fkp`; aviso de fixador desatualizado; camada empresa/usuário | P1 | M | N1 |

Em tempo: corrigir os rótulos de `docs/contextos/fixadores.md` ("Rivet Nut = ISO 15977", "Drive Rivet = DIN 7337") ao abrir as normas correspondentes. É tarefa de dados (P).

---

## 6. Fontes

**Código e documentos do Fenkra (lidos):**
- `crates/fenkra-solidos/src/biblioteca.rs`
- `crates/fenkra-solidos/src/fixadores.rs`
- `crates/fenkra-montagem/src/fixadores.rs`
- `crates/fenkra-solidos/src/hardware.rs`
- `crates/fenkra-solidos/src/lib.rs` (`HoleRow`, `hole_table`)
- `biblioteca/fixadores/iso4762.json`
- `docs/biblioteca/fontes-de-dados.md`
- `docs/arquitetura/adr/0008-biblioteca-publica.md`
- `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`
- `docs/benchmark/biblioteca-de-normas.md`
- `docs/benchmark/catalogo.md`
- `docs/contextos/fixadores.md`
- `docs/contextos/smart-fastener.md`

**Medidas (fasten.it, abertas):**
- https://fasten.it/norms/norm/din_en_iso_4762
- https://fasten.it/norms/norm/din_en_iso_10642
- https://fasten.it/norms/norm/din_en_iso_4014
- https://fasten.it/norms/norm/din_en_iso_7045
- https://fasten.it/norms/norm/din_985
- https://fasten.it/norms/norm/din_980
- https://fasten.it/norms/norm/din_6923
- https://fasten.it/norms/norm/din_127
- https://fasten.it/norms/norm/din_6798
- https://fasten.it/norms/norm/din_6796
- https://fasten.it/norms/norm/din_471
- https://fasten.it/norms/norm/din_472
- https://fasten.it/norms/norm/din_7
- https://fasten.it/norms/norm/din_1
- https://fasten.it/norms/norm/din_en_iso_8734
- https://fasten.it/norms/norm/din_1481
- https://fasten.it/norms/norm/din_94
- https://fasten.it/norms/norm/din_6888
- https://fasten.it/norms/norm/din_660
- https://fasten.it/norms/norm/din_661
- https://fasten.it/norms/norm/din_en_iso_15983

**Endereços que deram 404 (slug a descobrir):** `din_en_iso_7380(_1)`, `din_935`, `din_en_iso_7379`, `din_en_iso_2339`. A página `din_6885` abriu sem a tabela.

**Fixadores de chapa, travas e insertos:**
- PEM CLS (pemnet.com) e Swaco: https://catalog.pemnet.com/item/nuts-types-s-ss-cls-clss-cla-sp/self-clinching-nuts-types-s-ss-cls-clss-sp-unified/cls-256-2 · https://www.swaco.com/CLS-M10-2-Nut-for-Sheet-Metal
- Nord-Lock (revendas): https://www.transcat.com/nord-lock-nl6-wedge-locking-washers-m6-id-0-26 · https://www.transcat.com/nord-lock-nl10-wedge-locking-washers-m10-id-0-42
- Helicoil: https://www.mcmaster.com/products/helicoils/thread-size~m10/ · https://eshop-hu.boellhoff.com/out/media/pdf/HELICOIL_Plus_Bohr_Schneidwerkzeuge_M6-M16___en.pdf
- Rebite-porca: https://www.gesipa.com/products/polygrip-blind-rivet-nuts/1433792-polygrip-blind-rivet-nuts-alu-dome-head-m-6-x-9-x-18/ · https://www.honsel.de/en/products/productdetails/blind-rivet-nut-ssm-r/ · https://www.eugenefast.com/details/product/rivets/rivet-nuts/m6-flange-knurled-body-rivetnut-0-70-4-20mm-grip-steel-yellow-zinc-plated/RNNM06C1SY

**Polegada:**
- https://webstore.ansi.org/preview-pages/ASME/preview_ASME+B18.21.1-2009.pdf
- https://amesweb.info/Fasteners/Nut/Hex-Nut-Sizes-Dimensions-Table.aspx
- https://amesweb.info/Fasteners/Washers/Helical-Spring-Lock-Split-Washer-Dimensions.aspx
- https://ns1.engineersedge.com/heavy_nut.htm
- https://mail.torqbolt.com/asme-b18-3-hexagon-socket-and-spline-socket-button-head-cap-screws-dimensions-standards-specifications

**CAD avançado A:**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)

As páginas help.CAD avançado A.com só entregaram o menu do site no `WebFetch`. O detalhe veio dos resumos de busca e dos blogs GoEngineer e Hawk Ridge.

**CAD avançado C:**
- (fonte em procedencia.md) (Família)
- (fonte em procedencia.md) (Publish Guide)
- (fonte em procedencia.md) (Bolted Connection)
- (fonte em procedencia.md) (apostila AU; o PDF não foi lido direto, só o resumo da busca)
- (fonte em procedencia.md)

**CAD avançado B:**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md) (bloqueada, só o trecho da busca)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)

**Outros CADs:**
- CAD avançado H: https://www.soliddna.com/SEHelp/ST5/EN/a_h/fastener1c.htm · https://www.soliddna.com/SEHelp/ST5/EN/a_h/fastener1h.htm
- CAD avançado F: (fonte em procedencia.md) (a página de IFX redirecionou para 404; o conteúdo veio do resumo da busca) · (fonte em procedencia.md)
- CAD avançado G: (fonte em procedencia.md)
- Fusion: (fonte em procedencia.md) (403; só o trecho da busca) · (fonte em procedencia.md)
- CAD avançado E: (fonte em procedencia.md)

---

## Entrega (resumo para o lead)

- **Resultado:** relatório completo da seção 1 à 6, com fichas por família (parafusos, porcas, arruelas, pinos, anéis, chavetas, insertos, rebites, clinch, polegada e funções inteligentes), 24 RF e 8 RNF, e 16 itens de backlog. Tem cerca de 9.000 palavras, dentro do alvo.
- **Evidência:** código lido com linhas citadas. As medidas [V] vieram de páginas abertas nesta sessão (fasten.it, PEM, revendas).
- **Arquivos alterados:** nenhum. Não criei o arquivo do plano nem usei ExitPlanMode, porque a entrega pedida é o texto da resposta e não há ferramenta de saída do modo plano nesta sessão.
- **Verificações feitas:** só leitura e web. Não rodei `cargo`.
- **Falhas e limites:**
  - As páginas help.CAD avançado A.com não abriram no `WebFetch`, só deram o menu do site. O detalhe do CAD avançado A veio de resumos de busca e dos blogs GoEngineer e Hawk Ridge.
  - fasten.it deu 404 em ISO 7380, DIN 935, ISO 7379 e ISO 2339.
  - A DIN 6885 abriu sem tabela. Não encontrei t1 da Woodruff nem as medidas de castelo/cega/borboleta.
  - Os dados de polegada são de revendas.
  - As páginas de CAD avançado D, CAD avançado F e eng-tips foram bloqueadas, 403 ou 404.
- **Riscos:**
  - Quase todas as medidas têm uma só fonte; a 2ª conferência exigida pelo nível C não foi feita.
  - As normas DIN 7, DIN 1, DIN 94, DIN 980, DIN 6798, DIN 6888 e DIN 6923 constam como retiradas no fasten.it. A família precisa citar a norma vigente.
  - Os rótulos de norma do arquivo de contexto precisam de conferência.
  - Algumas afirmações sobre o CAD avançado B são [C].

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra em prendedores são: (1) modelo de família fechado (`enum` de 4 tipos, só "M…", `f64`); (2) fixadores sem PN, classe, material próprio e massa verificada; (3) preenchedor cego a rebaixo/escareado/polegada, com regra de comprimento 2d–10d e cache de arquivo sem versão; (4) ausência de pinos, anéis DIN 471/472, Woodruff/cônica, insertos, rebites, PEM e ASME; (5) pilha não editável nem recalculável após inserida.
