# ADR 0028 — Parâmetros tipados, unidades reais e peça v2

## Contexto

Os parâmetros eram só números em milímetros ou graus. Faltavam unidades reais (polegada, centímetro,
metro), valores que não são número (texto, verdadeiro/falso, escolha numa lista), limites com aviso e uma
forma de suprimir um comando conforme um parâmetro (uma peça com e sem chanfro, por exemplo).

## Decisão

- **O valor interno não muda.** Comprimento segue em milímetros e ângulo em graus. A unidade só converte o
  que a pessoa escreve (`2 in`, `10cm`, `1.5 rad`, tratada no analisador de expressões) ou vê
  (`Parameter.exibir`: mm, cm, m, pol). Trocar a unidade de exibição não move geometria.
- **Tipos** (`ParamTipo`): número, texto, booleano e lista. Booleano vale 1 ou 0 nas contas e aceita
  `true`/`false`. Lista guarda a opção escolhida em `expr` e as opções em `opcoes` (cada uma é expressão).
  Texto não entra nas contas.
- **Limites e checagens são avisos, nunca erro.** `min`, `max` e `verificacao` são expressões; o resultado
  vira `AvisoParam` mostrado na árvore. O recálculo continua com o valor que a pessoa digitou.
- **Supressão por expressão.** `Feature.suppress_if` suprime o comando enquanto o resultado for diferente de
  zero; vale em `Document::is_active`, então árvore, recálculo e exportação concordam. Expressão que não
  avalia não suprime (o erro aparece no painel do comando).
- **Formato aberto e versionado.** O arquivo da peça sobe para a versão 2, com um passo de migração
  (`peca_v1_para_v2`) e todos os campos novos com valor padrão. Arquivo v1 abre e é migrado; um Fenkra que só
  conhece v1 abre o v2 somente leitura, como manda `formato.rs`. A fixture do alpha 28 continua no teste.

## Consequências

- Documentos novos gravados por esta versão não editam em versões anteriores (abrem para leitura).
- A unidade escrita na expressão é conferida contra o tipo do parâmetro (aviso de unidade incompatível).
- `Parameter` ganhou campos; quem monta um à mão usa `Parameter::novo`.
