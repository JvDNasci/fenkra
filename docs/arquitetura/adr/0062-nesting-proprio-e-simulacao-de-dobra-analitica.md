# ADR 0062: nesting com motor próprio e simulação de dobra analítica na seção

Data: 2026-11-06. Estado: aceita. Itens: B116, B173.

## Contexto
O B116 pedia nesting "por jagua-rs/sparrow". Os dois são Rust (MPL-2.0 e MIT), mas trazem uma árvore de
dependências grande para uma máquina de 16 GB que já compila OpenCASCADE, recusam furo interno e a
otimização do sparrow leva minutos. O B173 pedia simulação de dobra com "busca limitada" e retorno
elástico "por fórmula analítica calibrável por material", sem tabela de ferramentas embutida.

## Decisão
1. Nesting próprio em `fenkra-solidos/src/nesting.rs`: contorno da planificada em polígono, colisão
   exata por distância entre segmentos, posição por canto inferior esquerdo com candidatos nos
   vértices das peças vizinhas e compactação por deslizamento. "Melhor esforço" repete com ordens
   diferentes e é cancelável. Os motores abertos continuam possíveis atrás da mesma interface
   (`PecaNesting`, `Opcoes`, `Layout`) se a heurística própria não bastar.
2. Folga mínima de 1 micrômetro: sem ela, duas peças sobrepostas borda sobre borda passam como livres.
3. Furos ficam só no desenho; a colisão trata a peça como preenchida. Peça pequena dentro de furo de
   peça grande fica fora desta versão.
4. Simulação de dobra em `chapa_dobra_sim.rs`, na seção perpendicular à dobra, no quadro da matriz.
   Só dobras já feitas e paralelas à atual entram na colisão; as demais são listadas como "não
   verificadas". A posição testada é a final de cada dobra. Não há colisão 3D completa com a máquina.
5. Ferramental e material são do usuário: estilos `EstiloRegras` (fórmulas com `espessura`), sem
   defaults numéricos. Falta de campo é erro, não valor assumido.
6. Retorno elástico: `Ri/Rf = 4K^3 - 3K + 1`, `K = Ri*Y/(E*t)` (válido para `K < 0,5`), arco neutro
   conservado, e um fator de calibração por material que escala a diferença.

## Consequências
- Sem dependência nova. A qualidade do nesting é de heurística gulosa, não de ótimo.
- A biblioteca de ferramentas vive na memória da janela; persistir no documento ou nas configurações
  fica para um item próprio.
- A marcação 3D da dobra inviável tem os dados (`Simulacao::inviaveis`), mas o realce na viewport
  ainda não foi ligado.
