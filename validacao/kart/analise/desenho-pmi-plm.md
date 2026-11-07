# Desenho, PMI, PLM e pacote de visualização (vis_des)

Base: capturas `0024`, `0094` a `0099`, `0131`, `0130/0115/0138` (variações só de cor, não abertas), `relatorio.md`, `resultados.md`, `bugs.md`. Tudo foi feito por código; a interface foi só o lugar de captura. PLM e pacote `.fkv` não têm captura no diário: só li os números de `resultados.md`.

## 1. O que foi usado e o que não foi

- Usado (Desenho): folha com carimbo, vista frontal e superior, corte, detalhe, planificada, tabela de furos, tabela de dobras, lista de corte, lista de materiais, 3 vistas em peça grande. 8 folhas exportadas.
- Usado (PMI): diâmetro, referência A, batimento, rugosidade, legenda 3D, nota, balão, STEP com PMI em 4 peças.
- Usado (PLM): PN, liberar revisão A (84 peças), revisão B com regressão, estado, onde usado, `.fkv` e página HTML.
- Não usado, e não vi: cotas de desenho (nenhuma folha tem cota), símbolos de solda, GD&T na folha, folhas múltiplas, vista de montagem com subconjuntos funcionando (K-03), explosão, comparação de revisões pela interface, nota de revisão. Motivo: o teste montou o documento em código e se concentrou em "calcula e exporta". Os resultados dizem "ok" sem conferir se a folha serve para fabricar.

## 2. O que deu certo

- Corte com hachura e linha de corte legíveis: `0097_dw-di08_folha.png` (manga, vista superior + corte C-C, carimbo preenchido com material, massa e escala).
- Planificada de chapa com linhas de dobra e tabela de dobras (ângulo, raio, desenvolvido, dedução): `0098_dw-po-04_folha.png`.
- Folha simples e limpa, boa escala 1:1, carimbo e zonas de referência: `0099_dw-ch09_folha.png`.
- Vistas de quadro tubular com lista de corte de 8 itens: `0094_dw-quadro_folha.png`.
- PMI: balão com letra "A" ancorado na superfície do eixo: `0024_ex-01_pmi.png`. STEP com PMI levou 3 de 3 anotações (relatório). PLM: 84 de 84 liberados em 2,6 s (relatório).

## 3. O que falhou

- Tabelas sobre o carimbo: lista de corte (`0094`), tabela de furos (`0096`), lista de materiais (`0131`) caem em cima do carimbo; a tabela de dobras sai pela borda inferior (`0098`). É K-19 (y conta de baixo), mas aparece em 4 de 8 folhas, então não é só dado do teste: não há aviso nem limite à folha.
- Vista do eixo estoura a folha: no `0095` a vista frontal passa do topo da moldura (escala 1:3,33 com o eixo longo) e o corte B-B fica minúsculo. O auto-ajuste de escala não considerou a folha inteira, só a largura.
- Detalhe A do quadro (`0094`) mostra dois traços dentro de um círculo grande e vazio; o círculo de origem na vista superior fica em área sem nada. Detalhe sem sentido de engenharia.
- Disco de freio (`0096`): a tabela lista furos A1 e A2 Ø6,5 passantes, mas a vista frontal não mostra nenhum furo. Pode ser dado do teste (furos em outro eixo), não confirmei qual dos dois.
- Folha do conjunto (`0131`): vistas se cobrem por inteiro e aparecem rótulos "Frente", "Lateral esquerda" e "Isométrica" no meio do desenho. É K-27 e K-03. Ilegível como desenho.
- PMI ilegível: legenda e nota na mesma âncora saem uma sobre a outra (`0024`, K-22). O diâmetro e o batimento da árvore não aparecem claros na captura; só vi a referência A e a legenda.
- Sem cotas em nenhuma folha: as folhas têm vistas, mas não dizem o tamanho de nada. Para o usuário, "ok" de desenho aqui é só "vistas e tabelas saíram".
- Etiqueta "Atualizada" aparece como caixa solta sobre cada vista (`0097`, `0099`, `0131`): ruído visual que vai parar até em captura.
- Pacote `.fkv`: 1170 s (K-17), falha por peça esqueleto (K-05) ou sem corpo (K-06). Sem imagem, só números.

## 4. Deu certo, mas dá para melhorar

- Carimbo: o campo VERIFICADO vem vazio e DATA tem formato pt-BR; bom. Faltaria ligar a revisão do PLM ao carimbo (aparece "A", mas não vi se muda para B).
- Nome do detalhe e do corte ("Detalhe A (1:2)", "Corte B-B (1:3,33)") repetem escala em árvore e em folha; a árvore está certa, mas o rótulo da vista "Frente (1:3,33)" sobrepõe o símbolo do corte no `0095`.
- Escala 1:3,33 não é padronizada; o ajuste automático deveria cair em 1:2, 1:5 etc.
- Árvore do Desenho tem só "Folha 1" e as vistas; poderia mostrar contagem de cotas, tabelas e problemas (tabela fora da folha, vista vazia).
- Para o usuário de 84 peças, 126 s para uma folha (K-15) pede barra de progresso e cancelamento; não vi nenhum dos dois.

## 5. Melhores imagens para o vídeo

1. `0097_dw-di08_folha.png`: vista superior e corte com hachura, a folha mais "de engenharia" do teste.
2. `0098_dw-po-04_folha.png`: planificação do assoalho com tabela de dobras (mostrar a ligação chapa e desenho).
3. `0099_dw-ch09_folha.png`: peça simples, folha limpa, carimbo completo.
4. `0094_dw-quadro_folha.png`: quadro tubular em vista frontal e superior, só se a tabela for corrigida; senão recortar a parte das vistas.
5. `0024_ex-01_pmi.png`: balão de referência A no eixo e a árvore PMI; só depois de afastar legenda e nota.
6. Evitar `0131`: serve para mostrar o problema, não o produto.

## 6. Regras para a skill

- Toda tabela e anotação criada em folha precisa ser limitada à área útil e afastada do carimbo, com aviso quando não couber.
- Escala automática deve considerar altura e largura da peça contra a folha e cair em escala padronizada.
- Desenho de montagem precisa de caminho para subconjuntos e peça esqueleto; vista vazia sempre gera aviso.
- Âncora de anotação duplicada deve ser afastada automaticamente, nunca sobreposta.
- Todo "ok" de desenho exige conferir cotas, furos mostrados na vista e tabela dentro da folha, não só exportação sem erro.

## 7. Itens novos de backlog

- Escala automática da vista leva em conta a altura e a folha inteira (eixo longo estoura a moldura): A.
- Tabela posicionada sobre o carimbo ou fora da folha emite aviso e se realoca para a área livre (generaliza K-19, visto em 4 folhas): A.
- Detalhe de vista sem geometria relevante emite aviso, ou o círculo se ajusta à peça: M.
- Conferir por teste que furos listados em tabela aparecem na vista (disco `0096`; causa não apurada): M.
- Remover ou esconder a etiqueta "Atualizada" nas capturas e na exportação: B.
- Progresso e cancelamento em desenho de conjunto e geração de `.fkv` (126 s e 1170 s): M.
- Escala em lista fixa de padrões (1:2, 1:5, 1:10) no ajuste automático: B.
