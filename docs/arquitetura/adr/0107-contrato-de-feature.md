# ADR 0107 — Contrato único de feature

Estado: aceito como direção (21/11/2026), implantação em ondas. Item: B565. Amarra B558–B576 (em especial
B559, B560, B561, B562, B563, B566, B568, B569, B574, B575, B576). Revisa na prática os ADRs 0017 (prévia
fantasma), 0021 (ajustes em struct própria), 0032 (seleção única), 0073 e 0075 (prévia leve); mantém o
0010 (referências v2) como base das chaves. Fontes do estado atual:
[motor-transversal.md](../comandos/motor-transversal.md),
[chapa-e-estrutural.md](../comandos/chapa-e-estrutural.md),
[superficies-e-sketch.md](../comandos/superficies-e-sketch.md),
[solidos-montagem-desenho.md](../comandos/solidos-montagem-desenho.md) e
[selecao-auditoria.md](../../benchmark/detalhe/selecao-auditoria.md).

## Contexto

O teste da alpha.41 (21/11) mostrou que cada bancada parece um programa diferente: a extrusão sólida e a
de superfície não se parecem, a aba por superfície não tem as opções das outras abas, selecionar vários
elementos só funciona em três campos e inverter tem cinco formas. Os mapas de 21/11 dão a causa: não
existe contrato de comando. Cada comando escolhe sozinho o widget, a regra de campo vazio, a prévia, o
inverter e o tratamento de erro, e o motor transversal repete a mesma decisão em vários `match`.

Fatos que sustentam a decisão (todos com âncora nos mapas):

1. **Sete widgets de seleção** (slots, `geo_ui::bag`, `faces_ui`, `sketch_pick`/`sketch_combo`,
   `plane_combo`, texto com coordenada, `MateRef`). Só quem escolheu o widget certo ganha Tab, cadeia e
   clique no 3D.
2. **Três regras de campo ativo** que precisam concordar: `campos.rs`, `entradas::campo_ativo` (com
   `match` próprio por `FeatureKind`) e `slots::fill`/`fill_special`.
3. **Três convenções de "ainda não escolhi"**: id sentinela `UNSET_*`, ponto `[0;3]` e comparação com o
   texto `"passava perto de (0.00, 0.00, 0.00)"` na mensagem de erro. Mudar a redação quebra a regra.
4. **Perfil por sketch inteiro** (`Extrude.sketch: Id`, `Loft.sections: Vec<Id>`): nenhum comando de
   perfil aceita curvas de origens diferentes; a única saída é criar uma Junção de curvas à mão.
5. **Curva como polilinha densa** (`Curve3`) em toda entrada de superfície; o kernel refaz uma BSpline só
   se nenhum canto passar de 20°, senão vira N segmentos retos (B559). A spline de sketch tem três
   definições diferentes (solver, desenho, sólido).
6. **Inverter** com nomes e campos diferentes (`flip`, `reverse`, `outward`, `side`, sinal do valor) e
   20+ comandos sem botão; nas superfícies o painel do filete chega a descartar o campo.
7. **Limites** só existem completos na aba por superfície (lado 1, lado 2, borda final); a extrusão tem
   oito términos próprios, a de superfície tem dois, a revolução nenhum.
8. **Posição do material** (face interna, externa, começo do raio) existe na aba de aresta, curva e laço,
   mas é fixa em `0` na aba por superfície (B563).
9. **Concluir não valida nem espera**: o comando entra na árvore com ✖, pode concluir antes de o
   resultado voltar e uma falha no meio do comando deixa corpo parcial e dobra fantasma (`run` não
   restaura `bodies` nem `sheet_maps`).
10. **Cache**: alternar dobrado/planificado refaz a linha do tempo inteira (`doc.sheet` está na assinatura
    global); quem cita face assina todos os corpos; a planificada exata usa um `Cache` novo.
11. **Modos irmãos divergentes**: só a aba tem comando unificado com modos (`aba_unif.rs`); dobra/joggle/
    veneziana, extrusão/corte (seis `Cmd`), quatro furos e as três extrações divergem em nomes e opções.
12. **Duas listas e dois tipos**: `features: Vec<Feature>` com `FeatureKind` e `references` com `RefKind`,
    com dois painéis, dois caches e dois laços de recompute. Superfícies não herdam prévia leve, faixa de
    entradas nem validação.

O pedido do usuário é a filosofia dos softwares de modelagem CAD avançados: **suporte → modo →
parâmetros → avançado**, com seleção fácil, prévia viva e confirmação que não engana.

## Decisão

Todo comando passa a ser descrito por **dados** num descritor declarativo (nome de trabalho:
`FeatureSpec`). O motor transversal (painel, seleção, campo ativo, filtro, prévia, validação, cache,
árvore, reparo, testes) lê o descritor em vez de cada comando escrever um `match` em nove lugares. Os
nomes de tipos abaixo são de trabalho; a implementação pode escolher outros, desde que cumpra cada item.

### 1. Descritor declarativo por feature

- Cada feature declara uma lista **ordenada** de entradas, agrupadas nas quatro seções fixas do painel:
  **Suporte** (o que recebe a operação), **Modo** (variante do comando), **Parâmetros** (valores,
  direção, limites, posição) e **Avançado** (recolhido, opções raras).
