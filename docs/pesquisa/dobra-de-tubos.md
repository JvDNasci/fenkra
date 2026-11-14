# Dobra de tubos e "nó com dobra" na bancada Estrutural

Pesquisa de apoio. Escrita com palavras próprias. As fontes consultadas ficam em
`docs/arquitetura/procedencia.md` (referências `[F#]` abaixo).

Legenda: **[V]** confirmado em fonte efetivamente lida (inclui páginas e resumos públicos; a leitura de
vários PDFs falhou, ver "Limites"). **[C]** conhecimento não confirmado, derivação própria ou regra de
oficina que precisa de teste antes de virar regra do produto.

## Limites desta pesquisa

- Vários PDFs acadêmicos (teses, artigos) bloquearam a leitura ou vieram comprimidos; só os resumos
  foram lidos. Fórmulas de ovalização, espessura e linha neutra abaixo são, portanto, majoritariamente [C].
- Nenhuma tabela paga de norma foi copiada. As normas aparecem só como referência a obter.
- A régua ou gabarito citado pelo usuário **não foi identificado** (ver seção 4).

## 1. Processos de dobra

| Processo | Como funciona | Quando serve | Limites |
|---|---|---|---|
| Tração rotativa (rotary draw) | O tubo é preso a uma matriz de raio fixo e puxado em volta dela; uma matriz de pressão apoia o lado de fora | Série, raios pequenos, boa repetição | Uma ferramenta por raio e por diâmetro [V][F9] |
| Com mandril | Haste com esferas entra no tubo e apoia a parede por dentro | Paredes finas, raio pequeno; evita enrugamento e achatamento [V][F1][F2] | Com raio muito fechado o mandril pode aumentar o afinamento externo [V][F1]; posição do mandril altera ovalização e enrugamento [V][F3] |
| Sem mandril | Só matrizes | Paredes grossas, raio folgado | Menor controle de achatamento [C] |
| Compressão | Rolo ou matriz de compressão envolve o tubo e uma sapata o dobra em torno da matriz fixa | Peças longas, poucos ciclos [C] | Costuma exigir raio maior; marca superficial [C] |
| Prensa (pontos) | Cargas pontuais em prensa, em passos | Raios muito grandes, arqueamento [V][F9] | Controle de forma ruim para raio fechado [C] |
| Rolos (três rolos) | Raio constante por passes com rolos ajustáveis | Arcos grandes, perfis estruturais [V][F9] | Raio mínimo depende de rolos que acompanham a seção, esbeltez e eixo de dobra [V][F9] |
| Indução | Bobina aquece uma faixa curta, o tubo é empurrado e temperado com água | Raios apertados em perfis grossos [V][F9] | Equipamento caro, metalurgia da zona afetada [C] |
| Incremental sincronizado | Cargas em vários pontos, em ciclos | Raio apertado com pouca distorção [V][F9] | Equipamento específico |
| Enchimento (areia, resina, liga) | Preenche o vazio para impedir o colapso e é retirado depois | Artesanal, peça única, parede fina [C] | Lento, limpeza; não confirmado em fonte lida |

Há um efeito de custo: a dobra de estruturais é cobrada pelo tempo por peça, não pelo peso [V][F9].

## 2. Redondo x quadrado/retangular

- Redondo é mais fácil de curvar que quadrado ou retangular [V][F9].
- Defeitos do redondo: ovalização (achatamento), afinamento da parede externa (extradorso), espessamento
  e enrugamento no lado interno (intradorso) [V][F1][F3]. Risco cresce quando a razão de curvatura
  (raio/diâmetro externo) e a espessura diminuem [V][F3]. O afinamento percentual diminui com raio maior [V][F4].
- Defeitos do quadrado/retangular: concavidade da parede interna, que cresce com a razão largura/altura na dobra
  pelo eixo fraco; achatamento; distorção das paredes laterais; riscos [V][F9]. Em estudo recente, o
  enrugamento apareceu só no quadrado, em todos os corpos de prova a 90° com parede de 0,7 a 0,8 mm [V][F2 resumo].
- Parede mais grossa tolera raio menor [V][F9]. Paredes finas tendem a embarrigar para dentro na face comprimida;
  paredes grossas, a abaular as laterais [V][F9, citando ensaios].
- Em perfis estruturais o raio costuma ser medido na face externa; em tubo de tubulação, no eixo [V][F9].
- Colapso da seção: [C] ocorre quando D/t é alto e CLR/D baixo sem apoio interno; o mandril, o enchimento ou
  a pressão interna adiam. Pressão interna aumenta o afinamento externo [V][F5 resumo].

