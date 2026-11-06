# ADR 0046: curso dentro do programa, mapa de macroprocessos e perfis neutros de fluxo

Estado: aceita. Itens B323, B324 e B325.

## Contexto
O programa não ensinava a si mesmo. Quem chega de outro jeito de trabalhar precisava descobrir sozinho
onde ficam as coisas, e a decisão de 05/11/2026 proíbe citar nome de software proprietário, inclusive
como "modo de compatibilidade". Já existiam as predefinições de atalhos e os estilos de navegação
(`keymap::Preset`, `NavStyle`), o padrão de pergunta única da primeira abertura (`sugestao_leve_vista`,
B236) e a esteira de mídia (ADR 0034).

## Decisão
1. **Aula = dados.** `curso.rs` guarda aulas, passos e trilhas como tabelas constantes. Cada passo tem uma
   condição (`Cond`) que olha o documento: quantos comandos ativos de uma etiqueta, quantos elementos de
   referência ou superfícies, as contagens da montagem e do desenho, ou o nome da peça de exemplo. O que
   não deixa rastro no documento é `Cond::Manual` e a pessoa confirma.
2. **Reconhecimento em ordem e idempotente.** Só o primeiro passo pendente é avaliado; o resultado é o
   conjunto de bits gravado. Reabrir o programa, pular e retomar não mudam o que já foi feito.
3. **Progresso nas preferências**, em uma única seção `curso` (classe Interface), com `serde(default)`:
   configuração antiga abre sem curso. Pular nunca apaga; retomar só zera o que foi pulado.
4. **Mapa de macroprocessos como índice.** Cada etapa aponta para uma aula (o título da etapa é o da aula)
   e lista os ícones dos comandos. Testes garantem que toda bancada tem mapa e que toda etapa leva a uma
   aula da própria bancada, a comandos que existem na barra e a exemplos que existem.
5. **Perfis neutros por jeito de trabalhar**, nunca por produto. Cada perfil escolhe uma predefinição
   existente (a navegação acompanha), indica uma trilha e traz três linhas de comparação: o que a pessoa
   faz, o comando daqui (a localização "Bancada › Grupo" é calculada da própria barra) e o que muda.
   Os atalhos trocados à mão são preservados. Um teste proíbe nomes de software nos textos.
6. **Interface por pedidos.** `curso_ui.rs` só desenha e devolve `Pedido` (abrir exemplo, mostrar comando,
   mudar de bancada); o aplicativo executa pelos caminhos de sempre, com a pergunta de alterações não
   salvas. O painel é livre e encaixável (`paineis.rs`); a pergunta inicial é uma janela única.
7. **Áudio opcional e sem geração.** O texto de cada passo está sempre na tela. A narração só fica
   disponível se existir `voz/<aula>` no pacote de mídia; nada é gerado nem baixado.

## Consequências
- Aula nova é uma linha em `AULAS` e, se houver etapa, uma em `MAPAS`; os testes acusam o que faltar.
- Os destaques sobre o botão ou a alça esperada (B297) ainda não existem: o passo mostra o ícone e o
  botão "Mostre-me" aciona o comando.
- A reprodução de áudio não foi implementada (não há dependência de áudio no projeto).
