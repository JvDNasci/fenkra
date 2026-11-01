# Módulos e recursos de softwares similares

**Data da pesquisa:** 2026-11-01. **Escopo:** só leitura. Nenhum arquivo foi criado nem alterado.

**Convenções.** `[V]` marca fato lido numa fonte desta pesquisa, listada na seção 5. `[C]` marca conhecimento prévio que não consegui confirmar aqui e que deve ser conferido antes de virar requisito. Licenças são leitura técnica, não parecer jurídico. O `reuso-opensource.md` do Fenkra já pede validação antes da primeira versão pública.

**Lacunas de evidência:**
- `wiki.freecad.org` bloqueou o acesso (Anubis). Usei resumos secundários do FreeCAD 1.0 e 1.1.
- Não achei detalhes de Mold Wizard e Routing do NX, nem de Mold Tooling e Human Builder do CATIA.
- Não achei detalhes do ZW3D nem do Solid Edge Frame/Piping/Harness em 2026.
- Não verifiquei Creo 12 e 13 (os resultados eram de versões antigas).
- Não verifiquei SU2. Não verifiquei a licença do OIDN (marcada `[C]`).

---

## 1. Resumo e as 15 maiores oportunidades

### 1.1 Leitura geral

O Fenkra já cobre bem a **geometria**: sketch, sólidos, superfícies, chapa, estrutural e soldas, montagem, desenho e PMI. Os softwares comparados diferenciam-se hoje em cinco áreas, quase todas ausentes do catálogo (168 features) e do backlog (B01–B109):

1. **Engenharia a jusante do modelo.** O CAD passa a avaliar o que desenhou: fabricabilidade (DFMXpress, Costing), simulação (Fusion, Onshape Simulation, Inventor Nastran) e inspeção (SOLIDWORKS Inspection).
2. **Fabricação e saída.** Nesting associativo (Fusion), impressão 3D com orientação e suportes (Fusion, Shapr3D), CAM embutido (Onshape CAM Studio, FreeCAD CAM) e moldes (SOLIDWORKS Mold Tools, NX Progressive Die Wizard).
3. **Sistemas além da peça.** Tubulação e tubos dobrados (Inventor Tube & Pipe, SOLIDWORKS Routing), chicotes (CATIA, Inventor, VEC/KBL), compósitos (Creo e CATIA).
4. **Colaboração, versão e automação.** Branch/merge (Onshape), scripts que criam features (FeatureScript), integração CAD–ERP (OpenBOM–Odoo, Onshape–Arena) e desenho automático por IA (SOLIDWORKS 2026 e Solid Edge 2026).
5. **Comunicação.** Render, animação e instruções ilustradas (Creo Illustrate, IsoDraw, Render Studio).

**Restrição de escopo do Fenkra.** A `visao.md` (seção 6) lista como **não-objetivos "por enquanto"**:
- CAM e G-code;
- simulação FEA/CFD e dinâmica de mecanismos com forças;
- renderização fotorrealista;
- PDM com servidor;
- colaboração em tempo real e nuvem;
- BIM.

Este relatório separa:
- o que cabe **dentro** desses limites, como cálculo 1D, cinemática com gráficos, DFM, nesting, custeio e export 3MF;
- o que **reabre** um não-objetivo, que exige decisão do mantenedor (FEM via solver externo, render, IFC).

**Padrão técnico recomendado** (explícito na seção 2.0): bibliotecas permissivas ou MPL-2.0 entram no código. Ferramentas GPL/AGPL (CalculiX, PrusaSlicer, Gmsh) entram **só como processo externo opcional**, com arquivos como interface.

### 1.2 As 15 maiores oportunidades

Ordenadas por valor para projetista mecânico/civil/aeronáutico dividido pelo esforço. Tamanhos: P = pequeno, M = médio, G = grande. IDs "BN" são as propostas da seção 4.

| # | Oportunidade | Quem tem | Valor | Esforço | Por quê |
|---|---|---|---|---|---|
| 1 | **Calculadoras de projeto mecânico** (parafuso, ajuste forçado, mola, viga, rolamento, eixo, engrenagem) ligadas ao modelo (BN01) | Inventor Design Accelerator, Toolbox `[C]` | Alto | M | Rust puro, sem risco de licença. Reaproveita B52, B28, B90, B42. |
| 2 | **Verificador de fabricação (DFM)** (BN02) | SOLIDWORKS DFMXpress `[V]`, Costing `[V]` | Alto | M | Regras sobre features que o Fenkra já conhece (furo, bolso, dobra, parede). Evita retrabalho. |
| 3 | **Análise de pórtico/barras 1D** (BN03) | Inventor Frame Analysis `[C]`, Fusion `[C]` | Alto (civil, estrutural) | M | O Fenkra já tem membros, nós, perfis e massa linear. Matriz de rigidez própria, sem GPL. |
| 4 | **Nesting de chapa** com lista de corte (BN04) | Fusion Manufacturing Extension `[V]` | Alto | M | Há motor Rust MPL-2.0 e MIT (jagua-rs, sparrow). Usa a planificada e o DXF (B86). |
| 5 | **Plano de inspeção e FAI (AS9102)** (BN05) | SOLIDWORKS Inspection `[V]` | Alto (aeronáutico) | M | Balões de PMI e desenho (B81/B82) viram tabela e formulário. |
| 6 | **Custeio e orçamento** (BN06) | SOLIDWORKS Costing `[V]`, Fusion `[V]` | Alto | M | Soma massa (B92), planificada (B86), lista de corte (B80) e comprimento de solda. |
| 7 | **Impressão 3D / DfAM**: 3MF, orientação, balanço, estimativa (BN07) | Fusion `[V]`, Shapr3D (Prusa direto) `[V]`, nTop `[V]` | Médio-alto | M | lib3mf BSD-2. Slicer externo por CLI, sem AGPL no binário. |
| 8 | **Estudo de movimento cinemático**: gráficos, trajetória, envelope varrido, animação (BN08) | SW Motion/Inventor Studio `[C]`, FreeCAD 1.1 anima Assembly `[V]` | Alto (mecanismos, flaps) | M | O solver cinemático já existe. Respeita o não-objetivo "forças". |
| 9 | **Instruções de montagem animadas** (BN09) | Creo Illustrate `[V]`, IsoDraw `[V]`, Composer `[C]` | Médio-alto | M | Usa explodida, BOM e vistas. Saída HTML/PDF/vídeo. |
| 10 | **Grandes montagens**: carga leve, simplificado/envelope, LOD (BN10) | SW Large Design Review, Inventor Shrinkwrap `[C]` | Alto (escala) | M-G | Sem isso o app trava com milhares de peças. Interage com B68. |
| 11 | **Tolerância empilhada 1D** a partir do PMI (BN11) | SOLIDWORKS TolAnalyst `[C]` | Médio-alto | M | Só existem libs Python pequenas `[V]`. Implementação própria em Rust. |
| 12 | **Tubos dobrados e tubulação** (rota 3D, tabela de dobra, isométrico) (BN12) | Inventor Tube & Pipe `[V]`, SW Routing `[V]` | Alto (aeronáutico, hidráulico) | G | Reaproveita varredura, wireframe 3D e biblioteca. |
| 13 | **Branch, merge e comparação de documentos** (BN13) | Onshape `[V]` | Médio-alto | M-G | Árvore de features em JSON permite merge de 3 vias por uid. Coerente com o VPM em pasta. |
| 14 | **Features por script** (tipo FeatureScript) (BN14) | Onshape FeatureScript `[V]` | Alto, a longo prazo | G | Rhai (MIT/Apache) ou WASM. Complementa B39 (API Python). |
| 15 | **Simulação estática linear via solver externo** (CalculiX + Netgen) (BN15) | FreeCAD FEM `[V]`, Fusion `[V]`, Onshape Simulation `[V]` | Muito alto | G | **Reabre um não-objetivo.** Só como ponte opcional e GPL isolado em processo. |

