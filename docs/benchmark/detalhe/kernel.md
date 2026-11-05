# Benchmark de detalhe: peso do Fenkra e execução em PC fraco (bancada Kernel)

Base: commit `cbfa7f0`, versão 0.1.0-alpha.28. As linhas citadas em `kernel.cpp` são as desse commit,
porque o arquivo está em edição. O ponto das normais de exibição ficou fora deste estudo.

## Resumo

O peso do Fenkra não vem de um único defeito. São quatro problemas diferentes, e só o primeiro é
"muita conta":

1. **A tesselação faz trabalho desnecessário e em ordem ruim.** Três coisas se somam: um adaptador de
   superfície criado por aresta, que varre a face inteira a cada vez; a limpeza da malha em toda
   chamada, que joga fora o que estava pronto; e o uso de todos os núcleos em cada recálculo. Numa
   engrenagem de 40 dentes, 80% dos 444 ms da tesselação vão para decidir se as arestas são suaves, e
   só 34 ms são malha de verdade.
2. **A memória cresce sem limite na sessão.** O cache de malhas nunca descarta versão antiga. Medido:
   300 edições de uma peça simples levaram o processo de 31 MB a 160 MB.
3. **A interface repinta sem precisar em dois lugares.** A tela inicial pede quadro novo sempre, e
   qualquer painel com abas pede 20 quadros por segundo. Isso está no código, mas o consumo não foi
   medido, porque o aplicativo não foi executado neste estudo.
4. **O aplicativo pode falhar sem dizer nada.** O pacote de release não traz o runtime C++ que 34 dos
   seus módulos importam. Além disso, a versão final não tem console nem gancho de pânico, então um
   erro de GPU fecha o programa em silêncio.

A causa provável do colega que não conseguiu rodar está no item 4, não no peso. Isso é hipótese: não
temos o relato do erro nem a máquina dele. Antes de otimizar para esse caso, vale perguntar o que
apareceu na tela dele e se o Visual C++ Redistributable estava instalado.

Uma suspeita antiga caiu: clonar o documento a cada quadro (B68) custa pouco. O clone de um documento
de 300 comandos levou 0,23 ms. O B68 continua valendo pelo desfazer nomeado, mas não é o que pesa.

As boas notícias: o recompute já roda fora da interface, já tem cache por comando e já junta pedidos
em fila; a viewport não redesenha quando nada mudou; o release já tem backend OpenGL disponível. A base
é boa e os consertos são pequenos perto do ganho.

## Como foi medido

Só foi usado `cargo test` em modo release, sem editar código do projeto. Para não
colidir com a edição em andamento de `kernel.cpp` nem com a pasta de build compartilhada, as medidas
rodaram em uma cópia do commit, com um crate descartável de testes e pasta de build próprios, na área
temporária da sessão. Esse crate usa as funções públicas do Fenkra e, para comparar variantes, chama o
OpenCASCADE 8.0.1 direto em C++.

Limites que precisam ficar claros:

- Máquina única, de 16 processadores lógicos. **Nenhum número vem de um PC fraco.** As proporções devem
  se manter, os valores absolutos não.
- O aplicativo gráfico **não foi executado**. Não há medida de CPU em repouso, de tempo de partida, de
  memória do app inteiro nem de GPU. Tudo isso aparece abaixo como hipótese ou como leitura de código.
- As peças de teste são sintéticas: placa com grade de furos filetados, placa com grade de furos, a
  peça de exemplo e a engrenagem gerada pela biblioteca. Placas com muitos furos forçam faces planas
  com muitos contornos, que é o pior caso do malhador.
- Duas medidas de `scene::build` e `picking::pick` são reproduções do laço em um teste, porque esses
  módulos são privados do binário. Servem como ordem de grandeza.
- As variantes por etapa leem a forma exportada em STEP. A soma das etapas bate com o tempo real da
  `tessellate` (34 + 350 + resto, contra 444 ms na engrenagem).

## O caminho quente, passo a passo

