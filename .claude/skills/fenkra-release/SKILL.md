---
name: fenkra-release
description: Passos para gerar uma versão do Fenkra: testes, licenças, versão, pacote, abertura do executável e tag. Use ao compilar o executável, gerar alpha ou preparar lançamento.
---

# Release

Commit, tag e push só com pedido do usuário.

1. Integre tudo no `develop` e pare os agentes.
2. Confira dependências novas: `THIRD-PARTY-LICENSES` e `NOTICE` precisam citá-las.
3. Suba a versão em `Cargo.toml` (`version` do workspace) e comite com `Cargo.lock`.
4. Rode `powershell -ExecutionPolicy Bypass -File scripts/release.ps1`. Ele exige as licenças e copia o runtime C++, `fenkra-conv`, `fenkra-cli` e `fenkra-mcp`.
5. Abra o pacote de verdade: rode `dist/Fenkra-<versão>-windows-x64/fenkra.exe` com `FENKRA_DEMO_FOTO` e `APPDATA` temporário, e olhe a versão na tela inicial.
6. `cargo test --workspace --no-fail-fast` com a saída em log. Falha intermitente se investiga, não se ignora.
7. `git tag -a v<versão>` e push de `develop` e da tag.
8. Informe SHA256, tamanho e o que não foi testado. Não crie release no GitHub sem pedido.

Não cite marcas proprietárias em notas de versão.
