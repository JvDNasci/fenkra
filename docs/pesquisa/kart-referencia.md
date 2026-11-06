# Kart de referência para o projeto-teste (B353)

Pesquisa de apoio para modelar um kart de ponta a ponta no Fenkra. Escrita com palavras próprias. O
projeto é nosso: nenhum desenho de terceiros foi nem será copiado. Dimensões gerais e limites de
regulamento entram como fatos, com a fonte citada.

Legenda:

- **[V]**: confirmado em fonte efetivamente lida, com o texto aberto.
- **[C]**: estimado, derivado por nós ou visto só em trecho de busca, sem abrir a página. Precisa de
  conferência se for virar regra. Na modelagem, todo [C] é parâmetro editável.

As referências `[F#]` apontam para a seção 5.

## Limites desta pesquisa

- **Não há plano livre utilizável.** Nenhum desenho de kart de tubo de aço com licença aberta clara
  foi encontrado. Por isso a geometria do quadro é projeto nosso, limitada pelos regulamentos.
- O comprimento total na regra internacional depende de um desenho técnico (DT 2.1a) que não vem no
  texto do PDF. Esse valor só foi confirmado no regulamento britânico [F3].
- Os desenhos técnicos da regra internacional não foram lidos nem reproduzidos. Por isso, de onde se
  medem os balanços dianteiro e traseiro é suposição nossa: do eixo da roda correspondente.
- Vários artigos abertos bloquearam a leitura (erro 403). Deles só há trecho de busca, e os valores
  aparecem como [C] ou ficam fora da especificação.
- O regulamento britânico [F3] foi lido por extração automática, que trocou um rótulo: o entre-eixos
  aparece sob o nome "Gearbox". Os valores batem com a regra internacional.
- A norma ABNT NBR 8476, exigida pela CBA para tubos de chassi de categorias de base [F2], não teve
  título nem escopo confirmados em fonte confiável.

## 1. Especificação do kart Fenkra

**Conceito:** kart de pista para adulto, sem suspensão, motor de 125 cm³ a dois tempos representado
por volume simplificado, transmissão direta por corrente, sem embreagem nem câmbio. Segue as
dimensões do Grupo 2 da regra internacional. É um produto de teste, não um chassi homologável.

### 1.1 Medidas gerais

| Item | Valor adotado | Limite ou base | Fonte | Estado |
|---|---|---|---|---|
| Entre-eixos | 1040 mm | 1010 a 1070 mm | [F1] art. 9.1.1 | Faixa [V]; escolha no meio da faixa [C] |
| Bitola mínima | 693,3 mm | Pelo menos 2/3 do entre-eixos | [F1] art. 9.1.1, [F3] art. 3.2 | [V], conta nossa |
| Largura total traseira | 1380 mm | Máximo 1400 mm; a proteção traseira tem mínimo de 1340 mm e fica alinhada com o lado de fora das rodas | [F1] art. 9.1.1 e 9.5.5.1; [F2] art. 54 | Limites [V]; valor [C] |
| Bitola traseira, centro a centro | 1170 mm | 1380 menos a largura da roda (210 mm) | Derivado | [C] |
| Bitola dianteira, centro a centro | 1040 mm | Acima de 693,3 mm; bico com largura mínima de 1000 mm | [F1] art. 9.5.2 | Limites [V]; valor [C] |
| Largura total dianteira | 1170 mm | 1040 mais a largura da roda (130 mm) | Derivado | [C] |
| Comprimento total | 2060 mm | 2100 mm com carroceria; 1820 mm sem carenagem | [F3] art. 3.3 | Limite [V]; valor [C] |
| Balanço dianteiro | 640 mm | Bico: máximo 680 mm; para-choque: mínimo 350 mm | [F1] art. 9.4.1 e 9.5.2 | Limites [V]; valor [C] |
| Balanço traseiro | 380 mm | Máximo 400 mm | [F1] art. 9.5.5.1 | Limite [V]; valor [C] |
| Altura sem o banco | 620 mm | Máximo 650 mm do solo | [F1] art. 9.1.1, [F3] art. 3.5 | Limite [V]; valor [C] |
| Vão livre do quadro | 30 mm | Só os pneus podem tocar o solo com o piloto a bordo | [F1] art. 4.13 | Regra [V]; valor [C] |
| Massa mínima com piloto | 150 kg | Classe de 125 cm³ de transmissão direta; 155 kg na variante nacional | [F1] art. 9.9; [F2] art. 54 | [V] |

