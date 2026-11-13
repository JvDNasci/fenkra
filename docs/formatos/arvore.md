# Árvore paramétrica de intercâmbio (`.fkx`), versão 1

Formato aberto para levar a **receita** de uma peça, e não só o corpo pronto: esboços com geometria,
restrições e cotas, comandos com parâmetros, nomes, cores e estrutura de montagem. O Fenkra lê e grava
este formato. Qualquer programa ou script pode gravá-lo seguindo esta página.

- **Esquema (JSON Schema):** [`arvore-v1.schema.json`](../../crates/fenkra-conversor/esquema/arvore-v1.schema.json).
  A leitura do Fenkra valida por esse mesmo arquivo.
- **Decisão:** [ADR 0035](../arquitetura/adr/0035-esquema-aberto-de-arvore-parametrica.md).
- **Scripts para outros programas:** o projeto não distribui nenhum por enquanto. Veja a
  [ADR 0034](../arquitetura/adr/0034-termos-das-apis-e-exportadores-por-script.md).

## Regras gerais

- Texto UTF-8 em JSON. Extensão `.fkx`.
- Medidas em milímetros, ângulos em graus. `unidades` é sempre `"mm"`: quem grava converte.
- Campos fora do esquema são erro. Um arquivo que a leitura aceita é um arquivo entendido por inteiro.
- Valores de parâmetros e comandos são **expressões em texto** (`"20"`, `"L*0.6"`), para continuarem
  ligados aos parâmetros. Coordenadas de esboço são números: são o estado resolvido, ponto de partida
  do solver.
- Ids são textos à sua escolha. Corpos, referências e itens têm id único na peça. Pontos e curvas têm id
  único dentro do seu esboço.
- O campo opcional `fora` lista, em texto, o que quem gravou não conseguiu levar. Ele avisa que a árvore
  está incompleta.

## Topo do arquivo

| Campo | Obrigatório | O que é |
|---|---|---|
| `formato` | sim | Sempre `"fenkra-arvore"` |
| `format_version` | sim | Versão do formato, hoje `1` |
| `unidades` | sim | Sempre `"mm"` |
| `pecas` | sim | Lista de peças, com pelo menos uma |
| `gerador` | não | Quem gravou |
| `montagem` | não | Estrutura de montagem |
| `fora` | não | O que ficou de fora |

Arquivo de versão mais nova que a do programa é recusado com aviso. Versões antigas sobem pela cadeia de
migração do programa, a mesma dos outros arquivos do Fenkra.

## Peça

`id`, `nome`, `arvore` (obrigatórios), `parametros`, `corpos`, `referencias`, `ativos` e `chapa`.

- **`parametros`:** `nome`, `expressao`, `grandeza` (`comprimento`, `angulo`, `numero`) e `comentario`. O nome
  começa com letra ou `_` e só tem letras ASCII, números e `_`.
- **`corpos`:** `id`, `nome`, `cor` (`[r, g, b]`, de 0 a 255). Sem corpos, tudo vai para um corpo só.
- **`ativos`:** quantos itens da árvore estão ativos, contando do início. Ausente, todos.
- **`chapa`:** presente, a peça é de chapa metálica. `espessura`, `raio` (interno de dobra) e `fator_k`
  (obrigatórios); `planificada` (a peça é calculada desdobrada), `material`, `metodo` (`fator_k`, `deducao` ou
  `tolerancia`, as duas últimas a 90° com o valor em `compensacao`), `alivio` e `alivio_forma` (`retangular`,
  `redondo`, `rasgo`) padrão das abas, e `canto`, o alívio de canto padrão. Quem grava leva os valores já
  resolvidos: estilo de chapa não vai para o arquivo.

## Planos, eixos e pontos

