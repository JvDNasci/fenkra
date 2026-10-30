# Fontes de dados da biblioteca (perfis e hardware mecânico)

Lista de fontes indicada pelo usuário em 29/10/2026 ([contexto](../contextos/hardwares.md)), com o que cada uma permite.
Segue a [ADR 0008](../arquitetura/adr/0008-biblioteca-publica.md): **medidas são fatos** e podem ser registradas, mas
**não se copiam texto, figuras nem tabelas formatadas**. Cada família guarda a norma de referência, a fonte das medidas e a
licença.

## Níveis de uso

| Nível | O que é | Como usar |
|---|---|---|
| **A — livre** | Domínio público ou licença compatível | Pode ser a fonte principal. Registrar o documento e a revisão. |
| **B — catálogo público** | Catálogo/planilha gratuita do fabricante ou entidade, com direitos sobre a apresentação | Extrair **só medidas**, família por família, citando a página. Conferir pelo menos uma amostra em outra fonte. Não reproduzir o catálogo inteiro de uma vez (direito de base de dados na UE). |
| **C — norma paga** | ISO, DIN, EN, ABNT, ASTM, JIS, AS/NZS, NAS | O texto não está disponível. A norma vira só a **referência** (o nome da família); as medidas vêm de fontes A/B que publicam a mesma norma, conferidas em **duas fontes independentes**. |
| **X — só conferência** | Licença incompatível com o Fenkra (GPL) ou termos desconhecidos | Pode ser usada para conferir valores, nunca como fonte de cópia. |

## Perfis metálicos

| Fonte | Conteúdo | Nível | Observação |
|---|---|---|---|
| Instituto Aço Brasil | Perfis laminados, soldados, tubos (NBR 7007, 7008, 8261, 8262, 8263) | B | NBRs são pagas (C); usar as tabelas públicas do Instituto |
| Gerdau — catálogo técnico | Perfis W/H (laminados no Brasil), U, L, barras, tubos | B | Fonte principal dos perfis W brasileiros |
| ArcelorMittal — perfis e tubos | IPE, HEA, HEB, HEM, UPN, UPE, cantoneiras | B | Empresa europeia: extrair por família, citando |
| Vallourec | Tubos estruturais e mecânicos | B | |
| AISC Shapes Database v16.0 | W, HP, S, C, MC, L, HSS (ASTM A500) | B (termos a confirmar) | Planilha gratuita em aisc.org; a página de termos não abriu (HTTP 403) |
| EN 10365 / EN 10210 / EN 10219 | IPE/HE/UPN; tubos a quente e a frio | C | Medidas via ArcelorMittal e catálogos de tubos |
| ASTM A500/A513/A106; A36/A572/A992 | Tubos e aços | C | Aços: só propriedades nominais públicas (fy, fu) |
| JIS G3192, G3444, G3452 | Perfis H/I/C e tubos japoneses | C | Precisa de fonte pública de fabricante japonês |
| AS/NZS 3679, 1163 | Perfis e tubos australianos | C | Precisa de fonte pública (ex.: catálogo de fabricante australiano) |

## Fixadores industriais

| Fonte | Conteúdo | Nível | Observação |
|---|---|---|---|
| ISO 4014/4017, 4032, 4762, 7089/7090; DIN 931/933/934/912/125 | Parafusos, porcas, arruelas | C | Medidas via catálogos B (Ciser etc.) |
| ABNT NBR 5580, 5590, 6323, 8160 | Parafusos, porcas, tubos, arruelas | C | Conferir se os números ainda são vigentes antes de citar |
| Ciser, Brasfixo, Vonder, Belzer/Gedore | Catálogos de fabricante | B | Ciser é a fonte principal (mais completa) |
| FreeCAD Fasteners Workbench | ~150 normas ISO/DIN/ASME | X | **GPLv2**: só para conferência |

## Fixadores aeronáuticos e militares

| Fonte | Conteúdo | Nível | Observação |
|---|---|---|---|
| MS (MS20004, MS20073, MS21250, MS21042, MS20002, MS20392…) | Parafusos, porcas, arruelas, pinos | A | Normas do governo dos EUA no ASSIST (quicksearch.dla.mil); usar só as com *Distribution Statement A* |
| AN (AN3–AN10, AN310, AN315, AN470, AN426) | Parafusos, porcas, rebites | A | Idem; várias foram substituídas por MS/NASM: registrar a revisão |
| MIL-HDBK-60, MIL-DTL-1222, MIL-DTL-18240 | Manual e especificações de fixadores | A | Idem |
| NAS (AIA) | Parafusos, porcas travantes, insertos, rebites | C | Normas pagas da AIA; medidas só por fontes públicas equivalentes (NASM no ASSIST, quando houver) |