- Cada entrada tem: `id` estável (usado em mensagens e testes), rótulo traduzido, dica de três linhas
  (o que faz, faixa, efeito), **tipo aceito** (`Face`, `Aresta`, `Vertice`, `Ponto`, `Curva`, `Perfil`,
  `Plano`, `Eixo`, `Superficie`, `Corpo`, `Feature`, `Numero(unidade)`, `Escolha`, `Direcao`, `Limite`,
  `Posicao`), **cardinalidade** (`Um`, `Varios{min, max}`, `Cadeia`), obrigatoriedade e **validador**
  (`fn(&Documento, &Valor) -> Result<(), Motivo>`; ex.: "arestas do mesmo corpo", "curva plana",
  "plano diferente do plano do sketch").
- Do descritor saem, por uma função só:
  - o **campo ativo** (primeira entrada obrigatória vazia, na ordem declarada);
  - o **filtro de seleção** temporário do 3D (tipos aceitos pelo campo ativo);
  - a **ordem do Tab** (Tab, Shift+Tab, Enter avança enquanto houver campo de seleção adiante);
  - o estado **incompleto** (`Inactive` sem erro), por um valor explícito `Vazio` no próprio campo.
- **Fim** das três convenções de campo vazio (`UNSET_*`, ponto `[0;3]`, texto da mensagem) e das três
  regras de campo ativo (`campos.rs`, `entradas::campo_ativo`, `slots::fill`). Ler arquivo antigo converte
  o sentinela no `Vazio`.
- O mesmo descritor declara: ícone, família de cor do tema, tooltip com animação, alças 3D por grandeza,
  nível de prévia (seção 8), irmãos (seção 10) e se o comando tem ciclo próprio (PMI, Desenho, Montagem,
  Forma, Medir enquanto não migrarem).
- **Valor com referência**: toda entrada topológica guarda a referência v2 (ADR 0010) por um único
  mecanismo. As cinco formas de hoje (listas paralelas, `*_key`, chave dentro do `*Ref`, `Feature::ancoras`,
  `RefKind{key}`) passam a ser leitura de compatibilidade; `Promoted` deixa de ter um ramo por comando.

### 2. Caixa de seleção universal em árvore

- Todo campo de seleção usa **o mesmo widget**: uma caixa opaca em forma de árvore, com nós por **tipo**
  (faces, arestas, curvas...) e por **origem** (sketch, wireframe, corpo, extração).
- Recebe clique no **3D** e na **árvore de recursos** pelo mesmo caminho (fim da cascata `if/else` de
  `app.rs` e de `painel_consome` como rota paralela).
- Teclas iguais em todo lugar: clique marca o item, **Ctrl** soma/alterna, **Shift** marca intervalo,
  **Del** ou **X** tira os marcados; clique no 3D sobre um item já presente o tira.
- **Validação na hora**: cada item que entra passa pelo validador do campo; item recusado não entra e o
  campo mostra a mensagem dele (ex.: "esta aresta é de outro corpo"). Contador mínimo/máximo visível.
- **Ordem quando a feature precisar**: o descritor marca `ordenado`; a caixa mostra a posição de cada item
  e deixa reordenar (▲▼ ou arrastar). Ex.: nó de tubos do B574 (o primeiro tubo é cortado pelo de baixo,
  o seguinte pelos anteriores), seções do loft, sequência de guias.
- Substitui `faces_ui`, `plane_combo` (vira campo `Plano` com clique no 3D), `sketch_pick`/`sketch_combo`
  (o sketch vira um item de origem na caixa de curvas), os campos de texto com coordenada (rosca,
  converter, joggle, dobra, veneziana, placa, esquadro) e, na onda da Montagem, o `MateRef`.
- Cores de realce e do retângulo de seleção só pelo tema (hoje há RGB fixo em `app.rs`).

### 3. Perfil e guia como lista de curvas de origens mistas

- Campo de **perfil** ou **guia** é `Curva` com cardinalidade `Varios` ou `Cadeia` e aceita, no mesmo
  campo, curvas de sketches diferentes, wireframe, arestas do corpo e curvas extraídas.
- **Cadeia tangente é recurso do campo** (`cadeia: Tangente | Ponto | Nenhuma`), não do comando: o clique
  numa curva expande pela mesma implementação (`extract::edge_set` G0/G1/G2), com **uma** tolerância de
  encadeamento, e a regra fica gravada (como o filete já faz desde o ADR 0032). Fim das cinco
  implementações de "seguir tangentes" (`App::chained`, `tangent_chain`, `aresta_clicada`, `extract`,
  `selecao_regras`).
- **Junção na hora**: quando a feature precisa de um suporte só, o motor junta as curvas (ou superfícies)
  do campo antes de calcular, com o código de `join_curves`/`chain_indices`. **Recomendação deste ADR:
  gravar a lista no próprio comando** (o campo guarda os itens e a regra; a árvore não ganha item
  escondido; `refs_of`, `dependents` e a assinatura enxergam o mesmo grafo). Ver Decisões do usuário, 1 (subitem dentro do comando).
- **Botão direito "unir" no item**: força juntar itens marcados num grupo do campo (ou desfazer), com
  aviso se as pontas não se tocam na tolerância.
- Aplica-se aos comandos de perfil de todas as bancadas: extrusão, revolução, varredura, loft, nervura,
  plástico, furo por sketch, parede base, recorte, estampo, aba de perfil, aba entre perfis, dobra por
  sketch, furo com colar, extrusão/revolução/varredura/concordância/preencher de superfície e guia do
  membro estrutural (B575).

