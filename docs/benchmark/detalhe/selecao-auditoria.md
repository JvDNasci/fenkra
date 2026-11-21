# Auditoria dos campos de seleção (B561 e B562)

Levantamento de 21/11/2026, só de leitura do código (develop após a alpha.41). Base para a seleção
universal (B561), o botão de inverter (B562) e o contrato de feature (B565).

## Como está hoje

**Onde ficam os painéis.** `feat_ui.rs::panel` (Sólidos, Chapa e superfície-sólido), `editor.rs`
(Sketch, Extrusão, Revolução, Furo, Filete/Chanfro), `geo_ui.rs::panel` (bancada de Superfícies, que são
`RefKind::Surf*`, não `FeatureKind`), `st_*_ui.rs` (Estrutural), `chapa_*_ui.rs` (Chapa), `asm_ui.rs`
(Montagem).

**Widgets de seleção que existem (sete jeitos diferentes):**

| Widget | Onde | Clique no 3D | Tab / campo ativo |
|---|---|---|---|
| Slots de um elemento (`point_slot`, `curve_slot`, `axis_slot`, `plane_slot`, `surf_slot`) | Superfícies, partes dos Sólidos | sim | sim (`campos.rs`) |
| Caixa de vários `geo_ui::bag` (`curves_list`, `surfs_list`, `points_list`) | Fill, Join, Loft, Ruled, Extrapolate, Filete/Chanfro, aba (`campo_aresta`, laço) | sim | sim |
| `feat_ui::faces_ui` (lista antiga) | Casca, Ângulo de saída, Espessura, Edição direta | botão "+ seleção" | não |
| `sketch_pick` / `sketch_combo` | todo comando de perfil por sketch | só o primeiro clique | não |
| `plane_combo` | Espelhar, plano neutro, Mover corpo, Joggle, Sketch, Primitiva | não | não |
| Texto com coordenada + "usar a face" | Rosca, Converter, Joggle, Dobra, Veneziana, Rasgo em arco, cantos, placa e esquadro estruturais | botão | não |
| Seleção própria da montagem (`MateRef`) | restrições | sim | à parte |

A regra do campo ativo está espalhada em três lugares (`slots::fill`, `entradas::campo_ativo`,
`campos.rs`): widget novo tem de entrar nos três, senão fica fora do Tab.

**Cadeia tangente:** só na aba curva (`so_aresta`) e no menu "Extrair com tangência" dos campos de
curva. `App::chained` encadeia no clique só para Preencher, Junção e Junção de curvas, e só dentro do
mesmo sketch, wireframe ou corpo. Um sketch é uma entrada única: nenhum comando de perfil aceita curvas
de sketches diferentes. O único jeito de juntar é criar à mão uma Junção de curvas (`CurveJoin`).

**Inverter:** a Chapa já tem botão padrão (`chapa_unif_ui::inverter`). Nos Sólidos são caixas de marcar
com nomes diferentes. Nas Superfícies só existe a seta do 3D, sem botão no painel (no filete de
superfícies o painel até descarta o campo). Revolução, Varredura, Loft, Mover corpo, Padrão linear,
extrusão, revolução, deslocamento, varredura e loft de superfície não têm inverter.

## Chapa

| Feature | Campo | Tipo | Vários? | Widget | Cadeia? | Junção na hora? | Inverter |
|---|---|---|---|---|---|---|---|
| Parede base | Perfil | sketch ou curva | 1 | `sketch_pick` | não | precisa | botão |
| Parede base (de superfície) | Superfície | sup | 1 | `surf_slot` | – | – | botão |
| Aba de aresta | Aresta | aresta | 1 | `campo_aresta` (bag de 1) | não | – | botão + lado |
| Aba curva | Aresta (cadeia) | aresta | 1 | `campo_aresta` | só no corpo | precisa (origens diferentes) | botão |
| Laço de abas | Arestas | aresta | V | bag | laço da face | – | botão |
| Aba de perfil | Perfil aberto | sketch | 1 | `sketch_pick` | não | precisa | botão |
| Aba por superfície | Suporte; face; limites | sup; face; sup/curva | 1 | `surf_slot`; texto; slots | – | – | botão |
| Bainha | Aresta | aresta | 1 | texto | não | – | botão |
| Joggle | Linha; plano | pontos; plano | 1 | texto; `plane_combo` | – | – | botão |
| Dobra por linha | Linha ou sketch | pontos; sketch | 1 | texto; `sketch_pick` | – | – | botão + para baixo |
| Veneziana | Linha | pontos | 1 | texto | – | – | botão + para baixo |
| Estampo | Perfil/curvas/pontos; punção | sketch; corpo | 1 | `sketch_pick`; lista | não | precisa | botão |
| Furo com colar | Centros | pontos ou sketch | V | texto + contador | – | – | botão |
| Recorte | Perfil | sketch | 1 | `sketch_pick` | não | precisa | botão |
| Aba de transição (loft) | Perfis 1 e 2 | sketch | 2 | 2× `sketch_pick` | não | precisa | botão |
| Desdobrar parcial | Abas | features | V | caixas de marcar | – | – | refazer |
| Converter em chapa | Face fixa; faces abertas | faces | 1; V | texto + lista | – | – | – |
| Rasgo em arco, alívio e fechamento de canto | Centro / canto | ponto / vértice | 1 | texto | – | – | só o fechamento ("qual aba cobre") |

