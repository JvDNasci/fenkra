# Backlog

Consolidado do [design review](../contextos/design-review.md), dos [contextos](../contextos/) e do [benchmark](../benchmark/).
Versão visual (kanban + comparador): [painel.html](painel.html), gerada por `scripts/benchmark/pagina.py`.
Gerado por `scripts/benchmark/backlog.py`; o estado de cada item é atualizado aqui a cada entrega.

**Estimativa total:** ≈ 7.6 M tokens (≈ 25 mil linhas de código, pela média das últimas versões).

| Onda | Versão | Itens | Tokens | % |
|---|---|---|---|---|
| Onda 1 — bugs do design review | `0.1.0-alpha.27` | 15 | ≈ 1300 k | 17.0% |
| Onda 2 — filosofia base + VPM local | `0.1.0-alpha.28` | 12 | ≈ 1770 k | 23.2% |
| Onda 3 — módulos novos e lacunas do benchmark | `0.1.0-alpha.29+` | 20 | ≈ 4570 k | 59.8% |

Prioridade: **P0** quebra o uso · **P1** importante · **P2** desejável · **P3** futuro.
Teste: **Auto** = `cargo test` com documento montado em código · **Visual** = captura do app com `FENKRA_DEMO_*` · **Manual** = usuário.

## Onda 1 — bugs do design review → `0.1.0-alpha.27`

| ID | P | Item | Origem | Catálogo | Tokens | % | Teste | Estado |
|---|---|---|---|---|---|---|---|---|
| B01 | P0 | **Chapa: abas em sequência, múltiplas e vizinhas** — Corrigir dobras em sequência, várias dobras + outra lateral, abas compostas lado a lado (com validação de colisão) e aba em cadeia reta + curva. | DR-54, DR-55, DR-56, DR-57 | CHP-03, CHP-04 | 200 k | 2.6% | Auto + Visual | a fazer |
| B02 | P0 | **Chapa: recorte planificado → dobrado no raio** — O recorte feito no planificado deve voltar ao dobrado mesmo atravessando o raio de dobra. | DR-20 | CHP-11 | 60 k | 0.8% | Auto | a fazer |
| B03 | P0 | **Chapa: dobra por linha em spline** — Dobra por linha aceitando spline e emendando os raios. | DR-17, DR-22 | CHP-10 | 70 k | 0.9% | Auto | a fazer |
| B04 | P0 | **Chapa: aba por superfície (nervura de asa)** — Fazer a aba por superfície funcionar nos casos do usuário, incluindo a nervura de asa. | DR-19, DR-23 | CHP-06 | 90 k | 1.2% | Auto + Visual | a fazer |
| B05 | P0 | **Rosca: preview, personalização e início** — Preview ao vivo; passo, perfil e tamanho personalizados; deslocamento inicial e chanfro de entrada; corrigir o tap. | DR-02, DR-03 | SOL-07, GER-02 | 140 k | 1.8% | Auto | a fazer |
| B06 | P0 | **Casca: spline, furo escareado, opção reta** — Casca funcionando com spline e furo escareado; para fora com opção reta/raio editável; espessura por face. | DR-02, DR-07 | SOL-11 | 110 k | 1.4% | Auto | a fazer |
| B08 | P0 | **Sketch: tangência da spline e interseção** — Spline tangente a outros elementos; interseção com elementos de outro plano devolvendo todos os pontos. | DR-02, DR-06 | SKT-04, SKT-21 | 120 k | 1.6% | Auto | a fazer |
| B09 | P0 | **Fill, Join e Preencher com sketch** — Aceitar linhas de sketch e seleção em cadeia em Fill, Join e Preencher fronteiras. | DR-13 | SUP-12, SUP-15 | 140 k | 1.8% | Auto | a fazer |
| B07 | P1 | **Engrossar face de sólido** — Espessura aplicada a face de sólido. | DR-12 | SOL-14 | 40 k | 0.5% | Auto | a fazer |
| B10 | P1 | **Render: linhas indevidas e filete de superfície** — Remover riscos no render (como o modo sem arestas, mas com as arestas principais) e corrigir arestas em filetes de superfície. | DR-08, DR-10 | GER-06, SUP-19 | 60 k | 0.8% | Visual | a fazer |
| B11 | P1 | **Nervura com parede dos dois lados** — Espessura para um lado, para o outro ou simétrica. | DR-05 | SOL-13 | 40 k | 0.5% | Auto | a fazer |
| B12 | P1 | **Loft com restrições e tensão** — Restrição inicial/final por elemento, tensão de concordância, coletor com seleção múltipla. | DR-07 | SOL-05 | 110 k | 1.4% | Auto | a fazer |
| B13 | P1 | **Planos de face e plano na curva** — Plano de face entra em erro quando a face some; plano na curva por ponto contido ou projetado. | DR-21, DR-04 | SUP-03 | 60 k | 0.8% | Auto | a fazer |
| B14 | P1 | **Superfícies: "até elemento" e dois limites** — Extensões de superfície com até elemento; distâncias inicial e final juntas. | DR-24, DR-16 | SUP-09 | 60 k | 0.8% | Auto | a fazer |
| B41 | P1 | **Teste com o mouse (usuário)** — Pendências antigas que dependem do usuário: desenho por mouse, cotagem smart, menu rápido, arrastar na árvore, manipulador, bancadas. | histórico |  | 0 k | 0.0% | Manual | a fazer |

