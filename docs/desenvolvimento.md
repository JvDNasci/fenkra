# Desenvolvimento

Como compilar, testar, gerar release e versionar o Fenkra (Rust + OpenCASCADE 8.0.1).
O que já existe em cada módulo está no [checklist](requisitos/checklist.md) e nos arquivos de [módulos](modulos/).

## Como compilar e rodar (Windows)

Pré-requisitos (uma vez):

1. Visual Studio com o workload **Desenvolvimento para desktop com C++** (MSVC).
2. Rust (rustup, toolchain `stable-x86_64-pc-windows-msvc`).
3. OpenCASCADE via vcpkg em `C:\dev\vcpkg`:

```bash
git clone https://github.com/microsoft/vcpkg.git C:/dev/vcpkg
```

```bash
C:/dev/vcpkg/bootstrap-vcpkg.bat -disableMetrics
```

```bash
C:/dev/vcpkg/vcpkg.exe install opencascade:x64-windows-release
```

Outro local: defina a variável `OCCT_ROOT` apontando para a pasta `installed/<triplet>`.

Rodar:

```bash
cargo run -p fenkra-app --release
```

Testes:

```bash
cargo test --workspace
```

## Testes

- Os testes são `#[test]` dentro de cada crate (ex.: `tests_aba_curva` em `crates/fenkra-solidos/src/lib.rs`).
  O padrão é montar o documento em código, recalcular e conferir **status Ok, volume, caixa envolvente**
  e, na chapa, o estado **dobrado e planificado**.
- Rodar um grupo: `cargo test -p fenkra-solidos aba_`.
- A tela inicial tem exemplos que também são testados (`exemplos_recalculam_sem_erro`).

## Automação sem interface

`fenkra-cli` roda comandos de peça com saída JSON, e `fenkra-mcp` é o servidor local para agentes. O contrato está em
[formatos/api-json.md](formatos/api-json.md), e o guia do servidor, em [automacao/mcp.md](automacao/mcp.md).

```bash
cargo test -p fenkra-cli -p fenkra-mcp
```

O teste da API Python roda junto, se houver `python` no `PATH`.

## Variáveis de ambiente para testar o app

Qualquer variável `FENKRA_DEMO*` (e `FENKRA_ABRIR`, `FENKRA_BANCADA`) faz o app **não gravar** as
preferências do usuário.