### 4. Extrair continua Extrair; curva exata no kernel

- **Extrair continua Extrair** (B576): extrair face, aresta, borda ou curva gera recurso Extrair na
  árvore, associativo ao original, com o mesmo tipo geométrico. Não vira Linha, Curva nem Junção de
  curvas. A junção na hora **não** substitui o Extrair; o campo oferece "Extrair" no menu do item.
- **Curva exata como representação** (B559): nada de polilinha densa como dado da curva. Linha, arco e
  círculo chegam ao kernel como geometria analítica; spline e aresta curva como BSpline/NURBS; uma aresta
  extraída mantém a curva do B-rep. Polilinha só existe como malha de desenho e de picking.
- **Uma definição única de spline** em sketch, wireframe e kernel (mesma parametrização e tangentes),
  usada pelo solver, pelo desenho, pelo Aparar e pelo sólido.
- Continuidade G1/G2/G3 (conectar, concordância, preencher) é medida na curva exata, não em amostras.

### 5. Um único inverter

- Toda entrada `Direcao` tem **um** estado `inverter: bool` e **um** widget padrão ("⇄ Inverter direção",
  o mesmo de `chapa_unif_ui::inverter`), com o mesmo texto e ícone em todas as bancadas, inclusive na
  Montagem.
- A seta clicável no 3D e o botão do painel ligam ao **mesmo campo**; a prévia atualiza nos dois casos
  (B562). Fim das caixas "inverter sentido", "para fora", "espelhar seção", "Inverter D1 e D2", rádios
  Externo/Interno e Acrescentar/Remover usados como inverter.
- Onde o sentido é o sinal do próprio valor (mover corpo, deslocar), o descritor declara
  `sinal_do_valor`, o painel não mostra o botão e a dica diz por quê.
- Um segundo inverter só existe quando é outra grandeza (ex.: "lado" da dobra, além da direção), com
  nome próprio igual em todos os irmãos.

### 6. Tipo de limite único

- Um tipo `Limite` serve a todo comando com extensão: **lado 1**, **lado 2** e **borda final**, cada um
  podendo ser valor, ou "até" um **plano, face, curva, aresta, sketch ou elemento de construção**, mais
  passante, até o próximo e simétrico onde fizer sentido.
- **Extrapolar, aparar e projetar são automáticos**: se o elemento de limite não alcança, o motor o
  estende; se passa, apara; se não está no plano da operação, projeta. A pessoa não cria superfície
  auxiliar.
- A chave do elemento de limite é referência v2 (`Promoted::Ate` vira o caso geral).
- Validação antes do kernel: valor positivo, limite não paralelo à direção, limite que intercepta (após
  estender). Erro sai no campo do limite.
- Unifica `Extent` (extrusão, rosca, furo), os limites da extrusão de superfície e `LimitesAba` da aba por
  superfície; a revolução e a extrusão de superfície ganham os términos que faltam.

### 7. Posição do material

- Onde houver dobra ou espessura (abas, bainha, parede base, nervura, parede fina, espessar, placa), a
  entrada `Posicao` tem os mesmos valores e nomes em todos os modos: **face interna**, **face externa**,
  **começo do raio** (e "centrada" onde a espessura se divide).
- A aba por superfície deixa de passar `side = 0` fixo (B563); a superfície escolhida pode ser a própria
  face da aba, com distância zero.
- Inverter e posição são independentes e as quatro combinações têm de dar sólido válido.

### 8. Pré-cálculo, validação antes do OK e confirmação com espera

- **Pré-cálculo leve** durante a edição (B568): o descritor declara o nível (`Leve{caixas|linhas|
  ferramenta}` ou `Completa`); o motor roda a validação dos campos e a checagem rápida (geometria,
  limites, vai planificar, autointerseção) e mostra o erro **no campo** antes do OK. A vista continua livre
  para girar.
- **Erro por campo**: o motor devolve `Erro{campo, causa, sugestao, acao}` (a estrutura de `Acionavel` já
  existe) em vez de um `String` montado com `format!`. A mensagem é traduzível com argumentos. Fim do
  `campo_citado` por texto.
- **OK que espera o resultado**: Concluir envia o cálculo completo (sem leve), **congela a tela** com
  indicador de progresso (`Cache::progresso`) e só fecha a sessão quando o resultado daquela geração volta
  sem erro. Cancelar durante a espera usa `Cache::cancelar`.
- **Painel não fecha em falha**: volta ao painel com o erro no campo; o comando não entra na árvore com ✖.
  Exceção: atualização manual (`ModoAtualizacao`), que segue as regras dela.
- **Rollback**: `Ctx::run` guarda `bodies`, `sheet_maps`, `previas`, `tools` e `last_sheet` antes do
  comando e restaura tudo em `Err`. Nenhum corpo parcial, nenhuma dobra fantasma.
- A validade do sólido (`validade::invalido`) roda dentro da confirmação, não depois.

### 9. Cache por feature e por estado

- A assinatura de cada comando é calculada **dos campos declarados** (valores, referências e chaves), não
  do JSON filtrado por substring. Quem cita uma face depende do corpo e das chaves citadas, não de todos os
  corpos.
- **Invalidação só pela árvore**: o que invalida é mudança de campo, de entrada ou de parâmetro citado;
  nome, cor e visibilidade não invalidam.
