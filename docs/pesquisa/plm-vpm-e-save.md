# PLM/VPM, save management e arquivos

> Pesquisa para o Fenkra (Rust + OpenCascade, egui em português), datada de 2026-10-30.
> Escopo: salvamento e gestão de arquivos, PLM/VPM, referências entre arquivos, empacotamento, multiusuário e travamento.
>
> **Legenda:**
> - **[V]** confirmado em fonte pública (página lida, ou trecho que a busca citou da fonte).
> - **[C]** conhecimento geral não confirmado nesta pesquisa.
> - Quando só tenho o resumo de uma busca (a página inteira deu 403 ou não carregou), escrevo "(resumo de busca)".
>
> **Limitações:**
> - As páginas help.CAD avançado A.com carregam por JavaScript e voltaram só com a navegação do site. Para o CAD avançado A usei blogs de revendas, resumos de busca e o que já conheço.
> - Vários blogs (Javelin, Hawk Ridge, 3DCS, PLM Group) devolveram 403.
> - Não consegui nenhuma fonte pública detalhada sobre CAD avançado F/PLM avançado, CAD avançado G/PLM avançado nem CAD avançado H. Esses três ficam como [C] ou ausentes.

---

## 1. Resumo e principais lacunas

**Resumo (10 linhas)**

1. O CAD avançado B separa "o que está na memória" de "o que está no disco" com o **Save Management**. Ele mostra cada documento (estado, nome, caminho, ação) e deixa decidir Salvar, Salvar como ou Salvar Auto antes de gravar qualquer coisa. [V]
2. **Propagate Directory** e **Send To** existem porque, no CAD avançado B, um link entre arquivos é nome mais caminho. Copiar um produto exige reapontar todos os links de uma vez. [V]
3. O **Pack and Go** do CAD avançado A e o do CAD avançado C resolvem o mesmo problema com mais opções: layout plano ou hierárquico, prefixo/sufixo, zip, desenhos, bibliotecas, log. [V]
4. As referências são frágeis em todos eles e todos têm uma ordem de busca. CAD avançado A: memória, pastas de Referenced Documents, pasta da montagem, último caminho, caminho salvo, depois "Browse". CAD avançado C: workspace, workgroup, bibliotecas, depois o diálogo Resolve Link. [V]
5. Renomear ou mover fora do CAD quebra links. A solução profissional é uma operação do gerenciador (Design Assistant, PDM Move Tree, Update References) que reescreve os dependentes. [V]
6. PDM, Vault e PLM avançado tratam **versão** (automática a cada check-in) e **revisão** (rótulo deliberado que marca uma versão aprovada) como coisas diferentes. O Fenkra já tem essa ideia, com versão por hash e revisão A, B, C. [V]
7. O ciclo de vida da plataforma PLM avançada (Privado, Em trabalho, Congelado, Liberado, Obsoleto) permite transição em lote, incluindo filhos, com regra "filhos antes do pai". [V]
8. O Vault numera na hora do **Copy Design** e reserva o número só quando a cópia termina. Cada esquema tem no máximo uma sequência automática. [V]
9. O Fenkra tem um índice JSON por pasta, versões endereçadas por hash e revisões congeladas, que são uma boa base. Faltam ID estável entre arquivos, Salvar tudo, Salvar como, empacotar, onde-usado, travamento, multiusuário seguro e derivados do conversor.
10. O maior risco hoje é **perda silenciosa de dados**: abrir outro arquivo não pergunta, fechar a janela não pergunta, a escrita não é atômica e duas pessoas na mesma pasta se sobrescrevem.

**Principais lacunas do Fenkra neste tema**

| # | Lacuna | Onde aparece |
|---|---|---|
| 1 | Sem plano de salvamento (Salvar tudo / Save Management) e sem aviso de alterações não salvas ao abrir outro arquivo, editar peça pela montagem ou fechar o programa | `app.rs:795`, `app.rs:864`, `asm_ui.rs:897`, `main.rs:38` |
| 2 | Links entre arquivos são caminhos de texto. `rel_path` só relativiza descendentes e o link externo sem projeto resolve a partir do diretório de trabalho. Não há ID estável nem busca em ordem | `asm_ui.rs:317`, `app.rs:2267`, `fenkra-core/doc.rs:449` |
| 3 | Sem Salvar como, Empacotar, Renomear/Mover seguros, Onde é usado e Reparar links | ausentes |
| 4 | Multiusuário sem proteção: sem trava, sem detecção de conflito, contadores de ID e PN colidem em pasta compartilhada | `fenkra-project/lib.rs:154,169,176` |
| 5 | Sem modelo de documento derivado (importado), sem estrutura de produto no projeto (só peças) e sem baseline/efetividade | `fenkra-project/lib.rs:72` |

---

## 2. Fichas por comando/função

Ordem das fichas:

1. Salvar e Salvar tudo
2. Gerenciamento de salvamento
3. Salvar como e cópias
4. Propagar diretório e Enviar para
5. Mesa e gerenciador de relações
6. Empacotar
7. Referências entre arquivos: caminhos e busca
8. Renomear e mover sem quebrar
9. Substituir referência
10. Recuperação e backup automático
11. Check-in/check-out e travamento
12. Pasta compartilhada e multiusuário
13. Versão × revisão
14. Ciclo de vida e fluxo de aprovação
15. Estrutura de produto
16. Onde é usado
17. Baseline e efetividade
18. Regras de part number
19. Copiar projeto
20. Documentos derivados e conversor
21. Montagem: salvar contexto
22. Histórico, comparação e auditoria
23. Propriedades e metadados
24. Permissões

---

### Salvar e Salvar tudo (CAD avançado B: Save / Save All / Save Management / CAD avançado A: Save / Save All / CAD avançado C: Save / Save All / outros: Fusion Save / CAD avançado E salva sozinho / Fenkra: Ctrl+S por bancada)

- **O que faz:** grava o documento ativo (Salvar) ou todos os modificados (Salvar tudo). No Fenkra, "Salvar" deveria agir sobre o documento ativo e "Salvar tudo" deveria listar cada arquivo modificado e o motivo (ADR 0011, princípio 5).

- **CAD avançado B — no detalhe:**
  - Save grava o documento corrente. Save All grava todos os documentos abertos e modificados. [V, EDS]
  - O guia da EDS diz que o Save Management grava só o que é novo ou modificado e deixa o resto, enquanto o Save comum reescreve o modelo todo, o que pesa em servidores. [V]
  - Em Tools ▸ Options ▸ General ▸ Data Save há três modos: sem backup automático, backup automático (padrão 30 min) e backup incremental. O incremental guarda os documentos abertos num diretório temporário, e nem toda bancada o suporta. [V]
  - O documento só aparece como "modificado" se o CAD avançado B detecta mudança. Documentos dependentes (componentes) ficam em Save Auto quando o pai é salvo com novo nome. [V]
  - Um documento de só leitura não pode ser salvo no mesmo caminho. O botão Save fica indisponível para ele, e o usuário precisa de Save As. [V]

- **CAD avançado A — no detalhe:**
  - Save grava o ativo. Save All grava todos os abertos e modificados e abre a lista de modificados com opções ("Save all", "Don't save", "Save modified"). [C]
  - Ao salvar uma montagem, o CAD avançado A grava o caminho e o nome de cada componente referenciado dentro dela. Por isso renomear fora do CAD avançado A quebra o link. [V, resumo de busca de c_Saving_an_Assembly_and_Its_Components]
  - Existe aviso de reconstrução e de componentes não salvos (Rebuild/Save prompt). [C]

- **CAD avançado C — no detalhe:**
  - Save grava o documento ativo. Save All grava todos os abertos e modificados, e arquivos novos pedem nome e pasta. [C]
  - Durante o Save All, o CAD avançado C verifica as referências: um componente novo criado dentro da montagem exige destino antes. [C]
  - Em projeto Vault, o arquivo só pode ser salvo se estiver em check-out. [C]

- **Outros:**
  - Fusion: o salvamento é contínuo na nuvem, e as versões só são numeradas quando criadas de propósito. [V, blog fornecedor de CAD avançado, resumo de busca]
  - CAD avançado E: não há "salvar". Toda alteração entra no histórico, e a versão é um instantâneo nomeado e imutável. [V]

- **Fenkra hoje:**
  - `App::save` (`app.rs:795-829`) tem três caminhos. Revisão congelada bloqueia com aviso. Peça de projeto (`entry`) grava no projeto e cria versão por hash (`Project::save`, `lib.rs:205`). Peça avulsa grava com `std::fs::write` (`app.rs:819`) num arquivo escolhido por diálogo.
  - A montagem tem outro `save` (`asm_ui.rs:920`) e o desenho tem outro (`dwg_ui.rs:303`). Ctrl+S muda de efeito conforme a bancada (`app.rs:6449-6457`).
  - O menu Arquivo só tem "Salvar Ctrl+S" da peça (`app.rs:2523`).
  - Salvar a montagem não salva as peças editadas. "Voltar à montagem" chama `save_pub` (`asm_ui.rs:909-911`), que salva só o documento aberto.
  - Não existe Salvar tudo, Salvar como nem lista de modificados. O indicador "•" existe no título (`app.rs:2607`, `asm_ui.rs:1031`).
  - Bugs prováveis:
    - A escrita não é atômica: `fs::write` trunca o arquivo antes de gravar, e uma queda no meio perde o único exemplar do arquivo avulso. Só o índice do projeto usa gravar-em-temporário-e-renomear (`lib.rs:154-159`).
    - `open_path` e `load_doc` não verificam `open.modified` (`app.rs:784-791`, `864-876`). "Nova peça" e "Abrir peça" descartam edições sem perguntar. Só `open_entry` salva sozinho antes de trocar (`app.rs:1153`).
    - `edit_part` carrega a peça por cima do documento aberto, também sem checar (`asm_ui.rs:897-900`).
    - `main.rs` não trata o fechamento da janela. Não há aviso de alterações não salvas.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** um único grupo Salvar / Salvar tudo / Salvar como na barra, igual nas três bancadas (B56).
    - Ctrl+S salva o documento ativo.
    - Ctrl+Shift+S abre o **plano de salvamento** (ficha seguinte).
    - O menu Arquivo repete os três.
  - **Regras que evitam bugs simples:**
    - Toda troca de documento (abrir, nova, editar no contexto, fechar) passa por `confirmar_descartes()`, que consulta a lista de modificados.
    - Gravação atômica para todo formato: escrever `x.fkp.tmp`, `fsync`, renomear sobre o destino, e manter uma geração `.bak` (ou cópia em `.fenkra/recuperacao`).
    - Antes de gravar, comparar mtime e hash do disco com os da leitura. Se mudaram fora do Fenkra, mostrar "o arquivo mudou no disco" com Sobrescrever, Salvar como ou Recarregar.
  - **Casos de teste:**
    - Abrir B com A modificada oferece Salvar/Descartar/Cancelar.
    - Fechar a janela com montagem e uma peça modificadas lista as duas.
    - Matar o processo durante o salvamento deixa o arquivo antigo íntegro ou o novo completo.
    - Editar o arquivo por fora e salvar dispara o aviso de conflito.
    - Ctrl+S na peça em contexto salva a peça, não a montagem.

---

### Gerenciamento de salvamento / plano de salvamento (CAD avançado B: Save Management / CAD avançado A: lista do Save All / CAD avançado C: lista do Save All / Fenkra: inexistente)

- **O que faz:** mostra, antes de gravar, cada documento com estado, nome, caminho e a ação que será feita, e deixa mudar nome e pasta de cada um. É o painel de controle do salvamento.

- **CAD avançado B — no detalhe:**
  - File ▸ Save Management abre a lista de arquivos abertos com quatro colunas. [V, tutor de montagem]
    - **State:** novo, modificado ou só leitura.
    - **Name:** nome atual, inclusive o novo depois de um Save As.
    - **Path:** local atual, inclusive o novo.
    - **Action:** o que o CAD avançado B fará ao clicar em OK.
  - Botões [V]:
    - **Save** mantém nome e local. Só vale para modificado e não somente leitura.
    - **Save As** abre diálogo para novo nome e local.
    - **Save Auto** é aplicado por padrão a dependentes modificados (componentes de uma montagem) e ao pai quando um componente é salvo com novo nome ou caminho.
    - **Propagate Directory** leva os dependentes do pai para o novo diretório depois de um Save As (ficha própria).
    - **Reset** limpa a ação só do selecionado.
    - **Enable Independent Saves** impede que o CAD avançado B atribua Save Auto aos dependentes. Vem desligado.
  - As ações só valem ao clicar OK. Cancel não altera nada. [V]
  - Cuidado documentado: expandir a coluna Location e conferir o caminho inteiro, senão o usuário pode sobrescrever os originais a cada gravação. [V, 3DCS, resumo de busca]
  - O tutor recomenda Save Management no lugar de Save, Save As e Save All, porque mostra o que será salvo e onde. [V]
  - Peças ligadas por link podem não aparecer com "Save" na ação após a propagação. [V, 3DCS, resumo de busca]

- **CAD avançado A — no detalhe:**
  - Não há um painel único como o do CAD avançado B. O fluxo é Save All, com a lista de modificados, e Save As com opção "Include all referenced components". [C]
  - O botão Advanced do Save As abre o **Save As with References**, uma tabela editável por duplo clique. Células editadas ficam verdes, e o menu de contexto traz "Undo change". Dá para colocar prefixo ou sufixo em todos. [V, resumo de busca]
  - Se um componente é renomeado nessa tabela, as edições nele afetam só a cópia nova, não o original. [V, resumo de busca]

- **CAD avançado C — no detalhe:**
  - Não há painel equivalente no Save. A lista de dependentes aparece no Save All. [C]
  - O Copy Design do Vault tem painel com o arquivo original e o novo nome, por esquema de numeração (ficha 19). [V]

