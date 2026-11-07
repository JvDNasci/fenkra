# Relatório do projeto-teste B353: kart no Fenkra

Modelo: `validacao/kart/modelo/`. Código reproduzível: `validacao/kart/src/` (rodar `kart.exe`, ou
`kart DIAG`, `kart ACAB`). Resultado de cada item, com status, volume, caixa e matriz de comandos:
[resultados.md](resultados.md), gerado pelo programa. Bugs: [bugs.md](bugs.md). Capturas úteis:
`capturas/`. Base: develop com B373, B374 e B375 mesclados.

## Resumo

- 70 peças da árvore tentadas, mais quadro soldado, 10 variações para cobrir comandos, 7 subconjuntos,
  montagem de topo, 8 desenhos, PMI, PLM e pacote.
- Todas as peças têm `.fkp` com status sem erro, exceto EX-12 na primeira tentativa (a segunda passou).
  "Ok" aqui quer dizer que o comando calculou e a caixa bate; a qualidade está nas notas abaixo. Os
  volumes não foram comparados com conta fechada, peça por peça.
- Fixadores: as peças existem, mas nenhuma pilha foi colocada, porque o comando recusa Ø8,5 (K-18).
- As seis lacunas de biblioteca e gerador têm contorno, então a peça existe mas não é a peça de catálogo.
- A regressão do entre-eixos funcionou nas peças: quadro de 1347,3 para 1367,3 mm. Só funcionou rodando
  a atualização em cada subconjunto (K-04), e as posições da montagem não acompanham (ver Limites).

## Como cada peça foi feita

Modelei em lote, antes da orientação de pensar a fabricação peça a peça. A linha "como se fabrica"
foi escrita depois, para as peças já feitas, e mostra onde a escolha não foi a melhor.

| Peças | Como se fabrica | Bancada e comando | Nota 1–5 | Por quê |
|---|---|---|---|---|
| MESTRE | Não é peça: parâmetros e pontos | Parâmetros com mínimo, máximo e verificação; pontos, polilinhas e planos publicados | 3 | Limites do regulamento entram bem. Falta sketch 3D de verdade: o esqueleto é ponto a ponto por expressão |
| CH-01 a CH-07, QUADRO | Tubo dobrado e soldado | Estrutural: Membro com canto Dobra e Encaixe, lista de corte | 3 | Dobra e desenvolvido certos (1212,2 mm). K-02 obriga a publicar cada trecho; K-09 deixa pontas sem recorte |
| CH-08, CH-11 a CH-14, DI-05, TR-04, PO-02, PO-04 a PO-06 | Chapa cortada e dobrada | Chapa: Parede, Aba, Recorte, Furo, planificação | 4 | Fluxo direto, tabela de dobras certa. Aresta por ponto é frágil fora da interface |
| CH-09, EX-02, EX-05 a EX-11, DI-02, DI-04, DI-08 a DI-10, MO-01, MO-02 | Usinagem | Sólidos: Revolução, Extrusão, Furo, Padrão, Filete, Chanfro, Saída, Unir corpos | 4 | Nenhuma falha de kernel. K-13 e K-14 custam tentativas |
| EX-01 | Tubo usinado | Sólidos, com a cota Ø pelo mestre. O gerador de eixos seria mais simples, mas não faz tubo vazado | 4 | Quatro rasgos com espelho |
| RD-01, RD-02 | Aro fundido ou usinado | Sólidos | 3 | Peça sai, mas a interferência acusa volume falso (K-10) |
| RD-03, RD-04 | Pneu moldado | Superfícies: spline, revolução, engrossar | 4 | Direto e dirigido pelo mestre |
| DI-01, PO-01, PR-06 a PR-09 | Moldado ou laminado | Forma livre e conversão em B-rep | 2 | Converte, mas a gaiola foi feita por código. O bico e o pontão ficaram volumes maciços, não cascas. Anel sem primitiva (F-05). O banco leva 16 s para recalcular |
| DI-03, DI-06, PO-03, PO-07, PR-01 a PR-05, PR-10, CH-10 | Tubo e barra | Estrutural | 4 | Dobras de para-choque sem erro |
| TR-01 a TR-03 | Peça de catálogo | Fiz à mão, por dente em padrão e círculos; o certo era o gerador de transmissão | 2 | Ver comparação abaixo |
| EX-03, EX-04, DI-07, DI-11, FX-01 a FX-04 | Peça de catálogo | Biblioteca para chaveta, parafuso e porca; rolamento e terminal por revolução | 3 | Sem rolamento, terminal e autotravante (F-02 a F-04) |
| EX-12, MO-03, PO-08 | Tubo, escape, tanque | Superfície falhou em EX-12 (K-08); varredura sólida e multi-seções passaram | 3 | Mensagem do erro aparece no comando seguinte |

## Coroa, pinhão e corrente: à mão contra o gerador

A pedido do coordenador, medi o gerador de transmissão depois de ter feito as três peças à mão.

| Caminho | Esforço | Resultado |
|---|---|---|
| À mão, corrente 219 | Cerca de 30 linhas e três peças: disco, assento de rolo, padrão de 82; círculos em cada elo | Dentes por parâmetro; corrente é só rolos, sem placas, e sem junta nem relação |
| Gerador, corrente 219 | 1 chamada | Recusa: "corrente '219' fora da biblioteca" (F-01) |
| Gerador, ISO 08B-1 | 1 chamada: 5 peças, 2 juntas, razão 0,1341, 104 elos, centros corrigidos para 334,32 mm | Pronto para simular, mas a roda é maior que o kart comporta e a volta para em 5 de 9 quadros (K-07) |

