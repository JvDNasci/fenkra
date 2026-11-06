# ADR 0053: PMI 3D com alvos de referência, acabamento estruturado e balões com prévia

Data: 2026-11-06. Itens: B299, B189, B190.

## Contexto

O PMI 3D guardava rugosidade como `Ra` e processo, solda só no sistema A, e criava os balões
automáticos trocando todos os anteriores sem avisar. Não havia alvo de referência, e a regra 3-2-1 não
tinha onde ser conferida.

## Decisão

- **Balões (B299).** O comando não altera o documento ao abrir: monta um `Plano` (`pmi_baloes::planejar`)
  com o que será criado, o que sai e o que fica, e o diálogo mostra "N criados, M trocados" com a prévia
  no 3D. Só `Aplicar` mexe na lista, depois de um passo de desfazer (`push_undo`), então Ctrl+Z volta tudo
  de uma vez. `PmiKind::Balloon` ganhou `manual` (padrão `false`, então documentos antigos contam como
  automáticos, que é o que eram). Balão manual é preservado e o número dele não ganha segundo balão.
  A legenda 3D abre um diálogo de canto (os oito cantos da caixa do modelo ou o vértice selecionado).
- **Alvos (B189).** `PmiKind::DatumTarget { alvo: AlvoRef }` em `pmi_alvos.rs`. A forma (ponto, linha,
  círculo, círculo-curva, retângulo, área, curva) escolhe o símbolo vetorial e o tipo do AP242
  (`placed_datum_target_feature`); área e curva vão como alvo explícito com `advanced_face`. O alvo
  móvel guarda direção e curso e não conta na regra 3-2-1. A regra confere, por referência em ordem
  alfabética, os graus de liberdade que os alvos tiram (ponto 1, linha e curva 2, superfície 3) contra
  3, 2 e 1, e acusa letra sem referência, três pontos em linha, número repetido e dimensão inválida.
  Cotas ligadas ao alvo aparecem enquadradas na hora de desenhar, sem mexer no estilo gravado delas.
- **Acabamento (B190).** `Roughness` ganhou `extra: RugExtra` (remoção, contorno todo, parâmetro, limites,
  máximo, amostragem, estrias, sobremetal, norma, área). Um dado só; a norma escolhe a posição dos campos
  (`pmi_acabamento::linhas`). Solda ganhou `SistemaSolda` (A, B de linha dupla, AWS com a junta do lado da
  seta embaixo), letra de acabamento e penetração total em `WeldExtra`. O comando "Solda dos cordões" cria
  um símbolo por cordão da bancada Estrutural com `fenkra_desenho::solda_cordao::solda_do_cordao`, o mesmo
  cálculo do desenho.
- **Troca de dados.** Rugosidade e solda não têm entidade semântica no AP242: `pmi_acabamento::via_step`
  os manda como PMI gráfico e `apresentacao_grafica` entrega as polilinhas 3D do símbolo. O gravador
  AP242 do projeto ainda não escreve as anotações do PMI (só repassa o que o XDE lê), então essa ponta
  fica pronta no núcleo, sem exportador que a chame.

## Consequências

- Tudo é `serde(default)`: arquivos antigos abrem iguais.
- A regra 3-2-1 usa a ordem alfabética das letras como ordem de primária, secundária e terciária; é a
  convenção do desenho, mas um projeto que use outra ordem terá avisos a ignorar.
- Sistema B e AWS foram descritos a partir de fontes públicas resumidas, sem o texto das normas; a
  disposição das linhas deve ser revista contra a norma por quem a tiver.
