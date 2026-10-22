# Requisitos — Árvore da peça e linha de tempo de comandos

A árvore organiza **o que existe** no documento. A linha de tempo mostra **em que ordem** os comandos
são executados. São duas vistas do mesmo modelo: reorganizar a árvore (pastas, coletores) **não**
muda a ordem de execução, a não ser quando o usuário reordena explicitamente na linha de tempo.

## Estrutura fixa da árvore (todo documento de peça)

```
Peça
├── Origem
│   ├── Ponto de origem
│   ├── Eixos X, Y, Z
│   └── Planos XY, XZ, YZ
├── Referências          (pasta de trabalho de referências)
├── Parâmetros           (pasta de trabalho de parâmetros)
├── Coletores de corpos
│   └── Corpo principal ...
├── [pastas personalizadas]
└── [features]
```

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-ARV-001 | Todo documento de peça e de geometria mestre nasce com o nó **Origem**: ponto de origem, eixos X/Y/Z e planos **XY, XZ, YZ**. | M | 1 | rascunho |
| RF-ARV-002 | Os elementos da Origem não podem ser apagados nem movidos; podem ser ocultados/exibidos e renomeados na exibição. | M | 1 | rascunho |
| RF-ARV-003 | A Origem pode ser usada como entrada de qualquer feature (plano de sketch, eixo de revolução, referência de restrição). | M | 1 | rascunho |

## Pasta de trabalho de Referências

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-ARV-010 | Pasta fixa **Referências** que contém a geometria de referência criada pelo usuário: planos, eixos, pontos, sistemas de coordenadas, curvas e superfícies auxiliares. | M | 1 | rascunho |
| RF-ARV-011 | Geometria que **entra por link** (de geometria mestre ou de outra peça) aparece em Referências, com ícone do estado do link (ativo, congelado, quebrado). | M | 1 | rascunho |
| RF-ARV-012 | Elementos **publicados** por esta peça são marcados em Referências (ou na feature de origem) com indicador de publicação. | M | 1 | rascunho |
| RF-ARV-013 | Subpastas dentro de Referências. | S | 1 | rascunho |

## Pasta de trabalho de Parâmetros

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-ARV-020 | Pasta fixa **Parâmetros** listando os parâmetros do documento (nome = valor / expressão), editáveis direto na árvore. | M | 1 | rascunho |
| RF-ARV-021 | Parâmetros que vêm por link aparecem nessa pasta com indicador de link e são somente leitura localmente. | M | 1 | rascunho |
| RF-ARV-022 | Subpastas para agrupar parâmetros (ex.: "Dimensões gerais", "Folgas"). | S | 1 | rascunho |
| RF-ARV-023 | Propriedades da peça (PN, descrição, material, tipo, personalizadas) acessíveis pelo nó raiz da peça. | M | 1 | rascunho |

## Pastas personalizadas

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-ARV-030 | O usuário cria pastas com nome livre para agrupar features (ex.: "Furação", "Acabamentos"). | M | 1 | rascunho |
| RF-ARV-031 | Pastas podem ser aninhadas, recolhidas, renomeadas e coloridas. | S | 1 | rascunho |
| RF-ARV-032 | Mover uma feature para uma pasta **não altera** a ordem de execução. | M | 1 | rascunho |
| RF-ARV-033 | Ações em lote na pasta: suprimir, ocultar, isolar, apagar (com confirmação e lista de dependências). | M | 1 | rascunho |

## Coletores de corpos

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-ARV-040 | **Coletor de corpos**: contêiner nomeado que agrupa corpos sólidos e/ou superfícies da peça. | M | 1 | rascunho |
| RF-ARV-041 | Toda peça nasce com um coletor "Corpo principal". O usuário cria outros coletores. | M | 1 | rascunho |
| RF-ARV-042 | **Coletor ativo**: novas features de sólido atuam no coletor ativo; cada feature pode escolher outro coletor como destino. | M | 1 | rascunho |
| RF-ARV-043 | Operações entre coletores: unir, subtrair, interseção (booleanas entre corpos, RF-SOL-017). | M | 2 | rascunho |
| RF-ARV-044 | Cada coletor/corpo tem propriedades próprias (material, PN, descrição) e pode virar peça separada ou item da BOM (soldas, multi-corpo). | M | 2 | rascunho |
| RF-ARV-045 | Coletor pode ser publicado e ligado em outra peça (link tipo corpo, RF-LNK-010). | M | 2 | rascunho |
| RF-ARV-046 | Exibir/ocultar, cor e transparência por coletor. | M | 1 | rascunho |

## Linha de tempo de comandos

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-ARV-050 | **Linha de tempo** horizontal com todos os comandos na ordem de execução, cada um com ícone e nome. | M | 1 | rascunho |
| RF-ARV-051 | **Marcador de rollback** arrastável: comandos depois dele ficam temporariamente desativados; criar comando novo insere no ponto do marcador. | M | 1 | rascunho |
| RF-ARV-052 | Selecionar um comando na linha de tempo destaca o item na árvore e vice-versa. | M | 1 | rascunho |
| RF-ARV-053 | Reordenar comandos arrastando na linha de tempo, bloqueando quando uma dependência seria violada, com explicação. | S | 2 | rascunho |
| RF-ARV-054 | Agrupar comandos na linha de tempo (grupos recolhíveis), espelhando as pastas personalizadas quando o usuário quiser. | S | 2 | rascunho |
| RF-ARV-055 | Indicadores na linha de tempo: erro, aviso, suprimido, entrada por link desatualizada. | M | 1 | rascunho |
| RF-ARV-056 | Editar um comando por duplo clique na linha de tempo ou na árvore abre o mesmo painel de feature (RF-UI-002). | M | 1 | rascunho |
| RF-ARV-057 | Linha de tempo também em montagens (inserções, restrições, features de montagem) e na geometria mestre. | M | 2 | rascunho |
