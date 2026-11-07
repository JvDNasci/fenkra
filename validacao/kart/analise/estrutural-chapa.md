# Estrutural e Chapa metálica: análise pós-kart (`vis_est`)

Olhei de verdade: 0002, 0003, 0007, 0008 (quadro), 0010, 0019, 0062, 0063, 0065, 0075, 0081 do `diario/`, e
`capturas/ch-08_painel_aba_pintado.jpg`, `capturas/po-04_planificado.jpg`, `capturas/quadro_topo.jpg`.
Não abri as demais peças de tubo/chapa (CH-11 a CH-14, PR-02, PR-04, PR-05, PO-05, PO-06 etc.); vale só o
número do `resultados.md` para elas. Também não vi captura de lista de corte, tabela de dobras nem de
símbolo de solda: só aparecem como texto em `resultados.md` e no desenho.

## 1. O que foi usado e o que não foi

- Usado, Estrutural: Membro (tubo Ø30 sobre caminho), cantos Dobra e Encaixe, lista de corte, Reforço
  (`StructGusset`), Cordão de solda (`StructWeld`). Peças de tubo: longarinas, travessas, arco, apoios do
  banco, para-choques, coluna, tirante, vareta.
- Usado, Chapa: Parede base, Aba (15), Furo, Padrão linear, Recorte, Estampo, Veneziana, Bainha, Desvio, Alívio
  de canto, Furo flangeado, Converter em chapa, planificação (via desenho com tabela de dobras).
- Não usado: perfis estruturais que não sejam tubo redondo (cantoneira, U, I, retangular), nenhuma biblioteca de
  perfis, simbologia de solda no desenho, aba entre perfis, dobra por linha visível nas capturas. Motivo: o kart
  é um quadro de tubo redondo e chapa simples; não pedia.
- Limite do teste: tudo foi montado em código. As capturas de tubo mostram a aba "Modelagem Sólida" ativa e a
  barra de ferramentas de sólidos, nunca a de Estrutural. Então a barra e os comandos de Estrutural pela
  interface não foram avaliados.

## 2. O que deu certo

- Tubo dobrado segue o esqueleto com raio de dobra suave, sem dobra quebrada: `0002_ch-01_longarina.png`,
  `0065_pr-01_pronta.png` (para-choque com 3 dobras limpas).
- Quadro inteiro de 8 tubos que parece o chassi de um kart, com longarinas dobradas, travessas e apoios do
  banco: `0007_quadro_completo.png`, `0008_quadro_topo.png`.
- Suporte de chapa dobrada com abas, raio interno, furo grande e 3 furos pequenos: `0010_ch-08_pronta.png`.
  Parece a peça real.
- Assoalho com 42 furos em padrão e duas abas laterais: `0062_po-04_pronta.png`.
- Painel de Aba (versão pintada) é claro: "Entradas completas", abas Suporte/Modo/Parâmetros/Avançado, prévia
  azul translúcida da aba no 3D e alças com rótulo (Raio 2 mm, Ângulo 90°, Comprimento 40 mm):
  `capturas/ch-08_painel_aba_pintado.jpg`.
- Estampo e veneziana, com furo flangeado, na mesma chapa: `0075_cv-chapa1_pronta.png`.
- O Assistente de reparo oferece Refazer referência, Suprimir e Apagar diante do erro do cordão:
  `0081_cv-est_pronta.png`.

## 3. O que falhou

- Captura "painel de Aba" do diário não mostra painel nenhum: `0019_ch-08_painel_aba.png` é visualmente idêntica a
  `0010_ch-08_pronta.png` (só muda o tempo de recálculo). Só a versão `capturas/..._pintado.jpg` mostra o painel.
  Idem `0003_ch-01_painel.png`: a legenda diz "Comando Membro aberto", a imagem mostra a peça sem painel. Não vi
  nenhuma captura do painel de Membro. Falha do método de captura, não do produto.
- Captura "planificado" não mostra planificação: `0063_po-04_planificado.png` e `capturas/po-04_planificado.jpg`
  mostram o assoalho dobrado, igual a `0062`. A planificação só foi conferida no desenho (`DW-PO-04`, tabela de
  dobras com 2 linhas). Não vi a chapa plana no 3D. Atenção: "planificado ok" no relatório não é visual.