**Fora do top 15, mas relevantes:**
- **Moldes** (núcleo/cavidade, linha de partição, undercut). Sobre Draft, Split e Fill existentes.
- **Exportação IFC e ligações de aço.** Reabre o não-objetivo BIM.
- **Compósitos, chicotes e otimização topológica.** Esforço G e nicho.
- **Render e IA.** IA de comando, reconhecimento de features e text-to-CAD por LLM.
- **Visualizador web, ECAD–MCAD e ERP.**

---

## 2. Catálogo de MÓDULOS novos

### 2.0 Regra de licença usada neste relatório

O Fenkra é **LGPL-2.1-or-later** (ADR 0003). Seguindo o `reuso-opensource.md`:
- ✅ **Incorporável:** MIT, BSD, Apache-2.0, MPL-2.0 (copyleft por arquivo), Boost, zlib, LGPL-2.1+.
- ⚠️ **Só como biblioteca vinculada dinamicamente:** LGPL-3.0. O conjunto distribuído passa a ser LGPL-3.
- ❌ **Não incorporar:** GPL e AGPL. Só como **executável separado** (arquivos como interface, fonte entregue à parte, como o FreeCAD faz com o CalculiX). Isso é leitura técnica comum, mas exige validação jurídica.

Licenças confirmadas nesta pesquisa:

| Projeto | Licença | Compatível com LGPL-2.1+? | Fonte |
|---|---|---|---|
| CalculiX (ccx) | GPL-2.0-or-later `[V]` | ❌ só processo externo | calculix.de |
| Gmsh | GPL-2+ com exceção de vinculação a libs externas; licença comercial à parte `[V]` | ❌ não embutir | onelab.info |
| Elmer | ElmerSolver (lib) LGPL. Maioria dos módulos, ElmerGUI e ElmerGrid GPL `[V]` | ❌ para o conjunto. Lib LGPL não usável sozinha com os módulos | Elmer License Policy |
| Netgen | LGPL-2.1 `[V]` | ✅ biblioteca de malha de tetraedros | github NGSolve/netgen |
| MFEM | BSD `[V, fonte fraca: Wikipedia]` | ✅ | Wikipedia FEM list |
| deal.II | LGPL, em migração para Apache-2.0 `[V, fonte fraca]` | ✅ | idem |
| OpenCAMLib | LGPL-2.1+ desde 2018 `[V]` | ✅ | Debian copyright |
| libnest2d | LGPL-3.0-only `[V]` | ⚠️ só lib dinâmica | openSUSE spec |
| jagua-rs (Rust) | MPL-2.0 `[V]` | ✅ | github |
| sparrow (Rust) | MIT `[V]` | ✅ | github |
| PrusaSlicer | AGPL-3.0 `[V]` | ❌ só processo externo | github |
| CuraEngine | AGPL-3.0 `[V]` | ❌ só processo externo | github |
| Cycles | Apache-2.0 `[V]` | ✅ | code.blender.org |
| Embree | Apache-2.0 `[V]` | ✅ | github RenderKit |
| Manifold | Apache-2.0 `[V]` | ✅ (já listado) | github |
| fidget (Rust) | MPL-2.0 `[V]` | ✅ | github |
| libfive | lib e bindings MPL-2.0. Studio e guile GPL-2+ `[V]` | ✅ só a lib | github |
| cavalier_contours (Rust) | MIT ou Apache-2.0 `[V]` | ✅ | github |
| rapier (Rust) | Apache-2.0 `[V]` | ✅ (físico de jogo, não de engenharia) | github |
| faer (Rust) | MIT `[V]` | ✅ álgebra linear | github |
| lib3mf | BSD-2-Clause `[V]` | ✅ | github 3MFConsortium |
| OpenVDB | Apache-2.0 (antes MPL-2.0) `[V]` | ✅ | github |
| IfcOpenShell | núcleo LGPL-3.0+. Bonsai GPL-3 `[V]` | ⚠️ lib dinâmica / ❌ Bonsai | github |
| Rhai (Rust) | MIT ou Apache-2.0 `[V]` | ✅ (sandbox limitado `[V]`) | rhai.rs |
| topopt (Rust, 2D) | MIT `[V]` | ✅ (só 2D) | docs.rs |
| opencascade.js | LGPL `[V]`. OCCT em WASM sem exceção própria `[V]` | ⚠️ módulo separado ou licença comercial | dev.opencascade.org |
| KittyCAD modeling-app | MIT. Motor geométrico hospedado e fechado `[V]` | ✅ app, ❌ kernel | github |

O projeto tem Rust + egui. As libs C++ (Netgen, OpenCAMLib, OpenVDB) exigem FFI (`cxx`/`bindgen`). Isso é o mesmo custo já pago com o OCCT. Alternativas em Rust puro estão marcadas.

---

### 2.1 Calculadoras de projeto mecânico (BN01)

- **O que é:** assistentes de cálculo ligados ao modelo. Parafuso (pré-carga, VDI 2230 `[C]`), ajuste forçado e interferência, mola helicoidal, viga, vida de rolamento (ISO 281 `[C]`), resistência de eixo, engrenagem (ISO 6336 / AGMA `[C]`), estriados.
- **Quem tem:** Inventor Design Accelerator, que ganhou o *Involute Splines Calculator* na versão 2026 `[V]`. SOLIDWORKS Toolbox `[C]`.
- **Como funciona:** o usuário escolhe geometria e carga, o módulo calcula, aponta fatores de segurança e insere ou ajusta o componente (como o gerador de engrenagens do B28).
- **Libs abertas:** nenhuma necessária. Aritmética própria.
- **Viabilidade:** M. Cada calculadora é pequena e testável com exemplos de norma.
- **Riscos e deps:** tabelas de norma têm direito autoral (valores de referência, não texto). Responsabilidade do cálculo exige aviso. Deps: B28/B52 (engrenagens, eixos), B42, B90, B92.

### 2.2 Verificador de fabricação (DFM) (BN02)

- **O que é:** conjunto de regras que varre o modelo e lista violações com realce no 3D.
- **Quem tem:** SOLIDWORKS DFMXpress. Cobre fresa/furo, torneamento, chapa e injeção. Checa razão diâmetro/profundidade de furo, fundo plano, bolso profundo, canto interno vivo, raio de dobra e distância furo-borda `[V]`. Os valores são personalizáveis `[V]`. Fusion e outros têm análise de espessura e saliência `[C]`.
- **Como funciona:** o usuário escolhe o processo e um perfil de regras (JSON). Cada regra devolve elemento, valor medido, limite e sugestão.
- **Libs abertas:** não precisa. Usa a B-rep do OCCT (normais, distância mínima, `BRepExtrema`).
- **Viabilidade:** M. Começar por chapa e furo, que o Fenkra já modela por feature.
- **Riscos e deps:** falsos positivos. Os perfis devem ser editáveis e versionados como a biblioteca. Deps: B90, B86, B75.

### 2.3 Análise de pórtico/barras 1D (BN03)

- **O que é:** transforma uma estrutura de membros em um modelo de vigas (nós, seções, apoios, cargas). Resolve deslocamentos, esforços e reações. Mostra diagramas M/V/N e verifica tensão e flecha.
- **Quem tem:** Inventor Frame Analysis `[C]`. Fusion/Onshape para modelos de barras `[C]`. O Fenkra já tem "Barras (cálculo)" parcial (EST-09).
- **Como funciona:** matriz de rigidez de elemento de viga 3D (Euler-Bernoulli ou Timoshenko), montagem global, solução esparsa. As propriedades de seção vêm da biblioteca de perfis (B27).
- **Libs abertas:** faer (MIT) para a solução esparsa `[V]`. Dispensa solver externo.
- **Viabilidade:** M. É o primeiro passo seguro de "simulação" e não reabre o não-objetivo FEA.
- **Riscos e deps:** combinações de carga e verificação por norma (NBR 8800, EN 1993, AISC `[C]`) crescem em escopo. Deps: B27, B37, B79, B92.

### 2.4 Nesting de chapa (BN04)

