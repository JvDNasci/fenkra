# Benchmark de detalhe — Superfícies e wireframe

> Texto próprio. Compara cada comando da bancada com softwares de modelagem CAD avançados, Blender e FreeCAD,
> no nível do campo e do gesto. Aprofunda [superficies-e-wireframe.md](../superficies-e-wireframe.md) e
> [superficies.md](../../pesquisa/superficies.md), que já tratam de kernel e paridade. O padrão comum de prévia,
> alça, dica e mídia está em [interface.md](interface.md); aqui fica o que é próprio de superfícies. Referências
> de código foram lidas, não executadas, salvo quando dito.

## 1. Resumo: as 5 maiores lacunas de experiência

1. **O comando não mostra o que vai fazer.** Abrir um comando cria o elemento real no documento
   (`app.rs:1850`). Só o recálculo automático (`app.rs:6828`) desenha algo, e só com todos os campos
   preenchidos. Não há fantasma nem realce dos suportes: `scene.rs` e `render.rs` não consultam a sessão aberta.
   A pessoa não vê normal, lado, início da curva nem qual borda foi escolhida.
2. **Alça 3D existe em poucos comandos e a do offset engana.** Só 7 elementos de geometria têm seta
   (`handles.rs:126-168`), mais o gizmo de pontos e transformações (`gizmo.rs:252`). O caso citado pelo usuário
   tem defeito: a direção vem da diferença entre centros de malha (`handles.rs:155`), não da normal. Com distância
   negativa, a seta aponta para o lado oposto; com "dos dois lados", os centros coincidem e ela some.
3. **Continuidade é promessa, não controle.** Preenchimento passa toda aresta como C0 e sem face de apoio
   (`surfaces.inc:54-60`). Concordância reamostra as bordas em 24 pontos (`lib.rs:850`) e o G2 é aproximado, como o
   painel admite. Multi-seções não tem condição nas pontas. Nada disso aparece no 3D.
4. **Lado, borda e solução se escolhem por caixa ou número.** "Outro lado da 1", "manter o outro lado",
   "Solução", número da borda (`geo_ui.rs:1578`) e ponto "perto da borda" (`geo_ui.rs:1609`) pedem que a pessoa
   imagine o resultado. Os softwares avançados resolvem isso com clique na região e seta que inverte.
5. **Não existe modelagem direta.** Nenhuma curva ou superfície tem nós arrastáveis: Spline e Spline controle são
   listas de pontos no painel (`geo_ui.rs:1131`, `geo_ui.rs:1284`). Falta o caminho caixa, nó, eixos e suavizar que o
   usuário procurou (seção 5).

## 2. Como a bancada funciona hoje

- **Barra:** grupos Wireframe, Superfícies, Operações, Híbrido e Análise (`toolbar.rs:416-485`), com famílias por
  seta. O catálogo fica em `GeoKind::info` (`geo_ui.rs:120-199`).
- **Tooltip:** nome e uma frase; o texto avançado só surge após 5 s parado (`icons.rs:411`). Sem imagem nem atalho.
- **Campo de valor:** a dica é a mesma em todos, sobre expressões (`editor.rs:143`). O rótulo de seleção não tem dica
  (`geo_ui.rs:612`); só o botão ⊙ tem texto genérico (`geo_ui.rs:620`).
- **Seleção:** campo vazio pede o tipo certo (`slots.rs:39`) e o clique preenche o próximo campo. Essa é a melhor
  parte do fluxo atual.
- **Ícones:** avaliados em folha de contato gerada de `assets/icons`. Pares quase iguais: `sup_preenchida` e
  `op_contorno`; `sup_filete` e `sup_filete_aresta`; `sup_varredura` e `sup_varredura_av`; `sup_cilindro` e
  `hib_fechar`; `sup_extraida`, `extrair`, `op_multi_extrair` e `curva_extraida`.

## 3. Checklist por comando