- **Outros:** Fusion avisa sobre referências a versões antigas em um painel de "Get Latest" na hora de salvar. [V, resumo de busca]

- **Fenkra hoje:** não existe. O mais próximo é a lista "Recentes" da tela inicial (`app.rs:1047-1074`), que só mostra arquivos existentes e pula os ausentes sem avisar.

- **Melhor prática para o Fenkra:**
  - **Fluxo (suporte → modo → parâmetros → avançado):**
    - *Suporte:* conjunto de documentos abertos (peça ativa, peças em edição, montagem, desenhos, derivados).
    - *Modo:* Salvar tudo (padrão), Salvar só selecionados, Salvar como.
    - *Parâmetros:* por linha, com colunas **Estado** (Novo, Editado, Dependência, Derivado desatualizado, Só leitura, Conflito no disco), **Nome**, **Pasta**, **Ação** (Salvar, Salvar como, Ignorar).
    - *Avançado:* "Habilitar salvamentos independentes" (copiando o botão do CAD avançado B), versão do projeto e comentário da versão.
  - **Regras:**
    - Novo documento exige nome e pasta (nunca grava em lugar adivinhado).
    - Pasta fora da pasta da montagem mostra aviso de caminho absoluto.
    - Pai com filho novo marca o pai como Dependência.
    - OK aplica tudo numa transação, e uma falha reverte as gravações já feitas (usar `.tmp` e renomear no fim).
  - **Casos de teste:**
    - Montagem com 3 peças, uma modificada e uma nova: o plano lista as duas com motivos corretos.
    - Cancelar não toca em nenhum arquivo.
    - Ignorar uma peça nova mantém o link como pendente com aviso.
    - Plano com pasta somente leitura mostra o erro por linha.

---

### Salvar como e cópias (CAD avançado B: Save As / Save As New / CAD avançado A: Save As, Save as copy and continue / open / CAD avançado C: Save As, Save Copy As / Fenkra: ausente)

- **O que faz:** grava com outro nome ou local. Há duas intenções diferentes: **renomear o mesmo documento** (links acompanham) e **criar um documento novo a partir deste** (identidade nova).

- **CAD avançado B — no detalhe:**
  - Save As renomeia a peça mantendo o identificador (citação de documentação da fornecedor de CAD avançado em uma resposta do Eng-Tips). Já Send To / New From gera nome e UUID novos. [V, fórum]
  - A opção "Save as new document" não existe no Save Management (mesma discussão). [V, fórum]
  - Problema conhecido: Save As quando duas peças com o mesmo identificador estão no mesmo produto. [V, fórum]
  - Exportar para formato neutro (STEP, IGES) também é Save As. [V, EDS]
  - Tools ▸ Downward Compatibility ▸ Save grava para uma versão anterior do CAD avançado B. [V, EDS]

- **CAD avançado A — no detalhe:** [V, resumo de busca de AFX_HIDD_FILESAVE]
  - **Save as copy and continue:** grava a cópia com novo nome ou caminho sem trocar o documento ativo.
  - **Save as copy and open:** a cópia vira o documento ativo e o original continua aberto.
  - **Include all referenced components** (montagem): copia todos os componentes referenciados para o novo local, com prefixo ou sufixo.
  - **Advanced** abre o Save As with References (tabela editável, verde em edição, Undo change). Também há "Save all as copy".
  - Salvar com nome novo em Pack and Go é outro caminho (ficha 6).

- **CAD avançado C — no detalhe:**
  - Save As muda o nome e o caminho e atualiza os links dos arquivos da sessão. Save Copy As não troca o documento aberto. [C]
  - A API `FileSaveAs` (Apprentice) atualiza os links do arquivo salvo, mas só nos arquivos que também foram incluídos no mesmo salvamento. [V, blog fornecedor de CAD avançado]
  - Design Assistant e Vault Copy Design são os caminhos "de verdade" para copiar um conjunto (fichas 8 e 19).

- **Outros:**
  - Fusion: o Save As gera uma cópia que ainda aponta, nas referências externas, para os projetos originais, e os contextos de montagem são quebrados. Ganhou depois a opção **Include Drawings**. [V, ajuda fornecedor de CAD avançado e blog]
  - CAD avançado E: copiar documento ou criar **branch** dentro do mesmo documento. O blog da CAD avançado E sugere branch quando o novo PN substitui o antigo, e documento novo quando os dois vivem por muito tempo com donos próprios. [V]

- **Fenkra hoje:**
  - Não existe Salvar como em nenhuma bancada. Peça avulsa sem arquivo abre diálogo, depois grava no mesmo caminho para sempre (`app.rs:811-828`). Montagem idem (`asm_ui.rs:922-925`). Desenho tem `ask: bool` (`dwg_ui.rs:303-311`) e reescreve os caminhos das fontes quando a pasta muda (`dwg_ui.rs:312-336`). É o único trecho que faz isso.
  - `Project::duplicate` copia como novo item (novo ID e PN, sem histórico) (`lib.rs:259-264`). Para peça, pelo menos.
  - Bug provável: se a montagem só pudesse ser regravada em outra pasta, os caminhos relativos antigos ficariam apontando para o lugar errado. O trecho de reescrita da montagem (`asm_ui.rs:929-937`) só converte absolutos em relativos, nunca relativos antigos em novos.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** Salvar como oferece quatro modos num seletor.
    1. *Renomear/mover (mesmo documento):* mantém o `uid`. Atualiza os links de quem referencia, se o índice de onde-usado achar dependentes.
    2. *Cópia e continuar:* não troca o documento ativo.
    3. *Cópia e abrir.*
    4. *Novo documento:* novo `uid` e novo PN pela regra.
  - **Parâmetros:** nome, pasta, e "incluir referenciados" (marcado em montagem), com prefixo/sufixo e tabela editável com desfazer.
  - **Avançado:** copiar desenhos associados (Include Drawings), reapontar o desenho da cópia para a peça nova, e destino fora do projeto.
  - **Regras:**
    - A cópia nunca herda revisão nem estado Liberado, e começa Em trabalho sem revisões (como `duplicate`).
    - Dois arquivos no destino com o mesmo nome bloqueiam, e a tabela marca o conflito.
    - Referências para fora da árvore copiada ficam apontando para os originais, e a tabela avisa (comportamento do CAD avançado D, mas visível).
  - **Casos de teste:**
    - Salvar como numa montagem de 3 níveis com prefixo `B_`: todas as referências da cópia apontam para arquivos `B_*`.
    - Cópia e continuar não altera o título nem o `file` do documento ativo.
    - Salvar como em pasta irmã preserva os links relativos.
    - Salvar como em cima de arquivo existente pede confirmação.

---

### Propagar diretório e Enviar para (CAD avançado B: Propagate Directory / Send To / CAD avançado A: Pack and Go (parcial) / CAD avançado C: Pack and Go (parcial) / Fenkra: ausente)

- **O que faz:** leva um produto inteiro, com todos os componentes, para outra pasta mantendo os links consistentes.

- **CAD avançado B — no detalhe:**
  - **Propagate Directory** (dentro do Save Management): selecionar o produto de topo, Save As para a nova pasta, depois Propagate Directory. Todos os CATPart e CATProduct passam a mostrar "Save" na coluna Ação. OK executa a cópia. Gravar de volta no local original pede confirmação de sobrescrita arquivo por arquivo. [V, 3DCS]
  - Propaga o diretório de **um** arquivo para todos os outros da árvore, em vez de repetir o Save As em cada um. [V, resumo de busca]
  - Aviso: manter o diretório antigo como backup. Peças ligadas por link podem não propagar sozinhas. [V, 3DCS]
  - Bug conhecido (APAR HD35827): com cache mode ligado, Propagate Directory aparecia cinza na R13SP08 e não reproduzia na R15. [V]
  - Um guia de interoperabilidade PLM avançado/CAD avançado B recomenda **não usar Propagate Directory** em salvamentos de rotina, só quando precisar copiar. [V, resumo de busca]
  - **Send To ▸ Directory:** grava o conjunto numa pasta nova a partir do estado "sem arquivos carregados", gerando nomes e UUIDs novos conforme a documentação lida pelos usuários. Usuários o tratam como equivalente em efeito ao Propagate Directory para clonar (Eng-Tips). [V, fórum]
  - **Diferença prática:** Send To é pensado para transferir dados, e o Propagate Directory serve para mudar o local de trabalho do mesmo conjunto. [V, fórum, não confirmado oficialmente]

- **CAD avançado A — no detalhe:** não há comando com esse nome. O equivalente é **Pack and Go** (ficha 6), mais Save As com Include all referenced components. [V]

- **CAD avançado C — no detalhe:** equivalente é **Pack and Go** (ficha 6) e o **Copy** do Design Assistant com Change Location. [V]

- **Fenkra hoje:** ausente. `save_doc` (`asm_ui.rs:2847-2857`) grava uma peça avulsa gerada (engrenagem) em pasta escolhida, mas sem tocar em referências.

- **Melhor prática para o Fenkra:**
  - Fundir as duas coisas em **Copiar/Mover conjunto** (Pack and Go + Salvar como com referências). O botão se chama **Enviar para…** com destinos Pasta, Zip e Projeto.
  - **Regras:**
    - Nunca mexer na origem.
    - Gerar um `manifesto.json` no destino com uid, hash e caminho antigo e novo de cada arquivo.
    - Pedir confirmação antes de qualquer sobrescrita, uma única vez com a lista.
  - **Casos de teste:**
    - Enviar para pasta vazia e abrir a cópia: a montagem resolve todos os componentes.
    - Reenviar para a mesma pasta lista as diferenças em vez de sobrescrever em silêncio.
    - Árvore com duas peças de mesmo nome em pastas diferentes: o destino plano resolve o conflito e reaponta (vide Pack and Go).

---

### Mesa e gerenciador de relações (CAD avançado B: Desk / Links / CAD avançado A: Find References, File ▸ Open ▸ References / CAD avançado C: Design Assistant / Fusion: Data Panel / Fenkra: ausente)

- **O que faz:** uma janela que mostra a árvore de arquivos de um documento e os links entre eles, independentemente da árvore de modelagem.

- **CAD avançado B — no detalhe:** File ▸ Desk abre uma janela com a estrutura da montagem e as relações entre peças e produtos (resumo de busca da página 3DCS). É de onde se vê qual arquivo aponta para qual. Funciona sem abrir a geometria. [V, resumo de busca]

- **CAD avançado A — no detalhe:**
  - File ▸ Find References, numa montagem, lista de onde cada componente foi carregado. No diálogo Open, o botão References mostra os caminhos dos arquivos referenciados. [V, resumo de busca e Visiativ]
  - O PDM tem a aba **Contains / Where Used** (ficha 16) e o assistente **Update References**. [V]

- **CAD avançado C — no detalhe:**
  - O **Design Assistant** é um aplicativo à parte (clique direito no arquivo no Windows Explorer ▸ Design Assistant). Ele mostra os arquivos relacionados e permite renomear, copiar, substituir e reapontar. [V, ajuda fornecedor de CAD avançado]
  - Usa o projeto ativo do CAD avançado C para saber onde procurar. [V]
  - Não pode alterar relações quando o projeto é semi-isolado, o arquivo está Liberado, não está em check-out ou o atributo é só leitura. [V]
  - A fornecedor de CAD avançado recomenda o Vault para gerenciar arquivos. [V, resumo de busca]

- **Outros:** o Data Panel do Fusion mostra "onde é usado", "o que usa" e os desenhos relacionados. [V, ajuda fornecedor de CAD avançado]

- **Fenkra hoje:**
  - `sub_listing` (`asm_ui.rs:993-1008`) lê os subconjuntos recursivamente só para exibir a árvore. A BOM faz `load_sub` (`bom.rs:58`).
  - A janela de links (`link_window`, `app.rs:2296`) mostra publicações, mas não há tela única "arquivos e relações".
  - O `PartCache` (`load.rs:20`) guarda peças lidas pela mtime, mas não expõe quem usa quem.

- **Melhor prática para o Fenkra:**
  - **Painel Relações** com duas abas: **Contém** (árvore para baixo) e **Onde é usado** (para cima). Colunas: nome, PN, Rev, estado, caminho resolvido, estado do link (`->`, `->?`, `->*`, `->x` do ADR 0011).
  - Ações por linha: Abrir, Mostrar no explorador, **Reapontar**, **Substituir**, Travar/Quebrar link.
  - **Regras:** o painel funciona sem carregar geometria (só lê o cabeçalho de referências do arquivo). Ciclos são detectados na leitura e mostrados como erro, não como recursão até profundidade 16 (`bom.rs:82`, `bom.rs:177`).
  - **Casos de teste:**
    - Montagem com componente movido mostra o link `->x` e oferece Reapontar.
    - Dois níveis com ciclo A→B→A mostram o aviso sem travar.

---

### Empacotar (CAD avançado B: Send To / CAD avançado A: Pack and Go / CAD avançado C: Pack and Go / Fenkra: ausente, previsto no B56)

- **O que faz:** reúne uma peça, montagem ou desenho com tudo que ele referencia em uma pasta ou zip que abre em outro computador sem links quebrados.

- **CAD avançado B — no detalhe:** não há comando com esse nome. O vídeo do canal "CAD avançado B Shorts #020" apresenta o Propagate Directory como "meio que um Pack and Go". [V, título de resultado de busca] Ver ficha 4.

