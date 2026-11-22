# ADR 0108 — Curva exata no kernel e definição única da spline

Estado: aceito (22/11/2026). Implementa o item 4 do ADR 0107 (curva exata e Extrair continua Extrair).
Itens: B559, B576.

## Contexto

Toda curva de entrada de superfície, de chapa e de varredura chegava ao kernel como polilinha densa
(`Curve3::pts`). O kernel refazia uma B-spline só se nenhum canto passasse de 20°; com um canto, a
curva inteira virava N retas. Círculo de sketch virava 64 pontos (a extrusão não era cilindro), a
aresta extraída virava polilinha (perdia o tipo) e a aba curva refazia a borda do perfil de asa por
heurística, com uma faixa por trecho. A spline do sketch tinha três definições (Hermite no solver,
Catmull-Rom no desenho, `GeomAPI_Interpolate` no sólido).

## Decisão

1. **A curva carrega a geometria exata.** `Curve3` ganha `exata: Option<Curva3>`. `Curva3`
   (`fenkra-kernel/src/curva.rs`) é uma lista de trechos: reta, arco por três pontos, círculo, elipse,
   B-spline (polos, nós, pesos), spline pelos pontos (`Interpolada`) e, como último recurso, polilinha
   (`Pontos`). A polilinha `pts` fica para desenhar, medir e escolher.
2. **Ponte.** A curva vai ao C++ como `&[f64]` codificado (`[NaN, n, (tipo, tamanho, dados)…]`). O NaN
   inicial separa da lista crua antiga, que continua aceita. `wire_curva` (`cpp/curva.inc`) faz uma
   aresta exata por trecho: cantos entre trechos são G0, sem virar "tudo reto". Na polilinha crua, os
   trechos suaves entre cantos viram B-spline (antes, um canto bastava para tudo virar reta).
3. **Fontes exatas.** Reta, círculo e arco do wireframe; curva do sketch (`EvalSketch::exatas`);
   aresta do modelo (`Shape::edge_curve_at`, que lê a `Geom_Curve` da aresta: reta, círculo, elipse ou
   B-spline); spline e spline por controle; Conectar (Bézier de grau 3, 5 ou 7); elipse e parábola;
   canto entre retas. A polilinha segue como pontos (já servia de contorno suave): o kernel quebra só
   nos cantos acima de 20°. Junção e Extrair de várias encadeiam os trechos no
   sentido certo; se uma peça não tem forma exata, ou as pontas exatas não coincidem, a junção vai como
   polilinha.
4. **Consumidores.** Extrusão, revolução, varredura, loft, preenchimento e corte por curva de
   superfície; varredura sólida; membro estrutural curvo; aba curva e aba por superfície (a cadeia sai
   das arestas da chapa, com a tangente do começo tirada da curva exata).
5. **Definição única da spline pelos pontos** = a interpolação do kernel (`GeomAPI_Interpolate`):
   parâmetro pelo comprimento das cordas, C2 nos pontos internos e, nas pontas, a derivada do
   polinômio de Lagrange pelos quatro pontos da ponta (parábola com três pontos, reta com dois). A
   réplica em Rust (`fenkra-core/src/spline.rs`) serve ao desenho, à prévia, ao inserir ponto e ao
   wireframe, e o teste
   `spline_unica_igual_ao_kernel` a confere contra o kernel (desvio abaixo de 1e-5 mm). A spline
   fechada do wireframe é a cúbica C2 periódica pelas cordas, mandada pronta como B-spline: a
   interpolação periódica do kernel não é essa curva (sai torta numa curva simétrica).
6. **Extrair continua Extrair (B576).** Extrair várias curvas de uma vez cria um `CurveCopy` com as
   outras em `mais` (antes, uma Junção de curvas). Aresta extraída e borda mantêm o tipo pela curva do
   B-rep.

## Consequências

- Círculo extrudado é cilindro (face analítica), spline vira uma face B-spline, aba curva no perfil de
  asa sai com 6 faces novas em vez de 18 e calcula em segundos (antes, mais de dois minutos).
- A aba só usa a cadeia exata quando ela é lisa (G1); com canto vivo entre trechos fica a polilinha.
- A dobra por linha curva fecha a região além da curva pela saída média (com a parábola de três
  pontos, as pontas inclinadas faziam a região dar a volta na chapa).
- Varredura com perfil fixo: a tangente inicial da spline do wireframe mudou, e o perfil é varrido
  com a inclinação que tem em relação a ela (dois testes do corpus passaram a esperar π·L·cos θ).
- Resultados que dependiam da polilinha mudam: curva amostrada com um canto vivo deixa de virar retas
  inteira (só os cantos quebram), spline do wireframe sem tangente troca a Catmull-Rom pela interpolação do kernel (a forma muda pouco,
  os pontos são os mesmos), desenho da spline do sketch passa a coincidir com o sólido.
- Arquivos antigos abrem: o modelo de dados só ganhou `CurveCopy::mais` (opcional); a curva exata é
  calculada, não gravada.
- Aparar e Estender do sketch seguem a Hermite de parâmetro uniforme do solver (e dos pontos de controle,
  B549): para manter a forma ao aparar, eles gravam tangentes próprias nesse modelo, que não reproduz
  exatamente a spline pelas cordas. Na spline sem tangente própria, o ponto de corte pode ficar um pouco
  fora da curva desenhada. Fechar isso pede tangente pelo comprimento das cordas no modelo da spline do
  sketch (bancada Sketch).
- Ficam como polilinha, por ora: hélice, espiral, hipérbole, curva paralela, curva projetada, concordância
  e preenchimento com condição G1/G2 (que medem por amostras), a aba que recua (deslocar a curva no
  plano) e a aba por superfície com limites dos lados (a OML aparada não coincide com a aresta da web).