**Recompute.** Roda na thread `recompute` (`crates/fenkra-app/src/app.rs:68-96`), que mantém um
`Cache` entre cálculos e descarta pedidos intermediários (`app.rs:81-84`). A cada pedido, o laço de
`recompute_cached` (`crates/fenkra-solidos/src/lib.rs:4214`) passa por todos os comandos e calcula uma
assinatura com o JSON do comando, os parâmetros citados e a assinatura do que ele lê. Assinatura igual
reaproveita o resultado (`lib.rs:4332`). Medido: nada mudou, 0,05 ms na placa de exemplo e 0,95 ms na
engrenagem. Esse desenho está certo e não precisa de recompute incremental novo.

O que falta é cancelar. O kernel não usa `Message_ProgressRange` em nenhuma operação, então um cálculo
velho em andamento vai até o fim, mesmo que o usuário já tenha mudado o valor de novo.

**Tesselação.** Também na thread `recompute`, na etapa final (`lib.rs:4521-4535`): só refaz o corpo cuja
assinatura mudou, e sempre a forma inteira. `tessellate` (`crates/fenkra-kernel/cpp/kernel.cpp:829`)
chama `BRepTools::Clean` (linha 834) e depois `BRepMesh_IncrementalMesh` em paralelo (linha 835). Ou
seja, a resposta às perguntas do brief é: Clean em toda chamada, sim; malha por forma inteira, sim, não
por face alterada; fora da thread principal, sim no modo peça. Na montagem, não (ver abaixo).

Comandos também tesselam no meio do recálculo (`lib.rs:1661` e `lib.rs:1942`), pagando o mesmo custo.

**Cena e GPU.** Na thread da interface, quando `scene_dirty` está ligado (`app.rs:6914-6937`),
`scene::build` duplica vértices por face para colorir (`crates/fenkra-app/src/scene.rs:98-115`) e
`set_scene` cria buffers novos (`crates/fenkra-app/src/render.rs:797`). Reprodução do laço: 32 ms para
390 mil triângulos. O vértice tem 56 bytes (`render.rs:326-333`), cerca de 25 MB de GPU nesse caso. É um
soluço curto, não uma causa de peso. Seleção e hover também não entram na lista: a varredura de todos
os triângulos a cada movimento (`crates/fenkra-app/src/picking.rs:194`) custou 2,7 ms com 390 mil.

**Repintura.** `render()` só desenha se câmera, tamanho, cena ou estilo mudaram (`render.rs:879-881`),
então a GPU descansa em repouso. Sem nada acontecendo, a peça aberta fica parada. As exceções são a
tela inicial (`app.rs:1142`, incondicional) e a barra de abas dos painéis (`editor.rs:286`,
`request_repaint_after` de 50 ms, em todo painel com abas aberto). A abertura (`app.rs:1002`), os
avisos (`app.rs:5953`) e a varredura da árvore (`app.rs:6488`) são temporários e estão corretos.

Cada quadro que roda clona o documento duas vezes (`app.rs:6669` e `app.rs:3352`) e compara com o
último enviado (`app.rs:6828`). Medido com 300 comandos: clone 0,23 ms, igualdade 0,015 ms. Fica abaixo
de 1 ms por quadro. A meta do B68 (menos de 2 ms) já é atendida com documentos desse tipo; sketches
muito grandes podem mudar isso.

**Memória.** Uma mesma malha vive em vários lugares: triangulação do OpenCASCADE dentro da forma, cópia
no cache, cópia no resultado e buffer de GPU. As cópias são baratas de fazer (2,4 ms para 390 mil
triângulos) e aceitáveis de guardar. O problema é o acúmulo por versão, descrito na tabela. O desfazer
também não tem limite (`app.rs:151`), já previsto no B68. Na GPU, o alvo com 4 amostras em 1920x1080
ocupa até cerca de 74 MB entre cor, profundidade e resolução, mais 8 MB do array de texturas
(`render.rs:382-383`).

**Partida.** `fenkra.exe` tem 32 MB; o pacote tem 74 MB, 33 DLLs e 28 MB compactado. O executável
importa 18 módulos TK diretamente, e o fechamento chega a 27. `TKV3d`, `TKService` e `freetype` entram
só porque a leitura de STEP puxa a camada XCAF; carregá-los custa pouco e o ganho de removê-los é
incerto. O tempo de carga não foi medido.

