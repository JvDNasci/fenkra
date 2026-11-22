---
name: fenkra-comando
description: Contrato de todo comando/feature do Fenkra (ADR 0107). Use SEMPRE ao criar ou mudar qualquer comando, feature, modo, campo de seleção, painel de comando, prévia, inverter, limite ou validação de qualquer bancada do Fenkra (sólidos, superfícies, chapa, estrutural, sketch, montagem, desenho, PMI), mesmo que o pedido pareça pequeno ou seja só um bug de um comando.
---

# Contrato de comando do Fenkra

Todo comando segue o [ADR 0107](../../../docs/arquitetura/adr/0107-contrato-de-feature.md):
**suporte → modo → parâmetros → avançado**, descrito por dados, com o mesmo comportamento em todas as
bancadas. Antes de mexer, leia o mapa da bancada em `docs/arquitetura/comandos/` e o
[grafo](../../../docs/arquitetura/grafo-do-fenkra.md). Comando novo já nasce no contrato; comando
antigo que você tocar migra o pedaço que tocou, sem deixar dois jeitos no mesmo comando.

## Checklist (marque cada item no relatório: ok, parcial com motivo, ou n/a)

### 1. Contrato e paridade

- [ ] Li o ADR 0107 e a seção da bancada no mapa (`comandos/*.md`) e na
      [auditoria de seleção](../../../docs/benchmark/detalhe/selecao-auditoria.md).
- [ ] **Listei os comandos irmãos** (mesma família em qualquer bancada: aba de aresta/curva/laço/perfil/
      superfície/bainha; extrusão sólida/de superfície/espessar/nervura; furos; dobra/joggle/veneziana;
      extrair uma/várias/com tangência; filete/chanfro; dividir/aparar/costurar...).
- [ ] **Copiei dos irmãos** opções, nomes, rótulos, faixas, ordem das seções e lógica. Toda opção que
      existe num irmão existe aqui com o mesmo nome e efeito, ou o relatório diz por que não se aplica.
- [ ] Irmãos são **um comando com modo** (padrão `aba_unif.rs`); trocar de modo leva os campos comuns.
- [ ] Textos de operação iguais: Adicionar, Remover, Interseção (Corpo novo onde houver).
- [ ] Entradas declaradas no descritor: tipo aceito, cardinalidade (1, N, cadeia), obrigatoriedade,
      validador, dica de 3 linhas. Campo vazio é `Vazio` explícito; **nunca** sentinela `UNSET_*`, ponto
      `[0;3]` ou comparação com texto de mensagem.
- [ ] Campo ativo, filtro de seleção, ordem do Tab e estado incompleto saem do descritor; não acrescentei
      `match` por comando em `entradas::campo_ativo`, `slots::fill`, `selecao::tipos_do_comando` nem na
      cascata de cliques de `app.rs`.
- [ ] Vale também para `RefKind` (superfícies): mesmo contrato que `FeatureKind`.

### 2. Seleção universal

- [ ] Todo campo de seleção usa a **caixa em árvore** (nós por tipo/origem): clique no 3D e na árvore,
      Ctrl soma, Shift intervalo, Del/X tira, validação na hora com mensagem no campo.
- [ ] Seleção ordenada quando a feature depende da ordem (seções, nó de tubos do B574).
- [ ] Nada de `faces_ui`, `plane_combo`, `sketch_combo`/`sketch_pick` nem texto com coordenada em campo
      novo.
- [ ] Perfil e guia aceitam **lista de curvas de origens mistas** (sketches, wireframe, arestas, extrações)
      com **cadeia tangente como recurso do campo** e junção na hora; botão direito "unir" no item.
      Até a decisão pendente 1 do ADR, a lista fica gravada no próprio comando.

### 3. Direção, limites, posição

- [ ] **Um** inverter: widget padrão "⇄ Inverter direção" ligado ao **mesmo campo** da seta clicável do
      3D. Sem caixa, rádio ou nome próprio. Se o sentido é o sinal do valor, declarei `sinal_do_valor` e a
      dica explica.
- [ ] **Limites** pelo tipo único (lado 1, lado 2, borda final; valor ou até plano, face, curva, aresta,
      sketch ou elemento de construção). Extrapolar, aparar e projetar automáticos; chave v2 no elemento.