87 comandos. Critérios:

- **Prévia 3D:** ok = resultado ao vivo com marca do parâmetro no 3D; parcial = resultado só depois de preencher
  tudo, sem marca de lado ou direção; falta = nada no 3D.
- **Manipulação 3D:** ok = cobre os valores principais; parcial = cobre parte.
- **Tooltip:** ok = diz o que faz e o que escolher, ainda sem mídia. Nenhum comando tem GIF.
- **Dica por campo:** parcial = nota no rodapé ou opção com dica; ok = comando sem campos.
- **Fluxo:** ok = seleção e cliques bastam; falta = exige comando anterior ou número.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| **Superfícies** | | | | | | | | |
| Extrudada | parcial | parcial | parcial | ok | parcial | parcial | ok | Seta só no Limite 1 |
| Revolução | parcial | parcial | falta | ok | falta | parcial | ok | Sem arco de ângulo |
| Esfera | ok | ok | ok | ok | ok | falta | ok | Seta fixa em X |
| Cilindro | parcial | ok | parcial | parcial | parcial | parcial | ok | Sem alça de raio; ícone igual a Fechar |
| Deslocada | parcial | parcial | parcial | ok | ok | falta | ok | Seta falha com valor negativo e dois lados |
| Varredura | parcial | parcial | falta | parcial | parcial | parcial | parcial | Modos Frenet são jargão |
| Varredura+ | falta | parcial | falta | falta | parcial | parcial | falta | 7 tipos em lista; ícone igual a Varredura |
| Tubo | ok | parcial | falta | ok | ok | falta | ok | Raio sem alça |
| Multi-seções | parcial | parcial | falta | parcial | parcial | parcial | parcial | Sem conectores nem condição nas pontas |
| Concordância | falta | parcial | falta | parcial | parcial | parcial | falta | Exige Contorno ou Curva extraída antes |
| Conectar | parcial | parcial | falta | parcial | ok | parcial | parcial | Bordas adivinhadas, sem confirmação visual |
| Filete | parcial | parcial | falta | falta | parcial | falta | parcial | Lado por caixa, sem seta |
| Filete aresta | parcial | parcial | falta | falta | parcial | parcial | falta | Exige Juntar antes; ícone igual a Filete |
| Preenchimento | parcial | parcial | falta | falta | falta | parcial | parcial | Só G0; ícone igual a Contorno |
| **Operações** | | | | | | | | |
| Juntar | parcial | parcial | falta | ok | parcial | falta | parcial | Sem relatório de folgas |
| Curar | parcial | parcial | falta | parcial | parcial | parcial | ok | Não mostra o que fechou |
| Dividir | parcial | parcial | falta | ok | parcial | falta | parcial | Lado por caixa, não por clique |
| Aparar | parcial | parcial | falta | falta | parcial | falta | parcial | Ícone não mostra corte; lado por caixa |
| Extrair | ok | parcial | falta | falta | ok | falta | ok | Propagação só na interface |
| Extraída | parcial | parcial | falta | falta | parcial | parcial | parcial | Botão extra para usar a face |
| Multi-extração | parcial | parcial | falta | falta | parcial | parcial | parcial | Lista manual de faces |
| Contorno | parcial | parcial | falta | falta | parcial | parcial | falta | Borda escolhida por número |
| Desaparar | parcial | parcial | falta | ok | parcial | falta | ok | Sem prévia do recorte perdido |
| Desmontar | falta | parcial | falta | ok | parcial | parcial | falta | Face escolhida por número |
| Extrapolar | parcial | parcial | falta | ok | parcial | falta | parcial | Borda por ponto; sem seta |
| Mover | ok | ok | ok | parcial | parcial | parcial | ok | Gizmo completo |
| Girar | ok | ok | ok | ok | parcial | parcial | ok | Gizmo completo |
| Simetria | parcial | parcial | falta | ok | parcial | parcial | ok | Sem plano realçado |
| Escala | ok | ok | ok | ok | parcial | parcial | ok | Gizmo completo |
| **Híbrido** | | | | | | | | |
| Dividir corpo | parcial | parcial | falta | ok | ok | falta | parcial | Lado sem seta |
| Engrossar | parcial | parcial | falta | ok | ok | parcial | ok | Sem seta de espessura |
| Fechar | parcial | parcial | falta | falta | parcial | parcial | parcial | Não mostra as bordas abertas |
| Costurar | parcial | parcial | falta | parcial | parcial | parcial | parcial | Lado por caixa |
| Preencher | falta | parcial | falta | falta | parcial | falta | falta | Células por ponto, sem cor |
| Malha | parcial | falta | falta | ok | parcial | falta | parcial | Janela de importação |
| **Análise** | | | | | | | | |
| Zebra | ok | ok | falta | ok | ok | ok | ok | Sem ajuste de faixas |
| Curvatura | parcial | ok | falta | ok | ok | ok | ok | Depende do zoom; sem escala |
| **Wireframe: pontos** | | | | | | | | |
| Coordenadas | ok | ok | ok | ok | ok | falta | ok | Gizmo completo |
| Na curva | ok | parcial | falta | parcial | ok | falta | ok | Sem ponto arrastável na curva |
| No plano | parcial | parcial | falta | ok | parcial | falta | ok | u, v sem eixos no 3D |
| Na superfície | parcial | parcial | falta | ok | parcial | falta | parcial | u, v sem eixos no 3D |
| Centro | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Tangente | parcial | parcial | falta | ok | parcial | falta | parcial | Dois extremos sem escolha visual |
| Entre | ok | parcial | falta | ok | ok | falta | ok | Proporção sem alça |
| Curva × plano | parcial | parcial | falta | ok | parcial | falta | parcial | Solução por número |
| Curva × curva | ok | parcial | falta | ok | parcial | falta | ok | Direto |
| Projeção | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Na curva (proj.) | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Deslocado | ok | ok | ok | ok | ok | falta | ok | Seta com cota |
| Repetir pontos | parcial | parcial | falta | parcial | parcial | falta | parcial | Sem marcas antes de concluir |
| **Wireframe: linhas e curvas** | | | | | | | | |
| Ponto-ponto | ok | parcial | falta | ok | ok | falta | ok | Prolongamento sem alça |
| Ponto-direção | ok | ok | parcial | ok | ok | falta | ok | Seta só no fim |
| Normal à sup. | ok | parcial | falta | ok | ok | falta | ok | Comprimento sem alça |
| Ângulo/normal | parcial | parcial | falta | ok | ok | falta | parcial | Ângulo sem arco |
| Tangente | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Planos × plano | ok | parcial | falta | parcial | ok | falta | ok | Direto |
| Bissetriz | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Eixo de face | parcial | parcial | falta | ok | ok | parcial | parcial | Botão extra para usar a face |
| Polilinha | ok | parcial | falta | ok | ok | falta | ok | Pontos não arrastam |
| Juntar curvas | parcial | parcial | falta | ok | ok | parcial | parcial | Não realça a cadeia |
| Círculo | ok | parcial | falta | ok | ok | falta | ok | Raio sem alça |
| Círculo 3P | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Cônica | parcial | parcial | falta | ok | parcial | falta | parcial | a e b sem marcas |
| Spline | parcial | parcial | falta | parcial | parcial | falta | parcial | Pontos não arrastam; sem tangência |
| Spline controle | parcial | parcial | falta | ok | ok | falta | parcial | Polígono não aparece para editar |
| Conexão | parcial | parcial | falta | ok | ok | falta | parcial | Tensão sem alça; ponta por rádio |
| Canto | ok | parcial | falta | ok | ok | falta | ok | Raio sem alça |
| Hélice | parcial | parcial | falta | ok | parcial | falta | parcial | Passo e altura sem alças |
| Espiral | parcial | parcial | falta | ok | parcial | falta | parcial | Raios sem alças |
| Spine | falta | parcial | falta | parcial | falta | falta | parcial | Termo sem explicação |
| Curva na superfície | parcial | parcial | falta | ok | ok | falta | parcial | Direção sem seta |
| Curva no plano | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Interseção | ok | parcial | falta | ok | ok | falta | parcial | Solução por número |
| Paralela | parcial | parcial | falta | ok | ok | parcial | parcial | Sem seta de lado |
| **Wireframe: planos** | | | | | | | | |
| Deslocado | ok | ok | ok | ok | ok | falta | ok | Referência da bancada |
| Paralelo/ponto | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Ângulo/normal | parcial | parcial | falta | parcial | ok | falta | parcial | Sem arco de ângulo |
| 3 pontos | ok | parcial | falta | ok | ok | falta | ok | Direto |
| 2 linhas | ok | parcial | falta | parcial | ok | falta | ok | Ícone pouco legível |
| Ponto e linha | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Ponto + normal | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Plano médio | parcial | parcial | falta | falta | ok | falta | ok | Nome e ícone confundem com Médio |
| Curva planar | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Normal à curva | ok | parcial | falta | ok | ok | falta | parcial | Posição sem ponto arrastável |
| Tangente | ok | parcial | falta | ok | ok | falta | ok | Direto |
| Equação | parcial | parcial | falta | ok | ok | falta | ok | Numérico por natureza |
| Médio | parcial | parcial | falta | falta | parcial | falta | ok | Nome confunde com Plano médio |

