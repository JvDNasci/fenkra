# ADR 0025: conjunto único de seleção e seleção por intenção

Estado: aceita (parte da etapa de recompute pendente). Item B95.

## Contexto

A árvore guardava a seleção em `multi`/`sel` e o 3D em `picks`, cada um com a própria regra de clique:
no 3D clicar de novo no único item selecionado o desmarcava, na árvore não. Selecionar na árvore não
acendia nada no 3D e escolher no 3D não acendia a árvore. O filtro de tipos era só o da barra, mesmo
quando o comando aberto só aceitava faces ou arestas. Faltavam formas de pegar de uma vez as faces
tangentes de um filete, a cadeia de arestas de um contorno e os furos coaxiais.

## Decisão

1. **Uma função de clique para os dois lados** (`selecao::aplicar`, modos Trocar, Somar, Alternar). O
   clique simples nunca desmarca; desmarcar é Ctrl ou clicar no vazio.
2. **O conjunto é a união, por Id da árvore**: seleção múltipla da árvore, item único e o dono de cada
   escolha do 3D (`selecao::dono_na_arvore`). Mudou, a cena é refeita e o que está no conjunto acende
   no 3D; a árvore realça as linhas do conjunto.
3. **Filtro temporário vem do comando aberto** (`selecao::tipos_do_comando`) e não é gravado: ao fechar
   o comando o filtro da barra volta sem ninguém restaurar nada.
4. **Seleção por intenção é pura sobre a malha** (`selecao_regras`): tangentes por `edge_smooth` e
   continuidade de direção, cadeia por vértice de grau dois, concêntricas por eixo dos círculos. O
   laço usa o mesmo núcleo da seleção em caixa (`selecionar_regiao`) com teste de ponto em polígono.

## Pendente

Gravar a regra na referência v2 (`FaceKey`/`EdgeKey`) e refazê-la no recompute exige que `Topology`
exponha a suavidade da aresta e o eixo dos círculos (hoje só a `Mesh` tem). Até lá a regra expande no
momento da seleção e o comando guarda as faces e arestas resultantes, cada uma com a chave v2 própria.

## Consequências

- Mudança de comportamento: clicar no único item selecionado do 3D não o desmarca mais.
- O realce no 3D passa a mostrar a seleção da árvore, não só o item sob o mouse.
