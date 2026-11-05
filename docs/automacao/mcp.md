# Servidor MCP do Fenkra

`fenkra-mcp` deixa um agente reconstruir uma peça por comandos: ele lê um STEP, monta a árvore, compara com o alvo e
corrige. O resultado é árvore de verdade, com esboços cotados e parâmetros, não um corpo importado.

Roda só na sua máquina, por stdio. O programa não abre conexão de rede. Quem manda dados para um modelo é o cliente
que você usa, e por isso o aceite abaixo existe.

## Como ligar

O cliente MCP inicia o servidor. Na configuração dele, informe o programa e os argumentos:

```json
{
  "mcpServers": {
    "fenkra": {
      "command": "C:/Fenkra/fenkra-mcp.exe",
      "args": ["--pasta", "C:/projetos/reconstrucao"]
    }
  }
}
```

| Argumento | O que faz |
|---|---|
| `--pasta DIR` | Obrigatório. Única pasta que o servidor lê e grava |
| `--liberar-step` | Aceite explícito: o que for lido de um STEP pode ir para o modelo do cliente |

## O STEP não sai sem o seu aceite

O servidor não sabe se o modelo do outro lado é local, e tudo o que uma ferramenta devolve chega a esse modelo. Por
isso `consultar_step` e `comparar_alvo` são recusadas, com o código `step_bloqueado`, até você iniciar o servidor com
`--liberar-step`. Use a opção com um modelo local, ou se concordar em mandar esses dados a um modelo remoto.

O aceite fica na linha de comando porque só você a escreve: nenhuma ferramenta o liga. O arquivo em si nunca é
enviado, só o resumo geométrico (medidas, faces, arestas, furos). Esse resumo descreve a peça, então trate-o como dado
do cliente.

**Limite conhecido:** o bloqueio vale para arquivos STEP. Uma peça `.fkp` que já tenha um corpo importado abre em
`abrir_peca`, e as medidas dela chegam ao modelo. Não deixe na pasta o que não deve ser lido.

## Ferramentas

| Ferramenta | Para quê |
|---|---|
| `consultar_step` | Caixa, massa, faces, arestas, planos, cilindros e furos de um STEP. Pede aceite |
| `nova_peca`, `abrir_peca` | Começar do zero ou abrir `.fkx` e `.fkp` |
| `listar_parametros`, `definir_parametro` | Parâmetros da peça |
| `criar_plano`, `criar_esboco` | Plano deslocado; esboço com geometria, restrições e cotas |
| `extrudar`, `furar`, `filetar`, `chanfrar` | Os comandos de modelagem |
| `desfazer` | Volta o último comando que mudou a peça |
| `recalcular`, `medir`, `topologia_modelo` | Status, medidas e topologia da peça em edição |
| `comparar_alvo` | Checador de similaridade contra o STEP alvo. Pede aceite |
| `salvar_peca`, `exportar_step` | Gravar na pasta. Nunca sobrescreve sem `sobrescrever` |

Os campos seguem o [formato da árvore](../formatos/arvore.md), e as respostas seguem o
[contrato JSON](../formatos/api-json.md).

## Recursos

| Endereço | Conteúdo |
|---|---|
| `fenkra://arvore` | A peça em edição, em `.fkx` |
| `fenkra://relatorio` | Status de cada item, medidas e a última comparação |
| `fenkra://comandos` | A sequência de comandos da sessão |

## O que fica na pasta

Cada sessão guarda seus comandos em `comandos/sessao-NNN.jsonl`, e a sessão seguinte usa o próximo número. Esse
arquivo é o artefato: a conversa com o modelo muda, a sequência não. Para refazer a peça sem agente:

```bash
fenkra-cli --pasta projeto reproduzir comandos/sessao-001.jsonl --saida peca.fkx
```

## Segurança

- Sem execução de código: as ferramentas são uma lista fixa, e nenhuma aceita script, macro ou comando do sistema.
- Sem acesso fora da pasta, inclusive por ligação simbólica ou junção.
- Um STEP é lido num processo separado: se ele derrubar o leitor, a sessão continua.
- A peça montada pelo agente é calculada no processo do servidor. Uma falha grave do kernel nesse ponto encerra o
  servidor, mas o registro já gravado permite refazer a peça.

## Versões do protocolo

Atende clientes com aperto de mão `initialize` (2024-11-05 a 2025-11-25) e pedidos com versão própria em `_meta`
(2026-07-28), incluindo `server/discover`. O estado da peça vive no processo do servidor, um por cliente.

## O que esperar

Peças prismáticas com furos, bolsos e filetes são o caso bom. Peça orgânica ou montagem grande não é. O teste
automático reconstrói as peças com um agente de roteiro, não com um modelo: ele prova que as ferramentas bastam, não
que um modelo específico dê conta.
