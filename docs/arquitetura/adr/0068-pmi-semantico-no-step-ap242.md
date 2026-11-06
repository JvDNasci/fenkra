# ADR 0068: PMI semântico no STEP AP242 pela ponte XDE

Data: 2026-11-06. Item: B191.

## Contexto

A ponte XDE (B87) já lia e gravava STEP com o documento XCAF e tinha o gancho `SetDimTolMode`, mas nenhuma
anotação do Fenkra chegava ao arquivo, e o que o arquivo trazia só era contado. Os nomes de entidade que
`gdt.rs` guardava (`step_nome`, `step_forma`) vinham de conhecimento geral do formato, não do que o
OpenCASCADE 8.0.1 instalado de fato escreve.

## Decisão

- **A fonte da verdade é o XCAF, conferida por ida e volta.** `cpp/xde_pmi.inc` cria, no `XCAFDoc_DimTolTool`,
  uma dimensão, tolerância ou datum por anotação, com o objeto (`XCAFDimTolObjects_*`) e a ligação às faces
  por subforma (`AddSubShape`). `pmi_xde.rs` espelha os enums do OCCT na ordem exata dos cabeçalhos e o teste
  `discriminantes_seguem_o_occt` os fixa. Ninguém escreve entidade STEP à mão: quem decide a entidade é o
  gravador, a partir do enum.
- **Só AP242 e só com a chave ligada.** Conferido: em AP203 e AP214, e em AP242 sem `SetDimTolMode`, as
  anotações do documento não saem. A interface oferece "Anotações semânticas (PMI)" só com AP242.
- **O mapeamento mora no conversor.** `fenkra-conversor/src/pmi_step.rs` liga cada `Pmi` à face da forma
  gravada pela mesma chave geométrica que o PMI usa para reencontrar o elemento (`face_do_alvo`) e devolve,
  junto, uma nota por item que o STEP não leva. O que não atravessa é dito na saída, não calado.
- **Tipos que derrubam o OCCT são recusados antes.** Dimensão "com caminho" e rótulo comum dão falha de
  segmento dentro do gravador quando falta a aresta de caminho; `TipoDimensaoXde::atravessa` os recusa com
  erro. Os 26 tipos restantes foram gravados e relidos um a um.
- **Leitura devolve datums únicos.** O leitor do OCCT cria um datum novo por uso em cada tolerância e um por
  alvo; `ler_anotacoes` os junta pela letra, faces e alvo, e remapeia as referências das tolerâncias.

## O que não atravessa (OCCT 8.0.1, conferido)

Sem solução na API: zona de tolerância projetada e a altura dela; zona não uniforme e batimento como
modificador de zona; modificadores de contorno (todo o contorno, toda a superfície); perfil dinâmico, feição
contínua, feições unidas e "entre" (não existem no XCAF); ordem diferente dos datums por quadro (a
precedência é uma propriedade do datum no arquivo); sinal do desvio inferior de uma cota (o gravador escreve
o módulo); nome da cota (vira o nome do tipo); quadro composto (segunda linha); alvo de curva e de círculo
curvo; alvos na leitura quando nenhum quadro cita a referência; dimensões orientada, com caminho, rótulo
comum e apresentação. PMI gráfico (rugosidade, solda, notas, balões) e vistas salvas **não foram feitos**:
o gravador só leva apresentação com geometria própria por anotação.

Duas esquisitices do OCCT a conhecer: o ângulo é gravado cru numa unidade em radianos e lido em graus (a
ponte converte na gravação); e toda tolerância com zona de forma sai com uma `RUNOUT_ZONE_DEFINITION` sem
orientação, que o próprio leitor ignora.

## Consequências

- Um arquivo gravado aqui abre com cotas, quadros e referências em outro programa que leia AP242; a
  checagem com o analisador de arquivos do NIST fica como etapa manual (não há como rodá-lo daqui).
- Se o OCCT corrigir um dos itens acima, o teste correspondente cai e este ADR e `docs/modulos/pmi-3d.md`
  se atualizam.
- `gdt.rs`: `Modif::Mmc/Lmc::step_nome` corrigidos para `*_requirement`, o nome que o arquivo traz.