- **CAD avançado A — no detalhe:** [V, resumo de busca, cadimensions e Hawk Ridge]
  - Acesso por File ▸ Pack and Go, ou botão direito no arquivo no Windows Explorer ▸ CAD avançado A (esta via tem menos opções).
  - **Include drawings:** traz desenhos da pasta do modelo ou dos Search Paths definidos em File Locations.
  - **Include simulation results:** traz relatórios do Simulation, mesmo fora da pasta do modelo. Aumenta muito o tamanho.
  - **Include biblioteca de normas components:** por padrão o hardware do biblioteca de normas **não** vai.
  - **Add prefix / Add suffix:** vale para todos os arquivos. Muito usado para marcar revisão.
  - **Select / Replace:** renomeia em lote (coluna "New Filename"). Desmarcar arquivos aqui pode quebrar referências.
  - Duplo clique em "New Filename" renomeia um a um sem quebrar nada.
  - **Layout:** Flatten to single folder (o mais comum), Keep full folder structure (espelha o explorador) e opções mínimas que mantêm pastas isoladas, como a de um subconjunto.
  - **Saída:** Save to folder (caminho e nome da pasta) ou Save to Zip (caminho e nome). E-mail depois do pacote só com zip.
  - A cadimensions recomenda **sempre salvar em zip**: salvar em pasta costuma deixar referências sem atualizar e arquivos duplicados, e o zip congela o conjunto num ponto. [V]
  - O que fica de fora se não marcado: biblioteca de normas, resultados de simulação, desenhos, arquivos desmarcados, e a gravação para versão anterior só vale até duas versões atrás (2024 para 2022). [V]

- **CAD avançado C — no detalhe:** [V, ajuda fornecedor de CAD avançado, página lida]
  - **Source File**, **Destination Folder** (cria se não existir, com pergunta), **Project File** (padrão o projeto ativo).
  - **Find referenced files:** busca nos locais do projeto (workspaces, workgroups e bibliotecas) ou numa pasta específica com ou sem subpastas. Acha também os arquivos que **referenciam** o selecionado (desenhos que apontam para a montagem), sem duplicar.
  - **Copy to Single Path:** tudo numa pasta só. **Keep Folder Hierarchy:** recria a hierarquia.
  - **Collect workgroups:** junta workgroups e workspace numa raiz. Desmarcado, cada local vira sua subpasta (Workspace, Locals, Workgroups, Libraries).
  - **Model Files Only** (.iam, .ipt, .idw, .dwg, .ipn) ou **Include Linked Files** (planilhas, texto etc.).
  - **Skip Libraries** (o novo projeto aponta para as bibliotecas originais e caminhos relativos viram absolutos), **Skip Styles**, **Skip Templates**.
  - **Nomes repetidos:** só um arquivo pode ficar no destino. As referências aos demais são redirecionadas para o que foi copiado. É um risco silencioso. [V]
  - Mostra Total Files e espaço em disco necessário, e cria um **log** na pasta com nomes e caminhos de origem de todos os referenciados. O log é sobrescrito a cada empacotamento no mesmo destino. [V]
  - Cria um arquivo de projeto novo no destino. [V, parcialmente: a página não descreve opção dedicada]

- **Outros:** Fusion usa o Archive/"Export" e o upload de arquivos; para copiar sem dependências, um funcionário da fornecedor de CAD avançado sugere exportar o design e subir no novo local. [V, fórum]

- **Fenkra hoje:** ausente. A única ligação é que `PartCache::get` (`load.rs:38-52`) lê peça por caminho e a BOM (`bom.rs:81-97`) já percorre subconjuntos com limite de profundidade 16, o que dá a base para o fecho transitivo.

- **Melhor prática para o Fenkra:**
  - **Fluxo:**
    1. *Suporte:* escolher o documento raiz (peça, montagem ou desenho).
    2. *Modo:* Pasta ou Zip, com Zip como padrão (por causa do conselho acima).
    3. *Parâmetros:* layout Plano ou Hierárquico, prefixo/sufixo, incluir desenhos associados, incluir arquivos linkados (planilhas, imagens, texturas, malhas), incluir derivados e suas origens.
    4. *Avançado:* tabela de renomeação com desfazer e regras de substituição em lote; pular bibliotecas (a biblioteca de normas/fixadores).
  - **Regras (o que evita os bugs documentados nos outros CADs):**
    - Colisão de nome no layout plano **nunca** redireciona em silêncio. A ferramenta renomeia com sufixo automático e reaponta, listando tudo na tabela.
    - Desmarcar um arquivo mostra os pais que passarão a ter link quebrado.
    - Sempre produzir `manifesto.json` + `log.txt` (origem, destino, hash, estado, revisão).
    - A verificação final **reabre o pacote em memória** e confirma que todos os links resolvem. Se não, o pacote não é marcado como íntegro.
  - **Casos de teste:**
    - Pacote plano de montagem com duas peças `base.fkp` em pastas diferentes: ambas presentes e links certos.
    - Abrir o zip descompactado em outra máquina (outro caminho/drive) tem 0 links quebrados.
    - Desenho associado só vem com a opção ligada.
    - Origem não é alterada (hash antes = depois).

---

### Referências entre arquivos: caminhos e busca (CAD avançado B: Linked Document Localization / Search Order / CAD avançado A: Referenced Documents / CAD avançado C: Project / Fenkra: caminhos de texto)

- **O que faz:** define como um arquivo guarda onde está seu componente e como o programa o encontra quando abre.

- **CAD avançado B — no detalhe:**
  - Na documentação antiga, o CAD avançado B compara a opção **Search Order** com **Linked Document Localization**. Na segunda, a pasta do link é pesquisada primeiro. Na primeira, outras pastas vêm antes. O Search Order é a lista de diretórios onde os documentos ligados ficam, e aceita multisseleção. [V, resumo de busca de CAD avançado Bdoc.free.fr]
  - A variável `CATReffilesPath` define o caminho de busca dos arquivos de referência. [V, resumo de busca]
  - Os documentos têm identificador interno (UUID). Save As mantém e Send To troca (ficha 3). [V, fórum]
  - Quando o link não resolve, o CAD avançado B abre o resto e avisa. [C]

- **CAD avançado A — no detalhe:** ordem de busca [V, Visiativ, página lida]
  1. Memória: arquivo com o mesmo nome já aberto, **mesmo que seja outra versão**.
  2. Pastas de **Referenced Documents** (File Locations). Só valem se a opção "Search file locations for external references" estiver marcada. [V, resumo de busca]
  3. Pasta de onde a montagem foi aberta.
  4. Último caminho usado na sessão (pulado em sessão nova).
  5. Caminho salvo, na unidade atual.
  6. Caminho salvo, na unidade original.
  7. Browse: pede para o usuário apontar. É assim que se atualiza após renomear fora do CAD avançado A.
  - Erro típico: "Referenced document has non-matching internal ID". A causa mais comum é nome de arquivo duplicado. [V]
  - Correções recomendadas: nomes únicos, pasta do projeto em Referenced Documents, manter tudo junto ou usar Pack and Go, e usar Find References para ver de onde cada um veio. [V]
  - Fontes divergem um pouco na ordem exata (uma coloca a busca recursiva depois de Referenced Documents). [V, resumo de busca]

- **CAD avançado C — no detalhe:**
  - Cada arquivo guarda os nomes dos arquivos que referencia. [V]
  - O **projeto (.ipj)** define onde procurar. [V, ajuda fornecedor de CAD avançado, página lida]
    - **Workspace:** área pessoal de edição, normalmente na raiz do projeto, onde fica o .ipj.
    - **Workgroup:** local de rede mestre dos arquivos compartilhados (um por projeto; mais caminhos deixam a busca mais lenta).
    - **Bibliotecas:** somente leitura.
    - Tipos de projeto: single user, Vault, semi-isolated, shared.
  - Ordem de busca (2020): bibliotecas (só quando uma peça de biblioteca é carregada), workspace, caminhos do workgroup, pasta do arquivo de projeto. A de 2022 acrescenta o "work path" no começo e a pasta do arquivo no fim. [V, resumo de busca]
  - **Evitar caminhos aninhados** e definir um só workgroup/workspace. Caminhos aninhados aparecem em vermelho no editor porque costumam causar problemas de resolução. [V]
  - Se o arquivo não é achado, abre o diálogo **Resolve Link**. O evento de API `OnFileResolution` dispara para todo arquivo que o CAD avançado C precisa. [V]

- **Outros:**
  - Fusion: referências por projeto e versão na nuvem, com "Get Latest". [V, resumo de busca]
  - CAD avançado E: ao atualizar referências entre documentos cria uma **versão automática**, que aparece no gráfico. [V]

- **Fenkra hoje:**
  - `NodeKind::Part { path }` guarda texto (`asm_ui.rs:435`). `full_path` (`load.rs:27-33`) junta com a pasta da montagem se o caminho é relativo.
  - `rel_path` (`asm_ui.rs:317-324`) usa `strip_prefix`, então só vira relativo quem está dentro da pasta da montagem. Peça em pasta irmã fica **absoluta**, e a montagem deixa de ser portátil.
  - `ExternalRef.path` é "relativo à pasta do projeto, ou absoluto" (`fenkra-core/doc.rs:457`). `load_link_source` (`app.rs:2267-2275`) faz o join só se há projeto aberto. **Sem projeto, um caminho relativo vira relativo ao diretório de trabalho do processo**, o que quebra depende de como o Fenkra foi iniciado.
  - `MeshImport` guarda `p.to_string_lossy()` absoluto (`app.rs:1756`).
  - O `assign_items` usa o caminho relativo como chave do número de item (`bom.rs:53-56`, `108`). Renomear o arquivo faz o item perder o número.
  - Não há ordem de busca nem diálogo "Resolve Link". A falha vira mensagem "Links: N atualizado(s), M quebrado(s)" (`app.rs:2292`).

- **Melhor prática para o Fenkra:**
  - **Modelo de referência** em cada arquivo, com três campos: `uid` (identidade estável), `rel` (último caminho relativo à pasta do dono) e, opcionalmente, `hash`/`rev` (alvo: revisão de trabalho ou revisão congelada, como o ADR 0006 prevê).
  - **Ordem de resolução** (inspirada no CAD avançado A e no CAD avançado C, com a diferença de que o `uid` vence o nome):
    1. documento já aberto com o mesmo `uid`;
    2. `rel` relativo à pasta do dono;
    3. índice `uid → caminho` do projeto (varredura em cache);
    4. pastas extras do projeto (bibliotecas);
    5. caminho absoluto salvo (último recurso);
    6. diálogo **Resolver**, que mostra candidatos por `uid` ou nome.
  - **Regras:**
    - Sempre gravar caminho relativo com `..` quando o alvo estiver no mesmo drive/raiz. Absoluto só se o alvo está em outro drive.
    - Nunca resolver em silêncio para um arquivo de `uid` diferente (evita o "non-matching internal ID" do CAD avançado A). Avisar e pedir confirmação.
    - Nomes de arquivo duplicados no mesmo projeto geram aviso de projeto.
    - Número de item de BOM guardado por `uid`, não por caminho.
  - **Casos de teste:**
    - Mover a pasta inteira do projeto para outro drive: 0 links quebrados.
    - Mover só uma peça: a montagem a acha pelo `uid` no índice e corrige o `rel` no próximo salvamento.
    - Dois arquivos com mesmo nome em pastas diferentes: a montagem abre os certos.
    - `ExternalRef` relativo sem projeto aberto resolve contra a pasta do documento, nunca contra o cwd.

---

### Renomear e mover sem quebrar (CAD avançado B: Save As + Propagate / CAD avançado A: PDM Move Tree, rename pelo FeatureManager, Update References / CAD avançado C: Design Assistant / Fenkra: ausente)

- **O que faz:** muda nome ou pasta de um arquivo e atualiza todos os arquivos que o referenciam.

- **CAD avançado B — no detalhe:** sem comando próprio. Renomeia-se com Save As (mantém identificador) dentro do Save Management, e os pais recebem Save Auto. Fora do CAD avançado B, o link quebra. [V]

- **CAD avançado A — no detalhe:**
  - Fora do PDM, renomear no Explorer quebra o link, e se resolve com Browse na abertura. [V]
  - Dentro do PDM, **renomear** um componente pela árvore do FeatureManager atualiza as referências automaticamente, com o arquivo em check-out e uma opção de sistema ligada. [V, resumo de busca]
  - **Move Tree:** renomeia e move vários arquivos de uma vez, e as referências são atualizadas automaticamente se os dependentes entram na operação. [V, resumo de busca]
  - **Update References (assistente):** é preciso fazer check-out do pai e dos subconjuntos antes de editar as referências. Pode reparar links quebrados **sem abrir o pai no CAD avançado A**. [V, resumo de busca]
  - Se um arquivo em cache aberto só leitura tem as referências movidas ou renomeadas por outro usuário, o add-in do PDM sinaliza os afetados, avisa e repontua para o local mais recente. [V, resumo de busca]

- **CAD avançado C — no detalhe:** [V, ajuda fornecedor de CAD avançado]
  - Design Assistant, quatro operações: **Renomear** (atualiza os links em todos os arquivos que referenciam, por exemplo trocar nome provisório por PN), **Revisar** (copia o liberado para nova revisão, relinka o pai à cópia e deixa o original), **Substituir** (atualiza todas as ocorrências na montagem; mesmo tipo de arquivo; desenhos e apresentações usam "update additional referencing files") e **Criar configurações** (copiar uma montagem compartilhando as peças inalteradas).
  - As mudanças só valem em Save.
  - Renomear fora do Design Assistant quebra o link da montagem (Arkance, resumo de busca). Problema reportado: peça usada por duas montagens dá "File not found" ao abrir a segunda. [V, fórum]
  - Não se deve usar Design Assistant em arquivos em check-out de projeto semi-isolado ou compartilhado. [V, resumo de busca]

