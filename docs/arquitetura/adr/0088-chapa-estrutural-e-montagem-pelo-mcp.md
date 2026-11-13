# ADR 0088: Chapa e estrutural no formato aberto, montagem pela sessão do MCP (B420)

- **Status:** aceita
- **Data:** 2026-11-13
- **Estende:** [ADR 0035](0035-esquema-aberto-de-arvore-parametrica.md)

## Contexto
O servidor MCP gera uma ferramenta para cada `tipo` de comando do esquema publicado da árvore (`.fkx`).
Chapa metálica, estrutural e montagem não estavam no esquema, e `listar_comandos` os dava como "fora deste
servidor": com as ferramentas de então, o kart de validação não saía. A montagem é outro documento (`.fka`),
com restrições que o formato aberto não descreve.

## Decisão
- **Chapa e estrutural entram no `.fkx` versão 1, de forma aditiva.** A peça ganha `chapa` (espessura, raio,
  fator K, planificada, método e alívios); a árvore ganha `parede_chapa`, `aba`, `bainha`, `dobra`, `membro`,
  `placa_extremidade` e `reforco`; as referências ganham `reta_pontos`, que é como o esqueleto estrutural
  costuma ser desenhado. Arquivo de antes continua válido. Um Fenkra antigo recusa o arquivo novo pelo tipo
  desconhecido, com o caminho do item, em vez de perder o comando calado.
- **Só o que tem leitura e escrita completas.** Aba por perfil ou superfície, recorte, estampo, cantos,
  solda, dobra de tubo no nó, regra própria por canto e reforço de contorno livre ficam fora e vão escritos
  em `fora`, como já acontecia. Estilo de chapa vai resolvido em valores; a ordem de dobra manual vai para
  `fora` porque é presa a ids internos.
- **Referência por ponto, como o filete.** Aba e bainha apontam a aresta, placa e reforço o lugar, por um
  ponto no corpo daquele passo. A aresta do modelo que guia um membro vira polilinha fixa ao gravar, como o
  plano de face vira `plano_fixo`.
- **As ferramentas da chapa e do estrutural saem do esquema**, pelo mesmo gerador das outras; só título e
  descrição ficam numa tabela. `definir_chapa` é escrita à mão porque muda a peça, não a árvore.
- **A montagem não entra no formato aberto por ora.** A sessão ganha uma montagem ao lado da peça
  (`nova_montagem`, `inserir_componente`, `restringir`, `mover_componente`, `montagem`, `bom`,
  `salvar_montagem`), sobre o mesmo `fenkra_montagem::Assembly` e o mesmo solucionador do aplicativo. Ela
  grava `.fka`, que o aplicativo abre com as restrições; em `.fkx` saem componentes e posições, e as
  restrições vão para `fora`.
- **Elemento de restrição sem clique:** um ponto no mundo sobre a face (plana vira plano, cilíndrica vira
  eixo, como o clique do aplicativo), um plano ou eixo da origem da peça, ou um ponto. O componente é uma
  peça `.fkp` da pasta, porque é o que a montagem gravada consegue abrir.
- **Desfazer e repetição valem para a montagem.** O histórico guarda peça e montagem juntas, e os comandos
  que mudam a montagem entram no registro da sessão e em `reproduzir`.

## Consequências
- Um agente monta suporte de chapa, quadro soldado e conjunto com restrições só pelo servidor, e cada passo
  confere volume e caixa. O registro refaz tudo sem agente.
- O formato aberto cresce sem mudar de versão; quando a montagem precisar de restrições no `.fkx`, isso
  pede uma decisão própria sobre como referenciar faces entre peças de arquivos diferentes.
- Juntas com movimento, engrenagens, subconjuntos e repetição de componentes continuam só no aplicativo, e
  `listar_comandos` diz isso.