## 3. Mecânica

Variáveis: D diâmetro externo (ou altura H da seção), t parede, CLR raio de centro, theta ângulo de dobra,
y distância de uma fibra ao eixo do tubo.

- **Raio de centro (CLR)**: distância do centro da curva ao eixo do tubo. É a referência de medida da peça
  dobrada, porque o eixo é a linha de referência prática [V][F10]; oficinas medem o tubo pelo eixo [V][F8].
- **Comprimento desenvolvido de uma dobra** (arco no eixo): `L = CLR * theta_rad`, equivalente a
  `CLR * theta_graus * 0,01745` [V][F8]. Comprimento de corte total = soma dos trechos retos + arcos [V][F8].
  Dedução de dobra: `BD = 2*CLR*tan(theta/2) - L` [V][F8].
- **Fator de dobra**: razões `CLR/D` (curvatura) e `D/t` (esbeltez). Valores de corte dependem de material,
  máquina e ferramenta; não há razão universal em fonte lida [V][F9]. Regra prática de forum: raio interno
  de gaiola de segurança em torno de 3 vezes o diâmetro; máquina chega a 1,5 vezes [V como relato de forum,
  baixa autoridade][F7].
- **Linha neutra**: em seção homogênea a linha neutra coincide com o centroide; ao dobrar, o material se
  redistribui e a linha neutra se desloca para o lado interno, e o momento de inércia deixa de ser constante [V][F6].
  Há solução analítica de retorno elástico que modela o deslocamento da camada neutra, com erro médio abaixo
  de 5,8% para dobras de 90° ou mais em titânio [V][F11]. Valor numérico do deslocamento: [C]; deve ser
  parâmetro por material/ferramenta, calibrado em ensaio.
- **Alongamento** (modelo simples, neutra no eixo) [C]: a fibra a distância y do eixo tem comprimento
  `(CLR + y) * theta`; a deformação é `e = y / CLR`. Fibra externa: `e_ext = (D/2) / CLR`.
  Exemplo: D = 50 mm, CLR = 100 mm (CLR/D = 2): `e_ext = 25/100 = 25%`. Compare com o alongamento
  uniforme do material para ver se rompe; é um critério de triagem, não de projeto.
- **Afinamento externo aproximado** [C]: por conservação de volume em fibra, `t_ext ~ t * CLR / (CLR + D/2)`.
  Exemplo anterior: `t_ext ~ t * 100/125 = 0,8 t`, ou 20% de afinamento. Na prática depende do mandril e da
  tração; usar só como estimativa e permitir tabela empírica por ferramenta.
- **Exemplo de desenvolvido**: tubo D 50, dobra de 90° com CLR 100: `L = 100 * pi/2 = 157,08 mm`. Dois trechos
  retos de 200 mm: corte = 200 + 157,08 + 200 = 557,08 mm [C, aritmética sobre a fórmula [V]].
- **Retorno elástico**: ocorre após descarregar; compensa-se sobredobrando [V][F11 e resumo]. Cresce com
  limite de escoamento e CLR/D, cai com t; deve ser parâmetro (graus por material e raio) vindo de ensaio [C].
- **Comprimento a mais**: o material precisa de comprimento extra para a pega da máquina nas pontas [V][F9 cita
  sobrecomprimento em estruturais].
- Limitações do arco de eixo: ignora retorno, perda de tangente, achatamento e rotação entre planos [V][F8].

## 4. Dobra por corte (entalhes em V e rasgos)

O que as fontes confirmam:

- Para dobrar quadrado, corta-se um V atravessando três paredes e deixa-se a quarta inteira. A quarta é
  riscada no vértice, para virar dobradiça sem retorno elástico, e depois se faz ponto de solda. Há ferramenta de
  prensa que faz isso, e o ângulo do V é um pouco maior que a dobra pretendida por causa de rebarbas [V][F12 patente].
- Há também furo pequeno na nervura interna para evitar abaulamento ou trinca; recomenda-se começar o corte onde
  termina a parede interna; o método dispensa alinhar e soldar cantos em quadro; não serve para estruturas de
  carga sem ensaio ou simulação [V][F13].
- O ângulo do V do gabarito comercial é o ângulo que o canto terá depois de dobrado [V][F14]. A relação
  `180 - 360/n` é de polígono regular, usada em calculadora de entalhe [V][F15].
- Ajuste fino do ângulo para retorno elástico é prática conhecida [V][F16].

