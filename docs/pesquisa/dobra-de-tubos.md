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

## 7. Proposta de itens de backlog (bancada Estrutural)

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