O gerador é muito melhor, e só a falta da 219 impede usá-lo. As peças TR-01 a TR-03 ficaram à mão.

## Aparência e material

Rodado em todas as peças pelo `kart ACAB`: quadro vermelho brilhante, carroceria branca e painel
vermelho, pneus pretos, aros dourados, coroa laranja, alumínio escovado, aço polido, pedais amarelos.
Material: AISI 4130 e borracha não existem na tabela (F-06); a massa usa densidade padrão nessas peças.

| Nível | Aparência | Material |
|---|---|---|
| Peça aberta | Funciona: cor e acabamento aparecem (`ch-08_painel_aba_pintado`) | Funciona, com F-06; a legenda 3D mostra material e massa |
| Subconjunto e montagem | Não funciona: tudo cinza (K-25). Com `Node.color` a cor aparece (`kart_geral_colorido`), mas sem acabamento metálico | A lista de peças do desenho traz o material certo de cada peça |
| Montagem de topo com subconjuntos | Não usa as cores dos nós de dentro: fica numa cor só (`kart_subconjuntos_pintado`) | Massa de 1415,9 kg na legenda: o bico e o pontão maciços pesam demais |

## Por bancada

| Bancada | Resultado | Evidência |
|---|---|---|
| Estrutural | Funciona, com K-02 e K-09 | QUADRO: 1 corpo, 1.238.479 mm³, 3 tubos dobrados, lista de corte com 8 itens; captura `quadro_topo` |
| Chapa | Funciona | Planificação do assoalho e do suporte com tabela de dobras; estampo, veneziana, colar, bainha, desvio e conversão passaram |
| Sólidos | Funciona, com K-10 | Sem erro de comando; um sólido inválido passou sem aviso |
| Superfícies | Funciona, com K-08 | Pneus, extrusão, deslocada, preenchimento |
| Forma livre | Funciona por código | Conversão em todas as peças; interface não exercitada |
| Montagem | Parcial | Direção com laço fechado resolve até ±30° no volante; K-07 trava a transmissão; K-04 no topo |
| Desenho | Parcial | 8 folhas exportadas; K-03 com subconjuntos; 126 s no conjunto |
| PMI | Funciona | STEP com PMI levou 3 de 3 no eixo e na manga; nota e legenda ficam fora do STEP, como esperado. Balões nos 7 subconjuntos |
| PLM | Funciona | 84 de 84 liberados em 2,6 s, revisão B, regressão ok; pacote de 1170 s (K-17) |

## Peças com PMI, capturas e referências

Cumprido em todas as peças da árvore pelo `kart ACAB`: legenda e nota, capturas "Todas as anotações",
"Sem anotações", "Notas e legenda" e, onde há tolerância, "Tolerâncias"; origem e referências ocultas.
Foi feito por código nos campos do documento. O caminho pela interface não foi conferido.

## O que um usuário sentiria (visto nas capturas)

- Bom: a bancada Montagem abre certa, a árvore mostra PN, quantidade e estado; os balões saem limpos.
  O painel de Aba é claro: "Entradas completas", abas Suporte, Modo, Parâmetros, Avançado, e prévia
  com alças no 3D. Não cobre a árvore nem o modelo.
- Ruim: peça de tubo ou Forma livre abre em Modelagem Sólida (K-20). Rótulos `mestre_...` poluem a
  vista (K-24). Duas anotações na mesma âncora ficam ilegíveis (K-22). "Aviso" em rosca normal (K-21).
- Mensagens enganosas: "linha de comprimento zero" para trecho de curva ligada (K-02); erro de
  superfície aparecendo no Engrossar seguinte (K-08); "furo sem tamanho compatível" em Ø8,5 (K-18);
  vista vazia sem aviso em montagem com subconjuntos (K-03).
- Falhas silenciosas, as piores: K-03, K-04, K-10, K-25 e K-09. O usuário só nota quando confere.
- Tooltips: nenhum aparece em captura sem passar o mouse, então não foi possível avaliar animação nem
  texto. Ficam como não conferidos, junto do B377.
- Peças longas de calcular: PO-01 passa de 9 s e a captura padrão pegou a tela de abertura.

## Correções a `resultados.md`

A tabela foi gerada na rodada completa. Depois dela, em `kart DIAG` e `kart ACAB`:

| Item | Resultado novo |
|---|---|
| CV-EST | Ok: placa, reforço e cordão, depois de corrigir os pontos das faces. O erro da tabela foi pontos meus |
| FX-PILHA | FALHA, K-18 |
| TR-GER219 | FALHA, F-01 |
| TR-GER08B | Ok, com K-07 na simulação |
| RD-02A, RD-02B | Diagnóstico de K-10 |
| Direção | Volante até ±30°: ok, manga externa a 30° e interna a 8°. A ±45° o solver não fecha: é ponto morto do meu desenho da placa, não defeito |
| Interferência do conjunto | 111 achados em 123 s; a maior parte é posição aproximada minha ou K-10 |

## Limites do que foi validado

- A montagem é aproximada. As posições foram calculadas a partir do mestre na hora de gerar o `.fka` e
  ficam gravadas como números. Quase nada usa restrição, exceto eixo no mancal e as juntas da direção.
  Mudar o entre-eixos não move a dianteira na montagem; o relatório de interferência tem muitos pares
  que são erro meu de posição, não do produto.
- Motor, banco e carroceria são volumes simples. Bico e pontão são maciços.
- Nada passou por mouse real. Prévia, arraste, alças, seleção e tooltips com animação não foram
  conferidos. A interface foi vista só em capturas estáticas.
