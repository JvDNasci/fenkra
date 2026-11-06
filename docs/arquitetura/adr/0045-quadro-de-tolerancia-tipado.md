# ADR 0045 — Quadro de tolerância geométrica tipado e consultor por norma

## Contexto

O quadro de GD&T guardava texto: `tol` e `datums` em strings, `zone_dia` e `modifier` num único caractere. Nada
impedia uma planeza com referência, um modificador de material num plano de superfície, a letra I como referência
ou um Ⓢ que as normas atuais não têm. Os símbolos saíam como glifos de fonte, e a fonte padrão não tem os circulados.

## Decisão

- **Módulo `fenkra-core::gdt`:** enums `Zona`, `Modif`, `Elemento`, `Norma`; `compat()` é a matriz de validade por
  característica, elemento, posição (valor, referência, chamada) e norma; `GdtExtra::validar()` devolve erros (a norma
  não admite) e avisos (depende da norma ou do contrato); `consultar()` confere o modelo todo.
- **Compatível com arquivos antigos:** `GdtExtra` ganhou campos opcionais com `serde(default)`. `zone_dia` e `modifier`
  continuam lidos; `resolvido()` os traduz para os tipos e `migrar()` os passa para os campos novos quando a pessoa edita.
  Ⓢ não tem correspondente e sai da leitura, com aviso.
- **Uma só montagem das casas:** `GdtExtra::linhas()` produz as linhas do quadro; o 3D, o desenho e o texto da árvore
  usam a mesma função, em vez de três cópias da concatenação.
- **Símbolos como traços:** `GdtChar::tracado()` e `Modif::tracado()` dão polilinhas no quadro unitário. Quem desenha
  (egui e folha) converte; não há mais dependência de glifo.
- **Regras com palavras próprias:** nenhuma tabela de norma paga foi copiada. Erro vira bloqueio só quando a regra é
  geral; o que muda entre ISO e ASME vira aviso. A matriz precisa de conferência na edição adotada pelo projeto.
- **Letras de referência:** nunca I, O ou Q; depois de Z, letra dobrada. `proxima_letra()` escolhe a primeira livre.

## Consequências

- A troca de dados (STEP) ainda não escreve PMI semântico: `step_nome()` e `step_forma()` guardam o mapeamento para
  `geometric_tolerance_with_modifiers` e `tolerance_zone_form`; o exportador usa quando existir.
- Mensagens montadas com a letra ou o nome do modificador no meio não passam pela tabela de tradução.
