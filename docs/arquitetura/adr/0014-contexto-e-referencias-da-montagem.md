# ADR 0014: Recursos, contexto e referências da montagem

- **Status:** aceita
- **Data:** 2026-11-05

## Contexto
Os itens B57, B54 e B55 mudam o que a montagem guarda. O furo entre peças precisa existir nas peças,
editar no contexto não pode sair do conjunto, e a montagem ganha geometria e publicações próprias.
A ADR 0011 pede essas regras, e esta registra onde cada dado mora.

## Decisão
1. **A montagem guarda a regra, e a peça guarda o resultado.** Uma série de furos fica em
   `AsmFeatKind::Serie`. Cada peça recebe um comando `Furo` comum, com um `ExternalRef` cuja publicação
   começa com `montagem:furo:`. Reaproveitar o link dá estado, Travar e Isolar sem campo novo no
   formato da peça.
2. **Esse link não entra no grafo de dependências.** Ele aponta para a montagem, que já usa a peça, e
   contá-lo fecharia ciclo em toda peça furada. A peça existe sem a montagem.
3. **Sincronizar compara estados.** Não há código para desfazer, suprimir ou tirar do escopo: a
   passada compara o que as séries pedem com o que as peças têm. Isolar se reconhece por sobrar o
   comando sem o link, e isso distingue o caso de um refazer, em que os dois somem juntos.
4. **O eixo é a referência.** Ele fica preso a um componente, a um ponto publicado, a cotas ou a um
   sketch da montagem. Referência que não resolve deixa a série em erro, e os furos não se movem.
5. **O contexto muda de sistema, não a peça.** A viewport mostra a peça nas coordenadas dela, então o
   resto do conjunto é levado ao sistema do ativo e entra na cena como camada translúcida.
6. **Peça nova mora no cache até o Salvar.** `PartCache` entrega a peça pendente como se o arquivo
   existisse, e toda gravação passa por ele. Só viram arquivo as pendentes que a montagem ainda usa.
7. **A geometria da montagem é um `Document` embutido** (`Assembly::geometria`). Editor de sketch,
   comandos de referência, recálculo e publicações valem sem cópia, e ela se edita como uma peça
   ativa. A alternativa era um modelo próprio de sketch e referências, que duplicaria tudo isso.
8. **Publicação é um nome.** Renomear deixa apelido em `Document::apelidos_publicacao`, trocar a
   geometria passa o nome a outro elemento, e remover mostra antes quem depende.
9. **Substituir planeja antes de mudar.** Vale a ordem nome, assinatura e proximidade. Proximidade é
   palpite e só entra confirmada. O que não casa fica suprimido e aparece na tabela.

## Consequências
- Arquivos antigos abrem sem migração: os campos novos têm padrão. O furo antigo (`AsmFeatKind::Hole`)
  continua valendo, agora com cilindro exato no lugar do prisma.
- Uma versão anterior do Fenkra não lê `.fka` com série de furos, porque a variante é nova e o
  `format_version` continua 1.
- O furo vale para todas as instâncias do arquivo, e a montagem avisa quando isso acontece.
- Furos de peças dentro de um subconjunto respondem ao subconjunto, não à montagem de cima.
- O furo só na montagem não passa pelos Fixadores, que leem a tabela de furos das peças.
