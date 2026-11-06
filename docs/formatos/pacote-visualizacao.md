# Pacote de visualização `.fkv` (versão 1)

O navegador não roda o kernel, então o arquivo nativo do Fenkra não abre sozinho na web. O `.fkv`
(Fenkra Visualização) é o arquivo de leitura que o Fenkra gera da peça, da montagem ou do desenho, e que
o visualizador web abre sem instalar nada. É um formato aberto, só de leitura, sem geometria exata de
projeto: não volta para o CAD.

Quem gera: `fenkra-pacote` (menu Arquivo > Exportar visualização, ou `fenkra-cli visualizar`). Quem lê: a
página `crates/fenkra-pacote/web/visualizador.html` e o leitor de referência em Rust
(`Pacote::ler` e `Pacote::validar`).

## Contêiner

ZIP comum (métodos 0 e 8, sem criptografia, sem ZIP64). Os nomes são fixos:

| Arquivo | Conteúdo |
|---|---|
| `pacote.json` | manifesto (abaixo); sempre o primeiro |
| `cena.glb` | glTF 2.0 binário com as malhas e a hierarquia; ausente em desenho |
| `folhas/f<N>.json` | uma folha de desenho em vetor, uma por folha |

A geração é função do documento: o mesmo documento dá o mesmo arquivo, byte a byte.

## Versão

`formato` vale `fenkra-visualizacao` e `versao` é um inteiro. Campo novo não muda a versão; mudar ou tirar
campo muda. Um leitor da versão N recusa pacote de versão maior e avisa o motivo; ignora campo que não
conhece. Esta é a versão 1.

## Cena (`cena.glb`)

glTF 2.0 válido em qualquer leitor, com a extensão `KHR_mesh_quantization` obrigatória (normais em 8
bits, índices em 16 bits quando cabem). As posições ficam em **milímetros, com Z para cima**, como no
modelo. Para o arquivo continuar correto em leitores genéricos, o nó `fkv:raiz` (marcado com
`extras.fkv_raiz`) leva a matriz que passa para metros e gira Z para Y. O visualizador do Fenkra ignora
essa raiz.

- Cada nó tem `extras.fkv` com o **ID estável**: o mesmo texto do campo `id` da árvore do manifesto.
- Cada malha tem `extras.fkv` com o id da malha (`m<N>`), que aparece em `malhas` no manifesto.
- Peça usada em vários lugares é uma malha só, com um nó por ocorrência (a malha entra uma vez).
- As matrizes dos nós de peça já são as do mundo (os nós de grupo têm identidade).
- Com `decimado` no manifesto, a malha vem sem normais e o leitor calcula.

## Manifesto (`pacote.json`)

Campos de topo:

| Campo | Tipo | Significado |
|---|---|---|
| `formato`, `versao` | texto, inteiro | identificação |
| `gerador` | texto | programa e versão que gerou |
| `tipo` | `peca`, `montagem`, `desenho` | o que o pacote mostra |
| `nome`, `unidade` (`mm`), `eixo_para_cima` (`Z`) | texto | |
| `cena` | texto ou nulo | caminho do GLB |
| `atributos` | objeto | da peça, montagem ou desenho (abaixo) |
| `arvore` | lista | nós, em ordem de profundidade |
| `malhas` | lista | uma por corpo, na ordem das malhas do GLB |
| `pmi` | lista | anotações 3D e MBD, com alvos |
| `capturas` | lista | capturas de vista das anotações |
| `vistas` | lista | câmeras salvas |
| `estados` | lista | estados de exibição da montagem |
| `analise` | objeto | interferências e folgas da montagem já calculadas (abaixo); só em `montagem` |
| `lista` | lista | lista de peças da montagem (item, quantidade, nome, arquivo) |
| `folhas` | lista | folhas do desenho (id, nome, arquivo, largura, altura, formato) |
| `tabelas` | lista | texto das tabelas do desenho (cabeçalho e linhas) |
| `estatisticas` | objeto | peças únicas, instâncias, triângulos |
| `decimado` | número ou nulo | fração de triângulos mantida, se houve decimação |
| `avisos` | lista de texto | o que o gerador não conseguiu levar |

### `arvore`

Cada entrada: `id` (estável), `pai` (id ou nulo), `nome`, `tipo` (`peca`, `corpo`, `grupo`, `montagem`),
`no` (índice do nó no GLB), `malha` (id da malha ou nulo), `visivel`, `cor` (RGB, quando há), `atributos`.

**IDs estáveis.** Vêm dos IDs do documento: `n<id>` para componente de montagem, `c<id>` para corpo
(coletor) dentro da peça, com o ID da peça na frente (`n3/c1`). Dentro de um subconjunto, o prefixo é o ID
do componente do subconjunto (`n7/n2`). O mesmo documento gera sempre os mesmos IDs, e os de PMI
(`p<id>`) e capturas (`k<id>`) também.

### `atributos` da peça

`nome`, `pn`, `descricao`, `material`, `tipo_de_peca`, `massa_kg`, `volume_mm3`, `area_mm2`,
`caixa_mm` (mínimo e máximo), `dimensoes_mm`, `centro_de_massa_mm`, `personalizados`. A massa usa a
densidade do material de cada corpo.

### `malhas`

