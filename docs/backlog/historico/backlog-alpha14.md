# Backlog de melhorias (pedido de 23/10/2026, após alpha.13)

Status: [ ] a fazer · [~] parcial · [x] feito

## A. Geral: 3D, seleção, árvore, interface
- [x] A1 Planos ocultos continuam selecionáveis no 3D (todos os módulos)
- [x] A2 Superfícies selecionáveis no 3D, com faces, arestas e vértices
- [x] A3 Duplo clique em qualquer elemento no 3D abre a edição
- [x] A4 Orbitar com centro definido pelo usuário (clique define o centro)
- [x] A5 Roda do mouse sobre campos numéricos aumenta/diminui o valor
- [x] A6 Roda do mouse rola a barra de comandos e a árvore
- [x] A7 Shift/Ctrl/Alt na árvore (multisseleção)
- [x] A8 Menu de seleção: caixa janela/cruzada (dentro, parcialmente), com Ctrl
- [x] A9 Recurso em edição: entradas escolhidas brilham e têm rótulo 3D ("Perfil", "Guia"…)
- [x] A10 Setas 3D de arraste também em superfícies e chapa metálica
- [x] A11 Aparência: tabela de cores com nome, cores do usuário, copiar/colar aparência, aplicar a vários
- [x] A12 Tema dos menus com faixas de cor por grupo; doca com realce do encaixe ao arrastar; botão fixar/soltar
- [x] A13 Painel lateral do sketch muito largo
- [x] A14 Aviso "crie ou abra uma montagem" aparecendo repetido
- [x] A15 Aparência de linhas/curvas também nos subelementos do sketch

## B. Sketch e wireframe
- [x] B1 Curvas e pontos do sketch selecionáveis no 3D (subelementos) e usáveis em superfícies/referências
- [x] B2 Converter pega elementos de outro sketch no 3D
- [x] B3 Spline e chanfro (bevel) no sketch
- [x] B4 Polilinhas/splines/linhas com subelementos selecionáveis (pontos, segmentos)
- [x] B5 Wireframe: spline e conector de curvas G0–G3 no menu
- [x] B6 Canto entre linha e spline
- [x] B7 Repetição de pontos em linha/curva
- [x] B8 Extrair unificado (simples = vários elementos; múltiplo = um só), propagação ponto/tangente/curvatura, publicável

## C. Superfícies
- [x] C1 Conectar superfícies (blend superfície–superfície)
- [x] C2 Varredura: perfil explícito (1/2 guias, spine), linear, circular e cônica implícitas, suavização, soluções
- [x] C3 Superfície nova consome (oculta) as entradas
- [x] C4 Filetes de superfície (entre superfícies, aresta, variável)
- [x] C5 Offset: dois lados, ligação dos elementos (arredondado/suave/plano/automático)
- [x] C6 Extrapolar vários elementos (arestas) de uma vez
- [x] C7 Curva paralela: offset por lado, conjunto de elementos
- [x] C8 Dividir com planos, superfícies e wireframe

## D. Chapa metálica
- [x] D1 Parâmetros: material, método de compensação, tabelas (raio mínimo, K, joggle) — tabela de materiais pesquisada
- [x] D2 Parede base de sketch com offset/lado do material; parede de superfície com setas
- [x] D3 Aba complexa (recurso base, plano/superfície, compasso 3D, direções, limite EOP, sides 1/2, raio por side)
- [x] D4 Abas por superfície (curvas) e interseção entre abas
- [x] D5 Joggle: ângulo, por superfície, compensação
- [x] D6 Aba: tipo de alívio, lado do material; bainha aberta/fechada/gota; aba por sketch
- [x] D7 Recortes (normal, com saída, offset), furo, recorte flangeado
- [x] D8 Estampos: nervura (bead), circular, por superfície, por curva, reforço, punção e matriz
- [x] D9 Alívios de canto (redondo, quadrado, gota)
- [x] D10 Padrões na chapa
- [x] D11 Desdobrar mostrando OML/linhas de dobra
- [x] D12 Setas 3D na chapa; furo flangeado com seleção como furo e tabela cadastrável

