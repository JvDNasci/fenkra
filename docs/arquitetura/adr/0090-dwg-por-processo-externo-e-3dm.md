# ADR 0090: DWG pelo LibreDWG como processo externo, desenho 2D na folha e o caminho do 3DM (B211)

- **Status:** aceita
- **Data:** 2026-11-13
- **Revê:** [ADR 0055](0055-formatos-cad-e-2d-abertos.md), na parte do DWG e do 3DM

## Contexto
O ADR 0055 deixou o DWG e o 3DM de fora. O DWG esperava decisão de produto: a biblioteca aberta, o LibreDWG, é
GPL-3.0, e o Fenkra é LGPL-2.1 com o `deny.toml` recusando GPL como dependência. O 3DM esperava uma ponte com a
biblioteca aberta do formato, o openNURBS. O usuário decidiu: DWG pelo LibreDWG só como programa externo; 3DM pelo
openNURBS; e o DXF 2D com ida e volta também no desenho técnico, não só no sketch.

## Decisão

### Regra da GPL
1. **Nada do LibreDWG dentro do Fenkra.** Nem link estático, nem dinâmico, nem código, cabeçalho ou binário no
   repositório ou no pacote. O Fenkra só procura os programas `dwg2dxf` e `dxf2dwg` que a pessoa instalou e os chama
   como processo separado, trocando arquivos. É a fronteira de "programas que se comunicam por arquivo e linha de
   comando": o Fenkra não vira obra derivada, e o LibreDWG segue com a licença dele, instalado e atualizado pela
   pessoa.
2. **O `deny.toml` não muda.** O LibreDWG não é crate nem dependência de compilação; a regra "nada de GPL" continua
   valendo para tudo o que o Cargo traz.
3. **O instalador não traz o LibreDWG.** Se um dia trouxer, o pacote precisa levar o fonte e a licença dele à parte,
   e isso pede outro ADR.

### Integração do DWG (`crates/fenkra-conversor/src/dwg.rs`)
- **Onde procura**, nesta ordem: a pasta da variável `FENKRA_LIBREDWG` (as Configurações > Programas externos gravam
  nela; quando apontada e sem o programa, não procura em outro lugar, para ninguém rodar um programa que não
  escolheu); o `PATH`; e as pastas de costume (`Arquivos de Programas\LibreDWG`, `tools\libredwg` do vcpkg).
- **Leitura:** copia o DWG para uma pasta temporária com nome curto (o programa de linha de comando pode não
  aceitar acento no caminho), roda `dwg2dxf -y -o saida.dxf entrada.dwg` sem console, com limite de 120 s, e lê o
  DXF pelo leitor que já existe. Daí em diante o DWG é um DXF: no documento vira um sketch por camada; no desenho
  técnico, uma folha com um esboço por camada. Quem decide se deu certo é o arquivo de saída, não o código de
  retorno, porque o LibreDWG devolve erro até em aviso.
- **Gravação:** grava o DXF R2000 de sempre e roda `dxf2dwg -y -o saida.dwg entrada.dxf`.
- **Sem o programa:** erro `Ausente` com o texto de [`COMO_INSTALAR`]: onde baixar (pacote para Windows do projeto
  GNU, ou `vcpkg install libredwg[tools]`), onde apontar a pasta e, sem ele, pedir o arquivo em DXF.
- **Detecção pelo conteúdo:** o DWG abre com a versão em texto (`AC1015` a `AC1032`). A matriz de formatos lista o DWG
  com o provedor "LibreDWG (externo)"; a sonda da matriz só confere o DWG quando o programa está instalado.

