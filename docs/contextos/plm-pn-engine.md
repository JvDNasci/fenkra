# Contexto: VPM em pasta compartilhada e regras de part-number (PNEngine)

Texto trazido pelo usuário em 29/10/2026 (gerado com assistente de IA). Resumo:

## Modo pasta (LAN / OneDrive)
- O CAD lê e grava JSON e arquivos numa pasta; a sincronização (OneDrive, pasta de rede) fica por conta do sistema.
- Sem servidor. Conflitos tratados por um `lock.json` (check-out por arquivo).
- Módulos propostos: **VPMReader** (lê itens, BOM, revisões, normas, ambiente e mostra num painel lateral),
  **VPMWriter** (grava metadados, revisões, BOM, histórico, arquivos e gera códigos) e
  **VPMEnvironment** (escolhe a fonte de dados, detecta o modo local/LAN/OneDrive/servidor, permissões).
- Painel web opcional.

## Ambientes
```
/VPM/Environments/
    /Industrial/     PartNumberRules.json  Items/  BOM/
    /Aeroespacial/   PartNumberRules.json  Items/  BOM/
    /ClienteX/       PartNumberRules.json  Items/  BOM/
```
O usuário escolhe o ambiente no CAD; cada ambiente tem as suas regras e contadores.

## PNEngine (regras de part-number embutidas)
- **Editor de regras** no CAD, com campos entre colchetes:
  `AERO-[NORMA]-[CODIGO]-[REV]`, `MECH-[TIPO]-[MATERIAL]-[DIM]-[REV]`,
  `MIL-[MS]-[DIAM]-[COMPR]-[REV]`, `[SETOR]-[CATEGORIA]-[NORMA]-[SERIAL:5]-[REV]`.
- **Interpretador:** preenche os campos com dados da peça (norma, material, dimensões, categoria, revisão) e gera o código.
  Ex.: `AERO-[NORMA]-[CODIGO]-[REV]` com NAS, 620-3, B → `AERO-NAS-620-3-B`.
- **Armazenamento** em JSON por ambiente:
```json
{ "nome": "Aeroespacial", "padrao": "AERO-[NORMA]-[CODIGO]-[REV]",
  "campos": { "NORMA": ["NAS", "MS", "AN", "MIL"], "REV": "auto", "CODIGO": "auto" } }
```
- **Tipos de campo:** por norma (NAS, MS, AN, MIL-SPEC, DIN, ISO, ABNT), categoria (mecânico, estrutural, tubulação,
  elétrico, aeroespacial), material, dimensões (diâmetro, comprimento, espessura) e serial (auto-incremento por ambiente
  ou por categoria).
- Histórico opcional em SQLite.

## O que já existe no Fenkra (alpha.26)
`fenkra-project`: PN automático com prefixo fixo e contador (`FK-0001`), bloqueio de PN duplicado, estados e
revisões; índice `projeto.fenkra.json`. Não há regras com campos, ambientes nem trava para pasta compartilhada.