### 1.2 Quadro

| Item | Valor adotado | Limite ou base | Fonte | Estado |
|---|---|---|---|---|
| Construção | Tubos redondos de aço soldados em peça única, sem suspensão | Obrigatório | [F1] art. 4.1, [F3] | [V] |
| Tubo principal | Ø30 × 2,4 mm | Seção Ø30,26 × 2,40 medida em chassi homologado; Ø28 × 2,0 obrigatório na classe menor; Ø30 e Ø32 citados por usuários | [F4] seção 3.3; [F1] art. 10.1.2; [F8] | Dados [V]; escolha [C] |
| Material | Aço AISI 4130 | Aço magnético; elemento de liga com até 5% em massa; ISO 4948 e 4949 | [F1] art. 4.1.2; [F4] | Regra [V]; o próprio TCC trata o 4130 como hipótese, logo [C] |
| Largura entre pinos-mestres | 640 mm | Sem limite em regra | Nosso | [C] |
| Cáster | 12° | Cerca de 10° a 15°, só em trecho de busca | Fora da especificação citável | [C] |
| Rigidez à torção, para conferir | Da ordem de 317 N·m/grau | Resultado numérico de um chassi; serve de ordem de grandeza | [F4] | [V] |

Os 31 pontos do quadro publicados no TCC [F4] **não serão usados**: são o levantamento de um chassi
comercial, e usá-los seria copiar aquele produto.

### 1.3 Eixo traseiro e freio

| Item | Valor adotado | Limite ou base | Fonte | Estado |
|---|---|---|---|---|
| Eixo traseiro | Ø50 × 2,5 mm | Máximo Ø50; parede mínima de 1,9 mm nesse diâmetro; aço magnético | [F1] art. 4.3 e 9.2; [F2] art. 54 | Limites [V]; parede [C] |
| Comprimento do eixo | 1040 mm | Não pode sair do perímetro das rodas. O único valor lido é 960 ± 2 mm, para Ø35 de categoria infantil | [F2] art. 51 e cap. IX | Regra [V]; valor [C] |
| Rasgos de chaveta | 4: dois cubos, disco e coroa | Limite escrito para a classe com câmbio; usado aqui como boa referência | [F1] art. 4.3 | [V] |
| Rolamento do eixo | 50 × 80 mm, 3 mancais | Existe 50 × 90 mm [F7]; 50 × 80 só em trecho de busca. Largura sem confirmação | [F7] | [C] |
| Disco de freio | Ø200 × 6 mm, aço | Aço, inox ou ferro fundido. Ø180 e Ø206 só em trecho de busca | [F1] art. 4.12.3 | Material [V]; medidas [C] |
| Comando do freio | Hidráulico, um pedal, atua nas rodas traseiras, ligação pedal-cilindro dupla | Obrigatório | [F1] art. 4.12.2; [F2] cap. IX | [V] |

### 1.4 Transmissão

| Item | Valor adotado | Limite ou base | Fonte | Estado |
|---|---|---|---|---|
| Corrente | Passo 7,774 mm (219) | Obrigatória nas categorias nacionais, exceto as de passo 428 e a com câmbio | [F2] cap. IX, inciso VIII; [F5] | [V] |
| Pinhão | 11 dentes | 11 ou 12 dentes, original do motor | [F2] art. 54 | [V] |
| Coroa | 82 dentes | Definida por prova, sem valor fixo em regra | [F2] art. 54 | [C] |
| Relação | 7,45:1 | Exemplos de 9:1 em [F5]; faixa de 6,5 a 8,5:1 só em trecho de busca | [F5] | [C] |
| Corrente, elos | 106 | Vendida de 94 a 116 elos, sempre em número par | [F5] | Faixa [V]; valor [C] |
| Distância entre centros | 213 mm | Sai de 106 elos pela fórmula do Fenkra (B342) | Derivado | [C] |
| Diâmetros primitivos | Pinhão 27,6 mm; coroa 203,0 mm | Fórmula `p / sen(180°/N)` | Derivado | [C] |
| Protetor de corrente | Cobre o topo e os dois lados, até o plano inferior do eixo | Obrigatório | [F1] art. 5.9 | [V] |

A largura interna e o rolo da corrente 219 **não foram confirmados**: há uma única fonte para a
largura (cerca de 4,77 mm, em trecho de busca), e o valor de rolo encontrado repete o do passo, o que
parece engano do vendedor.