- **O que é:** arranjar as peças planificadas em chapas comerciais, minimizando sobra. Gera layout, DXF por chapa e relatório de aproveitamento.
- **Quem tem:** Fusion Manufacturing Extension, com nesting associativo (atualiza quando o 3D muda) e de múltiplas chapas `[V]`.
- **Como funciona:** polígonos das planificadas (contorno e furos) entram no motor com rotações permitidas, espaçamento mínimo e tamanho de chapa. O motor devolve posições. O layout vira DXF (B105).
- **Libs abertas:**
  - jagua-rs (MPL-2.0) é motor de colisão 2D em Rust puro `[V]`.
  - sparrow (MIT) é heurística de nesting sobre o jagua-rs `[V]`. Aceita rotação discreta, passo fixo ou contínua. **Rejeita furos internos nas peças** `[V]`: o contorno entra preenchido, ou as peças pequenas precisam ser tratadas à parte.
  - libnest2d é LGPL-3.0-only `[V]`, então só como lib dinâmica. A alternativa Rust é melhor.
- **Viabilidade:** M.
- **Riscos e deps:** sparrow é pesquisa recente (`arXiv 2509.13329` `[V]`) com tempo de otimização de minutos. Precisa de botão "melhor esforço" e cancelamento. Deps: B86, B105, B80.

### 2.5 Plano de inspeção e FAI (AS9102) (BN05)

- **O que é:** balonar cada cota, tolerância, nota e GD&T do desenho ou do PMI e gerar a tabela de características. Opcionalmente, o formulário FAI e a importação de valores medidos.
- **Quem tem:** SOLIDWORKS Inspection. Balonagem automática de cotas, furos, acabamento e GD&T. Relatório em Excel com modelos AS9102/PPAP. Importa resultados de CMM (PC-DMIS, Calypso) `[V]`. QIF não foi confirmado nas fontes `[C: ISO 23952]`.
- **Como funciona:** cada anotação ancorada (B81/B82) recebe número. A tabela exporta CSV/XLSX. Colunas "medido" e "aprovado" são editáveis.
- **Libs abertas:** gravação de XLSX por crate Rust (verificar licença) `[C]`. Sem dependências de licença complexas.
- **Viabilidade:** M.
- **Riscos e deps:** o formato dos formulários AS9102 é público, mas o conteúdo exigido varia por cliente. Deps: B81, B82, B107, B108.

### 2.6 Custeio e orçamento (BN06)

- **O que é:** estima custo por peça e por montagem a partir de matéria-prima, processos e quantidades.
- **Quem tem:** SOLIDWORKS Costing. Reconhece chapa e usinagem pela geometria. Usa modelos de custo editáveis (preço de material, máquinas, custos fixos por dobra). Também faz peças soldadas, impressas e montagens `[V]`.
- **Como funciona:** modelo de custo em arquivo. Corte (perímetro × velocidade + número de furações), dobra (nº × tempo), solda (comprimento × velocidade), usinagem (volume removido do bloco bruto × taxa de remoção), tratamento (área × preço). Resultado na BOM.
- **Libs abertas:** nenhuma necessária.
- **Viabilidade:** M. Quase tudo já é calculado pelo Fenkra.
- **Riscos e deps:** a precisão depende dos dados do usuário `[V]`. Deve ficar claro que é estimativa. Deps: B92, B86, B80, B37, BN04 (aproveitamento).

### 2.7 Impressão 3D e DfAM (BN07)

- **O que é:** preparar para impressão: export 3MF com unidade e materiais, orientação, análise de balanço (overhang), estimativa de suporte, tempo e material. Opcionalmente lattice e preenchimento.
- **Quem tem:**
  - Fusion Manufacturing Extension: orientação automática, suportes associativos e export `[V]`.
  - Shapr3D: envio direto ao PrusaSlicer na versão 26.120 `[V]`.
  - nTop: lattice em três partes (célula, mapa e parâmetros), com reconstrução de 50 mil células em 1–2 s `[V]`.
  - FreeCAD Fasteners: compensação para rosca impressa `[V]`.
  - 3MF Beam Lattice v1.2.0 e Production v1.1.2 são padrões abertos `[V]`.
- **Como funciona (proposta):** (a) 3MF via lib3mf (BSD-2) `[V]`; (b) análise de balanço pelas normais da malha e ângulo limite; (c) abrir o 3MF no PrusaSlicer pela linha de comando (processo externo, AGPL-3 `[V]`), sem vincular; (d) lattice como **corpo implícito/malha**, nunca B-rep.
- **Libs abertas:**
  - lib3mf (BSD-2) `[V]`.
  - Manifold (Apache-2.0) para booleanas de malha `[V]`.
  - fidget (MPL-2.0, Rust, meshing próprio "menos testado que o libfive") `[V]`.
  - libfive (MPL-2.0) para f-rep `[V]`.
  - OpenVDB (Apache-2.0) para volumes esparsos `[V]`.
  - CuraEngine e PrusaSlicer são AGPL `[V]`: só como processo.
- **Viabilidade:** M para 3MF e análise. G para lattice.
- **Riscos e deps:** lattice cria um tipo novo de corpo que o OCCT não representa bem. O PrusaSlicer não suporta a Production Extension segundo a wiki da Bambu `[V]`. Deps: B84, B87, B92.

### 2.8 Estudo de movimento cinemático, envelope e animação (BN08)

- **O que é:** conduzir um mecanismo por um driver (ângulo ou deslocamento), gravar posições, gerar gráficos de posição/velocidade/aceleração de pontos, traçar trajetória e criar o **volume varrido** (envelope) para verificar folgas.
- **Quem tem:** FreeCAD 1.1 adicionou ferramenta de animação de movimento no Assembly `[V]`. SOLIDWORKS Motion Study, Inventor Dynamic Simulation e CATIA DMU Kinematics `[C]`; a última foi descrita como simulação de mecanismos a partir de juntas `[V]`.
- **Como funciona:** o solver de montagem (juntas e acoplamentos, MON-04/05) avança por passos. Cada passo grava poses. Interferência (MON-09) roda ao longo do curso.
- **Libs abertas:** nenhuma obrigatória. rapier (Apache-2.0) `[V]` é motor de jogo, não de engenharia. Só faria sentido para dinâmica, hoje não-objetivo.
- **Viabilidade:** M. Cinemática pura, sem forças, respeita o não-objetivo.
- **Riscos e deps:** envelope varrido sólido é custoso. Começar com amostragem de poses e envelope por união de malha (Manifold). Deps: B103, B36, B71.

### 2.9 Instruções de montagem e ilustração técnica (BN09)

- **O que é:** sequência de passos (qual peça entra, em qual posição) com explodida animada, BOM por passo, chamadas e exportação para HTML, PDF ou vídeo.
- **Quem tem:** Creo Illustrate (passos, chamadas e animações 3D, associativo ao CAD) `[V]`. Arbortext IsoDraw `[V]`. cadasio (web) `[V]`. SOLIDWORKS Composer `[C]`. Inventor Studio `[C]`.
- **Como funciona:** cada passo guarda estado de visibilidade e posição de componentes (estende a explodida). O player interpola. Exporta quadros e glTF.
- **Libs abertas:** `ffmpeg` externo (LGPL/GPL conforme build, verificar) `[C]`. HTML autocontido com visualizador próprio dispensa.
- **Viabilidade:** M.
- **Riscos e deps:** vídeo exige encoder. Começar por sequência de imagens e HTML. Deps: MON-10 (explodida, feita), B92, BN08.

### 2.10 Grandes montagens (BN10)

- **O que é:** abrir e navegar montagens com milhares de peças: carga leve (só malha), representações simplificadas, envelope, LOD, abrir só um subconjunto.
- **Quem tem:** SOLIDWORKS Large Design Review, Inventor Shrinkwrap/Level of Detail `[C]`. O Fenkra tem qualidade gráfica ajustável (B61).
- **Como funciona:** cada peça guarda malhas em vários níveis de detalhe e uma versão simplificada (casca externa ou caixa). A montagem escolhe a representação por componente ou por distância.
- **Libs abertas:** Manifold (Apache-2.0) `[V]` ou OCCT para casca externa.
- **Viabilidade:** M-G.
- **Riscos e deps:** consistência de cache (representação desatualizada). Deps: B61, B68, B64.

