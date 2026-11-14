# ADR 0093 — Coluna à esquerda e painéis que reservam espaço

Estado: aceito (14/11/2026). Revoga a premissa de "painéis livres sobre a viewport" do ADR 0012 e do
padrão de interface B319. Itens: B433, B434, B435 e B447.

## Contexto

O ADR 0012 pôs a área 3D ocupando a janela toda, com a árvore transparente no canto e os painéis
flutuando translúcidos por cima; o encaixe só alinhava o painel à borda e nunca reservava espaço. O B319
tornou isso obrigatório para todo comando novo ("painel de comando livre e encaixável", "árvore
transparente").

A auditoria visual de 12/11/2026 mediu o resultado: árvore e painel do comando somavam 900 px, 47% da
largura, por cima da peça; o painel do comando cobria o canto da geometria em edição em todas as
bancadas; "Lista de corte" abria por cima do menu e das abas de bancada; e o teste com mouse real de
13/11 achou a árvore solta desenhada fora da área útil, painel da base sobre a barra de status e a
janela da chapa sobre a barra de menus (B447). O usuário pediu o layout dos softwares de modelagem CAD
avançados que ele usa: árvore à esquerda, recursos à direita, topo fino, navegação embaixo e nada sobre
a peça.

## Decisão

1. **Coluna à esquerda que reserva espaço** (`app/coluna.rs`). É um painel lateral de verdade: a área 3D
   começa depois dela. Sem comando, mostra a árvore. Com comando aberto, o painel do comando toma a
   coluna e a árvore recolhe numa seta na primeira linha; a seta mostra a árvore no lugar dos campos,
   para escolher entradas, com Concluir e Cancelar ainda à vista, e outro clique volta aos campos. A
   largura é arrastável e fica nas preferências (`coluna_largura`). Árvore minimizada vira um trilho
   com a seta; fechada, a coluna some enquanto não houver comando.
   Com a árvore aberta pela seta, o clique num item que não preenche nenhum campo vazio não encerra o
   comando (fora dela, escolher outro item da árvore continua concluindo o comando aberto).
   Propriedades de um item escolhido (peça, parâmetros, anotação, vista do desenho, restrição) não são
   comando: a árvore fica na metade de cima da coluna e as propriedades na de baixo.
2. **Painéis só na direita ou na base, reservando espaço** (`paineis.rs`). Não existe mais painel livre.
   `Encaixe::efetivo` leva o que foi salvo como livre ou na esquerda para a direita e o que estava no
   topo para a base; painel largo sem borda escolhida nasce na base. Arrastado para longe das bordas, o
   painel volta para a borda em que estava. Os painéis encaixados registram o retângulo e, no quadro
   seguinte, `paineis::reservas` devolve as faixas que o app tira da área 3D antes de desenhar a vista.
   Os da base ficam à esquerda da faixa da direita. A reserva tem teto de 60% da área.
3. **Janelas de ferramenta viram painel encaixado** (`paineis::encaixado`): lista de corte, biblioteca
   de perfis, simulação, lista de peças, fixadores, engrenagens, transmissão, cantos, escada, corte de
   tubo, regras de dobra, chapa, fabricação da chapa, propriedades gráficas, reparo, extrair, repetir,
   publicações, links, diagnóstico, pais e filhos, estilo das anotações, comparação de versões,
   comparação no 3D, lista de elementos e ajuda do comando. Os painéis que já usavam `paineis::mostrar`
   (análises, importação, exportação, lote, features, gerenciador, biblioteca, montagem) passam pela
   mesma regra sem mudar de código.
4. **Só a janela nativa do sistema fica solta** (`Encaixe::Janela`, B366): é o caminho para outro
   monitor. O painel do comando vai para ela pelo menu de contexto do título.
5. **Barra de baixo e topo são faixas de verdade**: o painel da base fica acima da barra de baixo e
   nenhuma janela nasce sobre o topo.
6. **Diálogos curtos e modais** (salvar como, empacotar, confirmações, configurações, sobre, macros,
   personalizar comandos, novo desenho, exportar, editar cota) continuam janelas centradas: existem por
   um instante e pedem resposta.

Continua código próprio, sem biblioteca de docking: o motivo do ADR 0012 era a vista inteira atrás de
painéis translúcidos, que deixou de valer, mas o código já tem cabeçalho, minimizar, fechar, sombra de
encaixe, tamanho por bancada e janela nativa, com testes. Trocar por uma biblioteca seria refazer tudo
isso e migrar as preferências de novo.

## Migração das preferências

`Settings::migrar_layout`, chamada ao ler as camadas de configuração. `layout_versao` vale zero por
padrão, porque o `config.json` só guarda o que difere do padrão e arquivo antigo não traz a chave.

- A disposição antiga (painéis, docas das barras, lado do painel, rótulos e tamanho dos ícones) é
  copiada inteira para `layout_anterior` antes de qualquer mudança.
- Cada painel guarda tamanho, minimizado e fechado; só a borda muda, pela regra do item 2.
- O painel do comando passa a morar na coluna, e a largura que a pessoa tinha dado a ele vira a largura
  da coluna. Janela nativa em outro monitor continua onde estava.
- `Encaixe::Livre`, `Topo` e `Esquerda` continuam no formato do arquivo, para um perfil antigo importado
  depois ainda ser lido; em uso, valem pela regra do item 2.

## Topo, direita e base (B434) e painel enxuto (B435)

- Topo de cerca de 26 px: menu num ícone, abas de bancada em ícones e, à direita, Propriedades e o
  estado de versões/PLM. O nome do documento vai para o título da janela do sistema.
- Recursos à direita, só ícones (`app/barras.rs`, `toolbar::Forma::Coluna`): uma coluna quando cabe,
  duas quando não cabe, rolagem além disso. As docas antigas (topo, lateral esquerda, flutuante) valem
  como "na barra"; só "oculta" continua sendo escolha.
- Barra de baixo: recalculado, auto, avisos que pedem ação e a navegação (vista, projeção, modo de
  exibição e filtro de seleção). Reconstruir, Scan e o passo a passo ficam em Editar > Histórico da
  peça; versão do kernel, tempo de recálculo e objeto em trabalho ficam em Propriedades da peça.
- O painel do comando mostra só suporte, modo, parâmetros e avançado. O bloco comum (tipo, nome, selo
  de estado, suprimido, visível, suprimir quando) e o quadro de entradas completas ficam em
  Propriedades (`editor::bloco_comum`). O quadro de entradas volta ao painel enquanto pede algo: campo
  por preencher ou corpo por escolher.

## Consequências

- "Painel de comando livre" e "árvore transparente" saem do padrão B319. O checklist de interface passa
  a pedir: painel do comando na coluna, aberto só durante o comando; nenhum painel sobre a peça.
- A reserva vale um quadro depois de o painel aparecer: no primeiro quadro ele ainda cobre a borda da
  vista. Não se percebe em uso, e evita reescrever cada painel como área fixa.
- A área 3D muda de tamanho quando um painel abre ou fecha. A câmera não muda; a peça só é reenquadrada
  se a pessoa pedir.
- Sobre a área 3D ficam o cubo de vistas, pequeno, no canto, e o que pertence à própria manipulação:
  alças, rótulos da prévia e avisos passageiros. Ainda sobram três controles de canto, a migrar para as
  barras: "Símbolos" (montagem), "Espelhar a seção" e "Alinhar à aresta" (estrutural) e o seletor de
  folha (desenho).
- Dois painéis no mesmo trecho da direita ficam lado a lado e a faixa cresce; não há abas.
