# Benchmark de detalhe: PDM / PLM e migração de projetos

> Texto próprio, a partir de documentação pública e do código do Fenkra. Aprofunda, sem repetir,
> [pdm-plm.md](../pdm-plm.md), [plm-vpm-e-save.md](../../pesquisa/plm-vpm-e-save.md) e
> [conversor-de-arquivos.md](../../pesquisa/conversor-de-arquivos.md). Comparação com "softwares de
> modelagem CAD avançados" e com o FreeCAD. Fontes consultadas entram em `procedencia.md` pela líder.
> Legenda de confiança: [V] lido em fonte pública; [C] conhecimento geral, não confirmado agora.

## 1. Resumo: as 5 maiores lacunas

1. **Migrar não tem porta de entrada.** `import_step` existe no kernel
   (`crates/fenkra-kernel/src/lib.rs:706`, declarado em `ffi.rs:198`) e nenhuma tela o chama. O menu
   Arquivo só sabe exportar (`crates/fenkra-app/src/app.rs:2564` e `:2568`). Quem chega de outro CAD não
   consegue nem abrir o STEP que exportou. É a lacuna que mais pesa na barreira de entrada.
2. **Gravação que pode perder dados e sem rede de proteção.** O índice usa arquivo temporário e
   `rename` (`crates/fenkra-project/src/lib.rs:154-159`), mas a versão e o documento de trabalho são
   gravados direto (`lib.rs:211` e `:214`). O Salvar solto grava em cima do arquivo
   (`app.rs:852`) e não pergunta nada ao abrir outro documento. Os outros CADs têm plano de salvamento,
   salvar como, cópia de segurança e recuperação (B56, B65, B93 cobrem; nada pronto).
3. **Revisão e estado são rótulos sem fluxo.** `DocState` tem três valores (`lib.rs:40-44`) e a troca
   é livre: o botão "Tornar obsoleto" muda o estado sem pedir motivo, sem papel e sem registro de quem fez
   (`app.rs:5294`). Não há transições nomeadas, aprovação, nem bloqueio de edição em "Liberado". Os
   gerenciadores de dados dos CADs avançados guardam quem, quando e por quê.
4. **Sem onde é usado, sem comparar versões, sem estrutura de produto.** O projeto só conhece peças
   (`DocEntry`, `lib.rs:72-82`); o CSV (`lib.rs:267`) lista documentos, não estrutura. Não dá para
   perguntar "quem usa esta peça?" antes de mudar uma revisão, nem "o que mudou entre v3 e v5?" (B94,
   B214 cobrem; nada pronto).
5. **Identidade e unidade de numeração frágeis.** `next_pn` e `next_id` são contadores no índice
   (`lib.rs:169`, `:176`); duas pessoas na mesma pasta colidem. PN com prefixo fixo `FK-`, sem regra por
   campos (B47) e sem imutabilidade após a primeira revisão.

Fora do rol das cinco, mas decisivo para o objetivo do produto: **não existe um registro de
"origem importada"** no documento, então um corpo vindo de STEP perde para sempre a ligação com o
arquivo de onde veio (ver seção de migração).

## 2. Checklist por comando

Critérios: **Intuitivo** (descobre sem manual), **Prévia** (mostra o efeito antes de confirmar),
**Manipulação** (age direto na tela, sem diálogo longo), **Ícone claro**, **Tooltip** (nome, atalho,
frase), **Dica por campo**, **Fluxo** (a ordem de passos é a esperada pelo mercado). Valores: ok,
parcial, falta. A avaliação do Fenkra vem do código lido; "Prévia" e "Manipulação" não se aplicam bem a
comandos de arquivo e valem como "mostra o que vai acontecer" e "age sem sair do contexto".

