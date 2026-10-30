# Estrutural e soldas

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/estrutural-e-soldas.md](../benchmark/estrutural-e-soldas.md).

## ✅ Pronto

- **Membro estrutural** (Membro) — StructMember (paths, anchor, angle, offsets).
- **Chapas, reforços e tampas** (Placa, reforço) — StructPlate, StructGusset; teste estrutural_quadro_cantos_placa_reforco.

## 🟡 Parcial

- **Tratamento de cantos e uniões** (Cantos e uniões) — CornerMode e joints; sem entalhe (notch) por perfil. `BM-EST-01`
- **Biblioteca de perfis** (Biblioteca de perfis) — Poucas famílias (tubos, W, U, L, barras, T); faltam IPE/HEA/HEB/RHS/CHS EN e ABNT com dimensões (CTX-PRF). `CTX-PRF`
- **Cordões de solda** (Símbolo de solda) — Símbolo ISO 2553 no desenho/PMI; sem cordão 3D. `BM-EST-02`
- **Lista de corte** (Lista de corte) — cut_list() agrupa por designação + comprimento; balões não funcionam para estrutural (DR-33). `DR-33`
- **Otimização de barras** (Barras (cálculo)) — Cálculo de barras na lista de corte; sem plano de corte otimizado. `BM-EST-03`

## ❌ Ainda não existe

- **Corpos iguais → peça + instâncias** (—) —  `CTX-EST`
- **Estrutura Pai → montagem dinâmica** (—) —  `CTX-EST`
