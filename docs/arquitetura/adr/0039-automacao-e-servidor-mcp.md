# ADR 0039: Automação sem interface e servidor MCP

- **Status:** aceita
- **Data:** 2026-11-05
- **Item:** B39, B287. Relacionados: [ADR 0023](0023-conversor-em-processo-filho.md), [ADR 0035](0035-esquema-aberto-de-arvore-parametrica.md),
  [contrato JSON](../../formatos/api-json.md), [guia do servidor](../../automacao/mcp.md)

## Contexto

O Fenkra só era dirigido pela interface e por macros. Script, lote e agente de IA pedem uma porta estável, e o agente
traz dois riscos próprios: o STEP do cliente pode acabar num modelo remoto, e um agente não deve alcançar nada além
dos comandos do CAD.

## Decisão

- **Uma lista de comandos, três portas.** `fenkra-cli` guarda a lista fechada em `Sessao`. A linha de comando, o
  Python e o MCP são traduções finas dela. O servidor usa a biblioteca, e não o binário, porque a sessão precisa
  manter a peça entre chamadas sem pagar um processo por comando.
- **A peça em edição é um `.fkx`.** Cada comando vira um item validado pelo esquema publicado, e o documento é
  remontado dessa árvore. Não há um segundo caminho de montagem para divergir do formato aberto. Um `.fkp` com comando
  fora do `.fkx` continua medindo e exportando, mas não recebe item: gravar pela árvore perderia o que ficou de fora.
- **Python por processo, não por extensão nativa.** O cliente é Python puro e fala JSON com `fenkra-cli servir`. Uma
  extensão nativa prenderia o pacote a uma versão do Python e levaria o OpenCASCADE para dentro do processo do script,
  onde uma queda mata o script.
- **Sem linguagem.** Nenhuma porta avalia texto como código. O registro de comandos é dado, e `reproduzir` só aceita os
  comandos que mudam o modelo.
- **Uma porta de arquivo.** Todo caminho passa por `Pasta`: checagem do texto e, depois, do caminho canônico, porque um
  nome limpo pode ser uma junção. A escrita exclusiva faz a checagem e a reserva num passo só.
- **O aceite do STEP mora na linha de comando do servidor.** O servidor não sabe se o modelo é local, e tudo o que uma
  ferramenta devolve chega a ele. Um argumento de ferramenta não serve de aceite, porque quem o preenche é o próprio
  agente. `--liberar-step` só a pessoa escreve.
- **STEP lido em processo filho.** O programa chama a si mesmo com `--filho`, como no ADR 0023, e a comparação inteira
  roda lá. A peça do agente fica no processo do servidor: passar cada recálculo pelo filho custaria a remontagem a
  cada chamada, e o dado é nosso, não de fora.
- **A saída padrão é do protocolo.** O OpenCASCADE escreve estatística em `std::cout`. Os binários guardam o descritor
  real e apontam o 1 para a saída de erro, em vez de silenciar cada ponto do kernel.
- **Furo pela normal da malha.** O sinal `REVERSED` do B-Rep engana em cilindro espelhado, que tem sistema de eixos
  invertido. A normal da malha diz para onde a face olha, sem depender da história da forma.
- **O checador mora aqui, por enquanto.** O servidor precisa dele e o B237 ainda não existia. Ele cobre a comparação
  e o JSON; o mapa de cor no 3D do B237 falta.

## Consequências

- Campo novo no JSON não muda `api`. Mudar sentido ou tirar campo muda.
- Uma falha grave do kernel ao calcular a peça do agente encerra o servidor. O registro, gravado linha a linha, permite
  refazer a peça.
- O bloqueio cobre arquivos STEP. Uma `.fkp` com corpo importado ainda entrega medidas ao modelo.
- A comparação não alinha os corpos: uma peça certa em outro lugar reprova, de propósito.
