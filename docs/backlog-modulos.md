# Backlog dos módulos (pedido de 2026-10-23)

Ordem de execução: cada módulo entra como bancada (aba) sobre o mesmo projeto. A Montagem vem
primeiro porque Cinemática, Exibição/Composer e Detalhamento dependem dela.

Regra geral para todos: toda feature nova tem ícone próprio, tooltip curto + explicação em PT e EN,
painel com parâmetros que aceitam expressões, e um teste automático básico.

## M1 — Montagem

- [ ] Documento de montagem (`.sccasm`) no projeto; árvore com componentes, grupos de organização
      vazios, subconjuntos e peças
- [ ] Inserir peça existente; criar peça nova no contexto; substituir componente mantendo vínculos
- [ ] Reordenar a árvore só muda a visualização; numeração automática (índices sequenciais)
- [ ] Carregamento seletivo (só o visível); botão direito: modo editável × visualização
- [ ] Menu superior com modos de visualização da montagem
- [ ] Restrições: coincidência (eixos, pontos, faces coplanares; direta/invertida), contato,
      distância (±, fórmula), ângulo (explícito, paralelo, perpendicular), fixação, fixação conjunta,
      restrição automática, conversão de tipo sem apagar, repetição inteligente (parafusos em padrão)
- [ ] Movimentação manual (translação/rotação por eixos/planos) respeitando ou não as restrições;
      alinhamento instantâneo sem histórico; movimento inteligente (cria restrição); parada por colisão
- [ ] Interferências, folgas e zonas de colisão

## M2 — Cinemática

- [ ] Juntas: revolução, prismática, cilíndrica, parafuso, esférica, planar, rígida, ponto-em-curva,
      came/seguidor (curva-curva), engrenagem e cabo (razão)
- [ ] Mecanismo automático a partir das restrições; comandos por cursor; quadros de movimento
- [ ] Análise de graus de liberdade (comandos × GDL restantes)
- [ ] Simulação por leis de tempo; velocidade e aceleração

## M3 — Chapa metálica

- [ ] Parâmetros globais: espessura, raio de dobra, fator K / tabela de dobra
- [ ] Parede base, aba de aresta (altura, ângulo, limites laterais), flange com raio e alinhamento
      interno/externo, bainha (circular/esmagada), extrusão de chapa
- [ ] Abas por superfície de base, limites por objetos de referência; controle hierárquico de
      flange e alma
- [ ] Dobra cega, cônica, por linha; joggle (desvio em Z); cortes flangeados; furos estampados;
      estampos (veneziana, rebaixo, reforço); alívios de canto (inclusive por norma aeronáutica)
- [ ] Dobrar/desdobrar sincronizados; planificação com linhas e ângulos de dobra; aviso quando o
      planificado não puder ser calculado exatamente

## M4 — Estrutural

- [ ] Membros estruturais por linha guia: perfil, ponto de ancoragem, orientação, offsets —
      totalmente associativos ao esqueleto
- [ ] Biblioteca de famílias de perfis (cadastro) com pontos de origem e regras de criação a partir
      do arame 3D
- [ ] Meia-esquadria, aparar, recorte de encaixe com folga de solda
- [ ] Placas de extremidade, reforços triangulares/retangulares com chanfro de alívio

## M5 — Exibição / Composer

- [ ] Estados de exibição salvos (visível/invisível, cores), vista explodida
- [ ] Animações simples com linha do tempo e estados dinâmicos (estilo Composer)
- [ ] Coletores de exibição usados pelo detalhamento 2D/3D

## M6 — Detalhamento 3D (PMI)

- [ ] Árvore de PMI por nível (montagem, submontagem, peça)
- [ ] Cotas, tolerâncias, ajustes, rugosidade, solda e notas no 3D, associativas

## M7 — Detalhamento 2D (módulo separado)

- [ ] Folhas e formatos; vistas padrão, isométrica, corte com hachura, detalhe — associativas
- [ ] Cotas lineares, angulares, radiais e diametrais; tolerâncias; rugosidade; solda; notas
- [ ] Tabelas: furação, dobras, chapas, perfis, BOM; vista planificada com linhas de dobra
- [ ] Esquemas cinemáticos simplificados; normas de projeção e cotagem

## Transversais

- [ ] Configurações: todas as personalizações viáveis e visuais
- [ ] Toda feature com figura e tooltip PT/EN
