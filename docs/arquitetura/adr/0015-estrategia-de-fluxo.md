# ADR 0015: Estratégia de fluxo de trabalho do Fenkra

- **Status:** proposta
- **Data:** 2026-11-05
- **Itens de backlog:** B316 (esta decisão), B256 (roda de comandos), B20 (ordem dos campos), B60 (teste de contrato)

## Contexto

O benchmark de detalhe da interface (`docs/benchmark/detalhe/interface.md`) mostra que as dores do usuário
têm uma raiz comum: cada comando decide sozinho como começa, como mostra a prévia e como termina. O
resultado é desigual entre bancadas. Softwares de modelagem CAD avançados resolvem isso com um fluxo
reconhecível (escolher o recurso, indicar o suporte, ajustar, confirmar), mas pagam com muitos cliques,
painéis que escondem o modelo e pouca indicação do que fazer depois. O Fenkra quer o mesmo caminho
conhecido, com menos cliques e com o próximo passo sempre à vista.

## Decisão

### 1. Cinco jornadas, uma gramática

Todo o programa é descrito por cinco jornadas. Cada uma tem um trilho de passos que a barra de status e a
roda de comandos conhecem.

| Jornada | Trilho | Ponto de chegada |
|---|---|---|
| Peça | Sketch, sólido base, recursos, padrão, aparência | Peça com status válido, volume e caixa conferíveis |
| Sketch | Plano, geometria, restrições, cotas, graus de liberdade zero | Perfil fechado e totalmente restrito |
| Montagem | Inserir, restringir, verificar interferência, lista de materiais | Montagem sem conflito e com GDL esperado |
| Desenho | Folha, vistas, cotas, anotações, exportação | Folha pronta para exportar |
| Liberação | Revisão, estado, número de peça, versão | Revisão liberada e imutável |

O trilho é dado, não código por bancada: cada comando declara em qual jornada e em qual passo está, e o
passo seguinte provável. Assim a bancada nova nasce já encaixada.

### 2. Próximo passo sempre visível

A barra de status mostra duas coisas: o que falta no comando ativo ("falta: escolher o perfil") e, fora de
comando, o próximo passo da jornada ("o perfil está fechado: extrude"). A mensagem sai do mesmo texto que
alimenta o tooltip, para existir um só lugar a traduzir (PT/EN em `i18n.rs`).

### 3. Um jeito de iniciar, confirmar e cancelar

- **Iniciar:** o comando abre já pedindo o suporte; se há seleção válida, ela é usada e o comando pula
  para o passo seguinte.
- **Confirmar:** Enter, o botão Concluir ou clique no vazio quando o comando não tem campo ativo.
- **Cancelar:** Esc. Cancelar não deixa rastro: nada no documento e nenhum passo de desfazer.
- **Prévia:** viva desde o primeiro suporte válido, sem confirmar (contrato do `interface.md`, seção 1.4).
- **Ordem dos campos:** suporte, modo, parâmetros, avançado recolhido (B20).

Exceções precisam de registro neste ADR.

### 4. Menos cliques, medidos

Cada jornada tem uma tarefa de exemplo cuja contagem de cliques e teclas é registrada no repositório e só
pode crescer com justificativa. Contam-se os gestos do estado inicial ao resultado (clique, tecla,
digitação de campo). A meta inicial é igualar o caminho conhecido de softwares de modelagem CAD avançados
e reduzir um terço onde o trilho permite pular passos (suporte pela seleção, valor padrão sensato,
confirmação por Enter). A medição entra como teste automático sobre o roteiro de demonstração
(`FENKRA_DEMO*`), no mesmo espírito do B60.

### 5. Roda de comandos sugere o próximo

A roda de comandos (B256) usa o trilho: os dois setores preenchidos pelo uso local são complementados por
uma sugestão da jornada (o passo seguinte do último comando concluído). A sugestão nunca substitui os
setores fixos, para a memória muscular não quebrar.

### 6. Conferência por bancada

Cada bancada é conferida contra este fluxo e contra o checklist padrão de
`docs/benchmark/detalhe/interface.md` (intuitivo, prévia 3D, manipulação 3D, ícone, tooltip, dica por campo,
fluxo). O resultado por bancada fica na ficha do módulo em `docs/modulos/`, com ok, parcial ou falta.

## Alternativas consideradas

- **Deixar cada bancada definir o seu fluxo:** é o estado atual; gera a desigualdade que motivou o ADR.
- **Assistente guiado obrigatório:** ajuda quem começa, mas atrapalha quem já sabe e aumenta cliques.
  Aproveitamos a ideia só como lições opcionais (seção 3.3 do `interface.md`).
- **Copiar o fluxo de um software específico:** além de ferir a regra de marcas, herdaria os defeitos que
  queremos corrigir. Usamos a ideia geral, escrita com nossas palavras.

## Consequências

- Comando novo declara jornada, passo e próximo passo; a falta disso falha num teste.
- A barra de status e a roda de comandos passam a depender de uma tabela única de trilhos.
- Custo: tocar em todas as bancadas na conferência. Fazer por bancada, sem reescrever tudo de uma vez.
- Risco: o "próximo passo" sugerido errado irrita mais do que ajuda. Começar discreto (texto na barra de
  status) e só então levar à roda.
- Implementação fica em itens próprios (tabela de trilhos, barra de status, medição de cliques, sugestão
  na roda); este ADR só fixa a direção.