| Comando | Intuitivo | Prévia | Manipulação | Ícone claro | Tooltip | Dica por campo | Fluxo | Nota |
|---|---|---|---|---|---|---|---|---|
| Salvar (peça solta) | ok | falta | parcial | falta | falta | falta | parcial | Ctrl+S existe; sem confirmação de pasta nova, sem aviso ao descartar (`app.rs:821-856`) |
| Salvar (projeto, "Salvar versão") | parcial | falta | falta | falta | falta | falta | parcial | grava com nota fixa "salvo", sem campo de comentário (`app.rs:827`, botão em `:5263`) |
| Salvar tudo / plano de salvamento | falta | falta | falta | falta | falta | falta | falta | B56 |
| Salvar como / cópia / empacotar | falta | falta | falta | falta | falta | falta | falta | só "Duplicar" (`lib.rs:259`); B93 |
| Histórico de versões | parcial | falta | parcial | falta | falta | falta | parcial | lista as 30 últimas e abre; sem restaurar, sem nome, sem comparar (`app.rs:5310`) |
| Liberar revisão | parcial | falta | falta | parcial | parcial | falta | parcial | diálogo com comentário; letra automática; sem conferência pré-liberação (`lib.rs:240`, `app.rs:5287`) |
| Abrir revisão congelada | ok | falta | ok | parcial | parcial | falta | ok | "abrir 🔒" somente leitura (`app.rs:5303`) |
| Estados de ciclo de vida | parcial | falta | falta | falta | falta | falta | falta | três estados, troca livre (`lib.rs:253`) |
| Part number | parcial | falta | falta | falta | falta | falta | parcial | prefixo fixo; duplicado é barrado (`lib.rs:169`, `:188`) |
| Onde é usado | falta | falta | falta | falta | falta | falta | falta | B94 |
| Comparar versões | falta | falta | falta | falta | falta | falta | falta | B214 |
| Trava / check-in e check-out | falta | falta | falta | falta | falta | falta | falta | B25, B130 |
| Duplicar documento | ok | falta | falta | falta | falta | falta | parcial | cópia sem histórico, PN novo (`lib.rs:259`) |
| Importar STEP/BREP | falta | falta | falta | falta | falta | falta | falta | só no kernel; B88 |
| Exportar STEP / STL | parcial | falta | falta | falta | falta | falta | parcial | dois itens soltos, sem opções, aceita qualquer extensão (`app.rs:906-917`); B84 |
| Exportar estrutura (CSV) | parcial | falta | falta | falta | falta | falta | parcial | lista plana de documentos (`lib.rs:267`) |
| Recuperação automática | falta | falta | falta | falta | falta | falta | falta | B65 |
| Migração de formato do arquivo | falta | falta | falta | falta | falta | falta | falta | `format_version: 1` fixo (`lib.rs:143`); B64 |

Contagem (18 comandos x 7 critérios = 126 células): ok 5, parcial 21, falta 100.

## 3. Por comando: o que os outros têm e o Fenkra não

### 3.1 Salvar, Salvar tudo, Salvar como

**Nos CADs avançados.** O salvar é uma família, não um botão. Há o plano de salvamento, que lista cada
documento aberto com o motivo de estar na lista (novo, editado, dependência, só leitura, conflito) e
deixa escolher ação por linha antes de gravar [V, pesquisa anterior]. O "salvar como" tem modos: mover,
cópia que continua no original, cópia que passa a ser o aberto; e uma tabela de referências que o
usuário edita. O FreeCAD é o oposto: um documento é um único arquivo compactado e tudo está dentro; é
simples, mas não há noção de projeto compartilhado [V, wiki pública do FreeCAD].

**No Fenkra.** O Salvar de peça solta decide pelo caminho: sem caminho abre diálogo, com caminho grava
em cima (`app.rs:835-856`). O Salvar de projeto cria versão por hash (`lib.rs:205-227`) e só gera nova
linha se o conteúdo mudou, o que é bom (sem lixo no histórico). Faltam:

