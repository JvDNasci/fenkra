# Estrutural e soldas

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/estrutural-e-soldas.md](../benchmark/estrutural-e-soldas.md).

## ✅ Pronto

- **Membro estrutural** (Membro) — StructMember (paths, anchor, angle, offsets).
- **Chapas, reforços e tampas** (Placa, reforço) — StructPlate, StructGusset; teste estrutural_quadro_cantos_placa_reforco.
- **Lista de corte** (Lista de corte por item) — structural_groups(): item por perfil + comprimento + geometria; balões no desenho (B26). `DR-33`
- **Corpos iguais → peça + instâncias** (Gerar montagem (itens)) — Corpos iguais viram uma peça com quantidade (estrutura_para_montagem, B26). `CTX-EST`

## 🟡 Parcial

- **Tratamento de cantos e uniões** (Cantos e uniões) — CornerMode e joints; sem entalhe (notch) por perfil. `BM-EST-01`
- **Biblioteca de perfis** (Biblioteca de perfis) — Poucas famílias (tubos, W, U, L, barras, T); faltam IPE/HEA/HEB/RHS/CHS EN e ABNT com dimensões (CTX-PRF). `CTX-PRF`
- **Cordões de solda** (Símbolo de solda) — Símbolo ISO 2553 no desenho/PMI; sem cordão 3D. `BM-EST-02`
- **Estrutura Pai → montagem dinâmica** (Gerar montagem) — Gera montagem com peças por item, mas os links ficam congelados: atualizar = gerar de novo; sem submontagens de solda. `B50`
- **Otimização de barras** (Barras (cálculo)) — Cálculo de barras na lista de corte; sem plano de corte otimizado. `BM-EST-03`
