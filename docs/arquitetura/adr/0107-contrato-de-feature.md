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
- **Dobrada de peça grande (pendência 1, resolvida em 22/11/2026)**. Asa de corda `c` (web de `c` × 0,6 `c`,
  dorso de perfil de quatro dígitos extrudado em 2 `c`), aba de 20 mm, t = r = 2, k = 0,44.
  - *Diagnóstico* (função de kernel `Shape::diag_fusao`: `BRepCheck` por subforma, tolerâncias, tipos de face,
    `BOPAlgo_ArgumentAnalyzer` e a fusão com tolerância difusa de 0 a 0,01). As duas partes eram válidas e com
    tolerância pequena (web 5e-6, aba 1,5e-7). A fusão não dava erro: avisava "não foi possível orientar a
    forma" e devolvia vazio (corda 400) ou só a aba (corda 1000, volume da ferramenta). A ferramenta tinha 150 a
    200 faces (planos, revoluções e extrusões) e, na corda 1000, o analisador acusava autointerseção dela. As
    causas, medidas: (a) a OML vem de `section` do suporte pelo plano da web, que é polilinha; as amostras ficam
    nas cordas, até 0,08 mm fora da aresta verdadeira da web na corda 400, e a flecha cresce com o tamanho da
    peça, enquanto as tolerâncias da booleana são absolutas; (b) `cadeia_exata` nunca achava a aresta (as
    amostras das pontas ficam até um passo antes das pontas da aresta), então a varredura seguia a polilinha,
    que o kernel refaz em retas e arcos com quinas entre eles e arcos aceitos a 0,2 % do raio (mais de 1 mm em
    raio grande); a transição arredondada nas quinas se cruza. Resultado: a face da dobra, tangente à face da
    web, nasce décimos de mm fora da borda dela, e a booleana não classifica o contato quase tangente. A
    tolerância difusa só "consertava" com 0,01 mm (tolerâncias de 3e-3 no resultado) e mesmo assim errava o
    volume na corda 1000; costura ou planos exatos não atacavam a causa (as faces planas já saíam planos).
  - *Correção*: `oml_exata` pega a aresta exata da web relimitada (uma aresta, a interseção do suporte com o
    plano), leva as amostras para cima dela (`Shape::curva_projeta`) e a apara entre a primeira e a última
    amostra (`Shape::curva_aparada`); a varredura segue essa curva (10 faces). A faixa planificada usa spline
    pelas amostras nas bordas, e o recorte do recuo (posição por fora e por dentro) a mesma borda
    (`chain_segs` também ajustava arcos a 0,2 % do raio e a fusão perdia 3 % da faixa sem erro). Valia só sem limites de lado (a segunda parte, abaixo, estende aos limites e ao recuo). A fusão é conferida e aplicada
    de uma vez (`apply_aceita`), em vez de duas.
  - *Números* (teste `chapa_b566_dobrada_grande.rs`, dobrada / planificada, status ok nos dois):
    corda 200: 37 180,9 / 37 231,1 mm³, 1,9 s / 0,8 s; corda 400: 128 846,2 / 128 820,1 mm³, 1,9 s / 1,0 s;
    corda 1000: 730 867,8 / 730 610,1 mm³, 2,5 s / 1,7 s; corda 1000 com suporte deslocado de 20 mm:
    770 957,2 / 770 705,3 mm³, 6,3 s / 2,4 s. A diferença (de −50 a +260 mm³, até 0,6 mm³ por mm de corda) é a do
    fator K, θ·(0,5 − k)·t² ≈ 0,38 mm³ por mm de linha de dobra, mais o efeito da curvatura da OML na faixa
    desenvolvida para fora. Caixa: x de 0 a `c` (+0,1 mm na ponta da dobra), z até 24 mm (2 + r + 20) na
    dobrada e 2 mm na planificada; em y, o topo do dorso + r + t na dobrada e + ba + 20 na planificada. Na
    corda 200 a dobrada caiu de 3,3 s para 1,9 s (varredura 0,11 s, fusão 1,3 s); com o suporte deslocado a
    fusão contra a face deslocada leva 5 s.
