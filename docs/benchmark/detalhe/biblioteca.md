# Benchmark de detalhe: Biblioteca de normas

> Texto próprio. Compara comando a comando a bancada com o que os softwares de modelagem CAD avançados
> oferecem em campo e em interação, e o software livre de fixadores do FreeCAD. Complementa (sem repetir)
> [biblioteca-de-normas.md](../biblioteca-de-normas.md) e as pesquisas de prendedores em `docs/pesquisa/`.
> Estado lido do código em 2026-11-04. Itens de backlog citados: B28, B29, B42, B51, B52, B58, B85, B90, B98.

## Resumo: as 5 maiores lacunas de experiência

1. **Nenhuma prévia 3D em toda a bancada.** As duas janelas (fixadores e engrenagens) só mostram
   campos e uma grade numérica. O usuário aperta o botão e descobre o resultado depois, na montagem. Os
   outros mostram o fixador ou a engrenagem se atualizando no desenho enquanto se digita, e mostram os
   furos candidatos antes de confirmar.
2. **Não existe "inserir um item normalizado" avulso.** O único caminho para um parafuso é "Preencher
   furos", que é em lote e automático. Falta navegar o catálogo (norma, categoria, tamanho), ver a ficha
   com imagem e a tabela de tamanhos, e arrastar o item até um furo, aresta ou ponto. O catálogo existe
   em dados (6 famílias em `biblioteca/fixadores/`), mas não tem tela.
3. **Sem manipulador 3D.** Comprimento, largura da engrenagem, trechos do eixo e posição do rasgo só se
   mudam digitando. Um CAD avançado deixa puxar uma alça e para o comprimento nos valores comprados de
   verdade (a série da norma), mostrando a cota ao lado.
4. **Campos sem explicação visual.** "Módulo", "x", "t2", "Folga fina/normal/folgada" aparecem como texto
   seco. Falta o esquema com as letras (o que é dk, k, s, b, d, df) e uma frase de efeito. Poucos campos têm
   dica, e a maior parte está nos botões, não nos campos.
5. **Fluxo sem retorno e sem edição depois.** Fixador inserido não pode ter o tamanho trocado pela
   própria pilha; engrenagem inserida não reabre a janela com os valores usados; eixo e chaveta saem como
   arquivos soltos, sem ligação com o furo do cubo. Os outros reabrem o assistente a partir do item na
   árvore e atualizam tudo junto (comprimento acompanha a pilha, porca e arruela acompanham o tamanho).

## Tabela checklist

Legenda: ok, parcial, falta. Ícones avaliados pelo papel e pelo nome do arquivo em `assets/icons/`
(`asm_fixadores.svg`, `asm_engrenagens.svg`); falta revisão visual do desenho do ícone.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Preencher furos com fixadores | parcial | falta | falta | parcial | parcial | parcial | parcial | Janela com família, folga, arruela e porca; sem ver quais furos entram. |
| Gerar engrenagens (parâmetros) | parcial | falta | falta | parcial | parcial | parcial | parcial | Campos corretos e cálculo ao vivo, mas sem imagem do dente. |
| Cálculo ao vivo da engrenagem | ok | parcial | falta | falta | falta | parcial | ok | Grade de d, db, da, df, passo e centros; ponto forte, só numérico. |
| Inserir par na montagem | parcial | falta | falta | falta | ok | parcial | parcial | Tooltip explica bem; grava em `engrenagens/` e não reabre depois. |
| Salvar pinhão ou coroa avulsos | parcial | falta | falta | falta | ok | falta | parcial | Dois botões em linha; o nome do arquivo não é pedido. |
| Eixo escalonado (grade de trechos) | parcial | falta | falta | falta | parcial | falta | parcial | Oito colunas curtas (Ø, L, Chanfro...); sem desenho lateral. |
| Chaveta DIN 6885 A | parcial | falta | falta | falta | ok | parcial | parcial | Escolhida pelo diâmetro do trecho; comprimento automático sem aviso claro. |
| Rasgo de chaveta no cubo | parcial | falta | falta | falta | ok | ok | parcial | Caixa de marcação com tooltip e resumo b×h e t2. |
| Inserir item normalizado avulso | falta | falta | falta | falta | falta | falta | falta | Não existe. Só o lote do primeiro comando. |
| Trocar tamanho ou família de um fixador | falta | falta | falta | falta | falta | falta | falta | Não existe; vai para B98. |

Contagem (calculada sobre as 70 células): ok 7, parcial 25, falta 38.

