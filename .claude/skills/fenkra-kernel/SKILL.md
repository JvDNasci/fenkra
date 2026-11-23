---
name: fenkra-kernel
description: Diretrizes da bancada Kernel do Fenkra. Use ao criar, corrigir ou pesquisar B-Rep do OpenCASCADE, tolerâncias, topologia, ponte cxx e diagnóstico de falhas, ou ao tocar em crates/fenkra-kernel/ (lib.rs.
---

# Kernel

Antes de criar ou mudar um comando, siga a skill `fenkra-comando` (contrato do ADR 0107).

## Domínio

crates/fenkra-kernel/ (lib.rs, cpp/kernel.cpp, kernel.h)

## Antes de agir

1. Leia a seção da área em `docs/modulos/geral.md` e, se precisar de contexto, `docs/pesquisa/sintese.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Kernel\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de B-Rep do OpenCASCADE, tolerâncias, topologia, ponte cxx e diagnóstico de falhas, busque primeiro documentação oficial do OpenCASCADE, normas,
livros e artigos técnicos, e depois documentação pública de outros CADs. Guarde uma nota por assunto
na pasta de conhecimento, com: como funciona, fórmulas, casos de borda, sintomas de falha e como
depurar. Escreva com suas palavras, sem copiar texto ou imagem, e registre as fontes em
`docs/arquitetura/procedencia.md`. Pesquise só o que a tarefa pede: pesquisa tem custo.

## Diretrizes

Preencha com o que for aprendido. Cada regra leva o motivo.

## Como verificar

`cargo test -p <crate> <filtro>`, com documento montado em código conferindo status, volume e caixa
envolvente. Mudança visual: capture com as variáveis `FENKRA_DEMO*`.

## Vizinhos

Consulte todas as bancadas quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

- Booleana e malha em threads só sobre cópia profunda (`Shape::comuns`, `Shape::em_threads`, ADR 0081). A
  booleana não tem estado global, mas a mesma forma entra em vários pares e o comum pode devolver as faces da
  entrada: tesselar grava a malha nelas, e duas threads gravariam na mesma face. `Shape::clone` divide a topologia
  e não serve de cópia (B394).
- `BRepOffset_MakeOffset::MakeOffsetShape` só fecha o resultado em SOLID quando a entrada é do tipo
  SOLID (`MakeSolid` testa `myShape.ShapeType()`); com o COMPOUND que a booleana devolve, sai casca solta.
  Corpo vindo de STEP chega como SOLID e escondia o problema. Desloque sólido por sólido (`face_offset_t`, B464).
  Antes de culpar tolerância quando "histórico falha e STEP passa", compare o tipo do contêiner.
- `BRepGProp::VolumeProperties` sem `eps` usa pontos de Gauss fixos e erra na quarta casa em face aparada por curva
  NURBS racional (furo vindo de 3DM: 1130,84 em vez de 1130,97), com a geometria certa. Antes de culpar a
  tradução, compare a malha fina ou use `Shape::volume_preciso` (B211). E não converta cilindro, cone, esfera e toro
  para NURBS sem precisar: a forma racional tem outro parâmetro e o volume medido nela erra 1 %.
- Referência a face, aresta ou ponta nunca é coordenada (ADR 0010, B516). Campo novo que guarda um ponto clicado
  precisa de chave: os que já têm slot usam `FaceKey`/`EdgeKey` ao lado do ponto; os demais usam a âncora do comando
  (`Feature::ancoras`, resolvida em `ancoras.rs`). A âncora só vale para o ponto em que nasceu: trocar o ponto a
  invalida sozinho. Sem chave, nunca entregue o ponto ao kernel quando ele não cai em nada: o `sm_flange` (e outros que
  usam `BRepExtrema_DistShapeShape` para achar "a aresta perto") pega a mais perto sem limite, e isso é escolha
  silenciosa. Confira antes com `edges_through`/`face_index` e devolva "referência perdida" com candidatas.
- Ferramenta nova sem papéis deixa as arestas dela sem nome, e o comando seguinte que clicar nelas volta ao ponto. Dê
  papel na saída (`extrude_roles`, `revolve_roles`) ou pela geometria (`papeis.rs`: varredura, loft, furos, aba de
  chapa) e passe em `apply_roles`. A booleana já leva a origem de cada face (`boolean_tagged_src`, histórico do
  OCCT); em geral não é preciso `BRepTools_History` novo na fachada. Na dúvida (face sem papel ou papel único que
  repete), devolva `None`: sem papel a referência segue pelo ponto; papel errado escolhe a face errada sem aviso.
- `Shape::section` devolve polilinha: pontos subdivididos entre os vértices ficam nas cordas, décimos de mm fora
  da curva em peça grande (a flecha cresce com a escala; as tolerâncias da booleana não). Varrer uma dobra
  tangente à chapa por essas amostras faz a fusão devolver vazio sem erro (B566). Varra pela aresta exata
  (`edge_curve_at`, `Shape::curva_aparada`) e leve as amostras para cima dela (`Shape::curva_projeta`). Para
  investigar uma fusão que sai vazia ou incompleta, comece por `Shape::diag_fusao`.
- Ferramenta que precisa casar com uma borda que outra booleana criou (a dobra sobre a borda recuada da chapa): não
  varra pela curva que fez o recorte; depois do recorte, leia a aresta que ficou (`edge_curve_at`) e varra por ela.
  A booleana grava a interseção do jeito dela, com a tolerância dela. Deslocar curva no plano é
  `Shape::curva_deslocada` (`Geom_OffsetCurve` aproximada); o cabeçalho do `Geom_OffsetCurve` diz que o lado é
  V × T, o medido no 8.0.1 é T × V: confira o lado no resultado, nunca pelo comentário (B566).
- Uma ponta de ferramenta no meio de uma face cara de avaliar (`Geom_OffsetSurface`) custa segundos de booleana: a
  aba que parava um passo antes da borda da web deslocada levava 4,8 s na fusão, e 2,0 s indo até o canto (B566).
  Antes de mexer em tolerância, veja onde as faces da ferramenta cortam faces do corpo sem precisar.
- Spline por pontos (`GeomAPI_Interpolate`, `Seg2::Spline`) com dois pontos quase juntos no meio de pontos espaçados
  oscila e se cruza: a face sai inválida sem erro na construção, e a tangente medida entre os dois vira ruído. Ao
  acrescentar um ponto exato (a ponta de uma aresta) a amostras, troque a amostra vizinha quando ele cai a menos de
  meio passo dela, em vez de inserir (B566).
- Tolerância absoluta pedida a uma aproximação (`GeomConvert_ApproxCurve` a 1e-7 mm) quebra em peça grande: numa
  curva de 2 m é 5e-11 do tamanho e a aproximação desiste, sem erro visível para quem chama (a função devolvia
  `None` e o comando voltava à polilinha, 7,6 % de volume a menos). Peça a tolerância relativa ao comprimento e a
  continuidade que a curva tem de verdade: o deslocamento de uma B-spline cúbica C2 é só C1 (B566).
- Constante em mm num critério (0,05 mm de folga, 0,5° por vértice, 0,2 % do raio) quase sempre é escala escondida:
  ou fica pequena demais em peça grande, ou vira um passo inteiro em peça de 10 mm. Ao criar um critério, diga de
  onde vem o número (a flecha da seção é 0,1 mm, `FLECHA_SECAO`; o desvio da malha é relativo à diagonal) e teste
  o caso escalado de 10 a 10 000 mm, como `chapa_b566_escala.rs` (B566).
- `GeomAPI_ProjectPointOnCurve` sem projeção ortogonal (ponto além da ponta do trecho) dá `NbPoints() == 0`: quem
  mantém o ponto como estava fica com um ponto fora da curva que parece projetado. `Shape::curva_projeta` leva o
  ponto para a ponta mais perto (B566).
- `volume_preciso` (integração adaptativa) não converge em sólido de faces planas aparadas por spline longa: informa
  erro de 1e-16 e o volume muda 3e-5 quando o eps muda. Ali a integração padrão (`mass`) é a certa; em faces varridas
  longas é o contrário (a padrão erra ~6e-6 do total). Antes de concluir que a geometria está errada, compare as
  duas com uma conta feita à parte (B566).
- `posicionado` (montagem) copia a forma sem a malha: quem tessela a peça posicionada refaz a malha inteira a cada
  vez. Use a malha da vista quando ela basta, ou tessele em threads (B394).

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.
