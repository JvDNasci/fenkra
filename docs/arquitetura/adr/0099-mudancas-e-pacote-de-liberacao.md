# ADR 0099: pedidos e avisos de mudança em arquivo e pacote de liberação em ZIP (B141)

Data: 2026-11-14. Estado: aceita. Item: B141.

## Contexto

O projeto já tem revisões, ciclo de vida e diário só-acréscimo, mas não registra **por que** uma revisão
nova saiu nem entrega à fábrica um conjunto fechado de arquivos de uma liberação.

## Decisão

1. **Registro em `mudancas.fenkra.json`**, na raiz do projeto, gravado de forma atômica, com pedidos
   (ECR) e avisos (ECN) numerados por projeto, sem reaproveitar número. Cada passo também vai para o
   diário (`TipoEvento::Mudanca`), que é a trilha que nunca se reescreve. Arquivo ilegível é erro, e não
   registro vazio: regravar por cima apagaria o histórico.
2. **Pedido guarda a revisão de antes; aviso só sai com revisão nova em todos os documentos.** O aviso é
   a prova de que a mudança foi liberada ("de A para B"), não uma intenção.
3. **Pacote de liberação em ZIP** (`fenkra_pacote::liberacao`), montado acima do kernel porque precisa
   recalcular a peça para o STEP. Cada peça entra na **versão da revisão vigente** (o arquivo de trabalho
   pode ter edição não liberada); obsoleta não entra. Com um aviso escolhido, entram as peças dele na
   revisão "depois". Desenhos viram PDF e DXF; montagens de topo viram lista estruturada em CSV, JSON e
   XML com estado e revisão. O manifesto leva o BLAKE3 de cada arquivo.
4. O que falha (peça com erro, desenho ilegível) vira aviso no manifesto e não derruba o pacote.

## Consequências

- Quem recebe o ZIP não precisa do Fenkra e confere a integridade pelo manifesto.
- O registro não tem papéis próprios: aprovar usa o autor do projeto; o controle de quem pode aprovar
  fica com o ciclo de vida (B265) quando o servidor de equipe existir.