- [ ] **Posição do material** (face interna, externa, começo do raio) onde há dobra ou espessura, com os
      mesmos nomes dos irmãos; as quatro combinações de inverter × posição dão sólido válido.

### 4. Validação, confirmação, rollback

- [ ] Validação **antes do OK**: validadores de campo e pré-cálculo leve mostram o erro **no campo**
      (`Erro{campo, causa, sugestao, acao}`), com a vista livre.
- [ ] Mensagem traduzível com argumentos (nada de `format!` que `tr` não traduz); o campo vem da
      estrutura, não do texto.
- [ ] OK **espera** o cálculo completo com progresso; o painel **não fecha** em falha.
- [ ] **Rollback**: falha no meio não deixa corpo parcial, dobra fantasma, `sheet_maps`, `previas` nem
      `tools` sobrando (estado guardado antes e restaurado em `Err`).
- [ ] Nunca escolhe referência em silêncio: ambiguidade vira erro com sugestão.

### 5. Cache e geometria

- [ ] Assinatura vem dos campos declarados; não depende de todos os corpos sem necessidade; nome, cor e
      visibilidade não invalidam.
- [ ] Estados calculados juntos e guardados (dobrado e planificado na chapa, B566); malha de exibição e
      prévia em cache, invalidadas só pela árvore (B568).
- [ ] **Extrair continua Extrair** (B576): extração vira recurso Extrair associativo, com o mesmo tipo
      geométrico; nunca Linha, Curva ou Junção de curvas.
- [ ] **Curva exata** no kernel (B559): linha/arco/círculo analíticos, spline como BSpline pela
      definição única de spline; polilinha só para desenho e picking.

### 6. Interface (B319)

- [ ] Cores só por tema; a peça sempre distinta do fundo; realce e retângulo de seleção sem RGB fixo.
- [ ] Ícone da família, gerado com `scripts/gerar_icones.py` em pasta temporária, copiando só os nomes
      novos.
- [ ] Painel livre e encaixável, aberto só durante o comando (ADR 0105); árvore transparente quando ele
      flutua.
- [ ] Prévia e alças no 3D por grandeza; rótulo editável.
- [ ] Dica por campo; tooltip em camadas com animação (`dica_anim.rs`); F1.
- [ ] Todo texto novo (rótulo, dica, mensagem, modo) com tradução em `crates/fenkra-app/src/i18n.rs`.
- [ ] Clique único roda uma vez; duplo clique prende; Enter/Esc; Ctrl+Enter aplica e continua.
- [ ] Passa no checklist de [interface.md](../../../docs/benchmark/detalhe/interface.md).

### 7. Testes

- [ ] Documento montado em código, recompute, conferência de **status, volume e caixa envolvente**.
- [ ] Teste do inverter conferindo a caixa invertida; das posições do material; de entrada inválida dando
      erro **antes** do OK no campo certo; de falha sem corpo parcial.
- [ ] **Teste de paridade**: os campos comuns do grupo de irmãos existem em todos, com o mesmo id, rótulo,
      faixa, inverter e posição (extensão de `checklist_comandos.rs`).
- [ ] Arquivo antigo abre igual (volume e caixa) depois da migração.

### 8. Material e registro

- [ ] Nota do comando em `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Comandos\<Comando>.md`, com
      `Templates/Comando.md`, em linguagem simples: o que faz e como funciona no kernel. Caminho na resposta.
- [ ] Mapa da bancada em `docs/arquitetura/comandos/` e grafo atualizados se entradas, motor ou saídas
      mudaram (ou peça ao agente `arquiteto`).
- [ ] Fonte consultada em `docs/arquitetura/procedencia.md`; decisão nova de arquitetura vira ADR.
- [ ] Sem nome de software proprietário: "softwares de modelagem CAD avançados".

## Quando parar e perguntar

- O comando precisaria fugir do contrato (ex.: ciclo próprio): diga por quê e peça decisão.
- Um irmão faz diferente e não está claro qual é o certo: leve ao agente `arquiteto` antes de copiar.
- A mudança toca uma decisão pendente do ADR 0107: siga a recomendação do ADR e marque no relatório.