### DXF 2D na folha do desenho (`crates/fenkra-conversor/src/folha.rs`)
- A exportação da folha em DXF já existia (`fenkra_desenho::export::dxf`). A volta é nova: **Abrir** no desenho
  técnico aceita DXF, DWG e SVG e monta um desenho com uma folha, um esboço solto por camada (B352) e uma nota por
  texto. As coordenadas são milímetros da folha; a folha é a menor de A4 a A0 que cabe o conteúdo a partir da origem,
  então a folha que o Fenkra gravou volta no mesmo formato. Antes de montar os esboços, sai do arquivo toda
  entidade igual (a 0,001 mm) às que a folha nova já desenha: moldura, zonas e campos da legenda com o mesmo texto.
  Sem isso, cada número de zona viraria uma nota repetida.
- O tipo de linha sai do nome das cinco camadas que o Fenkra grava, depois do tipo de linha da camada (tracejado,
  traço e ponto) e, por último, da espessura (menos de 0,35 mm é fina). Hachura fica de fora e é contada.
- O conversor passa a depender de `fenkra-desenho`. Não há ciclo: o desenho não depende do conversor.

### 3DM: o que entrou e o caminho
- **Licença, conferida em parte:** o arquivo `LICENSE` do repositório oficial (github.com/mcneel/opennurbs, ramo
  8.x) traz o aviso de direitos da McNeel, a isenção de garantia e a licença do zlib que vai junto, que ele descreve
  como parecida com a do openNURBS. O cabeçalho `opennurbs_public.h` manda ao site do openNURBS para os termos
  completos. O openNURBS é conhecido como livre para uso comercial e aberto, no estilo do zlib, o que seria
  compatível com a LGPL do Fenkra e com o `deny.toml`; mas o texto de concessão não está no `LICENSE` lido. **Antes de
  vendorizar, a frente do 3DM confere o texto de concessão completo e o registra em `procedencia.md` e no pacote.**
  Os nomes de produto citados no aviso são marcas e não entram em código, interface nem documentação.
- **Nesta rodada só a detecção.** O fonte do openNURBS não está na máquina (nem pacote do vcpkg) e baixá-lo pede o
  aval do usuário. Além disso, a ponte é pesada: cerca de 400 arquivos C++ compilados pelo `cc`, mais a tradução
  `ON_Brep` para `TopoDS` (superfícies NURBS, curvas 2D de aparo, laços, arestas com tolerância, costura) e a volta.
  Isso é uma frente própria, não cabe junto com o DWG.
- O que existe: o 3DM é reconhecido pelo cabeçalho (`3D Geometry File Format`) e a leitura responde com a explicação
  e a saída (exportar STEP AP214 no programa de origem), em vez de "formato desconhecido".
- **Desenho da ponte** para a próxima frente: crate `fenkra-3dm` com o openNURBS vendorizado e compilado pelo `cc`
  (sem cmake), ponte `cxx` mínima que devolve uma lista neutra por objeto (superfície NURBS com polos, pesos, nós e
  graus; aparos 2D por laço; curvas 3D das arestas; camada; cor) e a montagem da forma do lado do `fenkra-kernel`,
  com `Geom_BSplineSurface`, `BRepBuilderAPI_MakeFace` com os aparos, `ShapeFix` e costura. A gravação faz o
  caminho inverso pelas faces convertidas para NURBS. Teste de ida e volta: placa com furo e superfície livre,
  conferindo volume, caixa, camada e cor.

## Fora desta rodada
- **PDF 3D (PRC)** e **STEP AP242 em XML**: seguem como no ADR 0055 (sem biblioteca aberta compatível; o OpenCASCADE
  8.0.1 não traz a parte 28).
- **3DM com geometria**, como dito acima.

## Consequências
- O DWG funciona em qualquer máquina com o LibreDWG instalado. O teste de ida e volta com o programa de verdade se
  declara ignorado quando o programa não está lá; o encanamento (procura, argumentos, pasta temporária, leitura do
  DXF gerado, mensagem sem o programa) é conferido com um programa falso.
- O `dxf2dwg` do LibreDWG grava DWG R2000 e a importação de DXF dele ainda é marcada como experimental pelo próprio
  projeto. Um DWG gravado pelo Fenkra deve ser aberto e conferido no programa de destino antes de virar entrega.