- `CV-EST` terminou em FALHA no `resultados.md` ("os dois conjuntos de faces não se encontram"), e a captura
  `0081_cv-est_pronta.png` mostra a peça com erro. O relatório diz que o erro foram pontos errados do teste, e
  isso confere com a mensagem. Mas a captura que fica no diário é a do erro, e eu não vi nenhuma captura do
  cordão funcionando.
- O reforço (`Reforço1`) em `0081` aparece como um triângulo solto no cotovelo, sem encostar bem nos dois tubos
  de forma legível; não dá para afirmar se está soldado. Não verifiquei por medida.
- K-02 e K-09 do `bugs.md` se confirmam na imagem: o caminho precisou ser dividido em curvas próprias
  (`mestre_LONG_E_0..3`) e as pontas das travessas em `0008` mostram V e bicos diferentes entre si
  (`mestre_TR_MONT` e `mestre_BANCO_*` com corte reto contra a longarina).
- Rótulo da legenda 3D se sobrepõe a si mesmo ("FK-CH-08 Suporte do mancal … Aço …" ilegível) em todas as
  capturas. Já é K-22; só confirmo que atinge Estrutural e Chapa em todas as peças.

## 4. Deu certo, mas dá para melhorar

- Nome das ações do tubo na árvore: `0007` lista 21 referências externas, 19 curvas `mestre_*·1` e depois
  `CH-07 Apoio banco AE/AD/FE/FD`. A árvore vira lista longa; agrupar as curvas ligadas num só nó.
- Rótulos longos `mestre_LONG_E_1` em cima do tubo no 3D poluem a vista do quadro (`0007`, `0008`); sem
  eles o quadro ficaria limpo para o vídeo.
- Passos: um tubo é um Membro por trecho, e o quadro tem 10 comandos para 8 tubos. Um membro com caminho
  composto (polilinha ligada) reduziria isso; é o ponto do K-02.
- Chapa: o suporte do mancal tem Aba1, Aba2, Sketch2, Furo1, Sketch3, Furo2 na árvore (`0010`). Nomes
  genéricos; nomear a aba (esquerda/direita) ajuda a achar depois.
- Estrutural não aparece como bancada ativa nas capturas de tubo; vale uma captura da barra de Estrutural
  com Membro, Cantos, Lista de corte e Cordão para a demonstração.

## 5. Melhores imagens para o vídeo

1. `diario/0007_quadro_completo.png`: o quadro do kart de 8 tubos dobrados (melhor se tirar os rótulos `mestre_*`).
2. `diario/0008_quadro_topo.png`: vista de cima com os recortes das travessas nas longarinas.
3. `capturas/ch-08_painel_aba_pintado.jpg`: o painel de Aba com prévia e alças no 3D.
4. `diario/0010_ch-08_pronta.png`: o suporte de chapa dobrada com furos, pronto.
5. `diario/0062_po-04_pronta.png`: assoalho com 42 furos em padrão linear.
6. `diario/0075_cv-chapa1_pronta.png`: estampo, veneziana e furo flangeado numa só chapa.

Não usar: `0019`, `0063` (sem o que a legenda promete) e `0081` (estado de erro, só serve para mostrar o reparo).

## 6. Regras para a skill da bancada

- Capture o painel do comando depois de abrir o painel, não só a peça: a legenda não prova que o painel
  apareceu (0003 e 0019 saíram sem painel).
- Para mostrar planificação, o 3D deve estar na chapa plana; "tabela de dobras certa" no desenho não vale
  como captura da chapa planificada.
- Caminho de membro estrutural tem de ser testado com polilinha ligada por link, não só com curvas separadas.
- Corte entre membros: conferir todas as pontas do quadro na lista de corte, não só uma por tipo (K-09).
- Cordão de solda exige as duas faces se tocando; a mensagem deveria dizer qual conjunto de faces falhou.

## 7. Itens novos de backlog

- Captura em lote do diário sem painel aberto (`0003`, `0019`): conferir a abertura do painel antes de fotografar. Prioridade B.
- Captura 3D da chapa planificada (estado plano) e botão visível "Planificar" na barra de Chapa: nenhuma
  captura do diário mostra a chapa plana. Prioridade M.
- Mensagem do Cordão de solda dizer qual das duas seleções de faces não encontrou a outra ("conjunto A
  longe do B a N mm"). Prioridade B.
- Mostrar o reforço (`StructGusset`) com contorno ou cor de solda para ficar claro que encosta nos dois
  tubos. Prioridade B.
- Agrupar numa linha só as curvas `mestre_*` ligadas na árvore quando passam de uns 6. Prioridade B.