- **Outros:** CAD avançado E/Fusion não têm o problema (nuvem, referência por ID). O CAD avançado E guarda versão automática ao atualizar. [V]

- **Fenkra hoje:** ausente. Nem a UI de projeto (`project_panel`, `app.rs:5147-5284`) tem Renomear. `DocEntry.name` e `pn` são alterados só pelo salvamento (`lib.rs:216-218`).

- **Melhor prática para o Fenkra:**
  - **Fluxo:** *Renomear/Mover…* pelo painel de Relações ou pelo projeto.
    1. Escolhe novo nome e pasta.
    2. O Fenkra consulta o **onde-usado** e mostra a lista de dependentes que serão reescritos (pré-visualização).
    3. O usuário confirma.
    4. A operação roda como **transação**: escreve tudo em temporários, troca todos juntos e guarda um desfazer (`.fenkra/historico`).
  - **Regras:**
    - Peça Liberada: renomear arquivo é permitido (o `uid` e o PN não mudam). Renomear **PN** gera aviso e exige revisão (ficha 18).
    - Dependente em check-out por outra pessoa bloqueia a operação inteira (nada de mudança parcial).
    - Fora do Fenkra: o assistente **Reparar links** varre a pasta, casa por `uid` e propõe os reapontamentos sem abrir a geometria (copiando o Update References).
  - **Casos de teste:**
    - Renomear peça usada em 3 montagens e 1 desenho: os 4 abrem sem erro.
    - Falha de disco no meio da transação: nenhum arquivo fica pela metade.
    - Desfazer restaura nomes e links.
    - Mover o arquivo no Explorer e abrir a montagem: o assistente propõe o novo caminho.

---

### Substituir referência (CAD avançado B: Replace Component / CAD avançado A: Replace Components / CAD avançado C: Replace / Design Assistant Replace / Fenkra: AsmReplace parcial)

- **O que faz:** troca um componente por outro arquivo mantendo posição e, se possível, restrições.

- **CAD avançado B — no detalhe:** Replace Component na montagem. Restrições que apontavam para a geometria antiga podem ficar quebradas e entram no diagnóstico de update. [C] No PLM avançado há "Replace by Revision" e "Replace by Version", com mudanças de interface na R2019x. [V, resumo de busca]

- **CAD avançado A — no detalhe:** Replace Components troca em lote e pergunta como reaplicar mates, com opção de mapear por nome de face. Com o PDM também existe "Update References". [C]

- **CAD avançado C — no detalhe:** Replace (uma ocorrência) ou Replace All no assembly. Só entre arquivos do mesmo tipo. Os desenhos não são substituídos diretamente. [V] A API `ReplaceReference` exige que o arquivo novo tenha a mesma ancestralidade (pode ser uma versão modificada). [V, blog]

- **Fenkra hoje:**
  - `Cmd::AsmReplace` (`asm_ui.rs:479-491`) troca o componente selecionado por um arquivo escolhido e chama `asm.replace` (`fenkra-montagem/lib.rs:651-655`), que só troca o `kind`. O aviso diz que "as restrições reencontram as faces pelo ponto clicado" (`refresh_refs`, `load.rs:77-100`).
  - Substitui **um** componente por vez, sem "substituir todas as instâncias", sem mapa de faces e sem relatório do que ficou sem restrição.
  - A BOM guarda número de item por caminho. A peça substituta ganha número novo.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** *Suporte:* selecionar componente(s) ou "todas as ocorrências do arquivo". *Modo:* Substituir por outro arquivo, por outra revisão, por outra versão. *Parâmetros:* reaplicar restrições por **nome publicado** (ADR 0010) e, na falta, por ponto clicado. *Avançado:* manter número de item, manter nome da instância.
  - **Regras:** relatório obrigatório "restrições reaplicadas / sem alvo", e restrição sem alvo fica em estado "falhou" visível, nunca em silêncio.
  - **Casos de teste:** substituir peça por revisão B mantém o item e as restrições; substituir por peça sem as faces deixa as restrições listadas como pendentes e o GDL é recalculado.

---

### Recuperação e backup automático (CAD avançado B: Automatic/Incremental Backup / CAD avançado A: AutoRecover / CAD avançado C: Autosave / Fenkra: ausente)

- **O que faz:** salva cópias de segurança fora do arquivo de trabalho para recuperar depois de queda.

- **CAD avançado B — no detalhe:**
  - Tools ▸ Options ▸ General ▸ Data Save: sem backup, backup automático (padrão 30 min, intervalo configurável) ou incremental. O incremental guarda os documentos abertos em diretório temporário e permite recuperar até a última interação. Não está em todas as bancadas. [V, EDS]
  - Há também gravação para versão anterior (Downward Compatibility). [V]

- **CAD avançado A e CAD avançado C — no detalhe:** ambos têm intervalo configurável, pasta de recuperação e diálogo de recuperação na próxima abertura, que lista os documentos recuperáveis. [C]

- **Fenkra hoje:** ausente. O único "salvamento automático" é antes de trocar de documento de projeto (`app.rs:1153-1155`, RNF-007). Preferências são gravadas ao fechar a janela de configurações (`app.rs:2262`).

- **Melhor prática para o Fenkra:**
  - **Fluxo:** a cada N minutos (padrão 5, configurável) e depois de operações longas, gravar `.fenkra/recuperacao/<uid>.<ts>.fkp` para cada documento modificado, sem tocar no arquivo do usuário. Ao abrir, se existir recuperação mais nova que o arquivo, oferecer **Recuperar / Descartar / Comparar**.
  - **Regras:** manter só as 3 últimas por documento. Apagar ao salvar com sucesso. Nunca recuperar sem perguntar.
  - **Casos de teste:** matar o processo com a peça modificada, reabrir: o diálogo oferece a recuperação. Salvar limpa as cópias. O disco cheio não derruba o programa (erro tratado).

---

### Check-in/check-out e travamento de arquivo (CAD avançado B/PLM avançado: Lock / CAD avançado A PDM: Check Out, Check In / CAD avançado C-Vault: Check Out, Check In / Fenkra: ausente)

- **O que faz:** garante que uma só pessoa edita um documento por vez e registra cada entrega como nova versão.

- **CAD avançado B/PLM avançado V5 — no detalhe:**
  - No VPM, o **lock** do item é pré-requisito para revisar e para salvar. Casos reportados em APAR: salvar do CAD avançado B recusado com "Operation not authorized" até travar a referência no PLM avançado, e anexar documento exige travar a instância. [V, resumo de busca]
  - New Reference e New Revision são comandos do PLM avançado, não só do CAD avançado B. [V, resumo de busca]
  - Na plataforma PLM avançada, o lock é opcional e por permissão no estado Em trabalho, e bloqueia as alterações de outros. Congelado e Liberado impedem modificação. [V]

- **CAD avançado A PDM — no detalhe:**
  - **Check Out** dá o direito exclusivo de editar. Outros podem abrir para ver e copiar, mas não alterar. **Undo Check Out** cancela sem salvar as mudanças. [V]
  - Os comentários de check-in e de mudança de estado podem ser obrigatórios por estado ou transição. [V]
  - A versão sobe a cada check-in com mudança. [V]
  - Arquivos fora de check-out ficam somente leitura no cache local (a "local view"). [C]
  - Renomear no FeatureManager exige o arquivo em check-out (ficha 8). [V]

- **CAD avançado C/Vault — no detalhe:**
  - Check-in grava as alterações como **nova versão**. Undo Check Out descarta e libera o bloqueio. [V, resumo de busca]
  - Ao abrir ou posicionar um arquivo, o Vault primeiro traz a **última versão do arquivo e dos filhos** e depois faz o check-out do que precisa. [V]
  - Com **Model States**, o Vault baixa só os arquivos que aquele estado usa. Componentes suprimidos não são baixados nem entram em check-out ou check-in. [V]
  - Arquivo Liberado não pode ter relações mudadas pelo Design Assistant. [V]

- **Outros:** CAD avançado G/PLM avançado e CAD avançado F/PLM avançado usam check-out em workspace local e check-in. [C] CAD avançado H tem o Revision Manager e Insight. [C] CAD avançado D/CAD avançado E: edição simultânea ou branch, sem trava. [V, parcialmente]

- **Fenkra hoje:** ausente (catálogo PLM-04 "não"). A propriedade `Liberado` não trava edição. `Project::save` apenas volta o estado para Em trabalho quando o conteúdo muda (`lib.rs:221-223`). Revisão congelada só protege a *versão aberta como congelada* (`app.rs:796-798`). O `plm-pn-engine.md` já propõe `lock.json`.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** *Check-out* (trava + cópia de trabalho gravável) → editar → *Check-in* (comentário, nova versão, libera trava) | *Desfazer check-out*.
  - **Regras:**
    - Trava = arquivo `.fenkra/locks/<uid>.json` com `{usuario, maquina, pid, desde, batimento}`. Em disco local, somar trava do SO (`LockFileEx`) [C].
    - Em pasta de rede/OneDrive, o trava-arquivo tem batimento e expiração (TTL), com "assumir trava" depois de TTL e confirmação.
    - Não-donos abrem **somente leitura** com banner "em check-out por X desde Y".
    - Check-in compara hash da base: se o disco mudou desde o check-out, bloqueia com "conflito".
  - **Casos de teste:**
    - Dois processos, mesmo documento: o segundo abre só leitura.
    - Processo do dono morto: a trava vence após o TTL e é assumida com aviso.
    - Desfazer check-out restaura o conteúdo anterior.
    - Check-in sem mudança não cria versão.

---

### Pasta compartilhada e multiusuário (CAD avançado B: servidor PLM avançado / CAD avançado A: PDM server / Vault: servidor / Fenkra: modo pasta proposto)

- **O que faz:** permite trabalho em equipe sem servidor, em pasta de rede ou OneDrive, que é o cenário do `plm-pn-engine.md`.

- **O que os três fazem:** exigem **servidor** (catálogo PLM-07: "os três exigem servidor"). O CAD avançado C tem projetos *semi-isolated* e *shared* com workgroup em rede, mas a fornecedor de CAD avançado recomenda o Vault. [V]

- **Riscos documentados do ambiente:**
  - **SQLite em pasta de rede:** a documentação do SQLite desaconselha, porque a rede pode perder, atrasar ou reordenar escritas e porque alguns sistemas de arquivos de rede implementam travas de forma incorreta. Dois clientes podem achar que têm a trava e corromper o banco. Recomenda cliente/servidor (PostgreSQL) para vários computadores. [V]
  - **OneDrive:** ao detectar conflito (arquivo alterado em dois lugares), cria um segundo arquivo com o **nome do computador** no nome: `Arquivo-NOMEDOPC.ext`. [V, fonte secundária]
  - **Arquivos de trava do Office** (`~$arquivo`): conhecimento geral, não confirmado aqui. [C]

- **Fenkra hoje:**
  - `Project::open` lê o índice uma vez (`lib.rs:149-152`). `save_index` regrava o **índice inteiro** (`lib.rs:154-159`) a partir da memória do processo.
  - Duas instâncias na mesma pasta: o último a gravar apaga as mudanças do outro (atualização perdida).
  - Colisão de identidade: `next_id` e `next_pn` são contadores no índice (`lib.rs:103,169-173,181-182`). Duas pessoas criando peça ao mesmo tempo recebem o mesmo `id`, portanto o mesmo arquivo `pecas/00007.fkp` (`lib.rs:185`), e uma sobrescreve a outra sem aviso.
  - `plm-vpm.md` propõe SQLite local e depois PostgreSQL. A estratégia de fase 1 é sensata para um usuário; para **modo pasta** o índice em SQLite não é seguro.

- **Melhor prática para o Fenkra:**
  - **Modo pasta sem banco compartilhado.** Propostas:
    - Um arquivo por registro, em vez de um índice monolítico: `itens/<uid>.json` (um escritor por vez, protegido pela trava do item).
    - O índice agregado vira **cache local derivado** (pode ser SQLite, mas fica no disco do usuário e é reconstruído por varredura).
    - Contadores (PN, ID) por **reserva**: arquivo `contadores/<amb>.json` com trava curta e retentativa, ou faixas por usuário/máquina. O PN só é "consumido" quando o documento é efetivamente gravado (como o Vault, ficha 18).
    - Diário de mudanças `.fenkra/diario/<usuario>.jsonl` (um arquivo por usuário, só acréscimo): evita escrita concorrente no mesmo arquivo e dá o histórico/auditoria (ficha 22).
  - **Detecção de conflito:** antes de qualquer gravação, conferir `mtime+hash` base. Arquivos `-NOMEDOPC` ou `(conflito)` na pasta são detectados e listados no painel como **Conflitos** com Comparar e Escolher.
  - **Regras:** nunca gravar na pasta compartilhada fora de `tmp + rename`. O programa avisa se a pasta está em "somente online" (placeholder do OneDrive) e pede para baixar antes. [C]
  - **Casos de teste:**
    - Duas instâncias criam peça ao mesmo tempo: ids distintos, nenhum arquivo perdido.
    - Dois processos gravam o contador: 100 PNs sem repetição.
    - Simular arquivo de conflito do OneDrive: aparece no painel.
    - Queda da rede no meio do check-in: nada fica parcial.

---

### Versão × revisão (CAD avançado B/PLM avançado: Version, Revision / CAD avançado A PDM: Version, Revision / CAD avançado C-Vault: Version, Revision / CAD avançado E: Version, Revision / Fenkra: Version, Revision)

- **O que faz:** separa o histórico contínuo (versões, automáticas) dos marcos aprovados (revisões, deliberadas).

