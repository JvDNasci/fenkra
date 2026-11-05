# Benchmark de detalhe — Desenho técnico

> Texto próprio, a partir de ajudas públicas de softwares de modelagem CAD avançados e do FreeCAD (bancada TechDraw). Este documento não repete o catálogo de módulos de [desenho-tecnico.md](../desenho-tecnico.md) nem as fichas de [pesquisa/desenho.md](../../pesquisa/desenho.md): aqui a pergunta é outra. Não é "o comando existe?", é "o comando explica o que faz, mostra o resultado antes do clique e deixa mexer direto na folha?". Na folha, a "manipulação 3D" vira arrastar vista, cota, balão e linha de corte.

## Resumo: as 5 maiores lacunas de experiência

1. **Nenhuma ferramenta mostra o resultado antes do clique.** Ao posicionar vista, tabela ou cota, o Fenkra desenha só uma dica de texto no canto, o marcador de snap e uma linha elástica do último clique (`dwg_ui.rs:655-669`). Os outros mostram a vista inteira (ou o contorno dela) presa ao cursor, com a escala já aplicada, e a cota viva seguindo o mouse antes de fixar. Quem erra só descobre depois de criar e apagar.
2. **As opções ficam longe da ferramenta e só aparecem depois.** A direção da vista única, a origem e o estilo ficam no painel "Desenho" (`dwg_ui.rs:1686-1700`), e as opções da vista (escala, linhas ocultas, rótulo, renderização) só existem depois de criada (`dwg_ui.rs:1388-1405`). Nos outros, ao ativar a ferramenta abre um painel de opções com escala, estilo, rótulo e alinhamento, editáveis enquanto a vista ainda está presa ao cursor.
3. **Cortes e detalhes não têm alças.** A linha de corte nasce com dois cliques, não tem pontas arrastáveis, e inverter o lado "exige apagar e traçar no sentido oposto" (texto do próprio painel, `dwg_ui.rs:1429`). O detalhe tem o raio só num campo numérico. Nos outros, as pontas, setas e o círculo são puxados na folha, o lado se inverte com um clique e o corte pode ter vários segmentos.
4. **Tooltip pobre e sem dica por campo.** Cada comando tem só "breve" e "avançado" em texto (`toolbar.rs:110`, `303-350`); não há imagem nem GIF, e quase todos os campos do painel não têm dica ao passar o mouse (só dois botões têm `on_hover_text`, `dwg_ui.rs:1692` e `1747`). Os outros trazem uma linha de ajuda por campo, símbolos clicáveis para o texto da cota e mensagem de estado que diz o próximo passo.
5. **Cota sem edição em contexto.** O Fenkra tem uma ferramenta por tipo (horizontal, vertical, alinhada, raio, diâmetro, ângulo) e edita texto, tolerância e setas só no painel lateral (`dwg_ui.rs:1493-1540`). Os outros têm uma ferramenta que adivinha o tipo pelo que foi clicado, deixam trocar o tipo na hora, abrem a edição por duplo clique com caixa flutuante (valor, precisão, tolerância tipada, símbolos) e ficam em modo contínuo até Esc.

Contagem do checklist abaixo (33 ok, 151 parcial, 54 falta em 238 células): leitura honesta é que o Fenkra cobre bem o "o quê" e mal o "como se usa".

## Como ler a tabela

- **ok** = comparável ao que os outros têm. Também vale para o que não se aplica ao comando (por exemplo, "manipulação direta" de um comando de arquivo).
- **parcial** = existe, mas incompleto ou longe de onde se usa.
- **falta** = não existe.
- Os ícones foram avaliados pelo nome e pela organização na barra (`assets/icons/des_*.svg`, 34 arquivos, um por comando), sem captura visual: o que é "parcial" aqui merece confirmação com a pessoa que testou.

## Tabela checklist

