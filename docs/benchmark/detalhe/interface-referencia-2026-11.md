# Interface: referência de superfícies e sólidos (2026-11)

Estudo de material público sobre como o "software de referência de superfícies" (versão 5, bancadas de
wireframe e superfície) e o "software de referência de sólidos" organizam a interface. Tudo reescrito
com nossas palavras. Onde as fontes públicas não cobriram o ponto, está marcado **[não confirmado]**
e a regra do Fenkra vira decisão de projeto, não cópia.

Convenção de teste: cada regra "Fenkra deve" é verificável por teste de estado (documento de interface
montado em código) ou por captura com `FENKRA_DEMO*`.

---

## 1. Personalização de barras de ferramentas

### Como a referência faz
1. Cada barra tem uma alça na ponta. Arrastar a alça solta a barra: ela flutua como janelinha.
2. Soltando perto de qualquer borda da janela (topo, base, esquerda, direita), a barra encaixa ali.
   No padrão da bancada de superfícies, as barras ficam à direita e no topo.
3. Barra flutuante tem um "x" para fechar. Barra fechada some, mas continua na lista de barras do
   menu de exibição (clique direito numa barra ou menu Exibir > Barras), onde se marca de novo.
4. Diálogo de personalização (menu Ferramentas > Personalizar, ou clique direito numa barra) com abas:
   - Barras: lista todas, botão "Novo" (barra própria), "Restaurar posição" (volta ao layout padrão),
     "Restaurar tudo/conteúdo".
   - Comandos: lista de categorias e "todos os comandos"; arrasta-se o comando para a barra própria.
     Comando sem ícone aparece com o nome em texto.
5. Barras de bancadas compatíveis podem ser reaproveitadas em outra bancada (ex.: wireframe e sólido).
6. O layout é guardado por bancada (cada bancada tem seu conjunto de barras e posições) no perfil do
   usuário, não no documento.

### O que o Fenkra deve fazer
- R1.1 Toda barra tem alça visível; arrastar a alça por mais de 6 px a destaca em janela flutuante.
- R1.2 Zonas de encaixe nas 4 bordas, com retângulo de prévia da zona antes de soltar. Soltar fora de
  zona deixa flutuando.
- R1.3 Barra flutuante tem "x"; fechar não apaga: a barra continua listada em "Barras" (menu e clique
  direito) com caixa de marcação. Teste: fechar, reabrir pela lista, posição anterior mantida.
- R1.4 "Restaurar posição" por bancada e "Restaurar tudo". Teste: bagunçar, restaurar, comparar com o
  layout padrão serializado (igualdade exata).
- R1.5 "Nova barra" + arrastar comando da lista para ela; comando sem ícone mostra rótulo. Remover
  comando da barra própria arrastando-o para fora. Barras do sistema não perdem comandos (só ocultam).
- R1.6 Layout salvo por bancada, em arquivo de perfil do usuário (versionado, tolerante a chaves
  desconhecidas). Trocar de bancada carrega o layout daquela bancada.
- R1.7 Comando disponível em outra bancada pode ser colocado em barra própria sem duplicar o código.
- R1.8 Strings novas em `i18n.rs`; cores só do tema.

### Armadilhas
- Barra própria "sumindo" depois de atualização: a referência tem defeitos conhecidos de barra
  personalizada desaparecendo. No Fenkra, ao carregar um layout cuja barra referencia comando
  inexistente, manter a barra e mostrar o item como indisponível, nunca descartar a barra inteira.
- Barra fora da tela ao trocar de monitor/resolução: limitar a posição flutuante à área visível ao
  carregar.
- Restaurar tudo não pode apagar barras próprias sem confirmação.

---

## 2. Árvore de especificações

### Como a referência faz
1. A árvore fica na área do modelo, sobre o 3D, e pode ser ampliada/movida com o mouse (pan e zoom
   da árvore independentes da câmera). **[não confirmado]** o fundo transparente: as fontes públicas
   consultadas não o descrevem; é decisão do Fenkra (já exigida pelo padrão B319).