| Variável | Efeito |
|---|---|
| `FENKRA_DEMO_EXEMPLO=n` | abre o exemplo n (0–6) da tela inicial |
| `FENKRA_DEMO_CAMERA=yaw,pitch,fator` | gira a câmera no primeiro enquadramento (graus, graus, distância × enquadrada) — capturas reproduzíveis |
| `FENKRA_DEMO_LEVE=1` | mostra a caixa de sugestão do modo leve (motivos de exemplo) |
| `FENKRA_DEMO_RELATORIO=1` | mostra um relatório de falha de exemplo, sem gravar nada |
| `FENKRA_DEMO_INICIO=1` | mostra a tela inicial |
| `FENKRA_DEMO_TEMA=<nome>` | usa o tema (Escuro, Claro, NeoTech Minimalista…) |
| `FENKRA_DEMO_IMPORTAR=<arquivo>` | abre o painel de importação com o arquivo; `FENKRA_DEMO_IMPORTAR_OK=1` confirma sozinho |
| `FENKRA_DEMO_DEFEITO=n` | com a confirmação automática, já realça o defeito n do relatório |
| `FENKRA_DEMO_PASTA=<pasta>` | grava as peças de um conjunto importado nessa pasta, sem perguntar |
| `FENKRA_DEMO_EXPORTAR=iges` | abre o painel de exportação (`iges` ou outro valor para STEP) |
| `FENKRA_DEMO_CHAPA`, `_EST`, `_SUP`, `_ASM`, `_PMI`, `_DWG` | abre o exemplo de cada bancada |
| `FENKRA_DEMO_ASM=<nome>` | montagem de exemplo com: `mec` (mecanismo), `cmd` (restrição com fantasma), `junta` (junta automática), `simbolos` (cota e junta no modelo), `manip` (manipulador numa dobradiça), `limites` (arco e trilho de uma junta escolhida), `limites_cmd` (limites na criação da junta), `conflito` (proposta sem solução, com a restrição apontada), `exibicao`, `explosao` e `explosao_anim` (painel de exibição em cada aba), `duzentas` (200 restrições), `inserir` e `inserir_vazia` (peça no cursor), `interf`, `interf_oculta` e `interf_transparente` (realce da interferência), `repeticao` (fantasmas e marcas), `fixadores` (assistente), `encaixe` (pontos de encaixe), `grade<N>` (N peças iguais), `bom_config` (lista de peças com o PN de cada configuração e o erro de uma inexistente) |
| `FENKRA_DEMO_ASM=grade<N>` / `variado<N>` / `inserir_limite` | N peças iguais; N peças diferentes (nenhuma divide malha); peça na mão com teto de triângulos baixo, para ver a oferta da versão simplificada |
| `FENKRA_MEDIR_QUADROS=<arquivo.csv>` | grava o tempo de CPU de cada quadro, as fases pesadas e os bytes na placa (B125, B259) |
| `FENKRA_MEDIR_QUADROS=<arquivo.csv>` (B347) | além do acima, grava o intervalo entre quadros (`intervalo_ms`), o total da interface (`ui_total_ms`) e cada etapa do quadro (`etapa_<nome>_ms`: `pre_*`, `top`, `paineis`, `viewport`, `vp_*`, `cena`...); a etapa que passar de 2 ou 3 ms num app parado é a suspeita |
| `FENKRA_DEMO_GIRAR=<graus por quadro>` | gira a câmera sozinha a cada quadro, como um arrasto contínuo, para medir o quadro em movimento (use com `FENKRA_MEDIR_QUADROS`) |
| `FENKRA_MEDIR_FIM=<n>` | fecha o programa depois de n quadros |
| `FENKRA_MEDIR_GPU=1` | com `FENKRA_MEDIR_QUADROS`, espera a placa depois da viewport e grava `gpu_vp_ms` (tira o paralelismo entre CPU e placa, então o tempo de quadro medido sobe) |
| `FENKRA_DEMO_ABA=<n>` | abre o painel de comando já na aba n (a primeira é 0), para capturas sem mouse |
| `FENKRA_DEMO_DOBRA=1` | tubo retangular dobrado em U na bancada Estrutural; com `FENKRA_EDITAR=Membro` abre o painel do nó de dobra |
| `FENKRA_DEMO_FORMA=<modo>` | abre uma cúpula em edição na bancada Forma livre; `gaiola`, `liso`, `laco`, `tangencia`, `suave` (círculo de influência), `selecao` (laço selecionado) `vinculos` (nós presos e cota) e `perdido` (o pai apagado) já mudam o que aparece; `converter`, `g0` e `g2` abrem o relatório da conversão |
| `FENKRA_DEMO_DOBRA=1` | tubo retangular dobrado em U na bancada Estrutural; com `FENKRA_EDITAR=Membro` abre o painel do nó de dobra. Com `canto`, `v` ou `multiplo` no lugar do 1, as duas dobras usam V com raio, V comum ou V múltiplo (tubo fechado no 3D) |
| `FENKRA_DEMO_ASM=<nome>` | montagem de exemplo com: `mec` (mecanismo), `cmd` (restrição com fantasma), `junta` (junta automática), `simbolos` (cota e junta no modelo), `manip` (manipulador numa dobradiça), `duzentas` (200 restrições) |
| `FENKRA_DEMO_MALHA=<modo>` | acrescenta um corpo de malha ondulado à peça e abre o comando: `importar`, `reparar` (calota aberta), `decimar`, `booleana` |
| `FENKRA_DEMO_IMPRESSAO=1` | abre o preparo de impressão com a peça aberta; `FENKRA_DEMO_IMPRESSAO_ARQUIVO=<arquivo>` grava o 3MF sem perguntar |
| `FENKRA_EXIBIR=inclinacao\|isofotas\|dfm` | abre já no modo de análise (além de `zebra` e `curvatura`); com `dfm`, `FENKRA_DEMO_DFM=<parte do nome do perfil>` escolhe o processo (`fresagem`, `chapa`, `inje`, `3d`), liga o mapa de espessura e varre sozinho |
| `FENKRA_DEMO_DWG=1` com `_ALCA`, `_DIALOGO`, `_CENTROS`, `_TABELA`, `_SIMBOLO` | desenho de exemplo já no estado a capturar: `_ALCA=corte\|detalhe\|projetada\|arrastando` (vista selecionada com alças; `arrastando` finge o arraste da vista de topo, com linha de alinhamento e cota ao vizinho), `_DIALOGO=novo\|exportar\|cota`, `_CENTROS=simples\|linear\|circular\|todos`, `_TABELA=1` (sem a coluna Rosca, com etiquetas e a linha A2 em destaque), `_SIMBOLO=rug\|solda` |
| `FENKRA_DEMO_FOTO=<arquivo.png>` | grava a janela depois de 9 s e fecha o programa |
| `FENKRA_DEMO_POLIMENTO=<caso>` | abre uma peça de prova do polimento da superfície (B346): `esfera`, `cilindro`, `toro`, `superficie` (painel de dupla curvatura), `filete`, `bolso`, `filete_sup` |
| `FENKRA_DEMO_QUALIDADE=<desvio>,<ângulo>` | força a malha de trabalho, por exemplo `0.02,45` (a mais grossa aceita); `FENKRA_EXIBIR` também aceita `sombreado` e `render` |
| `FENKRA_DEMO_SEM_POLIMENTO=1` | não pede a malha de tela, para comparar; `FENKRA_DEMO_GIRO=<graus por quadro>` gira a câmera sem parar, para medir o quadro com `FENKRA_MEDIR_QUADROS` (a fase `placa` traz o tempo da placa de vídeo) |
| `FENKRA_DEMO_JANELA=<nome>` | abre no segundo quadro o que só o mouse abriria: `salvar_como`, `empacotar`, `reparar` (gravam a peça numa pasta temporária com dois links), `liberar` (com `FENKRA_DEMO_PROJETO=1`; cria uma montagem que usa a peça), `resumo` (com `FENKRA_DEMO_EST=1`), `roda`, `roda_editor`, `tabela`, `pmi_barra` e `pmi_fcf` (com `FENKRA_DEMO_PMI=1`), `daltonico` (liga a paleta segura) |
| `FENKRA_DEMO_SIMULAR=<protanopia\|deuteranopia\|tritanopia>` | com `FENKRA_DEMO_FOTO`, grava a captura como essa visão a veria |
| `FENKRA_DEMO_CHAPA=aba` ou `abaerro` | com `FENKRA_EDITAR=AbaUnificada`, abre o comando de aba unificado (o segundo com valores fora do limite); `FENKRA_DEMO_ABA=<n>` escolhe a aba do painel |
| `FENKRA_DEMO_CHAPA=canal` | canal em U sem comando de chapa e o Converter em chapa sobre ele (mapa de dobras); com `FENKRA_DEMO_PLANA=1` já aparece desdobrado; `FENKRA_DEMO_CHAPA=canto` com `FENKRA_EDITAR=FechaCanto` mostra a alça da folga do canto |
| `FENKRA_DEMO_PREVIA=<nome>` | abre um comando como novo, para ver a prévia fantasma: `ultimo` (último comando da peça de exemplo), `membro` (com `FENKRA_DEMO_EST`), `casca`, `espessura`, `rosca`, `nervura`, `plano`; da bancada Sólidos: `padrao_pontos` e `padrao_circular` (pontos clicáveis, setas e arco), `espelho` (plano do espelho), `furo_guia` (o guia pousa no centro da face de cima), `filete_tangente` (cadeia tangente realçada), `filete_grande` (raio que não cabe: erro junto da seta e superfície tentada), `escareado_casca` (casca abrindo o escareado), `dois_lados`, `saida`, `primitiva_base`, `mover_corpo`, `escala_corpo` (alças 3D); os comandos de superfície têm `sup_dividir`, `sup_aparar`, `sup_filete`, `sup_paralela`, `sup_tangente`, `sup_intersecao`, `sup_varredura`, `sup_engrossar`, `sup_corpo`, `sup_regioes`, `sup_regioes_manter`, `sup_conectores`, `sup_concordancia`, `sup_conectar`, `sup_spline` (com `FENKRA_DEMO_SPLINE=1`, já escolhe o segundo ponto e liga o pente) |
| `FENKRA_DEMO_PREVIA=<nome>` | abre um comando como novo, para ver a prévia fantasma: `ultimo` (último comando da peça de exemplo), `membro` (com `FENKRA_DEMO_EST`), `casca`, `espessura`, `rosca`, `nervura`, `plano`; os comandos de superfície têm `sup_dividir`, `sup_aparar`, `sup_filete`, `sup_paralela`, `sup_tangente`, `sup_intersecao`, `sup_varredura`, `sup_engrossar`, `sup_corpo`, `sup_regioes`, `sup_regioes_manter`, `sup_conectores`, `sup_concordancia`, `sup_conectar`, `sup_spline` (com `FENKRA_DEMO_SPLINE=1`, já escolhe o segundo ponto e liga o pente) |
| `FENKRA_DEMO_PREVIA=sup_*` (preenchimento e alças) | `sup_preencher` (G1 em volta de um buraco de esfera; com `FENKRA_EXIBIR=zebra` mostra a junção), `sup_preencher_g0`, `sup_preencher_aberto` (bolas nas pontas e rótulo da folga), `sup_offset`, `sup_offset_neg`, `sup_offset_dois`, `sup_offset_curva`, `sup_extrusao`, `sup_revolucao`, `sup_cilindro`, `sup_tubo`, `sup_extrapolar`, `sup_ponto`, `sup_loft_solido` |
| `FENKRA_DEMO_ASM=resolver` | janela Resolver referências com um componente de dois candidatos e um arquivo sumido |
| `FENKRA_DEMO_IMPORTAR=<arquivo>` | abre o painel de importação com o arquivo, sem o seletor; o `fenkra-conv` precisa estar ao lado do executável; com `FENKRA_DEMO_IMPORTAR_OK=1`, confirma sozinho e mostra o relatório |
| `FENKRA_DEMO_SKETCH=<caso>` | abre um sketch pronto com o estado que só o mouse daria: `retangulo`, `canto` (painel do comando), `setas`, `pegas`, `barra`, `conflito`, `sombra`, `regiao` (quadrado dentro de retângulo, vizinho e cadeia aberta), `oculto`, `fatia`, `poligono`, `chanfro`, `copia`, `aparar`, `estender`, `aparar_shift`, `janela`, `cruzada`, `cadeia`, `cadeia_offset`, `snap`, `arraste`, `arraste_ctrl`, `inferencia`, `perpendicular`, `cota_raio`, `cota_diametro`, `cota_angulo`, `cota_suplemento`, `vinculo_tipo`, `vinculo_pontos`, `vinculo_quebrado` |
| `FENKRA_DEMO_PREVIA=rosca_furo\|rosca_furo_modelada\|rosca_interna` e `FENKRA_DEMO_CONCLUIR=1` | rosca em furo (B351): bloco com furos de rosca cosmética (escareado à direita, passante à esquerda) e o comando aberto sobre um furo de rosca modelada (só linhas na edição); `rosca_interna` abre o comando Rosca na face de um furo escareado; `FENKRA_DEMO_CONCLUIR=1` conclui o comando logo depois de aberto e mostra o sulco real |
| `FENKRA_DEMO_PREVIA=passo_<mm>` e `FENKRA_DEMO_PASSO=1` | caixa com esse lado e a extrusão aberta; a seta mostra o salto do arrasto em mm e em pixels |
| `FENKRA_DEMO_SKETCH_CURSOR=x,y` | com `FENKRA_DEMO_SKETCH`, põe o cursor nessas coordenadas do plano do sketch |
| `FENKRA_DEMO_SKETCH_GESTO=<passos>` | com `FENKRA_DEMO_SKETCH`, manda eventos de ponteiro crus para o app, no caminho do mouse de verdade: `clique:x,y`, `shift:x,y`, `ctrl:x,y` e `arrasto:x0,y0,x1,y1`, separados por `;`, em coordenadas do plano do sketch. Começa 5 s depois de abrir; a seleção de cada passo sai na saída de erro |
| `FENKRA_DEMO_ENCAIXE=1` | abre um tubo redondo oblíquo encaixado noutro (B155), com a janela de cantos e o cartão do encaixe abertos; `=folga` mostra também o rótulo flutuante da folga |
| `FENKRA_DEMO_LIMITE=<x\|y\|z>:<mm>[:estender]` | com `FENKRA_DEMO_EST=1`, abre a janela de cantos com a seção de aparar/estender até limite (B37) e o plano escolhido |
| `FENKRA_DEMO_GOMOS=1` | L de tubo redondo com corte simples em quatro gomos (B326); com `FENKRA_EDITAR=Membro` abre o painel do nó |
| `FENKRA_DEMO_SOLDA=<cosmetico\|solido\|intermitente>` | tubo contra placa com o cordão de filete (B285) |
| `FENKRA_DEMO_ESTADO=<nome>` | força o estado na barra: `sem`, `liberada`, `editado`, `leitura` ou `conflito` |
| `FENKRA_DEMO_PROJETO=1` | abre um projeto de exemplo com o painel à vista; `comparar` já mostra a comparação com a revisão |
| `FENKRA_DEMO_SECAO=1` | liga o corte de seção na prévia da casca |
| `FENKRA_ABRIR=<arquivo>`, `FENKRA_BANCADA=<bancada>` | abre um arquivo / uma bancada |
| `FENKRA_EN=1` | interface em inglês |

