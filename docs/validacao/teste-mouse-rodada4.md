# Teste com mouse e teclado reais, rodada 4 (B465 a B476, B20, B40), 15/11/2026

Mesmo método da [rodada 3](teste-mouse-rodada3.md): mouse e teclado do Windows (`SendInput`) pelo
equipamento `scripts/teste_interface/fk.py`, sem `FENKRA_DEMO`, com `APPDATA`/`LOCALAPPDATA` e
`FK_ESTADO` próprios em pasta temporária e a trava `scratchpad\mouse.lock`. Antes do mouse, 30 s de
amostra de foco e cursor sem movimento de outra pessoa. Cada passo foi conferido por captura antes do
seguinte. O Fenkra testado foi aberto e fechado pelo próprio PID.

- Binário: release de `70c849f` (todos os itens desta rodada já estão nele), compilado de uma cópia
  em `C:/cargo-target/mouse4`, apagada no fim. O `develop` em `4d4cbea` (B63) não compila o
  `fenkra-app`: `biblioteca_ui.rs:731`, o `match ficha.tipo` não cobre `Pino`, `Anel` e
  `PinoComColar`.
- Tela: 1920x1080, janela maximizada, área cliente 1920x1009.
- Capturas: `docs/validacao/capturas-mouse/rodada4/`.

## Resumo

| Item | Caso | Veredito |
|---|---|---|
| B466 | Varredura cônica por 3 guias | **ok** |
| B469 | Dobra cega e por linha: menu de posição e linha no meio | **ok** |
| B470 | Prévia da dobra cônica mostra o cone | **ok** |
| B476 | Salvar como aberto pela biblioteca vem para a frente | **ok** |
| B465 | Painel novo sem "referência perdida" | **falha** (Rosca) |
| B467 | Plano escondido não é selecionado | **ok** |
| B468 | Esc limpa a seleção antes de fechar o comando | **ok** |
| B471 | Abas: Editar a peça e exemplo sobre aba editada | **ok em parte** |
| B472 | Balões avisam quando a origem é uma peça | **ok** |
| B473 | Vistas cabem na A3 | **ok** |
| B474 | Painel aberto depois do Reancorar | **não conferido** |
| B20 | Arco do ângulo da Saída e seta do comprimento da Rosca | Rosca **ok**; Saída **falha** |
| B475 | Variantes de mover em português | **ok** |
| B40 | Painéis da varredura cônica e da dobra cega na coluna esquerda | **ok** |

## B466, varredura cônica por 3 guias (ok)

Exemplo Suporte de chapa › Superfícies › Varredura ▾ › Varredura avançada › Definição › Tipo
"Cônica por 3 guias" (`b466_conica_3_guias_painel.png`).

- ▶ começa na Guia 1. O clique na borda de trás da placa preenche a Guia 1, e o ▶ passa para a Guia 2
  (`b466_clique_preenche_guia1.png`). Clique em face não preenche guia, como deve.
- A lista ▼ oferece os eixos da Origem, as curvas do Sketch1 e as arestas do corpo, com o comprimento
  (`b466_lista_curvas_e_arestas.png`). Uma curva escolhida na lista preenche a Guia 2 e o ▶ vai para a
  Guia do ápice (`b466_guia2_pela_lista.png`).
- ✖ esvazia a Guia 2 e o ▶ volta para ela (`b466_x_esvazia_guia2.png`).
- Com Guia 1 = borda da bainha, Guia 2 = topo da parede e ápice = topo da aba dobrada, a prévia mostra
  a superfície e "Solução 1 de 2" (`b466_tres_guias_previa.png`). Concluir cria Superficie1 em
  Geometria (`b466_aplicada.png`).
- Observação: a lista não destaca no 3D o item sob o mouse, então escolher "aresta 2 (30 mm)" pelo nome
  é às cegas. Pegar aresta pelo clique no 3D é difícil quando ela não está na silhueta: o clique cai na
  face vizinha.

## B469, dobra cega e por linha (ok)

Suporte de chapa › Dobra ▾ (ícone de linha sobre a chapa) › Dobra por linha.

- Até clicar, nada é aplicado e o balão diz o que fazer (`b469_nada_aplicado_ate_clicar.png`).
- Com "dobra cega" marcada, o clique na face fixa cria a linha (50; 48) → (50; 12), no meio da face,
  com ângulo, raio e minibarra (`b469_linha_no_meio_da_face.png`).
