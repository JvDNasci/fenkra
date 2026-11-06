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
| `FENKRA_DEMO_CHAPA`, `_EST`, `_SUP`, `_ASM`, `_PMI`, `_DWG` | abre o exemplo de cada bancada |
| `FENKRA_DEMO_ASM=<nome>` | montagem de exemplo com: `mec` (mecanismo), `cmd` (restrição com fantasma), `junta` (junta automática), `simbolos` (cota e junta no modelo), `manip` (manipulador numa dobradiça), `duzentas` (200 restrições), `inserir` e `inserir_vazia` (peça no cursor), `interf`, `interf_oculta` e `interf_transparente` (realce da interferência), `repeticao` (fantasmas e marcas), `fixadores` (assistente), `encaixe` (pontos de encaixe), `grade<N>` (N peças iguais) |
| `FENKRA_DEMO_ABA=<n>` | abre o painel de comando já na aba n (a primeira é 0), para capturas sem mouse |
| `FENKRA_DEMO_DOBRA=1` | tubo retangular dobrado em U na bancada Estrutural; com `FENKRA_EDITAR=Membro` abre o painel do nó de dobra |
| `FENKRA_DEMO_FORMA=<modo>` | abre uma cúpula em edição na bancada Forma livre; `gaiola`, `liso`, `laco` e `tangencia` já mudam o que aparece; `converter`, `g0` e `g2` abrem o relatório da conversão |
| `FENKRA_DEMO_DOBRA=1` | tubo retangular dobrado em U na bancada Estrutural; com `FENKRA_EDITAR=Membro` abre o painel do nó de dobra. Com `canto`, `v` ou `multiplo` no lugar do 1, as duas dobras usam V com raio, V comum ou V múltiplo (tubo fechado no 3D) |
| `FENKRA_DEMO_ASM=<nome>` | montagem de exemplo com: `mec` (mecanismo), `cmd` (restrição com fantasma), `junta` (junta automática), `simbolos` (cota e junta no modelo), `manip` (manipulador numa dobradiça), `duzentas` (200 restrições) |
| `FENKRA_DEMO_FOTO=<arquivo.png>` | grava a janela depois de 9 s e fecha o programa |
| `FENKRA_DEMO_PREVIA=<nome>` | abre um comando como novo, para ver a prévia fantasma: `ultimo` (último comando da peça de exemplo), `membro` (com `FENKRA_DEMO_EST`), `casca`, `espessura`, `rosca`, `nervura`, `plano`; os comandos de superfície têm `sup_dividir`, `sup_aparar`, `sup_filete`, `sup_paralela`, `sup_tangente`, `sup_intersecao`, `sup_varredura`, `sup_engrossar`, `sup_corpo`, `sup_regioes`, `sup_regioes_manter` |
| `FENKRA_DEMO_IMPORTAR=<arquivo>` | abre o painel de importação com o arquivo, sem o seletor; o `fenkra-conv` precisa estar ao lado do executável; com `FENKRA_DEMO_IMPORTAR_OK=1`, confirma sozinho e mostra o relatório |
| `FENKRA_DEMO_SKETCH=<caso>` | abre um sketch pronto com o estado que só o mouse daria: `retangulo`, `canto` (painel do comando), `setas`, `pegas`, `barra`, `conflito`, `sombra`, `oculto`, `fatia` |
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
