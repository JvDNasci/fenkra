# Benchmark — Estrutural e soldas

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#estrutural-e-soldas).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Structure Design cria perfis sobre esqueleto e trata pontas por endcuts (snipe, trim, weld); Weld Design define os cordões. A organização em Product é manual.
- **SOLIDWORKS:** Weldment: um part multicorpo com membros estruturais, Trim/Extend com folga de solda, gussets, end caps e cordões; a Cut List agrupa corpos iguais e alimenta os desenhos (vistas por corpo e balões ligados à lista).
- **Inventor:** Frame Generator gera uma montagem com um arquivo por membro, a partir de um esqueleto; tratamentos de ponta (miter, notch, corner joint, trim/extend) e informações por membro.

## O que adotar no Fenkra

- O "Estrutura Pai" do contexto do usuário: um modelo mestre multicorpo (como o SW) que gera montagem com instâncias (como o Inventor) e reconhece corpos iguais.
- Desenho por corpo com balões ligados à lista de corte (SW).

## O que evitar

- Um arquivo por membro sem agrupamento de iguais (explosão de arquivos do Inventor).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| EST-01 | Membro estrutural | ✅ | StructMember (paths, anchor, angle, offsets). |  |
| EST-02 | Tratamento de cantos e uniões | 🟡 | CornerMode e joints; sem entalhe (notch) por perfil. | BM-EST-01 |
| EST-03 | Biblioteca de perfis | 🟡 | Poucas famílias (tubos, W, U, L, barras, T); faltam IPE/HEA/HEB/RHS/CHS EN e ABNT com dimensões (CTX-PRF). | CTX-PRF |
| EST-04 | Chapas, reforços e tampas | ✅ | StructPlate, StructGusset; teste estrutural_quadro_cantos_placa_reforco. |  |
| EST-05 | Cordões de solda | 🟡 | Símbolo ISO 2553 no desenho/PMI; sem cordão 3D. | BM-EST-02 |
| EST-06 | Lista de corte | 🟡 | cut_list() agrupa por designação + comprimento; balões não funcionam para estrutural (DR-33). | DR-33 |
| EST-07 | Corpos iguais → peça + instâncias | ❌ |  | CTX-EST |
| EST-08 | Estrutura Pai → montagem dinâmica | ❌ |  | CTX-EST |
| EST-09 | Otimização de barras | 🟡 | Cálculo de barras na lista de corte; sem plano de corte otimizado. | BM-EST-03 |