Geometria completa (derivação própria, **[C], validar com protótipo**). Considere perfil quadrado de altura
H na direção da dobra e parede t. As paredes do intradorso e as laterais são cortadas; a parede do extradorso é
a dobradiça, e o pivô fica na face interna dela. Distância do pivô à face externa do intradorso:
`h = H - t`.

1. Ângulo total Theta e número de entalhes `n`. Ângulo de cada entalhe `phi = Theta / n`.
2. Abertura do V na face externa do intradorso: `w = 2 * h * tan(phi/2)`.
3. Passo entre entalhes, medido na linha do pivô: `p = 2 * R_p * sin(phi/2)`, onde `R_p` é o raio na linha do
   pivô (extradorso). Para `phi` pequeno, `p ~ R_p * phi`.
4. Raio equivalente do eixo: `R_eixo ~ R_p - h/2` (eixo no meio da seção). Raio interno (intradorso): `R_p - h`.
5. Desenvolvido na linha do pivô: `n * p`; é essa linha que não encurta (dobradiça). Comprimento de corte de
   estoque usa a linha do pivô, não o eixo [C].
6. Rasgo reto (sem V, só abertura de corte): fecha `phi = 2 * atan(k / (2*h))` com k a largura do corte;
   serve para laser; o número de rasgos sai de `n = ceil(Theta / phi)`.
7. Escolha de `n`: mais entalhes dão curva mais lisa e menos solda por entalhe, porém mais corte. Limite
   prático: abertura w mínima de ferramenta e largura da lâmina; máximo `phi` tal que a aresta do V não
   interfira com a parede lateral (cortar também as laterais).
8. Solda depois: os V fechados formam juntas de topo ou de esquadria; a solda fecha cada entalhe, e a
   resistência é a de uma junta soldada, não de tubo contínuo [V][F13 e F12; inferência].
9. Redondo: o corte tipo V em redondo é mais raro; usa-se rasgo transversal ou aproximação por segmentos
   com esquadria [C].

**Régua ou gabarito citado**: nenhuma fonte lida identificou uma régua chinesa ou japonesa para curvas em
metalon. Os resultados trouxeram só um gabarito de cantoneira com V riscado para marcar 45° [V][F17] e ferramentas de
traçar raios genéricas. Não afirmamos qual é; o produto não depende disso: o gabarito imprimível sai da
geometria acima.

## 5. Regras e tabelas

- A norma aeroespacial de raio de dobra de tubo existe, com tabela de tamanhos preferenciais e fórmula de
  raio mínimo, trechos retos entre dobras e limites de achatamento [V][F18 resumo]. Revisão vigente varia
  entre listagens; obter a vigente antes de citar. Existe equivalente europeu para tubos de motor [V][F18].
  Há outra norma sobre aceitação de dobras em tubulação hidráulica [V][F18]. São pagas: o produto só deve
  oferecer o formato de tabela, com campos do usuário, e não os valores.
- Raio mínimo por material e espessura: guardar como regra cadastrável (material, D, t, processo, ferramenta
  -> CLR mínimo, retorno, afinamento medido). Valores iniciais devem vir do próprio usuário ou de ensaio [C].
- Automotivo: relato de forum cita 3 x D como raio interno em gaiola e 1,5 x D como limite de máquina [V,
  baixa autoridade][F7]. Tratar como exemplo de preenchimento, não como valor padrão.
- **Dados de máquina**: LRA é o conjunto padrão: L distância entre dobras (avanço), R rotação do plano, A ângulo
  da dobra. YBC é o mesmo trio com outras letras (Y avanço, B rotação, C ângulo) [V][F19, F20]. XYZ descreve
  pontos do eixo; a maioria das máquinas não roda direto XYZ; a conversão é um cálculo inverso [V][F19, F21].
  Dados "absolutos" (cumulativos) mudam a interpretação de tração x compressão; nomes variam por fabricante [V][F21].
  Validar a conversão no software da máquina antes de produzir [V][F21].
- Exportações de CAD costumam trazer: comprimento de corte, avanço, rotação e raio por dobra, coordenadas de
  tangente e interseção, comprimento total [V][F22, F23].

## 6. Como softwares de modelagem CAD avançados tratam o tubo dobrado

Em linhas gerais, pela documentação pública [V][F22, F23, F24]:

- O tubo nasce de uma **rota** (caminho 3D de segmentos retos e arcos); a seção segue a rota.
- O **raio pode variar por trecho**, e a tabela de dobra mostra vários raios para o mesmo tubo.
- **Tabela de dobra**: por dobra, raio, ângulo, rotação, avanço e comprimento total de corte; exportação em
  texto, CSV ou HTML.
