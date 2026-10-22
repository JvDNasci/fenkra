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
- [ ] Texturas de imagem (adiado)
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
- [ ] Filete e aparar/estender no sketch
- [ ] Testar o desenho por mouse com o usuário (só verificado por testes e capturas)

## alpha.4 — Inicialização, ícones de opções, seleção e navegação (pedidos de 22/10)

- [ ] **Tela inicial** antes de abrir um modelo: novo/abrir projeto, recentes, exemplo → gerenciador de projetos
- [ ] **Ícones em todas as opções** dos comandos (tipos de término, tipos de furo, operação...): uma variação de criação = um ícone, sem botões de texto com "+"
- [ ] Refazer os ícones no **estilo da referência** (azul em degradê, contorno azul-escuro, tracejado antes/depois, setas)
- [ ] **Estilos de navegação** da câmera (SolidWorks, Fusion, CATIA, Blender, personalizado)
- [ ] **Seleção de faces, arestas e vértices** na viewport + **filtro de seleção** em menu
- [ ] Português / inglês
- [ ] Personalização de comandos nos grupos (escolher, reordenar, criar grupos); menus destacáveis
- [ ] Automação de desenhos (scripts e gravação de macros)
- [ ] Ícone do app vetorizado a partir da arte do usuário (aguardando o arquivo em `assets/app/fenix-ref.png`)

## alpha.5 — Referências do modelo

- [ ] Seleção de faces, arestas e vértices na viewport (com realce)
- [ ] Sketch em **face plana** (e em superfície via plano tangente)
- [ ] Filete/chanfro em arestas selecionadas
- [ ] **Extrair geometria** (vértice, aresta, face, elemento de sketch) como referência ou como **parâmetro global**
- [ ] Booleanas entre coletores (unir, subtrair, interseção)
- [ ] Medição

## alpha.6 — Geometria 3D de referência e referências externas

- [ ] Ponto 3D, linha 3D, círculo 3D, curva 3D (spline) e conectores entre linhas 3D
- [ ] Planos por todos os modos de [modos-de-criacao.md](specs/modos-de-criacao.md)
- [ ] **Publicações** com nome único e uso em outras peças (ADR 0006)
- [ ] No sketch: usar parâmetro global ou referência externa publicada; criar referências internas de elementos da própria peça

## Sempre

- [ ] Releases locais com `scripts/release.ps1` quando fizer sentido