| Critério | ok | parcial | falta |
|---|---|---|---|
| Intuitivo | 36 | 46 | 5 |
| Prévia 3D | 11 | 75 | 1 |
| Manipulação 3D | 7 | 4 | 76 |
| Ícone claro | 59 | 15 | 13 |
| Tooltip | 47 | 37 | 3 |
| Dica por campo | 2 | 25 | 60 |
| Fluxo | 47 | 34 | 6 |
| **Total** | **209** | **236** | **164** |

## 4. Comandos mais usados

Formato: o que os outros têm; como deveria ser no Fenkra, na ordem suporte, modo, parâmetros e avançado; o que
aparece no 3D; roteiro do GIF.

### Deslocada (offset de superfície)

**Outros:** a cópia aparece paralela à referência assim que ela é escolhida. O lado inverte clicando na seta, arrastando
a alça para trás da superfície ou pelo menu do botão direito. Há opção de dois lados e de gerar sólido. A cópia pode
ser de várias faces de uma vez.

**Fenkra:** suporte: faces ou superfície, em lista. Modo: um lado, dois lados, sólido. Parâmetros: distância e
botão Inverter. Avançado: ligação e faces a excluir.

**3D:** cópia translúcida; seta saindo do ponto clicado, na normal real (`surface_near`, `surfaces.inc:88`);
rótulo "Distância 2 mm" arrastável; duas setas no modo dois lados. Passar pelo zero inverte sem saltar. Se falhar por
curvatura, marcar em vermelho a região com raio menor que a distância.

