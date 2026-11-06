# ADR 0046 — Solver de montagem por aglomerados, batentes e núcleo de conflito

## Contexto

O solucionador punha todos os corpos numa matriz só: 6 incógnitas por corpo livre, com o jacobiano por diferença
finita e `JᵀJ` densa. Uma montagem com dezenas de fixadores pagava por todos eles a cada arraste. Limite de junta não
existia como conta: era um par guardado que só guiava o Animar. E `failed` listava quem ficou com resíduo, o que não
diz quem causou o conflito: o mínimos quadrados espalha o erro, e uma restrição inocente aparecia como culpada.

## Decisão

- **Aglomerados:** união de corpos pelas restrições, juntas e acoplamentos ativos. Componente fixo não une nada, porque
  não transmite movimento. Cada aglomerado é um sistema próprio; corpo sem restrição não entra em conta nenhuma. A
  conferência final continua vendo a montagem inteira, então nada que o sistema reduzido deixe de fora passa sem aviso.
- **Reunir o que já está travado:** se as restrições diretas entre dois corpos estão satisfeitas e têm posto 6, não sobra
  movimento relativo. Os dois viram um bloco de 6 incógnitas, e as linhas entre eles saem. Contra o que está fixo, o
  corpo sai do sistema. É o caso comum de um fixador com 0 GDL. Par não satisfeito fica de fora e é resolvido normalmente:
  a redução nunca decide posição, só aproveita a que já existe.
- **Batente como desigualdade:** cada grau limitado gera um resíduo que vale zero dentro da faixa e o excesso fora dela.
  O quadrado disso tem derivada contínua, e o Levenberg–Marquardt para na borda sem conjunto ativo. Essas linhas ficam
  fora de `residuals`: quem conta GDL não as vê, porque um batente não tira grau de liberdade nem com a junta encostada.
- **Comando além do limite:** o valor é preso à faixa antes de virar resíduo. A junta para no batente e sai em
  `no_limite`, que não é falha. Falha é restrição exigindo passar do limite: aí o conflito é real.
- **Núcleo de conflito:** `conflito::nucleo` faz filtro por remoção entre as vizinhas do mesmo aglomerado, com pedaços
  que diminuem até uma por vez. Tirar qualquer item do núcleo resolve o conflito. O orçamento é de 48 soluções, porque a
  prévia roda a cada mudança no painel; estourou, o conjunto vem marcado como não mínimo.

## Consequências

- `SolveReport` ganhou `no_limite`, `aglomerados` e `incognitas`. `Limites` passou de pares opcionais para `Faixa` com
  um `Limite` por extremo; arquivos com o par antigo continuam abrindo.
- `dof_per_comp`, `liberdade` e `dof_with` seguem no sistema inteiro: medem a montagem parada, uma vez por seleção.
- Giro de parafuso e de revolução continua medido em ±180°: uma faixa acima de uma volta não conta voltas.
- Largura, simetria e caminho, e limite em Distância e Ângulo, ficam fora desta decisão.
