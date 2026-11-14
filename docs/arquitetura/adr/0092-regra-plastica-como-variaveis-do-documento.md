# ADR 0092: regra plástica do documento como variáveis das fórmulas

Data: 2026-11-14. Estado: aceita. Itens: B198 (base de B148).

## Contexto

Peças injetadas seguem três números (espessura nominal, ângulo de saída, raio nominal) e alguns limites. Casca, nervura, saída e os recursos de plástico (boss, snap-fit, grelha, lip) precisam ler esses números, mudar todos juntos quando a regra muda, aceitar sobrescrita só em modo avançado e avisar na árvore quando um recurso sai da regra. O estilo de chapa (B283) resolve um problema parecido com campos por chave dentro de `SheetParams`; aqui os consumidores são comandos de tipos diferentes.

## Decisão

1. **A regra é um objeto do documento** (`Document::plastico: Option<RegraPlastica>`, em `fenkra-core/src/regra_plastica.rs`), copiado de uma biblioteca: as de fábrica em `regras_plasticas.json` e as do usuário em `%APPDATA%\Fenkra\regras_plasticas`. Cada gravação na biblioteca sobe `versao`; o documento guarda a versão que copiou, e a tela oferece trazer uma mais nova.
2. **Os comandos consomem a regra pelas fórmulas.** O recálculo põe `plastico_espessura`, `plastico_saida`, `plastico_raio` e `plastico_nervura` no conjunto de variáveis (como a chapa faz com `espessura`). Um comando criado com a regra na peça já nasce com a fórmula. Como a assinatura de cada comando inclui os valores das variáveis que ele cita, mudar a regra refaz só os comandos que a citam.
3. **Sobrescrita é escrever um número no lugar da fórmula**, e a tela só deixa fazer isso com o modo avançado da regra ligado; sem ele, o campo volta à fórmula.
4. **O diagnóstico roda depois do recálculo, fora do cache dos comandos** (`fenkra-solidos/src/plastico.rs`): cada campo governado é avaliado e comparado com os limites; o que sai vira aviso no status, somado ao que já houver. Assim, mudar só um limite atualiza a árvore sem refazer geometria.

## Consequências

- Fórmula `plastico_*` sem regra na peça vira erro de expressão nomeado no comando.
- Os recursos do B148 entram no diagnóstico acrescentando os campos deles em `plastico::campos`.
- `final_shape` e a exportação também recebem as variáveis da regra; outros leitores que avaliam parâmetros por conta própria (lista de corte, tabelas) não precisam delas porque não leem os comandos de plástico.
