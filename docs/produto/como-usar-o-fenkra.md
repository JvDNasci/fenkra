# Como usar o Fenkra

Guia para quem vai modelar no Fenkra. Explica os gestos que valem em todo o programa e, por bancada, o
que cada comando faz e quando usar. O Fenkra está em alpha: onde um comando ainda tem defeito conhecido,
o texto diz **em correção**, sem data. O estado foi levantado em 21/11/2026 (develop depois da alpha.41).

A meta é que todo comando funcione do mesmo jeito (contrato do [ADR 0107](../arquitetura/adr/0107-contrato-de-feature.md)).
Enquanto a migração não termina, alguns comandos ainda usam o jeito antigo; o guia avisa onde.

Termos: veja o [glossário](glossario.md).

---

## 1. Navegação na vista 3D

| Gesto | O que faz |
|---|---|
| **Botão do meio + arrastar** | Gira a vista. A órbita é livre, 360° em qualquer direção: passa por cima e por baixo da peça (Alt+L troca para giro de mesa, que para em 90°) |
| **Shift + botão do meio + arrastar** | Desloca a vista (panorâmica) |
| **Roda do mouse** | Zoom, em direção ao ponto sob o cursor |
| **Botão direito** | Só abre o menu de contexto. Não gira nem desloca |
| **Alt+C** (ou duplo clique no botão do meio sobre o modelo) | Define o centro da órbita no ponto sob o cursor |
| **N** | Alinha a vista de frente para a face sob o cursor (normal à face) |
| **F** | Enquadra tudo |
| **O** | Alterna perspectiva e ortográfica |
| **Ctrl+1 a Ctrl+7** | Vistas padrão (frente, trás, direita, esquerda, topo, base, isométrica) |
| **H** / **Shift+H** | Esconde o selecionado / mostra o que estava escondido |
| **Cubo de vistas** (canto superior direito) | Clique numa face, aresta ou canto do cubo para ir àquela vista |

Os atalhos dependem do estilo de navegação escolhido nas preferências; a tabela mostra o padrão.

---

## 2. Seleção

### 2.1 Na vista e na árvore

- **Clique** seleciona a face, aresta, vértice, plano ou curva sob o cursor. Clique simples nunca
  desmarca o que já estava marcado.
- **Ctrl + clique** soma ou tira um item do conjunto. **Clique no vazio** limpa a seleção.
- **Alt + roda** sobre o modelo troca entre os elementos empilhados sob o cursor ("selecionar outro").
- **Arrastar da esquerda para a direita** faz seleção por janela (pega o que fica todo dentro);
  **da direita para a esquerda** faz seleção cruzada (pega o que toca).
- A **árvore** e a vista 3D compartilham a mesma seleção: clicar num item da árvore acende no 3D e
  vice-versa.
- **Esc** desfaz um nível por vez: fecha a lista aberta, depois limpa a seleção, depois sai do comando.

### 2.2 Caixa de seleção do comando

Dentro de um comando, cada entrada (perfil, arestas, faces, superfície...) é uma **caixa de seleção**. A
caixa ativa fica destacada e é ela que recebe o próximo clique, no 3D ou na árvore.

- **Tab** e **Shift+Tab** passam para a próxima caixa ou voltam; **Enter** avança enquanto houver caixa
  adiante e, na última, conclui.
- Na caixa em forma de lista: clique marca o item, **Ctrl** soma, **Shift** marca um intervalo, **Del**
  ou **X** tira os marcados. Clicar de novo no 3D sobre um item já escolhido o tira.
- Durante o comando, o filtro de seleção aceita só o tipo que a caixa ativa pede (ex.: só arestas).
- O comando mostra **A ESCOLHER** enquanto falta entrada obrigatória; não é erro, a prévia só espera.

**Em correção:** a caixa em forma de árvore, com os mesmos gestos em todo campo, hoje existe em poucos
campos (preencher, junção, loft de superfície, filete e chanfro, abas). Em vários comandos a entrada
ainda é uma lista antiga sem Tab (casca, ângulo de saída, espessura, edição direta), um menu de sketch
(comandos de perfil), um menu de plano, ou um texto com coordenada e o botão "usar a face selecionada"
(rosca, joggle, dobra, veneziana, placa, esquadro). A migração está nas ondas do ADR 0107.

### 2.3 Cadeia e junção

- **Cadeia tangente**: numa caixa de curvas ou arestas, a opção "seguir tangentes" faz um clique pegar
  toda a sequência tangente. Hoje vale na aba curva, no menu "Extrair com tangência" e, ao abrir, em
  Preencher, Junção e Junção de curvas.
