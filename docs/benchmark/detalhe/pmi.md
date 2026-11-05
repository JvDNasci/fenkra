# Benchmark de detalhe: bancada PMI 3D

Foco: experiência de uso, nos campos e na manipulação. Os tipos de cota já estão em `docs/pesquisa/bench-pmi-cotas.md` (B185–B192) e não se repetem aqui. A comparação é com "softwares de modelagem CAD avançados", feita a partir de documentação pública. Onde a ajuda oficial não pôde ser lida (páginas que exigem JavaScript, wikis bloqueadas), a conclusão vem de trechos de busca e é menos firme. A avaliação do Fenkra foi feita sobre o código; os ícones foram julgados pela estrutura dos arquivos, e a impressão visual ainda precisa de auditoria com captura.

## Resumo: as 5 maiores lacunas de experiência

1. **Não há prévia 3D antes de criar.** O comando só chama `create` quando as seleções bastam (`crates/fenkra-app/src/pmi_ui.rs:76-135`). Até lá, o usuário vê um aviso (toast) e uma faixa "Escolha os elementos da anotação no 3D…" (`crates/fenkra-app/src/app.rs:4661-4667`). Nos CADs avançados, depois de escolher poucos elementos a cota aparece como fantasma que segue o cursor, já com o valor medido, e o clique só confirma. Esta é a causa direta da sensação de "pouco intuitivo".
2. **O plano da anotação não é escolhido durante o comando.** Nos outros CADs, o plano onde o texto fica é decidido na prévia: tecla para percorrer planos candidatos, outra para trocar a face de referência, outra para alinhar à geometria. No Fenkra o texto nasce de frente para a tela (`plane: None`, `pmi_ui.rs:126`) e o plano só muda depois, no painel (`pmi_ui.rs:1290-1318`) ou no menu de contexto do rótulo (`pmi_ui.rs:683`).
3. **A manipulação 3D se resume a arrastar o rótulo e as âncoras.** Existem o arraste do rótulo (`pmi_ui.rs:647-662`) e as alças das âncoras da anotação selecionada (`pmi_ui.rs:617-640`). Faltam alça de afastamento da linha de cota, alça de extensão, vértices editáveis da chamada, encaixe em eixos e cursor que mostre o que será ancorado.
4. **Os campos do painel não explicam nada.** Em `panel_inner` (`pmi_ui.rs:1180-1325`) quase todo campo é um `text_edit_singleline` sem exemplo, unidade, limite ou validação: Ra, Letra, Item, Tolerância + e −. Só a solda tem exemplos (`hint_text`, `pmi_ui.rs:1121-1137`) e o quadro de tolerância tem dois textos de ajuda (`pmi_ui.rs:1077-1082`). Os outros CADs oferecem listas, mini barra com precisão e estilo, e valor padrão vindo da norma ativa.
5. **O tooltip é uma frase e o ícone não mostra o resultado.** Cada comando tem `brief` e `advanced` em `crates/fenkra-app/src/toolbar.rs:352-371`, sem imagem, atalho nem passo a passo; as traduções estão em `crates/fenkra-app/src/i18n.rs` (a partir da linha 1381). Os 11 ícones (`assets/icons/pmi_*.svg`) usam a mesma paleta azul e as quatro cotas diferem pouco entre si. O B60 cobre o tooltip em dois estágios; falta o conteúdo próprio da bancada (GIF de 5 a 10 s por comando).

## Checklist por comando

Legenda: ok, parcial, falta. "Prévia 3D" é a imagem do resultado antes do clique final; "Manipulação 3D" é mexer na anotação criada, direto na viewport; "Dica por campo" é o texto de ajuda dentro do painel.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Cota linear (0) | parcial | falta | parcial | parcial | parcial | falta | parcial | Mede pontos soltos (vértice, meio de aresta, ponto de face), `pmi_ui.rs:50-73`. Sem escolha de eixo nem de tipo de medida. |
| Raio (1) | parcial | falta | parcial | parcial | parcial | falta | parcial | Exige aresta circular; o erro só aparece depois do clique. Ícone quase igual ao do diâmetro. |
| Diâmetro (2) | parcial | falta | parcial | parcial | parcial | falta | parcial | Mesmo fluxo do raio; não há gesto para alternar raio e diâmetro. |
| Ângulo (3) | falta | falta | parcial | parcial | parcial | falta | falta | A dica "vértice e dois pontos" não diz a ordem; nada indica qual clique é o vértice. |
| Nota (4) | parcial | falta | parcial | ok | ok | falta | parcial | O texto só é digitado no painel, depois de criada. Sem edição direta no 3D. |
| Rugosidade (5) | parcial | falta | parcial | parcial | parcial | falta | parcial | Ra nasce "1,6" sem unidade; processo é texto livre, sem lista (`pmi_ui.rs:1207-1216`). |
| Solda (6) | parcial | falta | parcial | parcial | ok | parcial | parcial | Melhor tooltip e exemplos nos campos (`weld_ui`, `pmi_ui.rs:1096-1140`); o símbolo só se vê depois de criado. |
| Referência (7) | parcial | falta | parcial | ok | parcial | falta | parcial | Letra "A" fixa de início (`pmi_ui.rs:23`); não propõe a próxima letra livre nem recusa I, O, Q. |
| Tolerância (8) | parcial | falta | parcial | parcial | ok | parcial | parcial | Quadro completo, mas as casas só são editadas no painel (`gdt_ui`, `pmi_ui.rs:1055-1095`). |
| Legenda 3D (9) | ok | falta | parcial | parcial | ok | parcial | ok | Um clique, sem seleção; vai ao canto da caixa envolvente, sem escolha de lugar (`pmi_ui.rs:82-102`). Campos com botões de inserção. |
| Balões (10) | ok | falta | parcial | parcial | parcial | falta | ok | Só na montagem, avisa por toast (`pmi_ui.rs:140-143`). Refaz os balões anteriores sem confirmar. |