### 2.11 Tolerância empilhada (BN11)

- **O que é:** análise de pior caso, RSS e Monte Carlo sobre uma cadeia dimensional definida no PMI.
- **Quem tem:** SOLIDWORKS TolAnalyst `[C]`. Em código aberto só há bibliotecas Python pequenas: `pytolerance` (distribuição gaussiana, v0.0.2), `stackcore`, `Tol Ninja` (cadeias 1D e coaxiais) `[V]`.
- **Como funciona:** o usuário seleciona cotas e tolerâncias (B108) em ordem, define sinais e distribuições. O módulo calcula pior caso, RSS e histograma Monte Carlo, e a contribuição de cada cota.
- **Libs abertas:** nenhuma necessária (crate `rand`, MIT/Apache `[C]`). As libs Python servem como referência, não como dependência.
- **Viabilidade:** M para 1D. 3D por GD&T com datums é G e P3.
- **Riscos e deps:** a interpretação de GD&T na cadeia é sutil. Entregar 1D primeiro. Deps: B108, B82.

### 2.12 Tubos dobrados e tubulação (BN12)

- **O que é:** rota 3D de tubo rígido ou dobrado com diâmetro, espessura e raio de dobra. Valida trechos retos mínimos e colisões. Gera tabela de dobra para máquina, lista de corte e isométrico.
- **Quem tem:** Inventor Tube & Pipe. Roteia tubo rígido, tubo dobrado e mangueira na montagem. Escolhe conexões por regra de norma e fornece dados de dobra para máquinas de CNC `[V]`. SOLIDWORKS Routing (2026: roteamento por diretriz para elétrica, tubo e tubulação) `[V]`. Solid Edge tem piping `[V, fonte fraca]`.
- **Como funciona:** polilinha 3D (existe no wireframe) → arcos de dobra → varredura do perfil (existe). Dados de dobra (comprimento, rotação, ângulo `[C]`) saem por tabela. Conexões vêm da biblioteca.
- **Libs abertas:** nenhuma. Normas dimensionais (ISO 1127, ASME B36.10, AN/MS de flange `[C]`) seguem a regra de dados por nível (A/B/C) do B42.
- **Viabilidade:** G.
- **Riscos e deps:** biblioteca de conexões é trabalho de dados. Compensação de retorno elástico (springback) é específica de máquina. Deps: B32 (sketch 3D), B42, B99.

### 2.13 Moldes e matrizes (BN13)

- **O que é:** análise de saída e undercut, linha de partição, superfícies de fechamento (shut-off) e de partição, divisão em núcleo e cavidade.
- **Quem tem:** SOLIDWORKS Mold Tools: análise de saída, linha de partição, undercut, shut-off, superfície de partição e Tooling Split `[V]`. NX Progressive Die Wizard: layout de tira, bibliotecas de base e insertos `[V]`. Inventor Mold Design e Mold Wizard `[C]`.
- **Como funciona:** classifica faces por normal contra a direção de abertura, extrai a silhueta como linha de partição, fecha furos com Fill (existe) e divide o bloco com Split (existe).
- **Libs abertas:** nenhuma específica.
- **Viabilidade:** M para análise e divisão. G para base de molde com biblioteca.
- **Riscos e deps:** catálogos de base de molde (HASCO, DME) têm direitos autorais e entram como dados B/C. Deps: SOL-12 (Draft), B09, B06.

### 2.14 Branch, merge e comparação de documentos (BN14)

- **O que é:** ramificar um projeto para experimentar, comparar e mesclar de volta com resolução de conflito.
- **Quem tem:** Onshape: branches são espaços de trabalho. O merge pode substituir, mesclar tudo ou manter o alvo, por aba, e compara contra o ponto de ramificação. Em Part Studio e montagem, itens novos ou apagados persistem. Desenhos e formatos não nativos são substituídos por inteiro `[V]`.
- **Como funciona (proposta):** ao salvar, cada feature tem uid e hash. O merge de 3 vias compara base × ramo A × ramo B por uid. Conflitos aparecem por feature, com parâmetros lado a lado.
- **Libs abertas:** `similar`/`diffy` (MIT) `[C]`. A lógica é própria.
- **Viabilidade:** M-G.
- **Riscos e deps:** referências topológicas (ADR 0010) precisam sobreviver ao merge. Deps: B15, B64, B66, B25, B94.

### 2.15 Features por script (BN15)

- **O que é:** o usuário escreve um recurso novo (com parâmetros tipados e ícone) que entra na árvore e recalcula como qualquer feature.
- **Quem tem:** Onshape FeatureScript. A versão 2026 melhorou navegação no Feature Studio, busca na biblioteca padrão e símbolos `[V]`. SOLIDWORKS e Inventor oferecem API/iLogic `[V parcial]`.
- **Como funciona:** o script recebe contexto (faces selecionadas, parâmetros) e devolve operações do núcleo. O Fenkra grava o script no documento com versão.
- **Libs abertas:** Rhai (MIT/Apache) é fácil de embutir, mas o sandbox é limitado `[V]`. WASM (wasmtime, Apache-2.0 `[C]`) dá isolamento real. As duas dão determinismo desde que sem I/O.
- **Viabilidade:** G.
- **Riscos e deps:** versionar a API (SemVer). O corpus (B53) deve cobrir scripts. Deps: B39, B67, B53.

### 2.16 Simulação estática linear via solver externo (BN16) — decisão necessária

- **O que é:** tensão e deslocamento em peça ou montagem, com restrições, cargas, contato simples e malha de tetraedros.
- **Quem tem:**
  - FreeCAD FEM: CalculiX (padrão), Elmer, Mystran e Z88 como solvers; malhas por Netgen e Gmsh `[V]`. A versão 1.1 trouxe eletromagnetismo, filtros de pós-processamento, membrana/treliça e contato *tied/hard* `[V]`.
  - Fusion: estático, modal, flambagem, térmico, tensão térmica, otimização de forma, não linear e evento `[V]`.
  - Onshape: simulação estática linear na nuvem, agora assíncrona `[V]`.
  - Inventor Nastran `[V]`.
- **Como funciona:** gerar malha → escrever o arquivo de entrada do solver → rodar o executável → ler resultados → colorir no viewport.
- **Libs abertas:**
  - **Malha:** Netgen (LGPL-2.1, embutível) `[V]`. Gmsh é GPL e fica fora `[V]`.
  - **Solver externo:** CalculiX (GPL-2+) como processo `[V]`.
  - **Solver embutido:** MFEM (BSD) em C++ `[V, fonte fraca]` ou solver próprio em Rust com faer (MIT) `[V]`.
- **Viabilidade:** G. Em fases: (1) pórtico 1D (BN03), (2) ponte CalculiX opcional, (3) solver próprio.
- **Riscos e deps:**
  - **Reabre um não-objetivo da visão.**
  - Qualidade de malha e contato são fontes grandes de suporte.
  - Resultados exigem aviso de uso.
  - Deps: B92, B87 (STEP/XCAF), BN03.

### 2.17 Otimização topológica (BN17)

- **O que é:** remover material de baixa tensão sob cargas e restrições. Gera forma orgânica para impressão ou fundição.
- **Quem tem:** Fusion generative design, que gera e testa centenas de alternativas `[V]`. Creo Generative Topology Optimization e Generative Design (nuvem) `[V]`.
- **Libs abertas:** `topopt` (Rust, MIT) é só 2D `[V]`. Kratos tem módulo SIMP `[V]`, licença não confirmada.
- **Viabilidade:** G. Depende de BN16 em 3D.
- **Riscos e deps:** pós-processamento (suavizar, voltar a B-rep) é difícil. P3.

### 2.18 Exportação IFC e ligações de aço (civil) (BN18) — reabre BIM