## Causas

| # | Causa | Evidência (arquivo:linha ou medida) | Comprovada ou hipótese | Ganho esperado | Custo |
|---|---|---|---|---|---|
| 1 | Adaptador de superfície criado por aresta. `BRepAdaptor_Surface` calcula os limites da face varrendo as arestas dela, e isso roda duas vezes por aresta, então o custo cresce com o quadrado em faces com muitos contornos | `kernel.cpp:913`. Etapa de suavidade: engrenagem 350 ms de 444 ms; placa 20x20, 866 ms; placa 40x40, 13,5 s. Sem projeção mas com adaptador: 297 ms (logo, o custo é do adaptador). Sem adaptador: 1,7 ms e 2,4 ms | Comprovada | Engrenagem cerca de 5 vezes mais rápida na tesselação; troca de qualidade cai de 540 ms para menos de 200 ms | 60 k |
| 2 | `BRepTools::Clean` em toda chamada descarta faces já malhadas | `kernel.cpp:834`. Placa 20x20, edição numa face lateral: 1.034 ms com Clean, 84 ms sem (CPU 3.609 contra 562 ms). Edição que toca as faces grandes: 1.126 contra 951 ms | Comprovada; o ganho depende de onde a edição cai | 1,2 a 12 vezes no tempo de malha por edição; CPU até 6 vezes menor | 80 k |
| 3 | Cache de malhas sem descarte: a chave leva a assinatura, então cada versão vira uma entrada nova; só a troca de qualidade limpa | `lib.rs:522`, `lib.rs:4221-4226`, `lib.rs:4530`; `fills` igual (`lib.rs:4505`). 300 edições: 30,8 para 159,7 MB; descartando o cache, 31,8 MB | Comprovada | Memória estável; evita troca em disco em sessões longas | 30 k |
| 4 | Pacote sem `vcruntime140.dll`, `vcruntime140_1.dll` e `msvcp140.dll` | Tabela de importação do pacote alpha.28: 34, 28 e 28 módulos dependem delas e nenhuma está na pasta. `scripts/release.ps1:50-55` só procura na pasta do vcpkg | Falta comprovada; ser a causa do colega é hipótese | Abre em PC sem o Redistributable | 20 k |
| 5 | Falha de GPU ou pânico termina sem mensagem | `crates/fenkra-app/src/main.rs:2` (sem console) e `main.rs:65` (só devolve o erro); nenhum `set_hook` no projeto; nenhum registro do adaptador escolhido | Comprovada por leitura; não reproduzida | Erro vira mensagem acionável | 100 k |
| 6 | Tela inicial repinta sempre; painel com abas repinta a 20 Hz | `app.rs:1142`; `crates/fenkra-app/src/editor.rs:286`. Consumo não medido | Código comprovado; impacto hipótese | Provavelmente a maior queda de CPU em repouso | 40 k |
| 7 | Malhador padrão lento em face plana com muitos contornos, e uma face só não se divide entre núcleos | Placa 20x20: 2.032 ms, Delabella 237 ms, mesmos 97.612 triângulos. De 97 mil para 390 mil triângulos, o tempo foi de 2 s para 19 s. Engrenagem: 34 contra 23 ms, mas 1.916 triângulos contra 4.224 | Velocidade comprovada; qualidade e robustez em aberto | Até 8 vezes na malha nesses casos | 90 k |
| 8 | Todo recálculo usa todos os núcleos com retorno baixo | `kernel.cpp:835`. Placa 20x20: sequencial 2.400 ms; 4 fios 1.163 ms e 2.641 ms de CPU; 16 fios 1.049 ms e 3.875 ms de CPU | Comprovada | Com 4 fios: 11% mais lento e 32% menos CPU | 40 k |
| 9 | Adaptador de software ou MSAA 4x em GPU fraca. O padrão é 4 amostras e preferência de alto desempenho | `crates/fenkra-app/src/settings.rs:301`; `egui-wgpu` `setup.rs:245-256` | Hipótese | Grande se ocorrer: render por CPU explicaria uso alto | 150 k (modo leve) |
| 10 | Limites exigidos do dispositivo podem barrar adaptador antigo, sem nova tentativa | `main.rs:44-49` força textura de 8192 mesmo com a base de OpenGL | Hipótese | Abre em mais máquinas | 30 k |
| 11 | Trabalho pesado na thread da interface na montagem e no desenho | Só existem as threads `recompute` e a de `examples.rs:332`. `view::compose` em `asm_ui.rs:299`, solver em `asm_ui.rs:310`, `models.refresh()` em `dwg_ui.rs:477`. `tesselar` na interface em `crates/fenkra-montagem/src/view.rs:126` | Local comprovado; impacto hipótese | Interface não congela | 350 k |
| 12 | Sem cancelamento de cálculo em andamento | Nenhum `Message_ProgressRange` em `crates/fenkra-kernel/cpp/` | Comprovada | Arrastar cota responde melhor em PC fraco | Junto do 11 |
| 13 | Massa exata refeita junto da malha, inclusive só ao trocar qualidade | `lib.rs:4529`. Engrenagem 65 ms; placa 20x20 filetada 145 ms | Comprovada | 10% a 15% por troca de qualidade | 10 k |
| 14 | Clonar o documento por quadro | 0,23 ms com 300 comandos | Descartada como causa principal | Abaixo de 1 ms por quadro | Fica no B68 |

