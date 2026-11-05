# ADR 0022 — VPM fase 1: diário só-acréscimo, ciclo de vida em dados e pacote autossuficiente

## Contexto

O projeto em pasta já versionava e liberava revisões (ADR 0009), mas qualquer documento mudava de estado
para qualquer outro, ninguém sabia quem fez o quê, o contador de PN avançava mesmo se a gravação falhasse e
não havia como mandar um conjunto inteiro para outra pessoa. Os itens B25, B47, B93, B94 e B265 fecham isso
sem servidor: tudo continua em arquivos que o usuário enxerga.

## Decisões

1. **Diário em linhas, nunca regravado** (`.diario.jsonl`). Cada evento (versão, restauração, revisão,
   transição) é uma linha acrescentada em modo `append`. O índice é regravado inteiro a cada salvamento, e um
   bug de edição poderia reescrever o histórico dentro dele; um arquivo que só recebe linhas no fim não tem
   esse caminho. Linha quebrada por queda de energia é pulada na leitura e o evento seguinte começa em linha
   nova.
2. **Restaurar versão é uma versão nova** que aponta para o conteúdo antigo (`restored_from`). Nenhuma versão
   anterior muda; o histórico só cresce.
3. **Contadores só avançam depois de gravar.** `add_document` reserva o PN e o id, grava, e só então os
   consolida; em falha desfaz tudo na memória. O disco e a memória nunca discordam sobre o próximo número.
4. **O ciclo de vida é dado** (`estados::Ciclo`, JSON): estados, transições (`de`, `para`, `papel`,
   `exige_motivo`) e esquema de revisão. A troca livre de estado foi removida; fora da tabela, a mudança é
   recusada com o motivo. Liberar é a transição para `Liberado` e só acontece junto com a letra, depois da
   lista de conferência. O crate não recalcula nada sozinho (o recálculo mora em `fenkra-solidos`); quem tem o
   documento aberto informa o resultado em `Externos`, e sem ele o item aparece como "não verificado".
5. **Referências são lidas do JSON, sem abrir o documento** (`relacoes`). Montagem guarda o caminho no nó,
   desenho na vista, link entre peças guarda uid + caminho. Reparar troca só o texto do caminho, preservando
   o resto do arquivo byte a byte.
6. **Pacote sem dependência nova.** O `.zip` é gravado sem compressão (método armazenado) por um escritor de
   cerca de cem linhas, conferido contra o leitor do Python. O pacote leva `manifesto.json` com origem, hash
   (blake3) e uid de cada arquivo, colisões renomeadas e o que ficou de fora; ao final é reaberto e verificado.
7. **Salvar como** muda o `uid` de toda cópia (identidade nova) e mantém o `uid` só ao renomear/mover. Falha
   no meio desfaz tudo o que foi gravado.

## Consequências

- Quem editava `DocState` à mão precisa passar por `transicionar`/`liberar`. Um estado novo (`EmAprovacao`) foi
  acrescentado: arquivos gravados com ele não abrem em versões anteriores do programa.
- O papel do usuário (`Project::papeis`) hoje é local e não autenticado; ele registra intenção, não segurança.
  Controle de acesso de verdade pertence à fase com servidor.
- "Derivado" no pacote significa documento com selo de origem; "linkado", referência por uid + caminho. Se o
  produto passar a ter outros tipos de derivação, a regra de `pacote::reunir` precisa acompanhar.
