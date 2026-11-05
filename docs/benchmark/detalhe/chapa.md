# Benchmark de detalhe — Chapa metálica

> Texto próprio. Compara o nível de detalhe de cada comando (campos, prévia, manipuladores, dicas, fluxo)
> com softwares de modelagem CAD avançados e com o módulo livre de chapa do FreeCAD. Não repete o
> [benchmark por módulo](../chapa-metalica.md) nem a [pesquisa](../../pesquisa/chapa.md), que tratam de
> funções e de kernel; aqui o foco é a experiência de usar o comando. Estado do código: alpha.26.

## Resumo: as 5 maiores lacunas de experiência

1. **Nenhum comando de chapa tem prévia 3D antes de confirmar.** Nos CADs avançados, escolher a aresta
   já mostra a aba com valores padrão, e cada mudança de campo atualiza a prévia. No Fenkra o painel
   (`crates/fenkra-app/src/feat_ui.rs:453-830`) só edita números; o usuário vê o efeito depois do
   recálculo. Uma busca por prévia/fantasma em `feat_ui.rs`, `editor.rs` e `handles.rs` não achou nada.
2. **Manipulação 3D mínima.** Só cinco comandos têm uma seta de comprimento (`handles.rs:97-117`,
   `222-226`: aba, bainha, estampo, furo flangeado, veneziana). Não há manipulador de ângulo, de raio, de
   posição da dobra, de largura da aba nem de recuo lateral, e as setas não carregam rótulo com o valor.
3. **Campos sem explicação própria.** O helper `expr` (`editor.rs:146`) mostra, no rótulo, o mesmo texto
   genérico de expressões para todos os campos. Só alguns comandos têm uma linha de ajuda fraca embaixo
   (bainha, estampo, aba por superfície, aba curva). Faltam, por campo, o que ele muda, o limite
   válido, o padrão e uma figura curta.
4. **Seleção da geometria por números e botões.** Dobra por linha, joggle e veneziana pedem dois
   vértices e um botão "Usar os dois vértices selecionados" (`feat_ui.rs:545-596`), mostram a linha
   como texto `de (x,y,z) a (x,y,z)` e não desenham a linha no 3D. Os outros CADs pedem a face fixa,
   desenham a linha no sketch e mostram o sentido da dobra com seta.
5. **Parâmetros de chapa em janela solta, sem regra reaproveitável.** A janela "Chapa metálica"
   (`app.rs:3125-3225`) tem material, bitola, K e tabela, mas não há estilo nomeado, raio como fórmula da
   espessura, nem marca de "esta feature sobrescreve o padrão" (comum em CADs avançados). Também faltam
   extensão lateral da aba (largura centrada ou deslocada, entre referências), laço de arestas e
   fechamento de canto com tipo e folga.

## Checklist por comando

Legenda: ok, parcial, falta. "Prévia 3D" conta só a prévia viva antes de confirmar. "Ícone claro" foi
avaliado pelo nome do arquivo e por B60 (revisão pendente), sem auditoria visual de cada SVG.