| Comando | Intuitivo | Prévia | Manipulação direta | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Novo desenho | parcial | falta | ok | ok | parcial | falta | falta | Só folha A3 vazia; sem assistente de formato, norma, diedro e origem. |
| Abrir (.fkd) | ok | ok | ok | ok | parcial | ok | ok | Falta lista de recentes com miniatura da folha. |
| Salvar | ok | ok | ok | ok | ok | ok | ok | Ctrl+S funciona; sem aviso de vistas desatualizadas ao salvar. |
| Exportar PDF | ok | falta | ok | ok | parcial | falta | parcial | Sem prévia de impressão nem opções (folhas, cor/preto e branco, espessuras). |
| Exportar SVG | ok | falta | ok | ok | parcial | falta | parcial | Só a folha atual; sem opções. |
| Exportar DXF | ok | falta | ok | ok | parcial | falta | parcial | R12 fixo; camadas não editáveis; sem escolha de versão. |
| Nova folha | parcial | falta | ok | parcial | parcial | falta | parcial | Herda o formato da atual; sem diálogo de formato, escala geral e modelo. |
| 3 vistas | parcial | falta | parcial | parcial | parcial | falta | parcial | Clique único posiciona o conjunto; sem fantasma e sem escolha de distância entre vistas. |
| Vista única | parcial | falta | parcial | parcial | parcial | parcial | parcial | Direção escolhida antes, no painel do desenho (longe da ferramenta). |
| Vista projetada | parcial | falta | parcial | parcial | parcial | falta | parcial | Exige selecionar a mãe antes; erro vem só como aviso depois do clique. |
| Vista isométrica | parcial | falta | parcial | parcial | parcial | falta | parcial | Sem escolha do tipo (isométrica, dimétrica, trimétrica). |
| Vista em corte | parcial | parcial | falta | falta | parcial | parcial | parcial | Linha de corte só com 2 cliques; inverter lado exige apagar e refazer; nome igual ao da lista de corte. |
| Vista de detalhe | parcial | falta | falta | parcial | parcial | parcial | parcial | Só círculo; raio sem alça; ampliação fixa em 2x na criação. |
| Vista planificada | parcial | falta | parcial | parcial | parcial | falta | parcial | Sem opção de linhas de dobra e eixo de centro. |
| Esquema cinemático | parcial | falta | parcial | parcial | parcial | falta | parcial | Nome técnico demais para quem não usa mecanismos. |
| Vista pela câmera | ok | falta | parcial | ok | ok | ok | ok | Bom: cobre o que falta de vistas nomeadas. |
| Cota horizontal | parcial | parcial | parcial | parcial | parcial | parcial | parcial | Sem prévia da cota; tipo precisa ser escolhido antes. |
| Cota vertical | parcial | parcial | parcial | parcial | parcial | parcial | parcial | Idem horizontal. |
| Cota alinhada | parcial | parcial | parcial | parcial | parcial | parcial | parcial | Idem horizontal. |
| Cota de raio | parcial | falta | parcial | parcial | parcial | parcial | parcial | Sem opção de cota com quebra (raio grande). |
| Cota de diâmetro | parcial | falta | parcial | parcial | parcial | parcial | parcial | Sem escolha de linear versus radial. |
| Cota angular | parcial | falta | parcial | parcial | parcial | parcial | parcial | Sem escolha de quadrante (interno, externo, complementar). |
| Nota de texto | parcial | falta | parcial | ok | parcial | parcial | parcial | Sem paleta de símbolos (Ø, ±, °) nem negrito/sobrescrito. |
| Balão | parcial | falta | parcial | ok | parcial | falta | parcial | Forma, texto auxiliar e alinhamento ausentes. |
| Balões automáticos | ok | falta | falta | parcial | parcial | falta | parcial | Sem escolha de lado, distância e vistas; sem renumerar por posição. |
| Rugosidade | parcial | falta | parcial | parcial | parcial | falta | parcial | Campo único de texto; sem campos por posição do símbolo. |
| Solda | parcial | falta | parcial | parcial | parcial | falta | parcial | Campo único de texto; sem lados, contorno e campo de cauda. |
| Marca de centro | parcial | falta | falta | parcial | parcial | falta | parcial | Sem modo linear, circular e eixo entre dois furos. |
| GD&T | parcial | falta | parcial | parcial | parcial | falta | parcial | Quadro sem construtor por campos e sem biblioteca de símbolos clicável. |
| Referência (datum) | parcial | falta | parcial | parcial | parcial | parcial | parcial | Letra livre; sem sequência automática. |
| Lista de materiais | parcial | falta | parcial | parcial | parcial | parcial | parcial | Colunas fixas; sem ordenar, escolher colunas nem renumerar. |
| Tabela de furos | parcial | falta | parcial | parcial | parcial | falta | parcial | Sem etiquetas de furo nas vistas ligadas à tabela. |
| Tabela de dobras | parcial | falta | parcial | parcial | parcial | falta | parcial | Sem colunas configuráveis. |
| Lista de corte | parcial | falta | parcial | falta | parcial | falta | parcial | Nome 'Corte' repetido com a vista em corte. |