- **O que é:** (a) exportar estrutura soldada como IFC (`IfcMember`, `IfcPlate`) para coordenar com projeto civil; (b) verificar ligações parafusadas e soldadas (chapa de ligação, pernos, filetes) com cálculo à mão.
- **Quem tem:** FreeCAD BIM/Arch e o addon Reinforcement (armaduras) `[V]`. BricsCAD BIM `[V, fonte de revenda]`.
- **Libs abertas:** IfcOpenShell: núcleo LGPL-3.0+, com IFC4x3 e API C++/Python `[V]`. Bonsai é GPL-3 `[V]`, só estudar.
- **Viabilidade:** M para export IFC. M-G para verificação de ligação.
- **Riscos e deps:** BIM é não-objetivo. Limitar a export de estrutura. Normas NBR 8800 e NBR 6118 exigem revisão técnica `[C]`. Deps: B27, B26, B87.

### 2.19 Compósitos (laminados) (BN19)

- **O que é:** camadas (plies) por zona, direção de fibra e espessura variável. Simula *draping* para detectar cisalhamento excessivo, gera o padrão plano de cada camada e o ply book.
- **Quem tem:** Creo Composite Design: draping, plano de corte, DXF e ply book automático `[V]`. CATIA Composites `[V parcial]`. Fibersim e FEMPLY `[V]`.
- **Libs abertas:** não achei biblioteca aberta madura. O algoritmo clássico (fishnet) é implementável `[C]`.
- **Viabilidade:** G. Nicho aeronáutico.
- **Riscos e deps:** exige superfície de molde (OML) robusta. Deps: B04, B34, B86. P3.

### 2.20 Chicotes e cabos (BN20)

- **O que é:** rota de fios e feixes em 3D, lista de fios, achatamento (formboard) e desenho de chicote.
- **Quem tem:** SOLIDWORKS Routing elétrico `[V]`. Inventor Cable & Harness `[V catálogo]`. CATIA Electrical Harness (montagem, instalação, achatamento) `[V, fonte não oficial]`. Troca de dados padrão: VEC (VDA 4968 / PSI 21) e KBL `[V]`.
- **Libs abertas:** o esquema VEC é público. A licença do esquema não foi verificada.
- **Viabilidade:** G. Sem ECAD embutido, a lista de fios entra manual ou por CSV.
- **Riscos e deps:** nicho. P3.

### 2.21 Render e comunicação visual (BN21) — toca um não-objetivo

- **O que é:** imagem de qualidade com materiais, luzes e sombras.
- **Quem tem:** Onshape Render Studio: câmera, luz, conjuntos de aparência, render em lote `[V]`. SOLIDWORKS Visualize `[V]`. FreeCAD Render usa renderizadores externos (Cycles standalone, LuxCore, Pov-Ray, Appleseed, OSPRay). O mantenedor parou de manter em nov/2025 `[V]`.
- **Libs abertas:** Cycles (Apache-2.0) `[V]`, Embree (Apache-2.0) `[V]`, OIDN `[C]`.
- **Recomendação:** não perseguir fotorrealismo. Melhorar o viewport em tempo real (sombra suave, oclusão de ambiente, PBR por material da biblioteca). Exportar glTF para Blender/Cycles.
- **Viabilidade:** M (viewport) / G (renderizador próprio).
- **Riscos e deps:** B61, B92. P2–P3.

### 2.22 ERP, liberação e ECAD (BN22–BN23)

- **ERP/ECO:** OpenBOM entrega EBOM ao Odoo (itens e BOMs, com anexos STEP/PDF) e valida o EBOM antes do ERP `[V]`. A integração Onshape–Arena executa o fluxo de liberação e mudança com PN sincronizado e PDFs sem marca-d'água `[V]`. O Fenkra não terá servidor (não-objetivo), mas pode exportar **pacote de liberação** (STEP + PDF + DXF + BOM em CSV/JSON/XML) e um registro de ECR/ECN em arquivo. Deps: B25, B47, B93. Viabilidade M.
- **ECAD–MCAD:** IDX é o protocolo moderno (leva intenção de projeto, não só geometria). O suporte no KiCad ainda é um merge request aberto, só recebendo, com possível release em 2027 `[V]`. Hoje o fluxo é STEP, como o KiCad StepUp `[V]`. O Fenkra pode importar STEP de PCB (B88), extrair contorno e furos para sketch e checar interferência. Viabilidade P-M. P3.

### 2.23 Camada de IA (BN24–BN25)

Panorama `[V, fontes agregadoras e de fornecedor]`:
- **Assistentes dentro do CAD ajudam a usar o software.** SOLIDWORKS AURA, Onshape AI Advisor (ajuda), Solid Edge Copilot, Autodesk Assistant (preview com extrusões e filetes), BricsCAD Assist e PARAMETRIZE.
- **Geração de geometria por texto** vem de ferramentas dedicadas, como a Zoo, que gera STEP editável. O app e a linguagem KCL são MIT, mas o **motor geométrico é hospedado e fechado** `[V]`.
- **Desenhos automáticos:** SOLIDWORKS 2026 e Solid Edge 2026 geram desenhos 2D completos de peças e montagens `[V]`.
- **Reconhecimento de features:** o BricsCAD melhorou o reconhecimento de chapa e reparo local em 2026 `[V]`.

Proposta realista para o Fenkra, sem depender de nuvem:
1. **Interfaces estáveis para agentes.** CLI/JSON (B39) com esquema versionado e opcionalmente um servidor de ferramentas. Qualquer LLM do usuário passa a dirigir o Fenkra.
2. **Reconhecimento determinístico de features** em corpos importados: furos, bolsos, filetes, chapa. Alimenta B35 (converter em chapa) e BN02 (DFM).
3. **Desenho automático por regras:** vistas padrão, cotas de contorno e de furo, tabela e carimbo. Sem ML.

Viabilidade: M para cada item. O ganho de IA generativa de geometria é incerto e vem em P3.

### 2.24 Visualizador web autocontido (BN26)

- **O que é:** exportar um HTML com modelo, árvore de montagem, medição e seção, para revisão por quem não tem o Fenkra.
- **Quem tem:** Onshape é nativo web. cadasio publica ilustrações interativas `[V]`.
- **Libs abertas:** glTF + visualizador em JS. Compilar o OCCT para WASM exige cuidado: um mantenedor do OCCT respondeu que a LGPL **não tem exceção para WASM**; a saída é módulo separado carregado dinamicamente ou licença comercial `[V]`.
- **Recomendação:** só o visualizador de malha (sem kernel) é seguro. Edição na web fica fora.
- **Viabilidade:** M. P2–P3.

### 2.25 Sustentabilidade (BN27)

- **O que é:** pegada de carbono e energia por material e processo.
- **Quem tem:** SOLIDWORKS Sustainability, baseado na base GaBi (proprietária) com métodos CML/TRACI `[V]`.
- **Libs abertas:** não confirmei base aberta equivalente.
- **Recomendação:** campo "kg CO₂e/kg" na tabela de materiais (B92), preenchido pelo usuário, somado na BOM. P3, P.

### 2.26 Módulos não recomendados agora

- **Ergonomia/humano** (CATIA Human Builder `[C]`): nicho e dependente de bibliotecas antropométricas.
- **Navio** (FreeCAD Ship `[V]`): fora do perfil.
- **CAM** (FreeCAD CAM com OpenCAMLib LGPL-2.1+ `[V]`; Onshape CAM Studio `[V]`; Fusion Manufacturing `[V]`): não-objetivo. Se um dia reabrir, `cavalier_contours` (MIT/Apache) cobre offset 2D com arcos e ilhas, mas só com junta arredondada e booleanas entre duas polilinhas fechadas `[V]`. OpenCAMLib cobre 3D. Pós-processadores e simulação de remoção são o grosso do trabalho.

---

## 3. RECURSOS novos dentro das bancadas existentes

Itens ausentes do catálogo (alpha.26) e do backlog, por bancada. O que veio de fonte desta pesquisa está marcado `[V]`.