| Tipo | Formas aceitas |
|---|---|
| Plano | `"XY"`, `"XZ"`, `"YZ"` ou `{"referencia": id}` |
| Eixo | `"X"`, `"Y"`, `"Z"`, `{"esboco": id, "curva": id}`, `{"referencia": id}`, `{"origem": xyz, "direcao": xyz}`, `{"face": xyz}`, `{"aresta": xyz}`, `{"planos": [plano, plano]}` |
| Ponto | `[x, y, z]`, `{"referencia": id}`, `{"esboco": id, "ponto": id}` |
| Curva | `"X"`, `"Y"`, `"Z"`, `{"referencia": id}` (reta de referência), `{"esboco": id, "curva": id}` ou `{"pontos": [xyz, ...]}`, polilinha fixa que não segue o modelo |

## Referências

Cada uma tem `id`, `nome` e `definicao`, com `tipo`:

| `tipo` | Campos |
|---|---|
| `plano_deslocado` | `base` (plano), `deslocamento` |
| `plano_fixo` | `origem`, `x`, `normal`. Fica parado: não segue nenhuma face |
| `ponto` | `x`, `y`, `z` (expressões), `sistema` (plano, opcional) |
| `reta` | `ponto`, `direcao` (eixo), `inicio`, `fim` |
| `reta_pontos` | `a`, `b` (pontos); `inicio`, `fim`, prolongamento além de cada ponta |

## Itens da árvore

A ordem da lista é a ordem do histórico. Cada item tem `id`, `nome`, `comando` e, se precisar, `corpo`,
`suprimido`, `suprimir_se` (expressão), `oculto` e `cor`.

`operacao` é `unir`, `cortar` ou `intersectar`. `termino` é um objeto com `tipo`: `cego` e `simetrico`
(`distancia`), `dois_lados` (`a`, `b`), `atravessar`, `ate_plano` (`plano`), `ate_proxima`, `ate_ultima`.

| `comando.tipo` | Campos |
|---|---|
| `esboco` | ver abaixo |
| `extrusao` | `esboco`, `operacao`, `termino`; `inverter`, `saida`, `fina`, `direcao` |
| `revolucao` | `esboco`, `eixo`, `angulo`, `operacao`; `angulo2`, `simetrico`, `fina` |
| `furo` | `esboco` (os pontos dele são os centros), `forma`, `diametro`, `termino`; `inverter`, `rosca`, `ponta` |
| `filete` | `raio`, `arestas` |
| `chanfro` | `distancia`, `arestas` |
| `casca` | `faces` (um ponto sobre cada face aberta), `espessura`; `para_fora`, `fechada` |
| `primitiva` | `forma`, `plano`, `a`, `b`, `c`, `d`, `operacao` |
| `padrao` | `forma`; `fontes` (ids; vazio repete o corpo), `pular` (pares `[i, j]`) |
| `espelho` | `plano`; `fontes` |
| `parede_chapa` | `esboco` (perfil fechado); `inverter`, `deslocamento`, `meio`. Pede `chapa` na peça |
| `aba` | `aresta` (ponto sobre uma aresta reta da chapa), `comprimento`, `angulo`; `raio`, `recuo1`, `recuo2`, `inverter`, `alivio`, `alivio_forma`, `lado` (`dobra_fora`, `material_fora`, `material_dentro`), `meia_esquadria` |
| `bainha` | `aresta`, `comprimento`; `folga` (0 esmaga), `inverter`, `forma` (`fechada`, `gota`, `enrolada`) |
| `dobra` | `a`, `b` (pontos da linha sobre a face), `angulo`; `raio`, `inverter` (o outro lado gira), `para_baixo`, `esboco` (cada reta dele vira uma dobra), `posicao` (`centro`, `inicio`, `fim`, `vertice_externo`) |
| `membro` | `caminhos` (curvas), `secao`, `cotas`; `familia`, `designacao`, `ancoragem` (0 a 9), `angulo`, `deslocamento_x`, `deslocamento_y`, `prolongamento1`, `prolongamento2`, `canto` (`nenhum`, `meia_esquadria`, `aparar`, `encaixe`), `folga`, `material`, `espelhar` |
| `placa_extremidade` | `em` (ponto na face da ponta do membro), `espessura`, `sobra`; `lado` (`externo`, `interno`), `recuo`, `sobra_em` (`mm` ou `parede`), `furos` (`diametro`, `borda`) |
| `reforco` | `em` (ponto perto do encontro de dois membros), `aba`, `espessura`; `chanfro`, `aba2`, `trocar`, `lado` (`dois_lados`, `interno`, `externo`), `posicao` (`inicio`, `meio`, `fim`), `afastamento`, `repeticao` (`nenhuma`, `quantidade`, `espacamento`), `quantidade`, `espacamento` |