## E. Estrutural
- [x] E1 Menu de operações de canto por par de membros (membro 1 / membro 2)
- [x] E2 Membro com várias linhas gera elementos separados ao concluir
- [x] E3 Menu de wireframe no estrutural
- [x] E4 Propriedades do projeto: comprimento máximo da matéria-prima → alerta de emenda (também na lista de corte)

## F. Furos e filetes
- [x] F1 Interface do furo sem redundância; rosca cosmética/modelada com rebaixo/escareado
- [x] F2 Furo múltiplo esperto: pega pontos de sketch/extração múltipla, projeta na superfície
- [x] F3 Filetes/chanfros: propagação tangente / nenhuma / mínima

## G. Montagem
- [x] G1 Restrições rápidas: selecionar elementos de duas peças → menu (inclusive botão direito) com as restrições possíveis
- [x] G2 Painel de edição dinâmico como nos outros módulos; restrição como recurso (pede o que falta)
- [x] G3 Setas 3D com rotação
- [x] G4 Parar na colisão de verdade; arraste travado pelas restrições; edição de restrições; inverter sem bug
- [x] G5 Restrições de engrenagem, pinhão, polia
- [x] G6 Salvar montagem como componente e inserir montagens em montagens (vários níveis)
- [x] G7 Árvore: totalmente definido × solto; vários níveis com rolagem
- [x] G8 Subárvore da peça dentro da montagem; duplo clique = objeto de trabalho (pai em destaque); modo edição × visualização
- [x] G9 Recursos entre peças (furo, booleana, chapa, aba/ranhura) com link
- [x] G10 Publicações automáticas entre peças, filtro de referências externas, instâncias, sem referência circular; modo restrição × relacional

## H. PMI / FTI
- [x] H1 Notas e GD&T mais personalizáveis
- [x] H2 Legenda (bloco de título) no 3D com campos da peça; balões automáticos no 3D
- [x] H3 Capturas completas: câmera, zoom, anotações, visibilidade de corpos/recursos/peças (também na montagem), separadas das anotações; pastas de anotações
- [x] H4 Selecionar a anotação realça o elemento referenciado
- [x] H5 Símbolos de solda conforme ISO 2553 (2D e 3D)

## I. Listas de peças
- [x] I1 Lista de peças/cortes por nível, dinâmica, mantendo posições pelas revisões; usada no desenho

## J. Sólidos com wireframe e lógica geométrica (pedido de 24/10/2026)
- [x] J1 Recursos sólidos (extrusão, recorte, revolução, varredura, loft, nervura…) aceitam elementos do wireframe além de sketches: curvas, linhas, polilinhas, splines, extrações, curvas de outros sketches, arestas
- [x] J2 Se faltar direção/plano/eixo (ex.: curva 3D sem plano), o comando pede a referência em vez de falhar
- [x] J3 Tirar as regras de "só faces" / "só pontos": cada campo aceita o que geometricamente faz sentido (aresta reta = eixo/direção, face plana = plano/direção normal, curva = perfil/guia, vértice/ponto = ponto, cilindro = eixo)

## K. Correções e pedidos de 24/10/2026 (2ª rodada)
- [x] K1 Chapa: dobrar/desdobrar errado (abas somem no planificado)
- [x] K2 Spline em extrusão/chapa sai facetada (sombra do contorno mostra os segmentos)
- [x] K3 Filete na chapa que dobra e planifica junto
- [x] K4 Recortes e operações valem no planificado e no dobrado (um transmite para o outro)
- [x] K5 Perfil aberto não é erro: recurso fino (espessura para um lado/dois lados) em extrusão e recorte
- [x] K6 Estrutural: polilinha bagunça a orientação do membro (cada trecho deve ser um membro)
- [x] K7 Atalho H (ocultar) não funciona nas pastas
- [x] K8 GD&T desenhado como símbolo (quadro com casas, datum com triângulo), não texto
- [x] K9 PMI com cara profissional e mais personalização (fonte, cores, setas, linhas)

