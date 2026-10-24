# Backlog de melhorias (pedido de 23/10/2026, após alpha.13)

Status: [ ] a fazer · [~] parcial · [x] feito

## A. Geral: 3D, seleção, árvore, interface
- [x] A1 Planos ocultos continuam selecionáveis no 3D (todos os módulos)
- [x] A2 Superfícies selecionáveis no 3D, com faces, arestas e vértices
- [x] A3 Duplo clique em qualquer elemento no 3D abre a edição
- [x] A4 Orbitar com centro definido pelo usuário (clique define o centro)
- [x] A5 Roda do mouse sobre campos numéricos aumenta/diminui o valor
- [~] A6 Roda do mouse rola a barra de comandos e a árvore
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
- [ ] D1 Parâmetros: material, método de compensação, tabelas (raio mínimo, K, joggle) — tabela de materiais pesquisada
- [ ] D2 Parede base de sketch com offset/lado do material; parede de superfície com setas
- [ ] D3 Aba complexa (recurso base, plano/superfície, compasso 3D, direções, limite EOP, sides 1/2, raio por side)
- [ ] D4 Abas por superfície (curvas) e interseção entre abas
- [ ] D5 Joggle: ângulo, por superfície, compensação
- [ ] D6 Aba: tipo de alívio, lado do material; bainha aberta/fechada/gota; aba por sketch
- [ ] D7 Recortes (normal, com saída, offset), furo, recorte flangeado
- [ ] D8 Estampos: nervura (bead), circular, por superfície, por curva, reforço, punção e matriz
- [ ] D9 Alívios de canto (redondo, quadrado, gota)
- [ ] D10 Padrões na chapa
- [ ] D11 Desdobrar mostrando OML/linhas de dobra
- [ ] D12 Setas 3D na chapa; furo flangeado com seleção como furo e tabela cadastrável

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
- [ ] G1 Restrições rápidas: selecionar elementos de duas peças → menu (inclusive botão direito) com as restrições possíveis
- [ ] G2 Painel de edição dinâmico como nos outros módulos; restrição como recurso (pede o que falta)
- [ ] G3 Setas 3D com rotação
- [ ] G4 Parar na colisão de verdade; arraste travado pelas restrições; edição de restrições; inverter sem bug
- [ ] G5 Restrições de engrenagem, pinhão, polia
- [ ] G6 Salvar montagem como componente e inserir montagens em montagens (vários níveis)
- [ ] G7 Árvore: totalmente definido × solto; vários níveis com rolagem
- [ ] G8 Subárvore da peça dentro da montagem; duplo clique = objeto de trabalho (pai em destaque); modo edição × visualização
- [ ] G9 Recursos entre peças (furo, booleana, chapa, aba/ranhura) com link
- [ ] G10 Publicações automáticas entre peças, filtro de referências externas, instâncias, sem referência circular; modo restrição × relacional

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