- **Junção na hora**: a meta é que curvas de origens diferentes (dois sketches, uma aresta do corpo e uma
  curva extraída) entrem juntas no mesmo campo e o comando as una sozinho, com "unir" no botão direito do
  item. **Em correção**: hoje cada comando de perfil aceita um sketch inteiro só; para juntar curvas de
  origens diferentes, crie antes uma **Junção de curvas** (Superfícies).
- **Extrair** continua sendo Extrair: a face, aresta ou borda extraída fica na árvore como recurso
  Extrair, ligado à peça. Use Extrair quando quiser reaproveitar uma geometria da peça como entrada de
  outro comando. **Em correção**: a aresta extraída ainda perde o tipo exato (vira curva aproximada).

---

## 3. Fluxo de um comando

Todo comando segue a mesma ordem de cima para baixo no painel:

1. **Suporte**: o que recebe a operação (perfil, arestas, faces, superfície). Escolha no 3D ou na árvore.
2. **Modo**: a variante (ex.: aba de aresta, de perfil, de superfície; filete constante ou variável).
3. **Parâmetros**: valores, direção (botão **⇄ Inverter direção** ou a seta clicável no 3D), limites e
   posição do material.
4. **Avançado**: opções raras, recolhidas.

Durante o comando:

- **Prévia**: o resultado aparece translúcido assim que o suporte é válido e muda a cada campo. Em
  comandos pesados a prévia é leve (linhas ou caixas) e o resultado completo vem ao concluir.
- **Alças no 3D**: setas, arcos e losangos puxam valores (comprimento, ângulo, raio, quantidade). Clique
  no rótulo do valor para digitar; campos aceitam expressão com parâmetros.
- **Erro**: aparece no topo do painel e ao lado da prévia, com a causa. Corrija o campo e a prévia volta.
- **Concluir** (Enter), **Ctrl+Enter** (aplicar e continuar no mesmo comando), **Cancelar** (Esc).

**Em correção:**
- Hoje o Concluir não bloqueia com erro: o comando entra na árvore com ✖ e o assistente de reparo abre.
  A meta é validar antes do OK, esperar o cálculo com a tela congelada e não fechar o painel em falha.
- Se o cálculo ainda está rodando, o Concluir pode fechar antes de o resultado voltar.
- O botão de inverter ainda tem nomes diferentes em alguns comandos (caixas "inverter sentido", "para
  fora", "espelhar seção") e falta em outros (revolução, varredura, loft, padrão, extrusão e revolução de
  superfície).

---

## 4. Clique único e duplo clique no ícone

- **Clique único** no ícone roda o comando **uma vez** e volta à seleção ao concluir.
- **Duplo clique** **prende** o comando: o ícone fica realçado e o comando reabre sozinho a cada
  conclusão. Solte com **Esc**, com novo clique no ícone ou abrindo outro comando.
- No sketch vale para linha, retângulo, círculo, arco, ponto, cota e restrição.
- **Duplo clique numa face** da peça abre o comando que a criou.

---

## 5. Árvore

- A árvore fica à esquerda. Com um comando aberto, o painel do comando ocupa a coluna e a árvore recolhe
  numa seta; com o painel solto (flutuando), a árvore fica transparente sobre a vista.
- A ordem da árvore é a **linha de tempo**: os comandos são calculados de cima para baixo. O **marcador
  de rollback** desativa o que vem depois dele.
- **O recurso nasce dentro do gerador**: o sketch consumido por uma extrusão aparece dentro dela; o que um
  comando gera fica pendurado nele. É só apresentação: a ordem de cálculo não muda.
- **Arrastar** um item muda a posição na linha de tempo (quando as dependências permitem) ou o leva para
  uma pasta. Pastas não mudam a ordem de cálculo.
- Marcas: ✖ erro, ⚠ aviso, ⟳ desatualizado. Clique com o botão direito para editar, suprimir, renomear,
  apagar ou "Atualizar esta feature".
- O **assistente de reparo** abre sozinho quando um comando perde a referência e fecha quando não sobra
  nada quebrado; ele mostra candidatas no 3D para trocar a entrada perdida.

---

## 6. Bancadas e comandos

Legenda: **ok** = funciona no uso comum; **em correção** = tem defeito conhecido no backlog.

### 6.1 Sketch