- **CAD avançado B/PLM avançado — no detalhe:** a plataforma PLM avançada cria revisões a partir do estado Liberado: **New Revision**, **New Revision From** (estrutura baseada em outra fonte) e **New Branch**. Revisões diferentes podem estar em estados de maturidade diferentes ao mesmo tempo. [V] Há um modo de "Engineering Revision" desacoplada do "Model Version" em estruturas configuradas. [V, resumo de busca] O formato pode ser Primary ou Primary.Secondary, o primário pula I, O, Q, S, X, Z (guia da TriMech), e ao salvar do CAD avançado A vem A.1, depois A.2. [V]

- **CAD avançado A PDM — no detalhe:** [V, GoEngineer]
  - **Versões** automáticas: a cada check-in com mudança, número 1, 2, 3, com comentário do que mudou. Também surgem versões sem check-out, por ação de workflow em segundo plano.
  - **Revisões** são deliberadas: marcam uma versão que passou pela aprovação. O administrador define o esquema (letras, números, A.1, nomes como "Protótipo").
  - **Esquemas de numeração de revisão** = texto fixo + um ou mais **contadores** que sobem a cada aplicação. Trocar o esquema faz o próximo recomeçar do valor inicial dos componentes do novo esquema. [V, resumo de busca de help.CAD avançado A]
  - Configurações de transição atualizam a revisão local do PDM, não a *variável* Revisão, então se combina com "Set variable". [V, GoEngineer]
  - Para ver o histórico: menu Ações ▸ Get Version, com a opção "Always work with latest versions" desligada. [V]

- **CAD avançado C/Vault — no detalhe:** check-in cria versão. Mudança de estado de ciclo de vida pode criar revisão. Mudar um Liberado de volta a Em andamento cria nova revisão, mas o check-out é um passo à parte (post de ideias da fornecedor de CAD avançado). [V, resumo de busca]

- **CAD avançado E — no detalhe:** a *versão* é um instantâneo nomeado e imutável. A *revisão* é um estado formalmente liberado de uma peça, montagem ou desenho, ligado a um PN e a um esquema (A, B, C ou 1, 2, 3). Toda revisão aponta para uma versão, mas a maioria das versões nunca vira revisão. [V]

- **Fenkra hoje (acerto do modelo):**
  - `Version { hash, time, note }` e `Revision { label, hash, time, comment }` (`lib.rs:57-69`). Versão é conteúdo endereçado por hash em `.versoes/<hash>.fkp` e só nasce quando o hash muda (`lib.rs:219-224`).
  - `release` marca a última versão salva com a próxima letra e muda o estado para Liberado (`lib.rs:240-251`). Recusa se nada mudou desde a última revisão (`NothingToRelease`). O teste `versoes_e_revisao_congelada` confirma que a revisão A permanece intacta (`lib.rs:297-322`).
  - `next_label` gera A…Z, AA, AB (`lib.rs:117-135`) e não pula as letras proibidas.
  - Lacunas:
    - A versão não guarda autor nem máquina.
    - A nota é sempre "salvo". Ctrl+S pode gerar dezenas de versões sem comentário.
    - O PN é editável mesmo com revisões liberadas (`lib.rs:216-218`), e os JSON congelados guardam o PN antigo. Isso dá incoerência.
    - `.versoes` só cresce, sem compactação nem limpeza (nem o conceito de "versão intermediária descartável").
    - Esquema de revisão fixo (letras).

- **Melhor prática para o Fenkra:**
  - **Fluxo:** versão automática no Salvar do projeto (com campo opcional de comentário), com **agrupamento** de salvamentos seguidos na mesma sessão (colapsar em "sessão de edição" para não poluir). Revisão = ação deliberada (Liberar revisão).
  - **Parâmetros:** esquema de revisão configurável por ambiente: letras (A, B, … com opção de pular I, O, Q, S, X, Z [C: convenção ASME Y14.35]), números, maior.menor (A.1), com contadores.
  - **Regras:** PN imutável depois da primeira revisão (mudar exige "Salvar como novo documento"). Versão grava autor e data. A limpeza só remove versões que não são revisão nem linha de base.
  - **Casos de teste:** liberar A, editar, liberar B, abrir A congelada dá o conteúdo original (já existe); revisão com esquema pulando I e O vai de H para J; PN bloqueado após liberar; 200 salvamentos agrupados aparecem como poucas sessões.

---

### Ciclo de vida e fluxo de aprovação (CAD avançado B/plataforma PLM avançada: Maturity / CAD avançado A PDM: Workflow states / Vault: Lifecycle / Fenkra: DocState)

- **O que faz:** define em que fase o documento está (em trabalho, em revisão, liberado, obsoleto) e quem pode movê-lo.

- **plataforma PLM avançada — no detalhe:** [V, Hawk Ridge, página lida]
  - Estados: **Private** (só o dono), **In Work** (padrão ao salvar na plataforma; visível; pode ser travado), **Frozen** (revisão; mudanças bloqueadas), **Released** (produção; "não pode ser modificado"), **Obsolete** (guardado só para consulta).
  - Transições: Private→In Work, In Work→Frozen, Frozen→In Work (rejeitado ou mais mudanças), Frozen→Released (aceito), Released→Obsolete.
  - Transições podem ser aplicadas a vários arquivos de uma vez, incluindo filhos estruturais. Regras podem exigir que todos os filhos transitem antes do pai, por exemplo liberar os filhos antes da montagem.
  - Quem pode agir depende de permissão. A fonte não detalha papéis.
  - O PLM avançado tem uma discussão de impedir mudança de maturidade se a EIN não estiver preenchida (ligação com engenharia de mudança). [V, só título]

- **CAD avançado A PDM — no detalhe:**
  - **Workflow** com estados e transições. O estado inicial pode ser escolhido. Estados representam "Em edição", "Aguardando aprovação", "Aprovado". [V]
  - Comentário obrigatório por estado e transição. [V]
  - O Standard tem um fluxo com até dez estados (fonte secundária). O Professional permite vários fluxos, aprovações paralelas e notificações. [V, resumo de busca; C]
  - Estado pode **definir permissões** de leitura/escrita/exclusão por grupo. [V, título de página de ajuda]
  - **Check Out durante a transição** é discutido nos fóruns como armadilha. [V, só título]

- **CAD avançado C/Vault — no detalhe:**
  - Definições de ciclo de vida com transições. As **Lifecycle Transition settings** podem bloquear a liberação quando critérios não são atendidos (por exemplo, filhos não liberados). Um Get/Checkout pode falhar se os subcomponentes estão num ciclo de vida que só alguns usuários acessam. [V, resumo de busca ImaginIT]
  - Arquivos Liberados não aceitam mudança de relações no Design Assistant. [V]
  - Items (BOM) acompanham o estado. [C]

- **Fenkra hoje:**
  - `DocState { EmTrabalho, Liberado, Obsoleto }` (`lib.rs:40-44`), sem transições configuráveis, sem aprovador, sem comentário de transição.
  - A UI oferece só "Liberar revisão…" (`app.rs:5250`, diálogo `5863-5887`) e "Tornar obsoleto" (`5258`). Não existe "reabrir" de Obsoleto.
  - `set_state(...)` aceita qualquer estado sem validar (`lib.rs:253-256`), e não há regra de filhos.
  - Estados não se aplicam a montagens nem desenhos (o projeto só tem peças).
  - Editar uma peça Obsoleta é permitido e ela continua Obsoleta.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** o ambiente define o **grafo de estados**, e o padrão é Privado → Em trabalho → Congelado → Liberado → Obsoleto, com transições nomeadas e comentário obrigatório configurável.
  - **Regras:**
    - Congelado e Liberado ficam só leitura no Fenkra, e para editar é preciso uma *nova revisão* (como o CAD avançado C "Revisar").
    - Liberar uma montagem exige filhos Liberados (ou lista explícita de exceções).
    - Obsoleto não pode ser inserido em montagem nova (aviso, não bloqueio).
    - Transição em lote com preview da lista de afetados.
  - **Casos de teste:** montagem com filho Em trabalho não libera; liberar em lote libera filhos antes do pai; Liberado não aceita Salvar sem criar revisão; transição sem comentário (quando exigido) é recusada.

---

### Estrutura de produto: produto, item, instância, BOM (PLM avançado: Product Reference / Instance / CAD avançado A PDM: BOM / Vault: Items / Fenkra: BOM de montagem, sem estrutura no projeto)

- **O que faz:** separa a *definição* (referência de produto, item) das *ocorrências* (instâncias) e liga ambas à lista de materiais.

- **PLM avançado VPM / CAD avançado B — no detalhe:**
  - Conceitos: referência de produto (definição, com PN e revisões), instância (uso dentro de um pai, com posição) e documento (arquivo CATPart/CATProduct). [C, conhecimento geral; os APARs lidos confirmam que "instance" e "reference" existem e que New Reference duplica a referência. [V]]
  - Os contextos de CAD avançado B (links contextuais) precisam ser redefinidos depois de duplicar a instância. [V, resumo de busca]
  - A estrutura de produto é configurada por *effectivity* (ficha 17). [V]

- **CAD avançado A PDM — no detalhe:** BOM calculada a partir das referências, com variáveis do cartão de dados (PN, descrição) como colunas. Aba Contains e Where Used. [C]

- **Vault/CAD avançado C — no detalhe:** **Items** (itens de engenharia) agrupam o arquivo com propriedades de BOM. Model States com o mesmo PN compartilham BOM e item, e com PN diferente têm BOM e item separados. [V]

- **Fenkra hoje:**
  - `bom()` (`bom.rs:139-174`) calcula a lista em três níveis (primeiro nível, só peças, estruturada), com **número de item fixo guardado na montagem** (`ItemNo`, `bom.rs:35`), que mantém o número entre revisões e nunca reaproveita o de itens que saíram. O teste cobre isso (`bom.rs:208-227`).
  - A chave do número é o **caminho relativo** (`key_of`, `bom.rs:53`).
  - O projeto (`Project`) só tem `DocEntry` de **peças** (`lib.rs:72-81`). Não tem montagens, desenhos, quantidades nem instâncias. O CSV exporta uma linha por documento (`lib.rs:267-275`), sem estrutura. Não escapa `;` nem aspas e não grava BOM UTF-8, o que pode estragar acentos no Excel [C].
  - `plm-vpm.md` descreve `STRUCTURE` (pai, filho, quantidade, matriz, configuração) mas é só contexto.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** o projeto passa a registrar **todos os tipos de documento** (peça, montagem, desenho, derivado) com `uid`, PN, revisão e estado. A estrutura é **derivada** do conteúdo das montagens (não digitada à mão), guardada no índice como cache: `(pai_uid, filho_uid, qtd, instâncias, rev_alvo)`.
  - **Regras:** a BOM exportada tem coluna do item, PN, descrição, quantidade, revisão, estado, material, massa. CSV com escape correto e BOM UTF-8. Item pelo `uid`. Dois componentes com mesmo PN e arquivos diferentes geram aviso.
  - **Casos de teste:** montagem com 2 níveis e peça repetida somam quantidade; renomear arquivo mantém o número de item; exportar CSV com `;` na descrição não desloca colunas.

---

### Onde é usado (CAD avançado B/PLM avançado: Where-Used / CAD avançado A PDM: Where Used / CAD avançado C-Vault: Uses, Where Used / Fusion: Data Panel / Fenkra: ausente)

- **O que faz:** responde "quem usa esta peça?" (e o inverso, "o que esta montagem contém?"), com revisão e estado.

- **CAD avançado B/PLM avançado — no detalhe:** consulta de onde-usado na estrutura de produto, por nível, com filtro por maturidade. [C] No PLM avançado, as revisões diferentes de um mesmo item aparecem como usos diferentes. [C]

- **CAD avançado A PDM — no detalhe:** aba **Where Used** (pais, em um nível ou todos) e **Contains** (filhos, com opção de mostrar a árvore de referências). Respeita a versão, e dá para ver "usado em revisão Liberada". [C]

- **CAD avançado C/Vault — no detalhe:** **Uses** e **Where Used** por arquivo ou item, nos dois sentidos. [V, resumo de busca] Fusion: o Data Panel mostra "onde é usado", "o que usa" e desenhos relacionados. [V]

- **Fenkra hoje:** ausente (catálogo PLM-05). O ADR 0006 diz que "Onde é usado" e gerenciador de links ficam "triviais" se as publicações forem indexadas, mas nada foi indexado ainda.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** botão direito num documento ▸ **Onde é usado** (um nível / todos os níveis) com colunas pai, PN, revisão alvo, estado, quantidade, e atalho Abrir.
  - **Implementação:** índice invertido `filho_uid → [(pai_uid, rev)]` atualizado a cada salvamento do pai e reconstruído por varredura (também no modo pasta, sem servidor).
  - **Regras:** o aviso de impacto antecede qualquer operação destrutiva (renomear, excluir, tornar obsoleto): "usada em 4 montagens (1 Liberada)".
  - **Casos de teste:** tornar obsoleta uma peça usada em montagem Liberada mostra o aviso; where-used de peça só usada em subconjunto mostra a cadeia até o topo; ciclo não trava.

---

### Linha de base e efetividade (PLM avançado: Effectivity / PDM: revisão de montagem / Vault: Baseline (Change Order) / Fenkra: ausente)

- **O que faz:** *Baseline* congela o conjunto exato de versões de uma estrutura num momento (o que foi liberado junto). *Efetividade* diz em que condições (data, lote, número de série, unidade) uma revisão vale.

- **PLM avançado — no detalhe:** existe efetividade em instâncias de peça e **cortes configurados**, e erros reportados mostram que é preciso desanexar a efetividade para operar. [V, resumo de busca de APAR] A comunidade descreve revisão de engenharia "baseada em efetividade" e evolução por efetividade, em que a versão do modelo pode seguir Em trabalho enquanto a estrutura muda. [V, resumo de busca]

