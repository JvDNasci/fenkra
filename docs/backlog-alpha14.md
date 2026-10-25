# Backlog de melhorias (pedido de 23/10/2026, apÃ³s alpha.13)

Status: [ ] a fazer Â· [~] parcial Â· [x] feito

## A. Geral: 3D, seleÃ§Ã£o, Ã¡rvore, interface
- [x] A1 Planos ocultos continuam selecionÃ¡veis no 3D (todos os mÃ³dulos)
- [x] A2 SuperfÃ­cies selecionÃ¡veis no 3D, com faces, arestas e vÃ©rtices
- [x] A3 Duplo clique em qualquer elemento no 3D abre a ediÃ§Ã£o
- [x] A4 Orbitar com centro definido pelo usuÃ¡rio (clique define o centro)
- [x] A5 Roda do mouse sobre campos numÃ©ricos aumenta/diminui o valor
- [~] A6 Roda do mouse rola a barra de comandos e a Ã¡rvore
- [x] A7 Shift/Ctrl/Alt na Ã¡rvore (multisseleÃ§Ã£o)
- [x] A8 Menu de seleÃ§Ã£o: caixa janela/cruzada (dentro, parcialmente), com Ctrl
- [x] A9 Recurso em ediÃ§Ã£o: entradas escolhidas brilham e tÃªm rÃ³tulo 3D ("Perfil", "Guia"â€¦)
- [x] A10 Setas 3D de arraste tambÃ©m em superfÃ­cies e chapa metÃ¡lica
- [x] A11 AparÃªncia: tabela de cores com nome, cores do usuÃ¡rio, copiar/colar aparÃªncia, aplicar a vÃ¡rios
- [x] A12 Tema dos menus com faixas de cor por grupo; doca com realce do encaixe ao arrastar; botÃ£o fixar/soltar
- [x] A13 Painel lateral do sketch muito largo
- [x] A14 Aviso "crie ou abra uma montagem" aparecendo repetido
- [x] A15 AparÃªncia de linhas/curvas tambÃ©m nos subelementos do sketch

## B. Sketch e wireframe
- [x] B1 Curvas e pontos do sketch selecionÃ¡veis no 3D (subelementos) e usÃ¡veis em superfÃ­cies/referÃªncias
- [x] B2 Converter pega elementos de outro sketch no 3D
- [x] B3 Spline e chanfro (bevel) no sketch
- [x] B4 Polilinhas/splines/linhas com subelementos selecionÃ¡veis (pontos, segmentos)
- [x] B5 Wireframe: spline e conector de curvas G0â€“G3 no menu
- [x] B6 Canto entre linha e spline
- [x] B7 RepetiÃ§Ã£o de pontos em linha/curva
- [x] B8 Extrair unificado (simples = vÃ¡rios elementos; mÃºltiplo = um sÃ³), propagaÃ§Ã£o ponto/tangente/curvatura, publicÃ¡vel

## C. SuperfÃ­cies
- [x] C1 Conectar superfÃ­cies (blend superfÃ­cieâ€“superfÃ­cie)
- [x] C2 Varredura: perfil explÃ­cito (1/2 guias, spine), linear, circular e cÃ´nica implÃ­citas, suavizaÃ§Ã£o, soluÃ§Ãµes
- [x] C3 SuperfÃ­cie nova consome (oculta) as entradas
- [x] C4 Filetes de superfÃ­cie (entre superfÃ­cies, aresta, variÃ¡vel)
- [x] C5 Offset: dois lados, ligaÃ§Ã£o dos elementos (arredondado/suave/plano/automÃ¡tico)
- [x] C6 Extrapolar vÃ¡rios elementos (arestas) de uma vez
- [x] C7 Curva paralela: offset por lado, conjunto de elementos
- [x] C8 Dividir com planos, superfÃ­cies e wireframe