- **Dobrado e planificado calculados juntos** (B566): cada comando de chapa guarda os dois estados no
  cache; `flat` sai da assinatura global; alternar só troca a exibição. A planificada exata reaproveita o
  cache do documento aberto.
- **Malha de exibição em cache** por corpo e por assinatura (B568): reabrir sem mudança não re-tessela;
  prévia já calculada fica guardada.
- `FeatureKind` e `RefKind` entram na mesma tabela de cache.

### 10. Modos irmãos são um comando com modo

- Irmãos viram **um comando com `modo`**, como `aba_unif.rs`: aba (arestas, perfil, superfície, bainha),
  extrusão e corte, os furos, dobra/joggle/veneziana, extrair (uma, várias, com tangência), extrusão
  sólida e de superfície onde couber. Trocar de modo leva os campos comuns.
- **Paridade por teste automático**: para cada grupo de irmãos declarado no descritor, os campos comuns
  existem em todos, com o mesmo `id`, rótulo, faixa, inverter e posição. Falta quebra o teste (extensão de
  `checklist_comandos.rs`).
- **Textos de operação iguais** em todo lugar: Adicionar, Remover, Interseção (e Corpo novo onde houver),
  um só componente (hoje `op_combo`, `hyb_op` com "Acrescentar", `op_choice` e caixas próprias).

### 11. `features` e `references` no mesmo contrato

- `FeatureKind` e `RefKind` implementam o mesmo descritor e passam pelo mesmo painel, campo ativo, prévia,
  validação, cache e reparo. **Não** é obrigatório fundir os dois tipos nem as duas listas agora; a fusão,
  se vier, é outro ADR.
- `geo_ui::panel` deixa de ser um `match` de mil linhas: cada `RefKind` declara suas entradas.

### 12. Interface (B319)

Todo comando novo ou alterado cumpre o checklist de
[interface.md](../../benchmark/detalhe/interface.md): cores só por tema com a peça distinta do fundo, ícone
da família, painel livre e encaixável aberto só durante o comando (ADR 0105), árvore transparente, prévia
e manipulação no 3D, dica por campo, tooltip com animação e tradução de todo texto em `i18n.rs`. O
checklist medido por código passa a ler o descritor.

## Consequências

- Comando novo escreve o descritor e a função de cálculo; painel, seleção, Tab, filtro, prévia, erro por
  campo, cache e testes de paridade vêm de graça. Cada bancada passa a parecer o mesmo software.
- O B561, o B562, o B563 e o B568 deixam de ser correções por comando e viram migração para o contrato.
- A migração é grande (estimativa da auditoria: cerca de 2 milhões de tokens para a seleção e o inverter,
  mais confirmação, cache e curva exata) e toca os gargalos `app.rs`, `editor.rs`, `feat_ui.rs`,
  `geo_ui.rs`, `fenkra-solidos/src/lib.rs` e `doc.rs`. Precisa de ondas com coordenação entre bancadas
  (agente `arquiteto`).
- Arquivos antigos: o leitor converte sentinelas em `Vazio`, as cinco formas de chave em referência única
  e os comandos irmãos antigos (`SmFlange`, `SmCurveFlange`, `SmFlangeLoop`...) no comando com modo, sem
  mudar o resultado (teste de abertura com volume e caixa iguais).
- Curva exata muda resultados que hoje dependem da polilinha (cilindro passa a ser cilindro, furo e rosca
  passam a ser reconhecidos). Testes que fixaram volume de geometria facetada precisam ser revistos.
- Confirmação com espera torna o OK mais lento em modelos pesados; em troca, a árvore só recebe comandos
  válidos. O progresso e o cancelar tornam a espera visível.
- Até cada onda chegar, convivem o caminho antigo e o novo; a skill `fenkra-comando` diz a cada especialista
  que comando novo já nasce no contrato.

## Alternativas

1. **Trait por comando em vez de descritor de dados.** Cada comando implementaria `campos()`, `validar()`,
   `previa()`. Recusada como forma principal: a paridade entre irmãos e o checklist precisam comparar
   **dados**; código livre por comando repete o problema de hoje. Um trait fino para o cálculo continua
   possível ao lado do descritor.
2. **Só padronizar widgets, sem descritor.** Trocaria os sete widgets por um, mas manteria as três regras
   de campo ativo, as três de campo vazio e o `match` por comando no filtro, na prévia e no reparo.
   Recusada: atacaria o sintoma.
3. **Fundir `FeatureKind` e `RefKind` primeiro.** Daria um tipo só, mas é a mudança mais arriscada (dois
   laços de recompute, dois caches, migração de todo arquivo) e não entrega nada ao usuário sozinha. Fica
   para depois do contrato, se ainda fizer sentido.
4. **Junção na hora como Junção de curvas oculta na árvore.** Reaproveita `CurveJoin` sem mudar o tipo de
   entrada dos comandos. Não recomendada: cria item invisível com dependentes, confunde apagar e
   substituir referência e duplica a fonte de dependência. Mantida como opção na Decisão pendente 1.
5. **Bloquear o OK só desabilitando o botão, sem esperar o cálculo.** Mais simples, mas não fecha a
   lacuna de concluir antes de o resultado voltar. Recusada.

## Ondas

Coerentes com a [auditoria de seleção](../../benchmark/detalhe/selecao-auditoria.md); chapa e superfícies
primeiro porque são as bancadas da missão aeronáutica (B567) e as que mais sofrem.

