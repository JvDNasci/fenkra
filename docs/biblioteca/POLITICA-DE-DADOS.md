# Política de dados de normas e catálogos

Consolida a [ADR 0008](../arquitetura/adr/0008-biblioteca-publica.md) e
[fontes-de-dados.md](fontes-de-dados.md). Vale para todo JSON em `biblioteca/`. Não é parecer jurídico;
os pontos de direito autoral e de normas pagas estão na lista do advogado (B207).

## 1. Fatos x tabelas

- **Medidas, pesos e propriedades são fatos** e podem ser registrados.
- **Nunca copiar**: o layout ou a ordem de uma tabela, as notas, o texto, as figuras e os desenhos de
  uma norma ou catálogo. Registrar o valor, refazer a estrutura com a nossa organização.

## 2. Níveis

| Nível | Significado | Uso |
|---|---|---|
| A | Domínio público ou licença compatível | Pode ser fonte principal |
| B | Catálogo ou planilha pública de fabricante ou entidade | Só medidas, família por família, citando a página |
| C | Norma paga (ISO, DIN, EN, ABNT, ASTM, JIS...) | A norma é só o nome da família; medidas vêm de fontes A/B |
| X | Licença incompatível (GPL) ou termos desconhecidos | Só conferência; nunca copiar |

## 3. Regras de fonte

1. **Uma fonte por valor**, registrada com documento, revisão e página.
2. **Nível C exige duas fontes independentes**, e ao menos uma não pode ser agregadora (site que
   apenas repete outra fonte). Valor com uma só fonte vai marcado como pendente.
3. **Proibido copiar de fonte GPL** (nível X), incluindo o FreeCAD Fasteners Workbench (GPL-2.0): serve
   só para conferir um valor depois de obtido por outra fonte.
4. Fonte vaga ("contexto do projeto", "tabela de referência") não é fonte. Toda fonte é um documento
   que outra pessoa consegue abrir.

## 4. Rótulo de licença nos JSON

Campos obrigatórios por família: `id`, `nome`, `norma`, `nivel`, `licenca`, `fontes` (cada uma com
documento e página ou URL) e `confirmado_por`. A licença dos dados do Fenkra hoje é o rótulo
`"CC BY 4.0 (dados)"`, ainda como **proposta do mantenedor** (ADR 0008). A atribuição exigida é
"Biblioteca de dados do Fenkra, CC BY 4.0", mais a lista de fontes do arquivo.

## 5. Termos de cada fonte

Guardar, para cada fonte nível B, uma cópia datada dos termos de uso vigentes (PDF ou captura da página)
em `docs/biblioteca/termos/<fonte>-AAAA-MM-DD.*`, com a data de consulta. Fontes a coletar: Gerdau,
Instituto Aço Brasil, AISC (a página de termos devolveu HTTP 403 na consulta anterior), ArcelorMittal,
Vallourec, Ciser, Brasfixo, Vonder e demais catálogos usados. Nenhuma cópia foi guardada ainda.

## 6. Validação automática (a fazer)

Um teste deve falhar se algum JSON da biblioteca não tiver fonte, página e nível válidos, se uma
família nível C tiver menos de duas fontes, ou se a fonte for vaga. O teste fica em `crates/` e não faz
parte deste arquivo.

## 7. Pendências conhecidas

- `biblioteca/chavetas/din6885a.json`: a fonte 1 ("contexto do projeto") é vaga e precisa ser
  substituída por uma fonte A ou B citável; t2 ainda tem fonte única.
- `docs/arquitetura/reuso-opensource.md` (fora dos arquivos desta tarefa): corrigir a entrada do Fasteners Workbench para GPL-2.0, nível X.