`id`, `nome`, `triangulos`, `vertices`; e, sem decimação, `faces` (início de cada face, em índices) e
`tipos_de_face` (tipo 0 plano, 1 cilindro, 2 cone, 3 esfera, 4 toro, 5 livre, e a área). Com a opção de
capturas ligada (padrão sem decimação), `capturas`:

- `pontos`: `[x, y, z, tipo]` em mm e precisão dupla, na referência da peça; tipo 0 vértice, 1 ponto
  médio, 2 centro de círculo;
- `circulos`: `[cx, cy, cz, raio, nx, ny, nz]`.

Saem da fórmula de cada aresta do B-rep, não da malha. Por isso a medida entre dois deles no visualizador
é a do CAD, sem kernel (arredondamento de 1e-9 mm no arquivo).

### `pmi`

`id`, `nome`, `tipo`, `dimensional`, `texto` (já com prefixo e tolerância), `valor` (medido, quando há),
`ancoras`, `deslocamento` do texto, `alvos` (um por âncora, ou nulo; cada um com `classe`, `ponto`,
`direcao`, `raio`, `furo`), `no`, `visivel`, `pasta`. Quadros de tolerância, referências, rugosidade e
solda levam o texto pronto em `texto`.

### `analise` (montagem)

Calculada pelo Fenkra na hora de gerar, porque o navegador não tem kernel. Campos: `calculada`
(`false` com `motivo` quando não deu: sem kernel ou mais de 150 componentes de topo), `tolerancia_mm3`,
`limite_folga_mm`, `interferencias` e `folgas`.

- Interferência: `a` e `b` (ids da árvore), `nome_a`, `nome_b` e `volume_mm3` do sólido comum.
- Folga: `a`, `b`, `nome_a`, `nome_b`, `distancia_mm` e `metodo`. Vale entre peças que não interferem e
  estão a menos de `limite_folga_mm`. O método `vertices` mede uma amostra de vértices de cada peça contra
  o sólido da outra: é um limite superior da folga real, exato entre faces planas e cilindros.

Só entram componentes de topo (id `n<id>`); um subconjunto conta como uma peça.

### `capturas`, `vistas`, `estados`

- Câmera: `alvo`, `yaw`, `pitch` (radianos), `distancia`, `ortografica`. A câmera olha o alvo de
  `alvo + distancia * (cos(pitch) cos(yaw), cos(pitch) sin(yaw), sin(pitch))`, com Z para cima.
- Captura: `id`, `nome`, `camera`, `pmi` (ids visíveis), `ocultos` (IDs numéricos do documento, que o leitor
  casa com `n<id>` ou `c<id>`), `com_modelo`.
- Estado: `nome`, `ocultos` (ids da árvore), `cores`, `deslocamentos` (explosão, mm no mundo), `camera`.

## Folhas (`folhas/f<N>.json`)

Vetor nativo, em milímetros de papel, com Y para cima e origem no canto inferior esquerdo. Não é PDF nem
imagem. Campos: `id`, `nome`, `largura`, `altura`, `formato`, `vistas` (id, nome, `escala`, `caixa`,
`origem`) e `itens`. Cada item tem `t`:

| `t` | Campos |
|---|---|
| `l` linha | `p` (x, y, x, y, ...), `e` (0 visível, 1 fina, 2 oculta, 3 centro, 4 grossa), `w` espessura opcional |
| `f` polígono cheio | `p` |
| `x` texto | `p` (linha de base), `s`, `h` (altura), `a` (graus), `al` (0 esquerda, 1 centro, 2 direita), `b` negrito |

Opcionais em qualquer item: `c` cor própria, `o` id do objeto do desenho, `er` objeto com problema. Cotas,
carimbo e tabelas já vêm como linhas e textos.

## O visualizador no PLM (B339)

A página `visualizador.html` tem, além do que está acima, as abas Zonas, Montagem e Comparar:

- **Zonas**: região do produto em caixa (modo "totalmente dentro" ou "toca"), faixa de volume, material,
  texto; lista com massa e volume somados; isolar e destacar o resultado; peças por faixa de um eixo.
- **Montagem** (só em `tipo` `montagem`): resumo, interferências e folgas de `analise`, e onde é usado
  (peças com o mesmo código ou a mesma malha, com o caminho na árvore).
- **Comparar**: dois pacotes pelos IDs estáveis. Lista adicionados, removidos, movidos (matriz do nó),
  alterados (nome, código, material, massa, volume, área, triângulos, visibilidade), mudança nos totais e
  interferências que surgiram ou sumiram. Pinta o que o pacote aberto tem de diferente e troca de lado.

Entradas extras da página: segundo pacote embutido (`pagina_html_completa`), `?cmp=URL` junto com
`?src=`, e `postMessage({fkv, comparar, meta})`. `meta` é opcional e leva `estado`, `revisao`, `versao`,
`pn` e `comparando`; aparece ao lado do título. Ele não vai no pacote porque o estado muda depois.

## Segurança

O pacote não carrega código. O visualizador só lê dados, escapa todo texto antes de pôr na página e não
faz pedido de rede por conta própria (só busca o `.fkv` quando recebe uma URL).

## Conformidade

O teste `crates/fenkra-pacote/tests/corpus.rs` gera pacote de peça, montagem e desenho, abre pelo leitor de
referência, valida as referências (nós, malhas, folhas, IDs únicos, pais) e confere números contra o
documento. A página foi aberta em Chromium nas mesmas amostras (ver o ADR 0062).
