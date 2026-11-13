# Teste do usuário com mouse e teclado reais (B41), 13/11/2026

Um agente usou o Fenkra como uma pessoa usaria: mouse e teclado do Windows por `SendInput`, sem
variáveis `FENKRA_DEMO` e sem atalhos internos. Cada ação foi conferida por captura antes e depois.
Quando havia arraste, também houve uma captura com o botão ainda apertado.

- Binário: `F:/cargo-target/projeto-cad/release/fenkra-teste.exe` (0.1.0-alpha.36), com `APPDATA` e
  `LOCALAPPDATA` numa pasta temporária. O estado do usuário não foi tocado.
- Tela: 1920x1080, janela maximizada (área cliente 1920x1009). Coordenadas abaixo são da área cliente.
- Equipamento: `scripts/teste_interface/fk.py`, com os roteiros em `scripts/teste_interface/roteiros/`.
- Capturas de falha e de estado final: `docs/validacao/capturas-mouse/`.

## Resumo

| # | Caso | Resultado |
|---|---|---|
| 1 | Arrastar recurso na árvore | **falhou em parte**: funciona entre recursos e para pasta de geometria; falha em pasta de comandos, Parâmetros, corpo e Grupos vazios |
| 2 | Menu "⋯" da árvore | **falhou em parte**: minimizar e fechar ok; árvore solta fica cortada |
| 3 | Painéis: bordas, redimensionar, minimizar, fechar | **ok, com defeitos**: encaixe e tamanho ok; na base cobre a barra de status; redesenho atrasado |
| 4 | Roleta Shift+W | ok |
| 5 | X e Cancelar de comando novo | ok |
| 6 | PMI: Ctrl, Shift, F2, duplo clique, arrastar a seta | **falhou em parte**: F2, duplo clique na nota e arraste da âncora ok; duplo clique na seta e Ctrl falham |
| 7 | Montagem: inserir pela prévia, abrir peça e voltar, Reancorar | inserir e abrir/voltar ok; Reancorar **não feito** |
| 8 | Biblioteca: furo, ponto, solto | **falhou**: solto não entra (montagem sem arquivo, aviso ilegível); furo e ponto **não feitos** |
| 9 | Desenho | **não feito** |
| 10 | Chapa: alças, planificar, dobrar e desdobrar animado | ok; a janela da chapa cobre a barra de menus |
| 11 | Teclado: setas, Page Up/Down, Alt+↑/↓, Tab | **falhou em parte**: setas mortas depois de clicar na árvore; o resto ok |
| 12 | Tooltips e F1 | ok |

Achado transversal: **o Fenkra só redesenha depois do próximo evento de entrada.** O resultado de um
clique ou de soltar o botão só aparece quando o mouse se mexe de novo. Visto ao minimizar e expandir
o painel, ao encaixar o painel e ao planificar a chapa. Daqui em diante, o equipamento move o cursor
1 px e volta antes de cada captura. Com `FK_CUTUCAR=0` esse movimento é desligado e o atraso aparece.

## 1. Arrastar um recurso na árvore

Exemplo "Placa paramétrica". Roteiro: `roteiros/arvore_arrastar.json`.

| Origem → destino | Resultado | O que aconteceu |
|---|---|---|
| Filete1 → sobre Furos M6 (recurso sobre recurso, no corpo) | ok | linha de soltura apareceu; Filete1 (e a pasta Acabamentos) passou para antes de Furos M6 |
| Sketch base → pasta Geometria | ok | contorno na pasta durante o arraste; o sketch entrou em Geometria (2) |
| Furos M6 → pasta Acabamentos (pasta de comandos, dentro do corpo) | **falhou** | nenhum indicador; ao soltar, nada mudou e a pasta ficou selecionada |
| L → sobre margem (pasta Parâmetros) | **falhou** | nenhum indicador; ao soltar, "margem" ficou selecionado |
| PlanoTopo → Corpo principal | **falhou** | nenhum indicador; o corpo ficou selecionado |
| Furos M6 → Grupos vazios › Capturas | **falhou** | nenhum indicador; Capturas ficou selecionado |

