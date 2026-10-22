# ADR 0009: Projeto em pasta com versões imutáveis e revisões congeladas

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
A gestão de projeto precisa de estrutura de produto, revisões congeladas e gestão de links, sem exigir
servidor. Os links (ADR 0006) podem apontar para revisões específicas.

## Decisão
- Um **projeto** é uma pasta com:
  - os documentos de trabalho;
  - um **índice** (banco SQLite) com documentos, PN, propriedades, publicações e links;
  - um **armazém de versões imutáveis** (arquivos endereçados por hash do conteúdo, como o Git faz).
- **Versão** = instantâneo automático de um documento (ao salvar).
- **Revisão** = versão marcada pelo usuário com letra/número e comentário; fica **congelada** (somente leitura).
- Documentos são identificados por **ID interno estável**, não pelo caminho do arquivo; renomear ou mover
  não quebra links.
- Liberação de montagem congela todos os componentes juntos.

## Alternativas consideradas
- **Usar Git diretamente:** ótimo para texto, ruim para arquivos binários grandes e pouco amigável para usuários de CAD.
- **PDM com servidor:** fora do escopo (não-objetivo).

## Consequências
- ✅ Funciona offline, em pasta local ou compartilhada.
- ✅ Revisões antigas sempre abrem exatamente como foram liberadas.
- ⚠️ Uso de disco cresce com as versões; prever limpeza de versões não marcadas como revisão.
- ⚠️ Edição simultânea em pasta compartilhada precisa de bloqueio simples (RF-GPR-025).
