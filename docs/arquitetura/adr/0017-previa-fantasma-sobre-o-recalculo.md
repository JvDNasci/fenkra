# ADR 0017 — Prévia fantasma usa o recálculo do comando aberto

## Contexto

Todo comando precisa mostrar o resultado translúcido antes de confirmar (B223, B226, B319). A função
`previa_comando` já devolvia malha, volume e status de um comando de chapa, e parecia o caminho natural.

Só que o comando aberto já está no documento: `App::command` acrescenta a feature e `Session` guarda a cópia
para cancelar. O fio `recompute` calcula esse documento com cache, a cada mudança de campo. Rodar
`previa_comando` junto faria a mesma conta duas vezes, a segunda sem cache e em outro fio. Ela também põe o
comando no fim da árvore, o que erra ao reabrir um comando do meio.

## Decisão

A prévia muda como o resultado aparece, sem calcular nada a mais. As faces cujo dono é o comando aberto saem
da malha opaca e vão para a camada `Scene::ghost`, na cor de destaque do tema (`previa.rs`).

- Fora do fio da interface: é o fio de recálculo que já existia, que descarta pedidos velhos.
- Se demorar: `App::result` só troca quando o novo chega, então a tela fica com o último resultado.
- Se falhar: a causa vem do status do comando e aparece numa etiqueta sobre a viewport.
- Seleção: `picking::Filter::fantasma` tira do clique as faces do comando, as arestas que só tocam nelas e
  os vértices que só essas arestas usam.
- Comandos que refazem o corpo inteiro (dobra por linha, joggle, veneziana) marcam todas as faces como
  suas. Nesse caso a cena desenha por baixo o corpo guardado em `Session::corpos_antes`.

O fantasma é desenhado em dois passos, profundidade e depois cor, para cada pixel misturar só a camada mais
próxima, sem ordenar triângulos.

`previa_comando` continua na API para testes e para quem precisar da prévia sem abrir o comando.

## Consequências

- Com um comando aberto, o modo manual também calcula: a prévia é o motivo de o comando estar aberto.
- Qualquer comando novo ganha a prévia de graça, desde que marque as faces que cria com o id dele.
- Comandos que escolhem faces da peça (`wants_faces`) ficam fora do filtro de seleção, senão a pessoa não
  conseguiria desmarcar uma face que o comando já refez.
- Em comando que refaz o corpo inteiro, a peça de baixo não é selecionável enquanto o comando está aberto,
  porque os índices de face do clique são os do resultado novo.
- Reabrir um comando desses mostra por baixo a versão com os valores antigos, e não a peça sem o comando.