## O que o Fenkra tem hoje na bancada (referência com arquivo:linha)

- **Barra** em cinco grupos: Arquivo, Vistas, Cotas, Anotações e Tabelas (`toolbar.rs:302-350`). Cada item é `it(ícone, nome, breve, avançado, ação)` (`toolbar.rs:110`). O tooltip tem uma linha de título e um parágrafo.
- **Famílias** na barra: exportar (PDF, SVG, DXF), cotas lineares (H, V, alinhada) e balão (balão e automático).
- **Ferramenta por cliques**: `command` arma a ferramenta (`dwg_ui.rs:220-301`) e `click` cria o objeto (`dwg_ui.rs:856`). O texto-guia do passo vem de `hint` (`dwg_ui.rs:193-217`), desenhado no canto da folha com "Esc cancela" (`dwg_ui.rs:655`).
- **Mensagens de pré-condição** aparecem como aviso depois do clique no botão: "Selecione antes a vista de origem", "A folha não tem vistas para cotar", "Escolha a origem" (`dwg_ui.rs:271-290`).
- **Snap** circular laranja nos vértices e centros de arco das vistas, raio de 10 px (`dwg_ui.rs:625-650`).
- **Mover**: 1 clique realça, 2 cliques editam, arrastar move; alça de texto da cota (`dwg_ui.rs:672-760`). Roda dá zoom no cursor; botão do meio ou direito arrasta a folha (`dwg_ui.rs:505-515`).
- **Menu de contexto** por objeto: editar, renomear (F2), estilo de renderização, copiar e colar estilo, setas, ponta, texto no lugar automático, apagar (`dwg_ui.rs:1075-1210`).
- **Painel** da vista, da cota, da nota e da tabela, e do desenho (`dwg_ui.rs:1375-1757`).
- **Teclas**: Esc cancela, Del apaga, F2 renomeia, F5 recarrega, Ctrl+S salva (`dwg_ui.rs:422-459`).

## Por comando: o que os outros têm e o Fenkra não

### Vistas (base, projetada, 3 vistas, isométrica)

**O que os outros têm em detalhe**
- A vista nasce presa ao cursor, com o contorno real da peça na escala escolhida. A projetada fica limitada às direções válidas (acima/abaixo/lados) e o fantasma mostra em qual posição vai cair antes de soltar.
- Um painel com o modelo de origem, o estado de exibição, o estilo (linhas ocultas visíveis, removidas ou sombreada), a escala (preset, valor livre ou "da vista mãe"), o rótulo (liga/desliga e texto) e opções de herança: alinhar à vista base, herdar orientação, herdar cortes, mostrar tangentes (e com encurtamento para diferenciar), ocultar peças pequenas por tamanho.
- Casas de "orientação": vista nomeada (frente, topo...), vista da câmera salva e, na base, os botões de rotação de 90 graus.
- Quando falta algo, o campo fica inativo com o motivo, não aparece só um aviso depois.
- A bancada de código aberto (TechDraw) oferece escala em três modos: da folha, automática (cabe na folha) e personalizada com numerador e denominador, mais botões de direção e rotação.