2. Arrastar e soltar itens: um item pode ser levado de um conjunto geométrico para outro; o item
   mantém nome e conteúdo, só muda de lugar. Corpos de peça só se movem para dentro/fora de um
   conjunto ordenado.
3. Menu de contexto do conjunto: "Reordenar filhos", "Mudar de conjunto geométrico" (para
   multisseleção, quando o arrastar não serve).
4. Há um conceito de "objeto em trabalho": o item onde o próximo recurso será inserido. Arrastar um
   corpo ou conjunto o torna o objeto em trabalho. Inserir depois de = definir o objeto em trabalho
   antes de criar.
5. Restrições: não se cria conjunto ordenado dentro de não ordenado e vice-versa; arrastar com
   multisseleção funciona em uma versão e não na outra.

### O que o Fenkra deve fazer
- R2.1 Árvore sobreposta ao 3D com fundo transparente (somente texto/ícones com contorno suave, nunca
  caixa opaca), largura redimensionável pela borda, mover pelo cabeçalho, zoom da árvore (Ctrl+roda
  sobre ela) sem mexer na câmera. Teste de captura: peça sempre distinta do fundo atrás da árvore.
- R2.2 Arrastar item: mostrar (a) linha de inserção entre dois itens = "inserir depois de"; (b)
  realce do contêiner inteiro = "mover para dentro". Nada de queda ambígua: só uma indicação por vez.