Contagem (11 comandos × 7 critérios = 77 células, a coluna Nota não conta): **ok 9, parcial 47, falta 21**. Nenhuma célula de prévia 3D é ok, e a coluna de dica por campo tem só 3 células parciais (solda, tolerância, legenda) e nenhuma ok. Antes de fechar a coluna de ícone, convém revisar com captura visual.

## O que os outros têm e o Fenkra não, por comando

### Cota linear, raio, diâmetro e ângulo

**O que os outros têm, em detalhe**
- *Seleção com realce.* Ao passar o cursor, o elemento candidato se realça; uma tecla muda o tipo de alvo (face, aresta, eixo de furo). No Fenkra o realce só existe depois que a anotação existe (`highlight`, `pmi_ui.rs:345`).
- *Prévia viva.* Com dois elementos aceitos, a cota aparece fantasma, com o valor, e acompanha o cursor; o clique fixa a posição. No Fenkra a cota nasce no deslocamento padrão `[0, 0, 12]` (`pmi_ui.rs:119-123`) e o usuário reposiciona depois.
- *Plano de anotação na prévia.* Uma tecla troca para o próximo plano candidato; outra troca a face de referência; o menu de contexto tem as mesmas opções. Um modificador (Tab) alinha o texto à geometria.
- *Mini barra flutuante* sobre a cota recém-criada, com precisão, estilo e tolerância, e menus que seguem a norma ativa nas configurações do documento.
- *Promover* uma cota do modelo (croqui ou feature) a anotação 3D, levando a tolerância junto.
- *Chamada com vértices*, adicionados e removidos por menu de contexto.
- *Raio e diâmetro* escolhidos pelo tipo de curva e por preferência, com troca por clique direito.

**Como deveria ser no Fenkra**
- *Suporte.* Cota sobre alvos (B82); aqui o foco é a mão: modo automático que infere o tipo pela seleção (aresta circular, diâmetro; duas faces paralelas, distância; duas arestas, ângulo).
- *Modo.* Um comando "Cota" com Tab alternando raio e diâmetro e a variante da medida (centro, interior, exterior).
- *Parâmetros.* Valor medido, texto substituto, tolerância, precisão, prefixo "4×" e sufixo. *Avançado:* plano, estilo de seta, linha de extensão.
- *No 3D.* Candidatos realçados antes do clique; fantasma da cota seguindo o cursor com o valor; plano candidato como moldura translúcida; contagem do passo ("1 de 2") junto ao cursor; cursor verde para alvo válido, vermelho para inválido.
- *GIF do tooltip (7 s).* O cursor passa sobre duas faces paralelas e as duas se realçam; a cota fantasma surge com "24,00"; Tab muda para "ao longo de X"; o clique fixa; a cota é arrastada para fora da peça.

### Nota e chamada

**O que os outros têm**: edição do texto no próprio quadro, com mini barra de fonte e tamanho; campos associados a propriedades do modelo (o Fenkra só os tem na Legenda, `pmi_ui.rs:1235`); chamada com várias pontas e vértices intermediários; opções "manter paralela à tela" e "travar tamanho e posição" (o Fenkra deita o texto num plano, mas não trava); prévia do texto com a seta já presa à geometria.

**Como deveria ser no Fenkra**
- *Suporte:* um alvo ou vários. *Modo:* nota simples, nota com chamada, nota presa a plano. *Parâmetros:* texto, campos de propriedade ({material}, {massa}), tipo da ponta. *Avançado:* travar tamanho, quebra de linha.
- *No 3D:* aparece um cursor de texto no ponto do clique e o usuário digita ali; Enter fecha.
- *GIF (6 s):* clique numa face, abre-se a caixa de texto, digita-se "Tratar 58 HRC", a seta acompanha ao arrastar.

### Rugosidade

**O que os outros têm**: assistente com o símbolo e um campo por parâmetro (Ra, Rz…), lista de processos, direção das marcas, e símbolo que se alinha à face (o Fenkra o desenha de frente para a tela). Cada caixa atualiza o símbolo montado em tempo real.