- **Prévia do que será gravado**: lista de arquivos, tamanho, "o que mudou desde a última versão".
- **Comentário na versão**: hoje o Salvar de projeto grava com a nota fixa "salvo" (`app.rs:827`); um campo opcional curto,
  com sugestão automática ("ajustei furo Ø8"), aproveitaria a futura comparação (B214).
- **Ícone e tooltip**: o botão tem emoji, não ícone do conjunto; tooltip sem atalho nem frase.
- **Escrita atômica**: gravar em `.tmp`, `fsync`, `rename`, com `.bak`, como o índice já faz.

**Como deveria ser.** Um grupo Salvar / Salvar tudo / Salvar como igual em todas as bancadas. Ctrl+S
sem diálogo quando o documento já tem lugar; Ctrl+Shift+S abre o plano com uma coluna "motivo" e
"o que vai acontecer". O Salvar como pergunta o modo em linguagem de projeto ("Mover", "Cópia e
continuar aqui", "Cópia e abrir"), mostra as referências afetadas e só então grava. Já planejado em B56,
B65 e B93; este detalhe acrescenta a prévia por linha e o comentário de versão.

### 3.2 Histórico e comparação de versões

**Nos CADs avançados.** O histórico é uma tabela com versão, data, autor, comentário e ação (abrir,
restaurar, comparar, obter). Comparar tem dois níveis: de metadados (quem mudou, quando) e geométrico
(sobreposição colorida de antes e depois, com lista das features alteradas) [V parcial]. Os gerenciadores
guardam uma linha de auditoria por mudança de estado.

**No Fenkra.** `Version { hash, time, note }` (`lib.rs:57-62`) não tem autor. A UI lista só 30
(`app.rs:5310`) e só abre. Não há restaurar, "nomear versão", filtro por revisão, nem comparar.

**Como deveria ser.** Linha do tempo com: rótulo editável, autor (da identidade do B215 quando houver,
senão nome local), comentário, ícone de revisão. Menu da linha: Abrir (somente leitura), Restaurar como
nova versão (nunca apaga), Comparar com a atual, Comparar com outra. A comparação (B214) mostra
"Furo: Ø8 → Ø10", "Filete removido", e sobreposição 3D. Detalhe novo: **restaurar nunca reescreve a
história**; cria versão nova apontando para o conteúdo antigo (barato, por causa do endereçamento por
hash).

### 3.3 Revisões e estados

**Nos CADs avançados.** Revisão é um esquema configurável (letras, números, letras mais números,
prefixo por tipo de documento). A transição entre estados é um grafo nomeado com permissão por papel,
motivo obrigatório, notificação e, em alguns, assinatura. Liberar pode disparar tarefas: gerar PDF,
STEP, DXF e BOM na hora da liberação [V parcial]. Há "congelar" antes de "liberar" no ciclo de
maturidade (privado, em trabalho, congelado, liberado, obsoleto) com transição em lote que respeita a
ordem filhos antes do pai [V, pesquisa anterior].

**No FreeCAD.** Não tem revisão nem estados no núcleo; existe só o campo de propriedades do documento e
add-ons de terceiros [C].

**No Fenkra.** O ciclo é `EmTrabalho -> Liberado -> Obsoleto` e o salvar volta de Liberado para
EmTrabalho sozinho (`lib.rs:221-223`), correto. Mas `set_state` aceita qualquer troca (`lib.rs:253`),
`Revision` não tem autor nem tipo, e a letra `next_label` (`lib.rs:117`) é fixa A, B, ..., AA.

**Como deveria ser.**

- Estados com **tabela de transições** em JSON (de, para, papel, exige motivo), com um modelo padrão de
  três estados e um modelo "com aprovação" opcional. Sem servidor: o papel é local até o B215.
- **Liberar em duas etapas**: lista de conferência na liberação (documento recalcula sem erro? referências
  resolvidas? PN definido? desenho atualizado?), e só então a revisão. Isso é o aceite natural do
  "checador de qualidade" já citado em B130.
- Esquema de revisão configurável (letras, números, "prévia" 0.1, 0.2 até a primeira liberação, comum
  em fábrica).
- Registro de auditoria só-acréscimo por transição (quem, quando, motivo).

### 3.4 Part number

**Nos CADs avançados.** O PN nasce de regra com campos (categoria, norma, material, serial). A prévia
mostra o número sem consumir o contador; a reserva acontece só ao gravar com sucesso. Cada esquema tem
no máximo uma sequência automática. O número é imutável depois da primeira liberação [V, pesquisa
anterior].

**No Fenkra.** `next_pn` consome o contador na hora (`lib.rs:169-173`), mesmo que o `add_document`
falhe em seguida; um PN consumido e não usado vira buraco. A prévia não existe.

**Como deveria ser.** B47 já descreve o motor de regras. Acrescento dois critérios de aceite de
interação: (a) campo de PN com **prévia ao vivo em cinza** ("FK-0042") enquanto se preenche categoria;
(b) tooltip do campo explicando cada máscara com exemplo. E um teste de falha: se `add_document` falha,
o contador não avança.

### 3.5 Onde é usado e estrutura de produto

**Nos CADs avançados.** Painel com duas abas, "Contém" e "Onde é usado", com estado de cada link,
filtro por revisão e opção de subir níveis até a montagem de topo. Antes de obsoletar ou liberar, o
gerenciador mostra o impacto. Há também "linha de base": retrato congelado da estrutura inteira numa
data [V parcial].

**No Fenkra.** Ausente. Os links entre documentos são caminhos de texto (pesquisa anterior; B66 troca
por uid).

**Como deveria ser.** O painel (B94) com a ação "Tornar obsoleto" mostrando, **antes de confirmar**, a
lista de quem usa a peça. Pela regra de manipulação direta: clicar numa linha seleciona o componente na
árvore da montagem. Cabeçalho lido sem abrir o documento inteiro, para ser rápido em projetos grandes.

### 3.6 Trava e multiusuário

**Nos CADs avançados.** Check-out trava o arquivo; quem chega depois abre somente leitura com banner
dizendo quem está com ele. Aviso de "alguém alterou o arquivo no disco".

**Decisão do Fenkra** (B130/B216): ramos e mesclagem por uid de feature, sem trava. Isso é coerente
com o formato (JSON textual, features com uid). O ponto de interação a detalhar: **o banner de estado**
(somente leitura, desatualizado, em conflito) precisa existir já na fase local, com a mesma linguagem,
para o usuário aprender uma vez.

### 3.7 Duplicar e Copiar projeto

`duplicate` (`lib.rs:259-265`) cria novo ID e novo PN e perde o histórico, o que é o esperado. Faltam
o aviso de que as referências dentro da cópia ainda apontam para os originais, e a opção "copiar
projeto inteiro com renomeação em lote" (prefixo, sufixo, tabela editável), que é o Salvar como do B93.

### 3.8 Exportar e Importar (interface)

**Nos CADs avançados.** O Arquivo ▸ Abrir aceita STEP e abre um diálogo de opções, com relatório depois
(o que entrou, o que foi reparado, o que falhou). A exportação tem uma janela de opções por formato
(esquema do STEP, unidades, o que levar: cores, camadas, PMI, estrutura). O FreeCAD tem Importar e
Exportar genéricos que escolhem o filtro pela extensão, e as opções ficam nas preferências [V].

**No Fenkra.** Dois itens soltos (`app.rs:2564-2571`), sem opção alguma; a extensão não é validada
(`app.rs:906-917`, B84). A importação não existe na interface.

**Como deveria ser.** Arquivo ▸ Importar… e Arquivo ▸ Exportar… separados (B88), diálogo único com a
ordem suporte → modo → parâmetros → avançado. Pontos que ainda não estão escritos nos itens:

- **Prévia de importação**: antes de aceitar, mostrar a miniatura 3D, a caixa envolvente em mm, a
  contagem de corpos e faces e o aviso de unidade suspeita ("a peça tem 25 400 mm; era polegada?").
- **Relatório copiável** após importar, com contagem de faces reparadas, sólidos que viraram cascas
  (não fechou), e o que o formato não carrega.
- **Selo de origem** gravado no documento (nome do arquivo, hash do original, data, esquema do STEP),
  e o original guardado no cofre por hash.

## 4. Migração de projetos de outros CADs

### 4.0 O que a migração precisa entregar (e o que não promete)

Princípio de honestidade para a interface: **a geometria exata atravessa; a intenção de projeto só
atravessa se alguém a exportar de propósito.** O STEP comum leva o corpo (superfícies, topologia),
não a receita. Fontes acadêmicas e de mercado dizem o mesmo: o STEP de B-rep não traduz informação
paramétrica e o reparo de geometria não recupera histórico [V]. Existe um capítulo da norma para
restrições e parâmetros explícitos (ISO 10303-108) e outro para histórico procedimental (ISO 10303-55),
mas a primeira não cobre a árvore de construção e, pelo que li, as implementações de mercado raramente
o usam [V parcial; não confirmei suporte nas ferramentas]. Daí a divisão em três degraus de
fidelidade:

| Degrau | O que o usuário ganha | Como |
|---|---|---|
| 1. Geometria | Corpo exato, cores, nomes, estrutura de montagem | STEP via XDE (B87, B88) |
| 2. Features inferidas | Furos, filetes, bolsos reconhecidos; edição parcial | reconhecimento (B143) e edição direta (B33) |
| 3. Receita original | Esboços cotados + sequência + parâmetros | exportador por API do CAD de origem (caminho b) ou reconstrução por agente (caminho c) |

### 4.1 Caminho (a): STEP com reconhecimento de features (B143) e edição direta (B33)

**Como funciona.** Importa o STEP com cores e estrutura. Depois classifica as faces e monta um grafo de
adjacência com atributos: nós são faces, arestas do grafo são as arestas compartilhadas, e cada
aresta carrega a convexidade (côncava, convexa, tangente). Padrões do grafo viram features: cilindro
côncavo fechado por uma base é furo cego; cilindro que atravessa duas faces planas é furo passante;
cone coaxial é escareado; toroide ou cilindro tangente entre duas faces é filete; um laço de faces
côncavas sobre uma base plana é bolso. Essa é a técnica clássica (grafo de adjacência com atributos) e há
implementações abertas sobre o OpenCASCADE, em especial o AnalysisSitus (BSD-3, segundo página de terceiros;
**confirmar a licença no repositório antes de adotar** [V parcial]) e um pacote de Python que separa
reconhecimento de edição e se limita a superfícies analíticas (plano, cilindro, cone), declarando que
B-splines ficam fora do que foi validado [V].

**Em outros CADs.** O reconhecimento dos CADs avançados tem modos automático, interativo (o usuário
aponta as faces) e passo a passo (reconhece, salva, retoma), com uma lista de tipos suportados (furos de
vários perfis, filetes, chanfros, extrusões) e uma opção de apagar ou "engrossar" o que não foi
reconhecido [V, nota da pesquisa anterior]. Os limites declarados: padrões de padrão e marcas parciais
não saem. Isso calibra a expectativa: **nenhuma ferramenta devolve a árvore original**; todas devolvem
uma árvore plausível.

**No FreeCAD.** A importação de STEP traz o corpo como forma sem histórico; edição depende do módulo
de peça e de ferramentas geométricas, e não encontrei reconhecimento de features nativo na
documentação consultada [V parcial].

**Fidelidade esperada.** Furo, filete de raio constante, chanfro, escareado, rasgo: alta (comparável ao
estado da arte) quando as superfícies são analíticas. Extrusão e revolução: média, pois o esboço é
deduzido e as cotas ficam "as medidas que couberam", sem a lógica de projeto (um furo a 20 mm da borda
vira cota de 20 mm, não "centro da peça"). Filete variável, superfícies livres, loft: baixa; ficam como
corpo. Chapa: média se o corpo tiver espessura constante e dobras cilíndricas; desdobrar por
reconhecimento é pesquisa em aberto.

**Riscos.** (1) Falsos positivos que mudam a geometria ao reconstruir: mitigar pela regra "toda feature
reconhecida precisa reproduzir o corpo em volume, área e centroide dentro de tolerância, senão volta
para corpo". (2) Tempo em peças com milhares de faces. (3) Expectativa do usuário: a tela deve dizer
"estrutura inferida", não "histórico restaurado".

**Custo.** Fase 1 (analisar e marcar, sem reescrever): 300 k tokens no B143, sobre o B87. Fase 2
(converter a pedido com verificação): 300 k adicionais. Edição direta de faces (B33) é pré-requisito
para o corpo importado ser útil mesmo sem reconhecimento: mover/apagar face, deslocar face, trocar raio.

**MVP.** Importar STEP (B88) → "Analisar" gera um painel "Features encontradas" (furos com Ø e
profundidade, filetes com raio, planos) que seleciona no 3D ao clicar, sem alterar o corpo → botão
"Converter em feature" para furo passante e cego e filete de raio constante, com verificação de volume.
Aceite: no corpus de peças próprio, exportar para STEP e reimportar reconhece ao menos 90 % dos furos e
filetes de raio constante, e nenhum caso muda o volume mais que 1e-6 relativo.

### 4.2 Caminho (b): exportador por script na API pública do CAD de origem

**A ideia.** Um script (macro, suplemento ou programa externo por COM) que o próprio usuário roda
no CAD dele, com a licença que ele já paga, percorre a árvore e grava um arquivo aberto: esboços com
geometria e restrições, cotas, features com parâmetros, nomes, cores, estrutura de montagem. O Fenkra
lê esse arquivo (formato novo e documentado, ou JSON) e remonta a árvore **com histórico real**. É a
única rota, além da reconstrução por agente, que preserva a receita.

**O que as APIs públicas expõem (leitura de documentação pública, sem afirmar além).**

- Um CAD avançado de mercado médio tem API por COM com modelo de objetos documentado: documento,
  features, esboços, dimensões, parâmetros. A documentação traz exemplos oficiais; o texto da licença
  dos exemplos concede uso livre de royalty a usuários licenciados para criar aplicativos relacionados
  ao produto, e a mesma documentação diz que só as interfaces documentadas têm suporte [V, página de
  ajuda de versão antiga]. Um trecho citado em fórum diz que **distribuir** aplicativo feito com essa
  API a terceiros exige o programa de parceiros do fabricante [V parcial: citação informal, verificar no
  contrato vigente].
- Outro CAD de mecânica tem modelo de objetos documentado por versão, com coleções de features e de
  dimensões; a própria ajuda observa que as dimensões dos esboços ficam separadas das dimensões da
  feature e precisam ser consultadas à parte [V]. Não li termos de uso.
- Um CAD de alto nível tem API de automação por COM, e há uma biblioteca aberta de terceiros que a envolve
  em Python; relatos dizem que a automação básica pede só a licença base, mas **isso vem de blogs, não do
  contrato** [V parcial]. O modelo de objetos de automação não cobre tudo.
- Um CAD em nuvem expõe pela REST a lista de features de um estúdio em JSON, e a documentação recomenda
  exatamente "crie a feature na interface, leia a lista, e reaproveite a estrutura"; avisa que parâmetros
  podem mudar sem aviso e que os tipos aceitos não estão todos documentados [V]. Um conjunto público de
  pesquisa (centenas de milhares de modelos) foi montado recuperando esboço e extrusão por script nessa
  linguagem, o que prova a viabilidade técnica para a classe esboço+extrusão [V].
- Um conjunto público de sequências de esboço e extrusão de um fabricante tem licença de uso só
  não comercial [V]. **Não usar como dado de treino ou teste do produto.**
- Nenhuma das fontes que li fala em exportar a árvore para JSON; seria código escrito por nós percorrendo o
  modelo de objetos [V: ausência confirmada na busca, não nos produtos].

**Fidelidade esperada.** Esboço+extrusão/revolução/furo/filete/chanfro/padrão: alta, com cotas e nomes.
Features avançadas (varredura, loft, superfícies, chapa): média; a API expõe, mas o mapeamento para os
nossos comandos precisa de um por um. Referências por nome persistente: perdem-se, pois as faces do CAD de
origem não têm correspondência com as nossas; a remontagem precisa **reancorar por geometria** (ADR 0010).

**Riscos.** (1) Termos de licença: o script roda **na máquina do usuário** e escreve **arquivo dele**;
o risco jurídico cresce se distribuirmos binário que ligue à API. Mitigação: entregar o script como
código-fonte aberto que a pessoa lê e roda, sem binário de terceiros, e consultar o contrato vigente de
cada fabricante antes de publicar (ADR). (2) Manutenção: cada versão do CAD de origem muda a API; sem
verificar, quebra. (3) Não é engenharia reversa do formato nativo, o que a regra do projeto exige; só
documentação pública e API oficial. (4) Cobertura: um script por CAD de origem, um custo recorrente.

**Custo.** Esquema do arquivo de intercâmbio (JSON versionado, validável): 150 k. Leitor no Fenkra que
cria a árvore, reancora referências e valida volume: 400 k. Cada script de origem: 150–250 k, em ordem
de uso (decidir pela demanda do público-alvo, não por tamanho de mercado).

**MVP.** Esquema v0 só com esboço (linhas, arcos, círculos, restrições, cotas) + extrusão de adição e
corte + furo + filete. Um script de origem. Teste: peças do corpus montadas dos dois lados, comparando
volume, área e centroide após o recálculo (tolerância 1e-6 relativo), e a contagem de graus de liberdade
do esboço restaurado.

### 4.3 Caminho (c): IA por MCP

**A ideia.** Um servidor MCP do Fenkra expõe ferramentas (criar esboço, extrudar, furar, ler parâmetros,
recalcular, medir) e recursos (documento atual, árvore, relatório de importação). Um agente lê o STEP
(por ferramentas de consulta: lista de faces, cilindros, planos, caixa envolvente) e os desenhos
(imagem ou DXF), propõe a sequência de comandos e a **executa**; um checador compara o resultado com o
alvo e devolve o erro para a próxima tentativa.

**Estado da técnica (público).** O MCP tem três primitivas: ferramentas (o modelo decide chamar),
recursos (a aplicação decide expor) e prompts (o usuário escolhe) sobre JSON-RPC [V, fontes de terceiros
sobre a especificação; confirmar a versão corrente no site oficial]. Já existem servidores MCP
comunitários para geração por código em CadQuery (com saída de imagem e mensagens de erro para o modelo
corrigir) e para listar features de um CAD em nuvem [V]. Na pesquisa, um método de verificação em que o
modelo de visão gera perguntas de validação sobre a imagem do objeto e corrige o código melhora a taxa de
sucesso em poucos pontos percentuais [V]. Existem também modelos treinados para gerar código CadQuery a
partir de imagem e de nuvem de pontos [V]. Importante: **os ganhos publicados são modestos e os
conjuntos de teste são de peças simples**; a distribuição de modelos de treino é enviesada para poucas
operações [V]. Não se deve prometer reconstrução de peças complexas.

**Fidelidade esperada.** Peças prismáticas e de revolução com furos e filetes: boa, e o resultado tem
cotas legíveis e editáveis. Peças orgânicas, montagens grandes: baixa. A grande vantagem é que o
resultado **é a nossa árvore**, com parâmetros nomeados, e pode ser verificado automaticamente.

**O checador é o produto.** Sem ele a IA é um gerador de ruído. O checador compara: volume, área,
centroide, caixa envolvente, momentos de inércia, contagem de furos por diâmetro e distância de
Hausdorff amostrada entre superfícies. Aprova abaixo de uma tolerância e reporta *onde* diverge (mapa de
cor no 3D). O mesmo checador serve ao caminho (a) e ao (b): é uma peça compartilhada.

**Riscos.** Privacidade (STEP do cliente indo a um modelo remoto: deve funcionar com modelo local e
nunca enviar sem aceite explícito); custo em tokens por peça; não determinismo (guardar a sequência de
comandos gerada como o artefato, não a conversa); segurança (o servidor só expõe comandos do Fenkra, nunca
execução de código arbitrário nem acesso a arquivos fora do projeto).

**Custo.** Servidor MCP mínimo sobre a CLI do B39: 250 k. Checador de similaridade: 200 k. Corpus de
avaliação próprio (peças do B53 exportadas): incluído no B53.

**MVP.** Servidor com 8 ferramentas (consultar STEP; criar esboço com cotas; extrudar; furar; filetar;
recalcular; medir; comparar com o alvo). Aceite: o agente reconstrói 5 peças prismáticas do corpus a
partir do STEP em até 10 iterações, com volume dentro de 0,5 % e todos os furos no diâmetro certo; o
checador roda sem rede.

### 4.4 Comparação e ordem recomendada

| Critério | (a) STEP + reconhecimento | (b) Exportador por API | (c) IA por MCP |
|---|---|---|---|
| Preserva a receita original | não (infere) | sim | não (reinventa) |
| Exige o CAD de origem instalado | não | sim | não |
| Cobertura de CADs | qualquer um que exporte STEP | um script por CAD | qualquer um que exporte STEP |
| Custo recorrente | baixo | alto (manutenção por versão) | médio (tokens) |
| Risco jurídico | baixo | médio, ler contrato de cada API | baixo (privacidade é o risco) |
| Fidelidade em peças simples | alta | alta | média a alta |
| Fidelidade em peças complexas | média | média a alta | baixa |
| Já no backlog | B87, B88, B143, B33 | não | B39 como base; sem MCP |

**Ordem.** (1) B87 e B88 primeiro: sem importar STEP não há migração. (2) O **selo de origem e o
relatório** (novo). (3) B33 mais a fase 1 do B143: corpo editável e marcado. (4) O checador de
similaridade, que serve a todos. (5) O esquema aberto de árvore e o leitor (b), com um primeiro script,
porque é o único caminho que entrega parametria real. (6) O servidor MCP, depois do B39 estável.

**Um ponto de decisão** para a pessoa responsável: o esquema de intercâmbio do caminho (b) deve ser
documentado como formato aberto do Fenkra, **ou** reaproveitar um formato existente de comandos de
modelagem? Um formato próprio dá controle, mas é mais uma coisa a manter. Sugiro o próprio, pequeno,
versionado pelo mesmo mecanismo do B64, e que o Fenkra também **exporte** nele, assim vira formato de
troca e de teste.

## 5. Interações que valem para todo o conjunto

- **Todo comando de gestão mostra o que vai acontecer antes de gravar** (prévia como lista, não como
  janela de confirmação genérica).
- **Tooltip em dois estágios** e ícone do conjunto (B60) em Salvar versão, Liberar revisão, Importar,
  Exportar, Onde é usado, Comparar. Hoje os botões do painel usam texto e emoji (`app.rs:5254-5266`).
- **Dica por campo** em PN, estado, comentário de revisão e opções de importação: uma linha cinza com
  exemplo.
- **Estado sempre visível** na barra: "Rev B liberada", "Editado desde a Rev B", "Somente leitura".
  O cálculo existe (`dirty_since_release`, `lib.rs:88`); a barra não o exibe fora do painel.
- **Desfazer** para ações de gestão reversíveis (obsoletar, duplicar) e confirmação só nas que não são.