**GIF (7 s):** clicar numa folha curva; surge a cópia; puxar a seta; atravessar para o outro lado; digitar 5; ligar dois lados.

### Extrudada e Revolução

**Outros:** duas setas, uma por lado, com término por valor ou até elemento; na revolução, arco arrastável com ângulo.

**Fenkra:** suporte: curva. Modo: um lado, dois lados, simétrico. Parâmetros: limites 1 e 2 ou ângulos 1 e 2 (B14).
Avançado: conicidade, tampar pontas.

**3D:** seta por limite (hoje só `length`, `handles.rs:134`), eixo realçado em toda a extensão, arco reaproveitando
`gizmo::rotate_arc`. Passar sobre um elemento com a seta o propõe como limite.

**GIF (6 s):** clicar na curva; puxar a seta para cima e para baixo; na revolução, clicar no eixo e girar o arco até 270°.

### Varredura e Varredura+

**Outros** (conhecimento geral e pesquisa anterior, sem fonte nova nesta rodada): perfil visível no início da guia,
orientação e torção como opções, modos nomeados pelo efeito e não pela matemática.

**Fenkra:** unir os dois comandos. Suporte: perfil e guias. Modo em figuras: "Perfil + guia", "Entre guias", "Arco",
"Tubo", "Cônica". Parâmetros do modo. Avançado: spine, seções, suavizar. Trocar "Frenet corrigido" por "Acompanha sem
torcer", com o termo técnico na dica.