| Comando | Intuitivo | Prévia 3D | Manipulação 3D | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Parâmetros (`SmParams`) | parcial | ok | falta | parcial | parcial | parcial | ok | Prévia não se aplica (janela de dados). Tem sugestão por tabela e conversão BD/BA em K. Falta estilo nomeado. |
| Parede base (`SmWall`) | parcial | falta | falta | parcial | parcial | parcial | parcial | Escolhe sketch ou superfície; sem seta de espessura nem do lado. |
| Aba de aresta (`SmFlange`) | parcial | falta | parcial | parcial | parcial | falta | parcial | Seta só de comprimento. Falta ângulo, raio, largura, posição por vértice virtual. |
| Bainha (`SmHem`) | parcial | falta | parcial | parcial | parcial | parcial | parcial | Tem ajuda por tipo. Falta figura do perfil e folga de mitra. |
| Dobra por linha (`SmBend`) | falta | falta | falta | parcial | ok | falta | falta | Linha por dois vértices e botão; sem face fixa nem seta. |
| Joggle (`SmJoggle`) | falta | falta | falta | parcial | parcial | falta | falta | Dois modos (pontos ou plano); sem perfil da rampa no 3D. |
| Estampo (`SmStamp`) | parcial | falta | parcial | parcial | parcial | parcial | parcial | Cinco tipos; ajuda por tipo. Falta biblioteca visual de formas. |
| Furo flangeado (`SmFlangedHole`) | parcial | falta | parcial | parcial | parcial | falta | falta | Tabela de série e cadastro do usuário são bons; faltam cotas no 3D. |
| Veneziana (`SmLouver`) | falta | falta | parcial | parcial | ok | falta | falta | Linha por vértices; rampa sem prévia. |
| Recorte (`SmCut`) | parcial | falta | falta | parcial | parcial | parcial | parcial | Falha ao atravessar a dobra (DR-20). Sem opção "normal à face" visível. |
| Alívio de canto (`SmCornerRelief`) | ok | falta | falta | parcial | ok | falta | ok | Um clique no vértice; falta mostrar o formato antes. |
| Rasgo circular (`SmArcSlot`) | parcial | falta | falta | parcial | ok | falta | parcial | Quatro números sem arco desenhado. |
| Aba por perfil (`SmProfileFlange`) | parcial | falta | falta | parcial | parcial | parcial | parcial | Exige desenhar a cadeia antes, em plano certo. |
| Aba curva (`SmCurveFlange`) | parcial | falta | falta | parcial | ok | parcial | parcial | Dicas por opção de lado são o melhor exemplo da bancada. |
| Aba por superfície (`SmSurfFlange`) | falta | falta | falta | parcial | ok | parcial | falta | Três modos com dicas boas; sem mostrar a linha de dobra (OML). |
| Dobrar/Desdobrar (`SmUnfold`) | parcial | falta | falta | parcial | parcial | falta | ok | Troca seca de estado; sem animação nem controle de ângulo. |
| Furo e furo roscado (compartilhados) | parcial | falta | falta | parcial | parcial | falta | parcial | Valem dobrado e planificado; sem lembrar que ficam fora do raio. |
| Raio e chanfro (compartilhados) | parcial | falta | falta | parcial | parcial | falta | parcial | Ícone `arestas_selecionar` não diz "arredondar". |

Contagem das 126 células (18 linhas x 7 critérios) no final do arquivo.

## Por comando (os mais usados)

### Aba de aresta

**O que os outros têm e o Fenkra não.**
- Prévia imediata ao tocar a aresta, com valores padrão (ângulo 90°, raio da chapa); cada campo mexe na
  prévia. Em versões recentes de um CAD avançado, os manipuladores de ângulo e altura aparecem no 3D e
  o clique neles abre um campo de entrada ali mesmo.
- Posição da dobra com cinco opções (material por dentro, por fora, dobra por fora, a partir do vértice
  virtual, tangente à dobra). O Fenkra tem as três primeiras (`feat_ui.rs:488-492`).
- Medida do comprimento por referência escolhida: interseção das faces externas, das internas ou
  tangente; e comprimento "alinhado" ou "ortogonal" à base.
- Extensão lateral: aresta inteira, largura (centrada ou deslocada), entre duas referências, e seleção
  de laço de arestas para criar várias abas de uma vez. O Fenkra só tem recuo início/fim.
- Alívio personalizado com razão ou largura e profundidade; opção de "rasgar" ou "estender"; aparar
  dobras laterais.
- Sobrescrever raio e compensação só nesta aba, com o padrão da regra visível.
- Inverter para a aresta oposta mantendo os demais parâmetros.

**Como deveria ser no Fenkra.**
- *Suporte:* aresta, laço ou face lateral (clique ou arraste caixa).
- *Modo:* comprimento (referência), posição da dobra, extensão.
- *Parâmetros:* comprimento, ângulo, raio (padrão da regra, com cadeado para sobrescrever).
- *Avançado:* alívio, mitra, recuos, compensação local.
- *No 3D:* aba fantasma translúcida; seta de comprimento e arco de ângulo com rótulos; marca do raio;
  linha de dobra tracejada; aresta escolhida em destaque; aviso em vermelho se colide com a vizinha.
- *GIF (8 s):* 0-2 s o cursor passa por uma aresta e a aba fantasma aparece; 2-5 s arrasta a seta e o
  comprimento muda com rótulo; 5-7 s arrasta o arco do ângulo de 90° para 60°; 7-8 s alterna "material
  por dentro" e a aba se desloca pela espessura.

### Bainha

**Lacunas.** Os CADs avançados oferecem quatro tipos (fechada, aberta, gota, enrolada), cada um com
campos próprios: comprimento só em fechada e aberta, folga só em aberta, ângulo e raio só em gota e
enrolada, e folga de mitra quando duas bainhas se cruzam. O Fenkra tem os tipos e a ajuda por tipo
(`feat_ui.rs:516-533`), mas mostra todos os campos e não esconde os que não valem. Falta a figura do
perfil e a folga de mitra.

