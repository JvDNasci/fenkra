# ADR 0053: cópias compartilhadas e nível de detalhe na montagem

Data: 2026-11-06. Item: B125. Liga com B259 (trabalho fora da interface), B61 (presets de malha),
B236 (modo leve) e B305 (prévia de inserção).

## Contexto

A vista da montagem copiava a malha de cada componente para o mundo e a cena repetia todos os vértices
num buffer único. Com 100 peças iguais a cena passava do maior buffer da placa, e a saída provisória
era abrir sem faces e avisar. Composição, tesselação e montagem da cena rodavam na thread da interface:
ao abrir, a janela parava por segundos.

## Decisão

1. **Protótipo e cópia.** `lod::Prototipo` guarda a malha uma vez, no sistema da peça ou do subconjunto,
   e `lod::Instancia` guarda a posição. A placa recebe cada protótipo uma vez, direto dos vetores da
   malha, e desenha as cópias numa chamada só. Cor, acabamento e realce vêm por cópia, então passar o
   mouse não sobe malha nenhuma.
2. **Nível pelo erro na tela.** Até três níveis por protótipo: a qualidade escolhida, o rascunho do B61
   e os limites de digitação (2% e 45°). Vale o nível mais grosso cujo desvio, convertido em pixels na
   distância da cópia, fica abaixo de 1,5 px. Navegando com "Simplificar ao girar" o limiar é 4 vezes
   maior; o modo leve dobra. Os níveis grossos re-tesselam o cache do cálculo, sem refazer comandos.
3. **Carga leve.** Nada sobe antes de uma cópia pedir. O nível mais grosso sobe primeiro, para a peça
   aparecer, e os finos entram com cota de 48 MB por quadro. Passando de 1 GiB, sai o nível fino parado
   há mais tempo; o mais grosso de cada protótipo nunca sai.
4. **Composição em thread própria.** `asm_fundo` tem o próprio `PartCache`, porque a forma do
   OpenCASCADE não atravessa threads. Peças em memória e peças esquecidas vão em cada pedido. A posição
   das cópias muda no mesmo quadro, sem esperar a composição. Enquanto se arrasta, um pedido novo só
   cancela o anterior depois de 100 ms, senão nenhum terminaria.
5. **Seleção por caixa.** Cada corpo tem caixa pronta; o raio descarta o corpo antes de olhar triângulo.
6. **Versão simplificada.** `Node.simplificada` prende o componente no nível mais grosso. Só muda o
   desenho: volume, massa, seleção e medidas seguem a peça.

## Medição

`FENKRA_DEMO_ASM=grade100` em release, com `FENKRA_MEDIR_QUADROS`, numa máquina dividida com outras
compilações. O tempo é o de CPU do quadro, interface e desenho juntos.

| Grandeza | Antes | Depois |
|---|---|---|
| Pior quadro ao carregar | 3805 ms | 76 ms |
| Quadros acima de 100 ms | 4 | 0 |
| Composição | 898 ms na interface | 797 ms em thread própria |
| Montar a cena | 2051 ms | 0,05 ms |
| Subir a cena | 705 ms | 0,08 ms, mais 3,5 ms de malha por quadro |
| Malha na placa | 855 MB de faces, 10 MB de arestas | 5,5 MB, os três níveis |
| Seleção sob o cursor | 165 ms | 8 ms |
| Triângulos desenhados em repouso | todos os da malha fina | 321.600 |

Com `variado40`, 40 peças que não dividem malha: pior quadro de 71 ms, nenhum acima de 100 ms, 45 MB na
placa e 19 s de composição em fundo, com a janela respondendo. Capturas de longe e de perto ficaram
iguais às de antes a olho nu. O aceite de 100 ms do B259 fica medido para a carga; o cancelamento por
mudança de cota durante a composição tem teste, sem medição de tempo.

## Por que assim

Decimar malha foi descartado: os níveis do próprio malhador preservam as faces, então a seleção e o
realce não precisam de mapa entre malhas. Trocar `BodyResult.mesh` por uma malha preguiçosa economizaria
memória principal, mas mudaria o tipo que quase toda a interface lê, com várias bancadas no mesmo código.

## Consequências

- O corpo da vista ainda carrega a malha fina posicionada: seleção, medida e mapa de curvatura a usam.
  A placa de vídeo divide a malha; a memória principal ainda não.
- Mapa de curvatura colore por vértice e por isso volta à cena plana, que continua sujeita ao limite do
  maior buffer. Componente recortado por recurso de montagem também fica na cena plana.
- Restrições e links carregam peça com forma na thread da interface, no cache dela, ao serem usados.
  Ao abrir pelo comando Abrir montagem, os links leem os documentos ainda na interface, sem calcular.
- A composição calcula as peças uma por vez: o tempo total de carga não caiu, só deixou de travar.
- O orçamento de 1 GiB é fixo porque o wgpu não informa a memória livre da placa.