| Comando | Quando usar | Estado |
|---|---|---|
| Linha, retângulos (2 pontos, centro, 3 pontos, paralelogramo), círculos (centro, 3 pontos, tangente a 3), arcos (3 pontos, centro), rasgo, polígono, ponto | Desenhar o perfil plano de um comando | ok. **Em correção**: a linha sem estar presa termina no primeiro segmento (a meta é seguir encadeando até Esc) |
| Spline | Curvas livres (perfil de asa, contornos orgânicos) | **em correção**: a spline tem três definições internas e chega facetada a alguns comandos (B559) |
| Cotas (distância, horizontal, vertical, ângulo, raio, diâmetro) e restrições (coincidente, horizontal, vertical, paralela, perpendicular, tangente, igual, concêntrica, ponto médio, fixa, simétrica) | Prender a forma e a medida | ok; faltam colinear, continuidade de spline e cota de arco |
| Mover, copiar, espelhar, offset, filete, chanfro, aparar (clique ou pincel), estender, converter, intersecionar | Editar o desenho | ok |

Digite o valor junto ao cursor para cotar enquanto desenha; H e V forçam horizontal e vertical; Ctrl
desliga o encaixe.

### 6.2 Sólidos

| Comando | Quando usar | Estado |
|---|---|---|
| Extrusão (adicionar, remover, interseção) | Dar espessura a um perfil numa direção, com oito términos (cego, plano médio, dois sentidos, passante, até o próximo, até o último, até plano, até superfície) | ok; perfil de um sketch só |
| Revolução | Peças de giro em torno de um eixo | ok; sem inverter e sem término "até" (em correção) |
| Furo | Furo simples, rebaixado, escareado ou contrafurado, com rosca cosmética ou modelada, pela norma | ok |
| Varredura | Perfil acompanhando uma guia (nervura, tubo, rasgo) | ok; sem inverter |
| Multi-seções (loft) | Transição entre dois ou mais perfis | ok; sem guias |
| Nervura (reforço) | Parede fina a partir de linhas do sketch | ok |
| Primitiva | Caixa, cilindro, esfera e afins sobre um plano | ok |
| Recursos plásticos (ressalto, encaixe, grelha, lábio) | Peças injetadas | ok |
| Filete e chanfro | Arredondar ou chanfrar arestas; filete variável, face a face e completo | ok |
| Casca | Esvaziar o sólido abrindo faces | ok; lista de faces antiga (em correção) |
| Ângulo de saída, espessura, edição direta | Mexer em faces de uma peça pronta | ok; lista de faces antiga (em correção) |
| Rosca | Rosca numa face cilíndrica, pela norma | ok; face escolhida por botão (em correção) |
| Padrão, espelho, escala, mover corpo | Repetir, espelhar, escalar ou posicionar | ok; padrão e espelho sem inverter |
| Combinar | Booleana entre corpos de coletores diferentes | ok |
| Espessar, fechar, costurar, dividir corpo, preencher por fronteiras | Passar de superfície para sólido | ok; o lado só se inverte pela seta do 3D (em correção) |
| Forma livre | Modelar por gaiola de subdivisão e converter em B-rep | ok |
| Malha | Importar, reparar, decimar, booleana de malha, converter | ok |

### 6.3 Superfícies e wireframe

| Comando | Quando usar | Estado |
|---|---|---|
| Planos, pontos, linhas e curvas de referência | Construir o esqueleto de uma forma | ok; curvas viram aproximação interna (em correção, B559) |
| Extrair face, várias faces, contorno, aresta | Reaproveitar geometria da peça como entrada | ok; aresta extraída perde o tipo exato (em correção) |
| Extrusão e revolução de superfície | Superfície a partir de uma curva | **em correção**: sem inverter, sem simétrico, uma curva só (B561, B562), superfície de spline facetada (B559) |
| Varredura, varredura avançada, multi-seções, regrada | Superfícies guiadas por curvas | ok; entrada de curva única |
| Preencher, concordância, conectar | Fechar buracos e emendar com continuidade G0/G1/G2 | ok; continuidade medida em amostras (em correção) |
| Deslocar | Superfície paralela a outra | ok, o mais completo em validação |
| Junção, junção de curvas, curar, desaparar, parte | Costurar e consertar superfícies | ok |
| Dividir, aparar, regiões, extrapolar | Cortar, ficar com um lado, estender | ok; inverter só pela seta (em correção) |
| Filete entre superfícies e filete de aresta | Arredondar entre superfícies | ok; o painel ainda não mostra o inverter |
| Transformar, análises | Mover/girar/espelhar elementos; conferir conexão e desvio | ok |

