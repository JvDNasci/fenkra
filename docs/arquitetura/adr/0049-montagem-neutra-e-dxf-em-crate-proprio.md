# ADR 0049: montagem neutra com peças compartilhadas e DXF em crate próprio

Estado: aceita. Itens B104 e B105. Complementa o ADR 0023.

## Contexto
O ADR 0023 deixou duas pendências: a peça com N ocorrências virava N corpos, cada um com a própria cópia do
B-rep, e o relatório só existia como texto na tela. O OpenCASCADE traduz IGES, mas não traduz DXF. Quem grava
DXF é o desenho (`fenkra-desenho`) e a planificada (`fenkra-solidos`), que ficam abaixo do conversor na ordem
de dependência, e os dois tinham um gravador R12 próprio.

## Decisão
1. **Destino da importação.** `OpcoesImportar::destino` escolhe entre `Montagem` (padrão) e `UmaPeca`. Com
   `Montagem`, um STEP com mais de uma ocorrência vira `Conjunto`: um documento por produto e uma `Assembly`
   com um componente por ocorrência. A mesma estrutura do arquivo de árvore, para a tela e o projeto terem um
   caminho só. Um STEP com uma peça embrulhada em montagem continua sendo um documento.
2. **Subconjunto vira grupo** com as posições somadas, como no arquivo de árvore. A árvore fica igual e a pasta
   não ganha um `.fka` por nível.
3. **PN pelo motor.** O conversor não conhece o projeto: quem dá PN é `Project::adicionar_pecas_importadas`,
   uma vez por produto, e a tela troca os caminhos dos nós pelos arquivos do projeto.
4. **Exportar.** `ModeloNeutro::da_montagem` refaz produtos e instâncias a partir dos arquivos. `Achatada` grava
   um produto só e leva a cor de cada ocorrência como cor de face, porque um produto só tem uma cor.
5. **DXF em `fenkra-dxf`.** Crate sem dependência nenhuma, em Rust. Fica abaixo de quem grava e de quem lê, e um
   arquivo ruim vira `Err`, não uma queda: por isso a leitura de DXF não precisa do processo filho. Blocos têm
   teto de profundidade e de inserções, porque um arquivo pequeno pode pedir bilhões de entidades.
6. **Gravação em R2000.** É a primeira versão com polilinha leve, hachura e texto de várias linhas, e substitui
   os dois gravadores R12. Tudo fora do ASCII sai em `\U+XXXX`, para não depender da página de código de quem lê.
7. **Arcos de volta.** O desenho entrega curvas como pontos. `fenkra_dxf::reconhecer` devolve `ARC` e `CIRCLE`
   onde o trecho é arco amostrado. Vértices num círculo não bastam, porque um retângulo tem isso: cada corda
   precisa girar pouco e ter flecha na ordem da tolerância.
8. **Camada vira sketch.** O sketch não tem camadas: cada camada vira um sketch com o nome dela, e a opção
   Camadas desligada junta tudo em um.
9. **Relatório.** Vai embutido no selo do documento em JSON, sai em HTML e JSON, e traz antes e depois do reparo
   com a lista de defeitos e o traço de cada um, para a tela realçar sem o kernel.
10. **IGES atrás da mesma trava do STEP.** Tradutor não reentrante, e os dois dividem `Interface_Static`. O modo
    BRep ou Faces e a unidade só existem como `Interface_Static`, lidos no construtor do gravador.

## Consequências
- Um conjunto no projeto grava `montagens/<nome>.fka`, com caminhos para `pecas/`.
- Reimportar um conjunto com o mesmo conteúdo abre a montagem existente. Com conteúdo novo, ele entra como
  conjunto novo: atualizar peça a peça ainda não existe.
- O relatório fica embutido só na primeira peça do conjunto. Em todas, mil peças levariam mil cópias dele.
- O R2000 não guarda cor verdadeira: a cor de um item vai para o índice mais perto entre os nove fixos.
- A leitura de DXF traz linhas, arcos, círculos, polilinhas, elipses, splines e pontos. Texto, hachura e cota
  ficam de fora do sketch e vão para o relatório com a contagem.