- O menu de posição da minibarra abre (`b469_menu_posicao_aberto.png`); um clique no 3D vazio fecha o
  menu e chega à vista, limpando a seleção (`b469_clique_fora_fecha_menu.png`). Depois de escolher
  "Início da dobra", o clique numa face volta a selecionar (`b469_3d_responde_apos_menu.png`).
- Concluir: só a faixa entre as pontas da linha gira, com os furos de alívio nas pontas
  (`b469_dobra_cega_aplicada.png`).
- Observação: o campo "Furo de alívio" vazio mostra ⚠ vermelho, mas a dica diz que vazio usa o mínimo
  aeronáutico; o ⚠ não tem tooltip.

## B470, prévia da dobra cônica (ok)

Dobra ▾ › Dobra cônica: a prévia mostra o tronco de cone transparente sobre a peça, não mais a caixa
(`b470_previa_tronco_de_cone.png`). O GIF de tooltip (`docs/media/saida/sm_dobra_conica.gif`) foi
gravado nesta rodada pelo fluxo do B60.

## B476, Salvar como pela biblioteca (ok)

Montagem nova sem arquivo › Biblioteca › ISO 4762 › M6: o diálogo Salvar como do Fenkra abriu na
frente da janela principal; a janela em foco era o diálogo, do mesmo PID
(`b476_salvar_como_na_frente.png`, recortada sem o painel de pastas do Windows). Salvo, a aba vira
`conj4`.

## B465, painel novo sem "referência perdida" (falha)

Sem o erro: Varredura avançada (`b465_varredura_nova_sem_referencia_perdida.png`), Estampo
(`b465_estampo_sem_referencia_perdida.png`), Dobra por linha e Saída.

**Falha na Rosca.** Passos: Arquivo › Nova peça de exemplo › barra de sólidos › Rosca. O painel abre
com "Prévia inválida: referência perdida: rosca 'Rosca1' — a face que passava perto de (0.00, 0.00,
0.00) não existe mais" e E-REF-001, antes de qualquer clique (`falha_b465_rosca_referencia_perdida.png`).
A barra de cima ainda diz "1 escolhida(s)", sobra de outro comando.

## B467, plano escondido (ok)

Barra de baixo › filtro "Mostrar na viewport" › desmarcar Planos da Origem
(`b467_planos_desligados.png`). O clique onde estava o plano XY não seleciona nada
(`b467_plano_oculto_nao_seleciona.png`).

## B468, Esc em níveis (ok)

Na varredura avançada: Esc com a lista do campo aberta fecha só a lista
(`b468_esc_fecha_so_a_lista.png`); Esc com uma face selecionada limpa a seleção e o comando continua
aberto (`b468_esc_limpa_selecao.png`).

## B471, abas (ok em parte)

- **ok**: com a aba "Suporte de chapa •" editada, Tela inicial › Placa paramétrica abre o exemplo em
  aba nova, e a aba editada fica (`b471_exemplo_nao_substitui_aba_editada.png`).
- **ok**: Editar a peça pela montagem abre `placa4` no lugar da aba da montagem, com "Montagem ›
  placa4 ↩ Voltar"; Voltar retorna à `conj4` (`b471_editar_peca_abre_no_lugar.png`). Não houve aba
  duplicada "placa4 (2)".
- **Não conferido**: a peça já aberta em outra aba antes do Editar a peça. Arquivo › Abrir peça ›
  `placa4.fkp` não abriu aba nenhuma, sem aviso (ver achados).
- **Falha, caminho vizinho**: Tela inicial › Novo desenho substituiu a aba "conj4 •", que tinha
  alteração não salva. Passos: montagem salva, Editar a peça, apagar um recurso, Ctrl+S, Voltar (a aba
  fica "conj4 •"); Arquivo › Tela inicial › Novo desenho. A aba da montagem some, sem perguntar
  (`falha_novo_desenho_substitui_aba_editada.png`).

## B472, balões com origem peça (ok)

Desenho com origem `placa4.fkp` › Balões auto: o diálogo mostra "Balões automáticos precisam de uma
montagem como origem do desenho" e a janela volta a "a vista toda" (`b472_aviso_origem_peca.png`).
Observação: com o painel Edição aberto, a folha fica cortada em x ≈ 915 e o resto da área fica vazio.

