# Teste com mouse e teclado reais, rodada 3 (B451, B363, B450), 15/11/2026

Um agente usou o Fenkra com mouse e teclado do Windows (`SendInput`), sem `FENKRA_DEMO`, pelo
equipamento `scripts/teste_interface/fk.py`, com `APPDATA`/`LOCALAPPDATA` e `FK_ESTADO` próprios em
pasta temporária e com a trava `scratchpad\mouse.lock`. Antes de começar, 30 s de amostra de foco e
cursor sem movimento de outra pessoa. Cada passo foi conferido por captura antes do seguinte.
Rodadas anteriores: [rodada 1](teste-mouse-2026-11-13.md), [rodada 2](teste-mouse-rodada2.md).

- Binário: release do `develop` em `a78ee81` (B40 SmCone, B393, B455, B464), target próprio
  `C:/cargo-target/mouse3`, apagado no fim.
- Tela: 1920x1080, janela maximizada, área cliente 1920x1009.
- Capturas: `docs/validacao/capturas-mouse/rodada3/`.

## Resumo

| # | Caso | Resultado |
|---|---|---|
| 1 | B451: Reancorar restrição perdida | **ok** |
| 1b | B451: desenho (esboço na folha, balões automáticos, Cancelar na barra) | esboço **ok**; balões **ok com defeito**; Cancelar **não visto** (vistas saem na hora) |
| 2 | B363: abas (nomes distintos, trocar, fechar, reabrir) | **ok**, com dois defeitos (aba velha do mesmo arquivo; aba editada substituída) |
| 3 | B450: montagem sem arquivo pede salvar | **ok** |
| 4a | B40: Varredura cônica por 3 guias | **falhou em parte**: painel ok; não consegui preencher as guias pelo mouse |
| 4b | B40: Dobra cônica | **ok** |
| 4c | B40: Dobra cega | **ok com defeito**: painel e aplicação ok; a linha padrão gira a peça inteira |
| 5 | B464: Deslocar face em peça com boss e filete | **ok** (sem o erro de sólido aberto), com observação |

## 1. B451: Reancorar (ok)

Preparação: exemplo Placa paramétrica salvo como `placa2.fkp`; montagem nova; Inserir peça (o painel
agora abre na coluna esquerda, corrigindo o achado da rodada 2); Biblioteca › ISO 4762 M6 › Em furo:
o fantasma mostra "M6x12 → placa2 Ø6,6" e o clique insere com Coincidência, Trava de giro e Contato,
**sem** selecionar a face embaixo (corrigido). Montagem salva como `conj4.fka`.

1. Botão direito na instância › Editar a peça: abre `placa2` em aba; apaguei Furos M6 (Del); Ctrl+S;
   Voltar à montagem.
2. Coincidência e Trava de giro ganham ⚠ na árvore (a rodada 2 não acusava). Clique na Coincidência:
   painel mostra estado **PERDIDA**, "⚠ Referência perdida: placa2" e o botão "⊙ Reancorar na seleção
   atual" desligado. Captura `b451_referencia_perdida.png`.
3. Clique na parede curva do rasgo: a face fica selecionada, mas o painel da restrição fecha. Clicar de
   novo na Coincidência mostra o botão ativo (`b451_reancorar_ativo.png`).
4. Reancorar: estado **OK**, o parafuso vai para o eixo do arco do rasgo e o ⚠ some da Coincidência;
   a Trava de giro segue com ⚠, como deve (`b451_reancorado.png`).

Observação: o clique no 3D fecha o painel da restrição; o fluxo pede reabrir a restrição depois de
escolher a face. Seria mais direto o painel continuar aberto.

## 1b. B451: desenho técnico

- **Esboço na folha (ok)**: bancada Desenho › 3 vistas da `placa.fkp` › Esboço › clique fora das
  vistas: "Esboço solto na folha". Polilinha de três pontos, botão direito termina; duas retas com
  restrições automáticas e "Resolvido"; Concluir cria "Esboço 1" na árvore
  (`b451_esboco_na_folha.png`).