- Há **tabelas que mapeiam raio e diâmetro para o desenvolvido** (estoque reto necessário para formar a dobra).
- Para chapa, o desenvolvido usa fator K ou fator Y; para tubo, o desenvolvido usa o arco do eixo mais a
  correção da tabela [V][F24 resumo].
- Há extensões de terceiros que exportam XYZ e LRA/YBC com desenvolvido e conversão de unidades [V][F22].

## 7. Canto com gabarito

Aprofundamento a partir das fotos do usuário. O que as fotos mostram é dado do usuário, não fonte. Os
números abaixo vêm de derivação própria conferida por script numérico (`tubo_geometria.py`, fora do
repositório): ficam **[C]** até haver protótipo real.

### 7.1 O gabarito

- Nome genérico: **régua marcadora de ângulo e arco para tubo quadrado** (em inglês, "angle and arc
  marking tool" ou "angle arc scriber for square tube"). É uma cantoneira em L de aço inox que abraça duas
  faces do tubo, com escala de largura, arcos concêntricos nas pontas para riscar o raio do canto e um V de 90° no
  meio para riscar a bissetriz [fotos do usuário].
- **[V]** Existem anúncios de venda de ferramenta com esse nome e função, para riscar o corte de cantos em tubo
  quadrado, com faixas como R20–80 mm e uma com a sigla "S 2065 mm", que pode ser a escala 20–65 mm das fotos
  [F25]. Os resumos não trazem manual, fabricante confirmado nem origem.
- **[C]** Que seja a mesma peça das fotos, que tenha origem chinesa ou japonesa e que o uso seja o descrito abaixo.
  Não achamos guia de uso confiável; a descrição de uso é dedução. A busca em chinês não achou nada específico.
- Padrão de marcação por raio e ângulo existe em patentes de gabarito de tubo redondo [F26], o que sustenta a
  ideia de gabarito paramétrico por raio e ângulo.

### 7.2 Variáveis e quadro

H altura lateral (dimensão do tubo no plano da dobra: é a que a escala do gabarito lê), b largura da
placa externa, t parede, Ro raio da face externa da placa, Θ ângulo da dobra, h = Θ/2, rf = Ro − t (raio dos
arcos dos setores), rn raio da linha neutra da placa, K fator de neutra. Quadro: x ao longo da perna 1, contado do ponto
onde o arco externo começa; y profundidade a partir da face externa; C = (0, Ro) centro do arco.

### 7.3 Geometria derivada

Estrutura: a **placa externa** (parede do extradorso) fica inteira e é dobrada em arco; cada **parede lateral**
mantém, em cada perna, um **setor** de ângulo h com ápice em C; a **parede interna** termina nas bordas radiais
dos setores. As laterais são cortadas no contorno "andorinha"; a placa se enrola nos arcos dos setores, e a bissetriz e
os arcos viram solda.

1. Setor A (perna 1): pontos com y de t a H, dentro do círculo de centro C e raio rf para x > 0, do lado da perna 1
   em relação à bissetriz. O setor B (perna 2) é o espelho de A na bissetriz.
2. Comprimento plano da placa: `ℓ = rn · Θ`, com rn pela neutra de volume constante `rn = raiz(rf · Ro)`
   (equivale a K ≈ 0,46) ou por K cadastrado: `rn = rf + K·t`.
3. No estado plano, B é o espelho de A em torno de x = ℓ/2 (a reflexão na bissetriz seguida da rotação de −Θ
   em torno de C é a reflexão no eixo x = 0; conferido no script: nenhuma sobreposição e coincidência de 99,9%
   na grade, sobra só erro de borda da grade).
4. Contorno do corte por lateral (andorinha), fechado: (0,t) → (ℓ,t) [libera a placa] → arco de B até o fim do arco →
   bissetriz de B até (ℓ,H) → (ℓ,H) → (0,H) pela face interna → bissetriz de A até o fim do arco → arco de A de volta
   a (0,t). Fim do arco de A: `(rf·sen h, Ro − rf·cos h)`.
5. Folga plana no alto do corte entre os setores: `g = ℓ − 2·rf·sen h`, que fecha a zero ao dobrar. Folga de raiz de solda `e` (parâmetro, 1 a 2 mm): recua cada bissetriz de e/2, e a folga plana
   passa a `ℓ − 2·rf·sen(h − asen(e/(2·rf)))`.
6. Dedução de dobra e comprimento de corte, com X1 e X2 medidos nas faces externas até o canto virtual:
   `BD = 2·Ro·tan h − ℓ`; `L_corte = X1 + X2 − BD`. A zona começa em `X1 − Ro·tan h`.
7. Solda: arcos `4·rf·h` (2 laterais × 2 setores), bissetrizes `2·(rf + m)` e a linha do canto interno (b), com m abaixo.
8. Raio de dobra da placa: plana → arco com raio interno rf. Deformação da fibra externa da placa ≈ t / (2·rn) ≈ 2%,
   muito abaixo do alongamento do aço. O problema é o retorno elástico, grande para rf/t = 24: a placa precisa ser forçada contra os arcos
   dos setores e pontear antes de soldar [C].

**Exemplo 30×30×1,2, Θ = 90°, Ro = 30** (script): rf = 28,8; rn = 29,39 (K = 0,33 dá 29,20; K = 0,5 dá 29,40);
ℓ = 46,17 mm (K = 0,33: 45,86); arco externo 47,12; arco interno 45,24; BD = 13,83; folga plana 5,44; fim do arco em (20,36; 9,64);
e = 1 → folga 6,16; e = 2 → 6,88; solda: arcos 90,5 + bissetrizes 57,6 = 148,1 mm, mais a linha do canto interno de 30 mm.

### 7.4 Hipótese do coordenador: onde confirma e onde falha

Confirma (com t = 0): zona de πW/2 na face externa; setores de 45° com ápice no canto interno; os dois setores
ocupam 2·W·sen45° = 1,414 W no estado plano; folga plana de 0,157 W (script: 4,697 para W = 30, ou 0,1566 W).

Corrige ou falha:

1. O raio do setor é **W − t**, não W: a placa ocupa de W − t a W. Só o arco externo da placa vale W.
2. O comprimento plano da placa é o da **linha neutra**, 46,17, e não o da face externa (47,12): 2,0% a
   menos. Usar a face externa gera tira longa demais (cerca de 1 mm em 47).
3. A folga de 0,157 W é a folga **plana**; com parede 1,2 vira 5,44 (16% maior que 4,70). Fechada, é zero. A folga de solda é outro
   parâmetro e.
4. A parede interna não "some": as pontas das duas pernas terminam em planos radiais e **se tocam só na linha do canto
   interno**, sem sobreposição e sem área de contato. O encontro é soldado em filete na linha ou deixado com abertura.
5. O corte não é só "arco e bissetriz": falta o corte de liberação ao longo da face interna da placa (y = t),
   que passa pelos cantos arredondados de fábrica do tubo [C]. As lascas entre o arco e a placa são descartadas.
6. A hipótese vale **só para Ro = H**, quando o centro C coincide com o canto interno I. Para Ro ≠ H, I fica a `m = (H − Ro)/cos h` de C, sobre a bissetriz.
7. Retangular: o raio do canto é a altura lateral (dimensão no plano da dobra), nunca a largura da placa.

### 7.5 Variações (script)

| Caso | Resultado |
|---|---|
| Θ = 30, 60, 90, 120, 135, 150 (H = Ro = 30, t = 1,2) | ℓ = 15,39 / 30,78 / 46,17 / 61,56 / 69,26 / 76,95 · folga = 0,48 / 1,98 / 5,44 / 11,68 / 16,04 / 21,32 · BD = 0,69 / 3,86 / 13,83 / 42,36 / 75,60 / 146,97. Sem sobreposição em todos; abaixo de 30° o ganho é pequeno (use V ou rolos). |
| Ro = 15, 22,5, 30, 45, 60, 90 (H = 30, 90°) | ℓ = 22,60 / 34,39 / 46,17 / 69,74 / 93,30 / 140,43 · abertura plana da parede interna ℓ − 2·Ix = 52,6 / 49,4 / 46,2 / 39,7 / 33,3 / 20,4 · bissetriz = 35,0 / 31,9 / 28,8 / 22,6 / 16,4 / 4,0. Ro menor que H abre mais a parede interna; Ro maior estreita os setores e a abertura zera perto de Ro ≈ 4,6 H, quando o modo deixa de existir. |
| Retangular 20×40×1,5, dobra com lateral de 20 (placa de 40) | Ro = 20, ℓ = 30,21, folga 4,05, BD = 9,79, solda arcos 58,1 + bissetrizes 37,0 |
| Mesmo tubo, dobra pela outra face (lateral de 40, placa de 20) | Ro = 40, ℓ = 61,64, folga 7,20, BD = 18,36, solda arcos 121,0 + bissetrizes 77,0 |

Trocar a face da dobra muda o raio (20 ou 40), o comprimento da zona e a solda. O CAD deve perguntar qual dimensão fica no plano da dobra.
O campo "largura do tubo" do gabarito é a altura lateral nessa escolha.

## 8. Modos em detalhe: como o CAD modela

Convenção: o nó guarda modo, Θ, raio (Ro ou CLR), regra de dobra, parâmetros (n, e, K, ε_adm). Mapeamento com os modos de B326:
"dobra por dobradeira" = M1; "dobra por corte" = M2, M3 e M4 (cortar, fechar e soldar); "corte mais dobradeira" = M2 ou M3 fechados por ferramenta, com o
retorno compensado; "corte simples" = M5.

### M1. Dobradeira ou rolos (sólido curvo)

- Entradas: perfil, Θ, CLR (raio do eixo), regra com ε_adm, retorno Δ, processo (mandril, rolos).
- Geometria: varredura da seção ao longo de reta, arco de raio CLR, reta. Uma peça só. Prévia mostra afinamento e espessamento estimados.
- Lista de corte: `L_corte = X1 + X2 − BD`, `BD = 2·CLR·tan h − CLR·Θ`; por dobra: avanço, rotação, ângulo + Δ, CLR; pontas de pega.
- Limites: `CLR ≥ (D/2)/ε_adm` (triagem); redondo precisa de mandril em parede fina; retangular tem concavidade e achatamento.
- Quando usar: série, tubo redondo, raio ≥ ~2 D ou 3 H com a regra do material.
- Solda: nada.
- Exemplo redondo 25,4×1,5, Θ = 90°, CLR = 2 D = 50,8: L = 79,80; BD = 21,80; e_ext = 12,7/50,8 = 25%; parede externa ≈ 1,5·50,8/63,5 = 1,20
  (−20%); interna ≈ 1,5·50,8/38,1 = 2,00 (+33%) [C]. Com CLR = 1,5 D = 38,1: L = 59,85, e_ext = 33%, parede externa 1,125 (−25%).
- Exemplo 30×30×1,2, CLR = 60: L = 94,25; BD = 25,75; e_ext = 15/60 = 25%; parede externa 0,96 (−20%), interna 1,60 (+33%). Com ε_adm = 0,15, CLR mínimo = 15/0,15 = 100
  (3,3 H): o CAD deve avisar em 60. Retorno: ângulo da máquina 90 + Δ (Δ vem da regra, por exemplo 3°); o avanço não muda.
- Dados de máquina: `L1 = X1 − CLR·tan h`, rotação 0, ângulo 90 + Δ, `L2 = X2 − CLR·tan h`.

### M2. V único com dobradiça

- Entradas: Θ, ri da dobradiça, K, perfil quadrado ou retangular (não serve em redondo).
- Geometria: V atravessando as paredes lateral, interna e a outra lateral, com vértice na face interna da placa; a placa é a dobradiça (extradorso), que estica.
  Abertura do V na face externa do intradorso `w = 2·(H−t)·tan h`; lado do V `(H−t)/cos h`. A patente [F12] risca a dobradiça para reduzir a espessura.
- Lista de corte: fórmula de chapa pela face externa da placa: `BA = Θ·(ri + K·t)`, `OSSB = (ri + t)·tan h`, `BD = 2·OSSB − BA`, `L_corte = X1 + X2 − BD`;
  posição do V; contorno do V por parede.
- Limites: a deformação da fibra externa da dobradiça `t/(2·ri + t)` pede `ri ≥ t·(1/ε_adm − 1)/2` (ε_adm = 0,25 dá ri ≥ 1,5 t); a dobra fica com raio externo pequeno
  (ri + t). Estrutural de carga só com ensaio [F13].
- Quando usar: canto vivo, peça única, pouca série.
- Solda: as três costuras do V (ponto de solda na patente [F12]).
- Exemplo 30×30×1,2, 90°, ri = 1,8, K = 0,33: BA = 3,449; OSSB = 3,000; BD = 2,551; w = 57,6; lado do V = 40,73; solda = 2·40,73 + 30 = 111,5 mm.

### M3. V múltiplo (segmentado)

- Entradas: Θ, raio da linha de articulações Rp (na face interna da placa), n.
- Geometria: n V iguais; `φ = Θ/n`, passo `p = 2·Rp·sen(φ/2)`, abertura `w = 2·(H−t)·tan(φ/2)`.
  **Condição nova**: as sobras entre V na face do intradorso não podem ser negativas: `p ≥ w`, ou seja, `Rp ≥ (H−t)/cos(φ/2)`. Com Rp = H − t o canto vivo é impossível nesse modo (as sobras
  somem): para canto vivo, M4.
- Lista de corte: por face interna da placa, `L = X1i + X2i − 2·s + (n−1)·p`, `s = Rv·sen((n−1)φ/2)/cos(Θ/2)`, `Rv = p/(2·sen(φ/2))`; posições a cada p.
- Limites: n pequeno gera polígono visível; n grande multiplica corte e solda; ri das articulações pelo mesmo critério de M2.
- Quando usar: raio grande em quadrado, sem dobradeira.
- Solda: n × (2 costuras laterais de `(H−t)/cos(φ/2)` + a interna de b).
- Exemplo 30×30×1,2, 90°, Rp = 45: n = 3: φ = 30°, p = 23,29, w = 15,43, solda ≈ 269 mm; n = 6: φ = 15°, p = 11,75, w = 7,58,
  sobra 4,16, Σp = 70,48 contra arco 70,69, s = 38,74, `L = X1i + X2i − 18,75`, solda ≈ 529 mm; n = 9: p = 7,84, w = 5,04, solda ≈ 790 mm. Rp mínimo para n = 6: 29,05.

### M4. Canto com gabarito (andorinha)

- Entradas: H (lateral), b, t, Ro (por padrão H), Θ, K ou neutra, folga e.
- Geometria: seção 7.3. Saída por lateral: contorno do corte (arcos, bissetrizes, liberação da placa); da placa: ℓ.
- Lista de corte: `L_corte = X1 + X2 − BD`, `BD = 2·Ro·tan h − ℓ`; zona começando em `X1 − Ro·tan h`; contorno em DXF/SVG para laser.
- Limites: Ro até cerca de 4,6 H em 90°; Θ de 30° a 150° testado; placa precisa ser forçada contra os arcos (retorno).
- Quando usar: canto de quadro com raio externo e canto interno vivo, tubo quadrado ou retangular de parede fina, sem dobradeira.
- Solda: arcos (placa nos setores), bissetriz de cada lateral, linha do canto interno. 30×30×1,2: 148,1 mm mais 30.

### M5. Corte simples com solda (esquadria e gomos)

- Entradas: Θ, perfil, n de juntas (redondo), CLR de referência.
- Geometria: corte a Θ/2 em cada ponta (esquadria); gomos: m juntas de φ = Θ/m, peças do meio com `2·CLR·tan(φ/2)` e meias nas pontas.
- Lista de corte: comprimento externo e interno por peça (diferença `H·tan h` em quadrado), ângulo de cada corte, ordem.
- Limites: canto externo vivo (sem raio), costura mais longa, resistência de junta soldada.
- Quando usar: canto vivo, série pequena, redondo com raio aproximado.
- Solda: toda a costura. Quadrado 30×30, 90°: 2·30·√2 + 2·30 = 144,85 mm.
- Exemplo redondo 25,4: uma esquadria de 45° tem elipse de semieixos 17,96 e 12,70, perímetro 97,03 mm. Gomos para raio 50,8, 90°:
  m = 2: zona 84,17, solda 166,2; m = 3: φ = 30°, meio 27,22, ponta 13,61, zona 81,67, solda 243,6; m = 4: zona 80,84, solda 322,3 (arco 79,80).
  O polígono fica uns 2% mais longo que o arco.

Retorno e neutra em todos os modos: parâmetro cadastrável; erro de desenvolvido `δ·Θ` para um deslocamento de neutra δ [C]
(com δ = 1 mm e 90°, 1,57 mm).

## 9. Fontes novas desta rodada

Referências [F25] a [F26] estão em `tubo_fontes.md` para a procedência. Nenhuma norma paga foi copiada.

## 10. Proposta de itens de backlog (bancada Estrutural)

Tamanho em milhares de tokens (mil tokens), estimativa para o especialista com testes.

| # | Título | O que faz | Teste | Tamanho |
|---|---|---|---|---|
| 1 | Nó "dobra em nó" com modos | Estende o nó entre dois membros com modo "tubo único com dobra": dobra por dobradeira, por corte (V ou rasgo), corte + dobradeira, corte simples. Redondo ou quadrado, raio como parâmetro. Mantém a árvore transparente e a prévia no 3D (padrão B319). | Dois membros a 90° com raio dado: um sólido só, volume dentro de tolerância do esperado, caixa envolvente conferida; modo corte simples resulta em dois membros | 80 |
| 2 | Cálculo do desenvolvido e do alongamento | Calcula arco no eixo, comprimento de corte total, dedução de dobra, alongamento da fibra externa e afinamento estimado; avisa quando passa do limite da regra. | Caso numérico do item 3 (D 50, CLR 100, 90°): 157,08 mm; corte 557,08 mm; alongamento 25%; afinamento 20% | 30 |
| 3 | Corte em V paramétrico | Gera n entalhes, ângulo por entalhe, passo e abertura a partir de ângulo total, raio e perfil quadrado; deixa a parede dobradiça e o ponto de solda. | Perfil 40x40x2, 90°, n = 6: phi = 15°, w e p conferem com as fórmulas; volume removido bate com a soma dos V; documento recalcula | 60 |
| 4 | Gabarito imprimível | Exporta o gabarito de marcação em escala 1:1 (vetor) com passo, abertura e linhas de corte; vai para a folha de desenho. | PDF/DXF com cotas iguais à tabela; medir o passo e a abertura na saída | 40 |
| 5 | Regras e tabelas cadastráveis | Cadastro por material, D, t, processo e ferramenta: CLR mínimo, retorno elástico (graus), afinamento medido, sobrecomprimento. Igual ao padrão de tabelas já usado em chapa, guiado por parâmetros. Valores vêm do usuário. | Regra ativa recusa raio abaixo do mínimo e mostra o motivo; trocar de regra muda o desenvolvido | 50 |
| 6 | Lista de corte com desenvolvido | Inclui na lista de corte o comprimento desenvolvido e as pontas de pega; agrupa por perfil e por dobra. | Peça com 2 trechos retos e 1 dobra: linha única com 557,08 mm | 25 |
| 7 | Dados para máquina (LRA/YBC/XYZ) | Exporta tabela de dobra por rota: avanço, rotação, ângulo, raio, coordenadas; com opção absoluto e incremental; compensa retorno pela regra. | Rota planar com 2 dobras e rota 3D: converter LRA para XYZ e voltar reproduz a rota (erro < 1e-6 mm) | 60 |
| 8 | Tradução e tooltips | Textos novos em `i18n.rs`, dicas por campo, GIF de tooltip, ícone da família. | Teste de chaves de tradução presentes | 20 |

Ordem sugerida: 2 e 5 (sem geometria nova), depois 1, 3, 6, 4 e 7.

### Efeito da segunda rodada sobre os itens já abertos

- **B326** muda: a lista de modos passa a ter M1 a M5 da seção 8 (inclui "canto com gabarito" e gomos para redondo) e a
  pergunta de qual dimensão fica no plano da dobra no retangular.
- **B327** muda: usar neutra `raiz(rf·Ro)` ou K cadastrado, e `L_corte = X1 + X2 − BD` por modo; `ε_adm` e o critério `CLR ≥ (D/2)/ε_adm`.
- **B328** muda: acrescentar a condição `Rp ≥ (H−t)/cos(φ/2)`, o comprimento por face interna e as fórmulas de n, φ, p, w da seção 8 (M2, M3).
- **B329** muda: o gabarito imprimível cobre também o canto (arcos de raio Ro nas pontas, V de 90° da bissetriz, escala pela altura lateral).
- **B330** muda: campos novos `ε_adm`, K da placa, folga de raiz e, Rp mínimo.
- **Falta item novo**: "Estrutural: canto com gabarito (andorinha)", cerca de 220 mil tokens: contorno do corte por lateral, placa, prévia aberto e fechado, solda; teste:
  30×30×1,2 a 90° dá ℓ = 46,17, BD = 13,83, folga 5,44, solda 148,1. E "contorno plano do corte em DXF/SVG para laser", cerca de 100 mil tokens.

## 11. Folgas de dobra e de solda (B442)

Pedido depois do protótipo real (B328, B329, B333 fecharam como o gabarito). Derivação própria, sem
fonte nova:

- **Folga de dobra `c`** (só V): o vértice fica na face interna da placa e a abertura na parede
  interna passa de `w = 2·(H−t)·tan(φ/2)` para `w + c`. Cada lado do V gira `δ = atan(tan(φ/2) +
  c/(2·(H−t))) − φ/2` em torno do vértice; fechado, sobra uma cunha (zero na dobradiça), então as
  faces não se travam.
- **Folga de solda `e`** (V e canto com gabarito): as faces fechadas ficam paralelas a `e`. No tubo
  aberto cada lado do V recua `e/(2·cos(φ/2))` na direção do tubo e o vértice vira um fundo reto de
  `e/cos(φ/2)` (1,41·e a 90°). No canto, a bissetriz recua `e/2` (seção 7, já existente).
- No V múltiplo, `w + c + e/cos(φ/2)` não pode passar do passo `p`, senão os rasgos se encostam.
- Valores 0 reproduzem a geometria anterior (testes `folgas_zero_*`).
