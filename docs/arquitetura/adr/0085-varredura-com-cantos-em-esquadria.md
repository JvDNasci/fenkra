# ADR 0085: Varredura em guia com cantos gira a seção com junta de esquadria (B350)

- **Status:** aceita
- **Data:** 2026-11-12
- **Revê:** [ADR 0061](0061-varredura-por-escada-com-portao-de-erro.md) (o aviso de cantos sai)

## Contexto
A guia com cantos vivos (giro acima de 0,35 rad entre trechos) virava uma polilinha e ia para
`BRepOffsetAPI_MakePipe` com o Frenet corrigido. Num trecho reto o Frenet não tem normal, e o tubo de cada
trecho herdava a orientação do anterior: a seção seguia sem girar. Um círculo de raio 2 numa guia em L de
50 + 50 dava 628 mm³ em vez de 1257, numa em U dava zero, e `BRepCheck` aprovava os dois. A B337 só pôs
aviso na varredura e na nervura de chapa; membro estrutural e tubo do filete usam o mesmo caminho.

## Decisão
- No ramo com cantos, `sweep_tool` usa `BRepOffsetAPI_MakePipeShell` com transição `RightCorner`: cada
  canto vira uma junta de esquadria (as paredes dos dois trechos se encontram no plano bissetor).
- Guia plana: binormal presa à normal do plano. Guia 3D: Frenet corrigido e, se falhar, o triedro discreto.
- Entre dois cantos, um trecho de mais de dois pontos vira B-spline: só cantos de verdade levam junta.
- Cada laço do perfil é varrido à parte; os furos saem do laço de fora.
- Portão de volume: a esquadria conserva área projetada × comprimento da guia, a menos de
  `2 c tan(t/2)` por canto (c = distância do centro do perfil à guia). Fora disso, ou inválido, a
  varredura vira erro nomeado em vez de voltar ao caminho antigo, que aprovaria a seção tombada.
- Orientação fixa pedida pela pessoa segue sem girar e leva aviso explicando isso.
- Como a correção está no kernel, membro, tubo do filete e nervura de chapa herdam a junta certa; o aviso
  de "a seção não gira" sai de todos.

## Migração
A peça passa ao formato 3. O passo v2 -> v3 não muda os comandos: se o arquivo tem varredura ou nervura
de chapa, deixa a marca `B350` em `Document::migracoes` (campo que não é gravado). Com a marca, o recompute
avisa em cada varredura ou nervura com guia de cantos que o volume mudou em relação ao arquivo gravado.
Ao salvar, a marca some. Um Fenkra antigo abre o arquivo novo somente leitura, pela regra do B64.

## Consequências
- ✅ L e U dão área × comprimento; a guia 3D, o canto agudo e o perfil com furo também.
- ✅ Nenhum resultado com a seção tombada sai aprovado: ou volume coerente, ou erro nomeado.
- ⚠️ Documentos com guia de cantos mudam de volume ao abrir (com aviso na primeira sessão).
- ⚠️ Perfil muito fora do centro da guia em canto agudo pode cair no erro nomeado; a saída é arredondar
  o canto ou usar a orientação fixa.
