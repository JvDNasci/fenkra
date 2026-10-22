# Requisitos — Chapas Metálicas

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-CHP-001 | Parâmetros de chapa por peça/corpo: material, espessura, raio de dobra, fator K (ou tabela de dobra/dedução), alívio padrão — vindos da biblioteca (RF-BIB-017) ou de parâmetros/links. | M | 2 | rascunho |
| RF-CHP-002 | **Flange base** a partir de sketch fechado ou aberto (perfil com espessura). | M | 2 | rascunho |
| RF-CHP-003 | **Flange de aresta**: em uma ou várias arestas, com comprimento, ângulo, posição (material dentro/fora/dobra fora) e recuos. | M | 2 | rascunho |
| RF-CHP-004 | Flange de aresta com **perfil personalizado** (sketch). | M | 2 | rascunho |
| RF-CHP-005 | **Flange contínua (web flange) sobre superfícies**: gerar chapa seguindo superfícies ou faces de uma geometria mestre, com dobras nas arestas compartilhadas. | M | 3 | rascunho |
| RF-CHP-006 | **Flange de contorno**: perfil aberto varrido ao longo de arestas. | S | 3 | rascunho |
| RF-CHP-007 | Dobra por linha de sketch (dobrar chapa plana). | M | 2 | rascunho |
| RF-CHP-008 | Tratamento de cantos: alívio de canto (redondo, quadrado, rasgo), fechamento de canto com folga. | M | 2 | rascunho |
| RF-CHP-009 | Bainha (hem), jog (degrau), estampos simples (lanceta, persiana, nervura de reforço) como features. | S | 3 | rascunho |
| RF-CHP-010 | Recorte normal à chapa (o corte atravessa perpendicular às faces mesmo em regiões dobradas). | M | 2 | rascunho |
| RF-CHP-011 | Converter sólido de espessura constante em chapa (reconhecendo dobras). | S | 3 | rascunho |
| RF-CHP-012 | **Planificação** associativa com linhas de dobra, sentido e ângulo; exportação DXF para corte. | M | 2 | rascunho |
| RF-CHP-013 | Dobrar/desdobrar parcial para criar features que atravessam dobras. | S | 3 | rascunho |
| RF-CHP-014 | Toda feature de chapa aceita referências de geometria mestre por link; alterar o mestre atualiza a chapa e a planificação. | M | 2 | rascunho |
| RF-CHP-015 | Verificação de fabricação: dobras muito próximas, furos próximos a dobras, raio menor que o mínimo do material. | S | 3 | rascunho |
| RF-CHP-016 | Propriedades automáticas: dimensões do blank, área, massa, número de dobras. | M | 2 | rascunho |
