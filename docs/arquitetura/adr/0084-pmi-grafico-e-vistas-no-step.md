# ADR 0084: PMI gráfico e vistas salvas no STEP AP242

Data: 2026-11-08. Item: B191. Complementa a [ADR 0068](0068-pmi-semantico-no-step-ap242.md).

## Contexto

A ADR 0068 levou ao STEP a parte semântica do PMI (cotas, quadros, referências e alvos) e deixou de fora o
desenho das anotações, o que não tem semântica no arquivo (rugosidade, solda, notas, balões, legenda) e as
capturas do PMI. Lendo o gravador e o leitor do OpenCASCADE 8.0.1 (`STEPCAFControl_Writer.cxx` e
`STEPCAFControl_Reader.cxx` do código-fonte instalado pelo vcpkg), conferimos:

- toda dimensão, tolerância e datum do XCAF aceita uma apresentação (`SetPresentation`) que o gravador escreve
  como `tessellated_annotation_occurrence` dentro de um `draughting_callout` com o nome dado, mais o plano da
  anotação; o leitor devolve as arestas, o nome e o plano;
- um rótulo comum (`CommonLabel`) **sem faces** é gravado só como desenho; ligado a faces, o gravador o trata
  como dimensão e cai (o que a ADR 0068 viu). Para ele o gravador não escreve o plano;
- o gravador **não escreve vistas** (`camera_model_d3`); o leitor as lê;
- o gravador converte a geometria e o plano para a unidade do arquivo, mas escreve crus os pontos da
  apresentação, o ponto do texto e os valores (cota, desvios, zona, tamanho do alvo); o leitor multiplica
  tudo pela unidade. Em polegada, o PMI voltava 25,4 vezes maior; a janela da vista, ao contrário, o leitor
  devolve crua;
- o gravador tira o contexto do modelo de apresentação do primeiro elemento semântico ligado à forma: só com
  rótulos comuns, ele sai sem contexto e a gravação falha.

## Decisão

- **Diálogo com três modos**: nenhum, gráfico, gráfico + semântico (`OpcoesExportar::pmi_grafico` junto de
  `pmi`). No modo com desenho, toda anotação leva a apresentação; a que não tem semântica no STEP, ou que a
  semântica não conseguiu ligar a uma face, vai como rótulo comum, com uma nota dizendo que foi só como desenho.
- **O desenho sai do Fenkra, em segmentos** (`fenkra-conversor/src/pmi_grafico.rs`): chamada com seta, linha
  de cota com setas, quadro de tolerância com o símbolo vetorial de `GdtChar::tracado`, caixa do datum, anel
  do balão e o texto numa fonte de traços própria (grade 4 x 6, só maiúsculas). O texto exibido vai também no
  nome da apresentação, em ASCII. Plano de leitura: o da anotação, senão o da primeira captura que a mostra,
  senão a câmera isométrica padrão.
- **A ponte escreve as vistas à mão**, depois de `Transfer` e antes de `Write`: um `draughting_model` por
  captura, com a câmera (`camera_model_d3`, `view_volume`, `planar_box`) e os `draughting_callout` das
  anotações mostradas, achados pelo nome da apresentação, que o Rust torna único. Esta é a única exceção à
  regra da ADR 0068 de não escrever entidade STEP à mão, porque a API do OCCT não tem caminho para isso.
- **A ponte corrige a unidade**: antes de `Transfer`, em polegada ou metro, divide pela unidade os pontos da
  apresentação, o ponto do texto e os valores de comprimento (não os ângulos); na leitura de vista, multiplica
  a janela e a distância pela unidade do arquivo. Depois de `Transfer`, o modelo de apresentação sem contexto
  recebe o contexto da forma.

## Consequências

- Ida e volta conferida em mm, polegada e metro (`fenkra-kernel/tests/pmi_xde_grafico.rs`) e com a flange do
  corpus nos três modos (`fenkra-conversor/tests/pmi_step_grafico.rs`).
- Rótulo comum volta sem plano (limite do gravador). As vistas levam a câmera e as anotações, não a lista de
  corpos visíveis nem planos de corte.
- A conferência com o analisador de arquivos do NIST continua manual.
