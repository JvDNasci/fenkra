# ADR 0052: cordão de solda por conjuntos de faces, cosmético por padrão

Data: 2026-11-06. Item: B285. O símbolo guiado do desenho é do B251; o comprimento de solda de nós de tubo
dobrado (`ResultadoDobra::solda`) é do B326 a B331.

## Contexto

Havia símbolo de solda no desenho, mas nenhum cordão no modelo: o símbolo era digitado à mão e podia
discordar da peça. O comprimento soldado de um nó de tubo existia só como número na dobra.

## Decisão

- Um comando novo, `StructWeld`, guarda as faces numa lista só (`faces`, `face_keys`) e um índice `split`:
  as primeiras `split` são o conjunto 1, o resto o conjunto 2. Assim a promoção de chaves de face do
  ADR 0010 continua valendo sem tipo novo de promoção, e `carry_keys` funciona.
- Os caminhos vêm da interseção das faces dos dois conjuntos no kernel (`Shape::section`). Com
  "propagar tangentes", cada face escolhida vira a cadeia de faces tangentes a ela (`face_propagate`).
- Cada caminho vira uma `Trilha` com as duas pernas do filete por segmento. A perna de cada face é
  perpendicular ao caminho, fica no plano da face e aponta para o lado livre do canto (o lado das duas
  normais de saída). O comprimento do caminho é o comprimento de solda.
- O intermitente divide o caminho por comprimento de arco, em trechos inteiros (`l`) a cada passo
  (`l` mais o vão). O alternado atrasa meio passo os caminhos de ordem par. O término em plano recorta
  o caminho antes de dividir.
- Cosmético é o padrão: só caminhos e comprimentos, sem material. Sólido gera o metal por malha fechada
  (anéis de perfil com esquadria nas quinas, costurados no kernel), um sólido por trecho, adicionado como
  peça separada e registrado na lista estrutural como `StructItem::Weld`.
- O que o comando calculou sai por `weld_beads(doc)` (coleta numa variável de thread durante um
  recálculo, como `structural_members`), sem mudar `RecomputeResult`. O símbolo do desenho nasce de
  `fenkra_desenho::solda_cordao`: medida `z`, contorno do formato e `n×l (e)`.

## Consequências

O sólido é uma malha costurada: as faces do cordão são triângulos, não superfícies analíticas, e o
volume é exato para o perfil plano e aproximado (poligonal de 5 pontos) para convexo e côncavo. Cordão em
caminho com muitos segmentos pede uma consulta de normal por segmento. A interface recalcula
`weld_beads` quando o resultado de um recálculo chega e o mouse está solto; documentos grandes pagam um
cálculo a mais por geração. Faces que não se cruzam, ou que são paralelas, falham com mensagem clara.
