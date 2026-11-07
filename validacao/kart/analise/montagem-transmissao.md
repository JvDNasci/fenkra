# Análise pós-kart: Montagem, Transmissão, Biblioteca e fixadores

Autor: `vis_mon`. Abri 0047, 0048, 0049, 0087, 0088, 0089, 0107, 0109, 0127 e 0132 a 0137, mais recortes
ampliados de 0047, 0087 e 0132. As capturas `_pintado` dos subconjuntos comparei por amostragem de
pixels. Conferi os `.fka` em `modelo/` e `src/conjunto.rs`.

**Não vi, porque não há captura:** nada da Biblioteca (FX-01 a FX-04, EX-03, EX-04, DI-07, DI-11), o
`TESTE-FIXADORES`, o `TESTE-TRANSMISSAO`, o gerador (`TR-GER08B`), nenhum quadro de simulação com
a direção virada e nenhuma vista lateral real. O diário não tem entradas da bancada Biblioteca. Sobre
isso só tenho os números do relatório.

## 1. O que foi usado e o que não foi

| Recurso | Uso real, contado nos `.fka` | Por quê |
|---|---|---|
| Componente com posição gravada | 84 nós | A modelagem foi em código, com posição calculada do mestre |
| Fixar | Quase tudo; 84 de 84 em `KART-PLANO` e 8 de 8 em `KART` | Atalho do teste |
| Restrição | **1** em todo o kart (Coincidência em SC-EIXO) | Idem |
| Juntas | 7, todas em SC-DIRECAO, mais 2 nos testes | Só a direção foi modelada como mecanismo |
| Fixação conjunta | 4 (1 no eixo, 3 na direção) | |
| Acoplamento de engrenagem, simulação | Só em `TESTE-TRANSMISSAO` e `TR-GER08B` | Parou em K-07 |
| Gerador de transmissão | 1 chamada com ISO 08B-1; recusa a 219 (F-01) | As peças TR ficaram à mão |
| Biblioteca | 4 fixadores e a chaveta como peça solta; rolamento e terminal por revolução | F-02 a F-04 |
| Preencher furos | Tentado, 0 fixadores (K-18) | |

**Não usados** (0 nos `.fka`): repetição, parâmetros da montagem, estados de exibição, animação,
explosão e pilhas. Também não foram usados Automática, Recorte pela peça e Copiar geometria.

**A matriz de `resultados.md` engana:** "Restrição 8, Junta 8, Rígido 8" conta chamadas de `r.usa` por
subconjunto, não o uso. O real é 1, 7 e 4.

**Peças que ficaram fora de todo subconjunto:** DI-04, DI-07, DI-09, DI-11, EX-04 e FX-01 a FX-04. O
kart não tem pino-mestre, chaveta, rolamento dianteiro, terminal, mancal da coluna nem um único
fixador montado. A lista de peças sai "certa" para o que foi posto, mas não para o kart.

## 2. O que deu certo

- **Conjunto reconhecível:** 84 peças abrem e parecem um kart, com o motor do lado certo
  (`0132_kart_geral_colorido.png`, `0133_kart_lateral_colorido.png`).
- **Direção em laço fechado:** 7 juntas e símbolos claros no 3D. O diário registra 8 GDL, que batem com
  a conta: CH-09:2 solto (6) mais o giro próprio dos dois tirantes (2)
  (`0135_sc-direcao_pronta_colorido.png`).
- **Eixo traseiro:** peças coaxiais, ordem legível, mancais sentados nos suportes CH-08
  (`0134_sc-eixo_pronta_colorido.png`, recorte de `0132`).
- **Corrente por `Caminho`:** o laço de 106 rolos fecha. Conferi à mão: com passo 7,774 e 11:82, 106 elos
  dão centros de cerca de 213 mm, igual ao `DC` do mestre (`0049_tr-03_pronta.png`).