**3D:** guia colorida com seta de sentido; 3 fantasmas do perfil ao longo da guia, atualizados ao trocar o modo; bolinhas
para escolher a solução em vez de "1, 2, 3, 4".

**GIF (8 s):** clicar perfil e guia; passar pelos três modos e ver os fantasmas girarem; aceitar.

### Multi-seções (loft)

**Outros:** condição em cada ponta (nenhuma, normal, tangente, casar tangência, casar curvatura) com magnitude
positiva ou negativa; guias com continuidade própria; caminho com número de seções; opção de aparar nas guias.
Conexões entre seções aparecem no 3D em cor própria e arrastam, controlando a torção. Isocurvas com densidade ajustável.
Perfil inválido fica vermelho. Conectores também invertem pelo botão direito.

**Fenkra:** suporte: seções em ordem, com reordenar por arraste. Modo: suave, regrada, fechada. Parâmetros: condição e
tensão por ponta (B12). Avançado: guias, spine, conexões, isocurvas.

**3D:** número e seta de sentido em cada seção; conectores nos pontos de fechamento, arrastáveis ao longo da curva; seta de tangência
por ponta com tamanho igual à magnitude. Isso é o que mais evita torção, a queixa clássica desse comando.

**GIF (9 s):** clicar três seções; surge a superfície torcida; arrastar um conector e ela se desfaz; puxar a seta da ponta.

### Concordância e Conectar

**Outros:** a pessoa clica direto na borda da superfície; o apoio vem junto. Continuidade por borda, tensão por lado,
pente de curvatura com escala em cada direção. O comando de casar borda ainda oferece preservar a ponta oposta, média entre
as duas superfícies, refino até a tolerância e direção das isocurvas.

**Fenkra:** unir a família. Suporte: duas bordas, aceitando aresta clicada e deduzindo o apoio, sem passar por Contorno
(`geo_ui.rs:1518`). Modo: G0, G1, G2 com nomes "Encosta", "Tangente", "Curvatura". Parâmetros: tensão 1 e 2. Avançado:
aparar apoios, pente.

**3D:** bordas realçadas, seta de tangência em cada uma, arrastada para mudar a tensão; etiqueta de continuidade clicável na
borda; pente opcional. Conectar deve mostrar quais bordas deduziu (`facing_edge`) e aceitar troca por clique.

**GIF (7 s):** clicar duas bordas; ponte G0 reta; clicar na etiqueta para G1 e G2; puxar a seta da tensão.

### Preenchimento

**Outros:** continuidade por aresta (posição, tangência, curvatura) com face adjacente; peso da tangência; aplicar a todas ou
aresta por aresta; guias internas e pontos de passagem; seleção em cadeia; pontos vermelhos onde o contorno abre; isocurvas.
No FreeCAD a prévia surge quando o contorno fecha, e curvas e vértices internos forçam a passagem. Documentações
recomendam superfície de quatro lados aparada para áreas grandes e preenchimento só para buracos pequenos.

**Fenkra:** suporte: contorno em cadeia (B09). Modo: "Plano simples", "Seguir vizinhas". Parâmetros: continuidade por
aresta e peso. Avançado: guias, pontos, tolerância.

