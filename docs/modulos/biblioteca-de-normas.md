# Biblioteca de normas

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/biblioteca-de-normas.md](../benchmark/biblioteca-de-normas.md).

## ✅ Pronto

- **Materiais e bitolas** (Materiais, bitolas) — MATERIALS, SHEET_MATERIALS, SHEET_GAUGES.

## 🟡 Parcial

- **Tabelas de furos e roscas** (hardware.rs) — METRIC M3–M20 "preliminar" e 7 normas de rosca. `BM-BIB-01`
- **Fixadores padrão** (—) — ISO 4762, ISO 4017, DIN 933, ISO 4032, DIN 934, ISO 7089 de M3 a M20 (B29); faltam pinos, prisioneiros, porcas autotravantes, ASTM/polegada e tamanhos > M20. `B51`
- **Gerador de engrenagens** (Engrenagens) — Reta, helicoidal (loft), interna e cremalheira com cálculo e par na montagem (B28); filete de raiz por trocoide da cremalheira-ferramenta (com rebaixo seguido pelo cruzamento), correção x mínima calculada e mostrada, aba Verificar (Lewis no dente gerado e Hertz, relatório HTML); o perfil é redesenhado no recálculo quando m, z, α, β, x, folga ou L mudam na tabela do documento ou na configuração (`engrenagens_regen`); aba Eixo separada (cada aba numa rolagem), com rosca métrica modelada, recursos medidos pela borda e resistência estática do eixo (von Mises, seção líquida); dica com esquema em cada parâmetro; ISO 6336/AGMA ainda não (sem fonte pública: a opção avisa). Cônica e sem-fim/coroa (com garganta) ficam na Transmissão (B341). `B52`
- **Gerador de eixos e chavetas** (Eixo e chaveta) — Eixo escalonado, chaveta DIN 6885 A e rasgo de cubo (B28); perfil fixo no documento, sem ranhuras de anel, roscas e cálculo de resistência; t1 e t2 da DIN 6885-1 conferidos em duas fontes gratuitas em todas as linhas. `B52`

## ❌ Ainda não existe

- **Fixadores aeronáuticos e militares** (—) — Fontes AN/MS/MIL de domínio público no ASSIST (ver biblioteca/fontes-de-dados.md). `CTX-HW`
- **Rolamentos** (—) —  `CTX-HW`
- **Cilindros pneumáticos** (—) —  `CTX-HW`
- **Guias lineares e fusos** (—) —  `CTX-HW`
- **Rebites** (—) —  `B58`