- **Dobrada de peça grande, segunda parte (22/11/2026): limites, recuo, borda e tempo**.
  - *Limites de lado*: `oml_exata` passa a valer também com `lado1`/`lado2`. As amostras ficam entre os
    limites mais a margem da varredura; a aresta exata da web é aparada nelas e a varredura é cortada nos
    limites como antes (`aba_corta_lados`, que saiu de `aba_pos_ferramenta`). Dois defeitos antigos achados no
    teste e corrigidos: (a) na planificada a faixa ia até a margem (2·(comprimento + 2·(r + t)) + 1 mm além de
    cada limite, 5,6 cm³ a mais na asa de 1 m), porque o corte nos limites só existia na dobrada: a planificada
    apara a OML com margem zero; (b) com recuo, o recorte da faixa da web também passava do limite pela margem e
    deixava um entalhe sem aba: o recorte agora é cortado nos limites como a varredura.
  - *Recuo (material por fora e por dentro)*: a OML recuada é o deslocamento da curva exata no plano da web, no
    kernel (`Shape::curva_deslocada`: `Geom_OffsetCurve` aproximada por B-spline a 1e-7, `GeomConvert_ApproxCurve`).
    O recorte da faixa é uma face plana limitada pela curva recuada, duas retas normais e a curva 0,5 mm para fora
    (`Shape::face_curva`); depois do recorte a varredura segue a aresta que ele deixou na web (lida de novo com
    `oml_exata`, como no caso sem recuo), e não a curva da ferramenta. Só curva aberta de um trecho
    (`recuo_exato`); cadeia de vários trechos (a aba curva por arestas com cadeia de várias arestas) segue a
    polilinha, porque os deslocamentos de trechos não exatamente tangentes não se encontram nas emendas. A aba
    curva por arestas de uma aresta só (arco, spline) com recuo usa o mesmo caminho. O lado do deslocamento é
    conferido no kernel: o comentário do cabeçalho do `Geom_OffsetCurve` fala em V × T, o medido no 8.0.1 é T × V.
  - *Até a borda*: quando a primeira ou a última amostra fica a menos de um passo e meio da ponta da aresta
    exata, a ponta (o canto da web) entra como amostra e a aba vai até ela; se ela cai a menos de meio passo da
    amostra da ponta, toma o lugar dela (duas amostras quase juntas faziam a spline da faixa planificada oscilar,
    face inválida na nervura de 100 × 50, e a tangente da ponta virar ruído, canto sem achar a ponta da aba).
    Ela termina normal à linha de dobra,
    num plano que passa pelo canto: onde a borda da web é oblíqua à OML a dobra sai da web (r + t)·|cos| (0,6 mm
    na asa) na dobrada e (ba + comprimento)·|cos| na planificada, como numa aba de aresta. Com recuo, o mesmo
    plano passa pelo canto e pela ponta da curva recuada.
  - *Tempo do suporte deslocado*: medido por etapa na corda 1000 com o suporte deslocado de 20 mm: relimite
    0,17 s, ferramenta 0,07 s, fusão 4,8 s com a aba parando um passo antes da borda e **2,0 s** com a aba até a
    borda (a dobrada inteira de 5,7 s para 2,9 s). A ponta da varredura no meio da web cortava a face deslocada
    (`Geom_OffsetSurface`, cara de avaliar) e a booleana calculava essas interseções; no canto da web elas somem.
    A fusão restante contra a face deslocada (2,0 s, contra 1,0 s com o dorso puro) não foi atacada: converter a
    face deslocada em B-spline mexeria no suporte, que é de outra bancada, por ganho de no máximo 1 s.
  - *Números* (teste `chapa_b566_dobrada_grande.rs`, dobrada / planificada, status ok nos dois, aba dobrada por mm
    de linha de dobra contra 49,4 teóricos): corda 200: 37 224,9 / 37 282,2 mm³ (49,6); corda 400: 128 918,3 /
    128 899,0 mm³ (49,5); corda 1000: 731 049,1 / 730 798,2 mm³ (49,5), 1,7 s / 1,7 s; com suporte deslocado:
    771 222,8 / 770 974,3 mm³ (49,4), 2,9 s / 2,2 s; lados em x = 250 e 750: 706 171,3 / 706 025,5 mm³ (49,4),
    2,1 s / 0,6 s; material por fora: 726 998,0 / 726 752,7 mm³ (49,4), 2,6 s / 2,3 s; material por dentro:
    722 954,6 / 722 709,5 mm³ (49,4), 2,5 s / 2,3 s; recuo por dentro, lados e suporte deslocado juntos:
    742 336,3 / 742 177,5 mm³ (49,4), 3,8 s / 2,1 s. A ferramenta da aba tem 10 faces em todos os casos (a
    polilinha dava 150 a 200). O volume subiu ~180 mm³ nos casos sem limite em relação à primeira parte: são os
    dois trechos de até um passo que faltavam nas pontas. Caixa conferida em x (a dobra que sai da web nas pontas,
    calculada pela tangente do dorso), em y (topo da OML recuada + r + t, ou + ba + 20 na planificada, ou o topo
    da web fora dos limites) e em z.
