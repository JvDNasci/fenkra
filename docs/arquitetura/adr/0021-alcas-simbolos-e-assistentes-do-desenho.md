# ADR 0021: alças, símbolos guiados e assistentes do desenho

## Contexto
O desenho (B250 a B252) ganhou alças diretas na folha, construtores de rugosidade e solda, o assistente
de Novo desenho e as opções de exportação. Três decisões de modelo atravessam o formato do arquivo e a
interface, e merecem registro.

## Decisão
1. **Operações das alças ficam no crate de desenho** (`fenkra_desenho::alcas`): inverter o corte troca as
   pontas `a` e `b` da linha de corte (a normal da vista cortada e as setas derivam da ordem dos
   pontos), o raio do detalhe tem mínimo, e a escala anda numa série preferida. A interface só desenha
   e liga o mouse. Cada operação é uma mudança única do desenho.
2. **Arraste vira um passo de desfazer pelo desenho de antes**, guardado no início (`drag_snap`) e
   empilhado no fim, em vez de empilhar a cada quadro. Antes, o arraste de uma vista não tinha passo
   próprio: desfazer voltava para antes da edição anterior.
3. **Rugosidade ganha `Note::rough` (opcional, omitido do JSON quando vazio) e a solda reaproveita
   `DwgWeld`.** O valor principal continua no texto da nota. Um símbolo montado só com cliques volta a
   `None` (`simbolos::normalizar`), então sai idêntico ao do texto antigo, no arquivo e nas primitivas.
   A direção das estrias é desenhada com linhas, não com texto, para sair igual em PDF, SVG e DXF
   (a fonte do PDF só cobre Latin-1).
4. **O método de projeção é do desenho** (`Drawing::projection`, padrão 1º diedro, ausente nos arquivos
   antigos). Ele decide onde nascem a superior e a lateral, a direção da projetada pela posição do clique
   e o símbolo do diedro da legenda. A norma escolhida é o modelo de legenda que já existia.
5. **As opções de exportação mexem nas primitivas da folha** (`opcoes::Opcoes::aplicar`), antes de cada
   formato escrever seu arquivo. Cor, preto e branco e espessura valem igual nos três formatos.

## Consequências
- Arquivos antigos abrem sem mudança (campos novos com padrão). Arquivo novo com `rough` ou
  `projection` de 3º diedro, aberto numa versão antiga, perde esses campos.
- Preto e branco descarta preenchimentos claros (o sombreado de vistas): é intencional, mas muda o
  aspecto de vistas sombreadas.
- Exportar SVG ou DXF de várias folhas grava um arquivo por folha, com o nome da folha no nome.
