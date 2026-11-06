# ADR 0067: regras DFM por perfil JSON e espessura pela esfera máxima inscrita

Data: 2026-11-06. Estado: aceita. Itens: B112, B113, B197, B180.

## Contexto

A análise de fabricação precisa de um motor único para furo, bolso, chapa, injeção e impressão 3D, com limites que a pessoa mude sem recompilar, e de medidas geométricas confiáveis: espessura local, saliência, canto vivo, saída e undercut. O reconhecimento de features (`reconhece.rs`) já dava furos e filetes; `regras.rs` do core é para estilos de chapa com fórmula e não serve de perfil de processo.

## Decisão

1. **Perfil é um JSON** (`Perfil` com `Regra`s): identificador, entidade, medida, `min`, `max`, severidade, texto com `{valor}` e `{limite}`, e `ativa`. As combinações entidade e medida são uma lista fechada (`dfm::MEDIDAS`): o arquivo só escolhe limites, e uma medida inventada é rejeitada ao ler. Os de fábrica ficam em `crates/fenkra-solidos/perfis_dfm/` embutidos no binário; um JSON de mesmo nome em `%APPDATA%\Fenkra\perfis_dfm` substitui o de fábrica.
2. **A varredura mede só o que o perfil pede** e devolve violações com índice da face (ordem da malha), ponto e, quando há um corpo, a `FaceKey` estável.
3. **Espessura pela esfera máxima inscrita**, não pelo raio contra a normal. O raio solto mediria o comprimento de uma chapa na parede lateral. A esfera encosta no ponto, encolhe até só tocar a superfície e depois anda pelo eixo medial (afastando o centro dos dois contactos) até o raio parar de crescer. Aceite do B113: um degrau de 1 mm e 3 mm gera duas faixas com erro até 5%.
4. **Perto de quinas a esfera mede menos que a parede** (ela só contém o ponto se ficar menor). Por isso uma face só é acusada de parede fina se pelo menos 5% da área dela está fora do limite, e a legenda do mapa agrupa só as faixas com 2% a 5% da área.
5. **Saída e undercut por triângulo**: inclinação `asin(|n·d|)`, classe cavidade, núcleo, lateral ou ambígua (90% da área da face), undercut quando algo bloqueia o caminho até a abertura. O mesmo cálculo vale para o mapa de inclinação (cor por vértice) e para as regras.
6. A aceleração é uma árvore de caixas própria em `dfm_geo.rs` (raio e ponto mais próximo), sem dependência nova.

## Consequências

- B166 e B167 reaproveitam `Perfil`, `Relatorio` e `Malha`.
- A medida é na malha de exibição, não no B-Rep: o erro vem da tesselação (desvio de 1e-3 da diagonal).
- O realce 3D e a chave da face só valem com um corpo; com vários corpos a lista aparece sem realce.
