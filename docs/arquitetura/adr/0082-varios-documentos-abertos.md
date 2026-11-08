# ADR 0082: vários documentos abertos ao mesmo tempo (B363)

## Contexto

O app guarda um documento só: `App::open` (`OpenDoc`: documento, arquivo, entrada no projeto, revisão
congelada, ramo e "modificado") e, ao lado, tudo o que depende dele espalhado em campos do `App`: o
histórico de desfazer (`hist`), o resultado do cálculo (`result`) e a geração pedida ao `Worker`, a
seleção (`sel`, `multi`, `picks`), a sessão de comando (`session`), o sketch em edição, a câmera e a
cena da viewport (`viewport`, `scene_dirty`), a montagem e o desenho abertos.

Trocar de peça hoje é abrir o arquivo de novo: o estado de quem sai se perde (câmera, seleção, comando
aberto, desfazer). O único caminho que preserva estado é o da peça ativa na montagem, que usa
`guardar_aberto` / `devolver_aberto` (`DocGuardado`: `OpenDoc` mais `Historico`) para tirar o
documento de cena e devolvê-lo depois. Esse par mostra que a troca funciona, mas guarda pouco: o
resultado do cálculo é refeito ao voltar e a câmera e a seleção não voltam.

O B363 pede barra e atalho para ver e trocar entre os documentos abertos sem perder o estado de cada
um, e a opção de abrir um documento em janela nova.

## Decisão

1. **Um contêiner por documento.** Nasce `DocAberto` em módulo próprio (`docs_abertos.rs`) com tudo o
   que é do documento: `OpenDoc`, `Historico`, `Option<RecomputeResult>` com a geração dele, seleção,
   sessão de comando, sketch em edição, câmera e opções de exibição da viewport, e o estado da
   montagem ou do desenho quando o documento é um deles. O que é do programa (configurações, tema,
   painéis, roda, biblioteca, projeto aberto) fica no `App`.
2. **Ativo no lugar, os outros guardados.** O `App` continua com os campos de hoje para o documento
   ativo (nada no resto do código muda de nome) e ganha `abertos: Vec<DocAberto>` com os de fundo e o
   índice do ativo na barra. Trocar é `guardar_aberto` ampliado: move os campos do ativo para um
   `DocAberto` e move o escolhido para os campos do `App`. É troca por `std::mem::replace`, sem
   clonar documento nem malha; o resultado calculado vai junto, então voltar não recalcula.
3. **Cálculo só do ativo.** O `Worker` continua com uma fila. Um documento de fundo com pedido em
   andamento tem o pedido descartado ao sair de cena e marcado "a recalcular"; ao voltar, pede de novo.
   Isso evita mexer no contrato de gerações do `Worker` (B259) e mantém a memória sob controle.
4. **Barra de documentos** acima da viewport, no padrão B319 (cores do tema, encaixável, sem cor
   fixa): uma aba por documento com ícone do tipo (peça, montagem, desenho), nome, ponto de
   "modificado" e fechar. Ctrl+Tab e Ctrl+Shift+Tab percorrem na ordem do último uso; a lista de
   peças do rodapé vira um atalho para a mesma lista. Fechar um documento modificado pergunta.
5. **Janela nova é outro processo.** "Abrir em janela nova" chama o próprio executável com
   `FENKRA_ABRIR=<arquivo>` (o caminho já existe). Viewports nativas do egui dividem o contexto wgpu
   e a thread da interface, e o kernel não é reentrante para dois documentos calculando juntos na
   mesma thread de cálculo; processos separados isolam os dois e reaproveitam a abertura de hoje. A
   montagem que referencia uma peça aberta em outro processo vê a peça pelo arquivo salvo, como hoje.
6. **Limite de memória.** Com mais de oito documentos ou acima da cota de memória do modo leve, o
   documento de fundo mais antigo solta o `RecomputeResult` (fica "a recalcular") e mantém só o
   documento e o desfazer.

## Alternativas descartadas

- **Vários `App` no mesmo processo**: duplicaria painéis, tema, roda e worker; o `App` não foi feito
  para isso e a troca de documento ficaria mais cara que hoje.
- **Clonar o documento ao trocar**: simples, mas perde o resultado (recalcula toda troca) e dobra a
  memória do desfazer.
- **Janela nativa do egui (viewport múltipla) para a janela nova**: exigiria um segundo contexto de
  render e um segundo worker na mesma thread de interface; o ganho sobre o processo separado é pequeno.

## Consequências

- O estado de cada documento passa a ter dono claro; funções que leem `self.result` continuam
  valendo porque o ativo fica nos mesmos campos.
- Campos novos do `App` precisam decidir se são do documento ou do programa; o teste da troca falha se
  um campo do documento não voltar igual (ver plano).
- Dois processos podem abrir o mesmo arquivo: o segundo abre só leitura com aviso, por um arquivo de
  trava ao lado do documento (criado no passo 5 do plano).

## Plano de implementação

1. `docs_abertos.rs`: `DocAberto`, `guardar(app) -> DocAberto` e `ativar(app, DocAberto)`, migrando
   `DocGuardado` para ele (a peça ativa da montagem passa a usar o mesmo par). Teste: abrir duas peças
   montadas em código, trocar de uma para outra e voltar; conferir documento, desfazer, seleção, câmera,
   status e volume do resultado iguais, sem novo pedido ao worker.
2. Abrir arquivo, Novo e exemplos da tela inicial passam a acrescentar um `DocAberto` em vez de
   substituir o ativo; "modificado" por documento; fechar com pergunta.
3. Barra de documentos (ícones da família nova, tradução PT/EN, dica por aba), Ctrl+Tab no
   `keymap.rs`, lista do rodapé ligada à barra. `FENKRA_DEMO_DOCS=3` abre três exemplos para a captura.
4. Cálculo só do ativo, com "a recalcular" ao voltar; limite de memória do item 6 da decisão.
5. "Abrir em janela nova" no menu da aba e no menu de contexto da árvore do projeto: novo processo
   com `FENKRA_ABRIR`; trava só leitura no segundo processo.
6. Material do comando em `Comandos/Documentos abertos.md` e item da checklist de interface.
