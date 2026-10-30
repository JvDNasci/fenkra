# Benchmark — Biblioteca de normas

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#biblioteca-de-normas).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Catálogos de peças padrão; sem gerador de engrenagens nativo.
- **SOLIDWORKS:** Toolbox: fixadores por norma, Hole Wizard ligado aos mesmos dados, Smart Fasteners preenchendo furos com pilhas de porca/arruela.
- **Inventor:** Content Center (biblioteca) + Design Accelerator: geradores de engrenagem, eixo, chaveta e conexão parafusada com cálculo e restrições de movimento.

## O que adotar no Fenkra

- Uma base de dados única para furo, rosca e fixador (SW), em formato aberto (ADR 0008).
- Geradores com cálculo e inserção restrita (Inventor) — o usuário pediu "o mais próximo do Inventor".

## O que evitar

- Editar peças da biblioteca por sketch/feature sem atualizar a base (alerta do próprio SW).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| BIB-01 | Tabelas de furos e roscas | 🟡 | METRIC M3–M20 "preliminar" e 7 normas de rosca. | BM-BIB-01 |
| BIB-02 | Fixadores padrão | ❌ | Lista de normas em contextos/fixadores.md. | CTX-FXL |
| BIB-03 | Gerador de engrenagens | ❌ | Só acoplamento de engrenagem na montagem. | CTX-ENG |
| BIB-04 | Gerador de eixos e chavetas | ❌ |  | CTX-ENG |
| BIB-05 | Materiais e bitolas | ✅ | MATERIALS, SHEET_MATERIALS, SHEET_GAUGES. |  |