**O que falta no Fenkra**
- Fantasma com a escala aplicada (`click` só cria no clique; nada é desenhado antes).
- Escala em preset e "automática" no momento de criar; hoje há `scale_combo` só no painel da vista já criada (`dwg_ui.rs:1772`).
- Opções de herança da vista filha (orientação, estilo, escala, alinhamento).
- Ocultar peças pequenas; margem; estado "raster" para montagens grandes.

**Como deveria ser no Fenkra**
- **Suporte**: peça ou montagem aberta, ou arquivo escolhido; se não há origem, abrir o seletor em vez de avisar.
- **Modo**: ao ativar a ferramenta, abre uma barra de opções ancorada na folha com: Direção (menu com miniaturas de cubo), Escala (Auto, 1:1, 1:2, 1:5, 1:10, 2:1, livre), Estilo (3 botões: sombreada, só arestas, com ocultas), Rótulo (liga/desliga). A vista fantasma segue o cursor; Enter ou clique confirma.
- **Parâmetros**: nome, escala, direção, estilo, rótulo, alinhar à mãe.
- **Avançado**: estado de exibição, peças ocultas, tangentes, espessura/cor, ocultar peças menores que X mm.
- **Na folha**: alça de arrastar no canto da vista, linha de alinhamento tracejada com a mãe enquanto arrasta, alça de escala (arrastar muda a escala por passos) e indicador "alinhada à Vista A" com cadeado.
- **GIF do tooltip (7 s)**: 0-2 s clique no botão e o fantasma aparece preso ao cursor; 2-4 s a barra de opções muda a escala e o fantasma cresce; 4-6 s o cursor se aproxima de outra vista e a linha de alinhamento acende; 6-7 s clique e a vista fixa com o rótulo.

### Vista em corte e linha de corte

**O que os outros têm em detalhe**
- A linha de corte é desenhada com vários segmentos (clique, clique, clique direito para concluir) ou escolhida a partir de um croqui. É possível arrastar pontos, segmentos e as setas depois de criada, e mudar o sentido da seta puxando-a.
- Profundidade: total ou por distância. Fatia (somente a seção, sem o que está atrás), com opção de fatiar todas as peças.
- Projeção: ortogonal (padrão se os segmentos são perpendiculares) ou alinhada (padrão quando algum segmento é inclinado); o fantasma só se move dentro do alinhamento, e Ctrl solta a restrição.
- Rótulo editável (letra, texto, visibilidade) e acabamento da borda do corte (serrilhado ou liso). Hachura ligável/desligável por vista e herança de corte para vistas filhas.
- Peças padrão (parafusos) podem ser "sempre cortadas", "nunca cortadas" ou seguir a configuração da peça.

**O que falta no Fenkra**
- Corte em polilinha (desvio, alinhado, meio corte): hoje a linha tem dois pontos (`hint`, `dwg_ui.rs:198-200`).
- Alças na linha e nas setas; inverter o lado por clique (o painel pede apagar e refazer, `dwg_ui.rs:1429`).
- Profundidade, fatia, peças que não são cortadas, acabamento da borda.
- Projeção "alinhada" e liberar o alinhamento com Ctrl.

**Como deveria ser no Fenkra**
- **Suporte**: vista mãe selecionada; se não há, a ferramenta entra com a dica "clique numa vista" e pré-seleciona a que está sob o cursor.
- **Modo**: Total, Meio, Desvio, Alinhado (botões com ícones); a polilinha termina com Enter ou clique duplo.
- **Parâmetros**: letra, escala, lado visto (botão Inverter com seta), profundidade (Total/Distância).
- **Avançado**: fatia, peças sem corte, hachura (padrão, ângulo, passo), borda do corte.
- **Na folha**: setas gordas nas pontas (arrastar inverte), alça central para deslocar, a linha de corte mostra o fantasma da vista resultante em tracejado até a confirmação.
- **GIF do tooltip (9 s)**: traça dois pontos; a seta aparece e o fantasma da vista em corte já mostra a hachura; clica em Inverter e o fantasma espelha; posiciona e a letra A-A aparece nos dois lugares.