- **Árvore da montagem de topo:** limpa, com 7 subconjuntos e o mestre (`0109_kart_subconjuntos_pintado.png`).

## 3. O que falhou

**Defeitos do produto, novos:**

- **Massa errada na legenda.** A mesma coroa mostra 0,330 kg em `0047` (Alumínio 6061) e **0,959 kg** em
  `0127` (Alumínio 7075). O volume é 122.218 mm³, então 0,959 kg é densidade 7,85, a de aço. O 7075
  existe em `fenkra_core::materiais`, com 2,81, mas `pmi_ui.rs:961` e `editor.rs:748` procuram em
  `app::MATERIALS`, uma segunda tabela de 8 itens, e caem em 7,85 sem aviso. É diferente de F-06, em que o
  material não existe em tabela nenhuma.
- **"Definido" confunde fixado com restrito.** `0132` diz "84 definidos, 0 GDL livres" com 0 restrições.
  O relatório leu `dof=0 resolvida=true` no KART, que só prova que tudo foi fixado. Status verde não
  informa nada sobre a montagem.
- **Restrição sem efeito passa calada.** Em `SC-EIXO.fka`, EX-01 e os três mancais estão com `fixed: true`,
  então a Coincidência liga dois corpos fixos e o eixo não gira. Os 18 GDL são só os três rolamentos
  soltos. O comentário do código diz "a junta dá o giro", mas há 0 juntas. `conjunto.rs` não fixa EX-01:
  não descobri de onde veio esse `fixed`.
- **Barra de histórico de peça na montagem.** Todas as capturas de montagem mostram "Em trabalho:
  Filete1", "Reconstruir" e 5/5, que não se aplicam ao documento aberto (`0132`, `0109`).
- **Balões sobrepostos.** O relatório diz que "saem limpos", mas em `0135` há balões um sobre o outro (7,
  1 e 2, 8), em `0136` o 1 fica sob o 3, e em `0134` os balões 2 e 9 não aparecem.

**Defeitos conhecidos, confirmados na imagem:**

- **K-25:** `_pintado` é idêntico ao original nas amostras (0,00% de diferença em 0091/0107, 0093/0109,
  0084/0100/0114, 0087/0103, 0088/0104 e 0089/0105). A pintura não chegou à montagem.
- **K-07, K-18 e F-01:** seguem como em `bugs.md`.

**Limites do teste, que parecem piores que "posição aproximada":**

- **Coroa e pinhão não têm dentes.** São discos com furos fechados (`0047` ampliado, `0048`). O disco tem
  Ø dp+6 e o assento Ø4,9 fica no raio dp/2, então sobra uma parede de 0,55 mm e o corte nunca abre na
  borda. O relatório diz "Dentes por parâmetro": calculou, mas não é roda dentada. A folha fina
  também não fabrica.
- **Corrente invertida em SC-MOTOR.** O laço grande envolve o pinhão, e a ponta pequena fica no eixo
  traseiro (`0087`, recorte). `Caminho::novo` ordena os raios, então a origem de TR-03 é a roda menor
  (`0049`), e o teste a pôs no centro da coroa. Nenhuma captura mostra coroa, pinhão e corrente juntos,
  por isso passou.
- **Transmissão partida:** TR-01 está em SC-EIXO, TR-02 e TR-03 em SC-MOTOR. Juntas são locais ao `.fka`,
  então a relação não existe no kart nem existiria sem K-07.
- **Pedais deitados e sobrepostos:** PO-05 e PO-06 têm o comprimento em Y, foram postos sem rotação e
  se cobrem (`0088`).
- **Linha de freio solta:** EX-12B não toca a pinça nem o cilindro-mestre (`0088`).
- **Peças no ar:** escoras PO-03 soltas, sem tocar no banco (`0133`, `0089`, recorte de `0132`); CH-12 no
  meio do quadro, longe de qualquer tubo (`0136`, balão 5); as barras laterais não casam com os pontões
  (`0137`); o para-choque fica meio enterrado no bico (`0137`).