**No Fenkra.** Seletor visual com quatro miniaturas do perfil; só os campos do tipo; prévia da seção
transversal sobre a aresta; mitra automática com folga. *GIF (6 s):* troca de tipo faz a aba mudar de
perfil e os campos que não valem somem.

### Dobra por linha e Joggle

**Lacunas.** Os outros pedem a face fixa (a que não se move), uma linha de sketch, a posição da dobra
(linha de centro, material dentro, material fora, dobra fora), ângulo com botão de inverter, raio
padrão ou próprio, compensação própria. No joggle, a posição da rampa, a forma de cotar (por fora, por
dentro, total) e a opção de manter o comprimento projetado. O Fenkra tem boa parte dos números
(`feat_ui.rs:534-595`), mas a linha é texto, não há face fixa e a seta de sentido não existe.

**No Fenkra.** Clique na face; o comando entra em modo de desenho de linha com snap; a linha aparece
tracejada com setas de sentido; o lado que gira fica sombreado; ângulo por arco arrastável.
*GIF (7 s):* desenhar a linha, ver as duas metades coloridas (fixa e móvel), arrastar o arco e inverter.

### Estampo, veneziana e furo flangeado

**Lacunas.** Biblioteca de formas com miniatura e cotas editáveis no 3D, posicionamento por clique na
face com rotação, pré-visualização da ferramenta de forma e do que ela corta. O Fenkra tem a tabela de
séries de furo flangeado e o cadastro do usuário, um ponto forte raro (`feat_ui.rs:656-700`), mas sem
miniatura nem cotas.

**No Fenkra.** Cartões com miniatura por tipo; fantasma da forma seguindo o mouse; cotas de diâmetro e
altura como rótulos arrastáveis. *GIF (6 s):* passar o mouse na face mostra o fantasma, clique fixa,
arrasto da altura muda o colar.

### Recorte na chapa

**Lacunas.** Os CADs avançados oferecem recorte "através da dobra", em que o contorno do sketch é
planificado e vale também na peça dobrada, e recorte "normal à face". Há ainda projetar a planificada
dentro do sketch para cotar contra ela. O Fenkra tem o modo (`feat_ui.rs:701-717`), mas falha quando o
contorno atravessa o raio (DR-20, B02).

**No Fenkra.** Escolha "normal à face" ou "através da dobra"; prévia do corte na planificada e na
dobrada ao mesmo tempo; botão para projetar a planificada no sketch. *GIF (7 s):* desenhar um retângulo
sobre a dobra e ver o corte aparecer nas duas vistas.

### Aba por superfície, aba curva e aba por perfil

**Lacunas.** Mostrar a linha de dobra resultante (OML) e o limite (EOP) no 3D, com cores por modo. O
Fenkra tem três modos com dica ao passar o mouse, o melhor texto da bancada (`feat_ui.rs:768-806`), mas
nenhuma pista visual. Aba por perfil: mostrar o plano de desenho exigido e a cadeia com os cantos já
arredondados.

**No Fenkra.** Linha de dobra em magenta, EOP em ciano, a web sombreada. *GIF (8 s):* alternar entre
Exato, Aproximação e Ângulo e ver a linha de dobra mudar de curva para reta.

### Parâmetros de chapa

**Lacunas.** Estilo/regra nomeada reaproveitável, com raio como fórmula da espessura (por exemplo
1,15 x espessura), método de planificação, relevo de dobra, forma do alívio de canto de duas e três
dobras e tamanho padrão. Cada feature pode sobrescrever, e a interface mostra a marca de sobrescrita.
Tabela de bitola ligada à espessura. O Fenkra tem material, bitola, sugestão e conversão de método
(`app.rs:3135-3200`), mas guarda tudo no documento, sem estilo.

**No Fenkra.** Janela com seletor de estilo no topo, "salvar como estilo", raio como expressão, bloco
de alívios padrão; um ícone de cadeado em cada campo de feature indica herdar ou sobrescrever.
*GIF (5 s):* trocar de estilo muda espessura e a tabela de dobras recalcula.

### Dobrar/Desdobrar e planificada

**Lacunas.** Controle de ângulo de dobra de 0 a 100% animado; desdobrar só algumas dobras; anotação da
ordem de dobra editável (reordenar em sequência ou por intervalo, número duplicado permitido) que vai
para a tabela do desenho. O Fenkra alterna o estado inteiro (`app.rs:1593-1600`) sem animação.