- **Qualquer escala, terceira parte (23/11/2026)**. Pedido: "o erro de 1000 mm não pode acontecer em nenhuma
  dimensão, nem superior nem inferior". Teste novo `chapa_b566_escala.rs`: o mesmo caso montado da corda de 10 mm à
  de 10 000 mm (10, 20, 50, 100, 200, 400, 1000, 2000, 5000, 10 000), com t, r e comprimento proporcionais (2, 2 e
  20 mm na corda 200) e com t fixo de 1 e 3 mm (r = t, aba de 20 mm) em peça grande; 45°, 90° e 135°; posição na
  linha, por fora e por dentro; com e sem limites de lado; dobrada e planificada; suporte plano, dorso (spline de
  61 pontos) e dorso deslocado; e a aba curva por arestas numa cadeia reta + arco + reta (placa com canto
  arredondado), com recuo. A conta é feita à parte, sem o motor: a OML teórica são 20 001 pontos levados para
  cima da spline do kernel (`curva_projeta`), a web é a área sob ela, a aba dobrada é a seção varrida pela linha
  recuada pelo teorema de Pappus (`A·L + (Q − recuo·A)·Φ`, `Φ` o giro da tangente; com limite de lado, o corte por
  plano x = constante soma `Q'·Oₓ/Tₓ` em cada ponta) e a planificada é `t·(dev·L' + dev²·Φ/2)`. A caixa é o
  envelope da seção ao longo da OML. 119 casos (238 cálculos) rodam por padrão em ~2,2 min com 7 testes em
  paralelo; a combinação completa fica em `escala_completa` (`#[ignore]`).
  - *Causas achadas pela varredura, todas de constante absoluta ou de critério que depende da amostragem*:
    (a) `curva_deslocada` pedia a aproximação a 1e-7 mm e em C2: o deslocamento de uma B-spline cúbica C2 é só C1
    (a normal depende da derivada), e 1e-7 mm numa curva de 2 m é 5e-11 do tamanho; a partir de ~2 m a
    aproximação desistia, `recuo_exato` devolvia `None` e o recuo voltava à polilinha: −7,6 % no volume da aba
    na corda de 10 m (45°, por fora, com lados) e −1,6 % na corda 200 com o suporte deslocado (a OML da face
    deslocada tem muitos nós). Agora a tolerância é 1e-9 do comprimento (no mínimo 1e-7 mm), a continuidade é a
    que o deslocamento tem (C2, C1 ou C0, nessa ordem), os segmentos são 8 por nó e a reta deslocada é a reta
    transladada. (b) `curva_projeta` deixava o ponto além da ponta da curva onde estava (o
    `GeomAPI_ProjectPointOnCurve` não acha projeção ortogonal): a amostra da OML até 0,05 mm além do canto da web
    (o filtro da web deixa passar) virava zigue-zague na linha de dobra, e na corda de 10 mm a planificada saía
    válida, com status Ok e um quarto do volume. Agora o ponto vai para a ponta, e a projeção olha todos os trechos.
    (c) `oml_exata` recusava a aresta exata quando uma amostra ficava a mais de 0,05 + 0,1·passo dela, abaixo da
    flecha possível da seção (0,1 mm, a deflexão de `wires_to_pts`, agora `FLECHA_SECAO`): na corda de 50 mm a
    linha exata não era achada e a aba parava antes dos cantos (−0,6 % dobrada, −1 % planificada). Agora o limite é
    1,5·`FLECHA_SECAO` + 0,1·passo, o alcance da borda é 1,5·passo (sem os +0,05 mm, que na corda 10 eram um passo
    inteiro) e as amostras repetidas na ponta saem. (d) `chain_segs` (e a cadeia por pontos do `sm_chain_sweep` no
    kernel) aceitava arco com os pontos a até 0,2 % do raio e chamava de reta todo trecho de vértices com desvio
    de direção menor que 0,5° em cada um, o que depende do passo (uma curva suave de raio grande amostrada fino
    virava uma reta só da primeira à última amostra). Agora arco e reta só valem com todos os pontos a 1 µm
    (`TOL_CADEIA`); senão é spline pelos pontos. (e) `cadeia_exata` achava as arestas da cadeia com folga fixa de
    0,15 mm: a flecha da malha cresce com a peça, e em chapa mais fina que 0,4 mm a aresta da outra face também
    cabia. Agora a folga é max(0,15; 4e-4 da diagonal), limitada a 0,4·t. (f) As amostras da OML só ficavam se
    estivessem a menos de 0,05 mm da web: numa OML côncava para a aba (lado do suporte invertido) as cordas da
    seção ficam fora da web, as amostras do meio delas saíam (179 viravam 75, espaçadas de forma irregular) e a
    faixa planificada da nervura de 100 mm (teste B563, lado invertido) saía inválida, com status Ok; o teste
    antigo não conferia a validade. A folga agora é min(1,5·`FLECHA_SECAO`; max(0,05; passo)).
  - *Recuo em cadeia de várias arestas* (pendência 1, resolvida): `curva_deslocada` desloca cada trecho e emenda:
    onde dois trechos não são exatamente tangentes, a fresta do lado de fora do canto vira um arco de raio |d|
    centrado no canto e tangente aos dois (o que o deslocamento exato faz), e o cruzamento do lado de dentro é
    aparado nos dois (`GeomAPI_ExtremaCurveCurve`). `recuo_exato` aceita a cadeia; depois do recorte a varredura
    segue a cadeia das arestas que ele deixou na chapa (`cadeia_exata`), e só se ela não for achada a curva
    deslocada. Na placa de canto arredondado o erro da aba fica abaixo de 3,5e-6 em 10, 1000 e 10 000 mm.
  - *O que não cabe é erro, nunca corpo errado com status Ok*: a faixa planificada inválida (se cruza) é erro com a
    largura (numa cadeia fechada, o laço, a faixa é um anel com costura que a conferência acusa sem defeito: ali
    vale só a conferência da fusão); a fusão planificada é conferida como a dobrada (válida e com pelo menos metade do volume novo); o
    recorte do recuo tem de tirar pelo menos 80 % da faixa (recuo × linha × t), senão "o recuo não cabe na chapa"
    (na corda 10 com t = 3 a 135°, recuo de 14,5 mm numa web de 4 mm, a web sumia e sobrava a aba com status Ok);
    quando a aba falha, o aviso da validação (a dobra não cabe, a faixa se cruza) vai junto na mensagem. A
    validação agora mede a curvatura também perto das pontas (janela de um lado só) e o piso da janela é
    min(0,75 mm; 1/20 da linha).
  - *Medida do volume nos testes*: na dobrada, a integração padrão (`mass`, pontos de Gauss fixos) erra ~6e-6 do
    total nas faces varridas, e com t fixo em peça grande isso passa de 1e-3 do volume da aba; a adaptativa
    (`volume_preciso`) acerta a 1e-7. Na planificada (faces planas aparadas por spline longa e extrusões) é o
    contrário: a adaptativa não converge (informa erro de 1e-16 e o volume muda 3e-5 quando o eps vai de 1e-6 a
    1e-12) e a padrão bate com a conta a 2e-7. O teste usa a adaptativa na dobrada e a padrão na planificada.
  - *Números* (erro do volume da aba contra a conta, pior caso de cada grupo): suporte plano, todas as escalas e
    ângulos: < 5e-11; dorso proporcional, de 10 a 10 000 mm: dobrada ≤ 2,3e-6, planificada ≤ 4,1e-6; dorso
    deslocado proporcional: dobrada ≤ 1,3e-6, planificada ≤ 3,1e-6; t fixo de 1 e 3 mm, de 1000 a 10 000 mm:
    dobrada ≤ 1,2e-7, planificada ≤ 3,0e-5; com limites de lado na planificada: ≤ 1,1e-4 (corda 10); cadeia de três
    arestas com recuo: ≤ 2,4e-5. Tempo por cálculo: 0,1 s (plano), 1 a 3 s (dorso), 1,5 a 9 s (dorso deslocado,
    o maior na corda de 10 m a 135°). Caixa dentro de 6e-4 da corda (o desvio da malha de exibição).
