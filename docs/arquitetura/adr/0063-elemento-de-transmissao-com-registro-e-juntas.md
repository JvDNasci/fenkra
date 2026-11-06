# ADR 0063: elemento de transmissão com registro editável, eixos fixos e juntas

Data: 2026-11-06. Estado: aceita. Itens: B340, B341, B342, B343, B344, B345.

## Contexto
O gerador de engrenagens só inseria o par já engrenado por restrições (`Distance` e `Gear`) e esquecia os
parâmetros: não dava para editar depois, não havia cônicas, sem-fim, corrente nem correia, e o cálculo de
velocidade, torque e perdas não existia. O pedido era um comando único que gera, edita, restringe, anima e
calcula, com adaptação de projeto.

## Decisão
1. **Registro na montagem.** `Assembly.transmissoes` guarda um `Elemento` (os parâmetros do que foi gerado),
   o rpm e o torque de entrada e os ids criados. `editar_transmissao` apaga o que o registro criou e gera de
   novo, mantendo a posição do primeiro eixo e o id do registro; se a edição falha, a montagem volta ao que era.
2. **Eixo fixo por peça girante, junta de revolução e acoplamento.** Cada peça ganha um pino fixo (o furo dela),
   uma junta `Revolute` entre os dois e um acoplamento `Gear` entre as juntas, com a razão e o sinal do giro.
   O comando de giro `rpm · 6 · t` vai na junta motora, então a simulação da montagem já anda com o rpm do
   cálculo. O caminho antigo por restrições (`inserir_par_engrenagens`) continua para o par com cremalheira.
3. **Convenção de eixos.** O Z local de cada peça é o eixo; nas cônicas e no diferencial o Z local aponta para o
   vértice do cone, então o acoplamento de qualquer par cônico é `−z1/z2`. No sem-fim, o eixo do sem-fim vai ao
   longo de +Y do mundo e o da coroa ao longo de +Z; com o filete de mão direita a razão fica `−entradas/z2`.
4. **Cônica por Tredgold.** Perfil da engrenagem virtual (`z / cos δ` dentes) no plano da extremidade maior e
   `loft` reto até a mesma seção reduzida em direção ao vértice. É aproximação: o resíduo de ajuste do flanco
   aparece como 0 a 3 mm³ de interferência nos pares com ângulos diferentes de 45° (medido), e uma fase errada
   por 0,03 rad já passa de 7 mm³, o que é o critério dos testes.
5. **Sem-fim por `loft` reto** de seções giradas (até 15° entre seções, no máximo 160 seções). A coroa é a
   helicoidal do gerador comum, sem garganta.
6. **Espinha de peixe** é a helicoidal com o giro da seção `min(z, b − z) tan β / r`, e a folga de impressão
   afina o dente e aprofunda o pé.
7. **Dados de corrente e correia só com duas fontes**, com nível de conferência por linha; a geração recusa as
   linhas pendentes. Perfil da roda de corrente simplificado; correia lisa e polia dentada sem perfil de dente,
   porque a altura do dente não tem duas fontes.
8. **Cálculo preliminar** (`transmissao_calc.rs`): razão, voltas, velocidade, torque, potência, rendimento e perda
   por estágio e em cadeia; rendimentos típicos editáveis, e o do sem-fim pela fórmula clássica.
9. **Usinagens** (`usinagens.rs`) são comandos nomeados e paramétricos sobre a peça padronizada; o vínculo com o
   catálogo fica em `props.custom` (`catalogo` e `usinagens`), e a chave do arquivo ganha um sufixo por usinagem.

## Consequências
- Formato novo opcional no `.fka` (`transmissoes`, omitido quando vazio): arquivos antigos abrem como antes.
- O diferencial anima com a caixa presa; a caixa solta (`ω caixa = (ω1 + ω2)/2`) está só no cálculo, porque o
  acoplamento entre juntas é de duas em duas.
- A corrente desenhada é um corpo estático (rolos e placas); o movimento dos elos ao longo do laço está na
  geometria (`Caminho::elos` com fase), mas ainda não é desenhado na viewport.
- Sem-fim com muitas voltas é caro: o limite de 160 seções recusa o resto com mensagem.
