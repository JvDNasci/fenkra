# Quadros longos ao carregar o kart (B259)

## Pergunta

O aceite do B259 pede nenhum quadro acima de 100 ms ao carregar a montagem de referência (o `KART.fka`, 84
componentes). Nas medições anteriores, com outros agentes compilando na mesma máquina, apareciam quadros de
290 ms e 1077 ms, cada vez numa etapa diferente e trivial (avisos, painéis, árvore). A suspeita era o sistema
parar a thread da interface, não código do programa.

## Como separar programa de sistema

`FENKRA_MEDIR_QUADROS` passou a gravar, para cada passada da interface (de `App::ui` até a saída dele):

- `parede`: o tempo de relógio da passada;
- `cpu_thread`: quanto a thread da interface de fato rodou nela, usuário mais núcleo (`GetThreadTimes`);
- `ocioso_sistema`: a fração ociosa de todos os processadores no mesmo intervalo (`GetSystemTimes`).

Medir só a passada, e não o intervalo entre quadros, tira da conta a espera por evento (a interface parada não
desenha quadro). A classificação (`medir_quadros::causa`, igual em `scripts/benchmark/quadros_longos.py`):

| Caso | Leitura |
|---|---|
| CPU da thread ≥ metade da parede | programa: código nosso trabalhando |
| CPU baixa e máquina com menos de 10% ociosa | sistema ocupado: a thread esperou por processador |
| CPU baixa com processador livre | espera: paginação de memória, disco, placa de vídeo ou trava |

## Medição (13/11/2026, release, abrindo `KART.fka` com a câmera girando sem parar)

| Rodada | Passadas | Maior | p99 | Mediana | Máquina ociosa |
|---|---|---|---|---|---|
| Máquina livre (nenhuma compilação) | 2502 | 28 ms | 5,8 ms | 3,3 ms | 94% |
| 20 processos girando a CPU (16 núcleos lógicos; na média a máquina ficou 38% ocupada) | 2002 | 36 ms | 7,9 ms | 5,9 ms | 62% |

O maior quadro contado pelo eframe na rodada livre foi 45 ms (no começo, ao montar a janela). A composição da
montagem (27 s) rodou toda em segundo plano (`compor_fundo`), sem passar para a thread da interface.

## Conclusão

Nenhum quadro acima de 100 ms, com a máquina livre e com outros processos disputando a CPU: nessas condições
a thread da interface continua sendo escalada. Os quadros de 290 ms e 1077 ms das
medições anteriores não se repetiram e não vêm de etapa nenhuma do programa; o que diferia naquelas rodadas
era a máquina de 16 GB com vários compiladores em paralelo (cada `rustc` com 0,7 GB), ou seja, memória
apertada e paginação, que param qualquer thread sem que ela gaste CPU. Para conferir um caso novo, rode a
medição e o script: um quadro longo com pouca CPU da thread é do sistema.
