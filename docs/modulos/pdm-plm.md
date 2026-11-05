# PDM / PLM

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/pdm-plm.md](../benchmark/pdm-plm.md).

## ✅ Pronto

- **Versões e revisões** (Projeto em pasta) — fenkra-project: Version (com autor e comentário), Revision, restaurar versão como versão nova, diário só-acréscimo (`.diario.jsonl`). ADR 0018.
- **Estados com tabela de transições** — `estados::Ciclo` em JSON (modelo de três estados e modelo com aprovação), papel e motivo por transição, esquema de revisão (letras, números, prévia 0.x) e lista de conferência na liberação.
- **Regras de part-number por máscara e ambientes** — `pn`: `{PREFIXO}`, `{ANO}`, `{TIPO}`, `{AMB}`, `{CAMPO:nome}`, `{SEQ:N}`; prévia ao vivo em cinza; o contador só avança depois de gravar.
- **Salvar como com referências e Empacotar** — `salvar_como` (4 modos, tabela de referências, conflitos) e `pacote` (pasta ou zip, manifesto, colisões listadas, verificação reabrindo o pacote).
- **Onde é usado e reparar links** — `relacoes`: lista de quem usa a peça (com o nó da montagem), cabeçalhos sem abrir o documento, diagnóstico e reparo de referências.

## 🟡 Parcial

- **Estados de ciclo de vida** (Estados) — DocState sem fluxo de aprovação. `CTX-PLM`
- **Banco de produtos e estrutura (BOM)** (Índice JSON) — Índice JSON com hash blake3; sem banco nem estrutura de produto. `CTX-PLM`
- **Regras de part-number e ambientes** (PN com prefixo fixo) — fenkra-project: prefixo FK- + contador, bloqueio de duplicado; sem regra com campos nem ambientes. `CTX-PN`
- **Cofre em pasta compartilhada** (Projeto em pasta (sem trava)) — Projeto em pasta já existe; falta trava e detecção de conflito (RF-GPR-025). `CTX-PN`

## ❌ Ainda não existe

- **Check-in/out e multiusuário** (—) —  `CTX-PLM`
- **Onde é usado e comparação** (—) —  `CTX-PLM`
