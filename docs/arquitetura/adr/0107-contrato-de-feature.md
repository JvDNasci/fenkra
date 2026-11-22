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