## Onde está no código

- Barra: `crates/fenkra-app/src/toolbar.rs:237` (Fixadores) e `:238` (Engrenagens), ambos `One(...)` na
  família de montagem, sem submenu nem atalho. Tooltip longo em texto corrido; sem figura nem GIF.
- Tradução PT/EN: `crates/fenkra-app/src/i18n.rs:2505` (Fixadores), `:2533` (Engrenagens), `:2587`
  (Salvar eixo), `:2591` (Rasgo no cubo).
- Janela de fixadores: `crates/fenkra-app/src/asm_ui.rs:2720` (`fasteners_window`). Campos: família do
  parafuso, folga (Fina, Normal, Folgada, linha de ajuda com M6), arruela, porca, "só nos selecionados".
  Estado em `FastenersUi` (`:109`). Motor em `crates/fenkra-montagem/src/fixadores.rs:106`
  (`preencher_furos`); o comprimento vem da pega, arruelas, porca e 2 passos, arredondado na série
  (`:256-271`).
- Janela de engrenagens: `asm_ui.rs:2860` (`gears_window`), com a seção "Eixo e chaveta" na mesma janela
  (`:2999`). Estado em `GearsUi` (`:118`). Nenhum desenho no viewport enquanto a janela está aberta.
- Dados: `crates/fenkra-solidos/src/biblioteca.rs` (famílias, tamanhos, `TipoFixador` com 4 tipos,
  `chaveta_para_eixo` em `:323`), `fixadores.rs:26`, `engrenagens.rs`, `eixos.rs`.
- Peças geradas gravadas ao lado da montagem; exige montagem salva (aviso em `asm_ui.rs`, dentro dos
  blocos `apply` e `inserir`).

## O que os outros têm e o Fenkra não, por comando

### 1. Preencher furos com fixadores

**Nos outros.** O assistente equivalente reconhece furos de assistente, furos simples e cortes
cilíndricos; tem "preencher todos" e "adicionar nos selecionados"; mostra os furos reconhecidos antes de
confirmar; coloca porca e arruela automaticamente concêntricas e coincidentes; só oferece pilha inferior
em furo passante. O comprimento padrão é o próximo valor da série acima da pega em furo passante e o
próximo abaixo em furo cego; se o furo é mais fundo que o maior parafuso, usa o maior. Há opções de
configuração: "ajustar o tamanho ao diâmetro do furo" (marcado com um símbolo na árvore, o fixador
acompanha o furo se ele mudar), "atualizar o comprimento quando a pilha muda", "garantir engajamento
mínimo de rosca" (múltiplo do diâmetro, ou rosca além da porca) e "trocar arruela e porca junto com o
tamanho". Em outro CAD, a inserção no furo aceita aresta circular, face cônica ou cilíndrica, ponto de
sketch ou outro fixador como referência, com tamanho adaptativo. Em um terceiro, o item é solto do
catálogo sobre o furo escolhendo o ponto de encaixe, com botão de tamanho automático e pré-seleção de
uma face para preencher todos os furos dela. No software livre, a ferramenta de fixadores trata arestas
circulares como furo passante ou roscado conforme o botão ativo, mantém o fixador ligado à aresta e tem
"deslocamento" para deixar o parafuso suspenso acima do furo.

**No Fenkra.** Uma janela de 400 px com texto de abertura, combos de família, três botões de folga e
dois checks (`asm_ui.rs:2735-2770`). Não mostra no 3D quais furos entram, não permite excluir um, não
mostra o comprimento escolhido antes de gravar, e a única edição depois é desfazer. Os avisos aparecem
em lista no rodapé.

**Como deveria ser.**
- Suporte: furos da montagem (cilíndricos e roscados), arestas circulares selecionadas, face inteira
  (todos os furos dela), ponto publicado (B58).
- Modo: "Automático" (como hoje) e "Escolher furo a furo". O modo escolhe-se por dois botões no topo.
- Parâmetros: família do parafuso, folga com o diâmetro do exemplo ao vivo (6,4 / 6,6 / 7,0 para o
  tamanho do furo sob o cursor), arruela, porca, comprimento (Automático pela série ou fixo).
- Avançado (recolhido): engajamento mínimo de rosca em múltiplos de d, rosca além da porca, seguir o
  diâmetro do furo se mudar, tolerância de reconhecimento do furo (hoje 0,05 mm fixo).