- **CAD avançado A PDM / Vault — no detalhe:**
  - PDM: ao liberar uma montagem, a revisão do pai fixa as revisões dos filhos "como liberadas" (estado de referência). [C]
  - Vault Professional: *Change Orders* registram o que foi alterado e liberam em lote; *Items* têm efetividade por data. [C]
  - CAD avançado F/PLM avançado: *configuration specifications* (por exemplo "latest", "as matured", "baseline") escolhem as versões da estrutura. Uma página da fornecedor de CAD avançado trata "As Matured". [V, só título de resultado]

- **Fenkra hoje:** ausente. Existe a ideia do ADR 0006 de "revisão-alvo" por link (trabalho ou revisão congelada), sem implementação.

- **Melhor prática para o Fenkra:**
  - **Baseline (P2):** ao liberar uma montagem, gravar um `manifesto` `{uid → rev/hash}` de toda a árvore, com data e comentário. Abrir a baseline resolve cada componente para o hash congelado. Comparar duas baselines lista adicionados/removidos/trocados.
  - **Efetividade (P3):** campo opcional na revisão `{de, até, lote, série}`. Só depois do resto.
  - **Regras:** liberar montagem exige que cada filho tenha revisão liberada, e o manifesto guarda essa revisão (não "a mais recente").
  - **Casos de teste:** editar peça depois da baseline não altera a abertura da baseline; comparar baseline A × B mostra a peça trocada de rev B para C.

---

### Regras de part number (PLM avançado: Numbering / CAD avançado A PDM: Serial numbers / Vault: Numbering schemes / Fenkra: prefixo FK- + contador)

- **O que faz:** gera o código do item por regra, com campos fixos, listas, contadores e dados da peça.

- **PLM avançado/CAD avançado B — no detalhe:** regras de numeração configuradas no servidor, com contadores por tipo. O PN é atributo da referência de produto. [C] (O catálogo cita "Numbering rules (PLM avançado)".)

- **CAD avançado A PDM — no detalhe:** **números de série** (contadores) usados em variáveis do cartão de dados (data card) e no revisão. Contadores e texto compõem o número, e a troca de esquema reinicia contadores. [V, resumo de busca de help.CAD avançado A] Cartão de dados com campos preenchidos pelo usuário, listas ou propriedades do arquivo. [C]

- **Vault/CAD avançado C — no detalhe:** [V]
  - O administrador define **esquemas de numeração** e escolhe quais aparecem no Copy Design (caixa **Shown**) e qual é o padrão (**Default**).
  - No painel de numeração, cada arquivo mostra o nome original e o novo. Conforme o esquema, o Vault pede prefixo/sufixo, localizar-e-substituir, escolha de lista ou texto livre.
  - **Os números automáticos só são reservados quando a cópia é executada**, e aplicados depois que ela termina com sucesso. O painel mostra uma prévia.
  - Um esquema pode conter **só uma sequência automática**. As demais partes são texto fixo ou lista (resposta de um gerente de produto no fórum). O último número usado fica no banco, não nos nomes de arquivo existentes. [V, fórum]
  - Provedores de numeração personalizados exigem o SDK do Vault.
  - É possível copiar linhas para o Excel, editar e colar de volta. Tem a opção "None" (nomeação antiga).

- **Fenkra hoje:**
  - `Index.pn_prefix = "FK-"` e `next_pn` (`lib.rs:99-100, 143, 169-173`). O PN sai como `FK-0001`.
  - `check_pn` bloqueia PN duplicado dentro do projeto (`lib.rs:197-202`), sem normalizar maiúsculas e espaços (`"fk-0001"` ≠ `"FK-0001"`).
  - O PN é uma `String` livre em `Properties.pn` (`fenkra-core/doc.rs:1159`).
  - Não há regra com campos, ambientes nem serial por categoria (backlog B47, catálogo PLM-06 "parcial"). O documento `plm-pn-engine.md` descreve campos entre colchetes, ambientes e histórico opcional em SQLite.
  - Bugs prováveis: contador consumido mesmo quando a criação falha; contador entre duas instâncias colide (ficha 12); PN pode ser editado após liberação.

- **Melhor prática para o Fenkra:**
  - **Fluxo (alinhado a B47):** *Ambiente* → *Regra* → campos → prévia `AERO-NAS-620-3-B` → **reserva só ao gravar**. Campo de serial usa contador do ambiente ou da categoria, com `[SERIAL:5]`.
  - **Regras:**
    - No máximo **uma** sequência automática por regra (limitação do Vault que simplifica a reserva).
    - Prévia com `?????` no lugar do serial até gravar.
    - Normalizar (trim, maiúsculas) e checar unicidade **global do ambiente**, não só do projeto.
    - Campos por lista fechada (norma, material) vêm de tabelas do ambiente. Campo derivado (dimensão) é lido da peça e fica em aviso se a peça mudar depois.
    - PN imutável após a primeira revisão; mudança de dimensão que altera o PN pede nova peça.
    - Colagem em lote via planilha (como o Excel do Vault) para renumerar na cópia.
  - **Casos de teste:** `AERO-[NORMA]-[CODIGO]-[REV]` com NAS, 620-3, B dá `AERO-NAS-620-3-B`; regra com duas sequências é rejeitada; criar 100 documentos em paralelo não repete serial; falha de gravação não consome o número.

---

### Copiar projeto / Copy Design (Vault: Copy Design / CAD avançado A: Save As with References, Pack and Go / CAD avançado C: Design Assistant Copy / CAD avançado B: Send To / Fenkra: Duplicar peça)

- **O que faz:** cria um novo conjunto de arquivos a partir de um existente, com novo PN e novos nomes, escolhendo o que reaproveitar e o que copiar.

- **Vault — no detalhe:** [V]
  - O **Copy Design** abre o conjunto, cada arquivo com ação (copiar, reaproveitar, excluir) [a página lida não detalha as ações; ficam como [C]], e o painel de numeração dá o novo nome por esquema.
  - O botão direito num arquivo permite **Change Scheme**. A tabela copia para o Excel e volta.
  - Os números automáticos são reservados só na execução.

- **CAD avançado A — no detalhe:** Save As with References e Pack and Go com prefixo/sufixo (fichas 3 e 6). [V]

- **CAD avançado C — no detalhe:** Design Assistant ▸ Copy: marcar arquivos, Action = Copy, e **Change Location** escolhendo uma pasta ou biblioteca nomeada do projeto. Copiar uma montagem cria uma variante compartilhando as peças não alteradas. [V]

- **Fenkra hoje:** só `Project::duplicate` (`lib.rs:259-264`, botão "Duplicar", `app.rs:5253`), para uma peça: nome "(cópia)", PN novo, sem histórico. Não duplica montagens, não reapontando nada.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** *Copiar conjunto…* a partir de uma montagem. Tabela por linha: Ação (Copiar, Reutilizar, Excluir), Nome atual, Novo nome, Novo PN (pela regra do ambiente). Reutilizar mantém o `uid` e o link para o mesmo arquivo. Copiar gera `uid` novo. O botão "Aplicar esquema" preenche em massa.
  - **Regras:** peça Liberada é *reutilizada* por padrão (cópia só se pedir); a montagem nova referencia as cópias; o desenho associado é copiado e reapontado; a prévia do PN não consome contador.
  - **Casos de teste:** copiar montagem de 5 peças reutilizando 3 e copiando 2: as 2 cópias têm PN novo e a montagem nova aponta certo; nada na origem muda.

---

### Documentos derivados e conversor (CAD avançado B: STEP/IGES import, Associative export / CAD avançado A: Import Diagnostics / CAD avançado C: Import / Fusion: Upload / Fenkra: ausente)

- **O que faz:** trata um arquivo importado (STEP, IGES, STL…) como um documento que **veio de outro** e que pode ficar desatualizado, em vez de um arquivo qualquer.

- **CAD avançado B, CAD avançado A, CAD avançado C — no detalhe:** todos importam para um documento nativo. Em geral esse documento *não mantém associação* com o arquivo de origem (é uma cópia, sem atualizar). [C] O Fusion guarda o arquivo original enviado como "Archive". [C] Nenhum desses detalhes foi confirmado em fonte nesta pesquisa. Marcar como [C].

- **Fenkra hoje:**
  - Não há importador de STEP/IGES. Há **Exportar STEP/STL** (`app.rs:2528-2535`, `880-892`) e o comando **MeshImport**, que importa STL/OBJ como recurso e guarda o caminho absoluto do arquivo (`app.rs:1754-1756`).
  - Nenhuma estrutura registra origem, hash, versão do conversor ou data. O catálogo não menciona o "conversor" como item próprio.

- **Melhor prática para o Fenkra:**
  - **Modelo:** documento com `tipo: Derivado { origem: {formato, caminho_original, hash, tamanho, mtime, importado_em}, conversor: {nome, versão, opções}, estado: Sincronizado | Desatualizado | OrigemAusente }`.
  - **Fluxo:** Importar → copia a origem para `.fenkra/origens/<hash>.<ext>` (opção: embutir ou só referenciar), converte, grava o `.fkp` derivado com `uid` próprio. Se o arquivo de origem muda, o derivado fica **Desatualizado** e há **Reimportar** que mantém o `uid`, o PN e as referências (como a "substituição", ficha 9). Montagem que usa o derivado mostra o selo "derivado".
  - **Regras:**
    - Derivado é somente leitura de geometria até **Desvincular** (vira peça nativa, sem origem).
    - Exportação (STEP/STL) é *saída*, não documento: guarda no manifesto o hash e a revisão exportada, mas não cria arquivo no projeto.
    - No Empacotar, a origem entra por padrão.
    - O PN do derivado segue a regra de ambiente (ou herda o do fornecedor como atributo extra).
  - **Casos de teste:** importar, alterar o original, abrir o projeto: derivado marca "Desatualizado"; reimportar mantém o link da montagem; sem a origem, estado OrigemAusente mas abre; Desvincular remove o selo.

---

### Montagem: salvar contexto (CAD avançado B: objeto de trabalho, links contextuais / CAD avançado A: Edit Part, referências externas / CAD avançado C: Edit in place, Adaptive / Fenkra: edit_part e back_to_assembly)

- **O que faz:** decide, ao editar uma peça dentro de uma montagem, o que é salvo, onde, e como os links entre peça e montagem se mantêm.

- **CAD avançado B — no detalhe:** objeto de trabalho define o objeto em trabalho. Os links contextuais ficam no produto e se atualizam. O Save Management lista pai e filhos modificados, e Save Auto cobre os dependentes. [V, em parte]

- **CAD avançado A — no detalhe:** editar a peça no contexto cria **referências externas** (a peça guarda um link para a montagem). Salvar a peça sem a montagem aberta deixa o link fora de contexto. Os estados são `->` (em contexto), `->?` (fora), `->*` (travado) e `->x` (quebrado), e é possível Travar, Destravar e Quebrar (ADR 0011, princípio 3). [V, ADR; C, confirmação externa]

- **CAD avançado C — no detalhe:** edição no local (*edit in place*) e peças adaptativas. Ao salvar, o CAD avançado C salva os arquivos editados e a montagem. [C]

- **Fenkra hoje:**
  - `edit_part` (`asm_ui.rs:890-906`) carrega a peça por cima do documento aberto (`load_doc_pub`) e muda de bancada. A montagem fica em `st.editing_part`. O botão "↩ Voltar à montagem" (`app.rs:2600-2603`) chama `back_to_assembly` (`asm_ui.rs:909-918`): `save_pub` salva a peça (sem pergunta), esquece o cache, resolve a montagem.
  - Salvar a montagem não salva as peças. `back_to_assembly` salva sem checar se a peça é de revisão congelada (`save()` bloqueia e avisa, mas a montagem volta mesmo assim, deixando a peça sem salvar). A montagem não é marcada como modificada quando a peça muda.
  - `AsmNewPart` cria o arquivo vazio no disco **antes** de a montagem existir como arquivo (`asm_ui.rs:450-471`). Cancelar depois deixa um `.fkp` órfão. O caminho fica absoluto quando a montagem ainda não foi salva.
  - O ADR 0011 prevê B54 (edição no contexto sem sair da montagem) e B55 (referências e publicações na montagem).

- **Melhor prática para o Fenkra:**
  - **Fluxo:** a árvore tem um só componente ativo (B54). O indicador "•" por componente mostra modificados (também no desenho). Salvar tudo (ficha 2) cobre a montagem e as peças editadas.
  - **Regras:**
    - Ao voltar da edição de uma peça, se o salvamento falha ou é bloqueado, **não voltar**, mostrar o motivo.
    - Peça nova criada na montagem só grava no disco no Salvar tudo, com nome e pasta obrigatórios (já previsto em B56).
    - Peça modificada marca a montagem pai como "dependência" no plano.
    - Links de contexto guardam `uid` + `rel` e o estado (`->`, `->?`, `->*`, `->x`).
  - **Casos de teste:** editar peça no contexto, não salvar, fechar: aviso listando peça e montagem; cancelar "nova peça" não deixa arquivo; peça congelada na montagem não permite voltar sem decisão.

---

### Histórico, comparação e auditoria (CAD avançado B/PLM avançado: Compare / CAD avançado A PDM: History, Compare / Vault: History / CAD avançado E: Versions and history / Fenkra: lista de versões)

- **O que faz:** responde "quem mudou o quê e quando" e "o que mudou entre duas revisões".

- **CAD avançado A PDM — no detalhe:** o histórico do arquivo mostra versões, revisões, mudanças de estado e eventos como renomeação. [V, GoEngineer] Comentários obrigatórios dão um histórico confiável. [V] Os administradores podem pedir que também apareçam as operações de check-out no histórico (fórum). [V, só título]

