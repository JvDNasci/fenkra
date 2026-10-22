# Checklist de features

Lista de **todas as features**, uma por linha, para montar e conferir cada uma. Cada feature só é
dada como pronta quando passa pelas quatro etapas:

1. **Req** — requisitos aprovados (status `aprovado` nos arquivos de requisitos)
2. **Spec** — ficha escrita em `docs/specs/` a partir do [modelo](../specs/_modelo.md)
3. **Impl** — implementada
4. **Teste** — testes de aceitação da ficha passando

Marque `[x]` em cada etapa concluída. A coluna **Fase** segue o [roadmap](../roadmap.md).

## Fundação

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Árvore: Origem (ponto, eixos, planos XY/XZ/YZ) | RF-ARV-001..003 | 1 | [ ] | [ ] | [ ] | [ ] |
| Árvore: pasta Referências | RF-ARV-010..013 | 1 | [ ] | [ ] | [ ] | [ ] |
| Árvore: pasta Parâmetros | RF-ARV-020..023 | 1 | [ ] | [ ] | [ ] | [ ] |
| Árvore: pastas personalizadas | RF-ARV-030..033 | 1 | [ ] | [ ] | [ ] | [ ] |
| Coletores de corpos | RF-ARV-040..046 | 1 | [ ] | [ ] | [ ] | [ ] |
| Tipos de documento | RF-NUC-001 | 1 | [ ] | [ ] | [ ] | [ ] |
| Edição, supressão e diagnóstico de comandos | RF-NUC-003..009 | 1 | [ ] | [ ] | [ ] | [ ] |
| **Edição não destrutiva e substituir referência** | RF-NUC-020..025 | 1 | [ ] | [ ] | [ ] | [ ] |
| Linha de tempo de comandos e rollback | RF-ARV-050..057, RF-NUC-002 | 1 | [ ] | [ ] | [ ] | [ ] |
| Recompute incremental em background | RNF-001, RNF-002 | 1 | [ ] | [ ] | [ ] | [ ] |
| Referências topológicas persistentes | RNF-008 | 1 | [ ] | [ ] | [ ] | [ ] |
| Parâmetros e expressões | RF-PAR-001..010 | 1 | [ ] | [ ] | [ ] | [ ] |
| Propriedades (PN, descrição, material, tipo, personalizadas) | RF-PAR-020..026 | 1 | [ ] | [ ] | [ ] | [ ] |
| Biblioteca de materiais | RF-PAR-024, RF-BIB-016 | 1 | [ ] | [ ] | [ ] | [ ] |
| Geometria mestre e publicações | RF-LNK-001..004 | 1 | [ ] | [ ] | [ ] | [ ] |
| Links entre documentos | RF-LNK-010..017 | 1 | [ ] | [ ] | [ ] | [ ] |
| **Variáveis geométricas referenciáveis** | RF-LNK-030..037 | 1 | [ ] | [ ] | [ ] | [ ] |
| Gerenciador de links | RF-LNK-020..023 | 1 | [ ] | [ ] | [ ] | [ ] |
| Projeto em pasta e versões automáticas | RF-GPR-001, RF-GPR-010, RF-GPR-021 | 1 | [ ] | [ ] | [ ] | [ ] |
| Interface minimalista e painel de feature | RF-UI-001..006 | 1 | [ ] | [ ] | [ ] | [ ] |
| Estilos de exibição | RF-UI-015 | 1 | [ ] | [ ] | [ ] | [ ] |
| Import/export STEP, IGES, BREP | RF-NUC-010 | 1 | [ ] | [ ] | [ ] | [ ] |
| Import/export STL, OBJ, 3MF, PLY | RF-NUC-011 | 1 | [ ] | [ ] | [ ] | [ ] |
| Propriedades de massa e medição | RF-NUC-013, RF-NUC-014 | 1 | [ ] | [ ] | [ ] | [ ] |
| Import/export DXF e SVG | RF-NUC-012 | 2 | [ ] | [ ] | [ ] | [ ] |
| Seção dinâmica | RF-NUC-015 | 2 | [ ] | [ ] | [ ] | [ ] |
| DWG (avaliar) | RF-NUC-016 | 4 | [ ] | [ ] | [ ] | [ ] |
| Desfazer/refazer, salvamento automático | RNF-007, RNF-013 | 1 | [ ] | [ ] | [ ] | [ ] |