- No 3D: ao abrir a janela, cada pilha candidata ganha um fantasma translúcido do parafuso com o rótulo
  do tamanho (M6x35). Passar o mouse sobre o rótulo realça os furos da pilha; clique tira da lista. Pilha
  sem tamanho compatível fica em âmbar com o motivo ("Ø 4,9 sem tamanho"). A alça na ponta do parafuso
  puxa o comprimento e para nos valores da série, com a cota ao lado.
- Roteiro do GIF do tooltip (8 s): (1) montagem com duas chapas e quatro furos; (2) clique em Fixadores;
  (3) quatro fantasmas aparecem com "M6x25"; (4) clique em um fantasma o desliga; (5) botão Preencher; (6)
  três parafusos com porca e arruela entram, a tela mostra "3 fixadores: 3× M6x25".

### 2. Inserir item normalizado avulso (não existe)

**Nos outros.** Navegação por árvore de norma e categoria, busca por nome ou descrição, filtros salvos,
favoritos e histórico. Uma ficha da família com imagem de prévia e "vista de tabela" com uma linha por
tamanho. Dois modos de colocação: automático (arrasta e solta, acha o furo) e manual (abre a ficha),
alternáveis segurando uma tecla. "Como padrão" ou "como personalizado". Clique direito no item inserido
oferece "mudar tamanho", e há "substituir todos" para trocar todas as instâncias. No software livre há
diálogo de busca por nome ou descrição e uma calculadora de parafuso que fica aberta enquanto se trabalha.

**No Fenkra.** `biblioteca::familias()` já lista 6 famílias, mas só o preenchimento em lote as usa. Para
pôr um único parafuso é preciso modelar um furo e rodar o lote com "só nos selecionados".

**Como deveria ser.**
- Suporte: painel lateral "Biblioteca" na bancada de montagem e peça; busca no topo; árvore Norma >
  Tipo; chips de filtro (Métrico, Polegada, Tipo de cabeça, Material).
- Modo: arrastar para o viewport (encaixe em furo, aresta circular, ponto) ou clicar duas vezes (abre a
  ficha). Tecla Alt alterna o modo, como nos outros.
- Parâmetros: designação (tamanho), comprimento (lista da série, com o valor sugerido destacado),
  classe de resistência, acabamento.
- Avançado: "como padrão" (peça compartilhada) ou "personalizado" (cópia local); rótulo da norma e da
  revisão; quantidade.
- No 3D: o item vira fantasma preso ao cursor e salta para o furo quando o diâmetro casa; Tab inverte o
  sentido; a cota do comprimento aparece com a alça.
- GIF do tooltip (7 s): painel aberto, digitar "M8"; arrastar o parafuso; salta para o furo e se
  alinha; Tab inverte; clique confirma.

### 3. Gerar engrenagens e inserir o par

**Nos outros.** Há uma aba de projeto e uma de cálculo. O projeto tem tipo de engrenagem, módulo, dentes,
largura, ângulo e uma prévia gráfica do par no desenho que se atualiza a cada mudança; há um botão que
abre um esquema com as dimensões de módulo e número de dentes. A aba de cálculo tem escolha do método
de resistência entre várias normas (inclusive ISO 6336 e AGMA), tipo de cálculo (verificar resistência,
dimensionar material, dimensionar geometria) e resultados, mais relatório em HTML. Uma engrenagem com
mais dentes que a outra pode ser tratada como interna. Cada engrenagem pode ser componente ou feature.

**No Fenkra.** `gears_window` tem tipo, módulo, z1, z2, ângulo de pressão, hélice, largura, folga,
furo, rasgo e correção x; o cálculo mostra sete grandezas por engrenagem, centros e razão. Falta a
imagem; "x" e "Folga entre dentes (par)" não explicam efeito; a janela mistura engrenagem e eixo em uma
rolagem única de 20+ campos.

**Como deveria ser.**
- Suporte: montagem (par) ou peça avulsa; ponto ou eixo existente para posicionar o centro.
- Modo: Dimensionar por centro-a-centro e razão (digita a distância e a razão; o módulo e os dentes saem
  sugeridos) ou Projetar por módulo e dentes (como hoje). Dois botões de modo no topo.
- Parâmetros: módulo, z1, z2, ângulo de pressão, largura, furo; hélice só aparece para helicoidal.
- Avançado: correção x com o valor mínimo para evitar rebaixo (z < 17 para 20°), folga, diâmetro do cubo,
  alívio.
- No 3D: as duas engrenagens aparecem translúcidas na posição final, dentes encaixados; manipulador na
  face da engrenagem muda a largura; círculos primitivos desenhados como linhas finas tracejadas, com
  d e a anotados.