- `fina` é `{"espessura": expr, "lado": 0}`, com lado 0, 1 ou 2 (simétrica).
- `forma` do furo tem `tipo`: `simples`, `rebaixado` (`diametro`, `profundidade`), `escareado` (`diametro`,
  `angulo`), `contrafurado` (`diametro`, `profundidade`, `angulo`).
- `forma` da primitiva e o sentido de `a`, `b`, `c`, `d`, com `"0"` no que não se aplica: `caixa` (largura,
  profundidade, altura), `cilindro` (raio, altura), `esfera` (raio), `cone` (raio da base, raio do topo, altura),
  `toro` (raio maior, raio menor), `cunha` (X, Y, Z, X do topo), `prisma` (lados, raio, altura).
- `forma` do padrão tem `tipo`: `linear` (`direcao`, `quantidade`, `espacamento`; `direcao2`, `quantidade2`,
  `espacamento2`, `total`) ou `circular` (`eixo`, `quantidade`, `angulo`).
- `secao` do membro e a ordem das `cotas`: `tubo_quadrado` (lado, parede), `tubo_retangular` (largura, altura,
  parede), `tubo_redondo` (diâmetro, parede), `perfil_i`, `perfil_u` e `perfil_t` (altura, aba, alma, mesa),
  `cantoneira` (aba a, aba b, espessura), `barra_chata` (largura, espessura), `barra_redonda` (diâmetro),
  `barra_quadrada` (lado). `ancoragem` 0 é o centro da seção, 1 a 8 os pontos da caixa dela e 9 o centróide.
- Aba, bainha, placa e reforço apontam para a geometria por ponto, como o filete: o ponto vale no corpo
  daquele passo.

### Faces e arestas

`arestas` pode ser `"todas"`, `{"geradas_por": id}` ou uma lista de pontos `[x, y, z]`, um **sobre** cada aresta.
Numeração de face de outro programa não significa nada aqui: a referência é sempre por geometria.

O ponto pode ser qualquer um da aresta, a até 0,1 mm dela, mas não num vértice, onde duas arestas ficam à
mesma distância. A leitura acha a aresta no corpo daquele passo e passa a seguir essa aresta por dono e papel.
Se nenhuma aresta for claramente a mais próxima, o programa não escolhe: o comando abre em "referência
perdida", com aviso. O ponto médio da aresta é a escolha mais segura.

### Esboço

`plano`, `pontos`, `curvas` (obrigatórios), `restricoes`, `tangentes` e `posicao`.

- **`pontos`:** `id`, `x`, `y` e `fixo`. Curvas que compartilham um ponto usam o mesmo id: é assim que o perfil
  fecha.
- **`curvas`:** `id`, `construcao` e `geometria`, com `tipo`:

| `tipo` | Campos |
|---|---|
| `reta` | `a`, `b` |
| `circulo` | `centro`, `raio` (número) |
| `arco` | `centro`, `inicio`, `fim`, em sentido anti-horário |
| `ponto` | `ponto` |
| `spline` | `pontos`, passando por eles na ordem |
| `elipse` | `centro`, `inicio`, `fim`, `maior` (vetor), `menor`, `t0`, `t1` |

