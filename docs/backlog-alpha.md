# Backlog das versões alpha

Melhorias pedidas após o protótipo 0.1.0-alpha.1, organizadas em entregas. Marque `[x]` quando concluir.
Versionamento flexível enquanto o projeto for local (sem distribuição).

## alpha.2 — Interface, visualização e organização ✔

- [x] Sequência de comandos **só na árvore** (sem barra de linha de tempo embaixo); marcador de rollback na árvore
- [x] Menu de contexto na árvore para **aparência** (cor/material) da peça, do coletor e do comando
- [x] Barra de comandos em **grupos** (Sketch, Básicos, Formas, Referências, Organização) com ícone grande + nome
- [x] Grupos **livres**: fixar no topo, na lateral ou flutuar e arrastar pela tela
- [x] Ícones ilustrados em SVG (`assets/icons/`), editáveis no VectorCraft/Inkscape
- [x] **Configurações**: tema claro/escuro, fundo sólido ou **degradê de duas cores**, tamanho de fonte, padrões de cotagem
- [x] **Estilos de exibição**: sombreado com arestas, sombreado, sem arestas, linhas ocultas, arame, raio-X, semi-renderizado
- [x] **Ocultar/mostrar** por item, **ambiente de ocultos** (alternar entre visíveis e ocultos) e **filtro de exibição** (planos, sketches, pontos, eixos, corpos, referências)
- [x] Aparência da peça, do coletor e de **cada comando** (cor + acabamento: fosco, acetinado, brilhante, metal escovado/polido, borracha, vidro) para pré-render rápido
- [x] Texturas de imagem (biblioteca na peça, mapeamento triplanar) — alpha.5
- [x] Atalhos de câmera: recentralizar, ajustar, vistas padrão, alinhar ao plano

## alpha.3 — Sketch interativo ✔ (base)