Sobre a causa 2, há um cuidado. O comentário em `kernel.cpp:833` explica por que o Clean existe: o
malhador não afina nem engrossa uma malha pronta quando a qualidade muda. Portanto o Clean deve sair do
caminho comum e ficar só na troca de qualidade. A documentação do malhador confirma que ele reaproveita
o que atende aos parâmetros e limpa só o que falha na conferência, e que `AllowQualityDecrease` controla
a piora de qualidade. Testei uma variação de 0,1% no desvio, como acontece quando a caixa envolvente
muda um pouco: nada foi refeito (83 ms). É o comportamento desejado, mas o tamanho da mudança que dispara
a remalha ainda precisa ser levantado. Sem o Clean, também há um piso de cerca de 84 ms nessa peça, que é
a conferência de todas as faces.

Outro cuidado: o Clean altera dados compartilhados entre versões da forma, e outras chamadas de malha
rodam na thread da interface (`crates/fenkra-app/src/pmi_ui.rs:173`). Não encontrei corrida real, mas a
documentação pública não garante uso concorrente da mesma topologia. Tirar o Clean reduz esse risco.

## Plano em ordem de retorno

Ordem pelo ganho sobre o custo. Os primeiros cinco passos somam cerca de 250 k e resolvem o que está
comprovado.

1. **Parar de criar adaptador por aresta (60 k).** Guardar um adaptador por face, ou ler `(u, v)` pela
   curva da aresta na face e usar `Geom_Surface` direto. Aceite: engrenagem de 40 dentes com a etapa de
   suavidade abaixo de 5 ms, com o mesmo `edge_smooth` da versão atual no corpus.
2. **Colocar limite no cache de malhas (30 k).** Manter só as assinaturas vivas no último cálculo, como
   já se faz com `feats` e `refs`. Aceite: 300 edições sem crescimento além de 10%.
3. **Incluir o runtime no pacote (20 k).** Copiar as três DLLs em `scripts/release.ps1` agora. O caminho
   recomendado pelo fabricante é o instalador do Redistributable, que recebe atualizações, então o
   instalador futuro deve preferir ele. Aceite: abre em Windows limpo.
4. **Mensagem de erro na partida (100 k).** Ver a seção de diagnóstico.
5. **Parar a repintura em repouso (40 k).** Tela inicial só repinta com animação ativa; a barra de abas
   só agenda quadro enquanto algo anima. Antes de mexer, medir com o contador de quadros para ter o
   número que falta neste estudo. Aceite: zero quadros em 5 s parado.
