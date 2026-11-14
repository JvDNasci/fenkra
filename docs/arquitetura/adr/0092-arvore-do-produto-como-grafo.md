# ADR 0092: árvore do produto como grafo de arquivos, com miniatura tirada do pacote de visualização (B364)

Data: 2026-11-14. Estado: aceita. Itens: B364, B339 (montagens do projeto no visualizador).

## Contexto

O painel do projeto lista os documentos em linhas. Quem trabalha com um conjunto grande quer ver a
estrutura do produto: que montagem usa o quê, em que quantidade, o que já está liberado, e reconhecer a
peça de olho, por uma miniatura. As montagens (`.fka`) não estão no índice do projeto; só as peças estão.
Abrir cada montagem e recalcular cada peça para desenhar a árvore seria lento demais num conjunto grande.

## Decisão

1. **Um nó por arquivo, não por instância.** `Project::arvore_produto` lê só o JSON: o cabeçalho de cada
   arquivo e as referências `Part`/`SubAssembly` dos nós de cada montagem (as mesmas de "onde é usado",
   B94). A mesma peça em três montagens é um nó com três pais, e a aresta guarda a quantidade. O grafo
   cresce com o número de arquivos. A tela desdobra o grafo em árvore pelo caminho de nós desde a raiz,
   então cada ocorrência abre e fecha sozinha.
2. **Raízes**: montagens que ninguém usa, depois as peças do índice que nenhuma montagem usa. Uma
   montagem só alcançável por ciclo (A usa B que usa A) entra como raiz para não sumir; todo percurso
   guarda o caminho e para ao reencontrar um nó dele.
3. **Estado da montagem é derivado.** Ela não tem ciclo de vida próprio no índice. A regra de cor é uma
   só: liberado é peça liberada ou montagem cujas peças distintas de baixo estão todas liberadas;
   obsoleto e arquivo fora do projeto ficam apagados. As cores são as do tema.
4. **Miniatura sai do pacote `.fkv`**, rasterizada em software (`fenkra_pacote::miniatura`): vista
   isométrica, teste de profundidade em grade 2× e redução pela média. A imagem é só luz (tons de cinza
   com fundo transparente) e a tela a pinta com a cor do texto do tema, então a peça fica distinta do
   fundo em qualquer tema e a imagem não traz cor fixa. Não usamos a GPU nem o kernel: a miniatura roda
   numa linha de execução própria e funciona até onde o kernel não está.
5. **Pacote de montagem pela assinatura.** Peça do índice usa o pacote da versão atual (ADR 0070).
   Montagem e arquivo de fora usam `.visualizacao/arquivos/<nome>-<assinatura>.fkv`, em que a assinatura
   junta o conteúdo do arquivo e as assinaturas dos filhos: muda quando qualquer peça de baixo muda, e o
   pacote nunca fica velho. Ficam os 6 mais novos de cada arquivo; o anterior serve para comparar.
6. Sem pacote, a miniatura só aparece quando a pessoa pede "Gerar miniaturas" (ou com a opção do projeto
   de gerar ao salvar ligada): gerar exige recalcular a peça, e isso não pode acontecer sem pedido num
   conjunto grande.

## Alternativas

- **Captura pelo viewport wgpu**: a miniatura ficaria igual à tela, mas só para o documento aberto e
  com a GPU ocupada; não serve para gerar dezenas de miniaturas em segundo plano.
- **Guardar a miniatura em PNG ao lado do pacote**: evita rasterizar de novo ao abrir, mas duplica dado
  que o pacote já tem; rasterizar 96 px a partir da malha leva milissegundos. Fica para quando um
  projeto real mostrar que pesa.
- **Árvore por instância**: simples de desenhar, mas explode com subconjuntos repetidos.

## Consequências

- A árvore abre sem recalcular nada; a parte cara (gerar pacote) é pedida e roda em segundo plano.
- A montagem do projeto ganha pré-visualização versionada no navegador (B339) pelo mesmo pacote.
- Montagem dentro de pasta fora das pastas de busca do projeto não aparece como raiz; aparece só se
  alguma montagem de dentro a usar.
