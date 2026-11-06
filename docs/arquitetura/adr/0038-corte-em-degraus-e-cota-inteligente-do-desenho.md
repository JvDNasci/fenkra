# ADR 0038: corte em degraus, detalhe em perfil e cota inteligente do desenho

## Contexto
O corte do desenho (B106) só tinha uma reta de dois pontos, e a cota (B107) eram seis botões que exigiam
o tipo antes do clique. Os dois itens mudam o formato do arquivo e a forma de calcular a vista cortada.

## Decisão
1. **A linha de corte com vários segmentos é um prisma reto.** Todo plano de corte contém a direção em
   que a vista mãe olha, porque a linha é traçada nela. O material removido é o polígono do lado do
   observador, no plano da mãe, extrudado nessa direção (`Shape::poly_prism`) e subtraído da peça. Uma
   linha reta dá o meio espaço de antes; degraus dão o recorte em degraus; uma volta de 90 graus dá o
   meio corte. Não há uma segunda forma de cortar para cada modo.
2. **A face cortada de cada segmento é a seção do plano dele recortada pela faixa que ele ocupa.** Os
   degraus são paralelos ao olhar e somem na projeção. A junção entre segmentos não recebe contorno
   (`Proj::seams`). A projeção da vista continua ortogonal ao primeiro segmento; o corte "alinhado"
   (girar os segmentos inclinados para o plano do primeiro) ficou de fora.
3. **Profundidade por distância é um segundo prisma**, com os vértices internos pela bissetriz. Fatia
   guarda só os contornos da face cortada. Peças que não são cortadas entram no conjunto sem passar pelo
   prisma e sem hachura. Borda serrilhada troca o contorno da face por um traço de ruptura.
4. **As opções ficam em `View.corte` (`CorteOpcoes`) e `View.detalhe` (`DetalheOpcoes`)**, com
   `serde(default)`, em vez de crescer `ViewKind::Section`. Assim os pontos de construção e os padrões de
   `match` existentes seguem iguais, e arquivos antigos abrem sem mudança. As opções entram na chave do
   cache da projeção. Inverter o corte inverte o caminho inteiro.
5. **O fantasma do corte só desliza na perpendicular à linha**, pela reta que passa pelo centro da mãe
   (é onde a coordenada ao longo da linha coincide); Ctrl solta. A conta fica em
   `corte::alinhar_ao_corte`, testável sem tela.
6. **A cota inteligente decide o tipo em `fenkra_desenho::cotas`**: o alvo (arco, aresta, ponto) gera a
   lista de tipos que o Tab percorre, os pontos de âncora e o afastamento a partir do mouse. As opções de
   exibição (Ø, parênteses, inspeção, precisão da tolerância) ficam em `Dim.opcoes`.
7. **A chamada do furo vem de `pmi_feature::callout`**, o mesmo gerador do 3D, ligada ao arco clicado pelo
   diâmetro. É uma heurística: dois furos de mesmo diâmetro e formas diferentes pegam o primeiro.

## Consequências
- Arquivos antigos abrem sem mudança. Um arquivo novo com corte em degraus, aberto numa versão antiga,
  mostra o corte reto entre `a` e `b`.
- Linha de corte que se cruza, ou com voltas muito agudas, gera polígono que se cruza e a booleana pode
  falhar; a vista mostra o erro do kernel.
- Cada vértice novo do corte refaz a booleana e a remoção de linhas ocultas. O fantasma quantiza o
  vértice provisório em 1 mm para não recalcular a cada quadro.