## L. Pedido de 25/10/2026: montagem, desenho 2D e anotações
- [x] L1 Montagem: árvore e painel de edição na filosofia dos sólidos (Concluir/Cancelar/Excluir, abas, 1 clique realça, 2 cliques editam)
- [x] L2 Desenho: GD&T dinâmico (quadro em casas) e anotações ancoradas em elementos da vista (associativas)
- [x] L3 Desenho: árvore por vistas (vista → anotações dela), caixa 2D da vista detecta o que pertence a ela, botão direito com opções, propriedades hierárquicas
- [x] L4 Desenho de montagem: componentes visíveis ou estado de exibição por vista
- [x] L5 Cotas soltas: arrastar livremente, alças (texto, linha de cota, setas), várias setas
- [x] L6 Estilo de renderização da vista e vista pela câmera atual (montagem)
- [x] L7 Elementos 2D: 1 clique e arrastar move, 2 cliques editam, botão direito com opções
- [x] L8 Cor e espessura por elemento no 2D
- [x] L9 Molduras das anotações: nenhuma, retângulo, círculo, cápsula, triângulo, quadrado, bandeira dir./esq., losango, círculo/retângulo pontilhado, tamanho fixo; espessura, tipo de linha, cor
- [x] L10 Propriedades das anotações (3D e 2D): cor, negrito, tipo de linha, justificação, ponto de ancoragem, espaçamento de linhas, ângulo, espelhar, girar com a vista, fundo (nenhum / preenchido / preenchido recortando a geometria), deslocamento, tamanho, aplicar escala, exibição (mostrar, chamada), coordenadas X/Y, chamada (seta, espessura, estilo)
- [x] L11 Campos de seleção múltipla: botão "sacola" com a contagem → lista de elementos (cada clique acrescenta, remover); Ctrl+clique soma/tira da seleção

## M. Pedido de 25/10/2026 (3ª rodada): sketch, chapa, montagem, visual
- [x] M1 Spline como Bézier: botão direito → mostrar pontos de inflexão, tangenciar em pontos, adicionar e remover pontos
- [x] M2 Sketch com spline não é detectado como perfil para criar recursos
- [x] M3 Chapa: aba curva unida à aba reta, com raio de dobra e planificação corretos
- [x] M4 Chapa: abas vizinhas sem recuo manual (flag "recuo mínimo para aba vizinha", 0,0001)
- [x] M5 Separador decimal pelo idioma (vírgula em PT, ponto em EN)
- [x] M6 Planificação perdendo um raio
- [x] M7 Chapa: raios e chanfros de canto, furos e rasgos circulares
- [x] M8 Aba por superfície (estilo Surfacic Flange): base, suporte exato/aproximado/ângulo, raio, EOP, lado do material, planificação
- [x] M9 Montagem: anéis de rotação coloridos por eixo e o 3º eixo funcionando
- [x] M10 Ícones dos componentes na árvore da montagem no tema (não emoji)
- [x] M11 Estética uniforme: contraste pelo tema (claro/escuro) em todos os comandos e abas
- [x] M12 Subárvore da peça na montagem igual à árvore do sólido (expandir corpos/recursos)
- [x] M13 Edição na montagem dinâmica como nos outros módulos (um painel só)
- [x] M14 Marca (brand) do software a partir do ícone; tela de carregamento e inicial animadas
- [x] M15 Temas: 5 temas prontos e temas do usuário (extensões)
- [x] M16 Peças de exemplo com prévia na tela inicial
- [x] M17 Recentes (peças e montagens) na tela inicial
- [x] M18 Painéis de comandos com muitos itens divididos em abas (menos horizontal)
- [x] M19 Ícones e dicas em todos os subparâmetros dos comandos
- [x] M20 Mais animações na interface
- [x] M21 Renderização mais real: cor inicial e arestas sem atravessar faces (raios visíveis como na referência)

## Fechamento alpha.26
- [x] Aba por superfície: modos Exato, Aproximação (plano tangente, dobra reta) e Ângulo (ângulo constante)
- [x] Aba curva: lado do material (dobra por fora, material por fora, material por dentro)
- [x] Campos de arrastar e controles deslizantes com vírgula decimal em PT; coordenadas como (x; y; z)
- [x] Contraste no tema claro: abas, ícones de cabeçalho, selos e cores de estado pelo tema
- [x] Ícones nas caixas de marcar dos painéis
- [x] Rótulo único para a aresta da aba curva na viewport
