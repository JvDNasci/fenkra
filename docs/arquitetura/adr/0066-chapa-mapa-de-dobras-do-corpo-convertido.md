# ADR 0066 — Chapa: mapa de dobras do corpo convertido, faixa válida no solver e recorte sem mapa

## Contexto

O ADR 0048 deixou três pontas soltas na bancada Chapa metálica. O comando Converter em chapa reconhecia
espessura e dobras mas não registrava mapa de dobras, então a tabela, as linhas de dobra e a planificada
não existiam sobre o corpo convertido. O painel da aba pintava de vermelho um valor fora da faixa, mas o
recálculo aceitava o valor e o selo do comando ficava em OK. E um recorte desenhado com a chapa dobrada
podia cair no ar na planificada, sem aviso, quando a parte da chapa onde ele cai não tem mapa de dobras.

## Decisão

- **O corpo convertido ganha `SheetMap` por dobra reconhecida** (`chapa_converte_mapa.rs`), o mesmo
  registro das abas, para tudo que já lê esse registro (tabela, linhas de dobra, `map_point`, `map_tool`)
  valer sem código novo. A topologia sai só da malha: cada trecho plano é um nó, identificado pelo plano
  médio da espessura; cada dobra liga os dois nós cujas faces principais são tangentes ao cilindro interno.
  Uma busca em largura a partir da face fixa (ou da maior face plana) dá a árvore; um laço fechado de
  dobras é recusado em palavras. A face de referência de cada dobra é a do lado do centro de curvatura, e
  o ângulo é sempre positivo para esse lado; subir ou descer vem do giro acumulado até a base.
- **A planificada do corpo convertido tira o arco de cada dobra**, gira as pernas de volta ao plano do pai
  (da última dobra para a primeira, como `map_tool`) e fecha o vão com uma tira do comprimento desenvolvido.
  O volume do resultado é conferido contra a conta (corpo menos arcos mais tiras) e o comando erra se não
  bate, em vez de mostrar uma peça errada. Corpo que já veio de comandos de chapa tem o mapa deles e não é
  mapeado de novo.
- **A faixa válida mora no solver** (`chapa_laco::ev_faixa`) e é a mesma que o painel usa para pintar o
  campo: comprimento reto de 0 mm ou mais (zero é a dobra pura, que outras abas usam como base), raio acima de 0,01 mm, ângulo de 0,01 a 180 graus, folga do canto de 0 a 10 mm,
  razão de sobreposição de 0 a 1. Fora dela o comando erra com o limite escrito e o selo vira ERRO.
- **Recorte levado a outro estado que não acha material é erro**, não silêncio (`apply_roles`): o status
  vira ERRO, o comando entra na lista de ausentes da planificada e a exportação para fabricação recusa até
  o recorte ser refeito no estado certo.
- **Estampo por punção na planificada** não amassa a chapa plana: a pegada do punção (a seção pelo plano da
  chapa) vira marca de ferramenta, como os outros estampos.

## Consequências

- A aba por perfil continua sem mapa de dobras próprio: a faixa dela é uma extrusão de cadeia de pontos, e o
  ponto de dobra entre a chapa e o primeiro trecho depende de para que lado a espessura cresce. Um recorte
  desenhado dobrado sobre ela cai no erro acima na planificada, com a instrução de refazê-lo planificado.
- Corpo convertido com orelhas mais largas que a dobra que as prende não se planifica (erro de volume).
- Perfil num plano inclinado em relação à chapa continua recusado na planificada, agora com o ângulo medido:
  a faixa extrudada pela normal do plano só prende na aresta quando essa normal fica no plano da chapa.
