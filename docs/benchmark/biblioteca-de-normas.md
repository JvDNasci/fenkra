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
| BIB-02 | Fixadores padrão | 🟡 | ISO 4762, ISO 4017, DIN 933, ISO 4032, DIN 934, ISO 7089 de M3 a M20 (B29); faltam pinos, prisioneiros, porcas autotravantes, ASTM/polegada e tamanhos > M20. | B51 |
| BIB-03 | Gerador de engrenagens | 🟡 | Reta, helicoidal (loft), interna e cremalheira com cálculo e par na montagem (B28); perfil fixo no documento, sem filete de raiz, sem cônica e sem coroa/sem-fim. | B52 |
| BIB-04 | Gerador de eixos e chavetas | 🟡 | Eixo escalonado, chaveta DIN 6885 A e rasgo de cubo (B28); perfil fixo no documento, sem ranhuras de anel, roscas e cálculo de resistência; t2 da tabela sem segunda fonte. | B52 |
| BIB-05 | Materiais e bitolas | ✅ | MATERIALS, SHEET_MATERIALS, SHEET_GAUGES. |  |
| BIB-06 | Fixadores aeronáuticos e militares | ❌ | Fontes AN/MS/MIL de domínio público no ASSIST (ver biblioteca/fontes-de-dados.md). | CTX-HW |
| BIB-07 | Rolamentos | ❌ |  | CTX-HW |
| BIB-08 | Cilindros pneumáticos | ❌ |  | CTX-HW |
| BIB-09 | Guias lineares e fusos | ❌ |  | CTX-HW |