- Abas "Projeto" e "Verificar" para separar o que define a geometria do que só confere.
- GIF do tooltip (9 s): digitar z1=20, z2=50; as duas rodas surgem no viewport; arrastar a alça da
  largura; ver o cálculo mudar; clicar em Inserir par; as rodas giram juntas.

### 4. Eixo escalonado e chaveta

**Nos outros.** O gerador de eixo tem recursos de borda (chanfro, arredondado, rosca, rasgo de ponta) e
de seção (rasgo, anel elástico, furo transversal, ranhura, abertura de chave), com a regra de um recurso
por borda e quantos quiser no meio da seção. Dá para editar por alças 3D no viewport. A ranhura do anel
tem diálogo próprio com posição medida da borda esquerda ou direita e pega o anel da biblioteca. Vários
rasgos podem ser criados como um só recurso, com rotação controlada. A saída do cálculo de engrenagem
alimenta as cargas do eixo.

**No Fenkra.** Grade de 8 colunas (Ø, L, Chanfro, Rasgo, L rasgo, Início, Chaveta) em `asm_ui.rs:2999`
sem desenho; um só rasgo por trecho, sempre em +X; sem anel, rosca nem furo transversal; sem
valores de referência (o que é "Início"?).

**Como deveria ser.**
- Suporte: eixo avulso (peça) com perfil lateral 2D ao lado da grade; selecionar um trecho no desenho o
  destaca na grade.
- Modo: Tabela (como hoje) e Gráfico (alças). A seleção de uma seção abre "Adicionar recurso".
- Parâmetros: Ø, L por trecho; chanfro nas duas pontas; rasgo com comprimento e posição a partir da
  borda esquerda ou direita (escolha).
- Avançado: rotação angular do rasgo, mais de um rasgo, anel elástico pela norma, rosca, furo de centro,
  tolerância de ajuste (h6, k6) para o diâmetro.
- No 3D: eixo em fantasma com alças nas arestas de cada trecho; o rasgo aparece em corte; chaveta
  aparece dentro do rasgo.
- GIF do tooltip (8 s): dois trechos; selecionar o segundo; Adicionar recurso > Rasgo; arrastar a alça
  de posição; a chaveta aparece e a legenda mostra "8×7, t1 4,0".

### 5. Chaveta e rasgo no cubo

**Nos outros.** O rasgo do eixo e o do cubo se pareiam com a mesma tabela e a chaveta sai da biblioteca
como peça padrão, que entra restrita ao rasgo.

**No Fenkra.** Rasgo no cubo é uma caixa de marcação dentro da engrenagem (`asm_ui.rs:2918`), a chaveta é
um arquivo salvo à parte e não entra na montagem. A tabela cobre Ø6 a Ø85.

**Como deveria ser.** Comando "Chaveta" que seleciona o eixo e o cubo, acha os dois rasgos, pergunta o
comprimento (série) e insere a chaveta restrita; aviso quando faltar t2 ou os rasgos não casarem. O 3D
mostra o fantasma da chaveta na posição.

## Lições transversais

- **Mesmo vocabulário de tooltip.** Padrão: uma frase do que faz, uma do que muda no modelo, a tecla de
  atalho, e o GIF de 5 a 10 s. Hoje o texto é um parágrafo único que corta no meio quando a barra é
  estreita.
- **Dica por campo com esquema.** Um pequeno desenho (com as letras) aparece ao passar o mouse sobre o
  rótulo: dk, k, s, b, d, da, df, t1, t2, módulo, x.
- **Prévia primeiro, gravar depois.** Qualquer comando que cria peça precisa mostrar o resultado antes de
  gravar arquivos ao lado da montagem.
- **Reabrir a partir da árvore.** Cada fixador, engrenagem e eixo guarda os parâmetros e reabre a
  mesma janela.
- **Idempotência visível.** O fixador já diz que rodar de novo não duplica; replicar isso nos outros
  (engrenagem inserida duas vezes pergunta antes).

## Relação com o backlog

B98 (assistente de prendedores) cobre a tabela de troca e a pilha editável, mas não a prévia nem a
inserção avulsa; B52 cobre perfil recalculado, filete de raiz, cônica, ranhuras e roscas no eixo, mas não
a interface gráfica; B51 amplia as famílias, o que torna o navegador de catálogo mais urgente porque a
lista crescerá. Os itens novos de `biblioteca.json` fecham esse espaço sem repetir os existentes.
