---
name: fenkra-superficies
description: Diretrizes da bancada Superfícies e wireframe do Fenkra. Use ao criar, corrigir ou pesquisar NURBS, continuidade G0/G1/G2, costura, offset e preenchimento, ou ao tocar em crates/fenkra-solidos/src/geo3d.rs.
---

# Superfícies e wireframe

## Domínio

crates/fenkra-solidos/src/geo3d.rs; crates/fenkra-kernel/cpp/surfaces.inc, hybrid.inc; crates/fenkra-app/src/geo_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/superficies-e-wireframe.md` e, se precisar de contexto, `docs/pesquisa/superficies.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Superfícies e wireframe\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de NURBS, continuidade G0/G1/G2, costura, offset e preenchimento, busque primeiro documentação oficial do OpenCASCADE, normas,
livros e artigos técnicos, e depois documentação pública de outros CADs. Guarde uma nota por assunto
na pasta de conhecimento, com: como funciona, fórmulas, casos de borda, sintomas de falha e como
depurar. Escreva com suas palavras, sem copiar texto ou imagem, e registre as fontes em
`docs/arquitetura/procedencia.md`. Pesquise só o que a tarefa pede: pesquisa tem custo.

## Diretrizes

Preencha com o que for aprendido. Cada regra leva o motivo.

- Deslocar e engrossar sempre sobre o casco costurado (`casco_costurado`, tolerância 1e-4 a 1e-2). Sem a
  costura cada face anda sozinha e as vizinhas ficam sobrepostas ou abertas na emenda (B518).
- Todo deslocamento de superfície passa por `confere_raio` antes do OpenCASCADE: além do raio de curvatura
  do lado deslocado a face dobra sobre si, e o deslocamento simples não confere autointerseção e entrega
  sólido torcido que às vezes passa no `BRepCheck` (B518). Aceite só sólido único, válido e com volume
  perto de área × espessura (`solido_coerente`).
- Engrossar casco aberto: `BRepOffset_MakeOffset::Initialize(..., Thickening = true)` + `MakeOffsetShape`
  (o que o comando `thickshell` faz); o `MakeThickSolidBySimple` fica como reserva para casco liso.
- Filete entre superfícies: o corte de cada uma pela outra estendida é separado, então a junta precisa da
  costura adaptativa; superfície que só encosta (não atravessa) fica inteira (B517).
- Subelemento de superfície de referência: dono = a superfície, papel = índice da face
  (`PAPEL_SUPERFICIE + i`), aresta com `EdgeKey.edge`; borda repete a face nos dois lados (B515, ADR 0010).
- Comando que consome superfícies inteiras (filete, junção, aparar) esconde as entradas ao sair o resultado
  (`sup3d::entradas_consumidas`): entrada e resultado no mesmo lugar brigam pelo pixel (B517).

## Como verificar

`cargo test -p <crate> <filtro>`, com documento montado em código conferindo status, volume e caixa
envolvente. Mudança visual: capture com as variáveis `FENKRA_DEMO*`.

## Vizinhos

Consulte solidos, kernel quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

- Ordem dos campos vazios: `serde_json::Value` guarda objetos em ordem alfabética (sem `preserve_order`). Percorrer o `Value` para achar o próximo campo inverte campos do mesmo tipo (`apex` antes de `guide1`, B466). Use `slots::fill` / `slots::vazios_em_ordem`, que seguem a ordem da declaração.
- A pasta de compilação é compartilhada entre os agentes (`F:/cargo-target/projeto-cad`): com compilação
  incremental, duas sessões no mesmo crate quebram o cache ("failed to move dependency graph"). Compile com
  `CARGO_INCREMENTAL=0` quando houver outros agentes compilando.
- `ComboBox::width` é largura mínima e `truncate` só corta até a largura disponível: para caber na coluna com botões ao lado, ponha a lista numa faixa (`allocate_ui`), como `geo_ui::na_faixa`.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Superfície de varredura com perfil circular em spline 3D falha: use varredura sólida como contorno até corrigir (K-08).