## Superfícies

| Feature | Campo | Tipo | Vários? | Widget | Cadeia? | Junção na hora? | Inverter |
|---|---|---|---|---|---|---|---|
| Extrair / Extrair vários | Face | face | 1 / V | clique | propagação tangente | – | – |
| Deslocar | Superfície | sup | 1 | `surf_slot` | – | – | não (sinal, dois lados) |
| Preencher | Contorno; apoio | curva; sup | V | `curves_list`; condições | `chained` (mesma origem) | precisa | – |
| Extrusão de superfície | Curva; direção; limites | curva; eixo; sup | 1 | `curve_slot`; `axis_slot`; limites | não | precisa | não |
| Revolução de superfície | Curva; eixo | curva; eixo | 1 | slots | não | precisa | não |
| Esfera / cilindro | Centro; direção | ponto; eixo | 1 | slots | – | – | início/fim |
| Varredura (simples e avançada) | Perfil; guias; espinha | curva | 1 cada | `curve_slot` | só pelo menu | precisa | – |
| Loft de superfície | Seções; guias; caminho | curva | V; V; 1 | `curves_list`; `curve_slot` | não | precisa (seção como cadeia) | – |
| Regrada | Curvas; apoio; direção | curva; sup; eixo | V; 1; 1 | `curves_list`; slots | não | precisa | só a seta |
| Concordância (blend) | Bordas; apoios | curva; sup | 1 | `curve_slot`; slot | não | precisa | – |
| Conectar | Sup 1 e 2 | sup | 1 | `surf_slot` | – | – | – |
| Junção | Itens; curvas | sup; curva | V | `surfs_list`; `curves_list` | – | é a junção | – |
| Reparar / desaparar / partes | Superfície | sup | 1 | `surf_slot` | – | – | – |
| Dividir | Sup; cortar com; curva | sup; curva | 1; V; 1 | slots + botão "+" | não | precisa | só a seta |
| Aparar | Sup 1 e 2 | sup | 1 | `surf_slot` | – | – | só a seta |
| Regiões | Sup; ferramentas; curva | sup; curva | 1; V; 1 | slots + botão "+" | – | – | marcas |
| Extrapolar | Sup; ponto; arestas; limite | sup; ponto; curva | 1; V; V; 1 | slots; listas; limite | propagação | – | – |
| Filete entre superfícies | Sup 1 e 2 | sup | 1 | `surf_slot` | – | – | seta (painel descarta) |
| Filete de aresta de superfície | Sup; arestas | sup; aresta | 1; V | slot; clique | – | – | – |
| Curva paralela | Curva; apoio; mais curvas | curva; sup | 1; V | slots; `curves_list` | – | – | só a seta |
| Junção de curvas | Curvas | curva | V | `curves_list` | `chained` | é a junção | – |
| Transformar | Fonte, direção, eixo, plano, centro | vários | 1 | slots | – | – | números |
| Planos e pontos de referência | curva, sup, ponto, plano | vários | 1 / listas | slots; `points_list` | – | – | caixa "medir do fim" |

## Sólidos

| Feature | Campo | Tipo | Vários? | Widget | Cadeia? | Junção na hora? | Inverter |
|---|---|---|---|---|---|---|---|
| Sketch | Plano | plano/face | 1 | `plane_combo` | – | – | – |
| Extrusão | Perfil; direção; término | sketch/curva; eixo; plano/sup | 1 | `sketch_combo`; `axis_slot`; término | não | precisa | caixa "inverter sentido" |
| Revolução | Perfil; eixo | sketch; eixo | 1 | `sketch_combo`; combo próprio | não | precisa | não |
| Furo | Perfil ou pontos | sketch; ponto | V | combo ou `points_list` | – | – | botão "inverter sentido" |
| Filete / chanfro | Arestas; faces | aresta; face | V; 2–3 | bag; clique | propagação | – | – |
| Varredura | Perfil; guia | sketch; curva | 1 | `sketch_pick`; `curve_slot` | só pelo menu | precisa | não |
| Loft | Perfis | sketch | V | lista própria ▲▼ | não | precisa | não |
| Primitiva | Plano | plano | 1 | `plane_combo` | – | – | – |
| Casca | Faces abertas | face | V | `faces_ui` | regra | – | caixa "para fora" |
| Ângulo de saída | Faces; plano neutro | face; plano | V; 1 | `faces_ui`; `plane_combo` | regra | – | caixa |
| Espessura / edição direta | Faces | face | V | `faces_ui` | regra | – | rádio / sinal |
| Padrão / espelhar | Eixos, pontos; fontes; plano | eixo; ponto; features; plano | 1/V | slots; caixas de marcar; `plane_combo` | – | – | não |
| Escala | Centro | ponto | 1 | `point_slot` | – | – | – |
| Nervura | Linhas | sketch | 1 | `sketch_pick` | não | precisa | caixa + lado |
| Rosca | Face cilíndrica | face | 1 | texto + botão | – | – | – |
| Combinar | Corpo ferramenta | corpo | 1 | combo + clique | – | – | – |
| Dividir corpo / costurar | Superfície | sup | 1 | `surf_slot` | – | – | só a seta |
| Espessar superfície | Superfície; direção | sup; eixo/plano | 1 | slots | – | – | só a seta |
| Fechar superfície | Superfície | sup | 1 | `surf_slot` | – | – | – |
| Preencher por fronteiras | Ferramentas; células | sup; ponto | V | `surfs_list`; lista | – | – | – |
| Mover corpo | Direção, eixo, plano | eixo; plano | 1 | slots; `plane_combo` | – | – | não (sinal) |
| Plástico (ressalto, encaixe, grelha, lábio) | Centros, raízes, região, contorno | sketch | 1 | `sketch_pick` | não | precisa | caixas soltas |