- **Balões automáticos (ok com defeito)**: com origem de peça, Confirmar não faz nada e não avisa.
  Com a origem trocada para `conj4.fka` (Escolher arquivo…) e 3 vistas da montagem, Confirmar põe os
  balões 1 e 2 na vista Frente da montagem (`b451_baloes_automaticos.png`).
  - **Defeito**: a janela de dois cliques fica guardada entre usos. Na segunda abertura, sem perceber
    a janela velha, Enter confirmou e nada aconteceu. Foi preciso "Limpar janela".
  - **Defeito**: com origem de peça, Confirmar fecha sem aviso. Falta dizer "use uma montagem".
- **Cancelar na barra de progresso (não visto)**: vistas da peça e da montagem de uma instância saem
  em menos de um quadro, e a barra não aparece. Precisa de um modelo pesado.
- **Defeito**: as vistas automáticas passam da margem da folha A3 (isométrica à direita, superior
  embaixo; `falha_vistas_saem_da_folha.png`). Vale para peça e montagem, com escala 1:1 automática.

## 2. B363: abas (ok, com defeitos)

- Arquivo › Nova peça de exemplo duas vezes: abas "Placa de exemplo" e "Placa de exemplo (2)"
  (corrigido) (`b363_abas_nomes_distintos.png`). Depois de Ctrl+S, a aba mostra `placa2`; a dica mostra
  tipo, caminho e estado (`b363_dica_da_aba.png`).
- Clique na aba e Ctrl+Tab trocam de documento. ok.
- × numa aba salva fecha sem perguntar; Janela › Abrir recente › `placa2.fkp` reabre. ok.
- **Defeito**: com `placa2` aberta numa aba, "Editar a peça" pela montagem abre **outra** aba do mesmo
  arquivo ("placa2 (2)"). Depois de apagar os furos e salvar nela, a primeira aba continua com os furos
  e diz "sem alterações por salvar" (`falha_aba_velha_com_furos.png`). Salvar a velha apaga a edição.
  Esperado: reaproveitar a aba já aberta, ou recarregar/avisar quando o arquivo mudar.
- **Defeito**: abrir um exemplo pela tela inicial com a aba da frente editada ("Suporte de chapa •",
  com dobras não salvas) substituiu essa aba sem perguntar. A edição se perdeu.

## 3. B450: sem arquivo pede salvar (ok)

Montagem nova sem arquivo › Biblioteca › família › "Solto": abre o diálogo Salvar como do Fenkra
(`b450_solto_pede_salvar.png`). Salvo, o item entra ("Montagem salva", "Inserido como peça";
`b450_depois_de_salvar_insere.png`). Observação: o diálogo abriu atrás da janela principal. O
equipamento só o achou listando as janelas do processo.

## 4. Painéis novos do B40

Os três abrem na coluna esquerda com Concluir/Cancelar (ADR 0094). Dica por campo confirmada em Seções
(varredura), Raio da boca menor (dobra cônica) e no aviso do furo de alívio. O ícone da Dobra cônica
existe e substitui o da família na barra depois de usado.

### 4a. Varredura cônica por 3 guias (falhou em parte)

Superfícies › Varredura ▾ › Varredura avançada › Definição › Tipo "Cônica por 3 guias": campos Guia 1,
Guia 2, Guia do ápice, ρ (ou raio na guia 1 para G2), spine e seções (`b40_varredura_conica_painel.png`).
Não consegui preencher pelo mouse:

