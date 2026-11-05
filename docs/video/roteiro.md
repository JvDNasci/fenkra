# Roteiro: como o Fenkra nasceu

Vídeo de 5 a 6 minutos, narrado em primeira pessoa. As imagens estão em `capturas/roteiro/`, numeradas
na ordem de uso. Trechos entre colchetes são seus: só você sabe essa parte.

**Números (05/11/2026):** 16 dias do primeiro commit até hoje, 287 commits, cerca de 118 mil linhas de
código, 1.014 testes, 28 versões alpha, 356 ícones. Backlog: 325 itens, 83 feitos.

**Ideia central:** um CAD não é um programa, são vários programas que precisam conversar. A história é
cada bancada nova aparecendo, até que a peça do começo vira um produto.

| Tempo | Imagem | Fala |
|---|---|---|
| 0:00 | `25-hoje-desenho`, depois corte rápido para `01-primeira-peca` | "Esta folha técnica saiu de um CAD que não existia há 16 dias. E esta placa com quatro furos foi a primeira coisa que ele conseguiu fazer." |
| 0:15 | `01-primeira-peca` | "[Quem é você e por que decidiu fazer um CAD. Qual problema real te irritava?]" |
| 0:45 | `02-sketch-cotas` | "A base é um desenho 2D com cotas que dirigem a geometria. Muda o número, muda a peça. Parece simples, mas tudo o que vem depois depende disso." |
| 1:00 | `03-logo-gerado-por-codigo`, `04-logo-fenix-final` | "Ele precisava de uma cara. O primeiro logo foi gerado por código, e dá para ver. A fênix ficou. [Por que a fênix?]" |
| 1:15 | `05-icones` | "Depois vieram os ícones: hoje são 356, todos gerados por um script." |
| 1:25 | `06-modos-de-exibicao`, `07-superficies-zebra` | "Então a peça deixou de ser só um sólido: modos de exibição, superfícies e análise de reflexo para ver a qualidade da curva." |
| 1:45 | `08-chapa-dobrada`, `09-chapa-planificada` (alterne as duas) | "Chapa metálica foi um divisor de águas. A peça dobrada precisa virar a peça plana que vai para o corte, com a conta certa de cada dobra." |
| 2:05 | `10-primeiro-desenho` | "No terceiro dia saiu o primeiro desenho técnico, com vistas, corte, detalhe e carimbo." |
| 2:20 | `11-montagem-mecanismo` | "Depois, montagem: peças que se encaixam por restrições e mecanismos que se movem." |
| 2:35 | `12-estrutural-quadro` | "Estruturas de tubos, com cantos em meia-esquadria e lista de corte." |
| 2:50 | `13-pmi-3d`, `14-simbolo-de-solda` | "E anotações direto no modelo 3D: tolerâncias, notas e até o símbolo de solda." |
| 3:05 | `15-aba-curva` | "Mas nem tudo funcionava. [Conte um problema real. Exemplo: dobras em sequência que quebravam a chapa.]" |
| 3:25 | Slides do design review (`docs/contextos/review/`) | "Aí parei para usar de verdade. Anotei 58 slides de bugs e pedidos. Essa foi a virada: de fazer funcionalidades para fazer um produto." |
| 3:50 | `16-tela-inicial-antes`, `17-tela-inicial-depois`, `18-carregamento` | "O visual também amadureceu: de uma tela vazia para exemplos com prévia 3D girando e uma marca." |
| 4:05 | `19-tema-claro`, `20-hoje-inicio` | "Temas, e um nome novo. Ele nasceu SolidCadCraft, mas o nome era parecido demais com projetos que já existiam. Virou Fenkra: fênix mais craft." |
| 4:25 | Painel do backlog + (gravar) agentes trabalhando | "Quase tudo foi feito com inteligência artificial, e o trabalho mudou: eu defino o que é certo, testo e decido. [Sua opinião sincera: onde a IA ajudou e onde errou.]" |
| 4:55 | `21-hoje-chapa` a `25-hoje-desenho`, cortes rápidos | "Hoje são 287 commits, mais de mil testes e as bancadas conversando entre si." |
| 5:15 | `25-hoje-desenho` | "Ainda é alpha: dos 325 itens do backlog, 83 estão feitos. Ele vai ser aberto, porque [seu motivo]. Se você usa CAD, testa e me diz o que quebrou." |

## Gravar com o app aberto

Imagem parada não mostra o que mais impressiona. Vale gravar estes trechos de tela:

1. Mudar uma cota e ver a peça se atualizar.
2. Alternar dobrado e planificado na chapa.
3. Gerar a montagem a partir de uma estrutura.
4. Preencher furos com parafusos (Fixadores).
5. Animar um mecanismo.
6. Agentes rodando no painel de orquestração.

## Antes de publicar

- Os itens 3 e 4 não têm imagem: são recursos posteriores à alpha.26 que ainda precisam ser gravados.
- As capturas antigas mostram o nome SolidCadCraft. Isso é parte da história, mas confira se o trecho do
  nome explica isso antes de elas aparecerem, ou cite o nome antigo logo no começo.
- Os números acima são de 05/11/2026; atualize antes de gravar.
- Não cite softwares proprietários; use "softwares de modelagem CAD avançados".