- **Pendente**
  1. Aproximação para superfície de dupla curvatura (estiramento/enrugamento) e recorte de alívio onde a
     OML fecha (hoje só o aviso).
  2. Os limites dos lados, a borda final e os cantos não aparecem na prévia simplificada (só no resultado).
  3. O cache dos dois estados não entra no teto de memória das malhas (`TETO_CACHE_MALHAS`).
  4. A validação não cobre "abas se cruzam" entre abas diferentes (só dentro da mesma OML).
  5. Textos de aviso do motor ainda em português fixo (sem tabela i18n, como os demais erros do motor).
  6. A integração padrão de volume (`mass`, a que a interface mostra) erra ~6e-6 do total em faces varridas
     longas; a adaptativa erra em faces planas aparadas por spline longa. Um volume que escolha por face ficou de
     fora (muda a medida de todas as peças, não só da chapa).
  7. A aba curva por arestas com cadeia de canto vivo (não G1) continua varrida pela polilinha (a varredura de
     seção fixa não contorna o canto); o recuo dela já é exato.

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

### B579, nervura de asa e caverna de fuselagem: aba em perfil curvo fechado e conformação (23/11/2026)

Pedido: a aba por superfície e a aba curva por arestas têm de contornar o perímetro inteiro de um perfil fechado
(aerofólio NACA 2412, oval de fuselagem), para os dois lados da chapa, dobrada e planificada, e a planificação
de aba com curvatura segue a conformação (prensa com borracha, hidroconformação), não a dobradeira. O teste com
mouse de 23/11 (`docs/pesquisa/asa/relatorio-asa.md`) mostrou a aba só num trecho, erro de raio pequeno e "o
suporte não encontra a borda da web".