- **Defeito**: o comando novo já abre com "Prévia inválida: referência perdida
  (#18446744073709551603)", o mesmo id sentinela da rodada 2, agora na varredura
  (`falha_varredura_referencia_perdida_sentinela.png`).
- **Defeito**: o primeiro clique numa aresta preencheu "Guia do ápice", não a Guia 1. Os cliques
  seguintes em outras arestas não preencheram nada. A lista ▾ dos campos só oferece os eixos da
  Origem.
- **Defeito**: com Planos da Origem desligados em Exibir, o clique ainda seleciona o plano XZ
  escondido (`falha_plano_oculto_selecionavel.png`).
- **Defeito**: Esc com a lista do campo aberta, ou com uma face selecionada, fecha o comando inteiro
  (a barra diz "Esc limpa").
- Os campos e a ajuda ficam cortados na borda direita da coluna.

### 4b. Dobra cônica (ok)

Suporte de chapa › Dobra ▾ › Dobra cônica: raios 20/40, altura 50, centro, fator K próprio. A prévia
3D é só uma caixa envolvente, sem o cone (`b40_dobra_conica_painel.png`). Concluir cria o tronco de
cone. A árvore mostra ⚠ com "fator K 0.44: o volume da planificada difere do dobrado em 0.4%", aviso
informativo (`b40_dobra_conica_aplicada.png`).

### 4c. Dobra cega (ok com defeito)

Dobra ▾ › Dobra por linha › "dobra cega": aparece Furo de alívio com dica do mínimo aeronáutico
(`b40_dobra_cega_painel.png`). Concluir aplica, mas a linha padrão (0,0)→(0,10) fica na ponta da peça
e uma faixa inteira gira para baixo (`falha_dobra_cega_linha_padrao.png`).

- **Defeito**: com o comando aberto, o menu "Linha de centro / Início / Fim / Vértice externo" da
  minibarra fica aberto sobre a vista e segura os cliques no 3D. Não deu para escolher a face fixa nem
  desenhar a linha, então a dobra saiu com a linha padrão.

## 5. B464: Deslocar face com boss e filete (ok, com observação)

Exemplo Flange (disco + cubo central por extrusão unida) › Filete ▾ › Filete nas arestas selecionadas
› Todas, raio 1 › Concluir. Mais › Mover face › operação "Deslocar face" › clique no topo do cubo
(Face 1 em z = 28) › Distância 3 › Concluir: "Mover face1" sem erro, sem "não deixou um sólido
fechado" (`b464_deslocar_face_previa.png`, `b464_deslocar_face_aplicado.png`).

Observação: como todas as arestas tinham filete, as faces formam uma cadeia tangente, e o deslocamento
se propagou pela peça toda (furos quase fechados, disco maior). Falta conferir com filete só na base do
cubo: neste teste o clique no 3D não pegou essa aresta (pegava a curva do Sketch2, depois parou de
responder).

## Achados fora do roteiro

- Variantes com termo em inglês copiado entre parênteses: "Mover o corpo (Translation)", "Girar o
  corpo (Rotation)", "Simetria do corpo (Symmetry)", "Afinidade (Affinity)"
  (`falha_variantes_termos_em_ingles.png`). Mesmo tipo de achado da rodada 2.
- Menu Mais › seta ▾ de "Mover face" fecha o menu sem mostrar as variantes.
- O filtro de seleção (faces/arestas/vértices) vale para todas as abas, e não há aviso de que Faces
  está desligado.
- Filete em todas as arestas do Flange: a borda externa sai serrilhada na malha exibida.
- O ícone de Chanfro fica ao lado do de Filete com desenhos parecidos. O clique no que parecia Filete
  abriu o Chanfro.

## Itens de backlog propostos

1. Comandos novos (varredura avançada incluída) sem entrada não mostram "referência perdida" com id
   sentinela.
2. Varredura avançada: o clique preenche o campo ativo, na ordem dos campos (Guia 1 primeiro); a lista
   do campo oferece as arestas e curvas do modelo; os campos cabem na coluna.
3. Planos e objetos escondidos não entram na seleção por clique.
4. Esc dentro de lista ou com seleção limpa só a seleção, sem fechar o comando.
5. Dobra por linha: o menu de posição da minibarra não segura os cliques no 3D; a linha padrão não fica
   na ponta da peça.
6. Dobra cônica: prévia com o cone, não só a caixa.
7. Abas: "Editar a peça" reaproveita a aba do mesmo arquivo; aba de fundo recarrega ou avisa quando o
   arquivo muda no disco; abrir exemplo nunca substitui aba com alterações.
8. Balões automáticos: avisar quando a origem não é montagem; a janela de dois cliques não fica
   guardada de um uso para o outro.
9. Vistas automáticas cabem na folha (escala automática considera as quatro vistas).
10. Reancorar: o painel da restrição continua aberto ao clicar a face nova.
11. Trocar os termos em inglês das variantes de mover, girar, simetria e afinidade.
12. Diálogo Salvar como aberto pela biblioteca vem para a frente.

## Equipamento

- O diálogo Salvar como aberto pela biblioteca ficou atrás da janela principal. O `fk.py` recusou o
  clique seguinte por falta de foco. Resolvido trazendo o diálogo do mesmo processo para a frente.
  Proposta: `fk.py dialogo` procurar o diálogo do processo mesmo fora de foco.
- `fk.py recorte` aceita a escala só como `--escala N`.