**Como deveria ser no Fenkra**: campos com unidade (µm); lista de valores preferenciais (0,4; 0,8; 1,6; 3,2; 6,3; 12,5); lista de processos (torneado, fresado, retificado, fundido); validação de formato; o símbolo atualiza ao digitar; ao arrastar a âncora para outra face, ele reancora. GIF (5 s): clicar numa face, abrir a lista de Ra, o símbolo troca de valor. A estrutura completa do acabamento é o B190; aqui entra só a interação.

### Solda

**O que os outros têm**: prévia do símbolo ainda na hora de escolher o lado (seta ou outro), tecla para alternar o alinhamento e continuação da definição pelo botão direito. O Fenkra cobre bem os campos, mas o diálogo só abre depois do objeto criado.

**Como deveria ser**: abrir o painel já no comando, com o símbolo desenhado junto ao cursor e o lado da seta indicado no 3D por uma seta colorida. GIF (8 s): clicar na junta, escolher "filete", o símbolo muda de lado quando Tab é apertado.

### Referência (datum) e Tolerância (quadro)

**O que os outros têm**
- Datum com letra proposta automaticamente (a próxima livre) e bloqueio das letras proibidas; símbolo ligado à face por uma linha que se reposiciona sem quebrar.
- Quadro montado por barra: característica, valor, modificadores e datums escolhidos de uma lista dos datums existentes; mensagem imediata se a característica não vale para o tipo de elemento (as regras são o B108 e o B38; aqui o ponto novo é a interação).
- O primeiro passo é "escolha o plano de colocação"; depois, o objeto terminal da seta.
- Cada casa do quadro se edita por clique duplo no próprio quadro.

**Como deveria ser no Fenkra**
- *Datum:* sem painel; criado com a letra escolhida sozinha, editável por clique duplo no 3D. Dica do campo: "as letras I, O e Q não são usadas".
- *Quadro:* no clique aparece o quadro em branco ligado à face; cada casa é clicável com lista (símbolo, valor, modificador, datum). Os datums existentes viram botões.
- *No 3D:* face de aplicação realçada em azul; datum candidato em verde; erro de característica em vermelho, com explicação curta.
- *GIF (9 s):* clicar numa face, escolher "Planeza" na lista, digitar 0,05, o quadro surge ligado à face; arrastar o quadro e a seta continua presa.

### Legenda 3D e Balões

**O que os outros têm**: modelo de legenda com campos ligados às propriedades e, nos balões, prévia dos números antes de aplicar. O Fenkra executa direto e "refaz os balões anteriores" (`pmi_ui.rs:229`) sem prévia.
**Como deveria ser**: confirmação com o contador "12 balões serão criados; 3 antigos serão trocados" e opção de preservar os manuais. GIF (6 s): apertar o ícone e os balões surgem numerados em todos os componentes.

## Padrões transversais

**Fluxo de criação em quatro estados.** (1) comando ativo, com a mensagem do próximo passo na barra de status; (2) o hover realça o candidato e diz o tipo (face, aresta, furo); (3) a prévia fantasma segue o cursor; (4) o clique fixa e uma mini barra aparece sobre a anotação. Esc cancela sem criar nada; Backspace retira o último elemento escolhido. Hoje o Fenkra tem o estado 1 só como faixa laranja (`app.rs:4661`) e cancela por botão.

**Plano de anotação.** Uma moldura translúcida mostra onde o texto ficará; a barra de espaço percorre os planos candidatos (faces vizinhas e planos principais) e uma segunda tecla troca a face base. A escolha vira o padrão da sessão para as próximas anotações. Quando o texto não cabe no plano ativo (por exemplo, a cota é perpendicular a ele), a mensagem diz "este plano não mostra a cota; escolha outro".

**Edição depois de criada.** Clique duplo no texto edita no lugar. Alças: texto, linha de cota, extensões e vértices da chamada. Arrastar a âncora para outra face mostra a nova âncora válida em verde ou inválida em vermelho (já existe em parte, `pmi_ui.rs:629-631`; falta encaixe em eixo e vértices da chamada).

**Dicas por campo.** Cada campo precisa de rótulo com unidade, exemplo em cinza, faixa válida com borda vermelha se inválida e tooltip de uma linha com o porquê. Campos que hoje não têm nada: Ra, Processo, Letra, Item, Texto da nota, Tolerância + e −, Prefixo, Sufixo, Pasta e Posição X/Y/Z do texto.

**Valores padrão e mensagens.** Os iniciais (Ra "1,6", datum "A", tolerância "0,05") deveriam vir da norma ativa, com uma dica discreta no primeiro uso. As mensagens dizem o que fazer: em vez de "escolha uma aresta circular", algo como "falta uma aresta circular; clique no contorno do furo".

## Fontes consultadas

Páginas públicas de ajuda de CADs avançados sobre anotações gerais (seleção, plano candidato, troca por teclas, prévia, vértices da chamada, mini barra), cotagem de tamanho e de localização por feature, orientação de anotação em definição baseada em modelo, um artigo técnico sobre a colocação do quadro de tolerância (plano primeiro, objeto terminal depois) e a wiki de um CAD livre para seleção e arraste de cotas. Os links vão na resposta do especialista, não neste arquivo.