### 1.5 Rodas

| Item | Dianteira | Traseira | Fonte | Estado |
|---|---|---|---|---|
| Limite de diâmetro externo | 280 mm | 300 mm | [F1] art. 4.13.1; [F2] art. 54 | [V] |
| Limite de largura | 135 mm | 215 mm | [F1] art. 4.13.1; [F2] art. 54 | [V] |
| Pneu | 10 × 4.50-5 | 11 × 7.10-5 | [F6] | [V] |
| Diâmetro adotado | 260 mm | 276 mm | 276 só em trecho de busca | [C] |
| Largura adotada | 130 mm | 210 mm | 210 só em trecho de busca | [C] |
| Aro | 5 polegadas | 5 polegadas | [F1] art. 4.14 | [V] |
| Assento do pneu no aro | Ø126,2 (+0/−1) mm | Igual | [F1] art. 4.14 | [V] |
| Diâmetro externo do aro | Mínimo 136,2 mm | Igual | [F1] art. 4.14 | [V] |
| Fixação da roda | Parafuso e porca autotravante M8 | Igual | [F1] art. 4.13 | [V] |
| Retenção do talão | Pelo menos 3 pinos no lado de fora do aro | Igual | [F1] art. 4.14.1 | [V] |

### 1.6 Direção

| Item | Valor adotado | Limite ou base | Fonte | Estado |
|---|---|---|---|---|
| Coluna | Ø20 × 2 mm, aço magnético | Mínimo Ø18 × 1,8 mm | [F1] art. 4.5.2 | Limite [V]; valor [C] |
| Fixação do volante | M6, classe 8.8 ou melhor, porca autotravante | Mínimo | [F1] art. 4.5.1 | [V] |
| Volante | Aro fechado, Ø320 mm | Aro contínuo; diâmetro sem fonte | [F1] art. 4.5.1 | Regra [V]; valor [C] |
| Braços | Aço ou alumínio, com terminal rotular nas duas pontas | Permitido | [F1] art. 4.5.3 | [V] |
| Manga de eixo | Ponta Ø25 mm, pino-mestre Ø10 mm | Só em trecho de busca | Fora da especificação citável | [C] |

### 1.7 Proteções, carroceria e posto do piloto

| Item | Valor adotado | Limite | Fonte | Estado |
|---|---|---|---|---|
| Para-choque dianteiro superior | Ø16 mm, trecho reto de 385 mm, topo a 225 mm do solo | Mínimo Ø16; reto de 375 a 395; topo de 200 a 250; apoios a 550 mm | [F1] art. 9.4.1 | Limites [V]; valores [C] |
| Para-choque dianteiro inferior | Ø20 mm, trecho reto de 305 mm, topo a 90 mm do solo | Mínimo Ø20; reto de 295 a 315; topo de 70 a 110; apoios a 450 mm; encaixe de 50 mm | [F1] art. 9.4.1 | Limites [V]; valores [C] |
| Barra lateral inferior | Ø20 mm, 500 mm do eixo central | 480 a 520 mm; reto mínimo de 400 mm | [F1] art. 9.4.2 | Limites [V]; valor [C] |
| Barra lateral superior | Ø20 mm, 560 mm do eixo central, topo a 170 mm | 480 a 600 mm; reto mínimo de 300 mm; topo a pelo menos 160 mm | [F1] art. 9.4.2 | Limites [V]; valores [C] |
| Apoios laterais | 500 mm entre si | 500 ± 5 mm; encaixe de 50 mm | [F1] art. 9.4.2 | [V] |
| Bico | Largura de 1050 mm | Mínimo 1000 mm; não mais alto que as rodas dianteiras; vão até as rodas de no máximo 180 mm | [F1] art. 9.5.2 | Limites [V]; valor [C] |
| Painel frontal | Largura de 280 mm | 250 a 300 mm; 50 mm de folga do volante | [F1] art. 9.5.3 | Limites [V]; valor [C] |
| Pontões laterais | Vão do solo de 40 mm | 25 a 60 mm; vão até a roda dianteira de no máximo 150 mm, até a traseira de no máximo 60 mm | [F1] art. 9.5.4 | Limites [V]; valor [C] |
| Proteção traseira | Largura de 1380 mm, vão de 30 mm até as rodas | Mínimo 1340 mm; vão de 15 a 50 mm; vão do solo de 25 a 60 mm | [F1] art. 9.5.5.1 | Limites [V]; valores [C] |
| Raio mínimo da carroceria | 5 mm | Mínimo, exceto nas coberturas de roda | [F1] art. 4.10.2 | [V] |
| Assoalho | Chapa de 1,5 mm, furos de Ø8 | Metálico, mínimo 1,4 mm; furos de até Ø10, afastados de 4 diâmetros; até dois furos de Ø35 | [F1] art. 4.6; [F3] | Limites [V]; valores [C] |
| Chapa de reforço do banco | 2 mm, Ø50 | Mínimo 1,5 mm, 13 cm² e Ø40 | [F1] art. 4.8.1 | Limites [V]; valores [C] |
| Tanque | 8 litros, entre os tubos principais, à frente do banco e atrás do eixo dianteiro | Mínimo 8 litros; posição obrigatória | [F1] art. 4.7 e 9.3 | [V] |
| Fixação do motor | Sobre os tubos, um suporte e duas presilhas inferiores, M8 | Mínimo M8 | [F1] art. 5.1.3 | [V] |