## Elementos de máquina

| Fonte | Conteúdo | Nível | Observação |
|---|---|---|---|
| SKF, FAG/INA (Schaeffler), NSK | Rolamentos | B | Usar as **medidas de contorno ISO 15** (d, D, B, r), que são as mesmas em todos os fabricantes; capacidade de carga é dado do fabricante (citar) |
| Festo, Parker, SMC | Cilindros e atuadores pneumáticos | B | Começar pelos cilindros **ISO 15552/6432** (interfaces padronizadas); séries próprias ficam como camada de fabricante |
| Bosch Rexroth, THK, Hiwin | Guias lineares e fusos | B | Medidas são de cada fabricante (não há norma comum de guia): cada um vira uma família própria, com o nome da série |

## Famílias importadas

Arquivos em [`biblioteca/fixadores/`](../../biblioteca/fixadores/) (formato aberto, um JSON por família, licença dos dados
CC BY 4.0). Todas são **nível C** (normas pagas: só o nome da norma é referência; as medidas vêm de catálogos públicos
gratuitos, em duas fontes salvo onde indicado). Carregador: `crates/fenkra-solidos/src/biblioteca.rs`; geometria
simplificada: `fixadores.rs`. Modelam-se M3, M4, M5, M6, M8, M10, M12, M16 e M20.

| Arquivo | Norma | Tipo | Campos (mm) | Fontes |
|---|---|---|---|---|
| `iso4762.json` | ISO 4762 (DIN 912) | Parafuso Allen | P, dk, k, s, b | fasten.it (ISO 4762), tspi.at, opac.net |
| `iso4017.json` | ISO 4017 | Parafuso sextavado | P, s, k | fasten.it (ISO 4017), wermac.org |
| `din933.json` | DIN 933 | Parafuso sextavado | P, s, k | fasten.it, wermac.org |
| `iso4032.json` | ISO 4032 | Porca sextavada | s, m | fasten.it, torqbolt.com, tspi.at |
| `din934.json` | DIN 934 | Porca sextavada | s, m | fasten.it, torqbolt.com, tspi.at |
| `iso7089.json` | ISO 7089 (DIN 125 A) | Arruela lisa | d1, d2, h | roymech.org (DIN 125), fasten.it, torqbolt.com |

Legenda: P passo; dk Ø da cabeça; k altura da cabeça; s abertura (entre faces) da chave ou do sextavado interno; b comprimento
mínimo de rosca (L <= 125); m altura da porca; d1 Ø interno e d2 Ø externo da arruela; h espessura. O canto a canto do
sextavado (e = s / cos 30°) **não** está nos arquivos: é calculado na geometria. Os parafusos trazem também a série de
comprimentos L padrão (6 a 120 mm); para cada tamanho valem `2·d <= L <= 10·d`, com mínimo de 6 mm.

**Valores com uma só fonte** (campo `confirmado_por: 1`, com `nota` no arquivo; conferir com uma segunda fonte):

| Família | Tamanho | O que |
|---|---|---|
| ISO 4762 | M16, M20 | `b`: uma fonte + regra b = 2d + 12 (as demais medidas da linha foram tratadas como o resto da família) |
| ISO 4032 | M6 | linha confirmada só por torqbolt.com |

**Pendências de fonte**: três URLs de página não foram dadas no levantamento e foram montadas pelo padrão do site (fasten.it
ISO 4032 e ISO 7089; torqbolt ISO 7089), e as páginas específicas de DIN 933 e DIN 934 no fasten.it ainda não foram
identificadas (os arquivos apontam para as páginas ISO equivalentes). Todas estão marcadas no `rotulo` de cada fonte e devem
ser abertas e conferidas antes de tirar o aviso.

## Regras para cada família importada

1. Arquivo da família com `norma`, `fonte` (documento, página, revisão), `nivel` (A/B/C) e `licenca`.
2. Para perfis, as propriedades (área, peso por metro, inércias) são **calculadas** pela geometria e conferidas com o valor
   publicado (tolerância de 2%). Um teste falha se a diferença passar disso.
3. Para fixadores, cada tamanho tem as medidas mínimas para modelar e furar: diâmetro, passo, cabeça (largura e altura),
   comprimentos de linha e furo de passagem/broca.
4. Nomes de fabricantes e séries aparecem só como referência; nada de logos ou imagens de catálogo.