## Onda 2 — filosofia base + VPM local → `0.1.0-alpha.28`

| ID | P | Item | Origem | Catálogo | Tokens | % | Teste | Estado |
|---|---|---|---|---|---|---|---|---|
| B15 | P0 | **Referências v2 (ADR 0010)** — Referências por dono + papel + assinatura, com o ponto como reserva; re-anexar sozinho ou mostrar reparo com candidatos no 3D. | DR-11 | SOL-24, GER-05 | 400 k | 5.2% | Auto | a fazer |
| B16 | P0 | **Ocultar hierárquico** — O pai oculta os filhos e cada um guarda o próprio estado; ocultar a origem e os menus da árvore no próprio item. | DR-15, DR-30 | GER-01 | 80 k | 1.0% | Auto + Visual | a fazer |
| B17 | P1 | **Câmera no estilo CATIA** — Duplo clique do meio recentra por raycast; pivô sob o cursor (SW/Rhino); zoom no cursor; turntable (Inventor). | CTX-CAM | NAV-02, NAV-03, NAV-04 | 100 k | 1.3% | Visual | a fazer |
| B18 | P1 | **Relimitação do sketch** — Quick trim (borracha) em todas as curvas, break, trim-extend, close arc e complement. | DR-50, DR-51, DR-52, DR-53 | SKT-10, SKT-11, SKT-12, SKT-13, SKT-14 | 220 k | 2.9% | Auto | a fazer |
| B19 | P1 | **Seleção por janela e geometria externa no sketch** — Janela/cruzada pelo sentido do arrasto dentro do sketch; política de seleção externa (publicar ou bloquear). | DR-05, DR-18 | SKT-22, SKT-23 | 60 k | 0.8% | Visual | a fazer |
| B20 | P1 | **Filosofia dos painéis de comando** — Ordem suporte → modo → parâmetros → avançado em todos os comandos; caixas quando o menu é curto e abas quando é longo; direção normal a plano/superfície. | DR-09, DR-25, DR-26, DR-27, DR-28, DR-29 | SUP-23, GER-03 | 120 k | 1.6% | Visual | a fazer |
| B21 | P1 | **Extract com propagação gravada** — Propagação recalculada a cada atualização; extract disponível no botão direito de outros comandos. | DR-13, DR-14 | SUP-08 | 80 k | 1.0% | Auto | a fazer |
| B22 | P1 | **Montagem: origem, seleção e contexto** — Origem da montagem; Ctrl/Alt; ocultar/apagar como nas peças; editar peças vendo as outras. | DR-31, DR-32 | MON-11, MON-13, MON-14 | 120 k | 1.6% | Visual | a fazer |
| B23 | P1 | **PMI: realce e chamadas** — Realce das entidades da anotação, várias setas por anotação e seta que não atravessa o quadro. | DR-31, DR-47, DR-48 | PMI-04 | 70 k | 0.9% | Visual | a fazer |
| B25 | P1 | **VPM fase 1 (local e em pasta)** — SQLite + cofre em pasta: produto, revisão, documento, estrutura (BOM) e histórico; migração do índice JSON do fenkra-project; modo pasta compartilhada (LAN/OneDrive) com lock.json e detecção de conflito; VPMReader/Writer/Environment e painel lateral. | CTX-PLM, CTX-PN | PLM-02, PLM-03, PLM-04, PLM-05, PLM-07 | 340 k | 4.5% | Auto | a fazer |
| B47 | P1 | **PNEngine: regras de part-number e ambientes** — Editor de regras com campos ([NORMA], [CATEGORIA], [MATERIAL], [DIM], [SERIAL:n], [REV]), interpretador que preenche pela peça, contadores por ambiente/categoria, ambientes (Industrial, Aeroespacial, Cliente X) salvos em JSON. | CTX-PN | PLM-06 | 120 k | 1.6% | Auto | a fazer |
| B24 | P2 | **Pastas e setas de referência** — "Selecionados → nova pasta" escolhendo o pai; setas 3D do referencial nos pontos. | DR-46, DR-49 | GER-04, SUP-01 | 60 k | 0.8% | Visual | a fazer |