## 2. Árvore de produto

São 70 peças únicas, em 9 subconjuntos. Direita e esquerda iguais, por espelho ou repetição, contam
uma vez, exceto as longarinas. Medidas e materiais vêm da seção 1.

### Chassi (14)

| Código | Peça | Bancada |
|---|---|---|
| CH-01 | Longarina esquerda, tubo dobrado | Estrutural |
| CH-02 | Longarina direita, tubo dobrado | Estrutural |
| CH-03 | Travessa dianteira | Estrutural |
| CH-04 | Montante central | Estrutural |
| CH-05 | Travessa traseira | Estrutural |
| CH-06 | Arco dianteiro dos pedais, tubo dobrado | Estrutural |
| CH-07 | Tubo de apoio do banco (×4) | Estrutural |
| CH-08 | Suporte do mancal do eixo (×3) | Chapa |
| CH-09 | Suporte em C do pino-mestre (×2) | Sólidos |
| CH-10 | Soquete de para-choque (×8) | Estrutural |
| CH-11 | Apoio do motor nos tubos | Chapa |
| CH-12 | Suporte da coluna de direção | Chapa |
| CH-13 | Orelha de pedal (×2) | Chapa |
| CH-14 | Suporte da pinça de freio | Chapa |

### Eixo traseiro e freio (12)

| Código | Peça | Bancada |
|---|---|---|
| EX-01 | Eixo traseiro com rasgos de chaveta | Sólidos |
| EX-02 | Caixa do mancal (×3) | Sólidos |
| EX-03 | Rolamento do eixo (×3) | Biblioteca |
| EX-04 | Chaveta (×4) | Biblioteca |
| EX-05 | Cubo traseiro (×2) | Sólidos |
| EX-06 | Porta-coroa | Sólidos |
| EX-07 | Porta-disco | Sólidos |
| EX-08 | Disco de freio com furos em padrão | Sólidos |
| EX-09 | Corpo da pinça | Sólidos |
| EX-10 | Pastilha (×2) | Sólidos |
| EX-11 | Cilindro-mestre | Sólidos |
| EX-12 | Linha de freio, varredura em curva 3D | Superfícies |

### Transmissão (4)

| Código | Peça | Bancada |
|---|---|---|
| TR-01 | Coroa de 82 dentes | Transmissão |
| TR-02 | Pinhão de 11 dentes | Transmissão |
| TR-03 | Corrente de 106 elos | Transmissão |
| TR-04 | Protetor de corrente | Chapa |

### Direção (11)

| Código | Peça | Bancada |
|---|---|---|
| DI-01 | Volante, aro com empunhadura | Forma livre |
| DI-02 | Cubo do volante | Sólidos |
| DI-03 | Coluna de direção | Estrutural |
| DI-04 | Mancal inferior da coluna | Sólidos |
| DI-05 | Placa dos braços na coluna | Chapa |
| DI-06 | Tirante (×2) | Estrutural |
| DI-07 | Terminal rotular (×4) | Biblioteca |
| DI-08 | Manga de eixo (espelhada) | Sólidos |
| DI-09 | Pino-mestre (×2) | Sólidos |
| DI-10 | Cubo dianteiro (×2) | Sólidos |
| DI-11 | Rolamento dianteiro (×4) | Biblioteca |

