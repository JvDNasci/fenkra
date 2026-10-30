# Biblioteca de normas

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/biblioteca-de-normas.md](../benchmark/biblioteca-de-normas.md).

## ✅ Pronto

- **Materiais e bitolas** (Materiais, bitolas) — MATERIALS, SHEET_MATERIALS, SHEET_GAUGES.

## 🟡 Parcial

- **Tabelas de furos e roscas** (hardware.rs) — METRIC M3–M20 "preliminar" e 7 normas de rosca. `BM-BIB-01`
- **Fixadores padrão** (—) — ISO 4762, ISO 4017, DIN 933, ISO 4032, DIN 934, ISO 7089 de M3 a M20 (B29); faltam pinos, prisioneiros, porcas autotravantes, ASTM/polegada e tamanhos > M20. `B51`
- **Gerador de engrenagens** (Engrenagens) — Reta, helicoidal (loft), interna e cremalheira com cálculo e par na montagem (B28); perfil fixo no documento, sem filete de raiz, sem cônica e sem coroa/sem-fim. `B52`
- **Gerador de eixos e chavetas** (Eixo e chaveta) — Eixo escalonado, chaveta DIN 6885 A e rasgo de cubo (B28); perfil fixo no documento, sem ranhuras de anel, roscas e cálculo de resistência; t2 da tabela sem segunda fonte. `B52`

## ❌ Ainda não existe

- **Fixadores aeronáuticos e militares** (—) — Fontes AN/MS/MIL de domínio público no ASSIST (ver biblioteca/fontes-de-dados.md). `CTX-HW`
- **Rolamentos** (—) —  `CTX-HW`
- **Cilindros pneumáticos** (—) —  `CTX-HW`
- **Guias lineares e fusos** (—) —  `CTX-HW`