- [x] Abrir o sketch **alinha a vista ao plano** (ortogonal); barra própria do sketch (desenho + restrições), destacável
- [x] Ferramentas sem parâmetros obrigatórios: linha/**polilinha**, retângulo (2 cantos / centro), círculo (centro / 3 pontos), arco (3 pontos / centro), rasgo, polígono, ponto
- [x] **Solver de restrições** próprio (Levenberg–Marquardt) com graus de liberdade e conflitos
- [x] **Cotagem inteligente** com restrições integradas (comprimento, distância, horizontal/vertical, raio, diâmetro, ângulo, ponto-linha); valor aceita expressão
- [x] Restrições: coincidente, horizontal, vertical, paralela, perpendicular, tangente, igual, concêntrica, ponto médio, fixa, simétrica
- [x] **Snaps** (pontos, origem, grade), **restrições automáticas** (H/V, coincidência), **cotas automáticas** opcionais
- [x] Grade adaptativa ao zoom; **sombra** nos perfis fechados (face real do kernel)
- [x] Cores de estado: livre (azul), totalmente restrito (branco/preto), conflito (vermelho)
- [x] Dica breve no ícone e **descrição avançada após 5 s**
- [x] Filete, aparar e estender no sketch; projetar referências no plano do sketch — alpha.5
- [ ] Testar o desenho por mouse com o usuário (só verificado por testes e capturas) — **depende de você**

## alpha.4 — Organização, seleção, ambientes de comando ✔ (base)

- [x] Tela inicial (novo/abrir projeto, nova peça, exemplo, recentes) → gerenciador de projetos
- [x] Ícones no estilo da referência e ícone em cada opção/variação (término, operação, furo, arestas, planos, cotas)
- [x] Barra em blocos com nome do grupo; famílias com setinha ▾; grupos destacáveis/encaixáveis
- [x] Dica breve + descrição avançada após 5 s
- [x] Estilos de navegação (SolidCadCraft, SolidWorks, Fusion, CATIA, Blender, Onshape)
- [x] Seleção de faces, arestas, vértices e planos na viewport + filtro de seleção; sketch no plano/face clicado
- [x] Sketch com menus: Criação, Transformar (mover, copiar, espelhar, offset, filete), Cotagem (6 tipos), Restrições, Ferramentas (checar contornos, construção, grade, snaps, auto)
- [x] Painel do sketch limpo: restrições do elemento clicado; símbolos discretos no desenho
- [x] Extrusão/recorte até plano (inclusive inclinado), plano médio; plano por ângulo; filete/chanfro em arestas escolhidas ou todas
- [x] Todo comando abre como ambiente (Concluir/Cancelar); editar por duplo clique ou botão direito
- [x] Hierarquia: linha do tempo por coletor; pastas de geometria livre; referências externas separadas
- [x] Cubo de vistas + seletor (Espaço) + vistas salvas como câmeras
- [x] Furo por norma (ISO grossa/fina, UNC, UNF, BSP, NPT, trapezoidal) com rosca cosmética ou **modelada** (booleana)
- [x] Correção: arestas de costura e profundidade das arestas
- [x] Português / inglês
- [x] Personalização de comandos nos grupos (escolher, reordenar, criar grupos)
- [x] Automação de desenhos (scripts e macros)
- [x] Pontos, linhas e curvas 3D nas pastas de geometria
- [x] Referências externas reais (publicações entre peças)
- [x] Ícone do app vetorizado a partir da arte do usuário (`tools/vetorizar` → `assets/app/fenix.svg`, refinável no VectorCraft)

## alpha.5a — Sketch smart e seletor de vistas

- [x] **Cotagem smart**: clicar no elemento não cria a cota na hora; a cota fica em prévia seguindo o mouse e o segundo clique confirma tipo (horizontal / vertical / alinhada pela posição do mouse) e posição
- [x] Dois elementos clicados → cota entre os dois (distância, ângulo entre linhas, centro-centro)
- [x] Botão direito durante a cotagem → menu com as cotas possíveis (inclusive ângulo) e restrições aplicáveis
- [x] Posição do texto da cota salva no sketch (arrastável com Selecionar)
- [x] Botão direito em elemento(s) selecionado(s) → **menu rápido de restrições** aplicáveis; remover o grupo separado de Restrições da barra
- [x] Botão direito no vazio mostra as ferramentas de esboço; grupos flutuantes empilhados sem se sobrepor
- [x] **Seletor de vistas (Espaço)** como caixa central transparente: cubo 3D translúcido grande (faces, arestas e cantos clicáveis; arrastar gira), vistas padrão + dimétrica/trimétrica, orto/perspectiva, ajustar, normal, girar 90° e câmeras salvas
- [ ] Testar cotagem smart e menu rápido com o mouse (verificado por testes e capturas) — **depende de você**

## alpha.5b — Geometria de referência e correções de fluxo

- [x] Planos: deslocado, paralelo por ponto, ângulo/normal, 3 pontos, 2 linhas, ponto+linha, curva planar, normal à curva, tangente à superfície, equação, médio
- [x] Pontos: coordenadas (Origem ou plano), na curva (distância/proporção, início/meio/fim), no plano, na superfície, centro, tangente, entre
- [x] Linhas e curvas: ponto-ponto, ponto-direção, ângulo/normal/tangente à curva, normal à superfície, polilinha, círculo/arco, círculo 3P, spline, hélice, espiral, cônica, conexão G0/G1/G2, canto, spine
- [x] Entradas aceitam a seleção da viewport (vértices, arestas, faces, planos, pontos e curvas); erros aparecem no elemento
- [x] Filete/chanfro: pré-selecionar arestas **ou** abrir o comando vazio e clicar nas arestas (clicar de novo tira)
- [x] Clique na peça só marca o que foi clicado; duplo clique na face abre o comando que a criou
- [x] Apagar corpo (coletor) com seus comandos; sempre resta um coletor
- [x] Superfícies como geometria de referência (extrair face, offset, preencher)

## alpha.6 — Features por família, atalhos, câmera e carregamento

- [x] Rosca modelada aparece no 3D (sentido do material e início abaixo do rebaixo); filete "todas" ignora arestas de rosca
- [x] Tela de carregamento com logo e barra; tela inicial com fundo em degradê, brilho e grade; logo PNG voltou a aparecer dentro do app
- [x] Atalhos configuráveis com predefinições SolidWorks, CATIA, FreeCAD, Fusion 360, Blender, Onshape e Inventor (teclado + mouse); navegação FreeCAD, Inventor e Rhino
- [x] Câmera animada (vistas, ajustar, normal, câmeras salvas, alinhar sketch) e reenquadramento suave ao abrir comandos
- [x] **Sólidos**: extrusão com ângulo de saída e parede fina; revolução fina; Ranhura (groove); Nervura e Rasgo por varredura (orientação acompanha/fixa/Frenet); Multi-seções e Multi-seções removida (regrada/fechada); primitivas (caixa, cilindro, esfera, cone, toro, cunha, prisma); Reforço
- [x] **Acabamento**: filete variável; chanfro por 2 distâncias e por distância + ângulo; Casca; Ângulo de saída; Espessura de faces; Rosca externa/interna em face cilíndrica
- [x] **Padrões e transformações**: padrão linear (1 ou 2 direções), circular, por pontos; espelho; escala — de comandos escolhidos ou do corpo inteiro
- [x] Barra de comandos quebra em linhas quando não cabe
- [x] Filete face-face e tritangente (o OCCT não tem operação direta; estudar via superfícies)
- [x] Reforço "até o corpo" (hoje usa profundidade)
- [x] Rosca: clicar de novo numa face para tirar; marcar faces por identidade além do ponto

## Próximos pedidos (prioridade)

- [x] **Pastas de geometria como gerenciador**: subpastas, mover (menu ou arrastar), copiar/recortar/colar/duplicar (Ctrl+C/X/V/D), criar qualquer geometria dentro da pasta — superfícies quando existirem
- [x] **Fast 3D**: seta arrastável e cota editável na viewport para extrusão/recorte, plano deslocado, altura das primitivas e reforço (valor ligado a parâmetro fica travado)
- [x] Fast 3D para ângulo da revolução, raio do filete e espessura da casca
- [x] Formas: Pad/Pocket (já: extrusão/recorte), Shaft/Groove (revolução que remove), Rib e Slot (varredura por spine), Multi-seções (loft) e remoção por multi-seções, Stiffener, Thin solids (ver alpha.6)
- [x] Acabamento: filete face-face, tritangente e variável; draft; shell; thickness; thread/tap em eixos (ver alpha.6)
- [x] Padrões e transformações: retangular, circular, por pontos, espelhar, escala (ver alpha.6)

## alpha.5 — Referências do modelo

- [x] Seleção de faces, arestas e vértices na viewport (com realce)
- [x] Sketch em **face plana** (e em superfície via plano tangente)
- [x] Filete/chanfro em arestas selecionadas
- [x] **Extrair geometria** (vértice, aresta, face, elemento de sketch) como referência ou como **parâmetro global**
- [x] Booleanas entre coletores (unir, subtrair, interseção)
- [x] Medição

## alpha.6 — Geometria 3D de referência e referências externas

- [x] Ponto 3D, linha 3D, círculo 3D, curva 3D (spline) e conectores entre linhas 3D
- [x] Planos por todos os modos de [modos-de-criacao.md](specs/modos-de-criacao.md)
- [x] **Publicações** com nome único e uso em outras peças (ADR 0006)
- [x] No sketch: usar parâmetro global ou referência externa publicada; criar referências internas de elementos da própria peça

## 0.1.0-alpha.5 — release ✔

Fecha todos os itens acima. Destaques que ainda não constavam:

- [x] Modos extras de geometria: interseção curva×plano e curva×curva, projeção em superfície/curva, ponto deslocado, interseção de planos, bissetriz, eixo de face, plano por ponto+normal, plano médio, curva projetada, spline por polígono de controle, curva extraída
- [x] Superfícies de referência: extraída, deslocada, preenchimento, extrudada, revolução (translúcidas, selecionáveis, usáveis como entrada)
- [x] Publicações (📤) de elementos e parâmetros, importação por link (ativo/congelado/quebrado, atualizar), projeção no sketch
- [x] Booleanas entre corpos; extrusão "até o próximo"; reforço "até o corpo"; filete face-face e completo (tritangente)
- [x] Medição (distâncias, ângulos, comprimento, raio, área) com "→ parâmetro" e extração de ponto/curva/superfície
- [x] Português/inglês; personalização dos grupos; macros; texturas
- [ ] Aguardando teste do usuário com o mouse: cotagem smart, menu rápido, arrastar na árvore, setas 3D

## 0.1.0-alpha.6 — Bancada de Superfícies, términos unificados e recálculo incremental

- [x] Recálculo incremental: só refaz a partir do primeiro comando que mudou; malhas e sombras de sketch em cache; referências memorizadas por cálculo
- [x] Términos unificados (cego, passante, até o próximo, até plano, **até a superfície/face**) em extrusão, recorte, furo e rosca (rosca: comprimento, face inteira, até plano/superfície)
- [x] Abas de bancada: Modelagem Sólida e Superfícies sobre o mesmo modelo
  - comuns: Sketch, Referências (wireframe), Volume (superfície ↔ sólido), Análise, Organização, Seleção, Vista
  - só Sólidos: Sólidos, Acabamento, Padrões · só Superfícies: Superfícies, Operações
- [x] Wireframe: curva de interseção entre superfícies/planos, curva projetada na superfície, contorno (borda livre)
- [x] Superfícies: esfera, cilindro, varredura explícita e circular (tubo) com orientação, multi-seções (guia opcional, regrada, fechada), concordância G0/G1/G2 entre bordas com superfícies de apoio
- [x] Operações: juntar, curar, dividir, aparar, multi-extração, desmontar (uma face), desaparar, extrapolar; transformações (mover, girar, simetria, escala) de pontos, curvas, planos e superfícies
- [x] Volume: dividir corpo por superfície, engrossar (Thick Surface/Thicken), fechar (Close Surface/Stitch), costurar no corpo (Sew), preencher fronteiras (Boundary Fill, com células excluídas), malha STL/OBJ → B-rep
- [x] Análise: zebra e mapa de curvatura no shader
- [x] Manipulador 3D: setas por eixo, quadrados planares, arco de rotação e alça de escala (ponto por coordenadas e transformações)
- [x] 38 ícones novos no estilo da modelagem sólida; traduções
- [ ] Varredura cônica/adaptativa e várias guias; G2 exato na concordância (hoje: curvatura nula na borda)
- [ ] Formas T-Spline (subdivisão) — ainda não há kernel para isso
- [ ] Aguardando teste do usuário com o mouse: manipulador, bancadas, análise

## Sempre

- [x] Releases locais com `scripts/release.ps1` quando fizer sentido