### Rodas (4)

| Código | Peça | Bancada |
|---|---|---|
| RD-01 | Aro dianteiro (×2) | Sólidos |
| RD-02 | Aro traseiro (×2) | Sólidos |
| RD-03 | Pneu dianteiro (×2) | Superfícies |
| RD-04 | Pneu traseiro (×2) | Superfícies |

### Posto do piloto (8)

| Código | Peça | Bancada |
|---|---|---|
| PO-01 | Banco | Forma livre |
| PO-02 | Chapa de reforço do banco (×4) | Chapa |
| PO-03 | Escora do banco (×2) | Estrutural |
| PO-04 | Assoalho furado | Chapa |
| PO-05 | Pedal de freio | Chapa |
| PO-06 | Pedal de acelerador | Chapa |
| PO-07 | Vareta do pedal (×2) | Estrutural |
| PO-08 | Tanque, casca | Superfícies |

### Proteções e carroceria (10)

| Código | Peça | Bancada |
|---|---|---|
| PR-01 | Para-choque dianteiro superior, tubo dobrado | Estrutural |
| PR-02 | Para-choque dianteiro inferior, tubo dobrado | Estrutural |
| PR-03 | Suporte de ligação dos para-choques | Estrutural |
| PR-04 | Barra lateral inferior (×2) | Estrutural |
| PR-05 | Barra lateral superior (×2) | Estrutural |
| PR-06 | Bico | Superfícies |
| PR-07 | Painel frontal | Superfícies |
| PR-08 | Pontão lateral (espelhado) | Forma livre |
| PR-09 | Proteção traseira | Forma livre |
| PR-10 | Suporte da proteção traseira (×2) | Estrutural |

### Motor (3)

Volume simplificado. Nenhum motor comercial é reproduzido.

| Código | Peça | Bancada |
|---|---|---|
| MO-01 | Volume do motor, com ponta de eixo | Sólidos |
| MO-02 | Suporte do motor | Sólidos |
| MO-03 | Escape, varredura com seção variável | Superfícies |

### Fixadores (4)

| Código | Peça | Bancada |
|---|---|---|
| FX-01 | Parafuso M8 | Biblioteca |
| FX-02 | Porca autotravante M8 | Biblioteca |
| FX-03 | Parafuso M6 | Biblioteca |
| FX-04 | Porca autotravante M6 | Biblioteca |

### Bancadas que atuam sobre o conjunto

| Bancada | Entrega no kart |
|---|---|
| Montagem | Conjunto de topo e subconjuntos; junta de rotação nas rodas e nos pinos-mestres; relação pinhão-coroa; direção animada do volante às mangas; verificação de interferência no giro da direção |
| Desenho | Folha de conjunto com lista de peças; lista de corte do quadro; detalhes do eixo, da manga e do disco; planificação do assoalho e dos suportes |
| PMI | Tolerâncias e referências em EX-01, DI-08, EX-02 e nos furos de CH-09; exportação em STEP com PMI semântico |
| PLM | Número de peça nos 70 itens; estados e revisões; uma mudança de entre-eixos como revisão de teste, propagada a todo o conjunto |

## 3. Ordem de modelagem

Cada etapa é feita por comandos reproduzíveis e termina em teste de status, volume e caixa envolvente.
Cada falha vira item do backlog.

1. **Parâmetros mestres.** Tabela com todos os valores da seção 1, incluindo os limites do regulamento,
   para que qualquer violação apareça como verificação.
2. **Esqueleto.** Sketch 3D do quadro, com planos do solo, dos eixos e do centro, e as posições de roda,
   pino-mestre, banco e motor. Todas as peças dependem dele.
3. **Quadro.** Tubos dobrados CH-01 a CH-07 sobre o esqueleto, cortes entre membros e lista de corte.
4. **Suportes de chapa e soquetes.** CH-08 a CH-14, com planificação e cordões de solda.
5. **Eixo e rodas.** EX-01 a EX-08, RD-01 a RD-04, com rolamentos e chavetas da Biblioteca.
6. **Direção.** DI-01 a DI-11. Primeira montagem com movimento: virar o volante e conferir a folga.
7. **Transmissão.** MO-01, MO-02, TR-01 a TR-04. Conferir a relação e os 106 elos no painel de cálculo.
8. **Freio e comandos.** EX-09 a EX-12, PO-05 a PO-07.
9. **Posto do piloto.** PO-01 a PO-04, PO-08.
10. **Proteções.** PR-01 a PR-05 e PR-10, encaixadas nos soquetes.
11. **Carroceria e escape.** PR-06 a PR-09 e MO-03. Formas livres convertidas e conferidas contra os
    vãos do regulamento.
