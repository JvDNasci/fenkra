# ADR 0047 — Rasgos em V e canto com gabarito: geometria 3D, contorno plano e gabarito

## Contexto

O ADR 0041 deixou os modos V comum, V com raio (canto com gabarito) e corte + dobradeira só com cálculo:
o 3D mostrava o canto vivo e o status avisava. Os itens B328, B333, B329 e B334 pedem a geometria do tubo
dobrado por cortes, o tubo aberto com os recortes, o gabarito de marcação em escala 1:1 e o contorno plano
para laser. A geometria vem de `docs/pesquisa/dobra-de-tubos.md` (seções 7 e 8), derivada sem protótipo.

## Decisão

- **Tubo fechado no 3D, um sólido só.** `st_dobra_v.rs` termina cada nó de rasgo: o V comum mantém a
  esquadria das paredes e substitui o canto de fora por uma placa em arco (raio interno `ri`, externo
  `ri + t`); o V múltiplo é um polígono de esquadrias com vértices na face interna da placa (canto vivo);
  o canto com gabarito troca placa e laterais por uma placa em arco de raio externo `Ro` e setores de raio
  `Ro − t` com ápice no centro da curva, e a parede interna segue até a bissetriz. Pernas e corpos extras
  se fundem na cadeia (`blocos_fundidos`), então o tubo é uma linha da lista de corte, como na dobradeira.
- **O tubo aberto não é uma segunda geometria no recompute.** É o planificado em 2D (`st_dobra_corte.rs`):
  quatro faixas do perímetro nominal da face externa, com um hexágono por V e a andorinha do canto. O painel
  alterna Fechado (3D) e Aberto (planificado). Assim o documento guarda um estado só.
- **Uma fonte de números.** O 3D, o contorno plano e o gabarito leem o mesmo `ResultadoDobra`
  (`rasgos`, `folga_raiz`, `placa`, `passo_v`, `abertura_v`). A posição do primeiro rasgo vem do início da
  zona de dobra do `TuboDobrado`. A folga de raiz desloca a bissetriz de e/2 (`ix_do_canto`).
- **Saídas escritas do zero** (`st_dobra_saida.rs`): SVG em mm, DXF R12 com camadas CORTE, MARCACAO, DOBRA,
  CONTORNO e COTA, PDF A4 vetorial em 1:1 que se divide em folhas com marcas de canto. O gabarito leva uma
  barra de 100 mm para conferir a escala de impressão. `fenkra-desenho` depende de `fenkra-solidos`, então
  a saída mora aqui e não reaproveita o exportador de folhas.
- **Honestidade.** Todo painel e todo arquivo dizem "a validar em peça real". O V múltiplo tem canto vivo no
  3D porque a pesquisa não dá raio de dobradiça por articulação. A seção precisa estar alinhada ao plano da
  dobra.

## Consequências

- Os testes do 3D conferem volume contra as peças somadas (pernas retas, placa, setores, cunha da parede
  interna), caixa envolvente e comprimento de corte; não provam que a peça real fecha. Isso depende do protótipo.
- A fusão depende de faces coincidentes entre as pernas cortadas e os corpos novos; mudar um plano de corte
  exige conferir o volume de novo.
- Dobras em planos com a seção virada (retangular) ainda não saem no planificado; o painel diz o motivo.
