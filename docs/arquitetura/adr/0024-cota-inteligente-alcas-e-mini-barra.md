# ADR 0024 — Cota inteligente, alças de anotação e mini barra

## Contexto

B268, B269 e B270 pedem que a anotação 3D se comporte como os softwares de modelagem CAD avançados: o tipo da
cota sai do que foi escolhido, a anotação selecionada tem alças próprias e uma barra sobre ela traz os campos
principais. O painel lateral não pode ser o único caminho de edição.

## Decisão

- A inferência do tipo é função pura dos alvos (`fenkra_core::pmi_infer::infer`): recebe os elementos na ordem
  do clique e devolve o veredito (`Empty`, `Pending`, `Provisional`, `Complete`, `Invalid`) e as variantes que o
  Tab percorre. Não cria tipo de anotação novo: o resultado é um `PmiKind` que já existe, então o documento, o
  diagnóstico de alvo e o exportador não mudam.
- Uma cota que ainda aceita outro clique (um furo pode virar diâmetro ou, com outro furo, distância entre eixos;
  dois vértices podem virar distância ou, com o terceiro, ângulo) é `Provisional`: Enter a cria. As demais criam
  sozinhas no clique que as completa.
- O encaixe das alças (1 mm, 15°, eixos do modelo) e o que cada alça grava saem de `fenkra_core::pmi_snap`, em
  milímetros do modelo e independentes da câmera. A interface só leva o raio do cursor ao plano de trabalho.
- Dois campos novos em `PmiStyle`, ambos opcionais no arquivo: `ext_over` (avanço das linhas de apoio) e
  `text_along` (posição do texto ao longo da linha de cota), os dois em mm no modelo, então seguem o componente
  na montagem. `decimals` fixa as casas da medida (antes sempre até 2).
- Os vértices da chamada acompanham a ponta quando ela se move (reancoragem pelo alvo), para a chamada não ficar
  para trás quando um componente da montagem anda.
- A mini barra é uma sessão: guarda a anotação como estava, Enter aceita, Esc devolve, e a sessão inteira vira um
  passo de desfazer. Com a barra aberta o painel lateral da anotação fica recolhido; "Avançado…" o abre.

## Consequências

Nenhuma migração: arquivos antigos abrem com os valores padrão. O Tab da cota inteligente é contador de sessão
(como o plano da anotação), não parte do documento. Faces planas não paralelas são recusadas com explicação em
vez de virar ângulo entre planos, que fica para quando houver âncoras de interseção.
