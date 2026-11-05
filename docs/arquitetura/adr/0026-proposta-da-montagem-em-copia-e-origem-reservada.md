# ADR 0026 — Proposta da montagem em cópia e origem como id reservado

## Contexto

Restrição e junta nasciam no segundo clique, e o erro só aparecia depois de aplicado. O B244 pede prévia e
confirmar, com o aceite "cancelar deixa a montagem idêntica". A ADR 0017 resolve a prévia das peças com o
comando aberto dentro do documento, porque ali o recálculo já roda em outro fio e com cache. Na montagem isso
não vale: o solucionador é rápido, roda na thread da interface e grava as posições na própria `Assembly`.

O B22 pede origem própria da montagem, usada para restringir. Uma restrição liga dois `MateRef`, e cada um
aponta para um componente.

## Decisão

**Proposta fora da montagem.** `fenkra_montagem::proposta::Comando` guarda os elementos, o tipo e os parâmetros.
`resolver` clona a montagem, acrescenta o item e resolve o clone. A montagem real só muda quando a cópia toma o
lugar dela, ao aceitar. O fantasma é a diferença de posição entre as duas.

**Origem como id reservado.** `ORIGEM = Id(0)` entra em `MateRef.comp` e `JRef.comp`. `Assembly::lugar` e `place`
respondem por ele com a posição identidade. Ele não é nó: os ids de nós começam em 1.

## Alternativas descartadas

- Criar o item na montagem e usar o desfazer para cancelar: cancelar mexeria no histórico, e a montagem passaria
  por estados que ninguém aceitou, com efeito nos links e nos furos entre peças.
- Comando aberto dentro da `Assembly`, como nas peças: exigiria marcar o item como provisório em todo lugar que
  lê restrições.
- Origem como nó `NodeKind::Origem`: todo `match` de nós, a lista de peças, a numeração e a árvore precisariam
  ignorá-lo, e ele teria posição editável.
- Origem como componente fixo escondido: cada montagem antiga ganharia um nó na abertura, e ele apareceria nos
  arquivos e nos subconjuntos.

## Consequências

- Cada mudança na proposta clona a montagem e resolve duas vezes (a segunda separa o que já falhava antes). Numa
  montagem muito grande isso vai pesar; a conta só roda quando a proposta muda.
- Quem lê a posição de uma referência usa `lugar`, não `node`: `node(ORIGEM)` é `None`.
- Restrição com a origem atravessa subconjunto como as outras: é religada ao nó do subconjunto.
