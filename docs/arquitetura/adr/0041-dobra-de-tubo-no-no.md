# ADR 0041 — Dobra de tubo no nó do esqueleto

## Contexto

O esqueleto da bancada Estrutural liga linhas guia em nós, e cada nó escolhia entre meia-esquadria,
aparar, encaixe ou nada (`CornerMode`). Faltava dizer que duas linhas são um tubo só, dobrado. A pesquisa
(`docs/pesquisa/dobra-de-tubos.md`) descreve cinco jeitos de fazer a curva: dobradeira ou rolos, V comum, V
com raio (gabarito), corte mais dobradeira e corte simples. As fórmulas de comprimento desenvolvido têm fonte
lida; as de alongamento, afinamento e linha neutra são derivação própria, sem ensaio.

## Decisão

- **O nó guarda a dobra, não um comando novo.** `CornerMode::Dobra` marca o nó e `StructJoint.bend`
  (`TubeBend`: modo, raio, regra, K, retorno, dimensão no plano, rasgos, folga de raiz, gomos) guarda os dados.
  A árvore continua transparente: a dobra é uma regra de canto do mesmo comando de membro, e o painel de
  cantos já existente a mostra. Arquivos antigos abrem iguais (`bend` some do arquivo quando vazio).
- **Tubo único = um comando só.** Os dois lados do nó precisam estar no mesmo comando; entre comandos
  diferentes a dobra é recusada com a causa. Assim perna, arco e perna viram um sólido e uma linha da lista
  de corte.
- **Contas separadas do sólido** (`st_dobra.rs`, sem kernel). O recompute, o painel, a ficha e a lista de corte
  chamam as mesmas funções, então os números nunca discordam. Cada resultado traz a origem dos valores.
- **Geometria da dobradeira.** Cada perna recua `R·tan(θ/2)`, a seção da ponta recuada gira `θ` em torno do eixo
  pelo centro da curva (`revolve`) e as três peças são fundidas. As seções das pernas são reorientadas ao longo da
  cadeia (transporte paralelo), por isso a rotação do membro não vale em perna dobrada; no tubo retangular o
  usuário escolhe qual dimensão fica no plano.
- **Modos em V e canto com gabarito ficam preparados.** Têm cálculo (dedução, abertura, passo, placa, solda,
  folga plana) conferido contra a pesquisa, mas a geometria do rasgo é dos itens B328 e B333: por enquanto o 3D
  mostra o canto vivo (meia-esquadria) e o status avisa. Corte simples usa a meia-esquadria e gera dois membros.
- **Regras cadastráveis no documento** (`Document.tube_rules`). O programa não traz tabela de material nem de
  norma: o usuário preenche e diz a fonte. Regra vazia não diz nada; sem regra, retorno é zero e a linha neutra
  fica no eixo (dito na tela). A regra avisa quando o raio é menor que o mínimo, mas não impede.
- **Estimativa nunca é valor de engenharia.** Alongamento, afinamento e deslocamento da neutra saem com o rótulo
  "estimativa" no painel, na lista de corte e na dica.

## Consequências

- O comprimento desenvolvido de um tubo dobrado vira o comprimento de corte da linha (a assinatura do grupo usa
  esse comprimento); `CutRow.tubo` leva modo, raios e ângulos. A tabela de dobras do desenho lê os mesmos números.
- Dobras em ciclo fechado, mais de duas dobras numa perna, linha guia fora do centro da seção e raio que não
  cabe na perna falham com motivo por linha (isolamento de falha do B79).
- Quando B328 e B333 gerarem os rasgos, só a escolha de geometria muda: os campos e as contas já existem.
