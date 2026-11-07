# ADR 0077: setas do PMI com ancoragem própria, cordão de solda e etapas no pacote (B368, B369)

## Contexto

A seta de uma nota sempre nascia do lado voltado para a ponta, sem escolha, e a legenda não tinha como
dizer de qual borda a seta saía. O pacote de visualização levava as anotações, mas o visualizador as
desenhava como um rótulo solto no centro, sem seta, sem lado e sem as capturas como etapas. A solda
mostrava só a aresta realçada, sem o tamanho do cordão.

## Decisão

1. **A ancoragem fica no estilo da anotação**, por seta: `PmiStyle::leader_side` (0 automático, 1
   esquerda, 2 direita, 3 topo, 4 base) e `leader_arrow` (ponta própria). Listas vazias significam o
   comportamento anterior, então arquivos antigos abrem sem mudança. Não mexe em `Pmi` nem no número de
   âncoras; as contas ficam em `fenkra_core::pmi_leader`.
2. **O cordão é só exibição.** `WeldExtra::hide_bead` esconde; o padrão mostra. A perna vem da medida do
   símbolo (`weld_leg_mm`: `z5` é a perna, `a4` é a garganta e vale `a·√2`). Nada disso entra no sólido,
   na massa ou na lista de corte.
3. **O pacote ganha campos aditivos**, sem subir a versão do formato (leitores antigos ignoram o que não
   conhecem): `pmi[].setas` (âncora, lado, ponta e vértices de cada seta) e `pmi[].cordao` (perna, se é
   para mostrar e os segmentos da aresta marcada). `capturas` já existia e continua na ordem de criação.
4. **O visualizador desenha as anotações numa tela 2D sobre a cena**, projetando os pontos do modelo, para
   a seta sair da borda da caixa como no aplicativo. O cordão é um cilindro por segmento na cena 3D.
   As capturas viram etapas: aba própria, barra com anterior e próximo, e só as anotações da captura.

## Consequências

- Duas listas paralelas ao `leader_via`, mantidas juntas por `remove_tip`.
- O pacote recalcula a peça uma segunda vez quando há solda, para achar a aresta; sem solda, não paga.
- Soldas de montagem ainda não levam o cordão ao pacote (a montagem não tem a malha da peça à mão).
