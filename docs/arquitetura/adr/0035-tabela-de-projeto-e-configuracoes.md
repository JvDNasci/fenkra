# ADR 0035 — Tabela de projeto e configurações da peça

## Contexto

Uma peça com variantes (três comprimentos, com e sem chanfro) obrigava a copiar o arquivo. O ADR 0028 trouxe
parâmetros tipados e supressão de comando por expressão; faltava um objeto que junte os dois por variante, e
uma forma de a montagem escolher qual variante usar sem cair numa errada em silêncio.

## Decisão

- **Tabela de projeto** (`Document.tabela`, módulo `configuracoes`): linhas são configurações (nome único),
  colunas são `Parametro(nome)`, `Supressao(id do comando)` ou `Pn`. Célula vazia herda o valor base. Documento
  sem tabela não grava o campo, então o arquivo e o hash das versões antigas não mudam e o formato segue v2.
- **Aplicar é copiar e trocar.** `Document::com_configuracao(nome)` devolve uma cópia (sem a tabela) com a
  expressão dos parâmetros, o `suppress_if` dos comandos e o PN trocados. O recálculo, a forma final e a
  exportação usam essa cópia; não há segundo caminho de cálculo.
- **Configuração ativa** fica na própria tabela (`ativa`; vazio = valores base). `recompute_cached` e
  `final_shape` a aplicam.
- **Nome inexistente é erro visível, nunca "a primeira".** `ErroConfig::Inexistente` lista os nomes que existem.
  Na peça, o recálculo usa os valores base e devolve o motivo em `param_error` (mostrado no painel de parâmetros
  e na árvore); `final_shape` devolve nada. Na montagem, `compose` põe o erro no componente.
- **A montagem referencia por nome.** `Assembly.configs` guarda (componente, nome); vazio = a ativa da peça. O
  cache de peças guarda o cálculo por (arquivo, nome) e recusa nome que a peça não tem.
- **CSV sem planilha.** Cabeçalho `Configuração; PN; <parâmetro>; suprimir:<comando>`. Lê `;`, `,` ou tabulação e
  aspas; escreve com `;`. Coluna que não casa com parâmetro ou comando recusa a importação inteira e deixa a
  tabela como estava.

## Consequências

- Uma coluna presa ao id de um comando acompanha renomeações; apagar o comando faz a configuração dar erro
  visível ao aplicar, em vez de ignorar a coluna.
- A lista de peças da montagem ainda lê o PN do arquivo base; o PN por configuração está em `props.pn` da cópia
  e a ligação com a lista fica para depois.
