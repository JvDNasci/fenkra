# ADR 0092: origem e eixos H/V do sketch como entidades fixas, e posição do sketch por referência (B437)

## Contexto

O sketch só tinha a origem quando alguém clicava nela (um ponto fixo criado na hora) e não tinha eixos que uma
restrição pudesse usar. A posição do sketch sobre o plano era um deslocamento numérico com trocar e inverter
(B97). O usuário pediu: origem e eixos H e V sempre selecionáveis e aceitos em qualquer restrição, a origem
automática ou escolhida no modelo (origem da peça, ponto projetado, interseção de duas linhas, ponto médio,
centro da face), H seguindo um eixo global, uma linha, um ponto ou um plano, e arquivos antigos abrindo como
antes.

## Decisão

1. **Origem e eixos são entidades comuns do sketch, fixas.** `SketchData.eixos` guarda o ponto da origem e duas
   linhas de construção com pontas fixas, de `-ALCANCE` a `+ALCANCE` (10 m). O solver já trata a linha como reta
   infinita em ponto-na-curva, distância ponto-linha, paralela, perpendicular e simetria, então toda restrição
   aceita os eixos sem caso especial no solver. O comprimento só serve para o clique e o snap acharem o eixo em
   qualquer lugar da tela; as pontas ficam escondidas e não se clicam.
2. **A interface protege o que é da origem.** Apagar, a seleção por janela e a cota de comprimento ignoram a
   origem e os eixos; `remove_curve` recusa os eixos. Horizontal e vertical com um eixo selecionado valem para o
   resto da seleção; ponto com eixo alinha o ponto à origem; igual com eixo é recusado.
3. **Arquivo antigo ganha os eixos ao abrir no editor** (`garantir_eixos`), reaproveitando o ponto fixo em
   (0, 0) que a versão antiga criava. São só construção e pontos fixos: a geometria não muda. Fora do editor o
   sketch antigo calcula sem eixos.
4. **A posição tem modo de origem e de orientação** (`SketchPos.modo`, `SketchPos.orienta`), com padrão
   automático. As referências reaproveitam `LinkSrc`, o mesmo tipo da geometria externa, e são reencontradas a
   cada cálculo por `link_world`. A origem escolhida é projetada no plano-base; a direção de H é projetada no
   plano e V fica a 90°. O deslocamento anda nos eixos orientados, antes de trocar e inverter; com tudo
   automático isso é exatamente a regra antiga (`f.point(origem)`), então arquivos antigos dão a mesma peça.
5. **Referência perdida não derruba a peça.** A parte que falhou volta a ser automática e o sketch fica com
   aviso dizendo qual foi. Direção perpendicular ao plano também cai na automática, com aviso.

## Consequências

- O cálculo da posição vive em `fenkra_solidos::sketch_posicao` (filho de `lib.rs`, usa os resolvedores do
  contexto) e a conta pura em `profile::posiciona_em`.
- Os eixos entram em `EvalSketch.lines` (servem de eixo de revolução) mas não em `viz` nem em `curves`: não se
  desenham fora do editor e outro sketch não os projeta.
- O conversor de árvore ainda não exporta os modos novos; uma árvore exportada volta como posição automática.
