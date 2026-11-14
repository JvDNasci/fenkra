# Teste do servidor MCP com o Claude Code (13/11/2026)

Estado: **bloqueado, nenhum dos três pedidos foi executado por um modelo.**

## O que foi feito

- Binários conferidos: `fenkra-mcp.exe` (14:46) é mais novo que o último commit de `crates/fenkra-mcp` (11:26), então não houve recompilação.
- Config temporário (fora do repositório e das configurações do usuário): `{"mcpServers":{"fenkra":{"command":"F:/cargo-target/projeto-cad/release/fenkra-mcp.exe","args":["--pasta","<pasta de saída>"]}}}`, uma pasta por pedido (a, b, c); no pedido c, `--liberar-step` também.
- Comando: `claude -p --output-format stream-json --verbose --mcp-config mcp_a.json --strict-mcp-config --model sonnet --allowedTools "mcp__fenkra__*"`, com o pedido pela entrada padrão.

## Resultado

- O Claude Code iniciou e o servidor `fenkra` apareceu como `connected` (aperto de mão MCP ok).
- A chamada ao modelo falhou na hora: `Failed to authenticate: OAuth session expired and could not be refreshed` (`authentication_failed`, 0 tokens). O processo-filho herda `ANTHROPIC_BASE_URL` do ambiente desta sessão e não tem login renovado.
- Não contornei (não limpei variáveis nem refiz login): isso mexeria na autenticação do usuário.

## Pendências

1. Rodar de um terminal do usuário, já logado (`claude` interativo uma vez para renovar o OAuth), os três pedidos: placa 100x60x10 com 4 furos Ø8 a 10 mm das bordas (volume esperado 60000 - 4*pi*16*10 = 57985,5 mm3, caixa 100x60x10); suporte em L com filete; leitura de STEP.
2. Não há STEP no repositório (`*.step`/`*.stp`): gerar um com `fenkra-cli exportar placa.fkp placa.step` para o pedido c.
3. Conferir cada `.fkp` com `fenkra-cli recalcular`/`medir` e preencher este relatório.

## Parecer

B287 **não** pode ser marcado como feito por este teste.
