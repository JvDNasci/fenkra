# Automação sem interface: comandos e contrato JSON, versão 1

O mesmo conjunto de comandos atende à linha de comando (`fenkra-cli`), à [API Python](../../python/fenkra/__init__.py) e
ao [servidor MCP](../automacao/mcp.md). Nenhum deles interpreta código: há uma lista fechada de comandos, com dados em
JSON. **Decisão:** [ADR 0038](../arquitetura/adr/0038-automacao-e-servidor-mcp.md).

## Regras que valem para todos

- **Pasta:** todo caminho fica dentro da pasta informada (`--pasta`). Sai com `caminho_fora` qualquer `..`, caminho de
  outra unidade, fluxo alternativo, nome de dispositivo e ligação ou junção que leve para fora.
- **Sobrescrever:** nada troca um arquivo que existe sem `sobrescrever: true` (`--sobrescrever`). O arquivo aparece
  inteiro ou não aparece: a gravação passa por um provisório ao lado do destino.
- **Unidades:** milímetros e graus. Valores saem arredondados a 1e-6.
- **Estabilidade:** campo novo não muda a versão. Mudar o sentido de um campo, ou tirá-lo, muda `api`.

## Resposta

```json
{ "api": 1, "ok": true, "resultado": { } }
{ "api": 1, "ok": false, "erro": { "codigo": "ja_existe", "mensagem": "texto para gente" } }
```

Decida pelo `codigo`, nunca pela `mensagem`.

| `codigo` | Quando |
|---|---|
| `argumento` | Falta campo ou o tipo está errado |
| `comando_desconhecido` | O comando não existe |
| `caminho_fora` | O caminho sai da pasta ou não é permitido |
| `ja_existe` | O arquivo existe e `sobrescrever` não veio |
| `formato` | Extensão não aceita, ou arquivo de outro formato ou versão |
| `leitura`, `gravacao` | Falha ao abrir ou gravar |
| `sem_peca`, `sem_solido`, `sem_historico` | Nenhuma peça aberta, peça sem sólido, nada a desfazer |
| `arvore` | O comando não passa no esquema do `.fkx` ou não monta. A árvore fica como estava |
| `arvore_incompleta` | A peça tem comandos fora do `.fkx`: mede, muda parâmetro e exporta, mas não recebe item |
| `parametro` | A expressão não avalia. O parâmetro fica como estava |
| `kernel` | O OpenCASCADE recusou a operação |
| `queda`, `tempo` | O leitor isolado caiu ou passou do limite. A sessão continua |
| `step_bloqueado` | Só no MCP: falta o aceite `--liberar-step` |

Um comando que entra na árvore mas falha no cálculo não é erro da chamada: a resposta traz `status: "erro"` e a
`mensagem`, e `desfazer` volta. Assim a pessoa vê o que falhou.

## Comandos