## Sketch

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Sketch 2D: entidades | RF-ESB-001, 002 | 1 | [ ] | [ ] | [ ] | [ ] |
| Restrições geométricas e dimensionais | RF-ESB-004..009, 014, 017 | 1 | [ ] | [ ] | [ ] | [ ] |
| Ferramentas de sketch (aparar, offset...) | RF-ESB-010, 011 | 1 | [ ] | [ ] | [ ] | [ ] |
| Projetar arestas / elementos publicados | RF-ESB-012 | 1 | [ ] | [ ] | [ ] | [ ] |
| Detecção de perfis fechados | RF-ESB-013 | 1 | [ ] | [ ] | [ ] | [ ] |
| Splines | RF-ESB-003 | 2 | [ ] | [ ] | [ ] | [ ] |
| Sketch 3D | RF-ESB-015 | 2 | [ ] | [ ] | [ ] | [ ] |
| Texto em sketch | RF-ESB-016 | 3 | [ ] | [ ] | [ ] | [ ] |

## Modelagem Sólida

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Extrusão | RF-SOL-001, 002 | 1 | [ ] | [ ] | [ ] | [ ] |
| Revolução | RF-SOL-002, 003 | 1 | [ ] | [ ] | [ ] | [ ] |
| Recorte (cutout) | RF-SOL-004 | 1 | [ ] | [ ] | [ ] | [ ] |
| Ângulo de saída (draft) | RF-SOL-005 | 1 | [ ] | [ ] | [ ] | [ ] |
| Nervura por caminho | RF-SOL-006 | 1 | [ ] | [ ] | [ ] | [ ] |
| Furo | RF-SOL-007 | 1 | [ ] | [ ] | [ ] | [ ] |
| Furo inteligente | RF-SOL-008, 009 | 1–2 | [ ] | [ ] | [ ] | [ ] |
| Rosca paramétrica | RF-SOL-010..012, RF-BIB-014 | 1 | [ ] | [ ] | [ ] | [ ] |
| Filete | RF-SOL-013 | 1 | [ ] | [ ] | [ ] | [ ] |
| Chanfro | RF-SOL-014 | 1 | [ ] | [ ] | [ ] | [ ] |
| Casca | RF-SOL-015 | 2 | [ ] | [ ] | [ ] | [ ] |
| Padrões e espelhar | RF-SOL-016 | 2 | [ ] | [ ] | [ ] | [ ] |
| Booleanas entre corpos/coletores | RF-SOL-017, RF-ARV-043 | 2 | [ ] | [ ] | [ ] | [ ] |
| Varredura | RF-SOL-018 | 2 | [ ] | [ ] | [ ] | [ ] |
| Operações sólido × superfície | RF-SOL-021 | 2 | [ ] | [ ] | [ ] | [ ] |
| Filete variável | RF-SOL-019 | 3 | [ ] | [ ] | [ ] | [ ] |
| Modelagem direta | RF-SOL-020 | 3 | [ ] | [ ] | [ ] | [ ] |

## Superfícies Complexas

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Curvas 3D e continuidade | RF-SUP-001..003 | 2 | [ ] | [ ] | [ ] | [ ] |
| Loft | RF-SUP-010 | 2 | [ ] | [ ] | [ ] | [ ] |
| Extrusão/revolução/varredura de superfície | RF-SUP-011 | 2 | [ ] | [ ] | [ ] | [ ] |
| Preenchimento (fill) | RF-SUP-012 | 2 | [ ] | [ ] | [ ] | [ ] |
| Aparar, estender, costurar, offset | RF-SUP-014, 015 | 2 | [ ] | [ ] | [ ] | [ ] |
| Análise de zebra | RF-SUP-020 | 2 | [ ] | [ ] | [ ] | [ ] |
| Análise de desvio | RF-SUP-021 | 2 | [ ] | [ ] | [ ] | [ ] |
| Verificação de continuidade | RF-SUP-024 | 2 | [ ] | [ ] | [ ] | [ ] |
| Mapas de curvatura e de ângulo de saída | RF-SUP-022, 023 | 2 | [ ] | [ ] | [ ] | [ ] |
| Blend | RF-SUP-013 | 3 | [ ] | [ ] | [ ] | [ ] |
| Superfícies por pontos de controle | RF-SUP-030..033 | 5 | [ ] | [ ] | [ ] | [ ] |