- **`restricoes`:** `tipo`, `entidades` (ids) e, nas cotas, `valor`. `referencia: true` é cota que só mede.
  `invertida: true` diz que a expressão vale menos que zero e a geometria já está do outro lado.

| Entidades | Restrições | Cotas (com `valor`) |
|---|---|---|
| 1 | `horizontal`, `vertical`, `fixa` | `comprimento`, `raio`, `diametro`, `angulo_reta` |
| 2 | `coincidente`, `paralela`, `perpendicular`, `igual`, `tangente`, `concentrica`, `ponto_medio`, `sobre_curva`, `horizontal_pontos`, `vertical_pontos` | `distancia`, `distancia_x`, `distancia_y`, `angulo`, `angulo_suplementar`, `distancia_ponto_reta` |
| 3 | `simetrica` | |

## Montagem

`nome` e `componentes`. Cada componente tem `id`, `nome`, `origem`, `rotacao` (matriz 3 × 3, linha a linha) e, se
precisar, `peca` (id da peça; sem ela é um grupo), `pai` (id de um componente que vem **antes** na lista), `fixo`
e `cor`. Posições valem no sistema da montagem. Restrições entre peças não entram na versão 1.

## Exemplo mínimo

```json
{
  "formato": "fenkra-arvore",
  "format_version": 1,
  "unidades": "mm",
  "pecas": [{
    "id": "placa",
    "nome": "Placa",
    "parametros": [{ "nome": "L", "expressao": "40", "grandeza": "comprimento" }],
    "arvore": [
      { "id": "s1", "nome": "Perfil", "comando": {
        "tipo": "esboco", "plano": "XY",
        "pontos": [
          { "id": "a", "x": 0, "y": 0 }, { "id": "b", "x": 40, "y": 0 },
          { "id": "c", "x": 40, "y": 20 }, { "id": "d", "x": 0, "y": 20 }
        ],
        "curvas": [
          { "id": "l1", "geometria": { "tipo": "reta", "a": "a", "b": "b" } },
          { "id": "l2", "geometria": { "tipo": "reta", "a": "b", "b": "c" } },
          { "id": "l3", "geometria": { "tipo": "reta", "a": "c", "b": "d" } },
          { "id": "l4", "geometria": { "tipo": "reta", "a": "d", "b": "a" } }
        ],
        "restricoes": [{ "tipo": "comprimento", "entidades": ["l1"], "valor": "L" }]
      } },
      { "id": "e1", "nome": "Corpo", "comando": {
        "tipo": "extrusao", "esboco": "s1", "operacao": "unir",
        "termino": { "tipo": "cego", "distancia": "5" }
      } },
      { "id": "f1", "nome": "Canto", "comando": {
        "tipo": "filete", "raio": "2", "arestas": [[0, 0, 2.5]]
      } }
    ]
  }]
}
```

## O que a versão 1 não leva

Varredura, multi-seções, superfícies e outros comandos fora da tabela; na chapa, aba por perfil, curva ou
superfície, recorte, estampo, veneziana, furo flangeado, cantos, desdobrar parcial, parede a partir de
superfície e ordem de dobra escolhida à mão; no estrutural, solda, dobra de tubo no nó, regra própria por canto
e reforço de contorno livre; término até superfície;
início deslocado e segundo término de extrusão; furo com pontos 3D, direção própria, conicidade ou rosca modelada;
filete variável e chanfro de duas medidas; padrão por pontos; geometria externa de esboço; parâmetros que não são
número; posição do texto das cotas; acabamento e textura; pastas; PMI; restrições de montagem.

Ao gravar, o Fenkra lista cada comando desses, e quem dependia dele, no campo `fora` e no aviso da tela. O plano
de uma face vira `plano_fixo`, na posição gravada no documento, e deixa de seguir a face. Do mesmo jeito, a
aresta do modelo que guia um membro vira `{"pontos": [...]}`.
