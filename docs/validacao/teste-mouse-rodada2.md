# Teste com mouse e teclado reais, rodada 2 (B451), 14/11/2026

Um agente usou o Fenkra com mouse e teclado do Windows (`SendInput`), sem `FENKRA_DEMO`, pelo
equipamento `scripts/teste_interface/fk.py`, com `APPDATA`/`LOCALAPPDATA` e `FK_ESTADO` próprios em
pasta temporária e com a trava `scratchpad\mouse.lock`. Relatório da rodada 1:
[teste-mouse-2026-11-13.md](teste-mouse-2026-11-13.md).

- Binário: release do worktree do B451 (develop com B363, B433 a B436, B446 a B450), 0.1.0-alpha.36.
- Tela: 1920x1080, janela maximizada, área cliente 1920x1009.
- Capturas das falhas: `docs/validacao/capturas-mouse/r2_*.png`.

**A rodada parou no meio.** Durante a volta da peça para a montagem, o Fenkra perdeu o foco por alguns
segundos: uma janela de erro de outro programa da máquina (`b455_antes.exe`, falta `TKDEIGES.dll`)
ficou por cima da vista, e outra janela tomou a frente. A regra era parar ao perder o foco. Desenho
e Reancorar ficaram sem fazer.

## Resumo

| # | Caso | Resultado |
|---|---|---|
| 1 | B446: arrastar na árvore em todas as pastas | ok |
| 2 | B449: setas na árvore depois do clique e redesenho imediato | ok |
| 3 | B448: nota com várias setas, duplo clique na seta, menu Mais | ok |
| 4 | Layout B433: coluna esquerda e seta da árvore | **falhou em parte**: a seta funciona; o painel do Inserir peça abre à direita |
| 5 | Layout B434/B435: topo fino, recursos à direita, painel enxuto | ok |
| 6 | Layout B436: barras por tipo e seta de variantes | **falhou em parte**: a seta funciona; uma variante tem termo em inglês copiado |
| 7 | B363: abas de documentos, Ctrl+Tab, fechar pergunta só pela editada | ok, com defeito: abas com o mesmo nome |
| 8 | Biblioteca em furo | **falhou em parte**: insere com restrição; ver casos 9 e 10 |
| 9 | Biblioteca em ponto | ok, com o mesmo efeito colateral de seleção |
| 10 | B450: inserir da biblioteca sem arquivo pede salvar | não feito (a montagem já estava salva) |
| 11 | B450: voltar da peça não rearma | **falhou em parte**: não rearma, mas o painel da Biblioteca fechado reabre |
| 12 | Reancorar restrição perdida | **não feito**; achado no caminho: o fixador em furo não acusa perda |
| 13 | Desenho: esboço na folha | não feito |
| 14 | Desenho: balões automáticos | não feito |
| 15 | Desenho: Cancelar na barra de progresso | não feito |
| — | Achados fora do roteiro | extrusão nova acusa referência perdida; Salvar como recusa documento novo |

## 1. B446: arrastar na árvore (ok)

Exemplo "Placa paramétrica", "Grupos vazios" e "Parâmetros" expandidos.

| Origem → destino | Resultado |
|---|---|
| Furos M6 → pasta Acabamentos | ok: contorno na pasta durante o arraste; entrou na pasta |
| L → sobre margem (Parâmetros) | ok: L passou para depois de margem |
| PlanoTopo → Corpo principal | ok: dica "Não dá para soltar aqui: Geometria de r…"; o corpo não fica selecionado |
| Furos M6 → Capturas | ok: "Não dá para soltar aqui: Este lugar não guarda este item"; Capturas não fica selecionado (o realce some ao afastar o mouse) |
| PlanoTopo → Publicações | ok: PlanoTopo publicado, Publicações (1) sobe para o topo |
| Placa → Acabamentos | ok: entrou na pasta, Centros dos furos ficou antes dela |

## 2. B449: setas e redesenho (ok)

Com `FK_CUTUCAR=0` (o equipamento não mexe o cursor antes da captura). Clique em Placa; ↓ ↓ andam
Placa → Furos M6 → Filete1; → gira a vista; ↑ volta para Furos M6 sem mexer a vista; Esc; ↓ gira a
vista. Cada captura já mostra o resultado da tecla, sem movimento do mouse.

## 3. B448: PMI (ok)