## Montagem e Mecanismos

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Inserir componentes, fixo/flutuante, GDL | RF-MON-001..003 | 2 | [ ] | [ ] | [ ] | [ ] |
| Restrições geométricas | RF-MON-010, 017 | 2 | [ ] | [ ] | [ ] | [ ] |
| Juntas e limites | RF-MON-011, 012 | 2 | [ ] | [ ] | [ ] | [ ] |
| Drives de mecanismo | RF-MON-014, 015 | 2 | [ ] | [ ] | [ ] | [ ] |
| Rosca entre peças | RF-MON-020, RF-SOL-009 | 2 | [ ] | [ ] | [ ] | [ ] |
| Peça no contexto + links | RF-MON-022, RF-LNK-014 | 2 | [ ] | [ ] | [ ] | [ ] |
| Modos visualizar / contexto / editar | RF-UI-010..014 | 2 | [ ] | [ ] | [ ] | [ ] |
| Padrões de componentes | RF-MON-004 | 2 | [ ] | [ ] | [ ] | [ ] |
| Interferência | RF-MON-023 | 2 | [ ] | [ ] | [ ] | [ ] |
| Modos e parâmetros de montagem | RF-MON-005, 006 | 2 | [ ] | [ ] | [ ] | [ ] |
| Acoplamentos (engrenagem, cremalheira, came) | RF-MON-013 | 3 | [ ] | [ ] | [ ] | [ ] |
| Colisão dinâmica | RF-MON-016 | 3 | [ ] | [ ] | [ ] | [ ] |
| Fixadores automáticos | RF-MON-021 | 3 | [ ] | [ ] | [ ] | [ ] |
| Vista explodida | RF-MON-024 | 3 | [ ] | [ ] | [ ] | [ ] |
| Substituir componente | RF-MON-025 | 3 | [ ] | [ ] | [ ] | [ ] |

## Desenho Técnico

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Folhas, carimbo, normas | RF-DES-001..004 | 2 | [ ] | [ ] | [ ] | [ ] |
| Vista como bloco ligada/travada | RF-DES-010, 011 | 2 | [ ] | [ ] | [ ] | [ ] |
| Vistas padrão, projetadas, auxiliares, detalhe | RF-DES-012, 013, 015, 016 | 2 | [ ] | [ ] | [ ] | [ ] |
| Cortes e seções | RF-DES-014 | 2 | [ ] | [ ] | [ ] | [ ] |
| Cotagem associativa | RF-DES-020..024 | 2 | [ ] | [ ] | [ ] | [ ] |
| Hachuras e notas | RF-DES-026, 027 | 2 | [ ] | [ ] | [ ] | [ ] |
| Geometria 2D e transformações | RF-DES-030, 031, 034 | 2 | [ ] | [ ] | [ ] | [ ] |
| BOM e balões | RF-DES-040..042 | 2 | [ ] | [ ] | [ ] | [ ] |
| Tabelas (corte, dobras, furos, revisões) | RF-DES-043, 044 | 2 | [ ] | [ ] | [ ] | [ ] |
| Vista planificada de chapa | RF-DES-017 | 2 | [ ] | [ ] | [ ] | [ ] |
| Exportação PDF/DXF/SVG/PNG | RF-DES-045 | 2 | [ ] | [ ] | [ ] | [ ] |
| Vistas de montagem e explodida | RF-DES-018 | 3 | [ ] | [ ] | [ ] | [ ] |
| GD&T, tolerâncias, símbolos | RF-DES-025 | 3 | [ ] | [ ] | [ ] | [ ] |
| Restrições 2D e blocos 2D | RF-DES-032, 033 | 3 | [ ] | [ ] | [ ] | [ ] |

## Chapas Metálicas

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Parâmetros de chapa | RF-CHP-001, RF-BIB-017 | 2 | [ ] | [ ] | [ ] | [ ] |
| Flange base | RF-CHP-002 | 2 | [ ] | [ ] | [ ] | [ ] |
| Flange de aresta (incl. perfil próprio) | RF-CHP-003, 004 | 2 | [ ] | [ ] | [ ] | [ ] |
| Dobra por linha | RF-CHP-007 | 2 | [ ] | [ ] | [ ] | [ ] |
| Alívios e cantos | RF-CHP-008 | 2 | [ ] | [ ] | [ ] | [ ] |
| Recorte normal | RF-CHP-010 | 2 | [ ] | [ ] | [ ] | [ ] |
| Planificação + DXF | RF-CHP-012, 016 | 2 | [ ] | [ ] | [ ] | [ ] |
| Ligação com mestre | RF-CHP-014 | 2 | [ ] | [ ] | [ ] | [ ] |
| Flange contínua sobre superfícies | RF-CHP-005 | 3 | [ ] | [ ] | [ ] | [ ] |
| Flange de contorno | RF-CHP-006 | 3 | [ ] | [ ] | [ ] | [ ] |
| Bainha, jog, estampos | RF-CHP-009 | 3 | [ ] | [ ] | [ ] | [ ] |
| Converter sólido em chapa | RF-CHP-011 | 3 | [ ] | [ ] | [ ] | [ ] |
| Dobrar/desdobrar parcial | RF-CHP-013 | 3 | [ ] | [ ] | [ ] | [ ] |
| Verificação de fabricação | RF-CHP-015 | 3 | [ ] | [ ] | [ ] | [ ] |