## D. Chapa metÃ¡lica
- [ ] D1 ParÃ¢metros: material, mÃ©todo de compensaÃ§Ã£o, tabelas (raio mÃ­nimo, K, joggle) â€” tabela de materiais pesquisada
- [ ] D2 Parede base de sketch com offset/lado do material; parede de superfÃ­cie com setas
- [ ] D3 Aba complexa (recurso base, plano/superfÃ­cie, compasso 3D, direÃ§Ãµes, limite EOP, sides 1/2, raio por side)
- [ ] D4 Abas por superfÃ­cie (curvas) e interseÃ§Ã£o entre abas
- [ ] D5 Joggle: Ã¢ngulo, por superfÃ­cie, compensaÃ§Ã£o
- [ ] D6 Aba: tipo de alÃ­vio, lado do material; bainha aberta/fechada/gota; aba por sketch
- [ ] D7 Recortes (normal, com saÃ­da, offset), furo, recorte flangeado
- [ ] D8 Estampos: nervura (bead), circular, por superfÃ­cie, por curva, reforÃ§o, punÃ§Ã£o e matriz
- [ ] D9 AlÃ­vios de canto (redondo, quadrado, gota)
- [ ] D10 PadrÃµes na chapa
- [ ] D11 Desdobrar mostrando OML/linhas de dobra
- [ ] D12 Setas 3D na chapa; furo flangeado com seleÃ§Ã£o como furo e tabela cadastrÃ¡vel

## E. Estrutural
- [x] E1 Menu de operaÃ§Ãµes de canto por par de membros (membro 1 / membro 2)
- [x] E2 Membro com vÃ¡rias linhas gera elementos separados ao concluir
- [x] E3 Menu de wireframe no estrutural
- [x] E4 Propriedades do projeto: comprimento mÃ¡ximo da matÃ©ria-prima â†’ alerta de emenda (tambÃ©m na lista de corte)

## F. Furos e filetes
- [x] F1 Interface do furo sem redundÃ¢ncia; rosca cosmÃ©tica/modelada com rebaixo/escareado
- [x] F2 Furo mÃºltiplo esperto: pega pontos de sketch/extraÃ§Ã£o mÃºltipla, projeta na superfÃ­cie
- [x] F3 Filetes/chanfros: propagaÃ§Ã£o tangente / nenhuma / mÃ­nima

## G. Montagem
- [ ] G1 RestriÃ§Ãµes rÃ¡pidas: selecionar elementos de duas peÃ§as â†’ menu (inclusive botÃ£o direito) com as restriÃ§Ãµes possÃ­veis
- [ ] G2 Painel de ediÃ§Ã£o dinÃ¢mico como nos outros mÃ³dulos; restriÃ§Ã£o como recurso (pede o que falta)
- [ ] G3 Setas 3D com rotaÃ§Ã£o
- [ ] G4 Parar na colisÃ£o de verdade; arraste travado pelas restriÃ§Ãµes; ediÃ§Ã£o de restriÃ§Ãµes; inverter sem bug
- [ ] G5 RestriÃ§Ãµes de engrenagem, pinhÃ£o, polia
- [ ] G6 Salvar montagem como componente e inserir montagens em montagens (vÃ¡rios nÃ­veis)
- [ ] G7 Ãrvore: totalmente definido Ã— solto; vÃ¡rios nÃ­veis com rolagem
- [ ] G8 SubÃ¡rvore da peÃ§a dentro da montagem; duplo clique = objeto de trabalho (pai em destaque); modo ediÃ§Ã£o Ã— visualizaÃ§Ã£o
- [ ] G9 Recursos entre peÃ§as (furo, booleana, chapa, aba/ranhura) com link
- [ ] G10 PublicaÃ§Ãµes automÃ¡ticas entre peÃ§as, filtro de referÃªncias externas, instÃ¢ncias, sem referÃªncia circular; modo restriÃ§Ã£o Ã— relacional

## H. PMI / FTI
- [x] H1 Notas e GD&T mais personalizÃ¡veis
- [x] H2 Legenda (bloco de tÃ­tulo) no 3D com campos da peÃ§a; balÃµes automÃ¡ticos no 3D
- [x] H3 Capturas completas: cÃ¢mera, zoom, anotaÃ§Ãµes, visibilidade de corpos/recursos/peÃ§as (tambÃ©m na montagem), separadas das anotaÃ§Ãµes; pastas de anotaÃ§Ãµes
- [x] H4 Selecionar a anotaÃ§Ã£o realÃ§a o elemento referenciado
- [x] H5 SÃ­mbolos de solda conforme ISO 2553 (2D e 3D)

