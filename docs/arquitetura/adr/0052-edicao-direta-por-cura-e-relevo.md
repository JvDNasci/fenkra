# ADR 0052: edição direta de faces por cura do kernel e relevo transladado

Data: 2026-11-06. Item: B33. Um corpo vindo de STEP chega sem sketch nem comandos; sem editar pelas faces ele
não serve para nada depois da migração.

## Contexto

O Fenkra recalcula tudo por histórico: cada comando vira uma ferramenta e entra no corpo por booleana. Um
corpo importado não tem histórico, então mover um furo, tirar um filete ou crescer um boss precisa agir
direto nas faces e ainda manter as referências (dono e papel de cada face) que o resto do documento usa.

## Decisão

Um comando só, `DirectEdit`, com quatro operações. Todas partem do corpo atual do coletor e terminam na
verificação de sólido válido (volume positivo, sólido fechado, análise de topologia do kernel); se falhar,
o comando fica em erro e o corpo anterior fica como estava.

- Apagar: o kernel remove as faces e as vizinhas se estendem até se encontrar (cura).
- Deslocar: o algoritmo de offset do kernel recebe a distância só para as faces escolhidas e zero para o
  resto. Em cilindro e esfera isso muda o raio.
- Mover: cura as faces; a diferença de volume entre o corpo curado e o original é o relevo. Se o curado é
  maior, o relevo é uma reentrância (furo, rasgo): o vazio é transladado e subtraído de novo. Se é menor, é uma
  saliência (boss, nervura): o excesso é transladado e unido de novo. O volume nunca muda.
- Raio do filete: cura a face do filete, acha a aresta viva que reaparece (reta, paralela ao eixo, perto
  dele) e arredonda de novo com o raio novo. Zero desfaz o filete. Só filetes cilíndricos.

As etiquetas de face passam por cada passo (a cura e as booleanas propagam o dono e o papel).

## Consequências

- Escolher só uma face plana para "Mover" falha com mensagem (apagá-la não muda o volume): para empurrar uma
  face plana a operação certa é Deslocar.
- Filete de raio variável e canto esférico não entram em "Raio do filete" nesta versão.
- A cura depende do kernel: casos em que as vizinhas não se encontram falham com a causa, sem estragar o corpo.