## Estruturas Soldadas

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Membro estrutural | RF-SDA-001, 002, 004, 013 | 2 | [ ] | [ ] | [ ] | [ ] |
| Cadastro de perfis | RF-SDA-003, RF-BIB-010..012 | 2 | [ ] | [ ] | [ ] | [ ] |
| Uniões e gerenciador de uniões | RF-SDA-005..007 | 2 | [ ] | [ ] | [ ] | [ ] |
| Lista de corte e pesos | RF-SDA-010..012 | 2 | [ ] | [ ] | [ ] | [ ] |
| Atualização pelo esqueleto/mestre | RF-SDA-014 | 2 | [ ] | [ ] | [ ] | [ ] |
| Gussets, tampas, placas de base | RF-SDA-008 | 3 | [ ] | [ ] | [ ] | [ ] |
| Cordões de solda | RF-SDA-009 | 3 | [ ] | [ ] | [ ] | [ ] |
| Otimização de barras | RF-SDA-015 | 4 | [ ] | [ ] | [ ] | [ ] |

## Gestão de Projeto

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Estrutura de produto | RF-GPR-002..006 | 2 | [ ] | [ ] | [ ] | [ ] |
| Revisões congeladas | RF-GPR-011..017 | 2 | [ ] | [ ] | [ ] | [ ] |
| Gestão de links do projeto | RF-GPR-020, 022, 023 | 2 | [ ] | [ ] | [ ] | [ ] |
| Pacote do projeto | RF-GPR-024 | 2 | [ ] | [ ] | [ ] | [ ] |
| Bloqueio em pasta compartilhada | RF-GPR-025 | 3 | [ ] | [ ] | [ ] | [ ] |

## Biblioteca pública

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Formato e camadas da biblioteca | RF-BIB-001, 003, 004, 006 | 1 | [ ] | [ ] | [ ] | [ ] |
| Roscas | RF-BIB-014 | 1 | [ ] | [ ] | [ ] | [ ] |
| Perfis estruturais com peso | RF-BIB-010..012 | 2 | [ ] | [ ] | [ ] | [ ] |
| Fixadores | RF-BIB-013 | 2 | [ ] | [ ] | [ ] | [ ] |
| Edição comunitária | RF-BIB-002 | 2 | [ ] | [ ] | [ ] | [ ] |
| Elementos de máquina | RF-BIB-015 | 4 | [ ] | [ ] | [ ] | [ ] |
| Cadastro pela interface, offline, versão fixada | RF-BIB-005, 007, 008 | 2 | [ ] | [ ] | [ ] | [ ] |

## Malha → sólido

| Feature | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Importar e reparar malha | RF-MAL-001, 002 | 2 | [ ] | [ ] | [ ] | [ ] |
| Conversão nível 1 e 2 | RF-MAL-003, 004, 006..008 | 2 | [ ] | [ ] | [ ] | [ ] |
| Conversão nível 3 (primitivas) | RF-MAL-005 | 4 | [ ] | [ ] | [ ] | [ ] |
| Superfícies livres (fora do escopo) | RF-MAL-009 | — | [ ] | [ ] | [ ] | [ ] |

## Requisitos não-funcionais (verificados continuamente)

| Item | Requisitos | Fase | Req | Spec | Impl | Teste |
|---|---|---|---|---|---|---|
| Desempenho de viewport e inicialização | RNF-003, 004 | 1 | [ ] | [ ] | [ ] | [ ] |
| Núcleo headless / WASM | RNF-005 | 1 | [ ] | [ ] | [ ] | [ ] |
| Formato de arquivo aberto e versionado | RNF-006 | 1 | [ ] | [ ] | [ ] | [ ] |
| Plataformas e unidades | RNF-009, 014 | 1 | [ ] | [ ] | [ ] | [ ] |
| i18n português/inglês | RNF-010 | 1 | [ ] | [ ] | [ ] | [ ] |
| API Python | RNF-011 | 2 | [ ] | [ ] | [ ] | [ ] |
| Testes de regressão geométrica | RNF-012 | 1 | [ ] | [ ] | [ ] | [ ] |
| Arquitetura modular | RNF-015 | 1 | [ ] | [ ] | [ ] | [ ] |
| Montagens grandes e desenhos com vistas travadas | RNF-016, 017 | 2 | [ ] | [ ] | [ ] | [ ] |
