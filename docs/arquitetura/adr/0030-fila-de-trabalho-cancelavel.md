# ADR 0030 — Fila de trabalho cancelável fora da thread da interface

## Contexto

O recálculo já rodava numa thread própria, mas um pedido novo esperava o anterior terminar mesmo já superado
(uma booleana ou uma malha longa), e a interface não sabia o andamento.

## Decisão

- `fenkra_core::trabalho::Fila`: uma thread, um pedido novo **cancela** o em andamento (sinal em
  `Controle`) e o resultado superado é descartado. Pedidos acumulados se fundem (`fundir`), herdando
  por exemplo o "reconstruir do zero". O estado pesado nasce dentro da thread, então não precisa ser `Send`.
- **Kernel:** `fenkra_kernel::ligar_cancelamento(bandeira)` registra a bandeira na thread. As booleanas e a
  malha passam a receber uma `Message_ProgressRange` ligada a ela (`Message_ProgressIndicator::UserBreak`) e
  devolvem o erro "cancelado" em vez de um resultado parcial.
- **Recálculo:** ao ver cancelamento no meio de um comando, o recálculo não grava esse comando no cache e o
  marca como desatualizado; o cálculo seguinte recomeça dali e dá o resultado de um cálculo do zero. Cada
  comando terminado conta no `Controle`, e a barra de status mostra "recalculando… n/total".

## Consequências

- Só booleanas e malhas têm ponto de interrupção dentro do OpenCASCADE. Filete, casca e varredura ainda só
  param entre comandos.
- A composição/tesselação da montagem e a atualização das vistas do desenho ainda rodam na interface: pedem
  a mesma fila, mas dependem de mover o cache de peças (que guarda formas não `Send`) para dentro da thread.
