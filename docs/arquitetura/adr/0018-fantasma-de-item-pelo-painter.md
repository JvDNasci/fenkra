# ADR 0018: fantasma de item da biblioteca desenhado pelo painter da interface

## Contexto
O catálogo de normas (B241), a inserção avulsa (B242) e os candidatos do preenchimento de furos
(B248) precisam mostrar um parafuso, porca ou arruela translúcido que segue o cursor ou fica sobre cada
furo. A prévia fantasma de comandos (ADR 0017) entra na cena da viewport, que só é refeita quando o
documento muda. Um item que acompanha o cursor mudaria a cada quadro e refaria a cena inteira.

## Decisão
O item é uma malha pequena, gerada uma vez pelo recálculo do próprio documento do fixador e guardada em
cache por (família, tamanho, comprimento). O fantasma e a prévia 3D do painel saem pelo `Painter` do egui:
triângulos projetados pela câmera da viewport, sem faces de costas, ordenados de trás para a frente, com
transparência e contorno na cor de destaque do tema.

## Consequências
- O fantasma não passa pela profundidade da cena: ele aparece por cima das peças, sem ser escondido por
  elas. Para um item de poucos milímetros que precisa ser visto dentro de um furo, isso é desejável.
- Sem faces de costas e sem teste de profundidade entre triângulos, itens côncavos (nenhum da biblioteca
  atual) poderiam sair com sobreposição errada. Se a biblioteca ganhar peças côncavas, o caminho é a
  camada `ghost` da cena, com a malha enviada quando o item muda e a posição passada por uniforme.
- Nenhum recálculo da cena a cada movimento do mouse.
