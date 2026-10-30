# Backlog dos módulos (pedido de 2026-10-23)

Ordem de execução: cada módulo entra como bancada (aba) sobre o mesmo projeto. A Montagem vem
primeiro porque Cinemática, Exibição/Composer e Detalhamento dependem dela.

Regra geral para todos: toda feature nova tem ícone próprio, tooltip curto + explicação em PT e EN,
painel com parâmetros que aceitam expressões, e um teste automático básico.

## M1 — Montagem

- [x] Documento de montagem (`.fka`) no projeto; árvore com componentes, grupos de organização
      vazios, subconjuntos e peças
- [x] Inserir peça existente; criar peça nova no contexto; substituir componente mantendo vínculos
- [x] Reordenar a árvore só muda a visualização; numeração automática (índices sequenciais)
- [x] Carregamento seletivo (só o visível); botão direito: modo editável × visualização
- [x] Menu superior com modos de visualização da montagem
- [x] Restrições: coincidência (eixos, pontos, faces coplanares; direta/invertida), contato,
      distância (±, fórmula), ângulo (explícito, paralelo, perpendicular), fixação, fixação conjunta,
      restrição automática, conversão de tipo sem apagar, repetição inteligente (parafusos em padrão)
- [x] Movimentação manual (translação/rotação por eixos/planos) respeitando ou não as restrições;
      alinhamento instantâneo sem histórico; movimento inteligente (cria restrição); parada por colisão
      — *o movimento inteligente é feito pela Restrição automática (escolha dois elementos); não há
      ainda arrastar-e-encaixar com criação de restrição*
- [x] Interferências (volume comum) — *folga mínima entre peças ainda não*

## M2 — Cinemática

- [x] Juntas: revolução, prismática, cilíndrica, parafuso, esférica, planar, rígida, ponto-em-curva,
      came/seguidor (perfil do came × ponto/rolete), engrenagem, pinhão-cremalheira e cabo (razão)
- [x] Mecanismo automático a partir das restrições; comandos por cursor; quadros de movimento
- [x] Análise de graus de liberdade (comandos × GDL restantes)
- [x] Simulação por leis de tempo; velocidade e aceleração

## M3 — Chapa metálica

- [x] Parâmetros globais: espessura, raio de dobra, fator K / tabela de dobra
- [x] Parede base, aba de aresta (altura, ângulo, limites laterais), flange com raio e alinhamento
      interno/externo, bainha (circular/esmagada), extrusão de chapa
- [ ] Abas por superfície de base, limites por objetos de referência; controle hierárquico de
      flange e alma — *pendente*
- [x] Dobra por linha; joggle (desvio em Z); furos flangeados; estampos (veneziana, rebaixo);
      alívio de canto retangular
- [ ] Dobra cega e cônica; cortes flangeados; alívios por norma aeronáutica — *pendente*
- [x] Dobrar/desdobrar sincronizados; tabela de dobras; aviso quando o planificado não puder ser
      calculado exatamente — *linhas de dobra desenhadas na vista planificada ainda não*

## M4 — Estrutural

- [x] Membros estruturais por linha guia: perfil, ponto de ancoragem, orientação, offsets —
      totalmente associativos ao esqueleto
- [x] Biblioteca de famílias de perfis (cadastro) com pontos de origem e regras de criação a partir
      do arame 3D
- [x] Meia-esquadria, aparar, recorte de encaixe com folga de solda
- [x] Placas de extremidade, reforços triangulares/retangulares com chanfro de alívio

## M5 — Exibição / Composer

- [x] Estados de exibição salvos (visível/invisível, cores), vista explodida
- [x] Animações simples com linha do tempo e estados dinâmicos (estilo Composer)
- [x] Coletores de exibição usados pelo detalhamento 2D/3D

## M6 — Detalhamento 3D (PMI)

- [x] Árvore de PMI por nível (montagem, submontagem, peça)
- [x] Cotas, tolerâncias, ajustes, rugosidade, solda e notas no 3D, associativas

## M7 — Detalhamento 2D (módulo separado)

- [x] Folhas e formatos; vistas padrão, isométrica, corte com hachura, detalhe — associativas
- [x] Cotas lineares, angulares, radiais e diametrais; tolerâncias; rugosidade; solda; notas
- [x] Tabelas: furação, dobras, lista de corte de perfis, BOM; vista planificada; balões automáticos
- [x] Esquemas cinemáticos simplificados; 1º diedro (ABNT/ISO E), folhas ISO 5457, PDF/SVG/DXF
      — *cotas presas às posições projetadas (não reconhecem a aresta se a peça mudar muito)*

## Transversais

- [x] Configurações: todas as personalizações viáveis e visuais
- [x] Toda feature com figura e tooltip PT/EN