Por que falha: em `crates/fenkra-app/src/app.rs`, só três lugares aceitam o arraste (têm
`dnd_release_payload`): a linha de um recurso (`TreeAction::Move`), a pasta de geometria
(`TreeAction::MoveTo`) e as linhas do PMI. A pasta de comandos, o nó do corpo, as linhas de
parâmetro e os grupos de Referências, Publicações, PMI e Capturas não aceitam nada. Além disso,
`Document::move_to_folder` (`fenkra-core/src/doc.rs`) recusa qualquer pasta que não seja
`FolderKind::Geometria`. Então, mesmo que a pasta de comandos aceitasse o arraste, o documento
rejeitaria. Outro defeito: soltar sobre um alvo que não aceita conta como clique e seleciona o alvo.

Capturas: `capturas-mouse/01_arvore_pasta_de_comandos.png` (segurando | solto),
`01_arvore_parametro_corpo_grupos.png` (três alvos, segurando), `01_arvore_pasta_geometria_ok.png`.

Backlog:
- Árvore: soltar recurso numa pasta de comandos do mesmo corpo move para ela, mantendo a ordem válida (`move_to_folder` aceitar `Comandos` dentro do coletor certo).
- Árvore: soltar sobre o nó do corpo move o recurso para esse corpo, ou mostra o motivo da recusa.
- Árvore: parâmetros reordenáveis por arraste.
- Árvore: alvo que não aceita a soltura mostra o cursor de proibido e não vira seleção ao soltar.

## 2. Menu "⋯" da árvore

Passos: clicar em "⋯" (206, 117) › "Soltar a árvore"; arrastar o fundo da lista entre as linhas;
"Minimizar a árvore"; "Expandir"; "Fechar a árvore"; reabrir por Exibir › Árvore; desmarcar "Soltar".

- Soltar: a árvore se desloca com o arraste, mas **fica cortada** na largura original do painel (o
  texto some em x ≈ 327). Causa: `ui.set_clip_rect(regiao)` em `app.rs`, logo depois de
  `Rect::from_min_size(regiao.min + self.arvore.pan, ...)`. O recorte continua no lugar fixo do painel.
  Também não dá para soltar arrastando o fundo vazio abaixo da lista: ali o arraste gira a câmera.
- Minimizar, expandir e fechar: ok. Reabrir por Exibir › Árvore: ok, mas a árvore volta no
  deslocamento anterior e continua cortada. Desmarcar "Soltar" zera o deslocamento.

Captura: `capturas-mouse/02_arvore_solta_cortada.png`.

Backlog:
- Árvore solta: recortar pela área útil da janela, não pelo retângulo original do painel.

## 3. Painéis

Painel de comando da Caixa (Sólidos › Caixa).

| Passo | Resultado |
|---|---|
| Arrastar a barra de título até a borda direita | ok: aparece "solte para encaixar" e o painel encaixa à direita |
| Até a borda esquerda | ok: encaixa ao lado da árvore |
| Até o topo | ok: fica abaixo da barra de ferramentas |
| Até a base | **defeito**: o painel desce até a borda e cobre a barra de status (Reconstruir e navegação ficam por baixo) |
| Redimensionar pelo canto inferior direito | ok |
| Arrastar a borda de cima | redimensiona (esperado), mas a borda fica a ~16 px acima do título e é fácil pegá-la sem querer |
| Minimizar (_) e expandir | ok, mas o resultado só aparece no evento seguinte (atraso de redesenho) |
| Fechar (X) | ok (ver caso 5) |

Observações: ao soltar na borda direita, o X do painel fica sob o cursor e o tooltip "fechar"
aparece. Um clique logo em seguida fecharia o comando. O painel também reabre minimizado no comando
seguinte, porque guarda o último estado.

Capturas: `capturas-mouse/03_painel_sobre_barra_de_status.png` e `03_atraso_de_redesenho.png`. Na
segunda, à esquerda está a captura logo depois do clique em minimizar (o painel ainda aberto) e à
direita a captura depois de mover o mouse (o painel minimizado).

Backlog:
- Painel encaixado na base respeita a barra de status.
- Pedir redesenho (`request_repaint`) quando um clique ou uma soltura muda o estado da interface (minimizar, encaixar, planificar): hoje o resultado só aparece no próximo evento.

## 4. Roleta Shift+W

Passos: mouse na vista (1100, 600), Shift+W; rolar a roda 3 vezes sobre a roleta; arrastar dentro dela.

Resultado: ok. A roleta abre fixa com 8 setores. A rolagem não aproxima a câmera, e com 8 comandos
não há página para virar. O arraste realça o setor sob o cursor e a câmera não se mexe. Esc fecha.

## 5. X e Cancelar de um comando novo