### Sketch
- **Auto-escala ao colocar a primeira cota** e **detecção automática de contornos com faces selecionáveis** ao sair do sketch (opções do FreeCAD 1.1) `[V]`. Reduzem cliques.
- **Restrição de fluxo (Flow constraint)** no sketch (Onshape 1.223) `[V]`.
- **Imagem de referência calibrada** no plano do sketch (escala por duas pontos) `[C]`.
- **Curvas por equação e ajuste de spline a pontos** `[C]`.
- **Blocos de sketch reutilizáveis** com pontos de conexão `[C]`.

### Sólidos
- **Features de plástico:** nervura em grade, boss de montagem, snap-fit, grelha de ventilação e lip/groove `[C]`. São compostas de features existentes.
- **Análise de espessura de parede, de saliência e de curvatura de filete** `[C]` (base do BN02).
- **Furos múltiplos em corpos separados dentro de um mesmo corpo** (multi-sólido no Body, FreeCAD 1.1) `[V]`.
- **Pré-visualização transparente e ajuste direto de parâmetros no 3D** (FreeCAD 1.1) `[V]`. Complementa GER-02.
- **Ferramenta de transformação com pivô** em origem, centro de massa ou ponto livre, e destino por ponto (FreeCAD 1.1) `[V]`.

### Superfícies
- **Mapa de desvio e curvatura** já está em B34. Falta **ferramenta de seleção por proximidade** (lista objetos sob o cursor e deixa os obstrutivos transparentes, FreeCAD 1.1 *Clarify Selection*) `[V]`, que se soma ao B95.

### Chapa
- **Loft de chapa (aba entre perfis) e jog** foram lançados no Onshape em 2026 (1.210 e 1.220) `[V]`. Confirmam B35.
- **Tabelas de fator K como arquivo** (CSV/planilha, normas ANSI/DIN) e **modo de engenharia** que exige K explícito (FreeCAD SheetMetal) `[V]`. Entra em B77.
- **Perfuração de dobra (bend perforation)**, **auto-esquadro (miter)** e **hem** (FreeCAD SheetMetal) `[V]`.
- **Reparo local de chapa e reconhecimento melhorado** (BricsCAD V26.2: SMREPAIR) `[V]`.
- **Decomposição de dobras em loft** importadas em abas múltiplas (BricsCAD) `[V]`.
- **Abas e rasgos de encaixe (tab and slot)**, **corte normal** e **desdobramento com notas de dobra** `[C]`.
- **Matriz progressiva e layout de tira** para chapa estampada (NX PDW) `[V]`. P3.

### Estrutural e soldas
- **Corrimão, escada e guarda-corpo paramétricos** (NR-12, NBR 14718 `[C]`): alto valor industrial no Brasil.
- **Chapa de base, chumbadores e ligações padronizadas** (cantoneira, chapa de topo) `[C]`.
- **Mapa de soldas com comprimento por processo** (alimenta BN06).

### Montagem
- **Posições nomeadas** de montagem, com associação ao contexto (Onshape 1.209) `[V]`. Útil para configurações de mecanismo (flap recolhido/estendido).
- **Designador de referência** como propriedade de instância (Onshape 1.218) `[V]`.
- **Supressão de instância por expressão** (Onshape 1.221) `[V]`.
- **Travar subconjuntos com peças defeituosas** (Onshape 1.222) `[V]`.
- **Indicador persistente de centro de massa** (Onshape 1.219/1.222) `[V]`.
- **Encaixe magnético** (Solid Edge 2026, inferência por IA) `[V]`. Já coberto em intenção por B99.
- **Distância mínima/folga entre conjuntos selecionados** e **volume varrido** `[C]`.
- **Percurso imersivo (Walkthrough)** do Solid Edge 2026 `[V]`. Baixa prioridade.

### Desenho técnico
- **Replicar anotações entre vistas** (Onshape 1.213, beta) e **replicar chamadas de furo e rosca** (1.218) `[V]`.
- **Cota de comprimento de arco, meia cota de diâmetro e vista auxiliar por eixo** (Onshape 1.216/1.222) `[V]`.
- **Símbolos de condição de borda ISO 13715** (Onshape 1.223) `[V]`.
- **Símbolos de acabamento com até 7 linhas** (Onshape 1.220) `[V]`.
- **Vistas sugeridas que posicionam as anotações MBD sozinhas** (Onshape 1.220) `[V]`.
- **Controles de tolerância +/+ e −/−** (Onshape 1.213) `[V]`. Complementa B108.
- **Copiar folhas entre desenhos** (Onshape 1.218) `[V]`.
- **Desenho automático completo** (SOLIDWORKS 2026, Solid Edge 2026) `[V]`. Ver BN25.
- **Explodida e BOM em páginas de desenho** (FreeCAD TechDraw 1.0) `[V]`. O Fenkra já tem explodida e BOM; falta o encaixe no desenho.
- **Exportar linha oculta para desenho técnico** (Plasticity 2026.1) `[V]`.

### PMI
- **Exportação STEP AP242 com PMI semântico** e **plano de inspeção a partir do PMI** (BN05). O B87 prepara a ponte XDE.

### Biblioteca de normas
- **Motores elétricos** (carcaças IEC/NEMA) `[C]`, **correntes e rodas dentadas** (ISO 606) `[C]`, **correias e polias** (síncronas e trapezoidais) `[C]`, **molas** (gerador), **O-rings e canais** (ISO 3601) `[C]`, **perfis de alumínio** com ranhura em T `[C]`, **parafusos de potência trapezoidais**, **flanges** (ASME B16.5, EN 1092) `[C]`, **estriados** (calculadora no Inventor 2026 `[V]`).
- **Inserts de chapa (PEM) e insertos de rosca para impressão** (FreeCAD Fasteners) `[V]`.
- **Compensação de rosca para impressão 3D, busca de fixadores, simplificar fixador para troca de arquivos e geração de BOM** (FreeCAD Fasteners) `[V]`.
- **Fixadores reconhecidos e associados automaticamente mesmo sem biblioteca** (SOLIDWORKS 2026) `[V]`. O Fenkra tem B29/B58/B98.

### Navegação, interface e geral
- **Versionamento e migração de arquivos de configuração** (FreeCAD 1.1) `[V]`. Já coberto por B64/B100.
- **Iluminação em três pontos** no viewport (FreeCAD 1.1) `[V]`.
- **Tabelas de variáveis lidas de CSV** (Onshape 1.209) `[V]`. Casa com B101 e B102.
- **Comentários ancorados na geometria e em vistas de desenho** (Onshape 1.222) `[V]`. Para revisão de projeto sem servidor, via arquivo anexo.
- **Notificação e fila de tarefas assíncronas** (Onshape Simulation assíncrono) `[V]`. Útil para BN04 e BN16.

---

## 4. Itens de backlog propostos

IDs "BN" para não colidir com B01–B109. Prioridades: P0 quebra o uso, P1 importante, P2 desejável, P3 futuro. Tamanhos em milhares de tokens, na faixa 80–450. **Todos devem entrar depois das Ondas 0–2**, porque dependem da fundação (referências v2, salvar, corpus).

