# Bugs e comandos que faltam (B353, kart)

Um item por linha, pronto para virar backlog. Prioridade sugerida: A (quebra o fluxo do projeto),
M (tem contorno), B (acabamento). `K-nn` é o número usado nos comentários de `validacao/kart/src`.

| ID | Peça | Passos | Esperado | Obtido | Bancada | Prio |
|---|---|---|---|---|---|---|
| K-01 | Todas ligadas ao mestre | Importar um parâmetro publicado por link (`add_link` com valor) | O parâmetro chama `mestre_EE`, como o nome de uma curva ligada | Nasce `mestre_EE1`, porque `unique_name` sempre põe número; curvas não têm o sufixo. A fórmula escrita com o nome esperado falha com "parâmetro desconhecido" | PLM/links | M |
| K-02 | CH-01, CH-02, CH-06 | Publicar polilinha no mestre, importar por link, usar `CurveRef::Segment` como caminho do membro | Cada trecho vira membro, como na polilinha local | "linha de comprimento zero (pontos coincidentes)": o vértice de curva ligada não acha o índice. A mensagem não diz o motivo. Contorno: publicar cada trecho como curva própria | Estrutural | A |
| K-03 | KART, DW-KART | Abrir vista de desenho de montagem que só tem subconjuntos | Vista com todas as peças | Vista vazia, sem aviso: `source.rs` só percorre `NodeKind::Part`. A lista de peças da mesma folha sai certa. Contorno: `KART-PLANO.fka` sem subconjuntos | Desenho | A |
| K-04 | KART, regressão | Mudar `EE` no mestre e rodar Atualizar links na montagem de topo | Propaga para as peças dos subconjuntos | 0 peças atualizadas: `links::pecas` não desce em subconjunto. Só funciona rodando em cada `.fka`. A mudança de entre-eixos não chega pelo topo | Montagem | A |
| K-05 | KART, pacote | Gerar `.fkv` de montagem com peça esqueleto | O esqueleto fica de fora | Falha inteira: "'MESTRE Desenho mestre' não tem corpo visível para mostrar" | PLM/pacote | A |
| K-06 | KART, pacote | Gerar `.fkv` de montagem com uma peça sem corpo (comando em erro) | Pacote sai, com aviso da peça que faltou | O pacote inteiro falha por uma peça | PLM/pacote | M |
| K-07 | TESTE-TRANSMISSAO | Junta de revolução acionada mais acoplamento de engrenagem (11:82), simular de 0 a 360° | Coroa gira 48,3° numa volta do pinhão | Até 180° certo (−24,146°); de 225° em diante `ok=false` e a coroa para. `kin.rs` aplica `wrap180` no ângulo antes de multiplicar pela razão: o pinhão não passa de meia volta | Montagem | A |
| K-08 | EX-12 | Superfície de varredura com perfil circular (`SweepProfile::Circle`) numa spline 3D, nos modos Corrigido e Frenet | Tubo fino | "não foi possível varrer o perfil ao longo da guia", no comando Engrossar, não na superfície. A varredura sólida na mesma spline é o contorno (ver EX-12B em resultados.md) | Superfícies | M |
| K-09 | QUADRO | Membros retos com canto `Encaixe` chegando no meio de uma longarina dobrada | Toda ponta recortada no tubo | Só o CH-04 sai recortado; CH-03, CH-05 e CH-07 ficam com corte reto (a1 = 0°) e entram no tubo da longarina. Sem aviso | Estrutural | M |
| K-10 | RD-01, RD-02 | Análise de interferência com peça dentro do aro | Volume comum menor que o da menor peça | Volume comum igual ao da peça inteira, e aro × pneu maior que o aro: ver resultado do teste "invertido" em relatorio.md | Montagem | A |
| K-11 | Abrir arquivo | Passar um `.fkp` na linha de comando ou dar duplo clique | Abre o arquivo | `main.rs` só trata `.fkv`; `.fkp`, `.fka` e `.fkd` são ignorados. Foi preciso criar `FENKRA_DEMO_ARQUIVO` | Interface | M |
| K-12 | Documentação | `FENKRA_ABRIR=<arquivo>` em `docs/desenvolvimento.md` | Abre o arquivo | A variável recebe nome de comando. Tabela corrigida neste trabalho | Interface | B |
| K-13 | EX-05, EX-09 | Chanfro ou filete por ponto clicado em aresta circular | O ponto na aresta basta | Exige o ponto médio exato da aresta, que na circular fica do lado oposto da costura; "referência perdida" sem sugerir a aresta vizinha. Pela interface o clique resolve; na API e no `fenkra-mcp` é armadilha | Sólidos | M |
| K-14 | EX-09, EX-11 | Extrusão de corte com "inverter" ligado, pensando como na adição | Mesmo sentido da adição | O corte padrão vai contra a normal, e a adição a favor. A mensagem "não removeu material" ajuda, mas o usuário descobre por tentativa | Sólidos | B |
| K-15 | DW-KART | Desenho de conjunto de 85 peças | Vistas em segundos | 135 s para quatro vistas, sem barra de progresso conhecida na API | Desenho | M |
| K-16 | KART | Interferência do conjunto, 85 peças | Menos de 30 s | 123 s | Montagem | B |
| F-01 | TR-01, TR-02, TR-03 | Gerar roda e corrente 219 | Biblioteca de transmissão gera | Fora da biblioteca (B354 já cobre). Contorno: dente por padrão circular de 82 e corrente por círculos no caminho | Transmissão | A |
| F-02 | EX-03, DI-11 | Inserir rolamento 50×80 e 25×47 | Família de rolamentos na Biblioteca | Não existe nenhuma, depois do B374 também. Contorno: revolução | Biblioteca | A |
| F-03 | DI-07 | Inserir terminal rotular | Item de biblioteca | Não existe. Contorno: revolução de marcação | Biblioteca | M |
| F-04 | FX-02, FX-04 | Inserir porca autotravante | Família ISO 7042 ou equivalente | Não existe. Contorno: porca comum ISO 4032 | Biblioteca | M |
| F-05 | DI-01 | Criar anel na Forma livre | Primitiva de anel, ou ponte entre faces | `PrimitivaForma` só tem Caixa, Plano, Cilindro e Esfera. Só deu pela API de teste (`novo_para_teste`), que a interface não tem | Forma livre | M |