- Caixa nova › X do painel: a "Caixa1" sai da árvore. ok.
- Caixa nova › Cancelar: a "Caixa1" sai da árvore. ok.

## 6. PMI

Passos: Mais › Detalhamento 3D › Nota; clicar numa face; escrever o texto (roteiro
`roteiros/pmi_nota_texto.json`); Enter.

| Passo | Resultado |
|---|---|
| Criar a nota numa face e escrever o texto | ok |
| Duplo clique no texto da nota | ok: abre o painel de edição da nota (mas o painel abre por cima da própria nota) |
| Duplo clique na seta | **falhou**: abriu a edição da extrusão "Placa", a face sob a seta. O clique atravessa a seta |
| Arrastar o início da seta com a nota selecionada | ok: prévia "aresta circular" e a âncora vai para o furo |
| Arrastar o início da seta sem a nota selecionada | gira a câmera (as alças só existem com a nota selecionada) |
| F2 com a nota selecionada | ok: abre o campo de renomear na árvore |
| Ctrl+clique em dois alvos no comando Nota, depois Enter | **falhou**: a nota é criada no primeiro clique, com uma seta só; o segundo Ctrl+clique não adiciona seta |
| Shift+clique | não testado à parte (o código trata Shift igual a Ctrl) |

Outros defeitos vistos:
- O menu "Mais" continua aberto depois de escolher Nota e cobre a vista.
- O painel "Edição" do comando Nota mostra as propriedades do item selecionado antes (Sketch base,
  depois Placa), e não os campos da nota.

Capturas: `capturas-mouse/06_duplo_clique_na_seta_abre_extrusao.png`,
`06_ctrl_cria_nota_de_uma_seta.png`, `06_menu_mais_fica_aberto.png` e `06_ancora_arrastada_ok.png`.

Backlog:
- PMI: a seta da nota entra no teste de clique antes da face, para que o duplo clique abra a nota com a seta expandida (B368).
- PMI: Ctrl ou Shift no comando Nota acumula alvos e só cria a nota com Enter.
- Menu "Mais" fecha ao escolher um comando.
- Comando Nota: o painel começa com os campos da nota, sem herdar a seleção anterior.

## 7. Montagem

Passos: aba Montagem › Nova montagem › Inserir peça. Clicar na miniatura da peça aberta. O Fenkra
abriu o diálogo "Salvar como" e a peça foi gravada em `scratchpad/pecas/suporte.fkp`. Depois, dois
cliques na vista; clique direito numa instância › "Editar a peça"; "← Voltar".

- Inserir pela prévia: ok. A peça fantasma segue o cursor, o primeiro clique fixa na origem e cada
  clique cria uma cópia ("2 colocada(s)").
- Abrir a peça e voltar: ok. A peça abre com o conjunto esmaecido em volta e com o caminho
  "Montagem › suporte.2 ← Voltar". O Voltar retorna à montagem.
- Ao voltar, o comando Inserir peça reaparece armado, com a peça na mão. Um clique distraído na vista
  insere mais uma cópia, e foi assim que entrou uma terceira cópia neste teste.
- Reancorar restrição perdida: **não feito**. Criar pela interface uma restrição e depois apagar a face
  dela custaria mais passos do que coube nesta rodada.

Capturas: `capturas-mouse/07_inserir_pela_previa_ok.png` e `07_voltou_para_montagem.png`.

Backlog:
- Voltar da peça para a montagem não rearma o Inserir peça (ou pergunta antes).

## 8. Biblioteca

Passos: Montagem › Biblioteca; catálogo AN › Porca › AN 310; tamanho AN310-3; "Solto"; "Adicionar à
montagem"; clicar na vista; tentar Enter.

- Solto: **falhou**. A porca fantasma segue o cursor, mas nenhum clique a coloca. O motivo aparece
  num aviso: "Salve a montagem antes: as peças dos fixadores são gravadas na pasta dela"
  (`biblioteca_ui.rs`, `st.dir()` vazio). O aviso sai numa coluna estreita, com uma ou duas sílabas
  por linha, e dura pouco. Só foi lido numa captura feita 0,4 s depois do clique.
- Um clique sobre o plano YZ seleciona o plano em vez de pôr a porca.
- Visual: a fileira de filtros (Métrico, Polegada…) fica parcialmente sob o campo de busca, e o
  cabeçalho "Tamanho" da tabela fica sob a prévia 3D.
- Em furo e em ponto: **não feitos**. A peça de teste não tem furo, e a montagem sem arquivo bloqueia
  qualquer inserção.

