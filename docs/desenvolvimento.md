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

## Variáveis de ambiente para testar o app

Qualquer variável `FENKRA_DEMO*` (e `FENKRA_ABRIR`, `FENKRA_BANCADA`) faz o app **não gravar** as
preferências do usuário.

| Variável | Efeito |
|---|---|
| `FENKRA_DEMO_EXEMPLO=n` | abre o exemplo n (0–5) da tela inicial |
| `FENKRA_DEMO_INICIO=1` | mostra a tela inicial |
| `FENKRA_DEMO_TEMA=<nome>` | usa o tema (Escuro, Claro, NeoTech Minimalista…) |
| `FENKRA_DEMO_CHAPA`, `_EST`, `_SUP`, `_ASM`, `_PMI`, `_DWG` | abre o exemplo de cada bancada |
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