- **CAD avançado E — no detalhe:** o painel Versions and history mostra num gráfico toda mudança de todos os usuários, em versões e branches. Dá para buscar por palavra-chave e filtrar por aba, tipo de aba, autor e data. Há branch, merge, restauração e versões automáticas ao atualizar referências. [V, página lida]

- **PLM avançado/Vault — no detalhe:** log de alterações (PLM avançado: `CHANGE_LOG` no contexto do usuário é proposta nossa, não do produto). Vault tem histórico por arquivo e por item. [C]

- **Fenkra hoje:** `Version` guarda hash, hora e uma nota fixa "salvo". O painel lista as 30 últimas (`app.rs:5273-5283`) com **abrir**. Não compara versões, não grava autor.

- **Melhor prática para o Fenkra:**
  - **Fluxo:** painel de histórico com autor, hora, comentário, tipo de evento (versão, revisão, transição, renomeação, check-in/out). **Comparar** duas versões: lista de comandos, parâmetros e propriedades diferentes (diff de JSON estruturado), com volume/massa e caixa envolvente quando disponíveis.
  - **Regras:** o diário (ficha 12) é só acréscimo. Eventos de tarefa longa não entram.
  - **Casos de teste:** comparar v3 × v5 mostra "parâmetro L: 20→25"; renomear arquivo aparece como evento; autor sempre preenchido.

---

### Propriedades e metadados (CAD avançado A PDM: Data Card / Vault: propriedades e Item / CAD avançado B: Properties / Fenkra: Properties)

- **O que faz:** mantém PN, descrição, material etc. consistentes entre o arquivo, o gerenciador, a BOM e o desenho.

- **CAD avançado A PDM — no detalhe:** o cartão de dados mapeia variáveis para propriedades do arquivo, e há um fórum com o clássico "renomeei o arquivo e o cartão não atualizou" (só título). [V] **Vault:** sincronização de propriedades arquivo ↔ banco, definida por mapeamentos, com "primary model state" para itens. [V]

- **Fenkra hoje:** `Properties { pn, description, material, part_type, custom, stock_length }` (`fenkra-core/doc.rs:1158-1168`). O índice **copia** pn, nome e descrição do documento ao salvar (`lib.rs:216-218`). A fonte da verdade é o documento, e o índice é cache. Se o JSON mudar por fora, o índice fica desatualizado. O desenho resolve campos da legenda por `resolve` (`fenkra-desenho/lib.rs:125`).

- **Melhor prática para o Fenkra:**
  - **Regras:** uma só fonte da verdade (o documento), o índice sempre reconstruível. Ao abrir o projeto, comparar `hash/mtime` do arquivo com o do índice e reindexar o que mudou. Propriedades obrigatórias por ambiente (PN, descrição, material) validadas no Liberar.
  - **Casos de teste:** editar o `.fkp` por fora e reabrir o projeto: índice atualiza; liberar sem descrição (quando exigida) é recusado.

---

### Permissões e papéis (CAD avançado A PDM: grupos e permissões por estado / Vault: roles / PLM avançado: roles / Fenkra: ausente)

- **O que faz:** define quem lê, escreve, libera, exclui.

- **CAD avançado A PDM — no detalhe:** permissões por grupo, por estado e por pasta; o estado pode definir quem altera. [V, título de página de ajuda "IDD_USER_GROUP_STATEPERMISSION"]
- **plataforma PLM avançada — no detalhe:** o proprietário controla Privado, e travar e tornar Obsoleto exigem permissão. [V]
- **CAD avançado C — no detalhe:** projetos single-user, Vault, semi-isolated e shared. [V]
- **Fenkra hoje:** ausente. O `plm-vpm.md` prevê `USER/ROLE/PERMISSION` para a fase 3.
- **Melhor prática para o Fenkra:** na fase pasta, só **papéis simples** (Autor, Aprovador, Leitor) em `.fenkra/papeis.json`, aplicados pela interface (não é segurança de verdade em pasta compartilhada, e o programa deve dizer isso). Segurança real só com servidor (fase 2 do `plm-vpm.md`). **Caso de teste:** Leitor não vê o botão Liberar nem consegue pelo atalho; a tentativa vira evento no diário.

---

## 3. Requisitos levantados

Prioridades: P0 quebra o uso ou perde dado, P1 importante, P2 desejável, P3 futuro.

### Funcionais (RF-PLM)

| ID | Requisito | Prio | Critério de aceite |
|---|---|---|---|
| RF-PLM-01 | O Fenkra pergunta antes de descartar alterações ao abrir, criar, editar-no-contexto ou fechar | P0 | Com peça modificada, abrir outro arquivo, Nova peça, duplo clique em componente e fechar a janela mostram Salvar/Descartar/Cancelar. Teste de UI com os 4 gatilhos |
| RF-PLM-02 | Gravação atômica de `.fkp`, `.fka`, `.fkd` e do índice, com uma geração `.bak` | P0 | Interromper o processo em 100 gravações simuladas nunca deixa arquivo truncado. Teste de injeção de falha |
| RF-PLM-03 | Detecção de mudança externa no salvamento (mtime + hash base) | P0 | Alterar o arquivo por fora e salvar abre o aviso "mudou no disco" com Sobrescrever/Salvar como/Recarregar |
| RF-PLM-04 | Plano de salvamento (Salvar tudo) listando cada arquivo e o motivo (Novo, Editado, Dependência, Derivado desatualizado, Só leitura, Conflito) | P0 | Montagem com 1 peça editada, 1 nova e 1 só leitura: o plano mostra 3 linhas corretas; Cancelar não grava nada |
| RF-PLM-05 | Ctrl+S salva o documento ativo; Ctrl+Shift+S abre o plano; grupo Salvar / Salvar tudo / Salvar como igual nas 3 bancadas | P0 | Os três atalhos têm o mesmo efeito em peça, montagem e desenho |
| RF-PLM-06 | Salvar como com 4 modos (renomear/mover, cópia e continuar, cópia e abrir, novo documento) | P1 | Cada modo tem teste: `uid` mantido ou novo, documento ativo trocado ou não, links atualizados |
| RF-PLM-07 | Salvar como com referências: tabela editável, prefixo/sufixo, desfazer, conflito de nomes | P1 | Montagem de 3 níveis salva com prefixo: todos os links da cópia resolvem; o original permanece |
| RF-PLM-08 | Identidade estável: todo documento tem `uid` no arquivo; referências guardam `uid` + `rel` (+ `hash/rev` opcional) | P0 | Renomear e mover um arquivo no Explorer e abrir a montagem: o assistente acha pelo `uid` |
| RF-PLM-09 | Caminhos sempre relativos quando possível (inclusive `..`), absolutos só entre drives; nunca relativos ao diretório de trabalho do processo | P0 | Mover a árvore inteira para outro drive: 0 links quebrados; link externo relativo sem projeto resolve contra a pasta do documento |
| RF-PLM-10 | Resolução de referências em ordem definida e diálogo Resolver | P1 | Cenário com nome duplicado nunca abre o arquivo errado em silêncio; teste cobre cada etapa da ordem |
| RF-PLM-11 | Painel de Relações (Contém / Onde é usado) com estado do link (`->`, `->?`, `->*`, `->x`) | P1 | Árvore de 3 níveis com um arquivo movido mostra `->x` e a ação Reapontar |
| RF-PLM-12 | Renomear/Mover como transação com pré-visualização dos dependentes e desfazer | P1 | Renomear peça usada em 3 montagens e 1 desenho: todos abrem. Falha no meio não deixa estado parcial |
| RF-PLM-13 | Assistente Reparar links (varredura por `uid`) | P1 | Pasta com 5 arquivos movidos: o assistente propõe os 5 reapontamentos sem abrir a geometria |
| RF-PLM-14 | Empacotar para pasta ou zip (plano/hierárquico, prefixo/sufixo, desenhos, linkados, derivados, pular bibliotecas) com manifesto e log | P1 | Pacote reaberto em outra máquina: 0 links quebrados; o hash da origem não muda |
| RF-PLM-15 | Empacotar nunca redireciona colisão de nome em silêncio; renomeia e lista | P1 | Duas peças `base.fkp` em pastas diferentes viram dois arquivos e a tabela mostra os dois |
| RF-PLM-16 | Substituir componente: todas as instâncias, por revisão ou versão, com relatório de restrições reaplicadas | P1 | Substituir por rev B mantém número de item e reaplica restrições por nome publicado; as sem alvo ficam listadas |
| RF-PLM-17 | Recuperação automática em `.fenkra/recuperacao` e diálogo na abertura | P1 | Matar o processo com peça modificada e reabrir oferece a recuperação; salvar a limpa |
| RF-PLM-18 | Check-out/check-in por documento, com trava, comentário e desfazer check-out | P1 | Dois processos: o segundo abre só leitura com banner "em check-out por X"; desfazer restaura |
| RF-PLM-19 | Trava com batimento e expiração em pasta de rede/OneDrive, e check-in com verificação de hash base | P1 | Dono morto: a trava vence após o TTL e pode ser assumida com confirmação; base alterada bloqueia o check-in |
| RF-PLM-20 | Modo pasta com um arquivo por registro; índice agregado como cache reconstruível | P0 | Apagar o índice e reabrir reconstrói tudo; duas instâncias não se sobrescrevem |
| RF-PLM-21 | Contadores de ID e PN por reserva com trava curta | P0 | 100 criações concorrentes de 2 processos: 0 `id` ou PN repetidos |
| RF-PLM-22 | Detectar e listar arquivos de conflito do OneDrive (`-NOMEDOPC`) e abrir Comparar/Escolher | P2 | Arquivo sintético `x-PC.fkp` aparece no painel Conflitos |
| RF-PLM-23 | Versão automática com autor, hora e comentário opcional; agrupamento por sessão | P1 | 200 salvamentos aparecem como sessões colapsadas; todo registro tem autor |
| RF-PLM-24 | Esquema de revisão configurável (letras com opção de pular I,O,Q,S,X,Z; números; maior.menor) | P2 | Esquema com saltos vai de H a J; maior.menor vai de A.1 a A.2 |
| RF-PLM-25 | PN imutável após a primeira revisão; mudança exige "novo documento" | P0 | Alterar PN de peça com revisão A é recusado com a explicação |
| RF-PLM-26 | Grafo de estados configurável por ambiente com comentário obrigatório opcional | P1 | Transição sem comentário (quando exigido) é recusada; estados e transições vêm do JSON do ambiente |
| RF-PLM-27 | Liberado e Congelado são só leitura; editar exige nova revisão | P1 | Salvar em documento Liberado cria uma ação "Nova revisão" em vez de gravar em silêncio |
| RF-PLM-28 | Transição em lote, filhos antes do pai | P1 | Liberar montagem com filho Em trabalho é recusado; liberar em lote libera os filhos primeiro |
| RF-PLM-29 | Projeto registra todos os tipos de documento (peça, montagem, desenho, derivado) | P1 | Projeto novo com 1 de cada: o painel lista os quatro com PN, rev e estado |
| RF-PLM-30 | Estrutura derivada do conteúdo das montagens (pai, filho, qtd, rev alvo), no índice | P1 | Editar uma montagem atualiza a estrutura no próximo salvamento; reconstruir do zero dá o mesmo resultado |
| RF-PLM-31 | Número de item da BOM por `uid`, não por caminho | P1 | Renomear o arquivo mantém o número de item |
| RF-PLM-32 | Exportação CSV/XLSX da BOM com escape correto e BOM UTF-8 | P2 | Descrição com `;`, aspas e acento abre certa no Excel |
| RF-PLM-33 | Onde é usado (um nível e todos) a partir de índice invertido | P1 | Peça em subconjunto mostra a cadeia até o topo; tornar obsoleta peça usada em Liberada mostra o aviso |
| RF-PLM-34 | Baseline: manifesto `{uid → rev/hash}` ao liberar montagem; abrir e comparar baselines | P2 | Editar peça depois não muda a baseline; a comparação lista trocas |
| RF-PLM-35 | Efetividade opcional na revisão (`de`, `até`, `lote`, `série`) | P3 | Consulta "o que vale na série 105" retorna as revisões certas |
| RF-PLM-36 | Regras de PN com campos entre colchetes, ambientes, lista e uma sequência automática por regra | P1 | `AERO-[NORMA]-[CODIGO]-[REV]` gera `AERO-NAS-620-3-B`; duas sequências são rejeitadas |
| RF-PLM-37 | Prévia do PN sem consumir contador; reserva ao gravar com sucesso | P0 | Falha de gravação não avança o contador; prévia mostra `?????` |
| RF-PLM-38 | Unicidade do PN normalizada (trim, maiúsculas) e global no ambiente | P1 | `fk-0001` e `FK-0001` colidem |
| RF-PLM-39 | Copiar conjunto: ação por linha (Copiar/Reutilizar/Excluir), esquema de numeração em lote | P2 | Copiar montagem de 5 peças com 3 reutilizadas gera 2 cópias com PN novo e links certos |
| RF-PLM-40 | Documento derivado: origem, hash, conversor, estado (Sincronizado/Desatualizado/OrigemAusente), Reimportar mantendo `uid` | P1 | Alterar o original marca "Desatualizado"; reimportar mantém o link da montagem |
| RF-PLM-41 | Derivado só leitura de geometria até Desvincular; origem incluída no Empacotar | P2 | Editar derivado é bloqueado; pacote contém a origem |
| RF-PLM-42 | Edição no contexto: voltar à montagem só se o salvamento da peça tiver sucesso; peça modificada marca a montagem | P0 | Peça congelada ou salvamento com erro mantém o usuário na peça com a mensagem |
| RF-PLM-43 | Peça nova na montagem só grava no disco no Salvar tudo, com nome e pasta obrigatórios | P1 | Cancelar a criação não deixa `.fkp` órfão |
| RF-PLM-44 | Histórico com tipo de evento e Comparar duas versões (comandos, parâmetros, propriedades, massa) | P2 | Comparar v3 × v5 mostra "L: 20→25" |
| RF-PLM-45 | Reindexar na abertura quando hash/mtime do arquivo difere do índice | P1 | Editar `.fkp` por fora e reabrir o projeto atualiza nome/PN/descrição |
| RF-PLM-46 | Papéis simples (Autor, Aprovador, Leitor) na fase pasta, com aviso de que não são segurança | P3 | Leitor não consegue liberar pela UI nem pelo atalho |
| RF-PLM-47 | Detecção de ciclo entre documentos na leitura e na criação do link | P1 | A→B→A é recusado na criação e sinalizado ao abrir |
| RF-PLM-48 | Limpeza de `.versoes` que preserva revisões e baselines | P2 | Rodar a limpeza remove só versões intermediárias não referenciadas |