| ID | Título | Módulo/área | O que muda | P | k tokens | Dependências |
|---|---|---|---|---|---|---|
| BN01 | Calculadoras de projeto: parafuso, ajuste forçado, mola, viga | Cálculo mecânico | Painel de cálculo com fatores de segurança. Insere ou ajusta o componente. | P1 | 300 | B28, B52, B42, B90 |
| BN01b | Calculadora de rolamento, engrenagem e estriado | Cálculo mecânico | Vida L10 e verificação de engrenagem. Dados do fabricante como camada. | P2 | 250 | BN01, B44 |
| BN02 | Perfil de regras DFM (furo, bolso, chapa) | Fabricabilidade | Varredura com lista de violações e realce 3D. Perfis JSON editáveis. | P1 | 300 | B90, B86, B75 |
| BN02b | Análise de espessura, saliência e canto vivo | Fabricabilidade | Mapa de cor sobre as faces. Limites por processo. | P2 | 200 | BN02 |
| BN03 | Análise de pórtico 1D | Estrutural | Matriz de rigidez de viga, apoios e cargas, diagramas M/V/N, flecha. | P1 | 400 | B27, B37, B79, B92 |
| BN03b | Verificação por norma de barras (aço) | Estrutural | Utilização por NBR 8800 / EN 1993 / AISC. Relatório. | P2 | 300 | BN03 |
| BN04 | Nesting de chapa | Chapa | Layout em chapas comerciais por jagua-rs/sparrow. Exporta DXF por chapa e aproveitamento. | P1 | 350 | B86, B105, B80 |
| BN05 | Plano de inspeção e FAI | Qualidade | Balões numerados do desenho/PMI, tabela de características, CSV/XLSX, formulário AS9102. | P1 | 300 | B81, B82, B107 |
| BN06 | Custeio por modelo de custo | Custeio | Modelo de custo em arquivo (material, processos, taxas). Coluna de custo na BOM. | P1 | 300 | B92, B86, B80, B37 |
| BN07 | Exportação 3MF e preparo de impressão | Impressão 3D | 3MF via lib3mf, orientação, análise de balanço, abertura no slicer externo. | P2 | 250 | B84, B87, B92 |
| BN07b | Lattice e preenchimento implícito | Impressão 3D | Corpo implícito/malha (TPMS, vigas) com export 3MF Beam Lattice. | P3 | 450 | BN07 |
| BN08 | Estudo de movimento cinemático | Montagem | Driver, gráficos de posição/velocidade/aceleração, trajetória, animação. | P1 | 300 | B103, B36, B71 |
| BN08b | Envelope varrido e folga ao longo do curso | Montagem | Volume varrido por amostragem e interferência em cada passo. | P2 | 250 | BN08 |
| BN08c | Posições nomeadas de montagem | Montagem | Salva e restaura configurações de mecanismo. | P2 | 100 | B71, B102 |
| BN09 | Instruções de montagem animadas | Comunicação | Passos com explodida, BOM por passo, saída HTML/PDF/quadros. | P2 | 300 | B92, BN08 |
| BN10 | Grandes montagens: carga leve e LOD | Montagem | Representações simplificadas, envelope, carga parcial. | P1 | 400 | B61, B68, B64 |
| BN11 | Tolerância empilhada 1D | Tolerâncias | Cadeia de cotas do PMI, pior caso/RSS/Monte Carlo, contribuição. | P2 | 250 | B108, B82 |
| BN12 | Rota de tubo dobrado | Tubulação | Rota 3D, raio de dobra, tabela de dobra, lista de corte. | P2 | 400 | B32, B42, B99 |
| BN12b | Biblioteca de conexões e isométrico | Tubulação | Conexões AN/MS e industriais. Desenho isométrico com lista. | P3 | 350 | BN12, B42 |
| BN13 | Análise de moldagem (saída, undercut, partição) | Moldes | Classificação de faces, linha de partição e divisão núcleo/cavidade. | P2 | 350 | B09, B06, B75 |
| BN14 | Branch e merge de documento | Gestão | Ramificação e merge de 3 vias por uid de feature. Comparação lado a lado. | P2 | 450 | B15, B64, B66, B25, B94 |
| BN15 | Features por script | Automação | Rhai/WASM, parâmetros tipados, versão da API. | P2 | 450 | B39, B67, B53 |
| BN16 | Ponte para solver externo (CalculiX) | Simulação | Malha (Netgen), arquivo .inp, execução, leitura e coloração de resultados. | P3 | 450 | B87, B92, BN03 (**decisão do mantenedor**) |
| BN17 | Otimização topológica | Simulação | SIMP 3D sobre BN16, saída em malha. | P3 | 450 | BN16, BN07 |
| BN18 | Exportação IFC de estrutura de aço | Civil | `IfcMember`/`IfcPlate` por IfcOpenShell (lib dinâmica). | P2 | 250 | B27, B26, B87 (**reabre BIM**) |
| BN18b | Ligações de aço e chumbadores | Civil | Chapa de base, ligações parafusadas e soldadas com cálculo à mão. | P2 | 400 | BN03b, B27 |
| BN18c | Corrimão, escada e guarda-corpo | Estrutural | Geradores paramétricos por norma (NR-12, NBR 14718). | P2 | 300 | B26, B42 |
| BN19 | Laminado: camadas e ply book | Compósitos | Plies por zona, direção de fibra e padrão plano por draping. | P3 | 450 | B04, B34, B86 |
| BN20 | Chicote: rota, lista e achatamento | Elétrica | Feixes 3D, lista de fios (CSV), formboard. VEC como destino futuro. | P3 | 400 | B32, B105 |
| BN21 | Viewport com sombra suave, oclusão e PBR | Visual | SSAO, sombra, PBR por material. Exporta glTF. | P2 | 200 | B61, B92 |
| BN21b | Exportação de cena para Cycles/Blender | Visual | glTF/USD com materiais. Sem renderizador próprio. | P3 | 150 | BN21 |
| BN22 | Pacote de liberação e exportação de BOM/ECO | Gestão | STEP+PDF+DXF+BOM (CSV/JSON/XML) e registro de ECR/ECN em arquivo. | P2 | 250 | B25, B47, B93 |
| BN23 | Importar placa (STEP/IDX) | ECAD–MCAD | Contorno e furos para sketch. IDX quando o suporte amadurecer. | P3 | 200 | B88, B105 |
| BN24 | Reconhecimento de features | IA / automação | Furos, bolsos, filetes e chapa em corpos importados. | P2 | 300 | B87, B35, BN02 |
| BN25 | Desenho automático por regras | Desenho | Vistas padrão, cotas de contorno e furo, tabela e carimbo. | P2 | 400 | B30, B106, B107, B81 |
| BN25b | Interface para agentes (JSON/ferramentas) | IA / automação | Esquema versionado do CLI. Servidor de ferramentas opcional. | P2 | 200 | B39 |
| BN26 | Visualizador HTML autocontido | Comunicação | glTF + visualizador JS, árvore, medição, seção. Sem kernel. | P2 | 250 | B87, BN21 |
| BN27 | Pegada de carbono por material | Sustentabilidade | Campo kg CO₂e/kg em materiais e soma na BOM. | P3 | 80 | B92 |
| BN28 | Recursos de plástico (boss, snap, grelha, lip) | Sólidos | Features compostas parametrizadas. | P2 | 250 | B06, B11, B48 |
| BN29 | Recursos de desenho do Onshape | Desenho | Replicar anotações, meia cota, arco, ISO 13715, acabamento multilinha. | P2 | 250 | B107, B81 |
| BN30 | Bibliotecas de máquinas (correntes, correias, motores, O-rings, perfis T) | Biblioteca | Novas famílias sobre o modelo genérico, com nível de dado A/B/C. | P2 | 400 | B42 |

**Soma aproximada:** cerca de 10 milhões de tokens, equivalente a um terço do backlog atual (≈25,8 M). Como ordem de entrega, recomendo BN03, BN02, BN04, BN01, BN06 e BN05. Esses entregam o maior valor com o menor risco de licença e sem reabrir não-objetivo.

---

## 5. Fontes (URLs)

### Fenkra (internas)
- `docs/benchmark/catalogo.md`, `docs/backlog/backlog.md`, `docs/arquitetura/reuso-opensource.md`, `docs/arquitetura/adr/0003-licenca.md`, `docs/produto/visao.md` (seção 6, não-objetivos), `docs/produto/roadmap.md`.

### FreeCAD
- Release notes 1.1: https://wiki.freecad.org/Release_notes_1.1 (bloqueado para leitura; resumo via busca)
- https://industryinsider.eu/3d-designing-and-simulation/freecad-1-1-improvements-following-the-latest-major-update/
- https://en.linuxadictos.com/FreeCAD-1.1-introduces-profound-improvements-in-visualization-and-interaction-with-transparent-previews-and-three-point-lighting.html
- https://librearts.org/2024/11/freecad-1-0/
- https://wiki.freecad.org/FEM_Module
- https://wiki.freecad.org/CAM_Workbench
- https://github.com/FreeCAD/FreeCAD-Addons
- https://github.com/FreeCAD/FreeCAD-render
- https://github.com/shaise/FreeCAD_SheetMetal
- https://github.com/shaise/FreeCAD_FastenersWB