## Estrutural

| Feature | Campo | Tipo | Vários? | Widget | Cadeia? | Junção na hora? | Inverter |
|---|---|---|---|---|---|---|---|
| Membro | Linhas guia; cantos | curva; membro | V; 1 por canto | `curves_list`; combo | não (cada linha vira um membro) | precisa (B575) | caixa "espelhar seção" |
| Placa de ponta | Ponta | face | 1 | texto + botão | – | – | rádio externo/interno |
| Esquadro | Encontro | aresta | 1 | texto | – | – | caixa "inverter D1 e D2" |
| Solda | Faces 1 e 2 | face | V | lista própria | tangentes | – | – |

## Montagem

Restrições (coincidente, contato, distância, ângulo, paralela, perpendicular, engrenagem, cremalheira)
e furo na montagem usam a seleção própria do `asm_ui` (`MateRef`), um elemento por lado, com botões
"Inverter alinhamento", "Inverter lado" e "Inverter sentido".

## Achados

**B561**
1. As entradas por sketch são o gargalo: Extrusão, Revolução, Varredura, Loft, Nervura, Plástico, Parede
   base, Recorte, Estampo, Aba de perfil, Aba de transição, Dobra (modo sketch) e Furo com colar (modo
   sketch). Cada uma precisa de entrada do tipo curva com caixa de vários, cadeia tangente de origens
   diferentes e junção na hora.
2. `faces_ui` e os campos de texto com coordenada viram caixa universal e entram no Tab.
3. `plane_combo` vira `plane_slot` (clique no 3D); a Revolução passa a usar `axis_slot`.
4. Faltam caixas de curva em: extrusão, revolução, varredura e concordância de superfície, guia das
   varreduras sólidas, projeção de curva e plano de curva.
5. Unificar a regra de campo ativo hoje espalhada em `slots::fill`, `entradas::campo_ativo` e
   `campos.rs`.

**B562**
1. Chapa já padronizada.
2. Sólidos: trocar as caixas de marcar (Extrusão, Furo, Ângulo de saída, Nervura, Plástico, Casca,
   Espessura) pelo mesmo botão "Inverter direção".
3. Acrescentar inverter em Revolução, Varredura, Loft, Mover corpo, Padrão linear e nas superfícies
   (extrusão, revolução, deslocamento, varredura, loft).
4. Superfícies: botão no painel ligado ao mesmo campo da seta (dividir, aparar, filete, regrada, curva
   paralela, dividir corpo, costurar, espessar).
5. Montagem: mesmo texto e ícone dos outros.

## Proposta de ondas

| Onda | Escopo | Tokens |
|---|---|---|
| 0 | Base comum: caixa e slots universais com cadeia tangente de origens diferentes e junção na hora; regra de campo ativo unificada; botão de inverter padrão | 400 mil |
| 1 | Chapa: comandos de perfil para campo universal com junção; bainha, conversão, furo com colar, joggle e veneziana para caixa; aba curva com cadeia de origens diferentes | 450 mil |
| 2 | Superfícies, curvas de entrada (extrusão, revolução, varreduras, loft, concordância, regrada, dividir) | 400 mil |
| 3 | Superfícies, inverter no painel e nos que faltam | 150 mil |
| 4 | Superfícies, campos de face e restantes (regiões, extrapolar, filete de aresta, extrair vários, preencher) | 150 mil |
| 5 | Sólidos de perfil (extrusão, revolução, varredura, loft, nervura, plástico, furo) com inverter padrão | 400 mil |
| 6 | Sólidos de faces (casca, ângulo de saída, espessura, edição direta, rosca); planos e eixo por clique | 150 mil |
| 7 | Estrutural (solda, placa, esquadro, cadeia na guia do membro) | 150 mil |
| 8 | Montagem (seleção das restrições pelo widget universal, botões de inverter) | 50 a 150 mil |

Total: cerca de 1,9 a 2,0 milhões de tokens. A onda 0 vem antes das outras.

**Para decidir:** (1) a junção na hora cria uma Junção de curvas escondida na árvore ou guarda a lista de
curvas no próprio comando? (2) o B561 cobre Estrutural e Montagem ou só Chapa, Superfícies e Sólidos?
