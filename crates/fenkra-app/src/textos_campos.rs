// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dica própria de cada campo numérico (B278): o que o campo faz, a faixa que costuma valer e o
//! efeito no modelo. A unidade vem do próprio campo e entra na dica em tempo de execução.
//!
//! A chave é o rótulo exato passado a `editor::expr` (em português). Um teste lê o código-fonte e
//! falha se algum rótulo literal novo chegar sem linha aqui, ou sem a versão em inglês.

/// (rótulo, dica em português, dica em inglês)
pub static CAMPOS: &[(&str, &str, &str)] = &[
    // análises de superfície
    ("Amostras por borda", "Quantos pontos a análise mede em cada borda da junção. Mais amostras acham defeitos menores; típico 8 a 64.", "How many points the analysis measures on each junction edge. More samples find smaller defects; typically 8 to 64."),
    ("Dentes", "Quantos dentes o pente desenha ao longo da curva. Típico 32 a 200; não muda a curva.", "How many teeth the comb draws along the curve. Typically 32 to 200; it does not change the curve."),
    ("Escala", "Tamanho dos dentes do pente: milímetros de dente por unidade de curvatura. Só muda o desenho.", "Comb tooth size: millimeters of tooth per unit of curvature. It only changes the drawing."),
    ("Folga máxima da junção", "Bordas mais afastadas que isto não contam como junção. Maior que zero; aumente se a análise não achar a emenda.", "Edges farther apart than this do not count as a junction. Above zero; raise it if the analysis misses the seam."),
    // medidas gerais
    ("Aba D1", "Comprimento da aba no lado 1 da dobra. Maior que zero; mede-se na face externa.", "Flange length on side 1 of the bend. Above zero; measured on the outer face."),
    ("Aba D2", "Comprimento da aba no lado 2 da dobra. Maior que zero; mede-se na face externa.", "Flange length on side 2 of the bend. Above zero; measured on the outer face."),
    ("Afastamento", "Distância de afastamento a partir da referência. Zero encosta; positivo afasta; típico 0,1 a 5.", "Standoff distance from the reference. Zero touches; positive moves away; typically 0.1 to 5."),
    ("Desvio", "Desvio lateral em relação à linha de referência. Zero mantém alinhado; negativo vai para o outro lado.", "Sideways deviation from the reference line. Zero stays aligned; negative goes the other way."),
    ("Diâmetro do furo", "Diâmetro do furo na peça. Maior que zero; costuma seguir a tabela do fixador mais a folga.", "Hole diameter in the part. Above zero; usually follows the fastener table plus clearance."),
    ("Furo até a borda", "Distância do centro do furo até a borda. Em geral pelo menos 1,5 vez o diâmetro.", "Distance from the hole center to the edge. Usually at least 1.5 times the diameter."),
    // cordão de solda (B285)
    ("Perna do filete", "Comprimento de cada perna do filete, medido sobre as faces. Maior que zero; típico de 3 a 10 mm. A garganta vale a perna dividida por raiz de 2.", "Length of each fillet leg, measured on the faces. Above zero; typically 3 to 10 mm. The throat is the leg divided by the square root of 2."),
    ("Comprimento do trecho", "Comprimento de cada trecho do cordão intermitente. Maior que zero; só entram trechos inteiros no caminho.", "Length of each segment of the intermittent bead. Above zero; only whole segments fit along the path."),
    ("Vão entre trechos", "Distância livre entre o fim de um trecho e o começo do próximo. Maior que zero; passo é comprimento mais vão.", "Free distance between the end of one segment and the start of the next. Above zero; pitch is length plus gap."),
    ("Passo entre trechos", "Distância de início a início entre trechos vizinhos. Não pode ser menor que o comprimento do trecho.", "Distance from start to start between neighboring segments. It cannot be smaller than the segment length."),
    ("Recuo da face", "Quanto o elemento recua a partir da face escolhida. Zero fica rente; positivo afunda.", "How far the element sets back from the chosen face. Zero is flush; positive sinks it."),
    ("A", "Primeira medida do comando. A prévia no 3D destaca qual dimensão ela é. Positiva.", "First measure of the command. The 3D preview highlights which dimension it is. Positive."),
    ("B", "Segunda medida do comando. A prévia no 3D destaca qual dimensão ela é. Positiva.", "Second measure of the command. The 3D preview highlights which dimension it is. Positive."),
    ("C", "Terceira medida do comando. A prévia no 3D destaca qual dimensão ela é. Positiva.", "Third measure of the command. The 3D preview highlights which dimension it is. Positive."),
    ("D", "Quarta medida do comando. A prévia no 3D destaca qual dimensão ela é. Positiva.", "Fourth measure of the command. The 3D preview highlights which dimension it is. Positive."),
    ("a", "Primeiro parâmetro da curva. Deve ser maior que zero; valores grandes alargam a curva.", "First curve parameter. Must be above zero; large values widen the curve."),
    ("Foco (a)", "Distância do vértice ao foco da parábola. Maior que zero; valores pequenos fecham a curva.", "Distance from the vertex to the parabola focus. Above zero; small values close the curve."),
    ("b", "Segundo parâmetro da curva. Deve ser maior que zero; valores grandes alargam a curva.", "Second curve parameter. Must be above zero; large values widen the curve."),
    ("u", "Posição ao longo da primeira direção da superfície, de 0 (início) a 1 (fim).", "Position along the surface's first direction, from 0 (start) to 1 (end)."),
    ("v", "Posição ao longo da segunda direção da superfície, de 0 (início) a 1 (fim).", "Position along the surface's second direction, from 0 (start) to 1 (end)."),
    ("X", "Coordenada X. Mede-se a partir da origem do plano ou do sistema escolhido; aceita negativos.", "X coordinate. Measured from the origin of the chosen plane or system; negatives allowed."),
    ("Y", "Coordenada Y. Mede-se a partir da origem do plano ou do sistema escolhido; aceita negativos.", "Y coordinate. Measured from the origin of the chosen plane or system; negatives allowed."),
    ("Z", "Coordenada Z. Mede-se a partir da origem do plano ou do sistema escolhido; aceita negativos.", "Z coordinate. Measured from the origin of the chosen plane or system; negatives allowed."),
    ("Valor", "Valor do comando. A prévia mostra o efeito enquanto você digita; zero costuma não fazer nada.", "Value of the command. The preview shows the effect while you type; zero usually does nothing."),
    ("Tamanho", "Tamanho do elemento. Maior que zero; cresce em todas as direções a partir do ponto de origem.", "Size of the element. Above zero; grows in every direction from the origin point."),
    ("ρ", "Razão de achatamento da curva cônica: 0,5 é parábola, abaixo é elipse, acima é hipérbole.", "Flattening ratio of the conic curve: 0.5 is a parabola, below is an ellipse, above is a hyperbola."),
    ("Peso", "Peso do ponto de controle. Maior que zero; peso alto puxa a curva para o ponto, 1 é neutro.", "Control point weight. Above zero; a high weight pulls the curve toward the point, 1 is neutral."),
    ("Tensão 1", "Quanto a ponte insiste na direção do primeiro apoio, em múltiplos da distância entre as bordas.", "How far the bridge keeps the direction of the first support, in multiples of the distance between the edges."),
    ("Tensão 2", "Quanto a ponte insiste na direção do segundo apoio, em múltiplos da distância entre as bordas.", "How far the bridge keeps the direction of the second support, in multiples of the distance between the edges."),
    ("Tensão", "Tensão da curva nas pontas. De 0 (frouxa) a 1 (esticada); mais tensão reduz o abaulamento.", "Curve tension at the ends. From 0 (loose) to 1 (taut); more tension reduces bulging."),
    ("Tolerância", "Distância máxima aceita entre o resultado e o desenho ideal. Quanto menor, mais fiel e mais lento.", "Largest distance accepted between the result and the ideal shape. Smaller is more faithful and slower."),
    ("Razão", "Razão entre duas medidas. Maior que zero; 1 mantém as duas iguais.", "Ratio between two measures. Above zero; 1 keeps both equal."),
    ("Proporção", "Proporção entre as duas medidas. Maior que zero; 1 mantém o formato original.", "Proportion between the two measures. Above zero; 1 keeps the original shape."),
    ("Fator", "Multiplicador aplicado ao tamanho. 1 não muda nada; 2 dobra; abaixo de 1 reduz.", "Multiplier applied to the size. 1 changes nothing; 2 doubles; below 1 shrinks."),
    ("Fator X", "Multiplicador na direção X. 1 não muda; 2 dobra; negativo espelha.", "Multiplier along X. 1 changes nothing; 2 doubles; negative mirrors."),
    ("Fator Y", "Multiplicador na direção Y. 1 não muda; 2 dobra; negativo espelha.", "Multiplier along Y. 1 changes nothing; 2 doubles; negative mirrors."),
    ("Fator Z (normal)", "Multiplicador ao longo da normal do plano. 1 não muda; 2 dobra; negativo espelha.", "Multiplier along the plane normal. 1 changes nothing; 2 doubles; negative mirrors."),
    // comprimentos e distâncias
    ("Comprimento", "Comprimento total do elemento. Maior que zero; estende na direção indicada pela prévia.", "Total length of the element. Above zero; extends in the direction shown by the preview."),
    ("Comprimento 1", "Comprimento do primeiro lado, medido a partir do plano ou da linha de referência.", "Length of the first side, measured from the plane or the reference line."),
    ("Comprimento 2", "Comprimento do segundo lado, no sentido contrário ao primeiro.", "Length of the second side, in the direction opposite the first."),
    ("Comprimento lado 1", "Quanto o elemento se estende para o lado 1. Zero deixa esse lado sem material.", "How far the element extends to side 1. Zero leaves that side without material."),
    ("Comprimento lado 2", "Quanto o elemento se estende para o lado 2. Zero deixa esse lado sem material.", "How far the element extends to side 2. Zero leaves that side without material."),
    ("Comprimento reto", "Trecho reto antes ou depois da curva. Zero elimina o trecho; evite menor que a espessura.", "Straight stretch before or after the bend. Zero removes it; avoid less than the thickness."),
    ("Comprimento (EOP)", "Comprimento da peça medido do início ao fim do perfil. Maior que zero; define o tamanho final.", "Length of the piece from the profile start to its end. Above zero; sets the final size."),
    ("Comprimento total", "Comprimento que o padrão ocupa de ponta a ponta; as cópias se distribuem dentro dele.", "Length the pattern spans end to end; the copies spread within it."),
    ("Distância", "Distância medida entre os dois elementos escolhidos. Zero junta os dois; negativa inverte o lado.", "Distance measured between the two chosen elements. Zero joins them; negative flips the side."),
    ("Distância 2", "Segunda distância, para o outro lado ou a outra aresta. Zero desliga esse lado.", "Second distance, to the other side or edge. Zero turns that side off."),
    ("Distância inicial", "Onde o efeito começa, medido a partir do ponto de partida. Menor que a distância final.", "Where the effect starts, measured from the starting point. Less than the final distance."),
    ("Distância final", "Onde o efeito termina, medido a partir do ponto de partida. Maior que a distância inicial.", "Where the effect ends, measured from the starting point. Greater than the initial distance."),
    ("Afastamento do plano", "Distância do plano novo até o plano de apoio. Positiva sobe pela normal; negativa desce.", "Distance from the new plane to the support plane. Positive goes up the normal; negative goes down."),
    ("Deslocamento", "Quanto a referência anda a partir da posição original. Zero mantém no lugar; negativo inverte.", "How far the reference moves from the original position. Zero keeps it in place; negative reverses."),
    ("Deslocamento X", "Deslocamento na direção X. Zero mantém a posição; negativo anda para o outro lado.", "Offset along X. Zero keeps the position; negative moves the other way."),
    ("Deslocamento Y", "Deslocamento na direção Y. Zero mantém a posição; negativo anda para o outro lado.", "Offset along Y. Zero keeps the position; negative moves the other way."),
    ("Deslocamento da ponta", "Quanto a ponta anda em relação à base. Zero deixa reta; positivo inclina para fora.", "How far the tip moves relative to the base. Zero leaves it straight; positive leans outward."),
    ("Deslocamento do término", "Avanço além do alvo (positivo) ou recuo antes dele (negativo) no término escolhido.", "Overshoot beyond the target (positive) or setback before it (negative) at the chosen end."),
    ("Deslocamento do término 2", "Avanço ou recuo do término do segundo lado. Positivo passa do alvo; negativo para antes.", "Overshoot or setback of the second side's end. Positive passes the target; negative stops before."),
    ("Extensão", "Quanto a peça se prolonga além do ponto escolhido. Zero não prolonga; cresce no sentido da prévia.", "How far the piece extends beyond the chosen point. Zero does not extend; grows along the preview direction."),
    ("Fim", "Posição onde o trecho termina, medida do início. Maior que o início.", "Position where the stretch ends, measured from the start. Greater than the start."),
    ("Início", "Posição onde o trecho começa, medida da origem. Menor que o fim.", "Position where the stretch begins, measured from the origin. Less than the end."),
    ("Início deslocado", "O perfil começa a esta distância do plano do sketch. Zero começa no plano; negativo recua atrás dele.", "The profile starts this far from the sketch plane. Zero starts on the plane; negative starts behind it."),
    ("Prolongar início", "Estende o início da linha além do ponto escolhido. Zero não estende.", "Extends the line start beyond the chosen point. Zero does not extend."),
    ("Prolongar fim", "Estende o fim da linha além do ponto escolhido. Zero não estende.", "Extends the line end beyond the chosen point. Zero does not extend."),
    ("Recuo início", "Encurta a peça no início, para dar folga ou espaço de solda. Zero não recua.", "Shortens the piece at the start, for clearance or weld space. Zero does not set back."),
    ("Recuo fim", "Encurta a peça no fim, para dar folga ou espaço de solda. Zero não recua.", "Shortens the piece at the end, for clearance or weld space. Zero does not set back."),
    ("Sobra", "Material a mais deixado para usinar ou aparar depois. Zero corta na medida; típico 0,2 a 2.", "Extra material left to machine or trim later. Zero cuts to size; typical 0.2 to 2."),
    ("Total", "Valor total que o padrão ou a distribuição deve cobrir. Maior que zero.", "Total the pattern or distribution must cover. Above zero."),
    ("Cego até", "Profundidade em que o corte para sem atravessar a peça. Menor que a espessura do material.", "Depth at which the cut stops without going through the part. Less than the material thickness."),
    ("Altura", "Altura do elemento ao longo da normal do plano. Maior que zero; negativa vai para o lado contrário.", "Height of the element along the plane normal. Above zero; negative goes the other way."),
    ("Altura da abertura", "Altura livre da abertura. Maior que zero e menor que a altura da peça.", "Clear height of the opening. Above zero and less than the part height."),
    ("Altura do colar", "Altura do colar do furo repuxado. Em geral 1 a 3 vezes a espessura da chapa.", "Height of the extruded-hole collar. Usually 1 to 3 times the sheet thickness."),
    ("Altura do ápice", "Altura do ponto mais alto da curva ou do telhado. Maior que zero; define a inclinação.", "Height of the highest point of the curve or ridge. Above zero; sets the slope."),
    ("Largura", "Largura do elemento no plano. Maior que zero; centrada no eixo, salvo indicação da prévia.", "Width of the element in the plane. Above zero; centered on the axis unless the preview shows otherwise."),
    ("Largura da rampa", "Largura da rampa de saída. Maior que zero; rampas largas suavizam a transição.", "Width of the exit ramp. Above zero; wide ramps soften the transition."),
    ("Largura do alívio", "Largura do corte de alívio no canto. Em geral igual ou maior que a espessura da chapa.", "Width of the corner relief cut. Usually equal to or greater than the sheet thickness."),
    ("Profundidade", "Profundidade do corte ou do elemento ao longo da normal. Maior que zero; negativa inverte o sentido.", "Depth of the cut or element along the normal. Above zero; negative reverses direction."),
    ("Profundidade da rosca", "Quanto da peça recebe rosca a partir da face. Até o comprimento do furo; típico 1,5 vez o diâmetro.", "How much of the part is threaded from the face. Up to the hole length; typically 1.5 times the diameter."),
    ("Prof. contrafuro", "Profundidade do contrafuro para cabeça cilíndrica. Igual à altura da cabeça mais folga.", "Depth of the counterbore for a cylindrical head. Equal to the head height plus clearance."),
    ("Prof. rebaixo", "Profundidade do rebaixo na face. Menor que a espessura restante da peça.", "Depth of the recess in the face. Less than the remaining thickness of the part."),
    ("Espessura", "Espessura da parede ou da chapa. Maior que zero; cresce para o lado indicado pela prévia.", "Wall or sheet thickness. Above zero; grows toward the side shown by the preview."),
    ("Espessura 1", "Espessura da parede do lado 1. Maior que zero; zero deixa esse lado sem parede.", "Wall thickness on side 1. Above zero; zero leaves that side without a wall."),
    ("Espessura 2", "Espessura da parede do lado 2. Maior que zero; zero deixa esse lado sem parede.", "Wall thickness on side 2. Above zero; zero leaves that side without a wall."),
    // raios e diâmetros
    ("Raio", "Raio do arredondamento ou da forma. Maior que zero; raios grandes suavizam e consomem o canto.", "Radius of the rounding or shape. Above zero; large radii soften and consume the corner."),
    ("Raio de dobra", "Raio interno da dobra da chapa. Em geral 1 a 3 vezes a espessura; menor trinca o material.", "Inner bend radius of the sheet. Usually 1 to 3 times the thickness; smaller may crack the material."),
    ("Raio do arco", "Raio do arco que liga os dois trechos. Maior que zero; arcos grandes deixam o desenho mais aberto.", "Radius of the arc joining the two stretches. Above zero; large arcs open up the outline."),
    ("Raio do rolete", "Raio do rolete da ferramenta de conformação. Maior que zero; raios pequenos marcam mais a chapa.", "Radius of the forming tool roller. Above zero; small radii mark the sheet more."),
    ("Raio do tubo", "Raio externo do tubo varrido. Maior que zero e menor que o raio da trajetória.", "Outer radius of the swept tube. Above zero and less than the path radius."),
    ("Raio da 2ª face", "Raio do filete na segunda face da aresta (o raio principal vale para a primeira). Maior que zero; a seta de inverter troca os lados.", "Fillet radius on the second face of the edge (the main radius applies to the first). Above zero; the flip arrow swaps the sides."),
    ("Início do trecho", "Onde o filete parcial começa, em mm contados desde o começo da aresta. De 0 até o comprimento da aresta.", "Where the partial fillet starts, in mm from the start of the edge. From 0 up to the edge length."),
    ("Fim do trecho", "Onde o filete parcial termina, em mm contados desde o começo da aresta. Maior que o início; passar do fim da aresta prende no fim.", "Where the partial fillet ends, in mm from the start of the edge. Above the start; going past the edge end clamps to the end."),
    ("Raio final", "Raio na extremidade final. Zero faz ponta; igual ao inicial mantém reto.", "Radius at the final end. Zero makes a point; equal to the initial keeps it straight."),
    ("Raio inicial", "Raio na extremidade inicial. Zero faz ponta; igual ao final mantém reto.", "Radius at the initial end. Zero makes a point; equal to the final keeps it straight."),
    ("Raio interno", "Raio da parte interna do canto. Maior que zero; fixa a folga entre as faces.", "Radius of the inner part of the corner. Above zero; sets the clearance between faces."),
    ("Diâmetro", "Diâmetro do círculo ou do furo. Maior que zero; o raio é a metade.", "Diameter of the circle or hole. Above zero; the radius is half."),
    ("Ø contrafuro", "Diâmetro do contrafuro para a cabeça do parafuso. Maior que o furo; cabeça mais folga.", "Counterbore diameter for the screw head. Larger than the hole; head plus clearance."),
    ("Ø escareado", "Diâmetro externo do escareado na face. Maior que o furo; define o tamanho do cone.", "Outer countersink diameter at the face. Larger than the hole; sets the cone size."),
    ("Ø nesta peça", "Diâmetro do furo ou do eixo medido nesta peça. Maior que zero; conferido com o par da montagem.", "Hole or shaft diameter measured on this part. Above zero; checked against the assembly mate."),
    ("Ø rebaixo", "Diâmetro do rebaixo na face. Maior que o furo; define a área rebaixada.", "Diameter of the recess in the face. Larger than the hole; sets the recessed area."),
    // ângulos
    ("Ângulo", "Ângulo do comando. De 0 a 360; positivo segue a regra da mão direita em torno do eixo.", "Angle of the command. From 0 to 360; positive follows the right-hand rule around the axis."),
    ("Ângulo 1", "Ângulo do primeiro lado, medido a partir do plano de referência. De 0 a 180.", "Angle of the first side, measured from the reference plane. From 0 to 180."),
    ("Ângulo 2", "Ângulo do segundo lado, medido a partir do plano de referência. De 0 a 180.", "Angle of the second side, measured from the reference plane. From 0 to 180."),
    ("Ângulo da ponta", "Ângulo incluído na ponta da broca ou do cone. Típico 118 a 140 para brocas.", "Included angle at the drill or cone tip. Typically 118 to 140 for drills."),
    ("Ângulo da rampa", "Inclinação da rampa em relação à superfície. Entre 0 e 90; típico 15 a 45.", "Slope of the ramp relative to the surface. Between 0 and 90; typically 15 to 45."),
    ("Ângulo de saída", "Inclinação das paredes para desmoldar. Típico 1 a 5; zero deixa paredes retas.", "Wall slope for mold release. Typically 1 to 5; zero leaves walls straight."),
    ("Ângulo do fundo", "Ângulo do fundo do furo ou do rasgo. Zero dá fundo plano; 118 imita ponta de broca.", "Angle of the hole or slot bottom. Zero gives a flat bottom; 118 mimics a drill point."),
    ("Ângulo final", "Ângulo onde o arco ou a varredura termina. Deve ser maior que o inicial.", "Angle where the arc or sweep ends. Must be greater than the initial."),
    ("Ângulo inicial", "Ângulo onde o arco ou a varredura começa. Deve ser menor que o final.", "Angle where the arc or sweep starts. Must be less than the final."),
    ("Ângulo total", "Abertura total do arco ou da varredura. Até 360; 360 fecha o círculo.", "Total span of the arc or sweep. Up to 360; 360 closes the circle."),
    ("Conicidade", "Quanto o diâmetro muda por unidade de comprimento. Zero é cilíndrico; positivo afina para o fim.", "How much the diameter changes per unit length. Zero is cylindrical; positive tapers toward the end."),
    ("Rotação", "Giro aplicado em torno do eixo escolhido. De 0 a 360; positivo segue a mão direita.", "Rotation applied around the chosen axis. From 0 to 360; positive follows the right hand."),
    ("Varredura", "Ângulo total percorrido pela varredura. Até 360; valores menores deixam o sólido aberto.", "Total angle covered by the sweep. Up to 360; smaller values leave the solid open."),
    // chapa
    ("Fator K", "Posição da linha neutra da dobra, como fração da espessura. Entre 0 e 0,5; típico 0,3 a 0,5.", "Position of the bend neutral line as a fraction of thickness. Between 0 and 0.5; typically 0.3 to 0.5."),
    ("Dedução a 90°", "Quanto se subtrai do comprimento planificado em uma dobra de 90 graus. Vem da tabela do material.", "How much is subtracted from the flat length at a 90-degree bend. Comes from the material table."),
    ("Tolerância a 90°", "Acréscimo de tolerância de dobra a 90 graus, medido no ensaio. Vem da tabela do material.", "Bend allowance at 90 degrees, measured by test. Comes from the material table."),
    ("Aba", "Comprimento da aba dobrada, medido na face externa. Maior que o raio de dobra mais a espessura.", "Length of the bent flange, measured on the outer face. Greater than bend radius plus thickness."),
    ("Aba do reforço", "Largura da aba do reforço dobrado. Em geral 3 a 6 vezes a espessura.", "Width of the folded stiffener flange. Usually 3 to 6 times the thickness."),
    ("Chanfro de alívio", "Tamanho do chanfro que alivia o canto da dobra. Maior que zero; evita rasgos na chapa.", "Size of the chamfer relieving the bend corner. Above zero; avoids tearing in the sheet."),
    ("Rampa", "Comprimento da rampa de transição. Maior que zero; rampas longas dobram com menos esforço.", "Length of the transition ramp. Above zero; long ramps bend with less effort."),
    // soldas, folgas, perfis
    ("Folga (offset)", "Folga mantida entre os dois corpos. Zero encosta; positivo afasta; típico 0,1 a 1.", "Gap kept between the two bodies. Zero touches; positive separates; typically 0.1 to 1."),
    ("Folga de solda", "Espaço deixado na junta para a solda entrar. Típico 1 a 3; zero deixa as peças encostadas.", "Space left in the joint for the weld to enter. Typically 1 to 3; zero leaves the pieces touching."),
    ("Folga interna", "Espaço livre dentro do encaixe. Zero é justo; positivo facilita a montagem.", "Free space inside the fit. Zero is tight; positive eases assembly."),
    ("Folga no fundo", "Espaço deixado no fundo do furo ou do rasgo. Zero encosta; típico 0,5 a 2.", "Space left at the bottom of the hole or slot. Zero touches; typically 0.5 to 2."),
    // padrões e distribuição
    ("Espaçamento", "Distância entre cópias vizinhas, de centro a centro. Maior que zero; menor que o tamanho da peça faz sobrepor.", "Distance between neighboring copies, center to center. Above zero; less than the part size makes them overlap."),
    ("Espaçamento 2", "Distância entre as cópias na segunda direção, de centro a centro. Maior que zero.", "Distance between copies in the second direction, center to center. Above zero."),
    ("Quantidade", "Número de cópias, contando a original. Inteiro a partir de 1; 1 não repete.", "Number of copies, counting the original. Integer from 1; 1 does not repeat."),
    ("Quantidade 2", "Número de cópias na segunda direção, contando a original. Inteiro a partir de 1.", "Number of copies in the second direction, counting the original. Integer from 1."),
    ("Sentido 1", "Direção do primeiro lado: 1 segue a seta da prévia; -1 inverte para o lado oposto.", "Direction of the first side: 1 follows the preview arrow; -1 reverses to the opposite side."),
    ("Sentido 2", "Direção do segundo lado: 1 segue a seta da prévia; -1 inverte para o lado oposto.", "Direction of the second side: 1 follows the preview arrow; -1 reverses to the opposite side."),
    ("Seções", "Número de seções intermediárias da transição. Inteiro de 1 a 50; mais seções acompanham melhor a curva.", "Number of intermediate sections of the transition. Integer 1 to 50; more sections follow the curve better."),
    // roscas
    ("Passo", "Distância entre filetes vizinhos da rosca. Vem da norma (ex.: 1,5 em M10); maior que zero.", "Distance between neighboring thread crests. Comes from the standard (e.g. 1.5 for M10); above zero."),
    ("Passo (mm/volta)", "Avanço por volta completa da hélice. Maior que zero; passos grandes deixam a hélice mais esticada.", "Advance per full turn of the helix. Above zero; large pitches stretch the helix."),
    ("mm por volta", "Avanço da hélice a cada volta completa. Maior que zero; é o passo da rosca ou da mola.", "Helix advance per full turn. Above zero; it is the thread or spring pitch."),
    ("Voltas", "Número de voltas da hélice. Aceita frações (2,5); maior que zero.", "Number of helix turns. Accepts fractions (2.5); above zero."),
    // chapa: estilos (B283)
    ("Alívio padrão", "Largura do alívio que toda aba herda nas pontas recuadas. Zero desliga; típico 1 a 1,5 vez a espessura.", "Relief width every flange inherits at recessed ends. Zero turns it off; typically 1 to 1.5 times the thickness."),
    ("Largura da aba", "Largura do trecho da aresta que recebe a aba. Maior que zero e até o comprimento da aresta; vale em cada aresta do laço.", "Width of the edge stretch that receives the flange. Above zero and up to the edge length; applies to each edge in the loop."),
    ("Início da aba", "Distância da ponta de referência da aresta até onde a aba começa. Zero encosta na ponta; início mais a largura não passa do fim da aresta.", "Distance from the edge's reference end to where the flange starts. Zero touches the end; start plus width cannot pass the edge end."),
    ("Tamanho do canto", "Tamanho do alívio de canto sugerido ao criar o comando. Maior que zero; costuma ser 1 a 2 vezes a espessura.", "Corner relief size suggested when the command is created. Above zero; usually 1 to 2 times the thickness."),
    ("Espessura esperada", "Espessura que o corpo deve ter para ser aceito como chapa. Maior que zero; vazio mede a do corpo, preenchido confere com tolerância de 2 %.", "Thickness the body must have to be accepted as sheet metal. Above zero; empty measures the body, filled checks it within 2 %."),
    ("Folga do canto","Distância livre entre as faces que se encontram no canto fechado. De 0 a 10; zero une as pontas, 0,2 a 1 é o usual para a peça dobrar sem encostar.", "Free distance between the faces that meet at the closed corner. From 0 to 10; zero joins the ends, 0.2 to 1 is usual so the part folds without touching."),
    ("Razão de sobreposição", "Quanto da espessura da aba coberta a aba de cima cobre. De 0 a 1; 1 cobre tudo até a face de fora, 0 não avança sobre o canto.", "How much of the covered flange's thickness the top flange covers. From 0 to 1; 1 covers up to the outer face, 0 does not advance over the corner."),
];