- R2.3 Soltar em destino inválido mostra cursor proibido e motivo em uma linha (ex.: "um recurso não
  pode vir antes do que ele usa"). Nunca falha em silêncio.
- R2.4 Mover recurso com dependências: validar o grafo antes de aplicar; se criar ciclo ou usar algo
  ainda inexistente, recusar com a lista de dependências.
- R2.5 Multisseleção arrasta em bloco, preservando a ordem relativa.
- R2.6 "Inserir depois de" explícito: marcador de "ponto de inserção atual" na árvore; novo recurso
  entra após ele e o marcador avança.
- R2.7 Desfazer/refazer cobre mover. Nome e identidade (ID estável) do item não mudam ao mover.
- R2.8 Ação alternativa por menu (mover para...) para quem não arrasta (acessibilidade).

### Armadilhas
- Mover pode quebrar referências posteriores: recomputar e mostrar status, com opção de desfazer.
- Soltar sobre item errado por tremor do mouse: limiar de arraste e área de queda com margem.
- Corpos de sólido e conjuntos de superfície têm regras diferentes de movimento: expor a regra no
  texto de recusa, não só bloquear.

---

## 3. Barra de propriedades gráficas (topo)

### Como a referência faz
1. A barra vem oculta por padrão; liga-se pelo menu de barras. Os valores mostrados refletem o
   elemento selecionado.
2. Campos: cor de linha/face, espessura da linha, tipo de linha (contínua, tracejada...), símbolo do
   ponto, camada (por padrão "nenhuma"). Transparência e preenchimento ficam no diálogo de
   propriedades (aba gráfica) com um controle deslizante, não na barra.
3. A lista de cor da barra altera só faces quando o objeto é sólido/superfície. Para linhas, o
   conjunto de campos ativos muda: ponto mostra símbolo; curva mostra espessura e tipo.
4. Pincel (copiar formato): copia as propriedades gráficas de um elemento para outro.
5. Em montagens, símbolo de ponto, espessura e tipo de linha ficam no nível da peça e não herdam.
6. Armadilha documentada: transformar linha em linha de construção perde atributos e eles não voltam.

### O que o Fenkra deve fazer
- R3.1 Barra no topo com os campos: cor, transparência (slider 0 a 100%), espessura, tipo de linha,
  símbolo de ponto, camada, material. Visível por padrão (a referência a oculta; nós ligamos).
- R3.2 Campos inaplicáveis ficam desabilitados com tooltip do motivo (ex.: "símbolo só vale para
  pontos"), em vez de sumir, para a barra não pular de tamanho.
- R3.3 Seleção mista: campo mostra "vários" quando valores diferem; escolher um valor aplica a todos.
- R3.4 Sem seleção, a barra edita o padrão do que será criado a seguir.
- R3.5 Mudança aplicada como uma única operação desfazível; propriedade gráfica não dispara
  recompute geométrico.
- R3.6 Cores só pelo tema; todo valor de cor novo testado com preto e branco atrás.
- R3.7 Pincel de formato (copiar/colar propriedades) com atalho.
- R3.8 Preservar atributos ao converter para construção (guardar e restaurar ao reverter).

### Armadilhas
- Transparência: peça transparente demais some no fundo; limitar o mínimo de opacidade visível ou
  manter contorno sempre.
- Acabamento da peça não chega na montagem (lição K-25): conferir na peça aberta e na montagem.
- Camada por padrão "nenhuma" confunde: nomear como "sem camada" em português.

---

## 4. Barra de vistas padrão (embaixo)

### Como a referência faz
A barra de vista fica na base. O que as fontes públicas confirmam: enquadrar tudo (ajusta o zoom para
caber todo o conteúdo na área gráfica), vista normal (alinha a câmera perpendicular ao que foi
selecionado), vista isométrica (clique repetido gira entre as oito direções isométricas), estilos de
exibição (sombreado, sombreado com arestas, com arestas ocultas, com material, aramado), ocultar/mostrar,
trocar o espaço visível, multivistas (automática, a partir de três vistas adjacentes, ou manual) e
aplicar material. **[não confirmado]** a ordem exata dos ícones; as fontes consultadas só tratam de
parte deles. Ordem proposta pelo Fenkra, por decisão de projeto:

### O que o Fenkra deve fazer
- R4.1 Conjunto padrão, nesta ordem, da esquerda para a direita:
  1 Enquadrar tudo, 2 Zoom na seleção, 3 Vista normal à seleção, 4 Vistas padrão (menu: frente, trás,
  esquerda, direita, topo, base, isométrica; clique repetido na isométrica gira entre as 8),
  5 Estilo de exibição (menu: sombreado, sombreado com arestas, arestas ocultas, aramado, material),
  6 Ocultar/mostrar, 7 Trocar espaço visível (visível <-> oculto), 8 Multivistas, 9 Aplicar material.
- R4.2 Barra única, na base, centralizada, encaixável como as demais (regra R1). Teste: ordem dos ids
  de comando igual à lista acima.
- R4.3 Cada botão com atalho e tooltip com GIF; ícone da família "vista".
- R4.4 Enquadrar tudo ignora geometria oculta e auxiliares desligados (origem, rótulos), segundo a
  lição do kart.
- R4.5 Trocar espaço visível não apaga nada: só inverte a flag de visibilidade de exibição; reversível.
- R4.6 Multivistas: layouts de 1, 2 e 4 vistas, com câmera independente por vista e seleção
  compartilhada.
- R4.7 Estado de vista (estilo, vista) guardado por documento; layout de barra guardado por perfil.

### Armadilhas
- Vista normal sem seleção: definir comportamento (usar o plano de trabalho ativo) e dizer na dica.
- Isométrica cíclica confunde: mostrar no ícone/tooltip qual das 8 direções está ativa.
- Aplicar material em montagem não pode propagar para fora da peça aberta sem aviso.

---

## 5. Comandos de wireframe e superfície organizados pelo elemento resultante

### Como a referência faz
1. Um comando por elemento resultante, com o tipo escolhido dentro do diálogo:
   - Ponto: coordenadas; sobre curva; sobre plano; sobre superfície; centro de círculo/esfera/elipse;
     tangente em curva; entre dois pontos (proporção).
   - Plano: deslocamento (offset); paralelo por ponto; ângulo/normal a plano; por três pontos (falha
     se colineares); por duas retas; por ponto e reta; por curva plana; normal à curva; tangente à
     superfície.
   - Linha, curva e superfície seguem o mesmo princípio (tipo dentro do painel).
2. Operações que produzem resultado a partir de outros elementos ficam separadas: interseção,
   projeção, extração, junção. O resultado pode ter vários pedaços desconexos.
3. Resultado múltiplo: abre aviso/diálogo de gestão de resultados múltiplos. Duas saídas: "mais
   perto / mais longe" de uma referência (escolhe um pedaço), ou "extrair" (cada pedaço vira recurso
   próprio). Se o recurso já tem filhos, edita-se o recurso gerador.
4. Repetição de pontos: criar vários pontos em uma curva por espaçamento ou quantidade, a partir de
   um ponto de referência, sem refazer o comando.

### O que o Fenkra deve fazer
- R5.1 Um comando por elemento: Ponto, Linha, Plano, Curva, Superfície. Menu "Tipo" dentro do painel;
  trocar o tipo preserva entradas compatíveis e limpa as incompatíveis com aviso.
- R5.2 Cada tipo declara as entradas exigidas e valida antes de criar (ex.: três pontos colineares =
  erro legível com sugestão).
- R5.3 Interseção, Projeção, Extrair, Junção são comandos separados e não duplicam os de criação.
- R5.4 Resultado com vários pedaços: o recompute devolve a lista; o painel mostra "N pedaços" e as
  opções "manter o mais perto/mais longe de [referência]" e "extrair todos". Padrão: manter todos
  agrupados, sem escolher nada em silêncio.
- R5.5 "Mais perto/longe" guarda a referência; se na atualização a referência deixar de existir ou o
  resultado mudar de contagem, o recurso fica com status de aviso (não some).
- R5.6 Repetição de pontos: quantidade ou espaçamento, a partir de um ponto/extremidade, com prévia
  no 3D de todos os pontos; resultado como um recurso com N filhos.
- R5.7 Prévia no 3D e manipulação direta do que for numérico (arrastar offset, ângulo).
- R5.8 Testes: monta curva + plano, interseção com 2 pontos, confere contagem de pedaços, perto/longe
  escolhe o esperado, extrair gera N recursos.

### Armadilhas
- Mais perto/longe é frágil: após edição a escolha pode trocar de pedaço. Preferir identidade
  topológica estável ou mostrar aviso claro.
- Tipos demais no mesmo painel poluem: agrupar e esconder campos não usados.
- Interseção de elementos tangentes dá resultado instável; usar tolerância e informar.

---

## 6. Medir (entre elementos e item)

### Como a referência faz
1. Dois comandos: medir entre (dois conjuntos de elementos) e medir item (um elemento).
2. Modos de seleção: medida única (padrão), multisseleção em cada lista, seleção complementar
   (preenche a segunda lista com o restante).
3. Tipos de resultado entre elementos: distância mínima (padrão), distância máxima, máxima de 1 para
   2, medida ao longo de uma direção; ângulo; plano a plano; eixo; geometria infinita (tratar
   plano/reta como ilimitados).
4. Medir item: comprimento, raio, diâmetro, área, ângulo, centro, conforme o tipo do elemento.
5. Mostra caixa/rótulo com os valores na tela; aba "Personalizar" escolhe o que aparece; modo de
   cálculo/critério de refinamento controla precisão.
6. "Manter medida": marcada, a medida é criada como recurso persistente no ramo de notas da árvore
   (nome tipo Medida.N), associativa (atualiza quando o modelo muda). Desmarcada, vira só valor e não
   entra na árvore; some ao fechar. Medida bloqueada (sem vínculo) tem ícone diferente. Remover vínculo
   pelo menu de contexto. Clique duplo na árvore reabre para editar.
7. Pode-se ligar uma medida a um parâmetro (associação medida-parâmetro).

### O que o Fenkra deve fazer
- R6.1 Dois comandos (ou um com modo): "Medir item" e "Medir entre". Fenkra pode chamá-los de
  "paquímetro" na dica.
- R6.2 Medir item por tipo: aresta (comprimento), arco/círculo (raio, diâmetro, centro), face (área,
  perímetro), ângulo entre arestas. Valores em unidade do documento; copiar valor com um clique.
- R6.3 Medir entre: distância mínima e máxima, ângulo, plano a plano, eixo a eixo, e "tratar como
  infinito" (plano/reta/eixo). Mostrar ambos os pontos de apoio no 3D com a linha de medida.
- R6.4 Seleção: simples, multisseleção por lista, complementar. Cada lista visível no painel, com
  botão de limpar.
- R6.5 Rótulo/caixa ancorados no 3D, legíveis por tema, que seguem a câmera e não cobrem a peça;
  campos escolhidos por caixas de marcação.
- R6.6 "Manter medida": ligada cria recurso "Medida N" na árvore (ícone próprio), guardado no
  documento, referenciando por identidade estável, e recalculado a cada recompute. Desligada: valor
  efêmero, sem tocar no documento.
- R6.7 Medida persistente com referência perdida: estado "órfã" visível; nunca mostrar valor velho
  sem sinalizar.
- R6.8 Ligar medida a parâmetro (somente leitura ou condutora, escolha explícita).
- R6.9 Testes: bloco 10x20x30, comprimento 30, distância mínima entre faces opostas = 10, ângulo entre
  faces perpendiculares = 90; alterar parâmetro e conferir atualização do valor persistente.

### Armadilhas
- Medida bloqueada vs. associativa: deixar explícito por ícone e tooltip.
- Seleções de tipos incompatíveis: bloquear antes do cálculo, com mensagem do que falta.
- Precisão: mostrar a precisão usada e permitir mais casas decimais; distâncias em NURBS dependem de
  tolerância.

---

## 7. Painéis de comando e assistentes de reparo

### Como a referência faz
1. Diálogos de comando são janelas que o usuário arrasta para onde quiser, ficando sobre a área
   gráfica. **[não confirmado]** que lembram a posição entre usos em todos os casos; a prática comum
   do software é reabrir próximo à última posição da sessão.
2. Assistentes de atualização/erro (como o aviso de resultado múltiplo, descrito em 5) abrem de modo
   automático só quando a situação exige (ex.: elemento não conexo detectado na criação). Não ficam
   abertos fora disso.

### O que o Fenkra deve fazer
- R7.1 Painel de comando livre e encaixável; aberto só durante o comando; ao concluir/cancelar,
  fecha (alinha com o padrão B319).
- R7.2 Lembrar posição e tamanho por comando, persistido no perfil. Ao reabrir, validar contra a área
  visível; se fora da tela, reposicionar. Teste: mover, fechar, reabrir, mesma posição; resolução
  menor, painel visível.
- R7.3 Painel nunca cobre a seleção em curso por padrão (primeira abertura perto da borda direita).
- R7.4 Assistentes (resultado múltiplo, referência perdida, atualização falha): abrem só com erro ou
  ambiguidade, explicam em uma frase a causa e oferecem ações diretas (reparar, escolher, ignorar,
  editar recurso). Sem erro, nenhum assistente aparece. Teste: documento válido = zero assistentes.
- R7.5 Erro de recompute leva ao recurso com problema, com prévia do que falhou.
- R7.6 Enter confirma, Esc cancela; foco inicial no primeiro campo vazio obrigatório.
- R7.7 Dica por campo e tooltip com GIF.

### Armadilhas
- Painel que abre fora da tela após mudar de monitor.
- Assistente em cascata: um erro, uma janela; agrupar vários erros do mesmo recompute numa lista.
- Lembrar posição entre bancadas diferentes: guardar por comando, não global.

---

## Fontes

Ajuda pública oficial do software de referência de superfícies, blogs e tutoriais públicos de revendas,
notas públicas de suporte e a wiki do FreeCAD, só lidos. A lista com os links fica no cofre do
projeto (`Conhecimento/Estudo de cursos/fontes-interface-2026-11.md`), para que o repositório não
cite marcas. Resumo em `docs/arquitetura/procedencia.md` (18/11/2026).