**Causas, medidas uma a uma** (cada uma só aparecia depois de resolver a anterior):

1. *A seção do suporte chega em pedaços.* A interseção de um suporte fechado com o plano da web é uma ou várias
   polilinhas, quebradas na costura ou entre faces; o código ficava com a maior e a aba saía num trecho.
   `chapa_fechada::junta` une os pedaços pelas pontas e `fechada` reconhece o laço.
2. *Relimitar a web pelo suporte não existe num laço.* O suporte contorna a web inteira (ou coincide com a borda
   dela): `split_side` não divide e a ferramenta de corte do lado "que sai" não tem lado. Em laço a web não é
   relimitada quando a borda dela já é a OML (as 8 amostras caem na aresta, com a folga da flecha da seção); se a
   OML cai dentro da web, a web fica só dentro dela (prisma da OML, `Common`).
3. *"Para fora" pela caixa da peça.* Vale em perfil aberto; no contorno de um furo (caverna) o lado de fora aponta
   para o centro. `chapa_fechada::lado_fora` põe dois pontos no plano da face, um de cada lado da borda, e fica
   com o que cai fora do material.
4. *A faixa planificada é um anel.* Com um laço só, a tira voltava ao começo por um corte de largura zero. A face
   do kernel com dois laços decide quem é o furo pela área integrada, que erra em laço de spline longa (a
   faixa saía cheia); `chapa_fechada::anel` faz o prisma do laço maior menos o do menor, cada laço em duas
   metades (uma aresta que fecha sobre si mesma deixa a costura frágil nas booleanas). A borda deslocada vira
   polígono, não spline: no nariz do aerofólio ela abre em leque (pontos a dezenas de mm ao lado de pontos a 1 mm)
   e a spline interpolada oscila.
5. *A costura da spline fechada.* A tangente do primeiro e do último ponto atravessa o começo (senão a linha de
   fora do começo e a do fim não se encontram e o anel não fecha). A varredura da dobra mantém a tangente de um
   lado só no começo (a seção tem de ficar normal ao primeiro trecho do caminho).