| Onda | Escopo | Itens |
|---|---|---|
| 0 | **Base comum**: descritor mínimo (entradas, tipos, cardinalidade, validador); `Vazio` explícito; campo ativo, filtro, Tab e incompleto derivados do descritor; caixa universal em árvore com Ctrl/Shift/Del/X e validação; cadeia como recurso do campo; junção na hora; widget único de inverter; rollback em `Ctx::run`; erro por campo (`Erro{campo,...}`) | B561, B562, B565 |
| 1 | **Chapa**: aba em um modo Arestas (B560); aba por superfície com posição, inversão e limites no padrão (B563); comandos de perfil pelo campo universal; bainha, conversão, furo com colar, joggle e veneziana para a caixa; dobrado e planificado no cache (B566); prévia leve cacheada da aba por superfície (B564) | B560, B563, B564, B566 |
| 2 | **Superfícies**: curvas de entrada pelo campo universal (extrusão, revolução, varreduras, loft, concordância, regrada, dividir); inverter no painel ligado à seta; `RefKind` no mesmo contrato (painel, prévia leve, validação); curva exata e spline única | B559, B576 |
| 3 | **Confirmação e cache**: pré-cálculo leve declarado, OK com espera e painel que não fecha, cache por campos declarados, malha de exibição em cache | B568 |
| 4 | **Sólidos**: comandos de perfil e de faces no contrato; limite único na extrusão, revolução, furo e rosca; operação com textos iguais; planos e eixo por clique | B569 |
| 5 | **Estrutural**: guia do membro como cadeia com junção e Extrair; placa, esquadro e solda na caixa; nó de tubos com ordem; inverter padrão | B574, B575 |
| 6 | **Montagem, Desenho, PMI, Forma, Medir**: seleção pela caixa universal, inverter padrão; migrar para a sessão comum ou declarar ciclo próprio no descritor | — |
| 7 | **Testes de paridade** de todos os grupos de irmãos e checklist B319 lido do descritor (começa na onda 0 e cresce a cada onda) | B565, B569 |

A onda 0 vem antes de todas. As ondas 1 e 2 podem correr em paralelo depois dela, em worktrees, porque
tocam arquivos diferentes (`feat_ui`/`chapa_*` contra `geo_ui`/`geo3d`), com o agente `arquiteto` revendo
a coerência antes de integrar.

## Decisões do usuário (21/11/2026)

1. **Junção na hora**: a união fica **dentro do comando, como subitem** dele (na caixa de seleção e na
   árvore, sob o comando), sem recurso Junção de curvas oculto. O botão direito no item continua
   oferecendo "unir" explícito.
2. **Abrangência do B561**: **todas as bancadas** (Chapa, Superfícies, Sólidos, Estrutural e Montagem;
   Montagem por último).
3. **OK em modelo pesado**: **sempre espera**, com a tela congelada e progresso até terminar ou falhar;
   Esc cancela. Nada de cálculo em segundo plano depois do OK.
4. **Extrusão (e revolução) sólida e de superfície**: **dois comandos irmãos com as mesmas opções**,
   conferidas por teste automático de paridade (não um comando com modo).

## Decisão ainda em aberto

- **Fundir `FeatureKind` e `RefKind`** depois do contrato, ou manter os dois tipos. Não bloqueia as ondas.

## Estado da implantação

### Onda 0, base comum (22/11/2026; B561 e B562, parte base)

- **Descritor** em `crates/fenkra-app/src/contrato.rs` (`FeatureSpec`, `Entrada`, `TipoEntrada`,
  `Cardinalidade`, `Candidato`, `Motivo`). O descritor é montado do valor atual do comando, então cada
  entrada já traz `n` (quantos itens reais) e o campo vazio é um estado explícito. Dele saem o campo ativo
  (`entradas::campo_ativo`), o estado incompleto (`slots::feature_incomplete`), a contagem do painel, o
  filtro temporário do 3D (`selecao::tipos_do_comando`) e o campo ativo do painel (`campos::definir_contrato`).
  `PONTO_VAZIO` e `ponto_vazio` são o único lugar que sabe como um campo de coordenada vazio se parece.
- **Adaptador**: comando que ainda não declarou entradas é lido pelos marcadores `UNSET_*` (mesma ordem de
  prioridade de antes). Migrados: casca, saída, espessura, edição direta, rosca, filete e chanfro (arestas,
  entre faces, completo), aba, bainha, laço de abas, aba curva, dobra por linha, joggle, veneziana, loft e
  membro estrutural (sólidos, chapa e estrutural); dividir, aparar, filete entre superfícies, regrada,
  curva paralela, juntar, loft e junção de curvas (superfícies).
- **Caixa universal em árvore** em `caixa_arvore.rs` (desenho) e `caixa_selecao.rs` (lógica). `geo_ui::bag`
  é a mesma caixa sem descritor; `geo_ui::bag_contrato` é a caixa com ordem, junção, validação e nós por
  origem. O clique na árvore de recursos entra no campo ativo pelo mesmo caminho do 3D
  (`contrato::picks_da_arvore`).
- **Junção na hora como subitem do comando**: os grupos ficam em `Document::campos` (`DadosCampo`, em
  `fenkra-core/src/campo_dados.rs`), junto do comando e do campo, com `serde(default)`; a lista de itens do
  comando não muda. **Pendente para a onda das superfícies (geo3d):** o cálculo ainda trata cada item da
  lista como uma curva; falta ler `Document::dados_campo` para juntar os itens de um grupo antes do kernel.