12. **Fixadores e montagem final.** FX-01 a FX-04, por padrão, e interferência do conjunto.
13. **Desenho e PMI.** Folhas e tolerâncias sobre o conjunto final.
14. **PLM e regressão.** Liberar a revisão A, mudar o entre-eixos de 1040 para 1060 mm, recalcular tudo
    e registrar como teste de regressão.

## 4. Lacunas conhecidas no Fenkra

- **Corrente 219 e 428 fora da biblioteca.** `transmissao_dados.rs` não tem nenhuma das duas, e a regra
  do B342 exige duas fontes concordantes. O passo está confirmado, mas o rolo e a largura interna não.
  A etapa 7 fica bloqueada para a corrente 219 até haver decisão (seção 6).
- A disponibilidade de rolamento 50 × 80, terminal rotular e porca autotravante na Biblioteca não foi
  verificada nesta pesquisa.

## 5. Fontes

"Só fatos" quer dizer que os números podem ser citados como referência, mas texto, tabelas e desenhos
não podem ser reproduzidos.

### Lidas

| Ref. | Fonte | Licença ou termos | O que oferece | Pode usar? |
|---|---|---|---|---|
| F1 | [Regulamento técnico internacional de karting, CIK-FIA, 2026](https://www.fiakarting.com/sites/default/files/2026-01/6.0%20RT2025_Clean%20Web_1.2_16.12.2025_2.pdf) | "All rights reserved" | Limites de quadro, eixo, rodas, para-choques, carroceria | Só fatos. Texto e desenhos técnicos: não usar sem permissão |
| F2 | [Regulamento Nacional de Kart 2026, parte técnica, CBA](https://www.cba.org.br/upload/downloads//467/regulamento-nacional-de-kart-2026-regulamento-tecnico-.pdf) | Público, sem licença declarada | Passo de corrente, pinhão, eixo, rodas, largura, massas | Só fatos. Texto: não usar sem permissão |
| F3 | [Motorsport UK, regulamento nacional 2026, página 672](https://ebooks.motorsportuk.org/motorsport-uk-ncr-2026-edition-4/0952123001742394230/p672) | Termos não lidos | Comprimento, entre-eixos, bitola, altura, assoalho | Só fatos. Texto: não usar sem permissão |
| F4 | [Vogel, TCC UFSC 2026, rigidez torcional de chassi tubular de kart](https://repositorio.ufsc.br/handle/123456789/274335?show=full) ([PDF](https://repositorio.ufsc.br/bitstream/123456789/274335/3/TCC-Lucas_Vogel.pdf)) | "Open Access", sem licença Creative Commons indicada | Seção de tubo medida, rigidez, método | Só fatos, com citação. Coordenadas do quadro e figuras: não usar |
| F5 | [ANGRi Racing Academy, correntes, coroas e relações](https://copper-pike-6n4z.squarespace.com/chains-sprockets-and-ratios) | Sem aviso visível | Passo, faixa de elos, exemplos de relação | Só fatos. Texto: não usar sem permissão |
| F6 | [Pittalks, tamanhos de pneu de kart](https://pittalks.com/go-kart-tire-size-chart) | Termos não lidos | Leitura da designação do pneu | Só fatos |
| F7 | [Point Karting, rolamento de eixo de 50 mm](https://pointkarting.com/products/50-mm-axle-bearing) | Página de venda | Rolamento 50 × 90 mm | Só fatos |
| F8 | [KartPulse, discussão sobre diâmetro de tubo](https://forums.kartpulse.com/t/kart-chassis-tubing-diameter-durability-discussion-from-pro-tour-discussion/9496?page=2) | Fórum, termos não lidos | Ø30 e Ø32 citados por usuários | Só como indício |
| F9 | [KartPulse, relações com correntes 219 e 35](https://forums.kartpulse.com/t/gear-ratios-219-vs-35/10203) | Fórum, termos não lidos | Relato de outra classe, pouco aplicável | Não usado nas medidas |
| F10 | [Stavrou e Constantinou, Design of a Go-Kart, Instituto Técnico Superior de Chipre](https://athena.cut.ac.cy/files/original/ec69fc574706de0b7bd40677c46eee0574a97ff6.pdf) | Sem licença declarada | Relatório antigo, com regras da época; texto digitalizado com erros | Não usar sem permissão |
| F11 | [Raghunandan et al., Design and Analysis of Go-Kart Chassis, Zenodo](https://zenodo.org/records/164912) | CC BY 4.0 | Só a página do registro foi lida, sem medidas | Pode, com crédito. É a única licença aberta confirmada |
| F12 | [DIY Go Karts, planos](https://mail.diygokarts.com/kart-plans/kart-plans-main.html) | "© 2015", termos não lidos | Roteiro geral, sem medidas | Não usar sem permissão |
| F13 | [The Drive, kart elétrico com planos gratuitos](https://www.thedrive.com/features/my-cheap-electric-go-kart-is-better-than-ever-and-now-you-can-build-it-too.md) | Gratuito, sem licença formal | Kart de viga de alumínio, outro conceito | Não usar sem permissão |

### Só em trecho de busca ou com leitura bloqueada

Nenhum valor destas fontes entra como [V].

| Fonte | Situação | Pode usar? |
|---|---|---|
| [Artigo E3S, chassi de kart com vários aços](https://www.e3s-conferences.org/articles/e3sconf/pdf/2023/28/e3sconf_icmed-icmpc2023_01042.pdf) | 403; trecho cita entre-eixos 1210, bitola 940 e tubo Ø26,4 × 4 | Não usar sem permissão, até ler a licença |
| [Artigo do simpósio ISFT 2016](https://rdi.rmutsb.ac.th/digipro/isft2016/content/papers/Paper%2048.pdf) | Conexão recusada; trecho cita 1100, 850 e Ø25 | Não usar sem permissão |
| [Mihalić et al., Applied Sciences, 2023](https://www.mdpi.com/2076-3417/13/20/11312) | 403; resumo por índices, aço S355 | Não usar sem permissão, até ler a licença |
| [Mahapatra et al., Indian Journal of Science and Technology, 2020](https://indjst.org/articles/impact-behaviour-analysis-of-a-newly-designed-go-kart-chassis) | 403; trecho declara Creative Commons Attribution | Conferir antes de usar |
| [CBA, Kartcross, regulamento técnico 2023, adendo 1](https://www.cba.org.br/upload/downloads//709/kartcross-regulamento-tecnico-2023-adendo-1-.pdf) | Trecho: SAE 1020, mínimo Ø22; é outro tipo de veículo | Só fatos, depois de ler |
| [Folheto de pneus Mitas 2025](https://mitas-moto.com/-/media/mitas-moto/leaflets/mitas-go-kart-leaflet-en-2025-1.pdf?rev=-1) | 404; trecho: 11 × 7.10-5 com 276 × 210 mm | Só fatos, depois de ler |
| [Fab Academy, projeto de aluno](https://fabacademy.org/2026/labs/dilijan/students/daviti-khukhua/assignments/week19.html) | Trecho cita CC BY-NC-SA 4.0 | Não usar: cláusula não comercial |
| [Purdue, folha do kart elétrico](https://engineering.purdue.edu/evGrandPrix/highschool/Documents/EV-Kart.pdf) | PDF só com imagem, sem texto; trecho cita manga Ø25 e pino Ø8 | Não usar sem permissão |

### Descartadas

Páginas de planos em domínios suspeitos, cópias em sites de compartilhamento de documentos sem
autorização clara, e uma imagem "blueprint" gerada por IA: não são fontes técnicas confiáveis e têm
direito autoral incerto.

## 6. Decisões do usuário

1. **Corrente.** A 219 é a escolha fiel ao conceito, mas exige completar dados na biblioteca com duas
   fontes, ou medir uma corrente real. A alternativa é usar ISO 08B-1, de passo 12,7 mm, já gerável,
   mudando a relação e o tamanho da coroa. Recomendação: manter a 219 e registrar a falta como item.
2. **Carroceria regulamentar.** Seguir os vãos da regra, como proposto, ou liberar a forma, se o teste
   priorizar a Forma livre.
3. **Motor.** Manter volume simplificado, como proposto, ou projetar um motor próprio, o que aumentaria
   bastante o número de peças.
4. **Tubo principal.** Confirmar Ø30 × 2,4 mm ou preferir Ø32 × 2,0 mm.
