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

## alpha.3 — Sketch interativo

- [ ] Abrir o sketch **alinha a vista ao plano**; barra de ferramentas própria do sketch (editável)
- [ ] Ferramentas sem parâmetros obrigatórios: ponto, linha, **polilinha**, retângulo (2 cantos / centro), círculo (centro / 3 pontos), arco (3 pontos / centro / tangente), rasgo, polígono, filete de sketch
- [ ] **Solver de restrições** (geometria livre que vai sendo restringida)
- [ ] **Ferramenta de cotagem** com restrições integradas (distância, horizontal/vertical, raio, diâmetro, ângulo); valor aceita expressão e parâmetros globais
- [ ] Restrições: coincidente, horizontal, vertical, paralela, perpendicular, tangente, igual, concêntrica, ponto médio, simétrica, fixa
- [ ] **Snaps** e **restrições automáticas** ligáveis; **cotas automáticas** opcionais
- [ ] Grade liga/desliga, passo configurável
- [ ] **Sombra** (preenchimento) nos perfis fechados
- [ ] Estado do sketch: sub-restrito / totalmente restrito / conflito

## alpha.4 — Seleção na viewport e referências do modelo

- [ ] Seleção de faces, arestas e vértices na viewport (com realce)
- [ ] Sketch em **face plana** (e em superfície via plano tangente)
- [ ] Filete/chanfro em arestas selecionadas
- [ ] **Extrair geometria** (vértice, aresta, face, elemento de sketch) como referência ou como **parâmetro global**
- [ ] Booleanas entre coletores (unir, subtrair, interseção)
- [ ] Medição

## alpha.5 — Geometria 3D de referência e referências externas

- [ ] Ponto 3D, linha 3D, círculo 3D, curva 3D (spline) e conectores entre linhas 3D
- [ ] Planos por todos os modos de [modos-de-criacao.md](specs/modos-de-criacao.md)
- [ ] **Publicações** com nome único e uso em outras peças (ADR 0006)
- [ ] No sketch: usar parâmetro global ou referência externa publicada; criar referências internas de elementos da própria peça

## Sempre

- [ ] Releases locais com `scripts/release.ps1` quando fizer sentido