**3D:** bola vermelha em cada ponta aberta, com contagem; etiqueta G0, G1 ou G2 em cada aresta; ponto de passagem arrastável.

**Kernel:** `BRepOffsetAPI_MakeFilling` já aceita face de apoio por aresta, ordem de continuidade e pontos.
A lacuna é da fachada, que usa só `GeomAbs_C0` e recebe polilinhas (`lib.rs:1279`). Por isso a superfície não fica
tangente às vizinhas, mesmo com zebra ligada.

**GIF (8 s):** clicar uma aresta e a cadeia fecha; superfície surge; clicar a etiqueta para G1; zebra passa a fluir; puxar um ponto interno.

### Juntar, Curar e Fechar

**Outros:** controle de folgas lista pares de arestas com problema dentro de uma faixa, com tolerância editável; opção de
unir faces do mesmo suporte; opção de formar sólido na hora.

**Fenkra:** suporte: faces em lista. Modo: juntar, juntar e formar sólido. Parâmetros: tolerância. Avançado: faixa de folga,
unir faces. Curar vira opção do mesmo painel.

**3D:** bordas livres em laranja, folgas com rótulo em mm; ao aumentar a tolerância, cada folga fica verde ao fechar. Mensagem final:
"1 corpo, 2 bordas livres, maior folga 0,03 mm".

**GIF (6 s):** selecionar 4 folhas; bordas laranjas; subir a tolerância; ficam verdes; marcar "formar sólido".

### Dividir e Aparar

**Outros:** dois tipos (por ferramenta e mútuo) e ação de manter ou remover pedaços, escolhidos por clique. No modo mútuo há
prévia do que entra e do que sai.

**Fenkra:** unir num comando. Suporte: superfícies. Modo: "Por ferramenta", "Mútuo". Parâmetros: pedaços. Avançado:
estender ferramenta, manter tudo em corpos separados.

**3D:** pedaços em duas cores, cheio para o que fica, translúcido para o que sai; clicar alterna. O clique grava âncora espacial
(mesma ideia do B76), substituindo "outro lado da 1".

**GIF (6 s):** duas folhas cruzadas; clicar no pedaço a remover; ele fica translúcido; clicar outro; aceitar.

### Extrapolar

**Outros:** seta laranja na borda; distância negativa apara; termina em valor, face, superfície ou vértice; manter curvatura;
propagar por tangência; a superfície inteira move todas as bordas.

**Fenkra:** suporte: borda clicada, sem o ponto "perto de". Modo: tangente ou mesma curvatura. Parâmetros: distância ou até elemento (B14).
Avançado: propagação.

**3D:** seta na borda, faixa nova translúcida, rótulo arrastável, valor negativo mostrando a faixa a perder em vermelho.

**GIF (5 s):** clicar uma borda; puxar a seta para fora; puxar para dentro; aceitar.

### Filete de superfície

**Fenkra:** lado escolhido por seta em cada superfície, não por caixa; alça de raio na aresta, como no filete de sólido
(`handles.rs:74`); Filete aresta aceitando superfícies ainda não juntadas. A seta mostra de que lado o arco fica, a informação
que hoje exige tentativa.

**GIF (6 s):** clicar duas folhas; setas aparecem; inverter uma; puxar o raio.

### Engrossar

**Fenkra:** seta por lado sobre a normal da superfície, com rótulo; hoje o painel só diz "lado da normal", que ninguém vê. Normal
visível em todos os comandos de superfície, com clique para inverter.

**GIF (5 s):** clicar na folha; seta azul mostra a normal; puxar; inverter.

### Spline 3D e Spline controle

**Outros** (pesquisa anterior): pontos arrastáveis, tangente e curvatura por ponto, polígono de controle visível. No FreeCAD, curva de
concordância paramétrica entre duas arestas com continuidade escolhida.