Capturas: `capturas-mouse/08_aviso_estreito_salve_a_montagem.png` e `08_filtros_sob_a_busca.png`.

Backlog:
- Biblioteca: com a montagem sem arquivo, o botão "Adicionar" pede para salvar (abre o diálogo) em vez de armar o fantasma que não pode ser colocado.
- Aviso (toast) com largura mínima legível.
- Biblioteca: filtros e cabeçalho da tabela sem sobreposição.

## 9. Desenho

**Não feito.** Faltou tempo nesta rodada. Fica para a próxima, com o mesmo equipamento.

## 10. Chapa metálica

Exemplo "Suporte de chapa".

- Planifica (barra): ok, mas a peça só aparece planificada depois de mover o mouse (atraso de redesenho).
- Planifica de novo: volta a dobrar.
- Parâmetros › "Dobrar e desdobrar" › ▶: ok. Desdobra animado: capturas a cada 0,3 s mostram a aba
  abrindo aos poucos. Os glifos numerados aparecem nas linhas de dobra.
- Alças da aba (duplo clique em Aba1): ok. Arrastar a alça "Ângulo 90°" muda para 111°, com o campo
  do painel e a prévia atualizados durante o arraste.
- Defeito: a janela "Chapa metálica" abre em (0, 0), por cima da barra de menus e de parte da barra
  de ferramentas. Isso fere a regra "janela fora da barra de menus".
- Concluir na janela deixa a peça planificada se o controle terminou em 100%. Esperado, mas não há aviso.

Capturas: `capturas-mouse/10_desdobrar_animado_ok.png`, `10_alca_angulo_ok.png` e `10_janela_chapa_sobre_menu.png`.

Backlog:
- Janela da chapa abre abaixo da barra de ferramentas, como os outros painéis livres.

## 11. Teclado

| Passo | Resultado |
|---|---|
| Clicar em "Placa" na árvore; seta ↓ duas vezes | **falhou**: nada acontece (nem a vista gira, nem a seleção anda) |
| Esc; depois seta → duas vezes | ok: a vista gira 15° por toque |
| Page Up | ok: aproxima |
| Alt+↑ com Placa selecionada | ok: escolhe o comando anterior (Sketch base) |
| Tab três vezes | ok: o anel de foco passa por Arquivo, Exibir e Editar, bem visível |

Por que falha: depois do clique na árvore, o rótulo clicado fica com o foco do teclado. Pelo B321
(`teclado.rs`), as setas só giram a vista "sem nada em foco". Elas também não andam pela árvore,
então ficam sem efeito até um Esc. É um beco sem saída para quem não sabe do Esc.

Captura: `capturas-mouse/11_anel_de_foco_tab.png`.

Backlog:
- Com um item da árvore em foco, ↑/↓ andam pela árvore (ou o clique não prende o foco e as setas voltam para a vista).

## 12. Tooltips e F1

- Passar o mouse em Furo e esperar 1,5 s: ok. Tooltip com nome, descrição e animação.
- F1 com o mouse sobre Furo: ok. Abre "Ajuda do comando — Furo". O tooltip continua aberto ao lado.

Captura: `capturas-mouse/12_tooltip_e_f1.png`.

## O que não deu para fazer

- Desenho (caso 9), Reancorar (7) e inserção em furo e em ponto (8): faltou tempo e, no caso 8, faltou
  a pré-condição (montagem gravada e peça com furo).
- Mouse 3D e segundo monitor: o equipamento só usa mouse e teclado comuns. A máquina tem dois monitores,
  mas o teste ficou no principal.
- O foco nunca se perdeu para outra janela durante o teste: nenhuma espera de 30 s foi necessária.

## Como repetir

```bash
set FK_ESTADO=%TEMP%\fk_estado.json
python -I scripts/teste_interface/fk.py abrir --exe F:/cargo-target/projeto-cad/release/fenkra-teste.exe --appdata %TEMP%\fk_appdata --saida %TEMP%\fk_cap
python -I scripts/teste_interface/fk.py clicar 890 530 --esperar 6   # abre "Placa paramétrica"
python -I scripts/teste_interface/fk.py roteiro scripts/teste_interface/roteiros/arvore_arrastar.json
python -I scripts/teste_interface/fk.py fechar
```

Cada comando grava uma captura numerada na pasta de saída. `recorte`, `reduzir` e `juntar` preparam
as imagens para leitura.