| Comando | Argumentos | `resultado` |
|---|---|---|
| `versao` | | `api`, `programa`, `kernel` |
| `comandos` | | `comandos` |
| `nova_peca` | `nome` | `nome` |
| `abrir` | `arquivo` (`.fkx`, `.fkp`) | `arquivo`, `editavel`, `fora` |
| `salvar` | `arquivo` (`.fkx`, `.fkp`), `sobrescrever` | `arquivo`, `fora` |
| `exportar` | `arquivo` (`.step`, `.stp`), `sobrescrever` | `arquivo` |
| `arvore` | | o arquivo `.fkx` inteiro |
| `parametros` | | `parametros`: `nome`, `expressao`, `valor`, `grandeza`, `comentario` |
| `definir_parametro` | `nome`, `expressao`, `grandeza` | `nome`, `expressao`, `valor` |
| `criar_plano` | `deslocamento`, `base`, `id`, `nome` | `id` |
| `criar_esboco` | campos de `esboco` no [`.fkx`](arvore.md), `id`, `nome` | `id`, `status`, `graus_de_liberdade`, `convergiu`, `volume` |
| `extrudar`, `furar`, `filetar`, `chanfrar` | campos do comando no `.fkx`, `id`, `nome` | `id`, `status`, `mensagem`, `volume` |
| `desfazer` | | `itens` |
| `recalcular` | | `status`, `itens`, `corpos`, `volume`, `area`, `faces`, `caixa`, `erro_de_parametro` |
| `relatorio` | | o de `recalcular`, mais `comparacao` e `registro` |
| `medir` | | `contagem`, `caixa`, `massa` |
| `ver` | `vista` (`frente`, `tras`, `topo`, `base`, `esquerda`, `direita`, `iso`, `todas`), `corte` (`eixo`, `posicao` em mm, `lado` `menor` ou `maior`), `zoom`, `largura`, `altura` (64 a 2400), `arestas`, `fundo`, `saida` (`.png`), `sobrescrever` | `vista`, `largura`, `altura`, `caixa`, `corte`, `aviso` e `arquivo`; sem `saida`, `imagem_base64` e `tipo` (PNG) |
| `topologia` | `arquivo` (ausente: a peça aberta), `secoes`, `limite` | ver abaixo |
| `comparar` | `alvo`, `arquivo` (ausente: a peça aberta), `tolerancias` | ver abaixo |
| `reproduzir` | `arquivo` (`.jsonl`) | `comandos` |
| `registro` | | `arquivo`, `comandos` |
| `adicionar` | `tipo` (qualquer `comando.tipo` do [`.fkx`](arvore.md): `revolucao`, `casca`, `primitiva`, `padrao`, `espelho`, `parede_chapa`, `aba`, `bainha`, `dobra`, `membro`, `placa_extremidade`, `reforco`...), os campos dele, `id`, `nome` | `id`, `status`, `mensagem`, `volume` |
| `selecionar` | `alvo` (`arestas`, `faces`), `lado`, `extrema`, `tipo`, `raio`, `perto`, `quantas` | `quantidade`, `pontos` |
| `biblioteca` | `busca`, `familia`, `tamanho`, `furo` | famílias ou tamanhos com medidas |
| `verificar` | `requisitos` | `ok`, `falhas`, `resultados` |
| `definir_chapa` | campos de `chapa` no `.fkx` (`espessura`, `raio`, `fator_k`, `planificada`, `material`, `metodo`, `compensacao`, `alivio`, `alivio_forma`, `canto`); só os informados mudam | `chapa`, `status`, `volume`, `caixa` |
| `nova_montagem` | `nome` | `nome` |
| `inserir_componente` | `arquivo` (`.fkp`), `id`, `nome`, `origem`, `rotacao` (3 × 3, linha a linha), `fixo` | `id`, `nome`, `arquivo`, `fixo` |
| `restringir` | `tipo` (`coincidente`, `contato`, `distancia`, `angulo`, `paralela`, `perpendicular`), `a` e `b` (`componente` com `face`, `plano`, `eixo` ou `ponto`), `valor`, `invertida`, `alinhamento`, `lado_invertido`, `id` | `id`, `status`, `convergiu`, `residuo`, `graus_de_liberdade`, `falhas` |
| `mover_componente` | `componente`, `origem`, `rotacao`, `deslocar`, `girar` (`eixo`, `angulo`, `centro`) | `origem`, `rotacao`, `fixo` e o relatório do solucionador |
| `montagem` | | `componentes` (posição, graus de liberdade, volume, caixa), `restricoes`, `solucao`, `volume`, `caixa` |
| `bom` | `nivel` (`topo`, `pecas`, `estruturada`) | `linhas` (`item`, `nome`, `quantidade`, `arquivo`, `pn`, `volume_unitario`), `itens`, `pecas` |
| `salvar_montagem` | `arquivo` (`.fka`, `.fkx`), `sobrescrever` | `arquivo`, `fora` |

## Topologia

`secoes` escolhe entre `caixa`, `massa`, `faces`, `arestas`, `planos` e `cilindros`; sem ela vêm todas. `contagem` sempre vem.
`limite` corta faces e arestas (padrão 2000), e `cortado: true` avisa.

| Seção | Campos |
|---|---|
| `caixa` | `min`, `max`, `tamanho` |
| `massa` | `volume`, `area`, `centroide`, `inercia` (momentos principais em ordem crescente, densidade 1) |
| `faces` | `indice`, `tipo` (`plano`, `cilindro`, `cone`, `esfera`, `toro`, `livre`), `area`, `ponto` (sobre a face), `direcao`, `arestas`, `raio` |
| `arestas` | `indice`, `tipo`, `faces`, `meio`; `reta`: `a`, `b`; `arco` e `circulo`: `centro`, `raio`, `normal`, `inicio`, `fim`, `angulo` |
| `planos` | `normal`, `deslocamento`, `area`, `faces`: faces de mesmo plano reunidas |
| `cilindros` | `face`, `origem`, `eixo`, `raio`, `diametro`, `inicio`, `fim`, `interno`, `area` |
| `furos` (vem com `cilindros`) | `diametro`, `eixo`, `inicio`, `fim`, `comprimento`, `faces` |