### 6.4 Chapa metálica

Espessura, raio, fator K, material e estilo ficam em **Chapa › Parâmetros**, não em cada comando.

| Comando | Quando usar | Estado |
|---|---|---|
| Parede base (de sketch ou de superfície) | Primeira placa da peça | **em correção**: perfil em spline fechada sai deformado (B558, B559) |
| Aba de aresta, aba curva, laço de abas | Dobrar abas em arestas retas, curvas ou no contorno de uma face | ok; **em correção**: os três modos viram um só modo "Arestas" (B560) |
| Aba de perfil | Aba que segue um perfil aberto desenhado | ok |
| Aba por superfície | Aba que encosta numa superfície (nervura de asa), com limites de lado 1, lado 2 e borda final | **em correção**: falta posição do material, inverter em todas as combinações e prévia rápida; ainda não planifica (B563, B564, B566) |
| Bainha | Dobrar a borda sobre si mesma | ok |
| Joggle (desvio em Z), dobra por linha, dobra cônica, veneziana | Dobras e conformações pontuais | ok |
| Estampo, furo com colar, recorte | Conformar, furar e recortar a chapa | ok |
| Aba entre perfis | Transição entre dois perfis fechados | ok; planificação aproximada |
| Alívio, fechamento e rasgo de canto | Tratar cantos entre abas | ok |
| Converter em chapa | Transformar um sólido de parede uniforme em chapa | ok |
| Desdobrar e redobrar (global ou parcial), planificada, DXF | Ver e exportar a chapa planificada | ok; **em correção**: alternar dobrado/planificado recalcula tudo (B566) |

### 6.5 Estrutural e soldas

| Comando | Quando usar | Estado |
|---|---|---|
| Membro | Perfil do catálogo varrido ao longo das linhas guia | ok em linhas retas; **em correção**: caminho com dobra, spline e várias guias como um membro só (B575); guia por Extrair (B576) |
| Cantos entre membros | Meia-esquadria, aparar, encaixe, dobra de tubo | ok; **em correção**: encaixe de tubo com corte normal à parede e nó de vários tubos (B574) |
| Placa de ponta, esquadro | Fechar pontas e reforçar encontros | ok; ponto escolhido por botão |
| Solda | Cordão entre dois conjuntos de faces, cosmético ou sólido | ok |
| Lista de corte | Barras com comprimento, ângulos e exportação | ok; falta tabela de dobras de tubo (B575) |

### 6.6 Montagem

| Comando | Quando usar | Estado |
|---|---|---|
| Inserir | Trazer peças e submontagens | ok |
| Restrições e juntas | Posicionar e dar movimento (coincidente, distância, ângulo, engrenagem, revolução, prismática...) | ok; a prévia resolve numa cópia e só grava se houver solução |
| Repetição | Copiar componentes em linha, círculo ou pelos furos | ok |
| Explosão e animação | Vistas explodidas e passos | ok |
| Lista de peças, furo de montagem, fixadores, interferência, estudo de movimento | Documentar e conferir a montagem | ok |

Montagem ainda usa seleção e botões de inverter próprios; entram no padrão comum numa onda posterior.

### 6.7 Desenho

| Comando | Quando usar | Estado |
|---|---|---|
| Vistas (padrão, projetada, isométrica, planificada) | Criar folhas a partir da peça ou montagem | ok |
| Cortes e detalhes | Mostrar o interior e ampliar regiões | ok |
| Cota inteligente | Cotar: o que está sob o cursor sugere o tipo, Tab troca | ok |
| Balões, tabelas, marcas de centro, desenho automático | Completar a folha | ok |

### 6.8 PMI 3D

Anotações no próprio modelo (medidas, tolerâncias geométricas, rugosidade, solda, notas). Escolha os
elementos no 3D; se a seleção já serve, a anotação nasce na hora. Ela acompanha o elemento quando a peça
muda e fica marcada como quebrada se ele some. Exporta pelo STEP.

---

## 7. Quando algo dá errado

- **✖ na árvore**: abra o assistente de reparo; ele diz a causa e, se a referência se perdeu, mostra
  candidatas no 3D.
- **Comando A ESCOLHER**: falta entrada obrigatória; a caixa destacada diz qual.
- **Resultado facetado em superfície de spline**: defeito conhecido (B559), em correção.
- Reporte o que achar estranho: cada reporte vira item no backlog.