- **Inverter único** em `inverter.rs`: o botão do painel e a seta do 3D chamam a mesma função
  (`alternar_ref`, `alternar_feature`); `chapa_unif_ui::inverter` é o widget padrão.

### Onda 1, chapa (22/11/2026; B560 e B563, mais as pendências da revisão da onda 0)

- **B560, aba de arestas num modo só.** A barra de modos da aba tem Arestas, Perfil e Superfície. O
  suporte do modo Arestas é uma lista que mistura retas e curvas, com "seguir tangentes" e contorno da
  face; os três recursos antigos continuam gravados e abrem iguais. Correção de kernel achada pelo teste
  da lista mista: `ordena_cadeia` crescia a cadeia só pelo fim, e a lista "borda, canto, borda" falhava.
- **B563, aba por superfície no padrão das irmãs.** `SmSurfFlange::side` (posição do material) e
  `inv_lado` (lado da superfície); nervura de asa plana e curva nas três posições e quatro inversões.
- **Descritor.** Declaradas as entradas de face dos comandos de chapa e estrutural que ainda estavam no
  adaptador. `feat_ui::wants_faces` é agora uma consulta ao descritor (`contrato::quer_face`), e o filtro
  do 3D (`selecao::tipos_do_comando`) sai só dele. `contrato::preencher` substitui os braços de chapa de
  `slots::fill_special`. A busca de `UNSET_*` no JSON em `fill_from_selection` foi trocada por uma
  consulta aos campos vazios do descritor.
- **Inverter.** `inverter::bool_de_feature` é o campo único do botão e da seta; `controles`, `estado` e
  `alternar_feature` ganharam os comandos com `bool` de inverter; teste `botao_e_seta_ligam_ao_mesmo_campo`.
- **Tab.** `campos.rs` usa `ordem_tab` do descritor no lugar do "primeiro vazio" quando o campo ativo não
  é um dos declarados.
- **Pendente.** (1) Rótulo e dica do descritor ainda não são lidos pelo painel (cada widget traz os
  seus). (2) A seta clicável de inverter do 3D das features (extrusão, furo, abas) depende de
  `sup3d::setas_de_lado`, que está na onda das superfícies; o campo já é o mesmo, falta desenhar a seta.
  (3) `slots::aresta_clicada` (antigo) segue como retorno para a bainha. (4) A prévia leve e o cache do
  dobrado e planificado (B564, B566) ficam para a próxima onda.

### Onda 2, superfícies (22/11/2026; B561 e B562)

- **Junção na hora com efeito geométrico** (fecha o pendente da onda 0): `Ctx::curvas_do_campo`
  (`fenkra-solidos/src/lib.rs`) lê `Document::dados_campo` do comando e junta os itens de cada grupo numa
  cadeia exata (`geo3d::juntar_no_campo`, que usa `join_curves`: trechos encadeados, G0/G1) antes do kernel.
  Ligado na Revolução (`g_pf`), Regrada (`g_rc`) e Loft de superfície (`g_sec`). Tolera índice fora da lista
  (nota) e avisa quando as pontas não se tocam na tolerância (os itens seguem separados). A junção entra em
  `ref_sig`, então unir ou separar refaz o cache. A Revolução ganhou `mais` (perfil em lista, `serde(default)`).
- **Curvas de entrada pelo campo universal**: Extrudada, Revolução, Varredura (perfil e guia), Loft (caminho),
  Concordância (bordas), Dividir (curva de corte), Paralela e Extrair (mostra os itens de `mais`) usam
  `bag_contrato` (`geo_ui::curva_contrato` para um item, `perfil_lista` para lista); a cadeia dupla
  (`curve` + `mais`) passa pelo motor do campo em `contrato::editar_curvas`. Os `RefKind` de superfície que
  estavam no adaptador declaram os campos em `contrato::declaradas_ref`.
- **Inverter da extrusão de superfície**: sentido = sinal dos dois limites (`sinal_do_valor`), com o botão
  padrão logo abaixo da extensão (como a extrusão sólida) e seta no 3D; os dois chamam
  `inverter::alternar_extrusao` (`expr::negar`, volta ao texto de antes). O botão do Engrossar mostra o estado.
- **Mesmo nome para o mesmo papel**: `g_s` Superfície, `g_apoio*` Apoio, `g_a`/`g_b` primeiro e segundo
  elemento, `g_borda1/2` Borda, `g_pf` Perfil, conferido por teste no descritor e no painel.
- **Testes de paridade**: extrusão sólida × de superfície × engrossar (inverter, seções, Direção), botão == seta
  para Dividir, Aparar, Filete entre superfícies, Paralela, Regrada e Extrudada.
- **Pendente**: perfil em lista na Extrudada e na Varredura (o literal desses `RefKind` está em `handles.rs`,
  fora desta onda); Engrossar e Espessura com as mesmas opções e nomes (`feat_ui.rs`, fora desta onda).

### Onda 5, estrutural: tubo dobrado, spline e cadeia (22/11/2026; B575)