6. **Clean só na troca de qualidade (80 k).** A fachada recebe a informação de que a qualidade mudou.
   Aceite: edição lateral na placa 20x20 abaixo de 150 ms, sem furos nem emendas na malha.
7. **Limitar fios e tirar massa da troca de qualidade (50 k).** Teto de fios configurável pelo pool de
   threads do OpenCASCADE, padrão de metade dos processadores lógicos, com mínimo de 2. O pool só aceita
   mudança sem tarefa ativa, então o ajuste deve ocorrer entre cálculos.
8. **Avaliar o Delabella (90 k).** Rodar no corpus do B53 comparando desvio real, volume e aparência. A
   queda de triângulos na engrenagem pode ser defeito ou ganho, e é preciso saber qual antes de adotar.
9. **Modo leve e perfil de hardware (150 k).** Ver a seção própria.
10. **Tirar montagem e desenho da thread da interface, com cancelamento (350 k).** É o passo mais caro e
    depende das bancadas de Montagem e Desenho.

Fora de escopo agora: LOD por distância e carga parcial (B125) só fazem sentido com montagens grandes, e
hoje o gargalo está antes disso. Também não precisa de recompute novo.

## Como projetos abertos lidam com isso

O FreeCAD expõe desvio linear relativo à caixa envolvente e desvio angular, como o Fenkra já faz com o
B61, e tem desvio por objeto. Sua história mostra dois erros a evitar: valores muito pequenos travaram o
programa pelo excesso de triângulos, e misturar malha de exibição com malha de exportação gerou bug. Não
achei fonte pública sobre malha em segundo plano nele. Softwares de modelagem CAD avançados usam malha
mais grossa em documentos grandes para reduzir o tempo de carga, o que reforça o caminho do B125.

No OpenCASCADE, a documentação diz que arestas e faces são processadas em separado, o que explica o
paralelismo por face, e o fórum do projeto recomenda o algoritmo Delabella quando a triangulação domina
o tempo. O pool de threads usa por padrão todos os processadores lógicos e aceita limite.

No egui, o modo reativo é o padrão e só redesenha sob evento; chamar `request_repaint` em todo quadro
leva ao ritmo do monitor. Há relatos públicos de janela quase vazia com uso relevante de CPU nesse modo
contínuo, que é o motivo de tratar a causa 6 antes de medir mais.

No wgpu, Dx12, Vulkan e Metal são a camada principal e OpenGL é a secundária. `WGPU_BACKEND` permite
forçar a escolha. Há projeto que relata perda de dispositivo Dx12 ao reconectar área de trabalho remota.
Não achei documentação que confirme como o renderizador de software do Windows é reportado; `get_info()`
e o tipo `Cpu` resolvem isso na prática, e essa checagem deve entrar no diagnóstico.

## Requisitos mínimos sugeridos

Proposta inicial, **não testada em máquina real**. Precisa de um teste em PC fraco de verdade antes de
ser publicada. O FreeCAD, por comparação, só exige OpenGL funcional e driver atualizado.

| Item | Mínimo | Recomendado |
|---|---|---|
| Sistema | Windows 10 64 bits | Windows 11 64 bits |
| Processador | x64, 2 núcleos e 4 fios | 4 núcleos e 8 fios ou mais |
| Memória | 4 GB (peças simples, modo leve) | 16 GB |
| GPU | Dx12 ou Vulkan, ou OpenGL 3.3 (a confirmar) | GPU com driver atual, 2 GB de memória |
| Disco | 200 MB livres | SSD |
| Tela | 1366x768 | 1920x1080 |
| Runtime | Visual C++ 2015-2022 x64 | Igual |

O teste de aceite destes números: abrir, modelar a placa de exemplo, girar e salvar em cada um dos três
backends, sem travar, numa máquina de cada faixa.

## Modo leve para PC fraco

Um interruptor em Configurações, com sugestão automática na primeira execução, nunca imposta.

**Quando sugerir:** adaptador do tipo `Cpu`, backend OpenGL, memória abaixo de 8 GB ou até 4 processadores
lógicos.