### Vista de detalhe

**O que os outros têm em detalhe**
- Estilo do contorno (círculo ou perfil/polígono), letra (formato definido por norma), escala própria e opção de mostrar o contorno completo da região na vista mãe.
- O círculo é arrastado para mover e tem alça de raio; ao soltar o fantasma da ampliação acompanha.

**O que falta no Fenkra**
- Alça de raio na folha (hoje só o campo "Raio (mm do modelo)", `dwg_ui.rs:1437`).
- Contorno em perfil; a escala é fixa em 2x na criação (`toolbar.rs:319`), e depois editável em `scale_combo`.
- Linha de conexão e "contorno completo" opcionais.

**Como deveria ser no Fenkra**
- **Modo**: Círculo, Retângulo, Perfil livre.
- **Parâmetros**: letra, escala (menu com 2:1, 5:1, 10:1 e livre), raio por alça.
- **Avançado**: mostrar contorno completo, linha de conexão, rótulo.
- **GIF (6 s)**: clique no centro, arrasta o raio com o fantasma crescendo, muda para 5:1 e a ampliação se redesenha.

### Cotas (horizontal, vertical, alinhada, raio, diâmetro, ângulo)

**O que os outros têm em detalhe**
- Uma ferramenta contextual: o que foi clicado (aresta, dois pontos, arco, duas linhas) sugere o tipo, e o tipo pode ser trocado na hora (menu de contexto ou tecla). A cota viva segue o mouse e fixa no clique que escolhe o afastamento. A ferramenta continua ativa até Esc ou clique direito.
- Conjuntos automáticos: ordenada (referência de origem zero), linha de base e cadeia; alinhar ao longo de um zero; salto (jog) na linha de chamada.
- Edição: duplo clique ou botão direito abre uma caixa com o texto (marcador de valor medido que se pode apagar e reinserir), precisão da unidade principal e da tolerância separadas, tipo de tolerância (nenhuma, simétrica, bilateral, limites, ajuste ISO), sobrescrever o valor (sem alterar o modelo, e sem atualizar sozinho).
- Opções de exibição: como diâmetro, raio ou linear; entre parênteses (referência); inspeção (balão oval); texto deslocado com linha de chamada.
- Importar cotas do modelo para a vista; as cotas importadas não duplicam as já criadas.

**O que falta no Fenkra**
- Ferramenta contextual e troca de tipo (hoje seis botões e `needs_view`, `dwg_ui.rs:278`).
- Prévia da cota viva enquanto o mouse se move.
- Caixa de edição flutuante com duplo clique (hoje duplo clique só seleciona e abre o painel, `dwg_ui.rs:679-687`).
- Tipos ordenada, linha de base, cadeia; "como diâmetro/raio"; entre parênteses; inspeção.
- Precisão da tolerância separada; ajuste ISO tipado (o campo `tol` é texto livre, `dwg_ui.rs:1500`).

**Como deveria ser no Fenkra**
- **Suporte**: aresta/vértice/arco de uma vista. Sem âncora válida, o cursor mostra "sem aresta aqui" em vez de silêncio.
- **Modo**: Inteligente (padrão), e alternativas na barra: Linear, Alinhada, Ordenada, Linha de base, Cadeia, Raio, Diâmetro, Ângulo.
- **Parâmetros**: valor medido (somente leitura), texto, afastamento.
- **Avançado**: precisão, tolerância tipada, parênteses, inspeção, setas, "como diâmetro".
- **Na folha**: cota fantasma, alça do texto, alça do afastamento, duplo clique abre a caixa flutuante com os botões Ø, ±, °, <> (valor medido), 2 casas.
- **GIF (8 s)**: passa o mouse num furo e o fantasma mostra Ø8; Tab alterna entre Ø e R; clique fixa; duplo clique abre a caixa e ele escolhe "H7"; a cota ganha o ajuste.