### Não funcionais (RNF-PLM)

| ID | Requisito | Prio | Critério de aceite |
|---|---|---|---|
| RNF-PLM-01 | Nenhuma operação de arquivo corrompe dados em queda de energia ou de processo | P0 | Suite de injeção de falha em salvar, mover, empacotar e check-in: 0 arquivos truncados |
| RNF-PLM-02 | Operações em lote são transações com desfazer | P1 | Falha simulada no arquivo N restaura os N−1 anteriores |
| RNF-PLM-03 | Modo pasta não usa banco de dados compartilhado em rede; qualquer SQLite é cache local | P0 | Auditoria de código: nenhum `.sqlite` dentro da pasta compartilhada |
| RNF-PLM-04 | Painel de Relações e Reparar links funcionam sem carregar geometria | P1 | Abrir a árvore de 500 arquivos em < 2 s lendo só cabeçalhos |
| RNF-PLM-05 | Escala: projeto com 5.000 documentos abre o índice em < 3 s e reindexa de forma incremental | P2 | Benchmark com projeto sintético |
| RNF-PLM-06 | Resolver referências é determinístico e nunca escolhe outro `uid` em silêncio | P0 | Teste com nomes duplicados e `uid` divergente |
| RNF-PLM-07 | Todo evento relevante grava no diário (usuário, máquina, hora, ação, alvo) em arquivo por usuário, só acréscimo | P1 | Dois usuários geram diários separados sem escrita concorrente |
| RNF-PLM-08 | Formato de arquivo tem `format_version` e migração testada | P1 | Corpus de arquivos antigos abre e salva sem perda |
| RNF-PLM-09 | Mensagens de falha dizem o arquivo, o motivo e a saída (Editar, Substituir, Isolar, Desativar, Reordenar) | P1 | Cada erro de arquivo do catálogo tem texto com arquivo e ação |
| RNF-PLM-10 | Operações de projeto não bloqueiam a interface (progresso e cancelamento) | P2 | Empacotar 500 arquivos mantém a UI responsiva e cancelável |
| RNF-PLM-11 | Compatibilidade Windows: caminhos longos, `\` e `/`, nomes reservados, maiúsculas/minúsculas e acentos | P1 | Corpus de caminhos com acento, espaço e > 260 caracteres |
| RNF-PLM-12 | Segurança dos arquivos importados: derivado e origem não executam código; tamanho máximo configurável | P2 | Arquivo malformado é recusado sem travar |
| RNF-PLM-13 | Nada do que o usuário não pediu é gravado fora da pasta do projeto (exceto preferências) | P1 | Auditoria de escrita por teste de integração |
| RNF-PLM-14 | Cada função deste relatório tem teste com nome próprio no corpus (B53) | P1 | Lista de testes mapeada para RF-PLM-nn |
| RNF-PLM-15 | Pastas somente online/placeholder do OneDrive são detectadas antes de ler ou gravar | P2 | Arquivo-placeholder gera pedido de download, não erro genérico |

---

## 4. Itens de backlog propostos

Dependências usam os IDs atuais do backlog: B25 (VPM fase 1), B47 (PNEngine), B53 (corpus), B54 (edição no contexto), B55 (publicações na montagem), B56 (Salvar tudo) e B59 (árvore e diagnóstico). Os novos IDs abaixo são provisórios, `PLM-xx`.

| ID | Título | O que muda | Prio | Tam. | Dependências |
|---|---|---|---|---|---|
| PLM-01 | Salvamento seguro e confirmação de descarte | Gravação atômica + `.bak` para `.fkp/.fka/.fkd` e índice. Diálogo de alterações não salvas em abrir, nova, editar-no-contexto e fechar (`main.rs`, `app.rs`, `asm_ui.rs`). Detecção de mudança no disco | P0 | M | B53 |
| PLM-02 | Plano de salvamento (estende B56) | Salvar tudo com estados e motivos, nome/pasta para novos, Ctrl+Shift+S, Salvar como com 4 modos e tabela de referências | P0 | G | PLM-01, B56 |
| PLM-03 | Identidade e referências estáveis | `uid` no arquivo, referências `uid+rel(+hash/rev)`, relativos com `..`, resolução em ordem, Resolver, caminho relativo ao documento (corrige `load_link_source` e `rel_path`) | P0 | G | B55 |
| PLM-04 | Painel de Relações e Reparar links | Contém / Onde é usado, estado do link, Reapontar/Substituir/Travar/Quebrar, assistente por `uid`, detecção de ciclo | P1 | G | PLM-03, B59 |
| PLM-05 | Renomear/Mover transacional | Operação com pré-visualização dos dependentes, transação, desfazer, bloqueio por check-out de terceiros | P1 | M | PLM-03, PLM-04 |
| PLM-06 | Empacotar (Pack and Go do Fenkra) | Pasta/zip, plano/hierárquico, prefixo/sufixo, desenhos, linkados, derivados, manifesto e log, verificação final reabrindo o pacote | P1 | G | PLM-03, PLM-02 |
| PLM-07 | Substituir componente avançado | Todas as instâncias, por revisão/versão, relatório de restrições, mapa por nome publicado, número de item por `uid` | P1 | M | PLM-03, B55 |
| PLM-08 | Recuperação automática | Cópias em `.fenkra/recuperacao`, diálogo na abertura, limpeza | P1 | P | PLM-01 |
| PLM-09 | Modo pasta seguro (revisa B25) | Um arquivo por registro, índice como cache, contadores por reserva, diário por usuário, sem SQLite na pasta compartilhada | P0 | G | B25 |
| PLM-10 | Check-out/check-in e travas | `.fenkra/locks`, batimento/TTL, banner de só leitura, comentário, desfazer, verificação de hash base | P1 | G | PLM-09 |
| PLM-11 | Conflitos de sincronização | Detectar `-NOMEDOPC` do OneDrive, painel Conflitos com Comparar/Escolher, placeholders online-only | P2 | M | PLM-09, PLM-12 |
| PLM-12 | Histórico e comparação | Autor/hora/comentário, agrupamento por sessão, Comparar versões, limpeza de `.versoes` | P2 | M | PLM-09 |
| PLM-13 | Ciclo de vida configurável | Grafo de estados por ambiente, comentário obrigatório, Liberado/Congelado só leitura, transição em lote com filhos antes do pai, PN imutável após revisão | P1 | G | PLM-09, PLM-14 |
| PLM-14 | Projeto com todos os tipos de documento e estrutura | Peça/montagem/desenho/derivado no índice, estrutura derivada, item por `uid`, CSV/XLSX correto, reindexação ao abrir | P1 | G | PLM-03, B25 |
| PLM-15 | Onde é usado e impacto | Índice invertido, aviso de impacto em renomear/excluir/obsoletar | P1 | M | PLM-14 |
| PLM-16 | PNEngine com reserva | Regras com campos, ambientes, uma sequência automática, prévia sem consumo, reserva na gravação, unicidade normalizada global (estende B47) | P1 | G | B47, PLM-09 |
| PLM-17 | Copiar conjunto | Tabela Copiar/Reutilizar/Excluir, esquema em lote, desenho associado reapontado | P2 | M | PLM-16, PLM-06 |
| PLM-18 | Baseline e efetividade | Manifesto de baseline ao liberar, abrir/comparar baselines; efetividade como P3 | P2 | M | PLM-13, PLM-14 |
| PLM-19 | Documentos derivados e conversor | Modelo de derivado com origem/hash/conversor/estado, Reimportar mantendo `uid`, Desvincular, origem no pacote | P1 | M | PLM-03 e o importador |
| PLM-20 | Montagem: salvar contexto | Voltar só se salvar, marcação de modificados por componente, peça nova sem arquivo órfão (parte do B54/B56) | P0 | M | B54, PLM-02 |
| PLM-21 | Papéis simples | Autor/Aprovador/Leitor por arquivo em `.fenkra/papeis.json`, com aviso de limite | P3 | P | PLM-13 |
| PLM-22 | Corpus de testes PLM | Um teste nomeado por RF-PLM no B53, incluindo injeção de falha, concorrência e corpus de caminhos | P1 | M | B53 |

**Ordem sugerida:** PLM-01 e PLM-20 (perda de dados) → PLM-03 → PLM-02 → PLM-09 → PLM-04/05/06 → PLM-10/13/14/16 → o resto.

---

## 5. Fontes

**Lidas na íntegra (WebFetch)**
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://knowledge.cadimensions.com/knowledge/pack-and-go-tips-and-tricks
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.sqlite.org/useovernet.html

**Usadas só por resumo de busca (página completa deu 403, 404 ou não carregou)**
- https://community.3dcs.com/help_manual/savemanagement_2.htm
- https://community.3dcs.com/help_manual/savemanagement.htm
- (fonte em procedencia.md)
- https://community.3dcs.com/help_manual/savesendtofunction.htm
- https://community.3dcs.com/help_manual/desk.htm
- https://www.eng-tips.com/viewthread.cfm?qid=136848
- https://www.eng-tips.com/threads/assembly-loosing-links.125258/
- https://www.ibm.com/support/pages/apar/HD35827
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.youtube.com/shorts/qSb0u6pfGCs (só o título)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.javelin-tech.com/blog/?p=70750
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.ibm.com/support/pages/apar/HD34075 e demais APARs PLM avançado citados nos resultados
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://uwindsor.teamdynamix.com/TDClient/KB/ArticleDet?ID=29832
- (fonte em procedencia.md) (só título)

**Código e documentos do Fenkra lidos**
- `crates/fenkra-project/src/lib.rs`
- `crates/fenkra-app/src/app.rs`
- `crates/fenkra-app/src/asm_ui.rs`
- `crates/fenkra-app/src/dwg_ui.rs`
- `crates/fenkra-montagem/src/bom.rs`
- `crates/fenkra-montagem/src/load.rs`
- `crates/fenkra-montagem/src/lib.rs`
- `crates/fenkra-core/src/doc.rs`
- `docs/contextos/plm-vpm.md`
- `docs/contextos/plm-pn-engine.md`
- `docs/arquitetura/adr/0006-links-e-projeto-relacional.md`
- `docs/arquitetura/adr/0011-filosofia-de-modelagem.md`
- `docs/benchmark/catalogo.json`
- `docs/backlog/backlog.json`

---

## Handoff

- **Resultado:** relatório completo, com 24 fichas, 48 RF-PLM, 15 RNF-PLM e 22 itens de backlog propostos.
- **Evidência:**
  - Código lido: projeto, salvar da peça, montagem e desenho, BOM, cache de peças, links externos.
  - Páginas lidas de CAD avançado B (EDS, tutor de montagem), CAD avançado C (Pack and Go, projeto, Design Assistant, Vault), Vault, plataforma PLM avançada, CAD avançado E, cadimensions, Visiativ, GoEngineer e SQLite.
- **Arquivos alterados:** nenhum. Todo o conteúdo está nesta resposta, para o lead gravar em `docs/pesquisa/plm-vpm-e-save.md`.
- **Verificações:** só leitura de código e pesquisa web. Não rodei build nem testes.
- **Falhas:**
  - help.CAD avançado A.com não carregou (só a navegação do site). Blogs Javelin, Hawk Ridge, 3DCS e PLM Group deram 403.
  - CAD avançado Bdoc.free.fr recusou a conexão.
  - Nada público útil sobre CAD avançado F/PLM avançado, CAD avançado G/PLM avançado e CAD avançado H, que ficam como [C].
  - O comportamento exato de Save As do CAD avançado A e do CAD avançado C, e a distinção Propagate Directory × Send To, estão em fontes secundárias ou de fórum.
- **Riscos:**
  - Os "bugs prováveis" vêm da leitura do código, não de execução. Convém confirmar com testes antes de priorizar: descarte silencioso ao abrir, `rel_path` só para descendentes, link externo relativo sem projeto e colisão de IDs em pasta compartilhada.
  - Os trechos marcados "(resumo de busca)" devem ser reconferidos nas ajudas oficiais antes de virarem regra.
- **Nota de processo:** a sessão estava em modo de planejamento (somente leitura) e a ferramenta ExitPlanMode não estava disponível. Como a entrega pedida é este relatório, ele vai como resposta final e nenhum arquivo foi criado.

STATUS: DONE_WITH_CONCERNS
SUMMARY: As 5 maiores lacunas do Fenkra são: (1) perda silenciosa de dados, sem confirmação ao descartar ou fechar e com escrita não atômica; (2) links entre arquivos feitos de caminhos de texto, sem `uid` estável, com `rel_path` só para descendentes e links externos relativos ao cwd; (3) ausência de Salvar tudo/Salvar como, Empacotar, Renomear/Mover seguro e Onde é usado; (4) modo pasta sem trava e com IDs e PNs que colidem entre usuários; (5) projeto só com peças, sem estrutura de produto, baseline ou modelo de documento derivado do conversor.
