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

## Consequências
- ✅ Qualquer pessoa pode revisar, corrigir e ampliar a biblioteca.
- ✅ Empresas mantêm bibliotecas privadas sobre a pública.
- ⚠️ Exige processo de revisão para manter a qualidade dos dados.
