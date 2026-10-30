# Benchmark — PDM / PLM

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#pdm--plm).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** ENOVIA/VPM: produto, revisões, maturidade e estrutura num banco central.
- **SOLIDWORKS:** SOLIDWORKS PDM: cofre com check-in/out, fluxos e estados, busca por metadados.
- **Inventor:** Vault: check-in/out, esquemas de revisão e estados de ciclo de vida.

## O que adotar no Fenkra

- Fase 1 local (SQLite + cofre em pasta) do contexto PLM, evoluindo o fenkra-project.
- Onde é usado e comparação de revisões.

## O que evitar

- Servidor obrigatório para o uso individual.

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| PLM-01 | Versões e revisões | ✅ | fenkra-project: Version, Revision, next_label. |  |
| PLM-02 | Estados de ciclo de vida | 🟡 | DocState sem fluxo de aprovação. | CTX-PLM |
| PLM-03 | Banco de produtos e estrutura (BOM) | 🟡 | Índice JSON com hash blake3; sem banco nem estrutura de produto. | CTX-PLM |
| PLM-04 | Check-in/out e multiusuário | ❌ |  | CTX-PLM |
| PLM-05 | Onde é usado e comparação | ❌ |  | CTX-PLM |