**Fenkra:** suporte: pontos, por clique em sequência no 3D. Modo: por pontos ou por polos, com troca sem perder a forma. Parâmetros: fechada,
grau. Avançado: tangência por ponto e tensão.

**3D:** pontos como esferas; clique abre o gizmo; alça de tangente com tamanho igual à tensão; pente de curvatura. Ponto vindo de
referência fica preso, com ícone de vínculo.

**GIF (7 s):** clicar 4 pontos; arrastar um; ligar tangência no último e girar a alça.

## 5. Modelagem orgânica intuitiva

### Quem faz o quê

| Tema | Blender | FreeCAD | Módulos de forma livre dos CADs avançados |
|---|---|---|---|
| Base | Malha com modificador de subdivisão Catmull-Clark, níveis de visão e de render | Bancada Surface com preenchimento, seções e curva sobre malha; bancada externa de curvas NURBS; subdivisão não consta nelas | Caixa de controle sobre superfície lisa, ou polos NURBS |
| Mover | Gizmo e teclas por vértice, aresta e face | Pouca edição direta | Manipulador único de mover, girar e escalar, com espaços do mundo, da vista, da seleção e local por elemento |
| Vizinhança | Edição proporcional: raio pela roda, curva de queda, só conectados | Não consta na bancada Surface | Modificação suave por distância ou número de faces, transição e peso |
| Vinco | Peso por aresta | Não consta na bancada Surface | Vincar e desvincar; aproximar arestas deixa o canto mais vivo |
| Inserir | Corte em laço com deslize | Não consta na bancada Surface | Inserir aresta exata (preserva a forma, cria mais faces) ou simples; arrastar com tecla cria faces |
| Simetria | Modificador com recorte no plano e fusão por distância | Não consta na bancada Surface | Simetria ligada à forma |
| Ligar ao CAD | Nenhum vínculo paramétrico | Tudo paramétrico, pouco direto | Encaixe de nó em corpos, conversão para sólido; relatos de usuários sobre continuidade nos nós irregulares |

**Leitura:** o Blender dá o gesto, o CAD avançado dá a exatidão e o FreeCAD mostra que só paramétrico não basta. O Fenkra pode
somar os três: **gesto do Blender, sem atalhos ocultos, dentro da árvore paramétrica**. Editar polos NURBS sozinho não resolve:
a grade de polos é retangular, então um ponto novo obriga a uma fileira inteira. Por isso a forma orgânica nasce em subdivisão (B153)
e polos ficam para refino de curvas e faces (B154).

### Fluxo ideal

1. **Criar a caixa.** Bancada Forma livre, Caixa. Clicar no plano e arrastar o tamanho. Painel com faces por lado; lisa por padrão.
   A árvore ganha "Forma 1". Uma bolha de ajuda diz: "Clique num nó".
2. **Clicar num nó.** Passar o mouse acende nó, aresta ou face sem trocar de ferramenta. A gaiola fica em linhas finas sobre a superfície
   lisa; tecla Tab alterna gaiola, liso e ambos.
3. **Mover pelos eixos.** Surge o manipulador que já existe em `gizmo.rs`: setas X, Y e Z, quadrados planares e arcos. O seletor de
   espaço fica em chip no 3D: Mundo, Vista, Normal, Local. Rótulo mostra o deslocamento e aceita digitar. Arrastar o nó direto move no plano da vista.
4. **Suavizar a vizinhança.** Com a roda durante o arraste, muda o raio de influência, desenhado como círculo com gradiente. Chip "Só
   conectados" evita puxar a outra asa.
5. **Criar e cortar.** Selecionar face e arrastar com Alt extruda. Corte em laço com prévia e deslize. Duplo clique seleciona laço;
   Shift duplo clique, anel.
