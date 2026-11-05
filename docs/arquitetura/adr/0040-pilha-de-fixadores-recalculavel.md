# ADR 0040 — Pilha de fixadores como objeto recalculável

## Contexto

O preenchimento de furos (B29) põe parafuso, arruelas e porca na montagem e esquece de onde vieram. Depois disso
nada liga as peças ao furo: mudar o furo de M6 para M8 deixa o parafuso errado sem aviso, e não há o que reabrir para
trocar família ou comprimento (B98).

## Decisão

- **Objeto novo no documento:** `Assembly::pilhas`, uma `Pilha` por conjunto, com a âncora, o que a pessoa pediu e os
  componentes que ela gerou. As peças continuam nós comuns da árvore, com as mesmas restrições. Lista de peças,
  desenho e seleção não mudam.
- **Âncora:** comando do furo e índice entre os centros dele, mais a boca da última vez. Só a posição não serve,
  porque o furo pode mudar de lugar. Só o índice também não, porque apagar um centro renumera os seguintes. Por isso
  a ordem é: mesmo lugar; senão mesmo índice, se o comando não perdeu centro; senão o ponto foi apagado e a pilha sai.
- **Assinatura:** cada pilha guarda um texto com o que as peças representam, com a boca no sistema da peça. Recalcular
  só refaz o que mudou, e mover a peça não refaz nada: as restrições já levam os fixadores junto.
- **Refazer troca as peças:** apaga e insere de novo pelo mesmo código do preenchimento, em vez de editar arquivo e
  posição de cada peça. Assim há um só caminho que sabe montar a pilha. O custo é que os ids das peças mudam;
  restrição que a pessoa tenha criado num fixador sai junto.
- **Falha fica na linha:** pega que não cabe ou rosca curta vira `Diagnostico` de erro, com as peças mantidas, em vez de
  recusa. Sumir com o fixador esconderia o problema.
- **Escolha à mão vence o automático:** trocar tamanho ou comprimento na tabela desliga "seguir o furo" ou "atualizar
  o comprimento" daquela linha. Sem isso o recálculo seguinte desfaria a escolha.
- **Ponto manual:** pilha sem furo, de um ponto numa face. O recálculo geral não muda nada nela.

## Consequências

- Fixadores inseridos antes desta versão não têm pilha: não aparecem no assistente. Rodar o preenchimento de novo não
  os adota, porque o eixo já consta como ocupado.
- Item avulso do catálogo (B242) também fica fora das pilhas.
- O recálculo roda ao voltar da edição de uma peça e pelo botão do assistente, não a cada resolução: ler os furos custa
  um recálculo da peça.