- **Um membro só.** `StructMember::tube_bend` (`Option<TubeBend>`, `serde(default)` e fora do arquivo quando
  vazio: arquivo antigo abre e recalcula igual). Ligado, os trechos vizinhos de um mesmo caminho ganham uma
  junta de dobra automática (`Ctx::juntas_dobra`), com o raio digitado ou o mínimo automático
  (`st_dobra::raio_minimo_automatico`: 2 vezes o diâmetro ou o maior lado, 3 vezes em parede fina; regra
  prática de duas fontes públicas, não norma). O canto entre eles é `Dobra` em `joint_rule`: sem encaixe nem
  esquadria. A dobra, o arco, a lista de corte e o desenho reaproveitam o nó de dobra do B326.
- **Caminho do membro** (`Ctx::pernas_do_caminho`): o campo "caminhos" passa por `curvas_do_campo` (junção
  na hora do ADR 0107), cada cadeia é quebrada em retas e cada trecho guarda o caminho de origem
  (`MemberGeom::caminho`). `Ctx::members` ganhou o id do comando. Com a dobra ligada o limite do canto é 5
  graus; sem ela, 11,5 como antes.
- **Spline e curva exata** seguem como um trecho só (B559); o teste de hélice 3D confere status, volume e caixa.
- **Painel.** A caixa de guias usa o id do campo declarado (`caminhos`), o que liga a junção na hora e o Tab
  do descritor; a seção "Tubo dobrado" tem a caixa de ligar, o raio (com o automático como dica) e "Unir as
  guias numa cadeia" (grava a junção pelo pedido `Pedido::Dados`, o mesmo do botão direito).
- **Tabela de dobras.** `st_dobra_maquina::tabela_de_dobras`: CSV com avanço, rotação, ângulo, raio, arco,
  ponto de dobra e posição no tubo reto; botão na lista de corte. A tabela de dobras do desenho ganhou
  Avanço, Rotação e Ponto de dobra.
- **Pendente.** Trecho curvo exato entre retas num caminho com dobra; seta e alças 3D do raio de dobra no
  caminho (as alças existentes são as do nó); descritor com o bloco "tubo dobrado" como entrada declarada.

### Pendências das ondas 1 e 2 (22/11/2026; B561 e B562)

- **Perfil em lista na Extrudada e na Varredura de superfície.** `RefKind::SurfExtrude` e `SurfSweep`
  ganharam `mais` (`serde(default)`, fora do arquivo quando vazio: arquivo antigo abre e grava igual). O
  painel usa `geo_ui::perfil_lista`; o motor do campo (`contrato::editar_curvas`) trata `curve` + `mais`
  como uma lista. O cálculo lê `Ctx::curvas_do_campo(id, "g_pf")`: um grupo unido vai ao kernel como uma
  cadeia exata e o prisma (`BRepPrimAPI_MakePrism` do fio) sai como **uma** superfície contínua (reta +
  arco: um domínio, 6 bordas livres; sem a junção, dois domínios e 8). Teste
  `fenkra-solidos/tests/perfil_em_lista_b561.rs`.
- **Espessura e Engrossar com as mesmas opções e nomes** (`feat_ui.rs`): entrada, seção Espessura (campo
  "Espessura" e `inverter::botao`), Operação com os textos da família (`feat_ui::OPERACOES`, os mesmos de
  `editor::op_combo`). Na Espessura o inverter e a operação ligam ao mesmo campo (`remove`): a camada de
  uma face sólida só tem um lado útil, então Dois lados, Simétrico e Interseção não se aplicam. O editor
  não soma mais um segundo botão de inverter depois do painel (a chapa aparecia com dois); cada painel
  desenha o seu. Teste `espessar_paridade_tests.rs`.
- **Seta clicável de inverter das features** (`sup3d::setas_de_feature`): extrusão, furo, espessura,
  parede, abas (reta, curva, laço, perfil, superfície com as duas setas), bainha, dobra, joggle e
  veneziana. Regra explícita onde existe (normal do sketch, lado da espessura da parede, lado móvel da
  linha de dobra); nos demais, do ponto de partida para o centro das faces que o comando criou. O clique
  chama `inverter::alternar_feature`. Teste `setas_feature_tests.rs` (botão == seta, a seta vira ao
  inverter, a da extrusão aponta para o corpo).
- **Rótulo e dica do descritor.** `contrato::ficha_no_doc` e `rotulo_no_doc`; a caixa em árvore mostra o
  rótulo e a dica da `Entrada`; `curva_contrato` e `perfil_lista` não recebem mais rótulo (lêem do
  descritor); `surf_slot` usa o do descritor quando o campo é declarado; a lista de arestas da aba (B560)
  usa o rótulo da entrada ("Arestas" nos três recursos). Teste `rotulo_descritor_tests.rs`.
- **Pendente.** Os widgets de ponto, eixo e plano ainda recebem o rótulo do chamador (o descritor ainda
  não declara esses campos em todos os comandos); a dica da `Entrada` só está preenchida no perfil em
  lista. A seta de feature por resultado só aparece depois de um cálculo com sucesso.
### Onda 5, estrutural: boca de lobo (22/11/2026; B574, parcial)

- **Corte normal à parede** no encaixe de tubo, com a parede que a curva segue e a face do tubo que recebe
  como opções no documento (`StructMember::fit`, `StructJoint::fit`). Decisão e conferência no
  [ADR 0109](0109-encaixe-de-tubo-corte-normal-a-parede.md).
- **Pendente**: tubo de passagem com rasgo, tubo contra superfície qualquer com ponte, escolha dos tubos
  que cortam um nó pela caixa universal (hoje a ordem é a da árvore) e o descritor dessas entradas.

