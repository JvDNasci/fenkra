# Análise de interface (vis_int)

Amostra: 28 capturas de `diario/` (PNG originais), de Estrutural, Chapa, Sólidos, Superfícies, Forma livre, Transmissão, PMI, Montagem e Desenho, nas versões normal, pintado e colorido. Só vi o tema claro padrão; não vi tema escuro nem outra densidade. A modelagem foi feita em código, então prévia, alça 3D e fluxo do comando (itens do checklist de `interface.md`) não foram exercitados: só vi dois painéis de comando abertos (Aba e Membro). Não dá para dar nota de "intuitivo" nem de "tooltip com GIF".

## 1. O que foi usado e o que não foi

- Usado: faixa de bancadas e de comandos (Sólidos, Superfícies, Chapa, Montagem, Desenho), árvore, ViewCube, barra de status, painel de comando (Aba e Membro), Assistente de reparo, balões e lista de materiais, folha de desenho.
- Não usado: prévia translúcida, alças de arraste, dica por campo, tooltip, roda de comandos, busca Ctrl+K (a barra de status a cita), temas. O teste não passa pela interface, então esses itens ficam sem evidência.

## 2. O que deu certo

- `0132_kart_geral_colorido.png`, `0136_sc-quadro_pronta_colorido.png`, `0134_sc-eixo_pronta_colorido.png`: com `Node.color` a montagem parece um kart de verdade (quadro vermelho, pneus pretos com aro dourado, carenagem branca, chapas escuras). Cada peça se distingue da vizinha.
- `0054_ex-12_falhou.png`: erro bem comunicado. Item em vermelho com X na árvore, "2 com erro — reparar" na barra de status e o Assistente de reparo com Refazer referência, Suprimir e Apagar.
- `0134`, `0136`, `0089`: balões numerados no 3D casam com "Balão EX-01" na árvore, e as restrições aparecem escritas ("Coincidência EX-02 ↔ EX-01", "Fixação conjunta").
- `0091`/`0109`: árvore de montagem com contagem "84 definidos, 0 parciais, 0 soltos" e a barra de status "0 GDL livres, 0 restrições com erro". É informação útil e compacta.
- `0123_ch-08_painel_aba_pintado.png`: painel do comando com Concluir, Cancelar, Excluir, selo "OK", faixa verde "Entradas completas" e a prévia azul translúcida da aba, com cotas "Raio 2mm", "Ângulo 90°". É o padrão B319 funcionando.
- `0094_dw-quadro_folha.png`, `0138_dw-kart_folha_colorido.png`: folha com moldura, zonas, legenda e lista de materiais legíveis; a árvore da folha (Folha 1, Superior, Frente, Detalhe) é clara.
- Peças ficam distintas do fundo no tema claro em quase tudo (cinza-azulado sobre cinza claro): `0007`, `0047`, `0064`.

## 3. O que falhou

- **Pintado igual a sem pintar na montagem.** `0107_kart_geral_pintado.png` é idêntico ao `0091`; `0109_kart_subconjuntos_pintado.png` sai todo num azul chapado. Já é o K-25; a imagem confirma. Só o `colorido` (via `Node.color`) funciona.
- **Peça pintada de preto perde a forma.** `0119_po-01_pronta_pintado.png` (banco) e `0117_rd-04_pronta_pintado.png` (pneu) são massas quase pretas, sem arestas visíveis e com pouco realce; no banco não se lê o côncavo. `0130_ex-01_pmi_pintado.png`: o eixo vira cinza-escuro e as legendas ficam o que restou. Uma peça pintada de escuro precisa de luz de preenchimento ou de contorno.
- **Branco sobre fundo claro.** Em `0132`/`0133` a carenagem branca quase some no fundo; só a sombra de baixo a separa.
- **Peça maior que a janela, sem enquadrar.** `0008_quadro_topo.png`: o quadro sai cortado em cima e embaixo e invade a barra de status. `0024_ex-01_pmi.png`: o eixo sai da tela e passa por baixo do texto da árvore ("Corpo principal (ativo)" cortado). Em contraste, `0091` deixa o kart ocupando só uns 30% da área.
- **Rótulos sobrepostos.** Em toda peça a etiqueta "FK-... Tolerância geral ISO 2768" se sobrepõe à legenda (K-22). Em `0007` e `0008` os rótulos `mestre_*` das curvas ligadas colidem entre si (K-24). Em `0125` os rótulos de alça do Membro ("Deslocamento X/Y", "Rotação", "Prolongar") se escondem uns aos outros; em `0123`, "Recuo do início" fica atrás de "Comprimento da aba" (K-26).
- **Painel de comando abre pequeno e corta o conteúdo.** `0123` e `0125`: o painel mostra só botões de seção (Suporte, Modo da aba, Parâmetros, Avançado, Relações) e uma faixa de 1 linha; em `0125` o título "PERFIL" aparece cortado. Não deu para ver nenhum campo editável. Não sei se é o tamanho de captura ou o padrão do painel; se for o padrão, o usuário abre o comando e não vê os campos.
- **Captura sem o painel que a legenda promete.** `0003_ch-01_painel.png` ("Comando Membro aberto") e `0019_ch-08_painel_aba.png` não mostram painel nenhum; só as versões pintadas (`0123`, `0125`) mostram. Isso é limite do teste.
- **Linhas dos planos atravessam a árvore.** `0001_mestre_esqueleto.png`: uma linha laranja infinita corta a árvore e o título; com árvore transparente, o texto compete com a geometria.
- **Caminho local na árvore do Desenho.** `0094`, `0096`, `0138` mostram `C:\Users\João\Projetos\projeto-cad\.claude\worktrees\agent-...\kart\modelo\DW-*.fkd` abaixo do título. É ruído, e vaza o caminho do usuário em qualquer vídeo.
- **Tabela por cima da legenda e vistas se cobrindo** em `0094`, `0096`, `0138`: são K-19 e K-27. Em `0096` a tabela de furos cai sobre a legenda, e a vista do disco mostra só dois círculos, sem os 24 furos que a legenda promete (não sei se é falha de projeção ou da própria vista; é da bancada Desenho).
- **`0133_kart_lateral_colorido.png` mostra o kart de frente**, com o ViewCube marcando "Direita". Ou o kart foi modelado com o eixo longitudinal em outra direção, ou a vista nomeada está trocada. Não consegui decidir por imagem.
- `0063_po-04_planificado.png`: a captura "Assoalho planificado" mostra a chapa com as abas dobradas em 3D, não o plano. Não consegui confirmar a planificação por imagem.