- **Subconjuntos sem vínculo:** SC-QUADRO tem 12 peças soltas e 72 GDL, com suportes que deveriam estar
  soldados. CH-10 ×4 está em fila na origem, não nos para-choques (`conjunto.rs:75`).

## 4. Deu certo, mas dá para melhorar

- **Capturas de montagem:** planos e rótulos da origem aparecem em tudo (`0132`). Na peça pintada a origem
  já some (`0127`). A montagem precisa do mesmo.
- **`KART-PLANO`:** contorno de K-03 que perde hierarquia, fixações e juntas. Ele é o que aparece nas
  imagens bonitas, e a direção ali não vira.
- **Legenda errada:** "Kart de lado" (`0133`) é a frente. O cubo diz "Direita" porque o nariz aponta para
  +X. Vale fixar a convenção no mestre.
- **Corrente:** 1 corpo com 106 sólidos soltos e sem placas. Serve de marcação, mas o nome promete elos.
- **Cor em dois passos:** repetir em `Node.color` funciona, mas perde o acabamento (K-25). `0132` sai plano
  perto de `0127`.

## 5. Melhores imagens para o vídeo

| Arquivo | O que mostrar |
|---|---|
| `0132_kart_geral_colorido.png` | O resultado: 84 peças formando um kart |
| `0135_sc-direcao_pronta_colorido.png` | Mecanismo de verdade: juntas, tirantes e coluna no 3D |
| `0134_sc-eixo_pronta_colorido.png` | Eixo traseiro com balões e a coroa em cor |
| `0133_kart_lateral_colorido.png` | Frente do kart: volante, banco e motor do lado certo |
| `0109_kart_subconjuntos_pintado.png` | Montagem de topo: mestre mais 7 subconjuntos (e o K-25) |
| `0049_tr-03_pronta.png` | Corrente gerada pelo caminho das rodas |

Evite `0047`, `0048` e `0127` (roda sem dentes), `0087` (corrente invertida) e `0088` (pedais deitados).

## 6. Regras para a skill da bancada

- Só conta como montada a peça restrita à vizinha: fixar com posição gravada dá `dof=0` e não prova nada.
- Peças ligadas por acoplamento ficam no mesmo `.fka`: junta é local ao arquivo.
- Compare a lista de peças com a árvore do produto: "ok" não vê a peça que ficou de fora.
- A origem de `Caminho` é a roda menor: posicione a corrente pelo pinhão e capture as três peças juntas.
- Roda dentada só vale se o corte abrir na borda: confira o dente na imagem, não o volume.

## 7. Itens novos de backlog

| Título | Bancada | Prio |
|---|---|---|
| Legenda 3D e propriedades usam `app::MATERIALS` em vez de `fenkra_core::materiais`: massa 2,8× maior em 7075, sem aviso | Interface/PLM | A |
| Segunda rodada da montagem do kart: restrições reais, dentes abertos, corrente na posição, peças faltantes, relação coroa e pinhão no mesmo conjunto, eixo girando | Validação | A |
| Status da montagem deve separar "fixado" de "definido por restrição" | Montagem | M |
| Mecanismo de subconjunto na montagem de topo: a direção não vira no KART. Não achei subconjunto flexível por busca de nome | Montagem | M |
| Avisar restrição sem efeito entre corpos fixos | Montagem | M |
| Capturas que faltam: Biblioteca, gerador, simulação, transmissão junta e lateral real | Validação | M |
| Barra de histórico de peça aparece na montagem ("Em trabalho: Filete1") | Interface | B |
| Afastar balões sobrepostos, ou avisar | Montagem/PMI | B |
| Ocultar a origem na montagem em modo de apresentação | Interface | B |
| Matriz de comandos do kart deve contar uso real, não chamadas de `r.usa` | Validação | B |