## I. Listas de peÃ§as
- [x] I1 Lista de peÃ§as/cortes por nÃ­vel, dinÃ¢mica, mantendo posiÃ§Ãµes pelas revisÃµes; usada no desenho

## J. SÃ³lidos com wireframe e lÃ³gica geomÃ©trica (pedido de 24/10/2026)
- [x] J1 Recursos sÃ³lidos (extrusÃ£o, recorte, revoluÃ§Ã£o, varredura, loft, nervuraâ€¦) aceitam elementos do wireframe alÃ©m de sketches: curvas, linhas, polilinhas, splines, extraÃ§Ãµes, curvas de outros sketches, arestas
- [x] J2 Se faltar direÃ§Ã£o/plano/eixo (ex.: curva 3D sem plano), o comando pede a referÃªncia em vez de falhar
- [x] J3 Tirar as regras de "sÃ³ faces" / "sÃ³ pontos": cada campo aceita o que geometricamente faz sentido (aresta reta = eixo/direÃ§Ã£o, face plana = plano/direÃ§Ã£o normal, curva = perfil/guia, vÃ©rtice/ponto = ponto, cilindro = eixo)

## K. CorreÃ§Ãµes e pedidos de 24/10/2026 (2Âª rodada)
- [x] K1 Chapa: dobrar/desdobrar errado (abas somem no planificado)
- [x] K2 Spline em extrusÃ£o/chapa sai facetada (sombra do contorno mostra os segmentos)
- [x] K3 Filete na chapa que dobra e planifica junto
- [x] K4 Recortes e operaÃ§Ãµes valem no planificado e no dobrado (um transmite para o outro)
- [x] K5 Perfil aberto nÃ£o Ã© erro: recurso fino (espessura para um lado/dois lados) em extrusÃ£o e recorte
- [x] K6 Estrutural: polilinha bagunÃ§a a orientaÃ§Ã£o do membro (cada trecho deve ser um membro)
- [x] K7 Atalho H (ocultar) nÃ£o funciona nas pastas
- [x] K8 GD&T desenhado como sÃ­mbolo (quadro com casas, datum com triÃ¢ngulo), nÃ£o texto
- [x] K9 PMI com cara profissional e mais personalizaÃ§Ã£o (fonte, cores, setas, linhas)

## L. Pedido de 25/10/2026: montagem, desenho 2D e anotações
- [x] L1 Montagem: árvore e painel de edição na filosofia dos sólidos (Concluir/Cancelar/Excluir, abas, 1 clique realça, 2 cliques editam)
- [ ] L2 Desenho: GD&T dinâmico (quadro em casas) e anotações ancoradas em elementos da vista (associativas)
- [ ] L3 Desenho: árvore por vistas (vista → anotações dela), caixa 2D da vista detecta o que pertence a ela, botão direito com opções, propriedades hierárquicas
- [ ] L4 Desenho de montagem: componentes visíveis ou estado de exibição por vista
- [ ] L5 Cotas soltas: arrastar livremente, alças (texto, linha de cota, setas), várias setas
- [ ] L6 Estilo de renderização da vista e vista pela câmera atual (montagem)
- [ ] L7 Elementos 2D: 1 clique e arrastar move, 2 cliques editam, botão direito com opções
- [ ] L8 Cor e espessura por elemento no 2D
- [x] L9 Molduras das anotações: nenhuma, retângulo, círculo, cápsula, triângulo, quadrado, bandeira dir./esq., losango, círculo/retângulo pontilhado, tamanho fixo; espessura, tipo de linha, cor
- [~] L10 Propriedades das anotações (3D e 2D): cor, negrito, tipo de linha, justificação, ponto de ancoragem, espaçamento de linhas, ângulo, espelhar, girar com a vista, fundo (nenhum / preenchido / preenchido recortando a geometria), deslocamento, tamanho, aplicar escala, exibição (mostrar, chamada), coordenadas X/Y, chamada (seta, espessura, estilo)
- [ ] L11 Campos de seleção múltipla: botão "sacola" com a contagem → lista de elementos (cada clique acrescenta, remover); Ctrl+clique soma/tira da seleção
