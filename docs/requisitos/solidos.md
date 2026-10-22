# Requisitos — Modelagem Sólida

Toda feature: entradas explícitas, aceita entradas por link (RF-NUC-009), parâmetros por expressão
(RF-PAR-002) e pré-visualização em tempo real.

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-SOL-001 | **Extrusão** (adicionar, remover, interseção, novo corpo) com término: cego, simétrico, dois sentidos, até o próximo, até face/vértice/superfície, passante, offset de face. | M | 1 | rascunho |
| RF-SOL-002 | Ângulo de saída e paredes finas (espessura) opcionais na extrusão e na revolução. | M | 1 | rascunho |
| RF-SOL-003 | **Revolução** (adicionar/remover) com ângulo, simétrica e dois sentidos. | M | 1 | rascunho |
| RF-SOL-004 | **Recorte** (cutout): remoção de material a partir de sketch, com as mesmas condições de término da extrusão e opção de recorte normal a superfície. | M | 1 | rascunho |
| RF-SOL-005 | **Ângulo de saída** (draft) em faces, por plano neutro ou linha de partição. | M | 1 | rascunho |
| RF-SOL-006 | **Nervura por caminho** (rib path): nervura a partir de curva aberta, estendendo até as paredes, com espessura, sentido e ângulo de saída. | M | 1 | rascunho |
| RF-SOL-007 | **Furo**: simples, escareado, rebaixado, cônico; término cego, até o próximo, passante; posicionado por pontos de sketch. | M | 1 | rascunho |
| RF-SOL-008 | **Furo inteligente**: escolhe norma e tamanho do fixador (biblioteca) e gera automaticamente diâmetros de passagem (ajustado/normal/folgado), rebaixo/escareado e rosca. | M | 1 | rascunho |
| RF-SOL-009 | **Furo inteligente ligado**: um furo pode ser ligado a furos de outras peças e ao fixador da montagem; trocar a especificação propaga para todos. | M | 2 | rascunho |
| RF-SOL-010 | **Rosca paramétrica** em faces cilíndricas (interna e externa): norma, tamanho, passo, sentido, classe, comprimento. | M | 1 | rascunho |
| RF-SOL-011 | Representação da rosca: **cosmética** (padrão, leve) ou **modelada** (geometria helicoidal real, para impressão 3D/visualização). | M | 1 | rascunho |
| RF-SOL-012 | Rosca aparece corretamente no desenho (representação normalizada) e na BOM (especificação). | M | 2 | rascunho |
| RF-SOL-013 | **Filete** de raio constante em arestas, faces e cadeias tangentes. | M | 1 | rascunho |
| RF-SOL-014 | **Chanfro** (distância, distância-distância, distância-ângulo). | M | 1 | rascunho |
| RF-SOL-015 | **Casca** (shell) com faces removidas e espessura. | M | 2 | rascunho |
| RF-SOL-016 | **Padrão** linear, circular e por curva/sketch, de features ou corpos; espelhar. | M | 2 | rascunho |
| RF-SOL-017 | Booleanas entre corpos: unir, subtrair, interseção. | M | 2 | rascunho |
| RF-SOL-018 | Varredura (sweep) de sólido. | S | 2 | rascunho |
| RF-SOL-019 | Filete de raio variável. | C | 3 | rascunho |
| RF-SOL-020 | Modelagem direta: mover/deslocar/excluir face, gravada como feature na timeline. | S | 3 | rascunho |
| RF-SOL-021 | Usar superfícies (do módulo de superfícies) para cortar, substituir faces ou espessar em sólido. | M | 2 | rascunho |
