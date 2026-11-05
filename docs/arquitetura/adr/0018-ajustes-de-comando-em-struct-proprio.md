# ADR 0018 — Ajustes novos de extrusão, furo e padrão ficam em structs próprias

## Contexto

B89, B90 e B91 pedem campos novos em três comandos que já existem: início deslocado e término do segundo lado
na extrusão, profundidade de rosca e deslocamento da ponta no furo, instâncias puladas no padrão. Os três
comandos são montados por literal de struct em dezenas de lugares (testes, montagem, exemplos). Cada campo solto
obrigaria a mexer em todos os literais de novo a cada item, e os colegas em paralelo quebrariam a cada fusão.

## Decisão

Cada comando ganha um único campo novo com `#[serde(default)]` e tipo com `Default`:

- `Extrude::ends: ExtrudeEnds` (início, deslocamento do término, término e deslocamento do lado 2);
- `Hole::extra: HoleExtra` (profundidade da rosca, folga no fundo em passos, deslocamento da ponta);
- `Pattern::skip: Vec<(u32, u32)>` (instâncias puladas por posição).

Campos de texto vazios valem zero. Arquivos antigos abrem sem mudança. Os próximos ajustes desses comandos
entram dentro dessas structs, sem tocar nos literais.

As instâncias puladas guardam a posição (i, j) e não a ordem: mudar a quantidade não faz a pulada trocar de
lugar. Posição que deixou de existir é ignorada e o comando avisa.

O cálculo dos términos por lado fica em `fenkra-solidos/src/termino.rs`, fora do `lib.rs`.

## Consequências

- Uma única passada por todos os literais (feita neste item) e nenhuma nas próximas.
- `BodyCollector` ganha `material` por corpo (B92) pelo mesmo motivo: campo com `default`.
