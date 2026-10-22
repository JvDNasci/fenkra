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
- [ ] Português / inglês
- [ ] Personalização de comandos nos grupos (escolher, reordenar, criar grupos)
- [ ] Automação de desenhos (scripts e macros)
- [ ] Pontos, linhas e curvas 3D nas pastas de geometria
- [ ] Referências externas reais (publicações entre peças)
- [ ] Ícone do app a partir da arte do usuário (aguardando assets/app/fenix-ref.png)

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