### Notas, balões e símbolos (rugosidade, solda, GD&T, datum, centro)

**O que os outros têm em detalhe**
- Balão: ao passar sobre uma vista ou face acende um ponto de snap; um clique ancora a ponta e outro posiciona o balão; clicar em branco cria sem chamada. Linhas finas tracejadas de inferência alinham com outros balões.
- Propriedade do balão: número do item, coluna da lista, instância, nota de marcação; texto acima, abaixo e nas laterais; símbolo; forma (círculo, quadrado, nenhuma), altura do texto, largura em caracteres.
- Nós de chamada: botão direito adiciona nó, remove nó, inicia outra chamada a partir de um nó. Selecionar vários balões e alinhar na horizontal ou na vertical. Estilo (fonte, justificação, direção da pilha, seta) num painel.
- Se a ordem da lista muda, "atualizar desenho" renumera os balões.
- Balões automáticos: seleção por janela, Shift remove, "selecionar tudo", e as opções de posicionamento (lado, distância) antes de confirmar.
- Marca de centro: simples, circular (círculo de furos) e linear. Linha de centro por dois pontos, duas arestas ou uma aresta de silhueta de cilindro. Ficam ocultas por padrão em geometrias circulares e se liga pelo menu da vista.
- Observação importante de limitação: no relato de usuários, nem todo CAD cota ou centra a partir de arestas ocultas; a solução usada é fazer um corte pelo centro. O Fenkra pode fazer melhor aqui.

**O que falta no Fenkra**
- Fantasma e ponto de snap na vista para a ponta da chamada.
- Nós de chamada, alinhar vários, estilo de forma/altura/largura.
- Rugosidade e solda como campo único de texto; sem seletor de método, direção de estrias, lado da seta/outro lado, contorno, solda em campo.
- Centro: modos linear e circular, e eixo entre dois furos.
- Datum com sequência de letras automática.

**Como deveria ser no Fenkra**
- **Balão**: Suporte: ponto sobre vista ou componente. Modo: manual ou automático (janela). Parâmetros: propriedade (item, código, quantidade), forma, tamanho. Avançado: nós, alinhar, empilhar. Na folha: ponta com alça e linha tracejada de inferência.
- **Rugosidade**: um construtor de símbolo com os campos nas posições do símbolo (valor, método, sobremedida, direção, amostragem), prévia grande do símbolo e valores comuns em botões (Ra 0,8, 1,6, 3,2, 6,3).
- **Solda**: construtor com linha de referência, seta e outro lado, tipo de cordão em botões com o desenho, medida, comprimento, passo, contorno e solda em campo.
- **Marca de centro**: modo simples, circular (seleciona os furos) e linear (seleciona dois furos).
- **GIF do balão (6 s)**: o mouse passa sobre o componente, acende o ponto; clique ancora; arrasta e as linhas tracejadas de inferência alinham com o balão vizinho; clica; surge o número 3.

### Tabelas (BOM, furos, dobras, lista de corte)

**O que os outros têm em detalhe**
- Colunas configuráveis, ordenação, escolha de nível (só topo, estruturada), tabela com cabeçalho repetido em cada folha, tabela dividida em colunas, renumerar itens com sobrescritas controladas, e etiquetas de furo nas vistas ligadas à tabela.
- Tabela de furos com origem escolhida na vista (ordenada a partir de um zero).

**O que falta no Fenkra**
- Seleção de colunas e ordenação; só há nível e exportar CSV (`dwg_ui.rs:1601-1617`).
- Origem da tabela de furos por vista; etiquetas A1, A2 nas vistas.
- Divisão da tabela entre folhas.