**No Fenkra.** Controle deslizante que dobra e desdobra; glifos numerados nas linhas de dobra; arrastar
um glifo reordena. *GIF (6 s):* mover o controle e ver a peça planificar; clicar nos glifos para trocar
a ordem.

### Fechamento de canto (novo no Fenkra)

Os CADs avançados oferecem tipos de canto (topo, sobreposto, sob), folga e razão de sobreposição, mais
opções como canto estreito e propagação automática. O Fenkra só tem alívio de canto. Entra como item
novo no backlog.

## Detalhes finos que os outros cuidam e o Fenkra ainda não

- **Valores padrão visíveis:** o campo de raio nasce com o valor da regra (por exemplo uma fórmula da
  espessura) e mostra de onde veio. No Fenkra, o raio próprio vem com "2" fixo ao marcar a caixa
  (`feat_ui.rs:484`), sem relação com a espessura.
- **Inverter sem refazer:** botão "inverter direção" que troca para a aresta oposta mantendo os demais
  parâmetros. No Fenkra há caixas "dobrar para o outro lado", espalhadas e com nomes diferentes em cada
  comando ("mover o outro lado da linha", "desviar o outro lado", "levantar para o outro lado").
  Padronizar um único botão com seta.
- **Estado do comando na barra:** mensagem do próximo passo ("escolha a aresta", "escolha a face fixa").
  Existe um aviso solto na aba curva e na aba por superfície, mas não como regra da bancada (B60 cobre a
  barra de status em geral).
- **Mensagens de erro específicas:** "a aba colide com a vizinha", "raio menor que o mínimo do material",
  "a aresta não é da chapa". Hoje o diagnóstico é genérico (B69).
- **Limites claros:** ângulo entre 0 e 180°, raio maior que zero, recuo menor que a aresta; o campo fica
  vermelho e diz o limite, não só "inválido".
- **Atalhos e repetição:** repetir o último comando (Enter ou barra de espaço) e menu de contexto com os
  comandos de chapa ao clicar numa aresta. Em CADs avançados, clicar com o botão direito numa aresta de
  chapa oferece aba, bainha e alívio.
- **Aviso de fabricação no campo:** ao digitar um raio abaixo do mínimo da tabela, o campo avisa na hora
  (a tabela já existe em `app.rs:3160`, só falta ligar ao campo).
- **Editar com a peça no estado certo:** ao editar uma feature criada na planificada, o app deveria
  trocar sozinho de estado e voltar depois, com um aviso curto.

## Modelo de painel proposto para todo comando de chapa

Estrutura única, nesta ordem: **Suporte** (o que escolher no 3D, com contador e botão limpar) -> **Modo**
(miniaturas com o resultado de cada tipo) -> **Parâmetros** (o essencial, 2 a 4 campos, cada um com ícone
e dica em dois estágios) -> **Avançado** (recolhido: alívio, mitra, compensação local, sobrescritas).
No 3D, o comando sempre mostra: fantasma translúcido do resultado, a geometria escolhida em destaque, e
os manipuladores de cada parâmetro numérico com rótulo. O tooltip do comando traz o GIF de 5 a 10 s do
roteiro acima (laço, sem som, 16:9, sem texto na imagem, legenda em linguagem simples).

## O que não dá para ser 3D, e por quê

- **Tabelas de material, bitola, fator K e dedução:** são dados numéricos; o 3D só mostra o efeito. A
  prévia vira o comprimento desenvolvido, mostrado como texto junto da peça.
- **Compensação de dobra (K, BD, BA):** não tem forma visível no sólido dobrado; só na planificada. Mostrar
  o valor ao lado da linha de dobra.
- **Regras e estilos:** são configuração; o 3D mostra só a diferença que causam.
- **Cadastro de furos flangeados do usuário:** é lista, não geometria.
- **Normas de fabricação (raio mínimo, aba mínima):** viram avisos com realce da região, não um desenho.
- **Ordem de dobra:** é uma sequência; o 3D mostra os números nas linhas, e a ordem em si vive numa lista.
- **Mensagens de erro:** o 3D realça a face, mas a causa é texto.

## Fontes consultadas

Ajudas oficiais de softwares de modelagem CAD avançados (chapa metálica: aba de aresta, bainha, dobra
esboçada, joggle, canto fechado, regras e estilos, recorte através da dobra, ordem de dobra) e a ajuda do
módulo de chapa do FreeCAD. As URLs ficam no relato da orquestração e em
`docs/arquitetura/procedencia.md`.

## Contagem do checklist

ok: 11, parcial: 66, falta: 49 (total 126 células).
