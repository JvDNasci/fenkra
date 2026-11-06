---
name: fenkra-integracao
description: Regras para mesclar branches de agentes no develop do Fenkra sem perder trabalho. Use ao integrar um worktree, resolver conflito, renumerar ADR ou fechar uma leva antes do push.
---

# Integração no develop

## Conflitos

Nunca aceite os dois lados às cegas em código: isso já duplicou braços de `match` sem quebrar a compilação.

| Arquivo | Como resolver |
|---|---|
| `i18n.rs`, `main.rs`, listas de `mod` | Mantenha as duas entradas, uma por linha |
| `procedencia.md`, `desenvolvimento.md` | Mantenha as duas linhas |
| `icons.rs` | Una os nomes; a lista é um conjunto |
| `gerar_icones2.py` (`SO_INTERFACE`) | Una os conjuntos numa linha só |
| `Cargo.lock` | Fique com o seu e deixe o cargo atualizar |
| Código com lógica | Leia os dois lados e escreva a versão certa |

Antes de comitar: `git grep -E '^(<<<<<<< |>>>>>>> )' -- crates docs scripts` deve voltar vazio.

## Depois de cada merge

1. Procure `unreachable pattern` no `cargo check --workspace --tests --message-format=short`: é sinal de lado duplicado.
2. Campo novo em struct quebra literais em outro branch, inclusive em testes: `Extrude`, `Hole`, `Pattern`, `EdgeOp`, `Ctx`. Use `Default::default()`.
3. Variante nova em enum quebra `match` em outro branch: `Seg2`, `Bench`.
4. Lista com tamanho fixo (`KeyAction::ALL`) e contagens em testes (`icons.rs`) precisam ser atualizadas.
5. ADR: os agentes escolhem o mesmo número. Renumere para o próximo livre e ajuste a primeira linha.
6. Rótulo novo de campo precisa de tradução e dica (`textos_campos.rs`).

## Antes do push

- Máquina de 16 GB: rode a suíte completa com poucos agentes ativos e `CARGO_BUILD_JOBS` baixo. "memory allocation failed" é falta de memória, não defeito.
- Redirecione a saída para um log e leia o log. Não use `| head` no pipe do `cargo test`, porque corta a execução.
- Use `git add` por pasta (`crates docs`), nunca `-A`: há arquivos locais do usuário.
- Só marque feito no backlog depois que o merge entrou no develop.

Conflito no arquivo inteiro em `lib.rs` ou `app.rs` é fim de linha (CRLF contra LF): extraia as três versões com `git show :1:`, `:2:`, `:3:`, normalize para LF e rode `git merge-file`.
