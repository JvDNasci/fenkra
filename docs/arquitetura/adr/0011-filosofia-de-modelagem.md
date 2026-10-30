# ADR 0011: Filosofia de modelagem robusta (CATIA, SolidWorks e Inventor)

- **Status:** proposta (orienta a onda 0 do backlog)
- **Data:** 2026-10-30
- **Base:**
  - pesquisa pública orquestrada sobre os três CADs (fontes em [benchmark/fontes.md](../../benchmark/fontes.md));
  - auditoria do código do Fenkra;
  - plano de corpus de testes.

## Contexto
O usuário encontra bugs "simples" ao testar fluxos comuns. Alguns exemplos:
- o furo entre peças só existe na montagem;
- editar uma peça na montagem tira o usuário da montagem;
- há três botões Salvar;
- a qualidade de tesselação é fixa;
- as dicas dos ícones são só texto.

Os três CADs de referência evitam isso com regras de produto, não com comandos isolados. Cerca de 25–30% das features marcadas como ok ou parcial não têm teste com nome próprio.

## Decisão: sete princípios

1. **Referência estável ou erro visível.**
   - Referências entre comandos e entre peças usam elementos de referência (planos, eixos, pontos), **publicações com nome** ou chaves dono+papel (ADR 0010).
   - Se não resolver, o comando fica em estado "falhou" com diagnóstico. Nunca escolhe outra geometria em silêncio.
   - O diagnóstico oferece: Editar, Substituir, Isolar, Desativar e Reordenar (CATIA Update Diagnosis, SW What's Wrong, Inventor Design Doctor).
2. **Um contexto ativo, sempre visível.**
   - Há um só objeto em trabalho na árvore: onde entra o próximo comando, com barra de fim de peça.
   - Na montagem, há um só componente ativo por vez. Editar uma peça não sai da montagem: o resto fica esmaecido e continua selecionável.
   - Voltar, Voltar ao pai e Voltar ao topo estão sempre à mão.
3. **Links entre arquivos têm estado.**
   - Cada referência externa mostra `->` (em contexto), `->?` (fora), `->*` (travada) ou `->x` (quebrada).
   - É possível Travar, Destravar e Quebrar.
   - Substituir um componente religa pelo nome publicado.
4. **Feature de montagem diz onde mora.**
   - O furo de montagem atravessa o escopo pelo eixo e grava um furo real em cada peça, com link.
   - Alternativamente, fica só na montagem, com participantes explícitos.
   - Fixadores e desenhos enxergam esses furos.
5. **Nada modificado fica escondido.**
   - Salvar age sobre o documento ativo. Salvar tudo lista cada arquivo modificado e o motivo.
   - Componente novo exige nome e pasta.
   - Empacotar verifica referências.
6. **O comando se explica.**
   - O ícone mostra o resultado.
   - A dica tem dois estágios: primeiro nome e atalho, depois um mini fluxo entrada → operação → resultado.
   - A barra de status mostra o próximo passo.
   - Os painéis seguem suporte → modo → parâmetros → avançado (B20).
7. **Toda feature tem peça de teste.**
   - O corpus de peças de referência (B53) roda em cada commit:
     - recompute;
     - medidas;
     - malha fechada;
     - mudar e voltar cada cota;
     - salvar e abrir;
     - captura visual.
   - A qualidade gráfica (B61) nunca altera o modelo, só a vista.

## O que muda no backlog
A **onda 0** entra antes das demais:

| Item | O que é |
|---|---|
| B53 | Corpus de peças |
| B54 | Edição no contexto |
| B55 | Sketch, referências e publicações na montagem |
| B56 | Salvar tudo |
| B57 | Furo entre peças |
| B58 | Conexão parafusada e rebites por pontos |
| B59 | Árvore e diagnóstico |
| B60 | Ícones e tooltips |
| B61 | Qualidade gráfica |

O B22 perde a parte de "editar vendo as outras", que foi para o B54.

## Alternativas consideradas
- **Seguir item por item do backlog antigo:** corrige sintomas, mas mantém as causas (contexto duplo, feature de montagem sem dono, salvar ambíguo).
- **Copiar um único CAD:** o CATIA tem as melhores regras de referência (publicações, objeto em trabalho). O Inventor tem o melhor assistente de fixação. O SolidWorks tem o melhor furo entre peças e os estados de link. O Fenkra combina os três.

## Consequências
- ✅ Bugs de referência e de contexto viram estados visíveis e testáveis.
- ✅ O corpus pega regressões antes do usuário.
- ⚠️ O B54 e o B55 mexem no modelo da montagem (`Assembly` ganha sketches, referências e publicações) e no app (viewport com peça ativa). É a maior mudança desde o B15.
- ⚠️ Itens marcados como "não verificado" na pesquisa precisam ser validados antes de virar regra, entre eles:
  - cores do Edit Part do SW;
  - tooltips do V5;
  - comprimento automático do Bolted Connection.