### B566 e B564, chapa: estados em cache, validação e prévia simplificada (22/11/2026)

- **Dobrado e planificado no cache** (`Cache::trocar_estado`, `recompute_ambos`, `preaquecer_outro_estado`).
  A assinatura de cada comando já carrega `doc.sheet.flat` (via assinatura global), então o resultado de um
  estado nunca vale para o outro; em vez de tirar `flat` da assinatura (o que quebraria os comandos que
  recalculam por estado), o cache guarda **dois conjuntos** (`feats`, `meshes`, `props`) e `recompute`
  entrega o do estado pedido. O app calcula o outro estado em segundo plano depois de cada resultado
  (`Pedido::aquecer`, sem resultado, cancelável); alternar vira só `recompute_cached` com tudo reaproveitado.
  Medido: calcular os dois estados da nervura de 100 × 60 leva 4,4 s; **alternar leva 1,2 ms** (máximo 1,6 ms
  em 6 voltas, todos os comandos vindos do cache). Mudar um campo a montante invalida só o que depende, nos
  dois estados (teste `mudar_a_montante_invalida_so_o_que_depende_nos_dois_estados`).
- **Validação leve da aba por superfície** (`chapa_aba_diag.rs`): menor raio de curvatura côncavo da OML
  contra `r + t` (a dobra cabe) e `ba + comprimento` (a faixa plana não se cruza); o aviso dá o ponto, o raio
  e o comprimento máximo que ainda planifica. Entra como aviso do comando.
- **Aceite**: a nervura de asa com aba de 20 mm dobra e planifica nas três posições do material (teste
  `nervura_de_asa_com_aba_de_20_mm_dobra_e_planifica`).
- **Prévia simplificada da aba por superfície** (B564): com o comando aberto a OML é amostrada em 60 pontos,
  a web fica inteira (o corte pelo suporte só no resultado final) e a parede aparece como contorno em linhas
  (`PreviaViz::linhas`); sem varredura da dobra nem união. A superfície de suporte (varredura/deslocamento)
  fica no `Cache::suportes`, com chave pela definição (`ref_sig`). O resultado final é o do cálculo completo.
- **Pendente**
  1. **Dobrada de peça grande** (corda a partir de 400 mm): a união da varredura da dobra com a web sai
     inválida no kernel e o comando dá erro; o planificado funciona em todas as escalas medidas (até corda
     1000). Precisa de investigação no kernel (tolerância da união, seção entrando 0,05 mm). Tentativa de
     trocar o caminho exato pela polilinha não resolveu.
  2. Aproximação para superfície de dupla curvatura (estiramento/enrugamento) e recorte de alívio onde a
     OML fecha (hoje só o aviso).
  3. Os limites dos lados, a borda final e os cantos não aparecem na prévia simplificada (só no resultado).
  4. O cache dos dois estados não entra no teto de memória das malhas (`TETO_CACHE_MALHAS`).
  5. A validação não cobre "abas se cruzam" entre abas diferentes (só dentro da mesma OML).
  6. Textos de aviso do motor ainda em português fixo (sem tabela i18n, como os demais erros do motor).

### B577, subelementos com nome próprio (22/11/2026)

- **Regra.** Todo subelemento aparece como `Dono · Face n · Aresta m · Vértice k`, nunca por coordenada nem
  por índice interno. `fenkra_solidos::nomes_sub::SubNomes` é montada de cada resultado de recálculo
  (corpos e superfícies de referência). **Face n**: n-ésima do dono pela ordem do papel (`FaceKey::role`;
  na superfície, o índice da face). **Aresta m**: m-ésima da face de menor ordem entre as duas vizinhas,
  pela ordem da vizinha. **Vértice k**: ponta 1 ou 2 daquela aresta. A estrutura não muda quando uma cota
  muda, então o nome segue o elemento; o comando guarda a chave (`FaceKey`/`EdgeKey`) e o recálculo continua
  resolvendo pela chave (ADR 0010). A tabela só traduz.
- **Onde aparece.** `ref_nome` (campos de painel, regra gravada, âncoras), `Pick::label` (menu, barra
  "Seleção:"), `geo_ui` (extrair face, borda de superfície, superfície por faces), listas de face e aresta
  (`feat_ui`, `editor`, `st_solda_ui`), canto e face aberta da chapa (nome do que está no ponto), canto do
  estrutural (linhas que chegam nele) e nó do pórtico ("Nó n"). Sem chave, o campo diz "sem nome (espera o
  cálculo)"; o ponto vai só na dica.
- **Árvore na caixa** (`caixa_sub.rs`, em `caixa_arvore::caixa`, campos de face, aresta, superfície e
  curva): "Elementos do modelo" com dono, faces, arestas e vértices; a seta abre, o clique escolhe
  (`Pedido::Selecionar`), acende no 3D e entra no campo ativo pelo caminho do clique no 3D.
- **Pendente.** Vértice sem chave própria (`ChaveAncora::Vertice`); faces com `role < 0` (varredura, loft,
  revolução, membro estrutural) têm número mas não chave; `SketchLine`, `SketchCurve` e `Pick::SkCurve`
  ainda mostram "#id"; nível "dono" escolhe só faces (falta recurso inteiro para campos de corpo); listas de
  pontos de arquivo antigo (extrapolar) seguem por ponto; realce ao passar o mouse sobre um nível.