Em `faces`, `direcao` é a normal para fora no plano e o eixo em cilindro, cone e toro. Um furo é um grupo de faces
cilíndricas voltadas para o eixo, de mesmo eixo e raio, que fecha a volta: um STEP costuma trazê-lo em duas metades.
Canto arredondado por dentro tem `interno: true`, mas não é furo.

## Comparação

Os dois corpos valem no mesmo sistema de coordenadas: não há alinhamento automático.

```json
{ "aprovado": false,
  "medidas": [{ "nome": "volume", "alvo": 100, "modelo": 90, "desvio": 0.1, "tolerancia": 0.005, "ok": false }],
  "furos": { "ok": false, "alvo": 2, "modelo": 1, "faltam": [], "sobram": [], "deslocados": [], "diametro_errado": [] },
  "distancia": { "maxima": 4.2, "media": 0.3, "onde": [0, 0, 8], "fracao": 0.03, "tolerancia": 0.01, "ok": false } }
```

`medidas` traz `volume`, `area`, `centroide`, `caixa` e `inercia`. `distancia` é o maior afastamento entre as superfícies,
amostrado nos dois sentidos sobre a malha, e `onde` é o ponto.

| Tolerância | Padrão | Sentido |
|---|---|---|
| `volume`, `area`, `inercia` | 0,005; 0,01; 0,01 | Fração do valor do alvo |
| `centroide`, `caixa`, `distancia` | 0,005; 0,005; 0,01 | Fração da diagonal do alvo |
| `furo_diametro`, `furo_posicao` | 0,01; 0,05 | Milímetros |

## Registro de comandos

Com o registro ligado (sempre, no MCP), cada comando que muda o modelo vira uma linha em `comandos/sessao-NNN.jsonl`, na
hora. A primeira linha é `{"formato": "fenkra-comandos", "format_version": 1, ...}`; as outras, `{"comando": ..., "args": ...}`.
`reproduzir` refaz a peça sem agente nenhum e recusa qualquer comando que não mude o modelo.

## Linha de comando

```bash
fenkra-cli --pasta projeto recalcular suporte.fkx
fenkra-cli --pasta projeto definir suporte.fkx L 120 --saida suporte-120.fkx
fenkra-cli --pasta projeto exportar suporte-120.fkx suporte-120.step
fenkra-cli --pasta projeto cilindros cliente.step
fenkra-cli --pasta projeto comparar cliente.step suporte-120.fkx
```

Código de saída: 0 ok, 1 erro do comando (com o JSON de erro), 2 uso errado. `fenkra-cli servir` atende um pedido por
linha, `{"id": 1, "comando": "versao", "args": {}}`, e devolve a resposta com o mesmo `id`.

## Python

Só biblioteca padrão. Copie a pasta `python/fenkra` para o projeto, ou ponha `python/` no `PYTHONPATH`. O programa vem do
argumento `programa`, da variável `FENKRA_CLI` ou do `PATH`.

```python
from fenkra import Fenkra, FenkraErro

with Fenkra("projeto") as f:
    f.abrir("suporte.fkx")
    f.definir_parametro("L", 120)
    print(f.medir()["massa"]["volume"])
    print(f.furos("cliente.step"))
    f.exportar("suporte-120.step")
```

Falha vira `FenkraErro`, com `codigo` e `mensagem`. `f.chamar(comando, **args)` alcança qualquer comando da tabela.

## Erros acionáveis

Todo erro vem com `codigo`, `mensagem` e `sugestao` (o que fazer em seguida). O item que falha no
cálculo (resposta de `extrudar`, `furar`, `filetar`, `chanfrar` e cada item de `recalcular`) traz também
`codigo` (`E-REF-001`, `E-DEP-001`, `E-SOL-001`, `E-SUP-001`, `E-SKT-001`, `E-VAL-001` ou `E-GER-001`), `item`, `campo` (quando
a mensagem cita um), `sugestao` e `acao` (`refazer_referencia`, `corrigir_antes`, `ajustar_valor` ou `refazer_contorno`);
com `corrigir_antes`, vem `corrigir_primeiro`. São os mesmos códigos do Assistente de reparo da interface.