- Mais › Nota: o menu fecha e o painel Nota abre na coluna com Texto e Alvos, sem as propriedades da
  Placa selecionada antes. A árvore recolhe na seta "Árvore".
- Ctrl, Ctrl e Shift em três alvos: a nota não nasce no primeiro clique; a lista mostra os alvos
  ("face plana", "aresta circular Ø9"). Dois cliques na mesma face contam uma vez só. OK cria a
  Nota 1 com duas setas.
- Duplo clique na seta sobre a face do topo: abre a Nota 1 com a Seta 2 expandida, não a extrusão.
- Mais › Nota, dois alvos com Ctrl e Enter: Nota 2 com duas setas.
- Mais › Zebra: o menu fecha; o painel da zebra encaixa à direita e a vista encolhe.

Observação: a edição da nota existente abre embaixo da árvore, na mesma coluna, e não no lugar dela
como o painel de comando; a última linha ("Adicionar seta…") fica rente à barra de baixo.

## 4 a 6. Layout novo

- Topo de ~26 px com menu e bancadas em ícones; recursos só com ícones à direita, agrupados por
  separadores; barra de baixo com navegação e "recalculado em … ms"; cubo pequeno no canto. ok.
- Extrusão nova: o painel toma a coluna, só com Perfil, Operação e Extensão (o bloco comum não
  aparece). Clicar em "▸ Árvore" mostra a árvore com Concluir/Cancelar à vista ("voltar a
  Extrusão · Extrusão1"); clicar de novo volta aos campos. Cancelar devolve a coluna à árvore e
  apaga a Extrusão1. ok.
- Seta ▾ ao lado da Extrusão: lista as quatro variantes com ícone. ok. **Falha**: a quarta variante
  se chama "Extrusão com saída e filetes (drafted filleted pad)". O parêntese é o nome do comando
  num software de modelagem CAD avançado, e a regra do projeto proíbe copiar esses termos.
  Captura: `r2_02_variante_termo_em_ingles.png` (texto em `toolbar.rs` e `i18n.rs`).
- **Falha**: na montagem, o painel do Inserir peça abre encaixado à direita, e a coluna da esquerda
  segue com a árvore. Pelo ADR 0094, o painel de comando abre na coluna. Captura:
  `r2_03_inserir_peca_painel_a_direita.png`.

## 7. B363: documentos em abas (ok, com defeito)

- Com uma peça aberta não há barra de abas; Arquivo › Nova peça de exemplo abre a segunda e a barra
  aparece. ok.
- Ctrl+Tab troca de aba e volta com notas, seleção e vista de antes. ok.
- Apagar Filete1 na segunda aba põe "•" só nela. Fechar essa aba pergunta "Salvar as alterações?"
  listando só ela; Descartar fecha. Uma aba nova sem edição fecha sem perguntar. ok.
- **Defeito**: as duas abas mostram "Placa de exemplo", inclusive a que já foi salva como
  `placa.fkp`. A dica da aba não mostra o arquivo. Captura: `r2_07_abas_com_mesmo_nome.png`.

## 8 a 11. Montagem e biblioteca

Preparação: a peça foi salva com Ctrl+S em `placa.fkp`; Montagem › Nova montagem; Inserir peça; dois
cliques na vista (a primeira fixa na origem); Ctrl+S gravou `conj.fka`.

- **Em furo** (ISO 4762, M6, "Pelo furo"): o parafuso fantasma segue o cursor e mostra
  "M6x12 → placa.2 Ø6,6" sobre o furo; o clique insere com Coincidência, Trava de giro e Contato.
  ok.
- **Em ponto**: "M6x25 → ponto" sobre a face; o clique insere o parafuso de pé na face, solto (sem
  restrição, "1 soltos"). ok.
- **Defeito** nos dois: o clique que insere também seleciona a face da placa embaixo e abre o gizmo
  de mover a instância. Captura: `r2_09_inserir_fixador_seleciona_face.png`.
- **Defeito**: no painel da Biblioteca, a linha "Comprimento" cobre o cabeçalho "Tamanho" da tabela, e
  a fileira de filtros fica cortada pela borda da busca (já visto na rodada 1). O painel não cresce:
  arrastar o canto rola o conteúdo. Captura: `r2_05_biblioteca_cabecalho_sob_comprimento.png`.
- **B450, voltar da peça**: Editar a peça › Voltar não rearma Inserir peça nem a biblioteca. ok.
  **Defeito**: o painel da Biblioteca, fechado antes, reabre ao voltar. Captura:
  `r2_08_biblioteca_reabre_ao_voltar.png`.
- B450 "sem arquivo pede salvar": não conferido nesta rodada, porque a montagem foi salva antes.

## 12. Reancorar (não feito; achado)

Para perder uma referência, apaguei Furos M6 na peça (Editar a peça), salvei e voltei. As faces dos
furos sumiram, e os parafusos ficaram no ar na última posição. **Falha**: as três restrições do
parafuso inserido "em furo" continuam "OK" na árvore e no painel, sem ⚠ nem o botão "Reancorar". O
motivo provável: `releitura::perdida` só marca referências guardadas como `Viva::Face` ou
`Viva::ArestaNomeada`, e o fixador em furo guarda a referência de outro jeito. Captura:
`r2_06_fixador_sem_furo_restricao_ok.png`.

Depois disso, o Ctrl+Z na peça reaberta não devolveu os furos, porque o histórico de desfazer não
sobrevive a reabrir a peça. O teste do Reancorar ficou para a próxima rodada, com uma restrição por
face criada à mão.

## Achados fora do roteiro

- **Extrusão nova acusa referência perdida**: Sólidos › Extrusão cega, sem perfil escolhido, já abre
  com "✖ referência perdida: sketch #18446744073709551599 não existe — reedite ou substitua a
  referência (E-REF-001)". O número é um id sentinela perto de u64::MAX, mostrado como se fosse
  um sketch apagado. Captura: `r2_01_extrusao_nova_referencia_perdida.png`.
- **Salvar como recusa documento novo**: Arquivo › Salvar como… numa peça nunca salva só mostra
  "Salve o documento uma vez antes de usar Salvar como com referências" (`app/projeto.rs`). O Ctrl+S
  abre o diálogo normalmente. Captura: `r2_04_salvar_como_recusa_documento_novo.png`.
- Menu principal: o submenu segue o cursor. Ir de Arquivo direto em diagonal para um item passa por
  Exibir e troca o submenu (Barras de ferramentas). Comportamento comum, registrado só para o roteiro.
- A miniatura do Inserir peça e o nome da instância mostram "Placa de exemplo" (nome do documento),
  não `placa` (arquivo).

## Itens de backlog propostos

1. Extrusão (e todo comando novo) sem entrada escolhida não mostra "referência perdida"; o id
   sentinela não aparece na interface.
2. Trocar "(drafted filleted pad)" por um nome nosso, em português e na tradução, e revisar as
   outras variantes atrás de termos copiados.
3. Inserir peça (e os painéis de comando da montagem) abre na coluna da esquerda, como manda o ADR 0094.
4. Arquivo › Salvar como… num documento nunca salvo abre o diálogo, como o Ctrl+S.
5. Biblioteca: linha "Comprimento" fora do cabeçalho da tabela, filtros fora da busca, e o painel
   usa a altura livre da coluna ou da borda.
6. Fixador inserido em furo: quando o furo some, as restrições ficam marcadas como perdidas e oferecem
   Reancorar.
7. Inserir da biblioteca (furo e ponto): o clique que coloca não seleciona a face nem abre o gizmo.
8. Abas (B363): mostram o nome do arquivo quando há, desempatam nomes iguais e trazem o caminho na dica.
9. Voltar da peça para a montagem não reabre o painel da Biblioteca que a pessoa fechou.

## Equipamento

- `fk.py mover`: o caminho começa dentro da janela quando o cursor está fora da área cliente (outro
  monitor). Antes, os pontos intermediários caíam fora e o clique era recusado.
- `fk.py mover`: um ponto intermediário sob a janela de outro programa é pulado. Antes, o comando
  inteiro abortava, mesmo com o destino livre.
- O bloqueio do worktree recusa comandos de shell que contenham a sequência "git", e `digitar` a
  contém. Texto se digita por um roteiro JSON (`{"acao": "digitar", ...}`).
- Caminho com acento ("João") digitado no diálogo do sistema dá "nome de arquivo não é válido"; use uma
  pasta só com ASCII para os arquivos do teste.

## O que falta para a rodada 3

Desenho (esboço na folha, balões automáticos, Cancelar na barra de progresso), Reancorar com uma
restrição por face, e B450 "sem arquivo pede salvar". Rodar com a máquina sem outros programas
abrindo janelas.