## Onda 3 — módulos novos e lacunas do benchmark → `0.1.0-alpha.29+`

| ID | P | Item | Origem | Catálogo | Tokens | % | Teste | Estado |
|---|---|---|---|---|---|---|---|---|
| B42 | P0 | **Infraestrutura da biblioteca** — Formato aberto por família (norma, fonte, nível A/B/C, licença), camadas pública/empresa/usuário, gerador de geometria por família, cálculo e conferência de propriedades nos testes. | CTX-HW, ADR-0008 | BIB-01, EST-03 | 150 k | 2.0% | Auto | em andamento |
| B26 | P1 | **Estrutura Pai → montagem dinâmica** — Corpos iguais viram uma peça com instâncias; subpeças linkadas; submontagens de solda; balões e lista de corte; desenho por corpo. | CTX-EST, DR-33, DR-44, DR-45 | EST-06, EST-07, EST-08, DES-08, DES-11 | 500 k | 6.5% | Auto + Visual | feito |
| B27 | P1 | **Catálogo de perfis com dimensões** — Perfis W/H/U/L Gerdau e Aço Brasil (NBR), IPE/HEA/HEB/HEM/UPN/UPE (ArcelorMittal, EN 10365), W/HP/S/C/MC/L/HSS (AISC), tubos EN 10210/10219, ASTM A500 e Vallourec; JIS e AS/NZS depois. Propriedades calculadas e conferidas com o publicado. | CTX-PRF, CTX-HW | EST-03 | 250 k | 3.3% | Auto | a fazer |
| B29 | P1 | **Fixadores industriais e smart fasteners** — Parafusos, porcas e arruelas ISO/DIN/ABNT (medidas pelos catálogos Ciser, Brasfixo, Vonder, Belzer) e preenchimento automático de furos com pilhas e restrições. | CTX-FIX, CTX-FXL, CTX-HW | BIB-01, BIB-02, MON-16 | 450 k | 5.9% | Auto + Visual | feito |
| B28 | P2 | **Gerador de engrenagens, eixos e chavetas** — Reta, helicoidal, interna e cremalheira (involuta), eixos e chaveta DIN 6885, com cálculo e inserção com restrições, no estilo do Inventor. | CTX-ENG | BIB-03, BIB-04 | 400 k | 5.2% | Auto | a fazer |
| B30 | P2 | **Desenho: atualização, hachura e normas** — Estado de atualização por vista; propriedades de hachura; 3º diedro; herança norma → folha → vista → elemento; vista interrompida e corte parcial. | DR-34, DR-35, DR-36, DR-37, DR-38, DR-39, DR-40, DR-41, DR-42, DR-43 | DES-02, DES-04, DES-06, DES-07, DES-10 | 300 k | 3.9% | Auto + Visual | a fazer |
| B31 | P2 | **Comando de aba unificado** — Um comando com tipo (aresta, cadeia, perfil, superfície, entre perfis) que abre as caixas do tipo. | DR-54 | CHP-17 | 150 k | 2.0% | Visual | a fazer |
| B32 | P2 | **Sketch: entidades e ferramentas que faltam** — Retângulo por 3 pontos e paralelogramo, elipse/cônicas, rasgo em arco, girar/escalar, padrão, texto, sketch 3D com restrições. | BM-SKT-01, BM-SKT-02, BM-SKT-03, BM-SKT-05, BM-SKT-06, BM-SKT-07, BM-SKT-08 | SKT-02, SKT-05, SKT-06, SKT-17, SKT-19, SKT-24, SKT-25 | 250 k | 3.3% | Auto | a fazer |
| B33 | P2 | **Sólidos: relevo, mola, modelagem direta** — Relevo/gravação, comando de mola (coil) e edição direta de faces. | BM-SOL-01, BM-SOL-02, BM-SOL-03 | SOL-21, SOL-22, SOL-23 | 200 k | 2.6% | Auto | a fazer |
| B34 | P2 | **Superfícies: guias, regrada e análises** — Loft com várias guias, superfície regrada, análise de desvio e de continuidade. | BM-SUP-01, BM-SUP-02, BM-SUP-03 | SUP-11, SUP-18, SUP-22 | 150 k | 2.0% | Auto | a fazer |
| B35 | P2 | **Chapa: entre perfis, desdobrar parcial, converter** — Aba entre perfis (lofted), desdobrar/redobrar parcial e converter sólido em chapa. | BM-CHP-01, BM-CHP-02, BM-CHP-03 | CHP-07, CHP-15, CHP-16 | 250 k | 3.3% | Auto | a fazer |
| B36 | P2 | **Montagem: restrições avançadas** — Largura, simetria, caminho e limite. | BM-MON-01 | MON-06 | 100 k | 1.3% | Auto | a fazer |
| B37 | P2 | **Estrutural: entalhe, cordões, barras** — Entalhe (notch) por perfil, cordão de solda 3D e plano de corte otimizado. | BM-EST-01, BM-EST-02, BM-EST-03 | EST-02, EST-05, EST-09 | 150 k | 2.0% | Auto | a fazer |
| B43 | P2 | **Fixadores aeronáuticos e militares** — AN/MS/MIL de domínio público (ASSIST, Distribution A): AN3–AN10, MS20004, MS20073, MS21250, MS21042, MS20002, MS20392, AN470/AN426, AN310/AN315; NAS só por fontes públicas equivalentes. | CTX-HW, CTX-FXL | BIB-06 | 200 k | 2.6% | Auto | a fazer |
| B44 | P2 | **Rolamentos** — Rolamentos por medidas de contorno ISO 15 (iguais em SKF, FAG/INA, NSK); capacidade de carga citada do fabricante. | CTX-HW | BIB-07 | 150 k | 2.0% | Auto | a fazer |
| B38 | P3 | **PMI: consultor de tolerância** — Verificar se a peça está totalmente cotada e tolerada. | BM-PMI-01 | PMI-05 | 120 k | 1.6% | Auto | a fazer |
| B39 | P3 | **API Python** — Automação por scripts (RNF-011). | BM-GER-01 | GER-07 | 250 k | 3.3% | Auto | a fazer |
| B40 | P3 | **Herdados: varredura cônica, subdivisão, dobra cega** — Varredura cônica/adaptativa com várias guias e G2 exato; formas por subdivisão; dobra cega e cônica, cortes flangeados, alívios aeronáuticos. | histórico |  | 200 k | 2.6% | Auto | a fazer |
| B45 | P3 | **Pneumática** — Cilindros ISO 15552/6432 (Festo, Parker, SMC) com cursos e fixações; séries próprias como camada de fabricante. | CTX-HW | BIB-08 | 150 k | 2.0% | Auto | a fazer |
| B46 | P3 | **Guias lineares e fusos** — Trilhos, patins e fusos por série de fabricante (THK, Hiwin, Bosch Rexroth). | CTX-HW | BIB-09 | 200 k | 2.6% | Auto | a fazer |

## Versionamento

- Trabalho em `develop`; um commit por item citando o ID (`B05 Rosca: …`).
- Fim de cada onda: versão no `Cargo.toml`, tag `v0.1.0-alpha.N`, `scripts\release.ps1`, commit do `Cargo.lock`.
- Push para `origin` só com confirmação do dono do repositório.
