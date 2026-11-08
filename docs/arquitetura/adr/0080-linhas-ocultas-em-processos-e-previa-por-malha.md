# ADR 0080: linhas ocultas exatas em processos, prévia por malha em threads (B393)

## Contexto

Depois da ADR 0079 a carga do kart caiu para cerca de 40 a 50 s, mas as linhas ocultas (HLR) ainda levavam de 9 a
26 s por vista, uma depois da outra: 72 s para as quatro. A vista projetada esperava a mãe chegar, e a fila mandava
uma vista por vez à thread de projeção.

Tentamos rodar a HLR exata do OpenCASCADE em várias threads, cada uma com a sua cópia profunda da forma. O
resultado sai errado sem erro nenhum: quatro cópias da mesma vista isométrica deram 2067, 2063, 2068 e 0 linhas,
contra 2067 da sequencial. O algoritmo exato guarda estado global e não serve a threads. A projeção por malha
(`HLRBRep_PolyAlgo`) deu o mesmo número de linhas nas quatro threads em três rodadas seguidas.

## Decisão

1. **Lote.** As vistas pedidas no mesmo quadro vão juntas à thread de projeção (`fundo::Lote`): a montagem é
   carregada uma vez. `pedir` só enfileira; quem manda é `seguir`, chamado por `sondar` uma vez por quadro.
2. **A projetada não espera a mãe.** Ela só usa a direção, não as linhas da mãe; corte e detalhe continuam
   esperando.
3. **Duas etapas por lote.** Primeiro a prévia, projetada sobre a malha em uma thread por vista
   (`fenkra_kernel::hlr_varias` com `poligonal`), marcada `Proj::previa` e com "prévia" na folha. Depois a exata,
   que a substitui. O corte vai direto à exata, para não refazer a peça cortada duas vezes.
4. **Exata em processos.** A thread grava a forma em binário (`Shape::para_bin`, uma vez por topologia) numa pasta
   temporária e chama o próprio executável com `--fenkra-hlr` uma vez por vista, até quatro ao mesmo tempo,
   abaixo da prioridade normal e sem janela (`hlr_processos`). As linhas voltam num arquivo binário. Cancelar
   encerra os processos, então agora a HLR para de verdade. Sem executável conhecido, com uma vista só ou com
   processo que cai, a vista é projetada na própria thread, uma depois da outra.
5. `hlr_varias` exata (sem `poligonal`) roda uma vista depois da outra, para ninguém chamá-la em threads por engano.

## Medição (kart, 84 peças, quatro vistas: frente, duas projetadas e isométrica)

Máquina de desenvolvimento ocupada por outros processos (os números variam de uma rodada para outra), testes
`medir_kart` em release, um por vez:

| Etapa | Antes | Agora |
|---|---|---|
| Carga das formas | 47 a 54 s | igual (fora deste item) |
| HLR exata das quatro, uma depois da outra | 72 a 106 s | 37 a 52 s em quatro processos |
| Primeiras linhas na folha (prévia por malha) | não havia | 7,7 a 10,6 s depois da carga |
| Folha toda, pela thread de projeção | 126 a 135 s | prévia aos 57 s, exata aos 95 s |

A exata nos processos dá as mesmas linhas da sequencial (1373, 2728, 1019, 2067).

## Consequências

- O executável do aplicativo atende `--fenkra-hlr` antes de abrir a janela (`main.rs`). Os testes usam o binário
  `fenkra-hlr` do crate de desenho.
- Cada processo carrega a forma inteira: quatro processos no kart somam algumas centenas de MB a mais por um
  instante.
- A prévia tem arestas retas (curvas viram polígonos da malha). É só para ver a folha cedo; a exportação deve
  esperar a exata.