**O que muda:**

| Ajuste | Normal | Leve |
|---|---|---|
| Qualidade da malha | Normal | Rascunho (metade dos triângulos e do tempo) |
| Anti-serrilhamento | 4x | Desligado |
| Simplificar ao girar | Desligado | Ligado |
| Iluminação realista | Ligada | Desligada |
| Fios do malhador | Metade dos lógicos | 2 |
| Atualização automática | Ligada | Ao soltar o controle, com atraso curto |
| Animações da interface | Ligadas | Desligadas |
| Tela inicial | Ligada | Sem fundo animado |

Medido na placa 20x20: Rascunho gera 155 mil triângulos contra 318 mil e leva 603 ms contra 1.089 ms.
Volume, massa e medidas não mudam, porque vêm da forma exata.

O modo leve deve ser só uma combinação dos ajustes existentes, sem caminho de código separado. Quase tudo
já existe no B61. Os efeitos do B139 devem nascer desligados nesse modo, e a meta de PBR não pode virar
requisito de abertura.

## Diagnóstico de partida quando o app não abre

Há três momentos de falha, e cada um pede uma resposta diferente.

**1. Antes do `main`: faltam DLLs.** O carregador do Windows falha antes de qualquer código do Fenkra, e
por isso uma checagem dentro do `fenkra.exe` nunca roda. A mensagem do sistema cita o nome da DLL, mas não
diz o que fazer. As saídas são levar as DLLs no pacote (passo 3 do plano) ou criar um iniciador pequeno com
runtime estático, que confere as dependências e explica o que falta. Recomendo a primeira agora.

**2. Depois do `main`, ao criar a janela e a GPU.** Tratar o `Err` de `run_native` em `main.rs`:

- Tentar de novo em ordem: padrão, depois Vulkan, depois OpenGL.
- Se tudo falhar, abrir caixa de mensagem nativa em texto simples, por exemplo: "O Fenkra não conseguiu
  usar a placa de vídeo. Atualize o driver e tente de novo. Detalhes salvos em ...".
- Oferecer botões Copiar detalhes e Abrir pasta do registro.
- Texto novo vai para `crates/fenkra-app/src/i18n.rs`. A caixa nativa deve evitar stdout e stderr, que
  não existem nesse modo.

**3. Durante o uso: pânico ou dispositivo perdido.** Gancho de pânico que grava arquivo e mostra a mesma
caixa. O B65 já prevê gancho e autorrecuperação; este item cobre a parte da GPU e a mensagem.

**Registro de partida** em `%LOCALAPPDATA%\Fenkra\logs\partida.log`, escrito a cada etapa antes de ela
começar, para que a última linha aponte onde parou:

- versão do Fenkra, do Windows e do OpenCASCADE;
- processadores lógicos e memória;
- adaptadores vistos, backend e tipo escolhidos, driver;
- limites pedidos e obtidos, MSAA efetivo;
- tempo de cada etapa até o primeiro quadro.

**Modo de segurança:** `fenkra.exe --seguro` força OpenGL, sem MSAA, malha Rascunho e sem abrir o último
documento. `fenkra.exe --diagnostico` gera um relatório e sai, sem abrir janela de trabalho, para o colega
enviar por e-mail. Na tela Sobre, um botão copia o mesmo relatório.

Se o adaptador escolhido for de software, o Fenkra abre e avisa uma vez: "Sem aceleração de vídeo. O
programa vai ficar lento. Atualize o driver." e sugere o modo leve. Esse caso hoje seria invisível.

## O que ainda falta medir

Em ordem de valor, e todos dependem de executar o aplicativo:

1. CPU em repouso em três telas: inicial, peça parada e painel com abas abertas.
2. Tempo até o primeiro quadro, separando DLLs, GPU, compilação de shaders e peça de exemplo.
3. Memória do processo inteiro ao abrir.
4. O erro real do colega. Uma pergunta resolve mais do que qualquer item acima.
5. Execução em PC fraco de verdade, com os backends Dx12, Vulkan e OpenGL forçados por `WGPU_BACKEND`.
