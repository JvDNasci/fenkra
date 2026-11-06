# ADR 0064: dobra em curva como parede varrida e mapa de dobras por linha

Estado: aceita

## Contexto

Uma chapa só gira em torno de reta. Uma dobra por linha desenhada em spline não tem um giro rígido que
leve a parte de fora ao lugar, e o mapa de dobras (`SheetMap`) descreve justamente esse giro. Por isso
a dobra em curva era recusada, e aba curva, aba por superfície e joggle não apareciam nas linhas de
dobra, no DXF nem na tabela por linha.

## Decisão

1. A dobra por linha em curva corta a chapa na curva e levanta uma parede varrida ao longo dela, pelo
   mesmo caminho da aba curva. A largura da parede sai da área da parte cortada, então o desenvolvido
   se mantém, mas o contorno além da dobra vira uma faixa de largura constante. O comando avisa isso.
2. Na planificada, a chapa fica como estava e a dobra entra só como linha curva.
3. O mapa geral guarda dois tipos de registro: `SheetMap`, para a dobra que gira a chapa e leva
   recortes de um estado ao outro, e a linha de dobra (`BendLine`, com `curva` quando não é reta), para
   as que não giram. As duas usam `chapa::compensacao`.
4. A planificada do joggle fica plana, com as duas linhas; sem "manter comprimento", a rampa acrescenta
   material.

## Consequências

Recorte feito depois de uma dobra em curva, de um joggle ou de uma aba curva não é levado pelo arco
entre os estados: isso continua valendo só para `SheetMap`. Superfície desenvolvível de verdade (dobra
cônica) fica para quando houver necessidade medida.
