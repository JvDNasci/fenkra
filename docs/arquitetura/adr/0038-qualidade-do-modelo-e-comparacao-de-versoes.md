# ADR 0038: qualidade do modelo com perfil travável e comparação de versões por feature

Estado: aceita. Itens B305 e B214.

## Contexto
O PLM precisava de uma regra de peso e saúde da peça que valesse numa equipe e de um jeito de ver o
que mudou entre duas versões. A lista de conferência da liberação (B265) já existia; o diário
(`.diario.jsonl`) só acrescenta linhas.

## Decisão
1. O checador é uma função pura (`qualidade_modelo::avaliar`) que recebe `Medidas` prontas. Quem mede
   (o recálculo, no app) fica separado de quem decide (o perfil).
2. O perfil vive em `.qualidade.json` na raiz do projeto, gravado de forma atômica. Perfil travado só
   endurece para o usuário local (limite menor, gravidade maior, mais regras e momentos, teto de
   montagem menor); só o papel `administrador` trava, destrava e afrouxa.
3. Exceção é evento `Excecao` no diário (autor = quem aceitou, regra, hash da versão, motivo). Vale só
   para a versão em que foi dada: editar a peça exige justificar de novo. Em perfil travado, regra que
   bloqueia só o administrador libera.
4. A comparação casa as features por `Id` e compara o conteúdo campo a campo pelo JSON serializado,
   para um tipo novo de comando entrar no diff sem código extra. A posição relativa só conta entre
   features que existem nos dois lados, para uma inserção não parecer mover todas as outras.
5. O mesmo comparador alimenta a conferência de migração (`ComparacaoDocs::como_conferencia`).

## Consequências
O limiar de "minúsculo" (face menor que 0,01 mm², aresta menor que 0,05 mm) está em constantes de
`app/plm.rs`. O administrador no PLM local é só o papel `administrador` em `Project::papeis`; a
identidade real depende do B215.