6. **Tangência no nó.** Chip de três estados, com figura: **Suave**, **Vinco**, **Canto**.
   - Suave é o padrão: superfície contínua no nó.
   - Vinco tira a tangência ao longo da aresta, com peso de 0 a 1 em controle deslizante e prévia ao vivo.
   - Canto faz o nó ser ponta.
   - **Colocar tangência:** alça de disco no nó define o plano tangente. O sistema move o anel vizinho por mínimos quadrados, pois na
     superfície limite o plano tangente depende desses vizinhos. Tamanho da seta é a tensão.
7. **Suavizar bordas.** Selecionar aresta, comando Suavizar borda, com raio e segmentos, ou aproximar duas arestas. Mesma lógica de chanfro de
   aresta da forma livre, com local e número de faces.
8. **Vínculos.** Botão direito no nó, Vincular a:
   - **ponto, curva, plano ou face** do modelo: o nó segue a referência, com ícone de corrente, e fica restrito a deslizar nela;
   - **parâmetro:** distância do nó a um plano vira cota com nome;
   - **borda:** aresta da forma casa com aresta B-rep em G0, G1 ou G2, com etiqueta na junção;
   - **simetria:** plano de espelho com nós presos ao plano, sem atravessá-lo, e fusão por distância.

   Vínculo perdido mostra estado na árvore e nunca muda a forma calado.
9. **Conferir.** Zebra e curvatura ao vivo durante o arraste (B180), etiquetas de continuidade nas junções.
10. **Virar sólido.** Botão Converter (B179): mostra desvio máximo e os nós problemáticos antes de aceitar; escolha G0, G1 ou G2
    nos nós irregulares; o resultado aceita filete, casca e booleana.
11. **Voltar e editar.** A forma continua na árvore: mudar um parâmetro ou reabrir a gaiola refaz a conversão e os filhos.

### Regras para ser intuitivo

- Nada de modo oculto: chips sempre visíveis no 3D e toda tecla com botão correspondente.
- Desfazer por gesto, não por comando.
- Mesmo manipulador de superfície, ponto e montagem.
- Índices da malha persistentes, para os vínculos não quebrarem ao inserir aresta.
- Limite claro: subdivisão não é malha de render. Faces só quadrilaterais recomendadas, com aviso nos nós de valência alta.

**GIF do comando (10 s):** criar caixa; clicar num nó de cima; puxar a seta Z com raio grande; aparece uma cúpula; vincar uma aresta; ligar simetria; converter.

## 6. O que não dá para ser 3D, e por quê

| Item | Por quê | Como apresentar |
|---|---|---|
| Tolerâncias (juntar, curar, fechar) | São da ordem de 0,01 mm: invisíveis e impossíveis de arrastar em escala de peça | Campo, com o efeito no 3D (folgas verdes) |
| Expressões e parâmetros | Texto por natureza; `L/2 + 5` não tem gesto | Campo; a alça trava com cadeado, como já faz |
| Equação do plano, coordenadas exatas | Dado numérico | Campo, com o plano desenhado |
| Número de seções, grau, densidade | Inteiro abstrato; seta passaria falsa precisão | Campo com passos e prévia |
| Tipo de varredura, modo e propagação | Escolha discreta | Figuras no painel, com prévia ao passar o mouse |
| Ordem da lista de seções | A lista é a referência; o 3D só confirma | Arraste no painel, com número no 3D |
| Publicação, vínculo e congelamento | Estado do documento | Árvore e painel |
| Valores do verificador de continuidade | Medida, não entrada | Rótulo somente leitura no 3D |
| Mensagens de erro do kernel | Podem não ter local | Painel, com realce no 3D quando houver |

Tudo que hoje é número de solução, índice de borda, lado ou ponto "perto de" **deve** ir para o 3D; não entra nesta tabela.

## 7. Itens derivados

Ver [superficies.json](superficies.json): amplia B09, B14, B153, B154 e B179 e propõe itens que dependem do contrato de manipulação e
da esteira de mídia de [interface.md](interface.md). Os títulos atuais de B153 e B154 citam marcas e devem ser renomeados na
próxima edição do `backlog.py`.