6. *A integração de volume do kernel (`mass`) erra até 13 %* numa face plana limitada por uma spline longa: a web
   da nervura de 1500 mm dava 239 602 mm³ contra 275 962 da malha, e as conferências "a fusão tem volume"
   (`aceita`) rejeitavam fusões boas e aceitavam ruins. Em perfil fechado elas usam o volume da malha
   (`chapa_fechada::volume`, teorema da divergência, flecha 2e-4 da diagonal, uns 2 % abaixo do verdadeiro nas paredes curvas, o que basta para uma conferência com margem de 50 %). O volume que a interface mostra
   continua o do kernel (pendência).
7. *A varredura pela curva exata de um laço saía inválida* (nervura de 1500 mm, G1 aparente por causa do trecho
   único); pela polilinha da aresta da web (cadeia de arcos e splines pelos pontos) sai válida. Na aba por
   superfície a linha de dobra de um laço é a aresta da web (`edge_at`), e o suporte só a escolhe: a polilinha da
   seção do kernel tem a flecha da tesselação.
8. *Canto vivo.* Uma spline fechada que se fecha num bordo de fuga agudo (canto de 14°, giro de 164°) não une: o
   erro diz o canto, o ângulo, o ponto e o que fazer (`Analise::dica_falha`), em vez de "a união saiu inválida".

*Armadilha do comando de aba curva (não era defeito do kernel, mas custou muito teste).* O comando acha a aresta
pelo elemento do meio da lista (`edge[len/2]`) e leva a cadeia inteira pelo quanto esse ponto andou até o ponto
médio que o kernel dá à aresta. Numa aresta fechada o ponto do meio da polilinha não é o ponto médio do kernel, e
a aba saía deslocada (4 a 12 mm, exatamente a coordenada do primeiro ponto): o teste monta a lista com o ponto
médio do kernel no meio. O app grava a aresta pelo clique, que é o ponto médio dela.

**Testes antigos.** O aviso de conformação aparece como `Status::Warning` em abas de curva forte dos testes do B529 a B566 (arco de raio 72 mm com aba de 20 mm, o dorso de asa em escala); a conferência de geometria deles passou a aceitar `Ok` ou aviso de conformação (`ok_ou_conformacao` em `tests/common/mod.rs`), sem tocar nos números de volume e caixa. A bandeja curva de exemplo ganhou aba de 10 mm (a de 18 mm acusava contração).

**Caminhos comparados** (aba de 90°, mesmo contorno; tempo do recálculo completo; erro = (medido − teoria) / volume
da aba; volume pela malha, teoria de Pappus):

| caso | caminho | dobrada | planificada |
|---|---|---|---|
| oval 1200 × 900, t 1,2, aba 12, cima | superfície (suporte = extrusão) | −0,28 %, 19 s | −0,19 %, 13 s |
| idem, baixo | superfície | −0,14 %, 19 s | −0,19 %, 13 s |
| idem, cima | aresta com ângulo | −0,28 %, 17 s | −0,20 %, 12 s |
| idem, baixo | aresta com ângulo | −0,15 %, 17 s | −0,20 %, 12 s |
| fuselagem de dupla curvatura (loft 1200 × 900 → 800 × 600, estação 1500), cima | superfície (suporte = loft) | 0,00 %, 111 s | +0,53 %, 11 s |
| NACA 2412 corda 2000, t 1, aba 8, cima | aresta | **só com `--ignored`** (fusão de minutos) | −0,26 %, 21 s |
| idem | superfície | idem | −0,26 %, 19 s |
| NACA 2412 corda 2000, t 2, aba 25, baixo | aresta / superfície | idem | −0,09 %, 8 s / 9 s |
| NACA 2412 corda 1500, t 1,5, aba 15, cima | aresta / superfície | idem | −0,15 %, 8 s / 9 s |
| asa enflechada (loft 2000 → 1500, envergadura 4000, recuo 600), estação 1500 | superfície (suporte = loft) | idem | −0,48 %, 19 s |

