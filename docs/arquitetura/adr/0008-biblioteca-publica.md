# ADR 0008: Biblioteca pública de hardware e perfis em formato aberto

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
Um diferencial do Fenkra é uma biblioteca **pública e editável** de fixadores, roscas, perfis
estruturais (com peso por metro e propriedades de seção), materiais e tabelas de chapa.

## Decisão
- Biblioteca mantida em **repositório Git público separado**, em arquivos de texto estruturado
  (ex.: YAML/JSON), um arquivo por família.
- Cada família = **gerador parametrizado** (seção ou peça desenhada por parâmetros) + **tabela de tamanhos**
  + metadados (norma de referência, fonte dos dados, licença, autor).
- Propriedades derivadas (área, peso por metro, inércia) são **calculadas** a partir da geometria e
  conferidas com os valores tabelados nos testes automáticos.
- **Camadas:** pública → empresa → usuário/projeto. O aplicativo junta as camadas; as de cima sobrepõem.
- Contribuições pela interface do aplicativo geram uma proposta (pull request) revisada pela comunidade.
- Projetos fixam a versão da biblioteca usada.
- **Licença dos dados (proposta): CC BY 4.0** — decisão do mantenedor.

## Cuidados
- Dados dimensionais (medidas, pesos) são fatos e podem ser registrados, mas **não copiar texto, figuras
  ou tabelas formatadas** de normas ABNT/ISO ou de catálogos de fabricantes. Registrar a fonte.

## Fontes de dados (29/10/2026)
As fontes indicadas pelo usuário e o nível de uso de cada uma (A livre, B catálogo público, C norma paga,
X só conferência) estão em [biblioteca/fontes-de-dados.md](../../biblioteca/fontes-de-dados.md). Dados de
projetos com licença incompatível (ex.: FreeCAD Fasteners, GPLv2) servem só para conferência.

## Consequências
- ✅ Qualquer pessoa pode revisar, corrigir e ampliar a biblioteca.
- ✅ Empresas mantêm bibliotecas privadas sobre a pública.
- ⚠️ Exige processo de revisão para manter a qualidade dos dados.

## Atualização B42: formato genérico das famílias

- A família escolhe um **gerador nomeado** (`"gerador"`, hoje parafuso_allen, parafuso_sextavado, porca,
  arruela). Família nova que reaproveita um gerador é só um arquivo de dados.
- A designação do tamanho é **livre** ("M6", "1/4-20 UNC", "#10-32", "-06"). Fora de "M<n>" o tamanho
  traz `d_nominal` em mm e, se quiser, `original`. A busca ignora caixa, espaços e vírgula decimal.
- Cada tamanho pode trazer a **série de comprimentos** (`comprimentos`), a faixa de pega
  (`grip_min`/`grip_max`), a faixa de furo (`furo_min`/`furo_max`), `pn` e `massa_g`. A regra geral
  2d a 10d acabou: vale a série publicada para o tamanho. Sem `furo_min/max`, os métricos usam a faixa
  de folga fina a folgada da tabela de furos.
- A norma tem `revisao`; a família tem `versao` (dos dados), `classe` (define o material da peça),
  `acabamento` e `unidades` por campo (mm quando não declarada).
- **Camadas** pública, empresa e usuário: `Biblioteca::mesclar` faz a de cima substituir a de baixo
  pelo `id`, família inteira. Hoje só a pública é carregada do repositório.
- A peça gerada leva o selo "Biblioteca: geração" (constante `VERSAO_BIBLIOTECA`, sobe quando a forma
  muda) e "Biblioteca: família" (`id@versao`); `situacao_do_selo` diz se está desatualizada. A
  montagem avisa e não regrava o arquivo, para não quebrar restrições.
- A peça publica o **eixo**, o **plano de apoio** e, nos parafusos, a **ponta** (dependem de `L`).