/// Dica do campo pelo rótulo em português.
pub fn dica(rotulo: &str) -> Option<&'static str> {
    CAMPOS.iter().find(|(r, _, _)| *r == rotulo).map(|(_, d, _)| *d)
}

/// Pares português → inglês para o mapa de tradução (`i18n.rs` os incorpora).
pub fn pares() -> impl Iterator<Item = (&'static str, &'static str)> {
    CAMPOS.iter().map(|(_, pt, en)| (*pt, *en)).chain(SIMBOLOS.iter().copied())
}

/// Rótulos que são símbolos e valem igual nos dois idiomas, ou ainda não tinham tradução.
const SIMBOLOS: &[(&str, &str)] = &[("Alívio padrão", "Default relief"), ("Início da aba", "Flange start"), ("Largura da aba", "Flange width"), ("Tamanho do canto", "Corner size"), ("Início do trecho", "Segment start"), ("Fim do trecho", "Segment end"), ("Raio da 2ª face", "2nd face radius"), ("A", "A"), ("B", "B"), ("C", "C"), ("D", "D"), ("X", "X"), ("Y", "Y"), ("Z", "Z"), ("a", "a"), ("b", "b"), ("u", "u"), ("v", "v"), ("ρ", "ρ"), ("Total", "Total"), ("Foco (a)", "Focus (a)")];

