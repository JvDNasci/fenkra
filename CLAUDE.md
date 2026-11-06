# Fenkra

CAD 3D paramétrico open source em alpha: Rust, egui, wgpu e OpenCASCADE 8.0.1 via `cxx`. Responda em
português do Brasil.

## Comandos

```bash
cargo run -p fenkra-app --release
cargo test --workspace
cargo test -p fenkra-solidos aba_
python -I scripts/benchmark/backlog.py
```

Pré-requisitos, variáveis `FENKRA_DEMO*` e release: [docs/desenvolvimento.md](docs/desenvolvimento.md).

## Mapa

| Crate | Papel |
|---|---|
| `fenkra-core` | Documento, árvore, parâmetros, sketch e solver, referências |
| `fenkra-kernel` | Fachada C++ do OpenCASCADE (`cpp/*.inc`) |
| `fenkra-malha` | Corpo de malha: formatos, solda, reparo, decimação, BVH, desvio, booleanas |
| `fenkra-solidos` | Recompute: sólidos, superfícies, chapa, estrutural, biblioteca |
| `fenkra-montagem` | Restrições, solver, juntas, mecanismos, BOM |
| `fenkra-desenho` | Folhas, vistas, cotas, exportação |
| `fenkra-project` | Versões, revisões, PN, estados |
| `fenkra-app` | Interface egui, viewport wgpu, bancadas, temas |

`fenkra-solidos/src/lib.rs` e `fenkra-app/src/app.rs` são grandes: busque antes de ler inteiro.

Cada bancada tem um especialista em `.claude/agents` e diretrizes em `.claude/skills/fenkra-*`.

## Regras do projeto

- Trabalhe na branch `develop`. Um commit por item de backlog, em português, citando o ID, e só
  quando pedirem.
- Texto novo de interface sempre com tradução em `crates/fenkra-app/src/i18n.rs`.
- Todo comando ou bancada nova segue o padrão de interface (B319): cores só por tema, com a peça
  sempre distinta do fundo; ícone da família nova; painel de comando livre e encaixável, aberto só
  durante o comando; árvore transparente; prévia e manipulação no 3D; dica por campo e tooltip com
  GIF. Critério de pronto: checklist de [interface](docs/benchmark/detalhe/interface.md).
- Ícones novos: gere com `scripts/gerar_icones.py` em pasta temporária e copie só os nomes novos.
- Testes montam o documento em código, recalculam e conferem status, volume e caixa envolvente.
- Não cite softwares proprietários nem suas marcas no código, na interface ou na documentação, nem
  como compatibilidade. Escreva "softwares de modelagem CAD avançados". Software livre confiável pode
  ser citado.
- Outros CADs podem ser estudados pela documentação pública, mas tudo é reescrito com nossas palavras.
  Não copie texto, imagens ou ícones. Use o [glossário](docs/produto/glossario.md).
- Toda fonte consultada entra em [procedencia.md](docs/arquitetura/procedencia.md).
- Decisão de arquitetura vira ADR em `docs/arquitetura/adr/`.

## Backlog e orçamento

O backlog nasce em `scripts/benchmark/backlog.py`, que gera `docs/backlog/`; não edite os gerados à
mão. Para executar qualquer trabalho, siga a skill `fenkra-fluxo`: ela escolhe os especialistas e orça
custo, tempo e consumo da sessão e da semana antes de começar. Reportes de bugs ficam no cofre JV (`F:\Obsidian JV\JV\Fenkra\Reportes`).

## Ao terminar um comando do CAD

Sempre que criar ou mudar o comportamento de um comando, escreva o material simples em
`F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Comandos\<Comando>.md`, usando `Templates/Comando.md`.
Ele explica em linguagem simples, para a pessoa ganhar familiaridade, o que o comando faz e como
funciona no kernel. Cite o caminho na resposta final.
