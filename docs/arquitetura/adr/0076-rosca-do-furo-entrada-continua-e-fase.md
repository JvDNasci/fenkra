# ADR 0076: rosca do furo com entrada contínua, ângulo de início e medidas de entrada e fim (B373)

## Contexto

Testando uma rosca modelada num furo M16x2 (broca de 14 mm), apareceram estes defeitos:

1. **Faces soltas (medido).** A ferramenta era um conjunto solto de trechos de duas voltas, que se
   encostavam numa tampa plana em comum. Depois do corte, cada emenda deixava duas membranas planas
   coincidentes, sem espessura (0,58 mm² cada, no plano que contém o eixo), dentro do sulco: um furo cego de
   20 mm tinha 12 faces planas pequenas a mais (medido por `topology()` antes de fundir os trechos), e o STL
   exportado nesse estado saiu com arestas sem par.
2. **Degrau na entrada (deduzido da geometria e coberto por teste).** A hélice nascia exatamente na boca, com a
   ponta do perfil cortada em plano, então o arco que o sulco abre na face de cima só existia de um lado do
   ponto de início (meia volta de arco), o que aparece como degrau na primeira volta.
3. **Início "dentro do furo" (hipótese não observada).** A escada de tentativas de B351 deslocava o início
   da hélice em frações do passo (0,23, 0,41 e 0,67 passo) quando a primeira tentativa falhava; isso empurra
   o início para dentro. Não consegui fazer a primeira tentativa falhar num M16x2 simples, então não vi o
   defeito acontecer; o deslocamento foi retirado de qualquer modo.
4. **Falha com ponta, rosca e folga (não reproduzida).** Numa matriz de 108 combinações de profundidade
   (cego curto, cego, passante), profundidade da rosca, folga, deslocamento da ponta e diâmetro, o código
   novo cortou todas (o único erro é o legítimo "a profundidade da rosca passa da profundidade do furo").
   O que a medição mostrou de frágil: na primeira versão da hélice vinda de fora, 3 de 12 fases (0, 300 e 330
   graus) do M16x2 passante saíram com sólido inválido e a booleana as aceitava como cortadas. Por isso a
   aceitação do corte agora exige sólido válido e volume tirado coerente.

## Decisão

1. **A hélice começa um passo acima da boca** (`rosca_furo::ferramenta_do_furo`) sempre que a rosca
   começa na boca, e atravessa a face de cima como o sulco de um macho. O fim continua uma meia largura
   de perfil antes do fundo (cego) ou fora da peça (passante), como no B351. Havendo rebaixo, escareado
   ou folga em cima, o sulco começa na profundidade pedida, como antes.
2. **Os trechos são fundidos num sólido só** antes do corte (`Bool::Fuse`); se a fusão falhar, o conjunto solto
   ainda corta. Isso elimina as membranas das emendas.
3. **A fase da hélice é uma escolha do usuário.** `Fase { referencia, angulo, esquerda }`: a hélice é
   construída com a direção X do cilindro apontada para a referência girada pelo ângulo (regra da mão
   direita em torno do sentido de entrada do furo). Isso exigiu uma entrada nova no kernel
   (`thread_tool_x`, `thread_tool_ext_x`), que recebe a direção de início; as antigas chamam as novas com
   direção nula (padrão do OpenCASCADE). Os trechos de duas voltas recebem a direção girada por
   `360 * (distância já feita) / passo`, então cada emenda cai no ângulo exato. A rosca à esquerda é
   construída à direita no ângulo oposto e espelhada pelo plano que contém o eixo e a referência.
4. **A escada de tentativas não muda mais a fase nem o início**: os degraus variam só o tamanho dos
   trechos (`ESCADA`, sete degraus). Cada tentativa é aceita só se o volume tirado passa de 35 % do
   estimado pelo V da rosca (`volume_estimado`) e o corpo continua válido (`is_valid`).
5. **Medidas de entrada e fim** em `HoleExtra`: chanfro de entrada (cone de 45 graus na boca, feito antes
   do sulco), folga em cima, e a folga embaixo, cada uma em mm, passos da rosca ou porcentagem
   (`UnidadeMedida`). Porcentagem do chanfro = profundidade do filete; das folgas = profundidade útil do
   furo.

## Formato do arquivo

`HoleExtra` ganha campos, todos com `#[serde(default)]`: `thread_relief_unit` (`null` = passos, que era a
única unidade), `thread_top_gap` e `thread_top_gap_unit`, `thread_chamfer` e `thread_chamfer_unit`,
`thread_start_angle` e `thread_start_ref` (um `AxisRef`). Arquivos anteriores abrem sem mudança e
recalculam com o mesmo sulco, agora com a entrada contínua. Arquivos novos abertos numa versão anterior
perdem só os campos novos.

## Consequências

- O comando Rosca (sobre uma face cilíndrica) continua com a hélice começando na face, com a fase padrão:
  a face pode ser vizinha de outra e a hélice não pode sair dela.
- A prévia leve (B351, `Cache::leve`) não muda: só linhas, o sulco real ao concluir. O chanfro de entrada é
  aplicado também na prévia, porque custa um cone.
- O número de faces ainda cresce com o número de trechos (cada trecho tem os próprios flancos e cada volta
  da parede entre sulcos é uma face): um furo cego de 20 mm tem 31 faces, das quais 8 são planas (as seis do
  bloco, o fundo do furo e a tampa onde o sulco termina).
- A caixa envolvente do kernel (`bbox`) de uma peça roscada passa da peça em alguns milímetros, porque conta
  os polos das superfícies do sulco; a caixa da malha e a exportação estão certas.