### Onshape
- https://www.onshape.com/en/changelog/
- https://cad.onshape.com/help/Content/Document/merging.htm
- https://onshape.com/en/features/onshape-arena-connection
- https://forum.onshape.com/discussion/comment/132178/
- https://www.digitalengineering247.com/article/ptc-adds-designer-targeted-simulation-to-cloud-cad-onshape/simulate

### Fusion
- https://www.autodesk.com/products/fusion-360/blog/fusion-roadmap-2026/
- https://www.autodesk.com/products/fusion-360/simulation-extension
- https://www.autodesk.com/products/fusion-360/manufacturing-extension.html
- https://www.themanufacturer.com/articles/autodesk-expands-fusion-capabilities-leveraging-ai-to-unlock-automation-and-creativity-across-design-and-make/
- https://static.au-uw2-stg.autodesk.com/Class_Handout_IM463460.pdf

### SOLIDWORKS
- https://discover.solidworks.com/whats-new-in-solidworks-2026
- https://www.technia.com/en/resources/solidworks-latest-version/
- https://www.solidsolutions.co.uk/the-top-10-best-new-features-solidworks-2026-whats-new/
- https://help.solidworks.com/2023/English/SolidWorks/Sldworks/c_costing_overview.htm
- https://help.solidworks.com/2020/English/SolidWorks/sldworks/c_Mold_Design_Tools_Overview.htm
- https://www.javelin-tech.com/blog/2015/12/dfmxpress-solidworks-design-for-manufacturability/
- https://www.javelin-tech.com/3d/technology/solidworks-sustainability/
- https://www.cati.com/?p=124333 (SOLIDWORKS Inspection)

### Solid Edge, Inventor, CATIA, Creo, NX, outros CAD
- https://blogs.sw.siemens.com/solidedge/designcenter-solid-edge-2026/
- https://www.designworldonline.com/solid-edge-st3-delivers-new-ways-to-produce-better-designs-faster/
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-WhatsNew/files/INV2026-WHATS-NEW-ASSEMBLIES.htm
- https://help.autodesk.com/cloudhelp/2026/ENU/Inventor-Help/files/GUID-DA656EDC-7762-45F7-815F-75A8D80062E0.htm
- https://arkance.world/gb-en/resources/read/blogs/autodesk-inventor-2026-performance-modelling-and-interoperability-improvements
- https://plm-service.web.cern.ch/how-to/catia-workbenches
- https://www.scribd.com/document/259636122/Workbench-Inventory
- https://support.ptc.com/help/creo/creo_pma/r13/usascii/composite_design/draping_simulation.html
- https://www.4cadgroup.com/en-ca/products/creo-generative-design-extension
- https://www.plm.automation.siemens.com/en_us/Images/wp_nx_prog_die_wizard_tcm1023-4504.pdf
- https://blog.cadsoftwaredirect.com/?p=9983 (BricsCAD V26)
- https://www.digitalengineering247.com/article/octave-bricscad-v26.2-release-unveiled-ntroduces-30-new-and-enhanced-features
- https://digitalproduction.com/2026/10/21/plasticity-2026-2
- https://en.wikipedia.org/wiki/Shapr3D
- https://www.ptc.com/en/products/arbortext/isodraw
- https://support.ptc.com/help/creo/creoillustrate/en/creo_illustrate/gxy1.html
- https://www.fabbaloo.com/2020/08/cadasio-interactive-technical-illustrations-of-cad-data

### IA, nuvem, ERP, ECAD, impressão 3D
- https://blog.texocad.ai/posts/ai-cad-software-2026
- https://superkind.ai/blog/ai-cad-tools
- https://github.com/KittyCAD/modeling-app
- https://www.openbom.com/blog/from-cad-to-odoo-erp-how-openbom-bridges-engineering-and-manufacturing
- https://www.prostep.org/en/medialibrary/fact-sheets/vehicle-electric-container-vec
- https://fosdem.org/schedule/event/UHBKS8-ecad-mcad-collaboration-idx
- https://forum.kicad.info/t/kicad-stepup-a-seamless-ecad-mcad-pcb-data-integration/12344
- https://3mf.io/wp-content/uploads/sites/106/2025/02/3MF_Beam_Lattice_Extension_v1.2.0.pdf
- https://3mf.io/wp-content/uploads/sites/106/2025/02/3MF_Production_Extension_v1_1_2.pdf
- https://github.com/prusa3d/PrusaSlicer/wiki/Command-Line-Interface
- https://www.ntop.com/resources/blog/new-latticing-technology/

### Bibliotecas e licenças
- https://www.calculix.de/
- https://onelab.info/pipermail/gmsh/2015/009903.html
- https://github.com/ElmerCSC/elmerfem/blob/devel/license_texts/ElmerLicensePolicy.md
- https://github.com/NGSolve/netgen
- https://en.wikipedia.org/wiki/List_of_finite_element_software_packages
- https://metadata.ftp-master.debian.org/changelogs/main/o/opencamlib/stable_copyright
- https://src.opensuse.org/pool/libnest2d/raw/branch/factory/libnest2d.spec
- https://github.com/JeroenGar/jagua-rs
- https://github.com/JeroenGar/sparrow
- https://github.com/prusa3d/PrusaSlicer
- https://github.com/Ultimaker/CuraEngine
- https://code.blender.org/2013/08/cycles-render-engine-released-with-permissive-license/
- https://github.com/RenderKit/embree
- https://github.com/elalish/manifold
- https://github.com/mkeeter/fidget
- https://github.com/libfive/libfive
- https://github.com/jbuckmccready/cavalier_contours
- https://github.com/sebcrozet/rapier
- https://github.com/sarah-quinones/faer-rs
- https://github.com/3MFConsortium/lib3mf
- https://github.com/AcademySoftwareFoundation/openvdb
- https://github.com/IfcOpenShell/IfcOpenShell
- https://rhai.rs/book/about/license.html
- https://docs.rs/crate/topopt/latest/source
- https://dev.opencascade.org/project/opencascadejs
- https://pypi.org/project/pytolerance/
- https://github.com/slehmann1/Tol-Ninja

---

## Handoff

- **Resultado:** relatório entregue. Inclui 15 oportunidades priorizadas, 27 módulos novos com licença e viabilidade, recursos por bancada, 38 itens de backlog (BN01–BN30 com variantes) e fontes.
- **Evidência:** leitura de `catalogo.md`, `backlog.md`, `reuso-opensource.md`, ADR 0003, `visao.md` e `roadmap.md`. Cerca de 45 consultas web (buscas e leituras de página). Licenças lidas nas páginas dos projetos (tabela na seção 2.0).
- **Arquivos alterados:** nenhum.
- **Verificações:** cada item de backlog foi cruzado com B01–B109 e com o catálogo para evitar repetição.
- **Falhas e limites:**
  - A wiki do FreeCAD ficou bloqueada.
  - Faltaram fontes para NX Mold Wizard e Routing, CATIA Mold e Human, ZW3D, Solid Edge Frame/Piping/Harness 2026, Creo 12/13 e SU2.
  - Parte dos recursos de SOLIDWORKS, Inventor, CATIA e Composer vem de conhecimento prévio e está marcada `[C]`.
- **Riscos:**
  - BN16 (FEM), BN18 (IFC) e BN21 (render) tocam não-objetivos da `visao.md` e precisam de decisão do mantenedor.
  - Toda leitura de licença exige validação jurídica, principalmente a execução de GPL/AGPL como processo separado.
  - A documentação do Fenkra ainda cita C++/Qt (ADR 0002), enquanto o projeto usa Rust/egui. Conferi as propostas contra a stack Rust, mas o ADR pode estar desatualizado.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Relatório entregue com 15 oportunidades, 27 módulos novos com licenças verificadas, recursos por bancada e 38 itens de backlog; ressalvas: wiki do FreeCAD bloqueada, fontes ausentes para NX Mold, CATIA Mold e ZW3D, e três propostas (FEM, IFC, render) reabrem não-objetivos que o mantenedor precisa decidir.