#[cfg(test)]
mod testes {
    use super::*;
    use crate::i18n::tem_traducao;

    /// Rótulos literais passados a `expr(ui, ...)` em todo o código (inclusive `if ... { "A" } else { "B" }`).
    fn rotulos_no_codigo() -> Vec<String> {
        fn visitar(pasta: &std::path::Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(pasta).expect("pasta").flatten() {
                let p = e.path();
                if p.is_dir() {
                    visitar(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "textos_campos.rs") {
                    let f = std::fs::read_to_string(&p).expect("fonte");
                    let f = f.split("#[cfg(test)]").next().unwrap_or(&f);
                    for trecho in f.split("expr(ui, ").skip(1) {
                        // a primeira expressão vai até a vírgula que fecha o argumento
                        let mut prof = 0i32;
                        let mut fim = trecho.len();
                        let mut aspas = false;
                        for (i, c) in trecho.char_indices() {
                            match c {
                                '"' => aspas = !aspas,
                                '(' | '{' if !aspas => prof += 1,
                                ')' | '}' if !aspas => {
                                    prof -= 1;
                                    // chamada de um argumento só: não tem rótulo
                                    if prof < 0 {
                                        fim = 0;
                                        break;
                                    }
                                }
                                ',' if !aspas && prof == 0 => {
                                    fim = i;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        let arg = &trecho[..fim];
                        for (n, parte) in arg.split('"').enumerate() {
                            if n % 2 == 1 && !parte.is_empty() {
                                out.push(parte.to_owned());
                            }
                        }
                    }
                }
            }
        }
        let mut v = vec![];
        visitar(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")), &mut v);
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn todo_rotulo_de_campo_tem_dica_propria_e_traducao() {
        let rotulos = rotulos_no_codigo();
        assert!(rotulos.len() > 100, "o leitor do código achou só {} rótulos", rotulos.len());
        let sem_dica: Vec<&String> = rotulos.iter().filter(|r| dica(r).is_none()).collect();
        assert!(sem_dica.is_empty(), "campo sem dica em textos_campos.rs: {sem_dica:?}");
        let sem_rotulo_en: Vec<&String> = rotulos.iter().filter(|r| !tem_traducao(r)).collect();
        assert!(sem_rotulo_en.is_empty(), "rótulo de campo sem tradução: {sem_rotulo_en:?}");
    }

    #[test]
    fn dicas_sao_unicas_traduzidas_e_dizem_algo() {
        let mut vistos = std::collections::HashSet::new();
        for (r, pt, en) in CAMPOS {
            assert!(vistos.insert(*r), "rótulo repetido: {r}");
            assert!(pt.chars().count() >= 30 && en.chars().count() >= 30, "dica curta demais em {r}");
            assert_ne!(pt, en, "dica de {r} sem tradução de fato");
            assert!(tem_traducao(pt), "dica de {r} fora do mapa de tradução");
        }
    }
}