**Como deveria ser no Fenkra**
- **Suporte**: origem da montagem ou da peça; para furos, a vista base escolhida.
- **Parâmetros**: nível, colunas (caixas marcáveis arrastáveis), ordenação.
- **Avançado**: dividir em colunas ou folhas, renumerar, etiquetas nas vistas.
- **Na folha**: alça de canto para mover; cabeçalho clicável para ordenar.
- **GIF (7 s)**: clique no botão, o fantasma da tabela segue o cursor, abre o painel com colunas, tira uma coluna e a tabela se reflui, fixa.

### Arquivo (Novo, Abrir, Salvar, Folha, exportações)

**O que os outros têm em detalhe**
- Novo desenho com assistente: formato e orientação, norma, método de projeção (1º ou 3º diedro), escala geral, modelo de folha e origem.
- Exportação com opções: quais folhas, cor ou preto e branco, mapeamento de espessura, camadas e versão de formato; prévia de impressão.
- Ao salvar ou atualizar, o estado de cada vista (atualizada, desatualizada, quebrada).

**O que falta no Fenkra**
- Assistente de novo desenho: hoje cria uma folha A3 vazia (`dwg_ui.rs:227`).
- Opções e prévia de exportação.
- Estado de atualização visível por vista (já planejado, B30).

**Como deveria ser no Fenkra**
- **Novo**: diálogo curto com Formato, Orientação, Norma, Diedro, Escala, Origem (peça aberta como padrão) e uma miniatura da folha.
- **Exportar**: diálogo com folhas, cor, espessura, prévia.
- **GIF do Novo (6 s)**: abre o diálogo, muda o diedro e o símbolo da folha troca, escolhe a peça, termina com as três vistas já sugeridas.

## Detalhes finos transversais

- **Mensagem de estado**: os outros dizem o próximo passo e o que está bloqueando ("selecione uma vista mãe"). O Fenkra diz o passo (bom) mas mostra o impedimento só depois do clique no botão, por aviso temporário.
- **Atalhos**: Esc, Del, F2 e F5 existem; falta Enter para confirmar e Tab para alternar o tipo de cota, e nenhuma ferramenta tem tecla de atalho exibida no tooltip (só "Salvar" tem).
- **Nomes repetidos**: "Corte" aparece para a vista em corte e para a lista de corte. "Esquema" e "Câmera" só fazem sentido para quem já conhece o módulo. Os nomes deveriam dizer o substantivo completo ("Vista em corte", "Lista de corte").
- **Ícones**: um por comando, mas as três cotas lineares compartilham a mesma família de ícone, e sem texto de apoio. Mostrar o nome ao lado do ícone na família ativa ajuda.
- **Valores padrão**: os outros mostram o valor padrão no próprio campo (cinza) e o que mudou fica em destaque; o Fenkra mostra o valor, sem sinalizar se é padrão.
- **Reversão**: Ctrl+Z existe para o desenho inteiro (bom). Falta voltar uma opção ao padrão do estilo com um clique.

## Padrão de tooltip proposto para a bancada

Três camadas, na ordem em que o mouse permanece:
1. **0 s**: nome completo e atalho ("Vista em corte   S").
2. **0,6 s**: uma frase do que faz e o pré-requisito ("Corta a vista selecionada ao longo de uma linha. Selecione antes a vista de origem.").
3. **1,5 s**: GIF de 5 a 10 s em laço, sem som, com o cursor desenhado, mais a linha "Avançado:" do `it(...)` atual.

Cada campo do painel segue a mesma regra: rótulo, unidade e uma frase de efeito ("Afastamento (mm): distância da linha de cota até a peça"). Os textos entram em `i18n.rs` com tradução, e os GIFs ficam gerados por `FENKRA_DEMO*` para não envelhecerem com a interface.

## Fontes (resumo, sem endereço)

Ajudas oficiais de um CAD avançado para desenho (diálogos de vista, seção, cota, ordenada, detalhe, balão, chamada de furo), documentação de um CAD na nuvem para balões e linhas de centro, e a documentação aberta do FreeCAD (TechDraw: vista, cota). As limitações citadas em fóruns foram usadas só como indício e estão marcadas como tal.