## Release

1. Subir a versão na linha 14 do `Cargo.toml` (`0.1.0-alpha.N`).
2. Commit e tag `v0.1.0-alpha.N`.
3. Fechar o app e rodar `powershell -ExecutionPolicy Bypass -File scripts\release.ps1`
   → `dist\Fenkra-<versão>-windows-x64\` e o `.zip`.
4. Commit do `Cargo.lock` atualizado.

## Versionamento no git

- Branch de trabalho: `develop` (remoto `origin` = `github.com/JvDNasci/fenkra`); `main` recebe as releases.
- Um commit por item de backlog, mensagem em português, citando o ID (`DR-20`, `BM-SKT-03`…).
- Ícones novos: gerar com `scripts/gerar_icones.py` numa pasta temporária e copiar **só os nomes novos**.
- Textos novos da interface sempre com tradução em `crates/fenkra-app/src/i18n.rs`.

## CI e licenças

`.github/workflows/ci.yml` roda em cada commit e não publica nada. Sem OpenCASCADE: clippy e testes. Com OpenCASCADE: testes e corpus. Licenças: `cargo deny check licenses bans sources` e a lista do cargo-about.

Todo fonte abre com o cabeçalho SPDX. Aplique com `python scripts/spdx.py` e confira com `python scripts/spdx.py --verificar`; o teste `crates/fenkra-app/tests/spdx.rs` reprova arquivo sem cabeçalho.
