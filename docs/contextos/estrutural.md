<!-- origem: contexto estrutural.txt (contexto do usuário, gerado com assistente de IA) -->

```text
MÓDULO ESTRUTURAL PAI, MONTAGENS DINÂMICAS E SUBMONTAGENS DE SOLDA

Arquitetura detalhada para um CAD novo com Weldment inteligente, PMI 3D e detalhamento peça a peça

1. VISÃO GERAL DO CONCEITO

Este documento descreve a arquitetura de um CAD novo com um módulo estrutural inteligente, capaz de:

Criar uma Estrutura Pai (modelo mestre estrutural) com múltiplos corpos.

Detectar corpos iguais e agrupá‑los automaticamente.

Gerar uma montagem dinâmica linkada ao estrutural Pai.

Criar peças individuais a partir de cada corpo (subparts) com links paramétricos.

Gerar PMI 3D para cada peça e para grupos estruturais.

Gerar detalhamento 2D folha a folha com links de parâmetros para corte a laser/plasma.

Criar submontagens de solda selecionáveis como subgrupos, para gerar montagens de componentes.

Permitir que placas e nervuras sejam criadas como outros Parts, integrados ao módulo estrutural.

A ideia é superar limitações de sistemas atuais (como Weldment do SOLIDWORKS) e se aproximar da filosofia de sistemas de alto nível (CATIA V5, NX, CREO), com uma arquitetura moderna e dinâmica.

2. ESTRUTURA PAI – O MODELO MESTRE ESTRUTURAL

2.1 Conceito

A Estrutura Pai é o arquivo que contém toda a definição estrutural:

Perfis estruturais (tubos, vigas, cantoneiras, barras).

Chapas, placas, nervuras, reforços.

Operações de corte, chanfro, furação, encaixe.

Soldas (geométricas ou simbólicas).

Parâmetros globais (comprimentos, ângulos, espaçamentos, materiais).

Ela funciona como um PartBody avançado, com múltiplos corpos inteligentes, mas ainda é o modelo mestre.

2.2 Estrutura interna

A Estrutura Pai contém:

StructuralBodies – cada corpo estrutural (perfil, chapa, nervura).

StructuralFeatures – operações aplicadas aos corpos (cortes, furos, chanfros).

StructuralGroups – agrupamentos lógicos de corpos (submontagens de solda, módulos, painéis).

GlobalParameters – parâmetros que controlam toda a estrutura.

Metadata – material, peso, código, família, etc.

Exemplo de árvore lógica:

EstruturaPai
 ├── Perfis
 │     ├── Perfil_40x40_Corpo_01
 │     ├── Perfil_40x40_Corpo_02
 │     └── Perfil_80x40_Corpo_03
 ├── Chapas
 │     ├── Chapa_Base
 │     └── Chapa_Reforço
 ├── Nervuras
 │     ├── Nervura_01
 │     └── Nervura_02
 ├── Soldas
 │     ├── Solda_Grupo_A
 │     └── Solda_Grupo_B
 └── ParâmetrosGlobais
       ├── Largura
       ├── Altura
       └── Comprimento

3. MÓDULO “STRUCTURAL ASSEMBLY GENERATOR”

Este é o módulo especial responsável por transformar a Estrutura Pai em:

Montagem dinâmica (Assembly Pai).

Peças individuais (Subparts) linkadas.

Submontagens de solda (grupos selecionáveis).

PMI 3D por peça e por grupo.

Detalhamento 2D automático.

3.1 Funções principais

3.1.1 BodyComparator (detecção de corpos iguais)

Responsável por comparar corpos estruturais e identificar quais são idênticos dentro de uma tolerância.

Critérios de comparação:

Perfil (seção transversal, tipo, família).

Comprimento.

Orientação (vetor, rotação).

Material.

Operações aplicadas (cortes, furos, chanfros).

Parâmetros locais.

Se dois corpos forem equivalentes → são considerados instâncias de uma mesma peça.

3.1.2 GroupingEngine (agrupamento na árvore)

Agrupa corpos equivalentes em Grupos Estruturais:

GruposEstruturais
 ├── Grupo_A (Perfil 40x40)
 │     ├── Instância_01
 │     ├── Instância_02
 │     └── Instância_03
 ├── Grupo_B (Perfil 80x40)
 │     ├── Instância_01
 │     └── Instância_02
 └── Chapas
       ├── Chapa_Base
       └── Chapa_Reforço

Cada grupo pode ser tratado como submontagem potencial.

3.1.3 InstanceManager (instâncias paramétricas)

Cria e gerencia instâncias paramétricas de cada grupo:

Cada grupo tem uma peça base (Part).

Cada instância é uma ocorrência dessa peça na montagem.

As instâncias mantêm links paramétricos com a Estrutura Pai.

3.1.4 PartGenerator (geração de subparts)

Gera automaticamente arquivos de peça (Subparts) a partir de cada corpo ou grupo:

Cada Subpart é um Part com:

Geometria derivada do corpo.

Parâmetros expostos.

Metadados (material, código, família).

Publicações (faces, eixos, planos importantes).

Subparts são linkados à Estrutura Pai:

Alterações no Pai atualizam o Subpart.

Alterações no Subpart podem, opcionalmente, voltar para o Pai.

3.1.5 LinkManager (links paramétricos Pai ↔ Subparts ↔ Assembly)

Gerencia todos os links:

Estrutura Pai → Subparts.

Subparts → Assembly.

Assembly → PMI 3D.

Assembly → Drawings.

Links são:

Paramétricos.

Bidirecionais (quando permitido).

Controlados por regras (quem manda em quem).

3.1.6 PMI3DGenerator (PMI 3D por peça e por grupo)

Gera automaticamente PMI 3D:

Para cada Subpart:

Vistas 3D.

Cotas.

Notas.

Símbolos.

Metadados.

Para cada grupo estrutural:

Vistas de conjunto.

Notas de solda.

Identificadores.

3.1.7 DrawingGenerator (detalhamento 2D automático)

Gera automaticamente Drawings:

Um Drawing por Subpart.

Um Drawing por submontagem de solda (se desejado).

Vistas automáticas (frontal, lateral, superior, corte, detalhe).

Cotas associativas.

Tabelas de parâmetros.

Exportação DXF/DWG para corte a laser/plasma.

4. SUBMONTAGENS DE SOLDA SELECIONÁVEIS

4.1 Conceito

Além da montagem global, o sistema deve permitir criar submontagens de solda:

Grupos de corpos que formam um módulo de solda.

Selecionáveis diretamente na Estrutura Pai.

Com comportamento de subgrupo dentro da montagem.

Exemplo:

EstruturaPai
 ├── Submontagem_Solda_A
 │     ├── Perfil_40x40_Corpo_01
 │     ├── Perfil_40x40_Corpo_02
 │     └── Chapa_Reforço_01
 ├── Submontagem_Solda_B
 │     ├── Perfil_80x40_Corpo_03
 │     └── Chapa_Base
 └── Submontagem_Solda_C
       ├── Nervura_01
       └── Nervura_02

Cada submontagem de solda pode ser:

Transformada em Assembly local.

Detalhada com PMI 3D próprio.

Detalhada com Drawing próprio.

Exportada como módulo de fabricação.

4.2 Seleção e criação

O usuário pode:

Selecionar corpos na Estrutura Pai.

Criar um Subgrupo de Solda.

Definir:

Nome.

Código.

Material.

Processo de solda.

Tratamento térmico.

O módulo gera:

Submontagem.

Subparts associados.

PMI 3D.

Drawing.

5. INTEGRAÇÃO COM PLACAS E NERVURAS (OUTROS PARTS)

5.1 Placas e nervuras como Parts externos

Placas e nervuras podem ser criadas como Parts independentes, fora da Estrutura Pai, e depois:

Inseridas na montagem.

Linkadas à Estrutura Pai.

Associadas a grupos estruturais.

5.2 Tratamento especial no módulo estrutural

O módulo deve:

Reconhecer placas e nervuras como componentes especiais.

Permitir que elas sejam:

Referenciadas por parâmetros da Estrutura Pai.

Controladas por GlobalParameters.

Incluídas em submontagens de solda.

Detalhadas com PMI 3D e Drawing próprios.

5.3 Fluxo típico

Criar Estrutura Pai.

Criar perfis estruturais.

Criar placas e nervuras como Parts externos.

Inserir placas/nervuras na montagem.

Linkar placas/nervuras à Estrutura Pai.

Criar submontagens de solda.

Gerar PMI 3D.

Gerar Drawings.

Exportar DXF/DWG para corte.

6. COMPORTAMENTO DINÂMICO – COMO NO CATIA V5

6.1 Atualização automática

Quando um parâmetro muda na Estrutura Pai:

Corpos estruturais são recalculados.

Subparts são atualizados.

Assembly é atualizado.

Submontagens de solda são atualizadas.

PMI 3D é atualizado.

Drawings são marcados como desatualizados e podem ser atualizados.

6.2 Links paramétricos

Os links devem ser:

Claros (quem é Pai, quem é Filho).

Controláveis (permitir ou bloquear propagação).

Documentados (metadados de link).

6.3 Filosofia de controle

Estrutura Pai manda na geometria global.

Subparts representam corpos individuais.

Assembly representa a organização física.

Submontagens de solda representam módulos de fabricação.

PMI 3D e Drawings representam documentação.

7. RESUMO DA ARQUITETURA

Este módulo especial permite:

Criar uma Estrutura Pai rica e paramétrica.

Detectar e agrupar corpos iguais.

Gerar Subparts linkados.

Gerar uma montagem dinâmica com instâncias.

Criar submontagens de solda selecionáveis.

Integrar placas e nervuras como Parts externos.

Gerar PMI 3D por peça e por grupo.

Gerar detalhamento 2D folha a folha.

Exportar DXF/DWG para corte a laser/plasma.

É uma arquitetura de CAD moderna, pensada para:

Estruturas soldadas.

Fabricação industrial.

Automação de documentação.

Robustez paramétrica.

Escalabilidade.

FIM DO DOCUMENTO

Este .txt descreve, em nível conceitual e arquitetural, o módulo estrutural Pai, o gerador de montagem dinâmica, as submontagens de solda e a integração com placas/nervuras como Parts, para um CAD novo com foco em estruturas e soldagem.
```