## B473, vistas na A3 (ok)

Novo desenho com origem `placa4.fkp`, escala automática: Frente, Superior e Lateral esquerda a 1:1,
todas dentro da margem da A3 (`b473_vistas_cabem_na_a3.png`). Este modelo não gerou isométrica.

## B474, painel aberto depois do Reancorar (não conferido)

Montagem `conj4` com placa4 e um ISO 4762 M6x25 ligados por Coincidência (vértice do parafuso ↔
aresta circular do furo). Editar a peça › apagar Furos M6 › Ctrl+S › Voltar. A restrição **não** ficou
perdida: o estado continua OK e o parafuso foi para a aresta do canto da placa, sem aviso
(`falha_b474_referencia_religada_sem_aviso.png`). Sem referência perdida, o botão Reancorar não
aparece, então B474 não pôde ser conferido. A religação silenciosa vai como achado.

## B20, alças da Saída e da Rosca

- **Rosca (ok)**: com a face do furo escolhida, a seta "Comprimento 15 mm" aparece no 3D, com as marcas
  Início e Fim (`b20_rosca_seta_comprimento.png`).
- **Saída (falha)**: na Placa de exemplo, face lateral da frente + plano neutro, a prévia fica sempre
  inválida ("a face não aceita ângulo de saída... cruzar o plano neutro"), com o topo (PlanoFace1),
  Origem·YZ e Origem·XY. O anel do ângulo aparece vermelho, com "3°", mas o arco não é desenhado
  (`falha_b20_saida_previa_invalida.png`). Passos: Placa de exemplo › Saída › clique na face lateral
  da frente › Escolher o plano neutro no 3D › clique no topo.
- **Defeito na Saída**: com "Campo ativo: Plano" no painel, o clique no topo entrou como Face 2, não
  como plano neutro. Foi preciso o botão "Escolher o plano neutro no 3D"
  (`falha_saida_plano_neutro_entra_como_face.png`).

## B475, variantes de mover (ok)

Mover ▾: "Mover o corpo (translação)", "Girar o corpo (rotação)", "Simetria do corpo (reflexão)",
"Afinidade (escala por eixo)" (`b475_variantes_mover.png`).

## B40, painéis na coluna esquerda (ok)

Varredura cônica e dobra cega abrem na coluna esquerda com Concluir/Cancelar. Dica por campo vista em
Seções (`b466_conica_3_guias_painel.png`) e no texto do furo de alívio
(`b469_linha_no_meio_da_face.png`). O ícone da dobra cônica substitui o da família na barra depois de
usado (`b470_previa_tronco_de_cone.png`).

## Achados fora do roteiro

1. Rosca abre com "referência perdida" perto de (0,0,0) (B465 não cobriu a Rosca).
2. Saída: prévia sempre inválida na Placa de exemplo com plano neutro no topo, em XY ou em YZ; e o clique
   com "Campo ativo: Plano" entra como face.
3. Coincidência religada a outra aresta quando a referência some, sem aviso nem ⚠.
4. Novo desenho pela tela inicial substitui uma aba com alterações não salvas.
5. Arquivo › Abrir peça com a aba da montagem na frente não abre o `.fkp` escolhido e não avisa.
   "Abrir recente" também não lista a peça salva por Ctrl+S.
6. Biblioteca "Em furo" sem furo compatível: entra solto na origem e o fantasma continua, com o aviso
   "nenhum furo casa" por cima do modelo.
7. Termos em inglês ainda na interface: tooltip "Rosca em eixo ou furo (Thread/Tap)" e o item "Joggle
   (desvio em Z)" no menu Dobra.
8. Janela › "Nesta janela: placa4.fkp" com a aba `conj4` na frente.
9. A tabela de tamanhos da biblioteca fica escondida até aumentar a janela.
10. O ícone de Grupo da montagem fica ao lado do de Biblioteca; Ctrl+Z não desfez o Grupo criado.

## Equipamento

- Em menu do egui, clicar no item de nível 1 (Arquivo) com o submenu já aberto fecha o menu; o clique
  seguinte cai na árvore. Passar o mouse no item e andar na horizontal até o submenu resolve.
- O diálogo Salvar como abriu antes do clique em "Adicionar à montagem" (já no clique em M6 ou em
  Solto); o `fk.py` esperou o foco e trouxe a janela principal para a frente, mas o diálogo continuou
  por cima dela.
