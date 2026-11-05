# ADR 0035: Esquema aberto de árvore paramétrica

- **Status:** aceita
- **Data:** 2026-11-05
- **Item:** B266. Relacionados: [ADR 0010](0010-referencias-v2.md), [ADR 0034](0034-termos-das-apis-e-exportadores-por-script.md),
  [especificação](../../formatos/arvore.md)

## Contexto

O STEP leva o corpo, não a receita. Para migrar com histórico falta um arquivo que um script de fora consiga
escrever e que o Fenkra remonte como árvore viva. O relatório de PLM sugeriu um formato próprio, pequeno e
versionado, em vez de adaptar um formato de comandos existente.

## Decisão

- **Formato próprio, separado do `.fkp`.** O `.fkp` espelha as estruturas internas: muda com o programa, tem ids
  numéricos e dezenas de campos. Quem escreve de fora precisa de um contrato pequeno e estável, então o `.fkx` é uma
  tradução com nomes próprios.
- **Versão pela cadeia do B64:** `Tipo::Arvore`, com `format_version` e passos de migração. Versão futura é
  **recusada**, ao contrário do `.fkp`, que abre só para leitura. O `.fkp` guarda tudo; aqui, ler o que se conhece
  faria um comando desconhecido sumir da peça sem aviso.
- **Um esquema só.** O JSON Schema publicado é embutido no programa e é o que a leitura usa. Campo desconhecido é
  erro, e o ponteiro JSON do erro diz onde.
- **Validador próprio de um subconjunto.** Interpreta só as palavras que o esquema usa e recusa as outras, para o
  esquema não ganhar uma regra ignorada. Um validador completo traria muitas dependências para um arquivo só.
- **Ligações fora do esquema.** Id repetido ou solto é barrado na remontagem, porque o JSON Schema não vê ligações.
- **Referência por ponto, sem chaves.** Dono e papel usam ids e numeração internos, que nada significam para outro
  programa. O arquivo traz um ponto sobre a aresta. A leitura acha a aresta no corpo daquele passo, troca o ponto
  pelo médio e deixa o recálculo promover a referência a chave, como em arquivo antigo (ADR 0010). Sem aresta
  claramente mais próxima, nada é escolhido e o comando fica em "referência perdida".
- **Exportar atualiza os pontos.** O ponto guardado é o do clique e fica para trás depois de uma edição. Antes de
  gravar, a chave acha a aresta atual e o ponto é refeito.
- **O que não cabe fica escrito.** Comando sem equivalente, e quem depende dele, vão para `fora`, dentro do próprio
  arquivo. Árvore incompleta nunca passa por inteira.
- **Fora do processo isolado, só a gravação.** A leitura roda no `fenkra-conv`, como a do STEP: o recálculo chama o
  kernel com dados de fora. Por isso `fenkra-conversor` passou a depender de `fenkra-solidos` e `fenkra-montagem`.
- **Montagem achatada.** Um arquivo não aponta para outro: subconjunto vira grupo, com as posições somadas. Nomes de
  arquivo tirados do JSON são limpos, para um arquivo hostil não escrever fora da pasta escolhida.

## Alternativas consideradas

- **Publicar o `.fkp` como formato aberto:** cada mudança interna quebraria scripts de terceiros.
- **Partes de procedimento e parâmetros do STEP:** cobrem pouco da árvore de construção, e o relatório de PLM não
  confirmou suporte nas ferramentas.
- **Guardar as chaves de referência no arquivo:** só serviriam na ida e volta dentro do Fenkra.

## Consequências

- A versão 1 cobre os comandos comuns e as 16 peças do corpus. Os demais comandos entram em versões novas.
- Mudar `FeatureKind` não obriga a mudar o formato, mas comando novo fica em `fora` até ganhar tradução.
- Plano de face vira plano fixo: depois da leitura, ele não segue mais a face.
- Reancorar recalcula o corpo antes de cada filete ou chanfro com pontos. O cache evita refazer o início da árvore,
  mas peça com muitos desses comandos demora mais para abrir.