## 4. Deu certo, mas dá para melhorar

- **Árvore de montagem (`0091`):** 38+ linhas de códigos ("CH-08:2 ×3", "EX-02:3 ×3") sem nome da peça e sem agrupar por subconjunto. Mostrar nome ao lado do código e colapsar por subconjunto. `0109` (com SC-*) é muito mais legível.
- **Glifos de estado sem legenda** (`0134`, `0136`): ■, ○ e alfinete antes da linha, sem tooltip visível. Uma legenda de uma linha resolve.
- **Árvore de peça longa e plana** (`0001`, `0007`: 60 curvas "mestre_LONG_E_0 ·1"): o sufixo "·1" não é explicado. Agrupar por origem e esconder o sufixo.
- **Fluxo do painel:** os botões de seção parecem abas, mas "Entradas completas" fica no topo e a seção ativa mostra 1 linha. Deixar a primeira seção aberta e dizer o que falta.
- **Tamanho da fonte:** legenda do painel, "Suprimir quando", resumo da árvore ("84 definidos...") e barra de status estão em corpo muito pequeno na captura 1920 px.
- **Etiqueta "Atualizada" nas vistas do desenho** (`0094`, `0138`) sobrepõe a vista vizinha; deveria ficar fora da área de desenho.
- **Barra de status:** o "Próximo passo: selecione uma face ou aresta, ou use Ctrl+K" é igual em todas as telas, inclusive em desenho e montagem. Trocar pela dica do contexto.

## 5. Melhores imagens para o vídeo

1. `0132_kart_geral_colorido.png`: o kart completo e colorido, com a árvore de montagem ao lado.
2. `0136_sc-quadro_pronta_colorido.png`: quadro vermelho com chapas escuras e balões numerados.
3. `0134_sc-eixo_pronta_colorido.png`: eixo com rodas, balões e restrições escritas.
4. `0123_ch-08_painel_aba_pintado.png`: painel de comando com prévia translúcida e cotas no 3D.
5. `0054_ex-12_falhou.png`: erro explicado e Assistente de reparo (mostra que o programa não esconde falha).
6. `0138_dw-kart_folha_colorido.png`: o mesmo kart em folha A1 com vistas e lista de materiais.

Não usar `0125` nem `0001` no vídeo (rótulos e linha atravessados) e evitar as pintadas escuras.

## 6. Regras para a skill da bancada

- Antes de capturar, enquadre a peça (zoom ao conteúdo) e esconda origem e rótulos de curva: sem isso, linhas e etiquetas cobrem a árvore e a peça.
- Pintura escura exige luz de preenchimento e contorno; testar toda cor nova também com preto e branco contra o fundo do tema.
- Capture o painel do comando aberto com tamanho fixo que mostre os campos, senão a legenda promete algo que a imagem não mostra.
- Qualquer texto na árvore que não seja nome do usuário (caminho de arquivo, sufixo "·1", códigos) precisa de nome legível ou sair da tela.
- Ao comparar "pintado" e "colorido", abra a montagem: o acabamento da peça não chega na montagem (K-25), então a conferência vale só na peça aberta.

## 7. Itens novos de backlog (fora de `bugs.md`)

- Painel de comando abre com altura que corta os campos (ou conferir se é só a captura): **M**.
- Enquadrar automaticamente ao abrir peça, subconjunto e vista nomeada (quadro cortado em `0008`, eixo em `0024`, kart pequeno em `0091`): **M**.
- Aparência escura sem realce: luz de preenchimento e contorno de silhueta para cores com luminância baixa (e alto contraste com fundo para brancas): **M**.
- Árvore do Desenho não mostrar caminho completo do arquivo (usar nome): **B**.
- Árvore de montagem: nome ao lado do código, agrupamento por subconjunto e legenda dos glifos (■ ○ alfinete): **B**.
- Etiqueta "Atualizada" das vistas fora da área de desenho: **B**.
- Verificar o nome da vista lateral: "Direita" no ViewCube com o kart visto de frente (`0133`): **B**.
- Dica da barra de status por contexto (desenho, montagem, comando aberto) em vez da frase fixa: **B**.