Os dois caminhos de aba dão o mesmo resultado (diferença de décimos de por cento); a aba por superfície é o padrão
(a escolha do suporte acha a aresta e o laço sozinha, e é a mais rápida na dobrada). O planificado é mais exato que
o dobrado porque a faixa é um anel de superfície plana. A varredura exata pela aresta fechada, a terceira via
pedida, foi descartada para laço (causa 7). A aba dobrada do aerofólio (spline de 139 pontos com a ponta de
bordo de fuga) varre em 8 s, mas a fusão com a web leva de 100 a 500 s e às vezes é recusada: a face lateral da web
é uma superfície B-spline e a parede entra nela 0,05 mm, contato quase tangente que a booleana do kernel resolve
mal; o teste dela (`nervura_naca_dobrada_aresta`) fica `#[ignore]`. Na oval (spline de 96 pontos) a mesma fusão leva
17 s.

**Conformação (hidroconformação, prensa com borracha)** em `chapa_hidro.rs`. A aba reta continua na dobradeira
(desenvolvimento `ba = ângulo·(r + k·t)`). Quando a linha de dobra é curva a faixa plana é a curva deslocada, para
fora, por `D = ba + comprimento` (a linha neutra conserva o comprimento e cada fibra normal à linha de dobra
mantém o dela: o desenvolvimento geodésico da aba), e a peça dobrada tem a borda livre a `x_f = (r + k·t)·sen θ +
comprimento·cos θ` da linha de dobra. Uma curva paralela a uma de curvatura κ (positiva no lado convexo) tem o
comprimento multiplicado por `1 + κ·d`; logo a deformação circunferencial da borda é

```
ε = (1 + κ·x_f) / (1 + κ·D) − 1  ≈  −κ·(D − x_f)        (aba reta de altura h num arco de raio R: ε ≈ −h/R)
t' = t / (1 + ε)                                           (constância de volume, comprimento radial fixo)
```

Convexo: `ε < 0`, flange de contração (a borda se comprime, a chapa espessa, enruga). Côncavo: `ε > 0`, flange de
estiramento (a borda estica, a chapa afina, trinca). Risco em três faixas (ok abaixo de 0,5, atenção até 1,
falha de 1 em diante): na compressão, `ρ = |ε| / (ψ·ε_cr)` com `ε_cr = 0,425·π²/(12·(1 − ν²))·(t/h)²` (flambagem
local da borda livre, placa com uma borda apoiada e a outra livre) e `ψ = 20` (apoio da borracha ou da pressão);
na tração, `ρ = ε / ε_trinca` com `ε_trinca = 15 %` (alumínio recozido; editável em Chapa › Parâmetros ›
"Limite de conformação da borda"). Cantos vivos (giro de mais de 35° num vértice) são tratados à parte: a
curvatura não vale ali e a aba pede alívio de canto. O resultado é um `Status::Warning` com o trecho, o valor, a
espessura final e o comprimento de aba que ainda fica abaixo do limite; nunca um erro. `ψ` e `ε_trinca` são
hipóteses de trabalho a calibrar (nenhuma fonte pública foi consultada; ver `procedencia.md`).

**Ficou de fora.** (a) O mapa de deformação "tipo elementos finitos" com cores no 3D, a legenda e a opção
"Mostrar deformação" (pedido de 23/11, depois do núcleo): o cálculo por ponto da linha de dobra
(`Analise::pontos`: κ, ε, t'/t, regime, faixa de risco) está pronto e testado; falta a ponte com o renderer (cor por
vértice da malha da aba). (b) A simulação completa de conformação (FEA) vai para depois do lançamento. (c) O bordo
de fuga agudo: pede alívio de canto ou aba em dois trechos (o erro diz como); o alívio automático não foi feito.
(d) Aba em perfil fechado com limites de lado e canto: não se aplicam (a cadeia é um laço); o recuo (posição do
material) de um laço segue a polilinha. (e) O volume que a interface mostra (`mass`) segue com o erro da
integração em face de spline longa. (f) A aba dobrada da nervura de aerofólio (spline longa na web) pela fusão lenta e às vezes recusada, descrita acima: o caminho natural é uma fusão sem contato quase tangente (a seção encostar na face da web em vez de entrar 0,05 mm) ou converter a face lateral da web em B-spline de menos nós, e fica para a bancada de kernel. (g) Aba por superfície em perfil fechado com loft de aerofólio (asa enflechada) só testada planificada; a dobrada do mesmo caso cai na mesma fusão lenta.
