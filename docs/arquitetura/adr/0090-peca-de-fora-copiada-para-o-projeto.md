# ADR 0090: peça de fora do projeto entra na montagem por uma cópia (B445)

## Contexto

Com um projeto aberto, o perfil de qualidade avalia a peça ao inserir na montagem (B305). As exceções ao perfil
moram no projeto, presas ao documento pelo uid. Uma peça de outra pasta não tem documento no projeto, então não
havia exceção possível: se o perfil a barrava, ela não entrava de jeito nenhum. Além disso, a montagem passava a
depender de um arquivo sem PN, sem versões e fora do diário.

O usuário decidiu: a peça de fora passa a ser permitida, com um aviso claro que oferece copiá-la para o projeto
(o padrão) ou cancelar.

## Decisão

1. **"É do projeto"** quer dizer: o arquivo é o de trabalho de um documento do índice, ou tem o uid de um deles
   (o mesmo documento guardado em outra pasta de trabalho continua sendo dele). `Project::pertence`.
2. **A cópia vai para `pecas/` com o nome do arquivo original.** Se o nome já existe na pasta ou em outro
   documento do índice, ganha `_2`, `_3`... O aviso mostra o nome novo antes da decisão e repete depois.
3. **A cópia é outro documento: uid novo.** Manter o uid deixaria dois arquivos com a mesma identidade, e o
   resolvedor de referências (B66) acharia "vários candidatos". É a mesma regra da cópia do Salvar como (B93).
4. **As dependências vão junto.** Os links (`external`) são seguidos até o fim da cadeia; o que já é do projeto
   não é copiado e o link passa a apontar para ele. Cada link reescrito recebe o caminho novo, relativo à pasta
   da cópia, e o uid novo da peça copiada. Malha importada e texturas são copiadas e gravadas com o caminho
   absoluto da cópia, porque o comando as lê pelo caminho como está. O furo que uma montagem manda para a peça
   aponta para a montagem, que não é dependência: só é reapontado.
5. **O histórico vem inteiro.** A cópia é lida e gravada pelo formato do documento; só caminhos e uids das
   referências mudam. A primeira versão de cada cópia tem a nota "copiada de <caminho do original>".
6. **PN:** o do arquivo é mantido se estiver livre no projeto (peça de catálogo ou comprada conserva o número);
   se outro documento já o usa, a cópia ganha o próximo da numeração e o aviso diz a troca.
7. **Tudo ou nada.** Se uma gravação falha, os arquivos criados são apagados e o índice volta ao que era.
8. Só peça passa pelo aviso. O subconjunto não passa pelo perfil de qualidade e segue entrando como antes.

## Consequências

- A peça copiada passa a ter exceções, versões e diário como qualquer documento do projeto.
- Editar a original depois não muda a montagem: ela usa a cópia. Quem quiser a ligação viva deve trazer a peça
  para o projeto antes de editá-la.
- `relacoes::referencias_do_json` não reconhece os links de peça (`path` + `uid` em `external`), por isso a
  cópia percorre o documento pelos tipos em vez de reaproveitar o plano do Salvar como.

## Conferência

`fenkra-project/src/copia_projeto.rs` (links, colisão de nome e PN, dependência que já é do projeto, malha) e
`fenkra-app/src/asm_inserir_projeto.rs` (aviso, cópia na mão, montagem recalcula ok com volume e caixa iguais
aos da original, colisão de nome, cancelar).
