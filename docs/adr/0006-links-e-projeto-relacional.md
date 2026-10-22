# ADR 0006: Links por publicação explícita e projeto relacional

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
O projeto relacional (peças que se atualizam a partir de uma geometria mestre ou de outras peças) é
um pilar do SolidCadCraft. Em muitos CADs, links entre peças apontam para qualquer face ou aresta de
outro documento, o que os torna frágeis e difíceis de rastrear.

## Decisão
1. **Publicação explícita.** Um documento só pode ser referenciado pelo que ele **publica**
   (elementos geométricos, corpos, sistemas de coordenadas e parâmetros com nome). As publicações
   são a "interface pública" do documento.
2. **Tipos de link:** geometria, parâmetro, posição e corpo.
3. **Estado e política por link:** ativo / congelado / quebrado; atualização automática, ao abrir ou manual.
4. **Cache:** todo link guarda o último resultado; um link quebrado ou congelado continua funcionando com o cache.
5. **Grafo acíclico entre documentos**, verificado ao criar o link.
6. **Revisão-alvo:** o link aponta para "a revisão de trabalho" ou para uma revisão congelada específica.
7. **Variáveis geométricas:** todo elemento publicado ou ligado (superfície, plano, ponto, curva,
   eixo, sistema de coordenadas, corpo, parâmetro) é uma variável **tipada e com nome**. Features
   recebem variáveis como entrada; redirecionar a variável atualiza todos os usuários dela.
8. **Edição não destrutiva:** links e variáveis são reeditados ou redirecionados, nunca apagados e
   recriados; o ID interno se mantém e as referências dos comandos não quebram.
9. Links criados no contexto da montagem geram publicações automaticamente na peça de origem,
   para que o usuário não precise publicar à mão (mas elas ficam visíveis e gerenciáveis).

## Alternativas consideradas
- **Referência livre a qualquer entidade:** mais flexível, porém frágil e difícil de auditar.
- **Só cópia (sem associatividade):** simples, mas perde o principal benefício do projeto relacional.

## Consequências
- ✅ Alterações internas no mestre não quebram links enquanto as publicações existirem.
- ✅ "Onde é usado" e gerenciador de links ficam triviais: basta indexar as publicações.
- ⚠️ Usuário precisa entender o conceito de publicação; a criação automática no contexto reduz o atrito.
